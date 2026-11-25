# P7 - macOS desktop notifications (Codex)

Branch `feat/desktop-notifications` (already pushed; open your PR into `integration/ui-round`). Owner: Codex. Depends on: -.
For the whole round (other plans, merge order, names) read `00-orchestrator.md` in this folder.

## Shared rules
- Read `AGENTS.md`, `AGENTS.local.md` and `docs/canopy/herdr-map.md` first. Load the
  ponytail skill (full) and finish with ponytail-review, then fix its findings.
- Your branch already exists and carries these plans. Create your own worktree on it, using the shared
  `CARGO_TARGET_DIR` the orchestrator gives you. Never create a per-worktree `target/`.
  Run `export PATH="$(brew --prefix zig@0.16)/bin:$PATH"` on macOS.
- Checks: `cargo test --locked` (scoped with `-- <module>` while iterating), plus `just test`
  when mods change. Do not run `scripts/dev`, the app, `just install` or a bare binary: the
  orchestrator owns launches. On Linux, UI checks go only through `scripts/sandbox`.
- Types: use one enum instead of booleans that must agree. Parse JSON args once, at the
  boundary. Every user-visible error is a toast. Match exhaustively, with no `_ =>`.
- No em dashes. Commits are `type(scope): description`, in a few logical commits. Push to your
  branch (standing authorization for the canopy repo), open a PR into `integration/ui-round`
  (never main), and merge nothing yourself.
- Docs travel with the change. Update the `docs/wiki/` page named in your plan (what it is,
  how to use it, how it works; follow `docs/canopy/doc-style.md` and `doc-owners.md`). Update
  `defaults/AGENTS.md` and `src/SKILL.md` when agent-facing behavior changes. Never edit
  CHANGELOG.md.
- End with a short report: what changed, how to see it, what you could not verify.

---

## Decisions (from Krishna)
- **Notifications**: a macOS system notification when Canopy isn't the front app. It uses the
  app icon. The body is the agent's OSC 9/777 text, else the terminal title, else a generic
  line. Settings points to macOS's own banner (temporary) vs alert (persistent) style.
  Linux comes later.
- **Color**: each space has a color, and a pane inherits it unless it has its own. A wire takes
  the color of the pane it starts from (born wires: the parent; drawn wires: end `a`). The
  color also shows in the status strip and on the edges. Tints are pale on dark themes and a
  touch darker on light themes.
- **Edges**: drawn under panes. An edge is drawn above panes only while both of its endpoints
  are selected, or the wire itself is selected.
- **Browser wiring**: per pane, not per tab. A docked page's tab shows the wire color dot.
- **Drawer**: browser and simulator open in the Files panel by default. Pinned stays as a
  third place.
- **Annotate**: one feature, combining Claude's and ChatGPT's annotate flows, in Canopy's
  design language:
  - tools are icon-only (select, pen, box, text, X); actions are text-only pills;
  - color and stroke show only once a tool needs them;
  - the bar can't be dragged;
  - an inline single-line comment box, Shift+Enter for a new line, with an Add ↵ /
    Send ⌘↵ menu on the send button;
  - no mic, no sliders;
  - the target agent is chosen in the browser chrome next to Annotate, never inside the
    page; with exactly one wired agent, no picker at all;
  - react-grab's own toolbar is removed.
- **Linux**: built on `feat/linux-desktop-integration`, which must merge first. Window
  controls and the menu move into the top strip, one row.
- **SVG**: a preview picture for the Canopy layout card in Settings > Layouts.

## The task
**Pieces.**
1. **OSC capture.**
   - Register `GHOSTTY_TERMINAL_OPT_DESKTOP_NOTIFICATION` (id 29;
     `GhosttyTerminalDesktopNotificationFn`, `vendor/ghostty/include/ghostty/vt/terminal.h:1108`,
     bound by bindgen in `crates/ghostty-sys`) in `Vt::new` next to the title callback
     (`src/vt.rs:641, 686`).
   - Keep `last_notice: Option<(String, Instant)>`, truncated to 200 characters with control
     characters stripped, because agent output is untrusted.
   - **First, verify** that a `Vt` exists for terminals that aren't on screen. If only drawn
     terminals have one, take the text only when present; never attach a terminal just to
     read OSC.
2. **Posting.**
   - New `src/notify_mac.rs`, `#[cfg(target_os = "macos")]`, using `objc2-user-notifications`
     (the same objc2 family already in Cargo.toml:71-77; license Zlib OR Apache-2.0 OR MIT;
     state its size before adding).
   - Use `UNUserNotificationCenter`, guarded: only when `NSBundle.mainBundle.bundleIdentifier`
     is `dev.canopy.app`, because an unbundled process crashes on `currentNotificationCenter`.
   - Request authorization (alert and sound off, since Canopy plays its own sounds) on the
     first alert, not at startup.
   - A delegate's `didReceiveNotificationResponse` reads `userInfo.address` and runs
     `agent.focus`.
   - `willPresent` returns none, because the app is in front and a toast shows instead.
3. **Wiring in `fn alert`** (`window.rs:2186-2213`).
   - If the app is not active (`NSApplication.isActive`, or the gpui window active state),
     post the system notification. Otherwise keep today's toast.
   - Title: "{name} needs you" or "{name} finished".
   - Body: the OSC text if it is younger than 60 s, else the terminal title, else empty.
   - Sounds stay as they are.
4. **Info.plist.** Add `NSUserNotificationAlertStyle` = `banner` in `scripts/install-macos` and
   `scripts/mac-dmg`'s plist, and check the dmg path builds the same plist.
5. **Settings > Alerts** (`defaults/frame/settings/main.js:893-911`).
   - Add a `desktop_notifications` toggle (`person_settings!` in settings.rs:386-433, plus
     `Settings`/`compose`).
   - Add a line explaining Banners (temporary) vs Alerts (stay until dismissed), and a text
     button "Notification style" that opens
     `x-apple.systempreferences:com.apple.Notifications-Settings.extension?id=dev.canopy.app`
     (verify that this URL opens Canopy's page on the installed macOS).
   - Add a hint that agents only include their message when their own terminal notifications
     are on. Name no specific agent unless it is installed, per the UI rule, so write it
     generically.
6. **Harness facts.** Check which installed agents emit OSC 9/777 and under which of their
   settings. Write that into `docs/canopy/harness-notes/` with versions. Never change their
   config.

**Docs.** `docs/wiki/Agents.md` (alerts), `Settings-and-Themes.md`.
**Can't verify in CI.** Only the installed .app shows real notifications; the orchestrator
checks after `just install`, with Krishna's OK.
