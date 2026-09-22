# SQRust VS Code Extension — Changelog

## [0.1.1] — 2026-04-14

### Improved
- Debouncing (300 ms) on lint-on-open and lint-on-save — prevents redundant processes when opening multiple files or saving rapidly.
- Per-file process cancellation — a new lint for the same file kills the previous in-flight process.
- `sqrust.executablePath` is now `"scope": "machine"` — prevents an untrusted workspace from pointing the extension at a malicious binary.
- Added `"extensionKind": ["workspace"]` — enables remote development (SSH, Dev Containers, WSL).
- Added `untrustedWorkspaces` capability declaration with `"supported": "limited"`.
- Async `fs.realpath` with in-process cache replaces synchronous `realpathSync` — no longer blocks the extension host thread.
- Per-element validation of JSON output — malformed objects are logged and skipped instead of causing a runtime crash.
- Progress indicator (`ProgressLocation.Window`) shown during `SQRust: Check Workspace`.

## [0.1.0] — 2026-03-28

### Added
- Initial release.
- Lint on save and on open — violations appear in the Problems panel.
- `SQRust: Check File` and `SQRust: Check Workspace` commands.
- `sqrust.executablePath`, `sqrust.dialect`, `sqrust.enabled`, `sqrust.lintOnSave`, `sqrust.lintOnOpen` settings.
- Severity mapping: parse errors → Error, lint violations → Warning.
