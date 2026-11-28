# 知识卡覆盖门禁（#146）

> 判据一句话：**选项里点名的引理，必须出现在学习者点「我不知道」时实际推送的那张卡里。**
> 可机检部分由 `scripts/card-lint.py` 判；本文件是判据正文、误报治理、已知限制与
> 补卡流程的唯一说明。

## 1. 问题与判据

学习者点「我不知道」时，知识卡应当给出**选项里那几个引理的定义**。`#146` 的现场：
`I.8.le-empty-3`（`h : A ≼ ∅` 要证 `A ⊆ ∅`，选项 `le_empty` / `subset_of_le_empty` /
`subset_le` / `le_trans`）推送的是通用卡 `cardinal-le`（讲 `≼` 本身），四个引理一个都没讲。

判据（`scripts/card-lint.py`，errors 即红）：

1. **题目范围**：有 `options` 的 choice 题（缺省 `kind` 也是 choice）。
2. **点名**：取每个选项里**反引号片段**的第一个标识符，且它后面是结尾 / 空白 /
   `(` / `（` / `：` / `:` ⇒ 视为在点名这个引理：
   - `le_empty`：…（裸名）✓；`subset_trans α A B C h1 h2`（应用形式）✓；
   - `∃ f, Injective f` / `a ∈ univ` / `apply Set.ext`（表达式/策略）✗ 不算点名。
3. **声明来源**：`courses/set-theory/lib` 与 `courses/set-theory/units` 下的
   `.sokonanoda`（题目 `source` 指向的单元都在其中）。只收 `theorem`，或**结果类型是
   `Prop` 的 `def`**（谓词/关系定义，如 `Injective` / `Set.LeftInvOn` / `IsZero`）。
4. **实际推送卡**：用 `engine.explain_for` 求；点名标识符必须出现在**卡片 JSON 文本**
   里（`definition.code`、`oneLiner`、`sections[].items` / `text` / 表格行都算——
   术语可以写在解释里）。没有卡（fallback）⇒ 这些引理全部算未覆盖。
5. **存量**：欠账冻结在 `scripts/card-lint-baseline.json`（键 = `qid:name`，**双向
   棘轮**：基线外新增判红，已修条目没同步收紧也判红）。

## 2. 误报治理（每条都有来由，改判据前先读）

1. **只统计「有声明」的标识符**：泛型缩写（`h` / `a` / `f`）没有声明 ⇒ 不计。
2. **命名空间/类型名不计**：命名空间 = 自己还是别的声明的点号前缀（`Set.Le` 是
   `Set.Le.refl` 的前缀、`Set.Equiv` 是 `Set.Equiv.mk` 的前缀）⇒ 不计；
   结果类型不是 `Prop` 的声明是类型/数据（`Set` / `Rel` / `univ` / `Prod.fst`）
   ⇒ 不计。
3. **表达式不算点名**：`∃ f, Injective f`、`a ∈ univ`、`apply Set.ext` 这类片段
   不是「点名某个引理」，跳过；只有片段**以引理名开头**（裸名或应用形式）才算。
4. **v1 只查不带命名空间的引理名**：`Set.Le.refl`、`Set.mem_image_elim` 这类限定名
   留待后续批次（见 §5 已知限制），避免把命名空间的前缀误当引理。
5. **同名不同义**：判定按**裸名**匹配，跨模块同名引理会被当作同一个名字
   （例：`le_refl` 既是 `≼` 的自反定理，也是 `LeNat` 的构造子）。卡片作者按题目语境
   写对的那条；这是 v1 的已知粗粒度。

## 3. 存量基线（只减不许增）

- `scripts/card-lint-baseline.json`，`entries` 每条形如
  `{"qid": "I.3.ordinal-2", "name": "IsZero", "card": null}`。
- 不在基线里的未覆盖 = 红；基线里有、但现在已经覆盖的条目 = **过期**（也红）——
  必须 `python3 scripts/card-lint.py --update-baseline` 收紧后与内容改动同一 commit。
- 基线**不是豁免**：清账后应保持 `0 条`。若只是暂时不打算清的欠账，要在 PR 里说明。

## 4. 补卡/扩卡流程（备课批次）

1. 先跑 `python3 scripts/card-lint.py --no-baseline` 看未覆盖清单；
2. 按题型分组：**「一道题比较 N 个引理」优先建专用卡**（如 `cardinal-le-empty`），
   不要往通用卡里硬塞；已有卡的同类缺口优先**扩卡**（追加「同名引理速查」一节）；
3. 新卡要能靠 `kps` 命中这些题（卡片加可选 `kps`，`explain_for` kp 优先）；
   注意卡片的 `kps` 会把这些题**全部**路由到这张卡，所以卡里要写清它们点名的引理；
4. 每批跑：

   ```bash
   python3 courses/set-theory/knowledge/validate.py
   python3 courses/set-theory/knowledge/coverage.py --no-report
   python3 courses/set-theory/knowledge/engine.py selftest
   python3 courses/set-theory/knowledge/engine.py simulate --questions 50
   python3 scripts/notation-lint.py
   python3 courses/set-theory/knowledge/text_lint.py
   python3 scripts/card-lint.py --no-baseline
   ```

5. 清完后 `python3 scripts/card-lint.py --update-baseline` 把基线收紧到 0 条；
6. 改了卡面文本后，按 `text-axioms.md` 的规则把新卡文本写给 `text_lint.py` 看
   （errors 0、无新增 warnings）。

> 大改（≥20 张卡）先给方案、经用户确认再落盘（与题库批次同一条纪律）。

## 5. 已知限制（v1 有意划的边界）

- **只查裸引理名**：`Set.mem_image_elim`、`Set.Le.refl` 这类限定名不查。它们的
  命名空间前缀容易被当成「类型名」，v1 选择保守地跳过 —— 后续版本可加「限定名
  = 声明表里的 theorem 全名」这一档。
- **同名不同义**：见 §2.5。
- **声明解析是行首正则**（`theorem` / `def`），不是真 parser：注释里的伪声明不计，
  缩进声明能认；`inductive` / `structure` / `axiom` 不是「引理」来源。
- **只覆盖 set-theory 课的程序树**：题目来源指向 `courses/analysis` 的，声明按
  set-theory 树里的同名副本判定（卷 II 内容在 `courses/set-theory/units/I.6` 等有
  课程副本）；跨课程同名会按裸名合并。

## 6. 门禁接线

- `python3 scripts/card-lint.py`（默认，按基线判红）+ `--selftest`（红/绿 fixtures）；
- `scripts/soko gate`（含 `--fast`）、`scripts/ci-local.sh` 的 gates 段、
  `.github/workflows/ci.yml` 的 `gates-course` job 都会跑这两条；
- 缺脚本 / 解释器 / 数据 ⇒ exit 3，**不静默跳过**。
