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
  --head "name=native-search,agent=demo-search,variant=native_search,preset=claude-code,bin=scripts/demo-local-agent.sh,dangerously_skip_permissions=true" \
  --head "name=guided,agent=demo-guided,variant=ctxhelm_mcp,ctxhelm=true,ctxhelm_bin=scripts/demo-ctxhelm.sh,pack=true,pack_budget=brief,preset=claude-code,bin=scripts/demo-local-agent.sh,dangerously_skip_permissions=true" \
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
  "suite": "suites/local-run-smoke.json",
  "repo": ".",
  "outDir": ".helmbench/matrix-demo",
  "setupCommands": [],
  "failOnRegression": true,
  "qualityGate": {
    "minTaskCount": 10,
    "maxAverageTimeToFirstRelevantFileMillisDelta": 0,
    "maxTotalToolCallsDelta": 0,
    "maxTotalTokenEstimateDelta": 0,
    "maxToolCallsPerSuccessDelta": 0,
    "maxTokenEstimatePerSuccessDelta": 0
  },
  "healthMinCommits": 1,
  "allowDirtyHealth": false,
  "baseline": {
    "name": "native",
    "agent": "demo-baseline",
    "variant": "native"
  },
  "heads": [
    {
      "name": "native-search",
      "agent": "demo-search",
      "variant": "native_search",
      "preset": "claude-code",
      "bin": "scripts/demo-local-agent.sh",
      "dangerouslySkipPermissions": true
    },
    {
      "name": "guided",
      "agent": "demo-guided",
      "variant": "ctxhelm_mcp",
      "ctxhelm": true,
      "ctxhelmBin": "scripts/demo-ctxhelm.sh",
      "pack": true,
      "packBudget": "brief",
      "preset": "claude-code",
      "bin": "scripts/demo-local-agent.sh",
      "dangerouslySkipPermissions": true
    }
  ]
}
```

For public repository suites, generate a real-agent matrix config with fixture
health checked up front:

```bash
helmbench init-public-matrix \
  --preset refactoring-miner \
  --repo /tmp/RefactoringMiner \
  --suite suites/refactoring-miner-public.json \
  --out /tmp/refactoring-miner-matrix.json \
  --out-dir /tmp/refactoring-miner-matrix \
  --agent-preset claude-code \
  --dangerously-skip-permissions \
  --ctxhelm-bin ctxhelm \
  --pack \
  --force

helmbench validate-matrix \
  --config /tmp/refactoring-miner-matrix.json

helmbench run-matrix \
  --config /tmp/refactoring-miner-matrix.json \
  --force
```

The generator writes a normal `run-matrix` config with a `native` baseline row
and a `ctxhelm` guided row. It fails before writing if the suite does not match
the requested public preset or if the fixture repo fails the source-free
suite-health gate. Defaults write machine-specific configs under `.helmbench/`
so local repo paths are not accidentally committed.

CLI values override `suite`, `repo`, `outDir`, `baseline`, and `heads` when
provided. `qualityGate` configures the source-free quality gate written to
`reports/quality-gate.json`; the `--min-task-count`,
`--max-average-time-to-first-relevant-file-millis-delta`,
`--max-total-tool-calls-delta`, `--max-total-token-estimate-delta`,
`--max-tool-calls-per-success-delta`, and
`--max-token-estimate-per-success-delta` CLI flags override those optional
thresholds for one run. `healthMinCommits` and `allowDirtyHealth` control the
matrix suite-health gate. `setupCommands` from the config run before
additional `--setup-command` values. Config paths are resolved from the current
working directory. The checked-in `suites/demo-matrix.json` uses the tracked
`local-run-smoke` suite plus `scripts/demo-ctxhelm.sh`, so it can be validated
and run from a fresh HelmBench checkout without a real ctxhelm install.

Before any agent row executes, `run-matrix` writes `reports/suite-health.json`
and fails if the suite/repo preflight is unhealthy. This keeps publishable
matrix evidence tied to a checked git repo, expected file/test existence,
success-command coverage, and source-free privacy flags.

Run specs use comma-separated `key=value` fields:

- `name`: stable run identifier used in output paths;
- `agent`: source-free agent label for reports;
- `variant`: one of `native`, `native_search`, `ctxhelm_plan`, `ctxhelm_mcp`,
  `ctxhelm_pack`, or `other`;
- `native_search` is for agent-native repository search or built-in context
  discovery without ctxhelm; keep `native` for the agent-alone baseline;
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
- `preset`: optional direct-agent adapter preset, either `claude-code` or
  `codex`; when present HelmBench generates the same source-free launch command
  used by `claude-run` or `codex-run`;
- `bin` / `adapter_bin`: optional binary path for a preset, defaulting to
  `claude` or `codex`;
- `model`: optional model passed to the preset command;
- `args` / `adapterArgs`: optional extra CLI arguments for the preset command;
- `dangerouslySkipPermissions`: for `claude-code`, pass Claude Code's
  non-interactive permission bypass flag for isolated benchmark clones;
- `dangerouslyBypassApprovalsAndSandbox`: for `codex`, pass Codex's
  unrestricted mode for externally isolated benchmark clones;
- `capture_stream`: optional `true`/`false`; when true, HelmBench captures
  adapter stdout as structured JSONL, converts it to source-free events, and
  discards the raw stream. In JSON config this field is `captureStream`.

The baseline command can be omitted. In that case HelmBench still clones the
repo, runs the task validation command, infers edited files, and records a
source-free baseline trace.
Do not combine `command` and `preset` in the same run row; presets are the safer
path for real Claude/Codex matrices because they inject the source-free
`record-event` instructions automatically.

## Outputs

```text
/tmp/helmbench-matrix
├── matrix-manifest.json
├── traces/
│   ├── native/
│   ├── native-search/
│   └── guided/
├── reports/
│   ├── suite-health.json
│   ├── native.json
│   ├── native-search.json
│   ├── guided.json
│   ├── compare-native-search.json
│   ├── compare-guided.json
│   ├── benchmark-summary.json
│   ├── privacy-report.json
│   └── quality-gate.json
├── docs/
│   ├── compare-native-search.md
│   ├── compare-guided.md
│   ├── benchmark-summary.md
│   ├── quality-gate.md
│   ├── privacy-report.md
│   ├── native-autopsy.md
│   ├── native-search-autopsy.md
│   ├── guided-autopsy.md
│   ├── reproduction.md
│   └── dashboard.html
└── evidence/
    ├── health.json
    └── manifest.json
```

`matrix-manifest.json` is the top-level source-free run identity. It records the
suite path, repo path, baseline/head run labels, relative report, trace,
autopsy, and comparison paths, suite-health artifact, key artifact paths,
artifact byte counts/content hashes, quality-gate status, evidence-bundle
verification status, source-free privacy-report paths, privacy flags, and
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

`reports/privacy-report.json` and `docs/privacy-report.md` summarize the
source-free privacy contract for the matrix. They list recorded metadata classes,
forbidden raw data classes, safeguards, and per-run source-free trace/report
checks. `verify-matrix` parses the JSON privacy report and fails if it reports
raw source, prompts, transcripts, terminal logs, or non-source-free traces.

`reports/benchmark-summary.json` includes confidence metadata. HelmBench writes
95% Wilson score intervals for success and validation coverage and warns when a
suite has fewer than 10 tasks, so small demo runs are clearly marked as
directional evidence. Use `qualityGate.minTaskCount` or `--min-task-count` when
CI should fail instead of only warning on underpowered suites. Each run summary
also includes a source-free failure taxonomy for failed/skipped tasks,
validation gaps, missing relevant reads, missing expected edits,
recommendation misses, and irrelevant-read tasks. Run reports and benchmark
summaries also include command mix counts for test, build, lint, typecheck,
other, successful, and failed commands plus average time to first relevant file
when traces include timing metadata.

Verify the bundle before publishing:

```bash
helmbench verify-matrix \
  --matrix /tmp/helmbench-matrix

helmbench verify-bundle \
  --bundle /tmp/helmbench-matrix/evidence
```

`verify-matrix` validates `matrix-manifest.json`, checks that every referenced
report, trace directory, suite-health artifact, reproduction guide,
Markdown/HTML artifact, and evidence manifest exists, recomputes source-free
artifact hashes for matrix-owned files and trace JSON, and then verifies the
nested evidence bundle hashes.

Use `--fail-on-regression` when this command runs in CI and should exit
non-zero if the configured quality gate fails.

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
recall, context precision, edited-file recall, irrelevant reads, total
tool/token cost, tool/token cost per success, and average time to first relevant
file when timing is available.

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
  --baseline "name=native,agent=claude-code,variant=native,preset=claude-code,dangerously_skip_permissions=true" \
  --head "name=ctxhelm,agent=claude-code,variant=ctxhelm_mcp,ctxhelm=true,mode=bug-fix,target_agent=claude-code,pack=true,pack_budget=brief,preset=claude-code,dangerously_skip_permissions=true" \
  --force
```

The ctxhelm row records `recommended_file` events from `ctxhelm prepare-task`.
If `pack=true`, it also records token metadata from `ctxhelm get-pack` without
persisting pack sections, snippets, raw source, or raw prompts.
