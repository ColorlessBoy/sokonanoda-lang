//! **依赖边与脏集**（G-29 / 用户 2026-10-01 拍板的 §5.1 模型，材料 ① ②）。
//!
//! 用户原话（验收标准）：「对齐 Lean4 的**依赖图脏传播** …… 改一个声明 ⇒ 环境回滚到
//! 该声明之前 ⇒ 只重编译「被改声明 + **依赖它的下游**（脏集）」，其余后缀从环境快照
//! 恢复」。本模块只做**纯计算**：从**已经有**的解析结果建边、算脏集，
//! **不碰编译、不改判定** —— 消费它的是 S6（脏集重编译）。
//!
//! # 为什么不需要重新扫文本（材料都是现成的）
//!
//! * 每个**名字使用点**在 elaboration 期就已经解析过，并记在
//!   `HoverType.resolution = Some(ResolvedTarget::Declaration { name, .. })`
//!   （`crates/front/src/compile/elab.rs` 的 `HoverNode.resolution`）✓；
//! * 每条 hover 属于哪条命令也是现成的：`DocumentReport.hover_cmds` 与
//!   `hovers` **平行**（注释原文：「the command index each hover belongs to」）✓；
//! * 每条声明属于哪条命令：`DeclState.cmd` ✓（注释原文：「Index of the command
//!   that produced this state … without span guessing」）。
//!
//! ⇒ 建边 = 一次线性扫描，**零额外解析、零文本比对**（硬规则 4）。
//!
//! # 粒度
//!
//! 边建在**命令**上（`file.commands` 的下标）：验收读数就是「**重查命令数**」
//! （§5.1 表格），而 `DeclState.cmd` 把它与声明一一对上 ✓。
//!
//! # 一个刻意的保守选择
//!
//! **解析不到目标的 hover 不建边**（prelude 名字、未解析 ident、记法符号的
//! `Notation` 变体、局部 binder 的 `Binder` 变体都不算**文件内**的依赖）——
//! 少建一条边 = 脏集**偏小** ⇒ 复用一段**本该重查**的后缀 ⇒ **错编** ✗。
//! 所以本模块**只**回答"我确知它引用了谁"；S6 必须把**未知**（`None` 解析、
//! 跨模块名字）当成**脏**（保守方向：宁可多查，不可少查）✓。
//! `unknown_references` 就是给 S6 用的那个读数。

use std::collections::{BTreeSet, HashMap};

use crate::compile::{DocumentReport, ResolvedTarget};

/// 文件内的**声明级**依赖图（边在命令下标上）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DepGraph {
    /// 命令数（下标空间的上界）。
    commands: usize,
    /// `uses[i]` = 命令 `i` 引用到的**文件内声明**所在的命令（去重、升序、去自引用）。
    uses: Vec<Vec<usize>>,
    /// `used_by[j]` = 引用了命令 `j` 那条声明的命令（去重、升序、去自引用）。
    used_by: Vec<Vec<usize>>,
    /// 名字 → 声明它的命令（文件内）。
    decl_of: HashMap<String, usize>,
    /// 引用了**解析不到文件内声明**的名字的命令（prelude / 跨模块 / 未解析）。
    /// S6 必须把它们当**脏**（保守方向见模块文档）。
    unknown: BTreeSet<usize>,
}

impl DepGraph {
    /// 从一份**已经编译好的**报告建图（纯函数，零 IO、零解析）。
    pub fn from_report(report: &DocumentReport) -> Self {
        // 名字 → 声明命令（同名重复声明取**先**出现的那个：后一个本来就编不过）。
        let mut decl_of: HashMap<String, usize> = HashMap::new();
        for decl in &report.decls {
            if let Some(name) = decl.name.as_deref() {
                decl_of.entry(name.to_string()).or_insert(decl.cmd);
            }
        }
        // 命令数 = 所有出现过的命令下标 + 1（报告里没有"文件有多少条命令"这个数）。
        let commands = report
            .decls
            .iter()
            .map(|d| d.cmd)
            .chain(report.hover_cmds.iter().copied())
            .max()
            .map_or(0, |m| m + 1);

        let mut uses: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); commands];
        let mut used_by: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); commands];
        let mut unknown: BTreeSet<usize> = BTreeSet::new();

        for (hover, cmd) in report.hovers.iter().zip(report.hover_cmds.iter()) {
            if *cmd >= commands {
                continue; // 报告与命令数不自洽时**不猜**（宁可少建边 ⇒ 由 unknown 兜）
            }
            match &hover.resolution {
                Some(ResolvedTarget::Declaration { name, .. }) => match decl_of.get(name) {
                    // 文件内声明：建边（自引用不算依赖 —— 改它本来就要重查它自己）。
                    Some(&target) if target != *cmd => {
                        uses[*cmd].insert(target);
                        used_by[target].insert(*cmd);
                    }
                    Some(_) => {}
                    // **跨模块/prelude**：名字不在本文件的声明表里 ⇒ 保守记脏。
                    None => {
                        unknown.insert(*cmd);
                    }
                },
                // 解析不到（未解析 ident）⇒ 保守记脏。
                None => {
                    unknown.insert(*cmd);
                }
                // 局部 binder / 记法符号：**不是**文件内声明依赖 ⇒ 不建边、不算未知。
                Some(ResolvedTarget::Binder(_)) | Some(ResolvedTarget::Notation { .. }) => {}
            }
        }

        Self {
            commands,
            uses: uses.into_iter().map(|s| s.into_iter().collect()).collect(),
            used_by: used_by
                .into_iter()
                .map(|s| s.into_iter().collect())
                .collect(),
            decl_of,
            unknown,
        }
    }

    /// 命令数（下标空间）。
    pub fn commands(&self) -> usize {
        self.commands
    }

    /// 命令 `cmd` 引用到的文件内声明（按命令下标升序）。
    pub fn uses(&self, cmd: usize) -> &[usize] {
        self.uses.get(cmd).map_or(&[][..], |v| v.as_slice())
    }

    /// 直接引用命令 `cmd` 那条声明的命令（按升序）。
    pub fn used_by(&self, cmd: usize) -> &[usize] {
        self.used_by.get(cmd).map_or(&[][..], |v| v.as_slice())
    }

    /// 名字 → 声明它的命令。
    pub fn declaration_command(&self, name: &str) -> Option<usize> {
        self.decl_of.get(name).copied()
    }

    /// 引用了**解析不到文件内声明**的名字的命令（prelude / 跨模块 / 未解析）——
    /// S6 的**保守**输入：这些命令不许被当成"干净"。
    pub fn unknown_references(&self) -> &BTreeSet<usize> {
        &self.unknown
    }

    /// **脏集**：改命令 `changed` ⇒ 要重查的命令集合 = `{changed}` ∪
    /// **传递**依赖它的那些命令（升序）。
    ///
    /// 传递闭包是硬要求（`t00 ← d00 ← d01` ⇒ 改 `t00` 必须连 `d01` 一起重查）；
    /// 只算**直接**依赖会漏 ⇒ **错编** ✗。
    pub fn dirty_commands(&self, changed: usize) -> Vec<usize> {
        let mut seen: BTreeSet<usize> = BTreeSet::new();
        if changed >= self.commands {
            return Vec::new();
        }
        seen.insert(changed);
        let mut stack = vec![changed];
        while let Some(cmd) = stack.pop() {
            for &dependent in self.used_by(cmd) {
                if seen.insert(dependent) {
                    stack.push(dependent);
                }
            }
        }
        seen.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::{DeclKind, DeclState, DeclStatus, HoverType};
    use crate::{Pos, Span};

    fn span(offset: usize) -> Span {
        Span::new(
            Pos {
                offset,
                line: 0,
                column: 0,
            },
            Pos {
                offset: offset + 1,
                line: 0,
                column: 1,
            },
        )
    }

    fn decl(name: &str, cmd: usize) -> DeclState {
        DeclState {
            kind: DeclKind::Theorem,
            name: Some(name.to_string()),
            span: span(cmd * 100),
            status: DeclStatus::Checked,
            error: None,
            goal: None,
            binders: Vec::new(),
            cmd,
            universe: Vec::new(),
            holes: Vec::new(),
            sub_goals: Vec::new(),
            ty_text: None,
            val_text: None,
            refine_template: None,
            hints: Vec::new(),
            by_steps: Vec::new(),
            by_root: None,
        }
    }

    /// 一条 hover：命令 `cmd` 引用名字 `name`（`None` ⇒ 解析不到）。
    fn use_of(cmd: usize, name: Option<&str>) -> HoverType {
        HoverType {
            span: span(cmd * 100 + 1),
            text: "x".to_string(),
            scope_names: Vec::new(),
            resolution: name.map(|n| ResolvedTarget::Declaration {
                name: n.to_string(),
                span: span(0),
            }),
            binder: false,
            lexical: false,
        }
    }

    fn report(decls: Vec<DeclState>, uses: Vec<(usize, Option<&str>)>) -> DocumentReport {
        let mut hovers = Vec::new();
        let mut hover_cmds = Vec::new();
        for (cmd, name) in uses {
            hovers.push(use_of(cmd, name));
            hover_cmds.push(cmd);
        }
        DocumentReport {
            decls,
            hovers,
            hover_cmds,
            errors: Vec::new(),
            checks: Vec::new(),
            prints: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// 传递闭包：`t00 ← d00 ← d01`，改 `t00` 必须把 `d01` 也带上。
    #[test]
    fn dirty_set_is_transitive() {
        // 命令：0=t00 · 1=t01 · 2=d00(用 t00) · 3=d01(用 d00) · 4=d02(用 t00)
        let report = report(
            vec![
                decl("t00", 0),
                decl("t01", 1),
                decl("d00", 2),
                decl("d01", 3),
                decl("d02", 4),
            ],
            vec![(2, Some("t00")), (3, Some("d00")), (4, Some("t00"))],
        );
        let graph = DepGraph::from_report(&report);
        assert_eq!(graph.uses(2), &[0], "d00 引用 t00");
        assert_eq!(graph.used_by(0), &[2, 4], "t00 被 d00/d02 直接引用");
        assert_eq!(
            graph.dirty_commands(0),
            vec![0, 2, 3, 4],
            "改 t00 ⇒ 脏集必须含**间接**依赖它的 d01（传递闭包）"
        );
        assert_eq!(
            graph.dirty_commands(1),
            vec![1],
            "无人依赖的一条 ⇒ 脏集只有自己"
        );
        assert_eq!(graph.dirty_commands(3), vec![3], "链尾无人依赖 ⇒ 只有自己");
    }

    /// 自引用不算依赖；**库层/prelude 名字不进 `unknown`**（依赖指纹兜底），
    /// **解析不到**的才进（S6 的保守输入）。
    #[test]
    fn self_reference_and_unknown_are_handled() {
        let report = report(
            vec![decl("a", 0), decl("b", 1)],
            vec![
                (1, Some("a")),
                (1, Some("a")),   // 重复引用同一条 ⇒ 去重
                (1, Some("Nat")), // 库层/prelude ⇒ **不算** unknown（依赖指纹兜底）
                (1, None),        // 解析不到 ⇒ unknown
                (0, Some("a")),   // 自引用 ⇒ 不建边
            ],
        );
        let graph = DepGraph::from_report(&report);
        assert_eq!(graph.uses(1), &[0], "重复引用去重、跨模块名不进 uses");
        assert_eq!(graph.used_by(0), &[1]);
        assert_eq!(graph.dirty_commands(0), vec![0, 1]);
        assert_eq!(
            graph
                .unknown_references()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![1],
            "**解析不到**的命令必须进 unknown（S6 不许把它当干净）；库层名字不算 unknown"
        );
    }
}
