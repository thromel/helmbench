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

For repeatable runs, use a JSON config:

```bash
HELMBENCH_BIN=$(pwd)/target/debug/helmbench \
  helmbench validate-matrix \
    --config suites/demo-matrix.json

HELMBENCH_BIN=$(pwd)/target/debug/helmbench \
  helmbench run-matrix \
    --config suites/demo-matrix.json \
    --force
```

Config format:

```json
{
  "suite": "suites/demo-tiny-repo.json",
  "repo": ".helmbench/demo-repo",
  "outDir": ".helmbench/matrix-demo",
  "setupCommands": [],
  "failOnRegression": true,
  "baseline": {
    "name": "native",
    "agent": "demo-baseline",
    "variant": "native"
  },
  "heads": [
    {
      "name": "guided",
      "agent": "demo-guided",
      "variant": "ctxhelm_mcp",
      "ctxhelm": true,
      "pack": true,
      "packBudget": "brief",
      "command": "HELMBENCH_BIN=${HELMBENCH_BIN:?set HELMBENCH_BIN} sh scripts/demo-agent.sh"
    }
  ]
}
```

CLI values override `suite`, `repo`, `outDir`, `baseline`, and `heads` when
provided. `setupCommands` from the config run before additional
`--setup-command` values. Config paths are resolved from the current working
directory.

Run specs use comma-separated `key=value` fields:

- `name`: stable run identifier used in output paths;
- `agent`: source-free agent label for reports;
- `variant`: one of `native`, `ctxhelm_plan`, `ctxhelm_mcp`, `ctxhelm_pack`,
  or `other`;
- `ctxhelm`: optional `true`/`false`; when true, HelmBench calls ctxhelm before
  the adapter command and records source-free recommendation events;
- `ctxhelm_bin`: optional ctxhelm binary path, default `ctxhelm`;
- `mode`: optional ctxhelm mode, default `explain`;
- `target_agent`: optional ctxhelm target agent, default `generic`;
- `semantic`: optional `true`/`false` switch passed to ctxhelm;
- `semantic_provider`, `semantic_model`, `semantic_dimensions`: optional
  semantic retrieval settings passed through to ctxhelm;
- `pack`: optional `true`/`false`; when true, HelmBench calls
  `ctxhelm get-pack --format json` and stores only source-free pack metadata;
- `pack_budget`: optional pack budget, default `brief`;
- `command`: optional adapter command executed inside each isolated task clone.

The baseline command can be omitted. In that case HelmBench still clones the
repo, runs the task validation command, infers edited files, and records a
source-free baseline trace.

## Outputs

```text
/tmp/helmbench-matrix
├── matrix-manifest.json
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

`matrix-manifest.json` is the top-level source-free run identity. It records the
suite path, repo path, baseline/head run labels, relative report and trace
paths, key artifact paths, quality-gate status, evidence-bundle verification
status, and source-free privacy flags.

Verify the bundle before publishing:

```bash
helmbench verify-bundle \
  --bundle /tmp/helmbench-matrix/evidence
```

Use `--fail-on-regression` when this command runs in CI and should exit
non-zero if the default quality gate fails.

## ctxhelm Row

```bash
helmbench run-matrix \
  --suite /tmp/refactoringminer-suite.json \
  --repo /tmp/RefactoringMiner \
  --out-dir /tmp/refactoringminer-matrix \
  --baseline "name=native,agent=claude-code,variant=native,command=claude --print" \
  --head "name=ctxhelm,agent=claude-code,variant=ctxhelm_mcp,ctxhelm=true,mode=bug-fix,target_agent=claude-code,pack=true,pack_budget=brief,command=claude --print" \
  --force
```

The ctxhelm row records `recommended_file` events from `ctxhelm prepare-task`.
If `pack=true`, it also records token metadata from `ctxhelm get-pack` without
persisting pack sections, snippets, raw source, or raw prompts.
