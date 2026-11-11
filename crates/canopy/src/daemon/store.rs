//! SQLite on its own OS thread (ADR 0008). Writes are batched: each wakeup drains what is
//! queued (up to 256 messages) into one transaction. Only completed items arrive here.

use super::hub::Msg;
use anyhow::Result;
use protocol::{Item, Node, ServerMsg, SessionId};
use rusqlite::{Connection, params};
use serde_json::json;
use std::path::Path;
use std::sync::mpsc as smpsc;
use tokio::sync::mpsc;

enum Req {
    Node(Box<Node>),
    Item(Box<Item>),
    Load { session: SessionId, limit: usize, hub: mpsc::Sender<Msg> },
    History { session: SessionId, before: u64, limit: u32, id: u64, client: mpsc::Sender<String> },
    Shutdown(smpsc::Sender<()>),
}

pub struct Store {
    tx: smpsc::Sender<Req>,
}

/// Opens the database and returns every stored node with its highest item seq.
pub fn open(path: &Path) -> Result<(Store, Vec<(Node, u64)>)> {
    let db = Connection::open(path)?;
    db.execute_batch(
        "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA cache_size = -2048;
         CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, parent TEXT, root TEXT NOT NULL,
             node TEXT NOT NULL, status TEXT NOT NULL, updated INTEGER NOT NULL) STRICT;
         CREATE INDEX IF NOT EXISTS sessions_root ON sessions(root);
         CREATE TABLE IF NOT EXISTS items (session TEXT NOT NULL, seq INTEGER NOT NULL, item TEXT NOT NULL,
             PRIMARY KEY (session, seq)) STRICT, WITHOUT ROWID;",
    )?;
    let mut restored = Vec::new();
    {
        let mut q = db.prepare("SELECT s.node, COALESCE((SELECT MAX(seq) FROM items i WHERE i.session = s.id), 0) FROM sessions s")?;
        let rows = q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        for row in rows {
            let (node, seq) = row?;
            // Tolerant: a node from an older build that no longer parses is skipped, not fatal.
            if let Ok(n) = serde_json::from_str::<Node>(&node) {
                restored.push((n, seq as u64));
            }
        }
    }
    let (tx, rx) = smpsc::channel();
    std::thread::Builder::new().name("store".into()).spawn(move || writer(db, rx))?;
    Ok((Store { tx }, restored))
}

impl Store {
    pub fn node(&self, n: &Node) {
        let _ = self.tx.send(Req::Node(Box::new(n.clone())));
    }
    pub fn item(&self, i: &Item) {
        let _ = self.tx.send(Req::Item(Box::new(i.clone())));
    }
    /// The last `limit` items of a session, delivered to the hub as `Msg::Loaded`.
    pub fn load(&self, session: SessionId, limit: usize, hub: mpsc::Sender<Msg>) {
        let _ = self.tx.send(Req::Load { session, limit, hub });
    }
    /// Answers a client's `history` request directly on its connection.
    pub fn history(&self, session: SessionId, before: u64, limit: u32, id: u64, client: mpsc::Sender<String>) {
        let _ = self.tx.send(Req::History { session, before, limit, id, client });
    }
    /// Commits everything queued and stops the thread.
    pub fn shutdown(&self) {
        let (tx, rx) = smpsc::channel();
        if self.tx.send(Req::Shutdown(tx)).is_ok() {
            let _ = rx.recv();
        }
    }
}

fn writer(mut db: Connection, rx: smpsc::Receiver<Req>) {
    while let Ok(first) = rx.recv() {
        let mut batch = vec![first];
        batch.extend(rx.try_iter().take(255));
        let mut done = None;
        if let Ok(t) = db.transaction() {
            for req in batch {
                if let Err(e) = handle(&t, req, &mut done) {
                    eprintln!("store: {e:#}");
                }
            }
            if let Err(e) = t.commit() {
                eprintln!("store: commit failed: {e:#}");
            }
        }
        if let Some(d) = done {
            let _ = d.send(());
            return;
        }
    }
}

fn handle(db: &Connection, req: Req, done: &mut Option<smpsc::Sender<()>>) -> Result<()> {
    match req {
        Req::Node(n) => {
            let status = serde_json::to_value(&n.status)?.get("state").cloned().unwrap_or_default();
            db.execute(
                "INSERT INTO sessions (id, parent, root, node, status, updated) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET node = excluded.node, status = excluded.status, updated = excluded.updated",
                params![n.id, n.parent, n.root, serde_json::to_string(&n)?, status.as_str().unwrap_or(""), n.updated as i64],
            )?;
        }
        Req::Item(i) => {
            db.execute(
                "INSERT OR REPLACE INTO items (session, seq, item) VALUES (?1, ?2, ?3)",
                params![i.session, i.seq as i64, serde_json::to_string(&i)?],
            )?;
        }
        Req::Load { session, limit, hub } => {
            let items = query(db, &session, u64::MAX, limit as u32)?;
            let _ = hub.blocking_send(Msg::Loaded { session, items });
        }
        Req::History { session, before, limit, id, client } => {
            let items = query(db, &session, before, limit)?;
            let msg = ServerMsg::Res { id, result: json!({"session": session, "items": items}) };
            let _ = client.try_send(serde_json::to_string(&msg)?);
        }
        Req::Shutdown(d) => *done = Some(d),
    }
    Ok(())
}

fn query(db: &Connection, session: &str, before: u64, limit: u32) -> Result<Vec<Item>> {
    let before = before.min(i64::MAX as u64) as i64;
    let mut q = db.prepare_cached("SELECT item FROM items WHERE session = ?1 AND seq < ?2 ORDER BY seq DESC LIMIT ?3")?;
    let mut items: Vec<Item> = q
        .query_map(params![session, before, limit], |r| r.get::<_, String>(0))?
        .filter_map(|r| r.ok().and_then(|s| serde_json::from_str(&s).ok()))
        .collect();
    items.reverse();
    Ok(items)
}
