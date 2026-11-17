//! The canvas.pane_* commands: open, read, shoot and point canvas panes. See docs/panes.md.
use super::*;

impl AppWindow {
    /// Runs one canvas.pane_* command and answers the caller.
    pub(in crate::window) fn pane_command(&mut self, name: &str, a: &Value, window: &mut Window, cx: &mut Context<Self>) {
        let r = match name {
            "canvas.pane_new" => self.pane_new(a, window, cx),
            "canvas.pane_read" => self.pane_read(a),
            "canvas.pane_set" => self.pane_set(a, cx),
            "canvas.pane_shot" => self.pane_shot(a, window),
            _ => self.pane_open(a),
        };
        self.answer(r);
    }

    fn pane_new(&mut self, a: &Value, window: &Window, cx: &mut Context<Self>) -> Result<Value, String> {
        let mut kinds = vec!["file".to_owned()];
        kinds.extend(self.folders.iter().filter(|(_, f)| f.hint.pane.is_some()).map(|(id, _)| format!("mod:{id}")));
        let kind = a["kind"].as_str().unwrap_or_default();
        if !kinds.iter().any(|k| k == kind) {
            return Err(format!("canvas.pane_new: kind must be one of: {}", kinds.join(", ")));
        }
        let doc = &self.canvas_state.doc;
        let (point, space) = if let Some(t) = a["wire"].as_str() {
            let pane = doc.panes.iter().find(|p| p.id == t && p.terminal().is_some()).ok_or_else(|| format!("{t} is not a terminal pane on the canvas"))?;
            let r = self.wire_cell(doc, t).unwrap_or(pane.rect);
            ([r[0] + r[2] + self.person.canvas_gap, r[1]], pane.space.clone())
        } else {
            let point = match a.get("point") {
                Some(p) => match serde_json::from_value::<canvas::Point>(p.clone()) {
                    Ok(p) if p.iter().all(|v| v.is_finite()) => p,
                    _ => return Err("invalid canvas point".into()),
                },
                None => self.view_center(window),
            };
            (point, doc.room.clone())
        };
        let (id, path) = if kind == "file" {
            let path = a["path"].as_str().filter(|p| std::path::Path::new(p).is_absolute()).ok_or("canvas.pane_new: path must be absolute")?;
            let (id, path) = self.add_file_pane(std::path::Path::new(path), point, space, cx)?;
            (id, Some(path))
        } else {
            (self.add_mod_pane(kind.to_owned(), point, space, cx), None)
        };
        if let Some(t) = a["wire"].as_str() {
            self.canvas_state.doc.connect(&id, t);
            self.canvas_changed(cx);
        }
        let mut reply = json!({"pane":id,"kind":kind,"wired":a["wire"].is_string()});
        if let Some(path) = path { reply["path"] = json!(path); }
        Ok(reply)
    }

    /// Shallow-merges mod state only after validating the complete saved props.
    fn pane_set(&mut self, a: &Value, cx: &mut Context<Self>) -> Result<Value, String> {
        let Some(id) = a["pane"].as_str() else { return Err("canvas.pane_set needs pane".into()); };
        let pane = self.canvas_state.doc.panes.iter_mut().find(|p| p.id == id).ok_or_else(|| format!("no pane {id}"))?;
        let kind = pane.kind_name().to_owned();
        let canvas::Kind::Mod { props, .. } = &mut pane.kind else { return Err(format!("pane {id} is a {kind}: only mod panes take canvas.pane_set")); };
        let patch = a["props"].as_object().ok_or("canvas.pane_set: props must be an object")?;
        let mut merged = props.clone();
        merged.extend(patch.clone());
        if serde_json::to_vec(&merged).unwrap().len() > 1024 * 1024 { return Err("canvas.pane_set: props over 1 MiB".into()); }
        *props = merged;
        self.canvas_changed(cx);
        Ok(json!({}))
    }

    fn pane_read(&self, a: &Value) -> Result<Value, String> {
        let Some(id) = a["pane"].as_str() else { return Err("canvas.pane_read needs pane".into()); };
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id).ok_or_else(|| format!("no pane {id}"))?;
        match &pane.kind {
            canvas::Kind::File { path } => {
                let text = canvas::read_preview(std::path::Path::new(path)).map_err(|e| format!("cannot read {path}: {e}"))?;
                Ok(json!({"text":canvas::envelope(id, "file", &[("path", path)], &text)}))
            }
            canvas::Kind::Mod { kind, props } => Ok(json!({"text":canvas::envelope(id, kind, &[], props.get("text").and_then(Value::as_str).unwrap_or(""))})),
            canvas::Kind::Terminal { .. } => Err(format!("pane {id} is a terminal: read it with hc read {id}")),
            canvas::Kind::Sub { parent, .. } => Err(format!("pane {id} is a sub-agent cell: read its parent with hc read {parent}")),
        }
    }

    fn pane_shot(&self, a: &Value, window: &mut Window) -> Result<Value, String> {
        let Some(id) = a["pane"].as_str() else { return Err("canvas.pane_shot needs pane".into()); };
        let doc = &self.canvas_state.doc;
        let pane = doc.panes.iter().find(|p| p.id == id).ok_or_else(|| format!("no pane {id}"))?;
        if !doc.shows(pane.space.as_ref()) { return Err(format!("pane {id} is in another space: go there first")); }
        let r = if pane.boxed() { pane.rect } else { self.wire_cell(doc, id).ok_or_else(|| format!("pane {id} is not drawn in this view"))? };
        let bounds = self.bounds.borrow().get("canopy-canvas").copied().ok_or("the canvas is not shown")?;
        let [x, y] = doc.camera.to_screen([r[0], r[1]]);
        let (w, h) = (r[2] * doc.camera.zoom, r[3] * doc.camera.zoom);
        let (x0, y0) = (x.max(0.), y.max(0.));
        let (x1, y1) = ((x + w).min(f32::from(bounds.size.width)), (y + h).min(f32::from(bounds.size.height)));
        if x1 <= x0 || y1 <= y0 { return Err(format!("pane {id} is off screen: bring it into view first")); }
        let path = if let Some(value) = a.get("path") {
            let path = value.as_str().map(std::path::PathBuf::from).filter(|p| p.is_absolute() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")))
                .ok_or("canvas.pane_shot: path must be an absolute .png path")?;
            path
        } else {
            let path = std::env::temp_dir().join("canopy").join(format!("{}.png", id.replace(':', "-")));
            std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| format!("cannot create the shot directory: {e}"))?;
            path
        };
        let image = window.render_to_image().map_err(|e| format!("cannot take the shot: {e:#}"))?;
        let s = window.scale_factor();
        let physical = |origin: Pixels, local: f32, limit: u32| ((f32::from(origin) + local) * s).round().clamp(0., limit as f32) as u32;
        let (x0, x1) = (physical(bounds.origin.x, x0, image.width()), physical(bounds.origin.x, x1, image.width()));
        let (y0, y1) = (physical(bounds.origin.y, y0, image.height()), physical(bounds.origin.y, y1, image.height()));
        let crop = image::imageops::crop_imm(&image, x0, y0, x1 - x0, y1 - y0).to_image();
        crate::control::save_png(&crop, &path).map_err(|e| format!("cannot save the shot: {e:#}"))?;
        Ok(json!({"path":path}))
    }

    fn pane_open(&self, a: &Value) -> Result<Value, String> {
        let Some(id) = a["pane"].as_str() else { return Err("canvas.pane_open needs pane".into()); };
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id).ok_or_else(|| format!("no pane {id}"))?;
        Err(format!("pane {id} ({}) has no verb open", pane.kind_name()))
    }
}

#[cfg(test)]
mod tests {
    use super::{canvas, json, Bounds, point, px, size, TestAppContext};
    use crate::window::tests::{in_own_process, open_window, running_alone};
    const TEST: &str = "window::canvas_view::pane_commands::tests::pane_commands_headless";

    #[test]
    fn pane_commands_open_read_move_refuse_and_close() { in_own_process(TEST); }

    const MOD_TEST: &str = "window::canvas_view::pane_commands::tests::mod_pane_commands_headless";

    #[test]
    fn mod_pane_commands() { in_own_process(MOD_TEST); }

    #[gpui::test]
    fn mod_pane_commands_headless(cx: &mut TestAppContext) {
        if !running_alone(MOD_TEST) { return; }
        let (view, cx) = open_window(cx, "mod-pane-commands", "", json!({"id":"root","type":"canvas"}));
        view.update_in(cx, |this, window, cx| {
            this.folders.insert("note".into(), crate::mods::Found {
                id: "note".into(), dir: std::path::PathBuf::from("/tmp/note"),
                hint: crate::mods::Hint { pane: Some(crate::mods::PaneHint { label: "Note".into(), icon: "mods/note/sticky-note.svg".into(), actions: vec![] }), ..Default::default() },
                stamp: std::time::UNIX_EPOCH,
            });
            this.canvas_state.doc.place("t1", [0., 0.], [220., 36.]);
            assert_eq!(this.pane_new(&json!({"kind":"mod:nope"}), window, cx).unwrap_err(), "canvas.pane_new: kind must be one of: file, mod:note");
            let reply = this.pane_new(&json!({"kind":"mod:note","wire":"t1"}), window, cx).unwrap();
            assert_eq!(reply, json!({"pane":"p1","kind":"mod:note","wired":true}));
            assert_eq!(crate::wire::neighbors(&this.canvas_state.doc.wires, "t1"), ["p1"]);
            assert_eq!(this.pane_read(&json!({"pane":"p1"})).unwrap()["text"], canvas::envelope("p1", "mod:note", &[], ""));
            assert!(this.pane_set(&json!({"pane":"p1","props":{"text":"hello"}}), cx).is_ok());
            assert_eq!(this.pane_read(&json!({"pane":"p1"})).unwrap()["text"], canvas::envelope("p1", "mod:note", &[], "hello"));
            assert_eq!(this.pane_set(&json!({"pane":"p1","props":3}), cx).unwrap_err(), "canvas.pane_set: props must be an object");
            assert_eq!(this.pane_set(&json!({"pane":"t1","props":{}}), cx).unwrap_err(), "pane t1 is a terminal: only mod panes take canvas.pane_set");
            assert_eq!(this.pane_set(&json!({"pane":"p1","props":{"text":"x".repeat(1024 * 1024 + 1)}}), cx).unwrap_err(), "canvas.pane_set: props over 1 MiB");
            assert_eq!(this.pane_read(&json!({"pane":"p1"})).unwrap()["text"], canvas::envelope("p1", "mod:note", &[], "hello"));
            assert_eq!(this.pane_set(&json!({"props":{}}), cx).unwrap_err(), "canvas.pane_set needs pane");
            // An agent cannot delete the person's text; the confirmed dialog's list can.
            this.close_canvas_cell(&json!({"pane":"p1"}), window, cx);
            assert!(this.canvas_state.doc.panes.iter().any(|p| p.id == "p1"));
            this.close_canvas_cell(&json!({"panes":["p1"]}), window, cx);
            assert!(!this.canvas_state.doc.panes.iter().any(|p| p.id == "p1"));
            let blank = this.pane_new(&json!({"kind":"mod:note"}), window, cx).unwrap();
            this.close_canvas_cell(&json!({"pane":blank["pane"]}), window, cx);
            assert!(!this.canvas_state.doc.panes.iter().any(|p| p.id == blank["pane"]), "a blank note closes without asking");
            assert!(this.canvas_state.doc.wires.is_empty());
        });
    }

    #[gpui::test]
    fn pane_commands_headless(cx: &mut TestAppContext) {
        if !running_alone(TEST) { return; }
        let (view, cx) = open_window(cx, "pane-commands", "", json!({"id":"root","type":"canvas"}));
        view.update_in(cx, |this, window, cx| {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md");
            this.canvas_state.doc.place("t1", [0., 0.], [220., 36.]);
            assert_eq!(this.pane_new(&json!({"kind":"browser","path":path}), window, cx).unwrap_err(), "canvas.pane_new: kind must be one of: file, mod:note");
            assert_eq!(this.pane_new(&json!({"kind":"file","path":"README.md"}), window, cx).unwrap_err(), "canvas.pane_new: path must be absolute");
            assert_eq!(this.pane_new(&json!({"kind":"file","path":path,"point":null}), window, cx).unwrap_err(), "invalid canvas point");
            assert_eq!(this.pane_new(&json!({"kind":"file","path":path,"wire":"missing"}), window, cx).unwrap_err(), "missing is not a terminal pane on the canvas");
            let reply = this.pane_new(&json!({"kind":"file","path":path,"wire":"t1"}), window, cx).unwrap();
            let id = reply["pane"].as_str().unwrap();
            assert_eq!(reply["wired"], true);
            assert_eq!(crate::wire::neighbors(&this.canvas_state.doc.wires, "t1"), [id]);
            let read = this.pane_read(&json!({"pane":id})).unwrap();
            assert_eq!(read["text"], canvas::envelope(id, "file", &[("path", reply["path"].as_str().unwrap())], &canvas::read_preview(&path).unwrap()));
            assert_eq!(this.pane_read(&json!({"pane":"missing"})).unwrap_err(), "no pane missing");
            assert_eq!(this.pane_read(&json!({"pane":"t1"})).unwrap_err(), "pane t1 is a terminal: read it with hc read t1");
            assert_eq!(this.pane_open(&json!({"pane":id,"target":"x"})).unwrap_err(), format!("pane {id} (file) has no verb open"));
            this.canvas_state.doc.room = Some("other".into());
            assert_eq!(this.pane_shot(&json!({"pane":id}), window).unwrap_err(), format!("pane {id} is in another space: go there first"));
            let moved = this.pane_new(&json!({"kind":"file","path":path,"point":[10000,10000]}), window, cx).unwrap();
            assert_eq!(moved["pane"], id);
            assert_eq!(this.canvas_state.doc.panes.iter().find(|p| p.id == id).unwrap().space.as_deref(), Some("other"));
            assert!(this.canvas_state.doc.wires.is_empty(), "a pane moved to another space leaves its wires");
            this.bounds.borrow_mut().insert("canopy-canvas".into(), Bounds::new(point(px(0.), px(0.)), size(px(800.), px(600.))));
            assert_eq!(this.pane_shot(&json!({"pane":id}), window).unwrap_err(), format!("pane {id} is off screen: bring it into view first"));
            this.close_canvas_cell(&json!({"pane":id}), window, cx);
            assert!(!this.canvas_state.doc.panes.iter().any(|p| p.id == id));
            assert!(this.canvas_state.doc.wires.is_empty());
        });
    }
}
