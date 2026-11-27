//! `canopy pane new|read|shot|open|tap|swipe|type|press|close|devices`: agents' commands for
//! canvas panes. Each is sugar over an app command through the control socket (`canvas.pane_new`,
//! `canvas.pane_read`, `canvas.pane_shot`, `canvas.pane_open`, `canvas.sim_input` for a sim
//! pane's tap, swipe, type and press, `canvas.close {pane}`, `canvas.sim_devices`). Browser and
//! sim panes opened here land on the canvas unless `--place` says otherwise, and their replies
//! wait for the page or simulator, so the printed line says how it turned out.
//! docs/wiki/Panes.md explains them.

use serde::Deserialize;
use serde_json::{Value, json};

const USAGE: &str = "usage: canopy pane new file <path> [--wire] | new browser <url|path> [--wire] [--place panel|canvas|pinned] | new mod:<name> [--wire] | new sim [<udid>|booted] [--url <url>] [--wire] [--place panel|canvas|pinned] | devices | read <pane> | shot <pane> [file.png] | open <pane> <url|path|udid|booted> | tap <pane> <x> <y> | swipe <pane> <x1> <y1> <x2> <y2> [ms] | type <pane> <text> | press <pane> <button> | close <pane>";

/// The app command and its arguments for `canopy pane <args>`; `caller` is
/// `$HC_TERMINAL_ID`, which `--wire` needs.
fn request(args: &[String], caller: Option<&str>) -> Result<(&'static str, Value), String> {
    let mut words: Vec<_> = args.iter().map(String::as_str).collect();
    // `--place` and `--url` work anywhere after `new`; the window checks the kind accepts them.
    let mut flag = |name: &str| -> Result<Option<&str>, String> {
        let Some(i) = words.iter().position(|w| *w == name).filter(|_| words.first() == Some(&"new")) else { return Ok(None) };
        let value = words.get(i + 1).copied().ok_or_else(|| format!("{name} needs a value"))?;
        words.drain(i..=i + 1);
        Ok(Some(value))
    };
    let place = flag("--place")?;
    if place.is_some_and(|p| !matches!(p, "panel" | "canvas" | "pinned")) { return Err("--place is panel, canvas or pinned".into()); }
    let url = flag("--url")?;
    let (name, mut a) = plain_request(&words, caller)?;
    if let Some(url) = url {
        if a["kind"] != "sim" { return Err("--url is for sim panes; a browser takes its address directly: canopy pane new browser <url>".into()); }
        a["url"] = json!(url);
    }
    // An agent opening a page or simulator wants to see it beside its work, not in a panel tab.
    let shown = matches!(a["kind"].as_str(), Some("browser" | "sim"));
    if let Some(place) = place.or(shown.then_some("canvas")) { a["place"] = json!(place); }
    Ok((name, a))
}

fn plain_request(words: &[&str], caller: Option<&str>) -> Result<(&'static str, Value), String> {
    let absolute = |path: &str| std::path::absolute(path).map(|p| p.to_string_lossy().into_owned()).map_err(|e| format!("cannot resolve path: {e}"));
    let browser_target = |target: &str| {
        if target.starts_with("./") || target.starts_with("../") { absolute(target) } else { Ok(target.to_owned()) }
    };
    match words {
        ["new", kind, rest @ ..] if kind.starts_with("mod:") && (rest.is_empty() || *rest == ["--wire"]) => {
            let mut a = json!({"kind":kind});
            if !rest.is_empty() {
                a["wire"] = json!(caller.filter(|s| !s.is_empty()).ok_or("--wire works only inside a Canopy terminal ($HC_TERMINAL_ID is not set)")?);
            }
            Ok(("canvas.pane_new", a))
        }
        ["new", "sim", rest @ ..] if matches!(rest, [] | [_] | [_, "--wire"]) => {
            let mut a = json!({"kind":"sim"});
            if let Some(udid) = rest.first().filter(|w| **w != "--wire") { a["udid"] = json!(udid); }
            if rest.last() == Some(&"--wire") {
                a["wire"] = json!(caller.filter(|s| !s.is_empty()).ok_or("--wire works only inside a Canopy terminal ($HC_TERMINAL_ID is not set)")?);
            }
            Ok(("canvas.pane_new", a))
        }
        ["new", kind, target] | ["new", kind, target, "--wire"] => {
            // The window validates browser addresses and local paths.
            let mut a = if *kind == "browser" { json!({"kind":kind,"url":browser_target(target)?}) } else { json!({"kind":kind,"path":absolute(target)?}) };
            if words.len() == 4 {
                a["wire"] = json!(caller.filter(|s| !s.is_empty()).ok_or("--wire works only inside a Canopy terminal ($HC_TERMINAL_ID is not set)")?);
            }
            Ok(("canvas.pane_new", a))
        }
        ["devices"] => Ok(("canvas.sim_devices", json!({}))),
        ["read", pane] => Ok(("canvas.pane_read", json!({"pane":pane}))),
        ["shot", pane] => Ok(("canvas.pane_shot", json!({"pane":pane}))),
        ["shot", pane, file] => {
            if std::path::Path::new(file).extension().is_none_or(|ext| !ext.eq_ignore_ascii_case("png")) { return Err("the shot path must end in .png".into()); }
            Ok(("canvas.pane_shot", json!({"pane":pane,"path":absolute(file)?})))
        }
        ["open", pane, target] => Ok(("canvas.pane_open", json!({"pane":pane,"target":browser_target(target)?}))),
        ["tap", pane, x, y] => Ok(("canvas.sim_input", json!({"pane":pane,"action":"tap","x":number(x)?,"y":number(y)?}))),
        ["swipe", pane, x1, y1, x2, y2, ms @ ..] if ms.len() <= 1 => {
            let mut a = json!({"pane":pane,"action":"swipe","x1":number(x1)?,"y1":number(y1)?,"x2":number(x2)?,"y2":number(y2)?});
            if let Some(ms) = ms.first() { a["ms"] = json!(ms.parse::<u64>().map_err(|_| format!("{ms} is not a whole number of milliseconds"))?); }
            Ok(("canvas.sim_input", a))
        }
        ["type", pane, text @ ..] if !text.is_empty() => Ok(("canvas.sim_input", json!({"pane":pane,"action":"type","text":text.join(" ")}))),
        ["press", pane, button] => Ok(("canvas.sim_input", json!({"pane":pane,"action":"press","button":button}))),
        ["close", pane] => Ok(("canvas.close", json!({"pane":pane}))),
        _ => Err(USAGE.into()),
    }
}

/// A point coordinate.
fn number(word: &str) -> Result<f64, String> {
    word.parse::<f64>().ok().filter(|n| n.is_finite()).ok_or_else(|| format!("{word} is not a number"))
}

/// A sim pane's reply once its refresh finished (see window sim.rs `Opened`).
#[derive(serde::Deserialize)]
struct SimOpened { pane: String, name: String, runtime: String, state: String, udid: String, url: Option<String>, screenshot: Option<String>, #[serde(default)] wired: bool }

/// A browser pane's reply once its page loaded.
#[derive(serde::Deserialize)]
struct PageOpened { pane: String, url: String, title: String, #[serde(default)] wired: bool }

/// One simulator from `canvas.sim_devices`.
#[derive(serde::Deserialize)]
struct Listed { udid: String, name: String, runtime: String, state: String }

/// What a successful command prints.
fn answer(name: &str, args: &Value, reply: &Value) -> String {
    let wired = |w: bool| if w { ", wired to you" } else { "" };
    if let Ok(s) = SimOpened::deserialize(reply) {
        let opened = s.url.as_ref().map(|u| format!(", opening {u}")).unwrap_or_default();
        let shot = s.screenshot.map(|p| format!(". Screenshot: {p}")).unwrap_or_default();
        let read = if s.url.is_some() { "See the page with" } else { "Read it with" };
        return format!("{}: {} ({}) {} [{}]{opened}{}{shot}. {read} canopy pane read {}", s.pane, s.name, s.runtime, s.state, s.udid, wired(s.wired), s.pane);
    }
    if let Ok(p) = PageOpened::deserialize(reply) {
        return format!("{}: {} loaded, title {:?}{}. Read it with canopy pane read {}", p.pane, p.url, p.title, wired(p.wired), p.pane);
    }
    match name {
        "canvas.sim_devices" => {
            let list: Vec<Listed> = Vec::deserialize(&reply["devices"]).unwrap_or_default();
            if list.is_empty() { return "No simulators are available: add one in Xcode (Window > Devices and Simulators)".into(); }
            list.iter().map(|d| format!("{}\t{} ({})\t{}", d.udid, d.name, d.runtime, d.state)).collect::<Vec<_>>().join("\n")
        }
        "canvas.pane_new" => {
            let pane = reply["pane"].as_str().unwrap_or_default();
            let path = reply["path"].as_str().or(reply["url"].as_str()).or(reply["kind"].as_str()).unwrap_or_default();
            let wired = if reply["wired"] == true { ", wired to you" } else { "" };
            format!("{pane}: {path}{wired}. Read it with canopy pane read {pane}")
        }
        "canvas.pane_read" => reply["text"].as_str().unwrap_or_default().to_owned(),
        "canvas.pane_shot" => reply["path"].as_str().unwrap_or_default().to_owned(),
        "canvas.close" => format!("closed {}", args["pane"].as_str().unwrap_or_default()),
        "canvas.pane_open" if reply["url"].is_string() => format!("{}: {}", reply["pane"].as_str().unwrap_or_default(), reply["url"].as_str().unwrap_or_default()),
        _ => "ok".into(),
    }
}

/// `canopy pane ...` from a shell; the exit code: 2 for usage or no window, 1 when the
/// window refused, 0 after printing the answer.
pub fn cli(args: &[String]) -> i32 {
    let fail = |message: &str, code| { eprintln!("canopy pane: {message}"); code };
    let caller = std::env::var("HC_TERMINAL_ID").ok();
    let (name, a) = match request(args, caller.as_deref()) { Ok(r) => r, Err(e) => return fail(&e, 2) };
    let reply = match crate::control::call(&json!({"do":name,"args":a})) { Ok(r) => r, Err(e) => return fail(&e, 2) };
    if let Some(e) = reply["error"].as_str() { return fail(e, 1); }
    println!("{}", answer(name, &a, &reply));
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_cli_maps_verbs_to_commands() {
        let req = |args: &[&str], caller| request(&args.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(), caller);
        assert_eq!(req(&["new", "mod:note"], None).unwrap(), ("canvas.pane_new", json!({"kind":"mod:note"})));
        assert_eq!(req(&["new", "mod:note", "--wire"], Some("t1")).unwrap(), ("canvas.pane_new", json!({"kind":"mod:note","wire":"t1"})));
        assert_eq!(answer("canvas.pane_new", &json!({}), &json!({"pane":"p2","kind":"mod:note","wired":true})), "p2: mod:note, wired to you. Read it with canopy pane read p2");
        let path = std::path::absolute("README.md").unwrap();
        assert_eq!(req(&["new", "file", "README.md"], None).unwrap(), ("canvas.pane_new", json!({"kind":"file","path":path})));
        assert_eq!(req(&["new", "file", "README.md", "--wire"], Some("t1")).unwrap(), ("canvas.pane_new", json!({"kind":"file","path":path,"wire":"t1"})));
        for caller in [None, Some("")] {
            assert_eq!(req(&["new", "file", "README.md", "--wire"], caller).unwrap_err(), "--wire works only inside a Canopy terminal ($HC_TERMINAL_ID is not set)");
        }
        assert_eq!(req(&["new", "browser", "localhost:3000", "--wire"], Some("t1")).unwrap(), ("canvas.pane_new", json!({"kind":"browser","url":"localhost:3000","wire":"t1","place":"canvas"})));
        assert_eq!(answer("canvas.pane_new", &Value::Null, &json!({"pane":"p3","kind":"browser","url":"http://localhost:3000","wired":false})), "p3: http://localhost:3000. Read it with canopy pane read p3");
        assert_eq!(req(&["read", "p1"], None).unwrap(), ("canvas.pane_read", json!({"pane":"p1"})));
        assert_eq!(req(&["shot", "p1"], None).unwrap(), ("canvas.pane_shot", json!({"pane":"p1"})));
        assert_eq!(req(&["shot", "p1", "a.PNG"], None).unwrap(), ("canvas.pane_shot", json!({"pane":"p1","path":std::path::absolute("a.PNG").unwrap()})));
        assert_eq!(req(&["shot", "p1", "a.jpg"], None).unwrap_err(), "the shot path must end in .png");
        assert_eq!(req(&["new", "sim"], None).unwrap(), ("canvas.pane_new", json!({"kind":"sim","place":"canvas"})));
        assert_eq!(req(&["new", "sim", "booted", "--url", "http://127.0.0.1:8765/", "--place", "panel"], None).unwrap(), ("canvas.pane_new", json!({"kind":"sim","udid":"booted","url":"http://127.0.0.1:8765/","place":"panel"})));
        assert!(req(&["new", "browser", "localhost:3000", "--url", "x"], None).unwrap_err().starts_with("--url is for sim panes"));
        assert_eq!(req(&["new", "sim", "booted", "--url"], None).unwrap_err(), "--url needs a value");
        assert_eq!(req(&["devices"], None).unwrap(), ("canvas.sim_devices", json!({})));
        assert_eq!(req(&["new", "browser", "localhost:3000", "--place", "canvas"], None).unwrap(), ("canvas.pane_new", json!({"kind":"browser","url":"localhost:3000","place":"canvas"})));
        assert_eq!(req(&["new", "sim", "--place", "pinned", "--wire"], Some("t1")).unwrap(), ("canvas.pane_new", json!({"kind":"sim","wire":"t1","place":"pinned"})));
        assert!(req(&["new", "sim", "--place", "dock"], None).is_err() && req(&["close", "p1", "--place", "panel"], None).is_err());
        assert_eq!(req(&["type", "p1", "--place"], None).unwrap(), ("canvas.sim_input", json!({"pane":"p1","action":"type","text":"--place"})));
        assert_eq!(req(&["new", "sim", "--wire"], Some("t1")).unwrap(), ("canvas.pane_new", json!({"kind":"sim","wire":"t1","place":"canvas"})));
        assert_eq!(req(&["new", "sim", "booted", "--wire"], Some("t1")).unwrap(), ("canvas.pane_new", json!({"kind":"sim","udid":"booted","wire":"t1","place":"canvas"})));
        assert!(req(&["new", "sim", "a", "b"], None).is_err());
        assert_eq!(req(&["open", "p1", "target"], None).unwrap(), ("canvas.pane_open", json!({"pane":"p1","target":"target"})));
        assert_eq!(req(&["close", "p1"], None).unwrap(), ("canvas.close", json!({"pane":"p1"})));
        assert_eq!(req(&["unknown"], None).unwrap_err(), USAGE);
        for (wired, suffix) in [(false, ""), (true, ", wired to you")] {
            assert_eq!(answer("canvas.pane_new", &Value::Null, &json!({"pane":"p1","path":"/a.md","wired":wired})), format!("p1: /a.md{suffix}. Read it with canopy pane read p1"));
        }
        assert_eq!(answer("canvas.pane_read", &Value::Null, &json!({"text":"hello"})), "hello");
        assert_eq!(answer("canvas.pane_shot", &Value::Null, &json!({"path":"/a.png"})), "/a.png");
        assert_eq!(answer("canvas.close", &json!({"pane":"p1"}), &Value::Null), "closed p1");
        assert_eq!(answer("canvas.pane_open", &Value::Null, &Value::Null), "ok");
        assert_eq!(answer("canvas.pane_open", &Value::Null, &json!({"pane":"p2","url":"http://localhost:3000"})), "p2: http://localhost:3000");
        assert_eq!(req(&["tap", "p2", "201", "437.5"], None).unwrap(), ("canvas.sim_input", json!({"pane":"p2","action":"tap","x":201.0,"y":437.5})));
        assert_eq!(req(&["swipe", "p2", "1", "2", "3", "4", "500"], None).unwrap().1, json!({"pane":"p2","action":"swipe","x1":1.0,"y1":2.0,"x2":3.0,"y2":4.0,"ms":500}));
        assert_eq!(req(&["type", "p2", "hello", "world"], None).unwrap().1, json!({"pane":"p2","action":"type","text":"hello world"}));
        assert_eq!(req(&["press", "p2", "home"], None).unwrap().1, json!({"pane":"p2","action":"press","button":"home"}));
        assert_eq!(req(&["tap", "p2", "x", "1"], None).unwrap_err(), "x is not a number");
        let sim = json!({"pane":"p3","kind":"sim","wired":true,"udid":"U","name":"iPhone 17","runtime":"iOS 26.5","state":"booted","url":"http://127.0.0.1:8765/"});
        assert_eq!(answer("canvas.pane_new", &Value::Null, &sim), "p3: iPhone 17 (iOS 26.5) booted [U], opening http://127.0.0.1:8765/, wired to you. See the page with canopy pane read p3");
        let sim = json!({"pane":"p3","udid":"U","name":"iPhone 17","runtime":"iOS 26.5","state":"booted","screenshot":"/tmp/s.png"});
        assert_eq!(answer("canvas.pane_open", &Value::Null, &sim), "p3: iPhone 17 (iOS 26.5) booted [U]. Screenshot: /tmp/s.png. Read it with canopy pane read p3");
        let page = json!({"pane":"p2","kind":"browser","wired":false,"url":"http://127.0.0.1:8765/","title":"Claude","loaded":true});
        assert_eq!(answer("canvas.pane_open", &Value::Null, &page), "p2: http://127.0.0.1:8765/ loaded, title \"Claude\". Read it with canopy pane read p2");
        assert_eq!(answer("canvas.sim_devices", &Value::Null, &json!({"devices":[{"udid":"U","name":"iPhone 17","runtime":"iOS 26.5","state":"booted"}]})), "U\tiPhone 17 (iOS 26.5)\tbooted");
        assert!(answer("canvas.sim_devices", &Value::Null, &json!({"devices":[]})).starts_with("No simulators"));
    }
}
