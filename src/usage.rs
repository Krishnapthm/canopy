//! Read account usage through the installed CLI, without starting an agent turn.
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Mutex, mpsc};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const FRESH: Duration = Duration::from_secs(60);

struct Cache {
    executable: Option<PathBuf>,
    attempted: Option<Instant>,
    reading: Option<(Instant, Value)>,
    pending: bool,
}
static CODEX: Mutex<Cache> = Mutex::new(Cache { executable: None, attempted: None, reading: None, pending: false });

/// Resolve an installed manifest command against the application's login PATH.
pub fn executable(command: &str) -> Option<PathBuf> {
    let executable = |path: &Path| {
        path.is_absolute() && path.metadata().is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    };
    let path = Path::new(command);
    if path.is_absolute() { return executable(path).then(|| path.to_owned()); }
    if path.components().count() != 1 { return None; }
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(path)).find(|path| executable(path))
}

/// Show cached rows immediately. A click may start a bounded refresh, with no idle polling.
pub fn codex(executable: &Path, refresh: bool) -> Vec<String> {
    let mut cache = CODEX.lock().unwrap_or_else(|e| e.into_inner());
    if cache.executable.as_deref() != Some(executable) {
        cache.executable = Some(executable.to_owned());
        cache.attempted = None;
        cache.reading = None;
        // Let the previous child finish before another one starts.
    }
    if !cache.pending && (refresh || cache.attempted.is_none_or(|at| at.elapsed() >= FRESH)) {
        cache.pending = true;
        let executable = executable.to_owned();
        std::thread::spawn(move || {
            let limits = read_codex(&executable);
            let mut cache = CODEX.lock().unwrap_or_else(|e| e.into_inner());
            cache.pending = false;
            if cache.executable.as_ref() == Some(&executable) {
                let at = Instant::now();
                cache.attempted = Some(at);
                if let Ok(limits) = limits { cache.reading = Some((at, limits)); }
            }
        });
    }
    if let Some((at, limits)) = &cache.reading {
        let mut rows = format_windows(limits);
        if at.elapsed() >= FRESH {
            rows.push(format!("Updated {}m ago", at.elapsed().as_secs() / 60));
        }
        rows
    } else if cache.pending {
        vec!["Loading…".into()]
    } else {
        vec!["Unavailable".into()]
    }
}

fn read_codex(executable: &Path) -> Result<Value, String> {
    let mut child = crate::spawn::command(executable).args([
        "-s", "read-only", "-a", "never", "app-server", "--stdio", "-c", "analytics.enabled=false",
    ])
        .current_dir(crate::mods::app_dir())
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null())
        .spawn().map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().ok_or("missing stdout")?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut lines = BufReader::new(stdout.take(1024 * 1024)).lines();
        while let Some(Ok(line)) = lines.next() {
            if let Ok(value) = serde_json::from_str::<Value>(&line) && tx.send(value).is_err() { break; }
        }
    });
    let result = (|| {
        let input = child.stdin.as_mut().ok_or("missing stdin")?;
        writeln!(input, "{}", json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"canopy","version":env!("CARGO_PKG_VERSION")}}})).map_err(|e| e.to_string())?;
        let deadline = Instant::now() + Duration::from_secs(8);
        let response = |id, rx: &mpsc::Receiver<Value>| -> Result<Value, String> {
            loop {
                let wait = deadline.saturating_duration_since(Instant::now());
                let value = rx.recv_timeout(wait).map_err(|e| e.to_string())?;
                if value["id"] == id { return value.get("result").cloned().ok_or("request failed".into()); }
            }
        };
        response(1, &rx)?;
        writeln!(input, "{}\n{}", json!({"jsonrpc":"2.0","method":"initialized"}), json!({"jsonrpc":"2.0","id":2,"method":"account/rateLimits/read"})).map_err(|e| e.to_string())?;
        let value = response(2, &rx)?;
        if value.get("rateLimits").is_some_and(Value::is_object) {
            Ok(value)
        } else {
            Err("no usage limits".into())
        }
    })();
    let _ = child.kill();
    let _ = child.wait();
    result
}

fn format_windows(limits: &Value) -> Vec<String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let mut buckets: Vec<_> = vec![&limits["rateLimits"]];
    if let Some(extra) = limits["rateLimitsByLimitId"].as_object() {
        buckets.extend(extra.iter().filter(|(key, _)| key.as_str() != "codex").map(|(_, v)| v));
    }
    let mut rows = Vec::new();
    for (idx, bucket) in buckets.into_iter().enumerate() {
        let windows: Vec<_> = ["primary", "secondary"].into_iter().filter_map(|key| {
            let window = &bucket[key];
            let used = window["usedPercent"].as_i64()?;
            if !(0..=100).contains(&used) { return None; }
            let duration = window["windowDurationMins"].as_u64().filter(|&min| min > 0).map(|min| {
                if min % 1440 == 0 { format!("{}d", min / 1440) }
                else if min % 60 == 0 { format!("{}h", min / 60) }
                else { format!("{min}m") }
            }).unwrap_or_else(|| key.into());
            let reset = window["resetsAt"].as_u64().map_or_else(|| "Reset unknown".into(), |time| {
                let minutes = time.saturating_sub(now).div_ceil(60);
                if minutes < 60 { format!("Resets in {minutes}m") }
                else { format!("Resets in {}h {}m", minutes / 60, minutes % 60) }
            });
            Some(format!("{duration:<9} {used:>3}% used   {reset}"))
        }).collect();
        if idx > 0 && !windows.is_empty() {
            if let Some(name) = bucket["limitName"].as_str().or_else(|| bucket["limitId"].as_str()) {
                rows.push(name.chars().filter(|c| !c.is_control()).take(60).collect());
            }
        }
        rows.extend(windows);
    }
    if rows.is_empty() { vec!["Unavailable".into()] } else { rows }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_show_usage_and_missing_data() {
        let rows = format_windows(&json!({"rateLimits":{"primary":{"usedPercent":42,"windowDurationMins":300},"secondary":null}}));
        assert_eq!(rows, ["5h         42% used   Reset unknown"]);
        assert_eq!(format_windows(&json!({})), ["Unavailable"]);
    }
}
