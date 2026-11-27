// Modified for Canopy.
use std::process::Command;

#[test]
fn skill_prints_the_window_manual() {
    let output = Command::new(env!("CARGO_BIN_EXE_canopy")).arg("--skill").output().unwrap();
    assert!(output.status.success());
    let skill = String::from_utf8(output.stdout).unwrap();
    assert!(skill.starts_with(include_str!("../src/skill/SKILL.md").trim_end()));
    // The loaded part stays short; references hold the depth.
    assert!(include_str!("../src/skill/SKILL.md").lines().count() < 200, "SKILL.md must stay under 200 lines");
    assert!(skill.starts_with("---\nname: canopy\n"));
    assert!(skill.contains("hc --skill"));
    for reference in ["simulator", "browser", "panes-and-canvas", "mods", "customize"] {
        assert!(skill.contains(&format!("<!-- references/{reference}.md -->")), "{reference}");
    }
}
