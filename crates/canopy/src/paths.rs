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
