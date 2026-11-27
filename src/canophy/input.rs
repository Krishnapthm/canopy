use super::traits::Traits;

/// Where a pane is shown. Derived each time from window state, never stored.
pub enum Placement {
    Canvas { entered: bool },
    Docked,
    Popped,
}

/// Who handles a pinch or wheel over the pane.
#[derive(Debug, PartialEq, Eq)]
pub enum Owner {
    Canvas,
    Content,
}

/// Law 6: a pinch zooms content with its own zoom when the person is working in it.
pub fn pinch_owner(t: &Traits, at: Placement) -> Owner {
    match at {
        Placement::Canvas { entered: false } => Owner::Canvas,
        Placement::Canvas { entered: true } | Placement::Docked | Placement::Popped => {
            if t.zooms { Owner::Content } else { Owner::Canvas }
        }
    }
}

/// Law 9: a plain wheel scrolls content when the person is working in it. Over any other pane
/// it pans the canvas. Cmd/Ctrl+wheel zooms the canvas before this is asked.
pub fn wheel_owner(t: &Traits, at: Placement) -> Owner {
    match at {
        Placement::Canvas { entered: false } => Owner::Canvas,
        Placement::Canvas { entered: true } | Placement::Docked | Placement::Popped => {
            if t.scrolls { Owner::Content } else { Owner::Canvas }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{canvas::Kind, canophy::traits};

    #[test]
    fn pinch_follows_the_content_and_its_placement() {
        let browser = traits(&Kind::Browser { tabs: vec![], active: 0, home: String::new(), viewport: None });
        assert_eq!(pinch_owner(&browser, Placement::Canvas { entered: false }), Owner::Canvas);
        for at in [Placement::Canvas { entered: true }, Placement::Docked, Placement::Popped] {
            assert_eq!(pinch_owner(&browser, at), Owner::Content);
        }
        for kind in [Kind::Pdf { path: String::new(), highlights: vec![] }, Kind::Terminal { size: Some([400., 250.]) }] {
            assert_eq!(pinch_owner(&traits(&kind), Placement::Canvas { entered: true }), Owner::Canvas);
        }
    }

    #[test]
    fn wheel_scrolls_entered_content_and_pans_otherwise() {
        let pdf = traits(&Kind::Pdf { path: String::new(), highlights: vec![] });
        assert_eq!(wheel_owner(&pdf, Placement::Canvas { entered: false }), Owner::Canvas);
        for kind in [Kind::Pdf { path: String::new(), highlights: vec![] }, Kind::File { path: String::new() }, Kind::Terminal { size: Some([400., 250.]) }] {
            assert_eq!(wheel_owner(&traits(&kind), Placement::Canvas { entered: true }), Owner::Content);
        }
        let image = traits(&Kind::Image { path: String::new() });
        assert_eq!(wheel_owner(&image, Placement::Canvas { entered: true }), Owner::Canvas);
    }
}
