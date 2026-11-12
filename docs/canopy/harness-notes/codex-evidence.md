# codex installed evidence
Executable: `/Users/krishna/.codex/packages/standalone/current/codex`
SHA256: `ebabe5f0d79145a9e61274ce9ecf7c02c9021b7b04bfa59b7e61ad4461c32b0b`
Collected 2026-11-12. Display normalization: em dashes and escaped Unicode em dashes replaced with plain dashes; control bytes rendered as escapes. Native binary excerpts below are evidence of embedded text/source, not observed runtime behavior. Byte offsets are zero-based. Reproduce with Python Path(path).read_bytes()[offset:offset+length].decode("utf-8", "replace").

## Command: codex --version
Exit: 0
```text
codex-cli 0.167.1

```
stderr:
WARNING: proceeding, even though we could not create PATH aliases: Operation not permitted (os error 1)


## Command: codex --help
Exit: 0
```text
Codex CLI

If no subcommand is specified, options will be forwarded to the interactive CLI.

Usage: codex [OPTIONS] [PROMPT]
       codex [OPTIONS] <COMMAND> [ARGS]

Commands:
  agents            Open the agent command center to browse and switch sessions
  exec              Run Codex non-interactively [aliases: e]
  review            Run a code review non-interactively
  login             Manage login
  logout            Remove stored authentication credentials
  mcp               Manage external MCP servers for Codex
  plugin            Manage Codex plugins
  app-server        [experimental] Run the app server or related tooling
  remote-control    [experimental] Manage the app-server daemon with remote control enabled
  app               Launch the Desktop app (opens the app installer if missing)
  completion        Generate shell completion scripts
  update            Update Codex to the latest version
  doctor            Diagnose local Codex installation, config, auth, and runtime health
  sandbox           Run commands within a Codex-provided sandbox
  debug             Debugging tools
  apply             Apply the latest diff produced by Codex agent as a `git apply` to your local
                    working tree [aliases: a]
  resume            Resume a previous interactive session (picker by default; use --last to continue
                    the most recent)
  queue             Queue a message for an existing session
  archive           Archive a saved session by id or session name
  delete            Permanently delete a saved session by id or session name
  migrate-rollouts  Inspect or migrate legacy local sessions to paginated thread history
  migrate-nfs-db    [experimental] Migrate configured SQLite databases to DELETE mode in place
  unarchive         Unarchive a saved session by id or session name
  fork              Fork a previous interactive session (picker by default; use --last to fork the
                    most recent)
  cloud             [EXPERIMENTAL] Browse tasks from Codex Cloud and apply changes locally
  exec-server       [EXPERIMENTAL] Run the standalone exec-server service
  features          Inspect feature flags
  help              Print this message or the help of the given subcommand(s)

Arguments:
  [PROMPT]
          Optional user prompt to start the session

Options:
  -c, --config <key=value>
          Override a configuration value that would otherwise be loaded from `~/.codex/config.toml`.
          Use a dotted path (`foo.bar.baz`) to override nested values. The `value` portion is parsed
          as TOML. If it fails to parse as TOML, the raw string is used as a literal.
          
          Examples: - `-c model="o3"` - `-c 'sandbox_permissions=["disk-full-read-access"]'` - `-c
          shell_environment_policy.inherit=all`

      --enable <FEATURE>
          Enable a feature (repeatable). Equivalent to `-c features.<name>=true`

      --disable <FEATURE>
          Disable a feature (repeatable). Equivalent to `-c features.<name>=false`

      --remote <ADDR>
          Connect the TUI to a remote app server endpoint.
          
          Accepted forms: `ws://host:port`, `wss://host:port`, `unix://`, or `unix://PATH`.

      --remote-auth-token-env <ENV_VAR>
          Name of the environment variable containing the bearer token to send to a remote app
          server websocket

      --strict-config
          Error out when config.toml contains fields that are not recognized by this version of
          Codex

  -i, --image <FILE>...
          Optional image(s) to attach to the initial prompt

  -m, --model <MODEL>
          Model the agent should use

      --oss
          Use open-source provider

      --local-provider <OSS_PROVIDER>
          Specify which local provider to use (lmstudio or ollama). If not specified with --oss,
          will use config default or show selection

  -p, --profile <CONFIG_PROFILE_V2>
          Layer $CODEX_HOME/<name>.config.toml on top of the base user config

  -s, --sandbox <SANDBOX_MODE>
          Select the sandbox policy to use when executing model-generated shell commands
          
          [possible values: read-only, workspace-write, danger-full-access]

      --approve-for-me
          Route approval requests through automatic review using the workspace-write sandbox

      --dangerously-bypass-approvals-and-sandbox
          Skip all confirmation prompts and execute commands without sandboxing. EXTREMELY
          DANGEROUS. Intended solely for running in environments that are externally sandboxed

      --dangerously-bypass-hook-trust
          Run enabled hooks without requiring persisted hook trust for this invocation. DANGEROUS.
          Intended only for automation that already vets hook sources

  -C, --cd <DIR>
          Tell the agent to use the specified directory as its working root

      --worktree
          Run the session in a new managed Git worktree

      --add-dir <DIR>
          Additional directories that should be writable alongside the primary workspace

  -a, --ask-for-approval <APPROVAL_POLICY>
          Configure when the model requires human approval before executing a command

          Possible values:
          - on-request: The model decides when to ask the user for approval
          - never:      Never ask for user approval Execution failures are immediately returned to
            the model

      --search
          Enable live web search. When enabled, the native Responses `web_search` tool is available
          to the model (no per‑call approval)

      --no-alt-screen
          Disable alternate screen mode
          
          Runs the TUI in inline mode, preserving terminal scrollback history.

      --no-daemon
          Run without the shared background server, even if it is already running

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

```
stderr:
WARNING: proceeding, even though we could not create PATH aliases: Operation not permitted (os error 1)


## E1: byte 209246800, length 1250
```text
ot need to make any decisions.

## Mode rules (strict)

You are in **Plan Mode** until a developer message explicitly ends it.

Plan Mode is not changed by user intent, tone, or imperative language. If a user asks for execution while still in Plan Mode, treat it as a request to **plan the execution**, not perform it.

## Plan Mode vs update_plan tool

Plan Mode is a collaboration mode that can involve requesting user input and eventually issuing a `<proposed_plan>` block.

Separately, `update_plan` is a checklist/progress/TODOs tool; it does not enter or exit Plan Mode. Do not confuse it with Plan mode or try to use it while in Plan mode. If you try to use `update_plan` in Plan mode, it will return an error.

## Execution vs. mutation in Plan Mode

You may explore and execute **non-mutating** actions that improve the plan. You must not perform **mutating** actions.

### Allowed (non-mutating, plan-improving)

Actions that gather truth, reduce ambiguity, or validate feasibility without changing repo-tracked state. Examples:

* Reading or searching files, configs, schemas, types, manifests, and docs
* Static analysis, inspection, and repo exploration
* Dry-run style commands when they do not edit repo-tracked files
* Tests, builds, 
```

## E2: byte 209252300, length 1050
```text
ecision complete and leaves no decisions to the implementer.

When you present the official plan, wrap it in a `<proposed_plan>` block so the client can render it specially:

1) The opening tag must be on its own line.
2) Start the plan content on the next line (no text on the same line as the tag).
3) The closing tag must be on its own line.
4) Use Markdown inside the block.
5) Keep the tags exactly as `<proposed_plan>` and `</proposed_plan>` (do not translate or rename them), even if the plan content is in another language.

Example:

<proposed_plan>
plan content
</proposed_plan>

plan content should be human and agent digestible. The final plan must be plan-only, concise by default, and include:

* A clear title
* A brief summary section
* Important changes or additions to public APIs/interfaces/types
* Test cases and scenarios
* Explicit assumptions and defaults chosen where needed

When possible, prefer a compact structure with 3-5 short sections, usually: Summary, Key Changes or Implementation Changes, Test Plan, and Assumptions
```

## E3: byte 206073850, length 650
```text
ckgroundExit Codex and leave the task runningStop the current task and exit CodexCannot open external editor: set $VISUAL or $EDITOR before starting Codex.failed to clear terminal UIconnected app-server platformTUI task delegation is unavailable without its MCP servercodex.status_linePermission overrides are not supported when resuming a remote task.cli_subcommandPermission overrides are not supported when forking a remote task.tui startup initial frame scheduled)Could not restart the background server: � \x00�`\x00terminal input stream closed; shutting down active threadapp-server event stream closedReconnect failed - agent list is stale; relaun
```

## E4: byte 210141750, length 450
```text
lay.Find text in the full transcript.Focus transcript activity groups to expand or collapse their details.Open the current draft in an external editor.Copy the last agent response to the clipboard.Clear the terminal UI.Turn Vim composer mode on or off.Turn Fast mode on or off.Toggle raw scrollback mode.Switch between a side conversation and its parent.Set the prefix used by shared leader shortcuts.Interrupt the active turn.Decrease reasoning effo
```

## E5: byte 208319650, length 850
```text
{{ base_path }}/rollout_summaries/ (per-rollout recaps + evidence snippets)
  - The paths of these entries can be found in {{ base_path }}/MEMORY.md or {{ base_path }}/rollout_summaries/ as `rollout_path`
  - These files are append-only `jsonl`: `session_meta.payload.id` identifies the session, `turn_context` marks turn boundaries, `event_msg` is the lightweight status stream, and `response_item` contains actual messages, tool calls, and tool outputs.
  - For efficient lookup, prefer matching the filename suffix or `session_meta.payload.id`; avoid broad full-content scans unless needed.

Quick memory pass (when applicable):

1. Skim the MEMORY_SUMMARY below and extract task-relevant keywords.
2. Search {{ base_path }}/MEMORY.md using those keywords.
3. Only if MEMORY.md directly points to rollout summaries/skills, open the 1-2
   most rel
```

## E6: byte 205810150, length 600
```text
t_msnum_turnsagent_nicknameagent_rolesummarydetailsGranularApprovalConfigsandbox_approvalskill_approvalmcp_elicitationsverificationsSessionConfiguredEventforked_from_idthread_namemodel_provider_idservice_tierapproval_policyapprovals_reviewerpermission_profileactive_permission_profileruntime_workspace_rootsreasoning_summarypersonalitycollaboration_modedisabled_plugin_idstransparent_backgroundfailureinput_imagesgeneration_idauthorrecipientother_recipientsencrypted_contentinternal_chat_message_metadata_passthroughtrigger_turnreadyreauthentication_requireddelta\x00\x00\x00\x00\x00\x00\x00rolenew_thread_idnew_agent_nic
```

## E7: byte 204493000, length 750
```text
sub-agent for a well-scoped task. �\x01 �\x003
        Spawn a sub-agent for a well-scoped task. �\x01 �\x01
�\x00	
        �\x01
�\x00	
        ��\x10\x02
This spawn_agent tool provides you access to sub-agents that inherit your current model by default. Do not set the `model` field unless the user explicitly asks for a different model. You should follow the rules and guidelines below to use this tool.

Do not spawn sub-agents unless the user or applicable AGENTS.md/skill instructions explicitly ask for sub-agents, delegation, or parallel agent work.
Requests for depth, thoroughness, research, investigation, or detailed codebase analysis do not count as permission to spawn.���\x0c

### When to delegate vs. do the subtask yourself
- First, quickly analyze the overall us
```

## E8: byte 205956950, length 600
```text
 ID not available, please re-run `codex login`Failed to send requestFailed to parse JSON responsesession_metapayloaddynamic_tools_updatedresponse_itemmetadatainter_agent_communicationinter_agent_communication_metadatacompactedturn_contexttoken_usage_recordworld_statesecurity_risk_scoreevent_msgrealtime_itemreplacement_historyreplacement_history_metadatamcp_resource_originswindow_numberfirst_window_idprevious_window_idwindow_idcompaction_response_idlatest_token_usage_recordresume_metadatatrigger_turnturn_idcall_idquestionsverified_answersincompleteuser_messagesuser_messages_incompleteassistant_
```
