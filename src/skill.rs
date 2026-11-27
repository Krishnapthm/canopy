//! Canopy's skill (src/skill/: SKILL.md and the references it links) in each installed
//! agent's global skills folder (programs.json `skills`). Written on start, only into
//! `<folder>/canopy/`, and only over files Canopy wrote. docs/wiki/Agent-Setup.md.

use std::path::Path;

/// The skill's files, by path inside `<skills>/canopy/`. SKILL.md comes first: its mark says
/// the folder is Canopy's. A harness loads SKILL.md, and the agent opens a reference when
/// SKILL.md sends it there.
const FILES: &[(&str, &str)] = &[
    ("SKILL.md", include_str!("skill/SKILL.md")),
    ("references/simulator.md", include_str!("skill/references/simulator.md")),
    ("references/browser.md", include_str!("skill/references/browser.md")),
    ("references/panes-and-canvas.md", include_str!("skill/references/panes-and-canvas.md")),
    ("references/mods.md", include_str!("skill/references/mods.md")),
    ("references/customize.md", include_str!("skill/references/customize.md")),
];
/// Marks a file Canopy wrote; a SKILL.md without it is someone else's and the folder stays untouched.
const MARK: &str = "<!-- Installed by Canopy, rewritten on start. `canopy skill remove` deletes it. -->";

fn text(body: &str) -> String { format!("{}\n\n{MARK}\n", body.trim_end()) }

/// `canopy --skill`: SKILL.md, then every reference under its path, for harnesses without skills.
pub fn printed() -> String {
    let mut out = FILES[0].1.trim_end().to_owned();
    for (path, body) in &FILES[1..] { out.push_str(&format!("\n\n<!-- {path} -->\n\n{}", body.trim_end())); }
    out.push('\n');
    out
}

/// Whether `<dir>/canopy/` is Canopy's to write: missing, or its SKILL.md carries the mark.
fn ours(dir: &Path) -> std::io::Result<bool> {
    match std::fs::read_to_string(dir.join("canopy/SKILL.md")) {
        Ok(old) => Ok(old.contains(MARK)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(e) => Err(e),
    }
}

/// Writes every skill file under `<dir>/canopy/` that is missing or differs, when the folder
/// is Canopy's. True when anything was written.
pub fn install(dir: &Path) -> std::io::Result<bool> {
    if !ours(dir)? { return Ok(false); }
    let mut wrote = false;
    // SKILL.md last: a half-written install still has no mark, so the next start finishes it.
    for (path, body) in FILES.iter().rev() {
        let path = dir.join("canopy").join(path);
        let text = text(body);
        if std::fs::read_to_string(&path).is_ok_and(|old| old == text) { continue; }
        std::fs::create_dir_all(path.parent().unwrap())?;
        crate::persist::write_atomic(&path, text.as_bytes())?;
        wrote = true;
    }
    Ok(wrote)
}

/// Deletes the skill files when Canopy wrote them, then `references/` and `canopy/` if empty.
/// True when deleted.
pub fn remove(dir: &Path) -> std::io::Result<bool> {
    let skill = dir.join("canopy/SKILL.md");
    if !skill.exists() || !ours(dir)? { return Ok(false); }
    for (path, _) in FILES {
        match std::fs::remove_file(dir.join("canopy").join(path)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e),
            _ => {}
        }
    }
    let _ = std::fs::remove_dir(dir.join("canopy/references"));
    let _ = std::fs::remove_dir(dir.join("canopy"));
    Ok(true)
}

/// On the window thread: installs for every installed agent with a skills folder, off-thread. Best effort, logged.
#[cfg(not(test))]
pub fn install_for_installed() {
    let folders = crate::programs::skill_folders();
    let list = crate::programs::startable();
    std::thread::spawn(move || {
        let commands: Vec<String> = list.iter().filter_map(|a| a["run"][0].as_str().map(str::to_owned)).collect();
        let found = match crate::mods::installed_cached(None, commands, crate::programs::installed) {
            Ok(found) => found,
            Err(e) => { tracing::warn!("Canopy skill: {e}"); return; }
        };
        let mut seen = std::collections::HashSet::new();
        for (command, dir) in folders {
            if !found.contains(&command) || !seen.insert(dir.clone()) { continue; }
            match install(&dir) {
                Ok(true) => tracing::info!("installed Canopy skill: {}", dir.join("canopy/SKILL.md").display()),
                Ok(false) => {}
                Err(e) => tracing::warn!("Canopy skill: {}: {e}", dir.display()),
            }
        }
    });
}

/// `canopy skill remove`; the exit code.
pub fn cli(args: &[String]) -> i32 {
    if args != ["remove"] {
        eprintln!("usage: canopy skill remove");
        return 2;
    }
    let _ = crate::programs::load();
    let mut seen = std::collections::HashSet::new();
    let mut removed = false;
    let mut failed = false;
    for (_, dir) in crate::programs::skill_folders() {
        if !seen.insert(dir.clone()) { continue; }
        let path = dir.join("canopy/SKILL.md");
        match remove(&dir) {
            Ok(true) => { println!("removed {}", path.display()); removed = true; }
            Ok(false) => {}
            Err(e) => { eprintln!("canopy skill: {}: {e}", path.display()); failed = true; }
        }
    }
    if !removed { println!("no Canopy skill installed"); }
    i32::from(failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Temp(std::path::PathBuf);
    impl Temp {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            Self(std::env::temp_dir().join(format!("canopy-skill-{}-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(), NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed))))
        }
        fn path(&self) -> std::path::PathBuf { self.0.join("canopy/SKILL.md") }
    }
    impl Drop for Temp {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
    }

    #[test]
    fn install_is_idempotent() {
        let dir = Temp::new();
        assert!(install(&dir.0).unwrap());
        let first = std::fs::read(dir.path()).unwrap();
        assert_eq!(first, text(FILES[0].1).as_bytes());
        for (path, body) in FILES { assert_eq!(std::fs::read_to_string(dir.0.join("canopy").join(path)).unwrap(), text(body)); }
        assert!(!install(&dir.0).unwrap());
        assert_eq!(std::fs::read(dir.path()).unwrap(), first);
    }

    #[test]
    fn install_updates_canopys_text() {
        let dir = Temp::new();
        std::fs::create_dir_all(dir.0.join("canopy")).unwrap();
        std::fs::write(dir.path(), format!("old\n{MARK}")).unwrap();
        assert!(install(&dir.0).unwrap());
        assert_eq!(std::fs::read_to_string(dir.path()).unwrap(), text(FILES[0].1));
    }

    #[test]
    fn foreign_skill_stays_untouched() {
        let dir = Temp::new();
        std::fs::create_dir_all(dir.0.join("canopy")).unwrap();
        std::fs::write(dir.path(), "someone else's skill\n").unwrap();
        assert!(!install(&dir.0).unwrap());
        assert!(!remove(&dir.0).unwrap());
        assert_eq!(std::fs::read(dir.path()).unwrap(), b"someone else's skill\n");
    }

    #[test]
    fn remove_keeps_other_files() {
        let dir = Temp::new();
        assert!(!remove(&dir.0).unwrap());
        install(&dir.0).unwrap();
        assert!(remove(&dir.0).unwrap());
        assert!(!dir.0.join("canopy").exists(), "references/ and canopy/ go too");
        install(&dir.0).unwrap();
        let extra = dir.0.join("canopy/extra");
        std::fs::write(&extra, "keep").unwrap();
        assert!(remove(&dir.0).unwrap());
        assert!(!dir.path().exists());
        assert_eq!(std::fs::read_to_string(extra).unwrap(), "keep");
    }

    #[test]
    fn skill_has_discoverable_frontmatter_and_links_every_reference() {
        let skill = FILES[0].1;
        assert!(skill.starts_with("---\nname: canopy\ndescription: "));
        // A reference SKILL.md never names is one no agent opens; mods.md links customize.md itself.
        for (path, _) in &FILES[1..] { assert!(skill.contains(&format!("]({path})")), "SKILL.md links {path}"); }
    }
}
