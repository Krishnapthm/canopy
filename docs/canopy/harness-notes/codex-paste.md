# Codex CLI 0.169.0, 2026-11-21

Source/bundle inspection, no live session. Evidence comes from Canopy
source, strings and disassembly of the installed Codex binary, and the
local Crossterm source identified below. Byte offsets are decimal. Hex
addresses are Mach-O virtual addresses as printed by `llvm-objdump -d`.
Earlier checks are in [Codex](codex.md).

## 1. Installed CLI

`codex --version` returned `codex-cli 0.169.0`. `command -v codex`
returned `/Users/krishna/.local/bin/codex`, a symlink through
`~/.codex/packages/standalone/current/bin/codex` to
`/Users/krishna/.codex/packages/standalone/releases/0.169.0-aarch64-apple-darwin/bin/codex`.
It is an arm64 Rust Mach-O. The adjacent release's `codex-package.json`
lists version `0.169.0` and entrypoint `bin/codex`. The release package,
Cargo checkouts and registry, and npm cache had no matching Codex TUI
source. The executable embeds `tui/src/bottom_pane/chat_composer.rs`
and `tui/src/bottom_pane/paste_burst.rs`. The version/help calls printed
the sandbox warning about being unable to create PATH aliases.

**Confidence.** High for the installed version and binary. Source paths in
the executable identify the build's files.

## 2. Bracketed paste

Codex ships the mode enable `ESC[?2004h` (217808943), literal
`ESC[200~` and `ESC[201~` (218111392 and 218111399), and
`assertion failed: buffer.starts_with(b"\x1B[200~")` (218110330).
The binary names its Crossterm checkout as
`/Users/runner/.cargo/git/checkouts/crossterm-ce0318b92aca4e8d/ed1cdab/src/event/sys/unix/parse.rs`
(212166556-212167190). The available local reference is
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/crossterm-0.29.0/`,
a different revision. In that source `src/event.rs` has
`EnableBracketedPaste` and `csi!("?2004h")`. In
`src/event/sys/unix/parse.rs`, `parse_csi_bracketed_paste` buffers until the
close marker and emits one `Event::Paste`. The installed binary's
`ChatWidget::handle_paste` (0x10a7c8020) and the composer call reached
via 0x10a89c76c handle the resulting text as one event.

Canopy's annotation path is `send_annotation`
(`src/window/canvas_view/sim_screen.rs:523`) to `AppWindow::paste_agent`
(`src/window/canvas_view.rs:2128`). `format_quote` produces
`@<path>\n<caption>`, using a relative path if the PNG is under the agent's
working directory. The simulator caption is
`Annotated screenshot of <name> (<runtime>), <w>x<h> points. The red marks show what to look at.`
The formatter leaves the path unquoted, strips trailing CR/LF
from the caption, and appends `\n\n<trimmed message>` only when an optional
message is supplied. `TerminalView::paste_quote` (`src/terminal.rs:750`)
requires an attached channel in mode 2004. `Vt::paste_bytes`
(`src/vt.rs:1504`) sends `1b 5b 32 30 30 7e`, the UTF-8 body with all
embedded closing markers removed, then `1b 5b 32 30 31 7e`. The repeated
removal also catches markers formed during removal. Body line endings pass
through. `TerminalView::send` uses `Core::input` (`src/hc.rs:522`) to send
the bytes as a DATA payload and can mirror them to synced terminals.
`paste_agent` focuses the target after writing. It sends no Enter. Its
100 ms retries up to 30 s wait for readiness before the paste.

**Confidence.** High for Canopy's bytes and the installed paste support.
The installed build's mode announcement and one-event behavior still need
a PTY trace.

## 3. Enter after paste

The available Crossterm parser consumes one byte at a time and clears its
buffer when an event is complete (`Parser::advance` in
`src/event/source/unix/tty.rs` and `mio.rs`). A CR immediately after the
close marker starts a separate `KeyCode::Enter` event, even in a single
write. This matches the installed binary's marker and parser evidence.
The installed explicit paste path calls
`PasteBurst::flush_before_modified_input` at 0x10a21726c, then passes
the paste on to the composer. The binary also embeds `disable_paste_burst`
(213043801), and has a separate `handle_paste_burst_tick` symbol.

The rapid-key heuristic uses **8 ms** on this build. The executable compares
an elapsed duration with 8,000,001 ns at 0x10a21024c and 0x10a218690, and
with 8,000,000 ns at 0x10a218bec. The first two encode a strict
greater-than-8-ms boundary. The rapid-key path also sets a **120 ms**
deadline by adding 120,000,000 ns after buffering a key
(0x10a2100dc-0x10a210110). The burst predicate at 0x10a210184 checks
that deadline alongside pending characters. A complete bracketed paste
goes through its own handler and takes no rapid-key delay on an idle
composer with no prior burst pending.
For a plain draft in the default idle composer, the next CR is expected to
submit, with **0 ms required by the parser or burst window**. The actual
Canopy quote includes an `@` prefix. Whether a mention picker consumes
Enter for that draft, or image preparation affects submit, remains open.

**Confidence.** High for the timing constants and the separate event and
paste paths. Medium for a same-write CR submitting a plain draft. There is
no verified delay to use for the annotation itself.

## 4. Image paths

Codex's candidate for an inline image is a paste of the **standalone bare
absolute path**, such as `/tmp/screen.png`. Matching single or double
quotes also work for the candidate, including
`'/tmp/my screen.png'` and `"/tmp/my screen.png"`. The path parser reached
from the composer at 0x10a219450 trims the input, strips matching quotes
(0x10acf5dd4), and validates a whole path (0x10acf6208). It checks for
an absolute path form and also has a local `file:` URL branch. The bare
`~/screen.png` and `@/tmp/screen.png` forms fail the absolute-path check.
A paste containing the path plus a caption cannot be treated as that
single path. Use a separate paste for the path before adding the text.
The path must be readable and supported by the active model.

Binary evidence connects the path candidate to the image branch in the
composer. `tui/src/bottom_pane/chat_composer.rs` sits beside
`image dimensions=`, `attached image format=`, and
`selected image dimensions=` (212095356-212095500). The executable also
has `attach_image path=` under `tui/src/chatwidget/input_flow.rs`
(212107563), `local_image_paths`, and `[Image #` (212026139, with a
format argument before the closing bracket). The image clipboard action
`fixed.paste_image` and `paste_image_as_png` are a separate input path.
The existing `@` file-search UI has its own `tui/src/file_search.rs`
(212113715). Evidence does not establish that submitting pasted
`@/tmp/screen.png`, a quoted `@` mention, or `@~/screen.png` attaches
image bytes. `codex --help` separately confirms `-i, --image <FILE>...`
for the initial prompt at launch.

**Confidence.** High for the candidate path checks. Medium for the
attachment outcome in the default composer and for exact `@` mention
behavior at submit.

## 5. Other PTY details

- The general `Vt::paste_bytes` fallback turns CRLF and LF into CR when
  mode 2004 is off. The quote path waits for mode 2004. Canopy's usual
  Enter key sends CR, with mode-dependent alternatives such as
  `ESC[13u` (`src/terminal.rs:4281`). The local Crossterm source maps raw
  LF to Ctrl+J and CR to Enter outside paste.
- The Codex composer path starting at 0x10a218c18 replaces CRLF with LF
  (the operands for the call at 0x10a218c68), then replaces remaining CR
  with LF (0x10a218d58-0x10a218d74).
- Normalized text over 1000 Unicode scalar values takes a large-paste
  branch in the composer (0x10a21941c-0x10a219428). The binary contains
  `[Pasted Content ` and ` chars]` (212094203) and
  `ChatComposer::expand_pending_pastes`. The display cutoff gives no
  hard end-to-end paste limit.
- In the available Crossterm reference, an incomplete bracketed frame
  yields no event. Ordinary ESC and `0x03` inside a complete frame become
  paste data. The raw-mode setup uses `cfmakeraw`, which prevents the
  PTY from generating SIGINT for input byte `0x03`. The close delimiter
  terminates accumulation. The installed fork's handling of malformed
  frames, including any timeout, and the composer's treatment of the
  pasted control bytes remain unverified. Canopy's
  `bracketed_paste_cannot_end_itself_early` test covers sender sanitization.

**Confidence.** High for Canopy's fallback and the compiled composer
normalization. Medium for the large-paste and control-byte findings.

## Open questions for a live check

With the user's permission, use an isolated Codex session in a test PTY
with an approved provider or a local model that supports images. Capture
stdout and the write boundaries.

1. Wait for `ESC[?2004h`. Send one multiline framed text paste without
   CR and confirm it stays in the editor. Compare a same-write CR with
   a separate CR after 10 ms and a human Enter. Repeat with and without
   the literal Canopy `@path\ncaption` body. Record picker state,
   submitted text, and number of turns. Also repeat after a burst of
   unframed character keys to test whether prior burst state affects Enter.
2. Paste a readable PNG path alone, then both quoted forms of a path with
   spaces. Try `~/`, the bare and quoted `@` forms, and path plus caption
   in the same frame. Compare the draft pill and actual submitted image
   items. Repeat with the path in one frame and caption in a second.
3. For an accepted image path, compare immediate CR with Enter after the
   attachment appears. Verify whether image preparation delays or
   suppresses submit.
4. Split a frame across writes. Try delays of 1 s and 3 s before its close
   marker. In fresh drafts, compare LF, CRLF, ESC and `0x03` inside the
   frame with keys sent afterward. Paste around 1000 characters and then
   increase the size without submitting, recording any truncation or
   lost input.

## Live check, 2026-11-21 (Codex CLI 0.171.0)

Driven in a PTY (120x40) with Canopy's bytes, one prompt submitted.

- A bracketed paste of a bare absolute PNG path attaches: the draft shows `[Image #1]`.
- Image paste, settle, text paste, settle, `\r`: submitted once, the image arrived (the model
  named its colour).
- Codex redraws its status line continuously, so "unchanged for 300 ms" never holds: each wait
  reached Canopy's 3 s cap. Sending still works; it is 3 s slower per step than it needs to be.
- At start Codex may show an update prompt whose default is "Update now". Anything that types
  into a fresh Codex must not press Enter on it (Esc skips). Canopy only types into an agent
  the person already uses, past that prompt.
