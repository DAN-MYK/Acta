import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { needsTauriBuild } from "../tauri-build-coordinator.js";

const tempRoots: string[] = [];

function createTempDir() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "acta-wdio-build-"));
  tempRoots.push(dir);
  return dir;
}

function writeFile(targetPath: string, contents: string, mtimeMs: number) {
  fs.mkdirSync(path.dirname(targetPath), { recursive: true });
  fs.writeFileSync(targetPath, contents);
  const timestamp = new Date(mtimeMs);
  fs.utimesSync(targetPath, timestamp, timestamp);
}

describe("tauri build coordinator", () => {
  afterEach(() => {
    for (const root of tempRoots.splice(0)) {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  it("requires a build when the application binary is missing", () => {
    const root = createTempDir();
    const appPath = path.join(root, "acta-tauri.exe");
    const dependencyPath = path.join(root, "frontend", "src", "App.svelte");

    writeFile(dependencyPath, "<div />", 20_000);

    expect(
      needsTauriBuild({
        applicationPath: appPath,
        dependencyPaths: [dependencyPath]
      })
    ).toBe(true);
  });

  it("skips the build when the binary is newer than all watched inputs", () => {
    const root = createTempDir();
    const appPath = path.join(root, "acta-tauri.exe");
    const frontendPath = path.join(root, "frontend", "src", "App.svelte");
    const backendPath = path.join(root, "src-tauri", "src", "main.rs");

    writeFile(frontendPath, "<div />", 20_000);
    writeFile(backendPath, "fn main() {}", 30_000);
    writeFile(appPath, "binary", 50_000);

    expect(
      needsTauriBuild({
        applicationPath: appPath,
        dependencyPaths: [frontendPath, backendPath]
      })
    ).toBe(false);
  });

  it("requires a build when any watched dependency is newer than the binary", () => {
    const root = createTempDir();
    const appPath = path.join(root, "acta-tauri.exe");
    const dependencyDir = path.join(root, "src-tauri", "src");
    const dependencyPath = path.join(dependencyDir, "main.rs");

    writeFile(appPath, "binary", 20_000);
    writeFile(dependencyPath, "fn main() {}", 50_000);

    expect(
      needsTauriBuild({
        applicationPath: appPath,
        dependencyPaths: [dependencyDir]
      })
    ).toBe(true);
  });

  it("skips a duplicate build when the current dependency snapshot is already stamped", () => {
    const root = createTempDir();
    const appPath = path.join(root, "acta-tauri.exe");
    const dependencyPath = path.join(root, "frontend", "src", "App.svelte");
    const stampPath = path.join(root, "tmp", "wdio", "tauri-build-stamp.json");

    writeFile(appPath, "binary", 20_000);
    writeFile(dependencyPath, "<div />", 50_000);
    writeFile(
      stampPath,
      JSON.stringify({
        latestDependencyMtimeMs: 50_000,
        writtenAt: new Date(60_000).toISOString()
      }),
      60_000
    );

    expect(
      needsTauriBuild({
        applicationPath: appPath,
        dependencyPaths: [dependencyPath],
        stampPath
      })
    ).toBe(false);
  });
});
