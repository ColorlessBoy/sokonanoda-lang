# RecRuleProofIrrelNested

Regression fixture from `leanprover/lean-kernel-arena`
`tests/bad/bugs/rec-rule-proof-irrel-nested` (Apache-2.0), corpus
`master@9867c5929` (2026-11-22).

The nested-inductive variant of [`RecRuleProofIrrel`](../RecRuleProofIrrel):
a lying iota rule differs from the reconstructed one only in a proof-irrelevant
argument. Rejecting it needs the same proof-irrelevance-free rule comparison.
