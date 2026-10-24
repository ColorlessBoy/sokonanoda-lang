# UnsafeDef

Regression fixture from `leanprover/lean-kernel-arena`
`tests/tutorial/140_falseFromUnsafe` (Apache-2.0), revision
`d4f15adc61e131de91b11a7bfa827045df86d193` (2026-10-23).

The kernel does not support `unsafe`/`partial` definitions. It must **decline**
them with a controlled error (exit code 2 in `sokonanoda-kernel`), not panic:
a panic is indistinguishable from a broken kernel invariant and cannot be
classified as a rejection by the arena harness.
