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

## Claude Code

```bash
helmbench claude-run \
  --suite suites/local-run-smoke.json \
  --repo . \
  --dangerously-skip-permissions \
  --out-dir traces/claude-run
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
