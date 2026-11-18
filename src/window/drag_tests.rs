// Modified for Canopy.
#[test]
fn dragging_squeezed_layout_blocks_never_crashes() {
    in_own_process("squeezed_layout_drag");
}

#[gpui::test]
fn squeezed_layout_drag(cx: &mut gpui::TestAppContext) {
    use gpui::{point, px, size};
    use serde_json::json;
    if !running_alone("squeezed_layout_drag") { return; }
    let block = |id: &str, height: f32| json!({
        "id": id, "type": "block", "size": {"px": height}, "content": {"mod": "drag-probe"}
    });
    let layout = json!({"id": "root", "type": "row", "children": [
        {"id": "sidebar", "type": "column", "size": {"px": 268}, "resizable": false, "children": [
            block("brand", 46.0), block("button", 52.0),
            {"id": "lists", "type": "column", "divider": {"width": 1, "color": "border"}, "children": [
                block("heading", 34.0),
                {"id": "stack", "type": "stack", "children": [
                    {"id": "list", "type": "block", "content": {"mod": "drag-probe"}}
                ]},
                block("zero1", 0.0), block("zero2", 0.0), block("zero3", 0.0), block("zero4", 0.0), block("zero5", 0.0)
            ]},
            block("pendientes", 200.0), block("vistas", 46.0)
        ]}
    ]});
    let (view, cx) = open_window(cx, "drag-probe", "export default () => null;", layout.clone());
    for height in [800.0, 380.0, 344.0, 300.0] {
        cx.simulate_resize(size(px(900.0), px(height)));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        view.update(cx, |this, _| {
            for (container, count) in [("sidebar", 4), ("lists", 6)] {
                for index in 0..count {
                    for y in [-2000.0, 0.0, 2000.0] {
                        this.chrome = serde_json::from_value(layout.clone()).unwrap();
                        let parent = this.chrome.find(container).unwrap();
                        let a = &parent.children()[index];
                        let b = &parent.children()[index + 1];
                        let bounds = this.bounds.borrow();
                        let total = f32::from(bounds[&a.id].size.height + bounds[&b.id].size.height) / super::settings::zoom();
                        drop(bounds);
                        let before = serde_json::to_value(parent).unwrap();
                        this.drag = Some(super::Drag { container: container.into(), index, axis: super::Axis::Column });
                        this.drag_to(point(px(100.0), px(y)));
                        let after = this.chrome.find(container).unwrap();
                        if container == "sidebar" || total <= 48.0 {
                            assert_eq!(serde_json::to_value(after).unwrap(), before, "{container}[{index}], height {height}");
                        } else {
                            for child in &after.children()[index..=index + 1] {
                                if let Some(super::Span::Px { px }) = child.size {
                                    assert!(px >= 0.0 && px <= total, "{}: {px} out of {total}", child.id);
                                }
                            }
                        }
                    }
                }
            }
            this.drag = None;
        });
    }
}

#[test]
fn a_press_is_a_click_until_it_moves_three_pixels() {
    in_own_process("canvas_drag_threshold");
}

#[gpui::test]
fn canvas_drag_threshold(cx: &mut gpui::TestAppContext) {
    use serde_json::json;
    if !running_alone("canvas_drag_threshold") { return; }
    let (view, cx) = open_window(cx, "probe", "", json!({"id":"root","type":"row","children":[]}));
    view.update_in(cx, |this, window, cx| {
        this.canvas_state.doc.panes = serde_json::from_value(json!([
            {"id":"t1","kind":"terminal","props":{"size":[600,400]},"rect":[0,0,600,400],"z":0},
            {"id":"t2","kind":"terminal","props":{"size":[600,400]},"rect":[700,0,600,400],"z":1}
        ])).unwrap();
        for t in ["t1", "t2"] { this.live.insert(t.into(), json!({})); }
        this.tile_canvas(true, window, cx);
        this.canvas_state.settle_for_test();
        let tiled: Vec<_> = this.canvas_state.doc.panes.iter().map(|c| c.rect).collect();
        // Jitter stays a click: tile view stays on.
        assert!(!this.canvas_state.drag_live([2., 2.]) && this.canvas_state.tile.is_some());
        // Past 3 px it is a drag: tile view ends and the tiled rects stay as the free layout.
        assert!(this.canvas_state.drag_live([3., 1.]) && this.canvas_state.tile.is_none());
        assert!(this.canvas_state.drag_live([0., 0.]), "it stays a drag until release");
        let free: Vec<_> = this.canvas_state.doc.panes.iter().map(|c| c.rect).collect();
        assert_eq!(free, tiled);
        this.canvas_state.end_drag();
        assert!(!this.canvas_state.drag_live([0., 0.]), "a new press starts as a click");
    });
}
