# RefactoringMiner Public Recommendation Proof

This proof runs `ctxhelm prepare-task` across a 10-task public benchmark suite
for RefactoringMiner and scores the source-free recommendations against each
task's expected files and tests.

It is not an agent task-success benchmark. The traces are `ctxhelm_plan`
recommendation traces, so task status and validation coverage are intentionally
`skipped` / `0.0%`. The point of this proof is narrower and useful: can
HelmBench measure whether ctxhelm points a coding agent toward the expected
files on a real public repository suite?

## Fixture Health

The checked fixture health report proves:

- preset: `refactoring-miner`;
- suite: `refactoringminer-public`;
- tasks: `10`;
- repository basename: `RefactoringMiner`;
- commit count: `5744`;
- dirty checkout: `false`;
- git fsck ok: `true`;
- missing expected files/tests: `0`;
- source-free: `true`.

Health artifact:
[`reports/refactoringminer-suite-health.json`](../reports/refactoringminer-suite-health.json)

## Recommendation-Suite Outcome Readiness

The recommendation-oriented RefactoringMiner suite is **not** ready for
task-success claims.
The source-free validation-baseline gate was run with
`--check-success-commands --fail-fast-success-commands` and stopped after the
first clean-checkout validation command passed before any agent changes.

Outcome-health artifact:
[`reports/refactoringminer-outcome-health.json`](../reports/refactoringminer-outcome-health.json)

| Metric | Value |
| --- | ---: |
| Evidence use | navigation_only |
| Validation baseline ready | false |
| Baseline success-command passes | 1 |
| Baseline success-command skipped by fail-fast | 9 |

This means the checked `ctxhelm prepare-task` proof should be treated as a
navigation/recommendation proof.

## Seeded Outcome Suite

For task-success evidence, HelmBench now includes a generated git-regression
suite that seeds each task by restoring expected implementation files from the
parent commit while keeping current tests as the oracle.

Suite artifact:
[`suites/refactoring-miner-git-regressions.json`](../suites/refactoring-miner-git-regressions.json)

Health artifact:
[`reports/refactoringminer-git-regressions-health.json`](../reports/refactoringminer-git-regressions-health.json)

| Metric | Value |
| --- | ---: |
| Evidence use | outcome_ready |
| Tasks | 10 |
| Validation baseline ready | true |
| Baseline success-command failures | 10 |
| Baseline success-command passes | 0 |
| Setup failures | 0 |
| Validation timeouts | 0 |

## Result

Report artifact:
[`reports/refactoringminer-ctxhelm-plan.json`](../reports/refactoringminer-ctxhelm-plan.json)

Markdown report:
[`docs/refactoringminer-ctxhelm-plan.md`](refactoringminer-ctxhelm-plan.md)

| Metric | Value |
| --- | ---: |
| Tasks | 10 |
| Recommended files per task | 20 |
| Average recommendation precision | 14.0% |
| Average recommendation recall | 61.3% |
| Tool calls | 210 |
| Source-free | true |

Per-task recommendation recall:

| Task | Recall |
| --- | ---: |
| `rm-mcp-intent-validation-001` | 100.0% |
| `rm-mcp-tools-contract-001` | 80.0% |
| `rm-mcp-service-repository-001` | 80.0% |
| `rm-mcp-server-startup-001` | 80.0% |
| `rm-astdiff-comments-001` | 80.0% |
| `rm-git-history-merge-001` | 50.0% |
| `rm-webdiff-viewed-files-001` | 50.0% |
| `rm-astdiff-matchers-001` | 40.0% |
| `rm-cli-command-line-001` | 33.3% |
| `rm-astdiff-python-001` | 20.0% |

## Real-Agent Matrix Result

HelmBench now includes a verified real-agent matrix over the seeded
git-regression suite:
[`docs/refactoringminer-real-matrix/matrix-manifest.json`](refactoringminer-real-matrix/matrix-manifest.json).

This matrix is outcome-ready and source-free, but the quality gate failed. It
should be read as a diagnostic result: ctxhelm improved recommendation recall,
but the `ctxhelm_mcp` agent row did not read the recommended files or make
successful repairs.

| Metric | Native Claude Code | Claude Code + ctxhelm_mcp | Delta |
| --- | ---: | ---: | ---: |
| Success | 30.0% | 0.0% | -30.0% |
| Validation coverage | 30.0% | 0.0% | -30.0% |
| Recommendation recall | 0.0% | 50.1% | +50.1% |
| Recommendation follow-through | 0.0% | 0.0% | +0.0% |
| Context precision | 74.2% | 0.0% | -74.2% |
| Edited-file recall | 60.0% | 0.0% | -60.0% |
| Irrelevant reads | 7.7% | 0.0% | -7.7% |

The quality gate includes `minRecommendationFollowThrough = 0.1` for this
diagnostic matrix, so the checked artifact fails explicitly when a guided row
receives recommendations but inspects none of them.

Artifacts:

- Benchmark summary:
  [`docs/refactoringminer-real-matrix/docs/benchmark-summary.md`](refactoringminer-real-matrix/docs/benchmark-summary.md)
- Quality gate:
  [`docs/refactoringminer-real-matrix/docs/quality-gate.md`](refactoringminer-real-matrix/docs/quality-gate.md)
- Privacy report:
  [`docs/refactoringminer-real-matrix/docs/privacy-report.md`](refactoringminer-real-matrix/docs/privacy-report.md)
- Static dashboard:
  [`docs/refactoringminer-real-matrix/docs/dashboard.html`](refactoringminer-real-matrix/docs/dashboard.html)

## Regenerate

```bash
cargo run -- init-public-suite \
  --preset refactoring-miner \
  --repo <refactoringminer-repo> \
  --suite-out suites/refactoring-miner-public.json \
  --health-out reports/refactoringminer-suite-health.json \
  --min-commits 1000 \
  --force

cargo run -- ctxhelm-trace \
  --suite suites/refactoring-miner-public.json \
  --repo <refactoringminer-repo> \
  --ctxhelm-bin ctxhelm \
  --mode bug-fix \
  --target-agent claude-code \
  --out-dir /tmp/helmbench-refactoringminer-proof/traces

cargo run -- run \
  --suite suites/refactoring-miner-public.json \
  --trace-dir /tmp/helmbench-refactoringminer-proof/traces \
  --out reports/refactoringminer-ctxhelm-plan.json

cargo run -- run \
  --suite suites/refactoring-miner-public.json \
  --trace-dir /tmp/helmbench-refactoringminer-proof/traces \
  --out docs/refactoringminer-ctxhelm-plan.md \
  --format markdown
```

## Privacy

The committed artifacts store suite metadata, paths, counts, statuses, timing,
tool-call counts, and privacy flags. They do not store raw source, raw prompts,
raw transcripts, raw terminal logs, raw MCP payloads, or ctxhelm pack snippets.

## Next Step

The next launch-grade proof is a full `run-matrix` over the seeded
git-regression suite with at least one real agent baseline and one
ctxhelm-guided agent row. Use the `preset=claude-code` or `preset=codex` matrix rows from
[`docs/run-matrix.md`](run-matrix.md) so HelmBench injects the source-free event
contract instead of relying on hand-written adapter commands.

For task-success evidence, first generate a seeded git-regression suite from
public commits and require an outcome-ready health report:

```bash
cargo run -- init-git-regression-suite \
  --repo <refactoringminer-repo> \
  --suite-out /tmp/refactoring-miner-git-regressions.json \
  --health-out /tmp/refactoring-miner-git-regressions-health.json \
  --success-command-template 'JAVA_HOME=$(/usr/libexec/java_home -v 17 2>/dev/null || echo "$JAVA_HOME") ./gradlew --no-daemon test {gradle_test_filters}' \
  --require-changed-tests \
  --require-code-files \
  --max-tasks 10 \
  --max-changed-tests 4 \
  --commit 949bddcd3509 \
  --commit 4fa3c1a48ad4 \
  --commit bd0b2277933f \
  --commit 1b9f2cf08b3c \
  --commit 092c13f035f9 \
  --commit fa8df046b0e0 \
  --commit 1b04d6aae2e4 \
  --commit 23e298ae221c \
  --commit 97e31265fd95 \
  --commit fa29ed0c80c8 \
  --check-success-commands \
  --fail-fast-success-commands \
  --force
```

Then generate the real-agent matrix config for that seeded suite:

```bash
cargo run -- init-agent-matrix \
  --suite /tmp/refactoring-miner-git-regressions.json \
  --repo <refactoringminer-repo> \
  --out /tmp/refactoring-miner-git-regressions-matrix.json \
  --out-dir /tmp/refactoring-miner-git-regressions-matrix \
  --health-out /tmp/refactoring-miner-git-regressions-matrix-health.json \
  --agent-preset claude-code \
  --dangerously-skip-permissions \
  --ctxhelm-bin ctxhelm \
  --pack \
  --health-check-success-commands \
  --health-require-setup-commands \
  --force
```

Generate the matrix config with suite-health checked up front:

```bash
cargo run -- init-public-matrix \
  --preset refactoring-miner \
  --repo <refactoringminer-repo> \
  --suite suites/refactoring-miner-public.json \
  --out /tmp/refactoring-miner-matrix.json \
  --out-dir /tmp/refactoring-miner-matrix \
  --health-out /tmp/refactoring-miner-matrix-health.json \
  --agent-preset claude-code \
  --dangerously-skip-permissions \
  --ctxhelm-bin ctxhelm \
  --pack \
  --force

cargo run -- validate-matrix \
  --config /tmp/refactoring-miner-matrix.json

cargo run -- suite-health \
  --preset refactoring-miner \
  --suite suites/refactoring-miner-public.json \
  --repo <refactoringminer-repo> \
  --out /tmp/refactoring-miner-outcome-health.json \
  --min-commits 1000 \
  --check-success-commands \
  --fail-fast-success-commands

cargo run -- run-matrix \
  --config /tmp/refactoring-miner-matrix.json \
  --force

cargo run -- refresh-matrix \
  --matrix /tmp/refactoring-miner-matrix \
  --min-task-count 10 \
  --min-recommendation-follow-through 0.1
```

This recommendation proof established the public-suite target and source-free
measurement contract first. The seeded git-regression suite then supplied
outcome-ready task setup, and the checked real-agent matrix now supplies the
first 10-task public outcome run. The remaining launch-readiness gap is not
missing infrastructure; it is that the `ctxhelm_mcp` row failed the quality
gate, including the absolute recommendation follow-through check. The next
iteration should harden the ctxhelm-guided adapter/prompt path and rerun the
same matrix until the public-matrix quality gate passes.
