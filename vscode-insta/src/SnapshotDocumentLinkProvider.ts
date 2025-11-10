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
};

export const SOURCE_COMMAND = "mitsuhiko.insta.open-source-location";

function extractSourceLink(document: TextDocument): SourceLink | undefined {
  let sourcePath: string | undefined;
  let lineNumber: number | undefined;
  let sourceLineIndex = -1;

  // Simple pass through all lines
  for (let i = 0; i < document.lineCount; i++) {
    const text = document.lineAt(i).text;
    
    // Find source path
    if (text.startsWith("source:")) {
      sourcePath = text.slice(7).trim(); // "source:" is 7 characters
      sourceLineIndex = i;
    }
    
    // Find assertion line (optional)
    if (text.startsWith("assertion_line:")) {
      const lineStr = text.slice(15).trim(); // "assertion_line:" is 15 characters
      lineNumber = parseInt(lineStr, 10);
    }
  }

  if (sourcePath && sourceLineIndex >= 0) {
    // Create a simple range for the entire source line
    const line = document.lineAt(sourceLineIndex);
    const range = new Range(
      new Position(sourceLineIndex, 0),
      new Position(sourceLineIndex, line.text.length)
    );
    
    return {
      sourcePath,
      range,
      line: lineNumber,
    };
  }

  return undefined;
}

async function resolveSourceUri(
  document: TextDocument,
  sourcePath: string
): Promise<Uri | undefined> {
  // Handle absolute paths
  if (path.isAbsolute(sourcePath)) {
    return Uri.file(sourcePath);
  }

  // For relative paths, resolve against workspace root
  // Insta snapshots are always workspace-relative
  const workspaceFolder = workspace.getWorkspaceFolder(document.uri);
  if (workspaceFolder) {
    return Uri.file(path.join(workspaceFolder.uri.fsPath, sourcePath));
  }

  return undefined;
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
      const position = new Position(zeroBasedLine, 0);
      editor.selection = new Selection(position, position);
      editor.revealRange(new Range(position, position));
    }
  } catch (error) {
    const message =
      error instanceof Error ? error.message : "Unknown error opening file";
    window.showErrorMessage(`Could not open source file: ${message}`);
  }
}
