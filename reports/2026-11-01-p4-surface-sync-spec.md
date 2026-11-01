# P4 前置：**发版面同步**（用户可见改动必须同一轮改完，2026-11-01 17:20）

> 起因：`P2-12 S6`（`cfb64bf8`）让 **strict 成为默认档**，两道环境变量消失。这是**用户可见**的语义变化，
> 按 `AGENTS.md`「收尾义务」必须同一轮同步：`skills/` 三件 · `editor/vscode/` · `AGENTS.md` ·
> `REQUIREMENTS.md §9` · `docs/ONBOARDING.md` · `docs/design/dialect-config.md`。
> 站点数据由 `scripts/gen-site-data.py` 在 bump 时生成（**不手写**），归 P4。

## 0. 已实测的事实（本单的依据，别重测）

| 事实 | 读数 |
|---|---|
| 不设任何 `SOKO_*` 跑 CLI ⇒ **走 strict**（内核路径） | c1 ⇒ `elab-implicit-argument-unsolved` @ `2:14` |
| 代码里已无 `SOKO_STRICT_KERNEL_APP` / `SOKO_STRICT_NO_FALLBACK` | `git grep` 在 `crates/ scripts/ skills/ docs/ editor/` = **0** |
| `SOKO_STRICT_AT` 仍在（方言来源，不是本线的闸门） | `crates/cli/src/dialect.rs`、`crates/lsp/src/dialect.rs` |
| 判卷面 | `strict-score.py --course`：`course_strict` **112/112 failed 0** |

## 1. 一个必须先修的**不一致**（用户可见）

`editor/vscode/package.json:307` 的设置项：

```json
"sokonanoda.dialect.strictImplicitArgs": { "type": "boolean", "default": false,
  "markdownDescription": "… off by default = the old reading …" }
```

`default: false` ⇒ 扩展默认把方言设成 **loose**，而 CLI 默认已经是 **strict** ⇒ **编辑器与 CLI 判定不一致**。
修法（**二选一，默认取 (a)**）：
* **(a)** 把 `default` 改成 `true`，并把描述改成「默认开 = strict（位置实参只绑显式形参，喂隐式位要写 `@`）；
  关掉 = 旧读法（loose）」；
* (b) 换成一个新键 `sokonanoda.dialect.loose`（默认 `false`）—— 更符合新默认，但要处理旧键的兼容（读旧键仍认）。
无论选哪个：**设置项文案算用户可见改动**（AGENTS 规则），`README` / `CHANGELOG` 同步。

## 2. `skills/` 三件（`skills/sokonanoda-dev` 里有一条**与现状冲突的规则**，必须显式处理）

`skills/sokonanoda-dev/SKILL.md:72-74` 现在写着：

> ## 1.5 方言：严格档是 **opt-in**（F5，2026-10-20）
> - 默认 `Dialect::Loose`；严格档用 `SOKO_STRICT_AT=1` 或 CLI `--dialect strict`（S3）。
> - **不许翻默认**：翻默认是独立决策（≥2 周 opt-in + 0 回归报告）。

⇒ **改写**（不是删）：说明 2026-11-01 用户拍板「strict 彻底上线」**取代**了那条 opt-in 门槛；
新事实 = strict 默认、`--dialect loose` / 设置项可退回、内核路径 + `Report::Fallback` 例外表。
另外两件：`skills/sokonanoda-teacher/SKILL.md`（§里提到 `--dialect strict` 的地方改成"现在默认就是它，不用传"）、
`skills/sokonanoda-ci/SKILL.md`（若提到闸门 env 就删）。**`.agents/skills/` 入口不动**（`crates/cli/tests/dsh.rs` 守）。

## 3. `AGENTS.md`

硬规则 7（strict = 绑定策略那段）追加一句：**「2026-11-01 起 strict 是默认档（S6），闸门 env 已删；
可退回用 `--dialect loose`」**。其余不动。

## 4. `REQUIREMENTS.md §9` 追加一条（**这是"用户要求"，必须留档**）

内容要点：用户 2026-11-01「我们的计划是 strict 模式彻底推上线」⇒ strict 成为默认档；两处 env 删除；
**两档的有意差异**（写明的）：① 报错形如 `@f <隐式位> <显式实参>`；② 例外表（`input_contains_a_meta` /
`shape` / `ho_motive` / `prefix_meta` / `argmismatch_text` / `deferred`）仍回落老路 —— 因为 Lean 在这些形状上会拒、
而我们的老路接受，删掉会让现在能过的课程文件被拒；③ `strict ≤ loose` 在全语料 **348/355** 个文件上成立，
其余 7 个的差额全部来自例外类回落（全语料 calls 1.023×）；④ 2 个 orphan 解答（不在 `course.json`）在 strict 下被拒。
**格式**：照该文件既有条目的写法（日期 + 起因 + 要求 + 判据）；注意 `docs-lint` ② 单文件行数上限。

## 5. 其余两处

* `docs/ONBOARDING.md`：若写着"strict 是 opt-in / 用 `SOKO_STRICT_AT=1`"⇒ 改成新事实。
* `docs/design/dialect-config.md`：方言的三层来源（flag > 环境 > 源码指令 > 设置）+ 默认值变化。

## 6. 判据（可执行）

```bash
python3 scripts/docs-lint.py                       # 绿（①–⑨；别把哪份文档写超）
grep -rn "SOKO_STRICT_KERNEL_APP\|SOKO_STRICT_NO_FALLBACK" skills/ docs/ AGENTS.md editor/   # 期望 0 行
grep -rn "默认 .Dialect::Loose\|opt-in" skills/ AGENTS.md docs/ONBOARDING.md                 # 期望 0 行（或已明确标注被取代）
cargo test -p sokonanoda-cli --test skill          # 技能与 DSH 入口不漂移
cargo test -p sokonanoda-cli --test dsh
node editor/vscode/test-extension-host.js          # 改了 package.json 就必须跑（stub 宿主）
```

## 7. 边界

- **不许改判定代码**（这条线只改文档/技能/设置项文案/package.json）。
- 不许 `git commit` / `git push`；不许动版本号（bump 是 P4，且要用户点头）。
- `REQUIREMENTS.md` 是权威文件：**只追加 §9 一条**，不许改写既有条目。
