# RecRuleProofIrrel

Regression fixture from `leanprover/lean-kernel-arena`
`tests/bad/bugs/rec-rule-proof-irrel` (Apache-2.0), corpus
`master@9867c5929` (2026-11-22).

The exported `J.rec` iota rule for `J.j1` returns `id (motive j1) (mk true)`
instead of the minor premise. The two sides are **definitionally equal under
proof irrelevance** but not as terms, so a rule comparison that takes the
proof-irrelevance shortcut accepts a recursion rule the block never admitted —
and the file then proves `False`.

The fix is in `nested_rule_hygiene.rs`: the admitted-vs-reconstructed rule
comparison runs with `TypeChecker::suppress_proof_irrel` set.
