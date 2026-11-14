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

/// The shared width of terminal and sub-agent cells at zoom 1.
pub const PILL_WIDTH: f32 = 220.;
/// A space's card on Home at zoom 1.
pub const SPACE_CARD: Point = [260., 120.];
/// The header strip of an expanded cell at zoom 1: the pill's one-line height.
pub const HEADER: f32 = 36.;
/// Below this camera zoom an expanded cell shows a flat stand-in instead of live text.
pub const EXPANDED_MIN_ZOOM: f32 = 0.5;

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
/// Screen px a pressed cell must move before the press is a drag; less is a click.
pub const DRAG_THRESHOLD: f32 = 3.;
/// A tile smaller than this (world px) shows its icon and name instead of terminal text.
pub const TILE_COMPACT: Point = [320., 140.];
/// The active tile's share of its row, and (at least) its row's share of the height.
const TILE_ACTIVE: f32 = 0.6;

/// The indices of `rects` in reading order: top to bottom, then left to right.
pub fn reading(rects: &[Rect]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..rects.len()).collect();
    order.sort_by(|a, b| (rects[*a][1], rects[*a][0]).partial_cmp(&(rects[*b][1], rects[*b][0])).unwrap_or(std::cmp::Ordering::Equal));
    order
}

/// Tile view: fills `area` with `tiles` tiles like tmux's tiled layout, and lines `pills`
/// (sizes) up along its bottom. Both are placed in the order given (the caller keeps it, so
/// nothing swaps when the active one changes or a rect jitters). The `active` tile's row is
/// taller and the tile wider, so it is always the largest; the rest share what is left.
/// Returns the tiles' and pills' rects in the order given.
pub fn tile(area: Rect, tiles: usize, active: Option<usize>, pills: &[Rect], gap: f32) -> (Vec<Rect>, Vec<Rect>) {
    let [x0, y0, w, h] = [area[0] + gap, area[1] + gap, (area[2] - 2. * gap).max(1.), (area[3] - 2. * gap).max(1.)];
    // Pills wrap into rows along the bottom; the tiles get what is above them.
    let mut placed = vec![[0.; 4]; pills.len()];
    let (mut x, mut row_h, mut rows) = (0., 0., vec![]);
    for i in 0..pills.len() {
        let [pw, ph] = [pills[i][2], pills[i][3]];
        if x > 0. && x + pw > w { rows.push(row_h); x = 0.; row_h = 0.; }
        placed[i] = [x, rows.iter().map(|r| r + gap).sum::<f32>(), pw, ph];
        x += pw + gap;
        row_h = f32::max(row_h, ph);
    }
    if !pills.is_empty() { rows.push(row_h); }
    let strip = rows.iter().sum::<f32>() + gap * rows.len() as f32;
    for r in &mut placed { r[0] += x0; r[1] += y0 + h - strip + gap; }
    if !pills.is_empty() && tiles == 0 { for r in &mut placed { r[1] -= h - strip + gap; } }
    let th = (h - strip).max(1.);
    // A run of `k` where the active one (if any) takes `big` of it and the others share the rest:
    // (offset, length) of item `i`, as fractions.
    let share = |k: usize, i: usize, active: Option<usize>, big: f32| {
        let size = |j: usize| match active { Some(a) if k > 1 => if j == a { big } else { (1. - big) / (k - 1) as f32 }, _ => 1. / k as f32 };
        ((0..i).map(size).sum::<f32>(), size(i))
    };
    let n = tiles;
    let mut out = vec![[0.; 4]; n];
    if n == 0 { return (out, placed); }
    // tmux's tiled grid: the last row is the short one and stretches.
    let cols = (n as f32).sqrt().ceil() as usize;
    let row_count = n.div_ceil(cols);
    let in_row = |r: usize| cols.min(n - r * cols);
    let at = active.map(|p| (p / cols, p % cols));
    // The active row is TILE_ACTIVE tall, or taller when a shorter row's stretched tiles would
    // otherwise outgrow the active tile.
    let tall = at.map_or(TILE_ACTIVE, |(r, _)| {
        let wide = if in_row(r) > 1 { TILE_ACTIVE } else { 1. };
        let fewest = (0..row_count).filter(|o| *o != r).map(in_row).min().unwrap_or(1) as f32;
        let needed = 1. / (1. + wide * (row_count - 1) as f32 * fewest);
        TILE_ACTIVE.max(needed + 0.05).min(0.9)
    });
    for p in 0..n {
        let (r, c) = (p / cols, p % cols);
        let k = in_row(r);
        let (ry, rh) = share(row_count, r, at.map(|a| a.0), tall);
        let (cx, cw) = share(k, c, at.filter(|a| a.0 == r).map(|a| a.1), TILE_ACTIVE);
        let (aw, ah) = (w - gap * (k - 1) as f32, th - gap * (row_count - 1) as f32);
        out[p] = [x0 + cx * aw + gap * c as f32, y0 + ry * ah + gap * r as f32, cw * aw, rh * ah];
    }
    (out, placed)
}

/// Toggle a space fit in the visible canvas: a second press restores the camera from before
/// the fit, unless the camera moved since, in which case it fits again.
pub fn focus_fit(camera: &mut Camera, previous: &mut Option<(Camera, Camera)>, rects: &[Rect], visible: Point) {
    match previous.take() {
        Some((before, fitted)) if *camera == fitted => *camera = before,
        _ if !rects.is_empty() => {
            let before = *camera;
            camera.fit(rects, visible);
            *previous = Some((before, *camera));
        }
        _ => {}
    }
}

/// The nearest centre in a strict half-plane; equal distances keep document order.
pub fn nearest(rects: &[Rect], from: Rect, direction: Point) -> Option<usize> {
    let center = [from[0] + from[2] / 2., from[1] + from[3] / 2.];
    rects.iter().enumerate().filter_map(|(i, r)| {
        let d = [r[0] + r[2] / 2. - center[0], r[1] + r[3] / 2. - center[1]];
        (d[0] * direction[0] + d[1] * direction[1] > 0.).then_some((i, d[0] * d[0] + d[1] * d[1]))
    }).min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0))).map(|(i, _)| i)
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
#[cfg(test)]
const GAP: f32 = 24.;

/// Resolve from rest positions, fixing the pinned rect first and then its nearest neighbours.
pub fn solve(rest: &[Rect], pinned: usize, rect: Rect, gap: f32) -> Vec<Rect> {
    if pinned >= rest.len() {
        return rest.to_vec();
    }
    let mut out = rest.to_vec();
    out[pinned] = rect;
    let mut order: Vec<_> = (0..rest.len()).filter(|i| *i != pinned).collect();
    let distance = |r: Rect| {
        (r[0] + r[2] / 2. - rect[0] - rect[2] / 2.).powi(2)
            + (r[1] + r[3] / 2. - rect[1] - rect[3] / 2.).powi(2)
    };
    order.sort_by(|a, b| {
        distance(rest[*a])
            .total_cmp(&distance(rest[*b]))
            .then(a.cmp(b))
    });
    let mut fixed = vec![pinned];
    for i in order {
        let r = rest[i];
        let clear = |r: Rect| fixed.iter().all(|j| separated(r, out[*j], gap));
        if !clear(r) {
            let mut best = None;
            let mut cost = f32::INFINITY;
            for j in &fixed {
                let b = out[*j];
                for (axis, value) in [
                    (0, b[0] - gap - r[2]),
                    (0, b[0] + b[2] + gap),
                    (1, b[1] - gap - r[3]),
                    (1, b[1] + b[3] + gap),
                ] {
                    let d = (value - r[axis]).abs();
                    if d < cost {
                        let mut candidate = r;
                        candidate[axis] = value;
                        if clear(candidate) {
                            best = Some(candidate);
                            cost = d;
                        }
                    }
                }
            }
            out[i] = best.expect("an outer obstacle boundary is always free");
        }
        fixed.push(i);
    }
    out
}
fn separated(a: Rect, b: Rect, gap: f32) -> bool {
    a[0] + a[2] + gap <= b[0]
        || b[0] + b[2] + gap <= a[0]
        || a[1] + a[3] + gap <= b[1]
        || b[1] + b[3] + gap <= a[1]
}


/// Arrange cells in reading order, or breadth-first parent layers, preserving their sizes.
/// Cells joined by `links` (wires, either way) come together, connected groups first.
pub fn arrange(rects: &[Rect], parents: &[Option<usize>], links: &[(usize, usize)], layout: &str) -> Vec<Rect> {
    let gap = crate::settings::get().canvas_gap;
    let Some(bounds) = union(rects) else {
        return vec![];
    };
    let mut order: Vec<_> = (0..rects.len()).collect();
    order.sort_by(|a, b| {
        rects[*a][1]
            .total_cmp(&rects[*b][1])
            .then(rects[*a][0].total_cmp(&rects[*b][0]))
            .then(a.cmp(b))
    });
    let order = connected_first(&order, links);
    let mut layers = vec![None; rects.len()];
    let mut queue = std::collections::VecDeque::new();
    if layout == "wires" {
        for i in &order {
            if parents
                .get(*i)
                .copied()
                .flatten()
                .is_none_or(|p| p >= rects.len())
            {
                layers[*i] = Some(0);
                queue.push_back(*i);
            }
        }
        // Cycles have no root: seed each remaining component once.
        for seed in &order {
            if queue.is_empty() && layers[*seed].is_none() {
                layers[*seed] = Some(0);
                queue.push_back(*seed);
            }
            while let Some(i) = queue.pop_front() {
                for j in &order {
                    if layers[*j].is_none() && parents.get(*j) == Some(&Some(i)) {
                        layers[*j] = Some(layers[i].unwrap() + 1);
                        queue.push_back(*j);
                    }
                }
            }
        }
    }
    let columns = match layout {
        "row" => rects.len(),
        "column" => 1,
        _ => (rects.len() as f32).sqrt().ceil() as usize,
    };
    let slots: Vec<_> = order
        .iter()
        .enumerate()
        .map(|(n, i)| {
            if layout == "wires" {
                (layers[*i].unwrap(), 0)
            } else {
                (n % columns, n / columns)
            }
        })
        .collect();
    let mut widths = vec![0_f32; rects.len()];
    for (i, (col, _)) in order.iter().zip(&slots) {
        widths[*col] = widths[*col].max(rects[*i][2]);
    }
    let mut out = rects.to_vec();
    let mut bottoms = vec![bounds[1]; rects.len()];
    let height = rects.iter().map(|r| r[3]).fold(0_f32, f32::max);
    for (i, (col, row)) in order.iter().zip(slots) {
        out[*i][0] = bounds[0] + widths[..col].iter().map(|w| w + gap).sum::<f32>();
        out[*i][1] = if layout == "wires" {
            bottoms[col]
        } else {
            bounds[1] + row as f32 * (height + gap)
        };
        bottoms[col] = out[*i][1] + rects[*i][3] + gap;
    }
    out
}

/// One layout snapshot; restoring ignores deleted objects and leaves new objects alone.
#[derive(Clone, Default, PartialEq)]
pub struct Rest {
    cards: Vec<(String, Rect)>,
    spaces: Vec<(String, Rect)>,
    previews: Vec<(String, Rect)>,
}
impl Rest {
    /// Capture only geometry, without terminal or panel state.
    pub fn capture(doc: &Document) -> Self {
        Self {
            cards: doc
                .cards
                .iter()
                .map(|c| (c.key(), c.rect))
                .collect(),
            spaces: doc
                .spaces
                .iter()
                .map(|s| (s.space.clone(), s.rect))
                .collect(),
            previews: doc
                .previews
                .iter()
                .map(|p| (p.path.clone(), p.rect))
                .collect(),
        }
    }
    /// The entries of `later` whose geometry differs from this snapshot's.
    pub fn changed(&self, later: &Rest) -> Rest {
        let diff = |before: &[(String, Rect)], after: &[(String, Rect)]| -> Vec<(String, Rect)> {
            after.iter().filter(|(id, r)| before.iter().any(|(b, was)| b == id && was != r)).cloned().collect()
        };
        Rest { cards: diff(&self.cards, &later.cards), spaces: diff(&self.spaces, &later.spaces), previews: diff(&self.previews, &later.previews) }
    }
    /// Put each object in `moved` back where this snapshot had it, unless it has been moved again since.
    pub fn put_back(&self, moved: &Rest, doc: &mut Document) {
        let back = |home: &[(String, Rect)], left: &[(String, Rect)], at: &mut Rect, id: &str| {
            if left.iter().any(|(l, r)| l == id && r == at) && let Some((_, r)) = home.iter().find(|(h, _)| h == id) {
                *at = *r;
            }
        };
        for c in &mut doc.cards { let key = c.key(); back(&self.cards, &moved.cards, &mut c.rect, &key); }
        for s in &mut doc.spaces { back(&self.spaces, &moved.spaces, &mut s.rect, &s.space); }
        for p in &mut doc.previews { back(&self.previews, &moved.previews, &mut p.rect, &p.path); }
    }
    /// Restore geometry of objects that still exist.
    pub fn restore(&self, doc: &mut Document) {
        for c in &mut doc.cards {
            if let Some((_, r)) = self.cards.iter().find(|(id, _)| *id == c.key()) {
                c.rect = *r;
            }
        }
        for s in &mut doc.spaces {
            if let Some((_, r)) = self.spaces.iter().find(|(id, _)| *id == s.space) {
                s.rect = *r;
            }
        }
        for p in &mut doc.previews {
            if let Some((_, r)) = self.previews.iter().find(|(id, _)| *id == p.path) {
                p.rect = *r;
            }
        }
    }
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
/// Where a space made outside the canvas goes on Home: right of every existing space card.
pub fn new_space_rect(spaces: &[Space]) -> Rect {
    let rects: Vec<Rect> = spaces.iter().map(|s| s.rect).collect();
    let x = union(&rects).map_or(0., |u| u[0] + u[2] + 48.);
    [x, 0., SPACE_CARD[0], SPACE_CARD[1]]
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
/// `order` regrouped so linked cells sit next to each other: each connected group
/// breadth-first from its first cell, groups in `order`, then the unlinked cells.
fn connected_first(order: &[usize], links: &[(usize, usize)]) -> Vec<usize> {
    let linked = |i: usize| links.iter().any(|(a, b)| *a == i || *b == i);
    let mut out: Vec<usize> = vec![];
    for seed in order.iter().copied().filter(|i| linked(*i)) {
        if out.contains(&seed) { continue; }
        let mut queue = std::collections::VecDeque::from([seed]);
        out.push(seed);
        while let Some(i) = queue.pop_front() {
            for j in order.iter().copied() {
                if !out.contains(&j) && links.iter().any(|l| *l == (i, j) || *l == (j, i)) {
                    out.push(j);
                    queue.push_back(j);
                }
            }
        }
    }
    out.extend(order.iter().copied().filter(|i| !linked(*i)));
    out
}

/// How far an edge leaves a cell before it turns, and how far it keeps from other cells.
const EDGE_PAD: f32 = 16.;

/// An orthogonal edge from `a` to `b`: it leaves and enters at whichever sides make the
/// shortest clear path, and goes around `others`. When no candidate is clear the cheapest
/// is kept; the canvas paints edges above cells, so it still shows.
/// shortcut: tries the two L shapes and Z shapes through the middle and along each cell's
/// padded edges, not a full pathfinder; upgrade if dense canvases still cross cells.
pub fn route(a: Rect, b: Rect, others: &[Rect]) -> Vec<Point> {
    let side = |r: Rect, s: usize| -> (Point, Point) {
        match s {
            0 => ([r[0] + r[2] / 2., r[1]], [0., -1.]),
            1 => ([r[0] + r[2], r[1] + r[3] / 2.], [1., 0.]),
            2 => ([r[0] + r[2] / 2., r[1] + r[3]], [0., 1.]),
            _ => ([r[0], r[1] + r[3] / 2.], [-1., 0.]),
        }
    };
    // Only cells near the pair can be in the way.
    let near = union(&[a, b]).map(|u| [u[0] - 200., u[1] - 200., u[2] + 400., u[3] + 400.]).unwrap();
    let others: Vec<Rect> = others.iter().copied().filter(|r| *r != a && *r != b && intersects(*r, near)).collect();
    let between = union(&[a, b]).map(|u| [u[0] - EDGE_PAD, u[1] - EDGE_PAD, u[2] + 2. * EDGE_PAD, u[3] + 2. * EDGE_PAD]).unwrap();
    let mut best: Option<(f32, Vec<Point>)> = None;
    for sa in 0..4 {
        for sb in 0..4 {
            let ((pa, na), (pb, nb)) = (side(a, sa), side(b, sb));
            let s = [pa[0] + na[0] * EDGE_PAD, pa[1] + na[1] * EDGE_PAD];
            let e = [pb[0] + nb[0] * EDGE_PAD, pb[1] + nb[1] * EDGE_PAD];
            let mut xs = vec![(s[0] + e[0]) / 2.];
            let mut ys = vec![(s[1] + e[1]) / 2.];
            // Bends hug the cells that can be in the way: the two ends and those between them.
            for r in others.iter().filter(|r| intersects(**r, between)).chain([&a, &b]) {
                xs.extend([r[0] - EDGE_PAD, r[0] + r[2] + EDGE_PAD]);
                ys.extend([r[1] - EDGE_PAD, r[1] + r[3] + EDGE_PAD]);
            }
            let mids = [vec![[s[0], e[1]]], vec![[e[0], s[1]]]].into_iter()
                .chain(xs.iter().map(|x| vec![[*x, s[1]], [*x, e[1]]]))
                .chain(ys.iter().map(|y| vec![[s[0], *y], [e[0], *y]]));
            for mid in mids {
                let mut path = vec![pa, s];
                path.extend(mid);
                path.extend([e, pb]);
                path.dedup();
                let cost = route_cost(&path, a, b, &others);
                if best.as_ref().is_none_or(|(c, _)| cost < *c) { best = Some((cost, path)); }
            }
        }
    }
    best.map(|(_, p)| p).unwrap_or_default()
}

/// Length, plus a little per bend, plus a lot per cell crossed or turn back on itself.
fn route_cost(path: &[Point], a: Rect, b: Rect, others: &[Rect]) -> f32 {
    let mut cost = 0.;
    let last = path.len() - 1;
    for (i, w) in path.windows(2).enumerate() {
        let (p, q) = (w[0], w[1]);
        cost += (q[0] - p[0]).abs() + (q[1] - p[1]).abs();
        // The stubs touch their own cell; the rest may touch neither end.
        let ends: &[Rect] = if i == 0 { &[b] } else if i + 1 == last { &[a] } else { &[a, b] };
        cost += 10_000. * others.iter().chain(ends).filter(|r| segment_hits(p, q, **r)).count() as f32;
        if i > 0 {
            let o = path[i - 1];
            let dot = (p[0] - o[0]) * (q[0] - p[0]) + (p[1] - o[1]) * (q[1] - p[1]);
            cost += if dot < 0. { 1_000. } else if dot == 0. { 24. } else { 0. };
        }
    }
    cost
}

/// Whether an axis-aligned segment passes through a rect's inside.
fn segment_hits(p: Point, q: Point, r: Rect) -> bool {
    let (x0, x1, y0, y1) = (p[0].min(q[0]), p[0].max(q[0]), p[1].min(q[1]), p[1].max(q[1]));
    x1 > r[0] + 0.5 && x0 < r[0] + r[2] - 0.5 && y1 > r[1] + 0.5 && y0 < r[1] + r[3] - 0.5
}

/// A routed path as the soft curve that is drawn and clicked: straight runs merge, and every
/// bend is rounded as far as both of its runs allow (half of each), so a Z route reads as the
/// old side-to-side S-curve and a detour as an arc around the cell. Returns sampled points.
pub fn smooth(path: &[Point]) -> Vec<Point> {
    let mut pts: Vec<Point> = Vec::with_capacity(path.len());
    for &p in path {
        if pts.last() == Some(&p) { continue; }
        // Drop the middle of three collinear points.
        if let [.., a, b] = pts[..] && ((a[0] == b[0] && b[0] == p[0]) || (a[1] == b[1] && b[1] == p[1])) { pts.pop(); }
        pts.push(p);
    }
    if pts.len() < 3 { return pts; }
    let len = |a: Point, b: Point| ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
    let toward = |v: Point, to: Point, r: f32| { let d = len(v, to); [v[0] + (to[0] - v[0]) * r / d, v[1] + (to[1] - v[1]) * r / d] };
    let mut out = vec![pts[0]];
    for (i, w) in pts.windows(3).enumerate() {
        let (o, v, q) = (w[0], w[1], w[2]);
        // The first and last runs end at a cell: they may be used whole; inner runs are shared by two bends.
        let share = |end: bool| if end { 1. } else { 0.5 };
        let r = (len(o, v) * share(i == 0)).min(len(v, q) * share(i + 3 == pts.len()));
        let (a, b) = (toward(v, o, r), toward(v, q, r));
        const STEPS: usize = 10;
        for s in 0..=STEPS {
            let t = s as f32 / STEPS as f32;
            let u = 1. - t;
            // A quadratic with its control on the corner: tangent to both runs.
            out.push([u * u * a[0] + 2. * u * t * v[0] + t * t * b[0], u * u * a[1] + 2. * u * t * v[1] + t * t * b[1]]);
        }
    }
    out.push(pts[pts.len() - 1]);
    out.dedup();
    out
}

/// The point `t` (0 to 1) of the way along a path, by length.
pub fn along(path: &[Point], t: f32) -> Point {
    let lens: Vec<f32> = path.windows(2).map(|w| ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt()).collect();
    let mut left = lens.iter().sum::<f32>() * t.clamp(0., 1.);
    for (w, len) in path.windows(2).zip(&lens) {
        if left <= *len && *len > 0. {
            let k = left / len;
            return [w[0][0] + (w[1][0] - w[0][0]) * k, w[0][1] + (w[1][1] - w[0][1]) * k];
        }
        left -= len;
    }
    path.last().copied().unwrap_or_default()
}

/// How far a point is from a path.
pub fn path_distance(path: &[Point], p: Point) -> f32 {
    path.windows(2).map(|w| {
        let (a, b) = (w[0], w[1]);
        let d = [b[0] - a[0], b[1] - a[1]];
        let len2 = d[0] * d[0] + d[1] * d[1];
        let k = if len2 > 0. { (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / len2).clamp(0., 1.) } else { 0. };
        ((a[0] + d[0] * k - p[0]).powi(2) + (a[1] + d[1] * k - p[1]).powi(2)).sqrt()
    }).fold(f32::INFINITY, f32::min)
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
/// The action required before a selected canvas cell can close.
#[derive(Debug, PartialEq)]
pub enum CloseChoice { RefuseSub, ClearSub, ConfirmAgent, ConfirmTerminal, CloseTerminal }

/// Sub-agents never terminate a process; agent confirmation cannot be silenced.
pub fn close_choice(sub_done: Option<bool>, agent: bool, confirm_terminal: bool) -> CloseChoice {
    match sub_done {
        Some(false) => CloseChoice::RefuseSub,
        Some(true) => CloseChoice::ClearSub,
        None if agent => CloseChoice::ConfirmAgent,
        None if confirm_terminal => CloseChoice::ConfirmTerminal,
        None => CloseChoice::CloseTerminal,
    }
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
    /// Its card on Home ([`SPACE_CARD`] in size); the cells inside keep their own coordinates.
    pub rect: Rect,
    pub color: String,
    /// Kept so older files load; spaces have no frame since they became rooms.
    #[serde(default)]
    pub frame: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Card {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub terminal: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    pub rect: Rect,
    pub z: u64,
    /// Set while the cell is expanded into a live terminal: its size in world px at zoom 1, header included.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<Point>,
    /// The space (room) the cell lives in; None for Home. The window keeps it in step with the core.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space: Option<String>,
}
impl Card {
    /// The canvas identity, distinct from every real terminal address.
    pub fn key(&self) -> String {
        match (&self.sub, &self.parent) {
            (Some(id), Some(parent)) => format!("sub:{parent}:{id}"),
            _ => self.terminal.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Panel {
    #[serde(default)]
    pub tree_hidden: bool,
    #[serde(default)]
    pub editor: Option<String>,
    /// The space last clicked on the canvas: the panel's file tree shows its folder.
    #[serde(default)]
    pub space: Option<String>,
    pub width: Option<f32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preview {
    pub path: String,
    pub rect: Rect,
    /// The space (room) it was opened in; None for Home.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space: Option<String>,
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
    /// Last arrange layout, defaulting to tidy for older documents.
    #[serde(default)]
    pub arrange_layout: String,
    /// Drawn Wire edges between cells of one space. Born edges are computed, never stored.
    #[serde(default)]
    pub wires: Vec<Wire>,
    /// The space being shown, its cells filling the canvas; None is Home (space cards and root cells).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room: Option<String>,
    /// Each place's last camera while another is shown: a space id, or "" for Home.
    #[serde(default)]
    pub cameras: std::collections::HashMap<String, Camera>,
}
impl Default for Document {
    fn default() -> Self {
        Self {
            version: 2,
            camera: Camera::default(),
            view: View::All,
            spaces: vec![],
            cards: vec![],
            panel: Panel::default(),
            previews: vec![],
            recent_folders: vec![],
            arrange_layout: String::new(),
            wires: vec![],
            room: None,
            cameras: Default::default(),
        }
    }
}

/// An undirected Wire edge between two cells, addressed by `Card::key()`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Wire {
    pub a: String,
    pub b: String,
    #[serde(default)]
    pub born: bool,
}
impl Wire {
    /// Whether this edge joins these two cells, in either order.
    pub fn joins(&self, a: &str, b: &str) -> bool {
        (self.a == a && self.b == b) || (self.a == b && self.b == a)
    }
}

impl Document {
    /// The space a cell lives in; None for cells on Home.
    pub fn space_of(&self, key: &str) -> Option<&Space> {
        let id = self.cards.iter().find(|c| c.key() == key)?.space.as_ref()?;
        self.spaces.iter().find(|s| &s.space == id)
    }
    /// Whether a cell or preview in `space` is shown in the current place.
    pub fn shows(&self, space: Option<&String>) -> bool {
        space == self.room.as_ref()
    }
    /// Shows another place, keeping the camera of the one left: `room` None is Home.
    pub fn go(&mut self, room: Option<String>) {
        if room == self.room { return; }
        self.cameras.insert(self.room.clone().unwrap_or_default(), self.camera);
        self.camera = self.cameras.remove(&room.clone().unwrap_or_default()).unwrap_or_default();
        self.room = room;
    }
    /// Whether both cells exist and share a space (or both sit in the root space).
    pub fn same_space(&self, a: &str, b: &str) -> bool {
        let has = |k: &str| self.cards.iter().any(|c| c.key() == k);
        has(a) && has(b) && self.space_of(a).map(|s| &s.space) == self.space_of(b).map(|s| &s.space)
    }
    /// Draws an edge between two cells of one space; false when it exists or is refused.
    pub fn connect(&mut self, a: &str, b: &str) -> bool {
        if a == b || !self.same_space(a, b) || self.wires.iter().any(|w| w.joins(a, b)) { return false; }
        self.wires.push(Wire { a: a.into(), b: b.into(), born: false });
        true
    }
    /// Drops wires of cells that no longer exist (closing a cell removes its wires); true when any went.
    pub fn prune_wires(&mut self) -> bool {
        let before = self.wires.len();
        let cards = &self.cards;
        let has = |k: &String| cards.iter().any(|c| &c.key() == k);
        self.wires.retain(|w| has(&w.a) && has(&w.b));
        before != self.wires.len()
    }
    pub fn place(&mut self, terminal: &str, point: Point, size: Point) {
        let z = self
            .cards
            .iter()
            .map(|c| c.z)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        if let Some(card) = self.cards.iter_mut().find(|c| c.key() == terminal) {
            card.rect[0] = point[0];
            card.rect[1] = point[1];
        } else {
            self.cards.push(Card {
                terminal: terminal.into(),
                sub: None, parent: None,
                rect: [point[0], point[1], size[0], size[1]],
                z,
                size: None,
                space: self.room.clone(),
            });
        }
    }
    /// Adds labelled children once and drops cells whose labels were cleared.
    pub fn sync_subs(&mut self, subs: &std::collections::HashMap<String, Vec<crate::subs::Sub>>) -> bool {
        let before = self.cards.len();
        self.cards.retain(|c| c.sub.as_ref().is_none_or(|id| c.parent.as_ref()
            .and_then(|p| subs.get(p)).is_some_and(|children| children.iter().any(|s| &s.id == id))));
        let mut changed = before != self.cards.len();
        let mut parents: Vec<_> = subs.keys().collect();
        parents.sort();
        for parent in parents {
            let Some((rect, space)) = self.cards.iter().find(|c| &c.terminal == parent && c.sub.is_none()).map(|c| (c.rect, c.space.clone())) else { continue };
            let mut children: Vec<_> = subs[parent].iter().collect();
            children.sort_by_key(|s| &s.id);
            for sub in children {
                if self.cards.iter().any(|c| c.parent.as_ref() == Some(parent) && c.sub.as_ref() == Some(&sub.id)) { continue; }
                let siblings: Vec<_> = self.cards.iter().filter(|c| c.parent.as_ref() == Some(parent)).map(|c| c.rect).collect();
                let z = self.cards.iter().map(|c| c.z).max().unwrap_or(0).saturating_add(1);
                let child = child_position(rect, &siblings, [PILL_WIDTH, 36.]);
                // The cell belongs to its parent's space.
                self.cards.push(Card { terminal: String::new(), sub: Some(sub.id.clone()), parent: Some(parent.clone()), rect: child, z, size: None, space: space.clone() });
                changed = true;
            }
        }
        changed
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
            if doc.version != 1 && doc.version != 2 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "unsupported canvas version",
                ));
            }
            if doc.version == 1 { doc.into_rooms(); }
            if doc.camera.x.is_finite()
                && doc.camera.y.is_finite()
                && doc.camera.zoom.is_finite()
                && doc.cards.iter().all(|c| valid_rect(c.rect) && c.size.is_none_or(|s| valid_rect([0., 0., s[0], s[1]])))
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
    /// Version 1 to 2: a cell belonged to the space frame holding its center; now it says so,
    /// and a space is a fixed-size card on Home at its old top left.
    fn into_rooms(&mut self) {
        let center = |r: Rect| [r[0] + r[2] / 2., r[1] + r[3] / 2.];
        for c in &mut self.cards { c.space = space_at(&self.spaces, center(c.rect)).map(|s| s.space.clone()); }
        for p in &mut self.previews { p.space = space_at(&self.spaces, center(p.rect)).map(|s| s.space.clone()); }
        for s in &mut self.spaces { s.rect = [s.rect[0], s.rect[1], SPACE_CARD[0], SPACE_CARD[1]]; }
        self.version = 2;
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        crate::persist::write_atomic(path, serde_json::to_vec_pretty(self)?)
    }
}

pub fn edge_visible(camera: Camera, path: &[Point], viewport: Point) -> bool {
    let rects: Vec<Rect> = path.iter().map(|p| [p[0] - 3., p[1] - 3., 6., 6.]).collect();
    union(&rects).is_some_and(|r| visible(camera, r, viewport))
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
    fn spacing_is_deterministic_pinned_and_returns_to_rest() {
        let rest = [[0.,0.,100.,50.],[124.,0.,100.,50.],[248.,0.,100.,50.],[124.,74.,100.,50.]];
        let pin = [100.,0.,100.,50.];
        let result = solve(&rest,0,pin,GAP);
        assert_eq!(result,solve(&rest,0,pin,GAP));
        assert_eq!(result[0],pin);
        for i in 0..result.len() { for j in 0..i { assert!(separated(result[i],result[j],GAP)); } }
        assert_eq!(solve(&rest,0,rest[0],GAP),rest);
        assert_eq!(solve(&[],0,pin,GAP),Vec::<Rect>::new());
    }

    

    #[test]
    fn arrange_layouts_keep_gap_sizes_and_parent_layers() {
        let rects = [[80.,30.,100.,40.],[0.,0.,70.,90.],[0.,50.,150.,60.],[0.,50.,30.,100.]];
        let parents = [None,Some(0),Some(0),Some(1)];
        for layout in ["tidy","row","column","wires"] {
            let result = arrange(&rects,&parents,&[],layout);
            assert_eq!(result,arrange(&rects,&parents,&[],layout));
            for i in 0..result.len() {
                assert_eq!(&result[i][2..],&rects[i][2..]);
                for j in 0..i { assert!(separated(result[i],result[j],GAP),"{layout}: {result:?}"); }
            }
            if layout == "wires" { assert!(result[0][0]<result[1][0] && result[1][0]<result[3][0]); assert_eq!(result[1][0],result[2][0]); }
        }
        let cycle = arrange(&rects,&[Some(1),Some(0),Some(2),None],&[],"wires");
        for i in 0..cycle.len() { for j in 0..i { assert!(separated(cycle[i],cycle[j],GAP)); } }
    }

    #[test]
    fn version_one_cells_move_into_their_frames_room() {
        let mut doc = Document { version: 1, ..Document::default() };
        doc.spaces.push(Space { space: "s".into(), rect: [0., 0., 960., 640.], color: "blue".into(), frame: true });
        doc.place("in", [24., 24.], [220., 36.]);
        doc.place("out", [2000., 24.], [220., 36.]);
        doc.previews.push(Preview { path: "/tmp/a.md".into(), rect: [100., 100., 200., 100.], space: None });
        doc.into_rooms();
        assert_eq!(doc.version, 2);
        assert_eq!(doc.space_of("in").map(|s| s.space.as_str()), Some("s"));
        assert!(doc.space_of("out").is_none());
        assert_eq!(doc.previews[0].space.as_deref(), Some("s"));
        assert_eq!(doc.spaces[0].rect, [0., 0., SPACE_CARD[0], SPACE_CARD[1]]);
    }

    #[test]
    fn going_into_a_room_keeps_each_places_camera() {
        let mut doc = Document::default();
        doc.camera.x = 5.;
        doc.go(Some("s".into()));
        assert_eq!((doc.camera.x, doc.room.as_deref()), (0., Some("s")));
        doc.camera.x = 9.;
        doc.go(None);
        assert_eq!(doc.camera.x, 5.);
        doc.go(Some("s".into()));
        assert_eq!(doc.camera.x, 9.);
        assert!(doc.shows(Some(&"s".to_owned())) && !doc.shows(None));
    }

    #[test]
    fn tidy_puts_wired_cells_next_to_each_other() {
        // Four in reading order; 0 is wired to 3, so 3 comes second, beside 0 in the 2-column grid.
        let rects = [[0.,0.,100.,40.],[200.,0.,100.,40.],[0.,100.,100.,40.],[200.,100.,100.,40.]];
        let out = arrange(&rects,&[None;4],&[(3,0)],"tidy");
        assert_eq!(out[3][1],out[0][1]);
        assert!(out[3][0] > out[0][0]);
    }

    #[test]
    fn undo_restores_geometry_without_resurrecting_deleted_cells() {
        let mut doc = Document::default();
        doc.place("a",[0.,0.],[100.,50.]);
        doc.spaces.push(Space { space:"s".into(),rect:[0.,0.,200.,200.],color:"blue".into(),frame:true });
        doc.previews.push(Preview { path:"/tmp/plan.md".into(),rect:[24.,24.,100.,80.], space: None });
        let rest = Rest::capture(&doc);
        doc.spaces[0].rect = [500.,500.,900.,900.];
        doc.previews[0].rect[0] = 600.;
        doc.cards[0].rect[0] = 600.;
        doc.place("new",[500.,0.],[100.,50.]);
        rest.restore(&mut doc);
        assert_eq!(doc.cards[0].rect,[0.,0.,100.,50.]);
        assert_eq!(doc.spaces[0].rect,[0.,0.,200.,200.]);
        assert_eq!(doc.previews[0].rect,[24.,24.,100.,80.]);
        assert_eq!(doc.cards[1].rect[0],500.);
        doc.cards.remove(0);
        rest.restore(&mut doc);
        assert_eq!(doc.cards.len(),1);
    }

    #[test]
    fn expanding_pins_the_cell_and_collapsing_puts_neighbours_back() {
        let pill = |x, y| [x, y, 220., 36.];
        let mut doc = Document::default();
        for (name, r) in [("a", pill(0., 0.)), ("b", pill(244., 0.)), ("c", pill(0., 60.))] {
            doc.place(name, [r[0], r[1]], [r[2], r[3]]);
        }
        let rects = |doc: &Document| doc.cards.iter().map(|c| c.rect).collect::<Vec<_>>();
        let base = Rest::capture(&doc);
        // What solving with "a" expanded to 600 x 400 does to its neighbours.
        let expanded = [0., 0., 600., 400.];
        let mut rest = rects(&doc);
        rest[0] = expanded;
        let solved = solve(&rest, 0, expanded, GAP);
        assert_eq!(solved[0], expanded);
        for (card, r) in doc.cards.iter_mut().zip(&solved) { card.rect = *r; }
        doc.cards[0].size = Some([600., 400.]);
        for i in 1..3 { assert!(separated(solved[0], solved[i], GAP)); }
        let moved = base.changed(&Rest::capture(&doc));
        // A neighbour dragged since is left where the person put it.
        doc.cards[2].rect[0] += 500.;
        let dragged = doc.cards[2].rect;
        doc.cards[0].size = None;
        base.put_back(&moved, &mut doc);
        assert_eq!(doc.cards[1].rect, pill(244., 0.));
        assert_eq!(doc.cards[2].rect, dragged);
        assert_eq!(doc.cards[0].rect[..2], [0., 0.]);
    }

    #[test]
    fn expanded_size_loads_from_old_and_new_files() {
        let mut doc = Document::default();
        doc.place("a", [0., 0.], [220., 36.]);
        let old = serde_json::to_string(&doc).unwrap();
        assert!(!old.contains("size"));
        assert_eq!(serde_json::from_str::<Document>(&old).unwrap().cards[0].size, None);
        doc.cards[0].size = Some([600., 400.]);
        let saved = serde_json::to_string(&doc).unwrap();
        assert_eq!(serde_json::from_str::<Document>(&saved).unwrap().cards[0].size, Some([600., 400.]));
    }

    #[test]
    fn undo_keeps_each_sub_agent_cell_apart() {
        let sub = |id: &str, x| Card { terminal: String::new(), sub: Some(id.into()), parent: Some("t1".into()), rect: [x, 0., 100., 36.], z: 0, size: None, space: None };
        let mut doc = Document { cards: vec![sub("a", 0.), sub("b", 200.)], ..Document::default() };
        let rest = Rest::capture(&doc);
        doc.cards[0].rect[0] = 50.;
        doc.cards[1].rect[0] = 400.;
        rest.restore(&mut doc);
        assert_eq!((doc.cards[0].rect[0], doc.cards[1].rect[0]), (0., 200.));
    }

    #[test]
    fn fifty_cells_solve_quickly() {
        let rest: Vec<_> = (0..50).map(|i|[(i%10) as f32*124.,(i/10) as f32*74.,100.,50.]).collect();
        let start = std::time::Instant::now();
        for _ in 0..100 { std::hint::black_box(solve(&rest,0,[124.,74.,100.,50.],GAP)); }
        eprintln!("50-cell solve: {:?} per pass",start.elapsed()/100);
        let result = solve(&rest,0,[124.,74.,100.,50.],GAP);
        for i in 0..result.len() { for j in 0..i { assert!(separated(result[i],result[j],GAP)); } }
    }

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
        value.as_object_mut().unwrap().remove("arrange_layout");
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
    fn route_picks_facing_sides() {
        let side_by_side = route([24., 24., 160., 36.], [264., 24., 160., 36.], &[]);
        assert_eq!((side_by_side[0], *side_by_side.last().unwrap()), ([184., 42.], [264., 42.]));
        // A cell straight below joins bottom to top, whichever way the wire was drawn.
        let below = route([0., 300., 100., 40.], [0., 0., 100., 40.], &[]);
        assert_eq!((below[0], *below.last().unwrap()), ([50., 300.], [50., 40.]));
    }
    #[test]
    fn route_goes_around_a_cell_in_the_way() {
        let (a, b, block) = ([0., 0., 100., 40.], [400., 0., 100., 40.], [180., -40., 140., 120.]);
        let path = route(a, b, &[block]);
        assert!(path.windows(2).all(|w| !segment_hits(w[0], w[1], block)), "{path:?}");
        assert!(path.windows(2).all(|w| w[0][0] == w[1][0] || w[0][1] == w[1][1]), "orthogonal: {path:?}");
    }
    #[test]
    fn tile_fills_the_area_without_overlap_and_grows_the_active_tile() {
        let area = [0., 0., 1200., 800.];
        for n in 1..=7 {
            for active in std::iter::once(None).chain((0..n).map(Some)) {
                let (out, _) = tile(area, n, active, &[], 24.);
                for (i, a) in out.iter().enumerate() {
                    assert!(a[0] >= 23.9 && a[1] >= 23.9 && a[0] + a[2] <= 1176.1 && a[1] + a[3] <= 776.1, "n={n} {a:?} leaves the area");
                    for b in &out[i + 1..] { assert!(separated(*a, *b, 23.9), "n={n} {a:?} {b:?} overlap"); }
                }
                let area_of = |r: &Rect| r[2] * r[3];
                if let Some(k) = active && n > 1 {
                    assert!(out.iter().enumerate().all(|(j, r)| j == k || area_of(r) < area_of(&out[k])), "n={n} active {k} is the largest");
                }
                // Filled: the tiles and gaps cover the inner area.
                let covered: f32 = out.iter().map(area_of).sum();
                assert!(covered > 1152. * 752. * 0.85, "n={n} covers {covered}");
            }
        }
    }

    #[test]
    fn tile_keeps_the_given_order_and_lines_pills_along_the_bottom() {
        // The tiles sit in the order given, whichever is active.
        let pills = [[0., 0., 220., 36.], [0., 0., 220., 36.]];
        for active in [None, Some(0), Some(1)] {
            let (out, _) = tile([0., 0., 1000., 600.], 2, active, &pills, 20.);
            assert!(out[0][0] < out[1][0], "{active:?}");
        }
        let (out, placed) = tile([0., 0., 1000., 600.], 2, None, &pills, 20.);
        assert!(placed.iter().all(|p| p[2] == 220. && p[3] == 36. && p[1] + p[3] <= 580.1 && p[1] > out[0][1] + out[0][3]));
        assert!(separated(placed[0], placed[1], 19.9));
        // Pills alone sit at the top of the area.
        let (_, alone) = tile([0., 0., 1000., 600.], 0, None, &pills, 20.);
        assert_eq!(alone[0][1], 20.);
    }

    #[test]
    fn reading_order_is_top_to_bottom_then_left_to_right() {
        assert_eq!(reading(&[[500., 0., 1., 1.], [0., 100., 1., 1.], [0., 0., 1., 1.]]), vec![2, 0, 1]);
    }

    #[test]
    fn smooth_keeps_straight_lines_and_rounds_bends_inside_the_route() {
        assert_eq!(smooth(&[[0., 0.], [0., 16.], [0., 100.]]), vec![[0., 0.], [0., 100.]]);
        // A Z: down, across, down. The curve stays in the route's box and keeps both ends.
        let z = [[0., 0.], [0., 50.], [100., 50.], [100., 100.]];
        let s = smooth(&z);
        assert_eq!((s[0], *s.last().unwrap()), ([0., 0.], [100., 100.]));
        assert!(s.iter().all(|p| (0. ..=100.).contains(&p[0]) && (0. ..=100.).contains(&p[1])));
        // Rounded as far as the runs allow: no sample sits on the sharp corners.
        assert!(!s.contains(&[0., 50.]) && !s.contains(&[100., 50.]));
        // Through the middle of the shared run, like the old S-curve.
        assert!(path_distance(&s, [50., 50.]) < 1.);
    }

    #[test]
    fn path_measures() {
        let path = [[0., 0.], [10., 0.], [10., 10.]];
        assert_eq!(along(&path, 0.5), [10., 0.]);
        assert_eq!(path_distance(&path, [5., 3.]), 3.);
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
            &[[-1000., 200.], [2000., 200.]],
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
    fn valid_document_round_trips() {
        let path =
            std::env::temp_dir().join(format!("canopy-roundtrip-{}.json", std::process::id()));
        let doc = Document {
            view: View::Agents,
            arrange_layout: "wires".into(),
            ..Document::default()
        };
        doc.save(&path).unwrap();
        let (loaded, bad) = Document::load(&path).unwrap();
        assert!(!bad);
        assert_eq!(loaded.view, View::Agents);
        assert_eq!(loaded.arrange_layout, "wires");
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn unsupported_version_is_untouched() {
        let path = std::env::temp_dir().join(format!("canopy-version-{}.json", std::process::id()));
        Document {
            version: 3,
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
        open_panel_tab(&mut tabs, "/tmp/b.rs");
        open_panel_tab(&mut tabs, "/tmp/a.rs");
        open_panel_tab(&mut tabs, "/tmp/b.rs");
        assert_eq!(tabs, ["/tmp/b.rs", "/tmp/a.rs"]);
        assert_eq!(
            close_panel_tab(&mut tabs, Some("/tmp/a.rs"), "/tmp/b.rs").as_deref(),
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

#[cfg(test)]
mod cell_tests {
    use super::*;
    use std::collections::HashMap;
    #[test]
    fn old_cards_load_and_sub_cards_keep_their_position_until_labels_clear() {
        let card: Card = serde_json::from_str(r#"{"terminal":"t1","rect":[0,0,320,36],"z":1}"#).unwrap();
        assert!(card.sub.is_none());
        let mut doc: Document = serde_json::from_value(serde_json::json!({
            "version":1,"camera":{"x":0,"y":0,"zoom":1},"view":"all","spaces":[],
            "cards":[{"terminal":"t1","rect":[0,0,320,36],"z":1}],"panel":{"width":null}
        })).unwrap();
        assert_eq!(doc.cards[0].key(), card.key());
        let mut subs = HashMap::from([("t1".into(), vec![crate::subs::Sub { id:"a".into(), name:"task".into(), program:serde_json::Value::Null, transcript:None, done:false }])]);
        doc.spaces.push(Space { space: "s".into(), rect: [-24., -24., 368., 84.], color: "gray".into(), frame: true });
        assert!(doc.sync_subs(&subs));
        assert_eq!(doc.cards.len(), 2);
        assert_eq!(doc.cards[1].space, doc.cards[0].space, "a sub cell lives in its parent's space");
        let key = doc.cards[1].key();
        doc.place(&key, [600., 200.], [320., 36.]);
        subs.get_mut("t1").unwrap()[0].done = true;
        assert!(!doc.sync_subs(&subs));
        assert_eq!(&doc.cards[1].rect[..2], &[600., 200.]);
        let saved = serde_json::to_value(&doc).unwrap();
        assert_eq!(saved["cards"][1]["sub"], "a");
        assert_eq!(saved["cards"][1]["parent"], "t1");
        assert!(saved["cards"][1].get("terminal").is_none());
        let mut loaded: Document = serde_json::from_value(saved).unwrap();
        assert_eq!(loaded.cards[1].key(), key);
        subs.clear();
        assert!(loaded.sync_subs(&subs));
        assert_eq!(loaded.cards.len(), 1);
    }
    #[test]
    fn closing_distinguishes_processes_from_observations() {
        assert_eq!(close_choice(Some(false), true, false), CloseChoice::RefuseSub);
        assert_eq!(close_choice(Some(true), true, true), CloseChoice::ClearSub);
        assert_eq!(close_choice(None, true, false), CloseChoice::ConfirmAgent);
        assert_eq!(close_choice(None, true, true), CloseChoice::ConfirmAgent);
        assert_eq!(close_choice(None, false, true), CloseChoice::ConfirmTerminal);
        assert_eq!(close_choice(None, false, false), CloseChoice::CloseTerminal);
        let person: crate::settings::Person = serde_json::from_str("{}").unwrap();
        assert!(person.confirm_close_terminal);
        let person: crate::settings::Person = serde_json::from_str(r#"{"confirm_close_terminal":false}"#).unwrap();
        assert!(!person.confirm_close_terminal);
    }
}

#[cfg(test)]
mod keyboard_tests {
    use super::*;

    #[test]
    fn nearest_cell_uses_strict_direction_distance_and_document_order_for_ties() {
        let from = [0., 0., 10., 10.];
        let rects = [[20., 10., 10., 10.], [20., -10., 10., 10.], [-1., 0., 10., 10.], [0., 20., 10., 10.]];
        assert_eq!(nearest(&rects, from, [1., 0.]), Some(0));
        assert_eq!(nearest(&rects, from, [-1., 0.]), Some(2));
        assert_eq!(nearest(&rects, from, [0., 1.]), Some(3));
        assert_eq!(nearest(&[from, [0., -20., 10., 10.]], from, [1., 0.]), None);
        assert_eq!(nearest(&[], from, [1., 0.]), None);
    }

    #[test]
    fn focus_fit_centres_in_visible_canvas_and_returns_to_exact_camera() {
        let original = Camera { x: -123., y: 99., zoom: 1.25 };
        let mut camera = original;
        let mut previous = None;
        let space = [500., 200., 900., 600.];
        let visible = [800., 700.]; // A 1200 px window with a 400 px file panel.
        focus_fit(&mut camera, &mut previous, &[space], visible);
        assert_eq!(previous.map(|p| p.0), Some(original));
        let center = camera.to_screen([space[0] + space[2] / 2., space[1] + space[3] / 2.]);
        assert!((center[0] - visible[0] / 2.).abs() < 0.001);
        assert!((center[1] - visible[1] / 2.).abs() < 0.001);
        assert!(space[2] * camera.zoom <= visible[0] - 48.);
        assert!(space[3] * camera.zoom <= visible[1] - 48.);
        focus_fit(&mut camera, &mut previous, &[space], visible);
        assert_eq!(camera, original);
        assert_eq!(previous, None);
        focus_fit(&mut camera, &mut previous, &[], visible);
        assert_eq!(previous, None);
        // Moved after a fit: the next press fits again instead of jumping back.
        focus_fit(&mut camera, &mut previous, &[space], visible);
        let fitted = camera;
        camera.x += 50.;
        focus_fit(&mut camera, &mut previous, &[space], visible);
        assert_eq!(camera, fitted);
    }
}
