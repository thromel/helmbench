# ctxhelm-Guided Runs

`ctxhelm-run` measures how ctxhelm context affects an agent or adapter run.

It reuses the local runner:

1. clone the target repo for each task;
2. call `ctxhelm prepare-task --no-trace`;
3. record returned target files and related tests as `recommended-file` events;
4. optionally call `ctxhelm get-pack --format json --no-trace`;
5. store only source-free pack metadata, such as token estimate;
6. run the adapter command;
7. infer edited files and run validation;
8. write normal HelmBench trace JSON.

## Command

```bash
helmbench ctxhelm-run \
  --suite /tmp/helmbench-demo-suite.json \
  --repo /tmp/helmbench-demo-repo \
  --ctxhelm-bin ctxhelm \
  --mode bug-fix \
  --pack \
  --pack-budget brief \
  --adapter-command "HELMBENCH_BIN=$(pwd)/target/debug/helmbench sh scripts/demo-agent.sh" \
  --out-dir /tmp/helmbench-ctxhelm-traces
```

Useful options:

- `--semantic`: enable ctxhelm semantic retrieval.
- `--semantic-provider`: pass a ctxhelm semantic provider.
- `--semantic-model`: pass a ctxhelm semantic model.
- `--semantic-dimensions`: pass ctxhelm semantic dimensions.
- `--pack`: call `ctxhelm get-pack`.
- `--pack-budget`: `brief`, `standard`, or `deep`.
- `--variant`: choose `ctxhelm-mcp` or `ctxhelm-pack`.
  Non-ctxhelm baselines are modeled separately as `native` and `native-search`
  traces.

## Privacy Boundary

`ctxhelm-run` does not persist raw ctxhelm pack content. Even when `--pack` is
enabled, HelmBench discards section contents and snippets after extracting
source-free metadata.

Trace files may include:

- recommended relative paths;
- command hashes for ctxhelm calls;
- pack token estimates;
- adapter read/edit/validation metadata.

Trace files must not include:

- source snippets;
- raw ctxhelm pack sections;
- raw model transcripts;
- raw terminal output;
- raw MCP payloads.
