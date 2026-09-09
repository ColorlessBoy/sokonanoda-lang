// Unit tests for the download logic in extension.js.
// Run: node editor/vscode/test-download.js
// No VS Code, no cargo build required — pure Node.js.

const http = require("http");
const https = require("https");
const fs = require("fs");
const path = require("path");
const os = require("os");
const { execSync } = require("child_process");
const assert = require("assert");

// ── Extract followRedirects + download logic from extension.js (duplicated for
// test isolation; the real code stays in extension.js unchanged). ──────────

function followRedirects(reqUrl, redirectsLeft) {
  const mod = reqUrl.startsWith("https") ? https : http;
  return new Promise((resolve, reject) => {
    mod.get(reqUrl, (res) => {
      if (
        (res.statusCode === 301 || res.statusCode === 302 || res.statusCode === 303) &&
        res.headers.location
      ) {
        if (redirectsLeft <= 0) return reject(new Error("too many redirects"));
        res.resume();
        return followRedirects(res.headers.location, redirectsLeft - 1).then(resolve, reject);
      }
      if (res.statusCode !== 200) {
        res.resume();
        return reject(new Error(`HTTP ${res.statusCode}`));
      }
      resolve(res);
    }).on("error", reject);
  });
}

// ── Helpers ──────────────────────────────────────────────────────────────

function startServer(handler) {
  return new Promise((resolve) => {
    const srv = http.createServer(handler);
    srv.listen(0, "127.0.0.1", () => {
      const { port } = srv.address();
      resolve({ srv, port, url: `http://127.0.0.1:${port}` });
    });
  });
}

function closeServer(srv) {
  return new Promise((resolve) => srv.close(resolve));
}

// ── Tests ────────────────────────────────────────────────────────────────

let passed = 0;
let failed = 0;

async function test(name, fn) {
  try {
    await fn();
    passed++;
    console.log(`  ✓ ${name}`);
  } catch (e) {
    failed++;
    console.error(`  ✗ ${name}`);
    console.error(`    ${e.message}`);
  }
}

async function run() {
  console.log("followRedirects unit tests\n");

  // 1. No redirect — direct 200
  await test("200 direct (no redirect)", async () => {
    const { srv, url, port } = await startServer((_, res) => {
      res.writeHead(200, { "content-type": "text/plain" });
      res.end("ok");
    });
    try {
      const res = await followRedirects(url, 5);
      const body = await new Promise((resolve) => {
        let d = "";
        res.on("data", (c) => (d += c));
        res.on("end", () => resolve(d));
      });
      assert.strictEqual(body, "ok");
    } finally {
      await closeServer(srv);
    }
  });

  // 2. Single 302 redirect
  await test("single 302 redirect", async () => {
    const { srv, port } = await startServer((req, res) => {
      if (req.url === "/start") {
        res.writeHead(302, { location: `http://127.0.0.1:${port}/target` });
        res.end();
      } else {
        res.writeHead(200, { "content-type": "text/plain" });
        res.end("hello");
      }
    });
    try {
      const res = await followRedirects(`http://127.0.0.1:${port}/start`, 5);
      const body = await new Promise((resolve) => {
        let d = "";
        res.on("data", (c) => (d += c));
        res.on("end", () => resolve(d));
      });
      assert.strictEqual(body, "hello");
    } finally {
      await closeServer(srv);
    }
  });

  // 3. Double 302 redirect (the GitHub pattern)
  await test("double 302 redirect (GitHub pattern)", async () => {
    const { srv, port } = await startServer((req, res) => {
      if (req.url === "/latest") {
        res.writeHead(302, { location: `http://127.0.0.1:${port}/versioned` });
        res.end();
      } else if (req.url === "/versioned") {
        res.writeHead(302, { location: `http://127.0.0.1:${port}/cdn` });
        res.end();
      } else {
        res.writeHead(200, { "content-type": "application/octet-stream" });
        res.end("binary-data");
      }
    });
    try {
      const res = await followRedirects(`http://127.0.0.1:${port}/latest`, 5);
      const body = await new Promise((resolve) => {
        let d = "";
        res.on("data", (c) => (d += c));
        res.on("end", () => resolve(d));
      });
      assert.strictEqual(body, "binary-data");
    } finally {
      await closeServer(srv);
    }
  });

  // 4. Too many redirects
  await test("rejects on too many redirects", async () => {
    const { srv, port } = await startServer((req, res) => {
      res.writeHead(302, { location: `http://127.0.0.1:${port}/loop` });
      res.end();
    });
    try {
      await assert.rejects(
        followRedirects(`http://127.0.0.1:${port}/loop`, 2),
        /too many redirects/,
      );
    } finally {
      await closeServer(srv);
    }
  });

  // 5. HTTP error (non-redirect, non-200)
  await test("rejects on HTTP 500", async () => {
    const { srv, url } = await startServer((_, res) => {
      res.writeHead(500);
      res.end();
    });
    try {
      await assert.rejects(followRedirects(url, 5), /HTTP 500/);
    } finally {
      await closeServer(srv);
    }
  });

  // 6. 301 permanent redirect
  await test("follows 301 redirect", async () => {
    const { srv, port } = await startServer((req, res) => {
      if (req.url === "/old") {
        res.writeHead(301, { location: `http://127.0.0.1:${port}/new` });
        res.end();
      } else {
        res.writeHead(200);
        res.end("moved");
      }
    });
    try {
      const res = await followRedirects(`http://127.0.0.1:${port}/old`, 5);
      const body = await new Promise((resolve) => {
        let d = "";
        res.on("data", (c) => (d += c));
        res.on("end", () => resolve(d));
      });
      assert.strictEqual(body, "moved");
    } finally {
      await closeServer(srv);
    }
  });

  // 7. Full download + tar extract simulation
  await test("download → tar extract → binary exists", async () => {
    // Create a fake tarball
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "dl-test-"));
    const tarSrc = path.join(tmpDir, "src");
    const tarDst = path.join(tmpDir, "dst");
    const binName = "fake-binary";
    fs.mkdirSync(tarSrc, { recursive: true });
    fs.mkdirSync(tarDst, { recursive: true });
    fs.writeFileSync(path.join(tarSrc, binName), "ELF-binary-content");
    execSync(`tar czf "${tmpDir}/ball.tar.gz" -C "${tarSrc}" .`, { stdio: "pipe" });
    const tarBuf = fs.readFileSync(`${tmpDir}/ball.tar.gz`);

    // Serve the tarball
    const { srv, url } = await startServer((_, res) => {
      res.writeHead(200, { "content-type": "application/gzip" });
      res.end(tarBuf);
    });

    try {
      const res = await followRedirects(url, 5);
      const tmpFile = path.join(tmpDir, "dl.tar.gz");
      const ws = fs.createWriteStream(tmpFile);
      await new Promise((resolve, reject) => {
        res.pipe(ws);
        ws.on("finish", resolve);
        ws.on("error", reject);
      });
      execSync(`tar xzf "${tmpFile}" -C "${tarDst}"`, { stdio: "pipe" });
      fs.unlinkSync(tmpFile);
      const extracted = path.join(tarDst, binName);
      assert.ok(fs.existsSync(extracted), "extracted binary must exist");
      assert.strictEqual(fs.readFileSync(extracted, "utf8"), "ELF-binary-content");
    } finally {
      await closeServer(srv);
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  // ── Summary ──────────────────────────────────────────────────────────
  console.log(`\n${passed + failed} tests, ${passed} passed, ${failed} failed\n`);
  process.exit(failed > 0 ? 1 : 0);
}

run();
