use super::*;
use ratatui::layout::Position;
#[derive(Clone)]
pub(super) enum Target {
    Command(CommandId),
    Row(String),
    Open(String),
    Dismiss(usize),
    Field(usize),
    Harness(usize),
    TreePane,
    SessionPane,
    Block,
}
impl App {
    pub(super) fn hit(&self, x: u16, y: u16) -> Option<Target> {
        self.hits.iter().rev().find(|(r, _)| r.contains(Position::new(x, y))).map(|(_, t)| t.clone())
    }
    pub(super) async fn mouse(&mut self, event: crossterm::event::MouseEvent, client: &mut Client) -> Result<bool> {
        let Some(target) = self.hit(event.column, event.row) else {
            return Ok(false);
        };
        match event.kind {
            MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                let down = event.kind == MouseEventKind::ScrollDown;
                if self.mode == Mode::Help {
                    self.scroll = if down {
                        (self.scroll + 1).min(input::COMMANDS.len().saturating_sub(1) as u16)
                    } else {
                        self.scroll.saturating_sub(1)
                    };
                    return Ok(false);
                }
                match target {
                    Target::TreePane | Target::Row(_) => {
                        self.tree_offset = if down {
                            (self.tree_offset + 1).min(tree::rows(self).len().saturating_sub(1))
                        } else {
                            self.tree_offset.saturating_sub(1)
                        };
                    }
                    Target::SessionPane => self.scroll = if down { self.scroll.saturating_add(3) } else { self.scroll.saturating_sub(3) },
                    _ => {}
                }
            }
            MouseEventKind::Down(MouseButton::Left) => match target {
                Target::Command(id) => return self.command(id, client).await,
                Target::Row(id) => {
                    let double = self.last_click.as_ref().is_some_and(|(old, at)| old == &id && at.elapsed() < Duration::from_millis(400));
                    self.last_click = Some((id.clone(), std::time::Instant::now()));
                    self.select(client, id).await?;
                    if double {
                        return self.command(CommandId::Open, client).await;
                    }
                }
                Target::Open(id) => {
                    self.select(client, id).await?;
                    self.mode = Mode::Tree;
                }
                Target::Dismiss(i) => {
                    self.toasts.remove(i);
                }
                Target::Harness(i) => {
                    self.form.harness = Some(i);
                    self.form.focus = 0;
                }
                Target::Field(i) => {
                    self.form.focus = i;
                    if i == 3 {
                        return self.form_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), client).await;
                    }
                }
                _ => {}
            },
            _ => {}
        }
        Ok(false)
    }
}
pub(super) fn record_panes(app: &mut App, columns: &[Rect]) {
    app.hits.push((columns[0], Target::TreePane));
    app.hits.push((columns[1], Target::SessionPane));
    let mut y = columns[0].y + 1;
    let rows: Vec<_> = tree::rows(app)
        .into_iter()
        .skip(app.tree_offset)
        .map(|(n, _)| (n.id.clone(), if n.status == protocol::Status::Failed && n.error.is_some() { 3 } else { 2 }))
        .collect();
    for (id, height) in rows {
        let height = height.min(columns[0].bottom().saturating_sub(1).saturating_sub(y));
        if height == 0 {
            break;
        }
        app.hits.push((Rect::new(columns[0].x + 1, y, columns[0].width.saturating_sub(2), height), Target::Row(id)));
        y += height;
    }
}
pub(super) fn draw_commands(frame: &mut Frame, app: &mut App, area: Rect, permission: &str) {
    frame.render_widget(Block::bordered(), area);
    let mut x = area.x + 1;
    let mut y = area.y + 1;
    if !permission.is_empty() {
        frame.render_widget(Paragraph::new(permission), Rect::new(x, y, area.width.saturating_sub(2), 1));
        y += 1;
    }
    let mut buttons: Vec<_> = input::COMMANDS
        .iter()
        .filter(|c| {
            !matches!(
                c.id,
                // Allow and Deny get their own buttons while a permission is pending.
                CommandId::Allow
                    | CommandId::Deny
                    | CommandId::ExtensionNext
                    | CommandId::ExtensionPrevious
                    | CommandId::ExtensionToggle
                    | CommandId::ExtensionAdopt
                    | CommandId::ExtensionRefresh
            )
        })
        .map(|c| (format!("[{} {}]", c.key, c.title), c.id))
        .collect();
    if !permission.is_empty() {
        buttons.splice(
            0..0,
            [("[Allow]".into(), CommandId::Allow), ("[Deny]".into(), CommandId::Deny), ("[Stop]".into(), CommandId::Interrupt)],
        );
    }
    if app.mode == Mode::Plugins {
        for (label, code) in [
            ("[↑]", CommandId::ExtensionPrevious),
            ("[↓]", CommandId::ExtensionNext),
            ("[Toggle]", CommandId::ExtensionToggle),
            ("[Adopt]", CommandId::ExtensionAdopt),
            ("[Refresh]", CommandId::ExtensionRefresh),
        ] {
            let width = ratatui::text::Line::from(label).width() as u16;
            let rect = Rect::new(x, y, width, 1);
            frame.render_widget(Paragraph::new(label), rect);
            app.hits.push((rect, Target::Command(code)));
            x += width + 1;
        }
        x = area.x + 1;
        y += 1;
    }
    for (label, id) in buttons {
        let width = ratatui::text::Line::from(label.as_str()).width() as u16;
        if x + width >= area.right() {
            x = area.x + 1;
            y += 1;
        }
        if y >= area.bottom().saturating_sub(1) {
            break;
        }
        let rect = Rect::new(x, y, width.min(area.width.saturating_sub(2)), 1);
        frame.render_widget(Paragraph::new(label), rect);
        app.hits.push((rect, Target::Command(id)));
        x += width + 1;
    }
}
