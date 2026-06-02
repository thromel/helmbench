# HelmBench

**Measure how coding agents navigate, validate, and succeed.**

HelmBench is a local, source-free benchmark and observability harness for AI
coding agents. It measures whether an agent found the right files, avoided
wasted context, ran useful validation, and solved the task.

HelmBench is designed as a companion to `ctxhelm`:

- `ctxhelm` improves agent context.
- `HelmBench` proves whether that context helped.

## MVP status

This repository currently implements the first MVP slice:

- task suite schema
- source-free trace schema
- suite and trace validation
- run report generation from trace JSON
- report comparison
- Markdown and JSON output
- privacy checks that reject raw-source/raw-transcript traces
- `ctxhelm-trace` adapter that calls `ctxhelm prepare-task` and emits
  source-free recommendation traces
- `claude-trace` importer that converts sanitized Claude Code event JSONL into
  source-free read/edit/test traces
- `local-run` runner that clones a git repo per task, runs a source-free adapter
  command, executes validation, infers edited files from git diff, and emits
  trace JSON
- recommendation precision and recall metrics for context-plan evaluation

It does **not** yet launch Claude Code, Codex, Cursor, or other agents directly.
The current runner ingests source-free traces, can generate ctxhelm plan traces,
can convert source-free Claude Code events, and can execute explicit local
adapter commands. Direct Claude/Codex process adapters come next.

## Quickstart

Create an example suite:

```bash
cargo run -- init-suite --out suites/example-auth-bugs.json
```

Validate a suite:

```bash
cargo run -- validate-suite suites/example-auth-bugs.json
```

Build reports from source-free traces:

```bash
cargo run -- run \
  --suite suites/example-auth-bugs.json \
  --trace-dir examples/traces/native \
  --out reports/example-native.json

cargo run -- run \
  --suite suites/example-auth-bugs.json \
  --trace-dir examples/traces/ctxhelm-mcp \
  --out reports/example-ctxhelm.json
```

Compare reports:

```bash
cargo run -- compare \
  --base reports/example-native.json \
  --head reports/example-ctxhelm.json \
  --format markdown
```

Render a static source-free dashboard:

```bash
cargo run -- dashboard \
  --report reports/example-native.json \
  --report reports/example-ctxhelm.json \
  --report reports/example-claude-code.json \
  --out docs/example-dashboard.html
```

Convert sanitized Claude Code events into traces:

```bash
cargo run -- claude-trace \
  --suite suites/example-auth-bugs.json \
  --events examples/events/claude-code/auth-redirect-001.jsonl \
  --variant ctxhelm-mcp \
  --out-dir examples/traces/claude-code

cargo run -- run \
  --suite suites/example-auth-bugs.json \
  --trace-dir examples/traces/claude-code \
  --out reports/example-claude-code.json
```

Generate a ctxhelm recommendation trace over the HelmBench repo:

```bash
cargo run -- ctxhelm-trace \
  --suite suites/helmbench-meta.json \
  --repo . \
  --ctxhelm-bin /path/to/ctxhelm \
  --mode feature \
  --target-agent claude-code \
  --out-dir examples/traces/ctxhelm-plan-meta

cargo run -- run \
  --suite suites/helmbench-meta.json \
  --trace-dir examples/traces/ctxhelm-plan-meta \
  --out reports/example-ctxhelm-plan-meta.json
```

Current checked-in ctxhelm-plan example over `suites/helmbench-meta.json`:

```text
Recommendation recall:    100.0%
Recommendation precision: 25.0%
Recommended files:        8
Expected files found:     2
Validation coverage:      0.0%  # plan-only trace; no agent/test execution
```

Run the checked-in local-run smoke suite:

```bash
cargo build
chmod +x scripts/demo-local-agent.sh

cargo run -- local-run \
  --suite suites/local-run-smoke.json \
  --repo . \
  --agent demo-local-agent \
  --variant native \
  --adapter-command "HELMBENCH_BIN=$(pwd)/target/debug/helmbench sh scripts/demo-local-agent.sh" \
  --out-dir traces/local-run-smoke

cargo run -- run \
  --suite suites/local-run-smoke.json \
  --trace-dir traces/local-run-smoke \
  --out reports/local-run-smoke.json
```

`local-run` clones the repo into `.helmbench/workdirs/<task-id>` by default and
removes the workdir after writing the trace unless `--keep-workdirs` is set.

The repo includes `.ctxhelmignore` so generated reports and traces do not
pollute ctxhelm recommendation quality.

## What HelmBench Measures

| Metric | Meaning |
| --- | --- |
| Task success | Whether the trace reports success, failure, or skip. |
| Files read | Source-free paths the agent inspected. |
| Irrelevant file reads | Files read that were not in the expected evidence set. |
| Recommendation precision | Expected evidence paths divided by recommended paths. |
| Recommendation recall | Recommended expected evidence divided by all expected evidence. |
| Context precision | Relevant reads divided by all reads. |
| Edited-file recall | Expected target files edited divided by expected files. |
| Validation coverage | Whether expected tests or validation command classes were run successfully. |
| Time to first relevant file | How quickly the agent reached a target file. |
| Tool/token cost | Source-free cost proxies from trace metadata. |

## Source-Free Claude Event JSONL

`claude-trace` accepts newline-delimited JSON events such as:

```json
{"schemaVersion":1,"taskId":"auth-redirect-001","eventKind":"file_read","path":"src/auth/session.ts","observedAtMillis":550}
```

Supported `eventKind` values:

- `recommended_file`
- `file_read`
- `file_edit`
- `command`
- `usage`
- `status`

These events are intended to be produced by Claude Code hooks or wrappers
without storing raw transcripts, raw tool payloads, raw terminal logs, or source
snippets.

For hook-friendly commands, see [Claude Code Event Capture](docs/claude-code-events.md).

## Privacy Contract

HelmBench reports are source-free by default. Trace files may contain:

- relative paths
- path hashes
- command classes
- command hashes
- exit statuses
- timings
- counts
- task ids
- agent variants

Trace files must not contain:

- raw source code
- raw prompts beyond task-suite prompts
- raw model transcripts
- raw terminal logs
- secrets

If a trace sets any privacy flag indicating raw source, prompt, transcript, or
terminal logs were captured, HelmBench rejects it.

## Architecture

```text
helmbench-core
  task suite schema
  source-free trace schema
  metrics
  privacy checks

helmbench-cli
  init-suite
  validate-suite
  run
  ctxhelm-trace
  claude-trace
  local-run
  record-event
  compare
  dashboard
  doctor

adapters
  ctxhelm prepare-task trace generation
  Claude Code source-free event import
  explicit local adapter command runner

future direct-agent adapters
  claude-code
  codex
  cursor
```

## Next Milestones

1. Add a Claude Code adapter that drives `local-run` with Claude Code commands
   and source-free hooks.
2. Extend the ctxhelm adapter from `prepare-task` plans to MCP resource reads
   and pack usage without raw MCP payloads.
3. Add public benchmark suites over small reproducible repositories.
4. Add Agent Diff Autopsy for agent-created PRs.
