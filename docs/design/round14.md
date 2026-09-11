# 设计：稳定 hole_id + 归纳块 recursor 自动派生 + spine meta 路线

> 状态：设计定稿（2026-09-07，第十四轮实施 hole_id 与 auto-derivation；
> spine meta 只定路线，实现留下一轮）。依据：`docs/notes/gap-analysis.md` 余项、
> 第十三轮遗留（`elab-missing-inductive-rec` 停gap、refine 子洞 kernel 级
> expected type）。

## 0. 头脑风暴与取舍

1. **hole_id 的稳定性边界**：完全跨版本稳定（增删声明后 id 不变）需要
   内容寻址，成本高于收益；采取 **(声明名， 洞序号)** 方案——同一文档版本内
   稳定、声明名不变时跨版本稳定；匿名 example 用 `example@<行号>` 形式
   （与 `render::decl_name` 一致）。wire：`soko/goals` 的 `holes` 从
   `[Range]` 变 `[{range, id}]`；`soko/nextHole` 保持裸 Range（客户端
   navigation 不依赖 id，避免无谓 churn）。
2. **auto-derivation 的教学定位**：rec 块（显式写 iota 规则）仍是单元⑤的
   正课内容；auto-derivation 是其**之后的便利层**——学生先手写 Bool 的 rec
   理解 iota，再获准省略。实现顺序上让两者并存：显式 rec 优先，无 rec 时
   自动派生（同一条 elab 路径，合成 `RecDecl`+`IotaRule` AST 后复用既有
   代码）。第十三轮的 `elab-missing-inductive-rec` 守卫**移除**（被本功能
   取代）。
3. **派生规则的正确性锚点**：与 py-nat 的手写 rec 同构——
   - rec 类型：`forall (motive : (x : Ind) -> Sort u) (m_i : <ctor_i 望远镜,
     minor = motive (c_i 字段…)>) … (target : Ind) -> motive target`；
   - 每构造子的规则值：telescope = (motive, 全部 minors, 本构造子字段)，
     返回 `m_i <字段…>`，**递归字段后面追加自调用**
     `<Ind>.rec.{u} motive minors… 字段…`（py-nat succ 规则同形）；
   - 递归字段判定复用 `mentions_ident`/result 望远镜扫描（与 is_recursive
     镜像同源）。
4. **spine meta（refine 子洞 kernel 级 expected type）路线**（第十五轮按
   B′ 实施）：
   - 现状：walk 的子洞类型来自**模板文本实例化**（仅 Ident 字段类型做
     binder→goal 实参替换，复合字段类型原样渲染，实例化失败为 `None`）；
   - 方案 A（完整解，远期）：elab 的 App 实参级 expected 穿透——需新 elab
     机制（应用头类型求值 + 实参位取域），M–L；
   - **方案 B′（本轮实施）**：`field_type_text` 升级为**深度 AST 替换**——
     模板 binder 名 → goal 实参 AST 在字段类型表达式树内全量替换（带遮蔽
     守卫：同名 Forall/Lambda binder 下不替换），再 render。覆盖复合字段
     类型（`Eq α a b` → `Eq α x y`）；零流水线重构（walk 仍在命令期）；
   - 方案 C（不可行）：judge 探针合成 fresh axiom——内核无 unknown 项概念。
   - 全量 kernel 渲染（elab 字段类型 + pp）留待确有需求时再评估（需把
     子洞类型计算挪到 ops 期）。

## 1. hole_id（P）

- 服务端（`lsp/src/lib.rs` `goal_decls`）：`holes` 序列化为
  `Vec<HoleInfo{range, id}>`；id = `format!("{decl}:{index}")`，decl 用
  `render::decl_name(d)`；`sub_goals` 不变（与 holes 位置对齐）；
- 客户端（extension.js）：消费处改为读 `hole.range`；既有契约
  （客户端禁止文本扫洞、nextHole server 端计算）不变；
- 契约测试（extension.rs）：新增「goals holes 必须带 id」断言防回归。

## 2. auto-derivation（Q）

- `install_inductive_block`：移除缺 rec 守卫；`recursor == None` 时合成：
  ```text
  rec <Ind>.rec {u} :
    (motive : (x : Ind) -> Sort u) ->
    (m<i> : forall (<字段望远镜>, motive (<c_i> <字段>…))) …  -- 每构造子一条 minor
    (target : Ind) -> motive target
  iota <c_i> := fun (motive : …) => fun (m_0 : …) => … =>
    fun (<字段望远镜>) => m_i <字段…> [<自调用…>]
  ```
  合成后走与显式 rec 完全相同的 elab 路径（universe `u`、known.insert、
  RecursorData 组装复用）；
- 名字卫生：`motive`/`m0..mn`/`target` 与字段名冲突时由 elab 的作用域
  遮蔽语义兜底（内层赢）；
- 显式 rec 优先：源里有 rec 时零行为变化（py-nat/课程块不变）；
- 移除物：`ErrorKind::ElabMissingInductiveRec` 变体 + code + hint +
  穷尽清单条目 + protocol.md 条目 + 第十三轮的两个测试（front 1 + cli 1，
  改写为 auto-derivation 正例）；
- 测试三层：
  - front：非递归（Unit，无 rec）编译 + `Unit.rec` 归约；Bool 无 rec →
    `not tt => ff`（if-then-else 语义）；递归块无 rec（Nat 加法）→
    `add two two` 归约；显式 rec 优先（py-nat 既有测试不动）；
  - CLI e2e：Bool auto-derivation 冒烟；
  - 语料回归：全量测试绿。

## 3. 文件分工（互斥清单）

| owner | 允许修改 |
|---|---|
| 主会话（预接） | 本设计文档（§0.4 spine meta 路线）、`docs/protocol.md`（hole_id 节 + 移除 missing-rec 条目——合并期统一处理） |
| P（hole_id） | `crates/lsp/src/lib.rs`（goals wire + tests）、`editor/vscode/{extension.js,package.json}`、`crates/cli/tests/extension.rs` |
| Q（auto-derivation） | `crates/front/src/compile/{elab.rs,error.rs,tests.rs}`、`crates/cli/tests/cli.rs` |
