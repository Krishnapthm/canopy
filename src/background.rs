//! Runs the macOS Dock tile after the main app quits. The core owns the terminals.

use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{
    AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel,
};
use objc2_app_kit::{
    NSAlert, NSAlertFirstButtonReturn, NSApplication, NSApplicationActivationPolicy,
    NSApplicationDelegate, NSImage, NSMenu,
};
use objc2_foundation::{
    NSBundle, NSData, NSNotification, NSObjectNSThreadPerformAdditions, NSString, ns_string,
};
use serde::Deserialize;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::io::{self, BufRead, BufReader};
use std::os::unix::{ffi::OsStrExt, process::CommandExt};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

const CORE_BIN: &str = "CANOPY_BACKGROUND_CORE";

fn pidfile() -> PathBuf {
    crate::mods::app_dir().join("background.pid")
}

fn detached_app() -> io::Result<Command> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // Each launch gets a private session, independent of its parent.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    Ok(command)
}

/// Starts the tile with the same core executable and session as the window.
pub fn spawn() -> io::Result<()> {
    detached_app()?
        .arg("--background")
        .env(CORE_BIN, crate::core_bin::path())
        .spawn()?;
    Ok(())
}

/// Retires a tile from this app home before opening the full app.
pub fn retire() {
    let path = pidfile();
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return;
    };
    if let Ok(pid) = contents.trim().parse::<libc::pid_t>()
        && pid > 1
        && pid as u32 != std::process::id()
        && let Ok(exe) = std::env::current_exe().and_then(std::fs::canonicalize)
    {
        let mut buf = [0; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
        let length = unsafe { libc::proc_pidpath(pid, buf.as_mut_ptr().cast(), buf.len() as u32) };
        if length > 0
            && std::fs::canonicalize(OsStr::from_bytes(
                buf.split(|&byte| byte == 0).next().unwrap(),
            ))
            .ok()
            .as_ref()
                == Some(&exe)
            && unsafe { libc::getsid(pid) } == pid
            && unsafe { libc::getpgid(pid) } == pid
        {
            // The tile's private group includes its events reader, so neither can linger.
            unsafe { libc::kill(-pid, libc::SIGTERM) };
        }
    }
    let _ = std::fs::remove_file(path);
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Fact {
    Snapshot { terminals: Vec<Terminal> },
    Event { params: Event },
}

#[derive(Deserialize)]
struct Terminal {
    address: String,
    state: State,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum State {
    Live,
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Event {
    Created {
        data: Terminal,
    },
    Finished {
        terminal: String,
    },
    Removed {
        terminal: String,
    },
    #[serde(other)]
    Other,
}

impl Fact {
    fn apply(self, live: &mut HashSet<String>) {
        match self {
            Self::Snapshot { terminals } => {
                *live = terminals
                    .into_iter()
                    .filter_map(|t| matches!(t.state, State::Live).then_some(t.address))
                    .collect();
            }
            Self::Event {
                params:
                    Event::Created {
                        data:
                            Terminal {
                                address,
                                state: State::Live,
                            },
                    },
            } => {
                live.insert(address);
            }
            Self::Event {
                params: Event::Finished { terminal } | Event::Removed { terminal },
            } => {
                live.remove(&terminal);
            }
            Self::Event {
                params:
                    Event::Created {
                        data:
                            Terminal {
                                state: State::Other,
                                ..
                            },
                    }
                    | Event::Other,
            } => {}
        }
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    struct BadgeTarget;

    impl BadgeTarget {
        #[unsafe(method(updateBadge:))]
        fn update_badge(&self, label: &NSString) {
            let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
            if label.is_empty() {
                app.terminate(None);
            } else {
                app.dockTile().setBadgeLabel(Some(label));
            }
        }
    }
);

fn watch_events(stdout: impl io::Read) {
    let target: Retained<BadgeTarget> = unsafe { msg_send![BadgeTarget::alloc(), init] };
    let send = |label: &str| {
        objc2::rc::autoreleasepool(|_| unsafe {
            target.performSelectorOnMainThread_withObject_waitUntilDone(
                sel!(updateBadge:),
                Some(&NSString::from_str(label)),
                false,
            );
        })
    };
    let mut live = HashSet::new();
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        let Ok(fact) = serde_json::from_str::<Fact>(&line) else {
            continue;
        };
        let before = live.len();
        fact.apply(&mut live);
        if live.is_empty() {
            break;
        }
        if live.len() != before {
            send(&live.len().to_string());
        }
    }
    send("");
}

struct Tile {
    core: PathBuf,
    events: std::cell::RefCell<Child>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Tile]
    struct TileDelegate;

    unsafe impl NSObjectProtocol for TileDelegate {}

    unsafe impl NSApplicationDelegate for TileDelegate {
        #[unsafe(method_id(applicationDockMenu:))]
        fn dock_menu(&self, _: &NSApplication) -> Option<Retained<NSMenu>> {
            let menu = NSMenu::new(self.mtm());
            for (title, action) in [
                (ns_string!("Open Canopy"), sel!(openCanopy:)),
                (ns_string!("Quit Completely"), sel!(quitCompletely:)),
            ] {
                unsafe { menu.addItemWithTitle_action_keyEquivalent(title, Some(action), ns_string!("")).setTarget(Some(self)); }
            }
            Some(menu)
        }

        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn should_reopen(&self, _: &NSApplication, _: bool) -> bool {
            self.open();
            false
        }

        #[unsafe(method(applicationWillTerminate:))]
        fn will_terminate(&self, _: &NSNotification) {
            clean_up(&mut self.ivars().events.borrow_mut());
        }
    }

    impl TileDelegate {
        #[unsafe(method(openCanopy:))]
        fn open_from_menu(&self, _: &NSObject) {
            self.open();
        }

        #[unsafe(method(quitCompletely:))]
        fn quit_completely(&self, _: &NSObject) {
            let alert = NSAlert::new(self.mtm());
            alert.setMessageText(ns_string!("Stop all terminals?"));
            alert.setInformativeText(ns_string!("Quitting completely stops the core and every terminal and agent in it."));
            alert.addButtonWithTitle(ns_string!("Quit Completely"));
            alert.addButtonWithTitle(ns_string!("Cancel"));
            if alert.runModal() != NSAlertFirstButtonReturn { return; }
            match Command::new(&self.ivars().core).args(["server", "stop"]).output() {
                Ok(out) if out.status.success() => NSApplication::sharedApplication(self.mtm()).terminate(None),
                Ok(out) => show_error(self.mtm(), "Could not stop the core", &String::from_utf8_lossy(&out.stderr)),
                Err(err) => show_error(self.mtm(), "Could not stop the core", &err.to_string()),
            }
        }
    }
);

impl TileDelegate {
    fn open(&self) {
        match detached_app().and_then(|mut cmd| cmd.env_remove(CORE_BIN).spawn()) {
            Ok(_) => NSApplication::sharedApplication(self.mtm()).terminate(None),
            Err(err) => show_error(self.mtm(), "Could not open Canopy", &err.to_string()),
        }
    }
}

fn show_error(mtm: MainThreadMarker, title: &str, detail: &str) {
    let alert = NSAlert::new(mtm);
    alert.setMessageText(&NSString::from_str(title));
    alert.setInformativeText(&NSString::from_str(detail));
    alert.runModal();
}

fn clean_up(events: &mut Child) {
    let _ = events.kill();
    let _ = events.wait();
    let path = pidfile();
    if std::fs::read_to_string(&path).ok().as_deref() == Some(&std::process::id().to_string()) {
        let _ = std::fs::remove_file(path);
    }
}

pub fn run() -> io::Result<()> {
    let mtm = MainThreadMarker::new().unwrap();
    let core = std::env::var_os(CORE_BIN)
        .map(PathBuf::from)
        .unwrap_or_else(crate::core_bin::path);
    let mut events = Command::new(&core)
        .arg("events")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let stdout = events.stdout.take().unwrap();
    let watched = crate::persist::write_atomic(&pidfile(), std::process::id().to_string())
        .and_then(|()| std::thread::Builder::new().spawn(move || watch_events(stdout)));
    if let Err(err) = watched {
        clean_up(&mut events);
        return Err(err);
    }

    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    if NSBundle::mainBundle().bundleIdentifier().is_none() {
        let data = NSData::with_bytes(include_bytes!("../assets/icons/Canopy.icns"));
        if let Some(icon) = NSImage::initWithData(NSImage::alloc(), &data) {
            unsafe {
                app.setApplicationIconImage(Some(&icon));
            }
        }
    }
    let delegate: Retained<TileDelegate> = unsafe {
        msg_send![
            super(TileDelegate::alloc(mtm).set_ivars(Tile {
                core,
                events: events.into()
            })),
            init
        ]
    };
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    app.run();
    clean_up(&mut delegate.ivars().events.borrow_mut());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_live_addresses_from_snapshot_and_events() {
        let snapshot = r#"{"channel":null,"seq":0,"terminals":[{"address":"t1","state":"live"},{"address":"old","state":"finished"}]}"#;
        let created = r#"{"jsonrpc":"2.0","method":"event","params":{"data":{"address":"t1","state":"live"},"seq":1,"terminal":"t1","type":"created"}}"#;
        let foreground = r#"{"jsonrpc":"2.0","method":"event","params":{"data":{"argv":["sleep","10"],"name":"sleep","pid":96109},"seq":2,"terminal":"t1","type":"foreground"}}"#;
        let finished = r#"{"jsonrpc":"2.0","method":"event","params":{"data":{"exit_code":null,"finished_ms":1795772868251,"reason":"killed","signal":1},"seq":3,"terminal":"t1","type":"finished"}}"#;
        let mut live = HashSet::new();
        for (line, expected) in [
            (snapshot, &["t1"][..]),
            (created, &["t1"]),
            (&created.replace("t1", "t2"), &["t1", "t2"]),
            (foreground, &["t1", "t2"]),
            (finished, &["t2"]),
            (
                &finished
                    .replace("t1", "t2")
                    .replace("finished\"", "removed\""),
                &[],
            ),
        ] {
            serde_json::from_str::<Fact>(line).unwrap().apply(&mut live);
            assert_eq!(
                live.iter().map(String::as_str).collect::<HashSet<_>>(),
                expected.iter().copied().collect()
            );
        }
    }
}
