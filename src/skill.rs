//! Canopy's skill (src/SKILL.md) in each installed agent's global skills folder (programs.json `skills`). Written on start, only into `<folder>/canopy/`, and only over a file Canopy wrote. docs/wiki/Agent-Setup.md.

use std::path::Path;

const TEXT: &str = include_str!("SKILL.md");
/// Marks a SKILL.md Canopy wrote; one without it is someone else's and stays untouched.
const MARK: &str = "<!-- Installed by Canopy, rewritten on start. `canopy skill remove` deletes it. -->";

fn text() -> String { format!("{}\n\n{MARK}\n", TEXT.trim_end()) }

/// Writes `<dir>/canopy/SKILL.md` when missing or Canopy's and different. True when written.
pub fn install(dir: &Path) -> std::io::Result<bool> {
    let path = dir.join("canopy").join("SKILL.md");
    match std::fs::read_to_string(&path) {
        Ok(old) if !old.contains(MARK) || old == text() => return Ok(false),
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    std::fs::create_dir_all(dir.join("canopy"))?;
    crate::persist::write_atomic(&path, text().as_bytes())?;
    Ok(true)
}

/// Deletes `<dir>/canopy/SKILL.md` when Canopy wrote it, then the `canopy` folder if empty. True when deleted.
pub fn remove(dir: &Path) -> std::io::Result<bool> {
    let path = dir.join("canopy").join("SKILL.md");
    match std::fs::read_to_string(&path) {
        Ok(old) if !old.contains(MARK) => return Ok(false),
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e),
    }
    std::fs::remove_file(path)?;
    let _ = std::fs::remove_dir(dir.join("canopy"));
    Ok(true)
}

/// On the window thread: installs for every installed agent with a skills folder, off-thread. Best effort, logged.
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
        assert_eq!(first, text().as_bytes());
        assert!(!install(&dir.0).unwrap());
        assert_eq!(std::fs::read(dir.path()).unwrap(), first);
    }

    #[test]
    fn install_updates_canopys_text() {
        let dir = Temp::new();
        std::fs::create_dir_all(dir.0.join("canopy")).unwrap();
        std::fs::write(dir.path(), format!("old\n{MARK}")).unwrap();
        assert!(install(&dir.0).unwrap());
        assert_eq!(std::fs::read_to_string(dir.path()).unwrap(), text());
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
        assert!(!dir.0.join("canopy").exists());
        install(&dir.0).unwrap();
        let extra = dir.0.join("canopy/extra");
        std::fs::write(&extra, "keep").unwrap();
        assert!(remove(&dir.0).unwrap());
        assert!(!dir.path().exists());
        assert_eq!(std::fs::read_to_string(extra).unwrap(), "keep");
    }

    #[test]
    fn skill_has_discoverable_frontmatter() {
        assert!(TEXT.starts_with("---\nname: canopy\ndescription: "));
    }
}
