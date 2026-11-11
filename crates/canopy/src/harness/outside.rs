//! On-demand vendor discovery. No transcript or credential files are read.
use protocol::OutsideSession;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::HashSet, os::unix::fs::PermissionsExt, time::Duration};

pub fn available(binary: &str) -> bool {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .any(|p| p.join(binary).metadata().is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Claude {
    pid: u32,
    cwd: String,
    started_at: u64,
    session_id: String,
    #[serde(default)]
    name: String,
    status: String,
}

async fn claude(cwd: Option<&str>) -> Result<Vec<Claude>, String> {
    let mut cmd = tokio::process::Command::new("claude");
    cmd.args(["agents", "--json"]).kill_on_drop(true);
    if let Some(cwd) = cwd {
        cmd.args(["--cwd", cwd]);
    }
    let output = tokio::time::timeout(Duration::from_secs(3), cmd.output())
        .await
        .map_err(|_| "claude agents timed out".to_owned())?
        .map_err(|e| format!("claude agents: {e}"))?;
    if !output.status.success() {
        return Err(format!("claude agents exited with {}", output.status));
    }
    serde_json::from_slice(&output.stdout).map_err(|e| format!("claude agents: {e}"))
}

/// Live Claude Code sessions (optionally under `cwd`) that Canopy does not own. Read-only:
/// like herdr, Canopy shows sessions it did not start but never drives them.
pub async fn discover(cwd: Option<String>, owned: HashSet<String>) -> Value {
    let mut sessions = Vec::new();
    let mut errors = Vec::new();
    if available("claude") {
        match claude(cwd.as_deref()).await {
            Ok(rows) => sessions.extend(rows.into_iter().filter(|r| !owned.contains(&r.session_id)).map(|r| OutsideSession {
                harness: "claude".into(),
                vendor_id: r.session_id,
                cwd: r.cwd,
                title: r.name,
                started: r.started_at,
                busy: r.status == "busy",
                pid: r.pid,
            })),
            Err(e) => errors.push(e),
        }
    }
    json!({"sessions":sessions,"errors":errors})
}
