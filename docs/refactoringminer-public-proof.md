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

## Outcome Readiness

The current RefactoringMiner suite is **not** ready for task-success claims.
The source-free validation-baseline gate was run with
`--check-success-commands --fail-fast-success-commands` and stopped after the
first clean-checkout validation command passed before any agent changes.

Outcome-health artifact:
[`reports/refactoringminer-outcome-health.json`](../reports/refactoringminer-outcome-health.json)

| Metric | Value |
| --- | ---: |
| Validation baseline ready | false |
| Baseline success-command passes | 1 |
| Baseline success-command skipped by fail-fast | 9 |

This means the checked RefactoringMiner proof should be treated as a
navigation/recommendation proof until seeded task setup is added.

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

The next launch-grade proof is a full `run-matrix` over this same 10-task suite
with at least one real agent baseline and one ctxhelm-guided agent row, but
only after the validation baseline is checked. Use the `preset=claude-code` or
`preset=codex` matrix rows from
[`docs/run-matrix.md`](run-matrix.md) so HelmBench injects the source-free event
contract instead of relying on hand-written adapter commands.

Generate the matrix config with suite-health checked up front:

```bash
cargo run -- init-public-matrix \
  --preset refactoring-miner \
  --repo <refactoringminer-repo> \
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
```

This recommendation proof establishes the public-suite target and source-free
measurement contract first; `init-public-matrix` is the repeatable bridge from
that target to real agent outcome evidence. If the success commands already
pass before any agent changes, treat the matrix as navigation/validation
behavior evidence, not task-success evidence, until seeded task setup is added.
