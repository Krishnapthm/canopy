//! The canvas.pane_* commands: open, read, shoot and point canvas panes. See docs/wiki/Agent-Pane-Commands.md.
use super::*;

impl AppWindow {
    /// Runs one canvas.pane_* command and answers the caller.
    pub(in crate::window) fn pane_command(&mut self, name: &str, a: &Value, window: &mut Window, cx: &mut Context<Self>) {
        let r = match name {
            "canvas.pane_new" => self.pane_new(a, window, cx),
            "canvas.pane_pin" => {
                let id = a["pane"].as_str().map(str::to_owned).or_else(|| self.canvas_state.selected.last().cloned());
                match id {
                    Some(id) => self.pin_pane(&id, a["pinned"].as_bool(), false, window, cx).map(|pinned| json!({"pane":id,"pinned":pinned})),
                    None => Err("canvas.pane_pin needs pane".into()),
                }
            }
            "canvas.pane_read" | "canvas.pane_shot" if self.is_browser(a) => {
                let shot = (name == "canvas.pane_shot").then(|| self.shot_path(a));
                match shot.transpose().and_then(|shot| self.browser_reply(a["pane"].as_str().unwrap(), shot, window, cx)) {
                    // The page answers the held reply once it has loaded.
                    Ok(()) => return,
                    Err(e) => Err(e),
                }
            }
            "canvas.pane_read" => self.pane_read(a, cx),
            "canvas.pane_set" => self.pane_set(a, cx),
            "canvas.pane_shot" => self.pane_shot(a, window),
            _ if self.is_browser(a) => self.browser_open(a["pane"].as_str().unwrap(), a["target"].as_str().unwrap_or_default(), window, cx),
            _ => self.pane_open(a, window, cx),
        };
        self.answer(r);
    }

    /// Whether `a["pane"]` names a browser pane.
    fn is_browser(&self, a: &Value) -> bool {
        a["pane"].as_str().is_some_and(|id| self.canvas_state.doc.panes.iter().any(|p| p.id == id && p.url().is_some()))
    }

    fn pane_new(&mut self, a: &Value, window: &mut Window, cx: &mut Context<Self>) -> Result<Value, String> {
        let mut kinds = vec!["file".to_owned(), "browser".to_owned()];
        if crate::sim::available() { kinds.push("sim".to_owned()); }
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
                // A browser pane is large: centre it in the view rather than hang it from the middle.
                None if kind == "browser" => { let [x, y] = self.view_center(window); [x - super::browser::SIZE[0] / 2., y - super::browser::SIZE[1] / 2.] }
                None => self.view_center(window),
            };
            (point, doc.room.clone())
        };
        let url = if kind == "browser" { Some(super::browser::web_url(a["url"].as_str().ok_or("canvas.pane_new: a browser pane needs url")?)?) } else { None };
        // A named simulator must exist before a pane points at it.
        let udid = match a["udid"].as_str() { Some(target) if kind == "sim" => super::sim::resolve(target)?, _ => String::new() };
        // A pane opened pinned never lands on the canvas, so nothing should make room for it:
        // the layout solve that adding runs is undone once it is pinned.
        let pinned = a["pinned"].as_bool() == Some(true);
        let before: Option<HashMap<String, canvas::Rect>> = pinned.then(|| self.canvas_state.doc.panes.iter().map(|p| (p.id.clone(), p.rect)).collect());
        let (id, path, kind) = if let Some(url) = url.clone() {
            (self.add_browser_pane(url, point, space, cx), None, kind.to_owned())
        } else if kind == "file" {
            let path = a["path"].as_str().filter(|p| std::path::Path::new(p).is_absolute()).ok_or("canvas.pane_new: path must be absolute")?;
            let (id, path, kind) = self.add_file_pane(std::path::Path::new(path), point, space, window, cx)?;
            (id, Some(path), kind)
        } else if kind == "sim" {
            (self.add_sim_pane(udid.clone(), point, space, window, cx), None, kind.to_owned())
        } else {
            (self.add_mod_pane(kind.to_owned(), point, space, cx), None, kind.to_owned())
        };
        if let Some(before) = before {
            for p in &mut self.canvas_state.doc.panes {
                if let Some(r) = before.get(&p.id) { p.rect = *r; }
            }
            self.pin_pane(&id, Some(true), true, window, cx)?;
        }
        if let Some(t) = a["wire"].as_str() {
            self.canvas_state.doc.connect(&id, t);
            self.canvas_changed(cx);
        }
        let mut reply = json!({"pane":id,"kind":kind,"wired":a["wire"].is_string()});
        if pinned { reply["pinned"] = json!(true); }
        if let Some(path) = path { reply["path"] = json!(path); }
        if let Some(url) = url { reply["url"] = json!(url); }
        if kind == "sim" { reply["udid"] = json!(udid); }
        Ok(reply)
    }

    /// Pins a pane (`pin` None toggles) where it shows on screen, or with `corner` at the canvas's
    /// top right, stepped in from panes already pinned there. Unpinning leaves it on the canvas
    /// where it was showing, and the panes around it make room. Returns whether it is pinned.
    pub(super) fn pin_pane(&mut self, id: &str, pin: Option<bool>, corner: bool, window: &Window, cx: &mut Context<Self>) -> Result<bool, String> {
        const MARGIN: f32 = 16.;
        let size = self.bounds.borrow().get("canopy-canvas").map(|b| b.size).unwrap_or(window.viewport_size());
        let size = [f32::from(size.width), f32::from(size.height)];
        let doc = &mut self.canvas_state.doc;
        let camera = doc.camera;
        let stacked = doc.panes.iter().filter(|p| p.pinned.is_some() && p.id != id && doc.shows(p.space.as_ref())).count() as f32 * 24.;
        let pane = doc.panes.iter_mut().find(|p| p.id == id).ok_or_else(|| format!("no pane {id}"))?;
        let pin = pin.unwrap_or(pane.pinned.is_none());
        if !pin {
            pane.pinned = None;
        } else if pane.pinned.is_none() {
            let [x, y] = camera.to_screen([pane.rect[0], pane.rect[1]]);
            let [w, h] = [pane.rect[2] * camera.zoom, pane.rect[3] * camera.zoom];
            // It keeps its shape, at most the window's height and (in the corner) 60% of its width.
            let room = if corner { size[0] * 0.6 } else { size[0] - 2. * MARGIN };
            let k = (room / w).min((size[1] - 2. * MARGIN) / h).min(1.);
            let [w, h] = [(w * k).max(240.), (h * k).max(120.)];
            pane.pinned = Some(if corner { [size[0] - w - MARGIN - stacked, MARGIN + stacked, w, h] } else { [x, y, w, h] });
        }
        self.raise_pane(id, cx);
        self.pin_panes();
        if !pin { self.solve_layout(Some(id), None, cx); }
        self.canvas_changed(cx);
        cx.notify();
        Ok(pin)
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

    fn pane_read(&self, a: &Value, cx: &App) -> Result<Value, String> {
        let Some(id) = a["pane"].as_str() else { return Err("canvas.pane_read needs pane".into()); };
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id).ok_or_else(|| format!("no pane {id}"))?;
        match &pane.kind {
            canvas::Kind::File { path } => {
                let text = canvas::read_preview(std::path::Path::new(path)).map_err(|e| format!("cannot read {path}: {e}"))?;
                Ok(json!({"text":canvas::envelope(id, "file", &[("path", path)], &text)}))
            }
            canvas::Kind::Image { path } => {
                let &(w, h) = self.canvas_state.image_sizes.get(path).ok_or_else(|| format!("cannot read {path}: not a readable image"))?;
                Ok(json!({"text":canvas::envelope(id, "image", &[("path", path), ("size", &format!("{w}x{h}"))], "")}))
            }
            canvas::Kind::Pdf { path, .. } => Ok(json!({"text":pdf::read_pdf(id, path)?})),
            canvas::Kind::Mod { kind, props } => Ok(json!({"text":canvas::envelope(id, kind, &[], props.get("text").and_then(Value::as_str).unwrap_or(""))})),
            canvas::Kind::Sim { udid } => Ok(json!({"text":self.sim_read(id, udid, cx)})),
            canvas::Kind::Terminal { .. } => Err(format!("pane {id} is a terminal: read it with hc read {id}")),
            canvas::Kind::Sub { parent, .. } => Err(format!("pane {id} is a sub-agent cell: read its parent with hc read {parent}")),
            canvas::Kind::Browser { .. } => Err(format!("pane {id} is a browser pane: its page answers only `canopy pane read`")),
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
        let path = self.shot_path(a)?;
        let image = window.render_to_image().map_err(|e| format!("cannot take the shot: {e:#}"))?;
        let s = window.scale_factor();
        let physical = |origin: Pixels, local: f32, limit: u32| ((f32::from(origin) + local) * s).round().clamp(0., limit as f32) as u32;
        let (x0, x1) = (physical(bounds.origin.x, x0, image.width()), physical(bounds.origin.x, x1, image.width()));
        let (y0, y1) = (physical(bounds.origin.y, y0, image.height()), physical(bounds.origin.y, y1, image.height()));
        let crop = image::imageops::crop_imm(&image, x0, y0, x1 - x0, y1 - y0).to_image();
        crate::control::save_png(&crop, &path).map_err(|e| format!("cannot save the shot: {e:#}"))?;
        Ok(json!({"path":path}))
    }

    /// Where canvas.pane_shot saves: `path` (absolute, .png), else <temp>/canopy/<pane>.png.
    fn shot_path(&self, a: &Value) -> Result<std::path::PathBuf, String> {
        if let Some(value) = a.get("path") {
            return value.as_str().map(std::path::PathBuf::from).filter(|p| p.is_absolute() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")))
                .ok_or_else(|| "canvas.pane_shot: path must be an absolute .png path".into());
        }
        let path = std::env::temp_dir().join("canopy").join(format!("{}.png", a["pane"].as_str().unwrap_or_default().replace(':', "-")));
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| format!("cannot create the shot directory: {e}"))?;
        Ok(path)
    }

    fn pane_open(&mut self, a: &Value, window: &mut Window, cx: &mut Context<Self>) -> Result<Value, String> {
        let Some(id) = a["pane"].as_str() else { return Err("canvas.pane_open needs pane".into()); };
        let pane = self.canvas_state.doc.panes.iter().find(|p| p.id == id).ok_or_else(|| format!("no pane {id}"))?;
        if pane.sim().is_none() { return Err(format!("pane {id} ({}) has no verb open", pane.kind_name())); }
        // A sim pane shows the simulator (a udid or "booted"), booting it; its own udid refreshes.
        let target = a["target"].as_str().ok_or("canvas.pane_open needs target")?;
        let udid = super::sim::resolve(target)?;
        self.point_sim(id, udid.clone(), window, cx);
        Ok(json!({"pane":id,"udid":udid}))
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
            assert_eq!(this.pane_new(&json!({"kind":"mod:nope"}), window, cx).unwrap_err(), format!("canvas.pane_new: kind must be one of: file, browser, {}mod:note", if crate::sim::available() { "sim, " } else { "" }));
            let reply = this.pane_new(&json!({"kind":"mod:note","wire":"t1"}), window, cx).unwrap();
            assert_eq!(reply, json!({"pane":"p1","kind":"mod:note","wired":true}));
            assert_eq!(crate::wire::neighbors(&this.canvas_state.doc.wires, "t1"), ["p1"]);
            assert_eq!(this.pane_read(&json!({"pane":"p1"}), cx).unwrap()["text"], canvas::envelope("p1", "mod:note", &[], ""));
            assert!(this.pane_set(&json!({"pane":"p1","props":{"text":"hello"}}), cx).is_ok());
            assert_eq!(this.pane_read(&json!({"pane":"p1"}), cx).unwrap()["text"], canvas::envelope("p1", "mod:note", &[], "hello"));
            assert_eq!(this.pane_set(&json!({"pane":"p1","props":3}), cx).unwrap_err(), "canvas.pane_set: props must be an object");
            assert_eq!(this.pane_set(&json!({"pane":"t1","props":{}}), cx).unwrap_err(), "pane t1 is a terminal: only mod panes take canvas.pane_set");
            assert_eq!(this.pane_set(&json!({"pane":"p1","props":{"text":"x".repeat(1024 * 1024 + 1)}}), cx).unwrap_err(), "canvas.pane_set: props over 1 MiB");
            assert_eq!(this.pane_read(&json!({"pane":"p1"}), cx).unwrap()["text"], canvas::envelope("p1", "mod:note", &[], "hello"));
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
            // A sim pane before simctl answers: read says what it knows; open needs a target.
            this.canvas_state.doc.panes.push(canvas::Pane { id: "p9".into(), kind: canvas::Kind::Sim { udid: String::new() }, rect: [0., 0., 320., 720.], z: 9, space: None, pinned: None });
            assert_eq!(this.pane_read(&json!({"pane":"p9"}), cx).unwrap()["text"], canvas::envelope("p9", "sim", &[("udid", "")], "No simulator picked yet."));
            assert_eq!(this.pane_open(&json!({"pane":"p9"}), window, cx).unwrap_err(), "canvas.pane_open needs target");
        });
    }

    #[gpui::test]
    fn pane_commands_headless(cx: &mut TestAppContext) {
        if !running_alone(TEST) { return; }
        let (view, cx) = open_window(cx, "pane-commands", "", json!({"id":"root","type":"canvas"}));
        view.update_in(cx, |this, window, cx| {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md");
            this.canvas_state.doc.place("t1", [0., 0.], [220., 36.]);
            assert_eq!(this.pane_new(&json!({"kind":"video","path":path}), window, cx).unwrap_err(), format!("canvas.pane_new: kind must be one of: file, browser, {}mod:note", if crate::sim::available() { "sim, " } else { "" }));
            assert_eq!(this.pane_new(&json!({"kind":"browser"}), window, cx).unwrap_err(), "canvas.pane_new: a browser pane needs url");
            assert_eq!(this.pane_new(&json!({"kind":"browser","url":"file:///etc/passwd"}), window, cx).unwrap_err(), "only http and https pages open in a browser pane, not file:");
            let web = this.pane_new(&json!({"kind":"browser","url":"localhost:3000","wire":"t1"}), window, cx).unwrap();
            assert_eq!(web, json!({"pane":"p1","kind":"browser","wired":true,"url":"http://localhost:3000"}));
            assert_eq!(this.canvas_state.doc.panes.iter().find(|p| p.id == "p1").unwrap().kind, canvas::Kind::Browser { url: "http://localhost:3000".into(), home: "http://localhost:3000".into() });
            // Pinned: it opens in the top right corner, and nothing on the canvas moves for it.
            let rects = |this: &crate::window::AppWindow| this.canvas_state.doc.panes.iter().filter(|p| p.pinned.is_none()).map(|p| (p.id.clone(), p.rect)).collect::<Vec<_>>();
            let before = rects(this);
            let pinned = this.pane_new(&json!({"kind":"browser","url":"localhost:3000","pinned":true}), window, cx).unwrap();
            let id = pinned["pane"].as_str().unwrap().to_owned();
            assert_eq!(pinned["pinned"], json!(true));
            assert_eq!(rects(this), before);
            let view = this.bounds.borrow()["canopy-canvas"].size;
            let s = this.canvas_state.doc.panes.iter().find(|p| p.id == id).unwrap().pinned.unwrap();
            assert_eq!((s[0] + s[2], s[1]), (f32::from(view.width) - 16., 16.), "flush with the top right margin");
            // Toggling moves it onto the canvas right where it showed, then pins it again there.
            let shown = this.canvas_state.doc.panes.iter().find(|p| p.id == id).unwrap().rect;
            assert_eq!(this.pin_pane(&id, None, false, window, cx), Ok(false));
            let pane = this.canvas_state.doc.panes.iter().find(|p| p.id == id).unwrap();
            assert_eq!((pane.pinned, &pane.rect[..2]), (None, &shown[..2]));
            assert_eq!(this.pin_pane(&id, None, false, window, cx), Ok(true));
            this.canvas_state.doc.panes.retain(|p| p.id != id);
            assert!(this.browser_reply("p1", None, window, cx).unwrap_err().contains("canopy pane"), "a read with no caller to answer refuses");
            this.close_canvas_cell(&json!({"pane":"p1"}), window, cx);
            assert_eq!(this.pane_new(&json!({"kind":"file","path":"README.md"}), window, cx).unwrap_err(), "canvas.pane_new: path must be absolute");
            assert_eq!(this.pane_new(&json!({"kind":"file","path":path,"point":null}), window, cx).unwrap_err(), "invalid canvas point");
            assert_eq!(this.pane_new(&json!({"kind":"file","path":path,"wire":"missing"}), window, cx).unwrap_err(), "missing is not a terminal pane on the canvas");
            let reply = this.pane_new(&json!({"kind":"file","path":path,"wire":"t1"}), window, cx).unwrap();
            let id = reply["pane"].as_str().unwrap();
            assert_eq!(reply["wired"], true);
            assert_eq!(crate::wire::neighbors(&this.canvas_state.doc.wires, "t1"), [id]);
            let read = this.pane_read(&json!({"pane":id}), cx).unwrap();
            assert_eq!(read["text"], canvas::envelope(id, "file", &[("path", reply["path"].as_str().unwrap())], &canvas::read_preview(&path).unwrap()));
            assert_eq!(this.pane_read(&json!({"pane":"missing"}), cx).unwrap_err(), "no pane missing");
            assert_eq!(this.pane_read(&json!({"pane":"t1"}), cx).unwrap_err(), "pane t1 is a terminal: read it with hc read t1");
            assert_eq!(this.pane_open(&json!({"pane":id,"target":"x"}), window, cx).unwrap_err(), format!("pane {id} (file) has no verb open"));
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
    const VIEWERS_TEST: &str = "window::canvas_view::pane_commands::tests::viewer_pane_commands_headless";

    #[test]
    fn viewer_pane_commands() { in_own_process(VIEWERS_TEST); }

    #[gpui::test]
    fn viewer_pane_commands_headless(cx: &mut TestAppContext) {
        if !running_alone(VIEWERS_TEST) { return; }
        let (view, cx) = open_window(cx, "viewer-panes", "", json!({"id":"root","type":"canvas"}));
        let dir = crate::mods::app_dir();
        let image = dir.join("viewer.PNG");
        let pdf = dir.join("viewer.pdf");
        let bytes = sample_pdf();
        image::RgbaImage::new(10, 6).save(&image).unwrap();
        std::fs::write(&pdf, &bytes).unwrap();
        view.update_in(cx, |this, window, cx| {
            let png = this.run_as("canvas.pane_new", json!({"kind":"file","path":image}), false, window, cx);
            assert_eq!(png["kind"], "image", "{png}");
            let read = this.pane_read(&json!({"pane":png["pane"]}), cx).unwrap();
            assert!(read["text"].as_str().unwrap().contains("kind=\"image\""));
            assert!(read["text"].as_str().unwrap().contains("size=\"10x6\""));
            // A PNG saved as .jpg still opens: the size is read from the bytes.
            let lie = dir.join("lie.jpg");
            image::RgbaImage::new(3, 2).save_with_format(&lie, image::ImageFormat::Png).unwrap();
            assert_eq!(this.run_as("canvas.pane_new", json!({"kind":"file","path":lie}), false, window, cx)["kind"], "image");

            let reply = this.run_as("canvas.pane_new", json!({"kind":"file","path":pdf}), false, window, cx);
            assert_eq!(reply["kind"], "pdf", "{reply}");
            assert_eq!(std::fs::read(&pdf).unwrap(), bytes);
            let id = reply["pane"].as_str().unwrap();
            let read = this.pane_read(&json!({"pane":id}), cx).unwrap();
            let text = read["text"].as_str().unwrap();
            assert!(text.contains("pages=\"1\""), "{text}");
            assert!(text.contains("[page 1]\nHello canvas"), "{text}");
            assert_eq!(std::fs::read(&pdf).unwrap(), bytes);
            let mark = canvas::PdfMark { page: 1, quote: "Hello canvas".into(), occurrence: 0 };
            this.add_pdf_mark(id, mark.clone(), point(px(10.), px(10.)), cx);
            let pane = this.canvas_state.doc.panes.iter().find(|p| p.id == id).unwrap();
            assert!(matches!(&pane.kind, canvas::Kind::Pdf { highlights, .. } if highlights == &[mark]));
            assert_eq!(std::fs::read(&pdf).unwrap(), bytes);
            let quote = &this.canvas_state.quote_chip.as_ref().unwrap().0;
            assert_eq!(quote["page"], 1);
            assert_eq!(quote["text"], "Hello canvas");
            let mut send = quote.clone();
            send.as_object_mut().unwrap().remove("point");
            crate::commands::validate_args(crate::commands::find("quote.send").unwrap(), &send).unwrap();
            // Clear removes the saved marks of the selected pane and never touches the file.
            this.canvas_state.selected = vec![id.to_owned()];
            this.pdf_command("canvas.pdf_clear", window, cx);
            assert!(this.canvas_state.doc.panes.iter().any(|p| p.id == id && matches!(&p.kind, canvas::Kind::Pdf { highlights, .. } if highlights.is_empty())));
            assert_eq!(std::fs::read(&pdf).unwrap(), bytes);
            this.close_canvas_cell(&json!({"pane":id}), window, cx);
            assert!(this.canvas_state.pdfs.is_empty());
            assert_eq!(this.canvas_state.pdf_drops.len(), 1);
            this.release_pdf_drops(window, cx);
        });
    }

    fn sample_pdf() -> Vec<u8> {
        use std::fmt::Write;
        let text = "BT /F1 12 Tf 10 40 Td (Hello canvas) Tj ET\n";
        let mut pdf = "%PDF-1.4\n".to_owned();
        let mut offsets = Vec::new();
        for (i, object) in [
            "<< /Type /Catalog /Pages 2 0 R >>".into(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".into(),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".into(),
            format!("<< /Length {} >>\nstream\n{text}endstream", text.len()),
        ].iter().enumerate() {
            offsets.push(pdf.len());
            writeln!(pdf, "{} 0 obj\n{object}\nendobj", i + 1).unwrap();
        }
        let xref = pdf.len();
        let size = offsets.len() + 1;
        writeln!(pdf, "xref\n0 {size}\n0000000000 65535 f ").unwrap();
        for offset in offsets {
            writeln!(pdf, "{offset:010} 00000 n ").unwrap();
        }
        writeln!(pdf, "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF").unwrap();
        pdf.into_bytes()
    }

}
