//! Fresh extension operations. Vendor commands never run on the hub task.
use crate::jsonrpc::{Incoming, Peer};
use anyhow::{Context, Result, bail, ensure};
use protocol::{ExtKind, ExtScope, Extension, Op};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub async fn execute(op: Op) -> Result<Value> {
    tokio::time::timeout(std::time::Duration::from_secs(15), async {
        match op {
            Op::ExtList { cwd } => list(cwd).await,
            Op::ExtSet { harness, kind, id, scope, enabled } => {
                ensure!(matches!(harness.as_str(), "claude" | "codex"), "unsupported harness");
                let cwd = project(&scope)?;
                let fresh = list(cwd.as_ref().map(|p| p.to_string_lossy().into_owned())).await?;
                let rows: Vec<Extension> = serde_json::from_value(fresh["items"].clone())?;
                let row = rows.iter().find(|r| r.harness == harness && r.id == id && r.scope == scope &&
                    (r.kind == kind || kind == ExtKind::AdoptSkill && r.kind == ExtKind::Skill))
                    .context("extension no longer available; refresh the list")?;
                ensure!(row.command.is_none(), "declared command requires separate confirmation; command execution is unavailable");
                if matches!(kind, ExtKind::LibrarySkill | ExtKind::AdoptSkill) {
                    let row = row.clone();
                    tokio::task::spawn_blocking(move || skill_change(&row, kind, enabled)).await??;
                } else {
                    ensure!(row.writable, "extension is read-only; adopt the skill first");
                    if harness == "claude" {
                        let action = if enabled { "enable" } else { "disable" };
                        let scope_arg = if cwd.is_some() { "project" } else { "user" };
                        claude(&["plugin", action, &id, "--scope", scope_arg, "--json"], cwd.as_deref()).await?;
                    } else if kind == ExtKind::Plugin {
                        let file = match cwd { Some(p) => p.join(".codex/config.toml"), None => codex_home()?.join("config.toml") };
                        rpc("config/value/write", json!({"keyPath":format!("plugins.{}.enabled", serde_json::to_string(&id)?), "value":enabled,"mergeStrategy":"upsert","filePath":file})).await?;
                    } else {
                        ensure!(scope == ExtScope::Global, "Codex skill switches are user-scoped");
                        rpc("skills/config/write", json!({"path":id,"enabled":enabled})).await?;
                    }
                }
                Ok(json!({}))
            }
            _ => bail!("not an extension operation"),
        }
    }).await.context("extension command timed out")?
}

fn project(scope: &ExtScope) -> Result<Option<PathBuf>> {
    match scope {
        ExtScope::Global => Ok(None),
        ExtScope::Project(cwd) => {
            ensure!(Path::new(cwd).is_absolute() && Path::new(cwd).is_dir(), "project must be an absolute directory");
            Ok(Some(PathBuf::from(cwd)))
        }
    }
}
fn user_home() -> Result<PathBuf> {
    Ok(std::env::var_os("HOME").context("HOME missing")?.into())
}
fn codex_home() -> Result<PathBuf> {
    Ok(std::env::var_os("CODEX_HOME").map(PathBuf::from).unwrap_or(user_home()?.join(".codex")))
}
fn skill_dir(harness: &str, scope: &ExtScope) -> Result<PathBuf> {
    if let Some(p) = project(scope)? {
        return Ok(p.join(if harness == "claude" { ".claude/skills" } else { ".agents/skills" }));
    }
    Ok(if harness == "claude" {
        std::env::var_os("CLAUDE_CONFIG_DIR").map(PathBuf::from).unwrap_or(user_home()?.join(".claude")).join("skills")
    } else {
        user_home()?.join(".agents/skills")
    })
}
async fn claude(args: &[&str], cwd: Option<&Path>) -> Result<Value> {
    let mut c = crate::harness::command("claude", &args.iter().map(|s| s.to_string()).collect::<Vec<_>>(), cwd.unwrap_or(Path::new("/")));
    c.stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    let out = c.output().await?;
    ensure!(out.status.success(), "claude plugin command failed ({})", out.status);
    Ok(serde_json::from_slice(&out.stdout)?)
}
async fn rpc(method: &str, params: Value) -> Result<Value> {
    let mut child = crate::harness::command("codex", &["app-server".into()], Path::new("/")).spawn()?;
    let (peer, mut incoming) =
        Peer::spawn(child.stdout.take().context("stdout missing")?, child.stdin.take().context("stdin missing")?, false);
    let request = async {
        peer.request("initialize", json!({"clientInfo":{"name":"canopy","version":env!("CARGO_PKG_VERSION")}})).await?;
        peer.notify("initialized", json!({}));
        peer.request(method, params).await
    };
    tokio::pin!(request);
    let result = loop {
        tokio::select! {
            result = &mut request => break result,
            Some(msg) = incoming.recv() => {
                if let Incoming::Request { id, .. } = msg {
                    peer.respond_err(id, -32601, "extension operations never approve commands");
                }
            }
        }
    };
    let _ = child.kill().await;
    let _ = child.wait().await;
    result
}
fn array(v: &Value) -> &[Value] {
    v.as_array().map(Vec::as_slice).unwrap_or_default()
}
fn row(harness: &str, kind: ExtKind, id: &str, name: &str, scope: ExtScope, enabled: bool, writable: bool) -> Extension {
    Extension { harness: harness.into(), kind, id: id.into(), name: name.into(), scope, enabled, writable, command: None }
}
async fn timed(name: &str, probe: impl std::future::Future<Output = Result<Value>>) -> Result<Value> {
    let start = std::time::Instant::now();
    let result = tokio::time::timeout(std::time::Duration::from_secs(10), probe).await;
    eprintln!("{name}: {:?}", start.elapsed());
    result.context("vendor probe timed out")?
}
async fn list(cwd: Option<String>) -> Result<Value> {
    if let Some(cwd) = &cwd {
        project(&ExtScope::Project(cwd.clone()))?;
    }
    let mut items = Vec::new();
    let mut errors = Vec::new();
    let cwds: Vec<_> = cwd.iter().collect();
    let (claude_plugins, codex_plugins, codex_skills) = tokio::join!(
        timed("Claude plugins", claude(&["plugin", "list", "--json"], cwd.as_deref().map(Path::new))),
        timed("Codex plugins", rpc("plugin/installed", json!({"cwds":cwds}))),
        timed("Codex skills", rpc("skills/list", json!({"cwds":cwds,"forceReload":true}))),
    );
    match claude_plugins {
        Ok(v) => {
            for p in array(v.get("plugins").unwrap_or(&v)) {
                let (Some(id), Some(enabled)) = (p["id"].as_str(), p["enabled"].as_bool()) else { continue };
                let scope = match p["scope"].as_str() {
                    Some("user") => ExtScope::Global,
                    Some("project") if cwd.is_some() => ExtScope::Project(cwd.clone().unwrap_or_default()),
                    _ => continue,
                };
                let mut r = row("claude", ExtKind::Plugin, id, id, scope, enabled, true);
                r.command = p["command"].as_str().map(str::to_owned);
                items.push(r);
            }
        }
        Err(e) => errors.push(format!("Claude plugins: {e}")),
    }
    match codex_plugins {
        Ok(v) => {
            for m in array(&v["marketplaces"]) {
                for p in array(&m["plugins"]) {
                    if p["installed"].as_bool() != Some(true) {
                        continue;
                    }
                    let (Some(id), Some(enabled)) = (p["id"].as_str(), p["enabled"].as_bool()) else { continue };
                    let mut r = row("codex", ExtKind::Plugin, id, p["name"].as_str().unwrap_or(id), ExtScope::Global, enabled, true);
                    r.command = p["command"].as_str().map(str::to_owned);
                    items.push(r.clone());
                    if let Some(cwd) = &cwd {
                        r.scope = ExtScope::Project(cwd.clone());
                        items.push(r);
                    }
                }
            }
        }
        Err(e) => errors.push(format!("Codex plugins: {e}")),
    }
    match codex_skills {
        Ok(v) => {
            for entry in array(&v["data"]) {
                for s in array(&entry["skills"]) {
                    let (Some(path), Some(name), Some(enabled)) = (s["path"].as_str(), s["name"].as_str(), s["enabled"].as_bool()) else {
                        continue;
                    };
                    items.push(row("codex", ExtKind::Skill, path, name, ExtScope::Global, enabled, true));
                }
            }
        }
        Err(e) => errors.push(format!("Codex skills: {e}")),
    }
    let mut scopes = vec![ExtScope::Global];
    if let Some(cwd) = cwd {
        scopes.push(ExtScope::Project(cwd));
    }
    let library = crate::paths::home()?.canonicalize()?.join("skills");
    for scope in scopes {
        for harness in ["claude", "codex"] {
            let dir = skill_dir(harness, &scope)?;
            for entry in std::fs::read_dir(&library).into_iter().flatten().flatten() {
                let target = entry.path();
                if !target.join("SKILL.md").is_file() {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().into_owned();
                let enabled = std::fs::read_link(dir.join(&name)).ok().as_ref() == Some(&target);
                items.push(row(harness, ExtKind::LibrarySkill, &name, &name, scope.clone(), enabled, true));
            }
            for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                if !entry.path().join("SKILL.md").is_file() || entry.file_type()?.is_symlink() {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().into_owned();
                if harness == "claude" {
                    items.push(row(harness, ExtKind::Skill, &entry.path().to_string_lossy(), &name, scope.clone(), true, false));
                }
            }
        }
    }
    Ok(json!({"items":items,"errors":errors}))
}
fn skill_change(row: &Extension, kind: ExtKind, enabled: bool) -> Result<()> {
    let name = if kind == ExtKind::AdoptSkill { &row.name } else { &row.id };
    ensure!(!name.is_empty() && name != "." && name != ".." && !name.contains('/') && !name.contains('\\'), "invalid skill name");
    let library = crate::paths::home()?.canonicalize()?.join("skills");
    std::fs::create_dir_all(&library)?;
    let target = library.join(name);
    let dir = skill_dir(&row.harness, &row.scope)?;
    let link = dir.join(name);
    if kind == ExtKind::AdoptSkill {
        ensure!(enabled, "adoption requires enabled=true");
        let source = Path::new(&row.id);
        let source =
            if source.file_name().is_some_and(|n| n == "SKILL.md") { source.parent().context("skill parent missing")? } else { source };
        ensure!(link == source, "only vendor-directory skills can be adopted");
        ensure!(!std::fs::symlink_metadata(&link)?.file_type().is_symlink(), "cannot adopt a symlink");
        ensure!(!target.exists(), "library skill already exists");
        std::fs::rename(&link, &target).context("cannot move skill to library")?;
        if let Err(e) = std::os::unix::fs::symlink(&target, &link) {
            std::fs::rename(&target, &link).context("cannot restore adopted skill")?;
            return Err(e.into());
        }
    } else {
        ensure!(target.join("SKILL.md").is_file(), "library skill missing");
        match std::fs::symlink_metadata(&link) {
            Ok(_) => {
                ensure!(std::fs::read_link(&link).ok().as_ref() == Some(&target), "refusing to replace an existing vendor skill");
                if !enabled {
                    std::fs::remove_file(link)?;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if enabled {
                    std::fs::create_dir_all(dir)?;
                    std::os::unix::fs::symlink(target, link)?;
                }
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
