# Claude Code: Phase R, no-token research

2026-11-12, installed **2.1.322**. All answers below apply only to that version. [Evidence](claude-code-evidence.md:6) records version/help commands and installed executable hash. E references are embedded-source excerpts with byte offsets, not live observations. Old main notes were consulted, but no old runtime result is inherited. Question 7 is absent from the plan's numbering.

1. **Plan existence - verified; transitions need live run.** `claude --help` lists `--permission-mode plan`. Embedded `ExitPlanMode` result exits planning on approval (E5, evidence:352). Interactive entry/exit shortcuts and permission behavior need a live check; use the flag for the experiment.
2. **Storage - verified path/source; lifecycle needs live run.** `plansDirectory` defaults to `~/.claude/plans/`, may be overridden relative to project root (E1, evidence:332). Path builder uses `<slug>.md`, or `<slug>-agent-<agent-id>.md`; reads use UTF-8 (E2/E3, evidence:337). Slug is session metadata, not the session UUID. Resume can recover a missing plan from snapshots/history in source inspected around byte 266377000. Creation time, every update trigger, deletion policy and behavior with experimental storage modes are not established. Do not find a session's plan by newest mtime.
3. **Editor - verified source; handoff needs live run.** Ctrl+G is named by `planWasEdited` (E4, evidence:347). Resolver prefers VISUAL, then EDITOR, then installed code/vi/nano; host editor may override local resolver. `code` maps to `code -w`, `subl` to `subl --wait`. Editor function checks process status and reads the file after synchronous return (E6, evidence:381). Exact argv/temp path and IDE precedence need the shim experiment.
4. **Awareness - verified source; asynchronous edits need live run.** `planWasEdited` means edit via CCR UI or Ctrl+G; result includes `Approved Plan (edited by user)` and the plan body (E4/E5). The fresh read/cache code exists, but does not establish that any external write immediately notifies the model. Do not promise awareness when Save alone writes the file. Measure editor return, approval, and idle external write separately.
5. **Hooks - verified names/flags; payload and plan timing need live run.** Embedded lifecycle list includes PreToolUse, PostToolUse, PermissionRequest, SubagentStart and SubagentStop (E7, evidence:387); no named plan-exit event in that list. `--settings <file|json>` and `--setting-sources` exist in help (evidence:13). A tool matcher for ExitPlanMode is a candidate, not a proven plan-ready notification. Do not install hooks or parse whole transcripts to discover plans in this phase.
6. **Paste - needs live run.** Embedded terminal-mode code contains bracketedPaste and ESC [200~/201~ markers (binary bytes 278099227 and 261899871 inspected). Their presence does not prove one-paste semantics or a maximum size. Test multi-line, trailing newline, slash-leading text, @paths, Unicode and large paste. Never append Enter; embedded delimiters in selected text need rejection or explicit escaping before forwarding.
8. **Status words - needs live run.** No stable spinner-word output contract established. Recommend core-reported status glyph. Only use a word explicitly reported by a verified status channel; do not scrape animated screen text or hardcode a vocabulary. Current core exposes state/title/msg, not a promised spinner feed ([core evidence](herdr-core-evidence.md:8)).
9. **Children - verified background CLI surface; topology needs live run.** Help documents --bg, attach, agents and logs, plus --forward-subagent-text for print stream output (evidence:13). This proves background-session support, not that every tool sub-agent owns a PTY. SubagentStart/Stop exist (E7); separate team processes, in-process tasks, transcript mapping and core parent correspondence remain unverified. Use real core terminal parents when present; add nonterminal child nodes only from verified lifecycle IDs. Clicking a nonterminal child should open an explicitly selected read-only transcript, never fabricate a terminal.
10. **Name - verified help/source; updates need live run.** `--name` sets display name in prompt box, resume picker and terminal title (evidence:13). Embedded storage reads `custom-title.json` with `{customTitle: string}` beside session data (E8, evidence:392); history treats custom-title and ai-title as last-wins (E9). Exact session-directory mapping and rename propagation need verification. Prefer terminal title fallback until that mapping is proven.
11. **Herdr integration - verified source mismatch; installation behavior needs live run/source follow-up.** Upstream master at `7edc5dfbe3f54ac23ad22313ce32f552f219ff44` lacks both requested directories. Current source consumes OSC 7501 status reports ([core evidence](herdr-core-evidence.md:8)); it is not proof that installed hc installs no hooks. No hc command or config mutation was performed. Do not enable any integration installer.

Limits: static inspection only. Neither plan-edit acceptance nor paste safety is signed off. The existing user's credentials, sessions and configuration were never opened. Help startup itself is not proven side-effect-free.

## Live checks for Wire, 2026-11-13 (2.1.322, Haiku, scratch core, hc 0.1.20)

Launched as `claude --model haiku --setting-sources '' --strict-mcp-config --settings <file>`
through `hc run` in a separate core session, with hooks that log their payload and print
`hookSpecificOutput.additionalContext`. Clean environment (`env -i`); a launch that inherits
another Claude session's `CLAUDE_CODE_*` variables turns transcript saving off.

1. **Hook context reaches the model - verified.** `SessionStart` and `UserPromptSubmit`
   `additionalContext` were both quoted back by the model on a fresh start.
2. **Resume gets fresh hook context - verified.** After `--resume <id>` with a changed
   marker the model saw the new value; `SessionStart` payload has `source: "resume"`
   (`"startup"` on a new session).
3. **A system prompt passed on resume is ignored - verified.** `--append-system-prompt`
   added on the resumed launch was not visible to the model. Dynamic facts must come
   from hooks or commands, never the launch note.
4. **Typed line while idle - verified.** `hc send <t> '<wire from=... kind="ask">...</wire>'`
   is taken as a prompt and answered. The model answered in its reply, as it had not been
   told about `canopy wire reply`.
5. **Typed line mid-turn is queued, not an interrupt - verified.** Sent 9 s into a 25 s
   tool call: the TUI showed it as a queued message ("Press up to edit queued messages",
   "ctrl+enter to send now"), the running tool finished, then the model handled both.
   `hc send` returned `status: "working"`.
6. **Core status works without any installed integration - verified.** `hc get` showed
   `status.state` `working` then `done`, with `app: "claude-code"`.
7. **Sub-agent hook payloads - verified.** `SubagentStart`: `agent_id`, `agent_type`,
   `session_id`, `transcript_path`, `cwd`, `prompt_id`; no description. `SubagentStop`
   adds `agent_transcript_path`, `last_assistant_message` and `background_tasks`
   (id, description, agent_type). Resuming a finished sub-agent fires `SubagentStart`
   again with the same `agent_id`.
8. **A parent relays to a finished sub-agent - verified.** Asked to pass a message to
   "the same sub-agent", the parent used `SendMessage`; the sub-agent resumed and
   answered with its earlier context.
9. **Standalone `sleep` is refused by the Bash tool** in this version; use another
   long command when a test needs a long turn.

10. **Typed line during a permission dialog is refused - verified.** With a Bash
    approval up, core status was `blocked` (`kind: permission`, `msg: "approve Bash:
    ..."`) and `hc send` exited 9: "typing would answer its dialog". Needs `USER` in the
    `env -i` launch, or Claude reports "Not logged in".

Not checked: the live pill repro in the app (folded into the phase 1 hand check).
