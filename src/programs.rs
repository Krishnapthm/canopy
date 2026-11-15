// Modified for Canopy.
//! What runs in a terminal, as people see it: a name, an icon and a tile.
//! One table for every mod: `defaults/programs.json` (built in), with the
//! person's `programs.json` in the home folder on top (entry by entry). The
//! shipped icons are written to `icons/` in the home folder (the app's;
//! rewritten when they change). Mods read the result as `program` on
//! panes, agents and terminals.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::BTreeMap;

const BUILT_IN: &str = include_str!("../defaults/programs.json");
include!(concat!(env!("OUT_DIR"), "/default_icons.rs"));

#[derive(Deserialize, Default)]
struct Table {
    #[serde(default)]
    shells: Vec<String>,
    #[serde(default)]
    interpreters: Vec<String>,
    #[serde(default)]
    programs: BTreeMap<String, Entry>,
    note: Option<String>,
}

#[derive(Deserialize, Serialize, Default, Clone)]
struct Entry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// A path in the home folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    icon: Option<String>,
    /// A one-color glyph, drawn in `color` (or the text color); else a picture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tint: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    /// Other process names (or agents' 7501 app names) for this entry.
    #[serde(default, rename = "match", skip_serializing_if = "Vec::is_empty")]
    also: Vec<String>,
    /// How to start it, for an agent people can start from a menu (`knownAgents()`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    run: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    open: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    flags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    app: Option<String>,
    /// `"queue"`: the CLI queues a line typed mid-turn itself, so Wire may ring it while it works.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    wire: Option<String>,
}

impl Entry {
    fn over(&mut self, mine: Entry) {
        self.name = mine.name.or(self.name.take());
        self.icon = mine.icon.or(self.icon.take());
        self.tint = mine.tint.or(self.tint);
        self.tile = mine.tile.or(self.tile.take());
        self.color = mine.color.or(self.color.take());
        if !mine.also.is_empty() {
            self.also = mine.also;
        }
        if !mine.flags.is_empty() { self.flags = mine.flags; }
        self.app = mine.app.or(self.app.take());
        self.wire = mine.wire.or(self.wire.take());
        if !mine.open.is_empty() {
            self.open = mine.open;
        }
        if !mine.run.is_empty() {
            self.run = mine.run;
        }
    }
}

/// The agents people pick most, first in menus; the rest follow by name.
const AGENTS_FIRST: &[&str] = &["claude", "codex", "pi", "opencode", "cursor", "copilot", "amp", "gemini", "grok"];

/// Every agent the table can start (entries with `run`), most picked first:
/// `{ key, name, icon, tint, tile, color, run }`.
pub fn startable() -> Vec<Value> {
    TABLE.with(|t| {
        let t = t.borrow();
        let mut keys: Vec<&String> = t.programs.iter().filter(|(_, e)| !e.run.is_empty()).map(|(k, _)| k).collect();
        keys.sort_by_key(|k| (AGENTS_FIRST.iter().position(|f| f == k).unwrap_or(AGENTS_FIRST.len()), k.to_string()));
        keys.into_iter()
            .map(|k| {
                let e = &t.programs[k];
                let mut v = shown(k, e, None, false, true);
                v["run"] = json!(e.run);
                v
            })
            .collect()
    })
}

/// Whether this program's CLI queues a line typed while it works (`"wire": "queue"` in programs.json).
pub fn queues(program: &Value) -> bool {
    TABLE.with(|t| program["key"].as_str().is_some_and(|key| t.borrow().programs.get(key).is_some_and(|e| e.wire.as_deref() == Some("queue"))))
}

/// Whether this program's flags install Canopy's turn-end hook (`canopy hook idle`), which sends
/// the idle notice itself, with the last answer; for the others the app sends a plain one.
pub fn reports_idle(program: &Value) -> bool {
    TABLE.with(|t| program["key"].as_str().is_some_and(|key| t.borrow().programs.get(key).is_some_and(|e| e.flags.iter().any(|f| f.contains("hook idle")))))
}

pub fn agent_program(program: &Value) -> bool {
    program["agent"] == true || TABLE.with(|t| program["key"].as_str()
        .and_then(|key| t.borrow().programs.get(key).map(|e| !e.run.is_empty())).unwrap_or(false))
}

pub fn append_flags(argv: &mut Vec<String>, exe: &str) {
    let program = of(None, argv);
    TABLE.with(|t| {
        let t = t.borrow();
        let entry = t.programs.values().filter(|e| !e.run.is_empty() && argv.starts_with(&e.run))
            .max_by_key(|e| e.run.len()).or_else(|| program["key"].as_str().and_then(|key| t.programs.get(key)));
        if let Some(e) = entry {
            let flags: Vec<_> = e.flags.iter().map(|arg| expand_flag(arg, exe, t.note.as_deref().unwrap_or_default())).collect();
            if !flags.first().is_some_and(|flag| argv.contains(flag)) { argv.extend(flags); }
        }
    });
}

fn shell_quote(word: &str, fish: bool) -> String {
    let word = if fish { word.replace('\\', "\\\\").replace('\'', "\\'") } else { word.replace('\'', "'\\''") };
    format!("'{word}'")
}

fn expand_flag(arg: &str, exe: &str, note: &str) -> String {
    fn text(s: &str, exe: &str, note: &str) -> String {
        // Replace in one pass so note text cannot introduce executable placeholders.
        let mut out = String::new();
        let mut rest = s;
        while let Some(i) = rest.find('{') {
            out.push_str(&rest[..i]);
            rest = &rest[i..];
            if let Some(tail) = rest.strip_prefix("{canopy}") { out.push_str(&shell_quote(exe, false)); rest = tail; }
            else if let Some(tail) = rest.strip_prefix("{note_json}") { out.push_str(&json!(note).to_string()); rest = tail; }
            else if let Some(tail) = rest.strip_prefix("{note}") { out.push_str(note); rest = tail; }
            else { out.push('{'); rest = &rest[1..]; }
        }
        out.push_str(rest);
        out
    }
    fn expand(value: &mut Value, exe: &str, note: &str) {
        match value {
            Value::String(s) => *s = text(s, exe, note),
            Value::Array(a) => a.iter_mut().for_each(|v| expand(v, exe, note)),
            Value::Object(o) => o.values_mut().for_each(|v| expand(v, exe, note)),
            _ => {}
        }
    }
    if let Ok(mut value) = serde_json::from_str::<Value>(arg) {
        expand(&mut value, exe, note);
        value.to_string()
    } else { text(arg, exe, note) }
}

fn shell_files(table: &Table, exe: &str) -> [(&'static str, String); 2] {
    let mut zsh = String::from("if [[ -n $CANOPY_USER_ZDOTDIR ]]; then\n  export ZDOTDIR=\"$CANOPY_USER_ZDOTDIR\"\nelse\n  unset ZDOTDIR\nfi\nunset CANOPY_USER_ZDOTDIR\n[[ ! -f ${ZDOTDIR:-$HOME}/.zshenv ]] || source \"${ZDOTDIR:-$HOME}/.zshenv\"\n");
    let mut fish = String::from("if set -q CANOPY_USER_XDG_DATA_DIRS\n  set -gx XDG_DATA_DIRS $CANOPY_USER_XDG_DATA_DIRS\n  set -e CANOPY_USER_XDG_DATA_DIRS\nend\n");
    for e in table.programs.values().filter(|e| !e.run.is_empty() && !e.flags.is_empty()) {
        let name = &e.run[0];
        if name.is_empty() || name.starts_with('-') || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b)) { continue; }
        // Only Canopy's flags: the menu's own launch arguments (`run[1..]`) are not what the person typed.
        let flags: Vec<_> = e.flags.iter().map(|arg| expand_flag(arg, exe, table.note.as_deref().unwrap_or_default())).collect();
        let words = |fish| flags.iter().map(|s| shell_quote(s, fish)).collect::<Vec<_>>().join(" ");
        let command = shell_quote(name, false);
        let first = shell_quote(&expand_flag(&e.flags[0], exe, table.note.as_deref().unwrap_or_default()), false);
        zsh.push_str(&format!("function {command} {{\n  local arg\n  for arg in \"$@\"; do\n    if [[ \"$arg\" == {first} ]]; then\n      command {command} \"$@\"\n      return\n    fi\n  done\n  command {command} {} \"$@\"\n}}\n", words(false)));
        let command = shell_quote(name, true);
        let first = shell_quote(&expand_flag(&e.flags[0], exe, table.note.as_deref().unwrap_or_default()), true);
        fish.push_str(&format!("function {command}\n  if contains -- {first} $argv\n    command {command} $argv\n  else\n    command {command} {} $argv\n  end\nend\n", words(true)));
    }
    [("zsh/.zshenv", zsh), ("fish/fish/vendor_conf.d/canopy.fish", fish)]
}

fn install_shell_files(table: &Table) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    for (name, text) in shell_files(table, &exe.to_string_lossy()) {
        let path = crate::mods::app_dir().join("shell").join(name);
        if std::fs::read_to_string(&path).ok().as_deref() == Some(&text) { continue; }
        std::fs::create_dir_all(path.parent().unwrap()).and_then(|_| crate::persist::write_atomic(&path, text))
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

pub fn shell_env(shell: &str, env: &mut Value) {
    let dir = crate::mods::app_dir().join("shell");
    // shortcut: only local zsh/fish get typed-agent flags, add bash/Windows when their startup integration is needed.
    if cfg!(windows) { return; }
    match base(shell) {
        "zsh" => {
            env["ZDOTDIR"] = json!(dir.join("zsh"));
            env["CANOPY_USER_ZDOTDIR"] = json!(std::env::var("ZDOTDIR").unwrap_or_default());
        }
        "fish" => {
            let inherited = std::env::var("XDG_DATA_DIRS").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| "/usr/local/share:/usr/share".into());
            env["XDG_DATA_DIRS"] = json!(format!("{}:{inherited}", dir.join("fish").display()));
            env["CANOPY_USER_XDG_DATA_DIRS"] = json!(inherited);
        }
        _ => {}
    }
}

#[derive(Clone)]
pub struct Editor {
    pub key: String,
    pub name: String,
    pub icon: String,
    pub tint: bool,
    pub argv: Vec<String>,
    app: Option<String>,
}

pub fn editors() -> Vec<Editor> {
    TABLE.with(|t| t.borrow().programs.iter().filter(|(_, e)| !e.open.is_empty() || e.app.is_some()).map(|(key, e)| Editor {
        key: key.clone(), name: e.name.clone().unwrap_or_else(|| key.clone()),
        icon: e.icon.clone().unwrap_or_else(|| "icons/program.svg".into()), tint: e.tint.unwrap_or(true), argv: e.open.clone(), app: e.app.clone(),
    }).collect())
}

fn editor_argv(open: &[String], app: Option<&str>, cli: bool, app_found: bool) -> Option<Vec<String>> {
    if cli && !open.is_empty() { Some(open.to_vec()) }
    else { app.filter(|_| app_found).map(|app| vec!["/usr/bin/open".into(), "-a".into(), app.into()]) }
}

fn installed_apps(apps: &[String], _: Option<&str>) -> Result<Vec<String>, String> {
    Ok(apps.iter().filter(|app| cfg!(target_os = "macos") && crate::spawn::command("/usr/bin/open")
        .args(["-Ra", app]).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .status().is_ok_and(|s| s.success())).cloned().collect())
}

/// The installed macOS app's own icon as a PNG in the home folder's icons/ (made once).
fn app_icon(key: &str, app: &str) -> Option<String> {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from)?;
    let bundle = [std::path::PathBuf::from("/Applications"), home.join("Applications")].into_iter().map(|d| d.join(format!("{app}.app"))).find(|b| b.exists())?;
    let name = format!("icons/editor-{key}.png");
    let out = crate::mods::app_dir().join(&name);
    if !out.exists() {
        let plist = bundle.join("Contents/Info.plist");
        let read = crate::spawn::command("/usr/bin/plutil").args(["-extract", "CFBundleIconFile", "raw", "-o", "-"]).arg(&plist).output().ok()?;
        let file = String::from_utf8(read.stdout).ok()?.trim().to_owned();
        // A plain name with no folder: the plist is the app's own, but keep it inside Resources.
        if file.is_empty() || file.contains('/') { return None; }
        let file = if file.ends_with(".icns") { file } else { format!("{file}.icns") };
        let made = crate::spawn::command("/usr/bin/sips").args(["-s", "format", "png", "-Z", "64"]).arg(bundle.join("Contents/Resources").join(file))
            .arg("--out").arg(&out).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().ok()?;
        if !made.success() { return None; }
    }
    Some(name)
}

pub fn installed_editors(mut editors: Vec<Editor>) -> Result<Vec<Editor>, String> {
    let commands = editors.iter().filter_map(|e| e.argv.first().cloned()).collect();
    let found = crate::mods::installed_cached(None, commands, installed)?;
    let have_apps = crate::mods::installed_cached(None, editors.iter().filter_map(|e| e.app.clone()).collect(), installed_apps)?;
    editors.retain_mut(|e| {
        let app = e.app.as_deref();
        let app_found = app.is_some_and(|a| have_apps.iter().any(|s| s == a));
        if let Some(argv) = editor_argv(&e.argv, app, e.argv.first().is_some_and(|c| found.contains(c)), app_found) {
            e.argv = argv;
            if let Some(icon) = app.filter(|_| app_found).and_then(|a| app_icon(&e.key, a)) {
                (e.icon, e.tint) = (icon, false);
            }
            true
        } else { false }
    });
    Ok(editors)
}

/// Which of `commands` this machine can start, asked of the shell agents start
/// in (so an app opened from the Dock finds what the person's terminal finds).
/// On another machine (`on`): its login/interactive shell, as remote agents start.
pub fn installed(commands: &[String], on: Option<&str>) -> Result<Vec<String>, String> {
    if commands.is_empty() {
        return Ok(Vec::new());
    }
    // Each found one as a marked line: an interactive shell may print its own
    // things (a prompt's escapes, a greeting) around the answer.
    let result = match on {
        Some(m) => {
            let argv = crate::remote_shell::check(commands);
            let result = match crate::mods::run_remote(m, &argv[0], &argv[1..], None) {
                Err(e) if e.contains("cannot start sh: program not found") => {
                    crate::mods::run_remote(m, "powershell.exe", &installed_powershell(commands), None)
                }
                r => r,
            };
            result.map(|(code, out, err)| (code, String::from_utf8_lossy(&out).into_owned(), String::from_utf8_lossy(&err).into_owned()))
        }
        None if cfg!(windows) => {
            // The core refreshes user/system PATH; the app may have inherited only the system PATH.
            let mut args = vec!["exec".into(), "--".into(), crate::window::shell()];
            args.extend(installed_powershell(commands));
            crate::mods::run_blocking(&crate::core_bin::path().to_string_lossy(), &args, None, std::time::Duration::from_secs(30))
        }
        None => {
            let shell = crate::window::shell();
            let line = installed_line(commands, base(&shell) == "fish");
            let mode = if cfg!(target_os = "macos") { "-lic" } else { "-ic" };
            crate::mods::run_blocking(&shell, &[mode.into(), line], None, std::time::Duration::from_secs(30))
        }
    };
    installed_result(commands, result)
}

fn installed_result(commands: &[String], result: Result<(i32, String, String), String>) -> Result<Vec<String>, String> {
    let (code, out, err) = result?;
    if code != 0 || !out.lines().any(|line| line == "herdr-check-complete") {
        return Err(format!("could not check installed agents (exit {code}): {}", err.trim()));
    }
    let found: std::collections::HashSet<&str> = out.split("herdr-has:").skip(1).filter_map(|s| s.split_whitespace().next()).collect();
    Ok(commands.iter().filter(|c| found.contains(c.as_str())).cloned().collect())
}

fn installed_powershell(commands: &[String]) -> Vec<String> {
    let list = commands.iter().map(|c| powershell_quote(c)).collect::<Vec<_>>().join(",");
    let script = format!(
        "[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); foreach($c in @({list})){{ if(Get-Command $c -ErrorAction SilentlyContinue){{ \"herdr-has:$c\" }} }}; \"herdr-check-complete\""
    );
    vec!["-NoLogo".into(), "-Command".into(), script]
}

/// Check in the shell itself for functions, but not aliases (`"$@"` cannot run them).
fn installed_line(commands: &[String], fish: bool) -> String {
    let checks = commands
        .iter()
        .map(|c| {
            let word = format!("'{}'", c.replace('\'', "'\\''"));
            let unalias = if fish { String::new() } else { format!("unalias -- {word} 2>/dev/null; ") };
            format!("{unalias}command -v {word} >/dev/null 2>&1 && printf 'herdr-has:%s\\n' {word};")
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!("{checks} printf '\\nherdr-check-complete\\n'")
}

thread_local! {
    static TABLE: RefCell<Table> = RefCell::new(serde_json::from_str(BUILT_IN).expect("defaults/programs.json"));
}

pub fn path() -> std::path::PathBuf {
    crate::mods::app_dir().join("programs.json")
}

/// Writes the shipped icons into the home folder.
pub fn install_icons() {
    let dir = crate::mods::app_dir().join("icons");
    let _ = std::fs::create_dir_all(&dir);
    for (name, bytes) in DEFAULT_ICONS {
        let file = dir.join(name);
        if std::fs::read(&file).ok().as_deref() != Some(*bytes) {
            let _ = crate::persist::write_atomic(&file, bytes);
        }
    }
}

/// Reads the person's table on top of the built-in one. Err: what's wrong with their file.
pub fn load() -> Result<(), String> {
    let mut table: Table = serde_json::from_str(BUILT_IN).expect("defaults/programs.json");
    let result = match std::fs::read_to_string(path()) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("programs.json: {e}")),
        Ok(text) => match serde_json::from_str::<Table>(&text) {
            Err(e) => Err(crate::settings::json_error("programs.json", e)),
            Ok(mine) => {
                table.note = mine.note.or(table.note);
                table.shells.extend(mine.shells);
                table.interpreters.extend(mine.interpreters);
                for (key, entry) in mine.programs {
                    table.programs.entry(key).or_default().over(entry);
                }
                Ok(())
            }
        },
    };
    result?;
    let installed = install_shell_files(&table);
    TABLE.with(|t| *t.borrow_mut() = table);
    installed
}

/// How a terminal shows: from the agent's own report (`app`), else the
/// program it runs (`argv`). `{ key, name, icon, tint, tile, color, shell, agent }`.
pub fn of(app: Option<&str>, argv: &[String]) -> Value {
    TABLE.with(|t| resolve(&t.borrow(), app, argv))
}

/// How the app calls an agent ("Kiro finished"), from its 7501 `app`: the
/// name `of` shows for it (the person's `programs.json` included), else the `app`.
pub fn name(app: &str) -> String {
    TABLE.with(|t| find(&t.borrow(), app).map(|(key, e)| e.name.clone().unwrap_or_else(|| key.to_owned()))).unwrap_or_else(|| app.to_owned())
}

fn base(s: &str) -> &str {
    let s = s.rsplit(['/', '\\']).next().unwrap_or(s);
    let s = s.trim_start_matches('-');
    // Windows: `pwsh.exe` is `pwsh`.
    match s.len().checked_sub(4) {
        Some(i) if s.is_char_boundary(i) && s[i..].eq_ignore_ascii_case(".exe") => &s[..i],
        _ => s,
    }
}

/// PowerShell's single quotes, which include the curly ones.
const PS_QUOTES: [char; 5] = ['\'', '\u{2018}', '\u{2019}', '\u{201A}', '\u{201B}'];

/// `& 'prog' 'arg 1'`: a command for PowerShell's `-Command`, each word quoted.
pub fn powershell_line(cmd: &[String]) -> String {
    format!("& {}", cmd.iter().map(|w| powershell_quote(w)).collect::<Vec<_>>().join(" "))
}

fn powershell_quote(word: &str) -> String {
    let mut s = String::from("'");
    for ch in word.chars() {
        if PS_QUOTES.contains(&ch) {
            s.push(ch);
        }
        s.push(ch);
    }
    s + "'"
}

/// The words back from `powershell_line`.
fn powershell_words(line: &str) -> Option<Vec<String>> {
    let mut chars = line.strip_prefix("& ")?.chars().peekable();
    let mut words = Vec::new();
    while let Some(c) = chars.next() {
        if c == ' ' {
            continue;
        }
        if !PS_QUOTES.contains(&c) {
            return None;
        }
        let mut w = String::new();
        while let Some(ch) = chars.next() {
            if PS_QUOTES.contains(&ch) {
                if chars.peek().is_some_and(|n| PS_QUOTES.contains(n)) {
                    w.push(chars.next().unwrap_or(ch));
                    continue;
                }
                break;
            }
            w.push(ch);
        }
        words.push(w);
    }
    Some(words)
}

fn find<'a>(t: &'a Table, name: &str) -> Option<(&'a str, &'a Entry)> {
    let lower = name.to_lowercase();
    t.programs
        .iter()
        .find(|(k, _)| k.as_str() == name || k.to_lowercase() == lower)
        .or_else(|| t.programs.iter().find(|(_, e)| e.also.iter().any(|a| a == name || a.to_lowercase() == lower)))
        .map(|(k, e)| (k.as_str(), e))
}

fn resolve(t: &Table, app: Option<&str>, argv: &[String]) -> Value {
    if let Some(app) = app.filter(|a| !a.is_empty()) {
        return match find(t, app) {
            Some((key, e)) => shown(key, e, None, false, true),
            None => shown("agent", t.programs.get("agent").unwrap_or(&Entry::default()), Some(app), false, true),
        };
    }
    // The app starts commands in the person's shell: `zsh -ic '"$@"; exec "$0" -i' zsh <cmd>...`.
    if argv.len() > 4
        && t.shells.iter().any(|s| s == base(&argv[0]))
        && argv[1].starts_with('-')
        && argv[1].ends_with('c')
        && (argv[2].contains("\"$@\"") || argv[2] == "command $argv[2..]; exec $argv[1] -i")
    {
        return resolve(t, None, &argv[4..]);
    }
    // On Windows: `pwsh -NoExit -Command "& 'codex' ..."`.
    if argv.len() == 4
        && t.shells.iter().any(|s| s == base(&argv[0]))
        && argv[1] == "-NoExit"
        && argv[2] == "-Command"
        && let Some(words) = powershell_words(&argv[3])
    {
        return resolve(t, None, &words);
    }
    let Some(first) = argv.first().map(|a| base(a)) else {
        return shown("shell", t.programs.get("shell").unwrap_or(&Entry::default()), None, true, false);
    };
    // `node .../vite.js dev`, `npx vite`, `uv run pytest`: the program they run, when it's known.
    if t.interpreters.iter().any(|i| i == first) {
        // `env PATH=... claude`: an assignment is not the program.
        for arg in argv[1..].iter().filter(|a| !a.starts_with('-') && !a.contains('=')) {
            // A script's name without its extension (`vite.js`), never a module path (`http.server`).
            let name = base(arg);
            let stem = [".js", ".mjs", ".cjs", ".ts", ".py", ".rb"].iter().find_map(|x| name.strip_suffix(x)).unwrap_or(name);
            if let Some((key, e)) = find(t, stem).filter(|(k, _)| !t.interpreters.iter().any(|i| i == k)) {
                return shown(key, e, None, false, false);
            }
        }
    }
    if let Some((key, e)) = find(t, first) {
        return shown(key, e, None, false, false);
    }
    if t.shells.iter().any(|s| s == first) {
        return shown("shell", t.programs.get("shell").unwrap_or(&Entry::default()), Some(first), true, false);
    }
    let mut e = t.programs.get("program").cloned().unwrap_or_default();
    e.tile = Some(hash_color(first).into());
    e.color = Some("#ffffff".into());
    shown("program", &e, Some(first), false, false)
}

fn shown(key: &str, e: &Entry, name: Option<&str>, shell: bool, agent: bool) -> Value {
    json!({
        "key": key,
        "name": name.map(str::to_owned).or_else(|| e.name.clone()).unwrap_or_else(|| key.to_owned()),
        "icon": e.icon,
        "tint": e.tint.unwrap_or(true),
        "tile": e.tile,
        "color": e.color,
        "shell": shell,
        "agent": agent,
    })
}

/// A color for a program nobody listed, the same every time.
fn hash_color(s: &str) -> &'static str {
    const PALETTE: [&str; 8] = ["#6d5bd0", "#2f80ed", "#0f9d8a", "#d9822b", "#c2417d", "#3a7d44", "#8a5a44", "#4e5d94"];
    let h = s.bytes().fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619));
    PALETTE[(h % PALETTE.len() as u32) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t() -> Table {
        serde_json::from_str(BUILT_IN).unwrap()
    }
    fn key(app: Option<&str>, argv: &[&str]) -> String {
        let argv: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
        resolve(&t(), app, &argv)["key"].as_str().unwrap().to_owned()
    }

    #[test]
    fn wire_is_data_a_queueing_cli_the_note_and_the_hooks() {
        assert!(queues(&of(Some("claude-code"), &[])));
        assert!(!queues(&of(Some("codex"), &[])) && !queues(&of(None, &["zsh".into()])));
        let table = t();
        let note = table.note.as_deref().unwrap();
        assert!(note.contains("canopy spawn <program>") && note.contains("never your user's approval") && !note.contains("tell|ask"));
        let flags = table.programs["claude"].flags.join(" ");
        assert!(flags.contains("\"UserPromptSubmit\"") && flags.contains("hook wire") && flags.contains("Bash(canopy wire:*)"));
        assert!(reports_idle(&of(Some("claude-code"), &[])) && !reports_idle(&of(Some("codex"), &[])), "Claude's Stop hook reports idle");
        assert!(!table.programs["codex"].flags.join(" ").contains("hook"), "Codex ignores hook context: note and pull only");
    }

    #[test]
    fn flag_placeholders_preserve_quotes_newlines_and_literal_placeholders() {
        let note = "Say \"hi\"\nwith 'quotes' \\ and {canopy} {note}";
        let encoded = json!(note).to_string();
        assert_eq!(expand_flag("developer_instructions={note_json}", "/a'b", note), format!("developer_instructions={encoded}"));
        assert_eq!(expand_flag("{note}", "/a'b", note), note);
        let arg = json!({"plain":"{note}", "encoded":["{note_json}"], "command":"{canopy} hook"}).to_string();
        let expanded: Value = serde_json::from_str(&expand_flag(&arg, "/a'b", note)).unwrap();
        assert_eq!(expanded["plain"], note);
        assert_eq!(expanded["encoded"][0], encoded);
        assert_eq!(expanded["command"], "'/a'\\''b' hook");
        let mut argv = vec!["claude".into(), "--settings".into(), "mine.json".into()];
        let original = argv.clone();
        append_flags(&mut argv, "/canopy");
        assert_eq!(argv, original);
    }

    #[test]
    fn shell_integrations_restore_environment_quote_words_and_skip_existing_flags() {
        let table: Table = serde_json::from_value(json!({"note":"a'b\\c\n\"$HOME\"", "programs":{
            "test":{"run":["test-agent", "--local"],"flags":["--settings","{note}","{canopy}"]},
            "bare":{"run":["bare"]}
        }})).unwrap();
        let files = shell_files(&table, "/a'b/canopy");
        let zsh = &files[0].1;
        let fish = &files[1].1;
        assert_eq!(files[0].0, "zsh/.zshenv");
        assert!(zsh.starts_with("if [[ -n $CANOPY_USER_ZDOTDIR ]]; then\n  export ZDOTDIR=\"$CANOPY_USER_ZDOTDIR\"\nelse\n  unset ZDOTDIR\nfi\nunset CANOPY_USER_ZDOTDIR\n"));
        assert!(zsh.contains("source \"${ZDOTDIR:-$HOME}/.zshenv\"\nfunction 'test-agent' {"));
        assert!(zsh.contains("for arg in \"$@\"; do\n    if [[ \"$arg\" == '--settings' ]]; then\n      command 'test-agent' \"$@\"\n      return"));
        assert!(zsh.contains("command 'test-agent' '--settings' 'a'\\''b\\c\n\"$HOME\"'"));
        assert_eq!(files[1].0, "fish/fish/vendor_conf.d/canopy.fish");
        assert!(fish.starts_with("if set -q CANOPY_USER_XDG_DATA_DIRS\n  set -gx XDG_DATA_DIRS $CANOPY_USER_XDG_DATA_DIRS\n  set -e CANOPY_USER_XDG_DATA_DIRS\nend\n"));
        assert!(fish.contains("function 'test-agent'\n  if contains -- '--settings' $argv\n    command 'test-agent' $argv\n  else\n"));
        assert!(fish.contains("command 'test-agent' '--settings' 'a\\'b\\\\c\n\"$HOME\"'"));
        assert!(!zsh.contains("function 'bare'"));
        assert!(!fish.contains("function 'bare'"));
        for (shell, text) in [("zsh", zsh), ("fish", fish)] {
            use std::io::Write;
            let child = std::process::Command::new(shell).arg("-n").stdin(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn();
            let mut child = match child {
                Ok(child) => child,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => panic!("{shell}: {e}"),
            };
            child.stdin.take().unwrap().write_all(text.as_bytes()).unwrap();
            let out = child.wait_with_output().unwrap();
            assert!(out.status.success(), "{shell}: {}", String::from_utf8_lossy(&out.stderr));
        }
    }

    #[cfg(unix)]
    #[test]
    fn generated_zsh_keeps_user_startup_and_arguments() {
        use std::os::unix::fs::PermissionsExt;
        if std::process::Command::new("zsh").arg("--version").output().is_err() { return; }
        let dir = std::env::temp_dir().join(format!("canopy-zsh-{}", std::process::id()));
        let user = dir.join("user");
        let integration = dir.join("integration");
        let bin = dir.join("bin");
        for path in [&user, &integration, &bin] { std::fs::create_dir_all(path).unwrap(); }
        let note = "a'b\\c\n\"$HOME\" {note}";
        let table: Table = serde_json::from_value(json!({"note":note,"programs":{
            "test":{"run":["canopy-test-agent"],"flags":["--settings","{note}"]}
        }})).unwrap();
        std::fs::write(integration.join(".zshenv"), &shell_files(&table, "/canopy")[0].1).unwrap();
        for (file, variable) in [(".zshenv", "ENV"), (".zprofile", "PROFILE"), (".zshrc", "RC"), (".zlogin", "LOGIN")] {
            std::fs::write(user.join(file), format!("export CANOPY_TEST_{variable}=loaded\n")).unwrap();
        }
        let agent = bin.join("canopy-test-agent");
        std::fs::write(&agent, "#!/bin/sh\nprintf '%s\\0' \"$@\"\n").unwrap();
        std::fs::set_permissions(&agent, std::fs::Permissions::from_mode(0o700)).unwrap();
        let out = std::process::Command::new("zsh").args(["-lic", "printf '%s\\0' \"$ZDOTDIR\" \"$CANOPY_TEST_ENV$CANOPY_TEST_PROFILE$CANOPY_TEST_RC$CANOPY_TEST_LOGIN\"; canopy-test-agent 'caller arg'; canopy-test-agent --settings mine"])
            .env_clear().env("HOME", &user).env("ZDOTDIR", &integration).env("CANOPY_USER_ZDOTDIR", &user)
            .env("PATH", format!("{}:/usr/bin:/bin", bin.display())).output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let words: Vec<_> = out.stdout.split(|b| *b == 0).map(|b| String::from_utf8_lossy(b).into_owned()).collect();
        assert_eq!(words, [user.display().to_string(), "loadedloadedloadedloaded".into(), "--settings".into(), note.into(), "caller arg".into(), "--settings".into(), "mine".into(), "".into()]);
        let out = std::process::Command::new("zsh").args(["-c", "[[ ! -v ZDOTDIR && ! -v CANOPY_USER_ZDOTDIR && $CANOPY_TEST_ENV == loaded ]]"])
            .env_clear().env("HOME", &user).env("ZDOTDIR", &integration).env("CANOPY_USER_ZDOTDIR", "").output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn flag_argv_expands_inside_json_and_merges_like_run() {
        let mut entry: Entry = serde_json::from_value(json!({"run":["agent"],"flags":["--one"]})).unwrap();
        entry.over(serde_json::from_value(json!({"name":"Renamed"})).unwrap());
        assert_eq!(entry.flags, ["--one"]);
        entry.over(serde_json::from_value(json!({"run":["new"],"flags":["--two"]})).unwrap());
        assert_eq!(entry.run, ["new"]);
        assert_eq!(entry.flags, ["--two"]);
        TABLE.with(|t| t.borrow_mut().programs.insert("test-hook".into(), entry));
        let mut custom = vec!["new".into()];
        append_flags(&mut custom, "/tmp/canopy");
        assert_eq!(custom, ["new", "--two"]);
        TABLE.with(|t| t.borrow_mut().programs.remove("test-hook"));
        let mut argv = vec!["claude".into()];
        append_flags(&mut argv, "/tmp/a'b \"app\"");
        assert_eq!(argv[1], "--settings");
        let settings: Value = serde_json::from_str(&argv[2]).unwrap();
        assert_eq!(settings["hooks"]["SubagentStart"][0]["hooks"][0]["command"], "'/tmp/a'\\''b \"app\"' hook sub-start");
        assert_eq!(settings["hooks"]["SubagentStop"][0]["hooks"][0]["type"], "command");
        let mut argv = vec!["codex".into()];
        append_flags(&mut argv, "/tmp/canopy");
        assert_eq!(argv[1], "-c");
        assert!(argv[2].starts_with("developer_instructions=\"You are running inside Canopy"));
    }

    #[test]
    fn editor_fields_merge_and_launch_prefers_cli() {
        let mut e: Entry = serde_json::from_value(json!({"open":["code"],"app":"Code","name":"Code"})).unwrap();
        e.over(serde_json::from_value(json!({"name":"Mine"})).unwrap());
        assert_eq!(e.open, ["code"]);
        assert_eq!(e.app.as_deref(), Some("Code"));
        e.over(serde_json::from_value(json!({"open":["custom","--reuse"],"app":"Custom"})).unwrap());
        assert_eq!(editor_argv(&e.open, e.app.as_deref(), true, true), Some(vec!["custom".into(), "--reuse".into()]));
        assert_eq!(editor_argv(&e.open, e.app.as_deref(), false, true), Some(vec!["/usr/bin/open".into(), "-a".into(), "Custom".into()]));
        assert_eq!(editor_argv(&e.open, e.app.as_deref(), false, false), None);
        assert_eq!(editor_argv(&[], None, false, true), None);
    }

    #[test]
    fn powershell_words_survive_quotes() {
        let words: Vec<String> = ["pi", "it's \u{2019}fine\u{2019}", "", "a  b"].map(String::from).to_vec();
        let line = powershell_line(&words);
        assert_eq!(line, "& 'pi' 'it''s \u{2019}\u{2019}fine\u{2019}\u{2019}' '' 'a  b'");
        assert_eq!(powershell_words(&line), Some(words));
    }

    #[test]
    fn powershell_checks_keep_command_names_literal() {
        let argv = installed_powershell(&["claude".into(), "missing'command; $x".into(), "O’Brien".into()]);
        assert_eq!(argv[..2], ["-NoLogo", "-Command"]);
        assert!(argv[2].contains("@('claude','missing''command; $x','O’’Brien')"));
        assert!(argv[2].contains("[Console]::OutputEncoding"));
    }

    #[cfg(windows)]
    #[test]
    fn powershell_finds_installed_commands() {
        let commands = ["cmd.exe".into(), "herdr_missing_agent_440".into()];
        let result = crate::mods::run_blocking("powershell.exe", &installed_powershell(&commands), None, std::time::Duration::from_secs(30));
        assert_eq!(installed_result(&commands, result).unwrap(), ["cmd.exe"]);
    }

    #[test]
    fn agents_shells_tools_and_what_interpreters_run() {
        assert_eq!(key(Some("claude-code"), &["/bin/zsh"]), "claude");
        assert_eq!(key(Some("grok-build"), &["/bin/zsh"]), "grok");
        assert_eq!(key(Some("dsh-tui"), &["/bin/zsh"]), "dsh-tui");
        assert_eq!(key(None, &["dsh-tui"]), "dsh-tui");
        assert_eq!(key(Some("newagent"), &[]), "agent");
        assert_eq!(key(None, &["/bin/zsh"]), "shell");
        assert_eq!(key(None, &["-zsh"]), "shell");
        assert_eq!(key(None, &["nvim", "x.rs"]), "nvim");
        assert_eq!(key(None, &["vi"]), "vim");
        assert_eq!(key(None, &["node", "/x/node_modules/.bin/vite", "dev"]), "vite");
        assert_eq!(key(None, &["npx", "vitest"]), "vitest");
        assert_eq!(key(None, &["node", "server.js"]), "node");
        assert_eq!(key(None, &["python3", "-m", "http.server", "8799"]), "python");
        assert_eq!(key(None, &["uv", "run", "pytest"]), "pytest");
        assert_eq!(key(None, &["env", "PATH=/home/me/Library/pnpm:/usr/bin", "claude"]), "claude");
        assert_eq!(key(None, &["mytool"]), "program");
        assert_eq!(key(None, &["/bin/zsh", "-ic", "\"$@\"; exec \"$0\" -i", "/bin/zsh", "vi", "x"]), "vim");
        assert_eq!(key(None, &["/usr/bin/fish", "-ic", "command $argv[2..]; exec $argv[1] -i", "/usr/bin/fish", "vi", "x"]), "vim");
        let line = powershell_line(&["codex".into(), "fix it".into()]);
        assert_eq!(key(None, &["pwsh.exe", "-NoExit", "-Command", &line]), "codex");
        assert_eq!(key(None, &[r"C:\Windows\powershell.exe"]), "shell");
    }

    #[cfg(unix)]
    #[test]
    fn installed_checks_functions_and_quoted_names_but_not_aliases() {
        let commands = ["issue57_function", "issue57_alias", "printf", "missing'command; false"].map(String::from);
        let line = installed_line(&commands, false);
        let out = std::process::Command::new("sh")
            .args(["-c", &format!("issue57_function() {{ :; }}; alias issue57_alias='printf alias'; {line}")])
            .output()
            .unwrap();
        assert!(out.status.success());
        assert_eq!(installed_result(&commands, Ok((0, String::from_utf8(out.stdout).unwrap(), String::new()))).unwrap(), ["issue57_function", "printf"]);
    }

    #[test]
    fn installed_check_failures_are_not_missing_agents() {
        let commands = ["claude".into()];
        assert_eq!(installed_result(&commands, Ok((0, "\nherdr-check-complete\n".into(), String::new()))).unwrap(), Vec::<String>::new());
        assert!(installed_result(&commands, Err("machine offline".into())).unwrap_err().contains("machine offline"));
        for (code, output) in [(1, "herdr-check-complete\n"), (0, ""), (0, "herdr-has:claude\n")] {
            assert!(installed_result(&commands, Ok((code, output.into(), "shell failed".into()))).is_err());
        }
    }

    #[test]
    fn every_entry_has_its_icon() {
        for (k, e) in t().programs {
            let icon = e.icon.unwrap_or_else(|| panic!("{k} has no icon"));
            let file = icon.strip_prefix("icons/").unwrap();
            assert!(DEFAULT_ICONS.iter().any(|(n, _)| *n == file), "{k}: {icon} not shipped");
        }
    }
}
