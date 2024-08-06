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

async function checkSingleProject(root: Uri): Promise<boolean> {
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

export async function projectUsesInsta(roots: Uri[]): Promise<boolean> {
  const results = await Promise.all(roots.map(checkSingleProject));
  return results.some((x) => x);
}

export async function findCargoRoots(): Promise<Uri[]> {
  // we search for the lockfile to only include workspace roots
  const uris = await workspace.findFiles('**/Cargo.lock');
  const roots = uris.map((uri) => Uri.joinPath(uri, '..'));
  console.log('found cargo roots', roots);
  return roots;
}
