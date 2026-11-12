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
        self.zoom = snap(zoom);
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
pub fn snap(z: f32) -> f32 {
    ZOOMS
        .into_iter()
        .min_by(|a, b| (a - z).abs().total_cmp(&(b - z).abs()))
        .unwrap()
}
pub fn intersects(a: Rect, b: Rect) -> bool {
    a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
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
    pub width: Option<f32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub version: u32,
    pub camera: Camera,
    pub view: View,
    pub spaces: Vec<Space>,
    pub cards: Vec<Card>,
    pub panel: Panel,
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
                && doc.spaces.iter().all(|s| valid_rect(s.rect))
                && doc.panel.width.is_none_or(|w| w.is_finite() && w > 0.)
            {
                doc.camera.zoom = snap(doc.camera.zoom);
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
