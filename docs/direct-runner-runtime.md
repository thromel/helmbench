# HelmBench Doctor

Repo: `.`

Privacy: source-free reports enforced

Status: **ok**

## Required Checks

- git available: `ok`
- cargo available: `ok`
- repo is a git checkout: `ok`
- Cargo.toml exists: `ok`
- verification script exists: `ok`
- CI workflow exists: `ok`
- release workflow exists: `ok`
- example suite loads: `ok`
- example native report is source-free: `ok`
- example ctxhelm report is source-free: `ok`

## Optional Integrations

- ctxhelm (`ctxhelm`): `ok` (version:b1a4ea3d40893d30)
- claude-code (`claude`): `ok` (version:0b9961f23b21474c)
- codex (`codex`): `ok` (version:5eba67c1b011c7dd)

## Direct Runner Readiness

| Runner | Command | Available | Runtime preflight | Event contract | Capture stream | Raw output suppressed | Isolated clones |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `claude-run` | `claude` | yes | ok | yes | yes | yes | yes |
| `codex-run` | `codex` | yes | warn (cli_upgrade_required) | yes | yes | yes | yes |

## Observation Modes

- `record-event`: agent or hook appends validated source-free events; source-free `yes`, persists raw stream `no`
- `capture-stream`: structured stdout is parsed in memory and discarded; source-free `yes`, persists raw stream `no`
- `git-diff-inference`: edited files are inferred from git status after each isolated run; source-free `yes`, persists raw stream `no`
- `validation-command-summary`: success commands are stored by class/hash/exit status; source-free `yes`, persists raw stream `no`

## Supported Variants

- `Native`
- `NativeSearch`
- `CtxhelmPlan`
- `CtxhelmMcp`
- `CtxhelmPack`

## Privacy

- Source-free: `true`
- Raw source logged: `false`
- Raw prompts logged: `false`
- Raw transcripts logged: `false`
- Raw terminal logs logged: `false`
