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
low-sample warning.

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
- Generated benchmark summary:
  [`docs/example-benchmark-summary.md`](example-benchmark-summary.md)
- Static dashboard: [`docs/example-dashboard.html`](example-dashboard.html)
- Comparison report: [`docs/example-compare.md`](example-compare.md)
- Autopsy report: [`docs/example-autopsy.md`](example-autopsy.md)

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

./scripts/verify.sh
```

## Privacy Contract

The proof artifacts store paths, counts, statuses, timings, command classes,
hashes, and source-free privacy flags. They do not store raw source, raw
prompts, raw transcripts, raw terminal logs, raw MCP payloads, or ctxhelm pack
snippets.

## Next Proof Step

Run the same matrix shape on a public suite with at least `10` tasks, such as
the RefactoringMiner preset, then publish the verified matrix directory and
evidence bundle. That is the path from smoke proof to launch-grade benchmark
evidence.
