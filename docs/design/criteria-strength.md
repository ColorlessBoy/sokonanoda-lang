# 判据强度普查（E00）

> 2026-09-27。**为什么先做它**：**判据太弱 = 等于没有判据** —— 不先修这个，
> 后面每个环节做完都**看不出来做没做**（`docs/PLAN-0.74-0.79.md` §E00）。

## 1. 实证：一个「看不见 bug」的判据（G-37）

`docs/gaps/repro/G37-notation-decl-target-not-a-use-point.js` 原来只断言
`summary !== 'null'`（**有答案**），**不断言落在哪一行**（**答案对不对**）。
2026-09-27 实测（真 LSP，`courses/set-theory/lib/Set.sokonanoda`）：

| 目标 | 应落 | 实际落 | 旧判据怎么说 |
|---|---|---|---|
| `Set.powerset` | **L81**（`def powerset`） | **L124** = 光标自己那一行 | 非 null ⇒ **已修** ✗ |
| `Set.compl` | **L79**（`def compl`） | **L125** = 光标自己那行 | 非 null ⇒ **已修** ✗ |

⇒ 台账写着 `fixed`（`fixed_in: 0.65.5`），而这两条**全是自跳**（F12 视觉上等于没反应）。
**这就是「账面绿、实际坏」的标本** —— 防翻车复盘第 2 条。

## 2. 四类弱判据 + 扫描口径

| 类 | 形态 | 扫描命令 |
|---|---|---|
| ① | 只断言非空/非 null/exit 0，**不断言具体值** | `grep -nE "!== ?'null'\|!== null\|\.length > 0"` |
| ② | 只断言数量 >0，**不断言是哪几个** | `grep -nE "> ?0\b\|>= ?1\b"` |
| ③ | **缺反向验证**（守卫咬不住已知 bug） | 有没有 `--selftest` / 负例 |
| ④ | 把 `skipped` 当绿 | 读 CI **整轮** `conclusion` 的地方 |

## 3. 普查结果

### 3.1 类 ① / ②（repro 探针）

| 文件:行 | 类 | 弱在哪 | 升级后判据 |
|---|---|---|---|
| `docs/gaps/repro/G37-….js:119,124` | ① | 只 `!== 'null'` ⇒ **自跳也判绿** | **已升级 ✓**：落点行号 == `^def <name>` 的行号；自跳单独标出 |
| `docs/gaps/repro/G23-….js:159` | ② | `defList.length > 0`（只问"有没有"） | 落点**行号** == 期望（178 行已断言"落到哪个文件"✓，缺的是**行**） |
| `docs/gaps/repro/G24-….py:172` | ② | `stale["hits"] >= 1` | 断言**哪几个** cache 命中（键名集合相等） |
| `docs/gaps/repro/G27-….py:116` | ② | `hits2 >= 1` | 同上（断言具体键） |

### 3.2 类 ③（缺反向验证 —— 守卫咬不住）

**有 `--selftest`（能自证咬得住）✓**：`docs-lint.py` · `audit-wire-fields.py` ·
`gap.py` · `courses/set-theory/tools/check.py` · 本轮新增 `ci-green.py` · `target-hygiene.py`

**没有 selftest ✗**：

| 文件 | 风险 |
|---|---|
| `scripts/audit-notation-paths.py` | **棘轮基线 59 是地板**，却无法自证"能咬住**新增**绕过" |
| `scripts/notation-lint.py` | 课程记法门禁（进 gate 与 CI） |
| `scripts/status-lint.py` | STATUS.md 瘦身门禁 |
| `scripts/ci-yml-lint.py` | 工作流 lint |

### 3.3 类 ④（把 skipped 当绿）

**已修 ✓**（前置 2）：`scripts/ci-green.py` —— 逐 job 判，**skipped 单独列出、
绝不折进绿**；重活被 skip ⇒ `exit 2`（假绿，**不构成证据**）。
真轮次夹具：`--run 36291907128`（纯 docs 推送）⇒ **exit 2** ✓ ·
`--run 36285752786`（0.73.0 发版 bump）⇒ **exit 0** ✓。
对照：`scripts/ci-watch.sh` 对前者 **exit 0**（只判 failure ⇒ **咬不住**）✗。

## 4. 变异测试：**有几条判据会红**

「改坏实现 ⇒ 判据必须红」。本轮可判定的 5 条变异：

| # | 变异（改坏成已知 bug 的行为） | 应判红的判据 | 实测 |
|---|---|---|---|
| M1 | definition 返回**光标自己那一行**的 span（= E05 的 bug） | 升级后的 G37 | **红** ✓（exit 0 + 打出「自跳」L124/L125） |
| M2 | CI 轮次里重活**全 skipped** | `ci-green.py` | **红** ✓（exit 2 假绿） |
| M3 | `target/` 里 **966,858** 个 `.rcgu.o` + 第二套 target | `target-hygiene.py` | **红** ✓（selftest 用例） |
| M4 | `docs/PLAN-….md` 626 行**未登记** | `docs-lint.py` | **红** ✓（626 > 400，本轮实测过一次） |
| M5 | **旧判据**（非 null）对 M1 | —— | **绿** ✗ ← **这就是 E00 要修的东西** |

**结论：4 条判据能咬住各自的已知 bug ✓；M5 是升级前的状态**（判据本身是坏的）。

## 5. 还没做（**如实记**，别当已完成）

- 3.1 里 **G23 / G24 / G27 三条升级还没落地**（**G37 已落地 ✓**）；
- 3.2 的**四个 `--selftest` 还没补**；
- 变异测试目前是**手工 5 条**，不是计划里说的"故意改坏 N 处"的**全量**；
- `docs/gaps/ledger.jsonl` 的 **G-37 已按新判据改回 `open`** —— 原 `fixed` 是
  **弱判据下的误判** ✓（这条是"坏消息自己找上门"的实例：升级判据 ⇒ 台账门禁判红 ⇒
  被迫面对）。
