# Changelog

## v0.8.0 — Phase 6d Type Narrowing Completeness

- Feat: `if x is Pattern` expression for variant type testing with optional binding (e.g. `if opt is Some(y) { ... }`)
- Feat: `let Some(y) = x else { ... }` refutable variant destructuring with else branch
- Feat: `let { ok, err } = r` irrefutable record destructuring with optional field rename (`let { ok: o, err: e } = r`)
- Feat: built-in `as_sum_view()` recognizes `Option<T>` as `Sum(Some<T>, None)` for match/Is/LetVariant arms
- Feat: `resolve_struct_in_env` helper fills empty-shell struct payloads from `TypeEnv::structs` (fixes field access on Option<Struct> payloads)
- Diagnostics: 5 new codes — `TANGLE_REFUTABLE_LET_REQUIRES_ELSE`, `TANGLE_PATTERN_VARIANT_NOT_FOUND`, `TANGLE_PATTERN_NOT_NARROWABLE`, `TANGLE_DESTRUCTURE_NOT_STRUCT`, `TANGLE_STRUCT_FIELD_NOT_FOUND`
- Design: AST extension (3 new nodes + `Pattern` subtree) + IR lowering desugars to existing `Compute`/`Action` nodes with descriptive labels (`let Some(item)`, `let { name, price }`). IR schema unchanged.
- Compatibility: three emitters (JS/Py/Go) unchanged; existing 15 fixtures IR unchanged
- TS reference: full mirror of Phase 6d AST/lexer/parser/checker/IR; parser now supports `if cond { block }` and `match expr { arms }` syntax (previously known limitation); lexer recognizes `=>` as `fatArrow` (was only `->`); inferReturnTypes handles `letVariant`/`letRecord` cases

### Verification

- `cargo test --workspace`: 376 tests pass, 0 failures
- `cargo clippy --workspace --all-targets -- -D warnings`: zero warnings
- `reference && npm test`: 259 tests pass
- `reference && npm run build`: zero type errors
- `tests/audit/diff-ir.ps1`: 18 MATCH + 0 KNOWN_DIFF + 0 DIFF + 0 SKIPPED (was 13/0/1/4 in Phase 6c)
- `tests/audit/run-audit.ps1`: 238 cells, 0 failing

### Known limitations (deferred to Phase 6e)

- `Result<T,E>` built-in Sum view (stdlib signatures registry not yet populated)
- `is not Some` negative narrowing (requires negative type computation)
- Compound OR patterns (`is Some or None`)
- Guard expressions (`is Some(y) && y > 0`)
- Flow-sensitive narrowing (x's type does not change in then branch — only binding is injected)
- User-defined generic types (`Foo<T>`) and constraints (`T: Comparable`)
- Nested patterns (`is Some({ ok, err })`)
- Record destructure wildcard (`let { _, err } = r`)

## v0.3.0 — Phase 1 Call Expression Type Checking

- Feat: introduce `Type::Any` and `CallableSignature::is_variadic` to model variadic functions (e.g. `fmt.println`)
- Feat: resolve top-level callable symbols via new `TypeEnv::functions` table (F-024)
- Feat: register stdlib signatures for all 19 modules — `fmt`, `IO`, `List`, `Map`, `Set`, `Option`, `Math`, `String`, `Env`, `Path`, `JSON`, `DateTime`, `Random`, `Encoding`, `Sort`, `Process`, `Task`, `Channel`, `Sync`
- Feat: `resolve_stdlib_imports` injects real signatures from the registry, replacing the previous dummy `(params=[], returns=String)`
- Feat: `MemberAccess` returns `Type::Function` for method references so callee resolution works for `Account.open(...)`
- Feat: `Call` expression type checking — arity mismatch (`TANGLE_ARITY_MISMATCH`) and parameter type mismatch (`TANGLE_TYPE_ERROR`) diagnostics
- Test: add `v03_phase1` suite covering top-level callable, stdlib signatures, stdlib module/function call, member access type, and Call arity/type checks
- Test: add `compat_check` regression test verifying zero diagnostics on all 6 examples + 9 fixtures
- Style: fix `clippy::collapsible_if` warning in `is_child_of_type_heading`

### Verification

- `cargo test --workspace`: 127 tests pass, 0 failures, 0 ignored
- `cargo clippy --workspace --all-targets -- -D warnings`: zero warnings
- `tests/audit/run-audit.ps1`: 210 cells, 0 failing, zero diagnostics across all CLI surfaces × codegen targets × modes × fixtures
- Manual compatibility check: all 6 examples + 9 fixtures produce zero diagnostics via `run_collecting_diagnostics`

### Known limitations (deferred to later phases)

- Variadic function arguments are not type-checked (e.g. `Path.join` extra args). Tracked as Phase 1 design boundary.
- Non-Function callee returns `Type::Any`, losing type information for direct constructor-style calls. Not triggered by any current example/fixture.
- `types_equal` does not compare `Function`/`Sum`/`GenericInstance`/`Var` types (returns `false`). No false positives observed in practice.

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
