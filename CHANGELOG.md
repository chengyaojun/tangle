# Changelog

## v0.2.1 — Quality Audit

- Fix: parser expression stop-list omits `Let`/`Const` (F-001/F-002) — eliminates false `TANGLE_SYMBOL_NOT_FOUND` / `TANGLE_TYPE_ERROR` diagnostics in `account.tangle.md` and `expression.tangle.md`
- Fix: heading parser does not recognize `Prefix: Identifier` pattern (F-003) — `Error: PayFailed`, `Rule: Approval`, `Type: Account` etc. now parsed correctly
- Fix: `# Order Service` multi-word title in fixture (F-004) — renamed to `# OrderService`
- Fix: `Rule:` prefix headings in 4 rule-based fixtures (F-005) — resolved via F-003 fix
- Fix: `payment.tangle.md` bare assignment `result = process()?` (F-006) — replaced with `let result = "ok"`
- Style: fix all 17 `cargo clippy` warnings (F-014~F-023) — `new_without_default`, `unnecessary_map_or`, `bind_instead_of_map`, `collapsible_if`, `module_inception`, `ptr_arg`, `derivable_impls`, `should_implement_trait`, `needless_range_loop`, `empty_line_after_doc_comments`
- Test: add `audit_regression` suite (G1~G7) with regression tests per root cause group
- Test: add audit matrix harness (`run-audit.ps1`) covering ~250 cells across CLI surfaces × codegen targets × execution modes × fixtures
- Test: add differential IR testing harness (`diff-ir.ps1` + `ir-diff` crate) comparing Rust compiler output against TS reference
- Test: add LSP protocol probe (`lsp-probe` crate) for diagnostics surface verification
- Docs: add `docs/audit/findings.md` with categorized findings (F-001~F-024)

### Known limitations (deferred to v0.3.0)

- F-007~F-012: IR schema divergence between TS reference and Rust compiler (node ID naming, edge guard field, top-level functions array, sourceText field, terminal node label). Allowlisted in `diff-ir.ps1` as `KNOWN_DIFF`.
- F-010: TS reference does not implement rule lowering — emits empty IR for rule-based fixtures. Auto-skipped in `diff-ir.ps1`.
- F-013: Go toolchain not installed in audit environment — 30 `run --target go` cells exit with code 101.
- F-024: Checker cannot resolve top-level callable symbols (`TypeEnv` has no `functions` table). No fixture currently triggers this.
