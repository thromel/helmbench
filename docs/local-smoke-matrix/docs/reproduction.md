# HelmBench Reproduction

This source-free guide describes how to verify and rerun the matrix evidence without storing raw source, prompts, transcripts, terminal logs, adapter commands, setup commands, or ctxhelm pack sections.

## Verify Published Artifacts

```bash
helmbench verify-matrix --matrix <matrix-dir>
helmbench verify-bundle --bundle <matrix-dir>/evidence
```

## Run Identity

| Field | Value |
| --- | --- |
| HelmBench version | `0.1.0` |
| Suite hash | `suite:24f1f885604b852d` |
| Repo HEAD | `dd41d7c579ec42ef8292cabd29be1feb56a2f1fc` |
| Dirty checkout | yes |
| Setup commands | 0 hashed command(s) |

## Runs

| Run | Agent | Variant | Preset | ctxhelm | Pack | Stream | Report | Trace Dir | Autopsy | Comparison JSON | Comparison Markdown | Adapter Hash | ctxhelm Hash |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `native` | `demo-baseline` | `Native` | `none` | no | no | no | `reports/native.json` | `traces/native` | `docs/native-autopsy.md` | `none` | `none` | `none` | `none` |
| `native-search` | `demo-search` | `NativeSearch` | `claude-code` | no | no | no | `reports/native-search.json` | `traces/native-search` | `docs/native-search-autopsy.md` | `reports/compare-native-search.json` | `docs/compare-native-search.md` | `cmd:9e066200e1e53d23` | `none` |
| `guided` | `demo-guided` | `CtxhelmMcp` | `claude-code` | yes | yes | no | `reports/guided.json` | `traces/guided` | `docs/guided-autopsy.md` | `reports/compare-guided.json` | `docs/compare-guided.md` | `cmd:9e066200e1e53d23` | `ctxhelm:0a4cd58a5b88f97e` |

## Key Artifacts

| Artifact | Path |
| --- | --- |
| Suite health | `reports/suite-health.json` |
| Benchmark summary JSON | `reports/benchmark-summary.json` |
| Benchmark summary Markdown | `docs/benchmark-summary.md` |
| Quality gate JSON | `reports/quality-gate.json` |
| Quality gate Markdown | `docs/quality-gate.md` |
| Privacy report JSON | `reports/privacy-report.json` |
| Privacy report Markdown | `docs/privacy-report.md` |
| Dashboard | `docs/dashboard.html` |
| Baseline autopsy | `docs/native-autopsy.md` |
| Evidence manifest | `evidence/manifest.json` |

## Rerun Notes

- Check out the target repository at the recorded repo HEAD before rerunning.
- Use a suite with the recorded suite hash.
- Recreate adapter/setup commands from local configuration; HelmBench stores only hashes for privacy.
- Compare a new run with the published matrix using `helmbench matrix-history --matrix <old-matrix-dir> --matrix <new-matrix-dir> --out <history.md>`.

## Privacy

- Source-free: `true`
- Raw source logged: `false`
- Raw prompts logged: `false`
- Raw transcripts logged: `false`
- Raw terminal logs logged: `false`
