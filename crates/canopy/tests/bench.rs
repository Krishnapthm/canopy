//! Performance budgets (PLAN.md section 10). Run with `make bench` (release build). A change
//! that breaks a budget is a bug. Budgets not yet measured here: idle wakeups, TUI, web, and
//! the 50-agent streaming load, which land with M4-M6.

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
            .args(["daemon", "--foreground"])
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
