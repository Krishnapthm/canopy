//! Event-driven socket client. Terminal input blocks on a dedicated reader thread.
mod input;
mod session;
mod tree;

use crate::client::Client;
use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event as TerminalEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use input::CommandId;
use protocol::{Decision, Event, ExtKind, Extension, HarnessDetection, Item, ItemKind, ItemState, Node, Op, Policy, SpawnReq};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    widgets::{Block, Paragraph},
};
use std::{collections::HashSet, time::Duration};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    #[default]
    Tree,
    New,
    Compose,
    Filter,
    Palette,
    Help,
    Plugins,
}

#[derive(Default)]
struct App {
    nodes: Vec<Node>,
    items: Vec<Item>,
    selected: Option<String>,
    collapsed: HashSet<String>,
    text: String,
    filter: String,
    mode: Mode,
    message: String,
    harnesses: String,
    extensions: Vec<Extension>,
    extension_index: usize,
    extension_errors: String,
    parent: Option<String>,
    prefix: bool,
    scroll: u16,
}

impl App {
    fn event(&mut self, event: Event) {
        match event {
            Event::Node { node } => {
                if let Some(n) = self.nodes.iter_mut().find(|n| n.id == node.id) {
                    *n = *node;
                } else {
                    self.nodes.push(*node);
                }
            }
            Event::Item { item } if Some(&item.session) == self.selected.as_ref() => {
                if let Some(i) = self.items.iter_mut().find(|i| i.id == item.id) {
                    *i = item;
                } else {
                    self.items.push(item);
                }
            }
            Event::Delta { session, item, text } if Some(&session) == self.selected.as_ref() => {
                if let Some(i) = self.items.iter_mut().find(|i| i.id == item) {
                    let mut body = i.body["text"].as_str().unwrap_or_default().to_owned();
                    body.push_str(&text);
                    i.body["text"] = body.into();
                }
            }
            _ => {}
        }
    }

    async fn select(&mut self, client: &mut Client, id: String) -> Result<()> {
        if self.selected.as_ref() == Some(&id) {
            return Ok(());
        }
        if let Some(old) = &self.selected {
            client.call(Op::UnsubSession { session: old.clone() }).await?;
        }
        let response = client.call(Op::SubSession { session: id.clone(), after_seq: None, tail: Some(300) }).await?;
        self.items = serde_json::from_value(response["items"].clone())?;
        self.selected = Some(id);
        self.scroll = 0;
        Ok(())
    }

    async fn key(&mut self, key: KeyEvent, client: &mut Client) -> Result<bool> {
        if key.kind == KeyEventKind::Release {
            return Ok(false);
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Ok(true);
        }
        if key.code == KeyCode::Char('k') && key.modifiers.intersects(KeyModifiers::SUPER | KeyModifiers::CONTROL) {
            return self.command(CommandId::Palette, client).await;
        }
        if self.prefix {
            self.prefix = false;
            if let Some(id) = input::command(key.code, true) {
                return self.command(id, client).await;
            }
            return Ok(false);
        }
        if self.mode == Mode::Plugins {
            match key.code {
                KeyCode::Char('g') => self.prefix = true,
                KeyCode::Char('q') => return Ok(true),
                KeyCode::Esc => self.mode = Mode::Tree,
                KeyCode::Down => self.extension_index = (self.extension_index + 1).min(self.extensions.len().saturating_sub(1)),
                KeyCode::Up => self.extension_index = self.extension_index.saturating_sub(1),
                KeyCode::Char('r') => self.refresh_extensions(client).await?,
                KeyCode::Char(' ') | KeyCode::Char('a') => {
                    if let Some(row) = self.extensions.get(self.extension_index).cloned() {
                        let adopt = key.code == KeyCode::Char('a') && row.kind == ExtKind::Skill;
                        client
                            .call(Op::ExtSet {
                                harness: row.harness,
                                kind: if adopt { ExtKind::AdoptSkill } else { row.kind },
                                id: row.id,
                                scope: row.scope,
                                enabled: adopt || !row.enabled,
                            })
                            .await?;
                        self.refresh_extensions(client).await?;
                    }
                }
                _ => {}
            }
            return Ok(false);
        }
        if matches!(self.mode, Mode::Compose | Mode::New | Mode::Filter | Mode::Palette) {
            match key.code {
                KeyCode::Esc => {
                    self.mode = Mode::Tree;
                    self.text.clear();
                }
                KeyCode::Backspace => {
                    self.text.pop();
                }
                KeyCode::Char(c) => self.text.push(c),
                KeyCode::Enter => {
                    let text = self.text.clone();
                    match self.mode {
                        Mode::New => {
                            let (harness, prompt) = text.split_once(' ').context("enter: harness task")?;
                            anyhow::ensure!(!prompt.trim().is_empty(), "enter a task");
                            let r = client
                                .call(Op::Spawn(SpawnReq {
                                    harness: harness.into(),
                                    prompt: prompt.into(),
                                    cwd: std::env::current_dir()?.to_string_lossy().into_owned(),
                                    model: None,
                                    policy: Policy::Ask,
                                    parent: self.parent.clone(),
                                    title: None,
                                }))
                                .await?;
                            let id = r["session"].as_str().context("missing session id")?;
                            self.select(client, id.into()).await?;
                        }
                        Mode::Compose => {
                            anyhow::ensure!(!text.trim().is_empty(), "enter a message");
                            if let Some(session) = &self.selected {
                                let r = client.call(Op::Send { session: session.clone(), text, interrupt: false }).await?;
                                self.message = r["delivery"].as_str().unwrap_or_default().into();
                            }
                        }
                        Mode::Filter => self.filter = text,
                        Mode::Palette => {
                            let id = input::COMMANDS.iter().find(|c| c.title.to_lowercase().contains(&text.to_lowercase())).map(|c| c.id);
                            self.mode = Mode::Tree;
                            self.text.clear();
                            if let Some(id) = id {
                                return self.command(id, client).await;
                            }
                        }
                        _ => {}
                    }
                    self.mode = Mode::Tree;
                    self.text.clear();
                }
                _ => {}
            }
            return Ok(false);
        }
        match key.code {
            KeyCode::Char('g') => self.prefix = true,
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(10),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(10),
            code => {
                if let Some(id) = input::command(code, false) {
                    return self.command(id, client).await;
                }
            }
        }
        Ok(false)
    }

    async fn refresh_extensions(&mut self, client: &mut Client) -> Result<()> {
        let cwd = std::env::current_dir()?.to_string_lossy().into_owned();
        let value = client.call(Op::ExtList { cwd: Some(cwd) }).await?;
        self.extensions = serde_json::from_value(value["items"].clone())?;
        self.extension_errors = value["errors"]
            .as_array()
            .map(|rows| rows.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join("\n"))
            .unwrap_or_default();
        self.extension_index = self.extension_index.min(self.extensions.len().saturating_sub(1));
        Ok(())
    }

    async fn command(&mut self, id: CommandId, client: &mut Client) -> Result<bool> {
        match id {
            CommandId::Quit => return Ok(true),
            CommandId::New | CommandId::Child => {
                self.parent = if id == CommandId::Child { self.selected.clone() } else { None };
                self.mode = Mode::New;
                self.text.clear();
            }
            CommandId::Compose | CommandId::Filter | CommandId::Palette | CommandId::Help | CommandId::Plugins => {
                self.mode = match id {
                    CommandId::Compose => Mode::Compose,
                    CommandId::Filter => Mode::Filter,
                    CommandId::Palette => Mode::Palette,
                    CommandId::Help => Mode::Help,
                    CommandId::Plugins => Mode::Plugins,
                    _ => unreachable!(),
                };
                self.text.clear();
                if self.mode == Mode::Plugins {
                    self.refresh_extensions(client).await?;
                }
            }
            CommandId::Back | CommandId::Tree => self.mode = Mode::Tree,
            CommandId::Collapse => {
                if let Some(id) = &self.selected {
                    self.collapsed.insert(id.clone());
                }
            }
            CommandId::Expand => {
                if let Some(id) = &self.selected {
                    self.collapsed.remove(id);
                }
            }
            CommandId::Down | CommandId::Up | CommandId::Open => {
                let rows = tree::rows(self);
                let pos = rows.iter().position(|(n, _)| Some(&n.id) == self.selected.as_ref());
                let index = match (id, pos) {
                    (CommandId::Down, Some(p)) => (p + 1).min(rows.len().saturating_sub(1)),
                    (CommandId::Up, Some(p)) => p.saturating_sub(1),
                    (_, Some(p)) => p,
                    _ => 0,
                };
                if let Some((node, _)) = rows.get(index) {
                    self.select(client, node.id.clone()).await?;
                }
            }
            CommandId::Resume => {
                if let Some(session) = &self.selected {
                    client.call(Op::Resume { session: session.clone() }).await?;
                }
            }
            CommandId::Interrupt => {
                if let Some(session) = &self.selected {
                    client.call(Op::Interrupt { session: session.clone() }).await?;
                }
            }
            CommandId::Allow | CommandId::Deny => {
                if let Some(i) = self.items.iter().find(|i| i.kind == ItemKind::Permission && i.state == ItemState::Streaming) {
                    client
                        .call(Op::Permission {
                            session: i.session.clone(),
                            request: i.id.clone(),
                            decision: if id == CommandId::Allow { Decision::Allow } else { Decision::Deny },
                        })
                        .await?;
                }
            }
        }
        Ok(false)
    }
}

/// One line per known CLI, in the doctor's words. Login commands are shown, never run.
fn harness_list(rows: &[HarnessDetection]) -> String {
    rows.iter()
        .map(|h| {
            let state = match (&h.version, h.supported, h.logged_in) {
                (None, _, _) => "not installed".to_owned(),
                (Some(_), false, _) => "detected, not supported yet".to_owned(),
                (Some(_), true, Some(true)) => "ready".to_owned(),
                (Some(_), true, _) => format!("logged out - run {}", h.login_command),
            };
            let glyph = if h.supported && h.logged_in == Some(true) { "✓" } else { "○" };
            format!("{glyph} {:<12} {state}", h.name)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn draw(frame: &mut Frame, app: &App) {
    let vertical = Layout::vertical([Constraint::Min(0), Constraint::Length(3), Constraint::Length(3)]).split(frame.area());
    let columns = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).split(vertical[0]);
    let globe = if std::env::var_os("CANOPY_NERD_FONT").is_some() { "\u{f01e7}" } else { "⊕" };
    frame.render_widget(tree::widget(app).block(Block::bordered().title(format!("{globe} Agents"))), columns[0]);
    let special = match app.mode {
        Mode::Plugins => {
            let mut groups: Vec<Vec<(usize, &Extension)>> = Vec::new();
            for (i, row) in app.extensions.iter().enumerate() {
                let group = groups.iter_mut().find(|g| {
                    row.kind == ExtKind::LibrarySkill && g[0].1.kind == row.kind && g[0].1.id == row.id && g[0].1.scope == row.scope
                });
                if let Some(group) = group {
                    group.push((i, row));
                } else {
                    groups.push(vec![(i, row)]);
                }
            }
            let rows = groups
                .iter()
                .map(|group| {
                    let r = group[0].1;
                    let scope = match &r.scope {
                        protocol::ExtScope::Global => globe.into(),
                        protocol::ExtScope::Project(cwd) => format!("▱ {cwd}"),
                    };
                    let switches = group
                        .iter()
                        .map(|(i, r)| {
                            format!(
                                "{}{} [{}]{}{}",
                                if *i == app.extension_index { "›" } else { " " },
                                r.harness,
                                if r.enabled { "on" } else { "off" },
                                if r.writable { "" } else { " read-only; a: adopt" },
                                r.command.as_ref().map(|c| format!("\nDeclared command (execution unavailable): {c}")).unwrap_or_default()
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("  ");
                    format!("{scope} {}  {switches}", r.name)
                })
                .collect::<Vec<_>>()
                .join("\n");
            Some(format!(
                "◇ Plugins and skills\nRunning agents are not affected.\n↑/↓ select; Space toggle; a adopt; r refresh; Esc back\n{rows}\n{}",
                app.extension_errors
            ))
        }
        Mode::Help | Mode::Palette => {
            Some(input::COMMANDS.iter().map(|c| format!("{}  {}", c.key, c.title)).collect::<Vec<_>>().join("\n"))
        }
        _ if app.nodes.is_empty() => {
            Some(format!("Detected harnesses:\n{}\n\n+ New agent: press c, enter harness task, then Enter", app.harnesses))
        }
        _ => None,
    };
    if let Some(text) = special {
        let area = if app.mode == Mode::Plugins { vertical[0] } else { columns[1] };
        frame.render_widget(
            Paragraph::new(text)
                .scroll((
                    if app.mode == Mode::Plugins {
                        app.extension_index.saturating_sub(area.height.saturating_sub(7) as usize) as u16
                    } else {
                        0
                    },
                    0,
                ))
                .block(Block::bordered().title("Canopy")),
            area,
        );
    } else {
        frame.render_widget(session::widget(app).block(Block::bordered().title("↵ Session - PgUp/PgDn")), columns[1]);
    }
    let permission = app.items.iter().find(|i| i.kind == ItemKind::Permission && i.state == ItemState::Streaming);
    let bar = permission
        .map(|i| format!("! {}  a allow / d deny", session::permission(&i.body)))
        .unwrap_or_else(|| format!("{}  c new · i message · x interrupt · : commands · q quit", app.message));
    frame.render_widget(Paragraph::new(bar).block(Block::bordered()), vertical[1]);
    frame.render_widget(
        Paragraph::new(app.text.as_str()).block(Block::bordered().title(match app.mode {
            Mode::New => "+ New agent: harness task",
            Mode::Filter => "⌕ Filter",
            Mode::Palette => "≡ Command name",
            _ => "› Message",
        })),
        vertical[2],
    );
}

struct Restore;
impl Drop for Restore {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
    }
}

pub async fn run() -> Result<()> {
    enable_raw_mode()?;
    let _restore = Restore;
    execute!(std::io::stdout(), EnterAlternateScreen)?;
    let mut terminal = ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stdout()))?;
    let mut app = App::default();
    terminal.draw(|f| draw(f, &app))?;
    let mut client = Client::connect(&crate::paths::home()?).await?;
    client.call(Op::Hello { version: protocol::VERSION }).await?;
    app.harnesses = "Checking installed CLIs...".into();
    // Probes take up to a second: run them on their own connection so keys and paint never wait.
    let (detect_tx, mut detect_rx) = tokio::sync::mpsc::channel(1);
    let home = crate::paths::home()?;
    tokio::spawn(async move {
        let rows = async {
            let mut c = Client::connect(&home).await?;
            let r = c.call(Op::Detect).await?;
            anyhow::Ok(serde_json::from_value::<Vec<HarnessDetection>>(r["harnesses"].clone())?)
        };
        let _ = detect_tx.send(rows.await).await;
    });
    let tree = client.call(Op::SubTree).await?;
    app.nodes = serde_json::from_value(tree["nodes"].clone())?;
    if let Some(n) = app.nodes.first() {
        app.select(&mut client, n.id.clone()).await?;
    }
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    std::thread::spawn(move || {
        loop {
            let e = event::read();
            let failed = e.is_err();
            if tx.blocking_send(e).is_err() || failed {
                break;
            }
        }
    });
    let mut connected = true;
    let mut pending = true;
    let mut last = tokio::time::Instant::now();
    loop {
        tokio::select! {
            e = client.next_event(), if connected => {
                match e { Ok(e) => app.event(e), Err(_) => {
                    connected = false;
                    app.message = "Daemon disconnected. Waiting to reconnect...".into();
                }}
                pending = true;
            }
            _ = tokio::time::sleep(Duration::from_secs(1)), if !connected => {
                if let Ok(mut c) = Client::attach(&crate::paths::home()?).await {
                    let sync = async {
                        let tree = c.call(Op::SubTree).await?;
                        app.nodes = serde_json::from_value(tree["nodes"].clone())?;
                        if let Some(id) = app.selected.take() { app.select(&mut c, id).await?; }
                        anyhow::Ok(())
                    }.await;
                    if sync.is_ok() { client = c; connected = true; app.message.clear(); }
                    pending = true;
                }
            }
            Some(rows) = detect_rx.recv() => {
                app.harnesses = match rows { Ok(rows) => harness_list(&rows), Err(e) => format!("Could not check CLIs: {e}") };
                pending = true;
            }
            e = rx.recv() => {
                match e.context("terminal input closed")?? {
                    TerminalEvent::Key(k) if connected || matches!(k.code, KeyCode::Char('q')) => match app.key(k, &mut client).await { Ok(true) => break, Ok(false) => {}, Err(e) => app.message = e.to_string() },
                    TerminalEvent::Resize(_, _) => {},
                    _ => continue,
                }
                pending = true;
            }
            _ = tokio::time::sleep_until(last + Duration::from_millis(34)), if pending => {
                terminal.draw(|f| draw(f, &app))?; last = tokio::time::Instant::now(); pending = false;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(dead_code)]
pub async fn scripted_test(home: &std::path::Path) -> Result<()> {
    use ratatui::backend::TestBackend;
    let mut client = Client::connect(home).await?;
    let mut app = App::default();
    let mut terminal = ratatui::Terminal::new(TestBackend::new(100, 30))?;
    terminal.draw(|f| draw(f, &app))?;
    assert!(terminal.backend().buffer().content.iter().any(|c| c.symbol() == "+"));
    let tree = client.call(Op::SubTree).await?;
    assert!(tree["nodes"].as_array().is_some_and(Vec::is_empty));
    for (key, mode) in [('p', Mode::Plugins), ('t', Mode::Tree)] {
        app.key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE), &mut client).await?;
        app.key(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE), &mut client).await?;
        assert_eq!(app.mode, mode);
    }
    app.key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE), &mut client).await?;
    for c in "fake ask test".chars() {
        app.key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE), &mut client).await?;
    }
    app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &mut client).await?;
    let session = app.selected.clone().context("launch did not select a session")?;
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            app.event(client.next_event().await?);
            if app.items.iter().any(|i| i.kind == ItemKind::Permission && i.state == ItemState::Streaming) {
                break;
            }
        }
        anyhow::Ok(())
    })
    .await??;
    terminal.draw(|f| draw(f, &app))?;
    app.key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE), &mut client).await?;
    app.key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE), &mut client).await?;
    app.text = "follow up".into();
    app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &mut client).await?;
    assert!(!app.message.is_empty());
    app.key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE), &mut client).await?;
    assert!(app.key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE), &mut client).await?);
    drop(client);
    let mut client = Client::connect(home).await?;
    let tree = client.call(Op::SubTree).await?;
    assert!(tree["nodes"].as_array().context("nodes")?.iter().any(|n| n["id"] == session));
    let mut app = App::default();
    app.select(&mut client, session).await?;
    terminal.draw(|f| draw(f, &app))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_rows_render_state_and_login_hint() {
        let row = |version: Option<&str>, supported, logged_in| HarnessDetection {
            name: "Codex".into(),
            binary: "codex".into(),
            version: version.map(Into::into),
            logged_in,
            recorded: "0.165.0".into(),
            supported,
            login_command: "codex login".into(),
        };
        let text = harness_list(&[
            row(None, true, None),
            row(Some("1"), true, Some(false)),
            row(Some("1"), true, Some(true)),
            row(Some("1"), false, None),
        ]);
        let lines: Vec<_> = text.lines().map(|l| l.split_whitespace().skip(2).collect::<Vec<_>>().join(" ")).collect();
        assert_eq!(lines, ["not installed", "logged out - run codex login", "ready", "detected, not supported yet"]);
    }
}
