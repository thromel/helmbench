# HelmBench Benchmark Summary: `refactoringminer-git-regressions`

Baseline: **claude-code / Native**

## Confidence

- Confidence level: `95%`
- Tasks: `10`
- Recommended minimum tasks: `10`
- Low sample warning: `false`

- Task count meets the recommended minimum of 10 tasks.
- Intervals use a Wilson score interval for binary per-task rates.

## Runs

| Run | Tasks | Success | 95% CI | Validation | 95% CI | Rec recall | Rec follow-through | Context precision | Edited recall | Irrelevant reads | Avg first relevant | Tools | Tokens | Tools/success | Tokens/success |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| claude-code / Native | 10 | 30.0% | 10.8-60.3% | 30.0% | 10.8-60.3% | 0.0% | 0.0% | 74.2% | 60.0% | 7.7% | n/a | 68 | 0 | 22.7 | 0.0 |
| claude-code / CtxhelmMcp | 10 | 0.0% | 0.0-27.8% | 0.0% | 0.0-27.8% | 50.1% | 0.0% | 0.0% | 0.0% | 0.0% | n/a | 250 | 26587 | n/a | n/a |

## Command Mix

| Run | Total | Test | Build | Lint | Typecheck | Other | Successful | Failed |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| claude-code / Native | 10 | 10 | 0 | 0 | 0 | 0 | 3 | 7 |
| claude-code / CtxhelmMcp | 30 | 10 | 0 | 0 | 0 | 20 | 20 | 10 |

## Failure Taxonomy

Counts are source-free and may overlap when one task has multiple issues.

| Run | Failed | Skipped | Validation gaps | No relevant read | No expected edit | Recommendation miss | Ignored recommendations | Irrelevant-read tasks |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| claude-code / Native | 7 | 0 | 7 | 2 | 2 | 10 | 0 | 2 |
| claude-code / CtxhelmMcp | 10 | 0 | 10 | 10 | 10 | 1 | 10 | 0 |

## Deltas From Baseline

| Variant | Verdict | Success | Validation | Rec recall | Rec follow-through | Context precision | Edited recall | Irrelevant reads | First relevant | Tools | Tokens | Tools/success | Tokens/success |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| claude-code / CtxhelmMcp | Mixed | -30.0% | -30.0% | +50.1% | +0.0% | -74.2% | -60.0% | -7.7% | n/a | +182 | +26587 | n/a | n/a |

## Privacy

- Source-free: `true`
- Raw source logged: `false`
- Raw prompts logged: `false`
- Raw transcripts logged: `false`
- Raw terminal logs logged: `false`
