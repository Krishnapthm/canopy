use super::App;
use protocol::{Node, Status};
use ratatui::widgets::{List, ListItem};

pub fn rows(app: &App) -> Vec<(&Node, usize)> {
    let mut out = Vec::new();
    let mut stack: Vec<_> = app.nodes.iter().filter(|n| n.parent.is_none()).rev().map(|n| (n, 0)).collect();
    while let Some((n, depth)) = stack.pop() {
        if app.filter.is_empty() || n.title.contains(&app.filter) || n.harness.contains(&app.filter) {
            out.push((n, depth));
        }
        if !app.collapsed.contains(&n.id) {
            stack.extend(app.nodes.iter().filter(|k| k.parent.as_ref() == Some(&n.id)).rev().map(|n| (n, depth + 1)));
        }
    }
    out
}

pub fn widget(app: &App) -> List<'static> {
    List::new(
        rows(app)
            .into_iter()
            .map(|(n, depth)| {
                let glyph = match n.status {
                    Status::Running | Status::Starting => "▷",
                    Status::AwaitingPermission => "!",
                    Status::Failed => "×",
                    Status::RateLimited { .. } => "∥",
                    Status::Done => "✓",
                    _ => "○",
                };
                let cost = n.usage.reported_cost_usd.map(|v| format!(" ${v:.2}")).unwrap_or_default();
                ListItem::new(format!(
                    "{}{}{} [{}] {} {}\n{}{}  {}t{} +{}/-{} {}",
                    if app.selected.as_deref() == Some(n.id.as_str()) { "›" } else { " " },
                    "  ".repeat(depth),
                    glyph,
                    n.harness,
                    n.title,
                    if app.collapsed.contains(&n.id) { "+" } else { "−" },
                    "  ".repeat(depth),
                    &n.id[n.id.len().saturating_sub(8)..],
                    n.subtree.tokens,
                    cost,
                    n.changes.added,
                    n.changes.removed,
                    n.activity
                ))
            })
            .collect::<Vec<_>>(),
    )
}
