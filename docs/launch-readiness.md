# HelmBench Launch Readiness

Suite: `example-auth-bugs`

Status: **smoke_proof**

## Benchmark

| Metric | Value |
| --- | ---: |
| Tasks | 1 |
| Runs | 3 |
| Real-agent rows | 3 |
| Best success rate | 100.0% |
| Best validation coverage | 100.0% |
| Best recommendation recall | 100.0% |
| Best context precision | 50.0% |
| Best edited-file recall | 100.0% |

## Checks

| Check | Status | Detail |
| --- | --- | --- |
| suite contract | `pass` | suite `example-auth-bugs` validates with 1 task(s) |
| source-free reports | `pass` | 3 report(s) accepted by benchmark-summary |
| recommended task count | `warn` | 1 task(s) observed; launch target is 10 |
| real-agent rows | `pass` | 3 real-agent row(s) observed; launch target is 1 |
| outcome-health evidence | `warn` | no matching suite-health artifact was supplied |
| verified run matrix | `warn` | no verified run-matrix artifact was supplied |
| privacy boundary | `pass` | artifacts store paths, counts, statuses, hashes, and source-free flags only |

## Artifacts

| Kind | Label | Source-free |
| --- | --- | --- |
| `suite` | `suite:ea0afd9fc6dc6762` | yes |
| `base_report` | `base_report:5f76f396d2220151` | yes |
| `head_report` | `head_report:7ccc8f4cb3498567` | yes |
| `head_report` | `head_report:03339f5e8144cbb4` | yes |

## Privacy

- Source-free: `true`
- Raw source logged: `false`
- Raw prompts logged: `false`
- Raw transcripts logged: `false`
- Raw terminal logs logged: `false`
