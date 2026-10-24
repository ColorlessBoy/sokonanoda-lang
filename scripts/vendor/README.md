# scripts/vendor —— 仓库自带的第三方纯 Python 依赖

| 包 | 版本 | 许可证 | 为什么在这里 |
| --- | --- | --- | --- |
| [`yaml`](https://pyyaml.org/)（PyYAML） | **6.0.3** | MIT（见 `LICENSE-PyYAML`） | `scripts/{check-ci-wiring,ci-push-range,ci-yml-lint}.py` 要读 GitHub workflow 的 YAML。系统 `pyyaml` 是**未声明的外部依赖**（CI 镜像恰好自带、开发机不一定有）——2026-10-24 实测：本机缺它 ⇒ 推送前快层整层红 ⇒ 只能 `--no-verify` 绕过整道门禁。 |

约定：

* **只用纯 Python 实现**（`cyaml` C 扩展已删，`yaml/__init__.py` 的 try/except 会回退）；
* 唯一入口是 `scripts/_yaml.py`（把 `vendor/` 放进 `sys.path` 再 `import yaml`），
  **禁止**任何脚本直接 `import yaml`；
* 升级时整目录替换 + 同步改本表与 `scripts/_yaml.py` 的版本号，并跑
  `python3 scripts/check-ci-wiring.py --selftest` 与 `python3 scripts/ci-yml-lint.py`。

维护者：仓库本体。上游版权声明随 `LICENSE-PyYAML` 一并保留。
