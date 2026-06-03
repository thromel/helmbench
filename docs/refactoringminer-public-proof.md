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
with at least one real agent baseline and one ctxhelm-guided agent row. This
recommendation proof establishes the public-suite target and source-free
measurement contract first.
