# Public Benchmark Suites

`init-public-suite` creates benchmark suites for known public repositories and
writes a source-free health report before the suite is trusted.

The command is for larger, recruiter-readable runs where the tiny demo fixture
is too small to prove navigation quality. The first preset is
`refactoring-miner`, a Java/Gradle codebase with a long Git history and real
MCP, web diff, AST diff, and git-history components.

## RefactoringMiner

```bash
helmbench init-public-suite \
  --preset refactoring-miner \
  --repo ../ctxhelm-proof-fixtures/RefactoringMiner \
  --suite-out /tmp/refactoringminer-suite.json \
  --health-out /tmp/refactoringminer-health.json \
  --force
```

The health report records only metadata:

- preset name;
- repository basename;
- HEAD commit SHA;
- commit count;
- dirty/clean status;
- `git fsck` pass/fail status;
- checked relative paths;
- missing relative paths.

It does not store raw source, prompts beyond suite task prompts, transcripts,
terminal logs, or absolute repository paths.

## Included Tasks

The RefactoringMiner preset currently emits four source-free tasks:

- `rm-mcp-intent-validation-001`
- `rm-mcp-tools-contract-001`
- `rm-webdiff-viewed-files-001`
- `rm-git-history-merge-001`

Each task contains expected source files, expected test files, tags, timeout
metadata, and a targeted Gradle `successCommand`. The suite is meant to compare
agent navigation and validation behavior across variants such as native agent
runs, ctxhelm plan traces, ctxhelm-guided runs, and ctxhelm pack runs.

## Run Pattern

After generating the suite, the usual HelmBench flow applies:

```bash
helmbench ctxhelm-trace \
  --suite /tmp/refactoringminer-suite.json \
  --repo ../ctxhelm-proof-fixtures/RefactoringMiner \
  --out-dir /tmp/refactoringminer-ctxhelm-plan

helmbench run \
  --suite /tmp/refactoringminer-suite.json \
  --trace-dir /tmp/refactoringminer-ctxhelm-plan \
  --out /tmp/refactoringminer-ctxhelm-plan-report.json
```

For full agent runs, use `claude-run`, `codex-run`, `local-run`, or
`ctxhelm-run` with the same suite. HelmBench clones the repository per task, so
the source repository must be a healthy Git checkout.

## Health Failures

If the repository is dirty, corrupt, too shallow, or missing expected anchor
files, `init-public-suite` writes the health JSON and exits with an error. This
makes failed benchmark setup inspectable without leaking source.
