import * as path from "path";
import * as vscode from "vscode";
import { LanguageClient, LanguageClientOptions, ServerOptions, TransportKind } from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export function activate(context: vscode.ExtensionContext) {
	// Adjust this path if needed
	const serverBinary = process.platform === "win32" ? "mylang_server.exe" : "mylang_server";

	const serverPath = context.asAbsolutePath(path.join("server", "bin", serverBinary));

	const serverOptions: ServerOptions = {
		run: { command: serverPath, transport: TransportKind.stdio },
		debug: { command: serverPath, transport: TransportKind.stdio },
	};

	const clientOptions: LanguageClientOptions = {
		documentSelector: [{ scheme: "file", language: "my-lang" }],
		synchronize: {
			fileEvents: vscode.workspace.createFileSystemWatcher("**/*.my"),
		},
	};

	client = new LanguageClient("myLangServer", "MyLang Language Server", serverOptions, clientOptions);
	client.start();

	context.subscriptions.push({ dispose: () => client?.stop() });
}

export function deactivate(): Thenable<void> | undefined {
	return client?.stop();
}
