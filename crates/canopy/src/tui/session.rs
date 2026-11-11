use super::App;
use protocol::ItemKind;
use ratatui::widgets::{Paragraph, Wrap};
use serde_json::Value;

fn label(kind: &ItemKind) -> &'static str {
    match kind {
        ItemKind::UserMessage => "› You",
        ItemKind::AgentMessage => "‹ Agent",
        ItemKind::Reasoning => "∴ Thinking",
        ItemKind::ToolCall => "◇ Tool",
        ItemKind::FileChange => "+ File change",
        ItemKind::Permission => "! Permission",
        ItemKind::Error => "× Error",
        ItemKind::Notice => "○ Notice",
    }
}

fn summary(value: &Value) -> String {
    match value {
        Value::String(text) => text.split_whitespace().collect::<Vec<_>>().join(" "),
        Value::Object(fields) => fields.iter().map(|(key, value)| format!("{key}: {}", summary(value))).collect::<Vec<_>>().join("; "),
        Value::Array(values) => values.iter().map(summary).collect::<Vec<_>>().join(", "),
        Value::Null => String::new(),
        value => value.to_string(),
    }
}

pub fn permission(body: &Value) -> String {
    let title =
        body.get("title").and_then(Value::as_str).or_else(|| body.get("tool").and_then(Value::as_str)).unwrap_or("Permission request");
    let input = body.get("input").or_else(|| body.pointer("/tool/rawInput")).or_else(|| body.get("request"));
    let input = input.map(summary).unwrap_or_default();
    let short: String = input.chars().take(100).collect();
    if input.is_empty() { title.into() } else { format!("{title}: {short}{}", if input.chars().count() > 100 { "…" } else { "" }) }
}

pub fn widget(app: &App) -> Paragraph<'static> {
    let text = app
        .items
        .iter()
        .map(|i| {
            let body = if i.kind == ItemKind::Permission {
                permission(&i.body)
            } else {
                i.body
                    .get("text")
                    .or_else(|| i.body.get("title"))
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .unwrap_or_else(|| summary(&i.body))
            };
            format!("{}: {body}", label(&i.kind))
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    Paragraph::new(text).wrap(Wrap { trim: false }).scroll((app.scroll, 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn transcript_renders_plain_kind_labels() {
        let mut app = App::default();
        app.items.push(protocol::Item {
            id: "item".into(),
            session: "session".into(),
            seq: 1,
            ts: 0,
            kind: ItemKind::AgentMessage,
            state: protocol::ItemState::Complete,
            body: json!({"text":"Hello"}),
        });
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(40, 3)).unwrap();
        terminal.draw(|frame| frame.render_widget(widget(&app), frame.area())).unwrap();
        let text: String = terminal.backend().buffer().content.iter().map(|cell| cell.symbol()).collect();
        assert!(text.contains("‹ Agent: Hello"));
        assert!(!text.contains("AgentMessage"));
    }

    #[test]
    fn permission_summarizes_adapter_inputs_as_text() {
        assert_eq!(
            permission(&json!({"title":"Bash", "input":{"command":"cargo test\n--offline"}})),
            "Bash: command: cargo test --offline"
        );
        assert_eq!(permission(&json!({"title":"Edit", "tool":{"rawInput":{"path":"src/main.rs"}}})), "Edit: path: src/main.rs");
        assert_eq!(permission(&json!({"title":"Run", "request":{"command":"pwd"}})), "Run: command: pwd");
        assert_eq!(permission(&json!({"tool":"Bash"})), "Bash");
        assert!(permission(&json!({"input":"x".repeat(120)})).ends_with('…'));
    }
}
