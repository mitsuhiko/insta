import * as cp from "child_process";
import {
  ProviderResult,
  TreeDataProvider,
  TreeItem,
  Event,
  WorkspaceFolder,
  EventEmitter,
} from "vscode";
import { Snapshot } from "./Snapshot";

export class PendingSnapshotsProvider implements TreeDataProvider<Snapshot> {
  private _onDidChangeTreeData: EventEmitter<
    Snapshot | undefined | void
  > = new EventEmitter<Snapshot | undefined | void>();
  onDidChangeTreeData?:
    | Event<void | Snapshot | null | undefined>
    | undefined = this._onDidChangeTreeData.event;
  public cachedInlineSnapshots: { [key: string]: Snapshot } = {};

  constructor(private workspaceRoot?: WorkspaceFolder) {}

  refresh(): void {
    this._onDidChangeTreeData.fire();
  }

  getTreeItem(element: Snapshot): TreeItem | Thenable<TreeItem> {
    return element;
  }

  getChildren(element?: Snapshot): ProviderResult<Snapshot[]> {
    const { workspaceRoot } = this;
    if (element || !workspaceRoot) {
      return Promise.resolve([]);
    }

    return new Promise((resolve, reject) => {
      let buffer = "";
      const child = cp.spawn(
        "cargo",
        ["insta", "pending-snapshots", "--as-json"],
        {
          cwd: workspaceRoot.uri.fsPath,
        }
      );
      if (!child) {
        reject(new Error("could not spawn cargo-insta"));
        return;
      }
      child.stdout?.setEncoding("utf8");
      child.stdout.on("data", (data) => (buffer += data));
      child.on("close", (_exitCode) => {
        const snapshots = buffer
          .split(/\n/g)
          .map((line) => {
            try {
              const snapshot = new Snapshot(JSON.parse(line));
              if (snapshot.inlineInfo) {
                this.cachedInlineSnapshots[snapshot.key] = snapshot;
              }
              return snapshot;
            } catch (e) {
              return null;
            }
          })
          .filter((x) => x !== null);
        resolve(snapshots as any);
      });
    });
  }
}
