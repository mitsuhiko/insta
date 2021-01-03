import {
  ProviderResult,
  TreeDataProvider,
  TreeItem,
  Event,
  WorkspaceFolder,
  EventEmitter,
  Uri,
} from "vscode";
import { getPendingSnapshots } from "./insta";
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

  getInlineSnapshot(uri: Uri): Snapshot | undefined {
    return (
      (uri.scheme === "instaInlineSnapshot" &&
        this.cachedInlineSnapshots[uri.fragment]) ||
      undefined
    );
  }

  getTreeItem(element: Snapshot): TreeItem | Thenable<TreeItem> {
    return element;
  }

  getChildren(element?: Snapshot): ProviderResult<Snapshot[]> {
    const { workspaceRoot } = this;
    if (element || !workspaceRoot) {
      return Promise.resolve([]);
    }

    return getPendingSnapshots(workspaceRoot.uri).then((snapshots) => {
      return snapshots.map((snapshot) => {
        if (snapshot.inlineInfo) {
          this.cachedInlineSnapshots[snapshot.key] = snapshot;
        }
        return snapshot;
      });
    });
  }
}
