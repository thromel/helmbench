# HelmBench Quality Gate: `local-run-smoke`

Status: **passed**

| Variant | Metric | Rule | Actual | Result |
| --- | --- | --- | ---: | --- |
| demo-search / NativeSearch | `success_rate_delta` | >= 0.0000 | 1.0000 | pass |
| demo-search / NativeSearch | `validation_coverage_rate_delta` | >= 0.0000 | 1.0000 | pass |
| demo-search / NativeSearch | `irrelevant_read_rate_delta` | <= 0.0000 | 0.0000 | pass |
| demo-search / NativeSearch | `recommendation_recall_delta` | >= 0.0000 | 0.5000 | pass |
| demo-search / NativeSearch | `recommendation_follow_through_delta` | >= 0.0000 | 1.0000 | pass |
| demo-search / NativeSearch | `context_precision_delta` | >= 0.0000 | 1.0000 | pass |
| demo-search / NativeSearch | `edited_file_recall_delta` | >= 0.0000 | 1.0000 | pass |
| demo-guided / CtxhelmMcp | `success_rate_delta` | >= 0.0000 | 1.0000 | pass |
| demo-guided / CtxhelmMcp | `validation_coverage_rate_delta` | >= 0.0000 | 1.0000 | pass |
| demo-guided / CtxhelmMcp | `irrelevant_read_rate_delta` | <= 0.0000 | 0.0000 | pass |
| demo-guided / CtxhelmMcp | `recommendation_recall_delta` | >= 0.0000 | 1.0000 | pass |
| demo-guided / CtxhelmMcp | `recommendation_follow_through_delta` | >= 0.0000 | 0.5000 | pass |
| demo-guided / CtxhelmMcp | `context_precision_delta` | >= 0.0000 | 1.0000 | pass |
| demo-guided / CtxhelmMcp | `edited_file_recall_delta` | >= 0.0000 | 1.0000 | pass |

## Warnings

- Low sample size: 1 task(s). Treat deltas as directional until the suite has at least 10 tasks.
- Intervals use a Wilson score interval for binary per-task rates.

## Privacy

- Source-free: `true`
