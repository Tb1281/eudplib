import * as vscode from "vscode";
import {
  CloseAction,
  ErrorAction,
  LanguageClient,
  State,
  TransportKind,
  type ErrorHandler,
  type LanguageClientOptions,
  type ServerOptions,
  type StateChangeEvent,
} from "vscode-languageclient/node";
import {
  bundledServerPath,
  resolveServer,
  type SearchAttempt,
  type ServerSource,
} from "./serverPath";
import { checkServerVersion, downloadServerBinary } from "./serverDownload";

const CLIENT_ID = "eudkit-lsp-client";
const CLIENT_NAME = "eudkit LSP";
const OUTPUT_CHANNEL_NAME = "eudkit LSP";
const CONFIGURATION_SECTION = "eudkit";
const SERVER_PATH_SETTING = "lsp.path";
const RESTART_COMMAND = "eudkit.restartLanguageServer";

class EudkitLanguageServer implements vscode.Disposable {
  private readonly output: vscode.OutputChannel;
  private client: LanguageClient | undefined;
  private runningClient: LanguageClient | undefined;
  private intentionalStop = false;
  private disposed = false;
  private queue: Promise<void> = Promise.resolve();

  constructor(
    private readonly extensionRoot: string,
    private readonly storagePath: string,
    private readonly extensionVersion: string,
  ) {
    this.output = vscode.window.createOutputChannel(OUTPUT_CHANNEL_NAME);
  }

  start(): Promise<void> {
    return this.enqueue(() => this.startNow());
  }

  restart(reason: string): Promise<void> {
    return this.enqueue(async () => {
      this.log(`Restarting language server (${reason}).`);
      await this.stopNow();
      await this.startNow();
    });
  }

  dispose(): void {
    this.disposed = true;
    const client = this.client;
    this.client = undefined;
    this.runningClient = undefined;
    this.intentionalStop = true;

    if (client) {
      void client
        .stop()
        .catch((error: unknown) => {
          this.log(`Could not stop language server during shutdown: ${formatError(error)}`);
        })
        .finally(() => {
          client.dispose();
          this.output.dispose();
        });
    } else {
      this.output.dispose();
    }
  }

  private enqueue(operation: () => Promise<void>): Promise<void> {
    const next = this.queue.then(operation, operation);
    this.queue = next.catch((error: unknown) => {
      this.log(`Language server operation failed: ${formatError(error)}`);
      void this.promptRestart(
        `eudkit LSP operation failed: ${formatError(error)}`,
      );
    });
    return this.queue;
  }

  private async startNow(): Promise<void> {
    if (this.disposed || this.client) {
      return;
    }

    const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    const resolution = resolveServer({
      configuredPath: vscode.workspace
        .getConfiguration(CONFIGURATION_SECTION)
        .get<string>(SERVER_PATH_SETTING, ""),
      environmentPath: process.env.EUDKIT_LSP_PATH,
      workspaceRoot,
      extensionRoot: this.extensionRoot,
      platform: process.platform,
      arch: process.arch,
      pathValue: process.env.PATH,
      pathExtensions: process.env.PATHEXT,
    });

    let serverPath: string;
    let serverSource: ServerSource;

    if (resolution.found) {
      const version = await checkServerVersion(
        resolution.path,
        this.extensionVersion,
      );
      if (this.disposed || this.client) {
        return;
      }
      if (!version.ok) {
        this.reportVersionFailure(resolution.path, version.error);
        return;
      }
      this.log(
        `Found eudkit-lsp ${version.version} from ${describeSource(resolution.source)}: ${resolution.path}`,
      );
      serverPath = resolution.path;
      serverSource = resolution.source;
    } else {
      this.logSearched(resolution.searched);
      this.log(
        `Downloading the eudkit-lsp ${this.extensionVersion} release asset for ${process.platform}/${process.arch}.`,
      );
      const download = await downloadServerBinary({
        extensionVersion: this.extensionVersion,
        storagePath: this.storagePath,
        platform: process.platform,
        arch: process.arch,
        log: (line) => this.log(line),
      });
      if (this.disposed || this.client) {
        return;
      }
      if (!download.ok) {
        this.reportNotFound(download.error);
        return;
      }
      this.log(
        `Using eudkit-lsp ${download.version} from ${
          download.source === "cache" ? "the verified download cache" : "GitHub Releases"
        }: ${download.path}`,
      );
      serverPath = download.path;
      serverSource = "download";
    }

    this.log(
      `Starting eudkit-lsp from ${describeSource(serverSource)}: ${serverPath}`,
    );

    const serverOptions: ServerOptions = {
      command: serverPath,
      args: [],
      transport: TransportKind.stdio,
      options: {
        cwd: workspaceRoot ?? this.extensionRoot,
      },
    };

    const clientOptions: LanguageClientOptions = {
      documentSelector: [
        { language: "python" },
        { language: "typescript" },
        { language: "typescriptreact" },
        { language: "javascript" },
        { language: "javascriptreact" },
      ],
      outputChannel: this.output,
      errorHandler: this.createErrorHandler(),
      // Intentionally no handleDiagnostics middleware: eudkit's Map Symbol/EUD
      // warnings must reach the Problems panel. This client only wires the
      // shared server and never publishes or synthesizes type diagnostics;
      // Pylance and tsserver remain separate clients.
    };

    const client = new LanguageClient(
      CLIENT_ID,
      CLIENT_NAME,
      serverOptions,
      clientOptions,
    );
    this.client = client;
    client.onDidChangeState((event) => this.onStateChange(client, event));

    try {
      await client.start();
      if (this.client !== client) {
        return;
      }
      this.runningClient = client;
      this.log("Language server started.");
    } catch (error) {
      if (this.client === client) {
        this.client = undefined;
        this.runningClient = undefined;
        this.intentionalStop = true;
        try {
          await client.stop();
        } catch {
          // The process may already be gone after a failed start.
        }
        client.dispose();
        this.intentionalStop = false;
      }
      this.log(
        `Failed to start eudkit-lsp at ${serverPath}: ${formatError(error)}`,
      );
      void this.promptStartFailure(serverPath, error);
    }
  }

  private async stopNow(): Promise<void> {
    const client = this.client;
    if (!client) {
      return;
    }

    this.client = undefined;
    this.runningClient = undefined;
    this.intentionalStop = true;
    this.log("Stopping language server.");

    try {
      await client.stop(2000);
    } catch (error) {
      this.log(`Could not stop language server: ${formatError(error)}`);
    }
    client.dispose();
    this.intentionalStop = false;
    this.log("Language server stopped.");
  }

  private createErrorHandler(): ErrorHandler {
    return {
      error: (error, message, count) => {
        const occurrence = count === undefined ? "" : ` (occurrence ${count})`;
        this.log(
          `Language server error${occurrence}: ${error.message}${
            message === undefined ? "" : `\n${JSON.stringify(message)}`
          }`,
        );
        return { action: ErrorAction.Shutdown };
      },
      closed: () => {
        this.log("Language server connection closed.");
        return { action: CloseAction.DoNotRestart };
      },
    };
  }

  private onStateChange(
    client: LanguageClient,
    event: StateChangeEvent,
  ): void {
    this.log(`Language server state: ${describeState(event.newState)}.`);

    if (
      event.newState !== State.Stopped ||
      this.intentionalStop ||
      client !== this.runningClient
    ) {
      return;
    }

    this.client = undefined;
    this.runningClient = undefined;
    try {
      client.dispose();
    } catch (error) {
      this.log(`Could not dispose crashed language server: ${formatError(error)}`);
    }

    this.log("Language server stopped unexpectedly.");
    void this.promptRestart(
      "eudkit LSP stopped unexpectedly. Check the 'eudkit LSP' output channel for details.",
    );
  }

  private logSearched(searched: readonly SearchAttempt[]): void {
    this.log("Could not find eudkit-lsp. Searched:");
    for (const attempt of searched) {
      this.log(
        `  ${describeSource(attempt.source)}: ${attempt.candidate}${
          attempt.reason === undefined ? "" : ` (${attempt.reason})`
        }`,
      );
    }
  }

  private reportNotFound(downloadError: string): void {
    const bundled = bundledServerPath(
      this.extensionRoot,
      process.platform,
      process.arch,
    );

    this.log(`Release download failed: ${downloadError}`);

    const message =
      `Could not find or download eudkit-lsp for ${process.platform}/${process.arch}. ` +
      `${downloadError} ` +
      `Set "eudkit.lsp.path" (absolute or workspace-relative), set EUDKIT_LSP_PATH, ` +
      `add eudkit-lsp to PATH, or bundle it at ${bundled}. ` +
      `See the '${OUTPUT_CHANNEL_NAME}' output channel for every source, URL, and error that was tried.`;

    void vscode.window
      .showErrorMessage(message, "Open Settings", "Show Output", "Restart")
      .then((selection) => this.handlePrompt(selection, "retry after resolution failure"));
  }

  private reportVersionFailure(serverPath: string, error: string): void {
    this.log(`Rejected eudkit-lsp at ${serverPath}: ${error}`);
    const message =
      `${error} The installed eudkit-vscode extension is ${this.extensionVersion}. ` +
      `Set "eudkit.lsp.path" to a compatible eudkit-lsp or reinstall the matching release assets. ` +
      `See the '${OUTPUT_CHANNEL_NAME}' output channel for details.`;

    void vscode.window
      .showErrorMessage(message, "Open Settings", "Show Output", "Restart")
      .then((selection) => this.handlePrompt(selection, "retry after version mismatch"));
  }

  private async promptStartFailure(
    serverPath: string,
    error: unknown,
  ): Promise<void> {
    const selection = await vscode.window.showErrorMessage(
      `eudkit LSP failed to start from ${serverPath}: ${formatError(error)}`,
      "Show Output",
      "Open Settings",
      "Restart",
    );
    await this.handlePrompt(selection, "retry after start failure");
  }

  private async promptRestart(message: string): Promise<void> {
    const selection = await vscode.window.showErrorMessage(
      message,
      "Restart",
      "Show Output",
    );
    await this.handlePrompt(selection, "crash recovery");
  }

  private async handlePrompt(
    selection: string | undefined,
    restartReason: string,
  ): Promise<void> {
    try {
      if (selection === "Restart") {
        await this.restart(restartReason);
      } else if (selection === "Open Settings") {
        await vscode.commands.executeCommand(
          "workbench.action.openSettings",
          `${CONFIGURATION_SECTION}.${SERVER_PATH_SETTING}`,
        );
      } else if (selection === "Show Output") {
        this.output.show(true);
      }
    } catch (error) {
      this.log(`Could not handle notification action: ${formatError(error)}`);
    }
  }

  private log(message: string): void {
    this.output.appendLine(`[${new Date().toISOString()}] ${message}`);
  }
}

function describeSource(source: ServerSource): string {
  switch (source) {
    case "setting":
      return `setting "${CONFIGURATION_SECTION}.${SERVER_PATH_SETTING}"`;
    case "environment":
      return "EUDKIT_LSP_PATH";
    case "path":
      return "PATH";
    case "bundled":
      return "bundled binary";
    case "download":
      return "GitHub release download";
  }
}

function describeState(state: State): string {
  switch (state) {
    case State.Stopped:
      return "stopped";
    case State.Starting:
      return "starting";
    case State.Running:
      return "running";
  }
}

function formatError(error: unknown): string {
  if (error instanceof Error) {
    return error.stack ?? error.message;
  }
  return String(error);
}

export function activate(context: vscode.ExtensionContext): void {
  const server = new EudkitLanguageServer(
    context.extensionUri.fsPath,
    context.globalStorageUri.fsPath,
    String(context.extension.packageJSON.version),
  );

  context.subscriptions.push(
    server,
    vscode.commands.registerCommand(RESTART_COMMAND, () =>
      server.restart("command"),
    ),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (
        event.affectsConfiguration(
          `${CONFIGURATION_SECTION}.${SERVER_PATH_SETTING}`,
        )
      ) {
        void server.restart(
          `setting "${CONFIGURATION_SECTION}.${SERVER_PATH_SETTING}" changed`,
        );
      }
    }),
  );

  void server.start();
}

export function deactivate(): void {
  // The language server and output channel are disposed through
  // context.subscriptions.
}
