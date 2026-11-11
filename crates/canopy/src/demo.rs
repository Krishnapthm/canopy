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
    let cli = |args: &[&str]| -> Result<String> {
        let out = Command::new(&exe).args(args).env("CANOPY_HOME", &demo.home).output()?;
        ensure!(out.status.success(), "demo command failed: {}", String::from_utf8_lossy(&out.stderr));
        Ok(String::from_utf8(out.stdout)?)
    };
    let script = r#"say Planning a small change
 tool spawn_agent {"harness":"demo","title":"Implement","prompt":"say Working without spending tokens\nsay Finished"}
 tool spawn_agent {"harness":"demo","title":"Review","prompt":"say Reviewing without spending tokens\nsay Finished"}
 tool wait_agent {"ids":["$1","$2"],"timeout_s":5}
 say Demo tasks finished"#;
    let root = cli(&["run", "demo", "--title", "Plan a change", script])?;
    print!("{}", cli(&["tail", root.trim(), "-f"])?);
    print!("{}", cli(&["ls"])?);
    println!("Demo complete. No tokens spent; temporary sessions are removed on exit.");
    Ok(())
}
