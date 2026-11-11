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

fn tool_summary(body: &Value, app: &App) -> String {
    let title = body["title"].as_str().unwrap_or("Tool");
    let output = body.pointer("/output/0/content/text").and_then(Value::as_str);
    let result: Value = output.and_then(|s| serde_json::from_str(s).ok()).unwrap_or(Value::Null);
    let short = |id: &str| id[id.len().saturating_sub(8)..].to_owned();
    let details = if title.ends_with("spawn_agent") {
        result["id"]
            .as_str()
            .map(|id| {
                let name = app.nodes.iter().find(|n| n.id == id).map(|n| n.title.as_str()).unwrap_or("agent");
                format!("started {name} {}", short(id))
            })
            .unwrap_or_default()
    } else if title.ends_with("wait_agent") {
        let entries = result.as_array().or_else(|| result.get("agents").and_then(Value::as_array));
        if let Some(entries) = entries {
            entries
                .iter()
                .map(|entry| format!("{}: {}", short(entry["id"].as_str().unwrap_or("")), entry["status"].as_str().unwrap_or("unknown")))
                .collect::<Vec<_>>()
                .join(", ")
        } else if let Some(ids) = result["still_running"].as_array() {
            ids.iter().filter_map(Value::as_str).map(|id| format!("{}: running", short(id))).collect::<Vec<_>>().join(", ")
        } else {
            String::new()
        }
    } else {
        output.map(str::trim).unwrap_or("").to_owned()
    };
    let details = if details.is_empty() { output.map(|s| summary(&Value::String(s.to_owned()))).unwrap_or_default() } else { details };
    if details.is_empty() { title.to_owned() } else { format!("{title}\n  {details}") }
}

pub fn widget(app: &App) -> Paragraph<'static> {
    let text = app
        .items
        .iter()
        .map(|i| {
            let body = if i.kind == ItemKind::ToolCall {
                tool_summary(&i.body, app)
            } else if i.kind == ItemKind::Permission {
                permission(&i.body)
            } else {
                i.body
                    .get("text")
                    .or_else(|| i.body.get("title"))
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .unwrap_or_else(|| summary(&i.body))
            };
            format!("{}: {}", label(&i.kind), body.trim())
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
    fn tool_results_and_spacing() {
        let mut app = App::default();
        for (kind, body) in [
            (ItemKind::AgentMessage, json!({"text":"Planning\n\n"})),
            (ItemKind::ToolCall, json!({"title":"canopy spawn_agent", "output":[{"content":{"text":"{\"id\":\"session12345678\"}"}}]})),
            (
                ItemKind::ToolCall,
                json!({"title":"canopy wait_agent", "output":[{"content":{"text":"{\"agents\":[{\"id\":\"session12345678\",\"status\":\"idle\"}]}"}}]}),
            ),
            (ItemKind::AgentMessage, json!({"text":"Finished\n"})),
        ] {
            app.items.push(protocol::Item {
                id: app.items.len().to_string(),
                session: "session".into(),
                seq: 1,
                ts: 0,
                kind,
                state: protocol::ItemState::Complete,
                body,
            });
        }
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 12)).unwrap();
        terminal.draw(|f| f.render_widget(widget(&app), f.area())).unwrap();
        let lines: Vec<String> = terminal
            .backend()
            .buffer()
            .content
            .chunks(80)
            .map(|row| row.iter().map(|c| c.symbol()).collect::<String>().trim_end().to_owned())
            .collect();
        assert_eq!(
            &lines[..9],
            &[
                "‹ Agent: Planning",
                "",
                "◇ Tool: canopy spawn_agent",
                "  started agent 12345678",
                "",
                "◇ Tool: canopy wait_agent",
                "  12345678: idle",
                "",
                "‹ Agent: Finished"
            ]
        );
    }

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
