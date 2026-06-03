# Demo Benchmark

`demo-run` creates a tiny reproducible benchmark repository, runs a failing
native baseline and a successful guided adapter variant, and writes source-free
reports, autopsy, dashboard, benchmark summary, privacy report, quality gate,
and evidence bundle artifacts. It is meant for smoke testing HelmBench itself
and for demos where a real agent call would be too slow or expensive.

## One-Command Demo

```bash
helmbench demo-run \
  --out-dir /tmp/helmbench-demo-run \
  --force
```

Important outputs:

- `/tmp/helmbench-demo-run/reports/native.json`
- `/tmp/helmbench-demo-run/reports/guided.json`
- `/tmp/helmbench-demo-run/reports/benchmark-summary.json`
- `/tmp/helmbench-demo-run/reports/privacy-report.json`
- `/tmp/helmbench-demo-run/reports/quality-gate.json`
- `/tmp/helmbench-demo-run/docs/privacy-report.md`
- `/tmp/helmbench-demo-run/docs/dashboard.html`
- `/tmp/helmbench-demo-run/evidence/manifest.json`

Verify the demo evidence bundle:

```bash
helmbench verify-bundle \
  --bundle /tmp/helmbench-demo-run/evidence
```

## Create The Demo Repo

`init-demo-repo` is the lower-level fixture generator used by `demo-run`.

```bash
helmbench init-demo-repo \
  --repo-out /tmp/helmbench-demo-repo \
  --suite-out /tmp/helmbench-demo-suite.json \
  --force
```

The generated repo contains two tasks:

- `demo-auth-redirect-001`
- `demo-billing-rounding-001`

It also contains `scripts/demo-agent.sh`, a deterministic adapter that emits
source-free read/recommendation events and makes the expected fixes.

## Manual Pipeline

```bash
helmbench local-run \
  --suite /tmp/helmbench-demo-suite.json \
  --repo /tmp/helmbench-demo-repo \
  --adapter-command "HELMBENCH_BIN=$(pwd)/target/debug/helmbench sh scripts/demo-agent.sh" \
  --out-dir /tmp/helmbench-demo-traces

helmbench run \
  --suite /tmp/helmbench-demo-suite.json \
  --trace-dir /tmp/helmbench-demo-traces \
  --out /tmp/helmbench-demo-report.json

helmbench autopsy \
  --suite /tmp/helmbench-demo-suite.json \
  --trace-dir /tmp/helmbench-demo-traces \
  --out /tmp/helmbench-demo-autopsy.md

helmbench dashboard \
  --report /tmp/helmbench-demo-report.json \
  --out /tmp/helmbench-demo-dashboard.html
```

Expected result:

- guided task success: 100%;
- guided validation coverage: 100%;
- guided context precision: 100%;
- guided edited-file recall: 100%;
- quality gate: pass.

The generated repository is initialized with git because `local-run` clones
benchmark repos per task.
