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
            .skip(app.tree_offset)
            .map(|(n, depth)| {
                let glyph = match n.status {
                    Status::Running => "▷",
                    Status::Starting => "◌",
                    Status::AwaitingPermission => "!",
                    Status::Failed => "×",
                    Status::RateLimited { .. } => "∥",
                    Status::Done => "✓",
                    Status::Idle => "○",
                    Status::Cancelled => "⊘",
                    Status::Interrupted => "■",
                };
                let cost = n.usage.reported_cost_usd.map(|v| format!(" ${v:.2}")).unwrap_or_default();
                let collapse = if app.nodes.iter().any(|child| child.parent.as_ref() == Some(&n.id)) {
                    if app.collapsed.contains(&n.id) { "▸ " } else { "▾ " }
                } else {
                    ""
                };
                let prefix = format!(
                    "{}{}{} [{}] {}",
                    if app.selected.as_deref() == Some(n.id.as_str()) { "›" } else { " " },
                    "  ".repeat(depth),
                    glyph,
                    n.harness,
                    collapse
                );
                let detail = format!(
                    "{prefix}{}\n{}{}  {}t{} +{}/-{} {}",
                    n.title,
                    " ".repeat(ratatui::text::Line::from(prefix.as_str()).width()),
                    &n.id[n.id.len().saturating_sub(8)..],
                    n.subtree.tokens,
                    cost,
                    n.changes.added,
                    n.changes.removed,
                    n.activity
                );
                let color = match n.status {
                    Status::Failed => ratatui::style::Color::Red,
                    Status::AwaitingPermission => ratatui::style::Color::Yellow,
                    Status::Running => ratatui::style::Color::Green,
                    _ => ratatui::style::Color::Reset,
                };
                let mut lines: Vec<ratatui::text::Line> = detail.lines().map(|s| ratatui::text::Line::from(s.to_owned())).collect();
                if let Some(line) = lines.first_mut() {
                    let pos = line.spans[0].content.find(glyph).unwrap_or(0);
                    let text = line.spans[0].content.to_string();
                    *line = ratatui::text::Line::from(vec![
                        ratatui::text::Span::raw(text[..pos].to_owned()),
                        ratatui::text::Span::styled(glyph, ratatui::style::Style::default().fg(color)),
                        ratatui::text::Span::raw(text[pos + glyph.len()..].to_owned()),
                    ]);
                }
                if n.status == Status::Failed
                    && let Some(error) = &n.error
                {
                    // Align the reason under the row's detail line, not at the pane edge.
                    let indent: String = detail.lines().last().unwrap_or_default().chars().take_while(|c| *c == ' ').collect();
                    lines.push(ratatui::text::Line::styled(
                        format!("{indent}{}", error.lines().next().unwrap_or_default()),
                        ratatui::style::Style::default().add_modifier(ratatui::style::Modifier::DIM),
                    ));
                }
                ListItem::new(lines)
            })
            .collect::<Vec<_>>(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn details_align_with_titles_and_only_parents_have_chevrons() {
        let mut app = App::default();
        for (id, parent, title) in [("root12345678", None, "Root"), ("child87654321", Some("root12345678"), "Child")] {
            app.nodes.push(serde_json::from_value(serde_json::json!({
                "id":id,"parent":parent,"root":"root12345678","origin":"user","spawn_call":null,
                "harness":"demo","vendor_session":null,"title":title,"cwd":"/tmp","model":null,
                "policy":"ask","status":{"state":"idle"},"activity":"","usage":{"input":0,"output":0,"cache_read":0,"cache_write":0,"reported_cost_usd":null},
                "changes":{"files":[],"added":0,"removed":0},"subtree":{"tokens":0,"active":0},"created":0,"updated":0,"ended":null
            })).unwrap());
        }
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 6)).unwrap();
        terminal.draw(|f| f.render_widget(widget(&app), f.area())).unwrap();
        let lines: Vec<String> = terminal.backend().buffer().content.chunks(80).map(|r| r.iter().map(|c| c.symbol()).collect()).collect();
        assert_eq!(lines[0].split("Root").next().unwrap().chars().count(), lines[1].find("12345678").unwrap());
        assert_eq!(lines[2].split("Child").next().unwrap().chars().count(), lines[3].find("87654321").unwrap());
        assert!(lines[0].contains("▾ Root"));
        assert!(!lines[2].contains('▾'));
        assert!(!lines[2].contains('−'));
    }
    #[test]
    fn scope_fallback_has_one_cell() {
        assert_eq!(ratatui::text::Line::from("⊕").width(), 1);
        for globe in ["\u{1f30d}", "\u{1f30e}", "\u{1f30f}"] {
            assert_eq!(ratatui::text::Line::from(globe).width(), 2);
        }
    }
}
