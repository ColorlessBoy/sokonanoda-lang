# P2-12 **S7 实施单**：翻默认档（strict 成为默认，`--dialect loose` 成为退路）

> 前置：S6（`cfb64bf8`）已拆掉 strict 档内的两道过渡 env；发版面（`26400a5c`）已把措辞写成"闸门已删、
> **默认档未翻**"。本刀 = 把最后那一格翻过来。用户 2026-11-01：「我们的计划是 strict 模式彻底推上线」。

## 0. 一句话

`Dialect::default()` 从 `Loose` 改成 **`Strict`**；`--dialect loose` / 源码指令 / 设置项三条退路保留；
VS Code 设置项 `sokonanoda.dialect.strictImplicitArgs` 的 `default` 同轮改 `true`（否则编辑器与 CLI 又反向不一致）。

## 1. 已量过的风险（`measured`，决定这刀能不能做）

| 形状 | loose | strict | 结论 |
|---|---|---|---|
| `add3 1 2 3`（普通多参） | ✓ | ✓ | 不受影响 |
| `adder 1 2` / `(adder 1) 2`（脊上两步） | ✓ | ✓ | **不受影响** |
| `h x` / `h x y`（依赖应用） | ✓ | ✓ | **不受影响** |
| `Set.notMem_empty (f x) (hn x hx)` | ✓ | **✗** | **只在 2 个 orphan 解答里**（`unit12-solution` / `unit19-solution`，不在 `course.json`） |

⇒ 除那 2 个 orphan 外，**357 个语料文件里没有别的变化**（S6 读数 §③）。所以翻默认的用户可见代价 = 这 2 个文件 + 例外表
（例外表**行为与 loose 相同**，不产生新拒绝）。

## 2. 改哪

1. `crates/front/src/compile/dialect.rs`：`#[default] Loose` → `Strict`（`:38` 附近）+ 文件头注释（"默认 loose"那类）改准。
2. `CompileOptions::default()` / `resolve()` 末尾的 `unwrap_or_default()` 自动跟着变（核对一遍，别留第二处默认）。
3. `crates/cli/src/dialect.rs` 头注释（仍写"默认 loose"）。
4. `crates/lsp/src/dialect.rs`：设置的布尔形态极性（`true = strict`）不变，但**默认值来源**跟着变。
5. **VS Code**：`editor/vscode/package.json` 的 `sokonanoda.dialect.strictImplicitArgs.default` → `true` + 文案
   （开 = strict（现在的默认）；关 = 旧读法 loose）· `README` / `CHANGELOG` 同轮。
6. **两个扩展测试必须同轮改**（发版面子代理踩过）：`test-extension-host.js` 的"默认"断言（它从 package.json 播种默认值）
   与 `src/test/extension.test.js` 的 `dialect setting flips…` 整条（默认 strict ⇒ 诊断在，关掉 ⇒ 消失）。
7. 文档：`REQUIREMENTS §9.40`（把"默认档未翻"改成"已翻，`--dialect loose` 退路"）· `AGENTS.md` 硬规则 7 ·
   `skills/sokonanoda-dev` §1.5 · `docs/design/dialect-config.md` · `docs/ONBOARDING.md` 纪律 11。

## 3. 判据（逐条；跑完贴读数）

```bash
# ① 默认档实测：不设任何 env / 不传 --dialect
#    `id2 Nat 3` 必须 **rc=1**（strict 行为）；`--dialect loose` 必须 rc=0
# ② 判卷面（**默认档**，不带任何 env/flag）
python3 scripts/strict-score.py --bin <bin> --course     # lib 17/17 两档 · work_le_loose · course_strict 112/112
python3 courses/set-theory/tools/check.py                # 默认档 = strict ⇒ 必须全过（G1–G7）
# ③ 语料：默认档跑 714 行，与"S6 的 strict 档"逐字节相同；`--dialect loose` 与"S6 的 loose 档"逐字节相同
#    （即：翻默认只是换了默认值，两档本身的行为一字未变）
# ④ 退路三条都要能走：--dialect loose · 源码指令 · 设置项（LSP 用手改 JSON 的 wire 测试）
# ⑤ 扩展三层：cargo test -p sokonanoda-cli --test skill --test dsh · node editor/vscode/test-extension-host.js · docs-lint
```

## 4. 边界

- 不许改判定逻辑（只改**默认值**与文档/设置）。
- 不许碰 `crates/kernel/**`；`elab.rs 8841` 不许涨。
- 已知代价（**必须原样写进 §9.40**，不许淡化）：`unit12-solution` / `unit19-solution` 两个 orphan 在默认档下被拒
  （`Ne` 隐式位闭环 · 脊上分两步应用 + 级联），它们不在 `course.json`（112 个判卷目标不受影响）。
- 不许 `git commit` / `git push`；bump 与 push main 是 P4，且**必须先问用户**。
