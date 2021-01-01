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
  Uri,
} from "vscode";

const NAMED_SNAPSHOT_ASSERTION: RegExp = /(?:\binsta::)?(?:assert(?:_\w+)_snapshot!)\(\s*['"]([^'"]+)['"]/;
const UNNAMED_SNAPSHOT_ASSERTION: RegExp = /(?:\binsta::)?(?:assert(?:_\w+)_snapshot!)\(/;
const FUNCTION: RegExp = /\bfn\s+([\w]+)\s*\(/;
const TEST_DECL: RegExp = /#\[test\]/;
const FILENAME_PARTITION: RegExp = /^(.*)[/\\](.*?)\.rs$/;
const SNAPSHOT_FUNCTION_STRIP: RegExp = /^test_(.*?)$/;

const RUST_SELECTOR: DocumentFilter = {
  scheme: "file",
  language: "rust",
};

class SnapshotPathProvider implements DefinitionProvider {
  public selector = [RUST_SELECTOR];

  /**
   * This looks up an explicitly named snapshot (simple case)
   */
  private resolveNamedSnapshot(
    document: TextDocument,
    position: Position
  ): Uri | null {
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
    const moduleName = fileNameMatch[2];
    const snapshotFile = `${path}/snapshots/${moduleName}__${snapshotName}.snap`;
    return Uri.file(snapshotFile);
  }

  /**
   * This locates an implicitly (unnamed) snapshot.
   */
  private resolveUnnamedSnapshot(
    document: TextDocument,
    position: Position
  ): Uri | null {
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
      console.log(document.lineAt(scanLine).text.match(FUNCTION));
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

    const snapshotName = functionName.match(SNAPSHOT_FUNCTION_STRIP)![1];
    const fileNameMatch = document.fileName.match(FILENAME_PARTITION);
    if (!fileNameMatch) {
      return null;
    }

    const path = fileNameMatch[1];
    const moduleName = fileNameMatch[2];
    const snapshotFile = `${path}/snapshots/${moduleName}__${snapshotName}${
      snapshotNumber > 1 ? `-${snapshotNumber}` : ""
    }.snap`;
    return Uri.file(snapshotFile);
  }

  public provideDefinition(
    document: TextDocument,
    position: Position,
    _token: CancellationToken
  ): ProviderResult<Definition> {
    const snapshot =
      this.resolveNamedSnapshot(document, position) ||
      this.resolveUnnamedSnapshot(document, position);
    return snapshot ? new Location(snapshot, new Position(0, 0)) : null;
  }
}

export function activate(context: ExtensionContext): void {
  const definitions = new SnapshotPathProvider();
  context.subscriptions.push(
    languages.registerDefinitionProvider(definitions.selector, definitions)
  );
}
