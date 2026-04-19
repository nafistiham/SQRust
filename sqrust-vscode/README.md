# SQRust — VS Code Extension

Fast SQL linting in your editor. Powered by [SQRust](https://github.com/nafistiham/SQRust) — the Ruff for SQL.

## Features

- **Lint on save and open** — violations appear instantly in the Problems panel.
- **SQRust: Check File** — lint the active SQL file on demand.
- **SQRust: Check Workspace** — lint all SQL files in the workspace.
- **Dialect-aware** — set `sqrust.dialect` to `bigquery`, `snowflake`, `duckdb`, `postgres`, or `mysql`.
- **330 rules** covering Convention, Layout, Lint, Structure, Ambiguous, and Capitalisation.

## Requirements

The `sqrust` binary must be installed and on your `PATH`.

**Homebrew (macOS):**

```bash
brew install nafistiham/tap/sqrust
```

**One-line installer (macOS / Linux):**

```bash
curl -sSL https://raw.githubusercontent.com/nafistiham/SQRust/main/install.sh | sh
```

**Cargo:**

```bash
cargo install sqrust-cli
```

If the binary is not on `PATH`, set `sqrust.executablePath` in your user settings to the full path.

## Extension Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `sqrust.executablePath` | `"sqrust"` | Path to the sqrust binary. Set in **user** settings (machine-scoped). |
| `sqrust.dialect` | `""` | SQL dialect. Leave empty to use `sqrust.toml` or ANSI. |
| `sqrust.enabled` | `true` | Enable or disable linting. |
| `sqrust.lintOnSave` | `true` | Lint when a `.sql` file is saved. |
| `sqrust.lintOnOpen` | `true` | Lint when a `.sql` file is opened. |

## Configuration File

Create a `sqrust.toml` in your project root to configure rules and exclusions:

```toml
[sqrust]
dialect = "bigquery"
exclude = ["dbt_packages/**", "target/**"]

[rules]
disable = [
    "Convention/SelectStar",
    "Layout/LongLines",
]
```

Settings in `sqrust.toml` are picked up automatically — no VS Code settings needed for per-project config.

## Dialect Support

Set `sqrust.dialect` in workspace settings or in `sqrust.toml`:

- `ansi` (default)
- `bigquery`
- `snowflake`
- `duckdb`
- `postgres` / `postgresql`
- `mysql`

## Remote Development

The extension declares `"extensionKind": ["workspace"]`, so it runs on the remote side in SSH, Dev Containers, and WSL environments — linting happens where the files are, using the remote `sqrust` binary.

## License

MIT
