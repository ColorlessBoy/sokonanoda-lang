# NestedRecRuleNFields

Regression fixture from `leanprover/lean-kernel-arena`
`tests/bugs/nested-rec-rule-nfields` (Apache-2.0), revision
`d4f15adc61e131de91b11a7bfa827045df86d193` (2026-10-23).

The export declares `T.mk` with two fields but lies that its rule in `T.rec`
has one field. A sound kernel must reject it: the field count is used by
`fire_recursor` to split the constructor arguments, so trusting the lie turns
`badProp : Prop` into `Bool` and lets `bad : False` through.
