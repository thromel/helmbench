# Demo Benchmark

`init-demo-repo` creates a tiny reproducible benchmark repository and matching
suite. It is meant for smoke testing HelmBench itself and for demos where a real
agent call would be too slow or expensive.

## Create The Demo Repo

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

## Run The Full Pipeline

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

- task success: 100%;
- validation coverage: 100%;
- context precision: 100%;
- edited-file recall: 100%;
- no high-risk autopsy findings.

The generated repository is initialized with git because `local-run` clones
benchmark repos per task.
