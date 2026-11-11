use anyhow::{Context, Result};
use std::os::unix::fs::DirBuilderExt;
use std::path::PathBuf;

/// `$CANOPY_HOME`, or `~/.canopy`. Created 0700 on first use.
pub fn home() -> Result<PathBuf> {
    let home = match std::env::var_os("CANOPY_HOME") {
        Some(h) => PathBuf::from(h),
        None => PathBuf::from(std::env::var_os("HOME").context("HOME is not set")?).join(".canopy"),
    };
    validate_socket(&home)?;
    std::fs::DirBuilder::new().recursive(true).mode(0o700).create(&home)?;
    Ok(home)
}

/// sockaddr_un reserves one byte for the terminating NUL.
pub fn validate_socket(home: &std::path::Path) -> Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let limit = if cfg!(target_os = "macos") { 104 } else { 108 };
    let length = home.join("sock").as_os_str().as_bytes().len();
    anyhow::ensure!(
        length < limit,
        "CANOPY_HOME socket path is {length} bytes; Unix socket paths must be shorter than {limit} bytes including /sock. Set CANOPY_HOME to a shorter absolute path, for example /tmp/canopy"
    );
    Ok(())
}

/// Git toplevel of cwd, else cwd.
pub fn project_root(cwd: &str) -> String {
    std::path::Path::new(cwd)
        .ancestors()
        .find(|dir| dir.join(".git").exists())
        .map(|dir| dir.to_string_lossy().into_owned())
        .unwrap_or_else(|| cwd.to_owned())
}

#[cfg(test)]
mod tests {
    #[test]
    fn project_roots() {
        let temp = std::env::temp_dir().join(format!("canopy-project-roots-{}", std::process::id()));
        std::fs::create_dir_all(&temp).unwrap();
        let repo = temp.join("repo");
        let nested = repo.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(repo.join(".git"), "gitdir: elsewhere").unwrap();
        assert_eq!(super::project_root(nested.to_str().unwrap()), repo.to_str().unwrap());
        assert_eq!(super::project_root(temp.to_str().unwrap()), temp.to_str().unwrap());
        std::fs::remove_dir_all(temp).unwrap();
    }
}
