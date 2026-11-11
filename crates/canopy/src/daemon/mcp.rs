//! The cross-vendor spawn tools (ADR 0010), served to each agent over its MCP shim. The
//! connection itself says who the caller is, which is how the parent edge is recorded.

use super::hub::{MAX_WAIT_S, McpCall, Msg};
use crate::jsonrpc::{Incoming, Peer};
use protocol::SessionId;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{mpsc, oneshot};

const PROTOCOL: &str = "2025-06-18";

pub async fn serve(
    caller: SessionId,
    reader: impl AsyncRead + Unpin + Send + 'static,
    writer: impl AsyncWrite + Unpin + Send + 'static,
    hub: mpsc::Sender<Msg>,
    harnesses: Arc<Vec<String>>,
) {
    let (peer, mut incoming) = Peer::spawn(reader, writer, true);
    while let Some(msg) = incoming.recv().await {
        let Incoming::Request { id, method, params } = msg else { continue };
        match method.as_str() {
            "initialize" => {
                let version = params.get("protocolVersion").and_then(Value::as_str).unwrap_or(PROTOCOL);
                peer.respond(
                    id,
                    json!({"protocolVersion": version, "capabilities": {"tools": {}},
                           "serverInfo": {"name": "canopy", "version": env!("CARGO_PKG_VERSION")},
                           "instructions": INSTRUCTIONS}),
                );
            }
            "tools/list" => peer.respond(id, json!({"tools": tools(&harnesses)})),
            "tools/call" => {
                let (peer, hub, caller) = (peer.clone(), hub.clone(), caller.clone());
                tokio::spawn(async move {
                    let r = call(&caller, &params, &hub).await;
                    let (text, is_error) = match r {
                        Ok(v) => (v.to_string(), false),
                        Err(e) => (e, true),
                    };
                    peer.respond(id, json!({"content": [{"type": "text", "text": text}], "isError": is_error}));
                });
            }
            "ping" => peer.respond(id, json!({})),
            _ => peer.respond_err(id, -32601, "method not found"),
        }
    }
}

async fn call(caller: &str, params: &Value, hub: &mpsc::Sender<Msg>) -> Result<Value, String> {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let a = params.get("arguments").cloned().unwrap_or(json!({}));
    let s = |k: &str| a.get(k).and_then(Value::as_str).map(str::to_owned);
    let need = |k: &str| s(k).filter(|v| !v.is_empty()).ok_or(format!("missing required argument {k:?}"));
    let mut timeout = None;
    let call = match name {
        "spawn_agent" => {
            McpCall::Spawn { harness: need("harness")?, prompt: need("prompt")?, title: s("title"), model: s("model"), cwd: s("cwd") }
        }
        "wait_agent" => {
            let ids: Vec<String> = match a.get("ids") {
                Some(Value::Array(v)) => v.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect(),
                Some(Value::String(one)) => vec![one.clone()],
                _ => return Err("missing required argument \"ids\"".into()),
            };
            let t = a.get("timeout_s").and_then(Value::as_u64).unwrap_or(MAX_WAIT_S).clamp(1, MAX_WAIT_S);
            timeout = Some((Duration::from_secs(t), ids.clone()));
            McpCall::Wait { ids }
        }
        "message_agent" => McpCall::Message { id: need("id")?, text: need("text")? },
        "list_agents" => McpCall::List,
        _ => return Err(format!("unknown tool {name:?}")),
    };
    let (tx, rx) = oneshot::channel();
    hub.send(Msg::Mcp { caller: caller.to_owned(), call, reply: tx }).await.map_err(|_| "canopy daemon is shutting down")?;
    match timeout {
        // Bounded so it never trips a harness tool timeout; the agent simply calls again.
        Some((t, ids)) => match tokio::time::timeout(t, rx).await {
            Ok(r) => r.map_err(|_| "canopy daemon is shutting down".to_owned())?,
            Err(_) => Ok(json!({"still_running": ids, "hint": "call wait_agent again to keep waiting"})),
        },
        None => rx.await.map_err(|_| "canopy daemon is shutting down".to_owned())?,
    }
}

fn tools(harnesses: &[String]) -> Value {
    let names = harnesses.join(", ");
    json!([
        {
            "name": "spawn_agent",
            "description": format!("Start a new coding agent as your child to work on a self-contained task, on any installed harness (vendor): {names}. Returns {{\"id\"}} immediately; the child runs in parallel. Use wait_agent to get its result. Give the child everything it needs in the prompt: it does not see your conversation."),
            "inputSchema": {"type": "object", "required": ["harness", "prompt"], "properties": {
                "harness": {"type": "string", "enum": harnesses, "description": "Which agent CLI runs the child."},
                "prompt": {"type": "string", "description": "The complete task for the child."},
                "title": {"type": "string", "description": "Short label shown in the tree."},
                "model": {"type": "string", "description": "Model for the child, if not the harness default."},
                "cwd": {"type": "string", "description": "Absolute working directory; defaults to yours."}
            }}
        },
        {
            "name": "wait_agent",
            "description": format!("Wait until at least one of the given child agents finishes its turn, for at most timeout_s seconds (max {MAX_WAIT_S}). Returns each agent's status and final message, or still_running. Call again to keep waiting."),
            "inputSchema": {"type": "object", "required": ["ids"], "properties": {
                "ids": {"type": "array", "items": {"type": "string"}},
                "timeout_s": {"type": "integer", "minimum": 1, "maximum": MAX_WAIT_S}
            }}
        },
        {
            "name": "message_agent",
            "description": "Send a follow-up message to one of your descendant agents, or to your parent. Returns how it was delivered: started_turn, queued, or steered.",
            "inputSchema": {"type": "object", "required": ["id", "text"], "properties": {
                "id": {"type": "string"}, "text": {"type": "string"}
            }}
        },
        {
            "name": "list_agents",
            "description": "List every agent you spawned, and theirs, with status and last message.",
            "inputSchema": {"type": "object", "properties": {}}
        }
    ])
}

const INSTRUCTIONS: &str = "Canopy runs coding agents from several vendors as one tree. Use spawn_agent to hand a self-contained task to another agent, possibly from another vendor, then wait_agent for its result.";

pub fn child_header(parent: &str) -> String {
    format!("Spawned by Canopy agent {parent}; your final message returns to that parent.")
}

#[test]
fn awareness_budget() {
    // Approximate tokens conservatively as characters / 4, rounded up.
    assert!((INSTRUCTIONS.chars().count() + child_header("01ABCDEFGHIJKLMNOPQRSTUVWXYZ").chars().count()).div_ceil(4) <= 150);
}
