"""仓库内唯一的 YAML 入口 —— **vendored 纯 Python PyYAML 6.0.3（MIT）**。

为什么不用系统 `pyyaml`：它是**未声明的外部依赖**（CI 的 ubuntu 镜像恰好自带，
开发机不一定有）——2026-10-24 实测：本机缺它 ⇒ `check-ci-wiring.py` rc=2 ⇒
`ci-local.sh --fast` 整层红 ⇒ 只能 `git push --no-verify` **把整道推送前门禁
一起绕过去**（当天的 queue-lint 红就是这么漏出去的）。

vendored 版本是纯 Python 实现（不含 `cyaml` C 扩展），零编译、零网络、跨平台；
版本与来源见 `scripts/vendor/README.md`，许可证见 `scripts/vendor/LICENSE-PyYAML`。
"""

from __future__ import annotations

import os
import sys

_VENDOR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "vendor")
if _VENDOR not in sys.path:
    sys.path.insert(0, _VENDOR)

import yaml  # noqa: E402  （vendored 包；路径已在上面钉死）

safe_load = yaml.safe_load


def __getattr__(name: str):
    """其余名字（`SafeLoader` / `YAMLError` / `load` …）一律透传给 vendored 包。"""
    return getattr(yaml, name)


def is_vendored() -> bool:
    """调用方确实用的是仓库内那一份（给自检/咬住证明用）。"""
    return os.path.realpath(getattr(yaml, "__file__", "")).startswith(
        os.path.realpath(_VENDOR) + os.sep
    )
