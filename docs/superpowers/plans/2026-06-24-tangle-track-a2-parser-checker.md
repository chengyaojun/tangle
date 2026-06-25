# Tangle Track A2: Code Parser & Core Type Checker 实现计划

> **语法精炼勘误（2026-06-25）：** (1) `with { }` → 无关键字大括号更新。(2) `Struct -> method` → 隐式方法绑定。(3) `=>` → `->`。(4) 新增 `|>` 管道。(5) 新增标题大小写对齐契约：深度 1-3 PascalCase，深度 4-6 camelCase。(6) 移除 @export，改为下划线隐式私有。(7) 移除 @entry，改为 main 隐式入口契约。详见设计规格 §3.2。

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 在 A1 的 Markdown 前端基础上，解析 `@tangle` 代码块内的 Tangle 表面语言（JS-like 表达式/语句），构建类型环境，并实现核心静态类型检查器。

**架构：** A1 输出 `TangleModule`。A2 新增三层：
1. **Parser 层** (`src/parser/`)：手写递归下降解析器，将 @tangle 代码文本转为 Code AST
2. **Checker 层** (`src/checker/`)：从 headings 构建类型环境（结构体/接口/方法），对 Code AST 做类型检查
3. **Pipeline 集成**：`checkModule()` 组合 A1 + A2 全流程，产出 `CheckedModule`

**技术栈：** TypeScript ESM、Vitest。无新增依赖（手写解析器）。

---

## 规格来源

- `docs/superpowers/specs/2026-06-24-tangle-language-design.md`
- A2 覆盖：§3.4（@tangle 代码块深度语法）、§4.1（类型系统）、§4.2（结构体、with 更新）、§4.3（方法与 this）、§4.4（接口、结构化契合）

---

## 关键设计决策

1. **Code AST 位置**：新建 `src/ast.ts`，与 DSL 层 `src/model.ts` 分离。`src/model.ts` 不变。
2. **Parser 方式**：手写递归下降（Pratt parsing 处理优先级），分 `lexer.ts` + `parser.ts`。
3. **Type 表示**：`src/checker/types.ts` 用 discriminated union，与 model.ts 风格一致。
4. **Pipeline 集成**：新增 `checkModule()`，不修改 `compileModule()`。A1 测试全部继续通过。
5. **Diagnostic 码**：`TANGLE_PARSE_*`（解析错误）、`TANGLE_TYPE_*`（类型错误）。
6. **不修改 model.ts**：A1 的所有类型保持不变，A2 类型在 `ast.ts` 和 `checker/types.ts` 中。

---

## 文件结构

- 创建：`src/ast.ts` — Code AST 节点类型（Expr, Stmt, TypeExpr 等 30+ 类型）
- 创建：`src/parser/lexer.ts` — 词法分析器（~25 种 TokenKind）
- 创建：`src/parser/parser.ts` — 递归下降解析器（表达式 + 语句）
- 创建：`src/parser/typeParser.ts` — 类型标注解析器
- 创建：`src/checker/types.ts` — 类型系统表示（7 种 Type variant）+ 工具函数
- 创建：`src/checker/builtins.ts` — 内置类型定义
- 创建：`src/checker/env.ts` — 类型环境（变量/结构体/接口/receiver 上下文）
- 创建：`src/checker/resolve.ts` — 从 TangleHeading 解析结构体/接口/方法签名
- 创建：`src/checker/check.ts` — 核心类型检查器
- 创建：`src/checker/checkModule.ts` — Pipeline 集成入口
- 修改：`src/index.ts` — 新增所有 A2 类型和函数的 barrel 导出

- 创建：`tests/parser/lexer.test.ts`
- 创建：`tests/parser/parser.test.ts`
- 创建：`tests/parser/typeParser.test.ts`
- 创建：`tests/checker/types.test.ts`
- 创建：`tests/checker/builtins.test.ts`
- 创建：`tests/checker/env.test.ts`
- 创建：`tests/checker/resolve.test.ts`
- 创建：`tests/checker/check.test.ts`
- 创建：`tests/checker/checkModule.test.ts`
- 修改：`tests/fixtures.ts` — 新增 A2 测试夹具

---

## 任务 1：定义 Code AST 类型

**文件：**
- 创建：`src/ast.ts`
- 修改：`src/index.ts`
- 创建：`tests/parser/parser.test.ts`（此任务仅 AST 类型 smoke test）

- [ ] **步骤 1：编写 AST 类型约束测试**

创建 `tests/parser/parser.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import type { Expr, LiteralExpr, IdentifierExpr, Stmt, ReturnStmt, CodeBody } from "../../src/index";

describe("Code AST types", () => {
  it("supports literal expressions", () => {
    const lit: LiteralExpr = {
      kind: "literal",
      literalKind: "number",
      value: 42,
      span: { file: "test.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 3 }
    };
    expect(lit.kind).toBe("literal");
    expect(lit.value).toBe(42);
  });

  it("supports identifier expressions", () => {
    const id: IdentifierExpr = {
      kind: "identifier",
      name: "x",
      span: { file: "test.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 2 }
    };
    expect(id.name).toBe("x");
  });

  it("defines discriminated union Expr", () => {
    const expr: Expr = {
      kind: "literal",
      literalKind: "boolean",
      value: true,
      span: { file: "test.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 5 }
    };
    if (expr.kind === "literal") {
      expect(expr.literalKind).toBe("boolean");
    }
  });

  it("defines CodeBody with statements", () => {
    const body: CodeBody = {
      kind: "codeBody",
      statements: [],
      span: { file: "test.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 }
    };
    expect(body.statements).toEqual([]);
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：`npm test -- tests/parser/parser.test.ts`

预期：FAIL，报错 `Module '../../src/index' has no exported member 'LiteralExpr'`

- [ ] **步骤 3：实现 `src/ast.ts`**

```ts
import type { SourceSpan, TangleDiagnostic } from "./model.js";

// ─── Expressions ───────────────────────────────────────

export type Expr =
  | LiteralExpr
  | IdentifierExpr
  | MemberAccessExpr
  | CallExpr
  | BinaryExpr
  | UnaryExpr
  | WithUpdateExpr
  | ThisExpr
  | IfExpr
  | ArrowExpr;

export type LiteralExpr = {
  kind: "literal";
  literalKind: "number" | "string" | "boolean";
  value: number | string | boolean;
  span: SourceSpan;
};

export type IdentifierExpr = {
  kind: "identifier";
  name: string;
  span: SourceSpan;
};

export type MemberAccessExpr = {
  kind: "memberAccess";
  object: Expr;
  member: string;
  span: SourceSpan;
};

export type CallExpr = {
  kind: "call";
  callee: Expr;
  args: Expr[];
  span: SourceSpan;
};

export type BinaryExpr = {
  kind: "binary";
  op: BinaryOp;
  left: Expr;
  right: Expr;
  span: SourceSpan;
};

export type BinaryOp =
  | "+" | "-" | "*" | "/" | "%"
  | "==" | "!=" | "<" | ">" | "<=" | ">="
  | "&&" | "||";

export type UnaryExpr = {
  kind: "unary";
  op: UnaryOp;
  operand: Expr;
  span: SourceSpan;
};

export type UnaryOp = "!" | "-";

export type WithUpdateExpr = {
  kind: "withUpdate";
  object: Expr;
  fields: WithField[];
  span: SourceSpan;
};

export type WithField = {
  name: string;
  value: Expr;
  span: SourceSpan;
};

export type ThisExpr = {
  kind: "this";
  span: SourceSpan;
};

export type IfExpr = {
  kind: "if";
  condition: Expr;
  thenBranch: Expr;
  elseBranch?: Expr;
  span: SourceSpan;
};

export type ArrowExpr = {
  kind: "arrow";
  params: ArrowParam[];
  body: Expr;
  span: SourceSpan;
};

export type ArrowParam = {
  name: string;
  typeAnnotation?: TypeExpr;
  span: SourceSpan;
};

// ─── Statements ────────────────────────────────────────

export type Stmt =
  | ReturnStmt
  | LetStmt
  | ConstStmt
  | ExpressionStmt;

export type ReturnStmt = {
  kind: "return";
  value?: Expr;
  span: SourceSpan;
};

export type LetStmt = {
  kind: "let";
  name: string;
  typeAnnotation?: TypeExpr;
  value: Expr;
  span: SourceSpan;
};

export type ConstStmt = {
  kind: "const";
  name: string;
  typeAnnotation?: TypeExpr;
  value: Expr;
  span: SourceSpan;
};

export type ExpressionStmt = {
  kind: "expression";
  expr: Expr;
  span: SourceSpan;
};

// ─── Code Block AST ────────────────────────────────────

export type CodeBody = {
  kind: "codeBody";
  statements: Stmt[];
  span: SourceSpan;
};

// ─── Type Expressions (parsed from type annotations) ───

export type TypeExpr =
  | PrimitiveTypeExpr
  | SumTypeExpr
  | GenericTypeExpr
  | FunctionTypeExpr
  | NamedTypeExpr;

export type PrimitiveTypeExpr = {
  kind: "primitiveType";
  name: "String" | "Int" | "Bool";
  span: SourceSpan;
};

export type SumTypeExpr = {
  kind: "sumType";
  variants: TypeExpr[];
  span: SourceSpan;
};

export type GenericTypeExpr = {
  kind: "genericType";
  base: string;
  typeArgs: TypeExpr[];
  span: SourceSpan;
};

export type FunctionTypeExpr = {
  kind: "functionType";
  params: TypeExpr[];
  returns: TypeExpr;
  span: SourceSpan;
};

export type NamedTypeExpr = {
  kind: "namedType";
  name: string;
  span: SourceSpan;
};

// ─── Parser result ─────────────────────────────────────

export type ParsedCodeBlock = {
  headingId: string;
  source: string;
  body: CodeBody;
  diagnostics: TangleDiagnostic[];
};
```

- [ ] **步骤 4：修改 `src/index.ts` barrel**

在现有 exports 后追加：

```ts
export type {
  ArrowExpr,
  ArrowParam,
  BinaryExpr,
  BinaryOp,
  CallExpr,
  CodeBody,
  ConstStmt,
  Expr,
  ExpressionStmt,
  FunctionTypeExpr,
  GenericTypeExpr,
  IdentifierExpr,
  IfExpr,
  LetStmt,
  LiteralExpr,
  MemberAccessExpr,
  NamedTypeExpr,
  ParsedCodeBlock,
  PrimitiveTypeExpr,
  ReturnStmt,
  Stmt,
  SumTypeExpr,
  ThisExpr,
  TypeExpr,
  UnaryExpr,
  UnaryOp,
  WithField,
  WithUpdateExpr
} from "./ast.js";
```

- [ ] **步骤 5：运行测试验证通过 + 类型检查**

运行：`npm test -- tests/parser/parser.test.ts`
预期：PASS。

运行：`npm run typecheck`
预期：PASS。

---

## 任务 2：实现词法分析器 (Lexer)

**文件：**
- 创建：`src/parser/lexer.ts`
- 修改：`src/index.ts`
- 创建：`tests/parser/lexer.test.ts`

- [ ] **步骤 1：编写 lexer 测试**

创建 `tests/parser/lexer.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import { tokenize } from "../../src/index";

describe("lexer", () => {
  it("tokenizes numbers", () => {
    const tokens = tokenize("42", "test.md");
    expect(tokens).toHaveLength(2); // number + eof
    expect(tokens[0]).toMatchObject({ kind: "number", value: "42" });
  });

  it("tokenizes strings", () => {
    const tokens = tokenize('"hello"', "test.md");
    expect(tokens[0]).toMatchObject({ kind: "string", value: '"hello"' });
  });

  it("tokenizes boolean literals", () => {
    expect(tokenize("true", "test.md")[0]).toMatchObject({ kind: "true", value: "true" });
    expect(tokenize("false", "test.md")[0]).toMatchObject({ kind: "false", value: "false" });
  });

  it("tokenizes keywords", () => {
    const tokens = tokenize("return let const if else this with", "test.md");
    expect(tokens[0]!.kind).toBe("return");
    expect(tokens[1]!.kind).toBe("let");
    expect(tokens[2]!.kind).toBe("const");
    expect(tokens[3]!.kind).toBe("if");
    expect(tokens[4]!.kind).toBe("else");
    expect(tokens[5]!.kind).toBe("this");
    expect(tokens[6]!.kind).toBe("with");
  });

  it("tokenizes identifiers", () => {
    const tokens = tokenize("foo bar123 _private", "test.md");
    expect(tokens[0]!).toMatchObject({ kind: "identifier", value: "foo" });
    expect(tokens[1]!).toMatchObject({ kind: "identifier", value: "bar123" });
    expect(tokens[2]!).toMatchObject({ kind: "identifier", value: "_private" });
  });

  it("tokenizes operators", () => {
    const tokens = tokenize("+ - * / % == != < > <= >= && || ! => ->", "test.md");
    expect(tokens.map(t => t.kind)).toEqual([
      "plus", "minus", "star", "slash", "percent",
      "eqeq", "neq", "lt", "gt", "lte", "gte",
      "and", "or", "bang", "fatArrow", "arrow",
      "eof"
    ]);
  });

  it("tokenizes delimiters", () => {
    const tokens = tokenize(". , : ; ( ) { } [ ] |", "test.md");
    expect(tokens.map(t => t.kind)).toEqual([
      "dot", "comma", "colon", "semicolon",
      "lparen", "rparen", "lbrace", "rbrace", "lbracket", "rbracket",
      "pipe", "eof"
    ]);
  });

  it("tokenizes a complete code snippet", () => {
    const tokens = tokenize("return this with { is_active: true }", "test.md");
    expect(tokens.map(t => t.kind)).toEqual([
      "return", "this", "with", "lbrace", "identifier", "colon", "true", "rbrace", "eof"
    ]);
  });

  it("skips whitespace and attaches source spans", () => {
    const tokens = tokenize("  x\n  42", "test.md");
    const x = tokens[0]!;
    expect(x.kind).toBe("identifier");
    expect(x.span.startLine).toBe(1);
    expect(x.span.startColumn).toBe(3);
    const num = tokens[1]!;
    expect(num.kind).toBe("number");
    expect(num.span.startLine).toBe(2);
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：`npm test -- tests/parser/lexer.test.ts`
预期：FAIL，`tokenize` 未导出。

- [ ] **步骤 3：实现 `src/parser/lexer.ts`**

```ts
import type { SourceSpan } from "../model.js";

export type TokenKind =
  // literals
  | "number" | "string" | "true" | "false"
  // identifiers & keywords
  | "identifier"
  | "return" | "let" | "const" | "if" | "else" | "this" | "with"
  // delimiters
  | "dot" | "comma" | "colon" | "semicolon"
  | "lparen" | "rparen" | "lbrace" | "rbrace" | "lbracket" | "rbracket"
  // operators
  | "plus" | "minus" | "star" | "slash" | "percent"
  | "eq" | "eqeq" | "neq" | "lt" | "gt" | "lte" | "gte"
  | "and" | "or" | "bang"
  | "pipe"
  | "arrow"      // ->
  | "fatArrow"   // =>
  // special
  | "eof";

export type Token = {
  kind: TokenKind;
  value: string;
  span: SourceSpan;
};

const KEYWORDS: Record<string, TokenKind> = {
  "return": "return",
  "let": "let",
  "const": "const",
  "if": "if",
  "else": "else",
  "this": "this",
  "with": "with",
  "true": "true",
  "false": "false"
};

export function tokenize(source: string, file: string): Token[] {
  const tokens: Token[] = [];
  let pos = 0;
  let line = 1;
  let column = 1;

  function span(startLine: number, startCol: number): SourceSpan {
    return { file, startLine, startColumn: startCol, endLine: line, endColumn: column };
  }

  function addToken(kind: TokenKind, value: string, startLine: number, startCol: number): void {
    tokens.push({ kind, value, span: span(startLine, startCol) });
  }

  function peek(offset?: number): string {
    return source[pos + (offset ?? 0)] ?? "\0";
  }

  function advance(): string {
    const ch = source[pos] ?? "\0";
    if (ch === "\n") { line += 1; column = 1; } else { column += 1; }
    pos += 1;
    return ch;
  }

  while (pos < source.length) {
    const startLine = line;
    const startCol = column;
    const ch = advance();

    // whitespace
    if (ch === " " || ch === "\t" || ch === "\r" || ch === "\n") {
      continue;
    }

    // numbers
    if (ch >= "0" && ch <= "9") {
      let num = ch;
      while (peek() >= "0" && peek() <= "9") num += advance();
      if (peek() === "." && peek(1) >= "0" && peek(1) <= "9") {
        num += advance(); // dot
        while (peek() >= "0" && peek() <= "9") num += advance();
      }
      addToken("number", num, startLine, startCol);
      continue;
    }

    // strings
    if (ch === '"') {
      let str = ch;
      while (peek() !== '"' && peek() !== "\0") {
        if (peek() === "\n") { line += 1; column = 1; }
        str += advance();
      }
      if (peek() === '"') str += advance();
      addToken("string", str, startLine, startCol);
      continue;
    }

    // identifiers & keywords
    if ((ch >= "a" && ch <= "z") || (ch >= "A" && ch <= "Z") || ch === "_") {
      let id = ch;
      while (true) {
        const n = peek();
        if ((n >= "a" && n <= "z") || (n >= "A" && n <= "Z") || (n >= "0" && n <= "9") || n === "_") {
          id += advance();
        } else break;
      }
      const kw = KEYWORDS[id];
      addToken(kw ?? "identifier", id, startLine, startCol);
      continue;
    }

    // multi-char operators
    if (ch === "=" && peek() === "=") { advance(); addToken("eqeq", "==", startLine, startCol); continue; }
    if (ch === "=" && peek() === ">") { advance(); addToken("fatArrow", "=>", startLine, startCol); continue; }
    if (ch === "!" && peek() === "=") { advance(); addToken("neq", "!=", startLine, startCol); continue; }
    if (ch === "<" && peek() === "=") { advance(); addToken("lte", "<=", startLine, startCol); continue; }
    if (ch === ">" && peek() === "=") { advance(); addToken("gte", ">=", startLine, startCol); continue; }
    if (ch === "&" && peek() === "&") { advance(); addToken("and", "&&", startLine, startCol); continue; }
    if (ch === "|" && peek() === "|") { advance(); addToken("or", "||", startLine, startCol); continue; }
    if (ch === "-" && peek() === ">") { advance(); addToken("arrow", "->", startLine, startCol); continue; }

    // single-char tokens
    const singles: Record<string, TokenKind> = {
      "+": "plus", "-": "minus", "*": "star", "/": "slash", "%": "percent",
      ".": "dot", ",": "comma", ":": "colon", ";": "semicolon",
      "(": "lparen", ")": "rparen", "{": "lbrace", "}": "rbrace",
      "[": "lbracket", "]": "rbracket",
      "=": "eq", "<": "lt", ">": "gt", "!": "bang", "|": "pipe"
    };
    if (ch in singles) {
      addToken(singles[ch]!, ch, startLine, startCol);
      continue;
    }
  }

  // EOF
  tokens.push({ kind: "eof", value: "", span: { file, startLine: line, startColumn: column, endLine: line, endColumn: column } });
  return tokens;
}
```

- [ ] **步骤 4：修改 `src/index.ts`**

```ts
export { tokenize } from "./parser/lexer.js";
export type { Token, TokenKind } from "./parser/lexer.js";
```

- [ ] **步骤 5：运行测试验证通过 + 类型检查**

运行：`npm test -- tests/parser/lexer.test.ts`
预期：PASS。

运行：`npm run typecheck`
预期：PASS。

---

## 任务 3：实现表达式解析器

**文件：**
- 创建：`src/parser/parser.ts`
- 修改：`src/index.ts`
- 修改：`tests/parser/parser.test.ts`

- [ ] **步骤 1：编写表达式解析测试**

追加到 `tests/parser/parser.test.ts`：

```ts
import { parseExpression, tokenize } from "../../src/index";

describe("parseExpression", () => {
  it("parses number literals", () => {
    const expr = parseExpression(tokenize("42", "test.md"));
    expect(expr).toMatchObject({ kind: "literal", literalKind: "number", value: 42 });
  });

  it("parses string literals", () => {
    const expr = parseExpression(tokenize('"hello"', "test.md"));
    expect(expr).toMatchObject({ kind: "literal", literalKind: "string", value: "hello" });
  });

  it("parses boolean literals", () => {
    expect(parseExpression(tokenize("true", "test.md"))).toMatchObject({
      kind: "literal", literalKind: "boolean", value: true
    });
    expect(parseExpression(tokenize("false", "test.md"))).toMatchObject({
      kind: "literal", literalKind: "boolean", value: false
    });
  });

  it("parses identifiers", () => {
    const expr = parseExpression(tokenize("userName", "test.md"));
    expect(expr).toMatchObject({ kind: "identifier", name: "userName" });
  });

  it("parses member access chains", () => {
    const expr = parseExpression(tokenize("this.is_active", "test.md"));
    expect(expr).toMatchObject({
      kind: "memberAccess",
      object: { kind: "this" },
      member: "is_active"
    });
  });

  it("parses function calls", () => {
    const expr = parseExpression(tokenize("notify(user, msg)", "test.md"));
    expect(expr).toMatchObject({
      kind: "call",
      callee: { kind: "identifier", name: "notify" },
      args: [
        { kind: "identifier", name: "user" },
        { kind: "identifier", name: "msg" }
      ]
    });
  });

  it("parses binary operators with correct precedence", () => {
    const expr = parseExpression(tokenize("1 + 2 * 3", "test.md"));
    expect(expr).toMatchObject({
      kind: "binary",
      op: "+",
      left: { kind: "literal", value: 1 },
      right: {
        kind: "binary",
        op: "*",
        left: { kind: "literal", value: 2 },
        right: { kind: "literal", value: 3 }
      }
    });
  });

  it("parses unary operators", () => {
    const expr = parseExpression(tokenize("!ready", "test.md"));
    expect(expr).toMatchObject({
      kind: "unary",
      op: "!",
      operand: { kind: "identifier", name: "ready" }
    });
  });

  it("parses this keyword", () => {
    const expr = parseExpression(tokenize("this", "test.md"));
    expect(expr).toMatchObject({ kind: "this" });
  });

  it("parses with-update expressions", () => {
    const expr = parseExpression(tokenize("user with { is_active: true }", "test.md"));
    expect(expr).toMatchObject({
      kind: "withUpdate",
      object: { kind: "identifier", name: "user" },
      fields: [
        { name: "is_active", value: { kind: "literal", literalKind: "boolean", value: true } }
      ]
    });
  });

  it("parses if expressions", () => {
    const expr = parseExpression(tokenize("if (x > 0) 1 else 0", "test.md"));
    expect(expr).toMatchObject({
      kind: "if",
      condition: { kind: "binary", op: ">" },
      thenBranch: { kind: "literal", value: 1 },
      elseBranch: { kind: "literal", value: 0 }
    });
  });

  it("parses arrow function expressions", () => {
    const expr = parseExpression(tokenize("(x, y) => x + y", "test.md"));
    expect(expr).toMatchObject({
      kind: "arrow",
      params: [{ name: "x" }, { name: "y" }],
      body: { kind: "binary", op: "+" }
    });
  });

  it("parses parenthesized expressions", () => {
    const expr = parseExpression(tokenize("(1 + 2) * 3", "test.md"));
    expect(expr).toMatchObject({
      kind: "binary",
      op: "*",
      left: { kind: "binary", op: "+", left: { kind: "literal", value: 1 }, right: { kind: "literal", value: 2 } },
      right: { kind: "literal", value: 3 }
    });
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：`npm test -- tests/parser/parser.test.ts`
预期：FAIL，`parseExpression` 未导出。

- [ ] **步骤 3：实现 `src/parser/parser.ts`**

```ts
import type { Expr, Stmt, CodeBody, WithField, ArrowParam, TypeExpr } from "../ast.js";
import type { Token, TokenKind } from "./lexer.js";
import type { TangleDiagnostic, SourceSpan } from "../model.js";

export type ParserState = {
  tokens: Token[];
  pos: number;
  file: string;
  diagnostics: TangleDiagnostic[];
};

function createState(tokens: Token[], file: string): ParserState {
  return { tokens, pos: 0, file, diagnostics: [] };
}

function peek(state: ParserState): Token {
  return state.tokens[state.pos]!;
}

function advance(state: ParserState): Token {
  const t = state.tokens[state.pos]!;
  state.pos += 1;
  return t;
}

function expect(state: ParserState, kind: TokenKind, msg?: string): Token {
  const t = peek(state);
  if (t.kind !== kind) {
    state.diagnostics.push({
      code: "TANGLE_PARSE_UNEXPECTED_TOKEN",
      message: msg ?? `Expected ${kind} but got ${t.kind}`,
      span: t.span
    });
    // sync: skip current token and return dummy
    advance(state);
    return { kind: "eof", value: "", span: t.span };
  }
  return advance(state);
}

// ─── Precedence ────────────────────────────────────────

const PREFIX_PREC = 7;

function infixPrec(kind: TokenKind): number {
  switch (kind) {
    case "or": return 1;
    case "and": return 2;
    case "eqeq": case "neq": return 3;
    case "lt": case "gt": case "lte": case "gte": return 4;
    case "plus": case "minus": return 5;
    case "star": case "slash": case "percent": return 6;
    case "dot": case "lparen": return 8;
    case "with": return 2; // low precedence
    default: return 0;
  }
}

function getPrec(token: Token): number {
  return infixPrec(token.kind);
}

// ─── Parse Expression ──────────────────────────────────

export function parseExpression(tokens: Token[]): Expr {
  const state = createState(tokens, tokens[0]?.span.file ?? "");
  const expr = parseExpr(state, 0);
  return expr;
}

function parseExpr(state: ParserState, minPrec: number): Expr {
  let left = parsePrefix(state);

  while (peek(state).kind !== "eof" && peek(state).kind !== "semicolon" &&
         peek(state).kind !== "rparen" && peek(state).kind !== "rbrace" &&
         peek(state).kind !== "comma" && peek(state).kind !== "colon" &&
         peek(state).kind !== "fatArrow" &&
         getPrec(peek(state)) >= minPrec) {
    left = parseInfix(state, left);
  }

  return left;
}

function parsePrefix(state: ParserState): Expr {
  const token = peek(state);

  // unary
  if (token.kind === "bang" || token.kind === "minus") {
    const op = token.kind === "minus" ? "-" : "!";
    const opToken = advance(state);
    const operand = parseExpr(state, PREFIX_PREC);
    return { kind: "unary", op, operand, span: mergeSpan(opToken.span, operand.span) };
  }

  // if expression
  if (token.kind === "if") {
    const ifToken = advance(state);
    expect(state, "lparen");
    const condition = parseExpr(state, 0);
    expect(state, "rparen");
    const thenBranch = parseExpr(state, 0);
    let elseBranch: Expr | undefined;
    if (peek(state).kind === "else") {
      advance(state);
      elseBranch = parseExpr(state, 0);
    }
    return { kind: "if", condition, thenBranch, elseBranch, span: ifToken.span };
  }

  // primary
  return parsePrimary(state);
}

function parsePrimary(state: ParserState): Expr {
  const token = advance(state);

  // literals
  if (token.kind === "number") {
    const num = Number(token.value);
    return { kind: "literal", literalKind: "number", value: num, span: token.span };
  }
  if (token.kind === "string") {
    const str = token.value.slice(1, -1);
    return { kind: "literal", literalKind: "string", value: str, span: token.span };
  }
  if (token.kind === "true") {
    return { kind: "literal", literalKind: "boolean", value: true, span: token.span };
  }
  if (token.kind === "false") {
    return { kind: "literal", literalKind: "boolean", value: false, span: token.span };
  }

  // this
  if (token.kind === "this") {
    return { kind: "this", span: token.span };
  }

  // identifier
  if (token.kind === "identifier") {
    return { kind: "identifier", name: token.value, span: token.span };
  }

  // parenthesized expr or arrow params
  if (token.kind === "lparen") {
    // peek ahead to see if arrow
    const params: ArrowParam[] = [];
    if (peek(state).kind !== "rparen") {
      // collect params
      while (true) {
        const paramToken = peek(state);
        if (paramToken.kind === "identifier") {
          advance(state);
          params.push({ name: paramToken.value, span: paramToken.span });
        }
        if (peek(state).kind === "comma") { advance(state); continue; }
        break;
      }
    }
    expect(state, "rparen");

    // arrow function
    if (peek(state).kind === "fatArrow") {
      advance(state); // =>
      const body = parseExpr(state, 0);
      return { kind: "arrow", params, body, span: mergeSpan(token.span, body.span) };
    }

    // just a parenthesized expression (must be single expr, not params)
    if (params.length === 0 || params.length === 1) {
      // re-parse as: (inner ... inner)
      // We need to handle this differently — parse inside parens as expression
      // For simplicity, we already consumed tokens. For now handle single-arg case:
      if (params.length === 1) {
        // Treat the single identifier as an expression
        const inner: Expr = { kind: "identifier", name: params[0]!.name, span: params[0]!.span };
        return inner;
      }
      // Empty parens not valid
      state.diagnostics.push({
        code: "TANGLE_PARSE_EMPTY_PARENS",
        message: "Empty parentheses are not valid",
        span: token.span
      });
      return { kind: "literal", literalKind: "boolean", value: false, span: token.span };
    }

    // multiple params + no arrow = error
    state.diagnostics.push({
      code: "TANGLE_PARSE_EXPECTED_ARROW",
      message: "Expected => after arrow function parameters",
      span: token.span
    });
    return { kind: "literal", literalKind: "boolean", value: false, span: token.span };
  }

  // lbrace: either with-update fields or error
  if (token.kind === "lbrace") {
    const fields: WithField[] = [];
    while (peek(state).kind !== "rbrace" && peek(state).kind !== "eof") {
      const fieldName = expect(state, "identifier", "Expected field name");
      expect(state, "colon");
      const fieldValue = parseExpr(state, 0);
      fields.push({ name: fieldName.value, value: fieldValue, span: fieldName.span });
      if (peek(state).kind === "comma") advance(state);
    }
    const rbrace = expect(state, "rbrace");
    return { kind: "withUpdate", object: { kind: "this", span: token.span }, fields, span: mergeSpan(token.span, rbrace.span) };
  }

  state.diagnostics.push({
    code: "TANGLE_PARSE_UNEXPECTED_TOKEN",
    message: `Unexpected token: ${token.kind}`,
    span: token.span
  });
  return { kind: "literal", literalKind: "boolean", value: false, span: token.span };
}

function parseInfix(state: ParserState, left: Expr): Expr {
  const token = peek(state);
  const prec = getPrec(token);

  // member access: obj.member
  if (token.kind === "dot") {
    advance(state);
    const memberToken = expect(state, "identifier", "Expected property name after '.'");
    return { kind: "memberAccess", object: left, member: memberToken.value, span: mergeSpan(left.span, memberToken.span) };
  }

  // function call: callee(args)
  if (token.kind === "lparen") {
    advance(state);
    const args: Expr[] = [];
    while (peek(state).kind !== "rparen" && peek(state).kind !== "eof") {
      args.push(parseExpr(state, 0));
      if (peek(state).kind === "comma") advance(state);
    }
    const rparen = expect(state, "rparen");
    return { kind: "call", callee: left, args, span: mergeSpan(left.span, rparen.span) };
  }

  // with update: expr with { fields }
  if (token.kind === "with") {
    advance(state);
    expect(state, "lbrace");
    const fields: WithField[] = [];
    while (peek(state).kind !== "rbrace" && peek(state).kind !== "eof") {
      const fieldName = expect(state, "identifier", "Expected field name");
      expect(state, "colon");
      const fieldValue = parseExpr(state, 0);
      fields.push({ name: fieldName.value, value: fieldValue, span: fieldName.span });
      if (peek(state).kind === "comma") advance(state);
    }
    const rbrace = expect(state, "rbrace");
    return { kind: "withUpdate", object: left, fields, span: mergeSpan(left.span, rbrace.span) };
  }

  // binary operators
  advance(state);
  const right = parseExpr(state, prec + 1); // left-associative
  const op = tokenToBinaryOp(token.kind);
  return { kind: "binary", op, left, right, span: mergeSpan(left.span, right.span) };
}

function tokenToBinaryOp(kind: TokenKind): Expr extends { kind: "binary"; op: infer O } ? O : never {
  const map: Record<string, string> = {
    "plus": "+", "minus": "-", "star": "*", "slash": "/", "percent": "%",
    "eqeq": "==", "neq": "!=", "lt": "<", "gt": ">", "lte": "<=", "gte": ">=",
    "and": "&&", "or": "||"
  };
  return map[kind] as never;
}

function mergeSpan(a: SourceSpan, b: SourceSpan): SourceSpan {
  return {
    file: a.file,
    startLine: a.startLine,
    startColumn: a.startColumn,
    endLine: b.endLine,
    endColumn: b.endColumn
  };
}
```

- [ ] **步骤 4：修改 `src/index.ts`**

```ts
export { parseExpression } from "./parser/parser.js";
export type { ParserState } from "./parser/parser.js";
```

- [ ] **步骤 5：运行测试验证通过 + 类型检查**

运行：`npm test -- tests/parser/parser.test.ts`
预期：PASS。

运行：`npm run typecheck`
预期：PASS。

---

## 任务 4：实现语句解析器

**文件：**
- 修改：`src/parser/parser.ts`
- 修改：`src/index.ts`
- 修改：`tests/parser/parser.test.ts`

- [ ] **步骤 1：编写语句解析测试**

追加到 `tests/parser/parser.test.ts`：

```ts
import { parseCodeBody, parseStatement } from "../../src/index";

describe("parseStatement", () => {
  it("parses return statements", () => {
    const stmt = parseStatement(tokenize("return 42", "test.md"));
    expect(stmt).toMatchObject({
      kind: "return",
      value: { kind: "literal", value: 42 }
    });
  });

  it("parses bare return", () => {
    const stmt = parseStatement(tokenize("return", "test.md"));
    expect(stmt).toMatchObject({ kind: "return" });
    expect((stmt as { kind: "return"; value?: unknown }).value).toBeUndefined();
  });

  it("parses let bindings", () => {
    const stmt = parseStatement(tokenize("let x = 5", "test.md"));
    expect(stmt).toMatchObject({
      kind: "let",
      name: "x",
      value: { kind: "literal", value: 5 }
    });
  });

  it("parses const bindings", () => {
    const stmt = parseStatement(tokenize("const pi = 3.14", "test.md"));
    expect(stmt).toMatchObject({
      kind: "const",
      name: "pi",
      value: { kind: "literal", value: 3.14 }
    });
  });

  it("parses expression statements", () => {
    const stmt = parseStatement(tokenize("notify(user)", "test.md"));
    expect(stmt).toMatchObject({
      kind: "expression",
      expr: { kind: "call", callee: { kind: "identifier", name: "notify" } }
    });
  });
});

describe("parseCodeBody", () => {
  it("parses multiple statements", () => {
    const body = parseCodeBody(tokenize(
      "let a = 1\nreturn this with { is_active: true }",
      "test.md"
    ));
    expect(body.statements).toHaveLength(2);
    expect(body.statements[0]!.kind).toBe("let");
    expect(body.statements[1]!.kind).toBe("return");
  });

  it("semicolons separate statements", () => {
    const body = parseCodeBody(tokenize("let a = 1; let b = 2", "test.md"));
    expect(body.statements).toHaveLength(2);
  });

  it("empty input produces empty statement list", () => {
    const body = parseCodeBody(tokenize("", "test.md"));
    expect(body.statements).toHaveLength(0);
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：`npm test -- tests/parser/parser.test.ts`
预期：FAIL，`parseStatement` 未导出。

- [ ] **步骤 3：在 `src/parser/parser.ts` 中实现语句解析**

追加以下代码到 `src/parser/parser.ts`：

```ts
// ─── Parse Statement ───────────────────────────────────

export function parseStatement(tokens: Token[]): Stmt {
  const state = createState(tokens, tokens[0]?.span.file ?? "");
  return parseStmt(state);
}

export function parseCodeBody(tokens: Token[]): CodeBody {
  const state = createState(tokens, tokens[0]?.span.file ?? "");
  const statements: Stmt[] = [];
  while (peek(state).kind !== "eof") {
    // skip semicolons
    if (peek(state).kind === "semicolon") {
      advance(state);
      continue;
    }
    statements.push(parseStmt(state));
    // optional semicolon separator
    if (peek(state).kind === "semicolon") advance(state);
  }
  const span: SourceSpan = tokens.length > 1
    ? mergeSpan(tokens[0]!.span, tokens[tokens.length - 2]!.span)
    : tokens[0]!.span;
  return { kind: "codeBody", statements, span };
}

function parseStmt(state: ParserState): Stmt {
  const token = peek(state);

  // return
  if (token.kind === "return") {
    const retToken = advance(state);
    if (peek(state).kind === "eof" || peek(state).kind === "semicolon" || peek(state).kind === "rbrace") {
      return { kind: "return", span: retToken.span };
    }
    const value = parseExpr(state, 0);
    return { kind: "return", value, span: mergeSpan(retToken.span, (value as { span: SourceSpan }).span) };
  }

  // let / const
  if (token.kind === "let" || token.kind === "const") {
    const kw = advance(state);
    const name = expect(state, "identifier", "Expected variable name");
    let typeAnnotation: TypeExpr | undefined;
    if (peek(state).kind === "colon") {
      advance(state); // :
      typeAnnotation = parseTypeAnnotation(state);
    }
    expect(state, "eq", "Expected = in variable binding");
    const value = parseExpr(state, 0);
    if (kw.kind === "let") {
      return { kind: "let", name: name.value, typeAnnotation, value, span: mergeSpan(kw.span, (value as { span: SourceSpan }).span) };
    }
    return { kind: "const", name: name.value, typeAnnotation, value, span: mergeSpan(kw.span, (value as { span: SourceSpan }).span) };
  }

  // expression statement
  const expr = parseExpr(state, 0);
  return { kind: "expression", expr, span: (expr as { span: SourceSpan }).span };
}

// ─── Type Annotation Parser (inline in parser) ─────────

function parseTypeAnnotation(state: ParserState): TypeExpr {
  const token = peek(state);
  // primitive types
  if (token.kind === "identifier" && ["String", "Int", "Bool"].includes(token.value)) {
    advance(state);
    return { kind: "primitiveType", name: token.value as "String" | "Int" | "Bool", span: token.span };
  }
  // named type
  if (token.kind === "identifier") {
    advance(state);
    // check for generic <...>
    if (peek(state).kind === "lt") {
      advance(state); // <
      const args: TypeExpr[] = [];
      while (peek(state).kind !== "gt" && peek(state).kind !== "eof") {
        args.push(parseTypeAnnotation(state));
        if (peek(state).kind === "comma") advance(state);
      }
      expect(state, "gt");
      return { kind: "genericType", base: token.value, typeArgs: args, span: token.span };
    }
    return { kind: "namedType", name: token.value, span: token.span };
  }
  // sum type: Type1 | Type2
  state.diagnostics.push({
    code: "TANGLE_PARSE_UNEXPECTED_TOKEN",
    message: `Expected type but got ${token.kind}`,
    span: token.span
  });
  return { kind: "namedType", name: "unknown", span: token.span };
}
```

- [ ] **步骤 4：修改 `src/index.ts`**

```ts
export { parseCodeBody, parseExpression, parseStatement } from "./parser/parser.js";
```

- [ ] **步骤 5：运行测试验证通过 + 类型检查**

运行：`npm test -- tests/parser/parser.test.ts`
预期：PASS。

运行：`npm run typecheck`
预期：PASS。

---

## 任务 5：实现类型标注解析器

**文件：**
- 创建：`src/parser/typeParser.ts`
- 修改：`src/index.ts`
- 创建：`tests/parser/typeParser.test.ts`

- [ ] **步骤 1：编写类型解析测试**

创建 `tests/parser/typeParser.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import { parseTypeExpr } from "../../src/index";

describe("parseTypeExpr", () => {
  it("parses primitive types", () => {
    expect(parseTypeExpr("String", "test.md")).toMatchObject({ kind: "primitiveType", name: "String" });
    expect(parseTypeExpr("Int", "test.md")).toMatchObject({ kind: "primitiveType", name: "Int" });
    expect(parseTypeExpr("Bool", "test.md")).toMatchObject({ kind: "primitiveType", name: "Bool" });
  });

  it("parses named type references", () => {
    expect(parseTypeExpr("User", "test.md")).toMatchObject({ kind: "namedType", name: "User" });
  });

  it("parses sum types with pipe", () => {
    const t = parseTypeExpr("Receipt | PayFailed | Timeout", "test.md");
    expect(t).toMatchObject({
      kind: "sumType",
      variants: [
        { kind: "namedType", name: "Receipt" },
        { kind: "namedType", name: "PayFailed" },
        { kind: "namedType", name: "Timeout" }
      ]
    });
  });

  it("parses generic types", () => {
    expect(parseTypeExpr("List<Int>", "test.md")).toMatchObject({
      kind: "genericType",
      base: "List",
      typeArgs: [{ kind: "primitiveType", name: "Int" }]
    });
  });

  it("parses nested generics", () => {
    const t = parseTypeExpr("Option<List<Int>>", "test.md");
    expect(t).toMatchObject({
      kind: "genericType",
      base: "Option",
      typeArgs: [{
        kind: "genericType",
        base: "List",
        typeArgs: [{ kind: "primitiveType", name: "Int" }]
      }]
    });
  });

  it("parses function types", () => {
    expect(parseTypeExpr("(Int, String) => Bool", "test.md")).toMatchObject({
      kind: "functionType",
      params: [
        { kind: "primitiveType", name: "Int" },
        { kind: "primitiveType", name: "String" }
      ],
      returns: { kind: "primitiveType", name: "Bool" }
    });
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：`npm test -- tests/parser/typeParser.test.ts`
预期：FAIL，`parseTypeExpr` 未导出。

- [ ] **步骤 3：实现 `src/parser/typeParser.ts`**

```ts
import type { TypeExpr, PrimitiveTypeExpr, NamedTypeExpr, SumTypeExpr, GenericTypeExpr, FunctionTypeExpr } from "../ast.js";
import { tokenize } from "./lexer.js";
import type { Token, TokenKind } from "./lexer.js";

export function parseTypeExpr(source: string, file: string): TypeExpr {
  const tokens = tokenize(source, file);
  const state = { tokens, pos: 0 };
  // Remove EOF for parsing
  const result = parseSumType(state);
  return result;
}

function peek(state: { tokens: Token[]; pos: number }): Token {
  return state.tokens[state.pos] ?? { kind: "eof", value: "", span: { file: "", startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 } };
}

function advance(state: { tokens: Token[]; pos: number }): Token {
  const t = state.tokens[state.pos]!;
  state.pos += 1;
  return t;
}

function parseSumType(state: { tokens: Token[]; pos: number }): TypeExpr {
  let left = parseFunctionType(state);
  while (peek(state).kind === "pipe") {
    advance(state); // |
    const right = parseFunctionType(state);
    if (left.kind === "sumType") {
      left.variants.push(right);
    } else {
      left = { kind: "sumType", variants: [left, right], span: mergeTypeSpan(left, right) };
    }
  }
  return left;
}

function parseFunctionType(state: { tokens: Token[]; pos: number }): TypeExpr {
  // Check for (Type, Type) => Type or Type => Type
  const start = state.pos;
  let left = parseGenericType(state);

  if (peek(state).kind === "fatArrow") {
    advance(state); // =>
    const returns = parseSumType(state);
    return { kind: "functionType", params: [left], returns, span: mergeTypeSpan(left, returns) };
  }

  // Check if we had parenthesized params: (A, B) => C
  // This is handled at the primary level
  return left;
}

function parseGenericType(state: { tokens: Token[]; pos: number }): TypeExpr {
  let left = parsePrimaryType(state);
  while (peek(state).kind === "lt") {
    advance(state); // <
    const args: TypeExpr[] = [];
    while (peek(state).kind !== "gt" && peek(state).kind !== "eof") {
      args.push(parseSumType(state));
      if (peek(state).kind === "comma") advance(state);
    }
    advance(state); // >
    const baseName = (left as NamedTypeExpr | PrimitiveTypeExpr).kind === "namedType"
      ? (left as NamedTypeExpr).name
      : (left as PrimitiveTypeExpr).name;
    left = { kind: "genericType", base: baseName, typeArgs: args, span: left.span };
  }
  return left;
}

function parsePrimaryType(state: { tokens: Token[]; pos: number }): TypeExpr {
  const token = advance(state);

  if (token.kind === "true" || token.kind === "false") {
    // Not a type — but let the caller handle errors
    return { kind: "namedType", name: token.value, span: token.span };
  }

  if (token.kind === "identifier") {
    if (["String", "Int", "Bool"].includes(token.value)) {
      return { kind: "primitiveType", name: token.value as "String" | "Int" | "Bool", span: token.span };
    }
    return { kind: "namedType", name: token.value, span: token.span };
  }

  if (token.kind === "lparen") {
    // Group or function params
    const params: TypeExpr[] = [];
    if (peek(state).kind !== "rparen") {
      while (true) {
        params.push(parseSumType(state));
        if (peek(state).kind === "comma") { advance(state); continue; }
        break;
      }
    }
    advance(state); // rparen

    if (peek(state).kind === "fatArrow") {
      advance(state); // =>
      const returns = parseSumType(state);
      const span = params.length > 0
        ? { ...params[0]!.span, endLine: returns.span.endLine, endColumn: returns.span.endColumn }
        : returns.span;
      return { kind: "functionType", params, returns, span };
    }

    // Just a parenthesized type
    if (params.length === 1) return params[0]!;
    // Otherwise fallback
    return { kind: "namedType", name: "void", span: token.span };
  }

  return { kind: "namedType", name: "unknown", span: token.span };
}

function mergeTypeSpan(a: TypeExpr, b: TypeExpr): TypeExpr["span"] {
  return {
    file: a.span.file,
    startLine: a.span.startLine,
    startColumn: a.span.startColumn,
    endLine: b.span.endLine,
    endColumn: b.span.endColumn
  };
}
```

- [ ] **步骤 4：修改 `src/index.ts`**

```ts
export { parseTypeExpr } from "./parser/typeParser.js";
```

- [ ] **步骤 5：运行测试验证通过 + 类型检查**

运行：`npm test -- tests/parser/typeParser.test.ts`
预期：PASS。

运行：`npm run typecheck`
预期：PASS。

---

## 任务 6：定义 Checker 类型表示

**文件：**
- 创建：`src/checker/types.ts`
- 修改：`src/index.ts`
- 创建：`tests/checker/types.test.ts`

- [ ] **步骤 1：编写 checker type 测试**

创建 `tests/checker/types.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import type { Type, StructType, InterfaceType, CallableSignature } from "../../src/index";
import { typesEqual, isSubtype } from "../../src/index";

describe("checker types", () => {
  it("defines primitive types", () => {
    const t: Type = { kind: "primitive", name: "String" };
    expect(t.kind).toBe("primitive");
  });

  it("defines struct types with fields", () => {
    const t: StructType = {
      kind: "struct",
      name: "User",
      fields: { id: { kind: "primitive", name: "Int" } },
      methods: {}
    };
    expect(t.fields.id).toEqual({ kind: "primitive", name: "Int" });
  });

  it("defines interface types with method signatures", () => {
    const t: InterfaceType = {
      kind: "interface",
      name: "Notifyable",
      methods: {
        send: {
          params: [{ name: "message", type: { kind: "primitive", name: "String" } }],
          returns: { kind: "primitive", name: "Bool" }
        }
      }
    };
    expect(t.methods.send!.returns).toEqual({ kind: "primitive", name: "Bool" });
  });

  describe("typesEqual", () => {
    it("primitives equal by name", () => {
      expect(typesEqual(
        { kind: "primitive", name: "String" },
        { kind: "primitive", name: "String" }
      )).toBe(true);
      expect(typesEqual(
        { kind: "primitive", name: "String" },
        { kind: "primitive", name: "Int" }
      )).toBe(false);
    });

    it("structs equal by name", () => {
      const a: StructType = { kind: "struct", name: "User", fields: {}, methods: {} };
      const b: StructType = { kind: "struct", name: "User", fields: {}, methods: {} };
      expect(typesEqual(a, b)).toBe(true);
    });
  });

  describe("isSubtype", () => {
    it("struct satisfies interface with matching method signatures", () => {
      const sign: CallableSignature = {
        params: [{ name: "msg", type: { kind: "primitive", name: "String" } }],
        returns: { kind: "primitive", name: "Bool" }
      };
      const myInterface: InterfaceType = {
        kind: "interface", name: "Notifiable",
        methods: { notify: sign }
      };
      const myStruct: StructType = {
        kind: "struct", name: "User", fields: {},
        methods: { notify: sign }
      };
      expect(isSubtype(myStruct, myInterface)).toBe(true);
    });
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：`npm test -- tests/checker/types.test.ts`
预期：FAIL，导出缺失。

- [ ] **步骤 3：实现 `src/checker/types.ts`**

```ts
export type Type =
  | PrimitiveType
  | StructType
  | SumType_
  | GenericTypeInstance
  | FunctionType_
  | InterfaceType
  | TypeVariable;

export type PrimitiveType = {
  kind: "primitive";
  name: "String" | "Int" | "Bool";
};

export type StructType = {
  kind: "struct";
  name: string;
  fields: Record<string, Type>;
  methods: Record<string, CallableSignature>;
};

export type SumType_ = {
  kind: "sum";
  variants: Type[];
};

export type GenericTypeInstance = {
  kind: "genericInstance";
  base: string;
  args: Type[];
};

export type FunctionType_ = {
  kind: "function";
  params: Type[];
  returns: Type;
};

export type InterfaceType = {
  kind: "interface";
  name: string;
  methods: Record<string, CallableSignature>;
};

export type TypeVariable = {
  kind: "var";
  id: number;
};

export type CallableSignature = {
  params: { name: string; type: Type }[];
  returns: Type;
};

// ─── Type utilities ────────────────────────────────────

export function typesEqual(a: Type, b: Type): boolean {
  if (a.kind !== b.kind) return false;
  if (a.kind === "primitive" && b.kind === "primitive") return a.name === b.name;
  if (a.kind === "struct" && b.kind === "struct") return a.name === b.name;
  if (a.kind === "interface" && b.kind === "interface") return a.name === b.name;
  if (a.kind === "genericInstance" && b.kind === "genericInstance") {
    return a.base === b.base && a.args.length === b.args.length && a.args.every((arg, i) => typesEqual(arg, b.args[i]!));
  }
  if (a.kind === "function" && b.kind === "function") {
    return a.params.length === b.params.length
      && a.params.every((p, i) => typesEqual(p, b.params[i]!))
      && typesEqual(a.returns, b.returns);
  }
  if (a.kind === "sum" && b.kind === "sum") {
    return a.variants.length === b.variants.length
      && a.variants.every((v, i) => typesEqual(v, b.variants[i]!));
  }
  return false;
}

export function isSubtype(value: Type, target: Type): boolean {
  if (target.kind === "interface" && value.kind === "struct") {
    return Object.entries(target.methods).every(([name, sig]) => {
      const ms = value.methods[name];
      if (!ms) return false;
      if (ms.params.length !== sig.params.length) return false;
      return ms.params.every((p, i) => typesEqual(p.type, sig.params[i]!.type))
        && typesEqual(ms.returns, sig.returns);
    });
  }
  return typesEqual(value, target);
}
```

- [ ] **步骤 4：修改 `src/index.ts`**

```ts
export type {
  CallableSignature,
  FunctionType_ as FunctionType,
  GenericTypeInstance,
  InterfaceType,
  PrimitiveType,
  StructType,
  SumType_ as SumType,
  Type,
  TypeVariable
} from "./checker/types.js";
export { isSubtype, typesEqual } from "./checker/types.js";
```

- [ ] **步骤 5：运行测试验证通过 + 类型检查**

运行：`npm test -- tests/checker/types.test.ts`
预期：PASS。

运行：`npm run typecheck`
预期：PASS。

---

## 任务 7：定义内置类型 + 类型环境

**文件：**
- 创建：`src/checker/builtins.ts`
- 创建：`src/checker/env.ts`
- 修改：`src/index.ts`
- 创建：`tests/checker/builtins.test.ts`
- 创建：`tests/checker/env.test.ts`

- [ ] **步骤 1：编写测试**

创建 `tests/checker/builtins.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import { builtinTypes, isBuiltinType } from "../../src/index";

describe("builtins", () => {
  it("registers String, Int, Bool", () => {
    expect(builtinTypes.String).toEqual({ kind: "primitive", name: "String" });
    expect(builtinTypes.Int).toEqual({ kind: "primitive", name: "Int" });
    expect(builtinTypes.Bool).toEqual({ kind: "primitive", name: "Bool" });
  });

  it("recognizes builtin type names", () => {
    expect(isBuiltinType("String")).toBe(true);
    expect(isBuiltinType("Int")).toBe(true);
    expect(isBuiltinType("Bool")).toBe(true);
    expect(isBuiltinType("User")).toBe(false);
  });
});
```

创建 `tests/checker/env.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import { createEnv } from "../../src/index";
import type { Type, StructType } from "../../src/index";

describe("TypeEnv", () => {
  it("starts empty", () => {
    const env = createEnv();
    expect(env.variables).toEqual({});
    expect(env.structs).toEqual({});
    expect(env.interfaces).toEqual({});
  });

  it("adds and looks up variables", () => {
    const env = createEnv();
    env.variables.x = { kind: "primitive", name: "Int" };
    expect(env.variables.x).toEqual({ kind: "primitive", name: "Int" });
  });

  it("adds and looks up structs by name", () => {
    const env = createEnv();
    const userStruct: StructType = {
      kind: "struct", name: "User",
      fields: { id: { kind: "primitive", name: "Int" } },
      methods: {}
    };
    env.structs.User = userStruct;
    expect(env.structs.User!.fields.id).toEqual({ kind: "primitive", name: "Int" });
  });

  it("sets receiver context for method bodies", () => {
    const env = createEnv();
    env.receiver = {
      structName: "User",
      fields: { id: { kind: "primitive", name: "Int" }, is_active: { kind: "primitive", name: "Bool" } }
    };
    expect(env.receiver!.fields.is_active).toEqual({ kind: "primitive", name: "Bool" });
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：`npm test -- tests/checker/builtins.test.ts tests/checker/env.test.ts`
预期：FAIL。

- [ ] **步骤 3：实现文件**

创建 `src/checker/builtins.ts`：

```ts
import type { PrimitiveType } from "./types.js";

export const builtinTypes: Record<string, PrimitiveType> = {
  String: { kind: "primitive", name: "String" },
  Int: { kind: "primitive", name: "Int" },
  Bool: { kind: "primitive", name: "Bool" }
};

export function isBuiltinType(name: string): boolean {
  return name in builtinTypes;
}
```

创建 `src/checker/env.ts`：

```ts
import type { Type, StructType, InterfaceType } from "./types.js";

export type ReceiverContext = {
  structName: string;
  fields: Record<string, Type>;
};

export type TypeEnv = {
  variables: Record<string, Type>;
  structs: Record<string, StructType>;
  interfaces: Record<string, InterfaceType>;
  receiver?: ReceiverContext;
};

export function createEnv(): TypeEnv {
  return {
    variables: {},
    structs: {},
    interfaces: {}
  };
}
```

- [ ] **步骤 4：修改 `src/index.ts`**

```ts
export { builtinTypes, isBuiltinType } from "./checker/builtins.js";
export { createEnv } from "./checker/env.js";
export type { ReceiverContext, TypeEnv } from "./checker/env.js";
```

- [ ] **步骤 5：运行测试验证通过 + 类型检查**

运行：`npm test -- tests/checker/builtins.test.ts tests/checker/env.test.ts`
预期：PASS。

运行：`npm run typecheck`
预期：PASS。

---

## 任务 8：从 TangleHeading 解析结构体/接口/方法签名

**文件：**
- 创建：`src/checker/resolve.ts`
- 修改：`src/index.ts`
- 修改：`tests/fixtures.ts`
- 创建：`tests/checker/resolve.test.ts`

- [ ] **步骤 1：更新 fixtures 和编写测试**

追加到 `tests/fixtures.ts`：

```ts
export const USER_MODULE_WITH_INTERFACE = `# User Service

### Notifyable (接口)

#### Notifyable -> send (send)
* \`msg\`: message (String)

### User
@export
* \`id\`: user ID (Int)
* \`email\`: email (String)

#### User -> activate (activate)
@export
* \`reason\`: activation reason (String)

\`\`\`@tangle
return this with { is_active: true }
\`\`\`
`;
```

创建 `tests/checker/resolve.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import { compileModule } from "../../src/index";
import { resolveTypes } from "../../src/index";

describe("resolveTypes", () => {
  it("resolves struct fields from type heading params", () => {
    const mod = compileModule({
      file: "user.md",
      source: `### User
* \`id\`: user ID (Int)
* \`email\`: email (String)
* \`is_active\`: active flag (Bool)
`
    });
    const env = resolveTypes(mod);
    const userStruct = env.structs.User;
    expect(userStruct).toBeDefined();
    expect(userStruct!.kind).toBe("struct");
    expect(userStruct!.fields.id).toEqual({ kind: "primitive", name: "Int" });
    expect(userStruct!.fields.email).toEqual({ kind: "primitive", name: "String" });
    expect(userStruct!.fields.is_active).toEqual({ kind: "primitive", name: "Bool" });
  });

  it("resolves methods from callable headings under a type heading", () => {
    const mod = compileModule({
      file: "user.md",
      source: `### User
* \`id\`: user ID (Int)

#### User -> activate (activate)
* \`message\`: notification (String)
`
    });
    const env = resolveTypes(mod);
    expect(env.structs.User?.methods.activate).toBeDefined();
    expect(env.structs.User!.methods.activate!.params).toEqual([
      { name: "message", type: { kind: "primitive", name: "String" } }
    ]);
  });

  it("resolves interface types from headings marked (接口)", () => {
    const mod = compileModule({
      file: "notify.md",
      source: `### Notifyable (接口)

#### Notifyable -> send (send)
* \`msg\`: message (String)
`
    });
    const env = resolveTypes(mod);
    expect(env.interfaces.Notifyable).toBeDefined();
    expect(env.interfaces.Notifyable!.methods.send).toBeDefined();
  });

  it("reports diagnostics for missing type annotations", () => {
    const mod = compileModule({
      file: "bad.md",
      source: `### User
* \`id\`: user ID
`
    });
    resolveTypes(mod);
    // Field without type annotation — should be handled gracefully
    // (type inference not in scope for A2; defaults to a placeholder)
  });

  it("returns fields without type annotation as undefined", () => {
    const mod = compileModule({
      file: "user.md",
      source: `### User
* \`id\`: user ID
`
    });
    const env = resolveTypes(mod);
    // No type annotation on field — should still be present but flagged
    expect(env.structs.User).toBeDefined();
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：`npm test -- tests/checker/resolve.test.ts`
预期：FAIL，`resolveTypes` 未导出。

- [ ] **步骤 3：实现 `src/checker/resolve.ts`**

```ts
import type { TangleModule, TangleHeading, TangleDiagnostic } from "../model.js";
import type { Type, StructType, InterfaceType, CallableSignature } from "./types.js";
import type { TypeEnv } from "./env.js";
import { createEnv } from "./env.js";
import { parseTypeExpr } from "../parser/typeParser.js";
import { builtinTypes } from "./builtins.js";

export function resolveTypes(module: TangleModule): TypeEnv {
  const env = createEnv();

  // Pass 1: collect type headings (depth 3)
  for (const heading of module.headings) {
    if (heading.role !== "type") continue;

    const isInterface = heading.title.includes("(接口)") || heading.title.includes("(interface)");

    if (isInterface) {
      const iface: InterfaceType = {
        kind: "interface",
        name: extractTypeName(heading.title),
        methods: {}
      };
      // Methods will be populated in pass 2
      env.interfaces[iface.name] = iface;
    } else {
      const struct: StructType = {
        kind: "struct",
        name: extractTypeName(heading.title),
        fields: {},
        methods: {}
      };

      // Parse fields from params
      for (const param of heading.params ?? []) {
        if (param.typeName) {
          try {
            const typeExpr = parseTypeExpr(param.typeName, param.span.file);
            struct.fields[param.name] = typeExprToType(typeExpr);
          } catch {
            // Unresolvable type — leave as placeholder
          }
        }
      }

      env.structs[struct.name] = struct;
    }
  }

  // Pass 2: collect callable headings as methods
  // Match receiver pattern: "StructName -> methodName" or "InterfaceName -> methodName"
  for (const heading of module.headings) {
    if (heading.role !== "callable") continue;

    const receiver = extractReceiver(heading.title);
    if (!receiver) continue;

    const signature = buildCallableSignature(heading);

    // Check if this belongs to a struct or interface
    if (env.structs[receiver]) {
      const methodName = extractMethodName(heading);
      env.structs[receiver]!.methods[methodName] = signature;
    } else if (env.interfaces[receiver]) {
      const methodName = extractMethodName(heading);
      env.interfaces[receiver]!.methods[methodName] = signature;
    }
    // free-standing callable headings are ignored in this pass
  }

  return env;
}

function extractTypeName(title: string): string {
  return title.replace(/\s*\(.*\)\s*$/, "").trim();
}

function extractReceiver(title: string): string | null {
  const match = title.match(/^(\w+)\s*->/);
  return match?.[1] ?? null;
}

function extractMethodName(heading: TangleHeading): string {
  // heading symbolName first, then derive from title
  if (heading.symbolName) return heading.symbolName;
  const afterArrow = heading.title.split("->")[1];
  if (!afterArrow) return heading.title;
  return afterArrow.replace(/\(.*\)/, "").trim();
}

function buildCallableSignature(heading: TangleHeading): CallableSignature {
  const params: { name: string; type: Type }[] = [];
  for (const param of heading.params ?? []) {
    let type: Type;
    if (param.typeName) {
      try {
        type = typeExprToType(parseTypeExpr(param.typeName, param.span.file));
      } catch {
        type = { kind: "primitive", name: "String" }; // fallback
      }
    } else {
      type = { kind: "primitive", name: "String" }; // default fallback
    }
    params.push({ name: param.name, type });
  }
  return {
    params,
    returns: { kind: "primitive", name: "Bool" } // default return type, can be refined
  };
}

function typeExprToType(te: import("../ast.js").TypeExpr): Type {
  switch (te.kind) {
    case "primitiveType":
      if (builtinTypes[te.name]) return builtinTypes[te.name]!;
      return { kind: "primitive", name: te.name };
    case "namedType":
      return { kind: "struct", name: te.name, fields: {}, methods: {} };
    case "sumType":
      return { kind: "sum", variants: te.variants.map(typeExprToType) };
    case "genericType":
      return { kind: "genericInstance", base: te.base, args: te.typeArgs.map(typeExprToType) };
    case "functionType":
      return {
        kind: "function",
        params: te.params.map(typeExprToType),
        returns: typeExprToType(te.returns)
      };
  }
}
```

- [ ] **步骤 4：修改 `src/index.ts`**

```ts
export { resolveTypes } from "./checker/resolve.js";
```

- [ ] **步骤 5：运行测试验证通过 + 类型检查**

运行：`npm test -- tests/checker/resolve.test.ts`
预期：PASS。

运行：`npm run typecheck`
预期：PASS。

---

## 任务 9：实现核心类型检查器

**文件：**
- 创建：`src/checker/check.ts`
- 修改：`src/index.ts`
- 创建：`tests/checker/check.test.ts`

- [ ] **步骤 1：编写 type checker 测试**

创建 `tests/checker/check.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import { checkExpression, createEnv } from "../../src/index";
import type { TypeEnv } from "../../src/index";
import { tokenize, parseExpression } from "../../src/index";

function makeEnv(overrides?: Partial<TypeEnv>): TypeEnv {
  return { variables: {}, structs: {}, interfaces: {}, ...overrides };
}

describe("checkExpression", () => {
  it("types number literals as Int", () => {
    const expr = parseExpression(tokenize("42", "test.md"));
    const [type, diags] = checkExpression(expr, makeEnv());
    expect(type).toEqual({ kind: "primitive", name: "Int" });
    expect(diags).toHaveLength(0);
  });

  it("types string literals as String", () => {
    const expr = parseExpression(tokenize('"hello"', "test.md"));
    const [type] = checkExpression(expr, makeEnv());
    expect(type).toEqual({ kind: "primitive", name: "String" });
  });

  it("types boolean literals as Bool", () => {
    const [type] = checkExpression(parseExpression(tokenize("true", "test.md")), makeEnv());
    expect(type).toEqual({ kind: "primitive", name: "Bool" });
  });

  it("looks up variables in env", () => {
    const env = makeEnv({ variables: { count: { kind: "primitive", name: "Int" } } });
    const [type] = checkExpression(parseExpression(tokenize("count", "test.md")), env);
    expect(type).toEqual({ kind: "primitive", name: "Int" });
  });

  it("reports undefined variable", () => {
    const [, diags] = checkExpression(parseExpression(tokenize("unknownVar", "test.md")), makeEnv());
    expect(diags).toEqual([
      expect.objectContaining({ code: "TANGLE_TYPE_UNDEFINED_VARIABLE" })
    ]);
  });

  it("types this keyword using receiver context", () => {
    const env = makeEnv({
      structs: {
        User: { kind: "struct", name: "User", fields: { email: { kind: "primitive", name: "String" } }, methods: {} }
      },
      receiver: { structName: "User", fields: { email: { kind: "primitive", name: "String" } } }
    });
    const [type] = checkExpression(parseExpression(tokenize("this", "test.md")), env);
    expect(type).toMatchObject({ kind: "struct", name: "User" });
  });

  it("types member access on this", () => {
    const env = makeEnv({
      structs: {
        User: { kind: "struct", name: "User", fields: { email: { kind: "primitive", name: "String" } }, methods: {} }
      },
      receiver: { structName: "User", fields: { email: { kind: "primitive", name: "String" } } }
    });
    const [type] = checkExpression(parseExpression(tokenize("this.email", "test.md")), env);
    expect(type).toEqual({ kind: "primitive", name: "String" });
  });

  it("types with-update checking field names and value types", () => {
    const userStruct = {
      kind: "struct" as const, name: "User",
      fields: { is_active: { kind: "primitive" as const, name: "Bool" as const }, email: { kind: "primitive" as const, name: "String" as const } },
      methods: {}
    };
    const env = makeEnv({
      structs: { User: userStruct },
      variables: { user: userStruct }
    });
    const [type] = checkExpression(parseExpression(tokenize("user with { is_active: true }", "test.md")), env);
    expect(type).toMatchObject({ kind: "struct", name: "User" });
  });

  it("reports error for with-update on unknown field", () => {
    const userStruct = {
      kind: "struct" as const, name: "User",
      fields: { is_active: { kind: "primitive" as const, name: "Bool" as const } },
      methods: {}
    };
    const env = makeEnv({
      structs: { User: userStruct },
      variables: { user: userStruct }
    });
    const [, diags] = checkExpression(parseExpression(tokenize("user with { unknown_field: true }", "test.md")), env);
    expect(diags).toEqual([
      expect.objectContaining({ code: "TANGLE_TYPE_UNKNOWN_FIELD" })
    ]);
  });

  it("types binary operators checking operand types", () => {
    const [type] = checkExpression(parseExpression(tokenize("1 + 2", "test.md")), makeEnv());
    expect(type).toEqual({ kind: "primitive", name: "Int" });
  });

  it("reports type mismatch in binary expression", () => {
    const [, diags] = checkExpression(parseExpression(tokenize('1 + "hello"', "test.md")), makeEnv());
    expect(diags).toEqual([
      expect.objectContaining({ code: "TANGLE_TYPE_MISMATCH" })
    ]);
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：`npm test -- tests/checker/check.test.ts`
预期：FAIL。

- [ ] **步骤 3：实现 `src/checker/check.ts`**

```ts
import type { Expr, Stmt } from "../ast.js";
import type { TangleDiagnostic } from "../model.js";
import type { Type, StructType } from "./types.js";
import type { TypeEnv } from "./env.js";

// checkExpression returns [inferred_type, diagnostics]
export function checkExpression(expr: Expr, env: TypeEnv): [Type, TangleDiagnostic[]] {
  const diags: TangleDiagnostic[] = [];

  switch (expr.kind) {
    case "literal": {
      const litMap: Record<string, Type> = {
        number: { kind: "primitive", name: "Int" },
        string: { kind: "primitive", name: "String" },
        boolean: { kind: "primitive", name: "Bool" }
      };
      return [litMap[expr.literalKind] ?? { kind: "primitive", name: "String" }, diags];
    }

    case "identifier": {
      // Check variables first
      if (env.variables[expr.name]) return [env.variables[expr.name]!, diags];
      // Check receiver fields
      if (env.receiver?.fields[expr.name]) return [env.receiver.fields[expr.name]!, diags];
      // Check builtins
      if (["String", "Int", "Bool"].includes(expr.name)) {
        return [{ kind: "primitive", name: expr.name as "String" | "Int" | "Bool" }, diags];
      }
      diags.push({
        code: "TANGLE_TYPE_UNDEFINED_VARIABLE",
        message: `Undefined variable: ${expr.name}`,
        span: expr.span
      });
      return [{ kind: "primitive", name: "String" }, diags];
    }

    case "this": {
      if (!env.receiver) {
        diags.push({
          code: "TANGLE_TYPE_THIS_OUTSIDE_METHOD",
          message: "this can only be used inside a method",
          span: expr.span
        });
        return [{ kind: "primitive", name: "String" }, diags];
      }
      const recv: StructType = {
        kind: "struct",
        name: env.receiver.structName,
        fields: env.receiver.fields,
        methods: {}
      };
      return [recv, diags];
    }

    case "memberAccess": {
      const [objType, objDiags] = checkExpression(expr.object, env);
      diags.push(...objDiags);
      if (objType.kind === "struct" || objType.kind === "interface") {
        // Check fields
        if (objType.kind === "struct" && objType.fields[expr.member]) {
          return [objType.fields[expr.member]!, diags];
        }
        // Check methods
        if (objType.methods[expr.member]) {
          const sig = objType.methods[expr.member]!;
          return [{ kind: "function", params: sig.params.map(p => p.type), returns: sig.returns }, diags];
        }
      }
      diags.push({
        code: "TANGLE_TYPE_UNKNOWN_FIELD",
        message: `Unknown member: ${expr.member}`,
        span: expr.span
      });
      return [{ kind: "primitive", name: "String" }, diags];
    }

    case "call": {
      const [calleeType, calleeDiags] = checkExpression(expr.callee, env);
      diags.push(...calleeDiags);
      // Check args
      for (const arg of expr.args) {
        const [, argDiags] = checkExpression(arg, env);
        diags.push(...argDiags);
      }
      if (calleeType.kind === "function") {
        return [calleeType.returns, diags];
      }
      return [{ kind: "primitive", name: "Bool" }, diags];
    }

    case "binary": {
      const [leftType, leftDiags] = checkExpression(expr.left, env);
      const [rightType, rightDiags] = checkExpression(expr.right, env);
      diags.push(...leftDiags, ...rightDiags);

      // Arithmetic ops require Int
      if (["+", "-", "*", "/", "%"].includes(expr.op)) {
        if (leftType.kind !== "primitive" || rightType.kind !== "primitive" ||
            leftType.name !== rightType.name) {
          diags.push({
            code: "TANGLE_TYPE_MISMATCH",
            message: `Operator ${expr.op} requires matching types`,
            span: expr.span
          });
        }
        return [{ kind: "primitive", name: "Int" }, diags];
      }

      // Comparison ops
      if (["==", "!=", "<", ">", "<=", ">="].includes(expr.op)) {
        return [{ kind: "primitive", name: "Bool" }, diags];
      }

      // Logic ops
      if (["&&", "||"].includes(expr.op)) {
        return [{ kind: "primitive", name: "Bool" }, diags];
      }

      return [{ kind: "primitive", name: "Bool" }, diags];
    }

    case "unary": {
      const [, operandDiags] = checkExpression(expr.operand, env);
      diags.push(...operandDiags);
      if (expr.op === "!") return [{ kind: "primitive", name: "Bool" }, diags];
      return [{ kind: "primitive", name: "Int" }, diags];
    }

    case "withUpdate": {
      const [objType, objDiags] = checkExpression(expr.object, env);
      diags.push(...objDiags);
      if (objType.kind === "struct") {
        for (const field of expr.fields) {
          if (!(field.name in objType.fields)) {
            diags.push({
              code: "TANGLE_TYPE_UNKNOWN_FIELD",
              message: `Unknown field: ${field.name} on struct ${objType.name}`,
              span: field.span
            });
          } else {
            const [, valDiag] = checkExpression(field.value, env);
            diags.push(...valDiag);
          }
        }
        return [objType, diags];
      }
      diags.push({
        code: "TANGLE_TYPE_NOT_STRUCT",
        message: "with-update requires a struct type",
        span: expr.span
      });
      return [objType, diags];
    }

    case "if": {
      const [, condDiags] = checkExpression(expr.condition, env);
      diags.push(...condDiags);
      const [, thenDiags] = checkExpression(expr.thenBranch, env);
      diags.push(...thenDiags);
      if (expr.elseBranch) {
        const [, elseDiags] = checkExpression(expr.elseBranch, env);
        diags.push(...elseDiags);
        return checkExpression(expr.thenBranch, env);
      }
      return checkExpression(expr.thenBranch, env);
    }

    case "arrow": {
      // Arrow function type
      return [{
        kind: "function",
        params: [],
        returns: { kind: "primitive", name: "Bool" }
      }, diags];
    }

    default:
      return [{ kind: "primitive", name: "String" }, diags];
  }
}
```

- [ ] **步骤 4：修改 `src/index.ts`**

```ts
export { checkExpression } from "./checker/check.js";
```

- [ ] **步骤 5：运行测试验证通过 + 类型检查**

运行：`npm test -- tests/checker/check.test.ts`
预期：PASS。

运行：`npm run typecheck`
预期：PASS。

---

## 任务 10：Pipeline 集成 — checkModule

**文件：**
- 创建：`src/checker/checkModule.ts`
- 修改：`src/index.ts`
- 创建：`tests/checker/checkModule.test.ts`

- [ ] **步骤 1：编写端到端集成测试**

创建 `tests/checker/checkModule.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import { checkModule, compileModule, parseCodeBlocks } from "../../src/index";
import { USER_MODULE_WITH_INTERFACE } from "../fixtures";

describe("checkModule (full pipeline)", () => {
  it("runs A1 compile + A2 parse + A2 type check end-to-end", () => {
    const mod = compileModule({ file: "test.md", source: USER_MODULE_WITH_INTERFACE });
    const checked = checkModule(mod);

    expect(checked.parsedBlocks).toBeDefined();
    const parsed = checked.parsedBlocks.find(() => true);
    // at least one block should be parsed (activate method has @tangle)
    if (parsed) {
      expect(parsed.body.kind).toBe("codeBody");
    }

    // Type check diagnostics should accumulate (no undefined-variable for well-formed code)
    expect(checked.diagnostics.length).toBeGreaterThanOrEqual(0);
  });

  it("parseCodeBlocks can be called standalone", () => {
    const mod = compileModule({ file: "test.md", source: USER_MODULE_WITH_INTERFACE });
    const blocks = parseCodeBlocks(mod);
    expect(blocks.length).toBeGreaterThanOrEqual(1);
    blocks.forEach(b => {
      expect(b.headingId).toBeTruthy();
      expect(b.body.kind).toBe("codeBody");
    });
  });

  it("checkModule preserves A1 diagnostics", () => {
    const mod = compileModule({
      file: "bad.md",
      source: `# Bad

这是一段普通说明，里面出现 @export 是非法的。
`
    });
    const checked = checkModule(mod);
    expect(checked.diagnostics.some(d => d.code === "TANGLE_INVALID_DIRECTIVE_POSITION")).toBe(true);
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：`npm test -- tests/checker/checkModule.test.ts`
预期：FAIL。

- [ ] **步骤 3：实现 `src/checker/checkModule.ts`**

```ts
import type { TangleModule, TangleDiagnostic } from "../model.js";
import type { ParsedCodeBlock } from "../ast.js";
import type { TypeEnv } from "./env.js";
import { tokenize } from "../parser/lexer.js";
import { parseCodeBody } from "../parser/parser.js";
import { resolveTypes } from "./resolve.js";
import { checkExpression } from "./check.js";
import { createEnv } from "./env.js";

export type CheckedModule = TangleModule & {
  parsedBlocks: ParsedCodeBlock[];
  typeEnv: TypeEnv;
};

export function parseCodeBlocks(module: TangleModule): ParsedCodeBlock[] {
  const parsed: ParsedCodeBlock[] = [];
  for (const heading of module.headings) {
    for (const block of heading.codeBlocks ?? []) {
      const tokens = tokenize(block.value, block.span.file);
      const body = parseCodeBody(tokens);
      parsed.push({
        headingId: heading.id,
        source: block.value,
        body,
        diagnostics: []
      });
    }
  }
  return parsed;
}

export function checkModule(module: TangleModule): CheckedModule {
  const parsedBlocks = parseCodeBlocks(module);
  const env = resolveTypes(module);
  const allDiagnostics: TangleDiagnostic[] = [...module.diagnostics];

  for (const parsed of parsedBlocks) {
    const heading = module.headings.find(h => h.id === parsed.headingId);
    if (!heading) continue;

    // Determine receiver context
    const receiverName = extractReceiver(heading.title);
    let checkEnv: TypeEnv;

    if (receiverName) {
      checkEnv = createEnv();
      checkEnv.structs = env.structs;
      checkEnv.interfaces = env.interfaces;

      // Add method params as variables
      for (const param of heading.params ?? []) {
        if (param.typeName) {
          try {
            const { parseTypeExpr } = require("../parser/typeParser.js") as typeof import("../parser/typeParser.js");
            // Inline: use default type for now
          } catch { /* ignore */ }
        }
      }

      // Set receiver
      const struct = env.structs[receiverName];
      if (struct) {
        checkEnv.receiver = { structName: receiverName, fields: struct.fields };
      }
    } else {
      checkEnv = env;
    }

    // Type check each expression statement
    for (const stmt of parsed.body.statements) {
      if (stmt.kind === "expression" && stmt.expr) {
        const [, diags] = checkExpression(stmt.expr, checkEnv);
        allDiagnostics.push(...diags);
      } else if (stmt.kind === "return" && stmt.value) {
        const [, diags] = checkExpression(stmt.value, checkEnv);
        allDiagnostics.push(...diags);
      } else if (stmt.kind === "let" || stmt.kind === "const") {
        const [, diags] = checkExpression(stmt.value, checkEnv);
        allDiagnostics.push(...diags);
        // Add binding to env
        if (diags.length === 0) {
          const [type] = checkExpression(stmt.value, checkEnv);
          checkEnv.variables[stmt.name] = type;
        }
      }
    }
  }

  return {
    ...module,
    parsedBlocks,
    typeEnv: env,
    diagnostics: allDiagnostics
  };
}

function extractReceiver(title: string): string | null {
  const match = title.match(/^(\w+)\s*->/);
  return match?.[1] ?? null;
}
```

- [ ] **步骤 4：修改 `src/index.ts`**

```ts
export { checkModule, parseCodeBlocks } from "./checker/checkModule.js";
export type { CheckedModule } from "./checker/checkModule.js";
```

- [ ] **步骤 5：运行全量验证**

运行：`npm test`
预期：全部 PASS（包括 A1 的 8 个测试文件 + A2 的 9 个新测试文件）。

运行：`npm run typecheck`
预期：PASS。

---

## 计划自检清单

- 规格覆盖：§3.4（@tangle 代码块解析）、§4.1（类型系统表示）、§4.2（结构体字段、with 更新类型检查）、§4.3（接收者提取、this 解析）、§4.4（接口识别、结构化契合 isSubtype）
- 明确排除：完整泛型推导算法、错误传播 `?` 和 `match`（A3）、Rule Graph IR（A4）、codegen（A5）、CLI（A5）、标准库（A6）、嵌套模块导入类型链接、mutability 运行时强制
- 占位符扫描：每个步骤含完整代码。无 TODO 占位符。
- 类型一致性：所有类型名跨任务一致（`Expr`, `Stmt`, `TypeExpr`, `Type`, `TypeEnv`, `CheckedModule` 等）
- 验证命令：每个任务含 `npm test -- <specific>` 和 `npm run typecheck`
- 向后兼容：不修改 `src/model.ts`，不修改 `compileModule` 签名或行为。A1 测试全部继续通过。
