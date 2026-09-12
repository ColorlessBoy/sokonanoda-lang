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
# 与 CI 完全一致的命令（.github/workflows/ci.yml），顺序执行：
cargo fmt -p sokonanoda-front -p sokonanoda-cli -p sokonanoda-lsp -- --check
cargo clippy --workspace --all-targets -q; echo "EXIT=$?"
cargo test --workspace --locked -q; echo "EXIT=$?"
cargo run -q -p sokonanoda-cli --bin sokonanoda -- --json playground.sokonanoda
```

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
| `download-artifact@v4` 不带 `name:` | 每个 artifact 下载进**同名目录**（`lsp-<target>/`、`sokonanoda-vsix/`），不是平铺 | 引用路径前先确认落盘布局；要平铺用 `pattern:` + `merge-multiple: true` |
| tag 触发的 workflow 用**哪个文件** | 用 **tag 指向的 commit** 上的文件，不是 main 最新 | 修 workflow 后要重跑 release：`git tag -f v<ver> <fix-commit> && git push -f origin v<ver>`；`gh run rerun` 只会重放旧文件 |
| `gh release create` 非幂等 | 已存在 → 422 | 永远 `|| true`（存在即跳过） |
| `gh release upload` 非幂等 | 同名 asset 已存在 → 422 → `bash -e` 全 job 死 | 永远 `--clobber`（重跑覆盖） |
| 强移 tag 前先想清楚 | release 上可能已有首跑传了一半的 asset | 上传步骤必须幂等后再强移 |
| `vsce package --no-dependencies` | 跳过生产依赖收集 → 12 文件/21KB 空壳 VSIX | 冒烟用与 release.yml 相同的命令（无该 flag）；平台包正确基线 ≈327 文件（含 `bin/<target>/`） |
| vsce publish Azure 超时 | `Request timeout: /_apis/gallery` 是**间歇性**网络问题 | 直接重跑该 job；连续两次超时再查代理 |
| action 名拼写 | 多个 s / 复数错名 → action 不存在 | 新 action 首次使用先验证存在 |
| run 步骤默认 `bash -e` | 任何一步非零退出 → 整个 step 死 | 想容忍的命令才加 `\|\| true`；别把 `bash -e` 当没有 |
| 平台包 exec 位 | VSIX 的 zip 记录 unix mode；**Windows 上 `vsce package` 会丢执行位**（vsce #152/#512） | 只在 Linux/macOS 打包；stage 时 `chmod 755`；冒烟用 python `zipfile` 断言 `mode & 0o111` |
| 平台包发布顺序 | Marketplace 同版本 universal + 多 target 并存；“Validating”窗口有装错 target 的竞态（vscode#141696） | 先 universal 后 target；每包独立重试；装错反馈先让用户卸载重装 |
| 版本门禁 | tag / `Cargo.toml` / `package.json` 三者不一致时 release 必须 fail（历史上靠人工） | release `package-vsix` 的 version gate + `cargo_and_extension_versions_match` 契约测试双保险；tag 前先 `cargo check` 更新 lock |

## 2. 触发与监控

```bash
git push origin main                      # → ci（main）
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
# 某提交触发了哪些 run
curl -sS "https://api.github.com/repos/ColorlessBoy/sokonanoda-lang/actions/runs?head_sha=$SHA" \
  | $NODE -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>{const j=JSON.parse(s);
      console.log((j.workflow_runs||[]).map(r=>r.id+" "+r.name+" "+r.status+" "+(r.conclusion||"-")).join("\n"))})'
# 某 run 的 job 与逐 step 状态（判断卡在哪一步）
curl -sS "https://api.github.com/repos/ColorlessBoy/sokonanoda-lang/actions/runs/$RUN/jobs?per_page=30" \
  | $NODE -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>{const j=JSON.parse(s);
      for(const x of j.jobs) console.log(x.name, x.status, x.conclusion, "|",
        x.steps.map(t=>t.number+":"+t.name+"="+t.conclusion).join(", "))})'
```

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
# ① GitHub Release：必须恰 25 个资产（lsp tarball ×8 + cli tarball ×8 + vsix ×9）
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
