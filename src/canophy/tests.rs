//! Tests for canophy: the unit laws of each rule, then the invariants checked after random
//! operation sequences.
use super::camera::{Camera, ZOOMS, focus_fit, reveal, visible, widen, zoom_range};
use super::rules::*;
use super::check::{self, Violation};
use super::step::{Intent, Viewport, step};
use super::units::{Point, Rect};
use crate::canvas::{Document, Kind, Pane};
use proptest::prelude::*;

/// The gap the layout tests leave between panes.
const GAP: f32 = 24.;

#[test]
fn reveal_centers_a_pane_only_when_it_is_off_screen() {
    let camera = Camera::new(0., 0., 1.);
    assert_eq!(reveal(camera, [10., 10., 100., 100.], [0., 0., 800., 600.], camera.zoom()), None);
    let to = reveal(camera, [1000., 0., 200., 100.], [0., 0., 800., 600.], camera.zoom()).unwrap();
    assert_eq!(to.to_screen([1100., 50.]), [400., 300.]);
}

/// The `Open` law: a pane the person opens comes to 100%, centered in the part no chrome covers,
/// even when it was already on screen at another zoom.
#[test]
fn opening_eases_to_full_size_centered_in_the_open_part() {
    let camera = Camera::new(0., 0., 0.5);
    let open = [200., 40., 600., 560.];
    let to = reveal(camera, [100., 100., 200., 100.], open, 1.).unwrap();
    assert_eq!(to.zoom(), 1.);
    assert_eq!(to.to_screen([200., 150.]), [500., 320.]);
    assert_eq!(reveal(to, [100., 100., 200., 100.], open, 1.), None);
}

#[test]
fn spacing_is_deterministic_pinned_and_returns_to_rest() {
    let rest = [[0.,0.,100.,50.],[124.,0.,100.,50.],[248.,0.,100.,50.],[124.,74.,100.,50.]];
    let pin = [100.,0.,100.,50.];
    let result = solve(&rest,0,pin,GAP, |_| false);
    assert_eq!(result,solve(&rest,0,pin,GAP, |_| false));
    assert_eq!(result[0],pin);
    for i in 0..result.len() { for j in 0..i { assert!(separated(result[i],result[j],GAP)); } }
    assert_eq!(solve(&rest,0,rest[0],GAP, |_| false),rest);
    assert_eq!(solve(&[],0,pin,GAP, |_| false),Vec::<Rect>::new());
}

#[test]
fn arrange_layouts_keep_gap_sizes_and_parent_layers() {
    let rects = [[80.,30.,100.,40.],[0.,0.,70.,90.],[0.,50.,150.,60.],[0.,50.,30.,100.]];
    let parents = [None,Some(0),Some(0),Some(1)];
    for layout in ["tidy","row","column","wires"] {
        let result = arrange(&rects,&parents,&[], layout, 24.);
        assert_eq!(result,arrange(&rects,&parents,&[], layout, 24.));
        for i in 0..result.len() {
            assert_eq!(&result[i][2..],&rects[i][2..]);
            for j in 0..i { assert!(separated(result[i],result[j],GAP),"{layout}: {result:?}"); }
        }
        if layout == "wires" { assert!(result[0][0]<result[1][0] && result[1][0]<result[3][0]); assert_eq!(result[1][0],result[2][0]); }
    }
    let cycle = arrange(&rects,&[Some(1),Some(0),Some(2),None],&[], "wires", 24.);
    for i in 0..cycle.len() { for j in 0..i { assert!(separated(cycle[i],cycle[j],GAP)); } }
}

#[test]
fn tidy_puts_wired_cells_next_to_each_other() {
    // Four in reading order; 0 is wired to 3, so 3 comes second, beside 0 in the 2-column grid.
    let rects = [[0.,0.,100.,40.],[200.,0.,100.,40.],[0.,100.,100.,40.],[200.,100.,100.,40.]];
    let out = arrange(&rects,&[None;4],&[(3,0)],"tidy", 24.);
    assert_eq!(out[3][1],out[0][1]);
    assert!(out[3][0] > out[0][0]);
}

#[test]
fn fifty_cells_solve_quickly() {
    let rest: Vec<_> = (0..50).map(|i|[(i%10) as f32*124.,(i/10) as f32*74.,100.,50.]).collect();
    let start = std::time::Instant::now();
    for _ in 0..100 { std::hint::black_box(solve(&rest,0,[124.,74.,100.,50.],GAP, |_| false)); }
    eprintln!("50-cell solve: {:?} per pass",start.elapsed()/100);
    let result = solve(&rest,0,[124.,74.,100.,50.],GAP, |_| false);
    for i in 0..result.len() { for j in 0..i { assert!(separated(result[i],result[j],GAP)); } }
}

#[test]
fn camera_round_trip() {
    let c = Camera::new(-200., 32., 0.5);
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
    let mut c = Camera::new(0., 0., 0.52);
    c.step(1, [0., 0.], [ZOOMS[0], ZOOMS[11]]);
    assert_eq!(c.zoom(), 0.67);
    c.step(1, [0., 0.], [0.25, 0.7]);
    assert_eq!(c.zoom(), 0.7);
    c.step(-1, [0., 0.], [0.25, 0.7]);
    assert_eq!(c.zoom(), 0.67);
}

#[test]
fn zoom_range_keeps_the_biggest_pane_on_screen() {
    let view = [1080., 880.];
    // Nothing on the canvas: the whole ZOOMS range.
    assert_eq!(zoom_range(&[], view), [ZOOMS[0], ZOOMS[11]]);
    // A 1000 px wide pane fits at 1.0 with padding, not beyond; a small one allows 2.
    assert_eq!(zoom_range(&[[0., 0., 1000., 400.]], view)[1], 1.);
    assert_eq!(zoom_range(&[[0., 0., 100., 100.]], view)[1], 2.);
    // Two panes far apart: zooming out stops at the fit-all step, in at the bigger pane.
    let r = zoom_range(&[[0., 0., 500., 400.], [3000., 0., 200., 200.]], view);
    assert_eq!(r, [0.25, 2.]);
    let r = zoom_range(&[[0., 0., 500., 400.], [1500., 0., 200., 200.]], view);
    assert_eq!(r, [0.5, 2.]);
    // A pane bigger than the window at any zoom: both ends pin to the smallest zoom.
    assert_eq!(zoom_range(&[[0., 0., 99999., 100.]], view), [ZOOMS[0], ZOOMS[0]]);
    // Left at 2 after the limit fell to 1: zooming out moves freely, not straight to 1.
    let mut c = Camera::new(0., 0., 2.);
    c.step(1, [0., 0.], widen([0.25, 1.], 2.));
    assert_eq!(c.zoom(), 2., "zoom in never zooms out");
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
fn fit_uses_largest_fitting_zoom_step() {
    let mut c = Camera::default();
    c.fit(&[[0., 0., 100., 100.]], [800., 600.]);
    assert_eq!(c.zoom(), 2.);
}

#[test]
fn focus_fit_centres_in_visible_canvas_and_returns_to_exact_camera() {
    let original = Camera::new(-123., 99., 1.25);
    let mut camera = original;
    let mut previous = None;
    let space = [500., 200., 900., 600.];
    let visible = [0., 0., 800., 700.]; // A 1200 px window with a 400 px file panel.
    focus_fit(&mut camera, &mut previous, &[space], visible);
    assert_eq!(previous.map(|p| p.0), Some(original));
    let center = camera.to_screen([space[0] + space[2] / 2., space[1] + space[3] / 2.]);
    assert!((center[0] - visible[2] / 2.).abs() < 0.001);
    assert!((center[1] - visible[3] / 2.).abs() < 0.001);
    assert!(space[2] * camera.zoom() <= visible[2] - 48.);
    assert!(space[3] * camera.zoom() <= visible[3] - 48.);
    focus_fit(&mut camera, &mut previous, &[space], visible);
    assert_eq!(camera, original);
    assert_eq!(previous, None);
    focus_fit(&mut camera, &mut previous, &[], visible);
    assert_eq!(previous, None);
    // Moved after a fit: the next press fits again instead of jumping back.
    focus_fit(&mut camera, &mut previous, &[space], visible);
    let fitted = camera;
    camera = Camera::new(camera.origin()[0] + 50., camera.origin()[1], camera.zoom());
    focus_fit(&mut camera, &mut previous, &[space], visible);
    assert_eq!(camera, fitted);
    // A toolbar over the top 60 px: the space centres in the open part below it.
    let (mut camera, mut previous) = (original, None);
    focus_fit(&mut camera, &mut previous, &[space], [0., 60., 800., 640.]);
    let center = camera.to_screen([space[0] + space[2] / 2., space[1] + space[3] / 2.]);
    assert!((center[0] - 400.).abs() < 0.001 && (center[1] - 380.).abs() < 0.001);
    let fitted = camera;
    focus_fit(&mut camera, &mut previous, &[space], [0., 60., 800., 640.]);
    assert_eq!(camera, original);
    focus_fit(&mut camera, &mut previous, &[space], [0., 60., 800., 640.]);
    assert_eq!(camera, fitted);
}

#[test]
fn new_panes_step_down_right_wrap_and_never_share_an_origin() {
    let view = [0., 0., 1000., 800.];
    let size = [400., 300.];
    assert_eq!(cascade(None, view, size, &[], 32.), [32., 32.]);
    assert_eq!(cascade(Some([32., 32.]), view, size, &[[32., 32.]], 32.), [64., 64.]);
    // No room at the bottom right: back to the top left.
    assert_eq!(cascade(Some([580., 480.]), view, size, &[], 32.), [32., 32.]);
    // The top left is taken: the next free step.
    assert_eq!(cascade(Some([580., 480.]), view, size, &[[32., 32.], [64., 64.]], 32.), [96., 96.]);
    // A view smaller than a pane still never stacks two exactly.
    assert_eq!(cascade(None, [0., 0., 100., 100.], size, &[[32., 32.]], 32.), [64., 64.]);
}

#[test]
fn every_edge_and_corner_moves_only_its_own_sides() {
    let (o, s, d, min) = ([100., 100.], [400., 300.], [10., 20.], [50., 50.]);
    for (zone, want) in [
        ((1, 1), ([100., 100.], [410., 320.])),
        ((-1, -1), ([110., 120.], [390., 280.])),
        ((1, -1), ([100., 120.], [410., 280.])),
        ((-1, 1), ([110., 100.], [390., 320.])),
        ((1, 0), ([100., 100.], [410., 300.])),
        ((-1, 0), ([110., 100.], [390., 300.])),
        ((0, 1), ([100., 100.], [400., 320.])),
        ((0, -1), ([100., 120.], [400., 280.])),
    ] {
        assert_eq!(resize_from(o, s, zone, d, min), want, "zone {zone:?}");
    }
    // Shrinking past the minimum stops there, and the right/bottom edge stays put.
    assert_eq!(resize_from(o, s, (-1, -1), [1000., 1000.], min), ([450., 350.], [50., 50.]));
}

#[test]
fn traits_keep_the_sizes_the_app_used() {
    use super::traits::{Content, traits};
    use crate::canvas::{Kind, header_height};
    let browser = Kind::Browser { tabs: vec![], active: 0, home: String::new(), viewport: None };
    let unknown = Kind::Mod { kind: "mod:unknown-traits-test".into(), props: Default::default() };
    let file = Kind::File { path: "/a.md".into() };
    let terminal = Kind::Terminal { size: None };
    assert_eq!(<[f32; 2]>::from(traits(&terminal).min), [330., 108.]);
    assert_eq!(<[f32; 2]>::from(traits(&file).min), [240., 120.]);
    assert_eq!(<[f32; 2]>::from(traits(&browser).default), [960., 680.]);
    assert_eq!(<[f32; 2]>::from(traits(&Kind::Sim { udid: String::new() }).default), [340., 700.]);
    assert_eq!(<[f32; 2]>::from(traits(&Kind::Pdf { path: String::new(), highlights: vec![] }).default), [560., 720.]);
    assert_eq!(<[f32; 2]>::from(traits(&unknown).default), [320., 240.]);
    assert_eq!((header_height(&browser), header_height(&unknown), header_height(&file)), (66., 36., 36.));
    assert_eq!((traits(&terminal).content, traits(&browser).dockable, traits(&file).dockable), (Content::Grid, true, false));
}

#[test]
fn pdf_open_size_fits_the_first_page() {
    use super::{Natural, WorldSize, open::OPEN_MARGIN, open_size, traits};

    let t = traits(&Kind::Pdf { path: String::new(), highlights: vec![] });
    let (chrome_w, chrome_h) = gpui_pdf::FIT_CHROME;
    let frame = 2. * super::layout::BORDER;
    for [vw, vh] in [[1440., 900.], [800., 600.]] {
        let view = Viewport { size: [vw + 200., vh], open: [200., 0., vw, vh] };
        for (w, h) in [(612., 792.), (595.28, 841.89)] {
            let size = open_size(&t, Some(Natural {
                content: WorldSize::new(gpui_pdf::PAGE_WIDTH, gpui_pdf::PAGE_WIDTH * h / w),
                chrome: WorldSize::new(chrome_w, chrome_h),
            }), &view);
            let page_w = size.w - chrome_w - frame;
            let page_h = size.h - chrome_h - frame - t.header.unwrap();
            assert!((page_h - page_w * h / w).abs() < 0.5);
            assert!(size.w <= vw - 2. * OPEN_MARGIN + 0.01);
            assert!((size.h - (vh - 2. * OPEN_MARGIN)).abs() < 0.01);
        }
        assert_eq!(open_size(&t, None, &view), t.default);
    }
}

#[test]
fn image_open_size_shrinks_by_width_and_leaves_small_images_at_their_size() {
    use super::{Natural, WorldSize, open::OPEN_MARGIN, open_size, traits};

    let t = traits(&Kind::Image { path: String::new() });
    let view = Viewport::whole([1440., 900.]);
    let image = |w, h| Some(Natural { content: WorldSize::new(w, h), chrome: WorldSize::default() });
    let size = open_size(&t, image(4000., 1000.), &view);
    let frame = 2. * super::layout::BORDER;
    let header = t.header.unwrap();
    assert!((size.w - (view.open[2] - 2. * OPEN_MARGIN)).abs() < 0.01);
    assert!((size.h - frame - header - (size.w - frame) / 4.).abs() < 0.01);
    let size = open_size(&t, image(100., 80.), &view);
    // The image uses its own pixels inside the pane's minimum frame.
    assert_eq!(size, WorldSize::new(t.min.w, (80. + frame + header).max(t.min.h)));
    assert_eq!(open_size(&t, image(4000., 1000.), &Viewport::whole([100., 80.])), t.min);
}

#[test]
fn column_open_size_uses_the_canvas_height_and_fixed_size_uses_the_default() {
    use super::{WorldSize, open::OPEN_MARGIN, open_size, traits};
    use super::traits::TEXT_COLUMN;

    let file = traits(&Kind::File { path: String::new() });
    let terminal = traits(&Kind::Terminal { size: None });
    for size in [[1440., 900.], [800., 600.], [500., 600.], [100., 80.]] {
        let view = Viewport::whole(size);
        let w = TEXT_COLUMN.min(size[0] - 2. * OPEN_MARGIN).max(file.min.w);
        let h = (size[1] - 2. * OPEN_MARGIN).max(file.min.h);
        assert_eq!(open_size(&file, None, &view), WorldSize::new(w, h));
        assert_eq!(open_size(&terminal, None, &view), terminal.default);
    }
}

#[test]
fn rest_scale_uses_the_destination_or_waits_for_a_gesture() {
    use super::{Motion, motion::GESTURE_REST, rest_scale};
    use std::time::Instant;

    let mut pane = Pane { id: "p1".into(), kind: Kind::Pdf { path: String::new(), highlights: vec![] },
        rect: [0., 0., 560., 720.].into(), z: 1, space: None, color: None, pinned: None };
    let now = Instant::now();
    let camera = Camera::new(0., 0., 0.5);
    let ease = Motion::ease(camera, Camera::new(0., 0., 1.25), [0., 0.], now);
    let gesture = Motion::Gesture { since: now };
    assert_eq!(rest_scale(&Motion::Rested, camera, &pane, now), Some(0.5));
    assert_eq!(rest_scale(&ease, camera, &pane, now), Some(1.25));
    assert_eq!(rest_scale(&gesture, camera, &pane, now), None);
    assert_eq!(rest_scale(&gesture, camera, &pane, now + GESTURE_REST), Some(0.5));
    pane.pinned = Some([0., 0., 560., 720.]);
    assert_eq!(rest_scale(&ease, camera, &pane, now), Some(1.));
}

/// A saved canvas with every pane kind, in the exact bytes `Document::save` writes.
const SAVED: &str = include_str!("canvas.fixture.json");

#[test]
fn a_saved_canvas_round_trips_byte_for_byte() {
    let doc = crate::canvas::Document::parse(SAVED.as_bytes()).unwrap().unwrap();
    assert_eq!(doc.panes.len(), 8, "every kind is in the fixture");
    check::document(&doc).unwrap();
    assert_eq!(serde_json::to_string_pretty(&doc).unwrap(), SAVED.trim_end());
}

/// One action on the canvas, as the window would run it through `step`.
#[derive(Clone, Debug)]
enum Op {
    Pan(Point),
    ZoomAt(f32, Point),
    Step(bool, Point),
    Fit,
    Reveal(usize),
    Move(usize, Point),
    Resize(usize, (i8, i8), Point),
    Arrange(&'static str),
    Tile(usize, Option<usize>),
    Pin(usize, Option<Rect>),
    /// The window resized with pane `usize` (mod n) entered.
    Resized(Point, usize),
}

const MIN: Point = [240., 120.];

/// A canvas of file panes (one per rect; the first `pinned` of them pinned) in a viewport.
fn world(rects: &[Rect], view: Point) -> (Document, Viewport) {
    let mut doc = Document::default();
    for (i, r) in rects.iter().enumerate() {
        doc.panes.push(Pane { id: format!("p{i}"), kind: Kind::File { path: format!("/{i}.md") }, rect: (*r).into(), z: i as u64, space: None, color: None, pinned: None });
    }
    (doc, Viewport::whole(view))
}

/// The canvas rects (pinned panes left out: they float over it).
fn rects(doc: &Document) -> Vec<Rect> { doc.panes.iter().filter(|p| p.pinned.is_none()).map(|p| p.rect.array()).collect() }
fn screens(doc: &Document) -> Vec<Option<Rect>> { doc.panes.iter().map(|p| p.pinned).collect() }
fn ids(doc: &Document) -> Vec<String> { doc.panes.iter().map(|p| p.id.clone()).collect() }

/// After a push the window places what `solve` returned, pinning the pane `i` moved.
fn push(doc: &mut Document, view: &Viewport, i: usize) -> Result<(), Violation> {
    let free: Vec<usize> = (0..doc.panes.len()).filter(|j| doc.panes[*j].pinned.is_none()).collect();
    let Some(at) = free.iter().position(|j| *j == i) else { return Ok(()) };
    let rs = rects(doc);
    let solved = solve(&rs, at, rs[at], GAP, |_| false);
    check::apart(&solved, GAP)?;
    let places = free.iter().zip(&solved).map(|(j, r)| (doc.panes[*j].id.clone(), (*r).into())).collect();
    step(doc, Intent::Place(places), view);
    Ok(())
}

/// Applies `op` through `step` the way the window does, then checks every invariant that applies.
fn apply(doc: &mut Document, view: &mut Viewport, op: &Op) -> Result<(), Violation> {
    let (before, pinned_before) = (doc.camera, screens(doc));
    let (n, free) = (doc.panes.len(), rects(doc));
    let range = widen(zoom_range(&free, view.size), doc.camera.zoom());
    let rects_before = rects(doc);
    match *op {
        Op::Pan(d) => { step(doc, Intent::Pan(d), view); }
        Op::ZoomAt(zoom, anchor) => {
            step(doc, Intent::ZoomAt { zoom, anchor, range }, view);
            check::anchored(before, doc.camera, anchor)?;
        }
        Op::Step(up, anchor) => {
            // The window eases to this target; the last frame is an exact `Camera` intent.
            let mut to = doc.camera;
            to.step(if up { 1 } else { -1 }, anchor, range);
            step(doc, Intent::Camera(to), view);
            check::zoom(doc.camera, range)?;
            check::anchored(before, doc.camera, anchor)?;
        }
        Op::Fit => {
            let mut to = doc.camera;
            to.fit(&free, view.size);
            step(doc, Intent::Camera(to), view);
            if let Some(all) = super::units::union(&free) { check::revealed(doc.camera, all, view.size)?; }
        }
        Op::Reveal(i) => {
            if let Some(rect) = free.get(i % free.len().max(1)).copied() {
                if let Some(to) = reveal(doc.camera, rect, [0., 0., view.size[0], view.size[1]], doc.camera.zoom()) { step(doc, Intent::Camera(to), view); }
                check::revealed(doc.camera, rect, view.size)?;
            }
        }
        Op::Move(i, d) => {
            let i = i % n;
            step(doc, Intent::Move { ids: vec![doc.panes[i].id.clone()], world: d, screen: d }, view);
            push(doc, view, i)?;
        }
        Op::Resize(i, zone, d) => {
            let i = i % n;
            step(doc, Intent::Resize { id: doc.panes[i].id.clone(), zone, world: d, screen: d }, view);
            let p = &doc.panes[i];
            check::at_least(p.pinned.map_or([p.rect.w, p.rect.h], |s| [s[2], s[3]]), MIN)?;
            push(doc, view, i)?;
        }
        Op::Arrange(layout) => {
            let free_ids: Vec<_> = doc.panes.iter().filter(|p| p.pinned.is_none()).map(|p| p.id.clone()).collect();
            let out = arrange(&free, &vec![None; free.len()], &[], layout, 24.);
            check::apart(&out, 24.)?;
            step(doc, Intent::Place(free_ids.into_iter().zip(out.into_iter().map(Into::into)).collect()), view);
        }
        Op::Tile(tiles, active) => {
            // Tile view lays out a picture of the panes; the check is on the picture.
            let (out, _) = tile(view.open, tiles, active.filter(|a| *a < tiles), &[], GAP);
            check::apart(&out, GAP)?;
        }
        Op::Pin(i, screen) => { step(doc, Intent::Pin { id: doc.panes[i % n].id.clone(), screen }, view); }
        Op::Resized(size, i) => {
            *view = Viewport::whole(size);
            let id = doc.panes.get(i % n.max(1)).map(|p| p.id.clone());
            step(doc, Intent::ViewportChanged { reveal: id.clone() }, view);
            if let Some(p) = doc.panes.iter().find(|p| Some(&p.id) == id.as_ref() && p.pinned.is_none()) { check::revealed(doc.camera, p.rect.array(), view.size)?; }
        }
    }
    // I12: camera-only actions never touch panes on the canvas.
    if matches!(op, Op::Pan(_) | Op::ZoomAt(..) | Op::Step(..) | Op::Fit | Op::Reveal(_) | Op::Tile(..) | Op::Resized(..)) {
        check::untouched(&rects_before, &rects(doc))?;
        // I10: pinned panes keep their screen rect while the camera moves.
        if screens(doc) != pinned_before { return Err(Violation::PanesMoved); }
        // Each is drawn at its saved spot, pulled inside the viewport.
        for p in doc.panes.iter().filter(|p| p.pinned.is_some()) {
            let [x, y, w, h] = p.pinned.unwrap();
            let want = [x.min(view.size[0] - w).max(0.), y.min(view.size[1] - h).max(0.)];
            let drawn = doc.camera.rect_to_screen(p.rect);
            if (drawn[0] - want[0]).abs() > 0.5 || (drawn[1] - want[1]).abs() > 0.5 { return Err(Violation::PanesMoved); }
        }
    }
    check::document(doc)?;
    let r = doc.panes[0].rect;
    check::round_trip(doc.camera, [r.x + r.w / 2., r.y + r.h / 2.])
}

fn op() -> impl Strategy<Value = Op> {
    let point = || (-1500f32..1500., -1500f32..1500.).prop_map(|(x, y)| [x, y]);
    let screen = || (0f32..1200., 0f32..800.).prop_map(|(x, y)| [x, y]);
    let zone = prop_oneof![Just((-1, -1)), Just((0, -1)), Just((1, -1)), Just((-1, 0)), Just((1, 0)), Just((-1, 1)), Just((0, 1)), Just((1, 1))];
    let pin = proptest::option::of((0f32..900., 0f32..600., MIN[0]..600., MIN[1]..500.).prop_map(|(x, y, w, h)| [x, y, w, h]));
    prop_oneof![
        point().prop_map(Op::Pan),
        (ZOOMS[0]..=ZOOMS[11], screen()).prop_map(|(z, a)| Op::ZoomAt(z, a)),
        (any::<bool>(), screen()).prop_map(|(up, a)| Op::Step(up, a)),
        Just(Op::Fit),
        (0usize..16).prop_map(Op::Reveal),
        (0usize..16, point()).prop_map(|(i, d)| Op::Move(i, d)),
        (0usize..16, zone, point()).prop_map(|(i, z, d)| Op::Resize(i, z, d)),
        prop_oneof![Just("tidy"), Just("row"), Just("column")].prop_map(Op::Arrange),
        (0usize..10, proptest::option::of(0usize..10)).prop_map(|(n, a)| Op::Tile(n, a)),
        (0usize..16, pin).prop_map(|(i, s)| Op::Pin(i, s)),
        (400f32..2400., 300f32..1400., 0usize..16).prop_map(|(w, h, i)| Op::Resized([w, h], i)),
    ]
}

fn rect() -> impl Strategy<Value = Rect> {
    (-2000f32..2000., -2000f32..2000., MIN[0]..1200., MIN[1]..900.).prop_map(|(x, y, w, h)| [x, y, w, h])
}

/// Runs `ops` from `rects`, stopping at the first broken invariant. Returns what a replay
/// must reproduce: the pane ids, rects, pinned screen rects and camera.
fn run(rects: &[Rect], ops: &[Op]) -> Result<(Vec<String>, Vec<Rect>, Vec<Option<Rect>>, Camera), (usize, Violation)> {
    let (mut doc, mut view) = world(rects, [1200., 800.]);
    for (i, op) in ops.iter().enumerate() {
        apply(&mut doc, &mut view, op).map_err(|v| (i, v))?;
    }
    Ok((ids(&doc), doc.panes.iter().map(|p| p.rect.array()).collect(), screens(&doc), doc.camera))
}

proptest! {
    /// Random action sequences through `step` keep every invariant after every step (I1, I3,
    /// I4, I7, I8, I9, I10, I12) and replay to the same canvas (I13). `PROPTEST_CASES=5000`
    /// runs a deeper search.
    #[test]
    fn invariants_hold_after_any_sequence(rects in proptest::collection::vec(rect(), 1..8), ops in proptest::collection::vec(op(), 0..200)) {
        let first = run(&rects, &ops);
        prop_assert!(first.is_ok(), "{:?}", first.as_ref().err());
        prop_assert_eq!(first, run(&rects, &ops));
    }
}

#[test]
fn a_viewport_change_keeps_a_pinned_pane_inside_the_window() {
    let (mut doc, view) = world(&[[0., 0., 300., 200.]], [1200., 800.]);
    step(&mut doc, Intent::Pin { id: "p0".into(), screen: Some([1000., 16., 180., 300.]) }, &view);
    let drawn = |doc: &Document| doc.camera.rect_to_screen(doc.panes[0].rect);
    assert_eq!(drawn(&doc)[0], 1000.);
    step(&mut doc, Intent::Pan([300., 40.]), &view);
    assert_eq!(drawn(&doc)[..2], [1000., 16.], "pan leaves it where it sits on screen");
    let narrow = Viewport::whole([900., 800.]);
    step(&mut doc, Intent::ViewportChanged { reveal: None }, &narrow);
    assert_eq!(drawn(&doc)[0], 720., "a narrower window keeps it whole on screen");
    assert_eq!(doc.panes[0].pinned, Some([1000., 16., 180., 300.]), "its saved spot is kept");
}

#[test]
fn a_viewport_change_reveals_the_entered_pane_and_keeps_the_left_edge() {
    let (mut doc, _) = world(&[[0., 0., 300., 200.], [900., 0., 200., 200.]], [1200., 800.]);
    let (left, narrow) = (doc.camera.to_world([0., 0.]), Viewport::whole([600., 800.]));
    step(&mut doc, Intent::ViewportChanged { reveal: Some("p0".into()) }, &narrow);
    assert_eq!(doc.camera.to_world([0., 0.]), left, "a pane still in view leaves the camera");
    step(&mut doc, Intent::ViewportChanged { reveal: Some("p1".into()) }, &narrow);
    assert!(check::revealed(doc.camera, doc.panes[1].rect.array(), narrow.size).is_ok(), "the covered pane comes back");
}

#[test]
fn resize_never_shrinks_a_pane_below_its_kind() {
    let (mut doc, view) = world(&[[0., 0., 300., 200.]], [1200., 800.]);
    step(&mut doc, Intent::Resize { id: "p0".into(), zone: (1, 1), world: [-1000., -1000.], screen: [0., 0.] }, &view);
    assert_eq!([doc.panes[0].rect.w, doc.panes[0].rect.h], MIN);
}

#[test]
fn contain_centers_content_with_chrome() {
    assert_eq!(contain([0., 0., 1000., 400.], 0.5, [10., 40.]), [405., 0., 190., 400.]);
    assert_eq!(contain([0., 0., 100., 500.], 0.5, [10., 40.]), [0., 140., 100., 220.]);
    assert_eq!(contain([0., 0., 100., 30.], 0.5, [10., 40.]), [0., 0., 100., 30.]);
    let cell = [3., 5., 100., 500.];
    for aspect in [0., -1., f32::NAN, f32::INFINITY] {
        assert_eq!(contain(cell, aspect, [10., 40.]), cell);
    }
}
