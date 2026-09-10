// Language-server acquisition for the sokonanoda VS Code extension.
//
// Kept free of the `vscode` module so plain Node can unit-test it
// (`test-server.js`). Acquisition order (docs/design-bundled-lsp.md §3.2):
//
//   1. `sokonanoda.serverPath` setting (explicit override)
//   2. `SOKONANODA_LSP_BIN` environment variable
//   3. bundled `bin/<target>/sokonanoda-lsp[.exe]` shipped in the VSIX
//   4. workspace `target/{debug,release}` build (development)
//   5. version-marked download cache (legacy)
//   6. version-pinned GitHub Release download (universal fallback VSIX)
//
// The bundled binary is the primary path: platform-specific VSIXes carry the
// exact server built from the same tag as the extension, so no network call
// and no client/server version skew. The download path is kept for the
// universal fallback VSIX (unsupported platforms) and is pinned to
// `v${extensionVersion}` — never the mutable latest alias.

const fs = require("fs");
const http = require("http");
const https = require("https");
const path = require("path");
const { execSync } = require("child_process");

const RELEASES_BASE = "https://github.com/ColorlessBoy/sokonanoda-lang/releases";

/// Alpine/musl detection: VS Code selects the `alpine-*` package by checking
/// `/etc/alpine-release` (src/vs/base/common/platform.ts), and the runtime
/// must map to the matching musl binary the same way.
function isAlpineLinux(platform, fsImpl = fs) {
  if (platform !== "linux") return false;
  return fsImpl.existsSync("/etc/alpine-release");
}

/// VS Code target platform (`vsce --target`) for a Node platform/arch pair.
/// `undefined` = no bundled binary and no downloadable build for this platform.
function platformTarget(platform, arch, alpine = false) {
  if (platform === "darwin") {
    if (arch === "arm64") return "darwin-arm64";
    if (arch === "x64") return "darwin-x64";
    return undefined;
  }
  if (platform === "linux") {
    if (alpine) {
      if (arch === "x64") return "alpine-x64";
      if (arch === "arm64") return "alpine-arm64";
      return undefined;
    }
    if (arch === "x64") return "linux-x64";
    if (arch === "arm64") return "linux-arm64";
    return undefined;
  }
  if (platform === "win32") {
    if (arch === "x64") return "win32-x64";
    if (arch === "arm64") return "win32-arm64";
    return undefined;
  }
  return undefined;
}

/// Rust target triple used in GitHub Release asset names. Mirrors
/// `platformTarget` (unsupported pairs have no asset either).
function rustTarget(platform, arch, alpine = false) {
  if (!platformTarget(platform, arch, alpine)) return undefined;
  if (platform === "darwin") {
    return arch === "arm64" ? "aarch64-apple-darwin" : "x86_64-apple-darwin";
  }
  if (platform === "linux") {
    if (alpine) {
      return arch === "x64" ? "x86_64-unknown-linux-musl" : "aarch64-unknown-linux-musl";
    }
    return arch === "x64" ? "x86_64-unknown-linux-gnu" : "aarch64-unknown-linux-gnu";
  }
  return arch === "x64" ? "x86_64-pc-windows-msvc" : "aarch64-pc-windows-msvc";
}

function binaryName(platform) {
  return platform === "win32" ? "sokonanoda-lsp.exe" : "sokonanoda-lsp";
}

/// Path of a bundled binary in this extension (`bin/<target>/<baseName>[.exe]`).
function bundledBinaryPath(extensionPath, platform, arch, baseName, alpine = false) {
  const target = platformTarget(platform, arch, alpine);
  if (!target) return undefined;
  return path.join(
    extensionPath,
    "bin",
    target,
    `${baseName}${platform === "win32" ? ".exe" : ""}`,
  );
}

/// Path of the server bundled in this extension, for the current platform.
function bundledServerPath(extensionPath, platform, arch, alpine = false) {
  return bundledBinaryPath(extensionPath, platform, arch, "sokonanoda-lsp", alpine);
}

function firstExisting(candidates, fsImpl = fs) {
  for (const candidate of candidates) {
    if (fsImpl.existsSync(candidate)) return candidate;
  }
  return undefined;
}

function builtBinaryCandidates(roots, name) {
  return roots.flatMap((root) => [
    path.join(root, "target", "debug", name),
    path.join(root, "target", "release", name),
  ]);
}

function isExecutable(file, fsImpl = fs) {
  try {
    fsImpl.accessSync(file, fs.constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

/// Resolve the bundled server, repairing a lost executable bit (VSIXes built
/// on Windows lose POSIX modes). Returns `undefined` when this platform has no
/// bundled target, the file is missing, or it stays non-executable (e.g. a
/// read-only extension directory) — callers then fall through to the next
/// acquisition path.
function resolveBundledBinary(options) {
  const { extensionPath, platform, arch, baseName = "sokonanoda-lsp", log } = options;
  const fsImpl = options.fs ?? fs;
  const alpine = options.alpine ?? isAlpineLinux(platform, fsImpl);
  const binary = bundledBinaryPath(extensionPath, platform, arch, baseName, alpine);
  if (!binary || !fsImpl.existsSync(binary)) return undefined;
  if (platform === "win32") return binary;
  if (isExecutable(binary, fsImpl)) return binary;
  try {
    fsImpl.chmodSync(binary, 0o755);
  } catch (error) {
    log?.(`bundled binary is not executable and chmod failed: ${binary}: ${error}`);
    return undefined;
  }
  if (isExecutable(binary, fsImpl)) return binary;
  log?.(`bundled binary is still not executable after chmod: ${binary}`);
  return undefined;
}

function resolveBundledServer(options) {
  return resolveBundledBinary({ ...options, baseName: "sokonanoda-lsp" });
}

/// Resolve the `sokonanoda` CLI (used by the course map): bundled in the VSIX
/// first, then a workspace build, then PATH. No network, no cargo needed.
function resolveCliCommand(options) {
  const { extensionPath, roots = [], platform, arch, log } = options;
  const fsImpl = options.fs ?? fs;
  const bundled = resolveBundledBinary({
    extensionPath,
    platform,
    arch,
    baseName: "sokonanoda",
    fs: fsImpl,
    log,
  });
  if (bundled) return bundled;
  const cliName = platform === "win32" ? "sokonanoda.exe" : "sokonanoda";
  const found = firstExisting(builtBinaryCandidates(roots, cliName), fsImpl);
  if (found) return found;
  return "sokonanoda";
}

/// Auto-download cache directory for the language server binary (legacy
/// fallback path; kept for the universal VSIX).
function serverCacheDir() {
  return path.join(
    process.env.HOME || process.env.USERPROFILE || "",
    ".local",
    "share",
    "sokonanoda",
    "bin",
  );
}

function serverDest() {
  return path.join(
    serverCacheDir(),
    process.platform === "win32" ? "sokonanoda-lsp.exe" : "sokonanoda-lsp",
  );
}

function serverVersionMarker() {
  return serverDest() + ".version";
}

/// The cached server binary is reusable only when its recorded version marker
/// matches the current extension version.
function cachedServerIsCurrent(version, fsImpl = fs) {
  const dest = serverDest();
  if (!fsImpl.existsSync(dest)) return false;
  const marker = serverVersionMarker();
  if (!fsImpl.existsSync(marker)) return false;
  try {
    return fsImpl.readFileSync(marker, "utf8").trim() === String(version);
  } catch {
    return false;
  }
}

/// Version-pinned asset URL. `undefined` for platforms with no build.
function downloadUrl(version, platform, arch, alpine = false) {
  const target = rustTarget(platform, arch, alpine);
  if (!target) return undefined;
  return `${RELEASES_BASE}/download/v${version}/sokonanoda-lsp-${target}.tar.gz`;
}

/// Resolve the server without any network I/O.
/// Options: `{setting, envBin, extensionPath, roots, platform, arch, version,
/// fs, log}`. Returns a path or `undefined` (caller may then download).
function resolveServerCommand(options) {
  const {
    setting,
    envBin,
    extensionPath,
    roots = [],
    platform,
    arch,
    version,
    log,
  } = options;
  const fsImpl = options.fs ?? fs;

  if (typeof setting === "string" && setting.trim() !== "") return setting.trim();
  if (envBin) return envBin;

  const alpine = options.alpine ?? isAlpineLinux(platform, fsImpl);
  const bundled = resolveBundledServer({
    extensionPath,
    platform,
    arch,
    alpine,
    fs: fsImpl,
    log,
  });
  if (bundled) return bundled;

  const found = firstExisting(
    builtBinaryCandidates(roots, binaryName(platform)),
    fsImpl,
  );
  if (found) return found;

  if (cachedServerIsCurrent(version, fsImpl)) return serverDest();
  return undefined;
}

/// Follow redirects recursively (GitHub uses a versioned asset URL directly,
// but proxies/CDNs may still redirect). Exported for unit tests.
function followRedirects(reqUrl, redirectsLeft) {
  const mod = reqUrl.startsWith("https") ? https : http;
  return new Promise((resolve, reject) => {
    mod
      .get(reqUrl, (res) => {
        if (
          (res.statusCode === 301 || res.statusCode === 302 || res.statusCode === 303) &&
          res.headers.location
        ) {
          if (redirectsLeft <= 0) return reject(new Error("too many redirects"));
          res.resume(); // drain the redirect response body
          return followRedirects(res.headers.location, redirectsLeft - 1).then(resolve, reject);
        }
        if (res.statusCode !== 200) {
          res.resume();
          return reject(new Error(`HTTP ${res.statusCode}`));
        }
        resolve(res);
      })
      .on("error", reject);
  });
}

/// Download the version-pinned server binary into the cache directory.
/// Options: `{version, platform, arch, log}`.
async function downloadLspBinary(options) {
  const { version, platform, arch } = options;
  const alpine = options.alpine ?? isAlpineLinux(platform);
  const url = downloadUrl(version, platform, arch, alpine);
  if (!url) {
    throw new Error(
      `当前平台（${platform}-${arch}）没有内置语言服务器，也没有对应的下载构建；` +
        "请设置 sokonanoda.serverPath 指向本地编译的 sokonanoda-lsp。",
    );
  }

  const dir = serverCacheDir();
  const dest = serverDest();
  if (cachedServerIsCurrent(version)) return dest;

  fs.mkdirSync(dir, { recursive: true });
  const tmp = dest + ".tmp.tar.gz";
  const res = await followRedirects(url, 5);
  await new Promise((resolve, reject) => {
    const file = fs.createWriteStream(tmp);
    res.pipe(file);
    file.on("finish", () => file.close(resolve));
    file.on("error", (error) => {
      try {
        fs.unlinkSync(tmp);
      } catch {}
      reject(error);
    });
  });

  try {
    execSync(`tar xzf "${tmp}" -C "${dir}"`, { stdio: "pipe" });
    fs.unlinkSync(tmp);
    if (!fs.existsSync(dest)) throw new Error("tar extracted but binary not found");
    // Release tarballs may carry 0644 (artifact round-trips strip the exec
    // bit); restore it so the fallback path can actually spawn the server.
    if (platform !== "win32") {
      try {
        fs.chmodSync(dest, 0o755);
      } catch {
        // read-only cache dir: the caller surfaces the spawn failure
      }
    }
    fs.writeFileSync(serverVersionMarker(), String(version));
    return dest;
  } catch (error) {
    try {
      fs.unlinkSync(tmp);
    } catch {}
    throw new Error(`tar extraction failed: ${error.message}`);
  }
}

module.exports = {
  RELEASES_BASE,
  isAlpineLinux,
  platformTarget,
  rustTarget,
  binaryName,
  bundledBinaryPath,
  bundledServerPath,
  resolveBundledBinary,
  resolveBundledServer,
  resolveCliCommand,
  firstExisting,
  builtBinaryCandidates,
  serverCacheDir,
  serverDest,
  serverVersionMarker,
  cachedServerIsCurrent,
  downloadUrl,
  downloadLspBinary,
  resolveServerCommand,
  followRedirects,
};
