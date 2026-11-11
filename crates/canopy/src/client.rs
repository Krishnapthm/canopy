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
}

impl Client {
    pub async fn connect(home: &Path) -> Result<Client> {
        crate::paths::validate_socket(home)?;
        let sock = home.join("sock");
        let stream = match UnixStream::connect(&sock).await {
            Ok(s) => s,
            Err(_) => {
                autostart()?;
                let mut tries = 0;
                loop {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    match UnixStream::connect(&sock).await {
                        Ok(s) => break s,
                        Err(e) if tries > 150 => return Err(e).context("the daemon did not start; see daemon.log"),
                        Err(_) => tries += 1,
                    }
                }
            }
        };
        let (r, w) = stream.into_split();
        let mut c = Client { lines: BufReader::new(r).lines(), w, next: 0, events: VecDeque::new() };
        c.call(Op::Hello { version: protocol::VERSION }).await?;
        Ok(c)
    }

    pub async fn attach(home: &Path) -> Result<Client> {
        let stream = UnixStream::connect(home.join("sock")).await?;
        let (r, w) = stream.into_split();
        let mut c = Client { lines: BufReader::new(r).lines(), w, next: 0, events: VecDeque::new() };
        c.call(Op::Hello { version: protocol::VERSION }).await?;
        Ok(c)
    }

    /// Sends one request and waits for its response, buffering events that arrive meanwhile.
    pub async fn call(&mut self, op: Op) -> Result<Value> {
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
