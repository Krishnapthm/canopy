//! Performance budgets (PLAN.md section 10). Run with `make bench` (release build). A change
//! that breaks a budget is a bug. Browser frame rate and heap require a visible browser.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const EXE: &str = env!("CARGO_BIN_EXE_canopy");

struct Daemon {
    home: PathBuf,
    child: Child,
}

impl Daemon {
    /// Starts a daemon and returns it with the time until its socket accepted a connection.
    fn start(name: &str) -> (Daemon, Duration) {
        let home = std::env::temp_dir().join(format!("canopy-bench-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let t = Instant::now();
        let child = Command::new(EXE)
            .args(["daemon", "--foreground", "--max-live", "50", "--acp", &format!("fake={EXE} fake-agent")])
            .env("CANOPY_HOME", &home)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let sock = home.join("sock");
        while std::os::unix::net::UnixStream::connect(&sock).is_err() {
            assert!(t.elapsed() < Duration::from_secs(5), "daemon did not start");
            std::thread::sleep(Duration::from_micros(200));
        }
        (Daemon { home, child }, t.elapsed())
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = Command::new("kill").args(["-TERM", &self.child.id().to_string()]).status();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

fn median(mut v: Vec<Duration>) -> Duration {
    v.sort();
    v[v.len() / 2]
}

#[test]
fn daemon_start_to_socket_ready_within_20ms() {
    let runs: Vec<Duration> = (0..5).map(|i| Daemon::start(&format!("start{i}")).1).collect();
    let m = median(runs);
    println!("start to socket ready: {m:?} (budget 20ms)");
    assert!(m <= Duration::from_millis(20));
}

#[test]
fn cli_ls_round_trip_within_10ms() {
    let (d, _) = Daemon::start("ls");
    let runs: Vec<Duration> = (0..20)
        .map(|_| {
            let t = Instant::now();
            let o = Command::new(EXE).arg("ls").env("CANOPY_HOME", &d.home).output().unwrap();
            assert!(o.status.success());
            t.elapsed()
        })
        .collect();
    let m = median(runs);
    println!("canopy ls round trip: {m:?} (budget 10ms)");
    assert!(m <= Duration::from_millis(10));
}

#[test]
fn daemon_idle_rss_within_15mb() {
    let (d, _) = Daemon::start("rss");
    std::thread::sleep(Duration::from_millis(200));
    let o = Command::new("ps").args(["-o", "rss=", "-p", &d.child.id().to_string()]).output().unwrap();
    let kb: u64 = String::from_utf8_lossy(&o.stdout).trim().parse().unwrap();
    println!("daemon idle RSS: {} KB (budget 15360 KB)", kb);
    assert!(kb <= 15 * 1024);
}

#[cfg(target_os = "linux")]
fn cpu_seconds(pid: u32) -> f64 {
    std::fs::read_dir(format!("/proc/{pid}/task"))
        .unwrap()
        .map(|entry| {
            let stat = std::fs::read_to_string(entry.unwrap().path().join("schedstat")).unwrap();
            stat.split_whitespace().next().unwrap().parse::<f64>().unwrap() / 1_000_000_000.0
        })
        .sum()
}

#[cfg(not(target_os = "linux"))]
fn cpu_seconds(pid: u32) -> f64 {
    let output = Command::new("ps").args(["-o", "time=", "-p", &pid.to_string()]).output().unwrap();
    assert!(output.status.success(), "ps CPU measurement failed: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8_lossy(&output.stdout).trim().split(':').fold(0.0, |n, part| n * 60.0 + part.parse::<f64>().unwrap())
}

#[test]
fn fifty_agents_cpu_and_p99_latency() {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;
    let (daemon, _) = Daemon::start("load");
    let mut writer = UnixStream::connect(daemon.home.join("sock")).unwrap();
    writer.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let mut reader = BufReader::new(writer.try_clone().unwrap());
    let mut next = 0;
    let mut call = |op: serde_json::Value| {
        next += 1;
        let mut op = op;
        op["id"] = next.into();
        writeln!(writer, "{op}").unwrap();
        loop {
            let mut line = String::new();
            assert!(reader.read_line(&mut line).unwrap() > 0);
            let msg: serde_json::Value = serde_json::from_str(&line).unwrap();
            if msg["id"] == next {
                assert_eq!(msg["t"], "res", "{msg}");
                break msg["result"].clone();
            }
        }
    };
    call(serde_json::json!({"op":"hello","version":protocol::VERSION}));
    for _ in 0..50 {
        let result =
            call(serde_json::json!({"op":"spawn","harness":"fake","prompt":"sleep 1500\nload 500","cwd":daemon.home,"policy":"ask"}));
        call(serde_json::json!({"op":"sub_session","session":result["session"],"tail":0}));
    }
    let warmup = Instant::now();
    let mut samples = Vec::new();
    let mut baseline = None;
    while warmup.elapsed() < Duration::from_secs(6) {
        let mut line = String::new();
        assert!(reader.read_line(&mut line).unwrap() > 0);
        if warmup.elapsed() < Duration::from_secs(2) {
            continue;
        }
        if baseline.is_none() {
            baseline = Some((Instant::now(), cpu_seconds(daemon.child.id())));
        }
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
        let msg: serde_json::Value = serde_json::from_str(&line).unwrap();
        for event in msg["events"].as_array().into_iter().flatten() {
            if event["t"] != "delta" {
                continue;
            }
            for stamp in event["text"].as_str().unwrap_or("").lines() {
                if let Ok(stamp) = stamp.parse::<u64>() {
                    samples.push(now.saturating_sub(stamp));
                }
            }
        }
    }
    let (start, before) = baseline.unwrap();
    let cpu = (cpu_seconds(daemon.child.id()) - before) / start.elapsed().as_secs_f64() * 100.0;
    samples.sort_unstable();
    assert!(samples.len() >= 5000, "insufficient load samples: {}", samples.len());
    let p99 = samples[samples.len() * 99 / 100];
    println!("50 agents x 100 deltas/s: CPU {cpu:.2}%, latency p99 {p99}ms ({} samples)", samples.len());
    assert!(cpu <= 15.0, "CPU budget exceeded: {cpu:.2}%");
    assert!(p99 <= 50, "p99 budget exceeded: {p99}ms");
}

#[cfg(target_os = "linux")]
#[test]
fn daemon_idle_has_no_wakeups() {
    let (d, _) = Daemon::start("wakeups");
    let switches = || {
        let status = std::fs::read_to_string(format!("/proc/{}/schedstat", d.child.id())).unwrap();
        status.split_whitespace().nth(2).unwrap().parse::<u64>().unwrap()
    };
    std::thread::sleep(Duration::from_millis(300));
    let before = switches();
    std::thread::sleep(Duration::from_secs(1));
    let wakeups = switches() - before;
    println!("idle scheduled wakeups: {wakeups}/s (budget 0)");
    assert_eq!(wakeups, 0);
}

#[test]
fn tui_first_paint_within_50ms() {
    let (daemon, _) = Daemon::start("paint");
    let script = r#"import os, pty, select, sys, time, fcntl, termios, struct
start = time.monotonic()
pid, fd = pty.fork()
if pid == 0:
    os.execv(sys.argv[1], [sys.argv[1]])
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 120, 0, 0))
try:
    output = b''
    end = time.monotonic() + 3
    while b'Agents' not in output:
        assert time.monotonic() < end, 'TUI did not paint'
        if select.select([fd], [], [], 0.05)[0]: output += os.read(fd, 65536)
    elapsed = time.monotonic() - start
    print(f'TUI first paint: {elapsed * 1000:.2f}ms (budget 50ms)')
    time.sleep(0.2)
    os.write(fd, b'q')
    end = time.monotonic() + 3
    while time.monotonic() < end:
        child, status = os.waitpid(pid, os.WNOHANG)
        if child: break
        if select.select([fd], [], [], 0.05)[0]:
            try: os.read(fd, 65536)
            except OSError: pass
    else: raise AssertionError('TUI failed to quit')
    assert elapsed <= 0.05, elapsed
finally:
    try: os.kill(pid, 9)
    except ProcessLookupError: pass
    try: os.waitpid(pid, 0)
    except ChildProcessError: pass
    os.close(fd)
"#;
    assert!(Command::new("python3").args(["-c", script, EXE]).env("CANOPY_HOME", &daemon.home).status().unwrap().success());
}
