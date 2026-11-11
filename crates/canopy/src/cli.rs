//! One-shot CLI commands. Each is a thin client of the daemon protocol.

use crate::client::Client;
use crate::paths;
use anyhow::{Context, Result, bail};
use protocol::{Decision, Event, Item, ItemKind, ItemState, Node, Op, Policy, SpawnReq};
use serde_json::Value;
use std::collections::HashMap;
use std::io::Write;

pub async fn run(cmd: String, mut p: pico_args::Arguments) -> Result<()> {
    if cmd == "doctor" {
        return doctor().await;
    }
    let mut c = Client::connect(&paths::home()?).await?;
    match cmd.as_str() {
        "run" => {
            let cwd: Option<String> = p.opt_value_from_str("--cwd")?;
            let model = p.opt_value_from_str("--model")?;
            let title = p.opt_value_from_str("--title")?;
            let parent = p.opt_value_from_str("--parent")?;
            let policy = match p.opt_value_from_str::<_, String>("--policy")?.as_deref() {
                None | Some("ask") => Policy::Ask,
                Some("auto_edit") => Policy::AutoEdit,
                Some("yolo") => Policy::Yolo,
                Some(o) => bail!("unknown policy {o}"),
            };
            let free = free(p)?;
            let (harness, prompt) = free.split_first().context("usage: canopy run <harness> <prompt...>")?;
            let cwd = match cwd {
                Some(d) => std::fs::canonicalize(d)?,
                None => std::env::current_dir()?,
            };
            let req = SpawnReq {
                harness: harness.clone(),
                prompt: prompt.join(" "),
                cwd: cwd.to_string_lossy().into_owned(),
                model,
                policy,
                parent,
                title,
            };
            let r = c.call(Op::Spawn(req)).await?;
            println!("{}", r["session"].as_str().unwrap_or_default());
        }
        "ls" => {
            let r = c.call(Op::SubTree).await?;
            let nodes: Vec<Node> = serde_json::from_value(r["nodes"].clone())?;
            print_tree(&nodes);
        }
        "send" => {
            let interrupt = p.contains("--interrupt");
            let free = free(p)?;
            let (id, text) = free.split_first().context("usage: canopy send <id> <text...>")?;
            let r = c.call(Op::Send { session: id.clone(), text: text.join(" "), interrupt }).await?;
            println!("{}", r["delivery"].as_str().unwrap_or_default());
        }
        "stop" => {
            c.call(Op::Stop { session: one(p)? }).await?;
        }
        "permit" => {
            let free = free(p)?;
            let [id, request, d] = free.as_slice() else { bail!("usage: canopy permit <id> <request> allow|deny|always") };
            let decision = match d.as_str() {
                "allow" => Decision::Allow,
                "deny" => Decision::Deny,
                "always" => Decision::AllowAlways,
                _ => bail!("decision must be allow, deny, or always"),
            };
            c.call(Op::Permission { session: id.clone(), request: request.clone(), decision }).await?;
        }
        "tail" => {
            let follow = p.contains("-f") || p.contains("--follow");
            let id = one(p)?;
            tail(&mut c, &id, follow).await?;
        }
        _ => bail!("unknown command {cmd}; try canopy --help"),
    }
    Ok(())
}

fn free(p: pico_args::Arguments) -> Result<Vec<String>> {
    Ok(p.finish().into_iter().map(|s| s.to_string_lossy().into_owned()).collect())
}

fn one(p: pico_args::Arguments) -> Result<String> {
    free(p)?.into_iter().next().context("missing session id")
}

/// Prints a transcript. With `follow`, streams until the session settles, and exits non-zero
/// if it failed, so scripts can chain on it.
async fn tail(c: &mut Client, id: &str, follow: bool) -> Result<()> {
    let tree = if follow { Some(c.call(Op::SubTree).await?) } else { None };
    let r = c.call(Op::SubSession { session: id.to_owned(), after_seq: None, tail: None }).await?;
    let sid = r["session"].as_str().unwrap_or(id).to_owned();
    let items: Vec<Item> = serde_json::from_value(r["items"].clone())?;
    let mut out = std::io::stdout();
    let mut streaming: Option<String> = None;
    for i in &items {
        print_item(&mut out, i, &mut streaming)?;
    }
    let Some(tree) = tree else { return Ok(()) };
    let settled = |n: &Node| n.id == sid && n.status.is_settled();
    let nodes: Vec<Node> = serde_json::from_value(tree["nodes"].clone())?;
    if let Some(n) = nodes.iter().find(|n| settled(n)) {
        return finish(n);
    }
    loop {
        match c.next_event().await? {
            Event::Item { item } if item.session == sid => print_item(&mut out, &item, &mut streaming)?,
            Event::Delta { session, text, .. } if session == sid => {
                write!(out, "{text}")?;
                out.flush()?;
            }
            Event::Node { node } if settled(&node) => return finish(&node),
            _ => {}
        }
    }
}

fn finish(n: &Node) -> Result<()> {
    match n.status {
        protocol::Status::Failed => bail!("session failed"),
        _ => Ok(()),
    }
}

fn print_item(out: &mut impl Write, i: &Item, streaming: &mut Option<String>) -> Result<()> {
    let label = match i.kind {
        ItemKind::UserMessage => "user",
        ItemKind::AgentMessage => "agent",
        ItemKind::Reasoning => "thinking",
        ItemKind::ToolCall => "tool",
        ItemKind::FileChange => "change",
        ItemKind::Permission => "permission",
        ItemKind::Error => "error",
        ItemKind::Notice => "notice",
    };
    let text = i.body.get("text").and_then(Value::as_str);
    match (&i.state, streaming.as_deref() == Some(i.id.as_str())) {
        // Opening a streamed item: print the label; deltas follow on the same line.
        (ItemState::Streaming, _) if text.is_some() => {
            write!(out, "{label}: {}", text.unwrap_or(""))?;
            *streaming = Some(i.id.clone());
        }
        (_, true) => {
            writeln!(out)?;
            *streaming = None;
        }
        (ItemState::Streaming, _) if i.kind == ItemKind::Permission => {
            let title = i.body.get("title").and_then(Value::as_str).unwrap_or("");
            writeln!(out, "permission: {title} [request {}]", i.id)?;
        }
        (ItemState::Streaming, _) => {}
        _ if i.kind == ItemKind::Permission => {
            let title = i.body.get("title").and_then(Value::as_str).unwrap_or("");
            let decision = i.body.get("decision").and_then(Value::as_str).unwrap_or("cancelled");
            writeln!(out, "permission: {title} -> {decision}")?;
        }
        _ => match (text, i.body.get("title").and_then(Value::as_str)) {
            (Some(t), _) => writeln!(out, "{label}: {t}")?,
            (None, Some(title)) => writeln!(out, "{label}: {title} ({})", serde_json::to_value(&i.state)?.as_str().unwrap_or(""))?,
            (None, None) => writeln!(out, "{label}: {}", i.body)?,
        },
    }
    out.flush()?;
    Ok(())
}

fn print_tree(nodes: &[Node]) {
    let mut kids: HashMap<Option<&str>, Vec<&Node>> = HashMap::new();
    for n in nodes {
        kids.entry(n.parent.as_deref()).or_default().push(n);
    }
    let mut stack: Vec<(&Node, usize)> = kids.get(&None).into_iter().flatten().rev().map(|n| (*n, 0)).collect();
    while let Some((n, depth)) = stack.pop() {
        let status = serde_json::to_value(&n.status).ok().and_then(|v| v["state"].as_str().map(str::to_owned)).unwrap_or_default();
        println!(
            "{:indent$}{} {:<18} {:<8} {}  {}t{}",
            "",
            &n.id[n.id.len().saturating_sub(8)..],
            status,
            n.harness,
            n.title,
            n.subtree.tokens,
            if n.activity.is_empty() { String::new() } else { format!("  {}", n.activity) },
            indent = depth * 2
        );
        stack.extend(kids.get(&Some(n.id.as_str())).into_iter().flatten().rev().map(|k| (*k, depth + 1)));
    }
}

/// Prints each built-in harness's version and login state. Runs only the CLIs' own
/// status commands, discarding their output (see docs/harness-notes/).
async fn doctor() -> Result<()> {
    println!("HARNESS       INSTALLED                       RECORDED     STATUS");
    for row in crate::harness::detect().await {
        let status = if row.version.is_none() {
            "not installed".to_owned()
        } else if !row.supported {
            "detected, not supported yet".to_owned()
        } else if row.logged_in == Some(true) {
            "logged in".to_owned()
        } else {
            format!("login unknown or logged out; run {}", row.login_command)
        };
        println!("{:<13} {:<31} {:<12} {}", row.name, row.version.as_deref().unwrap_or("-"), row.recorded, status);
    }
    Ok(())
}
