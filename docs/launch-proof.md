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
10-task recommendation proof as public benchmark coverage. It also verifies a
10-task RefactoringMiner real-agent public matrix, but still keeps launch
readiness at `smoke_proof` because that matrix's quality gate failed.

HelmBench also includes a real direct-agent smoke run over
`suites/local-run-smoke.json`. That proof launches Claude Code through
`claude-run`, suppresses raw stdout/stderr, records only source-free telemetry,
infers edits from git status, and validates the isolated clone after the agent
exits. The tracked fixture is healthy at rest; the suite's task-level
`setupCommands` seed the failing state inside each isolated clone before the
agent runs, so `suite-health --check-success-commands` can prove the validation
command fails pre-agent and passes only after repair.

The checked direct-runner runtime snapshot is also source-free. It proves the
CLI adapters are installed and privacy-safe, while recording the current rerun
blockers as coarse failure classes: Claude Code is blocked by `session_limit`
and Codex is blocked by `cli_upgrade_required`.

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

## Real-Agent Public Matrix

HelmBench now has a checked-in 10-task RefactoringMiner real-agent matrix over
the seeded git-regression suite. This is outcome-ready evidence, but it is a
diagnostic result rather than a ctxhelm win: native Claude Code solved `30.0%`
of tasks and the `ctxhelm_mcp` row solved `0.0%`.

| Metric | Native | ctxhelm-mcp | Delta |
| --- | ---: | ---: | ---: |
| Success | 30.0% | 0.0% | -30.0% |
| Validation coverage | 30.0% | 0.0% | -30.0% |
| Recommendation recall | 0.0% | 50.1% | +50.1% |
| Recommendation follow-through | 0.0% | 0.0% | +0.0% |
| Context precision | 74.2% | 0.0% | -74.2% |
| Edited-file recall | 60.0% | 0.0% | -60.0% |
| Irrelevant reads | 7.7% | 0.0% | -7.7% |
| Tool calls | 68 | 250 | +182 |
| Token estimate | 0 | 26587 | +26587 |

The quality gate failed on success rate, validation coverage, context
precision, recommendation follow-through, and edited-file recall. That failure
is useful proof of the product: HelmBench detected that ctxhelm recommendations
improved recall, but the ctxhelm-guided agent row read `0.0%` of those
recommended paths.

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
- Direct-runner runtime report:
  [`docs/direct-runner-runtime.md`](direct-runner-runtime.md)
- Direct-runner runtime JSON:
  [`reports/direct-runner-runtime.json`](../reports/direct-runner-runtime.json)
- RefactoringMiner outcome-readiness report:
  [`reports/refactoringminer-outcome-health.json`](../reports/refactoringminer-outcome-health.json)
- RefactoringMiner seeded git-regression suite:
  [`suites/refactoring-miner-git-regressions.json`](../suites/refactoring-miner-git-regressions.json)
- RefactoringMiner seeded suite health:
  [`reports/refactoringminer-git-regressions-health.json`](../reports/refactoringminer-git-regressions-health.json)
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
- Verified RefactoringMiner real-agent matrix:
  [`docs/refactoringminer-real-matrix/matrix-manifest.json`](refactoringminer-real-matrix/matrix-manifest.json)
- RefactoringMiner real-agent benchmark summary:
  [`docs/refactoringminer-real-matrix/docs/benchmark-summary.md`](refactoringminer-real-matrix/docs/benchmark-summary.md)
- RefactoringMiner real-agent quality gate:
  [`docs/refactoringminer-real-matrix/docs/quality-gate.md`](refactoringminer-real-matrix/docs/quality-gate.md)

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

cargo run -- refresh-matrix \
  --matrix docs/refactoringminer-real-matrix \
  --min-task-count 10 \
  --min-recommendation-follow-through 0.1

cargo run -- doctor \
  --repo . \
  --check-direct-runners \
  --format json \
  --out reports/direct-runner-runtime.json

cargo run -- doctor \
  --repo . \
  --check-direct-runners \
  --format markdown \
  --out docs/direct-runner-runtime.md

cargo run -- launch-readiness \
  --suite suites/local-run-smoke.json \
  --base-report docs/local-smoke-matrix/reports/native.json \
  --head-report docs/local-smoke-matrix/reports/native-search.json \
  --head-report docs/local-smoke-matrix/reports/guided.json \
  --health docs/local-smoke-matrix/reports/suite-health.json \
  --matrix docs/local-smoke-matrix \
  --matrix docs/refactoringminer-real-matrix \
  --real-agent-report reports/claude-real-smoke.json \
  --public-report reports/refactoringminer-ctxhelm-plan.json \
  --doctor-report reports/direct-runner-runtime.json \
  --out docs/launch-readiness.md \
  --format markdown

cargo run -- launch-readiness \
  --suite suites/local-run-smoke.json \
  --base-report docs/local-smoke-matrix/reports/native.json \
  --head-report docs/local-smoke-matrix/reports/native-search.json \
  --head-report docs/local-smoke-matrix/reports/guided.json \
  --health docs/local-smoke-matrix/reports/suite-health.json \
  --matrix docs/local-smoke-matrix \
  --matrix docs/refactoringminer-real-matrix \
  --real-agent-report reports/claude-real-smoke.json \
  --public-report reports/refactoringminer-ctxhelm-plan.json \
  --doctor-report reports/direct-runner-runtime.json \
  --out reports/launch-readiness.json \
  --format json

./scripts/verify.sh
```

Before a real-agent rerun, add source-free runtime evidence:

```bash
cargo run -- doctor \
  --repo . \
  --check-direct-runners \
  --format json \
  --out /tmp/helmbench-doctor-runtime.json

cargo run -- launch-readiness \
  --suite suites/local-run-smoke.json \
  --base-report docs/local-smoke-matrix/reports/native.json \
  --head-report docs/local-smoke-matrix/reports/native-search.json \
  --head-report docs/local-smoke-matrix/reports/guided.json \
  --health docs/local-smoke-matrix/reports/suite-health.json \
  --matrix docs/local-smoke-matrix \
  --matrix docs/refactoringminer-real-matrix \
  --real-agent-report reports/claude-real-smoke.json \
  --public-report reports/refactoringminer-ctxhelm-plan.json \
  --doctor-report /tmp/helmbench-doctor-runtime.json \
  --out /tmp/helmbench-launch-readiness-runtime.md \
  --format markdown
```

## Privacy Contract

The proof artifacts store paths, counts, statuses, timings, command classes,
hashes, and source-free privacy flags. They do not store raw source, raw
prompts, raw transcripts, raw terminal logs, raw MCP payloads, or ctxhelm pack
snippets.

## Next Proof Step

HelmBench now has both the 10-task
[RefactoringMiner public recommendation proof](refactoringminer-public-proof.md)
and the 10-task real-agent matrix. The next proof step is ctxhelm adapter and
prompt hardening followed by a rerun that can pass the public-matrix quality
gate.
