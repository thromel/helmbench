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

It does **not** yet launch Claude Code, Codex, Cursor, or other agents directly.
The current runner ingests source-free traces. Agent adapters come next.

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

## What HelmBench Measures

| Metric | Meaning |
| --- | --- |
| Task success | Whether the trace reports success, failure, or skip. |
| Files read | Source-free paths the agent inspected. |
| Irrelevant file reads | Files read that were not in the expected evidence set. |
| Context precision | Relevant reads divided by all reads. |
| Edited-file recall | Expected target files edited divided by expected files. |
| Validation coverage | Whether expected tests or validation command classes were run successfully. |
| Time to first relevant file | How quickly the agent reached a target file. |
| Tool/token cost | Source-free cost proxies from trace metadata. |

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
  compare
  doctor

future adapters
  claude-code
  codex
  cursor
  ctxhelm
```

## Next Milestones

1. Add a Claude Code adapter that records source-free file-read/edit/test
   summaries.
2. Add a ctxhelm adapter that captures `prepare_task`, MCP resource reads, and
   pack usage without raw MCP payloads.
3. Add a static HTML dashboard from generated report JSON.
4. Add public benchmark suites over small reproducible repositories.
5. Add Agent Diff Autopsy for agent-created PRs.
