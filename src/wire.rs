//! Wire: the edges between cells, and the small CLI around them. An edge drawn between two
//! cells of one space (or a born edge, parent to child) is an introduction: it says who may be
//! talked to. Agents message each other with the core's own `hc send`, `hc wait` and `hc read`;
//! nothing here carries a message. This file holds the edge model (`neighbors`, `born`, `all`),
//! `canopy wire` (who you are and your wired peers), `canopy spawn` and the peer-change hook.
//! Wired peers include file panes, which agents read with `canopy pane`; docs/wire.md explains it.

use crate::canvas::{Document, Wire};
use serde_json::Value;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

/// `sub:<parent>:<id>` taken apart.
pub fn sub_parts(key: &str) -> Option<(&str, &str)> {
    key.strip_prefix("sub:")?.split_once(':')
}

/// The cells wired to `me`, from drawn and born edges.
pub fn neighbors(wires: &[Wire], me: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for w in wires {
        let peer = if w.a == me { &w.b } else if w.b == me { &w.a } else { continue };
        if !out.contains(peer) {
            out.push(peer.clone());
        }
    }
    out
}

/// Born edges, parent to child: a sub-agent to its parent, and a cell started by `hc run`
/// from another cell to its parent (`parent_of` is the core's record). Computed, never stored.
pub fn born(doc: &Document, parent_of: impl Fn(&str) -> Option<String>) -> Vec<Wire> {
    doc.panes.iter().filter(|p| p.file().is_none()).filter_map(|c| {
        let key = c.id.clone();
        let parent = match &c.kind { crate::canvas::Kind::Sub { parent, .. } => Some(parent.clone()), _ => parent_of(&key) }?;
        (parent != key && doc.panes.iter().filter(|p| p.file().is_none()).any(|p| p.id == parent)).then_some(Wire { a: parent, b: key, born: true })
    }).collect()
}

/// Every edge of a document: its drawn wires and its born ones.
pub fn all(doc: &Document, parent_of: impl Fn(&str) -> Option<String>) -> Vec<Wire> {
    doc.wires.iter().cloned().chain(born(doc, parent_of)).collect()
}

/// The configured core (`crate::core_bin::path()`) with these arguments: its exit code, output
/// and error text. Every core call of the CLI entry points goes through here.
pub(crate) fn hc<S: AsRef<str>>(args: &[S], limit: Duration) -> Result<(i32, String, String), String> {
    let args: Vec<String> = args.iter().map(|a| a.as_ref().to_owned()).collect();
    crate::mods::run_blocking(&crate::core_bin::path().to_string_lossy(), &args, None, limit)
}

/// The message in an `hc` error (`{"error":{"message":...}}` on stderr), else the text.
fn hc_message(err: &str) -> String {
    serde_json::from_str::<Value>(err.trim()).ok().and_then(|v| v["error"]["message"].as_str().map(str::to_owned)).unwrap_or_else(|| err.trim().to_owned())
}

/// A terminal as `hc get --json` reports it (shape not checked against a live core, see docs/wire.md).
pub struct Rec {
    pub addr: String,
    pub name: String,
    pub state: Option<String>,
    pub agent: bool,
    pub parent: Option<String>,
    pub raw: Value,
}

impl Rec {
    /// Reads a record. None for a finished terminal: its cell is closed in all but name.
    pub fn parse(addr: &str, v: &Value) -> Option<Rec> {
        if v["state"] == "finished" {
            return None;
        }
        let report = &v["status"];
        let state = report["state"].as_str().or(report.as_str()).filter(|s| !s.is_empty()).map(str::to_owned);
        let launch: Vec<String> = v["argv"].as_array().into_iter().flatten().filter_map(|a| a.as_str().map(str::to_owned)).collect();
        let running: Vec<String> = v.pointer("/foreground/argv").and_then(Value::as_array).into_iter().flatten().filter_map(|a| a.as_str().map(str::to_owned)).collect();
        let program = crate::programs::of(report["app"].as_str(), if running.is_empty() { &launch } else { &running });
        // An agent in the foreground runs even when its status never reached the core
        // (an inherited agent environment can hide it, docs/canopy/harness-notes/claude-code.md).
        let state = state.or_else(|| (!running.is_empty() && crate::programs::agent_program(&program)).then(|| "running".into()));
        // An agent that exited leaves a shell with no status: its launch program still says what it was.
        // shortcut: told apart from a plain terminal by the launch program only; use a core fact if hc reports one.
        let agent = state.is_some() || crate::programs::agent_program(&crate::programs::of(None, &launch));
        let name = v["name"].as_str().filter(|s| !s.is_empty()).or(program["name"].as_str().filter(|_| agent)).unwrap_or(addr).to_owned();
        Some(Rec { addr: addr.into(), name, state, agent, parent: v["parent"].as_str().filter(|s| !s.is_empty()).map(str::to_owned), raw: v.clone() })
    }
    fn get(addr: &str) -> Option<Rec> {
        let (code, out, _) = hc(&["get", addr, "--json"], Duration::from_secs(10)).ok()?;
        let v: Value = serde_json::from_str(&out).ok().filter(|_| code == 0)?;
        // Some core versions wrap the record as {"terminal": {...}}.
        Rec::parse(addr, if v["terminal"].is_object() { &v["terminal"] } else { &v })
    }
}

// ---- `canopy wire` ------------------------------------------------------------------

type Failed = (i32, String);

fn fail<T>(msg: impl Into<String>) -> Result<T, Failed> {
    Err((1, msg.into()))
}

fn load_doc() -> Result<Document, Failed> {
    let path = crate::mods::app_dir().join("canvas.json");
    let bytes = std::fs::read(&path).or_else(|e| fail(format!("cannot read {}: {e}", path.display())))?;
    // Parse only: this runs in agents' shells and must never move the app's file aside.
    Document::parse(&bytes).or_else(|e| fail(format!("canvas.json: {e}")))?
        .map_or_else(|| fail("canvas.json is not a valid canvas"), Ok)
}

/// The caller: its terminal address and the cell it is.
struct Me {
    rec: Rec,
    key: String,
    wires: Vec<Wire>,
    doc: Document,
}

impl Me {
    fn new() -> Result<Me, Failed> {
        let addr = std::env::var("HC_TERMINAL_ID").ok().filter(|s| !s.is_empty()).map_or_else(|| fail("run this inside a Canopy terminal ($HC_TERMINAL_ID is not set)"), Ok)?;
        let rec = Rec::get(&addr).map_or_else(|| fail(format!("cannot read terminal {addr} from the core")), Ok)?;
        let doc = load_doc()?;
        let key = addr.clone();
        if !doc.panes.iter().filter(|p| p.file().is_none()).any(|c| c.id == key) {
            return fail(format!("{addr} is not a cell on the canvas, so nothing is wired to it"));
        }
        let mut wires = doc.wires.clone();
        // Born edges the CLI can know without scanning every terminal: its parent, and its children.
        wires.extend(born(&doc, |k| (k == key).then(|| rec.parent.clone()).flatten()).into_iter().filter(|w| w.b == key));
        if let Ok((0, out, _)) = hc(&["ls", "--json"], Duration::from_secs(10)) {
            let list: Value = serde_json::from_str(&out).unwrap_or_default();
            for t in list.as_array().or(list["terminals"].as_array()).into_iter().flatten() {
                if let (Some(child), Some(parent)) = (t["address"].as_str(), t["parent"].as_str()) {
                    if parent == key && doc.panes.iter().filter(|p| p.file().is_none()).any(|c| c.id == child) {
                        wires.push(Wire { a: key.clone(), b: child.into(), born: true });
                    }
                }
            }
        }
        Ok(Me { rec, key, wires, doc })
    }
    /// Peers it can reach: wired cells, except its own sub-agents (they live inside its agent).
    fn peers(&self) -> Vec<Peer> {
        neighbors(&self.wires, &self.key).into_iter().filter(|k| sub_parts(k).is_none_or(|(p, _)| p != self.key)).map(|k| Peer::new(&k, &self.doc)).collect()
    }
}

/// A wired cell as the core knows it now.
struct Peer {
    key: String,
    name: String,
    /// The terminal to address: the cell's own, or a sub-agent cell's parent.
    rec: Option<Rec>,
    /// The kind and description of a peer without a core terminal.
    pane: Option<(&'static str, String)>,
}

impl Peer {
    fn new(key: &str, doc: &Document) -> Peer {
        if let Some(pane) = doc.panes.iter().find(|p| p.id == key && p.file().is_some()) {
            let path = pane.file().unwrap();
            let name = std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.to_owned());
            return Peer { key: key.into(), name, rec: None, pane: Some(("file", format!("file {path} - canopy pane {} {key}", pane.verbs().join("|")))) };
        }
        match sub_parts(key) {
            Some((parent, id)) => {
                let rec = Rec::get(parent);
                let name = rec.as_ref().and_then(|r| crate::subs::hooks(&r.raw).into_iter().find(|s| s.id == id)).map_or_else(|| format!("sub {id}"), |s| s.name);
                Peer { key: key.into(), name, rec, pane: None }
            }
            None => {
                let rec = Rec::get(key);
                Peer { key: key.into(), name: rec.as_ref().map_or_else(|| key.to_owned(), |r| r.name.clone()), rec, pane: None }
            }
        }
    }
    /// What it is, for a list: its state, or what kind of cell it is.
    fn describe(&self) -> String {
        if let Some((_, about)) = &self.pane { return about.clone(); }
        match &self.rec {
            None => "closed".into(),
            Some(_) if sub_parts(&self.key).is_some() => "sub-agent: message its parent agent".into(),
            Some(r) if !r.agent => "plain terminal".into(),
            Some(r) => r.state.clone().unwrap_or_else(|| "not running".into()),
        }
    }
}

/// `canopy wire`: who you are and your wired peers, with their `hc` address and state.
fn run(args: &[String]) -> Result<String, Failed> {
    if !args.is_empty() {
        return fail("canopy wire only lists your peers: message one with hc send <terminal> \"<text>\"");
    }
    let _ = crate::programs::load();
    let me = Me::new()?;
    let peers = me.peers();
    let mut out = vec![format!("You are {} ({}).", me.rec.name, me.rec.addr)];
    if peers.is_empty() {
        out.push("No wires: nothing is connected to this cell. Ask the person to draw one.".into());
    } else {
        out.push("Wired to:".into());
        out.extend(peers.iter().map(|p| format!("  {} ({}): {}", p.name, p.rec.as_ref().map_or(p.key.as_str(), |r| r.addr.as_str()), p.describe())));
    }
    Ok(out.join("\n"))
}

/// `canopy wire ...` from a shell; the exit code.
pub fn cli(args: &[String]) -> i32 {
    match run(args) {
        Ok(text) => {
            println!("{text}");
            0
        }
        Err((code, msg)) => {
            eprintln!("canopy wire: {msg}");
            code
        }
    }
}

// ---- `canopy hook idle` -------------------------------------------------------------

/// Sessions started before this change still run Claude's `Stop` hook with this command. It does
/// nothing now (a spawned child reports with `hc send`); drop it one release after.
pub fn hook_idle() {}

// ---- `canopy spawn` -----------------------------------------------------------------

/// The argv `canopy spawn` runs: the program's `run` from programs.json, Canopy's flags, the
/// model, then the first message with one sentence telling the child to report to `parent`
/// (the terminal that spawned it) with `hc send` (nothing is added without a first message).
fn spawn_argv(run: &[String], model: Option<&str>, prompt: Option<&str>, parent: &str, exe: &str) -> Vec<String> {
    let mut argv = run.to_vec();
    crate::programs::append_flags(&mut argv, exe);
    if let Some(m) = model {
        argv.extend(["--model".into(), m.into()]);
    }
    argv.extend(prompt.map(|p| format!("{p}\n\nWhen you are done or stuck, report to your parent with: hc send {parent} \"<what you did or need>\"")));
    argv
}

/// `canopy spawn <program> --name <n> [--model <m>] [-- <prompt>]`: starts another agent as a
/// real terminal beside the caller, wired to it (the core records the caller as its parent).
fn spawn(args: &[String]) -> Result<String, Failed> {
    const USAGE: &str = "usage: canopy spawn <program> --name <name> [--model <model>] [-- <first message>]";
    let (head, prompt) = match args.iter().position(|a| a == "--") {
        Some(i) => (&args[..i], Some(args[i + 1..].join(" ")).filter(|p| !p.trim().is_empty())),
        None => (args, None),
    };
    let (mut program, mut name, mut model) = (None, None, None);
    let mut words = head.iter();
    while let Some(w) = words.next() {
        match w.as_str() {
            "--name" => name = words.next().cloned(),
            "--model" => model = words.next().cloned(),
            _ if program.is_none() && !w.starts_with('-') => program = Some(w.clone()),
            _ => return fail(format!("unexpected '{w}'\n{USAGE}")),
        }
    }
    let (Some(program), Some(name)) = (program, name) else { return fail(USAGE) };
    let _ = crate::programs::load();
    let known = crate::programs::startable();
    let Some(run) = known.iter().find(|p| p["key"] == program.as_str() || p["name"].as_str().is_some_and(|n| n.eq_ignore_ascii_case(&program))) else {
        let keys: Vec<&str> = known.iter().filter_map(|p| p["key"].as_str()).collect();
        return fail(format!("'{program}' is not an agent Canopy knows: {}", keys.join(", ")));
    };
    let run: Vec<String> = serde_json::from_value(run["run"].clone()).unwrap_or_default();
    let me = Me::new()?;
    let exe = std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| "canopy".into());
    let argv = spawn_argv(&run, model.as_deref(), prompt.as_deref(), &me.rec.addr, &exe);
    // `hc run` from inside a core terminal makes the caller the parent. `hc()` runs it from the
    // home folder, so the child's folder is passed: the one the caller works in.
    let mut hc_args = vec!["run".to_owned(), "--bg".into(), "--name".into(), name.clone()];
    if let Ok(cwd) = std::env::current_dir() {
        hc_args.extend(["--cwd".into(), cwd.to_string_lossy().into_owned()]);
    }
    // The parent's space, so the app places the child there (born wire included).
    if let Some(space) = me.rec.raw["labels"]["herdr.space"].as_str().filter(|v| !v.is_empty()) {
        hc_args.extend(["--label".into(), format!("herdr.space={space}")]);
    }
    hc_args.push("--".into());
    hc_args.extend(argv);
    let refs: Vec<&str> = hc_args.iter().map(String::as_str).collect();
    match hc(&refs, Duration::from_secs(30)) {
        Ok((0, ..)) => Ok(format!("started {name}, wired to you. It reports back with hc send; wait for it with hc wait --children any, and find its address with canopy wire")),
        Ok((_, _, err)) => fail(hc_message(&err)),
        Err(e) => fail(e),
    }
}

/// `canopy spawn ...` from a shell; the exit code.
pub fn spawn_cli(args: &[String]) -> i32 {
    match spawn(args) {
        Ok(text) => {
            println!("{text}");
            0
        }
        Err((code, msg)) => {
            eprintln!("canopy spawn: {msg}");
            code
        }
    }
}

// ---- `canopy hook wire` -------------------------------------------------------------

/// The one line a hook says about a changed peer list: everything on a fresh start or resume,
/// else what was added or removed. None when there is nothing to tell. Maps are key to label.
pub fn told_line(told: &std::collections::BTreeMap<String, String>, now: &std::collections::BTreeMap<String, String>, fresh: bool) -> Option<String> {
    let change = if fresh {
        now.values().cloned().collect::<Vec<_>>()
    } else {
        now.iter().filter(|(k, _)| !told.contains_key(*k)).map(|(_, v)| format!("+{v}")).chain(told.iter().filter(|(k, _)| !now.contains_key(*k)).map(|(_, v)| format!("-{v}"))).collect()
    };
    (!change.is_empty()).then(|| format!("Wired: {}. `canopy wire` lists your peers.", change.join(", ")))
}

// shortcut: one small file per terminal id, never swept; delete them when terminals end if they pile up.
fn told_path(terminal: &str) -> Option<PathBuf> {
    crate::subs::valid_id(terminal).then(|| crate::mods::app_dir().join("wires").join("told").join(format!("{terminal}.json")))
}

/// Session hook (`SessionStart`, `UserPromptSubmit`): tells the agent its drawn peers when they
/// changed since this terminal was last told, and says nothing otherwise. Best effort.
pub fn hook() {
    let Ok(terminal) = std::env::var("HC_TERMINAL_ID") else { return };
    let Some(told_file) = told_path(&terminal) else { return };
    let mut bytes = Vec::new();
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = std::thread::Builder::new().spawn(move || {
        let mut buf = Vec::new();
        if std::io::stdin().take(512 * 1024).read_to_end(&mut buf).is_ok() {
            let _ = tx.send(buf);
        }
    });
    if let Ok(b) = rx.recv_timeout(Duration::from_millis(700)) {
        bytes = b;
    }
    let payload: Value = serde_json::from_slice(&bytes).unwrap_or_default();
    let event = payload["hook_event_name"].as_str().unwrap_or("UserPromptSubmit");
    let fresh = event == "SessionStart";
    let Ok(doc) = load_doc() else { return };
    let keys = neighbors(&doc.wires, &terminal);
    let told: std::collections::BTreeMap<String, String> = std::fs::read_to_string(&told_file).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    // The common case costs a file read: same peers as last time, no `hc` call, no output.
    if !fresh && told.len() == keys.len() && keys.iter().all(|k| told.contains_key(k)) {
        return;
    }
    let _ = crate::programs::load();
    let now: std::collections::BTreeMap<String, String> = keys.iter().map(|k| {
        let peer = Peer::new(k, &doc);
        let program = peer.pane.as_ref().map(|(kind, _)| (*kind).to_owned()).or_else(|| peer.rec.as_ref().filter(|r| !r.agent).map(|_| "terminal".to_owned())).or_else(|| peer.rec.as_ref().and_then(|r| r.raw["status"]["app"].as_str().map(crate::programs::name)));
        (k.clone(), program.filter(|p| *p != peer.name).map_or(peer.name.clone(), |p| format!("{} ({p})", peer.name)))
    }).collect();
    if let Some(parent) = told_file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = crate::persist::write_atomic(&told_file, serde_json::to_string(&now).unwrap_or_default());
    if let Some(line) = told_line(&told, &now, fresh) {
        println!("{}", serde_json::json!({"hookSpecificOutput": {"hookEventName": event, "additionalContext": line}}));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn file_panes_wire_to_terminals_and_round_trip() {
        let mut d = doc(serde_json::json!({}));
        for id in ["p1", "p2"] {
            d.panes.push(crate::canvas::Pane { id: id.into(), kind: crate::canvas::Kind::File { path: format!("/tmp/{id}.md") }, rect: [0., 0., 560., 420.], z: 0, space: None });
        }
        assert!(d.connect("t1", "p1"));
        assert!(!d.connect("p1", "p2"));
        assert_eq!(serde_json::to_value(&d).unwrap()["wires"], serde_json::json!([{"a":"t1","b":"p1","born":false}]));
        let mut back = Document::parse(&serde_json::to_vec(&d).unwrap()).unwrap().unwrap();
        assert_eq!(back.wires, d.wires);
        assert_eq!(neighbors(&back.wires, "t1"), ["p1"]);
        back.panes.retain(|p| p.id != "p1");
        assert!(back.prune_wires());
        assert!(back.wires.is_empty());
    }

    #[test]
    fn file_peers_describe_their_verbs() {
        let mut d = doc(serde_json::json!({}));
        d.panes.push(crate::canvas::Pane { id: "p1".into(), kind: crate::canvas::Kind::File { path: "/tmp/a.md".into() }, rect: [0.; 4], z: 0, space: None });
        let peer = Peer::new("p1", &d);
        assert_eq!(peer.name, "a.md");
        assert!(peer.rec.is_none());
        assert_eq!(peer.describe(), "file /tmp/a.md - canopy pane read|shot|close p1");
    }


    fn doc(json: Value) -> Document {
        let mut base = serde_json::json!({"version":3,"camera":{"x":0,"y":0,"zoom":1},"view":"all","spaces":[],"panel":{"width":null},
            "panes":[{"id":"t1","kind":"terminal","props":{},"rect":[0,0,220,36],"z":1},{"id":"t2","kind":"terminal","props":{},"rect":[300,0,220,36],"z":2},{"id":"t3","kind":"terminal","props":{},"rect":[600,0,220,36],"z":3}]});
        base.as_object_mut().unwrap().extend(json.as_object().unwrap().clone());
        serde_json::from_value(base).unwrap()
    }

    #[test]
    fn old_canvas_files_load_and_edges_are_undirected_and_born_ones_computed() {
        let mut d = doc(serde_json::json!({}));
        assert!(d.wires.is_empty());
        assert!(d.connect("t1", "t2") && !d.connect("t2", "t1") && !d.connect("t1", "t1") && !d.connect("t1", "gone"));
        let saved = serde_json::to_value(&d).unwrap();
        assert_eq!(saved["wires"], serde_json::json!([{"a":"t1","b":"t2","born":false}]));
        let mut wires = d.wires.clone();
        wires.push(Wire { a: "t3".into(), b: "t1".into(), born: false });
        assert_eq!(neighbors(&wires, "t1"), ["t2", "t3"]);
        assert_eq!(neighbors(&wires, "t2"), ["t1"]);
        d.panes.push(crate::canvas::Pane { id: "sub:t1:a".into(), kind: crate::canvas::Kind::Sub { parent: "t1".into(), sub: "a".into() }, rect: [0., 100., 220., 36.], z: 4, space: None });
        let edges = born(&d, |k| (k == "t3").then(|| "t2".to_owned()));
        assert_eq!(edges, [Wire { a: "t2".into(), b: "t3".into(), born: true }, Wire { a: "t1".into(), b: "sub:t1:a".into(), born: true }]);
        // Closing a cell removes its wires.
        d.panes.retain(|c| c.id != "t2");
        assert!(d.prune_wires() && d.wires.is_empty());
    }

    #[test]
    fn edges_join_cells_of_one_space_only() {
        let spaces = serde_json::json!([{"space":"a","rect":[-10,-10,500,100],"color":"blue","frame":true},{"space":"b","rect":[550,-10,400,100],"color":"red","frame":true}]);
        let mut d = doc(serde_json::json!({"spaces": spaces}));
        for (card, space) in d.panes.iter_mut().zip(["a", "a", "b"]) { card.space = Some(space.into()); }
        assert!(d.connect("t1", "t2"));
        assert!(!d.connect("t2", "t3"), "t3 sits in another space");
    }

    #[test]
    fn spawn_runs_the_program_with_flags_model_and_task() {
        let run = vec!["claude".to_owned()];
        let argv = spawn_argv(&run, Some("haiku"), Some("review src/x.rs"), "t7", "/c");
        assert_eq!(argv[0], "claude");
        assert_eq!(argv[argv.len() - 3..argv.len() - 1], ["--model", "haiku"]);
        let task = argv.last().unwrap();
        assert!(task.starts_with("review src/x.rs\n\n") && task.contains("hc send t7 "), "{task}");
        assert_eq!(spawn_argv(&["pi".to_owned()], None, None, "t7", "/c"), ["pi"], "no task, nothing appended");
    }

    #[test]
    fn records_say_agent_and_state() {
        let claude = Rec::parse("t1", &serde_json::json!({"status":{"state":"working","app":"claude-code"},"argv":["claude"]})).unwrap();
        assert!(claude.agent && claude.state.as_deref() == Some("working"));
        let shell = Rec::parse("t2", &serde_json::json!({"argv":["zsh"]})).unwrap();
        assert!(!shell.agent && shell.state.is_none() && shell.name == "t2");
        let gone = Rec::parse("t3", &serde_json::json!({"argv":["claude"]})).unwrap();
        assert!(gone.agent && gone.state.is_none(), "an agent that exited");
        let back = Rec::parse("t3", &serde_json::json!({"argv":["/bin/zsh","-lic","\"$@\"; exec \"$0\" -li","/bin/zsh","claude"],"foreground":{"argv":["zsh"]}})).unwrap();
        assert!(back.agent && back.state.is_none(), "its shell is back in front");
        let hidden = Rec::parse("t6", &serde_json::json!({"status":null,"argv":["/bin/zsh","-lic","\"$@\"; exec \"$0\" -li","/bin/zsh","claude"],"foreground":{"argv":["claude","--settings","{}"]}})).unwrap();
        assert!(hidden.agent && hidden.state.as_deref() == Some("running"), "live, but no status reached the core");
        assert!(Rec::parse("t4", &serde_json::json!({"state":"finished"})).is_none());
        let named = Rec::parse("t5", &serde_json::json!({"name":"reviewer","status":"idle","argv":["codex"]})).unwrap();
        assert_eq!((named.name.as_str(), named.state.as_deref()), ("reviewer", Some("idle")));
    }

    #[test]
    fn the_hook_speaks_only_when_the_peers_changed() {
        let map = |l: &[(&str, &str)]| l.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<BTreeMap<_, _>>();
        let (none, ab) = (map(&[]), map(&[("a", "api"), ("b", "reviewer (codex)")]));
        assert_eq!(told_line(&none, &none, true), None);
        assert_eq!(told_line(&none, &ab, true).unwrap(), "Wired: api, reviewer (codex). `canopy wire` lists your peers.");
        assert_eq!(told_line(&ab, &ab, false), None);
        assert_eq!(told_line(&ab, &ab, true).unwrap(), "Wired: api, reviewer (codex). `canopy wire` lists your peers.");
        assert_eq!(told_line(&ab, &map(&[("b", "reviewer (codex)"), ("c", "web")]), false).unwrap(), "Wired: +web, -api. `canopy wire` lists your peers.");
    }

}
