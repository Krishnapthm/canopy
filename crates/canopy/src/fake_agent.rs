//! `canopy fake-agent`: a scripted ACP agent for tests, benches, and UI work without spending
//! tokens (ADR 0016). It exercises the real ACP adapter and the real MCP shim.
//!
//! Each prompt line is one step:
//!   say <text>          stream <text> as the agent's message, a word per chunk
//!   think <text>        stream <text> as reasoning
//!   sleep <ms>          pause (cancellable)
//!   tool <name> <json>  call a tool on the session's first MCP server; `$N` in the JSON is
//!                       replaced by the `id` field of the Nth tool result (1-based)
//!   ask <title>         request permission, then say "allowed" or "denied"
//!   edit <path>         report a file edit (a diff tool call) of one line
//!   usage <n>           report n tokens used
//!   crash <message>     write to stderr and exit with status 1
//!   fail <message>      end the turn with a JSON-RPC error
//! Any other line is said verbatim, so parent notices are echoed back.

use crate::jsonrpc::{Incoming, Peer};
use anyhow::{Context, Result, anyhow};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, watch};

struct Mcp {
    peer: Peer,
    _child: tokio::process::Child,
}

pub async fn run() -> Result<()> {
    let (peer, mut incoming) = Peer::spawn(tokio::io::stdin(), tokio::io::stdout(), true);
    let mut servers: Vec<Value> = Vec::new();
    let (cancel_tx, cancel_rx) = watch::channel(0u64);
    // Prompts run one at a time, so this lock is never contended; it only keeps the MCP
    // connection alive across turns.
    let mcp: Arc<Mutex<Option<Mcp>>> = Arc::default();
    while let Some(msg) = incoming.recv().await {
        match msg {
            Incoming::Request { id, method, params } => match method.as_str() {
                "initialize" => {
                    peer.respond(id, json!({"protocolVersion": 1, "agentCapabilities": {"loadSession": true}, "authMethods": []}))
                }
                "session/load" => {
                    servers = params.get("mcpServers").and_then(Value::as_array).cloned().unwrap_or_default();
                    peer.respond(id, json!({}));
                }
                "session/new" => {
                    servers = params.get("mcpServers").and_then(Value::as_array).cloned().unwrap_or_default();
                    peer.respond(id, json!({"sessionId": format!("fake-{}", std::process::id())}));
                }
                "session/prompt" => {
                    let text: String = params["prompt"].as_array().into_iter().flatten().filter_map(|b| b["text"].as_str()).collect();
                    let sid = params["sessionId"].clone();
                    let (peer, mut cancel, server) = (peer.clone(), cancel_rx.clone(), servers.first().cloned());
                    cancel.mark_unchanged();
                    let mcp = mcp.clone();
                    tokio::spawn(async move {
                        let r = script(&peer, &sid, &text, &mut cancel, server, &mut *mcp.lock().await).await;
                        match r {
                            Ok(reason) => peer.respond(id, json!({"stopReason": reason})),
                            Err(e) => peer.respond_err(id, -32000, &format!("{e:#}")),
                        }
                    });
                }
                _ => peer.respond_err(id, -32601, "not supported by fake-agent"),
            },
            Incoming::Notification { method, .. } if method == "session/cancel" => {
                cancel_tx.send_modify(|n| *n += 1);
            }
            Incoming::Notification { .. } => {}
        }
    }
    Ok(())
}

async fn script(
    peer: &Peer,
    sid: &Value,
    text: &str,
    cancel: &mut watch::Receiver<u64>,
    server: Option<Value>,
    mcp: &mut Option<Mcp>,
) -> Result<&'static str> {
    // Demo scripts are inherited by fake children; ordinary test prompts remain scripts.
    let scripts: Value = std::env::var("CANOPY_FAKE_SCRIPTS").ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
    let task = text.lines().filter(|line| !line.starts_with("Spawned by Canopy agent ")).collect::<Vec<_>>().join("\n");
    let text = scripts.get(task.trim()).and_then(Value::as_str).unwrap_or(text);
    let update = |u: Value| peer.notify("session/update", json!({"sessionId": sid, "update": u}));
    let mut results: Vec<Value> = Vec::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if line.starts_with("Spawned by Canopy agent ") {
            continue;
        }
        if cancel.has_changed().unwrap_or(false) {
            return Ok("cancelled");
        }
        let (verb, arg) = line.split_once(' ').unwrap_or((line, ""));
        match verb {
            "say" | "think" => {
                let kind = if verb == "say" { "agent_message_chunk" } else { "agent_thought_chunk" };
                for w in arg.split_inclusive(' ') {
                    update(json!({"sessionUpdate": kind, "content": {"type": "text", "text": w}}));
                }
                update(json!({"sessionUpdate": kind, "content": {"type": "text", "text": "\n"}}));
            }
            "load" => {
                let count: u32 = arg.parse().context("load takes a pulse count")?;
                let start = tokio::time::Instant::now();
                for pulse in 0..count {
                    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis();
                    update(json!({"sessionUpdate":"agent_message_chunk", "content":{"type":"text","text":format!("{stamp}\n")}}));
                    tokio::select! {
                        _ = tokio::time::sleep_until(start + Duration::from_millis(u64::from(pulse + 1) * 10)) => {},
                        _ = cancel.changed() => return Ok("cancelled"),
                    }
                }
            }
            "sleep" => {
                let ms: u64 = arg.parse().context("sleep takes milliseconds")?;
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_millis(ms)) => {}
                    _ = cancel.changed() => return Ok("cancelled"),
                }
            }
            "tool" => {
                let (name, args) = arg.split_once(' ').unwrap_or((arg, "{}"));
                let mut args = args.to_owned();
                for (n, r) in results.iter().enumerate().rev() {
                    args = args.replace(&format!("${}", n + 1), r["id"].as_str().unwrap_or(""));
                }
                let args: Value = serde_json::from_str(&args).context("tool arguments must be JSON")?;
                if mcp.is_none() {
                    *mcp = Some(connect(server.as_ref().ok_or_else(|| anyhow!("no MCP server configured"))?).await?);
                }
                let m = mcp.as_ref().map(|m| m.peer.clone()).context("mcp")?;
                let tc = format!("tc{}", results.len() + 1);
                update(
                    json!({"sessionUpdate": "tool_call", "toolCallId": tc, "title": format!("canopy {name}"), "kind": "other", "status": "in_progress", "rawInput": args}),
                );
                let r = m.request("tools/call", json!({"name": name, "arguments": args})).await?;
                let out = r["content"][0]["text"].as_str().unwrap_or("").to_owned();
                let failed = r["isError"].as_bool().unwrap_or(false);
                update(
                    json!({"sessionUpdate": "tool_call_update", "toolCallId": tc, "status": if failed { "failed" } else { "completed" },
                              "content": [{"type": "content", "content": {"type": "text", "text": out}}]}),
                );
                results.push(serde_json::from_str(&out).unwrap_or(json!({"text": out})));
                if failed {
                    update(
                        json!({"sessionUpdate": "agent_message_chunk", "content": {"type": "text", "text": format!("tool error: {out}\n")}}),
                    );
                }
            }
            "ask" => {
                let r = peer
                    .request(
                        "session/request_permission",
                        json!({"sessionId": sid, "toolCall": {"toolCallId": "ask", "title": arg, "kind": "execute"},
                               "options": [{"optionId": "yes", "name": "Allow", "kind": "allow_once"},
                                           {"optionId": "always", "name": "Always", "kind": "allow_always"},
                                           {"optionId": "no", "name": "Deny", "kind": "reject_once"}]}),
                    )
                    .await?;
                let said = match r.pointer("/outcome/optionId").and_then(Value::as_str) {
                    Some("yes" | "always") => "allowed",
                    Some(_) => "denied",
                    None => return Ok("cancelled"),
                };
                update(json!({"sessionUpdate": "agent_message_chunk", "content": {"type": "text", "text": format!("{said}\n")}}));
            }
            "edit" => {
                update(
                    json!({"sessionUpdate": "tool_call", "toolCallId": "edit", "title": format!("Editing {arg}"), "kind": "edit", "status": "completed",
                              "content": [{"type": "diff", "path": arg, "oldText": "a", "newText": "b"}]}),
                );
            }
            "usage" => update(json!({"sessionUpdate": "usage_update", "used": arg.parse::<u64>().unwrap_or(0), "size": 200000})),
            "crash" => {
                eprintln!("{arg}");
                std::process::exit(1);
            }
            "fail" => return Err(anyhow!("{arg}")),
            _ => update(json!({"sessionUpdate": "agent_message_chunk", "content": {"type": "text", "text": format!("{line}\n")}})),
        }
    }
    Ok("end_turn")
}

/// Starts the stdio MCP server from `session/new` and performs the MCP handshake.
async fn connect(server: &Value) -> Result<Mcp> {
    let command = server["command"].as_str().context("MCP server has no command")?;
    let args: Vec<String> = server["args"].as_array().into_iter().flatten().filter_map(|a| a.as_str().map(str::to_owned)).collect();
    let mut child = tokio::process::Command::new(command)
        .args(&args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let (stdin, stdout) = (child.stdin.take().context("stdin")?, child.stdout.take().context("stdout")?);
    let (peer, _incoming) = Peer::spawn(stdout, stdin, true);
    peer.request(
        "initialize",
        json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "fake-agent", "version": "0"}}),
    )
    .await?;
    peer.notify("notifications/initialized", json!({}));
    Ok(Mcp { peer, _child: child })
}
