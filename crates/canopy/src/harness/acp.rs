//! ACP adapter: Gemini, OpenCode, any ACP agent, and `canopy fake-agent`.
//! Wire facts: `docs/harness-notes/acp.md`. ACP cannot steer, so a send during a turn is held
//! here and prompted when the turn ends (`Queued`).

use super::{Command, Ev, Out, SpawnSpec, Stream, command, kill_group, new_id};
use crate::jsonrpc::{Incoming, Peer};
use anyhow::{Context, Result, anyhow};
use protocol::{Decision, Delivery, ItemKind, ItemState, Policy, Status, Usage};
use serde_json::{Value, json};
use std::collections::HashMap;
use tokio::sync::mpsc;

pub async fn run(program: String, args: Vec<String>, spec: SpawnSpec, mut out: Out, cmds: mpsc::Receiver<Command>) {
    let mut child = match command(&program, &args, &spec.cwd).spawn() {
        Ok(c) => c,
        Err(e) => return out.ev(Ev::Exited { error: Some(format!("cannot start {program}: {e}")) }).await,
    };
    let pid = child.id();
    let error = match session(&mut child, pid, spec, &mut out, cmds).await {
        Ok(()) => None,
        Err(e) => Some(format!("{e:#}")),
    };
    kill_group(pid);
    let _ = child.wait().await;
    out.ev(Ev::Exited { error }).await;
}

struct Turn {
    peer: Peer,
    session_id: String,
    done: mpsc::UnboundedSender<Result<Value>>,
}

impl Turn {
    fn prompt(&self, text: String) {
        let (peer, done, sid) = (self.peer.clone(), self.done.clone(), self.session_id.clone());
        tokio::spawn(async move {
            let r = peer.request("session/prompt", json!({"sessionId": sid, "prompt": [{"type": "text", "text": text}]})).await;
            let _ = done.send(r);
        });
    }
}

async fn session(
    child: &mut tokio::process::Child,
    pid: Option<u32>,
    spec: SpawnSpec,
    out: &mut Out,
    mut cmds: mpsc::Receiver<Command>,
) -> Result<()> {
    let stdin = child.stdin.take().context("no stdin")?;
    let stdout = child.stdout.take().context("no stdout")?;
    let (peer, mut incoming) = Peer::spawn(stdout, stdin, true);

    let init = peer
        .request(
            "initialize",
            json!({"protocolVersion": 1, "clientCapabilities": {"fs": {"readTextFile": false, "writeTextFile": false}, "terminal": false},
                   "clientInfo": {"name": "canopy", "version": env!("CARGO_PKG_VERSION")}}),
        )
        .await
        .context("initialize")?;
    let mcp = json!([{"name": "canopy", "command": spec.mcp.0, "args": spec.mcp.1, "env": []}]);
    let cwd = spec.cwd.to_string_lossy();
    let can_load = init.pointer("/agentCapabilities/loadSession").and_then(Value::as_bool).unwrap_or(false);
    let session_id = match spec.resume.filter(|_| can_load) {
        Some(id) => {
            peer.request("session/load", json!({"sessionId": id, "cwd": cwd, "mcpServers": mcp})).await.context("session/load")?;
            id
        }
        None => {
            let r = peer.request("session/new", json!({"cwd": cwd, "mcpServers": mcp})).await.context("session/new")?;
            r.get("sessionId").and_then(Value::as_str).ok_or_else(|| anyhow!("session/new returned no sessionId"))?.to_owned()
        }
    };
    out.ev(Ev::Started { vendor_session: Some(session_id.clone()), pid }).await;

    let (done_tx, mut done_rx) = mpsc::unbounded_channel();
    let turn = Turn { peer: peer.clone(), session_id: session_id.clone(), done: done_tx };
    let mut running = false;
    let mut queued: Vec<String> = Vec::new();
    let mut text = Stream::default();
    // Permission request id we showed the user -> (JSON-RPC id, offered options).
    let mut perms: HashMap<String, (Value, Value)> = HashMap::new();
    let mut tools: HashMap<String, (String, String)> = HashMap::new();

    if !spec.prompt.is_empty() {
        turn.prompt(spec.prompt);
        running = true;
        out.ev(Ev::Status(Status::Running)).await;
    }

    loop {
        tokio::select! {
            msg = incoming.recv() => {
                let Some(msg) = msg else { return Ok(()) };
                match msg {
                    Incoming::Notification { method, params } if method == "session/update" => {
                        update(out, &mut text, &mut tools, params.get("update").unwrap_or(&Value::Null)).await;
                    }
                    Incoming::Request { id, method, params } if method == "session/request_permission" => {
                        let options = params.get("options").cloned().unwrap_or(json!([]));
                        let kind = params.pointer("/toolCall/kind").and_then(Value::as_str).unwrap_or("other");
                        if let Some(d) = auto_decision(spec.policy, kind) {
                            answer(&peer, id, &options, d);
                            continue;
                        }
                        text.close(out, None).await;
                        let req = new_id();
                        let title = params.pointer("/toolCall/title").and_then(Value::as_str).unwrap_or("permission").to_owned();
                        out.ev(Ev::Item { id: req.clone(), kind: ItemKind::Permission, state: ItemState::Streaming,
                                          body: json!({"title": title, "tool": params.get("toolCall")}) }).await;
                        out.ev(Ev::Status(Status::AwaitingPermission)).await;
                        perms.insert(req, (id, options));
                    }
                    Incoming::Request { id, .. } => peer.respond_err(id, -32601, "method not supported by canopy"),
                    Incoming::Notification { .. } => {}
                }
            }
            r = done_rx.recv() => {
                text.close(out, None).await;
                cancel_perms(&peer, out, &mut perms).await;
                match r {
                    Some(Err(e)) => out.ev(Ev::Item { id: new_id(), kind: ItemKind::Error, state: ItemState::Complete,
                                                      body: json!({"text": format!("{e:#}")}) }).await,
                    Some(Ok(v)) if v.get("stopReason").and_then(Value::as_str) == Some("refusal") => {
                        out.ev(Ev::Item { id: new_id(), kind: ItemKind::Notice, state: ItemState::Complete,
                                          body: json!({"text": "the agent refused"}) }).await
                    }
                    _ => {}
                }
                if queued.is_empty() {
                    running = false;
                    out.ev(Ev::Status(Status::Idle)).await;
                } else {
                    turn.prompt(std::mem::take(&mut queued).join("\n\n"));
                }
            }
            cmd = cmds.recv() => {
                match cmd {
                    Some(Command::Send { text: t, interrupt, reply }) => {
                        if !running {
                            turn.prompt(t);
                            running = true;
                            out.ev(Ev::Status(Status::Running)).await;
                            let _ = reply.send(Delivery::StartedTurn);
                        } else {
                            if interrupt {
                                peer.notify("session/cancel", json!({"sessionId": session_id}));
                            }
                            queued.push(t);
                            let _ = reply.send(Delivery::Queued);
                        }
                    }
                    Some(Command::Interrupt) => {
                        if running {
                            peer.notify("session/cancel", json!({"sessionId": session_id}));
                        }
                    }
                    Some(Command::Permission { request, decision }) => {
                        if let Some((id, options)) = perms.remove(&request) {
                            answer(&peer, id, &options, decision);
                            out.ev(Ev::Item { id: request, kind: ItemKind::Permission, state: ItemState::Complete,
                                              body: json!({"decision": decision}) }).await;
                            if perms.is_empty() {
                                out.ev(Ev::Status(Status::Running)).await;
                            }
                        }
                    }
                    Some(Command::Stop) | None => return Ok(()),
                }
            }
            _ = out.due() => out.flush().await,
        }
    }
}

/// Policy decides without asking: `Yolo` allows everything, `AutoEdit` allows non-executing
/// tool kinds. `Ask` always asks.
fn auto_decision(policy: Policy, kind: &str) -> Option<Decision> {
    match policy {
        Policy::Yolo => Some(Decision::Allow),
        Policy::AutoEdit if matches!(kind, "read" | "edit" | "search" | "think") => Some(Decision::Allow),
        _ => None,
    }
}

fn answer(peer: &Peer, id: Value, options: &Value, decision: Decision) {
    let want: &[&str] = match decision {
        Decision::Allow => &["allow_once", "allow_always"],
        Decision::AllowAlways => &["allow_always", "allow_once"],
        Decision::Deny => &["reject_once", "reject_always"],
    };
    let opts = options.as_array().map(Vec::as_slice).unwrap_or_default();
    let pick = want.iter().find_map(|k| opts.iter().find(|o| o.get("kind").and_then(Value::as_str) == Some(k)));
    match pick.and_then(|o| o.get("optionId")) {
        Some(option) => peer.respond(id, json!({"outcome": {"outcome": "selected", "optionId": option}})),
        // No matching option offered: the only honest answer is to not run the tool.
        None => peer.respond(id, json!({"outcome": {"outcome": "cancelled"}})),
    }
}

async fn cancel_perms(peer: &Peer, out: &mut Out, perms: &mut HashMap<String, (Value, Value)>) {
    for (req, (id, _)) in perms.drain() {
        peer.respond(id, json!({"outcome": {"outcome": "cancelled"}}));
        out.ev(Ev::Item { id: req, kind: ItemKind::Permission, state: ItemState::Failed, body: json!({"decision": "cancelled"}) }).await;
    }
}

async fn update(out: &mut Out, text: &mut Stream, tools: &mut HashMap<String, (String, String)>, u: &Value) {
    let s = |k: &str| u.get(k).and_then(Value::as_str);
    match s("sessionUpdate").unwrap_or("") {
        "agent_message_chunk" => {
            if let Some(t) = u.pointer("/content/text").and_then(Value::as_str) {
                text.push(out, None, ItemKind::AgentMessage, t).await;
            }
        }
        "agent_thought_chunk" => {
            if let Some(t) = u.pointer("/content/text").and_then(Value::as_str) {
                text.push(out, None, ItemKind::Reasoning, t).await;
            }
        }
        "tool_call" => {
            text.close(out, None).await;
            let id = new_id();
            let title = s("title").unwrap_or("tool").to_owned();
            if let Some(tc) = s("toolCallId") {
                tools.insert(tc.to_owned(), (id.clone(), title.clone()));
            }
            out.ev(Ev::Activity(title.clone())).await;
            out.ev(Ev::Item {
                id,
                kind: ItemKind::ToolCall,
                state: ItemState::Streaming,
                body: json!({"title": title, "input": u.get("rawInput")}),
            })
            .await;
            changes(out, u).await;
        }
        "tool_call_update" => {
            changes(out, u).await;
            let status = s("status").unwrap_or("");
            if matches!(status, "completed" | "failed")
                && let Some((id, title)) = s("toolCallId").and_then(|tc| tools.remove(tc))
            {
                let state = if status == "completed" { ItemState::Complete } else { ItemState::Failed };
                out.ev(Ev::Item {
                    id,
                    kind: ItemKind::ToolCall,
                    state,
                    body: json!({"title": s("title").unwrap_or(&title), "output": u.get("content")}),
                })
                .await;
            }
        }
        "usage_update" => {
            let used = u.get("used").and_then(Value::as_u64).unwrap_or(0);
            let cost = u
                .pointer("/cost/amount")
                .and_then(Value::as_f64)
                .filter(|_| u.pointer("/cost/currency").and_then(Value::as_str) == Some("USD"));
            out.ev(Ev::Usage(Usage { input: used, reported_cost_usd: cost, ..Usage::default() })).await;
        }
        _ => {}
    }
}

async fn changes(out: &mut Out, u: &Value) {
    for c in u.get("content").and_then(Value::as_array).into_iter().flatten() {
        if c.get("type").and_then(Value::as_str) == Some("diff") {
            let path = c.get("path").and_then(Value::as_str).unwrap_or_default().to_owned();
            let lines = |k: &str| c.get(k).and_then(Value::as_str).map_or(0, |t| t.lines().count() as u64);
            out.ev(Ev::Change { path, added: lines("newText"), removed: lines("oldText") }).await;
        }
    }
}
