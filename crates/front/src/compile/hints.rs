//! Hint ladders authored in the canvas as `-- soko:hint <text>` comment
//! directives (docs/design-hints-suggestions.md).
//!
//! Rules:
//! - a directive must own its line (only whitespace before `--`);
//! - each directive attaches to the first declaration whose span starts
//!   after it (directives after the last declaration are ignored — teachers
//!   may write ahead of the code);
//! - the ladder order is the source order; the tier convention (idea →
//!   target shape → key lemma → skeleton) is an authoring convention, the
//!   mechanism only preserves order.

use super::DocumentReport;

/// One parsed `-- soko:hint …` directive: its byte offset (the `--` token
/// start, so it is always before the declaration it attaches to) and text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceHint {
    pub offset: usize,
    pub text: String,
}

const DIRECTIVE: &str = "-- soko:hint";

/// Scan source text for hint directives, in source order.
pub fn source_hints(src: &str) -> Vec<SourceHint> {
    let mut hints = Vec::new();
    let mut offset = 0usize;
    for line in src.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(DIRECTIVE) {
            let indent = line.len() - trimmed.len();
            let text = rest.trim();
            if !text.is_empty() {
                hints.push(SourceHint {
                    offset: offset + indent,
                    text: text.to_string(),
                });
            }
        }
        offset += line.len() + 1;
    }
    hints
}

/// Attach the not-yet-consumed hints that precede `decl_start` to this
/// declaration. `consumed` is the shared cursor across the declaration walk
/// (hints before it were already attached to earlier declarations).
pub fn attach_hints(hints: &[SourceHint], decl_start: usize, consumed: &mut usize) -> Vec<String> {
    let mut attached = Vec::new();
    while *consumed < hints.len() && hints[*consumed].offset < decl_start {
        attached.push(hints[*consumed].text.clone());
        *consumed += 1;
    }
    attached
}

/// Post-process a report produced for `src`: fill every declaration's hint
/// ladder (`DeclState.hints`). DeclStates are sorted by span, so a single
/// cursor walk attaches each directive to the first declaration after it.
/// Every session update runs this over the merged report, so hint edits
/// recompute ladders exactly like any other edit.
pub fn attach_hints_to_report(src: &str, report: &mut DocumentReport) {
    let hints = source_hints(src);
    if hints.is_empty() {
        return;
    }
    let mut consumed = 0usize;
    for decl in &mut report.decls {
        decl.hints = attach_hints(&hints, decl.span.start.offset, &mut consumed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directives_must_own_their_line() {
        let src = "def a : Nat := 1 -- soko:hint trailing does not count\n\
                   -- soko:hint   own line counts\n\
                   def b : Nat := 2\n";
        let hints = source_hints(src);
        assert_eq!(hints.len(), 1, "trailing comments are not directives");
        assert_eq!(hints[0].text, "own line counts");
    }

    #[test]
    fn ladder_order_is_source_order() {
        let src = "-- soko:hint 第一层\n-- soko:hint 第二层\nexample : Prop := sorry\n";
        let hints = source_hints(src);
        assert_eq!(
            hints.iter().map(|h| h.text.as_str()).collect::<Vec<_>>(),
            vec!["第一层", "第二层"]
        );
    }

    #[test]
    fn empty_or_whitespace_text_is_not_a_hint() {
        let src = "-- soko:hint\n-- soko:hint    \nexample : Prop := sorry\n";
        assert!(source_hints(src).is_empty());
    }

    #[test]
    fn attach_to_the_next_declaration_only() {
        let src = "-- soko:hint for-a\ndef a : Nat := 1\n\
                   -- soko:hint for-b\n-- soko:hint also-b\nexample : Prop := sorry\n\
                   -- soko:hint ignored-after-last\n";
        let mut report = super::super::check_document_with(
            &crate::parser::parse(src).expect("parses"),
            &Default::default(),
        );
        assert!(report.decls.len() >= 2);
        attach_hints_to_report(src, &mut report);
        let mut decls = report.decls.iter().filter(|d| !d.hints.is_empty());
        let a = decls.next().expect("a carries its hint");
        assert_eq!(a.name.as_deref(), Some("a"));
        assert_eq!(a.hints, vec!["for-a".to_string()]);
        let b = decls.next().expect("b carries two hints");
        assert_eq!(b.hints, vec!["for-b".to_string(), "also-b".to_string()]);
        assert!(
            decls.next().is_none(),
            "hints after the last decl are dropped"
        );
    }

    #[test]
    fn directives_skip_non_declaration_commands() {
        // A `#check` between the hint and the exercise must not consume it.
        let src = "-- soko:hint for-example\n#check Nat\nexample : Prop := sorry\n";
        let mut report = super::super::check_document_with(
            &crate::parser::parse(src).expect("parses"),
            &Default::default(),
        );
        attach_hints_to_report(src, &mut report);
        let example = report
            .decls
            .iter()
            .find(|d| d.status == super::super::DeclStatus::Open)
            .expect("the open exercise");
        assert_eq!(example.hints, vec!["for-example".to_string()]);
    }

    #[test]
    fn no_directives_leaves_hints_empty() {
        let src = "def a : Nat := 1\n";
        let mut report = super::super::check_document_with(
            &crate::parser::parse(src).expect("parses"),
            &Default::default(),
        );
        attach_hints_to_report(src, &mut report);
        assert!(report.decls.iter().all(|d| d.hints.is_empty()));
    }
}
