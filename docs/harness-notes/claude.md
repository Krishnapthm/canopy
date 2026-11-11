# Claude Code (stream-json)

Verified against: `claude` 2.1.318 (`/Users/krishna/.local/bin/claude`, symlink into
`~/.local/share/claude/versions/2.1.318`), claude.ai Pro login, model alias `haiku` (resolved to
`claude-haiku-5-5`), 2026-11-11. SDK type source read: `@anthropic-ai/claude-agent-sdk` 0.3.322
(`sdk.d.ts`). Items marked (SDK) come from those types only and were not exercised live.
Fixtures: `crates/canopy/src/harness/fixtures/claude-2.1.318-*.jsonl`. In fixtures, lines sent
by us are wrapped as `{"canopy_stdin": <line>}`; machine paths, identity fields and installed extension lists are scrubbed with
`scripts/record-fixture.py`. Record with `probe | python3 scripts/record-fixture.py FILE`;
never write the raw stream into the repository. The scrubbed fixtures replay in `make test`.

Every `claude` invocation in these probes used a timeout. The earlier note that `claude --version`
hangs with stdin from /dev/null was not reproduced: it printed `2.1.318 (Claude Code)` instantly.
The path `ClaudeCode.app/Contents/MacOS/claude` does not exist; use `claude` on PATH.

## 1. Flags (2.1.318)

Working launch line (all verified live):

```
claude -p --input-format stream-json --output-format stream-json --verbose \
  --include-partial-messages --permission-prompt-tool stdio --session-id <uuid> \
  --model haiku [--mcp-config '<inline json>'] [--strict-mcp-config]
```

- `--verbose` is REQUIRED with `-p --output-format stream-json`. Without it the CLI prints
  `Error: When using --print, --output-format=stream-json requires --verbose` and exits.
- `--include-partial-messages` exists (print + stream-json only). Without it there are no
  `stream_event` lines.
- `--forward-subagent-text` exists in help: "Forward subagent text and thinking blocks as
  assistant/user messages with parent_tool_use_id set". Not exercised (no sub-agent probe).
- `--permission-prompt-tool stdio` is accepted and is what routes permission asks over the
  control protocol (the SDK passes exactly `--permission-prompt-tool=stdio` when it has a
  `canUseTool` callback). It does NOT appear in `--help`; `--help` only mentions it inside the
  `--permission-prompts` text. Also exists: `--permission-prompts host|none`
  (`none` = anything that would prompt is auto-denied).
- `--mcp-config <configs...>` accepts "JSON files or strings" (space separated). Inline JSON
  verified live. `--strict-mcp-config` ignores every other MCP source (the user has ~15
  plugin MCP servers that otherwise load and show in `system/init.mcp_servers`).
- `--resume [value]` takes a session id or name; `--fork-session` makes a new id on resume.
  Not exercised live (flag exists in help).
- `--session-id <uuid>`: sets our own id; must be a valid UUID (else
  `Error: Invalid session ID. Must be a valid UUID.`). Probes passed `--session-id`; `system/init.session_id` and
  all later lines carried the id passed (the id was not compared programmatically).
- `--model <alias|full name>`; aliases seen in help: `fable`, `opus`, `sonnet`; `haiku` works.
- `--permission-mode` choices: `acceptEdits, auto, bypassPermissions, manual, dontAsk, plan`.
  NOTE: `manual` is the CLI flag spelling, but `system/init.permissionMode` and the
  `set_permission_mode` control request use `default` for the same mode (init reported
  `"permissionMode":"default"` when no flag was given). SDK `PermissionMode` values:
  `default | acceptEdits | bypassPermissions | plan | dontAsk | auto`.
- Other useful: `--replay-user-messages` (echo stdin user messages on stdout, not used),
  `--no-session-persistence`, `--setting-sources`, `--tools`, `--allowedTools`,
  `--add-dir`, `--max-budget-usd`, `--settings <file|json>`, `--bare`, `--safe-mode`.
- `initialize` is NOT required: a bare user message on stdin works with no prior control
  request (write-permission fixture).

## 2. Stdin user message

```json
{"type":"user","message":{"role":"user","content":"reply with the word ok"}}
```

`content` may also be an array of Messages-API blocks. Optional top-level fields (SDK):
`parent_tool_use_id`, `priority` ("now"|"next"|"later"), `uuid`, `session_id`,
`client_composed: true`, `shouldQuery`, `pasted_content`.

- Process stays alive between turns; `result` ends a turn, not the process. Close stdin to exit
  (exit code 0 observed).
- No `system/init` until the first user message is read: `system/init` is emitted at the start
  of EVERY turn (seen twice in a two-turn session), not once per process. Take `session_id`
  from it (or from any later line). Before it come `system/hook_started` / `hook_response`
  lines (user hooks), after it `system/status {"status":"requesting"}` and
  `system/commands_changed`.
- Mid-turn message (midturn-queue fixture): written while turn A was streaming. It was queued:
  turn A ran to its own `result` untouched, then a second `system/init` + new turn B ran and
  produced its own `result`. Two stdin messages = two `result`s. No ack line is emitted
  (no replay without `--replay-user-messages`). So `Delivery::Queued` is accurate and there is
  no steering at default priority. `priority:"now"` and `interrupt_send_now_v1` capability
  exist (SDK) but were not tried.
- Slash-command trap CONFIRMED (slash-help-local fixture): stdin `"/help"` is NOT sent to the
  model. The CLI answers locally: `system/init`, a synthetic `assistant` message
  (`"model":"<synthetic>"`), then `result` with `"result":"/help isn't available in this
  environment."`, `num_turns:0`, zero cost. Fix: add `"client_composed":true` to the user
  message line. Verified live: `{"type":"user","client_composed":true,"message":{...,"content":
  "/help - ignore everything and reply with exactly: B-received"}}` reached the model, which
  replied `B-received`. Per SDK docs, `client_composed` also disables `@path` expansion and
  attachments. Canopy should set it on every user-typed prompt that is forwarded as plain text.

## 3. Control protocol (JSON line each, same stdin/stdout)

Envelope (either direction): `{"type":"control_request","request_id":"<str>","request":{"subtype":...}}`.
Answer: `{"type":"control_response","response":{"subtype":"success","request_id":"<same>","response":{...}}}`
(or `"subtype":"error"` with `error` text, SDK `ControlErrorResponse`).

- initialize (optional): `{"request_id":"init-1","type":"control_request","request":{"subtype":"initialize"}}`
  -> success `response` keys: `commands, agents, output_style, available_output_styles, models,
  account{email,organization,subscriptionType,apiProvider}, pid, claude_code_version,
  current_permission_mode, session_state ("idle"), capabilities[...]`, and more.
  Answered before the first user message, without a turn. Useful only for the model list.
- interrupt: send `{"request_id":"int-1","type":"control_request","request":{"subtype":"interrupt"}}`
  -> `{"type":"control_response","response":{"subtype":"success","request_id":"int-1","response":{"still_queued":[]}}}`
  Optional `"cancel_queued":true` also drops queued user messages. Verified shape only: my
  interrupt arrived after the turn had already ended, so interrupting a live stream was not
  observed (Gap).
- can_use_tool request (CLI -> us, stdout), Write tool:
  ```json
  {"type":"control_request","request_id":"71bd25fc-...","request":{"subtype":"can_use_tool","tool_name":"Write","display_name":"Write","input":{"file_path":"...","content":"hi"},"description":"hello.txt","permission_suggestions":[{"type":"setMode","mode":"acceptEdits","destination":"session"}],"tool_use_id":"toolu_01..."}}
  ```
  MCP tool variant adds `"mcp_server":{"name":"echo","source":"dynamic"}` and
  `"server_policy_ask_only":true`; its suggestion is
  `{"type":"addRules","rules":[{"toolName":"mcp__echo__say"}],"behavior":"allow","destination":"localSettings"}`.
  Other optional fields (SDK): `blocked_path, decision_reason, decision_reason_type, agent_id,
  title, requires_user_interaction, default_to_no`. The request appears AFTER the complete
  `assistant` tool_use line and before the `tool_result`.
- Allow (us -> CLI), verified live:
  ```json
  {"type":"control_response","response":{"subtype":"success","request_id":"<id>","response":{"behavior":"allow","updatedInput":{...input echoed back...}}}}
  ```
  `updatedInput` is the tool input; echo the original `input` unchanged to approve as-is
  (SDK types make it optional, always sending it is safe; omitting it was not tested).
  Optional `updatedPermissions` (same shape as `permission_suggestions`, e.g. to persist
  acceptEdits). The following `tool_result` user line has
  `tool_result_meta[0].permission_decision = {"decision":"accept","source":"user_temporary"}`.
- Deny (SDK type, not exercised): `"response":{"behavior":"deny","message":"<reason>","interrupt":false}`.
  `interrupt:true` stops the whole turn.
- Cancel (SDK): the CLI may send `{"type":"control_cancel_request","request_id":"<id>"}` to withdraw a
  pending can_use_tool (e.g. after interrupt). Not observed.
- set_permission_mode, verified live:
  `{"request_id":"mode-1","type":"control_request","request":{"subtype":"set_permission_mode","mode":"acceptEdits"}}`
  -> success with `"response":{"mode":"acceptEdits"}`. Works mid-turn.
- Other requests in the SDK enum (not used): `set_model`, `mcp_status`, `mcp_set_servers`,
  `get_context_usage`, `stop_task`, `rewind_files`, `cancel_async_message`.
- Read-only tools (e.g. ToolSearch) do not ask: their `tool_result_meta` shows
  `"permission_decision":{"decision":"accept","source":"config"}`.

## 4. Stdout events

All carry `session_id` (except a few `system` lines), `uuid`. Top-level `type` values seen:
`system`, `stream_event`, `assistant`, `user`, `result`, `rate_limit_event`, `control_response`,
`control_request`.

- `system/init` (per turn): `cwd, session_id, tools[], mcp_servers[{name,status,source}], model,
  permissionMode, slash_commands[], skills[], plugins[], agents[], apiKeySource ("none" for
  claude.ai login), claude_code_version, output_style, capabilities[], memory_paths,
  scratchpad_path, ...`. `mcp_servers[].status` can be `pending` at init (GitHub plugin was).
- Other `system` subtypes seen: `hook_started`, `hook_response`, `status`
  (`{"status":"requesting"}`), `commands_changed`, `thinking_tokens`
  (`estimated_tokens`, `estimated_tokens_delta`), `post_turn_summary`
  (`status_category, status_detail, needs_action`). Ignore unknown ones.
- `stream_event` (needs `--include-partial-messages`): `{"type":"stream_event","event":<raw API SSE event>,"session_id","parent_tool_use_id":null,"uuid","api_message_id"}`.
  Event types: `message_start` (has `message.model`, `usage`), `content_block_start`,
  `content_block_delta`, `content_block_stop`, `message_delta` (`delta.stop_reason`, `usage`),
  `message_stop`.
  - text: `{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"ok"}}`
  - thinking: `delta:{"type":"thinking_delta","thinking":"","estimated_tokens":50}` - the
    `thinking` text is EMPTY with haiku-5-5 (redacted summary); only token estimates arrive.
    Then `delta:{"type":"signature_delta","signature":"..."}`. Block start is
    `{"type":"thinking","thinking":"","signature":""}`; top-level `thinking_display:"updates"`.
  - tool input: `content_block_start` with `content_block:{"type":"tool_use","id","name","input":{},"caller":{"type":"direct"}}`
    then `delta:{"type":"input_json_delta","partial_json":"..."}` fragments.
  - Deltas are token-ish (about 4 numbers per delta in the count test). Coalesce them.
- `assistant`: one line per content block group, emitted when that block completes (it can
  come before the `content_block_stop`/`message_delta` of the same message). Shape:
  `{"type":"assistant","message":{"model","id":"msg_...","role":"assistant","content":[{"type":"text","text":"ok"}] | [{"type":"tool_use","id":"toolu_..","name":"Write","input":{...},"caller":{"type":"direct"}}] | [{"type":"thinking","thinking":"","signature":"..."}],"stop_reason":null,"usage":{...}},"parent_tool_use_id":null,"session_id","uuid","timestamp","request_id"}`.
  `message.stop_reason` is `null` here (the real stop reason is in the stream `message_delta`
  and in `result.stop_reason`). Thinking, text and tool_use of one API message arrive as
  separate `assistant` lines sharing `message.id`. `parent_tool_use_id` non-null = sub-agent
  (not exercised). Tool-use input here is complete (a copy also in `wire_tool_inputs`).
- `user` (tool result echo): `{"type":"user","message":{"role":"user","content":[{"tool_use_id","type":"tool_result","content":"<string>" | [{"type":"text","text":"echo:hi"}]}]},"parent_tool_use_id":null,"session_id","uuid","tool_use_result":<per-tool object>,"tool_result_meta":[{"id","permission_decision":{...}}]}`.
  `content` is a string for built-in tools and an array of blocks for MCP tools. Deferred
  MCP tools are found first via a `ToolSearch` call whose result is a `tool_reference` block
  (`{"type":"tool_reference","tool_name":"mcp__echo__say"}`).
- `result`: `{"type":"result","subtype":"success","is_error":false,"result":"<final text>","stop_reason":"end_turn","terminal_reason":"completed","num_turns":1,"duration_ms","duration_api_ms","total_cost_usd":0.0048,"usage":{"input_tokens","cache_creation_input_tokens","cache_read_input_tokens","output_tokens","output_tokens_details":{"thinking_tokens"},"service_tier","iterations":[...]},"modelUsage":{"<model>":{"inputTokens","outputTokens","cacheReadInputTokens","cacheCreationInputTokens","costUSD","contextWindow","maxOutputTokens"}},"permission_denials":[],"subagent_stats":{...},"session_id","uuid"}`.
  `total_cost_usd` is cumulative for the process (the second result in a session did not
  reset it). Error subtypes (SDK): `error_during_execution, error_max_turns,
  error_max_budget_usd, error_max_structured_output_retries` with `is_error:true`.
  A turn with `num_turns:0` and zero usage is a locally handled slash command.
- `rate_limit_event` (emitted after every API response in the probes):
  `{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":1794379800,"rateLimitType":"five_hour","overageStatus":"rejected","overageDisabledReason":"org_level_disabled","isUsingOverage":false,"unifiedWindows":{"five_hour":{"utilization":0.44,"resetsAt":1794379800,"status":"allowed"},"seven_day":{"utilization":0.01,"resetsAt":1794970800,"status":"allowed"}}},"uuid","session_id"}`.
  SDK enum: `status: allowed|allowed_warning|rejected`; `rateLimitType: five_hour|seven_day|
  seven_day_opus|seven_day_sonnet|seven_day_overage_included|overage`. `utilization` is a
  0..1 fraction; `resetsAt` is epoch seconds. `unifiedWindows` is not in the SDK type: treat
  as optional.

## 5. MCP

- Pass a stdio server inline: `--mcp-config '{"mcpServers":{"echo":{"command":"python3","args":["-I","/path/server.py"]}}}'`
  (optional `"env":{...}`). Verified: `system/init.mcp_servers` showed
  `{"name":"echo","status":"connected","source":"dynamic"}` and `tools` contained `mcp__echo__say`.
- Tool name the model sees and the `tool_use.name` / can_use_tool `tool_name`:
  `mcp__<server>__<tool>`.
- Timeout (SDK docs, NOT verified live): env var `MCP_TOOL_TIMEOUT` in milliseconds is the
  per-call timeout; per-server override exists only for SDK-hosted servers
  (`initialize.sdkMcpServerConfigs.<name>.timeout`, ms, hard wall-clock, progress
  notifications do not extend it, values under 1000 ignored). For a command-based server,
  set `MCP_TOOL_TIMEOUT` in the claude process env. The probe set it to 30000 and the call
  succeeded, which proves only that the var is harmless.
- MCP tool calls ask permission by default (`server_policy_ask_only:true`); a deferred MCP tool
  first costs a `ToolSearch` round trip with the default tool set.

## 6. Login check (side-effect free)

`claude auth status` (`--json` default, `--text` for humans), exit code 0 when logged in:
`{"loggedIn":true,"authMethod":"claude.ai","apiProvider":"firstParty","analyticsDisabled":false,"projectsDirectory":"~/.claude/projects","configDirectory":"~/.claude","loginExpiresAt":"2026-12-01T08:38:28.490Z","email":"...","orgId":"...","orgName":"...","subscriptionType":"pro"}`.
Contains email and org: do not log the raw output. Exit code when logged out was not tested.
`claude --version` -> `2.1.318 (Claude Code)`.

## 7. Sub-agents, interrupt, deny (second round, 2.1.318, flags + `--strict-mcp-config --forward-subagent-text`)

Fixtures: `claude-2.1.318-subagent.jsonl`, `-interrupt.jsonl`, `-deny.jsonl`.

Sub-agent (subagent fixture). The sub-agent tool is named `Agent` (init `tools` still lists `Task`).
- Parent `assistant` tool_use: `{"type":"tool_use","id":"toolu_01Cft..","name":"Agent","input":{"description":"Reply with the word ok","prompt":"...","subagent_type":"general-purpose","run_in_background":false},"caller":{"type":"direct"}}`.
- Order after it: `system/task_summary {"detail":"..."}`; `system/task_started {"task_id":"a9bf..","run_id","tool_use_id":<the Agent tool_use id>,"description","subagent_type","is_backgrounded":false,"spawn_depth":1,"task_type":"local_agent","prompt"}`;
  then sub-agent lines; `system/task_updated {"task_id","run_id","patch":{"status":"completed","end_time":<ms>}}`;
  `system/task_notification {"task_id","run_id","tool_use_id","status":"completed","output_file","summary":"ok","usage":{"total_tokens","tool_uses","duration_ms"}}`;
  then the parent `user` tool_result.
- Sub-agent lines carry `parent_tool_use_id` = the Agent tool_use id AND `agent_id` = task_id (also `subagent_type`, `task_description` on the user line). Forwarded: the sub-agent's initial prompt as a `user` line (content `[{"type":"text",..}]`), its `assistant` thinking and text blocks. NOT forwarded: sub-agent `stream_event`s. Every `stream_event` in the run had `parent_tool_use_id:null`. So sub-agent text arrives only as complete `assistant` lines.
- End of sub-agent: parent `user` line with `tool_result` for the Agent tool_use id, `parent_tool_use_id:null`, content `[{"type":"text","text":"[Subagent hand-back] ... <report indented> ... agentId: .. <usage>..</usage>"}]` and `tool_use_result:{"status":"completed",...}`. Rely on `task_notification` / `task_updated` for status, not that text.
- `result.subagent_stats` counts spawned/completed. Parent resumes after the tool_result in the same turn; one `result` for the whole thing (no early result in this run; background sub-agents not tried).
- A trailing `system/task_summary {"detail":null}` arrives after `result`.

Interrupt (interrupt fixture; first prompt of the round failed to catch the stream: haiku finished 200 numbers before the interrupt, and that run is in `-deny.jsonl`, where the control_response is just `{"still_queued":[]}` after the turn). Second attempt, interrupt 0.4s after the first delta (still in the thinking block, before any text):
- Reply: `control_response` success `{"still_queued":[]}`, then `user` line `{"type":"user","message":{"role":"user","content":[{"type":"text","text":"[Request interrupted by user]"}]},"parent_tool_use_id":null,...}`, the stream closes (`content_block_stop`, `message_delta` with `stop_reason:null`, `message_stop`), then ONE `result`:
  `{"type":"result","subtype":"error_during_execution","is_error":true,"stop_reason":null,"terminal_reason":"aborted_streaming","num_turns":2,"total_cost_usd":0,"usage":{all zero},"modelUsage":{}}`.
  Note `is_error:true` with zero usage/cost; Canopy should map `terminal_reason:"aborted_streaming"` to "interrupted", not to an error. Interrupt during text streaming and during tool execution not observed.
- Process stays alive: a following stdin message ran a normal turn (a plain `/help` gave its local `result`, subtype success). No second interrupt `result`; one interrupt = one `result`.

Deny (deny fixture). Sent `{"type":"control_response","response":{"subtype":"success","request_id":"<id>","response":{"behavior":"deny","message":"denied by canopy probe"}}}` (message text differs from the one requested; shape identical, no `interrupt` field).
- Model sees a `user` tool_result: `{"type":"tool_result","content":"denied by canopy probe","is_error":true,"tool_use_id":"toolu_.."}`, with `tool_use_result:"Error: denied by canopy probe"` and `tool_result_meta:[{"id","non_execution_kind":"permission-rule","permission_decision":{"decision":"reject","source":"user_reject"}}]`. The file was not written. The model quoted the message back verbatim and did not retry.
- The turn continues normally and ends with `result` subtype `success`, `is_error:false`, `stop_reason:"end_turn"`, `permission_denials:[]` (the denial is NOT listed there). `interrupt:true` on deny not tried.

## Contradictions with PLAN.md section 4

- PLAN omits `--permission-prompt-tool stdio`; it is required to get `can_use_tool` on stdout.
  Without it permissions are decided by the CLI itself.
- `--mcp-config <shim>` and `--forward-subagent-text` in the PLAN line are valid flags.
- "Mid-turn it is queued into the running loop": true, but it is queued as a SEPARATE turn
  with its own `system/init` and `result` (not folded into the running turn).
- "`result` message carries token usage and total_cost_usd": true; cost is cumulative per process.
- Slash trap is real and the escape is `client_composed:true`, not a text prefix.
- `system/init` is per turn, and not emitted until the first user message.
- Thinking text is empty in `thinking_delta`; only `estimated_tokens` and a signature arrive.
- Rate-limit events are `rate_limit_event` lines, emitted frequently, not only on change.
- `--permission-mode` flag value `manual` is `default` on the wire.

## Gaps

- Interrupt during text streaming or while a tool/can_use_tool is pending (and whether a
  `control_cancel_request` is sent) not observed; only interrupt-during-thinking was.
- Omitted `updatedInput` and deny with `interrupt:true` not exercised.
- `--resume` / `--fork-session` not run; `--session-id` reuse after exit untested.
- Sub-agents: only a foreground one; background sub-agents, nested sub-agents, sub-agent permission asks (`agent_id` on can_use_tool) not observed.
- Result with `is_error:true`, `rate_limit_event` with `allowed_warning|rejected`, and the logged-out
  `claude auth status` not observed.
- `MCP_TOOL_TIMEOUT` effect (timeout firing) not verified; `--mcp-config <file>` not run.
- `priority:"now"` (steer-style send) not tried.
- `result` timing with background sub-agents not tested.
- All probes ran with the user's real settings and plugins loaded (hooks emit extra
  `system` lines); a clean `CLAUDE_CONFIG_DIR` was not used because it would require
  logging in.

UNVERIFIED: whether the model sees MCP initialize instructions. Keep the MCP instructions
and child prompt line; no live prompt was authorized for M3.5.

## Extension control (M5.5)

UNVERIFIED: `claude plugin list --json` response shape and
`claude plugin enable|disable ID --scope user|project --json` behavior on 2.1.318.
No real CLI was run for this milestone. The conservative synthetic fixture in
`crates/canopy/tests/ext_fake.py` models an array (or `plugins` array) containing
`id`, `scope` (`user` or `project`), and boolean `enabled`. Unknown entries are
ignored. Missing methods and failed commands surface as view errors. A declared
`command` is displayed, blocks toggling, and is never run by Canopy.
UNVERIFIED: Claude loading symlinked skill directories and one unchanged SKILL.md
in Claude and Codex. This requires an explicitly authorized live prompt. Library
link operations are filesystem-tested only. Non-library Claude skills are
read-only until explicitly adopted. Adoption moves the whole vendor skill folder
into the library and replaces it with a symlink; cross-filesystem moves fail safely.

UNVERIFIED (M6): --resume is documented by CLI help; interactive resume after a daemon crash has not been live tested. Canopy passes only the saved session id and waits for a new user message.

## Outside sessions (2026-11-11, Claude Code 2.1.318)

Orchestrator-verified: `claude agents --json [--cwd <path>]` exits with a JSON
array of active interactive and background sessions, without a TTY, in about
0.2-0.4 seconds. `--all` also includes completed background sessions and is not used.
Rows contain pid, cwd, kind, startedAt (milliseconds), sessionId, name, and
status (idle or busy). Observed kind: interactive. The scrubbed shape is recorded
in `fixtures/claude-2.1.318-agents.json`.

Discovery uses a three second timeout and is read-only (ADR 0026).

Attaching, verified from `claude --help` (2.1.318): `claude attach <id|name>` opens
a background session (started with `--bg`) as a TUI in a terminal; there is no
way for another program to drive an interactive session. `--bg --resume <id>` on a
running session "starts a copy and says so". `--fork-session` resumes under a new id.
No transcript or credential files are read.
