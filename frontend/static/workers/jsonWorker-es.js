var ra = Object.defineProperty;
var na = (e, t, r) => t in e ? ra(e, t, { enumerable: !0, configurable: !0, writable: !0, value: r }) : e[t] = r;
var vt = (e, t, r) => (na(e, typeof t != "symbol" ? t + "" : t, r), r);
class ia {
  constructor() {
    this.listeners = [], this.unexpectedErrorHandler = function(t) {
      setTimeout(() => {
        throw t.stack ? gt.isErrorNoTelemetry(t) ? new gt(t.message + `

` + t.stack) : new Error(t.message + `

` + t.stack) : t;
      }, 0);
    };
  }
  addListener(t) {
    return this.listeners.push(t), () => {
      this._removeListener(t);
    };
  }
  emit(t) {
    this.listeners.forEach((r) => {
      r(t);
    });
  }
  _removeListener(t) {
    this.listeners.splice(this.listeners.indexOf(t), 1);
  }
  setUnexpectedErrorHandler(t) {
    this.unexpectedErrorHandler = t;
  }
  getUnexpectedErrorHandler() {
    return this.unexpectedErrorHandler;
  }
  onUnexpectedError(t) {
    this.unexpectedErrorHandler(t), this.emit(t);
  }
  // For external errors, we don't want the listeners to be called
  onUnexpectedExternalError(t) {
    this.unexpectedErrorHandler(t);
  }
}
const sa = new ia();
function Ss(e) {
  aa(e) || sa.onUnexpectedError(e);
}
function bn(e) {
  if (e instanceof Error) {
    const { name: t, message: r } = e, n = e.stacktrace || e.stack;
    return {
      $isError: !0,
      name: t,
      message: r,
      stack: n,
      noTelemetry: gt.isErrorNoTelemetry(e)
    };
  }
  return e;
}
const Lr = "Canceled";
function aa(e) {
  return e instanceof oa ? !0 : e instanceof Error && e.name === Lr && e.message === Lr;
}
class oa extends Error {
  constructor() {
    super(Lr), this.name = this.message;
  }
}
class gt extends Error {
  constructor(t) {
    super(t), this.name = "CodeExpectedError";
  }
  static fromError(t) {
    if (t instanceof gt)
      return t;
    const r = new gt();
    return r.message = t.message, r.stack = t.stack, r;
  }
  static isErrorNoTelemetry(t) {
    return t.name === "CodeExpectedError";
  }
}
class et extends Error {
  constructor(t) {
    super(t || "An unexpected bug occurred."), Object.setPrototypeOf(this, et.prototype);
    debugger;
  }
}
function la(e) {
  const t = this;
  let r = !1, n;
  return function() {
    return r || (r = !0, n = e.apply(t, arguments)), n;
  };
}
var zt;
(function(e) {
  function t(y) {
    return y && typeof y == "object" && typeof y[Symbol.iterator] == "function";
  }
  e.is = t;
  const r = Object.freeze([]);
  function n() {
    return r;
  }
  e.empty = n;
  function* i(y) {
    yield y;
  }
  e.single = i;
  function s(y) {
    return t(y) ? y : i(y);
  }
  e.wrap = s;
  function a(y) {
    return y || r;
  }
  e.from = a;
  function o(y) {
    return !y || y[Symbol.iterator]().next().done === !0;
  }
  e.isEmpty = o;
  function l(y) {
    return y[Symbol.iterator]().next().value;
  }
  e.first = l;
  function u(y, x) {
    for (const b of y)
      if (x(b))
        return !0;
    return !1;
  }
  e.some = u;
  function f(y, x) {
    for (const b of y)
      if (x(b))
        return b;
  }
  e.find = f;
  function* h(y, x) {
    for (const b of y)
      x(b) && (yield b);
  }
  e.filter = h;
  function* d(y, x) {
    let b = 0;
    for (const S of y)
      yield x(S, b++);
  }
  e.map = d;
  function* g(...y) {
    for (const x of y)
      for (const b of x)
        yield b;
  }
  e.concat = g;
  function p(y, x, b) {
    let S = b;
    for (const M of y)
      S = x(S, M);
    return S;
  }
  e.reduce = p;
  function* m(y, x, b = y.length) {
    for (x < 0 && (x += y.length), b < 0 ? b += y.length : b > y.length && (b = y.length); x < b; x++)
      yield y[x];
  }
  e.slice = m;
  function v(y, x = Number.POSITIVE_INFINITY) {
    const b = [];
    if (x === 0)
      return [b, y];
    const S = y[Symbol.iterator]();
    for (let M = 0; M < x; M++) {
      const A = S.next();
      if (A.done)
        return [b, e.empty()];
      b.push(A.value);
    }
    return [b, { [Symbol.iterator]() {
      return S;
    } }];
  }
  e.consume = v;
})(zt || (zt = {}));
globalThis && globalThis.__awaiter;
function As(e) {
  if (zt.is(e)) {
    const t = [];
    for (const r of e)
      if (r)
        try {
          r.dispose();
        } catch (n) {
          t.push(n);
        }
    if (t.length === 1)
      throw t[0];
    if (t.length > 1)
      throw new AggregateError(t, "Encountered errors while disposing of store");
    return Array.isArray(e) ? [] : e;
  } else if (e)
    return e.dispose(), e;
}
function ua(...e) {
  return Gt(() => As(e));
}
function Gt(e) {
  return {
    dispose: la(() => {
      e();
    })
  };
}
class Ke {
  constructor() {
    this._toDispose = /* @__PURE__ */ new Set(), this._isDisposed = !1;
  }
  /**
   * Dispose of all registered disposables and mark this object as disposed.
   *
   * Any future disposables added to this object will be disposed of on `add`.
   */
  dispose() {
    this._isDisposed || (this._isDisposed = !0, this.clear());
  }
  /**
   * @return `true` if this object has been disposed of.
   */
  get isDisposed() {
    return this._isDisposed;
  }
  /**
   * Dispose of all registered disposables but do not mark this object as disposed.
   */
  clear() {
    if (this._toDispose.size !== 0)
      try {
        As(this._toDispose);
      } finally {
        this._toDispose.clear();
      }
  }
  /**
   * Add a new {@link IDisposable disposable} to the collection.
   */
  add(t) {
    if (!t)
      return t;
    if (t === this)
      throw new Error("Cannot register a disposable on itself!");
    return this._isDisposed ? Ke.DISABLE_DISPOSED_WARNING || console.warn(new Error("Trying to add a disposable to a DisposableStore that has already been disposed of. The added object will be leaked!").stack) : this._toDispose.add(t), t;
  }
}
Ke.DISABLE_DISPOSED_WARNING = !1;
class lr {
  constructor() {
    this._store = new Ke(), this._store;
  }
  dispose() {
    this._store.dispose();
  }
  /**
   * Adds `o` to the collection of disposables managed by this object.
   */
  _register(t) {
    if (t === this)
      throw new Error("Cannot register a disposable on itself!");
    return this._store.add(t);
  }
}
lr.None = Object.freeze({ dispose() {
} });
class ca {
  constructor() {
    this.dispose = () => {
    }, this.unset = () => {
    }, this.isset = () => !1;
  }
  set(t) {
    let r = t;
    return this.unset = () => r = void 0, this.isset = () => r !== void 0, this.dispose = () => {
      r && (r(), r = void 0);
    }, this;
  }
}
class J {
  constructor(t) {
    this.element = t, this.next = J.Undefined, this.prev = J.Undefined;
  }
}
J.Undefined = new J(void 0);
class Jt {
  constructor() {
    this._first = J.Undefined, this._last = J.Undefined, this._size = 0;
  }
  get size() {
    return this._size;
  }
  isEmpty() {
    return this._first === J.Undefined;
  }
  clear() {
    let t = this._first;
    for (; t !== J.Undefined; ) {
      const r = t.next;
      t.prev = J.Undefined, t.next = J.Undefined, t = r;
    }
    this._first = J.Undefined, this._last = J.Undefined, this._size = 0;
  }
  unshift(t) {
    return this._insert(t, !1);
  }
  push(t) {
    return this._insert(t, !0);
  }
  _insert(t, r) {
    const n = new J(t);
    if (this._first === J.Undefined)
      this._first = n, this._last = n;
    else if (r) {
      const s = this._last;
      this._last = n, n.prev = s, s.next = n;
    } else {
      const s = this._first;
      this._first = n, n.next = s, s.prev = n;
    }
    this._size += 1;
    let i = !1;
    return () => {
      i || (i = !0, this._remove(n));
    };
  }
  shift() {
    if (this._first !== J.Undefined) {
      const t = this._first.element;
      return this._remove(this._first), t;
    }
  }
  pop() {
    if (this._last !== J.Undefined) {
      const t = this._last.element;
      return this._remove(this._last), t;
    }
  }
  _remove(t) {
    if (t.prev !== J.Undefined && t.next !== J.Undefined) {
      const r = t.prev;
      r.next = t.next, t.next.prev = r;
    } else
      t.prev === J.Undefined && t.next === J.Undefined ? (this._first = J.Undefined, this._last = J.Undefined) : t.next === J.Undefined ? (this._last = this._last.prev, this._last.next = J.Undefined) : t.prev === J.Undefined && (this._first = this._first.next, this._first.prev = J.Undefined);
    this._size -= 1;
  }
  *[Symbol.iterator]() {
    let t = this._first;
    for (; t !== J.Undefined; )
      yield t.element, t = t.next;
  }
}
globalThis && globalThis.__awaiter;
let fa = typeof document < "u" && document.location && document.location.hash.indexOf("pseudo=true") >= 0;
function ha(e, t) {
  let r;
  return t.length === 0 ? r = e : r = e.replace(/\{(\d+)\}/g, (n, i) => {
    const s = i[0], a = t[s];
    let o = n;
    return typeof a == "string" ? o = a : (typeof a == "number" || typeof a == "boolean" || a === void 0 || a === null) && (o = String(a)), o;
  }), fa && (r = "［" + r.replace(/[aouei]/g, "$&$&") + "］"), r;
}
function da(e, t, ...r) {
  return ha(t, r);
}
var dr;
const ct = "en";
let Cr = !1, kr = !1, gr = !1, Ns = !1, Ft, $t = ct, yn = ct, ga, we;
const he = typeof self == "object" ? self : typeof global == "object" ? global : {};
let se;
typeof he.vscode < "u" && typeof he.vscode.process < "u" ? se = he.vscode.process : typeof process < "u" && (se = process);
const ma = typeof ((dr = se == null ? void 0 : se.versions) === null || dr === void 0 ? void 0 : dr.electron) == "string", pa = ma && (se == null ? void 0 : se.type) === "renderer";
if (typeof navigator == "object" && !pa)
  we = navigator.userAgent, Cr = we.indexOf("Windows") >= 0, kr = we.indexOf("Macintosh") >= 0, (we.indexOf("Macintosh") >= 0 || we.indexOf("iPad") >= 0 || we.indexOf("iPhone") >= 0) && navigator.maxTouchPoints && navigator.maxTouchPoints > 0, gr = we.indexOf("Linux") >= 0, (we == null ? void 0 : we.indexOf("Mobi")) >= 0, Ns = !0, // This call _must_ be done in the file that calls `nls.getConfiguredDefaultLocale`
  // to ensure that the NLS AMD Loader plugin has been loaded and configured.
  // This is because the loader plugin decides what the default locale is based on
  // how it's able to resolve the strings.
  da({ key: "ensureLoaderPluginIsLoaded", comment: ["{Locked}"] }, "_"), Ft = ct, $t = Ft, yn = navigator.language;
else if (typeof se == "object") {
  Cr = se.platform === "win32", kr = se.platform === "darwin", gr = se.platform === "linux", gr && se.env.SNAP && se.env.SNAP_REVISION, se.env.CI || se.env.BUILD_ARTIFACTSTAGINGDIRECTORY, Ft = ct, $t = ct;
  const e = se.env.VSCODE_NLS_CONFIG;
  if (e)
    try {
      const t = JSON.parse(e), r = t.availableLanguages["*"];
      Ft = t.locale, yn = t.osLocale, $t = r || ct, ga = t._translationsConfigFile;
    } catch {
    }
} else
  console.error("Unable to resolve platform.");
const At = Cr, va = kr;
Ns && he.importScripts;
const Pe = we, je = $t;
var xn;
(function(e) {
  function t() {
    return je;
  }
  e.value = t;
  function r() {
    return je.length === 2 ? je === "en" : je.length >= 3 ? je[0] === "e" && je[1] === "n" && je[2] === "-" : !1;
  }
  e.isDefaultVariant = r;
  function n() {
    return je === "en";
  }
  e.isDefault = n;
})(xn || (xn = {}));
const ba = typeof he.postMessage == "function" && !he.importScripts;
(() => {
  if (ba) {
    const e = [];
    he.addEventListener("message", (r) => {
      if (r.data && r.data.vscodeScheduleAsyncWork)
        for (let n = 0, i = e.length; n < i; n++) {
          const s = e[n];
          if (s.id === r.data.vscodeScheduleAsyncWork) {
            e.splice(n, 1), s.callback();
            return;
          }
        }
    });
    let t = 0;
    return (r) => {
      const n = ++t;
      e.push({
        id: n,
        callback: r
      }), he.postMessage({ vscodeScheduleAsyncWork: n }, "*");
    };
  }
  return (e) => setTimeout(e);
})();
const ya = !!(Pe && Pe.indexOf("Chrome") >= 0);
Pe && Pe.indexOf("Firefox") >= 0;
!ya && Pe && Pe.indexOf("Safari") >= 0;
Pe && Pe.indexOf("Edg/") >= 0;
Pe && Pe.indexOf("Android") >= 0;
const xa = he.performance && typeof he.performance.now == "function";
class ur {
  static create(t = !0) {
    return new ur(t);
  }
  constructor(t) {
    this._highResolution = xa && t, this._startTime = this._now(), this._stopTime = -1;
  }
  stop() {
    this._stopTime = this._now();
  }
  reset() {
    this._startTime = this._now(), this._stopTime = -1;
  }
  elapsed() {
    return this._stopTime !== -1 ? this._stopTime - this._startTime : this._now() - this._startTime;
  }
  _now() {
    return this._highResolution ? he.performance.now() : Date.now();
  }
}
globalThis && globalThis.__awaiter;
var Er;
(function(e) {
  e.None = () => lr.None;
  function t(_, w) {
    return f(_, () => {
    }, 0, void 0, !0, void 0, w);
  }
  e.defer = t;
  function r(_) {
    return (w, N = null, k) => {
      let L = !1, F;
      return F = _((D) => {
        if (!L)
          return F ? F.dispose() : L = !0, w.call(N, D);
      }, null, k), L && F.dispose(), F;
    };
  }
  e.once = r;
  function n(_, w, N) {
    return u((k, L = null, F) => _((D) => k.call(L, w(D)), null, F), N);
  }
  e.map = n;
  function i(_, w, N) {
    return u((k, L = null, F) => _((D) => {
      w(D), k.call(L, D);
    }, null, F), N);
  }
  e.forEach = i;
  function s(_, w, N) {
    return u((k, L = null, F) => _((D) => w(D) && k.call(L, D), null, F), N);
  }
  e.filter = s;
  function a(_) {
    return _;
  }
  e.signal = a;
  function o(..._) {
    return (w, N = null, k) => ua(..._.map((L) => L((F) => w.call(N, F), null, k)));
  }
  e.any = o;
  function l(_, w, N, k) {
    let L = N;
    return n(_, (F) => (L = w(L, F), L), k);
  }
  e.reduce = l;
  function u(_, w) {
    let N;
    const k = {
      onWillAddFirstListener() {
        N = _(L.fire, L);
      },
      onDidRemoveLastListener() {
        N == null || N.dispose();
      }
    }, L = new Me(k);
    return w == null || w.add(L), L.event;
  }
  function f(_, w, N = 100, k = !1, L = !1, F, D) {
    let z, W, P, T = 0, R;
    const I = {
      leakWarningThreshold: F,
      onWillAddFirstListener() {
        z = _((U) => {
          T++, W = w(W, U), k && !P && (j.fire(W), W = void 0), R = () => {
            const q = W;
            W = void 0, P = void 0, (!k || T > 1) && j.fire(q), T = 0;
          }, typeof N == "number" ? (clearTimeout(P), P = setTimeout(R, N)) : P === void 0 && (P = 0, queueMicrotask(R));
        });
      },
      onWillRemoveListener() {
        L && T > 0 && (R == null || R());
      },
      onDidRemoveLastListener() {
        R = void 0, z.dispose();
      }
    }, j = new Me(I);
    return D == null || D.add(j), j.event;
  }
  e.debounce = f;
  function h(_, w = 0, N) {
    return e.debounce(_, (k, L) => k ? (k.push(L), k) : [L], w, void 0, !0, void 0, N);
  }
  e.accumulate = h;
  function d(_, w = (k, L) => k === L, N) {
    let k = !0, L;
    return s(_, (F) => {
      const D = k || !w(F, L);
      return k = !1, L = F, D;
    }, N);
  }
  e.latch = d;
  function g(_, w, N) {
    return [
      e.filter(_, w, N),
      e.filter(_, (k) => !w(k), N)
    ];
  }
  e.split = g;
  function p(_, w = !1, N = []) {
    let k = N.slice(), L = _((z) => {
      k ? k.push(z) : D.fire(z);
    });
    const F = () => {
      k == null || k.forEach((z) => D.fire(z)), k = null;
    }, D = new Me({
      onWillAddFirstListener() {
        L || (L = _((z) => D.fire(z)));
      },
      onDidAddFirstListener() {
        k && (w ? setTimeout(F) : F());
      },
      onDidRemoveLastListener() {
        L && L.dispose(), L = null;
      }
    });
    return D.event;
  }
  e.buffer = p;
  class m {
    constructor(w) {
      this.event = w, this.disposables = new Ke();
    }
    /** @see {@link Event.map} */
    map(w) {
      return new m(n(this.event, w, this.disposables));
    }
    /** @see {@link Event.forEach} */
    forEach(w) {
      return new m(i(this.event, w, this.disposables));
    }
    filter(w) {
      return new m(s(this.event, w, this.disposables));
    }
    /** @see {@link Event.reduce} */
    reduce(w, N) {
      return new m(l(this.event, w, N, this.disposables));
    }
    /** @see {@link Event.reduce} */
    latch() {
      return new m(d(this.event, void 0, this.disposables));
    }
    debounce(w, N = 100, k = !1, L = !1, F) {
      return new m(f(this.event, w, N, k, L, F, this.disposables));
    }
    /**
     * Attach a listener to the event.
     */
    on(w, N, k) {
      return this.event(w, N, k);
    }
    /** @see {@link Event.once} */
    once(w, N, k) {
      return r(this.event)(w, N, k);
    }
    dispose() {
      this.disposables.dispose();
    }
  }
  function v(_) {
    return new m(_);
  }
  e.chain = v;
  function y(_, w, N = (k) => k) {
    const k = (...z) => D.fire(N(...z)), L = () => _.on(w, k), F = () => _.removeListener(w, k), D = new Me({ onWillAddFirstListener: L, onDidRemoveLastListener: F });
    return D.event;
  }
  e.fromNodeEventEmitter = y;
  function x(_, w, N = (k) => k) {
    const k = (...z) => D.fire(N(...z)), L = () => _.addEventListener(w, k), F = () => _.removeEventListener(w, k), D = new Me({ onWillAddFirstListener: L, onDidRemoveLastListener: F });
    return D.event;
  }
  e.fromDOMEventEmitter = x;
  function b(_) {
    return new Promise((w) => r(_)(w));
  }
  e.toPromise = b;
  function S(_, w) {
    return w(void 0), _((N) => w(N));
  }
  e.runAndSubscribe = S;
  function M(_, w) {
    let N = null;
    function k(F) {
      N == null || N.dispose(), N = new Ke(), w(F, N);
    }
    k(void 0);
    const L = _((F) => k(F));
    return Gt(() => {
      L.dispose(), N == null || N.dispose();
    });
  }
  e.runAndSubscribeWithStore = M;
  class A {
    constructor(w, N) {
      this.obs = w, this._counter = 0, this._hasChanged = !1;
      const k = {
        onWillAddFirstListener: () => {
          w.addObserver(this);
        },
        onDidRemoveLastListener: () => {
          w.removeObserver(this);
        }
      };
      this.emitter = new Me(k), N && N.add(this.emitter);
    }
    beginUpdate(w) {
      this._counter++;
    }
    handleChange(w, N) {
      this._hasChanged = !0;
    }
    endUpdate(w) {
      --this._counter === 0 && this._hasChanged && (this._hasChanged = !1, this.emitter.fire(this.obs.get()));
    }
  }
  function C(_, w) {
    return new A(_, w).emitter.event;
  }
  e.fromObservable = C;
})(Er || (Er = {}));
class mt {
  constructor(t) {
    this.listenerCount = 0, this.invocationCount = 0, this.elapsedOverall = 0, this.durations = [], this.name = `${t}_${mt._idPool++}`, mt.all.add(this);
  }
  start(t) {
    this._stopWatch = new ur(!0), this.listenerCount = t;
  }
  stop() {
    if (this._stopWatch) {
      const t = this._stopWatch.elapsed();
      this.durations.push(t), this.elapsedOverall += t, this.invocationCount += 1, this._stopWatch = void 0;
    }
  }
}
mt.all = /* @__PURE__ */ new Set();
mt._idPool = 0;
let wa = -1;
class _a {
  constructor(t, r = Math.random().toString(18).slice(2, 5)) {
    this.threshold = t, this.name = r, this._warnCountdown = 0;
  }
  dispose() {
    var t;
    (t = this._stacks) === null || t === void 0 || t.clear();
  }
  check(t, r) {
    const n = this.threshold;
    if (n <= 0 || r < n)
      return;
    this._stacks || (this._stacks = /* @__PURE__ */ new Map());
    const i = this._stacks.get(t.value) || 0;
    if (this._stacks.set(t.value, i + 1), this._warnCountdown -= 1, this._warnCountdown <= 0) {
      this._warnCountdown = n * 0.5;
      let s, a = 0;
      for (const [o, l] of this._stacks)
        (!s || a < l) && (s = o, a = l);
      console.warn(`[${this.name}] potential listener LEAK detected, having ${r} listeners already. MOST frequent listener (${a}):`), console.warn(s);
    }
    return () => {
      const s = this._stacks.get(t.value) || 0;
      this._stacks.set(t.value, s - 1);
    };
  }
}
class an {
  static create() {
    var t;
    return new an((t = new Error().stack) !== null && t !== void 0 ? t : "");
  }
  constructor(t) {
    this.value = t;
  }
  print() {
    console.warn(this.value.split(`
`).slice(2).join(`
`));
  }
}
class Sa {
  constructor(t, r, n) {
    this.callback = t, this.callbackThis = r, this.stack = n, this.subscription = new ca();
  }
  invoke(t) {
    this.callback.call(this.callbackThis, t);
  }
}
class Me {
  constructor(t) {
    var r, n, i, s, a;
    this._disposed = !1, this._options = t, this._leakageMon = !((r = this._options) === null || r === void 0) && r.leakWarningThreshold ? new _a((i = (n = this._options) === null || n === void 0 ? void 0 : n.leakWarningThreshold) !== null && i !== void 0 ? i : wa) : void 0, this._perfMon = !((s = this._options) === null || s === void 0) && s._profName ? new mt(this._options._profName) : void 0, this._deliveryQueue = (a = this._options) === null || a === void 0 ? void 0 : a.deliveryQueue;
  }
  dispose() {
    var t, r, n, i;
    this._disposed || (this._disposed = !0, this._listeners && this._listeners.clear(), (t = this._deliveryQueue) === null || t === void 0 || t.clear(this), (n = (r = this._options) === null || r === void 0 ? void 0 : r.onDidRemoveLastListener) === null || n === void 0 || n.call(r), (i = this._leakageMon) === null || i === void 0 || i.dispose());
  }
  /**
   * For the public to allow to subscribe
   * to events from this Emitter
   */
  get event() {
    return this._event || (this._event = (t, r, n) => {
      var i, s, a;
      if (this._listeners || (this._listeners = new Jt()), this._leakageMon && this._listeners.size > this._leakageMon.threshold * 3)
        return console.warn(`[${this._leakageMon.name}] REFUSES to accept new listeners because it exceeded its threshold by far`), lr.None;
      const o = this._listeners.isEmpty();
      o && (!((i = this._options) === null || i === void 0) && i.onWillAddFirstListener) && this._options.onWillAddFirstListener(this);
      let l, u;
      this._leakageMon && this._listeners.size >= Math.ceil(this._leakageMon.threshold * 0.2) && (u = an.create(), l = this._leakageMon.check(u, this._listeners.size + 1));
      const f = new Sa(t, r, u), h = this._listeners.push(f);
      o && (!((s = this._options) === null || s === void 0) && s.onDidAddFirstListener) && this._options.onDidAddFirstListener(this), !((a = this._options) === null || a === void 0) && a.onDidAddListener && this._options.onDidAddListener(this, t, r);
      const d = f.subscription.set(() => {
        var g, p;
        l == null || l(), this._disposed || ((p = (g = this._options) === null || g === void 0 ? void 0 : g.onWillRemoveListener) === null || p === void 0 || p.call(g, this), h(), this._options && this._options.onDidRemoveLastListener && (this._listeners && !this._listeners.isEmpty() || this._options.onDidRemoveLastListener(this)));
      });
      return n instanceof Ke ? n.add(d) : Array.isArray(n) && n.push(d), d;
    }), this._event;
  }
  /**
   * To be kept private to fire an event to
   * subscribers
   */
  fire(t) {
    var r, n, i;
    if (this._listeners) {
      this._deliveryQueue || (this._deliveryQueue = new Na((r = this._options) === null || r === void 0 ? void 0 : r.onListenerError));
      for (const s of this._listeners)
        this._deliveryQueue.push(this, s, t);
      (n = this._perfMon) === null || n === void 0 || n.start(this._deliveryQueue.size), this._deliveryQueue.deliver(), (i = this._perfMon) === null || i === void 0 || i.stop();
    }
  }
  hasListeners() {
    return this._listeners ? !this._listeners.isEmpty() : !1;
  }
}
class Aa {
  constructor(t = Ss) {
    this._onListenerError = t, this._queue = new Jt();
  }
  get size() {
    return this._queue.size;
  }
  push(t, r, n) {
    this._queue.push(new La(t, r, n));
  }
  clear(t) {
    const r = new Jt();
    for (const n of this._queue)
      n.emitter !== t && r.push(n);
    this._queue = r;
  }
  deliver() {
    for (; this._queue.size > 0; ) {
      const t = this._queue.shift();
      try {
        t.listener.invoke(t.event);
      } catch (r) {
        this._onListenerError(r);
      }
    }
  }
}
class Na extends Aa {
  clear(t) {
    this._queue.clear();
  }
}
class La {
  constructor(t, r, n) {
    this.emitter = t, this.listener = r, this.event = n;
  }
}
function Ca(e) {
  return typeof e == "string";
}
function ka(e) {
  let t = [], r = Object.getPrototypeOf(e);
  for (; Object.prototype !== r; )
    t = t.concat(Object.getOwnPropertyNames(r)), r = Object.getPrototypeOf(r);
  return t;
}
function Mr(e) {
  const t = [];
  for (const r of ka(e))
    typeof e[r] == "function" && t.push(r);
  return t;
}
function Ea(e, t) {
  const r = (i) => function() {
    const s = Array.prototype.slice.call(arguments, 0);
    return t(i, s);
  }, n = {};
  for (const i of e)
    n[i] = r(i);
  return n;
}
const Ls = Object.freeze(function(e, t) {
  const r = setTimeout(e.bind(t), 0);
  return { dispose() {
    clearTimeout(r);
  } };
});
var Xt;
(function(e) {
  function t(r) {
    return r === e.None || r === e.Cancelled || r instanceof qt ? !0 : !r || typeof r != "object" ? !1 : typeof r.isCancellationRequested == "boolean" && typeof r.onCancellationRequested == "function";
  }
  e.isCancellationToken = t, e.None = Object.freeze({
    isCancellationRequested: !1,
    onCancellationRequested: Er.None
  }), e.Cancelled = Object.freeze({
    isCancellationRequested: !0,
    onCancellationRequested: Ls
  });
})(Xt || (Xt = {}));
class qt {
  constructor() {
    this._isCancelled = !1, this._emitter = null;
  }
  cancel() {
    this._isCancelled || (this._isCancelled = !0, this._emitter && (this._emitter.fire(void 0), this.dispose()));
  }
  get isCancellationRequested() {
    return this._isCancelled;
  }
  get onCancellationRequested() {
    return this._isCancelled ? Ls : (this._emitter || (this._emitter = new Me()), this._emitter.event);
  }
  dispose() {
    this._emitter && (this._emitter.dispose(), this._emitter = null);
  }
}
class Ma {
  constructor(t) {
    this._token = void 0, this._parentListener = void 0, this._parentListener = t && t.onCancellationRequested(this.cancel, this);
  }
  get token() {
    return this._token || (this._token = new qt()), this._token;
  }
  cancel() {
    this._token ? this._token instanceof qt && this._token.cancel() : this._token = Xt.Cancelled;
  }
  dispose(t = !1) {
    var r;
    t && this.cancel(), (r = this._parentListener) === null || r === void 0 || r.dispose(), this._token ? this._token instanceof qt && this._token.dispose() : this._token = Xt.None;
  }
}
class Ta {
  constructor(t) {
    this.fn = t, this.lastCache = void 0, this.lastArgKey = void 0;
  }
  get(t) {
    const r = JSON.stringify(t);
    return this.lastArgKey !== r && (this.lastArgKey = r, this.lastCache = this.fn(t)), this.lastCache;
  }
}
class Cs {
  constructor(t) {
    this.executor = t, this._didRun = !1;
  }
  /**
   * True if the lazy value has been resolved.
   */
  get hasValue() {
    return this._didRun;
  }
  /**
   * Get the wrapped value.
   *
   * This will force evaluation of the lazy value if it has not been resolved yet. Lazy values are only
   * resolved once. `getValue` will re-throw exceptions that are hit while resolving the value
   */
  get value() {
    if (!this._didRun)
      try {
        this._value = this.executor();
      } catch (t) {
        this._error = t;
      } finally {
        this._didRun = !0;
      }
    if (this._error)
      throw this._error;
    return this._value;
  }
  /**
   * Get the wrapped value without forcing evaluation.
   */
  get rawValue() {
    return this._value;
  }
}
var ks;
function Pa(e) {
  return e.replace(/[\\\{\}\*\+\?\|\^\$\.\[\]\(\)]/g, "\\$&");
}
function Ra(e) {
  return e.split(/\r\n|\r|\n/);
}
function Fa(e) {
  for (let t = 0, r = e.length; t < r; t++) {
    const n = e.charCodeAt(t);
    if (n !== 32 && n !== 9)
      return t;
  }
  return -1;
}
function Ia(e, t = e.length - 1) {
  for (let r = t; r >= 0; r--) {
    const n = e.charCodeAt(r);
    if (n !== 32 && n !== 9)
      return r;
  }
  return -1;
}
function Es(e) {
  return e >= 65 && e <= 90;
}
function Tr(e) {
  return 55296 <= e && e <= 56319;
}
function Va(e) {
  return 56320 <= e && e <= 57343;
}
function Da(e, t) {
  return (e - 55296 << 10) + (t - 56320) + 65536;
}
function Oa(e, t, r) {
  const n = e.charCodeAt(r);
  if (Tr(n) && r + 1 < t) {
    const i = e.charCodeAt(r + 1);
    if (Va(i))
      return Da(n, i);
  }
  return n;
}
const ja = /^[\t\n\r\x20-\x7E]*$/;
function Ua(e) {
  return ja.test(e);
}
class ye {
  static getInstance(t) {
    return ye.cache.get(Array.from(t));
  }
  static getLocales() {
    return ye._locales.value;
  }
  constructor(t) {
    this.confusableDictionary = t;
  }
  isAmbiguous(t) {
    return this.confusableDictionary.has(t);
  }
  /**
   * Returns the non basic ASCII code point that the given code point can be confused,
   * or undefined if such code point does note exist.
   */
  getPrimaryConfusable(t) {
    return this.confusableDictionary.get(t);
  }
  getConfusableCodePoints() {
    return new Set(this.confusableDictionary.keys());
  }
}
ks = ye;
ye.ambiguousCharacterData = new Cs(() => JSON.parse('{"_common":[8232,32,8233,32,5760,32,8192,32,8193,32,8194,32,8195,32,8196,32,8197,32,8198,32,8200,32,8201,32,8202,32,8287,32,8199,32,8239,32,2042,95,65101,95,65102,95,65103,95,8208,45,8209,45,8210,45,65112,45,1748,45,8259,45,727,45,8722,45,10134,45,11450,45,1549,44,1643,44,8218,44,184,44,42233,44,894,59,2307,58,2691,58,1417,58,1795,58,1796,58,5868,58,65072,58,6147,58,6153,58,8282,58,1475,58,760,58,42889,58,8758,58,720,58,42237,58,451,33,11601,33,660,63,577,63,2429,63,5038,63,42731,63,119149,46,8228,46,1793,46,1794,46,42510,46,68176,46,1632,46,1776,46,42232,46,1373,96,65287,96,8219,96,8242,96,1370,96,1523,96,8175,96,65344,96,900,96,8189,96,8125,96,8127,96,8190,96,697,96,884,96,712,96,714,96,715,96,756,96,699,96,701,96,700,96,702,96,42892,96,1497,96,2036,96,2037,96,5194,96,5836,96,94033,96,94034,96,65339,91,10088,40,10098,40,12308,40,64830,40,65341,93,10089,41,10099,41,12309,41,64831,41,10100,123,119060,123,10101,125,65342,94,8270,42,1645,42,8727,42,66335,42,5941,47,8257,47,8725,47,8260,47,9585,47,10187,47,10744,47,119354,47,12755,47,12339,47,11462,47,20031,47,12035,47,65340,92,65128,92,8726,92,10189,92,10741,92,10745,92,119311,92,119355,92,12756,92,20022,92,12034,92,42872,38,708,94,710,94,5869,43,10133,43,66203,43,8249,60,10094,60,706,60,119350,60,5176,60,5810,60,5120,61,11840,61,12448,61,42239,61,8250,62,10095,62,707,62,119351,62,5171,62,94015,62,8275,126,732,126,8128,126,8764,126,65372,124,65293,45,120784,50,120794,50,120804,50,120814,50,120824,50,130034,50,42842,50,423,50,1000,50,42564,50,5311,50,42735,50,119302,51,120785,51,120795,51,120805,51,120815,51,120825,51,130035,51,42923,51,540,51,439,51,42858,51,11468,51,1248,51,94011,51,71882,51,120786,52,120796,52,120806,52,120816,52,120826,52,130036,52,5070,52,71855,52,120787,53,120797,53,120807,53,120817,53,120827,53,130037,53,444,53,71867,53,120788,54,120798,54,120808,54,120818,54,120828,54,130038,54,11474,54,5102,54,71893,54,119314,55,120789,55,120799,55,120809,55,120819,55,120829,55,130039,55,66770,55,71878,55,2819,56,2538,56,2666,56,125131,56,120790,56,120800,56,120810,56,120820,56,120830,56,130040,56,547,56,546,56,66330,56,2663,57,2920,57,2541,57,3437,57,120791,57,120801,57,120811,57,120821,57,120831,57,130041,57,42862,57,11466,57,71884,57,71852,57,71894,57,9082,97,65345,97,119834,97,119886,97,119938,97,119990,97,120042,97,120094,97,120146,97,120198,97,120250,97,120302,97,120354,97,120406,97,120458,97,593,97,945,97,120514,97,120572,97,120630,97,120688,97,120746,97,65313,65,119808,65,119860,65,119912,65,119964,65,120016,65,120068,65,120120,65,120172,65,120224,65,120276,65,120328,65,120380,65,120432,65,913,65,120488,65,120546,65,120604,65,120662,65,120720,65,5034,65,5573,65,42222,65,94016,65,66208,65,119835,98,119887,98,119939,98,119991,98,120043,98,120095,98,120147,98,120199,98,120251,98,120303,98,120355,98,120407,98,120459,98,388,98,5071,98,5234,98,5551,98,65314,66,8492,66,119809,66,119861,66,119913,66,120017,66,120069,66,120121,66,120173,66,120225,66,120277,66,120329,66,120381,66,120433,66,42932,66,914,66,120489,66,120547,66,120605,66,120663,66,120721,66,5108,66,5623,66,42192,66,66178,66,66209,66,66305,66,65347,99,8573,99,119836,99,119888,99,119940,99,119992,99,120044,99,120096,99,120148,99,120200,99,120252,99,120304,99,120356,99,120408,99,120460,99,7428,99,1010,99,11429,99,43951,99,66621,99,128844,67,71922,67,71913,67,65315,67,8557,67,8450,67,8493,67,119810,67,119862,67,119914,67,119966,67,120018,67,120174,67,120226,67,120278,67,120330,67,120382,67,120434,67,1017,67,11428,67,5087,67,42202,67,66210,67,66306,67,66581,67,66844,67,8574,100,8518,100,119837,100,119889,100,119941,100,119993,100,120045,100,120097,100,120149,100,120201,100,120253,100,120305,100,120357,100,120409,100,120461,100,1281,100,5095,100,5231,100,42194,100,8558,68,8517,68,119811,68,119863,68,119915,68,119967,68,120019,68,120071,68,120123,68,120175,68,120227,68,120279,68,120331,68,120383,68,120435,68,5024,68,5598,68,5610,68,42195,68,8494,101,65349,101,8495,101,8519,101,119838,101,119890,101,119942,101,120046,101,120098,101,120150,101,120202,101,120254,101,120306,101,120358,101,120410,101,120462,101,43826,101,1213,101,8959,69,65317,69,8496,69,119812,69,119864,69,119916,69,120020,69,120072,69,120124,69,120176,69,120228,69,120280,69,120332,69,120384,69,120436,69,917,69,120492,69,120550,69,120608,69,120666,69,120724,69,11577,69,5036,69,42224,69,71846,69,71854,69,66182,69,119839,102,119891,102,119943,102,119995,102,120047,102,120099,102,120151,102,120203,102,120255,102,120307,102,120359,102,120411,102,120463,102,43829,102,42905,102,383,102,7837,102,1412,102,119315,70,8497,70,119813,70,119865,70,119917,70,120021,70,120073,70,120125,70,120177,70,120229,70,120281,70,120333,70,120385,70,120437,70,42904,70,988,70,120778,70,5556,70,42205,70,71874,70,71842,70,66183,70,66213,70,66853,70,65351,103,8458,103,119840,103,119892,103,119944,103,120048,103,120100,103,120152,103,120204,103,120256,103,120308,103,120360,103,120412,103,120464,103,609,103,7555,103,397,103,1409,103,119814,71,119866,71,119918,71,119970,71,120022,71,120074,71,120126,71,120178,71,120230,71,120282,71,120334,71,120386,71,120438,71,1292,71,5056,71,5107,71,42198,71,65352,104,8462,104,119841,104,119945,104,119997,104,120049,104,120101,104,120153,104,120205,104,120257,104,120309,104,120361,104,120413,104,120465,104,1211,104,1392,104,5058,104,65320,72,8459,72,8460,72,8461,72,119815,72,119867,72,119919,72,120023,72,120179,72,120231,72,120283,72,120335,72,120387,72,120439,72,919,72,120494,72,120552,72,120610,72,120668,72,120726,72,11406,72,5051,72,5500,72,42215,72,66255,72,731,105,9075,105,65353,105,8560,105,8505,105,8520,105,119842,105,119894,105,119946,105,119998,105,120050,105,120102,105,120154,105,120206,105,120258,105,120310,105,120362,105,120414,105,120466,105,120484,105,618,105,617,105,953,105,8126,105,890,105,120522,105,120580,105,120638,105,120696,105,120754,105,1110,105,42567,105,1231,105,43893,105,5029,105,71875,105,65354,106,8521,106,119843,106,119895,106,119947,106,119999,106,120051,106,120103,106,120155,106,120207,106,120259,106,120311,106,120363,106,120415,106,120467,106,1011,106,1112,106,65322,74,119817,74,119869,74,119921,74,119973,74,120025,74,120077,74,120129,74,120181,74,120233,74,120285,74,120337,74,120389,74,120441,74,42930,74,895,74,1032,74,5035,74,5261,74,42201,74,119844,107,119896,107,119948,107,120000,107,120052,107,120104,107,120156,107,120208,107,120260,107,120312,107,120364,107,120416,107,120468,107,8490,75,65323,75,119818,75,119870,75,119922,75,119974,75,120026,75,120078,75,120130,75,120182,75,120234,75,120286,75,120338,75,120390,75,120442,75,922,75,120497,75,120555,75,120613,75,120671,75,120729,75,11412,75,5094,75,5845,75,42199,75,66840,75,1472,108,8739,73,9213,73,65512,73,1633,108,1777,73,66336,108,125127,108,120783,73,120793,73,120803,73,120813,73,120823,73,130033,73,65321,73,8544,73,8464,73,8465,73,119816,73,119868,73,119920,73,120024,73,120128,73,120180,73,120232,73,120284,73,120336,73,120388,73,120440,73,65356,108,8572,73,8467,108,119845,108,119897,108,119949,108,120001,108,120053,108,120105,73,120157,73,120209,73,120261,73,120313,73,120365,73,120417,73,120469,73,448,73,120496,73,120554,73,120612,73,120670,73,120728,73,11410,73,1030,73,1216,73,1493,108,1503,108,1575,108,126464,108,126592,108,65166,108,65165,108,1994,108,11599,73,5825,73,42226,73,93992,73,66186,124,66313,124,119338,76,8556,76,8466,76,119819,76,119871,76,119923,76,120027,76,120079,76,120131,76,120183,76,120235,76,120287,76,120339,76,120391,76,120443,76,11472,76,5086,76,5290,76,42209,76,93974,76,71843,76,71858,76,66587,76,66854,76,65325,77,8559,77,8499,77,119820,77,119872,77,119924,77,120028,77,120080,77,120132,77,120184,77,120236,77,120288,77,120340,77,120392,77,120444,77,924,77,120499,77,120557,77,120615,77,120673,77,120731,77,1018,77,11416,77,5047,77,5616,77,5846,77,42207,77,66224,77,66321,77,119847,110,119899,110,119951,110,120003,110,120055,110,120107,110,120159,110,120211,110,120263,110,120315,110,120367,110,120419,110,120471,110,1400,110,1404,110,65326,78,8469,78,119821,78,119873,78,119925,78,119977,78,120029,78,120081,78,120185,78,120237,78,120289,78,120341,78,120393,78,120445,78,925,78,120500,78,120558,78,120616,78,120674,78,120732,78,11418,78,42208,78,66835,78,3074,111,3202,111,3330,111,3458,111,2406,111,2662,111,2790,111,3046,111,3174,111,3302,111,3430,111,3664,111,3792,111,4160,111,1637,111,1781,111,65359,111,8500,111,119848,111,119900,111,119952,111,120056,111,120108,111,120160,111,120212,111,120264,111,120316,111,120368,111,120420,111,120472,111,7439,111,7441,111,43837,111,959,111,120528,111,120586,111,120644,111,120702,111,120760,111,963,111,120532,111,120590,111,120648,111,120706,111,120764,111,11423,111,4351,111,1413,111,1505,111,1607,111,126500,111,126564,111,126596,111,65259,111,65260,111,65258,111,65257,111,1726,111,64428,111,64429,111,64427,111,64426,111,1729,111,64424,111,64425,111,64423,111,64422,111,1749,111,3360,111,4125,111,66794,111,71880,111,71895,111,66604,111,1984,79,2534,79,2918,79,12295,79,70864,79,71904,79,120782,79,120792,79,120802,79,120812,79,120822,79,130032,79,65327,79,119822,79,119874,79,119926,79,119978,79,120030,79,120082,79,120134,79,120186,79,120238,79,120290,79,120342,79,120394,79,120446,79,927,79,120502,79,120560,79,120618,79,120676,79,120734,79,11422,79,1365,79,11604,79,4816,79,2848,79,66754,79,42227,79,71861,79,66194,79,66219,79,66564,79,66838,79,9076,112,65360,112,119849,112,119901,112,119953,112,120005,112,120057,112,120109,112,120161,112,120213,112,120265,112,120317,112,120369,112,120421,112,120473,112,961,112,120530,112,120544,112,120588,112,120602,112,120646,112,120660,112,120704,112,120718,112,120762,112,120776,112,11427,112,65328,80,8473,80,119823,80,119875,80,119927,80,119979,80,120031,80,120083,80,120187,80,120239,80,120291,80,120343,80,120395,80,120447,80,929,80,120504,80,120562,80,120620,80,120678,80,120736,80,11426,80,5090,80,5229,80,42193,80,66197,80,119850,113,119902,113,119954,113,120006,113,120058,113,120110,113,120162,113,120214,113,120266,113,120318,113,120370,113,120422,113,120474,113,1307,113,1379,113,1382,113,8474,81,119824,81,119876,81,119928,81,119980,81,120032,81,120084,81,120188,81,120240,81,120292,81,120344,81,120396,81,120448,81,11605,81,119851,114,119903,114,119955,114,120007,114,120059,114,120111,114,120163,114,120215,114,120267,114,120319,114,120371,114,120423,114,120475,114,43847,114,43848,114,7462,114,11397,114,43905,114,119318,82,8475,82,8476,82,8477,82,119825,82,119877,82,119929,82,120033,82,120189,82,120241,82,120293,82,120345,82,120397,82,120449,82,422,82,5025,82,5074,82,66740,82,5511,82,42211,82,94005,82,65363,115,119852,115,119904,115,119956,115,120008,115,120060,115,120112,115,120164,115,120216,115,120268,115,120320,115,120372,115,120424,115,120476,115,42801,115,445,115,1109,115,43946,115,71873,115,66632,115,65331,83,119826,83,119878,83,119930,83,119982,83,120034,83,120086,83,120138,83,120190,83,120242,83,120294,83,120346,83,120398,83,120450,83,1029,83,1359,83,5077,83,5082,83,42210,83,94010,83,66198,83,66592,83,119853,116,119905,116,119957,116,120009,116,120061,116,120113,116,120165,116,120217,116,120269,116,120321,116,120373,116,120425,116,120477,116,8868,84,10201,84,128872,84,65332,84,119827,84,119879,84,119931,84,119983,84,120035,84,120087,84,120139,84,120191,84,120243,84,120295,84,120347,84,120399,84,120451,84,932,84,120507,84,120565,84,120623,84,120681,84,120739,84,11430,84,5026,84,42196,84,93962,84,71868,84,66199,84,66225,84,66325,84,119854,117,119906,117,119958,117,120010,117,120062,117,120114,117,120166,117,120218,117,120270,117,120322,117,120374,117,120426,117,120478,117,42911,117,7452,117,43854,117,43858,117,651,117,965,117,120534,117,120592,117,120650,117,120708,117,120766,117,1405,117,66806,117,71896,117,8746,85,8899,85,119828,85,119880,85,119932,85,119984,85,120036,85,120088,85,120140,85,120192,85,120244,85,120296,85,120348,85,120400,85,120452,85,1357,85,4608,85,66766,85,5196,85,42228,85,94018,85,71864,85,8744,118,8897,118,65366,118,8564,118,119855,118,119907,118,119959,118,120011,118,120063,118,120115,118,120167,118,120219,118,120271,118,120323,118,120375,118,120427,118,120479,118,7456,118,957,118,120526,118,120584,118,120642,118,120700,118,120758,118,1141,118,1496,118,71430,118,43945,118,71872,118,119309,86,1639,86,1783,86,8548,86,119829,86,119881,86,119933,86,119985,86,120037,86,120089,86,120141,86,120193,86,120245,86,120297,86,120349,86,120401,86,120453,86,1140,86,11576,86,5081,86,5167,86,42719,86,42214,86,93960,86,71840,86,66845,86,623,119,119856,119,119908,119,119960,119,120012,119,120064,119,120116,119,120168,119,120220,119,120272,119,120324,119,120376,119,120428,119,120480,119,7457,119,1121,119,1309,119,1377,119,71434,119,71438,119,71439,119,43907,119,71919,87,71910,87,119830,87,119882,87,119934,87,119986,87,120038,87,120090,87,120142,87,120194,87,120246,87,120298,87,120350,87,120402,87,120454,87,1308,87,5043,87,5076,87,42218,87,5742,120,10539,120,10540,120,10799,120,65368,120,8569,120,119857,120,119909,120,119961,120,120013,120,120065,120,120117,120,120169,120,120221,120,120273,120,120325,120,120377,120,120429,120,120481,120,5441,120,5501,120,5741,88,9587,88,66338,88,71916,88,65336,88,8553,88,119831,88,119883,88,119935,88,119987,88,120039,88,120091,88,120143,88,120195,88,120247,88,120299,88,120351,88,120403,88,120455,88,42931,88,935,88,120510,88,120568,88,120626,88,120684,88,120742,88,11436,88,11613,88,5815,88,42219,88,66192,88,66228,88,66327,88,66855,88,611,121,7564,121,65369,121,119858,121,119910,121,119962,121,120014,121,120066,121,120118,121,120170,121,120222,121,120274,121,120326,121,120378,121,120430,121,120482,121,655,121,7935,121,43866,121,947,121,8509,121,120516,121,120574,121,120632,121,120690,121,120748,121,1199,121,4327,121,71900,121,65337,89,119832,89,119884,89,119936,89,119988,89,120040,89,120092,89,120144,89,120196,89,120248,89,120300,89,120352,89,120404,89,120456,89,933,89,978,89,120508,89,120566,89,120624,89,120682,89,120740,89,11432,89,1198,89,5033,89,5053,89,42220,89,94019,89,71844,89,66226,89,119859,122,119911,122,119963,122,120015,122,120067,122,120119,122,120171,122,120223,122,120275,122,120327,122,120379,122,120431,122,120483,122,7458,122,43923,122,71876,122,66293,90,71909,90,65338,90,8484,90,8488,90,119833,90,119885,90,119937,90,119989,90,120041,90,120197,90,120249,90,120301,90,120353,90,120405,90,120457,90,918,90,120493,90,120551,90,120609,90,120667,90,120725,90,5059,90,42204,90,71849,90,65282,34,65284,36,65285,37,65286,38,65290,42,65291,43,65294,46,65295,47,65296,48,65297,49,65298,50,65299,51,65300,52,65301,53,65302,54,65303,55,65304,56,65305,57,65308,60,65309,61,65310,62,65312,64,65316,68,65318,70,65319,71,65324,76,65329,81,65330,82,65333,85,65334,86,65335,87,65343,95,65346,98,65348,100,65350,102,65355,107,65357,109,65358,110,65361,113,65362,114,65364,116,65365,117,65367,119,65370,122,65371,123,65373,125,119846,109],"_default":[160,32,8211,45,65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"cs":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"de":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"es":[8211,45,65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"fr":[65374,126,65306,58,65281,33,8216,96,8245,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"it":[160,32,8211,45,65374,126,65306,58,65281,33,8216,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"ja":[8211,45,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65292,44,65307,59],"ko":[8211,45,65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"pl":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"pt-BR":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"qps-ploc":[160,32,8211,45,65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"ru":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,305,105,921,73,1009,112,215,120,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"tr":[160,32,8211,45,65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"zh-hans":[65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65288,40,65289,41],"zh-hant":[8211,45,65374,126,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65307,59]}'));
ye.cache = new Ta((e) => {
  function t(u) {
    const f = /* @__PURE__ */ new Map();
    for (let h = 0; h < u.length; h += 2)
      f.set(u[h], u[h + 1]);
    return f;
  }
  function r(u, f) {
    const h = new Map(u);
    for (const [d, g] of f)
      h.set(d, g);
    return h;
  }
  function n(u, f) {
    if (!u)
      return f;
    const h = /* @__PURE__ */ new Map();
    for (const [d, g] of u)
      f.has(d) && h.set(d, g);
    return h;
  }
  const i = ks.ambiguousCharacterData.value;
  let s = e.filter((u) => !u.startsWith("_") && u in i);
  s.length === 0 && (s = ["_default"]);
  let a;
  for (const u of s) {
    const f = t(i[u]);
    a = n(a, f);
  }
  const o = t(i._common), l = r(o, a);
  return new ye(l);
});
ye._locales = new Cs(() => Object.keys(ye.ambiguousCharacterData.value).filter((e) => !e.startsWith("_")));
class Je {
  static getRawData() {
    return JSON.parse("[9,10,11,12,13,32,127,160,173,847,1564,4447,4448,6068,6069,6155,6156,6157,6158,7355,7356,8192,8193,8194,8195,8196,8197,8198,8199,8200,8201,8202,8203,8204,8205,8206,8207,8234,8235,8236,8237,8238,8239,8287,8288,8289,8290,8291,8292,8293,8294,8295,8296,8297,8298,8299,8300,8301,8302,8303,10240,12288,12644,65024,65025,65026,65027,65028,65029,65030,65031,65032,65033,65034,65035,65036,65037,65038,65039,65279,65440,65520,65521,65522,65523,65524,65525,65526,65527,65528,65532,78844,119155,119156,119157,119158,119159,119160,119161,119162,917504,917505,917506,917507,917508,917509,917510,917511,917512,917513,917514,917515,917516,917517,917518,917519,917520,917521,917522,917523,917524,917525,917526,917527,917528,917529,917530,917531,917532,917533,917534,917535,917536,917537,917538,917539,917540,917541,917542,917543,917544,917545,917546,917547,917548,917549,917550,917551,917552,917553,917554,917555,917556,917557,917558,917559,917560,917561,917562,917563,917564,917565,917566,917567,917568,917569,917570,917571,917572,917573,917574,917575,917576,917577,917578,917579,917580,917581,917582,917583,917584,917585,917586,917587,917588,917589,917590,917591,917592,917593,917594,917595,917596,917597,917598,917599,917600,917601,917602,917603,917604,917605,917606,917607,917608,917609,917610,917611,917612,917613,917614,917615,917616,917617,917618,917619,917620,917621,917622,917623,917624,917625,917626,917627,917628,917629,917630,917631,917760,917761,917762,917763,917764,917765,917766,917767,917768,917769,917770,917771,917772,917773,917774,917775,917776,917777,917778,917779,917780,917781,917782,917783,917784,917785,917786,917787,917788,917789,917790,917791,917792,917793,917794,917795,917796,917797,917798,917799,917800,917801,917802,917803,917804,917805,917806,917807,917808,917809,917810,917811,917812,917813,917814,917815,917816,917817,917818,917819,917820,917821,917822,917823,917824,917825,917826,917827,917828,917829,917830,917831,917832,917833,917834,917835,917836,917837,917838,917839,917840,917841,917842,917843,917844,917845,917846,917847,917848,917849,917850,917851,917852,917853,917854,917855,917856,917857,917858,917859,917860,917861,917862,917863,917864,917865,917866,917867,917868,917869,917870,917871,917872,917873,917874,917875,917876,917877,917878,917879,917880,917881,917882,917883,917884,917885,917886,917887,917888,917889,917890,917891,917892,917893,917894,917895,917896,917897,917898,917899,917900,917901,917902,917903,917904,917905,917906,917907,917908,917909,917910,917911,917912,917913,917914,917915,917916,917917,917918,917919,917920,917921,917922,917923,917924,917925,917926,917927,917928,917929,917930,917931,917932,917933,917934,917935,917936,917937,917938,917939,917940,917941,917942,917943,917944,917945,917946,917947,917948,917949,917950,917951,917952,917953,917954,917955,917956,917957,917958,917959,917960,917961,917962,917963,917964,917965,917966,917967,917968,917969,917970,917971,917972,917973,917974,917975,917976,917977,917978,917979,917980,917981,917982,917983,917984,917985,917986,917987,917988,917989,917990,917991,917992,917993,917994,917995,917996,917997,917998,917999]");
  }
  static getData() {
    return this._data || (this._data = new Set(Je.getRawData())), this._data;
  }
  static isInvisibleCharacter(t) {
    return Je.getData().has(t);
  }
  static get codePoints() {
    return Je.getData();
  }
}
Je._data = void 0;
const Ba = "$initialize";
class $a {
  constructor(t, r, n, i) {
    this.vsWorker = t, this.req = r, this.method = n, this.args = i, this.type = 0;
  }
}
class wn {
  constructor(t, r, n, i) {
    this.vsWorker = t, this.seq = r, this.res = n, this.err = i, this.type = 1;
  }
}
class qa {
  constructor(t, r, n, i) {
    this.vsWorker = t, this.req = r, this.eventName = n, this.arg = i, this.type = 2;
  }
}
class Wa {
  constructor(t, r, n) {
    this.vsWorker = t, this.req = r, this.event = n, this.type = 3;
  }
}
class Ha {
  constructor(t, r) {
    this.vsWorker = t, this.req = r, this.type = 4;
  }
}
class za {
  constructor(t) {
    this._workerId = -1, this._handler = t, this._lastSentReq = 0, this._pendingReplies = /* @__PURE__ */ Object.create(null), this._pendingEmitters = /* @__PURE__ */ new Map(), this._pendingEvents = /* @__PURE__ */ new Map();
  }
  setWorkerId(t) {
    this._workerId = t;
  }
  sendMessage(t, r) {
    const n = String(++this._lastSentReq);
    return new Promise((i, s) => {
      this._pendingReplies[n] = {
        resolve: i,
        reject: s
      }, this._send(new $a(this._workerId, n, t, r));
    });
  }
  listen(t, r) {
    let n = null;
    const i = new Me({
      onWillAddFirstListener: () => {
        n = String(++this._lastSentReq), this._pendingEmitters.set(n, i), this._send(new qa(this._workerId, n, t, r));
      },
      onDidRemoveLastListener: () => {
        this._pendingEmitters.delete(n), this._send(new Ha(this._workerId, n)), n = null;
      }
    });
    return i.event;
  }
  handleMessage(t) {
    !t || !t.vsWorker || this._workerId !== -1 && t.vsWorker !== this._workerId || this._handleMessage(t);
  }
  _handleMessage(t) {
    switch (t.type) {
      case 1:
        return this._handleReplyMessage(t);
      case 0:
        return this._handleRequestMessage(t);
      case 2:
        return this._handleSubscribeEventMessage(t);
      case 3:
        return this._handleEventMessage(t);
      case 4:
        return this._handleUnsubscribeEventMessage(t);
    }
  }
  _handleReplyMessage(t) {
    if (!this._pendingReplies[t.seq]) {
      console.warn("Got reply to unknown seq");
      return;
    }
    const r = this._pendingReplies[t.seq];
    if (delete this._pendingReplies[t.seq], t.err) {
      let n = t.err;
      t.err.$isError && (n = new Error(), n.name = t.err.name, n.message = t.err.message, n.stack = t.err.stack), r.reject(n);
      return;
    }
    r.resolve(t.res);
  }
  _handleRequestMessage(t) {
    const r = t.req;
    this._handler.handleMessage(t.method, t.args).then((i) => {
      this._send(new wn(this._workerId, r, i, void 0));
    }, (i) => {
      i.detail instanceof Error && (i.detail = bn(i.detail)), this._send(new wn(this._workerId, r, void 0, bn(i)));
    });
  }
  _handleSubscribeEventMessage(t) {
    const r = t.req, n = this._handler.handleEvent(t.eventName, t.arg)((i) => {
      this._send(new Wa(this._workerId, r, i));
    });
    this._pendingEvents.set(r, n);
  }
  _handleEventMessage(t) {
    if (!this._pendingEmitters.has(t.req)) {
      console.warn("Got event for unknown req");
      return;
    }
    this._pendingEmitters.get(t.req).fire(t.event);
  }
  _handleUnsubscribeEventMessage(t) {
    if (!this._pendingEvents.has(t.req)) {
      console.warn("Got unsubscribe for unknown req");
      return;
    }
    this._pendingEvents.get(t.req).dispose(), this._pendingEvents.delete(t.req);
  }
  _send(t) {
    const r = [];
    if (t.type === 0)
      for (let n = 0; n < t.args.length; n++)
        t.args[n] instanceof ArrayBuffer && r.push(t.args[n]);
    else
      t.type === 1 && t.res instanceof ArrayBuffer && r.push(t.res);
    this._handler.sendMessage(t, r);
  }
}
function Ms(e) {
  return e[0] === "o" && e[1] === "n" && Es(e.charCodeAt(2));
}
function Ts(e) {
  return /^onDynamic/.test(e) && Es(e.charCodeAt(9));
}
function Ga(e, t, r) {
  const n = (a) => function() {
    const o = Array.prototype.slice.call(arguments, 0);
    return t(a, o);
  }, i = (a) => function(o) {
    return r(a, o);
  }, s = {};
  for (const a of e) {
    if (Ts(a)) {
      s[a] = i(a);
      continue;
    }
    if (Ms(a)) {
      s[a] = r(a, void 0);
      continue;
    }
    s[a] = n(a);
  }
  return s;
}
class Ja {
  constructor(t, r) {
    this._requestHandlerFactory = r, this._requestHandler = null, this._protocol = new za({
      sendMessage: (n, i) => {
        t(n, i);
      },
      handleMessage: (n, i) => this._handleMessage(n, i),
      handleEvent: (n, i) => this._handleEvent(n, i)
    });
  }
  onmessage(t) {
    this._protocol.handleMessage(t);
  }
  _handleMessage(t, r) {
    if (t === Ba)
      return this.initialize(r[0], r[1], r[2], r[3]);
    if (!this._requestHandler || typeof this._requestHandler[t] != "function")
      return Promise.reject(new Error("Missing requestHandler or method: " + t));
    try {
      return Promise.resolve(this._requestHandler[t].apply(this._requestHandler, r));
    } catch (n) {
      return Promise.reject(n);
    }
  }
  _handleEvent(t, r) {
    if (!this._requestHandler)
      throw new Error("Missing requestHandler");
    if (Ts(t)) {
      const n = this._requestHandler[t].call(this._requestHandler, r);
      if (typeof n != "function")
        throw new Error(`Missing dynamic event ${t} on request handler.`);
      return n;
    }
    if (Ms(t)) {
      const n = this._requestHandler[t];
      if (typeof n != "function")
        throw new Error(`Missing event ${t} on request handler.`);
      return n;
    }
    throw new Error(`Malformed event name ${t}`);
  }
  initialize(t, r, n, i) {
    this._protocol.setWorkerId(t);
    const o = Ga(i, (l, u) => this._protocol.sendMessage(l, u), (l, u) => this._protocol.listen(l, u));
    return this._requestHandlerFactory ? (this._requestHandler = this._requestHandlerFactory(o), Promise.resolve(Mr(this._requestHandler))) : (r && (typeof r.baseUrl < "u" && delete r.baseUrl, typeof r.paths < "u" && typeof r.paths.vs < "u" && delete r.paths.vs, typeof r.trustedTypesPolicy !== void 0 && delete r.trustedTypesPolicy, r.catchError = !0, globalThis.require.config(r)), new Promise((l, u) => {
      const f = globalThis.require;
      f([n], (h) => {
        if (this._requestHandler = h.create(o), !this._requestHandler) {
          u(new Error("No RequestHandler!"));
          return;
        }
        l(Mr(this._requestHandler));
      }, u);
    }));
  }
}
class We {
  /**
   * Constructs a new DiffChange with the given sequence information
   * and content.
   */
  constructor(t, r, n, i) {
    this.originalStart = t, this.originalLength = r, this.modifiedStart = n, this.modifiedLength = i;
  }
  /**
   * The end point (exclusive) of the change in the original sequence.
   */
  getOriginalEnd() {
    return this.originalStart + this.originalLength;
  }
  /**
   * The end point (exclusive) of the change in the modified sequence.
   */
  getModifiedEnd() {
    return this.modifiedStart + this.modifiedLength;
  }
}
function _n(e, t) {
  return (t << 5) - t + e | 0;
}
function Xa(e, t) {
  t = _n(149417, t);
  for (let r = 0, n = e.length; r < n; r++)
    t = _n(e.charCodeAt(r), t);
  return t;
}
class Sn {
  constructor(t) {
    this.source = t;
  }
  getElements() {
    const t = this.source, r = new Int32Array(t.length);
    for (let n = 0, i = t.length; n < i; n++)
      r[n] = t.charCodeAt(n);
    return r;
  }
}
function Za(e, t, r) {
  return new ze(new Sn(e), new Sn(t)).ComputeDiff(r).changes;
}
class it {
  static Assert(t, r) {
    if (!t)
      throw new Error(r);
  }
}
class st {
  /**
   * Copies a range of elements from an Array starting at the specified source index and pastes
   * them to another Array starting at the specified destination index. The length and the indexes
   * are specified as 64-bit integers.
   * sourceArray:
   *		The Array that contains the data to copy.
   * sourceIndex:
   *		A 64-bit integer that represents the index in the sourceArray at which copying begins.
   * destinationArray:
   *		The Array that receives the data.
   * destinationIndex:
   *		A 64-bit integer that represents the index in the destinationArray at which storing begins.
   * length:
   *		A 64-bit integer that represents the number of elements to copy.
   */
  static Copy(t, r, n, i, s) {
    for (let a = 0; a < s; a++)
      n[i + a] = t[r + a];
  }
  static Copy2(t, r, n, i, s) {
    for (let a = 0; a < s; a++)
      n[i + a] = t[r + a];
  }
}
class An {
  /**
   * Constructs a new DiffChangeHelper for the given DiffSequences.
   */
  constructor() {
    this.m_changes = [], this.m_originalStart = 1073741824, this.m_modifiedStart = 1073741824, this.m_originalCount = 0, this.m_modifiedCount = 0;
  }
  /**
   * Marks the beginning of the next change in the set of differences.
   */
  MarkNextChange() {
    (this.m_originalCount > 0 || this.m_modifiedCount > 0) && this.m_changes.push(new We(this.m_originalStart, this.m_originalCount, this.m_modifiedStart, this.m_modifiedCount)), this.m_originalCount = 0, this.m_modifiedCount = 0, this.m_originalStart = 1073741824, this.m_modifiedStart = 1073741824;
  }
  /**
   * Adds the original element at the given position to the elements
   * affected by the current change. The modified index gives context
   * to the change position with respect to the original sequence.
   * @param originalIndex The index of the original element to add.
   * @param modifiedIndex The index of the modified element that provides corresponding position in the modified sequence.
   */
  AddOriginalElement(t, r) {
    this.m_originalStart = Math.min(this.m_originalStart, t), this.m_modifiedStart = Math.min(this.m_modifiedStart, r), this.m_originalCount++;
  }
  /**
   * Adds the modified element at the given position to the elements
   * affected by the current change. The original index gives context
   * to the change position with respect to the modified sequence.
   * @param originalIndex The index of the original element that provides corresponding position in the original sequence.
   * @param modifiedIndex The index of the modified element to add.
   */
  AddModifiedElement(t, r) {
    this.m_originalStart = Math.min(this.m_originalStart, t), this.m_modifiedStart = Math.min(this.m_modifiedStart, r), this.m_modifiedCount++;
  }
  /**
   * Retrieves all of the changes marked by the class.
   */
  getChanges() {
    return (this.m_originalCount > 0 || this.m_modifiedCount > 0) && this.MarkNextChange(), this.m_changes;
  }
  /**
   * Retrieves all of the changes marked by the class in the reverse order
   */
  getReverseChanges() {
    return (this.m_originalCount > 0 || this.m_modifiedCount > 0) && this.MarkNextChange(), this.m_changes.reverse(), this.m_changes;
  }
}
class ze {
  /**
   * Constructs the DiffFinder
   */
  constructor(t, r, n = null) {
    this.ContinueProcessingPredicate = n, this._originalSequence = t, this._modifiedSequence = r;
    const [i, s, a] = ze._getElements(t), [o, l, u] = ze._getElements(r);
    this._hasStrings = a && u, this._originalStringElements = i, this._originalElementsOrHash = s, this._modifiedStringElements = o, this._modifiedElementsOrHash = l, this.m_forwardHistory = [], this.m_reverseHistory = [];
  }
  static _isStringArray(t) {
    return t.length > 0 && typeof t[0] == "string";
  }
  static _getElements(t) {
    const r = t.getElements();
    if (ze._isStringArray(r)) {
      const n = new Int32Array(r.length);
      for (let i = 0, s = r.length; i < s; i++)
        n[i] = Xa(r[i], 0);
      return [r, n, !0];
    }
    return r instanceof Int32Array ? [[], r, !1] : [[], new Int32Array(r), !1];
  }
  ElementsAreEqual(t, r) {
    return this._originalElementsOrHash[t] !== this._modifiedElementsOrHash[r] ? !1 : this._hasStrings ? this._originalStringElements[t] === this._modifiedStringElements[r] : !0;
  }
  ElementsAreStrictEqual(t, r) {
    if (!this.ElementsAreEqual(t, r))
      return !1;
    const n = ze._getStrictElement(this._originalSequence, t), i = ze._getStrictElement(this._modifiedSequence, r);
    return n === i;
  }
  static _getStrictElement(t, r) {
    return typeof t.getStrictElement == "function" ? t.getStrictElement(r) : null;
  }
  OriginalElementsAreEqual(t, r) {
    return this._originalElementsOrHash[t] !== this._originalElementsOrHash[r] ? !1 : this._hasStrings ? this._originalStringElements[t] === this._originalStringElements[r] : !0;
  }
  ModifiedElementsAreEqual(t, r) {
    return this._modifiedElementsOrHash[t] !== this._modifiedElementsOrHash[r] ? !1 : this._hasStrings ? this._modifiedStringElements[t] === this._modifiedStringElements[r] : !0;
  }
  ComputeDiff(t) {
    return this._ComputeDiff(0, this._originalElementsOrHash.length - 1, 0, this._modifiedElementsOrHash.length - 1, t);
  }
  /**
   * Computes the differences between the original and modified input
   * sequences on the bounded range.
   * @returns An array of the differences between the two input sequences.
   */
  _ComputeDiff(t, r, n, i, s) {
    const a = [!1];
    let o = this.ComputeDiffRecursive(t, r, n, i, a);
    return s && (o = this.PrettifyChanges(o)), {
      quitEarly: a[0],
      changes: o
    };
  }
  /**
   * Private helper method which computes the differences on the bounded range
   * recursively.
   * @returns An array of the differences between the two input sequences.
   */
  ComputeDiffRecursive(t, r, n, i, s) {
    for (s[0] = !1; t <= r && n <= i && this.ElementsAreEqual(t, n); )
      t++, n++;
    for (; r >= t && i >= n && this.ElementsAreEqual(r, i); )
      r--, i--;
    if (t > r || n > i) {
      let h;
      return n <= i ? (it.Assert(t === r + 1, "originalStart should only be one more than originalEnd"), h = [
        new We(t, 0, n, i - n + 1)
      ]) : t <= r ? (it.Assert(n === i + 1, "modifiedStart should only be one more than modifiedEnd"), h = [
        new We(t, r - t + 1, n, 0)
      ]) : (it.Assert(t === r + 1, "originalStart should only be one more than originalEnd"), it.Assert(n === i + 1, "modifiedStart should only be one more than modifiedEnd"), h = []), h;
    }
    const a = [0], o = [0], l = this.ComputeRecursionPoint(t, r, n, i, a, o, s), u = a[0], f = o[0];
    if (l !== null)
      return l;
    if (!s[0]) {
      const h = this.ComputeDiffRecursive(t, u, n, f, s);
      let d = [];
      return s[0] ? d = [
        new We(u + 1, r - (u + 1) + 1, f + 1, i - (f + 1) + 1)
      ] : d = this.ComputeDiffRecursive(u + 1, r, f + 1, i, s), this.ConcatenateChanges(h, d);
    }
    return [
      new We(t, r - t + 1, n, i - n + 1)
    ];
  }
  WALKTRACE(t, r, n, i, s, a, o, l, u, f, h, d, g, p, m, v, y, x) {
    let b = null, S = null, M = new An(), A = r, C = n, _ = g[0] - v[0] - i, w = -1073741824, N = this.m_forwardHistory.length - 1;
    do {
      const k = _ + t;
      k === A || k < C && u[k - 1] < u[k + 1] ? (h = u[k + 1], p = h - _ - i, h < w && M.MarkNextChange(), w = h, M.AddModifiedElement(h + 1, p), _ = k + 1 - t) : (h = u[k - 1] + 1, p = h - _ - i, h < w && M.MarkNextChange(), w = h - 1, M.AddOriginalElement(h, p + 1), _ = k - 1 - t), N >= 0 && (u = this.m_forwardHistory[N], t = u[0], A = 1, C = u.length - 1);
    } while (--N >= -1);
    if (b = M.getReverseChanges(), x[0]) {
      let k = g[0] + 1, L = v[0] + 1;
      if (b !== null && b.length > 0) {
        const F = b[b.length - 1];
        k = Math.max(k, F.getOriginalEnd()), L = Math.max(L, F.getModifiedEnd());
      }
      S = [
        new We(k, d - k + 1, L, m - L + 1)
      ];
    } else {
      M = new An(), A = a, C = o, _ = g[0] - v[0] - l, w = 1073741824, N = y ? this.m_reverseHistory.length - 1 : this.m_reverseHistory.length - 2;
      do {
        const k = _ + s;
        k === A || k < C && f[k - 1] >= f[k + 1] ? (h = f[k + 1] - 1, p = h - _ - l, h > w && M.MarkNextChange(), w = h + 1, M.AddOriginalElement(h + 1, p + 1), _ = k + 1 - s) : (h = f[k - 1], p = h - _ - l, h > w && M.MarkNextChange(), w = h, M.AddModifiedElement(h + 1, p + 1), _ = k - 1 - s), N >= 0 && (f = this.m_reverseHistory[N], s = f[0], A = 1, C = f.length - 1);
      } while (--N >= -1);
      S = M.getChanges();
    }
    return this.ConcatenateChanges(b, S);
  }
  /**
   * Given the range to compute the diff on, this method finds the point:
   * (midOriginal, midModified)
   * that exists in the middle of the LCS of the two sequences and
   * is the point at which the LCS problem may be broken down recursively.
   * This method will try to keep the LCS trace in memory. If the LCS recursion
   * point is calculated and the full trace is available in memory, then this method
   * will return the change list.
   * @param originalStart The start bound of the original sequence range
   * @param originalEnd The end bound of the original sequence range
   * @param modifiedStart The start bound of the modified sequence range
   * @param modifiedEnd The end bound of the modified sequence range
   * @param midOriginal The middle point of the original sequence range
   * @param midModified The middle point of the modified sequence range
   * @returns The diff changes, if available, otherwise null
   */
  ComputeRecursionPoint(t, r, n, i, s, a, o) {
    let l = 0, u = 0, f = 0, h = 0, d = 0, g = 0;
    t--, n--, s[0] = 0, a[0] = 0, this.m_forwardHistory = [], this.m_reverseHistory = [];
    const p = r - t + (i - n), m = p + 1, v = new Int32Array(m), y = new Int32Array(m), x = i - n, b = r - t, S = t - n, M = r - i, C = (b - x) % 2 === 0;
    v[x] = t, y[b] = r, o[0] = !1;
    for (let _ = 1; _ <= p / 2 + 1; _++) {
      let w = 0, N = 0;
      f = this.ClipDiagonalBound(x - _, _, x, m), h = this.ClipDiagonalBound(x + _, _, x, m);
      for (let L = f; L <= h; L += 2) {
        L === f || L < h && v[L - 1] < v[L + 1] ? l = v[L + 1] : l = v[L - 1] + 1, u = l - (L - x) - S;
        const F = l;
        for (; l < r && u < i && this.ElementsAreEqual(l + 1, u + 1); )
          l++, u++;
        if (v[L] = l, l + u > w + N && (w = l, N = u), !C && Math.abs(L - b) <= _ - 1 && l >= y[L])
          return s[0] = l, a[0] = u, F <= y[L] && 1447 > 0 && _ <= 1447 + 1 ? this.WALKTRACE(x, f, h, S, b, d, g, M, v, y, l, r, s, u, i, a, C, o) : null;
      }
      const k = (w - t + (N - n) - _) / 2;
      if (this.ContinueProcessingPredicate !== null && !this.ContinueProcessingPredicate(w, k))
        return o[0] = !0, s[0] = w, a[0] = N, k > 0 && 1447 > 0 && _ <= 1447 + 1 ? this.WALKTRACE(x, f, h, S, b, d, g, M, v, y, l, r, s, u, i, a, C, o) : (t++, n++, [
          new We(t, r - t + 1, n, i - n + 1)
        ]);
      d = this.ClipDiagonalBound(b - _, _, b, m), g = this.ClipDiagonalBound(b + _, _, b, m);
      for (let L = d; L <= g; L += 2) {
        L === d || L < g && y[L - 1] >= y[L + 1] ? l = y[L + 1] - 1 : l = y[L - 1], u = l - (L - b) - M;
        const F = l;
        for (; l > t && u > n && this.ElementsAreEqual(l, u); )
          l--, u--;
        if (y[L] = l, C && Math.abs(L - x) <= _ && l <= v[L])
          return s[0] = l, a[0] = u, F >= v[L] && 1447 > 0 && _ <= 1447 + 1 ? this.WALKTRACE(x, f, h, S, b, d, g, M, v, y, l, r, s, u, i, a, C, o) : null;
      }
      if (_ <= 1447) {
        let L = new Int32Array(h - f + 2);
        L[0] = x - f + 1, st.Copy2(v, f, L, 1, h - f + 1), this.m_forwardHistory.push(L), L = new Int32Array(g - d + 2), L[0] = b - d + 1, st.Copy2(y, d, L, 1, g - d + 1), this.m_reverseHistory.push(L);
      }
    }
    return this.WALKTRACE(x, f, h, S, b, d, g, M, v, y, l, r, s, u, i, a, C, o);
  }
  /**
   * Shifts the given changes to provide a more intuitive diff.
   * While the first element in a diff matches the first element after the diff,
   * we shift the diff down.
   *
   * @param changes The list of changes to shift
   * @returns The shifted changes
   */
  PrettifyChanges(t) {
    for (let r = 0; r < t.length; r++) {
      const n = t[r], i = r < t.length - 1 ? t[r + 1].originalStart : this._originalElementsOrHash.length, s = r < t.length - 1 ? t[r + 1].modifiedStart : this._modifiedElementsOrHash.length, a = n.originalLength > 0, o = n.modifiedLength > 0;
      for (; n.originalStart + n.originalLength < i && n.modifiedStart + n.modifiedLength < s && (!a || this.OriginalElementsAreEqual(n.originalStart, n.originalStart + n.originalLength)) && (!o || this.ModifiedElementsAreEqual(n.modifiedStart, n.modifiedStart + n.modifiedLength)); ) {
        const u = this.ElementsAreStrictEqual(n.originalStart, n.modifiedStart);
        if (this.ElementsAreStrictEqual(n.originalStart + n.originalLength, n.modifiedStart + n.modifiedLength) && !u)
          break;
        n.originalStart++, n.modifiedStart++;
      }
      const l = [null];
      if (r < t.length - 1 && this.ChangesOverlap(t[r], t[r + 1], l)) {
        t[r] = l[0], t.splice(r + 1, 1), r--;
        continue;
      }
    }
    for (let r = t.length - 1; r >= 0; r--) {
      const n = t[r];
      let i = 0, s = 0;
      if (r > 0) {
        const h = t[r - 1];
        i = h.originalStart + h.originalLength, s = h.modifiedStart + h.modifiedLength;
      }
      const a = n.originalLength > 0, o = n.modifiedLength > 0;
      let l = 0, u = this._boundaryScore(n.originalStart, n.originalLength, n.modifiedStart, n.modifiedLength);
      for (let h = 1; ; h++) {
        const d = n.originalStart - h, g = n.modifiedStart - h;
        if (d < i || g < s || a && !this.OriginalElementsAreEqual(d, d + n.originalLength) || o && !this.ModifiedElementsAreEqual(g, g + n.modifiedLength))
          break;
        const m = (d === i && g === s ? 5 : 0) + this._boundaryScore(d, n.originalLength, g, n.modifiedLength);
        m > u && (u = m, l = h);
      }
      n.originalStart -= l, n.modifiedStart -= l;
      const f = [null];
      if (r > 0 && this.ChangesOverlap(t[r - 1], t[r], f)) {
        t[r - 1] = f[0], t.splice(r, 1), r++;
        continue;
      }
    }
    if (this._hasStrings)
      for (let r = 1, n = t.length; r < n; r++) {
        const i = t[r - 1], s = t[r], a = s.originalStart - i.originalStart - i.originalLength, o = i.originalStart, l = s.originalStart + s.originalLength, u = l - o, f = i.modifiedStart, h = s.modifiedStart + s.modifiedLength, d = h - f;
        if (a < 5 && u < 20 && d < 20) {
          const g = this._findBetterContiguousSequence(o, u, f, d, a);
          if (g) {
            const [p, m] = g;
            (p !== i.originalStart + i.originalLength || m !== i.modifiedStart + i.modifiedLength) && (i.originalLength = p - i.originalStart, i.modifiedLength = m - i.modifiedStart, s.originalStart = p + a, s.modifiedStart = m + a, s.originalLength = l - s.originalStart, s.modifiedLength = h - s.modifiedStart);
          }
        }
      }
    return t;
  }
  _findBetterContiguousSequence(t, r, n, i, s) {
    if (r < s || i < s)
      return null;
    const a = t + r - s + 1, o = n + i - s + 1;
    let l = 0, u = 0, f = 0;
    for (let h = t; h < a; h++)
      for (let d = n; d < o; d++) {
        const g = this._contiguousSequenceScore(h, d, s);
        g > 0 && g > l && (l = g, u = h, f = d);
      }
    return l > 0 ? [u, f] : null;
  }
  _contiguousSequenceScore(t, r, n) {
    let i = 0;
    for (let s = 0; s < n; s++) {
      if (!this.ElementsAreEqual(t + s, r + s))
        return 0;
      i += this._originalStringElements[t + s].length;
    }
    return i;
  }
  _OriginalIsBoundary(t) {
    return t <= 0 || t >= this._originalElementsOrHash.length - 1 ? !0 : this._hasStrings && /^\s*$/.test(this._originalStringElements[t]);
  }
  _OriginalRegionIsBoundary(t, r) {
    if (this._OriginalIsBoundary(t) || this._OriginalIsBoundary(t - 1))
      return !0;
    if (r > 0) {
      const n = t + r;
      if (this._OriginalIsBoundary(n - 1) || this._OriginalIsBoundary(n))
        return !0;
    }
    return !1;
  }
  _ModifiedIsBoundary(t) {
    return t <= 0 || t >= this._modifiedElementsOrHash.length - 1 ? !0 : this._hasStrings && /^\s*$/.test(this._modifiedStringElements[t]);
  }
  _ModifiedRegionIsBoundary(t, r) {
    if (this._ModifiedIsBoundary(t) || this._ModifiedIsBoundary(t - 1))
      return !0;
    if (r > 0) {
      const n = t + r;
      if (this._ModifiedIsBoundary(n - 1) || this._ModifiedIsBoundary(n))
        return !0;
    }
    return !1;
  }
  _boundaryScore(t, r, n, i) {
    const s = this._OriginalRegionIsBoundary(t, r) ? 1 : 0, a = this._ModifiedRegionIsBoundary(n, i) ? 1 : 0;
    return s + a;
  }
  /**
   * Concatenates the two input DiffChange lists and returns the resulting
   * list.
   * @param The left changes
   * @param The right changes
   * @returns The concatenated list
   */
  ConcatenateChanges(t, r) {
    const n = [];
    if (t.length === 0 || r.length === 0)
      return r.length > 0 ? r : t;
    if (this.ChangesOverlap(t[t.length - 1], r[0], n)) {
      const i = new Array(t.length + r.length - 1);
      return st.Copy(t, 0, i, 0, t.length - 1), i[t.length - 1] = n[0], st.Copy(r, 1, i, t.length, r.length - 1), i;
    } else {
      const i = new Array(t.length + r.length);
      return st.Copy(t, 0, i, 0, t.length), st.Copy(r, 0, i, t.length, r.length), i;
    }
  }
  /**
   * Returns true if the two changes overlap and can be merged into a single
   * change
   * @param left The left change
   * @param right The right change
   * @param mergedChange The merged change if the two overlap, null otherwise
   * @returns True if the two changes overlap
   */
  ChangesOverlap(t, r, n) {
    if (it.Assert(t.originalStart <= r.originalStart, "Left change is not less than or equal to right change"), it.Assert(t.modifiedStart <= r.modifiedStart, "Left change is not less than or equal to right change"), t.originalStart + t.originalLength >= r.originalStart || t.modifiedStart + t.modifiedLength >= r.modifiedStart) {
      const i = t.originalStart;
      let s = t.originalLength;
      const a = t.modifiedStart;
      let o = t.modifiedLength;
      return t.originalStart + t.originalLength >= r.originalStart && (s = r.originalStart + r.originalLength - t.originalStart), t.modifiedStart + t.modifiedLength >= r.modifiedStart && (o = r.modifiedStart + r.modifiedLength - t.modifiedStart), n[0] = new We(i, s, a, o), !0;
    } else
      return n[0] = null, !1;
  }
  /**
   * Helper method used to clip a diagonal index to the range of valid
   * diagonals. This also decides whether or not the diagonal index,
   * if it exceeds the boundary, should be clipped to the boundary or clipped
   * one inside the boundary depending on the Even/Odd status of the boundary
   * and numDifferences.
   * @param diagonal The index of the diagonal to clip.
   * @param numDifferences The current number of differences being iterated upon.
   * @param diagonalBaseIndex The base reference diagonal.
   * @param numDiagonals The total number of diagonals.
   * @returns The clipped diagonal index.
   */
  ClipDiagonalBound(t, r, n, i) {
    if (t >= 0 && t < i)
      return t;
    const s = n, a = i - n - 1, o = r % 2 === 0;
    if (t < 0) {
      const l = s % 2 === 0;
      return o === l ? 0 : 1;
    } else {
      const l = a % 2 === 0;
      return o === l ? i - 1 : i - 2;
    }
  }
}
let Qe;
if (typeof he.vscode < "u" && typeof he.vscode.process < "u") {
  const e = he.vscode.process;
  Qe = {
    get platform() {
      return e.platform;
    },
    get arch() {
      return e.arch;
    },
    get env() {
      return e.env;
    },
    cwd() {
      return e.cwd();
    }
  };
} else
  typeof process < "u" ? Qe = {
    get platform() {
      return process.platform;
    },
    get arch() {
      return process.arch;
    },
    get env() {
      return process.env;
    },
    cwd() {
      return process.env.VSCODE_CWD || process.cwd();
    }
  } : Qe = {
    // Supported
    get platform() {
      return At ? "win32" : va ? "darwin" : "linux";
    },
    get arch() {
    },
    // Unsupported
    get env() {
      return {};
    },
    cwd() {
      return "/";
    }
  };
const Zt = Qe.cwd, Qa = Qe.env, Ya = Qe.platform;
Qe.arch;
const Ka = 65, eo = 97, to = 90, ro = 122, Xe = 46, ne = 47, le = 92, Ue = 58, no = 63;
class Ps extends Error {
  constructor(t, r, n) {
    let i;
    typeof r == "string" && r.indexOf("not ") === 0 ? (i = "must not be", r = r.replace(/^not /, "")) : i = "must be";
    const s = t.indexOf(".") !== -1 ? "property" : "argument";
    let a = `The "${t}" ${s} ${i} of type ${r}`;
    a += `. Received type ${typeof n}`, super(a), this.code = "ERR_INVALID_ARG_TYPE";
  }
}
function io(e, t) {
  if (e === null || typeof e != "object")
    throw new Ps(t, "Object", e);
}
function Z(e, t) {
  if (typeof e != "string")
    throw new Ps(t, "string", e);
}
const de = Ya === "win32";
function $(e) {
  return e === ne || e === le;
}
function Pr(e) {
  return e === ne;
}
function Be(e) {
  return e >= Ka && e <= to || e >= eo && e <= ro;
}
function Qt(e, t, r, n) {
  let i = "", s = 0, a = -1, o = 0, l = 0;
  for (let u = 0; u <= e.length; ++u) {
    if (u < e.length)
      l = e.charCodeAt(u);
    else {
      if (n(l))
        break;
      l = ne;
    }
    if (n(l)) {
      if (!(a === u - 1 || o === 1))
        if (o === 2) {
          if (i.length < 2 || s !== 2 || i.charCodeAt(i.length - 1) !== Xe || i.charCodeAt(i.length - 2) !== Xe) {
            if (i.length > 2) {
              const f = i.lastIndexOf(r);
              f === -1 ? (i = "", s = 0) : (i = i.slice(0, f), s = i.length - 1 - i.lastIndexOf(r)), a = u, o = 0;
              continue;
            } else if (i.length !== 0) {
              i = "", s = 0, a = u, o = 0;
              continue;
            }
          }
          t && (i += i.length > 0 ? `${r}..` : "..", s = 2);
        } else
          i.length > 0 ? i += `${r}${e.slice(a + 1, u)}` : i = e.slice(a + 1, u), s = u - a - 1;
      a = u, o = 0;
    } else
      l === Xe && o !== -1 ? ++o : o = -1;
  }
  return i;
}
function Rs(e, t) {
  io(t, "pathObject");
  const r = t.dir || t.root, n = t.base || `${t.name || ""}${t.ext || ""}`;
  return r ? r === t.root ? `${r}${n}` : `${r}${e}${n}` : n;
}
const ee = {
  // path.resolve([from ...], to)
  resolve(...e) {
    let t = "", r = "", n = !1;
    for (let i = e.length - 1; i >= -1; i--) {
      let s;
      if (i >= 0) {
        if (s = e[i], Z(s, "path"), s.length === 0)
          continue;
      } else
        t.length === 0 ? s = Zt() : (s = Qa[`=${t}`] || Zt(), (s === void 0 || s.slice(0, 2).toLowerCase() !== t.toLowerCase() && s.charCodeAt(2) === le) && (s = `${t}\\`));
      const a = s.length;
      let o = 0, l = "", u = !1;
      const f = s.charCodeAt(0);
      if (a === 1)
        $(f) && (o = 1, u = !0);
      else if ($(f))
        if (u = !0, $(s.charCodeAt(1))) {
          let h = 2, d = h;
          for (; h < a && !$(s.charCodeAt(h)); )
            h++;
          if (h < a && h !== d) {
            const g = s.slice(d, h);
            for (d = h; h < a && $(s.charCodeAt(h)); )
              h++;
            if (h < a && h !== d) {
              for (d = h; h < a && !$(s.charCodeAt(h)); )
                h++;
              (h === a || h !== d) && (l = `\\\\${g}\\${s.slice(d, h)}`, o = h);
            }
          }
        } else
          o = 1;
      else
        Be(f) && s.charCodeAt(1) === Ue && (l = s.slice(0, 2), o = 2, a > 2 && $(s.charCodeAt(2)) && (u = !0, o = 3));
      if (l.length > 0)
        if (t.length > 0) {
          if (l.toLowerCase() !== t.toLowerCase())
            continue;
        } else
          t = l;
      if (n) {
        if (t.length > 0)
          break;
      } else if (r = `${s.slice(o)}\\${r}`, n = u, u && t.length > 0)
        break;
    }
    return r = Qt(r, !n, "\\", $), n ? `${t}\\${r}` : `${t}${r}` || ".";
  },
  normalize(e) {
    Z(e, "path");
    const t = e.length;
    if (t === 0)
      return ".";
    let r = 0, n, i = !1;
    const s = e.charCodeAt(0);
    if (t === 1)
      return Pr(s) ? "\\" : e;
    if ($(s))
      if (i = !0, $(e.charCodeAt(1))) {
        let o = 2, l = o;
        for (; o < t && !$(e.charCodeAt(o)); )
          o++;
        if (o < t && o !== l) {
          const u = e.slice(l, o);
          for (l = o; o < t && $(e.charCodeAt(o)); )
            o++;
          if (o < t && o !== l) {
            for (l = o; o < t && !$(e.charCodeAt(o)); )
              o++;
            if (o === t)
              return `\\\\${u}\\${e.slice(l)}\\`;
            o !== l && (n = `\\\\${u}\\${e.slice(l, o)}`, r = o);
          }
        }
      } else
        r = 1;
    else
      Be(s) && e.charCodeAt(1) === Ue && (n = e.slice(0, 2), r = 2, t > 2 && $(e.charCodeAt(2)) && (i = !0, r = 3));
    let a = r < t ? Qt(e.slice(r), !i, "\\", $) : "";
    return a.length === 0 && !i && (a = "."), a.length > 0 && $(e.charCodeAt(t - 1)) && (a += "\\"), n === void 0 ? i ? `\\${a}` : a : i ? `${n}\\${a}` : `${n}${a}`;
  },
  isAbsolute(e) {
    Z(e, "path");
    const t = e.length;
    if (t === 0)
      return !1;
    const r = e.charCodeAt(0);
    return $(r) || // Possible device root
    t > 2 && Be(r) && e.charCodeAt(1) === Ue && $(e.charCodeAt(2));
  },
  join(...e) {
    if (e.length === 0)
      return ".";
    let t, r;
    for (let s = 0; s < e.length; ++s) {
      const a = e[s];
      Z(a, "path"), a.length > 0 && (t === void 0 ? t = r = a : t += `\\${a}`);
    }
    if (t === void 0)
      return ".";
    let n = !0, i = 0;
    if (typeof r == "string" && $(r.charCodeAt(0))) {
      ++i;
      const s = r.length;
      s > 1 && $(r.charCodeAt(1)) && (++i, s > 2 && ($(r.charCodeAt(2)) ? ++i : n = !1));
    }
    if (n) {
      for (; i < t.length && $(t.charCodeAt(i)); )
        i++;
      i >= 2 && (t = `\\${t.slice(i)}`);
    }
    return ee.normalize(t);
  },
  // It will solve the relative path from `from` to `to`, for instance:
  //  from = 'C:\\orandea\\test\\aaa'
  //  to = 'C:\\orandea\\impl\\bbb'
  // The output of the function should be: '..\\..\\impl\\bbb'
  relative(e, t) {
    if (Z(e, "from"), Z(t, "to"), e === t)
      return "";
    const r = ee.resolve(e), n = ee.resolve(t);
    if (r === n || (e = r.toLowerCase(), t = n.toLowerCase(), e === t))
      return "";
    let i = 0;
    for (; i < e.length && e.charCodeAt(i) === le; )
      i++;
    let s = e.length;
    for (; s - 1 > i && e.charCodeAt(s - 1) === le; )
      s--;
    const a = s - i;
    let o = 0;
    for (; o < t.length && t.charCodeAt(o) === le; )
      o++;
    let l = t.length;
    for (; l - 1 > o && t.charCodeAt(l - 1) === le; )
      l--;
    const u = l - o, f = a < u ? a : u;
    let h = -1, d = 0;
    for (; d < f; d++) {
      const p = e.charCodeAt(i + d);
      if (p !== t.charCodeAt(o + d))
        break;
      p === le && (h = d);
    }
    if (d !== f) {
      if (h === -1)
        return n;
    } else {
      if (u > f) {
        if (t.charCodeAt(o + d) === le)
          return n.slice(o + d + 1);
        if (d === 2)
          return n.slice(o + d);
      }
      a > f && (e.charCodeAt(i + d) === le ? h = d : d === 2 && (h = 3)), h === -1 && (h = 0);
    }
    let g = "";
    for (d = i + h + 1; d <= s; ++d)
      (d === s || e.charCodeAt(d) === le) && (g += g.length === 0 ? ".." : "\\..");
    return o += h, g.length > 0 ? `${g}${n.slice(o, l)}` : (n.charCodeAt(o) === le && ++o, n.slice(o, l));
  },
  toNamespacedPath(e) {
    if (typeof e != "string" || e.length === 0)
      return e;
    const t = ee.resolve(e);
    if (t.length <= 2)
      return e;
    if (t.charCodeAt(0) === le) {
      if (t.charCodeAt(1) === le) {
        const r = t.charCodeAt(2);
        if (r !== no && r !== Xe)
          return `\\\\?\\UNC\\${t.slice(2)}`;
      }
    } else if (Be(t.charCodeAt(0)) && t.charCodeAt(1) === Ue && t.charCodeAt(2) === le)
      return `\\\\?\\${t}`;
    return e;
  },
  dirname(e) {
    Z(e, "path");
    const t = e.length;
    if (t === 0)
      return ".";
    let r = -1, n = 0;
    const i = e.charCodeAt(0);
    if (t === 1)
      return $(i) ? e : ".";
    if ($(i)) {
      if (r = n = 1, $(e.charCodeAt(1))) {
        let o = 2, l = o;
        for (; o < t && !$(e.charCodeAt(o)); )
          o++;
        if (o < t && o !== l) {
          for (l = o; o < t && $(e.charCodeAt(o)); )
            o++;
          if (o < t && o !== l) {
            for (l = o; o < t && !$(e.charCodeAt(o)); )
              o++;
            if (o === t)
              return e;
            o !== l && (r = n = o + 1);
          }
        }
      }
    } else
      Be(i) && e.charCodeAt(1) === Ue && (r = t > 2 && $(e.charCodeAt(2)) ? 3 : 2, n = r);
    let s = -1, a = !0;
    for (let o = t - 1; o >= n; --o)
      if ($(e.charCodeAt(o))) {
        if (!a) {
          s = o;
          break;
        }
      } else
        a = !1;
    if (s === -1) {
      if (r === -1)
        return ".";
      s = r;
    }
    return e.slice(0, s);
  },
  basename(e, t) {
    t !== void 0 && Z(t, "ext"), Z(e, "path");
    let r = 0, n = -1, i = !0, s;
    if (e.length >= 2 && Be(e.charCodeAt(0)) && e.charCodeAt(1) === Ue && (r = 2), t !== void 0 && t.length > 0 && t.length <= e.length) {
      if (t === e)
        return "";
      let a = t.length - 1, o = -1;
      for (s = e.length - 1; s >= r; --s) {
        const l = e.charCodeAt(s);
        if ($(l)) {
          if (!i) {
            r = s + 1;
            break;
          }
        } else
          o === -1 && (i = !1, o = s + 1), a >= 0 && (l === t.charCodeAt(a) ? --a === -1 && (n = s) : (a = -1, n = o));
      }
      return r === n ? n = o : n === -1 && (n = e.length), e.slice(r, n);
    }
    for (s = e.length - 1; s >= r; --s)
      if ($(e.charCodeAt(s))) {
        if (!i) {
          r = s + 1;
          break;
        }
      } else
        n === -1 && (i = !1, n = s + 1);
    return n === -1 ? "" : e.slice(r, n);
  },
  extname(e) {
    Z(e, "path");
    let t = 0, r = -1, n = 0, i = -1, s = !0, a = 0;
    e.length >= 2 && e.charCodeAt(1) === Ue && Be(e.charCodeAt(0)) && (t = n = 2);
    for (let o = e.length - 1; o >= t; --o) {
      const l = e.charCodeAt(o);
      if ($(l)) {
        if (!s) {
          n = o + 1;
          break;
        }
        continue;
      }
      i === -1 && (s = !1, i = o + 1), l === Xe ? r === -1 ? r = o : a !== 1 && (a = 1) : r !== -1 && (a = -1);
    }
    return r === -1 || i === -1 || // We saw a non-dot character immediately before the dot
    a === 0 || // The (right-most) trimmed path component is exactly '..'
    a === 1 && r === i - 1 && r === n + 1 ? "" : e.slice(r, i);
  },
  format: Rs.bind(null, "\\"),
  parse(e) {
    Z(e, "path");
    const t = { root: "", dir: "", base: "", ext: "", name: "" };
    if (e.length === 0)
      return t;
    const r = e.length;
    let n = 0, i = e.charCodeAt(0);
    if (r === 1)
      return $(i) ? (t.root = t.dir = e, t) : (t.base = t.name = e, t);
    if ($(i)) {
      if (n = 1, $(e.charCodeAt(1))) {
        let h = 2, d = h;
        for (; h < r && !$(e.charCodeAt(h)); )
          h++;
        if (h < r && h !== d) {
          for (d = h; h < r && $(e.charCodeAt(h)); )
            h++;
          if (h < r && h !== d) {
            for (d = h; h < r && !$(e.charCodeAt(h)); )
              h++;
            h === r ? n = h : h !== d && (n = h + 1);
          }
        }
      }
    } else if (Be(i) && e.charCodeAt(1) === Ue) {
      if (r <= 2)
        return t.root = t.dir = e, t;
      if (n = 2, $(e.charCodeAt(2))) {
        if (r === 3)
          return t.root = t.dir = e, t;
        n = 3;
      }
    }
    n > 0 && (t.root = e.slice(0, n));
    let s = -1, a = n, o = -1, l = !0, u = e.length - 1, f = 0;
    for (; u >= n; --u) {
      if (i = e.charCodeAt(u), $(i)) {
        if (!l) {
          a = u + 1;
          break;
        }
        continue;
      }
      o === -1 && (l = !1, o = u + 1), i === Xe ? s === -1 ? s = u : f !== 1 && (f = 1) : s !== -1 && (f = -1);
    }
    return o !== -1 && (s === -1 || // We saw a non-dot character immediately before the dot
    f === 0 || // The (right-most) trimmed path component is exactly '..'
    f === 1 && s === o - 1 && s === a + 1 ? t.base = t.name = e.slice(a, o) : (t.name = e.slice(a, s), t.base = e.slice(a, o), t.ext = e.slice(s, o))), a > 0 && a !== n ? t.dir = e.slice(0, a - 1) : t.dir = t.root, t;
  },
  sep: "\\",
  delimiter: ";",
  win32: null,
  posix: null
}, so = (() => {
  if (de) {
    const e = /\\/g;
    return () => {
      const t = Zt().replace(e, "/");
      return t.slice(t.indexOf("/"));
    };
  }
  return () => Zt();
})(), re = {
  // path.resolve([from ...], to)
  resolve(...e) {
    let t = "", r = !1;
    for (let n = e.length - 1; n >= -1 && !r; n--) {
      const i = n >= 0 ? e[n] : so();
      Z(i, "path"), i.length !== 0 && (t = `${i}/${t}`, r = i.charCodeAt(0) === ne);
    }
    return t = Qt(t, !r, "/", Pr), r ? `/${t}` : t.length > 0 ? t : ".";
  },
  normalize(e) {
    if (Z(e, "path"), e.length === 0)
      return ".";
    const t = e.charCodeAt(0) === ne, r = e.charCodeAt(e.length - 1) === ne;
    return e = Qt(e, !t, "/", Pr), e.length === 0 ? t ? "/" : r ? "./" : "." : (r && (e += "/"), t ? `/${e}` : e);
  },
  isAbsolute(e) {
    return Z(e, "path"), e.length > 0 && e.charCodeAt(0) === ne;
  },
  join(...e) {
    if (e.length === 0)
      return ".";
    let t;
    for (let r = 0; r < e.length; ++r) {
      const n = e[r];
      Z(n, "path"), n.length > 0 && (t === void 0 ? t = n : t += `/${n}`);
    }
    return t === void 0 ? "." : re.normalize(t);
  },
  relative(e, t) {
    if (Z(e, "from"), Z(t, "to"), e === t || (e = re.resolve(e), t = re.resolve(t), e === t))
      return "";
    const r = 1, n = e.length, i = n - r, s = 1, a = t.length - s, o = i < a ? i : a;
    let l = -1, u = 0;
    for (; u < o; u++) {
      const h = e.charCodeAt(r + u);
      if (h !== t.charCodeAt(s + u))
        break;
      h === ne && (l = u);
    }
    if (u === o)
      if (a > o) {
        if (t.charCodeAt(s + u) === ne)
          return t.slice(s + u + 1);
        if (u === 0)
          return t.slice(s + u);
      } else
        i > o && (e.charCodeAt(r + u) === ne ? l = u : u === 0 && (l = 0));
    let f = "";
    for (u = r + l + 1; u <= n; ++u)
      (u === n || e.charCodeAt(u) === ne) && (f += f.length === 0 ? ".." : "/..");
    return `${f}${t.slice(s + l)}`;
  },
  toNamespacedPath(e) {
    return e;
  },
  dirname(e) {
    if (Z(e, "path"), e.length === 0)
      return ".";
    const t = e.charCodeAt(0) === ne;
    let r = -1, n = !0;
    for (let i = e.length - 1; i >= 1; --i)
      if (e.charCodeAt(i) === ne) {
        if (!n) {
          r = i;
          break;
        }
      } else
        n = !1;
    return r === -1 ? t ? "/" : "." : t && r === 1 ? "//" : e.slice(0, r);
  },
  basename(e, t) {
    t !== void 0 && Z(t, "ext"), Z(e, "path");
    let r = 0, n = -1, i = !0, s;
    if (t !== void 0 && t.length > 0 && t.length <= e.length) {
      if (t === e)
        return "";
      let a = t.length - 1, o = -1;
      for (s = e.length - 1; s >= 0; --s) {
        const l = e.charCodeAt(s);
        if (l === ne) {
          if (!i) {
            r = s + 1;
            break;
          }
        } else
          o === -1 && (i = !1, o = s + 1), a >= 0 && (l === t.charCodeAt(a) ? --a === -1 && (n = s) : (a = -1, n = o));
      }
      return r === n ? n = o : n === -1 && (n = e.length), e.slice(r, n);
    }
    for (s = e.length - 1; s >= 0; --s)
      if (e.charCodeAt(s) === ne) {
        if (!i) {
          r = s + 1;
          break;
        }
      } else
        n === -1 && (i = !1, n = s + 1);
    return n === -1 ? "" : e.slice(r, n);
  },
  extname(e) {
    Z(e, "path");
    let t = -1, r = 0, n = -1, i = !0, s = 0;
    for (let a = e.length - 1; a >= 0; --a) {
      const o = e.charCodeAt(a);
      if (o === ne) {
        if (!i) {
          r = a + 1;
          break;
        }
        continue;
      }
      n === -1 && (i = !1, n = a + 1), o === Xe ? t === -1 ? t = a : s !== 1 && (s = 1) : t !== -1 && (s = -1);
    }
    return t === -1 || n === -1 || // We saw a non-dot character immediately before the dot
    s === 0 || // The (right-most) trimmed path component is exactly '..'
    s === 1 && t === n - 1 && t === r + 1 ? "" : e.slice(t, n);
  },
  format: Rs.bind(null, "/"),
  parse(e) {
    Z(e, "path");
    const t = { root: "", dir: "", base: "", ext: "", name: "" };
    if (e.length === 0)
      return t;
    const r = e.charCodeAt(0) === ne;
    let n;
    r ? (t.root = "/", n = 1) : n = 0;
    let i = -1, s = 0, a = -1, o = !0, l = e.length - 1, u = 0;
    for (; l >= n; --l) {
      const f = e.charCodeAt(l);
      if (f === ne) {
        if (!o) {
          s = l + 1;
          break;
        }
        continue;
      }
      a === -1 && (o = !1, a = l + 1), f === Xe ? i === -1 ? i = l : u !== 1 && (u = 1) : i !== -1 && (u = -1);
    }
    if (a !== -1) {
      const f = s === 0 && r ? 1 : s;
      i === -1 || // We saw a non-dot character immediately before the dot
      u === 0 || // The (right-most) trimmed path component is exactly '..'
      u === 1 && i === a - 1 && i === s + 1 ? t.base = t.name = e.slice(f, a) : (t.name = e.slice(f, i), t.base = e.slice(f, a), t.ext = e.slice(i, a));
    }
    return s > 0 ? t.dir = e.slice(0, s - 1) : r && (t.dir = "/"), t;
  },
  sep: "/",
  delimiter: ":",
  win32: null,
  posix: null
};
re.win32 = ee.win32 = ee;
re.posix = ee.posix = re;
de ? ee.normalize : re.normalize;
de ? ee.isAbsolute : re.isAbsolute;
de ? ee.join : re.join;
de ? ee.resolve : re.resolve;
de ? ee.relative : re.relative;
de ? ee.dirname : re.dirname;
de ? ee.basename : re.basename;
de ? ee.extname : re.extname;
de ? ee.format : re.format;
de ? ee.parse : re.parse;
de ? ee.toNamespacedPath : re.toNamespacedPath;
de ? ee.sep : re.sep;
de ? ee.delimiter : re.delimiter;
const ao = /^\w[\w\d+.-]*$/, oo = /^\//, lo = /^\/\//;
function Nn(e, t) {
  if (!e.scheme && t)
    throw new Error(`[UriError]: Scheme is missing: {scheme: "", authority: "${e.authority}", path: "${e.path}", query: "${e.query}", fragment: "${e.fragment}"}`);
  if (e.scheme && !ao.test(e.scheme))
    throw new Error("[UriError]: Scheme contains illegal characters.");
  if (e.path) {
    if (e.authority) {
      if (!oo.test(e.path))
        throw new Error('[UriError]: If a URI contains an authority component, then the path component must either be empty or begin with a slash ("/") character');
    } else if (lo.test(e.path))
      throw new Error('[UriError]: If a URI does not contain an authority component, then the path cannot begin with two slash characters ("//")');
  }
}
function uo(e, t) {
  return !e && !t ? "file" : e;
}
function co(e, t) {
  switch (e) {
    case "https":
    case "http":
    case "file":
      t ? t[0] !== _e && (t = _e + t) : t = _e;
      break;
  }
  return t;
}
const X = "", _e = "/", fo = /^(([^:/?#]+?):)?(\/\/([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?/;
let on = class Wt {
  static isUri(t) {
    return t instanceof Wt ? !0 : t ? typeof t.authority == "string" && typeof t.fragment == "string" && typeof t.path == "string" && typeof t.query == "string" && typeof t.scheme == "string" && typeof t.fsPath == "string" && typeof t.with == "function" && typeof t.toString == "function" : !1;
  }
  /**
   * @internal
   */
  constructor(t, r, n, i, s, a = !1) {
    typeof t == "object" ? (this.scheme = t.scheme || X, this.authority = t.authority || X, this.path = t.path || X, this.query = t.query || X, this.fragment = t.fragment || X) : (this.scheme = uo(t, a), this.authority = r || X, this.path = co(this.scheme, n || X), this.query = i || X, this.fragment = s || X, Nn(this, a));
  }
  // ---- filesystem path -----------------------
  /**
   * Returns a string representing the corresponding file system path of this URI.
   * Will handle UNC paths, normalizes windows drive letters to lower-case, and uses the
   * platform specific path separator.
   *
   * * Will *not* validate the path for invalid characters and semantics.
   * * Will *not* look at the scheme of this URI.
   * * The result shall *not* be used for display purposes but for accessing a file on disk.
   *
   *
   * The *difference* to `URI#path` is the use of the platform specific separator and the handling
   * of UNC paths. See the below sample of a file-uri with an authority (UNC path).
   *
   * ```ts
      const u = URI.parse('file://server/c$/folder/file.txt')
      u.authority === 'server'
      u.path === '/shares/c$/file.txt'
      u.fsPath === '\\server\c$\folder\file.txt'
  ```
   *
   * Using `URI#path` to read a file (using fs-apis) would not be enough because parts of the path,
   * namely the server name, would be missing. Therefore `URI#fsPath` exists - it's sugar to ease working
   * with URIs that represent files on disk (`file` scheme).
   */
  get fsPath() {
    return Rr(this, !1);
  }
  // ---- modify to new -------------------------
  with(t) {
    if (!t)
      return this;
    let { scheme: r, authority: n, path: i, query: s, fragment: a } = t;
    return r === void 0 ? r = this.scheme : r === null && (r = X), n === void 0 ? n = this.authority : n === null && (n = X), i === void 0 ? i = this.path : i === null && (i = X), s === void 0 ? s = this.query : s === null && (s = X), a === void 0 ? a = this.fragment : a === null && (a = X), r === this.scheme && n === this.authority && i === this.path && s === this.query && a === this.fragment ? this : new at(r, n, i, s, a);
  }
  // ---- parse & validate ------------------------
  /**
   * Creates a new URI from a string, e.g. `http://www.example.com/some/path`,
   * `file:///usr/home`, or `scheme:with/path`.
   *
   * @param value A string which represents an URI (see `URI#toString`).
   */
  static parse(t, r = !1) {
    const n = fo.exec(t);
    return n ? new at(n[2] || X, It(n[4] || X), It(n[5] || X), It(n[7] || X), It(n[9] || X), r) : new at(X, X, X, X, X);
  }
  /**
   * Creates a new URI from a file system path, e.g. `c:\my\files`,
   * `/usr/home`, or `\\server\share\some\path`.
   *
   * The *difference* between `URI#parse` and `URI#file` is that the latter treats the argument
   * as path, not as stringified-uri. E.g. `URI.file(path)` is **not the same as**
   * `URI.parse('file://' + path)` because the path might contain characters that are
   * interpreted (# and ?). See the following sample:
   * ```ts
  const good = URI.file('/coding/c#/project1');
  good.scheme === 'file';
  good.path === '/coding/c#/project1';
  good.fragment === '';
  const bad = URI.parse('file://' + '/coding/c#/project1');
  bad.scheme === 'file';
  bad.path === '/coding/c'; // path is now broken
  bad.fragment === '/project1';
  ```
   *
   * @param path A file system path (see `URI#fsPath`)
   */
  static file(t) {
    let r = X;
    if (At && (t = t.replace(/\\/g, _e)), t[0] === _e && t[1] === _e) {
      const n = t.indexOf(_e, 2);
      n === -1 ? (r = t.substring(2), t = _e) : (r = t.substring(2, n), t = t.substring(n) || _e);
    }
    return new at("file", r, t, X, X);
  }
  static from(t) {
    const r = new at(t.scheme, t.authority, t.path, t.query, t.fragment);
    return Nn(r, !0), r;
  }
  /**
   * Join a URI path with path fragments and normalizes the resulting path.
   *
   * @param uri The input URI.
   * @param pathFragment The path fragment to add to the URI path.
   * @returns The resulting URI.
   */
  static joinPath(t, ...r) {
    if (!t.path)
      throw new Error("[UriError]: cannot call joinPath on URI without path");
    let n;
    return At && t.scheme === "file" ? n = Wt.file(ee.join(Rr(t, !0), ...r)).path : n = re.join(t.path, ...r), t.with({ path: n });
  }
  // ---- printing/externalize ---------------------------
  /**
   * Creates a string representation for this URI. It's guaranteed that calling
   * `URI.parse` with the result of this function creates an URI which is equal
   * to this URI.
   *
   * * The result shall *not* be used for display purposes but for externalization or transport.
   * * The result will be encoded using the percentage encoding and encoding happens mostly
   * ignore the scheme-specific encoding rules.
   *
   * @param skipEncoding Do not encode the result, default is `false`
   */
  toString(t = !1) {
    return Fr(this, t);
  }
  toJSON() {
    return this;
  }
  static revive(t) {
    if (t) {
      if (t instanceof Wt)
        return t;
      {
        const r = new at(t);
        return r._formatted = t.external, r._fsPath = t._sep === Fs ? t.fsPath : null, r;
      }
    } else
      return t;
  }
};
const Fs = At ? 1 : void 0;
class at extends on {
  constructor() {
    super(...arguments), this._formatted = null, this._fsPath = null;
  }
  get fsPath() {
    return this._fsPath || (this._fsPath = Rr(this, !1)), this._fsPath;
  }
  toString(t = !1) {
    return t ? Fr(this, !0) : (this._formatted || (this._formatted = Fr(this, !1)), this._formatted);
  }
  toJSON() {
    const t = {
      $mid: 1
      /* MarshalledId.Uri */
    };
    return this._fsPath && (t.fsPath = this._fsPath, t._sep = Fs), this._formatted && (t.external = this._formatted), this.path && (t.path = this.path), this.scheme && (t.scheme = this.scheme), this.authority && (t.authority = this.authority), this.query && (t.query = this.query), this.fragment && (t.fragment = this.fragment), t;
  }
}
const Is = {
  [
    58
    /* CharCode.Colon */
  ]: "%3A",
  [
    47
    /* CharCode.Slash */
  ]: "%2F",
  [
    63
    /* CharCode.QuestionMark */
  ]: "%3F",
  [
    35
    /* CharCode.Hash */
  ]: "%23",
  [
    91
    /* CharCode.OpenSquareBracket */
  ]: "%5B",
  [
    93
    /* CharCode.CloseSquareBracket */
  ]: "%5D",
  [
    64
    /* CharCode.AtSign */
  ]: "%40",
  [
    33
    /* CharCode.ExclamationMark */
  ]: "%21",
  [
    36
    /* CharCode.DollarSign */
  ]: "%24",
  [
    38
    /* CharCode.Ampersand */
  ]: "%26",
  [
    39
    /* CharCode.SingleQuote */
  ]: "%27",
  [
    40
    /* CharCode.OpenParen */
  ]: "%28",
  [
    41
    /* CharCode.CloseParen */
  ]: "%29",
  [
    42
    /* CharCode.Asterisk */
  ]: "%2A",
  [
    43
    /* CharCode.Plus */
  ]: "%2B",
  [
    44
    /* CharCode.Comma */
  ]: "%2C",
  [
    59
    /* CharCode.Semicolon */
  ]: "%3B",
  [
    61
    /* CharCode.Equals */
  ]: "%3D",
  [
    32
    /* CharCode.Space */
  ]: "%20"
};
function Ln(e, t, r) {
  let n, i = -1;
  for (let s = 0; s < e.length; s++) {
    const a = e.charCodeAt(s);
    if (a >= 97 && a <= 122 || a >= 65 && a <= 90 || a >= 48 && a <= 57 || a === 45 || a === 46 || a === 95 || a === 126 || t && a === 47 || r && a === 91 || r && a === 93 || r && a === 58)
      i !== -1 && (n += encodeURIComponent(e.substring(i, s)), i = -1), n !== void 0 && (n += e.charAt(s));
    else {
      n === void 0 && (n = e.substr(0, s));
      const o = Is[a];
      o !== void 0 ? (i !== -1 && (n += encodeURIComponent(e.substring(i, s)), i = -1), n += o) : i === -1 && (i = s);
    }
  }
  return i !== -1 && (n += encodeURIComponent(e.substring(i))), n !== void 0 ? n : e;
}
function ho(e) {
  let t;
  for (let r = 0; r < e.length; r++) {
    const n = e.charCodeAt(r);
    n === 35 || n === 63 ? (t === void 0 && (t = e.substr(0, r)), t += Is[n]) : t !== void 0 && (t += e[r]);
  }
  return t !== void 0 ? t : e;
}
function Rr(e, t) {
  let r;
  return e.authority && e.path.length > 1 && e.scheme === "file" ? r = `//${e.authority}${e.path}` : e.path.charCodeAt(0) === 47 && (e.path.charCodeAt(1) >= 65 && e.path.charCodeAt(1) <= 90 || e.path.charCodeAt(1) >= 97 && e.path.charCodeAt(1) <= 122) && e.path.charCodeAt(2) === 58 ? t ? r = e.path.substr(1) : r = e.path[1].toLowerCase() + e.path.substr(2) : r = e.path, At && (r = r.replace(/\//g, "\\")), r;
}
function Fr(e, t) {
  const r = t ? ho : Ln;
  let n = "", { scheme: i, authority: s, path: a, query: o, fragment: l } = e;
  if (i && (n += i, n += ":"), (s || i === "file") && (n += _e, n += _e), s) {
    let u = s.indexOf("@");
    if (u !== -1) {
      const f = s.substr(0, u);
      s = s.substr(u + 1), u = f.lastIndexOf(":"), u === -1 ? n += r(f, !1, !1) : (n += r(f.substr(0, u), !1, !1), n += ":", n += r(f.substr(u + 1), !1, !0)), n += "@";
    }
    s = s.toLowerCase(), u = s.lastIndexOf(":"), u === -1 ? n += r(s, !1, !0) : (n += r(s.substr(0, u), !1, !0), n += s.substr(u));
  }
  if (a) {
    if (a.length >= 3 && a.charCodeAt(0) === 47 && a.charCodeAt(2) === 58) {
      const u = a.charCodeAt(1);
      u >= 65 && u <= 90 && (a = `/${String.fromCharCode(u + 32)}:${a.substr(3)}`);
    } else if (a.length >= 2 && a.charCodeAt(1) === 58) {
      const u = a.charCodeAt(0);
      u >= 65 && u <= 90 && (a = `${String.fromCharCode(u + 32)}:${a.substr(2)}`);
    }
    n += r(a, !0, !1);
  }
  return o && (n += "?", n += r(o, !1, !1)), l && (n += "#", n += t ? l : Ln(l, !1, !1)), n;
}
function Vs(e) {
  try {
    return decodeURIComponent(e);
  } catch {
    return e.length > 3 ? e.substr(0, 3) + Vs(e.substr(3)) : e;
  }
}
const Cn = /(%[0-9A-Za-z][0-9A-Za-z])+/g;
function It(e) {
  return e.match(Cn) ? e.replace(Cn, (t) => Vs(t)) : e;
}
let Re = class Ze {
  constructor(t, r) {
    this.lineNumber = t, this.column = r;
  }
  /**
   * Create a new position from this position.
   *
   * @param newLineNumber new line number
   * @param newColumn new column
   */
  with(t = this.lineNumber, r = this.column) {
    return t === this.lineNumber && r === this.column ? this : new Ze(t, r);
  }
  /**
   * Derive a new position from this position.
   *
   * @param deltaLineNumber line number delta
   * @param deltaColumn column delta
   */
  delta(t = 0, r = 0) {
    return this.with(this.lineNumber + t, this.column + r);
  }
  /**
   * Test if this position equals other position
   */
  equals(t) {
    return Ze.equals(this, t);
  }
  /**
   * Test if position `a` equals position `b`
   */
  static equals(t, r) {
    return !t && !r ? !0 : !!t && !!r && t.lineNumber === r.lineNumber && t.column === r.column;
  }
  /**
   * Test if this position is before other position.
   * If the two positions are equal, the result will be false.
   */
  isBefore(t) {
    return Ze.isBefore(this, t);
  }
  /**
   * Test if position `a` is before position `b`.
   * If the two positions are equal, the result will be false.
   */
  static isBefore(t, r) {
    return t.lineNumber < r.lineNumber ? !0 : r.lineNumber < t.lineNumber ? !1 : t.column < r.column;
  }
  /**
   * Test if this position is before other position.
   * If the two positions are equal, the result will be true.
   */
  isBeforeOrEqual(t) {
    return Ze.isBeforeOrEqual(this, t);
  }
  /**
   * Test if position `a` is before position `b`.
   * If the two positions are equal, the result will be true.
   */
  static isBeforeOrEqual(t, r) {
    return t.lineNumber < r.lineNumber ? !0 : r.lineNumber < t.lineNumber ? !1 : t.column <= r.column;
  }
  /**
   * A function that compares positions, useful for sorting
   */
  static compare(t, r) {
    const n = t.lineNumber | 0, i = r.lineNumber | 0;
    if (n === i) {
      const s = t.column | 0, a = r.column | 0;
      return s - a;
    }
    return n - i;
  }
  /**
   * Clone this position.
   */
  clone() {
    return new Ze(this.lineNumber, this.column);
  }
  /**
   * Convert to a human-readable representation.
   */
  toString() {
    return "(" + this.lineNumber + "," + this.column + ")";
  }
  // ---
  /**
   * Create a `Position` from an `IPosition`.
   */
  static lift(t) {
    return new Ze(t.lineNumber, t.column);
  }
  /**
   * Test if `obj` is an `IPosition`.
   */
  static isIPosition(t) {
    return t && typeof t.lineNumber == "number" && typeof t.column == "number";
  }
}, fe = class Y {
  constructor(t, r, n, i) {
    t > n || t === n && r > i ? (this.startLineNumber = n, this.startColumn = i, this.endLineNumber = t, this.endColumn = r) : (this.startLineNumber = t, this.startColumn = r, this.endLineNumber = n, this.endColumn = i);
  }
  /**
   * Test if this range is empty.
   */
  isEmpty() {
    return Y.isEmpty(this);
  }
  /**
   * Test if `range` is empty.
   */
  static isEmpty(t) {
    return t.startLineNumber === t.endLineNumber && t.startColumn === t.endColumn;
  }
  /**
   * Test if position is in this range. If the position is at the edges, will return true.
   */
  containsPosition(t) {
    return Y.containsPosition(this, t);
  }
  /**
   * Test if `position` is in `range`. If the position is at the edges, will return true.
   */
  static containsPosition(t, r) {
    return !(r.lineNumber < t.startLineNumber || r.lineNumber > t.endLineNumber || r.lineNumber === t.startLineNumber && r.column < t.startColumn || r.lineNumber === t.endLineNumber && r.column > t.endColumn);
  }
  /**
   * Test if `position` is in `range`. If the position is at the edges, will return false.
   * @internal
   */
  static strictContainsPosition(t, r) {
    return !(r.lineNumber < t.startLineNumber || r.lineNumber > t.endLineNumber || r.lineNumber === t.startLineNumber && r.column <= t.startColumn || r.lineNumber === t.endLineNumber && r.column >= t.endColumn);
  }
  /**
   * Test if range is in this range. If the range is equal to this range, will return true.
   */
  containsRange(t) {
    return Y.containsRange(this, t);
  }
  /**
   * Test if `otherRange` is in `range`. If the ranges are equal, will return true.
   */
  static containsRange(t, r) {
    return !(r.startLineNumber < t.startLineNumber || r.endLineNumber < t.startLineNumber || r.startLineNumber > t.endLineNumber || r.endLineNumber > t.endLineNumber || r.startLineNumber === t.startLineNumber && r.startColumn < t.startColumn || r.endLineNumber === t.endLineNumber && r.endColumn > t.endColumn);
  }
  /**
   * Test if `range` is strictly in this range. `range` must start after and end before this range for the result to be true.
   */
  strictContainsRange(t) {
    return Y.strictContainsRange(this, t);
  }
  /**
   * Test if `otherRange` is strictly in `range` (must start after, and end before). If the ranges are equal, will return false.
   */
  static strictContainsRange(t, r) {
    return !(r.startLineNumber < t.startLineNumber || r.endLineNumber < t.startLineNumber || r.startLineNumber > t.endLineNumber || r.endLineNumber > t.endLineNumber || r.startLineNumber === t.startLineNumber && r.startColumn <= t.startColumn || r.endLineNumber === t.endLineNumber && r.endColumn >= t.endColumn);
  }
  /**
   * A reunion of the two ranges.
   * The smallest position will be used as the start point, and the largest one as the end point.
   */
  plusRange(t) {
    return Y.plusRange(this, t);
  }
  /**
   * A reunion of the two ranges.
   * The smallest position will be used as the start point, and the largest one as the end point.
   */
  static plusRange(t, r) {
    let n, i, s, a;
    return r.startLineNumber < t.startLineNumber ? (n = r.startLineNumber, i = r.startColumn) : r.startLineNumber === t.startLineNumber ? (n = r.startLineNumber, i = Math.min(r.startColumn, t.startColumn)) : (n = t.startLineNumber, i = t.startColumn), r.endLineNumber > t.endLineNumber ? (s = r.endLineNumber, a = r.endColumn) : r.endLineNumber === t.endLineNumber ? (s = r.endLineNumber, a = Math.max(r.endColumn, t.endColumn)) : (s = t.endLineNumber, a = t.endColumn), new Y(n, i, s, a);
  }
  /**
   * A intersection of the two ranges.
   */
  intersectRanges(t) {
    return Y.intersectRanges(this, t);
  }
  /**
   * A intersection of the two ranges.
   */
  static intersectRanges(t, r) {
    let n = t.startLineNumber, i = t.startColumn, s = t.endLineNumber, a = t.endColumn;
    const o = r.startLineNumber, l = r.startColumn, u = r.endLineNumber, f = r.endColumn;
    return n < o ? (n = o, i = l) : n === o && (i = Math.max(i, l)), s > u ? (s = u, a = f) : s === u && (a = Math.min(a, f)), n > s || n === s && i > a ? null : new Y(n, i, s, a);
  }
  /**
   * Test if this range equals other.
   */
  equalsRange(t) {
    return Y.equalsRange(this, t);
  }
  /**
   * Test if range `a` equals `b`.
   */
  static equalsRange(t, r) {
    return !t && !r ? !0 : !!t && !!r && t.startLineNumber === r.startLineNumber && t.startColumn === r.startColumn && t.endLineNumber === r.endLineNumber && t.endColumn === r.endColumn;
  }
  /**
   * Return the end position (which will be after or equal to the start position)
   */
  getEndPosition() {
    return Y.getEndPosition(this);
  }
  /**
   * Return the end position (which will be after or equal to the start position)
   */
  static getEndPosition(t) {
    return new Re(t.endLineNumber, t.endColumn);
  }
  /**
   * Return the start position (which will be before or equal to the end position)
   */
  getStartPosition() {
    return Y.getStartPosition(this);
  }
  /**
   * Return the start position (which will be before or equal to the end position)
   */
  static getStartPosition(t) {
    return new Re(t.startLineNumber, t.startColumn);
  }
  /**
   * Transform to a user presentable string representation.
   */
  toString() {
    return "[" + this.startLineNumber + "," + this.startColumn + " -> " + this.endLineNumber + "," + this.endColumn + "]";
  }
  /**
   * Create a new range using this range's start position, and using endLineNumber and endColumn as the end position.
   */
  setEndPosition(t, r) {
    return new Y(this.startLineNumber, this.startColumn, t, r);
  }
  /**
   * Create a new range using this range's end position, and using startLineNumber and startColumn as the start position.
   */
  setStartPosition(t, r) {
    return new Y(t, r, this.endLineNumber, this.endColumn);
  }
  /**
   * Create a new empty range using this range's start position.
   */
  collapseToStart() {
    return Y.collapseToStart(this);
  }
  /**
   * Create a new empty range using this range's start position.
   */
  static collapseToStart(t) {
    return new Y(t.startLineNumber, t.startColumn, t.startLineNumber, t.startColumn);
  }
  /**
   * Create a new empty range using this range's end position.
   */
  collapseToEnd() {
    return Y.collapseToEnd(this);
  }
  /**
   * Create a new empty range using this range's end position.
   */
  static collapseToEnd(t) {
    return new Y(t.endLineNumber, t.endColumn, t.endLineNumber, t.endColumn);
  }
  /**
   * Moves the range by the given amount of lines.
   */
  delta(t) {
    return new Y(this.startLineNumber + t, this.startColumn, this.endLineNumber + t, this.endColumn);
  }
  // ---
  static fromPositions(t, r = t) {
    return new Y(t.lineNumber, t.column, r.lineNumber, r.column);
  }
  static lift(t) {
    return t ? new Y(t.startLineNumber, t.startColumn, t.endLineNumber, t.endColumn) : null;
  }
  /**
   * Test if `obj` is an `IRange`.
   */
  static isIRange(t) {
    return t && typeof t.startLineNumber == "number" && typeof t.startColumn == "number" && typeof t.endLineNumber == "number" && typeof t.endColumn == "number";
  }
  /**
   * Test if the two ranges are touching in any way.
   */
  static areIntersectingOrTouching(t, r) {
    return !(t.endLineNumber < r.startLineNumber || t.endLineNumber === r.startLineNumber && t.endColumn < r.startColumn || r.endLineNumber < t.startLineNumber || r.endLineNumber === t.startLineNumber && r.endColumn < t.startColumn);
  }
  /**
   * Test if the two ranges are intersecting. If the ranges are touching it returns true.
   */
  static areIntersecting(t, r) {
    return !(t.endLineNumber < r.startLineNumber || t.endLineNumber === r.startLineNumber && t.endColumn <= r.startColumn || r.endLineNumber < t.startLineNumber || r.endLineNumber === t.startLineNumber && r.endColumn <= t.startColumn);
  }
  /**
   * A function that compares ranges, useful for sorting ranges
   * It will first compare ranges on the startPosition and then on the endPosition
   */
  static compareRangesUsingStarts(t, r) {
    if (t && r) {
      const s = t.startLineNumber | 0, a = r.startLineNumber | 0;
      if (s === a) {
        const o = t.startColumn | 0, l = r.startColumn | 0;
        if (o === l) {
          const u = t.endLineNumber | 0, f = r.endLineNumber | 0;
          if (u === f) {
            const h = t.endColumn | 0, d = r.endColumn | 0;
            return h - d;
          }
          return u - f;
        }
        return o - l;
      }
      return s - a;
    }
    return (t ? 1 : 0) - (r ? 1 : 0);
  }
  /**
   * A function that compares ranges, useful for sorting ranges
   * It will first compare ranges on the endPosition and then on the startPosition
   */
  static compareRangesUsingEnds(t, r) {
    return t.endLineNumber === r.endLineNumber ? t.endColumn === r.endColumn ? t.startLineNumber === r.startLineNumber ? t.startColumn - r.startColumn : t.startLineNumber - r.startLineNumber : t.endColumn - r.endColumn : t.endLineNumber - r.endLineNumber;
  }
  /**
   * Test if the range spans multiple lines.
   */
  static spansMultipleLines(t) {
    return t.endLineNumber > t.startLineNumber;
  }
  toJSON() {
    return this;
  }
};
globalThis && globalThis.__awaiter;
var kn;
(function(e) {
  function t(i) {
    return i < 0;
  }
  e.isLessThan = t;
  function r(i) {
    return i > 0;
  }
  e.isGreaterThan = r;
  function n(i) {
    return i === 0;
  }
  e.isNeitherLessOrGreaterThan = n, e.greaterThan = 1, e.lessThan = -1, e.neitherLessOrGreaterThan = 0;
})(kn || (kn = {}));
function En(e) {
  return e < 0 ? 0 : e > 255 ? 255 : e | 0;
}
function ot(e) {
  return e < 0 ? 0 : e > 4294967295 ? 4294967295 : e | 0;
}
class go {
  constructor(t) {
    this.values = t, this.prefixSum = new Uint32Array(t.length), this.prefixSumValidIndex = new Int32Array(1), this.prefixSumValidIndex[0] = -1;
  }
  getCount() {
    return this.values.length;
  }
  insertValues(t, r) {
    t = ot(t);
    const n = this.values, i = this.prefixSum, s = r.length;
    return s === 0 ? !1 : (this.values = new Uint32Array(n.length + s), this.values.set(n.subarray(0, t), 0), this.values.set(n.subarray(t), t + s), this.values.set(r, t), t - 1 < this.prefixSumValidIndex[0] && (this.prefixSumValidIndex[0] = t - 1), this.prefixSum = new Uint32Array(this.values.length), this.prefixSumValidIndex[0] >= 0 && this.prefixSum.set(i.subarray(0, this.prefixSumValidIndex[0] + 1)), !0);
  }
  setValue(t, r) {
    return t = ot(t), r = ot(r), this.values[t] === r ? !1 : (this.values[t] = r, t - 1 < this.prefixSumValidIndex[0] && (this.prefixSumValidIndex[0] = t - 1), !0);
  }
  removeValues(t, r) {
    t = ot(t), r = ot(r);
    const n = this.values, i = this.prefixSum;
    if (t >= n.length)
      return !1;
    const s = n.length - t;
    return r >= s && (r = s), r === 0 ? !1 : (this.values = new Uint32Array(n.length - r), this.values.set(n.subarray(0, t), 0), this.values.set(n.subarray(t + r), t), this.prefixSum = new Uint32Array(this.values.length), t - 1 < this.prefixSumValidIndex[0] && (this.prefixSumValidIndex[0] = t - 1), this.prefixSumValidIndex[0] >= 0 && this.prefixSum.set(i.subarray(0, this.prefixSumValidIndex[0] + 1)), !0);
  }
  getTotalSum() {
    return this.values.length === 0 ? 0 : this._getPrefixSum(this.values.length - 1);
  }
  /**
   * Returns the sum of the first `index + 1` many items.
   * @returns `SUM(0 <= j <= index, values[j])`.
   */
  getPrefixSum(t) {
    return t < 0 ? 0 : (t = ot(t), this._getPrefixSum(t));
  }
  _getPrefixSum(t) {
    if (t <= this.prefixSumValidIndex[0])
      return this.prefixSum[t];
    let r = this.prefixSumValidIndex[0] + 1;
    r === 0 && (this.prefixSum[0] = this.values[0], r++), t >= this.values.length && (t = this.values.length - 1);
    for (let n = r; n <= t; n++)
      this.prefixSum[n] = this.prefixSum[n - 1] + this.values[n];
    return this.prefixSumValidIndex[0] = Math.max(this.prefixSumValidIndex[0], t), this.prefixSum[t];
  }
  getIndexOf(t) {
    t = Math.floor(t), this.getTotalSum();
    let r = 0, n = this.values.length - 1, i = 0, s = 0, a = 0;
    for (; r <= n; )
      if (i = r + (n - r) / 2 | 0, s = this.prefixSum[i], a = s - this.values[i], t < a)
        n = i - 1;
      else if (t >= s)
        r = i + 1;
      else
        break;
    return new mo(i, t - a);
  }
}
class mo {
  constructor(t, r) {
    this.index = t, this.remainder = r, this._prefixSumIndexOfResultBrand = void 0, this.index = t, this.remainder = r;
  }
}
class po {
  constructor(t, r, n, i) {
    this._uri = t, this._lines = r, this._eol = n, this._versionId = i, this._lineStarts = null, this._cachedTextValue = null;
  }
  dispose() {
    this._lines.length = 0;
  }
  get version() {
    return this._versionId;
  }
  getText() {
    return this._cachedTextValue === null && (this._cachedTextValue = this._lines.join(this._eol)), this._cachedTextValue;
  }
  onEvents(t) {
    t.eol && t.eol !== this._eol && (this._eol = t.eol, this._lineStarts = null);
    const r = t.changes;
    for (const n of r)
      this._acceptDeleteRange(n.range), this._acceptInsertText(new Re(n.range.startLineNumber, n.range.startColumn), n.text);
    this._versionId = t.versionId, this._cachedTextValue = null;
  }
  _ensureLineStarts() {
    if (!this._lineStarts) {
      const t = this._eol.length, r = this._lines.length, n = new Uint32Array(r);
      for (let i = 0; i < r; i++)
        n[i] = this._lines[i].length + t;
      this._lineStarts = new go(n);
    }
  }
  /**
   * All changes to a line's text go through this method
   */
  _setLineText(t, r) {
    this._lines[t] = r, this._lineStarts && this._lineStarts.setValue(t, this._lines[t].length + this._eol.length);
  }
  _acceptDeleteRange(t) {
    if (t.startLineNumber === t.endLineNumber) {
      if (t.startColumn === t.endColumn)
        return;
      this._setLineText(t.startLineNumber - 1, this._lines[t.startLineNumber - 1].substring(0, t.startColumn - 1) + this._lines[t.startLineNumber - 1].substring(t.endColumn - 1));
      return;
    }
    this._setLineText(t.startLineNumber - 1, this._lines[t.startLineNumber - 1].substring(0, t.startColumn - 1) + this._lines[t.endLineNumber - 1].substring(t.endColumn - 1)), this._lines.splice(t.startLineNumber, t.endLineNumber - t.startLineNumber), this._lineStarts && this._lineStarts.removeValues(t.startLineNumber, t.endLineNumber - t.startLineNumber);
  }
  _acceptInsertText(t, r) {
    if (r.length === 0)
      return;
    const n = Ra(r);
    if (n.length === 1) {
      this._setLineText(t.lineNumber - 1, this._lines[t.lineNumber - 1].substring(0, t.column - 1) + n[0] + this._lines[t.lineNumber - 1].substring(t.column - 1));
      return;
    }
    n[n.length - 1] += this._lines[t.lineNumber - 1].substring(t.column - 1), this._setLineText(t.lineNumber - 1, this._lines[t.lineNumber - 1].substring(0, t.column - 1) + n[0]);
    const i = new Uint32Array(n.length - 1);
    for (let s = 1; s < n.length; s++)
      this._lines.splice(t.lineNumber + s - 1, 0, n[s]), i[s - 1] = n[s].length + this._eol.length;
    this._lineStarts && this._lineStarts.insertValues(t.lineNumber, i);
  }
}
const vo = "`~!@#$%^&*()-=+[{]}\\|;:'\",.<>/?";
function bo(e = "") {
  let t = "(-?\\d*\\.\\d\\w*)|([^";
  for (const r of vo)
    e.indexOf(r) >= 0 || (t += "\\" + r);
  return t += "\\s]+)", new RegExp(t, "g");
}
const Ds = bo();
function yo(e) {
  let t = Ds;
  if (e && e instanceof RegExp)
    if (e.global)
      t = e;
    else {
      let r = "g";
      e.ignoreCase && (r += "i"), e.multiline && (r += "m"), e.unicode && (r += "u"), t = new RegExp(e.source, r);
    }
  return t.lastIndex = 0, t;
}
const Os = new Jt();
Os.unshift({
  maxLen: 1e3,
  windowSize: 15,
  timeBudget: 150
});
function ln(e, t, r, n, i) {
  if (i || (i = zt.first(Os)), r.length > i.maxLen) {
    let u = e - i.maxLen / 2;
    return u < 0 ? u = 0 : n += u, r = r.substring(u, e + i.maxLen / 2), ln(e, t, r, n, i);
  }
  const s = Date.now(), a = e - 1 - n;
  let o = -1, l = null;
  for (let u = 1; !(Date.now() - s >= i.timeBudget); u++) {
    const f = a - i.windowSize * u;
    t.lastIndex = Math.max(0, f);
    const h = xo(t, r, a, o);
    if (!h && l || (l = h, f <= 0))
      break;
    o = f;
  }
  if (l) {
    const u = {
      word: l[0],
      startColumn: n + 1 + l.index,
      endColumn: n + 1 + l.index + l[0].length
    };
    return t.lastIndex = 0, u;
  }
  return null;
}
function xo(e, t, r, n) {
  let i;
  for (; i = e.exec(t); ) {
    const s = i.index || 0;
    if (s <= r && e.lastIndex >= r)
      return i;
    if (n > 0 && s > n)
      return null;
  }
  return null;
}
class un {
  constructor(t) {
    const r = En(t);
    this._defaultValue = r, this._asciiMap = un._createAsciiMap(r), this._map = /* @__PURE__ */ new Map();
  }
  static _createAsciiMap(t) {
    const r = new Uint8Array(256);
    return r.fill(t), r;
  }
  set(t, r) {
    const n = En(r);
    t >= 0 && t < 256 ? this._asciiMap[t] = n : this._map.set(t, n);
  }
  get(t) {
    return t >= 0 && t < 256 ? this._asciiMap[t] : this._map.get(t) || this._defaultValue;
  }
  clear() {
    this._asciiMap.fill(this._defaultValue), this._map.clear();
  }
}
class wo {
  constructor(t, r, n) {
    const i = new Uint8Array(t * r);
    for (let s = 0, a = t * r; s < a; s++)
      i[s] = n;
    this._data = i, this.rows = t, this.cols = r;
  }
  get(t, r) {
    return this._data[t * this.cols + r];
  }
  set(t, r, n) {
    this._data[t * this.cols + r] = n;
  }
}
class _o {
  constructor(t) {
    let r = 0, n = 0;
    for (let s = 0, a = t.length; s < a; s++) {
      const [o, l, u] = t[s];
      l > r && (r = l), o > n && (n = o), u > n && (n = u);
    }
    r++, n++;
    const i = new wo(
      n,
      r,
      0
      /* State.Invalid */
    );
    for (let s = 0, a = t.length; s < a; s++) {
      const [o, l, u] = t[s];
      i.set(o, l, u);
    }
    this._states = i, this._maxCharCode = r;
  }
  nextState(t, r) {
    return r < 0 || r >= this._maxCharCode ? 0 : this._states.get(t, r);
  }
}
let mr = null;
function So() {
  return mr === null && (mr = new _o([
    [
      1,
      104,
      2
      /* State.H */
    ],
    [
      1,
      72,
      2
      /* State.H */
    ],
    [
      1,
      102,
      6
      /* State.F */
    ],
    [
      1,
      70,
      6
      /* State.F */
    ],
    [
      2,
      116,
      3
      /* State.HT */
    ],
    [
      2,
      84,
      3
      /* State.HT */
    ],
    [
      3,
      116,
      4
      /* State.HTT */
    ],
    [
      3,
      84,
      4
      /* State.HTT */
    ],
    [
      4,
      112,
      5
      /* State.HTTP */
    ],
    [
      4,
      80,
      5
      /* State.HTTP */
    ],
    [
      5,
      115,
      9
      /* State.BeforeColon */
    ],
    [
      5,
      83,
      9
      /* State.BeforeColon */
    ],
    [
      5,
      58,
      10
      /* State.AfterColon */
    ],
    [
      6,
      105,
      7
      /* State.FI */
    ],
    [
      6,
      73,
      7
      /* State.FI */
    ],
    [
      7,
      108,
      8
      /* State.FIL */
    ],
    [
      7,
      76,
      8
      /* State.FIL */
    ],
    [
      8,
      101,
      9
      /* State.BeforeColon */
    ],
    [
      8,
      69,
      9
      /* State.BeforeColon */
    ],
    [
      9,
      58,
      10
      /* State.AfterColon */
    ],
    [
      10,
      47,
      11
      /* State.AlmostThere */
    ],
    [
      11,
      47,
      12
      /* State.End */
    ]
  ])), mr;
}
let bt = null;
function Ao() {
  if (bt === null) {
    bt = new un(
      0
      /* CharacterClass.None */
    );
    const e = ` 	<>'"、。｡､，．：；‘〈「『〔（［｛｢｣｝］）〕』」〉’｀～…`;
    for (let r = 0; r < e.length; r++)
      bt.set(
        e.charCodeAt(r),
        1
        /* CharacterClass.ForceTermination */
      );
    const t = ".,;:";
    for (let r = 0; r < t.length; r++)
      bt.set(
        t.charCodeAt(r),
        2
        /* CharacterClass.CannotEndIn */
      );
  }
  return bt;
}
class Yt {
  static _createLink(t, r, n, i, s) {
    let a = s - 1;
    do {
      const o = r.charCodeAt(a);
      if (t.get(o) !== 2)
        break;
      a--;
    } while (a > i);
    if (i > 0) {
      const o = r.charCodeAt(i - 1), l = r.charCodeAt(a);
      (o === 40 && l === 41 || o === 91 && l === 93 || o === 123 && l === 125) && a--;
    }
    return {
      range: {
        startLineNumber: n,
        startColumn: i + 1,
        endLineNumber: n,
        endColumn: a + 2
      },
      url: r.substring(i, a + 1)
    };
  }
  static computeLinks(t, r = So()) {
    const n = Ao(), i = [];
    for (let s = 1, a = t.getLineCount(); s <= a; s++) {
      const o = t.getLineContent(s), l = o.length;
      let u = 0, f = 0, h = 0, d = 1, g = !1, p = !1, m = !1, v = !1;
      for (; u < l; ) {
        let y = !1;
        const x = o.charCodeAt(u);
        if (d === 13) {
          let b;
          switch (x) {
            case 40:
              g = !0, b = 0;
              break;
            case 41:
              b = g ? 0 : 1;
              break;
            case 91:
              m = !0, p = !0, b = 0;
              break;
            case 93:
              m = !1, b = p ? 0 : 1;
              break;
            case 123:
              v = !0, b = 0;
              break;
            case 125:
              b = v ? 0 : 1;
              break;
            case 39:
            case 34:
            case 96:
              h === x ? b = 1 : h === 39 || h === 34 || h === 96 ? b = 0 : b = 1;
              break;
            case 42:
              b = h === 42 ? 1 : 0;
              break;
            case 124:
              b = h === 124 ? 1 : 0;
              break;
            case 32:
              b = m ? 0 : 1;
              break;
            default:
              b = n.get(x);
          }
          b === 1 && (i.push(Yt._createLink(n, o, s, f, u)), y = !0);
        } else if (d === 12) {
          let b;
          x === 91 ? (p = !0, b = 0) : b = n.get(x), b === 1 ? y = !0 : d = 13;
        } else
          d = r.nextState(d, x), d === 0 && (y = !0);
        y && (d = 1, g = !1, p = !1, v = !1, f = u + 1, h = x), u++;
      }
      d === 13 && i.push(Yt._createLink(n, o, s, f, l));
    }
    return i;
  }
}
function No(e) {
  return !e || typeof e.getLineCount != "function" || typeof e.getLineContent != "function" ? [] : Yt.computeLinks(e);
}
class Ir {
  constructor() {
    this._defaultValueSet = [
      ["true", "false"],
      ["True", "False"],
      ["Private", "Public", "Friend", "ReadOnly", "Partial", "Protected", "WriteOnly"],
      ["public", "protected", "private"]
    ];
  }
  navigateValueSet(t, r, n, i, s) {
    if (t && r) {
      const a = this.doNavigateValueSet(r, s);
      if (a)
        return {
          range: t,
          value: a
        };
    }
    if (n && i) {
      const a = this.doNavigateValueSet(i, s);
      if (a)
        return {
          range: n,
          value: a
        };
    }
    return null;
  }
  doNavigateValueSet(t, r) {
    const n = this.numberReplace(t, r);
    return n !== null ? n : this.textReplace(t, r);
  }
  numberReplace(t, r) {
    const n = Math.pow(10, t.length - (t.lastIndexOf(".") + 1));
    let i = Number(t);
    const s = parseFloat(t);
    return !isNaN(i) && !isNaN(s) && i === s ? i === 0 && !r ? null : (i = Math.floor(i * n), i += r ? n : -n, String(i / n)) : null;
  }
  textReplace(t, r) {
    return this.valueSetsReplace(this._defaultValueSet, t, r);
  }
  valueSetsReplace(t, r, n) {
    let i = null;
    for (let s = 0, a = t.length; i === null && s < a; s++)
      i = this.valueSetReplace(t[s], r, n);
    return i;
  }
  valueSetReplace(t, r, n) {
    let i = t.indexOf(r);
    return i >= 0 ? (i += n ? 1 : -1, i < 0 ? i = t.length - 1 : i %= t.length, t[i]) : null;
  }
}
Ir.INSTANCE = new Ir();
class cn {
  constructor() {
    this._keyCodeToStr = [], this._strToKeyCode = /* @__PURE__ */ Object.create(null);
  }
  define(t, r) {
    this._keyCodeToStr[t] = r, this._strToKeyCode[r.toLowerCase()] = t;
  }
  keyCodeToStr(t) {
    return this._keyCodeToStr[t];
  }
  strToKeyCode(t) {
    return this._strToKeyCode[t.toLowerCase()] || 0;
  }
}
const Ht = new cn(), Vr = new cn(), Dr = new cn(), Lo = new Array(230), Co = /* @__PURE__ */ Object.create(null), ko = /* @__PURE__ */ Object.create(null);
(function() {
  const e = "", t = [
    // keyCodeOrd, immutable, scanCode, scanCodeStr, keyCode, keyCodeStr, eventKeyCode, vkey, usUserSettingsLabel, generalUserSettingsLabel
    [0, 1, 0, "None", 0, "unknown", 0, "VK_UNKNOWN", e, e],
    [0, 1, 1, "Hyper", 0, e, 0, e, e, e],
    [0, 1, 2, "Super", 0, e, 0, e, e, e],
    [0, 1, 3, "Fn", 0, e, 0, e, e, e],
    [0, 1, 4, "FnLock", 0, e, 0, e, e, e],
    [0, 1, 5, "Suspend", 0, e, 0, e, e, e],
    [0, 1, 6, "Resume", 0, e, 0, e, e, e],
    [0, 1, 7, "Turbo", 0, e, 0, e, e, e],
    [0, 1, 8, "Sleep", 0, e, 0, "VK_SLEEP", e, e],
    [0, 1, 9, "WakeUp", 0, e, 0, e, e, e],
    [31, 0, 10, "KeyA", 31, "A", 65, "VK_A", e, e],
    [32, 0, 11, "KeyB", 32, "B", 66, "VK_B", e, e],
    [33, 0, 12, "KeyC", 33, "C", 67, "VK_C", e, e],
    [34, 0, 13, "KeyD", 34, "D", 68, "VK_D", e, e],
    [35, 0, 14, "KeyE", 35, "E", 69, "VK_E", e, e],
    [36, 0, 15, "KeyF", 36, "F", 70, "VK_F", e, e],
    [37, 0, 16, "KeyG", 37, "G", 71, "VK_G", e, e],
    [38, 0, 17, "KeyH", 38, "H", 72, "VK_H", e, e],
    [39, 0, 18, "KeyI", 39, "I", 73, "VK_I", e, e],
    [40, 0, 19, "KeyJ", 40, "J", 74, "VK_J", e, e],
    [41, 0, 20, "KeyK", 41, "K", 75, "VK_K", e, e],
    [42, 0, 21, "KeyL", 42, "L", 76, "VK_L", e, e],
    [43, 0, 22, "KeyM", 43, "M", 77, "VK_M", e, e],
    [44, 0, 23, "KeyN", 44, "N", 78, "VK_N", e, e],
    [45, 0, 24, "KeyO", 45, "O", 79, "VK_O", e, e],
    [46, 0, 25, "KeyP", 46, "P", 80, "VK_P", e, e],
    [47, 0, 26, "KeyQ", 47, "Q", 81, "VK_Q", e, e],
    [48, 0, 27, "KeyR", 48, "R", 82, "VK_R", e, e],
    [49, 0, 28, "KeyS", 49, "S", 83, "VK_S", e, e],
    [50, 0, 29, "KeyT", 50, "T", 84, "VK_T", e, e],
    [51, 0, 30, "KeyU", 51, "U", 85, "VK_U", e, e],
    [52, 0, 31, "KeyV", 52, "V", 86, "VK_V", e, e],
    [53, 0, 32, "KeyW", 53, "W", 87, "VK_W", e, e],
    [54, 0, 33, "KeyX", 54, "X", 88, "VK_X", e, e],
    [55, 0, 34, "KeyY", 55, "Y", 89, "VK_Y", e, e],
    [56, 0, 35, "KeyZ", 56, "Z", 90, "VK_Z", e, e],
    [22, 0, 36, "Digit1", 22, "1", 49, "VK_1", e, e],
    [23, 0, 37, "Digit2", 23, "2", 50, "VK_2", e, e],
    [24, 0, 38, "Digit3", 24, "3", 51, "VK_3", e, e],
    [25, 0, 39, "Digit4", 25, "4", 52, "VK_4", e, e],
    [26, 0, 40, "Digit5", 26, "5", 53, "VK_5", e, e],
    [27, 0, 41, "Digit6", 27, "6", 54, "VK_6", e, e],
    [28, 0, 42, "Digit7", 28, "7", 55, "VK_7", e, e],
    [29, 0, 43, "Digit8", 29, "8", 56, "VK_8", e, e],
    [30, 0, 44, "Digit9", 30, "9", 57, "VK_9", e, e],
    [21, 0, 45, "Digit0", 21, "0", 48, "VK_0", e, e],
    [3, 1, 46, "Enter", 3, "Enter", 13, "VK_RETURN", e, e],
    [9, 1, 47, "Escape", 9, "Escape", 27, "VK_ESCAPE", e, e],
    [1, 1, 48, "Backspace", 1, "Backspace", 8, "VK_BACK", e, e],
    [2, 1, 49, "Tab", 2, "Tab", 9, "VK_TAB", e, e],
    [10, 1, 50, "Space", 10, "Space", 32, "VK_SPACE", e, e],
    [83, 0, 51, "Minus", 83, "-", 189, "VK_OEM_MINUS", "-", "OEM_MINUS"],
    [81, 0, 52, "Equal", 81, "=", 187, "VK_OEM_PLUS", "=", "OEM_PLUS"],
    [87, 0, 53, "BracketLeft", 87, "[", 219, "VK_OEM_4", "[", "OEM_4"],
    [89, 0, 54, "BracketRight", 89, "]", 221, "VK_OEM_6", "]", "OEM_6"],
    [88, 0, 55, "Backslash", 88, "\\", 220, "VK_OEM_5", "\\", "OEM_5"],
    [0, 0, 56, "IntlHash", 0, e, 0, e, e, e],
    [80, 0, 57, "Semicolon", 80, ";", 186, "VK_OEM_1", ";", "OEM_1"],
    [90, 0, 58, "Quote", 90, "'", 222, "VK_OEM_7", "'", "OEM_7"],
    [86, 0, 59, "Backquote", 86, "`", 192, "VK_OEM_3", "`", "OEM_3"],
    [82, 0, 60, "Comma", 82, ",", 188, "VK_OEM_COMMA", ",", "OEM_COMMA"],
    [84, 0, 61, "Period", 84, ".", 190, "VK_OEM_PERIOD", ".", "OEM_PERIOD"],
    [85, 0, 62, "Slash", 85, "/", 191, "VK_OEM_2", "/", "OEM_2"],
    [8, 1, 63, "CapsLock", 8, "CapsLock", 20, "VK_CAPITAL", e, e],
    [59, 1, 64, "F1", 59, "F1", 112, "VK_F1", e, e],
    [60, 1, 65, "F2", 60, "F2", 113, "VK_F2", e, e],
    [61, 1, 66, "F3", 61, "F3", 114, "VK_F3", e, e],
    [62, 1, 67, "F4", 62, "F4", 115, "VK_F4", e, e],
    [63, 1, 68, "F5", 63, "F5", 116, "VK_F5", e, e],
    [64, 1, 69, "F6", 64, "F6", 117, "VK_F6", e, e],
    [65, 1, 70, "F7", 65, "F7", 118, "VK_F7", e, e],
    [66, 1, 71, "F8", 66, "F8", 119, "VK_F8", e, e],
    [67, 1, 72, "F9", 67, "F9", 120, "VK_F9", e, e],
    [68, 1, 73, "F10", 68, "F10", 121, "VK_F10", e, e],
    [69, 1, 74, "F11", 69, "F11", 122, "VK_F11", e, e],
    [70, 1, 75, "F12", 70, "F12", 123, "VK_F12", e, e],
    [0, 1, 76, "PrintScreen", 0, e, 0, e, e, e],
    [79, 1, 77, "ScrollLock", 79, "ScrollLock", 145, "VK_SCROLL", e, e],
    [7, 1, 78, "Pause", 7, "PauseBreak", 19, "VK_PAUSE", e, e],
    [19, 1, 79, "Insert", 19, "Insert", 45, "VK_INSERT", e, e],
    [14, 1, 80, "Home", 14, "Home", 36, "VK_HOME", e, e],
    [11, 1, 81, "PageUp", 11, "PageUp", 33, "VK_PRIOR", e, e],
    [20, 1, 82, "Delete", 20, "Delete", 46, "VK_DELETE", e, e],
    [13, 1, 83, "End", 13, "End", 35, "VK_END", e, e],
    [12, 1, 84, "PageDown", 12, "PageDown", 34, "VK_NEXT", e, e],
    [17, 1, 85, "ArrowRight", 17, "RightArrow", 39, "VK_RIGHT", "Right", e],
    [15, 1, 86, "ArrowLeft", 15, "LeftArrow", 37, "VK_LEFT", "Left", e],
    [18, 1, 87, "ArrowDown", 18, "DownArrow", 40, "VK_DOWN", "Down", e],
    [16, 1, 88, "ArrowUp", 16, "UpArrow", 38, "VK_UP", "Up", e],
    [78, 1, 89, "NumLock", 78, "NumLock", 144, "VK_NUMLOCK", e, e],
    [108, 1, 90, "NumpadDivide", 108, "NumPad_Divide", 111, "VK_DIVIDE", e, e],
    [103, 1, 91, "NumpadMultiply", 103, "NumPad_Multiply", 106, "VK_MULTIPLY", e, e],
    [106, 1, 92, "NumpadSubtract", 106, "NumPad_Subtract", 109, "VK_SUBTRACT", e, e],
    [104, 1, 93, "NumpadAdd", 104, "NumPad_Add", 107, "VK_ADD", e, e],
    [3, 1, 94, "NumpadEnter", 3, e, 0, e, e, e],
    [94, 1, 95, "Numpad1", 94, "NumPad1", 97, "VK_NUMPAD1", e, e],
    [95, 1, 96, "Numpad2", 95, "NumPad2", 98, "VK_NUMPAD2", e, e],
    [96, 1, 97, "Numpad3", 96, "NumPad3", 99, "VK_NUMPAD3", e, e],
    [97, 1, 98, "Numpad4", 97, "NumPad4", 100, "VK_NUMPAD4", e, e],
    [98, 1, 99, "Numpad5", 98, "NumPad5", 101, "VK_NUMPAD5", e, e],
    [99, 1, 100, "Numpad6", 99, "NumPad6", 102, "VK_NUMPAD6", e, e],
    [100, 1, 101, "Numpad7", 100, "NumPad7", 103, "VK_NUMPAD7", e, e],
    [101, 1, 102, "Numpad8", 101, "NumPad8", 104, "VK_NUMPAD8", e, e],
    [102, 1, 103, "Numpad9", 102, "NumPad9", 105, "VK_NUMPAD9", e, e],
    [93, 1, 104, "Numpad0", 93, "NumPad0", 96, "VK_NUMPAD0", e, e],
    [107, 1, 105, "NumpadDecimal", 107, "NumPad_Decimal", 110, "VK_DECIMAL", e, e],
    [92, 0, 106, "IntlBackslash", 92, "OEM_102", 226, "VK_OEM_102", e, e],
    [58, 1, 107, "ContextMenu", 58, "ContextMenu", 93, e, e, e],
    [0, 1, 108, "Power", 0, e, 0, e, e, e],
    [0, 1, 109, "NumpadEqual", 0, e, 0, e, e, e],
    [71, 1, 110, "F13", 71, "F13", 124, "VK_F13", e, e],
    [72, 1, 111, "F14", 72, "F14", 125, "VK_F14", e, e],
    [73, 1, 112, "F15", 73, "F15", 126, "VK_F15", e, e],
    [74, 1, 113, "F16", 74, "F16", 127, "VK_F16", e, e],
    [75, 1, 114, "F17", 75, "F17", 128, "VK_F17", e, e],
    [76, 1, 115, "F18", 76, "F18", 129, "VK_F18", e, e],
    [77, 1, 116, "F19", 77, "F19", 130, "VK_F19", e, e],
    [0, 1, 117, "F20", 0, e, 0, "VK_F20", e, e],
    [0, 1, 118, "F21", 0, e, 0, "VK_F21", e, e],
    [0, 1, 119, "F22", 0, e, 0, "VK_F22", e, e],
    [0, 1, 120, "F23", 0, e, 0, "VK_F23", e, e],
    [0, 1, 121, "F24", 0, e, 0, "VK_F24", e, e],
    [0, 1, 122, "Open", 0, e, 0, e, e, e],
    [0, 1, 123, "Help", 0, e, 0, e, e, e],
    [0, 1, 124, "Select", 0, e, 0, e, e, e],
    [0, 1, 125, "Again", 0, e, 0, e, e, e],
    [0, 1, 126, "Undo", 0, e, 0, e, e, e],
    [0, 1, 127, "Cut", 0, e, 0, e, e, e],
    [0, 1, 128, "Copy", 0, e, 0, e, e, e],
    [0, 1, 129, "Paste", 0, e, 0, e, e, e],
    [0, 1, 130, "Find", 0, e, 0, e, e, e],
    [0, 1, 131, "AudioVolumeMute", 112, "AudioVolumeMute", 173, "VK_VOLUME_MUTE", e, e],
    [0, 1, 132, "AudioVolumeUp", 113, "AudioVolumeUp", 175, "VK_VOLUME_UP", e, e],
    [0, 1, 133, "AudioVolumeDown", 114, "AudioVolumeDown", 174, "VK_VOLUME_DOWN", e, e],
    [105, 1, 134, "NumpadComma", 105, "NumPad_Separator", 108, "VK_SEPARATOR", e, e],
    [110, 0, 135, "IntlRo", 110, "ABNT_C1", 193, "VK_ABNT_C1", e, e],
    [0, 1, 136, "KanaMode", 0, e, 0, e, e, e],
    [0, 0, 137, "IntlYen", 0, e, 0, e, e, e],
    [0, 1, 138, "Convert", 0, e, 0, e, e, e],
    [0, 1, 139, "NonConvert", 0, e, 0, e, e, e],
    [0, 1, 140, "Lang1", 0, e, 0, e, e, e],
    [0, 1, 141, "Lang2", 0, e, 0, e, e, e],
    [0, 1, 142, "Lang3", 0, e, 0, e, e, e],
    [0, 1, 143, "Lang4", 0, e, 0, e, e, e],
    [0, 1, 144, "Lang5", 0, e, 0, e, e, e],
    [0, 1, 145, "Abort", 0, e, 0, e, e, e],
    [0, 1, 146, "Props", 0, e, 0, e, e, e],
    [0, 1, 147, "NumpadParenLeft", 0, e, 0, e, e, e],
    [0, 1, 148, "NumpadParenRight", 0, e, 0, e, e, e],
    [0, 1, 149, "NumpadBackspace", 0, e, 0, e, e, e],
    [0, 1, 150, "NumpadMemoryStore", 0, e, 0, e, e, e],
    [0, 1, 151, "NumpadMemoryRecall", 0, e, 0, e, e, e],
    [0, 1, 152, "NumpadMemoryClear", 0, e, 0, e, e, e],
    [0, 1, 153, "NumpadMemoryAdd", 0, e, 0, e, e, e],
    [0, 1, 154, "NumpadMemorySubtract", 0, e, 0, e, e, e],
    [0, 1, 155, "NumpadClear", 126, "Clear", 12, "VK_CLEAR", e, e],
    [0, 1, 156, "NumpadClearEntry", 0, e, 0, e, e, e],
    [5, 1, 0, e, 5, "Ctrl", 17, "VK_CONTROL", e, e],
    [4, 1, 0, e, 4, "Shift", 16, "VK_SHIFT", e, e],
    [6, 1, 0, e, 6, "Alt", 18, "VK_MENU", e, e],
    [57, 1, 0, e, 57, "Meta", 91, "VK_COMMAND", e, e],
    [5, 1, 157, "ControlLeft", 5, e, 0, "VK_LCONTROL", e, e],
    [4, 1, 158, "ShiftLeft", 4, e, 0, "VK_LSHIFT", e, e],
    [6, 1, 159, "AltLeft", 6, e, 0, "VK_LMENU", e, e],
    [57, 1, 160, "MetaLeft", 57, e, 0, "VK_LWIN", e, e],
    [5, 1, 161, "ControlRight", 5, e, 0, "VK_RCONTROL", e, e],
    [4, 1, 162, "ShiftRight", 4, e, 0, "VK_RSHIFT", e, e],
    [6, 1, 163, "AltRight", 6, e, 0, "VK_RMENU", e, e],
    [57, 1, 164, "MetaRight", 57, e, 0, "VK_RWIN", e, e],
    [0, 1, 165, "BrightnessUp", 0, e, 0, e, e, e],
    [0, 1, 166, "BrightnessDown", 0, e, 0, e, e, e],
    [0, 1, 167, "MediaPlay", 0, e, 0, e, e, e],
    [0, 1, 168, "MediaRecord", 0, e, 0, e, e, e],
    [0, 1, 169, "MediaFastForward", 0, e, 0, e, e, e],
    [0, 1, 170, "MediaRewind", 0, e, 0, e, e, e],
    [114, 1, 171, "MediaTrackNext", 119, "MediaTrackNext", 176, "VK_MEDIA_NEXT_TRACK", e, e],
    [115, 1, 172, "MediaTrackPrevious", 120, "MediaTrackPrevious", 177, "VK_MEDIA_PREV_TRACK", e, e],
    [116, 1, 173, "MediaStop", 121, "MediaStop", 178, "VK_MEDIA_STOP", e, e],
    [0, 1, 174, "Eject", 0, e, 0, e, e, e],
    [117, 1, 175, "MediaPlayPause", 122, "MediaPlayPause", 179, "VK_MEDIA_PLAY_PAUSE", e, e],
    [0, 1, 176, "MediaSelect", 123, "LaunchMediaPlayer", 181, "VK_MEDIA_LAUNCH_MEDIA_SELECT", e, e],
    [0, 1, 177, "LaunchMail", 124, "LaunchMail", 180, "VK_MEDIA_LAUNCH_MAIL", e, e],
    [0, 1, 178, "LaunchApp2", 125, "LaunchApp2", 183, "VK_MEDIA_LAUNCH_APP2", e, e],
    [0, 1, 179, "LaunchApp1", 0, e, 0, "VK_MEDIA_LAUNCH_APP1", e, e],
    [0, 1, 180, "SelectTask", 0, e, 0, e, e, e],
    [0, 1, 181, "LaunchScreenSaver", 0, e, 0, e, e, e],
    [0, 1, 182, "BrowserSearch", 115, "BrowserSearch", 170, "VK_BROWSER_SEARCH", e, e],
    [0, 1, 183, "BrowserHome", 116, "BrowserHome", 172, "VK_BROWSER_HOME", e, e],
    [112, 1, 184, "BrowserBack", 117, "BrowserBack", 166, "VK_BROWSER_BACK", e, e],
    [113, 1, 185, "BrowserForward", 118, "BrowserForward", 167, "VK_BROWSER_FORWARD", e, e],
    [0, 1, 186, "BrowserStop", 0, e, 0, "VK_BROWSER_STOP", e, e],
    [0, 1, 187, "BrowserRefresh", 0, e, 0, "VK_BROWSER_REFRESH", e, e],
    [0, 1, 188, "BrowserFavorites", 0, e, 0, "VK_BROWSER_FAVORITES", e, e],
    [0, 1, 189, "ZoomToggle", 0, e, 0, e, e, e],
    [0, 1, 190, "MailReply", 0, e, 0, e, e, e],
    [0, 1, 191, "MailForward", 0, e, 0, e, e, e],
    [0, 1, 192, "MailSend", 0, e, 0, e, e, e],
    // See https://lists.w3.org/Archives/Public/www-dom/2010JulSep/att-0182/keyCode-spec.html
    // If an Input Method Editor is processing key input and the event is keydown, return 229.
    [109, 1, 0, e, 109, "KeyInComposition", 229, e, e, e],
    [111, 1, 0, e, 111, "ABNT_C2", 194, "VK_ABNT_C2", e, e],
    [91, 1, 0, e, 91, "OEM_8", 223, "VK_OEM_8", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_KANA", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_HANGUL", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_JUNJA", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_FINAL", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_HANJA", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_KANJI", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_CONVERT", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_NONCONVERT", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_ACCEPT", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_MODECHANGE", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_SELECT", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_PRINT", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_EXECUTE", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_SNAPSHOT", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_HELP", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_APPS", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_PROCESSKEY", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_PACKET", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_DBE_SBCSCHAR", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_DBE_DBCSCHAR", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_ATTN", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_CRSEL", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_EXSEL", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_EREOF", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_PLAY", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_ZOOM", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_NONAME", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_PA1", e, e],
    [0, 1, 0, e, 0, e, 0, "VK_OEM_CLEAR", e, e]
  ], r = [], n = [];
  for (const i of t) {
    const [s, a, o, l, u, f, h, d, g, p] = i;
    if (n[o] || (n[o] = !0, Co[l] = o, ko[l.toLowerCase()] = o), !r[u]) {
      if (r[u] = !0, !f)
        throw new Error(`String representation missing for key code ${u} around scan code ${l}`);
      Ht.define(u, f), Vr.define(u, g || f), Dr.define(u, p || g || f);
    }
    h && (Lo[h] = u);
  }
})();
var Mn;
(function(e) {
  function t(o) {
    return Ht.keyCodeToStr(o);
  }
  e.toString = t;
  function r(o) {
    return Ht.strToKeyCode(o);
  }
  e.fromString = r;
  function n(o) {
    return Vr.keyCodeToStr(o);
  }
  e.toUserSettingsUS = n;
  function i(o) {
    return Dr.keyCodeToStr(o);
  }
  e.toUserSettingsGeneral = i;
  function s(o) {
    return Vr.strToKeyCode(o) || Dr.strToKeyCode(o);
  }
  e.fromUserSettings = s;
  function a(o) {
    if (o >= 93 && o <= 108)
      return null;
    switch (o) {
      case 16:
        return "Up";
      case 18:
        return "Down";
      case 15:
        return "Left";
      case 17:
        return "Right";
    }
    return Ht.keyCodeToStr(o);
  }
  e.toElectronAccelerator = a;
})(Mn || (Mn = {}));
function Eo(e, t) {
  const r = (t & 65535) << 16 >>> 0;
  return (e | r) >>> 0;
}
class me extends fe {
  constructor(t, r, n, i) {
    super(t, r, n, i), this.selectionStartLineNumber = t, this.selectionStartColumn = r, this.positionLineNumber = n, this.positionColumn = i;
  }
  /**
   * Transform to a human-readable representation.
   */
  toString() {
    return "[" + this.selectionStartLineNumber + "," + this.selectionStartColumn + " -> " + this.positionLineNumber + "," + this.positionColumn + "]";
  }
  /**
   * Test if equals other selection.
   */
  equalsSelection(t) {
    return me.selectionsEqual(this, t);
  }
  /**
   * Test if the two selections are equal.
   */
  static selectionsEqual(t, r) {
    return t.selectionStartLineNumber === r.selectionStartLineNumber && t.selectionStartColumn === r.selectionStartColumn && t.positionLineNumber === r.positionLineNumber && t.positionColumn === r.positionColumn;
  }
  /**
   * Get directions (LTR or RTL).
   */
  getDirection() {
    return this.selectionStartLineNumber === this.startLineNumber && this.selectionStartColumn === this.startColumn ? 0 : 1;
  }
  /**
   * Create a new selection with a different `positionLineNumber` and `positionColumn`.
   */
  setEndPosition(t, r) {
    return this.getDirection() === 0 ? new me(this.startLineNumber, this.startColumn, t, r) : new me(t, r, this.startLineNumber, this.startColumn);
  }
  /**
   * Get the position at `positionLineNumber` and `positionColumn`.
   */
  getPosition() {
    return new Re(this.positionLineNumber, this.positionColumn);
  }
  /**
   * Get the position at the start of the selection.
  */
  getSelectionStart() {
    return new Re(this.selectionStartLineNumber, this.selectionStartColumn);
  }
  /**
   * Create a new selection with a different `selectionStartLineNumber` and `selectionStartColumn`.
   */
  setStartPosition(t, r) {
    return this.getDirection() === 0 ? new me(t, r, this.endLineNumber, this.endColumn) : new me(this.endLineNumber, this.endColumn, t, r);
  }
  // ----
  /**
   * Create a `Selection` from one or two positions
   */
  static fromPositions(t, r = t) {
    return new me(t.lineNumber, t.column, r.lineNumber, r.column);
  }
  /**
   * Creates a `Selection` from a range, given a direction.
   */
  static fromRange(t, r) {
    return r === 0 ? new me(t.startLineNumber, t.startColumn, t.endLineNumber, t.endColumn) : new me(t.endLineNumber, t.endColumn, t.startLineNumber, t.startColumn);
  }
  /**
   * Create a `Selection` from an `ISelection`.
   */
  static liftSelection(t) {
    return new me(t.selectionStartLineNumber, t.selectionStartColumn, t.positionLineNumber, t.positionColumn);
  }
  /**
   * `a` equals `b`.
   */
  static selectionsArrEqual(t, r) {
    if (t && !r || !t && r)
      return !1;
    if (!t && !r)
      return !0;
    if (t.length !== r.length)
      return !1;
    for (let n = 0, i = t.length; n < i; n++)
      if (!this.selectionsEqual(t[n], r[n]))
        return !1;
    return !0;
  }
  /**
   * Test if `obj` is an `ISelection`.
   */
  static isISelection(t) {
    return t && typeof t.selectionStartLineNumber == "number" && typeof t.selectionStartColumn == "number" && typeof t.positionLineNumber == "number" && typeof t.positionColumn == "number";
  }
  /**
   * Create with a direction.
   */
  static createWithDirection(t, r, n, i, s) {
    return s === 0 ? new me(t, r, n, i) : new me(n, i, t, r);
  }
}
const Tn = /* @__PURE__ */ Object.create(null);
function c(e, t) {
  if (Ca(t)) {
    const r = Tn[t];
    if (r === void 0)
      throw new Error(`${e} references an unknown codicon: ${t}`);
    t = r;
  }
  return Tn[e] = t, { id: e };
}
const O = {
  // built-in icons, with image name
  add: c("add", 6e4),
  plus: c("plus", 6e4),
  gistNew: c("gist-new", 6e4),
  repoCreate: c("repo-create", 6e4),
  lightbulb: c("lightbulb", 60001),
  lightBulb: c("light-bulb", 60001),
  repo: c("repo", 60002),
  repoDelete: c("repo-delete", 60002),
  gistFork: c("gist-fork", 60003),
  repoForked: c("repo-forked", 60003),
  gitPullRequest: c("git-pull-request", 60004),
  gitPullRequestAbandoned: c("git-pull-request-abandoned", 60004),
  recordKeys: c("record-keys", 60005),
  keyboard: c("keyboard", 60005),
  tag: c("tag", 60006),
  tagAdd: c("tag-add", 60006),
  tagRemove: c("tag-remove", 60006),
  person: c("person", 60007),
  personFollow: c("person-follow", 60007),
  personOutline: c("person-outline", 60007),
  personFilled: c("person-filled", 60007),
  gitBranch: c("git-branch", 60008),
  gitBranchCreate: c("git-branch-create", 60008),
  gitBranchDelete: c("git-branch-delete", 60008),
  sourceControl: c("source-control", 60008),
  mirror: c("mirror", 60009),
  mirrorPublic: c("mirror-public", 60009),
  star: c("star", 60010),
  starAdd: c("star-add", 60010),
  starDelete: c("star-delete", 60010),
  starEmpty: c("star-empty", 60010),
  comment: c("comment", 60011),
  commentAdd: c("comment-add", 60011),
  alert: c("alert", 60012),
  warning: c("warning", 60012),
  search: c("search", 60013),
  searchSave: c("search-save", 60013),
  logOut: c("log-out", 60014),
  signOut: c("sign-out", 60014),
  logIn: c("log-in", 60015),
  signIn: c("sign-in", 60015),
  eye: c("eye", 60016),
  eyeUnwatch: c("eye-unwatch", 60016),
  eyeWatch: c("eye-watch", 60016),
  circleFilled: c("circle-filled", 60017),
  primitiveDot: c("primitive-dot", 60017),
  closeDirty: c("close-dirty", 60017),
  debugBreakpoint: c("debug-breakpoint", 60017),
  debugBreakpointDisabled: c("debug-breakpoint-disabled", 60017),
  debugHint: c("debug-hint", 60017),
  primitiveSquare: c("primitive-square", 60018),
  edit: c("edit", 60019),
  pencil: c("pencil", 60019),
  info: c("info", 60020),
  issueOpened: c("issue-opened", 60020),
  gistPrivate: c("gist-private", 60021),
  gitForkPrivate: c("git-fork-private", 60021),
  lock: c("lock", 60021),
  mirrorPrivate: c("mirror-private", 60021),
  close: c("close", 60022),
  removeClose: c("remove-close", 60022),
  x: c("x", 60022),
  repoSync: c("repo-sync", 60023),
  sync: c("sync", 60023),
  clone: c("clone", 60024),
  desktopDownload: c("desktop-download", 60024),
  beaker: c("beaker", 60025),
  microscope: c("microscope", 60025),
  vm: c("vm", 60026),
  deviceDesktop: c("device-desktop", 60026),
  file: c("file", 60027),
  fileText: c("file-text", 60027),
  more: c("more", 60028),
  ellipsis: c("ellipsis", 60028),
  kebabHorizontal: c("kebab-horizontal", 60028),
  mailReply: c("mail-reply", 60029),
  reply: c("reply", 60029),
  organization: c("organization", 60030),
  organizationFilled: c("organization-filled", 60030),
  organizationOutline: c("organization-outline", 60030),
  newFile: c("new-file", 60031),
  fileAdd: c("file-add", 60031),
  newFolder: c("new-folder", 60032),
  fileDirectoryCreate: c("file-directory-create", 60032),
  trash: c("trash", 60033),
  trashcan: c("trashcan", 60033),
  history: c("history", 60034),
  clock: c("clock", 60034),
  folder: c("folder", 60035),
  fileDirectory: c("file-directory", 60035),
  symbolFolder: c("symbol-folder", 60035),
  logoGithub: c("logo-github", 60036),
  markGithub: c("mark-github", 60036),
  github: c("github", 60036),
  terminal: c("terminal", 60037),
  console: c("console", 60037),
  repl: c("repl", 60037),
  zap: c("zap", 60038),
  symbolEvent: c("symbol-event", 60038),
  error: c("error", 60039),
  stop: c("stop", 60039),
  variable: c("variable", 60040),
  symbolVariable: c("symbol-variable", 60040),
  array: c("array", 60042),
  symbolArray: c("symbol-array", 60042),
  symbolModule: c("symbol-module", 60043),
  symbolPackage: c("symbol-package", 60043),
  symbolNamespace: c("symbol-namespace", 60043),
  symbolObject: c("symbol-object", 60043),
  symbolMethod: c("symbol-method", 60044),
  symbolFunction: c("symbol-function", 60044),
  symbolConstructor: c("symbol-constructor", 60044),
  symbolBoolean: c("symbol-boolean", 60047),
  symbolNull: c("symbol-null", 60047),
  symbolNumeric: c("symbol-numeric", 60048),
  symbolNumber: c("symbol-number", 60048),
  symbolStructure: c("symbol-structure", 60049),
  symbolStruct: c("symbol-struct", 60049),
  symbolParameter: c("symbol-parameter", 60050),
  symbolTypeParameter: c("symbol-type-parameter", 60050),
  symbolKey: c("symbol-key", 60051),
  symbolText: c("symbol-text", 60051),
  symbolReference: c("symbol-reference", 60052),
  goToFile: c("go-to-file", 60052),
  symbolEnum: c("symbol-enum", 60053),
  symbolValue: c("symbol-value", 60053),
  symbolRuler: c("symbol-ruler", 60054),
  symbolUnit: c("symbol-unit", 60054),
  activateBreakpoints: c("activate-breakpoints", 60055),
  archive: c("archive", 60056),
  arrowBoth: c("arrow-both", 60057),
  arrowDown: c("arrow-down", 60058),
  arrowLeft: c("arrow-left", 60059),
  arrowRight: c("arrow-right", 60060),
  arrowSmallDown: c("arrow-small-down", 60061),
  arrowSmallLeft: c("arrow-small-left", 60062),
  arrowSmallRight: c("arrow-small-right", 60063),
  arrowSmallUp: c("arrow-small-up", 60064),
  arrowUp: c("arrow-up", 60065),
  bell: c("bell", 60066),
  bold: c("bold", 60067),
  book: c("book", 60068),
  bookmark: c("bookmark", 60069),
  debugBreakpointConditionalUnverified: c("debug-breakpoint-conditional-unverified", 60070),
  debugBreakpointConditional: c("debug-breakpoint-conditional", 60071),
  debugBreakpointConditionalDisabled: c("debug-breakpoint-conditional-disabled", 60071),
  debugBreakpointDataUnverified: c("debug-breakpoint-data-unverified", 60072),
  debugBreakpointData: c("debug-breakpoint-data", 60073),
  debugBreakpointDataDisabled: c("debug-breakpoint-data-disabled", 60073),
  debugBreakpointLogUnverified: c("debug-breakpoint-log-unverified", 60074),
  debugBreakpointLog: c("debug-breakpoint-log", 60075),
  debugBreakpointLogDisabled: c("debug-breakpoint-log-disabled", 60075),
  briefcase: c("briefcase", 60076),
  broadcast: c("broadcast", 60077),
  browser: c("browser", 60078),
  bug: c("bug", 60079),
  calendar: c("calendar", 60080),
  caseSensitive: c("case-sensitive", 60081),
  check: c("check", 60082),
  checklist: c("checklist", 60083),
  chevronDown: c("chevron-down", 60084),
  dropDownButton: c("drop-down-button", 60084),
  chevronLeft: c("chevron-left", 60085),
  chevronRight: c("chevron-right", 60086),
  chevronUp: c("chevron-up", 60087),
  chromeClose: c("chrome-close", 60088),
  chromeMaximize: c("chrome-maximize", 60089),
  chromeMinimize: c("chrome-minimize", 60090),
  chromeRestore: c("chrome-restore", 60091),
  circle: c("circle", 60092),
  circleOutline: c("circle-outline", 60092),
  debugBreakpointUnverified: c("debug-breakpoint-unverified", 60092),
  circleSlash: c("circle-slash", 60093),
  circuitBoard: c("circuit-board", 60094),
  clearAll: c("clear-all", 60095),
  clippy: c("clippy", 60096),
  closeAll: c("close-all", 60097),
  cloudDownload: c("cloud-download", 60098),
  cloudUpload: c("cloud-upload", 60099),
  code: c("code", 60100),
  collapseAll: c("collapse-all", 60101),
  colorMode: c("color-mode", 60102),
  commentDiscussion: c("comment-discussion", 60103),
  compareChanges: c("compare-changes", 60157),
  creditCard: c("credit-card", 60105),
  dash: c("dash", 60108),
  dashboard: c("dashboard", 60109),
  database: c("database", 60110),
  debugContinue: c("debug-continue", 60111),
  debugDisconnect: c("debug-disconnect", 60112),
  debugPause: c("debug-pause", 60113),
  debugRestart: c("debug-restart", 60114),
  debugStart: c("debug-start", 60115),
  debugStepInto: c("debug-step-into", 60116),
  debugStepOut: c("debug-step-out", 60117),
  debugStepOver: c("debug-step-over", 60118),
  debugStop: c("debug-stop", 60119),
  debug: c("debug", 60120),
  deviceCameraVideo: c("device-camera-video", 60121),
  deviceCamera: c("device-camera", 60122),
  deviceMobile: c("device-mobile", 60123),
  diffAdded: c("diff-added", 60124),
  diffIgnored: c("diff-ignored", 60125),
  diffModified: c("diff-modified", 60126),
  diffRemoved: c("diff-removed", 60127),
  diffRenamed: c("diff-renamed", 60128),
  diff: c("diff", 60129),
  discard: c("discard", 60130),
  editorLayout: c("editor-layout", 60131),
  emptyWindow: c("empty-window", 60132),
  exclude: c("exclude", 60133),
  extensions: c("extensions", 60134),
  eyeClosed: c("eye-closed", 60135),
  fileBinary: c("file-binary", 60136),
  fileCode: c("file-code", 60137),
  fileMedia: c("file-media", 60138),
  filePdf: c("file-pdf", 60139),
  fileSubmodule: c("file-submodule", 60140),
  fileSymlinkDirectory: c("file-symlink-directory", 60141),
  fileSymlinkFile: c("file-symlink-file", 60142),
  fileZip: c("file-zip", 60143),
  files: c("files", 60144),
  filter: c("filter", 60145),
  flame: c("flame", 60146),
  foldDown: c("fold-down", 60147),
  foldUp: c("fold-up", 60148),
  fold: c("fold", 60149),
  folderActive: c("folder-active", 60150),
  folderOpened: c("folder-opened", 60151),
  gear: c("gear", 60152),
  gift: c("gift", 60153),
  gistSecret: c("gist-secret", 60154),
  gist: c("gist", 60155),
  gitCommit: c("git-commit", 60156),
  gitCompare: c("git-compare", 60157),
  gitMerge: c("git-merge", 60158),
  githubAction: c("github-action", 60159),
  githubAlt: c("github-alt", 60160),
  globe: c("globe", 60161),
  grabber: c("grabber", 60162),
  graph: c("graph", 60163),
  gripper: c("gripper", 60164),
  heart: c("heart", 60165),
  home: c("home", 60166),
  horizontalRule: c("horizontal-rule", 60167),
  hubot: c("hubot", 60168),
  inbox: c("inbox", 60169),
  issueClosed: c("issue-closed", 60324),
  issueReopened: c("issue-reopened", 60171),
  issues: c("issues", 60172),
  italic: c("italic", 60173),
  jersey: c("jersey", 60174),
  json: c("json", 60175),
  bracket: c("bracket", 60175),
  kebabVertical: c("kebab-vertical", 60176),
  key: c("key", 60177),
  law: c("law", 60178),
  lightbulbAutofix: c("lightbulb-autofix", 60179),
  linkExternal: c("link-external", 60180),
  link: c("link", 60181),
  listOrdered: c("list-ordered", 60182),
  listUnordered: c("list-unordered", 60183),
  liveShare: c("live-share", 60184),
  loading: c("loading", 60185),
  location: c("location", 60186),
  mailRead: c("mail-read", 60187),
  mail: c("mail", 60188),
  markdown: c("markdown", 60189),
  megaphone: c("megaphone", 60190),
  mention: c("mention", 60191),
  milestone: c("milestone", 60192),
  mortarBoard: c("mortar-board", 60193),
  move: c("move", 60194),
  multipleWindows: c("multiple-windows", 60195),
  mute: c("mute", 60196),
  noNewline: c("no-newline", 60197),
  note: c("note", 60198),
  octoface: c("octoface", 60199),
  openPreview: c("open-preview", 60200),
  package_: c("package", 60201),
  paintcan: c("paintcan", 60202),
  pin: c("pin", 60203),
  play: c("play", 60204),
  run: c("run", 60204),
  plug: c("plug", 60205),
  preserveCase: c("preserve-case", 60206),
  preview: c("preview", 60207),
  project: c("project", 60208),
  pulse: c("pulse", 60209),
  question: c("question", 60210),
  quote: c("quote", 60211),
  radioTower: c("radio-tower", 60212),
  reactions: c("reactions", 60213),
  references: c("references", 60214),
  refresh: c("refresh", 60215),
  regex: c("regex", 60216),
  remoteExplorer: c("remote-explorer", 60217),
  remote: c("remote", 60218),
  remove: c("remove", 60219),
  replaceAll: c("replace-all", 60220),
  replace: c("replace", 60221),
  repoClone: c("repo-clone", 60222),
  repoForcePush: c("repo-force-push", 60223),
  repoPull: c("repo-pull", 60224),
  repoPush: c("repo-push", 60225),
  report: c("report", 60226),
  requestChanges: c("request-changes", 60227),
  rocket: c("rocket", 60228),
  rootFolderOpened: c("root-folder-opened", 60229),
  rootFolder: c("root-folder", 60230),
  rss: c("rss", 60231),
  ruby: c("ruby", 60232),
  saveAll: c("save-all", 60233),
  saveAs: c("save-as", 60234),
  save: c("save", 60235),
  screenFull: c("screen-full", 60236),
  screenNormal: c("screen-normal", 60237),
  searchStop: c("search-stop", 60238),
  server: c("server", 60240),
  settingsGear: c("settings-gear", 60241),
  settings: c("settings", 60242),
  shield: c("shield", 60243),
  smiley: c("smiley", 60244),
  sortPrecedence: c("sort-precedence", 60245),
  splitHorizontal: c("split-horizontal", 60246),
  splitVertical: c("split-vertical", 60247),
  squirrel: c("squirrel", 60248),
  starFull: c("star-full", 60249),
  starHalf: c("star-half", 60250),
  symbolClass: c("symbol-class", 60251),
  symbolColor: c("symbol-color", 60252),
  symbolCustomColor: c("symbol-customcolor", 60252),
  symbolConstant: c("symbol-constant", 60253),
  symbolEnumMember: c("symbol-enum-member", 60254),
  symbolField: c("symbol-field", 60255),
  symbolFile: c("symbol-file", 60256),
  symbolInterface: c("symbol-interface", 60257),
  symbolKeyword: c("symbol-keyword", 60258),
  symbolMisc: c("symbol-misc", 60259),
  symbolOperator: c("symbol-operator", 60260),
  symbolProperty: c("symbol-property", 60261),
  wrench: c("wrench", 60261),
  wrenchSubaction: c("wrench-subaction", 60261),
  symbolSnippet: c("symbol-snippet", 60262),
  tasklist: c("tasklist", 60263),
  telescope: c("telescope", 60264),
  textSize: c("text-size", 60265),
  threeBars: c("three-bars", 60266),
  thumbsdown: c("thumbsdown", 60267),
  thumbsup: c("thumbsup", 60268),
  tools: c("tools", 60269),
  triangleDown: c("triangle-down", 60270),
  triangleLeft: c("triangle-left", 60271),
  triangleRight: c("triangle-right", 60272),
  triangleUp: c("triangle-up", 60273),
  twitter: c("twitter", 60274),
  unfold: c("unfold", 60275),
  unlock: c("unlock", 60276),
  unmute: c("unmute", 60277),
  unverified: c("unverified", 60278),
  verified: c("verified", 60279),
  versions: c("versions", 60280),
  vmActive: c("vm-active", 60281),
  vmOutline: c("vm-outline", 60282),
  vmRunning: c("vm-running", 60283),
  watch: c("watch", 60284),
  whitespace: c("whitespace", 60285),
  wholeWord: c("whole-word", 60286),
  window: c("window", 60287),
  wordWrap: c("word-wrap", 60288),
  zoomIn: c("zoom-in", 60289),
  zoomOut: c("zoom-out", 60290),
  listFilter: c("list-filter", 60291),
  listFlat: c("list-flat", 60292),
  listSelection: c("list-selection", 60293),
  selection: c("selection", 60293),
  listTree: c("list-tree", 60294),
  debugBreakpointFunctionUnverified: c("debug-breakpoint-function-unverified", 60295),
  debugBreakpointFunction: c("debug-breakpoint-function", 60296),
  debugBreakpointFunctionDisabled: c("debug-breakpoint-function-disabled", 60296),
  debugStackframeActive: c("debug-stackframe-active", 60297),
  circleSmallFilled: c("circle-small-filled", 60298),
  debugStackframeDot: c("debug-stackframe-dot", 60298),
  debugStackframe: c("debug-stackframe", 60299),
  debugStackframeFocused: c("debug-stackframe-focused", 60299),
  debugBreakpointUnsupported: c("debug-breakpoint-unsupported", 60300),
  symbolString: c("symbol-string", 60301),
  debugReverseContinue: c("debug-reverse-continue", 60302),
  debugStepBack: c("debug-step-back", 60303),
  debugRestartFrame: c("debug-restart-frame", 60304),
  callIncoming: c("call-incoming", 60306),
  callOutgoing: c("call-outgoing", 60307),
  menu: c("menu", 60308),
  expandAll: c("expand-all", 60309),
  feedback: c("feedback", 60310),
  groupByRefType: c("group-by-ref-type", 60311),
  ungroupByRefType: c("ungroup-by-ref-type", 60312),
  account: c("account", 60313),
  bellDot: c("bell-dot", 60314),
  debugConsole: c("debug-console", 60315),
  library: c("library", 60316),
  output: c("output", 60317),
  runAll: c("run-all", 60318),
  syncIgnored: c("sync-ignored", 60319),
  pinned: c("pinned", 60320),
  githubInverted: c("github-inverted", 60321),
  debugAlt: c("debug-alt", 60305),
  serverProcess: c("server-process", 60322),
  serverEnvironment: c("server-environment", 60323),
  pass: c("pass", 60324),
  stopCircle: c("stop-circle", 60325),
  playCircle: c("play-circle", 60326),
  record: c("record", 60327),
  debugAltSmall: c("debug-alt-small", 60328),
  vmConnect: c("vm-connect", 60329),
  cloud: c("cloud", 60330),
  merge: c("merge", 60331),
  exportIcon: c("export", 60332),
  graphLeft: c("graph-left", 60333),
  magnet: c("magnet", 60334),
  notebook: c("notebook", 60335),
  redo: c("redo", 60336),
  checkAll: c("check-all", 60337),
  pinnedDirty: c("pinned-dirty", 60338),
  passFilled: c("pass-filled", 60339),
  circleLargeFilled: c("circle-large-filled", 60340),
  circleLarge: c("circle-large", 60341),
  circleLargeOutline: c("circle-large-outline", 60341),
  combine: c("combine", 60342),
  gather: c("gather", 60342),
  table: c("table", 60343),
  variableGroup: c("variable-group", 60344),
  typeHierarchy: c("type-hierarchy", 60345),
  typeHierarchySub: c("type-hierarchy-sub", 60346),
  typeHierarchySuper: c("type-hierarchy-super", 60347),
  gitPullRequestCreate: c("git-pull-request-create", 60348),
  runAbove: c("run-above", 60349),
  runBelow: c("run-below", 60350),
  notebookTemplate: c("notebook-template", 60351),
  debugRerun: c("debug-rerun", 60352),
  workspaceTrusted: c("workspace-trusted", 60353),
  workspaceUntrusted: c("workspace-untrusted", 60354),
  workspaceUnspecified: c("workspace-unspecified", 60355),
  terminalCmd: c("terminal-cmd", 60356),
  terminalDebian: c("terminal-debian", 60357),
  terminalLinux: c("terminal-linux", 60358),
  terminalPowershell: c("terminal-powershell", 60359),
  terminalTmux: c("terminal-tmux", 60360),
  terminalUbuntu: c("terminal-ubuntu", 60361),
  terminalBash: c("terminal-bash", 60362),
  arrowSwap: c("arrow-swap", 60363),
  copy: c("copy", 60364),
  personAdd: c("person-add", 60365),
  filterFilled: c("filter-filled", 60366),
  wand: c("wand", 60367),
  debugLineByLine: c("debug-line-by-line", 60368),
  inspect: c("inspect", 60369),
  layers: c("layers", 60370),
  layersDot: c("layers-dot", 60371),
  layersActive: c("layers-active", 60372),
  compass: c("compass", 60373),
  compassDot: c("compass-dot", 60374),
  compassActive: c("compass-active", 60375),
  azure: c("azure", 60376),
  issueDraft: c("issue-draft", 60377),
  gitPullRequestClosed: c("git-pull-request-closed", 60378),
  gitPullRequestDraft: c("git-pull-request-draft", 60379),
  debugAll: c("debug-all", 60380),
  debugCoverage: c("debug-coverage", 60381),
  runErrors: c("run-errors", 60382),
  folderLibrary: c("folder-library", 60383),
  debugContinueSmall: c("debug-continue-small", 60384),
  beakerStop: c("beaker-stop", 60385),
  graphLine: c("graph-line", 60386),
  graphScatter: c("graph-scatter", 60387),
  pieChart: c("pie-chart", 60388),
  bracketDot: c("bracket-dot", 60389),
  bracketError: c("bracket-error", 60390),
  lockSmall: c("lock-small", 60391),
  azureDevops: c("azure-devops", 60392),
  verifiedFilled: c("verified-filled", 60393),
  newLine: c("newline", 60394),
  layout: c("layout", 60395),
  layoutActivitybarLeft: c("layout-activitybar-left", 60396),
  layoutActivitybarRight: c("layout-activitybar-right", 60397),
  layoutPanelLeft: c("layout-panel-left", 60398),
  layoutPanelCenter: c("layout-panel-center", 60399),
  layoutPanelJustify: c("layout-panel-justify", 60400),
  layoutPanelRight: c("layout-panel-right", 60401),
  layoutPanel: c("layout-panel", 60402),
  layoutSidebarLeft: c("layout-sidebar-left", 60403),
  layoutSidebarRight: c("layout-sidebar-right", 60404),
  layoutStatusbar: c("layout-statusbar", 60405),
  layoutMenubar: c("layout-menubar", 60406),
  layoutCentered: c("layout-centered", 60407),
  layoutSidebarRightOff: c("layout-sidebar-right-off", 60416),
  layoutPanelOff: c("layout-panel-off", 60417),
  layoutSidebarLeftOff: c("layout-sidebar-left-off", 60418),
  target: c("target", 60408),
  indent: c("indent", 60409),
  recordSmall: c("record-small", 60410),
  errorSmall: c("error-small", 60411),
  arrowCircleDown: c("arrow-circle-down", 60412),
  arrowCircleLeft: c("arrow-circle-left", 60413),
  arrowCircleRight: c("arrow-circle-right", 60414),
  arrowCircleUp: c("arrow-circle-up", 60415),
  heartFilled: c("heart-filled", 60420),
  map: c("map", 60421),
  mapFilled: c("map-filled", 60422),
  circleSmall: c("circle-small", 60423),
  bellSlash: c("bell-slash", 60424),
  bellSlashDot: c("bell-slash-dot", 60425),
  commentUnresolved: c("comment-unresolved", 60426),
  gitPullRequestGoToChanges: c("git-pull-request-go-to-changes", 60427),
  gitPullRequestNewChanges: c("git-pull-request-new-changes", 60428),
  searchFuzzy: c("search-fuzzy", 60429),
  commentDraft: c("comment-draft", 60430),
  send: c("send", 60431),
  sparkle: c("sparkle", 60432),
  insert: c("insert", 60433),
  // derived icons, that could become separate icons
  dialogError: c("dialog-error", "error"),
  dialogWarning: c("dialog-warning", "warning"),
  dialogInfo: c("dialog-info", "info"),
  dialogClose: c("dialog-close", "close"),
  treeItemExpanded: c("tree-item-expanded", "chevron-down"),
  treeFilterOnTypeOn: c("tree-filter-on-type-on", "list-filter"),
  treeFilterOnTypeOff: c("tree-filter-on-type-off", "list-selection"),
  treeFilterClear: c("tree-filter-clear", "close"),
  treeItemLoading: c("tree-item-loading", "loading"),
  menuSelection: c("menu-selection", "check"),
  menuSubmenu: c("menu-submenu", "chevron-right"),
  menuBarMore: c("menubar-more", "more"),
  scrollbarButtonLeft: c("scrollbar-button-left", "triangle-left"),
  scrollbarButtonRight: c("scrollbar-button-right", "triangle-right"),
  scrollbarButtonUp: c("scrollbar-button-up", "triangle-up"),
  scrollbarButtonDown: c("scrollbar-button-down", "triangle-down"),
  toolBarMore: c("toolbar-more", "more"),
  quickInputBack: c("quick-input-back", "arrow-left")
};
var Or = globalThis && globalThis.__awaiter || function(e, t, r, n) {
  function i(s) {
    return s instanceof r ? s : new r(function(a) {
      a(s);
    });
  }
  return new (r || (r = Promise))(function(s, a) {
    function o(f) {
      try {
        u(n.next(f));
      } catch (h) {
        a(h);
      }
    }
    function l(f) {
      try {
        u(n.throw(f));
      } catch (h) {
        a(h);
      }
    }
    function u(f) {
      f.done ? s(f.value) : i(f.value).then(o, l);
    }
    u((n = n.apply(e, t || [])).next());
  });
};
class Mo {
  constructor() {
    this._tokenizationSupports = /* @__PURE__ */ new Map(), this._factories = /* @__PURE__ */ new Map(), this._onDidChange = new Me(), this.onDidChange = this._onDidChange.event, this._colorMap = null;
  }
  handleChange(t) {
    this._onDidChange.fire({
      changedLanguages: t,
      changedColorMap: !1
    });
  }
  register(t, r) {
    return this._tokenizationSupports.set(t, r), this.handleChange([t]), Gt(() => {
      this._tokenizationSupports.get(t) === r && (this._tokenizationSupports.delete(t), this.handleChange([t]));
    });
  }
  get(t) {
    return this._tokenizationSupports.get(t) || null;
  }
  registerFactory(t, r) {
    var n;
    (n = this._factories.get(t)) === null || n === void 0 || n.dispose();
    const i = new To(this, t, r);
    return this._factories.set(t, i), Gt(() => {
      const s = this._factories.get(t);
      !s || s !== i || (this._factories.delete(t), s.dispose());
    });
  }
  getOrCreate(t) {
    return Or(this, void 0, void 0, function* () {
      const r = this.get(t);
      if (r)
        return r;
      const n = this._factories.get(t);
      return !n || n.isResolved ? null : (yield n.resolve(), this.get(t));
    });
  }
  isResolved(t) {
    if (this.get(t))
      return !0;
    const n = this._factories.get(t);
    return !!(!n || n.isResolved);
  }
  setColorMap(t) {
    this._colorMap = t, this._onDidChange.fire({
      changedLanguages: Array.from(this._tokenizationSupports.keys()),
      changedColorMap: !0
    });
  }
  getColorMap() {
    return this._colorMap;
  }
  getDefaultBackground() {
    return this._colorMap && this._colorMap.length > 2 ? this._colorMap[
      2
      /* ColorId.DefaultBackground */
    ] : null;
  }
}
class To extends lr {
  get isResolved() {
    return this._isResolved;
  }
  constructor(t, r, n) {
    super(), this._registry = t, this._languageId = r, this._factory = n, this._isDisposed = !1, this._resolvePromise = null, this._isResolved = !1;
  }
  dispose() {
    this._isDisposed = !0, super.dispose();
  }
  resolve() {
    return Or(this, void 0, void 0, function* () {
      return this._resolvePromise || (this._resolvePromise = this._create()), this._resolvePromise;
    });
  }
  _create() {
    return Or(this, void 0, void 0, function* () {
      const t = yield this._factory.tokenizationSupport;
      this._isResolved = !0, t && !this._isDisposed && this._register(this._registry.register(this._languageId, t));
    });
  }
}
class Po {
  constructor(t, r, n) {
    this.offset = t, this.type = r, this.language = n, this._tokenBrand = void 0;
  }
  toString() {
    return "(" + this.offset + ", " + this.type + ")";
  }
}
var Pn;
(function(e) {
  const t = /* @__PURE__ */ new Map();
  t.set(0, O.symbolMethod), t.set(1, O.symbolFunction), t.set(2, O.symbolConstructor), t.set(3, O.symbolField), t.set(4, O.symbolVariable), t.set(5, O.symbolClass), t.set(6, O.symbolStruct), t.set(7, O.symbolInterface), t.set(8, O.symbolModule), t.set(9, O.symbolProperty), t.set(10, O.symbolEvent), t.set(11, O.symbolOperator), t.set(12, O.symbolUnit), t.set(13, O.symbolValue), t.set(15, O.symbolEnum), t.set(14, O.symbolConstant), t.set(15, O.symbolEnum), t.set(16, O.symbolEnumMember), t.set(17, O.symbolKeyword), t.set(27, O.symbolSnippet), t.set(18, O.symbolText), t.set(19, O.symbolColor), t.set(20, O.symbolFile), t.set(21, O.symbolReference), t.set(22, O.symbolCustomColor), t.set(23, O.symbolFolder), t.set(24, O.symbolTypeParameter), t.set(25, O.account), t.set(26, O.issues);
  function r(s) {
    let a = t.get(s);
    return a || (console.info("No codicon found for CompletionItemKind " + s), a = O.symbolProperty), a;
  }
  e.toIcon = r;
  const n = /* @__PURE__ */ new Map();
  n.set(
    "method",
    0
    /* CompletionItemKind.Method */
  ), n.set(
    "function",
    1
    /* CompletionItemKind.Function */
  ), n.set(
    "constructor",
    2
    /* CompletionItemKind.Constructor */
  ), n.set(
    "field",
    3
    /* CompletionItemKind.Field */
  ), n.set(
    "variable",
    4
    /* CompletionItemKind.Variable */
  ), n.set(
    "class",
    5
    /* CompletionItemKind.Class */
  ), n.set(
    "struct",
    6
    /* CompletionItemKind.Struct */
  ), n.set(
    "interface",
    7
    /* CompletionItemKind.Interface */
  ), n.set(
    "module",
    8
    /* CompletionItemKind.Module */
  ), n.set(
    "property",
    9
    /* CompletionItemKind.Property */
  ), n.set(
    "event",
    10
    /* CompletionItemKind.Event */
  ), n.set(
    "operator",
    11
    /* CompletionItemKind.Operator */
  ), n.set(
    "unit",
    12
    /* CompletionItemKind.Unit */
  ), n.set(
    "value",
    13
    /* CompletionItemKind.Value */
  ), n.set(
    "constant",
    14
    /* CompletionItemKind.Constant */
  ), n.set(
    "enum",
    15
    /* CompletionItemKind.Enum */
  ), n.set(
    "enum-member",
    16
    /* CompletionItemKind.EnumMember */
  ), n.set(
    "enumMember",
    16
    /* CompletionItemKind.EnumMember */
  ), n.set(
    "keyword",
    17
    /* CompletionItemKind.Keyword */
  ), n.set(
    "snippet",
    27
    /* CompletionItemKind.Snippet */
  ), n.set(
    "text",
    18
    /* CompletionItemKind.Text */
  ), n.set(
    "color",
    19
    /* CompletionItemKind.Color */
  ), n.set(
    "file",
    20
    /* CompletionItemKind.File */
  ), n.set(
    "reference",
    21
    /* CompletionItemKind.Reference */
  ), n.set(
    "customcolor",
    22
    /* CompletionItemKind.Customcolor */
  ), n.set(
    "folder",
    23
    /* CompletionItemKind.Folder */
  ), n.set(
    "type-parameter",
    24
    /* CompletionItemKind.TypeParameter */
  ), n.set(
    "typeParameter",
    24
    /* CompletionItemKind.TypeParameter */
  ), n.set(
    "account",
    25
    /* CompletionItemKind.User */
  ), n.set(
    "issue",
    26
    /* CompletionItemKind.Issue */
  );
  function i(s, a) {
    let o = n.get(s);
    return typeof o > "u" && !a && (o = 9), o;
  }
  e.fromString = i;
})(Pn || (Pn = {}));
var Rn;
(function(e) {
  e[e.Automatic = 0] = "Automatic", e[e.Explicit = 1] = "Explicit";
})(Rn || (Rn = {}));
var Fn;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.TriggerCharacter = 2] = "TriggerCharacter", e[e.ContentChange = 3] = "ContentChange";
})(Fn || (Fn = {}));
var In;
(function(e) {
  e[e.Text = 0] = "Text", e[e.Read = 1] = "Read", e[e.Write = 2] = "Write";
})(In || (In = {}));
var Vn;
(function(e) {
  const t = /* @__PURE__ */ new Map();
  t.set(0, O.symbolFile), t.set(1, O.symbolModule), t.set(2, O.symbolNamespace), t.set(3, O.symbolPackage), t.set(4, O.symbolClass), t.set(5, O.symbolMethod), t.set(6, O.symbolProperty), t.set(7, O.symbolField), t.set(8, O.symbolConstructor), t.set(9, O.symbolEnum), t.set(10, O.symbolInterface), t.set(11, O.symbolFunction), t.set(12, O.symbolVariable), t.set(13, O.symbolConstant), t.set(14, O.symbolString), t.set(15, O.symbolNumber), t.set(16, O.symbolBoolean), t.set(17, O.symbolArray), t.set(18, O.symbolObject), t.set(19, O.symbolKey), t.set(20, O.symbolNull), t.set(21, O.symbolEnumMember), t.set(22, O.symbolStruct), t.set(23, O.symbolEvent), t.set(24, O.symbolOperator), t.set(25, O.symbolTypeParameter);
  function r(n) {
    let i = t.get(n);
    return i || (console.info("No codicon found for SymbolKind " + n), i = O.symbolProperty), i;
  }
  e.toIcon = r;
})(Vn || (Vn = {}));
var Dn;
(function(e) {
  function t(r) {
    return !r || typeof r != "object" ? !1 : typeof r.id == "string" && typeof r.title == "string";
  }
  e.is = t;
})(Dn || (Dn = {}));
var On;
(function(e) {
  e[e.Collapsed = 0] = "Collapsed", e[e.Expanded = 1] = "Expanded";
})(On || (On = {}));
var jn;
(function(e) {
  e[e.Unresolved = 0] = "Unresolved", e[e.Resolved = 1] = "Resolved";
})(jn || (jn = {}));
var Un;
(function(e) {
  e[e.Editing = 0] = "Editing", e[e.Preview = 1] = "Preview";
})(Un || (Un = {}));
var Bn;
(function(e) {
  e[e.Published = 0] = "Published", e[e.Draft = 1] = "Draft";
})(Bn || (Bn = {}));
var $n;
(function(e) {
  e[e.Type = 1] = "Type", e[e.Parameter = 2] = "Parameter";
})($n || ($n = {}));
new Mo();
var qn;
(function(e) {
  e[e.None = 0] = "None", e[e.Option = 1] = "Option", e[e.Default = 2] = "Default", e[e.Preferred = 3] = "Preferred";
})(qn || (qn = {}));
var Wn;
(function(e) {
  e[e.Unknown = 0] = "Unknown", e[e.Disabled = 1] = "Disabled", e[e.Enabled = 2] = "Enabled";
})(Wn || (Wn = {}));
var Hn;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.Auto = 2] = "Auto";
})(Hn || (Hn = {}));
var zn;
(function(e) {
  e[e.None = 0] = "None", e[e.KeepWhitespace = 1] = "KeepWhitespace", e[e.InsertAsSnippet = 4] = "InsertAsSnippet";
})(zn || (zn = {}));
var Gn;
(function(e) {
  e[e.Method = 0] = "Method", e[e.Function = 1] = "Function", e[e.Constructor = 2] = "Constructor", e[e.Field = 3] = "Field", e[e.Variable = 4] = "Variable", e[e.Class = 5] = "Class", e[e.Struct = 6] = "Struct", e[e.Interface = 7] = "Interface", e[e.Module = 8] = "Module", e[e.Property = 9] = "Property", e[e.Event = 10] = "Event", e[e.Operator = 11] = "Operator", e[e.Unit = 12] = "Unit", e[e.Value = 13] = "Value", e[e.Constant = 14] = "Constant", e[e.Enum = 15] = "Enum", e[e.EnumMember = 16] = "EnumMember", e[e.Keyword = 17] = "Keyword", e[e.Text = 18] = "Text", e[e.Color = 19] = "Color", e[e.File = 20] = "File", e[e.Reference = 21] = "Reference", e[e.Customcolor = 22] = "Customcolor", e[e.Folder = 23] = "Folder", e[e.TypeParameter = 24] = "TypeParameter", e[e.User = 25] = "User", e[e.Issue = 26] = "Issue", e[e.Snippet = 27] = "Snippet";
})(Gn || (Gn = {}));
var Jn;
(function(e) {
  e[e.Deprecated = 1] = "Deprecated";
})(Jn || (Jn = {}));
var Xn;
(function(e) {
  e[e.Invoke = 0] = "Invoke", e[e.TriggerCharacter = 1] = "TriggerCharacter", e[e.TriggerForIncompleteCompletions = 2] = "TriggerForIncompleteCompletions";
})(Xn || (Xn = {}));
var Zn;
(function(e) {
  e[e.EXACT = 0] = "EXACT", e[e.ABOVE = 1] = "ABOVE", e[e.BELOW = 2] = "BELOW";
})(Zn || (Zn = {}));
var Qn;
(function(e) {
  e[e.NotSet = 0] = "NotSet", e[e.ContentFlush = 1] = "ContentFlush", e[e.RecoverFromMarkers = 2] = "RecoverFromMarkers", e[e.Explicit = 3] = "Explicit", e[e.Paste = 4] = "Paste", e[e.Undo = 5] = "Undo", e[e.Redo = 6] = "Redo";
})(Qn || (Qn = {}));
var Yn;
(function(e) {
  e[e.LF = 1] = "LF", e[e.CRLF = 2] = "CRLF";
})(Yn || (Yn = {}));
var Kn;
(function(e) {
  e[e.Text = 0] = "Text", e[e.Read = 1] = "Read", e[e.Write = 2] = "Write";
})(Kn || (Kn = {}));
var ei;
(function(e) {
  e[e.None = 0] = "None", e[e.Keep = 1] = "Keep", e[e.Brackets = 2] = "Brackets", e[e.Advanced = 3] = "Advanced", e[e.Full = 4] = "Full";
})(ei || (ei = {}));
var ti;
(function(e) {
  e[e.acceptSuggestionOnCommitCharacter = 0] = "acceptSuggestionOnCommitCharacter", e[e.acceptSuggestionOnEnter = 1] = "acceptSuggestionOnEnter", e[e.accessibilitySupport = 2] = "accessibilitySupport", e[e.accessibilityPageSize = 3] = "accessibilityPageSize", e[e.ariaLabel = 4] = "ariaLabel", e[e.autoClosingBrackets = 5] = "autoClosingBrackets", e[e.screenReaderAnnounceInlineSuggestion = 6] = "screenReaderAnnounceInlineSuggestion", e[e.autoClosingDelete = 7] = "autoClosingDelete", e[e.autoClosingOvertype = 8] = "autoClosingOvertype", e[e.autoClosingQuotes = 9] = "autoClosingQuotes", e[e.autoIndent = 10] = "autoIndent", e[e.automaticLayout = 11] = "automaticLayout", e[e.autoSurround = 12] = "autoSurround", e[e.bracketPairColorization = 13] = "bracketPairColorization", e[e.guides = 14] = "guides", e[e.codeLens = 15] = "codeLens", e[e.codeLensFontFamily = 16] = "codeLensFontFamily", e[e.codeLensFontSize = 17] = "codeLensFontSize", e[e.colorDecorators = 18] = "colorDecorators", e[e.colorDecoratorsLimit = 19] = "colorDecoratorsLimit", e[e.columnSelection = 20] = "columnSelection", e[e.comments = 21] = "comments", e[e.contextmenu = 22] = "contextmenu", e[e.copyWithSyntaxHighlighting = 23] = "copyWithSyntaxHighlighting", e[e.cursorBlinking = 24] = "cursorBlinking", e[e.cursorSmoothCaretAnimation = 25] = "cursorSmoothCaretAnimation", e[e.cursorStyle = 26] = "cursorStyle", e[e.cursorSurroundingLines = 27] = "cursorSurroundingLines", e[e.cursorSurroundingLinesStyle = 28] = "cursorSurroundingLinesStyle", e[e.cursorWidth = 29] = "cursorWidth", e[e.disableLayerHinting = 30] = "disableLayerHinting", e[e.disableMonospaceOptimizations = 31] = "disableMonospaceOptimizations", e[e.domReadOnly = 32] = "domReadOnly", e[e.dragAndDrop = 33] = "dragAndDrop", e[e.dropIntoEditor = 34] = "dropIntoEditor", e[e.emptySelectionClipboard = 35] = "emptySelectionClipboard", e[e.experimentalWhitespaceRendering = 36] = "experimentalWhitespaceRendering", e[e.extraEditorClassName = 37] = "extraEditorClassName", e[e.fastScrollSensitivity = 38] = "fastScrollSensitivity", e[e.find = 39] = "find", e[e.fixedOverflowWidgets = 40] = "fixedOverflowWidgets", e[e.folding = 41] = "folding", e[e.foldingStrategy = 42] = "foldingStrategy", e[e.foldingHighlight = 43] = "foldingHighlight", e[e.foldingImportsByDefault = 44] = "foldingImportsByDefault", e[e.foldingMaximumRegions = 45] = "foldingMaximumRegions", e[e.unfoldOnClickAfterEndOfLine = 46] = "unfoldOnClickAfterEndOfLine", e[e.fontFamily = 47] = "fontFamily", e[e.fontInfo = 48] = "fontInfo", e[e.fontLigatures = 49] = "fontLigatures", e[e.fontSize = 50] = "fontSize", e[e.fontWeight = 51] = "fontWeight", e[e.fontVariations = 52] = "fontVariations", e[e.formatOnPaste = 53] = "formatOnPaste", e[e.formatOnType = 54] = "formatOnType", e[e.glyphMargin = 55] = "glyphMargin", e[e.gotoLocation = 56] = "gotoLocation", e[e.hideCursorInOverviewRuler = 57] = "hideCursorInOverviewRuler", e[e.hover = 58] = "hover", e[e.inDiffEditor = 59] = "inDiffEditor", e[e.inlineSuggest = 60] = "inlineSuggest", e[e.letterSpacing = 61] = "letterSpacing", e[e.lightbulb = 62] = "lightbulb", e[e.lineDecorationsWidth = 63] = "lineDecorationsWidth", e[e.lineHeight = 64] = "lineHeight", e[e.lineNumbers = 65] = "lineNumbers", e[e.lineNumbersMinChars = 66] = "lineNumbersMinChars", e[e.linkedEditing = 67] = "linkedEditing", e[e.links = 68] = "links", e[e.matchBrackets = 69] = "matchBrackets", e[e.minimap = 70] = "minimap", e[e.mouseStyle = 71] = "mouseStyle", e[e.mouseWheelScrollSensitivity = 72] = "mouseWheelScrollSensitivity", e[e.mouseWheelZoom = 73] = "mouseWheelZoom", e[e.multiCursorMergeOverlapping = 74] = "multiCursorMergeOverlapping", e[e.multiCursorModifier = 75] = "multiCursorModifier", e[e.multiCursorPaste = 76] = "multiCursorPaste", e[e.multiCursorLimit = 77] = "multiCursorLimit", e[e.occurrencesHighlight = 78] = "occurrencesHighlight", e[e.overviewRulerBorder = 79] = "overviewRulerBorder", e[e.overviewRulerLanes = 80] = "overviewRulerLanes", e[e.padding = 81] = "padding", e[e.parameterHints = 82] = "parameterHints", e[e.peekWidgetDefaultFocus = 83] = "peekWidgetDefaultFocus", e[e.definitionLinkOpensInPeek = 84] = "definitionLinkOpensInPeek", e[e.quickSuggestions = 85] = "quickSuggestions", e[e.quickSuggestionsDelay = 86] = "quickSuggestionsDelay", e[e.readOnly = 87] = "readOnly", e[e.renameOnType = 88] = "renameOnType", e[e.renderControlCharacters = 89] = "renderControlCharacters", e[e.renderFinalNewline = 90] = "renderFinalNewline", e[e.renderLineHighlight = 91] = "renderLineHighlight", e[e.renderLineHighlightOnlyWhenFocus = 92] = "renderLineHighlightOnlyWhenFocus", e[e.renderValidationDecorations = 93] = "renderValidationDecorations", e[e.renderWhitespace = 94] = "renderWhitespace", e[e.revealHorizontalRightPadding = 95] = "revealHorizontalRightPadding", e[e.roundedSelection = 96] = "roundedSelection", e[e.rulers = 97] = "rulers", e[e.scrollbar = 98] = "scrollbar", e[e.scrollBeyondLastColumn = 99] = "scrollBeyondLastColumn", e[e.scrollBeyondLastLine = 100] = "scrollBeyondLastLine", e[e.scrollPredominantAxis = 101] = "scrollPredominantAxis", e[e.selectionClipboard = 102] = "selectionClipboard", e[e.selectionHighlight = 103] = "selectionHighlight", e[e.selectOnLineNumbers = 104] = "selectOnLineNumbers", e[e.showFoldingControls = 105] = "showFoldingControls", e[e.showUnused = 106] = "showUnused", e[e.snippetSuggestions = 107] = "snippetSuggestions", e[e.smartSelect = 108] = "smartSelect", e[e.smoothScrolling = 109] = "smoothScrolling", e[e.stickyScroll = 110] = "stickyScroll", e[e.stickyTabStops = 111] = "stickyTabStops", e[e.stopRenderingLineAfter = 112] = "stopRenderingLineAfter", e[e.suggest = 113] = "suggest", e[e.suggestFontSize = 114] = "suggestFontSize", e[e.suggestLineHeight = 115] = "suggestLineHeight", e[e.suggestOnTriggerCharacters = 116] = "suggestOnTriggerCharacters", e[e.suggestSelection = 117] = "suggestSelection", e[e.tabCompletion = 118] = "tabCompletion", e[e.tabIndex = 119] = "tabIndex", e[e.unicodeHighlighting = 120] = "unicodeHighlighting", e[e.unusualLineTerminators = 121] = "unusualLineTerminators", e[e.useShadowDOM = 122] = "useShadowDOM", e[e.useTabStops = 123] = "useTabStops", e[e.wordBreak = 124] = "wordBreak", e[e.wordSeparators = 125] = "wordSeparators", e[e.wordWrap = 126] = "wordWrap", e[e.wordWrapBreakAfterCharacters = 127] = "wordWrapBreakAfterCharacters", e[e.wordWrapBreakBeforeCharacters = 128] = "wordWrapBreakBeforeCharacters", e[e.wordWrapColumn = 129] = "wordWrapColumn", e[e.wordWrapOverride1 = 130] = "wordWrapOverride1", e[e.wordWrapOverride2 = 131] = "wordWrapOverride2", e[e.wrappingIndent = 132] = "wrappingIndent", e[e.wrappingStrategy = 133] = "wrappingStrategy", e[e.showDeprecated = 134] = "showDeprecated", e[e.inlayHints = 135] = "inlayHints", e[e.editorClassName = 136] = "editorClassName", e[e.pixelRatio = 137] = "pixelRatio", e[e.tabFocusMode = 138] = "tabFocusMode", e[e.layoutInfo = 139] = "layoutInfo", e[e.wrappingInfo = 140] = "wrappingInfo";
})(ti || (ti = {}));
var ri;
(function(e) {
  e[e.TextDefined = 0] = "TextDefined", e[e.LF = 1] = "LF", e[e.CRLF = 2] = "CRLF";
})(ri || (ri = {}));
var ni;
(function(e) {
  e[e.LF = 0] = "LF", e[e.CRLF = 1] = "CRLF";
})(ni || (ni = {}));
var ii;
(function(e) {
  e[e.None = 0] = "None", e[e.Indent = 1] = "Indent", e[e.IndentOutdent = 2] = "IndentOutdent", e[e.Outdent = 3] = "Outdent";
})(ii || (ii = {}));
var si;
(function(e) {
  e[e.Both = 0] = "Both", e[e.Right = 1] = "Right", e[e.Left = 2] = "Left", e[e.None = 3] = "None";
})(si || (si = {}));
var ai;
(function(e) {
  e[e.Type = 1] = "Type", e[e.Parameter = 2] = "Parameter";
})(ai || (ai = {}));
var oi;
(function(e) {
  e[e.Automatic = 0] = "Automatic", e[e.Explicit = 1] = "Explicit";
})(oi || (oi = {}));
var jr;
(function(e) {
  e[e.DependsOnKbLayout = -1] = "DependsOnKbLayout", e[e.Unknown = 0] = "Unknown", e[e.Backspace = 1] = "Backspace", e[e.Tab = 2] = "Tab", e[e.Enter = 3] = "Enter", e[e.Shift = 4] = "Shift", e[e.Ctrl = 5] = "Ctrl", e[e.Alt = 6] = "Alt", e[e.PauseBreak = 7] = "PauseBreak", e[e.CapsLock = 8] = "CapsLock", e[e.Escape = 9] = "Escape", e[e.Space = 10] = "Space", e[e.PageUp = 11] = "PageUp", e[e.PageDown = 12] = "PageDown", e[e.End = 13] = "End", e[e.Home = 14] = "Home", e[e.LeftArrow = 15] = "LeftArrow", e[e.UpArrow = 16] = "UpArrow", e[e.RightArrow = 17] = "RightArrow", e[e.DownArrow = 18] = "DownArrow", e[e.Insert = 19] = "Insert", e[e.Delete = 20] = "Delete", e[e.Digit0 = 21] = "Digit0", e[e.Digit1 = 22] = "Digit1", e[e.Digit2 = 23] = "Digit2", e[e.Digit3 = 24] = "Digit3", e[e.Digit4 = 25] = "Digit4", e[e.Digit5 = 26] = "Digit5", e[e.Digit6 = 27] = "Digit6", e[e.Digit7 = 28] = "Digit7", e[e.Digit8 = 29] = "Digit8", e[e.Digit9 = 30] = "Digit9", e[e.KeyA = 31] = "KeyA", e[e.KeyB = 32] = "KeyB", e[e.KeyC = 33] = "KeyC", e[e.KeyD = 34] = "KeyD", e[e.KeyE = 35] = "KeyE", e[e.KeyF = 36] = "KeyF", e[e.KeyG = 37] = "KeyG", e[e.KeyH = 38] = "KeyH", e[e.KeyI = 39] = "KeyI", e[e.KeyJ = 40] = "KeyJ", e[e.KeyK = 41] = "KeyK", e[e.KeyL = 42] = "KeyL", e[e.KeyM = 43] = "KeyM", e[e.KeyN = 44] = "KeyN", e[e.KeyO = 45] = "KeyO", e[e.KeyP = 46] = "KeyP", e[e.KeyQ = 47] = "KeyQ", e[e.KeyR = 48] = "KeyR", e[e.KeyS = 49] = "KeyS", e[e.KeyT = 50] = "KeyT", e[e.KeyU = 51] = "KeyU", e[e.KeyV = 52] = "KeyV", e[e.KeyW = 53] = "KeyW", e[e.KeyX = 54] = "KeyX", e[e.KeyY = 55] = "KeyY", e[e.KeyZ = 56] = "KeyZ", e[e.Meta = 57] = "Meta", e[e.ContextMenu = 58] = "ContextMenu", e[e.F1 = 59] = "F1", e[e.F2 = 60] = "F2", e[e.F3 = 61] = "F3", e[e.F4 = 62] = "F4", e[e.F5 = 63] = "F5", e[e.F6 = 64] = "F6", e[e.F7 = 65] = "F7", e[e.F8 = 66] = "F8", e[e.F9 = 67] = "F9", e[e.F10 = 68] = "F10", e[e.F11 = 69] = "F11", e[e.F12 = 70] = "F12", e[e.F13 = 71] = "F13", e[e.F14 = 72] = "F14", e[e.F15 = 73] = "F15", e[e.F16 = 74] = "F16", e[e.F17 = 75] = "F17", e[e.F18 = 76] = "F18", e[e.F19 = 77] = "F19", e[e.NumLock = 78] = "NumLock", e[e.ScrollLock = 79] = "ScrollLock", e[e.Semicolon = 80] = "Semicolon", e[e.Equal = 81] = "Equal", e[e.Comma = 82] = "Comma", e[e.Minus = 83] = "Minus", e[e.Period = 84] = "Period", e[e.Slash = 85] = "Slash", e[e.Backquote = 86] = "Backquote", e[e.BracketLeft = 87] = "BracketLeft", e[e.Backslash = 88] = "Backslash", e[e.BracketRight = 89] = "BracketRight", e[e.Quote = 90] = "Quote", e[e.OEM_8 = 91] = "OEM_8", e[e.IntlBackslash = 92] = "IntlBackslash", e[e.Numpad0 = 93] = "Numpad0", e[e.Numpad1 = 94] = "Numpad1", e[e.Numpad2 = 95] = "Numpad2", e[e.Numpad3 = 96] = "Numpad3", e[e.Numpad4 = 97] = "Numpad4", e[e.Numpad5 = 98] = "Numpad5", e[e.Numpad6 = 99] = "Numpad6", e[e.Numpad7 = 100] = "Numpad7", e[e.Numpad8 = 101] = "Numpad8", e[e.Numpad9 = 102] = "Numpad9", e[e.NumpadMultiply = 103] = "NumpadMultiply", e[e.NumpadAdd = 104] = "NumpadAdd", e[e.NUMPAD_SEPARATOR = 105] = "NUMPAD_SEPARATOR", e[e.NumpadSubtract = 106] = "NumpadSubtract", e[e.NumpadDecimal = 107] = "NumpadDecimal", e[e.NumpadDivide = 108] = "NumpadDivide", e[e.KEY_IN_COMPOSITION = 109] = "KEY_IN_COMPOSITION", e[e.ABNT_C1 = 110] = "ABNT_C1", e[e.ABNT_C2 = 111] = "ABNT_C2", e[e.AudioVolumeMute = 112] = "AudioVolumeMute", e[e.AudioVolumeUp = 113] = "AudioVolumeUp", e[e.AudioVolumeDown = 114] = "AudioVolumeDown", e[e.BrowserSearch = 115] = "BrowserSearch", e[e.BrowserHome = 116] = "BrowserHome", e[e.BrowserBack = 117] = "BrowserBack", e[e.BrowserForward = 118] = "BrowserForward", e[e.MediaTrackNext = 119] = "MediaTrackNext", e[e.MediaTrackPrevious = 120] = "MediaTrackPrevious", e[e.MediaStop = 121] = "MediaStop", e[e.MediaPlayPause = 122] = "MediaPlayPause", e[e.LaunchMediaPlayer = 123] = "LaunchMediaPlayer", e[e.LaunchMail = 124] = "LaunchMail", e[e.LaunchApp2 = 125] = "LaunchApp2", e[e.Clear = 126] = "Clear", e[e.MAX_VALUE = 127] = "MAX_VALUE";
})(jr || (jr = {}));
var Ur;
(function(e) {
  e[e.Hint = 1] = "Hint", e[e.Info = 2] = "Info", e[e.Warning = 4] = "Warning", e[e.Error = 8] = "Error";
})(Ur || (Ur = {}));
var Br;
(function(e) {
  e[e.Unnecessary = 1] = "Unnecessary", e[e.Deprecated = 2] = "Deprecated";
})(Br || (Br = {}));
var li;
(function(e) {
  e[e.Inline = 1] = "Inline", e[e.Gutter = 2] = "Gutter";
})(li || (li = {}));
var ui;
(function(e) {
  e[e.UNKNOWN = 0] = "UNKNOWN", e[e.TEXTAREA = 1] = "TEXTAREA", e[e.GUTTER_GLYPH_MARGIN = 2] = "GUTTER_GLYPH_MARGIN", e[e.GUTTER_LINE_NUMBERS = 3] = "GUTTER_LINE_NUMBERS", e[e.GUTTER_LINE_DECORATIONS = 4] = "GUTTER_LINE_DECORATIONS", e[e.GUTTER_VIEW_ZONE = 5] = "GUTTER_VIEW_ZONE", e[e.CONTENT_TEXT = 6] = "CONTENT_TEXT", e[e.CONTENT_EMPTY = 7] = "CONTENT_EMPTY", e[e.CONTENT_VIEW_ZONE = 8] = "CONTENT_VIEW_ZONE", e[e.CONTENT_WIDGET = 9] = "CONTENT_WIDGET", e[e.OVERVIEW_RULER = 10] = "OVERVIEW_RULER", e[e.SCROLLBAR = 11] = "SCROLLBAR", e[e.OVERLAY_WIDGET = 12] = "OVERLAY_WIDGET", e[e.OUTSIDE_EDITOR = 13] = "OUTSIDE_EDITOR";
})(ui || (ui = {}));
var ci;
(function(e) {
  e[e.TOP_RIGHT_CORNER = 0] = "TOP_RIGHT_CORNER", e[e.BOTTOM_RIGHT_CORNER = 1] = "BOTTOM_RIGHT_CORNER", e[e.TOP_CENTER = 2] = "TOP_CENTER";
})(ci || (ci = {}));
var fi;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Center = 2] = "Center", e[e.Right = 4] = "Right", e[e.Full = 7] = "Full";
})(fi || (fi = {}));
var hi;
(function(e) {
  e[e.Left = 0] = "Left", e[e.Right = 1] = "Right", e[e.None = 2] = "None", e[e.LeftOfInjectedText = 3] = "LeftOfInjectedText", e[e.RightOfInjectedText = 4] = "RightOfInjectedText";
})(hi || (hi = {}));
var di;
(function(e) {
  e[e.Off = 0] = "Off", e[e.On = 1] = "On", e[e.Relative = 2] = "Relative", e[e.Interval = 3] = "Interval", e[e.Custom = 4] = "Custom";
})(di || (di = {}));
var gi;
(function(e) {
  e[e.None = 0] = "None", e[e.Text = 1] = "Text", e[e.Blocks = 2] = "Blocks";
})(gi || (gi = {}));
var mi;
(function(e) {
  e[e.Smooth = 0] = "Smooth", e[e.Immediate = 1] = "Immediate";
})(mi || (mi = {}));
var pi;
(function(e) {
  e[e.Auto = 1] = "Auto", e[e.Hidden = 2] = "Hidden", e[e.Visible = 3] = "Visible";
})(pi || (pi = {}));
var $r;
(function(e) {
  e[e.LTR = 0] = "LTR", e[e.RTL = 1] = "RTL";
})($r || ($r = {}));
var vi;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.TriggerCharacter = 2] = "TriggerCharacter", e[e.ContentChange = 3] = "ContentChange";
})(vi || (vi = {}));
var bi;
(function(e) {
  e[e.File = 0] = "File", e[e.Module = 1] = "Module", e[e.Namespace = 2] = "Namespace", e[e.Package = 3] = "Package", e[e.Class = 4] = "Class", e[e.Method = 5] = "Method", e[e.Property = 6] = "Property", e[e.Field = 7] = "Field", e[e.Constructor = 8] = "Constructor", e[e.Enum = 9] = "Enum", e[e.Interface = 10] = "Interface", e[e.Function = 11] = "Function", e[e.Variable = 12] = "Variable", e[e.Constant = 13] = "Constant", e[e.String = 14] = "String", e[e.Number = 15] = "Number", e[e.Boolean = 16] = "Boolean", e[e.Array = 17] = "Array", e[e.Object = 18] = "Object", e[e.Key = 19] = "Key", e[e.Null = 20] = "Null", e[e.EnumMember = 21] = "EnumMember", e[e.Struct = 22] = "Struct", e[e.Event = 23] = "Event", e[e.Operator = 24] = "Operator", e[e.TypeParameter = 25] = "TypeParameter";
})(bi || (bi = {}));
var yi;
(function(e) {
  e[e.Deprecated = 1] = "Deprecated";
})(yi || (yi = {}));
var xi;
(function(e) {
  e[e.Hidden = 0] = "Hidden", e[e.Blink = 1] = "Blink", e[e.Smooth = 2] = "Smooth", e[e.Phase = 3] = "Phase", e[e.Expand = 4] = "Expand", e[e.Solid = 5] = "Solid";
})(xi || (xi = {}));
var wi;
(function(e) {
  e[e.Line = 1] = "Line", e[e.Block = 2] = "Block", e[e.Underline = 3] = "Underline", e[e.LineThin = 4] = "LineThin", e[e.BlockOutline = 5] = "BlockOutline", e[e.UnderlineThin = 6] = "UnderlineThin";
})(wi || (wi = {}));
var _i;
(function(e) {
  e[e.AlwaysGrowsWhenTypingAtEdges = 0] = "AlwaysGrowsWhenTypingAtEdges", e[e.NeverGrowsWhenTypingAtEdges = 1] = "NeverGrowsWhenTypingAtEdges", e[e.GrowsOnlyWhenTypingBefore = 2] = "GrowsOnlyWhenTypingBefore", e[e.GrowsOnlyWhenTypingAfter = 3] = "GrowsOnlyWhenTypingAfter";
})(_i || (_i = {}));
var Si;
(function(e) {
  e[e.None = 0] = "None", e[e.Same = 1] = "Same", e[e.Indent = 2] = "Indent", e[e.DeepIndent = 3] = "DeepIndent";
})(Si || (Si = {}));
class Pt {
  static chord(t, r) {
    return Eo(t, r);
  }
}
Pt.CtrlCmd = 2048;
Pt.Shift = 1024;
Pt.Alt = 512;
Pt.WinCtrl = 256;
function Ro() {
  return {
    editor: void 0,
    languages: void 0,
    CancellationTokenSource: Ma,
    Emitter: Me,
    KeyCode: jr,
    KeyMod: Pt,
    Position: Re,
    Range: fe,
    Selection: me,
    SelectionDirection: $r,
    MarkerSeverity: Ur,
    MarkerTag: Br,
    Uri: on,
    Token: Po
  };
}
var Ai;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Center = 2] = "Center", e[e.Right = 4] = "Right", e[e.Full = 7] = "Full";
})(Ai || (Ai = {}));
var Ni;
(function(e) {
  e[e.Inline = 1] = "Inline", e[e.Gutter = 2] = "Gutter";
})(Ni || (Ni = {}));
var Li;
(function(e) {
  e[e.Both = 0] = "Both", e[e.Right = 1] = "Right", e[e.Left = 2] = "Left", e[e.None = 3] = "None";
})(Li || (Li = {}));
function Fo(e, t, r, n, i) {
  if (n === 0)
    return !0;
  const s = t.charCodeAt(n - 1);
  if (e.get(s) !== 0 || s === 13 || s === 10)
    return !0;
  if (i > 0) {
    const a = t.charCodeAt(n);
    if (e.get(a) !== 0)
      return !0;
  }
  return !1;
}
function Io(e, t, r, n, i) {
  if (n + i === r)
    return !0;
  const s = t.charCodeAt(n + i);
  if (e.get(s) !== 0 || s === 13 || s === 10)
    return !0;
  if (i > 0) {
    const a = t.charCodeAt(n + i - 1);
    if (e.get(a) !== 0)
      return !0;
  }
  return !1;
}
function Vo(e, t, r, n, i) {
  return Fo(e, t, r, n, i) && Io(e, t, r, n, i);
}
class Do {
  constructor(t, r) {
    this._wordSeparators = t, this._searchRegex = r, this._prevMatchStartIndex = -1, this._prevMatchLength = 0;
  }
  reset(t) {
    this._searchRegex.lastIndex = t, this._prevMatchStartIndex = -1, this._prevMatchLength = 0;
  }
  next(t) {
    const r = t.length;
    let n;
    do {
      if (this._prevMatchStartIndex + this._prevMatchLength === r || (n = this._searchRegex.exec(t), !n))
        return null;
      const i = n.index, s = n[0].length;
      if (i === this._prevMatchStartIndex && s === this._prevMatchLength) {
        if (s === 0) {
          Oa(t, r, this._searchRegex.lastIndex) > 65535 ? this._searchRegex.lastIndex += 2 : this._searchRegex.lastIndex += 1;
          continue;
        }
        return null;
      }
      if (this._prevMatchStartIndex = i, this._prevMatchLength = s, !this._wordSeparators || Vo(this._wordSeparators, t, r, i, s))
        return n;
    } while (n);
    return null;
  }
}
function Oo(e, t = "Unreachable") {
  throw new Error(t);
}
function fn(e) {
  if (!e()) {
    debugger;
    e(), Ss(new et("Assertion Failed"));
  }
}
function js(e, t) {
  let r = 0;
  for (; r < e.length - 1; ) {
    const n = e[r], i = e[r + 1];
    if (!t(n, i))
      return !1;
    r++;
  }
  return !0;
}
class jo {
  static computeUnicodeHighlights(t, r, n) {
    const i = n ? n.startLineNumber : 1, s = n ? n.endLineNumber : t.getLineCount(), a = new Ci(r), o = a.getCandidateCodePoints();
    let l;
    o === "allNonBasicAscii" ? l = new RegExp("[^\\t\\n\\r\\x20-\\x7E]", "g") : l = new RegExp(`${Uo(Array.from(o))}`, "g");
    const u = new Do(null, l), f = [];
    let h = !1, d, g = 0, p = 0, m = 0;
    e:
      for (let v = i, y = s; v <= y; v++) {
        const x = t.getLineContent(v), b = x.length;
        u.reset(0);
        do
          if (d = u.next(x), d) {
            let S = d.index, M = d.index + d[0].length;
            if (S > 0) {
              const w = x.charCodeAt(S - 1);
              Tr(w) && S--;
            }
            if (M + 1 < b) {
              const w = x.charCodeAt(M - 1);
              Tr(w) && M++;
            }
            const A = x.substring(S, M);
            let C = ln(S + 1, Ds, x, 0);
            C && C.endColumn <= S + 1 && (C = null);
            const _ = a.shouldHighlightNonBasicASCII(A, C ? C.word : null);
            if (_ !== 0) {
              _ === 3 ? g++ : _ === 2 ? p++ : _ === 1 ? m++ : Oo();
              const w = 1e3;
              if (f.length >= w) {
                h = !0;
                break e;
              }
              f.push(new fe(v, S + 1, v, M + 1));
            }
          }
        while (d);
      }
    return {
      ranges: f,
      hasMore: h,
      ambiguousCharacterCount: g,
      invisibleCharacterCount: p,
      nonBasicAsciiCharacterCount: m
    };
  }
  static computeUnicodeHighlightReason(t, r) {
    const n = new Ci(r);
    switch (n.shouldHighlightNonBasicASCII(t, null)) {
      case 0:
        return null;
      case 2:
        return {
          kind: 1
          /* UnicodeHighlighterReasonKind.Invisible */
        };
      case 3: {
        const s = t.codePointAt(0), a = n.ambiguousCharacters.getPrimaryConfusable(s), o = ye.getLocales().filter((l) => !ye.getInstance(/* @__PURE__ */ new Set([...r.allowedLocales, l])).isAmbiguous(s));
        return { kind: 0, confusableWith: String.fromCodePoint(a), notAmbiguousInLocales: o };
      }
      case 1:
        return {
          kind: 2
          /* UnicodeHighlighterReasonKind.NonBasicAscii */
        };
    }
  }
}
function Uo(e, t) {
  return `[${Pa(e.map((n) => String.fromCodePoint(n)).join(""))}]`;
}
class Ci {
  constructor(t) {
    this.options = t, this.allowedCodePoints = new Set(t.allowedCodePoints), this.ambiguousCharacters = ye.getInstance(new Set(t.allowedLocales));
  }
  getCandidateCodePoints() {
    if (this.options.nonBasicASCII)
      return "allNonBasicAscii";
    const t = /* @__PURE__ */ new Set();
    if (this.options.invisibleCharacters)
      for (const r of Je.codePoints)
        ki(String.fromCodePoint(r)) || t.add(r);
    if (this.options.ambiguousCharacters)
      for (const r of this.ambiguousCharacters.getConfusableCodePoints())
        t.add(r);
    for (const r of this.allowedCodePoints)
      t.delete(r);
    return t;
  }
  shouldHighlightNonBasicASCII(t, r) {
    const n = t.codePointAt(0);
    if (this.allowedCodePoints.has(n))
      return 0;
    if (this.options.nonBasicASCII)
      return 1;
    let i = !1, s = !1;
    if (r)
      for (const a of r) {
        const o = a.codePointAt(0), l = Ua(a);
        i = i || l, !l && !this.ambiguousCharacters.isAmbiguous(o) && !Je.isInvisibleCharacter(o) && (s = !0);
      }
    return (
      /* Don't allow mixing weird looking characters with ASCII */
      !i && /* Is there an obviously weird looking character? */
      s ? 0 : this.options.invisibleCharacters && !ki(t) && Je.isInvisibleCharacter(n) ? 2 : this.options.ambiguousCharacters && this.ambiguousCharacters.isAmbiguous(n) ? 3 : 0
    );
  }
}
function ki(e) {
  return e === " " || e === `
` || e === "	";
}
class Us {
  constructor(t, r) {
    this.changes = t, this.hitTimeout = r;
  }
}
class Kt {
  constructor(t, r, n) {
    this.originalRange = t, this.modifiedRange = r, this.innerChanges = n;
  }
  toString() {
    return `{${this.originalRange.toString()}->${this.modifiedRange.toString()}}`;
  }
}
class Bs {
  constructor(t, r) {
    this.originalRange = t, this.modifiedRange = r;
  }
  toString() {
    return `{${this.originalRange.toString()}->${this.modifiedRange.toString()}}`;
  }
}
class Ae {
  /**
   * @param lineRanges An array of sorted line ranges.
   */
  static joinMany(t) {
    if (t.length === 0)
      return [];
    let r = t[0];
    for (let n = 1; n < t.length; n++)
      r = this.join(r, t[n]);
    return r;
  }
  /**
   * @param lineRanges1 Must be sorted.
   * @param lineRanges2 Must be sorted.
   */
  static join(t, r) {
    if (t.length === 0)
      return r;
    if (r.length === 0)
      return t;
    const n = [];
    let i = 0, s = 0, a = null;
    for (; i < t.length || s < r.length; ) {
      let o = null;
      if (i < t.length && s < r.length) {
        const l = t[i], u = r[s];
        l.startLineNumber < u.startLineNumber ? (o = l, i++) : (o = u, s++);
      } else
        i < t.length ? (o = t[i], i++) : (o = r[s], s++);
      a === null ? a = o : a.endLineNumberExclusive >= o.startLineNumber ? a = new Ae(a.startLineNumber, Math.max(a.endLineNumberExclusive, o.endLineNumberExclusive)) : (n.push(a), a = o);
    }
    return a !== null && n.push(a), n;
  }
  constructor(t, r) {
    if (t > r)
      throw new et(`startLineNumber ${t} cannot be after endLineNumberExclusive ${r}`);
    this.startLineNumber = t, this.endLineNumberExclusive = r;
  }
  /**
   * Indicates if this line range contains the given line number.
   */
  contains(t) {
    return this.startLineNumber <= t && t < this.endLineNumberExclusive;
  }
  /**
   * Indicates if this line range is empty.
   */
  get isEmpty() {
    return this.startLineNumber === this.endLineNumberExclusive;
  }
  /**
   * Moves this line range by the given offset of line numbers.
   */
  delta(t) {
    return new Ae(this.startLineNumber + t, this.endLineNumberExclusive + t);
  }
  /**
   * The number of lines this line range spans.
   */
  get length() {
    return this.endLineNumberExclusive - this.startLineNumber;
  }
  /**
   * Creates a line range that combines this and the given line range.
   */
  join(t) {
    return new Ae(Math.min(this.startLineNumber, t.startLineNumber), Math.max(this.endLineNumberExclusive, t.endLineNumberExclusive));
  }
  toString() {
    return `[${this.startLineNumber},${this.endLineNumberExclusive})`;
  }
  /**
   * The resulting range is empty if the ranges do not intersect, but touch.
   * If the ranges don't even touch, the result is undefined.
   */
  intersect(t) {
    const r = Math.max(this.startLineNumber, t.startLineNumber), n = Math.min(this.endLineNumberExclusive, t.endLineNumberExclusive);
    if (r <= n)
      return new Ae(r, n);
  }
  overlapOrTouch(t) {
    return this.startLineNumber <= t.endLineNumberExclusive && t.startLineNumber <= this.endLineNumberExclusive;
  }
  equals(t) {
    return this.startLineNumber === t.startLineNumber && this.endLineNumberExclusive === t.endLineNumberExclusive;
  }
}
const Bo = 3;
class $o {
  computeDiff(t, r, n) {
    var i;
    const a = new qs(t, r, {
      maxComputationTime: n.maxComputationTimeMs,
      shouldIgnoreTrimWhitespace: n.ignoreTrimWhitespace,
      shouldComputeCharChanges: !0,
      shouldMakePrettyDiff: !0,
      shouldPostProcessCharChanges: !0
    }).computeDiff(), o = [];
    let l = null;
    for (const u of a.changes) {
      let f;
      u.originalEndLineNumber === 0 ? f = new Ae(u.originalStartLineNumber + 1, u.originalStartLineNumber + 1) : f = new Ae(u.originalStartLineNumber, u.originalEndLineNumber + 1);
      let h;
      u.modifiedEndLineNumber === 0 ? h = new Ae(u.modifiedStartLineNumber + 1, u.modifiedStartLineNumber + 1) : h = new Ae(u.modifiedStartLineNumber, u.modifiedEndLineNumber + 1);
      let d = new Kt(f, h, (i = u.charChanges) === null || i === void 0 ? void 0 : i.map((g) => new Bs(new fe(g.originalStartLineNumber, g.originalStartColumn, g.originalEndLineNumber, g.originalEndColumn), new fe(g.modifiedStartLineNumber, g.modifiedStartColumn, g.modifiedEndLineNumber, g.modifiedEndColumn))));
      l && (l.modifiedRange.endLineNumberExclusive === d.modifiedRange.startLineNumber || l.originalRange.endLineNumberExclusive === d.originalRange.startLineNumber) && (d = new Kt(l.originalRange.join(d.originalRange), l.modifiedRange.join(d.modifiedRange), l.innerChanges && d.innerChanges ? l.innerChanges.concat(d.innerChanges) : void 0), o.pop()), o.push(d), l = d;
    }
    return fn(() => js(o, (u, f) => f.originalRange.startLineNumber - u.originalRange.endLineNumberExclusive === f.modifiedRange.startLineNumber - u.modifiedRange.endLineNumberExclusive && // There has to be an unchanged line in between (otherwise both diffs should have been joined)
    u.originalRange.endLineNumberExclusive < f.originalRange.startLineNumber && u.modifiedRange.endLineNumberExclusive < f.modifiedRange.startLineNumber)), new Us(o, a.quitEarly);
  }
}
function $s(e, t, r, n) {
  return new ze(e, t, r).ComputeDiff(n);
}
let Ei = class {
  constructor(t) {
    const r = [], n = [];
    for (let i = 0, s = t.length; i < s; i++)
      r[i] = qr(t[i], 1), n[i] = Wr(t[i], 1);
    this.lines = t, this._startColumns = r, this._endColumns = n;
  }
  getElements() {
    const t = [];
    for (let r = 0, n = this.lines.length; r < n; r++)
      t[r] = this.lines[r].substring(this._startColumns[r] - 1, this._endColumns[r] - 1);
    return t;
  }
  getStrictElement(t) {
    return this.lines[t];
  }
  getStartLineNumber(t) {
    return t + 1;
  }
  getEndLineNumber(t) {
    return t + 1;
  }
  createCharSequence(t, r, n) {
    const i = [], s = [], a = [];
    let o = 0;
    for (let l = r; l <= n; l++) {
      const u = this.lines[l], f = t ? this._startColumns[l] : 1, h = t ? this._endColumns[l] : u.length + 1;
      for (let d = f; d < h; d++)
        i[o] = u.charCodeAt(d - 1), s[o] = l + 1, a[o] = d, o++;
      !t && l < n && (i[o] = 10, s[o] = l + 1, a[o] = u.length + 1, o++);
    }
    return new qo(i, s, a);
  }
};
class qo {
  constructor(t, r, n) {
    this._charCodes = t, this._lineNumbers = r, this._columns = n;
  }
  toString() {
    return "[" + this._charCodes.map((t, r) => (t === 10 ? "\\n" : String.fromCharCode(t)) + `-(${this._lineNumbers[r]},${this._columns[r]})`).join(", ") + "]";
  }
  _assertIndex(t, r) {
    if (t < 0 || t >= r.length)
      throw new Error("Illegal index");
  }
  getElements() {
    return this._charCodes;
  }
  getStartLineNumber(t) {
    return t > 0 && t === this._lineNumbers.length ? this.getEndLineNumber(t - 1) : (this._assertIndex(t, this._lineNumbers), this._lineNumbers[t]);
  }
  getEndLineNumber(t) {
    return t === -1 ? this.getStartLineNumber(t + 1) : (this._assertIndex(t, this._lineNumbers), this._charCodes[t] === 10 ? this._lineNumbers[t] + 1 : this._lineNumbers[t]);
  }
  getStartColumn(t) {
    return t > 0 && t === this._columns.length ? this.getEndColumn(t - 1) : (this._assertIndex(t, this._columns), this._columns[t]);
  }
  getEndColumn(t) {
    return t === -1 ? this.getStartColumn(t + 1) : (this._assertIndex(t, this._columns), this._charCodes[t] === 10 ? 1 : this._columns[t] + 1);
  }
}
class ft {
  constructor(t, r, n, i, s, a, o, l) {
    this.originalStartLineNumber = t, this.originalStartColumn = r, this.originalEndLineNumber = n, this.originalEndColumn = i, this.modifiedStartLineNumber = s, this.modifiedStartColumn = a, this.modifiedEndLineNumber = o, this.modifiedEndColumn = l;
  }
  static createFromDiffChange(t, r, n) {
    const i = r.getStartLineNumber(t.originalStart), s = r.getStartColumn(t.originalStart), a = r.getEndLineNumber(t.originalStart + t.originalLength - 1), o = r.getEndColumn(t.originalStart + t.originalLength - 1), l = n.getStartLineNumber(t.modifiedStart), u = n.getStartColumn(t.modifiedStart), f = n.getEndLineNumber(t.modifiedStart + t.modifiedLength - 1), h = n.getEndColumn(t.modifiedStart + t.modifiedLength - 1);
    return new ft(i, s, a, o, l, u, f, h);
  }
}
function Wo(e) {
  if (e.length <= 1)
    return e;
  const t = [e[0]];
  let r = t[0];
  for (let n = 1, i = e.length; n < i; n++) {
    const s = e[n], a = s.originalStart - (r.originalStart + r.originalLength), o = s.modifiedStart - (r.modifiedStart + r.modifiedLength);
    Math.min(a, o) < Bo ? (r.originalLength = s.originalStart + s.originalLength - r.originalStart, r.modifiedLength = s.modifiedStart + s.modifiedLength - r.modifiedStart) : (t.push(s), r = s);
  }
  return t;
}
class wt {
  constructor(t, r, n, i, s) {
    this.originalStartLineNumber = t, this.originalEndLineNumber = r, this.modifiedStartLineNumber = n, this.modifiedEndLineNumber = i, this.charChanges = s;
  }
  static createFromDiffResult(t, r, n, i, s, a, o) {
    let l, u, f, h, d;
    if (r.originalLength === 0 ? (l = n.getStartLineNumber(r.originalStart) - 1, u = 0) : (l = n.getStartLineNumber(r.originalStart), u = n.getEndLineNumber(r.originalStart + r.originalLength - 1)), r.modifiedLength === 0 ? (f = i.getStartLineNumber(r.modifiedStart) - 1, h = 0) : (f = i.getStartLineNumber(r.modifiedStart), h = i.getEndLineNumber(r.modifiedStart + r.modifiedLength - 1)), a && r.originalLength > 0 && r.originalLength < 20 && r.modifiedLength > 0 && r.modifiedLength < 20 && s()) {
      const g = n.createCharSequence(t, r.originalStart, r.originalStart + r.originalLength - 1), p = i.createCharSequence(t, r.modifiedStart, r.modifiedStart + r.modifiedLength - 1);
      if (g.getElements().length > 0 && p.getElements().length > 0) {
        let m = $s(g, p, s, !0).changes;
        o && (m = Wo(m)), d = [];
        for (let v = 0, y = m.length; v < y; v++)
          d.push(ft.createFromDiffChange(m[v], g, p));
      }
    }
    return new wt(l, u, f, h, d);
  }
}
class qs {
  constructor(t, r, n) {
    this.shouldComputeCharChanges = n.shouldComputeCharChanges, this.shouldPostProcessCharChanges = n.shouldPostProcessCharChanges, this.shouldIgnoreTrimWhitespace = n.shouldIgnoreTrimWhitespace, this.shouldMakePrettyDiff = n.shouldMakePrettyDiff, this.originalLines = t, this.modifiedLines = r, this.original = new Ei(t), this.modified = new Ei(r), this.continueLineDiff = Mi(n.maxComputationTime), this.continueCharDiff = Mi(n.maxComputationTime === 0 ? 0 : Math.min(n.maxComputationTime, 5e3));
  }
  computeDiff() {
    if (this.original.lines.length === 1 && this.original.lines[0].length === 0)
      return this.modified.lines.length === 1 && this.modified.lines[0].length === 0 ? {
        quitEarly: !1,
        changes: []
      } : {
        quitEarly: !1,
        changes: [{
          originalStartLineNumber: 1,
          originalEndLineNumber: 1,
          modifiedStartLineNumber: 1,
          modifiedEndLineNumber: this.modified.lines.length,
          charChanges: void 0
        }]
      };
    if (this.modified.lines.length === 1 && this.modified.lines[0].length === 0)
      return {
        quitEarly: !1,
        changes: [{
          originalStartLineNumber: 1,
          originalEndLineNumber: this.original.lines.length,
          modifiedStartLineNumber: 1,
          modifiedEndLineNumber: 1,
          charChanges: void 0
        }]
      };
    const t = $s(this.original, this.modified, this.continueLineDiff, this.shouldMakePrettyDiff), r = t.changes, n = t.quitEarly;
    if (this.shouldIgnoreTrimWhitespace) {
      const o = [];
      for (let l = 0, u = r.length; l < u; l++)
        o.push(wt.createFromDiffResult(this.shouldIgnoreTrimWhitespace, r[l], this.original, this.modified, this.continueCharDiff, this.shouldComputeCharChanges, this.shouldPostProcessCharChanges));
      return {
        quitEarly: n,
        changes: o
      };
    }
    const i = [];
    let s = 0, a = 0;
    for (let o = -1, l = r.length; o < l; o++) {
      const u = o + 1 < l ? r[o + 1] : null, f = u ? u.originalStart : this.originalLines.length, h = u ? u.modifiedStart : this.modifiedLines.length;
      for (; s < f && a < h; ) {
        const d = this.originalLines[s], g = this.modifiedLines[a];
        if (d !== g) {
          {
            let p = qr(d, 1), m = qr(g, 1);
            for (; p > 1 && m > 1; ) {
              const v = d.charCodeAt(p - 2), y = g.charCodeAt(m - 2);
              if (v !== y)
                break;
              p--, m--;
            }
            (p > 1 || m > 1) && this._pushTrimWhitespaceCharChange(i, s + 1, 1, p, a + 1, 1, m);
          }
          {
            let p = Wr(d, 1), m = Wr(g, 1);
            const v = d.length + 1, y = g.length + 1;
            for (; p < v && m < y; ) {
              const x = d.charCodeAt(p - 1), b = d.charCodeAt(m - 1);
              if (x !== b)
                break;
              p++, m++;
            }
            (p < v || m < y) && this._pushTrimWhitespaceCharChange(i, s + 1, p, v, a + 1, m, y);
          }
        }
        s++, a++;
      }
      u && (i.push(wt.createFromDiffResult(this.shouldIgnoreTrimWhitespace, u, this.original, this.modified, this.continueCharDiff, this.shouldComputeCharChanges, this.shouldPostProcessCharChanges)), s += u.originalLength, a += u.modifiedLength);
    }
    return {
      quitEarly: n,
      changes: i
    };
  }
  _pushTrimWhitespaceCharChange(t, r, n, i, s, a, o) {
    if (this._mergeTrimWhitespaceCharChange(t, r, n, i, s, a, o))
      return;
    let l;
    this.shouldComputeCharChanges && (l = [new ft(r, n, r, i, s, a, s, o)]), t.push(new wt(r, r, s, s, l));
  }
  _mergeTrimWhitespaceCharChange(t, r, n, i, s, a, o) {
    const l = t.length;
    if (l === 0)
      return !1;
    const u = t[l - 1];
    return u.originalEndLineNumber === 0 || u.modifiedEndLineNumber === 0 ? !1 : u.originalEndLineNumber === r && u.modifiedEndLineNumber === s ? (this.shouldComputeCharChanges && u.charChanges && u.charChanges.push(new ft(r, n, r, i, s, a, s, o)), !0) : u.originalEndLineNumber + 1 === r && u.modifiedEndLineNumber + 1 === s ? (u.originalEndLineNumber = r, u.modifiedEndLineNumber = s, this.shouldComputeCharChanges && u.charChanges && u.charChanges.push(new ft(r, n, r, i, s, a, s, o)), !0) : !1;
  }
}
function qr(e, t) {
  const r = Fa(e);
  return r === -1 ? t : r + 1;
}
function Wr(e, t) {
  const r = Ia(e);
  return r === -1 ? t : r + 2;
}
function Mi(e) {
  if (e === 0)
    return () => !0;
  const t = Date.now();
  return () => Date.now() - t < e;
}
class K {
  static addRange(t, r) {
    let n = 0;
    for (; n < r.length && r[n].endExclusive < t.start; )
      n++;
    let i = n;
    for (; i < r.length && r[i].start <= t.endExclusive; )
      i++;
    if (n === i)
      r.splice(n, 0, t);
    else {
      const s = Math.min(t.start, r[n].start), a = Math.max(t.endExclusive, r[i - 1].endExclusive);
      r.splice(n, i - n, new K(s, a));
    }
  }
  static tryCreate(t, r) {
    if (!(t > r))
      return new K(t, r);
  }
  constructor(t, r) {
    if (this.start = t, this.endExclusive = r, t > r)
      throw new et(`Invalid range: ${this.toString()}`);
  }
  get isEmpty() {
    return this.start === this.endExclusive;
  }
  delta(t) {
    return new K(this.start + t, this.endExclusive + t);
  }
  get length() {
    return this.endExclusive - this.start;
  }
  toString() {
    return `[${this.start}, ${this.endExclusive})`;
  }
  equals(t) {
    return this.start === t.start && this.endExclusive === t.endExclusive;
  }
  containsRange(t) {
    return this.start <= t.start && t.endExclusive <= this.endExclusive;
  }
  contains(t) {
    return this.start <= t && t < this.endExclusive;
  }
  /**
   * for all numbers n: range1.contains(n) or range2.contains(n) => range1.join(range2).contains(n)
   * The joined range is the smallest range that contains both ranges.
   */
  join(t) {
    return new K(Math.min(this.start, t.start), Math.max(this.endExclusive, t.endExclusive));
  }
  /**
   * for all numbers n: range1.contains(n) and range2.contains(n) <=> range1.intersect(range2).contains(n)
   *
   * The resulting range is empty if the ranges do not intersect, but touch.
   * If the ranges don't even touch, the result is undefined.
   */
  intersect(t) {
    const r = Math.max(this.start, t.start), n = Math.min(this.endExclusive, t.endExclusive);
    if (r <= n)
      return new K(r, n);
  }
}
class Ve {
  static trivial(t, r) {
    return new Ve([new xe(new K(0, t.length), new K(0, r.length))], !1);
  }
  static trivialTimedOut(t, r) {
    return new Ve([new xe(new K(0, t.length), new K(0, r.length))], !0);
  }
  constructor(t, r) {
    this.diffs = t, this.hitTimeout = r;
  }
}
class xe {
  constructor(t, r) {
    this.seq1Range = t, this.seq2Range = r;
  }
  reverse() {
    return new xe(this.seq2Range, this.seq1Range);
  }
  toString() {
    return `${this.seq1Range} <-> ${this.seq2Range}`;
  }
  join(t) {
    return new xe(this.seq1Range.join(t.seq1Range), this.seq2Range.join(t.seq2Range));
  }
}
class Nt {
  isValid() {
    return !0;
  }
}
Nt.instance = new Nt();
class Ho {
  constructor(t) {
    if (this.timeout = t, this.startTime = Date.now(), this.valid = !0, t <= 0)
      throw new et("timeout must be positive");
  }
  // Recommendation: Set a log-point `{this.disable()}` in the body
  isValid() {
    if (!(Date.now() - this.startTime < this.timeout) && this.valid) {
      this.valid = !1;
      debugger;
    }
    return this.valid;
  }
  disable() {
    this.timeout = Number.MAX_SAFE_INTEGER, this.isValid = () => !0, this.valid = !0;
  }
}
class pr {
  constructor(t, r) {
    this.width = t, this.height = r, this.array = [], this.array = new Array(t * r);
  }
  get(t, r) {
    return this.array[t + r * this.width];
  }
  set(t, r, n) {
    this.array[t + r * this.width] = n;
  }
}
class zo {
  compute(t, r, n = Nt.instance, i) {
    if (t.length === 0 || r.length === 0)
      return Ve.trivial(t, r);
    const s = new pr(t.length, r.length), a = new pr(t.length, r.length), o = new pr(t.length, r.length);
    for (let p = 0; p < t.length; p++)
      for (let m = 0; m < r.length; m++) {
        if (!n.isValid())
          return Ve.trivialTimedOut(t, r);
        const v = p === 0 ? 0 : s.get(p - 1, m), y = m === 0 ? 0 : s.get(p, m - 1);
        let x;
        t.getElement(p) === r.getElement(m) ? (p === 0 || m === 0 ? x = 0 : x = s.get(p - 1, m - 1), p > 0 && m > 0 && a.get(p - 1, m - 1) === 3 && (x += o.get(p - 1, m - 1)), x += i ? i(p, m) : 1) : x = -1;
        const b = Math.max(v, y, x);
        if (b === x) {
          const S = p > 0 && m > 0 ? o.get(p - 1, m - 1) : 0;
          o.set(p, m, S + 1), a.set(p, m, 3);
        } else
          b === v ? (o.set(p, m, 0), a.set(p, m, 1)) : b === y && (o.set(p, m, 0), a.set(p, m, 2));
        s.set(p, m, b);
      }
    const l = [];
    let u = t.length, f = r.length;
    function h(p, m) {
      (p + 1 !== u || m + 1 !== f) && l.push(new xe(new K(p + 1, u), new K(m + 1, f))), u = p, f = m;
    }
    let d = t.length - 1, g = r.length - 1;
    for (; d >= 0 && g >= 0; )
      a.get(d, g) === 3 ? (h(d, g), d--, g--) : a.get(d, g) === 1 ? d-- : g--;
    return h(-1, -1), l.reverse(), new Ve(l, !1);
  }
}
function Ti(e, t, r) {
  let n = r;
  return n = Jo(e, t, n), n = Xo(e, t, n), n;
}
function Go(e, t, r) {
  const n = [];
  for (const i of r) {
    const s = n[n.length - 1];
    if (!s) {
      n.push(i);
      continue;
    }
    i.seq1Range.start - s.seq1Range.endExclusive <= 2 || i.seq2Range.start - s.seq2Range.endExclusive <= 2 ? n[n.length - 1] = new xe(s.seq1Range.join(i.seq1Range), s.seq2Range.join(i.seq2Range)) : n.push(i);
  }
  return n;
}
function Jo(e, t, r) {
  const n = [];
  r.length > 0 && n.push(r[0]);
  for (let i = 1; i < r.length; i++) {
    const s = n[n.length - 1], a = r[i];
    if (a.seq1Range.isEmpty) {
      let o = !0;
      const l = a.seq1Range.start - s.seq1Range.endExclusive;
      for (let u = 1; u <= l; u++)
        if (t.getElement(a.seq2Range.start - u) !== t.getElement(a.seq2Range.endExclusive - u)) {
          o = !1;
          break;
        }
      if (o) {
        n[n.length - 1] = new xe(s.seq1Range, new K(s.seq2Range.start, a.seq2Range.endExclusive - l));
        continue;
      }
    }
    n.push(a);
  }
  return n;
}
function Xo(e, t, r) {
  if (!e.getBoundaryScore || !t.getBoundaryScore)
    return r;
  for (let n = 0; n < r.length; n++) {
    const i = r[n];
    if (i.seq1Range.isEmpty) {
      const s = n > 0 ? r[n - 1].seq2Range.endExclusive : -1, a = n + 1 < r.length ? r[n + 1].seq2Range.start : t.length;
      r[n] = Pi(i, e, t, a, s);
    } else if (i.seq2Range.isEmpty) {
      const s = n > 0 ? r[n - 1].seq1Range.endExclusive : -1, a = n + 1 < r.length ? r[n + 1].seq1Range.start : e.length;
      r[n] = Pi(i.reverse(), t, e, a, s).reverse();
    }
  }
  return r;
}
function Pi(e, t, r, n, i) {
  let a = 1;
  for (; e.seq2Range.start - a > i && r.getElement(e.seq2Range.start - a) === r.getElement(e.seq2Range.endExclusive - a) && a < 20; )
    a++;
  a--;
  let o = 0;
  for (; e.seq2Range.start + o < n && r.getElement(e.seq2Range.start + o) === r.getElement(e.seq2Range.endExclusive + o) && o < 20; )
    o++;
  if (a === 0 && o === 0)
    return e;
  let l = 0, u = -1;
  for (let f = -a; f <= o; f++) {
    const h = e.seq2Range.start + f, d = e.seq2Range.endExclusive + f, g = e.seq1Range.start + f, p = t.getBoundaryScore(g) + r.getBoundaryScore(h) + r.getBoundaryScore(d);
    p > u && (u = p, l = f);
  }
  return l !== 0 ? new xe(e.seq1Range.delta(l), e.seq2Range.delta(l)) : e;
}
class Zo {
  compute(t, r, n = Nt.instance) {
    if (t.length === 0 || r.length === 0)
      return Ve.trivial(t, r);
    function i(g, p) {
      for (; g < t.length && p < r.length && t.getElement(g) === r.getElement(p); )
        g++, p++;
      return g;
    }
    let s = 0;
    const a = new Qo();
    a.set(0, i(0, 0));
    const o = new Yo();
    o.set(0, a.get(0) === 0 ? null : new Ri(null, 0, 0, a.get(0)));
    let l = 0;
    e:
      for (; ; )
        for (s++, l = -s; l <= s; l += 2) {
          if (!n.isValid())
            return Ve.trivialTimedOut(t, r);
          const g = l === s ? -1 : a.get(l + 1), p = l === -s ? -1 : a.get(l - 1) + 1, m = Math.min(Math.max(g, p), t.length), v = m - l, y = i(m, v);
          a.set(l, y);
          const x = m === g ? o.get(l + 1) : o.get(l - 1);
          if (o.set(l, y !== m ? new Ri(x, m, v, y - m) : x), a.get(l) === t.length && a.get(l) - l === r.length)
            break e;
        }
    let u = o.get(l);
    const f = [];
    let h = t.length, d = r.length;
    for (; ; ) {
      const g = u ? u.x + u.length : 0, p = u ? u.y + u.length : 0;
      if ((g !== h || p !== d) && f.push(new xe(new K(g, h), new K(p, d))), !u)
        break;
      h = u.x, d = u.y, u = u.prev;
    }
    return f.reverse(), new Ve(f, !1);
  }
}
class Ri {
  constructor(t, r, n, i) {
    this.prev = t, this.x = r, this.y = n, this.length = i;
  }
}
class Qo {
  constructor() {
    this.positiveArr = new Int32Array(10), this.negativeArr = new Int32Array(10);
  }
  get(t) {
    return t < 0 ? (t = -t - 1, this.negativeArr[t]) : this.positiveArr[t];
  }
  set(t, r) {
    if (t < 0) {
      if (t = -t - 1, t >= this.negativeArr.length) {
        const n = this.negativeArr;
        this.negativeArr = new Int32Array(n.length * 2), this.negativeArr.set(n);
      }
      this.negativeArr[t] = r;
    } else {
      if (t >= this.positiveArr.length) {
        const n = this.positiveArr;
        this.positiveArr = new Int32Array(n.length * 2), this.positiveArr.set(n);
      }
      this.positiveArr[t] = r;
    }
  }
}
class Yo {
  constructor() {
    this.positiveArr = [], this.negativeArr = [];
  }
  get(t) {
    return t < 0 ? (t = -t - 1, this.negativeArr[t]) : this.positiveArr[t];
  }
  set(t, r) {
    t < 0 ? (t = -t - 1, this.negativeArr[t] = r) : this.positiveArr[t] = r;
  }
}
class Ko {
  constructor() {
    this.dynamicProgrammingDiffing = new zo(), this.myersDiffingAlgorithm = new Zo();
  }
  computeDiff(t, r, n) {
    const i = n.maxComputationTimeMs === 0 ? Nt.instance : new Ho(n.maxComputationTimeMs), s = !n.ignoreTrimWhitespace, a = /* @__PURE__ */ new Map();
    function o(S) {
      let M = a.get(S);
      return M === void 0 && (M = a.size, a.set(S, M)), M;
    }
    const l = t.map((S) => o(S.trim())), u = r.map((S) => o(S.trim())), f = new Fi(l, t), h = new Fi(u, r), d = (() => f.length + h.length < 1500 ? this.dynamicProgrammingDiffing.compute(f, h, i, (S, M) => t[S] === r[M] ? r[M].length === 0 ? 0.1 : 1 + Math.log(1 + r[M].length) : 0.99) : this.myersDiffingAlgorithm.compute(f, h))();
    let g = d.diffs, p = d.hitTimeout;
    g = Ti(f, h, g);
    const m = [], v = (S) => {
      if (s)
        for (let M = 0; M < S; M++) {
          const A = y + M, C = x + M;
          if (t[A] !== r[C]) {
            const _ = this.refineDiff(t, r, new xe(new K(A, A + 1), new K(C, C + 1)), i, s);
            for (const w of _.mappings)
              m.push(w);
            _.hitTimeout && (p = !0);
          }
        }
    };
    let y = 0, x = 0;
    for (const S of g) {
      fn(() => S.seq1Range.start - y === S.seq2Range.start - x);
      const M = S.seq1Range.start - y;
      v(M), y = S.seq1Range.endExclusive, x = S.seq2Range.endExclusive;
      const A = this.refineDiff(t, r, S, i, s);
      A.hitTimeout && (p = !0);
      for (const C of A.mappings)
        m.push(C);
    }
    v(t.length - y);
    const b = rl(m, t, r);
    return new Us(b, p);
  }
  refineDiff(t, r, n, i, s) {
    const a = new Vi(t, n.seq1Range, s), o = new Vi(r, n.seq2Range, s), l = a.length + o.length < 500 ? this.dynamicProgrammingDiffing.compute(a, o, i) : this.myersDiffingAlgorithm.compute(a, o, i);
    let u = l.diffs;
    return u = Ti(a, o, u), u = el(a, o, u), u = Go(a, o, u), {
      mappings: u.map((h) => new Bs(a.translateRange(h.seq1Range), o.translateRange(h.seq2Range))),
      hitTimeout: l.hitTimeout
    };
  }
}
function el(e, t, r) {
  const n = [];
  let i;
  function s() {
    if (!i)
      return;
    const l = i.s1Range.length - i.deleted;
    i.s2Range.length - i.added, Math.max(i.deleted, i.added) + (i.count - 1) > l && n.push(new xe(i.s1Range, i.s2Range)), i = void 0;
  }
  for (const l of r) {
    let u = function(p, m) {
      var v, y, x, b;
      if (!i || !i.s1Range.containsRange(p) || !i.s2Range.containsRange(m))
        if (i && !(i.s1Range.endExclusive < p.start && i.s2Range.endExclusive < m.start)) {
          const A = K.tryCreate(i.s1Range.endExclusive, p.start), C = K.tryCreate(i.s2Range.endExclusive, m.start);
          i.deleted += (v = A == null ? void 0 : A.length) !== null && v !== void 0 ? v : 0, i.added += (y = C == null ? void 0 : C.length) !== null && y !== void 0 ? y : 0, i.s1Range = i.s1Range.join(p), i.s2Range = i.s2Range.join(m);
        } else
          s(), i = { added: 0, deleted: 0, count: 0, s1Range: p, s2Range: m };
      const S = p.intersect(l.seq1Range), M = m.intersect(l.seq2Range);
      i.count++, i.deleted += (x = S == null ? void 0 : S.length) !== null && x !== void 0 ? x : 0, i.added += (b = M == null ? void 0 : M.length) !== null && b !== void 0 ? b : 0;
    };
    var o = u;
    const f = e.findWordContaining(l.seq1Range.start - 1), h = t.findWordContaining(l.seq2Range.start - 1), d = e.findWordContaining(l.seq1Range.endExclusive), g = t.findWordContaining(l.seq2Range.endExclusive);
    f && d && h && g && f.equals(d) && h.equals(g) ? u(f, h) : (f && h && u(f, h), d && g && u(d, g));
  }
  return s(), tl(r, n);
}
function tl(e, t) {
  const r = [];
  for (; e.length > 0 || t.length > 0; ) {
    const n = e[0], i = t[0];
    let s;
    n && (!i || n.seq1Range.start < i.seq1Range.start) ? s = e.shift() : s = t.shift(), r.length > 0 && r[r.length - 1].seq1Range.endExclusive >= s.seq1Range.start ? r[r.length - 1] = r[r.length - 1].join(s) : r.push(s);
  }
  return r;
}
function rl(e, t, r) {
  const n = [];
  for (const i of il(e.map((s) => nl(s, t, r)), (s, a) => s.originalRange.overlapOrTouch(a.originalRange) || s.modifiedRange.overlapOrTouch(a.modifiedRange))) {
    const s = i[0], a = i[i.length - 1];
    n.push(new Kt(s.originalRange.join(a.originalRange), s.modifiedRange.join(a.modifiedRange), i.map((o) => o.innerChanges[0])));
  }
  return fn(() => js(n, (i, s) => s.originalRange.startLineNumber - i.originalRange.endLineNumberExclusive === s.modifiedRange.startLineNumber - i.modifiedRange.endLineNumberExclusive && // There has to be an unchanged line in between (otherwise both diffs should have been joined)
  i.originalRange.endLineNumberExclusive < s.originalRange.startLineNumber && i.modifiedRange.endLineNumberExclusive < s.modifiedRange.startLineNumber)), n;
}
function nl(e, t, r) {
  let n = 0, i = 0;
  e.modifiedRange.startColumn - 1 >= r[e.modifiedRange.startLineNumber - 1].length && e.originalRange.startColumn - 1 >= t[e.originalRange.startLineNumber - 1].length && (n = 1), e.modifiedRange.endColumn === 1 && e.originalRange.endColumn === 1 && e.originalRange.startLineNumber + n <= e.originalRange.endLineNumber && e.modifiedRange.startLineNumber + n <= e.modifiedRange.endLineNumber && (i = -1);
  const s = new Ae(e.originalRange.startLineNumber + n, e.originalRange.endLineNumber + 1 + i), a = new Ae(e.modifiedRange.startLineNumber + n, e.modifiedRange.endLineNumber + 1 + i);
  return new Kt(s, a, [e]);
}
function* il(e, t) {
  let r, n;
  for (const i of e)
    n !== void 0 && t(n, i) ? r.push(i) : (r && (yield r), r = [i]), n = i;
  r && (yield r);
}
class Fi {
  constructor(t, r) {
    this.trimmedHash = t, this.lines = r;
  }
  getElement(t) {
    return this.trimmedHash[t];
  }
  get length() {
    return this.trimmedHash.length;
  }
  getBoundaryScore(t) {
    const r = t === 0 ? 0 : Ii(this.lines[t - 1]), n = t === this.lines.length ? 0 : Ii(this.lines[t]);
    return 1e3 - (r + n);
  }
}
function Ii(e) {
  let t = 0;
  for (; t < e.length && (e.charCodeAt(t) === 32 || e.charCodeAt(t) === 9); )
    t++;
  return t;
}
class Vi {
  constructor(t, r, n) {
    this.lines = t, this.considerWhitespaceChanges = n, this.elements = [], this.firstCharOffsetByLineMinusOne = [], this.offsetByLine = [];
    let i = !1;
    r.start > 0 && r.endExclusive >= t.length && (r = new K(r.start - 1, r.endExclusive), i = !0), this.lineRange = r;
    for (let s = this.lineRange.start; s < this.lineRange.endExclusive; s++) {
      let a = t[s], o = 0;
      if (i)
        o = a.length, a = "", i = !1;
      else if (!n) {
        const l = a.trimStart();
        o = a.length - l.length, a = l.trimEnd();
      }
      this.offsetByLine.push(o);
      for (let l = 0; l < a.length; l++)
        this.elements.push(a.charCodeAt(l));
      s < t.length - 1 && (this.elements.push(`
`.charCodeAt(0)), this.firstCharOffsetByLineMinusOne[s - this.lineRange.start] = this.elements.length);
    }
    this.offsetByLine.push(0);
  }
  toString() {
    return `Slice: "${this.text}"`;
  }
  get text() {
    return [...this.elements].map((t) => String.fromCharCode(t)).join("");
  }
  getElement(t) {
    return this.elements[t];
  }
  get length() {
    return this.elements.length;
  }
  getBoundaryScore(t) {
    const r = Oi(t > 0 ? this.elements[t - 1] : -1), n = Oi(t < this.elements.length ? this.elements[t] : -1);
    if (r === 6 && n === 7)
      return 0;
    let i = 0;
    return r !== n && (i += 10, n === 1 && (i += 1)), i += Di(r), i += Di(n), i;
  }
  translateOffset(t) {
    if (this.lineRange.isEmpty)
      return new Re(this.lineRange.start + 1, 1);
    let r = 0, n = this.firstCharOffsetByLineMinusOne.length;
    for (; r < n; ) {
      const s = Math.floor((r + n) / 2);
      this.firstCharOffsetByLineMinusOne[s] > t ? n = s : r = s + 1;
    }
    const i = r === 0 ? 0 : this.firstCharOffsetByLineMinusOne[r - 1];
    return new Re(this.lineRange.start + r + 1, t - i + 1 + this.offsetByLine[r]);
  }
  translateRange(t) {
    return fe.fromPositions(this.translateOffset(t.start), this.translateOffset(t.endExclusive));
  }
  /**
   * Finds the word that contains the character at the given offset
   */
  findWordContaining(t) {
    if (t < 0 || t >= this.elements.length || !vr(this.elements[t]))
      return;
    let r = t;
    for (; r > 0 && vr(this.elements[r - 1]); )
      r--;
    let n = t;
    for (; n < this.elements.length && vr(this.elements[n]); )
      n++;
    return new K(r, n);
  }
}
function vr(e) {
  return e >= 97 && e <= 122 || e >= 65 && e <= 90 || e >= 48 && e <= 57;
}
const sl = {
  [
    0
    /* CharBoundaryCategory.WordLower */
  ]: 0,
  [
    1
    /* CharBoundaryCategory.WordUpper */
  ]: 0,
  [
    2
    /* CharBoundaryCategory.WordNumber */
  ]: 0,
  [
    3
    /* CharBoundaryCategory.End */
  ]: 10,
  [
    4
    /* CharBoundaryCategory.Other */
  ]: 2,
  [
    5
    /* CharBoundaryCategory.Space */
  ]: 3,
  [
    6
    /* CharBoundaryCategory.LineBreakCR */
  ]: 10,
  [
    7
    /* CharBoundaryCategory.LineBreakLF */
  ]: 10
};
function Di(e) {
  return sl[e];
}
function Oi(e) {
  return e === 10 ? 7 : e === 13 ? 6 : al(e) ? 5 : e >= 97 && e <= 122 ? 0 : e >= 65 && e <= 90 ? 1 : e >= 48 && e <= 57 ? 2 : e === -1 ? 3 : 4;
}
function al(e) {
  return e === 32 || e === 9;
}
const br = {
  smart: new $o(),
  experimental: new Ko()
};
var Fe = globalThis && globalThis.__awaiter || function(e, t, r, n) {
  function i(s) {
    return s instanceof r ? s : new r(function(a) {
      a(s);
    });
  }
  return new (r || (r = Promise))(function(s, a) {
    function o(f) {
      try {
        u(n.next(f));
      } catch (h) {
        a(h);
      }
    }
    function l(f) {
      try {
        u(n.throw(f));
      } catch (h) {
        a(h);
      }
    }
    function u(f) {
      f.done ? s(f.value) : i(f.value).then(o, l);
    }
    u((n = n.apply(e, t || [])).next());
  });
};
class ol extends po {
  get uri() {
    return this._uri;
  }
  get eol() {
    return this._eol;
  }
  getValue() {
    return this.getText();
  }
  getLinesContent() {
    return this._lines.slice(0);
  }
  getLineCount() {
    return this._lines.length;
  }
  getLineContent(t) {
    return this._lines[t - 1];
  }
  getWordAtPosition(t, r) {
    const n = ln(t.column, yo(r), this._lines[t.lineNumber - 1], 0);
    return n ? new fe(t.lineNumber, n.startColumn, t.lineNumber, n.endColumn) : null;
  }
  getWordUntilPosition(t, r) {
    const n = this.getWordAtPosition(t, r);
    return n ? {
      word: this._lines[t.lineNumber - 1].substring(n.startColumn - 1, t.column - 1),
      startColumn: n.startColumn,
      endColumn: t.column
    } : {
      word: "",
      startColumn: t.column,
      endColumn: t.column
    };
  }
  words(t) {
    const r = this._lines, n = this._wordenize.bind(this);
    let i = 0, s = "", a = 0, o = [];
    return {
      *[Symbol.iterator]() {
        for (; ; )
          if (a < o.length) {
            const l = s.substring(o[a].start, o[a].end);
            a += 1, yield l;
          } else if (i < r.length)
            s = r[i], o = n(s, t), a = 0, i += 1;
          else
            break;
      }
    };
  }
  getLineWords(t, r) {
    const n = this._lines[t - 1], i = this._wordenize(n, r), s = [];
    for (const a of i)
      s.push({
        word: n.substring(a.start, a.end),
        startColumn: a.start + 1,
        endColumn: a.end + 1
      });
    return s;
  }
  _wordenize(t, r) {
    const n = [];
    let i;
    for (r.lastIndex = 0; (i = r.exec(t)) && i[0].length !== 0; )
      n.push({ start: i.index, end: i.index + i[0].length });
    return n;
  }
  getValueInRange(t) {
    if (t = this._validateRange(t), t.startLineNumber === t.endLineNumber)
      return this._lines[t.startLineNumber - 1].substring(t.startColumn - 1, t.endColumn - 1);
    const r = this._eol, n = t.startLineNumber - 1, i = t.endLineNumber - 1, s = [];
    s.push(this._lines[n].substring(t.startColumn - 1));
    for (let a = n + 1; a < i; a++)
      s.push(this._lines[a]);
    return s.push(this._lines[i].substring(0, t.endColumn - 1)), s.join(r);
  }
  offsetAt(t) {
    return t = this._validatePosition(t), this._ensureLineStarts(), this._lineStarts.getPrefixSum(t.lineNumber - 2) + (t.column - 1);
  }
  positionAt(t) {
    t = Math.floor(t), t = Math.max(0, t), this._ensureLineStarts();
    const r = this._lineStarts.getIndexOf(t), n = this._lines[r.index].length;
    return {
      lineNumber: 1 + r.index,
      column: 1 + Math.min(r.remainder, n)
    };
  }
  _validateRange(t) {
    const r = this._validatePosition({ lineNumber: t.startLineNumber, column: t.startColumn }), n = this._validatePosition({ lineNumber: t.endLineNumber, column: t.endColumn });
    return r.lineNumber !== t.startLineNumber || r.column !== t.startColumn || n.lineNumber !== t.endLineNumber || n.column !== t.endColumn ? {
      startLineNumber: r.lineNumber,
      startColumn: r.column,
      endLineNumber: n.lineNumber,
      endColumn: n.column
    } : t;
  }
  _validatePosition(t) {
    if (!Re.isIPosition(t))
      throw new Error("bad position");
    let { lineNumber: r, column: n } = t, i = !1;
    if (r < 1)
      r = 1, n = 1, i = !0;
    else if (r > this._lines.length)
      r = this._lines.length, n = this._lines[r - 1].length + 1, i = !0;
    else {
      const s = this._lines[r - 1].length + 1;
      n < 1 ? (n = 1, i = !0) : n > s && (n = s, i = !0);
    }
    return i ? { lineNumber: r, column: n } : t;
  }
}
class Ge {
  constructor(t, r) {
    this._host = t, this._models = /* @__PURE__ */ Object.create(null), this._foreignModuleFactory = r, this._foreignModule = null;
  }
  dispose() {
    this._models = /* @__PURE__ */ Object.create(null);
  }
  _getModel(t) {
    return this._models[t];
  }
  _getModels() {
    const t = [];
    return Object.keys(this._models).forEach((r) => t.push(this._models[r])), t;
  }
  acceptNewModel(t) {
    this._models[t.url] = new ol(on.parse(t.url), t.lines, t.EOL, t.versionId);
  }
  acceptModelChanged(t, r) {
    if (!this._models[t])
      return;
    this._models[t].onEvents(r);
  }
  acceptRemovedModel(t) {
    this._models[t] && delete this._models[t];
  }
  computeUnicodeHighlights(t, r, n) {
    return Fe(this, void 0, void 0, function* () {
      const i = this._getModel(t);
      return i ? jo.computeUnicodeHighlights(i, r, n) : { ranges: [], hasMore: !1, ambiguousCharacterCount: 0, invisibleCharacterCount: 0, nonBasicAsciiCharacterCount: 0 };
    });
  }
  // ---- BEGIN diff --------------------------------------------------------------------------
  computeDiff(t, r, n, i) {
    return Fe(this, void 0, void 0, function* () {
      const s = this._getModel(t), a = this._getModel(r);
      return !s || !a ? null : Ge.computeDiff(s, a, n, i);
    });
  }
  static computeDiff(t, r, n, i) {
    const s = i === "experimental" ? br.experimental : br.smart, a = t.getLinesContent(), o = r.getLinesContent(), l = s.computeDiff(a, o, n);
    return {
      identical: l.changes.length > 0 ? !1 : this._modelsAreIdentical(t, r),
      quitEarly: l.hitTimeout,
      changes: l.changes.map((f) => {
        var h;
        return [f.originalRange.startLineNumber, f.originalRange.endLineNumberExclusive, f.modifiedRange.startLineNumber, f.modifiedRange.endLineNumberExclusive, (h = f.innerChanges) === null || h === void 0 ? void 0 : h.map((d) => [
          d.originalRange.startLineNumber,
          d.originalRange.startColumn,
          d.originalRange.endLineNumber,
          d.originalRange.endColumn,
          d.modifiedRange.startLineNumber,
          d.modifiedRange.startColumn,
          d.modifiedRange.endLineNumber,
          d.modifiedRange.endColumn
        ])];
      })
    };
  }
  static _modelsAreIdentical(t, r) {
    const n = t.getLineCount(), i = r.getLineCount();
    if (n !== i)
      return !1;
    for (let s = 1; s <= n; s++) {
      const a = t.getLineContent(s), o = r.getLineContent(s);
      if (a !== o)
        return !1;
    }
    return !0;
  }
  computeDirtyDiff(t, r, n) {
    return Fe(this, void 0, void 0, function* () {
      const i = this._getModel(t), s = this._getModel(r);
      if (!i || !s)
        return null;
      const a = i.getLinesContent(), o = s.getLinesContent();
      return new qs(a, o, {
        shouldComputeCharChanges: !1,
        shouldPostProcessCharChanges: !1,
        shouldIgnoreTrimWhitespace: n,
        shouldMakePrettyDiff: !0,
        maxComputationTime: 1e3
      }).computeDiff().changes;
    });
  }
  computeMoreMinimalEdits(t, r, n) {
    return Fe(this, void 0, void 0, function* () {
      const i = this._getModel(t);
      if (!i)
        return r;
      const s = [];
      let a;
      r = r.slice(0).sort((o, l) => {
        if (o.range && l.range)
          return fe.compareRangesUsingStarts(o.range, l.range);
        const u = o.range ? 0 : 1, f = l.range ? 0 : 1;
        return u - f;
      });
      for (let { range: o, text: l, eol: u } of r) {
        if (typeof u == "number" && (a = u), fe.isEmpty(o) && !l)
          continue;
        const f = i.getValueInRange(o);
        if (l = l.replace(/\r\n|\n|\r/g, i.eol), f === l)
          continue;
        if (Math.max(l.length, f.length) > Ge._diffLimit) {
          s.push({ range: o, text: l });
          continue;
        }
        const h = Za(f, l, n), d = i.offsetAt(fe.lift(o).getStartPosition());
        for (const g of h) {
          const p = i.positionAt(d + g.originalStart), m = i.positionAt(d + g.originalStart + g.originalLength), v = {
            text: l.substr(g.modifiedStart, g.modifiedLength),
            range: { startLineNumber: p.lineNumber, startColumn: p.column, endLineNumber: m.lineNumber, endColumn: m.column }
          };
          i.getValueInRange(v.range) !== v.text && s.push(v);
        }
      }
      return typeof a == "number" && s.push({ eol: a, text: "", range: { startLineNumber: 0, startColumn: 0, endLineNumber: 0, endColumn: 0 } }), s;
    });
  }
  computeHumanReadableDiff(t, r, n) {
    return Fe(this, void 0, void 0, function* () {
      const i = this._getModel(t);
      if (!i)
        return r;
      const s = [];
      let a;
      r = r.slice(0).sort((u, f) => {
        if (u.range && f.range)
          return fe.compareRangesUsingStarts(u.range, f.range);
        const h = u.range ? 0 : 1, d = f.range ? 0 : 1;
        return h - d;
      });
      for (let { range: u, text: f, eol: h } of r) {
        let y = function(b, S) {
          return new Re(b.lineNumber + S.lineNumber - 1, S.lineNumber === 1 ? b.column + S.column - 1 : S.column);
        }, x = function(b, S) {
          const M = [];
          for (let A = S.startLineNumber; A <= S.endLineNumber; A++) {
            const C = b[A - 1];
            A === S.startLineNumber && A === S.endLineNumber ? M.push(C.substring(S.startColumn - 1, S.endColumn - 1)) : A === S.startLineNumber ? M.push(C.substring(S.startColumn - 1)) : A === S.endLineNumber ? M.push(C.substring(0, S.endColumn - 1)) : M.push(C);
          }
          return M;
        };
        var o = y, l = x;
        if (typeof h == "number" && (a = h), fe.isEmpty(u) && !f)
          continue;
        const d = i.getValueInRange(u);
        if (f = f.replace(/\r\n|\n|\r/g, i.eol), d === f)
          continue;
        if (Math.max(f.length, d.length) > Ge._diffLimit) {
          s.push({ range: u, text: f });
          continue;
        }
        const g = d.split(/\r\n|\n|\r/), p = f.split(/\r\n|\n|\r/), m = br.experimental.computeDiff(g, p, n), v = fe.lift(u).getStartPosition();
        for (const b of m.changes)
          if (b.innerChanges)
            for (const S of b.innerChanges)
              s.push({
                range: fe.fromPositions(y(v, S.originalRange.getStartPosition()), y(v, S.originalRange.getEndPosition())),
                text: x(p, S.modifiedRange).join(i.eol)
              });
          else
            throw new et("The experimental diff algorithm always produces inner changes");
      }
      return typeof a == "number" && s.push({ eol: a, text: "", range: { startLineNumber: 0, startColumn: 0, endLineNumber: 0, endColumn: 0 } }), s;
    });
  }
  // ---- END minimal edits ---------------------------------------------------------------
  computeLinks(t) {
    return Fe(this, void 0, void 0, function* () {
      const r = this._getModel(t);
      return r ? No(r) : null;
    });
  }
  textualSuggest(t, r, n, i) {
    return Fe(this, void 0, void 0, function* () {
      const s = new ur(!0), a = new RegExp(n, i), o = /* @__PURE__ */ new Set();
      e:
        for (const l of t) {
          const u = this._getModel(l);
          if (u) {
            for (const f of u.words(a))
              if (!(f === r || !isNaN(Number(f))) && (o.add(f), o.size > Ge._suggestionsLimit))
                break e;
          }
        }
      return { words: Array.from(o), duration: s.elapsed() };
    });
  }
  // ---- END suggest --------------------------------------------------------------------------
  //#region -- word ranges --
  computeWordRanges(t, r, n, i) {
    return Fe(this, void 0, void 0, function* () {
      const s = this._getModel(t);
      if (!s)
        return /* @__PURE__ */ Object.create(null);
      const a = new RegExp(n, i), o = /* @__PURE__ */ Object.create(null);
      for (let l = r.startLineNumber; l < r.endLineNumber; l++) {
        const u = s.getLineWords(l, a);
        for (const f of u) {
          if (!isNaN(Number(f.word)))
            continue;
          let h = o[f.word];
          h || (h = [], o[f.word] = h), h.push({
            startLineNumber: l,
            startColumn: f.startColumn,
            endLineNumber: l,
            endColumn: f.endColumn
          });
        }
      }
      return o;
    });
  }
  //#endregion
  navigateValueSet(t, r, n, i, s) {
    return Fe(this, void 0, void 0, function* () {
      const a = this._getModel(t);
      if (!a)
        return null;
      const o = new RegExp(i, s);
      r.startColumn === r.endColumn && (r = {
        startLineNumber: r.startLineNumber,
        startColumn: r.startColumn,
        endLineNumber: r.endLineNumber,
        endColumn: r.endColumn + 1
      });
      const l = a.getValueInRange(r), u = a.getWordAtPosition({ lineNumber: r.startLineNumber, column: r.startColumn }, o);
      if (!u)
        return null;
      const f = a.getValueInRange(u);
      return Ir.INSTANCE.navigateValueSet(r, l, u, f, n);
    });
  }
  // ---- BEGIN foreign module support --------------------------------------------------------------------------
  loadForeignModule(t, r, n) {
    const a = {
      host: Ea(n, (o, l) => this._host.fhr(o, l)),
      getMirrorModels: () => this._getModels()
    };
    return this._foreignModuleFactory ? (this._foreignModule = this._foreignModuleFactory(a, r), Promise.resolve(Mr(this._foreignModule))) : Promise.reject(new Error("Unexpected usage"));
  }
  // foreign method request
  fmr(t, r) {
    if (!this._foreignModule || typeof this._foreignModule[t] != "function")
      return Promise.reject(new Error("Missing requestHandler or method: " + t));
    try {
      return Promise.resolve(this._foreignModule[t].apply(this._foreignModule, r));
    } catch (n) {
      return Promise.reject(n);
    }
  }
}
Ge._diffLimit = 1e5;
Ge._suggestionsLimit = 1e4;
typeof importScripts == "function" && (globalThis.monaco = Ro());
let Hr = !1;
function Ws(e) {
  if (Hr)
    return;
  Hr = !0;
  const t = new Ja((r) => {
    globalThis.postMessage(r);
  }, (r) => new Ge(r, e));
  globalThis.onmessage = (r) => {
    t.onmessage(r.data);
  };
}
globalThis.onmessage = (e) => {
  Hr || Ws(null);
};
/*!-----------------------------------------------------------------------------
 * Copyright (c) Microsoft Corporation. All rights reserved.
 * Version: 0.37.1(20a8d5a651d057aaed7875ad1c1f2ecf13c4e773)
 * Released under the MIT license
 * https://github.com/microsoft/monaco-editor/blob/main/LICENSE.txt
 *-----------------------------------------------------------------------------*/
function hn(e, t) {
  t === void 0 && (t = !1);
  var r = e.length, n = 0, i = "", s = 0, a = 16, o = 0, l = 0, u = 0, f = 0, h = 0;
  function d(b, S) {
    for (var M = 0, A = 0; M < b || !S; ) {
      var C = e.charCodeAt(n);
      if (C >= 48 && C <= 57)
        A = A * 16 + C - 48;
      else if (C >= 65 && C <= 70)
        A = A * 16 + C - 65 + 10;
      else if (C >= 97 && C <= 102)
        A = A * 16 + C - 97 + 10;
      else
        break;
      n++, M++;
    }
    return M < b && (A = -1), A;
  }
  function g(b) {
    n = b, i = "", s = 0, a = 16, h = 0;
  }
  function p() {
    var b = n;
    if (e.charCodeAt(n) === 48)
      n++;
    else
      for (n++; n < e.length && lt(e.charCodeAt(n)); )
        n++;
    if (n < e.length && e.charCodeAt(n) === 46)
      if (n++, n < e.length && lt(e.charCodeAt(n)))
        for (n++; n < e.length && lt(e.charCodeAt(n)); )
          n++;
      else
        return h = 3, e.substring(b, n);
    var S = n;
    if (n < e.length && (e.charCodeAt(n) === 69 || e.charCodeAt(n) === 101))
      if (n++, (n < e.length && e.charCodeAt(n) === 43 || e.charCodeAt(n) === 45) && n++, n < e.length && lt(e.charCodeAt(n))) {
        for (n++; n < e.length && lt(e.charCodeAt(n)); )
          n++;
        S = n;
      } else
        h = 3;
    return e.substring(b, S);
  }
  function m() {
    for (var b = "", S = n; ; ) {
      if (n >= r) {
        b += e.substring(S, n), h = 2;
        break;
      }
      var M = e.charCodeAt(n);
      if (M === 34) {
        b += e.substring(S, n), n++;
        break;
      }
      if (M === 92) {
        if (b += e.substring(S, n), n++, n >= r) {
          h = 2;
          break;
        }
        var A = e.charCodeAt(n++);
        switch (A) {
          case 34:
            b += '"';
            break;
          case 92:
            b += "\\";
            break;
          case 47:
            b += "/";
            break;
          case 98:
            b += "\b";
            break;
          case 102:
            b += "\f";
            break;
          case 110:
            b += `
`;
            break;
          case 114:
            b += "\r";
            break;
          case 116:
            b += "	";
            break;
          case 117:
            var C = d(4, !0);
            C >= 0 ? b += String.fromCharCode(C) : h = 4;
            break;
          default:
            h = 5;
        }
        S = n;
        continue;
      }
      if (M >= 0 && M <= 31)
        if (yt(M)) {
          b += e.substring(S, n), h = 2;
          break;
        } else
          h = 6;
      n++;
    }
    return b;
  }
  function v() {
    if (i = "", h = 0, s = n, l = o, f = u, n >= r)
      return s = r, a = 17;
    var b = e.charCodeAt(n);
    if (yr(b)) {
      do
        n++, i += String.fromCharCode(b), b = e.charCodeAt(n);
      while (yr(b));
      return a = 15;
    }
    if (yt(b))
      return n++, i += String.fromCharCode(b), b === 13 && e.charCodeAt(n) === 10 && (n++, i += `
`), o++, u = n, a = 14;
    switch (b) {
      case 123:
        return n++, a = 1;
      case 125:
        return n++, a = 2;
      case 91:
        return n++, a = 3;
      case 93:
        return n++, a = 4;
      case 58:
        return n++, a = 6;
      case 44:
        return n++, a = 5;
      case 34:
        return n++, i = m(), a = 10;
      case 47:
        var S = n - 1;
        if (e.charCodeAt(n + 1) === 47) {
          for (n += 2; n < r && !yt(e.charCodeAt(n)); )
            n++;
          return i = e.substring(S, n), a = 12;
        }
        if (e.charCodeAt(n + 1) === 42) {
          n += 2;
          for (var M = r - 1, A = !1; n < M; ) {
            var C = e.charCodeAt(n);
            if (C === 42 && e.charCodeAt(n + 1) === 47) {
              n += 2, A = !0;
              break;
            }
            n++, yt(C) && (C === 13 && e.charCodeAt(n) === 10 && n++, o++, u = n);
          }
          return A || (n++, h = 1), i = e.substring(S, n), a = 13;
        }
        return i += String.fromCharCode(b), n++, a = 16;
      case 45:
        if (i += String.fromCharCode(b), n++, n === r || !lt(e.charCodeAt(n)))
          return a = 16;
      case 48:
      case 49:
      case 50:
      case 51:
      case 52:
      case 53:
      case 54:
      case 55:
      case 56:
      case 57:
        return i += p(), a = 11;
      default:
        for (; n < r && y(b); )
          n++, b = e.charCodeAt(n);
        if (s !== n) {
          switch (i = e.substring(s, n), i) {
            case "true":
              return a = 8;
            case "false":
              return a = 9;
            case "null":
              return a = 7;
          }
          return a = 16;
        }
        return i += String.fromCharCode(b), n++, a = 16;
    }
  }
  function y(b) {
    if (yr(b) || yt(b))
      return !1;
    switch (b) {
      case 125:
      case 93:
      case 123:
      case 91:
      case 34:
      case 58:
      case 44:
      case 47:
        return !1;
    }
    return !0;
  }
  function x() {
    var b;
    do
      b = v();
    while (b >= 12 && b <= 15);
    return b;
  }
  return {
    setPosition: g,
    getPosition: function() {
      return n;
    },
    scan: t ? x : v,
    getToken: function() {
      return a;
    },
    getTokenValue: function() {
      return i;
    },
    getTokenOffset: function() {
      return s;
    },
    getTokenLength: function() {
      return n - s;
    },
    getTokenStartLine: function() {
      return l;
    },
    getTokenStartCharacter: function() {
      return s - f;
    },
    getTokenError: function() {
      return h;
    }
  };
}
function yr(e) {
  return e === 32 || e === 9 || e === 11 || e === 12 || e === 160 || e === 5760 || e >= 8192 && e <= 8203 || e === 8239 || e === 8287 || e === 12288 || e === 65279;
}
function yt(e) {
  return e === 10 || e === 13 || e === 8232 || e === 8233;
}
function lt(e) {
  return e >= 48 && e <= 57;
}
function ll(e, t, r) {
  var n, i, s, a, o;
  if (t) {
    for (a = t.offset, o = a + t.length, s = a; s > 0 && !ji(e, s - 1); )
      s--;
    for (var l = o; l < e.length && !ji(e, l); )
      l++;
    i = e.substring(s, l), n = ul(i, r);
  } else
    i = e, n = 0, s = 0, a = 0, o = e.length;
  var u = cl(r, e), f = !1, h = 0, d;
  r.insertSpaces ? d = xr(" ", r.tabSize || 4) : d = "	";
  var g = hn(i, !1), p = !1;
  function m() {
    return u + xr(d, n + h);
  }
  function v() {
    var L = g.scan();
    for (f = !1; L === 15 || L === 14; )
      f = f || L === 14, L = g.scan();
    return p = L === 16 || g.getTokenError() !== 0, L;
  }
  var y = [];
  function x(L, F, D) {
    !p && (!t || F < o && D > a) && e.substring(F, D) !== L && y.push({ offset: F, length: D - F, content: L });
  }
  var b = v();
  if (b !== 17) {
    var S = g.getTokenOffset() + s, M = xr(d, n);
    x(M, s, S);
  }
  for (; b !== 17; ) {
    for (var A = g.getTokenOffset() + g.getTokenLength() + s, C = v(), _ = "", w = !1; !f && (C === 12 || C === 13); ) {
      var N = g.getTokenOffset() + s;
      x(" ", A, N), A = g.getTokenOffset() + g.getTokenLength() + s, w = C === 12, _ = w ? m() : "", C = v();
    }
    if (C === 2)
      b !== 1 && (h--, _ = m());
    else if (C === 4)
      b !== 3 && (h--, _ = m());
    else {
      switch (b) {
        case 3:
        case 1:
          h++, _ = m();
          break;
        case 5:
        case 12:
          _ = m();
          break;
        case 13:
          f ? _ = m() : w || (_ = " ");
          break;
        case 6:
          w || (_ = " ");
          break;
        case 10:
          if (C === 6) {
            w || (_ = "");
            break;
          }
        case 7:
        case 8:
        case 9:
        case 11:
        case 2:
        case 4:
          C === 12 || C === 13 ? w || (_ = " ") : C !== 5 && C !== 17 && (p = !0);
          break;
        case 16:
          p = !0;
          break;
      }
      f && (C === 12 || C === 13) && (_ = m());
    }
    C === 17 && (_ = r.insertFinalNewline ? u : "");
    var k = g.getTokenOffset() + s;
    x(_, A, k), b = C;
  }
  return y;
}
function xr(e, t) {
  for (var r = "", n = 0; n < t; n++)
    r += e;
  return r;
}
function ul(e, t) {
  for (var r = 0, n = 0, i = t.tabSize || 4; r < e.length; ) {
    var s = e.charAt(r);
    if (s === " ")
      n++;
    else if (s === "	")
      n += i;
    else
      break;
    r++;
  }
  return Math.floor(n / i);
}
function cl(e, t) {
  for (var r = 0; r < t.length; r++) {
    var n = t.charAt(r);
    if (n === "\r")
      return r + 1 < t.length && t.charAt(r + 1) === `
` ? `\r
` : "\r";
    if (n === `
`)
      return `
`;
  }
  return e && e.eol || `
`;
}
function ji(e, t) {
  return `\r
`.indexOf(e.charAt(t)) !== -1;
}
var er;
(function(e) {
  e.DEFAULT = {
    allowTrailingComma: !1
  };
})(er || (er = {}));
function fl(e, t, r) {
  t === void 0 && (t = []), r === void 0 && (r = er.DEFAULT);
  var n = null, i = [], s = [];
  function a(l) {
    Array.isArray(i) ? i.push(l) : n !== null && (i[n] = l);
  }
  var o = {
    onObjectBegin: function() {
      var l = {};
      a(l), s.push(i), i = l, n = null;
    },
    onObjectProperty: function(l) {
      n = l;
    },
    onObjectEnd: function() {
      i = s.pop();
    },
    onArrayBegin: function() {
      var l = [];
      a(l), s.push(i), i = l, n = null;
    },
    onArrayEnd: function() {
      i = s.pop();
    },
    onLiteralValue: a,
    onError: function(l, u, f) {
      t.push({ error: l, offset: u, length: f });
    }
  };
  return dl(e, o, r), i[0];
}
function Hs(e) {
  if (!e.parent || !e.parent.children)
    return [];
  var t = Hs(e.parent);
  if (e.parent.type === "property") {
    var r = e.parent.children[0].value;
    t.push(r);
  } else if (e.parent.type === "array") {
    var n = e.parent.children.indexOf(e);
    n !== -1 && t.push(n);
  }
  return t;
}
function zr(e) {
  switch (e.type) {
    case "array":
      return e.children.map(zr);
    case "object":
      for (var t = /* @__PURE__ */ Object.create(null), r = 0, n = e.children; r < n.length; r++) {
        var i = n[r], s = i.children[1];
        s && (t[i.children[0].value] = zr(s));
      }
      return t;
    case "null":
    case "string":
    case "number":
    case "boolean":
      return e.value;
    default:
      return;
  }
}
function hl(e, t, r) {
  return r === void 0 && (r = !1), t >= e.offset && t < e.offset + e.length || r && t === e.offset + e.length;
}
function zs(e, t, r) {
  if (r === void 0 && (r = !1), hl(e, t, r)) {
    var n = e.children;
    if (Array.isArray(n))
      for (var i = 0; i < n.length && n[i].offset <= t; i++) {
        var s = zs(n[i], t, r);
        if (s)
          return s;
      }
    return e;
  }
}
function dl(e, t, r) {
  r === void 0 && (r = er.DEFAULT);
  var n = hn(e, !1);
  function i(w) {
    return w ? function() {
      return w(n.getTokenOffset(), n.getTokenLength(), n.getTokenStartLine(), n.getTokenStartCharacter());
    } : function() {
      return !0;
    };
  }
  function s(w) {
    return w ? function(N) {
      return w(N, n.getTokenOffset(), n.getTokenLength(), n.getTokenStartLine(), n.getTokenStartCharacter());
    } : function() {
      return !0;
    };
  }
  var a = i(t.onObjectBegin), o = s(t.onObjectProperty), l = i(t.onObjectEnd), u = i(t.onArrayBegin), f = i(t.onArrayEnd), h = s(t.onLiteralValue), d = s(t.onSeparator), g = i(t.onComment), p = s(t.onError), m = r && r.disallowComments, v = r && r.allowTrailingComma;
  function y() {
    for (; ; ) {
      var w = n.scan();
      switch (n.getTokenError()) {
        case 4:
          x(14);
          break;
        case 5:
          x(15);
          break;
        case 3:
          x(13);
          break;
        case 1:
          m || x(11);
          break;
        case 2:
          x(12);
          break;
        case 6:
          x(16);
          break;
      }
      switch (w) {
        case 12:
        case 13:
          m ? x(10) : g();
          break;
        case 16:
          x(1);
          break;
        case 15:
        case 14:
          break;
        default:
          return w;
      }
    }
  }
  function x(w, N, k) {
    if (N === void 0 && (N = []), k === void 0 && (k = []), p(w), N.length + k.length > 0)
      for (var L = n.getToken(); L !== 17; ) {
        if (N.indexOf(L) !== -1) {
          y();
          break;
        } else if (k.indexOf(L) !== -1)
          break;
        L = y();
      }
  }
  function b(w) {
    var N = n.getTokenValue();
    return w ? h(N) : o(N), y(), !0;
  }
  function S() {
    switch (n.getToken()) {
      case 11:
        var w = n.getTokenValue(), N = Number(w);
        isNaN(N) && (x(2), N = 0), h(N);
        break;
      case 7:
        h(null);
        break;
      case 8:
        h(!0);
        break;
      case 9:
        h(!1);
        break;
      default:
        return !1;
    }
    return y(), !0;
  }
  function M() {
    return n.getToken() !== 10 ? (x(3, [], [2, 5]), !1) : (b(!1), n.getToken() === 6 ? (d(":"), y(), _() || x(4, [], [2, 5])) : x(5, [], [2, 5]), !0);
  }
  function A() {
    a(), y();
    for (var w = !1; n.getToken() !== 2 && n.getToken() !== 17; ) {
      if (n.getToken() === 5) {
        if (w || x(4, [], []), d(","), y(), n.getToken() === 2 && v)
          break;
      } else
        w && x(6, [], []);
      M() || x(4, [], [2, 5]), w = !0;
    }
    return l(), n.getToken() !== 2 ? x(7, [2], []) : y(), !0;
  }
  function C() {
    u(), y();
    for (var w = !1; n.getToken() !== 4 && n.getToken() !== 17; ) {
      if (n.getToken() === 5) {
        if (w || x(4, [], []), d(","), y(), n.getToken() === 4 && v)
          break;
      } else
        w && x(6, [], []);
      _() || x(4, [], [4, 5]), w = !0;
    }
    return f(), n.getToken() !== 4 ? x(8, [4], []) : y(), !0;
  }
  function _() {
    switch (n.getToken()) {
      case 3:
        return C();
      case 1:
        return A();
      case 10:
        return b(!0);
      default:
        return S();
    }
  }
  return y(), n.getToken() === 17 ? r.allowEmptyContent ? !0 : (x(4, [], []), !1) : _() ? (n.getToken() !== 17 && x(9, [], []), !0) : (x(4, [], []), !1);
}
var ht = hn, gl = fl, ml = zs, pl = Hs, vl = zr;
function bl(e, t, r) {
  return ll(e, t, r);
}
function _t(e, t) {
  if (e === t)
    return !0;
  if (e == null || t === null || t === void 0 || typeof e != typeof t || typeof e != "object" || Array.isArray(e) !== Array.isArray(t))
    return !1;
  var r, n;
  if (Array.isArray(e)) {
    if (e.length !== t.length)
      return !1;
    for (r = 0; r < e.length; r++)
      if (!_t(e[r], t[r]))
        return !1;
  } else {
    var i = [];
    for (n in e)
      i.push(n);
    i.sort();
    var s = [];
    for (n in t)
      s.push(n);
    if (s.sort(), !_t(i, s))
      return !1;
    for (r = 0; r < i.length; r++)
      if (!_t(e[i[r]], t[i[r]]))
        return !1;
  }
  return !0;
}
function ge(e) {
  return typeof e == "number";
}
function Ie(e) {
  return typeof e < "u";
}
function Te(e) {
  return typeof e == "boolean";
}
function yl(e) {
  return typeof e == "string";
}
function xl(e, t) {
  if (e.length < t.length)
    return !1;
  for (var r = 0; r < t.length; r++)
    if (e[r] !== t[r])
      return !1;
  return !0;
}
function Lt(e, t) {
  var r = e.length - t.length;
  return r > 0 ? e.lastIndexOf(t) === r : r === 0 ? e === t : !1;
}
function tr(e) {
  var t = "";
  xl(e, "(?i)") && (e = e.substring(4), t = "i");
  try {
    return new RegExp(e, t + "u");
  } catch {
    try {
      return new RegExp(e, t);
    } catch {
      return;
    }
  }
}
var Ui;
(function(e) {
  e.MIN_VALUE = -2147483648, e.MAX_VALUE = 2147483647;
})(Ui || (Ui = {}));
var rr;
(function(e) {
  e.MIN_VALUE = 0, e.MAX_VALUE = 2147483647;
})(rr || (rr = {}));
var Se;
(function(e) {
  function t(n, i) {
    return n === Number.MAX_VALUE && (n = rr.MAX_VALUE), i === Number.MAX_VALUE && (i = rr.MAX_VALUE), { line: n, character: i };
  }
  e.create = t;
  function r(n) {
    var i = n;
    return E.objectLiteral(i) && E.uinteger(i.line) && E.uinteger(i.character);
  }
  e.is = r;
})(Se || (Se = {}));
var G;
(function(e) {
  function t(n, i, s, a) {
    if (E.uinteger(n) && E.uinteger(i) && E.uinteger(s) && E.uinteger(a))
      return { start: Se.create(n, i), end: Se.create(s, a) };
    if (Se.is(n) && Se.is(i))
      return { start: n, end: i };
    throw new Error("Range#create called with invalid arguments[" + n + ", " + i + ", " + s + ", " + a + "]");
  }
  e.create = t;
  function r(n) {
    var i = n;
    return E.objectLiteral(i) && Se.is(i.start) && Se.is(i.end);
  }
  e.is = r;
})(G || (G = {}));
var Ct;
(function(e) {
  function t(n, i) {
    return { uri: n, range: i };
  }
  e.create = t;
  function r(n) {
    var i = n;
    return E.defined(i) && G.is(i.range) && (E.string(i.uri) || E.undefined(i.uri));
  }
  e.is = r;
})(Ct || (Ct = {}));
var Bi;
(function(e) {
  function t(n, i, s, a) {
    return { targetUri: n, targetRange: i, targetSelectionRange: s, originSelectionRange: a };
  }
  e.create = t;
  function r(n) {
    var i = n;
    return E.defined(i) && G.is(i.targetRange) && E.string(i.targetUri) && (G.is(i.targetSelectionRange) || E.undefined(i.targetSelectionRange)) && (G.is(i.originSelectionRange) || E.undefined(i.originSelectionRange));
  }
  e.is = r;
})(Bi || (Bi = {}));
var Gr;
(function(e) {
  function t(n, i, s, a) {
    return {
      red: n,
      green: i,
      blue: s,
      alpha: a
    };
  }
  e.create = t;
  function r(n) {
    var i = n;
    return E.numberRange(i.red, 0, 1) && E.numberRange(i.green, 0, 1) && E.numberRange(i.blue, 0, 1) && E.numberRange(i.alpha, 0, 1);
  }
  e.is = r;
})(Gr || (Gr = {}));
var $i;
(function(e) {
  function t(n, i) {
    return {
      range: n,
      color: i
    };
  }
  e.create = t;
  function r(n) {
    var i = n;
    return G.is(i.range) && Gr.is(i.color);
  }
  e.is = r;
})($i || ($i = {}));
var qi;
(function(e) {
  function t(n, i, s) {
    return {
      label: n,
      textEdit: i,
      additionalTextEdits: s
    };
  }
  e.create = t;
  function r(n) {
    var i = n;
    return E.string(i.label) && (E.undefined(i.textEdit) || Ne.is(i)) && (E.undefined(i.additionalTextEdits) || E.typedArray(i.additionalTextEdits, Ne.is));
  }
  e.is = r;
})(qi || (qi = {}));
var St;
(function(e) {
  e.Comment = "comment", e.Imports = "imports", e.Region = "region";
})(St || (St = {}));
var Wi;
(function(e) {
  function t(n, i, s, a, o) {
    var l = {
      startLine: n,
      endLine: i
    };
    return E.defined(s) && (l.startCharacter = s), E.defined(a) && (l.endCharacter = a), E.defined(o) && (l.kind = o), l;
  }
  e.create = t;
  function r(n) {
    var i = n;
    return E.uinteger(i.startLine) && E.uinteger(i.startLine) && (E.undefined(i.startCharacter) || E.uinteger(i.startCharacter)) && (E.undefined(i.endCharacter) || E.uinteger(i.endCharacter)) && (E.undefined(i.kind) || E.string(i.kind));
  }
  e.is = r;
})(Wi || (Wi = {}));
var Jr;
(function(e) {
  function t(n, i) {
    return {
      location: n,
      message: i
    };
  }
  e.create = t;
  function r(n) {
    var i = n;
    return E.defined(i) && Ct.is(i.location) && E.string(i.message);
  }
  e.is = r;
})(Jr || (Jr = {}));
var ve;
(function(e) {
  e.Error = 1, e.Warning = 2, e.Information = 3, e.Hint = 4;
})(ve || (ve = {}));
var Hi;
(function(e) {
  e.Unnecessary = 1, e.Deprecated = 2;
})(Hi || (Hi = {}));
var zi;
(function(e) {
  function t(r) {
    var n = r;
    return n != null && E.string(n.href);
  }
  e.is = t;
})(zi || (zi = {}));
var De;
(function(e) {
  function t(n, i, s, a, o, l) {
    var u = { range: n, message: i };
    return E.defined(s) && (u.severity = s), E.defined(a) && (u.code = a), E.defined(o) && (u.source = o), E.defined(l) && (u.relatedInformation = l), u;
  }
  e.create = t;
  function r(n) {
    var i, s = n;
    return E.defined(s) && G.is(s.range) && E.string(s.message) && (E.number(s.severity) || E.undefined(s.severity)) && (E.integer(s.code) || E.string(s.code) || E.undefined(s.code)) && (E.undefined(s.codeDescription) || E.string((i = s.codeDescription) === null || i === void 0 ? void 0 : i.href)) && (E.string(s.source) || E.undefined(s.source)) && (E.undefined(s.relatedInformation) || E.typedArray(s.relatedInformation, Jr.is));
  }
  e.is = r;
})(De || (De = {}));
var kt;
(function(e) {
  function t(n, i) {
    for (var s = [], a = 2; a < arguments.length; a++)
      s[a - 2] = arguments[a];
    var o = { title: n, command: i };
    return E.defined(s) && s.length > 0 && (o.arguments = s), o;
  }
  e.create = t;
  function r(n) {
    var i = n;
    return E.defined(i) && E.string(i.title) && E.string(i.command);
  }
  e.is = r;
})(kt || (kt = {}));
var Ne;
(function(e) {
  function t(s, a) {
    return { range: s, newText: a };
  }
  e.replace = t;
  function r(s, a) {
    return { range: { start: s, end: s }, newText: a };
  }
  e.insert = r;
  function n(s) {
    return { range: s, newText: "" };
  }
  e.del = n;
  function i(s) {
    var a = s;
    return E.objectLiteral(a) && E.string(a.newText) && G.is(a.range);
  }
  e.is = i;
})(Ne || (Ne = {}));
var dt;
(function(e) {
  function t(n, i, s) {
    var a = { label: n };
    return i !== void 0 && (a.needsConfirmation = i), s !== void 0 && (a.description = s), a;
  }
  e.create = t;
  function r(n) {
    var i = n;
    return i !== void 0 && E.objectLiteral(i) && E.string(i.label) && (E.boolean(i.needsConfirmation) || i.needsConfirmation === void 0) && (E.string(i.description) || i.description === void 0);
  }
  e.is = r;
})(dt || (dt = {}));
var oe;
(function(e) {
  function t(r) {
    var n = r;
    return typeof n == "string";
  }
  e.is = t;
})(oe || (oe = {}));
var He;
(function(e) {
  function t(s, a, o) {
    return { range: s, newText: a, annotationId: o };
  }
  e.replace = t;
  function r(s, a, o) {
    return { range: { start: s, end: s }, newText: a, annotationId: o };
  }
  e.insert = r;
  function n(s, a) {
    return { range: s, newText: "", annotationId: a };
  }
  e.del = n;
  function i(s) {
    var a = s;
    return Ne.is(a) && (dt.is(a.annotationId) || oe.is(a.annotationId));
  }
  e.is = i;
})(He || (He = {}));
var nr;
(function(e) {
  function t(n, i) {
    return { textDocument: n, edits: i };
  }
  e.create = t;
  function r(n) {
    var i = n;
    return E.defined(i) && ir.is(i.textDocument) && Array.isArray(i.edits);
  }
  e.is = r;
})(nr || (nr = {}));
var Et;
(function(e) {
  function t(n, i, s) {
    var a = {
      kind: "create",
      uri: n
    };
    return i !== void 0 && (i.overwrite !== void 0 || i.ignoreIfExists !== void 0) && (a.options = i), s !== void 0 && (a.annotationId = s), a;
  }
  e.create = t;
  function r(n) {
    var i = n;
    return i && i.kind === "create" && E.string(i.uri) && (i.options === void 0 || (i.options.overwrite === void 0 || E.boolean(i.options.overwrite)) && (i.options.ignoreIfExists === void 0 || E.boolean(i.options.ignoreIfExists))) && (i.annotationId === void 0 || oe.is(i.annotationId));
  }
  e.is = r;
})(Et || (Et = {}));
var Mt;
(function(e) {
  function t(n, i, s, a) {
    var o = {
      kind: "rename",
      oldUri: n,
      newUri: i
    };
    return s !== void 0 && (s.overwrite !== void 0 || s.ignoreIfExists !== void 0) && (o.options = s), a !== void 0 && (o.annotationId = a), o;
  }
  e.create = t;
  function r(n) {
    var i = n;
    return i && i.kind === "rename" && E.string(i.oldUri) && E.string(i.newUri) && (i.options === void 0 || (i.options.overwrite === void 0 || E.boolean(i.options.overwrite)) && (i.options.ignoreIfExists === void 0 || E.boolean(i.options.ignoreIfExists))) && (i.annotationId === void 0 || oe.is(i.annotationId));
  }
  e.is = r;
})(Mt || (Mt = {}));
var Tt;
(function(e) {
  function t(n, i, s) {
    var a = {
      kind: "delete",
      uri: n
    };
    return i !== void 0 && (i.recursive !== void 0 || i.ignoreIfNotExists !== void 0) && (a.options = i), s !== void 0 && (a.annotationId = s), a;
  }
  e.create = t;
  function r(n) {
    var i = n;
    return i && i.kind === "delete" && E.string(i.uri) && (i.options === void 0 || (i.options.recursive === void 0 || E.boolean(i.options.recursive)) && (i.options.ignoreIfNotExists === void 0 || E.boolean(i.options.ignoreIfNotExists))) && (i.annotationId === void 0 || oe.is(i.annotationId));
  }
  e.is = r;
})(Tt || (Tt = {}));
var Xr;
(function(e) {
  function t(r) {
    var n = r;
    return n && (n.changes !== void 0 || n.documentChanges !== void 0) && (n.documentChanges === void 0 || n.documentChanges.every(function(i) {
      return E.string(i.kind) ? Et.is(i) || Mt.is(i) || Tt.is(i) : nr.is(i);
    }));
  }
  e.is = t;
})(Xr || (Xr = {}));
var Vt = function() {
  function e(t, r) {
    this.edits = t, this.changeAnnotations = r;
  }
  return e.prototype.insert = function(t, r, n) {
    var i, s;
    if (n === void 0 ? i = Ne.insert(t, r) : oe.is(n) ? (s = n, i = He.insert(t, r, n)) : (this.assertChangeAnnotations(this.changeAnnotations), s = this.changeAnnotations.manage(n), i = He.insert(t, r, s)), this.edits.push(i), s !== void 0)
      return s;
  }, e.prototype.replace = function(t, r, n) {
    var i, s;
    if (n === void 0 ? i = Ne.replace(t, r) : oe.is(n) ? (s = n, i = He.replace(t, r, n)) : (this.assertChangeAnnotations(this.changeAnnotations), s = this.changeAnnotations.manage(n), i = He.replace(t, r, s)), this.edits.push(i), s !== void 0)
      return s;
  }, e.prototype.delete = function(t, r) {
    var n, i;
    if (r === void 0 ? n = Ne.del(t) : oe.is(r) ? (i = r, n = He.del(t, r)) : (this.assertChangeAnnotations(this.changeAnnotations), i = this.changeAnnotations.manage(r), n = He.del(t, i)), this.edits.push(n), i !== void 0)
      return i;
  }, e.prototype.add = function(t) {
    this.edits.push(t);
  }, e.prototype.all = function() {
    return this.edits;
  }, e.prototype.clear = function() {
    this.edits.splice(0, this.edits.length);
  }, e.prototype.assertChangeAnnotations = function(t) {
    if (t === void 0)
      throw new Error("Text edit change is not configured to manage change annotations.");
  }, e;
}(), Gi = function() {
  function e(t) {
    this._annotations = t === void 0 ? /* @__PURE__ */ Object.create(null) : t, this._counter = 0, this._size = 0;
  }
  return e.prototype.all = function() {
    return this._annotations;
  }, Object.defineProperty(e.prototype, "size", {
    get: function() {
      return this._size;
    },
    enumerable: !1,
    configurable: !0
  }), e.prototype.manage = function(t, r) {
    var n;
    if (oe.is(t) ? n = t : (n = this.nextId(), r = t), this._annotations[n] !== void 0)
      throw new Error("Id " + n + " is already in use.");
    if (r === void 0)
      throw new Error("No annotation provided for id " + n);
    return this._annotations[n] = r, this._size++, n;
  }, e.prototype.nextId = function() {
    return this._counter++, this._counter.toString();
  }, e;
}();
(function() {
  function e(t) {
    var r = this;
    this._textEditChanges = /* @__PURE__ */ Object.create(null), t !== void 0 ? (this._workspaceEdit = t, t.documentChanges ? (this._changeAnnotations = new Gi(t.changeAnnotations), t.changeAnnotations = this._changeAnnotations.all(), t.documentChanges.forEach(function(n) {
      if (nr.is(n)) {
        var i = new Vt(n.edits, r._changeAnnotations);
        r._textEditChanges[n.textDocument.uri] = i;
      }
    })) : t.changes && Object.keys(t.changes).forEach(function(n) {
      var i = new Vt(t.changes[n]);
      r._textEditChanges[n] = i;
    })) : this._workspaceEdit = {};
  }
  return Object.defineProperty(e.prototype, "edit", {
    get: function() {
      return this.initDocumentChanges(), this._changeAnnotations !== void 0 && (this._changeAnnotations.size === 0 ? this._workspaceEdit.changeAnnotations = void 0 : this._workspaceEdit.changeAnnotations = this._changeAnnotations.all()), this._workspaceEdit;
    },
    enumerable: !1,
    configurable: !0
  }), e.prototype.getTextEditChange = function(t) {
    if (ir.is(t)) {
      if (this.initDocumentChanges(), this._workspaceEdit.documentChanges === void 0)
        throw new Error("Workspace edit is not configured for document changes.");
      var r = { uri: t.uri, version: t.version }, n = this._textEditChanges[r.uri];
      if (!n) {
        var i = [], s = {
          textDocument: r,
          edits: i
        };
        this._workspaceEdit.documentChanges.push(s), n = new Vt(i, this._changeAnnotations), this._textEditChanges[r.uri] = n;
      }
      return n;
    } else {
      if (this.initChanges(), this._workspaceEdit.changes === void 0)
        throw new Error("Workspace edit is not configured for normal text edit changes.");
      var n = this._textEditChanges[t];
      if (!n) {
        var i = [];
        this._workspaceEdit.changes[t] = i, n = new Vt(i), this._textEditChanges[t] = n;
      }
      return n;
    }
  }, e.prototype.initDocumentChanges = function() {
    this._workspaceEdit.documentChanges === void 0 && this._workspaceEdit.changes === void 0 && (this._changeAnnotations = new Gi(), this._workspaceEdit.documentChanges = [], this._workspaceEdit.changeAnnotations = this._changeAnnotations.all());
  }, e.prototype.initChanges = function() {
    this._workspaceEdit.documentChanges === void 0 && this._workspaceEdit.changes === void 0 && (this._workspaceEdit.changes = /* @__PURE__ */ Object.create(null));
  }, e.prototype.createFile = function(t, r, n) {
    if (this.initDocumentChanges(), this._workspaceEdit.documentChanges === void 0)
      throw new Error("Workspace edit is not configured for document changes.");
    var i;
    dt.is(r) || oe.is(r) ? i = r : n = r;
    var s, a;
    if (i === void 0 ? s = Et.create(t, n) : (a = oe.is(i) ? i : this._changeAnnotations.manage(i), s = Et.create(t, n, a)), this._workspaceEdit.documentChanges.push(s), a !== void 0)
      return a;
  }, e.prototype.renameFile = function(t, r, n, i) {
    if (this.initDocumentChanges(), this._workspaceEdit.documentChanges === void 0)
      throw new Error("Workspace edit is not configured for document changes.");
    var s;
    dt.is(n) || oe.is(n) ? s = n : i = n;
    var a, o;
    if (s === void 0 ? a = Mt.create(t, r, i) : (o = oe.is(s) ? s : this._changeAnnotations.manage(s), a = Mt.create(t, r, i, o)), this._workspaceEdit.documentChanges.push(a), o !== void 0)
      return o;
  }, e.prototype.deleteFile = function(t, r, n) {
    if (this.initDocumentChanges(), this._workspaceEdit.documentChanges === void 0)
      throw new Error("Workspace edit is not configured for document changes.");
    var i;
    dt.is(r) || oe.is(r) ? i = r : n = r;
    var s, a;
    if (i === void 0 ? s = Tt.create(t, n) : (a = oe.is(i) ? i : this._changeAnnotations.manage(i), s = Tt.create(t, n, a)), this._workspaceEdit.documentChanges.push(s), a !== void 0)
      return a;
  }, e;
})();
var Ji;
(function(e) {
  function t(n) {
    return { uri: n };
  }
  e.create = t;
  function r(n) {
    var i = n;
    return E.defined(i) && E.string(i.uri);
  }
  e.is = r;
})(Ji || (Ji = {}));
var Xi;
(function(e) {
  function t(n, i) {
    return { uri: n, version: i };
  }
  e.create = t;
  function r(n) {
    var i = n;
    return E.defined(i) && E.string(i.uri) && E.integer(i.version);
  }
  e.is = r;
})(Xi || (Xi = {}));
var ir;
(function(e) {
  function t(n, i) {
    return { uri: n, version: i };
  }
  e.create = t;
  function r(n) {
    var i = n;
    return E.defined(i) && E.string(i.uri) && (i.version === null || E.integer(i.version));
  }
  e.is = r;
})(ir || (ir = {}));
var Zi;
(function(e) {
  function t(n, i, s, a) {
    return { uri: n, languageId: i, version: s, text: a };
  }
  e.create = t;
  function r(n) {
    var i = n;
    return E.defined(i) && E.string(i.uri) && E.string(i.languageId) && E.integer(i.version) && E.string(i.text);
  }
  e.is = r;
})(Zi || (Zi = {}));
var Oe;
(function(e) {
  e.PlainText = "plaintext", e.Markdown = "markdown";
})(Oe || (Oe = {}));
(function(e) {
  function t(r) {
    var n = r;
    return n === e.PlainText || n === e.Markdown;
  }
  e.is = t;
})(Oe || (Oe = {}));
var Zr;
(function(e) {
  function t(r) {
    var n = r;
    return E.objectLiteral(r) && Oe.is(n.kind) && E.string(n.value);
  }
  e.is = t;
})(Zr || (Zr = {}));
var pe;
(function(e) {
  e.Text = 1, e.Method = 2, e.Function = 3, e.Constructor = 4, e.Field = 5, e.Variable = 6, e.Class = 7, e.Interface = 8, e.Module = 9, e.Property = 10, e.Unit = 11, e.Value = 12, e.Enum = 13, e.Keyword = 14, e.Snippet = 15, e.Color = 16, e.File = 17, e.Reference = 18, e.Folder = 19, e.EnumMember = 20, e.Constant = 21, e.Struct = 22, e.Event = 23, e.Operator = 24, e.TypeParameter = 25;
})(pe || (pe = {}));
var te;
(function(e) {
  e.PlainText = 1, e.Snippet = 2;
})(te || (te = {}));
var Qi;
(function(e) {
  e.Deprecated = 1;
})(Qi || (Qi = {}));
var Yi;
(function(e) {
  function t(n, i, s) {
    return { newText: n, insert: i, replace: s };
  }
  e.create = t;
  function r(n) {
    var i = n;
    return i && E.string(i.newText) && G.is(i.insert) && G.is(i.replace);
  }
  e.is = r;
})(Yi || (Yi = {}));
var Ki;
(function(e) {
  e.asIs = 1, e.adjustIndentation = 2;
})(Ki || (Ki = {}));
var Qr;
(function(e) {
  function t(r) {
    return { label: r };
  }
  e.create = t;
})(Qr || (Qr = {}));
var es;
(function(e) {
  function t(r, n) {
    return { items: r || [], isIncomplete: !!n };
  }
  e.create = t;
})(es || (es = {}));
var sr;
(function(e) {
  function t(n) {
    return n.replace(/[\\`*_{}[\]()#+\-.!]/g, "\\$&");
  }
  e.fromPlainText = t;
  function r(n) {
    var i = n;
    return E.string(i) || E.objectLiteral(i) && E.string(i.language) && E.string(i.value);
  }
  e.is = r;
})(sr || (sr = {}));
var ts;
(function(e) {
  function t(r) {
    var n = r;
    return !!n && E.objectLiteral(n) && (Zr.is(n.contents) || sr.is(n.contents) || E.typedArray(n.contents, sr.is)) && (r.range === void 0 || G.is(r.range));
  }
  e.is = t;
})(ts || (ts = {}));
var rs;
(function(e) {
  function t(r, n) {
    return n ? { label: r, documentation: n } : { label: r };
  }
  e.create = t;
})(rs || (rs = {}));
var ns;
(function(e) {
  function t(r, n) {
    for (var i = [], s = 2; s < arguments.length; s++)
      i[s - 2] = arguments[s];
    var a = { label: r };
    return E.defined(n) && (a.documentation = n), E.defined(i) ? a.parameters = i : a.parameters = [], a;
  }
  e.create = t;
})(ns || (ns = {}));
var is;
(function(e) {
  e.Text = 1, e.Read = 2, e.Write = 3;
})(is || (is = {}));
var ss;
(function(e) {
  function t(r, n) {
    var i = { range: r };
    return E.number(n) && (i.kind = n), i;
  }
  e.create = t;
})(ss || (ss = {}));
var Ee;
(function(e) {
  e.File = 1, e.Module = 2, e.Namespace = 3, e.Package = 4, e.Class = 5, e.Method = 6, e.Property = 7, e.Field = 8, e.Constructor = 9, e.Enum = 10, e.Interface = 11, e.Function = 12, e.Variable = 13, e.Constant = 14, e.String = 15, e.Number = 16, e.Boolean = 17, e.Array = 18, e.Object = 19, e.Key = 20, e.Null = 21, e.EnumMember = 22, e.Struct = 23, e.Event = 24, e.Operator = 25, e.TypeParameter = 26;
})(Ee || (Ee = {}));
var as;
(function(e) {
  e.Deprecated = 1;
})(as || (as = {}));
var os;
(function(e) {
  function t(r, n, i, s, a) {
    var o = {
      name: r,
      kind: n,
      location: { uri: s, range: i }
    };
    return a && (o.containerName = a), o;
  }
  e.create = t;
})(os || (os = {}));
var ls;
(function(e) {
  function t(n, i, s, a, o, l) {
    var u = {
      name: n,
      detail: i,
      kind: s,
      range: a,
      selectionRange: o
    };
    return l !== void 0 && (u.children = l), u;
  }
  e.create = t;
  function r(n) {
    var i = n;
    return i && E.string(i.name) && E.number(i.kind) && G.is(i.range) && G.is(i.selectionRange) && (i.detail === void 0 || E.string(i.detail)) && (i.deprecated === void 0 || E.boolean(i.deprecated)) && (i.children === void 0 || Array.isArray(i.children)) && (i.tags === void 0 || Array.isArray(i.tags));
  }
  e.is = r;
})(ls || (ls = {}));
var us;
(function(e) {
  e.Empty = "", e.QuickFix = "quickfix", e.Refactor = "refactor", e.RefactorExtract = "refactor.extract", e.RefactorInline = "refactor.inline", e.RefactorRewrite = "refactor.rewrite", e.Source = "source", e.SourceOrganizeImports = "source.organizeImports", e.SourceFixAll = "source.fixAll";
})(us || (us = {}));
var cs;
(function(e) {
  function t(n, i) {
    var s = { diagnostics: n };
    return i != null && (s.only = i), s;
  }
  e.create = t;
  function r(n) {
    var i = n;
    return E.defined(i) && E.typedArray(i.diagnostics, De.is) && (i.only === void 0 || E.typedArray(i.only, E.string));
  }
  e.is = r;
})(cs || (cs = {}));
var fs;
(function(e) {
  function t(n, i, s) {
    var a = { title: n }, o = !0;
    return typeof i == "string" ? (o = !1, a.kind = i) : kt.is(i) ? a.command = i : a.edit = i, o && s !== void 0 && (a.kind = s), a;
  }
  e.create = t;
  function r(n) {
    var i = n;
    return i && E.string(i.title) && (i.diagnostics === void 0 || E.typedArray(i.diagnostics, De.is)) && (i.kind === void 0 || E.string(i.kind)) && (i.edit !== void 0 || i.command !== void 0) && (i.command === void 0 || kt.is(i.command)) && (i.isPreferred === void 0 || E.boolean(i.isPreferred)) && (i.edit === void 0 || Xr.is(i.edit));
  }
  e.is = r;
})(fs || (fs = {}));
var hs;
(function(e) {
  function t(n, i) {
    var s = { range: n };
    return E.defined(i) && (s.data = i), s;
  }
  e.create = t;
  function r(n) {
    var i = n;
    return E.defined(i) && G.is(i.range) && (E.undefined(i.command) || kt.is(i.command));
  }
  e.is = r;
})(hs || (hs = {}));
var ds;
(function(e) {
  function t(n, i) {
    return { tabSize: n, insertSpaces: i };
  }
  e.create = t;
  function r(n) {
    var i = n;
    return E.defined(i) && E.uinteger(i.tabSize) && E.boolean(i.insertSpaces);
  }
  e.is = r;
})(ds || (ds = {}));
var gs;
(function(e) {
  function t(n, i, s) {
    return { range: n, target: i, data: s };
  }
  e.create = t;
  function r(n) {
    var i = n;
    return E.defined(i) && G.is(i.range) && (E.undefined(i.target) || E.string(i.target));
  }
  e.is = r;
})(gs || (gs = {}));
var ar;
(function(e) {
  function t(n, i) {
    return { range: n, parent: i };
  }
  e.create = t;
  function r(n) {
    var i = n;
    return i !== void 0 && G.is(i.range) && (i.parent === void 0 || e.is(i.parent));
  }
  e.is = r;
})(ar || (ar = {}));
var ms;
(function(e) {
  function t(s, a, o, l) {
    return new wl(s, a, o, l);
  }
  e.create = t;
  function r(s) {
    var a = s;
    return !!(E.defined(a) && E.string(a.uri) && (E.undefined(a.languageId) || E.string(a.languageId)) && E.uinteger(a.lineCount) && E.func(a.getText) && E.func(a.positionAt) && E.func(a.offsetAt));
  }
  e.is = r;
  function n(s, a) {
    for (var o = s.getText(), l = i(a, function(p, m) {
      var v = p.range.start.line - m.range.start.line;
      return v === 0 ? p.range.start.character - m.range.start.character : v;
    }), u = o.length, f = l.length - 1; f >= 0; f--) {
      var h = l[f], d = s.offsetAt(h.range.start), g = s.offsetAt(h.range.end);
      if (g <= u)
        o = o.substring(0, d) + h.newText + o.substring(g, o.length);
      else
        throw new Error("Overlapping edit");
      u = d;
    }
    return o;
  }
  e.applyEdits = n;
  function i(s, a) {
    if (s.length <= 1)
      return s;
    var o = s.length / 2 | 0, l = s.slice(0, o), u = s.slice(o);
    i(l, a), i(u, a);
    for (var f = 0, h = 0, d = 0; f < l.length && h < u.length; ) {
      var g = a(l[f], u[h]);
      g <= 0 ? s[d++] = l[f++] : s[d++] = u[h++];
    }
    for (; f < l.length; )
      s[d++] = l[f++];
    for (; h < u.length; )
      s[d++] = u[h++];
    return s;
  }
})(ms || (ms = {}));
var wl = function() {
  function e(t, r, n, i) {
    this._uri = t, this._languageId = r, this._version = n, this._content = i, this._lineOffsets = void 0;
  }
  return Object.defineProperty(e.prototype, "uri", {
    get: function() {
      return this._uri;
    },
    enumerable: !1,
    configurable: !0
  }), Object.defineProperty(e.prototype, "languageId", {
    get: function() {
      return this._languageId;
    },
    enumerable: !1,
    configurable: !0
  }), Object.defineProperty(e.prototype, "version", {
    get: function() {
      return this._version;
    },
    enumerable: !1,
    configurable: !0
  }), e.prototype.getText = function(t) {
    if (t) {
      var r = this.offsetAt(t.start), n = this.offsetAt(t.end);
      return this._content.substring(r, n);
    }
    return this._content;
  }, e.prototype.update = function(t, r) {
    this._content = t.text, this._version = r, this._lineOffsets = void 0;
  }, e.prototype.getLineOffsets = function() {
    if (this._lineOffsets === void 0) {
      for (var t = [], r = this._content, n = !0, i = 0; i < r.length; i++) {
        n && (t.push(i), n = !1);
        var s = r.charAt(i);
        n = s === "\r" || s === `
`, s === "\r" && i + 1 < r.length && r.charAt(i + 1) === `
` && i++;
      }
      n && r.length > 0 && t.push(r.length), this._lineOffsets = t;
    }
    return this._lineOffsets;
  }, e.prototype.positionAt = function(t) {
    t = Math.max(Math.min(t, this._content.length), 0);
    var r = this.getLineOffsets(), n = 0, i = r.length;
    if (i === 0)
      return Se.create(0, t);
    for (; n < i; ) {
      var s = Math.floor((n + i) / 2);
      r[s] > t ? i = s : n = s + 1;
    }
    var a = n - 1;
    return Se.create(a, t - r[a]);
  }, e.prototype.offsetAt = function(t) {
    var r = this.getLineOffsets();
    if (t.line >= r.length)
      return this._content.length;
    if (t.line < 0)
      return 0;
    var n = r[t.line], i = t.line + 1 < r.length ? r[t.line + 1] : this._content.length;
    return Math.max(Math.min(n + t.character, i), n);
  }, Object.defineProperty(e.prototype, "lineCount", {
    get: function() {
      return this.getLineOffsets().length;
    },
    enumerable: !1,
    configurable: !0
  }), e;
}(), E;
(function(e) {
  var t = Object.prototype.toString;
  function r(g) {
    return typeof g < "u";
  }
  e.defined = r;
  function n(g) {
    return typeof g > "u";
  }
  e.undefined = n;
  function i(g) {
    return g === !0 || g === !1;
  }
  e.boolean = i;
  function s(g) {
    return t.call(g) === "[object String]";
  }
  e.string = s;
  function a(g) {
    return t.call(g) === "[object Number]";
  }
  e.number = a;
  function o(g, p, m) {
    return t.call(g) === "[object Number]" && p <= g && g <= m;
  }
  e.numberRange = o;
  function l(g) {
    return t.call(g) === "[object Number]" && -2147483648 <= g && g <= 2147483647;
  }
  e.integer = l;
  function u(g) {
    return t.call(g) === "[object Number]" && 0 <= g && g <= 2147483647;
  }
  e.uinteger = u;
  function f(g) {
    return t.call(g) === "[object Function]";
  }
  e.func = f;
  function h(g) {
    return g !== null && typeof g == "object";
  }
  e.objectLiteral = h;
  function d(g, p) {
    return Array.isArray(g) && g.every(p);
  }
  e.typedArray = d;
})(E || (E = {}));
var or = class {
  constructor(e, t, r, n) {
    this._uri = e, this._languageId = t, this._version = r, this._content = n, this._lineOffsets = void 0;
  }
  get uri() {
    return this._uri;
  }
  get languageId() {
    return this._languageId;
  }
  get version() {
    return this._version;
  }
  getText(e) {
    if (e) {
      const t = this.offsetAt(e.start), r = this.offsetAt(e.end);
      return this._content.substring(t, r);
    }
    return this._content;
  }
  update(e, t) {
    for (let r of e)
      if (or.isIncremental(r)) {
        const n = Gs(r.range), i = this.offsetAt(n.start), s = this.offsetAt(n.end);
        this._content = this._content.substring(0, i) + r.text + this._content.substring(s, this._content.length);
        const a = Math.max(n.start.line, 0), o = Math.max(n.end.line, 0);
        let l = this._lineOffsets;
        const u = ps(r.text, !1, i);
        if (o - a === u.length)
          for (let h = 0, d = u.length; h < d; h++)
            l[h + a + 1] = u[h];
        else
          u.length < 1e4 ? l.splice(a + 1, o - a, ...u) : this._lineOffsets = l = l.slice(0, a + 1).concat(u, l.slice(o + 1));
        const f = r.text.length - (s - i);
        if (f !== 0)
          for (let h = a + 1 + u.length, d = l.length; h < d; h++)
            l[h] = l[h] + f;
      } else if (or.isFull(r))
        this._content = r.text, this._lineOffsets = void 0;
      else
        throw new Error("Unknown change event received");
    this._version = t;
  }
  getLineOffsets() {
    return this._lineOffsets === void 0 && (this._lineOffsets = ps(this._content, !0)), this._lineOffsets;
  }
  positionAt(e) {
    e = Math.max(Math.min(e, this._content.length), 0);
    let t = this.getLineOffsets(), r = 0, n = t.length;
    if (n === 0)
      return { line: 0, character: e };
    for (; r < n; ) {
      let s = Math.floor((r + n) / 2);
      t[s] > e ? n = s : r = s + 1;
    }
    let i = r - 1;
    return { line: i, character: e - t[i] };
  }
  offsetAt(e) {
    let t = this.getLineOffsets();
    if (e.line >= t.length)
      return this._content.length;
    if (e.line < 0)
      return 0;
    let r = t[e.line], n = e.line + 1 < t.length ? t[e.line + 1] : this._content.length;
    return Math.max(Math.min(r + e.character, n), r);
  }
  get lineCount() {
    return this.getLineOffsets().length;
  }
  static isIncremental(e) {
    let t = e;
    return t != null && typeof t.text == "string" && t.range !== void 0 && (t.rangeLength === void 0 || typeof t.rangeLength == "number");
  }
  static isFull(e) {
    let t = e;
    return t != null && typeof t.text == "string" && t.range === void 0 && t.rangeLength === void 0;
  }
}, Yr;
(function(e) {
  function t(i, s, a, o) {
    return new or(i, s, a, o);
  }
  e.create = t;
  function r(i, s, a) {
    if (i instanceof or)
      return i.update(s, a), i;
    throw new Error("TextDocument.update: document must be created by TextDocument.create");
  }
  e.update = r;
  function n(i, s) {
    let a = i.getText(), o = Kr(s.map(_l), (f, h) => {
      let d = f.range.start.line - h.range.start.line;
      return d === 0 ? f.range.start.character - h.range.start.character : d;
    }), l = 0;
    const u = [];
    for (const f of o) {
      let h = i.offsetAt(f.range.start);
      if (h < l)
        throw new Error("Overlapping edit");
      h > l && u.push(a.substring(l, h)), f.newText.length && u.push(f.newText), l = i.offsetAt(f.range.end);
    }
    return u.push(a.substr(l)), u.join("");
  }
  e.applyEdits = n;
})(Yr || (Yr = {}));
function Kr(e, t) {
  if (e.length <= 1)
    return e;
  const r = e.length / 2 | 0, n = e.slice(0, r), i = e.slice(r);
  Kr(n, t), Kr(i, t);
  let s = 0, a = 0, o = 0;
  for (; s < n.length && a < i.length; )
    t(n[s], i[a]) <= 0 ? e[o++] = n[s++] : e[o++] = i[a++];
  for (; s < n.length; )
    e[o++] = n[s++];
  for (; a < i.length; )
    e[o++] = i[a++];
  return e;
}
function ps(e, t, r = 0) {
  const n = t ? [r] : [];
  for (let i = 0; i < e.length; i++) {
    let s = e.charCodeAt(i);
    (s === 13 || s === 10) && (s === 13 && i + 1 < e.length && e.charCodeAt(i + 1) === 10 && i++, n.push(r + i + 1));
  }
  return n;
}
function Gs(e) {
  const t = e.start, r = e.end;
  return t.line > r.line || t.line === r.line && t.character > r.character ? { start: r, end: t } : e;
}
function _l(e) {
  const t = Gs(e.range);
  return t !== e.range ? { newText: e.newText, range: t } : e;
}
var H;
(function(e) {
  e[e.Undefined = 0] = "Undefined", e[e.EnumValueMismatch = 1] = "EnumValueMismatch", e[e.Deprecated = 2] = "Deprecated", e[e.UnexpectedEndOfComment = 257] = "UnexpectedEndOfComment", e[e.UnexpectedEndOfString = 258] = "UnexpectedEndOfString", e[e.UnexpectedEndOfNumber = 259] = "UnexpectedEndOfNumber", e[e.InvalidUnicode = 260] = "InvalidUnicode", e[e.InvalidEscapeCharacter = 261] = "InvalidEscapeCharacter", e[e.InvalidCharacter = 262] = "InvalidCharacter", e[e.PropertyExpected = 513] = "PropertyExpected", e[e.CommaExpected = 514] = "CommaExpected", e[e.ColonExpected = 515] = "ColonExpected", e[e.ValueExpected = 516] = "ValueExpected", e[e.CommaOrCloseBacketExpected = 517] = "CommaOrCloseBacketExpected", e[e.CommaOrCloseBraceExpected = 518] = "CommaOrCloseBraceExpected", e[e.TrailingComma = 519] = "TrailingComma", e[e.DuplicateKey = 520] = "DuplicateKey", e[e.CommentNotPermitted = 521] = "CommentNotPermitted", e[e.SchemaResolveError = 768] = "SchemaResolveError";
})(H || (H = {}));
var vs;
(function(e) {
  e.LATEST = {
    textDocument: {
      completion: {
        completionItem: {
          documentationFormat: [Oe.Markdown, Oe.PlainText],
          commitCharactersSupport: !0
        }
      }
    }
  };
})(vs || (vs = {}));
function Sl(e, t) {
  let r;
  return t.length === 0 ? r = e : r = e.replace(/\{(\d+)\}/g, (n, i) => {
    let s = i[0];
    return typeof t[s] < "u" ? t[s] : n;
  }), r;
}
function Al(e, t, ...r) {
  return Sl(t, r);
}
function Rt(e) {
  return Al;
}
var tt = function() {
  var e = function(t, r) {
    return e = Object.setPrototypeOf || { __proto__: [] } instanceof Array && function(n, i) {
      n.__proto__ = i;
    } || function(n, i) {
      for (var s in i)
        Object.prototype.hasOwnProperty.call(i, s) && (n[s] = i[s]);
    }, e(t, r);
  };
  return function(t, r) {
    if (typeof r != "function" && r !== null)
      throw new TypeError("Class extends value " + String(r) + " is not a constructor or null");
    e(t, r);
    function n() {
      this.constructor = t;
    }
    t.prototype = r === null ? Object.create(r) : (n.prototype = r.prototype, new n());
  };
}(), V = Rt(), Nl = {
  "color-hex": { errorMessage: V("colorHexFormatWarning", "Invalid color format. Use #RGB, #RGBA, #RRGGBB or #RRGGBBAA."), pattern: /^#([0-9A-Fa-f]{3,4}|([0-9A-Fa-f]{2}){3,4})$/ },
  "date-time": { errorMessage: V("dateTimeFormatWarning", "String is not a RFC3339 date-time."), pattern: /^(\d{4})-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])T([01][0-9]|2[0-3]):([0-5][0-9]):([0-5][0-9]|60)(\.[0-9]+)?(Z|(\+|-)([01][0-9]|2[0-3]):([0-5][0-9]))$/i },
  date: { errorMessage: V("dateFormatWarning", "String is not a RFC3339 date."), pattern: /^(\d{4})-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])$/i },
  time: { errorMessage: V("timeFormatWarning", "String is not a RFC3339 time."), pattern: /^([01][0-9]|2[0-3]):([0-5][0-9]):([0-5][0-9]|60)(\.[0-9]+)?(Z|(\+|-)([01][0-9]|2[0-3]):([0-5][0-9]))$/i },
  email: { errorMessage: V("emailFormatWarning", "String is not an e-mail address."), pattern: /^(([^<>()\[\]\\.,;:\s@"]+(\.[^<>()\[\]\\.,;:\s@"]+)*)|(".+"))@((\[[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}])|(([a-zA-Z0-9-]+\.)+[a-zA-Z]{2,}))$/ },
  hostname: { errorMessage: V("hostnameFormatWarning", "String is not a hostname."), pattern: /^(?=.{1,253}\.?$)[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[-0-9a-z]{0,61}[0-9a-z])?)*\.?$/i },
  ipv4: { errorMessage: V("ipv4FormatWarning", "String is not an IPv4 address."), pattern: /^(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)$/ },
  ipv6: { errorMessage: V("ipv6FormatWarning", "String is not an IPv6 address."), pattern: /^((([0-9a-f]{1,4}:){7}([0-9a-f]{1,4}|:))|(([0-9a-f]{1,4}:){6}(:[0-9a-f]{1,4}|((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3})|:))|(([0-9a-f]{1,4}:){5}(((:[0-9a-f]{1,4}){1,2})|:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3})|:))|(([0-9a-f]{1,4}:){4}(((:[0-9a-f]{1,4}){1,3})|((:[0-9a-f]{1,4})?:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9a-f]{1,4}:){3}(((:[0-9a-f]{1,4}){1,4})|((:[0-9a-f]{1,4}){0,2}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9a-f]{1,4}:){2}(((:[0-9a-f]{1,4}){1,5})|((:[0-9a-f]{1,4}){0,3}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9a-f]{1,4}:){1}(((:[0-9a-f]{1,4}){1,6})|((:[0-9a-f]{1,4}){0,4}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(:(((:[0-9a-f]{1,4}){1,7})|((:[0-9a-f]{1,4}){0,5}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:)))$/i }
}, rt = function() {
  function e(t, r, n) {
    n === void 0 && (n = 0), this.offset = r, this.length = n, this.parent = t;
  }
  return Object.defineProperty(e.prototype, "children", {
    get: function() {
      return [];
    },
    enumerable: !1,
    configurable: !0
  }), e.prototype.toString = function() {
    return "type: " + this.type + " (" + this.offset + "/" + this.length + ")" + (this.parent ? " parent: {" + this.parent.toString() + "}" : "");
  }, e;
}(), Ll = function(e) {
  tt(t, e);
  function t(r, n) {
    var i = e.call(this, r, n) || this;
    return i.type = "null", i.value = null, i;
  }
  return t;
}(rt), bs = function(e) {
  tt(t, e);
  function t(r, n, i) {
    var s = e.call(this, r, i) || this;
    return s.type = "boolean", s.value = n, s;
  }
  return t;
}(rt), Cl = function(e) {
  tt(t, e);
  function t(r, n) {
    var i = e.call(this, r, n) || this;
    return i.type = "array", i.items = [], i;
  }
  return Object.defineProperty(t.prototype, "children", {
    get: function() {
      return this.items;
    },
    enumerable: !1,
    configurable: !0
  }), t;
}(rt), kl = function(e) {
  tt(t, e);
  function t(r, n) {
    var i = e.call(this, r, n) || this;
    return i.type = "number", i.isInteger = !0, i.value = Number.NaN, i;
  }
  return t;
}(rt), wr = function(e) {
  tt(t, e);
  function t(r, n, i) {
    var s = e.call(this, r, n, i) || this;
    return s.type = "string", s.value = "", s;
  }
  return t;
}(rt), El = function(e) {
  tt(t, e);
  function t(r, n, i) {
    var s = e.call(this, r, n) || this;
    return s.type = "property", s.colonOffset = -1, s.keyNode = i, s;
  }
  return Object.defineProperty(t.prototype, "children", {
    get: function() {
      return this.valueNode ? [this.keyNode, this.valueNode] : [this.keyNode];
    },
    enumerable: !1,
    configurable: !0
  }), t;
}(rt), Ml = function(e) {
  tt(t, e);
  function t(r, n) {
    var i = e.call(this, r, n) || this;
    return i.type = "object", i.properties = [], i;
  }
  return Object.defineProperty(t.prototype, "children", {
    get: function() {
      return this.properties;
    },
    enumerable: !1,
    configurable: !0
  }), t;
}(rt);
function ue(e) {
  return Te(e) ? e ? {} : { not: {} } : e;
}
var ys;
(function(e) {
  e[e.Key = 0] = "Key", e[e.Enum = 1] = "Enum";
})(ys || (ys = {}));
var Tl = function() {
  function e(t, r) {
    t === void 0 && (t = -1), this.focusOffset = t, this.exclude = r, this.schemas = [];
  }
  return e.prototype.add = function(t) {
    this.schemas.push(t);
  }, e.prototype.merge = function(t) {
    Array.prototype.push.apply(this.schemas, t.schemas);
  }, e.prototype.include = function(t) {
    return (this.focusOffset === -1 || Js(t, this.focusOffset)) && t !== this.exclude;
  }, e.prototype.newSub = function() {
    return new e(-1, this.exclude);
  }, e;
}(), en = function() {
  function e() {
  }
  return Object.defineProperty(e.prototype, "schemas", {
    get: function() {
      return [];
    },
    enumerable: !1,
    configurable: !0
  }), e.prototype.add = function(t) {
  }, e.prototype.merge = function(t) {
  }, e.prototype.include = function(t) {
    return !0;
  }, e.prototype.newSub = function() {
    return this;
  }, e.instance = new e(), e;
}(), ce = function() {
  function e() {
    this.problems = [], this.propertiesMatches = 0, this.propertiesValueMatches = 0, this.primaryValueMatches = 0, this.enumValueMatch = !1, this.enumValues = void 0;
  }
  return e.prototype.hasProblems = function() {
    return !!this.problems.length;
  }, e.prototype.mergeAll = function(t) {
    for (var r = 0, n = t; r < n.length; r++) {
      var i = n[r];
      this.merge(i);
    }
  }, e.prototype.merge = function(t) {
    this.problems = this.problems.concat(t.problems);
  }, e.prototype.mergeEnumValues = function(t) {
    if (!this.enumValueMatch && !t.enumValueMatch && this.enumValues && t.enumValues) {
      this.enumValues = this.enumValues.concat(t.enumValues);
      for (var r = 0, n = this.problems; r < n.length; r++) {
        var i = n[r];
        i.code === H.EnumValueMismatch && (i.message = V("enumWarning", "Value is not accepted. Valid values: {0}.", this.enumValues.map(function(s) {
          return JSON.stringify(s);
        }).join(", ")));
      }
    }
  }, e.prototype.mergePropertyMatch = function(t) {
    this.merge(t), this.propertiesMatches++, (t.enumValueMatch || !t.hasProblems() && t.propertiesMatches) && this.propertiesValueMatches++, t.enumValueMatch && t.enumValues && t.enumValues.length === 1 && this.primaryValueMatches++;
  }, e.prototype.compare = function(t) {
    var r = this.hasProblems();
    return r !== t.hasProblems() ? r ? -1 : 1 : this.enumValueMatch !== t.enumValueMatch ? t.enumValueMatch ? -1 : 1 : this.primaryValueMatches !== t.primaryValueMatches ? this.primaryValueMatches - t.primaryValueMatches : this.propertiesValueMatches !== t.propertiesValueMatches ? this.propertiesValueMatches - t.propertiesValueMatches : this.propertiesMatches - t.propertiesMatches;
  }, e;
}();
function Pl(e, t) {
  return t === void 0 && (t = []), new Xs(e, t, []);
}
function Ye(e) {
  return vl(e);
}
function tn(e) {
  return pl(e);
}
function Js(e, t, r) {
  return r === void 0 && (r = !1), t >= e.offset && t < e.offset + e.length || r && t === e.offset + e.length;
}
var Xs = function() {
  function e(t, r, n) {
    r === void 0 && (r = []), n === void 0 && (n = []), this.root = t, this.syntaxErrors = r, this.comments = n;
  }
  return e.prototype.getNodeFromOffset = function(t, r) {
    if (r === void 0 && (r = !1), this.root)
      return ml(this.root, t, r);
  }, e.prototype.visit = function(t) {
    if (this.root) {
      var r = function(n) {
        var i = t(n), s = n.children;
        if (Array.isArray(s))
          for (var a = 0; a < s.length && i; a++)
            i = r(s[a]);
        return i;
      };
      r(this.root);
    }
  }, e.prototype.validate = function(t, r, n) {
    if (n === void 0 && (n = ve.Warning), this.root && r) {
      var i = new ce();
      return ie(this.root, r, i, en.instance), i.problems.map(function(s) {
        var a, o = G.create(t.positionAt(s.location.offset), t.positionAt(s.location.offset + s.location.length));
        return De.create(o, s.message, (a = s.severity) !== null && a !== void 0 ? a : n, s.code);
      });
    }
  }, e.prototype.getMatchingSchemas = function(t, r, n) {
    r === void 0 && (r = -1);
    var i = new Tl(r, n);
    return this.root && t && ie(this.root, t, new ce(), i), i.schemas;
  }, e;
}();
function ie(e, t, r, n) {
  if (!e || !n.include(e))
    return;
  var i = e;
  switch (i.type) {
    case "object":
      u(i, t, r, n);
      break;
    case "array":
      l(i, t, r, n);
      break;
    case "string":
      o(i, t, r);
      break;
    case "number":
      a(i, t, r);
      break;
    case "property":
      return ie(i.valueNode, t, r, n);
  }
  s(), n.add({ node: i, schema: t });
  function s() {
    function f(F) {
      return i.type === F || F === "integer" && i.type === "number" && i.isInteger;
    }
    if (Array.isArray(t.type) ? t.type.some(f) || r.problems.push({
      location: { offset: i.offset, length: i.length },
      message: t.errorMessage || V("typeArrayMismatchWarning", "Incorrect type. Expected one of {0}.", t.type.join(", "))
    }) : t.type && (f(t.type) || r.problems.push({
      location: { offset: i.offset, length: i.length },
      message: t.errorMessage || V("typeMismatchWarning", 'Incorrect type. Expected "{0}".', t.type)
    })), Array.isArray(t.allOf))
      for (var h = 0, d = t.allOf; h < d.length; h++) {
        var g = d[h];
        ie(i, ue(g), r, n);
      }
    var p = ue(t.not);
    if (p) {
      var m = new ce(), v = n.newSub();
      ie(i, p, m, v), m.hasProblems() || r.problems.push({
        location: { offset: i.offset, length: i.length },
        message: V("notSchemaWarning", "Matches a schema that is not allowed.")
      });
      for (var y = 0, x = v.schemas; y < x.length; y++) {
        var b = x[y];
        b.inverted = !b.inverted, n.add(b);
      }
    }
    var S = function(F, D) {
      for (var z = [], W = void 0, P = 0, T = F; P < T.length; P++) {
        var R = T[P], I = ue(R), j = new ce(), U = n.newSub();
        if (ie(i, I, j, U), j.hasProblems() || z.push(I), !W)
          W = { schema: I, validationResult: j, matchingSchemas: U };
        else if (!D && !j.hasProblems() && !W.validationResult.hasProblems())
          W.matchingSchemas.merge(U), W.validationResult.propertiesMatches += j.propertiesMatches, W.validationResult.propertiesValueMatches += j.propertiesValueMatches;
        else {
          var q = j.compare(W.validationResult);
          q > 0 ? W = { schema: I, validationResult: j, matchingSchemas: U } : q === 0 && (W.matchingSchemas.merge(U), W.validationResult.mergeEnumValues(j));
        }
      }
      return z.length > 1 && D && r.problems.push({
        location: { offset: i.offset, length: 1 },
        message: V("oneOfWarning", "Matches multiple schemas when only one must validate.")
      }), W && (r.merge(W.validationResult), r.propertiesMatches += W.validationResult.propertiesMatches, r.propertiesValueMatches += W.validationResult.propertiesValueMatches, n.merge(W.matchingSchemas)), z.length;
    };
    Array.isArray(t.anyOf) && S(t.anyOf, !1), Array.isArray(t.oneOf) && S(t.oneOf, !0);
    var M = function(F) {
      var D = new ce(), z = n.newSub();
      ie(i, ue(F), D, z), r.merge(D), r.propertiesMatches += D.propertiesMatches, r.propertiesValueMatches += D.propertiesValueMatches, n.merge(z);
    }, A = function(F, D, z) {
      var W = ue(F), P = new ce(), T = n.newSub();
      ie(i, W, P, T), n.merge(T), P.hasProblems() ? z && M(z) : D && M(D);
    }, C = ue(t.if);
    if (C && A(C, ue(t.then), ue(t.else)), Array.isArray(t.enum)) {
      for (var _ = Ye(i), w = !1, N = 0, k = t.enum; N < k.length; N++) {
        var L = k[N];
        if (_t(_, L)) {
          w = !0;
          break;
        }
      }
      r.enumValues = t.enum, r.enumValueMatch = w, w || r.problems.push({
        location: { offset: i.offset, length: i.length },
        code: H.EnumValueMismatch,
        message: t.errorMessage || V("enumWarning", "Value is not accepted. Valid values: {0}.", t.enum.map(function(F) {
          return JSON.stringify(F);
        }).join(", "))
      });
    }
    if (Ie(t.const)) {
      var _ = Ye(i);
      _t(_, t.const) ? r.enumValueMatch = !0 : (r.problems.push({
        location: { offset: i.offset, length: i.length },
        code: H.EnumValueMismatch,
        message: t.errorMessage || V("constWarning", "Value must be {0}.", JSON.stringify(t.const))
      }), r.enumValueMatch = !1), r.enumValues = [t.const];
    }
    t.deprecationMessage && i.parent && r.problems.push({
      location: { offset: i.parent.offset, length: i.parent.length },
      severity: ve.Warning,
      message: t.deprecationMessage,
      code: H.Deprecated
    });
  }
  function a(f, h, d, g) {
    var p = f.value;
    function m(N) {
      var k, L = /^(-?\d+)(?:\.(\d+))?(?:e([-+]\d+))?$/.exec(N.toString());
      return L && {
        value: Number(L[1] + (L[2] || "")),
        multiplier: (((k = L[2]) === null || k === void 0 ? void 0 : k.length) || 0) - (parseInt(L[3]) || 0)
      };
    }
    if (ge(h.multipleOf)) {
      var v = -1;
      if (Number.isInteger(h.multipleOf))
        v = p % h.multipleOf;
      else {
        var y = m(h.multipleOf), x = m(p);
        if (y && x) {
          var b = Math.pow(10, Math.abs(x.multiplier - y.multiplier));
          x.multiplier < y.multiplier ? x.value *= b : y.value *= b, v = x.value % y.value;
        }
      }
      v !== 0 && d.problems.push({
        location: { offset: f.offset, length: f.length },
        message: V("multipleOfWarning", "Value is not divisible by {0}.", h.multipleOf)
      });
    }
    function S(N, k) {
      if (ge(k))
        return k;
      if (Te(k) && k)
        return N;
    }
    function M(N, k) {
      if (!Te(k) || !k)
        return N;
    }
    var A = S(h.minimum, h.exclusiveMinimum);
    ge(A) && p <= A && d.problems.push({
      location: { offset: f.offset, length: f.length },
      message: V("exclusiveMinimumWarning", "Value is below the exclusive minimum of {0}.", A)
    });
    var C = S(h.maximum, h.exclusiveMaximum);
    ge(C) && p >= C && d.problems.push({
      location: { offset: f.offset, length: f.length },
      message: V("exclusiveMaximumWarning", "Value is above the exclusive maximum of {0}.", C)
    });
    var _ = M(h.minimum, h.exclusiveMinimum);
    ge(_) && p < _ && d.problems.push({
      location: { offset: f.offset, length: f.length },
      message: V("minimumWarning", "Value is below the minimum of {0}.", _)
    });
    var w = M(h.maximum, h.exclusiveMaximum);
    ge(w) && p > w && d.problems.push({
      location: { offset: f.offset, length: f.length },
      message: V("maximumWarning", "Value is above the maximum of {0}.", w)
    });
  }
  function o(f, h, d, g) {
    if (ge(h.minLength) && f.value.length < h.minLength && d.problems.push({
      location: { offset: f.offset, length: f.length },
      message: V("minLengthWarning", "String is shorter than the minimum length of {0}.", h.minLength)
    }), ge(h.maxLength) && f.value.length > h.maxLength && d.problems.push({
      location: { offset: f.offset, length: f.length },
      message: V("maxLengthWarning", "String is longer than the maximum length of {0}.", h.maxLength)
    }), yl(h.pattern)) {
      var p = tr(h.pattern);
      p != null && p.test(f.value) || d.problems.push({
        location: { offset: f.offset, length: f.length },
        message: h.patternErrorMessage || h.errorMessage || V("patternWarning", 'String does not match the pattern of "{0}".', h.pattern)
      });
    }
    if (h.format)
      switch (h.format) {
        case "uri":
        case "uri-reference":
          {
            var m = void 0;
            if (!f.value)
              m = V("uriEmpty", "URI expected.");
            else {
              var v = /^(([^:/?#]+?):)?(\/\/([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?/.exec(f.value);
              v ? !v[2] && h.format === "uri" && (m = V("uriSchemeMissing", "URI with a scheme is expected.")) : m = V("uriMissing", "URI is expected.");
            }
            m && d.problems.push({
              location: { offset: f.offset, length: f.length },
              message: h.patternErrorMessage || h.errorMessage || V("uriFormatWarning", "String is not a URI: {0}", m)
            });
          }
          break;
        case "color-hex":
        case "date-time":
        case "date":
        case "time":
        case "email":
        case "hostname":
        case "ipv4":
        case "ipv6":
          var y = Nl[h.format];
          (!f.value || !y.pattern.exec(f.value)) && d.problems.push({
            location: { offset: f.offset, length: f.length },
            message: h.patternErrorMessage || h.errorMessage || y.errorMessage
          });
      }
  }
  function l(f, h, d, g) {
    if (Array.isArray(h.items)) {
      for (var p = h.items, m = 0; m < p.length; m++) {
        var v = p[m], y = ue(v), x = new ce(), b = f.items[m];
        b ? (ie(b, y, x, g), d.mergePropertyMatch(x)) : f.items.length >= p.length && d.propertiesValueMatches++;
      }
      if (f.items.length > p.length)
        if (typeof h.additionalItems == "object")
          for (var S = p.length; S < f.items.length; S++) {
            var x = new ce();
            ie(f.items[S], h.additionalItems, x, g), d.mergePropertyMatch(x);
          }
        else
          h.additionalItems === !1 && d.problems.push({
            location: { offset: f.offset, length: f.length },
            message: V("additionalItemsWarning", "Array has too many items according to schema. Expected {0} or fewer.", p.length)
          });
    } else {
      var M = ue(h.items);
      if (M)
        for (var A = 0, C = f.items; A < C.length; A++) {
          var b = C[A], x = new ce();
          ie(b, M, x, g), d.mergePropertyMatch(x);
        }
    }
    var _ = ue(h.contains);
    if (_) {
      var w = f.items.some(function(L) {
        var F = new ce();
        return ie(L, _, F, en.instance), !F.hasProblems();
      });
      w || d.problems.push({
        location: { offset: f.offset, length: f.length },
        message: h.errorMessage || V("requiredItemMissingWarning", "Array does not contain required item.")
      });
    }
    if (ge(h.minItems) && f.items.length < h.minItems && d.problems.push({
      location: { offset: f.offset, length: f.length },
      message: V("minItemsWarning", "Array has too few items. Expected {0} or more.", h.minItems)
    }), ge(h.maxItems) && f.items.length > h.maxItems && d.problems.push({
      location: { offset: f.offset, length: f.length },
      message: V("maxItemsWarning", "Array has too many items. Expected {0} or fewer.", h.maxItems)
    }), h.uniqueItems === !0) {
      var N = Ye(f), k = N.some(function(L, F) {
        return F !== N.lastIndexOf(L);
      });
      k && d.problems.push({
        location: { offset: f.offset, length: f.length },
        message: V("uniqueItemsWarning", "Array has duplicate items.")
      });
    }
  }
  function u(f, h, d, g) {
    for (var p = /* @__PURE__ */ Object.create(null), m = [], v = 0, y = f.properties; v < y.length; v++) {
      var x = y[v], b = x.keyNode.value;
      p[b] = x.valueNode, m.push(b);
    }
    if (Array.isArray(h.required))
      for (var S = 0, M = h.required; S < M.length; S++) {
        var A = M[S];
        if (!p[A]) {
          var C = f.parent && f.parent.type === "property" && f.parent.keyNode, _ = C ? { offset: C.offset, length: C.length } : { offset: f.offset, length: 1 };
          d.problems.push({
            location: _,
            message: V("MissingRequiredPropWarning", 'Missing property "{0}".', A)
          });
        }
      }
    var w = function(vn) {
      for (var hr = m.indexOf(vn); hr >= 0; )
        m.splice(hr, 1), hr = m.indexOf(vn);
    };
    if (h.properties)
      for (var N = 0, k = Object.keys(h.properties); N < k.length; N++) {
        var A = k[N];
        w(A);
        var L = h.properties[A], F = p[A];
        if (F)
          if (Te(L))
            if (L)
              d.propertiesMatches++, d.propertiesValueMatches++;
            else {
              var x = F.parent;
              d.problems.push({
                location: { offset: x.keyNode.offset, length: x.keyNode.length },
                message: h.errorMessage || V("DisallowedExtraPropWarning", "Property {0} is not allowed.", A)
              });
            }
          else {
            var D = new ce();
            ie(F, L, D, g), d.mergePropertyMatch(D);
          }
      }
    if (h.patternProperties)
      for (var z = 0, W = Object.keys(h.patternProperties); z < W.length; z++)
        for (var P = W[z], T = tr(P), R = 0, I = m.slice(0); R < I.length; R++) {
          var A = I[R];
          if (T != null && T.test(A)) {
            w(A);
            var F = p[A];
            if (F) {
              var L = h.patternProperties[P];
              if (Te(L))
                if (L)
                  d.propertiesMatches++, d.propertiesValueMatches++;
                else {
                  var x = F.parent;
                  d.problems.push({
                    location: { offset: x.keyNode.offset, length: x.keyNode.length },
                    message: h.errorMessage || V("DisallowedExtraPropWarning", "Property {0} is not allowed.", A)
                  });
                }
              else {
                var D = new ce();
                ie(F, L, D, g), d.mergePropertyMatch(D);
              }
            }
          }
        }
    if (typeof h.additionalProperties == "object")
      for (var j = 0, U = m; j < U.length; j++) {
        var A = U[j], F = p[A];
        if (F) {
          var D = new ce();
          ie(F, h.additionalProperties, D, g), d.mergePropertyMatch(D);
        }
      }
    else if (h.additionalProperties === !1 && m.length > 0)
      for (var q = 0, Le = m; q < Le.length; q++) {
        var A = Le[q], F = p[A];
        if (F) {
          var x = F.parent;
          d.problems.push({
            location: { offset: x.keyNode.offset, length: x.keyNode.length },
            message: h.errorMessage || V("DisallowedExtraPropWarning", "Property {0} is not allowed.", A)
          });
        }
      }
    if (ge(h.maxProperties) && f.properties.length > h.maxProperties && d.problems.push({
      location: { offset: f.offset, length: f.length },
      message: V("MaxPropWarning", "Object has more properties than limit of {0}.", h.maxProperties)
    }), ge(h.minProperties) && f.properties.length < h.minProperties && d.problems.push({
      location: { offset: f.offset, length: f.length },
      message: V("MinPropWarning", "Object has fewer properties than the required number of {0}", h.minProperties)
    }), h.dependencies)
      for (var ae = 0, be = Object.keys(h.dependencies); ae < be.length; ae++) {
        var b = be[ae], nt = p[b];
        if (nt) {
          var Ce = h.dependencies[b];
          if (Array.isArray(Ce))
            for (var cr = 0, dn = Ce; cr < dn.length; cr++) {
              var gn = dn[cr];
              p[gn] ? d.propertiesValueMatches++ : d.problems.push({
                location: { offset: f.offset, length: f.length },
                message: V("RequiredDependentPropWarning", "Object is missing property {0} required by property {1}.", gn, b)
              });
            }
          else {
            var L = ue(Ce);
            if (L) {
              var D = new ce();
              ie(f, L, D, g), d.mergePropertyMatch(D);
            }
          }
        }
      }
    var mn = ue(h.propertyNames);
    if (mn)
      for (var fr = 0, pn = f.properties; fr < pn.length; fr++) {
        var ta = pn[fr], b = ta.keyNode;
        b && ie(b, mn, d, en.instance);
      }
  }
}
function Rl(e, t) {
  var r = [], n = -1, i = e.getText(), s = ht(i, !1), a = t && t.collectComments ? [] : void 0;
  function o() {
    for (; ; ) {
      var A = s.scan();
      switch (f(), A) {
        case 12:
        case 13:
          Array.isArray(a) && a.push(G.create(e.positionAt(s.getTokenOffset()), e.positionAt(s.getTokenOffset() + s.getTokenLength())));
          break;
        case 15:
        case 14:
          break;
        default:
          return A;
      }
    }
  }
  function l(A, C, _, w, N) {
    if (N === void 0 && (N = ve.Error), r.length === 0 || _ !== n) {
      var k = G.create(e.positionAt(_), e.positionAt(w));
      r.push(De.create(k, A, N, C, e.languageId)), n = _;
    }
  }
  function u(A, C, _, w, N) {
    _ === void 0 && (_ = void 0), w === void 0 && (w = []), N === void 0 && (N = []);
    var k = s.getTokenOffset(), L = s.getTokenOffset() + s.getTokenLength();
    if (k === L && k > 0) {
      for (k--; k > 0 && /\s/.test(i.charAt(k)); )
        k--;
      L = k + 1;
    }
    if (l(A, C, k, L), _ && h(_, !1), w.length + N.length > 0)
      for (var F = s.getToken(); F !== 17; ) {
        if (w.indexOf(F) !== -1) {
          o();
          break;
        } else if (N.indexOf(F) !== -1)
          break;
        F = o();
      }
    return _;
  }
  function f() {
    switch (s.getTokenError()) {
      case 4:
        return u(V("InvalidUnicode", "Invalid unicode sequence in string."), H.InvalidUnicode), !0;
      case 5:
        return u(V("InvalidEscapeCharacter", "Invalid escape character in string."), H.InvalidEscapeCharacter), !0;
      case 3:
        return u(V("UnexpectedEndOfNumber", "Unexpected end of number."), H.UnexpectedEndOfNumber), !0;
      case 1:
        return u(V("UnexpectedEndOfComment", "Unexpected end of comment."), H.UnexpectedEndOfComment), !0;
      case 2:
        return u(V("UnexpectedEndOfString", "Unexpected end of string."), H.UnexpectedEndOfString), !0;
      case 6:
        return u(V("InvalidCharacter", "Invalid characters in string. Control characters must be escaped."), H.InvalidCharacter), !0;
    }
    return !1;
  }
  function h(A, C) {
    return A.length = s.getTokenOffset() + s.getTokenLength() - A.offset, C && o(), A;
  }
  function d(A) {
    if (s.getToken() === 3) {
      var C = new Cl(A, s.getTokenOffset());
      o();
      for (var _ = !1; s.getToken() !== 4 && s.getToken() !== 17; ) {
        if (s.getToken() === 5) {
          _ || u(V("ValueExpected", "Value expected"), H.ValueExpected);
          var w = s.getTokenOffset();
          if (o(), s.getToken() === 4) {
            _ && l(V("TrailingComma", "Trailing comma"), H.TrailingComma, w, w + 1);
            continue;
          }
        } else
          _ && u(V("ExpectedComma", "Expected comma"), H.CommaExpected);
        var N = b(C);
        N ? C.items.push(N) : u(V("PropertyExpected", "Value expected"), H.ValueExpected, void 0, [], [4, 5]), _ = !0;
      }
      return s.getToken() !== 4 ? u(V("ExpectedCloseBracket", "Expected comma or closing bracket"), H.CommaOrCloseBacketExpected, C) : h(C, !0);
    }
  }
  var g = new wr(void 0, 0, 0);
  function p(A, C) {
    var _ = new El(A, s.getTokenOffset(), g), w = v(_);
    if (!w)
      if (s.getToken() === 16) {
        u(V("DoubleQuotesExpected", "Property keys must be doublequoted"), H.Undefined);
        var N = new wr(_, s.getTokenOffset(), s.getTokenLength());
        N.value = s.getTokenValue(), w = N, o();
      } else
        return;
    _.keyNode = w;
    var k = C[w.value];
    if (k ? (l(V("DuplicateKeyWarning", "Duplicate object key"), H.DuplicateKey, _.keyNode.offset, _.keyNode.offset + _.keyNode.length, ve.Warning), typeof k == "object" && l(V("DuplicateKeyWarning", "Duplicate object key"), H.DuplicateKey, k.keyNode.offset, k.keyNode.offset + k.keyNode.length, ve.Warning), C[w.value] = !0) : C[w.value] = _, s.getToken() === 6)
      _.colonOffset = s.getTokenOffset(), o();
    else if (u(V("ColonExpected", "Colon expected"), H.ColonExpected), s.getToken() === 10 && e.positionAt(w.offset + w.length).line < e.positionAt(s.getTokenOffset()).line)
      return _.length = w.length, _;
    var L = b(_);
    return L ? (_.valueNode = L, _.length = L.offset + L.length - _.offset, _) : u(V("ValueExpected", "Value expected"), H.ValueExpected, _, [], [2, 5]);
  }
  function m(A) {
    if (s.getToken() === 1) {
      var C = new Ml(A, s.getTokenOffset()), _ = /* @__PURE__ */ Object.create(null);
      o();
      for (var w = !1; s.getToken() !== 2 && s.getToken() !== 17; ) {
        if (s.getToken() === 5) {
          w || u(V("PropertyExpected", "Property expected"), H.PropertyExpected);
          var N = s.getTokenOffset();
          if (o(), s.getToken() === 2) {
            w && l(V("TrailingComma", "Trailing comma"), H.TrailingComma, N, N + 1);
            continue;
          }
        } else
          w && u(V("ExpectedComma", "Expected comma"), H.CommaExpected);
        var k = p(C, _);
        k ? C.properties.push(k) : u(V("PropertyExpected", "Property expected"), H.PropertyExpected, void 0, [], [2, 5]), w = !0;
      }
      return s.getToken() !== 2 ? u(V("ExpectedCloseBrace", "Expected comma or closing brace"), H.CommaOrCloseBraceExpected, C) : h(C, !0);
    }
  }
  function v(A) {
    if (s.getToken() === 10) {
      var C = new wr(A, s.getTokenOffset());
      return C.value = s.getTokenValue(), h(C, !0);
    }
  }
  function y(A) {
    if (s.getToken() === 11) {
      var C = new kl(A, s.getTokenOffset());
      if (s.getTokenError() === 0) {
        var _ = s.getTokenValue();
        try {
          var w = JSON.parse(_);
          if (!ge(w))
            return u(V("InvalidNumberFormat", "Invalid number format."), H.Undefined, C);
          C.value = w;
        } catch {
          return u(V("InvalidNumberFormat", "Invalid number format."), H.Undefined, C);
        }
        C.isInteger = _.indexOf(".") === -1;
      }
      return h(C, !0);
    }
  }
  function x(A) {
    switch (s.getToken()) {
      case 7:
        return h(new Ll(A, s.getTokenOffset()), !0);
      case 8:
        return h(new bs(A, !0, s.getTokenOffset()), !0);
      case 9:
        return h(new bs(A, !1, s.getTokenOffset()), !0);
      default:
        return;
    }
  }
  function b(A) {
    return d(A) || m(A) || v(A) || y(A) || x(A);
  }
  var S = void 0, M = o();
  return M !== 17 && (S = b(S), S ? s.getToken() !== 17 && u(V("End of file expected", "End of file expected."), H.Undefined) : u(V("Invalid symbol", "Expected a JSON object, array or literal."), H.Undefined)), new Xs(S, r, a);
}
function rn(e, t, r) {
  if (e !== null && typeof e == "object") {
    var n = t + "	";
    if (Array.isArray(e)) {
      if (e.length === 0)
        return "[]";
      for (var i = `[
`, s = 0; s < e.length; s++)
        i += n + rn(e[s], n, r), s < e.length - 1 && (i += ","), i += `
`;
      return i += t + "]", i;
    } else {
      var a = Object.keys(e);
      if (a.length === 0)
        return "{}";
      for (var i = `{
`, s = 0; s < a.length; s++) {
        var o = a[s];
        i += n + JSON.stringify(o) + ": " + rn(e[o], n, r), s < a.length - 1 && (i += ","), i += `
`;
      }
      return i += t + "}", i;
    }
  }
  return r(e);
}
var _r = Rt(), Fl = function() {
  function e(t, r, n, i) {
    r === void 0 && (r = []), n === void 0 && (n = Promise), i === void 0 && (i = {}), this.schemaService = t, this.contributions = r, this.promiseConstructor = n, this.clientCapabilities = i;
  }
  return e.prototype.doResolve = function(t) {
    for (var r = this.contributions.length - 1; r >= 0; r--) {
      var n = this.contributions[r].resolveCompletion;
      if (n) {
        var i = n(t);
        if (i)
          return i;
      }
    }
    return this.promiseConstructor.resolve(t);
  }, e.prototype.doComplete = function(t, r, n) {
    var i = this, s = {
      items: [],
      isIncomplete: !1
    }, a = t.getText(), o = t.offsetAt(r), l = n.getNodeFromOffset(o, !0);
    if (this.isInComment(t, l ? l.offset : 0, o))
      return Promise.resolve(s);
    if (l && o === l.offset + l.length && o > 0) {
      var u = a[o - 1];
      (l.type === "object" && u === "}" || l.type === "array" && u === "]") && (l = l.parent);
    }
    var f = this.getCurrentWord(t, o), h;
    if (l && (l.type === "string" || l.type === "number" || l.type === "boolean" || l.type === "null"))
      h = G.create(t.positionAt(l.offset), t.positionAt(l.offset + l.length));
    else {
      var d = o - f.length;
      d > 0 && a[d - 1] === '"' && d--, h = G.create(t.positionAt(d), r);
    }
    var g = {}, p = {
      add: function(m) {
        var v = m.label, y = g[v];
        if (y)
          y.documentation || (y.documentation = m.documentation), y.detail || (y.detail = m.detail);
        else {
          if (v = v.replace(/[\n]/g, "↵"), v.length > 60) {
            var x = v.substr(0, 57).trim() + "...";
            g[x] || (v = x);
          }
          h && m.insertText !== void 0 && (m.textEdit = Ne.replace(h, m.insertText)), m.label = v, g[v] = m, s.items.push(m);
        }
      },
      setAsIncomplete: function() {
        s.isIncomplete = !0;
      },
      error: function(m) {
        console.error(m);
      },
      log: function(m) {
        console.log(m);
      },
      getNumberOfProposals: function() {
        return s.items.length;
      }
    };
    return this.schemaService.getSchemaForResource(t.uri, n).then(function(m) {
      var v = [], y = !0, x = "", b = void 0;
      if (l && l.type === "string") {
        var S = l.parent;
        S && S.type === "property" && S.keyNode === l && (y = !S.valueNode, b = S, x = a.substr(l.offset + 1, l.length - 2), S && (l = S.parent));
      }
      if (l && l.type === "object") {
        if (l.offset === o)
          return s;
        var M = l.properties;
        M.forEach(function(w) {
          (!b || b !== w) && (g[w.keyNode.value] = Qr.create("__"));
        });
        var A = "";
        y && (A = i.evaluateSeparatorAfter(t, t.offsetAt(h.end))), m ? i.getPropertyCompletions(m, n, l, y, A, p) : i.getSchemaLessPropertyCompletions(n, l, x, p);
        var C = tn(l);
        i.contributions.forEach(function(w) {
          var N = w.collectPropertyCompletions(t.uri, C, f, y, A === "", p);
          N && v.push(N);
        }), !m && f.length > 0 && a.charAt(o - f.length - 1) !== '"' && (p.add({
          kind: pe.Property,
          label: i.getLabelForValue(f),
          insertText: i.getInsertTextForProperty(f, void 0, !1, A),
          insertTextFormat: te.Snippet,
          documentation: ""
        }), p.setAsIncomplete());
      }
      var _ = {};
      return m ? i.getValueCompletions(m, n, l, o, t, p, _) : i.getSchemaLessValueCompletions(n, l, o, t, p), i.contributions.length > 0 && i.getContributedValueCompletions(n, l, o, t, p, v), i.promiseConstructor.all(v).then(function() {
        if (p.getNumberOfProposals() === 0) {
          var w = o;
          l && (l.type === "string" || l.type === "number" || l.type === "boolean" || l.type === "null") && (w = l.offset + l.length);
          var N = i.evaluateSeparatorAfter(t, w);
          i.addFillerValueCompletions(_, N, p);
        }
        return s;
      });
    });
  }, e.prototype.getPropertyCompletions = function(t, r, n, i, s, a) {
    var o = this, l = r.getMatchingSchemas(t.schema, n.offset);
    l.forEach(function(u) {
      if (u.node === n && !u.inverted) {
        var f = u.schema.properties;
        f && Object.keys(f).forEach(function(m) {
          var v = f[m];
          if (typeof v == "object" && !v.deprecationMessage && !v.doNotSuggest) {
            var y = {
              kind: pe.Property,
              label: m,
              insertText: o.getInsertTextForProperty(m, v, i, s),
              insertTextFormat: te.Snippet,
              filterText: o.getFilterTextForValue(m),
              documentation: o.fromMarkup(v.markdownDescription) || v.description || ""
            };
            v.suggestSortText !== void 0 && (y.sortText = v.suggestSortText), y.insertText && Lt(y.insertText, "$1".concat(s)) && (y.command = {
              title: "Suggest",
              command: "editor.action.triggerSuggest"
            }), a.add(y);
          }
        });
        var h = u.schema.propertyNames;
        if (typeof h == "object" && !h.deprecationMessage && !h.doNotSuggest) {
          var d = function(m, v) {
            v === void 0 && (v = void 0);
            var y = {
              kind: pe.Property,
              label: m,
              insertText: o.getInsertTextForProperty(m, void 0, i, s),
              insertTextFormat: te.Snippet,
              filterText: o.getFilterTextForValue(m),
              documentation: v || o.fromMarkup(h.markdownDescription) || h.description || ""
            };
            h.suggestSortText !== void 0 && (y.sortText = h.suggestSortText), y.insertText && Lt(y.insertText, "$1".concat(s)) && (y.command = {
              title: "Suggest",
              command: "editor.action.triggerSuggest"
            }), a.add(y);
          };
          if (h.enum)
            for (var g = 0; g < h.enum.length; g++) {
              var p = void 0;
              h.markdownEnumDescriptions && g < h.markdownEnumDescriptions.length ? p = o.fromMarkup(h.markdownEnumDescriptions[g]) : h.enumDescriptions && g < h.enumDescriptions.length && (p = h.enumDescriptions[g]), d(h.enum[g], p);
            }
          h.const && d(h.const);
        }
      }
    });
  }, e.prototype.getSchemaLessPropertyCompletions = function(t, r, n, i) {
    var s = this, a = function(l) {
      l.properties.forEach(function(u) {
        var f = u.keyNode.value;
        i.add({
          kind: pe.Property,
          label: f,
          insertText: s.getInsertTextForValue(f, ""),
          insertTextFormat: te.Snippet,
          filterText: s.getFilterTextForValue(f),
          documentation: ""
        });
      });
    };
    if (r.parent)
      if (r.parent.type === "property") {
        var o = r.parent.keyNode.value;
        t.visit(function(l) {
          return l.type === "property" && l !== r.parent && l.keyNode.value === o && l.valueNode && l.valueNode.type === "object" && a(l.valueNode), !0;
        });
      } else
        r.parent.type === "array" && r.parent.items.forEach(function(l) {
          l.type === "object" && l !== r && a(l);
        });
    else
      r.type === "object" && i.add({
        kind: pe.Property,
        label: "$schema",
        insertText: this.getInsertTextForProperty("$schema", void 0, !0, ""),
        insertTextFormat: te.Snippet,
        documentation: "",
        filterText: this.getFilterTextForValue("$schema")
      });
  }, e.prototype.getSchemaLessValueCompletions = function(t, r, n, i, s) {
    var a = this, o = n;
    if (r && (r.type === "string" || r.type === "number" || r.type === "boolean" || r.type === "null") && (o = r.offset + r.length, r = r.parent), !r) {
      s.add({
        kind: this.getSuggestionKind("object"),
        label: "Empty object",
        insertText: this.getInsertTextForValue({}, ""),
        insertTextFormat: te.Snippet,
        documentation: ""
      }), s.add({
        kind: this.getSuggestionKind("array"),
        label: "Empty array",
        insertText: this.getInsertTextForValue([], ""),
        insertTextFormat: te.Snippet,
        documentation: ""
      });
      return;
    }
    var l = this.evaluateSeparatorAfter(i, o), u = function(g) {
      g.parent && !Js(g.parent, n, !0) && s.add({
        kind: a.getSuggestionKind(g.type),
        label: a.getLabelTextForMatchingNode(g, i),
        insertText: a.getInsertTextForMatchingNode(g, i, l),
        insertTextFormat: te.Snippet,
        documentation: ""
      }), g.type === "boolean" && a.addBooleanValueCompletion(!g.value, l, s);
    };
    if (r.type === "property" && n > (r.colonOffset || 0)) {
      var f = r.valueNode;
      if (f && (n > f.offset + f.length || f.type === "object" || f.type === "array"))
        return;
      var h = r.keyNode.value;
      t.visit(function(g) {
        return g.type === "property" && g.keyNode.value === h && g.valueNode && u(g.valueNode), !0;
      }), h === "$schema" && r.parent && !r.parent.parent && this.addDollarSchemaCompletions(l, s);
    }
    if (r.type === "array")
      if (r.parent && r.parent.type === "property") {
        var d = r.parent.keyNode.value;
        t.visit(function(g) {
          return g.type === "property" && g.keyNode.value === d && g.valueNode && g.valueNode.type === "array" && g.valueNode.items.forEach(u), !0;
        });
      } else
        r.items.forEach(u);
  }, e.prototype.getValueCompletions = function(t, r, n, i, s, a, o) {
    var l = i, u = void 0, f = void 0;
    if (n && (n.type === "string" || n.type === "number" || n.type === "boolean" || n.type === "null") && (l = n.offset + n.length, f = n, n = n.parent), !n) {
      this.addSchemaValueCompletions(t.schema, "", a, o);
      return;
    }
    if (n.type === "property" && i > (n.colonOffset || 0)) {
      var h = n.valueNode;
      if (h && i > h.offset + h.length)
        return;
      u = n.keyNode.value, n = n.parent;
    }
    if (n && (u !== void 0 || n.type === "array")) {
      for (var d = this.evaluateSeparatorAfter(s, l), g = r.getMatchingSchemas(t.schema, n.offset, f), p = 0, m = g; p < m.length; p++) {
        var v = m[p];
        if (v.node === n && !v.inverted && v.schema) {
          if (n.type === "array" && v.schema.items)
            if (Array.isArray(v.schema.items)) {
              var y = this.findItemAtOffset(n, s, i);
              y < v.schema.items.length && this.addSchemaValueCompletions(v.schema.items[y], d, a, o);
            } else
              this.addSchemaValueCompletions(v.schema.items, d, a, o);
          if (u !== void 0) {
            var x = !1;
            if (v.schema.properties) {
              var b = v.schema.properties[u];
              b && (x = !0, this.addSchemaValueCompletions(b, d, a, o));
            }
            if (v.schema.patternProperties && !x)
              for (var S = 0, M = Object.keys(v.schema.patternProperties); S < M.length; S++) {
                var A = M[S], C = tr(A);
                if (C != null && C.test(u)) {
                  x = !0;
                  var b = v.schema.patternProperties[A];
                  this.addSchemaValueCompletions(b, d, a, o);
                }
              }
            if (v.schema.additionalProperties && !x) {
              var b = v.schema.additionalProperties;
              this.addSchemaValueCompletions(b, d, a, o);
            }
          }
        }
      }
      u === "$schema" && !n.parent && this.addDollarSchemaCompletions(d, a), o.boolean && (this.addBooleanValueCompletion(!0, d, a), this.addBooleanValueCompletion(!1, d, a)), o.null && this.addNullValueCompletion(d, a);
    }
  }, e.prototype.getContributedValueCompletions = function(t, r, n, i, s, a) {
    if (!r)
      this.contributions.forEach(function(f) {
        var h = f.collectDefaultCompletions(i.uri, s);
        h && a.push(h);
      });
    else if ((r.type === "string" || r.type === "number" || r.type === "boolean" || r.type === "null") && (r = r.parent), r && r.type === "property" && n > (r.colonOffset || 0)) {
      var o = r.keyNode.value, l = r.valueNode;
      if ((!l || n <= l.offset + l.length) && r.parent) {
        var u = tn(r.parent);
        this.contributions.forEach(function(f) {
          var h = f.collectValueCompletions(i.uri, u, o, s);
          h && a.push(h);
        });
      }
    }
  }, e.prototype.addSchemaValueCompletions = function(t, r, n, i) {
    var s = this;
    typeof t == "object" && (this.addEnumValueCompletions(t, r, n), this.addDefaultValueCompletions(t, r, n), this.collectTypes(t, i), Array.isArray(t.allOf) && t.allOf.forEach(function(a) {
      return s.addSchemaValueCompletions(a, r, n, i);
    }), Array.isArray(t.anyOf) && t.anyOf.forEach(function(a) {
      return s.addSchemaValueCompletions(a, r, n, i);
    }), Array.isArray(t.oneOf) && t.oneOf.forEach(function(a) {
      return s.addSchemaValueCompletions(a, r, n, i);
    }));
  }, e.prototype.addDefaultValueCompletions = function(t, r, n, i) {
    var s = this;
    i === void 0 && (i = 0);
    var a = !1;
    if (Ie(t.default)) {
      for (var o = t.type, l = t.default, u = i; u > 0; u--)
        l = [l], o = "array";
      n.add({
        kind: this.getSuggestionKind(o),
        label: this.getLabelForValue(l),
        insertText: this.getInsertTextForValue(l, r),
        insertTextFormat: te.Snippet,
        detail: _r("json.suggest.default", "Default value")
      }), a = !0;
    }
    Array.isArray(t.examples) && t.examples.forEach(function(f) {
      for (var h = t.type, d = f, g = i; g > 0; g--)
        d = [d], h = "array";
      n.add({
        kind: s.getSuggestionKind(h),
        label: s.getLabelForValue(d),
        insertText: s.getInsertTextForValue(d, r),
        insertTextFormat: te.Snippet
      }), a = !0;
    }), Array.isArray(t.defaultSnippets) && t.defaultSnippets.forEach(function(f) {
      var h = t.type, d = f.body, g = f.label, p, m;
      if (Ie(d)) {
        t.type;
        for (var v = i; v > 0; v--)
          d = [d];
        p = s.getInsertTextForSnippetValue(d, r), m = s.getFilterTextForSnippetValue(d), g = g || s.getLabelForSnippetValue(d);
      } else if (typeof f.bodyText == "string") {
        for (var y = "", x = "", b = "", v = i; v > 0; v--)
          y = y + b + `[
`, x = x + `
` + b + "]", b += "	", h = "array";
        p = y + b + f.bodyText.split(`
`).join(`
` + b) + x + r, g = g || p, m = p.replace(/[\n]/g, "");
      } else
        return;
      n.add({
        kind: s.getSuggestionKind(h),
        label: g,
        documentation: s.fromMarkup(f.markdownDescription) || f.description,
        insertText: p,
        insertTextFormat: te.Snippet,
        filterText: m
      }), a = !0;
    }), !a && typeof t.items == "object" && !Array.isArray(t.items) && i < 5 && this.addDefaultValueCompletions(t.items, r, n, i + 1);
  }, e.prototype.addEnumValueCompletions = function(t, r, n) {
    if (Ie(t.const) && n.add({
      kind: this.getSuggestionKind(t.type),
      label: this.getLabelForValue(t.const),
      insertText: this.getInsertTextForValue(t.const, r),
      insertTextFormat: te.Snippet,
      documentation: this.fromMarkup(t.markdownDescription) || t.description
    }), Array.isArray(t.enum))
      for (var i = 0, s = t.enum.length; i < s; i++) {
        var a = t.enum[i], o = this.fromMarkup(t.markdownDescription) || t.description;
        t.markdownEnumDescriptions && i < t.markdownEnumDescriptions.length && this.doesSupportMarkdown() ? o = this.fromMarkup(t.markdownEnumDescriptions[i]) : t.enumDescriptions && i < t.enumDescriptions.length && (o = t.enumDescriptions[i]), n.add({
          kind: this.getSuggestionKind(t.type),
          label: this.getLabelForValue(a),
          insertText: this.getInsertTextForValue(a, r),
          insertTextFormat: te.Snippet,
          documentation: o
        });
      }
  }, e.prototype.collectTypes = function(t, r) {
    if (!(Array.isArray(t.enum) || Ie(t.const))) {
      var n = t.type;
      Array.isArray(n) ? n.forEach(function(i) {
        return r[i] = !0;
      }) : n && (r[n] = !0);
    }
  }, e.prototype.addFillerValueCompletions = function(t, r, n) {
    t.object && n.add({
      kind: this.getSuggestionKind("object"),
      label: "{}",
      insertText: this.getInsertTextForGuessedValue({}, r),
      insertTextFormat: te.Snippet,
      detail: _r("defaults.object", "New object"),
      documentation: ""
    }), t.array && n.add({
      kind: this.getSuggestionKind("array"),
      label: "[]",
      insertText: this.getInsertTextForGuessedValue([], r),
      insertTextFormat: te.Snippet,
      detail: _r("defaults.array", "New array"),
      documentation: ""
    });
  }, e.prototype.addBooleanValueCompletion = function(t, r, n) {
    n.add({
      kind: this.getSuggestionKind("boolean"),
      label: t ? "true" : "false",
      insertText: this.getInsertTextForValue(t, r),
      insertTextFormat: te.Snippet,
      documentation: ""
    });
  }, e.prototype.addNullValueCompletion = function(t, r) {
    r.add({
      kind: this.getSuggestionKind("null"),
      label: "null",
      insertText: "null" + t,
      insertTextFormat: te.Snippet,
      documentation: ""
    });
  }, e.prototype.addDollarSchemaCompletions = function(t, r) {
    var n = this, i = this.schemaService.getRegisteredSchemaIds(function(s) {
      return s === "http" || s === "https";
    });
    i.forEach(function(s) {
      return r.add({
        kind: pe.Module,
        label: n.getLabelForValue(s),
        filterText: n.getFilterTextForValue(s),
        insertText: n.getInsertTextForValue(s, t),
        insertTextFormat: te.Snippet,
        documentation: ""
      });
    });
  }, e.prototype.getLabelForValue = function(t) {
    return JSON.stringify(t);
  }, e.prototype.getFilterTextForValue = function(t) {
    return JSON.stringify(t);
  }, e.prototype.getFilterTextForSnippetValue = function(t) {
    return JSON.stringify(t).replace(/\$\{\d+:([^}]+)\}|\$\d+/g, "$1");
  }, e.prototype.getLabelForSnippetValue = function(t) {
    var r = JSON.stringify(t);
    return r.replace(/\$\{\d+:([^}]+)\}|\$\d+/g, "$1");
  }, e.prototype.getInsertTextForPlainText = function(t) {
    return t.replace(/[\\\$\}]/g, "\\$&");
  }, e.prototype.getInsertTextForValue = function(t, r) {
    var n = JSON.stringify(t, null, "	");
    return n === "{}" ? "{$1}" + r : n === "[]" ? "[$1]" + r : this.getInsertTextForPlainText(n + r);
  }, e.prototype.getInsertTextForSnippetValue = function(t, r) {
    var n = function(i) {
      return typeof i == "string" && i[0] === "^" ? i.substr(1) : JSON.stringify(i);
    };
    return rn(t, "", n) + r;
  }, e.prototype.getInsertTextForGuessedValue = function(t, r) {
    switch (typeof t) {
      case "object":
        return t === null ? "${1:null}" + r : this.getInsertTextForValue(t, r);
      case "string":
        var n = JSON.stringify(t);
        return n = n.substr(1, n.length - 2), n = this.getInsertTextForPlainText(n), '"${1:' + n + '}"' + r;
      case "number":
      case "boolean":
        return "${1:" + JSON.stringify(t) + "}" + r;
    }
    return this.getInsertTextForValue(t, r);
  }, e.prototype.getSuggestionKind = function(t) {
    if (Array.isArray(t)) {
      var r = t;
      t = r.length > 0 ? r[0] : void 0;
    }
    if (!t)
      return pe.Value;
    switch (t) {
      case "string":
        return pe.Value;
      case "object":
        return pe.Module;
      case "property":
        return pe.Property;
      default:
        return pe.Value;
    }
  }, e.prototype.getLabelTextForMatchingNode = function(t, r) {
    switch (t.type) {
      case "array":
        return "[]";
      case "object":
        return "{}";
      default:
        var n = r.getText().substr(t.offset, t.length);
        return n;
    }
  }, e.prototype.getInsertTextForMatchingNode = function(t, r, n) {
    switch (t.type) {
      case "array":
        return this.getInsertTextForValue([], n);
      case "object":
        return this.getInsertTextForValue({}, n);
      default:
        var i = r.getText().substr(t.offset, t.length) + n;
        return this.getInsertTextForPlainText(i);
    }
  }, e.prototype.getInsertTextForProperty = function(t, r, n, i) {
    var s = this.getInsertTextForValue(t, "");
    if (!n)
      return s;
    var a = s + ": ", o, l = 0;
    if (r) {
      if (Array.isArray(r.defaultSnippets)) {
        if (r.defaultSnippets.length === 1) {
          var u = r.defaultSnippets[0].body;
          Ie(u) && (o = this.getInsertTextForSnippetValue(u, ""));
        }
        l += r.defaultSnippets.length;
      }
      if (r.enum && (!o && r.enum.length === 1 && (o = this.getInsertTextForGuessedValue(r.enum[0], "")), l += r.enum.length), Ie(r.default) && (o || (o = this.getInsertTextForGuessedValue(r.default, "")), l++), Array.isArray(r.examples) && r.examples.length && (o || (o = this.getInsertTextForGuessedValue(r.examples[0], "")), l += r.examples.length), l === 0) {
        var f = Array.isArray(r.type) ? r.type[0] : r.type;
        switch (f || (r.properties ? f = "object" : r.items && (f = "array")), f) {
          case "boolean":
            o = "$1";
            break;
          case "string":
            o = '"$1"';
            break;
          case "object":
            o = "{$1}";
            break;
          case "array":
            o = "[$1]";
            break;
          case "number":
          case "integer":
            o = "${1:0}";
            break;
          case "null":
            o = "${1:null}";
            break;
          default:
            return s;
        }
      }
    }
    return (!o || l > 1) && (o = "$1"), a + o + i;
  }, e.prototype.getCurrentWord = function(t, r) {
    for (var n = r - 1, i = t.getText(); n >= 0 && ` 	
\r\v":{[,]}`.indexOf(i.charAt(n)) === -1; )
      n--;
    return i.substring(n + 1, r);
  }, e.prototype.evaluateSeparatorAfter = function(t, r) {
    var n = ht(t.getText(), !0);
    n.setPosition(r);
    var i = n.scan();
    switch (i) {
      case 5:
      case 2:
      case 4:
      case 17:
        return "";
      default:
        return ",";
    }
  }, e.prototype.findItemAtOffset = function(t, r, n) {
    for (var i = ht(r.getText(), !0), s = t.items, a = s.length - 1; a >= 0; a--) {
      var o = s[a];
      if (n > o.offset + o.length) {
        i.setPosition(o.offset + o.length);
        var l = i.scan();
        return l === 5 && n >= i.getTokenOffset() + i.getTokenLength() ? a + 1 : a;
      } else if (n >= o.offset)
        return a;
    }
    return 0;
  }, e.prototype.isInComment = function(t, r, n) {
    var i = ht(t.getText(), !1);
    i.setPosition(r);
    for (var s = i.scan(); s !== 17 && i.getTokenOffset() + i.getTokenLength() < n; )
      s = i.scan();
    return (s === 12 || s === 13) && i.getTokenOffset() <= n;
  }, e.prototype.fromMarkup = function(t) {
    if (t && this.doesSupportMarkdown())
      return {
        kind: Oe.Markdown,
        value: t
      };
  }, e.prototype.doesSupportMarkdown = function() {
    if (!Ie(this.supportsMarkdown)) {
      var t = this.clientCapabilities.textDocument && this.clientCapabilities.textDocument.completion;
      this.supportsMarkdown = t && t.completionItem && Array.isArray(t.completionItem.documentationFormat) && t.completionItem.documentationFormat.indexOf(Oe.Markdown) !== -1;
    }
    return this.supportsMarkdown;
  }, e.prototype.doesSupportsCommitCharacters = function() {
    if (!Ie(this.supportsCommitCharacters)) {
      var t = this.clientCapabilities.textDocument && this.clientCapabilities.textDocument.completion;
      this.supportsCommitCharacters = t && t.completionItem && !!t.completionItem.commitCharactersSupport;
    }
    return this.supportsCommitCharacters;
  }, e;
}(), Il = function() {
  function e(t, r, n) {
    r === void 0 && (r = []), this.schemaService = t, this.contributions = r, this.promise = n || Promise;
  }
  return e.prototype.doHover = function(t, r, n) {
    var i = t.offsetAt(r), s = n.getNodeFromOffset(i);
    if (!s || (s.type === "object" || s.type === "array") && i > s.offset + 1 && i < s.offset + s.length - 1)
      return this.promise.resolve(null);
    var a = s;
    if (s.type === "string") {
      var o = s.parent;
      if (o && o.type === "property" && o.keyNode === s && (s = o.valueNode, !s))
        return this.promise.resolve(null);
    }
    for (var l = G.create(t.positionAt(a.offset), t.positionAt(a.offset + a.length)), u = function(p) {
      var m = {
        contents: p,
        range: l
      };
      return m;
    }, f = tn(s), h = this.contributions.length - 1; h >= 0; h--) {
      var d = this.contributions[h], g = d.getInfoContribution(t.uri, f);
      if (g)
        return g.then(function(p) {
          return u(p);
        });
    }
    return this.schemaService.getSchemaForResource(t.uri, n).then(function(p) {
      if (p && s) {
        var m = n.getMatchingSchemas(p.schema, s.offset), v = void 0, y = void 0, x = void 0, b = void 0;
        m.every(function(M) {
          if (M.node === s && !M.inverted && M.schema && (v = v || M.schema.title, y = y || M.schema.markdownDescription || Sr(M.schema.description), M.schema.enum)) {
            var A = M.schema.enum.indexOf(Ye(s));
            M.schema.markdownEnumDescriptions ? x = M.schema.markdownEnumDescriptions[A] : M.schema.enumDescriptions && (x = Sr(M.schema.enumDescriptions[A])), x && (b = M.schema.enum[A], typeof b != "string" && (b = JSON.stringify(b)));
          }
          return !0;
        });
        var S = "";
        return v && (S = Sr(v)), y && (S.length > 0 && (S += `

`), S += y), x && (S.length > 0 && (S += `

`), S += "`".concat(Vl(b), "`: ").concat(x)), u([S]);
      }
      return null;
    });
  }, e;
}();
function Sr(e) {
  if (e) {
    var t = e.replace(/([^\n\r])(\r?\n)([^\n\r])/gm, `$1

$3`);
    return t.replace(/[\\`*_{}[\]()#+\-.!]/g, "\\$&");
  }
}
function Vl(e) {
  return e.indexOf("`") !== -1 ? "`` " + e + " ``" : e;
}
var Dl = Rt(), Ol = function() {
  function e(t, r) {
    this.jsonSchemaService = t, this.promise = r, this.validationEnabled = !0;
  }
  return e.prototype.configure = function(t) {
    t && (this.validationEnabled = t.validate !== !1, this.commentSeverity = t.allowComments ? void 0 : ve.Error);
  }, e.prototype.doValidation = function(t, r, n, i) {
    var s = this;
    if (!this.validationEnabled)
      return this.promise.resolve([]);
    var a = [], o = {}, l = function(d) {
      var g = d.range.start.line + " " + d.range.start.character + " " + d.message;
      o[g] || (o[g] = !0, a.push(d));
    }, u = function(d) {
      var g = n != null && n.trailingCommas ? Dt(n.trailingCommas) : ve.Error, p = n != null && n.comments ? Dt(n.comments) : s.commentSeverity, m = n != null && n.schemaValidation ? Dt(n.schemaValidation) : ve.Warning, v = n != null && n.schemaRequest ? Dt(n.schemaRequest) : ve.Warning;
      if (d) {
        if (d.errors.length && r.root && v) {
          var y = r.root, x = y.type === "object" ? y.properties[0] : void 0;
          if (x && x.keyNode.value === "$schema") {
            var b = x.valueNode || x, S = G.create(t.positionAt(b.offset), t.positionAt(b.offset + b.length));
            l(De.create(S, d.errors[0], v, H.SchemaResolveError));
          } else {
            var S = G.create(t.positionAt(y.offset), t.positionAt(y.offset + 1));
            l(De.create(S, d.errors[0], v, H.SchemaResolveError));
          }
        } else if (m) {
          var M = r.validate(t, d.schema, m);
          M && M.forEach(l);
        }
        Zs(d.schema) && (p = void 0), Qs(d.schema) && (g = void 0);
      }
      for (var A = 0, C = r.syntaxErrors; A < C.length; A++) {
        var _ = C[A];
        if (_.code === H.TrailingComma) {
          if (typeof g != "number")
            continue;
          _.severity = g;
        }
        l(_);
      }
      if (typeof p == "number") {
        var w = Dl("InvalidCommentToken", "Comments are not permitted in JSON.");
        r.comments.forEach(function(N) {
          l(De.create(N, w, p, H.CommentNotPermitted));
        });
      }
      return a;
    };
    if (i) {
      var f = i.id || "schemaservice://untitled/" + jl++, h = this.jsonSchemaService.registerExternalSchema(f, [], i);
      return h.getResolvedSchema().then(function(d) {
        return u(d);
      });
    }
    return this.jsonSchemaService.getSchemaForResource(t.uri, r).then(function(d) {
      return u(d);
    });
  }, e.prototype.getLanguageStatus = function(t, r) {
    return { schemas: this.jsonSchemaService.getSchemaURIsForResource(t.uri, r) };
  }, e;
}(), jl = 0;
function Zs(e) {
  if (e && typeof e == "object") {
    if (Te(e.allowComments))
      return e.allowComments;
    if (e.allOf)
      for (var t = 0, r = e.allOf; t < r.length; t++) {
        var n = r[t], i = Zs(n);
        if (Te(i))
          return i;
      }
  }
}
function Qs(e) {
  if (e && typeof e == "object") {
    if (Te(e.allowTrailingCommas))
      return e.allowTrailingCommas;
    var t = e;
    if (Te(t.allowsTrailingCommas))
      return t.allowsTrailingCommas;
    if (e.allOf)
      for (var r = 0, n = e.allOf; r < n.length; r++) {
        var i = n[r], s = Qs(i);
        if (Te(s))
          return s;
      }
  }
}
function Dt(e) {
  switch (e) {
    case "error":
      return ve.Error;
    case "warning":
      return ve.Warning;
    case "ignore":
      return;
  }
}
var xs = 48, Ul = 57, Bl = 65, Ot = 97, $l = 102;
function Q(e) {
  return e < xs ? 0 : e <= Ul ? e - xs : (e < Ot && (e += Ot - Bl), e >= Ot && e <= $l ? e - Ot + 10 : 0);
}
function ql(e) {
  if (e[0] === "#")
    switch (e.length) {
      case 4:
        return {
          red: Q(e.charCodeAt(1)) * 17 / 255,
          green: Q(e.charCodeAt(2)) * 17 / 255,
          blue: Q(e.charCodeAt(3)) * 17 / 255,
          alpha: 1
        };
      case 5:
        return {
          red: Q(e.charCodeAt(1)) * 17 / 255,
          green: Q(e.charCodeAt(2)) * 17 / 255,
          blue: Q(e.charCodeAt(3)) * 17 / 255,
          alpha: Q(e.charCodeAt(4)) * 17 / 255
        };
      case 7:
        return {
          red: (Q(e.charCodeAt(1)) * 16 + Q(e.charCodeAt(2))) / 255,
          green: (Q(e.charCodeAt(3)) * 16 + Q(e.charCodeAt(4))) / 255,
          blue: (Q(e.charCodeAt(5)) * 16 + Q(e.charCodeAt(6))) / 255,
          alpha: 1
        };
      case 9:
        return {
          red: (Q(e.charCodeAt(1)) * 16 + Q(e.charCodeAt(2))) / 255,
          green: (Q(e.charCodeAt(3)) * 16 + Q(e.charCodeAt(4))) / 255,
          blue: (Q(e.charCodeAt(5)) * 16 + Q(e.charCodeAt(6))) / 255,
          alpha: (Q(e.charCodeAt(7)) * 16 + Q(e.charCodeAt(8))) / 255
        };
    }
}
var Wl = function() {
  function e(t) {
    this.schemaService = t;
  }
  return e.prototype.findDocumentSymbols = function(t, r, n) {
    var i = this;
    n === void 0 && (n = { resultLimit: Number.MAX_VALUE });
    var s = r.root;
    if (!s)
      return [];
    var a = n.resultLimit || Number.MAX_VALUE, o = t.uri;
    if ((o === "vscode://defaultsettings/keybindings.json" || Lt(o.toLowerCase(), "/user/keybindings.json")) && s.type === "array") {
      for (var l = [], u = 0, f = s.items; u < f.length; u++) {
        var h = f[u];
        if (h.type === "object")
          for (var d = 0, g = h.properties; d < g.length; d++) {
            var p = g[d];
            if (p.keyNode.value === "key" && p.valueNode) {
              var m = Ct.create(t.uri, $e(t, h));
              if (l.push({ name: Ye(p.valueNode), kind: Ee.Function, location: m }), a--, a <= 0)
                return n && n.onResultLimitExceeded && n.onResultLimitExceeded(o), l;
            }
          }
      }
      return l;
    }
    for (var v = [
      { node: s, containerName: "" }
    ], y = 0, x = !1, b = [], S = function(A, C) {
      A.type === "array" ? A.items.forEach(function(_) {
        _ && v.push({ node: _, containerName: C });
      }) : A.type === "object" && A.properties.forEach(function(_) {
        var w = _.valueNode;
        if (w)
          if (a > 0) {
            a--;
            var N = Ct.create(t.uri, $e(t, _)), k = C ? C + "." + _.keyNode.value : _.keyNode.value;
            b.push({ name: i.getKeyLabel(_), kind: i.getSymbolKind(w.type), location: N, containerName: C }), v.push({ node: w, containerName: k });
          } else
            x = !0;
      });
    }; y < v.length; ) {
      var M = v[y++];
      S(M.node, M.containerName);
    }
    return x && n && n.onResultLimitExceeded && n.onResultLimitExceeded(o), b;
  }, e.prototype.findDocumentSymbols2 = function(t, r, n) {
    var i = this;
    n === void 0 && (n = { resultLimit: Number.MAX_VALUE });
    var s = r.root;
    if (!s)
      return [];
    var a = n.resultLimit || Number.MAX_VALUE, o = t.uri;
    if ((o === "vscode://defaultsettings/keybindings.json" || Lt(o.toLowerCase(), "/user/keybindings.json")) && s.type === "array") {
      for (var l = [], u = 0, f = s.items; u < f.length; u++) {
        var h = f[u];
        if (h.type === "object")
          for (var d = 0, g = h.properties; d < g.length; d++) {
            var p = g[d];
            if (p.keyNode.value === "key" && p.valueNode) {
              var m = $e(t, h), v = $e(t, p.keyNode);
              if (l.push({ name: Ye(p.valueNode), kind: Ee.Function, range: m, selectionRange: v }), a--, a <= 0)
                return n && n.onResultLimitExceeded && n.onResultLimitExceeded(o), l;
            }
          }
      }
      return l;
    }
    for (var y = [], x = [
      { node: s, result: y }
    ], b = 0, S = !1, M = function(C, _) {
      C.type === "array" ? C.items.forEach(function(w, N) {
        if (w)
          if (a > 0) {
            a--;
            var k = $e(t, w), L = k, F = String(N), D = { name: F, kind: i.getSymbolKind(w.type), range: k, selectionRange: L, children: [] };
            _.push(D), x.push({ result: D.children, node: w });
          } else
            S = !0;
      }) : C.type === "object" && C.properties.forEach(function(w) {
        var N = w.valueNode;
        if (N)
          if (a > 0) {
            a--;
            var k = $e(t, w), L = $e(t, w.keyNode), F = [], D = { name: i.getKeyLabel(w), kind: i.getSymbolKind(N.type), range: k, selectionRange: L, children: F, detail: i.getDetail(N) };
            _.push(D), x.push({ result: F, node: N });
          } else
            S = !0;
      });
    }; b < x.length; ) {
      var A = x[b++];
      M(A.node, A.result);
    }
    return S && n && n.onResultLimitExceeded && n.onResultLimitExceeded(o), y;
  }, e.prototype.getSymbolKind = function(t) {
    switch (t) {
      case "object":
        return Ee.Module;
      case "string":
        return Ee.String;
      case "number":
        return Ee.Number;
      case "array":
        return Ee.Array;
      case "boolean":
        return Ee.Boolean;
      default:
        return Ee.Variable;
    }
  }, e.prototype.getKeyLabel = function(t) {
    var r = t.keyNode.value;
    return r && (r = r.replace(/[\n]/g, "↵")), r && r.trim() ? r : '"'.concat(r, '"');
  }, e.prototype.getDetail = function(t) {
    if (t) {
      if (t.type === "boolean" || t.type === "number" || t.type === "null" || t.type === "string")
        return String(t.value);
      if (t.type === "array")
        return t.children.length ? void 0 : "[]";
      if (t.type === "object")
        return t.children.length ? void 0 : "{}";
    }
  }, e.prototype.findDocumentColors = function(t, r, n) {
    return this.schemaService.getSchemaForResource(t.uri, r).then(function(i) {
      var s = [];
      if (i)
        for (var a = n && typeof n.resultLimit == "number" ? n.resultLimit : Number.MAX_VALUE, o = r.getMatchingSchemas(i.schema), l = {}, u = 0, f = o; u < f.length; u++) {
          var h = f[u];
          if (!h.inverted && h.schema && (h.schema.format === "color" || h.schema.format === "color-hex") && h.node && h.node.type === "string") {
            var d = String(h.node.offset);
            if (!l[d]) {
              var g = ql(Ye(h.node));
              if (g) {
                var p = $e(t, h.node);
                s.push({ color: g, range: p });
              }
              if (l[d] = !0, a--, a <= 0)
                return n && n.onResultLimitExceeded && n.onResultLimitExceeded(t.uri), s;
            }
          }
        }
      return s;
    });
  }, e.prototype.getColorPresentations = function(t, r, n, i) {
    var s = [], a = Math.round(n.red * 255), o = Math.round(n.green * 255), l = Math.round(n.blue * 255);
    function u(h) {
      var d = h.toString(16);
      return d.length !== 2 ? "0" + d : d;
    }
    var f;
    return n.alpha === 1 ? f = "#".concat(u(a)).concat(u(o)).concat(u(l)) : f = "#".concat(u(a)).concat(u(o)).concat(u(l)).concat(u(Math.round(n.alpha * 255))), s.push({ label: f, textEdit: Ne.replace(i, JSON.stringify(f)) }), s;
  }, e;
}();
function $e(e, t) {
  return G.create(e.positionAt(t.offset), e.positionAt(t.offset + t.length));
}
var B = Rt(), nn = {
  schemaAssociations: [],
  schemas: {
    "http://json-schema.org/schema#": {
      $ref: "http://json-schema.org/draft-07/schema#"
    },
    "http://json-schema.org/draft-04/schema#": {
      $schema: "http://json-schema.org/draft-04/schema#",
      definitions: {
        schemaArray: {
          type: "array",
          minItems: 1,
          items: {
            $ref: "#"
          }
        },
        positiveInteger: {
          type: "integer",
          minimum: 0
        },
        positiveIntegerDefault0: {
          allOf: [
            {
              $ref: "#/definitions/positiveInteger"
            },
            {
              default: 0
            }
          ]
        },
        simpleTypes: {
          type: "string",
          enum: [
            "array",
            "boolean",
            "integer",
            "null",
            "number",
            "object",
            "string"
          ]
        },
        stringArray: {
          type: "array",
          items: {
            type: "string"
          },
          minItems: 1,
          uniqueItems: !0
        }
      },
      type: "object",
      properties: {
        id: {
          type: "string",
          format: "uri"
        },
        $schema: {
          type: "string",
          format: "uri"
        },
        title: {
          type: "string"
        },
        description: {
          type: "string"
        },
        default: {},
        multipleOf: {
          type: "number",
          minimum: 0,
          exclusiveMinimum: !0
        },
        maximum: {
          type: "number"
        },
        exclusiveMaximum: {
          type: "boolean",
          default: !1
        },
        minimum: {
          type: "number"
        },
        exclusiveMinimum: {
          type: "boolean",
          default: !1
        },
        maxLength: {
          allOf: [
            {
              $ref: "#/definitions/positiveInteger"
            }
          ]
        },
        minLength: {
          allOf: [
            {
              $ref: "#/definitions/positiveIntegerDefault0"
            }
          ]
        },
        pattern: {
          type: "string",
          format: "regex"
        },
        additionalItems: {
          anyOf: [
            {
              type: "boolean"
            },
            {
              $ref: "#"
            }
          ],
          default: {}
        },
        items: {
          anyOf: [
            {
              $ref: "#"
            },
            {
              $ref: "#/definitions/schemaArray"
            }
          ],
          default: {}
        },
        maxItems: {
          allOf: [
            {
              $ref: "#/definitions/positiveInteger"
            }
          ]
        },
        minItems: {
          allOf: [
            {
              $ref: "#/definitions/positiveIntegerDefault0"
            }
          ]
        },
        uniqueItems: {
          type: "boolean",
          default: !1
        },
        maxProperties: {
          allOf: [
            {
              $ref: "#/definitions/positiveInteger"
            }
          ]
        },
        minProperties: {
          allOf: [
            {
              $ref: "#/definitions/positiveIntegerDefault0"
            }
          ]
        },
        required: {
          allOf: [
            {
              $ref: "#/definitions/stringArray"
            }
          ]
        },
        additionalProperties: {
          anyOf: [
            {
              type: "boolean"
            },
            {
              $ref: "#"
            }
          ],
          default: {}
        },
        definitions: {
          type: "object",
          additionalProperties: {
            $ref: "#"
          },
          default: {}
        },
        properties: {
          type: "object",
          additionalProperties: {
            $ref: "#"
          },
          default: {}
        },
        patternProperties: {
          type: "object",
          additionalProperties: {
            $ref: "#"
          },
          default: {}
        },
        dependencies: {
          type: "object",
          additionalProperties: {
            anyOf: [
              {
                $ref: "#"
              },
              {
                $ref: "#/definitions/stringArray"
              }
            ]
          }
        },
        enum: {
          type: "array",
          minItems: 1,
          uniqueItems: !0
        },
        type: {
          anyOf: [
            {
              $ref: "#/definitions/simpleTypes"
            },
            {
              type: "array",
              items: {
                $ref: "#/definitions/simpleTypes"
              },
              minItems: 1,
              uniqueItems: !0
            }
          ]
        },
        format: {
          anyOf: [
            {
              type: "string",
              enum: [
                "date-time",
                "uri",
                "email",
                "hostname",
                "ipv4",
                "ipv6",
                "regex"
              ]
            },
            {
              type: "string"
            }
          ]
        },
        allOf: {
          allOf: [
            {
              $ref: "#/definitions/schemaArray"
            }
          ]
        },
        anyOf: {
          allOf: [
            {
              $ref: "#/definitions/schemaArray"
            }
          ]
        },
        oneOf: {
          allOf: [
            {
              $ref: "#/definitions/schemaArray"
            }
          ]
        },
        not: {
          allOf: [
            {
              $ref: "#"
            }
          ]
        }
      },
      dependencies: {
        exclusiveMaximum: [
          "maximum"
        ],
        exclusiveMinimum: [
          "minimum"
        ]
      },
      default: {}
    },
    "http://json-schema.org/draft-07/schema#": {
      definitions: {
        schemaArray: {
          type: "array",
          minItems: 1,
          items: { $ref: "#" }
        },
        nonNegativeInteger: {
          type: "integer",
          minimum: 0
        },
        nonNegativeIntegerDefault0: {
          allOf: [
            { $ref: "#/definitions/nonNegativeInteger" },
            { default: 0 }
          ]
        },
        simpleTypes: {
          enum: [
            "array",
            "boolean",
            "integer",
            "null",
            "number",
            "object",
            "string"
          ]
        },
        stringArray: {
          type: "array",
          items: { type: "string" },
          uniqueItems: !0,
          default: []
        }
      },
      type: ["object", "boolean"],
      properties: {
        $id: {
          type: "string",
          format: "uri-reference"
        },
        $schema: {
          type: "string",
          format: "uri"
        },
        $ref: {
          type: "string",
          format: "uri-reference"
        },
        $comment: {
          type: "string"
        },
        title: {
          type: "string"
        },
        description: {
          type: "string"
        },
        default: !0,
        readOnly: {
          type: "boolean",
          default: !1
        },
        examples: {
          type: "array",
          items: !0
        },
        multipleOf: {
          type: "number",
          exclusiveMinimum: 0
        },
        maximum: {
          type: "number"
        },
        exclusiveMaximum: {
          type: "number"
        },
        minimum: {
          type: "number"
        },
        exclusiveMinimum: {
          type: "number"
        },
        maxLength: { $ref: "#/definitions/nonNegativeInteger" },
        minLength: { $ref: "#/definitions/nonNegativeIntegerDefault0" },
        pattern: {
          type: "string",
          format: "regex"
        },
        additionalItems: { $ref: "#" },
        items: {
          anyOf: [
            { $ref: "#" },
            { $ref: "#/definitions/schemaArray" }
          ],
          default: !0
        },
        maxItems: { $ref: "#/definitions/nonNegativeInteger" },
        minItems: { $ref: "#/definitions/nonNegativeIntegerDefault0" },
        uniqueItems: {
          type: "boolean",
          default: !1
        },
        contains: { $ref: "#" },
        maxProperties: { $ref: "#/definitions/nonNegativeInteger" },
        minProperties: { $ref: "#/definitions/nonNegativeIntegerDefault0" },
        required: { $ref: "#/definitions/stringArray" },
        additionalProperties: { $ref: "#" },
        definitions: {
          type: "object",
          additionalProperties: { $ref: "#" },
          default: {}
        },
        properties: {
          type: "object",
          additionalProperties: { $ref: "#" },
          default: {}
        },
        patternProperties: {
          type: "object",
          additionalProperties: { $ref: "#" },
          propertyNames: { format: "regex" },
          default: {}
        },
        dependencies: {
          type: "object",
          additionalProperties: {
            anyOf: [
              { $ref: "#" },
              { $ref: "#/definitions/stringArray" }
            ]
          }
        },
        propertyNames: { $ref: "#" },
        const: !0,
        enum: {
          type: "array",
          items: !0,
          minItems: 1,
          uniqueItems: !0
        },
        type: {
          anyOf: [
            { $ref: "#/definitions/simpleTypes" },
            {
              type: "array",
              items: { $ref: "#/definitions/simpleTypes" },
              minItems: 1,
              uniqueItems: !0
            }
          ]
        },
        format: { type: "string" },
        contentMediaType: { type: "string" },
        contentEncoding: { type: "string" },
        if: { $ref: "#" },
        then: { $ref: "#" },
        else: { $ref: "#" },
        allOf: { $ref: "#/definitions/schemaArray" },
        anyOf: { $ref: "#/definitions/schemaArray" },
        oneOf: { $ref: "#/definitions/schemaArray" },
        not: { $ref: "#" }
      },
      default: !0
    }
  }
}, Hl = {
  id: B("schema.json.id", "A unique identifier for the schema."),
  $schema: B("schema.json.$schema", "The schema to verify this document against."),
  title: B("schema.json.title", "A descriptive title of the element."),
  description: B("schema.json.description", "A long description of the element. Used in hover menus and suggestions."),
  default: B("schema.json.default", "A default value. Used by suggestions."),
  multipleOf: B("schema.json.multipleOf", "A number that should cleanly divide the current value (i.e. have no remainder)."),
  maximum: B("schema.json.maximum", "The maximum numerical value, inclusive by default."),
  exclusiveMaximum: B("schema.json.exclusiveMaximum", "Makes the maximum property exclusive."),
  minimum: B("schema.json.minimum", "The minimum numerical value, inclusive by default."),
  exclusiveMinimum: B("schema.json.exclusiveMininum", "Makes the minimum property exclusive."),
  maxLength: B("schema.json.maxLength", "The maximum length of a string."),
  minLength: B("schema.json.minLength", "The minimum length of a string."),
  pattern: B("schema.json.pattern", "A regular expression to match the string against. It is not implicitly anchored."),
  additionalItems: B("schema.json.additionalItems", "For arrays, only when items is set as an array. If it is a schema, then this schema validates items after the ones specified by the items array. If it is false, then additional items will cause validation to fail."),
  items: B("schema.json.items", "For arrays. Can either be a schema to validate every element against or an array of schemas to validate each item against in order (the first schema will validate the first element, the second schema will validate the second element, and so on."),
  maxItems: B("schema.json.maxItems", "The maximum number of items that can be inside an array. Inclusive."),
  minItems: B("schema.json.minItems", "The minimum number of items that can be inside an array. Inclusive."),
  uniqueItems: B("schema.json.uniqueItems", "If all of the items in the array must be unique. Defaults to false."),
  maxProperties: B("schema.json.maxProperties", "The maximum number of properties an object can have. Inclusive."),
  minProperties: B("schema.json.minProperties", "The minimum number of properties an object can have. Inclusive."),
  required: B("schema.json.required", "An array of strings that lists the names of all properties required on this object."),
  additionalProperties: B("schema.json.additionalProperties", "Either a schema or a boolean. If a schema, then used to validate all properties not matched by 'properties' or 'patternProperties'. If false, then any properties not matched by either will cause this schema to fail."),
  definitions: B("schema.json.definitions", "Not used for validation. Place subschemas here that you wish to reference inline with $ref."),
  properties: B("schema.json.properties", "A map of property names to schemas for each property."),
  patternProperties: B("schema.json.patternProperties", "A map of regular expressions on property names to schemas for matching properties."),
  dependencies: B("schema.json.dependencies", "A map of property names to either an array of property names or a schema. An array of property names means the property named in the key depends on the properties in the array being present in the object in order to be valid. If the value is a schema, then the schema is only applied to the object if the property in the key exists on the object."),
  enum: B("schema.json.enum", "The set of literal values that are valid."),
  type: B("schema.json.type", "Either a string of one of the basic schema types (number, integer, null, array, object, boolean, string) or an array of strings specifying a subset of those types."),
  format: B("schema.json.format", "Describes the format expected for the value."),
  allOf: B("schema.json.allOf", "An array of schemas, all of which must match."),
  anyOf: B("schema.json.anyOf", "An array of schemas, where at least one must match."),
  oneOf: B("schema.json.oneOf", "An array of schemas, exactly one of which must match."),
  not: B("schema.json.not", "A schema which must not match."),
  $id: B("schema.json.$id", "A unique identifier for the schema."),
  $ref: B("schema.json.$ref", "Reference a definition hosted on any location."),
  $comment: B("schema.json.$comment", "Comments from schema authors to readers or maintainers of the schema."),
  readOnly: B("schema.json.readOnly", "Indicates that the value of the instance is managed exclusively by the owning authority."),
  examples: B("schema.json.examples", "Sample JSON values associated with a particular schema, for the purpose of illustrating usage."),
  contains: B("schema.json.contains", 'An array instance is valid against "contains" if at least one of its elements is valid against the given schema.'),
  propertyNames: B("schema.json.propertyNames", "If the instance is an object, this keyword validates if every property name in the instance validates against the provided schema."),
  const: B("schema.json.const", "An instance validates successfully against this keyword if its value is equal to the value of the keyword."),
  contentMediaType: B("schema.json.contentMediaType", "Describes the media type of a string property."),
  contentEncoding: B("schema.json.contentEncoding", "Describes the content encoding of a string property."),
  if: B("schema.json.if", 'The validation outcome of the "if" subschema controls which of the "then" or "else" keywords are evaluated.'),
  then: B("schema.json.then", 'The "if" subschema is used for validation when the "if" subschema succeeds.'),
  else: B("schema.json.else", 'The "else" subschema is used for validation when the "if" subschema fails.')
};
for (ws in nn.schemas) {
  jt = nn.schemas[ws];
  for (ut in jt.properties)
    Ut = jt.properties[ut], typeof Ut == "boolean" && (Ut = jt.properties[ut] = {}), Ar = Hl[ut], Ar ? Ut.description = Ar : console.log("".concat(ut, ": localize('schema.json.").concat(ut, `', "")`));
}
var jt, Ut, Ar, ut, ws, Ys;
Ys = (() => {
  var e = { 470: (n) => {
    function i(o) {
      if (typeof o != "string")
        throw new TypeError("Path must be a string. Received " + JSON.stringify(o));
    }
    function s(o, l) {
      for (var u, f = "", h = 0, d = -1, g = 0, p = 0; p <= o.length; ++p) {
        if (p < o.length)
          u = o.charCodeAt(p);
        else {
          if (u === 47)
            break;
          u = 47;
        }
        if (u === 47) {
          if (!(d === p - 1 || g === 1))
            if (d !== p - 1 && g === 2) {
              if (f.length < 2 || h !== 2 || f.charCodeAt(f.length - 1) !== 46 || f.charCodeAt(f.length - 2) !== 46) {
                if (f.length > 2) {
                  var m = f.lastIndexOf("/");
                  if (m !== f.length - 1) {
                    m === -1 ? (f = "", h = 0) : h = (f = f.slice(0, m)).length - 1 - f.lastIndexOf("/"), d = p, g = 0;
                    continue;
                  }
                } else if (f.length === 2 || f.length === 1) {
                  f = "", h = 0, d = p, g = 0;
                  continue;
                }
              }
              l && (f.length > 0 ? f += "/.." : f = "..", h = 2);
            } else
              f.length > 0 ? f += "/" + o.slice(d + 1, p) : f = o.slice(d + 1, p), h = p - d - 1;
          d = p, g = 0;
        } else
          u === 46 && g !== -1 ? ++g : g = -1;
      }
      return f;
    }
    var a = { resolve: function() {
      for (var o, l = "", u = !1, f = arguments.length - 1; f >= -1 && !u; f--) {
        var h;
        f >= 0 ? h = arguments[f] : (o === void 0 && (o = process.cwd()), h = o), i(h), h.length !== 0 && (l = h + "/" + l, u = h.charCodeAt(0) === 47);
      }
      return l = s(l, !u), u ? l.length > 0 ? "/" + l : "/" : l.length > 0 ? l : ".";
    }, normalize: function(o) {
      if (i(o), o.length === 0)
        return ".";
      var l = o.charCodeAt(0) === 47, u = o.charCodeAt(o.length - 1) === 47;
      return (o = s(o, !l)).length !== 0 || l || (o = "."), o.length > 0 && u && (o += "/"), l ? "/" + o : o;
    }, isAbsolute: function(o) {
      return i(o), o.length > 0 && o.charCodeAt(0) === 47;
    }, join: function() {
      if (arguments.length === 0)
        return ".";
      for (var o, l = 0; l < arguments.length; ++l) {
        var u = arguments[l];
        i(u), u.length > 0 && (o === void 0 ? o = u : o += "/" + u);
      }
      return o === void 0 ? "." : a.normalize(o);
    }, relative: function(o, l) {
      if (i(o), i(l), o === l || (o = a.resolve(o)) === (l = a.resolve(l)))
        return "";
      for (var u = 1; u < o.length && o.charCodeAt(u) === 47; ++u)
        ;
      for (var f = o.length, h = f - u, d = 1; d < l.length && l.charCodeAt(d) === 47; ++d)
        ;
      for (var g = l.length - d, p = h < g ? h : g, m = -1, v = 0; v <= p; ++v) {
        if (v === p) {
          if (g > p) {
            if (l.charCodeAt(d + v) === 47)
              return l.slice(d + v + 1);
            if (v === 0)
              return l.slice(d + v);
          } else
            h > p && (o.charCodeAt(u + v) === 47 ? m = v : v === 0 && (m = 0));
          break;
        }
        var y = o.charCodeAt(u + v);
        if (y !== l.charCodeAt(d + v))
          break;
        y === 47 && (m = v);
      }
      var x = "";
      for (v = u + m + 1; v <= f; ++v)
        v !== f && o.charCodeAt(v) !== 47 || (x.length === 0 ? x += ".." : x += "/..");
      return x.length > 0 ? x + l.slice(d + m) : (d += m, l.charCodeAt(d) === 47 && ++d, l.slice(d));
    }, _makeLong: function(o) {
      return o;
    }, dirname: function(o) {
      if (i(o), o.length === 0)
        return ".";
      for (var l = o.charCodeAt(0), u = l === 47, f = -1, h = !0, d = o.length - 1; d >= 1; --d)
        if ((l = o.charCodeAt(d)) === 47) {
          if (!h) {
            f = d;
            break;
          }
        } else
          h = !1;
      return f === -1 ? u ? "/" : "." : u && f === 1 ? "//" : o.slice(0, f);
    }, basename: function(o, l) {
      if (l !== void 0 && typeof l != "string")
        throw new TypeError('"ext" argument must be a string');
      i(o);
      var u, f = 0, h = -1, d = !0;
      if (l !== void 0 && l.length > 0 && l.length <= o.length) {
        if (l.length === o.length && l === o)
          return "";
        var g = l.length - 1, p = -1;
        for (u = o.length - 1; u >= 0; --u) {
          var m = o.charCodeAt(u);
          if (m === 47) {
            if (!d) {
              f = u + 1;
              break;
            }
          } else
            p === -1 && (d = !1, p = u + 1), g >= 0 && (m === l.charCodeAt(g) ? --g == -1 && (h = u) : (g = -1, h = p));
        }
        return f === h ? h = p : h === -1 && (h = o.length), o.slice(f, h);
      }
      for (u = o.length - 1; u >= 0; --u)
        if (o.charCodeAt(u) === 47) {
          if (!d) {
            f = u + 1;
            break;
          }
        } else
          h === -1 && (d = !1, h = u + 1);
      return h === -1 ? "" : o.slice(f, h);
    }, extname: function(o) {
      i(o);
      for (var l = -1, u = 0, f = -1, h = !0, d = 0, g = o.length - 1; g >= 0; --g) {
        var p = o.charCodeAt(g);
        if (p !== 47)
          f === -1 && (h = !1, f = g + 1), p === 46 ? l === -1 ? l = g : d !== 1 && (d = 1) : l !== -1 && (d = -1);
        else if (!h) {
          u = g + 1;
          break;
        }
      }
      return l === -1 || f === -1 || d === 0 || d === 1 && l === f - 1 && l === u + 1 ? "" : o.slice(l, f);
    }, format: function(o) {
      if (o === null || typeof o != "object")
        throw new TypeError('The "pathObject" argument must be of type Object. Received type ' + typeof o);
      return function(l, u) {
        var f = u.dir || u.root, h = u.base || (u.name || "") + (u.ext || "");
        return f ? f === u.root ? f + h : f + "/" + h : h;
      }(0, o);
    }, parse: function(o) {
      i(o);
      var l = { root: "", dir: "", base: "", ext: "", name: "" };
      if (o.length === 0)
        return l;
      var u, f = o.charCodeAt(0), h = f === 47;
      h ? (l.root = "/", u = 1) : u = 0;
      for (var d = -1, g = 0, p = -1, m = !0, v = o.length - 1, y = 0; v >= u; --v)
        if ((f = o.charCodeAt(v)) !== 47)
          p === -1 && (m = !1, p = v + 1), f === 46 ? d === -1 ? d = v : y !== 1 && (y = 1) : d !== -1 && (y = -1);
        else if (!m) {
          g = v + 1;
          break;
        }
      return d === -1 || p === -1 || y === 0 || y === 1 && d === p - 1 && d === g + 1 ? p !== -1 && (l.base = l.name = g === 0 && h ? o.slice(1, p) : o.slice(g, p)) : (g === 0 && h ? (l.name = o.slice(1, d), l.base = o.slice(1, p)) : (l.name = o.slice(g, d), l.base = o.slice(g, p)), l.ext = o.slice(d, p)), g > 0 ? l.dir = o.slice(0, g - 1) : h && (l.dir = "/"), l;
    }, sep: "/", delimiter: ":", win32: null, posix: null };
    a.posix = a, n.exports = a;
  }, 447: (n, i, s) => {
    var a;
    if (s.r(i), s.d(i, { URI: () => x, Utils: () => F }), typeof process == "object")
      a = process.platform === "win32";
    else if (typeof navigator == "object") {
      var o = navigator.userAgent;
      a = o.indexOf("Windows") >= 0;
    }
    var l, u, f = (l = function(P, T) {
      return (l = Object.setPrototypeOf || { __proto__: [] } instanceof Array && function(R, I) {
        R.__proto__ = I;
      } || function(R, I) {
        for (var j in I)
          Object.prototype.hasOwnProperty.call(I, j) && (R[j] = I[j]);
      })(P, T);
    }, function(P, T) {
      if (typeof T != "function" && T !== null)
        throw new TypeError("Class extends value " + String(T) + " is not a constructor or null");
      function R() {
        this.constructor = P;
      }
      l(P, T), P.prototype = T === null ? Object.create(T) : (R.prototype = T.prototype, new R());
    }), h = /^\w[\w\d+.-]*$/, d = /^\//, g = /^\/\//;
    function p(P, T) {
      if (!P.scheme && T)
        throw new Error('[UriError]: Scheme is missing: {scheme: "", authority: "'.concat(P.authority, '", path: "').concat(P.path, '", query: "').concat(P.query, '", fragment: "').concat(P.fragment, '"}'));
      if (P.scheme && !h.test(P.scheme))
        throw new Error("[UriError]: Scheme contains illegal characters.");
      if (P.path) {
        if (P.authority) {
          if (!d.test(P.path))
            throw new Error('[UriError]: If a URI contains an authority component, then the path component must either be empty or begin with a slash ("/") character');
        } else if (g.test(P.path))
          throw new Error('[UriError]: If a URI does not contain an authority component, then the path cannot begin with two slash characters ("//")');
      }
    }
    var m = "", v = "/", y = /^(([^:/?#]+?):)?(\/\/([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?/, x = function() {
      function P(T, R, I, j, U, q) {
        q === void 0 && (q = !1), typeof T == "object" ? (this.scheme = T.scheme || m, this.authority = T.authority || m, this.path = T.path || m, this.query = T.query || m, this.fragment = T.fragment || m) : (this.scheme = function(Le, ae) {
          return Le || ae ? Le : "file";
        }(T, q), this.authority = R || m, this.path = function(Le, ae) {
          switch (Le) {
            case "https":
            case "http":
            case "file":
              ae ? ae[0] !== v && (ae = v + ae) : ae = v;
          }
          return ae;
        }(this.scheme, I || m), this.query = j || m, this.fragment = U || m, p(this, q));
      }
      return P.isUri = function(T) {
        return T instanceof P || !!T && typeof T.authority == "string" && typeof T.fragment == "string" && typeof T.path == "string" && typeof T.query == "string" && typeof T.scheme == "string" && typeof T.fsPath == "string" && typeof T.with == "function" && typeof T.toString == "function";
      }, Object.defineProperty(P.prototype, "fsPath", { get: function() {
        return _(this, !1);
      }, enumerable: !1, configurable: !0 }), P.prototype.with = function(T) {
        if (!T)
          return this;
        var R = T.scheme, I = T.authority, j = T.path, U = T.query, q = T.fragment;
        return R === void 0 ? R = this.scheme : R === null && (R = m), I === void 0 ? I = this.authority : I === null && (I = m), j === void 0 ? j = this.path : j === null && (j = m), U === void 0 ? U = this.query : U === null && (U = m), q === void 0 ? q = this.fragment : q === null && (q = m), R === this.scheme && I === this.authority && j === this.path && U === this.query && q === this.fragment ? this : new S(R, I, j, U, q);
      }, P.parse = function(T, R) {
        R === void 0 && (R = !1);
        var I = y.exec(T);
        return I ? new S(I[2] || m, L(I[4] || m), L(I[5] || m), L(I[7] || m), L(I[9] || m), R) : new S(m, m, m, m, m);
      }, P.file = function(T) {
        var R = m;
        if (a && (T = T.replace(/\\/g, v)), T[0] === v && T[1] === v) {
          var I = T.indexOf(v, 2);
          I === -1 ? (R = T.substring(2), T = v) : (R = T.substring(2, I), T = T.substring(I) || v);
        }
        return new S("file", R, T, m, m);
      }, P.from = function(T) {
        var R = new S(T.scheme, T.authority, T.path, T.query, T.fragment);
        return p(R, !0), R;
      }, P.prototype.toString = function(T) {
        return T === void 0 && (T = !1), w(this, T);
      }, P.prototype.toJSON = function() {
        return this;
      }, P.revive = function(T) {
        if (T) {
          if (T instanceof P)
            return T;
          var R = new S(T);
          return R._formatted = T.external, R._fsPath = T._sep === b ? T.fsPath : null, R;
        }
        return T;
      }, P;
    }(), b = a ? 1 : void 0, S = function(P) {
      function T() {
        var R = P !== null && P.apply(this, arguments) || this;
        return R._formatted = null, R._fsPath = null, R;
      }
      return f(T, P), Object.defineProperty(T.prototype, "fsPath", { get: function() {
        return this._fsPath || (this._fsPath = _(this, !1)), this._fsPath;
      }, enumerable: !1, configurable: !0 }), T.prototype.toString = function(R) {
        return R === void 0 && (R = !1), R ? w(this, !0) : (this._formatted || (this._formatted = w(this, !1)), this._formatted);
      }, T.prototype.toJSON = function() {
        var R = { $mid: 1 };
        return this._fsPath && (R.fsPath = this._fsPath, R._sep = b), this._formatted && (R.external = this._formatted), this.path && (R.path = this.path), this.scheme && (R.scheme = this.scheme), this.authority && (R.authority = this.authority), this.query && (R.query = this.query), this.fragment && (R.fragment = this.fragment), R;
      }, T;
    }(x), M = ((u = {})[58] = "%3A", u[47] = "%2F", u[63] = "%3F", u[35] = "%23", u[91] = "%5B", u[93] = "%5D", u[64] = "%40", u[33] = "%21", u[36] = "%24", u[38] = "%26", u[39] = "%27", u[40] = "%28", u[41] = "%29", u[42] = "%2A", u[43] = "%2B", u[44] = "%2C", u[59] = "%3B", u[61] = "%3D", u[32] = "%20", u);
    function A(P, T) {
      for (var R = void 0, I = -1, j = 0; j < P.length; j++) {
        var U = P.charCodeAt(j);
        if (U >= 97 && U <= 122 || U >= 65 && U <= 90 || U >= 48 && U <= 57 || U === 45 || U === 46 || U === 95 || U === 126 || T && U === 47)
          I !== -1 && (R += encodeURIComponent(P.substring(I, j)), I = -1), R !== void 0 && (R += P.charAt(j));
        else {
          R === void 0 && (R = P.substr(0, j));
          var q = M[U];
          q !== void 0 ? (I !== -1 && (R += encodeURIComponent(P.substring(I, j)), I = -1), R += q) : I === -1 && (I = j);
        }
      }
      return I !== -1 && (R += encodeURIComponent(P.substring(I))), R !== void 0 ? R : P;
    }
    function C(P) {
      for (var T = void 0, R = 0; R < P.length; R++) {
        var I = P.charCodeAt(R);
        I === 35 || I === 63 ? (T === void 0 && (T = P.substr(0, R)), T += M[I]) : T !== void 0 && (T += P[R]);
      }
      return T !== void 0 ? T : P;
    }
    function _(P, T) {
      var R;
      return R = P.authority && P.path.length > 1 && P.scheme === "file" ? "//".concat(P.authority).concat(P.path) : P.path.charCodeAt(0) === 47 && (P.path.charCodeAt(1) >= 65 && P.path.charCodeAt(1) <= 90 || P.path.charCodeAt(1) >= 97 && P.path.charCodeAt(1) <= 122) && P.path.charCodeAt(2) === 58 ? T ? P.path.substr(1) : P.path[1].toLowerCase() + P.path.substr(2) : P.path, a && (R = R.replace(/\//g, "\\")), R;
    }
    function w(P, T) {
      var R = T ? C : A, I = "", j = P.scheme, U = P.authority, q = P.path, Le = P.query, ae = P.fragment;
      if (j && (I += j, I += ":"), (U || j === "file") && (I += v, I += v), U) {
        var be = U.indexOf("@");
        if (be !== -1) {
          var nt = U.substr(0, be);
          U = U.substr(be + 1), (be = nt.indexOf(":")) === -1 ? I += R(nt, !1) : (I += R(nt.substr(0, be), !1), I += ":", I += R(nt.substr(be + 1), !1)), I += "@";
        }
        (be = (U = U.toLowerCase()).indexOf(":")) === -1 ? I += R(U, !1) : (I += R(U.substr(0, be), !1), I += U.substr(be));
      }
      if (q) {
        if (q.length >= 3 && q.charCodeAt(0) === 47 && q.charCodeAt(2) === 58)
          (Ce = q.charCodeAt(1)) >= 65 && Ce <= 90 && (q = "/".concat(String.fromCharCode(Ce + 32), ":").concat(q.substr(3)));
        else if (q.length >= 2 && q.charCodeAt(1) === 58) {
          var Ce;
          (Ce = q.charCodeAt(0)) >= 65 && Ce <= 90 && (q = "".concat(String.fromCharCode(Ce + 32), ":").concat(q.substr(2)));
        }
        I += R(q, !0);
      }
      return Le && (I += "?", I += R(Le, !1)), ae && (I += "#", I += T ? ae : A(ae, !1)), I;
    }
    function N(P) {
      try {
        return decodeURIComponent(P);
      } catch {
        return P.length > 3 ? P.substr(0, 3) + N(P.substr(3)) : P;
      }
    }
    var k = /(%[0-9A-Za-z][0-9A-Za-z])+/g;
    function L(P) {
      return P.match(k) ? P.replace(k, function(T) {
        return N(T);
      }) : P;
    }
    var F, D = s(470), z = function(P, T, R) {
      if (R || arguments.length === 2)
        for (var I, j = 0, U = T.length; j < U; j++)
          !I && j in T || (I || (I = Array.prototype.slice.call(T, 0, j)), I[j] = T[j]);
      return P.concat(I || Array.prototype.slice.call(T));
    }, W = D.posix || D;
    (function(P) {
      P.joinPath = function(T) {
        for (var R = [], I = 1; I < arguments.length; I++)
          R[I - 1] = arguments[I];
        return T.with({ path: W.join.apply(W, z([T.path], R, !1)) });
      }, P.resolvePath = function(T) {
        for (var R = [], I = 1; I < arguments.length; I++)
          R[I - 1] = arguments[I];
        var j = T.path || "/";
        return T.with({ path: W.resolve.apply(W, z([j], R, !1)) });
      }, P.dirname = function(T) {
        var R = W.dirname(T.path);
        return R.length === 1 && R.charCodeAt(0) === 46 ? T : T.with({ path: R });
      }, P.basename = function(T) {
        return W.basename(T.path);
      }, P.extname = function(T) {
        return W.extname(T.path);
      };
    })(F || (F = {}));
  } }, t = {};
  function r(n) {
    if (t[n])
      return t[n].exports;
    var i = t[n] = { exports: {} };
    return e[n](i, i.exports, r), i.exports;
  }
  return r.d = (n, i) => {
    for (var s in i)
      r.o(i, s) && !r.o(n, s) && Object.defineProperty(n, s, { enumerable: !0, get: i[s] });
  }, r.o = (n, i) => Object.prototype.hasOwnProperty.call(n, i), r.r = (n) => {
    typeof Symbol < "u" && Symbol.toStringTag && Object.defineProperty(n, Symbol.toStringTag, { value: "Module" }), Object.defineProperty(n, "__esModule", { value: !0 });
  }, r(447);
})();
var { URI: pt, Utils: mu } = Ys;
function zl(e, t) {
  if (typeof e != "string")
    throw new TypeError("Expected a string");
  for (var r = String(e), n = "", i = t ? !!t.extended : !1, s = t ? !!t.globstar : !1, a = !1, o = t && typeof t.flags == "string" ? t.flags : "", l, u = 0, f = r.length; u < f; u++)
    switch (l = r[u], l) {
      case "/":
      case "$":
      case "^":
      case "+":
      case ".":
      case "(":
      case ")":
      case "=":
      case "!":
      case "|":
        n += "\\" + l;
        break;
      case "?":
        if (i) {
          n += ".";
          break;
        }
      case "[":
      case "]":
        if (i) {
          n += l;
          break;
        }
      case "{":
        if (i) {
          a = !0, n += "(";
          break;
        }
      case "}":
        if (i) {
          a = !1, n += ")";
          break;
        }
      case ",":
        if (a) {
          n += "|";
          break;
        }
        n += "\\" + l;
        break;
      case "*":
        for (var h = r[u - 1], d = 1; r[u + 1] === "*"; )
          d++, u++;
        var g = r[u + 1];
        if (!s)
          n += ".*";
        else {
          var p = d > 1 && (h === "/" || h === void 0 || h === "{" || h === ",") && (g === "/" || g === void 0 || g === "," || g === "}");
          p ? (g === "/" ? u++ : h === "/" && n.endsWith("\\/") && (n = n.substr(0, n.length - 2)), n += "((?:[^/]*(?:/|$))*)") : n += "([^/]*)";
        }
        break;
      default:
        n += l;
    }
  return (!o || !~o.indexOf("g")) && (n = "^" + n + "$"), new RegExp(n, o);
}
var ke = Rt(), Gl = "!", Jl = "/", Xl = function() {
  function e(t, r) {
    this.globWrappers = [];
    try {
      for (var n = 0, i = t; n < i.length; n++) {
        var s = i[n], a = s[0] !== Gl;
        a || (s = s.substring(1)), s.length > 0 && (s[0] === Jl && (s = s.substring(1)), this.globWrappers.push({
          regexp: zl("**/" + s, { extended: !0, globstar: !0 }),
          include: a
        }));
      }
      this.uris = r;
    } catch {
      this.globWrappers.length = 0, this.uris = [];
    }
  }
  return e.prototype.matchesPattern = function(t) {
    for (var r = !1, n = 0, i = this.globWrappers; n < i.length; n++) {
      var s = i[n], a = s.regexp, o = s.include;
      a.test(t) && (r = o);
    }
    return r;
  }, e.prototype.getURIs = function() {
    return this.uris;
  }, e;
}(), Zl = function() {
  function e(t, r, n) {
    this.service = t, this.uri = r, this.dependencies = /* @__PURE__ */ new Set(), this.anchors = void 0, n && (this.unresolvedSchema = this.service.promise.resolve(new xt(n)));
  }
  return e.prototype.getUnresolvedSchema = function() {
    return this.unresolvedSchema || (this.unresolvedSchema = this.service.loadSchema(this.uri)), this.unresolvedSchema;
  }, e.prototype.getResolvedSchema = function() {
    var t = this;
    return this.resolvedSchema || (this.resolvedSchema = this.getUnresolvedSchema().then(function(r) {
      return t.service.resolveSchemaContent(r, t);
    })), this.resolvedSchema;
  }, e.prototype.clearSchema = function() {
    var t = !!this.unresolvedSchema;
    return this.resolvedSchema = void 0, this.unresolvedSchema = void 0, this.dependencies.clear(), this.anchors = void 0, t;
  }, e;
}(), xt = function() {
  function e(t, r) {
    r === void 0 && (r = []), this.schema = t, this.errors = r;
  }
  return e;
}(), _s = function() {
  function e(t, r) {
    r === void 0 && (r = []), this.schema = t, this.errors = r;
  }
  return e.prototype.getSection = function(t) {
    var r = this.getSectionRecursive(t, this.schema);
    if (r)
      return ue(r);
  }, e.prototype.getSectionRecursive = function(t, r) {
    if (!r || typeof r == "boolean" || t.length === 0)
      return r;
    var n = t.shift();
    if (r.properties && typeof r.properties[n])
      return this.getSectionRecursive(t, r.properties[n]);
    if (r.patternProperties)
      for (var i = 0, s = Object.keys(r.patternProperties); i < s.length; i++) {
        var a = s[i], o = tr(a);
        if (o != null && o.test(n))
          return this.getSectionRecursive(t, r.patternProperties[a]);
      }
    else {
      if (typeof r.additionalProperties == "object")
        return this.getSectionRecursive(t, r.additionalProperties);
      if (n.match("[0-9]+")) {
        if (Array.isArray(r.items)) {
          var l = parseInt(n, 10);
          if (!isNaN(l) && r.items[l])
            return this.getSectionRecursive(t, r.items[l]);
        } else if (r.items)
          return this.getSectionRecursive(t, r.items);
      }
    }
  }, e;
}(), Ql = function() {
  function e(t, r, n) {
    this.contextService = r, this.requestService = t, this.promiseConstructor = n || Promise, this.callOnDispose = [], this.contributionSchemas = {}, this.contributionAssociations = [], this.schemasById = {}, this.filePatternAssociations = [], this.registeredSchemasIds = {};
  }
  return e.prototype.getRegisteredSchemaIds = function(t) {
    return Object.keys(this.registeredSchemasIds).filter(function(r) {
      var n = pt.parse(r).scheme;
      return n !== "schemaservice" && (!t || t(n));
    });
  }, Object.defineProperty(e.prototype, "promise", {
    get: function() {
      return this.promiseConstructor;
    },
    enumerable: !1,
    configurable: !0
  }), e.prototype.dispose = function() {
    for (; this.callOnDispose.length > 0; )
      this.callOnDispose.pop()();
  }, e.prototype.onResourceChange = function(t) {
    var r = this;
    this.cachedSchemaForResource = void 0;
    var n = !1;
    t = qe(t);
    for (var i = [t], s = Object.keys(this.schemasById).map(function(u) {
      return r.schemasById[u];
    }); i.length; )
      for (var a = i.pop(), o = 0; o < s.length; o++) {
        var l = s[o];
        l && (l.uri === a || l.dependencies.has(a)) && (l.uri !== a && i.push(l.uri), l.clearSchema() && (n = !0), s[o] = void 0);
      }
    return n;
  }, e.prototype.setSchemaContributions = function(t) {
    if (t.schemas) {
      var r = t.schemas;
      for (var n in r) {
        var i = qe(n);
        this.contributionSchemas[i] = this.addSchemaHandle(i, r[n]);
      }
    }
    if (Array.isArray(t.schemaAssociations))
      for (var s = t.schemaAssociations, a = 0, o = s; a < o.length; a++) {
        var l = o[a], u = l.uris.map(qe), f = this.addFilePatternAssociation(l.pattern, u);
        this.contributionAssociations.push(f);
      }
  }, e.prototype.addSchemaHandle = function(t, r) {
    var n = new Zl(this, t, r);
    return this.schemasById[t] = n, n;
  }, e.prototype.getOrAddSchemaHandle = function(t, r) {
    return this.schemasById[t] || this.addSchemaHandle(t, r);
  }, e.prototype.addFilePatternAssociation = function(t, r) {
    var n = new Xl(t, r);
    return this.filePatternAssociations.push(n), n;
  }, e.prototype.registerExternalSchema = function(t, r, n) {
    var i = qe(t);
    return this.registeredSchemasIds[i] = !0, this.cachedSchemaForResource = void 0, r && this.addFilePatternAssociation(r, [i]), n ? this.addSchemaHandle(i, n) : this.getOrAddSchemaHandle(i);
  }, e.prototype.clearExternalSchemas = function() {
    this.schemasById = {}, this.filePatternAssociations = [], this.registeredSchemasIds = {}, this.cachedSchemaForResource = void 0;
    for (var t in this.contributionSchemas)
      this.schemasById[t] = this.contributionSchemas[t], this.registeredSchemasIds[t] = !0;
    for (var r = 0, n = this.contributionAssociations; r < n.length; r++) {
      var i = n[r];
      this.filePatternAssociations.push(i);
    }
  }, e.prototype.getResolvedSchema = function(t) {
    var r = qe(t), n = this.schemasById[r];
    return n ? n.getResolvedSchema() : this.promise.resolve(void 0);
  }, e.prototype.loadSchema = function(t) {
    if (!this.requestService) {
      var r = ke("json.schema.norequestservice", "Unable to load schema from '{0}'. No schema request service available", Bt(t));
      return this.promise.resolve(new xt({}, [r]));
    }
    return this.requestService(t).then(function(n) {
      if (!n) {
        var i = ke("json.schema.nocontent", "Unable to load schema from '{0}': No content.", Bt(t));
        return new xt({}, [i]);
      }
      var s = {}, a = [];
      s = gl(n, a);
      var o = a.length ? [ke("json.schema.invalidFormat", "Unable to parse content from '{0}': Parse error at offset {1}.", Bt(t), a[0].offset)] : [];
      return new xt(s, o);
    }, function(n) {
      var i = n.toString(), s = n.toString().split("Error: ");
      return s.length > 1 && (i = s[1]), Lt(i, ".") && (i = i.substr(0, i.length - 1)), new xt({}, [ke("json.schema.nocontent", "Unable to load schema from '{0}': {1}.", Bt(t), i)]);
    });
  }, e.prototype.resolveSchemaContent = function(t, r) {
    var n = this, i = t.errors.slice(0), s = t.schema;
    if (s.$schema) {
      var a = qe(s.$schema);
      if (a === "http://json-schema.org/draft-03/schema")
        return this.promise.resolve(new _s({}, [ke("json.schema.draft03.notsupported", "Draft-03 schemas are not supported.")]));
      a === "https://json-schema.org/draft/2019-09/schema" ? i.push(ke("json.schema.draft201909.notsupported", "Draft 2019-09 schemas are not yet fully supported.")) : a === "https://json-schema.org/draft/2020-12/schema" && i.push(ke("json.schema.draft202012.notsupported", "Draft 2020-12 schemas are not yet fully supported."));
    }
    var o = this.contextService, l = function(m, v) {
      v = decodeURIComponent(v);
      var y = m;
      return v[0] === "/" && (v = v.substring(1)), v.split("/").some(function(x) {
        return x = x.replace(/~1/g, "/").replace(/~0/g, "~"), y = y[x], !y;
      }), y;
    }, u = function(m, v, y) {
      return v.anchors || (v.anchors = p(m)), v.anchors.get(y);
    }, f = function(m, v) {
      for (var y in v)
        v.hasOwnProperty(y) && !m.hasOwnProperty(y) && y !== "id" && y !== "$id" && (m[y] = v[y]);
    }, h = function(m, v, y, x) {
      var b;
      x === void 0 || x.length === 0 ? b = v : x.charAt(0) === "/" ? b = l(v, x) : b = u(v, y, x), b ? f(m, b) : i.push(ke("json.schema.invalidid", "$ref '{0}' in '{1}' can not be resolved.", x, y.uri));
    }, d = function(m, v, y, x) {
      o && !/^[A-Za-z][A-Za-z0-9+\-.+]*:\/\/.*/.test(v) && (v = o.resolveRelativePath(v, x.uri)), v = qe(v);
      var b = n.getOrAddSchemaHandle(v);
      return b.getUnresolvedSchema().then(function(S) {
        if (x.dependencies.add(v), S.errors.length) {
          var M = y ? v + "#" + y : v;
          i.push(ke("json.schema.problemloadingref", "Problems loading reference '{0}': {1}", M, S.errors[0]));
        }
        return h(m, S.schema, b, y), g(m, S.schema, b);
      });
    }, g = function(m, v, y) {
      var x = [];
      return n.traverseNodes(m, function(b) {
        for (var S = /* @__PURE__ */ new Set(); b.$ref; ) {
          var M = b.$ref, A = M.split("#", 2);
          if (delete b.$ref, A[0].length > 0) {
            x.push(d(b, A[0], A[1], y));
            return;
          } else if (!S.has(M)) {
            var C = A[1];
            h(b, v, y, C), S.add(M);
          }
        }
      }), n.promise.all(x);
    }, p = function(m) {
      var v = /* @__PURE__ */ new Map();
      return n.traverseNodes(m, function(y) {
        var x = y.$id || y.id;
        if (typeof x == "string" && x.charAt(0) === "#") {
          var b = x.substring(1);
          v.has(b) ? i.push(ke("json.schema.duplicateid", "Duplicate id declaration: '{0}'", x)) : v.set(b, y);
        }
      }), v;
    };
    return g(s, s, r).then(function(m) {
      return new _s(s, i);
    });
  }, e.prototype.traverseNodes = function(t, r) {
    if (!t || typeof t != "object")
      return Promise.resolve(null);
    for (var n = /* @__PURE__ */ new Set(), i = function() {
      for (var u = [], f = 0; f < arguments.length; f++)
        u[f] = arguments[f];
      for (var h = 0, d = u; h < d.length; h++) {
        var g = d[h];
        typeof g == "object" && o.push(g);
      }
    }, s = function() {
      for (var u = [], f = 0; f < arguments.length; f++)
        u[f] = arguments[f];
      for (var h = 0, d = u; h < d.length; h++) {
        var g = d[h];
        if (typeof g == "object")
          for (var p in g) {
            var m = p, v = g[m];
            typeof v == "object" && o.push(v);
          }
      }
    }, a = function() {
      for (var u = [], f = 0; f < arguments.length; f++)
        u[f] = arguments[f];
      for (var h = 0, d = u; h < d.length; h++) {
        var g = d[h];
        if (Array.isArray(g))
          for (var p = 0, m = g; p < m.length; p++) {
            var v = m[p];
            typeof v == "object" && o.push(v);
          }
      }
    }, o = [t], l = o.pop(); l; )
      n.has(l) || (n.add(l), r(l), i(l.items, l.additionalItems, l.additionalProperties, l.not, l.contains, l.propertyNames, l.if, l.then, l.else), s(l.definitions, l.properties, l.patternProperties, l.dependencies), a(l.anyOf, l.allOf, l.oneOf, l.items)), l = o.pop();
  }, e.prototype.getSchemaFromProperty = function(t, r) {
    var n, i;
    if (((n = r.root) === null || n === void 0 ? void 0 : n.type) === "object")
      for (var s = 0, a = r.root.properties; s < a.length; s++) {
        var o = a[s];
        if (o.keyNode.value === "$schema" && ((i = o.valueNode) === null || i === void 0 ? void 0 : i.type) === "string") {
          var l = o.valueNode.value;
          return this.contextService && !/^\w[\w\d+.-]*:/.test(l) && (l = this.contextService.resolveRelativePath(l, t)), l;
        }
      }
  }, e.prototype.getAssociatedSchemas = function(t) {
    for (var r = /* @__PURE__ */ Object.create(null), n = [], i = Kl(t), s = 0, a = this.filePatternAssociations; s < a.length; s++) {
      var o = a[s];
      if (o.matchesPattern(i))
        for (var l = 0, u = o.getURIs(); l < u.length; l++) {
          var f = u[l];
          r[f] || (n.push(f), r[f] = !0);
        }
    }
    return n;
  }, e.prototype.getSchemaURIsForResource = function(t, r) {
    var n = r && this.getSchemaFromProperty(t, r);
    return n ? [n] : this.getAssociatedSchemas(t);
  }, e.prototype.getSchemaForResource = function(t, r) {
    if (r) {
      var n = this.getSchemaFromProperty(t, r);
      if (n) {
        var i = qe(n);
        return this.getOrAddSchemaHandle(i).getResolvedSchema();
      }
    }
    if (this.cachedSchemaForResource && this.cachedSchemaForResource.resource === t)
      return this.cachedSchemaForResource.resolvedSchema;
    var s = this.getAssociatedSchemas(t), a = s.length > 0 ? this.createCombinedSchema(t, s).getResolvedSchema() : this.promise.resolve(void 0);
    return this.cachedSchemaForResource = { resource: t, resolvedSchema: a }, a;
  }, e.prototype.createCombinedSchema = function(t, r) {
    if (r.length === 1)
      return this.getOrAddSchemaHandle(r[0]);
    var n = "schemaservice://combinedSchema/" + encodeURIComponent(t), i = {
      allOf: r.map(function(s) {
        return { $ref: s };
      })
    };
    return this.addSchemaHandle(n, i);
  }, e.prototype.getMatchingSchemas = function(t, r, n) {
    if (n) {
      var i = n.id || "schemaservice://untitled/matchingSchemas/" + Yl++, s = this.addSchemaHandle(i, n);
      return s.getResolvedSchema().then(function(a) {
        return r.getMatchingSchemas(a.schema).filter(function(o) {
          return !o.inverted;
        });
      });
    }
    return this.getSchemaForResource(t.uri, r).then(function(a) {
      return a ? r.getMatchingSchemas(a.schema).filter(function(o) {
        return !o.inverted;
      }) : [];
    });
  }, e;
}(), Yl = 0;
function qe(e) {
  try {
    return pt.parse(e).toString(!0);
  } catch {
    return e;
  }
}
function Kl(e) {
  try {
    return pt.parse(e).with({ fragment: null, query: null }).toString(!0);
  } catch {
    return e;
  }
}
function Bt(e) {
  try {
    var t = pt.parse(e);
    if (t.scheme === "file")
      return t.fsPath;
  } catch {
  }
  return e;
}
function eu(e, t) {
  var r = [], n = [], i = [], s = -1, a = ht(e.getText(), !1), o = a.scan();
  function l(N) {
    r.push(N), n.push(i.length);
  }
  for (; o !== 17; ) {
    switch (o) {
      case 1:
      case 3: {
        var u = e.positionAt(a.getTokenOffset()).line, f = { startLine: u, endLine: u, kind: o === 1 ? "object" : "array" };
        i.push(f);
        break;
      }
      case 2:
      case 4: {
        var h = o === 2 ? "object" : "array";
        if (i.length > 0 && i[i.length - 1].kind === h) {
          var f = i.pop(), d = e.positionAt(a.getTokenOffset()).line;
          f && d > f.startLine + 1 && s !== f.startLine && (f.endLine = d - 1, l(f), s = f.startLine);
        }
        break;
      }
      case 13: {
        var u = e.positionAt(a.getTokenOffset()).line, g = e.positionAt(a.getTokenOffset() + a.getTokenLength()).line;
        a.getTokenError() === 1 && u + 1 < e.lineCount ? a.setPosition(e.offsetAt(Se.create(u + 1, 0))) : u < g && (l({ startLine: u, endLine: g, kind: St.Comment }), s = u);
        break;
      }
      case 12: {
        var p = e.getText().substr(a.getTokenOffset(), a.getTokenLength()), m = p.match(/^\/\/\s*#(region\b)|(endregion\b)/);
        if (m) {
          var d = e.positionAt(a.getTokenOffset()).line;
          if (m[1]) {
            var f = { startLine: d, endLine: d, kind: St.Region };
            i.push(f);
          } else {
            for (var v = i.length - 1; v >= 0 && i[v].kind !== St.Region; )
              v--;
            if (v >= 0) {
              var f = i[v];
              i.length = v, d > f.startLine && s !== f.startLine && (f.endLine = d, l(f), s = f.startLine);
            }
          }
        }
        break;
      }
    }
    o = a.scan();
  }
  var y = t && t.rangeLimit;
  if (typeof y != "number" || r.length <= y)
    return r;
  t && t.onRangeLimitExceeded && t.onRangeLimitExceeded(e.uri);
  for (var x = [], b = 0, S = n; b < S.length; b++) {
    var M = S[b];
    M < 30 && (x[M] = (x[M] || 0) + 1);
  }
  for (var A = 0, C = 0, v = 0; v < x.length; v++) {
    var _ = x[v];
    if (_) {
      if (_ + A > y) {
        C = v;
        break;
      }
      A += _;
    }
  }
  for (var w = [], v = 0; v < r.length; v++) {
    var M = n[v];
    typeof M == "number" && (M < C || M === C && A++ < y) && w.push(r[v]);
  }
  return w;
}
function tu(e, t, r) {
  function n(o) {
    for (var l = e.offsetAt(o), u = r.getNodeFromOffset(l, !0), f = []; u; ) {
      switch (u.type) {
        case "string":
        case "object":
        case "array":
          var h = u.offset + 1, d = u.offset + u.length - 1;
          h < d && l >= h && l <= d && f.push(i(h, d)), f.push(i(u.offset, u.offset + u.length));
          break;
        case "number":
        case "boolean":
        case "null":
        case "property":
          f.push(i(u.offset, u.offset + u.length));
          break;
      }
      if (u.type === "property" || u.parent && u.parent.type === "array") {
        var g = a(u.offset + u.length, 5);
        g !== -1 && f.push(i(u.offset, g));
      }
      u = u.parent;
    }
    for (var p = void 0, m = f.length - 1; m >= 0; m--)
      p = ar.create(f[m], p);
    return p || (p = ar.create(G.create(o, o))), p;
  }
  function i(o, l) {
    return G.create(e.positionAt(o), e.positionAt(l));
  }
  var s = ht(e.getText(), !0);
  function a(o, l) {
    s.setPosition(o);
    var u = s.scan();
    return u === l ? s.getTokenOffset() + s.getTokenLength() : -1;
  }
  return t.map(n);
}
function ru(e, t) {
  var r = [];
  return t.visit(function(n) {
    var i;
    if (n.type === "property" && n.keyNode.value === "$ref" && ((i = n.valueNode) === null || i === void 0 ? void 0 : i.type) === "string") {
      var s = n.valueNode.value, a = iu(t, s);
      if (a) {
        var o = e.positionAt(a.offset);
        r.push({
          target: "".concat(e.uri, "#").concat(o.line + 1, ",").concat(o.character + 1),
          range: nu(e, n.valueNode)
        });
      }
    }
    return !0;
  }), Promise.resolve(r);
}
function nu(e, t) {
  return G.create(e.positionAt(t.offset + 1), e.positionAt(t.offset + t.length - 1));
}
function iu(e, t) {
  var r = su(t);
  return r ? sn(r, e.root) : null;
}
function sn(e, t) {
  if (!t)
    return null;
  if (e.length === 0)
    return t;
  var r = e.shift();
  if (t && t.type === "object") {
    var n = t.properties.find(function(a) {
      return a.keyNode.value === r;
    });
    return n ? sn(e, n.valueNode) : null;
  } else if (t && t.type === "array" && r.match(/^(0|[1-9][0-9]*)$/)) {
    var i = Number.parseInt(r), s = t.items[i];
    return s ? sn(e, s) : null;
  }
  return null;
}
function su(e) {
  return e === "#" ? [] : e[0] !== "#" || e[1] !== "/" ? null : e.substring(2).split(/\//).map(au);
}
function au(e) {
  return e.replace(/~1/g, "/").replace(/~0/g, "~");
}
function ou(e) {
  var t = e.promiseConstructor || Promise, r = new Ql(e.schemaRequestService, e.workspaceContext, t);
  r.setSchemaContributions(nn);
  var n = new Fl(r, e.contributions, t, e.clientCapabilities), i = new Il(r, e.contributions, t), s = new Wl(r), a = new Ol(r, t);
  return {
    configure: function(o) {
      r.clearExternalSchemas(), o.schemas && o.schemas.forEach(function(l) {
        r.registerExternalSchema(l.uri, l.fileMatch, l.schema);
      }), a.configure(o);
    },
    resetSchema: function(o) {
      return r.onResourceChange(o);
    },
    doValidation: a.doValidation.bind(a),
    getLanguageStatus: a.getLanguageStatus.bind(a),
    parseJSONDocument: function(o) {
      return Rl(o, { collectComments: !0 });
    },
    newJSONDocument: function(o, l) {
      return Pl(o, l);
    },
    getMatchingSchemas: r.getMatchingSchemas.bind(r),
    doResolve: n.doResolve.bind(n),
    doComplete: n.doComplete.bind(n),
    findDocumentSymbols: s.findDocumentSymbols.bind(s),
    findDocumentSymbols2: s.findDocumentSymbols2.bind(s),
    findDocumentColors: s.findDocumentColors.bind(s),
    getColorPresentations: s.getColorPresentations.bind(s),
    doHover: i.doHover.bind(i),
    getFoldingRanges: eu,
    getSelectionRanges: tu,
    findDefinition: function() {
      return Promise.resolve([]);
    },
    findLinks: ru,
    format: function(o, l, u) {
      var f = void 0;
      if (l) {
        var h = o.offsetAt(l.start), d = o.offsetAt(l.end) - h;
        f = { offset: h, length: d };
      }
      var g = { tabSize: u ? u.tabSize : 4, insertSpaces: (u == null ? void 0 : u.insertSpaces) === !0, insertFinalNewline: (u == null ? void 0 : u.insertFinalNewline) === !0, eol: `
` };
      return bl(o.getText(), f, g).map(function(p) {
        return Ne.replace(G.create(o.positionAt(p.offset), o.positionAt(p.offset + p.length)), p.content);
      });
    }
  };
}
var Ks;
typeof fetch < "u" && (Ks = function(e) {
  return fetch(e).then((t) => t.text());
});
var lu = class {
  constructor(e, t) {
    vt(this, "_ctx");
    vt(this, "_languageService");
    vt(this, "_languageSettings");
    vt(this, "_languageId");
    this._ctx = e, this._languageSettings = t.languageSettings, this._languageId = t.languageId, this._languageService = ou({
      workspaceContext: {
        resolveRelativePath: (r, n) => {
          const i = n.substr(0, n.lastIndexOf("/") + 1);
          return fu(i, r);
        }
      },
      schemaRequestService: t.enableSchemaRequest ? Ks : void 0
    }), this._languageService.configure(this._languageSettings);
  }
  async doValidation(e) {
    let t = this._getTextDocument(e);
    if (t) {
      let r = this._languageService.parseJSONDocument(t);
      return this._languageService.doValidation(t, r, this._languageSettings);
    }
    return Promise.resolve([]);
  }
  async doComplete(e, t) {
    let r = this._getTextDocument(e);
    if (!r)
      return null;
    let n = this._languageService.parseJSONDocument(r);
    return this._languageService.doComplete(r, t, n);
  }
  async doResolve(e) {
    return this._languageService.doResolve(e);
  }
  async doHover(e, t) {
    let r = this._getTextDocument(e);
    if (!r)
      return null;
    let n = this._languageService.parseJSONDocument(r);
    return this._languageService.doHover(r, t, n);
  }
  async format(e, t, r) {
    let n = this._getTextDocument(e);
    if (!n)
      return [];
    let i = this._languageService.format(n, t, r);
    return Promise.resolve(i);
  }
  async resetSchema(e) {
    return Promise.resolve(this._languageService.resetSchema(e));
  }
  async findDocumentSymbols(e) {
    let t = this._getTextDocument(e);
    if (!t)
      return [];
    let r = this._languageService.parseJSONDocument(t), n = this._languageService.findDocumentSymbols(t, r);
    return Promise.resolve(n);
  }
  async findDocumentColors(e) {
    let t = this._getTextDocument(e);
    if (!t)
      return [];
    let r = this._languageService.parseJSONDocument(t), n = this._languageService.findDocumentColors(t, r);
    return Promise.resolve(n);
  }
  async getColorPresentations(e, t, r) {
    let n = this._getTextDocument(e);
    if (!n)
      return [];
    let i = this._languageService.parseJSONDocument(n), s = this._languageService.getColorPresentations(n, i, t, r);
    return Promise.resolve(s);
  }
  async getFoldingRanges(e, t) {
    let r = this._getTextDocument(e);
    if (!r)
      return [];
    let n = this._languageService.getFoldingRanges(r, t);
    return Promise.resolve(n);
  }
  async getSelectionRanges(e, t) {
    let r = this._getTextDocument(e);
    if (!r)
      return [];
    let n = this._languageService.parseJSONDocument(r), i = this._languageService.getSelectionRanges(r, t, n);
    return Promise.resolve(i);
  }
  _getTextDocument(e) {
    let t = this._ctx.getMirrorModels();
    for (let r of t)
      if (r.uri.toString() === e)
        return Yr.create(e, this._languageId, r.version, r.getValue());
    return null;
  }
}, uu = "/".charCodeAt(0), Nr = ".".charCodeAt(0);
function cu(e) {
  return e.charCodeAt(0) === uu;
}
function fu(e, t) {
  if (cu(t)) {
    const r = pt.parse(e), n = t.split("/");
    return r.with({ path: ea(n) }).toString();
  }
  return hu(e, t);
}
function ea(e) {
  const t = [];
  for (const n of e)
    n.length === 0 || n.length === 1 && n.charCodeAt(0) === Nr || (n.length === 2 && n.charCodeAt(0) === Nr && n.charCodeAt(1) === Nr ? t.pop() : t.push(n));
  e.length > 1 && e[e.length - 1].length === 0 && t.push("");
  let r = t.join("/");
  return e[0].length === 0 && (r = "/" + r), r;
}
function hu(e, ...t) {
  const r = pt.parse(e), n = r.path.split("/");
  for (let i of t)
    n.push(...i.split("/"));
  return r.with({ path: ea(n) }).toString();
}
self.onmessage = () => {
  Ws((e, t) => new lu(e, t));
};
