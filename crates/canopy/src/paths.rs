use anyhow::{Context, Result};
use std::os::unix::fs::DirBuilderExt;
use std::path::PathBuf;

/// `$CANOPY_HOME`, or `~/.canopy`. Created 0700 on first use.
pub fn home() -> Result<PathBuf> {
    let home = match std::env::var_os("CANOPY_HOME") {
        Some(h) => PathBuf::from(h),
        None => PathBuf::from(std::env::var_os("HOME").context("HOME is not set")?).join(".canopy"),
    };
    std::fs::DirBuilder::new().recursive(true).mode(0o700).create(&home)?;
    Ok(home)
}
