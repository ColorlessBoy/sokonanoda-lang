# DSH: what a project repository can ship to an agent

> **本仓库的调研底稿**（第八十六轮，2026-09-17）：对 DeepSeek Harness 源码
> （`/Users/penglingwei/Documents/code-agent/deepseek-harness`）的只读勘察记录，
> 回答「一个项目仓库能自带什么、必须由用户在 `$DSH_HOME` 配置什么」。
> 结论已提炼进设计文档 **`docs/design/deepseek-harness.md`**（差距 G1–G10 与
> 计划 H0–H4）；本文保留全部 `path:line` 证据，供实现阶段逐条核对。
> 事实基于 2026-09-17 的 DSH 开发者预览版，随版本可能变化。

Source-checkout evidence (read-only; checkout verified clean). Load-bearing claims cite `path:line`;
unproven items are marked **UNVERIFIED**. `$DSH_HOME` defaults to `~/.dsh`
(`packages/util/home-paths/src/index.ts:12,18`).

**Bottom line.** A repo ships **automatically, zero user config**: **skills** (`<git-root>/.dsh/skills`,
`<git-root>/.agents/skills`) and **`AGENTS.md`/`CLAUDE.md`**. Everything else — extra skill roots, real
`ctx.commands` slash commands, custom subagent agents, client-UI plugins — needs a row in the user's
`$DSH_HOME` profile (patch file or installed npm bundle); a repo may commit those files and have the
user opt in with one flag.

## 1. Skills discovery

Lowest rank wins a duplicate name. `packages/skill/skill-filesystem/src/index.ts:36-40` defines
`PROJECT_DSH_RANK = 100`, `PROJECT_AGENTS_RANK = 200`, `CUSTOM_RANK = 300`, `USER_DSH_RANK = 400`,
`USER_AGENTS_RANK = 500`; roots are assembled at `:250-262`:

| Rank | Source | Root (cited line) |
|---|---|---|
| 100 | `project-dsh` | `<projectRoot>/.dsh/skills` (`:250`) |
| 200 | `project-agents` | `<projectRoot>/.agents/skills` (`:251`) |
| 300 | `custom` | `Config.customSkillDirs` (`:254`) |
| 400 | `user-dsh` | `<dshHome>/skills`, skips `.system` (`:257`) |
| 500 | `user-agents` | `<agentsHome>/skills` (`$DSH_AGENTS_HOME` or `~/.agents`) (`:258`) |
| 600 | `bundled` | `bundledSkillDir` / `$DSH_BUNDLED_SKILL_DIR` (`:261-262`) |

`:250` verbatim: `{ path: join(projectRoot, '.dsh/skills'), source: 'project-dsh', rank: PROJECT_DSH_RANK, projectRoot },`

**Yes — both `.dsh/skills` and `.agents/skills` are supported.** Proven with the cwd inside a
*subdirectory*, so the upward walk is exercised
(`packages/skill/skill-filesystem/tests/skill-filesystem.spec.ts:187,203,210`): the test does
`mkdir(join(project, '.git'))`, then `ctx.skills.list({ cwd: join(project, 'src') })`, then asserts the
winner came from source `'project-dsh'` — rank 100 beats 200. **`<projectRoot>` = nearest ancestor
containing `.git`**, else the supplied cwd (`index.ts:938-950` `findProjectRoot`, probing
`join(current, '.git')` upward). The cwd is the **session's** (`packages/skill/tool-skill/src/index.ts:133`:
`{ cwd: exec.agent?.session.header.cwd, signal: exec.signal, scope: exec.agent }`). No other project
marker exists; monorepo subproject selection is unsupported (`skill-filesystem/README.md:149`).

**Accepted SKILL.md frontmatter** (`index.ts:814-839`, `:1000-1010`). Required: `name`
(`^[a-z0-9]+(?:-[a-z0-9]+)*$`, `:820`) and `description` (non-empty, `:816`). Optional: `whenToUse`
(camelCase — **not** `when-to-use`, `:834`), `metadata` (arbitrary object passthrough, `:1039-1045`),
`disable-model-invocation` (default `false`, `:1004,1007`) and `user-invocable` (default `true`,
`:1005,1008`) — both kebab-case. Legacy camelCase `disableModelInvocation`/`modelInvocable`/
`userInvocable` are **rejected**, dropping the whole skill with a warning (`:1001-1003`); booleans also
accept `yes/no/on/off/1/0`, case-insensitive (`:1018-1036`). Frontmatter must start line 1 with `---` (`:917-930`).

**Directory bundles + relative reference files: supported.** Layout is `<root>/<name>/SKILL.md` or flat
`<root>/<name>.md`, one level deep only — nested `**/SKILL.md` is deliberately not discovered
(`index.ts:723-733`; `README.md:148`). Discovery sets the resource base to the **bundle directory**:
`resourceBase: { kind: 'directory', path: locator.directory }` (`index.ts:745`; `locator.directory` is the
bundle dir per `:729`). The `skill` tool renders that (`packages/skill/skill/src/index.ts:193-198`) as
`"Base directory for this skill: <path>"` plus `"Resolve relative paths mentioned by this skill against
the base directory before using them. Load referenced resources only as needed."`, wrapped by
`renderSkillContent` as `<skill_content name>` / `<skill_resources>` / `<skill_instructions>` (`:170-183`).
The DSH repo does this itself: `.agents/skills/dsh-ci-test-reliability/SKILL.md:120` links
`[the CI flake diagnosis workflow](references/ci-flake-diagnosis.md)` and that file exists;
`.agents/skills/dsh-doc/templates/` is a second example.

Caveats: (a) the **model** catalog carries only `name` + `description`, truncated to
`catalogDescriptionMaxLength` (default 500, `packages/skill/tool-skill/src/index.ts:50-58,63`); `whenToUse`
is *not* model-visible (only the human `/` menu, `packages/api/session-controller/src/skill-catalog.ts:83`),
so put routing cues in `description`. (b) Body edits need no invalidation, but **resource files below a
bundle are not catalog changes** (`skill-filesystem/README.md:81`).

## 2. Custom skill dirs

Field **`customSkillDirs: string[]`** on plugin **`@deepseek-ai/dsh-skill-filesystem`** (`index.ts:59`,
schema `:81`; `docs/config-catalog.md:2335`). Only a non-standard root such as a repo-level `skills/` needs
this; `.dsh/skills` and `.agents/skills` do not.
```yaml
# $DSH_HOME/profiles/<profile>/cordis.patch.yml   — one profile
# $DSH_HOME/cordis.patch.yml                      — every profile (outranks the per-profile file)
- id: skill-filesystem            # existing base row: packages/bundle/base/cordis.patch.yml:276-277
  config:
    customSkillDirs:
      - /absolute/path/to/repo/skills
```
**A patch replaces the targeted row's whole `config` — no deep merge** (`apps/cli/reference/README.md:9`;
`packages/boot/app-boot/README.md:169`); omitted fields fall back to schema defaults, so
`includeDefaultRoots` stays `true`. **Do not `insert` a second `dsh-skill-filesystem` row** with the
default `providerName` — duplicate provider names throw (`packages/skill/skill/src/index.ts:334-336`); for a
second provider set `providerName` + `includeDefaultRoots: false`. Relative paths are `resolve()`d against
**process cwd** (`index.ts:169`), so use absolute paths. Layer order: bundle patches → profile
`cordis.patch.yml` → `$DSH_HOME/cordis.patch.yml` → `--patch <file>` in argv order
(`apps/cli/reference/README.md:9`). A repo may commit its own patch and have the user run
`dsh web --patch <repo>/.dsh/repo.patch.yml` — relative plugin names in inserted rows resolve beside the
patch (`packages/boot/app-boot/README.md:59`). Out-of-tree plugin bundles install with
`dsh plugin --profile <name> add <package-or-git-spec>`, where "the installed package owns its dependencies
and contributes its declared `cordis.patch.yml` layer" (`apps/cli/reference/README.md:105`), resolving from
the profile's pnpm-managed `node_modules` (`:11`).

## 3. Human-invocable slash commands

**(a) Host command registry — plugin-registered only, no file discovery.** `@deepseek-ai/dsh-commands`
(`packages/interaction/commands`): API `register(definition: CommandDefinition): () => void`
(`src/index.ts:285`), name grammar `const COMMAND_NAME = /^[a-z][a-z0-9_-]*$/u` (`:32`) — **no nested
`/a/b`**. Every shipped command is an ordinary plugin: `/goal`
(`packages/goal/command-goal/src/index.ts:193`), `/compact`, `/plan`, `/export`, `/feedback`, `/permission`.
**No hardcoded command table and no markdown-command discovery** — no `.opencode/command/*.md` equivalent
and no `.dsh/commands` root in the source.
```ts
// <repo>/.dsh/plugins/hello.ts   (production runs need built artifacts: apps/cli/README.md:56)
// imports: Context from '@deepseek-ai/cordis'; CommandResult from '@deepseek-ai/dsh-commands/types'
export const name = 'hello-command'
export const inject = ['commands']
export function apply(ctx: Context): void {
  ctx.commands.register({
    name: 'hello',                      // /^[a-z][a-z0-9_-]*$/
    description: 'Say hello',
    handler: ({ rawInput }): CommandResult => ({ kind: 'success', text: `hello ${rawInput.trim() || 'world'}` }),
  })
}
```
```yaml
# <repo>/.dsh/repo.patch.yml — launch: dsh web --patch .dsh/repo.patch.yml
- insert:
    - id: hello-command
      name: ./plugins/hello.js     # patch-relative → file URL (packages/boot/app-boot/README.md:59)
```
A repo can *ship* these files, but a **human must opt in** via `--patch` or a profile row.

**(b) Skills as `/name` — automatic, git-committable, zero config.** This is the real project-repo channel.
`packages/client/ui-skill/src/client/index.ts:1-11` states it registers the `/` skill source whose
candidates come from the `skills/list` Remote, that a pick lands the literal `/name ` text, and that "the
pre-step boundary (`dsh-tool-skill`) recognizes a leading `/name` naming a user-invocable skill and injects
the rendered body for every entry point, including `disable-model-invocation` skills the model-side catalog
never lists". Trigger registration is `trigger: '/', name: 'skill', order: 2` (`:141-143`). The host-side
gesture scan is `packages/skill/tool-skill/src/index.ts:409`:
`const SKILL_GESTURE = /(^|\s)\/([a-z0-9]+(?:-[a-z0-9]+)*)(?=\s|$)/g`. The catalog RPC returns
user-invocable skills for the session's cwd/preset
(`packages/api/session-controller/src/skill-catalog.ts:77`:
`(await skillRegistry.list({ cwd, scope })).filter(isUserInvocable)`), including `whenToUse`. `/name` works
from the composer menu **or** typed by hand; a name shared with a host command **resolves to the command**
(`packages/client/ui-skill/README.md:12`), and the source is disabled in subagent sessions
(`ui-skill/src/client/index.ts:145`). In-repo pattern for a user-only, model-invisible command
(`.agents/skills/dsh-translate-docs/SKILL.md`):
```yaml
---
name: sokonanoda-teacher
description: ...
disable-model-invocation: true    # out of the model catalog
user-invocable: true              # /sokonanoda-teacher stays available to humans
---
```
**A project repo ships commands by shipping skills** — commit `<git-root>/.agents/skills/<name>/SKILL.md`
and `/name` exists.

## 4. Subagents / agent definitions

DSH has **no agent-type registry**. `ctx.subagents` registers named **transports** (providers), not
personas — `packages/subagent/subagent/src/types.ts:344,346` declares
`interface SubagentProvider { readonly name: string }`, and a backend registers under a config-supplied name
(`packages/subagent/subagent-spawn-in-process/src/index.ts:69`). Shipped: `spawn`, `fork`, `acp`, `codex`,
`claude-code`, `dsh-sdk`. The delegation tool takes `description`, `prompt`, `provider`, `model`,
`reasoning_effort`, `run_in_background` (`packages/subagent/tool-subagent/src/index.ts:389-421`) — **no
`agent_type`**; DSH dropped that vocabulary deliberately
(`.agents/notes/archived/feature/2026-06-30-subagent-observe-enrich.md:22`).

A custom "teacher" agent is a **row**, not a file. The unit is the **agent preset**: a directory named by
the preset id (`^[a-z0-9][a-z0-9-]*$`, `packages/preset/agent-presets/src/preset.ts:18`) containing
`agent.cordis.yml` (`src/discovery.ts:37`), optionally plus `preset.yml` with display text only
(`src/metadata.ts:29-38`). Discovery roots in precedence order
(`packages/preset/agent-presets/src/index.ts:182-184`): the shipped `SHIPPED_PRESET_ROOT`
(`discovery.ts:60` — the package's own `../presets/`: `standard`, `ptc`, `minimal`, `cordis`), then
`config.roots` (deployment-configured), then `dshHomePath(USER_PRESET_DIR)` where
`USER_PRESET_DIR = '.agent-presets'` (`discovery.ts:51`) — i.e. `$DSH_HOME/.agent-presets`.
```yaml
# $DSH_HOME/.agent-presets/teacher/agent.cordis.yml   (+ preset.yml with name/description)
- id: tool-subagent-teacher
  name: '@deepseek-ai/dsh-tool-subagent'
  config:
    provider: spawn                     # name registered on ctx.subagents
    toolName: subagent_teacher          # one tool instance per toolName
    persona: You are a patient teacher. Explain in small steps and check understanding.
    toolFilter: { allow: [read, glob, grep] }
    agentOptions: { provider: deepseek, model: deepseek-chat, reasoningEffort: high }
    backgroundMode: continuable
```
Fields `provider`, `toolName`, `agentOptions`, `persona`, `toolFilter`, `maxDepth`, `backgroundMode`
(`packages/subagent/tool-subagent/src/index.ts:50-108`, schema `:106-125`); `persona`/`toolFilter` require
the provider's matching capability (`packages/subagent/subagent/src/types.ts:131-135`) and ACP/Codex/Claude
Code reject `agentOptions`. **Per-subagent model: yes**, per row —
`agentOptions: { provider, model, reasoningEffort, maxTokens }` (`tool-subagent/src/index.ts:112-119`);
per-call model choice is opt-in via `modelSelectionSettings: true` (`tool-subagent/README.md:67`).

**Project-committable? No.** There is **no project-level preset root** — presets live under
`$DSH_HOME/.agent-presets`, deployment `roots`, or the shipped system root; `<projectRoot>/.dsh/agents` does
**not** exist in the source. A repo becomes a preset source only if the *deployment* adds its path to
`Config.roots` (a `$DSH_HOME` edit). A project ships **skills**, and a skill can instruct the model to
delegate; it cannot define an agent by itself.

## 5. AGENTS.md / repository instructions

Plugin `@deepseek-ai/dsh-agent-instructions` (`packages/context/agent-instructions`), mounted
unconditionally by `dsh-base` (`packages/bundle/base/cordis.patch.yml:268-271`) with `maxBytes: 65536`.
Defaults (`packages/context/agent-instructions/src/config.ts:11-14,39-46`):
`DEFAULT_PROJECT_ROOT_MARKERS = ['.git']`, `DEFAULT_INSTRUCTION_FILE_CANDIDATES = ['AGENTS.md',
'CLAUDE.md']`, `DEFAULT_LOCAL_INSTRUCTION_FILE_CANDIDATES = ['AGENTS.local.md', 'CLAUDE.local.md']`,
`DEFAULT_MAX_SOURCE_BYTES = 1_048_576`. `maxBytes` is **required, no schema default** (`:42`);
`maxSourceBytes` = 1 MiB (`:43`); `projectRootMarkers` = `['.git']` (`:41`); candidate names may not contain
path separators (`:120-122`).

**Filenames:** `AGENTS.md`, `CLAUDE.md`, then the `.local` overlays — **CLAUDE.md is read in addition to
AGENTS.md, not as a fallback.** The user-global file is fixed: `export const USER_GLOBAL_FILE = 'AGENTS.md'`
(`src/render.ts:98`) at `$DSH_HOME/AGENTS.md` (`src/files.ts:285`). Not interpreted: `GEMINI.md`,
`.cursorrules`, `.claude/rules/`, lowercase names, `@path` imports
(`packages/context/agent-instructions/README.md:216`).

**Algorithm:** upward walk from session cwd to the first dir containing a marker; if none exists up to the
filesystem root it returns cwd itself (`src/files.ts:190,193`). The chain renders root→cwd broad-to-specific
(`return chain.reverse()`, `src/files.ts:216`), user-global first (`:285-293`), base files before that dir's
`.local` overlays. Precedence is stated in the injected intro (`src/render.ts:12-14`): "More specific
instructions take precedence over broader ones." The proven order (`tests/agent-instructions.spec.ts:361-367`)
is `['$DSH_HOME/AGENTS.md', 'AGENTS.md', 'CLAUDE.md', 'packages/CLAUDE.md', 'packages/app/AGENTS.md']`.
**Not recursive at startup** — subdirectory instructions are **touch-driven**: a first-party
`read`/`write`/`edit` (`src/index.ts:74`) queues a dynamic scope bounded to descendants of the session cwd
(`src/state.ts:299`, `src/files.ts:230`), so `packages/AGENTS.md` loads only after the agent touches
something under `packages/`.

**Budget:** a file over `maxSourceBytes` is ignored entirely (`src/files.ts:344,354`); the `maxBytes` total
drops whole broader files before truncating only the most specific one (`src/render.ts:289-315`) and never
exceeds the budget (`tests/agent-instructions.spec.ts:822`). No file-count cap; `maxBytes <= 0` disables
loading (`src/files.ts:419`). Dedup is by absolute path (`:280-282`) plus per-directory trimmed-content
SHA-1 (`:385-387`), so a `CLAUDE.md` duplicating `AGENTS.md` verbatim renders once. **Injection form:** an
ordinary durable **user-role** message wrapped in `<system-reminder>` framing, folded into the pre-step batch
(`src/index.ts:315-341`; `src/render.ts:242`). **Zero user config: YES** — commit `AGENTS.md` at the `.git`
root and it loads; the plugin only no-ops without a `ctx.fs` provider (`src/index.ts:119-120`), which base
always provides.

## 6. Goal / plan / todo surfaces

Base mounts all three (`packages/bundle/base/cordis.patch.yml:292-303,409-417`):

| Surface | Plugins | Model tools | UI |
|---|---|---|---|
| Todo | `dsh-tool-todo` | `todo_write` | latest `todo/write` event as a checklist |
| Goal | `dsh-goal` + `dsh-goal-round-driver` + `dsh-command-goal` + `dsh-tool-goal` | `create_goal`, `get_goal`, `update_goal` | `packages/client/ui-goal` + `/goal` |
| Plan | `dsh-plan-mode` | `exit_plan_mode` | `packages/client/ui-plan` + `/plan` |

**Todo** is durable, session-owned, whole-list replacement; statuses `pending | in_progress | completed`
(`docs/subsystems/todo.md:22-27`). `allowParallelInProgress` is "required with no default: a composition
that omits it fails at load" (`packages/todo/tool-todo/README.md:41-47`); base sets `true`
(`cordis.patch.yml:409-412`). One list per agent session — subagents keep their own and cannot share
(`tool-todo/README.md:57`). **Goal** is event-sourced and same-session, with a durable
`active | paused | blocked | complete` phase (`docs/subsystems/goal.md:26-30`) plus a process-local
activation flag; `goal-round-driver` supplies automatic continuation rounds and `blocked` is rejected before
3 admitted rounds (`docs/tool-catalog.md:33`), with the authority rule "create, edit, pause, and resume
require direct-human root authority". **Plan mode** is *soft guidance only* — "Plan mode is **soft guidance**.
[Sandbox mode] and [approval policy] enforce restrictions independently" (`docs/subsystems/plan.md:5`); it
owns the `plan:policy` prompt section at order 50 and registers `exit_plan_mode` + `/plan [off|message]`
(`:29,35`), and `exit_plan_mode` stays registered while inactive so the tool catalog never churns
(`docs/tool-catalog.md:22`).

**What a project can rely on:** all three are deployment-mounted by `dsh-base`, so a repo needs **no config**
to use them — but they are *session* state, not repo assets. A repo can document the intended workflow in a
skill ("one goal per teaching round, todos per step") and the surfaces will be there. `/goal` and `/plan` are
host commands; a repo cannot add another one automatically (§3).

## 7. Web GUI / client plugins

**Entry contract** — `docs/subsystems/client-modules.md:77`: "A package joins the table by declaring
`dsh.client` (`platform: 'web'`, optional `inject` edges, optional `immediately`) in its package.json and
exporting its built bundle at `exports["./client"]`." Manifest type
`packages/util/package-manifest/src/types.ts:69-81`; real declaration (`packages/client/ui-goal/package.json:21-23,28-30`)
is `"exports": { "./client": { "default": "./lib/client.js" } }` plus `"dsh": { "client": { "inject": [...],
"platform": "web" } }`. A pure-UI plugin still needs an empty host half so it is a Loader row —
`packages/client/ui-goal/src/index.ts:9` is exactly `export function apply(): void {}`.

**Host handler + client call (real).** `packages/api/session-controller/src/skill-catalog.ts:20,25,35-36`
declares `export class SessionSkillCatalog extends TypertRemoteService` whose constructor calls
`super(ctx, 'sessionSkillCatalog', { namespace: 'skills' })` and whose `@Remote`-decorated
`async list(request: SkillListRequest, signal: AbortSignal): Promise<SkillListValue>` is the handler. The
client (`packages/client/ui-skill/src/client/index.ts:64,77,103`) injects `'remote'`/`'remote.skills'`, takes
`const skills = ctx.remote.skills`, and calls `await skills.list({ sessionId }, abort.signal)`. Protocol:
`packages/typert/protocol/src/index.ts:153-167` (`TypertRemoteService`), `:174-201` (`Remote`).

**Loading is RUNTIME Node resolution over live Loader entries — not an in-repo bundle registry.**
`docs/subsystems/client-modules.md:77`: "Each live row resolves from its own Loader specifier and owning-tree
`baseUrl`, through the same `loader.internal.resolveSync` implementation". Bundles are served at
`/plugins/??<pkg>/client.js&rev=<rev>` (`:85`; route `packages/client/modules/src/index.ts:598-601`). Only two
things are build-time: the **roster** (config rows — "`dsh.client` rows are the browser roster the modules
node half scans", `packages/bundle/web-app/cordis.patch.yml:42-43`) and the shell seed `PLATFORM_MODULES`
(react, cordis, `client-store`, `ui-slots`, `ui-primitives`, `ui-dockkit` —
`packages/client/web/src/platform.ts:8-13`), bundled by `apps/web`.

**Verdict: an out-of-tree npm package CAN ship a client UI view.** The profile at `$DSH_HOME/profiles/<name>`
is pnpm-managed (`packages/boot/app-boot/README.md:50`); `dsh plugin --profile demo add ./pkg` installs it
(`docs/user/develop/basic/publish.md:80`) and an insert row mounts it (`:56-62`); out-of-tree bundles resolve
from the profile's `node_modules` (`apps/cli/reference/README.md:11`). Two hard constraints:
1. **The package must ship a prebuilt bundle.** "The host serves built client bundles, so `pnpm run build`
   must have produced each `lib/client.js` before launch; a missing bundle fails activation loudly"
   (`packages/client/modules/README.md:46`). Artifact contract: `format:'cjs'`, `platform:'browser'`,
   `sourcemap:true` (`packages/client/tsdown.client.ts:445-451`) with banner
   `window.__ModuleLoader__.load({ id: "<pkg>", factory: (require) => {` (`:577`). DSH's build presets are
   repo-internal and the root package is `"private": true` (`package.json:5`) — no published third-party
   preset found → **UNVERIFIED**. Git installs also need a `prepare` build + `allowBuilds`
   (`docs/user/develop/basic/publish.md:161-173`); npm/tarball installs do not (`:177-178`).
2. **A *typed* `ctx.remote.<ns>` handler is in-repo-only.** `ctx.remote` mounts only the hand-maintained
   static import list in `packages/api/remotes/src/client/index.ts:4-20`, and "The Client does not discover
   decorators from the running Host … its types, codecs, and Remote registration values always come from the
   most recently generated `lib/typert.remote-client.*` artifacts" (`docs/api-gateway.md:137`). An out-of-tree
   host handler must expose a raw route instead — `ctx.webServer.register({kind:'exact',path,handler})`
   (`packages/host/webserver/README.md:45`; real dual-half example
   `packages/host/open-in-app/src/index.ts:193` + `packages/client/ui-open-in-app/src/client/controller.ts:35`).

**No-build alternative:** agent-authored dynamic Cordis packages in `packages/extensions/` — host
`harness.handle(method, handler)` / browser `host.call(method, args)`
(`packages/extensions/tool-cordis/src/prompt.ts:98`), invoked via `@Remote('invoke') invoke(pluginId,
pluginRunId, method, args)` (`packages/extensions/cordis-host-runner/src/index.ts:736-764`), with the browser
half registering views through `slots.inject(...)→slots.register(...)`
(`packages/preset/agent-presets/presets/cordis/skills/cordis-plugin-development/SKILL.md:242-252,327-345`).
Limits: session-scoped and process-local, cleared on DSH restart
(`packages/extensions/tool-cordis/README.md:59`), needs user approval — an in-process extension path, not a
distribution format. **UNVERIFIED:** no doc, test fixture, or example in the checkout actually builds and
installs a *browser* half out-of-tree (the publishing tutorial's bundle is host-only,
`docs/user/develop/basic/publish.md:46-62`); the mechanism is unimpeded by source but undemonstrated.

## What a project repo can ship by itself

| Asset | Commit path | Zero user config? |
|---|---|---|
| Skills (model discovery **and** `/name` command) | `<git-root>/.agents/skills/<name>/SKILL.md` or `.dsh/skills/` | **Yes** |
| Skill with relative `references/`, `scripts/`, `templates/` | inside the bundle dir | **Yes** |
| Repo instructions | `<git-root>/AGENTS.md`, `CLAUDE.md`, `*.local.md` | **Yes** |
| Nested instructions | `<subdir>/AGENTS.md` | Yes, but only after the agent touches that subtree |
| Non-standard `skills/` root | `skills/` | No — needs `customSkillDirs` in a profile patch |
| Real slash command (`ctx.commands`) | `.dsh/plugins/*.js` + a patch file | No — user runs `--patch`, or installs a bundle |
| Custom subagent agent ("teacher") | — | No — must be a preset under `$DSH_HOME/.agent-presets/` |
| Client UI view | npm package with `dsh.client` + prebuilt `lib/client.js` | No — profile install; typed RPC still needs an in-repo handler |

**Recommended shape for this repo:** ship `.agents/skills/sokonanoda-teacher/SKILL.md` (plus `references/`)
with routing cues in `description` — that alone gives the model auto-discovery *and* a
`/sokonanoda-teacher` command in the GUI, committed to git, with no `$DSH_HOME` edit. Use
`disable-model-invocation: true` + `user-invocable: true` for a human-only command. The three existing
`skills/*/SKILL.md` files are already DSH-valid (only `name` + `description`), but the root-level `skills/`
dir is **not** scanned — either move/symlink it to `.agents/skills/` or document the `customSkillDirs` patch
as an opt-in.
