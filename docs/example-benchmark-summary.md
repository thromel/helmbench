# HelmBench Benchmark Summary: `example-auth-bugs`

Baseline: **claude-code / Native**

## Confidence

- Confidence level: `95%`
- Tasks: `1`
- Recommended minimum tasks: `10`
- Low sample warning: `true`

- Low sample size: 1 task(s). Treat deltas as directional until the suite has at least 10 tasks.
- Intervals use a Wilson score interval for binary per-task rates.

## Runs

| Run | Tasks | Success | 95% CI | Validation | 95% CI | Rec recall | Context precision | Edited recall | Irrelevant reads | Avg first relevant | Tools | Tokens | Tools/success | Tokens/success |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| claude-code / Native | 1 | 0.0% | 0.0-79.3% | 0.0% | 0.0-79.3% | 0.0% | 25.0% | 50.0% | 75.0% | 2600 ms | 14 | 6400 | n/a | n/a |
| claude-code / CtxhelmMcp | 1 | 100.0% | 20.7-100.0% | 100.0% | 20.7-100.0% | 100.0% | 66.7% | 100.0% | 33.3% | 600 ms | 9 | 4100 | 9.0 | 4100.0 |
| claude-code / CtxhelmMcp | 1 | 100.0% | 20.7-100.0% | 100.0% | 20.7-100.0% | 100.0% | 50.0% | 100.0% | 50.0% | 550 ms | 12 | 4100 | 12.0 | 4100.0 |

## Command Mix

| Run | Total | Test | Build | Lint | Typecheck | Other | Successful | Failed |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| claude-code / Native | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| claude-code / CtxhelmMcp | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| claude-code / CtxhelmMcp | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

## Failure Taxonomy

Counts are source-free and may overlap when one task has multiple issues.

| Run | Failed | Skipped | Validation gaps | No relevant read | No expected edit | Recommendation miss | Irrelevant-read tasks |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| claude-code / Native | 1 | 0 | 1 | 0 | 0 | 1 | 1 |
| claude-code / CtxhelmMcp | 0 | 0 | 0 | 0 | 0 | 0 | 1 |
| claude-code / CtxhelmMcp | 0 | 0 | 0 | 0 | 0 | 0 | 1 |

## Deltas From Baseline

| Variant | Verdict | Success | Validation | Rec recall | Context precision | Edited recall | Irrelevant reads | First relevant | Tools | Tokens | Tools/success | Tokens/success |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| claude-code / CtxhelmMcp | Improved | +100.0% | +100.0% | +100.0% | +41.7% | +50.0% | -41.7% | -2000 ms | -5 | -2300 | n/a | n/a |
| claude-code / CtxhelmMcp | Improved | +100.0% | +100.0% | +100.0% | +25.0% | +50.0% | -25.0% | -2050 ms | -2 | -2300 | n/a | n/a |

## Privacy

- Source-free: `true`
- Raw source logged: `false`
- Raw prompts logged: `false`
- Raw transcripts logged: `false`
- Raw terminal logs logged: `false`
