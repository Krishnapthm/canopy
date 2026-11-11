//! Wire types shared by the daemon, the CLI, the TUI, and (generated) the web app.
//! This crate is the source of truth (ADR 0015): change it, then run `make gen`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Bumped on any incompatible wire change; `hello` rejects a mismatch.
pub const VERSION: u32 = 1;

pub type SessionId = String;

macro_rules! wire {
    ($($item:item)*) => {$(
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts-rs", derive(ts_rs::TS))]
        $item
    )*};
}

wire! {
    /// Ordered from least to most permissive, so `min` is "cannot escalate".
    #[derive(Copy, Eq, PartialOrd, Ord, Default)]
    #[serde(rename_all = "snake_case")]
    pub enum Policy {
        #[default]
        Ask,
        AutoEdit,
        Yolo,
    }

    #[serde(rename_all = "snake_case")]
    pub enum Origin {
        User,
        Spawned,
        Native,
    }

    #[serde(tag = "state", rename_all = "snake_case")]
    pub enum Status {
        Starting,
        Running,
        Idle,
        AwaitingPermission,
        RateLimited { resets: Option<u64> },
        Done,
        Failed,
        Cancelled,
        Interrupted,
    }

    #[derive(Default)]
    pub struct Usage {
        pub input: u64,
        pub output: u64,
        pub cache_read: u64,
        pub cache_write: u64,
        /// Set only when the harness reports a figure itself; canopy never estimates.
        pub reported_cost_usd: Option<f64>,
    }

    #[derive(Default)]
    pub struct Changes {
        pub files: Vec<String>,
        pub added: u64,
        pub removed: u64,
    }

    /// This node plus every descendant.
    #[derive(Default)]
    pub struct Rollup {
        pub tokens: u64,
        pub active: u32,
    }

    pub struct Node {
        pub id: SessionId,
        pub parent: Option<SessionId>,
        pub root: SessionId,
        pub origin: Origin,
        /// The parent's tool call that created this node.
        pub spawn_call: Option<String>,
        pub harness: String,
        pub vendor_session: Option<String>,
        pub title: String,
        pub cwd: String,
        pub model: Option<String>,
        pub policy: Policy,
        pub status: Status,
        pub error: Option<String>,
        pub activity: String,
        pub usage: Usage,
        pub changes: Changes,
        pub subtree: Rollup,
        pub created: u64,
        pub updated: u64,
        pub ended: Option<u64>,
    }

    #[serde(rename_all = "snake_case")]
    pub enum ItemKind {
        UserMessage,
        AgentMessage,
        Reasoning,
        ToolCall,
        FileChange,
        Permission,
        Error,
        Notice,
    }

    #[serde(rename_all = "snake_case")]
    pub enum ItemState {
        Streaming,
        Complete,
        Failed,
    }

    /// `body.text` holds the text of message-like items; other fields are per kind.
    pub struct Item {
        pub id: String,
        pub session: SessionId,
        pub seq: u64,
        pub ts: u64,
        pub kind: ItemKind,
        pub state: ItemState,
        pub body: Value,
    }

    /// What `send` actually did, which differs per harness (PLAN.md section 1.4).
    #[derive(Copy, Eq)]
    #[serde(rename_all = "snake_case")]
    pub enum Delivery {
        Steered,
        Queued,
        StartedTurn,
    }

    #[derive(Copy, Eq)]
    #[serde(rename_all = "snake_case")]
    pub enum Decision {
        Allow,
        Deny,
        AllowAlways,
    }

    pub struct SpawnReq {
        pub harness: String,
        pub prompt: String,
        pub cwd: String,
        pub model: Option<String>,
        pub policy: Policy,
        pub parent: Option<SessionId>,
        pub title: Option<String>,
    }

    /// One fresh CLI probe. Login state comes only from the vendor status exit code.
    pub struct OutsideSession {
        pub harness: String,
        pub vendor_id: String,
        pub cwd: String,
        pub title: String,
        pub started: u64,
        pub busy: bool,
        pub pid: u32,
    }

    pub struct HarnessDetection {
        pub name: String,
        pub binary: String,
        pub version: Option<String>,
        pub logged_in: Option<bool>,
        pub recorded: String,
        pub supported: bool,
        pub login_command: String,
    }

    #[derive(Copy, Eq)]
    #[serde(rename_all = "snake_case")]
    pub enum ExtKind { Plugin, Skill, LibrarySkill, AdoptSkill }

    #[serde(tag = "scope", content = "cwd", rename_all = "snake_case")]
    pub enum ExtScope { Global, Project(String) }

    pub struct Extension {
        pub harness: String,
        pub kind: ExtKind,
        pub id: String,
        pub name: String,
        pub scope: ExtScope,
        pub enabled: bool,
        pub writable: bool,
        pub command: Option<String>,
    }

    #[serde(tag = "op", rename_all = "snake_case")]
    pub enum Op {
        Hello { version: u32 },
        /// Fresh probes, returned in `result.harnesses`; never blocks the hub.
        Detect,
        Discover { cwd: Option<String> },
        ExtList { cwd: Option<String> },
        ExtSet { harness: String, kind: ExtKind, #[serde(rename = "ext_id")] id: String, scope: ExtScope, enabled: bool },
        SubTree,
        SubSession { session: SessionId, after_seq: Option<u64>, tail: Option<u32> },
        UnsubSession { session: SessionId },
        Spawn(SpawnReq),
        Send { session: SessionId, text: String, interrupt: bool },
        Interrupt { session: SessionId },
        Permission { session: SessionId, request: String, decision: Decision },
        Stop { session: SessionId },
        Resume { session: SessionId },
        History { session: SessionId, before_seq: u64, limit: u32 },
    }

    /// Every request gets exactly one `res` or `err` with the same id.
    pub struct Request {
        pub id: u64,
        #[serde(flatten)]
        pub op: Op,
    }

    #[serde(tag = "t", rename_all = "snake_case")]
    pub enum Event {
        Node { node: Box<Node> },
        Item { item: Item },
        Delta { session: SessionId, item: String, text: String },
    }

    #[serde(tag = "t", rename_all = "snake_case")]
    pub enum ServerMsg {
        Res { id: u64, result: Value },
        Err { id: u64, error: String },
        Batch { events: Vec<Event> },
    }
}

impl Status {
    /// The session will not do anything more without a new message.
    pub fn is_settled(&self) -> bool {
        !matches!(self, Status::Starting | Status::Running | Status::AwaitingPermission)
    }

    /// The process is gone.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Status::Done | Status::Failed | Status::Cancelled | Status::Interrupted)
    }
}

/// `make gen`: writes every wire type, in one file, to the path in `CANOPY_GEN_OUT`.
#[cfg(all(test, feature = "ts-rs"))]
mod gen_ts {
    use super::*;
    use ts_rs::{Config, TS};

    #[test]
    fn write_typescript() {
        let Ok(path) = std::env::var("CANOPY_GEN_OUT") else { return };
        // u64 fields are ids, counts, and timestamps well below 2^53: plain numbers in JSON.
        let cfg = Config::new().with_large_int("number");
        let decls = [
            Value::decl(&cfg),
            Policy::decl(&cfg),
            Origin::decl(&cfg),
            Status::decl(&cfg),
            Usage::decl(&cfg),
            Changes::decl(&cfg),
            Rollup::decl(&cfg),
            Node::decl(&cfg),
            ItemKind::decl(&cfg),
            ItemState::decl(&cfg),
            Item::decl(&cfg),
            Delivery::decl(&cfg),
            Decision::decl(&cfg),
            SpawnReq::decl(&cfg),
            HarnessDetection::decl(&cfg),
            OutsideSession::decl(&cfg),
            ExtKind::decl(&cfg),
            ExtScope::decl(&cfg),
            Extension::decl(&cfg),
            Op::decl(&cfg),
            Request::decl(&cfg),
            Event::decl(&cfg),
            ServerMsg::decl(&cfg),
        ];
        let mut s = String::from("// Generated by `make gen` from crates/protocol. Do not edit.\n\n");
        s.push_str(&format!("export const PROTOCOL_VERSION = {VERSION};\n\n"));
        for d in decls {
            s.push_str("export ");
            s.push_str(&d);
            s.push_str("\n\n");
        }
        std::fs::write(path, s).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_wire_shape() {
        let r: Request = serde_json::from_str(r#"{"id":3,"op":"send","session":"s","text":"hi","interrupt":false}"#).unwrap();
        assert_eq!(r.op, Op::Send { session: "s".into(), text: "hi".into(), interrupt: false });
    }

    #[test]
    fn extension_id_does_not_collide_with_request_id() {
        let request = Request {
            id: 7,
            op: Op::ExtSet {
                harness: "claude".into(),
                kind: ExtKind::Plugin,
                id: "demo@local".into(),
                scope: ExtScope::Global,
                enabled: false,
            },
        };
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["id"], 7);
        assert_eq!(value["ext_id"], "demo@local");
        assert_eq!(serde_json::from_value::<Request>(value).unwrap(), request);
    }

    #[test]
    fn policy_min_cannot_escalate() {
        assert_eq!(Policy::Yolo.min(Policy::Ask), Policy::Ask);
    }
}
