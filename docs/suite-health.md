# Suite Health

`suite-health` checks whether a source-free task suite is trustworthy enough to
run against a local git repository.

This is a preflight gate for real benchmark claims. A benchmark can look good
for the wrong reasons if expected files are missing, expected tests are wrong,
the repo checkout is dirty, or tasks lack validation commands.

## Command

```bash
helmbench suite-health \
  --suite /tmp/helmbench-demo-suite.json \
  --repo /tmp/helmbench-demo-repo \
  --out /tmp/helmbench-suite-health.json
```

Markdown output is also supported:

```bash
helmbench suite-health \
  --suite /tmp/helmbench-demo-suite.json \
  --repo /tmp/helmbench-demo-repo \
  --out /tmp/helmbench-suite-health.md \
  --format markdown
```

For outcome benchmark proof, check whether each task's validation command fails
before any agent runs:

```bash
helmbench suite-health \
  --preset refactoring-miner \
  --suite suites/my-suite.json \
  --repo ~/work/example-repo \
  --out /tmp/example-health.json \
  --check-success-commands \
  --fail-fast-success-commands \
  --require-setup-commands
```

With this flag, HelmBench runs each `successCommand` inside an isolated clone
after that task's `setupCommands`, if any. It stores only source-free command
metadata: task id, command class, command hash, exit status, timeout status,
and elapsed milliseconds. If validation already passes before the agent edits
the repo, `validationBaselineReady` is `false` and the health command exits
non-zero after writing the report. If a task setup command fails, the report
records only that task id in `tasksFailedSetupCommand` and marks the baseline
not ready. `--fail-fast-success-commands` stops after the first pre-agent pass
and records the remaining tasks as skipped, which is useful for large public
suites. Use `--preset` for generated public suites so the report includes the
preset label and preset-specific anchor-file checks.

The report also includes `evidenceUse`:

- `outcome_ready`: fixture health is good and validation fails before the agent;
- `navigation_only`: fixture health is good, but task-success claims are not proven;
- `unhealthy`: the suite or repo preflight is not trustworthy enough to publish.

Use `--require-setup-commands` when an outcome suite should keep the fixture
repo healthy at rest and seed each task's failing state inside the isolated
clone. The report records only task ids in `tasksMissingSetupCommand` when a
task lacks setup seeding.

By default, HelmBench requires a clean checkout and at least one commit. For
local exploratory runs, a dirty checkout can be allowed explicitly:

```bash
helmbench suite-health \
  --suite suites/my-suite.json \
  --repo ~/work/example-repo \
  --out /tmp/example-health.json \
  --allow-dirty
```

## What It Checks

The report records only source-free metadata:

- suite name and task count;
- repo basename, not an absolute path;
- git HEAD hash and commit count;
- whether the checkout is dirty;
- whether `git fsck --no-progress` passed;
- expected file/test counts;
- missing expected files and tests;
- tasks missing `successCommand`;
- whether task-level setup commands were required;
- tasks missing task-level setup commands when required;
- tasks whose per-task setup command failed;
- optional validation-baseline status for `successCommand`s, including whether
  the check ran in fail-fast mode;
- `evidenceUse`, a source-free classification for how the report may be used;
- source-free privacy flags.

It does not store raw source, prompts beyond suite task prompts, transcripts,
terminal logs, stdout/stderr, or command text.

## Healthy Criteria

A suite is healthy when:

- the repo is a git checkout;
- `HEAD` can be resolved;
- commit count is at least `--min-commits`;
- the checkout is clean, unless `--allow-dirty` is set;
- git fsck passes;
- every expected file and expected test exists;
- every task has a non-empty `successCommand`.
- if `--require-setup-commands` is set, every task has at least one
  `setupCommands` entry.

When `--check-success-commands` is enabled, the suite is additionally healthy
only when every `successCommand` fails before the agent runs. This prevents
publishing task-success claims for suites whose validation commands already pass
on a clean checkout. Task-level `setupCommands` are applied inside the isolated
clone before the validation command, so suites can seed a failing state without
committing broken files to the fixture repo.

If the suite is unhealthy, HelmBench writes the report first and then exits
non-zero. This makes it useful in CI because the failure still leaves a
diagnostic artifact.

## Evidence Bundles

Use the generated health report with `evidence-bundle`:

```bash
helmbench evidence-bundle \
  --suite suites/my-suite.json \
  --health /tmp/example-health.json \
  --base-report reports/native.json \
  --head-report reports/ctxhelm.json \
  --out-dir /tmp/helmbench-evidence \
  --force
```

`verify-bundle` validates the copied health report path and source-free privacy
flags along with the rest of the evidence manifest.
