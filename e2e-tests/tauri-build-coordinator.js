import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const BUILD_LOCK_TIMEOUT_MS = 300000;
const BUILD_LOCK_POLL_MS = 250;
const TRANSIENT_BUILD_RETRY_COUNT = 3;
const TRANSIENT_BUILD_RETRY_DELAY_MS = 1500;

export function latestDependencyMtimeMs(
  dependencyPaths,
  existsSync = fs.existsSync,
  statSync = fs.statSync,
  readdirSync = fs.readdirSync
) {
  return dependencyPaths.reduce((latest, dependencyPath) => {
    if (!existsSync(dependencyPath)) {
      return latest;
    }

    return Math.max(latest, collectLatestMtimeMs(dependencyPath, statSync, readdirSync));
  }, 0);
}

export function collectLatestMtimeMs(targetPath, statSync = fs.statSync, readdirSync = fs.readdirSync) {
  if (!fs.existsSync(targetPath)) {
    return 0;
  }

  const stats = statSync(targetPath);
  if (!stats.isDirectory()) {
    return stats.mtimeMs;
  }

  let latest = stats.mtimeMs;
  for (const entry of readdirSync(targetPath, { withFileTypes: true })) {
    latest = Math.max(
      latest,
      collectLatestMtimeMs(path.join(targetPath, entry.name), statSync, readdirSync)
    );
  }

  return latest;
}

export function needsTauriBuild({
  applicationPath,
  dependencyPaths,
  stampPath,
  existsSync = fs.existsSync,
  statSync = fs.statSync,
  readdirSync = fs.readdirSync
}) {
  if (!existsSync(applicationPath)) {
    return true;
  }

  const latestDependencyMtime = latestDependencyMtimeMs(dependencyPaths, existsSync, statSync, readdirSync);
  const buildStamp = readBuildStamp(stampPath);

  if (buildStamp && buildStamp.latestDependencyMtimeMs >= latestDependencyMtime) {
    return false;
  }

  const applicationMtime = statSync(applicationPath).mtimeMs;
  return latestDependencyMtime > applicationMtime;
}

export async function ensureTauriBuild({
  applicationPath,
  dependencyPaths,
  lockPath,
  stampPath,
  repoRoot,
  buildCommand = "npm",
  buildArgs = ["run", "tauri", "build", "--", "--debug", "--no-bundle"],
  runBuild = defaultRunBuild,
  delayFn = delay,
  retryCount = TRANSIENT_BUILD_RETRY_COUNT,
  retryDelayMs = TRANSIENT_BUILD_RETRY_DELAY_MS,
  platform = process.platform
}) {
  if (!needsTauriBuild({ applicationPath, dependencyPaths, stampPath })) {
    return { built: false, reason: "fresh" };
  }

  fs.mkdirSync(path.dirname(lockPath), { recursive: true });
  const lockHandle = await acquireLock(lockPath);

  if (!lockHandle) {
    await waitForUnlock(lockPath);

    if (needsTauriBuild({ applicationPath, dependencyPaths, stampPath })) {
      throw new Error("Tauri build lock was released, але артефакт так і не став актуальним.");
    }

    return { built: false, reason: "built-by-peer" };
  }

  try {
    if (!needsTauriBuild({ applicationPath, dependencyPaths, stampPath })) {
      return { built: false, reason: "fresh-after-lock" };
    }

    const dependencyMtime = latestDependencyMtimeMs(dependencyPaths);
    const buildResult = await runBuildWithRetry({
      applicationPath,
      repoRoot,
      buildCommand,
      buildArgs,
      runBuild,
      delayFn,
      retryCount,
      retryDelayMs,
      platform
    });

    if (buildResult.reason === "used-existing-binary-after-lock") {
      return { built: false, reason: buildResult.reason };
    }

    writeBuildStamp(stampPath, dependencyMtime);
    return { built: true, reason: "rebuilt" };
  } finally {
    releaseLock(lockHandle, lockPath);
  }
}

async function acquireLock(lockPath) {
  const startedAt = Date.now();

  while (Date.now() - startedAt < BUILD_LOCK_TIMEOUT_MS) {
    try {
      return fs.openSync(lockPath, "wx");
    } catch (error) {
      if (!isAlreadyExistsError(error)) {
        throw error;
      }
    }

    await delay(BUILD_LOCK_POLL_MS);
  }

  throw new Error(`Не вдалося дочекатися build lock: ${lockPath}`);
}

async function waitForUnlock(lockPath) {
  const startedAt = Date.now();

  while (Date.now() - startedAt < BUILD_LOCK_TIMEOUT_MS) {
    if (!fs.existsSync(lockPath)) {
      return;
    }

    await delay(BUILD_LOCK_POLL_MS);
  }

  throw new Error(`Build lock не звільнився вчасно: ${lockPath}`);
}

function releaseLock(lockHandle, lockPath) {
  fs.closeSync(lockHandle);

  if (fs.existsSync(lockPath)) {
    fs.unlinkSync(lockPath);
  }
}

function delay(timeoutMs) {
  return new Promise((resolve) => setTimeout(resolve, timeoutMs));
}

async function runBuildWithRetry({
  applicationPath,
  stampPath,
  dependencyPaths,
  repoRoot,
  buildCommand,
  buildArgs,
  runBuild,
  delayFn,
  retryCount,
  retryDelayMs,
  platform
}) {
  for (let attempt = 1; attempt <= retryCount; attempt += 1) {
    const result = runBuild(buildCommand, buildArgs, repoRoot);

    if (result.status === 0) {
      return { built: true, reason: "rebuilt" };
    }

    const buildError = createBuildError(result);
    const canRetry =
      platform === "win32" &&
      attempt < retryCount &&
      isTransientWindowsLockError(buildError.message) &&
      fs.existsSync(applicationPath);

    if (!canRetry) {
      if (platform === "win32" && isTransientWindowsLockError(buildError.message) && fs.existsSync(applicationPath)) {
        console.warn(
          "Tauri build лишився заблокованим після повторів. Продовжую smoke з наявним binary без оновлення build stamp."
        );
        return { built: false, reason: "used-existing-binary-after-lock" };
      }

      throw buildError;
    }

    console.warn(
      `Tauri build завершився transient lock-помилкою. Повторюю спробу ${attempt + 1}/${retryCount} через ${retryDelayMs}мс.`
    );
    await delayFn(retryDelayMs);
  }
}

function defaultRunBuild(buildCommand, buildArgs, repoRoot) {
  return spawnSync(buildCommand, buildArgs, {
    cwd: repoRoot,
    stdio: "inherit",
    encoding: "utf8",
    shell: true
  });
}

function createBuildError(result) {
  if (result.error) {
    return result.error;
  }

  const details = [result.stdout, result.stderr]
    .filter((value) => typeof value === "string" && value.trim().length > 0)
    .join("\n")
    .trim();

  const suffix = details ? `\n${details}` : "";
  return new Error(`Tauri debug build failed with exit code ${result.status}.${suffix}`);
}

function isTransientWindowsLockError(message) {
  const normalizedMessage = message.toLowerCase();
  return (
    normalizedMessage.includes("os error 32") ||
    normalizedMessage.includes("being used by another process") ||
    normalizedMessage.includes("cannot access the file because it is being used by another process")
  );
}

function isAlreadyExistsError(error) {
  return error instanceof Error && "code" in error && error.code === "EEXIST";
}

function readBuildStamp(stampPath) {
  if (!stampPath || !fs.existsSync(stampPath)) {
    return null;
  }

  try {
    return JSON.parse(fs.readFileSync(stampPath, "utf8"));
  } catch {
    return null;
  }
}

function writeBuildStamp(stampPath, latestDependencyMtimeMsValue) {
  if (!stampPath) {
    return;
  }

  fs.mkdirSync(path.dirname(stampPath), { recursive: true });
  fs.writeFileSync(
    stampPath,
    JSON.stringify(
      {
        latestDependencyMtimeMs: latestDependencyMtimeMsValue,
        writtenAt: new Date().toISOString()
      },
      null,
      2
    )
  );
}
