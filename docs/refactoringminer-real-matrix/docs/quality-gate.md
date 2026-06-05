# HelmBench Quality Gate: `refactoringminer-git-regressions`

Status: **failed**

| Variant | Metric | Rule | Actual | Result |
| --- | --- | --- | ---: | --- |
| all / Other | `task_count` | >= 10.0000 | 10.0000 | pass |
| claude-code / CtxhelmMcp | `success_rate_delta` | >= 0.0000 | -0.3000 | fail |
| claude-code / CtxhelmMcp | `validation_coverage_rate_delta` | >= 0.0000 | -0.3000 | fail |
| claude-code / CtxhelmMcp | `irrelevant_read_rate_delta` | <= 0.0000 | -0.0769 | pass |
| claude-code / CtxhelmMcp | `recommendation_recall_delta` | >= 0.0000 | 0.5008 | pass |
| claude-code / CtxhelmMcp | `context_precision_delta` | >= 0.0000 | -0.7417 | fail |
| claude-code / CtxhelmMcp | `edited_file_recall_delta` | >= 0.0000 | -0.5995 | fail |

## Privacy

- Source-free: `true`
