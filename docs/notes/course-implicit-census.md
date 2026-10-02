# 课程侧隐式参数（"隐变量"）适配 · 普查结论（#5 第 0 步 · 2026-10-03）

> 队列：`docs/ONBOARDING.md` **§0.2 #5**。判据源 = `scripts/notation-lint.py`（`--census*` ✓）；
> 明细 = `courses/set-theory/gaps/notation-ok-census.md`（口径与逐轮读数）+ `gaps/census-ledger.json`（逐处台账 ✓）。
> **整批适配的前置** = 内核 IA-4 的 **U1/U2** ⇒ 本文只给**普查读数与归属** ✓。

## 一、数字（全量 · `--census-batch`）

| 项 | 数 | 口径 |
|---|---:|---|
| 豁免**命中** | **1876**（试点后 1872） | 逐处可定位（一行可命中多类 ✓）；**行数**口径 1615 行 |
| 其中**可机械换** | **629**（试点后 625） | 只有「删前导实参」一类：`And.left A B h` ⇒ `And.left h` ✓ |
| **需手工** | 1210 | 点名 → 中缀（`Set.mem α a A` ⇒ `a ∈ A`、`Eq.{1} T a b` ⇒ `a = b`）要**重构表达式** ⇒ 不在本口径 ✗ |
| 注释内 / 改写器空转 | 37 / **0** | 注释测了没意义 ✓；「说改却没改」被断言打红 ✓ |
| 覆盖 | **114 个文件** | 全部含可机械换命中的文件 ✓ |
| **组判卷**（整文件的候选一次改完再判） | **组绿 24 文件（45 处）** · 组红 90 | **组绿 = 该文件的豁免现在就能整批删** ✓ |
| **逐处裁定** | **绿 109 / 红 86**（实测 195 ⇒ 绿 **56%**） | 组红文件逐处回落，**每文件抽样 ≤6 处** ✓ |
| 未测 | 434 | `sampled-out` 232（抽样上限）· `group-red` 202（预算用光）⇒ **未测 ≠ 红** ✓；扩 `--census-limit` / `--census-per-file` 即可补测（≈20 秒/处 ✓） |

**抽样口径（重要 ✓）**：组判卷 = **全量**（114 文件全覆盖 ✓）；逐处裁定 = **抽样**（组红文件每文件 ≤6 处 ✓）
⇒ **56% 是抽样绿率，不是全量精确值** ✓。

## 二、结论

1. **豁免真的能删** ✓（试点实证）：`units/solutions/I.10/unit84-solution` 改写 4 处 + **删标记 4 处** ⇒
   `notation-lint` **exit 0** 且 `grade` **exit 0**（`checked=13 · open=0 · failed=0` ✓）。
2. **可机械换里约一半现在就能改** ✓：组绿 **45 处**有整批证据；另有逐处判绿 **64 处**（按文件复验后可删 ✓）。
3. **红的 86 处是引擎边界** ✗，不是课程写法问题：`h` 的类型是 **def-应用**时
   `And.left A B h` ⇒ `And.left h` **补不出前导命题**（诊断「期望 `Sort(0)`，实际是 `Set.mem …`」✓）。
4. **1210 处「需手工」**（点名 → 中缀）要**真改写器**（重构表达式，不是正则活 ✗）⇒ 同属后续批次 ✓。

## 三、86 处红的归属：**待内核 U1/U2** ✓

- **归属** = 内核线 **IA-4 的 U1/U2**（隐式参数推断：在 def-应用的期望类型 / 假设类型上补前导隐式实参）✓。
- **课程侧纪律** ✓：**不手工改这 86 处** ✗，也**不为它们加新豁免以外的东西** ✗（现有 `soko:notation-ok` 原样保留 ✓）。
- **验收（U1/U2 落地后）**：`--census-batch` 里这 86 处转 **green** ✓，且 `courses/set-theory/tools/check.py` 仍 **exit 0** ✓。
- **逐处清单**：`courses/set-theory/gaps/census-ledger.json` 中 `verdict == "red"` 的行（file / line / rule ✓）。

## 四、复现（三条命令）

```bash
python3 scripts/notation-lint.py --census-classes                    # 分类（秒级 ✓）
python3 scripts/notation-lint.py --census --census-limit 40          # 小批量「绿 x / 红 y」（复跑 19/21 ✓）
python3 scripts/notation-lint.py --census-batch --census-out /tmp/led.json   # 全量（≈2 小时 · 20 秒/处 ✓）
```
