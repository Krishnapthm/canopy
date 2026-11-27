use super::traits::Traits;

/// Where a pane is shown. Derived each time from window state, never stored.
pub enum Placement {
    Canvas { entered: bool },
    Docked,
    Popped,
}

/// Who a pinch over the pane zooms.
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
}
