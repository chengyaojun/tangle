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
