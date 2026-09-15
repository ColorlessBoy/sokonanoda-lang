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

fn media_file(name: &str) -> String {
    fs::read_to_string(vscode_dir().join("media").join(name))
        .unwrap_or_else(|_| panic!("media/{name} exists"))
}

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root exists")
}

#[test]
fn restart_server_re_resolves_and_never_keeps_a_stale_command() {
    // `sokonanoda: restart server` must go through the same version-aware
    // resolution as activation (including the version-pinned download
    // fallback), and must bail out instead of silently restarting the old
    // server when re-resolution yields nothing.
    let script = entry_script();
    assert!(
        script.contains("resolveServerForStart"),
        "restart must share the activation resolver (no stale-cache reuse)"
    );
    assert!(
        script.contains("if (next === undefined)"),
        "restart must abort when no usable server is resolved"
    );
    assert!(
        script.contains("downloadLspBinary"),
        "the shared resolver must keep the version-pinned download fallback"
    );
    // Extension-code upgrades need a window reload; the command must surface
    // that instead of pretending the restart fixed it.
    assert!(
        script.contains("newestInstalledExtensionVersion"),
        "restart must detect a newer installed extension and ask for a reload"
    );
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
    // Cursor goal view (docs/design/by-tactics.md §6): the tree's
    // 「当前光标处」 group polls state at the caret on selection changes.
    assert!(
        script.contains("soko/stateAt"),
        "the cursor goal view must consume soko/stateAt"
    );
    assert!(
        script.contains("onDidChangeTextEditorSelection"),
        "the cursor goal view must track selection changes"
    );
    // Multi-goal display (docs/design/goal-list.md): both goal views must
    // render the server's full open-goal list, not just the current goal.
    assert!(
        script.contains("cursor.goals") && script.contains("decl.goals"),
        "the goal views must render every open goal from the server (goals), not only one"
    );
    // Cursor-move performance (docs/design/goal-list.md): caret movement must
    // not refetch `soko/goals` / rebuild the exercise nodes — the client
    // caches the declaration TreeItems and refreshes only the cursor group.
    assert!(
        script.contains("declItems") && script.contains("refreshCursor"),
        "cursor movement must reuse cached declaration items (no soko/goals per caret move)"
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
    // the client must never re-derive unit status (docs/design/course-status.md
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
    // Teaching rule (docs/design/hints-suggestions.md §0.2): the UI must never
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
fn manifest_disables_confusable_unicode_highlight_for_the_language() {
    // 教学语言用希腊字母（α/β/γ）作 binder 名；VS Code 的 Trojan-Source
    // 混淆字符高亮（editor.unicodeHighlight.ambiguousCharacters，默认开）
    // 会给 α 画框。内置的 plaintext/markdown 已用语言级默认关掉它；
    // 本扩展对 [sokonanoda] 做同样的事（只影响本语言文件，不动全局设置）。
    let manifest = manifest();
    assert_eq!(
        manifest["contributes"]["configurationDefaults"]["[sokonanoda]"]
            ["editor.unicodeHighlight.ambiguousCharacters"],
        Value::Bool(false),
        "[sokonanoda] must default the confusable-character box off"
    );
}

#[test]
fn server_acquisition_prefers_the_bundled_binary() {
    // Bundled-LSP contract (docs/design/bundled-lsp.md §3.2): the extension
    // wires acquisition through server.js, which resolves the binary shipped
    // in the VSIX before any workspace build or download.
    let script = entry_script();
    assert!(
        script.contains("require(\"./server\")"),
        "extension.js must delegate server acquisition to server.js"
    );
    let server = server_script();
    for target in [
        "darwin-arm64",
        "darwin-x64",
        "linux-x64",
        "linux-arm64",
        "alpine-x64",
        "alpine-arm64",
        "win32-x64",
        "win32-arm64",
    ] {
        assert!(
            server.contains(target),
            "server.js must map the {target} platform to its bundled target directory"
        );
    }
    assert!(
        server.contains("/etc/alpine-release"),
        "server.js must detect Alpine like VS Code does (alpine-* packages)"
    );
    // The CLI is bundled too (course map works with nothing installed).
    assert!(
        server.contains("resolveCliCommand") && server.contains("sokonanoda.exe"),
        "server.js must resolve the bundled sokonanoda CLI (incl. the .exe name)"
    );
    assert!(
        server.contains("\"bin\"") && server.contains("chmodSync") && server.contains("0o755"),
        "server.js must resolve bin/<target>/ and repair a lost executable bit"
    );
    // Version-skew regression (docs/design/bundled-lsp.md §0.5): the fallback
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
fn bundled_first_policy_and_doctor_are_wired() {
    // Server policy (docs/design/extension-server-policy.md §2/§3/§5): the
    // default is bundled-first with an explicit `serverOverride` opt-in, and a
    // read-only `sokonanoda.doctor` self-check covers six checks.
    let manifest = manifest();
    let script = entry_script();
    let server = server_script();

    // New boolean setting, default off.
    let prop = &manifest["contributes"]["configuration"]["properties"]["sokonanoda.serverOverride"];
    assert_eq!(
        prop["type"].as_str(),
        Some("boolean"),
        "sokonanoda.serverOverride must be a boolean setting"
    );
    assert_eq!(
        prop["default"],
        Value::Bool(false),
        "sokonanoda.serverOverride must default to false (bundled-first)"
    );
    // The setting must be restricted in untrusted workspaces (it re-enables
    // workspace `target/` builds = running untrusted code).
    let restricted = manifest["capabilities"]["untrustedWorkspaces"]["restrictedConfigurations"]
        .as_array()
        .expect("restrictedConfigurations");
    assert!(
        restricted
            .iter()
            .any(|v| v.as_str() == Some("sokonanoda.serverOverride")),
        "sokonanoda.serverOverride must be a restricted configuration"
    );

    // `sokonanoda.doctor`: declared and registered.
    let commands = manifest["contributes"]["commands"]
        .as_array()
        .expect("contributes.commands");
    assert!(
        commands
            .iter()
            .any(|c| c["command"].as_str() == Some("sokonanoda.doctor")),
        "package.json must declare sokonanoda.doctor"
    );
    assert!(
        script.contains("\"sokonanoda.doctor\""),
        "extension.js must register sokonanoda.doctor"
    );

    // Bundled-first default: server.js consults setting/env only after the
    // explicit `override` gate (the old `setting → env → bundled` chain is
    // gone), and reports a `source`.
    let override_gate = server
        .find("if (override)")
        .expect("server.js must gate the explicit override behind `if (override)`");
    let setting_branch = server
        .find("typeof setting === \"string\"")
        .expect("server.js must keep the setting path");
    assert!(
        override_gate < setting_branch,
        "the setting/env paths must only run under the override gate (bundled-first default)"
    );
    assert!(
        server.contains("source: \"bundled\"") && server.contains("source: \"cache\""),
        "server.js must report the resolution source (bundled/cache)"
    );
    assert!(
        script.contains("serverOverride") && script.contains("source="),
        "extension.js must read serverOverride and include source= in receipts/doctor"
    );

    // Doctor covers all six checks (design §3) and is read-only.
    for check in [
        "resolution",
        "server-version",
        "host-version",
        "ignored-override",
        "download-cache",
        "obsolete-versions",
    ] {
        assert!(
            script.contains(check),
            "doctor must cover the `{check}` check"
        );
    }
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
fn release_workflow_packages_platform_specific_vsixes() {
    // Release contract (docs/design/bundled-lsp.md §3.3, docs/RELEASE.md):
    // per-target VSIXes with the server staged in, plus a universal fallback,
    // a tag/version gate, and marketplace publishing.
    let release =
        fs::read_to_string(repo_root().join(".github/workflows/release.yml")).expect("release.yml");
    for needle in [
        "package-vsix",
        "--target",
        "sokonanoda-universal.vsix",
        "Version gate",
        "stage-lsp.js",
        "vsce publish",
        // Full platform matrix + portable Linux builds.
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-musl",
        "aarch64-unknown-linux-musl",
        "aarch64-pc-windows-msvc",
        "cargo zigbuild",
        ".2.28",
        // Both binaries ship: VSIX embeds LSP + CLI; Releases carry the CLI too.
        "--cli-binary",
        "sokonanoda-cli-",
    ] {
        assert!(
            release.contains(needle),
            "release.yml must contain `{needle}`"
        );
    }
    assert!(
        !release.contains("sokonanoda-vsix/sokonanoda.vsix"),
        "release.yml must not use the old single universal VSIX path"
    );
}

#[test]
fn ci_stages_the_bundled_server_for_integration_tests() {
    // CI contract: integration tests run against the VSIX layout (bundled
    // resolution) and the node unit tests gate every push.
    let ci = fs::read_to_string(repo_root().join(".github/workflows/ci.yml")).expect("ci.yml");
    assert!(
        ci.contains("npm run test:unit"),
        "ci.yml must run npm run test:unit"
    );
    assert!(
        ci.contains("stage-lsp.js"),
        "ci.yml must stage the bundled server before integration tests"
    );
    assert!(
        ci.contains("Package host VSIX"),
        "ci.yml must smoke-package a platform VSIX"
    );
}

#[test]
fn infoview_view_and_command_are_consistent() {
    // Infoview webview (docs/design/webview-infoview.md, 方案 B): package.json
    // declares a webview view in the explorer container next to 「练习」/「课程」
    // and a command to reveal it; extension.js registers a provider for exactly
    // that view id and the command itself.
    let manifest = manifest();
    let script = entry_script();
    let views = manifest["contributes"]["views"]["explorer"]
        .as_array()
        .expect("explorer views");
    let view = views
        .iter()
        .find(|v| v["id"].as_str() == Some("sokonanoda.infoview"))
        .expect("package.json must declare the sokonanoda.infoview view");
    assert_eq!(
        view["type"].as_str(),
        Some("webview"),
        "sokonanoda.infoview must be a webview view"
    );
    let commands = manifest["contributes"]["commands"]
        .as_array()
        .expect("contributes.commands");
    assert!(
        commands
            .iter()
            .any(|c| c["command"].as_str() == Some("sokonanoda.openInfoview")),
        "package.json must declare sokonanoda.openInfoview"
    );
    assert!(
        script.contains("\"sokonanoda.infoview\"")
            && script.contains("registerWebviewViewProvider"),
        "extension.js must register a WebviewViewProvider for sokonanoda.infoview"
    );
    assert!(
        script.contains("\"sokonanoda.openInfoview\""),
        "extension.js must register sokonanoda.openInfoview"
    );
}

#[test]
fn infoview_assets_exist_and_are_referenced() {
    // The webview loads only from media/ (localResourceRoots); the HTML is a
    // template whose CSP nonce/style/script URIs are filled per load.
    let script = entry_script();
    for asset in ["infoview.js", "infoview.css", "infoview.html"] {
        assert!(
            vscode_dir().join("media").join(asset).exists(),
            "media/{asset} must exist (referenced by the provider)"
        );
        assert!(
            script.contains(asset),
            "extension.js must reference media/{asset}"
        );
    }
    let html = media_file("infoview.html");
    for needle in [
        "default-src 'none'",
        "script-src 'nonce-{{nonce}}'",
        "{{scriptUri}}",
        "{{styleUri}}",
        "{{cspSource}}",
    ] {
        assert!(
            html.contains(needle),
            "media/infoview.html must declare the CSP/nonce contract: `{needle}`"
        );
    }
    assert!(
        script.contains("localResourceRoots") && script.contains("enableScripts"),
        "the webview must enable scripts with localResourceRoots = media/"
    );
    assert!(
        script.contains("nonce") && script.contains("randomBytes"),
        "the CSP nonce must be generated randomly per load"
    );
}

#[test]
fn infoview_webview_is_hardened() {
    // Untrusted-text discipline (docs/design/webview-infoview.md §4): no
    // innerHTML, no remote assets, no release/latest, no inline handlers.
    let script = media_file("infoview.js");
    assert!(
        !script.contains("innerHTML"),
        "the webview script must render with textContent only (no innerHTML)"
    );
    assert!(
        !script.contains("http://") && !script.contains("https://"),
        "the webview script must not reference remote assets"
    );
    assert!(
        !script.contains("/latest/") && !script.contains("releases/latest"),
        "the webview must not reference a release `latest` URL"
    );
    let html = media_file("infoview.html");
    assert!(
        !html.contains("http://") && !html.contains("https://") && !html.contains("onclick="),
        "the webview HTML must not carry remote URLs or inline event attributes"
    );
    // Stale snapshots are dropped by document version (the host's own
    // cursorRequestSeq discipline, mirrored in the webview).
    assert!(
        script.contains("lastVersion") && script.contains("version"),
        "the webview must drop stale state snapshots by version"
    );
}

#[test]
fn infoview_message_protocol_matches_design() {
    // Protocol table (docs/design/webview-infoview.md §3): host → webview
    // state/decls/server/theme; webview → host ready/reveal/focusExercise.
    // Cursor moves post only `state` (decls follows diagnostics/file change),
    // the view releases hidden context, and the client still consumes
    // soko/stateAt / soko/version.
    let script = entry_script();
    let webview = media_file("infoview.js");
    for needle in [
        "INFOVIEW_PROTOCOL",
        "\"state\"",
        "\"decls\"",
        "\"server\"",
        "\"theme\"",
        "\"ready\"",
        "\"reveal\"",
        "\"focusExercise\"",
    ] {
        assert!(
            script.contains(needle),
            "extension.js must implement the Infoview message {needle}"
        );
    }
    for needle in [
        "\"state\"",
        "\"decls\"",
        "\"server\"",
        "\"theme\"",
        "\"ready\"",
        "\"reveal\"",
        "\"focusExercise\"",
    ] {
        assert!(
            webview.contains(needle),
            "media/infoview.js must handle the Infoview message {needle}"
        );
    }
    assert!(
        script.contains("retainContextWhenHidden: false"),
        "the Infoview must release hidden context (retainContextWhenHidden: false)"
    );
    assert!(
        script.contains("soko/stateAt") && script.contains("soko/version"),
        "the host must feed the webview from soko/stateAt / soko/version"
    );
}

#[test]
fn cargo_and_extension_versions_match() {
    // Version discipline (docs/design/bundled-lsp.md §3.3): the VSIX and the
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
