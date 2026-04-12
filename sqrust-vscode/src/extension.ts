import * as vscode from "vscode";
import * as child_process from "child_process";
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

export function activate(context: vscode.ExtensionContext): void {
  diagnosticCollection = vscode.languages.createDiagnosticCollection("sqrust");
  outputChannel = vscode.window.createOutputChannel("SQRust");
  context.subscriptions.push(diagnosticCollection, outputChannel);

  // Lint on open.
  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument((doc) => {
      if (isEnabled() && lintOnOpen() && isSql(doc)) {
        lintDocument(doc);
      }
    })
  );

  // Lint on save.
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument((doc) => {
      if (isEnabled() && lintOnSave() && isSql(doc)) {
        lintDocument(doc);
      }
    })
  );

  // Clear diagnostics when a document is closed.
  context.subscriptions.push(
    vscode.workspace.onDidCloseTextDocument((doc) => {
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
        lintDocument(doc);
      }
    });
  }
}

export function deactivate(): void {
  diagnosticCollection?.dispose();
  outputChannel?.dispose();
}

// ─── Core lint logic ────────────────────────────────────────────────────────

function lintDocument(doc: vscode.TextDocument): void {
  const filePath = doc.uri.fsPath;
  const args = buildArgs(filePath);
  const executable = getExecutablePath();

  runSqrust(executable, args, filePath, (violations, error) => {
    if (error) {
      if (isNotFound(error)) {
        vscode.window.showErrorMessage(
          `SQRust: '${executable}' not found. Install with: brew install nafistiham/tap/sqrust`
        );
      } else {
        outputChannel.appendLine(`[error] ${error.message}`);
      }
      // Keep existing diagnostics rather than silently clearing them.
      return;
    }
    const diagnostics = violations
      .filter((v) => resolvePath(v.file) === resolvePath(filePath))
      .map(toDiagnostic);
    diagnosticCollection.set(doc.uri, diagnostics);
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

  // Clear all diagnostics before a workspace-wide re-lint so stale entries
  // from fixed files do not persist.
  diagnosticCollection.clear();

  runSqrust(executable, args, workspacePath, (violations, error) => {
    if (error) {
      if (isNotFound(error)) {
        vscode.window.showErrorMessage(
          `SQRust: '${executable}' not found. Install with: brew install nafistiham/tap/sqrust`
        );
      } else {
        outputChannel.appendLine(`[error] ${error.message}`);
      }
      return;
    }
    // Group violations by file and update each file's diagnostics.
    const byFile = new Map<string, SqrustViolation[]>();
    for (const v of violations) {
      const key = resolvePath(v.file);
      if (!byFile.has(key)) {
        byFile.set(key, []);
      }
      byFile.get(key)!.push(v);
    }
    for (const [filePath, fileViolations] of byFile) {
      const uri = vscode.Uri.file(filePath);
      diagnosticCollection.set(uri, fileViolations.map(toDiagnostic));
    }
  });
}

// ─── Process execution ─────────────────────────────────────────────────────

function runSqrust(
  executable: string,
  args: string[],
  targetPath: string,
  callback: (violations: SqrustViolation[], error: Error | null) => void
): void {
  let stdout = "";
  let stderr = "";

  // Use the target file's directory (or workspace root) as cwd so that
  // sqrust can discover sqrust.toml by walking up from the target.
  const cwd =
    vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ??
    path.dirname(targetPath);

  const proc = child_process.spawn(executable, args, { cwd });

  proc.stdout.on("data", (chunk: Buffer) => {
    stdout += chunk.toString();
  });

  proc.stderr.on("data", (chunk: Buffer) => {
    stderr += chunk.toString();
  });

  proc.on("error", (err: Error) => {
    callback([], err);
  });

  proc.on("close", (code: number | null) => {
    if (code === null) {
      callback([], new Error("sqrust was terminated by a signal"));
      return;
    }
    // Exit code 2 = tool-level error (e.g. unknown dialect, config parse error).
    if (code === 2) {
      callback([], new Error(`sqrust exited with code 2: ${stderr.trim().slice(0, 200)}`));
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
        new Error(`sqrust output was not a JSON array: ${stdout.slice(0, 200)}`)
      );
      return;
    }
    callback(parsed as SqrustViolation[], null);
  });
}

// ─── Conversion helpers ─────────────────────────────────────────────────────

function toDiagnostic(v: SqrustViolation): vscode.Diagnostic {
  // sqrust reports 1-indexed lines and columns; VS Code uses 0-indexed.
  const line = Math.max(0, v.line - 1);
  const col = Math.max(0, v.col - 1);
  const range = new vscode.Range(line, col, line, col + 1);
  const severity =
    v.severity === "error"
      ? vscode.DiagnosticSeverity.Error
      : vscode.DiagnosticSeverity.Warning;
  const diag = new vscode.Diagnostic(range, `[${v.rule}] ${v.message}`, severity);
  diag.source = "sqrust";
  diag.code = v.rule;
  return diag;
}

// ─── Config helpers ─────────────────────────────────────────────────────────

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

function buildArgs(targetPath: string): string[] {
  const args = ["check", "--format", "json"];
  const dialect = getDialect();
  if (dialect) {
    args.push("--dialect", dialect);
  }
  args.push(targetPath);
  return args;
}

// ─── Utility ────────────────────────────────────────────────────────────────

function isSql(doc: vscode.TextDocument): boolean {
  return (
    doc.languageId === "sql" ||
    doc.uri.fsPath.toLowerCase().endsWith(".sql")
  );
}

function isNotFound(err: Error): boolean {
  return (err as NodeJS.ErrnoException).code === "ENOENT";
}

/** Resolve symlinks so that /var/... and /private/var/... compare equal. */
function resolvePath(p: string): string {
  try {
    // Use require here to avoid adding a top-level import that complicates
    // the module graph in tests. fs is always available in Node/VS Code host.
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    return require("fs").realpathSync(p) as string;
  } catch {
    // File may not exist yet (e.g. unsaved buffer) — fall back to normalize.
    return path.normalize(p);
  }
}
