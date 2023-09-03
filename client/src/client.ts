import * as lc from "vscode-languageclient/node";
import type * as vscode from "vscode";
import type * as ra from "./lsp_ext";
import type { Config } from "./config";

// Command URIs have about form of command:command-name?arguments, where
// arguments is a percent-encoded array of data we want to pass along to
// the command function. For "Show References" this is a list of all file
// URIs with locations of every reference, and it can get quite long.
//
// To work around it we use an intermediary linkToCommand command. When
// we render a command link, a reference to a command with all its arguments
// is stored in a map, and instead a linkToCommand link is rendered
// with the key to that map.
export const LINKED_COMMANDS = new Map<string, ra.CommandLink>();

// For now the map is cleaned up periodically (I've set it to every
// 10 minutes). In general case we'll probably need to introduce TTLs or
// flags to denote ephemeral links (like these in hover popups) and
// persistent links and clean those separately. But for now simply keeping
// the last few links in the map should be good enough. Likewise, we could
// add code to remove a target command from the map after the link is
// clicked, but assuming most links in hover sheets won't be clicked anyway
// this code won't change the overall memory use much.
setInterval(
  function cleanupOlderCommandLinks() {
    // keys are returned in insertion order, we'll keep a few
    // of recent keys available, and clean the rest
    const keys = [...LINKED_COMMANDS.keys()];
    const keysToRemove = keys.slice(0, keys.length - 10);
    for (const key of keysToRemove) {
      LINKED_COMMANDS.delete(key);
    }
  },
  10 * 60 * 1000,
);

export async function createClient(
  traceOutputChannel: vscode.OutputChannel,
  outputChannel: vscode.OutputChannel,
  initializationOptions: vscode.WorkspaceConfiguration,
  serverOptions: lc.ServerOptions,
  config: Config,
): Promise<lc.LanguageClient> {
  const clientOptions: lc.LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "kcl" }],
    initializationOptions,
    traceOutputChannel,
    outputChannel,
    middleware: {
      workspace: {
        // HACK: This is a workaround, when the client has been disposed, VSCode
        // continues to emit events to the client and the default one for this event
        // attempt to restart the client for no reason
        async didChangeWatchedFile(event, next) {
          if (client.isRunning()) {
            await next(event);
          }
        },
        async configuration(
          params: lc.ConfigurationParams,
          token: vscode.CancellationToken,
          next: lc.ConfigurationRequest.HandlerSignature,
        ) {
          const resp = await next(params, token);
          return resp;
        },
      },
    },
  };

  const client = new lc.LanguageClient(
    "kcl-language-server",
    "KittyCAD Language Server",
    serverOptions,
    clientOptions,
  );

  client.registerFeature(new ExperimentalFeatures());

  return client;
}

class ExperimentalFeatures implements lc.StaticFeature {
  getState(): lc.FeatureState {
    return { kind: "static" };
  }
  fillClientCapabilities(capabilities: lc.ClientCapabilities): void {
    capabilities.experimental = {
      snippetTextEdit: true,
      codeActionGroup: true,
      hoverActions: true,
      serverStatusNotification: true,
      colorDiagnosticOutput: true,
      openServerLogs: true,
      commands: {
        commands: ["editor.action.triggerParameterHints"],
      },
      ...capabilities.experimental,
    };
  }
  initialize(
    _capabilities: lc.ServerCapabilities,
    _documentSelector: lc.DocumentSelector | undefined,
  ): void {}
  dispose(): void {}
}
