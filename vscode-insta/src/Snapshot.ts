import { workspace, Uri, TreeItem } from "vscode";

export type InlineSnapshotInfo = {
  oldSnapshot?: string;
  newSnapshot: string;
  line: number;
  expression?: string;
  name?: string;
};

export class Snapshot extends TreeItem {
  public key: string;
  public inlineInfo?: InlineSnapshotInfo;

  constructor(public rootUri: Uri, snapshotInfo: any) {
    super(Uri.file(snapshotInfo.path));
    const relPath = workspace.asRelativePath(snapshotInfo.path);
    const line = snapshotInfo.line;
    this.label = line !== undefined ? `${relPath}:${line}` : relPath;
    this.key =
      line !== undefined ? `${snapshotInfo.path}:${line}` : snapshotInfo.path;

    if (snapshotInfo.type === "inline_snapshot") {
      this.description = snapshotInfo.name || "(inline)";
      this.inlineInfo = {
        oldSnapshot:
          snapshotInfo.old_snapshot === null
            ? undefined
            : snapshotInfo.old_snapshot,
        newSnapshot: snapshotInfo.new_snapshot,
        line: snapshotInfo.line,
        expression:
          snapshotInfo.expression === null
            ? undefined
            : snapshotInfo.expression,
        name: snapshotInfo.name === null ? undefined : snapshotInfo.name,
      };
    }

    this.command = {
      command: "mitsuhiko.insta.open-snapshot-diff",
      title: "",
      arguments: [this],
    };
  }

  contextValue = "pendingInstaSnapshot";
}
