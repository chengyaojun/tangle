/// Host-specific standard library bindings.
/// Each function returns the code snippet that replaces a stdlib call in the target language.

pub enum TargetHost { JavaScript, Python, Go }

pub fn stdlib_prelude(host: TargetHost) -> &'static str {
    match host {
        TargetHost::JavaScript => JS_STDLIB_PRELUDE,
        TargetHost::Python => PY_STDLIB_PRELUDE,
        TargetHost::Go => GO_STDLIB_PRELUDE,
    }
}

pub const ALL_STDLIB_MODULES: &[&str] = &[
    "List", "Map", "Option", "String", "JSON", "HTTP", "IO", "Math", "DateTime", "Regex", "Crypto"
];

const JS_STDLIB_PRELUDE: &str = r#"
// --- Tangle Standard Library (JS) ---
const List = (items) => ({ items, length: items.length, map: (fn) => List(items.map(fn)), filter: (fn) => List(items.filter(fn)) });
const Map = (entries) => ({ entries: new Map(entries), get: (k) => entries.get(k), set: (k,v) => Map(entries.set(k,v)), has: (k) => entries.has(k) });
const Option = { Some: (v) => ({ is_some: true, value: v }), None: { is_some: false } };
const JSON_lib = { parse: (s) => JSON.parse(s), stringify: (v) => JSON.stringify(v) };
const Math_lib = { abs: Math.abs, min: Math.min, max: Math.max, floor: Math.floor, ceil: Math.ceil, sqrt: Math.sqrt, pow: Math.pow };
const DateTime = { now: () => new Date(), format: (d,f) => d.toISOString(), timestamp: (d) => d.getTime() };
"#;

const PY_STDLIB_PRELUDE: &str = r#"
# --- Tangle Standard Library (Python) ---
import json, math, hashlib, hmac, re, datetime as dt, os
from typing import Any, TypeVar, Generic

class List:
    def __init__(self, items): self.items = list(items)
    def length(self): return len(self.items)
    def map(self, fn): return List([fn(x) for x in self.items])
    def filter(self, fn): return List([x for x in self.items if fn(x)])

class Option:
    @staticmethod
    def Some(v): return {'is_some': True, 'value': v}
    None_ = {'is_some': False}

class JSON_lib:
    @staticmethod
    def parse(s): return json.loads(s)
    @staticmethod
    def stringify(v): return json.dumps(v)

class DateTime:
    @staticmethod
    def now(): return dt.datetime.now()
    @staticmethod
    def format(d, f): return d.strftime(f)
    @staticmethod
    def timestamp(d): return d.timestamp()
"#;

const GO_STDLIB_PRELUDE: &str = r#"
// --- Tangle Standard Library (Go) ---
import (
    "crypto/hmac"
    "crypto/md5"
    "crypto/sha1"
    "crypto/sha256"
    "crypto/sha512"
    "encoding/json"
    "fmt"
    "math"
    "regexp"
    "time"
)

// List type
type List[T any] struct { Items []T }
func NewList[T any](items []T) List[T] { return List[T]{Items: items} }
func (l List[T]) Length() int { return len(l.Items) }
func (l List[T]) Get(i int) T { return l.Items[i] }

// Option type
type Option[T any] struct { IsSome bool; Value T }
func Some[T any](v T) Option[T] { return Option[T]{IsSome: true, Value: v} }
func None[T any]() Option[T] { return Option[T]{IsSome: false} }

// DateTime helpers
func DateTimeNow() time.Time { return time.Now() }
func DateTimeTimestamp(t time.Time) int64 { return t.Unix() }
"#;
