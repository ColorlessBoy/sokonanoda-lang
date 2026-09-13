# 当前状态与进度日志（agents 先读这里）

> 快照：2026-09-13（第四十二轮：文档治理——归档/去漂移/发布口径统一）
> 历史轮次（1–39）见 `docs/STATUS-ARCHIVE.md`。
> 仓库：`sokonanoda-lang`；权威计划 = `ROADMAP.md`；**用户要求总账 = `REQUIREMENTS.md`（先读）**；
> **文档地图 = `docs/README.md`**（入口/权威在仓库根，开发者参考在 `docs/` 顶层，
> 设计在 `docs/design/`，调研笔记在 `docs/notes/`）；
> 架构/内核 = `docs/architecture.md`；协议 = `docs/protocol.md`；测试地图 = `docs/TESTING.md`；
> 经验台账 = `docs/LESSONS.md`；CI 失败台账 = `docs/CI-FAILURES.md`；发布 = `docs/RELEASE.md`；
> agent 入口 = `AGENTS.md` + `skills/`。

## 一句话

`.sokonanoda` = **纯声明式教学文件（无 `#` 命令）+ 完整 sokonanoda 内核 + LSP 反馈通道**。
练习 = 带 `sorry` 洞的 `def name : T` / `theorem name : T` / `example : T` 声明。
CLI/REPL 的 `#check` 等只是调试/自测工具，不是文件格式。

## 本轮进度（2026-09-13，第四十二轮：文档治理）

> 用户要求：当前问题解决后，清理整理项目文档，防技术债，让后续的人 / code
> agent 高效接手。三路并行只读审计（根目录五份入口 / docs 顶层 / design+notes
> 全部 35 篇）后集中修复约 35 处。

1. **发布口径统一**：`docs/RELEASE.md` 主口径改为「bump → push main → auto-tag
   自动发版」，手动推 tag 降为应急；`AGENTS.md`/`vscode-dev-guide.md`/
   `docs/README.md` 同步（此前四处口径只有 skill 写对了）。
2. **矛盾清零**：TESTING「LSP 零测试/assumption 文本比对待替换/known_missing
   未闭环」三处自相矛盾；architecture §5.5 与 §8.8 矛盾；REQUIREMENTS §2.8
   过时括号注；teaching-session「单元⑤待支持」；binary-cli 的 `env` 父命令与
   `minreq` 两处失实（横幅更正）。
3. **状态横幅**：12 篇 design/notes 补 as-built / 快照横幅（term-apply 自称
   「只落设计」实则 0.18.0 已发布——最严重漂移）；docs/README 地图三条
   「待实施」改「已实现/已上线」。
4. **协议补全**：protocol.md 增 `soko/version` 小节（five custom requests）；
   `LSP_CUSTOM_METHODS` 词汇表补第 5 项（恢复守护）。
5. **STATUS 归档**：1–39 轮原文移 `docs/STATUS-ARCHIVE.md`，本文件只留
   最近 3 轮；gen-site-data.py 只解析最新轮标题，归档无破坏（已验证）。
6. **杂项**：`.gitignore` 加 `.workbuddy/`；README 门面补值位三关键字示例；
   ROADMAP §8「执行 M0」等陈旧定位更新、I10/I12/I11-S1 打勾标注版本；
   REQUIREMENTS §4 欠账更新为当前真实数字（lsp/lib.rs 3949 行、
   compile/tests.rs 3564 行）、§8 路线改指 I11 余项、§9 补四十二。

## 本轮进度（2026-09-13，第四十一轮：发布链自动化闭环）

> 用户复盘发布流程后发现三个流水线缺口，全部修复并落地自动化。本机 `gh`（owner
> 权限）可用后，此前「只能网页手点」的动作全部自动化了。

1. **auto-tag 自动发版**（`ci.yml` 新 job）：main 上 lint/test 全绿后，若
   `Cargo.toml` 版本还没有对应 v* tag → 打 tag 并 `workflow_dispatch`
   release.yml。机制：GITHUB_TOKEN 推的 tag 不触发其它 workflow（防递归），
   `workflow_dispatch` 是例外；dispatch 需要 `actions: write`（首跑 403 教训）。
   幂等：tag 已存在即跳过；两处版本不一致直接 fail。
2. **release.yml 两处 `if: github.event_name == 'push'` 改为按 ref 判**——
   dispatch 进来核心步骤（创建 Release/上架 Marketplace）被静默 skipped 而
   job 依然绿，v0.20.0 首发实况。
3. **pages 门禁改鉴权 `gh api`**——未鉴权 curl 的假 404/403 让门禁误判
   「未启用」→ 部署步骤全 skipped，站点迟迟不上线。
4. **发布结果**：v0.20.0（tag 强移到含修复的 commit，版本契约核验）→ Release
   资产 25 个（9 VSIX + 8 LSP tar + 8 CLI tar）✓；Marketplace 已收录 0.20.0
   （首跑遇 Azure gallery 503，`gh run rerun --failed` 恢复后成功）✓；站点
   https://colorlessboy.github.io/sokonanoda-lang/ 上线（About 的 homepage
   指向它）✓；About 三件套（description/website/topics）已用 gh repo edit
   填好。
5. **教训落台账**：`docs/CI-FAILURES.md` +2（skipped 步骤把 job 抬绿；gallery
   503）；`skills/sokonanoda-ci` §1/§2.1 +4 行。

## 本轮进度（2026-09-13，第四十轮：三个大方向落地）

> 承接第三十九轮的设计（`docs/design/term-apply.md` / `real-input-tests.md` / `site.md`），
> 用户确认「三条串行进行」后按依赖顺序执行：I11-S0 → I11-S1 → I10 → I12。

1. **I11-S0 真人输入脚本基建**（`lsp/src/testutil.rs`）：`TypedStep`
   `(offset, delete, insert)`（纯输入/纯删除/**选中重打**三种形态）+ `type_step`
   （一步一次 didChange + 等诊断落地，`refresh` 每次都发诊断 → **零 sleep**）+
   `char_steps`。两条用例：逐字符敲 `intro` 只在整词给展开项；「选中重打 + 折行」
   每个中间态都能拿到展开项。**教训**：最初设计成「跑完整条脚本返回各步快照」，
   而服务器只持有最新状态——据此写出的是**看起来会通过的假测试**，已改为
   「一步一断言」并写进设计文档 §2。
2. **I11-S1 修复 `by apply` 多子目标的 inlay 类型错配**：这些子目标在源码里
   **共用同一个位置**（`assemble` 给每个叶子洞传同一个 `hole_span`），旧实现用
   span 反查 `sub_goals`，两处都显示第一个子目标的类型（实测 `[": p", ": p"]`）。
   改为**按洞的位置顺序**对齐（红→绿）。`soko/nextHole` 无法在同址子目标间导航
   属**确认为限制**（伪造互异 offset 会让 documentHighlight/selectionRange 出假
   范围），已写进 `docs/protocol.md`。
3. **I10 值位 `apply`**：`theorem t (h : Q -> P) : P := apply h` → `h sorry`。
   设计里的关键取舍全部落地——内核只用来**推断**被应用名字的类型（`judge_infer`，
   front 侧无类型表可用），telescope 机械抽到 `crates/front/src/spine.rs` 与
   tactic `apply` **共用**；局部假设（最常用场景）经 `open_goal` 的局部模板覆盖层
   被 spine 走查认出；合成洞一律走 `judge_terms` 绕开 `judge_hole_fill` 的
   sorry 守卫；类型参数由目标实参经 σ 填充；实参用 `parse_app`（括号界定范围）；
   洞 span 覆盖整个 `apply <term>`。LSP 侧 `intro_at` 泛化为 `keyword_at`（第二
   个调用方出现），hover/补全/`sokonanoda.expandApply` 全套接入。两个新错误码。
4. **I10-S4 课程**：第 6 单元新增「值位 `apply`」一节（三种写法对照 + 2 道练习 +
   钥匙），英文镜像非逐字对齐。golden **刻意**变更：unit6 事件计数
   `(13, 6, 0)` → `(17, 10, 0)`，汇总 `33/27` → `37/31`（`course.rs` 与
   `course_status.rs` 同步）。
5. **I12 官网**：零构建静态 `site/`（9 页 + CSS + `site/data/site.json`）+
   `scripts/gen-site-data.py`（python3 标准库：版本←Cargo.toml、单元←course.json、
   轮次←STATUS.md）+ `scripts/check-site.py`（站内链接 + 禁止写死版本号）+
   `.github/workflows/pages.yml`（configure-pages / upload-pages-artifact /
   deploy-pages）。**一次性前置动作仍需仓库拥有者**：Settings → Pages → Source =
   GitHub Actions。顺带修 4 处文档漂移（README 写死 `V=0.9.0`、课程单元数
   5 vs 6 的三处口径）。
6. **版本**：0.17.0 → **0.18.0**（新语法 = 新能力 → minor）。
