import {
  CancellationToken,
  ProviderResult,
  workspace,
  Uri,
  TextDocumentContentProvider,
} from "vscode";
import { Snapshot } from "./Snapshot";

export class InlineSnapshotProvider implements TextDocumentContentProvider {
  constructor(private inlineSnapshots: { [key: string]: Snapshot }) {}
  provideTextDocumentContent(
    uri: Uri,
    token: CancellationToken
  ): ProviderResult<string> {
    const snapshot = this.inlineSnapshots[uri.fragment];
    if (!snapshot) {
      throw new Error("Snapshot not found");
    }
    const inlineInfo = snapshot.inlineInfo!;
    const contents =
      inlineInfo[uri.path == "inline.snap" ? "oldSnapshot" : "newSnapshot"];
    return `---\nsource: ${workspace.asRelativePath(
      snapshot.resourceUri!
    )}\nexpression: ${JSON.stringify(inlineInfo.expression)}\n---\n${contents}`;
  }
}
