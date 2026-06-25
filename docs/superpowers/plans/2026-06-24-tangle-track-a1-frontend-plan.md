# Tangle Track A1 Frontend 实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 构建 Tangle TypeScript 引导期的编译前端，能把 Markdown 源文件解析为包含六级标题语义、指令、导入、结构体、函数和基础符号表的 DSL 模型。

**架构：** 使用 TypeScript + Node.js 实现一个小型前端管线：Markdown 文本先进入 Markdown 解析层，再由 Tangle 语义层识别标题层级、指令、列表参数、导入链接和代码块，最后输出稳定的 `TangleModule` DSL 模型。Track A1 不实现完整类型推导、Rule Graph、codegen 或运行时，只为后续阶段提供清晰的数据结构和测试基线。

**技术栈：** TypeScript、Node.js、Vitest、unified、remark-parse、unist-util-visit。

---

## 规格来源

实现必须遵循：

- `docs/superpowers/specs/2026-06-24-tangle-language-design.md`

本计划覆盖规格中的以下部分：

- §3.1 文件与模块
- §3.2 六级标题与作用域
- §3.3 参数、字段与返回说明
- §3.4 `@tangle` 代码块
- §3.5 指令位置纪律
- §4.2 结构体的前端建模
- §4.3 方法标题的前端建模
- §4.4 接口标题的前端建模
- §4.5 `@error` 的声明和引用提取

## 文件结构

- 创建：`package.json`  
  项目脚本和依赖声明。
- 创建：`tsconfig.json`  
  TypeScript 编译配置。
- 创建：`vitest.config.ts`  
  测试配置。
- 创建：`src/index.ts`  
  对外导出编译前端 API。
- 创建：`src/model.ts`  
  `TangleModule`、`TangleHeading`、`TangleDirective`、`TangleSymbol` 等 DSL 模型类型。
- 创建：`src/markdown/parseMarkdown.ts`  
  封装 `unified + remark-parse`，输出 Markdown AST。
- 创建：`src/front-end/sourceMap.ts`  
  从 Markdown AST 节点提取 source span。
- 创建：`src/front-end/directives.ts`  
  识别和解析 `@export`、`@entry`、`@error`、`@rule.*` 等指令。
- 创建：`src/front-end/headings.ts`  
  实现六级标题语义映射、内部符号名提取和父子层级关系。
- 创建：`src/front-end/blocks.ts`  
  识别参数列表、导入链接、引用块和 `@tangle` 代码块。
- 创建：`src/front-end/compileModule.ts`  
  将 Markdown AST 编译为 `TangleModule`。
- 创建：`tests/fixtures.ts`  
  测试样本文本。
- 创建：`tests/front-end/headings.test.ts`  
  六级标题语义测试。
- 创建：`tests/front-end/directives.test.ts`  
  指令提取和位置纪律测试。
- 创建：`tests/front-end/compileModule.test.ts`  
  端到端模块编译测试。

## 任务 1：初始化 TypeScript 测试工程

**文件：**
- 创建：`package.json`
- 创建：`tsconfig.json`
- 创建：`vitest.config.ts`
- 创建：`src/index.ts`
- 创建：`tests/front-end/headings.test.ts`

- [ ] **步骤 1：创建失败的 smoke test**

创建 `tests/front-end/headings.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import { headingRoleForDepth } from "../../src/index";

describe("headingRoleForDepth", () => {
  it("maps six markdown heading levels to Tangle roles", () => {
    expect(headingRoleForDepth(1)).toBe("program");
    expect(headingRoleForDepth(2)).toBe("section");
    expect(headingRoleForDepth(3)).toBe("type");
    expect(headingRoleForDepth(4)).toBe("callable");
    expect(headingRoleForDepth(5)).toBe("semantic-section");
    expect(headingRoleForDepth(6)).toBe("semantic-atom");
  });
});
```

- [ ] **步骤 2：创建项目配置**

创建 `package.json`：

```json
{
  "name": "tangle",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "test": "vitest run",
    "typecheck": "tsc --noEmit"
  },
  "dependencies": {
    "unified": "^11.0.5",
    "remark-parse": "^11.0.0",
    "unist-util-visit": "^5.0.0"
  },
  "devDependencies": {
    "@types/node": "^20.14.10",
    "typescript": "^5.5.4",
    "vitest": "^2.0.5"
  }
}
```

创建 `tsconfig.json`：

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "exactOptionalPropertyTypes": true,
    "declaration": true,
    "outDir": "dist",
    "rootDir": "."
  },
  "include": ["src/**/*.ts", "tests/**/*.ts", "vitest.config.ts"]
}
```

创建 `vitest.config.ts`：

```ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node"
  }
});
```

- [ ] **步骤 3：运行测试验证失败**

运行：`npm install`

运行：`npm test -- tests/front-end/headings.test.ts`

预期：FAIL，报错包含 `Cannot find module '../../src/index'`。

- [ ] **步骤 4：创建最小导出**

创建 `src/index.ts`：

```ts
export type HeadingRole =
  | "program"
  | "section"
  | "type"
  | "callable"
  | "semantic-section"
  | "semantic-atom";

export function headingRoleForDepth(depth: number): HeadingRole {
  switch (depth) {
    case 1:
      return "program";
    case 2:
      return "section";
    case 3:
      return "type";
    case 4:
      return "callable";
    case 5:
      return "semantic-section";
    case 6:
      return "semantic-atom";
    default:
      throw new Error(`Invalid Markdown heading depth: ${depth}`);
  }
}
```

- [ ] **步骤 5：运行测试验证通过**

运行：`npm test -- tests/front-end/headings.test.ts`

预期：PASS。

- [ ] **步骤 6：类型检查**

运行：`npm run typecheck`

预期：PASS。

## 任务 2：定义编译前端 DSL 模型

**文件：**
- 创建：`src/model.ts`
- 修改：`src/index.ts`
- 创建：`tests/front-end/model.test.ts`

- [ ] **步骤 1：编写模型约束测试**

创建 `tests/front-end/model.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import type { TangleHeading, TangleModule } from "../../src/index";

describe("Tangle frontend model", () => {
  it("represents a module with headings, imports, symbols, and diagnostics", () => {
    const heading: TangleHeading = {
      id: "user",
      depth: 3,
      role: "type",
      title: "User",
      symbolName: "User",
      directives: [],
      span: { file: "user.md", startLine: 1, startColumn: 1, endLine: 1, endColumn: 8 },
      children: []
    };

    const mod: TangleModule = {
      file: "user.md",
      moduleName: "user",
      imports: [],
      headings: [heading],
      symbols: [],
      diagnostics: []
    };

    expect(mod.headings[0]?.role).toBe("type");
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：`npm test -- tests/front-end/model.test.ts`

预期：FAIL，报错包含 `Module '../../src/index' has no exported member 'TangleModule'`。

- [ ] **步骤 3：实现模型类型**

创建 `src/model.ts`：

```ts
export type SourceSpan = {
  file: string;
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
};

export type HeadingRole =
  | "program"
  | "section"
  | "type"
  | "callable"
  | "semantic-section"
  | "semantic-atom";

export type DirectiveKind =
  | "export"
  | "entry"
  | "deprecated"
  | "test"
  | "hideCode"
  | "error"
  | "rule.table"
  | "rule.tree"
  | "rule.toggle"
  | "rule.flow";

export type TangleDirective = {
  kind: DirectiveKind;
  raw: string;
  name?: string;
  args?: string;
  span: SourceSpan;
};

export type TangleImport = {
  alias: string;
  target: string;
  span: SourceSpan;
};

export type TangleParam = {
  name: string;
  description: string;
  typeName?: string;
  span: SourceSpan;
};

export type TangleCodeBlock = {
  language: "tangle";
  value: string;
  span: SourceSpan;
};

export type TangleHeading = {
  id: string;
  depth: number;
  role: HeadingRole;
  title: string;
  symbolName?: string;
  directives: TangleDirective[];
  params?: TangleParam[];
  codeBlocks?: TangleCodeBlock[];
  span: SourceSpan;
  children: TangleHeading[];
};

export type SymbolKind =
  | "entry"
  | "type"
  | "callable"
  | "semantic-internal";

export type TangleSymbol = {
  name: string;
  kind: SymbolKind;
  exported: boolean;
  headingId: string;
  span: SourceSpan;
};

export type TangleDiagnostic = {
  code: string;
  message: string;
  span: SourceSpan;
};

export type TangleModule = {
  file: string;
  moduleName: string;
  imports: TangleImport[];
  headings: TangleHeading[];
  symbols: TangleSymbol[];
  diagnostics: TangleDiagnostic[];
};
```

修改 `src/index.ts`：

```ts
export type {
  DirectiveKind,
  HeadingRole,
  SourceSpan,
  SymbolKind,
  TangleCodeBlock,
  TangleDiagnostic,
  TangleDirective,
  TangleHeading,
  TangleImport,
  TangleModule,
  TangleParam,
  TangleSymbol
} from "./model.js";

import type { HeadingRole } from "./model.js";

export function headingRoleForDepth(depth: number): HeadingRole {
  switch (depth) {
    case 1:
      return "program";
    case 2:
      return "section";
    case 3:
      return "type";
    case 4:
      return "callable";
    case 5:
      return "semantic-section";
    case 6:
      return "semantic-atom";
    default:
      throw new Error(`Invalid Markdown heading depth: ${depth}`);
  }
}
```

- [ ] **步骤 4：运行测试验证通过**

运行：`npm test -- tests/front-end/model.test.ts`

预期：PASS。

- [ ] **步骤 5：类型检查**

运行：`npm run typecheck`

预期：PASS。

## 任务 3：解析 Markdown AST 和源码位置

**文件：**
- 创建：`src/markdown/parseMarkdown.ts`
- 创建：`src/front-end/sourceMap.ts`
- 修改：`src/index.ts`
- 创建：`tests/front-end/parseMarkdown.test.ts`

- [ ] **步骤 1：编写 Markdown AST 测试**

创建 `tests/front-end/parseMarkdown.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import { parseMarkdown, spanFromNode } from "../../src/index";

describe("parseMarkdown", () => {
  it("parses headings and preserves source positions", () => {
    const tree = parseMarkdown("# App\n\n### User\n");
    expect(tree.type).toBe("root");
    expect(tree.children.map((child) => child.type)).toEqual(["heading", "heading"]);

    const first = tree.children[0];
    expect(spanFromNode("main.md", first)).toEqual({
      file: "main.md",
      startLine: 1,
      startColumn: 1,
      endLine: 1,
      endColumn: 6
    });
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：`npm test -- tests/front-end/parseMarkdown.test.ts`

预期：FAIL，报错包含 `parseMarkdown` 未导出。

- [ ] **步骤 3：实现 Markdown 解析和 span 提取**

创建 `src/markdown/parseMarkdown.ts`：

```ts
import { unified } from "unified";
import remarkParse from "remark-parse";

export type MarkdownNode = {
  type: string;
  children?: MarkdownNode[];
  value?: string;
  depth?: number;
  lang?: string;
  url?: string;
  position?: {
    start: { line: number; column: number };
    end: { line: number; column: number };
  };
};

export type MarkdownRoot = MarkdownNode & {
  type: "root";
  children: MarkdownNode[];
};

export function parseMarkdown(source: string): MarkdownRoot {
  return unified().use(remarkParse).parse(source) as MarkdownRoot;
}
```

创建 `src/front-end/sourceMap.ts`：

```ts
import type { MarkdownNode } from "../markdown/parseMarkdown.js";
import type { SourceSpan } from "../model.js";

export function spanFromNode(file: string, node: MarkdownNode): SourceSpan {
  const position = node.position;
  if (!position) {
    return { file, startLine: 1, startColumn: 1, endLine: 1, endColumn: 1 };
  }

  return {
    file,
    startLine: position.start.line,
    startColumn: position.start.column,
    endLine: position.end.line,
    endColumn: position.end.column
  };
}
```

修改 `src/index.ts` 增加导出：

```ts
export { parseMarkdown } from "./markdown/parseMarkdown.js";
export { spanFromNode } from "./front-end/sourceMap.js";
```

- [ ] **步骤 4：运行测试验证通过**

运行：`npm test -- tests/front-end/parseMarkdown.test.ts`

预期：PASS。

- [ ] **步骤 5：类型检查**

运行：`npm run typecheck`

预期：PASS。

## 任务 4：实现六级标题语义和符号名提取

**文件：**
- 创建：`src/front-end/headings.ts`
- 修改：`src/index.ts`
- 修改：`tests/front-end/headings.test.ts`

- [ ] **步骤 1：扩展失败测试**

替换 `tests/front-end/headings.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import { headingRoleForDepth, parseHeadingText } from "../../src/index";

describe("headingRoleForDepth", () => {
  it("maps six markdown heading levels to Tangle roles", () => {
    expect(headingRoleForDepth(1)).toBe("program");
    expect(headingRoleForDepth(2)).toBe("section");
    expect(headingRoleForDepth(3)).toBe("type");
    expect(headingRoleForDepth(4)).toBe("callable");
    expect(headingRoleForDepth(5)).toBe("semantic-section");
    expect(headingRoleForDepth(6)).toBe("semantic-atom");
  });
});

describe("parseHeadingText", () => {
  it("extracts a stable internal symbol from a trailing parenthesized identifier", () => {
    expect(parseHeadingText("发送通知 (send_notification)")).toEqual({
      title: "发送通知",
      symbolName: "send_notification"
    });
  });

  it("keeps the full text as title when no internal symbol exists", () => {
    expect(parseHeadingText("用户中心")).toEqual({
      title: "用户中心",
      symbolName: undefined
    });
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：`npm test -- tests/front-end/headings.test.ts`

预期：FAIL，报错包含 `parseHeadingText` 未导出。

- [ ] **步骤 3：实现标题工具**

创建 `src/front-end/headings.ts`：

```ts
import type { HeadingRole } from "../model.js";

export function headingRoleForDepth(depth: number): HeadingRole {
  switch (depth) {
    case 1:
      return "program";
    case 2:
      return "section";
    case 3:
      return "type";
    case 4:
      return "callable";
    case 5:
      return "semantic-section";
    case 6:
      return "semantic-atom";
    default:
      throw new Error(`Invalid Markdown heading depth: ${depth}`);
  }
}

export function parseHeadingText(text: string): { title: string; symbolName?: string } {
  const match = text.match(/^(.*?)\s+\(([A-Za-z_][A-Za-z0-9_]*)\)\s*$/);
  if (!match) {
    return { title: text.trim(), symbolName: undefined };
  }

  return {
    title: match[1].trim(),
    symbolName: match[2]
  };
}
```

修改 `src/index.ts`：删除本文件内的 `headingRoleForDepth` 实现，改为导出模块函数。

```ts
export { headingRoleForDepth, parseHeadingText } from "./front-end/headings.js";
```

- [ ] **步骤 4：运行测试验证通过**

运行：`npm test -- tests/front-end/headings.test.ts`

预期：PASS。

- [ ] **步骤 5：类型检查**

运行：`npm run typecheck`

预期：PASS。

## 任务 5：解析指令并执行位置纪律

**文件：**
- 创建：`src/front-end/directives.ts`
- 修改：`src/index.ts`
- 创建：`tests/front-end/directives.test.ts`

- [ ] **步骤 1：编写指令测试**

创建 `tests/front-end/directives.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import { parseDirectiveLine } from "../../src/index";

describe("parseDirectiveLine", () => {
  it("parses simple directives", () => {
    const directive = parseDirectiveLine("@export", {
      file: "main.md",
      startLine: 2,
      startColumn: 1,
      endLine: 2,
      endColumn: 8
    });

    expect(directive).toMatchObject({ kind: "export", raw: "@export" });
  });

  it("parses error directives with names and args", () => {
    const directive = parseDirectiveLine("@error PayFailed(\"支付失败\", code: Int)", {
      file: "pay.md",
      startLine: 3,
      startColumn: 1,
      endLine: 3,
      endColumn: 39
    });

    expect(directive).toMatchObject({
      kind: "error",
      name: "PayFailed",
      args: "\"支付失败\", code: Int"
    });
  });

  it("rejects unknown directives", () => {
    expect(() =>
      parseDirectiveLine("@unknown", {
        file: "main.md",
        startLine: 1,
        startColumn: 1,
        endLine: 1,
        endColumn: 9
      })
    ).toThrow("Unknown Tangle directive");
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：`npm test -- tests/front-end/directives.test.ts`

预期：FAIL，报错包含 `parseDirectiveLine` 未导出。

- [ ] **步骤 3：实现指令解析**

创建 `src/front-end/directives.ts`：

```ts
import type { DirectiveKind, SourceSpan, TangleDirective } from "../model.js";

const SIMPLE_DIRECTIVES: Record<string, DirectiveKind> = {
  "@export": "export",
  "@entry": "entry",
  "@hideCode": "hideCode",
  "@rule.table": "rule.table",
  "@rule.tree": "rule.tree",
  "@rule.toggle": "rule.toggle",
  "@rule.flow": "rule.flow"
};

export function parseDirectiveLine(rawLine: string, span: SourceSpan): TangleDirective {
  const raw = rawLine.trim();

  const simple = SIMPLE_DIRECTIVES[raw];
  if (simple) {
    return { kind: simple, raw, span };
  }

  const deprecated = raw.match(/^@deprecated\((.*)\)$/);
  if (deprecated) {
    return { kind: "deprecated", raw, args: deprecated[1], span };
  }

  const test = raw.match(/^@test\((.*)\)$/);
  if (test) {
    return { kind: "test", raw, args: test[1], span };
  }

  const error = raw.match(/^@error\s+([A-Za-z_][A-Za-z0-9_]*)(?:\((.*)\))?$/);
  if (error) {
    return { kind: "error", raw, name: error[1], args: error[2], span };
  }

  throw new Error(`Unknown Tangle directive: ${raw}`);
}
```

修改 `src/index.ts` 增加导出：

```ts
export { parseDirectiveLine } from "./front-end/directives.js";
```

- [ ] **步骤 4：运行测试验证通过**

运行：`npm test -- tests/front-end/directives.test.ts`

预期：PASS。

- [ ] **步骤 5：类型检查**

运行：`npm run typecheck`

预期：PASS。

## 任务 6：提取标题下方块、参数、导入和 `@tangle` 代码块

**文件：**
- 创建：`src/front-end/blocks.ts`
- 修改：`src/index.ts`
- 创建：`tests/front-end/blocks.test.ts`

- [ ] **步骤 1：编写块解析测试**

创建 `tests/front-end/blocks.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import { parseMarkdown, collectLinks, parseParamItem, isTangleCodeBlock } from "../../src/index";

describe("front-end blocks", () => {
  it("collects markdown links as imports", () => {
    const tree = parseMarkdown("## 依赖\n\n[Math](./math.md)\n");
    const imports = collectLinks("main.md", tree);
    expect(imports).toEqual([
      expect.objectContaining({ alias: "Math", target: "./math.md" })
    ]);
  });

  it("parses list items as named params", () => {
    const param = parseParamItem("`email`: 邮箱 (String)", {
      file: "user.md",
      startLine: 2,
      startColumn: 1,
      endLine: 2,
      endColumn: 28
    });

    expect(param).toMatchObject({
      name: "email",
      description: "邮箱",
      typeName: "String"
    });
  });

  it("recognizes @tangle code blocks", () => {
    expect(isTangleCodeBlock({ type: "code", lang: "@tangle", value: "return 1" })).toBe(true);
    expect(isTangleCodeBlock({ type: "code", lang: "ts", value: "return 1" })).toBe(false);
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：`npm test -- tests/front-end/blocks.test.ts`

预期：FAIL，报错包含 `collectLinks` 未导出。

- [ ] **步骤 3：实现块解析工具**

创建 `src/front-end/blocks.ts`：

```ts
import type { MarkdownNode, MarkdownRoot } from "../markdown/parseMarkdown.js";
import type { SourceSpan, TangleImport, TangleParam } from "../model.js";
import { spanFromNode } from "./sourceMap.js";

export function collectLinks(file: string, root: MarkdownRoot): TangleImport[] {
  const imports: TangleImport[] = [];

  function walk(node: MarkdownNode): void {
    if (node.type === "link" && node.url) {
      const alias = plainText(node).trim();
      if (alias && node.url.endsWith(".md")) {
        imports.push({ alias, target: node.url, span: spanFromNode(file, node) });
      }
    }

    for (const child of node.children ?? []) {
      walk(child);
    }
  }

  walk(root);
  return imports;
}

export function parseParamItem(text: string, span: SourceSpan): TangleParam {
  const match = text.match(/^`([^`]+)`:\s*(.*?)(?:\s+\(([^)]+)\))?$/);
  if (!match) {
    throw new Error(`Invalid Tangle parameter item: ${text}`);
  }

  return {
    name: match[1],
    description: match[2].trim(),
    typeName: match[3],
    span
  };
}

export function isTangleCodeBlock(node: Pick<MarkdownNode, "type" | "lang">): boolean {
  return node.type === "code" && node.lang === "@tangle";
}

export function plainText(node: MarkdownNode): string {
  if (typeof node.value === "string") {
    return node.value;
  }

  return (node.children ?? []).map(plainText).join("");
}
```

修改 `src/index.ts` 增加导出：

```ts
export { collectLinks, isTangleCodeBlock, parseParamItem, plainText } from "./front-end/blocks.js";
```

- [ ] **步骤 4：运行测试验证通过**

运行：`npm test -- tests/front-end/blocks.test.ts`

预期：PASS。

- [ ] **步骤 5：类型检查**

运行：`npm run typecheck`

预期：PASS。

## 任务 7：编译 Markdown 文件为 TangleModule

**文件：**
- 创建：`src/front-end/compileModule.ts`
- 修改：`src/index.ts`
- 创建：`tests/fixtures.ts`
- 创建：`tests/front-end/compileModule.test.ts`

- [ ] **步骤 1：创建测试夹具**

创建 `tests/fixtures.ts`：

```ts
export const USER_MODULE = `# 用户中心

## 依赖

[Notify](./notify.md)

### User
@export
* \`id\`: 用户 ID (Int)
* \`email\`: 邮箱 (String)

#### User -> 激活 (activate)
@export

\`\`\`@tangle
return this with { is_active: true }
\`\`\`

##### 前置条件

###### 邮箱存在
> email must not be empty
`;
```

- [ ] **步骤 2：编写端到端编译测试**

创建 `tests/front-end/compileModule.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import { compileModule } from "../../src/index";
import { USER_MODULE } from "../fixtures";

describe("compileModule", () => {
  it("builds a TangleModule with imports, headings, params, code blocks, and symbols", () => {
    const mod = compileModule({ file: "user.md", source: USER_MODULE });

    expect(mod.moduleName).toBe("user");
    expect(mod.imports).toEqual([
      expect.objectContaining({ alias: "Notify", target: "./notify.md" })
    ]);

    expect(mod.headings.map((heading) => [heading.depth, heading.role, heading.title])).toEqual([
      [1, "program", "用户中心"],
      [2, "section", "依赖"],
      [3, "type", "User"],
      [4, "callable", "User -> 激活"],
      [5, "semantic-section", "前置条件"],
      [6, "semantic-atom", "邮箱存在"]
    ]);

    expect(mod.symbols).toEqual([
      expect.objectContaining({ name: "User", kind: "type", exported: true }),
      expect.objectContaining({ name: "activate", kind: "callable", exported: true }),
      expect.objectContaining({ name: "前置条件", kind: "semantic-internal", exported: false }),
      expect.objectContaining({ name: "邮箱存在", kind: "semantic-internal", exported: false })
    ]);

    const userHeading = mod.headings.find((heading) => heading.title === "User");
    expect(userHeading?.params).toEqual([
      expect.objectContaining({ name: "id", typeName: "Int" }),
      expect.objectContaining({ name: "email", typeName: "String" })
    ]);

    const callable = mod.headings.find((heading) => heading.symbolName === "activate");
    expect(callable?.codeBlocks).toEqual([
      expect.objectContaining({ language: "tangle", value: "return this with { is_active: true }" })
    ]);
  });
});
```

- [ ] **步骤 3：运行测试验证失败**

运行：`npm test -- tests/front-end/compileModule.test.ts`

预期：FAIL，报错包含 `compileModule` 未导出。

- [ ] **步骤 4：实现模块编译**

创建 `src/front-end/compileModule.ts`：

```ts
import type { MarkdownNode } from "../markdown/parseMarkdown.js";
import type {
  TangleCodeBlock,
  TangleDiagnostic,
  TangleDirective,
  TangleHeading,
  TangleModule,
  TangleParam,
  TangleSymbol
} from "../model.js";
import { parseMarkdown } from "../markdown/parseMarkdown.js";
import { collectLinks, isTangleCodeBlock, parseParamItem, plainText } from "./blocks.js";
import { parseDirectiveLine } from "./directives.js";
import { headingRoleForDepth, parseHeadingText } from "./headings.js";
import { spanFromNode } from "./sourceMap.js";

export type CompileModuleInput = {
  file: string;
  source: string;
};

export function compileModule(input: CompileModuleInput): TangleModule {
  const root = parseMarkdown(input.source);
  const headings: TangleHeading[] = [];
  const diagnostics: TangleDiagnostic[] = [];

  const children = root.children ?? [];
  for (let index = 0; index < children.length; index += 1) {
    const node = children[index];
    if (node.type !== "heading" || typeof node.depth !== "number") {
      continue;
    }

    const nextHeadingIndex = findNextHeadingIndex(children, index + 1);
    const body = children.slice(index + 1, nextHeadingIndex);
    headings.push(buildHeading(input.file, node, body, diagnostics));
  }

  return {
    file: input.file,
    moduleName: moduleNameFromFile(input.file),
    imports: collectLinks(input.file, root),
    headings,
    symbols: buildSymbols(headings),
    diagnostics
  };
}

function buildHeading(
  file: string,
  node: MarkdownNode,
  body: MarkdownNode[],
  diagnostics: TangleDiagnostic[]
): TangleHeading {
  const parsed = parseHeadingText(plainText(node));
  const directives: TangleDirective[] = [];
  const params: TangleParam[] = [];
  const codeBlocks: TangleCodeBlock[] = [];

  for (const child of body) {
    if (child.type === "paragraph") {
      const text = plainText(child).trim();
      if (text.startsWith("@")) {
        try {
          directives.push(parseDirectiveLine(text, spanFromNode(file, child)));
        } catch (error) {
          diagnostics.push({
            code: "TANGLE_UNKNOWN_DIRECTIVE",
            message: error instanceof Error ? error.message : String(error),
            span: spanFromNode(file, child)
          });
        }
      }
    }

    if (child.type === "list") {
      for (const item of child.children ?? []) {
        const text = plainText(item).trim();
        if (text.startsWith("`")) {
          params.push(parseParamItem(text, spanFromNode(file, item)));
        }
      }
    }

    if (isTangleCodeBlock(child)) {
      codeBlocks.push({
        language: "tangle",
        value: (child.value ?? "").trim(),
        span: spanFromNode(file, child)
      });
    }
  }

  return {
    id: stableHeadingId(parsed.symbolName ?? parsed.title),
    depth: node.depth ?? 1,
    role: headingRoleForDepth(node.depth ?? 1),
    title: parsed.title,
    symbolName: parsed.symbolName,
    directives,
    params,
    codeBlocks,
    span: spanFromNode(file, node),
    children: []
  };
}

function buildSymbols(headings: TangleHeading[]): TangleSymbol[] {
  return headings.map((heading) => {
    const exported = heading.directives.some((directive) => directive.kind === "export" || directive.kind === "entry");
    const name = heading.symbolName ?? heading.title;

    if (heading.role === "type") {
      return { name, kind: "type", exported, headingId: heading.id, span: heading.span };
    }

    if (heading.role === "callable") {
      return { name, kind: "callable", exported, headingId: heading.id, span: heading.span };
    }

    if (heading.role === "program" && heading.directives.some((directive) => directive.kind === "entry")) {
      return { name, kind: "entry", exported: true, headingId: heading.id, span: heading.span };
    }

    return { name, kind: "semantic-internal", exported: false, headingId: heading.id, span: heading.span };
  });
}

function findNextHeadingIndex(nodes: MarkdownNode[], start: number): number {
  const next = nodes.findIndex((node, offset) => offset >= start && node.type === "heading");
  return next === -1 ? nodes.length : next;
}

function moduleNameFromFile(file: string): string {
  return file.replace(/\\/g, "/").split("/").pop()?.replace(/\.md$/, "") ?? file;
}

function stableHeadingId(text: string): string {
  return text
    .trim()
    .toLowerCase()
    .replace(/\s+/g, "-")
    .replace(/[^\p{L}\p{N}_-]/gu, "");
}
```

修改 `src/index.ts` 增加导出：

```ts
export { compileModule } from "./front-end/compileModule.js";
export type { CompileModuleInput } from "./front-end/compileModule.js";
```

- [ ] **步骤 5：运行测试验证通过**

运行：`npm test -- tests/front-end/compileModule.test.ts`

预期：PASS。

- [ ] **步骤 6：运行全量测试和类型检查**

运行：`npm test`

预期：PASS。

运行：`npm run typecheck`

预期：PASS。

## 任务 8：实现指令位置纪律诊断

**文件：**
- 修改：`src/front-end/compileModule.ts`
- 修改：`tests/front-end/directives.test.ts`

- [ ] **步骤 1：添加失败测试**

追加到 `tests/front-end/directives.test.ts`：

```ts
import { compileModule } from "../../src/index";

describe("directive placement", () => {
  it("reports directives embedded in ordinary paragraphs", () => {
    const mod = compileModule({
      file: "bad.md",
      source: `# Bad

这是一段普通说明，里面出现 @export 是非法的。
`
    });

    expect(mod.diagnostics).toEqual([
      expect.objectContaining({
        code: "TANGLE_INVALID_DIRECTIVE_POSITION",
        message: "Tangle directives must appear directly under a heading or directly above their target block"
      })
    ]);
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：`npm test -- tests/front-end/directives.test.ts`

预期：FAIL，诊断数组为空。

- [ ] **步骤 3：实现普通段落指令诊断**

在 `src/front-end/compileModule.ts` 的 `buildHeading` 循环中，替换 paragraph 分支为：

```ts
if (child.type === "paragraph") {
  const text = plainText(child).trim();
  if (text.startsWith("@")) {
    try {
      directives.push(parseDirectiveLine(text, spanFromNode(file, child)));
    } catch (error) {
      diagnostics.push({
        code: "TANGLE_UNKNOWN_DIRECTIVE",
        message: error instanceof Error ? error.message : String(error),
        span: spanFromNode(file, child)
      });
    }
  } else if (/\s@[A-Za-z]/.test(text)) {
    diagnostics.push({
      code: "TANGLE_INVALID_DIRECTIVE_POSITION",
      message: "Tangle directives must appear directly under a heading or directly above their target block",
      span: spanFromNode(file, child)
    });
  }
}
```

- [ ] **步骤 4：运行测试验证通过**

运行：`npm test -- tests/front-end/directives.test.ts`

预期：PASS。

- [ ] **步骤 5：运行全量验证**

运行：`npm test`

预期：PASS。

运行：`npm run typecheck`

预期：PASS。

## 任务 9：加入入口唯一性和导出层级诊断

**文件：**
- 修改：`src/front-end/compileModule.ts`
- 创建：`tests/front-end/symbols.test.ts`

- [ ] **步骤 1：编写失败测试**

创建 `tests/front-end/symbols.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import { compileModule } from "../../src/index";

describe("symbol diagnostics", () => {
  it("allows exactly one @entry", () => {
    const mod = compileModule({
      file: "main.md",
      source: `# App
@entry

#### Start (start)
@entry
`
    });

    expect(mod.diagnostics).toEqual([
      expect.objectContaining({ code: "TANGLE_DUPLICATE_ENTRY" })
    ]);
  });

  it("rejects exported semantic micro headings", () => {
    const mod = compileModule({
      file: "user.md",
      source: `# User Module

##### 前置条件
@export
`
    });

    expect(mod.diagnostics).toEqual([
      expect.objectContaining({ code: "TANGLE_INVALID_EXPORT_LEVEL" })
    ]);
  });
});
```

- [ ] **步骤 2：运行测试验证失败**

运行：`npm test -- tests/front-end/symbols.test.ts`

预期：FAIL，诊断数组为空。

- [ ] **步骤 3：实现符号诊断**

在 `src/front-end/compileModule.ts` 的 `compileModule` 中，构建 `symbols` 后追加验证：

```ts
  const symbols = buildSymbols(headings);
  validateSymbolRules(headings, diagnostics);

  return {
    file: input.file,
    moduleName: moduleNameFromFile(input.file),
    imports: collectLinks(input.file, root),
    headings,
    symbols,
    diagnostics
  };
```

在同文件追加函数：

```ts
function validateSymbolRules(headings: TangleHeading[], diagnostics: TangleDiagnostic[]): void {
  const entryHeadings = headings.filter((heading) =>
    heading.directives.some((directive) => directive.kind === "entry")
  );

  if (entryHeadings.length > 1) {
    diagnostics.push({
      code: "TANGLE_DUPLICATE_ENTRY",
      message: "A Tangle program must declare exactly one @entry",
      span: entryHeadings[1].span
    });
  }

  for (const heading of headings) {
    const exported = heading.directives.some((directive) => directive.kind === "export");
    const exportable = heading.role === "type" || heading.role === "callable";
    if (exported && !exportable) {
      diagnostics.push({
        code: "TANGLE_INVALID_EXPORT_LEVEL",
        message: "@export is only valid on type and callable headings",
        span: heading.span
      });
    }
  }
}
```

- [ ] **步骤 4：运行测试验证通过**

运行：`npm test -- tests/front-end/symbols.test.ts`

预期：PASS。

- [ ] **步骤 5：运行全量验证**

运行：`npm test`

预期：PASS。

运行：`npm run typecheck`

预期：PASS。

## 任务 10：导出稳定 API 并补 README 用法片段

**文件：**
- 修改：`src/index.ts`
- 创建：`README.md`
- 创建：`tests/front-end/public-api.test.ts`

- [ ] **步骤 1：编写 public API 测试**

创建 `tests/front-end/public-api.test.ts`：

```ts
import { describe, expect, it } from "vitest";
import {
  collectLinks,
  compileModule,
  headingRoleForDepth,
  parseDirectiveLine,
  parseHeadingText,
  parseMarkdown,
  parseParamItem,
  spanFromNode
} from "../../src/index";

describe("public API", () => {
  it("exports the Track A1 frontend API", () => {
    expect(typeof collectLinks).toBe("function");
    expect(typeof compileModule).toBe("function");
    expect(typeof headingRoleForDepth).toBe("function");
    expect(typeof parseDirectiveLine).toBe("function");
    expect(typeof parseHeadingText).toBe("function");
    expect(typeof parseMarkdown).toBe("function");
    expect(typeof parseParamItem).toBe("function");
    expect(typeof spanFromNode).toBe("function");
  });
});
```

- [ ] **步骤 2：运行测试验证失败或通过**

运行：`npm test -- tests/front-end/public-api.test.ts`

预期：若前面任务遗漏导出则 FAIL；补齐后 PASS。

- [ ] **步骤 3：整理 `src/index.ts` 导出**

确保 `src/index.ts` 内容为：

```ts
export type {
  DirectiveKind,
  HeadingRole,
  SourceSpan,
  SymbolKind,
  TangleCodeBlock,
  TangleDiagnostic,
  TangleDirective,
  TangleHeading,
  TangleImport,
  TangleModule,
  TangleParam,
  TangleSymbol
} from "./model.js";

export { collectLinks, isTangleCodeBlock, parseParamItem, plainText } from "./front-end/blocks.js";
export { compileModule } from "./front-end/compileModule.js";
export type { CompileModuleInput } from "./front-end/compileModule.js";
export { parseDirectiveLine } from "./front-end/directives.js";
export { headingRoleForDepth, parseHeadingText } from "./front-end/headings.js";
export { spanFromNode } from "./front-end/sourceMap.js";
export { parseMarkdown } from "./markdown/parseMarkdown.js";
```

- [ ] **步骤 4：创建 README 用法片段**

创建 `README.md`：

````markdown
# Tangle

Tangle is a Markdown-native programming language prototype.

## Track A1 frontend

```ts
import { compileModule } from "./src/index.js";

const mod = compileModule({
  file: "user.md",
  source: `# 用户中心

### User
@export
* \`email\`: 邮箱 (String)

#### User -> 激活 (activate)
@export

\`\`\`@tangle
return this with { is_active: true }
\`\`\`
`
});

console.log(mod.symbols);
```
````

- [ ] **步骤 5：运行最终验证**

运行：`npm test`

预期：PASS。

运行：`npm run typecheck`

预期：PASS。

## 计划自检清单

- 规格覆盖：本计划覆盖 Track A1 的 Markdown 解析、六级标题语义、指令、参数、导入、`@tangle` 代码块、符号表和关键诊断。
- 明确排除：完整类型推导、Rule Graph、错误传播语义执行、codegen、CLI 和标准库实现不在 Track A1 计划内。
- 占位符扫描：计划中的每个步骤都包含实际文件、代码片段、命令和预期结果。
- 类型一致性：`HeadingRole`、`TangleModule`、`TangleHeading`、`TangleDirective`、`TangleSymbol` 在各任务中保持同名同义。
- 验证命令：每个任务至少运行相关 Vitest 文件，最终运行 `npm test` 和 `npm run typecheck`。
