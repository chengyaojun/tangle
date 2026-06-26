// Tangle Standard Library — JS Runtime
// These are the runtime helpers that Tangle programs depend on at execution time.

export function __tangle_struct<T extends Record<string, unknown>>(fields: T): Readonly<T> {
  return Object.freeze({ ...fields });
}

export function __tangle_with<T extends Record<string, unknown>>(obj: Readonly<T>, updates: Partial<T>): Readonly<T> {
  return Object.freeze({ ...obj, ...updates });
}

export type TangleResult<T, E = string> =
  | { ok: true; value: T }
  | { ok: false; error: { variant: E; value: unknown } };

export function Ok<T>(value: T): TangleResult<T, never> {
  return { ok: true, value };
}

export function Err<E extends string>(variant: E, value?: unknown): TangleResult<never, E> {
  return { ok: false, error: { variant, value } };
}

export function __tangle_propagate<T, E extends string>(result: TangleResult<T, E>): T {
  if (!result.ok) throw result;
  return result.value;
}

export function __tangle_match<T, R>(
  result: TangleResult<T, string>,
  handlers: Record<string, (value: unknown) => R>
): R {
  if (result.ok) {
    if (handlers._) return handlers._(result.value);
    throw new Error("Non-exhaustive match: unexpected Ok value");
  }
  const h = handlers[result.error.variant];
  if (h) return h(result.error.value);
  if (handlers._) return handlers._(result);
  throw new Error(`Non-exhaustive match: missing variant '${result.error.variant}'`);
}

// List runtime
export function __tangle_list<T>(items: T[]): T[] {
  return [...items];
}

// Map runtime
export function __tangle_map<K extends string | number | symbol, V>(entries?: [K, V][]): Record<K, V> {
  const m = {} as Record<K, V>;
  if (entries) for (const [k, v] of entries) m[k] = v;
  return m;
}

// Option runtime
export type Option<T> = { kind: "some"; value: T } | { kind: "none" };
export function Some<T>(value: T): Option<T> { return { kind: "some", value }; }
export const None: Option<never> = { kind: "none" };
