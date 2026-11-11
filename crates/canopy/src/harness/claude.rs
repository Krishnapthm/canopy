//! Claude Code adapter: one `claude -p` stream-json process per session.
//! Every wire fact here is from `docs/harness-notes/claude.md` (verified on 2.1.318) and the
//! `fixtures/claude-2.1.318-*.jsonl` recordings the tests replay.

use super::{Command, Ev, Out, SpawnSpec, Stream, command, kill_group, new_id};
use crate::daemon::hub::MAX_WAIT_S;
use anyhow::{Context, Result};
use protocol::{Decision, Delivery, ItemKind, ItemState, Policy, Status, Usage};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::ChildStdin;
use tokio::sync::mpsc;

/// Canopy's own tools that only read state; asking the user about them would be noise.
const READ_ONLY_TOOLS: [&str; 2] = ["mcp__canopy__wait_agent", "mcp__canopy__list_agents"];

pub fn args(spec: &SpawnSpec) -> Vec<String> {
    let mcp = json!({"mcpServers": {"canopy": {"command": spec.mcp.0, "args": spec.mcp.1}}});
    let mut a: Vec<String> = [
        "-p",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--verbose",
        "--include-partial-messages",
        "--forward-subagent-text",
        "--permission-prompt-tool",
        "stdio",
        "--mcp-config",
    ]
    .map(str::to_owned)
    .into();
    a.push(mcp.to_string());
    if spec.policy == Policy::AutoEdit {
        a.extend(["--permission-mode".into(), "acceptEdits".into()]);
    }
    if let Some(m) = &spec.model {
        a.extend(["--model".into(), m.clone()]);
    }
    if let Some(r) = &spec.resume {
        a.extend(["--resume".into(), r.clone()]);
    }
    a
}

pub async fn run(spec: SpawnSpec, mut out: Out, mut cmds: mpsc::Receiver<Command>) {
    let mut c = command("claude", &args(&spec), &spec.cwd);
    // wait_agent blocks up to MAX_WAIT_S; the harness must not time the call out first.
    c.env("MCP_TOOL_TIMEOUT", ((MAX_WAIT_S + 30) * 1000).to_string());
    let mut child = match c.spawn() {
        Ok(c) => c,
        Err(e) => return out.ev(Ev::Exited { error: Some(format!("cannot start claude: {e}")) }).await,
    };
    let pid = child.id();
    let r = session(&mut child, pid, spec, &mut out, &mut cmds).await;
    kill_group(pid);
    let status = child.wait().await.ok();
    let error = match r {
        Err(e) => Some(format!("{e:#}")),
        Ok(true) => None,
        Ok(false) => status.filter(|s| !s.success()).map(|s| format!("claude exited with {s}")),
    };
    out.ev(Ev::Exited { error }).await;
}

/// Returns Ok(true) when stopped on request, Ok(false) when the process ended by itself.
async fn session(
    child: &mut tokio::process::Child,
    pid: Option<u32>,
    spec: SpawnSpec,
    out: &mut Out,
    cmds: &mut mpsc::Receiver<Command>,
) -> Result<bool> {
    let mut stdin = child.stdin.take().context("no stdin")?;
    let mut lines = BufReader::new(child.stdout.take().context("no stdout")?).lines();
    let mut p = Parser::new(spec.policy, pid);
    if !spec.prompt.is_empty() {
        write(&mut stdin, &user(&spec.prompt)).await?;
        p.pending_turns += 1;
        out.ev(Ev::Status(Status::Running)).await;
    }
    loop {
        tokio::select! {
            line = lines.next_line() => {
                let Some(line) = line? else { return Ok(false) };
                let Ok(v) = serde_json::from_str::<Value>(&line) else { p.unknown += 1; continue };
                for reply in p.line(&v, out).await {
                    write(&mut stdin, &reply).await?;
                }
            }
            cmd = cmds.recv() => match cmd {
                Some(Command::Send { text, interrupt, reply }) => {
                    if interrupt && p.pending_turns > 0 {
                        write(&mut stdin, &control("interrupt")).await?;
                    }
                    let delivery = if p.pending_turns > 0 { Delivery::Queued } else { Delivery::StartedTurn };
                    write(&mut stdin, &user(&text)).await?;
                    p.pending_turns += 1;
                    out.ev(Ev::Status(Status::Running)).await;
                    let _ = reply.send(delivery);
                }
                Some(Command::Interrupt) => {
                    if p.pending_turns > 0 {
                        write(&mut stdin, &control("interrupt")).await?;
                    }
                }
                Some(Command::Permission { request, decision }) => {
                    if let Some(reply) = p.decide(&request, decision, out).await {
                        write(&mut stdin, &reply).await?;
                    }
                }
                Some(Command::Stop) | None => return Ok(true),
            },
            _ = out.due() => out.flush().await,
        }
    }
}

async fn write(stdin: &mut ChildStdin, v: &Value) -> Result<()> {
    let mut line = v.to_string();
    line.push('\n');
    stdin.write_all(line.as_bytes()).await.context("claude stdin closed")?;
    Ok(())
}

/// `client_composed` stops a leading `/word` from running as a CLI slash command.
fn user(text: &str) -> Value {
    json!({"type": "user", "client_composed": true, "message": {"role": "user", "content": text}})
}

fn control(subtype: &str) -> Value {
    json!({"type": "control_request", "request_id": new_id(), "request": {"subtype": subtype}})
}

fn allow(request_id: &str, input: &Value) -> Value {
    json!({"type": "control_response", "response": {"subtype": "success", "request_id": request_id,
           "response": {"behavior": "allow", "updatedInput": input}}})
}

fn deny(request_id: &str) -> Value {
    json!({"type": "control_response", "response": {"subtype": "success", "request_id": request_id,
           "response": {"behavior": "deny", "message": "The user denied this in canopy."}}})
}

struct Perm {
    request_id: String,
    tool: String,
    input: Value,
}

/// Turns stdout lines into canopy events. Separate from the process so fixtures can replay it.
pub struct Parser {
    policy: Policy,
    pid: Option<u32>,
    started: bool,
    pub pending_turns: u32,
    text: Stream,
    /// tool_use id -> (item id, tool name, input), until its tool_result.
    tools: HashMap<String, (String, String, Value)>,
    /// Permission item id -> the CLI's request.
    perms: HashMap<String, Perm>,
    always: HashSet<String>,
    usage: Usage,
    rate_limited: Option<Option<u64>>,
    pub unknown: u64,
}

impl Parser {
    pub fn new(policy: Policy, pid: Option<u32>) -> Parser {
        Parser {
            policy,
            pid,
            started: false,
            pending_turns: 0,
            text: Stream::default(),
            tools: HashMap::new(),
            perms: HashMap::new(),
            always: HashSet::new(),
            usage: Usage::default(),
            rate_limited: None,
            unknown: 0,
        }
    }

    /// Handles one stdout line; returns lines to write back to stdin.
    pub async fn line(&mut self, v: &Value, out: &mut Out) -> Vec<Value> {
        let s = |k: &str| v.get(k).and_then(Value::as_str);
        let sub = s("parent_tool_use_id").map(str::to_owned);
        match (s("type").unwrap_or(""), s("subtype").unwrap_or("")) {
            ("system", "init") if !self.started => {
                self.started = true;
                out.ev(Ev::Started { vendor_session: s("session_id").map(str::to_owned), pid: self.pid }).await;
            }
            ("system", "task_started") => {
                if let Some(tu) = s("tool_use_id") {
                    let title = s("description").unwrap_or("sub-agent").to_owned();
                    out.send(Some(tu.to_owned()), Ev::NativeChild { title }).await;
                    out.send(Some(tu.to_owned()), Ev::Status(Status::Running)).await;
                }
            }
            ("system", "task_notification") => {
                if let Some(tu) = s("tool_use_id") {
                    let status = match s("status") {
                        Some("completed") => Status::Done,
                        Some("failed") => Status::Failed,
                        _ => Status::Cancelled,
                    };
                    if let Some(summary) = s("summary") {
                        out.send(
                            Some(tu.to_owned()),
                            Ev::Item {
                                id: new_id(),
                                kind: ItemKind::AgentMessage,
                                state: ItemState::Complete,
                                body: json!({"text": summary}),
                            },
                        )
                        .await;
                    }
                    out.send(Some(tu.to_owned()), Ev::Status(status)).await;
                }
            }
            ("system", _) => {}
            ("stream_event", _) if sub.is_none() => self.stream(v.get("event").unwrap_or(&Value::Null), out).await,
            ("stream_event", _) => {}
            ("assistant", _) => {
                for block in v.pointer("/message/content").and_then(Value::as_array).into_iter().flatten() {
                    self.assistant_block(block, sub.clone(), out).await;
                }
            }
            ("user", _) => {
                for block in v.pointer("/message/content").and_then(Value::as_array).into_iter().flatten() {
                    if block.get("type").and_then(Value::as_str) == Some("tool_result") {
                        self.tool_result(block, sub.clone(), out).await;
                    }
                }
            }
            ("result", _) => self.result(v, out).await,
            ("rate_limit_event", _) => {
                let info = v.get("rate_limit_info").unwrap_or(&Value::Null);
                let resets = info.get("resetsAt").and_then(Value::as_u64);
                self.rate_limited = (info.get("status").and_then(Value::as_str) == Some("rejected")).then_some(resets);
                if let Some(r) = self.rate_limited {
                    out.ev(Ev::Status(Status::RateLimited { resets: r })).await;
                }
            }
            ("control_request", _) => return self.control_request(v, out).await,
            ("control_cancel_request", _) => {
                let rid = s("request_id").unwrap_or("");
                let id = self.perms.iter().find(|(_, p)| p.request_id == rid).map(|(k, _)| k.clone());
                if let Some(id) = id {
                    self.perms.remove(&id);
                    out.ev(Ev::Item { id, kind: ItemKind::Permission, state: ItemState::Failed, body: json!({"decision": "cancelled"}) })
                        .await;
                }
            }
            ("control_response", _) => {}
            _ => self.unknown += 1,
        }
        Vec::new()
    }

    async fn stream(&mut self, e: &Value, out: &mut Out) {
        match e.get("type").and_then(Value::as_str) {
            Some("content_block_delta") => match e.pointer("/delta/type").and_then(Value::as_str) {
                Some("text_delta") => {
                    let t = e.pointer("/delta/text").and_then(Value::as_str).unwrap_or("");
                    self.text.push(out, None, ItemKind::AgentMessage, t).await;
                }
                Some("thinking_delta") => {
                    // Usually empty on current models; only shown when the harness sends text.
                    let t = e.pointer("/delta/thinking").and_then(Value::as_str).unwrap_or("");
                    if !t.is_empty() {
                        self.text.push(out, None, ItemKind::Reasoning, t).await;
                    }
                }
                _ => {}
            },
            Some("content_block_start") if e.pointer("/content_block/type").and_then(Value::as_str) == Some("tool_use") => {
                let name = e.pointer("/content_block/name").and_then(Value::as_str).unwrap_or("tool");
                out.ev(Ev::Activity(name.to_owned())).await;
            }
            _ => {}
        }
    }

    async fn assistant_block(&mut self, b: &Value, sub: Option<String>, out: &mut Out) {
        match b.get("type").and_then(Value::as_str) {
            Some("text") => {
                let full = b.get("text").and_then(Value::as_str).unwrap_or("");
                if sub.is_none() && self.text.kind == Some(ItemKind::AgentMessage) {
                    // The block's text is authoritative; replace what deltas accumulated.
                    self.text.text = full.to_owned();
                    self.text.close(out, None).await;
                } else {
                    out.send(
                        sub,
                        Ev::Item { id: new_id(), kind: ItemKind::AgentMessage, state: ItemState::Complete, body: json!({"text": full}) },
                    )
                    .await;
                }
            }
            Some("thinking") => {
                if sub.is_none() {
                    self.text.close(out, None).await;
                }
            }
            Some("tool_use") => {
                if sub.is_none() {
                    self.text.close(out, None).await;
                }
                let name = b.get("name").and_then(Value::as_str).unwrap_or("tool").to_owned();
                let input = b.get("input").cloned().unwrap_or(Value::Null);
                let id = new_id();
                out.send(sub.clone(), Ev::Activity(describe(&name, &input))).await;
                out.send(
                    sub,
                    Ev::Item {
                        id: id.clone(),
                        kind: ItemKind::ToolCall,
                        state: ItemState::Streaming,
                        body: json!({"title": name, "input": input}),
                    },
                )
                .await;
                if let Some(tu) = b.get("id").and_then(Value::as_str) {
                    self.tools.insert(tu.to_owned(), (id, name, input));
                }
            }
            _ => self.unknown += 1,
        }
    }

    async fn tool_result(&mut self, b: &Value, sub: Option<String>, out: &mut Out) {
        let Some((id, name, input)) = b.get("tool_use_id").and_then(Value::as_str).and_then(|tu| self.tools.remove(tu)) else { return };
        let failed = b.get("is_error").and_then(Value::as_bool).unwrap_or(false);
        let output = match b.get("content") {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Array(a)) => a.iter().filter_map(|x| x.get("text").and_then(Value::as_str)).collect::<Vec<_>>().join("\n"),
            _ => String::new(),
        };
        let output: String = output.chars().take(4000).collect();
        if !failed && let Some(path) = input.get("file_path").and_then(Value::as_str) {
            let lines = |k: &str| input.get(k).and_then(Value::as_str).map_or(0, |t| t.lines().count() as u64);
            let (added, removed) = match name.as_str() {
                "Write" => (lines("content"), 0),
                "Edit" => (lines("new_string"), lines("old_string")),
                _ => (0, 0),
            };
            if matches!(name.as_str(), "Write" | "Edit" | "MultiEdit" | "NotebookEdit") {
                out.send(sub.clone(), Ev::Change { path: path.to_owned(), added, removed }).await;
            }
        }
        let state = if failed { ItemState::Failed } else { ItemState::Complete };
        out.send(sub, Ev::Item { id, kind: ItemKind::ToolCall, state, body: json!({"title": name, "input": input, "output": output}) })
            .await;
    }

    async fn result(&mut self, v: &Value, out: &mut Out) {
        self.text.close(out, None).await;
        let u = v.get("usage").unwrap_or(&Value::Null);
        let n = |k: &str| u.get(k).and_then(Value::as_u64).unwrap_or(0);
        // `usage` is per turn; `total_cost_usd` is already cumulative for the process.
        self.usage.input += n("input_tokens");
        self.usage.output += n("output_tokens");
        self.usage.cache_read += n("cache_read_input_tokens");
        self.usage.cache_write += n("cache_creation_input_tokens");
        if let Some(c) = v.get("total_cost_usd").and_then(Value::as_f64) {
            self.usage.reported_cost_usd = Some(c);
        }
        out.ev(Ev::Usage(self.usage.clone())).await;
        let terminal = v.get("terminal_reason").and_then(Value::as_str);
        if terminal == Some("aborted_streaming") {
            out.ev(Ev::Item { id: new_id(), kind: ItemKind::Notice, state: ItemState::Complete, body: json!({"text": "interrupted"}) })
                .await;
        } else if v.get("is_error").and_then(Value::as_bool) == Some(true) {
            let what = v.get("result").and_then(Value::as_str).or(v.get("subtype").and_then(Value::as_str)).unwrap_or("error");
            out.ev(Ev::Item { id: new_id(), kind: ItemKind::Error, state: ItemState::Complete, body: json!({"text": what}) }).await;
        }
        self.pending_turns = self.pending_turns.saturating_sub(1);
        if self.pending_turns == 0 {
            let status = match self.rate_limited {
                Some(resets) => Status::RateLimited { resets },
                None => Status::Idle,
            };
            out.ev(Ev::Status(status)).await;
        }
    }

    async fn control_request(&mut self, v: &Value, out: &mut Out) -> Vec<Value> {
        let rid = v.get("request_id").and_then(Value::as_str).unwrap_or("").to_owned();
        let r = v.get("request").unwrap_or(&Value::Null);
        if r.get("subtype").and_then(Value::as_str) != Some("can_use_tool") {
            return vec![
                json!({"type": "control_response", "response": {"subtype": "error", "request_id": rid, "error": "not supported by canopy"}}),
            ];
        }
        let tool = r.get("tool_name").and_then(Value::as_str).unwrap_or("").to_owned();
        let input = r.get("input").cloned().unwrap_or(json!({}));
        if self.policy == Policy::Yolo || READ_ONLY_TOOLS.contains(&tool.as_str()) || self.always.contains(&tool) {
            return vec![allow(&rid, &input)];
        }
        self.text.close(out, None).await;
        let id = new_id();
        let title = match r.get("description").and_then(Value::as_str) {
            Some(d) => format!("{tool}: {d}"),
            None => tool.clone(),
        };
        // Shown on the session itself: sub-agent asks (`agent_id`) are unverified (harness notes, Gaps).
        out.send(
            None,
            Ev::Item {
                id: id.clone(),
                kind: ItemKind::Permission,
                state: ItemState::Streaming,
                body: json!({"title": title, "tool": tool, "input": input}),
            },
        )
        .await;
        out.ev(Ev::Status(Status::AwaitingPermission)).await;
        self.perms.insert(id, Perm { request_id: rid, tool, input });
        Vec::new()
    }

    pub async fn decide(&mut self, request: &str, decision: Decision, out: &mut Out) -> Option<Value> {
        let p = self.perms.remove(request)?;
        if decision == Decision::AllowAlways {
            // Remembered for this session only; never written to the user's Claude settings.
            self.always.insert(p.tool.clone());
        }
        out.ev(Ev::Item {
            id: request.to_owned(),
            kind: ItemKind::Permission,
            state: ItemState::Complete,
            body: json!({"decision": decision, "tool": p.tool}),
        })
        .await;
        if self.perms.is_empty() {
            out.ev(Ev::Status(Status::Running)).await;
        }
        Some(match decision {
            Decision::Deny => deny(&p.request_id),
            _ => allow(&p.request_id, &p.input),
        })
    }
}

/// One line for the tree's activity column.
fn describe(tool: &str, input: &Value) -> String {
    let arg = ["file_path", "command", "pattern", "description", "url"].iter().find_map(|k| input.get(*k).and_then(Value::as_str));
    match arg {
        Some(a) => format!("{tool} {}", crate::daemon::hub::first_line(a, 60)),
        None => tool.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::hub::Msg;
    use crate::harness::HarnessEvent;

    /// Replays a recorded fixture's stdout lines; returns the events and the stdin replies.
    async fn replay(name: &str, policy: Policy) -> (Vec<HarnessEvent>, Vec<Value>) {
        let path = format!("{}/src/harness/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
        let text = std::fs::read_to_string(path).unwrap();
        let (tx, mut rx) = mpsc::channel(100_000);
        let mut out = Out::new(tx, "S".into());
        let mut p = Parser::new(policy, None);
        p.pending_turns = 1;
        let mut replies = Vec::new();
        for line in text.lines() {
            let v: Value = serde_json::from_str(line).unwrap();
            if v.get("canopy_stdin").is_none() {
                replies.extend(p.line(&v, &mut out).await);
            }
        }
        out.flush().await;
        drop(out);
        let mut evs = Vec::new();
        while let Some(Msg::Harness(e)) = rx.recv().await {
            evs.push(e);
        }
        (evs, replies)
    }

    fn text_of(evs: &[HarnessEvent], kind: ItemKind) -> Vec<String> {
        evs.iter()
            .filter_map(|e| match &e.ev {
                Ev::Item { kind: k, state: ItemState::Complete, body, .. } if *k == kind => body["text"].as_str().map(str::to_owned),
                _ => None,
            })
            .collect()
    }

    #[tokio::test]
    async fn simple_text_streams_and_completes() {
        let (evs, _) = replay("claude-2.1.318-simple-text.jsonl", Policy::Ask).await;
        assert!(matches!(&evs[0].ev, Ev::Started { vendor_session: Some(_), .. }));
        assert!(text_of(&evs, ItemKind::AgentMessage).iter().any(|t| t.to_lowercase().contains("ok")));
        assert!(evs.iter().any(|e| matches!(&e.ev, Ev::Usage(u) if u.output > 0 && u.reported_cost_usd.is_some())));
        assert!(matches!(evs.last().map(|e| &e.ev), Some(Ev::Status(Status::Idle))));
    }

    #[tokio::test]
    async fn ask_policy_surfaces_permission_then_allow_echoes_input() {
        let (tx, _rx) = mpsc::channel(100_000);
        let mut out = Out::new(tx, "S".into());
        let mut p = Parser::new(Policy::Ask, None);
        let path = format!("{}/src/harness/fixtures/claude-2.1.318-write-permission.jsonl", env!("CARGO_MANIFEST_DIR"));
        let req = std::fs::read_to_string(path)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str::<Value>(l).unwrap())
            .find(|v| v["type"] == "control_request")
            .unwrap();
        assert!(p.line(&req, &mut out).await.is_empty(), "ask policy must not auto-answer");
        let id = p.perms.keys().next().unwrap().clone();
        let reply = p.decide(&id, Decision::Allow, &mut out).await.unwrap();
        assert_eq!(reply["response"]["request_id"], req["request_id"]);
        assert_eq!(reply["response"]["response"]["updatedInput"], req["request"]["input"]);
        assert!(p.decide(&id, Decision::Allow, &mut out).await.is_none(), "a decision applies once");
    }

    #[tokio::test]
    async fn yolo_answers_permission_without_asking() {
        let (evs, replies) = replay("claude-2.1.318-write-permission.jsonl", Policy::Yolo).await;
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0]["response"]["response"]["behavior"], "allow");
        assert!(!evs.iter().any(|e| matches!(&e.ev, Ev::Item { kind: ItemKind::Permission, .. })));
        assert!(evs.iter().any(|e| matches!(&e.ev, Ev::Change { added: 1, .. })));
    }

    #[tokio::test]
    async fn subagent_becomes_native_child() {
        let (evs, _) = replay("claude-2.1.318-subagent.jsonl", Policy::Yolo).await;
        let child = evs.iter().find(|e| matches!(e.ev, Ev::NativeChild { .. })).expect("native child");
        let key = child.sub.clone();
        assert!(key.is_some());
        assert!(evs.iter().any(|e| e.sub == key && matches!(e.ev, Ev::Status(Status::Done))));
        assert!(evs.iter().any(|e| e.sub == key && matches!(&e.ev, Ev::Item { kind: ItemKind::AgentMessage, .. })));
    }

    #[tokio::test]
    async fn interrupt_is_a_notice_not_an_error() {
        let (evs, _) = replay("claude-2.1.318-interrupt.jsonl", Policy::Ask).await;
        assert!(text_of(&evs, ItemKind::Notice).contains(&"interrupted".to_owned()));
        assert!(text_of(&evs, ItemKind::Error).is_empty());
    }

    #[test]
    fn user_messages_are_client_composed() {
        assert_eq!(user("/help")["client_composed"], true);
    }
}
