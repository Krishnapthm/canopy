//! Codex adapter: one shared `codex app-server` process; one thread per canopy session.
//! Wire facts: `docs/harness-notes/codex.md` (0.165.0). Handshake, threads, and per-thread
//! MCP are verified live; turn and approval shapes come from the CLI's generated schema only.

use super::{Command, Ev, Out, SpawnSpec, Stream, new_id};
use crate::daemon::hub::MAX_WAIT_S;
use crate::jsonrpc::{Incoming, Peer};
use anyhow::{Context, Result, anyhow};
use protocol::{Decision, Delivery, ItemKind, ItemState, Policy, Status, Usage};
use serde_json::{Value, json};
use std::collections::HashMap;
use tokio::sync::mpsc;

type Route = mpsc::UnboundedSender<Incoming>;

enum Ctl {
    Attach(Box<SpawnSpec>, Out, mpsc::Receiver<Command>),
    Route(String, Route),
    Unroute(String),
}

/// Handle to the shared app-server. Created on the first Codex session.
pub struct Server {
    ctl: mpsc::UnboundedSender<Ctl>,
    pid: Option<u32>,
}

impl Server {
    pub fn start(tx: mpsc::Sender<crate::daemon::hub::Msg>) -> Server {
        let (ctl, rx) = mpsc::unbounded_channel();
        let child = super::command("codex", &["app-server".to_owned()], &std::env::temp_dir()).spawn();
        let pid = child.as_ref().ok().and_then(|c| c.id());
        let me = ctl.clone();
        let (reason_tx, reason_rx) = tokio::sync::watch::channel(None);
        tokio::spawn(async move {
            let error = serve(child, rx, me, reason_rx).await;
            let _ = reason_tx.send(Some(error.clone()));
            let _ = tx.send(crate::daemon::hub::Msg::CodexExited(error)).await;
        });
        Server { ctl, pid }
    }

    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    pub fn attach(&self, spec: SpawnSpec, out: Out, cmds: mpsc::Receiver<Command>) {
        if let Err(e) = self.ctl.send(Ctl::Attach(Box::new(spec), out, cmds))
            && let Ctl::Attach(_, mut out, _) = e.0
        {
            tokio::spawn(async move {
                out.ev(Ev::Exited { error: Some("codex app-server exited".into()) }).await;
            });
        }
    }
}

/// Owns the process and routes every incoming message to its thread's session task.
async fn serve(
    child: std::io::Result<tokio::process::Child>,
    mut ctl: mpsc::UnboundedReceiver<Ctl>,
    me: mpsc::UnboundedSender<Ctl>,
    reason: tokio::sync::watch::Receiver<Option<String>>,
) -> String {
    let mut child = match child {
        Ok(c) => c,
        Err(e) => return format!("cannot start codex app-server: {e}"),
    };
    let tail = super::stderr(&mut child);
    let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
        return "codex app-server has no stdio".into();
    };
    let (peer, mut incoming) = Peer::spawn(stdout, stdin, false);
    let init = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        peer.request("initialize", json!({"clientInfo": {"name": "canopy", "version": env!("CARGO_PKG_VERSION")}})),
    )
    .await;
    if !matches!(init, Ok(Ok(_))) {
        let reason = match init {
            Ok(Err(e)) => format!("codex app-server initialize failed: {e:#}"),
            _ => "codex app-server initialize timed out".into(),
        };
        let _ = child.kill().await;
        return super::exit_error("codex app-server", &mut child, tail, Some(reason.clone())).await.unwrap_or(reason);
    }
    peer.notify("initialized", json!({}));
    let mut routes: HashMap<String, Route> = HashMap::new();
    loop {
        tokio::select! {
            msg = incoming.recv() => {
                let Some(msg) = msg else { break };
                let params = match &msg { Incoming::Request { params, .. } | Incoming::Notification { params, .. } => params };
                match params.get("threadId").and_then(Value::as_str).and_then(|t| routes.get(t)) {
                    Some(r) => { let _ = r.send(msg); }
                    None => if let Incoming::Request { id, .. } = msg {
                        peer.respond_err(id, -32601, "no canopy session for this thread");
                    },
                }
            }
            c = ctl.recv() => match c {
                Some(Ctl::Attach(spec, out, cmds)) => {
                    tokio::spawn(session(peer.clone(), *spec, out, cmds, me.clone(), reason.clone()));
                }
                Some(Ctl::Route(t, r)) => { routes.insert(t, r); }
                Some(Ctl::Unroute(t)) => { routes.remove(&t); }
                None => break,
            },
        }
    }
    // Dropping the routes closes every session's channel, so each reports it exited.
    drop(routes);
    let _ = child.kill().await;
    super::exit_error("codex app-server", &mut child, tail, Some("codex app-server exited".into()))
        .await
        .unwrap_or_else(|| "codex app-server exited".into())
}

fn policy(p: Policy) -> (&'static str, &'static str) {
    match p {
        Policy::Ask => ("untrusted", "workspace-write"),
        Policy::AutoEdit => ("on-request", "workspace-write"),
        Policy::Yolo => ("never", "danger-full-access"),
    }
}

async fn session(
    peer: Peer,
    spec: SpawnSpec,
    mut out: Out,
    mut cmds: mpsc::Receiver<Command>,
    ctl: mpsc::UnboundedSender<Ctl>,
    mut reason: tokio::sync::watch::Receiver<Option<String>>,
) {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let r = async {
        let (approval, sandbox) = policy(spec.policy);
        let mut params = json!({
            "cwd": spec.cwd, "model": spec.model, "approvalPolicy": approval, "sandbox": sandbox,
            "config": {
                "mcp_servers.canopy.command": spec.mcp.0,
                "mcp_servers.canopy.args": spec.mcp.1,
                // wait_agent blocks up to MAX_WAIT_S; Codex must not time the call out first.
                "mcp_servers.canopy.tool_timeout_sec": MAX_WAIT_S + 30,
            }
        });
        let method = match &spec.resume {
            Some(t) => {
                params["threadId"] = json!(t);
                "thread/resume"
            }
            None => "thread/start",
        };
        let res = peer.request(method, params).await.with_context(|| format!("codex {method}"))?;
        let thread =
            res.pointer("/thread/id").and_then(Value::as_str).ok_or_else(|| anyhow!("codex {method} returned no thread id"))?.to_owned();
        let _ = ctl.send(Ctl::Route(thread.clone(), tx));
        out.ev(Ev::Started { vendor_session: Some(thread.clone()), pid: None }).await;
        let mut t = Thread::new(peer, thread, spec.policy);
        if !spec.prompt.is_empty() {
            t.start_turn(&spec.prompt, &mut out).await?;
        }
        t.run(&mut out, &mut rx, &mut cmds).await
    }
    .await;
    if let Some(t) = &r.as_ref().ok().and_then(|t| t.clone()) {
        let _ = ctl.send(Ctl::Unroute(t.clone()));
    }
    let mut error = r.err().map(|e| format!("{e:#}"));
    if error.as_ref().is_some_and(|e| e.contains("peer closed") || e.contains("app-server exited")) {
        let _ = reason.wait_for(|r| r.is_some()).await;
        error = reason.borrow().clone().or(error);
    }
    out.ev(Ev::Exited { error }).await;
}

/// One Codex thread. Separate from the I/O so tests can drive it with schema-shaped messages.
pub struct Thread {
    peer: Peer,
    thread: String,
    policy: Policy,
    turn: Option<String>,
    queued: Vec<String>,
    text: Stream,
    /// Codex item id -> our item id.
    items: HashMap<String, String>,
    perms: HashMap<String, Value>,
    rate_limited: bool,
}

impl Thread {
    fn new(peer: Peer, thread: String, policy: Policy) -> Thread {
        Thread {
            peer,
            thread,
            policy,
            turn: None,
            queued: Vec::new(),
            text: Stream::default(),
            items: HashMap::new(),
            perms: HashMap::new(),
            rate_limited: false,
        }
    }

    async fn start_turn(&mut self, text: &str, out: &mut Out) -> Result<()> {
        let r = self.peer.request("turn/start", json!({"threadId": self.thread, "input": [{"type": "text", "text": text}]})).await?;
        self.turn = r.pointer("/turn/id").and_then(Value::as_str).map(str::to_owned);
        self.rate_limited = false;
        out.ev(Ev::Status(Status::Running)).await;
        Ok(())
    }

    /// Runs until stopped; returns the thread id so the route can be removed.
    async fn run(
        &mut self,
        out: &mut Out,
        rx: &mut mpsc::UnboundedReceiver<Incoming>,
        cmds: &mut mpsc::Receiver<Command>,
    ) -> Result<Option<String>> {
        loop {
            tokio::select! {
                msg = rx.recv() => match msg {
                    Some(Incoming::Notification { method, params }) => self.notification(&method, &params, out).await?,
                    Some(Incoming::Request { id, method, params }) => self.request(id, &method, &params, out).await,
                    None => return Err(anyhow!("codex app-server exited")),
                },
                cmd = cmds.recv() => match cmd {
                    Some(Command::Send { text, interrupt, reply }) => {
                        let d = self.send(text, interrupt, out).await?;
                        let _ = reply.send(d);
                    }
                    Some(Command::Interrupt) => self.interrupt().await,
                    Some(Command::Permission { request, decision }) => self.decide(&request, decision, out).await,
                    Some(Command::Stop) | None => {
                        self.interrupt().await;
                        return Ok(Some(self.thread.clone()));
                    }
                },
                _ = out.due() => out.flush().await,
            }
        }
    }

    async fn send(&mut self, text: String, interrupt: bool, out: &mut Out) -> Result<Delivery> {
        let Some(turn) = self.turn.clone() else {
            self.start_turn(&text, out).await?;
            return Ok(Delivery::StartedTurn);
        };
        if !interrupt {
            let steer = json!({"threadId": self.thread, "expectedTurnId": turn, "input": [{"type": "text", "text": text}]});
            if self.peer.request("turn/steer", steer).await.is_ok() {
                return Ok(Delivery::Steered);
            }
        } else {
            self.interrupt().await;
        }
        // Not steerable (or interrupting): delivered as a new turn when this one ends.
        self.queued.push(text);
        Ok(Delivery::Queued)
    }

    async fn interrupt(&self) {
        if let Some(turn) = &self.turn {
            let _ = self.peer.request("turn/interrupt", json!({"threadId": self.thread, "turnId": turn})).await;
        }
    }

    async fn notification(&mut self, method: &str, p: &Value, out: &mut Out) -> Result<()> {
        let item = p.get("item").unwrap_or(&Value::Null);
        let s = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_owned();
        match method {
            "item/agentMessage/delta" => {
                let d = s(p, "delta");
                self.text.push(out, None, ItemKind::AgentMessage, &d).await;
            }
            "item/reasoning/summaryTextDelta" | "item/reasoning/textDelta" => {
                let d = s(p, "delta");
                self.text.push(out, None, ItemKind::Reasoning, &d).await;
            }
            "item/started" => match s(item, "type").as_str() {
                "commandExecution" | "mcpToolCall" | "fileChange" => {
                    self.text.close(out, None).await;
                    let title = tool_title(item);
                    let id = new_id();
                    self.items.insert(s(item, "id"), id.clone());
                    out.ev(Ev::Activity(title.clone())).await;
                    out.ev(Ev::Item {
                        id,
                        kind: ItemKind::ToolCall,
                        state: ItemState::Streaming,
                        body: json!({"title": title, "input": item}),
                    })
                    .await;
                }
                _ => {}
            },
            "item/completed" => match s(item, "type").as_str() {
                "agentMessage" => {
                    if self.text.kind == Some(ItemKind::AgentMessage) {
                        self.text.text = s(item, "text");
                        self.text.close(out, None).await;
                    } else {
                        out.ev(Ev::Item {
                            id: new_id(),
                            kind: ItemKind::AgentMessage,
                            state: ItemState::Complete,
                            body: json!({"text": s(item, "text")}),
                        })
                        .await;
                    }
                }
                "reasoning" => self.text.close(out, None).await,
                kind @ ("commandExecution" | "mcpToolCall" | "fileChange") => {
                    let status = s(item, "status");
                    let ok = status == "completed";
                    if kind == "fileChange" && ok {
                        for c in item.get("changes").and_then(Value::as_array).into_iter().flatten() {
                            let diff = s(c, "diff");
                            let count = |pre: char| {
                                diff.lines().filter(|l| l.starts_with(pre) && !l.starts_with("+++") && !l.starts_with("---")).count() as u64
                            };
                            out.ev(Ev::Change { path: s(c, "path"), added: count('+'), removed: count('-') }).await;
                        }
                    }
                    let id = self.items.remove(&s(item, "id")).unwrap_or_else(new_id);
                    let state = if ok { ItemState::Complete } else { ItemState::Failed };
                    let output = item.get("aggregatedOutput").or(item.get("result")).or(item.get("error")).cloned();
                    out.ev(Ev::Item {
                        id,
                        kind: ItemKind::ToolCall,
                        state,
                        body: json!({"title": tool_title(item), "status": status, "output": output}),
                    })
                    .await;
                }
                _ => {}
            },
            "thread/tokenUsage/updated" => {
                let t = p.pointer("/tokenUsage/total").unwrap_or(&Value::Null);
                let n = |k: &str| t.get(k).and_then(Value::as_u64).unwrap_or(0);
                out.ev(Ev::Usage(Usage {
                    input: n("inputTokens"),
                    output: n("outputTokens"),
                    cache_read: n("cachedInputTokens"),
                    cache_write: n("cacheWriteInputTokens"),
                    reported_cost_usd: None,
                }))
                .await;
            }
            "error" if p.get("willRetry").and_then(Value::as_bool) != Some(true) => {
                let info = p.pointer("/error/codexErrorInfo");
                if info.and_then(Value::as_str).is_some_and(|i| i == "usageLimitExceeded" || i == "rateLimitExceeded") {
                    self.rate_limited = true;
                }
                let msg = p.pointer("/error/message").and_then(Value::as_str).unwrap_or("codex error");
                out.ev(Ev::Item { id: new_id(), kind: ItemKind::Error, state: ItemState::Complete, body: json!({"text": msg}) }).await;
            }
            "turn/completed" => {
                self.text.close(out, None).await;
                for (req, _) in self.perms.drain() {
                    out.ev(Ev::Item {
                        id: req,
                        kind: ItemKind::Permission,
                        state: ItemState::Failed,
                        body: json!({"decision": "cancelled"}),
                    })
                    .await;
                }
                if p.pointer("/turn/status").and_then(Value::as_str) == Some("interrupted") {
                    out.ev(Ev::Item {
                        id: new_id(),
                        kind: ItemKind::Notice,
                        state: ItemState::Complete,
                        body: json!({"text": "interrupted"}),
                    })
                    .await;
                }
                self.turn = None;
                if self.queued.is_empty() {
                    // shortcut: no reset time; codex rate-limit resetsAt units are unverified (harness notes, Gaps).
                    let st = if self.rate_limited { Status::RateLimited { resets: None } } else { Status::Idle };
                    out.ev(Ev::Status(st)).await;
                } else {
                    let text = std::mem::take(&mut self.queued).join("\n\n");
                    self.start_turn(&text, out).await?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn request(&mut self, id: Value, method: &str, p: &Value, out: &mut Out) {
        if !matches!(method, "item/commandExecution/requestApproval" | "item/fileChange/requestApproval") {
            return self.peer.respond_err(id, -32601, "not supported by canopy");
        }
        let title = match p.get("command").and_then(Value::as_str) {
            Some(c) => format!("run: {c}"),
            None => p.get("reason").and_then(Value::as_str).map_or("apply file changes".into(), |r| format!("file change: {r}")),
        };
        // Yolo threads run with approvalPolicy "never", so this is only a backstop.
        if self.policy == Policy::Yolo {
            return self.peer.respond(id, json!({"decision": "accept"}));
        }
        self.text.close(out, None).await;
        let req = new_id();
        out.ev(Ev::Item {
            id: req.clone(),
            kind: ItemKind::Permission,
            state: ItemState::Streaming,
            body: json!({"title": title, "request": p}),
        })
        .await;
        out.ev(Ev::Status(Status::AwaitingPermission)).await;
        self.perms.insert(req, id);
    }

    async fn decide(&mut self, request: &str, decision: Decision, out: &mut Out) {
        let Some(id) = self.perms.remove(request) else { return };
        let d = match decision {
            Decision::Allow => "accept",
            Decision::AllowAlways => "acceptForSession",
            Decision::Deny => "decline",
        };
        self.peer.respond(id, json!({"decision": d}));
        out.ev(Ev::Item {
            id: request.to_owned(),
            kind: ItemKind::Permission,
            state: ItemState::Complete,
            body: json!({"decision": decision}),
        })
        .await;
        if self.perms.is_empty() {
            out.ev(Ev::Status(Status::Running)).await;
        }
    }
}

fn tool_title(item: &Value) -> String {
    let s = |k: &str| item.get(k).and_then(Value::as_str);
    match s("type") {
        Some("commandExecution") => format!("run {}", crate::daemon::hub::first_line(s("command").unwrap_or(""), 60)),
        Some("mcpToolCall") => format!("{}.{}", s("server").unwrap_or("mcp"), s("tool").unwrap_or("tool")),
        Some("fileChange") => {
            let n = item.get("changes").and_then(Value::as_array).map_or(0, Vec::len);
            format!("edit {n} file(s)")
        }
        _ => "tool".into(),
    }
}
