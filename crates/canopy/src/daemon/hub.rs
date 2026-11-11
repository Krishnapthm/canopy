//! The hub: the single owner of all daemon state (ADR 0003). Everything else talks to it
//! through `Msg` on one channel. It never awaits anything slow; replies that depend on an
//! adapter are finished by small tasks that write to the client directly.

use super::store::Store;
use crate::harness::{self, Command, Ev, HarnessEvent, Kind, SpawnSpec, codex};
use protocol::{
    Changes, Decision, Delivery, Event, Item, ItemKind, ItemState, Node, Op, Origin, Request, Rollup, ServerMsg, SessionId, SpawnReq,
    Status, Usage,
};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

/// Completed items kept in memory per session for `sub_session` tails.
const TAIL: usize = 200;
/// Outbound batching window per client (ADR 0005).
const FLUSH: Duration = Duration::from_millis(16);
/// The daemon exits after this long with no clients and no live sessions.
const IDLE_EXIT: Duration = Duration::from_secs(600);
/// `wait_agent` never blocks longer than this, below every harness's MCP tool timeout.
pub const MAX_WAIT_S: u64 = 240;

pub type Reply = oneshot::Sender<Result<Value, String>>;

pub enum Msg {
    Harness(HarnessEvent),
    Connect {
        conn: u64,
        tx: mpsc::Sender<String>,
    },
    Disconnect {
        conn: u64,
    },
    Client {
        conn: u64,
        req: Request,
    },
    /// A request line that did not parse; `id` is 0 when even that was unreadable.
    BadRequest {
        conn: u64,
        id: u64,
        error: String,
    },
    Mcp {
        caller: SessionId,
        call: McpCall,
        reply: Reply,
    },
    Loaded {
        session: SessionId,
        items: Vec<Item>,
    },
    Flush,
    IdleCheck(u64),
    Shutdown,
}

pub enum McpCall {
    Spawn { harness: String, prompt: String, title: Option<String>, model: Option<String>, cwd: Option<String> },
    Wait { ids: Vec<String> },
    Message { id: String, text: String },
    List,
}

pub struct Config {
    pub harnesses: HashMap<String, Kind>,
    pub max_depth: usize,
    pub max_live: usize,
    pub root_budget: Option<u64>,
    /// Path of this binary, used to launch the MCP shim.
    pub exe: String,
    pub sock: PathBuf,
}

struct Session {
    node: Node,
    cmd: Option<mpsc::Sender<Command>>,
    pid: Option<u32>,
    /// Recent complete items plus any still streaming.
    items: VecDeque<Item>,
    seq: u64,
    /// False for sessions from a previous daemon run until their tail is read from SQLite.
    loaded: bool,
    loading: Vec<(u64, u64, Option<u64>, usize)>,
    stop_requested: bool,
    /// The parent has been told this child settled (by notice or `wait_agent`).
    reported: bool,
    natives: HashMap<String, SessionId>,
    last_message: String,
}

struct Client {
    tx: mpsc::Sender<String>,
    tree: bool,
    subs: HashSet<SessionId>,
    pending: Vec<Event>,
}

struct Waiter {
    ids: Vec<SessionId>,
    reply: Reply,
}

pub struct Hub {
    cfg: Config,
    tx: mpsc::Sender<Msg>,
    store: Store,
    sessions: HashMap<SessionId, Session>,
    children: HashMap<SessionId, Vec<SessionId>>,
    clients: HashMap<u64, Client>,
    waiters: Vec<Waiter>,
    dirty: HashSet<SessionId>,
    flush_armed: bool,
    idle_gen: u64,
    idle_armed: bool,
    codex: Option<codex::Server>,
}

pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

impl Hub {
    pub fn new(cfg: Config, tx: mpsc::Sender<Msg>, store: Store, restored: Vec<(Node, u64)>) -> Hub {
        let mut hub = Hub {
            cfg,
            tx,
            store,
            sessions: HashMap::new(),
            children: HashMap::new(),
            clients: HashMap::new(),
            waiters: Vec::new(),
            dirty: HashSet::new(),
            flush_armed: false,
            idle_gen: 0,
            idle_armed: false,
            codex: None,
        };
        for (mut node, seq) in restored {
            // Every process died with the previous daemon.
            if !node.status.is_terminal() {
                node.status = Status::Interrupted;
                node.ended.get_or_insert(now());
                hub.store.node(&node);
            }
            if let Some(p) = &node.parent {
                hub.children.entry(p.clone()).or_default().push(node.id.clone());
            }
            hub.sessions.insert(node.id.clone(), Session::new(node, None, seq, false));
        }
        let ids: Vec<_> = hub.sessions.keys().cloned().collect();
        for id in ids {
            hub.rollup(&id);
        }
        hub
    }

    /// Runs until shutdown. Returns the pids whose process groups must be killed.
    pub async fn run(mut self, mut rx: mpsc::Receiver<Msg>) -> Vec<u32> {
        self.check_idle();
        while let Some(msg) = rx.recv().await {
            match msg {
                Msg::Harness(e) => self.harness(e),
                Msg::Connect { conn, tx } => {
                    self.clients.insert(conn, Client { tx, tree: false, subs: HashSet::new(), pending: Vec::new() });
                }
                Msg::Disconnect { conn } => {
                    self.clients.remove(&conn);
                }
                Msg::Client { conn, req } => self.request(conn, req),
                Msg::BadRequest { conn, id, error } => self.reply(conn, id, Err(error)),
                Msg::Mcp { caller, call, reply } => self.mcp(caller, call, reply),
                Msg::Loaded { session, items } => self.loaded(session, items),
                Msg::Flush => self.flush(),
                Msg::IdleCheck(g) if g == self.idle_gen && self.is_idle() => break,
                Msg::IdleCheck(_) => self.idle_armed = false,
                Msg::Shutdown => break,
            }
            self.check_idle();
        }
        let mut pids: Vec<u32> = self.sessions.values().filter_map(|s| s.pid).collect();
        pids.extend(self.codex.as_ref().and_then(codex::Server::pid));
        self.store.shutdown();
        pids
    }

    fn is_idle(&self) -> bool {
        self.clients.is_empty() && self.sessions.values().all(|s| s.cmd.is_none())
    }

    /// Arms one exit timer when the daemon becomes idle; any activity invalidates it.
    fn check_idle(&mut self) {
        if !self.is_idle() {
            self.idle_gen += 1;
            self.idle_armed = false;
        } else if !self.idle_armed {
            self.idle_armed = true;
            let (tx, g) = (self.tx.clone(), self.idle_gen);
            tokio::spawn(async move {
                tokio::time::sleep(IDLE_EXIT).await;
                let _ = tx.send(Msg::IdleCheck(g)).await;
            });
        }
    }

    // ---- outbound -------------------------------------------------------------------------

    fn send_now(&mut self, conn: u64, msg: &ServerMsg) {
        let Some(c) = self.clients.get(&conn) else { return };
        let line = serde_json::to_string(msg).unwrap_or_default();
        // A full queue means a stuck client: drop it, it will reconnect and resync.
        if c.tx.try_send(line).is_err() {
            self.clients.remove(&conn);
        }
    }

    fn reply(&mut self, conn: u64, id: u64, r: Result<Value, String>) {
        let msg = match r {
            Ok(result) => ServerMsg::Res { id, result },
            Err(error) => ServerMsg::Err { id, error },
        };
        self.send_now(conn, &msg);
    }

    fn arm_flush(&mut self) {
        if !self.flush_armed {
            self.flush_armed = true;
            let tx = self.tx.clone();
            tokio::spawn(async move {
                tokio::time::sleep(FLUSH).await;
                let _ = tx.send(Msg::Flush).await;
            });
        }
    }

    fn emit(&mut self, session: &str, ev: Event) {
        let mut any = false;
        for c in self.clients.values_mut().filter(|c| c.subs.contains(session)) {
            c.pending.push(ev.clone());
            any = true;
        }
        if any {
            self.arm_flush();
        }
    }

    fn touch(&mut self, id: &str) {
        if let Some(s) = self.sessions.get_mut(id) {
            s.node.updated = now();
        }
        self.dirty.insert(id.to_owned());
        if self.clients.values().any(|c| c.tree) {
            self.arm_flush();
        }
    }

    fn flush(&mut self) {
        self.flush_armed = false;
        let nodes: Vec<Event> =
            self.dirty.drain().filter_map(|id| self.sessions.get(&id).map(|s| Event::Node { node: Box::new(s.node.clone()) })).collect();
        let mut dead = Vec::new();
        for (conn, c) in &mut self.clients {
            let mut events = std::mem::take(&mut c.pending);
            if c.tree {
                events.extend(nodes.iter().cloned());
            }
            if events.is_empty() {
                continue;
            }
            let line = serde_json::to_string(&ServerMsg::Batch { events }).unwrap_or_default();
            if c.tx.try_send(line).is_err() {
                dead.push(*conn);
            }
        }
        for conn in dead {
            self.clients.remove(&conn);
        }
    }

    // ---- client requests ------------------------------------------------------------------

    fn request(&mut self, conn: u64, req: Request) {
        let id = req.id;
        let r = match req.op {
            Op::Hello { version } if version != protocol::VERSION => {
                Err(format!("protocol version {version} does not match daemon version {}", protocol::VERSION))
            }
            Op::Hello { .. } => {
                let mut names: Vec<_> = self.cfg.harnesses.keys().cloned().collect();
                names.sort();
                Ok(json!({"daemon_version": env!("CARGO_PKG_VERSION"), "harnesses": names}))
            }
            Op::Detect => {
                if let Some(client) = self.clients.get(&conn) {
                    let tx = client.tx.clone();
                    tokio::spawn(async move {
                        let rows = harness::detect().await;
                        let msg = ServerMsg::Res { id, result: json!({"harnesses": rows}) };
                        let _ = tx.try_send(serde_json::to_string(&msg).unwrap_or_default());
                    });
                }
                return;
            }
            op @ (Op::ExtList { .. } | Op::ExtSet { .. }) => {
                if let Some(client) = self.clients.get(&conn) {
                    let tx = client.tx.clone();
                    tokio::spawn(async move {
                        let msg = match super::extensions::execute(op).await {
                            Ok(result) => ServerMsg::Res { id, result },
                            Err(e) => ServerMsg::Err { id, error: e.to_string() },
                        };
                        let _ = tx.try_send(serde_json::to_string(&msg).unwrap_or_default());
                    });
                }
                return;
            }
            Op::SubTree => {
                if let Some(c) = self.clients.get_mut(&conn) {
                    c.tree = true;
                }
                let mut nodes: Vec<&Node> = self.sessions.values().map(|s| &s.node).collect();
                nodes.sort_by_key(|n| n.created);
                Ok(json!({"nodes": nodes}))
            }
            Op::SubSession { session, after_seq, tail } => return self.sub_session(conn, id, &session, after_seq, tail),
            Op::UnsubSession { session } => {
                let session = self.resolve(&session).unwrap_or(session);
                if let Some(c) = self.clients.get_mut(&conn) {
                    c.subs.remove(&session);
                }
                Ok(json!({}))
            }
            Op::Spawn(req) => self.spawn(req, Origin::User).map(|s| json!({"session": s})),
            Op::Send { session, text, interrupt } => match self.send(&session, text, interrupt, ItemKind::UserMessage) {
                Ok(rx) => {
                    let tx = self.clients.get(&conn).map(|c| c.tx.clone());
                    tokio::spawn(async move {
                        let msg = match rx.await {
                            Ok(d) => ServerMsg::Res { id, result: json!({"delivery": d}) },
                            Err(_) => ServerMsg::Err { id, error: "the session ended before the message was delivered".into() },
                        };
                        if let Some(tx) = tx {
                            let _ = tx.try_send(serde_json::to_string(&msg).unwrap_or_default());
                        }
                    });
                    return;
                }
                Err(e) => Err(e),
            },
            Op::Interrupt { session } => self.command(&session, Command::Interrupt),
            Op::Permission { session, request, decision } => self.permission(&session, request, decision),
            Op::Stop { session } => self.stop(&session),
            Op::Resume { session } => self.resume(&session),
            Op::History { session, before_seq, limit } => match self.resolve(&session) {
                Some(s) => {
                    if let Some(c) = self.clients.get(&conn) {
                        self.store.history(s, before_seq, limit.min(1000), id, c.tx.clone());
                    }
                    return;
                }
                None => Err(format!("no session {session}")),
            },
        };
        self.reply(conn, id, r);
    }

    /// Accepts a full id or a unique suffix. UIs show the end of the id: a ULID starts with
    /// its timestamp, so agents spawned together share a prefix but not a suffix.
    fn resolve(&self, id: &str) -> Option<SessionId> {
        if self.sessions.contains_key(id) {
            return Some(id.to_owned());
        }
        let id = id.to_uppercase();
        let mut m = self.sessions.keys().filter(|k| k.ends_with(&id));
        match (m.next(), m.next()) {
            (Some(k), None) if !id.is_empty() => Some(k.clone()),
            _ => None,
        }
    }

    fn sub_session(&mut self, conn: u64, id: u64, session: &str, after: Option<u64>, tail: Option<u32>) {
        let Some(sid) = self.resolve(session) else { return self.reply(conn, id, Err(format!("no session {session}"))) };
        let tail = tail.unwrap_or(TAIL as u32) as usize;
        if let Some(c) = self.clients.get_mut(&conn) {
            c.subs.insert(sid.clone());
        }
        let Some(s) = self.sessions.get_mut(&sid) else { return };
        if s.loaded {
            let items = snapshot(&s.items, after, tail);
            return self.reply(conn, id, Ok(json!({"session": sid, "items": items})));
        }
        if s.loading.is_empty() {
            self.store.load(sid.clone(), TAIL, self.tx.clone());
        }
        s.loading.push((conn, id, after, tail));
    }

    fn loaded(&mut self, sid: SessionId, mut items: Vec<Item>) {
        let Some(s) = self.sessions.get_mut(&sid) else { return };
        let first_live = s.items.front().map_or(u64::MAX, |i| i.seq);
        items.retain(|i| i.seq < first_live);
        for i in items.into_iter().rev() {
            s.items.push_front(i);
        }
        s.loaded = true;
        let waiting = std::mem::take(&mut s.loading);
        let replies: Vec<_> = waiting.into_iter().map(|(conn, id, after, tail)| (conn, id, snapshot(&s.items, after, tail))).collect();
        for (conn, id, items) in replies {
            self.reply(conn, id, Ok(json!({"session": sid, "items": items})));
        }
    }

    fn resume(&mut self, session: &str) -> Result<Value, String> {
        let id = self.resolve(session).ok_or(format!("no session {session}"))?;
        let s = &self.sessions[&id];
        if s.cmd.is_some() || s.node.status != Status::Interrupted || s.node.origin == Origin::Native {
            return Err("only interrupted sessions without a running process can be resumed".into());
        }
        let vendor = s.node.vendor_session.clone().ok_or("no vendor session id was saved")?;
        let kind = self.cfg.harnesses.get(&s.node.harness).ok_or("harness is unavailable")?.clone();
        if self.sessions.values().filter(|s| s.cmd.is_some()).count() >= self.cfg.max_live {
            return Err("live agent limit reached".into());
        }
        let spec = SpawnSpec {
            session: id.clone(),
            prompt: String::new(),
            cwd: PathBuf::from(&s.node.cwd),
            model: s.node.model.clone(),
            policy: s.node.policy,
            resume: Some(vendor),
            mcp: (self.cfg.exe.clone(), mcp_args(&self.cfg.sock, &id)),
        };
        let cmd = harness::start(&kind, spec, self.tx.clone(), &mut self.codex);
        let s = self.sessions.get_mut(&id).ok_or("session disappeared")?;
        s.cmd = Some(cmd);
        s.stop_requested = false;
        s.reported = false;
        s.node.status = Status::Starting;
        s.node.ended = None;
        self.touch(&id);
        Ok(json!({"session":id}))
    }

    fn command(&mut self, session: &str, cmd: Command) -> Result<Value, String> {
        let s = self.resolve(session).and_then(|id| self.sessions.get(&id)).ok_or(format!("no session {session}"))?;
        let tx = s.cmd.as_ref().ok_or("the session is not running")?;
        tx.try_send(cmd).map_err(|_| "the session is not accepting commands".to_owned())?;
        Ok(json!({}))
    }

    fn permission(&mut self, session: &str, request: String, decision: Decision) -> Result<Value, String> {
        let sid = self.resolve(session).ok_or(format!("no session {session}"))?;
        let pending = self
            .sessions
            .get(&sid)
            .is_some_and(|s| s.items.iter().any(|i| i.id == request && i.kind == ItemKind::Permission && i.state == ItemState::Streaming));
        if !pending {
            return Err(format!("no pending permission request {request}"));
        }
        self.command(&sid, Command::Permission { request, decision })
    }

    /// Stops a session and every agent it spawned, so no orphan keeps spending.
    fn stop(&mut self, session: &str) -> Result<Value, String> {
        let sid = self.resolve(session).ok_or(format!("no session {session}"))?;
        let mut todo = vec![sid];
        while let Some(id) = todo.pop() {
            todo.extend(self.children.get(&id).cloned().unwrap_or_default());
            if let Some(s) = self.sessions.get_mut(&id)
                && let Some(tx) = &s.cmd
            {
                s.stop_requested = true;
                let _ = tx.try_send(Command::Stop);
            }
        }
        Ok(json!({}))
    }

    /// Queues `text` to a session; the receiver resolves with what the harness did with it.
    /// `kind` records who wrote it: `UserMessage` for a human, `Notice` for canopy or another agent.
    fn send(&mut self, session: &str, text: String, interrupt: bool, kind: ItemKind) -> Result<oneshot::Receiver<Delivery>, String> {
        let sid = self.resolve(session).ok_or(format!("no session {session}"))?;
        let s = self.sessions.get(&sid).ok_or("gone")?;
        if s.node.origin == Origin::Native {
            return Err("native sub-agents cannot be messaged; the harness does not allow it".into());
        }
        let tx = s.cmd.clone().ok_or("the session is not running")?;
        let (reply, rx) = oneshot::channel();
        tx.try_send(Command::Send { text: text.clone(), interrupt, reply })
            .map_err(|_| "the session is not accepting messages".to_owned())?;
        self.add_item(&sid, harness::new_id(), kind, ItemState::Complete, json!({"text": text}));
        Ok(rx)
    }

    // ---- spawning and limits --------------------------------------------------------------

    fn depth(&self, id: &str) -> usize {
        let mut d = 0;
        let mut cur = self.sessions.get(id).and_then(|s| s.node.parent.clone());
        while let Some(p) = cur {
            d += 1;
            cur = self.sessions.get(&p).and_then(|s| s.node.parent.clone());
        }
        d
    }

    fn spawn(&mut self, req: SpawnReq, origin: Origin) -> Result<SessionId, String> {
        let kind = self.cfg.harnesses.get(&req.harness).cloned().ok_or_else(|| {
            let mut names: Vec<_> = self.cfg.harnesses.keys().cloned().collect();
            names.sort();
            format!("unknown harness {:?}; available: {}", req.harness, names.join(", "))
        })?;
        let parent = match &req.parent {
            Some(p) => Some(self.resolve(p).ok_or(format!("no parent session {p}"))?),
            None => None,
        };
        let live = self.sessions.values().filter(|s| s.cmd.is_some()).count();
        if live >= self.cfg.max_live {
            return Err(format!("limit: {live} agents are live (max {}); wait for one to finish or stop one", self.cfg.max_live));
        }
        let now_s = now() / 1000;
        if let Some(n) =
            self.sessions.values().map(|s| &s.node).find(|n| {
                n.harness == req.harness && matches!(n.status, Status::RateLimited { resets } if resets.is_none_or(|r| r > now_s))
            })
        {
            let when = match n.status {
                Status::RateLimited { resets: Some(r) } => format!("until unix time {r}"),
                _ => "with no reset time reported".into(),
            };
            return Err(format!("harness {} is rate limited {when}; spawn on another harness", req.harness));
        }
        let (root, policy) = match &parent {
            Some(p) => {
                let depth = self.depth(p) + 1;
                if depth > self.cfg.max_depth {
                    return Err(format!("limit: spawn depth {depth} exceeds max {}", self.cfg.max_depth));
                }
                let pn = &self.sessions[p].node;
                if let Some(b) = self.cfg.root_budget {
                    let used = self.sessions.get(&pn.root).map_or(0, |r| r.node.subtree.tokens);
                    if used >= b {
                        return Err(format!("limit: this tree used {used} tokens of its {b} token budget"));
                    }
                }
                // A child never gets more permission than its parent.
                (pn.root.clone(), req.policy.min(pn.policy))
            }
            None => (String::new(), req.policy),
        };
        let id = harness::new_id();
        let root = if root.is_empty() { id.clone() } else { root };
        let title = req.title.clone().unwrap_or_else(|| first_line(&req.prompt, 60));
        let t = now();
        let node = Node {
            id: id.clone(),
            parent: parent.clone(),
            root,
            origin,
            spawn_call: None,
            harness: req.harness.clone(),
            vendor_session: None,
            title,
            cwd: req.cwd.clone(),
            model: req.model.clone(),
            policy,
            status: Status::Starting,
            activity: String::new(),
            usage: Usage::default(),
            changes: Changes::default(),
            subtree: Rollup::default(),
            created: t,
            updated: t,
            ended: None,
        };
        let spec = SpawnSpec {
            session: id.clone(),
            prompt: parent
                .as_ref()
                .map(|p| format!("{}\n{}", super::mcp::child_header(p), req.prompt))
                .unwrap_or_else(|| req.prompt.clone()),
            cwd: PathBuf::from(&req.cwd),
            model: req.model,
            policy,
            resume: None,
            mcp: (self.cfg.exe.clone(), mcp_args(&self.cfg.sock, &id)),
        };
        let cmd = harness::start(&kind, spec, self.tx.clone(), &mut self.codex);
        self.store.node(&node);
        self.sessions.insert(id.clone(), Session::new(node, Some(cmd), 0, true));
        if let Some(p) = &parent {
            self.children.entry(p.clone()).or_default().push(id.clone());
        }
        if !req.prompt.is_empty() {
            self.add_item(&id, harness::new_id(), ItemKind::UserMessage, ItemState::Complete, json!({"text": req.prompt}));
        }
        self.rollup(&id);
        self.touch(&id);
        Ok(id)
    }

    // ---- MCP tools ------------------------------------------------------------------------

    fn mcp(&mut self, caller: SessionId, call: McpCall, reply: Reply) {
        let Some(c) = self.sessions.get(&caller) else {
            let _ = reply.send(Err(format!("unknown caller session {caller}")));
            return;
        };
        let (cwd, policy) = (c.node.cwd.clone(), c.node.policy);
        let r = match call {
            McpCall::Spawn { harness, prompt, title, model, cwd: dir } => self
                .spawn(SpawnReq { harness, prompt, cwd: dir.unwrap_or(cwd), model, policy, parent: Some(caller), title }, Origin::Spawned)
                .map(|id| json!({"id": id})),
            McpCall::List => Ok(json!({"agents": self.subtree(&caller)})),
            McpCall::Message { id, text } => {
                let target = self.resolve(&id);
                let allowed = target
                    .as_ref()
                    .is_some_and(|t| self.is_descendant(t, &caller) || self.sessions[&caller].node.parent.as_deref() == Some(t.as_str()));
                if !allowed {
                    Err(format!("{id} is not your descendant or your parent"))
                } else {
                    match self.send(target.as_deref().unwrap_or(&id), text, false, ItemKind::Notice) {
                        Ok(rx) => {
                            tokio::spawn(async move {
                                let r = rx.await.map(|d| json!({"delivery": d})).map_err(|_| "the session ended".to_owned());
                                let _ = reply.send(r);
                            });
                            return;
                        }
                        Err(e) => Err(e),
                    }
                }
            }
            McpCall::Wait { ids } => {
                let mut resolved = Vec::new();
                for id in &ids {
                    match self.resolve(id).filter(|t| self.is_descendant(t, &caller)) {
                        Some(t) => resolved.push(t),
                        None => {
                            let _ = reply.send(Err(format!("{id} is not an agent you spawned")));
                            return;
                        }
                    }
                }
                if resolved.is_empty() {
                    Err("ids must name at least one agent".into())
                } else {
                    self.waiters.push(Waiter { ids: resolved, reply });
                    self.wake_waiters();
                    return;
                }
            }
        };
        let _ = reply.send(r);
    }

    fn is_descendant(&self, id: &str, ancestor: &str) -> bool {
        let mut cur = self.sessions.get(id).and_then(|s| s.node.parent.clone());
        while let Some(p) = cur {
            if p == ancestor {
                return true;
            }
            cur = self.sessions.get(&p).and_then(|s| s.node.parent.clone());
        }
        false
    }

    fn subtree(&self, root: &str) -> Vec<Value> {
        let mut out = Vec::new();
        let mut todo: Vec<(SessionId, usize)> = self.children.get(root).into_iter().flatten().rev().map(|c| (c.clone(), 1)).collect();
        while let Some((id, depth)) = todo.pop() {
            if let Some(s) = self.sessions.get(&id) {
                out.push(agent_json(s, depth));
            }
            todo.extend(self.children.get(&id).into_iter().flatten().rev().map(|c| (c.clone(), depth + 1)));
        }
        out
    }

    /// Answers every waiter that has a settled, unreported agent, or nothing left to wait for.
    fn wake_waiters(&mut self) {
        let mut i = 0;
        while i < self.waiters.len() {
            let w = &self.waiters[i];
            if w.reply.is_closed() {
                self.waiters.swap_remove(i);
                continue;
            }
            let ready = w.ids.iter().any(|id| self.sessions.get(id).is_some_and(|s| s.node.status.is_settled() && !s.reported));
            let all_settled = w.ids.iter().all(|id| self.sessions.get(id).is_none_or(|s| s.node.status.is_settled()));
            if !(ready || all_settled) {
                i += 1;
                continue;
            }
            let w = self.waiters.swap_remove(i);
            let mut agents = Vec::new();
            for id in &w.ids {
                if let Some(s) = self.sessions.get_mut(id) {
                    let settled = s.node.status.is_settled();
                    if settled {
                        s.reported = true;
                    }
                    let mut a = agent_json(s, 0);
                    if !settled {
                        a["final"] = Value::Null;
                    }
                    agents.push(a);
                }
            }
            let _ = w.reply.send(Ok(json!({"agents": agents})));
        }
    }

    /// Tells an idle parent about children that settled since it last heard. Parents never
    /// have to poll (PLAN.md section 5.3).
    fn notify_parent(&mut self, parent: &str) {
        let Some(p) = self.sessions.get(parent) else { return };
        if p.node.status != Status::Idle || p.cmd.is_none() {
            return;
        }
        let mut lines = Vec::new();
        for c in self.children.get(parent).cloned().unwrap_or_default() {
            let Some(s) = self.sessions.get_mut(&c) else { continue };
            if s.node.origin == Origin::Spawned && s.node.status.is_settled() && !s.reported {
                s.reported = true;
                lines.push(format!(
                    "[canopy] agent {} ({}) is {}: {}",
                    s.node.id,
                    s.node.title,
                    status_word(&s.node.status),
                    if s.last_message.is_empty() { "(no message)" } else { &s.last_message }
                ));
            }
        }
        if !lines.is_empty() {
            let text = lines.join("\n\n");
            let _ = self.send(parent, text, false, ItemKind::Notice);
        }
    }

    // ---- harness events -------------------------------------------------------------------

    fn harness(&mut self, e: HarnessEvent) {
        let HarnessEvent { session, sub, ev } = e;
        let target = match &sub {
            None => session.clone(),
            Some(key) => match self.sessions.get(&session).and_then(|s| s.natives.get(key)) {
                Some(id) => id.clone(),
                None => match ev {
                    Ev::NativeChild { title } => return self.native_child(&session, key.clone(), title),
                    _ => return,
                },
            },
        };
        if !self.sessions.contains_key(&target) {
            return;
        }
        match ev {
            Ev::Started { vendor_session, pid } => {
                if let Some(s) = self.sessions.get_mut(&target) {
                    s.node.vendor_session = vendor_session.or(s.node.vendor_session.take());
                    s.pid = pid.or(s.pid);
                    if s.node.status == Status::Starting {
                        s.node.status = Status::Idle;
                    }
                    self.store.node(&s.node);
                }
                self.touch(&target);
            }
            Ev::Item { id, kind, state, body } => self.add_item(&target, id, kind, state, body),
            Ev::Delta { id, text } => {
                let Some(s) = self.sessions.get_mut(&target) else { return };
                if let Some(item) = s.items.iter_mut().rev().find(|i| i.id == id)
                    && let Some(Value::String(t)) = item.body.get_mut("text")
                {
                    t.push_str(&text);
                }
                self.emit(&target, Event::Delta { session: target.clone(), item: id, text });
            }
            Ev::Status(status) => self.set_status(&target, status),
            Ev::Usage(u) => {
                if let Some(s) = self.sessions.get_mut(&target) {
                    s.node.usage = u;
                }
                self.rollup(&target);
            }
            Ev::Activity(a) => {
                if let Some(s) = self.sessions.get_mut(&target) {
                    s.node.activity = a;
                }
                self.touch(&target);
            }
            Ev::Change { path, added, removed } => {
                if let Some(s) = self.sessions.get_mut(&target) {
                    let c = &mut s.node.changes;
                    if !c.files.contains(&path) {
                        c.files.push(path);
                    }
                    c.added += added;
                    c.removed += removed;
                }
                self.touch(&target);
            }
            Ev::NativeChild { .. } => {}
            Ev::Exited { error } => self.exited(&target, error),
        }
    }

    fn native_child(&mut self, parent: &str, key: String, title: String) {
        let Some(p) = self.sessions.get(parent) else { return };
        let id = harness::new_id();
        let t = now();
        let node = Node {
            id: id.clone(),
            parent: Some(parent.to_owned()),
            root: p.node.root.clone(),
            origin: Origin::Native,
            spawn_call: Some(key.clone()),
            harness: p.node.harness.clone(),
            vendor_session: None,
            title,
            cwd: p.node.cwd.clone(),
            model: None,
            policy: p.node.policy,
            status: Status::Running,
            activity: String::new(),
            usage: Usage::default(),
            changes: Changes::default(),
            subtree: Rollup::default(),
            created: t,
            updated: t,
            ended: None,
        };
        self.store.node(&node);
        self.sessions.insert(id.clone(), Session::new(node, None, 0, true));
        if let Some(p) = self.sessions.get_mut(parent) {
            p.natives.insert(key, id.clone());
        }
        self.children.entry(parent.to_owned()).or_default().push(id.clone());
        self.rollup(&id);
        self.touch(&id);
    }

    fn add_item(&mut self, sid: &str, id: String, kind: ItemKind, state: ItemState, body: Value) {
        let Some(s) = self.sessions.get_mut(sid) else { return };
        if kind == ItemKind::AgentMessage
            && state == ItemState::Complete
            && let Some(t) = body.get("text").and_then(Value::as_str)
        {
            s.last_message = t.to_owned();
        }
        let item = match s.items.iter_mut().rev().find(|i| i.id == id) {
            Some(i) => {
                i.state = state;
                // Completion adds fields (output, decision) to what the item started with.
                match (&mut i.body, body) {
                    (Value::Object(old), Value::Object(new)) => old.extend(new),
                    (old, new) => *old = new,
                }
                i.ts = now();
                i.clone()
            }
            None => {
                s.seq += 1;
                let i = Item { id, session: sid.to_owned(), seq: s.seq, ts: now(), kind, state, body };
                s.items.push_back(i.clone());
                i
            }
        };
        if item.state != ItemState::Streaming {
            self.store.item(&item);
        }
        while s.items.len() > TAIL && s.items.front().is_some_and(|i| i.state != ItemState::Streaming) {
            s.items.pop_front();
        }
        self.emit(sid, Event::Item { item });
    }

    fn set_status(&mut self, id: &str, status: Status) {
        let Some(s) = self.sessions.get_mut(id) else { return };
        if s.node.status == status || s.node.status.is_terminal() {
            return;
        }
        let was_settled = s.node.status.is_settled();
        s.node.status = status;
        if s.node.status.is_terminal() {
            s.node.ended = Some(now());
        }
        if s.node.status.is_settled() {
            s.node.activity.clear();
        } else {
            s.reported = false;
        }
        self.store.node(&s.node);
        let (settled, parent) = (s.node.status.is_settled(), s.node.parent.clone());
        let idle = s.node.status == Status::Idle;
        self.rollup(id);
        self.touch(id);
        if settled && !was_settled {
            self.wake_waiters();
            if let Some(p) = parent {
                self.notify_parent(&p);
            }
        }
        if idle {
            self.notify_parent(id);
        }
    }

    fn exited(&mut self, id: &str, error: Option<String>) {
        let Some(s) = self.sessions.get_mut(id) else { return };
        s.cmd = None;
        s.pid = None;
        let natives: Vec<_> = s.natives.values().cloned().collect();
        let status = match (&error, s.stop_requested) {
            (_, true) => Status::Cancelled,
            (Some(_), false) => Status::Failed,
            (None, false) => Status::Done,
        };
        if let Some(e) = error {
            self.add_item(id, harness::new_id(), ItemKind::Error, ItemState::Complete, json!({"text": e}));
        }
        for n in natives {
            self.set_status(&n, Status::Done);
        }
        self.set_status(id, status);
    }

    /// Recomputes rollups for `id` and every ancestor. Depth is at most a handful.
    fn rollup(&mut self, id: &str) {
        let mut cur = Some(id.to_owned());
        while let Some(c) = cur {
            let mut r = match self.sessions.get(&c) {
                Some(s) => Rollup { tokens: s.node.usage.input + s.node.usage.output, active: u32::from(!s.node.status.is_settled()) },
                None => break,
            };
            for k in self.children.get(&c).into_iter().flatten() {
                if let Some(ks) = self.sessions.get(k) {
                    r.tokens += ks.node.subtree.tokens;
                    r.active += ks.node.subtree.active;
                }
            }
            let s = self.sessions.get_mut(&c).expect("checked above");
            if s.node.subtree != r {
                s.node.subtree = r;
                self.touch(&c);
            }
            cur = self.sessions.get(&c).and_then(|s| s.node.parent.clone());
        }
    }
}

impl Session {
    fn new(node: Node, cmd: Option<mpsc::Sender<Command>>, seq: u64, loaded: bool) -> Session {
        Session {
            node,
            cmd,
            pid: None,
            items: VecDeque::new(),
            seq,
            loaded,
            loading: Vec::new(),
            stop_requested: false,
            reported: false,
            natives: HashMap::new(),
            last_message: String::new(),
        }
    }
}

fn snapshot(items: &VecDeque<Item>, after: Option<u64>, tail: usize) -> Vec<Item> {
    let v: Vec<Item> = items.iter().filter(|i| after.is_none_or(|a| i.seq > a)).cloned().collect();
    v[v.len().saturating_sub(tail)..].to_vec()
}

fn status_word(s: &Status) -> String {
    serde_json::to_value(s).ok().and_then(|v| v.get("state").and_then(Value::as_str).map(str::to_owned)).unwrap_or_default()
}

fn agent_json(s: &Session, depth: usize) -> Value {
    let mut v = json!({
        "id": s.node.id, "title": s.node.title, "harness": s.node.harness,
        "status": status_word(&s.node.status), "final": s.last_message, "tokens": s.node.subtree.tokens,
    });
    if depth > 0 {
        v["depth"] = json!(depth);
    }
    v
}

pub fn first_line(s: &str, max: usize) -> String {
    let l = s.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim();
    if l.chars().count() > max { format!("{}...", l.chars().take(max).collect::<String>()) } else { l.to_owned() }
}

pub fn mcp_args(sock: &std::path::Path, session: &str) -> Vec<String> {
    vec!["mcp-stdio".into(), "--sock".into(), sock.to_string_lossy().into_owned(), "--session".into(), session.into()]
}
