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

const JS_STDLIB_PRELUDE: &str = r#"
// --- Tangle Standard Library (JS) ---
const List = (items) => ({ items, length: items.length, map: (fn) => List(items.map(fn)), filter: (fn) => List(items.filter(fn)) });
const Map = (entries) => ({ entries: new Map(entries), get: (k) => entries.get(k), set: (k,v) => Map(entries.set(k,v)), has: (k) => entries.has(k) });
const Set = (items) => ({ _set: new Set(items), add: (v) => _set.add(v) && this, remove: (v) => _set.delete(v), contains: (v) => _set.has(v), size: _set.size, to_list: () => List([..._set]) });
const Option = { Some: (v) => ({ is_some: true, value: v }), None: { is_some: false } };
const JSON_lib = { parse: (s) => JSON.parse(s), stringify: (v) => JSON.stringify(v) };
const Math_lib = { abs: Math.abs, min: Math.min, max: Math.max, floor: Math.floor, ceil: Math.ceil, sqrt: Math.sqrt, pow: Math.pow };
const DateTime = { now: () => new Date(), format: (d,f) => d.toISOString(), timestamp: (d) => d.getTime() };
const Console = { print: (...args) => process.stdout.write(args.join(' ')), println: (...args) => console.log(...args), input: (prompt) => { process.stdout.write(prompt); return ''; }, debug: (...args) => console.debug(...args), error: (...args) => console.error(...args) };
const Random = { int: () => Math.floor(Math.random() * Number.MAX_SAFE_INTEGER), int_range: (lo, hi) => Math.floor(Math.random() * (hi - lo)) + lo, float: () => Math.random(), bool: () => Math.random() < 0.5, bytes: (n) => { const buf = new Uint8Array(n); crypto.getRandomValues(buf); return Array.from(buf); }, shuffle: (arr) => { const a = [...arr]; for (let i = a.length-1; i>0; i--) { const j = Math.floor(Math.random()*(i+1)); [a[i],a[j]] = [a[j],a[i]]; } return a; }, choice: (arr) => arr[Math.floor(Math.random()*arr.length)] };
const Encoding = { hex_encode: (bytes) => Buffer.from(bytes).toString('hex'), hex_decode: (s) => Uint8Array.from(Buffer.from(s,'hex')), base64_encode: (bytes) => Buffer.from(bytes).toString('base64'), base64_decode: (s) => Buffer.from(s,'base64'), url_encode: (s) => encodeURIComponent(s), url_decode: (s) => decodeURIComponent(s) };
const Sort = { asc: (arr) => [...arr].sort((a,b) => a>b?1:a<b?-1:0), desc: (arr) => [...arr].sort((a,b) => a<b?1:a>b?-1:0), by_key_asc: (arr, fn) => [...arr].sort((a,b) => fn(a)>fn(b)?1:fn(a)<fn(b)?-1:0), by_key_desc: (arr, fn) => [...arr].sort((a,b) => fn(a)<fn(b)?1:fn(a)>fn(b)?-1:0), is_sorted: (arr) => arr.every((v,i,a) => !i || a[i-1] <= v), min: (arr) => Math.min(...arr), max: (arr) => Math.max(...arr) };
const Env = { get: (k) => (typeof process !== 'undefined' && process.env && process.env[k]) || null, set: (k,v) => { if (typeof process !== 'undefined') process.env[k] = v; }, remove: (k) => { if (typeof process !== 'undefined') delete process.env[k]; }, args: () => (typeof process !== 'undefined' && process.argv) ? process.argv.slice(2) : [], current_dir: () => (typeof process !== 'undefined') ? process.cwd() : '/', exit: (code) => { if (typeof process !== 'undefined') process.exit(code); } };
"#;

const PY_STDLIB_PRELUDE: &str = r#"
# --- Tangle Standard Library (Python) ---
import json, math, hashlib, hmac, re, datetime as dt, os, sys, random, base64, binascii, urllib.parse
from typing import Any, TypeVar, Generic

class List:
    def __init__(self, items): self.items = list(items)
    def length(self): return len(self.items)
    def map(self, fn): return List([fn(x) for x in self.items])
    def filter(self, fn): return List([x for x in self.items if fn(x)])

class Set_:
    def __init__(self, items=None): self._set = set(items) if items else set()
    def add(self, v): self._set.add(v); return self
    def remove(self, v): self._set.discard(v); return self
    def contains(self, v): return v in self._set
    def size(self): return len(self._set)
    def union(self, other): return Set_(self._set.union(other._set))
    def intersection(self, other): return Set_(self._set.intersection(other._set))
    def difference(self, other): return Set_(self._set.difference(other._set))
    def to_list(self): return List(list(self._set))

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

class Console:
    @staticmethod
    def print(*args, end=''): print(*args, end=end)
    @staticmethod
    def println(*args): print(*args)
    @staticmethod
    def input(prompt=''): return input(prompt)
    @staticmethod
    def debug(*args): print('[DEBUG]', *args)
    @staticmethod
    def error(*args): print('[ERROR]', *args, file=sys.stderr)

class Random:
    @staticmethod
    def int(): return random.randint(0, 2**63 - 1)
    @staticmethod
    def int_range(lo, hi): return random.randint(lo, hi - 1)
    @staticmethod
    def float(): return random.random()
    @staticmethod
    def bool(): return random.choice([True, False])
    @staticmethod
    def bytes(n): return random.randbytes(n)
    @staticmethod
    def shuffle(arr): a = list(arr); random.shuffle(a); return a
    @staticmethod
    def choice(arr): return random.choice(arr)

class Encoding:
    @staticmethod
    def hex_encode(data): return binascii.hexlify(bytes(data)).decode()
    @staticmethod
    def hex_decode(s): return list(binascii.unhexlify(s))
    @staticmethod
    def base64_encode(data): return base64.b64encode(bytes(data)).decode()
    @staticmethod
    def base64_decode(s): return list(base64.b64decode(s))
    @staticmethod
    def url_encode(s): return urllib.parse.quote(s)
    @staticmethod
    def url_decode(s): return urllib.parse.unquote(s)

class Sort:
    @staticmethod
    def asc(arr): return sorted(arr)
    @staticmethod
    def desc(arr): return sorted(arr, reverse=True)
    @staticmethod
    def by_key_asc(arr, fn): return sorted(arr, key=fn)
    @staticmethod
    def by_key_desc(arr, fn): return sorted(arr, key=fn, reverse=True)
    @staticmethod
    def is_sorted(arr): return all(arr[i] <= arr[i+1] for i in range(len(arr)-1))
    @staticmethod
    def min(arr): return min(arr)
    @staticmethod
    def max(arr): return max(arr)

class Env:
    @staticmethod
    def get(k, default=None): return os.environ.get(k, default)
    @staticmethod
    def set(k, v): os.environ[k] = v
    @staticmethod
    def remove(k): os.environ.pop(k, None)
    @staticmethod
    def args(): return sys.argv[1:]
    @staticmethod
    def current_dir(): return os.getcwd()
    @staticmethod
    def exit(code=0): sys.exit(code)
"#;

const GO_STDLIB_PRELUDE: &str = r#"
// --- Tangle Standard Library (Go) ---
import (
    "crypto/hmac"
    "crypto/md5"
    "crypto/sha1"
    "crypto/sha256"
    "crypto/sha512"
    "encoding/base64"
    "encoding/hex"
    "encoding/json"
    "fmt"
    "math"
    "math/rand"
    "net/url"
    "os"
    "regexp"
    "sort"
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

// Console helpers
func ConsolePrint(args ...interface{}) { fmt.Print(args...) }
func ConsolePrintln(args ...interface{}) { fmt.Println(args...) }
func ConsoleInput(prompt string) string { fmt.Print(prompt); var s string; fmt.Scan(&s); return s }
func ConsoleDebug(args ...interface{}) { fmt.Print("[DEBUG] "); fmt.Println(args...) }
func ConsoleError(args ...interface{}) { fmt.Fprintln(os.Stderr, args...) }

// Random helpers
func RandomInt() int { return rand.Int() }
func RandomIntRange(lo, hi int) int { return rand.Intn(hi-lo) + lo }
func RandomFloat() float64 { return rand.Float64() }
func RandomBool() bool { return rand.Intn(2) == 0 }
func RandomBytes(n int) []byte { b := make([]byte, n); rand.Read(b); return b }
func RandomShuffle(arr []interface{}) { rand.Shuffle(len(arr), func(i,j int) { arr[i], arr[j] = arr[j], arr[i] }) }
func RandomChoice(arr []interface{}) interface{} { return arr[rand.Intn(len(arr))] }

// Encoding helpers
func HexEncode(data []byte) string { return hex.EncodeToString(data) }
func HexDecode(s string) ([]byte, error) { return hex.DecodeString(s) }
func Base64Encode(data []byte) string { return base64.StdEncoding.EncodeToString(data) }
func Base64Decode(s string) ([]byte, error) { return base64.StdEncoding.DecodeString(s) }
func URLEncode(s string) string { return url.QueryEscape(s) }
func URLDecode(s string) (string, error) { return url.QueryUnescape(s) }

// Sort helpers
func SortAsc[T any](arr []T, less func(T, T) bool) []T { sorted := make([]T, len(arr)); copy(sorted, arr); sort.SliceStable(sorted, func(i,j int) bool { return less(sorted[i], sorted[j]) }); return sorted }
func SortDesc[T any](arr []T, less func(T, T) bool) []T { sorted := make([]T, len(arr)); copy(sorted, arr); sort.SliceStable(sorted, func(i,j int) bool { return !less(sorted[i], sorted[j]) && sorted[i] != sorted[j] }); return sorted }
func IsSorted[T any](arr []T, less func(T,T) bool) bool { return sort.SliceIsSorted(arr, func(i,j int) bool { return less(arr[i], arr[j]) }) }

// Env helpers
func EnvGet(key string) string { return os.Getenv(key) }
func EnvSet(key, value string) error { return os.Setenv(key, value) }
func EnvRemove(key string) error { return os.Unsetenv(key) }
func EnvArgs() []string { return os.Args[1:] }
func EnvCurrentDir() (string, error) { return os.Getwd() }
func EnvExit(code int) { os.Exit(code) }
"#;
