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
