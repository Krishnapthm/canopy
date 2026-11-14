//! The command menu shared by the native menu bar and the Linux titlebar.
use crate::canvas::View;
use serde_json::{Value, json};

/// Command groups for both menu surfaces, including the active view mark.
/// A `null` row is a separator (macOS only; the Linux menu skips it).
pub fn groups(view: &View) -> Vec<(&'static str, Vec<Value>)> {
    vec![
        ("File", vec![
            json!(["New Terminal", "canvas.create", {}]),
            json!(["Open File", "preview.open", {}]),
            json!(["Open Space", "canvas.create", {"kind":"space"}]),
            Value::Null,
            json!(["Pop Out Panel", "panel.pop_out", {}]),
            json!(["Close Panel", "panel.close", {}]),
        ]),
        ("Edit", vec![
            json!(["Copy", "terminal.copy", {}]),
            json!(["Paste", "terminal.paste", {}]),
            json!(["Select All", "terminal.select_all", {}]),
            json!(["Find", "terminal.search", {}]),
        ]),
        ("View", vec![
            json!(["Agents", "view.agents", {}, view == &View::Agents]),
            json!(["All", "view.all", {}, view == &View::All]),
            json!(["Terminals", "view.terminals", {}, view == &View::Terminals]),
            Value::Null,
            json!(["Zoom In", "canvas.zoom_in", {}]),
            json!(["Zoom Out", "canvas.zoom_out", {}]),
            json!(["Actual Size", "canvas.zoom_reset", {}]),
            json!(["Focus Space / Previous View", "canvas.fit", {}]),
            Value::Null,
            json!(["Command Palette", "app.palette", {}]),
        ]),
        ("Help", vec![
            json!(["Keyboard Shortcuts", "app.help", {}]),
            json!(["Welcome", "app.welcome", {}]),
        ]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn menus_share_valid_commands_and_one_active_view() {
        for view in [View::Agents, View::All, View::Terminals] {
            let groups = groups(&view);
            let mut checked = 0;
            for (_, rows) in groups {
                for row in rows.into_iter().filter(|r| !r.is_null()) {
                    let command = crate::commands::find(row[1].as_str().unwrap()).unwrap();
                    assert!(crate::commands::validate_args(command, &row[2]).is_ok());
                    checked += usize::from(row[3] == true);
                }
            }
            assert_eq!(checked, 1);
        }
    }
}
