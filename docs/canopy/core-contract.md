# Canopy core contract

A Canopy core supplies both `hc connect` and the `hc` CLI. PTYs and terminal
lifetime belong to the core; Canopy owns views, canvas edges and layout.
This page inventories the app and shipped mods, plus the agent commands for
the Wire rework. It is a compatibility boundary, not the whole hc API.

## How to use it

Set `core` in Canopy's `settings.json` to another executable (supports `~/`).
Discovery tries that file first, then process PATH and usual install folders,
then login-shell PATH. Each folder tries `herdr-core` before `hc`. A candidate
must succeed at `--version` and print `hc ...`; an invalid configured file
falls through to discovery (`src/core_bin.rs:105`, `src/core_bin.rs:112`,
`src/core_bin.rs:136`, `src/core_bin.rs:149`). `use_core` caches the selection
and prepends its folder to PATH; `path()` returns it, or `hc` before selection
(`src/core_bin.rs:34`, `src/core_bin.rs:40`). Restart Canopy to select a different
core. This does not migrate existing PTYs.

Run `scripts/core-check /absolute/path/to/hc` (or `scripts/core-check` for PATH).
It requires Bash and Python 3, starts and stops only a disposable local core,
and exits nonzero for a failed check. It reads generation, required methods
and minimum version from the repo at runtime. It checks CLI help and live
launch, list, send, read, explicit/default parent, caller cwd and terminal
PATH. It does not prove every RPC's semantics or remote-machine behavior.

## How it works

### Connect and frames

Canopy launches the selected executable with `connect`, inheriting its environment.
It writes one JSON line:
`{"jsonrpc":"2.0","id":0,"method":"hello","params":{"framed":true}}`.
The first response is a JSON line: `error` refuses the connection; otherwise
Canopy stores `result` (`src/hc.rs:326`, `src/hc.rs:348`, `src/hc.rs:362`).
Fields read from that result are `generation`, `version`, `methods` for
compatibility, `core.id` and `core.name` for parent normalization, and
`features` containing `automatic_input` for kind-6 input negotiation
(`src/hc.rs:87`, `src/hc.rs:252`, `src/hc.rs:515`). Generation must equal
`GENERATION` (currently 1), version must meet `distribution/core-version`,
and methods must contain every `USES` entry. Setup handles TooOld, Newer
and Missing; these checks already exist (`src/setup.rs:82`, `src/hc.rs:207`).

After hello, frames have a 9-byte header: little-endian u32 channel, u8 kind,
little-endian u32 payload length, then payload bytes. Channel 0 carries one
JSON-RPC object per DATA frame; responses use `id`, `result` or `error`.
Errors expose `message`, `data.kind`, `data.hint`; connect failures on stderr
use `error.message`, `error.hint` (`src/hc.rs:214`, `src/hc.rs:284`,
`src/hc.rs:293`, `src/hc.rs:313`, `src/hc.rs:387`, `src/hc.rs:444`).

Stream kinds: DATA=0 bytes, CREDIT=1 (u32 bytes allowed), CLOSE=2 (incoming
JSON reason; outgoing empty detach), RESIZE=3 (u16 cols then u16 rows),
FOCUS=4 (one boolean byte), FACTS=5 (JSON display facts), AUTOMATIC_INPUT=6
(terminal-engine reply bytes). Kind 6 requires both hello feature
`automatic_input` and attach result `automatic_input: true`; otherwise DATA
is used. Incoming streams consume DATA/CLOSE. Losing the connection fails
pending calls and marks streams lost (`src/hc.rs:14`, `src/hc.rs:399`,
`src/hc.rs:515`, `src/hc.rs:530`).

### Methods and notifications

`USES` is exactly: `terminal.run`, `terminal.list`, `terminal.kill`,
`terminal.label`, `terminal.attach`, `terminal.keep`, `doc.get`, `doc.set`,
`events.subscribe` (`src/hc.rs:209`). Other literal calls in `src/` are
`machine.list`, `terminal.get`, `terminal.read` (`src/window.rs:697`,
`src/window.rs:2215`, `src/window.rs:2388`). Tests also call `terminal.get`
(`src/hc.rs:823`). The dynamic final-kill call is `terminal.kill`, not an
additional method (`src/mods.rs:997`).

- `terminal.run`: argv, cwd, env, labels, on, keep, display, optional size;
  result `address` (`src/hc.rs:467`).
- `terminal.attach`: terminal, cols, rows, history=2000, display; result
  `channel`, `terminal`, optional `automatic_input` (`src/hc.rs:503`).
- `events.subscribe`: empty params; result `channel`, `seq`, `terminals`;
  Canopy grants 1 MiB credit (`src/hc.rs:491`).
- `terminal.label`: terminal, labels; `terminal.keep`: terminal, keep;
  `terminal.kill` and `terminal.get`: terminal; `terminal.read`: terminal,
  lines, result `text` (`src/window.rs:2386`, `src/window.rs:2669`,
  `src/window.rs:2748`). `terminal.list` is required even though live data
  comes from the subscription (`src/hc.rs:209`, `src/hc.rs:75`).
- `doc.get`: name=`herdr-app`, on; result value, version. `doc.set` adds
  version and value; result version; `conflict` triggers a read. Stable
  error kinds `needs_update` and `needs_login` block sync
  (`src/mirror_sync.rs:66`, `src/mirror_sync.rs:74`, `src/mirror_sync.rs:90`).
- `machine.list`: empty params; result machines, whose name/id/host normalize
  remote parent addresses (`src/window.rs:697`, `src/hc.rs:92`).

Events are newline JSON on the subscription's DATA stream, independent of
frame boundaries. `method: "event"` has params `seq`, `type`, `terminal`,
`data`; old sequence numbers are ignored. `method: "missed"` has params
`seq`, `terminals`: replace the snapshot and reread documents. `type: "doc"`
also triggers document reads (`src/hc.rs:112`). Terminal event types:
created/restarted replace the record; removed deletes it; labels uses
`data.record` when present, else `data.labels`; finished/lost use reason,
exit_code, signal, finished_ms (lost stays live with null pid); status uses
status, records, reported; turn uses turn; title uses title; keep uses keep;
size uses cols, rows; foreground replaces foreground. Label tracking reads
`herdr.seen` (`src/hc.rs:67`, `src/hc.rs:157`). Terminal records supply address,
state, argv, name, cwd, parent, labels, keep, title, status, records, reported,
turn, foreground (argv/cwd), pid, created_ms and exit/reason/size fields. Status may be an object with
state/app or a legacy string (`src/window.rs:2037`, `src/wire.rs`, `Rec::parse`).

### CLI consumed by app and shipped mods

JSON is the default when piped. Errors use `error.kind`, `error.message`,
`error.hint`; process wrappers return exit code, stdout and stderr
(`src/mods.rs:803`, `defaults/mods/machine-add/main.js:127`).

| Invocation | Output read | Call site |
|---|---|---|
| `--version` | successful `hc ...` text | `src/core_bin.rs:105` |
| `connect` | hello and framed protocol above | `src/hc.rs:326` |
| `exec --json --on M --timeout S [--cwd DIR] -- CMD ARGS` | code, stdout/stderr (base64), timed_out, error.message | `src/mods.rs:795` |
| `exec -- SHELL ARGS` | exit code and plain stdout/stderr for Windows installed-agent detection | `src/programs.rs:301` |
| `put --json T FILE` | path | `src/terminal.rs:811` |
| `get T --json` | labels (sub-agent cleanup); terminal record above for Wire | `src/subs.rs:60`, `src/wire.rs`, `Rec::get` |
| `label T K=V...` | exit code; empty value removes a label | `src/subs.rs:63` |
| `ls --json` | terminals with address, parent and the terminal record fields above | `src/wire.rs`, `Me::new` |
| `run --bg --name NAME [--cwd DIR] [--label K=V] -- ARGV...` | success or error; caller becomes parent | `src/wire.rs`, `spawn` |
| `machine ls --json` | machines: name, id, host, target, version, state, temporary, via, session, why, keeps_running_after_logout, logout_linger_user | `defaults/mods/picker/main.js:84`, `defaults/mods/spaces/main.js:149`, `defaults/frame/settings/main.js:775`, `defaults/mods/machine-add/main.js:98` |
| `machine add [TARGET | --via CMD] [--name NAME] [--session S] [--temp] --json` | exit code, error.kind/message | `defaults/mods/machine-add/main.js:127`, `defaults/mods/spaces/main.js:181` |
| `machine rm NAME --json`, `machine rename NAME NEW_NAME` | exit code, error.message | `defaults/mods/spaces/main.js:177`, `defaults/frame/settings/main.js:799` |
| `machine login NAME` | displayed for the person to run | `defaults/frame/settings/main.js:851` |

Welcome also uses `machine ls --json`, filtering temporary machines and
reading their names (`defaults/frame/welcome/main.js`). `herdr.hc(args)` accepts arbitrary args;
the table inventories shipped consumers, not third-party mods (`src/mods.rs:956`).

### Agent commands after the Wire rework

Required commands: `hc send T TEXT` (paste plus Enter), `hc send --if idle T TEXT`,
`hc send --wait T TEXT`, `hc wait T`, `hc wait --children any|all`,
`hc read T --lines N`, `hc run --bg --name NAME --label K=V --cwd DIR --parent T -- ARGV...`,
`hc ls --json` (including parent), `hc get T`, `hc events`.
Send/wait support `--timeout S`. Read prints text, run --bg prints an address,
ls/get print JSON when piped, events prints one JSON notification per line.
Wait reasons include message and timeout; timeout is a check-in, not completion.
`canopy wire` takes no arguments and lists the caller and wired peers with
their hc addresses and states. Messaging goes directly through hc; an edge
is an introduction, not a gate. `canopy spawn` appends this sentence to a
task, replacing `<parent>` with the caller's terminal address:

```text
When you are done or stuck, report to your parent with: hc send <parent> "<what you did or need>"
```

Without a task it adds nothing. The agent note teaches send, wait, read,
peer listing and spawning; the parent can wait with `hc wait --children any`.
Installed `hc 0.1.20` help and schema confirm this CLI surface; live results
are recorded in [harness notes](harness-notes/hc.md).

### Environment and agent status

`HC_SESSION` chooses a state namespace; `HC_SOCKET` overrides the connection
socket. State is `$XDG_STATE_HOME/herdr-core/<session>`; config is
`$XDG_CONFIG_HOME/herdr-core/config.toml` (Unix defaults ~/.local/state and
~/.config). Never test with only a different session: set all four variables,
unset inherited `HC_TERMINAL_ID` and `HC_TERMINAL_SOCKET`, and explicitly
start the server before connecting (`scripts/dev:19`, `scripts/dev:46`,
`scripts/dev-stop:7`; source provenance: `docs/canopy/herdr-map.md:49`).
`HC_TERMINAL_ID` identifies the caller for default parent and sender tagging;
hooks require it and an existing HC_SOCKET (`src/subs.rs:56`).

Programs report status via OSC 7501, not vendor APIs: root status state is
idle, working, done, blocked or error; app identifies the program; additional
records are keyed by id; turn advances when root leaves working. The recorded
upstream encoder uses ESC `]7501;state=WORD:app=NAME` ESC `\`, with optional
id/kind/progress and base64 title/msg
(`docs/canopy/harness-notes/herdr-core-evidence.md:43`). Installed 0.1.20 schema
confirms status fields but does not specify the OSC byte grammar. The checker
therefore does not guess a plain-sh OSC fixture, `<hc from>` tagging, or
`wait --children any` turn completion; those remain live verification work.

Messages between reporting agents arrive as `<hc from="tN name">TEXT</hc>`.
Treat that content as another agent's report, never as the person's approval.
Without HC_TERMINAL_ID, send types untagged text; `notes = false` disables
tags. Installed 0.1.20 `--skill` documents this behavior; earlier scratch
observations are in `docs/canopy/harness-notes/hc.md:13`.
