//! Read account usage through the installed CLI, without starting an agent turn.
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::{fs::PermissionsExt, process::CommandExt};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Mutex, mpsc};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const FRESH: Duration = Duration::from_secs(60);

#[derive(Clone, Copy)]
pub enum Provider {
    Claude,
    Codex,
}

impl Provider {
    fn from_key(key: &str) -> Option<Self> {
        match key {
            "claude" => Some(Self::Claude),
            "codex" => Some(Self::Codex),
            _ => None,
        }
    }

    fn cache(self) -> &'static Mutex<Cache> {
        static CLAUDE: Mutex<Cache> = Mutex::new(Cache::new());
        static CODEX: Mutex<Cache> = Mutex::new(Cache::new());
        match self {
            Self::Claude => &CLAUDE,
            Self::Codex => &CODEX,
        }
    }
}

pub fn programs() -> Vec<(Provider, String, PathBuf)> {
    crate::programs::startable().into_iter().filter_map(|program| Some((
        Provider::from_key(program["key"].as_str()?)?,
        program["name"].as_str()?.to_owned(),
        executable(program.pointer("/run/0")?.as_str()?)?,
    ))).collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct Quota {
    pub label: String,
    pub remaining: u8,
    pub reset: Option<Reset>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Reset {
    Epoch(u64),
    Reported(String),
}

impl Quota {
    pub fn detail(&self) -> String {
        match &self.reset {
            Some(Reset::Epoch(time)) => {
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                let minutes = time.saturating_sub(now).div_ceil(60);
                let left = if minutes >= 1440 { format!("{}d", minutes.div_ceil(1440)) }
                    else if minutes >= 60 { format!("{}h {}m", minutes / 60, minutes % 60) }
                    else { format!("{minutes}m") };
                format!("{}% left · Resets in {left}", self.remaining)
            }
            Some(Reset::Reported(text)) => format!("{}% left · Resets {text}", self.remaining),
            None => format!("{}% left", self.remaining),
        }
    }
}

struct Cache {
    executable: Option<PathBuf>,
    attempted: Option<Instant>,
    reading: Option<(Instant, Vec<Quota>)>,
    error: Option<String>,
    pending: bool,
}

impl Cache {
    const fn new() -> Self {
        Self { executable: None, attempted: None, reading: None, error: None, pending: false }
    }
}

pub struct Reading {
    pub quotas: Vec<Quota>,
    pub message: Option<String>,
    pub pending: bool,
}

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

/// Starts at most one probe per provider and returns immediately.
pub fn request(provider: Provider, executable: &Path, refresh: bool) {
    let shared = provider.cache();
    let mut cache = shared.lock().unwrap_or_else(|e| e.into_inner());
    if cache.executable.as_deref() != Some(executable) {
        let pending = cache.pending;
        *cache = Cache { executable: Some(executable.to_owned()), pending, ..Cache::new() };
    }
    if cache.pending || (!refresh && cache.attempted.is_some_and(|at| at.elapsed() < FRESH)) { return; }
    cache.pending = true;
    let executable = executable.to_owned();
    std::thread::spawn(move || {
        let result = match provider {
            Provider::Claude => read_claude(&executable),
            Provider::Codex => read_codex(&executable).and_then(|limits| {
                let quotas = codex_windows(&limits);
                if quotas.is_empty() { Err("CLI reported no quota windows".into()) } else { Ok(quotas) }
            }),
        };
        let mut cache = shared.lock().unwrap_or_else(|e| e.into_inner());
        cache.pending = false;
        if cache.executable.as_ref() == Some(&executable) {
            let at = Instant::now();
            cache.attempted = Some(at);
            match result {
                Ok(quotas) => { cache.reading = Some((at, quotas)); cache.error = None; }
                Err(error) => cache.error = Some(error),
            }
        }
    });
}

pub fn reading(provider: Provider) -> Reading {
    let cache = provider.cache().lock().unwrap_or_else(|e| e.into_inner());
    let (quotas, message) = if let Some((at, quotas)) = &cache.reading {
        let message = if let Some(error) = &cache.error {
            Some(format!("Updated {}m ago · {error}", at.elapsed().as_secs() / 60))
        } else if at.elapsed() >= FRESH {
            Some(format!("Updated {}m ago", at.elapsed().as_secs() / 60))
        } else { None };
        (quotas.clone(), message)
    } else {
        let message = if cache.pending { "Loading usage…".into() }
            else if let Some(error) = &cache.error { format!("Unavailable · {error}") }
            else { "Unavailable".into() };
        (vec![], Some(message))
    };
    Reading { quotas, message, pending: cache.pending }
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
                let value = rx.recv_timeout(wait).map_err(|e| match e {
                    mpsc::RecvTimeoutError::Timeout => "CLI usage request timed out",
                    mpsc::RecvTimeoutError::Disconnected => "CLI exited before reporting usage",
                })?;
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

fn codex_windows(limits: &Value) -> Vec<Quota> {
    let mut buckets = vec![(None, &limits["rateLimits"])];
    if let Some(extra) = limits["rateLimitsByLimitId"].as_object() {
        buckets.extend(extra.iter().filter(|(key, _)| key.as_str() != "codex").map(|(key, value)| {
            (Some(value["limitName"].as_str().unwrap_or(key)), value)
        }));
    }
    buckets.into_iter().flat_map(|(name, bucket)| {
        ["primary", "secondary"].into_iter().filter_map(move |key| {
            let window = &bucket[key];
            let used = u8::try_from(window["usedPercent"].as_u64()?).ok()?;
            let remaining = 100u8.checked_sub(used)?;
            let duration = match window["windowDurationMins"].as_u64() {
                Some(300) => "5-hour limit".to_owned(),
                Some(10080) => "Weekly".to_owned(),
                Some(min) if min > 0 && min % 1440 == 0 => format!("{}-day limit", min / 1440),
                Some(min) if min > 0 && min % 60 == 0 => format!("{}-hour limit", min / 60),
                Some(min) if min > 0 => format!("{min}-minute limit"),
                _ if key == "primary" => "Primary limit".to_owned(),
                _ => "Secondary limit".to_owned(),
            };
            let name = name.or_else(|| (duration == "Weekly").then_some("all models"));
            let label = name.map_or_else(|| duration.clone(), |name| {
                let name: String = name.chars().filter(|c| !c.is_control()).take(45).collect();
                format!("{duration} · {name}")
            });
            Some(Quota { label, remaining, reset: window["resetsAt"].as_u64().map(Reset::Epoch) })
        })
    }).collect()
}

fn claude_windows(screen: &str) -> Vec<Quota> {
    let mut rows: Vec<Quota> = Vec::new();
    let mut label = None;
    for line in screen.lines().map(str::trim) {
        if line.starts_with("What's contributing") { break; }
        if line == "Current session" {
            label = Some("5-hour limit".to_owned());
        } else if let Some(suffix) = line.strip_prefix("Current week") {
            label = Some(match suffix.trim().strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
                Some(scope) => format!("Weekly · {scope}"),
                None => "Weekly · all models".into(),
            });
        } else if let Some((usage, left)) = line.split_once("% used").map(|(n, _)| (n, false))
            .or_else(|| line.split_once("% left").map(|(n, _)| (n, true)))
        {
            let Some(percent) = usage.split_whitespace().last().and_then(|n| n.parse::<u8>().ok()).filter(|&n| n <= 100) else { continue };
            if let Some(label) = label.take() {
                rows.push(Quota { label, remaining: if left { percent } else { 100 - percent }, reset: None });
            }
        } else if let Some(reset) = line.strip_prefix("Resets ")
            && label.is_none()
            && let Some(row) = rows.last_mut()
        {
            row.reset = Some(Reset::Reported(reset.to_owned()));
        }
    }
    rows
}

/// A bounded PTY which sends only the built-in usage command to an empty workspace.
fn read_claude(executable: &Path) -> Result<Vec<Quota>, String> {
    let cwd = std::env::temp_dir().join(format!("canopy-usage-{}", objc2_foundation::NSUUID::UUID().UUIDString()));
    std::fs::create_dir(&cwd).map_err(|e| e.to_string())?;
    let result = claude_pty(executable, &cwd);
    let _ = std::fs::remove_dir(&cwd);
    result
}

fn claude_pty(executable: &Path, cwd: &Path) -> Result<Vec<Quota>, String> {
    use std::fs::File;
    const COLS: u16 = 160;
    const ROWS: u16 = 100;
    let (mut master, mut slave) = (-1, -1);
    let mut size = libc::winsize { ws_row: ROWS, ws_col: COLS, ws_xpixel: 0, ws_ypixel: 0 };
    // openpty initializes both descriptors on success. The child gets only its slave.
    if unsafe { libc::openpty(&mut master, &mut slave, std::ptr::null_mut(), std::ptr::null_mut(), &mut size) } != 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let (mut master, slave) = unsafe { (File::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) };
    let clone = || slave.try_clone().map_err(|e| e.to_string());
    let fd = master.as_raw_fd();
    if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } == -1
        || unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) } == -1
    {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let mut command = crate::spawn::command(executable);
    command.args([
        "--safe-mode", "--tools", "", "--setting-sources", "",
        "--settings", r#"{"disableAllHooks":true,"remoteControlAtStartup":false}"#,
        "--strict-mcp-config", "--mcp-config", r#"{"mcpServers":{}}"#,
    ]).current_dir(cwd)
        .env_remove("CLAUDECODE")
        .env("TERM", "xterm-256color")
        .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1")
        .env("DISABLE_AUTOUPDATER", "1")
        .stdin(Stdio::from(clone()?)).stdout(Stdio::from(clone()?)).stderr(Stdio::from(slave));
    // Run only async-signal-safe syscalls after fork, before exec.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 || libc::ioctl(0, libc::TIOCSCTTY.into(), 0) < 0 { return Err(std::io::Error::last_os_error()); }
            Ok(())
        });
    }
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    drop(command); // Close the parent's slave fds so child exit ends the stream.
    let result = (|| {
        let mut vt = crate::vt::Vt::new(COLS, ROWS);
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut asked = false;
        let mut confirmed = false;
        let mut moved = false;
        let mut submitted = false;
        let mut bytes = 0;
        let mut buf = [0; 8192];
        while Instant::now() < deadline {
            let wait = deadline.saturating_duration_since(Instant::now()).as_millis() as i32;
            let mut poll = libc::pollfd { fd, events: libc::POLLIN, revents: 0 };
            match unsafe { libc::poll(&mut poll, 1, wait) } {
                0 => break,
                -1 if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted => continue,
                -1 => return Err(std::io::Error::last_os_error().to_string()),
                _ => {}
            }
            let n = master.read(&mut buf).map_err(|_| "CLI exited before reporting usage")?;
            if n == 0 { return Err("CLI exited before reporting usage".into()); }
            bytes += n;
            if bytes > 1024 * 1024 { return Err("CLI usage output was too large".into()); }
            vt.write(&buf[..n]);
            master.write_all(&vt.take_replies()).map_err(|e| e.to_string())?;
            let screen = vt.screen_text();
            let send = |master: &mut File, text: &[u8]| master.write_all(text).map_err(|e| e.to_string());
            if !confirmed && screen.lines().any(|line| line.contains("❯") && line.contains("Yes, I trust this folder")) {
                // The CLI can ask to trust this newly created empty directory.
                send(&mut master, b"\r")?;
                confirmed = true;
            } else if !moved && screen.contains("Yes, I trust this folder") && screen.lines().any(|line| line.contains("❯") && line.contains("No, exit")) {
                send(&mut master, b"\x1bOB")?;
                moved = true;
            } else if !asked && !screen.contains("Yes, I trust this folder")
                && screen.lines().any(|line| line.trim_start().starts_with('❯'))
            {
                send(&mut master, b"/usage")?;
                asked = true;
            } else if asked && !submitted && screen.lines().any(|line| line.trim() == "❯ /usage") {
                send(&mut master, b"\r")?;
                submitted = true;
            }
            if submitted {
                let rows = claude_windows(&screen);
                if !rows.is_empty() && !screen.contains("Refreshing") && !screen.contains("Loading") { return Ok(rows); }
                let lower = screen.to_ascii_lowercase();
                if lower.contains("failed to load usage") || lower.contains("error loading usage") || lower.contains("couldn't load usage") {
                    return Err("Claude Code could not load usage".into());
                }
                if lower.contains("usage is only available") { return Err("This Claude Code account has no reported subscription quotas".into()); }
            }
            if screen.contains("Select login method") || screen.contains("Login required") {
                return Err("Sign in to Claude Code first".into());
            }
        }
        Err("CLI usage request timed out".into())
    })();
    let _ = child.kill();
    let _ = child.wait();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quota_windows_keep_extremes_and_omit_missing_data() {
        let rows = codex_windows(&json!({"rateLimits":{"primary":{"usedPercent":0,"windowDurationMins":300},"secondary":null}}));
        assert_eq!(rows, [Quota { label: "5-hour limit".into(), remaining: 100, reset: None }]);
        assert!(codex_windows(&json!({})).is_empty());
        let rows = claude_windows("Current session\n████ 100% used\nResets 2pm (UTC)\nCurrent week (Sonnet only)\n████ 0% used\nWhat's contributing to your limits usage?\n90% used");
        assert_eq!(rows, [
            Quota { label: "5-hour limit".into(), remaining: 0, reset: Some(Reset::Reported("2pm (UTC)".into())) },
            Quota { label: "Weekly · Sonnet only".into(), remaining: 100, reset: None },
        ]);
    }
}
