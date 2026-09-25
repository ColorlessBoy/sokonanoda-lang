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
    // `Sokonanoda: Restart Server (重启服务器)` must go through the same version-aware
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

    // ── 清单 v2：卷 → 章 → 单元（台账 G-07，设计 course-manifest-v2.md §4.3）──
    // 客户端只吃 `course.unit` 事件：事件里带 `volume`/`chapter` 时按卷/章分组，
    // 不带时**必须**保持 v1 的平铺路径（向后兼容是硬要求，不许回归）。
    assert!(
        script.contains("unit.volume") && script.contains("unit.chapter"),
        "the course tree must read the v2 `volume`/`chapter` fields off course.unit events"
    );
    assert!(
        script.contains("courseHasVolumes") && script.contains("courseVolumeGroups"),
        "the course tree must switch to volume→chapter→unit grouping when the events carry volumes"
    );
    assert!(
        script.contains("courseVolumeItem") && script.contains("courseChapterItem"),
        "the course tree must render volume and chapter nodes"
    );
    // v1 平铺路径逐字保留：`courseUnitItem` 仍在，且单元节点仍是叶子（None）。
    assert!(
        script.contains("courseUnitItem") && script.contains("TreeItemCollapsibleState.None"),
        "the v1 flat path must stay (courseUnitItem + leaf units)"
    );
    assert!(
        script.contains("courseErrorItem"),
        "v2 grouping must keep unreadable units visible (fallback group)"
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
fn marketplace_description_fits_the_gallery_limit() {
    // The Marketplace gallery renders `description` as the short blurb under the
    // extension name and hard-truncates it at 300 characters — mid-word, with no
    // ellipsis. Keep it a complete sentence under the limit; the long pitch
    // belongs in README.md (which the gallery renders in full). Regression guard
    // for the shipped 0.39.1 description, which was cut at "...(Claude Code / ope".
    const GALLERY_LIMIT: usize = 300;
    let manifest = manifest();
    let description = manifest["description"]
        .as_str()
        .expect("package.json must declare a string description");
    let len = description.chars().count();
    assert!(
        len <= GALLERY_LIMIT,
        "Marketplace truncates `description` at {GALLERY_LIMIT} chars (got {len}); \
         move the detail into editor/vscode/README.md:\n{description}"
    );
    assert!(
        !description.ends_with(' ') && description.ends_with('.'),
        "`description` must end on a complete sentence: {description:?}"
    );
    assert!(
        description.is_ascii(),
        "`description` must stay ASCII so every gallery surface renders it the same: {description:?}"
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
        // Supply-chain integrity (docs/RELEASE.md §6, 0.35.1): a checksum
        // manifest plus SLSA build-provenance attestations over the assets.
        "SHA256SUMS",
        "actions/attest-build-provenance@v2",
        "attestations: write",
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
fn unit_test_script_covers_every_node_layer() {
    // `npm run test:unit` 是扩展在**没有 Electron**的情况下唯一能跑的行为测试层：
    // server 解析 / 下载回退 / webview 渲染 / 扩展宿主接线（stub host）。
    // 少一个就会被 CI 静默放过——这里把清单钉死。
    let manifest = manifest();
    let script = manifest["scripts"]["test:unit"]
        .as_str()
        .expect("package.json declares scripts.test:unit");
    for file in [
        "test-server.js",
        "test-download.js",
        "test-webview.js",
        "test-extension-host.js",
    ] {
        assert!(
            script.contains(file),
            "scripts.test:unit must run {file}: {script}"
        );
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../editor/vscode")
            .join(file);
        assert!(path.exists(), "{file} must exist at {}", path.display());
    }
}

#[test]
fn infoview_view_and_command_are_consistent() {
    // Infoview webview (docs/design/goal-rendering.md §2.3): package.json
    // declares a webview view inside the `sokonanoda` secondary-side-bar
    // container (right dock) and a command to reveal it; extension.js registers
    // a provider for exactly that view id and the command itself.
    let manifest = manifest();
    let script = entry_script();
    let containers = manifest["contributes"]["viewsContainers"]["secondarySidebar"]
        .as_array()
        .expect("contributes.viewsContainers.secondarySidebar (lowercase b)");
    assert!(
        containers
            .iter()
            .any(|c| c["id"].as_str() == Some("sokonanoda")),
        "the Infoview container must live in the secondary (right) side bar"
    );
    let views = manifest["contributes"]["views"]["sokonanoda"]
        .as_array()
        .expect("the sokonanoda container's views");
    let view = views
        .iter()
        .find(|v| v["id"].as_str() == Some("sokonanoda.infoview"))
        .expect("package.json must declare the sokonanoda.infoview view");
    assert_eq!(
        view["type"].as_str(),
        Some("webview"),
        "sokonanoda.infoview must be a webview view"
    );
    // HideIfEmpty regression: a `when` clause meant the container had 0 visible
    // views without an active `.sokonanoda` editor, so VS Code hid it and the
    // focus commands did nothing. The view must have no `when` and be visible.
    assert!(
        view.get("when").is_none(),
        "sokonanoda.infoview must not carry a `when` (hideIfEmpty would hide the panel)"
    );
    assert_eq!(
        view["visibility"].as_str(),
        Some("visible"),
        "sokonanoda.infoview must be marked `visibility: visible`"
    );
    // Activation: the panel comes up even with no `.sokonanoda` editor open.
    let activation = manifest["activationEvents"]
        .as_array()
        .expect("activationEvents");
    assert!(
        activation
            .iter()
            .any(|event| event.as_str() == Some("onView:sokonanoda.infoview")),
        "activationEvents must include onView:sokonanoda.infoview"
    );
    assert!(
        activation
            .iter()
            .any(|event| event.as_str() == Some("onLanguage:sokonanoda")),
        "activationEvents must keep onLanguage:sokonanoda"
    );
    assert!(
        manifest["contributes"]["views"]["explorer"]
            .as_array()
            .expect("explorer views")
            .iter()
            .all(|v| v["id"].as_str() != Some("sokonanoda.infoview")),
        "the Infoview must no longer sit in the explorer container"
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
        script.contains("workbench.view.extension.sokonanoda"),
        "sokonanoda.openInfoview must reveal the sokonanoda container"
    );
    assert!(
        script.contains("\"sokonanoda.openInfoview\""),
        "extension.js must register sokonanoda.openInfoview"
    );
}

#[test]
fn infoview_opens_without_a_handshake_or_a_confusing_error() {
    // docs/design/goal-rendering.md §2.4: opening the panel focuses the view and
    // it renders on demand; the old 2 s readiness handshake plus the
    // 「目标面板 (Infoview) 暂时不可用…」 warning is gone for good.
    let script = entry_script();
    assert!(
        !script.contains("暂时不可用"),
        "the confusing temporary-unavailable warning must not come back"
    );
    assert!(
        !script.contains("waitReady") && !script.contains("INFOVIEW_READY_TIMEOUT_MS"),
        "the Infoview must not gate opening on a readiness handshake"
    );
}

#[test]
fn secondary_sidebar_container_requires_the_engine_bump() {
    // `viewsContainers.secondarySidebar` needs VS Code >= 1.106: 1.104/1.105
    // required the `contribSecondarySidebar` proposed API, and older versions
    // silently ignore the key — the view would simply disappear.
    // docs/design/goal-rendering.md §2.3.
    let manifest = manifest();
    let engine = manifest["engines"]["vscode"]
        .as_str()
        .expect("engines.vscode");
    let vscode_minor = |version: &str| -> u32 {
        version
            .trim_start_matches('^')
            .split('.')
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| panic!("{version:?} must be ^<major>.<minor>.<patch>"))
    };
    assert!(
        vscode_minor(engine) >= 106,
        "secondarySidebar containers require VS Code >=1.106 (got {engine:?})"
    );
    let types = manifest["devDependencies"]["@types/vscode"]
        .as_str()
        .expect("@types/vscode");
    assert!(
        vscode_minor(types) >= 106,
        "@types/vscode must follow the engine bump (got {types:?})"
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
fn infoview_colours_every_semantic_kind_from_the_single_source() {
    // docs/design/goal-rendering.md §2.2: the webview renders the server's
    // semantic runs (never re-tokenizing goal text), so every
    // `front::semantic::SemanticKind` wire name must have a `.tok-<kind>` rule
    // — a new kind that lacks one would silently render uncoloured and let the
    // Infoview drift from hover.
    let webview = media_file("infoview.js");
    assert!(
        webview.contains("goal_runs") && webview.contains("ty_runs"),
        "media/infoview.js must render the server's semantic runs"
    );
    assert!(
        webview.contains("\"tok tok-\""),
        "media/infoview.js must emit `tok tok-<kind>` spans for classified runs"
    );
    let css = media_file("infoview.css");
    for kind in sokonanoda_front::semantic::SemanticKind::ALL {
        let class = format!(".tok-{}", kind.as_str());
        assert!(
            css.contains(&class),
            "media/infoview.css must colour `{class}` (front::semantic is the single source)"
        );
    }
}

#[test]
fn rendered_language_text_uses_the_sokonanoda_fence() {
    // docs/design/goal-rendering.md §7: the extension renders `.sokonanoda` text
    // (tree tooltips) with the {sokonanoda} language id, so it is highlighted by
    // the same TM grammar as hover/completion/Infoview.
    let script = entry_script();
    assert!(
        script.contains("appendCodeblock"),
        "the extension must render language text as code blocks"
    );
    assert!(
        script.contains("appendCodeblock(String(text ?? \"\"), \"sokonanoda\")"),
        "code blocks must use the sokonanoda language id"
    );
}

#[test]
fn infoview_declaration_list_shows_types_and_line_hints() {
    // The declaration list renders each declaration's type as a small,
    // syntax-coloured hint (same runs as the goal state) plus a 1-based line
    // hint (`L12`, from `range.start.line`). Rows are read-only now: no click,
    // no `focusExercise` postMessage and no host-side jump plumbing (jumping is
    // the tree's job), so a row can never be mistaken for a button.
    let webview = media_file("infoview.js");
    assert!(
        webview.contains("ty_runs") && webview.contains("decl-ty"),
        "the declaration list must render the type hint"
    );
    // Goal line starts with `⊢` (user preference) and the type line wraps.
    assert!(
        webview.contains("\"⊢ \""),
        "the Infoview goal line must start with `⊢ `"
    );
    assert!(
        webview.contains("range.start.line") && webview.contains("\"L\" + (line + 1)"),
        "the declaration row must show a 1-based line hint derived from range.start.line"
    );
    assert!(
        webview.contains("decl-line-hint"),
        "the line hint must carry its own dedicated CSS class"
    );
    assert!(
        !webview.contains("focusExercise") && !webview.contains("declsUri"),
        "declaration rows must be non-interactive (no focusExercise jump plumbing)"
    );
    let css = media_file("infoview.css");
    assert!(
        css.contains(".decl-ty") && css.contains("white-space: pre-wrap"),
        "the type hint must wrap (it was truncated before)"
    );
    assert!(
        css.contains(".decl-line-hint"),
        "the line hint must be styled small/dim via .decl-line-hint"
    );
    assert!(
        !css.contains("text-overflow: ellipsis"),
        "the type hint must no longer be ellipsised"
    );
    let script = entry_script();
    assert!(
        !script.contains("jumpToRange") && !script.contains("focusExercise"),
        "the host must not keep the removed click-to-jump plumbing"
    );
}

#[test]
fn infoview_message_protocol_matches_design() {
    // Protocol table (docs/design/webview-infoview.md §3): host → webview
    // state/decls/status/server/theme; webview → host ready/reveal.
    // Cursor moves post only `state` (decls follows diagnostics/file change),
    // the view releases hidden context, and the client still consumes
    // soko/stateAt / soko/version.
    let script = entry_script();
    let webview = media_file("infoview.js");
    for needle in [
        "INFOVIEW_PROTOCOL",
        "\"state\"",
        "\"decls\"",
        "\"status\"",
        "\"server\"",
        "\"theme\"",
        "\"ready\"",
        "\"reveal\"",
    ] {
        assert!(
            script.contains(needle),
            "extension.js must implement the Infoview message {needle}"
        );
    }
    for needle in [
        "\"state\"",
        "\"decls\"",
        "\"status\"",
        "\"server\"",
        "\"theme\"",
        "\"ready\"",
        "\"reveal\"",
    ] {
        assert!(
            webview.contains(needle),
            "media/infoview.js must handle the Infoview message {needle}"
        );
    }
    assert!(
        !script.contains("focusExercise") && !webview.contains("focusExercise"),
        "the click-to-jump focusExercise message must be gone from both sides"
    );
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

fn alternation_words(grammar: &Value, rule: &str) -> Vec<String> {
    let pattern = grammar["repository"][rule]["match"]
        .as_str()
        .unwrap_or_else(|| panic!("syntaxes/sokonanoda.tmLanguage.json: repository.{rule}.match"));
    let mut words = Vec::new();
    for group in pattern.split('(').skip(1) {
        let inner = group.split(')').next().unwrap_or("");
        for word in inner.split('|') {
            let word = word.trim();
            if !word.is_empty()
                && word
                    .chars()
                    .all(|c| c.is_alphanumeric() || matches!(c, '_' | '#' | '∀'))
            {
                words.push(word.to_string());
            }
        }
    }
    words
}

fn collect_scope_names(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                if key == "name" {
                    if let Some(scope) = val.as_str() {
                        out.push(scope.to_string());
                    }
                } else {
                    collect_scope_names(val, out);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_scope_names(item, out);
            }
        }
        _ => {}
    }
}

#[test]
fn tm_grammar_declares_every_semantic_scope() {
    // docs/design/goal-rendering.md §2.2/§7: `front::semantic` is the single
    // classification source, so the TM grammar (used by hover/Infoview fences)
    // must actually emit every canonical `SemanticKind::tm_scope()`. A kind
    // whose scope has no rule would render uncoloured in fences while the
    // editor's semantic tokens coloured it — silent drift, caught here.
    let raw = fs::read_to_string(
        vscode_dir()
            .join("syntaxes")
            .join("sokonanoda.tmLanguage.json"),
    )
    .expect("syntaxes/sokonanoda.tmLanguage.json");
    let grammar: Value = serde_json::from_str(&raw).expect("grammar is valid JSON");

    let mut names = Vec::new();
    collect_scope_names(&grammar, &mut names);

    for kind in sokonanoda_front::semantic::SemanticKind::ALL {
        let scope = kind.tm_scope();
        assert!(
            names.iter().any(|name| name == scope),
            "syntaxes/sokonanoda.tmLanguage.json must emit `{scope}` for \
             SemanticKind::{kind:?} (front::semantic is the single source)"
        );
    }
}

#[test]
fn tm_grammar_keywords_follow_the_single_source() {
    // docs/design/goal-rendering.md §2.2/§7: hover colours its code fence with
    // the TextMate grammar while the editor's semantic tokens and the Infoview
    // colour from `front::semantic`. To keep them from drifting, the grammar's
    // word lists (keywords + `#` commands + sorts + `forall`) must equal that
    // single source exactly — no missing new keyword, no stale one (`Nat` used
    // to be hard-coded here).
    let raw = fs::read_to_string(
        vscode_dir()
            .join("syntaxes")
            .join("sokonanoda.tmLanguage.json"),
    )
    .expect("syntaxes/sokonanoda.tmLanguage.json");
    let grammar: Value = serde_json::from_str(&raw).expect("grammar is valid JSON");

    let mut actual = alternation_words(&grammar, "keywords");
    actual.extend(alternation_words(&grammar, "commands"));
    actual.extend(alternation_words(&grammar, "sorts"));
    actual.sort();
    actual.dedup();

    let mut expected: Vec<String> = sokonanoda_front::semantic::keywords()
        .iter()
        .chain(sokonanoda_front::semantic::sorts())
        .map(|word| word.to_string())
        .collect();
    // `forall`/`∀` are keywords via the lexer's `Forall` token, not the
    // `KEYWORDS` table (front `semantic.rs`).
    expected.push("forall".to_string());
    expected.push("∀".to_string());
    expected.sort();
    expected.dedup();

    assert_eq!(
        actual, expected,
        "the TM grammar word lists must mirror front::semantic (keywords + sorts + forall)"
    );
}

#[test]
fn tm_grammar_enriches_declarations_constructors_and_variables() {
    // Hover highlighting (docs/design/goal-rendering.md §7): beyond the pinned
    // keyword/sort word lists, the grammar must enrich declaration names,
    // constructors/constants, holes/operators and variables so a fenced
    // `sokonanoda` block is not flat. This is the "hover looks like the editor"
    // half of the single source; the word-list equality is pinned separately.
    let raw = fs::read_to_string(
        vscode_dir()
            .join("syntaxes")
            .join("sokonanoda.tmLanguage.json"),
    )
    .expect("syntaxes/sokonanoda.tmLanguage.json");
    let grammar: Value = serde_json::from_str(&raw).expect("grammar is valid JSON");
    for rule in [
        "declarations",
        "constructors",
        "constants",
        "variables",
        "operators",
        "holes",
    ] {
        assert!(
            grammar["repository"][rule].is_object(),
            "grammar repository must define the enriched `{rule}` rule"
        );
    }
    assert!(
        grammar["repository"]["declarations"]["captures"].is_object(),
        "the `declarations` rule must carry captures (keyword + entity name)"
    );
}

#[test]
fn infoview_provider_is_registered_before_the_server_resolution_await() {
    // Root cause of the blank/late Infoview: `activate` used to `await
    // resolveServerForStart` (filesystem probing, and a download on unbundled
    // platforms) *before* `registerWebviewViewProvider`. VS Code registers
    // extension view containers with `hideIfEmpty`, so with no provider the
    // panel was blank/absent until a side-bar toggle re-triggered resolution
    // and the late registration made it "suddenly appear". Registration must be
    // synchronous and textually precede the first `await resolveServerForStart`.
    let script = entry_script();
    let start = script
        .find("async function activate(context)")
        .expect("extension.js must define activate");
    let body = &script[start..];
    let register = body
        .find("registerWebviewViewProvider")
        .expect("activate must register the Infoview provider");
    let resolve = body
        .find("await resolveServerForStart")
        .expect("the async continuation must still resolve the server");
    assert!(
        register < resolve,
        "registerWebviewViewProvider must run before `await resolveServerForStart`"
    );
    // The slow work must be a fire-and-forget continuation, never blocking the
    // view: the resolver lives inside an async continuation with a `.catch`.
    assert!(
        body.contains("})().catch"),
        "activation must fire-and-forget the async server startup with a .catch"
    );
}

#[test]
fn open_infoview_reveals_the_auxiliary_bar_then_container_then_view() {
    // A view whose container is hidden cannot be focused: `openInfoview` must
    // first show the auxiliary bar (`hideIfEmpty` containers need it), then
    // reveal the `sokonanoda` container, then focus the view — in that order
    // (docs/design/goal-rendering.md §2.3).
    let script = entry_script();
    let start = script
        .find("async function openInfoview()")
        .expect("extension.js must define openInfoview");
    let body = &script[start..];
    let aux = body
        .find("workbench.action.focusAuxiliaryBar")
        .expect("openInfoview must show the auxiliary bar first");
    let container = body
        .find("workbench.view.extension.sokonanoda")
        .expect("openInfoview must reveal the sokonanoda container");
    let view = body
        .find("sokonanoda.infoview.focus")
        .expect("openInfoview must focus the Infoview view");
    assert!(
        aux < container && container < view,
        "openInfoview must issue focusAuxiliaryBar -> view.extension.sokonanoda -> infoview.focus, in order"
    );
}

#[test]
fn infoview_webview_renders_a_status_skeleton_and_handles_status() {
    // Feedback UX: the webview must never be a silent blank. It renders a
    // skeleton on load and consumes the host `status` message for progress.
    let webview = media_file("infoview.js");
    for needle in ["正在渲染…", "等待编译…"] {
        assert!(
            webview.contains(needle),
            "media/infoview.js must render the on-load skeleton placeholder `{needle}`"
        );
    }
    for needle in ["\"status\"", "编译中…", "已就绪", "等待 .sokonanoda 文件"] {
        assert!(
            webview.contains(needle),
            "media/infoview.js must handle `status` ({needle})"
        );
    }
    // The host posts `status`: loading when the soko/goals fetch starts, ready
    // with the declaration count when it completes.
    let script = entry_script();
    assert!(
        script.contains("\"status\"")
            && script.contains("state: \"loading\"")
            && script.contains("decls: decls.length"),
        "extension.js must post status loading at fetch start and ready with the decl count"
    );
    assert!(
        script.contains("\"idle\" : \"ready\""),
        "extension.js must distinguish the no-document `idle` state from `ready`"
    );
}

/// Strip `/* … */` blocks so the selector/declaration parsing below never trips
/// over prose in the stylesheet's comments.
fn strip_css_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        match rest[start + 2..].find("*/") {
            Some(end) => rest = &rest[start + 2 + end + 2..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// `(selector, declarations)` pairs, in source order, from a comment-free
/// stylesheet. One level of nesting is enough for this contract test.
fn css_blocks(css: &str) -> Vec<(String, String)> {
    let mut blocks = Vec::new();
    let mut rest = css;
    while let Some(open) = rest.find('{') {
        let selector = rest[..open].trim().to_string();
        let after = &rest[open + 1..];
        let close = after.find('}').expect("every CSS block is closed");
        blocks.push((selector, after[..close].to_string()));
        rest = &after[close + 1..];
    }
    blocks
}

#[test]
fn infoview_palette_colours_every_kind_with_a_guaranteed_fallback() {
    // VS Code never exposes editor token colours to webviews, so the Infoview
    // owns a fixed `--soko-*` palette (docs/design/highlighting.md §2.1/§4).
    // Contract: every kind's `.tok-*` rule reads a `--soko-*` variable, and
    // every referenced variable is defined for all four theme kinds.
    let css = strip_css_comments(&media_file("infoview.css"));
    let blocks = css_blocks(&css);

    // (a) every wire kind has a `.tok-<kind>` rule coloured by `--soko-*`.
    let mut referenced: Vec<String> = Vec::new();
    for kind in sokonanoda_front::semantic::SemanticKind::ALL {
        let class = format!(".tok-{}", kind.as_str());
        let mut colour = None;
        for (selector, body) in &blocks {
            if !selector.split(',').any(|part| part.trim() == class) {
                continue;
            }
            for declaration in body.split(';') {
                let Some(value) = declaration.trim().strip_prefix("color:") else {
                    continue;
                };
                colour = Some(value.trim().to_string());
            }
        }
        let value = colour.unwrap_or_else(|| {
            panic!("media/infoview.css must colour `{class}` (SemanticKind::{kind:?})")
        });
        let variable = value
            .strip_prefix("var(")
            .and_then(|rest| rest.split(')').next())
            .filter(|name| name.starts_with("--soko-"))
            .unwrap_or_else(|| {
                panic!(
                    "media/infoview.css: `{class}` (SemanticKind::{kind:?}) colour \
                     `{value}` must be a `--soko-*` variable"
                )
            })
            .to_string();
        if !referenced.contains(&variable) {
            referenced.push(variable);
        }
    }

    // (b) every referenced `--soko-*` variable is defined for all four kinds.
    for theme in ["dark", "light", "high-contrast", "high-contrast-light"] {
        let selector = format!("body[data-theme=\"{theme}\"]");
        let body = &blocks
            .iter()
            .find(|(sel, _)| sel == &selector)
            .unwrap_or_else(|| {
                panic!("media/infoview.css must define the palette selector `{selector}`")
            })
            .1;
        for variable in &referenced {
            assert!(
                body.contains(&format!("{variable}:")),
                "media/infoview.css: `{selector}` must define `{variable}` so every \
                 kind resolves under `{theme}`"
            );
        }
    }
}

#[test]
fn build_and_rebuild_commands_warm_the_compile_cache() {
    // 用户报的缺口：CLI 有 `sokonanoda build [--clean]`（预热/清理共享编译缓存），
    // 但扩展从来没把它接出来。契约：两个命令都声明 + 都注册；`rebuild` 必须先
    // `--clean` 再 build（"从头重编译一遍"），两条都走 `--json` 事件流并把
    // `build.summary` 渲染成人话；子进程纪律与课程树同款（超时 + kill）。
    let manifest = manifest();
    let script = entry_script();
    let commands = manifest["contributes"]["commands"]
        .as_array()
        .expect("contributes.commands");
    for id in ["sokonanoda.build", "sokonanoda.rebuild"] {
        assert!(
            commands.iter().any(|c| c["command"].as_str() == Some(id)),
            "package.json must declare {id}"
        );
        assert!(
            script.contains(&format!("\"{id}\"")),
            "extension.js must register {id}"
        );
    }
    // 命令面板标题里带 build/rebuild 字样（用户是照这个名字找的）。
    //
    // **大小写不敏感**（T-B2 / R-4）：R-4 要求命令词首字母大写 ⇒ 标题是
    // `Build (编译当前文件/工作区，预热缓存)`；原来的 `contains("build")` 会因为
    // 大写 B 而判红 ✗。找的是"这个名字还在不在"，不是它的字面大小写。
    for (id, needle) in [
        ("sokonanoda.build", "build"),
        ("sokonanoda.rebuild", "rebuild"),
    ] {
        let title = commands
            .iter()
            .find(|c| c["command"].as_str() == Some(id))
            .and_then(|c| c["title"].as_str())
            .unwrap_or_default();
        assert!(
            title.to_lowercase().contains(needle),
            "{id} 的标题必须含 {needle}（命令面板可见，大小写不敏感），实际 {title:?}"
        );
    }
    // 键位指向已声明命令（manifest 级一致性；alt+b / alt+shift+b）。
    let bindings = manifest["contributes"]["keybindings"]
        .as_array()
        .expect("keybindings");
    for (key, id) in [
        ("alt+b", "sokonanoda.build"),
        ("alt+shift+b", "sokonanoda.rebuild"),
    ] {
        assert!(
            bindings
                .iter()
                .any(|b| b["key"].as_str() == Some(key) && b["command"].as_str() == Some(id)),
            "keybinding {key} must target {id}"
        );
    }
    // rebuild = clean + build：`--clean` 必须出现，且 clean 只清缓存（CLI 语义），
    // 所以脚本要跑第二次把缓存重新预热。
    assert!(
        script.contains("\"build\"") && script.contains("\"--clean\""),
        "rebuild must invoke `build --json --clean` (cache clean) before rebuilding"
    );
    assert!(
        script.contains("build.summary") && script.contains("build.file"),
        "build must consume the CLI's JSON Lines events (build.file / build.summary)"
    );
    // 子进程纪律：有界运行 + 超时 kill（与课程树同款，绝不挂死编辑器）。
    assert!(
        script.contains("BUILD_TIMEOUT_MS") && script.contains("kill()"),
        "the build subprocess must be killed on timeout"
    );
    // 解析出的 CLI 命令必须来自共享解析器（bundled → target → PATH），
    // 不许自己拼路径（与 server.js 的版本/来源纪律一致）。
    assert!(
        script.contains("resolveCliCommand()"),
        "build must use the shared CLI resolver"
    );
}

/// **记法符号着色的防漂移**：TM 的 `mathsymbols` 类必须与
/// `front::notation_input::notation_symbol_chars` **逐字相等**。
///
/// 从前这个类是**手写的码点范围**（`U+2200–22FF` + `U+2A00–2AFF`），于是
/// `↔`(U+2194)、`¬`(U+00AC)、`𝒫`(U+1D4AB)、`ᶜ`(U+1D9C)、`×ˢ`(U+00D7/02E2)
/// **一律不着色**——课程里最常用的几个反而看不见（设计
/// `docs/design/notation-input.md` 的 R-6）。现在两边由这条测试钉在一起。
#[test]
fn tm_grammar_math_symbols_follow_the_single_source() {
    let raw = fs::read_to_string(
        vscode_dir()
            .join("syntaxes")
            .join("sokonanoda.tmLanguage.json"),
    )
    .expect("syntaxes/sokonanoda.tmLanguage.json");
    let grammar: Value = serde_json::from_str(&raw).expect("grammar is valid JSON");
    let class = grammar["repository"]["mathsymbols"]["match"]
        .as_str()
        .expect("mathsymbols.match is a string");
    // 类里只允许 `\x{XXXX}` 转义（不许码点范围：范围会静默漏掉符号）。
    let body = class
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix("]+"))
        .expect("mathsymbols is a single character class");
    let mut actual: Vec<char> = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find("\\x{") {
        let after = &rest[start + 3..];
        let end = after.find('}').expect("`\\x{` is closed");
        let code = u32::from_str_radix(&after[..end], 16).expect("hex code point");
        actual.push(char::from_u32(code).expect("valid char"));
        rest = &after[end + 1..];
    }
    assert_eq!(
        rest.trim(),
        "",
        "the math-symbol class must contain ONLY `\\x{{XXXX}}` escapes (no ranges): {body}"
    );
    actual.sort_unstable();
    actual.dedup();

    let expected = sokonanoda_front::notation_input::notation_symbol_chars();
    assert_eq!(
        actual, expected,
        "the TM grammar's math-symbol class must mirror front::notation_input"
    );
    assert!(
        !actual.contains(&'='),
        "`=` is ASCII: it belongs to the `operators` rule, not the math class"
    );
}

fn notation_input_script() -> String {
    fs::read_to_string(vscode_dir().join("src").join("abbreviation-rewriter.js"))
        .expect("editor/vscode/src/abbreviation-rewriter.js")
}

/// **记法缩写表的防漂移**（设计 `docs/design/notation-input.md` §6.2 / R-3）：
/// `editor/vscode/src/abbreviations.js` 必须与 `front::notation_input::TABLE`
/// **逐条相等**——符号、主缩写、别名、`supported`、以及**顺序**。
///
/// 与 `tm_grammar_keywords_follow_the_single_source` 同一形制：**真解析**，不做
/// grep 掩膜（硬规则 4）。JS 表被写成"一条一行、键带引号"的纯 JSON 数组字面量，
/// 就是为了这里能 `serde_json::from_str` 它：`const TABLE = [` 到行首 `];` 之间
/// 就是那段 JSON（格式即契约，改格式要同步改这条测试）。
#[test]
fn abbreviation_table_mirrors_the_single_source() {
    let raw = fs::read_to_string(vscode_dir().join("src").join("abbreviations.js"))
        .expect("editor/vscode/src/abbreviations.js");
    let start = raw
        .find("const TABLE = [")
        .expect("abbreviations.js declares `const TABLE = [`")
        + "const TABLE = ".len();
    let end = raw[start..]
        .find("\n];")
        .expect("the table literal is closed by a line-start `];`")
        + start;
    // `raw[end]` 是换行、`raw[end + 1]` 是 `]`：`raw[start..end]` 是数组体。
    // 容忍 JS 习惯的尾逗号（prettier 会加）——比对的是表，不是标点。
    let body = raw[start..end].trim_end();
    let body = body.strip_suffix(',').unwrap_or(body);
    let literal = format!("{body}\n]");
    let actual: Value = serde_json::from_str(&literal)
        .expect("the JS table is a JSON array of objects (the format is the contract)");

    let expected = Value::Array(
        sokonanoda_front::notation_input::TABLE
            .iter()
            .map(|entry| {
                serde_json::json!({
                    "symbol": entry.symbol,
                    "abbreviation": entry.abbreviation,
                    "aliases": entry.aliases,
                    "supported": entry.supported,
                })
            })
            .collect(),
    );
    assert_eq!(
        actual, expected,
        "editor/vscode/src/abbreviations.js must mirror front::notation_input::TABLE exactly \
         (same symbols, abbreviations, aliases, `supported` flags and order)"
    );
}

/// Tab 键位**绝不能吞掉普通 Tab**（设计 §9 R-2 的缓解）：它必须由扩展置位的
/// context key 把关——`when` 子句里的 key 名与 `abbreviation-rewriter.js` 里
/// 真正写的那个是同一个字符串，且 `sokonanoda.input.eager` 默认关（§8 NI-2）。
#[test]
fn notation_input_tab_binding_is_gated_by_its_context_key() {
    let manifest = manifest();
    let bindings = manifest["contributes"]["keybindings"]
        .as_array()
        .expect("contributes.keybindings");
    let binding = bindings
        .iter()
        .find(|binding| binding["command"].as_str() == Some("sokonanoda.input.replaceAbbreviation"))
        .expect("the abbreviation rewriter must be bound to a key");
    assert_eq!(binding["key"].as_str(), Some("tab"));
    let when = binding["when"]
        .as_str()
        .expect("the Tab binding must carry a `when` clause");
    assert!(
        when.contains("editorLangId == sokonanoda"),
        "the rewriter is a sokonanoda-only behaviour: {when}"
    );
    let context_key = "sokonanoda.input.abbreviationBeforeCursor";
    assert!(
        when.contains(context_key),
        "without the context key the Tab binding would swallow ordinary indentation: {when}"
    );
    assert!(
        notation_input_script().contains(&format!("\"{context_key}\"")),
        "the `when` clause must name the context key the extension actually sets"
    );

    let eager = &manifest["contributes"]["configuration"]["properties"]["sokonanoda.input.eager"];
    assert_eq!(
        eager["type"].as_str(),
        Some("boolean"),
        "the eager switch must be a documented setting"
    );
    assert_eq!(
        eager["default"].as_bool(),
        Some(false),
        "eager replacement must default to off (design §8 NI-2: Tab is the explicit path)"
    );
}

#[test]
fn command_naming_inventory_covers_every_contributed_command() {
    // **T-B1（R-4）的判据**：`docs/design/command-naming.md` §1 的盘点表必须覆盖
    // `contributes.commands` 的**每一条**命令，且**双向相等**：
    //   * 少了 ⇒ 有命令没盘点（R-4 改名时必漏）✗；
    //   * 多了 ⇒ 表里写了不存在的命令（表在说谎）✗。
    //
    // 为什么这条能长期用：改名只动**显示名**（`title`/`category`），`command` id 是
    // 协议（键位/菜单/`executeCommand`/文档/技能都在用）——一条都不动 ✓。所以表里
    // 那一列同时就是"命令清单"的契约。
    let manifest = manifest();
    let commands = manifest["contributes"]["commands"]
        .as_array()
        .expect("contributes.commands");
    let mut from_manifest: Vec<&str> = commands
        .iter()
        .map(|c| c["command"].as_str().expect("command id"))
        .collect();
    from_manifest.sort_unstable();

    let doc = fs::read_to_string(repo_root().join("docs/design/command-naming.md"))
        .expect("docs/design/command-naming.md（T-B1 的交付物）必须在仓库里");
    // 表行形如 `| 3 | \`sokonanoda.nextHole\` | …`：第二列（split 后 index 2）是 id。
    let mut from_doc: Vec<&str> = doc
        .lines()
        .filter_map(|line| {
            let cell = line.split('|').nth(2)?.trim();
            let id = cell.strip_prefix('`')?.strip_suffix('`')?;
            id.starts_with("sokonanoda.").then_some(id)
        })
        .collect();
    from_doc.sort_unstable();

    assert_eq!(
        from_doc, from_manifest,
        "盘点表与 contributes.commands 必须双向相等（左 = 表，右 = package.json）"
    );
}

#[test]
fn command_titles_follow_the_r4_naming_rule() {
    // **T-B2（R-4）的判据**：命令面板里每一行都必须是
    // `Sokonanoda: <Command> (说明)`（用户原话见 `REQUIREMENTS.md` §9 R-4，
    // 盘点表见 `docs/design/command-naming.md`）。
    //
    // 机制是 `category: "Sokonanoda"` + 纯 `title`（VS Code 用 `category: title`
    // 渲染面板行）—— 前缀**只写一处**。所以这里逐条钉四件事：
    //   ① `category` 必须是 `Sokonanoda`（大写 S；小写会被原样显示成 `sokonanoda:`）；
    //   ② `title` 里**不许**再出现包名（**前缀双写**是盘点出来的真实毛病：
    //      `openInfoview` 曾显示成 `sokonanoda: sokonanoda: 打开目标面板 (Infoview)`，
    //      `doctor` 曾显示成 `sokonanoda: doctor: 诊断服务器与版本`）；
    //   ③ `title` 形如 `<Title Case 命令词> (<说明>)`、说明含中文；
    //   ④ 括号统一**半角**（全角 `（）` 会让 15 行看起来是两种风格）。
    let manifest = manifest();
    let commands = manifest["contributes"]["commands"]
        .as_array()
        .expect("contributes.commands");
    for command in commands {
        let id = command["command"].as_str().expect("command id");
        assert_eq!(
            command["category"].as_str(),
            Some("Sokonanoda"),
            "{id} 的 category 必须是 Sokonanoda（面板靠它显示前缀）"
        );
        let title = command["title"].as_str().expect("title");
        assert!(
            !title.to_lowercase().contains("sokonanoda"),
            "{id} 的 title 不许再写包名前缀（前缀由 category 提供）：{title:?}"
        );
        assert!(
            !title.contains('（') && !title.contains('）'),
            "{id} 的 title 必须用半角括号：{title:?}"
        );
        let (name, hint) = title
            .split_once(" (")
            .unwrap_or_else(|| panic!("{id} 的 title 必须形如 `<Command> (说明)`：{title:?}"));
        let hint = hint
            .strip_suffix(')')
            .unwrap_or_else(|| panic!("{id} 的说明必须以半角 `)` 收尾：{title:?}"));
        assert!(
            name.chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
                && name
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == ' '),
            "{id} 的命令词必须首字母大写、只用 ASCII 字母/数字/空格：{name:?}"
        );
        assert!(
            hint.chars()
                .any(|ch| ('\u{4e00}'..='\u{9fff}').contains(&ch)),
            "{id} 的括号里必须是中文说明：{title:?}"
        );
    }
}
