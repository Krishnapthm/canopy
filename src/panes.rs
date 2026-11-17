//! `canopy pane new|read|shot|open|close`: agents' commands for canvas panes. Each is
//! sugar over an app command through the control socket (`canvas.pane_new`,
//! `canvas.pane_read`, `canvas.pane_shot`, `canvas.pane_open`, `canvas.close {pane}`).
//! docs/panes.md explains them.

use serde_json::{Value, json};

const USAGE: &str = "usage: canopy pane new file <path> [--wire] | new mod:<name> [--wire] | read <pane> | shot <pane> [file.png] | open <pane> <target> | close <pane>";

/// The app command and its arguments for `canopy pane <args>`; `caller` is
/// `$HC_TERMINAL_ID`, which `--wire` needs.
fn request(args: &[String], caller: Option<&str>) -> Result<(&'static str, Value), String> {
    let words: Vec<_> = args.iter().map(String::as_str).collect();
    let absolute = |path| std::path::absolute(path).map(|p| p.to_string_lossy().into_owned()).map_err(|e| format!("cannot resolve path: {e}"));
    match words.as_slice() {
        ["new", kind, rest @ ..] if kind.starts_with("mod:") && (rest.is_empty() || *rest == ["--wire"]) => {
            let mut a = json!({"kind":kind});
            if !rest.is_empty() {
                a["wire"] = json!(caller.filter(|s| !s.is_empty()).ok_or("--wire works only inside a Canopy terminal ($HC_TERMINAL_ID is not set)")?);
            }
            Ok(("canvas.pane_new", a))
        }
        ["new", kind, path] | ["new", kind, path, "--wire"] => {
            let mut a = json!({"kind":kind,"path":absolute(path)?});
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
        ["close", pane] => Ok(("canvas.close", json!({"pane":pane}))),
        _ => Err(USAGE.into()),
    }
}

/// What a successful command prints.
fn answer(name: &str, args: &Value, reply: &Value) -> String {
    match name {
        "canvas.pane_new" => {
            let pane = reply["pane"].as_str().unwrap_or_default();
            let path = reply["path"].as_str().or(reply["kind"].as_str()).unwrap_or_default();
            let wired = if reply["wired"] == true { ", wired to you" } else { "" };
            format!("{pane}: {path}{wired}. Read it with canopy pane read {pane}")
        }
        "canvas.pane_read" => reply["text"].as_str().unwrap_or_default().to_owned(),
        "canvas.pane_shot" => reply["path"].as_str().unwrap_or_default().to_owned(),
        "canvas.close" => format!("closed {}", args["pane"].as_str().unwrap_or_default()),
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
        assert_eq!(req(&["read", "p1"], None).unwrap(), ("canvas.pane_read", json!({"pane":"p1"})));
        assert_eq!(req(&["shot", "p1"], None).unwrap(), ("canvas.pane_shot", json!({"pane":"p1"})));
        assert_eq!(req(&["shot", "p1", "a.PNG"], None).unwrap(), ("canvas.pane_shot", json!({"pane":"p1","path":std::path::absolute("a.PNG").unwrap()})));
        assert_eq!(req(&["shot", "p1", "a.jpg"], None).unwrap_err(), "the shot path must end in .png");
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
    }
}
