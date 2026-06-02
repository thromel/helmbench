# Run Matrix

`run-matrix` is the publishable benchmark workflow. It runs one baseline and
one or more local adapter variants over the same suite, then writes the
source-free artifacts needed to compare agent behavior.

## Command

```bash
helmbench run-matrix \
  --suite /tmp/helmbench-demo-suite.json \
  --repo /tmp/helmbench-demo-repo \
  --out-dir /tmp/helmbench-matrix \
  --baseline "name=native,agent=demo-baseline,variant=native" \
  --head "name=guided,agent=demo-guided,variant=ctxhelm_mcp,command=HELMBENCH_BIN=$(pwd)/target/debug/helmbench sh scripts/demo-agent.sh" \
  --force
```

Run specs use comma-separated `key=value` fields:

- `name`: stable run identifier used in output paths;
- `agent`: source-free agent label for reports;
- `variant`: one of `native`, `ctxhelm_plan`, `ctxhelm_mcp`, `ctxhelm_pack`,
  or `other`;
- `command`: optional adapter command executed inside each isolated task clone.

The baseline command can be omitted. In that case HelmBench still clones the
repo, runs the task validation command, infers edited files, and records a
source-free baseline trace.

## Outputs

```text
/tmp/helmbench-matrix
├── traces/
│   ├── native/
│   └── guided/
├── reports/
│   ├── native.json
│   ├── guided.json
│   ├── compare-guided.json
│   ├── benchmark-summary.json
│   └── quality-gate.json
├── docs/
│   ├── compare-guided.md
│   ├── benchmark-summary.md
│   ├── quality-gate.md
│   ├── native-autopsy.md
│   └── dashboard.html
└── evidence/
    └── manifest.json
```

Verify the bundle before publishing:

```bash
helmbench verify-bundle \
  --bundle /tmp/helmbench-matrix/evidence
```

Use `--fail-on-regression` when this command runs in CI and should exit
non-zero if the default quality gate fails.
