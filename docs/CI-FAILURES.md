# CI 失败记录（每次失败的原因与修复）

> 目的：同一类失败不犯第二次。每次 CI 红了，在这里追加一条（失败原因、
> 修复方式、预防措施）。

## 格式

```
### YYYY-MM-DD — 简述
- 原因：
- 修复：
- 预防：
```

### 2026-09-11 — release github-release job 对 Windows artifact 目录 chmod 失败
- 原因：`release.yml` 的 tarball 循环无条件
  `chmod +x "${dir}sokonanoda-lsp"`；Windows 构建产物是
  `sokonanoda-lsp.exe`（artifact 目录 `lsp-*pc-windows-msvc/`），chmod 找不到
  文件即失败（`bash -e` 直接死），8 个 LSP/CLI tarball 与 9 个 VSIX 都没上传，
  Release 页只剩自动生成的 notes、零资产。Marketplace 发布（独立 job）不受
  影响，0.10.0 已正常上架。
- 修复：tarball 循环按 target 是否含 `windows` 追加 `.exe`
  （`exe=""; [[ "$target" == *windows* ]] && exe=".exe"`）；`git tag -f v0.10.0`
  指向修复 commit 后强推 tag 重跑（tag 触发 workflow 用的是 tag 指向的 commit
  上的文件，`gh run rerun` 只会重放旧文件）。
- 预防：**任何跨平台打包/上传循环都要按 artifact 里的实际文件名处理 `.exe`**
  （与 package-vsix 的 stage-lsp.js 一致）；发布后核对 GitHub Release 资产数
  （25 个）与 Marketplace 版本，双页都验证。v0.9.0 的 tarball exec 位教训同
  族（平台差异 → 发布资产损坏），发布清单里加「Release 资产数 + 可执行位
  抽样」项。
---

### 2026-09-08 — fmt 格式不匹配（debug 测试未格式化）
- 原因：添加了 debug 测试（`debug_bracket_hover_content`），推代码前忘跑 `cargo fmt`
- 修复：删除 debug 测试（它本来就是临时诊断用的）
- 预防：**推代码前必须跑 `cargo fmt -- --check`**（已在 AGENTS.md 命令清单里）

### 2026-09-08 — action 名写错（plural vs singular）
- 原因：`softprops/actions-gh-release` 写成了 `softprops/action-gh-release`
  （实际上正确的名字是 `softprops/action-gh-release`，我第一次写的是
  `softprops/actions-gh-release`（多了个 s），GitHub 找不到这个 action）
- 修复：改成正确名字
- 预防：**新 action 首次使用时用 `gh workflow` 或浏览器验证 action 存在**

### 2026-09-08 — gh release upload "release not found"
- 原因：vsix job 依赖 build job 创建 GitHub Release，但 build job 重写后
  不再创建 release（移到了独立 job），vsix job 的 `needs` 没有更新
- 修复：重写 workflow 使 job 依赖链正确（build → vsix → github-release）
- 预防：**改 workflow 的 job 结构时，检查所有 `needs` 和 `gh release` 引用**

### 2026-09-08 — gh release create "Release.tag_name already exists"
- 原因：`gh release create` 是非幂等的——release 已存在时报 422
- 修复：追加 `|| true`（release 已存在时跳过创建，只做 upload）
- 预防：**所有 `gh release create` 都追加 `|| true`（幂等）**

### 2026-09-08 — vsce publish "Request timeout: /_apis/gallery"
- 原因：GitHub Actions runner 到 Azure DevOps gallery API 的网络超时
  （间歇性，重跑有时能过有时不能）
- 修复：无根修——是 Azure DevOps 侧的问题。workflow 重跑 + `--skip-duplicate` 幂等
- 预防：无（Azure DevOps 侧问题）。考虑改用 `--oidc` 或 `--azure-credential`
  （但 Marketplace 侧支持尚未就绪，见 docs/RELEASE.md）

### 2026-09-08 — clippy lint（map over inspect）
- 原因：在 elab 里用 `.map(|ty_expr| { ...; ty_expr })` 做 side effect
- 修复：改成 `.inspect(|_| { ... })`
- 预防：**推代码前必须跑 `cargo clippy --workspace --all-targets`**（已在 AGENTS.md）

### 2026-09-08 — clippy lint（explicit lifetimes）
- 原因：测试函数签名里的显式生命周期可以省略
- 修复：去掉 `<'a>`
- 预防：同上，clippy 全跑

### 2026-09-08 — unclosed delimiter（花括号不平衡）
- 原因：python 脚本编辑 lib.rs 时花括号计数出错（多次编辑叠加）
- 修复：用 git checkout 恢复文件后重新编辑
- 预防：**用脚本改代码后必须 `cargo build` 验证编译通过**；复杂改动用
  git checkout 恢复后重做，不要在坏的基础上修补

### 2026-09-09 — fmt 格式不匹配（debug 测试第二次）
- 原因：同上——加了 debug 测试忘跑 fmt
- 修复：删 debug 测试 + fmt
- 预防：同上（这是第二次犯同一类错误）

### 2026-09-09 — clippy lint（unused variable）
- 原因：ty_text 渲染代码加了 debug eprintln 但变量名没改
- 修复：移除 debug 代码
- 预防：同上

### 2026-09-09 — E0425 cannot find value / E0599 no method（编辑冲突）
- 原因：多个 python 编辑脚本叠加修改同一文件，前后编辑互相覆盖
- 修复：git checkout 恢复后重新编辑
- 预防：**对同一文件的多次编辑要么合并为一个脚本，要么每步后 build 验证**

### 2026-09-09 — lint 失败：clippy `int_plus_one`（本地假绿）
- 原因：新代码 `h.span.start.offset >= open + 1` 触发
  `clippy::int_plus_one`（CI 的 clippy 经教学 crates 的
  `[lints.rust] warnings = "deny"` 以 `-D warnings` 执行，直接 error）。
  本地"验证"用了 `cargo clippy ... 2>&1 | grep -E "^(warning|error).*crates/"`
  ——**grep 掩膜吞掉了警告行**（crate 路径在 `-->` 行不在 warning 行），
  且 `$?` 取到的是 grep/head 的退出码——本地假绿，CI 必红。
- 修复：改成 `h.span.start.offset > open`（语义等价）。
- 预防：**本地验证必须跑与 CI 完全一致的命令
  `cargo clippy --workspace --all-targets`，退出码用
  `echo ${PIPESTATUS[0]}` 或不带管道直接查**；输出只许 tail 不许 grep 掩膜。

### 2026-09-09 — release github-release：VSIX 路径不存在（v0.4.0/v0.4.1 同因）
- 原因：`actions/download-artifact@v4` 不带 `name:` 时按 artifact 名
  **各建一个目录**——VSIX 落在 `sokonanoda-vsix/sokonanoda.vsix`，而
  `gh release upload` 写的是 `vsix/sokonanoda.vsix`（路径是编的，从未
  存在过）→ `no matches found` exit 1。v0.4.0 首跑记录的
  "github-release exit 1（原因未查）"实为同一根因。
- 修复：upload 路径改为 `sokonanoda-vsix/sokonanoda.vsix`，并在
  download 步骤加注释说明 v4 的目录布局。
- 预防：**改 release workflow 的任何路径引用前，先确认上一个 step 的
  实际落盘路径**（download-artifact v4 无 name = 每个 artifact 一个目录；
  带 `pattern` + `merge-multiple: true` 才会平铺）。tag 触发的 workflow
  修复后需**强制移动 tag**（`git tag -f && git push -f`）才会用新
  workflow 重跑，`gh run rerun` 只会用 tag 上的旧文件。

### 2026-09-09 — release 第三跑：tarball "asset under the same name already exists"
- 原因：首跑（路径 bug）在死掉前已把 4 个 tarball 传上 release；修路径后
  强移 tag 重跑，`gh release upload` 对同名 asset 报 422 → `bash -e`
  在第一个重复处退出。`gh release create` 有 `|| true` 但 **upload 没有
  幂等保护**。
- 修复：tarball upload 一并加 `--clobber`（与 VSIX upload 一致——
  同名 asset 覆盖，重跑幂等）。
- 预防：**release job 的每个写操作都要幂等**：create → `|| true`，
  upload → `--clobber`；强移 tag 重跑 release 前先想清楚哪些 asset
  已经落上去了。

### 2026-09-09 — marketplace-publish 连续两次 Azure gallery 超时（v0.5.2）
- 原因：`npx @vscode/vsce publish` 调 `/_apis/gallery`（Azure Marketplace 端点）
  连续两次 `Request timeout`（间歇性网络问题；同一天 v0.5.0/0.5.1 均一次通过）。
- 修复：第 3 次 `gh run rerun --failed` 通过（确认为瞬态）；同时在 release.yml 给
  publish 步骤加 **4 次重试、间隔 30s**（`--skip-duplicate` 幂等），以后一次超时
  自动重试，不再让整个 release 红。
- 预防：发布流程不再因一次 Azure 抖动失败；若连续重试仍失败再查代理/凭据。

## 2026-09-10 — release dry-run：package-vsix ENOENT（相对路径少一层）

- **现象**：`workflow_dispatch` dry-run 的 `package-vsix` job 在第一个
  target 就失败：`stage-lsp: ENOENT: ... copyfile '../lsp-x86_64-unknown-linux-gnu/
  sokonanoda-lsp'`。四个 `build` 全绿、版本门禁通过、artifact 也确实下载到了
  仓库根。
- **原因**：stage 步骤的 `working-directory` 是 `editor/vscode`，仓库根是
  `../../`；写成了 `../lsp-…` 会解析到 `editor/lsp-…`（不存在）。本地复现时
  同样写错一层，说明是路径推理错误而非 CI 环境问题。
- **修复**：`--binary "../../lsp-${rust}/sokonanoda-lsp${exe}"`（commit 见
  台账后一次 push）。
- **预防**：release dry-run（workflow_dispatch）就是为这类只存在于 CI 的
  打包路径问题设的闸——涉及新 job 的路径先跑 dry-run 再打 tag；本地复现
  相对路径时先 `pwd` + `ls` 验证解析目标。

## 2026-09-10 — release dry-run 第二红：vsce `--out dist/…` 不自建目录

- **现象**：路径修复后 staging 与 vsce 打包都成功（日志 tree 可见
  `bin/linux-x64/ (1 file) [4.15 MB]`），最后一步报
  `ENOENT: ... open '.../editor/vscode/dist/sokonanoda-linux-x64.vsix'`。
- **原因**：`vsce package --out dist/…` 只写文件、不创建父目录；`dist/` 只
  存在于 build job 各 runner 的仓库根，package-vsix job 的 `editor/vscode/`
  下没有。
- **修复**：打包步骤（平台包与 universal 包）先 `mkdir -p dist`。
- **预防**：新 job 里凡写文件到新路径，先显式建目录；dry-run 是唯一能
  覆盖跨 runner 文件布局的闸，继续保留。

## 2026-09-10 — ci / VS Code 集成测试：vscode-test ETIMEDOUT（间歇网络）

- **现象**：`xvfb-run -a npm test` 在 "Resolving version..." 后报
  `AggregateError [ETIMEDOUT]`，失败于 `@vscode/test-electron` 连接
  `update.code.visualstudio.com`（下载 VS Code 阶段）；同一提交下一轮 CI
  全绿，属间歇性。
- **修复**：无需改代码，重跑 job。
- **预防**：`docs/TESTING.md` 已记该风险；CI skill 台账补一行：集成测试
  ETIMEDOUT = 网络，直接重跑，不要当代码回归查。

## 2026-09-10 — release dry-run 第三红：musl 静态断言被 pipefail 反杀

- **现象**：8 平台矩阵 dry-run 中，两个 musl 构建（x86_64/aarch64）都失败于
  `Verify static musl binary`，但日志显示 `file` 已报 `statically linked`、
  `ldd` 已报 `not a dynamic executable`——二进制完全正确。
- **原因**：验证脚本是 `ldd "$bin" 2>&1 | grep -q "not a dynamic executable"`，
  而 step 带 `set -o pipefail`；`ldd` 对静态二进制退出码为 1，管道整体判负，
  `||` 兜底分支误报失败。
- **修复**：先 `$(ldd ... || true)` 捕获输出，再 `grep <<<`；并加 `file` 的
  `statically linked` 断言（双保险）。工作流注释已标注该坑。
- **预防**：带 `pipefail` 的断言不要依赖会以非零退出的工具（ldd 静态退出 1、
  grep 无匹配退出 1）作为管道上游；先捕获再断言。

## 2026-09-10 — v0.9.0 发布资产丢可执行位（CI 全绿但产物坏）

- **现象**：v0.9.0（与 v0.8.0）GitHub Release 的 16 个 tarball 里二进制是
  `0644`；`scripts/soko.sh setup` 解出后无法执行，扩展的 universal 回退
  下载同理（`server.js` 只查存在、没 chmod）。CI/release 全绿——坏的是
  产物内容，不是构建结果。
- **原因**：`upload-artifact`/`download-artifact` 往返会丢 unix mode；发布
  job 直接把 artifact 目录 `tar czf`，未补回 exec 位。
- **修复**：①发布 job 在 tar 前 `chmod +x` 并用 `tar tzvf | grep '^-rwx'`
  断言；②`scripts/soko.sh download_one` 解压后先 chmod 再判可执行；
  ③`server.js downloadLspBinary` 解压后 chmod 0755；④回填修复了 v0.9.0
  已发布的 16 个 tarball（重打包 + `--clobber`）。
- **预防**：产物断言必须检查**内容属性**（mode、可执行、`--version`），
  不能只看 job 绿；新增 tarball 消费方（脚本/扩展/launcher）一律自带
  chmod 兜底；dry-run 应加一条“下载解包后直接执行”的冒烟。
