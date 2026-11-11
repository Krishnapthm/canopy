//! A private, disposable fake-agent tree. The guard stops the daemon on every exit path.
use anyhow::{Result, ensure};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Demo {
    child: Child,
    home: std::path::PathBuf,
}
impl Drop for Demo {
    fn drop(&mut self) {
        let _ = Command::new("/bin/kill").args(["-TERM", &self.child.id().to_string()]).status();
        let deadline = Instant::now() + Duration::from_secs(2);
        while self.child.try_wait().ok().flatten().is_none() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

pub fn run() -> Result<()> {
    let exe = std::env::current_exe()?;
    let home = std::env::temp_dir().join(format!("canopy-demo-{}", ulid::Ulid::generate()));
    std::fs::create_dir(&home)?;
    let child = Command::new(&exe)
        .args(["daemon", "--foreground", "--acp", &format!("demo={} fake-agent", exe.display())])
        .env("CANOPY_HOME", &home)
        .env("CANOPY_FAKE_SCRIPTS", serde_json::json!({
            "Plan a small change to the parser": "say Planning a small change\ntool spawn_agent {\"harness\":\"demo\",\"title\":\"Implement the change\",\"prompt\":\"Implement the change\"}\ntool spawn_agent {\"harness\":\"demo\",\"title\":\"Review the change\",\"prompt\":\"Review the change\"}\ntool wait_agent {\"ids\":[\"$1\",\"$2\"],\"timeout_s\":5}\nsay Demo tasks finished",
            "Implement the change": "say Implemented the parser change",
            "Review the change": "say Reviewed the parser change"
        }).to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let child = match child {
        Ok(c) => c,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&home);
            return Err(e.into());
        }
    };
    let mut demo = Demo { child, home };
    let deadline = Instant::now() + Duration::from_secs(3);
    while !demo.home.join("sock").exists() {
        ensure!(demo.child.try_wait()?.is_none(), "demo daemon exited before becoming ready");
        ensure!(Instant::now() < deadline, "demo daemon startup timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let mut client = crate::client::Client::attach(&demo.home).await?;
        client
            .call(protocol::Op::Spawn(protocol::SpawnReq {
                harness: "demo".into(),
                prompt: "Plan a small change to the parser".into(),
                title: Some("Plan a small change to the parser".into()),
                cwd: std::env::current_dir()?.to_string_lossy().into_owned(),
                policy: protocol::Policy::Ask,
                parent: None,
                model: None,
            }))
            .await?;
        anyhow::Ok(())
    })?;
    let status = Command::new(&exe).env("CANOPY_HOME", &demo.home).status()?;
    ensure!(status.success(), "demo TUI exited with {status}");
    Ok(())
}
