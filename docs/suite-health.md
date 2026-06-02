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
- source-free privacy flags.

It does not store raw source, prompts beyond suite task prompts, transcripts,
terminal logs, or command text.

## Healthy Criteria

A suite is healthy when:

- the repo is a git checkout;
- `HEAD` can be resolved;
- commit count is at least `--min-commits`;
- the checkout is clean, unless `--allow-dirty` is set;
- git fsck passes;
- every expected file and expected test exists;
- every task has a non-empty `successCommand`.

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
