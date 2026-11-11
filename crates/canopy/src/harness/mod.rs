//! The contract between the hub and the per-vendor adapters, plus the delta coalescer that
//! every adapter uses (ADR 0006). See `AGENTS.md` in this directory for adapter rules.

pub mod acp;
pub mod claude;
pub mod codex;

use protocol::{Decision, Delivery, ItemKind, ItemState, Policy, SessionId, Status, Usage};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;

use crate::daemon::hub::Msg;

/// How long token deltas are held before being forwarded as one delta.
pub const COALESCE: Duration = Duration::from_millis(40);

/// Which adapter drives a harness name.
#[derive(Clone, Debug)]
pub enum Kind {
    Claude,
    Codex,
    Acp { command: String, args: Vec<String> },
}

pub struct SpawnSpec {
    pub session: SessionId,
    pub prompt: String,
    pub cwd: PathBuf,
    pub model: Option<String>,
    pub policy: Policy,
    /// The vendor session id to resume, if any.
    pub resume: Option<String>,
    /// Command and args of the canopy MCP shim for this session.
    pub mcp: (String, Vec<String>),
}

pub enum Command {
    Send { text: String, interrupt: bool, reply: oneshot::Sender<Delivery> },
    Interrupt,
    Permission { request: String, decision: Decision },
    Stop,
}

/// What an adapter reports. `sub` names a native sub-agent of the session (Claude Task
/// sub-agents); the hub turns each distinct `sub` into a child node of origin `native`.
#[derive(Debug)]
pub struct HarnessEvent {
    pub session: SessionId,
    pub sub: Option<String>,
    pub ev: Ev,
}

#[derive(Debug)]
pub enum Ev {
    /// `pid` is the process group to kill when the daemon exits.
    Started {
        vendor_session: Option<String>,
        pid: Option<u32>,
    },
    /// A full item. Streaming items are followed by deltas, then the same id with state
    /// `Complete` and the full body.
    Item {
        id: String,
        kind: ItemKind,
        state: ItemState,
        body: Value,
    },
    Delta {
        id: String,
        text: String,
    },
    Status(Status),
    /// Totals for this session, not increments.
    Usage(Usage),
    Activity(String),
    Change {
        path: String,
        added: u64,
        removed: u64,
    },
    /// A native sub-agent appeared under this session.
    NativeChild {
        title: String,
    },
    Exited {
        error: Option<String>,
    },
}

/// Event sink for one session. Buffers deltas for `COALESCE` and flushes them before any
/// other event, so the hub sees events in adapter order with ~20x fewer deltas.
pub struct Out {
    tx: mpsc::Sender<Msg>,
    pub session: SessionId,
    buf: Vec<(Option<String>, String, String)>,
    deadline: Option<Instant>,
}

impl Out {
    pub fn new(tx: mpsc::Sender<Msg>, session: SessionId) -> Out {
        Out { tx, session, buf: Vec::new(), deadline: None }
    }

    pub async fn send(&mut self, sub: Option<String>, ev: Ev) {
        self.flush().await;
        self.raw(sub, ev).await;
    }

    pub async fn ev(&mut self, ev: Ev) {
        self.send(None, ev).await;
    }

    async fn raw(&self, sub: Option<String>, ev: Ev) {
        let _ = self.tx.send(Msg::Harness(HarnessEvent { session: self.session.clone(), sub, ev })).await;
    }

    pub fn delta(&mut self, sub: Option<String>, id: &str, text: &str) {
        if text.is_empty() {
            return;
        }
        match self.buf.last_mut() {
            Some((s, i, t)) if *s == sub && i == id => t.push_str(text),
            _ => self.buf.push((sub, id.to_owned(), text.to_owned())),
        }
        self.deadline.get_or_insert_with(|| Instant::now() + COALESCE);
    }

    pub async fn flush(&mut self) {
        self.deadline = None;
        for (sub, id, text) in std::mem::take(&mut self.buf) {
            self.raw(sub, Ev::Delta { id, text }).await;
        }
    }

    /// Resolves when buffered deltas are due; never resolves when nothing is buffered, so an
    /// idle adapter has no timer (ADR 0003).
    pub async fn due(&self) {
        match self.deadline {
            Some(d) => tokio::time::sleep_until(d).await,
            None => std::future::pending().await,
        }
    }
}

/// Accumulates one streaming text item so the `Complete` event can carry the full text.
#[derive(Default)]
pub struct Stream {
    pub id: Option<String>,
    pub kind: Option<ItemKind>,
    pub text: String,
}

impl Stream {
    /// Appends text to the open item of `kind`, opening one first if needed.
    pub async fn push(&mut self, out: &mut Out, sub: Option<String>, kind: ItemKind, text: &str) {
        if self.kind.as_ref() != Some(&kind) {
            self.close(out, sub.clone()).await;
        }
        if self.id.is_none() {
            let id = new_id();
            out.send(sub.clone(), Ev::Item { id: id.clone(), kind: kind.clone(), state: ItemState::Streaming, body: json!({"text": ""}) })
                .await;
            self.id = Some(id);
            self.kind = Some(kind);
        }
        if let Some(id) = &self.id {
            out.delta(sub, id, text);
        }
        self.text.push_str(text);
    }

    pub async fn close(&mut self, out: &mut Out, sub: Option<String>) {
        if let (Some(id), Some(kind)) = (self.id.take(), self.kind.take()) {
            let text = std::mem::take(&mut self.text);
            out.send(sub, Ev::Item { id, kind, state: ItemState::Complete, body: json!({"text": text}) }).await;
        }
    }
}

pub fn new_id() -> String {
    ulid::Ulid::generate().to_string()
}

/// Starts the adapter for `kind` and returns its command channel. Never blocks: failures
/// arrive later as `Ev::Exited`.
pub fn start(kind: &Kind, spec: SpawnSpec, tx: mpsc::Sender<Msg>, codex: &mut Option<codex::Server>) -> mpsc::Sender<Command> {
    let (cmd_tx, cmd_rx) = mpsc::channel(64);
    let out = Out::new(tx.clone(), spec.session.clone());
    match kind {
        Kind::Claude => {
            tokio::spawn(claude::run(spec, out, cmd_rx));
        }
        Kind::Codex => {
            let server = codex.get_or_insert_with(codex::Server::start);
            server.attach(spec, out, cmd_rx);
        }
        Kind::Acp { command, args } => {
            tokio::spawn(acp::run(command.clone(), args.clone(), spec, out, cmd_rx));
        }
    }
    cmd_tx
}

/// Kills a child's whole process group: agents spawn tools and shims that must die with it.
pub fn kill_group(pid: Option<u32>) {
    if let Some(pid) = pid {
        // shortcut: shells out to kill(1) instead of adding libc; fine at human stop rates.
        let _ = std::process::Command::new("kill").args(["-TERM", &format!("-{pid}")]).status();
    }
}

/// A `tokio::process::Command` in its own process group with piped stdio.
pub fn command(program: &str, args: &[String], cwd: &std::path::Path) -> tokio::process::Command {
    let mut c = tokio::process::Command::new(program);
    c.args(args)
        .current_dir(cwd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .process_group(0)
        .kill_on_drop(true);
    c
}
