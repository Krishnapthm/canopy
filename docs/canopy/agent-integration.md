# Agent integration recommendation

Phase R no-token result, 2026-11-12. **Recommend one terminal-first design: a shared preview/editor and explicit review-before-submit text delivery, with optional blocking external-editor handoff once proven live.** Native plan Save must never imply agent acknowledgment. Keep Phase 3 live acceptance gated. Evidence: [Claude 2.1.322](harness-notes/claude-code.md), [Codex 0.167.1](harness-notes/codex.md).

Extend the existing program description (`src/programs.rs`) with data for launch argv, editor environment, plan source and send policy. Reuse its existing detection, label and icon fields. Do not create a second registry or add these fields in this research phase. The plan's core detect-manifest paths are missing at the fetched upstream revision, so resolve ownership before implementing a schema extension. Per-agent code is limited to plan/session mapping when the evidence requires it.

| Description | Claude Code 2.1.322 | Codex 0.167.1 |
|---|---|---|
| Detection / launch | Existing program entry / `claude` | Existing program entry / `codex` |
| Plan source | Native `<plansDirectory>/<slug>.md`; session-to-slug mapping needs live run | No native markdown file established; explicit user-selected markdown fallback |
| Editor environment | VISUAL and EDITOR; precedence verified in embedded source | VISUAL and EDITOR named by embedded error; precedence needs live run |
| Editor target | Plan or prompt through Ctrl+G; runtime needs live run | Draft action verified in embedded text; plan editor unproven |
| Send policy | Bracketed paste, no Enter; safety/limits need live run | Same, needs live run |
| Name | Terminal title fallback; --name verified | Terminal title fallback; session name support verified |
| Children / status | Core terminal parents/status when present; nonterminal lifecycle needs live run | Same |

View the explicitly identified plan in the same preview card used for ordinary files. For Claude, use native file mapping only after proving session identity; never select the newest plan globally. For Codex, start with a user-selected markdown document. Label it a shared plan, not a native Codex plan. Save writes that file; Send edits pastes an explicit message asking the agent to reread it, then the user submits. Until paste checks pass, keep Send disabled and allow Copy. These are recommendations, not implemented capabilities.

The editor handoff is a blocking command: Canopy opens the CLI-supplied file, waits for Done, then exits so the CLI can reread through its own flow. Claude source supports the mechanism; Codex evidence supports draft handoff only. Preserve the CLI's exit/error semantics and handle cancellation without silently reporting success. The [unrun shim experiment](harness-notes/shim-experiment.md) decides whether to enable each capability.

Use current core OSC status and real terminal-parent edges where available. Spinner words have no proven contract, so use the glyph fallback. Do not invent terminal cards for in-process workers. A nonterminal child may open a read-only transcript only after identity, lifecycle and storage mapping are verified. Never read all user transcripts to infer active sessions.

Rejected: screen scraping for plans/spinner words - unstable rendering and ambiguous session identity.
Rejected: editing rollouts/databases - no supported plan-edit contract and risks history corruption.
Rejected: vendor API/app-server/ACP replacement - changes the terminal ownership model and exceeds this phase.
Rejected: persistent hook/config installation - unapproved user-state mutation; no need for the fallback.

A third agent needs one existing program entry and evidence for launch, editor target/wait behavior, plan source, paste semantics, naming and child identity. Unknown capabilities remain unavailable. No new trait/plugin system is needed.

**Needs live run:** editor argv/wait/reread, model acknowledgment, idle file edits, paste matrix, plan identity/lifecycle, current key bindings, child topology/title updates and installed core compatibility. Question 11 additionally needs the correct integration source/version; an absent directory cannot establish installer behavior.
