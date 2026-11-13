// Modified for Canopy.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

const PAYLOAD_LIMIT: u64 = 512 * 1024;

#[derive(Clone, Deserialize, Serialize)]
pub struct Hook {
    pub name: String,
    pub transcript: PathBuf,
    /// Its turn ended (SubagentStop). It may still be on standby for messages, so it stays.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub idle: bool,
}

#[derive(Clone)]
pub struct Sub {
    pub id: String,
    pub name: String,
    pub program: Value,
    pub transcript: Option<PathBuf>,
    pub idle: bool,
}

pub fn valid_id(id: &str) -> bool {
    (1..=64).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn payload(bytes: &[u8], stop: bool) -> Option<(String, Hook)> {
    if bytes.len() as u64 > PAYLOAD_LIMIT { return None; }
    let value: Value = serde_json::from_slice(bytes).ok()?;
    let id = value["agent_id"].as_str().filter(|id| valid_id(id))?.to_owned();
    let transcript = value["agent_transcript_path"].as_str().map(PathBuf::from).or_else(|| {
        let parent = PathBuf::from(value["transcript_path"].as_str()?);
        (parent.extension()? == "jsonl").then(|| parent.with_extension("").join("subagents").join(format!("agent-{id}.jsonl")))
    })?;
    if !transcript.is_absolute() { return None; }
    let name = value["agent_type"].as_str().filter(|s| !s.is_empty() && s.len() <= 256)?.to_owned();
    Some((id, Hook { name, transcript, idle: stop }))
}

pub fn hook(action: Option<&str>) {
    let stop = match action { Some("sub-start") => false, Some("sub-stop") => true, _ => return };
    let Ok(terminal) = std::env::var("HC_TERMINAL_ID") else { return };
    // A hook must never fall back to a different core or start one without its terminal's socket.
    if terminal.is_empty() || std::env::var_os("HC_SOCKET").is_none_or(|s| s.is_empty() || !std::path::Path::new(&s).exists()) { return; }
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = std::thread::Builder::new().spawn(move || {
        let mut bytes = Vec::new();
        if std::io::stdin().take(PAYLOAD_LIMIT + 1).read_to_end(&mut bytes).is_ok() {
            let _ = tx.send(bytes);
        }
    });
    let Ok(bytes) = rx.recv_timeout(Duration::from_millis(700)) else { return };
    let Some((id, data)) = payload(&bytes, stop) else { return };
    let Ok(value) = serde_json::to_string(&data) else { return };
    let args = vec!["label".into(), terminal, format!("canopy.sub.{id}={value}")];
    let _ = crate::mods::run_blocking("hc", &args, None, Duration::from_millis(800));
}

pub fn hooks(record: &Value) -> Vec<Sub> {
    record["labels"].as_object().into_iter().flatten().filter_map(|(key, value)| {
        let id = key.strip_prefix("canopy.sub.").filter(|id| valid_id(id))?;
        let hook: Hook = serde_json::from_str(value.as_str()?).ok()?;
        if !hook.transcript.is_absolute() || hook.name.is_empty() || hook.name.len() > 256 { return None; }
        Some(Sub { id: id.into(), name: hook.name, program: record["program"].clone(), transcript: Some(hook.transcript), idle: hook.idle })
    }).collect()
}

/// The sidecar Claude Code writes beside a sub-agent's transcript: `<dir>/subagents/agent-<id>.meta.json`.
fn transcript_meta(path: &std::path::Path) -> Option<Value> {
    let name = path.file_name()?.to_str()?.strip_suffix(".jsonl")?;
    (name.starts_with("agent-") && path.parent()?.file_name()? == "subagents").then_some(())?;
    serde_json::from_str(&std::fs::read_to_string(path.with_file_name(format!("{name}.meta.json"))).ok()?).ok()
}

/// A sub-agent transcript's panel title: its type and description, "· done" once it ended.
pub fn transcript_title(path: &str) -> Option<String> {
    let meta = transcript_meta(std::path::Path::new(path))?;
    let mut title = [meta["agentType"].as_str(), meta["description"].as_str()].into_iter().flatten()
        .filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ");
    if title.is_empty() { title = "Sub-agent".into(); }
    if meta["finished"] == true { title.push_str(" · done"); }
    Some(title)
}

/// A Claude Code transcript (JSONL of `user`/`assistant` entries) as markdown to read, the
/// way its terminal would show it: the task, its words, each tool call as `● Tool what` with
/// its result's first line, its hand-back report, and "Done". None when the text is not one.
/// shortcut: Claude Code's transcript shape only; add another agent's when its sub-agents get pills.
pub fn transcript_markdown(text: &str, path: &str) -> Option<String> {
    let first = |s: &str| -> String {
        let line = s.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or_default();
        line.chars().take(100).collect::<String>().replace('`', "'")
    };
    let mut out = vec!["_A sub-agent's work, read-only. Its questions are answered in its parent agent._".to_owned()];
    let mut seen = false;
    let mut asked = false;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let entry: Value = serde_json::from_str(line).ok()?;
        let user = match entry["type"].as_str() {
            Some("user") => true,
            Some("assistant") => false,
            _ => continue,
        };
        seen = true;
        // Claude Code's own injected reminders, not the parent's words.
        if entry["isMeta"] == true { continue; }
        let content = &entry["message"]["content"];
        let texts: Vec<&str> = content.as_str().into_iter()
            .chain(content.as_array().into_iter().flatten().filter(|b| b["type"] == "text").filter_map(|b| b["text"].as_str()))
            .map(str::trim).filter(|t| !t.is_empty()).collect();
        for t in texts {
            if !user { out.push(t.to_owned()); }
            else if !asked { asked = true; out.push(format!("**Task**\n\n{t}")); }
            else { out.push(format!("› {t}")); }
        }
        for block in content.as_array().into_iter().flatten() {
            match block["type"].as_str() {
                Some("tool_use") => {
                    let input = &block["input"];
                    let what = ["command", "file_path", "path", "pattern", "url", "query", "description"].iter().find_map(|k| input[*k].as_str());
                    match (what, input["message"].as_str()) {
                        // A hand-back tool carries the sub-agent's report to its parent.
                        (None, Some(report)) => out.push(format!("**Report**\n\n{}", report.trim())),
                        _ => out.push(format!("`● {}` {}", block["name"].as_str().unwrap_or("tool"), first(what.unwrap_or_default()))),
                    }
                }
                Some("tool_result") => {
                    let result = &block["content"];
                    let text = result.as_str().map(str::to_owned).unwrap_or_else(|| result.as_array().into_iter().flatten()
                        .filter_map(|b| b["text"].as_str()).collect::<Vec<_>>().join("\n"));
                    let mark = if block["is_error"] == true { "✗" } else { "⎿" };
                    out.push(format!("> {mark} {}", first(&text)));
                }
                _ => {}
            }
        }
    }
    if transcript_meta(std::path::Path::new(path)).is_some_and(|m| m["finished"] == true) { out.push("─ Done".into()); }
    seen.then(|| out.join("\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn transcripts_read_as_a_conversation() {
        let dir = std::env::temp_dir().join(format!("canopy-subs-{}", std::process::id())).join("subagents");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("agent-abc.jsonl");
        std::fs::write(dir.join("agent-abc.meta.json"), r#"{"agentType":"general-purpose","description":"Check","finished":true}"#).unwrap();
        let text = [
            json!({"type":"user","message":{"content":"reply with pong"}}),
            json!({"type":"user","isMeta":true,"message":{"content":"<system-reminder>hand back</system-reminder>"}}),
            json!({"type":"attachment","attachment":{}}),
            json!({"type":"assistant","message":{"content":[{"type":"thinking","thinking":"hm"},{"type":"tool_use","name":"Bash","input":{"command":"echo `pong`\nmore"}}]}}),
            json!({"type":"user","message":{"content":[{"type":"tool_result","content":"\npong\nnext"}]}}),
            json!({"type":"assistant","message":{"content":[{"type":"tool_use","name":"SubagentHandback","input":{"message":"pong done"}}]}}),
        ].iter().map(Value::to_string).collect::<Vec<_>>().join("\n");
        let path = path.to_str().unwrap();
        assert_eq!(transcript_markdown(&text, path).unwrap().split("\n\n").skip(1).collect::<Vec<_>>(),
            ["**Task**", "reply with pong", "`● Bash` echo 'pong'", "> ⎿ pong", "**Report**", "pong done", "─ Done"]);
        assert_eq!(transcript_title(path).unwrap(), "general-purpose · Check · done");
        assert!(transcript_title("/tmp/x/agent-abc.jsonl").is_none());
        assert!(transcript_markdown("{\"other\":1}", path).is_none());
        assert!(transcript_markdown("not json", path).is_none());
        std::fs::remove_dir_all(dir.parent().unwrap()).unwrap();
    }

    #[test]
    fn hook_payload_and_ids() {
        let bytes = serde_json::to_vec(&json!({"agent_id":"abc_1-Z","agent_type":"review","transcript_path":"/tmp/session.jsonl"})).unwrap();
        let (id, data) = payload(&bytes, false).unwrap();
        assert_eq!(id, "abc_1-Z");
        let children = hooks(&json!({"program":{"icon":"icons/agent.svg"},"labels":{"canopy.sub.abc_1-Z":serde_json::to_string(&data).unwrap()}}));
        assert_eq!(children[0].id, id);
        assert_eq!(children[0].program["icon"], "icons/agent.svg");
        assert_eq!(data.name, "review");
        assert_eq!(data.transcript, PathBuf::from("/tmp/session/subagents/agent-abc_1-Z.jsonl"));
        assert!(payload(&bytes, true).unwrap().1.idle, "a stopped sub-agent stays, idle");
        for id in ["", "../escape", "a/b", "a b", "é", &"x".repeat(65)] { assert!(!valid_id(id)); }
        let stop = json!({"agent_id":"abc", "agent_transcript_path":"/tmp/done.jsonl", "agent_type":"helper"});
        assert_eq!(payload(&serde_json::to_vec(&stop).unwrap(), false).unwrap().1.transcript, PathBuf::from("/tmp/done.jsonl"));
        assert!(payload(&vec![b' '; PAYLOAD_LIMIT as usize + 1], false).is_none());
        assert!(payload(b"broken", false).is_none());
        assert!(hooks(&json!({"labels":{"canopy.sub.bad":"{}"}})).is_empty());
    }
}
