# Codex app-server - codex-cli 0.165.0

Verified 2026-11-11 against the installed `codex-cli 0.165.0`. Every protocol fact below is from its generated JSON Schema (S) or an observed probe (O). All claims are version-specific. JSON examples use placeholder strings; explicitly labelled projections omit unrelated fields.

Schema command: `codex app-server generate-json-schema --out <scratch>/schema`. Generation used the installed CLI; no web documentation or remembered protocol behavior was used. Schema references below are filenames relative to that generated directory, including their `definitions`.

Probes ran in `/private/tmp/claude-501/-Users-krishna-projects-canopy/eb61a289-64fd-40df-9c8d-0e09721fafba/scratchpad/codex-probe`, using an isolated `CODEX_HOME=<scratch>/home`, `log_dir=<scratch>/logs`, and `sqlite_home=<scratch>/state`. Starting app-server with the existing home failed with `Operation not permitted`; credentials were not copied. Two live `turn/start` attempts used `gpt-6-luna`, `effort:"low"`, `summary:"none"`, each bounded by a 50-second process deadline. Both completed as failed with HTTP 401, not a model response. No third live turn was used. All started app-server processes were reaped; the local MCP helper exited.

## Framing and handshake (0.165.0)

S: `JSONRPCRequest.json`, `JSONRPCNotification.json`, `JSONRPCResponse.json`, `JSONRPCError.json`, `ClientRequest.json`, `ClientNotification.json`. O: `codex-0.165.0-handshake.jsonl`, `codex-0.165.0-control.jsonl`.

Stdio is newline-delimited JSON, without Content-Length headers. Server messages omit `jsonrpc`; requests with `"jsonrpc":"2.0"` were accepted for both `initialize` and `model/list`. Request IDs are strings or integers. Basic envelopes:

```json
{"id":1,"method":"initialize","params":{"clientInfo":{"name":"canopy","version":"0.1.0"}}}
{"id":1,"result":{"userAgent":"<string>","codexHome":"<absolute path>","platformFamily":"unix","platformOs":"macos"}}
{"method":"initialized"}
{"id":2,"method":"thread/start","params":{"cwd":"<absolute path>"}}
{"id":2,"error":{"code":-32600,"message":"Not initialized"}}
```

The last line is the observed response if `thread/start` is sent **before** initialization. `initialize` is required once per connection; duplicate initialization returns `{"error":{"code":-32600,"message":"Already initialized"},"id":4}`. After its successful response, both `thread/start` and `model/list` succeeded **without** an intervening `initialized`. The notification exists in the schema and was sent in the turn fixtures, but these methods do not require it in this version. These are separate examples, not one transcript.

Optional initialize params: `clientInfo.title` is string/null; `capabilities` is object/null, including `experimentalApi` (boolean, default false) and `optOutNotificationMethods` (string array/null). No experimental opt-in was used. Live notifications also contain top-level `emittedAtMs` (integer); this field is absent from the generated generic notification schema. Accept unknown fields.

## Threads and per-thread MCP (0.165.0)

S: `v2/ThreadStartParams.json`, `v2/ThreadStartResponse.json`, `v2/ThreadResumeParams.json`, `v2/ThreadResumeResponse.json`. O: `codex-0.165.0-mcp-timeout.jsonl`, `codex-0.165.0-threads.jsonl`, both turn-attempt fixtures.

All `thread/start` params are optional. Relevant exact spelling:

```json
{"id":2,"method":"thread/start","params":{"cwd":"<absolute path>","model":"gpt-6-luna","approvalPolicy":"untrusted","approvalsReviewer":"user","sandbox":"read-only","config":{"mcp_servers.probe.command":"<absolute executable path>","mcp_servers.probe.args":["<script path>"],"mcp_servers.probe.tool_timeout_sec":1}}}
```

`cwd` and `model` are string/null. `config` is an arbitrary object/null. `approvalPolicy` is null or one of `"untrusted"`, `"on-request"`, `"never"`, or the following object. The schema does **not** list `"on-failure"`:

```json
{"granular":{"mcp_elicitations":true,"rules":true,"sandbox_approval":true,"request_permissions":false,"skill_approval":false}}
```

The first three granular booleans are required; the last two default false. `approvalsReviewer` can be `"user"`, `"auto_review"`, or legacy `"guardian_subagent"` (or null in params). `sandbox` is null or `"read-only"`, `"workspace-write"`, `"danger-full-access"`. These are thread-start strings; the returned sandbox and `turn/start.sandboxPolicy` are tagged objects, not those strings:

```json
{"type":"readOnly","networkAccess":false}
{"type":"workspaceWrite","writableRoots":["<absolute path>"],"networkAccess":false,"excludeSlashTmp":false,"excludeTmpdirEnvVar":false}
{"type":"dangerFullAccess"}
{"type":"externalSandbox","networkAccess":"restricted"}
```

For `externalSandbox`, `networkAccess` is `"restricted"` or `"enabled"`; the other examples' access fields are boolean. Only `type` is required in each sandbox object; other displayed fields have schema defaults.

Response projection, with the required top-level fields:

```json
{"id":2,"result":{"thread":{"id":"<thread id>"},"model":"gpt-6-luna","modelProvider":"openai","cwd":"<absolute path>","approvalPolicy":"untrusted","approvalsReviewer":"user","sandbox":{"type":"readOnly","networkAccess":false}}}
```

The ID is **`result.thread.id`**. Thread objects also include `sessionId`, `parentThreadId`, `source`, `status`, `ephemeral`, `turns`, and metadata; full observed objects are in the fixtures. Schema says Codex-generated thread IDs are UUIDv7. The response may additionally contain `reasoningEffort`, `serviceTier`, `disabledPluginIds`, `instructionSources`; live responses also included fields missing from the nonexperimental schema, such as `runtimeWorkspaceRoots` and `activePermissionProfile`.

O: The dotted MCP override keys above started a local Python stdio MCP server with the supplied script argument. `mcpServerStatus/list` with `{"threadId":"<thread id>"}` returned `data[0].name:"probe"`, `runtimeStatus:"connected"`, and its advertised `delay` tool. A direct, non-model call:

```json
{"id":4,"method":"mcpServer/tool/call","params":{"threadId":"<thread id>","server":"probe","tool":"delay","arguments":{}}}
{"error":{"code":-32603,"message":"tool call failed for `probe/delay`: timed out awaiting tools/call after 1000ms"},"id":4}
```

The helper deliberately waited three seconds. This verifies the command, args, and per-server timeout override without spending a model turn. One app-server accepted two distinct thread starts. S: `v2/ListMcpServerStatusParams.json`, `v2/McpServerToolCallParams.json`.

Resume uses `threadId`, not `id` or a CLI resume flag:

```json
{"id":4,"method":"thread/resume","params":{"threadId":"<thread id>"}}
```

Its result has the same required top-level fields as `thread/start`, plus optional history cursors and collaboration mode. Relevant optional resume overrides: `cwd`, `model`, `approvalPolicy`, `approvalsReviewer`, `sandbox`, `config`; `excludeTurns:true` avoids full turn hydration. O: resuming each persisted failed-turn thread returned its original ID. Resuming the ephemeral thread failed with `-32600`, `"no rollout found for thread id <id>"`. S: `ephemeral:true` means the thread should not be materialized on disk. The default generated resume schema describes history/path alternatives in prose but does not expose those params; only the `threadId` route was tested.

## Turn control (0.165.0)

S: `v2/TurnStartParams.json`, `v2/TurnStartResponse.json`, `v2/TurnSteerParams.json`, `v2/TurnSteerResponse.json`, `v2/TurnInterruptParams.json`, `v2/TurnInterruptResponse.json`. O: start/response/completion in both attempt fixtures. Steering and interruption are schema-only.

```json
{"id":3,"method":"turn/start","params":{"threadId":"<thread id>","input":[{"type":"text","text":"reply with the word ok"}]}}
{"id":3,"result":{"turn":{"id":"<turn id>","items":[],"status":"inProgress"}}}
{"id":5,"method":"turn/steer","params":{"threadId":"<thread id>","expectedTurnId":"<active turn id>","input":[{"type":"text","text":"<additional input>"}]}}
{"id":5,"result":{"turnId":"<turn id>"}}
{"id":6,"method":"turn/interrupt","params":{"threadId":"<thread id>","turnId":"<turn id>"}}
{"id":6,"result":{}}
```

The turn/start response above projects the turn's required fields. Text input's optional field is **`text_elements`**, default `[]`, not camelCase. `threadId` and `input` are required for start. Steer additionally requires **`expectedTurnId`**, whose mismatch fails the request according to its schema description. Interrupt requires both IDs. Start/steer can optionally carry `clientUserMessageId` (string/null). Input also supports image, localImage, audio, localAudio, skill, and mention tagged variants; these were not probed.

Start overrides include `cwd`, `model`, `approvalPolicy`, `approvalsReviewer`, `sandboxPolicy`, `effort`, `summary`, and `serviceTier`, with effects on subsequent turns documented in the schema. Schema descriptions explicitly allow `turn/start` to add input to an active turn. `CodexErrorInfo.activeTurnNotSteerable` covers active turns that cannot accept steering, such as review/compact. Do not assume start always creates a new turn. `parentTurnId` and `rootTurnId` are optional string/null attribution fields for newly started delegated work.

## Notifications and items (0.165.0)

S: `v2/TurnStartedNotification.json`, `v2/TurnCompletedNotification.json`, `v2/ItemStartedNotification.json`, `v2/ItemCompletedNotification.json`, `v2/AgentMessageDeltaNotification.json`, and `ThreadItem`/`Turn` definitions. O: lifecycle envelopes and userMessage items in both attempt fixtures. The five requested agent/tool variants below are schema-only.

```json
{"method":"turn/started","params":{"threadId":"<thread id>","turn":{"id":"<turn id>","items":[],"status":"inProgress"}}}
{"method":"turn/completed","params":{"threadId":"<thread id>","turn":{"id":"<turn id>","items":[],"status":"completed"}}}
{"method":"item/started","params":{"threadId":"<thread id>","turnId":"<turn id>","startedAtMs":0,"item":{"type":"agentMessage","id":"<item id>","text":""}}}
{"method":"item/completed","params":{"threadId":"<thread id>","turnId":"<turn id>","completedAtMs":0,"item":{"type":"agentMessage","id":"<item id>","text":"ok"}}}
{"method":"item/agentMessage/delta","params":{"threadId":"<thread id>","turnId":"<turn id>","itemId":"<item id>","delta":"ok"}}
```

Turn objects above project required fields. `status` is `"inProgress"`, `"completed"`, `"interrupted"`, or `"failed"`. Optional fields include `error` (TurnError/null), `rootTurnId`, `startedAt`, `completedAt`, `durationMs`, and `itemsView` (`"notLoaded"`, `"summary"`, `"full"`, default full). Start/completion timestamps in Turn are seconds; item timestamps are milliseconds. O: failed completion had `items:[]`, `itemsView:"notLoaded"`, even though userMessage item events had already been emitted. An empty array here does not prove there were no items.

Required-field item shapes, usable in both lifecycle envelopes:

```json
{"type":"agentMessage","id":"<item id>","text":"<text>"}
{"type":"reasoning","id":"<item id>"}
{"type":"commandExecution","id":"<item id>","command":"<command>","cwd":"<path>","commandActions":[],"status":"inProgress"}
{"type":"fileChange","id":"<item id>","changes":[{"path":"<path>","kind":{"type":"add"},"diff":"<diff>"}],"status":"inProgress"}
{"type":"mcpToolCall","id":"<item id>","server":"probe","tool":"delay","arguments":{},"status":"inProgress"}
```

Optional item fields:

- `agentMessage`: `phase` (`"commentary"`, `"partial_answer"`, `"final_answer"`, or null), `delivery` (`"async"`/null), `memoryCitation`, `questions`. Do not infer turn completion from phase.
- `reasoning`: `summary` and `content` are string arrays, default `[]`.
- `commandExecution`: `aggregatedOutput` string/null, `exitCode` integer/null, `durationMs` integer/null, `processId` string/null, `source` default `"agent"`, `pluginId` and `scriptPath` string/null. Status also allows completed, failed, declined.
- `fileChange`: status also allows completed, failed, declined. Change kinds are `{"type":"add"}`, `{"type":"delete"}`, `{"type":"update","move_path":null}`; `move_path` is optional string/null.
- `mcpToolCall`: status also allows completed, failed; `durationMs` integer/null; `result` null or `{"content":[],"structuredContent":{},"_meta":{}}` (only content required); `error` null or `{"message":"<text>"}`. Additional optional app/plugin presentation fields exist.

Reasoning delta params are `{threadId,turnId,itemId,delta,summaryIndex}` for `item/reasoning/summaryTextDelta`, `{threadId,turnId,itemId,delta,contentIndex}` for `item/reasoning/textDelta`, and `{threadId,turnId,itemId,summaryIndex}` for `item/reasoning/summaryPartAdded`. IDs/delta are strings; indices are integers. S: matching `v2/Reasoning*Notification.json` files.

## Usage, rate limits, and errors (0.165.0)

S: `v2/ThreadTokenUsageUpdatedNotification.json`, `TokenUsageBreakdown`, `v2/AccountRateLimitsUpdatedNotification.json`, `RateLimitSnapshot`, `RateLimitWindow`, `v2/ErrorNotification.json`, `TurnError`, `CodexErrorInfo`, `JSONRPCError.json`. Usage/rate-limit examples are schema-only.

```json
{"method":"thread/tokenUsage/updated","params":{"threadId":"<thread id>","turnId":"<turn id>","tokenUsage":{"last":{"inputTokens":0,"cachedInputTokens":0,"outputTokens":0,"reasoningOutputTokens":0,"totalTokens":0},"total":{"inputTokens":0,"cachedInputTokens":0,"outputTokens":0,"reasoningOutputTokens":0,"totalTokens":0},"modelContextWindow":null}}}
{"method":"account/rateLimits/updated","params":{"rateLimits":{"limitId":null,"limitName":null,"normalModelSlug":null,"primary":{"usedPercent":0,"windowDurationMins":null,"resetsAt":null},"secondary":null,"credits":null,"planType":null,"rateLimitReachedType":null,"spendControlReached":null,"individualLimit":null}}}
{"method":"error","params":{"threadId":"<thread id>","turnId":"<turn id>","willRetry":false,"error":{"message":"<text>","codexErrorInfo":{"httpConnectionFailed":{"httpStatusCode":401}},"additionalDetails":null,"misalignment":null}}}
{"id":7,"error":{"code":-32603,"message":"<text>","data":{}}}
```

Usage requires `last` and `total` with all five displayed counts; optional `cacheWriteInputTokens` defaults 0 in each breakdown. `modelContextWindow` is optional integer/null. This usage shape has no dollar cost field.

Rate updates require only the `rateLimits` object; all displayed snapshot fields are optional/nullable. Each non-null window requires integer `usedPercent`; `windowDurationMins` and **`resetsAt`** are optional integer/null. Thus a reset value is exposed but not guaranteed. The generated window schema does not specify its time unit. `individualLimit` can contain `{"limit":"<string>","used":"<string>","remainingPercent":0,"resetsAt":0}` (all four required). `credits` can contain `{"hasCredits":true,"unlimited":false,"balance":null}` (balance optional string/null).

The rate notification is explicitly a **sparse rolling update**: merge available values into the latest `account/rateLimits/read` snapshot or refetch it; null account metadata does not clear a previously observed value. `rateLimitReachedType` values: `rate_limit_reached`, `workspace_owner_credits_depleted`, `workspace_member_credits_depleted`, `workspace_owner_usage_limit_reached`, `workspace_member_usage_limit_reached`. `spendControlReached:null` means unavailable, not recovery.

O: error notifications included retries with `willRetry:true`, `codexErrorInfo:{"responseStreamDisconnected":{"httpStatusCode":401}}`, followed by a final `willRetry:false` error and failed turn completion. `TurnError.message` is required; `codexErrorInfo`, `additionalDetails`, `misalignment` are optional/nullable. Schema error-info string variants include `usageLimitExceeded`, `rateLimitExceeded`, `serverOverloaded`, `unauthorized`, and `sandboxError`; unknown string/object variants are also allowed. RPC errors are responses correlated by `id`, unlike the turn-scoped `error` notification; their `data` is optional arbitrary JSON.

## Approval requests and responses (0.165.0)

S only: `CommandExecutionRequestApprovalParams.json`, `FileChangeRequestApprovalParams.json`, `CommandExecutionRequestApprovalResponse.json`, `FileChangeRequestApprovalResponse.json`, their decision definitions, and `ServerRequest.json`. No live approval request was reached.

Minimal required server requests:

```json
{"id":10,"method":"item/commandExecution/requestApproval","params":{"threadId":"<thread id>","turnId":"<turn id>","itemId":"<item id>","startedAtMs":0}}
{"id":11,"method":"item/fileChange/requestApproval","params":{"threadId":"<thread id>","turnId":"<turn id>","itemId":"<item id>","startedAtMs":0}}
```

Additional command params: `approvalId`, `command`, `cwd`, `environmentId`, `reason` are optional string/null; `kind` is `"command"` or `"writeStdin"` (default command); `commandActions` is array/null; `proposedExecpolicyAmendment` is string array/null; `networkApprovalContext` is null or `{"host":"<host>","protocol":"https"}` (protocol: http, https, socks5Tcp, socks5Udp); `proposedNetworkPolicyAmendments` is null or an array of `{"host":"<host>","action":"allow"}` (action: allow/deny). An `approvalId` can distinguish multiple callbacks for one item; reply using the outer RPC `id`.

Additional file params: optional `reason` string/null and `grantRoot` string/null. The schema labels grantRoot unstable and explicitly says whether it is honored is unclear. File paths/diffs are in fileChange items, not required approval params.

For **either** request, exact responses, reusing its RPC ID:

```json
{"id":10,"result":{"decision":"accept"}}
{"id":10,"result":{"decision":"decline"}}
{"id":10,"result":{"decision":"acceptForSession"}}
{"id":10,"result":{"decision":"cancel"}}
```

`accept` allows once; `decline` denies but lets the turn continue; `cancel` denies and immediately interrupts the turn. `acceptForSession` uses the session approval cache for commands; for file changes it allows future changes to the same files. These are schema semantics, not live-tested persistence guarantees. Never substitute boolean `true`, `"allow"`, or a bare decision string for the result object.

Command decisions additionally support `{"acceptWithExecpolicyAmendment":{"execpolicy_amendment":["<prefix>"]}}` and `{"applyNetworkPolicyAmendment":{"network_policy_amendment":{"host":"<host>","action":"allow"}}}` as the value of `result.decision`. File decisions do not list these objects.

## Native parentage (0.165.0)

S: `Thread`, `SessionSource`, `SubAgentSource`, and `ThreadItem` definitions. No model sub-agent was spawned.

Thread metadata exposes optional `parentThreadId` (string/null, only set for sub-agents), required `sessionId` (shared by the session tree), optional `agentNickname`/`agentRole`, and optional `forkedFromId` (fork origin, distinct from parentage). A sub-agent source can be:

```json
{"subAgent":{"thread_spawn":{"parent_thread_id":"<parent thread id>","depth":1,"agent_nickname":null,"agent_role":null,"agent_path":null}}}
```

The source uses **snake_case** inside `thread_spawn`, while Thread uses camelCase. Required collab item fields:

```json
{"type":"collabAgentToolCall","id":"<item id>","tool":"spawnAgent","status":"inProgress","senderThreadId":"<parent thread id>","receiverThreadIds":["<child thread id>"],"agentsStates":{}}
{"type":"subAgentActivity","id":"<item id>","agentThreadId":"<child thread id>","agentPath":"<path>","kind":"started"}
```

Collab items may include `prompt`, `model`, `reasoningEffort`. Tool enum: spawnAgent, sendInput, resumeAgent, wait, closeAgent, sendMessage, followupTask, interruptAgent, listAgents. Collab status: inProgress, completed, failed, interrupted. Agent-state map values require `status` (pendingInit, running, interrupted, completed, errored, shutdown, notFound) and optionally `message`. Activity kind: started, interacted, interrupted, completed. These fields answer PLAN section 11's parentage question at the schema level.

## Doctor and PLAN implications (0.165.0)

O: bounded `codex login status` exited **0** and printed `Logged in using ChatGPT` on the existing login; with isolated `CODEX_HOME` it exited **1** and printed `Not logged in`. This invokes status, not a login flow, and did not require a model turn or credential inspection. Existing-home runs also printed a warning that creating PATH aliases was denied; do not promise the entire CLI startup is free of attempted filesystem writes. Other auth modes were not tested.

PLAN section 4's omitted-jsonrpc framing works, but the omission is not mandatory on incoming requests. It needs initialization, exact camelCase IDs, and the distinction between sandbox strings and sandboxPolicy objects. Steering, interrupt, approvals, usage, rate limits, per-thread MCP and parentage all have schema support; only the observations identified above have live confirmation. The generated schema does **not** mention `-32001`, queue-full errors, or a retry rule: PLAN's assertion that `-32001` means queue full remains unverified, not an established fact or a disproven claim.

## Recorded fixtures (0.165.0)

Each server stdout line is retained verbatim. Sent lines are represented as `{"canopy_stdin":"<exact JSON line without newline>"}`. Ordering is the probe's stdin writes and stdout line observations; stderr is not protocol data. No synthetic server responses or redactions were inserted.

- `codex-0.165.0-control.jsonl`: pre-init rejection, initialize, model/list with jsonrpc and before initialized, model catalog.
- `codex-0.165.0-handshake.jsonl`: pre-init thread/start rejection, initialize with jsonrpc, successful thread/start before initialized, duplicate-init rejection.
- `codex-0.165.0-threads.jsonl`: thread-scoped stdio MCP discovery; ephemeral resume failure.
- `codex-0.165.0-mcp-timeout.jsonl`: MCP command/args discovery, configured one-second tool timeout, two threads in one process.
- `codex-0.165.0-ok-attempt.jsonl`: initialize + thread/start + requested ok prompt through **failed** completion, then successful resume.
- `codex-0.165.0-approval-attempt.jsonl`: untrusted/read-only file-creation prompt through **failed** completion, then successful resume. Contains **no approval request or accepting response**.

## Live check through canopy (0.165.0, 2026-11-11)

One live run of `canopy run --policy auto_edit claude` whose Claude parent spawned a Codex
child (`gpt-6-luna`) through `spawn_agent`. Observed working end to end through `codex.rs`:
`initialize`, `thread/start` with `approvalPolicy:"on-request"`, `sandbox:"workspace-write"`
and the dotted `mcp_servers.canopy.*` config, `turn/start`, `item/agentMessage/delta`,
`item/started` / `item/completed` for `agentMessage` and `commandExecution`, `turn/completed`,
and `thread/tokenUsage/updated` totals. The child wrote the file with a shell command, so no
`fileChange` item and no approval request appeared. No raw fixture was recorded from this run.

## Gaps

- The requested successful ok and accepted file-creation fixtures could not be recorded: isolated auth was absent and both attempts returned 401. Existing-home app-server startup was denied. Two live turns were attempted, zero succeeded. No credentials/configs were copied or edited to bypass this.
- `model/list` advertised gpt-6-luna as fast and affordable; it exposes no numeric pricing. The cheapest model and authenticated availability could not be established from schema/probes. gpt-6-luna was selected on that advertised description.
- Raw fixture of a successful turn (the live check above went through canopy, not a recorder).
- Live steering, interruption, command/file approval exchange, denial/session reuse, agentMessage/reasoning/tool lifecycle events, token counts, account rate updates, reset-time units, real quota exhaustion, and native sub-agent notification delivery remain unverified. Schema examples must not be treated as recorded events.
- No `-32001` event was observed; queue saturation and jittered retry behavior were not tested. Nested MCP config objects and timeout defaults were not tested; dotted keys and an explicit one-second timeout were verified.
- Login status was tested only for ChatGPT login and an empty home. Refresh behavior and startup side effects outside this restricted filesystem were not audited.
- No Rust or adapter tests were written. `make check test` was attempted and failed with `No rule to make target check`; the checkout had no Makefile.
