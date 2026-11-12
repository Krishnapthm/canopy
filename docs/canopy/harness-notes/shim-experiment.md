# Editor shim experiment - needs live run

Not executed. Krishna's explicit OK is required before any CLI launch below. This gate comes from plan 3/7A and the task's hard limit. Installed targets: Claude Code 2.1.322 and Codex CLI 0.167.1. No claim about model awareness is established by this document.

Use a fresh terminal for each CLI. The commands create only throwaway state. Do not copy credentials, existing settings, plugins or transcripts. If isolated authentication is unavailable, stop and let Krishna choose an approved scratch authentication setup; do not repoint these commands to his real home. Do not log in automatically. A bare isolated launch may still make vendor network requests, so it belongs exclusively to the approved live phase.

After approval, prepare:

```sh
experiment_root="$(mktemp -d /tmp/canopy-shim.XXXXXX)"
export CANOPY_SHIM_ROOT="$experiment_root"
mkdir -p "$experiment_root/home" "$experiment_root/tmp" "$experiment_root/work" "$experiment_root/claude" "$experiment_root/codex" "$experiment_root/records"
cat > "$experiment_root/editor-shim" <<'PY'
#!/usr/bin/env python3
import json
import os
from pathlib import Path
import sys
import tempfile

root = Path(os.environ['CANOPY_SHIM_ROOT']).resolve(strict=True)
if len(sys.argv) < 2:
    raise SystemExit('missing editor file')
path = Path(sys.argv[-1]).resolve(strict=True)
if not path.is_relative_to(root) or not path.is_file():
    raise SystemExit('refusing file outside scratch root')
before = path.read_text(encoding='utf-8')
after = before.replace('CANOPY_BEFORE', 'CANOPY_AFTER', 1)
if before == after:
    raise SystemExit('missing synthetic marker; no edit performed')
record = {'argv': sys.argv[1:], 'file': str(path), 'before': before, 'after': after}
with tempfile.NamedTemporaryFile(mode='w', encoding='utf-8', dir=root / 'records',
                                 prefix='editor-', suffix='.json', delete=False) as log:
    json.dump(record, log, ensure_ascii=False, indent=2)
# Preserve the inode and permissions of the CLI-owned temporary file.
with path.open('w', encoding='utf-8') as edited:
    edited.write(after)
PY
chmod 700 "$experiment_root/editor-shim"
export TMPDIR="$experiment_root/tmp/"
export XDG_CONFIG_HOME="$experiment_root/home/config"
export XDG_STATE_HOME="$experiment_root/home/state"
export XDG_CACHE_HOME="$experiment_root/home/cache"
export CLAUDE_CONFIG_DIR="$experiment_root/claude"
export CODEX_HOME="$experiment_root/codex"
export VISUAL="$experiment_root/editor-shim"
export EDITOR="$experiment_root/editor-shim"
cd "$experiment_root/work"
```

The harness homes and XDG paths above are isolated; HOME itself is unchanged. Use a clean terminal so unrelated inherited harness/provider variables, home-based discovery and IDE connections cannot select existing sessions or credentials. Before launch, Krishna must confirm the scratch auth and environment are appropriate. Do not print the environment or inspect credential files.

Claude, exact launch after approval:

```sh
claude --permission-mode plan --setting-sources '' --strict-mcp-config --settings '{"plansDirectory":"plans"}'
```

Type: `Plan only: propose a one-line documentation change. Put the literal CANOPY_BEFORE in the plan. Do not implement or run tools beyond writing the plan.`
When the plan approval UI offers editor access, press Ctrl+G. Record whether it invokes the shim on the plan or a prompt, whether the UI resumes only after exit, and whether it displays CANOPY_AFTER. Approval can permit implementation: do not select an implementation/bypass option. If testing approval awareness requires it, obtain approval for that step separately with a no-op plan. Compare the UI and model response, not just the file. Never infer acknowledgment from a changed preview.

Codex, exact launch after approval, in a separately prepared scratch root:

```sh
codex --no-daemon --sandbox read-only --ask-for-approval on-request
```

Use session help to identify Plan Mode and the current external-editor binding. Type `CANOPY_BEFORE` into the composer without submitting, invoke that binding, and verify CANOPY_AFTER returns to the draft without submitting. This tests **prompt handoff only**. Then enter Plan Mode using the UI, ask for a synthetic no-op plan, and inspect whether any plan-editor action exists. If absent, report absent; test the fallback by manually submitting an edited-plan message containing CANOPY_AFTER. Do not claim the prompt shim edits native plans.

Both CLIs: separately save an explicit scratch markdown plan, change it while no editor is open, and ask whether the CLI notices before a new user message. Lack of a visible notice is not proof the model never rereads it. Record file change, next request and response as separate observations.

Paste matrix: while idle, paste `CANOPY_BEFORE\nsecond line\n` as one bracketed paste, with no Enter; repeat with `/help` as literal leading text, an @path, emoji, and 1 KiB / 64 KiB / 1 MiB of synthetic text. Observe draft vs submission and any truncation. Repeat once while busy only if approved. Include malicious raw ESC [201~ in a local input-validation check, not in a live terminal. Never infer unlimited size from the largest successful sample.

Record version, exact action/key, argv, file path, before/after, exit timing, UI draft/plan, model-visible result and any error in a new synthetic-only report. All records must come from these throwaway sessions. Quit each CLI normally; no process-name kill, no daemon, no hc, no app. Keep scratch records for review; do not copy them into fixtures without checking that they contain only synthetic content.
