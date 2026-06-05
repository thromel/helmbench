# HelmBench Privacy Report: `refactoringminer-git-regressions`

This source-free report describes what HelmBench recorded for a run matrix and which raw data classes were intentionally excluded.

## Summary

| Field | Value |
| --- | ---: |
| Tasks | 10 |
| Runs | 2 |
| Traces | 20 |
| Source-free | yes |

## Run Checks

| Run | Variant | Report source-free | Source-free traces | Raw source | Raw prompts | Raw transcripts | Raw terminal logs |
| --- | --- | --- | ---: | --- | --- | --- | --- |
| `native` | `Native` | yes | 10/10 | no | no | no | no |
| `ctxhelm` | `CtxhelmMcp` | yes | 10/10 | no | no | no | no |

## Recorded Metadata

- task ids
- agent labels
- variant labels
- relative file paths and path hashes
- command classes and command hashes
- touched expected-test paths
- exit statuses
- task status
- tool-call counts
- token estimates
- elapsed timing metadata
- ctxhelm recommendation paths
- ctxhelm pack token estimates
- artifact byte counts and hashes

## Forbidden Raw Data

- raw source
- raw prompts
- raw model transcripts
- raw terminal logs
- raw adapter commands
- raw setup commands
- raw MCP payloads
- raw ctxhelm pack sections or snippets
- secrets

## Safeguards

- trace and report readers reject privacy flags that indicate raw data logging
- structured streams are parsed in memory and not persisted by capture-stream
- adapter and setup commands are stored as source-free hashes
- matrix artifact paths are safe relative paths
- published matrix artifacts are byte-counted and content-hashed
- evidence bundles are verified before the matrix manifest is written

## Privacy Flags

- Source-free: `true`
- Raw source logged: `false`
- Raw prompts logged: `false`
- Raw transcripts logged: `false`
- Raw terminal logs logged: `false`
