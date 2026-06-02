# HelmBench Architecture

HelmBench is an evaluation harness, not another coding agent.

Its job is to answer:

```text
Did the agent inspect the right files, run the right validation, and solve the
task with less wasted context?
```

## Components

```text
Task Suite
  -> Agent Run / Trace Capture
  -> Source-Free Trace
  -> Metrics Engine
  -> Run Report
  -> Compare Report
  -> Dashboard / Markdown / JSON
```

## Source-Free Trace Model

A trace records only evaluation-safe metadata:

- task id
- agent name
- variant
- paths read, edited, and recommended
- command classes
- command hashes
- touched test paths
- exit status
- timing/count metadata
- privacy flags

It does not record raw code or model transcripts.

Generated reports and traces should be excluded from the repository context
engine used under test. This repo includes `.ctxhelmignore` so ctxhelm does not
rank HelmBench's own benchmark artifacts as task evidence.

## Variants

Initial variants:

- `native`
- `ctxhelm_plan`
- `ctxhelm_mcp`
- `ctxhelm_pack`
- `other`

The first MVP ingests manually produced or synthetic traces. Later adapters will
run agents and produce traces automatically. The current ctxhelm adapter already
generates source-free recommendation traces from `ctxhelm prepare-task`.

## Metrics

The core report computes:

- success rate
- recommendation precision
- recommendation recall
- total files read
- irrelevant file reads
- irrelevant read rate
- context precision
- edited-file recall
- validation coverage
- time to first relevant file
- tool call count
- token estimate

## Design Trade-Offs

### Why source-free first?

Because coding-agent telemetry can easily leak proprietary source, prompts,
terminal logs, secrets, and MCP payloads. HelmBench starts with paths, hashes,
counts, and classes so reports are safe to commit and share.

### Why trace ingestion before direct agent launching?

Direct agent adapters require brittle CLI/process instrumentation. Trace
ingestion makes the metric contract testable first, then adapters can target the
contract.

### Why add ctxhelm recommendation traces before direct Claude Code traces?

ctxhelm can already emit source-free `prepare-task` plans. Converting those
plans into HelmBench traces gives immediate measurement of recommendation
precision and recall, while direct agent adapters can be added without changing
the report contract.

### Why not pass/fail only?

Pass rate alone hides navigation quality. HelmBench measures how the agent got
there: whether it read the right files, touched relevant tests, and wasted less
context.
