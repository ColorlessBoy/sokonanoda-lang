#!/usr/bin/env python3
"""`.github/workflows/*.yml` 的静态校验（2026-09-26 事故 ✓）。

**事故** ✓：我给 `ci.yml` 加一步时，把新的 `run:` 写进了**上一个 step 的映射里** ✗
⇒ 同一个 step 出现**两个 `run:` 键** ✗ ⇒
**`yaml.safe_load` 静默取最后一个** ✓（所以我本地"验过了"是假的 ✗），
**而 GitHub 拒绝整个 workflow** ✗ ⇒ 整轮 **0 个 job、0 秒失败** ✗；
更糟的是：**若 GitHub 接受，`notation-lint` 会被顶掉** ✗（**悄悄关掉一条门禁** ✗）。

⇒ 所以这个脚本用**禁止重复键的严格 loader** ✓ ——
`yaml.safe_load` 不够用 ✗，它不报重复键 ✗。

判据：
  ① 所有 workflow 能被**严格 loader**（禁重复键）解析 ✓；
  ② 每个 job 至少有一步、每步有 `run` 或 `uses` ✓；
  ③ 每个 job 的 `steps` 是列表且非空 ✓。

退出码：0 = 通过 / 1 = 判红 / 2 = 用法或环境错。
"""
import sys
from pathlib import Path

try:
    import yaml
except ImportError:  # pragma: no cover
    print("需要 pyyaml ✗", file=sys.stderr)
    sys.exit(2)


class StrictLoader(yaml.SafeLoader):
    """禁止映射里的重复键 —— `yaml.safe_load` 会静默取最后一个 ✗。"""


def _no_duplicates(loader, node, deep=False):
    seen = set()
    for key_node, _ in node.value:
        key = loader.construct_object(key_node, deep=deep)
        if key in seen:
            raise yaml.constructor.ConstructorError(
                None, None, f"重复键 `{key}`", key_node.start_mark)
        seen.add(key)
    return yaml.SafeLoader.construct_mapping(loader, node, deep)


StrictLoader.add_constructor(
    yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, _no_duplicates)


def main() -> int:
    root = Path(".github/workflows")
    if not root.is_dir():
        print("找不到 .github/workflows ✗", file=sys.stderr)
        return 2
    files = sorted(list(root.glob("*.yml")) + list(root.glob("*.yaml")))
    if not files:
        print("没有 workflow 文件 ✗", file=sys.stderr)
        return 2

    bad = []
    for f in files:
        try:
            doc = yaml.load(f.read_text(encoding="utf-8"), Loader=StrictLoader)
        except yaml.YAMLError as e:
            mark = getattr(e, "problem_mark", None)
            where = f":{mark.line + 1}:{mark.column + 1}" if mark else str(f)
            bad.append(f"{where} YAML 不合法 ✗：{getattr(e, 'problem', e)}")
            continue
        if not isinstance(doc, dict) or "jobs" not in doc:
            bad.append(f"{f} 没有 jobs ✗")
            continue
        for job_name, job in (doc.get("jobs") or {}).items():
            steps = (job or {}).get("steps")
            if not steps:
                bad.append(f"{f} 的 job `{job_name}` 没有 steps ✗")
                continue
            for i, st in enumerate(steps):
                if not isinstance(st, dict):
                    bad.append(f"{f} 的 job `{job_name}` 第 {i+1} 步不是映射 ✗")
                elif "run" not in st and "uses" not in st:
                    bad.append(f"{f} 的 job `{job_name}` 第 {i+1} 步既无 run 也无 uses ✗")

    if bad:
        print(f"ci-yml-lint：{len(bad)} 条不通过 ✗", file=sys.stderr)
        for b in bad[:20]:
            print(f"  - {b}", file=sys.stderr)
        return 1
    n_jobs = sum(len((yaml.load(f.read_text(encoding='utf-8'), Loader=StrictLoader)
                      .get("jobs") or {})) for f in files)
    print(f"ci-yml-lint ✓ {len(files)} 个 workflow · {n_jobs} 个 job · 无重复键 ✓")
    return 0


if __name__ == "__main__":
    sys.exit(main())
