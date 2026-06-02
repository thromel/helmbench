# Claude Code Event Capture

HelmBench does not need raw Claude Code transcripts to evaluate behavior.
Instead, hooks or wrappers can append source-free events with `record-event`.

## Event File

Use one JSONL file per run:

```bash
export HELMBENCH_EVENTS=.helmbench/events/auth-redirect-001.jsonl
export HELMBENCH_TASK_ID=auth-redirect-001
```

The event file is safe to commit only if it contains no raw prompts, transcripts,
terminal logs, source snippets, secrets, or raw MCP payloads.

## Recording Events

Recommended file:

```bash
helmbench record-event \
  --events "$HELMBENCH_EVENTS" \
  --task-id "$HELMBENCH_TASK_ID" \
  --event-kind recommended-file \
  --path src/auth/session.ts \
  --observed-at-millis 100
```

File read:

```bash
helmbench record-event \
  --events "$HELMBENCH_EVENTS" \
  --task-id "$HELMBENCH_TASK_ID" \
  --event-kind file-read \
  --path src/auth/session.ts \
  --observed-at-millis 550
```

File edit:

```bash
helmbench record-event \
  --events "$HELMBENCH_EVENTS" \
  --task-id "$HELMBENCH_TASK_ID" \
  --event-kind file-edit \
  --path src/auth/session.ts \
  --observed-at-millis 2500
```

Validation command:

```bash
helmbench record-event \
  --events "$HELMBENCH_EVENTS" \
  --task-id "$HELMBENCH_TASK_ID" \
  --event-kind command \
  --command-class test \
  --command-hash cmd:targeted-auth-test \
  --touched-test tests/auth/session.test.ts \
  --exit-status 0 \
  --elapsed-millis 1800 \
  --observed-at-millis 4200
```

Usage:

```bash
helmbench record-event \
  --events "$HELMBENCH_EVENTS" \
  --task-id "$HELMBENCH_TASK_ID" \
  --event-kind usage \
  --token-estimate 4100 \
  --observed-at-millis 4300
```

Final status:

```bash
helmbench record-event \
  --events "$HELMBENCH_EVENTS" \
  --task-id "$HELMBENCH_TASK_ID" \
  --event-kind status \
  --status success \
  --observed-at-millis 4500
```

## Converting Events To Reports

```bash
helmbench claude-trace \
  --suite suites/example-auth-bugs.json \
  --events "$HELMBENCH_EVENTS" \
  --variant ctxhelm-mcp \
  --out-dir .helmbench/traces/claude-code

helmbench run \
  --suite suites/example-auth-bugs.json \
  --trace-dir .helmbench/traces/claude-code \
  --out .helmbench/reports/claude-code.json
```

## Hook Boundary

Claude Code hooks should emit only metadata:

- relative paths
- command classes
- command hashes
- touched test paths
- exit status
- timing
- token estimate
- task status

Do not emit:

- file contents
- model text
- raw tool payloads
- raw terminal output
- secrets
