# 课程下载与分发（course-distribution）—— 设计草案

> **状态**：**草案**（2026-10-06 立项；后续还要再调查研究，**未实现**）。
> **归属**：CLI + VS Code 新功能（"下载教程项目"）。
> **本文档是唯一记档点**；定稿后补实现计划。

## 0. 一句话

CLI 与 VS Code 都能**下载教程项目**：默认放到「用户文档目录 / `Sokonanoda` / `<course-id>`」，
把教程（当前 = 卷 I《集合论》）下载/生成到该目录，并自动装好**对应版本**的 CLI。
配套三个待定问题：教程是否独立 repo、是否预编译缓存、是否需要教材市场。

## 1. 现状盘点（量化）

| 项 | 数据 | 含义 |
|---|---|---|
| 课程源码（lib+units+gaps+tools） | ≈ **3.8M** | 独立 repo 极轻 |
| 课程编译缓存 `.sokonanoda` | **1.1G** | 缓存 ≈ 源码 290× |
| 课程规模 | 17 库 + 226 单元 + 113 解答 | `course.json`(schema/name/title/volumes) |
| `sokonanoda.toml` | 已有 `requires=0.82.0`；注释「与将来的独立课程仓同构」 | 独立 repo 是已设想方向 |
| CLI | 已有 `course` 子命令 + `setup/update` + **内嵌下载器**（版本钉链 `$SOKONANODA_VERSION → sokonanoda-version.txt → requires → Cargo.toml`） | 复用下载器 |
| VS Code | 已有 `installCli/uninstallCli` 命令 | 命令可类比添加 |

**关键**：源码 3.8M vs 缓存 1.1G ⇒ 预编译缓存是首次体验的**决定性**因素，也是最大的运维包袱。

## 2. 功能需求

### 2.1 CLI

```
sokonanoda course list              # 列出可用教程（读 catalog）
sokonanoda course get <id>          # 下载/生成到默认目录
```
- 默认目录：`用户文档目录/Sokonanoda/<id>`（macOS `~/Documents`，Windows `~\Documents`，自动探测）。
- 目录已存在且非空 → 询问覆盖，或走 git pull 更新。下载后自动 `setup` 装 `requires` 对应版本 CLI。`--json` 输出供机器读。

### 2.2 VS Code

- 新增命令 `sokonanoda.downloadCourse`：弹**文件选择器选位置**（仿 Lean 扩展 New Project）→ 下拉选教程 → 下载 → 自动 setup → 打开。复用 `installCli` 的下载/装 CLI 逻辑。

### 2.3 catalog（教程清单）

一个 JSON 注册表，每个教程含 `id / name / desc / version / source-url / requires / has-cache`。
阶段一只内置官方 catalog（1 个教程）；设计成**可扩展**（未来可加第三方 registry）。

## 3. 三个待定问题的当前判断

### 3.1 独立 Git Repo —— 方便，但非"纯独立"

- ✅ 课程已是自包含模块根，源码 3.8M，独立可行。
- ⚠ 判卷工具链（`tools/check.py`、判据脚本）在语言仓内 ⇒ 两种形态：**A（推荐）** 课程仓**依赖语言仓**（用 `requires` 钉版本）· **B** 工具链拷进课程仓 ⇒ 两仓维护同一套工具链，违背单一来源。

### 3.2 预编译缓存 —— 刚需，但策略排在 G-68 之后

- 现状：学习者本地编译要生成 1.1G 缓存；**G-68 未修**（共享依赖 5.51× 浪费，rebuild ~222s）⇒ 首次体验差。
- 参照 mathlib：按 git hash 托管 olean 缓存，`lake exe cache get` 拉取。1.1G 分发成本高，且按**平台 + 语言仓版本**都要维护。
- **关键洞察**：学习者主要改 `units/`，`lib/`（17 个稳定库）几乎不改 ⇒ 值得预编译的**只有 lib 层**。
- **决策依赖 G-68**：先修好 G-68（模块级缓存复用）⇒ 首次 build 大幅变快 ⇒ 也许**不需要分发 1.1G 缓存**。
  ⇒ **缓存策略排到 G-68 之后定，不现在做 1.1G 分发机制**。

### 3.3 教材市场 / 怎么知道下载哪些 —— 现阶段不做"市场"，按可扩展设计

- Lean 生态无真正"教材市场"，只有社区项目集合（leanprover-community）= **手工维护的注册表**。
- 对"只有一个大教程"阶段，市场 = 过度设计（需托管/审核/版本发现/质量门禁）。
- 架构上留好接口：catalog = 可扩展注册表；阶段一内置官方条目，阶段二开放第三方 registry。

## 4. 分阶段方案

| 阶段 | 内容 | 依赖 |
|---|---|---|
| **0** | CLI `course list` + `course get <id>`，默认 ~/Documents/Sokonanoda；内置 catalog | 无（复用下载器） |
| **1** | VS Code `downloadCourse`（文件选择器 + 自动 setup + 打开） | 阶段 0 |
| **2** | 集合论拆独立课程仓（A 形态）；**此时才定缓存策略** | **先修 G-68** |
| **3**（可选） | 开放 catalog / 第三方 registry | 用户拍板 |

## 5. 待调研清单（后续）

- [ ] 独立 repo 的判卷工具链归属（A/B 形态的落地细节）· catalog 的托管位置与更新机制。
- [ ] G-68 修复后首次 build 实测耗时 ⇒ 定缓存策略（不分发 / 只分发 lib / 全量）。
- [ ] 课程版本与语言仓版本钉的关系（`requires` 如何跨仓维护）。
- [ ] 教材市场形态调研（leanprover-community 的组织、是否有可借鉴的注册表）。
- [ ] Windows/macOS 默认文档目录探测的实现细节。

## 6. 关键决策点（待用户拍板）

1. **命令命名**：`course get`（并入现有 course）还是独立子命令。
2. **默认目录**：`~/Documents/Sokonanoda/<id>` 是否 OK。
3. **独立仓形态**：A（依赖语言仓）还是 B（自包含工具链）。
4. **缓存策略**：是否同意**先修 G-68、再定缓存**（不现在做 1.1G 分发）· **市场范围**：阶段一只做 catalog、不做市场。

## 7. 参考（外部调研，2026-10-06）

- Lean 4 `lake new`/`lake init`：模板脚手架新项目；VS Code Lean 扩展有 "New Project"（文件选择器 + lake init + update）。
- mathlib 预编译：按 git hash 命名的 olean 缓存（Azure 托管）；leanprover-community = 社区项目集合（手工注册表，非自动化市场）。