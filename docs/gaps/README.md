# 缺口台账（gaps）

> 用途：**教学项目在写作过程中撞到的语言/工具缺口**，每条都带最小复现，交给
> 「语言线」的另一个 agent 去实现。协议全文见 `docs/design/teaching-project.md` §6。
>
> 为什么放在语言仓库：缺口属于语言，不属于某一门课；课程侧只留 `requires = "x.y"`。

## 文件

| 文件 | 作用 |
|---|---|
| `ledger.jsonl` | 台账本体：一行一条 JSON（schema 见设计 §6.2），机器可读、可 diff。`kind` 现在有四类：`language` / `tooling` / `infra` / **`library`**（标准库欠账，另有 `owner`: `prelude`/`course-lib`/`exercise` 与 `lean_names`） |
| `spike/` | **试做稿**：卷 I 前两个单元真写一遍的现场（`lib/` 66 条 + `units/` 16 题），产出 L-01…L-05；报告见 `spike/README.md` |
| `repro/` | 每条缺口的**最小复现**（`.sokonanoda` / 自断言 `.sh` / 小项目夹具），必须入库。**LSP 层的缺口**用「`.sh` 外壳 + 同目录 `.js` 探针」：台账只认 `.sh`（它用 `bash` 跑），而驱动 LSP over stdio 需要 JSON-RPC 客户端——外壳 `exec node` 到隔壁的 `.js`，退出码约定不变（样板见 `G20-lsp-drops-rescued-report.{sh,js}`） |
| `../scripts/gap.py` | 台账工具：`list` / `show` / `next` / `check` / `close`（见下） |

## 日常命令

```bash
python3 scripts/gap.py list                 # 按级别 + WO 排序看全部缺口
python3 scripts/gap.py next                 # 下一条要修的缺口 + 可直接粘贴给另一个 agent 的 prompt
python3 scripts/gap.py selftest             # 自检判定规则本身（judge()，秒级，不跑判卷）
python3 scripts/gap.py check                # 跑全部复现，报告「台账 vs 现实」偏差（收尾/CI 用）
python3 scripts/gap.py close G-11 --version 0.59.0   # 关账（会先复跑复现，仍复现则拒绝关）
```

`selftest` + `check` 已接进两处：`scripts/soko gate` 的第四步（课程门禁之后）与
CI `test` job 的 `Gap ledger is consistent (docs/gaps)` step。台账因此是**被强制执行
的契约**，而不是一份会腐烂的文档。

## 复现脚本的退出码约定（自断言）

`.sh` 复现一律遵守：

| 退出码 | 含义 |
|---|---|
| **0** | 缺口仍在，且与台账 `today` 字段描述一致（**正常状态**） |
| **1** | 行为变了（很可能已修复）⇒ 回来更新台账（写 `fixed_in`）并升级课程 |
| **2** | 环境/前置缺失（跑不起来，不算结果） |

于是「缺口即测试」成立：`gap.py check` 红了 = 语言变了而台账没跟上（或修好忘了关账）。
`.sokonanoda` 复现没有脚本外壳，`gap.py check` 的判据是「是否干净判卷 + 有没有 checked
声明」（**故意不看 `exercise_open` 计数**——G-01 就是这么假绿的）。

期望默认从 `status` 推出（`fixed` ⇒ 复现应当转绿 / `.sh` 应当 exit≠0；`open` ⇒ 相反）。
**有些缺口的「修好」恰恰是判红**——例如 G-01 钉的是「签名写错必须被拒」。这类条目在
台账里写显式 `repro_expect` 覆盖推导：

| `repro_expect` | 适用 | 含义 |
|---|---|---|
| `clean` | `.sokonanoda` | 复现件必须干净判卷（有 `checked`、无 diagnostic、exit 0） |
| `rejected` | `.sokonanoda` | 复现件**必须**判红（契约就是「必须被拒」，如 G-01） |
| `exit0` | `.sh` | 缺口仍在（= 脚本 exit 0） |
| `nonzero` | `.sh` | 行为已变（= 脚本 exit 1/2） |

取值与复现类型不匹配、或拼错，`check` 会直接判不一致并把原因印出来（`selftest`
钉住这些形态）——写错的字段不会被默认推导悄悄盖过。

**复现一律在中性环境里跑**：`gap.py` 会从子进程环境里剔除 `SOKONANODA_BIN` /
`SOKONANODA_LSP_BIN`（`clean_env()`，`selftest` 钉住）。这两条是"显式指定二进制"的
逃生门，而 G-11/G-16 这类复现**测的就是启动器自己的解析链**——环境里带着覆盖，夹具里
那个"陈旧缓存必须被拒绝"的现场就被绕过了（本门禁接进 `gate` 后第一次跑就是这么红的，
教训见 `docs/LESSONS.md`）。要测覆盖的复现自己 inline 赋值，别依赖调用者的环境。

## 手工复跑（等价于 `gap.py check`）

```bash
# 集合论可行性（应 11 checked / 0 failed）
scripts/soko query check --file docs/gaps/repro/OK-set-spike.sokonanoda --compact

# 自断言脚本
bash docs/gaps/repro/G11-launcher-version-source.sh          # 课程仓没有版本源
bash docs/gaps/repro/G12-relative-entry-ancestor-manifest.sh # 相对=绝对 + 裸文件名上溯 + 模块根非空（已修 ⇒ exit 1）
bash docs/gaps/repro/G10-query-check-parse-error.sh          # query check parse 假绿（已修 ⇒ exit 1）
bash docs/gaps/repro/G17-query-goals-holes-parse-error.sh    # goals/holes 同款假绿（已修 ⇒ exit 1）
bash docs/gaps/repro/G06-course-import.sh                    # course 聚合不认 import
bash docs/gaps/repro/G13-axiom-binder-params.sh              # axiom 不吃 binder 参数表
bash docs/gaps/repro/G15-query-check-bare-offsets.sh         # query check 的 failed[]/warnings[] 只给裸 offset（已修 ⇒ exit 1）
bash docs/gaps/repro/G07-course-manifest-v2.sh               # 课程清单扁平（已修 ⇒ exit 1）
scripts/soko grade docs/gaps/repro/G14-single-universe-binder.sokonanoda  # 只允许一个宇宙 binder
scripts/soko grade docs/gaps/repro/G01-open-exercise-signature.sokonanoda    # 开练习签名免检（已修 ⇒ exit 1）
scripts/soko grade docs/gaps/repro/G01-course-signature-mutations.sokonanoda # 课程级签名变异体（已修 ⇒ exit 1）
```

> **G-01 修好后（0.59.0）**：开练习的签名与值位同罪——签名 elaborate 不了、不是类型、
> 或 `theorem` 的不是 Prop ⇒ 一条 diagnostic（span 取**签名自身**）+ 声明 Failed +
> **不发** `exercise.open`。所以上面两条命令**应该** exit 1（它们钉的是"签名受检"
> 这个契约）。顺带一条判据纪律：`exercise.open` 计数对签名腐烂永远是盲的，
> 判卷只认 `decl.checked` 与 `diagnostic`。

> G-10 / G-17 修好后（0.59.0）**两条通道同口径**：同一份解析不了的文本，
> `query check` 的 `failed[]` 带 parse 诊断 + 退出码 1，与 `grade` 一致；
> `query goals`/`holes` 答 `ok:false` + `not-parsable` + 退出码 1。
> 两个脚本按复现约定都是「修后 = exit 1」：exit 0 表示假绿回来了（回归）。

> **G-09 的收尾形态（0.59.0，可当"怎么撤 repro"的范例）**：这条记的是「内核断言以裸
> `left: 1 / right: 0` 外泄」。它的复现件与 G-03 共用（同一触发路径），G-03 修好后那份
> 文件转绿 ⇒ **撤下 `repro` 字段**（留一个"应当判绿"的文件只会假报缺口），把两半结论写进
> `notes`：①用户可见面已有稳定码 `kernel-internal` + 「这不是你的代码问题」提示
> （`crates/front/src/compile/error.rs`，测试钉住）；②唯一已知可达触发路径随 G-03 关闭。
> 内核冻结下「断言永不外泄」是不变量而非可达缺口——**日后若写出新的可达反例，按
> `kernel-internal` 码新开一条**，不要把这门旧账改回 `open`。

## 状态词表

`open`（已记账，未派工）→ `wo-filed`（有 WO）→ `fixed`（`fixed_in` 必填版本）；
另有 `workaround`（长期绕行，须写清代价）与 `wontfix`（须写理由）。
`wo_planned` 是**计划编号**（WO 文件还没写出来时不填 `wo` 路径，免得指向不存在的文件）。

## 与既有 backlog 的关系

`ROADMAP.md` §10 / `docs/HANDOVER.md` §3 是**语言自己的**待办；本台账是**课程驱动的**
待办，两者不重复记账：如果一条缺口已经在既有 backlog 里，本台账只写指针（见
`ledger.jsonl` 里 `notes` 字段），修完由语言线那边关账。
