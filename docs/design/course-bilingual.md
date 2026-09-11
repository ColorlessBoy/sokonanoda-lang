# 课程双语：course/ 英文镜像（design-course-bilingual）

## 1. 目标与定位

用户要求教程文档提供**中文与英文两种版本**。范围（用户选定）：
`course/` 单元课程为主——学习者直接面对的 5 个单元画布、解答钥匙与
课程清单。`docs/` 下开发者文档不在本轮范围。

形态（用户选定）：**中文/英文各一份独立文件**，不搞同文件内中英对照。

## 2. 约束与不变量（设计先行，动工前锁定）

1. **判定永远走 kernel**：英文镜像与中文版是**逐字节等价的代码**，
   仅 `--` 注释语言不同。声明体、axiom/def/theorem 类型、`sorry` 洞、
   `#reduce` 自测、`-- soko:hint` 阶梯**全部与中文版逐字一致**。
   这是本设计最硬的不变量：英文文件放进中文判卷器、中文文件放进英文
   判卷器，事件流（decl.checked / exercise.open / expr.reduced / diagnostic）
   必须完全一致。
2. **注释按语义重构，不按字节翻译**（用户原则 2026-09-09）：英文注释是
   重新写就的自然英文教学文案，按英语语感重组句子与段落，**不做中文的
   逐行镜像**（行数、注释块布局、分句都允许不同）。但知识点内容、
   提示阶梯的**条数与顺序**（3 条：思路 / 目标形态 / 关键件）与中文保持
   同构——阶梯是编辑器逐条揭示的教学机制，条数顺序即语义的一部分。
3. **现有 CI 零破坏**：`crates/cli/tests/course.rs` 目前只枚举 `course/`
   **顶层** `.sokonanoda`（不含 `solutions/`、不含子目录），所以新增
   `course/en/` 不会触发它的 `assert_eq!(files.len(), GOLDEN.len())`。
   我们额外为英文镜像加**成对一致性守卫**（见 §5）。
4. **不新增语法**：英文文件不引入课程白名单之外的任何写法（教学语法是
   真实 Lean 4 子集；同规则适用）。

## 3. 布局

```
course/
  unit1-….sokonanoda … unit5-….sokonanoda   (中文，原样不动)
  solutions/unitN-*-solution.sokonanoda       (中文钥匙，原样不动)
  en/                                          (英文镜像，本轮新增)
    unit1-….sokonanoda … unit5-….sokonanoda
    solutions/unitN-*-solution.sokonanoda
  course.json                                  (增加英文标题字段)
  README.md                                    (增加双语布局说明)
```

英文单元文件**复用中文文件的文件名**（`unit1-propositions-proofs.sokonanoda`
等），仅放置于 `en/` 子目录——文件名即"对应同一单元"的标识，无需另行命名
约定。解答钥匙同理，放 `en/solutions/`。注释是**按语义重构**的自然英文：
段落重组、分句自然，不以中文行号/行数为准（约束见 §2.2）。

## 4. course.json 双语化

现有条目 `{"file", "title", "unit"}`。`title` 保持中文（权威），新增
`"title_en"` 字段承载英文标题，方便工具/LSP/前端按需取用，同时**不破坏**
现有 `course.rs` 的 `course_json_lists_the_five_units_in_order` 断言
（该测试只检查 file 与 unit，不看 title）。

## 5. CI 守卫：英文镜像一致性

在 `crates/cli/tests/course.rs` 新增一条测试，守护"双语镜像"不变量：

- 对每个 `course/en/*.sokonanoda`，找到同名中文 `course/*.sokonanoda`，
  两者 **`--json` 事件计数**（decl.checked / exercise.open / expr.reduced /
  diagnostic）必须逐项相等；
- `course/en/solutions/*.sokonanoda` 必须 0 诊断、0 `exercise.open`，
  与中文钥匙同款（可解性守护延伸）；
- `course/en/` 顶层文件数 = `course/` 顶层文件数（5），且同名一一对应。

这样任何"只改中文不改英文 / 只改英文不改中文"的漂移都会在 CI 显红。
（判定走 kernel：比较事件计数而非比较注释文本，与项目"禁文本比对"纪律
一致——注释本来就允许、也应该不同。）

## 6. 不做的事（明确排除）

- 不改 `course/` 中文文件内容（它们继续作为权威源）。
- 不译 `docs/` 开发者文档、不译 `playground.sokonanoda` 根画布（用户选定
  course/ 为主；根画布留待后续按需扩展）。
- 不做运行时 i18n / 语言切换机制——双语只是"同一代码、不同注释语言"
  的两套静态文件。
