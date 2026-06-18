#!/usr/bin/env node
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { createRequire } from "node:module";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, "..");
const productName = "Bookmarker";
const supportedPlatforms = new Set(["linux", "win32"]);
const defaultDesktopPlatform = "win32";
const defaultConfigUrl = "https://linkdrop.ahara.io/config.js";
const execFileAsync = promisify(execFile);

const requireFromScript = createRequire(import.meta.url);
const electronPackage = requireFromScript("electron/package.json");
const electronChecksums = requireFromScript("electron/checksums.json");

const paths = {
  dist: path.join(projectRoot, "dist"),
  electronMain: path.join(projectRoot, "dist-electron", "main.js"),
  release: path.join(projectRoot, "release"),
};

async function main() {
  const target = desktopTarget();
  console.log(`Packaging ${productName} for ${target.platform}-${target.arch}...`);
  assertSupportedPlatform(target.platform);
  await assertBuildInputs();

  const packageDirectory = packagePath(target);
  const appDirectory = path.join(packageDirectory, "resources", "app");
  console.log("Preparing release directory...");
  await fs.rm(packageDirectory, { recursive: true, force: true });
  await fs.mkdir(appDirectory, { recursive: true });

  console.log("Extracting Electron runtime...");
  await extractRuntime(packageDirectory, target);
  console.log("Copying app assets...");
  await fs.cp(paths.dist, path.join(appDirectory, "dist"), { recursive: true });
  await writeRuntimeConfig(path.join(appDirectory, "dist", "config.js"));
  await fs.cp(path.join(projectRoot, "dist-electron"), path.join(appDirectory, "dist-electron"), {
    recursive: true,
  });
  await fs.writeFile(path.join(appDirectory, "package.json"), appPackageJson(), "utf8");
  await renameExecutable(packageDirectory, target.platform);
  const archivePath = await zipBundle(packageDirectory);

  console.log(`Desktop package created: ${packageDirectory}`);
  console.log(`Executable: ${path.join(packageDirectory, executableName(target.platform))}`);
  console.log(`Archive: ${archivePath}`);
}

function desktopTarget() {
  return {
    platform: process.env.BOOKMARKER_DESKTOP_PLATFORM?.trim() || defaultDesktopPlatform,
    arch: process.env.BOOKMARKER_DESKTOP_ARCH?.trim() || process.arch,
  };
}

function assertSupportedPlatform(platform) {
  if (!supportedPlatforms.has(platform)) {
    throw new Error(`Desktop packaging is supported for Linux and Windows, not ${platform}.`);
  }
}

async function assertBuildInputs() {
  await assertFile(path.join(paths.dist, "index.html"), "Run `pnpm run build` first.");
  await assertFile(paths.electronMain, "Run `pnpm run desktop:build` first.");
}

async function assertFile(filePath, message) {
  try {
    const stat = await fs.stat(filePath);
    if (stat.isFile()) {
      return;
    }
  } catch {
    // Fall through to the shared error.
  }
  throw new Error(`${message} Missing: ${filePath}`);
}

async function extractRuntime(packageDirectory, target) {
  const zipPath = await electronZipPath(target);
  console.log(`Using Electron runtime: ${zipPath}`);
  const tempDirectory = await fs.mkdtemp(path.join(os.tmpdir(), "bookmarker-electron-"));
  try {
    console.log("Unpacking Electron runtime...");
    await unzip(zipPath, tempDirectory);
    console.log("Copying Electron runtime...");
    await fs.cp(tempDirectory, packageDirectory, { recursive: true });
  } finally {
    await fs.rm(tempDirectory, { recursive: true, force: true });
  }
}

async function unzip(zipPath, destination) {
  await execFileAsync("unzip", ["-q", zipPath, "-d", destination], {
    maxBuffer: 1024 * 1024,
  });
}

async function zipBundle(packageDirectory) {
  const archivePath = `${packageDirectory}.zip`;
  await fs.rm(archivePath, { force: true });
  await execFileAsync("zip", ["-qr", archivePath, path.basename(packageDirectory)], {
    cwd: paths.release,
    maxBuffer: 1024 * 1024,
  });
  return archivePath;
}

async function writeRuntimeConfig(configPath) {
  const localConfigPath = process.env.BOOKMARKER_DESKTOP_CONFIG_PATH?.trim();
  if (localConfigPath) {
    await fs.copyFile(localConfigPath, configPath);
    return;
  }
  const configUrl = process.env.BOOKMARKER_DESKTOP_CONFIG_URL?.trim() || defaultConfigUrl;
  const response = await fetch(configUrl);
  if (!response.ok) {
    throw new Error(`Failed to download runtime config: ${response.status} ${response.statusText}`);
  }
  await fs.writeFile(configPath, desktopRuntimeConfig(await response.text()), "utf8");
}

function desktopRuntimeConfig(configText) {
  return `${configText.trim()}\nwindow.__APP_CONFIG__.productName = ${JSON.stringify(productName)};\n`;
}

async function electronZipPath(target) {
  const filename = electronZipName(target);
  console.log(`Resolving ${filename}...`);
  const existing = await existingElectronZip(filename);
  if (existing) {
    console.log("Verifying cached Electron runtime...");
    await verifyElectronZip(existing, filename);
    return existing;
  }
  const targetPath = path.join(bookmarkerElectronCache(), filename);
  await fs.mkdir(path.dirname(targetPath), { recursive: true });
  await downloadElectronZip(filename, targetPath);
  await verifyElectronZip(targetPath, filename);
  return targetPath;
}

async function existingElectronZip(filename) {
  const cacheRoots = [
    process.env.BOOKMARKER_ELECTRON_CACHE,
    path.join(os.homedir(), ".cache", "electron"),
    path.join(os.homedir(), ".electron"),
    bookmarkerElectronCache(),
  ].filter(Boolean);
  for (const cacheRoot of cacheRoots) {
    const found = await findFile(cacheRoot, filename, 4);
    if (found) {
      return found;
    }
  }
  return undefined;
}

async function findFile(directory, filename, remainingDepth) {
  const entries = await readDirectory(directory);
  for (const entry of entries) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isFile() && entry.name === filename) {
      return entryPath;
    }
    if (entry.isDirectory() && remainingDepth > 0) {
      const found = await findFile(entryPath, filename, remainingDepth - 1);
      if (found) {
        return found;
      }
    }
  }
  return undefined;
}

async function readDirectory(directory) {
  try {
    return await fs.readdir(directory, { withFileTypes: true });
  } catch {
    return [];
  }
}

async function downloadElectronZip(filename, targetPath) {
  const response = await fetch(electronReleaseUrl(filename));
  if (!response.ok) {
    throw new Error(`Failed to download ${filename}: ${response.status} ${response.statusText}`);
  }
  const body = Buffer.from(await response.arrayBuffer());
  await fs.writeFile(targetPath, body);
}

async function verifyElectronZip(zipPath, filename) {
  const expected = electronChecksums[filename];
  if (!expected) {
    throw new Error(`Electron checksum is missing for ${filename}.`);
  }
  const actual = await sha256File(zipPath);
  if (actual !== expected) {
    throw new Error(`Electron checksum mismatch for ${filename}.`);
  }
}

async function sha256File(filePath) {
  const file = await fs.readFile(filePath);
  return createHash("sha256").update(file).digest("hex");
}

function bookmarkerElectronCache() {
  return path.join(os.homedir(), ".cache", "bookmarker-electron");
}

function electronReleaseUrl(filename) {
  return `https://github.com/electron/electron/releases/download/v${electronPackage.version}/${filename}`;
}

function electronZipName(target) {
  return `electron-v${electronPackage.version}-${target.platform}-${target.arch}.zip`;
}

function packagePath(target) {
  return path.join(paths.release, `bookmarker-${target.platform}-${target.arch}`);
}

function appPackageJson() {
  return `${JSON.stringify(packageMetadata(), null, 2)}\n`;
}

function packageMetadata() {
  return {
    name: "bookmarker-desktop",
    productName,
    version: "0.1.0",
    type: "module",
    main: "dist-electron/main.js",
  };
}

async function renameExecutable(packageDirectory, platform) {
  const sourcePath = path.join(packageDirectory, sourceExecutableName(platform));
  const targetPath = path.join(packageDirectory, executableName(platform));
  if (sourcePath !== targetPath) {
    await fs.rename(sourcePath, targetPath);
  }
}

function sourceExecutableName(platform) {
  return platform === "win32" ? "electron.exe" : "electron";
}

function executableName(platform) {
  return platform === "win32" ? `${productName}.exe` : productName;
}

try {
  await main();
} catch (error) {
  console.error(error instanceof Error ? error.message : error);
  process.exit(1);
}
