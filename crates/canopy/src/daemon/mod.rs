//! Daemon startup: single-instance lock, socket, store, hub, and clean shutdown that takes
//! every agent process group down with it.

pub mod extensions;
pub mod http;
pub mod hub;
pub mod ipc;
pub mod mcp;
pub mod store;

use crate::harness::{Kind, kill_group};
use crate::paths;
use anyhow::{Context, Result, bail};
use hub::{Config, Hub, Msg};
use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::sync::Arc;
use tokio::signal::unix::{SignalKind, signal};

pub struct Opts {
    pub foreground: bool,
    /// Extra ACP agents as `name=command args...`.
    pub acp: Vec<String>,
    pub max_depth: usize,
    pub max_live: usize,
    pub root_budget: Option<u64>,
}

pub fn run(opts: Opts, raw_args: Vec<String>) -> Result<()> {
    let home = paths::home()?;
    if !opts.foreground {
        return detach(&home, raw_args);
    }
    let lock = std::fs::File::create(home.join("lock"))?;
    if lock.try_lock().is_err() {
        bail!("a canopy daemon is already running for {}", home.display());
    }
    let sock = home.join("sock");
    let _ = std::fs::remove_file(&sock);

    let mut harnesses: HashMap<String, Kind> =
        crate::harness::DETECTION.iter().filter_map(|r| r.adapter.clone().map(|k| (r.binary.to_owned(), k))).collect();
    for spec in &opts.acp {
        let (name, cmd) = spec.split_once('=').context("--acp takes name=command")?;
        // shortcut: whitespace split, no shell quoting; upgrade if a command needs quoted args.
        let mut parts = cmd.split_whitespace().map(str::to_owned);
        let command = parts.next().context("--acp command is empty")?;
        harnesses.insert(name.to_owned(), Kind::Acp { command, args: parts.collect() });
    }
    let mut names: Vec<String> = harnesses.keys().cloned().collect();
    names.sort();
    let exe = std::env::current_exe()?.to_string_lossy().into_owned();
    let (store, restored) = store::open(&home.join("db.sqlite"))?;
    let cfg =
        Config { harnesses, max_depth: opts.max_depth, max_live: opts.max_live, root_budget: opts.root_budget, exe, sock: sock.clone() };

    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    let pids = rt.block_on(async move {
        let listener = tokio::net::UnixListener::bind(&sock)?;
        std::fs::set_permissions(&sock, std::fs::Permissions::from_mode(0o600))?;
        http::start(&home).await?;
        let (tx, rx) = tokio::sync::mpsc::channel::<Msg>(4096);
        let hub = Hub::new(cfg, tx.clone(), store, restored);
        tokio::spawn(ipc::serve(listener, tx.clone(), Arc::new(names)));
        let (mut term, mut int) = (signal(SignalKind::terminate())?, signal(SignalKind::interrupt())?);
        tokio::spawn(async move {
            tokio::select! { _ = term.recv() => {}, _ = int.recv() => {} }
            let _ = tx.send(Msg::Shutdown).await;
        });
        eprintln!("canopy daemon ready on {}", sock.display());
        let pids = hub.run(rx).await;
        let _ = std::fs::remove_file(&sock);
        anyhow::Ok(pids)
    })?;
    // No orphaned agents spending money after the daemon is gone.
    for pid in pids {
        kill_group(Some(pid));
    }
    rt.shutdown_background();
    Ok(())
}

/// Re-runs this command with `--foreground` in a new process group, logging to the home dir.
fn detach(home: &std::path::Path, mut args: Vec<String>) -> Result<()> {
    let log = std::fs::OpenOptions::new().create(true).append(true).open(home.join("daemon.log"))?;
    args.push("--foreground".into());
    std::process::Command::new(std::env::current_exe()?)
        .args(&args)
        .stdin(std::process::Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log)
        .process_group(0)
        .spawn()?;
    Ok(())
}
