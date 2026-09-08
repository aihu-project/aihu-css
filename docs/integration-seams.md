# Integration seams

This repository owns the CSS engine and its local Rust, TypeScript, and
generated-gallery checks. The framework repository owns the full UI registry.

`crates/aihu-css-core/tests/apply_regression.rs` intentionally exercises the
checked-in `tests/fixtures/registry/button/button.aihu` fixture so standalone
clones do not depend on a sibling checkout. The full `packages/ui/registry/*`
matrix remains a retained integration seam in `aihu-project/aihu`: changes to
the utility table or `@apply` expansion should run that matrix there as part of
the framework compatibility checks.
