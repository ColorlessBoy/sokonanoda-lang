//! DeepSeek Harness (DSH) integration contract: the files this repository
//! commits so that a DSH session finds the role skills, reaches the pinned
//! binaries, and wires the language server — without any user-profile edit.
//!
//! Design + evidence: `docs/design/deepseek-harness.md`. The load-bearing DSH
//! facts asserted here (skill roots, frontmatter keys, slash-command naming,
//! the patch-file shape) are the ones the design documents with `path:line`
//! citations; this test only pins *our own files* so they cannot drift, never
//! DSH's internal behaviour.

use std::fs;
use std::path::{Path, PathBuf};

mod common;

use common::repo_root;

/// DSH skill roots scanned inside a repository (rank 100 / 200 respectively,
/// see `docs/design/deepseek-harness.md` §1.2 D2). `.agents/skills` is the one
/// this repository ships.
const DSH_SKILL_ROOT: &str = ".agents/skills";

/// Frontmatter keys DSH accepts on a SKILL.md. `whenToUse` is the only
/// camelCase key; the legacy camelCase *invocation* keys
/// (`disableModelInvocation`/`modelInvocable`/`userInvocable`) make DSH drop
/// the whole skill, so they must never appear.
const DSH_FRONTMATTER_KEYS: [&str; 6] = [
    "name",
    "description",
    "whenToUse",
    "metadata",
    "disable-model-invocation",
    "user-invocable",
];

/// Keys whose presence drops the skill in DSH.
const DSH_REJECTED_KEYS: [&str; 3] = ["disableModelInvocation", "modelInvocable", "userInvocable"];

struct Frontmatter {
    /// Key/value pairs in declaration order (value kept verbatim, trimmed).
    pairs: Vec<(String, String)>,
    /// Body after the closing fence.
    body: String,
}

impl Frontmatter {
    fn value(&self, key: &str) -> Option<&str> {
        self.pairs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

fn parse_frontmatter(text: &str, what: &str) -> Frontmatter {
    let rest = text
        .strip_prefix("---\n")
        .unwrap_or_else(|| panic!("{what}: must start with a `---` frontmatter fence"));
    let (block, body) = rest
        .split_once("\n---\n")
        .unwrap_or_else(|| panic!("{what}: frontmatter fence is not closed"));
    let mut pairs = Vec::new();
    for line in block.lines() {
        let Some((key, value)) = line.split_once(": ") else {
            continue;
        };
        pairs.push((key.trim().to_string(), value.trim().to_string()));
    }
    Frontmatter {
        pairs,
        body: body.to_string(),
    }
}

/// Directory names under `skills/` that actually carry a `SKILL.md`.
fn repo_skills() -> Vec<String> {
    let mut out: Vec<String> = fs::read_dir(repo_root().join("skills"))
        .expect("skills/ exists")
        .map(|entry| entry.expect("entry readable").path())
        .filter(|path| path.join("SKILL.md").is_file())
        .map(|path| {
            path.file_name()
                .expect("dir name")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    out.sort();
    assert!(!out.is_empty(), "skills/ must carry at least one SKILL.md");
    out
}

/// Directory names under `.agents/skills/`.
fn dsh_skill_entries() -> Vec<String> {
    let root = repo_root().join(DSH_SKILL_ROOT);
    assert!(
        root.is_dir(),
        "{DSH_SKILL_ROOT} must exist so DSH discovers the role skills \
         (docs/design/deepseek-harness.md H0)"
    );
    let mut out: Vec<String> = fs::read_dir(&root)
        .expect(".agents/skills readable")
        .map(|entry| entry.expect("entry readable").path())
        .filter(|path| path.is_dir() || path.is_symlink())
        .map(|path| {
            path.file_name()
                .expect("entry name")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    out.sort();
    out
}

/// Resolve a DSH skill entry's `SKILL.md`, following a symlinked bundle.
fn entry_skill_md(name: &str) -> Option<PathBuf> {
    let path = repo_root().join(DSH_SKILL_ROOT).join(name).join("SKILL.md");
    path.is_file().then_some(path)
}

/// A kebab-case DSH skill name: `^[a-z0-9]+(?:-[a-z0-9]+)*$`.
fn is_skill_name(name: &str) -> bool {
    !name.is_empty()
        && name.split('-').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

#[test]
fn every_repo_skill_is_reachable_from_the_dsh_skill_root() {
    let repo = repo_skills();
    let dsh = dsh_skill_entries();
    assert_eq!(
        repo, dsh,
        "every skills/<name>/SKILL.md must have a {DSH_SKILL_ROOT}/<name> entry and vice versa \
         (a missing entry silently hides the role from DSH; an orphan entry points at nothing)"
    );
}

#[test]
fn dsh_skill_entries_declare_a_valid_kebab_case_name_and_description() {
    for name in dsh_skill_entries() {
        let path = entry_skill_md(&name)
            .unwrap_or_else(|| panic!("{DSH_SKILL_ROOT}/{name}/SKILL.md must be readable"));
        let text = fs::read_to_string(&path).expect("SKILL.md readable");
        let what = format!("{DSH_SKILL_ROOT}/{name}/SKILL.md");
        let fm = parse_frontmatter(&text, &what);

        let declared = fm
            .value("name")
            .unwrap_or_else(|| panic!("{what}: frontmatter declares `name`"));
        assert_eq!(
            declared, name,
            "{what}: `name` must equal the directory name"
        );
        assert!(
            is_skill_name(declared),
            "{what}: `name` must be kebab-case, got `{declared}`"
        );

        let description = fm
            .value("description")
            .unwrap_or_else(|| panic!("{what}: frontmatter declares `description`"));
        assert!(
            description.len() >= 20,
            "{what}: `description` is the only model-visible routing field in DSH \
             (whenToUse reaches the human `/` menu only) — write a real routing sentence"
        );
    }
}

#[test]
fn dsh_frontmatter_uses_only_keys_dsh_accepts() {
    for name in dsh_skill_entries() {
        let path = entry_skill_md(&name).expect("entry SKILL.md readable");
        let text = fs::read_to_string(&path).expect("SKILL.md readable");
        let what = format!("{DSH_SKILL_ROOT}/{name}/SKILL.md");
        let fm = parse_frontmatter(&text, &what);

        for key in DSH_REJECTED_KEYS {
            assert!(
                fm.value(key).is_none(),
                "{what}: `{key}` is the legacy camelCase spelling; DSH rejects it and drops the \
                 whole skill — use the kebab-case key documented in docs/design/deepseek-harness.md"
            );
        }
        for (key, _) in &fm.pairs {
            assert!(
                DSH_FRONTMATTER_KEYS.contains(&key.as_str()),
                "{what}: unknown frontmatter key `{key}`; DSH's accepted set is {DSH_FRONTMATTER_KEYS:?}"
            );
        }
    }
}

#[test]
fn dsh_skill_entries_point_at_the_canonical_manual() {
    for name in dsh_skill_entries() {
        let path = entry_skill_md(&name).expect("entry SKILL.md readable");
        let text = fs::read_to_string(&path).expect("SKILL.md readable");
        let what = format!("{DSH_SKILL_ROOT}/{name}/SKILL.md");
        let fm = parse_frontmatter(&text, &what);

        // A symlinked bundle carries the manual itself; a thin gateway must
        // hand the model the repository path of the canonical manual, and that
        // path must exist. Either shape keeps one source of truth.
        let canonical = format!("skills/{name}/SKILL.md");
        if path.is_symlink() || !text.contains(&canonical) {
            assert!(
                text.contains(&canonical),
                "{what}: must name the canonical manual `{canonical}` so the model can read it \
                 (the entry's own directory is the DSH resource base, not the repo root)"
            );
        }
        assert!(
            repo_root().join(&canonical).is_file(),
            "{what}: `{canonical}` must exist"
        );
        assert!(
            text.contains("repository root"),
            "{what}: must tell the model to resolve `{canonical}` against the repository root, \
             not against the entry directory"
        );
        assert!(!fm.body.trim().is_empty(), "{what}: body must not be empty");
    }
}

/// The committed DSH patch file: a top-level YAML array of loader patch
/// entries (`docs/design/deepseek-harness.md` §1.2 D10). We assert only the
/// structural facts our own file must satisfy.
fn dsh_patch(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("{} readable: {e}", path.display()))
}

#[test]
fn dsh_patch_file_is_a_top_level_yaml_array() {
    let path = repo_root().join("dsh/cordis.patch.yml");
    let text = dsh_patch(&path);

    assert!(
        !text.trim().is_empty(),
        "an empty or comment-only patch file makes DSH fail to boot; write `[]` to disable a layer"
    );
    // Reason about the document, not the prose comment block above it.
    let body: String = text
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    let first = body
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default()
        .trim()
        .to_string();
    assert!(
        first.starts_with("- "),
        "a DSH patch file is a top-level YAML array of patch entries, got first line `{first}`"
    );

    for needle in [
        "@deepseek-ai/dsh-lsp",
        "@deepseek-ai/dsh-lsp-stdio",
        "@deepseek-ai/dsh-tool-lsp",
        "extensionToLanguage",
        "'.sokonanoda'",
        "sokonanoda",
    ] {
        assert!(
            text.contains(needle),
            "dsh/cordis.patch.yml must contain `{needle}` (got: {text})"
        );
    }
    assert!(
        !text.contains("/Users/"),
        "dsh/cordis.patch.yml must not hard-code a machine path; derive it from the patch location"
    );
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("- id: ") {
            assert!(
                !rest.trim().is_empty(),
                "a non-insert patch entry requires an `id`"
            );
        }
    }
}

#[test]
fn dsh_documentation_and_launcher_exist() {
    let root = repo_root();
    for (path, needles) in [
        (
            "docs/design/deepseek-harness.md",
            vec!["H0", "H4", "publishDiagnostics", "scripts/soko"],
        ),
        (
            "docs/notes/dsh-project-assets.md",
            vec!["path:line", "customSkillDirs"],
        ),
    ] {
        let text =
            fs::read_to_string(root.join(path)).unwrap_or_else(|e| panic!("{path} readable: {e}"));
        for needle in needles {
            assert!(text.contains(needle), "{path} must mention `{needle}`");
        }
    }

    // The launcher is the harness-neutral way to reach the pinned binaries.
    let launcher = root.join("scripts/soko");
    let text = fs::read_to_string(&launcher).expect("scripts/soko readable");
    assert!(
        text.starts_with("#!/usr/bin/env node"),
        "scripts/soko must be a Node program (cross-platform, no bash)"
    );
    for needle in [
        "SOKONANODA_BIN",
        "SOKONANODA_OFFLINE",
        "releases/download",
        "marker",
    ] {
        assert!(
            text.contains(needle),
            "scripts/soko must keep `{needle}` in its resolution chain"
        );
    }
    // Downloads carry the pinned version in the URL path, so the mutable
    // "latest" release can never be addressed. Assert the shape, not the word:
    // prose that tells maintainers not to use it must stay allowed.
    assert!(
        text.contains("/v${version}/"),
        "the download URL must stay version-tagged (`${{RELEASES}}/v${{version}}/…`)"
    );
    assert!(
        !text.contains("/latest/"),
        "scripts/soko must never build a download URL under /latest/"
    );
}

#[test]
fn dsh_lean_toolchain_deny_is_wired() {
    let root = repo_root();
    let config = root.join("dsh/hooks/hooks.json");
    let text = fs::read_to_string(&config).expect("dsh/hooks/hooks.json readable");
    for needle in [
        "PreToolUse",
        "\"matcher\": \"bash\"",
        "refuse-lean-toolchain.js",
    ] {
        assert!(
            text.contains(needle),
            "dsh/hooks/hooks.json must wire `{needle}` (the DSH equivalent of opencode.json's \
             permission rule; see dsh/README.md)"
        );
    }

    let hook = root.join("dsh/hooks/refuse-lean-toolchain.js");
    let text = fs::read_to_string(&hook).expect("dsh/hooks/refuse-lean-toolchain.js readable");
    assert!(
        text.starts_with("#!/usr/bin/env node"),
        "the deny hook must be a Node program (no bash)"
    );
    for tool in ["lean", "lake", "leanc", "lean4export", "elan"] {
        assert!(
            text.contains(tool),
            "the deny hook must cover `{tool}` (REQUIREMENTS.md §2 rule 2)"
        );
    }
    assert!(
        text.contains("process.exit(2)"),
        "the deny hook must block the call (exit 2 carries the reason on stderr)"
    );
}
