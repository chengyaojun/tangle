# Tangle

> Documents are programs. Files are modules.

Tangle is a **Markdown-native programming language** where every `.md` file is both readable documentation and a compilable program module. Headings define scopes, lists define parameters, code blocks carry executable logic, and semantic conventions replace all explicit directives — achieving a **zero-directive, literate programming** experience.

---

## Language Design

### Six-Level Heading Hierarchy

| Level | Role | Semantic | Casing |
|-------|------|----------|--------|
| `#` | `program` | Package / root context | PascalCase |
| `##` | `section` | Namespace / domain zone | PascalCase |
| `###` | `type` | Structs, interfaces, error families | PascalCase |
| `####` | `callable` | Functions, methods | camelCase |
| `#####` | `semantic-section` | Preconditions, steps, branches, rules | camelCase |
| `######` | `semantic-atom` | Atomic actions, assertions, tests | camelCase |

### Implicit Method Binding

When a `####` heading is physically nested under a `###` struct heading, the compiler automatically binds it as a method — no arrow syntax needed.

### Immutable Structs

Structs are immutable by default. Brace-expressions for both construction and copy-on-update:

```@tangle
user = User { id: 1, email: "alice@tangle.io" }
updated = user { is_active: true }
```

### Error Handling

Errors are return values, never thrown. Use sum types with the `Error:` prefix convention:

```@tangle
receipt = confirm(order)?
match result {
    Receipt(r) => process(r)
    PayFailed(e) => log(e.code)
}
```

### Visibility & Conventions

| Feature | Convention | Example |
|---------|-----------|---------|
| Visibility | `_` prefix = private | `_internalInit` |
| Entry point | depth-4 `main` identifier | `#### main` |
| Deprecation | `~~` strikethrough | `### ~~OldConfig~~` |
| Errors | `Error:` prefix heading | `##### Error: PayFailed` |
| Rules | `Rule:` prefix heading | `##### Rule: Approval` |
| Tests | `Test:` prefix heading | `##### Test: NormalFlow` |
| Imports | Markdown links | `[Alias](./module.md)` |

---

## Compilation Pipeline

```
Markdown Source (.md)
  → compileModule       (frontend: parse + heading tree + symbols + rule detection)
  → checkModule         (checker: parse @tangle code + type check)
  → compileToIR         (IR: lower to Rule Graph IR + rule lowering)
  → emitJS / emitPython / emitGo   (multi-host codegen)
  → host execution      (Node.js / Python / Go)
```

---

## Project Structure

```
tangle/
├── compiler/tangle-cli/     # Track B: Rust authority compiler (57 source files)
│   ├── src/
│   │   ├── model.rs, ast.rs, diagnostic.rs
│   │   ├── frontend/        # Markdown → TangleModule
│   │   ├── markdown/        # pulldown-cmark wrapper
│   │   ├── parser/          # Lexer + Pratt parser
│   │   ├── checker/         # Type checker + error handling (10 files)
│   │   ├── ir/              # Rule Graph IR + rule lowering (9 files)
│   │   ├── codegen/         # JS / Python / Go emitters
│   │   ├── stdlib/          # Multi-host stdlib bindings
│   │   ├── incremental/     # Incremental compilation
│   │   ├── lsp/             # LSP language server
│   │   ├── docgen/          # HTML doc generation
│   │   └── cli/             # tangle run / test / lsp / doc
│   └── Cargo.toml
├── library/std/             # Standard library (22 modules)
│   └── src/
│       ├── list, map, set, option
│       ├── string, regex, encoding
│       ├── fmt, io, env, path, process
│       ├── http, json
│       ├── math, random, sort
│       ├── task, channel, sync
│       ├── datetime
│       └── crypto
├── reference/               # Track A: TypeScript reference (frozen)
│   ├── src/                 # 36 source files
│   ├── tests/               # 31 test files, 132 tests
│   ├── stdlib/               # Stdlib .md modules
│   └── examples/mvp/        # Order service MVP
├── tests/                   # Shared test fixtures (9 .md files)
├── schemas/ir.json          # IR JSON Schema (diff-test contract)
├── docs/                    # Design docs + plans
└── Cargo.toml               # Rust workspace root
```

---

## Quick Start

### Rust Compiler (Track B)

```bash
# Build
cargo build

# Run
cargo run -- run tests/basic/hello.tangle.md
cargo run -- run tests/basic/hello.tangle.md --target py
cargo run -- run tests/basic/hello.tangle.md --target go
cargo run -- run tests/basic/hello.tangle.md --emit-ir
cargo run -- run tests/basic/hello.tangle.md --incremental
cargo run -- run tests/basic/hello.tangle.md --interp

# Test
cargo test -p tangle-cli        # 81 tests

# LSP & Docs
cargo run -- lsp
cargo run -- doc tests/basic/hello.tangle.md
```

### TypeScript Reference (Track A)

```bash
cd reference
npm install
npm run build

node dist/src/cli/main.js run ../tests/basic/hello.tangle.md
node dist/src/cli/main.js run ../tests/basic/hello.tangle.md --emit-ir
npm test                         # 132 tests
```

---

## Importing the Standard Library

Import stdlib modules via Markdown links. Three import granularities:

```markdown
## 依赖

[fmt](fmt)                     ← module: fmt.println("hello")
[println](fmt)                 ← single function: println("hello")
[print, println](fmt)          ← multiple functions: print("hi") + println("hi")
```

| Syntax | Usage |
|--------|-------|
| `[fmt](fmt)` | `fmt.println("hello")` |
| `[println](fmt)` | `println("hello")` |
| `[print, println](fmt)` | `println("hi"); print("hi")` |

Bare name = stdlib module. Path prefix (`./`) = local file.

---

## CLI Reference

### `tangle run` — Compile and execute

```bash
tangle run <file.md>                           # Compile to JS and run
tangle run <file.md> --target py               # Compile to Python and run
tangle run <file.md> --target go               # Compile to Go and run
tangle run <file.md> --incremental             # Skip unchanged files
tangle run <file.md> --interp                  # Run via native IR interpreter (no host)
```

| Flag | Description |
|------|-------------|
| `--target <js\|py\|go>` | Target language (default `js`) |
| `--incremental` | Enable incremental compilation, cached in `.cache/` |
| `--interp` | Execute via native Rust IR interpreter (no external host dependency) |

### `tangle build` — Compile only (output source)

```bash
tangle build <file.md>                         # Compile to JS, print source
tangle build <file.md> --target py             # Compile to Python source
tangle build <file.md> --emit-ir               # Output IR JSON
tangle build <file.md> --incremental           # Skip unchanged files
```

| Flag | Description |
|------|-------------|
| `--target <js\|py\|go>` | Target language (default `js`) |
| `--emit-ir` | Emit Rule Graph IR JSON, skip code generation |
| `--incremental` | Enable incremental compilation, cached in `.cache/` |

### `tangle test` — Run tests

```bash
tangle test                                    # Run all tests
tangle test --filter <pattern>                 # Filter by name
```

### `tangle doc` — Generate documentation

```bash
tangle doc <file.md>                           # Output HTML to stdout
tangle doc <file.md> --output docs/index.html  # Write to file
```

### `tangle lsp` — Start language server

```bash
tangle lsp                                     # stdio LSP server
```

---

## Roadmap

### ✅ Track A — TypeScript Bootstrap (0.x) — Complete

JS/TS codegen only. Semantic validation + business MVP.

| Phase | Status | Deliverable |
|-------|--------|-------------|
| A1 — Compiler Frontend | ✅ | Markdown → `TangleModule` DSL |
| A2 — Parser & Type Checker | ✅ | `@tangle` code parser, static type system |
| A3 — Error Semantics | ✅ | `?` propagation, `match` exhaustiveness, `panic` |
| A4 — Rule Graph IR | ✅ | Unified IR, `Rule:` lowering |
| A5 — JS Codegen & CLI | ✅ | IR → JS, `tangle run`, `tangle test` |
| A6 — Stdlib & MVP | ✅ | 7 stdlib modules, order service example |

### ✅ Track B — Rust Authority (v0.2.0) — Complete

| Phase | Status | Content |
|-------|--------|---------|
| B1 — Rust Compiler Skeleton | ✅ | Frontend → Parser → Checker → IR → JS Codegen + CLI |
| B2 — Differential Testing | ✅ | IR JSON Schema, shared fixtures, TS `--emit-ir` |
| B3 — Multi-host Codegen | ✅ | Python + Go emitters, `--target` flag |
| B4 — Standard Library | ✅ | 22 modules across 9 categories |
| B5 — Performance & Toolchain | ✅ | Incremental compilation + IR cache + LSP + Doc HTML |

**Post-B5 enhancements (v0.2.x):**
- On-demand stdlib imports via Markdown links `[fmt](fmt)`
- Single/multi-function imports: `[println](fmt)`, `[print, println](fmt)`
- `tangle build` — compile only (like `go build`)
- `tangle run` — compile + execute in one step
- Source-text codegen (AST → real code, not comments)
- Per-module stdlib prelude (only emit what's imported)

### ✅ v0.2.1 — Quality Audit — Complete

| Gate | Status |
|------|--------|
| Audit matrix (210 cells) zero false diagnostics | ✅ |
| `cargo test --workspace` (108 tests) | ✅ |
| `cargo clippy --workspace -- -D warnings` | ✅ |
| Differential IR test against TS reference | ✅ |

See [docs/audit/findings.md](docs/audit/findings.md) for audit details.

### ✅ v0.3.0 Phase 1 — Call Expression Type Checking — Complete

| Gate | Status |
|------|--------|
| Audit matrix (210 cells) zero false diagnostics | ✅ |
| `cargo test --workspace` (127 tests) | ✅ |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ |
| Compat check (6 examples + 9 fixtures zero diagnostics) | ✅ |

**Highlights:**
- `Type::Any` + `is_variadic` for variadic stdlib functions
- F-024: top-level callable symbol resolution
- 19-module stdlib signature registry
- `Call` arity + parameter type checking (`TANGLE_ARITY_MISMATCH`, `TANGLE_TYPE_ERROR`)

**Post-B5 v0.3.0 development path — four phases of tightening & independent execution:**

| Phase | Focus | Highlights |
|-------|-------|------------|
| 1 — Stdlib Signatures | Checker stdlib function signature refinement | Precise `CallableSignature { params, returns }` per function; eliminate `TANGLE_TYPE_ERROR` false positives |
| 2 — Rule Lowering | Rule form lowering completeness | Nested-list AND/OR with associativity; table row-priority sort + overlap detection; Mermaid graph/subgraph destructuring |
| 3 — Typed Codegen | AST-based typed codegen translation | Refactor `src/codegen/` to consume standard Tangle AST; tree-shaking via static analysis; per-module host preludes (e.g. `fmt.println` → `console.log`) |
| 4 — IR Interpreter | Native IR tree-walking interpreter **(core leap)** | Pure Rust execution inside `tangle-cli` driven by Rule Graph IR; general graph-traversal evaluator; `tangle run --interp` experimental flag; differential testing of `?` error-propagation |

### 🔮 2.0 — Self-Hosting (v1.0.0)

Write the Tangle compiler in Tangle itself. Rust edition becomes the bootstrap tool.

---

## Standard Library (22 modules)

| Category | Modules |
|----------|---------|
| Collections | List, Map, Set, Option |
| Text | String, Regex, Encoding |
| I/O & System | IO, fmt, Env, Path, Process |
| Network | HTTP, JSON |
| Math & Data | Math, Random, Sort |
| Concurrency | Task, Channel, Sync |
| Time | DateTime |
| Crypto | Crypto |

---

## License

MIT
