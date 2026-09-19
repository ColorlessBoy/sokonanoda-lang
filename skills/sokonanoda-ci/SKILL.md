---
name: sokonanoda-ci
description: Ship and monitor GitHub Actions for sokonanoda-lang without repeat failures - exact local-verification commands (exit codes, no grep masks), workflow pitfalls (download-artifact v4 layout, tag-pinned workflow files, gh release idempotency, vsce flags), run triage with gh CLI, and the CI-FAILURES.md ledger duty. Use when pushing, tagging, releasing, or debugging CI/Actions failures in this repo.
---

# sokonanoda-ci：GitHub Actions 推送 / 发布 / 排错纪律

> 背景：本仓库 CI 失败率曾偏高——根因几乎都不是代码本身，而是
> 「本地验证与 CI 不一致」和「对 Actions 机制想当然」。本 skill 把
> 踩过的每个坑固化成操作规程；失败台账在 `docs/CI-FAILURES.md`
> （同一类失败不犯第二次）。

## 0. 推送前的本地验证（必须逐条、必须看真退出码）

```bash
# 一条命令跑完 CI 门禁（fmt + clippy + test + playground 锚点 + **课程门禁** +
# **缺口台账门禁**，需要 cargo；后两段还要 python3，探不到即 exit 3——无法判定 ≠ 绿）：
scripts/soko gate; echo "EXIT=$?"

# 或与 CI 完全一致的命令（.github/workflows/ci.yml），顺序执行：
cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check
cargo clippy --workspace --all-targets -q; echo "EXIT=$?"
cargo test --workspace --locked -q; echo "EXIT=$?"
scripts/soko grade playground.sokonanoda --json
```

> **`gate` 与 `grade` 都会用"解析到的 `sokonanoda` 二进制"；`scripts/soko` 只认
> 版本匹配（`--version` 校验）的仓库构建与缓存**，所以先 `cargo build`，
> 否则 gate 会以 exit 3 拒绝跑旧二进制。
> 本机若 Xcode 许可未接受（`xcrun`/`ar` 被系统拦），加
> `DEVELOPER_DIR=/Library/Developer/CommandLineTools` 再跑 cargo（`docs/HANDOVER.md` §5）。

纪律：

1. **退出码必须直接看**：`echo $?` 或 `${PIPESTATUS[0]}`；
   绝不把管道末端（grep/head）的退出码当成 cargo 的。
2. **输出只许 tail/-A 上下文，不许 grep 掩膜**：`grep "^(warning|error)"`
   之类掩膜会吞掉真实警告（crate 路径在 `-->` 行，不在 warning 行）。
3. 教学 crates 的 `[lints.rust] warnings = "deny"` 会让**任何**
   rustc/clippy 警告变成 error——本地裸 clippy 看似只出 warning 的东西
   在 CI 是红。怀疑时不许自我辩解，用 CI 同款命令复现（历史教训：
   `int_plus_one` 假绿，见 CI-FAILURES 2026-09-09）。
4. 临时诊断代码（eprintln/debug 测试）**用完即删**，删完重跑 fmt+clippy。

## 1. Workflow 机制陷阱（全部踩过）

| 陷阱 | 事实 | 规程 |
|---|---|---|
| **课程门禁（卷 I）在 `test` job 的 step 里** | 判据 G1–G5 的唯一真相是 `courses/set-theory/tools/check.py`（python3）；CI 先 `--selftest` 再全量，`SOKONANODA_BIN` 指当轮 `target/debug/sokonanoda`，report 进 `course-gate-report` artifact。**不新建 job** ⇒ 课程红自动挡 `auto-tag` 发布 | 课程红了先看 step summary / artifact，再本地复跑 `scripts/soko gate`（等价；python3 探不到 ⇒ exit 3，不是绿）。设计 `docs/design/course-gate-in-ci.md` |
| **缺口台账门禁是同一个 job 的下一步** | `Gap ledger is consistent (docs/gaps)` step 跑 `scripts/gap.py selftest` + `check`——每条缺口的复现必须与台账 `status` 一致（`fixed` ⇒ 应转绿；`open` ⇒ 应仍复现；`repro_expect` 可显式覆盖，例如 G-01 的「修好 = 判红」） | 红了说明语言变了而台账没跟上：按输出改 `docs/gaps/ledger.jsonl` 的 `status`/`fixed_in`/`repro_expect`/`repro`，别改复现件去迎合旧结论。本地等价命令 `python3 scripts/gap.py check` |
| `download-artifact@v4` 不带 `name:` | 每个 artifact 下载进**同名目录**（`lsp-<target>/`、`sokonanoda-vsix/`），不是平铺 | 引用路径前先确认落盘布局；要平铺用 `pattern:` + `merge-multiple: true` |
| tag 触发的 workflow 用**哪个文件** | 用 **tag 指向的 commit** 上的文件，不是 main 最新 | 修 workflow 后要重跑 release：`git tag -f v<ver> <fix-commit> && git push -f origin v<ver>`；`gh run rerun` 只会重放旧文件 |
| `gh release create` 非幂等 | 已存在 → 422 | 永远 `|| true`（存在即跳过） |
| `gh release upload` 非幂等 | 同名 asset 已存在 → 422 → `bash -e` 全 job 死 | 永远 `--clobber`（重跑覆盖） |
| 强移 tag 前先想清楚 | release 上可能已有首跑传了一半的 asset | 上传步骤必须幂等后再强移 |
| `vsce package --no-dependencies` | 跳过生产依赖收集 → 12 文件/21KB 空壳 VSIX | 冒烟用与 release.yml 相同的命令（无该 flag）；平台包正确基线 ≈327 文件（含 `bin/<target>/`） |
| vsce publish Azure 超时 | `Request timeout: /_apis/gallery` 是**间歇性**网络问题 | publish 步骤已内置 **3 次重试**；仍失败就探活 `extensionquery`（200 即已恢复）后 `gh run rerun <id> --failed`，别改流水线 |
| action 名拼写 | 多个 s / 复数错名 → action 不存在 | 新 action 首次使用先验证存在 |
| run 步骤默认 `bash -e` | 任何一步非零退出 → 整个 step 死 | 想容忍的命令才加 `\|\| true`；别把 `bash -e` 当没有 |
| 平台包 exec 位 | VSIX 的 zip 记录 unix mode；**Windows 上 `vsce package` 会丢执行位**（vsce #152/#512） | 只在 Linux/macOS 打包；stage 时 `chmod 755`；冒烟用 python `zipfile` 断言 `mode & 0o111` |
| 平台包发布顺序 | Marketplace 同版本 universal + 多 target 并存；“Validating”窗口有装错 target 的竞态（vscode#141696） | 先 universal 后 target；每包独立重试；装错反馈先让用户卸载重装 |
| 版本门禁 | tag / `Cargo.toml` / `package.json` 三者不一致时 release 必须 fail（历史上靠人工） | release `package-vsix` 的 version gate + `cargo_and_extension_versions_match` 契约测试双保险；tag 前先 `cargo check` 更新 lock |
| **job success ≠ 关键步骤跑过** | 条件不满足的步骤显示 `skipped`，其余步骤 success 会把 **job 抬绿**（v0.20.0 首发：8 平台构建真跑、Release/上架两步被 `if: github.event_name == 'push'` 静默跳过） | 核对必须逐步骤；`conclusion == "skipped"` 出现在创建/上传/发布步骤 = 发布半坏。条件按 **ref** 判（`startsWith(github.ref,'refs/tags/')`），别按 event 判 |
| GITHUB_TOKEN 推 tag **不触发**其它 workflow | 防递归规则；但 `workflow_dispatch` / `repository_dispatch` 是**例外**，token 触发有效 | 自动发版 = 推 tag + 显式 `gh workflow run release.yml --ref v<tag>`（ci.yml 的 auto-tag job）；dispatch 还需 `actions: write`（只有 contents:write 会 403 "Resource not accessible by integration"） |
| **CI 机器人推的提交没有 check-run** | `GITHUB_TOKEN` 推的提交不触发 workflow（防递归），所以它**没有** check-run：仓库首页/main 的 HEAD 显示黄点 `Expected — Waiting for status to be reported`，`commits/<sha>/status` 是 `pending`——很容易被读成「CI 没成功」（0.58.0 的台账回提交踩过） | 凡「CI 回提交」的设计都要补状态：`gh api repos/$REPO/statuses/<sha> -f state=success -f context=<job>`（job 需要 `statuses: write`），描述里写明它由哪次全绿产生；排查时把「run 失败」「check-run」「commit status」三件事分开看 |
| **发版默认全自动** | bump 两处版本（`Cargo.toml` + `editor/vscode/package.json`）→ push main → `ci.yml` 的 auto-tag 自动打 tag 并 dispatch `release.yml` | 正常发布**不要**手推 tag；手动打 tag 仅应急（tag 触发的 release 用 tag 指向 commit 上的文件）。发布形态与校验见 `docs/RELEASE.md` |
| `upload-artifact@v7` `FinalizeArtifact` **403 Forbidden** | 上传成功后 finalize 被中介拒绝（`(403) Forbidden ... Error from intermediary`）是 GitHub Artifacts 服务**瞬时故障**，内容/代码无关（2026-09-15 v0.39.1：build 失败 → package-vsix/github-release 全 skipped） | 确认是 finalize（非 build/upload 内容）后 `gh run rerun <id> --failed`（下游依赖会随之重跑）；不改流水线 |
| **job 级 `if` 引用 `matrix`** | `jobs.<job_id>.if` 的可用上下文只有 `github`/`needs`/`vars`/`inputs`，**没有 `matrix`**（`runs-on`/`continue-on-error` 才有）。写 `if: matrix.os != 'macos-latest'` 要么按空值求值（该腿在 PR 上也跑），要么被判成未识别命名值让**整个 workflow 校验失败**（一条 job 都不会跑） | 想按矩阵值筛腿：把条件放到 **step 级**（那里有 `matrix`），或拆成**独立的 job** 只用 `github` 条件（`github.event_name == 'push' && github.ref == 'refs/heads/main'`）；改完先推**临时分支**验证 workflow 能被接受（本地 YAML 解析查不出上下文可用性） |
| runner 上的未鉴权 GitHub API 调用会假 404/403 | 匿名额度按 IP 共享，限流/风控返回 403/404，与资源真实状态无关（pages 门禁曾因此误判"未启用"→ 部署全 skipped） | workflow 里查仓库状态一律 `gh api` + `GH_TOKEN: ${{ github.token }}` |

## 2. 触发与监控

```bash
git push origin main   # bump 两处版本后 push = ci auto-tag 自动打 tag + dispatch release（默认发版）
# 手动 tag 仅应急：
git tag v0.4.x && git push origin v0.4.x  # → ci（tag）+ release
gh run list --limit 5                     # 状态总览
gh run view <id> --json jobs --jq '.jobs[] | {name, conclusion}'
gh run view <id> --log-failed | tail -30  # 只看失败 step 的日志尾部
```

**`gh` 未必存在**（2026-09-12 本机实测 `command not found: gh`）。不能因为没装
`gh` 就说"看不了 CI"——公开仓库可用未认证 REST（`curl` + `node`/`python3`
解析）；把下面这段存成习惯（`$SHA`=提交，`$RUN`=run id）：

```bash
NODE=$(command -v node)
# 某提交触发了哪些 run —— head_sha 必须是**完整** SHA（短 SHA 会返回空列表，
# 2026-09-12 实测踩过，很容易误判成"CI 根本没跑"）。先 `SHA=$(git rev-parse HEAD)`。
curl -sS "https://api.github.com/repos/ColorlessBoy/sokonanoda-lang/actions/runs?head_sha=$SHA" \
  | $NODE -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>{const j=JSON.parse(s);
      console.log((j.workflow_runs||[]).map(r=>r.id+" "+r.name+" "+r.status+" "+(r.conclusion||"-")).join("\n"))})'
# 某 run 的 job 与逐 step 状态（判断卡在哪一步）
curl -sS "https://api.github.com/repos/ColorlessBoy/sokonanoda-lang/actions/runs/$RUN/jobs?per_page=30" \
  | $NODE -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>{const j=JSON.parse(s);
      for(const x of j.jobs) console.log(x.name, x.status, x.conclusion, "|",
        x.steps.map(t=>t.number+":"+t.name+"="+t.conclusion).join(", "))})'
```

补充（2026-09-17 本机实测，三坑一起踩过）：

- **环境里的代理可能是坏的**：本机 shell/系统代理都指向 `127.0.0.1:7890`，但那里
  **没有监听**——`curl` 会先收到 `HTTP/1.1 200 Connection established`（CONNECT
  "成功"）再在 TLS 握手时 `SSL_ERROR_SYSCALL`，看起来像"GitHub 被墙"，其实
  **直连是好的**（`curl --noproxy '*' https://api.github.com/rate_limit` → 200）。
  `git push` 走 SSH，一直没受影响，更容易误判。规程：API/下载一律
  `curl --noproxy '*'`（或 `unset HTTPS_PROXY HTTP_PROXY NODE_USE_ENV_PROXY`）。
- **未认证额度只有 60 req/h**：带 job/step 的发布监控很容易打满
  （轮询 60s × 双 run × step 详情）。规程：`/rate_limit`（不计数）先看 reset；
  轮询间隔 ≥60s、只在状态**变化**时多查一次；额度耗尽时改用下面的 HTML 法。
- **`gh` 在 PATH 上 ≠ 能用**：本机 `gh auth status` = token invalid（不改用户凭据），
  所以监控仍按"无 gh"路径走。
- **额度耗尽后的核 CI 办法（无需 API）**：Actions workflow 页 HTML 的 run 行里带
  `aria-label="completed successfully: Run N of ci. <commit 标题>"` +
  `octicon-check-circle-fill`（失败则是 `octicon-x-circle-fill`）——
  `curl -sSL --noproxy '*' https://github.com/<owner>/<repo>/actions/workflows/ci.yml`
  后对该 commit 短 SHA 取上下文即可判读；`git ls-remote --tags origin` 走 SSH，
  核"tag 是否重复/指向哪个 commit"最省，也不受额度影响。
- **tag 已存在时 auto-tag 会跳过**：docs-only 的后续 push 不会再发一次版——
  用 `git ls-remote --tags` 确认同名 tag 只有一个、且指向 release commit。

- **步骤日志拉不到（403）**：`/actions/runs/<id>/logs` 即使在公开仓库也要鉴权
  （2026-09-12 实测 403）。所以**能拿到的最强信号是 job/step 的 `conclusion`**；
  真要日志得让用户在网页端看，或本机装 `gh` 后 `gh auth login`。
- **版本纪律先于 tag**：tag 之前确认 `Cargo.toml` 与
  `editor/vscode/package.json` 版本已 bump 且一致（feature→minor /
  fix→patch，见 `docs/vscode-dev-guide.md` §2）——release 的 version gate
  会直接 fail 不一致的 tag；发布形态（per-target VSIX + universal 回退包）
  与 dry-run 见 `docs/RELEASE.md`。
- **推送后必监控到终态**：每 2–5 分钟轮一次（`case "$OUT" in
  *in_progress*|*queued*) sleep 45;; esac`），别用一次查询下结论；红 → 立即取证。
- **release 期望的 job 形状**：`build`（matrix 8）→ `package-vsix` →
  `github-release` 与 `marketplace-publish`（后两者并行）。**`github-release`
  绿 ≠ 资产齐**，必须按 §2.1 核对双页。

### 2.1 发布后核对"双页"（2026-09-11 空 Release 事故留下的硬性预防）

```bash
# ① GitHub Release：必须恰 26 个资产（lsp tarball ×8 + cli tarball ×8 + vsix ×9
#    + SHA256SUMS；0.35.1 起每资产另有 SLSA provenance attestation）
curl -sS "https://api.github.com/repos/ColorlessBoy/sokonanoda-lang/releases/tags/v$VER" \
  | $NODE -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>{const r=JSON.parse(s);
      const a=r.assets.map(x=>x.name);
      console.log("assets:",r.assets.length,
        "lsp:",a.filter(n=>/^sokonanoda-lsp-.*tar\.gz$/.test(n)).length,
        "cli:",a.filter(n=>/^sokonanoda-cli-.*tar\.gz$/.test(n)).length,
        "vsix:",a.filter(n=>/\.vsix$/.test(n)).length)})'
# ② Marketplace：轮询到 versions 里出现目标版本号（见下面的延迟警告）
curl -sS -X POST "https://marketplace.visualstudio.com/_apis/public/gallery/extensionquery" \
  -H "Accept: application/json;api-version=7.2-preview.1" -H "Content-Type: application/json" \
  -d '{"filters":[{"criteria":[{"filterType":7,"value":"sokonanoda-lang.sokonanoda"}]}],"flags":3}' \
  | $NODE -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>{const e=JSON.parse(s).results[0].extensions[0];
      console.log(e.lastUpdated, [...new Set(e.versions.map(v=>v.version))].slice(0,4).join(","))})'
```

| 陷阱 | 事实 | 规程 |
|---|---|---|
| Marketplace 索引**延迟** | `vsce publish` 成功后 `extensionquery` 还要**几分钟**才收录新版本（2026-09-12 v0.17.0 实测约 2 分钟） | 轮询到出现目标版本号为止，别查一次就判失败；看 `lastUpdated` 是否推进 |
| 按版本号"探测"是**死路** | `/_apis/public/gallery/publishers/<p>/vsextensions/<n>/<v>` 这个路由**不存在**，一律 404（"controller … was not found"） | 只认 `extensionquery`；那个 404 不代表没上架 |
| 半坏状态 | `github-release` 与 `marketplace-publish` 是独立 job，可能"已上架但 Release 页零资产"（v0.10.0 实际发生） | 两个页面**都必须**核对，不能只看 run 绿 |
| 资产"能下"才算发布完 | 只核清单不够：tarball 可能解不出或丢 exec 位 | 发布后抽样 `curl` 下载 → `tar xzf` → 跑 `./sokonanoda <file>` 看真退出码（v0.17.0 做过，darwin-arm64 两个资产均 OK） |
| Azure gallery **503** | `vsce publish` 连续 4 次 HTTP 503 Service Unavailable 是服务端瞬时故障（v0.20.0 实况），与流水线无关 | 先 `curl` gallery 的 extensionquery 探健康度：503 → 等恢复后 `gh run rerun <run-id> --failed`；别改流水线 |

## 3. 排错三板斧

1. `gh run view <id> --json jobs` → 哪个 job 红；
2. `gh run view <id> --log-failed` → 失败 step 的**最后 30 行**（错误
   几乎总在尾部；警告墙会淹没中间）；
3. 区分三类：**代码问题**（修代码+测试）、**workflow 机制问题**
   （修 yml + 强移 tag）、**间歇性网络**（直接重跑 job）。

## 4. 失败必录

每次 CI 红了（本地推前发现的红也记），在 `docs/CI-FAILURES.md` 追加
原因 / 修复 / 预防；同一类失败第二次出现 = 流程没改进，返工。

补充（2026-09-10）：

| 陷阱 | 事实 | 规程 |
|---|---|---|
| `vscode-test` ETIMEDOUT | `Resolving version...` 后 `AggregateError [ETIMEDOUT]` 是连 `update.code.visualstudio.com` 下载 VS Code 失败，**间歇性** | 直接重跑 job；同提交下一轮绿即验证为网络问题；不要当代码回归查 |

| 陷阱 | 事实 | 规程 |
|---|---|---|
| Linux glibc 地板 | ubuntu-latest 原生构建会带 glibc 2.39 符号（VS Code 自身底线 2.28），老发行版装不上 | Linux 目标走 `cargo zigbuild` + `.2.28`；构建期 `readelf` 断言；musl（alpine）断言 `ldd` 静态；Zig/cargo-zigbuild 版本钉死 |

| 陷阱 | 事实 | 规程 |
|---|---|---|
| artifact 往返丢 unix mode | `upload/download-artifact` 后文件变 0644；直接打 tarball 会发布不可执行的二进制（v0.8/0.9 实际发生） | 打包前 `chmod +x` + `tar tzvf \| grep '^-rwx'` 断言；消费方一律自带 chmod 兜底 |
| 本机直连 GitHub 超时 | 本地验证/下载需走代理 | `export HTTPS_PROXY=http://127.0.0.1:7890 HTTP_PROXY=http://127.0.0.1:7890` |

| 陷阱 | 事实 | 规程 |
|---|---|---|
| GitHub Actions Node 20 弃用 | runner 把 node20 action 强制跑在 Node 24，annotation 点名 `actions/checkout@v4` / `setup-node@v4` | 升到 node24 版本：`checkout@v5`、`setup-node@v5`、`upload-artifact@v6+`、`download-artifact@v7+`；`Swatinem/rust-cache@v2` 已是 node24；`mlugg/setup-zig@v2`（最新 v2.2.1）仍 node20，暂无替代，留观察。判断某版本运行时：`gh api -H "Accept: application/vnd.github.raw" repos/<owner>/<repo>/contents/action.yml?ref=<tag>` 看 `using:` |
