//! End to end: a real daemon in a temp CANOPY_HOME, driven through the CLI as a user would,
//! with `canopy fake-agent` registered as two ACP harnesses ("fake" and "fake2").

use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

const EXE: &str = env!("CARGO_BIN_EXE_canopy");

struct Daemon {
    home: PathBuf,
    child: Option<Child>,
    flags: Vec<String>,
}

impl Daemon {
    fn start(name: &str, flags: &[&str]) -> Daemon {
        let home = std::env::temp_dir().join(format!("canopy-e2e-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let mut d = Daemon { home, child: None, flags: flags.iter().map(|s| s.to_string()).collect() };
        d.up();
        d
    }

    fn up(&mut self) {
        let fake = format!("{EXE} fake-agent");
        let child = Command::new(EXE)
            .args(["daemon", "--foreground", "--acp", &format!("fake={fake}"), "--acp", &format!("fake2={fake}")])
            .args(&self.flags)
            .env("CANOPY_HOME", &self.home)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        self.child = Some(child);
        let sock = self.home.join("sock");
        wait_until("daemon socket", || sock.exists());
    }

    /// Stops the daemon the way a user's Ctrl-C would, and waits for it to exit.
    fn down(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = Command::new("kill").args(["-TERM", &c.id().to_string()]).status();
            let deadline = Instant::now() + Duration::from_secs(5);
            while c.try_wait().unwrap().is_none() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(20));
            }
            let _ = c.kill();
            let _ = c.wait();
        }
    }

    fn cli(&self, args: &[&str]) -> Output {
        Command::new(EXE).args(args).env("CANOPY_HOME", &self.home).current_dir(&self.home).output().unwrap()
    }

    /// Runs a CLI command that must succeed and returns its stdout.
    fn ok(&self, args: &[&str]) -> String {
        let o = self.cli(args);
        assert!(o.status.success(), "canopy {args:?} failed: {}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).into_owned()
    }

    fn run(&self, harness: &str, prompt: &str) -> String {
        self.ok(&["run", harness, prompt]).trim().to_owned()
    }

    /// The `ls` row for a session id.
    fn row(&self, id: &str) -> String {
        let short = &id[id.len() - 8..];
        self.ok(&["ls"]).lines().find(|l| l.contains(short)).unwrap_or_default().to_owned()
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        self.down();
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

fn wait_until(what: &str, mut f: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !f() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn streams_output_and_queues_a_mid_turn_message() {
    let d = Daemon::start("queue", &[]);
    let id = d.run("fake", "say hello world\nsleep 600\nsay first done");
    wait_until("streaming", || d.ok(&["tail", &id]).contains("hello world"));
    assert_eq!(d.ok(&["send", &id, "say second turn"]).trim(), "queued");
    let out = d.ok(&["tail", &id, "-f"]);
    let (first, second) = (out.find("first done").unwrap(), out.rfind("second turn").unwrap());
    assert!(first < second, "the queued message must run after the turn ends:\n{out}");
    assert_eq!(d.ok(&["send", &id, "say third"]).trim(), "started_turn");
}

#[test]
fn history_survives_a_daemon_restart() {
    let mut d = Daemon::start("restart", &[]);
    let id = d.run("fake", "say remember me");
    d.ok(&["tail", &id, "-f"]);
    d.down();
    d.up();
    assert!(d.ok(&["tail", &id]).contains("remember me"));
    assert!(d.row(&id).contains("interrupted"), "a session from a dead daemon is interrupted: {}", d.row(&id));
}

#[test]
fn stop_cancels_the_session_and_its_children() {
    let d = Daemon::start("stop", &[]);
    let parent = d.run("fake", r#"tool spawn_agent {"harness":"fake","prompt":"sleep 30000"}"#);
    d.ok(&["tail", &parent, "-f"]);
    let child = d.ok(&["ls"]).lines().nth(1).map(|l| l.split_whitespace().next().unwrap_or("").to_owned()).unwrap();
    d.ok(&["stop", &parent]);
    wait_until("both cancelled", || {
        let ls = d.ok(&["ls"]);
        ls.lines().filter(|l| l.contains("cancelled")).count() == 2
    });
    assert!(d.row(&child).is_empty() || d.row(&child).contains("cancelled"));
}

#[test]
fn permission_request_is_answered_by_the_user() {
    let d = Daemon::start("perm", &[]);
    let id = d.run("fake", "ask deploy to prod");
    let mut request = String::new();
    wait_until("permission request", || {
        let t = d.ok(&["tail", &id]);
        request = t.lines().find_map(|l| l.split("[request ").nth(1)).map(|r| r.trim_end_matches(']').to_owned()).unwrap_or_default();
        !request.is_empty()
    });
    assert!(d.row(&id).contains("awaiting_permission"));
    d.ok(&["permit", &id, &request, "allow"]);
    assert!(d.ok(&["tail", &id, "-f"]).contains("allowed"));
    assert!(!d.cli(&["permit", &id, &request, "deny"]).status.success(), "a request is answered once");
}

#[test]
fn a_child_cannot_escalate_its_parents_policy() {
    let d = Daemon::start("policy", &[]);
    let parent = d.ok(&["run", "--policy", "ask", "fake", "say parent"]).trim().to_owned();
    d.ok(&["tail", &parent, "-f"]);
    let child = d.ok(&["run", "--policy", "yolo", "--parent", &parent, "fake", "ask rm -rf"]).trim().to_owned();
    wait_until("child asks", || d.row(&child).contains("awaiting_permission"));
    d.ok(&["stop", &parent]);
}

#[test]
fn fake_agent_spawns_two_harnesses_waits_and_rolls_up() {
    let d = Daemon::start("tree", &[]);
    let script = [
        r#"tool spawn_agent {"harness":"fake","prompt":"usage 100\nsay child one"}"#,
        r#"tool spawn_agent {"harness":"fake2","prompt":"usage 50\nsay child two"}"#,
        r#"tool wait_agent {"ids":["$1","$2"]}"#,
        "usage 10",
        "say parent done",
    ]
    .join("\n");
    let parent = d.run("fake", &script);
    let out = d.ok(&["tail", &parent, "-f"]);
    assert!(out.contains("child one") || out.contains("child two"), "wait_agent returns a final message:\n{out}");
    wait_until("all idle and rolled up", || {
        let ls = d.ok(&["ls"]);
        ls.lines().filter(|l| l.contains(" idle ")).count() == 3 && d.row(&parent).contains(" 160t")
    });
    let ls = d.ok(&["ls"]);
    assert!(ls.contains("fake2"), "{ls}");
    assert_eq!(ls.lines().filter(|l| l.starts_with("  ")).count(), 2, "two children indented under the parent:\n{ls}");
}

#[test]
fn an_idle_parent_is_told_when_a_child_finishes() {
    let d = Daemon::start("notice", &[]);
    let parent = d.run("fake", r#"tool spawn_agent {"harness":"fake2","prompt":"sleep 300\nsay the answer is 42"}"#);
    wait_until("notice delivered", || d.ok(&["tail", &parent]).contains("[canopy] agent"));
    let t = d.ok(&["tail", &parent]);
    assert!(t.lines().any(|l| l.starts_with("user: [canopy] agent") && l.ends_with("is idle: the answer is 42")), "{t}");
}

#[test]
fn spawn_depth_limit_is_a_tool_error() {
    let d = Daemon::start("depth", &["--max-depth", "1"]);
    let grandchild = r#"tool spawn_agent {\"harness\":\"fake\",\"prompt\":\"say too deep\"}"#;
    let parent = d.run("fake", &format!(r#"tool spawn_agent {{"harness":"fake","prompt":"{grandchild}"}}"#));
    d.ok(&["tail", &parent, "-f"]);
    wait_until("child reports the limit", || {
        d.ok(&["ls"]).lines().count() == 2 && {
            let child = d.ok(&["ls"]).lines().nth(1).unwrap().split_whitespace().next().unwrap().to_owned();
            d.ok(&["tail", &child]).contains("tool error: limit: spawn depth 2 exceeds max 1")
        }
    });
}

#[test]
fn token_budget_limit_is_a_tool_error() {
    let d = Daemon::start("budget", &["--budget", "50"]);
    let parent = d.run("fake", &("usage 100\nsleep 50\n".to_owned() + r#"tool spawn_agent {"harness":"fake","prompt":"say hi"}"#));
    let out = d.ok(&["tail", &parent, "-f"]);
    assert!(out.contains("tool error: limit: this tree used 100 tokens of its 50 token budget"), "{out}");
}

#[test]
fn unknown_harness_is_rejected_with_the_available_list() {
    let d = Daemon::start("unknown", &[]);
    let o = d.cli(&["run", "nope", "hi"]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("available: claude, codex, fake, fake2"));
}

#[allow(dead_code)]
#[path = "../src/client.rs"]
mod client;
#[allow(dead_code)]
#[path = "../src/paths.rs"]
mod paths;
#[allow(dead_code)]
#[path = "../src/tui/mod.rs"]
mod tui;

#[test]
fn tui_key_driven_control_and_reattach() {
    let daemon = Daemon::start("tui", &[]);
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(tui::scripted_test(&daemon.home)).unwrap();
}
