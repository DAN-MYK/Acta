import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const e2eDir = fileURLToPath(new URL(".", import.meta.url));
const repoRoot = path.resolve(e2eDir, "..");
const binaryName = process.platform === "win32" ? "acta-tauri.exe" : "acta-tauri";
const applicationPath = path.resolve(repoRoot, "src-tauri", "target", "debug", binaryName);
const tauriDriverName = process.platform === "win32" ? "tauri-driver.exe" : "tauri-driver";
const tauriDriverPath = resolveExecutable(tauriDriverName);

let tauriDriver;
let expectedShutdown = false;

export const config = {
  host: "127.0.0.1",
  port: 4444,
  specs: ["./test/specs/**/*.e2e.js"],
  maxInstances: 1,
  capabilities: [
    {
      maxInstances: 1,
      "tauri:options": {
        application: applicationPath
      }
    }
  ],
  reporters: ["spec"],
  framework: "mocha",
  mochaOpts: {
    ui: "bdd",
    timeout: 90000
  },
  onPrepare: () => {
    const result = spawnSync(
      "npm",
      ["run", "tauri", "build", "--", "--debug", "--no-bundle"],
      {
        cwd: repoRoot,
        stdio: "inherit",
        shell: true
      }
    );

    if (result.status !== 0) {
      throw new Error(`Tauri debug build failed with exit code ${result.status}`);
    }
  },
  beforeSession: () => {
    if (!tauriDriverPath) {
      throw new Error(
        "tauri-driver was not found. Install it with `cargo install tauri-driver --locked` before running e2e tests."
      );
    }

    tauriDriver = spawn(tauriDriverPath, [], {
      stdio: [null, process.stdout, process.stderr]
    });

    tauriDriver.on("error", (error) => {
      console.error("tauri-driver error:", error);
      process.exit(1);
    });

    tauriDriver.on("exit", (code) => {
      if (!expectedShutdown) {
        console.error("tauri-driver exited with code:", code);
        process.exit(1);
      }
    });
  },
  afterSession: () => {
    closeTauriDriver();
  }
};

function closeTauriDriver() {
  expectedShutdown = true;
  tauriDriver?.kill();
}

function cleanupAndExit(signal) {
  closeTauriDriver();
  process.exit(signal === "SIGINT" ? 130 : 0);
}

process.on("exit", closeTauriDriver);
process.on("SIGINT", () => cleanupAndExit("SIGINT"));
process.on("SIGTERM", () => cleanupAndExit("SIGTERM"));
process.on("SIGHUP", () => cleanupAndExit("SIGHUP"));
process.on("SIGBREAK", () => cleanupAndExit("SIGBREAK"));

function resolveExecutable(name) {
  const candidates = [
    path.resolve(os.homedir(), ".cargo", "bin", name),
    ...(process.env.PATH ?? "").split(path.delimiter).map((entry) => path.resolve(entry, name))
  ];

  return candidates.find((candidate) => fs.existsSync(candidate));
}
