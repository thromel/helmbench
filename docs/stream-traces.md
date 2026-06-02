# Structured Stream Import

`stream-trace` converts Claude/Codex-style JSONL tool streams into source-free
HelmBench traces.

It is useful when an agent can emit structured events but not explicit
`helmbench record-event` calls.

## Command

```bash
helmbench stream-trace \
  --suite suites/example-auth-bugs.json \
  --stream examples/streams/claude-code/auth-redirect-001.jsonl \
  --task-id auth-redirect-001 \
  --agent claude-code \
  --variant native \
  --status success \
  --out-dir traces/stream-claude
```

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

If a stream contains absolute paths, pass `--repo-root`; paths under that root
are normalized to relative paths. Absolute paths outside `--repo-root` are
ignored.
