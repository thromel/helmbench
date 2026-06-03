# Local Runner

`local-run` executes reproducible benchmark tasks without storing raw source,
raw transcripts, raw terminal logs, or raw agent output.

## Flow

```text
suite task
  -> git clone target repo into .helmbench/workdirs/<task-id>
  -> run optional setup commands
  -> run adapter command with HELMBENCH_* environment
  -> adapter appends source-free events
  -> infer edited paths from git status
  -> run successCommand
  -> append final status event
  -> convert events to trace JSON
```

## Adapter Environment

The adapter command receives:

- `HELMBENCH_TASK_ID`
- `HELMBENCH_TASK_PROMPT`
- `HELMBENCH_REPO`
- `HELMBENCH_EVENTS`
- `HELMBENCH_SUITE_NAME`

Adapters should call `helmbench record-event` to emit only metadata:

```bash
helmbench record-event \
  --events "$HELMBENCH_EVENTS" \
  --task-id "$HELMBENCH_TASK_ID" \
  --event-kind file-read \
  --path src/auth/session.ts
```

## Command

```bash
helmbench local-run \
  --suite suites/local-run-smoke.json \
  --repo . \
  --agent demo-local-agent \
  --variant native \
  --adapter-command "HELMBENCH_BIN=$(pwd)/target/debug/helmbench sh scripts/demo-local-agent.sh" \
  --out-dir traces/local-run-smoke
```

Use `--variant native` for an agent-alone baseline and `--variant
native-search` when the adapter exercises the agent's own repository search or
built-in context discovery without ctxhelm.

Use `--keep-workdirs` to preserve isolated clones for debugging.

## Privacy Boundary

`local-run` writes source-free traces. It records:

- relative paths;
- command class;
- command hash;
- touched test paths;
- exit status;
- timing metadata;
- final status.

It does not record:

- file contents;
- raw prompts in trace files;
- raw model transcripts;
- raw terminal output;
- raw MCP payloads;
- secrets.
