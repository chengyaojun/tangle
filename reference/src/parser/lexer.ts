import type { SourceSpan } from "../model.js";

export type TokenKind =
  | "number"
  | "string"
  | "true"
  | "false"
  | "identifier"
  | "return"
  | "let"
  | "const"
  | "if"
  | "else"
  | "this"
  | "pipeOp"
  | "dot"
  | "comma"
  | "colon"
  | "semicolon"
  | "lparen"
  | "rparen"
  | "lbrace"
  | "rbrace"
  | "lbracket"
  | "rbracket"
  | "plus"
  | "minus"
  | "star"
  | "slash"
  | "percent"
  | "eq"
  | "eqeq"
  | "neq"
  | "lt"
  | "gt"
  | "lte"
  | "gte"
  | "and"
  | "or"
  | "bang"
  | "pipe"
  | "arrow"
  | "fatArrow"
  | "error"
  | "question"
  | "eof";

export type Token = {
  kind: TokenKind;
  value: string;
  span: SourceSpan;
};

const KEYWORDS: Record<string, TokenKind> = {
  return: "return",
  let: "let",
  const: "const",
  if: "if",
  else: "else",
  this: "this",
  true: "true",
  false: "false",
};

const MULTI_CHAR_OPS: Record<string, Record<string, TokenKind>> = {
  "=": { "=": "eqeq" },
  "!": { "=": "neq" },
  "<": { "=": "lte" },
  ">": { "=": "gte" },
  "&": { "&": "and" },
  "|": { "|": "or", ">": "pipeOp" },
  "-": { ">": "fatArrow" },
};

function isDigit(ch: string): boolean {
  return ch >= "0" && ch <= "9";
}

function isIdentifierStart(ch: string): boolean {
  return (ch >= "a" && ch <= "z") || (ch >= "A" && ch <= "Z") || ch === "_";
}

function isIdentifierPart(ch: string): boolean {
  return isIdentifierStart(ch) || isDigit(ch);
}

export function tokenize(source: string, file: string): Token[] {
  const tokens: Token[] = [];
  let pos = 0;
  let line = 1;
  let column = 1;

  function pushToken(
    kind: TokenKind,
    value: string,
    sl: number,
    sc: number,
    el: number,
    ec: number,
  ): void {
    tokens.push({
      kind,
      value,
      span: { file, startLine: sl, startColumn: sc, endLine: el, endColumn: ec },
    });
  }

  function peek(offset: number): string | undefined {
    const idx = pos + offset;
    return idx < source.length ? source[idx] : undefined;
  }

  while (pos < source.length) {
    const startLine = line;
    const startColumn = column;
    const ch = source[pos]!;

    // ── Whitespace ──────────────────────────────────────
    if (ch === " " || ch === "\t" || ch === "\r") {
      pos++;
      column++;
      continue;
    }
    if (ch === "\n") {
      pos++;
      line++;
      column = 1;
      continue;
    }

    // ── Numbers (integer and decimal) ───────────────────
    if (isDigit(ch)) {
      let value = "";
      while (pos < source.length && isDigit(source[pos]!)) {
        value += source[pos];
        pos++;
        column++;
      }
      if (
        peek(0) === "." &&
        peek(1) !== undefined &&
        isDigit(peek(1)!)
      ) {
        value += ".";
        pos++;
        column++;
        while (pos < source.length && isDigit(source[pos]!)) {
          value += source[pos];
          pos++;
          column++;
        }
      }
      pushToken("number", value, startLine, startColumn, line, column);
      continue;
    }

    // ── Strings (double-quoted) ─────────────────────────
    if (ch === '"') {
      let value = '"';
      pos++; // skip opening "
      column++;
      while (pos < source.length && source[pos] !== '"') {
        if (source[pos] === "\n") {
          line++;
          column = 1;
        } else {
          column++;
        }
        value += source[pos];
        pos++;
      }
      if (pos < source.length && source[pos] === '"') {
        value += '"';
        pos++; // skip closing "
        column++;
        pushToken("string", value, startLine, startColumn, line, column);
      } else {
        pushToken("error", "Unterminated string literal", startLine, startColumn, line, column);
      }
      continue;
    }

    // ── Identifiers / Keywords ──────────────────────────
    if (isIdentifierStart(ch)) {
      let value = "";
      while (pos < source.length && isIdentifierPart(source[pos]!)) {
        value += source[pos];
        pos++;
        column++;
      }
      const kind: TokenKind = KEYWORDS[value] ?? "identifier";
      pushToken(kind, value, startLine, startColumn, line, column);
      continue;
    }

    // ── Multi-character operators (checked before single) ──
    const next = peek(1);
    if (next !== undefined) {
      const multiKind: TokenKind | undefined = MULTI_CHAR_OPS[ch]?.[next];
      if (multiKind !== undefined) {
        pushToken(multiKind, ch + next, startLine, startColumn, line, column + 2);
        pos += 2;
        column += 2;
        continue;
      }
    }

    // ── Single-character operators and delimiters ────────
    const singleKind: TokenKind | undefined = ((): TokenKind | undefined => {
      switch (ch) {
        case "+": return "plus";
        case "-": return "minus";
        case "*": return "star";
        case "/": return "slash";
        case "%": return "percent";
        case "=": return "eq";
        case "!": return "bang";
        case "<": return "lt";
        case ">": return "gt";
        case "|": return "pipe";
        case ".": return "dot";
        case ",": return "comma";
        case ":": return "colon";
        case ";": return "semicolon";
        case "(": return "lparen";
        case ")": return "rparen";
        case "{": return "lbrace";
        case "}": return "rbrace";
        case "[": return "lbracket";
        case "]": return "rbracket";
        case "?": return "question";
        default: return undefined;
      }
    })();

    if (singleKind !== undefined) {
      pushToken(singleKind, ch, startLine, startColumn, line, column + 1);
      pos++;
      column++;
      continue;
    }

    // ── Unknown character ────────────────────────────────
    pushToken("error", `Unexpected character: '${ch}'`, startLine, startColumn, line, column + 1);
    pos++;
    column++;
  }

  // EOF sentinel
  pushToken("eof", "", line, column, line, column);

  return tokens;
}
