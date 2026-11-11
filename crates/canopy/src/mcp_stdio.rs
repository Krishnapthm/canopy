//! `canopy mcp-stdio`: the stdio MCP server every agent is launched with. A dumb pipe to the
//! daemon socket (ADR 0010); the daemon implements the tools in `daemon/mcp.rs`.

use anyhow::{Context, Result};
use serde_json::json;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;

pub async fn run(sock: String, session: String) -> Result<()> {
    let stream = UnixStream::connect(&sock).await.with_context(|| format!("cannot reach the canopy daemon at {sock}"))?;
    let (mut r, mut w) = stream.into_split();
    w.write_all(format!("{}\n", json!({"mcp": session})).as_bytes()).await?;
    let mut stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    // Either side closing ends the shim; the agent restarting MCP gets a fresh connection.
    tokio::select! {
        _ = tokio::io::copy(&mut stdin, &mut w) => {}
        _ = async { let _ = tokio::io::copy(&mut r, &mut stdout).await; stdout.flush().await } => {}
    }
    Ok(())
}
