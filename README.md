# Tangle

> Documents are programs. Files are modules.

Tangle is a **Markdown-native programming language** where every `.md` file is both readable documentation and a compilable program module. Headings define scopes, lists define parameters, code blocks carry executable logic, and semantic conventions replace all explicit directives — achieving a **zero-directive, literate programming** experience.

---

## Language Design

### Heading Hierarchy (6-Level Scope System)

| Level | Role | Semantic | Casing |
|-------|------|----------|--------|
| `#` | `program` | Package / root context | PascalCase |
| `##` | `section` | Namespace / domain zone | PascalCase |
| `###` | `type` | Structs, interfaces, error families | PascalCase |
| `####` | `callable` | Functions, methods | camelCase |
| `#####` | `semantic-section` | Preconditions, steps, branches | camelCase |
| `######` | `semantic-atom` | Atomic actions, assertions, tests | camelCase |

### Implicit Method Binding

When a `####` heading is physically nested under a `###` struct heading, the compiler automatically binds it as a method — no arrow syntax needed.

### Immutable Structs & Update Syntax

Structs are immutable by default. Use brace-expressions for both construction and copy-on-update:

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

### Visibility

- **Default public** — depth 3-4 symbols without `_` prefix are exported
- **`_` prefix private** — symbols starting with `_` are module-private
- **No `@export` needed**

### Entry Point

A depth-4 callable with the identifier `main` is implicitly the program entry point:

```markdown
#### main
```

### Rule Graph IR

Tangle compiles code and rules into a unified **Rule Graph** intermediate representation (nodes, edges, error edges), enabling mixed execution of `@tangle` code blocks and decision logic expressed as tables, lists, Mermaid diagrams, or checkboxes — all marked with the `Rule:` heading prefix.

---

## Compilation Pipeline

```
Markdown Source (.md)
  → compileModule       (parse + heading tree + symbols)
  → checkModule         (parse @tangle code + type check)
  → compileToIR         (lower to Rule Graph IR)
  → emitJS              (generate JavaScript)
  → Node.js execution   (tangle run)
```

---

## Quick Start

### Install

```bash
npm install
npm run build
```

### Run a Tangle program

```bash
node dist/src/cli/main.js run ./examples/mvp/order-service.tangle.md
```

### Run tests

```bash
npm test       # 132 tests, 31 test files
```

### Type check

```bash
npm run typecheck
```

---

## Example: Immutable Struct with Methods

````markdown
### User
* `id`: user ID (Int)
* `email`: email (String)
* `is_active`: active flag (Bool)

#### 激活 (activate)
* `reason`: activation reason (String)

```@tangle
return this { is_active: true }
```
````

---

## Example: Error Handling

````markdown
#### 确认支付 (confirm)
* `order`: Order

##### Error: PayFailed
##### Error: Timeout

```@tangle
result = gateway.charge(order.amount)?
return Ok(result)
```
````

---

## Project Structure

```
tangle/
├── src/
│   ├── model.ts              # DSL type definitions
│   ├── ast.ts                # Code AST types
│   ├── front-end/            # Markdown → TangleModule (A1)
│   ├── markdown/             # Markdown parser wrapper
│   ├── parser/               # Lexer + recursive descent parser (A2)
│   ├── checker/              # Type checker + error handling (A2+A3)
│   ├── ir/                   # Rule Graph IR (A4)
│   ├── codegen/              # JS code generation (A5)
│   ├── cli/                  # CLI entry point
│   └── pipeline.ts           # Full compilation pipeline
├── tests/                    # 31 test files, 132 tests
├── stdlib/                   # Standard library (.tangle.md modules)
├── examples/mvp/             # Business MVP example
└── docs/
    ├── superpowers/specs/    # Language design specification
    └── superpowers/plans/    # Implementation plans (A1-A6)
```

---

## Roadmap

### ✅ Track A — TypeScript Bootstrap (0.x) — Complete

All six phases implemented. JS/TS codegen only. Semantic validation + business MVP.

| Phase | Status | Deliverable |
|-------|--------|-------------|
| A1 — Compiler Frontend | ✅ | Markdown → `TangleModule` DSL |
| A2 — Parser & Type Checker | ✅ | `@tangle` code parser, static type system |
| A3 — Error Semantics | ✅ | `?` propagation, `match` exhaustiveness, `panic` |
| A4 — Rule Graph IR | ✅ | Unified IR, `Rule:` lowering (flow/table/tree/toggle) |
| A5 — JS Codegen & CLI | ✅ | IR → JS, `tangle run`, `tangle test` |
| A6 — Stdlib & MVP | ✅ | 7 stdlib modules, order service example |

### ⬜ Track B — Rust Authority (1.0)

Official `tangle-cli` in Rust once semantic baseline is frozen:

- Rust compiler skeleton matching TS semantics
- Differential testing against TS reference
- Python / Go codegen
- Cross-host stdlib consistency suite
- Incremental compilation, IR caching, LSP, doc generation

### 🔮 2.0 — Self-Hosting

Long-term: write the Tangle compiler in Tangle itself. Rust edition becomes the bootstrap tool.

---

## Standard Library

| Module | Types / Functions |
|--------|------------------|
| `List` | `length`, `map`, `filter` |
| `Option` | `Some`, `None`, `unwrap` |
| `Map` | `get` |
| `String` | `length`, `concat` |
| `JSON` | `parse`, `stringify` |
| `IO` | `readFile`, `writeFile` |
| `Math` | `abs`, `min` |

---

## Language Semantics

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

## License

MIT
