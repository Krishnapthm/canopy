use anyhow::Result;

/// Prints each built-in harness's version and login state. Runs only the CLIs' own
/// status commands, discarding their output (see docs/harness-notes/).
pub async fn doctor() -> Result<()> {
    let rows = crate::harness::detect().await;
    if rows.is_empty() {
        println!("No coding agent CLI found on your PATH.");
        return Ok(());
    }
    println!("HARNESS       INSTALLED                       RECORDED     STATUS");
    for row in rows {
        let status = if !row.supported {
            "detected, not supported yet".to_owned()
        } else if row.logged_in == Some(true) {
            "logged in".to_owned()
        } else {
            format!("login unknown or logged out; run {}", row.login_command)
        };
        println!("{:<13} {:<31} {:<12} {}", row.name, row.version.as_deref().unwrap_or("-"), row.recorded, status);
    }
    Ok(())
}
