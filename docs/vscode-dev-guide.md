# VS Code 扩展开发规范

> 适用：`editor/vscode/` 下所有改动。接手前先读本文 + `editor/vscode/README.md`。
> 违反版本纪律或跳过测试层 = commit 打回。

## 1. 文件职责

| 文件 | 职责 | 禁止 |
|---|---|---|
| `extension.js` | 扩展入口：LSP 客户端接线、命令注册、练习树/课程树/状态栏/inlay/跳洞 | 业务逻辑、kernel 调用 |
| `package.json` | 清单：contributes、dependencies、engines | 运行时逻辑 |
| `syntaxes/*.tmLanguage.json` | TextMate 语法（即时高亮，LSP 语义高亮的降级层） | — |
| `language-configuration.json` | 括号配对、注释、缩进 | — |

## 2. 版本纪律（semver，硬规则）

### 判断标准

| 变更类型 | 版本升 | 判断依据 | 例 |
|---|---|---|---|
| **新功能** | **minor**（0.2 → 0.3） | 学习者能做**之前做不到的事** | 新增练习树、新增跳洞、新增 inlay hints、新增 rename |
| **改进 / bug 修复** | **patch**（0.3.0 → 0.3.1） | 已有功能变得**更好用或更正确**，但没多出新能力 | hover 临近回退、括号悬停、诊断分级、修坐标偏移 |
| **破坏性变更** | **major**（1.0.0） | 设置改名、命令移除、行为不兼容 | 移除某个设置项 |

### 判断口诀

> 问自己：学习者**能不能做一件之前做不了的事**？
> 能 → minor。不能（只是做得更好/更对）→ patch。

### 实际案例

| 改动 | 正确版本 | 为什么 |
|---|---|---|
| 新增练习面板 | minor | 学习者多了"看练习列表"的能力 |
| 新增跳洞（alt+n） | minor | 学习者多了"跳洞"的能力 |
| hover 括号回退 | **patch** | hover 一直存在，只是现在括号上也有信息了 |
| 修复 VSIX 打包 | **patch** | 没多出新能力，修的是安装后坏的问题 |
| 新增 rename | minor | 学习者多了"重命名"的能力 |

### 多个改动合并提交

一个 commit 里既有 minor 又有 patch 改动 → 取**最高的**（minor）。
一个 commit 纯 patch 改动 → patch。不确定时 → patch（宁可低不可虚高）。

- **每次 commit 涉及 `editor/vscode/` 改动时必须同步 bump `package.json` version**；
- 同步更新 `editor/vscode/CHANGELOG.md`（Keep a Changelog 格式）；
- 版本号变更后必须重新 `vsce package` + `code --install-extension` 并在本地 VS Code 验证。

## 3. 测试三层

| 层 | 工具 | 覆盖 | 文件 |
|---|---|---|---|
| 静态契约 | `cargo test -p sokonanoda-cli --test extension` | package.json 字段完整性、命令注册一致性、依赖打包安全、市场元数据 | `crates/cli/tests/extension.rs` |
| 集成测试 | `npm test`（@vscode/test-electron） | 扩展激活、诊断到达、hover 内容、sorry warning | `editor/vscode/src/test/extension.test.js` |
| 手动验证 | F5 开发宿主 | 全功能（面板、树、inlay、跳转、补全） | — |

**commit 前**：至少跑静态契约 + 集成测试；**发 tag 前**：三层全跑。

## 4. 开发循环

```bash
# 改 extension.js 或 package.json 后：
cd editor/vscode
npx --yes @vscode/vsce package --out sokonanoda.vsix
code --install-extension sokonanoda.vsix --force
# 手动 Reload Window（Cmd+Shift+P → Reload Window）
```

或按 F5 用开发宿主调试（`.vscode/launch.json` 已配置）。

## 5. 常见坑（全部踩过）

1. **`node_modules` 不能进 `.vscodeignore`**——vsce 靠它把生产依赖装进 VSIX；
2. **`vscode-languageclient` 必须在 `dependencies`**——放 `devDependencies` 的 VSIX 装上即坏；
3. **didOpen 是通知**——不发 id，不期待响应；探针/测试里发 id 会被当作未知请求；
4. **LSP 帧格式**——头块以 `\r\n\r\n` 结尾；探针/测试必须完整消费头块再读 body；
5. **服务器更新后须重载窗口**——LSP 进程在窗口激活时 spawn，改 Rust 代码后不重载 = 旧服务器；
6. **`code` CLI 与已开实例冲突**——集成测试在 macOS 上报"another instance running"时关掉 VS Code 再跑；
7. **代理**——vsce/Node 不读系统代理；需要时设 `HTTPS_PROXY=http://127.0.0.1:7890`。

## 6. 发布

```bash
# 本地发布（需要 PAT，见 docs/RELEASE.md）
cd editor/vscode
npx --yes @vscode/vsce publish

# CI 自动发布（tag 触发，VSCE_PAT secret 已配置）
git tag v0.X.Y && git push --tags
```

发布前检查清单：
- [ ] `package.json` version 已 bump
- [ ] `CHANGELOG.md` 已更新
- [ ] `README.md` 无占位符
- [ ] `cargo test --workspace --locked` 全绿
- [ ] `npm test` 集成测试全绿
- [ ] `icon.png` 存在且 ≥128×128 PNG
