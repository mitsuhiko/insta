import { platform } from "os";
import {
  ExtensionContext,
  DefinitionProvider,
  DocumentFilter,
  languages,
  TextDocument,
  CancellationToken,
  Definition,
  Location,
  Position,
  ProviderResult,
  workspace,
  Uri,
  commands,
  window,
  FileSystemError,
} from "vscode";

const NAMED_SNAPSHOT_ASSERTION: RegExp = /(?:\binsta::)?(?:assert(?:_\w+)?_snapshot!)\(\s*['"]([^'"]+)['"]\s*,/;
const UNNAMED_SNAPSHOT_ASSERTION: RegExp = /(?:\binsta::)?(?:assert(?:_\w+)?_snapshot!)\(/;
const FUNCTION: RegExp = /\bfn\s+([\w]+)\s*\(/;
const TEST_DECL: RegExp = /#\[test\]/;
const FILENAME_PARTITION: RegExp = /^(.*)[/\\](.*?)\.rs$/;
const SNAPSHOT_FUNCTION_STRIP: RegExp = /^test_(.*?)$/;

const RUST_SELECTOR: DocumentFilter = {
  scheme: "file",
  language: "rust",
};

type SnapshotMatch = {
  snapshotName: string;
  path: string;
  localModuleName: string;
};

class SnapshotPathProvider implements DefinitionProvider {
  /**
   * This looks up an explicitly named snapshot (simple case)
   */
  private resolveNamedSnapshot(
    document: TextDocument,
    position: Position
  ): SnapshotMatch | null {
    const line =
      (position.line >= 1 ? document.lineAt(position.line - 1).text : "") +
      document.lineAt(position.line).text;

    const snapshotMatch = line.match(NAMED_SNAPSHOT_ASSERTION);
    if (!snapshotMatch) {
      return null;
    }
    const snapshotName = snapshotMatch[1];
    const fileNameMatch = document.fileName.match(FILENAME_PARTITION);
    if (!fileNameMatch) {
      return null;
    }
    const path = fileNameMatch[1];
    const localModuleName = fileNameMatch[2];
    return { snapshotName, path, localModuleName };
  }

  /**
   * This locates an implicitly (unnamed) snapshot.
   */
  private resolveUnnamedSnapshot(
    document: TextDocument,
    position: Position
  ): SnapshotMatch | null {
    function unnamedSnapshotAt(lineno: number): boolean {
      const line = document.lineAt(lineno).text;
      return !!(
        line.match(UNNAMED_SNAPSHOT_ASSERTION) &&
        !line.match(NAMED_SNAPSHOT_ASSERTION)
      );
    }

    // if we can't find an unnnamed snapshot at the given position we bail.
    if (!unnamedSnapshotAt(position.line)) {
      return null;
    }

    // otherwise scan backwards for unnamed snapshot matches until we find
    // a test function declaration.
    let snapshotNumber = 1;
    let scanLine = position.line - 1;
    let functionName = null;

    while (scanLine >= 0) {
      // stop if we find a test function declaration
      let functionMatch;
      if (
        scanLine > 1 &&
        (functionMatch = document.lineAt(scanLine).text.match(FUNCTION)) &&
        document.lineAt(scanLine - 1).text.match(TEST_DECL)
      ) {
        functionName = functionMatch[1];
        break;
      }
      if (unnamedSnapshotAt(scanLine)) {
        snapshotNumber++;
      }
      scanLine--;
    }

    // if we couldn't find a function we have to bail.
    if (!functionName) {
      return null;
    }

    const snapshotName = `${functionName.match(SNAPSHOT_FUNCTION_STRIP)![1]}${
      snapshotNumber > 1 ? `-${snapshotNumber}` : ""
    }`;
    const fileNameMatch = document.fileName.match(FILENAME_PARTITION);
    if (!fileNameMatch) {
      return null;
    }

    const path = fileNameMatch[1];
    const localModuleName = fileNameMatch[2];
    return { snapshotName, path, localModuleName };
  }

  public provideDefinition(
    document: TextDocument,
    position: Position,
    token: CancellationToken
  ): ProviderResult<Definition> {
    const snapshotMatch =
      this.resolveNamedSnapshot(document, position) ||
      this.resolveUnnamedSnapshot(document, position);
    if (!snapshotMatch) {
      return null;
    }

    const getSearchPath = function (
      mode: "exact" | "wildcard-prefix" | "wildcard-all"
    ): string {
      return workspace.asRelativePath(
        `${snapshotMatch.path}/snapshots/${mode !== "exact" ? "*__" : ""}${
          snapshotMatch.localModuleName
        }${mode === "wildcard-all" ? "__*" : ""}__${
          snapshotMatch.snapshotName
        }.snap`
      );
    };

    function findFiles(path: string): Thenable<Uri | null> {
      return workspace
        .findFiles(path, "", 1, token)
        .then((results) => results[0] || null);
    }

    // we try to find the file in three passes:
    // - exact matchin the snapshot folder.
    // - with a wildcard module prefix (crate__foo__NAME__SNAP)
    // - with a wildcard module prefix and suffix (crate__foo__NAME__tests__SNAP)
    // This is needed since snapshots can be contained in submodules. Since
    // getting the actual module name is tedious we just hope the match is
    // unique.
    return findFiles(getSearchPath("exact"))
      .then((rv) => rv || findFiles(getSearchPath("wildcard-prefix")))
      .then((rv) => rv || findFiles(getSearchPath("wildcard-all")))
      .then((snapshot) =>
        snapshot ? new Location(snapshot, new Position(0, 0)) : null
      );
  }
}

function getSnapshotPairs(uri: Uri): [Uri, Uri] | undefined {
  if (uri.path.match(/\.snap$/)) {
    return [uri, Uri.parse(`${uri}.new`)];
  } else if (uri.path.match(/\.snap\.new$/)) {
    return [uri.with({ path: uri.path.substr(0, uri.path.length - 4) }), uri];
  }
}

async function openSnapshotDiff(selectedSnapshot?: Uri) {
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

  commands
    .executeCommand("vscode.diff", oldSnapshot, newSnapshot, "Snapshot Diff", {
      preview: true,
    })
    .then((result) => {
      console.log(result);
    });
}

async function performSnapshotAction(
  action: "accept" | "reject",
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

export function activate(context: ExtensionContext): void {
  context.subscriptions.push(
    languages.registerDefinitionProvider(
      [RUST_SELECTOR],
      new SnapshotPathProvider()
    ),
    commands.registerCommand(
      "mitsuhiko.insta.open-snapshot-diff",
      (selectedFile?: Uri) => openSnapshotDiff(selectedFile)
    ),
    commands.registerCommand(
      "mitsuhiko.insta.accept-snapshot",
      (selectedFile?: Uri) => performSnapshotAction("accept", selectedFile)
    ),
    commands.registerCommand(
      "mitsuhiko.insta.reject-snapshot",
      (selectedFile?: Uri) => performSnapshotAction("reject", selectedFile)
    ),
    commands.registerCommand(
      "mitsuhiko.insta.switch-snapshot-view",
      (selectedFile?: Uri) => switchSnapshotView(selectedFile)
    )
  );
}
