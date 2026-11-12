// Modified for Canopy.
//! Canvas geometry and document storage, independent of the window toolkit.
use serde::{Deserialize, Serialize};
use std::{io, path::Path};

#[derive(Default)]
pub struct SaveSchedule {
    pub dirty: bool,
    pub pending: bool,
}
impl SaveSchedule {
    pub fn changed(&mut self) -> bool {
        self.dirty = true;
        if self.pending {
            return false;
        }
        self.pending = true;
        true
    }
}

pub const ZOOMS: [f32; 12] = [
    0.25, 0.33, 0.5, 0.67, 0.75, 0.9, 1., 1.1, 1.25, 1.5, 1.75, 2.,
];
pub type Point = [f32; 2];
pub type Rect = [f32; 4];

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            x: 0.,
            y: 0.,
            zoom: 1.,
        }
    }
}
impl Camera {
    pub fn to_screen(self, p: Point) -> Point {
        [(p[0] - self.x) * self.zoom, (p[1] - self.y) * self.zoom]
    }
    pub fn to_world(self, p: Point) -> Point {
        [p[0] / self.zoom + self.x, p[1] / self.zoom + self.y]
    }
    pub fn pan(&mut self, d: Point) {
        self.x -= d[0] / self.zoom;
        self.y -= d[1] / self.zoom;
    }
    pub fn zoom_at(&mut self, zoom: f32, anchor: Point) {
        let p = self.to_world(anchor);
        self.zoom = zoom;
        self.x = p[0] - anchor[0] / self.zoom;
        self.y = p[1] - anchor[1] / self.zoom;
    }
    pub fn step(&mut self, direction: i32, anchor: Point) {
        let i = ZOOMS
            .iter()
            .position(|z| *z == snap(self.zoom))
            .unwrap_or(6);
        self.zoom_at(
            ZOOMS[(i as i32 + direction.signum()).clamp(0, 11) as usize],
            anchor,
        );
    }
    pub fn fit(&mut self, rects: &[Rect], viewport: Point) {
        let Some(bounds) = union(rects) else { return };
        let z = ((viewport[0] - 48.).max(1.) / bounds[2].max(1.))
            .min((viewport[1] - 48.).max(1.) / bounds[3].max(1.));
        self.zoom = ZOOMS
            .into_iter()
            .rev()
            .find(|v| *v <= z)
            .unwrap_or(ZOOMS[0]);
        self.x = bounds[0] + bounds[2] / 2. - viewport[0] / self.zoom / 2.;
        self.y = bounds[1] + bounds[3] / 2. - viewport[1] / self.zoom / 2.;
    }
}
pub fn zoom_factor(delta: f32) -> f32 { delta.clamp(-10., 10.).exp() }
pub fn ease_out(t: f32) -> f32 { 1. - (1. - t.clamp(0., 1.)).powi(3) }
pub fn friction(dt: f32) -> f32 { 0.92_f32.powf(dt / 0.016) }
pub fn rubber_band(z: f32) -> f32 {
    let limit = z.clamp(ZOOMS[0], ZOOMS[11]);
    let d = z - limit;
    limit + d / (1. + d.abs() / (limit * 0.08))
}
pub fn remember_folder(recent: &mut Vec<String>, folder: &str) {
    if folder.is_empty() { return; }
    recent.retain(|p| p != folder);
    recent.insert(0, folder.into());
    recent.truncate(10);
}

pub fn snap(z: f32) -> f32 {
    ZOOMS
        .into_iter()
        .min_by(|a, b| (a - z).abs().total_cmp(&(b - z).abs()))
        .unwrap()
}
pub fn intersects(a: Rect, b: Rect) -> bool {
    a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
}
/// The topmost space whose rect holds a world point (later spaces draw on top).
pub fn space_at(spaces: &[Space], p: Point) -> Option<&Space> {
    spaces.iter().rev().find(|s| p[0] >= s.rect[0] && p[1] >= s.rect[1] && p[0] <= s.rect[0] + s.rect[2] && p[1] <= s.rect[1] + s.rect[3])
}
pub fn clamp(mut r: Rect, space: Rect) -> Rect {
    r[0] = r[0].clamp(
        space[0] + 24.,
        (space[0] + space[2] - 24. - r[2]).max(space[0] + 24.),
    );
    r[1] = r[1].clamp(
        space[1] + 24.,
        (space[1] + space[3] - 24. - r[3]).max(space[1] + 24.),
    );
    r
}
pub fn union(rects: &[Rect]) -> Option<Rect> {
    let mut out = *rects.first()?;
    for r in &rects[1..] {
        let x = out[0].min(r[0]);
        let y = out[1].min(r[1]);
        out = [
            x,
            y,
            (out[0] + out[2]).max(r[0] + r[2]) - x,
            (out[1] + out[3]).max(r[1] + r[3]) - y,
        ];
    }
    Some(out)
}
pub fn min_size(space: Rect, cards: &[Rect]) -> Point {
    let b = union(cards).unwrap_or([space[0], space[1], 0., 0.]);
    [
        (b[0] + b[2] - space[0] + 24.).max(48.),
        (b[1] + b[3] - space[1] + 24.).max(48.),
    ]
}
/// Where a space made outside the canvas goes: right of every existing space.
pub fn new_space_rect(spaces: &[Space]) -> Rect {
    let rects: Vec<Rect> = spaces.iter().map(|s| s.rect).collect();
    let x = union(&rects).map_or(0., |u| u[0] + u[2] + 96.);
    [x, 0., 960., 640.]
}
pub fn child_position(parent: Rect, siblings: &[Rect], size: Point) -> Rect {
    [
        parent[0] + parent[2] + 80.,
        siblings.last().map_or(parent[1], |s| s[1] + s[3] + 16.),
        size[0],
        size[1],
    ]
}
pub fn free_spot(space: Rect, occupied: &[Rect], size: Point) -> Rect {
    let columns = ((space[2] - 48.) / (size[0] + 48.)).floor().max(1.) as usize;
    let rows = occupied
        .iter()
        .map(|r| ((r[1] + r[3] - space[1]).max(0.) / (size[1] + 48.)).ceil() as usize)
        .max()
        .unwrap_or(0)
        + 2;
    for i in 0..rows * columns {
        let r = [
            space[0] + 24. + (i % columns) as f32 * (size[0] + 48.),
            space[1] + 24. + (i / columns) as f32 * (size[1] + 48.),
            size[0],
            size[1],
        ];
        if !occupied.iter().any(|other| intersects(r, *other)) {
            return r;
        }
    }
    unreachable!()
}
pub fn edge_ends(parent: Rect, child: Rect) -> (Point, Point) {
    if child[0] < parent[0] {
        (
            [parent[0], parent[1] + parent[3] / 2.],
            [child[0] + child[2], child[1] + child[3] / 2.],
        )
    } else {
        (
            [parent[0] + parent[2], parent[1] + parent[3] / 2.],
            [child[0], child[1] + child[3] / 2.],
        )
    }
}
pub fn visible(camera: Camera, rect: Rect, viewport: Point) -> bool {
    let p = camera.to_world([0., 0.]);
    intersects(
        rect,
        [
            p[0] - 320.,
            p[1] - 36.,
            viewport[0] / camera.zoom + 640.,
            viewport[1] / camera.zoom + 72.,
        ],
    )
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum View {
    Agents,
    #[default]
    All,
    Terminals,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Space {
    pub space: String,
    pub rect: Rect,
    pub color: String,
    pub frame: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Card {
    pub terminal: String,
    pub rect: Rect,
    pub z: u64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Panel {
    #[serde(default)]
    pub tree_hidden: bool,
    /// The space last clicked on the canvas: the panel's file tree shows its folder.
    #[serde(default)]
    pub space: Option<String>,
    pub width: Option<f32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preview {
    pub path: String,
    pub rect: Rect,
}

pub const PREVIEW_LIMIT: u64 = 2 * 1024 * 1024;
pub fn preview_text(bytes: Vec<u8>) -> io::Result<String> {
    if bytes.len() as u64 > PREVIEW_LIMIT
        || bytes
            .iter()
            .any(|b| *b == 0 || (*b < 32 && !matches!(*b, 9 | 10 | 13)))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "not previewable text",
        ));
    }
    String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
pub fn read_preview(path: &Path) -> io::Result<String> {
    use std::io::Read;
    if !std::fs::metadata(path)?.is_file() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a file"));
    }
    let file = std::fs::File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a file"));
    }
    let mut bytes = Vec::new();
    file.take(PREVIEW_LIMIT + 1).read_to_end(&mut bytes)?;
    preview_text(bytes)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub version: u32,
    pub camera: Camera,
    pub view: View,
    pub spaces: Vec<Space>,
    pub cards: Vec<Card>,
    pub panel: Panel,
    #[serde(default)]
    pub previews: Vec<Preview>,
    #[serde(default)]
    pub recent_folders: Vec<String>,
}
impl Default for Document {
    fn default() -> Self {
        Self {
            version: 1,
            camera: Camera::default(),
            view: View::All,
            spaces: vec![],
            cards: vec![],
            panel: Panel::default(),
            previews: vec![],
            recent_folders: vec![],
        }
    }
}
impl Document {
    pub fn place(&mut self, terminal: &str, point: Point, size: Point) {
        let z = self
            .cards
            .iter()
            .map(|c| c.z)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        if let Some(card) = self.cards.iter_mut().find(|c| c.terminal == terminal) {
            card.rect[0] = point[0];
            card.rect[1] = point[1];
        } else {
            self.cards.push(Card {
                terminal: terminal.into(),
                rect: [point[0], point[1], size[0], size[1]],
                z,
            });
        }
    }
    pub fn load(path: &Path) -> io::Result<(Self, bool)> {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok((Self::default(), false)),
            Err(e) => return Err(e),
        };
        let parsed = serde_json::from_slice::<Self>(&bytes);
        if let Ok(mut doc) = parsed {
            let valid_rect = |r: Rect| r.iter().all(|n| n.is_finite()) && r[2] > 0. && r[3] > 0.;
            if doc.version != 1 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "unsupported canvas version",
                ));
            }
            if doc.camera.x.is_finite()
                && doc.camera.y.is_finite()
                && doc.camera.zoom.is_finite()
                && doc.cards.iter().all(|c| valid_rect(c.rect))
                && doc.previews.iter().all(|p| valid_rect(p.rect))
                && doc.spaces.iter().all(|s| valid_rect(s.rect))
                && doc.panel.width.is_none_or(|w| w.is_finite() && w > 0.)
            {
                doc.camera.zoom = doc.camera.zoom.clamp(ZOOMS[0], ZOOMS[11]);
                return Ok((doc, false));
            }
        }
        // Preserve earlier recovery files too; never overwrite somebody's saved geometry.
        let mut bad = path.with_extension("json.bad");
        let mut n = 1;
        while bad.exists() {
            bad = path.with_extension(format!("json.bad.{n}"));
            n += 1;
        }
        std::fs::rename(path, bad)?;
        Ok((Self::default(), true))
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        crate::persist::write_atomic(path, serde_json::to_vec_pretty(self)?)
    }
}

pub fn edge_visible(camera: Camera, (a, b): (Point, Point), viewport: Point) -> bool {
    visible(
        camera,
        [
            a[0].min(b[0]) - 3.,
            a[1].min(b[1]) - 3.,
            (a[0] - b[0]).abs() + 6.,
            (a[1] - b[1]).abs() + 6.,
        ],
        viewport,
    )
}
pub const COLORS: [&str; 8] = [
    "gray", "red", "orange", "yellow", "green", "teal", "blue", "purple",
];

pub fn unused_color(spaces: &[Space], seed: usize) -> &'static str {
    let unused: Vec<_> = COLORS
        .into_iter()
        .filter(|c| !spaces.iter().any(|s| s.color == *c))
        .collect();
    if unused.is_empty() {
        COLORS[seed % COLORS.len()]
    } else {
        unused[seed % unused.len()]
    }
}

pub fn move_space(space: &mut Space, cards: &mut [Card], members: &[String], delta: Point) {
    space.rect[0] += delta[0];
    space.rect[1] += delta[1];
    for card in cards.iter_mut().filter(|c| members.contains(&c.terminal)) {
        card.rect[0] += delta[0];
        card.rect[1] += delta[1];
    }
}

pub fn clear_hidden_focus(view: &View, focus: &mut Option<String>, agent: bool) {
    if !view_allows(view, agent) {
        *focus = None;
    }
}

pub fn view_allows(view: &View, agent: bool) -> bool {
    match view {
        View::All => true,
        View::Agents => agent,
        View::Terminals => !agent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn space_at_picks_the_topmost_space_holding_the_point() {
        let space = |id: &str, rect| Space { space: id.into(), rect, color: "red".into(), frame: true };
        let spaces = [space("a", [0., 0., 100., 100.]), space("b", [50., 50., 100., 100.])];
        assert_eq!(space_at(&spaces, [10., 10.]).unwrap().space, "a");
        assert_eq!(space_at(&spaces, [60., 60.]).unwrap().space, "b");
        assert!(space_at(&spaces, [200., 10.]).is_none());
    }
    #[test]
    fn preview_rejects_binary_large_and_invalid_utf8() {
        assert!(preview_text(b"hello\n".to_vec()).is_ok());
        for bytes in [vec![0], vec![255], vec![b'a'; PREVIEW_LIMIT as usize + 1]] {
            assert!(preview_text(bytes).is_err());
        }
    }
    #[test]
    fn document_loads_old_previews_and_validates_rects() {
        let dir = std::env::temp_dir().join(format!("canopy-previews-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("canvas.json");
        let mut value = serde_json::to_value(Document::default()).unwrap();
        value.as_object_mut().unwrap().remove("previews");
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(Document::load(&path).unwrap().0.previews.is_empty());
        value["previews"] = serde_json::json!([{"path":"/missing.md", "rect":[1,2,400,300]}]);
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(
            Document::load(&path).unwrap().0.previews[0].path,
            "/missing.md"
        );
        value["previews"][0]["rect"][2] = serde_json::json!(-1);
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(Document::load(&path).unwrap().1);
        assert!(path.with_extension("json.bad").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn new_space_never_lands_on_an_existing_one() {
        let space = |x| Space {
            space: "s".into(),
            rect: [x, 0., 960., 640.],
            color: "blue".into(),
            frame: true,
        };
        assert_eq!(new_space_rect(&[])[0], 0.);
        // After deleting the first of two, a count-based x would reuse 1056.
        let r = new_space_rect(&[space(1056.)]);
        assert!(!intersects(r, space(1056.).rect));
    }
    #[test]
    fn space_drag_moves_only_its_cards() {
        let mut space = Space {
            space: "s".into(),
            rect: [0., 0., 960., 640.],
            color: "blue".into(),
            frame: true,
        };
        let mut cards = vec![
            Card {
                terminal: "a".into(),
                rect: [24., 24., 160., 36.],
                z: 0,
            },
            Card {
                terminal: "b".into(),
                rect: [1., 2., 160., 36.],
                z: 0,
            },
        ];
        move_space(&mut space, &mut cards, &["a".into()], [10., -5.]);
        assert_eq!(space.rect, [10., -5., 960., 640.]);
        assert_eq!(cards[0].rect, [34., 19., 160., 36.]);
        assert_eq!(cards[1].rect, [1., 2., 160., 36.]);
    }
    #[test]
    fn new_card_clamps_at_click_point() {
        assert_eq!(
            clamp([900., 620., 240., 36.], [0., 0., 960., 640.]),
            [696., 580., 240., 36.]
        );
    }
    #[test]
    fn random_color_uses_unused_palette_entries() {
        let spaces: Vec<_> = COLORS[..7]
            .iter()
            .map(|c| Space {
                space: c.to_string(),
                rect: [0., 0., 960., 640.],
                color: c.to_string(),
                frame: true,
            })
            .collect();
        for seed in 0..32 {
            assert_eq!(unused_color(&spaces, seed), "purple");
        }
    }
    #[test]
    fn hiding_focused_card_clears_focus() {
        let mut focus = Some("shell".into());
        clear_hidden_focus(&View::Agents, &mut focus, false);
        assert_eq!(focus, None);
    }
    #[test]
    fn changes_share_one_pending_save() {
        let mut save = SaveSchedule::default();
        assert!(save.changed());
        assert!(!save.changed());
        save.dirty = false;
        assert!(!save.changed());
        assert!(save.dirty);
        save.pending = false;
        assert!(save.changed());
    }
    #[test]
    fn camera_round_trip() {
        let c = Camera {
            x: -200.,
            y: 32.,
            zoom: 0.5,
        };
        assert_eq!(c.to_world(c.to_screen([78., 143.])), [78., 143.]);
    }
    #[test]
    fn zoom_keeps_cursor_anchor() {
        let mut c = Camera::default();
        let anchor = [121., 432.];
        let world = c.to_world(anchor);
        c.zoom_at(1.25, anchor);
        assert_eq!(c.to_world(anchor), world);
    }
    #[test]
    fn zoom_step_snaps() {
        let mut c = Camera {
            zoom: 0.52,
            ..Camera::default()
        };
        c.step(1, [0., 0.]);
        assert_eq!(c.zoom, 0.67);
    }
    #[test]
    fn card_clamps_into_space() {
        assert_eq!(
            clamp([-100., 900., 160., 36.], [0., 0., 960., 640.]),
            [24., 580., 160., 36.]
        );
    }
    #[test]
    fn space_minimum_contains_cards() {
        assert_eq!(
            min_size([0., 0., 960., 640.], &[[24., 24., 160., 36.]]),
            [208., 84.]
        );
    }
    #[test]
    fn child_is_beside_parent() {
        assert_eq!(
            child_position([24., 24., 160., 36.], &[], [160., 36.]),
            [264., 24., 160., 36.]
        );
    }
    #[test]
    fn siblings_stack() {
        assert_eq!(
            child_position(
                [24., 24., 160., 36.],
                &[[264., 24., 160., 36.]],
                [160., 36.]
            ),
            [264., 76., 160., 36.]
        );
    }
    #[test]
    fn edge_joins_right_child() {
        assert_eq!(
            edge_ends([24., 24., 160., 36.], [264., 24., 160., 36.]),
            ([184., 42.], [264., 42.])
        );
    }
    #[test]
    fn edge_mirrors_for_left_child() {
        assert_eq!(
            edge_ends([264., 24., 160., 36.], [24., 24., 160., 36.]),
            ([264., 42.], [184., 42.])
        );
    }
    #[test]
    fn free_spot_skips_occupied() {
        assert_eq!(
            free_spot([0., 0., 960., 640.], &[[24., 24., 160., 36.]], [160., 36.]),
            [232., 24., 160., 36.]
        );
    }
    #[test]
    fn offscreen_card_is_culled() {
        assert!(!visible(
            Camera::default(),
            [2000., 0., 160., 36.],
            [800., 600.]
        ));
    }
    #[test]
    fn crossing_curve_is_kept() {
        assert!(edge_visible(
            Camera::default(),
            ([-1000., 200.], [2000., 200.]),
            [800., 600.]
        ));
    }
    #[test]
    fn hidden_edge_end_fails_view_filter() {
        assert!(!(view_allows(&View::Agents, true) && view_allows(&View::Agents, false)));
    }
    #[test]
    fn view_filter_selects_cards() {
        assert!(view_allows(&View::All, false));
        assert!(view_allows(&View::Terminals, false));
        assert!(!view_allows(&View::Terminals, true));
    }
    #[test]
    fn fit_uses_largest_fitting_zoom_step() {
        let mut c = Camera::default();
        c.fit(&[[0., 0., 100., 100.]], [800., 600.]);
        assert_eq!(c.zoom, 2.);
    }
    #[test]
    fn space_minimum_without_cards() {
        assert_eq!(min_size([0., 0., 960., 640.], &[]), [48., 48.]);
    }
    #[test]
    fn valid_document_round_trips() {
        let path =
            std::env::temp_dir().join(format!("canopy-roundtrip-{}.json", std::process::id()));
        let doc = Document {
            view: View::Agents,
            ..Document::default()
        };
        doc.save(&path).unwrap();
        let (loaded, bad) = Document::load(&path).unwrap();
        assert!(!bad);
        assert_eq!(loaded.view, View::Agents);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn unsupported_version_is_untouched() {
        let path = std::env::temp_dir().join(format!("canopy-version-{}.json", std::process::id()));
        Document {
            version: 2,
            ..Document::default()
        }
        .save(&path)
        .unwrap();
        let original = std::fs::read(&path).unwrap();
        assert!(Document::load(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn earlier_corrupt_backup_is_preserved() {
        let dir = std::env::temp_dir().join(format!("canopy-backup-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("canvas.json");
        std::fs::write(path.with_extension("json.bad"), "earlier").unwrap();
        std::fs::write(&path, "broken").unwrap();
        assert!(Document::load(&path).unwrap().1);
        assert_eq!(
            std::fs::read(path.with_extension("json.bad")).unwrap(),
            b"earlier"
        );
        assert_eq!(
            std::fs::read(path.with_extension("json.bad.1")).unwrap(),
            b"broken"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn corrupt_document_is_preserved() {
        let dir = std::env::temp_dir().join(format!("canopy-canvas-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("canvas.json");
        std::fs::write(&path, "broken").unwrap();
        assert!(Document::load(&path).unwrap().1);
        assert_eq!(
            std::fs::read(path.with_extension("json.bad")).unwrap(),
            b"broken"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}

pub fn open_panel_tab(tabs: &mut Vec<String>, item: &str) {
    if !tabs.iter().any(|t| t == item) {
        tabs.push(item.to_owned());
    }
}

pub fn close_panel_tab(tabs: &mut Vec<String>, active: Option<&str>, item: &str) -> Option<String> {
    tabs.retain(|t| t != item);
    active
        .filter(|a| *a != item && tabs.iter().any(|t| t == a))
        .map(str::to_owned)
        .or_else(|| tabs.last().cloned())
}

/// A path as "src / main.rs", relative to root when inside it; the root itself shows its name.
pub fn breadcrumb(path: &str, root: Option<&str>) -> String {
    let path = std::path::Path::new(path);
    let relative = root
        .and_then(|root| path.strip_prefix(root).ok())
        .filter(|p| !p.as_os_str().is_empty())
        .or_else(|| path.file_name().map(std::path::Path::new).filter(|_| root.is_some_and(|r| path == std::path::Path::new(r))))
        .unwrap_or(path);
    relative
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => Some(s.to_string_lossy()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" / ")
}

#[cfg(test)]
mod panel_tests {
    use super::*;
    #[test]
    fn tabs_activate_without_duplicates_and_close_to_neighbor() {
        let mut tabs = vec![];
        open_panel_tab(&mut tabs, "terminal");
        open_panel_tab(&mut tabs, "/tmp/a.rs");
        open_panel_tab(&mut tabs, "terminal");
        assert_eq!(tabs, ["terminal", "/tmp/a.rs"]);
        assert_eq!(
            close_panel_tab(&mut tabs, Some("/tmp/a.rs"), "terminal").as_deref(),
            Some("/tmp/a.rs")
        );
        assert_eq!(
            close_panel_tab(&mut tabs, Some("/tmp/a.rs"), "/tmp/a.rs"),
            None
        );
    }
    #[test]
    fn breadcrumbs_respect_directory_boundaries() {
        assert_eq!(breadcrumb("/repo", Some("/repo")), "repo");
        assert_eq!(
            breadcrumb("/repo/src/main.rs", Some("/repo")),
            "src / main.rs"
        );
        assert_eq!(
            breadcrumb("/repository/a", Some("/repo")),
            "repository / a"
        );
    }
}

#[cfg(test)]
mod motion_tests {
    use super::*;
    #[test]
    fn motion_math() {
        assert!((zoom_factor(2_f32.ln()) - 2.).abs() < 0.0001);
        assert_eq!(ease_out(0.), 0.); assert_eq!(ease_out(1.), 1.);
        assert!((friction(0.032) - friction(0.016).powi(2)).abs() < 0.0001);
        assert_eq!(rubber_band(1.), 1.);
        assert!(rubber_band(100.) > 2. && rubber_band(100.) < 2.16);
        assert!(rubber_band(0.01) > 0.23);
        let mut c = Camera::default(); let anchor = [142., 91.];
        let world = c.to_world(anchor); c.zoom_at(1.037, anchor);
        assert_eq!(c.to_world(anchor), world);
    }
    #[test]
    fn recent_folders_are_bounded_and_unique() {
        let mut recent = vec![];
        for i in 0..12 { remember_folder(&mut recent, &format!("/{i}")); }
        remember_folder(&mut recent, "/5"); remember_folder(&mut recent, "");
        assert_eq!(recent.len(), 10); assert_eq!(recent[0], "/5");
        assert_eq!(recent.iter().filter(|p| *p == "/5").count(), 1);
    }
}
