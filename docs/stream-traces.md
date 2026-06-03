# Structured Stream Import

`stream-trace` converts Claude/Codex-style JSONL tool streams into source-free
HelmBench traces.

It is useful when an agent can emit structured events but not explicit
`helmbench record-event` calls.

For direct local, Claude, Codex, or matrix runs, `--capture-stream` can perform
the same conversion during the run. HelmBench captures stdout in memory,
extracts source-free metadata, appends normal events, and discards the raw
stream instead of writing it to disk.

## Command

```bash
helmbench stream-trace \
  --suite suites/example-auth-bugs.json \
  --stream examples/streams/claude-code/auth-redirect-001.jsonl \
  --task-id auth-redirect-001 \
  --agent claude-code \
  --variant native-search \
  --status success \
  --out-dir traces/stream-claude
```

Use `--variant native` for an agent-alone trace and `--variant native-search`
when the stream represents the agent's own repository search or built-in
context discovery without ctxhelm.

Then build reports normally:

```bash
helmbench run \
  --suite suites/example-auth-bugs.json \
  --trace-dir traces/stream-claude \
  --out reports/stream-claude.json
```

## Extraction Rules

The importer recognizes source-free metadata from common tool objects:

- `Read`, `View`, `Open` -> file read path;
- `Edit`, `MultiEdit`, `Write`, `Create`, `apply_patch` -> file edit path;
- `Bash`, `Shell`, `exec`, `run_command` -> command class/hash;
- tool inputs under `input`, `tool_input`, `parameters`, `args`, or
  JSON-encoded `arguments`;
- path fields such as `file_path`, `filePath`, `target_file`, `targetFile`,
  `filename`, `file`, and `path`;
- explicit `usage` events with total, input/output, or prompt/completion token
  counts;
- explicit `status` events with source-free task outcomes;
- explicit `eventKind` values such as `recommended_file`, `file_read`, and
  `file_edit`.

For command tools, HelmBench stores only:

- command class;
- command hash;
- touched expected test paths;
- exit status when present.

It does not store command text.

## Privacy Boundary

The input stream may contain source-bearing data depending on the agent. Treat
raw streams as local, temporary artifacts. HelmBench outputs only source-free
trace JSON.

When `--capture-stream` is used by a direct runner, the raw stream is bounded,
parsed in memory, and not persisted.

If a stream contains absolute paths, pass `--repo-root`; paths under that root
are normalized to relative paths. Absolute paths outside `--repo-root` are
ignored.
