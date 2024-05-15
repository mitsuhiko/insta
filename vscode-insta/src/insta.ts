import * as cp from "child_process";
import { Uri } from "vscode";
import { Snapshot } from "./Snapshot";

async function getPendingSnapshotsFor(results: Snapshot[], root: Uri): Promise<void> {
  return new Promise((resolve, reject) => {
    let buffer = "";
    const child = cp.spawn(
      "cargo",
      ["insta", "pending-snapshots", "--as-json"],
      {
        cwd: root.fsPath,
      }
    );
    if (!child) {
      reject(new Error("could not spawn cargo-insta"));
      return;
    }
    child.stdout?.setEncoding("utf8");
    child.stdout.on("data", (data) => (buffer += data));
    child.on("close", (_exitCode) => {
      for (const line of buffer.split(/\n/g)) {
        try {
          results.push(new Snapshot(root, JSON.parse(line)))
        } catch (e) {
          console.error(e);
        }
      }
      resolve();
    });
  });
}

export async function getPendingSnapshots(roots: Uri[]): Promise<Snapshot[]> {
  const results: Snapshot[] = [];
  await Promise.all(roots.map((root) => getPendingSnapshotsFor(results, root)));
  return results;
}

export function processInlineSnapshot(
  snapshot: Snapshot,
  op: "accept" | "reject"
): Promise<boolean> {
  if (!snapshot.inlineInfo) {
    return Promise.resolve(false);
  }
  return new Promise((resolve, reject) => {
    const child = cp.spawn("cargo", ["insta", op, "--snapshot", snapshot.key], {
      cwd: snapshot.rootUri.fsPath,
    });
    if (!child) {
      reject(new Error("could not spawn cargo-insta"));
      return;
    }
    child.on("close", (exitCode) => {
      resolve(exitCode === 0);
    });
  });
}

export async function processAllSnapshots(
  roots: Uri[],
  op: "accept" | "reject"
): Promise<boolean> {
  const results = await Promise.all(roots.map((rootUri) => new Promise<boolean>((resolve, reject) => {
    const child = cp.spawn("cargo", ["insta", op], {
      cwd: rootUri.fsPath,
    });
    if (!child) {
      reject(new Error("could not spawn cargo-insta"));
      return;
    }
    child.on("close", (exitCode) => resolve(exitCode === 0));
  })));
  return results.every((x) => x);
}
