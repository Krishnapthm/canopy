//! Context menus: the one typed way to ask for one. `AppWindow::open_menu` is the only opener;
//! Rust callers pass a `MenuTab`, mods call `openMenu` and the request is parsed into a
//! `MenuRequest` here, once, so an unknown tab is an error toast and never a blank popup.

use gpui::{Pixels, Point};
use serde::Deserialize;
use serde_json::Value;

/// Which page of the `menu` mod to show (its `props().tab`). Keep in step with `case` in
/// defaults/mods/menu/main.js and `MenuTab` in defaults/herdr.d.ts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MenuTab {
    /// Rows given by the caller in `target.items`, or the app menu's pages.
    Primary,
    Pane,
    Tab,
    Space,
    CanvasCard,
    CanvasColors,
    CanvasPane,
    CanvasSpace,
    CanvasEditors,
    CanvasAgents,
    CanvasEmpty,
    CanvasArrange,
    CanvasWire,
}

impl MenuTab {
    /// The mod's `props().tab` string.
    pub fn name(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Pane => "pane",
            Self::Tab => "tab",
            Self::Space => "space",
            Self::CanvasCard => "canvas-card",
            Self::CanvasColors => "canvas-colors",
            Self::CanvasPane => "canvas-pane",
            Self::CanvasSpace => "canvas-space",
            Self::CanvasEditors => "canvas-editors",
            Self::CanvasAgents => "canvas-agents",
            Self::CanvasEmpty => "canvas-empty",
            Self::CanvasArrange => "canvas-arrange",
            Self::CanvasWire => "canvas-wire",
        }
    }

    /// Opens a submenu box beside itself, so its frame is two boxes wide and flips left at the
    /// right edge. `Primary` pages swap in place instead.
    pub fn has_submenus(self) -> bool {
        match self {
            Self::Primary => false,
            Self::Pane | Self::Tab | Self::Space | Self::CanvasCard | Self::CanvasColors | Self::CanvasPane
            | Self::CanvasSpace | Self::CanvasEditors | Self::CanvasAgents | Self::CanvasEmpty
            | Self::CanvasArrange | Self::CanvasWire => true,
        }
    }

    /// The width of the stock `menu` mod's frame: one box when it has no submenus, else the mod.json width.
    pub fn width(self, frame: f32) -> f32 {
        if self.has_submenus() { frame } else { crate::window::MENU_BOX }
    }

    /// Opening it again while it is open closes it (the app menu button toggles).
    pub fn toggles(self) -> bool {
        match self {
            Self::Primary => true,
            Self::Pane | Self::Tab | Self::Space | Self::CanvasCard | Self::CanvasColors | Self::CanvasPane
            | Self::CanvasSpace | Self::CanvasEditors | Self::CanvasAgents | Self::CanvasEmpty
            | Self::CanvasArrange | Self::CanvasWire => false,
        }
    }
}

/// Where a menu opens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MenuAt {
    /// Where the pointer last went down.
    Pointer,
    /// Over the floating block under the pointer (the toolbar), see `OverlaySpec.at`.
    Above,
    At(Point<Pixels>),
}

impl From<Option<Point<Pixels>>> for MenuAt {
    fn from(p: Option<Point<Pixels>>) -> Self {
        p.map_or(Self::Pointer, Self::At)
    }
}

/// What a mod's `openMenu({ tab, target?, at? })` asks for.
#[derive(Debug, Deserialize)]
pub struct MenuRequest {
    pub tab: MenuTab,
    #[serde(default)]
    pub target: Value,
    #[serde(default)]
    pub at: ModAt,
}

/// `at` as a mod writes it.
#[derive(Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModAt {
    #[default]
    Pointer,
    Above,
}

impl From<ModAt> for MenuAt {
    fn from(at: ModAt) -> Self {
        match at {
            ModAt::Pointer => Self::Pointer,
            ModAt::Above => Self::Above,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tab_names_round_trip() {
        for tab in [MenuTab::Primary, MenuTab::CanvasCard, MenuTab::CanvasAgents, MenuTab::Tab] {
            assert_eq!(serde_json::from_value::<MenuTab>(json!(tab.name())).unwrap(), tab);
        }
    }

    #[test]
    fn request_rejects_unknown_tab_and_defaults_at() {
        assert!(serde_json::from_value::<MenuRequest>(json!({ "tab": "nope" })).is_err());
        let r: MenuRequest = serde_json::from_value(json!({ "tab": "tab", "target": { "id": "t1" } })).unwrap();
        assert_eq!((r.tab, r.at), (MenuTab::Tab, ModAt::Pointer));
        let r: MenuRequest = serde_json::from_value(json!({ "tab": "canvas-agents", "at": "above" })).unwrap();
        assert_eq!(MenuAt::from(r.at), MenuAt::Above);
    }

    #[test]
    fn primary_is_one_box_wide() {
        assert_eq!(MenuTab::Primary.width(444.0), crate::window::MENU_BOX);
        assert_eq!(MenuTab::Pane.width(444.0), 444.0);
    }
}
