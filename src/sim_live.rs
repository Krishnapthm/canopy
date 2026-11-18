//! Live sim panes (macOS): the `canopy --sim-helper` process that streams a booted simulator's
//! framebuffer and takes its input, the device bezel from Xcode's DeviceKit, and the pure math
//! that maps a pane to device points. The Objective-C half is src/sim_live.m; the pane UI is
//! src/window/canvas_view/sim.rs. See docs/panes.md and docs/canopy/harness-notes/xcode-simctl.md.
//!
//! One helper runs per sim pane that is shown and live. It hands Canopy the framebuffer IOSurface
//! once (and again only if it changes), then writes `f` per presented frame: nothing moves while
//! the simulator's screen is still.
use std::io::{BufRead, Write};
use std::process::{Child, ChildStdin, Command, Stdio};

/// What a helper reports, in order.
pub enum Event {
    /// The framebuffer, to draw with gpui's `surface()`; it stays valid and keeps changing.
    Surface(Surface),
    /// The simulator presented a frame. Coalesced: at most one waits.
    Frame,
    /// Touch and keys reach the guest.
    Input,
    /// Something works partly; the pane stays live.
    Warn(String),
    /// The helper ended; `Some` says why it failed.
    Exit(Option<String>),
}

/// A +1 CVPixelBufferRef in transit from the reader thread; `take` it on the UI thread.
pub struct Surface(usize);

// The buffer is only retained here and used by whoever takes it.
unsafe impl Send for Surface {}

impl Surface {
    pub fn take(mut self) -> core_video::pixel_buffer::CVPixelBuffer {
        use core_foundation::base::TCFType;
        let buffer = std::mem::take(&mut self.0);
        unsafe { core_video::pixel_buffer::CVPixelBuffer::wrap_under_create_rule(buffer as core_video::pixel_buffer::CVPixelBufferRef) }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        if self.0 != 0 { unsafe { core_foundation::base::CFRelease(self.0 as core_foundation::base::CFTypeRef) } }
    }
}

/// Input for the guest. Touch points are 0..1 of the screen from its top left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Input {
    Touch { phase: Phase, x: f64, y: f64 },
    /// A whole touch from one point to another over `ms`, timed by the helper; a tap when
    /// both points are the same.
    Gesture { ms: u64, from: [f64; 2], to: [f64; 2] },
    Key { usage: u32, down: bool },
    Button { page: u32, usage: u32, down: bool },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase { Start = 0, Move = 1, End = 2 }

impl Input {
    /// The helper's stdin line (see `handle` in src/sim_live.m).
    pub fn line(&self) -> String {
        let state = |down: bool| if down { 1 } else { 2 };
        match *self {
            Self::Touch { phase, x, y } => format!("t {} {:.5} {:.5}\n", phase as u8, x.clamp(0., 1.), y.clamp(0., 1.)),
            Self::Gesture { ms, from, to } => format!("g {ms} {:.5} {:.5} {:.5} {:.5}\n", from[0].clamp(0., 1.), from[1].clamp(0., 1.), to[0].clamp(0., 1.), to[1].clamp(0., 1.)),
            Self::Key { usage, down } => format!("k {usage} {}\n", state(down)),
            Self::Button { page, usage, down } => format!("b {page} {usage} {}\n", state(down)),
        }
    }
}

/// Hardware buttons by the names `canopy pane press` takes, as (HID usage page, usage); the
/// bezel's own buttons come from chrome.json.
pub const BUTTONS: &[(&str, u32, u32)] = &[
    ("home", 0x0c, 0x40), ("lock", 0x0c, 0x30), ("power", 0x0c, 0x30), ("siri", 0x0c, 0xcf),
    ("volume-up", 0x0c, 0xe9), ("volume-down", 0x0c, 0xea), ("action", 0x0b, 0x2d),
];

/// A running helper. Dropping it closes its stdin, and it exits.
pub struct Helper {
    stdin: ChildStdin,
    /// Set while a `Frame` event waits: the reader sends one only while it is false, and the
    /// pane clears it once it has asked for a redraw.
    pub frame: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Helper {
    /// Starts the helper for a booted simulator under sandbox-exec; events go to `events`.
    pub fn start(udid: &str, events: async_channel::Sender<Event>) -> Result<Self, String> {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let developer = developer_dir().ok_or("Can't find Xcode (xcode-select -p)")?;
        let exe = std::env::current_exe().map_err(|e| format!("Can't find canopy itself: {e}"))?;
        let name = format!("dev.canopy.sim.{}.{}", std::process::id(), NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
        let port = unsafe { canopy_sim_listen(cstr(&name).as_ptr()) };
        if port == 0 { return Err("Can't open a port for the simulator screen".into()); }
        let home = std::env::var("HOME").unwrap_or_default();
        let spawned = Command::new("/usr/bin/sandbox-exec")
            .arg("-p").arg(SANDBOX)
            .arg("-D").arg(format!("EXE={}", exe.display()))
            .arg("-D").arg(format!("HOME={home}"))
            .arg("-D").arg(format!("DEVELOPER={developer}"))
            .arg("-D").arg(format!("RENDEZVOUS={name}"))
            .arg(&exe).args(["--sim-helper", &developer, udid, &name])
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null())
            .spawn();
        let mut child = match spawned {
            Ok(child) => child,
            Err(e) => { unsafe { canopy_sim_close(port) }; return Err(format!("Can't start the simulator helper: {e}")); }
        };
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let frame = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let pending = frame.clone();
        std::thread::Builder::new().name("sim-helper".into()).spawn(move || read(stdout, child, port, events, pending))
            .map_err(|e| format!("Can't watch the simulator helper: {e}"))?;
        Ok(Self { stdin, frame })
    }

    /// Sends input; a helper that died reports `Exit` on its own.
    pub fn send(&mut self, input: Input) {
        let _ = self.stdin.write_all(input.line().as_bytes()).and_then(|_| self.stdin.flush());
    }
}

/// The helper's stdout, line by line, until it exits.
fn read(stdout: std::process::ChildStdout, mut child: Child, port: u32, events: async_channel::Sender<Event>, pending: std::sync::Arc<std::sync::atomic::AtomicBool>) {
    use std::sync::atomic::Ordering;
    let mut reason = None;
    for line in std::io::BufReader::new(stdout).lines() {
        let Ok(line) = line else { break };
        let event = match line.split_once(' ').unwrap_or((&line, "")) {
            ("f", _) => {
                if pending.swap(true, Ordering::AcqRel) { continue; }
                Event::Frame
            }
            ("surface", _) => {
                let buffer = unsafe { canopy_sim_take_surface(port, 2000) };
                if buffer.is_null() { reason = Some("The simulator screen did not arrive".into()); break; }
                Event::Surface(Surface(buffer as usize))
            }
            ("input", _) => Event::Input,
            ("warn", text) => Event::Warn(text.to_owned()),
            ("error", text) => { reason = Some(text.to_owned()); continue; }
            _ => continue,
        };
        if events.send_blocking(event).is_err() { break; }
    }
    let _ = child.kill();
    let status = child.wait();
    unsafe { canopy_sim_close(port) };
    if reason.is_none() && !status.is_ok_and(|s| s.success()) { reason = Some("The simulator helper stopped".into()); }
    let _ = events.send_blocking(Event::Exit(reason));
}

/// Xcode's developer folder, once per run.
pub fn developer_dir() -> Option<String> {
    static DIR: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let out = Command::new("xcode-select").arg("-p").output().ok()?;
        let dir = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        (out.status.success() && !dir.is_empty()).then_some(dir)
    }).clone()
}

/// `canopy --sim-helper <developer dir> <udid> <rendezvous>`: never returns.
pub fn helper_main(args: &[String]) -> ! {
    let [developer, udid, rendezvous] = args else {
        eprintln!("usage: canopy --sim-helper <developer dir> <udid> <rendezvous>");
        std::process::exit(2);
    };
    let code = unsafe { canopy_sim_helper(cstr(developer).as_ptr(), cstr(udid).as_ptr(), cstr(rendezvous).as_ptr()) };
    std::process::exit(code);
}

// Modeled on Claude's simulator helper: deny by default, no network, exec and reads limited to
// this binary, system libraries and Xcode, home reads only under CoreSimulator, and Mach lookups
// only for CoreSimulator and Canopy's own rendezvous. A missing rule shows up as a hang or an
// `error` line, never as a silent success.
const SANDBOX: &str = r#"(version 1)
(deny default)
(import "system.sb")
(allow process-info* (target self))
(allow process-exec (literal (param "EXE")))
(allow file-read* file-map-executable
  (literal (param "EXE"))
  (subpath "/usr/lib") (subpath "/usr/share") (subpath "/System") (subpath "/Library/Developer")
  (subpath "/Library/Apple/System") (subpath (param "DEVELOPER")) (subpath "/private/var/db/timezone")
  (literal "/private/var/db/xcode_select_link") (literal "/private/etc/localtime"))
(allow file-read* (subpath (string-append (param "HOME") "/Library/Developer/CoreSimulator")))
(allow file-read-metadata)
(allow mach-lookup
  (global-name "com.apple.CoreSimulator.CoreSimulatorService")
  (global-name (param "RENDEZVOUS")))
(allow ipc-posix-shm-read* ipc-posix-shm-write-data)
(allow iokit-open (iokit-user-client-class "IOSurfaceRootUserClient"))
"#;

/// Saves the framebuffer as a PNG.
pub fn save_png(buffer: &core_video::pixel_buffer::CVPixelBuffer, path: &std::path::Path) -> Result<(), String> {
    use core_foundation::base::TCFType;
    let ok = unsafe { canopy_sim_save_png(buffer.as_concrete_TypeRef(), cstr(&path.to_string_lossy()).as_ptr()) } == 0;
    if ok { Ok(()) } else { Err(format!("Can't save the screen to {}", path.display())) }
}

/// The framebuffer as RGBA pixels.
pub fn rgba(buffer: &core_video::pixel_buffer::CVPixelBuffer) -> Option<image::RgbaImage> {
    use core_foundation::base::TCFType;
    let (w, h) = (buffer.get_width(), buffer.get_height());
    let mut bytes = vec![0u8; w * h * 4];
    let ok = unsafe { canopy_sim_copy_rgba(buffer.as_concrete_TypeRef(), bytes.as_mut_ptr(), bytes.len()) } == 0;
    ok.then(|| image::RgbaImage::from_raw(w as u32, h as u32, bytes)).flatten()
}

fn cstr(s: &str) -> std::ffi::CString { std::ffi::CString::new(s.replace('\0', "")).unwrap() }

unsafe extern "C" {
    fn canopy_sim_helper(developer: *const std::ffi::c_char, udid: *const std::ffi::c_char, rendezvous: *const std::ffi::c_char) -> i32;
    fn canopy_sim_listen(name: *const std::ffi::c_char) -> u32;
    fn canopy_sim_close(port: u32);
    fn canopy_sim_take_surface(port: u32, timeout_ms: u32) -> core_video::pixel_buffer::CVPixelBufferRef;
    fn canopy_sim_save_png(buffer: core_video::pixel_buffer::CVPixelBufferRef, path: *const std::ffi::c_char) -> i32;
    fn canopy_sim_copy_rgba(buffer: core_video::pixel_buffer::CVPixelBufferRef, out: *mut u8, capacity: usize) -> i32;
    fn canopy_sim_render_pdf(pdf: *const std::ffi::c_char, mask: *const std::ffi::c_char, x: f64, y: f64, w: f64, h: f64, scale: f64, fill: u32, png: *const std::ffi::c_char) -> i32;
}

// ---- Bezel ----

/// A device's bezel from Xcode's DeviceKit chrome, rendered to PNGs in the temp folder.
#[derive(Clone, Debug, PartialEq)]
pub struct Chrome {
    /// The phone body with the screen cut out, drawn over the live screen.
    pub body: std::path::PathBuf,
    /// Its size in points.
    pub size: [f32; 2],
    /// The screen's rect inside the body, in points.
    pub screen: [f32; 4],
    /// Room on each side for buttons that stick out, in points.
    pub pad: f32,
    pub buttons: Vec<ChromeButton>,
}

/// A hardware button drawn on the bezel's edge.
#[derive(Clone, Debug, PartialEq)]
pub struct ChromeButton {
    pub name: String,
    pub label: String,
    pub page: u32,
    pub usage: u32,
    pub image: std::path::PathBuf,
    pub down: std::path::PathBuf,
    /// Its rect relative to the body's top left, in points (x may be negative: outside), slid
    /// out as DeviceKit draws it under the pointer, so it can be seen and clicked.
    pub rect: [f32; 4],
    /// How far a press pushes it in toward the body, in points.
    pub press: f32,
}

/// A device type's screen and bezel.
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceInfo {
    /// The screen in points, portrait.
    pub points: [f32; 2],
    /// Framebuffer pixels per point.
    pub scale: f32,
    pub chrome: Option<Chrome>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Profile { chrome_identifier: Option<String>, framebuffer_mask: Option<String> }

#[derive(serde::Deserialize)]
struct Capabilities { capabilities: ScreenCapabilities }
#[derive(serde::Deserialize)]
struct ScreenCapabilities {
    #[serde(rename = "ScreenDimensionsCapability")]
    screen: ScreenDimensions,
}
#[derive(serde::Deserialize)]
struct ScreenDimensions {
    #[serde(rename = "main-screen-width")] width: f32,
    #[serde(rename = "main-screen-height")] height: f32,
    #[serde(rename = "main-screen-scale")] scale: f32,
}

#[derive(serde::Deserialize)]
struct ChromeJson { images: Images, #[serde(default)] inputs: Vec<ChromeInput> }
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Images { composite: String, padding: Option<Pad> }
#[derive(serde::Deserialize)]
struct Pad { width: f32 }
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChromeInput {
    name: String,
    #[serde(default)] accessibility_title: String,
    #[serde(rename = "type")] kind: String,
    #[serde(default)] usage_page: u32,
    #[serde(default)] usage: u32,
    image: Option<String>,
    image_down: Option<String>,
    anchor: String,
    offsets: Offsets,
}
#[derive(serde::Deserialize)]
struct Offsets { normal: Xy, rollover: Option<Xy> }
#[derive(serde::Deserialize)]
struct Xy { x: f32, y: f32 }

/// The screen and bezel of a device type (its `simctl list` id), the bezel rendered once per run
/// at `scale` pixels per point over `background` (0xRRGGBB, the pane's color). The bezel is None when Xcode has no chrome for the type; the
/// pane then draws the bare screen. Blocking: lists device types and renders PDFs.
pub fn device_info(device_type: &str, scale: f64, background: u32) -> Option<DeviceInfo> {
    let out = Command::new("xcrun").args(["simctl", "list", "-j", "devicetypes"]).output().ok()?;
    let list: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let bundle = list["devicetypes"].as_array()?.iter().find(|t| t["identifier"] == device_type)?["bundlePath"].as_str()?.to_owned();
    let resources = std::path::Path::new(&bundle).join("Contents/Resources");
    let screen: Capabilities = serde_json::from_str(&plist_json(&resources.join("capabilities.plist"))?).ok()?;
    let dims = screen.capabilities.screen;
    let points = [dims.width / dims.scale, dims.height / dims.scale];
    let chrome = serde_json::from_str::<Profile>(&plist_json(&resources.join("profile.plist"))?).ok()
        .and_then(|profile| chrome(&profile, &resources, &device_type.replace(['/', ' '], "-"), points, scale, background));
    Some(DeviceInfo { points, scale: dims.scale, chrome })
}

fn chrome(profile: &Profile, resources: &std::path::Path, key: &str, points: [f32; 2], scale: f64, background: u32) -> Option<Chrome> {
    let chrome_id = profile.chrome_identifier.as_deref()?;
    let folder = std::path::Path::new("/Library/Developer/DeviceKit/Chrome").join(format!("{}.devicechrome/Contents/Resources", chrome_id.rsplit('.').next()?));
    let json: ChromeJson = serde_json::from_slice(&std::fs::read(folder.join("chrome.json")).ok()?).ok()?;
    let out = std::env::temp_dir().join("canopy").join("sim-chrome").join(format!("{key}@{scale}-{background:06x}"));
    std::fs::create_dir_all(&out).ok()?;
    let body_pdf = folder.join(format!("{}.pdf", json.images.composite));
    let size = pdf_size(&body_pdf)?;
    // The screen is centered in the body; DeviceKit draws phones and tablets that way.
    let screen = [(size[0] - points[0]) / 2., (size[1] - points[1]) / 2., points[0], points[1]];
    let mask = profile.framebuffer_mask.as_ref().map(|m| resources.join(format!("{m}.pdf"))).filter(|m| m.exists());
    let body = out.join("body.png");
    render(&body_pdf, mask.as_deref(), screen, scale, background, &body)?;
    let pad = json.images.padding.map_or(0., |p| p.width);
    let mut buttons = vec![];
    for input in json.inputs.into_iter().filter(|i| i.kind == "button") {
        let (Some(image), Some(down)) = (input.image, input.image_down) else { continue };
        let (image_pdf, down_pdf) = (folder.join(format!("{image}.pdf")), folder.join(format!("{down}.pdf")));
        let Some([w, h]) = pdf_size(&image_pdf) else { continue };
        let (png, down_png) = (out.join(format!("{image}.png")), out.join(format!("{down}.png")));
        if render(&image_pdf, None, [0.; 4], scale, 0, &png).is_none() || render(&down_pdf, None, [0.; 4], scale, 0, &down_png).is_none() { continue; }
        let out = input.offsets.rollover.as_ref().unwrap_or(&input.offsets.normal);
        let rect = button_rect(&input.anchor, [out.x, out.y], [w, h], size[0], pad);
        let press = (out.x - input.offsets.normal.x).abs();
        buttons.push(ChromeButton { name: input.name, label: input.accessibility_title, page: input.usage_page, usage: input.usage, image: png, down: down_png, rect, press });
    }
    Some(Chrome { body, size, screen, pad, buttons })
}

/// A chrome.json button's rect in body points. DeviceKit offsets place the button's leading
/// edge from the padded frame's edge on the anchor side (`pad` wider than the body each side);
/// a button sits mostly behind the body.
fn button_rect(anchor: &str, offset: [f32; 2], size: [f32; 2], body_width: f32, pad: f32) -> [f32; 4] {
    let x = if anchor == "right" { body_width + pad + offset[0] - size[0] } else { offset[0] - pad };
    [x, offset[1], size[0], size[1]]
}

fn render(pdf: &std::path::Path, mask: Option<&std::path::Path>, cut: [f32; 4], scale: f64, fill: u32, png: &std::path::Path) -> Option<()> {
    if png.exists() { return Some(()); }
    let mask = mask.map(|m| m.to_string_lossy().into_owned()).unwrap_or_default();
    let [x, y, w, h] = cut.map(f64::from);
    (unsafe { canopy_sim_render_pdf(cstr(&pdf.to_string_lossy()).as_ptr(), cstr(&mask).as_ptr(), x, y, w, h, scale, fill, cstr(&png.to_string_lossy()).as_ptr()) } == 0).then_some(())
}

/// A one-page PDF's media box size in points, from `sips`, which reads PDFs at 72 dpi.
fn pdf_size(pdf: &std::path::Path) -> Option<[f32; 2]> {
    let out = Command::new("sips").args(["-g", "pixelWidth", "-g", "pixelHeight"]).arg(pdf).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let value = |key: &str| text.lines().find_map(|l| l.trim().strip_prefix(key)?.trim().parse::<f32>().ok());
    Some([value("pixelWidth:")?, value("pixelHeight:")?])
}

fn plist_json(path: &std::path::Path) -> Option<String> {
    let out = Command::new("plutil").args(["-convert", "json", "-o", "-"]).arg(path).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

// ---- Layout ----

/// Where a device draws inside a pane body: the bezel (or bare screen) contain-fitted and
/// centered. All rects are in body pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fit {
    /// Pixels per device point.
    pub scale: f32,
    /// The bezel body (the screen itself when there is no bezel).
    pub body: [f32; 4],
    pub screen: [f32; 4],
}

/// Fits a device of `screen_points` (with its bezel, if any) into `room` pixels with `margin`
/// on every side.
pub fn fit(room: [f32; 2], screen_points: [f32; 2], chrome: Option<&Chrome>, margin: f32) -> Fit {
    let (size, screen, pad) = match chrome {
        Some(c) => (c.size, c.screen, c.pad),
        None => (screen_points, [0., 0., screen_points[0], screen_points[1]], 0.),
    };
    let outer = [size[0] + 2. * pad, size[1]];
    let scale = ((room[0] - 2. * margin) / outer[0]).min((room[1] - 2. * margin) / outer[1]).max(0.01);
    let x = (room[0] - size[0] * scale) / 2.;
    let y = (room[1] - size[1] * scale) / 2.;
    Fit {
        scale,
        body: [x, y, size[0] * scale, size[1] * scale],
        screen: [x + screen[0] * scale, y + screen[1] * scale, screen[2] * scale, screen[3] * scale],
    }
}

impl Fit {
    /// A body pixel as 0..1 of the screen, or None outside it.
    pub fn normalized(&self, at: [f32; 2]) -> Option<[f64; 2]> {
        let [x, y, w, h] = self.screen;
        let (nx, ny) = ((at[0] - x) / w, (at[1] - y) / h);
        ((0.0..=1.0).contains(&nx) && (0.0..=1.0).contains(&ny)).then_some([nx as f64, ny as f64])
    }
}

// ---- Keys ----

/// The HID keyboard usage (page 7) and whether Shift is needed for a typed character on a US
/// layout, as the simulator's hardware keyboard reads it.
pub fn char_usage(c: char) -> Option<(u32, bool)> {
    const SHIFTED: &str = "!@#$%^&*()";
    Some(match c {
        'a'..='z' => (0x04 + (c as u32 - 'a' as u32), false),
        'A'..='Z' => (0x04 + (c as u32 - 'A' as u32), true),
        '1'..='9' => (0x1e + (c as u32 - '1' as u32), false),
        '0' => (0x27, false),
        '\n' | '\r' => (0x28, false),
        '\t' => (0x2b, false),
        ' ' => (0x2c, false),
        _ if SHIFTED.contains(c) => (0x1e + SHIFTED.find(c).unwrap() as u32, true),
        _ => {
            let (plain, shifted) = ("-=[]\\;'`,./", "_+{}|:\"~<>?");
            let i = plain.find(c).map(|i| (i, false)).or_else(|| shifted.find(c).map(|i| (i, true)))?;
            ([0x2d, 0x2e, 0x2f, 0x30, 0x31, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38][i.0], i.1)
        }
    })
}

/// The HID usage for a gpui key name that is not a printable character.
pub fn named_key_usage(key: &str) -> Option<u32> {
    Some(match key {
        "enter" => 0x28, "escape" => 0x29, "backspace" => 0x2a, "tab" => 0x2b, "space" => 0x2c,
        "delete" => 0x4c, "home" => 0x4a, "end" => 0x4d, "pageup" => 0x4b, "pagedown" => 0x4e,
        "right" => 0x4f, "left" => 0x50, "down" => 0x51, "up" => 0x52,
        _ => return None,
    })
}

/// Shift, Control, Option and Command as HID usages, in the order they go down.
pub const MODIFIERS: [(&str, u32); 4] = [("shift", 0xe1), ("control", 0xe0), ("alt", 0xe2), ("platform", 0xe3)];

/// The key presses that type `text`: (usage, shift) per character; characters a US keyboard
/// cannot type are skipped and counted.
pub fn type_text(text: &str) -> (Vec<(u32, bool)>, usize) {
    let mut keys = vec![];
    let mut skipped = 0;
    for c in text.chars() {
        match char_usage(c) { Some(k) => keys.push(k), None => skipped += 1 }
    }
    (keys, skipped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_lines_match_the_helper() {
        assert_eq!(Input::Touch { phase: Phase::Start, x: 0.5, y: 1.2 }.line(), "t 0 0.50000 1.00000\n");
        assert_eq!(Input::Key { usage: 4, down: false }.line(), "k 4 2\n");
        assert_eq!(Input::Gesture { ms: 300, from: [0., 0.5], to: [1., 0.25] }.line(), "g 300 0.00000 0.50000 1.00000 0.25000\n");
        assert_eq!(Input::Button { page: 12, usage: 64, down: true }.line(), "b 12 64 1\n");
    }

    #[test]
    fn typing_maps_a_us_keyboard() {
        assert_eq!(type_text("aZ1!?\n é"), (vec![(0x04, false), (0x1d, true), (0x1e, false), (0x1e, true), (0x38, true), (0x28, false), (0x2c, false)], 1));
        assert_eq!(char_usage('0'), Some((0x27, false)));
        assert_eq!(char_usage(')'), Some((0x27, true)));
        assert_eq!(char_usage('~'), Some((0x35, true)));
    }

    #[test]
    fn fit_centers_and_maps_points_exactly() {
        // iPhone 17: 402x874 points in a 436x908 body, buttons 9 points out on each side.
        let chrome = Chrome { body: "b".into(), size: [436., 908.], screen: [17., 17., 402., 874.], pad: 9., buttons: vec![] };
        let f = fit([454., 908.], [402., 874.], Some(&chrome), 0.);
        assert_eq!(f.scale, 1.);
        assert_eq!(f.screen, [26., 17., 402., 874.]);
        // A device point maps to the same fraction the helper sends.
        assert_eq!(f.normalized([26. + 201., 17. + 437.]), Some([0.5, 0.5]));
        assert_eq!(f.normalized([20., 400.]), None);
        // Bare screen, wide room: height decides, centered across.
        let f = fit([1000., 874. / 2. + 20.], [402., 874.], None, 10.);
        assert_eq!(f.scale, 0.5);
        assert_eq!(f.screen, [500. - 100.5, 10., 201., 437.]);
    }

    #[test]
    fn buttons_sit_on_their_edge() {
        // phone11 slid out: action at x 3 on the left, power at x -3 on the right; 16 points wide.
        assert_eq!(button_rect("left", [3., 160.], [16., 34.], 436., 9.), [-6., 160., 16., 34.]);
        assert_eq!(button_rect("right", [-3., 262.], [16., 101.], 436., 9.), [426., 262., 16., 101.]);
    }
}
