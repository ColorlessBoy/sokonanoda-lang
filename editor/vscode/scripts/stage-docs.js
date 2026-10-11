// Stage the reader-facing reference docs (`reference/tactics/*.md` and
// `reference/commands/*.md`) into `editor/vscode/docs/<sub>/` so the packaged
// VSIX carries them as real files and the extension can hand those directories
// to the LSP through `SOKONANODA_DOCS_DIR` (tactics) and
// `SOKONANODA_COMMAND_DOCS_DIR` (commands) —
// docs/design/tactic-docs.md §4.9 D10 + docs/design/command-docs.md.
//
// `docs/` is a generated directory, exactly like `bin/` (stage-lsp.js):
// gitignored, produced at packaging time, shipped by vsce. It is NOT a
// committed copy — a stale committed copy would let the packaged docs drift
// away from `reference/`, which is the one source of truth.
//
// Usage:
//   node scripts/stage-docs.js           # mirror every pair into docs/<sub>/
//   node scripts/stage-docs.js --check   # compare only; exit 1 on any drift
//
// The mirror is byte-exact and idempotent: identical files are left untouched,
// and files that no longer exist in `reference/<sub>/` are removed (a stale
// page in the VSIX is drift too).
//
// Exit codes: 0 = staged / in sync · 1 = drift (--check) · 2 = environment or
// usage error.

const fs = require("fs");
const path = require("path");

const EXTENSION_DIR = path.join(__dirname, "..");
const REPO_ROOT = path.join(EXTENSION_DIR, "..", "..");

/// Every source → packaged-copy pair. `env` documents which environment
/// variable the extension hands this directory to the LSP through (the wire
/// seam guard lives in `crates/cli/tests/extension.rs`).
const PAIRS = [
  { sub: "tactics", env: "SOKONANODA_DOCS_DIR" },
  { sub: "commands", env: "SOKONANODA_COMMAND_DOCS_DIR" },
].map((pair) => ({
  ...pair,
  source: path.join(REPO_ROOT, "reference", pair.sub),
  dest: path.join(EXTENSION_DIR, "docs", pair.sub),
}));

/// `reference/<sub>/*.md` (the pages plus the index). Sorted so the log and the
/// `--check` report are stable across platforms.
function sourceNames(pair) {
  return fs
    .readdirSync(pair.source)
    .filter((name) => name.endsWith(".md"))
    .sort();
}

/// What a stage would do for one pair, without touching anything:
/// `changed` = files that must be written (missing or not byte-equal),
/// `stale` = files in the generated directory that the source no longer has.
function plan(pair) {
  const names = sourceNames(pair);
  const changed = [];
  for (const name of names) {
    const source = path.join(pair.source, name);
    const dest = path.join(pair.dest, name);
    const want = fs.readFileSync(source);
    const have = fs.existsSync(dest) ? fs.readFileSync(dest) : undefined;
    if (have === undefined || !want.equals(have)) {
      changed.push({ name, source, dest, missing: have === undefined });
    }
  }
  const stale = fs.existsSync(pair.dest)
    ? fs
        .readdirSync(pair.dest)
        .filter((name) => !names.includes(name))
        .sort()
    : [];
  return { names, changed, stale };
}

/// Mirror every pair. Returns `{files, copied, removed}` (totals). Safe to call
/// from another script (wiring point ① in docs/design/tactic-docs.md §4.9.6
/// calls it from stage-lsp.js).
function stage(log = console.log) {
  let files = 0;
  let copied = 0;
  let removed = 0;
  for (const pair of PAIRS) {
    const { names, changed, stale } = plan(pair);
    fs.mkdirSync(pair.dest, { recursive: true });
    for (const item of changed) {
      fs.copyFileSync(item.source, item.dest);
      log(`staged reference/${pair.sub}/${item.name} -> docs/${pair.sub}/${item.name}`);
    }
    for (const name of stale) {
      fs.rmSync(path.join(pair.dest, name), { force: true });
      log(`removed docs/${pair.sub}/${name} (no longer in reference/${pair.sub})`);
    }
    files += names.length;
    copied += changed.length;
    removed += stale.length;
  }
  return { files, copied, removed };
}

function main(argv) {
  for (const arg of argv) {
    if (arg !== "--check") {
      console.error(`stage-docs: unknown argument: ${arg}`);
      return 2;
    }
  }
  for (const pair of PAIRS) {
    if (!fs.existsSync(pair.source)) {
      console.error(`stage-docs: ${path.relative(REPO_ROOT, pair.source)} 不存在 ⇒ 环境错`);
      return 2;
    }
  }
  if (!argv.includes("--check")) {
    const result = stage();
    const perDir = PAIRS.map((pair) => `${pair.sub} ${sourceNames(pair).length}`).join(" + ");
    console.log(
      `stage-docs: ✓ docs/ 与 reference/ 一致（${perDir} = ${result.files} 篇，` +
        `本次拷 ${result.copied}、删 ${result.removed}）`,
    );
    return 0;
  }
  let drift = 0;
  for (const pair of PAIRS) {
    const { names, changed, stale } = plan(pair);
    if (changed.length === 0 && stale.length === 0) {
      console.log(
        `stage-docs: ✓ docs/${pair.sub}/ ${names.length} 篇与 reference/${pair.sub}/ 逐字节一致`,
      );
      continue;
    }
    drift += changed.length + stale.length;
    console.error(`stage-docs: docs/${pair.sub}/ 与 reference/${pair.sub}/ 漂移 ✗`);
    for (const item of changed) {
      console.error(
        `  ✗ docs/${pair.sub}/${item.name}（${item.missing ? "缺失" : "内容不同"}）`,
      );
    }
    for (const name of stale) {
      console.error(`  ✗ docs/${pair.sub}/${name}（源里已没有这一篇）`);
    }
  }
  if (drift > 0) {
    console.error("修法：node editor/vscode/scripts/stage-docs.js");
    return 1;
  }
  return 0;
}

if (require.main === module) process.exit(main(process.argv.slice(2)));
module.exports = { PAIRS, sourceNames, plan, stage };
