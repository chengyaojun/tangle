/// Host-specific standard library bindings.
/// Maps each stdlib module to host-native implementations for JS, Python, and Go.

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
const fmt = { print: (...args) => process.stdout.write(args.join(' ')), println: (...args) => console.log(...args), input: (prompt) => { process.stdout.write(prompt); return ''; }, debug: (...args) => console.debug(...args), error: (...args) => console.error(...args), format: (s, ...args) => require('util').format(s, ...args) };
const Random = { int: () => Math.floor(Math.random() * Number.MAX_SAFE_INTEGER), int_range: (lo, hi) => Math.floor(Math.random() * (hi - lo)) + lo, float: () => Math.random(), bool: () => Math.random() < 0.5, bytes: (n) => { const buf = new Uint8Array(n); crypto.getRandomValues(buf); return Array.from(buf); }, shuffle: (arr) => { const a = [...arr]; for (let i = a.length-1; i>0; i--) { const j = Math.floor(Math.random()*(i+1)); [a[i],a[j]] = [a[j],a[i]]; } return a; }, choice: (arr) => arr[Math.floor(Math.random()*arr.length)] };
const Encoding = { hex_encode: (bytes) => Buffer.from(bytes).toString('hex'), hex_decode: (s) => Uint8Array.from(Buffer.from(s,'hex')), base64_encode: (bytes) => Buffer.from(bytes).toString('base64'), base64_decode: (s) => Buffer.from(s,'base64'), url_encode: (s) => encodeURIComponent(s), url_decode: (s) => decodeURIComponent(s) };
const Sort = { asc: (arr) => [...arr].sort((a,b) => a>b?1:a<b?-1:0), desc: (arr) => [...arr].sort((a,b) => a<b?1:a>b?-1:0), by_key_asc: (arr, fn) => [...arr].sort((a,b) => fn(a)>fn(b)?1:fn(a)<fn(b)?-1:0), by_key_desc: (arr, fn) => [...arr].sort((a,b) => fn(a)<fn(b)?1:fn(a)>fn(b)?-1:0), is_sorted: (arr) => arr.every((v,i,a) => !i || a[i-1] <= v), min: (arr) => Math.min(...arr), max: (arr) => Math.max(...arr) };
const Env = { get: (k) => (typeof process !== 'undefined' && process.env && process.env[k]) || null, set: (k,v) => { if (typeof process !== 'undefined') process.env[k] = v; }, remove: (k) => { if (typeof process !== 'undefined') delete process.env[k]; }, args: () => (typeof process !== 'undefined' && process.argv) ? process.argv.slice(2) : [], current_dir: () => (typeof process !== 'undefined') ? process.cwd() : '/', exit: (code) => { if (typeof process !== 'undefined') process.exit(code); } };
// --- I/O & System ---
const IO = { readFile: (p) => require('fs').readFileSync(p, 'utf-8'), writeFile: (p, d) => require('fs').writeFileSync(p, d), exists: (p) => require('fs').existsSync(p), stat: (p) => require('fs').statSync(p), mkdir: (p) => require('fs').mkdirSync(p, {recursive: true}), read_dir: (p) => require('fs').readdirSync(p), remove: (p) => require('fs').rmSync(p, {recursive: true}), rename: (a,b) => require('fs').renameSync(a,b), copy: (a,b) => require('fs').copyFileSync(a,b), chmod: (p,m) => require('fs').chmodSync(p,m), size: (p) => require('fs').statSync(p).size, is_dir: (p) => require('fs').statSync(p).isDirectory(), is_file: (p) => require('fs').statSync(p).isFile() };
const Path = { join: (...parts) => require('path').join(...parts), basename: (p) => require('path').basename(p), dirname: (p) => require('path').dirname(p), extension: (p) => require('path').extname(p), is_absolute: (p) => require('path').isAbsolute(p), normalize: (p) => require('path').normalize(p), relative: (from, to) => require('path').relative(from, to), split: (p) => p.split(/[\\/]/).filter(Boolean) };
const Process = { run: (cmd, args) => require('child_process').execFileSync(cmd, args), exec: (cmd) => require('child_process').execSync(cmd), spawn: (cmd, args) => require('child_process').spawn(cmd, args, {stdio: 'inherit'}), exit: (c) => process.exit(c), pid: process.pid, args: process.argv.slice(2), stdout: process.stdout, stderr: process.stderr, status: 0 };
// --- Concurrency ---
const Task = { spawn: (fn) => { const p = new Promise(r => r(fn())); return p; }, await: (p) => p.then(v => v), sleep: (ms) => new Promise(r => setTimeout(r, ms)), join: (...tasks) => Promise.all(tasks), parallel: (fns) => Promise.all(fns.map(f => f())), race: (fns) => Promise.race(fns.map(f => f())), all: (fns) => Promise.all(fns.map(f => f())), timeout: (p, ms) => Promise.race([p, new Promise((_, r) => setTimeout(() => r(new Error('timeout')), ms))]) };
const Channel = (() => { const EventEmitter = require('events'); class Ch { constructor(cap) { this._ee = new EventEmitter(); this._q = []; this._cap = cap || Infinity; this._closed = false; } send(v) { if (this._closed) throw new Error('closed'); this._q.push(v); this._ee.emit('data'); } async recv() { if (this._q.length) return this._q.shift(); return new Promise(r => this._ee.once('data', () => r(this._q.shift()))); } close() { this._closed = true; this._ee.emit('close'); } len() { return this._q.length; } cap() { return this._cap; } } const select = async (chs) => Promise.race(chs.map(ch => ch.recv())); return { new: (cap) => new Ch(cap), send: (ch, v) => ch.send(v), recv: (ch) => ch.recv(), close: (ch) => ch.close(), len: (ch) => ch.len(), cap: (ch) => ch.cap(), select: select, try_send: (ch, v) => { if (ch._closed) return false; ch.send(v); return true; }, try_recv: (ch) => ch._q.length ? ch._q.shift() : null }; })();
const Sync = { mutex_new: () => ({ _locked: false, _q: [] }), mutex_lock: async (m) => { if (!m._locked) { m._locked = true; return; } return new Promise(r => m._q.push(r)); }, mutex_unlock: (m) => { if (m._q.length) m._q.shift()(); else m._locked = false; }, once_do: (fn) => { let done = false, val; return () => { if (!done) { done = true; val = fn(); } return val; }; }, wait_group_new: () => ({ _count: 0, _resolve: null }), wait_group_add: (wg, n) => { wg._count += n; }, wait_group_done: (wg) => { wg._count--; if (wg._count === 0 && wg._resolve) wg._resolve(); }, wait_group_wait: (wg) => new Promise(r => { if (wg._count === 0) r(); else wg._resolve = r; }) };
"#;

const PY_STDLIB_PRELUDE: &str = r#"
# --- Tangle Standard Library (Python) ---
import json, math, hashlib, hmac, re, datetime as dt, os, sys, random, base64, binascii, urllib.parse, shutil, subprocess, time, threading, asyncio, queue
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

class fmt:
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
    @staticmethod
    def format(s, *args): return s.format(*args)

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

class Path:
    @staticmethod
    def join(*parts): return os.path.join(*parts)
    @staticmethod
    def basename(p): return os.path.basename(p)
    @staticmethod
    def dirname(p): return os.path.dirname(p)
    @staticmethod
    def extension(p): return os.path.splitext(p)[1]
    @staticmethod
    def is_absolute(p): return os.path.isabs(p)
    @staticmethod
    def normalize(p): return os.path.normpath(p)
    @staticmethod
    def relative(f, t): return os.path.relpath(t, os.path.dirname(f))
    @staticmethod
    def split(p): return p.replace('\\', '/').strip('/').split('/')

class IO:
    @staticmethod
    def readFile(p): return open(p, 'r').read()
    @staticmethod
    def writeFile(p, d):
        with open(p, 'w') as f: f.write(d)
    @staticmethod
    def exists(p): return os.path.exists(p)
    @staticmethod
    def stat(p): return os.stat(p)
    @staticmethod
    def mkdir(p): return os.makedirs(p, exist_ok=True)
    @staticmethod
    def read_dir(p): return os.listdir(p)
    @staticmethod
    def remove(p): return os.remove(p) if os.path.isfile(p) else shutil.rmtree(p)
    @staticmethod
    def rename(a, b): return os.rename(a, b)
    @staticmethod
    def copy(a, b): return shutil.copy2(a, b)
    @staticmethod
    def chmod(p, m): return os.chmod(p, m)
    @staticmethod
    def size(p): return os.path.getsize(p)
    @staticmethod
    def is_dir(p): return os.path.isdir(p)
    @staticmethod
    def is_file(p): return os.path.isfile(p)

class Process:
    @staticmethod
    def run(cmd, args=None): return subprocess.run([cmd] + (args or []), capture_output=True, text=True)
    @staticmethod
    def exec(cmd): return subprocess.check_output(cmd, shell=True, text=True)
    @staticmethod
    def spawn(cmd, args=None): return subprocess.Popen([cmd] + (args or []))
    @staticmethod
    def exit(code=0): sys.exit(code)
    @staticmethod
    def pid(): return os.getpid()
    @staticmethod
    def args(): return sys.argv[1:]
    @staticmethod
    def stdout(): return sys.stdout
    @staticmethod
    def stderr(): return sys.stderr
    @staticmethod
    def status(): return 0

class Task:
    @staticmethod
    async def spawn(fn): return await asyncio.create_task(fn())
    @staticmethod
    async def await_(coro): return await coro
    @staticmethod
    async def sleep(ms): return await asyncio.sleep(ms / 1000)
    @staticmethod
    async def join(*coros): return await asyncio.gather(*coros)
    @staticmethod
    async def parallel(fns): return await asyncio.gather(*[fn() for fn in fns])
    @staticmethod
    async def race(fns): return await asyncio.wait([fn() for fn in fns], return_when=asyncio.FIRST_COMPLETED)
    @staticmethod
    async def all(fns): return await asyncio.gather(*[fn() for fn in fns])
    @staticmethod
    async def timeout(coro, ms): return await asyncio.wait_for(coro, timeout=ms/1000)

class Channel:
    def __init__(self, cap=0): self._q = queue.Queue(maxsize=cap if cap > 0 else 0); self._closed = False
    def send(self, v): self._q.put(v)
    def recv(self): return self._q.get()
    def close(self): self._closed = True
    def len(self): return self._q.qsize()
    def cap(self): return self._q.maxsize
    @staticmethod
    def new(cap=0): return Channel(cap)
    @staticmethod
    def send_(ch, v): ch.send(v)
    @staticmethod
    def recv_(ch): return ch.recv()
    @staticmethod
    def close_(ch): ch.close()
    @staticmethod
    def len_(ch): return ch.len()
    @staticmethod
    def try_send(ch, v):
        try: ch._q.put_nowait(v); return True
        except queue.Full: return False
    @staticmethod
    def try_recv(ch):
        try: return ch._q.get_nowait()
        except queue.Empty: return None

class Sync:
    @staticmethod
    def mutex_new(): return threading.Lock()
    @staticmethod
    def mutex_lock(m): m.acquire()
    @staticmethod
    def mutex_unlock(m): m.release()
    @staticmethod
    def once_do(fn):
        done, val, lock = False, None, threading.Lock()
        def wrapper():
            nonlocal done, val
            with lock:
                if not done:
                    done = True
                    val = fn()
            return val
        return wrapper
    @staticmethod
    def wait_group_new(): return {'count': 0, 'cond': threading.Condition()}
    @staticmethod
    def wait_group_add(wg, n):
        with wg['cond']: wg['count'] += n
    @staticmethod
    def wait_group_done(wg):
        with wg['cond']:
            wg['count'] -= 1
            if wg['count'] == 0: wg['cond'].notify_all()
    @staticmethod
    def wait_group_wait(wg):
        with wg['cond']:
            while wg['count'] > 0: wg['cond'].wait()
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
    "os/exec"
    "path/filepath"
    "regexp"
    "sort"
    "strings"
    "sync"
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

// fmt helpers (Go's native fmt package)
func FmtPrint(args ...interface{}) { fmt.Print(args...) }
func FmtPrintln(args ...interface{}) { fmt.Println(args...) }
func FmtInput(prompt string) string { fmt.Print(prompt); var s string; fmt.Scan(&s); return s }
func FmtDebug(args ...interface{}) { fmt.Print("[DEBUG] "); fmt.Println(args...) }
func FmtError(args ...interface{}) { fmt.Fprintln(os.Stderr, args...) }
func FmtFormat(format string, args ...interface{}) string { return fmt.Sprintf(format, args...) }

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

// Path helpers (Go: path/filepath)
func PathJoin(parts ...string) string { return filepath.Join(parts...) }
func PathBasename(p string) string { return filepath.Base(p) }
func PathDirname(p string) string { return filepath.Dir(p) }
func PathExtension(p string) string { return filepath.Ext(p) }
func PathIsAbsolute(p string) bool { return filepath.IsAbs(p) }
func PathNormalize(p string) string { return filepath.Clean(p) }
func PathRelative(from, to string) (string, error) { return filepath.Rel(from, to) }
func PathSplit(p string) []string { return strings.Split(filepath.Clean(p), string(filepath.Separator)) }

// IO helpers (Go: os file I/O & filesystem ops)
func IOReadFile(p string) (string, error) { data, err := os.ReadFile(p); return string(data), err }
func IOWriteFile(p string, data string) error { return os.WriteFile(p, []byte(data), 0644) }
func IOExists(p string) bool { _, err := os.Stat(p); return err == nil }
func IOStat(p string) (os.FileInfo, error) { return os.Stat(p) }
func IOMkdir(p string) error { return os.MkdirAll(p, 0755) }
func IOReadDir(p string) ([]os.DirEntry, error) { return os.ReadDir(p) }
func IORemove(p string) error { return os.RemoveAll(p) }
func IORename(old, new string) error { return os.Rename(old, new) }
func IOCopy(src, dst string) error { data, err := os.ReadFile(src); if err != nil { return err }; return os.WriteFile(dst, data, 0644) }
func IOChmod(p string, mode os.FileMode) error { return os.Chmod(p, mode) }
func IOSize(p string) (int64, error) { info, err := os.Stat(p); if err != nil { return 0, err }; return info.Size(), nil }
func IOIsDir(p string) bool { info, err := os.Stat(p); return err == nil && info.IsDir() }
func IOIsFile(p string) bool { info, err := os.Stat(p); return err == nil && !info.IsDir() }

// Process helpers (Go: os/exec)
func ProcessRun(name string, args ...string) ([]byte, error) { return exec.Command(name, args...).Output() }
func ProcessExec(cmd string) ([]byte, error) { return exec.Command("sh", "-c", cmd).Output() }
func ProcessSpawn(name string, args ...string) (*exec.Cmd, error) { cmd := exec.Command(name, args...); cmd.Stdout = os.Stdout; cmd.Stderr = os.Stderr; return cmd, cmd.Start() }
func ProcessExit(code int) { os.Exit(code) }
func ProcessPid() int { return os.Getpid() }
func ProcessArgs() []string { return os.Args[1:] }

// Task helpers (Go: goroutines)
func TaskSpawn(fn func()) { go fn() }
func TaskSleep(ms int64) { time.Sleep(time.Duration(ms) * time.Millisecond) }

// Channel type (Go: native channel)
type Channel[T any] struct { ch chan T; closed bool }
func ChannelNew[T any](cap int) *Channel[T] { c := make(chan T, cap); return &Channel[T]{ch: c} }
func ChannelSend[T any](ch *Channel[T], v T) { ch.ch <- v }
func ChannelRecv[T any](ch *Channel[T]) T { return <-ch.ch }
func ChannelClose[T any](ch *Channel[T]) { close(ch.ch); ch.closed = true }
func ChannelLen[T any](ch *Channel[T]) int { return len(ch.ch) }
func ChannelCap[T any](ch *Channel[T]) int { return cap(ch.ch) }
func ChannelTrySend[T any](ch *Channel[T], v T) bool { select { case ch.ch <- v: return true; default: return false } }
func ChannelTryRecv[T any](ch *Channel[T]) (T, bool) { select { case v := <-ch.ch: return v, true; default: var zero T; return zero, false } }

// Sync helpers (Go: sync package)
func SyncMutexNew() *sync.Mutex { return &sync.Mutex{} }
func SyncOnceDo(fn func()) func() { var once sync.Once; return func() { once.Do(fn) } }
func SyncWaitGroupNew() *sync.WaitGroup { return &sync.WaitGroup{} }
"#;
