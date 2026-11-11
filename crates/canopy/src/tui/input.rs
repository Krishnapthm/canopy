use crossterm::event::KeyCode;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CommandId {
    Down,
    Up,
    Collapse,
    Expand,
    Open,
    New,
    Child,
    Compose,
    Back,
    Interrupt,
    Allow,
    Deny,
    Filter,
    Palette,
    Tree,
    Plugins,
    Help,
    Quit,
}

pub struct Command {
    pub id: CommandId,
    pub title: &'static str,
    pub key: &'static str,
}

pub const COMMANDS: &[Command] = &[
    Command { id: CommandId::Down, title: "↓ Next", key: "j" },
    Command { id: CommandId::Up, title: "↑ Previous", key: "k" },
    Command { id: CommandId::Collapse, title: "− Collapse", key: "h" },
    Command { id: CommandId::Expand, title: "+ Expand", key: "l" },
    Command { id: CommandId::Open, title: "↵ Open session", key: "Enter" },
    Command { id: CommandId::New, title: "+ New agent", key: "c" },
    Command { id: CommandId::Child, title: "+ Spawn child", key: "s" },
    Command { id: CommandId::Compose, title: "› Message", key: "i" },
    Command { id: CommandId::Back, title: "← Back", key: "Esc" },
    Command { id: CommandId::Interrupt, title: "× Interrupt", key: "x" },
    Command { id: CommandId::Allow, title: "✓ Allow", key: "a" },
    Command { id: CommandId::Deny, title: "× Deny", key: "d" },
    Command { id: CommandId::Filter, title: "⌕ Filter", key: "/" },
    Command { id: CommandId::Palette, title: "≡ Commands", key: ":" },
    Command { id: CommandId::Tree, title: "≡ Tree", key: "g t" },
    Command { id: CommandId::Plugins, title: "◇ Plugins and skills", key: "g p" },
    Command { id: CommandId::Help, title: "? Help", key: "?" },
    Command { id: CommandId::Quit, title: "× Quit", key: "q" },
];

pub fn command(code: KeyCode, prefix: bool) -> Option<CommandId> {
    COMMANDS
        .iter()
        .find(|c| match code {
            KeyCode::Char(k) if prefix => c.key.strip_prefix("g ").is_some_and(|suffix| suffix.starts_with(k)),
            _ if prefix => false,
            KeyCode::Char(k) => c.key.len() == 1 && c.key.starts_with(k),
            KeyCode::Enter => c.key == "Enter",
            KeyCode::Esc => c.key == "Esc",
            _ => false,
        })
        .map(|c| c.id)
}
