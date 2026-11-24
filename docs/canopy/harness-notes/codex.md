# Codex: Phase R, no-token research

2026-11-12, installed **codex-cli 0.167.1**. All answers below apply only to that version. [Evidence](codex-evidence.md:6) records version/help commands, embedded text offsets and executable hash. Rust binary strings prove the text ships, not which branches execute. Old main's generated-schema and live claims are not reused. No schema-generation or app-server command was run, per the help-only restriction. Question 7 is absent in the plan.

1. **Plan existence - verified embedded instructions; transitions need live run.** Plan Mode is a collaboration mode; `update_plan` is a separate progress checklist and cannot be used in Plan Mode (E1, evidence:163). Official plan is Markdown inside `<proposed_plan>` tags on separate lines (E2, evidence:193). Exact current TUI entry/exit command and implementation confirmation need verification in help inside an approved session; do not assume an undocumented flag.
2. **Storage - needs live run.** No native standalone plan markdown path established. Embedded instructions describe plan message blocks, not a writable plan file (E2). Embedded history guidance describes legacy rollout JSONL: session_meta.payload.id, turn_context, event_msg, response_item (E5, evidence:232). Help includes `migrate-rollouts` for paginated thread history (evidence:16), so legacy JSONL cannot be assumed universal. Exact current storage path, creation/update/delete times and native plan persistence are unresolved. Use an explicit user-owned markdown file as fallback; never edit vendor history databases or rollouts.
3. **Editor - verified draft capability; key/argv/wait need live run.** Embedded action description says `Open the current draft in an external editor`; error says set VISUAL or EDITOR before starting Codex (E3/E4, evidence:222). It does not say open the agent's plan. Current binding is identified symbolically as global.open_external_editor in binary byte 210362953. Check the session's shortcuts before assuming Ctrl+G; precedence, process waiting and temp-file reread need the shim.
4. **Awareness - needs live run.** Draft editor evidence supports editing the next user message, not mutating a model plan (E4). No verified spontaneous external-plan-file awareness. Recommend paste an explicit edited-plan message, review it, then submit manually. A saved file alone must not be presented as acknowledged.
5. **Hooks - verified per-invocation config override; lifecycle needs live run/source follow-up.** `codex --help` documents `-c key=value`, dotted keys, TOML values, overriding config.toml for that invocation (evidence:16). Hook-related strings exist, but no hook schema/payload or plan event was established. Do not copy old app-server schemas into a terminal integration or invent a hook key.
6. **Paste - needs live run.** `[Pasted Content ... chars]` UI text ships (binary byte 204829750 inspected), but is not a size/safety guarantee. Test one bracketed multi-line paste with no Enter, trailing newline, slash-leading text, @paths, Unicode and large content. Reject raw bracketed-paste terminators in input. Do not claim a numeric limit or enable automatic submit.
8. **Status words - needs live run.** No stable working-word interface established. Recommend core status glyph when reported, otherwise unknown state; do not infer busy from a changing terminal title. Current core OSC records expose state/title/msg ([core evidence](herdr-core-evidence.md:8)).
9. **Children - verified tool presence; process/PTY topology needs live run.** Embedded spawn_agent instructions exist (E7, evidence:252); metadata strings include parent_thread_id (binary byte 205757360 inspected). Help lists an agents session switcher (evidence:16). None proves one process or PTY per child, or establishes an installed transcript schema for child nodes. Prefer actual core terminal parent edges; only add thread children when IDs and lifecycle are verified. A nonterminal child opens a read-only transcript once its mapping is proven, not an invented PTY.
10. **Name - verified name support; mapping needs live run.** Help allows archive/delete by session name (evidence:16). Embedded SessionConfiguredEvent includes thread_name (E6, evidence:247); generated-title persistence error is present at byte 206066146. Exact title store, rename command and update cadence are unresolved. Use terminal title, then folder basename as display fallback.
11. **Herdr integration - verified source mismatch; installation behavior needs live run/source follow-up.** Requested integration/assets and detect/manifests paths are absent at upstream master `7edc5dfbe3f54ac23ad22313ce32f552f219ff44` ([core evidence](herdr-core-evidence.md:1)). OSC 7501 consumer exists; no installed-binary/source match proven. Do not infer config writes or run an installer from obsolete notes.

Observed CLI warning: version/help attempted to create PATH aliases and were denied by the sandbox. No config was deliberately written. This research does not certify all startup side effects.

## Live checks for Wire, 2026-11-13 (0.167.2, gpt-6-luna low, scratch core, hc 0.1.20)

Launched through `hc run` in a separate core with `env -i`, per-run `-c` overrides only
(project trust, hooks, approvals), so `~/.codex/config.toml` was never written.

- **Hooks can be passed per session - verified.**
  `-c 'hooks.SessionStart=[{hooks=[{type="command",command="..."}]}]'` is accepted.
- **New hooks need the person's trust - verified.** On start the TUI shows "Hooks need
  review: 2 hooks are new or changed", with Review, Trust all, or Continue without
  trusting (hooks will not run). Trust is stored as `hooks.state.<key>.trusted_hash` in
  `config.toml` (binary strings), so it persists, but only by writing the person's config.
  `--dangerously-bypass-hook-trust` runs all enabled hooks for one launch without review.
- **Hook context does not reach the model - verified, fails.** `SessionStart`
  (`source: "startup"`, and `"resume"` after `codex resume <id>`) and `UserPromptSubmit`
  ran, and their output (JSON `additionalContext` or plain stdout) was stored in the
  rollout as a developer message (`hooks.additional_context`). Asked five ways, fresh and
  resumed, the model answered NONE/UNKNOWN every time. So Codex gets the static note and
  the pull path (`canopy wire`) only, no hooks.
- **Typed line while idle - verified.** A `<wire ...>` line is taken as a prompt and
  answered.
- **Typed line mid-turn steers, it does not queue - verified.** Sent 9 s into a 25 s
  command: Codex added it to the running turn as a user message after the current tool
  call yielded; the command kept running and finished. `hc send` returned `working`.
  Unlike Claude, the line enters the current turn, so Codex is not a `"wire": "queue"`
  CLI.
- **Typed line during a dialog is refused - verified.** With an approval dialog up, core
  status was `blocked` (`kind: permission`) and `hc send` exited 9: "typing would
  answer its dialog".
- **Allow rule for one prefix - verified.** `~/.codex/rules/*.rules` with
  `prefix_rule(pattern=["canopy", "wire"], decision="allow")` matches
  `canopy wire tell x "hi"` and not `canopy wired` (`codex execpolicy check`). Rules live
  in the person's Codex home; there is no per-launch flag, so Canopy cannot add one. The
  approval dialog's "don't ask again for commands that start with" option writes the
  same kind of rule.
- `codex queue --thread <id> --message <text>` exists (help); needs the daemon.
- Feature flags: `hooks` stable; `agent_message_board` and `wake_on_agent_messages`
  under development, off.

Not checked: whether `canopy wire` (app home and core socket outside the workspace)
runs inside Codex's `workspace-write` sandbox without escalation.

## Agent teams, 2026-11-13 (0.169.0)

- **Turn-end payload exists - binary strings only, not run.** The binary ships
  `legacy_notify` with `agent-turn-complete`, `thread-id`, `turn-id`, `cwd`, `client`,
  `input-messages`, `last-assistant-message`, and a `stop` hook event with
  `last_assistant_message`. Canopy does not use either yet: `-c notify=[...]` replaces the
  person's own `notify` (this machine's config uses it for another tool's `turn-ended`),
  and hooks need the person's trust (see above). So a Codex child's idle line comes from
  the app and ends `canopy wire read <t>`.
- **`canopy wire tell` from Codex - verified, dev home only.** With the person's
  `sandbox_mode = "workspace-write"` and `approval_policy = "never"`, a spawned Codex ran
  `canopy wire tell t1 "hi from nova"` and it was delivered. The dev app home and core
  socket sit under `$TMPDIR`, which the sandbox allows; a real `~/.config/canopy` home is
  not checked.
- A Codex child's status goes `working` then `idle` in the core, which the app's
  fallback idle line relies on - verified.

## Global skills, 2026-11-17 (0.169.0)

- **Codex reads skills from both `$CODEX_HOME/skills` and `~/.agents/skills` - verified,
  isolated `HOME` and `CODEX_HOME`, no model call.** With a test `SKILL.md` (`name`,
  `description` frontmatter) in each, `codex debug prompt-input "hi"` listed both under
  "Available skills" with aliased roots `r0` (`$CODEX_HOME/skills`) and `r1`
  (`~/.agents/skills`). Canopy writes `~/.agents/skills/canopy/SKILL.md`, which does not
  depend on `CODEX_HOME` and is the shared folder other harnesses may read too.

## Terminal notifications, 2026-11-24 (0.171.0)

The installed executable contains `tui.notifications`,
`tui.notification_method` variants `auto`, `osc9`, `osc777` and `bel`, and
`tui.notification_condition` variants `unfocused` and `always`. Enable
`tui.notifications` and choose `osc9` or `osc777` explicitly to check message
capture in Canopy. `tui.notification_condition = "always"` removes the
terminal-focus condition when checking. The top-level `notify` setting runs a
command and is separate from terminal notifications.

Checked with `codex --version` and the installed binary's strings. Live PTY
output and this version's automatic method selection were not exercised.
