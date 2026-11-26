//! `ScaledPane`: draws one pane subtree at its scale, so the whole pane (header, pills, text)
//! zooms with the canvas as one picture. GPUI has no subtree transform; like Zed's
//! `WithRemSize`, this re-renders the subtree at a different rem size instead.
//!
//! Inside a pane the rem is the UI font size times the pane's scale (`canophy::Layout`), and
//! chrome is written in `wpx(n)`: `n` px at 100%, `n * scale` px in a pane. Outside a pane the
//! window rem is the UI font size, so `wpx(n)` is exactly `n` px there and shared components can
//! use it too. Lengths written in px stay screen-fixed: overlay chrome (frame, resize grips).
use gpui::{AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement, LayoutId, Pixels, Rems, Window, px, rems};

/// `n` world px of pane chrome (see the module docs).
pub(in crate::window) fn wpx(n: f32) -> Rems { rems(n / crate::settings::ui_font_size()) }

pub(in crate::window) struct ScaledPane {
    rem: Pixels,
    child: AnyElement,
}

impl ScaledPane {
    /// `child` drawn at `scale` (the pane's `canophy::Layout::scale`).
    pub(in crate::window) fn new(scale: f32, child: impl IntoElement) -> Self {
        Self { rem: px(crate::settings::ui_font_size() * scale), child: child.into_any_element() }
    }
}

impl IntoElement for ScaledPane {
    type Element = Self;
    fn into_element(self) -> Self { self }
}

impl Element for ScaledPane {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> { None }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> { None }

    fn request_layout(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, window: &mut Window, cx: &mut App) -> (LayoutId, ()) {
        (window.with_rem_size(Some(self.rem), |window| self.child.request_layout(window, cx)), ())
    }

    fn prepaint(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, _: Bounds<Pixels>, _: &mut (), window: &mut Window, cx: &mut App) {
        window.with_rem_size(Some(self.rem), |window| { self.child.prepaint(window, cx); });
    }

    fn paint(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, _: Bounds<Pixels>, _: &mut (), _: &mut (), window: &mut Window, cx: &mut App) {
        window.with_rem_size(Some(self.rem), |window| self.child.paint(window, cx));
    }
}

#[cfg(test)]
mod tests {
    use super::{ScaledPane, wpx};
    use gpui::{Bounds, Context, Pixels, Render, TestAppContext, Window, div, prelude::*, px};

    /// A pane header built from the real chrome helpers, placed at `scale` like the canvas does.
    struct Pane(f32);
    impl Render for Pane {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let s = self.0;
            let (fg, bg) = (gpui::black(), gpui::white());
            let header = div().debug_selector(|| "header".into()).flex().items_center().w_full().h(wpx(crate::canvas::HEADER)).px(wpx(8.)).gap(wpx(6.))
                .child(super::super::strip_tab("tab", true, fg, bg).debug_selector(|| "tab".into()).w(wpx(120.)).h_full())
                .child(super::super::panel_button("close", "icons/x.svg", fg, bg).debug_selector(|| "button".into()))
                .child(div().debug_selector(|| "title".into()).text_size(wpx(13.)).child("Terminal"));
            div().size_full().child(div().absolute().left(px(40. * s)).top(px(40. * s)).w(px(400. * s)).h(px(300. * s)).child(ScaledPane::new(s, header)))
        }
    }

    fn bounds(cx: &mut TestAppContext, scale: f32) -> Vec<Bounds<Pixels>> {
        let (_, cx) = cx.add_window_view(|_, _| Pane(scale));
        cx.run_until_parked();
        ["header", "tab", "button", "title"].map(|s| cx.debug_bounds(s).unwrap_or_else(|| panic!("{s} not drawn"))).to_vec()
    }

    /// Law 1, zoom is a picture: every part of a pane at 50% is its 100% bounds halved.
    #[gpui::test]
    fn pane_chrome_at_half_zoom_is_the_full_size_picture_halved(cx: &mut TestAppContext) {
        let (full, half) = (bounds(cx, 1.), bounds(cx, 0.5));
        for (f, h) in full.iter().zip(&half) {
            let want = [f.origin.x * 0.5, f.origin.y * 0.5, f.size.width * 0.5, f.size.height * 0.5];
            let got = [h.origin.x, h.origin.y, h.size.width, h.size.height];
            for (w, g) in want.iter().zip(got) {
                assert!((f32::from(*w) - f32::from(g)).abs() <= 1., "full {f:?} half {h:?}");
            }
        }
    }
}
