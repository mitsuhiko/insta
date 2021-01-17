import {
  DefinitionProvider,
  TextDocument,
  CancellationToken,
  Definition,
  Location,
  Position,
  ProviderResult,
  workspace,
  Uri,
} from "vscode";

const NAMED_SNAPSHOT_ASSERTION: RegExp = /(?:\binsta::)?(?:assert(?:_\w+)?_snapshot!)\(\s*['"]([^'"]+)['"]\s*,/;
const STRING_INLINE_SNAPSHOT_ASSERTION: RegExp = /(?:\binsta::)?(?:assert(?:_\w+)?_snapshot!)\(\s*['"]([^'"]+)['"]\s*,\s*@(r#*)?["']/;
const UNNAMED_SNAPSHOT_ASSERTION: RegExp = /(?:\binsta::)?(?:assert(?:_\w+)?_snapshot!)\(/;
const INLINE_MARKER: RegExp = /@(r#*)?["']/;
const FUNCTION: RegExp = /\bfn\s+([\w]+)\s*\(/;
const TEST_DECL: RegExp = /#\[test\]/;
const FILENAME_PARTITION: RegExp = /^(.*)[/\\](.*?)\.rs$/;
const SNAPSHOT_FUNCTION_STRIP: RegExp = /^test_(.*?)$/;
const SNAPSHOT_HEADER: RegExp = /^---\s*$(.*?)^---\s*$/ms;

type SnapshotMatch = {
  snapshotName: string | null;
  line: number | null;
  path: string;
  localModuleName: string | null;
  snapshotType: "inline" | "named";
};

type ResolvedSnapshotMatch = SnapshotMatch & {
  snapshotUri: Uri;
};

export class SnapshotPathProvider implements DefinitionProvider {
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
    return {
      snapshotName,
      line: null,
      path,
      localModuleName,
      snapshotType: "named",
    };
  }

  /**
   * This locates an implicitly (unnamed) snapshot.
   */
  private resolveUnnamedSnapshot(
    document: TextDocument,
    position: Position,
    noInline: boolean
  ): SnapshotMatch | null {
    function unnamedSnapshotAt(lineno: number): boolean {
      const line = document.lineAt(lineno).text;
      return !!(
        line.match(UNNAMED_SNAPSHOT_ASSERTION) &&
        !line.match(NAMED_SNAPSHOT_ASSERTION) &&
        (noInline || !line.match(STRING_INLINE_SNAPSHOT_ASSERTION))
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
    let isInline = !!document.lineAt(position.line).text.match(INLINE_MARKER);
    console.log("inline", document.lineAt(position.line), isInline);

    while (scanLine >= 0) {
      // stop if we find a test function declaration
      let functionMatch;
      const line = document.lineAt(scanLine);
      if (
        scanLine > 1 &&
        (functionMatch = line.text.match(FUNCTION)) &&
        document.lineAt(scanLine - 1).text.match(TEST_DECL)
      ) {
        functionName = functionMatch[1];
        break;
      }
      if (!isInline && line.text.match(INLINE_MARKER)) {
        isInline = true;
      }
      if (unnamedSnapshotAt(scanLine)) {
        // TODO: do not increment if the snapshot at that location
        snapshotNumber++;
      }
      scanLine--;
    }

    // if we couldn't find a function or an unexpected inline snapshot we have to bail.
    if (!functionName || (noInline && isInline)) {
      return null;
    }

    let snapshotName = null;
    let line = null;
    let path = null;
    let localModuleName = null;

    if (isInline) {
      line = position.line;
      path = document.fileName;
    } else {
      snapshotName = `${functionName.match(SNAPSHOT_FUNCTION_STRIP)![1]}${
        snapshotNumber > 1 ? `-${snapshotNumber}` : ""
      }`;
      const fileNameMatch = document.fileName.match(FILENAME_PARTITION);
      if (!fileNameMatch) {
        return null;
      }
      path = fileNameMatch[1];
      localModuleName = fileNameMatch[2];
    }

    return {
      snapshotName,
      line,
      path,
      localModuleName,
      snapshotType: isInline ? "inline" : "named",
    };
  }

  public findSnapshotAtLocation(
    document: TextDocument,
    position: Position,
    token: CancellationToken,
    noInline: boolean = false
  ): Thenable<ResolvedSnapshotMatch | null> {
    const snapshotMatch =
      this.resolveNamedSnapshot(document, position) ||
      this.resolveUnnamedSnapshot(document, position, noInline);
    if (!snapshotMatch) {
      return Promise.resolve(null);
    }

    if (snapshotMatch.snapshotType === "inline") {
      return Promise.resolve({
        snapshotUri: document.uri,
        ...snapshotMatch,
      });
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
        snapshot ? { snapshotUri: snapshot, ...snapshotMatch } : null
      );
  }

  public provideDefinition(
    document: TextDocument,
    position: Position,
    token: CancellationToken
  ): ProviderResult<Definition> {
    return this.findSnapshotAtLocation(document, position, token, true).then(
      (match) => {
        if (!match) {
          return null;
        }
        return workspace.fs.readFile(match.snapshotUri).then((contents) => {
          const stringContents = Buffer.from(contents).toString("utf-8");
          const header = stringContents.match(SNAPSHOT_HEADER);
          let location = new Position(0, 0);
          if (header) {
            location = new Position(header[0].match(/\n/g)!.length + 1, 0);
          }
          return new Location(match.snapshotUri, location);
        });
      }
    );
  }
}
