import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { argv, cwd, exit } from "node:process";
import { TextDecoder } from "node:util";

const roots = argv.slice(2);
const scanRoots = roots.length > 0 ? roots : ["frontend/src", "src", "tests", "e2e-tests/test"];
const extensions = new Set([".css", ".js", ".rs", ".svelte", ".ts"]);
const mojibakePatterns = [
  /(?:\u00d0|\u00d1|\u0432\u0402|\u0432\u045a|\u0432\u045c|\u0432\u201d){2,}/,
  /(?:[\u0420\u0421\u00d0\u00d1][\u0080-\u00ff]){3,}/,
  /(?:[\u0420\u0421][\u00b0-\u00b7\u0451\u0402-\u040f\u0452-\u045f\u2026]){2,}/,
  /\?{4,}/
];
const utf8Decoder = new TextDecoder("utf-8", { fatal: true });
const ignoreDirs = new Set([
  ".cache",
  ".git",
  ".playwright-mcp",
  ".worktrees",
  "dist",
  "node_modules",
  "target",
  "target_codex_test"
]);

async function collectFiles(root, files = []) {
  const { readdir, stat } = await import("node:fs/promises");
  const entries = await readdir(root, { withFileTypes: true });

  for (const entry of entries) {
    if (ignoreDirs.has(entry.name)) {
      continue;
    }

    const path = join(root, entry.name);
    if (entry.isDirectory()) {
      await collectFiles(path, files);
      continue;
    }

    if (!entry.isFile()) {
      continue;
    }

    const dotIndex = entry.name.lastIndexOf(".");
    const extension = dotIndex >= 0 ? entry.name.slice(dotIndex) : "";
    if (extensions.has(extension) && (await stat(path)).size > 0) {
      files.push(path);
    }
  }

  return files;
}

function lineLooksBroken(line) {
  return mojibakePatterns.some((pattern) => pattern.test(line));
}

const findings = [];

for (const root of scanRoots) {
  const files = await collectFiles(join(cwd(), root));
  for (const file of files) {
    const bytes = await readFile(file);
    if (bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf) {
      findings.push(`${file}:1: UTF-8 BOM detected`);
      continue;
    }

    let text;
    try {
      text = utf8Decoder.decode(bytes);
    } catch {
      findings.push(`${file}:1: invalid UTF-8 bytes`);
      continue;
    }

    const lines = text.split(/\r?\n/);

    lines.forEach((line, index) => {
      if (lineLooksBroken(line)) {
        findings.push(`${file}:${index + 1}: ${line.trim().slice(0, 160)}`);
      }
    });
  }
}

if (findings.length > 0) {
  console.error("Suspicious mojibake text was found:");
  for (const finding of findings.slice(0, 60)) {
    console.error(`- ${finding}`);
  }
  if (findings.length > 60) {
    console.error(`...and ${findings.length - 60} more`);
  }
  exit(1);
}
