import { workspace, ExtensionContext, window } from 'vscode'

import {
  Executable,
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from 'vscode-languageclient/node'

let client: LanguageClient

export async function activate(context: ExtensionContext) {
  const traceOutputChannel = window.createOutputChannel(
    'KCL Language Server trace'
  )
  // Eventually make this work for tcp.
  /*const socket: SocketTransport = {
    kind: TransportKind.socket,
    port: 6000,
  }*/
  let command = process.env.SERVER_PATH || 'kcl-language-server'
  const run: Executable = {
    command,
    args: ['--json', 'server'],
    transport: TransportKind.stdio,
    options: {
      env: {
        ...process.env,
        // eslint-disable-next-line @typescript-eslint/naming-convention
        RUST_LOG: 'debug',
      },
    },
  }
  const serverOptions: ServerOptions = {
    run,
    debug: run,
  }
  // If the extension is launched in debug mode then the debug server options are used
  // Otherwise the run options are used
  // Options to control the language client
  let clientOptions: LanguageClientOptions = {
    // Register the server for plain text documents
    documentSelector: [{ scheme: 'file', language: 'kcl' }],
    synchronize: {
      // Notify the server about file changes to '.clientrc files contained in the workspace
      fileEvents: workspace.createFileSystemWatcher('**/.clientrc'),
    },
    traceOutputChannel,
  }

  // Create the language client and start the client.
  client = new LanguageClient(
    'kcl-language-server',
    'kcl language server',
    serverOptions,
    clientOptions
  )
  client.start()
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined
  }
  return client.stop()
}
