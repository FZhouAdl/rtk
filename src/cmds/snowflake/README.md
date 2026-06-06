# Snowflake Ecosystem

> Part of [`src/cmds/`](../README.md) — see also [docs/contributing/TECHNICAL.md](../../../docs/contributing/TECHNICAL.md)

## Specifics

- `cortex_cmd.rs` handles **Snowflake Cortex Code CLI** (`cortex`), an interactive AI coding assistant.
- Interactive sessions (no args) and batch mode (`-p "<prompt>"`) pass through unfiltered.
- Non-interactive management commands get filtered output:
  - `cortex --version` — compacts to version number only
  - `cortex mcp list` — compacts JSON output to `status name` lines
  - `cortex update` — passes through (progress output)

## Cross-command

- No cross-command dependencies yet. Future support may add `snowsql` filter.
