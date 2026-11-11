use super::*;
use ratatui::widgets::Clear;

pub(super) struct Toast {
    pub title: String,
    pub body: String,
    pub target: Option<String>,
    pub expires: Option<tokio::time::Instant>,
}
impl Toast {
    pub fn error(title: String, body: String, target: Option<String>) -> Self {
        Self { title, body, target, expires: None }
    }
    pub fn info(title: &str) -> Self {
        Self { title: title.into(), body: String::new(), target: None, expires: Some(tokio::time::Instant::now() + Duration::from_secs(4)) }
    }
}
pub(super) fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let width = area.width.min(56);
    let mut y = area.y;
    for (index, toast) in app.toasts.iter().enumerate().rev() {
        let height = (3 + toast.body.lines().take(3).count() as u16).min(area.bottom().saturating_sub(y));
        if height < 3 {
            break;
        }
        let rect = Rect::new(area.right() - width, y, width, height);
        frame.render_widget(Clear, rect);
        app.hits.push((rect, mouse::Target::Block));
        let text = format!(
            "{}\n{}",
            if toast.target.is_some() { "[Open]" } else { "" },
            toast.body.lines().take(3).collect::<Vec<_>>().join("\n")
        );
        frame.render_widget(
            Paragraph::new(text)
                .block(Block::bordered().title(toast.title.as_str()).title_top(ratatui::text::Line::from("[×]").right_aligned())),
            rect,
        );
        if let Some(id) = &toast.target {
            app.hits.push((Rect::new(rect.x + 1, y + 1, 6.min(width.saturating_sub(2)), 1), mouse::Target::Open(id.clone())));
        }
        app.hits.push((Rect::new(rect.right().saturating_sub(4), y, 3.min(width), 1), mouse::Target::Dismiss(index)));
        y += height;
    }
}
