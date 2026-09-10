//! VS Code client contract tests (`editor/vscode/`): the unpackaged extension
//! is part of the product surface, so its manifest and entry script must stay
//! consistent with the server's custom goal-view protocol and the CLI's
//! course-map subcommand — without needing an Electron run in CI.

use std::fs;
use std::path::Path;

use serde_json::Value;

fn vscode_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../editor/vscode")
        .canonicalize()
        .expect("editor/vscode exists")
}

fn manifest() -> Value {
    let raw = fs::read_to_string(vscode_dir().join("package.json")).expect("package.json");
    serde_json::from_str(&raw).expect("package.json is valid JSON")
}

fn entry_script() -> String {
    fs::read_to_string(vscode_dir().join("extension.js")).expect("extension.js")
}

fn server_script() -> String {
    fs::read_to_string(vscode_dir().join("server.js")).expect("server.js")
}

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root exists")
}

#[test]
fn manifest_declares_commands_that_extension_registers() {
    let manifest = manifest();
    let script = entry_script();
    let commands = manifest["contributes"]["commands"]
        .as_array()
        .expect("contributes.commands");
    assert!(
        commands.len() >= 4,
        "showStatus/status/nextHole/goals.refresh at least"
    );
    for command in commands {
        let id = command["command"].as_str().expect("command id");
        assert!(
            script.contains(&format!("\"{id}\"")),
            "extension.js must register {id}"
        );
    }
    // keybindings reference declared commands only.
    for binding in manifest["contributes"]["keybindings"]
        .as_array()
        .expect("keybindings")
    {
        let id = binding["command"].as_str().expect("keybinding command");
        assert!(
            commands.iter().any(|c| c["command"].as_str() == Some(id)),
            "keybinding targets undeclared command {id}"
        );
    }
}

#[test]
fn entry_script_speaks_the_goal_view_protocol() {
    let script = entry_script();
    // The custom goal-view requests (docs/protocol.md) are consumed client-side.
    assert!(
        script.contains("soko/goals"),
        "the goals tree must consume soko/goals"
    );
    assert!(
        script.contains("soko/nextHole"),
        "next-hole navigation must consume soko/nextHole"
    );
    assert!(
        script.contains("soko/hints"),
        "the hint button must consume soko/hints"
    );
    // Cursor goal view (docs/design-by-tactics.md §6): the tree's
    // 「当前光标处」 group polls state at the caret on selection changes.
    assert!(
        script.contains("soko/stateAt"),
        "the cursor goal view must consume soko/stateAt"
    );
    assert!(
        script.contains("onDidChangeTextEditorSelection"),
        "the cursor goal view must track selection changes"
    );
    // Server-side hole logic: the client must NOT scan for holes by text.
    assert!(
        !script.contains("find(\"sorry\")") && !script.contains("indexOf(\"sorry\")"),
        "clients must not re-derive hole positions (ocaml-lsp lesson)"
    );
}

#[test]
fn goals_holes_carry_ids() {
    let script = entry_script();
    // `soko/goals` holes are `{"range": …, "id": …}` objects since the
    // stable hole-id wire change (docs/protocol.md) — never bare ranges.
    // The client must read hole positions through `hole.range`; feeding a
    // hole object itself where a range is expected would corrupt every
    // reveal/selection after the shape change.
    assert!(
        script.contains("hole.range"),
        "the goals tree must read hole positions as hole.range (holes are {{range, id}} objects)"
    );
    assert!(
        !script.contains("[this.uri, hole]"),
        "a holes element is {{range, id}} — it must never be passed as a bare range"
    );
}

#[test]
fn course_map_consumes_the_cli_course_subcommand() {
    let manifest = manifest();
    let script = entry_script();
    // package.json declares the 「课程」 tree view next to the goals tree.
    let views = manifest["contributes"]["views"]["explorer"]
        .as_array()
        .expect("explorer views");
    assert!(
        views
            .iter()
            .any(|v| v["id"].as_str() == Some("sokonanoda.courseMap")),
        "package.json must declare the sokonanoda.courseMap view"
    );
    let commands = manifest["contributes"]["commands"]
        .as_array()
        .expect("contributes.commands");
    assert!(
        commands
            .iter()
            .any(|c| c["command"].as_str() == Some("sokonanoda.courseRefresh")),
        "package.json must declare sokonanoda.courseRefresh"
    );
    assert!(
        script.contains("\"sokonanoda.courseRefresh\""),
        "extension.js must register sokonanoda.courseRefresh"
    );
    // The course map shells out to `sokonanoda course <manifest> --json`
    // (docs/protocol.md "Course map") and renders `course.unit` events —
    // the client must never re-derive unit status (docs/design-course-status.md
    // §0: aggregation lives in the CLI, the LSP server stays single-document).
    assert!(
        script.contains("\"course\"") && script.contains("--json"),
        "the course tree must invoke `sokonanoda course <manifest> --json`"
    );
    assert!(
        script.contains("course.unit"),
        "the course tree must parse course.unit events"
    );
    assert!(
        !script.contains("soko/courseStatus"),
        "course aggregation must not go through the LSP server (CLI subprocess by design)"
    );
    // Subprocess discipline: bounded runtime (timeout + kill).
    assert!(
        script.contains("setTimeout") && script.contains("kill"),
        "the course subprocess must be killed on timeout"
    );
}

#[test]
fn reveal_hint_command_is_wired_to_the_hints_protocol() {
    let manifest = manifest();
    let script = entry_script();
    // package.json declares the command; extension.js registers it.
    let commands = manifest["contributes"]["commands"]
        .as_array()
        .expect("contributes.commands");
    assert!(
        commands
            .iter()
            .any(|c| c["command"].as_str() == Some("sokonanoda.revealHint")),
        "package.json must declare sokonanoda.revealHint"
    );
    assert!(
        script.contains("\"sokonanoda.revealHint\""),
        "extension.js must register sokonanoda.revealHint"
    );
    // Progressive disclosure lives in the client (the server stays stateless):
    // the reveal counter is persisted in workspaceState, keyed per document
    // and declaration.
    assert!(
        script.contains("workspaceState"),
        "reveal progress must be persisted in workspaceState"
    );
    // Teaching rule (docs/design-hints-suggestions.md §0.2): the UI must never
    // advertise how many hints remain — previews push students to the answer.
    assert!(
        !script.contains("还剩"),
        "the reveal flow must never show the remaining hint count"
    );
}

#[test]
fn runtime_dependency_is_packaged() {
    // P0 regression guard: vscode-languageclient must live in dependencies —
    // devDependencies are excluded from VSIX, which shipped a broken package.
    let manifest = manifest();
    let deps = manifest["dependencies"].as_object().expect("dependencies");
    assert!(
        deps.contains_key("vscode-languageclient"),
        "vscode-languageclient must be a runtime dependency"
    );
    let dev = manifest["devDependencies"].as_object();
    assert!(
        dev.map(|d| !d.contains_key("vscode-languageclient"))
            .unwrap_or(true),
        "vscode-languageclient must not live in devDependencies"
    );
    // vsce bundles production node_modules into the VSIX unless ignored —
    // excluding them ships a package that fails with "Cannot find module".
    let ignore = fs::read_to_string(vscode_dir().join(".vscodeignore")).expect(".vscodeignore");
    assert!(
        !ignore.lines().any(|l| {
            let l = l.trim();
            l == "node_modules/**" || l == "node_modules" || l == "node_modules/*"
        }),
        ".vscodeignore must not exclude node_modules (runtime deps must ship)"
    );
}

#[test]
fn packaging_metadata_is_complete() {
    let manifest = manifest();
    for field in [
        "publisher",
        "license",
        "engines",
        "repository",
        "displayName",
    ] {
        assert!(
            manifest.get(field).is_some_and(|v| !v.is_null()),
            "package.json must declare {field} (vsce packaging requirement)"
        );
    }
    assert!(
        vscode_dir().join("LICENSE").exists() && vscode_dir().join("CHANGELOG.md").exists(),
        "LICENSE and CHANGELOG.md are required for packaging"
    );
}

#[test]
fn server_acquisition_prefers_the_bundled_binary() {
    // Bundled-LSP contract (docs/design-bundled-lsp.md §3.2): the extension
    // wires acquisition through server.js, which resolves the binary shipped
    // in the VSIX before any workspace build or download.
    let script = entry_script();
    assert!(
        script.contains("require(\"./server\")"),
        "extension.js must delegate server acquisition to server.js"
    );
    let server = server_script();
    for target in ["darwin-arm64", "darwin-x64", "linux-x64", "win32-x64"] {
        assert!(
            server.contains(target),
            "server.js must map the {target} platform to its bundled target directory"
        );
    }
    assert!(
        server.contains("\"bin\"") && server.contains("chmodSync") && server.contains("0o755"),
        "server.js must resolve bin/<target>/ and repair a lost executable bit"
    );
    // Version-skew regression (docs/design-bundled-lsp.md §0.5): the fallback
    // download must be pinned to the extension's own release, never `latest`.
    assert!(
        !server.contains("/releases/latest/") && !server.contains("/latest/download/"),
        "server.js must never download from releases/latest"
    );
    assert!(
        server.contains("/download/v${version}/"),
        "server.js must pin the fallback download to v<extension version>"
    );
}

#[test]
fn package_scripts_stage_the_bundled_binary() {
    let manifest = manifest();
    let scripts = manifest["scripts"].as_object().expect("scripts");
    for key in [
        "stage:lsp",
        "package:host",
        "package:universal",
        "clean:lsp",
    ] {
        assert!(
            scripts.contains_key(key),
            "package.json must declare the {key} script"
        );
    }
    assert!(
        scripts["stage:lsp"]
            .as_str()
            .is_some_and(|s| s.contains("scripts/stage-lsp.js")),
        "stage:lsp must run scripts/stage-lsp.js"
    );
    // `bin/` must ship inside the VSIX (the whole point of this feature).
    let ignore = fs::read_to_string(vscode_dir().join(".vscodeignore")).expect(".vscodeignore");
    assert!(
        !ignore.lines().any(|l| {
            let l = l.trim();
            l == "bin/**" || l == "bin" || l == "bin/*"
        }),
        ".vscodeignore must not exclude the bundled bin/ directory"
    );
    // Facade anti-drift: the old "downloads itself on first use" promise is
    // gone; the marketplace description now advertises the bundled server.
    let description = manifest["description"].as_str().expect("description");
    assert!(
        !description.contains("downloads itself"),
        "description must not promise a runtime download anymore"
    );
}

#[test]
fn cargo_and_extension_versions_match() {
    // Version discipline (docs/design-bundled-lsp.md §3.3): the VSIX and the
    // server it bundles are built from the same tag, so the two version fields
    // must be bumped together — caught here before a release tag is pushed.
    let cargo = fs::read_to_string(repo_root().join("Cargo.toml")).expect("Cargo.toml");
    let mut in_workspace_package = false;
    let mut cargo_version = None;
    for line in cargo.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_workspace_package = line == "[workspace.package]";
            continue;
        }
        if in_workspace_package {
            if let Some(rest) = line.strip_prefix("version") {
                let value = rest.trim_start_matches([' ', '=']).trim().trim_matches('"');
                cargo_version = Some(value.to_string());
                break;
            }
        }
    }
    let cargo_version = cargo_version.expect("workspace.package.version");
    let extension_version = manifest()["version"]
        .as_str()
        .expect("package.json version")
        .to_string();
    assert_eq!(
        cargo_version, extension_version,
        "Cargo.toml workspace version and editor/vscode/package.json must match"
    );
}
