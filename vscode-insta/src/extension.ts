import { platform } from "os";
import {
  ExtensionContext,
  languages,
  workspace,
  Uri,
  commands,
  window,
  FileSystemError,
  DocumentFilter,
} from "vscode";
import { projectUsesInsta } from "./cargo";
import { InlineSnapshotProvider } from "./InlineSnapshotProvider";
import { processAllSnapshots, processInlineSnapshot } from "./insta";
import { PendingSnapshotsProvider } from "./PendingSnapshotsProvider";
import { Snapshot } from "./Snapshot";
import { SnapshotPathProvider } from "./SnapshotPathProvider";

const INSTA_CONTEXT_NAME = "inInstaSnapshotsProject";
const RUST_FILTER: DocumentFilter = {
  scheme: "file",
  language: "rust",
};

function getSnapshotPairs(uri: Uri): [Uri, Uri] | undefined {
  if (uri.path.match(/\.snap$/)) {
    return [uri, Uri.parse(`${uri}.new`)];
  } else if (uri.path.match(/\.snap\.new$/)) {
    return [uri.with({ path: uri.path.substr(0, uri.path.length - 4) }), uri];
  }
}

async function openNamedSnapshotDiff(selectedSnapshot?: Uri) {
  if (!selectedSnapshot) {
    selectedSnapshot = window.activeTextEditor?.document.uri;
  }
  if (!selectedSnapshot) {
    window.showErrorMessage("No snapshot selected");
    return;
  }
  const pair = getSnapshotPairs(selectedSnapshot);
  if (!pair) {
    window.showErrorMessage("Not an insta snapshot file");
    return;
  }

  let [oldSnapshot, newSnapshot]: [Uri, Uri] = pair;
  try {
    await workspace.fs.stat(oldSnapshot);
  } catch (e) {
    // todo: windows
    oldSnapshot = Uri.file(platform() == "win32" ? "NUL" : "/dev/null");
  }

  await commands.executeCommand(
    "vscode.diff",
    oldSnapshot,
    newSnapshot,
    "Snapshot Diff",
    {
      preview: true,
    }
  );
}

async function openInlineSnapshotDiff(snapshot: Snapshot) {
  const key = encodeURIComponent(snapshot.key);
  await commands.executeCommand(
    "vscode.diff",
    Uri.parse(`instaInlineSnapshot:inline.snap#${key}`),
    Uri.parse(`instaInlineSnapshot:inline.snap.new#${key}`),
    "Inline Snapshot Diff",
    {
      preview: true,
    }
  );
}

async function performSnapshotAction(
  action: "accept" | "reject",
  pendingSnapshotsProvider: PendingSnapshotsProvider,
  selectedSnapshot?: Uri
) {
  // in most cases when we're invoked we don't have a selected snapshot yet.
  // in that cas we always go by the active text editor's document.  However in
  // case that document is not a snapshot file (because for instance it's the
  // empty file we open for completely new snapshots), then we look at all other
  // visible text editors for the first snapshot.
  if (!selectedSnapshot) {
    selectedSnapshot = window.activeTextEditor?.document.uri;
    if (selectedSnapshot && !selectedSnapshot.path.match(/\.snap(\.new)?$/)) {
      window.visibleTextEditors.forEach((editor) => {
        if (editor.document.uri.path.match(/\.snap(\.new)?$/)) {
          selectedSnapshot = editor.document.uri;
        }
      });
    }
  }

  if (!selectedSnapshot) {
    window.showErrorMessage(`Cannot ${action} snapshot: no snapshot selected`);
    return;
  }

  // inline snapshots need to be handled through cargo-insta due to the
  // patching.  special case it here.
  if (selectedSnapshot.scheme === "instaInlineSnapshot") {
    const snapshot = pendingSnapshotsProvider.getInlineSnapshot(
      selectedSnapshot
    );
    if (!snapshot || !(await processInlineSnapshot(snapshot, action))) {
      window.showErrorMessage(`Cannot ${action} snapshot: cargo-insta failed`);
    } else {
      const currentActiveUri = window.activeTextEditor?.document.uri;
      if (currentActiveUri && selectedSnapshot.path.match(/\.snap(\.new)?$/)) {
        commands.executeCommand("workbench.action.closeActiveEditor");
      }
    }
    return;
  }

  const pair = getSnapshotPairs(selectedSnapshot);
  if (!pair) {
    window.showErrorMessage(`Cannot ${action} snapshot: not an insta snapshot`);
    return;
  }

  if (action === "accept") {
    try {
      await workspace.fs.stat(pair[1]);
    } catch (error) {
      window.showErrorMessage("Could not accept snapshot: no new snapshot");
      return;
    }
    await workspace.fs.rename(pair[1], pair[0], { overwrite: true });
    window.showInformationMessage("New snapshot accepted");
  } else if (action === "reject") {
    try {
      await workspace.fs.delete(pair[1]);
    } catch (error) {
      if (error instanceof FileSystemError && error.code === "FileNotFound") {
        window.showInformationMessage("No new snapshot to reject");
      } else {
        throw error;
      }
      return;
    }
    window.showInformationMessage("New snapshot rejected");
  }
}

async function switchSnapshotView(selectedSnapshot?: Uri): Promise<void> {
  if (!selectedSnapshot) {
    selectedSnapshot = window.activeTextEditor?.document.uri;
  }
  if (!selectedSnapshot) {
    return;
  }

  const pair = getSnapshotPairs(selectedSnapshot);
  if (!pair) {
    window.showErrorMessage("Not an insta snapshot file");
    return;
  }

  const otherFile = pair[0].path == selectedSnapshot.path ? pair[1] : pair[0];
  try {
    await workspace.fs.stat(otherFile);
  } catch (e) {
    window.showInformationMessage("Alternative snapshot does not exist.");
    return;
  }
  await commands.executeCommand("vscode.open", otherFile);
}

async function setInstaContext(value: boolean): Promise<void> {
  await commands.executeCommand("setContext", INSTA_CONTEXT_NAME, value);
}

function checkInstaContext() {
  const rootUri = workspace.workspaceFolders?.[0].uri;
  if (rootUri) {
    projectUsesInsta(rootUri).then((usesInsta) => setInstaContext(usesInsta));
  } else {
    setInstaContext(false);
  }
}

function performOnAllSnapshots(op: "accept" | "reject") {
  const root = workspace.workspaceFolders?.[0];
  if (!root) {
    return;
  }
  processAllSnapshots(root.uri, op).then((okay) => {
    if (okay) {
      window.showInformationMessage(`Successfully ${op}ed all snapshots.`);
    } else {
      window.showErrorMessage(`Could not ${op} snapshots.`);
    }
  });
}

export function activate(context: ExtensionContext): void {
  const root = workspace.workspaceFolders?.[0];
  const pendingSnapshots = new PendingSnapshotsProvider(root);
  const snapshotPathProvider = new SnapshotPathProvider();

  const snapWatcher = workspace.createFileSystemWatcher(
    "**/*.{snap,snap.new,pending-snap}"
  );
  snapWatcher.onDidChange(() => pendingSnapshots.refreshDebounced());
  snapWatcher.onDidCreate(() => pendingSnapshots.refreshDebounced());
  snapWatcher.onDidDelete(() => pendingSnapshots.refreshDebounced());

  const cargoTomlWatcher = workspace.createFileSystemWatcher("**/Cargo.toml");
  cargoTomlWatcher.onDidChange(() => checkInstaContext());
  cargoTomlWatcher.onDidCreate(() => checkInstaContext());
  cargoTomlWatcher.onDidDelete(() => checkInstaContext());

  if (root) {
    projectUsesInsta(root.uri).then((usesInsta) => setInstaContext(usesInsta));
  }

  context.subscriptions.push(
    snapWatcher,
    cargoTomlWatcher,
    window.registerTreeDataProvider("pendingInstaSnapshots", pendingSnapshots),
    workspace.registerTextDocumentContentProvider(
      "instaInlineSnapshot",
      new InlineSnapshotProvider(pendingSnapshots)
    ),
    languages.registerDefinitionProvider([RUST_FILTER], snapshotPathProvider),
    commands.registerCommand(
      "mitsuhiko.insta.open-snapshot-diff",
      async (selectedFile?: Uri | Snapshot) => {
        // when we're invoked from the pending snapshots view the first
        // argument is the node (Snapshot) instead of the URI.
        if (selectedFile instanceof Snapshot) {
          if (selectedFile.inlineInfo) {
            await openInlineSnapshotDiff(selectedFile);
            return;
          } else {
            selectedFile = selectedFile.resourceUri;
          }
        }
        await openNamedSnapshotDiff(selectedFile);
      }
    ),
    commands.registerCommand(
      "mitsuhiko.insta.accept-snapshot",
      (selectedFile?: Uri) =>
        performSnapshotAction("accept", pendingSnapshots, selectedFile)
    ),
    commands.registerCommand(
      "mitsuhiko.insta.reject-snapshot",
      (selectedFile?: Uri) =>
        performSnapshotAction("reject", pendingSnapshots, selectedFile)
    ),
    commands.registerCommand(
      "mitsuhiko.insta.switch-snapshot-view",
      (selectedFile?: Uri) => switchSnapshotView(selectedFile)
    ),
    commands.registerCommand("mitsuhiko.insta.refresh-pending-snapshots", () =>
      pendingSnapshots.refresh()
    ),
    commands.registerCommand("mitsuhiko.insta.accept-all-snapshots", () =>
      performOnAllSnapshots("accept")
    ),
    commands.registerCommand("mitsuhiko.insta.reject-all-snapshots", () =>
      performOnAllSnapshots("reject")
    )
  );
}
