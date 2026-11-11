# ACP (Agent Client Protocol)

Source: agentclientprotocol.com, protocol v1 (`protocolVersion: 1`), read 2026-11-11.
Verified against: canopy `fake-agent` only. Gemini CLI and OpenCode are not installed on the
development machine, so their ACP behaviour is unverified (see Gaps).

## Framing

JSON-RPC 2.0 with `"jsonrpc":"2.0"`, one object per line over the agent's stdin/stdout.
Notifications have no `id`.

## Calls canopy makes (client -> agent)

- `initialize` `{protocolVersion: 1, clientCapabilities: {fs:{readTextFile:false,writeTextFile:false}, terminal:false}, clientInfo:{name,version}}`
  -> `{protocolVersion, agentCapabilities:{loadSession, ...}, authMethods:[]}`
- `session/new` `{cwd, mcpServers:[{name, command, args:[...], env:[{name,value}]}]}` -> `{sessionId}`.
  A stdio MCP entry has no `type` field and `env` is an array, not a map.
- `session/load` `{sessionId, cwd, mcpServers}` (only if `agentCapabilities.loadSession`); the
  agent replays history as `session/update` notifications, then returns `{}`.
- `session/prompt` `{sessionId, prompt:[{type:"text", text}]}` -> `{stopReason}` where
  stopReason is `end_turn | max_tokens | max_turn_requests | refusal | cancelled`.
- `session/cancel` notification `{sessionId}`. The agent must answer the pending prompt with
  `stopReason: "cancelled"`; the client must answer pending permission requests with
  `{outcome:{outcome:"cancelled"}}`.

## Calls the agent makes (agent -> client)

- `session/update` notification `{sessionId, update:{sessionUpdate: <variant>, ...}}`. Variants
  canopy maps: `agent_message_chunk` / `agent_thought_chunk` `{content: ContentBlock, messageId?}`,
  `tool_call` `{toolCallId, title, kind, status, content?, locations?, rawInput?}`,
  `tool_call_update` `{toolCallId, ...changed fields}`, `usage_update` `{used, size, cost?:{amount,currency}}`.
  Others (`plan`, `available_commands_update`, `current_mode_update`, ...) are counted as unknown.
- `session/request_permission` request `{sessionId, toolCall, options:[{optionId, name, kind}]}`
  with kind `allow_once | allow_always | reject_once | reject_always`. Response:
  `{outcome:{outcome:"selected", optionId}}` or `{outcome:{outcome:"cancelled"}}`.
- Tool call content items: `{type:"content",content}`, `{type:"diff",path,oldText?,newText}`,
  `{type:"terminal",terminalId}`. Tool status: `pending | in_progress | completed | failed`.

## Facts that differ from PLAN.md

- Token usage is `usage_update.used` (tokens in context), not an input/output split. Canopy
  shows it as input tokens.
- There is no rate-limit signal in ACP. An agent may surface one only as an error or a notice.

## Gaps

- No real ACP agent verified: `gemini --acp` vs `--experimental-acp`, and whether OpenCode
  reports `usage_update`, remain open until those CLIs are installed.
