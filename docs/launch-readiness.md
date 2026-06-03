# HelmBench Launch Readiness

Suite: `local-run-smoke`

Status: **smoke_proof**

## Benchmark

| Metric | Value |
| --- | ---: |
| Tasks | 1 |
| Runs | 3 |
| Real-agent rows | 0 |
| Real-agent reports | 1 |
| Public reports | 1 |
| Public report tasks | 10 |
| Best success rate | 100.0% |
| Best validation coverage | 100.0% |
| Best recommendation recall | 100.0% |
| Best context precision | 100.0% |
| Best edited-file recall | 100.0% |

## Checks

| Check | Status | Detail |
| --- | --- | --- |
| suite contract | `pass` | suite `local-run-smoke` validates with 1 task(s) |
| source-free reports | `pass` | 3 report(s) accepted by benchmark-summary |
| public benchmark coverage | `pass` | 1 outcome task(s), 10 public recommendation task(s); launch target is 10 |
| real-agent evidence | `pass` | 0 matching real-agent matrix row(s), 1 matching real-agent report(s), 0 suite mismatch(es), 0 non-real-agent report(s); launch target is 1 |
| outcome-health evidence | `pass` | matching suite-health evidenceUse: outcome_ready |
| verified run matrix | `pass` | 1 matching verified matrix output(s), 0 suite mismatch(es), 0 failure(s) |
| launch-grade public matrix | `warn` | 0 verified real-agent matrix output(s) at 10+ task(s); launch target is 1 real-agent row(s) |
| privacy boundary | `pass` | artifacts store paths, counts, statuses, hashes, and source-free flags only |

## Artifacts

| Kind | Label | Source-free |
| --- | --- | --- |
| `suite` | `suite:c086296f696d9338` | yes |
| `base_report` | `base_report:f852f451ef6918ca` | yes |
| `head_report` | `head_report:728e11ac30307a17` | yes |
| `head_report` | `head_report:79bc53e09722fd5b` | yes |
| `real_agent_report` | `real_agent_report:3270fb3ab1f8d24e` | yes |
| `public_report` | `public_report:a5dde3393a65557c` | yes |
| `health` | `health:7de86413dc8bde4c` | yes |
| `matrix` | `matrix:2b1d7fc8d4c043a4` | yes |
| `matrix_evidence_use` | `outcome_ready` | yes |

## Privacy

- Source-free: `true`
- Raw source logged: `false`
- Raw prompts logged: `false`
- Raw transcripts logged: `false`
- Raw terminal logs logged: `false`
