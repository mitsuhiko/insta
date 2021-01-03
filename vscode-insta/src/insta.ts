import * as cp from "child_process";
import { Uri } from "vscode";
import { Snapshot } from "./Snapshot";

export function getPendingSnapshots(root: Uri): Promise<Snapshot[]> {
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
      const snapshots = buffer
        .split(/\n/g)
        .map((line) => {
          try {
            return new Snapshot(root, JSON.parse(line));
          } catch (e) {
            return null;
          }
        })
        .filter((x) => x !== null);
      resolve(snapshots as any);
    });
  });
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

export function processAllSnapshots(
  rootUri: Uri,
  op: "accept" | "reject"
): Promise<boolean> {
  return new Promise((resolve, reject) => {
    const child = cp.spawn("cargo", ["insta", op], {
      cwd: rootUri.fsPath,
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
