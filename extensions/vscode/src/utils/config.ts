import * as vscode from "vscode";

export function getConfig() {
  const config = vscode.workspace.getConfiguration("prism");
  return {
    network: config.get<string>("network", "testnet"),
    binaryPath: config.get<string>("binaryPath", "prism"),
  };
}
