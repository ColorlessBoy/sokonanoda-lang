#!/usr/bin/env bash
# new-course-repo.sh —— 生成「独立课程仓」骨架（P0.0 / 设计 §3.4–§3.5）
#
# 用法：
#   scripts/new-course-repo.sh <目标目录> [--name <仓库名>] [--version <工具链版本>] [--no-git]
#
# 产物 = 一个自包含的课程仓库骨架：README/AGENTS/清单/共享库/单元目录/缺口发现目录/
# vendored 启动器/本地门禁/CI。生成后它**不再依赖本仓库的源码**，只依赖已发布的
# CLI/LSP（版本钉在 sokonanoda-version.txt）。
#
# 版本钉由 WO-001 落地：vendored 的 scripts/soko 读本仓库的 sokonanoda-version.txt
# （次选 sokonanoda.toml 的 requires）⇒ 本仓库零 cargo、零 SOKONANODA_BIN 自举。
#
# 设计与边界：docs/design/teaching-project.md §3。

set -euo pipefail

TARGET=""
NAME=""
VERSION=""
GIT=1
while [ $# -gt 0 ]; do
  case "$1" in
    --name) NAME="${2:?--name 需要一个值}"; shift 2 ;;
    --version) VERSION="${2:?--version 需要一个值}"; shift 2 ;;
    --no-git) GIT=0; shift ;;
    -h|--help) sed -n '2,16p' "$0"; exit 0 ;;
    *) TARGET="$1"; shift ;;
  esac
done
[ -n "$TARGET" ] || { echo "用法：scripts/new-course-repo.sh <目标目录> [--name N] [--version V] [--no-git]" >&2; exit 2; }

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$HERE/.." && pwd)"
[ -n "$NAME" ] || NAME="$(basename "$TARGET")"
if [ -z "$VERSION" ]; then
  VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$REPO_ROOT/Cargo.toml" | head -1)"
fi
[ -n "$VERSION" ] || { echo "无法确定工具链版本，请用 --version 指定" >&2; exit 2; }

if [ -e "$TARGET" ] && [ -n "$(ls -A "$TARGET" 2>/dev/null || true)" ]; then
  echo "目标目录已存在且非空：$TARGET" >&2; exit 2
fi
mkdir -p "$TARGET"

echo "== 生成课程仓骨架：${TARGET}（name=$NAME, toolchain=${VERSION}）=="
mkdir -p "$TARGET"/{lib,units/solutions,gaps/{found,repro},scripts,.github/workflows}

# ── 版本源（启动器的版本源链第一站）──────────────────────────────────────
printf '%s\n' "$VERSION" > "$TARGET/sokonanoda-version.txt"

# ── 共享库（唯一真相；单元画布 import 它）────────────────────────────────────
# 布局：**单模块根**（仓库根的 sokonanoda.toml），库在 lib/ 下 ⇒ 模块名 = 路径，
# 即 `import lib.Logic` / `import lib.Set`。刻意不给 lib/ 自己的 sokonanoda.toml：
# 两个模块根会让 import 名字随入口位置变化（同一段 import 在 lib/ 内是 `Logic`、
# 在外面是 `lib.Logic`），课程内容不该有这种双份真相。
# 注意 G-12：今天机器上用**相对路径**判卷时，祖先清单会让模块根退化成空路径，
# 所以 scripts/check-course.py 一律传绝对路径（见语言仓 docs/gaps/）。

cat > "$TARGET/lib/Logic.sokonanoda" <<'EOF'
-- Logic.sokonanoda —— 逻辑骨架（命题、真伪、联结词）。
-- 教学口径：先立公理三件套，等语言能力到位再升级成 inductive（对照单元⑨ Or 的做法）。

axiom True : Prop
axiom True.intro : True

axiom False : Prop
axiom False.rec : (C : Prop) -> False -> C

axiom And : Prop -> Prop -> Prop
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b
axiom And.left : (a : Prop) -> (b : Prop) -> And a b -> a
axiom And.right : (a : Prop) -> (b : Prop) -> And a b -> b

inductive Or (A B : Prop) : Prop
ctor inl (a : A) : Or A B
ctor inr (b : B) : Or A B
end

def Not (A : Prop) : Prop := A -> False
def Iff (A B : Prop) : Prop := And (A -> B) (B -> A)
EOF

cat > "$TARGET/lib/Set.sokonanoda" <<'EOF'
-- Set.sokonanoda —— 集合论骨架：集合 = 论域上的谓词。
-- 记法对照（语言还没有 notation，全部用点名形式；G-04 修好后逐条补记法）：
--   x ∈ A      ↔  Set.mem α x A      ↔  A x
--   A ⊆ B      ↔  Set.subset α A B
--   ∅          ↔  Set.empty α
--   A ∪ B      ↔  Set.union α A B
--   𝒫 A        ↔  Set.power α A
import lib.Logic

def Set (α : Type) : Type := α -> Prop

def Set.mem (α : Type) (a : α) (A : Set α) : Prop := A a

def Set.subset (α : Type) (A B : Set α) : Prop := forall (x : α), A x -> B x

def Set.empty (α : Type) : Set α := fun (x : α) => False

-- 注意：Eq 的宇宙参数要显式写（α : Type = Sort 1 ⇒ Eq.{1}）；这是单元②教过的坑。
def Set.singleton (α : Type) (a : α) : Set α := fun (x : α) => Eq.{1} α x a

def Set.union (α : Type) (A B : Set α) : Set α := fun (x : α) => Or (A x) (B x)

def Set.inter (α : Type) (A B : Set α) : Set α := fun (x : α) => And (A x) (B x)

def Set.power (α : Type) (A : Set α) : Set (Set α) := fun (B : Set α) => Set.subset α B A

-- 库自带的两条"已经证好"的定理，给学习者当范例（他们的练习从 units/ 开始）。
theorem Set.subset_refl (α : Type) (A : Set α) : Set.subset α A A :=
  fun (x : α) => fun (hx : A x) => hx

theorem Set.subset_trans (α : Type) (A B C : Set α)
    (h1 : Set.subset α A B) (h2 : Set.subset α B C) : Set.subset α A C :=
  fun (x : α) => fun (hx : A x) => h2 x (h1 x hx)

theorem Set.subset_union_left (α : Type) (A B : Set α) : Set.subset α A (Set.union α A B) :=
  fun (x : α) => fun (hx : A x) => inl (A x) (B x) hx
EOF

cat > "$TARGET/lib/Demo.sokonanoda" <<'EOF'
-- Demo.sokonanoda —— 共享库的自检入口：import 全部模块并真的判卷。
-- 课程仓的本地门禁会把这条文件也跑一遍（lib 坏了要第一时间知道）。
import lib.Logic
import lib.Set

theorem demo_and_comm (A : Prop) (B : Prop) (h : And A B) : And B A :=
  And.intro B A (And.right A B h) (And.left A B h)

theorem demo_subset_chain (α : Type) (A B C : Set α)
    (h1 : Set.subset α A B) (h2 : Set.subset α B C) : Set.subset α A C :=
  Set.subset_trans α A B C h1 h2
EOF

# ── 清单与单元占位 ──────────────────────────────────────────────────────────
cat > "$TARGET/course.json" <<'EOF'
[]
EOF
printf '# 单元画布放这里：unitNN-<slug>.sokonanoda；解答放 solutions/。\n' > "$TARGET/units/README.md"
touch "$TARGET/gaps/found/.gitkeep" "$TARGET/gaps/repro/.gitkeep"

# ── 项目清单 ────────────────────────────────────────────────────────────────
cat > "$TARGET/sokonanoda.toml" <<EOF
# 课程仓的项目清单：模块根 = 本文件所在目录（import 从仓库根解析）。
name = "$NAME"
requires = "$VERSION"   # 工具链版本钉：与二进制不一致时判卷只给 warning
EOF

# ── vendored 启动器 ─────────────────────────────────────────────────────────
cp "$REPO_ROOT/scripts/soko" "$TARGET/scripts/soko"
chmod +x "$TARGET/scripts/soko"

# ── 本地门禁 ────────────────────────────────────────────────────────────────
cat > "$TARGET/scripts/check-course.py" <<'PYEOF'
#!/usr/bin/env python3
"""课程仓本地门禁：判卷全部单元 + 共享库自检 + 汇总。

为什么不用 `sokonanoda course`：见语言仓 docs/gaps/ 的 G-06（它只按单文件编译，
不认 import）。这里逐单元走项目模式判卷，判据用 `grade` 的退出码
（0 = 全 checked 或有合法 open；1 = 解析/elaborate/内核拒绝）——判据仍只看 `grade`；
语言仓 G-10 已修（≥0.59.0）：`query check` 对解析失败也带 parse 诊断 + exit 1，
可以拿它交叉复核。

为什么一律传**绝对路径**：G-12——用相对路径判一个位于"有清单的仓库根的子孙目录"里的
文件时，模块根会退化成空路径，凡有 import 的文件全部报 `import-not-found`。
绝对路径能绕过（相对/绝对的行为差异本身就是那条缺口的证据）。

用法：python3 scripts/check-course.py [--json]
环境：SOKONANODA_BIN 可显式指定二进制（人工兜底；版本钉见 sokonanoda-version.txt）。
退出码：0 全绿 / 1 有单元被判负 / 2 用法或前置缺失。
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOKO = ROOT / "scripts" / "soko"


def resolve_soko() -> list[str] | None:
    """返回可用的判卷命令前缀；不可用时打印诊断并返回 None。

    启动器自己就是守卫（WO-001）：`version --json` 的 `version` 非空 ⇒ 版本源链
    （`sokonanoda-version.txt` → `requires` → `Cargo.toml`）解析出了钉，且陈旧缓存
    会被启动器自己拒绝（exit 3 + 人话）——这里不需要第二套判据。
    """
    if os.environ.get("SOKONANODA_BIN"):
        return [str(SOKO)]
    try:
        out = subprocess.run(
            ["node", str(SOKO), "version", "--json"],
            capture_output=True, text=True, timeout=60, cwd=ROOT,
        )
        info = json.loads(out.stdout or "{}")
        if info.get("version"):
            return [str(SOKO)]
    except Exception:
        pass
    node = shutil.which("node")
    if node is None:
        print("error: 需要 node（scripts/soko 是零依赖 Node 启动器）", file=sys.stderr)
        return None
    print(
        "error: 启动器解析不出工具链版本。\n"
        f"       先跑：node {SOKO} setup（按 {ROOT / 'sokonanoda-version.txt'} 下载锁定版本；\n"
        f"       没有该文件时回落 {ROOT / 'sokonanoda.toml'} 的 requires）\n"
        f"       人工兜底：SOKONANODA_BIN=<绝对路径> python3 {sys.argv[0]}",
        file=sys.stderr,
    )
    return None


def units() -> list[dict]:
    data = json.loads((ROOT / "course.json").read_text(encoding="utf-8"))
    return [u for u in data if u.get("file")]


def grade(cmd: list[str], path: Path) -> tuple[int, dict]:
    proc = subprocess.run(
        [*cmd, "grade", str(path)], capture_output=True, text=True, cwd=ROOT,
    )
    counts = {"checked": 0, "open": 0, "reduced": 0, "typed": 0}
    for line in proc.stdout.splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        kind = event.get("type")
        if kind == "decl.checked":
            counts["checked"] += 1
        elif kind == "exercise.open":
            counts["open"] += 1
        elif kind == "expr.reduced":
            counts["reduced"] += 1
        elif kind == "expr.typed":
            counts["typed"] += 1
    return proc.returncode, counts


def main() -> int:
    as_json = "--json" in sys.argv
    cmd = resolve_soko()
    if cmd is None:
        return 2

    targets: list[tuple[str, Path]] = [("lib/Demo", ROOT / "lib" / "Demo.sokonanoda")]
    for unit in units():
        targets.append((f"unit {unit.get('unit', '?')} {unit.get('title', '')}", ROOT / unit["file"]))

    rows, failed = [], 0
    for label, path in targets:
        if not path.exists():
            rows.append({"label": label, "file": str(path.relative_to(ROOT)), "status": "missing"})
            failed += 1
            continue
        code, counts = grade(cmd, path)
        status = "ok" if code == 0 else "rejected"
        failed += code != 0
        rows.append({"label": label, "file": str(path.relative_to(ROOT)), "status": status, **counts})

    if as_json:
        print(json.dumps({"schema": "soko.course.local/1", "units": len(units()), "targets": rows,
                          "failed": failed}, ensure_ascii=False, indent=2))
    else:
        print(f"{'状态':<9}{'checked':>8}{'open':>6}  目标")
        for row in rows:
            print(f"{row['status']:<9}{row.get('checked', 0):>8}{row.get('open', 0):>6}  {row['file']}")
        total_checked = sum(r.get("checked", 0) for r in rows)
        total_open = sum(r.get("open", 0) for r in rows)
        print(f"\n共 {len(rows)} 个目标（含库自检）—— {total_checked} checked · {total_open} open · "
              f"{failed} 个被判负")
        if total_open == 0 and len(units()) > 0:
            print("提示：单元里一个 open 练习都没有？确认画布没有被写满解答。")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
PYEOF
chmod +x "$TARGET/scripts/check-course.py"

# ── CI（G-11 修好后即可用；现在显式标注被什么挡住）────────────────────────────
cat > "$TARGET/.github/workflows/ci.yml" <<'EOF'
# 课程仓 CI：零 cargo —— 按 sokonanoda-version.txt 锁定下载 CLI，然后判卷全部单元。
name: course
on: [push, pull_request]
jobs:
  grade:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: '20' }
      - name: 安装工具链（版本钉 = sokonanoda-version.txt）
        run: node scripts/soko setup
      - name: 判卷全部单元
        run: python3 scripts/check-course.py
EOF

cat > "$TARGET/.gitignore" <<'EOF'
.DS_Store
*.vsix
__pycache__/
EOF

cat > "$TARGET/README.md" <<EOF
# $NAME

「集合论」教程（sokonanoda 教学项目，卷 I）。本仓库是**独立课程仓**：
只消费已发布的 sokonanoda 工具链，不含语言源码。

- 工具链版本钉：\`sokonanoda-version.txt\`（当前 ${VERSION}）
- 怎么判卷（零 cargo、零 SOKONANODA_BIN）：

\`\`\`bash
node scripts/soko setup            # 按版本钉下载 CLI + LSP 到缓存
python3 scripts/check-course.py    # 判卷全部单元（判据 = grade 退出码）
\`\`\`

- 目录：\`lib/\` 共享库（唯一真相）· \`units/\` 单元画布与解答 · \`gaps/\` 撞到的语言缺口
- 写作与判卷纪律见 \`AGENTS.md\`；缺口台账协议见语言仓
  \`docs/design/teaching-project.md\` §6 与 \`docs/gaps/\`
EOF

cat > "$TARGET/AGENTS.md" <<'EOF'
# AGENTS.md —— 课程线 agent 手册

你在**课程仓**里工作，目标是写出可用真内核判卷的集合论教材。语言仓（sokonanoda-lang）
不在这里；判卷只用已发布工具链。

## 每个单元的 DoD（缺一条不算完成）

1. 大纲条目：单元号 / 靶子（如 Tao §3.1）/ 先修 / 练习类型配额；
2. 画布 `units/unitNN-*.sokonanoda`：演示 + 练习（`sorry`），能 import 共享库就 import；
3. `-- soko:hint` 三段（思路 / 目标形态 / 关键件），**关键件只写触发条件 + 引理名**；
4. 解答 `units/solutions/unitNN-*-solution.sokonanoda`：洞全填、0 拒绝；
5. `course.json` 加一行（单元号、文件、标题、先修）；
6. `python3 scripts/check-course.py` 全绿；
7. README/大纲文档同步；
8. **撞到的语言缺口登记到 `gaps/found/`（含最小复现 `gaps/repro/`）**，再 push 给语言仓；
9. 收尾跑一次 `python3 scripts/check-course.py --json` 并把计数记进本轮说明。

## 判卷纪律

- 判据永远走内核：`node scripts/soko grade <file>`；退出码 0 = 全 checked 或有合法 open，
  1 = 有拒绝（解析/elaborate/内核）。
- **判卷一律给绝对路径**：语言仓 G-12——相对路径 + 祖先清单会让模块根退化成空路径，
  凡 import 的文件全部报 `import-not-found`（`scripts/check-course.py` 已经这么做）。
- **判据只看 `grade` 的退出码**（`scripts/check-course.py` 就是这么做的）；语言仓
  G-10 已修（≥0.59.0）：`query check` 对解析失败也带 parse 诊断 + exit 1，可以拿它
  与 `grade` 交叉复核。
- 想读某处还差什么：`node scripts/soko query state --file <file> --line L --col C`；
  想读洞清单：`query holes`；想读提示阶梯：`query hints --line L --col C`。
- 不要用文本比对判对错；不要手改既有库的证明来过门禁。

## 缺口登记（本仓库 → 语言仓）

在 `gaps/found/GNN-<slug>.json` 写一条（字段照语言仓 `docs/gaps/ledger.jsonl`），
复现放 `gaps/repro/GNN-<slug>.sokonanoda`，然后
`python3 <语言仓>/scripts/gap.py push --to <语言仓>`（P0.2 产出）或人工复制。
**每条缺口必须带：最小复现 + 期望的 Lean 4 语义 + 今天的表现 + 绕行代价。**
EOF

if [ "$GIT" = "1" ]; then
  ( cd "$TARGET" && git init -q && git add -A && git -c user.name=course-agent -c user.email=course-agent@local commit -qm "chore: 课程仓骨架（由 sokonanoda-lang/scripts/new-course-repo.sh 生成）" )
  echo "== git init + 首个 commit 完成 =="
fi

echo
echo "下一步："
echo "  cd $TARGET"
echo "  node scripts/soko setup && python3 scripts/check-course.py"
echo "  （版本钉 = sokonanoda-version.txt；需要完全离线时用 SOKONANODA_BIN=<绝对路径> 兜底）"
