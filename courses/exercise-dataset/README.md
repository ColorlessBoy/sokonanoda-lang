# 课程题目集数据集（exercise-dataset）

**一句话**：把三门课程教材里**已经被课程门禁判过**的练习，抽成一份稳定 ID、可校验、可被语言服务插件直接消费的 JSON 数据集。

- 大计划 / 路线：[#96 [C9] 语言服务插件 × 课程教材：题目集数据集（大计划）](https://github.com/ColorlessBoy/sokonanoda-lang/issues/96)
- 课程源文件是**唯一真相**：本目录只做**派生**，改题请改 `courses/<course>/**`，再跑 `build.py`。
- 生成物 `data/*.json` **进 git**（评审可 diff、CI 可判漂移），但它只由 `build.py` 生成。

## 当前基数（2026-11-24）

| 课程 | 单元 | 练习 | 解答 | 提示 | JSON |
|---|---:|---:|---:|---:|---:|
| `quick-start` 入门课 | 11 | 66 | 66 | 198 | 0.12 MB |
| `set-theory` 卷 I 集合论 | 112 | 910 | 910 | 1751 | 1.93 MB |
| `analysis` 卷 II/III 实分析 | 171 | 1019 | 1019 | 3000 | 2.54 MB |
| **合计** | **294** | **1995** | **1995（100%）** | **4949** | **4.59 MB** |

- 练习 = 画布中带 `sorry` 的 `theorem` / `def` / 匿名 `example`（匿名 example 命名为 `example-N`）。
- 190 道旧练习没有 `soko:hint`：数据集**照实记录**，不补写；补提示属于内容工作，见 #96 P1 的覆盖报告。
- 每道练习都有一条解答声明；`check.py` 拒绝「解答里还剩 `sorry`」的记录。

## 文件

| 路径 | 作用 |
|---|---|
| `model.py` | **画布/解答解析**：声明、`soko:hint`、题干、源行；`build` / `check` / `compose` 共用 |
| `course_manifest.py` | 课程清单（v1 扁平 / v2 volumes→chapters）与三种解答布局；新增课程只需在这里登记 |
| `dataset.py` | 记录组装（模板/解答匹配、匿名 example 命名、源映射、指纹）；`build` / `check` 共用 |
| `build.py` | 生成 `data/*.json`；`--check` 判「提交的数据是否与源文件一致」 |
| `check.py` | 独立校验：schema、ID 唯一、来源行、模板/解答在源文件里找得到、解答无 `sorry`、指纹、计数；`--selftest` 注入坏数据必须判红 |
| `compose.py` | 由记录 ID 组装**可判卷的完整 `.sokonanoda`**（模板或解答），前缀来自课程源文件 |
| `data/manifest.json` | 课程清单、计数、每门课的源文件指纹 |
| `data/<course>.json` | 每门课的单元表 + 练习记录 |

## 命令

```bash
# 生成 / 判漂移
python3 courses/exercise-dataset/build.py
python3 courses/exercise-dataset/build.py --check      # 提交的数据必须等于重新生成的结果
python3 courses/exercise-dataset/build.py --json       # 计数 + 每个文件的字节数

# 校验（结构 + 来源映射 + 解答 + 漂移）
python3 courses/exercise-dataset/check.py
python3 courses/exercise-dataset/check.py --json
python3 courses/exercise-dataset/check.py --selftest   # 判据通道自检

# 取一道题，组装成可直接判卷的文件
python3 courses/exercise-dataset/compose.py \
  set-theory/I.1/unit01-sets-membership/mem_of_subset \
  --out /tmp/template.sokonanoda
python3 courses/exercise-dataset/compose.py \
  set-theory/I.1/unit01-sets-membership/mem_of_subset --solution \
  --out /tmp/solution.sokonanoda

scripts/soko grade --root courses/set-theory /tmp/template.sokonanoda --json  # 有 exercise.open
scripts/soko grade --root courses/set-theory /tmp/solution.sokonanoda --json  # 0 open / 0 diagnostic
```

## 记录字段

```json
{
  "id": "set-theory/I.1/unit01-sets-membership/mem_of_subset",
  "course": "set-theory",
  "chapter": "I.1",
  "chapter_title": "集合、子集与集合运算",
  "unit": "① 集合与隶属",
  "unit_index": 1,
  "kind": "theorem",
  "name": "mem_of_subset",
  "anonymous": false,
  "exercise_no": 1,
  "title": "元素落在子集里。",
  "prompt": "练习 1（T 类）：元素落在子集里。",
  "hints": [{"label": "思路", "body": "…"}],
  "imports": ["import lib.Logic", "import lib.Set"],
  "template": "theorem mem_of_subset … := by\n  sorry",
  "solution": "theorem mem_of_subset … := by\n  intro x hx\n  exact h …",
  "source": {"file": "courses/set-theory/units/I.1/unit01-sets-membership.sokonanoda", "line": 51},
  "solution_source": {"file": "courses/set-theory/units/solutions/I.1/unit01-solution.sokonanoda", "line": 20}
}
```

- **ID 稳定**：`<course>/[<chapter>/]<unit-stem>/<decl-name>`。改单元标题/编号不换 ID；给练习改名 = 新题。
- `template` / `solution` 只存**该声明**，不存整份单元；完整文件由 `compose.py` 从源文件前缀组装（避免把每个单元复制两遍）。
- `prompt` 取题干最近的练习标签 / 编号小节；没有散文的单元为空串（此时 `title` 回落到声明名），不要用单元标题冒充题目。
- `anonymous: true` 的记录（匿名 `example`）没有声明名，靠 `example-N` 稳定标识。

## 插件怎么用（P3 目标）

1. 读 `data/manifest.json` + `data/<course>.json`，按 `chapter` / `unit` / `kind` 取题。
2. 卡片直接展示 `title` / `prompt` / `hints` / `template`。
3. 代码题判卷：`compose.py <id>` 生成文件 → `scripts/soko grade --json` → 读 `decl.checked` / `exercise.open` / `diagnostic`；**不伪造判卷**。
4. 进度、错题、调度状态放用户目录，不进仓库（与现有插件状态目录一致）。

选择题层（`soko.quiz-bank/1`）是 #96 的 P2；本数据集 v1 只做代码练习。

## 维护约定

- 改任何课程源文件后：`build.py` → `check.py`；提交里同时包含源文件与重建后的 `data/`。
- CI：`gates-course` job（`courses/**` 改动时）跑 `build.py --check` / `check.py` / `--selftest`；本地 `scripts/ci-local.sh` 的 gates 组跑同一条判据。
- 新增课程：在 `model.py` 的 `COURSE_ROOTS` / `COURSE_ORDER` 登记，**不需要**改解析逻辑；课程清单格式 v1（扁平数组）与 v2（volumes → chapters）都支持。
- 生成物不手改（`check.py` 会判出来）；`--selftest` 保证校验器不是空转。
- 本目录 `README.md` / `data/` 不进 `docs-lint` 的文档预算（它在 `courses/` 下，不是 `docs/`）。
