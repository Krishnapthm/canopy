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
            json!(["Pop Out Pane", "panel.pop_out", {}]),
            json!(["Close Pane", "panel.close", {}]),
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

/// Builds the rows for the in-window menu and its pages.
pub fn popup(view: &View, group: Option<&str>) -> Option<Value> {
    let groups = groups(view);
    let mut home: Vec<_> = groups.iter()
        .filter(|(name, _)| *name != "Help")
        .map(|(name, _)| json!([name, "app.menu", {"group":name}]))
        .collect();
    home.push(json!(["Settings", "app.settings", {}]));
    let mut pages = serde_json::Map::new();
    for (name, rows) in groups {
        let mut entries = vec![json!(["Back", "app.menu", {}])];
        for mut row in rows.into_iter().filter(|row| !row.is_null()) {
            if row[3] == true {
                row[0] = json!(format!("✓ {}", row[0].as_str().unwrap()));
            }
            row.as_array_mut().unwrap().truncate(3);
            if name == "Help" {
                home.push(row.clone());
            }
            entries.push(row);
        }
        pages.insert(name.into(), json!(entries));
    }
    let items = match group {
        Some(name) => pages.get(name)?.clone(),
        None => json!(home),
    };
    Some(json!({"items":items, "home":home, "groups":pages, "group":group}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn popup_has_direct_settings_and_plain_group_labels() {
        let menu = popup(&View::Agents, None).unwrap();
        let rows = menu["items"].as_array().unwrap();
        assert_eq!(rows[0], json!(["File", "app.menu", {"group":"File"}]));
        assert!(rows.contains(&json!(["Settings", "app.settings", {}])));
        assert!(rows.contains(&json!(["Keyboard Shortcuts", "app.help", {}])));
        let view = popup(&View::Agents, Some("View")).unwrap();
        assert_eq!(view["items"][0], json!(["Back", "app.menu", {}]));
        assert_eq!(view["items"][1][0], "✓ Agents");
        assert!(popup(&View::Agents, Some("unknown")).is_none());
    }

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
