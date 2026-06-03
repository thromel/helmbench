# HelmBench Launch Proof

This page is the recruiter-readable proof snapshot for HelmBench. It is built
from checked-in source-free reports and can be regenerated with local commands;
it is not a hand-written benchmark claim.

## What It Proves

HelmBench can compare coding-agent behavior across variants and report:

- task success;
- validation coverage;
- recommendation recall;
- context precision;
- edited-file recall;
- irrelevant reads;
- time to first relevant file;
- tool and token estimates;
- source-free privacy status.

The current checked-in proof is intentionally small: `1` task from
`suites/example-auth-bugs.json`. Treat the deltas as a directional smoke proof,
not a statistically powered benchmark. The generated summary records the same
low-sample warning. The generated
[launch-readiness report](launch-readiness.md) classifies the current checked-in
proof as `smoke_proof`. It verifies the checked-in local smoke matrix and
outcome-ready suite-health evidence, and it counts the checked-in real Claude
Code smoke report as real-agent evidence. It counts the RefactoringMiner
10-task recommendation proof as public benchmark coverage, while still warning
that launch-grade proof requires a 10-task real-agent public matrix.

HelmBench also includes a real direct-agent smoke run over
`suites/local-run-smoke.json`. That proof launches Claude Code through
`claude-run`, suppresses raw stdout/stderr, records only source-free telemetry,
infers edits from git status, and validates the isolated clone after the agent
exits. The tracked fixture is healthy at rest; the suite's task-level
`setupCommands` seed the failing state inside each isolated clone before the
agent runs, so `suite-health --check-success-commands` can prove the validation
command fails pre-agent and passes only after repair.

## Current Source-Free Result

Baseline: `claude-code / native`

Best checked-in ctxhelm-guided row: `claude-code / ctxhelm_mcp`

| Metric | Native | ctxhelm-guided | Delta |
| --- | ---: | ---: | ---: |
| Success | 0.0% | 100.0% | +100.0% |
| Validation coverage | 0.0% | 100.0% | +100.0% |
| Recommendation recall | 0.0% | 100.0% | +100.0% |
| Context precision | 25.0% | 66.7% | +41.7% |
| Edited-file recall | 50.0% | 100.0% | +50.0% |
| Irrelevant reads | 75.0% | 33.3% | -41.7% |
| First relevant file | 2600 ms | 600 ms | -2000 ms |
| Tool calls | 14 | 9 | -5 |
| Token estimate | 6400 | 4100 | -2300 |

## Evidence Artifacts

- Suite: [`suites/example-auth-bugs.json`](../suites/example-auth-bugs.json)
- Native report: [`reports/example-native.json`](../reports/example-native.json)
- ctxhelm-guided report:
  [`reports/example-ctxhelm.json`](../reports/example-ctxhelm.json)
- Claude Code event-import report:
  [`reports/example-claude-code.json`](../reports/example-claude-code.json)
- Real Claude Code smoke report:
  [`reports/claude-real-smoke.json`](../reports/claude-real-smoke.json)
- Real Claude Code smoke Markdown:
  [`docs/claude-real-smoke.md`](claude-real-smoke.md)
- RefactoringMiner outcome-readiness report:
  [`reports/refactoringminer-outcome-health.json`](../reports/refactoringminer-outcome-health.json)
- Generated benchmark summary:
  [`docs/example-benchmark-summary.md`](example-benchmark-summary.md)
- Static dashboard: [`docs/example-dashboard.html`](example-dashboard.html)
- Comparison report: [`docs/example-compare.md`](example-compare.md)
- Autopsy report: [`docs/example-autopsy.md`](example-autopsy.md)
- Launch readiness Markdown: [`docs/launch-readiness.md`](launch-readiness.md)
- Launch readiness JSON:
  [`reports/launch-readiness.json`](../reports/launch-readiness.json)
- Verified local smoke matrix:
  [`docs/local-smoke-matrix/matrix-manifest.json`](local-smoke-matrix/matrix-manifest.json)

## Regenerate

```bash
cargo run -- benchmark-summary \
  --base reports/example-native.json \
  --head reports/example-ctxhelm.json \
  --head reports/example-claude-code.json \
  --format markdown \
  --out docs/example-benchmark-summary.md

cargo run -- dashboard \
  --report reports/example-native.json \
  --report reports/example-ctxhelm.json \
  --report reports/example-claude-code.json \
  --out docs/example-dashboard.html

cargo run -- launch-readiness \
  --suite suites/local-run-smoke.json \
  --base-report docs/local-smoke-matrix/reports/native.json \
  --head-report docs/local-smoke-matrix/reports/native-search.json \
  --head-report docs/local-smoke-matrix/reports/guided.json \
  --health docs/local-smoke-matrix/reports/suite-health.json \
  --matrix docs/local-smoke-matrix \
  --real-agent-report reports/claude-real-smoke.json \
  --public-report reports/refactoringminer-ctxhelm-plan.json \
  --out docs/launch-readiness.md \
  --format markdown

cargo run -- launch-readiness \
  --suite suites/local-run-smoke.json \
  --base-report docs/local-smoke-matrix/reports/native.json \
  --head-report docs/local-smoke-matrix/reports/native-search.json \
  --head-report docs/local-smoke-matrix/reports/guided.json \
  --health docs/local-smoke-matrix/reports/suite-health.json \
  --matrix docs/local-smoke-matrix \
  --real-agent-report reports/claude-real-smoke.json \
  --public-report reports/refactoringminer-ctxhelm-plan.json \
  --out reports/launch-readiness.json \
  --format json

./scripts/verify.sh
```

## Privacy Contract

The proof artifacts store paths, counts, statuses, timings, command classes,
hashes, and source-free privacy flags. They do not store raw source, raw
prompts, raw transcripts, raw terminal logs, raw MCP payloads, or ctxhelm pack
snippets.

## Next Proof Step

HelmBench now also has a 10-task
[RefactoringMiner public recommendation proof](refactoringminer-public-proof.md)
that measures ctxhelm recommendation quality on a real public repository suite.

The next proof step is a full `run-matrix` over that same suite with at least
one real agent baseline and one ctxhelm-guided agent row, then publishing the
verified matrix directory and evidence bundle. That is the path from
recommendation proof to launch-grade agent outcome evidence. The checked
RefactoringMiner outcome-readiness report currently shows validation can pass
before any agent change and classifies the evidence as `navigation_only`, so
seeded task setup is required before treating that suite as task-success
evidence. Use `init-git-regression-suite` to derive seeded public tasks from
real commits and write a health artifact with `evidenceUse: outcome_ready`.
Then use `init-agent-matrix` to create the native-vs-ctxhelm real-agent matrix
config with `healthCheckSuccessCommands` and `healthRequireSetupCommands`, and
run `verify-matrix` on the generated output.
