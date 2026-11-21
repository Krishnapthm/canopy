//! iOS Simulator access for sim panes through Xcode's public `xcrun simctl`:
//! the device list, boot and screenshots. Blocking calls; run them off the UI
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
    if out.status.success() { Ok(out.stdout) } else { Err(format!("simctl {}: {}", args[0], last_line(&out.stderr))) }
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
}
