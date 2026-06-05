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
| Suite hash | `suite:3cb60b9dde3555a6` |
| Repo HEAD | `949bddcd3509a805f5e3bcc55fcdb71a691b0dac` |
| Dirty checkout | no |
| Setup commands | 0 hashed command(s) |

## Runs

| Run | Agent | Variant | Preset | ctxhelm | Pack | Stream | Report | Trace Dir | Autopsy | Comparison JSON | Comparison Markdown | Adapter Hash | ctxhelm Hash |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `native` | `claude-code` | `Native` | `claude-code` | no | no | no | `reports/native.json` | `traces/native` | `docs/native-autopsy.md` | `none` | `none` | `cmd:30a03a4d251df406` | `none` |
| `ctxhelm` | `claude-code` | `CtxhelmMcp` | `claude-code` | yes | yes | no | `reports/ctxhelm.json` | `traces/ctxhelm` | `docs/ctxhelm-autopsy.md` | `reports/compare-ctxhelm.json` | `docs/compare-ctxhelm.md` | `cmd:30a03a4d251df406` | `ctxhelm:9832d53201405f28` |

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
