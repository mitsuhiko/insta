import {
  CancellationToken,
  ProviderResult,
  workspace,
  Uri,
  TextDocumentContentProvider,
} from "vscode";
import { PendingSnapshotsProvider } from "./PendingSnapshotsProvider";

export class InlineSnapshotProvider implements TextDocumentContentProvider {
  constructor(private pendingSnapshotsProvider: PendingSnapshotsProvider) {}
  provideTextDocumentContent(
    uri: Uri,
    token: CancellationToken
  ): ProviderResult<string> {
    const snapshot = this.pendingSnapshotsProvider.getInlineSnapshot(uri);
    if (!snapshot) {
      throw new Error("Snapshot not found");
    }
    const inlineInfo = snapshot.inlineInfo!;
    const contents =
      inlineInfo[uri.path == "inline.snap" ? "oldSnapshot" : "newSnapshot"];
    return `---\nsource: ${workspace.asRelativePath(snapshot.resourceUri!)}:${
      inlineInfo.line
    }\nexpression: ${JSON.stringify(inlineInfo.expression)}\nname: ${
      inlineInfo.name || "unknown"
    }\n---\n${contents}`;
  }
}
