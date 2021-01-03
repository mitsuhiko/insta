import * as cp from "child_process";
import { Uri, workspace } from "vscode";

function metadataReferencesInsta(metadata: any): boolean {
  for (const pkg of metadata.packages) {
    if (pkg.name === "insta") {
      return true;
    }
    for (const dependency of pkg.dependencies) {
      if (dependency.name === "insta") {
        return true;
      }
    }
  }
  return false;
}

export async function projectUsesInsta(root: Uri): Promise<boolean> {
  const rootCargoToml = Uri.joinPath(root, "Cargo.toml");
  try {
    await workspace.fs.stat(rootCargoToml);
  } catch (e) {
    return false;
  }

  return new Promise((resolve, reject) => {
    let buffer = "";
    const child = cp.spawn("cargo", [
      "metadata",
      "--no-deps",
      "--format-version=1",
    ]);
    child.stdout?.setEncoding("utf8");
    child.stdout.on("data", (data) => (buffer += data));
    child.on("close", (exitCode) => {
      if (exitCode != 0) {
        return resolve(false);
      }
      try {
        resolve(metadataReferencesInsta(JSON.parse(buffer)));
      } catch (e) {
        reject(e);
      }
    });
  });
}
