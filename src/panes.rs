//! `canopy pane new|read|shot|open|tap|swipe|type|press|close`: agents' commands for canvas
//! panes. Each is sugar over an app command through the control socket (`canvas.pane_new`,
//! `canvas.pane_read`, `canvas.pane_shot`, `canvas.pane_open`, `canvas.sim_input` for a sim
//! pane's tap, swipe, type and press, `canvas.close {pane}`).
//! docs/wiki/Panes.md explains them.

use serde_json::{Value, json};

const USAGE: &str = "usage: canopy pane new file <path> [--wire] | new browser <url> [--wire] [--place panel|canvas|pinned] | new mod:<name> [--wire] | new sim [<udid>|booted] [--wire] [--place panel|canvas|pinned] | read <pane> | shot <pane> [file.png] | open <pane> <target> | tap <pane> <x> <y> | swipe <pane> <x1> <y1> <x2> <y2> [ms] | type <pane> <text> | press <pane> <button> | close <pane>";

/// The app command and its arguments for `canopy pane <args>`; `caller` is
/// `$HC_TERMINAL_ID`, which `--wire` needs.
fn request(args: &[String], caller: Option<&str>) -> Result<(&'static str, Value), String> {
    let mut words: Vec<_> = args.iter().map(String::as_str).collect();
    // `--place` works anywhere after `new`; the window checks the kind accepts it.
    let place = match words.iter().position(|w| *w == "--place") {
        Some(i) if words[0] == "new" => {
            let place = words.get(i + 1).copied().filter(|p| matches!(*p, "panel" | "canvas" | "pinned")).ok_or("--place is panel, canvas or pinned")?;
            words.drain(i..=i + 1);
            Some(place)
        }
        Some(_) => return Err(USAGE.into()),
        None => None,
    };
    let (name, mut a) = plain_request(&words, caller)?;
    if let Some(place) = place { a["place"] = json!(place); }
    Ok((name, a))
}

fn plain_request(words: &[&str], caller: Option<&str>) -> Result<(&'static str, Value), String> {
    let absolute = |path| std::path::absolute(path).map(|p| p.to_string_lossy().into_owned()).map_err(|e| format!("cannot resolve path: {e}"));
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
            // A browser pane's target is a URL, checked by the window; a file's is a path.
            let mut a = if *kind == "browser" { json!({"kind":kind,"url":target}) } else { json!({"kind":kind,"path":absolute(target)?}) };
            if words.len() == 4 {
                a["wire"] = json!(caller.filter(|s| !s.is_empty()).ok_or("--wire works only inside a Canopy terminal ($HC_TERMINAL_ID is not set)")?);
            }
            Ok(("canvas.pane_new", a))
        }
        ["read", pane] => Ok(("canvas.pane_read", json!({"pane":pane}))),
        ["shot", pane] => Ok(("canvas.pane_shot", json!({"pane":pane}))),
        ["shot", pane, file] => {
            if std::path::Path::new(file).extension().is_none_or(|ext| !ext.eq_ignore_ascii_case("png")) { return Err("the shot path must end in .png".into()); }
            Ok(("canvas.pane_shot", json!({"pane":pane,"path":absolute(file)?})))
        }
        ["open", pane, target] => Ok(("canvas.pane_open", json!({"pane":pane,"target":target}))),
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

/// What a successful command prints.
fn answer(name: &str, args: &Value, reply: &Value) -> String {
    match name {
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
        assert_eq!(req(&["new", "browser", "localhost:3000", "--wire"], Some("t1")).unwrap(), ("canvas.pane_new", json!({"kind":"browser","url":"localhost:3000","wire":"t1"})));
        assert_eq!(answer("canvas.pane_new", &Value::Null, &json!({"pane":"p3","kind":"browser","url":"http://localhost:3000","wired":false})), "p3: http://localhost:3000. Read it with canopy pane read p3");
        assert_eq!(req(&["read", "p1"], None).unwrap(), ("canvas.pane_read", json!({"pane":"p1"})));
        assert_eq!(req(&["shot", "p1"], None).unwrap(), ("canvas.pane_shot", json!({"pane":"p1"})));
        assert_eq!(req(&["shot", "p1", "a.PNG"], None).unwrap(), ("canvas.pane_shot", json!({"pane":"p1","path":std::path::absolute("a.PNG").unwrap()})));
        assert_eq!(req(&["shot", "p1", "a.jpg"], None).unwrap_err(), "the shot path must end in .png");
        assert_eq!(req(&["new", "sim"], None).unwrap(), ("canvas.pane_new", json!({"kind":"sim"})));
        assert_eq!(req(&["new", "browser", "localhost:3000", "--place", "canvas"], None).unwrap(), ("canvas.pane_new", json!({"kind":"browser","url":"localhost:3000","place":"canvas"})));
        assert_eq!(req(&["new", "sim", "--place", "pinned", "--wire"], Some("t1")).unwrap(), ("canvas.pane_new", json!({"kind":"sim","wire":"t1","place":"pinned"})));
        assert!(req(&["new", "sim", "--place", "dock"], None).is_err() && req(&["close", "p1", "--place", "panel"], None).is_err());
        assert_eq!(req(&["new", "sim", "--wire"], Some("t1")).unwrap(), ("canvas.pane_new", json!({"kind":"sim","wire":"t1"})));
        assert_eq!(req(&["new", "sim", "booted", "--wire"], Some("t1")).unwrap(), ("canvas.pane_new", json!({"kind":"sim","udid":"booted","wire":"t1"})));
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
    }
}
