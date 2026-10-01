# M0 基线冻结 + 能力清单（IA-4 元参数引擎 · **零行为变化**）

> 日期：2026-10-01（第 525 轮）。设计 ⇒ `docs/design/metavar-engine.md`（§4 的 **M0** 片）；
> 判据代码 ⇒ `crates/cli/tests/metavar_inventory.rs`（13 形状 × 2 开关态 = **26 次真进程**）。
> **M0 不改任何实现** ✓：只加**判据**（测试）与**取证**（本文）。

## 0. 一句话结论

**13/13 形状上，元参数引擎相对今天的「接受面增量 = 0」** ✗ —— 引擎与今天**逐条同判**；
差别只在**机制**（一条机械取代三条）、**冲突检出**（S12：从内核晚期拒绝提前到 elab 期）、
**sort/kind 守卫**（今天没有）与**范围 B 的地基** ⇒ 触发 **D7：缩范围**（§3）。

## 1. 冻结的基线（**M1 起每条都要逐项复现**）

| 读数 | 值（2026-10-01 · `target/release/sokonanoda` = v0.79.0）| 命令 |
|---|---|---|
| `--digest` 非课程(86) | `7646fe2eb6cf50c023cad698c2c20043bde9b5eb2f9ded5d0f33950d0a530ca1` | `bash scripts/kernel-diff.sh --digest ./target/release/sokonanoda` |
| `--digest` 课程(42) | `d0375577b5999b5b60ceb8606e16c5945115d35e02e47ed33e751eaee064454d` | 同上 |
| `--digest` 全语料(129) | `06370a380782182fc87af9a0e4a56509eb2c5e140804a71d622b503c6b259395` | 同上 |
| **非课程红线自对拍** | **0 差异**（同一二进制对拍自己 ⇒ 判据通道本身是活的 ✓）| `bash scripts/kernel-diff.sh --non-course <bin> <bin>` |
| 课程门禁 | **43 目标 · 377 checked · 99 open · 0 判负**（canvas_open 96）| `SOKONANODA_BIN="$PWD/target/release/sokonanoda" python3 courses/set-theory/tools/check.py --json` |
| 冷 `build` 结构计数 | `JUDGE_PREFIX runs=886 bytes=47,437,669` · `passes=1343` · `by_calls=21,268` · `hits/misses=20,853/302` · `compiled 42 / failed 0 / hit 0` | `build --clean courses/set-theory` **+ 空的 `SOKONANODA_CACHE_DIR`** ⇒ 见下方 ⚠ |
| G-48 复现件 | 默认 **exit 0** / `SOKO_NOTATION_METAVAR=0` **exit 1** | `<bin> docs/gaps/repro/G48-*.sokonanoda`（两态）|

⚠ **结构计数对缓存敏感（本轮实测的口径修正）**：只跑 `build --clean` **不够** —— 全局编译缓存
（`~/Library/Caches/sokonanoda`）里还剩东西时，同一条命令会给出 `compiled 24 / hit 18`、
`runs=606`（**不是**回归，是热启动 ✗）。**要复现冻结值必须同时给一个空的
`SOKONANODA_CACHE_DIR`**（见 §4 的第一条命令）—— 这条口径写进这里，免得下一轮把 `606` 当成退化 ✗。

⚠ 与 `metavar-engine.md` §1.4 的三指纹**逐字节相同** ✓（设计期与本轮各测一次，互相印证）。

## 2. 能力清单：13 形状 × 三态（**实测**，判据在 `metavar_inventory.rs`）

三态口径：**可解** = 两态都绿（元变量不参与）· **依赖默认** = 严格档红 + 默认态绿（靠 E19 的
「同形已解兄弟 ⇒ 取它的值」这条**选择规则**）· **解不出** = 两态都红（无来源，或**约束冲突**）。

| # | 形状 | 走哪条路线 | 默认态 | 严格档 | 类别 | **引擎推演**（约束集 ⇒ 解）| 增量 |
|---|---|---|---|---|---|---|---|
| S01 | `a ∈ A` | 记法① 首个显式操作数 | 绿 | 绿 | 可解 | `Set ?α ≟ Set α` ⇒ `?α := α` | **0** |
| S02 | `picks 3 b` | 应用① 扫后续层 | 绿 | 绿 | 可解 | `?α ≟ α`（第 2 个显式层）| **0** |
| S03 | `Or.inl h`（有期望）| 应用② 期望类型 | 绿 | 绿 | 可解 | `?A ≟ (1=1)`、`Or ?A ?B ≟ Or (1=1) (2=2)` | **0** |
| S04 | `∅ : Set Nat` | 裸常量②（G-40）| 绿 | 绿 | 可解 | `Set ?α ≟ Set Nat` | **0** |
| S05 | `f '' A` | 记法① Pi 域/陪域 | 绿 | 绿 | 可解 | `?α → ?β ≟ α → β` | **0** |
| S13 | `F A A`（`Set Nat` vs `?α → Prop`）| 两条约束 **defeq 一致** | 绿 | 绿 | 可解 | 需 **delta 兜底**才与今天同判 ⚠ | **0**（**回归风险**）|
| S14 | `Set.univ x` | 路线③ 富余实参（G-41）| 绿 | 绿 | 可解 | `?α ≟ α`（富余层）| **0** |
| S06 | `∅ ≈ {b}` | 记法 + 待定档（G-48）| 绿 | 红（notation 码）| 依赖默认 | `?β := β`；`?α` 无约束 ⇒ **defaulting** `?α ≟ ?β` | **0**（机制不同）|
| S07 | `Set.Equiv ∅ {b}` | 应用 + 待定档（刀2）| 绿 | 红（implicit 码）| 依赖默认 | 同 S06，入口是实参位 | **0** |
| S09 | `Set.Equiv {a} ∅` | 顺序相反 | 绿 | 红（implicit 码）| 依赖默认 | 已解的一侧在后 ⇒ defaulting 仍借得到 | **0** |
| S10 | `Set.Equiv ∅ ∅` | 两侧都零元糖 | 红（implicit 码）| 红（同码）| 解不出 | 全未解、无代表可依 ⇒ 失败（**不猜** ✓）| **0** |
| S11 | `ignores 3` | `α` 不可达 | 红（implicit 码）| 红（同码）| 解不出 | 唯一未解元变量、无同形兄弟 ⇒ 失败 | **0** |
| S12 | `Two A B`（`Set Nat` / `Set Bool`）| **约束冲突** | 红（**内核**码）| 红（同码）| 解不出 | `?α ≟ Nat` ∧ `?α ≟ Bool` ⇒ **clash** ⇒ 应**提前到 elab 期**报 | **0**（**诊断**增量）|

**三态分布**：可解 **7** · 依赖默认 **3** · 解不出 **3**（测试里钉死，变了就红 ✓）。

### 2.1 清单读出来的两条硬约束（M1 的验收条件，不是建议）

1. **S13 ⇒ 引擎必须复用 delta 展开兜底**：两条约束在**语法上不同形**（`Set Nat` vs `?α → Prop`）
   但**defeq 一致**；今天的贪心求解只查第一条就返回 ⇒ 绿。引擎若把第二条当刚性冲突 ⇒ **误拒**
   （把今天绿的判红 = 红线 ✗）⇒ 合一时必须先试原形、失败再展开（与 `solve_prefix` 既有两条路线同款）。
2. **S01–S14 的 26 个读数 ⇒ `sibling` 态必须逐字节不变**：M1 的开关三态里 `sibling` 那一档
   （`SOKO_METAVAR=sibling`）就是今天的语义 ⇒ 本表**每一条**都是回归判据（退出码 + 诊断码）。

### 2.2 没进清单的两类形状（**明写边界**）

* **kind 检查档**：今天「把一个 kind 放到要项的位置」落**内核** `def_eq mismatch expected: Sort(1) | actual: Sort(2)`
  （设计 §1.4 的探针实测）——它要 **M3** 的 sort/kind 检查才有 elab 期的说法，M0 不建夹具（避免把
  「内核兜底」误当「求解器判红」）。
* **期望类型传播档**（G-30/G-33）：属**范围 B**，前缀内闭环的引擎**不治** ⇒ 不进清单。

## 3. 结论：**值不值得做 A**（D7 落地）

* **量化**：接受面增量 **0/13**（13 个形状、覆盖记法/应用/裸常量/路线③/两条顺序/冲突/defeq 一致）。
  ⇒ 与设计 §0 的 **C2** 一致：实参类型永远是**具体项** ⇒ 约束恒为「模板 ≟ 具体项」的**单侧**形状，
  而 `unify_extract` 已经在做这件事（含嵌套位 / Pi / 记法头 / delta 兜底）。
* **引擎仍然值的部分**（不是接受面）：① **sort/kind 检查**（今天没有）② **冲突检出**（S12 从内核
  晚期提前到 elab）③ **一条机械取代三条**（`unify_extract` + 两个待定档 + `fill_pending_by_shape`）
  ④ **范围 B 的地基**（元变量进 elaborate 期 ⇒ G-30/G-33 一族）。
* **按 D7（用户 2026-10-01 拍板）= 照做但缩范围** ⇒ 范围 A 的后续切片按此重排：
  **M1 引擎内核**（含 S13 的 delta 兜底与 `sibling` 回归臂）→ **M3 sort/kind + 报错契约**（价值最高的一片）
  → **M2 一般路径接线**（机制收窄）→ **M4 默认开**（**只在 M1–M3 全绿后才谈**）。
  **不许**为凑接受面增量去放宽判定 ✗（D7 原文）。
* **另**：S12 一类「今天拖到内核才拒」的形状给了 **M3 的判据种子**（clash 检出后报哪个码 ——
  D6 = 不新增码 ⇒ 仍降级到既有码，但位置从内核挪到 elab）✓。

## 4. 复跑（每条可直接粘）

```bash
cargo test -p sokonanoda-cli --test metavar_inventory          # 13 形状 × 2 态（本表的判据）
bash scripts/kernel-diff.sh --digest ./target/release/sokonanoda
bash scripts/kernel-diff.sh --non-course ./target/release/sokonanoda ./target/release/sokonanoda
SOKONANODA_BIN="$PWD/target/release/sokonanoda" python3 courses/set-theory/tools/check.py --json
./target/release/sokonanoda build --clean courses/set-theory        # 清项目产物（+ 全局）
SOKONANODA_BUILD_JOBS=1 SOKO_STAGE_STATS=1 SOKONANODA_CACHE_DIR=/tmp/m0-cold \
  ./target/release/sokonanoda build --json courses/set-theory       # ← 空的 CACHE_DIR 才是冷启动
./target/release/sokonanoda docs/gaps/repro/G48-notation-nullary-sugar-operands.sokonanoda          # exit 0
SOKO_NOTATION_METAVAR=0 ./target/release/sokonanoda docs/gaps/repro/G48-*.sokonanoda                # exit 1
```

**记账**：M0 收口 ⇒ `metavar-engine.md` §4 的 M0 行 + `ONBOARDING.md` §0.2 + `STATUS.md`（第 525 轮）；
M1 的 as-built 追加到 `metavar-engine.md`（本文只做 M0 的基线 + 清单）。
