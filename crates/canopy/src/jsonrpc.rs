//! Line-delimited JSON-RPC peer shared by the Codex, ACP, and MCP code (ADR 0011).
//! One JSON object per line. Works in both directions: it sends requests and awaits their
//! responses, and hands incoming requests and notifications to the owner as `Incoming`.

use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub enum Incoming {
    Request { id: Value, method: String, params: Value },
    Notification { method: String, params: Value },
}

type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value>>>>>;

#[derive(Clone)]
pub struct Peer {
    out: mpsc::UnboundedSender<String>,
    // The pending map is RPC bookkeeping, not session state, so a short-held Mutex is fine.
    pending: Pending,
    closed: Arc<AtomicBool>,
    next_id: Arc<AtomicU64>,
    /// Codex omits the `jsonrpc` field; ACP and MCP require it.
    v2: bool,
}

impl Peer {
    /// Starts reader and writer tasks. Incoming requests and notifications arrive on the
    /// returned receiver, which closes when the remote side closes its output.
    pub fn spawn(
        reader: impl AsyncRead + Unpin + Send + 'static,
        mut writer: impl AsyncWrite + Unpin + Send + 'static,
        v2: bool,
    ) -> (Peer, mpsc::UnboundedReceiver<Incoming>) {
        let (out, mut out_rx) = mpsc::unbounded_channel::<String>();
        let (in_tx, in_rx) = mpsc::unbounded_channel();
        let pending: Pending = Arc::default();
        tokio::spawn(async move {
            while let Some(mut line) = out_rx.recv().await {
                line.push('\n');
                if writer.write_all(line.as_bytes()).await.is_err() || writer.flush().await.is_err() {
                    break;
                }
            }
        });
        let closed = Arc::new(AtomicBool::new(false));
        let reader_closed = closed.clone();
        let p = pending.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(reader).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
                route(v, &p, &in_tx);
            }
            // Fail every in-flight request so no caller waits forever on a dead peer.
            let waiting = p
                .lock()
                .map(|mut m| {
                    reader_closed.store(true, Ordering::Release);
                    std::mem::take(&mut *m)
                })
                .unwrap_or_default();
            for (_, tx) in waiting {
                let _ = tx.send(Err(anyhow!("peer closed")));
            }
        });
        (Peer { out, pending, closed, next_id: Arc::default(), v2 }, in_rx)
    }

    fn write(&self, mut v: Value) {
        if self.v2 {
            v["jsonrpc"] = json!("2.0");
        }
        let _ = self.out.send(v.to_string());
    }

    /// True once the other side's output has ended.
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    pub async fn request(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().map_err(|_| anyhow!("poisoned"))?;
            anyhow::ensure!(!self.closed.load(Ordering::Acquire), "peer closed");
            pending.insert(id, tx);
        }
        self.write(json!({"id": id, "method": method, "params": params}));
        rx.await.map_err(|_| anyhow!("peer closed"))?
    }

    pub fn notify(&self, method: &str, params: Value) {
        self.write(json!({"method": method, "params": params}));
    }

    pub fn respond(&self, id: Value, result: Value) {
        self.write(json!({"id": id, "result": result}));
    }

    pub fn respond_err(&self, id: Value, code: i64, message: &str) {
        self.write(json!({"id": id, "error": {"code": code, "message": message}}));
    }
}

fn route(mut v: Value, pending: &Pending, in_tx: &mpsc::UnboundedSender<Incoming>) {
    let method = v.get("method").and_then(Value::as_str).map(str::to_owned);
    let params = v.get_mut("params").map(Value::take).unwrap_or(Value::Null);
    match (method, v.get("id").cloned()) {
        (Some(method), Some(id)) => {
            let _ = in_tx.send(Incoming::Request { id, method, params });
        }
        (Some(method), None) => {
            let _ = in_tx.send(Incoming::Notification { method, params });
        }
        (None, Some(id)) => {
            let Some(id) = id.as_u64() else { return };
            let Some(tx) = pending.lock().ok().and_then(|mut m| m.remove(&id)) else { return };
            let res = match v.get("error") {
                Some(e) => Err(anyhow!(
                    "{} (code {})",
                    e.get("message").and_then(Value::as_str).unwrap_or("error"),
                    e.get("code").and_then(Value::as_i64).unwrap_or(0)
                )),
                None => Ok(v.get_mut("result").map(Value::take).unwrap_or(Value::Null)),
            };
            let _ = tx.send(res);
        }
        (None, None) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn request_after_eof_fails_without_waiting() {
        let (local, remote) = tokio::io::duplex(4096);
        let (r, w) = tokio::io::split(local);
        let (peer, mut incoming) = Peer::spawn(r, w, false);
        drop(remote);
        assert!(incoming.recv().await.is_none());
        assert!(peer.request("thread/start", json!({})).await.is_err());
    }

    #[tokio::test]
    async fn request_round_trip_both_directions() {
        let (a_io, b_io) = tokio::io::duplex(4096);
        let (ar, aw) = tokio::io::split(a_io);
        let (br, bw) = tokio::io::split(b_io);
        let (a, _a_in) = Peer::spawn(ar, aw, true);
        let (b, mut b_in) = Peer::spawn(br, bw, true);
        tokio::spawn(async move {
            while let Some(Incoming::Request { id, method, params }) = b_in.recv().await {
                b.respond(id, json!({"echo": method, "p": params}));
            }
        });
        let r = a.request("ping", json!(7)).await.unwrap();
        assert_eq!(r, json!({"echo": "ping", "p": 7}));
    }
}
