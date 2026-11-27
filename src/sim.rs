//! iOS Simulator access for sim panes through Xcode's public `xcrun simctl`:
//! device lists, boot, shutdown and screenshots. Blocking calls; run them off the UI
//! thread. See docs/wiki/Simulator-Devices.md and docs/canopy/harness-notes/xcode-simctl.md.
use std::process::{Command, Stdio};

/// One simulator from `simctl list -j devices available`.
#[derive(Clone, Debug, PartialEq)]
pub struct Device {
    pub udid: String,
    pub name: String,
    /// For people: "iOS 26.5".
    pub runtime: String,
    pub booted: bool,
    /// RFC 3339 UTC, or empty if never used: sorts as text.
    pub last_used: String,
    /// "com.apple.CoreSimulator.SimDeviceType.iPhone-17": finds the screen size and bezel.
    pub device_type: String,
}

/// Whether a simulator is running, as agents read it in replies.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State { Booted, Shutdown }

impl Device {
    pub fn state(&self) -> State { if self.booted { State::Booted } else { State::Shutdown } }
}

/// One simulator as `canopy pane devices` and sim pane replies report it.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Listing { pub udid: String, pub name: String, pub runtime: String, pub state: State }

impl From<&Device> for Listing {
    fn from(d: &Device) -> Self { Listing { udid: d.udid.clone(), name: d.name.clone(), runtime: d.runtime.clone(), state: d.state() } }
}

/// The simulator a command names: the first booted one, or one by UDID.
#[derive(Clone, Debug, PartialEq)]
pub enum Target { Booted, Udid(String) }

impl Target {
    pub fn parse(word: &str) -> Self { if word == "booted" { Target::Booted } else { Target::Udid(word.to_owned()) } }

    /// The device this names in `devices` (sorted booted first, so `Booted` is the most recent).
    pub fn find<'a>(&self, devices: &'a [Device]) -> Option<&'a Device> {
        match self {
            Target::Booted => devices.iter().find(|d| d.booted),
            Target::Udid(udid) => devices.iter().find(|d| &d.udid == udid),
        }
    }

    /// Why `find` found nothing.
    pub fn missing(&self) -> String {
        match self {
            Target::Booted => "No simulator is booted: pick one from canopy pane devices".into(),
            Target::Udid(udid) => format!("{udid} is not an available simulator (canopy pane devices lists them)"),
        }
    }
}

/// What `canopy pane open <sim pane> <target>` names: a simulator to show, or a URL to open on it.
#[derive(Clone, Debug, PartialEq)]
pub enum Open { Device(Target), Url(Url) }

impl Open {
    pub fn parse(word: &str) -> Result<Self, String> {
        if word == "booted" || is_udid(word) { return Ok(Open::Device(Target::parse(word))); }
        // Paths and file URLs get Url's own advice about serving files.
        if word.contains("://") || word.starts_with(['/', '.', '~']) || word.starts_with("file:") { return Url::parse(word).map(Open::Url); }
        Err(format!("{word} is neither a simulator (a UDID or booted) nor a URL such as http://127.0.0.1:8000/"))
    }
}

/// simctl's device id shape: 8-4-4-4-12 hex digits.
fn is_udid(word: &str) -> bool {
    word.len() == 36 && word.bytes().enumerate().all(|(i, b)| if matches!(i, 8 | 13 | 18 | 23) { b == b'-' } else { b.is_ascii_hexdigit() })
}

/// An address for a simulator to open with `simctl openurl`: a web page (http, https)
/// or an app link (myapp://...). Never a host file: the device cannot read the Mac's disk.
#[derive(Clone, Debug, PartialEq)]
pub struct Url(String);

impl Url {
    pub fn parse(text: &str) -> Result<Self, String> {
        let text = text.trim();
        let Some((scheme, rest)) = text.split_once("://") else {
            return Err(format!("{text} is not a URL: give a full address such as http://127.0.0.1:8000/ (the simulator cannot open the Mac's files, so serve them over loopback)"));
        };
        let valid_scheme = scheme.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            && scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
        if !valid_scheme || rest.is_empty() { return Err(format!("{text} is not a URL")); }
        if scheme.eq_ignore_ascii_case("file") {
            return Err("The simulator cannot open the Mac's files: serve the folder over loopback (for example python3 -m http.server -b 127.0.0.1) and open its http:// address".into());
        }
        Ok(Url(text.to_owned()))
    }

    pub fn as_str(&self) -> &str { &self.0 }
}

/// Opens `url` on a booted device: Safari for web pages, the owning app for app links.
/// Returns once the device accepted it, not when the page has loaded.
pub fn open_url(udid: &str, url: &Url) -> Result<(), String> {
    simctl(&["openurl", udid, url.as_str()]).map(drop)
}

/// True when this Mac has Xcode's simctl. Checked once per run.
pub fn available() -> bool {
    static FOUND: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *FOUND.get_or_init(|| {
        cfg!(target_os = "macos")
            && Command::new("xcrun").args(["--find", "simctl"]).stdout(Stdio::null()).stderr(Stdio::null())
                .status().is_ok_and(|s| s.success())
    })
}

/// Available simulators: booted first, then most recently used, then by name.
pub fn devices() -> Result<Vec<Device>, String> {
    parse(&String::from_utf8_lossy(&simctl(&["list", "-j", "devices", "available"])?))
}

/// Boots a device and waits until it has finished booting; a booted device is left as is.
pub fn boot(udid: &str) -> Result<(), String> {
    simctl(&["bootstatus", udid, "-b"]).map(drop)
}

/// Shuts down one device. An already stopped device succeeds too.
pub fn shutdown(udid: &str) -> Result<(), String> {
    // simctl also accepts aliases such as `all` and `booted`. Only address the pane's device.
    if !is_udid(udid) {
        return Err(format!("Invalid device: {udid}"));
    }
    simctl(&["shutdown", udid]).map(drop)
}

/// Saves the device's screen to `path` as a PNG with transparent rounded corners.
/// The device must be booted: on a shut down one simctl waits 60 s, then fails.
pub fn screenshot(udid: &str, path: &std::path::Path) -> Result<(), String> {
    // Not "-": Xcode 27.2's simctl writes that to a file named "-", not stdout.
    simctl(&["io", udid, "screenshot", "--type=png", "--mask=alpha", &path.to_string_lossy()]).map(drop)
}

/// "Xcode 27.2 (27B52)", once per run; None without Xcode. Blocking the first time.
pub fn xcode_version() -> Option<String> {
    static VERSION: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    VERSION.get_or_init(|| {
        let out = Command::new("xcodebuild").arg("-version").stdin(Stdio::null()).stderr(Stdio::null()).output().ok().filter(|o| o.status.success())?;
        let text = String::from_utf8_lossy(&out.stdout);
        let mut lines = text.lines();
        let version = lines.next()?.trim().to_owned();
        Some(match lines.next().and_then(|l| l.strip_prefix("Build version ")) { Some(build) => format!("{version} ({})", build.trim()), None => version })
    }).clone()
}

/// Opens Xcode's Device Hub, where the person can use the simulator.
pub fn open_device_hub() -> Result<(), String> {
    let out = Command::new("open").args(["-b", "com.apple.dt.Devices"]).output().map_err(|e| format!("Can't open Device Hub: {e}"))?;
    if out.status.success() { Ok(()) } else { Err(format!("Can't open Device Hub: {}", last_line(&out.stderr))) }
}

fn simctl(args: &[&str]) -> Result<Vec<u8>, String> {
    let out = Command::new("xcrun").arg("simctl").args(args).stdin(Stdio::null()).output()
        .map_err(|e| format!("Can't run xcrun simctl: {e}"))?;
    if out.status.success() { return Ok(out.stdout); }
    let reason = last_line(&out.stderr);
    if args[0] == "shutdown" && already_shutdown(out.status.code(), &reason) { return Ok(out.stdout); }
    Err(format!("simctl {}: {reason}", args[0]))
}

fn already_shutdown(code: Option<i32>, reason: &str) -> bool {
    code == Some(149) && reason == "Unable to shutdown device in current state: Shutdown"
}

/// simctl's reason is its last non-empty stderr line.
fn last_line(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr).lines().map(str::trim).filter(|l| !l.is_empty()).last().unwrap_or("failed").to_owned()
}

/// Parses `simctl list -j devices`: `{"devices": {<runtime id>: [device]}}`.
pub fn parse(json: &str) -> Result<Vec<Device>, String> {
    #[derive(serde::Deserialize)]
    struct List { devices: std::collections::HashMap<String, Vec<Raw>> }
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Raw { udid: String, name: String, state: String, #[serde(default = "yes")] is_available: bool, #[serde(default)] last_used_at: String, #[serde(default)] device_type_identifier: String }
    fn yes() -> bool { true }
    let list: List = serde_json::from_str(json).map_err(|e| format!("simctl list: unexpected output: {e}"))?;
    let mut out: Vec<Device> = list.devices.into_iter().flat_map(|(runtime, raw)| {
        let runtime = runtime_name(&runtime);
        raw.into_iter().filter(|d| d.is_available).map(move |d| Device {
            udid: d.udid, name: d.name, runtime: runtime.clone(), booted: d.state == "Booted", last_used: d.last_used_at, device_type: d.device_type_identifier,
        })
    }).collect();
    out.sort_by(|a, b| b.booted.cmp(&a.booted).then(b.last_used.cmp(&a.last_used)).then(b.runtime.cmp(&a.runtime)).then(a.name.cmp(&b.name)));
    Ok(out)
}

/// "com.apple.CoreSimulator.SimRuntime.iOS-26-5" -> "iOS 26.5".
fn runtime_name(id: &str) -> String {
    let short = id.rsplit('.').next().unwrap_or(id);
    match short.split_once('-') {
        Some((os, version)) => format!("{os} {}", version.replace('-', ".")),
        None => short.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sorts_and_names_runtimes() {
        let json = r#"{"devices":{
            "com.apple.CoreSimulator.SimRuntime.iOS-26-5":[
                {"udid":"A","name":"iPhone 17 Pro","state":"Shutdown","isAvailable":true,"lastUsedAt":"2026-09-16T05:38:30Z","deviceTypeIdentifier":"com.apple.CoreSimulator.SimDeviceType.iPhone-17-Pro"},
                {"udid":"B","name":"iPhone 17","state":"Shutdown","isAvailable":true,"lastUsedAt":"2026-11-17T14:31:39Z"},
                {"udid":"C","name":"iPad (A16)","state":"Shutdown","isAvailable":true},
                {"udid":"D","name":"Gone","state":"Shutdown","isAvailable":false}],
            "com.apple.CoreSimulator.SimRuntime.watchOS-12-0":[
                {"udid":"E","name":"Apple Watch","state":"Booted","isAvailable":true}]}}"#;
        let list = parse(json).unwrap();
        assert_eq!(list.iter().map(|d| d.udid.as_str()).collect::<Vec<_>>(), ["E", "B", "A", "C"]);
        assert_eq!(list[0].runtime, "watchOS 12.0");
        assert!(list[0].booted);
        assert_eq!(list[1].runtime, "iOS 26.5");
        assert_eq!((list[1].device_type.as_str(), list[2].device_type.as_str()), ("", "com.apple.CoreSimulator.SimDeviceType.iPhone-17-Pro"));
        assert!(parse("{}").unwrap_err().starts_with("simctl list: unexpected output"));
    }

    #[test]
    fn simctl_reason_is_the_last_line() {
        assert_eq!(last_line(b"An error was encountered (code=405):\nUnable to boot device in current state: Booted\n\n"), "Unable to boot device in current state: Booted");
        assert_eq!(last_line(b""), "failed");
    }

    #[test]
    fn shutdown_only_accepts_the_confirmed_off_state() {
        let off = "Unable to shutdown device in current state: Shutdown";
        assert!(already_shutdown(Some(149), off));
        assert!(!already_shutdown(Some(148), off));
        assert!(!already_shutdown(Some(149), "Unable to shutdown device in current state: Shutting Down"));
        assert!(shutdown("all").is_err());
    }

    #[test]
    fn targets_and_urls_parse_at_the_boundary() {
        let devices = parse(r#"{"devices":{"com.apple.CoreSimulator.SimRuntime.iOS-26-5":[
            {"udid":"A","name":"iPhone 17","state":"Shutdown"},{"udid":"B","name":"iPad","state":"Booted"}]}}"#).unwrap();
        assert_eq!(Target::parse("booted").find(&devices).map(|d| d.udid.as_str()), Some("B"));
        assert_eq!(Target::parse("A").find(&devices).map(|d| d.name.as_str()), Some("iPhone 17"));
        assert!(Target::parse("Z").find(&devices).is_none());
        assert_eq!(Target::Booted.find(&devices[1..]), None);
        assert_eq!(serde_json::to_value(Listing::from(&devices[0])).unwrap(), serde_json::json!({"udid":"B","name":"iPad","runtime":"iOS 26.5","state":"booted"}));
        assert_eq!(Url::parse(" http://127.0.0.1:8765/claude.html ").unwrap().as_str(), "http://127.0.0.1:8765/claude.html");
        assert!(Url::parse("myapp://settings").is_ok());
        let udid = "0123ABCD-0123-0123-0123-0123456789AB";
        assert_eq!(Open::parse(udid).unwrap(), Open::Device(Target::Udid(udid.into())));
        assert_eq!(Open::parse("booted").unwrap(), Open::Device(Target::Booted));
        assert!(matches!(Open::parse("http://127.0.0.1:8765/"), Ok(Open::Url(_))));
        assert!(Open::parse("./claude.html").unwrap_err().contains("loopback"));
        assert!(Open::parse("iPhone").unwrap_err().contains("neither"));
        for bad in ["localhost:3000", "./index.html", "/tmp/a.html", "file:///tmp/a.html", "http://", "1x://a"] {
            assert!(Url::parse(bad).is_err(), "{bad}");
        }
    }
}
