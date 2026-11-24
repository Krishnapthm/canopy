// Modified for Canopy.
//! Live settings and the resolved layout look, including persisted app zoom.
//!
//! A theme is a palette plus optional color overrides: a built-in preset
//! (Canopy Dark/Light, Herdr Dark/Light, ink, moss) or `themes/<name>.json`. Chrome colors are
//! derived from the palette (dark or light by its background), so every
//! theme is consistent and overrides only need what they change.

use crate::vt::Rgb;
use gpui::{Hsla, Pixels, Rgba, px};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Mutex;

/// Terminal colors before chrome derivation and overrides.
#[derive(Clone)]
struct Palette {
    background: u32,
    foreground: u32,
    selection: u32,
    cursor: Option<u32>,
    ansi: [u32; 16],
    /// working, blocked, done: the presets pick their own; theme files take
    /// them from ansi (yellow, red, blue). Idle is grey: it needs nobody.
    status: Option<[u32; 3]>,
}

const DARK_STATUS: Option<[u32; 3]> = Some([0xe5c07b, 0xf0705f, 0x76a8ff]);

const INK: Palette = Palette {
    background: 0x111720,
    foreground: 0xcdd6e4,
    selection: 0x344861,
    cursor: None,
    status: DARK_STATUS,
    ansi: [
        0x414b5a, 0xe58c9a, 0x89c5aa, 0xd9be82, 0x91b6ee, 0xb9a0e2, 0x85c4d0, 0xcdd6e4, 0x7f91a7, 0xf5a5b2, 0xa5dec3, 0xedd39c, 0xaccbfa,
        0xd0b7f4, 0xa0dce5, 0xf0f5fc,
    ],
};
const MOSS: Palette = Palette {
    background: 0x151c19,
    foreground: 0xcdd7ce,
    selection: 0x354a40,
    cursor: None,
    status: DARK_STATUS,
    ansi: [
        0x435248, 0xdf978d, 0xa0c4a2, 0xd4bc8b, 0x99b9cd, 0xbba4c8, 0x8bbdb5, 0xcdd7ce, 0x82968a, 0xf1afa3, 0xb8dab9, 0xe9d2a5, 0xb1d0e2,
        0xd1bddc, 0xa4d6ca, 0xeff5ee,
    ],
};

pub const PRESETS: &[&str] = &["canopy-dark", "canopy-light", "herdr-dark", "herdr-light", "ink", "moss"];

/// The theme a layout without one shows, by appearance.
pub const DEFAULT_DARK: &str = "canopy-dark";
pub const DEFAULT_LIGHT: &str = "canopy-light";

fn preset(name: &str) -> Option<&'static Palette> {
    match name {
        "ink" => Some(&INK),
        "moss" => Some(&MOSS),
        _ => None,
    }
}

/// The shipped theme files (defaults/themes), written to `themes/` when missing.
const THEMES: &[(&str, &str)] = &[
    ("atom-one-dark", include_str!("../defaults/themes/atom-one-dark.json")),
    ("win98", include_str!("../defaults/themes/win98.json")),
];
#[cfg(test)]
pub const SHIPPED_THEMES: &[&str] = &["atom-one-dark", "win98"];

/// `themes/<name>.json`: a palette, plus `colors` overrides like settings.json's.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeFile {
    background: String,
    foreground: String,
    selection: String,
    cursor: Option<String>,
    ansi: [String; 16],
    #[serde(default)]
    colors: BTreeMap<String, String>,
}

fn theme_path(name: &str) -> std::path::PathBuf {
    crate::mods::app_dir().join("themes").join(format!("{name}.json"))
}

// Only these exact shipped versions may be replaced; personal themes stay intact.
const PREVIOUS_THEMES: &[(&str, &str)] = &[
    ("atom-one-dark", r##"{
  "background": "#21252b",
  "foreground": "#abb2bf",
  "selection": "#323844",
  "cursor": "#abb2bf",
  "ansi": [
    "#21252b", "#e06c75", "#98c379", "#e5c07b", "#61afef", "#c678dd", "#56b6c2", "#abb2bf",
    "#767676", "#e06c75", "#98c379", "#e5c07b", "#61afef", "#c678dd", "#56b6c2", "#abb2bf"
  ]
}
"##),
    ("win98", r##"{
  "background": "#ffffff",
  "foreground": "#c0c0c0",
  "selection": "#000080",
  "cursor": "#c0c0c0",
  "ansi": [
    "#000000", "#aa0000", "#00aa00", "#aa5500", "#0000aa", "#aa00aa", "#00aaaa", "#aaaaaa",
    "#555555", "#ff5555", "#55ff55", "#ffff55", "#5555ff", "#ff55ff", "#55ffff", "#ffffff"
  ],
  "colors": {
    "foreground": "#000000",
    "surface": "#c0c0c0",
    "popover": "#c0c0c0",
    "raised": "#dfdfdf",
    "hover": "#dfdfdf",
    "border": "#808080",
    "input": "#ffffff",
    "muted": "#404040",
    "dim": "#808080",
    "accent": "#000080",
    "accent_foreground": "#ffffff",
    "ring": "#000000",
    "working": "#808000",
    "blocked": "#ff0000",
    "done": "#0000ff",
    "idle": "#808080",
    "unknown": "#808080"
  }
}
"##),
];

pub fn install_themes() {
    let _ = std::fs::create_dir_all(crate::mods::app_dir().join("themes"));
    for (name, text) in THEMES {
        install_theme(&theme_path(name), name, text);
    }
}

fn install_theme(path: &std::path::Path, name: &str, text: &str) {
    let previous = PREVIOUS_THEMES.iter().find(|(n, _)| *n == name).map(|(_, s)| *s);
    if !path.exists() || std::fs::read_to_string(path).ok().as_deref().is_some_and(|s| Some(s) == previous) {
        let _ = crate::persist::write_atomic(path, text.as_bytes());
    }
}

/// A theme by name: a preset, or `themes/<name>.json`.
fn palette(name: &str) -> Result<(Palette, BTreeMap<String, String>), String> {
    let builtin = match name {
        "canopy-dark" => Some(include_str!("../defaults/themes/canopy-dark.json")),
        "canopy-light" => Some(include_str!("../defaults/themes/canopy-light.json")),
        "herdr-dark" | "graphite" => Some(include_str!("../defaults/themes/herdr-dark.json")),
        "herdr-light" | "paper" => Some(include_str!("../defaults/themes/herdr-light.json")),
        _ => None,
    };
    if let Some(text) = builtin {
        return parse_theme_file(name, text);
    }
    if let Some(p) = preset(name) {
        return Ok((p.clone(), BTreeMap::new()));
    }
    let path = theme_path(name);
    let text = std::fs::read_to_string(&path)
        .map_err(|_| format!("theme {name:?} is not one of {} and there is no themes/{name}.json", PRESETS.join(", ")))?;
    parse_theme_file(name, &text)
}

fn parse_theme_file(name: &str, text: &str) -> Result<(Palette, BTreeMap<String, String>), String> {
    let f: ThemeFile = serde_json::from_str(text).map_err(|e| format!("themes/{name}.json: {e}"))?;
    let c = |k: &str, v: &str| {
        let h = v.trim().trim_start_matches('#');
        (h.len() == 6).then(|| u32::from_str_radix(h, 16).ok()).flatten().ok_or(format!("themes/{name}.json: {k} {v:?} is not #rrggbb"))
    };
    let mut ansi = [0; 16];
    for (i, v) in f.ansi.iter().enumerate() {
        ansi[i] = c(&format!("ansi[{i}]"), v)?;
    }
    for (k, v) in &f.colors {
        if parse_hex(v).is_none() {
            return Err(format!("themes/{name}.json: colors.{k}: {v:?} is not #rrggbb or #rrggbbaa"));
        }
    }
    let cursor = f.cursor.as_deref().map(|v| c("cursor", v)).transpose()?;
    let p = Palette {
        background: c("background", &f.background)?,
        foreground: c("foreground", &f.foreground)?,
        selection: c("selection", &f.selection)?,
        cursor,
        ansi,
        status: None,
    };
    Ok((p, f.colors))
}

#[test]
fn shipped_theme_upgrades_preserve_personal_edits() {
    let root = std::env::temp_dir().join(format!("herdr-theme-upgrades-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    for (name, previous) in PREVIOUS_THEMES {
        let path = root.join(format!("{name}.json"));
        let current = THEMES.iter().find(|(n, _)| n == name).unwrap().1;
        install_theme(&path, name, current);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), current);
        std::fs::write(&path, previous).unwrap();
        install_theme(&path, name, current);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), current);
        let edited = previous.replace("#abb2bf", "#abcdef").replace("#c0c0c0", "#abcdef");
        std::fs::write(&path, &edited).unwrap();
        install_theme(&path, name, current);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), edited);
        std::fs::remove_file(&path).unwrap();
    }
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn shipped_theme_files() {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("defaults/themes");
    let mut found = std::collections::BTreeSet::new();
    for entry in std::fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") { continue; }
        let name = path.file_stem().unwrap().to_str().unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        parse_theme_file(name, &text).unwrap_or_else(|e| panic!("{e}"));
        // Built-in presets are compiled in (see `palette`), not installed into themes/.
        if PRESETS.contains(&name) { continue; }
        assert_eq!(THEMES.iter().find(|(n, _)| *n == name).unwrap().1, text);
        found.insert(name.to_owned());
    }
    assert_eq!(found, THEMES.iter().map(|(n, _)| n.to_string()).collect());
    assert_eq!(found, SHIPPED_THEMES.iter().map(|n| n.to_string()).collect());
}

/// Whether `name` is a theme that can be shown.
pub fn theme_exists(name: &str) -> bool {
    palette(name).is_ok()
}

/// Whether a theme is light (by its background), for picking its half of a pair.
pub fn theme_is_light(name: &str) -> bool {
    palette(name).map(|(p, _)| is_light(p.background)).unwrap_or(false)
}

fn is_light(c: u32) -> bool {
    let ch = |s: u32| ((c >> s) & 255) as f32 / 255.0;
    0.2126 * ch(16) + 0.7152 * ch(8) + 0.0722 * ch(0) > 0.5
}

/// Every theme that can be shown, presets first, each with its colors
/// (the settings screen draws them as swatches).
pub fn themes() -> Vec<(String, Theme)> {
    let mut files: Vec<String> = std::fs::read_dir(crate::mods::app_dir().join("themes"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.strip_suffix(".json").map(str::to_owned))
        .filter(|n| !PRESETS.contains(&n.as_str()))
        .collect();
    files.sort();
    PRESETS
        .iter()
        .map(|n| n.to_string())
        .chain(files)
        .filter_map(|n| palette(&n).ok().map(|(p, o)| (n, resolve(&p, &o))))
        .collect()
}

fn hex(c: u32) -> String {
    format!("#{c:06x}")
}

fn mix(a: u32, b: u32, t: f32) -> u32 {
    [16, 8, 0].iter().fold(0, |acc, s| {
        let (x, y) = (((a >> s) & 255) as f32, ((b >> s) & 255) as f32);
        acc | (((x * (1.0 - t) + y * t).round() as u32) << s)
    })
}

/// Every color the app and mods use, resolved. Values are `#rrggbb` or `#rrggbbaa`.
pub type Theme = BTreeMap<String, String>;

/// The panel colors around a palette. `chrome` (sidebars, bars) and
/// `canvas` (behind cards) step away from the terminal's background:
/// darker on a dark theme, toward warm grey on a light one.
fn resolve(p: &Palette, overrides: &BTreeMap<String, String>) -> Theme {
    let bg = p.background;
    let light = is_light(bg);
    let fg = if light { 0x1b1b1e } else { 0xeeeeef };
    let [working, blocked, done] = p.status.unwrap_or([p.ansi[3], p.ansi[1], p.ansi[4]]);
    let (popover, raised, hover, selected, border, input, canvas, chrome) = if light {
        let warm = 0xe8e8e2;
        (hex(bg), hex(mix(bg, 0, 0.035)), "#0000000c".into(), "#00000012".into(), "#00000017".into(), "#00000026".into(), mix(bg, warm, 0.2), mix(bg, warm, 0.6))
    } else {
        (
            hex(mix(bg, 0xffffff, 0.06)),
            hex(mix(bg, 0xffffff, 0.09)),
            "#ffffff0e".into(),
            "#ffffff17".into(),
            "#ffffff18".into(),
            "#ffffff24".into(),
            mix(bg, 0, 0.16),
            mix(bg, 0, 0.32),
        )
    };
    let mut t: Theme = [
        ("background", hex(bg)),
        ("terminal_foreground", hex(p.foreground)),
        ("foreground", hex(fg)),
        ("surface", hex(bg)),
        ("ring", hex(p.ansi[4])),
        ("muted-foreground", hex(mix(bg, fg, if light { 0.66 } else { 0.6 }))),
        ("blue", hex(p.ansi[4])),
        ("gray", hex(mix(bg, fg, 0.65))),
        ("red", hex(p.ansi[1])),
        ("orange", hex(mix(p.ansi[1], p.ansi[3], 0.5))),
        ("yellow", hex(p.ansi[3])),
        ("green", hex(p.ansi[2])),
        ("teal", hex(p.ansi[6])),
        ("purple", hex(p.ansi[5])),
        ("canvas", hex(canvas)),
        ("chrome", hex(chrome)),
        ("popover", popover),
        ("raised", raised),
        ("hover", hover),
        ("selected", selected),
        ("border", border),
        ("input", input),
        ("muted", hex(mix(bg, fg, if light { 0.66 } else { 0.6 }))),
        ("dim", hex(mix(bg, fg, if light { 0.46 } else { 0.38 }))),
        ("accent", hex(p.ansi[4])),
        ("accent_foreground", hex(bg)),
        ("solid", hex(fg)),
        ("solid_foreground", hex(if light { 0xffffff } else { mix(bg, 0, 0.28) })),
        ("ring", hex(mix(bg, fg, 0.7))),
        ("selection", hex(p.selection)),
        ("cursor", hex(p.cursor.unwrap_or(p.foreground))),
        ("working", hex(working)),
        ("blocked", hex(blocked)),
        ("done", hex(done)),
        ("idle", hex(mix(bg, fg, 0.38))),
        ("unknown", hex(mix(bg, fg, 0.38))),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect();
    for (i, c) in p.ansi.iter().enumerate() {
        t.insert(format!("ansi{i}"), hex(*c));
    }
    for (k, v) in overrides {
        t.insert(k.clone(), v.clone());
    }
    let (toward, amount) = if light { (0, 0.12) } else { (0xffffff, 0.45) };
    for hue in crate::canvas::Hue::ALL {
        let key = hue.tint_token();
        if !t.contains_key(&key) {
            let Rgb(r, g, b) = rgb(&t[hue.name()]);
            let base = u32::from_be_bytes([0, r, g, b]);
            t.insert(key, hex(mix(base, toward, amount)));
        }
    }
    t
}

// One table drives defaults, the CLI reference and the guide's completeness check.
macro_rules! person_settings {
    ($( $(#[$attr:meta])* $key:ident: $ty:ty = $default:expr => $help:literal; )*) => {
        /// Preferences in settings.json, used where a layout leaves its look unset.
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
        #[serde(default)]
        pub struct Person { $( #[doc = $help] $(#[$attr])* pub $key: $ty, )* }
        impl Default for Person {
            fn default() -> Self { Self { $( $key: $default, )* } }
        }
        const SETTING_KEYS: &[&str] = &[$(stringify!($key)),*];
        /// Every setting's default and explanation, without reading the person's files.
        pub fn config() -> String {
            let person = Person::default();
            let mut out = String::new();
            $( out.push_str(&format!("{} = {}  {}\n", stringify!($key), serde_json::to_string(&person.$key).unwrap(), $help)); )*
            out
        }
    }
}

person_settings! {
    #[serde(skip_serializing_if = "is_true")]
    confirm_close_terminal: bool = true => "Ask before closing a plain terminal cell.";
    canvas_color: Option<String> = None => "Canvas background; null follows the theme.";
    canvas_gap: f32 = 24. => "Cell and space gap in world pixels (0..1024).";
    tile_gap: f32 = 0. => "Gap between cells in tile view in world pixels (0..1024).";
    tile_focus_grows: bool = false => "In tile view the selected tile takes more room; off keeps every tile the same size.";
    canvas_push: CanvasPush = CanvasPush::Live => "Move neighbours live, only on drop, or never so cells overlap (live, drop, off).";
    #[serde(skip_serializing_if = "KeepCookies::is_until_quit")]
    browser_keep_cookies: KeepCookies = KeepCookies::UntilQuit => "Browser pane cookies and site data: until_quit (deleted when Canopy quits or starts) or forever.";
    #[serde(skip_serializing_if = "OpenLinks::is_built_in")]
    browser_open_links: OpenLinks = OpenLinks::BuiltIn => "Where HTTP and HTTPS links clicked in terminals open: built_in (a new browser tab) or default (the system browser).";
    canvas_resize_terminals: bool = true => "Refit a canvas terminal's grid to its cell as the cell resizes (no longer in Settings; false keeps the grid and scales it).";
    space_fill: f32 = 0.04 => "Space card tint on Home (0..1).";
    terminal_size: [u16; 2] = [100, 30] => "New expanded terminal columns and rows (20..500, 5..200).";
    zoom: f32 = 1.0 => "Whole app scale (0.5..3).";
    #[serde(skip_serializing_if = "is_true")]
    run_in_background: bool = true => "On macOS, hide Canopy on Quit and keep it running in the menu bar.";
    desktop_notifications: bool = true => "Show macOS notifications for agent alerts when Canopy is in the background.";
    sounds: Sounds = Sounds::default() => "Attention sounds: enabled, done and blocked file overrides.";
    #[serde(skip_serializing_if = "Option::is_none")]
    core: Option<String> = None => "Core binary override, read at startup.";
    #[serde(skip_serializing_if = "Option::is_none")]
    font: Option<String> = None => "Terminal font where the layout leaves it unset.";
    #[serde(skip_serializing_if = "Option::is_none")]
    font_size: Option<f32> = None => "Terminal font size where the layout leaves it unset.";
    font_ligatures: bool = false => "Enable programming ligatures.";
    terminal_line_height: f32 = 1.0 => "Terminal row height multiplier (0.8..2).";
    terminal_scroll_speed: f32 = 1.0 => "Terminal scroll multiplier (0.1..5).";
    #[serde(skip_serializing_if = "Option::is_none")]
    ui_font: Option<String> = None => "Panel font where the layout leaves it unset.";
    #[serde(skip_serializing_if = "is_true")]
    clipboard_write: bool = true => "Allow terminal programs to copy text locally.";
    #[serde(skip_serializing_if = "is_true")]
    pane_scrollbars: bool = true => "Show terminal scrollback thumbs.";
    minimum_contrast: f32 = 1.0 => "Terminal text contrast threshold (1..21).";
    #[serde(skip_serializing_if = "OptionAsAlt::is_auto")]
    option_as_alt: OptionAsAlt = OptionAsAlt::Auto => "macOS Option behavior: auto, off, on, left, right.";
    #[serde(skip_serializing_if = "Appearance::is_system")]
    appearance: Appearance = Appearance::System => "Follow system appearance, or force light or dark.";
    #[serde(skip_serializing_if = "Option::is_none")]
    theme: Option<crate::layout::ThemeChoice> = None => "Theme name or light/dark names; null follows the layout.";
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    colors: BTreeMap<String, String> = BTreeMap::new() => "Theme color overrides by name.";
}

/// When neighbours move while a canvas cell is manipulated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CanvasPush {
    #[default]
    Live,
    Drop,
    Off,
}

/// How long browser panes keep cookies and website data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeepCookies {
    #[default]
    UntilQuit,
    Forever,
}

impl KeepCookies {
    fn is_until_quit(&self) -> bool {
        *self == Self::UntilQuit
    }
}

/// Where Canopy opens links.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenLinks {
    #[default]
    BuiltIn,
    Default,
}

impl OpenLinks {
    fn is_built_in(&self) -> bool {
        *self == Self::BuiltIn
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

impl Appearance {
    fn is_system(&self) -> bool {
        *self == Appearance::System
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OptionAsAlt {
    #[default]
    Auto,
    Off,
    On,
    Left,
    Right,
}

impl OptionAsAlt {
    fn is_auto(&self) -> bool {
        *self == Self::Auto
    }
}

fn is_true(b: &bool) -> bool {
    *b
}

/// What the app runs with: the shown layout's look over the defaults, at
/// the person's zoom. Everything reads this one value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Cell and space gap in world pixels.
    pub canvas_gap: f32,
    /// Whether neighbours move during a drag or on release.
    pub canvas_push: CanvasPush,
    pub browser_keep_cookies: KeepCookies,
    pub browser_open_links: OpenLinks,
    /// Whether canvas terminals refit their grid to the cell.
    pub canvas_resize_terminals: bool,
    /// Space background opacity.
    pub space_fill: f32,
    /// Columns and rows of a newly expanded terminal.
    pub terminal_size: [u16; 2],
    pub font_family: String,
    pub font_size: f32,
    pub font_ligatures: bool,
    /// Cell height multiplier, 0.8..2.
    pub terminal_line_height: f32,
    /// Wheel and trackpad scroll multiplier, 0.1..5.
    pub terminal_scroll_speed: f32,
    pub ui_font_family: String,
    pub ui_font_size: f32,
    /// How big the whole app is drawn (1 = as designed). The zoom keys change it.
    pub zoom: f32,
    /// The theme showing: a preset or a file in themes/.
    pub theme: String,
    /// The layout's theme for each appearance (the same one twice when it names one).
    pub theme_light: String,
    pub theme_dark: String,
    /// Whether the window is dark now (the person's `appearance`, else the system's).
    pub dark: bool,
    /// Any theme color by name (see `theme()` in herdr.d.ts), `#rrggbb` or `#rrggbbaa`.
    pub colors: BTreeMap<String, String>,
    /// A draggable scroll position thumb at a terminal pane's right edge.
    pub pane_scrollbars: bool,
    /// Space between a terminal's edge and its text, in px: [x, y].
    pub terminal_padding: [f32; 2],
    /// 0..1: below 1, what's behind a terminal shows through.
    pub terminal_opacity: f32,
    pub clipboard_write: bool,
    pub minimum_contrast: f32,
    pub option_as_alt: OptionAsAlt,
    pub sounds: Sounds,
    pub desktop_notifications: bool,
}

/// Attention sounds: when an agent needs you, and when one finished while you
/// weren't looking. `done` / `blocked`: an MP3 file instead of the bundled one
/// (absolute, or relative to ~/.config/canopy).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Sounds {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked: Option<String>,
}

impl Default for Sounds {
    fn default() -> Self {
        Sounds { enabled: true, done: None, blocked: None }
    }
}

impl Default for Settings {
    fn default() -> Self {
        let person = Person::default();
        Settings {
            canvas_gap: person.canvas_gap,
            canvas_push: person.canvas_push,
            browser_keep_cookies: person.browser_keep_cookies,
            browser_open_links: person.browser_open_links,
            canvas_resize_terminals: person.canvas_resize_terminals,
            space_fill: person.space_fill,
            terminal_size: person.terminal_size,
            font_family: DEFAULT_FONT.into(),
            font_size: 14.0,
            font_ligatures: false,
            terminal_line_height: 1.0,
            terminal_scroll_speed: 1.0,
            ui_font_family: "Geist".into(),
            ui_font_size: 13.0,
            zoom: 1.0,
            theme: DEFAULT_DARK.into(),
            theme_light: DEFAULT_LIGHT.into(),
            theme_dark: DEFAULT_DARK.into(),
            dark: true,
            colors: BTreeMap::new(),
            pane_scrollbars: true,
            terminal_padding: [10.0, 6.0],
            terminal_opacity: 1.0,
            clipboard_write: true,
            minimum_contrast: 1.0,
            option_as_alt: OptionAsAlt::Auto,
            sounds: Sounds::default(),
            desktop_notifications: true,
        }
    }
}

impl Settings {
    pub fn compose(person: &Person, look: &crate::layout::Look, system_dark: bool) -> Settings {
        let d = Settings::default();
        let mut colors = person.colors.clone();
        colors.extend(look.colors.clone().unwrap_or_default());
        let dark = match person.appearance {
            Appearance::System => system_dark,
            Appearance::Light => false,
            Appearance::Dark => true,
        };
        let (theme_light, theme_dark) = match look.theme.as_ref().or(person.theme.as_ref()) {
            Some(c) => (c.pick(false).to_owned(), c.pick(true).to_owned()),
            None => (DEFAULT_LIGHT.to_owned(), DEFAULT_DARK.to_owned()),
        };
        Settings {
            canvas_gap: person.canvas_gap,
            canvas_push: person.canvas_push,
            browser_keep_cookies: person.browser_keep_cookies,
            browser_open_links: person.browser_open_links,
            canvas_resize_terminals: person.canvas_resize_terminals,
            space_fill: person.space_fill,
            terminal_size: person.terminal_size,
            // The layout's own, else the person's, else Canopy's.
            font_family: look.font.clone().or(person.font.clone()).unwrap_or(d.font_family),
            font_size: look.font_size.or(person.font_size).unwrap_or(d.font_size),
            font_ligatures: person.font_ligatures,
            terminal_line_height: look.terminal_line_height.unwrap_or(person.terminal_line_height).clamp(0.8, 2.0),
            terminal_scroll_speed: look.terminal_scroll_speed.unwrap_or(person.terminal_scroll_speed).clamp(0.1, 5.0),
            ui_font_family: look.ui_font.clone().or(person.ui_font.clone()).unwrap_or(d.ui_font_family),
            ui_font_size: d.ui_font_size,
            zoom: person.zoom,
            theme: if dark { theme_dark.clone() } else { theme_light.clone() },
            theme_light,
            theme_dark,
            dark,
            colors,
            pane_scrollbars: person.pane_scrollbars,
            terminal_padding: look.terminal_padding.unwrap_or(d.terminal_padding),
            terminal_opacity: look.terminal_opacity.unwrap_or(1.0).clamp(0.0, 1.0),
            sounds: person.sounds.clone(),
            desktop_notifications: person.desktop_notifications,
            clipboard_write: person.clipboard_write,
            minimum_contrast: person.minimum_contrast,
            option_as_alt: person.option_as_alt,
        }
    }
}

pub fn parse_hex(s: &str) -> Option<Hsla> {
    let h = s.trim().trim_start_matches('#');
    let v = u32::from_str_radix(h, 16).ok()?;
    let (rgb, a) = match h.len() {
        6 => (v, 255),
        8 => (v >> 8, v & 255),
        _ => return None,
    };
    Some(
        Rgba {
            r: ((rgb >> 16) & 255) as f32 / 255.0,
            g: ((rgb >> 8) & 255) as f32 / 255.0,
            b: (rgb & 255) as f32 / 255.0,
            a: a as f32 / 255.0,
        }
        .into(),
    )
}

pub fn rgb(s: &str) -> Rgb {
    let h = s.trim().trim_start_matches('#');
    let v = u32::from_str_radix(&h[..h.len().min(6)], 16).unwrap_or(0xff00ff);
    Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// Flattens a terminal color onto its opaque background.
pub fn rgb_on(s: &str, background: Rgb) -> Rgb {
    let Some(color) = parse_hex(s) else { return rgb(s); };
    let color: Rgba = color.into();
    let blend = |foreground: f32, background: u8| (foreground * 255.0 * color.a + background as f32 * (1.0 - color.a)).round() as u8;
    Rgb(blend(color.r, background.0), blend(color.g, background.1), blend(color.b, background.2))
}

impl Settings {
    /// Problems an agent should hear about before the file is applied.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        for name in [&self.theme_light, &self.theme_dark] {
            if let Err(e) = palette(name) {
                out.push(e);
            }
        }
        out.dedup();
        for (k, v) in &self.colors {
            if parse_hex(v).is_none() {
                out.push(format!("colors.{k}: {v:?} is not #rrggbb or #rrggbbaa"));
            }
        }
        if !(1.0..=21.0).contains(&self.minimum_contrast) {
            out.push(format!("minimum_contrast {} is outside 1..21", self.minimum_contrast));
        }
        if !(6.0..=72.0).contains(&self.font_size) {
            out.push(format!("font_size {} is outside 6..72", self.font_size));
        }
        out
    }
}

pub fn path() -> std::path::PathBuf {
    crate::mods::app_dir().join("settings.json")
}

thread_local! {
    static CURRENT: RefCell<(Rc<Settings>, Rc<Theme>)> =
        RefCell::new((Rc::new(Settings::default()), Rc::new({ let (p, o) = palette(DEFAULT_DARK).expect("built-in dark theme"); resolve(&p, &o) })));
}

const ZOOM_MIN: f32 = 0.5;
const ZOOM_MAX: f32 = 3.0;

pub fn get() -> Rc<Settings> {
    CURRENT.with(|c| c.borrow().0.clone())
}

/// The resolved theme: every color by name.
pub fn theme() -> Rc<Theme> {
    CURRENT.with(|c| c.borrow().1.clone())
}

/// A theme color as GPUI paint (magenta if missing, so mistakes show).
pub fn color(name: &str) -> Hsla {
    theme().get(name).and_then(|v| parse_hex(v)).unwrap_or(gpui::hsla(0.83, 1.0, 0.5, 1.0))
}

/// A theme color name, or a literal `#rrggbb[aa]`.
pub fn color_or_literal(v: &str) -> Hsla {
    parse_hex(v).unwrap_or_else(|| color(v))
}

/// Reads settings.json (writing the defaults if it's missing), or says why it can't be used.
pub fn load_person() -> Result<Person, String> {
    let p = path();
    if !p.exists() {
        let _ = crate::persist::write_atomic(&p, serde_json::to_string_pretty(&Person::default()).unwrap() + "\n");
    }
    let text = std::fs::read_to_string(&p).map_err(|e| e.to_string())?;
    parse_person(&text)
}

fn parse_person(text: &str) -> Result<Person, String> {
    let person: Person = serde_json::from_str(text).map_err(|e| json_error("settings.json", e))?;
    for (valid, problem) in [
        ((ZOOM_MIN..=ZOOM_MAX).contains(&person.zoom), "zoom must be in 0.5..3"),
        ((0.0..=1024.0).contains(&person.canvas_gap), "canvas_gap must be in 0..1024"),
        ((0.0..=1024.0).contains(&person.tile_gap), "tile_gap must be in 0..1024"),
        ((0.0..=1.0).contains(&person.space_fill), "space_fill must be in 0..1"),
        ((20..=500).contains(&person.terminal_size[0]) && (5..=200).contains(&person.terminal_size[1]), "terminal_size must be [columns 20..500, rows 5..200]"),
    ] {
        if !valid { return Err(format!("settings.json: {problem}")); }
    }
    Ok(person)
}

/// Settings removed from the app, still allowed in old files without a warning.
const RETIRED_KEYS: &[&str] = &["wire_limit", "wire_budget"];

/// The keys of settings.json that no setting reads, each with a close spelling when there is
/// exactly one. A warning only: the rest of the file still applies.
pub fn unknown_keys(text: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let found: Vec<String> = value.as_object()?.keys().filter(|key| !SETTING_KEYS.contains(&key.as_str()) && !RETIRED_KEYS.contains(&key.as_str())).map(|key| {
        let mut close: Vec<_> = SETTING_KEYS.iter().map(|known| (distance(key, known), known)).filter(|(d, _)| *d <= 2).collect();
        close.sort();
        match close.as_slice() {
            [(d, known), rest @ ..] if rest.first().is_none_or(|(next, _)| next > d) => format!("unknown key `{key}`, did you mean `{known}`?"),
            _ => format!("unknown key `{key}`"),
        }
    }).collect();
    (!found.is_empty()).then(|| format!("settings.json: {}", found.join("; ")))
}

/// JSON diagnostics shared by the three hand-edited config files.
pub fn json_error(file: &str, error: serde_json::Error) -> String {
    format!("{file}:{}:{}: {error}", error.line(), error.column())
}

fn distance(a: &str, b: &str) -> usize {
    let mut row: Vec<_> = (0..=b.chars().count()).collect();
    for (i, x) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, y) in b.chars().enumerate() {
            let above = row[j + 1];
            row[j + 1] = (diagonal + usize::from(x != y)).min(row[j] + 1).min(above + 1);
            diagonal = above;
        }
    }
    *row.last().unwrap()
}

/// Disables terminal-close prompts while preserving other hand-edited settings.
pub fn skip_terminal_confirmation(path: &std::path::Path) -> Result<(), String> {
    // A fresh install has no settings.json yet.
    let text = match std::fs::read_to_string(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => "{}".into(),
        r => r.map_err(|e| e.to_string())?,
    };
    let mut value: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let object = value.as_object_mut().ok_or("settings.json must contain an object")?;
    object.insert("confirm_close_terminal".into(), serde_json::json!(false));
    crate::persist::write_atomic(path, serde_json::to_string_pretty(&value).unwrap() + "\n").map_err(|e| e.to_string())
}

pub fn write_person(person: &Person) -> std::io::Result<()> {
    crate::persist::write_atomic(&path(), serde_json::to_string_pretty(person).unwrap_or_default() + "\n")
}

/// Installs new settings; returns whether anything changed.
pub fn set(s: Settings) -> bool {
    #[cfg(target_os = "macos")]
    gpui_macos::set_option_as_alt(match s.option_as_alt {
        OptionAsAlt::Auto => None,
        OptionAsAlt::Off => Some(gpui_macos::OptionAsAlt::None),
        OptionAsAlt::On => Some(gpui_macos::OptionAsAlt::Both),
        OptionAsAlt::Left => Some(gpui_macos::OptionAsAlt::Left),
        OptionAsAlt::Right => Some(gpui_macos::OptionAsAlt::Right),
    });
    CURRENT.with(|c| {
        // Read the theme even when the settings are the same: its file may
        // have new colors under the same name.
        let (p, mut colors) = palette(&s.theme).unwrap_or_else(|_| palette(DEFAULT_DARK).expect("built-in dark theme"));
        colors.extend(s.colors.clone());
        let theme = resolve(&p, &colors);
        if *c.borrow().0 == s && *c.borrow().1 == theme {
            return false;
        }
        gpui_shell::set_pixel_scale(s.zoom);
        *c.borrow_mut() = (Rc::new(s), Rc::new(theme));
        true
    })
}

/// The zoom a first start picks: the system's text size (GNOME's large text,
/// Windows' "make text bigger"), a little more on a big screen at 100%
/// (logical px: a scaled screen already draws bigger), in tenths.
pub fn first_zoom(display_width: Option<f32>) -> f32 {
    let text = system_text_scale().clamp(1.0, 2.0);
    let screen = if display_width.is_some_and(|w| w >= 2400.0) { 1.15 } else { 1.0 };
    ((text * screen * 10.0).round() / 10.0).clamp(ZOOM_MIN, ZOOM_MAX)
}

fn system_text_scale() -> f32 {
    let run = |cmd: &str, args: &[&str]| -> Option<String> {
        let out = crate::spawn::command(cmd).args(args).stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    };
    if cfg!(windows) {
        // Percent, 100..225, as REG_DWORD hex.
        return run("reg", &["query", r"HKCU\Software\Microsoft\Accessibility", "/v", "TextScaleFactor"])
            .and_then(|o| o.split_whitespace().last().and_then(|h| u32::from_str_radix(h.trim_start_matches("0x"), 16).ok()))
            .map_or(1.0, |p| p as f32 / 100.0);
    }
    if cfg!(target_os = "linux") {
        return run("gsettings", &["get", "org.gnome.desktop.interface", "text-scaling-factor"]).and_then(|o| o.trim().parse().ok()).unwrap_or(1.0);
    }
    1.0
}

/// The next zoom level in `steps` tenths (0 resets).
pub fn next_zoom(steps: i32) -> f32 {
    if steps == 0 {
        return 1.0;
    }
    ((zoom() * 10.0).round() + steps as f32).clamp(ZOOM_MIN * 10.0, ZOOM_MAX * 10.0) / 10.0
}

pub fn zoom() -> f32 {
    get().zoom
}

/// A designed size in px, at the current zoom.
pub fn z(v: f32) -> Pixels {
    px(v * zoom())
}

pub fn font_size() -> f32 {
    get().font_size * zoom()
}

pub fn ui_font_size() -> f32 {
    get().ui_font_size * zoom()
}

/// Font families on this machine (the bundled ones and the home's included).
static FONTS: Mutex<Option<std::collections::HashSet<String>>> = Mutex::new(None);

pub fn note_fonts(names: Vec<String>) {
    *FONTS.lock().unwrap() = Some(names.into_iter().map(|n| n.to_lowercase()).collect());
}

/// Font files the home brings: `fonts/` and each mod's `fonts/` (a layout's
/// look travels with its mod, as Canopy 98's does). Loads the ones not loaded
/// yet; true when any was.
pub fn load_home_fonts(cx: &mut gpui::App) -> bool {
    static LOADED: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
    let home = crate::mods::app_dir();
    let mut dirs = vec![home.join("fonts")];
    dirs.extend(std::fs::read_dir(home.join("mods")).into_iter().flatten().flatten().map(|e| e.path().join("fonts")));
    let mut new = Vec::new();
    let mut loaded = LOADED.lock().unwrap();
    for dir in dirs {
        for path in std::fs::read_dir(&dir).into_iter().flatten().flatten().map(|e| e.path()) {
            let font = path.extension().and_then(|e| e.to_str()).is_some_and(|e| matches!(e.to_lowercase().as_str(), "ttf" | "otf" | "ttc"));
            if font && !loaded.contains(&path) && let Ok(bytes) = std::fs::read(&path) {
                new.push(std::borrow::Cow::Owned(bytes));
                loaded.push(path);
            }
        }
    }
    if new.is_empty() {
        return false;
    }
    if let Err(e) = cx.text_system().add_fonts(new) {
        tracing::warn!("fonts in {} not loaded: {e:#}", home.display());
    }
    FONTS_ADDED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    note_fonts(cx.text_system().all_font_names());
    true
}

static FONTS_ADDED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// How many times fonts were added while running: text shaped before may now
/// fall back to other glyphs.
pub fn fonts_added() -> u64 {
    FONTS_ADDED.load(std::sync::atomic::Ordering::Relaxed)
}

/// The terminal font, or the bundled default when this machine lacks it:
/// the platform's own fallback is proportional and breaks the grid.
pub fn terminal_font() -> String {
    let family = get().font_family.clone();
    match FONTS.lock().unwrap().as_ref() {
        Some(have) if !have.contains(&family.to_lowercase()) => DEFAULT_FONT.into(),
        _ => family,
    }
}

/// Fonts Canopy ships, so the defaults look the same everywhere.
pub const BUNDLED_FONTS: &[&[u8]] = &[
    include_bytes!("../assets/fonts/GeistMono-Variable.ttf"),
    include_bytes!("../assets/fonts/GeistMono-Italic-Variable.ttf"),
    include_bytes!("../assets/fonts/Geist.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono-Variable.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono-Italic-Variable.ttf"),
    SYMBOLS_FONT_FILE,
];

/// The symbols font itself: terminals read icons' glyphs from it directly.
pub const SYMBOLS_FONT_FILE: &[u8] = include_bytes!("../assets/fonts/SymbolsNerdFont-Regular.ttf");

/// Nerd Font icons for terminals, under any font (MIT, nerd-fonts).
pub const SYMBOLS_FONT: &str = "Symbols Nerd Font";

/// Terminals' font unless a layout or settings.json picks another.
pub const DEFAULT_FONT: &str = "JetBrains Mono";

#[cfg(test)]
mod tests {
    #[test]
    fn terminal_alpha_color_uses_its_background() {
        assert_eq!(super::rgb_on("#ededed1f", crate::vt::Rgb(12, 12, 12)), crate::vt::Rgb(39, 39, 39));
        assert_eq!(super::rgb_on("#11111112", crate::vt::Rgb(239, 239, 239)), crate::vt::Rgb(223, 223, 223));
    }

    #[test]
    fn builtin_contrast() {
        fn rgb(value: &str, bg: [f64; 3]) -> [f64; 3] {
            let hex = value.trim_start_matches('#');
            let alpha = if hex.len() == 8 { u8::from_str_radix(&hex[6..8], 16).unwrap() as f64 / 255.0 } else { 1.0 };
            std::array::from_fn(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap() as f64 / 255.0 * alpha + bg[i] * (1.0 - alpha))
        }
        fn luminance(rgb: [f64; 3]) -> f64 {
            rgb.into_iter().zip([0.2126, 0.7152, 0.0722]).map(|(c, w)| w * if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }).sum()
        }
        for (old, new) in [("graphite", "herdr-dark"), ("paper", "herdr-light")] {
            let (p, o) = palette(old).unwrap();
            let (q, r) = palette(new).unwrap();
            assert_eq!(resolve(&p, &o), resolve(&q, &r));
        }
        for name in ["canopy-dark", "canopy-light", "herdr-dark", "herdr-light"] {
            let (p, o) = palette(name).unwrap();
            let t = resolve(&p, &o);
            let mut pairs = vec![];
            for bg in ["canvas", "surface", "raised"] { pairs.push(("foreground".to_string(), bg, 4.5)); }
            for bg in ["canvas", "surface"] {
                pairs.push(("muted".to_string(), bg, 4.5));
                pairs.push(("accent".to_string(), bg, 3.0));
            }
            if [DEFAULT_DARK, DEFAULT_LIGHT].contains(&name) {
                pairs.extend(crate::canvas::Hue::ALL.map(|hue| (hue.tint_token(), "canvas", 3.0)));
            }
            for i in 0..16 { pairs.push((format!("ansi{i}"), "background", if [4,5,12,13].contains(&i) { 3.0 } else { 4.5 })); }
            for (fg, bg, min) in pairs {
                let background = rgb(&t[bg], [0.0;3]);
                let a = luminance(rgb(&t[&fg], background));
                let b = luminance(background);
                let ratio = (a.max(b) + 0.05) / (a.min(b) + 0.05);
                println!("{name} {fg}/{bg}: {ratio:.3}");
                assert!(ratio >= min, "{name} {fg}/{bg}: {ratio:.3} < {min}");
            }
        }
    }

    #[test]
    fn terminal_ligatures_default_off_and_can_be_enabled() {
        use super::*;
        let person: Person = serde_json::from_str("{}").unwrap();
        assert!(!Settings::compose(&person, &crate::layout::Look::default(), true).font_ligatures);
        assert!(!Settings::default().font_ligatures);
        let person: Person = serde_json::from_str(r#"{"font_ligatures":true}"#).unwrap();
        let saved = serde_json::to_string(&person).unwrap();
        let restored: Person = serde_json::from_str(&saved).unwrap();
        assert!(Settings::compose(&restored, &crate::layout::Look::default(), true).font_ligatures);
        assert!(serde_json::from_str::<Person>(r#"{"font_ligatures":"yes"}"#).is_err());
    }

    use super::*;

    #[test]
    fn terminal_multipliers_default_inherit_override_and_clamp() {
        let default: Person = serde_json::from_str("{}").unwrap();
        let look = crate::layout::Look::default();
        let s = Settings::compose(&default, &look, true);
        assert_eq!((s.terminal_line_height, s.terminal_scroll_speed), (1.0, 1.0));
        let person: Person = serde_json::from_str(r#"{"terminal_line_height":1.3,"terminal_scroll_speed":0.5}"#).unwrap();
        let roundtrip: Person = serde_json::from_value(serde_json::to_value(&person).unwrap()).unwrap();
        let s = Settings::compose(&roundtrip, &look, true);
        assert_eq!((s.terminal_line_height, s.terminal_scroll_speed), (1.3, 0.5));
        let look: crate::layout::Look = serde_json::from_str(r#"{"terminal_line_height":0.1,"terminal_scroll_speed":10}"#).unwrap();
        let s = Settings::compose(&person, &look, true);
        assert_eq!((s.terminal_line_height, s.terminal_scroll_speed), (0.8, 5.0));
        let person = Person { terminal_line_height: 5.0, terminal_scroll_speed: 0.0, ..default };
        let s = Settings::compose(&person, &crate::layout::Look::default(), true);
        assert_eq!((s.terminal_line_height, s.terminal_scroll_speed), (2.0, 0.1));
    }

    #[test]
    fn layout_look_wins_and_settings_fill_what_it_leaves_unset() {
        let person = Person {
            font: Some("person".into()), font_size: Some(19.0), ui_font: Some("person-ui".into()),
            theme: Some(crate::layout::ThemeChoice::One("paper".into())),
            colors: [("accent".into(), "#123456".into())].into(),
            ..Person::default()
        };
        let look = crate::layout::Look {
            font: Some("layout".into()), font_size: Some(14.0), ui_font: Some("layout-ui".into()),
            theme: Some(crate::layout::ThemeChoice::One("ink".into())),
            colors: Some([("accent".into(), "#abcdef".into()), ("text".into(), "#ffffff".into())].into()),
            ..Default::default()
        };
        let inherited = Settings::compose(&person, &crate::layout::Look::default(), true);
        assert_eq!((inherited.font_family.as_str(), inherited.font_size, inherited.ui_font_family.as_str(), inherited.theme.as_str()), ("person", 19.0, "person-ui", "paper"));
        assert_eq!(inherited.colors["accent"], "#123456");
        let own = Settings::compose(&person, &look, true);
        assert_eq!((own.font_family.as_str(), own.font_size, own.ui_font_family.as_str(), own.theme.as_str()), ("layout", 14.0, "layout-ui", "ink"));
        assert_eq!(own.colors["accent"], "#abcdef");
        assert_eq!(own.colors["text"], "#ffffff");
    }

    #[test]
    fn option_as_alt_defaults_to_auto_and_round_trips() {
        let default: Person = serde_json::from_str("{}").unwrap();
        assert_eq!(default.option_as_alt, OptionAsAlt::Auto);
        for (value, expected) in [("auto", OptionAsAlt::Auto), ("off", OptionAsAlt::Off), ("on", OptionAsAlt::On), ("left", OptionAsAlt::Left), ("right", OptionAsAlt::Right)] {
            let person: Person = serde_json::from_value(serde_json::json!({"option_as_alt": value})).unwrap();
            let settings = Settings::compose(&person, &crate::layout::Look::default(), true);
            assert_eq!(settings.option_as_alt, expected);
            assert_eq!(serde_json::to_value(settings).unwrap()["option_as_alt"], value);
        }
        assert!(serde_json::from_value::<Person>(serde_json::json!({"option_as_alt": "invalid"})).is_err());
    }

    #[test]
    fn minimum_contrast_defaults_off_and_validates_the_ratio() {
        let default: Person = serde_json::from_str("{}").unwrap();
        assert_eq!(default.minimum_contrast, 1.0);
        assert_eq!(Settings::default().minimum_contrast, 1.0);
        for ratio in [1.0, 1.6, 21.0, 0.0, 21.1] {
            let person: Person = serde_json::from_value(serde_json::json!({"minimum_contrast": ratio})).unwrap();
            let settings = Settings::compose(&person, &crate::layout::Look::default(), true);
            assert_eq!(settings.minimum_contrast, ratio);
            assert_eq!(serde_json::to_value(person).unwrap()["minimum_contrast"], serde_json::json!(ratio));
            assert_eq!(settings.problems().is_empty(), (1.0..=21.0).contains(&ratio));
        }
    }

    #[test]
    fn program_clipboard_writes_default_on_and_can_be_disabled() {
        let default: Person = serde_json::from_str("{}").unwrap();
        assert!(default.clipboard_write);
        let off: Person = serde_json::from_str(r#"{"clipboard_write":false}"#).unwrap();
        assert!(!Settings::compose(&off, &crate::layout::Look::default(), true).clipboard_write);
        assert_eq!(serde_json::to_value(off).unwrap()["clipboard_write"], false);
    }

    #[test]
    fn shipped_looks_and_theme_clicks_have_the_same_precedence() {
        use crate::layout::{Node, ThemeChoice};
        let person = Person { font_size: Some(19.0), theme: Some(ThemeChoice::One("moss".into())), ..Person::default() };
        let main: Node = serde_json::from_str(include_str!("../defaults/layouts/main.json")).unwrap();
        assert!(main.look.theme.is_none() && main.look.colors.is_none() && main.look.font.is_none() && main.look.font_size.is_none() && main.look.ui_font.is_none());
        assert_eq!(Settings::compose(&Person::default(), &main.look, true).font_size, 14.0);
        assert_eq!(main.look.terminal_padding, Some([14.0, 12.0]));
        let terminals: Node = serde_json::from_str(include_str!("../defaults/layouts/terminals.json")).unwrap();
        assert_eq!(Settings::compose(&person, &terminals.look, true).font_size, 15.0);
        let main_settings = Settings::compose(&person, &main.look, true);
        assert_eq!((main_settings.font_size, main_settings.theme.as_str()), (19.0, "moss"));
        let mut win98: Node = serde_json::from_str(include_str!("../defaults/layouts/win98.json")).unwrap();
        let win_settings = Settings::compose(&person, &win98.look, true);
        assert_eq!(win_settings.theme, "win98");
        assert_eq!(win_settings.font_family, "PxPlus IBM VGA 8x16");
        let clicks = crate::layout_sync::State { theme: Some(ThemeChoice::One("ink".into())), ..Default::default() };
        clicks.apply(&mut win98);
        assert_eq!(Settings::compose(&person, &win98.look, true).theme, "ink");
    }

    #[test]
    fn browser_preferences_validate_and_omit_defaults() {
        let defaults = parse_person("{}").unwrap();
        assert_eq!(defaults.browser_keep_cookies, KeepCookies::UntilQuit);
        assert_eq!(defaults.browser_open_links, OpenLinks::BuiltIn);
        let serialized = serde_json::to_value(defaults).unwrap();
        assert!(serialized.get("browser_keep_cookies").is_none());
        assert!(serialized.get("browser_open_links").is_none());

        let file = r#"{"browser_keep_cookies":"forever","browser_open_links":"default"}"#;
        assert_eq!(unknown_keys(file), None);
        let person = parse_person(file).unwrap();
        assert_eq!(person.browser_keep_cookies, KeepCookies::Forever);
        assert_eq!(person.browser_open_links, OpenLinks::Default);
        assert!(parse_person(r#"{"browser_keep_cookies":"session"}"#).is_err());
        assert!(parse_person(r#"{"browser_open_links":"external"}"#).is_err());
    }

    #[test]
    fn canvas_settings_validate_and_reference_every_default() {
        let defaults = parse_person("{}").unwrap();
        assert_eq!(defaults.canvas_gap, 24.);
        assert_eq!(defaults.canvas_push, CanvasPush::Live);
        assert!(defaults.canvas_resize_terminals);
        assert_eq!(parse_person(r#"{"canvas_push":"off","canvas_resize_terminals":false}"#).map(|p| (p.canvas_push, p.canvas_resize_terminals)),
            Ok((CanvasPush::Off, false)));
        assert_eq!(defaults.space_fill, 0.04);
        assert_eq!(defaults.terminal_size, [100, 30]);
        for invalid in [r#"{"canvas_gap":-1}"#, r#"{"canvas_gap":1025}"#,
            r#"{"canvas_push":"later"}"#, r#"{"space_fill":1.1}"#, r#"{"space_fill":-0.1}"#,
            r#"{"terminal_size":[0,30]}"#, r#"{"terminal_size":[100,201]}"#] {
            assert!(parse_person(invalid).unwrap_err().starts_with("settings.json:"), "{invalid}");
        }
        assert_eq!(parse_person(r#"{"canvas_push":"drop"}"#).unwrap().canvas_push, CanvasPush::Drop);
        assert_eq!(parse_person(r#"{"canvas_gab":24,"zoom":2}"#).unwrap().zoom, 2., "an unknown key never drops the rest");
        assert_eq!(unknown_keys(r#"{"canvas_gab":24}"#).unwrap(),
            "settings.json: unknown key `canvas_gab`, did you mean `canvas_gap`?");
        assert_eq!(unknown_keys(r#"{"unrelated":24}"#).unwrap(), "settings.json: unknown key `unrelated`");
        assert_eq!(unknown_keys(r#"{"fome":24}"#).unwrap(), "settings.json: unknown key `fome`");
        assert_eq!(unknown_keys(r#"{"zoom":1}"#), None);
        // Removed settings in an old file neither warn nor drop the rest.
        assert_eq!(unknown_keys(r#"{"wire_limit":500,"wire_budget":0}"#), None);
        assert_eq!(parse_person(r#"{"wire_limit":500,"wire_budget":0,"zoom":2}"#).unwrap().zoom, 2.);
        assert_eq!(distance("", "font"), 4);
        assert_eq!(distance("fönt", "font"), 1);
        let reference = config();
        assert_eq!(reference.lines().count(), SETTING_KEYS.len());
        for key in SETTING_KEYS { assert_eq!(reference.lines().filter(|l| l.starts_with(&format!("{key} = "))).count(), 1); }
        for line in reference.lines() {
            let (key, rest) = line.split_once(" = ").unwrap();
            let (default, help) = rest.split_once("  ").unwrap();
            assert!(!help.is_empty());
            parse_person(&format!("{{\"{key}\":{default}}}")).unwrap();
        }
    }

    #[test]
    fn background_mode_defaults_on_and_preserves_opt_out() {
        assert!(parse_person(r#"{"zoom":1.5}"#).unwrap().run_in_background);
        let disabled = parse_person(r#"{"run_in_background":false}"#).unwrap();
        assert!(!disabled.run_in_background);
        assert_eq!(serde_json::to_value(disabled).unwrap()["run_in_background"], false);
        assert!(parse_person(r#"{"run_in_background":"false"}"#).is_err());
    }

    /// The guide agents read (written to the home folder) explains every
    /// settings.json key.
    #[test]
    fn the_guide_explains_every_setting() {
        let keys = SETTING_KEYS;
        let guide = include_str!("../defaults/AGENTS.md");
        let section = guide.split("## The person").nth(1).and_then(|s| s.split("\n## ").next()).unwrap_or_default();
        let missing: Vec<_> = keys.iter().filter(|k| !section.contains(&format!("`{k}`"))).collect();
        assert!(missing.is_empty(), "defaults/AGENTS.md (## The person) doesn't explain: {missing:?}");
    }
}

#[cfg(test)]
mod close_tests {
    use super::*;
    #[test]
    fn disabling_close_prompts_preserves_unknown_settings_and_rejects_bad_files() {
        let path = std::env::temp_dir().join(format!("canopy-close-settings-{}.json", std::process::id()));
        std::fs::write(&path, r#"{"future_setting":42,"confirm_close_terminal":true}"#).unwrap();
        skip_terminal_confirmation(&path).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(value["future_setting"], 42);
        assert_eq!(value["confirm_close_terminal"], false);
        for invalid in ["[1,2]", "broken"] {
            std::fs::write(&path, invalid).unwrap();
            assert!(skip_terminal_confirmation(&path).is_err());
            assert_eq!(std::fs::read_to_string(&path).unwrap(), invalid);
        }
        std::fs::remove_file(&path).unwrap();
        skip_terminal_confirmation(&path).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{\n  \"confirm_close_terminal\": false\n}\n");
        std::fs::remove_file(path).unwrap();
    }
}
