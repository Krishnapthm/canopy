//! End to end: a real daemon in a temp CANOPY_HOME, driven through the CLI and unix socket,
//! with `canopy fake-agent` registered as two ACP harnesses ("fake" and "fake2").

mod support;

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
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
        if !self.home.join("kill").exists() {
            std::os::unix::fs::symlink("/bin/kill", self.home.join("kill")).unwrap();
        }
        let fake = format!("{EXE} fake-agent");
        let child = Command::new(EXE)
            .args(["daemon", "--foreground", "--acp", &format!("fake={fake}"), "--acp", &format!("fake2={fake}")])
            .args(&self.flags)
            .env("CANOPY_HOME", &self.home)
            .env("PATH", &self.home)
            .env("HOME", &self.home)
            .env("CODEX_HOME", self.home.join("codex-home"))
            .env("CLAUDE_CONFIG_DIR", self.home.join("claude-home"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        self.child = Some(child);
        let sock = self.home.join("sock");
        // A socket file left by the previous daemon exists before anything listens on it.
        wait_until("daemon socket", || UnixStream::connect(&sock).is_ok());
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

    fn call(&self, op: Value) -> Value {
        support::socket(&self.home)(op)
    }

    fn tree(&self) -> String {
        let r = self.call(json!({"op":"sub_tree"}));
        r["result"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| {
                format!(
                    "{}{} {} {} {} {}t",
                    if n["parent"].is_null() { "" } else { "  " },
                    n["id"].as_str().unwrap(),
                    n["status"]["state"].as_str().unwrap(),
                    n["harness"].as_str().unwrap(),
                    n["title"].as_str().unwrap(),
                    n["subtree"]["tokens"]
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn transcript(&self, id: &str, settle: bool) -> String {
        if settle {
            support::settle(&self.home, id);
        }
        support::transcript(&self.home, id)
    }
    fn send(&self, id: &str, text: &str) -> String {
        self.request(json!({"op":"send","session":id,"text":text,"interrupt":false}))["delivery"].as_str().unwrap().to_owned()
    }
    fn request(&self, op: Value) -> Value {
        let r = self.call(op);
        assert_eq!(r["t"], "res", "{r}");
        r["result"].clone()
    }
    fn run(&self, harness: &str, prompt: &str) -> String {
        support::spawn(&self.home, harness, prompt)
    }
    fn row(&self, id: &str) -> String {
        self.tree().lines().find(|l| l.contains(id)).unwrap_or_default().to_owned()
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
fn spawn_reports_a_missing_harness_binary_before_starting() {
    let d = Daemon::start("missing-bin", &[]);
    let mut call = support::socket(&d.home);
    let reply = call(json!({"op":"spawn","harness":"claude","prompt":"hi","cwd":d.home,"policy":"ask"}));
    assert_eq!(reply["t"], "err", "missing claude binary must be rejected before starting; got {reply}");
    let error = reply["error"].as_str().unwrap();
    assert!(error.contains("claude") && error.contains("not found") && error.contains("PATH"), "{error}");
}

#[test]
fn spawn_rejects_a_missing_working_directory() {
    let d = Daemon::start("missing-cwd", &[]);
    let mut call = support::socket(&d.home);
    let cwd = d.home.join("does-not-exist");
    let reply = call(json!({"op":"spawn","harness":"fake","prompt":"hi","cwd":cwd,"policy":"ask"}));
    assert_eq!(reply["t"], "err", "missing working directory must be rejected before starting; got {reply}");
    assert!(reply["error"].as_str().unwrap().contains(cwd.to_str().unwrap()), "{reply}");
}

#[test]
fn spawn_rejects_a_relative_working_directory() {
    let d = Daemon::start("relative-cwd", &[]);
    let mut call = support::socket(&d.home);
    let reply = call(json!({"op":"spawn","harness":"fake","prompt":"hi","cwd":".","policy":"ask"}));
    assert_eq!(reply["t"], "err", "a relative cwd resolves against the daemon, not the user; got {reply}");
}

#[test]
fn failed_agent_shows_its_stderr_reason_in_the_tree() {
    let d = Daemon::start("stderr-tree", &[]);
    let mut call = support::socket(&d.home);
    assert_eq!(call(json!({"op":"sub_tree"}))["t"], "res");
    let reply = call(json!({"op":"spawn","harness":"fake","prompt":"crash quota exceeded for this account","cwd":d.home,"policy":"ask"}));
    assert_eq!(reply["t"], "res", "{reply}");
    let id = &reply["result"]["session"];
    let mut node = Value::Null;
    wait_until("crashed agent reaches a terminal state", || {
        let tree = call(json!({"op":"sub_tree"}));
        assert_eq!(tree["t"], "res", "{tree}");
        node = tree["result"]["nodes"].as_array().unwrap().iter().find(|n| n["id"] == *id).unwrap().clone();
        matches!(node["status"]["state"].as_str(), Some("failed" | "done"))
    });
    assert_eq!(
        node["status"]["state"], "failed",
        "crashed agent must fail with quota exceeded in tree; status={}, error={}",
        node["status"], node["error"]
    );
    assert!(
        node["error"].as_str().is_some_and(|e| e.contains("quota exceeded")),
        "failed tree node must carry stderr reason containing quota exceeded; error={}",
        node["error"]
    );
    assert!(node["error"].as_str().unwrap().starts_with("canopy exited with exit status: 1"), "{}", node["error"]);
    assert!(!support::transcript(&d.home, id.as_str().unwrap()).contains("peer closed"), "the exit reason is reported once");
}

#[test]
fn codex_spawns_fail_visibly_after_its_server_dies() {
    use std::os::unix::fs::PermissionsExt;
    let d = Daemon::start("dead-codex", &[]);
    let binary = d.home.join("codex");
    // Answers `initialize` (request id 0), then dies: the server started fine and was lost later.
    std::fs::write(&binary, "#!/bin/sh\nread line\necho '{\"id\":0,\"result\":{}}'\nexit 1\n").unwrap();
    std::fs::set_permissions(binary, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut call = support::socket(&d.home);
    for attempt in 1..=2 {
        let reply = call(json!({"op":"spawn","harness":"codex","prompt":"hi","cwd":d.home,"policy":"ask"}));
        assert_eq!(reply["t"], "res", "{reply}");
        let id = &reply["result"]["session"];
        let mut last = Value::Null;
        wait_until(&format!("codex spawn {attempt} reaches failed after server exit"), || {
            let tree = call(json!({"op":"sub_tree"}));
            assert_eq!(tree["t"], "res", "{tree}");
            let node = tree["result"]["nodes"].as_array().unwrap().iter().find(|n| n["id"] == *id).unwrap();
            if node["status"] != last {
                last = node["status"].clone();
                eprintln!("codex spawn {attempt}: status={last}");
            }
            last["state"] == "failed"
        });
    }
}

#[test]
fn streams_output_and_queues_a_mid_turn_message() {
    let d = Daemon::start("queue", &[]);
    let id = d.run("fake", "say hello world\nsleep 600\nsay first done");
    wait_until("streaming", || d.transcript(&id, false).contains("hello world"));
    assert_eq!(d.send(&id, "say second turn").trim(), "queued");
    let out = d.transcript(&id, true);
    let (first, second) = (out.find("first done").unwrap(), out.rfind("second turn").unwrap());
    assert!(first < second, "the queued message must run after the turn ends:\n{out}");
    assert_eq!(d.send(&id, "say third").trim(), "started_turn");
}

#[test]
fn history_survives_a_daemon_restart() {
    let mut d = Daemon::start("restart", &[]);
    let id = d.run("fake", "say remember me");
    d.transcript(&id, true);
    d.down();
    d.up();
    assert!(d.transcript(&id, false).contains("remember me"));
    assert!(d.row(&id).contains("interrupted"), "a session from a dead daemon is interrupted: {}", d.row(&id));
}

#[test]
fn stop_cancels_the_session_and_its_children() {
    let d = Daemon::start("stop", &[]);
    let parent = d.run("fake", r#"tool spawn_agent {"harness":"fake","prompt":"sleep 30000"}"#);
    d.transcript(&parent, true);
    let child = d.tree().lines().nth(1).map(|l| l.split_whitespace().next().unwrap_or("").to_owned()).unwrap();
    d.request(json!({"op":"stop","session":parent}));
    wait_until("both cancelled", || {
        let ls = d.tree();
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
        let t = d.transcript(&id, false);
        request = t.lines().find_map(|l| l.split("[request ").nth(1)).map(|r| r.trim_end_matches(']').to_owned()).unwrap_or_default();
        !request.is_empty()
    });
    assert!(d.row(&id).contains("awaiting_permission"));
    d.request(json!({"op":"permission","session":id,"request":request,"decision":"allow"}));
    assert!(d.transcript(&id, true).contains("allowed"));
    assert!(
        d.call(json!({"op":"permission","session":id,"request":request,"decision":"deny"}))["t"] == "err",
        "a request is answered once"
    );
}

#[test]
fn a_child_cannot_escalate_its_parents_policy() {
    let d = Daemon::start("policy", &[]);
    let parent = d.run("fake", "say parent");
    d.transcript(&parent, true);
    let child = d.call(json!({"op":"spawn","harness":"fake","prompt":"ask rm -rf","cwd":d.home,"policy":"yolo","parent":parent}))["result"]
        ["session"]
        .as_str()
        .unwrap()
        .to_owned();
    wait_until("child asks", || d.row(&child).contains("awaiting_permission"));
    d.request(json!({"op":"stop","session":parent}));
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
    let out = d.transcript(&parent, true);
    assert!(out.contains("child one") || out.contains("child two"), "wait_agent returns a final message:\n{out}");
    wait_until("all idle and rolled up", || {
        let ls = d.tree();
        ls.lines().filter(|l| l.contains(" idle ")).count() == 3 && d.row(&parent).contains(" 160t")
    });
    let ls = d.tree();
    assert!(ls.contains("fake2"), "{ls}");
    assert_eq!(ls.lines().filter(|l| l.starts_with("  ")).count(), 2, "two children indented under the parent:\n{ls}");
}

#[test]
fn an_idle_parent_is_told_when_a_child_finishes() {
    let d = Daemon::start("notice", &[]);
    let parent = d.run("fake", r#"tool spawn_agent {"harness":"fake2","prompt":"sleep 300\nsay the answer is 42"}"#);
    wait_until("notice delivered", || d.transcript(&parent, false).contains("[canopy] agent"));
    let t = d.transcript(&parent, false);
    assert!(t.lines().any(|l| l.starts_with("notice: [canopy] agent") && l.ends_with("is idle: the answer is 42")), "{t}");
}

#[test]
fn spawn_depth_limit_is_a_tool_error() {
    let d = Daemon::start("depth", &["--max-depth", "1"]);
    let grandchild = r#"tool spawn_agent {\"harness\":\"fake\",\"prompt\":\"say too deep\"}"#;
    let parent = d.run("fake", &format!(r#"tool spawn_agent {{"harness":"fake","prompt":"{grandchild}"}}"#));
    d.transcript(&parent, true);
    wait_until("child reports the limit", || {
        d.tree().lines().count() == 2 && {
            let child = d.tree().lines().nth(1).unwrap().split_whitespace().next().unwrap().to_owned();
            d.transcript(&child, false).contains("tool error: limit: spawn depth 2 exceeds max 1")
        }
    });
}

#[test]
fn token_budget_limit_is_a_tool_error() {
    let d = Daemon::start("budget", &["--budget", "50"]);
    let parent = d.run("fake", &("usage 100\nsleep 50\n".to_owned() + r#"tool spawn_agent {"harness":"fake","prompt":"say hi"}"#));
    let out = d.transcript(&parent, true);
    assert!(out.contains("tool error: limit: this tree used 100 tokens of its 50 token budget"), "{out}");
}

#[test]
fn unknown_harness_is_rejected_with_the_available_list() {
    let d = Daemon::start("unknown", &[]);
    let r = d.call(json!({"op":"spawn","harness":"nope","prompt":"hi","cwd":d.home,"policy":"ask"}));
    assert_eq!(r["t"], "err");
    assert!(r["error"].as_str().unwrap().contains("available: claude, codex, fake, fake2"));
}

#[test]
fn doctor_detects_zero_one_two_and_logged_out() {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("canopy-doctor-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let doctor = || {
        let start = Instant::now();
        let out = Command::new(EXE).arg("doctor").env("PATH", &dir).env("CANOPY_HOME", &dir).output().unwrap();
        assert!(out.status.success());
        // Probes time out at 800 ms; a hung CLI must not hang doctor. Slack for a loaded machine.
        assert!(start.elapsed() < Duration::from_secs(3));
        String::from_utf8(out.stdout).unwrap()
    };
    // Only installed CLIs are listed; a missing one is never named.
    assert_eq!(doctor().trim(), "No coding agent CLI found on your PATH.");
    let fake = |name: &str, exit: u8| {
        let path = dir.join(name);
        std::fs::write(
            &path,
            format!("#!/bin/sh\nif [ \"$1\" = --version ]; then echo fixture-version; exit 0; fi\necho secret-account\nexit {exit}\n"),
        )
        .unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    };
    fake("claude", 0);
    let one = doctor();
    assert!(one.contains("Claude Code") && one.contains("logged in") && !one.contains("Codex"));
    fake("codex", 1);
    let two = doctor();
    assert!(two.contains("run codex login"));
    assert!(!two.contains("secret-account"));
    std::fs::write(dir.join("claude"), "#!/bin/sh\nexec /bin/sleep 10\n").unwrap();
    let bounded = doctor();
    assert!(bounded.contains("Codex"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn demo_opens_tui_and_cleans_up_on_quit() {
    let script = r#"import os, pty, select, sys, time, fcntl, termios, struct
pid, fd = pty.fork()
if pid == 0:
    os.execv(sys.argv[1], [sys.argv[1], 'demo'])
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 120, 0, 0))
end = time.monotonic() + 10
output = b''
sent = False
try:
    while time.monotonic() < end:
        if select.select([fd], [], [], 0.1)[0]:
            try: output += os.read(fd, 65536)
            except OSError: break
        if b'Implement' in output and not sent:
            os.write(fd, b'q')
            sent = True
        child, status = os.waitpid(pid, os.WNOHANG)
        if child:
            assert os.waitstatus_to_exitcode(status) == 0, output
            break
    else: raise AssertionError(output)
finally:
    try: os.kill(pid, 9)
    except ProcessLookupError: pass
    try: os.waitpid(pid, 0)
    except ChildProcessError: pass
    os.close(fd)
assert b'Implement' in output, output
"#;
    assert!(Command::new("python3").args(["-c", script, EXE]).status().unwrap().success());
}

#[test]
fn detection_is_fresh_and_does_not_block_the_hub() {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::{fs::PermissionsExt, net::UnixStream};
    let home = std::env::temp_dir().join(format!("canopy-detect-{}", std::process::id()));
    std::fs::create_dir_all(&home).unwrap();
    let binary = home.join("codex");
    std::fs::write(&binary, "#!/bin/sh\nif [ \"$1\" = --version ]; then\n touch_marker=$CANOPY_HOME/probed\n echo started > \"$touch_marker\"\n while [ ! -f \"$CANOPY_HOME/release\" ]; do /bin/sleep 0.01; done\n echo fixture-version\n exit 0\nfi\n[ -f \"$CANOPY_HOME/logged-in\" ]\n").unwrap();
    std::fs::set_permissions(binary, std::fs::Permissions::from_mode(0o755)).unwrap();
    let child = Command::new(EXE)
        .args(["daemon", "--foreground"])
        .env("CANOPY_HOME", &home)
        .env("PATH", &home)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let d = Daemon { home, child: Some(child), flags: vec![] };
    wait_until("detection test socket", || d.home.join("sock").exists());
    let connect = || {
        let socket = UnixStream::connect(d.home.join("sock")).unwrap();
        socket.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        BufReader::new(socket)
    };
    let read = |r: &mut BufReader<UnixStream>| -> serde_json::Value {
        let mut line = String::new();
        r.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    };
    let mut other = connect();
    writeln!(other.get_mut(), "{{\"id\":1,\"op\":\"hello\",\"version\":{}}}", protocol::VERSION).unwrap();
    let hello = read(&mut other);
    assert_eq!(hello["result"]["harnesses"], serde_json::json!(["codex"]));
    assert!(!d.home.join("probed").exists(), "startup must not probe harnesses");
    let mut detector = connect();
    writeln!(detector.get_mut(), "{{\"id\":2,\"op\":\"detect\"}}").unwrap();
    wait_until("probe running", || d.home.join("probed").exists());
    writeln!(other.get_mut(), "{{\"id\":3,\"op\":\"sub_tree\"}}").unwrap();
    assert_eq!(read(&mut other)["id"], 3);
    std::fs::write(d.home.join("release"), "").unwrap();
    let detected = read(&mut detector);
    assert_eq!(detected["result"]["harnesses"][0]["version"], "fixture-version");
    assert_eq!(detected["result"]["harnesses"][0]["logged_in"], false);
    std::fs::write(d.home.join("logged-in"), "").unwrap();
    writeln!(detector.get_mut(), "{{\"id\":4,\"op\":\"detect\"}}").unwrap();
    assert_eq!(read(&mut detector)["result"]["harnesses"][0]["logged_in"], true);
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

#[test]
fn extensions_use_vendor_commands_and_owned_skill_links() {
    use serde_json::{Value, json};
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::{fs::PermissionsExt, net::UnixStream};
    let home = std::env::temp_dir().join(format!("canopy-ext-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    let project = home.join("project");
    std::fs::create_dir_all(&project).unwrap();
    for name in ["claude", "codex"] {
        let path = home.join(name);
        std::fs::write(&path, include_str!("ext_fake.py")).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let library = home.join("skills/shared");
    std::fs::create_dir_all(&library).unwrap();
    std::fs::write(library.join("SKILL.md"), "# Shared\nA fixture skill.").unwrap();
    let child = Command::new(EXE)
        .args(["daemon", "--foreground"])
        .env("CANOPY_HOME", &home)
        .env("HOME", &home)
        .env("CLAUDE_CONFIG_DIR", home.join("claude-home"))
        .env("CODEX_HOME", home.join("codex-home"))
        .env("PATH", &home)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let d = Daemon { home, child: Some(child), flags: vec![] };
    wait_until("extension test socket", || d.home.join("sock").exists());
    let socket = UnixStream::connect(d.home.join("sock")).unwrap();
    socket.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
    let mut stream = BufReader::new(socket);
    let mut call = |mut op: Value| {
        op["id"] = json!(1);
        writeln!(stream.get_mut(), "{op}").unwrap();
        let mut line = String::new();
        stream.read_line(&mut line).unwrap();
        serde_json::from_str::<Value>(&line).unwrap()
    };
    for harness in ["claude", "codex"] {
        for scope in [json!({"scope":"global"}), json!({"scope":"project","cwd":project})] {
            for enabled in [false, true] {
                let r =
                    call(json!({"op":"ext_set","harness":harness,"kind":"plugin","ext_id":"demo@local","scope":scope,"enabled":enabled}));
                assert_eq!(r["t"], "res", "{r}");
                let state: Value = serde_json::from_slice(&std::fs::read(d.home.join("ext-state.json")).unwrap()).unwrap();
                let key = format!(
                    "{harness}:{}",
                    if scope["scope"] == "project" {
                        "project"
                    } else if harness == "claude" {
                        "user"
                    } else {
                        "global"
                    }
                );
                assert_eq!(state[&key], enabled);
            }
            for enabled in [true, false] {
                let r = call(
                    json!({"op":"ext_set","harness":harness,"kind":"library_skill","ext_id":"shared","scope":scope,"enabled":enabled}),
                );
                assert_eq!(r["t"], "res", "{r}");
                let base = if scope["scope"] == "project" {
                    project.join(if harness == "claude" { ".claude/skills" } else { ".agents/skills" })
                } else if harness == "claude" {
                    d.home.join("claude-home/skills")
                } else {
                    d.home.join(".agents/skills")
                };
                assert_eq!(std::fs::read_link(base.join("shared")).is_ok(), enabled);
                assert!(library.join("SKILL.md").exists());
            }
        }
    }
    for harness in ["claude", "codex"] {
        let vendor = d.home.join(if harness == "claude" { "claude-home/skills/native-claude" } else { ".agents/skills/native" });
        std::fs::create_dir_all(&vendor).unwrap();
        std::fs::write(vendor.join("SKILL.md"), "# Native fixture").unwrap();
        let id = if harness == "claude" { vendor.clone() } else { vendor.join("SKILL.md") };
        if harness == "codex" {
            let r = call(json!({"op":"ext_set","harness":harness,"kind":"skill","ext_id":id,"scope":{"scope":"global"},"enabled":false}));
            assert_eq!(r["t"], "res", "{r}");
            let state: Value = serde_json::from_slice(&std::fs::read(d.home.join("ext-state.json")).unwrap()).unwrap();
            assert_eq!(state["skill"], false);
        }
        let r = call(json!({"op":"ext_set","harness":harness,"kind":"adopt_skill","ext_id":id,"scope":{"scope":"global"},"enabled":true}));
        assert_eq!(r["t"], "res", "{r}");
        assert!(std::fs::symlink_metadata(&vendor).unwrap().file_type().is_symlink());
        assert_eq!(std::fs::read_to_string(vendor.join("SKILL.md")).unwrap(), "# Native fixture");
    }
    let r = call(
        json!({"op":"ext_set","harness":"claude","kind":"plugin","ext_id":"declared@local","scope":{"scope":"global"},"enabled":false}),
    );
    assert_eq!(r["t"], "err");
    assert!(!d.home.join("never-run").exists());
    // Existing vendor directories are preserved, even when their name matches a library skill.
    let occupied = d.home.join("claude-home/skills/shared");
    std::fs::create_dir_all(&occupied).unwrap();
    std::fs::write(occupied.join("SKILL.md"), "keep me").unwrap();
    let r =
        call(json!({"op":"ext_set","harness":"claude","kind":"library_skill","ext_id":"shared","scope":{"scope":"global"},"enabled":true}));
    assert_eq!(r["t"], "err");
    assert_eq!(std::fs::read_to_string(occupied.join("SKILL.md")).unwrap(), "keep me");
}

#[test]
fn interrupted_session_resumes_in_place() {
    let mut d = Daemon::start("resume", &[]);
    let id = d.run("fake", "say before crash\nsleep 30000");
    wait_until("running", || d.row(&id).contains("running"));
    d.down();
    d.up();
    assert!(d.row(&id).contains("interrupted"));
    d.request(json!({"op":"resume","session":id}));
    wait_until("resumed", || d.row(&id).contains("idle"));
    d.send(&id, "say after restart");
    let output = d.transcript(&id, true);
    assert!(output.contains("before crash"));
    assert!(output.contains("after restart"));
}

#[test]
fn long_socket_path_names_limit_and_fix() {
    let home = std::env::temp_dir().join("x".repeat(110));
    let output = Command::new(EXE).args(["daemon", "--foreground"]).env("CANOPY_HOME", home).output().unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("Unix socket paths must be shorter than"), "{error}");
    assert!(error.contains("Set CANOPY_HOME to a shorter"));
}

#[test]
fn websocket_reconnects_and_resubscribes_after_restart() {
    use fastwebsockets::{Frame, Payload, Role, WebSocket};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut d = Daemon::start("ws-restart", &[]);
    let id = d.run("fake", "say saved transcript");
    d.transcript(&id, true);
    let _web = Web::start(&d);
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let info: serde_json::Value = serde_json::from_slice(&std::fs::read(d.home.join("web.json")).unwrap()).unwrap();
        let port = info["port"].as_u64().unwrap();
        let url = info["url"].as_str().unwrap();
        let (origin, token) = url.split_once('#').unwrap();
        let origin = origin.trim_end_matches('/');
        for restart in [false, true] {
            if restart { d.down(); d.up(); }
            let mut tcp = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}")).await.unwrap();
            tcp.write_all(format!("GET /ws HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: {origin}\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n").as_bytes()).await.unwrap();
            let mut header = Vec::new();
            while !header.ends_with(b"\r\n\r\n") { header.push(tcp.read_u8().await.unwrap()); }
            assert!(String::from_utf8_lossy(&header).contains("101"));
            let mut ws = WebSocket::after_handshake(tcp, Role::Client);
            for request in [
                serde_json::json!({"id":1,"op":"hello","version":protocol::VERSION,"token":token}),
                serde_json::json!({"id":2,"op":"sub_session","session":id,"tail":300}),
            ] {
                ws.write_frame(Frame::text(Payload::Owned(request.to_string().into_bytes()))).await.unwrap();
                let response = ws.read_frame().await.unwrap();
                let result: serde_json::Value = serde_json::from_slice(&response.payload).unwrap();
                assert_eq!(result["t"], "res", "{result}");
                if request["id"] == 2 { assert!(result.to_string().contains("saved transcript")); }
            }
        }
    });
}

struct Web(Child);
impl Web {
    fn start(d: &Daemon) -> Self {
        let child = Command::new(EXE)
            .arg("web")
            .env("CANOPY_HOME", &d.home)
            .env("PATH", &d.home)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let web = Self(child);
        wait_until("web registration", || d.home.join("web.json").exists());
        web
    }
}
impl Drop for Web {
    fn drop(&mut self) {
        let _ = Command::new("kill").args(["-TERM", &self.0.id().to_string()]).status();
        let _ = self.0.wait();
    }
}

#[test]
fn web_lock_and_shutdown() {
    use std::os::unix::fs::PermissionsExt;
    let d = Daemon::start("web-lock", &[]);
    let web = Web::start(&d);
    let info: Value = serde_json::from_slice(&std::fs::read(d.home.join("web.json")).unwrap()).unwrap();
    assert_eq!(std::fs::metadata(d.home.join("web.json")).unwrap().permissions().mode() & 0o777, 0o600);
    let second = Command::new(EXE).arg("web").env("CANOPY_HOME", &d.home).output().unwrap();
    assert!(!second.status.success());
    assert!(String::from_utf8_lossy(&second.stderr).contains(info["url"].as_str().unwrap()));
    drop(web);
    assert!(!d.home.join("web.json").exists());
    let _next = Web::start(&d);
}

#[test]
fn two_windows_share_tree_events() {
    let d = Daemon::start("two-windows", &[]);
    let stream = UnixStream::connect(d.home.join("sock")).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let mut other = BufReader::new(stream);
    writeln!(other.get_mut(), "{}", json!({"id":1,"op":"sub_tree"})).unwrap();
    let mut line = String::new();
    other.read_line(&mut line).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&line).unwrap()["t"], "res");
    let mut first = support::socket(&d.home);
    first(json!({"op":"sub_tree"}));
    let id = first(json!({"op":"spawn","harness":"fake","prompt":"say hello","cwd":d.home,"policy":"ask"}))["result"]["session"]
        .as_str()
        .unwrap()
        .to_owned();
    loop {
        line.clear();
        assert!(other.read_line(&mut line).unwrap() > 0);
        let msg: Value = serde_json::from_str(&line).unwrap();
        if msg["events"].as_array().unwrap().iter().any(|e| e["node"]["id"] == id) {
            break;
        }
    }
}

#[test]
fn help_lists_only_public_commands() {
    let output = Command::new(EXE).arg("--help").output().unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    let commands: Vec<_> = help.lines().filter(|l| l.starts_with("  canopy")).map(|l| l.split_whitespace().nth(1).unwrap()).collect();
    assert_eq!(commands, ["open", "web", "doctor", "demo", "help", "--version"]);
}

fn outside_binary(d: &Daemon, name: &str, script: &str) {
    use std::os::unix::fs::PermissionsExt;
    let path = d.home.join(name);
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn discovers_outside_claude_without_blocking_hub() {
    let d = Daemon::start("outside-claude", &[]);
    std::fs::write(d.home.join("rows"), include_str!("../src/harness/fixtures/claude-2.1.318-agents.json")).unwrap();
    outside_binary(
        &d,
        "claude",
        "#!/bin/sh\necho started > \"$CANOPY_HOME/probed\"\nwhile [ ! -f \"$CANOPY_HOME/release\" ]; do /bin/sleep 0.01; done\n/bin/cat \"$CANOPY_HOME/rows\"\n",
    );
    let socket = UnixStream::connect(d.home.join("sock")).unwrap();
    socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut detector = BufReader::new(socket);
    writeln!(detector.get_mut(), "{{\"id\":1,\"op\":\"discover\",\"cwd\":\"/workspace/project\"}}").unwrap();
    wait_until("discovery running", || d.home.join("probed").exists());
    assert_eq!(d.call(json!({"op":"sub_tree"}))["t"], "res");
    std::fs::write(d.home.join("release"), "").unwrap();
    let mut line = String::new();
    detector.read_line(&mut line).unwrap();
    let result: Value = serde_json::from_str(&line).unwrap();
    let row = &result["result"]["sessions"][0];
    assert_eq!(row["vendor_id"], "outside-claude");
    assert_eq!(row["started"], 1794375374709_u64);
    assert_eq!(row["status"], "busy");
    assert_eq!(row["pid"], 12345);
    assert_eq!(result["result"]["sessions"][1]["kind"], "background");
    let background = &result["result"]["sessions"][1];
    assert_eq!((background["status"].as_str(), background["pid"].is_null()), (Some("blocked"), true));
    assert_eq!(result["result"]["sessions"].as_array().unwrap().len(), 2, "an unreadable row is skipped");
    assert_eq!(result["result"]["errors"], json!([]));
}

#[test]
fn extensions_without_vendor_clis_have_no_errors() {
    let d = Daemon::start("extensions-absent", &[]);
    let result = d.request(json!({"op":"ext_list","cwd":null}));
    assert_eq!(result["errors"], json!([]));
}
