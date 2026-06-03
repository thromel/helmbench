# Direct Agent Runs

HelmBench can launch Claude Code and Codex through `local-run` presets.

The presets are intentionally thin:

- clone the target repo per task;
- pass `HELMBENCH_TASK_ID`, `HELMBENCH_TASK_PROMPT`, `HELMBENCH_REPO`, and
  `HELMBENCH_EVENTS`;
- suppress raw agent stdout/stderr;
- ask the agent to emit source-free `record-event` calls;
- optionally capture structured stdout JSONL, convert it to source-free events,
  and discard the raw stream;
- infer edited files from `git status`;
- run the task `successCommand`;
- write normal HelmBench trace JSON.

They do not parse or persist raw transcripts.

## Preflight

Before running direct agent presets, generate a source-free readiness report:

```bash
helmbench doctor \
  --repo . \
  --format json \
  --out /tmp/helmbench-doctor.json
```

The report includes required HelmBench checks, optional `ctxhelm`/Claude/Codex
availability, direct-runner readiness, observation modes, and privacy flags. It
stores version hashes, not raw version strings.

## Claude Code

```bash
helmbench claude-run \
  --suite suites/local-run-smoke.json \
  --repo . \
  --dangerously-skip-permissions \
  --out-dir traces/claude-run
```

A checked-in source-free smoke artifact from this flow is available at
[`reports/claude-real-smoke.json`](../reports/claude-real-smoke.json), with a
Markdown rendering at [`docs/claude-real-smoke.md`](claude-real-smoke.md). It is
an observed one-task launch proof, not a statistically meaningful benchmark.
The tracked fixture is healthy at rest; `suites/local-run-smoke.json` uses
task-level `setupCommands` to seed the failing state inside the isolated clone
before Claude Code runs.

Prove the smoke task is outcome-ready before launching the agent:

```bash
helmbench suite-health \
  --suite suites/local-run-smoke.json \
  --repo . \
  --check-success-commands \
  --allow-dirty \
  --out /tmp/local-run-smoke-health.json
```

Options:

- `--claude-bin`: path to the Claude Code CLI.
- `--model`: Claude model or alias.
- `--claude-arg`: extra argument passed to Claude Code. Repeatable.
- `--dangerously-skip-permissions`: pass Claude Code's non-interactive
  permission bypass flag. Use only with isolated benchmark clones.
- `--capture-stream`: capture stdout as structured JSONL tool metadata,
  convert it to source-free events, and discard the raw stream.
- `--keep-workdirs`: preserve cloned task workdirs for debugging.

For comparative benchmark runs, use the same launcher through `run-matrix`:

```bash
helmbench run-matrix \
  --suite suites/refactoring-miner-public.json \
  --repo /tmp/RefactoringMiner \
  --baseline "name=native,agent=claude-code,variant=native,preset=claude-code,dangerously_skip_permissions=true" \
  --head "name=ctxhelm,agent=claude-code,variant=ctxhelm_mcp,ctxhelm=true,mode=bug-fix,target_agent=claude-code,pack=true,preset=claude-code,dangerously_skip_permissions=true" \
  --out-dir /tmp/refactoringminer-matrix \
  --force
```

## Codex

```bash
helmbench codex-run \
  --suite suites/local-run-smoke.json \
  --repo . \
  --out-dir traces/codex-run
```

By default HelmBench invokes `codex exec --full-auto --cd "$HELMBENCH_REPO"`.

Options:

- `--codex-bin`: path to the Codex CLI.
- `--model`: Codex model.
- `--codex-arg`: extra argument passed to `codex exec`. Repeatable.
- `--dangerously-bypass-approvals-and-sandbox`: pass Codex's unrestricted mode.
  Use only with externally sandboxed benchmark clones.
- `--capture-stream`: capture stdout as structured JSONL tool metadata,
  convert it to source-free events, and discard the raw stream.
- `--keep-workdirs`: preserve cloned task workdirs for debugging.

`run-matrix` also supports `preset=codex` rows with optional `bin`, `model`,
`args`, and `dangerously_bypass_approvals_and_sandbox` fields.

## Telemetry Limits

Direct launch presets can always record:

- edited files, inferred from git status;
- validation command class/hash/exit status;
- final task status;
- elapsed timing.

They can record reads and recommendations in two ways:

- the agent follows the injected `record-event` instruction;
- `--capture-stream` is enabled and the agent emits structured JSONL tool
  metadata on stdout.

This is a deliberate privacy trade-off: HelmBench does not scrape raw
transcripts just to recover richer telemetry. In capture mode, stdout is parsed
in memory with a bounded buffer and is not written to disk.
