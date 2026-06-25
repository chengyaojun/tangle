# Tangle Business MVP — Order Service

A complete order processing service demonstrating Tangle's zero-directive, literate-programming design.

## What It Demonstrates

| Feature | How It's Used |
|---------|---------------|
| **Immutable structs** | `Order` struct with typed fields (`id`, `amount`, `status`) |
| **Implicit method binding** | `create` and `confirm` are automatically bound to `Order` via heading nesting |
| **Zero-keyword updates** | `order { status: "paid" }` — no `with` keyword needed |
| **Error handling** | `Error:` prefix headings define error variants; `?` propagates; `Ok()` / `Err()` for return |
| **Implicit entry point** | `#### main` is auto-detected as the program entry — no `@entry` directive |
| **Zero directives** | No `@`-prefixed annotations anywhere in the source |

## Files

```
examples/mvp/
├── order-service.tangle.md   # Order processing service
└── README.md                  # This file
```

## Language Features in Action

### Struct Definition & Constructor

```markdown
### Order
* `id`: order ID (String)
* `amount`: order amount (Int)
* `status`: order status (String)

#### 创建订单 (create)
* `id`: order ID (String)
* `amount`: amount (Int)

```@tangle
return Order { id: id, amount: amount, status: "created" }
```
```

- `### Order` declares an immutable value struct with three typed fields
- `#### 创建订单 (create)` is implicitly bound as a method — the parenthesized `(create)` sets the code identifier to camelCase `create`

### Error Variants via `Error:` Prefix

```markdown
##### Error: PayFailed
##### Error: Timeout
```

- No `@error` directive — error variants are declared as `Error:` prefix sub-headings
- The compiler's `ErrorRegistry` collects them from heading titles

### Error Propagation & Immutable Updates

```@tangle
if (order.amount <= 0) Err("PayFailed", "Invalid amount")?
if (order.amount > 10000) Err("Timeout", "Amount too large")?
return Ok(order { status: "paid" })
```

- `Err()` returns a typed error variant; `?` propagates it to the caller
- `order { status: "paid" }` copies the struct with the `status` field updated (immutable)

### Implicit Program Entry

```markdown
#### main
* `config`: (Config)

```@tangle
let order = Order.create("ord-1", 100)?
return order { status: "confirmed" }
```
```

- `#### main` is the implicit entry point — the compiler recognizes `main` at depth 4
- No `@entry` directive needed

## Usage

```bash
# Build the compiler
npm run build

# Run the order service
node dist/src/cli/main.js run examples/mvp/order-service.tangle.md
```

### Expected Output

The CLI compiles the `.md` file through the full pipeline (DSL → typed AST → IR → JS) and outputs the generated JavaScript with runtime prelude. Diagnostics (type errors, casing violations) are reported with source spans pointing back to `.md` line numbers.

## See Also

- [Language Design Spec](../../docs/superpowers/specs/2026-06-24-tangle-language-design.md)
- [Main README](../../README.md)
- [Standard Library](../../stdlib/)
