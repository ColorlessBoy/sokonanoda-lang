// Stage the reader-facing tactic docs (`reference/tactics/*.md`) into
// `editor/vscode/docs/tactics/` so the packaged VSIX carries them as real
// files and the extension can hand that directory to the LSP through
// `SOKONANODA_DOCS_DIR` (docs/design/tactic-docs.md §4.9 D10).
//
// `docs/` is a generated directory, exactly like `bin/` (stage-lsp.js):
// gitignored, produced at packaging time, shipped by vsce. It is NOT a
// committed copy — a stale committed copy would let the packaged docs drift
// away from `reference/tactics/`, which is the one source of truth.
//
// Usage:
//   node scripts/stage-docs.js           # mirror the source into docs/tactics/
//   node scripts/stage-docs.js --check   # compare only; exit 1 on any drift
//
// The mirror is byte-exact and idempotent: identical files are left untouched,
// and files that no longer exist in `reference/tactics/` are removed (a stale
// page in the VSIX is drift too).
//
// Exit codes: 0 = staged / in sync · 1 = drift (--check) · 2 = environment or
// usage error.

const fs = require("fs");
const path = require("path");

const EXTENSION_DIR = path.join(__dirname, "..");
const REPO_ROOT = path.join(EXTENSION_DIR, "..", "..");
const SOURCE_DIR = path.join(REPO_ROOT, "reference", "tactics");
const DEST_DIR = path.join(EXTENSION_DIR, "docs", "tactics");

/// `reference/tactics/*.md` — the 14 tactic pages plus the index. Sorted so the
/// log and the `--check` report are stable across platforms.
function sourceNames() {
  return fs
    .readdirSync(SOURCE_DIR)
    .filter((name) => name.endsWith(".md"))
    .sort();
}

/// What a stage would do, without touching anything: `changed` = files that
/// must be written (missing or not byte-equal), `stale` = files in the
/// generated directory that the source no longer has.
function plan() {
  const names = sourceNames();
  const changed = [];
  for (const name of names) {
    const source = path.join(SOURCE_DIR, name);
    const dest = path.join(DEST_DIR, name);
    const want = fs.readFileSync(source);
    const have = fs.existsSync(dest) ? fs.readFileSync(dest) : undefined;
    if (have === undefined || !want.equals(have)) {
      changed.push({ name, source, dest, missing: have === undefined });
    }
  }
  const stale = fs.existsSync(DEST_DIR)
    ? fs
        .readdirSync(DEST_DIR)
        .filter((name) => !names.includes(name))
        .sort()
    : [];
  return { names, changed, stale };
}

/// Mirror `reference/tactics/` into `docs/tactics/`. Returns
/// `{files, copied, removed}`. Safe to call from another script (wiring point ①
/// in docs/design/tactic-docs.md §4.9.6 calls it from stage-lsp.js).
function stage(log = console.log) {
  const { names, changed, stale } = plan();
  fs.mkdirSync(DEST_DIR, { recursive: true });
  for (const item of changed) {
    fs.copyFileSync(item.source, item.dest);
    log(`staged reference/tactics/${item.name} -> docs/tactics/${item.name}`);
  }
  for (const name of stale) {
    fs.rmSync(path.join(DEST_DIR, name), { force: true });
    log(`removed docs/tactics/${name} (no longer in reference/tactics)`);
  }
  return { files: names.length, copied: changed.length, removed: stale.length };
}

function main(argv) {
  for (const arg of argv) {
    if (arg !== "--check") {
      console.error(`stage-docs: unknown argument: ${arg}`);
      return 2;
    }
  }
  if (!fs.existsSync(SOURCE_DIR)) {
    console.error(`stage-docs: ${path.relative(REPO_ROOT, SOURCE_DIR)} 不存在 ⇒ 环境错`);
    return 2;
  }
  if (!argv.includes("--check")) {
    const result = stage();
    console.log(
      `stage-docs: ✓ docs/tactics/ 与 reference/tactics/ 一致（${result.files} 篇，` +
        `本次拷 ${result.copied}、删 ${result.removed}）`,
    );
    return 0;
  }
  const { names, changed, stale } = plan();
  if (changed.length === 0 && stale.length === 0) {
    console.log(`stage-docs: ✓ ${names.length} 篇与 reference/tactics/ 逐字节一致`);
    return 0;
  }
  console.error("stage-docs: 插件里的文档与 reference/tactics/ 漂移 ✗");
  for (const item of changed) {
    console.error(`  ✗ docs/tactics/${item.name}（${item.missing ? "缺失" : "内容不同"}）`);
  }
  for (const name of stale) console.error(`  ✗ docs/tactics/${name}（源里已没有这一篇）`);
  console.error("修法：node editor/vscode/scripts/stage-docs.js");
  return 1;
}

if (require.main === module) process.exit(main(process.argv.slice(2)));
module.exports = { SOURCE_DIR, DEST_DIR, sourceNames, plan, stage };
