# Agent Diff Autopsy

`helmbench autopsy` turns source-free traces into a reviewer-style diagnosis of
agent behavior.

It answers:

- Did the agent finish successfully?
- Did it run expected validation?
- Did it edit files outside the expected target set?
- Did it edit files without a recorded read event?
- Did it miss expected files entirely?

## Command

```bash
helmbench autopsy \
  --suite suites/example-auth-bugs.json \
  --trace-dir examples/traces/native \
  --out docs/example-autopsy.md
```

Use JSON output for automation:

```bash
helmbench autopsy \
  --suite suites/example-auth-bugs.json \
  --trace-dir examples/traces/native \
  --format json \
  --out reports/example-autopsy.json
```

## Inputs

Autopsy uses only:

- suite expectations;
- trace paths;
- command classes and exit statuses;
- final task statuses;
- source-free privacy flags.

It does not read source files, raw prompts, raw transcripts, raw terminal logs,
or raw MCP payloads.

## Risk Levels

- `Low`: no source-free autopsy issues detected.
- `Medium`: expected files were neither read nor edited.
- `High`: task failed, validation was missing, overbroad edits occurred, or an
  edited file had no recorded read event.
