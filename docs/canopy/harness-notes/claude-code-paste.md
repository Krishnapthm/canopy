# Claude Code 2.1.322, 2026-11-21

Source/bundle inspection, no live session. This note follows the annotation
through the installed prompt editor. Byte offsets are decimal and refer to
the executable below. Earlier checks are in [Claude Code](claude-code.md).

## 1. Installed CLI

`claude --version` returned `2.1.322 (Claude Code)`. Both `command -v claude`
and `realpath` returned
`/Users/krishna/.local/share/claude/ClaudeCode.app/Contents/MacOS/claude`.
It is an arm64 Mach-O with embedded Bun JavaScript. The embedded
`// @bun @bytecode` and `// Version: 2.1.322` precede the paste helpers
around byte 277981700.

**Confidence.** High. Version command, resolved path, file type and bundle agree.

## 2. Bracketed paste

Claude enables mode 2004 when it enters raw input. The bundle builds the
escape sequence from `BRACKETED_PASTE:2004` and `Qti=_K(ab.BRACKETED_PASTE)`
in `chunk-sr2xtv1m.js` (277167586). `_K` calls `Kl` with
``?${E}h``, producing `ESC[?2004h`. The raw-input setup writes
`c.set("bracketedPaste")` (278356546). The bundle builds open and close
markers as `Yca=Kl("200~")` and `Xca=Kl("201~")` (261899871).
`Ia` accumulates into `pasteBuffer` across reads in `IN_PASTE`, emits one
`{kind:"paste",text:O}` on the close marker, then returns to normal input
(278251285-278255170). `Rb` dispatches that event with
`dispatchPasteEvent(_.text)` (278362557).

For a simulator annotation, `send_annotation` saves the PNG and emits its
path and caption. In this checkout `AppWindow::paste_agent` is in
`src/window/canvas_view.rs:2128`. Its `format_quote` builds `@<path>\n<caption>`,
using a path relative to the agent's working directory when the file is under
it. The simulator caption is
`Annotated screenshot of <name> (<runtime>), <w>x<h> points. The red marks show what to look at.`
The formatter strips trailing CR/LF from the caption and adds `\n\n<trimmed message>`
if a nonempty optional message is present. It leaves the path unquoted.
`TerminalView::paste_quote` (`src/terminal.rs:750`) waits for an attached
channel with bracketed paste enabled. `Vt::paste_bytes` (`src/vt.rs:1504`)
sends `1b 5b 32 30 30 7e`, then the UTF-8 body, then `1b 5b 32 30 31 7e`.
It removes all embedded closing markers, including any formed by an earlier
removal. CR and LF inside the body pass through unchanged. `TerminalView::send`
passes the bytes to `Core::input` as a DATA payload (`src/hc.rs:522`) and
can mirror them to synced terminals. `paste_agent` then focuses the target.
The last byte of the annotation payload is `7e`. It sends no Enter. The
100 ms retry is for readiness, with a 30 s timeout.

**Confidence.** High for Canopy's bytes and Claude's parser. Actual PTY
read boundaries remain unmeasured.

## 3. Enter after paste

A CR after the closing marker becomes a separate `return` key even in the
same input chunk. See `Ia`, `Rb`, and the `Mr` mapping for `"\r"` at
278259566. The composer still has a say. In `aTt` (282780762-282783574),
`ye` intercepts `return` while the paste is pending and records it in
`C.current`. Ordinary text insertion calls `M()`, which schedules a
0 ms callback that clears the pending flag and replays the recorded return.
When the paste reads image files asynchronously, it instead finishes via
`Y()`, which clears the recorded return. An immediate CR can therefore be
lost on the image path while the draft and image are still being prepared.

The main prompt's `rH` (299829700-299831000) collapses ordinary text above
`_Z=800` JS UTF-16 code units, or with more than
`Math.max(0,Math.min(vr-10,2))` newline sequences. `vr` is terminal rows.
`vV` counts newline sequences and `Due` formats the
`[Pasted text #<id> +<N> lines]` placeholder (269364310).
The threshold changes draft display. It uses the same text completion and
return replay. Claude's own `replyChain` PTY fallback writes the framed paste,
then writes `"\r"` in a separate 10 ms timer (277912069). That is a sender
choice. The image read can outlast it.

**Confidence.** High for event separation and the pending-return branches.
Static code predicts no fixed delay for ordinary text. For a paste that
loads an image, wait until attachment processing is complete or leave Enter
to the person. A sufficient delay in milliseconds is unknown.

## 4. Image paths

- **Draft image attachment.** Paste an existing absolute path such as
  `/tmp/screen.png` or put it on its own LF-delimited line beside the caption.
  `aTt.z` checks each line for image-path candidates and calls `l_i`.
  Matching outer double or single quotes are removed, so
  `"/tmp/my screen.png"` and `'/tmp/my screen.png'` work. A single bare
  path can contain spaces. A space-separated group has extra path checks.
  `b5t=/\.(png|jpe?g|gif|webp)$/i`, `dQo`, `cQo` and `l_i`
  are at 277980000-277981650. `l_i` reads absolute paths and has a
  separate basename match against the real clipboard's file URL. The
  raw PTY paste supplies no clipboard file URL. Use a full path in this
  branch. Bare `~/screen.png` has no home expansion here. The handler
  verifies image bytes and calls `onImagePaste`. `Cs` stores content with
  `type:"image"` and inserts `Cse(A)` (282969300), whose exact format is
  `[Image #${e}]` (269364487).
- **File mention on submit.** `@/tmp/screen.png` follows the mention path.
  `z3r` and `aGn` parse unquoted tokens and the double-quoted form
  `@"/tmp/my screen.png"` (274494200-274495680). Unquoted spaces end the
  token. Single quotes after `@` are outside the quoted mention syntax.
  `O3r` uses `ht` to resolve the path, including `@~/screen.png`
  (261777759, 274475940-274477370). The ordinary submit path invokes
  `M3r` via `at_mentioned_files` (274435380). `Vyr` calls the file
  reader and carries its result in `type:"file"` (274497987). That
  reader has an image result and converts it to an image content block
  (274313047 and 274304908). The file-mention path is conditional on
  read permissions, file limits and mention policy. The draft keeps the
  mention text and the paste itself produces no `[Image #1]` for this form.

**Confidence.** High for the draft paste syntax and separate mention parser.
Medium for which eligible mentions become image blocks in a submitted turn,
given the alternative policy branches.

## 5. Other PTY details

- With mode 2004 off, the general `Vt::paste_bytes` fallback turns CRLF and
  lone LF into CR. `paste_quote` waits instead of using that fallback.
  A physical Enter normally encodes as CR, with mode-dependent alternatives
  such as `ESC[13u` in the key tests (`src/terminal.rs:4281`).
- Inside the frame, the main prompt's text handler normalizes CRLF and CR
  to LF and expands tabs to four spaces before insertion. `aTt` splits on
  LF for image detection before that text normalization. Use LF between
  paths and prose. Outside the frame, `Mr` names CR `return` and LF `enter`.
- `Jc` schedules a 2000 ms wake for an unfinished paste (`vy=2000`,
  278290149). On flush, `Ia` can emit a partial paste. This is an
  incomplete-frame timeout. Neither the 800-unit display threshold nor
  `omt=1e5` (the expand-hint cutoff) establishes a maximum paste size.
  No hard end-to-end size limit was established.
- Inside a complete paste, ordinary `0x03` (Ctrl+C) text goes into the
  paste buffer in raw mode. ESC still goes through the tokenizer `c4e`
  (261903934). `Ia` recognizes a second opening marker as a reset,
  closes at the ending marker, drops sequences matched by `Pp`
  (including focus events), and decodes some encoded key sequences via
  `ex`. A lone ESC before the close marker or an incomplete escape
  sequence needs a PTY check. Canopy's existing test
  `bracketed_paste_cannot_end_itself_early` covers the sender's closing
  marker removal.

**Confidence.** High for sender normalization, the reader timer and named
parser branches. Medium for how control bytes ultimately appear in the draft.

## Open questions for a live check

With the user's permission, use an isolated Claude Code session in a test PTY.
Capture stdout and the write boundaries.

1. Wait for `ESC[?2004h`. Send one framed two-line draft with no CR and check
   that it stays in the editor. Compare a same-write CR, a separate CR after
   10 ms, and a human Enter in clean drafts. Repeat below and above 800 units
   and around the newline threshold. Count submits and compare the text.
2. Use a readable test PNG. Paste an absolute path on its own line with a
   caption and repeat the timing comparison. Use a larger PNG to exercise
   asynchronous loading. Record when the image pill appears and whether
   each Enter submits once.
3. Compare bare, single-quoted, double-quoted, `@/tmp/screen.png`,
   `@"/tmp/my screen.png"`, and both bare and mention forms with `~/`.
   Check the draft first, then inspect the submitted turn for a real image
   block or a file-reference message. Use an approved test account for
   submissions.
4. On fresh drafts, split the markers across writes, omit the close marker
   for over 2 s, and send texts containing LF, CRLF, literal ESC, and
   `0x03`. Compare draft contents and interrupts. Increase paste size
   without submitting and record the first truncation or failure.
