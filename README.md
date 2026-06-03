# HelmBench

**Measure how coding agents navigate, validate, and succeed.**

HelmBench is a local, source-free benchmark and observability harness for AI
coding agents. It measures whether an agent found the right files, avoided
wasted context, ran useful validation, and solved the task.

HelmBench is designed as a companion to `ctxhelm`:

- `ctxhelm` improves agent context.
- `HelmBench` proves whether that context helped.

## Proof Snapshot

The checked-in source-free smoke proof shows `ctxhelm_mcp` improving the
example Claude Code run from `0.0%` to `100.0%` task success, reducing
irrelevant reads from `75.0%` to `33.3%`, and cutting time to first relevant
file from `2600 ms` to `600 ms`.

See [HelmBench Launch Proof](docs/launch-proof.md) for the artifact-backed
summary, dashboard, privacy contract, and regeneration commands. This is a
1-task smoke proof with an explicit low-sample warning; larger public-suite
proofs should use `run-matrix` on the RefactoringMiner, Flask, ripgrep, or
Express presets.

The checked-in [RefactoringMiner public recommendation proof](docs/refactoringminer-public-proof.md)
runs `ctxhelm prepare-task` over a healthy 10-task public suite and records
`61.3%` average recommendation recall with source-free artifacts.
Its checked outcome-readiness report shows the suite is not yet valid for
task-success claims because validation can pass before any agent change.

The checked-in [real Claude Code smoke report](docs/claude-real-smoke.md)
launches Claude Code through `claude-run` on the local smoke suite and records
`100.0%` task success with one relevant file read, zero irrelevant reads, and
source-free privacy flags.

## Current status

This repository currently implements the core HelmBench workflow:

- task suite schema
- source-free trace schema
- published JSON Schema contracts for task suites, agent events, traces, run
  reports, compare reports, benchmark summaries, quality gates, matrix
  configs, matrix history, matrix manifests, doctor reports, autopsy reports,
  suite-health reports, evidence bundles, and matrix privacy reports
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
- `claude-run` and `codex-run` launch presets that run agents non-interactively
  inside isolated clones using the same source-free event contract
- checked-in real Claude Code smoke proof generated through the direct launch
  preset, with raw stdout/stderr suppressed and only source-free telemetry
  persisted
- `stream-trace` importer that converts structured Claude/Codex-style JSONL
  tool streams into source-free traces without storing command text or tool
  payloads
- opt-in `--capture-stream` mode for local, ctxhelm-guided, Claude, Codex, and
  matrix runs that parses structured stdout in memory and persists only
  source-free events
- `init-public-suite` generator for verified public-repo benchmark suites,
  currently including a 10-task RefactoringMiner suite plus Flask, ripgrep,
  and Express presets
- `init-public-matrix` generator for repeatable real-agent public-suite matrix
  configs, with source-free fixture health checked before the config is written
- `suite-health` checks any source-free suite against a local git repo before
  benchmark results are trusted
- opt-in `suite-health --check-success-commands` gate that runs validation
  commands in isolated clones and flags suites whose success commands already
  pass before any agent changes
- task-level `setupCommands` for seeding per-task failures inside isolated
  clones before ctxhelm, agents, and validation commands run
- `demo-run` one-command deterministic demo pipeline with reports, dashboard,
  privacy report, quality gate, and evidence bundle
- `run-matrix` benchmark coordinator that runs one baseline plus one or more
  local adapter variants and emits reports, comparisons, dashboard, quality
  gate, suite-health, per-run autopsies, privacy report, reproduction guide,
  evidence bundle artifacts, and source-free reproducibility provenance
- first-class `run-matrix` row presets for Claude Code and Codex, so real
  agent matrices can inject the source-free event contract without hand-written
  adapter commands
- `matrix-history` longitudinal comparison and static HTML trend dashboards for
  verified run-matrix outputs
- `diff-autopsy` reviewer report that compares a git worktree, branch diff, or
  GitHub PR changed-file list against one source-free benchmark task without
  reading patch contents
- GitHub release workflow with packaged binaries, SHA-256 checksums, and
  provenance attestations
- `benchmark-summary` reports that compare one baseline against multiple
  source-free variant reports, including confidence intervals and low-sample
  warnings plus source-free failure taxonomy counts
- `evidence-bundle` packaging for source-free suites, health reports, run
  reports, benchmark summaries, and artifact hashes
- `verify-bundle` validation for source-free evidence manifests, safe artifact
  paths, byte counts, and content hashes
- `quality-gate` checks that fail CI when benchmark-summary deltas regress
- recommendation precision and recall metrics for context-plan evaluation
- source-free command-class summaries for test/build/lint/typecheck/other
  validation behavior
- source-free tool/token cost-per-success metrics for practical efficiency
  comparisons

It does **not** yet parse raw Claude Code, Codex, Cursor, or other agent
transcripts. The current runner ingests source-free traces, can generate ctxhelm
plan traces, can convert source-free Claude Code events, can execute explicit
local adapter commands, and can launch Claude/Codex with source-free event
instructions.

## Quickstart

Run the local verification contract:

```bash
./scripts/verify.sh
```

This runs formatting, tests, clippy, CLI help checks, the reproducible demo
benchmark, dashboard generation, benchmark summary generation, evidence bundle
generation and verification, release workflow/docs consistency checks, and
whitespace checks. GitHub Actions runs the same script.

Check local prerequisites and optional agent integrations:

```bash
cargo run -- doctor --repo .

cargo run -- doctor \
  --repo . \
  --format json \
  --out /tmp/helmbench-doctor.json
```

The JSON doctor report is source-free. It records required checks, optional
`ctxhelm`/Claude/Codex availability, direct-runner readiness, supported
observation modes, and privacy flags without storing raw version strings.

Install from source:

```bash
cargo install --git https://github.com/thromel/helmbench --locked
```

Release tarball and provenance verification instructions are in
[Install HelmBench](docs/install.md).

Create an example suite:

```bash
cargo run -- init-suite --out suites/example-auth-bugs.json
```

Write a JSON Schema contract:

```bash
cargo run -- schema --kind agent-trace --out /tmp/agent-trace.schema.json
```

Write every published JSON Schema contract:

```bash
cargo run -- schema --all --out-dir /tmp/helmbench-schemas
```

Create a reproducible demo benchmark repo plus matching suite:

```bash
cargo run -- init-demo-repo \
  --repo-out /tmp/helmbench-demo-repo \
  --suite-out /tmp/helmbench-demo-suite.json \
  --force
```

The generated demo repo is healthy at rest. Its suite uses task-level
`setupCommands` to seed failing states inside isolated task clones, so
`suite-health --check-success-commands` can prove outcome readiness before an
agent run.

Run the full deterministic demo pipeline:

```bash
cargo run -- demo-run \
  --out-dir /tmp/helmbench-demo-run \
  --force
```

Run a baseline-vs-variant matrix and write publishable artifacts:

```bash
cargo run -- run-matrix \
  --suite /tmp/helmbench-demo-suite.json \
  --repo /tmp/helmbench-demo-repo \
  --out-dir /tmp/helmbench-matrix \
  --baseline "name=native,agent=demo-baseline,variant=native" \
  --head "name=native-search,agent=demo-search,variant=native_search,preset=claude-code,bin=scripts/demo-local-agent.sh,dangerously_skip_permissions=true" \
  --head "name=guided,agent=demo-guided,variant=ctxhelm_mcp,ctxhelm=true,ctxhelm_bin=scripts/demo-ctxhelm.sh,pack=true,pack_budget=brief,preset=claude-code,bin=scripts/demo-local-agent.sh,dangerously_skip_permissions=true" \
  --force

cargo run -- verify-bundle \
  --bundle /tmp/helmbench-matrix/evidence

cargo run -- matrix-history \
  --matrix /tmp/helmbench-matrix-week-1 \
  --matrix /tmp/helmbench-matrix-week-2 \
  --out /tmp/helmbench-matrix-history.md

cargo run -- matrix-history \
  --matrix /tmp/helmbench-matrix-week-1 \
  --matrix /tmp/helmbench-matrix-week-2 \
  --format html \
  --out /tmp/helmbench-matrix-history.html
```

For repeatable runs, put the matrix definition in JSON:

```bash
HELMBENCH_BIN=$(pwd)/target/debug/helmbench \
  cargo run -- validate-matrix \
    --config suites/demo-matrix.json

HELMBENCH_BIN=$(pwd)/target/debug/helmbench \
  cargo run -- run-matrix \
    --config suites/demo-matrix.json \
    --force
```

The checked-in `suites/demo-matrix.json` is self-contained for a fresh
HelmBench checkout. It runs the tracked `local-run-smoke` suite against this
repo, compares native, native-search, and ctxhelm-guided rows, and uses
`scripts/demo-local-agent.sh` through the `claude-code` matrix preset plus
`scripts/demo-ctxhelm.sh` as deterministic source-free shims. The smoke suite
keeps the fixture healthy at rest and uses task-level `setupCommands` to seed
the failing state inside isolated clones before each agent row runs.

Generate a real-agent public matrix config after creating or checking out a
public suite fixture:

```bash
cargo run -- init-public-matrix \
  --preset refactoring-miner \
  --repo /tmp/RefactoringMiner \
  --suite suites/refactoring-miner-public.json \
  --out /tmp/refactoring-miner-matrix.json \
  --out-dir /tmp/refactoring-miner-matrix \
  --agent-preset claude-code \
  --dangerously-skip-permissions \
  --ctxhelm-bin ctxhelm \
  --pack \
  --force

cargo run -- validate-matrix \
  --config /tmp/refactoring-miner-matrix.json

cargo run -- run-matrix \
  --config /tmp/refactoring-miner-matrix.json \
  --force
```

Every successful matrix run writes `matrix-manifest.json`, a source-free
top-level index of run labels, suite-health, report paths,
dashboard/evidence artifacts, quality-gate status, and evidence verification
status.
Matrix configs can include a `qualityGate` block, including an optional minimum
task count and caps for average time-to-first-relevant-file delta.
Use `verify-matrix --matrix <out-dir>` to validate the manifest, referenced
artifact hashes, and nested evidence bundle before publishing results.

Add `ctxhelm=true` to a `--head` spec when the row should call
`ctxhelm prepare-task` before the adapter. Add `pack=true` to also call
`ctxhelm get-pack --format json`; HelmBench stores only source-free
recommendation and token metadata.

Generate a source-free public benchmark suite after checking fixture health.
Supported presets are `refactoring-miner`, `flask`, `ripgrep`, and `express`. The
RefactoringMiner preset emits 10 tasks, enough to clear HelmBench's recommended
minimum benchmark size without a low-sample warning:

```bash
cargo run -- init-public-suite \
  --preset refactoring-miner \
  --repo ../ctxhelm-proof-fixtures/RefactoringMiner \
  --suite-out /tmp/refactoringminer-suite.json \
  --health-out /tmp/refactoringminer-health.json \
  --force

cargo run -- init-public-suite \
  --preset flask \
  --repo ../flask \
  --suite-out /tmp/flask-suite.json \
  --health-out /tmp/flask-health.json \
  --force

cargo run -- init-public-suite \
  --preset ripgrep \
  --repo ../ripgrep \
  --suite-out /tmp/ripgrep-suite.json \
  --health-out /tmp/ripgrep-health.json \
  --force

cargo run -- init-public-suite \
  --preset express \
  --repo ../express \
  --suite-out /tmp/express-suite.json \
  --health-out /tmp/express-health.json \
  --force
```

When `--suite-out` or `--health-out` are omitted, HelmBench writes
preset-specific defaults such as `suites/flask-public.json` and
`.helmbench/flask-public-suite-health.json`, or the matching ripgrep defaults
under `suites/ripgrep-public.json` and
`.helmbench/ripgrep-public-suite-health.json`; Express uses
`suites/express-public.json` and
`.helmbench/express-public-suite-health.json`.

Validate a suite:

```bash
cargo run -- validate-suite suites/example-auth-bugs.json
```

Check that a suite is healthy against a repo before running agents:

```bash
cargo run -- suite-health \
  --suite /tmp/helmbench-demo-suite.json \
  --repo /tmp/helmbench-demo-repo \
  --out /tmp/helmbench-suite-health.md \
  --format markdown
```

`suite-health` verifies the repo is a git checkout, commit depth satisfies the
configured minimum, expected files/tests exist, success commands are present,
and the checkout is clean unless `--allow-dirty` is set. The report is
source-free and can be included in evidence bundles.

For outcome claims, add `--check-success-commands`. HelmBench will run each
validation command in an isolated clone and fail the health check if validation
already passes before an agent changes the repo.
For large suites, add `--fail-fast-success-commands` to stop after the first
pre-agent validation pass. For public suites, pass `--preset <preset>` so the
health report includes the public-suite label and anchor checks.

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

Summarize a benchmark baseline against multiple variants:

```bash
cargo run -- benchmark-summary \
  --base reports/example-native.json \
  --head reports/example-ctxhelm.json \
  --head reports/example-claude-code.json \
  --out reports/example-benchmark-summary.md \
  --format markdown
```

Fail CI if a summary regresses against quality thresholds:

```bash
cargo run -- quality-gate \
  --summary reports/example-benchmark-summary.json \
  --max-average-time-to-first-relevant-file-millis-delta 0 \
  --max-total-tool-calls-delta 0 \
  --max-total-token-estimate-delta 0 \
  --max-tool-calls-per-success-delta 0 \
  --max-token-estimate-per-success-delta 0
```

Package a source-free evidence bundle:

```bash
cargo run -- evidence-bundle \
  --suite suites/example-auth-bugs.json \
  --base-report reports/example-native.json \
  --head-report reports/example-ctxhelm.json \
  --head-report reports/example-claude-code.json \
  --out-dir /tmp/helmbench-evidence \
  --force
```

Verify a published evidence bundle:

```bash
cargo run -- verify-bundle \
  --bundle /tmp/helmbench-evidence
```

Render a static source-free dashboard:

```bash
cargo run -- dashboard \
  --report reports/example-native.json \
  --report reports/example-ctxhelm.json \
  --report reports/example-claude-code.json \
  --out docs/example-dashboard.html
```

Generate a source-free agent autopsy:

```bash
cargo run -- autopsy \
  --suite suites/example-auth-bugs.json \
  --trace-dir examples/traces/native \
  --out docs/example-autopsy.md
```

Generate a source-free diff autopsy for a worktree or branch:

```bash
cargo run -- diff-autopsy \
  --suite suites/example-auth-bugs.json \
  --repo . \
  --task-id auth-redirect-001 \
  --base-ref origin/main \
  --head-ref HEAD \
  --out /tmp/helmbench-diff-autopsy.md
```

Analyze a GitHub PR by changed file names only:

```bash
cargo run -- diff-autopsy \
  --suite suites/example-auth-bugs.json \
  --repo . \
  --task-id auth-redirect-001 \
  --pr 42 \
  --out /tmp/helmbench-pr-autopsy.md
```

Run direct agent presets:

```bash
cargo run -- doctor --repo . --format json --out /tmp/helmbench-doctor.json

cargo run -- claude-run \
  --suite suites/local-run-smoke.json \
  --repo . \
  --dangerously-skip-permissions \
  --out-dir traces/claude-run

cargo run -- codex-run \
  --suite suites/local-run-smoke.json \
  --repo . \
  --out-dir traces/codex-run
```

These commands suppress agent stdout/stderr and do not store transcripts. They
ask the agent to emit source-free `record-event` calls. HelmBench still infers
edited files from git status and records validation from the suite
`successCommand`.

When an agent can emit structured JSONL tool metadata on stdout, add
`--capture-stream` to parse that stream in memory and persist only source-free
events.

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

Convert a structured agent JSONL stream into traces:

```bash
cargo run -- stream-trace \
  --suite suites/example-auth-bugs.json \
  --stream examples/streams/claude-code/auth-redirect-001.jsonl \
  --task-id auth-redirect-001 \
  --agent claude-code \
  --variant native-search \
  --status success \
  --out-dir examples/traces/stream-claude
```

Use `native` for an agent-alone baseline and `native-search` when the trace
captures the agent's own repository search or built-in context discovery
without ctxhelm.

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

Run a ctxhelm-guided benchmark:

```bash
cargo run -- ctxhelm-run \
  --suite /tmp/helmbench-demo-suite.json \
  --repo /tmp/helmbench-demo-repo \
  --ctxhelm-bin ctxhelm \
  --mode bug-fix \
  --pack \
  --pack-budget brief \
  --adapter-command "HELMBENCH_BIN=$(pwd)/target/debug/helmbench sh scripts/demo-agent.sh" \
  --out-dir /tmp/helmbench-ctxhelm-traces
```

`ctxhelm-run` records source-free `recommended-file` events from
`ctxhelm prepare-task`. With `--pack`, it calls `ctxhelm get-pack --format json`
but stores only source-free pack metadata such as token estimate; raw pack
sections and snippets are discarded.

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

Run the generated demo benchmark:

```bash
cargo build

cargo run -- local-run \
  --suite /tmp/helmbench-demo-suite.json \
  --repo /tmp/helmbench-demo-repo \
  --adapter-command "HELMBENCH_BIN=$(pwd)/target/debug/helmbench sh scripts/demo-agent.sh" \
  --out-dir /tmp/helmbench-demo-traces

cargo run -- run \
  --suite /tmp/helmbench-demo-suite.json \
  --trace-dir /tmp/helmbench-demo-traces \
  --out /tmp/helmbench-demo-report.json
```

The repo includes `.ctxhelmignore` so generated reports and traces do not
pollute ctxhelm recommendation quality.

## What HelmBench Measures

| Metric | Meaning |
| --- | --- |
| Task success | Whether the trace reports success, failure, or skip. |
| 95% confidence intervals | Wilson score intervals for binary per-task rates in benchmark summaries. |
| Low-sample warning | Whether a benchmark suite has fewer than the recommended 10 tasks; CI gates can require a minimum task count. |
| Failure taxonomy | Source-free counts for failed/skipped tasks, validation gaps, context misses, edit misses, recommendation misses, and irrelevant-read tasks. |
| Command mix | Source-free counts of test, build, lint, typecheck, other, successful, and failed commands. |
| Files read | Source-free paths the agent inspected. |
| Irrelevant file reads | Files read that were not in the expected evidence set. |
| Recommendation precision | Expected evidence paths divided by recommended paths. |
| Recommendation recall | Recommended expected evidence divided by all expected evidence. |
| Context precision | Relevant reads divided by all reads. |
| Edited-file recall | Expected target files edited divided by expected files. |
| Validation coverage | Whether expected tests or validation command classes were run successfully. |
| Time to first relevant file | How quickly the agent reached a target file; benchmark summaries, matrix history, and optional quality gates report average latency when traces provide timing. |
| Tool/token cost | Source-free cost proxies from trace metadata, including total cost and cost per successful task. |

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
  schema
  init-demo-repo
  demo-run
  run-matrix
  init-public-suite
  init-public-matrix
  suite-health
  validate-suite
  run
  ctxhelm-trace
  ctxhelm-run
  claude-trace
  stream-trace
  local-run
  claude-run
  codex-run
  record-event
  compare
  benchmark-summary
  matrix-history
  quality-gate
  evidence-bundle
  verify-bundle
  verify-matrix
  autopsy
  diff-autopsy
  dashboard
  doctor

adapters
  ctxhelm prepare-task trace generation
  ctxhelm-guided local run with source-free pack metadata
  Claude Code source-free event import
  structured agent stream import
  explicit local adapter command runner
  Claude Code direct launch preset
  Codex direct launch preset
  opt-in structured stdout capture for direct runs
  Agent Diff Autopsy from source-free traces and git diffs

future direct-agent adapters
  cursor
```

## Next Milestones

1. Add Cursor direct-run preset when a stable non-interactive launch contract is available.
2. Add more public benchmark presets with source-free fixture health checks.
3. Add richer direct-agent observation adapters while preserving the source-free contract.
