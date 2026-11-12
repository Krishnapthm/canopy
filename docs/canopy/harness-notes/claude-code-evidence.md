# claude-code installed evidence
Executable: `/Users/krishna/.local/share/claude/versions/2.1.322`
SHA256: `a9d237949db816a0b7afd88a9a8e1df36b3a597a30f5a41d1c0539eb180a3450`
Collected 2026-11-12. Display normalization: em dashes and escaped Unicode em dashes replaced with plain dashes; control bytes rendered as escapes. Native binary excerpts below are evidence of embedded text/source, not observed runtime behavior. Byte offsets are zero-based. Reproduce with Python Path(path).read_bytes()[offset:offset+length].decode("utf-8", "replace").

## Command: claude --version
Exit: 0
```text
2.1.322 (Claude Code)

```

## Command: claude --help
Exit: 0
```text
Usage: claude [options] [command] [prompt]

Claude Code - starts an interactive session by default, use -p/--print for
non-interactive output

Arguments:
  prompt                                Your prompt

Options:
  --add-dir <directories...>            Additional directories to allow tool
                                        access to
  --agent <agent>                       Agent for the current session. Overrides
                                        the 'agent' setting.
  --agents <json-or-file>               JSON object defining custom agents, or
                                        with --print the path to a file that
                                        holds one (e.g. '{"reviewer":
                                        {"description": "Reviews code",
                                        "prompt": "You are a code reviewer"}}')
  --allow-dangerously-skip-permissions  Enable bypassing all permission checks
                                        as an option, without it being enabled
                                        by default. Recommended only for
                                        sandboxes with no internet access.
  --allowedTools, --allowed-tools <tools...>
      Comma or space-separated list of tool names to allow (e.g. "Bash(git *)
      Edit")
  --append-system-prompt <prompt>       Append a system prompt to the default
                                        system prompt
  --autocompact <auto|tokens>           Auto-compact window size (auto, or
                                        100k–1M tokens)
  --ax-screen-reader                    Render screen-reader friendly output
                                        (flat text, no decorative borders or
                                        animations).
  --bg, --background                    Start the session in the background and
                                        return immediately. Prints the id that
                                        `claude attach`, `logs`, `stop` and `rm`
                                        take; `claude agents` lists them. With
                                        --resume <session-id>, continues that
                                        session in the background under the same
                                        ID, or starts a copy and says so when
                                        the session is already running
  --bare                                Minimal mode: skip hooks (those defined
                                        in settings and by installed plugins;
                                        features built into Claude Code are
                                        unaffected), LSP, plugin sync,
                                        attribution, auto-memory, background
                                        prefetches, keychain reads, and
                                        CLAUDE.md auto-discovery. Sets
                                        CLAUDE_CODE_SIMPLE=1. Anthropic auth is
                                        strictly ANTHROPIC_API_KEY or
                                        apiKeyHelper via --settings (OAuth and
                                        keychain are never read). 3P providers
                                        (Bedrock/Vertex/Foundry) use their own
                                        credentials. Skills still resolve via
                                        /skill-name. Explicitly provide context
                                        via: --system-prompt[-file],
                                        --append-system-prompt[-file], --add-dir
                                        (CLAUDE.md dirs), --mcp-config,
                                        --settings, --agents, --plugin-dir.
  --betas <betas...>                    Beta headers to include in API requests
                                        (API key users only)
  --brief                               Enable SendUserMessage tool for
                                        agent-to-user communication
  --chrome                              Enable Claude in Chrome integration
  --cloud [description|session_id|url]  Create a cloud session with the given
                                        description, or attach to an existing
                                        one by session ID or claude.ai/code URL
  -c, --continue                        Continue the most recent conversation in
                                        the current directory
  --dangerously-skip-permissions        Bypass all permission checks.
                                        Recommended only for sandboxes with no
                                        internet access.
  -d, --debug [filter]                  Enable debug mode with optional category
                                        filtering (e.g., --debug=hooks,mcp or
                                        --debug='!hooks,!mcp')
  --debug-file <path>                   Write debug logs to a specific file path
                                        (implicitly enables debug mode)
  --desktop                             Open in the Claude Desktop app instead
                                        of the terminal (with --continue or
                                        --resume <id> to pick the session)
  --disable-slash-commands              Disable all skills and commands
  --disallowedTools, --disallowed-tools <tools...>
      Comma or space-separated list of tool names to deny (e.g. "Bash(git *)
      Edit")
  --effort <level>                      Effort level for the current session
                                        (low, medium, high, xhigh, max)
  --environment <environment_id>        Create a new cloud session that runs on
                                        the given self-hosted environment
                                        (ccpool_...).
  --exclude-dynamic-system-prompt-sections
      Move per-machine sections (cwd, env info, memory paths, git status) from
      the system prompt into the first user message. Improves cross-user
      prompt-cache reuse. Only applies with the default system prompt (ignored
      with --system-prompt). (default: false)
  --fallback-model <model>              Enable automatic fallback to specified
                                        model(s) when the default model is
                                        overloaded or not available. Accepts a
                                        comma-separated list to try each in
                                        order. Re-tries the primary at the start
                                        of each user turn.
  --file <specs...>                     File resources to download at startup.
                                        Format: file_id:relative_path (e.g.,
                                        --file file_abc:doc.txt
                                        file_def:img.png)
  --fork-session                        When resuming, create a new session ID
                                        instead of reusing the original (use
                                        with --resume or --continue)
  --forward-subagent-text               Forward subagent text and thinking
                                        blocks as assistant/user messages with
                                        parent_tool_use_id set (only works with
                                        --print and --output-format=stream-json)
  --from-pr [value]                     Resume a session linked to a PR by PR
                                        number/URL, or open interactive picker
                                        with optional search term
  -h, --help                            Display help for command
  --ide                                 Automatically connect to IDE on startup
                                        if exactly one valid IDE is available
  --include-hook-events                 Include all hook lifecycle events in the
                                        output stream (only works with
                                        --output-format=stream-json)
  --include-partial-messages            Include partial message chunks as they
                                        arrive (only works with --print and
                                        --output-format=stream-json)
  --input-format <format>               Input format (only works with --print):
                                        "text" (default), or "stream-json"
                                        (realtime streaming input) (choices:
                                        "text", "stream-json")
  --json-schema <schema>                JSON Schema for structured output
                                        validation (only works with --print).
                                        Example:
                                        {"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}
  --max-budget-usd <amount>             Maximum dollar amount to spend on API
                                        calls (only works with --print)
  --mcp-config <configs...>             Load MCP servers from JSON files or
                                        strings (space-separated)
  --model <model>                       Model for the current session. Provide
                                        an alias for the latest model (e.g.
                                        'fable', 'opus', or 'sonnet') or a
                                        model's full name.
  -n, --name <name>                     Set a display name for this session
                                        (shown in the prompt box, /resume
                                        picker, and terminal title)
  --no-chrome                           Disable Claude in Chrome integration
  --no-session-persistence              Disable session persistence - sessions
                                        will not be saved to disk and cannot be
                                        resumed (only works with --print)
  --output-format <format>              Output format (only works with --print):
                                        "text" (default), "json" (single
                                        result), or "stream-json" (realtime
                                        streaming) (choices: "text", "json",
                                        "stream-json")
  --permission-mode <mode>              Permission mode to use for the session
                                        (choices: "acceptEdits", "auto",
                                        "bypassPermissions", "manual",
                                        "dontAsk", "plan")
  --permission-prompts <target>         Who answers permission prompts with
                                        --print: "host" (the SDK host or
                                        --permission-prompt-tool) or "none"
                                        (nobody: anything that would prompt is
                                        denied automatically; the permission
                                        mode still decides everything else)
                                        (choices: "host", "none", default:
                                        "host")
  --plugin-dir <path>                   Load a plugin from a directory or .zip
                                        for this session only; a folder of
                                        plugins loads each child (repeatable:
                                        --plugin-dir A --plugin-dir B.zip)
                                        (default: [])
  --plugin-url <url>                    Fetch a plugin .zip from a URL for this
                                        session only (repeatable: --plugin-url A
                                        --plugin-url B) (default: [])
  -p, --print                           Print response and exit (useful for
                                        pipes). Note: The workspace trust dialog
                                        is skipped when Claude is run in
                                        non-interactive mode (via -p, or when
                                        stdout is not a TTY, e.g. piped or
                                        redirected output). Only use this in
                                        directories you trust. Settings files
                                        that fail validation are silently
                                        ignored in this mode (no error dialog is
                                        shown).
  --prompt-suggestions [value]          Enable prompt suggestions. In print/SDK
                                        mode, emits a prompt_suggestion message
                                        after each turn with a predicted next
                                        user prompt (choices: "true", "false",
                                        "1", "0", "yes", "no", "on", "off",
                                        preset: "true")
  --remote-control [name]               Start an interactive session with Remote
                                        Control enabled (optionally named)
  --remote-control-session-name-prefix <prefix>
      Prefix for auto-generated Remote Control session names (default: hostname)
  --replay-user-messages                Re-emit user messages from stdin back on
                                        stdout for acknowledgment (only works
                                        with --input-format=stream-json and
                                        --output-format=stream-json)
  --restricted                          Restricted mode: removes the built-in
                                        tools that run commands or code (Bash,
                                        PowerShell and the other code-running
                                        tools) and WebFetch unless --tools names
                                        them, and ignores user, project and
                                        local settings files (managed settings
                                        and --settings still apply; add
                                        --strict-mcp-config to skip MCP servers
                                        too). Also confines the file tools to
                                        the working directories (--add-dir
                                        included), refuses bypassPermissions,
                                        and lets only a person or the configured
                                        permission handler approve writes to
                                        settings, git and tool-configuration
                                        files.
  -r, --resume [value]                  Resume a conversation by session ID or
                                        name, or open interactive picker with
                                        optional search term
  --safe-mode                           Start with all customizations
                                        (CLAUDE.md, skills, installed plugins,
                                        hooks, MCP servers, custom commands and
                                        agents, output styles, workflows, custom
                                        themes, keybindings, and more) disabled
                                        - useful for troubleshooting a broken
                                        configuration. Admin-managed (policy)
                                        settings still apply. Auth, model
                                        selection, built-in tools and plugins,
                                        and permissions work normally. Sets
                                        CLAUDE_CODE_SAFE_MODE=1.
  --session-id <uuid>                   Use a specific session ID for the
                                        conversation (must be a valid UUID)
  --setting-sources <sources>           Comma-separated list of setting sources
                                        to load (user, project, local).
  --settings <file-or-json>             Path to a settings JSON file or a JSON
                                        string to load additional settings from
  --strict-mcp-config                   Only use MCP servers from --mcp-config,
                                        ignoring all other MCP configurations
  --system-prompt <prompt>              System prompt to use for the session
  --system-prompt-snapshot <on|off>     Record the system prompt once per
                                        conversation and reuse it verbatim on
                                        every request and resume. on (the
                                        default, except under --bare outside
                                        cloud and background sessions): the
                                        prompt is rendered on the conversation's
                                        first request - a --system-prompt or
                                        --append-system-prompt included - sent,
                                        and recorded; every later request and
                                        resume sends the record as-is, even when
                                        a later launch passes different text,
                                        until the conversation is compacted.
                                        off: never record; the prompt is
                                        rendered fresh every request (for
                                        iterating on prompt text). (choices:
                                        "on", "off")
  --teleport [session]                  Resume a teleport session, optionally
                                        specify session ID
  --tmux                                Create a tmux session for the worktree
                                        (requires --worktree). Uses iTerm2
                                        native panes when available; use
                                        --tmux=classic for traditional tmux.
  --tools <tools...>                    Specify the list of available tools from
                                        the built-in set. Use "" to disable all
                                        tools, "default" to use the default set,
                                        or specify tool names (e.g.
                                        "Bash,Edit,Read").
  --verbose                             Override verbose mode setting from
                                        config
  -v, --version                         Output the version number
  -w, --worktree [name]                 Create a new git worktree for this
                                        session (optionally specify a name)

Commands:
  agents [options]                      Manage background agents
  attach <id|name>                      Open a background session in this
                                        terminal. <id> is the short id that
                                        `claude --bg` prints and `claude agents`
                                        lists. Part of the session name works
                                        too
  auth                                  Manage authentication
  auto-mode                             Inspect or reset auto mode configuration
  doctor                                Check the health of your Claude Code
                                        installation. Reads settings files in
                                        the current directory without a trust
                                        prompt. For a full checkup that can also
                                        fix issues, run /doctor in a session.
  gateway [options]                     Run the enterprise auth/telemetry
                                        gateway
  import [options] [source]             Import config from another AI coding
                                        agent into Claude Code
  install [options] [target]            Install Claude Code native build. Use
                                        [target] to specify version (stable,
                                        latest, or specific version)
  logs <id|name>                        Print a background session's recent
                                        terminal output
  mcp                                   Configure and manage MCP servers
  plugin|plugins                        Manage Claude Code plugins
  purge [options] [path]                Delete a project's Claude Code data
                                        (transcripts, tasks, file history, and
                                        its entry in ~/.claude.json) from this
                                        computer only
  respawn [options] [id]                Restart a background session, or all of
                                        them with --all, so it runs the current
                                        Claude Code version
  rm <id>                               Delete a background session, and its
                                        worktree when that is safe. Works on
                                        sessions that have already exited
  setup-token                           Set up a long-lived authentication token
                                        (requires Claude subscription)
  stop|kill <id>                        Stop a background session. Its
                                        conversation is kept: `claude attach
                                        <id>` opens it again, `claude --resume`
                                        works once it is stopped
  ultrareview [options] [target]        Run a cloud-hosted multi-agent code
                                        review of the current branch (or a PR
                                        number / base branch) and print the
                                        findings
  update|upgrade                        Check for updates and install if
                                        available; package-manager installs
                                        update through the package manager

```

## E1: byte 261544221, length 420
```text
plansDirectory:()=>o().optional().describe("Custom directory for plan files, relative to project root. If not set, defaults to ~/.claude/plans/"),tui:()=>B(["default","fullscreen"]).optional().describe('Terminal UI renderer. "fullscreen" uses the flicker-free alt-screen renderer with virtualized scrollback (equivalent to CLAUDE_CODE_NO_FLICKER=1). "default" uses the classic main-screen renderer.'),...!1,voice:()=>u({
```

## E2: byte 266372600, length 900
```text
(n,r)}catch(r){if(!K(r)&&!Yt(r))a(r);if(n!==io())return;if(K(r))PO().commitPlanListing(n,[])}}function io(){if(!ye()&&!p1e())return Ct();return Sp()}function Ct(){return ft(Be(),"plans")}function ot(e){return Ye.plan(e)}function On(e){return e.endsWith(".md")?e.slice(0,-3):e}var xn={publishDiscipline:"inPlace"};async function up(e){if(q())return xn;try{let n=await ep(ft(Sp(),`${e}.md`));return{...xn,mode:n.mode&511}}catch{return xn}}function At(){return Sp()===Ct()}function tRe(e,n){return PO().getPlanSlug(e??G(),n)}function lVe(e){return g1e().get(e??G())}function Cc(e){return typeof e==="string"&&e.length<=sp&&ip.test(e)&&!HGt.test(e)}function Ac(e,n){let r=typeof n==="string"?_(Nl(n.replace(M8,""),80)):`<non-string slug of type ${typeof n}>`;t(`${e}: refusing invalid plan slug ${r}`,{level:"error"})}function Rbo(e,n){if(!Cc(n)){Ac("setPlanSlug",n);return}g1e().set(e,n),PO().exemptSlug
```

## E3: byte 266375700, length 1300
```text
t(n));let s=r.get(Gt(n));if(s!==void 0)return s}return Tc(n)}async function jgt(e,n){if(n===void 0||!At())return ute(e);let r=Dc(e);await PO().settlePlanFile(r);let s=PO().planFileCache?.get(r);if(s!==void 0)return s;return xc(n,On(r),"getPlan")}function Dc(e){let n=Gt(MA(e)),r=PO();if(e&&r.planFileCache!==null)r.watchAgentPlanFile(tRe(G()),n);return n}async function QRs(e,n){if(n===void 0||!At())return ute(e)!==null;let r=Dc(e);await PO().settlePlanFile(r);let s=PO().planFileCache?.get(r);if(s!==void 0)return s!==null;return(await n.statMeta(ot(On(r)))).ok}async function ZRs(e){let n=MA();if(X()&&e!==void 0&&At()){let r=Gt(n),s=PO(),h=s.planFileWatches.get(r);if(h!==void 0){if(!await Ii(s.refreshPlanFile(r,h),s.waitCapMs))s.giveUpWaiting(r,h);let y=s.planFileCache?.get(r);if(y!==void 0)return y}return xc(e,On(r),"readPlanFileFresh")}return Tc(n)}function Tc(e){if(q())return null;try{return ae().readFileSync(e,{encoding:"utf-8"})}catch(n){if(K(n))return null;if(Yt(n))return t(`getPlan: read failed for ${e}: ${n}`),null;return a(n),null}}function Ic(e){let n,r;for(let s of e.messages){let h=s.slug;if(!h||h===n)continue;if(Cc(h))n=h;else r??=h}if(n===void 0&&r!==void 0)Ac("getSlugFromLog",r);return n}async function son(e,n,r){let s=Ic(e);if(!s)return!1;let h=n??G();if(Rbo(h,s),r&&A
```

## E4: byte 279503700, length 900
```text
e plan file path (injected by normalizeToolInput)")})),ne=p(()=>u({plan:o().nullable().describe("The plan that was presented to the user"),isAgent:I(),filePath:o().optional().describe("The file path where the plan was saved"),hasTaskTool:I().optional().describe("Whether the Agent tool is available in the current context"),planWasEdited:I().optional().describe("True when the user edited the plan (CCR web UI or Ctrl+G); determines whether the plan is echoed back in tool_result"),awaitingLeaderApproval:I().optional().describe("When true, the teammate has sent a plan approval request to the team leader"),requestId:o().optional().describe("Unique identifier for the plan approval request")}));function L(a){if(a.teammateContext)return a.teammateContext;let e=GH();return e?.agentId&&e.teamName?e:void 0}function J(a,e,n){let m=`The approved plan is ${cNn(a)}, as the user left it when they approve
```

## E5: byte 279513490, length 1450
```text
planWasEdited:r,awaitingLeaderApproval:h,requestId:s},d){if(h)return{type:"tool_result",content:`Your plan has been submitted to the team lead for approval.

Plan file: ${n}

**What happens next:**
1. Wait for the team lead to review your plan
2. You will receive a message in your inbox with approval/rejection
3. If approved, you can proceed with implementation
4. If rejected, refine your plan based on the feedback

**Important:** Do NOT proceed until you receive approval. Check your inbox for response.

Request ID: ${s}`,tool_use_id:d};if(a)return{type:"tool_result",content:'User has approved the plan. There is nothing else needed from you now. Please respond with "ok"',tool_use_id:d};if(!e||e.trim()==="")return{type:"tool_result",content:"User has approved exiting plan mode. You can now proceed.",tool_use_id:d};let v=Hor(Boolean(m)),x=r?"Approved Plan (edited by user)":"Approved Plan";return{type:"tool_result",content:`User has approved your plan. You can now start coding. Start with updating your todo list if applicable${n===void 0?"":`

Your plan has been saved to: ${n}
You can refer back to it if needed during implementation.`}${v}

## ${x}:
${e}`,tool_use_id:d}}});
export{Dmi,Lor,Lmi,Hor,yY};
\x00// @bun @bytecode
// (c) Anthropic PBC. All rights reserved. Use is subject to the Legal Agreements outlined here: https://code.claude.com/docs/en/legal-and-compliance.

// Version: 2.1.322
import{t}from"/$bunfs/root/chunk-wwzax483
```

## E6: byte 282900940, length 2050
```text
s.isResolved)return this.editor;if(i.VISUAL)this.editor=i.VISUAL;else if(i.EDITOR)this.editor=i.EDITOR;else{let n=["code","vi","nano"];this.editor=n.find((e)=>O(e))}return this.isResolved=!0,this.editor}reset(){this.isResolved=!1,this.editor=void 0}}var D=new W(()=>new b);function N(){return D.of(F().host)}function RL(){return vf()?.editor??N().resolve()}function bY(){let n=RL();if(!n)return;let e=y(n);return e&&e.length<=8?e:void 0}var U={code:"code -w",subl:"subl --wait"};function vve(n,e){let o=ae(),r=$i().get(process.stdout);if(!r)throw Error("Ink instance not found - cannot pause rendering");let l=RL();if(!l)return{content:null};try{o.statSync(n)}catch{return{content:null}}let u=h(l)===void 0;if(u)r.enterAlternateScreen();else r.prepareTerminalForHandoff();try{let m=U[l]??l,a=m.split(" "),s=a[0]??m,d=a.slice(1),g=x(l,n,e),c;{let f=sLo();if(c=R(s,[...d,...g],{stdio:"inherit",...f}),c.error&&f.cgroup!==void 0)c=R(s,[...d,...g],{stdio:"inherit"})}if(c.error||c.signal||c.status!==null&&c.status!==0){let f=Bk(l);return{content:null,error:c.error?`Couldn't open ${f} - ${c.error.message}`:c.signal?`${f} closed unexpectedly (${c.signal})`:`${f} quit unexpectedly (exit code ${c.status})`}}return{content:o.readFileSync(n,{encoding:"utf-8"})}}catch{return{content:null}}finally{if(u)r.exitAlternateScreen();else r.restoreTerminalAfterHandoff()}}function k(n,e,o){let r=n;for(let[l,u]of Object.entries(o))if(u.type==="text"){let m=parseInt(l),a=u.content;if(u.unavailable||a==="")continue;if(u.inline)continue;let s=r.indexOf(a);if(s!==-1){let d=vV(a),g=Due(m,d);r=r.slice(0,s)+g+r.slice(s+a.length)}}return r}var S="# \u2500\u2500\u2500 Write your reply below this line \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500",C=50;function H(n){let e=Yb(n).split(`
`);if(e.length>C)e=e.slice(-C),e.unshift("\u2026 (earlier output truncated)");return`# \u2500\u2500\u2500 Claude's last response (for reference; removed on save) \u
```

## E7: byte 261251000, length 650
```text
lUse","PostToolUse","PostToolUseFailure","PostToolBatch","Notification","UserPromptSubmit","UserPromptExpansion","SessionStart","SessionEnd","Stop","StopFailure","SubagentStart","SubagentStop","PreCompact","PostCompact","PreModelSwitch","PostModelSwitch","PermissionRequest","PermissionDenied","Setup","TeammateIdle","TaskCreated","TaskCompleted","Elicitation","ElicitationResult","ConfigChange","WorktreeCreate","WorktreeRemove","InstructionsLoaded","CwdChanged","FileChanged","DirectoryAdded","MessageDisplay"],qaa=["clear","resume","logout","prompt_input_exit","other"],C1="__SYSTEM_PROMPT_DYNAMIC_BOUNDARY__";var urt=["anthropic-skills"],r1r="ant
```

## E8: byte 269242800, length 700
```text
rt{dirname as z,join as B}from"path";var H=p(()=>u({customTitle:o()}));function Tbr(e,n){return B(z(e),n,"custom-title.json")}async function uLi(e,n,t){let r=await K(Tbr(e,n),t);if(r===void 0)return;let i;try{i=JSON.parse(r)}catch{return}let s=H().safeParse(i);if(!s.success)return;return sa(s.data.customTitle)||void 0}async function K(e,n){if(X()&&n!==void 0)try{let t=uR(e);if(t!==void 0){let r=await n.readText([t]);if(!r.ok)return;let i=r.value.items[0];return i.found?i.value:void 0}}catch{return}if(q())return;try{return await W(e,"utf8")}catch{return}}var kpt="Tool permission request failed",hhs="Tool permission stream closed before response received",sfo="Stream closed",$e=[`${kpt}: Abort
```

## E9: byte 271182400, length 650
```text
nt-replacement":"accumulate","api-request-shape":"boundary-cleared","api-request-blob":"boundary-cleared","api-request":"boundary-cleared","fork-context-ref":"accumulate","frame-link":"accumulate",summary:"last-wins","custom-title":"last-wins","ended-by-model":"last-wins","ai-title":"last-wins",tag:"last-wins",relocated:"last-wins","agent-name":"last-wins","agent-color":"last-wins","agent-setting":"last-wins","pr-link":"last-wins","artifact-comment-monitor":"accumulate","artifact-autoreact-ledger":"last-wins","bridge-session":"last-wins","history-suppression":"last-wins","attribution-snapshot":"last-wins",mode:"last-wins","permission-mode":"l
```
