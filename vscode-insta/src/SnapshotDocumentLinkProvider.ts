import * as path from "path";
import {
  DocumentLink,
  DocumentLinkProvider,
  Position,
  Range,
  Selection,
  TextDocument,
  Uri,
  window,
  workspace,
} from "vscode";

type SourceLink = {
  sourcePath: string;
  range: Range;
  line?: number;
  column?: number;
};

export const SOURCE_COMMAND = "mitsuhiko.insta.open-source-location";

function extractSourceLink(document: TextDocument): SourceLink | undefined {
  let pendingLineNumber: number | undefined;
  let pendingColumnNumber: number | undefined;

  for (let lineNumber = 0; lineNumber < document.lineCount; lineNumber++) {
    const line = document.lineAt(lineNumber);
    const text = line.text;

    if (text.trimStart().startsWith("source:")) {
      const colonIndex = text.indexOf(":");
      if (colonIndex === -1) {
        continue;
      }
      const afterColon = text.slice(colonIndex + 1);
      const trimmedTail = afterColon.trim();
      if (!trimmedTail) {
        continue;
      }

      let startChar =
        colonIndex + 1 + (afterColon.length - afterColon.trimStart().length);
      let endChar = startChar + trimmedTail.length;

      let sourcePath = trimmedTail;
      const hasQuotes =
        sourcePath.startsWith("\"") && sourcePath.endsWith("\"");
      if (hasQuotes && sourcePath.length >= 2) {
        sourcePath = sourcePath.slice(1, -1);
        startChar += 1;
        endChar -= 1;
      }

      const trailingLocation = sourcePath.match(/:(\d+)(?::(\d+))?$/);
      let lineHint = pendingLineNumber;
      let columnHint = pendingColumnNumber;
      if (trailingLocation) {
        lineHint = parseInt(trailingLocation[1], 10);
        if (trailingLocation[2]) {
          columnHint = parseInt(trailingLocation[2], 10);
        }
        sourcePath = sourcePath.slice(
          0,
          sourcePath.length - trailingLocation[0].length
        );
      }

      sourcePath = sourcePath.trim();
      if (!sourcePath) {
        return undefined;
      }

      return {
        sourcePath,
        range: new Range(
          new Position(lineNumber, startChar),
          new Position(lineNumber, endChar)
        ),
        line: lineHint,
        column: columnHint,
      };
    }

    if (pendingLineNumber === undefined) {
      const lineMatch = text.match(/^(?:assertion_)?line:\s*(\d+)/);
      if (lineMatch) {
        pendingLineNumber = parseInt(lineMatch[1], 10);
        continue;
      }
    }

    if (pendingColumnNumber === undefined) {
      const columnMatch = text.match(/^column:\s*(\d+)/);
      if (columnMatch) {
        pendingColumnNumber = parseInt(columnMatch[1], 10);
      }
    }
  }

  return undefined;
}

async function resolveSourceUri(
  document: TextDocument,
  sourcePath: string
): Promise<Uri | undefined> {
  if (sourcePath.startsWith("file://")) {
    return Uri.parse(sourcePath);
  }

  const normalized = sourcePath.replace(/\\/g, path.sep);
  if (path.isAbsolute(normalized)) {
    return Uri.file(normalized);
  }

  const baseCandidates = new Set<string>();
  const folder = workspace.getWorkspaceFolder(document.uri);
  if (folder) {
    baseCandidates.add(folder.uri.fsPath);
  }

  workspace.workspaceFolders?.forEach((f) => baseCandidates.add(f.uri.fsPath));

  if (document.uri.scheme === "file") {
    baseCandidates.add(path.dirname(document.uri.fsPath));
  }

  const attempted: Uri[] = [];
  for (const base of baseCandidates) {
    try {
      const candidate = Uri.file(path.resolve(base, normalized));
      attempted.push(candidate);
      await workspace.fs.stat(candidate);
      return candidate;
    } catch {
      // try next fallback
    }
  }

  return attempted[0];
}

export class SnapshotDocumentLinkProvider implements DocumentLinkProvider {
  async provideDocumentLinks(
    document: TextDocument
  ): Promise<DocumentLink[]> {
    const info = extractSourceLink(document);
    if (!info) {
      return [];
    }

    const target = await resolveSourceUri(document, info.sourcePath);
    if (!target) {
      return [];
    }

    const payload = encodeURIComponent(
      JSON.stringify({
        target: target.toString(true),
        line: info.line,
        column: info.column,
      })
    );

    const link = new DocumentLink(
      info.range,
      Uri.parse(`command:${SOURCE_COMMAND}?${payload}`)
    );
    link.tooltip = "Open snapshot source";
    return [link];
  }
}

export async function openSourceDocument(payload?: {
  target?: string;
  line?: number;
  column?: number;
}) {
  if (!payload?.target) {
    return;
  }

  const uri = Uri.parse(payload.target);
  try {
    const document = await workspace.openTextDocument(uri);
    const editor = await window.showTextDocument(document, { preview: true });
    if (payload.line !== undefined) {
      const zeroBasedLine = Math.max(payload.line - 1, 0);
      const zeroBasedColumn = Math.max((payload.column ?? 1) - 1, 0);
      const position = new Position(zeroBasedLine, zeroBasedColumn);
      editor.selection = new Selection(position, position);
      editor.revealRange(new Range(position, position));
    }
  } catch (error) {
    const message =
      error instanceof Error ? error.message : "Unknown error opening file";
    window.showErrorMessage(`Could not open source file: ${message}`);
  }
}
