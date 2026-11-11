//! Unix socket server. A connection whose first line is `{"mcp": "<session>"}` is an agent's
//! MCP shim; anything else is a client speaking the NDJSON protocol in `crates/protocol`.

use super::hub::Msg;
use super::mcp;
use protocol::Request;
use serde_json::Value;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;

/// Per-client outbound queue. A client that falls this far behind is disconnected.
const QUEUE: usize = 1024;

pub async fn serve(listener: UnixListener, hub: mpsc::Sender<Msg>, harnesses: Arc<Vec<String>>) {
    let mut next = 0u64;
    while let Ok((stream, _)) = listener.accept().await {
        next += 1;
        tokio::spawn(conn(stream, next, hub.clone(), harnesses.clone()));
    }
}

async fn conn(stream: UnixStream, conn: u64, hub: mpsc::Sender<Msg>, harnesses: Arc<Vec<String>>) {
    let (r, mut w) = stream.into_split();
    let mut reader = BufReader::new(r);
    let mut first = String::new();
    if reader.read_line(&mut first).await.unwrap_or(0) == 0 {
        return;
    }
    if let Some(session) = serde_json::from_str::<Value>(&first).ok().and_then(|v| v.get("mcp")?.as_str().map(str::to_owned)) {
        return mcp::serve(session, reader, w, hub, harnesses).await;
    }

    let (tx, mut rx) = mpsc::channel::<String>(QUEUE);
    if hub.send(Msg::Connect { conn, tx }).await.is_err() {
        return;
    }
    tokio::spawn(async move {
        while let Some(mut line) = rx.recv().await {
            line.push('\n');
            if w.write_all(line.as_bytes()).await.is_err() {
                break;
            }
        }
    });
    let mut line = first;
    loop {
        let msg = match serde_json::from_str::<Request>(&line) {
            Ok(req) => Msg::Client { conn, req },
            Err(e) => {
                let id = serde_json::from_str::<Value>(&line).ok().and_then(|v| v.get("id")?.as_u64()).unwrap_or(0);
                Msg::BadRequest { conn, id, error: format!("bad request: {e}") }
            }
        };
        if hub.send(msg).await.is_err() {
            return;
        }
        line.clear();
        if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
            break;
        }
    }
    let _ = hub.send(Msg::Disconnect { conn }).await;
}
