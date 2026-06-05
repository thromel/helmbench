# HelmBench Benchmark Summary: `local-run-smoke`

Baseline: **demo-baseline / Native**

## Confidence

- Confidence level: `95%`
- Tasks: `1`
- Recommended minimum tasks: `10`
- Low sample warning: `true`

- Low sample size: 1 task(s). Treat deltas as directional until the suite has at least 10 tasks.
- Intervals use a Wilson score interval for binary per-task rates.

## Runs

| Run | Tasks | Success | 95% CI | Validation | 95% CI | Rec recall | Rec follow-through | Context precision | Edited recall | Irrelevant reads | Avg first relevant | Tools | Tokens | Tools/success | Tokens/success |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| demo-baseline / Native | 1 | 0.0% | 0.0-79.3% | 0.0% | 0.0-79.3% | 0.0% | 0.0% | 0.0% | 0.0% | 0.0% | n/a | 2 | 0 | n/a | n/a |
| demo-search / NativeSearch | 1 | 100.0% | 20.7-100.0% | 100.0% | 20.7-100.0% | 50.0% | 100.0% | 100.0% | 100.0% | 0.0% | 20 ms | 5 | 0 | 5.0 | 0.0 |
| demo-guided / CtxhelmMcp | 1 | 100.0% | 20.7-100.0% | 100.0% | 20.7-100.0% | 100.0% | 50.0% | 100.0% | 100.0% | 0.0% | 20 ms | 10 | 64 | 10.0 | 64.0 |

## Command Mix

| Run | Total | Test | Build | Lint | Typecheck | Other | Successful | Failed |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| demo-baseline / Native | 1 | 0 | 0 | 0 | 0 | 1 | 0 | 1 |
| demo-search / NativeSearch | 1 | 0 | 0 | 0 | 0 | 1 | 1 | 0 |
| demo-guided / CtxhelmMcp | 3 | 0 | 0 | 0 | 0 | 3 | 3 | 0 |

## Failure Taxonomy

Counts are source-free and may overlap when one task has multiple issues.

| Run | Failed | Skipped | Validation gaps | No relevant read | No expected edit | Recommendation miss | Ignored recommendations | Irrelevant-read tasks |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| demo-baseline / Native | 1 | 0 | 1 | 1 | 1 | 1 | 0 | 0 |
| demo-search / NativeSearch | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| demo-guided / CtxhelmMcp | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

## Deltas From Baseline

| Variant | Verdict | Success | Validation | Rec recall | Rec follow-through | Context precision | Edited recall | Irrelevant reads | First relevant | Tools | Tokens | Tools/success | Tokens/success |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| demo-search / NativeSearch | Mixed | +100.0% | +100.0% | +50.0% | +100.0% | +100.0% | +100.0% | +0.0% | n/a | +3 | +0 | n/a | n/a |
| demo-guided / CtxhelmMcp | Mixed | +100.0% | +100.0% | +100.0% | +50.0% | +100.0% | +100.0% | +0.0% | n/a | +8 | +64 | n/a | n/a |

## Privacy

- Source-free: `true`
- Raw source logged: `false`
- Raw prompts logged: `false`
- Raw transcripts logged: `false`
- Raw terminal logs logged: `false`
