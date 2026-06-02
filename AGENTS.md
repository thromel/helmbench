# AGENTS.md

## Project Goal

Build HelmBench as a local, source-free benchmark and observability harness for
AI coding agents.

## Working Rules

- Keep reports source-free by default.
- Do not store raw source, raw transcripts, raw terminal logs, secrets, or raw
  MCP traffic.
- Prefer typed JSON contracts over ad hoc text output.
- Add tests for metric calculations, privacy checks, and CLI contract changes.
- Treat direct agent execution as an adapter layer; do not fake agent success in
  core metrics.

## Validation

- Run `cargo fmt --check`.
- Run `cargo test`.
- Run `cargo run -- --help` after CLI changes.
