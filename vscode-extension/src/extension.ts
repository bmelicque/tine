import * as path from "path";
import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;
let serverOptions!: ServerOptions;
let clientOptions!: LanguageClientOptions;

async function startServer() {
  if (client) {
    return;
  }

  client = new LanguageClient(
    "TineServer",
    "Tine Language Server",
    serverOptions,
    clientOptions,
  );

  await client.start();
}

async function stopServer() {
  if (!client) {
    return;
  }

  try {
    await client.stop();
  } finally {
    client = undefined;
  }
}

export async function restartServer() {
  await stopServer();
  await startServer();
}

export async function activate(context: vscode.ExtensionContext) {
  const serverBinary =
    process.platform === "win32" ? "tine_server.exe" : "tine_server";

  const serverPath = context.asAbsolutePath(
    path.join("server", "bin", serverBinary),
  );

  serverOptions = {
    run: { command: serverPath, transport: TransportKind.stdio },
    debug: { command: serverPath, transport: TransportKind.stdio },
  };

  clientOptions = {
    documentSelector: [{ scheme: "file", language: "tine" }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*.tine"),
    },
  };

  await startServer();

  context.subscriptions.push({ dispose: () => client?.stop() });
  context.subscriptions.push(
    vscode.commands.registerCommand("tine.restartServer", async () => {
      await restartServer();
      vscode.window.showInformationMessage("Tine language server restarted.");
    }),
  );
}

export async function deactivate(): Promise<void> {
  return stopServer();
}
