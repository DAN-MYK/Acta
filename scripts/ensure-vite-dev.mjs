import { spawn } from "node:child_process";
import http from "node:http";
import https from "node:https";
import { createRequire } from "node:module";

const devUrl = process.env.TAURI_DEV_URL ?? "http://localhost:1420";
const expectedTitle = "<title>Acta</title>";
const expectedViteClient = "/@vite/client";
const viteCliPath = createRequire(import.meta.url).resolve("vite/bin/vite.js");

function fetchText(url) {
  const client = url.startsWith("https:") ? https : http;

  return new Promise((resolve, reject) => {
    const request = client.get(
      url,
      {
        timeout: 3000
      },
      (response) => {
        const chunks = [];

        response.on("data", (chunk) => chunks.push(chunk));
        response.on("end", () => {
          const body = Buffer.concat(chunks).toString("utf8");
          resolve({
            ok:
              response.statusCode !== undefined &&
              response.statusCode >= 200 &&
              response.statusCode < 300,
            body
          });
        });
      }
    );

    request.on("timeout", () => {
      request.destroy(new Error("timeout"));
    });
    request.on("error", reject);
  });
}

async function hasRunningActaDevServer() {
  try {
    const response = await fetchText(devUrl);
    return (
      response.ok &&
      response.body.includes(expectedTitle) &&
      response.body.includes(expectedViteClient)
    );
  } catch {
    return false;
  }
}

function runViteDev() {
  // На Windows під Node 25 запуск `.cmd` через `spawn()` може падати з EINVAL,
  // тому стартуємо локальний Vite CLI напряму через поточний Node runtime.
  const child = spawn(
    process.execPath,
    [viteCliPath, "--host", "0.0.0.0", "--port", "1420"],
    {
      stdio: "inherit",
      shell: false
    }
  );

  child.on("exit", (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
      return;
    }

    process.exit(code ?? 0);
  });

  child.on("error", (error) => {
    console.error("Не вдалося запустити Vite dev server:", error);
    process.exit(1);
  });
}

if (await hasRunningActaDevServer()) {
  console.log(`Acta dev server already running at ${devUrl}, reusing it.`);
  process.exit(0);
}

runViteDev();
