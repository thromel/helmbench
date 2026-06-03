# Agent Diff Autopsy

HelmBench has two reviewer-style autopsy commands:

- `helmbench autopsy` diagnoses source-free agent traces.
- `helmbench diff-autopsy` diagnoses a git worktree or branch diff against one
  source-free benchmark task.

Both commands report paths and counts only. They do not read source files,
patch hunks, raw prompts, raw transcripts, raw terminal logs, or raw MCP
payloads.

## Trace Autopsy

`helmbench autopsy` turns source-free traces into a reviewer-style diagnosis of
agent behavior.

It answers:

- Did the agent finish successfully?
- Did it run expected validation?
- Did it edit files outside the expected target set?
- Did it edit files without a recorded read event?
- Did it miss expected files entirely?

### Command

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

### Inputs

Autopsy uses only:

- suite expectations;
- trace paths;
- command classes and exit statuses;
- final task statuses;
- source-free privacy flags.

It does not read source files, raw prompts, raw transcripts, raw terminal logs,
or raw MCP payloads.

### Risk Levels

- `Low`: no source-free autopsy issues detected.
- `Medium`: expected files were neither read nor edited.
- `High`: task failed, validation was missing, overbroad edits occurred, or an
  edited file had no recorded read event.

## Diff Autopsy

`helmbench diff-autopsy` compares changed git paths with a task's expected
source and test paths. This is useful for PR review and agent-created branch
inspection when no trace is available.

### Command

Analyze the current worktree:

```bash
helmbench diff-autopsy \
  --suite suites/example-auth-bugs.json \
  --repo . \
  --task-id auth-redirect-001 \
  --out reports/example-diff-autopsy.md
```

Analyze a branch or PR-style ref comparison:

```bash
helmbench diff-autopsy \
  --suite suites/example-auth-bugs.json \
  --repo . \
  --task-id auth-redirect-001 \
  --base-ref origin/main \
  --head-ref HEAD \
  --out reports/example-diff-autopsy.json \
  --format json
```

Analyze a GitHub PR by changed file names only:

```bash
helmbench diff-autopsy \
  --suite suites/example-auth-bugs.json \
  --repo . \
  --task-id auth-redirect-001 \
  --pr 42 \
  --out reports/example-pr-autopsy.md
```

If the current checkout is not connected to the target GitHub repository, pass
`--github-repo OWNER/REPO`. HelmBench shells out to `gh pr diff --name-only`,
never `--patch`; numeric PR identifiers are stored as `pr:<number>`, while URL
or branch identifiers are stored as source-free hashes.

### Inputs

Diff autopsy uses only:

- suite expectations;
- task id;
- changed file paths from `git status --short` or `git diff --name-only`;
- changed file paths from `gh pr diff --name-only` when `--pr` is used;
- base/head ref labels;
- source-free privacy flags.

It does not inspect patch content. A diff autopsy can say a patch changed or
did not change expected paths; it cannot prove what the agent read. Use trace
autopsy when read/edit/test sequence data is available.

### Risk Levels

- `Low`: the diff changes all expected source paths, expected test paths, and
  no extra paths.
- `Medium`: the diff changes expected source paths but skips some expected
  test or source paths that may still be intentionally unchanged.
- `High`: the diff is empty, changes no expected source path, or changes paths
  outside the task's expected source/test set.
