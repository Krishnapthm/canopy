//! Socket client used by the CLI (and later the TUI). Starts the daemon when none is running.

use anyhow::{Context, Result, bail};
use protocol::{Event, Op, Request, ServerMsg};
use serde_json::Value;
use std::collections::VecDeque;
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};

pub struct Client {
    lines: Lines<BufReader<OwnedReadHalf>>,
    w: OwnedWriteHalf,
    next: u64,
    events: VecDeque<Event>,
    pub warning: Option<String>,
}

impl Client {
    pub async fn connect(home: &Path) -> Result<Client> {
        crate::paths::validate_socket(home)?;
        let sock = home.join("sock");
        let stream = match UnixStream::connect(&sock).await {
            Ok(s) => s,
            Err(_) => {
                autostart().with_context(|| format!("could not start the daemon\n{}", log_tail(home)))?;
                let mut tries = 0;
                loop {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    match UnixStream::connect(&sock).await {
                        Ok(s) => break s,
                        Err(e) if tries > 150 => return Err(e).with_context(|| format!("the daemon did not start\n{}", log_tail(home))),
                        Err(_) => tries += 1,
                    }
                }
            }
        };
        let (r, w) = stream.into_split();
        let mut c = Client { lines: BufReader::new(r).lines(), w, next: 0, events: VecDeque::new(), warning: None };
        let hello = c.call(Op::Hello { version: protocol::VERSION }).await?;
        // Only restart an idle daemon; live agents must survive a client upgrade.
        let stale = stale_daemon(&hello, &std::env::current_exe()?, exe_mtime());
        if stale && hello["live_agents"].as_u64() == Some(0) {
            let pid = hello["pid"].as_u64().context("daemon hello has no pid")?;
            let status = std::process::Command::new("kill").args(["-TERM", &pid.to_string()]).status()?;
            anyhow::ensure!(status.success(), "could not stop the older daemon");
            tokio::time::timeout(Duration::from_secs(30), async {
                while sock.exists() {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            })
            .await
            .context("older daemon did not stop")?;
            drop(c);
            return Box::pin(Self::connect(home)).await;
        }
        if stale {
            c.warning = Some("the daemon is an older build; restart it when your agents finish".into());
        }
        Ok(c)
    }

    pub async fn attach(home: &Path) -> Result<Client> {
        let stream = UnixStream::connect(home.join("sock")).await?;
        let (r, w) = stream.into_split();
        let mut c = Client { lines: BufReader::new(r).lines(), w, next: 0, events: VecDeque::new(), warning: None };
        c.call(Op::Hello { version: protocol::VERSION }).await?;
        Ok(c)
    }

    /// Sends one request and waits for its response, buffering events that arrive meanwhile.
    pub async fn call(&mut self, op: Op) -> Result<Value> {
        tokio::time::timeout(Duration::from_secs(30), self.call_inner(op)).await.context("the daemon did not answer")?
    }

    async fn call_inner(&mut self, op: Op) -> Result<Value> {
        self.next += 1;
        let id = self.next;
        let mut line = serde_json::to_string(&Request { id, op })?;
        line.push('\n');
        self.w.write_all(line.as_bytes()).await?;
        loop {
            match self.read().await? {
                ServerMsg::Res { id: i, result } if i == id => return Ok(result),
                ServerMsg::Err { id: i, error } if i == id => bail!("{error}"),
                ServerMsg::Batch { events } => self.events.extend(events),
                _ => {}
            }
        }
    }

    pub async fn next_event(&mut self) -> Result<Event> {
        loop {
            if let Some(e) = self.events.pop_front() {
                return Ok(e);
            }
            if let ServerMsg::Batch { events } = self.read().await? {
                self.events.extend(events);
            }
        }
    }

    async fn read(&mut self) -> Result<ServerMsg> {
        let line = self.lines.next_line().await?.context("the daemon closed the connection")?;
        Ok(serde_json::from_str(&line)?)
    }
}

fn autostart() -> Result<()> {
    let status = std::process::Command::new(std::env::current_exe()?).arg("daemon").status()?;
    if !status.success() {
        bail!("could not start the daemon");
    }
    Ok(())
}

pub fn exe_mtime() -> Option<u64> {
    std::env::current_exe().ok()?.metadata().ok()?.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos().try_into().ok()
}

fn stale_daemon(hello: &Value, exe: &Path, mtime: Option<u64>) -> bool {
    hello.get("exe").and_then(Value::as_str) != exe.to_str() || hello.get("mtime").and_then(Value::as_u64) != mtime
}

fn log_tail(home: &Path) -> String {
    let log = std::fs::read_to_string(home.join("daemon.log")).unwrap_or_default();
    log.lines().rev().take(20).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_build_compares_path_and_mtime() {
        let hello = serde_json::json!({"exe":"/canopy", "mtime":42});
        assert!(!stale_daemon(&hello, Path::new("/canopy"), Some(42)));
        assert!(stale_daemon(&hello, Path::new("/other"), Some(42)));
        assert!(stale_daemon(&hello, Path::new("/canopy"), Some(43)));
    }
}
