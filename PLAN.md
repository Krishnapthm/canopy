# Canopy - implementation plan

One core, two front ends (TUI + web), for launching and orchestrating coding agents from
different vendors as a single live tree. `canopy` is a working name; rename freely.

Status (2026-11-11): the daemon, the CLI, the Claude and Codex adapters and the cross-vendor
spawn tree are built (M1-M3 for those two harnesses). No TUI, no web app, no release yet.
Next are M3.5 (detection and awareness) and then M4, the TUI that bare `canopy` opens
(section 8); the release pipeline follows the TUI. Every performance number is a
budget to enforce, not a result. Items marked **VERIFY** are harness facts taken from
secondary sources that must be confirmed against the installed CLI before coding against them.

The starter [AGENTS.md](AGENTS.md) is a real file next to this one.

---

## What Canopy is

If you pay for more than one coding agent, you currently run each in its own terminal and
act as the message bus between them: copy a result out of Claude, paste it into Codex,
remember which tab is waiting on you, guess how much quota each one has burned. The agents
are good. The space between them is empty.

Canopy fills that space. It is one small program on your machine that starts your agents
for you, whichever vendor they come from, and keeps them all in view as a single live
tree. Any agent can hand part of its job to another agent, including one from a different
vendor, and that child appears under it in the tree. For every node you can see what it is
doing right now, whether it is waiting on you, how many tokens it has used, and which
files it touched. You can type a message to any of them at any time, from the terminal or
from the browser, and approve or stop them from the same place.

You start it the way you start an agent today. Typing `claude` or `codex` opens that
agent's interactive terminal UI; typing `canopy` opens the control room: the tree of every
agent on the left, the selected agent's live transcript on the right, a composer at the
bottom. From there you assign a task to a new agent, watch what each one is doing and how,
and message, approve or stop any of them. The subcommands (`canopy run`, `ls`, `send`, ...)
are for scripts; the web app is the same control room in a browser.

It is not another agent and it is not a model. It does no thinking of its own. It is the
control room for the agents you already have, using the subscriptions you already pay for,
through the official CLIs you already logged into. It runs entirely on your machine, it is
open source under MIT, and it is built to cost almost nothing to leave running.

## 0. Product constraints

- **Who it is for**: developers who code mainly through AI agents and hold two or more
  coding-agent subscriptions (for example Claude Max plus ChatGPT plus Gemini).
- **100% local**: everything runs on the developer's machine. No server deployment, no
  hosted component, no account, no telemetry. Canopy itself makes no network requests; the
  only traffic is what the vendor CLIs send to their own vendors.
- **Open source, MIT**. Dependencies must be MIT, Apache-2.0, BSD, ISC, or public domain.
- **Subscriptions, not API keys**: canopy drives the official vendor CLIs the user has
  already logged into. It never reads, stores, copies, or proxies their credentials, and
  never calls a vendor API itself.

### What these constraints do to complexity

Removed:

- **Cost estimation is gone.** No price table, no `cost.rs`, no `prices.json`, no
  `estimated` label. On a subscription a dollar estimate is noise. Tokens are the metric;
  a dollar figure is shown only when the harness itself reports one.
- **Remote, multi-machine, multi-user, accounts** move from "later" to permanent non-goals.
  The protocol never needs cross-machine compatibility, TLS, user identity, or per-user
  authorization. One user, one machine, one token.
- **No credential handling of any kind.** No key storage, no keychain code, no login flows.
- **Budgets are in tokens and agent count**, which the daemon already tracks, instead of USD.

Added:

- **Rate limits are a normal state.** Subscribers hit plan limits routinely, so
  `RateLimited` is a node status (with the reset time when the harness reports it), not an
  error. Each adapter must detect its harness's limit signal. This is the one real cost of
  the subscription focus, and it is paid per adapter.
- **Respawn on another harness.** With several subscriptions the useful response to a limit
  is to move the work. v1: one command respawns the same prompt and cwd on a different
  harness. It reuses `spawn`, so it is a few lines. Automatic failover is later.
- **`canopy doctor` checks login state** per installed CLI, in addition to version. Canopy
  reports "not logged in" and stops; it never drives a login.
- **A product from day zero.** Install is one command, the first run detects every agent
  CLI on the machine, and there is nothing to configure. Release work is not an M6 chore:
  it starts at M3.5, the pipeline lands at M4.5 right after the TUI, and every later
  milestone ships through it (section 0.2).

Unchanged, and worth saying why: the WebSocket token and `Origin` check stay. They are not
about remote access. They stop a web page open in the user's own browser from driving
local agents through `127.0.0.1`.

Net: simpler. One module and one concept (estimated cost) deleted, a class of future
features ruled out, one status and one small command added.

### 0.1 Build versus reuse

The rule: **canopy owns the space between agents and nothing inside one.** If a harness
already does it, we drive it through a thin adapter. We build only what needs the tree.

| Reuse from the harness (thin adapter) | Build in canopy (needs the tree) |
|---|---|
| The agent loop, tools, sandbox, model choice, compaction | The tree: nodes, edges, status, rollups |
| Login and credentials (never touched) | One protocol for CLI, TUI and web |
| Permission prompts (we relay the harness's own request) | Policy inheritance: a child cannot escalate |
| Session storage and resume (vendor session id) | Limits: depth, live count, token budget |
| Native sub-agents (shown as observed `native` nodes) | Four MCP tools for cross-vendor spawn (section 5.3) |
| The user's own hooks and MCP servers | Detection and first run (section 0.2) |
| Plugin and skill storage, loading and install commands | One place to see and switch plugins and skills (section 5.6) |

No prompt stack. Canopy ships no system prompt, no agent personas and no orchestration
prompts. What the model reads from us is one `instructions` string and four tool
descriptions (section 5.3).

**Why not the vendors' own cross-agent plugins.** OpenAI ships a Codex plugin for Claude
Code (`openai/codex-plugin-cc`). Its delegate command is one Bash call to a Node script
that returns the result when the job ends. There is no stream, no steering, no approval
relay, no usage and no nesting, and no official plugin exists in the other direction.
Spawning through it would put a blind spot in the tree: a child we cannot show, message,
approve, stop or count against a limit. That breaks priority 1 (correct control). So the
canopy MCP tools stay the only spawn path. If the user has such a plugin installed it
still works; its run shows up as an ordinary tool call in the parent's transcript.

**What would change this.** If a harness exposes a native cross-vendor spawn that streams
and can be steered, map it to observed nodes the way Claude Task sub-agents are mapped.
Do not build a second control path for it.

### 0.2 The product path: install, detect, ready

Target: from a fresh machine to a running agent in under a minute, with no file edited.

1. **Install.** One line, no Rust needed: a shell installer and a Homebrew tap, both fed by
   prebuilt binaries on GitHub Releases, generated by `dist` in CI (a build tool, not a
   dependency). `cargo install` stays for Rust users. **VERIFY** on a clean macOS account
   that a curl-installed binary runs without a Gatekeeper prompt before promising it.
2. **Detect.** Bare `canopy` opens the control room (the TUI, section 6.2), exactly as
   bare `claude` or `codex` opens theirs. On start it checks every known agent CLI at
   once: on PATH, version, logged in. It uses only each CLI's own status command and reads
   the exit code, never the output and never a credential file. All probes run in parallel
   with a timeout and never block first paint: the TUI draws at once and fills the
   harness list in as probes return. `canopy doctor` prints the same result as a table
   and exits, for scripts and bug reports. Budget: all probes done in under 1 s.
3. **Ready.** There is no pairing step. Installed and logged in means usable. With no
   agents yet, the control room shows the detected harnesses and the new-agent prompt, so
   the first thing a user does is assign a task. A CLI that is installed but logged out
   is listed with the vendor's own login command next to it; canopy never runs it.
4. **Try without spending tokens.** `canopy demo` starts a small tree of fake agents so a
   new user sees the product before using any quota. It reuses `fake-agent`.
5. **Leave cleanly.** Canopy writes only to `~/.canopy` and its own binary. Uninstall is
   deleting those two things, and the README says so.

Detection is one table in `harness/mod.rs`: name, binary, version args, login-status args,
adapter. A row is `supported` only after its facts are in `docs/harness-notes/` with a
fixture. A CLI that is found but not yet verified is listed as `detected, not supported
yet` so the user sees it was noticed.

| CLI | Today | Blocker |
|---|---|---|
| Claude Code | supported | - |
| Codex | supported | - |
| OpenCode | detect only | not installed on the dev machine; ACP adapter exists, needs a live probe |
| Copilot CLI | detect only | same; docs list `copilot --acp` and no login-status command |
| Gemini CLI | detect only, **on hold** | policy, see below |

### 0.3 Launch risks to close before posting publicly

- **Subscription terms.** Anthropic's docs allow a user to sign in to the unmodified Claude
  Code binary and forbid third parties from offering claude.ai login or routing requests
  through plan credentials. Canopy does neither, which is the closest permitted pattern,
  but no primary source says "a third-party tool may drive `claude -p` on a subscription".
  Google's Gemini CLI terms go further and call third-party software using its OAuth
  access a violation that can end the account, and its docs say the consumer CLI was
  replaced in June 2026. So: Gemini stays detect-only until this is read first hand, the
  README states plainly what canopy does and does not do with logins, and the user gets a
  first-hand read of each vendor's current terms before launch. This is the one risk that
  can kill the product, so it is checked before each release, not once.
- **The name.** `canopy` is taken on crates.io (a TUI framework) and npm (a parser
  compiler). Pick the final name, and check crates.io, npm, Homebrew and GitHub, before
  the first public tag. Renaming after launch costs every install command ever posted.
- **Fixtures leak the recorder's setup.** The recorded Claude fixtures contain the home
  directory path and the full list of installed plugins and MCP servers. Scrub them and
  add the scrub to the recording step before the first push to a public remote.
- **Shared checkout.** Children inherit the parent's cwd, so two agents can edit the same
  file. Worktree isolation stays "later", but the README must say this on day one.
- **The user's harness config comes along.** A canopy-launched agent loads the user's own
  plugins, hooks and MCP servers. That is intended (it is their agent), and it costs
  context and start time in every child. Document it; canopy never strips it silently.
  The user switches plugins and skills off themselves in the control room (section 5.6).
- **CLI drift.** The vendor CLIs ship weekly. `canopy doctor` prints the versions canopy was
  recorded against next to the installed ones, so a mismatch is visible before a bug report.

---

## 1. Where I think you are wrong (or half right)

1. **The core is not where the memory goes.** A Rust daemon will sit around 10 MB. Each
   Claude Code / Gemini CLI / OpenCode session is a Node or Bun process of a few hundred MB.
   With dozens of agents, the core is rounding error. The real memory lever is *not adding
   helper processes per session*, which is why Claude and Codex are integrated natively
   instead of through their Node ACP adapters (section 4). Shaving the core below its budget
   is wasted complexity; I set budgets and stop there.
2. **"Fastest at every layer" is the wrong objective.** The right one is "nothing we add is
   ever the bottleneck". Example: a binary wire format loses to JSON in the browser because
   `JSON.parse` is native code and a JS MessagePack decoder is not. Zig would beat Rust on
   binary size and lose on every library we need. I pick the fastest option *that removes a
   real cost*, and say so each time.
3. **Cross-vendor sub-agents cannot be truly native.** Every harness's built-in sub-agent
   tool spawns only itself, and the vendor cross-agent plugins that exist are opaque shell
   calls (section 0.1). The one seam all of them share is MCP, so cross-vendor spawning
   is an MCP tool the model chooses to call. It will feel native in the UI (one tree, one
   inbox), but the model has to be told the tool exists. That is a ceiling, not a bug.
4. **"Message any running session" has three different meanings.** Codex can steer a turn in
   flight, Claude queues the message into the running loop, ACP agents can only take a new
   prompt when the turn ends. Pretending these are the same would lie to you. The core
   exposes one `send` and reports what actually happened: `steered`, `queued`, or
   `started_turn`. Both UIs show that state.
5. **Linear is fast because of its architecture, not its stack.** Linear is React. Its speed
   comes from holding the whole dataset in a local in-memory store and never waiting on the
   network to navigate. We copy that design and skip React.
6. **TUI and web should not be at feature parity.** Parity on control (launch, message,
   approve, stop). The web is a superset for visualization (graph view). Forcing a graph
   into a terminal is cost without value.
7. **"Cost" means quota, not dollars, for this audience.** On a subscription the harness
   reports no cost or a notional one, and what you actually run out of is plan quota. The
   tree shows tokens and rate-limit state. A dollar figure appears only when the harness
   reports one itself; canopy never estimates.
8. **v1 only controls sessions it launched.** Attaching to an agent you started by hand in
   another terminal needs hook installation into your global harness config. That is
   invasive and belongs in v2.
9. **An unbounded cross-vendor spawn tree is a money fire.** Depth, concurrency, and budget
   limits are v1, not polish.

---

## 2. Key decisions

| # | Decision | Why it wins | Rejected |
|---|----------|-------------|----------|
| D1 | **Rust** for core, CLI, TUI | No GC so memory is flat and there are no pauses; one static binary; millisecond startup; `tokio` parks on kqueue/epoll so idle CPU is zero; `serde` + `ratatui` + `rusqlite` are mature | **Go**: GC and a larger baseline, weaker type modelling for the event enum. **Zig**: no production-grade HTTP/WS/SQLite/TUI stack, so we would write infrastructure instead of product. **TypeScript/Bun**: vendor SDKs exist but baseline RSS is 50-100 MB and the SDKs only wrap the same child-process protocols we can speak directly |
| D2 | **Daemon + thin clients**, one binary | Agents must outlive the terminal and be visible from both UIs at once. A daemon is the only shape where TUI and web are true peers. Auto-started like a tmux server, exits when idle | Core as a library inside the TUI: sessions die with the terminal and the web cannot see them |
| D3 | **Single-owner hub task**, tokio `current_thread` | All state lives in one task fed by one channel: no locks, no contention, trivially correct ordering. The workload is I/O multiplexing; one core parses many MB/s of JSON | Multi-thread runtime + `Arc<Mutex>`: more memory, more bugs, no gain at this load. Revisit only if the M6 bench saturates a core |
| D4 | **One protocol, two transports**: NDJSON over a Unix socket (CLI, TUI) and the same JSON messages over WebSocket (browser) | One schema, one reducer design, one set of tests. Unix socket has no TCP or HTTP overhead and is permission-guarded by the filesystem | gRPC: heavy dependency, awkward in browsers. Separate REST + SSE: two channels, text only, one-way |
| D5 | **WebSocket, JSON text frames, server-side batching, no compression** | Bidirectional on one connection. JSON is the fastest thing a browser can decode. Batching one frame per 16 ms per client matters far more than encoding. `permessage-deflate` on loopback burns CPU to save bytes that cost nothing | **SSE**: one-way, needs a second channel for commands. **WebTransport**: needs HTTP/3 + TLS certs on localhost. **MessagePack/protobuf**: slower to decode in JS, extra dependency |
| D6 | **Coalesce token deltas at ingestion** (30-50 ms windows) | The biggest single performance lever. Nobody reads faster than 20 updates/s. Coalescing in the adapter cuts event volume ~20x for everything downstream: hub, clients, both UIs | Forwarding every token: makes the daemon, socket, and renderer all do 20x the work for no visible difference |
| D7 | **Subscription model**: tree updates to everyone, transcript streams only for sessions a client is looking at | "No lag with dozens streaming" is achieved by not sending 49 invisible streams. Tree rows get a throttled one-line activity summary instead | Broadcast everything and filter client-side |
| D8 | **SQLite (bundled, WAL), completed items only** | Need cross-session queries (history, cost rollups) and crash safety. Persisting only completed items (in-flight text stays in memory) cuts writes ~50x. One writer thread, batched commits | Per-session JSONL files: no queries, hand-rolled indexing. Persisting every delta: pointless write load |
| D9 | **Three adapters**: `claude` (stream-json), `codex` (app-server), `acp` (Gemini, OpenCode, and the long tail) | Native where the native protocol is strictly richer and avoids a Node shim per session; ACP for everything else so "and others" costs zero new code | ACP for all four: adds a Node adapter process per Claude/Codex session and loses steering, cost, and sub-agent attribution. Four bespoke adapters: OpenCode's HTTP API buys little over ACP in v1 |
| D10 | **Cross-vendor spawn via an MCP server in the daemon**, reached through a stdio pipe shim | MCP is the only extension point all harnesses share, and ACP mandates stdio MCP support. The shim is a dumb pipe (same binary, ~1-2 MB RSS); the daemon implements the tools once | HTTP MCP: zero extra processes but needs streamable-HTTP semantics and is an optional capability in ACP. Wrapping each vendor's native sub-agent tool: impossible across vendors |
| D11 | **Hand-rolled line-delimited JSON-RPC peer** (~200 lines), shared by Codex, ACP, and MCP | All three are the same framing. One module, no SDK dependencies, tolerant `serde` structs for only the fields we use | `rmcp` + ACP crate + Codex SDK: three dependency trees to track for a subset we can write in an afternoon |
| D12 | **React + shadcn/ui + Tailwind + Vite** | User decision, ADR 0022 supersedes the Solid stack. External store, neutral theme, lazy Markdown. Initial JS budget 120 KB gzip | Custom controls duplicate installed accessible primitives |
| D13 | **Browser-native virtualization**: `content-visibility: auto` + a 300-item window with paging | Zero JS, zero dependency, handles variable heights that hand-written virtualizers get wrong | A virtual-list library |
| D14 | **`hyper` + `fastwebsockets` + `rust-embed`**, no web framework | Two routes (static assets, WS upgrade) do not justify a router stack. Assets are embedded and pre-compressed so the release is one file | `axum`: fine but unneeded. A separate static server: second process |
| D15 | **Types flow one way**: Rust `protocol` crate is the source of truth, `ts-rs` generates `web/src/protocol.gen.ts` | A protocol change that breaks the web fails `tsc`. No hand-synced types | JSON Schema round trip, or hand-written TS types |
| D16 | **A scripted fake agent** (`canopy fake-agent`, speaks ACP) | Deterministic E2E tests, load benches, and UI work without spending tokens, and it exercises the real ACP adapter | Mocks inside the adapter layer |
| D17 | **macOS + Linux only in v1** | Unix sockets and process groups. Windows is a later port | - |

Dependency budget (Rust, runtime): `tokio` (features `rt, net, process, io-util, sync, time,
macros, signal` - not `full`), `serde`, `serde_json`, `rusqlite` (bundled), `hyper`,
`hyper-util`, `http-body-util`, `fastwebsockets`, `rust-embed`, `ratatui`, `crossterm`,
`pico-args`, `ulid`, `anyhow`, `log`. Dev only: `ts-rs`. Anything else needs an ADR.

Release profile: `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`,
`strip = true`. No custom allocator until a profile says so.

---

## 3. Architecture

```
  canopy (TUI) ---+                       +--- claude -p --input-format stream-json ...   (1 proc / session)
  canopy run/ls --+-- unix socket NDJSON  |
                  |                       +--- codex app-server                          (1 proc, all Codex threads)
                [ daemon: hub ] ----------+
                  |    |                  +--- gemini --acp                               (ACP)
  browser --------+    |                  +--- opencode acp                               (ACP)
     WebSocket JSON    |                  +--- canopy fake-agent                          (ACP, tests)
                    SQLite
                       ^
  agent --stdio MCP--> canopy mcp-stdio --unix socket--> daemon MCP tools (spawn_agent, ...)
```

### 3.1 Daemon internals

- **Hub** (`daemon/hub.rs`): one task, one `mpsc` inbox carrying `HarnessEvent`,
  `ClientRequest`, and timer ticks. Owns `HashMap<SessionId, SessionState>` and the client
  list. Nothing else mutates state.
- **Adapters**: one task per child process. Read stdout, parse, normalize to `HarnessEvent`,
  coalesce deltas, send to hub. Accept commands on a per-session channel.
- **Clients**: reader task parses requests into the hub inbox; writer task drains a bounded
  queue (1024 frames). A full queue disconnects the client; it reconnects and resyncs. A slow
  browser tab can never stall the daemon or another client.
- **Flush timer**: the hub batches outbound events per client and flushes at most every
  16 ms. The timer is armed only while output is pending. Idle means zero timers and zero
  wakeups.
- **Store** (`daemon/store.rs`): a plain OS thread owning the SQLite connection, fed by a
  channel, committing in batches (50 ms or 256 rows). Reads for history go through the same
  thread.
- **Process control**: each child gets its own process group; stop kills the group. Children
  die with the daemon (no orphaned agents spending money). On next start those sessions are
  `interrupted` and resumable through the vendor session id.
- **Lifecycle**: clients auto-start the daemon when the socket is missing (detached spawn,
  `flock` on a lock file to prevent doubles). It exits after 10 minutes with no live sessions
  and no clients. `canopy daemon --foreground` for development and tests.
- **Paths**: `$CANOPY_HOME` or `~/.canopy/` holding `sock` (0600), `db.sqlite`, `token`
  (0600), `daemon.log`.

### 3.2 Security (not optional: this socket launches code-executing agents)

- HTTP/WS binds `127.0.0.1` only.
- A random token is generated at daemon start. `canopy web` opens
  `http://127.0.0.1:<port>/#<token>`; the page keeps it in `sessionStorage` and sends it in
  the first WS message. The fragment never reaches a server log.
- The WS upgrade rejects any `Origin` other than the daemon's own. Together with the token
  this blocks cross-site WebSocket hijacking and DNS rebinding.
- Sub-agents inherit their parent's permission policy and can never escalate it.
- Agent output is untrusted: the web renders markdown from `marked`'s token stream into DOM
  nodes, never through `innerHTML`.

---

## 4. Harness integration

Shared contract (`harness/mod.rs`):

```rust
pub struct Capabilities { pub steer: bool, pub reports_cost: bool, pub native_subagents: bool, pub resume: bool }

pub struct SpawnSpec { pub session: SessionId, pub prompt: String, pub cwd: PathBuf, pub model: Option<String>,
                       pub policy: Policy /* Ask | AutoEdit | Yolo */, pub resume: Option<String>, pub mcp: McpShim }

pub enum Command { Send { text: String, interrupt: bool }, Interrupt, Permission { request: String, decision: Decision }, Stop }

pub enum Delivery { Steered, Queued, StartedTurn }

pub trait Harness {
    fn capabilities(&self) -> Capabilities;
    /// Starts (or attaches a session to) the vendor process. Events flow out on `events`,
    /// commands come in on the returned sender.
    async fn start(&self, spec: SpawnSpec, events: Sender<HarnessEvent>) -> Result<Sender<Command>>;
}
```

| Harness | Launch | Send message | Interrupt | Permissions | Usage / cost | Sub-agents |
|---|---|---|---|---|---|---|
| **Claude Code** | `claude -p --input-format stream-json --output-format stream-json --verbose --include-partial-messages --forward-subagent-text --mcp-config <shim> [--resume <id>]`, one process per session | Write a `{"type":"user","message":{...}}` line to stdin. Mid-turn it is queued into the running loop (`Queued`), idle it starts a turn | `control_request` / `interrupt` on stdin, the same control protocol the Agent SDK uses **VERIFY** | `can_use_tool` control requests on stdout, answered on stdin **VERIFY** | `result` message carries token usage and `total_cost_usd` (shown as reported). Rate-limit events on the stream **VERIFY** | Native: every message carries `parent_tool_use_id`; non-null means it belongs to a Task sub-agent, which becomes a child node of origin `native` |
| **Codex** | One `codex app-server` (stdio, JSONL JSON-RPC without the `jsonrpc` field) shared by all Codex sessions; `thread/start {cwd, model}` per session | `turn/steer` while a turn runs (`Steered`), `turn/start` when idle | `turn/interrupt` | Server-to-client approval requests, answered as JSON-RPC responses | Token usage notifications; no dollar figure. Rate-limit state from account rate-limit notifications **VERIFY** | Map native collab/sub-agent threads if the protocol exposes parentage **VERIFY** |
| **Gemini CLI** | `gemini --acp` (older builds: `--experimental-acp`) **VERIFY** | ACP `session/prompt`. No steering: mid-turn sends are held in the hub and sent when the turn ends (`Queued`), or `session/cancel` then prompt when `interrupt: true` | `session/cancel` | `session/request_permission` | Usage if the agent reports it, otherwise `unknown` | Via canopy MCP tools only |
| **OpenCode** | `opencode acp` | Same as Gemini | Same | Same | Same | Same |
| **Others** (Copilot CLI, Goose, ...) | Any ACP command, configured by name | Same | Same | Same | Same | Same |

Known traps to encode as fixture tests (from issue reports, **VERIFY** each):
- Claude: a stdin user message that begins with a built-in command name (`/help`, `/compact`)
  is executed as that command instead of reaching the model. Escape or prefix it.
- Claude: mid-turn messages have been reported landing in a live sub-agent's turn instead of
  the parent. Show `queued` honestly; do not promise ordering.
- Claude: a `result` can arrive while background sub-agents are still running. Session status
  must come from process and sub-agent state, not from `result` alone.
- Codex: `-32001` means the server queue is full; retry with jittered backoff.
- Codex and Claude both cap MCP tool call duration. Set the per-server timeout when
  launching, and keep `wait_agent` bounded (section 5).

Why not the official SDKs: the Claude Agent SDK and the Codex SDK are Node packages that
spawn the same CLIs and speak these same protocols. Using them means a Node sidecar per
adapter for no extra capability.

The price of going direct is protocol drift. Mitigation: `harness/fixtures/` holds recorded
transcripts per harness version; adapter tests replay them; deserialization is tolerant
(unknown fields and variants are ignored and counted, never fatal); `canopy doctor` prints
detected harness versions and the count of unknown events seen.

---

## 5. The tree

### 5.1 Model

A node is one agent session. Adjacency list, strictly a tree (one spawner per agent).
Workflows (DAGs) are a later layer on top, not a change to this model.

```rust
pub struct Node {
    pub id: SessionId,               // ULID; UIs show a short unique prefix
    pub parent: Option<SessionId>,
    pub root: SessionId,
    pub origin: Origin,              // User | Spawned (canopy MCP) | Native (harness-internal sub-agent)
    pub spawn_call: Option<String>,  // the parent's tool call that created this node: the edge
    pub harness: String,             // "claude" | "codex" | "gemini" | "opencode" | custom ACP name
    pub vendor_session: Option<String>,
    pub title: String, pub cwd: String, pub model: Option<String>, pub policy: Policy,
    pub status: Status,              // Starting | Running | Idle | AwaitingPermission | RateLimited { resets: Option<u64> } | Done | Failed | Cancelled | Interrupted
    pub activity: String,            // one line: "Editing src/hub.rs", throttled to ~4 Hz
    pub usage: Usage,                // tokens in/out/cache; reported_cost_usd: Option<f64>, set only if the harness reports it
    pub changes: Changes,            // files touched, lines added/removed (from tool events)
    pub subtree: Rollup,             // usage + active count for this node and all descendants
    pub created: u64, pub updated: u64, pub ended: Option<u64>,
}
```

- Rollups are computed once, in the hub, by walking ancestors on change (depth is tiny). Both
  UIs just display them.
- `Native` nodes are observed, not controlled: you cannot message a Claude Task sub-agent
  separately because the harness does not allow it. The UI disables the composer on them.
- Changes come from tool events (paths, and line counts when the harness supplies a diff).
  No filesystem watcher: it would cost idle CPU and cannot attribute edits when agents share
  a checkout.

### 5.2 Transcript items

```rust
pub enum ItemKind { UserMessage, AgentMessage, Reasoning, ToolCall, FileChange, Permission, Error, Notice }
pub struct Item { pub id: String, pub session: SessionId, pub seq: u64, pub ts: u64, pub kind: ItemKind,
                  pub state: ItemState /* Streaming | Complete | Failed */, pub body: serde_json::Value }
```

Lifecycle: `item` (started, full object) -> zero or more `delta` (coalesced text) -> `item`
(complete, full object). Only complete items are persisted. A late subscriber receives
in-flight items with their accumulated text from memory.

### 5.3 Cross-vendor spawning (MCP tools)

Every session is launched with one MCP server: `canopy mcp-stdio --session <id>`, a pipe to
the daemon. The session id on the pipe tells the daemon who the caller is, which is how the
parent edge is recorded. Tools:

| Tool | Behaviour |
|---|---|
| `spawn_agent(harness, prompt, title?, model?, cwd?)` | Creates a child node, returns `{id}` immediately |
| `wait_agent(ids, timeout_s <= 240)` | Blocks until any finishes or the timeout; returns status + final message per id, or `still_running`. Bounded so it never trips a harness tool timeout; the agent calls again |
| `message_agent(id, text)` | Same path as a human `send`; allowed to own descendants and parent only |
| `list_agents()` | The caller's subtree with status and one-line results |

When a child finishes while its parent is idle, the hub delivers a notice to the parent as an
ordinary message ("agent 01HX finished: ..."). Parents do not have to poll.

**How an agent knows it is in canopy.** Three things, and nothing else:

1. The MCP `instructions` string returned at initialize (one or two sentences, already in
   `daemon/mcp.rs`). It says what canopy is and when to spawn.
2. The four tool descriptions, which include the harnesses that are usable right now.
3. For a spawned child only, one line at the top of its prompt: who spawned it and that
   its final message is returned to that parent.

Budget: at most 150 tokens of standing text beyond the tool schemas, asserted by a test on
the strings. Awareness writes nothing to the user's harness config: no hook or plugin is
installed and no system prompt is replaced. (Plugin and skill switches are a separate,
user-driven path, section 5.6.) **VERIFY** per harness that the model
actually sees `instructions` (OpenCode documents that it does; Claude and Codex are
unconfirmed). Where it does not, the same sentence goes at the top of the first prompt.

Limits (`daemon/limits.rs`), all enforced at `spawn_agent` and returned as tool errors the
model can read: max depth 3, max 16 live agents, optional per-root token budget. Defaults
are daemon flags. A spawn onto a rate-limited harness fails with the reset time, so the
parent can choose another vendor.

### 5.4 Client protocol

Requests carry an `id`; every one gets a `res`. Events arrive in `batch` frames.

```
-> hello        {token, version}                       <- {daemon_version, harnesses:[{name, available, version, capabilities}]}
-> sub_tree     {}                                     <- snapshot {nodes:[Node]}, then `node` events
-> sub_session  {session, after_seq?, tail: 200}       <- snapshot {items:[Item]}, then `item` / `delta` events
-> unsub_session{session}
-> spawn        {harness, prompt, cwd, model?, policy, parent?, title?}   <- {session}
-> send         {session, text, interrupt: bool}       <- {delivery: steered|queued|started_turn}
-> interrupt    {session}
-> permission   {session, request, decision: allow|deny|allow_always}
-> stop | resume | archive {session}
-> history      {session, before_seq, limit}           <- {items}

<- batch {events: [ {t:"node", node} | {t:"item", item} | {t:"delta", session, item, seq, text} | {t:"gone", session} ]}
```

- `node` events carry the whole node (~300 bytes). Latest-wins per id inside a batch. No patch
  format, no ordering problems, idempotent resync.
- Tree resync is always a full snapshot (it is small). Session resync uses `after_seq`.

### 5.5 SQLite

```sql
PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA cache_size = -2048;
CREATE TABLE sessions (id TEXT PRIMARY KEY, parent TEXT, root TEXT NOT NULL, node TEXT NOT NULL /* Node as JSON */,
                       status TEXT NOT NULL, updated INTEGER NOT NULL) STRICT;
CREATE INDEX sessions_root ON sessions(root);
CREATE TABLE items (seq INTEGER PRIMARY KEY AUTOINCREMENT, session TEXT NOT NULL, item TEXT NOT NULL /* Item as JSON */) STRICT;
CREATE INDEX items_session ON items(session, seq);
```

Node stored as one JSON column: the struct will change often during development and nothing
queries its inner fields in v1. Promote fields to columns when a query needs them.

### 5.6 Plugins and skills (ADR 0020)

One view, in the TUI and the web, that lists every plugin and skill across the supported
harnesses and lets the user switch each one on or off. Canopy is the place they are
managed; the vendors still store and load them.

- **Scopes**: global and per project. Never per agent: an agent gets whatever its harness
  and project have switched on.
- **One switch per harness**: a row shows the item and a switch for each harness that has
  it, so a plugin can be on for Codex and off for Claude.
- **How a switch is applied**: through the vendor's own interface, never by hand-editing
  its files.

  | | Claude Code | Codex |
  |---|---|---|
  | List | `claude plugin list --json` | app-server `plugin/installed`, `skills/list` |
  | Plugin on/off | `claude plugin enable\|disable --scope user\|project --json` | app-server `config/value/write` on `plugins."name@marketplace".enabled` |
  | Skill on/off | no vendor switch: only library skills can be switched (below) | app-server `skills/config/write` |

- **One shared skill library**: `~/.canopy/skills/<name>/SKILL.md`. Both vendors read the
  same format. A library skill reaches a harness through a symlink in that harness's
  skill directory (`~/.claude/skills/`, `~/.agents/skills/`, or the project equivalents);
  off for that harness means the link is removed. A skill already sitting in a vendor
  directory is listed, and can be adopted into the library by an explicit action.
  **VERIFY** that Claude Code follows a symlinked skill folder (Codex documents that it
  does), and that one `SKILL.md` loads unchanged in both.
- **Plugins are not shared**: the two plugin formats differ, so plugins are listed per
  harness. Canopy does not translate one into the other.
- **Never auto-accept**: every change is one explicit user action on one named item. A
  command declared by a marketplace is shown in full and needs a yes each time. The first
  version does not install plugins; it shows the vendor's install command.
- **Running agents are not affected**: a change applies to sessions started after it, and
  the view says so next to the switch.
- **No cache, no polling**: the view reads the vendor's current answer when it opens and
  after each change. Nothing is stored in SQLite.
- **Protocol**: two requests, `ext_list {cwd?}` and `ext_set {harness, kind, id, scope,
  enabled}`. Tests run them against fake CLIs in a temp home, never the real configs.

---

## 6. Front ends

### 6.1 Shared interaction model

One command registry per client (id, title, default key). The palette and the keymap both
read it. Keys are identical in TUI and web:

`j/k` move - `h/l` collapse/expand - `Enter` open session - `c` new agent - `s` spawn child
of selection - `i` focus composer - `Esc` back - `x` interrupt - `a/d` allow/deny permission
- `/` filter - `Cmd-K` or `:` palette - `g t` tree - `g g` graph (web) - `g p` plugins and
skills - `?` help.

Visual rules, the same in both front ends. Attention to detail is a requirement here, not
polish:

- **Icons are required** for every status, harness, scope and action. A row never relies
  on a word or a colour alone.
- **Never an emoji**, in the UI, in agent-facing text, or in docs.
- **No eyebrow labels**: the small uppercase tag above a heading that generated UIs lean
  on. A heading stands on its own.
- **Global scope is an earth globe**: a world with continents, not the wireframe "www"
  grid globe. Project scope is a folder.
- **Web**: lucide icons with `currentColor`; status is conveyed by shape.
- **TUI**: terminals have no images, so an icon is a single-cell glyph. Nerd Font glyphs
  (a free font patched with icons, which has an earth globe) when the user turns them on;
  otherwise plain Unicode symbols in text presentation. **VERIFY** before M4 which glyphs
  render at one cell width in Terminal.app, iTerm2, Ghostty and tmux, and whether a
  Nerd Font can be detected or must be a setting. The default must look right with no
  setup.
- Enforced, not remembered: `make check` greps `tui/` and `web/src/` for emoji code points.

### 6.2 TUI (`crates/canopy/src/tui/`)

This is what bare `canopy` opens, and the main way the product is used (section 0.2).
`ratatui` + `crossterm`, a client of the Unix socket like any other. It auto-starts the
daemon, so there is nothing to run first; quitting the TUI leaves the agents running and
`canopy` reattaches to them. Left: outline tree with
status glyph, harness tag, title, activity, tokens/cost, +/- lines. Right: transcript of the
selected session. Bottom: composer with delivery state. Draws only when an event or key
arrives, capped at 30 fps. No tick loop, so an idle TUI uses no CPU.

### 6.3 Web (`web/`)

- **State**: one external React store `{nodes: Record<id, Node>, sessions: Record<id, {items, live}>}`
  mirrored from the daemon. Navigation, selection, filter, and palette never touch the
  network. That is the Linear trick.
- **Ingest**: `conn.ts` receives a batch, queues it, and applies all queued batches inside one
  store batch in `requestAnimationFrame`: at most one DOM commit per frame however fast events
  arrive. A hidden tab drops its session subscriptions (keeps the tree) and resyncs on focus.
- **Streaming text**: a delta appends to the selected session snapshot. Streaming items
  render as plain pre-wrapped text. Markdown is lexed once when the item completes, never per
  delta. `marked` is loaded lazily.
- **Views**: Outline tree (primary; rows as above). Graph (SVG: x = depth, y = outline row,
  elbow edges, status icons on nodes, live activity; layout is derived from the outline order
  in ~30 lines, no layout library). Session (transcript + composer + permission bar + node
  details). Palette. New-agent dialog.
- **Rendering cost controls**: `content-visibility: auto` on transcript rows, a 300-item
  window with `history` paging, CSS transitions only on `transform`/`opacity`, no layout
  thrash in the streaming path.
- **Stack (ADR 0022)**: React, shadcn/ui on Base UI, Tailwind, lucide icons. No router
  or state library. The neutral theme follows the system.
- **Dev**: `vite` dev server proxying `/ws` to the daemon. Release: `vite build` output is
  embedded into the binary.

---

## 7. Repository layout

```
canopy/
  AGENTS.md                    # always-loaded agent instructions (self-improving, section 9)
  CLAUDE.md                    # one line: @AGENTS.md
  .gemini/settings.json        # points Gemini CLI's context file at AGENTS.md
  LICENSE                      # MIT
  README.md                    # what it is, install, every startup command
  PLAN.md                      # this file; deleted once M6 ships (the code and ADRs replace it)
  Makefile                     # check, test, e2e, bench, web, build, gen, check-docs
  Cargo.toml                   # workspace + release profile
  rust-toolchain.toml
  .github/workflows/ci.yml     # make check test e2e check-docs; bench on main
  docs/
    decisions/                 # ADRs: 0001-rust-core.md ... one per row of section 2
      README.md                # generated index
    harness-notes/             # per-harness verified facts + CLI version they were verified on
  scripts/
    check-docs.sh              # AGENTS.md linter (section 9.3)
  crates/
    protocol/                  # no tokio; types only
      src/lib.rs  node.rs  item.rs  msg.rs
    canopy/
      src/
        main.rs                # subcommand dispatch: (none)=tui, daemon, run, ls, send, tail, stop, web, doctor, mcp-stdio, fake-agent
        paths.rs
        client.rs              # socket client + daemon autostart; used by cli and tui
        cli.rs
        jsonrpc.rs             # line-delimited JSON-RPC peer: codex, acp, mcp
        daemon/
          mod.rs               # startup, lock, idle exit, signal handling
          hub.rs               # the single state owner
          store.rs             # sqlite thread
          ipc.rs               # unix socket server
          http.rs              # static assets + websocket + token/origin checks
          mcp.rs               # spawn_agent / wait_agent / message_agent / list_agents
          limits.rs            # depth, live-agent count, token budget
        harness/
          mod.rs               # trait, SpawnSpec, HarnessEvent, delta coalescer
          claude.rs  codex.rs  acp.rs
          fixtures/            # recorded transcripts, named <harness>-<version>-<scenario>.jsonl
          AGENTS.md            # local rules for adapters
        tui/
          mod.rs  tree.rs  session.rs  input.rs
        fake_agent.rs          # scripted ACP agent
        mcp_stdio.rs           # stdin/stdout <-> daemon pipe
      tests/
        e2e.rs                 # real daemon in a temp CANOPY_HOME + fake agent
        bench.rs               # load + budget assertions
  web/
    AGENTS.md                  # local rules for the web app
    package.json  tsconfig.json  vite.config.ts  index.html
    src/
      main.tsx  conn.ts  store.ts  keys.ts  md.tsx  styles.css
      protocol.gen.ts          # generated by `make gen`; never edited
      views/  Tree.tsx  Graph.tsx  Session.tsx  Composer.tsx  Palette.tsx  NewAgent.tsx
```

---

## 8. Milestones

Each milestone ends with its exit test green and `make check-docs` passing. Build in order;
each is usable on its own.

**M0 - Scaffold.** Workspace, Makefile, CI, `protocol` crate with all section 5 types and
`ts-rs` export, `AGENTS.md` + nested ones, `scripts/check-docs.sh`, ADRs 0001-0017 from
section 2, MIT `LICENSE`, README with commands. Exit: `make check test check-docs` green.

**M1 - Core on a fake agent.** `jsonrpc.rs`, `fake_agent.rs`, `harness/acp.rs`, hub, store,
ipc, client autostart, CLI (`run`, `ls`, `send`, `tail`, `stop`, `daemon`). Exit: e2e spawns
a fake agent, tails streamed output, sends a mid-turn message and sees `queued` then
delivery, stops it, restarts the daemon and reads history. First budget check (section 10).

**M2 - Real harnesses.** Resolve every **VERIFY** first and write the answers into
`docs/harness-notes/`. Then `claude.rs`, `codex.rs`, Gemini and OpenCode through `acp.rs`,
permissions end to end, token usage, rate-limit detection, `canopy doctor` (version and
login state per CLI), recorded fixtures. Exit: fixture replay tests
for each harness; opt-in live smoke test (`CANOPY_LIVE=1 make e2e`) runs one trivial prompt
per installed harness and sends it a follow-up.

**M3 - The tree.** `mcp.rs`, `mcp_stdio.rs`, limits, rollups, Claude native sub-agent
mapping, completion notices. Exit: e2e where a fake agent spawns two children of different
"harnesses", waits, and the rollup is correct; depth and budget limits return tool errors.
Live: Claude spawns a Codex child that edits a file.

**M3.5 - Product path (next).** In this order, each step usable on its own:
1. Scrub the recorded fixtures and make the recording step scrub (section 0.3).
2. The detection table, parallel `doctor`, `canopy demo`.
3. Awareness: the child prompt line and the 150-token test (section 5.3).
Exit: e2e covers `doctor` with zero, one and two CLIs found (fake binaries on a temp
PATH) and with one logged out.

**M4 - TUI, the control room.** Bare `canopy` opens it (sections 0.2 and 6.2). Until this
lands, bare `canopy` prints the `doctor` table. Exit: scripted key-driven test against the
fake agent covering first run with no agents, launch, message, approve, interrupt, quit
and reattach; idle CPU budget holds with the TUI open.

**M4.5 - Release pipeline.** After the TUI, so the first public install opens the control
room. `dist` config, a tagged pre-release, shell installer and Homebrew tap, README
install section rewritten around them and re-run from a clean shell. Exit: the install
line works on a machine without Rust. Needs two things from the user first: the final
name and a public GitHub repository.

**M5 - Web.** `http.rs`, auth, the React + shadcn/ui app. Exit: in a real browser with 50 fake agents
streaming, the tree stays at 60 fps, the bundle and heap budgets hold, a killed and restarted
daemon resyncs the page without reload, and a cross-origin WS attempt is rejected.

**M5.5 - Plugins and skills (section 5.6, ADR 0020).** Resolve the two **VERIFY** items
first. Then, in order: the list view (read-only) in both front ends; plugin switches;
Codex skill switches; the shared skill library with adopt and per-harness links. Exit: e2e
against fake CLIs in a temp home toggles a plugin per harness and per scope, links and
unlinks a library skill, and proves a declared command is never run without a yes.

**M6 - Hardening and release.** Resume interrupted sessions, reconnect paths, budgets
enforced in CI, README complete, first non-pre-release tag, delete PLAN.md.

### v1 vs later

| v1 | Later |
|---|---|
| Launch, message, interrupt, approve, stop from CLI, TUI, web | Attaching to sessions started outside canopy (hook install) |
| Claude, Codex; OpenCode, Copilot and other ACP agents as each is verified | Gemini (on hold, section 0.3); OpenCode native HTTP adapter if ACP proves too thin |
| One-line install, detection, zero-config first run, `canopy demo` | An opt-in `canopy connect` that registers the canopy tools in hand-started sessions through each CLI's own `mcp add` command |
| Cross-vendor spawn tree with limits and rollups | Declarative workflows (DAGs, templates, retries) |
| Outline + graph views, transcripts, tokens/cost, files touched | Timeline/swimlane view, full diff review, search, syntax highlighting |
| SQLite history, resume | Git worktree isolation per agent with merge UI |
| Rate-limit state per node, one-command respawn on another harness | Automatic failover and quota-aware routing across subscriptions |
| One view to list and switch plugins and skills per harness, global and per project; one shared skill library | Installing plugins from canopy (command shown, confirmed every time, never auto-accepted) |
| Icons everywhere, no emoji, no eyebrow labels | Agent modes (plan, auto, manual) shown and switched per node |
| - | Feasibility check only: canopy managing plugins and skills natively, writing into `~/.claude` and `~/.codex` itself with no vendor command. Answer: which files, how stable across CLI releases, what the vendors' terms say, and what it would buy over section 5.6 |
| macOS, Linux | Windows, desktop notifications |

Permanent non-goals: server or hosted deployment, remote daemons, multi-user, accounts,
telemetry, handling vendor credentials, calling vendor APIs directly, cost estimation.

Worktree isolation deserves a note: agents sharing a checkout will collide, and it is the
first "later" item I would pull forward once the tree is in daily use.

---

## 9. Self-improving AGENTS.md

The failure mode of agent-maintained docs is append-only growth: every session adds a
"lesson", nothing is removed, half of it goes stale, and agents learn to ignore the file.
The design below makes staleness mechanically detectable and growth impossible.

### 9.1 Three tiers, by how often they are read

| Tier | File | Loaded | Holds | Mutability |
|---|---|---|---|---|
| 1 | `AGENTS.md` (root) | Every session | Invariants, commands, conventions, the update protocol. Hard cap 150 lines | Edited in place |
| 2 | Nested `AGENTS.md` (`harness/`, `web/`) | When working in that directory | Local rules. Cap 80 lines each | Edited in place |
| 3 | `docs/decisions/NNNN-title.md`, `docs/harness-notes/` | On demand, linked from tiers 1-2 | Why a decision was made; verified harness facts with the CLI version | ADRs are immutable; a reversal is a new ADR that marks the old one `Superseded by` |

### 9.2 Rules that keep it honest (these live in AGENTS.md itself)

1. **Executable beats prose.** If a lesson can be a type, a test, a lint, or a Makefile
   target, make it one and do not write the prose. When a prose rule later becomes
   enforced, delete the prose. AGENTS.md is the last resort, for what cannot be checked.
2. **Edit beats append.** A wrong line is fixed or deleted in the same change that
   discovered it. Contradictions are never resolved by adding a newer line below.
3. **A budget, not a backlog.** At the cap, adding a line means removing or demoting one.
4. **Every line must be checkable.** A rule names a path, a command, or an ADR. "Be careful
   with X" is not allowed; "X must go through `hub.rs` (ADR 0003)" is.
5. **Update triggers** (end of every task, same commit as the code): the user corrected the
   agent; a non-obvious constraint cost real time; a decision now constrains future work; a
   line in AGENTS.md turned out to be false or unneeded.
6. **Never record**: task status, changelogs, anything `grep` or `git log` answers.

### 9.3 Mechanical enforcement: `scripts/check-docs.sh` (in `make check-docs` and CI)

- Line caps for each tier.
- Every backticked repo path in any AGENTS.md exists.
- Every `make <target>` mentioned exists in the Makefile.
- Every `ADR NNNN` reference resolves to a file; `docs/decisions/README.md` matches the
  directory; a superseded ADR is not referenced from tier 1 or 2.
- Every bullet under `## Lessons` ends with a `(YYYY-MM)` stamp; stamps older than 90 days
  print a warning list to re-verify (warn, not fail).

So renaming a file, removing a Makefile target, or reversing a decision without updating
the docs fails CI. That is the anti-rot mechanism; the rest is convention.

### 9.4 Gardening

At the end of each milestone, one task re-reads tiers 1-2 line by line: verify, tighten,
promote to a check where possible, demote detail to tier 3, clear the 90-day warnings.

---

## 10. Verification

Budgets, asserted in `tests/bench.rs` (targets to validate at M1 and M5, then enforced):

| Metric | Budget |
|---|---|
| Daemon idle RSS | <= 15 MB |
| Daemon idle wakeups | 0 per second with no sessions |
| Daemon start to socket ready | <= 20 ms |
| CLI `canopy ls` round trip (daemon running) | <= 10 ms |
| TUI first paint | <= 50 ms |
| 50 fake agents x 100 deltas/s: daemon CPU | <= 15% of one core |
| Same load: event to client latency p99 | <= 50 ms |
| Web JS (gzip, initial) | <= 120 KB |
| Web: frame rate under the same load | 60 fps, no long tasks > 50 ms |
| Web heap with 50 sessions | <= 60 MB |

How to test end to end:
- `make e2e`: real daemon in a temp `CANOPY_HOME`, fake agents, drives the CLI as a user
  would. Daemons started by tests run in the foreground and are killed by the test.
- `CANOPY_LIVE=1 make e2e`: one cheap prompt per installed harness plus a follow-up message.
  Costs tokens, so never in CI.
- Web: open the app against a daemon running `canopy fake-agent` load, record a performance
  trace, check the budgets, and review the UI at pixel level in light and dark.
- Adapters: fixture replay per harness version; a new CLI release gets a new recording
  before the adapter is touched.

---

## 11. Things to verify before M2 (do not code from memory)

| Question | How |
|---|---|
| Claude: exact stdin shape for `interrupt` and for answering `can_use_tool` when driving the raw CLI | Read the Agent SDK's transport source for the installed version; record a fixture |
| Claude: `--forward-subagent-text` availability and the minimum CLI version | `claude --help`, release notes |
| Gemini: `--acp` vs `--experimental-acp`, whether one process serves several sessions | `gemini --help`, a two-session probe |
| Codex: does app-server expose sub-agent/collab thread parentage; per-thread MCP config and tool timeout | `codex app-server generate-json-schema`, read the schema |
| ACP: current usage-reporting update and which agents send it | agentclientprotocol.com spec, probe each agent |
| OpenCode: does `opencode acp` report token usage; if not, is the HTTP API worth an adapter | Probe, compare with `opencode serve` events |
| Every harness: what a plan rate limit looks like on the wire, and whether a reset time is given | Find the event in each protocol schema; record a fixture when one occurs |
| Every harness: a side-effect-free way to tell "installed but not logged in" for `canopy doctor` | Each CLI's status or auth subcommand. Done for Claude and Codex; Copilot documents none |
| Every harness: does the model see the MCP `instructions` string | One live prompt per harness asking what it knows about canopy; needs the user's go-ahead, it spends tokens |
| Claude: does it load a skill whose folder is a symlink; does one `SKILL.md` load unchanged in Claude and Codex | A throwaway skill in a temp project, linked into both; needs the user's go-ahead, it spends tokens |
| Codex: are `config/value/write`, `skills/config/write` and `plugin/installed` usable on the installed app-server (upstream marks plugin methods under development) | Call them against a temp `CODEX_HOME`; record a fixture |
| TUI: which icon glyphs render one cell wide in common terminals; can a Nerd Font be detected | Print a test card in Terminal.app, iTerm2, Ghostty and tmux |
| macOS: does a curl-installed binary run without a Gatekeeper prompt | Install the pre-release on a clean user account |
| Each vendor's current terms on a third-party tool driving its CLI on a subscription | Read the primary pages first hand before each release (section 0.3) |
