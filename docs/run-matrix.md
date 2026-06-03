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
  "healthMinCommits": 1,
  "allowDirtyHealth": false,
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
      "captureStream": true,
      "command": "HELMBENCH_BIN=${HELMBENCH_BIN:?set HELMBENCH_BIN} sh scripts/demo-agent.sh"
    }
  ]
}
```

CLI values override `suite`, `repo`, `outDir`, `baseline`, and `heads` when
provided. `healthMinCommits` and `allowDirtyHealth` control the matrix
suite-health gate. `setupCommands` from the config run before additional
`--setup-command` values. Config paths are resolved from the current working
directory.

Before any agent row executes, `run-matrix` writes `reports/suite-health.json`
and fails if the suite/repo preflight is unhealthy. This keeps publishable
matrix evidence tied to a checked git repo, expected file/test existence,
success-command coverage, and source-free privacy flags.

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
- `command`: optional adapter command executed inside each isolated task clone;
- `capture_stream`: optional `true`/`false`; when true, HelmBench captures
  adapter stdout as structured JSONL, converts it to source-free events, and
  discards the raw stream. In JSON config this field is `captureStream`.

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
│   ├── suite-health.json
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
│   ├── reproduction.md
│   └── dashboard.html
└── evidence/
    ├── health.json
    └── manifest.json
```

`matrix-manifest.json` is the top-level source-free run identity. It records the
suite path, repo path, baseline/head run labels, relative report and trace
paths, suite-health artifact, key artifact paths, quality-gate status,
evidence-bundle verification status, source-free privacy flags, and
reproducibility provenance.

The provenance block includes the HelmBench version, suite content hash, repo
HEAD, dirty-checkout flag, setup-command count, and setup-command hashes. Each
run row also records adapter command and ctxhelm configuration hashes when
present. The manifest does not store raw adapter commands, setup commands,
prompts, transcripts, terminal logs, ctxhelm pack sections, or source content.

`docs/reproduction.md` is generated from the source-free matrix manifest. It
lists verification commands, run identity, artifact paths, command/config
hashes, and rerun notes using placeholders such as `<matrix-dir>` instead of
raw local commands.

`reports/benchmark-summary.json` includes confidence metadata. HelmBench writes
95% Wilson score intervals for success and validation coverage and warns when a
suite has fewer than 10 tasks, so small demo runs are clearly marked as
directional evidence. Each run summary also includes a source-free failure
taxonomy for failed/skipped tasks, validation gaps, missing relevant reads,
missing expected edits, recommendation misses, and irrelevant-read tasks.
Run reports and benchmark summaries also include command mix counts for test,
build, lint, typecheck, other, successful, and failed commands plus average
time to first relevant file when traces include timing metadata.

Verify the bundle before publishing:

```bash
helmbench verify-matrix \
  --matrix /tmp/helmbench-matrix

helmbench verify-bundle \
  --bundle /tmp/helmbench-matrix/evidence
```

`verify-matrix` validates `matrix-manifest.json`, checks that every referenced
report, trace directory, suite-health artifact, reproduction guide,
Markdown/HTML artifact, and evidence manifest exists, and then verifies the
nested evidence bundle hashes.

Use `--fail-on-regression` when this command runs in CI and should exit
non-zero if the default quality gate fails.

## Longitudinal History

Use `matrix-history` to compare verified matrix outputs across repeated runs:

```bash
helmbench matrix-history \
  --matrix /tmp/helmbench-matrix-week-1 \
  --matrix /tmp/helmbench-matrix-week-2 \
  --out /tmp/helmbench-matrix-history.md

helmbench matrix-history \
  --matrix /tmp/helmbench-matrix-week-1 \
  --matrix /tmp/helmbench-matrix-week-2 \
  --format json \
  --out /tmp/helmbench-matrix-history.json

helmbench matrix-history \
  --matrix /tmp/helmbench-matrix-week-1 \
  --matrix /tmp/helmbench-matrix-week-2 \
  --format html \
  --out /tmp/helmbench-matrix-history.html
```

The command verifies every matrix first, loads each matrix's
`reports/benchmark-summary.json`, requires matching suite and run names, and
reports first-to-last deltas for success, validation coverage, recommendation
recall, context precision, edited-file recall, irrelevant reads, tool calls,
token estimates, and average time to first relevant file when timing is
available.

The history report is source-free in Markdown, JSON, and HTML forms. It does
not include raw source, prompts, transcripts, terminal logs, MCP payloads, or
absolute local matrix paths. The HTML output is a static dashboard with no
JavaScript or remote assets, suitable for publishing alongside matrix evidence.

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
