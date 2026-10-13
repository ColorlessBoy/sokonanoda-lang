//! Code-action helpers: map next-step suggestions into quick-fix text edits  // 本文件提到的 `docs/**` 指针都指向**已收口删除**的设计文档（原文在 git 历史里）：要读原文用 `git log --all -- <path>`。doc-refs: history-only
//! for open exercises (docs/design/hints-suggestions.md §4.3).
//!
//! 建议生成在 `front::suggest`（kernel 验证优先：exact → rfl → refine →
//! intro；第一条标 preferred）。这里只做映射：
//! * 洞位来自 walk 的 `DeclState.holes`（按洞下标取，绝不扫描文本）；
//! * 替换文本来自 kernel 验证过的候选（exact 的假设名、rfl 的项）或结构
//!   模板（refine / intro）；
//! * 判定永远走 kernel，禁止文本比对（REQUIREMENTS §2.8）。
//!
//! **记法头那两级**（A3/A4 的 **LSP 侧那一半**）：`front::suggest` 的形状判据
//! 全部走**点名头**（`refl_candidate` 要 `Eq α x y`），而课程一律写**记法**
//! （`a = a` / `p ∧ q` / `A ↔ B` 都是 `Expr::Notation`）⇒ 这些目标上梯子是
//! 0 条。本文件补两条后备（`notation_ladder_fallbacks` / 失败声明的整值 rfl），
//! 候选照旧交 kernel 终审；front 学会记法头之后整体删掉。
//! ⚠ **F12b2 之后**：**箭头头**那一档 front 已经学会（表喂进
//! `suggest_with_notations`）⇒ 与 front 的 `Intro` 候选是同一条编辑（A5 去重）；
//! 仍在 front 之外的只有 **`=` 记法头的 rfl** 与 **`∧`/`↔` 记法头的
//! `constructor`** 两档。
//!
//! kernel 拒绝的失败声明（`DeclStatus::Failed`）没有洞，编辑目标是整个
//! 值位（tokenize 定位 `:=` 与值首）。`front::suggest` 的失败声明建议梯子：
//! kernel 验证过的 refl 整值替换（`Eq`/`Iff` 形状声明：`Eq.refl …` /
//! `Iff.rfl`）→ 保留已写 lambda 前缀的部分重置（Reset）→ 整值重启骨架
//! （Restart）——都映射为值位整体替换（docs/design/kernel-taxonomy.md §2）。

use super::render::decl_at;
use sokonanoda_front::compile::{
    CompileOptions, DeclState, DeclStatus, DocumentReport, HoleMode, PreludeMode,
};
use sokonanoda_front::judge::{self as front_judge, Judgement};
use sokonanoda_front::suggest::{self, SuggestionKind};
use sokonanoda_front::{tokenize, NotationDecl, TokenKind};
use std::collections::HashMap;
use tower_lsp::lsp_types::*;

/// The code-action entry: suggestions in order (exact → rfl → refine →
/// intro), the first one marked preferred. A kernel-rejected declaration
/// offers at most one action: the restart skeleton for the whole value.
// 参数 8 个（clippy 上限 7）：第 8 个是 F12b2 新加的**闭包记法表**（`notations`）。
// 它与 `judge_prefix` 同属「闭包上下文」，本该一起打包成结构体；本轮为把改动面压到最小
// （不动 4 个调用点）先显式放行 ⇒ **记为一处待收拾的参数膨胀**（见 W-3/后续课题），不是豁免判定。
#[allow(clippy::too_many_arguments)]
pub(crate) fn code_actions(
    uri: Url,
    text: &str,
    mode: PreludeMode,
    report: &DocumentReport,
    pos: Position,
    // 判据前缀提供者（**闭包上下文**）：项目模式下 `suggest` 靠它看见被导入的
    // 名字；单文件传 `&|_| String::new()`。按声明起点问，便于将来按偏移裁剪。
    judge_prefix: &dyn Fn(usize) -> String,
    // **闭包记法表**（`import` 进来的模块声明的 `infix`/`notation`）：front 的
    // `suggest` 要读**目标文本**（源码级、课程一律写记法）⇒ 表必须一起给它
    // （F12b2）。空表 = 单文件路径，与改前逐字节相同 ✓。
    notations: &[NotationDecl],
    // **目标形状读取器**：把一段目标/类型文本读成 [`GoalShape`]（箭头层数 +
    // 记法头）。
    //
    // 为什么是回调而不是表：解析要带**闭包记法表**（`a ∈ A` / `A ⊆ B` 这些课程库
    // 符号不在内建表里）⇒ 调用方（`lib.rs`）手里有 `ProjectReport::notations`，
    // 由它把表喂进解析 ✓（[`NotationDecl`] 现在是 front 的公开类型 ✓）。
    goal: &dyn Fn(&str) -> Option<GoalShape>,
) -> Option<CodeActionResponse> {
    let d = decl_at(&report.decls, pos.line, pos.character)?;
    if d.status == DeclStatus::Checked {
        return None;
    }
    let options = CompileOptions { prelude: mode };
    // 建议判定需要看到声明本身：给到该声明结束为止的文本。
    //
    // ⚠ **交给 judge 的是 [`judge_source_view`] 那一份**（F12，2026-10-13）：
    // 入口自己的 `import` 行在合成判定文件里会落到**闭包声明之后** ⇒
    // `parse_fragment` 报 `ImportMustPrecedeDeclarations` ⇒ 所有 kernel 终审的
    // 候选（`rfl` / `exact h` / `Eq.refl …`）一律消失。视图是**等长改写** ⇒
    // 偏移逐字节不变 ⇒ 下面所有 `d.span` / `hole_range` 一个都不用动 ✓。
    let judge_src = judge_source_view(&text[..d.span.end.offset.min(text.len())]);
    let src = judge_src.as_str();
    let mut actions: Vec<CodeActionOrCommand> = Vec::new();
    if d.status == DeclStatus::Failed {
        if d.error.is_some() {
            let decl_src =
                &text[d.span.start.offset.min(text.len())..d.span.end.offset.min(text.len())];
            let suggestions = suggest::suggest_with_notations(
                &judge_prefix(d.span.start.offset),
                notations,
                src,
                Some(decl_src),
                &options,
                d,
            );
            // 三类失败声明建议都替换整个值位。
            if let Some(range) = value_range_at(text, d) {
                for suggestion in suggestions {
                    match &suggestion.kind {
                        SuggestionKind::Rfl { text: insert } => {
                            push_action(
                                &mut actions,
                                format!("{insert}（内核验证：两边就是同一个值，直接替换）"),
                                edit_on_hole(uri.clone(), range, insert.clone()),
                            );
                        }
                        SuggestionKind::Reset { new_text } => {
                            push_action(
                                &mut actions,
                                format!(
                                    "保留 fun 前缀，只重置主体为 sorry：{}（从剩余目标继续）",
                                    restart_summary(new_text)
                                ),
                                edit_on_hole(uri.clone(), range, new_text.clone()),
                            );
                        }
                        SuggestionKind::Restart { skeleton } => {
                            push_action(
                                &mut actions,
                                format!(
                                    "用目标形态重启：{}（先搭骨架，内核逐层判）",
                                    restart_summary(skeleton)
                                ),
                                edit_on_hole(uri.clone(), range, skeleton.clone()),
                            );
                        }
                        _ => {}
                    }
                }
                // **A4（LSP 侧那半）**：`=` 记法头的整值 rfl —— front 的
                // `refl_candidate` 只认**点名头** `Eq α x y`（`a = a` 是
                // `Expr::Notation{target:"Eq"}`）⇒ 判红声明上给 0 条，
                // 而同形状的 `Eq.refl α a` 判卷绿（用例实测）。见
                // [`notation_head_operand`] 的说明与 `SOKO_NO_NOTATION_RFL`。
                //
                // 只有 `=` 头有"两边就是同一个值"的**整值**候选；别的记法头
                // （`∧`/`↔`…）在整值替换这条路上没有东西可给（`constructor`
                // 是 tactic，不是项 ✗）⇒ 只认 `Eq`。
                let eq_type = notation_rfl_enabled()
                    .then(|| decl_type_text(decl_src).and_then(goal))
                    .flatten()
                    .and_then(|shape| shape.notation)
                    .filter(|(target, _)| target == "Eq")
                    .map(|(_, lhs)| lhs);
                if let Some(lhs) = eq_type {
                    let cand = format!("Eq.refl {lhs}");
                    let verdict = front_judge::judge_value_replace_with(
                        &judge_prefix(d.span.start.offset),
                        src,
                        &options,
                        d.span,
                        &[cand.as_str()],
                    );
                    if verdict.first() == Some(&Judgement::Match) {
                        push_action(
                            &mut actions,
                            format!("{cand}（内核验证：两边就是同一个值，直接替换）"),
                            edit_on_hole(uri.clone(), range, cand),
                        );
                    }
                }
            }
        }
    } else {
        // **闭包记法表一路传下去**（F12b2）：`suggest` 的 `intro_count`/`intro_text`
        // 读的是**目标文本**（源码级，课程一律写记法）⇒ 不给表就判 0 ⇒
        // 项位（`:= sorry`）的闭包记法箭头目标**一条候选都没有** ✗。
        for suggestion in suggest::suggest_with_notations(
            &judge_prefix(d.span.start.offset),
            notations,
            src,
            None,
            &options,
            d,
        ) {
            match &suggestion.kind {
                SuggestionKind::Exact {
                    binder,
                    hole,
                    text: insert,
                } => {
                    let Some(range) = hole_range_at(text, d, *hole) else {
                        continue;
                    };
                    let title = if d.holes.len() <= 1 {
                        format!("exact {binder}（用假设 {binder} 直接结束证明）")
                    } else {
                        format!("exact {binder}（用假设 {binder} 补第 {} 个洞）", hole + 1)
                    };
                    // **A1**：插入的是 front 按**洞的位置模式**定型、并**用同一份文本**
                    // 判过的 `insert`（tactic 位 `exact hp` / 项位 `hp`）—— LSP 不再自己
                    // 拼前缀 ✗（各拼一次 = 判定与插入两条路，正是 A1 那条缝）。
                    push_action(
                        &mut actions,
                        title,
                        edit_on_hole(uri.clone(), range, insert.clone()),
                    );
                }
                SuggestionKind::Rfl { text: insert } => {
                    let Some(range) = hole_range(text, d) else {
                        continue;
                    };
                    push_action(
                        &mut actions,
                        format!("{insert}（两边本来就是同一个值，rfl 即可）"),
                        edit_on_hole(uri.clone(), range, insert.clone()),
                    );
                }
                SuggestionKind::Refine => {
                    let Some(edit) = refine_edit(uri.clone(), text, d) else {
                        continue;
                    };
                    let template = d.refine_template.clone().unwrap_or_default();
                    push_action(
                        &mut actions,
                        format!("refine {template}（按构造子拆分子目标）"),
                        edit,
                    );
                }
                SuggestionKind::Intro { text: insert } => {
                    let Some(goal_text) = &d.goal else {
                        continue;
                    };
                    // **同一个形状读取器**（F12b）：这条守卫以前自己 `parse_expr_text`
                    // （内建表）⇒ 闭包记法目标上会把 front 的候选**丢掉**；现在两边
                    // 都用闭包记法表解析（F12b2 起 front 的 `intro_count` 也拿到了表
                    // ⇒ 这一档对**两个位置**都成立）。形状只有一处实现 ✓。
                    let Some(intros) = intro_count(goal, goal_text) else {
                        continue;
                    };
                    // **A1**：文本由 front 按落点那个洞的位置模式定型（tactic 位
                    // `intro x` / 项位 `fun (x : T) => sorry`）—— 落点仍是本地的
                    // `hole_range`（LSP 坐标），两者按同一条"第一个洞"规则对齐 ✓。
                    let Some(range) = hole_range(text, d) else {
                        continue;
                    };
                    push_action(
                        &mut actions,
                        format!("引入 {intros} 个 binder（把证明写成 lambda 的第一步）"),
                        edit_on_hole(uri.clone(), range, insert.clone()),
                    );
                }
                // 失败声明专属的建议不会出现在开放练习里。
                SuggestionKind::Reset { .. } | SuggestionKind::Restart { .. } => {}
            }
        }
        // **A3（LSP 侧那半）**：单洞开放练习上的记法头 rfl / constructor ——
        // front 的 `refl_candidate` 认不出 `=` 记法头（`a = a`），
        // `refine_template` 也穿不过记法头（`p ∧ q` / `A ↔ B`）⇒ 这些目标上
        // 灯泡给 0 条（用例实测四处全空）。见下面两个函数的说明。
        notation_ladder_fallbacks(
            &mut actions,
            (&uri, src),
            text,
            d,
            &options,
            judge_prefix,
            goal,
        );
    }
    if actions.is_empty() {
        return None;
    }
    // **A5**：同一份编辑给两条候选 = 纯噪声 —— 实测判红的
    // `theorem fail_fun (p q : Prop) : p → p := fun (x : p) => q` 上，
    // `Reset` 与 `Restart` 生成**逐字节相同**的 `fun (x : p) => sorry`
    //（只有标题不同）。判据 = **装配好的编辑**（LSP 侧那份唯一文本，不再各算
    // 一份 ✗，"判定与插入两条路"正是这条缝的病根）；保留**先出现**的那条
    //（梯子按强度排序 ⇒ 第一条的说法更贴切）。
    // 逃生门 `SOKO_NO_CODE_ACTION_DEDUP=1` ⇒ 回到旧行为（反向验证）。
    if std::env::var_os("SOKO_NO_CODE_ACTION_DEDUP").is_none() {
        let mut seen: Vec<String> = Vec::new();
        actions.retain(|action| {
            let CodeActionOrCommand::CodeAction(action) = action else {
                return true;
            };
            let key = edit_fingerprint(action.edit.as_ref());
            if seen.contains(&key) {
                return false;
            }
            seen.push(key);
            true
        });
    }
    // 恰好第一条标 preferred（TryThis / rust-analyzer 惯例）。
    if let Some(CodeActionOrCommand::CodeAction(first)) = actions.first_mut() {
        first.is_preferred = Some(true);
    }
    Some(actions)
}

/// **交给 `judge_*` 的文档视图**：把入口自己的 `import` 行**等长**改写成注释。
///
/// ## 为什么必须有它（F12，2026-10-13）
///
/// `judge_*` 合成的判定文件 = **闭包上下文**（`extra_prefix`，LSP 侧由
/// `front::query::judge_prefix` 给）+ **文档前缀** + 合成声明 —— 拼接点在
/// `front::judge::synthesized_prefix(extra_prefix, &doc_src[..decl_start])`。
/// 文档前缀是**原文**，里面就有入口自己的 `import` 行 ⇒ 合成文本里 `import`
/// 落在**闭包的声明之后**，而 `parse_fragment`
/// （`front::parser::parse_file_with`）要求 `import` 先于一切声明 ⇒ 整段前缀报
/// `ImportMustPrecedeDeclarations` ⇒ 所有 `judge_*` 一律
/// `all_parse_error("前缀源码无法解析")` ⇒ **凡要 kernel 终审的候选全消失**
/// （`rfl` / `exact h` / `Eq.refl …`），只有不过 judge 的结构性候选
/// （`constructor` / `intro` / `Restart`）活着 —— 所以现场看起来只坏了"一半"。
///
/// 触发条件恰好是"**闭包里真有声明**"：闭包前缀为空时（单文件、或只 import
/// 一门**零声明**的注释文件 —— `courses/set-theory/lib/Logic.sokonanoda` 正是
/// 这种），入口的 `import` 仍排在最前 ⇒ 不触发。这就是"单测绿、真机空"的缝。
///
/// ## 为什么改在 LSP 侧，而不是 `front::judge`
///
/// 判定与建议是两条路。`front` 那一半的修法是让 `synthesized_prefix` 也把文档
/// 前缀过一遍 `importless_source` —— 那要动**所有** `judge_*` 调用点共享的判定
/// 代码；本层能做的是**把手里的文本换成等价视图**，判定代码一行不动
/// （硬规则 1：接受/拒绝、事件计数、`--json` 逐字节不变）。闭包上下文已由
/// `extra_prefix` 给足 ⇒ 入口那几行 `import` 对判定本来就是冗余的。
///
/// ## 为什么是"等长改写"而不是"删行"
///
/// `DeclState` 的 `span` / `holes` / `hole_modes` 都是**原文坐标**（`judge_*`
/// 按它切声明、按它校验"洞就是 `sorry`"）⇒ 删行会让所有偏移失效。把 `import`
/// 六个字节原地写成 `------`（`--` 起行注释）⇒ **长度逐字节不变**、偏移全部
/// 照旧 ✓，而 parser 再也看不到 import 命令 ✓。行首判定与
/// `front::project::importless_source` 的 `is_import_line` 同一条
/// （`trim_start` 后是 `import ` 或整行 `import`；注释行里的 "import" 字样不动）。
///
/// **退役条件**：`front::judge` 的 `synthesized_prefix` 自己把文档前缀过一遍
/// `importless_source` 之后，本函数与逃生门 `SOKO_NO_JUDGE_IMPORT_BLANK` 一起删。
fn judge_source_view(text: &str) -> String {
    if judge_import_blank_disabled() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        // 行尾原样保留（`\n` / `\r\n` / 无）—— 换行是偏移与行列坐标的骨架。
        let (body, newline) = match line.strip_suffix('\n') {
            Some(rest) => match rest.strip_suffix('\r') {
                Some(body) => (body, "\r\n"),
                None => (rest, "\n"),
            },
            None => (line, ""),
        };
        let trimmed = body.trim_start();
        if trimmed.starts_with("import ") || trimmed == "import" {
            let indent = body.len() - trimmed.len();
            out.push_str(&body[..indent]);
            out.push_str("------");
            out.push_str(&body[indent + "import".len()..]);
        } else {
            out.push_str(body);
        }
        out.push_str(newline);
    }
    out
}

/// [`judge_source_view`] 的**逃生门**（反向验证用）：`SOKO_NO_JUDGE_IMPORT_BLANK=1`
/// ⇒ 不做等长改写 ⇒ F12 的缝立刻回来（单测
/// `an_imported_library_does_not_kill_the_kernel_verified_actions` 判红，
/// 真机 `editor-probe --case hole-lightbulb` 也回到 0 条）。
fn judge_import_blank_disabled() -> bool {
    std::env::var_os("SOKO_NO_JUDGE_IMPORT_BLANK").is_some()
}

/// 失败声明标题里的骨架摘要：超 40 字符截断加 `…`。
fn restart_summary(skeleton: &str) -> String {
    let mut out: String = skeleton.chars().take(RESTART_SUMMARY_CHARS).collect();
    if skeleton.chars().count() > RESTART_SUMMARY_CHARS {
        out.push('…');
    }
    out
}

const RESTART_SUMMARY_CHARS: usize = 40;

/// The failed declaration's value span: from the first token after `:=`
/// (the lexer skips whitespace/comments) to the declaration span's end —
/// the parser ends the span at the value's last token, so the trailing
/// newline is already excluded. Multi-line values are replaced as a whole.
/// Located by tokenizing the declaration command; never by scanning text.
fn value_range_at(text: &str, d: &DeclState) -> Option<Range> {
    let start = d.span.start.offset.min(text.len());
    let end = d.span.end.offset.min(text.len());
    let decl_src = text.get(start..end)?;
    let tokens = tokenize(decl_src).ok()?;
    let colon_eq = tokens.iter().position(|t| t.kind == TokenKind::ColonEq)?;
    let value_tok = tokens.get(colon_eq + 1)?;
    if value_tok.kind == TokenKind::Eof {
        return None;
    }
    range_at_offsets(text, start + value_tok.span.start.offset, end)
}

/// Byte offsets → 0-based LSP range（`offset_to_line_col` 是 1 基）。
fn range_at_offsets(text: &str, start: usize, end: usize) -> Option<Range> {
    let (sl, sc) = offset_to_line_col(text, start);
    let (el, ec) = offset_to_line_col(text, end);
    Some(Range {
        start: Position {
            line: (sl - 1) as u32,
            character: (sc - 1) as u32,
        },
        end: Position {
            line: (el - 1) as u32,
            character: (ec - 1) as u32,
        },
    })
}

/// 编辑的**指纹**（A5 去重用）：URI + 每个 edit 的 range 与替换文本。
///
/// 为什么不用 `serde_json::to_string(&edit)`：`changes` 是 `HashMap`（迭代序
/// 随机 ✗），同一份编辑可能给出两个指纹。这里按键排序后拼，恒定 ✓。
fn edit_fingerprint(edit: Option<&WorkspaceEdit>) -> String {
    let Some(edit) = edit else {
        return String::new();
    };
    let mut out = String::new();
    if let Some(changes) = &edit.changes {
        let mut keys: Vec<(&Url, &Vec<TextEdit>)> = changes.iter().collect();
        keys.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
        for (uri, edits) in keys {
            out.push_str(uri.as_str());
            for e in edits {
                out.push_str(&format!(
                    "|{}:{}:{}:{}:{}",
                    e.range.start.line,
                    e.range.start.character,
                    e.range.end.line,
                    e.range.end.character,
                    e.new_text
                ));
            }
        }
    }
    out
}

/// **记法头 / 箭头头那几级的灯泡后备**（A3/A4/F12b 的 **LSP 侧那一半**）。
///
/// **为什么需要它们**：`front::suggest` 的形状判据全部走**点名头** ——
/// `refl_candidate` 要 `Eq α x y` / `Iff A B`（外加 `↔` 记法），
/// `refine_template` 来自 walk 的构造子识别；而课程一律写**记法**
/// （`a = a` / `p ∧ q` / `A ↔ B` 都是 `Expr::Notation`，不是 `Ident` 应用节点）
/// ⇒ 这些目标上梯子是 **0 条**。三级后备：
///   * **箭头/量词头**（F12b）⇒ `intro x`。**F12b2 起 front 那半已经学会**
///     （`intro_count`/`intro_text` 现在拿得到闭包记法表 ⇒ 目标文本
///     `a ∈ A → a ∈ B` 读得回来）⇒ 这一档与 front 的 `Intro` 候选**逐字节同一条
///     编辑、同一句标题**（A5 的指纹去重收掉重复项，用户只看见一条 ✓）。
///     保留它是**兜底**：本函数不依赖 front 的建议梯子本身（哪怕 `Intro` 那条
///     因为别的原因没产生，箭头头这一档照样给得出 `intro x`）✓。
///     ⚠ 历史（为什么当初 LSP 得自己给）：front 的 `intro_count`/`intro_text`
///     用 `parse_expr_text`（**只带内建表**）⇒ 闭包记法（`∈` / `⊆`）的目标解析
///     失败 ⇒ 一条 `Intro` 都不产生；而本函数以前只问"**体内**是不是记法头"，
///     [`peel_binders`] 把箭头剥掉 ⇒ 连"目标头是箭头"都看不出来 ⇒ 真机课程画布
///     练习 1（`a ∈ A → a ∈ B`）0 条。
///   * **`=` 记法头** ⇒ `rfl`（tactic 位）/ `Eq.refl <左操作数>`（项位），
///     候选**照旧交 kernel 终审**（`judge_hole_fill_with` / `judge_value_replace_with`）
///     ⇒ 判不过就**不出现**，零假绿面 ✓；
///   * **`∧`/`↔` 记法头** ⇒ **结构性** `constructor`（拆子目标）。为什么不能
///     kernel 验证：`judge_hole_fill` 的语义是"填进去后整条命令闭合"，
///     而 `constructor` **必然**留下两个子目标 ⇒ 判定层如实回
///     `elab-hole-misplaced`（"洞不在可填写的位置"）—— 那是"tactic 成功了"
///     的信号，但它是个**错误码**，拿它当正面判据 = 咬不住的守卫 ✗。
///     所以这一条与 `Intro` / `Restart` 同档（结构建议，内核在学生写下的
///     每一步之后照旧终审）。
///
/// **目标头只有一个**：箭头/量词头时**体内**的记法头不是目标头 —— 在它上面给
/// `constructor`/`rfl` 是死胡同（`p → q ∧ r` 上 `constructor` 判不了）✗ ⇒
/// 这一档直接 `return`（[`GoalShape::notation`] 在箭头头上本来就是 `None`）。
///
/// **边界（如实）**：`=` 记法头**不写**显式类型实参（记法形态里没有它）——
/// `Eq.refl` 的首个参数是隐式的，判卷层能从期望类型反解（实测 `a = a` 上
/// `Eq.refl a` 判 `Match`）。副作用：生成的是 `Eq.refl a` 而不是 front 那条
/// `Eq.refl.{u} α a`；两者在真 Lean 4 里**都合法**（`Init/Core.lean` 的
/// `Eq.refl` 首参就是隐式）。`intro` 这一级**只给 tactic 位**（F12b 时的取舍）：
/// 项位要 `fun (x : T) => sorry`，那个文本得真剥一层才知道 `T`（front 的
/// `intro_text`）—— **项位由 front 那条给**（F12b2 已通：判据
/// `imported_term_position_notation_arrow_goal_gets_the_intro_step` ✓）。
///
/// **退役判据**：front 的 `refl_candidate` / refine 识别也学会（带闭包记法表的）
/// 记法头之后，这个函数应当**整体删掉**（判定形状只允许有一个实现）——
/// 箭头头那一档**已经**被 front 覆盖（F12b2），`=`/`∧`/`↔` 两档仍在 front 之外。
/// 逃生门 `SOKO_NO_NOTATION_RFL=1`（反向验证用，见 VERDICT）。
fn notation_ladder_fallbacks(
    actions: &mut Vec<CodeActionOrCommand>,
    // `(uri, src)` 打成一包：本函数原为 8 参 ⇒ clippy `too_many_arguments`（8/7），
    // CI 的 `lint-clippy` 是 `-D warnings` ⇒ 判红。两者只在本函数里成对使用 ✓。
    (uri, src): (&Url, &str),
    text: &str,
    d: &DeclState,
    options: &CompileOptions,
    judge_prefix: &dyn Fn(usize) -> String,
    goal: &dyn Fn(&str) -> Option<GoalShape>,
) {
    if !notation_rfl_enabled() {
        return;
    }
    // 只对**单洞、无子目标**的开放练习下这一手：多洞时 `judge_hole_fill` 读不出
    // "这一填对不对"（其余洞仍是 `sorry` ⇒ 合成声明照样 Open，见 `front::suggest`
    // 的同一条注释）⇒ 宁可少给，不给没判过的 ✗。
    if d.holes.len() != 1 || !d.sub_goals.is_empty() {
        return;
    }
    let (Some(hole), Some(mode)) = (d.holes.first().copied(), d.hole_modes.first().copied()) else {
        return;
    };
    let Some(range) = hole_range(text, d) else {
        return;
    };
    let Some(goal_text) = d.goal.as_deref() else {
        return;
    };
    let Some(shape) = goal(goal_text) else {
        return;
    };
    // **① 箭头/量词头**（F12b）：`a ∈ A → a ∈ B` 的第一步就是 `intro`。文本与
    // 标题跟 front 那条**逐字相同**（同一步骤、同一句话；装配好的编辑也相同 ⇒
    // A5 的指纹去重会把重复项收掉，用户只看见一条 ✓）。
    if shape.binders > 0 {
        if mode == HoleMode::Tactic {
            push_action(
                actions,
                format!(
                    "引入 {} 个 binder（把证明写成 lambda 的第一步）",
                    shape.binders
                ),
                edit_on_hole(uri.clone(), range, "intro x".to_string()),
            );
        }
        // 目标头就是箭头 ⇒ 体内的记法头不是目标头（见上）✗。
        return;
    }
    let Some((target, operand)) = shape.notation else {
        return;
    };
    let insert = match (target.as_str(), mode) {
        // `=` 记法：tactic 位就是 `rfl` tactic（本语言的 `rfl` 是 tactic，
        // `Eq.refl …` 是项 ✗）；项位是 `Eq.refl <左操作数>`。
        ("Eq", HoleMode::Tactic) => "rfl".to_string(),
        ("Eq", HoleMode::Term) => format!("Eq.refl {operand}"),
        // `∧` / `↔`：拆子目标。只在这两个**内建记法的目标名**上给 —— 别的记法
        // （`∈` / `⊆` / 课程自定义）背后不一定是归纳类型 ⇒ 给了就是死胡同 ✗。
        ("And" | "Iff", HoleMode::Tactic) => "constructor".to_string(),
        _ => return,
    };
    let verified = target == "Eq";
    if verified {
        let verdict = front_judge::judge_hole_fill_with(
            &judge_prefix(d.span.start.offset),
            src,
            options,
            d.span,
            hole,
            &[insert.as_str()],
        );
        if verdict.first() != Some(&Judgement::Match) {
            return;
        }
    }
    let title = match (target.as_str(), mode) {
        ("Eq", HoleMode::Tactic) => "rfl（内核验证：两边本来就是同一个值）".to_string(),
        _ if verified => format!("{insert}（内核验证：两边本来就是同一个值）"),
        _ => format!("constructor（把 `{target}` 目标拆成两个子目标，内核随后逐层判）"),
    };
    push_action(actions, title, edit_on_hole(uri.clone(), range, insert));
}

/// **形状后备阶梯的逃生门**（反向验证用）：`SOKO_NO_NOTATION_RFL=1` 时
/// `notation_ladder_fallbacks` 的**三级**（箭头头的 `intro` · 开放练习的
/// rfl/constructor · 判红声明的 rfl 整值）**整体关闭** ⇒ 步骤 A3/A4 必须回到
/// "不一致"（读数见 `.cache/ux/fix-lspA/VERDICT.md`）。
/// ⚠ **F12b2 之后它不再单独咬 F12b 那条判据**（如实）：`imported_notation_arrow_goal_gets_the_intro_step`
/// 现在由 **front** 那条 `Intro` 候选满足（闭包记法表进了 `intro_count`/
/// `intro_text`）⇒ **只关本阶梯，它照样绿**（实测 ✓）—— 那正是"退役判据"想要的
/// 方向：箭头头这一档的所有权已经从 LSP 后备**移回 front** ✓。
/// ⇒ F12b 那条的反向验证要**两条通道一起关**（本逃生门 **+** front 侧那条
/// 一次性闸门：让 `suggest_with_notations` 把表当空表用，读数见 VERDICT）⇒
/// tactic 位与项位的箭头头判据**一起**判红 ✓。那条闸门是**一次性的**、跑完已
/// 还原（`grep` = 0 ✗ 不留）。
/// 本逃生门**单独**仍咬得住的是 `=`/`∧`/`↔` 那两档（A3 的
/// `A3-codeaction-ladder-thin` ⇒ exit 1 ✓）。
///
/// 名字保留 `…NOTATION_RFL`（2026-10-13）：它是 A3/A4 那两轮**已在台账里**的
/// 逃生门名，改名会让那两条反向验证失效 ✗ —— 语义已扩到整条阶梯，名字按"历史
/// 名 + 本注释"处理（比引一个新名、留两个逃生门干净 ✓）。
fn notation_rfl_enabled() -> bool {
    std::env::var_os("SOKO_NO_NOTATION_RFL").is_none()
}

/// **目标形状**：箭头层数 + 记法头。由调用方（`lib.rs`）用**闭包记法表**解析
/// 后交回 —— 解析只做一次，形状只有一处实现 ✓。
///
/// 为什么需要它（F12b，2026-10-13）：LSP 以前只问"**体内**是不是记法头"
/// （`notation_head_operand` 先 [`peel_binders`]），于是 `a ∈ A → a ∈ B` 这种
/// **箭头头**目标上箭头被剥掉、只剩 `Set.mem` ⇒ 既看不出"这是箭头"（给不出
/// `intro`），`Set.mem` 也不在硬编码的 `Eq`/`And`/`Iff` 三族里 ⇒ 0 条。
/// 形状一次读全：**先看箭头，再看体内的记法头** —— 目标头只有一个 ✓。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GoalShape {
    /// 可剥的 binder 层数（`Forall` 的 `binders.len()` / `Arrow` 的 1）；
    /// 0 = 目标头不是箭头/量词。
    pub(crate) binders: usize,
    /// **目标头**的记法 `(目标名, 左操作数源码切片)`；`None` = 目标头不是记法
    /// （箭头头时**体内**的记法头不是目标头 ⇒ 这里也如实是 `None`）。
    pub(crate) notation: Option<(String, String)>,
}

/// 解析好的表达式 + 它来自的那段文本 ⇒ [`GoalShape`]（`lib.rs` 的闭包调它）。
pub(crate) fn goal_shape(expr: &sokonanoda_front::Expr, ty_text: &str) -> GoalShape {
    let binders = match expr {
        sokonanoda_front::Expr::Forall { binders, .. } => binders.len(),
        sokonanoda_front::Expr::Arrow { .. } => 1,
        _ => 0,
    };
    GoalShape {
        binders,
        notation: (binders == 0)
            .then(|| notation_head_operand(expr, ty_text))
            .flatten(),
    }
}

/// 解析好的表达式是**记法头**时返回 `(目标名, 左操作数源码切片)`。
///
/// 解析在调用方（`lib.rs`，它握着闭包记法表）走 `front::proof::parse_expr_text_with`
/// —— **与 front 自己读目标文本同一份 parser**，不是 LSP 另造一套；它内部带
/// `#check ` 前缀 ⇒ 切片要减掉 `CHECK_PREFIX_LEN`（`suggest.rs::check_slice`
/// 是同一条约定）。
pub(crate) fn notation_head_operand(
    expr: &sokonanoda_front::Expr,
    ty_text: &str,
) -> Option<(String, String)> {
    let body = peel_binders(expr);
    let sokonanoda_front::Expr::Notation { target, lhs, .. } = body else {
        return None;
    };
    let lhs = lhs.as_deref()?;
    let span = lhs.span();
    let start = span.start.offset.checked_sub(CHECK_PREFIX_LEN)?;
    let end = span.end.offset.checked_sub(CHECK_PREFIX_LEN)?;
    Some((target.clone(), ty_text.get(start..end)?.trim().to_string()))
}

/// 剥掉 `∀`/`→` 层（声明类型文本是整句量词式，记法头在**体内**）。
fn peel_binders(expr: &sokonanoda_front::Expr) -> &sokonanoda_front::Expr {
    let mut cur = expr;
    loop {
        cur = match cur {
            sokonanoda_front::Expr::Forall { body, .. } => body,
            sokonanoda_front::Expr::Arrow { codomain, .. } => codomain,
            _ => return cur,
        };
    }
}

/// `parse_expr_text` 的 `#check ` 前缀长度（`front::suggest::CHECK_PREFIX_LEN` 同值）。
const CHECK_PREFIX_LEN: usize = "#check ".len();

/// 声明**类型**那一半的源码切片（`:=` 之前、顶层 `:` 之后）—— 与
/// `front::suggest::decl_type_text` **同一份机械**（tokenize 定位，绝不扫文本）。
///
/// ⚠ 它是 front 的**私有**函数 ⇒ 这里是 LSP 侧的镜像。**退役条件**：front 把
/// `refl_candidate` 教成认记法头之后，这个函数与两个后备一起删。
fn decl_type_text(decl_src: &str) -> Option<&str> {
    let tokens = tokenize(decl_src).ok()?;
    let colon_eq = tokens.iter().position(|t| t.kind == TokenKind::ColonEq)?;
    let mut depth = 0usize;
    let mut colon_end = None;
    for tok in &tokens[..colon_eq] {
        match tok.kind {
            TokenKind::LParen | TokenKind::LBrace => depth += 1,
            TokenKind::RParen | TokenKind::RBrace => depth = depth.saturating_sub(1),
            TokenKind::Colon if depth == 0 && colon_end.is_none() => {
                colon_end = Some(tok.span.end.offset);
            }
            _ => {}
        }
    }
    Some(&decl_src[colon_end?..tokens[colon_eq].span.start.offset])
}

fn push_action(actions: &mut Vec<CodeActionOrCommand>, title: String, edit: WorkspaceEdit) {
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title,
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: None,
        edit: Some(edit),
        command: None,
        is_preferred: None,
        disabled: None,
        data: None,
    }));
}

/// intro 层数：goal 是 Forall/Arrow 时可剥的 binder 数；`None` 表示不适用。
///
/// **走同一个形状读取器**（F12b）：以前这里自己 `parse_expr_text`（**内建**记法表）
/// ⇒ 闭包记法（`∈` / `⊆`）的目标读不出来 ⇒ front 那条 `Intro` 候选会被这条守卫
/// **丢掉**。F12b2 起 front 的 `intro_count` 也拿到同一张表（`suggest_with_notations`）
/// ⇒ 守卫与生产者**同解**（都用闭包记法表解析同一段目标文本）✓。形状只有一处实现 ✓。
fn intro_count(goal: &dyn Fn(&str) -> Option<GoalShape>, goal_text: &str) -> Option<usize> {
    goal(goal_text)
        .map(|shape| shape.binders)
        .filter(|binders| *binders > 0)
}

/// Locate the `hole`-th `sorry` inside the declaration (0-based, from the
/// walk-derived `DeclState.holes`); returns the 0-based LSP range covering
/// the hole. The server derives hole positions from the walk — never by
/// scanning text.
pub(crate) fn hole_range_at(text: &str, d: &DeclState, hole: usize) -> Option<Range> {
    let span = d.holes.get(hole)?;
    let (hl, hc) = offset_to_line_col(text, span.start.offset);
    let (el, ec) = offset_to_line_col(text, span.end.offset);
    // `offset_to_line_col` is 1-based (matching our Spans); LSP wants 0-based.
    Some(Range {
        start: Position {
            line: (hl - 1) as u32,
            character: (hc - 1) as u32,
        },
        end: Position {
            line: (el - 1) as u32,
            character: (ec - 1) as u32,
        },
    })
}

/// The first hole (smallest offset; the walk orders holes by offset).
pub(crate) fn hole_range(text: &str, d: &DeclState) -> Option<Range> {
    let index = d
        .holes
        .iter()
        .enumerate()
        .min_by_key(|(_, span)| span.start.offset)
        .map(|(i, _)| i)?;
    hole_range_at(text, d, index)
}

pub(crate) fn edit_on_hole(uri: Url, range: Range, new_text: String) -> WorkspaceEdit {
    let mut changes = HashMap::new();
    changes.insert(uri, vec![TextEdit { range, new_text }]);
    WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    }
}

/// Replace the first `sorry` with the constructor skeleton the walk recovered
/// from the document (auto-filled parameters + one `sorry` per proof field),
/// e.g. `And.intro a b sorry sorry`. The suggestion is structural (from the
/// declaration's own axiom/ctor shape); the kernel stays the judge for
/// whatever the learner writes into the sub-holes.
pub(crate) fn refine_edit(uri: Url, text: &str, d: &DeclState) -> Option<WorkspaceEdit> {
    let template = d.refine_template.as_deref()?;
    let range = hole_range(text, d)?;
    Some(edit_on_hole(uri, range, template.to_string()))
}

pub(crate) fn offset_to_line_col(text: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, ch) in text.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

#[cfg(test)]
mod tests {
    use crate::testutil::{
        call, did_open, did_open_at, handshake, handshake_with_root, lsp_pos, offset_of,
        position_json, shutdown, test_service, wait_diagnostics, wait_diagnostics_for, URI,
    };
    use crate::Backend;
    use serde_json::json;
    use tower_lsp::jsonrpc::Request as RpcRequest;
    use tower_lsp::lsp_types::*;
    use tower_lsp::{ClientSocket, LspService};

    /// handshake + didOpen + 等到首轮诊断（报告已就绪）。
    async fn opened(src: &str) -> (LspService<Backend>, ClientSocket) {
        let (mut service, mut socket) = test_service();
        handshake(&mut service).await;
        did_open(&mut service, src).await;
        let _ = wait_diagnostics(&mut socket, "diagnostics").await;
        (service, socket)
    }

    /// 光标放在第一个 `sorry` 上的 codeAction 响应（只保留 CodeAction）。
    async fn code_actions_for(service: &mut LspService<Backend>, src: &str) -> Vec<CodeAction> {
        let hole_start = lsp_pos(src, offset_of(src, "sorry"));
        let result = call(
            service,
            RpcRequest::build("textDocument/codeAction")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "range": {"start": position_json(hole_start), "end": position_json(hole_start)},
                    "context": {"diagnostics": []},
                }))
                .id(90)
                .finish(),
        )
        .await
        .expect("codeAction must answer");
        let actions: Option<CodeActionResponse> =
            serde_json::from_value(result).expect("valid CodeActionResponse");
        actions
            .expect("code actions")
            .into_iter()
            .filter_map(|a| match a {
                CodeActionOrCommand::CodeAction(action) => Some(action),
                CodeActionOrCommand::Command(_) => None,
            })
            .collect()
    }

    /// 第一个编辑的 (起点, 替换文本)。
    fn first_edit(action: &CodeAction) -> (Position, String) {
        let edit = action.edit.as_ref().expect("action carries an edit");
        let changes = edit.changes.as_ref().expect("changes map");
        let edits = changes.values().next().expect("one document's edits");
        let edit = edits.first().expect("one edit");
        (edit.range.start, edit.new_text.clone())
    }

    /// 第一个编辑的完整 (range, 替换文本)。
    fn first_edit_full(action: &CodeAction) -> (Range, String) {
        let edit = action.edit.as_ref().expect("action carries an edit");
        let changes = edit.changes.as_ref().expect("changes map");
        let edits = changes.values().next().expect("one document's edits");
        let edit = edits.first().expect("one edit");
        (edit.range, edit.new_text.clone())
    }

    /// 光标放在 `offset` 上的 codeAction 响应（`None` = 无建议）。
    async fn code_actions_at(
        service: &mut LspService<Backend>,
        src: &str,
        offset: usize,
    ) -> Option<Vec<CodeAction>> {
        let pos = lsp_pos(src, offset);
        let result = call(
            service,
            RpcRequest::build("textDocument/codeAction")
                .params(json!({
                    "textDocument": {"uri": URI},
                    "range": {"start": position_json(pos), "end": position_json(pos)},
                    "context": {"diagnostics": []},
                }))
                .id(80)
                .finish(),
        )
        .await
        .expect("codeAction must answer");
        let actions: Option<CodeActionResponse> =
            serde_json::from_value(result).expect("valid CodeActionResponse");
        actions.map(|resp| {
            resp.into_iter()
                .filter_map(|a| match a {
                    CodeActionOrCommand::CodeAction(action) => Some(action),
                    CodeActionOrCommand::Command(_) => None,
                })
                .collect()
        })
    }

    fn titles_of(actions: &[CodeAction]) -> Vec<&str> {
        actions.iter().map(|a| a.title.as_str()).collect()
    }

    /// `code_actions_at` 的**项目模式**版本：URI 由调用方给（项目夹具在临时
    /// 目录里，不是 `testutil::URI`）。
    async fn code_actions_at_uri(
        service: &mut LspService<Backend>,
        uri: &Url,
        src: &str,
        offset: usize,
    ) -> Option<Vec<CodeAction>> {
        let pos = lsp_pos(src, offset);
        let result = call(
            service,
            RpcRequest::build("textDocument/codeAction")
                .params(json!({
                    "textDocument": {"uri": uri},
                    "range": {"start": position_json(pos), "end": position_json(pos)},
                    "context": {"diagnostics": []},
                }))
                .id(81)
                .finish(),
        )
        .await
        .expect("codeAction must answer");
        let actions: Option<CodeActionResponse> =
            serde_json::from_value(result).expect("valid CodeActionResponse");
        actions.map(|resp| {
            resp.into_iter()
                .filter_map(|a| match a {
                    CodeActionOrCommand::CodeAction(action) => Some(action),
                    CodeActionOrCommand::Command(_) => None,
                })
                .collect()
        })
    }

    const SPINE_WHOLE_DOC: &str = "axiom And : Prop -> Prop -> Prop\n\
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
theorem t : (a : Prop) -> (b : Prop) -> (whole : And a b) -> (ha : a) -> (hb : b) -> And a b := \
fun (a : Prop) => fun (b : Prop) => fun (whole : And a b) => fun (ha : a) => fun (hb : b) => \
And.intro a b sorry sorry\n";

    #[tokio::test]
    async fn code_action_spine_hole_exact_targets_its_own_hole() {
        let (mut service, _socket) = opened(SPINE_WHOLE_DOC).await;
        let actions = code_actions_for(&mut service, SPINE_WHOLE_DOC).await;
        let first_hole = lsp_pos(SPINE_WHOLE_DOC, offset_of(SPINE_WHOLE_DOC, "sorry"));
        let second_hole = lsp_pos(
            SPINE_WHOLE_DOC,
            SPINE_WHOLE_DOC.rfind("sorry").expect("2nd hole"),
        );
        // 假设 ha : a 恰是第 1 个子洞的期望类型 → 洞位正确。
        let ha = actions
            .iter()
            .find(|a| a.title.contains("exact ha"))
            .expect("exact ha offered for the first sub-hole");
        let (start, text) = first_edit(ha);
        assert_eq!(start, first_hole, "ha targets the first spine hole");
        assert_eq!(text, "ha");
        assert!(ha.title.contains("补第 1 个洞"), "title: {:?}", ha.title);
        // 假设 hb : b 恰是第 2 个子洞的期望类型。
        let hb = actions
            .iter()
            .find(|a| a.title.contains("exact hb"))
            .expect("exact hb offered for the second sub-hole");
        let (start, text) = first_edit(hb);
        assert_eq!(start, second_hole, "hb targets the second spine hole");
        assert_eq!(text, "hb");
        assert!(hb.title.contains("补第 2 个洞"), "title: {:?}", hb.title);
        // 回归：匹配外层 goal 的假设不得被塞进子洞。
        assert!(
            !actions.iter().any(|a| a.title.contains("exact whole")),
            "outer-goal hypothesis must not be suggested for a sub-hole: {:?}",
            titles_of(&actions)
        );
        shutdown(&mut service).await;
    }

    const EQ_GOAL_DOC: &str =
        "theorem eq_t : (a : Nat) -> Eq.{1} Nat a a := fun (a : Nat) => sorry\n";

    #[tokio::test]
    async fn code_action_rfl_for_eq_and_iff_goals_only() {
        let (mut service, _socket) = opened(EQ_GOAL_DOC).await;
        let actions = code_actions_for(&mut service, EQ_GOAL_DOC).await;
        let rfl = actions
            .iter()
            .find(|a| a.title.contains("rfl"))
            .expect("an Eq goal gets a kernel-verified rfl");
        assert!(
            rfl.title.contains("两边本来就是同一个值"),
            "title: {:?}",
            rfl.title
        );
        // 标题带**真的候选文本**（替换什么就说什么）。
        assert!(
            rfl.title.contains("Eq.refl.{1} Nat a"),
            "title: {:?}",
            rfl.title
        );
        let (start, text) = first_edit(rfl);
        assert_eq!(text, "Eq.refl.{1} Nat a", "kernel-verified rfl term");
        assert_eq!(
            start,
            lsp_pos(EQ_GOAL_DOC, offset_of(EQ_GOAL_DOC, "sorry")),
            "rfl targets the hole"
        );
        shutdown(&mut service).await;
    }

    const IFF_GOAL_DOC: &str = "theorem iff_t : (a : Prop) -> a ↔ a := fun (a : Prop) => sorry\n";

    #[tokio::test]
    async fn code_action_iff_goal_offers_the_kernel_verified_rfl() {
        // 与 `by rfl` 能力一致（同一份 `iff_head` 判据）：defeq 的 `↔` 目标
        // 也给 quick-fix，且候选经 kernel 验证（`Iff.rfl`，2026-10-11 从
        // `Iff.refl` 改来：设计 `docs/design/set-option-pp.md` §4）。
        let (mut service, _socket) = opened(IFF_GOAL_DOC).await;
        let actions = code_actions_for(&mut service, IFF_GOAL_DOC).await;
        let rfl = actions
            .iter()
            .find(|a| a.title.contains("Iff.rfl"))
            .expect("a defeq Iff goal gets a kernel-verified rfl");
        assert!(
            rfl.title.contains("两边本来就是同一个值"),
            "title: {:?}",
            rfl.title
        );
        let (start, text) = first_edit(rfl);
        assert_eq!(text, "Iff.rfl");
        assert_eq!(
            start,
            lsp_pos(IFF_GOAL_DOC, offset_of(IFF_GOAL_DOC, "sorry")),
            "rfl targets the hole"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn code_action_non_eq_non_iff_goal_gets_no_rfl() {
        // 非 `Eq`/非 `Iff` 头（箭头目标）不出 rfl。
        let arrow_src = "example : Prop -> Prop := sorry\n";
        let (mut service, _socket) = opened(arrow_src).await;
        let actions = code_actions_for(&mut service, arrow_src).await;
        assert!(
            !actions.iter().any(|a| a.title.contains("rfl")),
            "a non-Eq/non-Iff goal must not get rfl: {:?}",
            titles_of(&actions)
        );
        shutdown(&mut service).await;
    }

    const TRIPLE_DOC: &str = "axiom And : Prop -> Prop -> Prop\n\
axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
theorem t : (a : Prop) -> (b : Prop) -> (k : a -> b -> And a b) -> a -> b -> And a b := \
fun (a : Prop) => fun (b : Prop) => fun (k : a -> b -> And a b) => sorry\n";

    #[tokio::test]
    async fn code_action_marks_exactly_the_first_as_preferred() {
        let (mut service, _socket) = opened(TRIPLE_DOC).await;
        let actions = code_actions_for(&mut service, TRIPLE_DOC).await;
        assert_eq!(
            actions.len(),
            3,
            "exact + refine + intro: {:?}",
            titles_of(&actions)
        );
        assert_eq!(actions[0].is_preferred, Some(true), "first is preferred");
        for action in &actions[1..] {
            assert_eq!(action.is_preferred, None, "only the first is preferred");
        }
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn code_action_orders_exact_refine_intro() {
        let (mut service, _socket) = opened(TRIPLE_DOC).await;
        let actions = code_actions_for(&mut service, TRIPLE_DOC).await;
        let titles = titles_of(&actions);
        assert!(titles[0].contains("exact"), "exact first: {titles:?}");
        assert!(titles[1].contains("refine"), "refine second: {titles:?}");
        assert!(titles[2].contains("引入"), "intro last: {titles:?}");
        shutdown(&mut service).await;
    }

    // ---- 失败声明的重启骨架（docs/design/kernel-taxonomy.md §2）----

    const FAILED_DEF_EQ: &str = "example : (a : Prop) -> a -> a := fun (x : Prop) => 1\n";

    #[tokio::test]
    async fn code_action_restarts_failed_decl_over_the_whole_value_span() {
        let (mut service, _socket) = opened(FAILED_DEF_EQ).await;
        let cursor = offset_of(FAILED_DEF_EQ, "=> 1");
        let actions = code_actions_at(&mut service, FAILED_DEF_EQ, cursor)
            .await
            .expect("a kernel-rejected decl must offer the restarts");
        // 答案以 lambda 开头：Reset（保留前缀）在前，Restart（整值骨架）在后。
        assert_eq!(
            actions.len(),
            2,
            "reset + restart: {:?}",
            titles_of(&actions)
        );
        let reset = &actions[0];
        assert!(
            reset.title.contains("保留 fun 前缀"),
            "title: {:?}",
            reset.title
        );
        assert!(
            reset.title.contains("从剩余目标继续"),
            "title: {:?}",
            reset.title
        );
        assert_eq!(
            reset.is_preferred,
            Some(true),
            "the first failed-decl action is preferred"
        );
        let (_, reset_text) = first_edit_full(reset);
        assert_eq!(
            reset_text, "fun (x : Prop) => sorry",
            "the reset keeps the written lambda prefix"
        );
        let restart = &actions[1];
        assert!(
            restart.title.contains("用目标形态重启"),
            "title: {:?}",
            restart.title
        );
        assert!(
            restart
                .title
                .contains("fun (a : Prop) => fun (x : a) => sorry"),
            "the skeleton summary is in the title: {:?}",
            restart.title
        );
        assert!(
            restart.title.contains("先搭骨架，内核逐层判"),
            "title: {:?}",
            restart.title
        );
        assert_eq!(
            restart.is_preferred, None,
            "only the first action is preferred"
        );
        // 编辑目标 = 整个值位：从 `:=` 后第一个 token 到声明 span 末尾
        // （不含结尾换行），多行值也整体替换。
        let (range, new_text) = first_edit_full(restart);
        assert_eq!(
            range.start,
            lsp_pos(FAILED_DEF_EQ, offset_of(FAILED_DEF_EQ, "fun (x")),
            "the edit starts at the first value token"
        );
        assert_eq!(
            range.end,
            lsp_pos(
                FAILED_DEF_EQ,
                FAILED_DEF_EQ.rfind('\n').expect("trailing newline")
            ),
            "the edit ends at the declaration span's end (before the newline)"
        );
        assert_eq!(new_text, "fun (a : Prop) => fun (x : a) => sorry");
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn code_action_failed_decl_without_peelable_type_gets_none() {
        // kernel 拒绝但类型是 Prop（非 Pi）：没有可剥的望远镜，无建议。
        let src = "def bad : Prop := 1\n";
        let (mut service, _socket) = opened(src).await;
        let actions = code_actions_at(&mut service, src, offset_of(src, "1")).await;
        assert!(
            actions.is_none(),
            "a non-Pi failed decl must get no action: {actions:?}"
        );
        shutdown(&mut service).await;

        // elab 失败（unknown identifier）同样没有可剥的类型：无建议。
        let src = "example : Prop := undefined_name\n";
        let (mut service, _socket) = opened(src).await;
        let actions = code_actions_at(&mut service, src, offset_of(src, "undefined_name")).await;
        assert!(
            actions.is_none(),
            "an elab-failed decl must get no action: {actions:?}"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn code_action_restart_replaces_multiline_values_as_a_whole() {
        let src = "example : (a : Prop) -> a -> a :=\n  fun (x : Prop) => 1\n";
        let (mut service, _socket) = opened(src).await;
        let cursor = offset_of(src, "=> 1");
        let actions = code_actions_at(&mut service, src, cursor)
            .await
            .expect("a kernel-rejected decl must offer the restarts");
        let restart = actions
            .iter()
            .find(|a| a.title.contains("用目标形态重启"))
            .expect("the whole-value restart action");
        let (range, new_text) = first_edit_full(restart);
        assert_eq!(
            range.start,
            lsp_pos(src, offset_of(src, "fun (x")),
            "the edit starts at the first value token, on the value's own line"
        );
        assert_eq!(
            range.end,
            lsp_pos(src, src.rfind('\n').expect("trailing newline")),
            "the edit ends at the declaration span's end"
        );
        assert_eq!(new_text, "fun (a : Prop) => fun (x : a) => sorry");
        // 部分重启同样覆盖整个值位（换行后的缩进不进 new_text）。
        let reset = actions
            .iter()
            .find(|a| a.title.contains("保留 fun 前缀"))
            .expect("the prefix-preserving reset action");
        let (_, reset_text) = first_edit_full(reset);
        assert_eq!(reset_text, "fun (x : Prop) => sorry");
        shutdown(&mut service).await;
    }

    // ---- 失败声明的 kernel 验证 rfl（Eq 形状声明）----

    const FAILED_EQ_LAMBDA: &str = "example : Eq.{1} Nat 2 2 := fun (x : Nat) => 3\n";

    #[tokio::test]
    async fn code_action_failed_eq_decl_offers_kernel_verified_rfl_first() {
        let (mut service, _socket) = opened(FAILED_EQ_LAMBDA).await;
        let cursor = offset_of(FAILED_EQ_LAMBDA, "=> 3");
        let actions = code_actions_at(&mut service, FAILED_EQ_LAMBDA, cursor)
            .await
            .expect("a kernel-rejected Eq decl must offer the verified rfl");
        assert_eq!(actions.len(), 2, "rfl + reset: {:?}", titles_of(&actions));
        let rfl = &actions[0];
        assert!(rfl.title.contains("内核验证"), "title: {:?}", rfl.title);
        assert!(rfl.title.contains("直接替换"), "title: {:?}", rfl.title);
        // 标题带**真的候选文本**（不是写死的 `Eq.refl …`）：替换什么就说什么。
        assert!(
            rfl.title.contains("Eq.refl.{1} Nat 2"),
            "title: {:?}",
            rfl.title
        );
        assert_eq!(rfl.is_preferred, Some(true), "the verified rfl is first");
        // 编辑目标 = 整个值位，替换文本恰为 kernel 验证过的 rfl 项。
        let (range, new_text) = first_edit_full(rfl);
        assert_eq!(new_text, "Eq.refl.{1} Nat 2");
        assert_eq!(
            range.start,
            lsp_pos(FAILED_EQ_LAMBDA, offset_of(FAILED_EQ_LAMBDA, "fun (x")),
            "the edit starts at the first value token"
        );
        assert_eq!(
            range.end,
            lsp_pos(
                FAILED_EQ_LAMBDA,
                FAILED_EQ_LAMBDA.rfind('\n').expect("trailing newline")
            ),
            "the edit ends at the declaration span's end"
        );
        // Reset 其次：保留已写的 lambda 前缀。
        let reset = &actions[1];
        assert!(
            reset.title.contains("保留 fun 前缀"),
            "title: {:?}",
            reset.title
        );
        assert_eq!(reset.is_preferred, None, "only the first is preferred");
        let (_, reset_text) = first_edit_full(reset);
        assert_eq!(reset_text, "fun (x : Nat) => sorry");
        shutdown(&mut service).await;
    }

    // ---- 失败声明的 kernel 验证 rfl（Iff 形状声明，2026-10-10）----

    const FAILED_IFF_DECL: &str = "example : True ↔ True := 3\n";

    #[tokio::test]
    async fn code_action_failed_iff_decl_offers_kernel_verified_rfl() {
        let (mut service, _socket) = opened(FAILED_IFF_DECL).await;
        let cursor = offset_of(FAILED_IFF_DECL, "3");
        let actions = code_actions_at(&mut service, FAILED_IFF_DECL, cursor)
            .await
            .expect("a kernel-rejected Iff decl must offer the verified rfl");
        assert_eq!(actions.len(), 1, "rfl only: {:?}", titles_of(&actions));
        let rfl = &actions[0];
        // 标题带**真的候选文本**（`Iff.rfl`，不是写死的 `Eq.refl …`）。
        assert!(rfl.title.contains("Iff.rfl"), "title: {:?}", rfl.title);
        assert!(rfl.title.contains("内核验证"), "title: {:?}", rfl.title);
        assert_eq!(rfl.is_preferred, Some(true), "the verified rfl is first");
        let (range, new_text) = first_edit_full(rfl);
        assert_eq!(new_text, "Iff.rfl");
        assert_eq!(
            range.start,
            lsp_pos(FAILED_IFF_DECL, offset_of(FAILED_IFF_DECL, "3")),
            "the edit starts at the value"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn code_action_failed_eq_decl_with_non_lambda_answer_gets_only_rfl() {
        let src = "example : Eq.{1} Nat 2 2 := 3\n";
        let (mut service, _socket) = opened(src).await;
        let cursor = offset_of(src, "3");
        let actions = code_actions_at(&mut service, src, cursor)
            .await
            .expect("a kernel-rejected Eq decl must offer the verified rfl");
        assert_eq!(
            actions.len(),
            1,
            "the rfl replaces the whole value; no restart applies: {:?}",
            titles_of(&actions)
        );
        let rfl = &actions[0];
        assert!(rfl.title.contains("内核验证"), "title: {:?}", rfl.title);
        assert_eq!(rfl.is_preferred, Some(true));
        let (range, new_text) = first_edit_full(rfl);
        assert_eq!(new_text, "Eq.refl.{1} Nat 2");
        assert_eq!(
            range.start,
            lsp_pos(src, offset_of(src, "3")),
            "the edit starts at the value"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn code_action_failed_eq_decl_kernel_rejected_rfl_is_dropped() {
        // 2 ≢ 3：候选被内核拒绝，Eq 形状声明又不给重启——无建议。
        let src = "example : Eq.{1} Nat 2 3 := 5\n";
        let (mut service, _socket) = opened(src).await;
        let cursor = offset_of(src, "5");
        let actions = code_actions_at(&mut service, src, cursor).await;
        assert!(
            actions.is_none(),
            "a kernel-rejected rfl candidate must not be offered: {actions:?}"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn code_action_non_lambda_answer_gets_only_restart() {
        let src = "example : (a : Prop) -> a -> a := 1\n";
        let (mut service, _socket) = opened(src).await;
        let cursor = offset_of(src, "1");
        let actions = code_actions_at(&mut service, src, cursor)
            .await
            .expect("a kernel-rejected decl must offer the restart");
        assert_eq!(
            actions.len(),
            1,
            "a non-lambda answer has no prefix to keep: {:?}",
            titles_of(&actions)
        );
        let restart = &actions[0];
        assert!(
            restart.title.contains("用目标形态重启"),
            "title: {:?}",
            restart.title
        );
        assert_eq!(restart.is_preferred, Some(true));
        let (_, new_text) = first_edit_full(restart);
        assert_eq!(new_text, "fun (a : Prop) => fun (x : a) => sorry");
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn code_action_refine_edit_targets_the_hole() {
        // refine 族（构造子骨架）必须替换**洞**本身，与 exact/intro 同址：
        // 自动填充参数 + 每个证明位一个 sorry。
        let src = "axiom And : Prop -> Prop -> Prop\n\
                   axiom And.intro : (a : Prop) -> (b : Prop) -> a -> b -> And a b\n\
                   theorem t : (a : Prop) -> (b : Prop) -> a -> b -> And a b := sorry\n";
        let (mut service, _socket) = opened(src).await;
        let actions = code_actions_for(&mut service, src).await;
        let refine = actions
            .iter()
            .find(|a| a.title.contains("refine And.intro a b sorry sorry"))
            .expect("refine skeleton with auto-filled parameters must be offered");
        let (range, new_text) = first_edit_full(refine);
        assert_eq!(new_text, "And.intro a b sorry sorry");
        let hole = offset_of(src, "sorry");
        assert_eq!(
            range.start,
            lsp_pos(src, hole),
            "refine must start at the hole"
        );
        assert_eq!(
            range.end,
            lsp_pos(src, hole + "sorry".len()),
            "refine must cover exactly the hole"
        );
        shutdown(&mut service).await;
    }

    #[tokio::test]
    async fn code_action_open_goal_without_a_next_step_offers_none() {
        // 目标是不带构造子的公理命题 `P`：没有可填的假设（无 exact/rfl）、
        // 没有 refine 模板（P 不是构造子的目标）、也不是 Pi（无 intro）。
        // refine/intro 两族的「无候选」回退在此守护：不产出任何 action。
        let src = "axiom P : Prop\ntheorem t : P := sorry\n";
        let (mut service, _socket) = opened(src).await;
        let actions = code_actions_at(&mut service, src, offset_of(src, "sorry")).await;
        assert!(
            actions.is_none(),
            "no next step for a bare axiom goal: {actions:?}"
        );
        shutdown(&mut service).await;
    }

    /// **A1（2026-10-12 blocker）：wire 层的判据** —— 灯泡插入的文本必须与**洞的位置
    /// 模式**同形：`by` 块里剩下的目标是 **tactic 位** ⇒ `exact hp`；声明值位是
    /// **项位** ⇒ `hp`（不带 `exact`）。
    ///
    /// 这一层钉的是**用户实际拿到的那段 newText**（三层里的 wire 层）：修前这里读到的
    /// 是 `hp`（而标题写着 `exact hp`）⇒ 插进去语法错误、判卷红 ✗。
    #[tokio::test]
    async fn exact_action_inserts_a_tactic_in_a_by_hole_and_a_term_in_a_value_hole() {
        // ① tactic 位（`by` 块里的洞）。
        let by_src = "theorem by_hole (p : Prop) (hp : p) : p := by\n  sorry\n";
        let (mut service, _socket) = opened(by_src).await;
        let actions = code_actions_for(&mut service, by_src).await;
        let tactic = actions
            .iter()
            .find(|a| a.title.contains("exact hp"))
            .unwrap_or_else(|| panic!("tactic 位要有 `exact hp` 候选：{actions:?}"));
        assert_eq!(
            first_edit(tactic).1,
            "exact hp",
            "tactic 位的插入文本必须带 `exact`（标题与正文一致）"
        );
        shutdown(&mut service).await;

        // ② 项位（声明值位的洞）—— 同一个洞形、同一个假设名，**不带** `exact`。
        let term_src = "theorem term_hole (p : Prop) (hp : p) : p := sorry\n";
        let (mut service, _socket) = opened(term_src).await;
        let actions = code_actions_for(&mut service, term_src).await;
        let term = actions
            .iter()
            .find(|a| a.title.contains("exact hp"))
            .unwrap_or_else(|| panic!("项位要有 `exact hp` 候选：{actions:?}"));
        assert_eq!(
            first_edit(term).1,
            "hp",
            "项位的插入文本必须还是项（`:= exact hp` 不是项）"
        );
        shutdown(&mut service).await;
    }

    #[test]
    fn restart_summary_truncates_at_40_chars() {
        let short = "fun (a : Prop) => fun (x : a) => sorry";
        assert_eq!(
            super::restart_summary(short),
            short,
            "short skeletons pass through"
        );
        let long = "fun (a : Prop) => fun (b : Prop) => fun (c : Prop) => sorry";
        let summary = super::restart_summary(long);
        assert!(summary.ends_with('…'), "truncated summaries end with …");
        assert_eq!(
            summary.chars().count(),
            40 + 1,
            "40 characters plus the ellipsis"
        );
        assert!(long.starts_with(summary.trim_end_matches('…')));
    }

    /// **A3/A4（LSP 侧那半）**：`=` **记法头**（`Expr::Notation{target:"Eq"}`）的
    /// rfl —— front 的 `refl_candidate` 只认点名头 `Eq α x y`，于是这些目标上
    /// 灯泡给 0 条；候选由 kernel 终审（判不过不出现）。
    #[tokio::test]
    async fn eq_notation_goal_gets_a_kernel_verified_rfl() {
        let src = "theorem a3a (\u{3b1} : Type) (a : \u{3b1}) : a = a := by\n  sorry\n";
        let (mut service, _socket) = opened(src).await;
        let actions = code_actions_for(&mut service, src).await;
        let rfl = actions
            .iter()
            .find(|a| a.title.starts_with("rfl（内核验证"))
            .unwrap_or_else(|| panic!("`a = a` 目标要有内核验证过的 rfl：{actions:?}"));
        assert_eq!(
            first_edit(rfl).1,
            "rfl",
            "tactic 位的 rfl 插入文本就是 `rfl`（`Eq.refl …` 是项，插进 by 块是语法错误）"
        );
        shutdown(&mut service).await;
    }

    /// **反向**：`=` 头但**不是**自反的（`a = b`）⇒ 内核判红 ⇒ 0 条 rfl ——
    /// 后备层不许出现假绿（"形状对就报"正是要避免的）。
    #[tokio::test]
    async fn eq_notation_goal_that_is_not_reflexive_gets_no_rfl() {
        let src = "theorem t (\u{3b1} : Type) (a b : \u{3b1}) : a = b := by\n  sorry\n";
        let (mut service, _socket) = opened(src).await;
        // 这一档 front 本来就一条都不给 ⇒ 响应可以是 `None`（用 `_at` 那条口子，
        // 断言的是**没有**新候选，而不是"有响应"）。
        let actions = code_actions_at(&mut service, src, offset_of(src, "sorry"))
            .await
            .unwrap_or_default();
        assert!(
            !actions.iter().any(|a| a.title.contains("rfl")),
            "`a = b` 上不该有 rfl 候选（内核会判红）：{actions:?}"
        );
        assert!(
            !actions.iter().any(|a| a.title.contains("constructor")),
            "`=` 头不该给 constructor：{actions:?}"
        );
        shutdown(&mut service).await;
    }

    /// `∧` / `↔` **记法头**：`constructor`（结构性——`judge_hole_fill` 对"留子目标"
    /// 的 tactic 答不出 `Match`，所以这一条如实不是 kernel 验证档）。
    #[tokio::test]
    async fn and_notation_goal_gets_the_constructor_step() {
        let src = "theorem a3b (p q : Prop) (hp : p) (hq : q) : p \u{2227} q := by\n  sorry\n";
        let (mut service, _socket) = opened(src).await;
        let actions = code_actions_for(&mut service, src).await;
        let ctor = actions
            .iter()
            .find(|a| a.title.starts_with("constructor"))
            .unwrap_or_else(|| panic!("`p \u{2227} q` 目标要有 constructor 这一步：{actions:?}"));
        assert_eq!(first_edit(ctor).1, "constructor");
        shutdown(&mut service).await;
    }

    /// **A4**：判红的 `a = a := a` 给整值 `Eq.refl a`（front 只给 `Iff` 那一半）。
    #[tokio::test]
    async fn failed_eq_declaration_gets_the_refl_value() {
        let src = "theorem fail_eq (\u{3b1} : Type) (a : \u{3b1}) : a = a := a\n";
        let (mut service, _socket) = opened(src).await;
        let actions = code_actions_at(&mut service, src, offset_of(src, "a\n")).await;
        let actions = actions.expect("failed declarations answer with suggestions");
        let rfl = actions
            .iter()
            .find(|a| a.title.starts_with("Eq.refl a"))
            .unwrap_or_else(|| panic!("判红的 `a = a` 要有整值 rfl 候选：{actions:?}"));
        assert_eq!(
            first_edit(rfl).1,
            "Eq.refl a",
            "整值替换：新值就是 `Eq.refl a`"
        );
        shutdown(&mut service).await;
    }

    /// **A5**：同一份编辑不许给两条候选（标题不同不算不同候选）。
    /// 夹具 = 判红的 `fun (x : p) => q`：`Reset` 与 `Restart` 生成**逐字节相同**
    /// 的 `fun (x : p) => sorry`。
    ///
    /// **反向验证**：`SOKO_NO_CODE_ACTION_DEDUP=1`（逃生门）⇒ 本用例判红。
    #[tokio::test]
    async fn duplicate_code_actions_are_deduplicated() {
        let src = "theorem fail_fun (p q : Prop) : p \u{2192} p := fun (x : p) => q\n";
        let (mut service, _socket) = opened(src).await;
        let actions = code_actions_at(&mut service, src, offset_of(src, "fun (x"))
            .await
            .unwrap_or_default();
        // **判据是"只剩一条"**：改前这里给两条（`Reset` + `Restart`，编辑逐字节
        // 相同）⇒ 本断言当场判红（逃生门那档同样判红）。
        assert_eq!(actions.len(), 1, "同一份编辑只许留一条候选：{actions:?}");
        let mut keys: Vec<String> = Vec::new();
        for action in &actions {
            let key = super::edit_fingerprint(action.edit.as_ref());
            assert!(
                !keys.contains(&key),
                "两条候选的编辑逐字节相同（只有标题不同）：{actions:?}"
            );
            keys.push(key);
        }
        shutdown(&mut service).await;
    }

    // ── F12：**带 `import` 的库**那一半（2026-10-13）────────────────────────
    //
    // 现场（真 VS Code + 真课程画布，独立 LSP 客户端复现）：`import lib.Logic`
    // （一门**零声明**的注释文件）下 `a = a` 的洞给 1 条内核验证过的 `rfl`；
    // **只要多一行 `import lib.Set`（有声明）就 1ms 返 0 条** —— 课程画布 import
    // 了 `lib.Set` ⇒ **整门课的洞都没有 quick fix**。
    //
    // 根因（`crates/front/src/judge.rs::judge_hole_fill_uncached`）：
    // `full_prefix = synthesized_prefix(extra_prefix, &doc_src[..decl_start])` ——
    // **闭包源码在前、入口自己的 `import` 行在后**，而 `parse_fragment`
    // （`crates/front/src/parser.rs::parse_file_with`）要求 `import` 先于一切声明
    // ⇒ 整段前缀报 `ImportMustPrecedeDeclarations` ⇒ 所有 `judge_*` 一律
    // `all_parse_error("前缀源码无法解析")` ⇒ **凡要 kernel 终审的候选全消失**
    // （结构性候选 `constructor` 不过 judge，所以它还活着 —— 这正是"看似只坏
    // 一半"）。闭包前缀为空时（单文件 / 只 import 一门零声明的注释文件）入口的
    // `import` 仍排在最前 ⇒ 不触发 ✓ —— 这就是"单测绿、真机空"的缝。
    //
    // 修法在 **LSP 侧**：交给 `judge_*` 的文档视图里把入口自己的 `import` 行
    // **等长改写成注释**（偏移逐字节不变 ⇒ `DeclState` 的 span/洞位照旧有效）；
    // 闭包上下文已由 `extra_prefix` 给足，入口的 `import` 对判定是冗余的。
    //
    // **反向验证**：`SOKO_NO_JUDGE_IMPORT_BLANK=1`（逃生门）⇒ 本用例判红。

    /// 库模块：**有声明 + 一条记法**（与 `courses/set-theory/lib/Set.sokonanoda`
    /// 同形状）。`lib.Logic` 那种零声明的注释文件**不触发**这条缝 ⇒ 夹具必须有声明。
    const IMPORTED_LIB: &str = "\
def Set (\u{3b1} : Type) : Type := \u{3b1} -> Prop\n\
def Set.mem (\u{3b1} : Type) (a : \u{3b1}) (A : Set \u{3b1}) : Prop := A a\n\
infix:50 \" \u{2208} \" => Set.mem\n";

    /// 入口：`import` + 用库记法 + 一个 `=` 记法头的洞（**要 kernel 终审那一档**）。
    const IMPORTED_CANVAS: &str = "\
import SetLib\n\n\
theorem mem_self (\u{3b1} : Type) (a : \u{3b1}) (A : Set \u{3b1}) (h : a \u{2208} A) : a \u{2208} A := by\n\
  sorry\n\
\n\
theorem eq_self (\u{3b1} : Type) (a : \u{3b1}) : a = a := by\n\
  sorry\n";

    #[tokio::test]
    async fn an_imported_library_does_not_kill_the_kernel_verified_actions() {
        let dir = std::env::temp_dir().join(format!(
            "sokonanoda-lsp-import-actions-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        std::fs::write(dir.join("SetLib.sokonanoda"), IMPORTED_LIB).expect("write the library");
        let canvas_path = dir.join("Canvas.sokonanoda");
        std::fs::write(&canvas_path, IMPORTED_CANVAS).expect("write the canvas");
        let root = Url::from_directory_path(&dir).expect("dir url");
        let canvas = Url::from_file_path(&canvas_path).expect("file url");

        let (mut service, mut socket) = test_service();
        handshake_with_root(&mut service, &root).await;
        did_open_at(&mut service, &canvas, IMPORTED_CANVAS).await;
        // ① 闭包是好的（库记法随 import 生效）—— 缺口不在编译，在建议。
        //    开放练习本身会发 `sorry` 警告（合法状态）⇒ 判据是"**没有别的**诊断"。
        let diags = wait_diagnostics_for(&mut socket, &canvas, "canvas diagnostics").await;
        let unexpected: Vec<&str> = diags
            .diagnostics
            .iter()
            .map(crate::testutil::code_of)
            .filter(|code| *code != "sorry")
            .collect();
        assert!(
            unexpected.is_empty(),
            "闭包编译是好的（缺口在建议层），不该有 `sorry` 之外的诊断：{unexpected:?}"
        );

        // ② 第二个洞（`a = a`）必须仍拿到**内核验证过**的 `rfl`。
        let hole = IMPORTED_CANVAS.rfind("sorry").expect("second hole");
        let actions = code_actions_at_uri(&mut service, &canvas, IMPORTED_CANVAS, hole)
            .await
            .unwrap_or_default();
        assert!(
            actions.iter().any(|a| a
                .title
                .starts_with("rfl\u{ff08}\u{5185}\u{6838}\u{9a8c}\u{8bc1}")),
            "带 `import` 的库不得吃掉内核验证过的候选：{actions:?}"
        );
        shutdown(&mut service).await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── F12b：**闭包记法 + 箭头头**那一半（2026-10-13）──────────────────────
    //
    // 现场（真 VS Code + 真课程画布，`editor-probe --case hole-lightbulb` 的
    // 练习 1 那一臂）：`mem_of_subset` 的目标是 `a ∈ A → a ∈ B` —— **箭头头 +
    // 闭包记法**，灯泡给 **0 条 sokonanoda 候选**（菜单里只剩 VS Code 内置的
    // `Quick Fix`/`Fix`/`Explain`）。
    //
    // 根因是**两层各自的"记法盲"**，而单测夹具全都没有 `import`（这正是
    // "单测绿、真机空"的缝）：
    //   * front `suggest::intro_count` 用 `parse_expr_text`（**只带内建表**）
    //     ⇒ `∈` 不在表里 ⇒ 解析失败 ⇒ `SuggestionKind::Intro` 一条都不产生；
    //   * LSP 的 `notation_ladder_fallbacks` 只认 `Eq`/`And`/`Iff` 三个**体内**
    //     记法头，而 [`peel_binders`] 把箭头剥掉 ⇒ 连"目标头是箭头"都看不出来
    //     （`Set.mem` 不在硬编码的三族里）⇒ 0 条。
    //
    // 本判据钉的是 **LSP 侧那一半**：闭包记法表在手时，箭头头必须给得出
    // `intro x`（结构性，与 `constructor` 同档 —— 内核在学生写下的每一步之后
    // 照旧终审）。**front 侧那一半已由 F12b2 收口**（`suggest_with_notations`：
    // 表喂进 `intro_count`/`intro_text` 走的那条解析通道）⇒ 这一档现在**两条路
    // 都给**（同一句标题、同一条编辑 ⇒ A5 去重后用户只看见一条 ✓），
    // 判据对**两个位置**都成立（tactic 位见本用例，项位见下面 F12b2 那条）。

    /// 入口：`import` + **箭头头**（`a ∈ A → a ∈ B`，课程画布练习 1 的形状）。
    const IMPORTED_ARROW_CANVAS: &str = "\
import SetLib\n\n\
theorem mem_of_subset (\u{3b1} : Type) (A B : Set \u{3b1}) (a : \u{3b1}) :\n\
    a \u{2208} A \u{2192} a \u{2208} B := by\n\
  sorry\n";

    #[tokio::test]
    async fn imported_notation_arrow_goal_gets_the_intro_step() {
        let dir = std::env::temp_dir().join(format!(
            "sokonanoda-lsp-import-arrow-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        std::fs::write(dir.join("SetLib.sokonanoda"), IMPORTED_LIB).expect("write the library");
        let canvas_path = dir.join("Arrow.sokonanoda");
        std::fs::write(&canvas_path, IMPORTED_ARROW_CANVAS).expect("write the canvas");
        let root = Url::from_directory_path(&dir).expect("dir url");
        let canvas = Url::from_file_path(&canvas_path).expect("file url");

        let (mut service, mut socket) = test_service();
        handshake_with_root(&mut service, &root).await;
        did_open_at(&mut service, &canvas, IMPORTED_ARROW_CANVAS).await;
        // 前提：闭包编译是好的（缺口在**建议**层，不是判定层）。
        let diags = wait_diagnostics_for(&mut socket, &canvas, "arrow canvas diagnostics").await;
        let unexpected: Vec<&str> = diags
            .diagnostics
            .iter()
            .map(crate::testutil::code_of)
            .filter(|code| *code != "sorry")
            .collect();
        assert!(
            unexpected.is_empty(),
            "闭包编译是好的（缺口在建议层），不该有 `sorry` 之外的诊断：{unexpected:?}"
        );

        let hole = IMPORTED_ARROW_CANVAS.find("sorry").expect("the hole");
        let actions = code_actions_at_uri(&mut service, &canvas, IMPORTED_ARROW_CANVAS, hole)
            .await
            .unwrap_or_default();
        let intro = actions
            .iter()
            .find(|a| a.title.contains("\u{5f15}\u{5165} "))
            .unwrap_or_else(|| {
                panic!(
                    "`a \u{2208} A \u{2192} a \u{2208} B`（箭头头 + 闭包记法）必须有 sokonanoda 的候选：{actions:?}"
                )
            });
        assert_eq!(
            first_edit(intro).1,
            "intro x",
            "tactic 位的箭头头 ⇒ 插入的就是 `intro x`（`fun (x : T) => sorry` 是项 ✗）"
        );
        shutdown(&mut service).await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── F12b2：**同一形状、同一套判据**，只换洞的**位置模式**（2026-10-13）──
    //
    // 现场：`front::suggest` 的 `intro_count` / `intro_text`
    // （`crates/front/src/suggest.rs`）读目标文本走 `parse_expr_text`（**只带内建
    // 表**）⇒ 闭包记法（`∈` / `⊆`）的目标**读不回来** ⇒ `SuggestionKind::Intro`
    // 一条都不产生。tactic 位那条由 `notation_ladder_fallbacks` 兜住了（F12b），
    // 而它**只给 tactic 位**（项位要 `fun (x : T) => sorry`，那个文本得真剥一层
    // 才知道 `T`）⇒ **F12b2 补的就是这一格**：把闭包记法表喂进
    // `intro_count`/`intro_text` 走的那条解析通道（`parse_expr_text_with` +
    // `ProofState::start_with`），**不为项位另立一套语义** ✓。
    //
    // 判据与 `imported_notation_arrow_goal_gets_the_intro_step` **成对**：同一个
    // 目标文本（`a ∈ A → a ∈ B`），只把 `:= by sorry`（tactic 位）换成
    // `:= sorry`（项位）⇒ 候选的**标题逐字相同**（同一步骤同一句话），
    // **插入文本按位置模式定型**（A1 ✓）。改前这里是 **0 条候选**。

    /// 入口：`import` + **项位** + 闭包记法箭头目标（与 F12b 的夹具同形，去掉 `by`）。
    const IMPORTED_TERM_ARROW_CANVAS: &str = "\
import SetLib\n\n\
def mem_of_subset_term (\u{3b1} : Type) (A B : Set \u{3b1}) (a : \u{3b1}) :\n\
    a \u{2208} A \u{2192} a \u{2208} B :=\n\
  sorry\n";

    #[tokio::test]
    async fn imported_term_position_notation_arrow_goal_gets_the_intro_step() {
        let dir = std::env::temp_dir().join(format!(
            "sokonanoda-lsp-import-term-arrow-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        std::fs::write(dir.join("SetLib.sokonanoda"), IMPORTED_LIB).expect("write the library");
        let canvas_path = dir.join("TermArrow.sokonanoda");
        std::fs::write(&canvas_path, IMPORTED_TERM_ARROW_CANVAS).expect("write the canvas");
        let root = Url::from_directory_path(&dir).expect("dir url");
        let canvas = Url::from_file_path(&canvas_path).expect("file url");

        let (mut service, mut socket) = test_service();
        handshake_with_root(&mut service, &root).await;
        did_open_at(&mut service, &canvas, IMPORTED_TERM_ARROW_CANVAS).await;
        // 前提：闭包编译是好的（缺口在**建议**层，不是判定层）。
        let diags = wait_diagnostics_for(&mut socket, &canvas, "term arrow diagnostics").await;
        let unexpected: Vec<&str> = diags
            .diagnostics
            .iter()
            .map(crate::testutil::code_of)
            .filter(|code| *code != "sorry")
            .collect();
        assert!(
            unexpected.is_empty(),
            "闭包编译是好的（缺口在建议层），不该有 `sorry` 之外的诊断：{unexpected:?}"
        );

        let hole = IMPORTED_TERM_ARROW_CANVAS.find("sorry").expect("the hole");
        let actions = code_actions_at_uri(&mut service, &canvas, IMPORTED_TERM_ARROW_CANVAS, hole)
            .await
            .unwrap_or_default();
        let intro = actions
            .iter()
            .find(|a| a.title.contains("\u{5f15}\u{5165} "))
            .unwrap_or_else(|| {
                panic!(
                    "**项位**的 `a \u{2208} A \u{2192} a \u{2208} B`（箭头头 + 闭包记法）\
必须有 sokonanoda 的候选（改前 0 条）：{actions:?}"
                )
            });
        // 同一步骤、同一句话 —— 标题与 tactic 位那条**逐字相同**（同一套判据 ✓）。
        assert_eq!(
            intro.title,
            "\u{5f15}\u{5165} 1 \u{4e2a} binder\u{ff08}\u{628a}\u{8bc1}\u{660e}\u{5199}\u{6210} lambda \u{7684}\u{7b2c}\u{4e00}\u{6b65}\u{ff09}",
            "项位与 tactic 位的 intro 候选必须是**同一句话**（判据同一套）"
        );
        // **插入文本按位置模式定型**（A1）：项位 ⇒ `fun (x : T) => sorry`，
        // 而 `T` 只有把目标文本用**闭包记法表**读回来才剥得出来（`a ∈ A`）。
        assert_eq!(
            first_edit(intro).1,
            "fun (x : a \u{2208} A) => sorry",
            "项位的箭头头 ⇒ 插入的是 `fun (x : T) => sorry`（`intro x` 是 tactic ✗）"
        );
        shutdown(&mut service).await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 入口：`import` + **记法头 `↔`**（`({a} = {b}) ↔ a = b`，课程画布**练习 6** 的形状）。
    ///
    /// **为什么单列一条**（2026-10-13）：练习 6 是 `F12` 真机判据里**唯一**读不到候选的那一臂，
    /// 而真机那一路的红**已钉死在探针的按键时序**（`⌘.` 在 macOS 不触发；命令面板的 `<CR>`
    /// 会把菜单第一项应用掉 ⇒ 缓冲区变、请求被 cancel）⇒ **真机读不到 ≠ 服务端没给** ✗。
    /// 这条把**真相层**钉住：只要闭包编译是好的，`↔` 头的洞上服务端**必须**给出 sokonanoda 候选 ✓
    /// —— 真机那一臂以后无论怎么修探针，都不许把"服务端不给"当成解释 ✓。
    /// ⚠ **夹具刻意不含记法**（2026-10-13）：先用**纯 `↔` 头**把"目标头是 Iff 时给不给候选"
    /// 这一件事**单独**钉住 ✓ —— 混进 `{a}`/`∈` 会让失败同时来自记法解析
    /// （实测第一版夹具就撞了 `elab-notation-argument-unsolved` ⇒ 红在**前提**、判不了目标 ✗）。
    const IMPORTED_IFF_CANVAS: &str = "\
import SetLib\n\n\
theorem iff_probe (P Q : Prop) : P \u{2194} Q := by\n\
  sorry\n";

    #[tokio::test]
    async fn imported_notation_iff_goal_gets_a_sokonanoda_action() {
        let dir =
            std::env::temp_dir().join(format!("sokonanoda-lsp-import-iff-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        std::fs::write(dir.join("SetLib.sokonanoda"), IMPORTED_LIB).expect("write the library");
        let canvas_path = dir.join("Iff.sokonanoda");
        std::fs::write(&canvas_path, IMPORTED_IFF_CANVAS).expect("write the canvas");
        let root = Url::from_directory_path(&dir).expect("dir url");
        let canvas = Url::from_file_path(&canvas_path).expect("file url");

        let (mut service, mut socket) = test_service();
        handshake_with_root(&mut service, &root).await;
        did_open_at(&mut service, &canvas, IMPORTED_IFF_CANVAS).await;
        let diags = wait_diagnostics_for(&mut socket, &canvas, "iff canvas diagnostics").await;
        let unexpected: Vec<&str> = diags
            .diagnostics
            .iter()
            .map(crate::testutil::code_of)
            .filter(|code| *code != "sorry")
            .collect();
        assert!(
            unexpected.is_empty(),
            "闭包编译是好的（缺口在建议层），不该有 `sorry` 之外的诊断：{unexpected:?}"
        );

        let hole = IMPORTED_IFF_CANVAS.find("sorry").expect("the hole");
        let actions = code_actions_at_uri(&mut service, &canvas, IMPORTED_IFF_CANVAS, hole)
            .await
            .unwrap_or_default();
        // 记法头 `↔` 的候选 = 结构性拆分（`constructor`）；它**不**需要 kernel 终审，
        // 所以哪怕 `judge_*` 那条路出问题，这一条也必须活着 ✓。
        assert!(
            actions.iter().any(|a| a.title.starts_with("constructor")),
            "`↔`（记法头 + 闭包）的洞上必须有 sokonanoda 的 `constructor` 候选（把 `Iff` 拆成两个子目标）：{actions:?}"
        );
        shutdown(&mut service).await;
        let _ = std::fs::remove_dir_all(&dir);
    }
}
