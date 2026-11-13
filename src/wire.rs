//! Wire: postcards over a doorbell. An edge drawn between two cells of one space is
//! the address book and the permission; a postcard is a short text kept in the space's
//! thread file `<app home>/wires/<space>.jsonl` and typed into the receiver's terminal
//! only when it is at a turn boundary. This file holds the thread, the rules (edge,
//! length, breaker), the delivery line and `canopy wire`; docs/wire.md explains it.

use crate::canvas::{Document, Wire};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The person's name on every wire.
pub const PERSON: &str = "you";
pub const DEFAULT_LIMIT: usize = 1000;
pub const DEFAULT_BUDGET: u32 = 8;

/// One line of a thread file: a postcard (`tell`, `ask`, `reply`, or `idle`: a spawned child's
/// turn ended, sent to its parent), a breaker `reset`
/// on the wire `from`-`to`, or a `delivered`/`pending` receipt for postcard `n`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Line {
    #[serde(default)]
    pub n: u64,
    #[serde(default)]
    pub at: u64,
    /// Cell keys (`Card::key()`), or `you`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub from: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub to: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub re: Option<u64>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
    #[serde(default)]
    pub delivered: bool,
}

impl Line {
    /// A new postcard; its number is given when it is appended.
    pub fn card(from: &str, to: &str, kind: &str, re: Option<u64>, text: &str) -> Line {
        let refs = text.split_whitespace().filter(|w| w.len() > 1 && w.starts_with('@')).map(str::to_owned).collect();
        Line { from: from.into(), to: to.into(), kind: kind.into(), re, text: text.into(), refs, ..Default::default() }
    }
    fn is_card(&self) -> bool {
        matches!(self.kind.as_str(), "tell" | "ask" | "reply" | "idle")
    }
    fn on(&self, a: &str, b: &str) -> bool {
        (self.from == a && self.to == b) || (self.from == b && self.to == a)
    }
}

/// The thread file of a space (`None`: the unframed root space).
pub fn thread_path(space: Option<&str>) -> PathBuf {
    let name: String = space.unwrap_or("canvas").chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect();
    crate::mods::app_dir().join("wires").join(format!("{name}.jsonl"))
}

/// Whether a path is one of the thread files (the panel opens them by their resolved path).
pub fn is_thread(path: &str) -> bool {
    let path = Path::new(path);
    let dir = crate::mods::app_dir().join("wires");
    path.extension().is_some_and(|e| e == "jsonl")
        && path.parent().is_some_and(|p| p == dir || p.canonicalize().ok().is_some_and(|p| Some(p) == dir.canonicalize().ok()))
}

/// Postcards and resets in order, with receipts folded into `delivered`. Bad lines are skipped.
pub fn parse(text: &str) -> Vec<Line> {
    let mut out: Vec<Line> = vec![];
    for line in text.lines().filter_map(|l| serde_json::from_str::<Line>(l).ok()) {
        if matches!(line.kind.as_str(), "delivered" | "pending") {
            if let Some(card) = out.iter_mut().find(|c| c.n == line.n && c.is_card()) {
                card.delivered = line.kind == "delivered";
            }
        } else {
            out.push(line);
        }
    }
    out
}

/// A thread file's lines (none when it does not exist yet).
pub fn read(path: &Path) -> Vec<Line> {
    parse(&std::fs::read_to_string(path).unwrap_or_default())
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// The thread file open under an exclusive lock (released when dropped), with its lines.
struct Locked(std::fs::File, Vec<Line>);

impl Locked {
    // shortcut: a thread is re-read whole on every send and refresh; rotate or index it if one grows large.
    fn open(path: &Path) -> std::io::Result<Locked> {
        std::fs::create_dir_all(path.parent().unwrap_or(Path::new(".")))?;
        let mut file = std::fs::OpenOptions::new().create(true).read(true).append(true).open(path)?;
        file.lock()?;
        let mut text = String::new();
        file.read_to_string(&mut text)?;
        file.seek(std::io::SeekFrom::End(0))?;
        Ok(Locked(file, parse(&text)))
    }
    /// One `write` per line, so concurrent appenders never interleave.
    fn put(&mut self, line: &Line) -> std::io::Result<()> {
        let mut text = serde_json::to_string(line).map_err(std::io::Error::other)?;
        text.push('\n');
        self.0.write_all(text.as_bytes())
    }
}

/// Appends a postcard or reset, numbering it after the thread's last; returns its number.
pub fn append(path: &Path, mut line: Line) -> std::io::Result<u64> {
    let mut locked = Locked::open(path)?;
    line.n = locked.1.iter().map(|l| l.n).max().unwrap_or(0) + 1;
    line.at = now_ms();
    locked.put(&line)?;
    Ok(line.n)
}

fn receipt(kind: &str, n: u64) -> Line {
    Line { n, at: now_ms(), kind: kind.into(), delivered: kind == "delivered", ..Default::default() }
}

/// Takes the postcards waiting for `to` (marking them delivered under the lock, so the app's
/// postman and a `canopy wire` never type the same one twice). `release` undoes it.
pub fn claim(path: &Path, to: &str) -> std::io::Result<Vec<Line>> {
    let mut locked = Locked::open(path)?;
    let waiting: Vec<Line> = locked.1.iter().filter(|l| l.is_card() && l.to == to && !l.delivered).cloned().collect();
    for l in &waiting {
        locked.put(&receipt("delivered", l.n))?;
    }
    Ok(waiting)
}

/// Gives claimed postcards back, because typing them failed.
pub fn release(path: &Path, cards: &[Line]) {
    if let Ok(mut locked) = Locked::open(path) {
        for l in cards {
            let _ = locked.put(&receipt("pending", l.n));
        }
    }
}

/// The file a `canopy wire wait` holds locked while it blocks on the reply `n` for `key`.
fn wait_marker(thread: &Path, key: &str, n: u64) -> PathBuf {
    thread.parent().unwrap_or(Path::new(".")).join("waiting").join(format!("{key}.{n}"))
}

/// A `canopy wire wait` in progress: its marker file stays locked until this is dropped (or the
/// process dies, which is why a lock and not a file that could go stale).
struct Waiting {
    _lock: std::fs::File,
    marker: PathBuf,
}

impl Drop for Waiting {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.marker);
    }
}

fn hold_wait(thread: &Path, key: &str, n: u64) -> std::io::Result<Waiting> {
    let marker = wait_marker(thread, key, n);
    std::fs::create_dir_all(marker.parent().unwrap_or(Path::new(".")))?;
    let file = std::fs::OpenOptions::new().create(true).write(true).truncate(false).open(&marker)?;
    file.lock()?;
    Ok(Waiting { _lock: file, marker })
}

/// Whether a `canopy wire wait` is blocked on reply `n` for `key`, so the reply need not be typed.
fn waiter_alive(thread: &Path, key: &str, n: u64) -> bool {
    std::fs::OpenOptions::new().write(true).open(wait_marker(thread, key, n)).is_ok_and(|f| f.try_lock().is_err())
}

/// Postcards waiting for a cell.
pub fn waiting<'a>(lines: &'a [Line], to: &'a str) -> impl Iterator<Item = &'a Line> {
    lines.iter().filter(move |l| l.is_card() && l.to == to && !l.delivered)
}

/// Whether the wire `a`-`b` has used its `budget` of agent postcards since a person last wrote
/// to either end or reset it. Idle notices do not count. A budget of 0 turns the breaker off.
pub fn tripped(lines: &[Line], a: &str, b: &str, budget: u32) -> bool {
    let mut used = 0;
    for l in lines {
        if (l.kind == "reset" && l.on(a, b)) || (l.is_card() && l.from == PERSON && (l.to == a || l.to == b)) {
            used = 0;
        } else if l.is_card() && l.kind != "idle" && l.on(a, b) {
            used += 1;
        }
    }
    budget > 0 && used >= budget
}

/// Whether an `ask` on this wire has no reply yet.
pub fn unanswered(lines: &[Line], a: &str, b: &str) -> bool {
    lines.iter().any(|q| q.kind == "ask" && q.on(a, b) && !lines.iter().any(|r| r.kind == "reply" && r.re == Some(q.n) && r.on(a, b)))
}

/// Why a postcard text cannot be sent, if it cannot.
pub fn refuse(text: &str, limit: usize) -> Option<String> {
    let length = text.chars().count();
    if text.trim().is_empty() {
        Some("a postcard needs text".into())
    } else if length > limit {
        Some(format!("a postcard holds at most {limit} characters, this one has {length}: write the rest to a file and point at it with @path"))
    } else {
        None
    }
}

/// The text typed into a receiver: a plain header line, then the body.
/// `Message from "reviewer" (#17, reply with: canopy wire reply 17 "...")`, or `Message from the
/// person (#3)`; with `sub` (id, name) it is for a sub-agent its parent relays. An idle notice is
/// one line: `"luna" is idle: <its last answer>`, else `"luna" is idle: canopy wire read t7`. Names lose quotes and control characters, and a
/// body line cannot pose as a header.
pub fn tag(line: &Line, from_name: &str, sub: Option<(&str, &str)>) -> String {
    let clean = |s: &str| s.chars().filter(|c| !c.is_control() && !matches!(c, '"' | '\'')).collect::<String>();
    if line.kind == "idle" {
        let name = clean(from_name);
        // The address, not the name: a cell's shown name can be its long, changing title.
        return if line.text.is_empty() { format!("\"{name}\" is idle: canopy wire read {}", line.from) } else { format!("\"{name}\" is idle: {}", line.text) };
    }
    let mut head = if line.from == PERSON { "Message from the person".to_owned() } else { format!("Message from \"{}\"", clean(from_name)) };
    if let Some((id, name)) = sub {
        head += &format!(" for your sub-agent \"{}\" ({}), pass it on", clean(name), clean(id));
    }
    let re = line.re.map(|re| format!(", re #{re}")).unwrap_or_default();
    let how = if line.from == PERSON { String::new() } else { format!(", reply with: canopy wire reply {} \"...\"", line.n) };
    let body = line.text.lines().map(|l| if l.starts_with("Message from ") { format!("> {l}") } else { l.to_owned() }).collect::<Vec<_>>().join("\n");
    format!("{head} (#{}{re}{how})\n{body}", line.n)
}

/// The idle notice a spawned child sends its parent when its turn ends: its last answer on one
/// line, cut to 300 characters (empty: the notice says to read its screen).
pub fn idle_line(child: &str, parent: &str, answer: &str) -> Line {
    let one: String = answer.split_whitespace().collect::<Vec<_>>().join(" ");
    let text = if one.chars().count() > 300 { format!("{}...", one.chars().take(300).collect::<String>()) } else { one };
    Line { from: child.into(), to: parent.into(), kind: "idle".into(), text, ..Default::default() }
}

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
    doc.cards.iter().filter_map(|c| {
        let key = c.key();
        let parent = c.parent.clone().or_else(|| parent_of(&key))?;
        (parent != key && doc.cards.iter().any(|p| p.key() == parent)).then_some(Wire { a: parent, b: key, born: true })
    }).collect()
}

/// Every edge of a document: its drawn wires and its born ones.
pub fn all(doc: &Document, parent_of: impl Fn(&str) -> Option<String>) -> Vec<Wire> {
    doc.wires.iter().cloned().chain(born(doc, parent_of)).collect()
}

/// What the thread says about one wire, for drawing it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stat {
    pub waiting: usize,
    pub delivered: usize,
    pub unanswered: bool,
    pub tripped: bool,
}

pub fn stat(lines: &[Line], a: &str, b: &str, budget: u32) -> Stat {
    let cards = || lines.iter().filter(|l| l.is_card() && l.on(a, b));
    Stat { waiting: cards().filter(|l| !l.delivered).count(), delivered: cards().filter(|l| l.delivered).count(), unanswered: unanswered(lines, a, b), tripped: tripped(lines, a, b, budget) }
}

static NAMES: std::sync::Mutex<Option<std::collections::HashMap<String, String>>> = std::sync::Mutex::new(None);

/// The names the app shows for cell keys, kept for the thread reader (it runs off the window).
pub fn set_names(names: std::collections::HashMap<String, String>) {
    *NAMES.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(names);
}

/// A cell key's name from the last `set_names`, else the key.
pub fn name_of(key: &str) -> String {
    NAMES.lock().unwrap_or_else(std::sync::PoisonError::into_inner).as_ref().and_then(|n| n.get(key).cloned()).unwrap_or_else(|| key.to_owned())
}

/// A thread as markdown to read, one postcard after another. `name` turns a cell key into
/// the name to show. None when the text holds no postcards.
pub fn thread_markdown(text: &str, name: &dyn Fn(&str) -> String) -> Option<String> {
    let lines = parse(text);
    let who = |key: &str| if key == PERSON { PERSON.to_owned() } else { name(key) };
    let mut out = vec!["_Wire postcards of this space, oldest first. Type below to write on a wire as yourself._".to_owned()];
    for l in &lines {
        if l.is_card() {
            let re = l.re.map(|n| format!(" re #{n}")).unwrap_or_default();
            let state = if l.delivered { "" } else { " · waiting" };
            out.push(format!("**{} → {}** · {} #{}{re}{state}\n\n{}", who(&l.from), who(&l.to), l.kind, l.n, l.text));
        } else if l.kind == "reset" {
            out.push(format!("─ wire {} · {} reset", who(&l.from), who(&l.to)));
        }
    }
    (out.len() > 1).then(|| out.join("\n\n"))
}

/// What `hc send` said, as the sender is told.
#[derive(Debug, PartialEq)]
pub enum Sent {
    /// This many postcards were typed.
    Delivered(usize),
    /// Held in the thread until the receiver reaches a turn boundary.
    Queued,
    NotRunning,
    Failed(String),
}

impl Sent {
    pub fn say(&self) -> String {
        match self {
            Sent::Delivered(_) => "delivered".into(),
            Sent::Queued => "queued: the receiver is busy, it gets the postcard when its turn ends".into(),
            Sent::NotRunning => "not running: no agent runs in that cell, the postcard waits until one starts".into(),
            Sent::Failed(e) => format!("not typed ({e}); the postcard waits in the thread"),
        }
    }
}

/// The receiver as `hc` reports it: where to type, and whether its door is open.
pub struct Door {
    pub terminal: String,
    /// `idle`, `done`, `working`, `blocked`, `error`; None when no agent runs there.
    pub state: Option<String>,
    /// Its CLI queues a line typed mid-turn itself (`"wire": "queue"`).
    pub queue: bool,
}

impl Door {
    /// The `--if` flag for `hc send` when the door is open: `Some(None)` types without a check.
    fn open(&self) -> Option<Option<&'static str>> {
        match self.state.as_deref()? {
            "idle" => Some(Some("idle")),
            "done" => Some(None),
            "working" if self.queue => Some(None),
            _ => None,
        }
    }
}

/// Types the postcards waiting for cell `to` into its terminal if the door is open. `name` is
/// a cell key's name; `send` runs `hc send <terminal> <text> [--if idle]`: Ok, Err(Busy) for a
/// receiver that moved on, or Err(Gone)/Err(Other) otherwise.
pub fn ring(path: &Path, to: &str, door: &Door, name: &dyn Fn(&str) -> String, send: &dyn Fn(&str, &str, Option<&str>) -> Result<(), SendError>) -> Sent {
    if door.state.is_none() {
        return if read(path).iter().any(|l| l.is_card() && l.to == to && !l.delivered) { Sent::NotRunning } else { Sent::Delivered(0) };
    }
    let Some(flag) = door.open() else { return Sent::Queued };
    let cards = match claim(path, to) {
        Ok(cards) if cards.is_empty() => return Sent::Delivered(0),
        Ok(cards) => cards,
        Err(e) => return Sent::Failed(e.to_string()),
    };
    let sub = sub_parts(to).map(|(_, id)| id);
    let sub_name = sub.map(|_| name(to));
    let text = cards.iter().map(|l| tag(l, &if l.from == PERSON { PERSON.to_owned() } else { name(&l.from) }, sub.zip(sub_name.as_deref()))).collect::<Vec<_>>().join("\n\n");
    match send(&door.terminal, &text, flag) {
        Ok(()) => Sent::Delivered(cards.len()),
        Err(e) => {
            release(path, &cards);
            match e {
                SendError::Busy => Sent::Queued,
                SendError::Gone => Sent::NotRunning,
                SendError::Other(m) => Sent::Failed(m),
            }
        }
    }
}

pub enum SendError {
    Busy,
    Gone,
    Other(String),
}

/// `hc` with these arguments: its exit code, output and error text.
fn hc(args: &[&str], limit: Duration) -> Result<(i32, String, String), String> {
    let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
    crate::mods::run_blocking("hc", &args, None, limit)
}

/// The message in an `hc` error (`{"error":{"message":...}}` on stderr), else the text.
fn hc_message(err: &str) -> String {
    serde_json::from_str::<Value>(err.trim()).ok().and_then(|v| v["error"]["message"].as_str().map(str::to_owned)).unwrap_or_else(|| err.trim().to_owned())
}

/// `hc send`, which waits for a person's unfinished draft and refuses a blocked dialog.
pub fn hc_send(terminal: &str, text: &str, only_if: Option<&str>) -> Result<(), SendError> {
    let mut args = vec!["send", terminal, text];
    if let Some(state) = only_if {
        args.extend(["--if", state]);
    }
    match hc(&args, Duration::from_secs(90)) {
        Ok((0, ..)) => Ok(()),
        // blocked, or the status is not the one asked for: the next turn boundary rings.
        Ok((9 | 10, ..)) => Err(SendError::Busy),
        Ok((3 | 11, ..)) => Err(SendError::Gone),
        Ok((_, _, err)) => Err(SendError::Other(hc_message(&err))),
        Err(e) => Err(SendError::Other(e)),
    }
}

/// A terminal as `hc get --json` reports it (shape not checked against a live core, see docs/wire.md).
pub struct Rec {
    pub addr: String,
    pub name: String,
    pub state: Option<String>,
    pub agent: bool,
    pub queue: bool,
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
        // An agent that exited leaves a shell with no status: its launch program still says what it was.
        // shortcut: told apart from a plain terminal by the launch program only; use a core fact if hc reports one.
        let agent = state.is_some() || crate::programs::agent_program(&crate::programs::of(None, &launch));
        let name = v["name"].as_str().filter(|s| !s.is_empty()).or(program["name"].as_str().filter(|_| agent)).unwrap_or(addr).to_owned();
        Some(Rec { addr: addr.into(), name, state, agent, queue: crate::programs::queues(&program), parent: v["parent"].as_str().filter(|s| !s.is_empty()).map(str::to_owned), raw: v.clone() })
    }
    fn get(addr: &str) -> Option<Rec> {
        let (code, out, _) = hc(&["get", addr, "--json"], Duration::from_secs(10)).ok()?;
        let v: Value = serde_json::from_str(&out).ok().filter(|_| code == 0)?;
        // Some core versions wrap the record as {"terminal": {...}}.
        Rec::parse(addr, if v["terminal"].is_object() { &v["terminal"] } else { &v })
    }
    fn door(&self) -> Door {
        Door { terminal: self.addr.clone(), state: self.state.clone(), queue: self.queue }
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
    serde_json::from_slice(&bytes).or_else(|e| fail(format!("canvas.json: {e}")))
}

/// The caller: its terminal address and the cell it is.
struct Me {
    rec: Rec,
    key: String,
    doc: Document,
    wires: Vec<Wire>,
}

/// The caller's terminal, taken out of the environment: an `hc send` run from inside a terminal
/// wraps the text in `<hc from="t1">`, and Wire's line must reach the receiver as written.
fn my_terminal() -> Option<String> {
    let id = std::env::var("HC_TERMINAL_ID").ok().filter(|s| !s.is_empty());
    // SAFETY: CLI entry points call this first, before any thread starts.
    unsafe { std::env::remove_var("HC_TERMINAL_ID") };
    id
}

impl Me {
    fn new() -> Result<Me, Failed> {
        let addr = my_terminal().map_or_else(|| fail("run this inside a Canopy terminal ($HC_TERMINAL_ID is not set)"), Ok)?;
        let rec = Rec::get(&addr).map_or_else(|| fail(format!("cannot read terminal {addr} from the core")), Ok)?;
        let doc = load_doc()?;
        let key = addr.clone();
        if !doc.cards.iter().any(|c| c.key() == key) {
            return fail(format!("{addr} is not a cell on the canvas, so nothing is wired to it"));
        }
        let mut wires = doc.wires.clone();
        // Born edges the CLI can know without scanning every terminal: its parent, and its children.
        wires.extend(born(&doc, |k| (k == key).then(|| rec.parent.clone()).flatten()).into_iter().filter(|w| w.b == key));
        if let Ok((0, out, _)) = hc(&["ls", "--json"], Duration::from_secs(10)) {
            let list: Value = serde_json::from_str(&out).unwrap_or_default();
            for t in list.as_array().or(list["terminals"].as_array()).into_iter().flatten() {
                if let (Some(child), Some(parent)) = (t["address"].as_str(), t["parent"].as_str()) {
                    if parent == key && doc.cards.iter().any(|c| c.key() == child) {
                        wires.push(Wire { a: key.clone(), b: child.into(), born: true });
                    }
                }
            }
        }
        Ok(Me { rec, key, doc, wires })
    }
    /// The thread of the space this cell is in.
    fn thread(&self) -> PathBuf {
        thread_path(self.doc.space_of(&self.key).map(|s| s.space.as_str()))
    }
    /// Peers it can reach: wired cells, except its own sub-agents (it relays to those itself).
    fn peers(&self) -> Vec<Peer> {
        neighbors(&self.wires, &self.key).into_iter().filter(|k| sub_parts(k).is_none_or(|(p, _)| p != self.key)).map(|k| Peer::new(&k)).collect()
    }
    fn name(&self, key: &str) -> String {
        if key == self.key { self.rec.name.clone() } else { Peer::new(key).name }
    }
}

/// A wired cell as the core knows it now.
struct Peer {
    key: String,
    name: String,
    /// The terminal that gets typed into: the cell's own, or a sub-agent cell's parent.
    rec: Option<Rec>,
}

impl Peer {
    fn new(key: &str) -> Peer {
        match sub_parts(key) {
            Some((parent, id)) => {
                let rec = Rec::get(parent);
                let name = rec.as_ref().and_then(|r| crate::subs::hooks(&r.raw).into_iter().find(|s| s.id == id)).map_or_else(|| format!("sub {id}"), |s| s.name);
                Peer { key: key.into(), name, rec }
            }
            None => {
                let rec = Rec::get(key);
                Peer { key: key.into(), name: rec.as_ref().map_or_else(|| key.to_owned(), |r| r.name.clone()), rec }
            }
        }
    }
    fn is_sub(&self) -> bool {
        sub_parts(&self.key).is_some()
    }
    /// What it is, for a list: its state, or why it cannot be written to.
    fn describe(&self) -> String {
        match &self.rec {
            None => "closed".into(),
            Some(_) if self.is_sub() => "sub-agent, postcards go through its parent".into(),
            Some(r) if !r.agent => "plain terminal, read only: canopy wire read".into(),
            Some(r) => r.state.clone().unwrap_or_else(|| "not running".into()),
        }
    }
}

fn find_peer(me: &Me, who: &str) -> Result<Peer, Failed> {
    let peers = me.peers();
    // As `canopy wire` lists it too: `name (t2)`.
    let hit = |p: &Peer| p.key == who || p.name.eq_ignore_ascii_case(who) || p.rec.as_ref().is_some_and(|r| r.addr == who || format!("{} ({})", p.name, r.addr).eq_ignore_ascii_case(who));
    let mut found = peers.into_iter().filter(hit);
    match (found.next(), found.next()) {
        (Some(p), None) => Ok(p),
        (None, _) => fail(format!("no wire to '{who}': ask the person to draw one (canopy wire lists your peers)")),
        (Some(_), Some(_)) => fail(format!("'{who}' matches several peers: use the terminal id from canopy wire")),
    }
}

fn limits() -> (usize, u32) {
    let person: Value = std::fs::read_to_string(crate::settings::path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    (person["wire_limit"].as_u64().map_or(DEFAULT_LIMIT, |n| n as usize), person["wire_budget"].as_u64().map_or(DEFAULT_BUDGET, |n| n as u32))
}

/// Types waiting postcards for a cell, or reports why not.
fn ring_cell(me: &Me, peer: &Peer) -> Sent {
    let Some(rec) = &peer.rec else { return Sent::NotRunning };
    ring(&me.thread(), &peer.key, &rec.door(), &|k| me.name(k), &hc_send)
}

/// Whether `canopy wire` may send `text` to `peer` now.
fn check_send(me: &Me, peer: &Peer, text: &str) -> Result<(), Failed> {
    let (limit, budget) = limits();
    let Some(rec) = &peer.rec else { return fail(format!("{} is closed", peer.name)) };
    if !peer.is_sub() && !rec.agent {
        return fail(format!("{} is a plain terminal: it can only be read (canopy wire read {})", peer.name, peer.name));
    }
    if let Some(why) = refuse(text, limit) {
        return fail(why);
    }
    if tripped(&read(&me.thread()), &me.key, &peer.key, budget) {
        return fail(format!("the wire to {} tripped after {budget} postcards with no word from the person: ask them to reset it", peer.name));
    }
    Ok(())
}

/// Appends a postcard from `me` and rings; says how it went.
fn send(me: &Me, peer: &Peer, kind: &str, re: Option<u64>, text: &str) -> Result<String, Failed> {
    check_send(me, peer, text)?;
    let n = append(&me.thread(), Line::card(&me.key, &peer.key, kind, re, text)).or_else(|e| fail(format!("cannot write the thread: {e}")))?;
    // A sender blocked in `canopy wire wait` takes its reply from the thread: typing it too would show it twice.
    let how = if re.is_some_and(|n| waiter_alive(&me.thread(), &peer.key, n)) { Sent::Delivered(0) } else { ring_cell(me, peer) };
    let next = if kind == "ask" { format!("\nGet the reply with: canopy wire wait {n}") } else { String::new() };
    Ok(format!("{kind} #{n} to {}: {}{next}", peer.name, how.say()))
}

/// A postcard to `me`, by number.
fn mail(me: &Me, n: u64) -> Result<Line, Failed> {
    read(&me.thread()).into_iter().find(|l| l.n == n && l.is_card() && (l.to == me.key || l.from == me.key)).map_or_else(|| fail(format!("no postcard #{n} on your wires")), Ok)
}

fn show(me: &Me, l: &Line) -> String {
    tag(l, &if l.from == PERSON { PERSON.to_owned() } else { me.name(&l.from) }, None)
}

/// Blocks until a reply to `n` is in the thread, with no polling: the folder is watched.
fn wait_reply(me: &Me, n: u64, limit: Duration) -> Result<String, Failed> {
    let path = me.thread();
    let dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    std::fs::create_dir_all(&dir).or_else(|e| fail(e.to_string()))?;
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |_: notify::Result<notify::Event>| {
        let _ = tx.send(());
    }).or_else(|e| fail(format!("cannot watch {}: {e}", dir.display())))?;
    notify::Watcher::watch(&mut watcher, path.parent().unwrap_or(Path::new(".")), notify::RecursiveMode::NonRecursive).or_else(|e| fail(e.to_string()))?;
    let _waiting = hold_wait(&path, &me.key, n).or_else(|e| fail(format!("cannot wait: {e}")))?;
    let end = std::time::Instant::now() + limit;
    loop {
        let reply = read(&path).into_iter().find(|l| l.kind == "reply" && l.re == Some(n) && l.to == me.key);
        if let Some(l) = reply {
            // Consumed here: the postman must not type it again.
            if let Ok(mut locked) = Locked::open(&path) {
                let _ = locked.put(&receipt("delivered", l.n));
            }
            return Ok(show(me, &l));
        }
        let left = end.saturating_duration_since(std::time::Instant::now());
        if left.is_zero() || rx.recv_timeout(left).is_err() {
            return Err((124, format!("no reply to #{n} yet: wait again, or carry on and read it later with canopy wire")));
        }
    }
}

/// `canopy wire [tell|ask|reply|wait|read] ...`: the text to print, or the exit code and reason.
fn run(args: &[String]) -> Result<String, Failed> {
    let _ = crate::programs::load();
    let me = Me::new()?;
    let word = |i: usize, what: &str| args.get(i).cloned().map_or_else(|| fail(format!("usage: canopy wire {what}")), Ok);
    match args.first().map(String::as_str) {
        None => {
            let peers = me.peers();
            let mut out = vec![format!("You are {} ({}).", me.rec.name, me.rec.addr)];
            if peers.is_empty() {
                out.push("No wires: nothing is connected to this cell. Ask the person to draw one.".into());
            } else {
                out.push("Wired to:".into());
                out.extend(peers.iter().map(|p| format!("  {} ({}): {}", p.name, p.rec.as_ref().map_or(p.key.as_str(), |r| r.addr.as_str()), p.describe())));
            }
            // Pull delivery: mail that waited is handed over here and counts as delivered.
            let mine = claim(&me.thread(), &me.key).unwrap_or_default();
            if !mine.is_empty() {
                out.push("Mail for you:".into());
                out.extend(mine.iter().map(|l| show(&me, l)));
            }
            out.push("Send: canopy wire tell <peer> \"<text>\"   answer: reply <n> \"<text>\"   wait <n>   read <n|peer>".into());
            Ok(out.join("\n"))
        }
        Some(kind @ ("tell" | "ask")) => {
            let (who, text) = (word(1, &format!("{kind} <peer> \"<text>\""))?, word(2, &format!("{kind} <peer> \"<text>\""))?);
            send(&me, &find_peer(&me, &who)?, kind, None, &text)
        }
        Some("reply") => {
            let n: u64 = word(1, "reply <n> \"<text>\"")?.parse().or_else(|_| fail("reply needs the number of the postcard"))?;
            let text = word(2, "reply <n> \"<text>\"")?;
            let ask = mail(&me, n)?;
            if ask.to != me.key {
                return fail(format!("postcard #{n} was written by you: reply answers a postcard sent to you"));
            }
            send(&me, &find_peer(&me, &ask.from)?, "reply", Some(n), &text)
        }
        Some("wait") => {
            let n: u64 = word(1, "wait <n>")?.parse().or_else(|_| fail("wait needs the number of your ask"))?;
            let ask = mail(&me, n)?;
            if ask.from != me.key {
                return fail(format!("postcard #{n} was not sent by you"));
            }
            wait_reply(&me, n, Duration::from_secs(std::env::var("WIRE_WAIT").ok().and_then(|s| s.parse().ok()).unwrap_or(180)))
        }
        Some("read") => {
            let who = word(1, "read <n|peer>")?;
            if let Ok(n) = who.parse::<u64>() {
                return mail(&me, n).map(|l| show(&me, &l));
            }
            let peer = find_peer(&me, &who)?;
            let Some(rec) = &peer.rec else { return fail(format!("{} is closed", peer.name)) };
            match hc(&["read", &rec.addr, "--lines", "0"], Duration::from_secs(10)) {
                Ok((0, out, _)) => Ok(out.lines().rev().skip_while(|l| l.trim().is_empty()).take(40).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n")),
                Ok((_, _, err)) => fail(hc_message(&err)),
                Err(e) => fail(e),
            }
        }
        Some(other) => fail(format!("unknown wire command '{other}': tell, ask, reply, wait, read, or nothing to list")),
    }
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

/// Puts a spawned child's idle notice in its parent's thread when both are cells on the canvas.
/// Returns the thread it went to. Idle notices go to the spawner only, never along drawn edges.
pub fn post_idle(doc: &Document, child: &str, parent: &str, answer: &str) -> Option<PathBuf> {
    let on = |k: &str| doc.cards.iter().any(|c| c.key() == k);
    if !on(child) || !on(parent) {
        return None;
    }
    let path = thread_path(doc.space_of(child).map(|s| s.space.as_str()));
    append(&path, idle_line(child, parent, answer)).ok()?;
    Some(path)
}

/// Claude's `Stop` hook (payload on stdin): tells this terminal's parent agent that it is idle,
/// with its last answer (`last_assistant_message`). Best effort.
pub fn hook_idle() {
    let Some(terminal) = my_terminal() else { return };
    let mut text = String::new();
    let _ = std::io::stdin().take(512 * 1024).read_to_string(&mut text);
    let payload: Value = serde_json::from_str(&text).unwrap_or_default();
    let answer = payload["last_assistant_message"].as_str().unwrap_or_default();
    let _ = crate::programs::load();
    let Some(me) = Rec::get(&terminal) else { return };
    let Some(parent) = me.parent.as_deref().and_then(Rec::get).filter(|p| p.agent) else { return };
    let Ok(doc) = load_doc() else { return };
    if let Some(path) = post_idle(&doc, &terminal, &parent.addr, answer) {
        ring(&path, &parent.addr, &parent.door(), &|k| if k == terminal { me.name.clone() } else { k.to_owned() }, &hc_send);
    }
}

// ---- `canopy spawn` -----------------------------------------------------------------

/// The argv `canopy spawn` runs: the program's `run` from programs.json, Canopy's flags, the
/// model, then the first message. `path` keeps `canopy` reachable when the core's PATH lacks it.
fn spawn_argv(run: &[String], model: Option<&str>, prompt: Option<&str>, exe: &str, path: Option<&str>) -> Vec<String> {
    let mut argv = run.to_vec();
    crate::programs::append_flags(&mut argv, exe);
    if let Some(m) = model {
        argv.extend(["--model".into(), m.into()]);
    }
    argv.extend(prompt.map(str::to_owned));
    match path {
        Some(p) => ["env".to_owned(), format!("PATH={p}")].into_iter().chain(argv).collect(),
        None => argv,
    }
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
    let path = std::env::var("PATH").ok();
    let argv = spawn_argv(&run, model.as_deref(), prompt.as_deref(), &exe, path.as_deref());
    let mut hc_args = vec!["run".to_owned(), "--bg".into(), "--name".into(), name.clone(), "--parent".into(), me.rec.addr.clone()];
    // `hc` runs from the home folder; the child works where its parent does.
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
        Ok((0, ..)) => Ok(format!("started {name}, wired to you. When its turn ends you get a line \"{name}\" is idle; talk to it with canopy wire tell {name} \"...\"")),
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
    (!change.is_empty()).then(|| format!("Wired: {}. `canopy wire` lists your peers and mail.", change.join(", ")))
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
        let peer = Peer::new(k);
        let program = peer.rec.as_ref().filter(|r| !r.agent).map(|_| "terminal".to_owned()).or_else(|| peer.rec.as_ref().and_then(|r| r.raw["status"]["app"].as_str().map(crate::programs::name)));
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

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("canopy-wire-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("space.jsonl")
    }
    fn card(from: &str, to: &str, kind: &str, re: Option<u64>) -> Line {
        Line::card(from, to, kind, re, "hi")
    }
    fn doc(json: Value) -> Document {
        let mut base = serde_json::json!({"version":1,"camera":{"x":0,"y":0,"zoom":1},"view":"all","spaces":[],"panel":{"width":null},
            "cards":[{"terminal":"t1","rect":[0,0,220,36],"z":1},{"terminal":"t2","rect":[300,0,220,36],"z":2},{"terminal":"t3","rect":[600,0,220,36],"z":3}]});
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
        d.cards.push(crate::canvas::Card { terminal: String::new(), sub: Some("a".into()), parent: Some("t1".into()), rect: [0., 100., 220., 36.], z: 4, size: None });
        let edges = born(&d, |k| (k == "t3").then(|| "t2".to_owned()));
        assert_eq!(edges, [Wire { a: "t2".into(), b: "t3".into(), born: true }, Wire { a: "t1".into(), b: "sub:t1:a".into(), born: true }]);
        // Closing a cell removes its wires.
        d.cards.retain(|c| c.key() != "t2");
        assert!(d.prune_wires() && d.wires.is_empty());
    }

    #[test]
    fn edges_join_cells_of_one_space_only() {
        let spaces = serde_json::json!([{"space":"a","rect":[-10,-10,500,100],"color":"blue","frame":true},{"space":"b","rect":[550,-10,400,100],"color":"red","frame":true}]);
        let mut d = doc(serde_json::json!({"spaces": spaces}));
        assert!(d.connect("t1", "t2"));
        assert!(!d.connect("t2", "t3"), "t3 sits in another space");
    }

    #[test]
    fn the_thread_appends_numbers_and_folds_receipts() {
        let path = temp("thread");
        assert!(read(&path).is_empty());
        assert_eq!(append(&path, card("t1", "t2", "ask", None)).unwrap(), 1);
        assert_eq!(append(&path, card(PERSON, "t2", "tell", None)).unwrap(), 2);
        let lines = read(&path);
        assert_eq!(lines.len(), 2);
        assert_eq!(waiting(&lines, "t2").count(), 2);
        let claimed = claim(&path, "t2").unwrap();
        assert_eq!(claimed.len(), 2);
        assert!(claim(&path, "t2").unwrap().is_empty(), "claimed once");
        release(&path, &claimed[..1]);
        assert_eq!(waiting(&read(&path), "t2").map(|l| l.n).collect::<Vec<_>>(), [1]);
        // Every line stays one JSON object, receipts included, and a bad line is skipped.
        let text = std::fs::read_to_string(&path).unwrap() + "not json\n";
        assert!(text.lines().take(5).all(|l| serde_json::from_str::<Value>(l).is_ok()));
        assert_eq!(parse(&text).len(), 2);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn the_breaker_trips_at_the_budget_and_a_person_resets_it() {
        let mut lines = vec![];
        for n in 1..=8 {
            lines.push(Line { n, ..card(if n % 2 == 0 { "t1" } else { "t2" }, if n % 2 == 0 { "t2" } else { "t1" }, "tell", None) });
            assert_eq!(tripped(&lines, "t1", "t2", 8), n == 8);
        }
        assert!(!tripped(&lines, "t1", "t2", 0), "0 turns it off");
        assert!(!tripped(&lines, "t1", "t3", 8), "another wire");
        let mut written = lines.clone();
        written.push(Line { n: 9, ..card(PERSON, "t1", "tell", None) });
        assert!(!tripped(&written, "t1", "t2", 8));
        let mut reset = lines.clone();
        reset.push(Line { n: 9, kind: "reset".into(), from: "t2".into(), to: "t1".into(), ..Default::default() });
        assert!(!tripped(&reset, "t1", "t2", 8));
        reset.push(Line { n: 10, ..card("t1", "t2", "tell", None) });
        assert!(!tripped(&reset, "t1", "t2", 8));
    }

    #[test]
    fn a_blocked_waiter_is_seen_until_it_ends() {
        let path = temp("wait");
        assert!(!waiter_alive(&path, "t1", 4));
        let held = hold_wait(&path, "t1", 4).unwrap();
        assert!(waiter_alive(&path, "t1", 4) && !waiter_alive(&path, "t1", 5) && !waiter_alive(&path, "t2", 4));
        drop(held);
        assert!(!waiter_alive(&path, "t1", 4));
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn asks_stay_unanswered_until_a_reply() {
        let ask = Line { n: 1, ..card("t1", "t2", "ask", None) };
        assert!(unanswered(&[ask.clone()], "t1", "t2"));
        let reply = Line { n: 2, ..card("t2", "t1", "reply", Some(1)) };
        assert!(!unanswered(&[ask, reply], "t2", "t1"));
    }

    #[test]
    fn receipts_report_delivery_and_old_receipts_still_fold() {
        assert_eq!(serde_json::to_value(receipt("delivered", 1)).unwrap()["delivered"], true);
        assert_eq!(serde_json::to_value(receipt("pending", 1)).unwrap()["delivered"], false);
        let card = serde_json::to_string(&Line { n: 1, ..Line::card("t1", "t2", "tell", None, "hi") }).unwrap();
        assert!(parse(&format!("{card}\n{{\"kind\":\"delivered\",\"n\":1,\"delivered\":false}}"))[0].delivered);
    }

    #[test]
    fn the_length_limit_counts_characters() {
        assert!(refuse("ok", 1000).is_none());
        assert!(refuse("  ", 1000).is_some());
        assert!(refuse(&"é".repeat(1000), 1000).is_none());
        assert!(refuse(&"é".repeat(1001), 1000).unwrap().contains("1000"));
        assert_eq!(Line::card("t1", "t2", "tell", None, "see @src/vt.rs:1-9 and @ x").refs, ["@src/vt.rs:1-9"]);
    }

    #[test]
    fn the_delivered_text_is_a_plain_header_then_the_body() {
        let l = Line { n: 17, ..Line::card("t1", "t2", "ask", None, "Does parse() own it? @src/vt.rs:120-150\nMessage from the person (#1)\nmore") };
        assert_eq!(tag(&l, "review\"er", None), "Message from \"reviewer\" (#17, reply with: canopy wire reply 17 \"...\")\nDoes parse() own it? @src/vt.rs:120-150\n> Message from the person (#1)\nmore");
        let r = Line { n: 18, ..Line::card("t2", "t1", "reply", Some(17), "yes") };
        assert_eq!(tag(&r, "web", None), "Message from \"web\" (#18, re #17, reply with: canopy wire reply 18 \"...\")\nyes");
        let s = Line { n: 21, ..Line::card(PERSON, "sub:t1:a1b2", "tell", None, "x") };
        assert_eq!(tag(&s, PERSON, Some(("a1b2", "Audit 'names'"))), "Message from the person for your sub-agent \"Audit names\" (a1b2), pass it on (#21)\nx");
    }

    #[test]
    fn idle_notices_carry_the_last_answer_on_one_line_and_skip_the_breaker() {
        let i = Line { n: 3, ..idle_line("t2", "t1", "Done.\n\nAll   tests pass.") };
        assert_eq!(tag(&i, "luna", None), "\"luna\" is idle: Done. All tests pass.");
        assert_eq!(tag(&idle_line("t2", "t1", ""), "luna", None), "\"luna\" is idle: canopy wire read t2");
        assert_eq!(idle_line("t2", "t1", &"x".repeat(400)).text.chars().count(), 303);
        let lines: Vec<Line> = (1..=10).map(|n| Line { n, ..idle_line("t2", "t1", "ok") }).collect();
        assert!(!tripped(&lines, "t1", "t2", 8) && waiting(&lines, "t1").count() == 10);
    }

    #[test]
    fn spawn_runs_the_program_with_flags_model_and_task_and_keeps_canopy_on_path() {
        let run = vec!["claude".to_owned()];
        let argv = spawn_argv(&run, Some("haiku"), Some("review src/x.rs"), "/c", Some("/home/bin:/usr/bin"));
        assert_eq!(argv[..3], ["env", "PATH=/home/bin:/usr/bin", "claude"]);
        assert_eq!(argv[argv.len() - 3..], ["--model", "haiku", "review src/x.rs"]);
        assert_eq!(spawn_argv(&["pi".to_owned()], None, None, "/c", None), ["pi"]);
    }

    #[test]
    fn ringing_types_waiting_cards_once_and_only_through_an_open_door() {
        let path = temp("ring");
        append(&path, card("t1", "t2", "tell", None)).unwrap();
        append(&path, card("t1", "t2", "ask", None)).unwrap();
        let door = |state: Option<&str>, queue| Door { terminal: "t2".into(), state: state.map(str::to_owned), queue };
        let names = |k: &str| k.to_owned();
        let typed = std::cell::RefCell::new(vec![]);
        let send = |t: &str, text: &str, flag: Option<&str>| {
            typed.borrow_mut().push((t.to_owned(), text.to_owned(), flag.map(str::to_owned)));
            Ok(())
        };
        assert_eq!(ring(&path, "t2", &door(None, false), &names, &send), Sent::NotRunning);
        assert_eq!(ring(&path, "t2", &door(Some("working"), false), &names, &send), Sent::Queued);
        assert_eq!(ring(&path, "t2", &door(Some("blocked"), true), &names, &send), Sent::Queued);
        assert!(typed.borrow().is_empty());
        assert_eq!(ring(&path, "t2", &door(Some("idle"), false), &names, &send), Sent::Delivered(2));
        assert_eq!(typed.borrow().len(), 1, "one paste for the whole batch");
        assert_eq!(typed.borrow()[0].2.as_deref(), Some("idle"));
        assert_eq!(typed.borrow()[0].1.matches("Message from ").count(), 2);
        assert_eq!(ring(&path, "t2", &door(Some("idle"), false), &names, &send), Sent::Delivered(0));
        // A queueing CLI takes it mid-turn, without a status check.
        append(&path, card("t1", "t2", "tell", None)).unwrap();
        assert_eq!(ring(&path, "t2", &door(Some("working"), true), &names, &send), Sent::Delivered(1));
        assert_eq!(typed.borrow()[1].2, None);
        // A send that finds the receiver moved on puts the card back.
        append(&path, card("t1", "t2", "tell", None)).unwrap();
        let busy = |_: &str, _: &str, _: Option<&str>| Err(SendError::Busy);
        assert_eq!(ring(&path, "t2", &door(Some("idle"), false), &names, &busy), Sent::Queued);
        assert_eq!(waiting(&read(&path), "t2").count(), 1);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn records_say_agent_state_and_queueing() {
        let claude = Rec::parse("t1", &serde_json::json!({"status":{"state":"working","app":"claude-code"},"argv":["claude"]})).unwrap();
        assert!(claude.agent && claude.queue && claude.state.as_deref() == Some("working"));
        let shell = Rec::parse("t2", &serde_json::json!({"argv":["zsh"]})).unwrap();
        assert!(!shell.agent && shell.state.is_none() && shell.name == "t2");
        let gone = Rec::parse("t3", &serde_json::json!({"argv":["claude"]})).unwrap();
        assert!(gone.agent && gone.state.is_none(), "an agent that exited");
        assert!(Rec::parse("t4", &serde_json::json!({"state":"finished"})).is_none());
        let named = Rec::parse("t5", &serde_json::json!({"name":"reviewer","status":"idle","argv":["codex"]})).unwrap();
        assert_eq!((named.name.as_str(), named.state.as_deref(), named.queue), ("reviewer", Some("idle"), false));
    }

    #[test]
    fn the_hook_speaks_only_when_the_peers_changed() {
        let map = |l: &[(&str, &str)]| l.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<BTreeMap<_, _>>();
        let (none, ab) = (map(&[]), map(&[("a", "api"), ("b", "reviewer (codex)")]));
        assert_eq!(told_line(&none, &none, true), None);
        assert_eq!(told_line(&none, &ab, true).unwrap(), "Wired: api, reviewer (codex). `canopy wire` lists your peers and mail.");
        assert_eq!(told_line(&ab, &ab, false), None);
        assert_eq!(told_line(&ab, &ab, true).unwrap(), "Wired: api, reviewer (codex). `canopy wire` lists your peers and mail.");
        assert_eq!(told_line(&ab, &map(&[("b", "reviewer (codex)"), ("c", "web")]), false).unwrap(), "Wired: +web, -api. `canopy wire` lists your peers and mail.");
    }

    #[test]
    fn a_thread_reads_as_a_conversation_and_keeps_text_literal() {
        let path = temp("md");
        append(&path, Line::card("t1", "t2", "ask", None, "**bold** <b>x</b>")).unwrap();
        append(&path, Line::card(PERSON, "t2", "tell", None, "hey")).unwrap();
        let md = thread_markdown(&std::fs::read_to_string(&path).unwrap(), &|k| format!("cell {k}")).unwrap();
        assert!(md.contains("**cell t1 → cell t2** · ask #1 · waiting\n\n**bold** <b>x</b>"));
        assert!(md.contains("**you → cell t2** · tell #2"));
        assert!(thread_markdown("", &|k| k.into()).is_none());
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
