//! Read account usage through the installed CLI, without starting an agent turn.
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::{fs::{DirBuilderExt, PermissionsExt}, process::CommandExt};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
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

    fn should_refresh(&self, force: bool) -> bool {
        !self.pending && (force || self.attempted.is_none_or(|at| at.elapsed() >= FRESH))
    }

    fn snapshot(&self) -> Reading {
        let (quotas, message) = if let Some((at, quotas)) = &self.reading {
            let message = if let Some(error) = &self.error {
                Some(format!("Updated {}m ago · {error}", at.elapsed().as_secs() / 60))
            } else if at.elapsed() >= FRESH {
                Some(format!("Updated {}m ago", at.elapsed().as_secs() / 60))
            } else { None };
            (quotas.clone(), message)
        } else {
            let message = if self.pending { "Loading usage…".into() }
                else if let Some(error) = &self.error { format!("Unavailable · {error}") }
                else { "Unavailable".into() };
            (vec![], Some(message))
        };
        Reading { quotas, message, pending: self.pending }
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
    if !cache.should_refresh(refresh) { return; }
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
    provider.cache().lock().unwrap_or_else(|e| e.into_inner()).snapshot()
}

fn read_codex(executable: &Path) -> Result<Value, String> {
    let mut child = crate::spawn::command(executable).args([
        "-s", "read-only", "-a", "never", "app-server", "--stdio", "-c", "analytics.enabled=false",
    ])
        .current_dir(crate::mods::app_dir())
        .process_group(0)
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null())
        .spawn().map_err(|e| e.to_string())?;
    let result = (|| {
        let input = child.stdin.as_mut().expect("piped Codex stdin");
        let mut stdout = child.stdout.take().expect("piped Codex stdout");
        let fd = stdout.as_raw_fd();
        if unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) } == -1 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        writeln!(input, "{}", json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"canopy","version":env!("CARGO_PKG_VERSION")}}})).map_err(|e| e.to_string())?;
        let deadline = Instant::now() + Duration::from_secs(8);
        let mut pending = Vec::new();
        let mut bytes = 0;
        let mut response = |id| -> Result<Value, String> {
            let mut buf = [0; 8192];
            loop {
                while let Some(end) = pending.iter().position(|&byte| byte == b'\n') {
                    let value = serde_json::from_slice::<Value>(&pending[..end]);
                    pending.drain(..=end);
                    if let Ok(value) = value && value["id"] == id {
                        return value.get("result").cloned().ok_or_else(|| {
                            let message = value.pointer("/error/message").and_then(Value::as_str).unwrap_or("");
                            if message.to_ascii_lowercase().contains("log in") { "Sign in to Codex first".into() }
                            else { "Codex could not read account quotas".into() }
                        });
                    }
                }
                if !readable(fd, deadline.saturating_duration_since(Instant::now()))? {
                    return Err("CLI usage request timed out".into());
                }
                let n = stdout.read(&mut buf).map_err(|e| e.to_string())?;
                if n == 0 { return Err("CLI exited before reporting usage".into()); }
                bytes += n;
                if bytes > 1024 * 1024 { return Err("CLI usage output was too large".into()); }
                pending.extend_from_slice(&buf[..n]);
            }
        };
        response(1)?;
        writeln!(input, "{}\n{}", json!({"jsonrpc":"2.0","method":"initialized"}), json!({"jsonrpc":"2.0","id":2,"method":"account/rateLimits/read"})).map_err(|e| e.to_string())?;
        response(2)
    })();
    reap(child);
    result
}

/// Wait for output or stream closure without leaving a blocking reader behind.
fn readable(fd: std::os::fd::RawFd, timeout: Duration) -> Result<bool, String> {
    let deadline = Instant::now() + timeout;
    let mut poll = libc::pollfd { fd, events: libc::POLLIN, revents: 0 };
    while Instant::now() < deadline {
        let wait = deadline.saturating_duration_since(Instant::now()).as_millis() as i32;
        match unsafe { libc::poll(&mut poll, 1, wait) } {
            0 => return Ok(false),
            -1 if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted => continue,
            -1 => return Err(std::io::Error::last_os_error().to_string()),
            _ => return Ok(true),
        }
    }
    Ok(false)
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
    std::fs::DirBuilder::new().mode(0o700).create(&cwd).map_err(|e| e.to_string())?;
    let result = claude_pty(executable, &cwd, Duration::from_secs(15));
    let _ = std::fs::remove_dir(&cwd);
    result
}

fn claude_pty(executable: &Path, cwd: &Path, limit: Duration) -> Result<Vec<Quota>, String> {
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
    let child = command.spawn().map_err(|e| e.to_string())?;
    drop(command); // Close the parent's slave fds so child exit ends the stream.
    let result = (|| {
        let mut vt = crate::vt::Vt::new(COLS, ROWS);
        let deadline = Instant::now() + limit;
        let mut asked = false;
        let mut confirmed = false;
        let mut moved = false;
        let mut submitted = false;
        let mut settled = None;
        let mut bytes = 0;
        let mut buf = [0; 8192];
        while Instant::now() < deadline {
            let wait = deadline.saturating_duration_since(Instant::now());
            // A PTY read can end midway through a redraw, before later windows arrive.
            let wait = if settled.is_some() { wait.min(Duration::from_millis(180)) } else { wait };
            if !readable(fd, wait)? { return settled.ok_or_else(|| "CLI usage request timed out".into()); }
            let mut ended = || settled.take().ok_or_else(|| "CLI exited before reporting usage".into());
            let n = match master.read(&mut buf) {
                Ok(0) => return ended(),
                Err(error) if error.raw_os_error() == Some(libc::EIO) => return ended(),
                Err(error) => return Err(error.to_string()),
                Ok(n) => n,
            };
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
                settled = (!rows.is_empty() && !screen.contains("Refreshing") && !screen.contains("Loading")).then_some(rows);
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
    reap(child);
    result
}

fn reap(mut child: std::process::Child) {
    // Both probes start their own process group. End any helper commands too.
    if unsafe { libc::killpg(child.id() as libc::pid_t, libc::SIGKILL) } != 0 { let _ = child.kill(); }
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_uses_cached_readings_deduplicates_and_keeps_stale_success() {
        let mut cache = Cache::new();
        assert!(cache.should_refresh(false));
        cache.attempted = Some(Instant::now());
        assert!(!cache.should_refresh(false));
        assert!(cache.should_refresh(true));
        cache.pending = true;
        assert!(!cache.should_refresh(true));
        cache.pending = false;
        let stale = Instant::now() - Duration::from_secs(125);
        cache.attempted = Some(stale);
        let quota = Quota { label: "Weekly".into(), remaining: 13, reset: None };
        cache.reading = Some((stale, vec![quota.clone()]));
        cache.error = Some("CLI usage request timed out".into());
        assert!(cache.should_refresh(false));
        let snapshot = cache.snapshot();
        assert_eq!(snapshot.quotas, [quota]);
        assert!(snapshot.message.unwrap().starts_with("Updated 2m ago"));
    }

    #[test]
    fn pty_submits_only_usage_and_bounds_a_stalled_cli() {
        let dir = std::env::temp_dir().join(format!("canopy-usage-test-{}", objc2_foundation::NSUUID::UUID().UUIDString()));
        std::fs::DirBuilder::new().mode(0o700).create(&dir).unwrap();
        let executable = dir.join("usage-cli");
        let script = "#!/bin/sh\nprintf '❯ '\nIFS= read -r cmd\n[ \"$cmd\" = /usage ] || exit 2\nprintf 'Current session\\n████ 87%% used\\n'\n/bin/sleep 0.05\nprintf 'Resets 2pm (UTC)\\nCurrent week (all models)\\n████ 0%% used\\n'\n";
        std::fs::write(&executable, script).unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        let quotas = claude_pty(&executable, &dir, Duration::from_secs(3));
        std::fs::write(&executable, "#!/bin/sh\n/bin/sleep 30 & wait\n").unwrap();
        let stalled = claude_pty(&executable, &dir, Duration::from_millis(100));
        std::fs::remove_dir_all(&dir).unwrap();
        let quotas = quotas.unwrap();
        assert_eq!(quotas.iter().map(|q| q.remaining).collect::<Vec<_>>(), [13, 100]);
        assert!(matches!(&quotas[0].reset, Some(Reset::Reported(_))));
        assert_eq!(stalled.unwrap_err(), "CLI usage request timed out");
    }

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
