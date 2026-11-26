//! The placement rules: pure functions from rects to rects, all in world px.
use super::units::{Point, Rect, intersects, union};

/// The active tile's share of its row, and (at least) its row's share of the height.
const TILE_ACTIVE: f32 = 0.6;

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

/// Resolve from rest positions, holding the dragged group fixed and placing its neighbours.
pub fn solve(rest: &[Rect], pinned: usize, rect: Rect, gap: f32, carried: impl Fn(usize) -> bool) -> Vec<Rect> {
    if pinned >= rest.len() {
        return rest.to_vec();
    }
    let mut out = rest.to_vec();
    out[pinned] = rect;
    let (mut fixed, mut order): (Vec<_>, Vec<_>) = (0..rest.len())
        .filter(|i| *i != pinned)
        .partition(|i| carried(*i));
    fixed.insert(0, pinned);
    let distance = |r: Rect| {
        (r[0] + r[2] / 2. - rect[0] - rect[2] / 2.).powi(2)
            + (r[1] + r[3] / 2. - rect[1] - rect[3] / 2.).powi(2)
    };
    order.sort_by(|a, b| {
        distance(rest[*a])
            .total_cmp(&distance(rest[*b]))
            .then(a.cmp(b))
    });
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
pub(super) fn separated(a: Rect, b: Rect, gap: f32) -> bool {
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

/// Where the next ⌘T pane goes: `step` down and right of the last one, back at the view's
/// top-left when it would leave `view` ([x, y, w, h] world px), never on another pane's origin.
pub fn cascade(last: Option<Point>, view: Rect, size: Point, taken: &[Point], step: f32) -> Point {
    let start = [view[0] + step, view[1] + step];
    let fits = |p: Point| p[0] + size[0] <= view[0] + view[2] && p[1] + size[1] <= view[1] + view[3];
    let free = |p: Point| !taken.iter().any(|t| (t[0] - p[0]).abs() < 1. && (t[1] - p[1]).abs() < 1.);
    let mut p = last.map_or(start, |l| [l[0] + step, l[1] + step]);
    if !fits(p) { p = start; }
    // At most one step per pane: every free spot is reached before the walk repeats.
    for _ in 0..=taken.len() {
        if free(p) { break; }
        p = [p[0] + step, p[1] + step];
        if !fits(p) && fits(start) { p = start; }
    }
    p
}

/// Drags a pane edge or corner. `zone` is (-1 left, 0, 1 right) and (-1 top, 0, 1 bottom).
/// Left and top move the origin so the opposite edge stays put; size never drops below `min`.
pub fn resize_from(origin: Point, size: Point, zone: (i8, i8), d: Point, min: Point) -> (Point, Point) {
    let axis = |o: f32, s: f32, z: i8, d: f32, m: f32| match z {
        1 => (o, (s + d).max(m)),
        -1 => { let n = (s - d).max(m); (o + s - n, n) }
        _ => (o, s),
    };
    let (x, w) = axis(origin[0], size[0], zone.0, d[0], min[0]);
    let (y, h) = axis(origin[1], size[1], zone.1, d[1], min[1]);
    ([x, y], [w, h])
}
