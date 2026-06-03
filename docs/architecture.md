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

`init-demo-repo` provides a reproducible fixture lane for this flow. It creates
a tiny git repository, writes a matching task suite, and includes a source-free
demo agent script so the full runner/report/autopsy/dashboard pipeline can be
tested without external agents or network access.

`run-matrix` is the main orchestration command for real eval runs. It executes
one baseline plus one or more local adapter variants over the same suite, then
writes per-run reports, pairwise comparisons, a benchmark summary, quality
gate, suite-health report, per-run autopsies, privacy report, reproduction
guide, dashboard, and verifiable evidence bundle.

`suite-health` is the preflight check for custom benchmark suites. It verifies
that expected files and tests exist in the target repo, every task has a
validation command, git metadata is readable, and the checkout is clean unless
explicitly allowed. The resulting health report is source-free and can be
included in evidence bundles.

`run-matrix` runs the same suite-health check before launching any row. The
health JSON is saved under `reports/suite-health.json`, referenced from
`matrix-manifest.json`, and copied into the evidence bundle as `health.json` so
published matrix artifacts carry their own source-free preflight proof.

The matrix manifest also carries reproducibility provenance: HelmBench version,
suite hash, repo HEAD, dirty-checkout flag, setup-command hashes, and per-row
adapter/ctxhelm configuration hashes. It also records source-free artifact
digests for matrix-owned reports, pairwise comparisons, traces, docs, and
evidence manifests. This
lets reviewers compare, rerun, and tamper-check a benchmark without storing raw
commands, prompts, terminal logs, transcripts, ctxhelm pack sections, or source
content.

`docs/reproduction.md` renders that provenance into reviewer-facing verification
and rerun instructions. It uses placeholders and hashes rather than raw local
adapter/setup commands.

`reports/privacy-report.json` and `docs/privacy-report.md` summarize the
source-free privacy contract for each matrix. They list recorded metadata,
forbidden raw data classes, safeguards, and per-run source-free trace/report
checks. `verify-matrix` parses the JSON privacy report, so a published matrix
cannot pass verification if the report claims raw source, prompts, transcripts,
terminal logs, or non-source-free traces were recorded.

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

Published JSON Schema contracts live under `schemas/` and can be emitted with
`helmbench schema --kind <kind>`. These schemas cover suites, traces, events,
run reports, pairwise compare reports, benchmark summaries, quality gates,
run-matrix configs, matrix history, matrix manifests, doctor reports,
trace/diff autopsies, suite-health reports, evidence bundles, and matrix
privacy reports. They are intended for adapters, CI checks, and downstream
dashboards that need to validate HelmBench artifacts without depending on Rust
internals.

Run reports aggregate command observations into source-free command-class
summaries. The report stores counts for test, build, lint, typecheck, other,
successful, and failed commands so validation behavior can be audited without
storing raw terminal text.

Generated reports and traces should be excluded from the repository context
engine used under test. This repo includes `.ctxhelmignore` so ctxhelm does not
rank HelmBench's own benchmark artifacts as task evidence.

## Variants

Initial variants:

- `native`
- `native_search`
- `ctxhelm_plan`
- `ctxhelm_mcp`
- `ctxhelm_pack`
- `other`

Use `native` for an agent-alone baseline and `native_search` for an agent run
that uses the agent's own repository search or built-in context discovery
without ctxhelm.

The first MVP ingests manually produced or synthetic traces. Later adapters will
run agents and produce traces automatically. The current ctxhelm adapter already
generates source-free recommendation traces from `ctxhelm prepare-task`.
The current Claude Code path imports source-free JSONL events produced by hooks
or wrappers; it does not require raw transcripts.

`ctxhelm-run` combines ctxhelm context generation with the local runner. It
calls `ctxhelm prepare-task` inside each isolated task clone and records returned
target files/tests as source-free `recommended-file` events. When `--pack` is
set, it calls `ctxhelm get-pack --format json` but persists only source-free pack
metadata such as token estimates and command hashes. Pack sections and snippets
are discarded.

The current `local-run` path executes an explicit adapter command inside an
isolated clone of the target git repo. It passes source-free environment
variables such as `HELMBENCH_TASK_ID`, `HELMBENCH_REPO`, and
`HELMBENCH_EVENTS`, then:

1. lets the adapter append source-free events with `record-event`;
2. infers edited files from `git status --short`;
3. runs the task `successCommand` when present;
4. records command class, command hash, exit status, and final status; and
5. writes a normal HelmBench trace JSON.

`local-run` itself is not agent-specific. It is the isolation and observation
foundation the Claude/Codex launch presets use.

`run-matrix` uses the same local-run foundation for every row in the matrix, so
baseline and variant results share identical clone, validation, trace, and
privacy behavior.

When a matrix row sets `ctxhelm=true`, HelmBench calls ctxhelm before the
adapter command and records the returned target files/tests as source-free
recommendation events. With `pack=true`, it also calls `ctxhelm get-pack` and
records only source-free pack metadata such as token estimates.

`claude-run` and `codex-run` are thin launch presets over `local-run`. They
start the installed agent CLI non-interactively, suppress raw stdout/stderr, and
inject instructions that ask the agent to emit source-free `record-event` calls.
HelmBench still owns edited-file inference and validation recording.

`stream-trace` covers agents that can emit structured JSONL tool streams. It
parses those streams in memory, extracts safe file-read/file-edit/command
metadata, hashes command text, and writes normal source-free HelmBench traces.
Raw streams should be treated as local temporary artifacts.

`--capture-stream` applies the same parser during `local-run`, `ctxhelm-run`,
`claude-run`, `codex-run`, and `run-matrix` rows. It captures stdout with a
bounded in-memory buffer, converts structured tool metadata into source-free
events, and discards the raw stream. This provides better direct-agent
observation without persisting transcripts.

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
- tool/token cost per successful task

## Benchmark Summary

`compare` answers one pairwise question. `benchmark-summary` answers the larger
evaluation question: given a baseline, how did one or more variants perform on
the same suite?

Both commands require comparable reports: the suite name must match and the
task ID set must be identical. Partial reports are still valid standalone
artifacts, but HelmBench will not publish deltas for mismatched task coverage.

The summary artifact includes:

- one baseline run summary;
- one row per source-free run report;
- 95% Wilson score intervals for binary per-task rates such as task success
  and validation coverage;
- a low-sample warning when the suite has fewer than 10 tasks, so directional
  early results are not presented as high-confidence proof;
- source-free failure taxonomy counts for failed/skipped tasks, validation
  gaps, missing relevant reads, missing expected edits, recommendation misses,
  and tasks with irrelevant reads;
- source-free command mix counts for test/build/lint/typecheck/other commands
  plus successful and failed command counts;
- average time to first relevant file when traces include timing metadata;
- deltas from baseline for success, validation, recommendation recall, context
  precision, edited-file recall, irrelevant reads, total tool/token cost, and
  tool/token cost per successful task;
- a simple verdict per variant: `improved`, `regressed`, `mixed`, or
  `no_change`.

This is the artifact to publish when showing whether ctxhelm made an agent
better, cheaper, or less wasteful across a benchmark suite.

## Matrix History

`matrix-history` compares two or more verified `run-matrix` output directories
over time. It verifies every matrix, reads each matrix's source-free
`reports/benchmark-summary.json`, requires the suite and run names to match,
and reports first-to-last deltas for success, validation coverage,
recommendation recall, context precision, edited-file recall, irrelevant reads,
tool calls, token estimates, and average time to first relevant file when
timing is available.

The Markdown, JSON, and static HTML reports intentionally do not echo absolute
matrix paths. They use source-free sequence labels so a published history
artifact can show trend evidence without leaking local checkout locations.

## Quality Gate

`quality-gate` turns a benchmark summary into a CI decision. It reads a
source-free `benchmark-summary.json`, checks each variant delta against
thresholds, writes an optional JSON or Markdown gate report, and exits non-zero
if any check fails.

Default thresholds require no regression in success rate, validation coverage,
recommendation recall, context precision, edited-file recall, or irrelevant read
rate. Optional thresholds can require a minimum task count and cap tool-call,
token, tool/token-per-success, and average time to first relevant file deltas.
Timing checks are skipped with a warning when either side of a comparison lacks
timing metadata. Per-success cost checks are skipped with a warning when either
side has zero successful tasks.
Quality-gate reports preserve benchmark-summary confidence warnings; a gate can
pass while still warning that the suite is too small for strong claims unless
`minTaskCount` or `--min-task-count` turns that confidence floor into a failing
check.

This is the command to use when ctxhelm changes should be blocked unless they
preserve or improve agent behavior on a suite.

## Evidence Bundle

`evidence-bundle` packages the source-free proof for a benchmark run. It
validates the suite and reports, optionally validates a suite health report,
generates JSON and Markdown benchmark summaries, copies the artifacts into a
bundle directory, and writes `manifest.json`.

`verify-bundle` is the inverse proof check. It reads `manifest.json`, rejects
non-source-free privacy flags, validates every artifact path as a safe relative
path, rejects duplicate artifact paths, recomputes byte counts and content
hashes, and fails on any mismatch. This lets a reviewer or CI job validate a
published bundle without access to the source repository.

The manifest records only:

- suite name;
- baseline agent and variant;
- relative artifact paths;
- source filenames, not absolute source paths;
- byte counts;
- content hashes;
- source-free check status.

This gives ctxhelm and HelmBench a repeatable proof artifact: reviewers can
inspect exactly which suite, reports, summary, and health metadata supported a
claim without reading source files or model transcripts.

## Dashboard

`dashboard` renders one or more source-free run reports into a static HTML file.
It uses the same privacy gate as JSON and Markdown report readers: if a report
claims raw source, raw prompts, raw transcripts, or raw terminal logs were
captured, dashboard rendering fails.

The dashboard intentionally embeds no raw source, no JavaScript, and no remote
assets. It is safe to publish as an example artifact when the input reports are
source-free.

## Autopsy

`autopsy` diagnoses agent behavior from a suite plus trace directory. It is
designed for post-run review of agent-created patches without reading source or
transcripts.

Autopsy flags:

- failed tasks;
- validation gaps;
- edits outside expected files;
- edited files with no recorded read event;
- expected files that were neither read nor edited.

This makes trace files useful for reviewer-facing questions such as "what did
the agent change without inspecting?" and "which validation did it skip?"

`diff-autopsy` covers the weaker but common case where only a git branch,
worktree diff, or GitHub PR changed-file list is available. It collects changed
paths with `git status --short`, `git diff --name-only`, or
`gh pr diff --name-only`, compares them against one suite task's expected
source/test paths, and reports overbroad changes or missing expected path
changes without reading patch hunks.

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

### Why discard ctxhelm pack sections?

`ctxhelm get-pack` can include target snippets and other source-bearing context.
HelmBench is an evaluation harness with a source-free report contract, so
`ctxhelm-run --pack` records only metadata such as token estimates and command
hashes. The benchmark can measure whether ctxhelm reduced waste without storing
the context payload itself.

### Why import Claude Code events before launching Claude Code directly?

Claude Code process automation and hook integration can vary by local
installation and permissions. A source-free event importer gives us the durable
contract first: hooks or wrappers can emit `file_read`, `file_edit`, `command`,
`usage`, and `status` events, and HelmBench can score them without storing raw
model output.

### Why add local-run before direct Claude/Codex adapters?

Direct agent automation has two separate problems: process orchestration and
behavior observation. `local-run` solves the stable part first: per-task repo
isolation, event-file plumbing, validation command execution, edited-file
detection, and trace emission. Claude Code and Codex adapters can now focus on
starting the agent and emitting source-free events instead of each reinventing
runner mechanics.

### Why launch presets before transcript parsers?

Raw agent transcripts are high-leakage artifacts: they can contain source,
prompts, terminal output, MCP payloads, and secrets. The first direct adapters
therefore avoid transcript parsing. They launch the agent, ask it to emit
source-free events, infer edits from git status, and record validation results.
Later adapters can add deeper tool-stream capture only if that stream can be
sanitized before it is persisted.

### Why import structured streams?

Some agents expose tool-use streams but cannot be forced to call
`record-event`. `stream-trace` gives HelmBench a middle path: consume structured
tool metadata locally, persist only paths, command classes, command hashes, test
touches, and statuses, and discard raw tool payloads.

### Why add capture-stream mode?

Some direct agent runs can emit machine-readable tool events on stdout but
cannot conveniently call `record-event`. Capture mode lets HelmBench observe
those runs without storing stdout. It is opt-in because raw stdout may contain
source-bearing content if the agent is not configured carefully.

### Why not pass/fail only?

Pass rate alone hides navigation quality. HelmBench measures how the agent got
there: whether it read the right files, touched relevant tests, and wasted less
context.
