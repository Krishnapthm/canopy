// Modified for Canopy.
// Windows: a window app, but tests inherit Cargo's console for their child processes.
#![cfg_attr(all(windows, not(test)), windows_subsystem = "windows")]

mod bench;
mod app_menu;
mod commands;
mod control;
mod core_bin;
mod files;
mod hc;
mod keys;
mod canvas;
mod layout;
mod layout_sync;
mod links;
mod model;
mod mirror;
mod mirror_sync;
mod panes;
mod sim;
#[cfg(target_os = "macos")]
mod sim_live;
mod skill;
mod place;
mod persist;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
mod macos_check;
mod mods;
mod mod_files;
mod process_cwd;
mod subs;
mod programs;
mod remote_shell;
mod settings;
mod setup;
mod sound;
mod spawn;
mod sprites;
mod terminal;
mod vt;
mod win32_keys;
mod window;
mod wire;

use gpui::*;

const HELP: &str = "\
canopy: the Canopy desktop app. With no arguments it opens the window.

Usage:
  canopy                        open the window
  canopy ls                     spaces, their tabs and terminals, in the open window
  canopy do <command> [args]    run a command: --name api --argv '[\"pi\"]', or one JSON object
  canopy commands               every command, its arguments, what it does
  canopy status                 what the window shows, and any errors
  canopy shot [file.png]        save the last window frame (default: canopy-shot.png)
  canopy hook sub-start|sub-stop|session-end|wire|idle  report a session hook (JSON on stdin, best effort)
  canopy wire                   who you are, and the peers and panes wired to you
  canopy pane new file <path> [--wire] | read|shot|close <pane> | open <pane> <target>  canvas panes
  canopy spawn <program> --name <n> [--model <m>] [-- <message>]  start an agent beside this one, wired to it
  canopy check <mod folder>     load a mod once and report what's wrong
  canopy config                 Settings defaults and explanations
  canopy --skill                Agent Skill: driving the window, and where hc fits
  canopy skill remove           delete the skill Canopy installed for your agents
  canopy --version

The terminals run in herdr-core (`hc`), which keeps them alive when the
window closes. `hc --help`: start, read, wait on and talk to terminals and
agents. The app shows them and arranges them in spaces and tabs.

Home: ~/.config/canopy (HERDR_APP_HOME picks another).
";

/// A Windows window app has no console: answers to `canopy ls` and the
/// like go to the terminal it was started from. Output already piped (agents,
/// scripts) keeps its pipes.
#[cfg(windows)]
fn attach_console() {
    use windows_sys::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_OUTPUT_HANDLE};
    // Attaching would point the output at the console instead of the pipe or file.
    let out = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
    if out.is_null() || out == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
        unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // A live sim pane's helper, started by Canopy itself under sandbox-exec: nothing else runs.
    #[cfg(target_os = "macos")]
    if args.get(1).is_some_and(|s| s == "--sim-helper") {
        sim_live::helper_main(&args[2..]);
    }
    #[cfg(unix)]
    core_bin::hand_off_to_owner(&args);
    if args.get(1).is_some_and(|s| s == "hook") {
        match args.get(2).map(String::as_str) {
            Some("wire") => wire::hook(),
            Some("idle") => wire::hook_idle(),
            action => subs::hook(action),
        }
        return;
    }
    if args.get(1).is_some_and(|s| s == "wire") {
        std::process::exit(wire::cli(&args[2..]));
    }
    if args.get(1).is_some_and(|s| s == "spawn") {
        std::process::exit(wire::spawn_cli(&args[2..]));
    }
    if args.get(1).is_some_and(|s| s == "pane") { std::process::exit(panes::cli(&args[2..])); }
    if args.get(1).is_some_and(|s| s == "skill") { std::process::exit(skill::cli(&args[2..])); }
    #[cfg(target_os = "macos")]
    macos_check::startup(&args);
    #[cfg(windows)]
    if args.len() > 1 {
        attach_console();
    }
    // `canopy check <mod>` / `canopy types <mod>`: gpui-shell's own checker, with Canopy's module installed.
    if matches!(args.get(1).map(String::as_str), Some("check" | "types")) {
        mods::install_check_policy();
        gpui_shell::host::main_with_components(gpui_shell::FrozenComponentRegistry::default());
        return;
    }
    if matches!(args.get(1).map(String::as_str), Some("do" | "status" | "ls" | "commands" | "shot")) {
        std::process::exit(control::client(&args[1..]));
    }
    // Anything else from a shell answers in the shell; only a bare launch opens the window.
    match args.get(1).map(String::as_str) {
        // Already open on this home: that window comes to the front.
        None if control::raise_running() => return,
        None => {}
        Some("-h" | "--help" | "help") => return print!("{HELP}"),
        Some("config") => return print!("{}", settings::config()),
        Some("--skill") => return print!("{}", include_str!("SKILL.md")),
        Some("-V" | "--version" | "version") => return println!("canopy {}", env!("CARGO_PKG_VERSION")),
        Some(other) => {
            eprint!("canopy: unknown argument {other:?}\n\n{HELP}");
            std::process::exit(2);
        }
    }

    #[cfg(target_os = "macos")]
    core_bin::fill_path();
    #[cfg(unix)]
    core_bin::put_cli_on_path();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("HERDR_APP_LOG")
                .unwrap_or_else(|_| "warn,canopy=info,gpui_shell=info".into()),
        )
        .init();

    // A first start sizes the app to the screen and the system's text size.
    let fresh = !settings::path().exists();
    // The core people installed; without one, show manual setup instructions.
    let person = settings::load_person();
    let configured = person.as_ref().ok().and_then(|p| p.core.clone());
    let core: Result<std::sync::Arc<hc::Core>, setup::Need> = match core_bin::find(configured.as_deref()) {
        None => Err(setup::Need::Install),
        Some(bin) => {
            core_bin::use_core(bin);
            match hc::Core::connect() {
                // One that can't serve this app gets the screen that fixes it.
                Ok(core) => hc::check(&core.hello).map(|()| core).map_err(setup::Need::Fix),
                Err(e) => {
                    eprintln!("canopy: {e:#}");
                    Err(setup::Need::connection(e))
                }
            }
        }
    };
    #[cfg(target_os = "macos")]
    core_bin::install_cli_link();
    bench::start();
    if let Err(e) = mods::write_workspace() {
        eprintln!("canopy: cannot write {}: {e}", mods::app_dir().display());
    }

    gpui_platform::application()
        // Mods run programs already; the user's own links (dotfiles, Nix) may point out.
        .with_assets(gpui_shell::AppAssets::new(mods::app_dir()).following_links())
        .run(move |cx| {
            let fonts = settings::BUNDLED_FONTS.iter().map(|f| std::borrow::Cow::Borrowed(*f)).collect();
            if let Err(e) = cx.text_system().add_fonts(fonts) {
                tracing::warn!("bundled fonts not loaded: {e:#}");
            }
            settings::note_fonts(cx.text_system().all_font_names());
            settings::load_home_fonts(cx);
            mods::init(cx);
            cx.on_app_quit(|cx| {
                let deadline = std::time::Instant::now() + std::time::Duration::from_millis(250);
                // Where the window was, for the next start (a quit never asks the window).
                for w in cx.windows() {
                    if let Some(w) = w.downcast::<window::AppWindow>() {
                        let _ = w.update(cx, |this, window, cx| {
                            this.drain_documents(window, cx);
                            this.close_mods_now(deadline, window, cx);
                            place::save(window, cx);
                        });
                    }
                }
                mods::finish_shared();
                // GPUI drops the windows before polling this future, so WebKit can remove the store.
                async { drop(window::forget_browser_data_if_until_quit()); }
            })
            .detach();
            #[cfg(target_os = "macos")]
            macos::init(cx);
            if fresh {
                let width = cx.primary_display().map(|d| f32::from(d.bounds().size.width));
                let zoom = settings::first_zoom(width);
                if zoom != 1.0 {
                    let _ = settings::write_person(&settings::Person { zoom, ..Default::default() });
                }
            }
            let requests = mods::install_policy();
            if let Some(tx) = mods::request_sender() {
                control::listen(tx);
            }
            match core {
                Ok(core) => open_main(core, requests, cx),
                Err(need) => setup::open(need, requests, cx),
            }
        });
}

const LINUX_APP_ID: &str = "canopy";

/// Identifies Canopy to the desktop shell. Linux matches the installed canopy.desktop.
fn desktop_app_id() -> String {
    std::env::var("HERDR_APP_ID")
        .ok()
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| if cfg!(target_os = "linux") { LINUX_APP_ID } else { "dev.canopy.app" }.into())
}

#[cfg(test)]
mod desktop_tests {
    use super::LINUX_APP_ID;

    #[test]
    fn linux_app_id_matches_both_installers() {
        let wm_class = format!("StartupWMClass={LINUX_APP_ID}");
        let icon = format!("Icon={LINUX_APP_ID}");
        for installer in [
            include_str!("../scripts/install-linux"),
            include_str!("../distribution/install.sh"),
        ] {
            assert!(installer.contains(&format!("applications/{LINUX_APP_ID}.desktop")));
            assert!(installer.lines().any(|line| line == wm_class));
            assert!(installer.lines().any(|line| line == icon));
        }
    }
}

/// The main window, on a connected core.
pub fn open_main(core: std::sync::Arc<hc::Core>, requests: async_channel::Receiver<mods::Request>, cx: &mut App) {
    // Never waits on WebKit: a page opened before the removal ends keeps its cookies for one more run.
    if let Some(removal) = window::forget_browser_data_if_until_quit() {
        cx.background_executor().spawn(async move {
            if let Err(e) = removal.await { tracing::warn!("cannot remove the browser data store at startup: {e}"); }
        }).detach();
    }
            let (window_bounds, display_id) = place::initial(cx);
            let options = WindowOptions {
                window_bounds: Some(window_bounds),
                display_id,
                // macOS: always see-through, so a layout can draw its own title bar
                // live; the app drags the window (window/titlebar.rs).
                titlebar: Some(TitlebarOptions {
                    title: Some("Canopy".into()),
                    // Windows: hidden for good (it can't change live); the app draws it.
                    appears_transparent: cfg!(any(target_os = "macos", windows)),
                    traffic_light_position: cfg!(target_os = "macos").then(|| point(px(12.0), px(13.0))),
                }),
                app_owns_titlebar_drag: cfg!(target_os = "macos"),
                // Linux: open with the title bar the shipped layouts draw. Turning the
                // system's off on the first render, while KWin maps the window, shrinks
                // it to 20x20; turning it on later (a "system" layout) is fine.
                window_decorations: Some(WindowDecorations::Client),
                // Canopy Dev and other side-by-side builds get their own dock icon.
                app_id: Some(desktop_app_id()),
                ..Default::default()
            };
            cx.open_window(options, |window, cx| cx.new(|cx| window::AppWindow::new(core, requests, window, cx)))
                .expect("cannot open the Canopy window");
            cx.activate(true);
}
