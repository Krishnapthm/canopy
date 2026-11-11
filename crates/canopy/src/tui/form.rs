use super::*;
use ratatui::widgets::Clear;
#[derive(Default)]
pub(super) struct Form {
    pub harness: Option<usize>,
    pub directory: String,
    pub task: String,
    pub focus: usize,
}
pub(super) fn ready(h: &HarnessDetection) -> bool {
    h.version.is_some() && h.supported && h.logged_in == Some(true)
}
impl Form {
    pub fn new(rows: &[HarnessDetection]) -> Result<Self> {
        Ok(Self {
            harness: rows.iter().position(ready),
            directory: std::env::current_dir()?.to_string_lossy().into_owned(),
            ..Self::default()
        })
    }
}
impl App {
    pub(super) async fn form_key(&mut self, key: KeyEvent, client: &mut Client) -> Result<bool> {
        match key.code {
            KeyCode::Esc => self.mode = Mode::Tree,
            KeyCode::Tab => self.form.focus = (self.form.focus + 1) % 4,
            KeyCode::BackTab => self.form.focus = (self.form.focus + 3) % 4,
            KeyCode::Up | KeyCode::Down if self.form.focus == 0 => {
                let count = self.detections.len();
                for step in 1..=count {
                    let current = self.form.harness.unwrap_or(0);
                    let index = if key.code == KeyCode::Down { (current + step) % count } else { (current + count - step) % count };
                    if ready(&self.detections[index]) {
                        self.form.harness = Some(index);
                        break;
                    }
                }
            }
            KeyCode::Enter if self.form.focus == 3 || (self.form.focus == 2 && key.modifiers.contains(KeyModifiers::CONTROL)) => {
                let harness =
                    self.form.harness.and_then(|i| self.detections.get(i)).filter(|h| ready(h)).context("Select a ready harness")?;
                anyhow::ensure!(!self.form.task.trim().is_empty(), "Enter a task");
                let r = client
                    .call(Op::Spawn(SpawnReq {
                        harness: harness.binary.clone(),
                        prompt: self.form.task.clone(),
                        cwd: self.form.directory.clone(),
                        model: None,
                        policy: Policy::Ask,
                        parent: self.parent.clone(),
                        title: None,
                    }))
                    .await?;
                self.select(client, r["session"].as_str().context("missing session id")?.into()).await?;
                self.mode = Mode::Tree;
            }
            KeyCode::Enter if self.form.focus == 2 => self.form.task.push('\n'),
            KeyCode::Enter => self.form.focus = (self.form.focus + 1) % 4,
            KeyCode::Char(c) => match self.form.focus {
                1 => self.form.directory.push(c),
                2 => self.form.task.push(c),
                _ => {}
            },
            KeyCode::Backspace => match self.form.focus {
                1 => {
                    self.form.directory.pop();
                }
                2 => {
                    self.form.task.pop();
                }
                _ => {}
            },
            _ => {}
        }
        Ok(false)
    }
}
pub(super) fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    app.hits.clear();
    frame.render_widget(Clear, area);
    frame.render_widget(Block::bordered().title("+ New agent - Tab fields / Esc cancel"), area);
    let inner = Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), area.height.saturating_sub(2));
    let parts = Layout::vertical([
        Constraint::Length(app.detections.len() as u16 + 2),
        Constraint::Length(3),
        Constraint::Min(3),
        Constraint::Length(3),
    ])
    .split(inner);
    let text = app
        .detections
        .iter()
        .enumerate()
        .map(|(i, h)| format!("{} {}", if app.form.harness == Some(i) { "›" } else { " " }, harness_list(std::slice::from_ref(h))))
        .collect::<Vec<_>>()
        .join("\n");
    for (i, h) in app.detections.iter().enumerate() {
        if ready(h) && (i as u16) < parts[0].height.saturating_sub(2) {
            app.hits.push((
                Rect::new(parts[0].x + 1, parts[0].y + 1 + i as u16, parts[0].width.saturating_sub(2), 1),
                mouse::Target::Harness(i),
            ));
        }
    }
    for (i, text, title) in [
        (0, text, "Harness"),
        (1, app.form.directory.clone(), "Directory"),
        (2, app.form.task.clone(), "Task (Ctrl+Enter starts)"),
        (3, "[Start]  [Cancel]".into(), "Enter starts"),
    ] {
        frame.render_widget(
            Paragraph::new(text).block(Block::bordered().title(format!("{} {title}", if app.form.focus == i { "›" } else { " " }))),
            parts[i],
        );
        if i > 0 {
            app.hits.push((parts[i], mouse::Target::Field(i)));
        }
    }
    app.hits.push((
        Rect::new(parts[3].x + 10, parts[3].y + 1, 8.min(parts[3].width.saturating_sub(10)), 1),
        mouse::Target::Command(CommandId::Back),
    ));
}
