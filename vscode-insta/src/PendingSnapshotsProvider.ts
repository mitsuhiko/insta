import {
  Event,
  EventEmitter,
  TreeDataProvider,
  TreeItem,
  Uri
} from "vscode";
import { Snapshot } from "./Snapshot";
import { findCargoRoots } from "./cargo";
import { getPendingSnapshots } from "./insta";

export class PendingSnapshotsProvider implements TreeDataProvider<Snapshot> {
  private _onDidChangeTreeData: EventEmitter<
    Snapshot | undefined | void
  > = new EventEmitter<Snapshot | undefined | void>();
  onDidChangeTreeData: Event<void | Snapshot | null | undefined> = this
    ._onDidChangeTreeData.event;
  public cachedInlineSnapshots: { [key: string]: Snapshot } = {};
  private pendingRefresh?: NodeJS.Timeout;

  constructor() {}

  refresh(): void {
    this._onDidChangeTreeData.fire();
  }

  refreshDebounced(): void {
    if (this.pendingRefresh !== undefined) {
      clearTimeout(this.pendingRefresh);
    }
    this.pendingRefresh = setTimeout(() => {
      this.pendingRefresh = undefined;
      this.refresh();
    }, 200);
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

  async getChildren(element?: Snapshot): Promise<Snapshot[]> {
    if (element) {
      return [];
    }
    const roots = await findCargoRoots();
    if (roots.length === 0) {
      return [];
    }

    const snapshots = await getPendingSnapshots(roots);
    return snapshots.map((snapshot) => {
      if (snapshot.inlineInfo) {
        this.cachedInlineSnapshots[snapshot.key] = snapshot;
      }
      return snapshot;
    });
  }
}
