import * as vscode from "vscode";
import * as child_process from "child_process";
import * as fs from "fs";
import * as path from "path";

interface SqrustViolation {
  file: string;
  line: number;
  col: number;
  rule: string;
  message: string;
  severity: "error" | "warning";
}

let diagnosticCollection: vscode.DiagnosticCollection;
let outputChannel: vscode.OutputChannel;

/** Pending debounce timers keyed by file URI string. */
const debounceTimers = new Map<string, ReturnType<typeof setTimeout>>();

/** Active child processes keyed by file URI string (or "__workspace__"). */
const activeProcs = new Map<string, child_process.ChildProcess>();

/** Async realpath cache — avoids repeated I/O for the same path. */
const realpathCache = new Map<string, string>();

const DEBOUNCE_MS = 300;

export function activate(context: vscode.ExtensionContext): void {
  diagnosticCollection = vscode.languages.createDiagnosticCollection("sqrust");
  outputChannel = vscode.window.createOutputChannel("SQRust");
  context.subscriptions.push(diagnosticCollection, outputChannel);

  // Lint on open.
  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument((doc) => {
      if (isEnabled() && lintOnOpen() && isSql(doc)) {
        scheduleLint(doc);
      }
    })
  );

  // Lint on save.
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument((doc) => {
      if (isEnabled() && lintOnSave() && isSql(doc)) {
        scheduleLint(doc);
      }
    })
  );

  // Clear diagnostics when a document is closed.
  context.subscriptions.push(
    vscode.workspace.onDidCloseTextDocument((doc) => {
      cancelPending(doc.uri.toString());
      diagnosticCollection.delete(doc.uri);
    })
  );

  // Manual check commands.
  context.subscriptions.push(
    vscode.commands.registerCommand("sqrust.checkFile", () => {
      const editor = vscode.window.activeTextEditor;
      if (editor && isSql(editor.document)) {
        lintDocument(editor.document);
      }
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("sqrust.checkWorkspace", () => {
      lintWorkspace();
    })
  );

  // Lint all currently-open SQL files on activation.
  if (isEnabled()) {
    vscode.workspace.textDocuments.forEach((doc) => {
      if (isSql(doc)) {
        scheduleLint(doc);
      }
    });
  }
}

export function deactivate(): void {
  // Cancel all pending timers and kill active processes.
  for (const timer of debounceTimers.values()) {
    clearTimeout(timer);
  }
  debounceTimers.clear();

  for (const proc of activeProcs.values()) {
    proc.kill();
  }
  activeProcs.clear();

  diagnosticCollection?.dispose();
  outputChannel?.dispose();
}

// ─── Debounce ────────────────────────────────────────────────────────────────

function scheduleLint(doc: vscode.TextDocument): void {
  const key = doc.uri.toString();
  const existing = debounceTimers.get(key);
  if (existing !== undefined) {
    clearTimeout(existing);
  }
  const timer = setTimeout(() => {
    debounceTimers.delete(key);
    lintDocument(doc);
  }, DEBOUNCE_MS);
  debounceTimers.set(key, timer);
}

function cancelPending(key: string): void {
  const timer = debounceTimers.get(key);
  if (timer !== undefined) {
    clearTimeout(timer);
    debounceTimers.delete(key);
  }
  const proc = activeProcs.get(key);
  if (proc !== undefined) {
    proc.kill();
    activeProcs.delete(key);
  }
}

// ─── Core lint logic ─────────────────────────────────────────────────────────

function lintDocument(doc: vscode.TextDocument): void {
  const filePath = doc.uri.fsPath;
  const key = doc.uri.toString();
  const args = buildArgs(filePath);
  const executable = getExecutablePath();

  // Cancel any in-flight lint for the same file.
  cancelPending(key);

  runSqrust(executable, args, filePath, key, (violations, error) => {
    if (error) {
      handleRunError(error, executable);
      // Keep existing diagnostics rather than silently clearing them.
      return;
    }
    resolvePathAsync(filePath).then((resolved) => {
      const diagnostics = violations
        .filter((v) => {
          // Compare resolved paths to handle symlinks (e.g. /var → /private/var on macOS).
          const vResolved = realpathCacheGet(v.file);
          return vResolved !== null ? vResolved === resolved : v.file === filePath;
        })
        .map(toDiagnostic);
      diagnosticCollection.set(doc.uri, diagnostics);
    });
  });
}

function lintWorkspace(): void {
  const folders = vscode.workspace.workspaceFolders;
  if (!folders || folders.length === 0) {
    vscode.window.showWarningMessage("SQRust: No workspace folder open.");
    return;
  }
  const workspacePath = folders[0].uri.fsPath;
  const args = buildArgs(workspacePath);
  const executable = getExecutablePath();
  const key = "__workspace__";

  // Cancel any in-flight workspace lint.
  cancelPending(key);

  // Clear all diagnostics before a workspace-wide re-lint so stale entries
  // from fixed files do not persist.
  diagnosticCollection.clear();

  vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Window,
      title: "SQRust: linting workspace…",
      cancellable: false,
    },
    () =>
      new Promise<void>((resolve) => {
        runSqrust(executable, args, workspacePath, key, (violations, error) => {
          resolve();
          if (error) {
            handleRunError(error, executable);
            return;
          }
          // Resolve all file paths, then group violations by resolved path.
          const resolveAll = violations.map((v) =>
            resolvePathAsync(v.file).then((resolved) => ({ v, resolved }))
          );
          Promise.all(resolveAll).then((entries) => {
            const byFile = new Map<string, SqrustViolation[]>();
            for (const { v, resolved } of entries) {
              const existing = byFile.get(resolved) ?? [];
              existing.push(v);
              byFile.set(resolved, existing);
            }
            for (const [resolvedPath, fileViolations] of byFile) {
              const uri = vscode.Uri.file(resolvedPath);
              diagnosticCollection.set(uri, fileViolations.map(toDiagnostic));
            }
          });
        });
      })
  );
}

// ─── Process execution ───────────────────────────────────────────────────────

function runSqrust(
  executable: string,
  args: string[],
  targetPath: string,
  key: string,
  callback: (violations: SqrustViolation[], error: Error | null) => void
): void {
  let stdout = "";
  let stderr = "";

  // Use workspace root as cwd so sqrust.toml discovery works.
  const cwd =
    vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ??
    path.dirname(targetPath);

  const proc = child_process.spawn(executable, args, { cwd });
  activeProcs.set(key, proc);

  proc.stdout.on("data", (chunk: Buffer) => {
    stdout += chunk.toString();
  });

  proc.stderr.on("data", (chunk: Buffer) => {
    stderr += chunk.toString();
  });

  proc.on("error", (err: Error) => {
    activeProcs.delete(key);
    callback([], err);
  });

  proc.on("close", (code: number | null) => {
    activeProcs.delete(key);

    if (code === null) {
      // Killed by a signal — this is expected when we cancel; ignore silently.
      return;
    }
    // Exit code 2 = tool-level error (unknown dialect, config parse error, etc.).
    if (code === 2) {
      callback(
        [],
        new Error(`sqrust exited with code 2: ${stderr.trim().slice(0, 200)}`)
      );
      return;
    }
    if (!stdout.trim()) {
      callback([], null);
      return;
    }
    let parsed: unknown;
    try {
      parsed = JSON.parse(stdout);
    } catch {
      callback(
        [],
        new Error(`Failed to parse sqrust output: ${stdout.slice(0, 200)}`)
      );
      return;
    }
    if (!Array.isArray(parsed)) {
      callback(
        [],
        new Error(
          `sqrust output was not a JSON array: ${stdout.slice(0, 200)}`
        )
      );
      return;
    }
    // Validate each element's shape before using it.
    const violations: SqrustViolation[] = [];
    for (const item of parsed) {
      if (!isValidViolation(item)) {
        outputChannel.appendLine(
          `[warn] Skipping malformed violation: ${JSON.stringify(item)}`
        );
        continue;
      }
      violations.push(item as SqrustViolation);
    }
    callback(violations, null);
  });
}

// ─── Conversion helpers ──────────────────────────────────────────────────────

export function toDiagnostic(v: SqrustViolation): vscode.Diagnostic {
  // sqrust reports 1-indexed lines and columns; VS Code uses 0-indexed.
  const line = Math.max(0, v.line - 1);
  const col = Math.max(0, v.col - 1);
  const range = new vscode.Range(line, col, line, col + 1);
  const severity =
    v.severity === "error"
      ? vscode.DiagnosticSeverity.Error
      : vscode.DiagnosticSeverity.Warning;
  const diag = new vscode.Diagnostic(
    range,
    `[${v.rule}] ${v.message}`,
    severity
  );
  diag.source = "sqrust";
  diag.code = v.rule;
  return diag;
}

// ─── Validation ──────────────────────────────────────────────────────────────

function isValidViolation(item: unknown): item is SqrustViolation {
  if (typeof item !== "object" || item === null) {
    return false;
  }
  const v = item as Record<string, unknown>;
  return (
    typeof v["file"] === "string" &&
    typeof v["line"] === "number" &&
    typeof v["col"] === "number" &&
    typeof v["rule"] === "string" &&
    typeof v["message"] === "string" &&
    (v["severity"] === "error" || v["severity"] === "warning")
  );
}

// ─── Config helpers ──────────────────────────────────────────────────────────

function cfg(): vscode.WorkspaceConfiguration {
  return vscode.workspace.getConfiguration("sqrust");
}

function isEnabled(): boolean {
  return cfg().get<boolean>("enabled", true);
}

function lintOnSave(): boolean {
  return cfg().get<boolean>("lintOnSave", true);
}

function lintOnOpen(): boolean {
  return cfg().get<boolean>("lintOnOpen", true);
}

function getExecutablePath(): string {
  return cfg().get<string>("executablePath", "sqrust") || "sqrust";
}

function getDialect(): string {
  return cfg().get<string>("dialect", "") ?? "";
}

export function buildArgs(targetPath: string): string[] {
  const args = ["check", "--format", "json"];
  const dialect = getDialect();
  if (dialect) {
    args.push("--dialect", dialect);
  }
  args.push(targetPath);
  return args;
}

// ─── Utility ─────────────────────────────────────────────────────────────────

export function isSql(doc: vscode.TextDocument): boolean {
  return (
    doc.languageId === "sql" || doc.uri.fsPath.toLowerCase().endsWith(".sql")
  );
}

function isNotFound(err: Error): boolean {
  return (err as NodeJS.ErrnoException).code === "ENOENT";
}

function handleRunError(error: Error, executable: string): void {
  if (isNotFound(error)) {
    vscode.window.showErrorMessage(
      `SQRust: '${executable}' not found. Install with: brew install nafistiham/tap/sqrust`
    );
  } else {
    outputChannel.appendLine(`[error] ${error.message}`);
  }
}

/**
 * Resolve a path through symlinks asynchronously.
 * Result is cached so repeated calls for the same path are free.
 */
async function resolvePathAsync(p: string): Promise<string> {
  const cached = realpathCache.get(p);
  if (cached !== undefined) {
    return cached;
  }
  return new Promise((resolve) => {
    fs.realpath(p, (err, resolved) => {
      const result = err ? path.normalize(p) : resolved;
      realpathCache.set(p, result);
      resolve(result);
    });
  });
}

/**
 * Synchronous cache lookup — returns null if not yet resolved.
 * Used when we need a best-effort comparison without waiting for I/O.
 */
function realpathCacheGet(p: string): string | null {
  return realpathCache.get(p) ?? null;
}
