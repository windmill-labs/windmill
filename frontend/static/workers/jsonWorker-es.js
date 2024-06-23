var _a = Object.defineProperty;
var Sa = (e, t, n) => t in e ? _a(e, t, { enumerable: !0, configurable: !0, writable: !0, value: n }) : e[t] = n;
var Et = (e, t, n) => (Sa(e, typeof t != "symbol" ? t + "" : t, n), n);
class La {
  constructor() {
    this.listeners = [], this.unexpectedErrorHandler = function(t) {
      setTimeout(() => {
        throw t.stack ? St.isErrorNoTelemetry(t) ? new St(t.message + `

` + t.stack) : new Error(t.message + `

` + t.stack) : t;
      }, 0);
    };
  }
  emit(t) {
    this.listeners.forEach((n) => {
      n(t);
    });
  }
  onUnexpectedError(t) {
    this.unexpectedErrorHandler(t), this.emit(t);
  }
  // For external errors, we don't want the listeners to be called
  onUnexpectedExternalError(t) {
    this.unexpectedErrorHandler(t);
  }
}
const Na = new La();
function Hs(e) {
  Aa(e) || Na.onUnexpectedError(e);
}
function Br(e) {
  if (e instanceof Error) {
    const { name: t, message: n } = e, r = e.stacktrace || e.stack;
    return {
      $isError: !0,
      name: t,
      message: n,
      stack: r,
      noTelemetry: St.isErrorNoTelemetry(e)
    };
  }
  return e;
}
const Jn = "Canceled";
function Aa(e) {
  return e instanceof Ca ? !0 : e instanceof Error && e.name === Jn && e.message === Jn;
}
class Ca extends Error {
  constructor() {
    super(Jn), this.name = this.message;
  }
}
class St extends Error {
  constructor(t) {
    super(t), this.name = "CodeExpectedError";
  }
  static fromError(t) {
    if (t instanceof St)
      return t;
    const n = new St();
    return n.message = t.message, n.stack = t.stack, n;
  }
  static isErrorNoTelemetry(t) {
    return t.name === "CodeExpectedError";
  }
}
class Ke extends Error {
  constructor(t) {
    super(t || "An unexpected bug occurred."), Object.setPrototypeOf(this, Ke.prototype);
  }
}
function ka(e) {
  const t = this;
  let n = !1, r;
  return function() {
    return n || (n = !0, r = e.apply(t, arguments)), r;
  };
}
var mn;
(function(e) {
  function t(b) {
    return b && typeof b == "object" && typeof b[Symbol.iterator] == "function";
  }
  e.is = t;
  const n = Object.freeze([]);
  function r() {
    return n;
  }
  e.empty = r;
  function* i(b) {
    yield b;
  }
  e.single = i;
  function s(b) {
    return t(b) ? b : i(b);
  }
  e.wrap = s;
  function a(b) {
    return b || n;
  }
  e.from = a;
  function* o(b) {
    for (let y = b.length - 1; y >= 0; y--)
      yield b[y];
  }
  e.reverse = o;
  function l(b) {
    return !b || b[Symbol.iterator]().next().done === !0;
  }
  e.isEmpty = l;
  function u(b) {
    return b[Symbol.iterator]().next().value;
  }
  e.first = u;
  function c(b, y) {
    for (const _ of b)
      if (y(_))
        return !0;
    return !1;
  }
  e.some = c;
  function f(b, y) {
    for (const _ of b)
      if (y(_))
        return _;
  }
  e.find = f;
  function* d(b, y) {
    for (const _ of b)
      y(_) && (yield _);
  }
  e.filter = d;
  function* g(b, y) {
    let _ = 0;
    for (const N of b)
      yield y(N, _++);
  }
  e.map = g;
  function* m(...b) {
    for (const y of b)
      for (const _ of y)
        yield _;
  }
  e.concat = m;
  function p(b, y, _) {
    let N = _;
    for (const w of b)
      N = y(N, w);
    return N;
  }
  e.reduce = p;
  function* v(b, y, _ = b.length) {
    for (y < 0 && (y += b.length), _ < 0 ? _ += b.length : _ > b.length && (_ = b.length); y < _; y++)
      yield b[y];
  }
  e.slice = v;
  function x(b, y = Number.POSITIVE_INFINITY) {
    const _ = [];
    if (y === 0)
      return [_, b];
    const N = b[Symbol.iterator]();
    for (let w = 0; w < y; w++) {
      const S = N.next();
      if (S.done)
        return [_, e.empty()];
      _.push(S.value);
    }
    return [_, { [Symbol.iterator]() {
      return N;
    } }];
  }
  e.consume = x;
})(mn || (mn = {}));
function zs(e) {
  if (mn.is(e)) {
    const t = [];
    for (const n of e)
      if (n)
        try {
          n.dispose();
        } catch (r) {
          t.push(r);
        }
    if (t.length === 1)
      throw t[0];
    if (t.length > 1)
      throw new AggregateError(t, "Encountered errors while disposing of store");
    return Array.isArray(e) ? [] : e;
  } else if (e)
    return e.dispose(), e;
}
function Ea(...e) {
  return Ot(() => zs(e));
}
function Ot(e) {
  return {
    dispose: ka(() => {
      e();
    })
  };
}
class Ct {
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
        zs(this._toDispose);
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
    return this._isDisposed ? Ct.DISABLE_DISPOSED_WARNING || console.warn(new Error("Trying to add a disposable to a DisposableStore that has already been disposed of. The added object will be leaked!").stack) : this._toDispose.add(t), t;
  }
  /**
   * Deletes the value from the store, but does not dispose it.
   */
  deleteAndLeak(t) {
    t && this._toDispose.has(t) && this._toDispose.delete(t);
  }
}
Ct.DISABLE_DISPOSED_WARNING = !1;
class jt {
  constructor() {
    this._store = new Ct(), this._store;
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
jt.None = Object.freeze({ dispose() {
} });
class Q {
  constructor(t) {
    this.element = t, this.next = Q.Undefined, this.prev = Q.Undefined;
  }
}
Q.Undefined = new Q(void 0);
class Ma {
  constructor() {
    this._first = Q.Undefined, this._last = Q.Undefined, this._size = 0;
  }
  get size() {
    return this._size;
  }
  isEmpty() {
    return this._first === Q.Undefined;
  }
  clear() {
    let t = this._first;
    for (; t !== Q.Undefined; ) {
      const n = t.next;
      t.prev = Q.Undefined, t.next = Q.Undefined, t = n;
    }
    this._first = Q.Undefined, this._last = Q.Undefined, this._size = 0;
  }
  unshift(t) {
    return this._insert(t, !1);
  }
  push(t) {
    return this._insert(t, !0);
  }
  _insert(t, n) {
    const r = new Q(t);
    if (this._first === Q.Undefined)
      this._first = r, this._last = r;
    else if (n) {
      const s = this._last;
      this._last = r, r.prev = s, s.next = r;
    } else {
      const s = this._first;
      this._first = r, r.next = s, s.prev = r;
    }
    this._size += 1;
    let i = !1;
    return () => {
      i || (i = !0, this._remove(r));
    };
  }
  shift() {
    if (this._first !== Q.Undefined) {
      const t = this._first.element;
      return this._remove(this._first), t;
    }
  }
  pop() {
    if (this._last !== Q.Undefined) {
      const t = this._last.element;
      return this._remove(this._last), t;
    }
  }
  _remove(t) {
    if (t.prev !== Q.Undefined && t.next !== Q.Undefined) {
      const n = t.prev;
      n.next = t.next, t.next.prev = n;
    } else
      t.prev === Q.Undefined && t.next === Q.Undefined ? (this._first = Q.Undefined, this._last = Q.Undefined) : t.next === Q.Undefined ? (this._last = this._last.prev, this._last.next = Q.Undefined) : t.prev === Q.Undefined && (this._first = this._first.next, this._first.prev = Q.Undefined);
    this._size -= 1;
  }
  *[Symbol.iterator]() {
    let t = this._first;
    for (; t !== Q.Undefined; )
      yield t.element, t = t.next;
  }
}
const Ra = globalThis.performance && typeof globalThis.performance.now == "function";
class Mn {
  static create(t) {
    return new Mn(t);
  }
  constructor(t) {
    this._now = Ra && t === !1 ? Date.now : globalThis.performance.now.bind(globalThis.performance), this._startTime = this._now(), this._stopTime = -1;
  }
  stop() {
    this._stopTime = this._now();
  }
  elapsed() {
    return this._stopTime !== -1 ? this._stopTime - this._startTime : this._now() - this._startTime;
  }
}
var Xn;
(function(e) {
  e.None = () => jt.None;
  function t(L, E) {
    return f(L, () => {
    }, 0, void 0, !0, void 0, E);
  }
  e.defer = t;
  function n(L) {
    return (E, P = null, O) => {
      let V = !1, k;
      return k = L((A) => {
        if (!V)
          return k ? k.dispose() : V = !0, E.call(P, A);
      }, null, O), V && k.dispose(), k;
    };
  }
  e.once = n;
  function r(L, E, P) {
    return u((O, V = null, k) => L((A) => O.call(V, E(A)), null, k), P);
  }
  e.map = r;
  function i(L, E, P) {
    return u((O, V = null, k) => L((A) => {
      E(A), O.call(V, A);
    }, null, k), P);
  }
  e.forEach = i;
  function s(L, E, P) {
    return u((O, V = null, k) => L((A) => E(A) && O.call(V, A), null, k), P);
  }
  e.filter = s;
  function a(L) {
    return L;
  }
  e.signal = a;
  function o(...L) {
    return (E, P = null, O) => {
      const V = Ea(...L.map((k) => k((A) => E.call(P, A))));
      return c(V, O);
    };
  }
  e.any = o;
  function l(L, E, P, O) {
    let V = P;
    return r(L, (k) => (V = E(V, k), V), O);
  }
  e.reduce = l;
  function u(L, E) {
    let P;
    const O = {
      onWillAddFirstListener() {
        P = L(V.fire, V);
      },
      onDidRemoveLastListener() {
        P == null || P.dispose();
      }
    }, V = new Ae(O);
    return E == null || E.add(V), V.event;
  }
  function c(L, E) {
    return E instanceof Array ? E.push(L) : E && E.add(L), L;
  }
  function f(L, E, P = 100, O = !1, V = !1, k, A) {
    let T, F, B, U = 0, W;
    const _e = {
      leakWarningThreshold: k,
      onWillAddFirstListener() {
        T = L((de) => {
          U++, F = E(F, de), O && !B && (ee.fire(F), F = void 0), W = () => {
            const We = F;
            F = void 0, B = void 0, (!O || U > 1) && ee.fire(We), U = 0;
          }, typeof P == "number" ? (clearTimeout(B), B = setTimeout(W, P)) : B === void 0 && (B = 0, queueMicrotask(W));
        });
      },
      onWillRemoveListener() {
        V && U > 0 && (W == null || W());
      },
      onDidRemoveLastListener() {
        W = void 0, T.dispose();
      }
    }, ee = new Ae(_e);
    return A == null || A.add(ee), ee.event;
  }
  e.debounce = f;
  function d(L, E = 0, P) {
    return e.debounce(L, (O, V) => O ? (O.push(V), O) : [V], E, void 0, !0, void 0, P);
  }
  e.accumulate = d;
  function g(L, E = (O, V) => O === V, P) {
    let O = !0, V;
    return s(L, (k) => {
      const A = O || !E(k, V);
      return O = !1, V = k, A;
    }, P);
  }
  e.latch = g;
  function m(L, E, P) {
    return [
      e.filter(L, E, P),
      e.filter(L, (O) => !E(O), P)
    ];
  }
  e.split = m;
  function p(L, E = !1, P = [], O) {
    let V = P.slice(), k = L((F) => {
      V ? V.push(F) : T.fire(F);
    });
    O && O.add(k);
    const A = () => {
      V == null || V.forEach((F) => T.fire(F)), V = null;
    }, T = new Ae({
      onWillAddFirstListener() {
        k || (k = L((F) => T.fire(F)), O && O.add(k));
      },
      onDidAddFirstListener() {
        V && (E ? setTimeout(A) : A());
      },
      onDidRemoveLastListener() {
        k && k.dispose(), k = null;
      }
    });
    return O && O.add(T), T.event;
  }
  e.buffer = p;
  function v(L, E) {
    return (O, V, k) => {
      const A = E(new b());
      return L(function(T) {
        const F = A.evaluate(T);
        F !== x && O.call(V, F);
      }, void 0, k);
    };
  }
  e.chain = v;
  const x = Symbol("HaltChainable");
  class b {
    constructor() {
      this.steps = [];
    }
    map(E) {
      return this.steps.push(E), this;
    }
    forEach(E) {
      return this.steps.push((P) => (E(P), P)), this;
    }
    filter(E) {
      return this.steps.push((P) => E(P) ? P : x), this;
    }
    reduce(E, P) {
      let O = P;
      return this.steps.push((V) => (O = E(O, V), O)), this;
    }
    latch(E = (P, O) => P === O) {
      let P = !0, O;
      return this.steps.push((V) => {
        const k = P || !E(V, O);
        return P = !1, O = V, k ? V : x;
      }), this;
    }
    evaluate(E) {
      for (const P of this.steps)
        if (E = P(E), E === x)
          break;
      return E;
    }
  }
  function y(L, E, P = (O) => O) {
    const O = (...T) => A.fire(P(...T)), V = () => L.on(E, O), k = () => L.removeListener(E, O), A = new Ae({ onWillAddFirstListener: V, onDidRemoveLastListener: k });
    return A.event;
  }
  e.fromNodeEventEmitter = y;
  function _(L, E, P = (O) => O) {
    const O = (...T) => A.fire(P(...T)), V = () => L.addEventListener(E, O), k = () => L.removeEventListener(E, O), A = new Ae({ onWillAddFirstListener: V, onDidRemoveLastListener: k });
    return A.event;
  }
  e.fromDOMEventEmitter = _;
  function N(L) {
    return new Promise((E) => n(L)(E));
  }
  e.toPromise = N;
  function w(L) {
    const E = new Ae();
    return L.then((P) => {
      E.fire(P);
    }, () => {
      E.fire(void 0);
    }).finally(() => {
      E.dispose();
    }), E.event;
  }
  e.fromPromise = w;
  function S(L, E) {
    return E(void 0), L((P) => E(P));
  }
  e.runAndSubscribe = S;
  function C(L, E) {
    let P = null;
    function O(k) {
      P == null || P.dispose(), P = new Ct(), E(k, P);
    }
    O(void 0);
    const V = L((k) => O(k));
    return Ot(() => {
      V.dispose(), P == null || P.dispose();
    });
  }
  e.runAndSubscribeWithStore = C;
  class M {
    constructor(E, P) {
      this._observable = E, this._counter = 0, this._hasChanged = !1;
      const O = {
        onWillAddFirstListener: () => {
          E.addObserver(this);
        },
        onDidRemoveLastListener: () => {
          E.removeObserver(this);
        }
      };
      this.emitter = new Ae(O), P && P.add(this.emitter);
    }
    beginUpdate(E) {
      this._counter++;
    }
    handlePossibleChange(E) {
    }
    handleChange(E, P) {
      this._hasChanged = !0;
    }
    endUpdate(E) {
      this._counter--, this._counter === 0 && (this._observable.reportChanges(), this._hasChanged && (this._hasChanged = !1, this.emitter.fire(this._observable.get())));
    }
  }
  function I(L, E) {
    return new M(L, E).emitter.event;
  }
  e.fromObservable = I;
  function D(L) {
    return (E) => {
      let P = 0, O = !1;
      const V = {
        beginUpdate() {
          P++;
        },
        endUpdate() {
          P--, P === 0 && (L.reportChanges(), O && (O = !1, E()));
        },
        handlePossibleChange() {
        },
        handleChange() {
          O = !0;
        }
      };
      return L.addObserver(V), L.reportChanges(), {
        dispose() {
          L.removeObserver(V);
        }
      };
    };
  }
  e.fromObservableLight = D;
})(Xn || (Xn = {}));
class Lt {
  constructor(t) {
    this.listenerCount = 0, this.invocationCount = 0, this.elapsedOverall = 0, this.durations = [], this.name = `${t}_${Lt._idPool++}`, Lt.all.add(this);
  }
  start(t) {
    this._stopWatch = new Mn(), this.listenerCount = t;
  }
  stop() {
    if (this._stopWatch) {
      const t = this._stopWatch.elapsed();
      this.durations.push(t), this.elapsedOverall += t, this.invocationCount += 1, this._stopWatch = void 0;
    }
  }
}
Lt.all = /* @__PURE__ */ new Set();
Lt._idPool = 0;
let Ta = -1;
class Pa {
  constructor(t, n = Math.random().toString(18).slice(2, 5)) {
    this.threshold = t, this.name = n, this._warnCountdown = 0;
  }
  dispose() {
    var t;
    (t = this._stacks) === null || t === void 0 || t.clear();
  }
  check(t, n) {
    const r = this.threshold;
    if (r <= 0 || n < r)
      return;
    this._stacks || (this._stacks = /* @__PURE__ */ new Map());
    const i = this._stacks.get(t.value) || 0;
    if (this._stacks.set(t.value, i + 1), this._warnCountdown -= 1, this._warnCountdown <= 0) {
      this._warnCountdown = r * 0.5;
      let s, a = 0;
      for (const [o, l] of this._stacks)
        (!s || a < l) && (s = o, a = l);
      console.warn(`[${this.name}] potential listener LEAK detected, having ${n} listeners already. MOST frequent listener (${a}):`), console.warn(s);
    }
    return () => {
      const s = this._stacks.get(t.value) || 0;
      this._stacks.set(t.value, s - 1);
    };
  }
}
class Er {
  static create() {
    var t;
    return new Er((t = new Error().stack) !== null && t !== void 0 ? t : "");
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
class Fn {
  constructor(t) {
    this.value = t;
  }
}
const Fa = 2;
class Ae {
  constructor(t) {
    var n, r, i, s, a;
    this._size = 0, this._options = t, this._leakageMon = !((n = this._options) === null || n === void 0) && n.leakWarningThreshold ? new Pa((i = (r = this._options) === null || r === void 0 ? void 0 : r.leakWarningThreshold) !== null && i !== void 0 ? i : Ta) : void 0, this._perfMon = !((s = this._options) === null || s === void 0) && s._profName ? new Lt(this._options._profName) : void 0, this._deliveryQueue = (a = this._options) === null || a === void 0 ? void 0 : a.deliveryQueue;
  }
  dispose() {
    var t, n, r, i;
    this._disposed || (this._disposed = !0, ((t = this._deliveryQueue) === null || t === void 0 ? void 0 : t.current) === this && this._deliveryQueue.reset(), this._listeners && (this._listeners = void 0, this._size = 0), (r = (n = this._options) === null || n === void 0 ? void 0 : n.onDidRemoveLastListener) === null || r === void 0 || r.call(n), (i = this._leakageMon) === null || i === void 0 || i.dispose());
  }
  /**
   * For the public to allow to subscribe
   * to events from this Emitter
   */
  get event() {
    var t;
    return (t = this._event) !== null && t !== void 0 || (this._event = (n, r, i) => {
      var s, a, o, l, u;
      if (this._leakageMon && this._size > this._leakageMon.threshold * 3)
        return console.warn(`[${this._leakageMon.name}] REFUSES to accept new listeners because it exceeded its threshold by far`), jt.None;
      if (this._disposed)
        return jt.None;
      r && (n = n.bind(r));
      const c = new Fn(n);
      let f;
      this._leakageMon && this._size >= Math.ceil(this._leakageMon.threshold * 0.2) && (c.stack = Er.create(), f = this._leakageMon.check(c.stack, this._size + 1)), this._listeners ? this._listeners instanceof Fn ? ((u = this._deliveryQueue) !== null && u !== void 0 || (this._deliveryQueue = new Ia()), this._listeners = [this._listeners, c]) : this._listeners.push(c) : ((a = (s = this._options) === null || s === void 0 ? void 0 : s.onWillAddFirstListener) === null || a === void 0 || a.call(s, this), this._listeners = c, (l = (o = this._options) === null || o === void 0 ? void 0 : o.onDidAddFirstListener) === null || l === void 0 || l.call(o, this)), this._size++;
      const d = Ot(() => {
        f == null || f(), this._removeListener(c);
      });
      return i instanceof Ct ? i.add(d) : Array.isArray(i) && i.push(d), d;
    }), this._event;
  }
  _removeListener(t) {
    var n, r, i, s;
    if ((r = (n = this._options) === null || n === void 0 ? void 0 : n.onWillRemoveListener) === null || r === void 0 || r.call(n, this), !this._listeners)
      return;
    if (this._size === 1) {
      this._listeners = void 0, (s = (i = this._options) === null || i === void 0 ? void 0 : i.onDidRemoveLastListener) === null || s === void 0 || s.call(i, this), this._size = 0;
      return;
    }
    const a = this._listeners, o = a.indexOf(t);
    if (o === -1)
      throw console.log("disposed?", this._disposed), console.log("size?", this._size), console.log("arr?", JSON.stringify(this._listeners)), new Error("Attempted to dispose unknown listener");
    this._size--, a[o] = void 0;
    const l = this._deliveryQueue.current === this;
    if (this._size * Fa <= a.length) {
      let u = 0;
      for (let c = 0; c < a.length; c++)
        a[c] ? a[u++] = a[c] : l && (this._deliveryQueue.end--, u < this._deliveryQueue.i && this._deliveryQueue.i--);
      a.length = u;
    }
  }
  _deliver(t, n) {
    var r;
    if (!t)
      return;
    const i = ((r = this._options) === null || r === void 0 ? void 0 : r.onListenerError) || Hs;
    if (!i) {
      t.value(n);
      return;
    }
    try {
      t.value(n);
    } catch (s) {
      i(s);
    }
  }
  /** Delivers items in the queue. Assumes the queue is ready to go. */
  _deliverQueue(t) {
    const n = t.current._listeners;
    for (; t.i < t.end; )
      this._deliver(n[t.i++], t.value);
    t.reset();
  }
  /**
   * To be kept private to fire an event to
   * subscribers
   */
  fire(t) {
    var n, r, i, s;
    if (!((n = this._deliveryQueue) === null || n === void 0) && n.current && (this._deliverQueue(this._deliveryQueue), (r = this._perfMon) === null || r === void 0 || r.stop()), (i = this._perfMon) === null || i === void 0 || i.start(this._size), this._listeners)
      if (this._listeners instanceof Fn)
        this._deliver(this._listeners, t);
      else {
        const a = this._deliveryQueue;
        a.enqueue(this, t, this._listeners.length), this._deliverQueue(a);
      }
    (s = this._perfMon) === null || s === void 0 || s.stop();
  }
  hasListeners() {
    return this._size > 0;
  }
}
class Ia {
  constructor() {
    this.i = -1, this.end = 0;
  }
  enqueue(t, n, r) {
    this.i = 0, this.end = r, this.current = t, this.value = n;
  }
  reset() {
    this.i = this.end, this.current = void 0, this.value = void 0;
  }
}
function Da(e) {
  return typeof e == "string";
}
function Va(e) {
  let t = [];
  for (; Object.prototype !== e; )
    t = t.concat(Object.getOwnPropertyNames(e)), e = Object.getPrototypeOf(e);
  return t;
}
function Qn(e) {
  const t = [];
  for (const n of Va(e))
    typeof e[n] == "function" && t.push(n);
  return t;
}
function Oa(e, t) {
  const n = (i) => function() {
    const s = Array.prototype.slice.call(arguments, 0);
    return t(i, s);
  }, r = {};
  for (const i of e)
    r[i] = n(i);
  return r;
}
globalThis && globalThis.__awaiter;
let ja = typeof document < "u" && document.location && document.location.hash.indexOf("pseudo=true") >= 0;
function qa(e, t) {
  let n;
  return t.length === 0 ? n = e : n = e.replace(/\{(\d+)\}/g, (r, i) => {
    const s = i[0], a = t[s];
    let o = r;
    return typeof a == "string" ? o = a : (typeof a == "number" || typeof a == "boolean" || a === void 0 || a === null) && (o = String(a)), o;
  }), ja && (n = "［" + n.replace(/[aouei]/g, "$&$&") + "］"), n;
}
function Y(e, t, ...n) {
  return qa(t, n);
}
var In;
const vt = "en";
let Zn = !1, Yn = !1, Dn = !1, Gs = !1, en, Vn = vt, Ur = vt, Ba, Ne;
const Ee = typeof self == "object" ? self : typeof global == "object" ? global : {};
let ce;
typeof Ee.vscode < "u" && typeof Ee.vscode.process < "u" ? ce = Ee.vscode.process : typeof process < "u" && (ce = process);
const Ua = typeof ((In = ce == null ? void 0 : ce.versions) === null || In === void 0 ? void 0 : In.electron) == "string", $a = Ua && (ce == null ? void 0 : ce.type) === "renderer";
if (typeof navigator == "object" && !$a)
  Ne = navigator.userAgent, Zn = Ne.indexOf("Windows") >= 0, Yn = Ne.indexOf("Macintosh") >= 0, (Ne.indexOf("Macintosh") >= 0 || Ne.indexOf("iPad") >= 0 || Ne.indexOf("iPhone") >= 0) && navigator.maxTouchPoints && navigator.maxTouchPoints > 0, Dn = Ne.indexOf("Linux") >= 0, (Ne == null ? void 0 : Ne.indexOf("Mobi")) >= 0, Gs = !0, // This call _must_ be done in the file that calls `nls.getConfiguredDefaultLocale`
  // to ensure that the NLS AMD Loader plugin has been loaded and configured.
  // This is because the loader plugin decides what the default locale is based on
  // how it's able to resolve the strings.
  Y({ key: "ensureLoaderPluginIsLoaded", comment: ["{Locked}"] }, "_"), en = vt, Vn = en, Ur = navigator.language;
else if (typeof ce == "object") {
  Zn = ce.platform === "win32", Yn = ce.platform === "darwin", Dn = ce.platform === "linux", Dn && ce.env.SNAP && ce.env.SNAP_REVISION, ce.env.CI || ce.env.BUILD_ARTIFACTSTAGINGDIRECTORY, en = vt, Vn = vt;
  const e = ce.env.VSCODE_NLS_CONFIG;
  if (e)
    try {
      const t = JSON.parse(e), n = t.availableLanguages["*"];
      en = t.locale, Ur = t.osLocale, Vn = n || vt, Ba = t._translationsConfigFile;
    } catch {
    }
} else
  console.error("Unable to resolve platform.");
const qt = Zn, Wa = Yn;
Gs && Ee.importScripts;
const Ve = Ne, Ha = typeof Ee.postMessage == "function" && !Ee.importScripts;
(() => {
  if (Ha) {
    const e = [];
    Ee.addEventListener("message", (n) => {
      if (n.data && n.data.vscodeScheduleAsyncWork)
        for (let r = 0, i = e.length; r < i; r++) {
          const s = e[r];
          if (s.id === n.data.vscodeScheduleAsyncWork) {
            e.splice(r, 1), s.callback();
            return;
          }
        }
    });
    let t = 0;
    return (n) => {
      const r = ++t;
      e.push({
        id: r,
        callback: n
      }), Ee.postMessage({ vscodeScheduleAsyncWork: r }, "*");
    };
  }
  return (e) => setTimeout(e);
})();
const za = !!(Ve && Ve.indexOf("Chrome") >= 0);
Ve && Ve.indexOf("Firefox") >= 0;
!za && Ve && Ve.indexOf("Safari") >= 0;
Ve && Ve.indexOf("Edg/") >= 0;
Ve && Ve.indexOf("Android") >= 0;
class Ga {
  constructor(t) {
    this.fn = t, this.lastCache = void 0, this.lastArgKey = void 0;
  }
  get(t) {
    const n = JSON.stringify(t);
    return this.lastArgKey !== n && (this.lastArgKey = n, this.lastCache = this.fn(t)), this.lastCache;
  }
}
class Js {
  constructor(t) {
    this.executor = t, this._didRun = !1;
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
var Nt;
function Ja(e) {
  return e.replace(/[\\\{\}\*\+\?\|\^\$\.\[\]\(\)]/g, "\\$&");
}
function Xa(e) {
  return e.split(/\r\n|\r|\n/);
}
function Qa(e) {
  for (let t = 0, n = e.length; t < n; t++) {
    const r = e.charCodeAt(t);
    if (r !== 32 && r !== 9)
      return t;
  }
  return -1;
}
function Za(e, t = e.length - 1) {
  for (let n = t; n >= 0; n--) {
    const r = e.charCodeAt(n);
    if (r !== 32 && r !== 9)
      return n;
  }
  return -1;
}
function Xs(e) {
  return e >= 65 && e <= 90;
}
function Kn(e) {
  return 55296 <= e && e <= 56319;
}
function Ya(e) {
  return 56320 <= e && e <= 57343;
}
function Ka(e, t) {
  return (e - 55296 << 10) + (t - 56320) + 65536;
}
function eo(e, t, n) {
  const r = e.charCodeAt(n);
  if (Kn(r) && n + 1 < t) {
    const i = e.charCodeAt(n + 1);
    if (Ya(i))
      return Ka(r, i);
  }
  return r;
}
const to = /^[\t\n\r\x20-\x7E]*$/;
function no(e) {
  return to.test(e);
}
class lt {
  static getInstance(t) {
    return Nt.cache.get(Array.from(t));
  }
  static getLocales() {
    return Nt._locales.value;
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
Nt = lt;
lt.ambiguousCharacterData = new Js(() => JSON.parse('{"_common":[8232,32,8233,32,5760,32,8192,32,8193,32,8194,32,8195,32,8196,32,8197,32,8198,32,8200,32,8201,32,8202,32,8287,32,8199,32,8239,32,2042,95,65101,95,65102,95,65103,95,8208,45,8209,45,8210,45,65112,45,1748,45,8259,45,727,45,8722,45,10134,45,11450,45,1549,44,1643,44,8218,44,184,44,42233,44,894,59,2307,58,2691,58,1417,58,1795,58,1796,58,5868,58,65072,58,6147,58,6153,58,8282,58,1475,58,760,58,42889,58,8758,58,720,58,42237,58,451,33,11601,33,660,63,577,63,2429,63,5038,63,42731,63,119149,46,8228,46,1793,46,1794,46,42510,46,68176,46,1632,46,1776,46,42232,46,1373,96,65287,96,8219,96,8242,96,1370,96,1523,96,8175,96,65344,96,900,96,8189,96,8125,96,8127,96,8190,96,697,96,884,96,712,96,714,96,715,96,756,96,699,96,701,96,700,96,702,96,42892,96,1497,96,2036,96,2037,96,5194,96,5836,96,94033,96,94034,96,65339,91,10088,40,10098,40,12308,40,64830,40,65341,93,10089,41,10099,41,12309,41,64831,41,10100,123,119060,123,10101,125,65342,94,8270,42,1645,42,8727,42,66335,42,5941,47,8257,47,8725,47,8260,47,9585,47,10187,47,10744,47,119354,47,12755,47,12339,47,11462,47,20031,47,12035,47,65340,92,65128,92,8726,92,10189,92,10741,92,10745,92,119311,92,119355,92,12756,92,20022,92,12034,92,42872,38,708,94,710,94,5869,43,10133,43,66203,43,8249,60,10094,60,706,60,119350,60,5176,60,5810,60,5120,61,11840,61,12448,61,42239,61,8250,62,10095,62,707,62,119351,62,5171,62,94015,62,8275,126,732,126,8128,126,8764,126,65372,124,65293,45,120784,50,120794,50,120804,50,120814,50,120824,50,130034,50,42842,50,423,50,1000,50,42564,50,5311,50,42735,50,119302,51,120785,51,120795,51,120805,51,120815,51,120825,51,130035,51,42923,51,540,51,439,51,42858,51,11468,51,1248,51,94011,51,71882,51,120786,52,120796,52,120806,52,120816,52,120826,52,130036,52,5070,52,71855,52,120787,53,120797,53,120807,53,120817,53,120827,53,130037,53,444,53,71867,53,120788,54,120798,54,120808,54,120818,54,120828,54,130038,54,11474,54,5102,54,71893,54,119314,55,120789,55,120799,55,120809,55,120819,55,120829,55,130039,55,66770,55,71878,55,2819,56,2538,56,2666,56,125131,56,120790,56,120800,56,120810,56,120820,56,120830,56,130040,56,547,56,546,56,66330,56,2663,57,2920,57,2541,57,3437,57,120791,57,120801,57,120811,57,120821,57,120831,57,130041,57,42862,57,11466,57,71884,57,71852,57,71894,57,9082,97,65345,97,119834,97,119886,97,119938,97,119990,97,120042,97,120094,97,120146,97,120198,97,120250,97,120302,97,120354,97,120406,97,120458,97,593,97,945,97,120514,97,120572,97,120630,97,120688,97,120746,97,65313,65,119808,65,119860,65,119912,65,119964,65,120016,65,120068,65,120120,65,120172,65,120224,65,120276,65,120328,65,120380,65,120432,65,913,65,120488,65,120546,65,120604,65,120662,65,120720,65,5034,65,5573,65,42222,65,94016,65,66208,65,119835,98,119887,98,119939,98,119991,98,120043,98,120095,98,120147,98,120199,98,120251,98,120303,98,120355,98,120407,98,120459,98,388,98,5071,98,5234,98,5551,98,65314,66,8492,66,119809,66,119861,66,119913,66,120017,66,120069,66,120121,66,120173,66,120225,66,120277,66,120329,66,120381,66,120433,66,42932,66,914,66,120489,66,120547,66,120605,66,120663,66,120721,66,5108,66,5623,66,42192,66,66178,66,66209,66,66305,66,65347,99,8573,99,119836,99,119888,99,119940,99,119992,99,120044,99,120096,99,120148,99,120200,99,120252,99,120304,99,120356,99,120408,99,120460,99,7428,99,1010,99,11429,99,43951,99,66621,99,128844,67,71922,67,71913,67,65315,67,8557,67,8450,67,8493,67,119810,67,119862,67,119914,67,119966,67,120018,67,120174,67,120226,67,120278,67,120330,67,120382,67,120434,67,1017,67,11428,67,5087,67,42202,67,66210,67,66306,67,66581,67,66844,67,8574,100,8518,100,119837,100,119889,100,119941,100,119993,100,120045,100,120097,100,120149,100,120201,100,120253,100,120305,100,120357,100,120409,100,120461,100,1281,100,5095,100,5231,100,42194,100,8558,68,8517,68,119811,68,119863,68,119915,68,119967,68,120019,68,120071,68,120123,68,120175,68,120227,68,120279,68,120331,68,120383,68,120435,68,5024,68,5598,68,5610,68,42195,68,8494,101,65349,101,8495,101,8519,101,119838,101,119890,101,119942,101,120046,101,120098,101,120150,101,120202,101,120254,101,120306,101,120358,101,120410,101,120462,101,43826,101,1213,101,8959,69,65317,69,8496,69,119812,69,119864,69,119916,69,120020,69,120072,69,120124,69,120176,69,120228,69,120280,69,120332,69,120384,69,120436,69,917,69,120492,69,120550,69,120608,69,120666,69,120724,69,11577,69,5036,69,42224,69,71846,69,71854,69,66182,69,119839,102,119891,102,119943,102,119995,102,120047,102,120099,102,120151,102,120203,102,120255,102,120307,102,120359,102,120411,102,120463,102,43829,102,42905,102,383,102,7837,102,1412,102,119315,70,8497,70,119813,70,119865,70,119917,70,120021,70,120073,70,120125,70,120177,70,120229,70,120281,70,120333,70,120385,70,120437,70,42904,70,988,70,120778,70,5556,70,42205,70,71874,70,71842,70,66183,70,66213,70,66853,70,65351,103,8458,103,119840,103,119892,103,119944,103,120048,103,120100,103,120152,103,120204,103,120256,103,120308,103,120360,103,120412,103,120464,103,609,103,7555,103,397,103,1409,103,119814,71,119866,71,119918,71,119970,71,120022,71,120074,71,120126,71,120178,71,120230,71,120282,71,120334,71,120386,71,120438,71,1292,71,5056,71,5107,71,42198,71,65352,104,8462,104,119841,104,119945,104,119997,104,120049,104,120101,104,120153,104,120205,104,120257,104,120309,104,120361,104,120413,104,120465,104,1211,104,1392,104,5058,104,65320,72,8459,72,8460,72,8461,72,119815,72,119867,72,119919,72,120023,72,120179,72,120231,72,120283,72,120335,72,120387,72,120439,72,919,72,120494,72,120552,72,120610,72,120668,72,120726,72,11406,72,5051,72,5500,72,42215,72,66255,72,731,105,9075,105,65353,105,8560,105,8505,105,8520,105,119842,105,119894,105,119946,105,119998,105,120050,105,120102,105,120154,105,120206,105,120258,105,120310,105,120362,105,120414,105,120466,105,120484,105,618,105,617,105,953,105,8126,105,890,105,120522,105,120580,105,120638,105,120696,105,120754,105,1110,105,42567,105,1231,105,43893,105,5029,105,71875,105,65354,106,8521,106,119843,106,119895,106,119947,106,119999,106,120051,106,120103,106,120155,106,120207,106,120259,106,120311,106,120363,106,120415,106,120467,106,1011,106,1112,106,65322,74,119817,74,119869,74,119921,74,119973,74,120025,74,120077,74,120129,74,120181,74,120233,74,120285,74,120337,74,120389,74,120441,74,42930,74,895,74,1032,74,5035,74,5261,74,42201,74,119844,107,119896,107,119948,107,120000,107,120052,107,120104,107,120156,107,120208,107,120260,107,120312,107,120364,107,120416,107,120468,107,8490,75,65323,75,119818,75,119870,75,119922,75,119974,75,120026,75,120078,75,120130,75,120182,75,120234,75,120286,75,120338,75,120390,75,120442,75,922,75,120497,75,120555,75,120613,75,120671,75,120729,75,11412,75,5094,75,5845,75,42199,75,66840,75,1472,108,8739,73,9213,73,65512,73,1633,108,1777,73,66336,108,125127,108,120783,73,120793,73,120803,73,120813,73,120823,73,130033,73,65321,73,8544,73,8464,73,8465,73,119816,73,119868,73,119920,73,120024,73,120128,73,120180,73,120232,73,120284,73,120336,73,120388,73,120440,73,65356,108,8572,73,8467,108,119845,108,119897,108,119949,108,120001,108,120053,108,120105,73,120157,73,120209,73,120261,73,120313,73,120365,73,120417,73,120469,73,448,73,120496,73,120554,73,120612,73,120670,73,120728,73,11410,73,1030,73,1216,73,1493,108,1503,108,1575,108,126464,108,126592,108,65166,108,65165,108,1994,108,11599,73,5825,73,42226,73,93992,73,66186,124,66313,124,119338,76,8556,76,8466,76,119819,76,119871,76,119923,76,120027,76,120079,76,120131,76,120183,76,120235,76,120287,76,120339,76,120391,76,120443,76,11472,76,5086,76,5290,76,42209,76,93974,76,71843,76,71858,76,66587,76,66854,76,65325,77,8559,77,8499,77,119820,77,119872,77,119924,77,120028,77,120080,77,120132,77,120184,77,120236,77,120288,77,120340,77,120392,77,120444,77,924,77,120499,77,120557,77,120615,77,120673,77,120731,77,1018,77,11416,77,5047,77,5616,77,5846,77,42207,77,66224,77,66321,77,119847,110,119899,110,119951,110,120003,110,120055,110,120107,110,120159,110,120211,110,120263,110,120315,110,120367,110,120419,110,120471,110,1400,110,1404,110,65326,78,8469,78,119821,78,119873,78,119925,78,119977,78,120029,78,120081,78,120185,78,120237,78,120289,78,120341,78,120393,78,120445,78,925,78,120500,78,120558,78,120616,78,120674,78,120732,78,11418,78,42208,78,66835,78,3074,111,3202,111,3330,111,3458,111,2406,111,2662,111,2790,111,3046,111,3174,111,3302,111,3430,111,3664,111,3792,111,4160,111,1637,111,1781,111,65359,111,8500,111,119848,111,119900,111,119952,111,120056,111,120108,111,120160,111,120212,111,120264,111,120316,111,120368,111,120420,111,120472,111,7439,111,7441,111,43837,111,959,111,120528,111,120586,111,120644,111,120702,111,120760,111,963,111,120532,111,120590,111,120648,111,120706,111,120764,111,11423,111,4351,111,1413,111,1505,111,1607,111,126500,111,126564,111,126596,111,65259,111,65260,111,65258,111,65257,111,1726,111,64428,111,64429,111,64427,111,64426,111,1729,111,64424,111,64425,111,64423,111,64422,111,1749,111,3360,111,4125,111,66794,111,71880,111,71895,111,66604,111,1984,79,2534,79,2918,79,12295,79,70864,79,71904,79,120782,79,120792,79,120802,79,120812,79,120822,79,130032,79,65327,79,119822,79,119874,79,119926,79,119978,79,120030,79,120082,79,120134,79,120186,79,120238,79,120290,79,120342,79,120394,79,120446,79,927,79,120502,79,120560,79,120618,79,120676,79,120734,79,11422,79,1365,79,11604,79,4816,79,2848,79,66754,79,42227,79,71861,79,66194,79,66219,79,66564,79,66838,79,9076,112,65360,112,119849,112,119901,112,119953,112,120005,112,120057,112,120109,112,120161,112,120213,112,120265,112,120317,112,120369,112,120421,112,120473,112,961,112,120530,112,120544,112,120588,112,120602,112,120646,112,120660,112,120704,112,120718,112,120762,112,120776,112,11427,112,65328,80,8473,80,119823,80,119875,80,119927,80,119979,80,120031,80,120083,80,120187,80,120239,80,120291,80,120343,80,120395,80,120447,80,929,80,120504,80,120562,80,120620,80,120678,80,120736,80,11426,80,5090,80,5229,80,42193,80,66197,80,119850,113,119902,113,119954,113,120006,113,120058,113,120110,113,120162,113,120214,113,120266,113,120318,113,120370,113,120422,113,120474,113,1307,113,1379,113,1382,113,8474,81,119824,81,119876,81,119928,81,119980,81,120032,81,120084,81,120188,81,120240,81,120292,81,120344,81,120396,81,120448,81,11605,81,119851,114,119903,114,119955,114,120007,114,120059,114,120111,114,120163,114,120215,114,120267,114,120319,114,120371,114,120423,114,120475,114,43847,114,43848,114,7462,114,11397,114,43905,114,119318,82,8475,82,8476,82,8477,82,119825,82,119877,82,119929,82,120033,82,120189,82,120241,82,120293,82,120345,82,120397,82,120449,82,422,82,5025,82,5074,82,66740,82,5511,82,42211,82,94005,82,65363,115,119852,115,119904,115,119956,115,120008,115,120060,115,120112,115,120164,115,120216,115,120268,115,120320,115,120372,115,120424,115,120476,115,42801,115,445,115,1109,115,43946,115,71873,115,66632,115,65331,83,119826,83,119878,83,119930,83,119982,83,120034,83,120086,83,120138,83,120190,83,120242,83,120294,83,120346,83,120398,83,120450,83,1029,83,1359,83,5077,83,5082,83,42210,83,94010,83,66198,83,66592,83,119853,116,119905,116,119957,116,120009,116,120061,116,120113,116,120165,116,120217,116,120269,116,120321,116,120373,116,120425,116,120477,116,8868,84,10201,84,128872,84,65332,84,119827,84,119879,84,119931,84,119983,84,120035,84,120087,84,120139,84,120191,84,120243,84,120295,84,120347,84,120399,84,120451,84,932,84,120507,84,120565,84,120623,84,120681,84,120739,84,11430,84,5026,84,42196,84,93962,84,71868,84,66199,84,66225,84,66325,84,119854,117,119906,117,119958,117,120010,117,120062,117,120114,117,120166,117,120218,117,120270,117,120322,117,120374,117,120426,117,120478,117,42911,117,7452,117,43854,117,43858,117,651,117,965,117,120534,117,120592,117,120650,117,120708,117,120766,117,1405,117,66806,117,71896,117,8746,85,8899,85,119828,85,119880,85,119932,85,119984,85,120036,85,120088,85,120140,85,120192,85,120244,85,120296,85,120348,85,120400,85,120452,85,1357,85,4608,85,66766,85,5196,85,42228,85,94018,85,71864,85,8744,118,8897,118,65366,118,8564,118,119855,118,119907,118,119959,118,120011,118,120063,118,120115,118,120167,118,120219,118,120271,118,120323,118,120375,118,120427,118,120479,118,7456,118,957,118,120526,118,120584,118,120642,118,120700,118,120758,118,1141,118,1496,118,71430,118,43945,118,71872,118,119309,86,1639,86,1783,86,8548,86,119829,86,119881,86,119933,86,119985,86,120037,86,120089,86,120141,86,120193,86,120245,86,120297,86,120349,86,120401,86,120453,86,1140,86,11576,86,5081,86,5167,86,42719,86,42214,86,93960,86,71840,86,66845,86,623,119,119856,119,119908,119,119960,119,120012,119,120064,119,120116,119,120168,119,120220,119,120272,119,120324,119,120376,119,120428,119,120480,119,7457,119,1121,119,1309,119,1377,119,71434,119,71438,119,71439,119,43907,119,71919,87,71910,87,119830,87,119882,87,119934,87,119986,87,120038,87,120090,87,120142,87,120194,87,120246,87,120298,87,120350,87,120402,87,120454,87,1308,87,5043,87,5076,87,42218,87,5742,120,10539,120,10540,120,10799,120,65368,120,8569,120,119857,120,119909,120,119961,120,120013,120,120065,120,120117,120,120169,120,120221,120,120273,120,120325,120,120377,120,120429,120,120481,120,5441,120,5501,120,5741,88,9587,88,66338,88,71916,88,65336,88,8553,88,119831,88,119883,88,119935,88,119987,88,120039,88,120091,88,120143,88,120195,88,120247,88,120299,88,120351,88,120403,88,120455,88,42931,88,935,88,120510,88,120568,88,120626,88,120684,88,120742,88,11436,88,11613,88,5815,88,42219,88,66192,88,66228,88,66327,88,66855,88,611,121,7564,121,65369,121,119858,121,119910,121,119962,121,120014,121,120066,121,120118,121,120170,121,120222,121,120274,121,120326,121,120378,121,120430,121,120482,121,655,121,7935,121,43866,121,947,121,8509,121,120516,121,120574,121,120632,121,120690,121,120748,121,1199,121,4327,121,71900,121,65337,89,119832,89,119884,89,119936,89,119988,89,120040,89,120092,89,120144,89,120196,89,120248,89,120300,89,120352,89,120404,89,120456,89,933,89,978,89,120508,89,120566,89,120624,89,120682,89,120740,89,11432,89,1198,89,5033,89,5053,89,42220,89,94019,89,71844,89,66226,89,119859,122,119911,122,119963,122,120015,122,120067,122,120119,122,120171,122,120223,122,120275,122,120327,122,120379,122,120431,122,120483,122,7458,122,43923,122,71876,122,66293,90,71909,90,65338,90,8484,90,8488,90,119833,90,119885,90,119937,90,119989,90,120041,90,120197,90,120249,90,120301,90,120353,90,120405,90,120457,90,918,90,120493,90,120551,90,120609,90,120667,90,120725,90,5059,90,42204,90,71849,90,65282,34,65284,36,65285,37,65286,38,65290,42,65291,43,65294,46,65295,47,65296,48,65297,49,65298,50,65299,51,65300,52,65301,53,65302,54,65303,55,65304,56,65305,57,65308,60,65309,61,65310,62,65312,64,65316,68,65318,70,65319,71,65324,76,65329,81,65330,82,65333,85,65334,86,65335,87,65343,95,65346,98,65348,100,65350,102,65355,107,65357,109,65358,110,65361,113,65362,114,65364,116,65365,117,65367,119,65370,122,65371,123,65373,125,119846,109],"_default":[160,32,8211,45,65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"cs":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"de":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"es":[8211,45,65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"fr":[65374,126,65306,58,65281,33,8216,96,8245,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"it":[160,32,8211,45,65374,126,65306,58,65281,33,8216,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"ja":[8211,45,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65292,44,65307,59],"ko":[8211,45,65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"pl":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"pt-BR":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"qps-ploc":[160,32,8211,45,65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"ru":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,305,105,921,73,1009,112,215,120,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"tr":[160,32,8211,45,65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"zh-hans":[65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65288,40,65289,41],"zh-hant":[8211,45,65374,126,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65307,59]}'));
lt.cache = new Ga((e) => {
  function t(u) {
    const c = /* @__PURE__ */ new Map();
    for (let f = 0; f < u.length; f += 2)
      c.set(u[f], u[f + 1]);
    return c;
  }
  function n(u, c) {
    const f = new Map(u);
    for (const [d, g] of c)
      f.set(d, g);
    return f;
  }
  function r(u, c) {
    if (!u)
      return c;
    const f = /* @__PURE__ */ new Map();
    for (const [d, g] of u)
      c.has(d) && f.set(d, g);
    return f;
  }
  const i = Nt.ambiguousCharacterData.value;
  let s = e.filter((u) => !u.startsWith("_") && u in i);
  s.length === 0 && (s = ["_default"]);
  let a;
  for (const u of s) {
    const c = t(i[u]);
    a = r(a, c);
  }
  const o = t(i._common), l = n(o, a);
  return new Nt(l);
});
lt._locales = new Js(() => Object.keys(Nt.ambiguousCharacterData.value).filter((e) => !e.startsWith("_")));
class et {
  static getRawData() {
    return JSON.parse("[9,10,11,12,13,32,127,160,173,847,1564,4447,4448,6068,6069,6155,6156,6157,6158,7355,7356,8192,8193,8194,8195,8196,8197,8198,8199,8200,8201,8202,8203,8204,8205,8206,8207,8234,8235,8236,8237,8238,8239,8287,8288,8289,8290,8291,8292,8293,8294,8295,8296,8297,8298,8299,8300,8301,8302,8303,10240,12288,12644,65024,65025,65026,65027,65028,65029,65030,65031,65032,65033,65034,65035,65036,65037,65038,65039,65279,65440,65520,65521,65522,65523,65524,65525,65526,65527,65528,65532,78844,119155,119156,119157,119158,119159,119160,119161,119162,917504,917505,917506,917507,917508,917509,917510,917511,917512,917513,917514,917515,917516,917517,917518,917519,917520,917521,917522,917523,917524,917525,917526,917527,917528,917529,917530,917531,917532,917533,917534,917535,917536,917537,917538,917539,917540,917541,917542,917543,917544,917545,917546,917547,917548,917549,917550,917551,917552,917553,917554,917555,917556,917557,917558,917559,917560,917561,917562,917563,917564,917565,917566,917567,917568,917569,917570,917571,917572,917573,917574,917575,917576,917577,917578,917579,917580,917581,917582,917583,917584,917585,917586,917587,917588,917589,917590,917591,917592,917593,917594,917595,917596,917597,917598,917599,917600,917601,917602,917603,917604,917605,917606,917607,917608,917609,917610,917611,917612,917613,917614,917615,917616,917617,917618,917619,917620,917621,917622,917623,917624,917625,917626,917627,917628,917629,917630,917631,917760,917761,917762,917763,917764,917765,917766,917767,917768,917769,917770,917771,917772,917773,917774,917775,917776,917777,917778,917779,917780,917781,917782,917783,917784,917785,917786,917787,917788,917789,917790,917791,917792,917793,917794,917795,917796,917797,917798,917799,917800,917801,917802,917803,917804,917805,917806,917807,917808,917809,917810,917811,917812,917813,917814,917815,917816,917817,917818,917819,917820,917821,917822,917823,917824,917825,917826,917827,917828,917829,917830,917831,917832,917833,917834,917835,917836,917837,917838,917839,917840,917841,917842,917843,917844,917845,917846,917847,917848,917849,917850,917851,917852,917853,917854,917855,917856,917857,917858,917859,917860,917861,917862,917863,917864,917865,917866,917867,917868,917869,917870,917871,917872,917873,917874,917875,917876,917877,917878,917879,917880,917881,917882,917883,917884,917885,917886,917887,917888,917889,917890,917891,917892,917893,917894,917895,917896,917897,917898,917899,917900,917901,917902,917903,917904,917905,917906,917907,917908,917909,917910,917911,917912,917913,917914,917915,917916,917917,917918,917919,917920,917921,917922,917923,917924,917925,917926,917927,917928,917929,917930,917931,917932,917933,917934,917935,917936,917937,917938,917939,917940,917941,917942,917943,917944,917945,917946,917947,917948,917949,917950,917951,917952,917953,917954,917955,917956,917957,917958,917959,917960,917961,917962,917963,917964,917965,917966,917967,917968,917969,917970,917971,917972,917973,917974,917975,917976,917977,917978,917979,917980,917981,917982,917983,917984,917985,917986,917987,917988,917989,917990,917991,917992,917993,917994,917995,917996,917997,917998,917999]");
  }
  static getData() {
    return this._data || (this._data = new Set(et.getRawData())), this._data;
  }
  static isInvisibleCharacter(t) {
    return et.getData().has(t);
  }
  static get codePoints() {
    return et.getData();
  }
}
et._data = void 0;
const ro = "$initialize";
class io {
  constructor(t, n, r, i) {
    this.vsWorker = t, this.req = n, this.method = r, this.args = i, this.type = 0;
  }
}
class $r {
  constructor(t, n, r, i) {
    this.vsWorker = t, this.seq = n, this.res = r, this.err = i, this.type = 1;
  }
}
class so {
  constructor(t, n, r, i) {
    this.vsWorker = t, this.req = n, this.eventName = r, this.arg = i, this.type = 2;
  }
}
class ao {
  constructor(t, n, r) {
    this.vsWorker = t, this.req = n, this.event = r, this.type = 3;
  }
}
class oo {
  constructor(t, n) {
    this.vsWorker = t, this.req = n, this.type = 4;
  }
}
class lo {
  constructor(t) {
    this._workerId = -1, this._handler = t, this._lastSentReq = 0, this._pendingReplies = /* @__PURE__ */ Object.create(null), this._pendingEmitters = /* @__PURE__ */ new Map(), this._pendingEvents = /* @__PURE__ */ new Map();
  }
  setWorkerId(t) {
    this._workerId = t;
  }
  sendMessage(t, n) {
    const r = String(++this._lastSentReq);
    return new Promise((i, s) => {
      this._pendingReplies[r] = {
        resolve: i,
        reject: s
      }, this._send(new io(this._workerId, r, t, n));
    });
  }
  listen(t, n) {
    let r = null;
    const i = new Ae({
      onWillAddFirstListener: () => {
        r = String(++this._lastSentReq), this._pendingEmitters.set(r, i), this._send(new so(this._workerId, r, t, n));
      },
      onDidRemoveLastListener: () => {
        this._pendingEmitters.delete(r), this._send(new oo(this._workerId, r)), r = null;
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
    const n = this._pendingReplies[t.seq];
    if (delete this._pendingReplies[t.seq], t.err) {
      let r = t.err;
      t.err.$isError && (r = new Error(), r.name = t.err.name, r.message = t.err.message, r.stack = t.err.stack), n.reject(r);
      return;
    }
    n.resolve(t.res);
  }
  _handleRequestMessage(t) {
    const n = t.req;
    this._handler.handleMessage(t.method, t.args).then((i) => {
      this._send(new $r(this._workerId, n, i, void 0));
    }, (i) => {
      i.detail instanceof Error && (i.detail = Br(i.detail)), this._send(new $r(this._workerId, n, void 0, Br(i)));
    });
  }
  _handleSubscribeEventMessage(t) {
    const n = t.req, r = this._handler.handleEvent(t.eventName, t.arg)((i) => {
      this._send(new ao(this._workerId, n, i));
    });
    this._pendingEvents.set(n, r);
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
    const n = [];
    if (t.type === 0)
      for (let r = 0; r < t.args.length; r++)
        t.args[r] instanceof ArrayBuffer && n.push(t.args[r]);
    else
      t.type === 1 && t.res instanceof ArrayBuffer && n.push(t.res);
    this._handler.sendMessage(t, n);
  }
}
function Qs(e) {
  return e[0] === "o" && e[1] === "n" && Xs(e.charCodeAt(2));
}
function Zs(e) {
  return /^onDynamic/.test(e) && Xs(e.charCodeAt(9));
}
function uo(e, t, n) {
  const r = (a) => function() {
    const o = Array.prototype.slice.call(arguments, 0);
    return t(a, o);
  }, i = (a) => function(o) {
    return n(a, o);
  }, s = {};
  for (const a of e) {
    if (Zs(a)) {
      s[a] = i(a);
      continue;
    }
    if (Qs(a)) {
      s[a] = n(a, void 0);
      continue;
    }
    s[a] = r(a);
  }
  return s;
}
class co {
  constructor(t, n) {
    this._requestHandlerFactory = n, this._requestHandler = null, this._protocol = new lo({
      sendMessage: (r, i) => {
        t(r, i);
      },
      handleMessage: (r, i) => this._handleMessage(r, i),
      handleEvent: (r, i) => this._handleEvent(r, i)
    });
  }
  onmessage(t) {
    this._protocol.handleMessage(t);
  }
  _handleMessage(t, n) {
    if (t === ro)
      return this.initialize(n[0], n[1], n[2], n[3]);
    if (!this._requestHandler || typeof this._requestHandler[t] != "function")
      return Promise.reject(new Error("Missing requestHandler or method: " + t));
    try {
      return Promise.resolve(this._requestHandler[t].apply(this._requestHandler, n));
    } catch (r) {
      return Promise.reject(r);
    }
  }
  _handleEvent(t, n) {
    if (!this._requestHandler)
      throw new Error("Missing requestHandler");
    if (Zs(t)) {
      const r = this._requestHandler[t].call(this._requestHandler, n);
      if (typeof r != "function")
        throw new Error(`Missing dynamic event ${t} on request handler.`);
      return r;
    }
    if (Qs(t)) {
      const r = this._requestHandler[t];
      if (typeof r != "function")
        throw new Error(`Missing event ${t} on request handler.`);
      return r;
    }
    throw new Error(`Malformed event name ${t}`);
  }
  initialize(t, n, r, i) {
    this._protocol.setWorkerId(t);
    const o = uo(i, (l, u) => this._protocol.sendMessage(l, u), (l, u) => this._protocol.listen(l, u));
    return this._requestHandlerFactory ? (this._requestHandler = this._requestHandlerFactory(o), Promise.resolve(Qn(this._requestHandler))) : (n && (typeof n.baseUrl < "u" && delete n.baseUrl, typeof n.paths < "u" && typeof n.paths.vs < "u" && delete n.paths.vs, typeof n.trustedTypesPolicy !== void 0 && delete n.trustedTypesPolicy, n.catchError = !0, globalThis.require.config(n)), new Promise((l, u) => {
      const c = globalThis.require;
      c([r], (f) => {
        if (this._requestHandler = f.create(o), !this._requestHandler) {
          u(new Error("No RequestHandler!"));
          return;
        }
        l(Qn(this._requestHandler));
      }, u);
    }));
  }
}
class Qe {
  /**
   * Constructs a new DiffChange with the given sequence information
   * and content.
   */
  constructor(t, n, r, i) {
    this.originalStart = t, this.originalLength = n, this.modifiedStart = r, this.modifiedLength = i;
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
function Wr(e, t) {
  return (t << 5) - t + e | 0;
}
function fo(e, t) {
  t = Wr(149417, t);
  for (let n = 0, r = e.length; n < r; n++)
    t = Wr(e.charCodeAt(n), t);
  return t;
}
class Hr {
  constructor(t) {
    this.source = t;
  }
  getElements() {
    const t = this.source, n = new Int32Array(t.length);
    for (let r = 0, i = t.length; r < i; r++)
      n[r] = t.charCodeAt(r);
    return n;
  }
}
function ho(e, t, n) {
  return new Ye(new Hr(e), new Hr(t)).ComputeDiff(n).changes;
}
class ft {
  static Assert(t, n) {
    if (!t)
      throw new Error(n);
  }
}
class ht {
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
  static Copy(t, n, r, i, s) {
    for (let a = 0; a < s; a++)
      r[i + a] = t[n + a];
  }
  static Copy2(t, n, r, i, s) {
    for (let a = 0; a < s; a++)
      r[i + a] = t[n + a];
  }
}
class zr {
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
    (this.m_originalCount > 0 || this.m_modifiedCount > 0) && this.m_changes.push(new Qe(this.m_originalStart, this.m_originalCount, this.m_modifiedStart, this.m_modifiedCount)), this.m_originalCount = 0, this.m_modifiedCount = 0, this.m_originalStart = 1073741824, this.m_modifiedStart = 1073741824;
  }
  /**
   * Adds the original element at the given position to the elements
   * affected by the current change. The modified index gives context
   * to the change position with respect to the original sequence.
   * @param originalIndex The index of the original element to add.
   * @param modifiedIndex The index of the modified element that provides corresponding position in the modified sequence.
   */
  AddOriginalElement(t, n) {
    this.m_originalStart = Math.min(this.m_originalStart, t), this.m_modifiedStart = Math.min(this.m_modifiedStart, n), this.m_originalCount++;
  }
  /**
   * Adds the modified element at the given position to the elements
   * affected by the current change. The original index gives context
   * to the change position with respect to the modified sequence.
   * @param originalIndex The index of the original element that provides corresponding position in the original sequence.
   * @param modifiedIndex The index of the modified element to add.
   */
  AddModifiedElement(t, n) {
    this.m_originalStart = Math.min(this.m_originalStart, t), this.m_modifiedStart = Math.min(this.m_modifiedStart, n), this.m_modifiedCount++;
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
class Ye {
  /**
   * Constructs the DiffFinder
   */
  constructor(t, n, r = null) {
    this.ContinueProcessingPredicate = r, this._originalSequence = t, this._modifiedSequence = n;
    const [i, s, a] = Ye._getElements(t), [o, l, u] = Ye._getElements(n);
    this._hasStrings = a && u, this._originalStringElements = i, this._originalElementsOrHash = s, this._modifiedStringElements = o, this._modifiedElementsOrHash = l, this.m_forwardHistory = [], this.m_reverseHistory = [];
  }
  static _isStringArray(t) {
    return t.length > 0 && typeof t[0] == "string";
  }
  static _getElements(t) {
    const n = t.getElements();
    if (Ye._isStringArray(n)) {
      const r = new Int32Array(n.length);
      for (let i = 0, s = n.length; i < s; i++)
        r[i] = fo(n[i], 0);
      return [n, r, !0];
    }
    return n instanceof Int32Array ? [[], n, !1] : [[], new Int32Array(n), !1];
  }
  ElementsAreEqual(t, n) {
    return this._originalElementsOrHash[t] !== this._modifiedElementsOrHash[n] ? !1 : this._hasStrings ? this._originalStringElements[t] === this._modifiedStringElements[n] : !0;
  }
  ElementsAreStrictEqual(t, n) {
    if (!this.ElementsAreEqual(t, n))
      return !1;
    const r = Ye._getStrictElement(this._originalSequence, t), i = Ye._getStrictElement(this._modifiedSequence, n);
    return r === i;
  }
  static _getStrictElement(t, n) {
    return typeof t.getStrictElement == "function" ? t.getStrictElement(n) : null;
  }
  OriginalElementsAreEqual(t, n) {
    return this._originalElementsOrHash[t] !== this._originalElementsOrHash[n] ? !1 : this._hasStrings ? this._originalStringElements[t] === this._originalStringElements[n] : !0;
  }
  ModifiedElementsAreEqual(t, n) {
    return this._modifiedElementsOrHash[t] !== this._modifiedElementsOrHash[n] ? !1 : this._hasStrings ? this._modifiedStringElements[t] === this._modifiedStringElements[n] : !0;
  }
  ComputeDiff(t) {
    return this._ComputeDiff(0, this._originalElementsOrHash.length - 1, 0, this._modifiedElementsOrHash.length - 1, t);
  }
  /**
   * Computes the differences between the original and modified input
   * sequences on the bounded range.
   * @returns An array of the differences between the two input sequences.
   */
  _ComputeDiff(t, n, r, i, s) {
    const a = [!1];
    let o = this.ComputeDiffRecursive(t, n, r, i, a);
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
  ComputeDiffRecursive(t, n, r, i, s) {
    for (s[0] = !1; t <= n && r <= i && this.ElementsAreEqual(t, r); )
      t++, r++;
    for (; n >= t && i >= r && this.ElementsAreEqual(n, i); )
      n--, i--;
    if (t > n || r > i) {
      let f;
      return r <= i ? (ft.Assert(t === n + 1, "originalStart should only be one more than originalEnd"), f = [
        new Qe(t, 0, r, i - r + 1)
      ]) : t <= n ? (ft.Assert(r === i + 1, "modifiedStart should only be one more than modifiedEnd"), f = [
        new Qe(t, n - t + 1, r, 0)
      ]) : (ft.Assert(t === n + 1, "originalStart should only be one more than originalEnd"), ft.Assert(r === i + 1, "modifiedStart should only be one more than modifiedEnd"), f = []), f;
    }
    const a = [0], o = [0], l = this.ComputeRecursionPoint(t, n, r, i, a, o, s), u = a[0], c = o[0];
    if (l !== null)
      return l;
    if (!s[0]) {
      const f = this.ComputeDiffRecursive(t, u, r, c, s);
      let d = [];
      return s[0] ? d = [
        new Qe(u + 1, n - (u + 1) + 1, c + 1, i - (c + 1) + 1)
      ] : d = this.ComputeDiffRecursive(u + 1, n, c + 1, i, s), this.ConcatenateChanges(f, d);
    }
    return [
      new Qe(t, n - t + 1, r, i - r + 1)
    ];
  }
  WALKTRACE(t, n, r, i, s, a, o, l, u, c, f, d, g, m, p, v, x, b) {
    let y = null, _ = null, N = new zr(), w = n, S = r, C = g[0] - v[0] - i, M = -1073741824, I = this.m_forwardHistory.length - 1;
    do {
      const D = C + t;
      D === w || D < S && u[D - 1] < u[D + 1] ? (f = u[D + 1], m = f - C - i, f < M && N.MarkNextChange(), M = f, N.AddModifiedElement(f + 1, m), C = D + 1 - t) : (f = u[D - 1] + 1, m = f - C - i, f < M && N.MarkNextChange(), M = f - 1, N.AddOriginalElement(f, m + 1), C = D - 1 - t), I >= 0 && (u = this.m_forwardHistory[I], t = u[0], w = 1, S = u.length - 1);
    } while (--I >= -1);
    if (y = N.getReverseChanges(), b[0]) {
      let D = g[0] + 1, L = v[0] + 1;
      if (y !== null && y.length > 0) {
        const E = y[y.length - 1];
        D = Math.max(D, E.getOriginalEnd()), L = Math.max(L, E.getModifiedEnd());
      }
      _ = [
        new Qe(D, d - D + 1, L, p - L + 1)
      ];
    } else {
      N = new zr(), w = a, S = o, C = g[0] - v[0] - l, M = 1073741824, I = x ? this.m_reverseHistory.length - 1 : this.m_reverseHistory.length - 2;
      do {
        const D = C + s;
        D === w || D < S && c[D - 1] >= c[D + 1] ? (f = c[D + 1] - 1, m = f - C - l, f > M && N.MarkNextChange(), M = f + 1, N.AddOriginalElement(f + 1, m + 1), C = D + 1 - s) : (f = c[D - 1], m = f - C - l, f > M && N.MarkNextChange(), M = f, N.AddModifiedElement(f + 1, m + 1), C = D - 1 - s), I >= 0 && (c = this.m_reverseHistory[I], s = c[0], w = 1, S = c.length - 1);
      } while (--I >= -1);
      _ = N.getChanges();
    }
    return this.ConcatenateChanges(y, _);
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
  ComputeRecursionPoint(t, n, r, i, s, a, o) {
    let l = 0, u = 0, c = 0, f = 0, d = 0, g = 0;
    t--, r--, s[0] = 0, a[0] = 0, this.m_forwardHistory = [], this.m_reverseHistory = [];
    const m = n - t + (i - r), p = m + 1, v = new Int32Array(p), x = new Int32Array(p), b = i - r, y = n - t, _ = t - r, N = n - i, S = (y - b) % 2 === 0;
    v[b] = t, x[y] = n, o[0] = !1;
    for (let C = 1; C <= m / 2 + 1; C++) {
      let M = 0, I = 0;
      c = this.ClipDiagonalBound(b - C, C, b, p), f = this.ClipDiagonalBound(b + C, C, b, p);
      for (let L = c; L <= f; L += 2) {
        L === c || L < f && v[L - 1] < v[L + 1] ? l = v[L + 1] : l = v[L - 1] + 1, u = l - (L - b) - _;
        const E = l;
        for (; l < n && u < i && this.ElementsAreEqual(l + 1, u + 1); )
          l++, u++;
        if (v[L] = l, l + u > M + I && (M = l, I = u), !S && Math.abs(L - y) <= C - 1 && l >= x[L])
          return s[0] = l, a[0] = u, E <= x[L] && 1447 > 0 && C <= 1447 + 1 ? this.WALKTRACE(b, c, f, _, y, d, g, N, v, x, l, n, s, u, i, a, S, o) : null;
      }
      const D = (M - t + (I - r) - C) / 2;
      if (this.ContinueProcessingPredicate !== null && !this.ContinueProcessingPredicate(M, D))
        return o[0] = !0, s[0] = M, a[0] = I, D > 0 && 1447 > 0 && C <= 1447 + 1 ? this.WALKTRACE(b, c, f, _, y, d, g, N, v, x, l, n, s, u, i, a, S, o) : (t++, r++, [
          new Qe(t, n - t + 1, r, i - r + 1)
        ]);
      d = this.ClipDiagonalBound(y - C, C, y, p), g = this.ClipDiagonalBound(y + C, C, y, p);
      for (let L = d; L <= g; L += 2) {
        L === d || L < g && x[L - 1] >= x[L + 1] ? l = x[L + 1] - 1 : l = x[L - 1], u = l - (L - y) - N;
        const E = l;
        for (; l > t && u > r && this.ElementsAreEqual(l, u); )
          l--, u--;
        if (x[L] = l, S && Math.abs(L - b) <= C && l <= v[L])
          return s[0] = l, a[0] = u, E >= v[L] && 1447 > 0 && C <= 1447 + 1 ? this.WALKTRACE(b, c, f, _, y, d, g, N, v, x, l, n, s, u, i, a, S, o) : null;
      }
      if (C <= 1447) {
        let L = new Int32Array(f - c + 2);
        L[0] = b - c + 1, ht.Copy2(v, c, L, 1, f - c + 1), this.m_forwardHistory.push(L), L = new Int32Array(g - d + 2), L[0] = y - d + 1, ht.Copy2(x, d, L, 1, g - d + 1), this.m_reverseHistory.push(L);
      }
    }
    return this.WALKTRACE(b, c, f, _, y, d, g, N, v, x, l, n, s, u, i, a, S, o);
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
    for (let n = 0; n < t.length; n++) {
      const r = t[n], i = n < t.length - 1 ? t[n + 1].originalStart : this._originalElementsOrHash.length, s = n < t.length - 1 ? t[n + 1].modifiedStart : this._modifiedElementsOrHash.length, a = r.originalLength > 0, o = r.modifiedLength > 0;
      for (; r.originalStart + r.originalLength < i && r.modifiedStart + r.modifiedLength < s && (!a || this.OriginalElementsAreEqual(r.originalStart, r.originalStart + r.originalLength)) && (!o || this.ModifiedElementsAreEqual(r.modifiedStart, r.modifiedStart + r.modifiedLength)); ) {
        const u = this.ElementsAreStrictEqual(r.originalStart, r.modifiedStart);
        if (this.ElementsAreStrictEqual(r.originalStart + r.originalLength, r.modifiedStart + r.modifiedLength) && !u)
          break;
        r.originalStart++, r.modifiedStart++;
      }
      const l = [null];
      if (n < t.length - 1 && this.ChangesOverlap(t[n], t[n + 1], l)) {
        t[n] = l[0], t.splice(n + 1, 1), n--;
        continue;
      }
    }
    for (let n = t.length - 1; n >= 0; n--) {
      const r = t[n];
      let i = 0, s = 0;
      if (n > 0) {
        const f = t[n - 1];
        i = f.originalStart + f.originalLength, s = f.modifiedStart + f.modifiedLength;
      }
      const a = r.originalLength > 0, o = r.modifiedLength > 0;
      let l = 0, u = this._boundaryScore(r.originalStart, r.originalLength, r.modifiedStart, r.modifiedLength);
      for (let f = 1; ; f++) {
        const d = r.originalStart - f, g = r.modifiedStart - f;
        if (d < i || g < s || a && !this.OriginalElementsAreEqual(d, d + r.originalLength) || o && !this.ModifiedElementsAreEqual(g, g + r.modifiedLength))
          break;
        const p = (d === i && g === s ? 5 : 0) + this._boundaryScore(d, r.originalLength, g, r.modifiedLength);
        p > u && (u = p, l = f);
      }
      r.originalStart -= l, r.modifiedStart -= l;
      const c = [null];
      if (n > 0 && this.ChangesOverlap(t[n - 1], t[n], c)) {
        t[n - 1] = c[0], t.splice(n, 1), n++;
        continue;
      }
    }
    if (this._hasStrings)
      for (let n = 1, r = t.length; n < r; n++) {
        const i = t[n - 1], s = t[n], a = s.originalStart - i.originalStart - i.originalLength, o = i.originalStart, l = s.originalStart + s.originalLength, u = l - o, c = i.modifiedStart, f = s.modifiedStart + s.modifiedLength, d = f - c;
        if (a < 5 && u < 20 && d < 20) {
          const g = this._findBetterContiguousSequence(o, u, c, d, a);
          if (g) {
            const [m, p] = g;
            (m !== i.originalStart + i.originalLength || p !== i.modifiedStart + i.modifiedLength) && (i.originalLength = m - i.originalStart, i.modifiedLength = p - i.modifiedStart, s.originalStart = m + a, s.modifiedStart = p + a, s.originalLength = l - s.originalStart, s.modifiedLength = f - s.modifiedStart);
          }
        }
      }
    return t;
  }
  _findBetterContiguousSequence(t, n, r, i, s) {
    if (n < s || i < s)
      return null;
    const a = t + n - s + 1, o = r + i - s + 1;
    let l = 0, u = 0, c = 0;
    for (let f = t; f < a; f++)
      for (let d = r; d < o; d++) {
        const g = this._contiguousSequenceScore(f, d, s);
        g > 0 && g > l && (l = g, u = f, c = d);
      }
    return l > 0 ? [u, c] : null;
  }
  _contiguousSequenceScore(t, n, r) {
    let i = 0;
    for (let s = 0; s < r; s++) {
      if (!this.ElementsAreEqual(t + s, n + s))
        return 0;
      i += this._originalStringElements[t + s].length;
    }
    return i;
  }
  _OriginalIsBoundary(t) {
    return t <= 0 || t >= this._originalElementsOrHash.length - 1 ? !0 : this._hasStrings && /^\s*$/.test(this._originalStringElements[t]);
  }
  _OriginalRegionIsBoundary(t, n) {
    if (this._OriginalIsBoundary(t) || this._OriginalIsBoundary(t - 1))
      return !0;
    if (n > 0) {
      const r = t + n;
      if (this._OriginalIsBoundary(r - 1) || this._OriginalIsBoundary(r))
        return !0;
    }
    return !1;
  }
  _ModifiedIsBoundary(t) {
    return t <= 0 || t >= this._modifiedElementsOrHash.length - 1 ? !0 : this._hasStrings && /^\s*$/.test(this._modifiedStringElements[t]);
  }
  _ModifiedRegionIsBoundary(t, n) {
    if (this._ModifiedIsBoundary(t) || this._ModifiedIsBoundary(t - 1))
      return !0;
    if (n > 0) {
      const r = t + n;
      if (this._ModifiedIsBoundary(r - 1) || this._ModifiedIsBoundary(r))
        return !0;
    }
    return !1;
  }
  _boundaryScore(t, n, r, i) {
    const s = this._OriginalRegionIsBoundary(t, n) ? 1 : 0, a = this._ModifiedRegionIsBoundary(r, i) ? 1 : 0;
    return s + a;
  }
  /**
   * Concatenates the two input DiffChange lists and returns the resulting
   * list.
   * @param The left changes
   * @param The right changes
   * @returns The concatenated list
   */
  ConcatenateChanges(t, n) {
    const r = [];
    if (t.length === 0 || n.length === 0)
      return n.length > 0 ? n : t;
    if (this.ChangesOverlap(t[t.length - 1], n[0], r)) {
      const i = new Array(t.length + n.length - 1);
      return ht.Copy(t, 0, i, 0, t.length - 1), i[t.length - 1] = r[0], ht.Copy(n, 1, i, t.length, n.length - 1), i;
    } else {
      const i = new Array(t.length + n.length);
      return ht.Copy(t, 0, i, 0, t.length), ht.Copy(n, 0, i, t.length, n.length), i;
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
  ChangesOverlap(t, n, r) {
    if (ft.Assert(t.originalStart <= n.originalStart, "Left change is not less than or equal to right change"), ft.Assert(t.modifiedStart <= n.modifiedStart, "Left change is not less than or equal to right change"), t.originalStart + t.originalLength >= n.originalStart || t.modifiedStart + t.modifiedLength >= n.modifiedStart) {
      const i = t.originalStart;
      let s = t.originalLength;
      const a = t.modifiedStart;
      let o = t.modifiedLength;
      return t.originalStart + t.originalLength >= n.originalStart && (s = n.originalStart + n.originalLength - t.originalStart), t.modifiedStart + t.modifiedLength >= n.modifiedStart && (o = n.modifiedStart + n.modifiedLength - t.modifiedStart), r[0] = new Qe(i, s, a, o), !0;
    } else
      return r[0] = null, !1;
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
  ClipDiagonalBound(t, n, r, i) {
    if (t >= 0 && t < i)
      return t;
    const s = r, a = i - r - 1, o = n % 2 === 0;
    if (t < 0) {
      const l = s % 2 === 0;
      return o === l ? 0 : 1;
    } else {
      const l = a % 2 === 0;
      return o === l ? i - 1 : i - 2;
    }
  }
}
let yt;
if (typeof Ee.vscode < "u" && typeof Ee.vscode.process < "u") {
  const e = Ee.vscode.process;
  yt = {
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
  typeof process < "u" ? yt = {
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
  } : yt = {
    // Supported
    get platform() {
      return qt ? "win32" : Wa ? "darwin" : "linux";
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
const pn = yt.cwd, go = yt.env, mo = yt.platform, po = 65, vo = 97, bo = 90, yo = 122, tt = 46, le = 47, ge = 92, He = 58, xo = 63;
class Ys extends Error {
  constructor(t, n, r) {
    let i;
    typeof n == "string" && n.indexOf("not ") === 0 ? (i = "must not be", n = n.replace(/^not /, "")) : i = "must be";
    const s = t.indexOf(".") !== -1 ? "property" : "argument";
    let a = `The "${t}" ${s} ${i} of type ${n}`;
    a += `. Received type ${typeof r}`, super(a), this.code = "ERR_INVALID_ARG_TYPE";
  }
}
function wo(e, t) {
  if (e === null || typeof e != "object")
    throw new Ys(t, "Object", e);
}
function K(e, t) {
  if (typeof e != "string")
    throw new Ys(t, "string", e);
}
const it = mo === "win32";
function H(e) {
  return e === le || e === ge;
}
function er(e) {
  return e === le;
}
function ze(e) {
  return e >= po && e <= bo || e >= vo && e <= yo;
}
function vn(e, t, n, r) {
  let i = "", s = 0, a = -1, o = 0, l = 0;
  for (let u = 0; u <= e.length; ++u) {
    if (u < e.length)
      l = e.charCodeAt(u);
    else {
      if (r(l))
        break;
      l = le;
    }
    if (r(l)) {
      if (!(a === u - 1 || o === 1))
        if (o === 2) {
          if (i.length < 2 || s !== 2 || i.charCodeAt(i.length - 1) !== tt || i.charCodeAt(i.length - 2) !== tt) {
            if (i.length > 2) {
              const c = i.lastIndexOf(n);
              c === -1 ? (i = "", s = 0) : (i = i.slice(0, c), s = i.length - 1 - i.lastIndexOf(n)), a = u, o = 0;
              continue;
            } else if (i.length !== 0) {
              i = "", s = 0, a = u, o = 0;
              continue;
            }
          }
          t && (i += i.length > 0 ? `${n}..` : "..", s = 2);
        } else
          i.length > 0 ? i += `${n}${e.slice(a + 1, u)}` : i = e.slice(a + 1, u), s = u - a - 1;
      a = u, o = 0;
    } else
      l === tt && o !== -1 ? ++o : o = -1;
  }
  return i;
}
function Ks(e, t) {
  wo(t, "pathObject");
  const n = t.dir || t.root, r = t.base || `${t.name || ""}${t.ext || ""}`;
  return n ? n === t.root ? `${n}${r}` : `${n}${e}${r}` : r;
}
const he = {
  // path.resolve([from ...], to)
  resolve(...e) {
    let t = "", n = "", r = !1;
    for (let i = e.length - 1; i >= -1; i--) {
      let s;
      if (i >= 0) {
        if (s = e[i], K(s, "path"), s.length === 0)
          continue;
      } else
        t.length === 0 ? s = pn() : (s = go[`=${t}`] || pn(), (s === void 0 || s.slice(0, 2).toLowerCase() !== t.toLowerCase() && s.charCodeAt(2) === ge) && (s = `${t}\\`));
      const a = s.length;
      let o = 0, l = "", u = !1;
      const c = s.charCodeAt(0);
      if (a === 1)
        H(c) && (o = 1, u = !0);
      else if (H(c))
        if (u = !0, H(s.charCodeAt(1))) {
          let f = 2, d = f;
          for (; f < a && !H(s.charCodeAt(f)); )
            f++;
          if (f < a && f !== d) {
            const g = s.slice(d, f);
            for (d = f; f < a && H(s.charCodeAt(f)); )
              f++;
            if (f < a && f !== d) {
              for (d = f; f < a && !H(s.charCodeAt(f)); )
                f++;
              (f === a || f !== d) && (l = `\\\\${g}\\${s.slice(d, f)}`, o = f);
            }
          }
        } else
          o = 1;
      else
        ze(c) && s.charCodeAt(1) === He && (l = s.slice(0, 2), o = 2, a > 2 && H(s.charCodeAt(2)) && (u = !0, o = 3));
      if (l.length > 0)
        if (t.length > 0) {
          if (l.toLowerCase() !== t.toLowerCase())
            continue;
        } else
          t = l;
      if (r) {
        if (t.length > 0)
          break;
      } else if (n = `${s.slice(o)}\\${n}`, r = u, u && t.length > 0)
        break;
    }
    return n = vn(n, !r, "\\", H), r ? `${t}\\${n}` : `${t}${n}` || ".";
  },
  normalize(e) {
    K(e, "path");
    const t = e.length;
    if (t === 0)
      return ".";
    let n = 0, r, i = !1;
    const s = e.charCodeAt(0);
    if (t === 1)
      return er(s) ? "\\" : e;
    if (H(s))
      if (i = !0, H(e.charCodeAt(1))) {
        let o = 2, l = o;
        for (; o < t && !H(e.charCodeAt(o)); )
          o++;
        if (o < t && o !== l) {
          const u = e.slice(l, o);
          for (l = o; o < t && H(e.charCodeAt(o)); )
            o++;
          if (o < t && o !== l) {
            for (l = o; o < t && !H(e.charCodeAt(o)); )
              o++;
            if (o === t)
              return `\\\\${u}\\${e.slice(l)}\\`;
            o !== l && (r = `\\\\${u}\\${e.slice(l, o)}`, n = o);
          }
        }
      } else
        n = 1;
    else
      ze(s) && e.charCodeAt(1) === He && (r = e.slice(0, 2), n = 2, t > 2 && H(e.charCodeAt(2)) && (i = !0, n = 3));
    let a = n < t ? vn(e.slice(n), !i, "\\", H) : "";
    return a.length === 0 && !i && (a = "."), a.length > 0 && H(e.charCodeAt(t - 1)) && (a += "\\"), r === void 0 ? i ? `\\${a}` : a : i ? `${r}\\${a}` : `${r}${a}`;
  },
  isAbsolute(e) {
    K(e, "path");
    const t = e.length;
    if (t === 0)
      return !1;
    const n = e.charCodeAt(0);
    return H(n) || // Possible device root
    t > 2 && ze(n) && e.charCodeAt(1) === He && H(e.charCodeAt(2));
  },
  join(...e) {
    if (e.length === 0)
      return ".";
    let t, n;
    for (let s = 0; s < e.length; ++s) {
      const a = e[s];
      K(a, "path"), a.length > 0 && (t === void 0 ? t = n = a : t += `\\${a}`);
    }
    if (t === void 0)
      return ".";
    let r = !0, i = 0;
    if (typeof n == "string" && H(n.charCodeAt(0))) {
      ++i;
      const s = n.length;
      s > 1 && H(n.charCodeAt(1)) && (++i, s > 2 && (H(n.charCodeAt(2)) ? ++i : r = !1));
    }
    if (r) {
      for (; i < t.length && H(t.charCodeAt(i)); )
        i++;
      i >= 2 && (t = `\\${t.slice(i)}`);
    }
    return he.normalize(t);
  },
  // It will solve the relative path from `from` to `to`, for instance:
  //  from = 'C:\\orandea\\test\\aaa'
  //  to = 'C:\\orandea\\impl\\bbb'
  // The output of the function should be: '..\\..\\impl\\bbb'
  relative(e, t) {
    if (K(e, "from"), K(t, "to"), e === t)
      return "";
    const n = he.resolve(e), r = he.resolve(t);
    if (n === r || (e = n.toLowerCase(), t = r.toLowerCase(), e === t))
      return "";
    let i = 0;
    for (; i < e.length && e.charCodeAt(i) === ge; )
      i++;
    let s = e.length;
    for (; s - 1 > i && e.charCodeAt(s - 1) === ge; )
      s--;
    const a = s - i;
    let o = 0;
    for (; o < t.length && t.charCodeAt(o) === ge; )
      o++;
    let l = t.length;
    for (; l - 1 > o && t.charCodeAt(l - 1) === ge; )
      l--;
    const u = l - o, c = a < u ? a : u;
    let f = -1, d = 0;
    for (; d < c; d++) {
      const m = e.charCodeAt(i + d);
      if (m !== t.charCodeAt(o + d))
        break;
      m === ge && (f = d);
    }
    if (d !== c) {
      if (f === -1)
        return r;
    } else {
      if (u > c) {
        if (t.charCodeAt(o + d) === ge)
          return r.slice(o + d + 1);
        if (d === 2)
          return r.slice(o + d);
      }
      a > c && (e.charCodeAt(i + d) === ge ? f = d : d === 2 && (f = 3)), f === -1 && (f = 0);
    }
    let g = "";
    for (d = i + f + 1; d <= s; ++d)
      (d === s || e.charCodeAt(d) === ge) && (g += g.length === 0 ? ".." : "\\..");
    return o += f, g.length > 0 ? `${g}${r.slice(o, l)}` : (r.charCodeAt(o) === ge && ++o, r.slice(o, l));
  },
  toNamespacedPath(e) {
    if (typeof e != "string" || e.length === 0)
      return e;
    const t = he.resolve(e);
    if (t.length <= 2)
      return e;
    if (t.charCodeAt(0) === ge) {
      if (t.charCodeAt(1) === ge) {
        const n = t.charCodeAt(2);
        if (n !== xo && n !== tt)
          return `\\\\?\\UNC\\${t.slice(2)}`;
      }
    } else if (ze(t.charCodeAt(0)) && t.charCodeAt(1) === He && t.charCodeAt(2) === ge)
      return `\\\\?\\${t}`;
    return e;
  },
  dirname(e) {
    K(e, "path");
    const t = e.length;
    if (t === 0)
      return ".";
    let n = -1, r = 0;
    const i = e.charCodeAt(0);
    if (t === 1)
      return H(i) ? e : ".";
    if (H(i)) {
      if (n = r = 1, H(e.charCodeAt(1))) {
        let o = 2, l = o;
        for (; o < t && !H(e.charCodeAt(o)); )
          o++;
        if (o < t && o !== l) {
          for (l = o; o < t && H(e.charCodeAt(o)); )
            o++;
          if (o < t && o !== l) {
            for (l = o; o < t && !H(e.charCodeAt(o)); )
              o++;
            if (o === t)
              return e;
            o !== l && (n = r = o + 1);
          }
        }
      }
    } else
      ze(i) && e.charCodeAt(1) === He && (n = t > 2 && H(e.charCodeAt(2)) ? 3 : 2, r = n);
    let s = -1, a = !0;
    for (let o = t - 1; o >= r; --o)
      if (H(e.charCodeAt(o))) {
        if (!a) {
          s = o;
          break;
        }
      } else
        a = !1;
    if (s === -1) {
      if (n === -1)
        return ".";
      s = n;
    }
    return e.slice(0, s);
  },
  basename(e, t) {
    t !== void 0 && K(t, "ext"), K(e, "path");
    let n = 0, r = -1, i = !0, s;
    if (e.length >= 2 && ze(e.charCodeAt(0)) && e.charCodeAt(1) === He && (n = 2), t !== void 0 && t.length > 0 && t.length <= e.length) {
      if (t === e)
        return "";
      let a = t.length - 1, o = -1;
      for (s = e.length - 1; s >= n; --s) {
        const l = e.charCodeAt(s);
        if (H(l)) {
          if (!i) {
            n = s + 1;
            break;
          }
        } else
          o === -1 && (i = !1, o = s + 1), a >= 0 && (l === t.charCodeAt(a) ? --a === -1 && (r = s) : (a = -1, r = o));
      }
      return n === r ? r = o : r === -1 && (r = e.length), e.slice(n, r);
    }
    for (s = e.length - 1; s >= n; --s)
      if (H(e.charCodeAt(s))) {
        if (!i) {
          n = s + 1;
          break;
        }
      } else
        r === -1 && (i = !1, r = s + 1);
    return r === -1 ? "" : e.slice(n, r);
  },
  extname(e) {
    K(e, "path");
    let t = 0, n = -1, r = 0, i = -1, s = !0, a = 0;
    e.length >= 2 && e.charCodeAt(1) === He && ze(e.charCodeAt(0)) && (t = r = 2);
    for (let o = e.length - 1; o >= t; --o) {
      const l = e.charCodeAt(o);
      if (H(l)) {
        if (!s) {
          r = o + 1;
          break;
        }
        continue;
      }
      i === -1 && (s = !1, i = o + 1), l === tt ? n === -1 ? n = o : a !== 1 && (a = 1) : n !== -1 && (a = -1);
    }
    return n === -1 || i === -1 || // We saw a non-dot character immediately before the dot
    a === 0 || // The (right-most) trimmed path component is exactly '..'
    a === 1 && n === i - 1 && n === r + 1 ? "" : e.slice(n, i);
  },
  format: Ks.bind(null, "\\"),
  parse(e) {
    K(e, "path");
    const t = { root: "", dir: "", base: "", ext: "", name: "" };
    if (e.length === 0)
      return t;
    const n = e.length;
    let r = 0, i = e.charCodeAt(0);
    if (n === 1)
      return H(i) ? (t.root = t.dir = e, t) : (t.base = t.name = e, t);
    if (H(i)) {
      if (r = 1, H(e.charCodeAt(1))) {
        let f = 2, d = f;
        for (; f < n && !H(e.charCodeAt(f)); )
          f++;
        if (f < n && f !== d) {
          for (d = f; f < n && H(e.charCodeAt(f)); )
            f++;
          if (f < n && f !== d) {
            for (d = f; f < n && !H(e.charCodeAt(f)); )
              f++;
            f === n ? r = f : f !== d && (r = f + 1);
          }
        }
      }
    } else if (ze(i) && e.charCodeAt(1) === He) {
      if (n <= 2)
        return t.root = t.dir = e, t;
      if (r = 2, H(e.charCodeAt(2))) {
        if (n === 3)
          return t.root = t.dir = e, t;
        r = 3;
      }
    }
    r > 0 && (t.root = e.slice(0, r));
    let s = -1, a = r, o = -1, l = !0, u = e.length - 1, c = 0;
    for (; u >= r; --u) {
      if (i = e.charCodeAt(u), H(i)) {
        if (!l) {
          a = u + 1;
          break;
        }
        continue;
      }
      o === -1 && (l = !1, o = u + 1), i === tt ? s === -1 ? s = u : c !== 1 && (c = 1) : s !== -1 && (c = -1);
    }
    return o !== -1 && (s === -1 || // We saw a non-dot character immediately before the dot
    c === 0 || // The (right-most) trimmed path component is exactly '..'
    c === 1 && s === o - 1 && s === a + 1 ? t.base = t.name = e.slice(a, o) : (t.name = e.slice(a, s), t.base = e.slice(a, o), t.ext = e.slice(s, o))), a > 0 && a !== r ? t.dir = e.slice(0, a - 1) : t.dir = t.root, t;
  },
  sep: "\\",
  delimiter: ";",
  win32: null,
  posix: null
}, _o = (() => {
  if (it) {
    const e = /\\/g;
    return () => {
      const t = pn().replace(e, "/");
      return t.slice(t.indexOf("/"));
    };
  }
  return () => pn();
})(), ve = {
  // path.resolve([from ...], to)
  resolve(...e) {
    let t = "", n = !1;
    for (let r = e.length - 1; r >= -1 && !n; r--) {
      const i = r >= 0 ? e[r] : _o();
      K(i, "path"), i.length !== 0 && (t = `${i}/${t}`, n = i.charCodeAt(0) === le);
    }
    return t = vn(t, !n, "/", er), n ? `/${t}` : t.length > 0 ? t : ".";
  },
  normalize(e) {
    if (K(e, "path"), e.length === 0)
      return ".";
    const t = e.charCodeAt(0) === le, n = e.charCodeAt(e.length - 1) === le;
    return e = vn(e, !t, "/", er), e.length === 0 ? t ? "/" : n ? "./" : "." : (n && (e += "/"), t ? `/${e}` : e);
  },
  isAbsolute(e) {
    return K(e, "path"), e.length > 0 && e.charCodeAt(0) === le;
  },
  join(...e) {
    if (e.length === 0)
      return ".";
    let t;
    for (let n = 0; n < e.length; ++n) {
      const r = e[n];
      K(r, "path"), r.length > 0 && (t === void 0 ? t = r : t += `/${r}`);
    }
    return t === void 0 ? "." : ve.normalize(t);
  },
  relative(e, t) {
    if (K(e, "from"), K(t, "to"), e === t || (e = ve.resolve(e), t = ve.resolve(t), e === t))
      return "";
    const n = 1, r = e.length, i = r - n, s = 1, a = t.length - s, o = i < a ? i : a;
    let l = -1, u = 0;
    for (; u < o; u++) {
      const f = e.charCodeAt(n + u);
      if (f !== t.charCodeAt(s + u))
        break;
      f === le && (l = u);
    }
    if (u === o)
      if (a > o) {
        if (t.charCodeAt(s + u) === le)
          return t.slice(s + u + 1);
        if (u === 0)
          return t.slice(s + u);
      } else
        i > o && (e.charCodeAt(n + u) === le ? l = u : u === 0 && (l = 0));
    let c = "";
    for (u = n + l + 1; u <= r; ++u)
      (u === r || e.charCodeAt(u) === le) && (c += c.length === 0 ? ".." : "/..");
    return `${c}${t.slice(s + l)}`;
  },
  toNamespacedPath(e) {
    return e;
  },
  dirname(e) {
    if (K(e, "path"), e.length === 0)
      return ".";
    const t = e.charCodeAt(0) === le;
    let n = -1, r = !0;
    for (let i = e.length - 1; i >= 1; --i)
      if (e.charCodeAt(i) === le) {
        if (!r) {
          n = i;
          break;
        }
      } else
        r = !1;
    return n === -1 ? t ? "/" : "." : t && n === 1 ? "//" : e.slice(0, n);
  },
  basename(e, t) {
    t !== void 0 && K(t, "ext"), K(e, "path");
    let n = 0, r = -1, i = !0, s;
    if (t !== void 0 && t.length > 0 && t.length <= e.length) {
      if (t === e)
        return "";
      let a = t.length - 1, o = -1;
      for (s = e.length - 1; s >= 0; --s) {
        const l = e.charCodeAt(s);
        if (l === le) {
          if (!i) {
            n = s + 1;
            break;
          }
        } else
          o === -1 && (i = !1, o = s + 1), a >= 0 && (l === t.charCodeAt(a) ? --a === -1 && (r = s) : (a = -1, r = o));
      }
      return n === r ? r = o : r === -1 && (r = e.length), e.slice(n, r);
    }
    for (s = e.length - 1; s >= 0; --s)
      if (e.charCodeAt(s) === le) {
        if (!i) {
          n = s + 1;
          break;
        }
      } else
        r === -1 && (i = !1, r = s + 1);
    return r === -1 ? "" : e.slice(n, r);
  },
  extname(e) {
    K(e, "path");
    let t = -1, n = 0, r = -1, i = !0, s = 0;
    for (let a = e.length - 1; a >= 0; --a) {
      const o = e.charCodeAt(a);
      if (o === le) {
        if (!i) {
          n = a + 1;
          break;
        }
        continue;
      }
      r === -1 && (i = !1, r = a + 1), o === tt ? t === -1 ? t = a : s !== 1 && (s = 1) : t !== -1 && (s = -1);
    }
    return t === -1 || r === -1 || // We saw a non-dot character immediately before the dot
    s === 0 || // The (right-most) trimmed path component is exactly '..'
    s === 1 && t === r - 1 && t === n + 1 ? "" : e.slice(t, r);
  },
  format: Ks.bind(null, "/"),
  parse(e) {
    K(e, "path");
    const t = { root: "", dir: "", base: "", ext: "", name: "" };
    if (e.length === 0)
      return t;
    const n = e.charCodeAt(0) === le;
    let r;
    n ? (t.root = "/", r = 1) : r = 0;
    let i = -1, s = 0, a = -1, o = !0, l = e.length - 1, u = 0;
    for (; l >= r; --l) {
      const c = e.charCodeAt(l);
      if (c === le) {
        if (!o) {
          s = l + 1;
          break;
        }
        continue;
      }
      a === -1 && (o = !1, a = l + 1), c === tt ? i === -1 ? i = l : u !== 1 && (u = 1) : i !== -1 && (u = -1);
    }
    if (a !== -1) {
      const c = s === 0 && n ? 1 : s;
      i === -1 || // We saw a non-dot character immediately before the dot
      u === 0 || // The (right-most) trimmed path component is exactly '..'
      u === 1 && i === a - 1 && i === s + 1 ? t.base = t.name = e.slice(c, a) : (t.name = e.slice(c, i), t.base = e.slice(c, a), t.ext = e.slice(i, a));
    }
    return s > 0 ? t.dir = e.slice(0, s - 1) : n && (t.dir = "/"), t;
  },
  sep: "/",
  delimiter: ":",
  win32: null,
  posix: null
};
ve.win32 = he.win32 = he;
ve.posix = he.posix = ve;
it ? he.normalize : ve.normalize;
it ? he.resolve : ve.resolve;
it ? he.relative : ve.relative;
it ? he.dirname : ve.dirname;
it ? he.basename : ve.basename;
it ? he.extname : ve.extname;
it ? he.sep : ve.sep;
const So = /^\w[\w\d+.-]*$/, Lo = /^\//, No = /^\/\//;
function Ao(e, t) {
  if (!e.scheme && t)
    throw new Error(`[UriError]: Scheme is missing: {scheme: "", authority: "${e.authority}", path: "${e.path}", query: "${e.query}", fragment: "${e.fragment}"}`);
  if (e.scheme && !So.test(e.scheme))
    throw new Error("[UriError]: Scheme contains illegal characters.");
  if (e.path) {
    if (e.authority) {
      if (!Lo.test(e.path))
        throw new Error('[UriError]: If a URI contains an authority component, then the path component must either be empty or begin with a slash ("/") character');
    } else if (No.test(e.path))
      throw new Error('[UriError]: If a URI does not contain an authority component, then the path cannot begin with two slash characters ("//")');
  }
}
function Co(e, t) {
  return !e && !t ? "file" : e;
}
function ko(e, t) {
  switch (e) {
    case "https":
    case "http":
    case "file":
      t ? t[0] !== Ce && (t = Ce + t) : t = Ce;
      break;
  }
  return t;
}
const Z = "", Ce = "/", Eo = /^(([^:/?#]+?):)?(\/\/([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?/;
let Mr = class un {
  static isUri(t) {
    return t instanceof un ? !0 : t ? typeof t.authority == "string" && typeof t.fragment == "string" && typeof t.path == "string" && typeof t.query == "string" && typeof t.scheme == "string" && typeof t.fsPath == "string" && typeof t.with == "function" && typeof t.toString == "function" : !1;
  }
  /**
   * @internal
   */
  constructor(t, n, r, i, s, a = !1) {
    typeof t == "object" ? (this.scheme = t.scheme || Z, this.authority = t.authority || Z, this.path = t.path || Z, this.query = t.query || Z, this.fragment = t.fragment || Z) : (this.scheme = Co(t, a), this.authority = n || Z, this.path = ko(this.scheme, r || Z), this.query = i || Z, this.fragment = s || Z, Ao(this, a));
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
    return tr(this, !1);
  }
  // ---- modify to new -------------------------
  with(t) {
    if (!t)
      return this;
    let { scheme: n, authority: r, path: i, query: s, fragment: a } = t;
    return n === void 0 ? n = this.scheme : n === null && (n = Z), r === void 0 ? r = this.authority : r === null && (r = Z), i === void 0 ? i = this.path : i === null && (i = Z), s === void 0 ? s = this.query : s === null && (s = Z), a === void 0 ? a = this.fragment : a === null && (a = Z), n === this.scheme && r === this.authority && i === this.path && s === this.query && a === this.fragment ? this : new dt(n, r, i, s, a);
  }
  // ---- parse & validate ------------------------
  /**
   * Creates a new URI from a string, e.g. `http://www.example.com/some/path`,
   * `file:///usr/home`, or `scheme:with/path`.
   *
   * @param value A string which represents an URI (see `URI#toString`).
   */
  static parse(t, n = !1) {
    const r = Eo.exec(t);
    return r ? new dt(r[2] || Z, tn(r[4] || Z), tn(r[5] || Z), tn(r[7] || Z), tn(r[9] || Z), n) : new dt(Z, Z, Z, Z, Z);
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
    let n = Z;
    if (qt && (t = t.replace(/\\/g, Ce)), t[0] === Ce && t[1] === Ce) {
      const r = t.indexOf(Ce, 2);
      r === -1 ? (n = t.substring(2), t = Ce) : (n = t.substring(2, r), t = t.substring(r) || Ce);
    }
    return new dt("file", n, t, Z, Z);
  }
  /**
   * Creates new URI from uri components.
   *
   * Unless `strict` is `true` the scheme is defaults to be `file`. This function performs
   * validation and should be used for untrusted uri components retrieved from storage,
   * user input, command arguments etc
   */
  static from(t, n) {
    return new dt(t.scheme, t.authority, t.path, t.query, t.fragment, n);
  }
  /**
   * Join a URI path with path fragments and normalizes the resulting path.
   *
   * @param uri The input URI.
   * @param pathFragment The path fragment to add to the URI path.
   * @returns The resulting URI.
   */
  static joinPath(t, ...n) {
    if (!t.path)
      throw new Error("[UriError]: cannot call joinPath on URI without path");
    let r;
    return qt && t.scheme === "file" ? r = un.file(he.join(tr(t, !0), ...n)).path : r = ve.join(t.path, ...n), t.with({ path: r });
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
    return nr(this, t);
  }
  toJSON() {
    return this;
  }
  static revive(t) {
    var n, r;
    if (t) {
      if (t instanceof un)
        return t;
      {
        const i = new dt(t);
        return i._formatted = (n = t.external) !== null && n !== void 0 ? n : null, i._fsPath = t._sep === ea && (r = t.fsPath) !== null && r !== void 0 ? r : null, i;
      }
    } else
      return t;
  }
};
const ea = qt ? 1 : void 0;
class dt extends Mr {
  constructor() {
    super(...arguments), this._formatted = null, this._fsPath = null;
  }
  get fsPath() {
    return this._fsPath || (this._fsPath = tr(this, !1)), this._fsPath;
  }
  toString(t = !1) {
    return t ? nr(this, !0) : (this._formatted || (this._formatted = nr(this, !1)), this._formatted);
  }
  toJSON() {
    const t = {
      $mid: 1
      /* MarshalledId.Uri */
    };
    return this._fsPath && (t.fsPath = this._fsPath, t._sep = ea), this._formatted && (t.external = this._formatted), this.path && (t.path = this.path), this.scheme && (t.scheme = this.scheme), this.authority && (t.authority = this.authority), this.query && (t.query = this.query), this.fragment && (t.fragment = this.fragment), t;
  }
}
const ta = {
  58: "%3A",
  // gen-delims
  47: "%2F",
  63: "%3F",
  35: "%23",
  91: "%5B",
  93: "%5D",
  64: "%40",
  33: "%21",
  // sub-delims
  36: "%24",
  38: "%26",
  39: "%27",
  40: "%28",
  41: "%29",
  42: "%2A",
  43: "%2B",
  44: "%2C",
  59: "%3B",
  61: "%3D",
  32: "%20"
};
function Gr(e, t, n) {
  let r, i = -1;
  for (let s = 0; s < e.length; s++) {
    const a = e.charCodeAt(s);
    if (a >= 97 && a <= 122 || a >= 65 && a <= 90 || a >= 48 && a <= 57 || a === 45 || a === 46 || a === 95 || a === 126 || t && a === 47 || n && a === 91 || n && a === 93 || n && a === 58)
      i !== -1 && (r += encodeURIComponent(e.substring(i, s)), i = -1), r !== void 0 && (r += e.charAt(s));
    else {
      r === void 0 && (r = e.substr(0, s));
      const o = ta[a];
      o !== void 0 ? (i !== -1 && (r += encodeURIComponent(e.substring(i, s)), i = -1), r += o) : i === -1 && (i = s);
    }
  }
  return i !== -1 && (r += encodeURIComponent(e.substring(i))), r !== void 0 ? r : e;
}
function Mo(e) {
  let t;
  for (let n = 0; n < e.length; n++) {
    const r = e.charCodeAt(n);
    r === 35 || r === 63 ? (t === void 0 && (t = e.substr(0, n)), t += ta[r]) : t !== void 0 && (t += e[n]);
  }
  return t !== void 0 ? t : e;
}
function tr(e, t) {
  let n;
  return e.authority && e.path.length > 1 && e.scheme === "file" ? n = `//${e.authority}${e.path}` : e.path.charCodeAt(0) === 47 && (e.path.charCodeAt(1) >= 65 && e.path.charCodeAt(1) <= 90 || e.path.charCodeAt(1) >= 97 && e.path.charCodeAt(1) <= 122) && e.path.charCodeAt(2) === 58 ? t ? n = e.path.substr(1) : n = e.path[1].toLowerCase() + e.path.substr(2) : n = e.path, qt && (n = n.replace(/\//g, "\\")), n;
}
function nr(e, t) {
  const n = t ? Mo : Gr;
  let r = "", { scheme: i, authority: s, path: a, query: o, fragment: l } = e;
  if (i && (r += i, r += ":"), (s || i === "file") && (r += Ce, r += Ce), s) {
    let u = s.indexOf("@");
    if (u !== -1) {
      const c = s.substr(0, u);
      s = s.substr(u + 1), u = c.lastIndexOf(":"), u === -1 ? r += n(c, !1, !1) : (r += n(c.substr(0, u), !1, !1), r += ":", r += n(c.substr(u + 1), !1, !0)), r += "@";
    }
    s = s.toLowerCase(), u = s.lastIndexOf(":"), u === -1 ? r += n(s, !1, !0) : (r += n(s.substr(0, u), !1, !0), r += s.substr(u));
  }
  if (a) {
    if (a.length >= 3 && a.charCodeAt(0) === 47 && a.charCodeAt(2) === 58) {
      const u = a.charCodeAt(1);
      u >= 65 && u <= 90 && (a = `/${String.fromCharCode(u + 32)}:${a.substr(3)}`);
    } else if (a.length >= 2 && a.charCodeAt(1) === 58) {
      const u = a.charCodeAt(0);
      u >= 65 && u <= 90 && (a = `${String.fromCharCode(u + 32)}:${a.substr(2)}`);
    }
    r += n(a, !0, !1);
  }
  return o && (r += "?", r += n(o, !1, !1)), l && (r += "#", r += t ? l : Gr(l, !1, !1)), r;
}
function na(e) {
  try {
    return decodeURIComponent(e);
  } catch {
    return e.length > 3 ? e.substr(0, 3) + na(e.substr(3)) : e;
  }
}
const Jr = /(%[0-9A-Za-z][0-9A-Za-z])+/g;
function tn(e) {
  return e.match(Jr) ? e.replace(Jr, (t) => na(t)) : e;
}
let Oe = class st {
  constructor(t, n) {
    this.lineNumber = t, this.column = n;
  }
  /**
   * Create a new position from this position.
   *
   * @param newLineNumber new line number
   * @param newColumn new column
   */
  with(t = this.lineNumber, n = this.column) {
    return t === this.lineNumber && n === this.column ? this : new st(t, n);
  }
  /**
   * Derive a new position from this position.
   *
   * @param deltaLineNumber line number delta
   * @param deltaColumn column delta
   */
  delta(t = 0, n = 0) {
    return this.with(this.lineNumber + t, this.column + n);
  }
  /**
   * Test if this position equals other position
   */
  equals(t) {
    return st.equals(this, t);
  }
  /**
   * Test if position `a` equals position `b`
   */
  static equals(t, n) {
    return !t && !n ? !0 : !!t && !!n && t.lineNumber === n.lineNumber && t.column === n.column;
  }
  /**
   * Test if this position is before other position.
   * If the two positions are equal, the result will be false.
   */
  isBefore(t) {
    return st.isBefore(this, t);
  }
  /**
   * Test if position `a` is before position `b`.
   * If the two positions are equal, the result will be false.
   */
  static isBefore(t, n) {
    return t.lineNumber < n.lineNumber ? !0 : n.lineNumber < t.lineNumber ? !1 : t.column < n.column;
  }
  /**
   * Test if this position is before other position.
   * If the two positions are equal, the result will be true.
   */
  isBeforeOrEqual(t) {
    return st.isBeforeOrEqual(this, t);
  }
  /**
   * Test if position `a` is before position `b`.
   * If the two positions are equal, the result will be true.
   */
  static isBeforeOrEqual(t, n) {
    return t.lineNumber < n.lineNumber ? !0 : n.lineNumber < t.lineNumber ? !1 : t.column <= n.column;
  }
  /**
   * A function that compares positions, useful for sorting
   */
  static compare(t, n) {
    const r = t.lineNumber | 0, i = n.lineNumber | 0;
    if (r === i) {
      const s = t.column | 0, a = n.column | 0;
      return s - a;
    }
    return r - i;
  }
  /**
   * Clone this position.
   */
  clone() {
    return new st(this.lineNumber, this.column);
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
    return new st(t.lineNumber, t.column);
  }
  /**
   * Test if `obj` is an `IPosition`.
   */
  static isIPosition(t) {
    return t && typeof t.lineNumber == "number" && typeof t.column == "number";
  }
}, ae = class ne {
  constructor(t, n, r, i) {
    t > r || t === r && n > i ? (this.startLineNumber = r, this.startColumn = i, this.endLineNumber = t, this.endColumn = n) : (this.startLineNumber = t, this.startColumn = n, this.endLineNumber = r, this.endColumn = i);
  }
  /**
   * Test if this range is empty.
   */
  isEmpty() {
    return ne.isEmpty(this);
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
    return ne.containsPosition(this, t);
  }
  /**
   * Test if `position` is in `range`. If the position is at the edges, will return true.
   */
  static containsPosition(t, n) {
    return !(n.lineNumber < t.startLineNumber || n.lineNumber > t.endLineNumber || n.lineNumber === t.startLineNumber && n.column < t.startColumn || n.lineNumber === t.endLineNumber && n.column > t.endColumn);
  }
  /**
   * Test if `position` is in `range`. If the position is at the edges, will return false.
   * @internal
   */
  static strictContainsPosition(t, n) {
    return !(n.lineNumber < t.startLineNumber || n.lineNumber > t.endLineNumber || n.lineNumber === t.startLineNumber && n.column <= t.startColumn || n.lineNumber === t.endLineNumber && n.column >= t.endColumn);
  }
  /**
   * Test if range is in this range. If the range is equal to this range, will return true.
   */
  containsRange(t) {
    return ne.containsRange(this, t);
  }
  /**
   * Test if `otherRange` is in `range`. If the ranges are equal, will return true.
   */
  static containsRange(t, n) {
    return !(n.startLineNumber < t.startLineNumber || n.endLineNumber < t.startLineNumber || n.startLineNumber > t.endLineNumber || n.endLineNumber > t.endLineNumber || n.startLineNumber === t.startLineNumber && n.startColumn < t.startColumn || n.endLineNumber === t.endLineNumber && n.endColumn > t.endColumn);
  }
  /**
   * Test if `range` is strictly in this range. `range` must start after and end before this range for the result to be true.
   */
  strictContainsRange(t) {
    return ne.strictContainsRange(this, t);
  }
  /**
   * Test if `otherRange` is strictly in `range` (must start after, and end before). If the ranges are equal, will return false.
   */
  static strictContainsRange(t, n) {
    return !(n.startLineNumber < t.startLineNumber || n.endLineNumber < t.startLineNumber || n.startLineNumber > t.endLineNumber || n.endLineNumber > t.endLineNumber || n.startLineNumber === t.startLineNumber && n.startColumn <= t.startColumn || n.endLineNumber === t.endLineNumber && n.endColumn >= t.endColumn);
  }
  /**
   * A reunion of the two ranges.
   * The smallest position will be used as the start point, and the largest one as the end point.
   */
  plusRange(t) {
    return ne.plusRange(this, t);
  }
  /**
   * A reunion of the two ranges.
   * The smallest position will be used as the start point, and the largest one as the end point.
   */
  static plusRange(t, n) {
    let r, i, s, a;
    return n.startLineNumber < t.startLineNumber ? (r = n.startLineNumber, i = n.startColumn) : n.startLineNumber === t.startLineNumber ? (r = n.startLineNumber, i = Math.min(n.startColumn, t.startColumn)) : (r = t.startLineNumber, i = t.startColumn), n.endLineNumber > t.endLineNumber ? (s = n.endLineNumber, a = n.endColumn) : n.endLineNumber === t.endLineNumber ? (s = n.endLineNumber, a = Math.max(n.endColumn, t.endColumn)) : (s = t.endLineNumber, a = t.endColumn), new ne(r, i, s, a);
  }
  /**
   * A intersection of the two ranges.
   */
  intersectRanges(t) {
    return ne.intersectRanges(this, t);
  }
  /**
   * A intersection of the two ranges.
   */
  static intersectRanges(t, n) {
    let r = t.startLineNumber, i = t.startColumn, s = t.endLineNumber, a = t.endColumn;
    const o = n.startLineNumber, l = n.startColumn, u = n.endLineNumber, c = n.endColumn;
    return r < o ? (r = o, i = l) : r === o && (i = Math.max(i, l)), s > u ? (s = u, a = c) : s === u && (a = Math.min(a, c)), r > s || r === s && i > a ? null : new ne(r, i, s, a);
  }
  /**
   * Test if this range equals other.
   */
  equalsRange(t) {
    return ne.equalsRange(this, t);
  }
  /**
   * Test if range `a` equals `b`.
   */
  static equalsRange(t, n) {
    return !t && !n ? !0 : !!t && !!n && t.startLineNumber === n.startLineNumber && t.startColumn === n.startColumn && t.endLineNumber === n.endLineNumber && t.endColumn === n.endColumn;
  }
  /**
   * Return the end position (which will be after or equal to the start position)
   */
  getEndPosition() {
    return ne.getEndPosition(this);
  }
  /**
   * Return the end position (which will be after or equal to the start position)
   */
  static getEndPosition(t) {
    return new Oe(t.endLineNumber, t.endColumn);
  }
  /**
   * Return the start position (which will be before or equal to the end position)
   */
  getStartPosition() {
    return ne.getStartPosition(this);
  }
  /**
   * Return the start position (which will be before or equal to the end position)
   */
  static getStartPosition(t) {
    return new Oe(t.startLineNumber, t.startColumn);
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
  setEndPosition(t, n) {
    return new ne(this.startLineNumber, this.startColumn, t, n);
  }
  /**
   * Create a new range using this range's end position, and using startLineNumber and startColumn as the start position.
   */
  setStartPosition(t, n) {
    return new ne(t, n, this.endLineNumber, this.endColumn);
  }
  /**
   * Create a new empty range using this range's start position.
   */
  collapseToStart() {
    return ne.collapseToStart(this);
  }
  /**
   * Create a new empty range using this range's start position.
   */
  static collapseToStart(t) {
    return new ne(t.startLineNumber, t.startColumn, t.startLineNumber, t.startColumn);
  }
  /**
   * Create a new empty range using this range's end position.
   */
  collapseToEnd() {
    return ne.collapseToEnd(this);
  }
  /**
   * Create a new empty range using this range's end position.
   */
  static collapseToEnd(t) {
    return new ne(t.endLineNumber, t.endColumn, t.endLineNumber, t.endColumn);
  }
  /**
   * Moves the range by the given amount of lines.
   */
  delta(t) {
    return new ne(this.startLineNumber + t, this.startColumn, this.endLineNumber + t, this.endColumn);
  }
  // ---
  static fromPositions(t, n = t) {
    return new ne(t.lineNumber, t.column, n.lineNumber, n.column);
  }
  static lift(t) {
    return t ? new ne(t.startLineNumber, t.startColumn, t.endLineNumber, t.endColumn) : null;
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
  static areIntersectingOrTouching(t, n) {
    return !(t.endLineNumber < n.startLineNumber || t.endLineNumber === n.startLineNumber && t.endColumn < n.startColumn || n.endLineNumber < t.startLineNumber || n.endLineNumber === t.startLineNumber && n.endColumn < t.startColumn);
  }
  /**
   * Test if the two ranges are intersecting. If the ranges are touching it returns true.
   */
  static areIntersecting(t, n) {
    return !(t.endLineNumber < n.startLineNumber || t.endLineNumber === n.startLineNumber && t.endColumn <= n.startColumn || n.endLineNumber < t.startLineNumber || n.endLineNumber === t.startLineNumber && n.endColumn <= t.startColumn);
  }
  /**
   * A function that compares ranges, useful for sorting ranges
   * It will first compare ranges on the startPosition and then on the endPosition
   */
  static compareRangesUsingStarts(t, n) {
    if (t && n) {
      const s = t.startLineNumber | 0, a = n.startLineNumber | 0;
      if (s === a) {
        const o = t.startColumn | 0, l = n.startColumn | 0;
        if (o === l) {
          const u = t.endLineNumber | 0, c = n.endLineNumber | 0;
          if (u === c) {
            const f = t.endColumn | 0, d = n.endColumn | 0;
            return f - d;
          }
          return u - c;
        }
        return o - l;
      }
      return s - a;
    }
    return (t ? 1 : 0) - (n ? 1 : 0);
  }
  /**
   * A function that compares ranges, useful for sorting ranges
   * It will first compare ranges on the endPosition and then on the startPosition
   */
  static compareRangesUsingEnds(t, n) {
    return t.endLineNumber === n.endLineNumber ? t.endColumn === n.endColumn ? t.startLineNumber === n.startLineNumber ? t.startColumn - n.startColumn : t.startLineNumber - n.startLineNumber : t.endColumn - n.endColumn : t.endLineNumber - n.endLineNumber;
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
function Ro(e, t, n = (r, i) => r === i) {
  if (e === t)
    return !0;
  if (!e || !t || e.length !== t.length)
    return !1;
  for (let r = 0, i = e.length; r < i; r++)
    if (!n(e[r], t[r]))
      return !1;
  return !0;
}
function* To(e, t) {
  let n, r;
  for (const i of e)
    r !== void 0 && t(r, i) ? n.push(i) : (n && (yield n), n = [i]), r = i;
  n && (yield n);
}
function Po(e, t) {
  for (let n = 0; n <= e.length; n++)
    t(n === 0 ? void 0 : e[n - 1], n === e.length ? void 0 : e[n]);
}
function Fo(e, t) {
  for (let n = 0; n < e.length; n++)
    t(n === 0 ? void 0 : e[n - 1], e[n], n + 1 === e.length ? void 0 : e[n + 1]);
}
function Io(e, t) {
  for (const n of t)
    e.push(n);
}
var Xr;
(function(e) {
  function t(s) {
    return s < 0;
  }
  e.isLessThan = t;
  function n(s) {
    return s <= 0;
  }
  e.isLessThanOrEqual = n;
  function r(s) {
    return s > 0;
  }
  e.isGreaterThan = r;
  function i(s) {
    return s === 0;
  }
  e.isNeitherLessOrGreaterThan = i, e.greaterThan = 1, e.lessThan = -1, e.neitherLessOrGreaterThan = 0;
})(Xr || (Xr = {}));
function cn(e, t) {
  return (n, r) => t(e(n), e(r));
}
const fn = (e, t) => e - t;
function Do(e) {
  return (t, n) => -e(t, n);
}
function Qr(e) {
  return e < 0 ? 0 : e > 255 ? 255 : e | 0;
}
function gt(e) {
  return e < 0 ? 0 : e > 4294967295 ? 4294967295 : e | 0;
}
class Vo {
  constructor(t) {
    this.values = t, this.prefixSum = new Uint32Array(t.length), this.prefixSumValidIndex = new Int32Array(1), this.prefixSumValidIndex[0] = -1;
  }
  insertValues(t, n) {
    t = gt(t);
    const r = this.values, i = this.prefixSum, s = n.length;
    return s === 0 ? !1 : (this.values = new Uint32Array(r.length + s), this.values.set(r.subarray(0, t), 0), this.values.set(r.subarray(t), t + s), this.values.set(n, t), t - 1 < this.prefixSumValidIndex[0] && (this.prefixSumValidIndex[0] = t - 1), this.prefixSum = new Uint32Array(this.values.length), this.prefixSumValidIndex[0] >= 0 && this.prefixSum.set(i.subarray(0, this.prefixSumValidIndex[0] + 1)), !0);
  }
  setValue(t, n) {
    return t = gt(t), n = gt(n), this.values[t] === n ? !1 : (this.values[t] = n, t - 1 < this.prefixSumValidIndex[0] && (this.prefixSumValidIndex[0] = t - 1), !0);
  }
  removeValues(t, n) {
    t = gt(t), n = gt(n);
    const r = this.values, i = this.prefixSum;
    if (t >= r.length)
      return !1;
    const s = r.length - t;
    return n >= s && (n = s), n === 0 ? !1 : (this.values = new Uint32Array(r.length - n), this.values.set(r.subarray(0, t), 0), this.values.set(r.subarray(t + n), t), this.prefixSum = new Uint32Array(this.values.length), t - 1 < this.prefixSumValidIndex[0] && (this.prefixSumValidIndex[0] = t - 1), this.prefixSumValidIndex[0] >= 0 && this.prefixSum.set(i.subarray(0, this.prefixSumValidIndex[0] + 1)), !0);
  }
  getTotalSum() {
    return this.values.length === 0 ? 0 : this._getPrefixSum(this.values.length - 1);
  }
  /**
   * Returns the sum of the first `index + 1` many items.
   * @returns `SUM(0 <= j <= index, values[j])`.
   */
  getPrefixSum(t) {
    return t < 0 ? 0 : (t = gt(t), this._getPrefixSum(t));
  }
  _getPrefixSum(t) {
    if (t <= this.prefixSumValidIndex[0])
      return this.prefixSum[t];
    let n = this.prefixSumValidIndex[0] + 1;
    n === 0 && (this.prefixSum[0] = this.values[0], n++), t >= this.values.length && (t = this.values.length - 1);
    for (let r = n; r <= t; r++)
      this.prefixSum[r] = this.prefixSum[r - 1] + this.values[r];
    return this.prefixSumValidIndex[0] = Math.max(this.prefixSumValidIndex[0], t), this.prefixSum[t];
  }
  getIndexOf(t) {
    t = Math.floor(t), this.getTotalSum();
    let n = 0, r = this.values.length - 1, i = 0, s = 0, a = 0;
    for (; n <= r; )
      if (i = n + (r - n) / 2 | 0, s = this.prefixSum[i], a = s - this.values[i], t < a)
        r = i - 1;
      else if (t >= s)
        n = i + 1;
      else
        break;
    return new Oo(i, t - a);
  }
}
class Oo {
  constructor(t, n) {
    this.index = t, this.remainder = n, this._prefixSumIndexOfResultBrand = void 0, this.index = t, this.remainder = n;
  }
}
class jo {
  constructor(t, n, r, i) {
    this._uri = t, this._lines = n, this._eol = r, this._versionId = i, this._lineStarts = null, this._cachedTextValue = null;
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
    const n = t.changes;
    for (const r of n)
      this._acceptDeleteRange(r.range), this._acceptInsertText(new Oe(r.range.startLineNumber, r.range.startColumn), r.text);
    this._versionId = t.versionId, this._cachedTextValue = null;
  }
  _ensureLineStarts() {
    if (!this._lineStarts) {
      const t = this._eol.length, n = this._lines.length, r = new Uint32Array(n);
      for (let i = 0; i < n; i++)
        r[i] = this._lines[i].length + t;
      this._lineStarts = new Vo(r);
    }
  }
  /**
   * All changes to a line's text go through this method
   */
  _setLineText(t, n) {
    this._lines[t] = n, this._lineStarts && this._lineStarts.setValue(t, this._lines[t].length + this._eol.length);
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
  _acceptInsertText(t, n) {
    if (n.length === 0)
      return;
    const r = Xa(n);
    if (r.length === 1) {
      this._setLineText(t.lineNumber - 1, this._lines[t.lineNumber - 1].substring(0, t.column - 1) + r[0] + this._lines[t.lineNumber - 1].substring(t.column - 1));
      return;
    }
    r[r.length - 1] += this._lines[t.lineNumber - 1].substring(t.column - 1), this._setLineText(t.lineNumber - 1, this._lines[t.lineNumber - 1].substring(0, t.column - 1) + r[0]);
    const i = new Uint32Array(r.length - 1);
    for (let s = 1; s < r.length; s++)
      this._lines.splice(t.lineNumber + s - 1, 0, r[s]), i[s - 1] = r[s].length + this._eol.length;
    this._lineStarts && this._lineStarts.insertValues(t.lineNumber, i);
  }
}
const qo = "`~!@#$%^&*()-=+[{]}\\|;:'\",.<>/?";
function Bo(e = "") {
  let t = "(-?\\d*\\.\\d\\w*)|([^";
  for (const n of qo)
    e.indexOf(n) >= 0 || (t += "\\" + n);
  return t += "\\s]+)", new RegExp(t, "g");
}
const ra = Bo();
function Uo(e) {
  let t = ra;
  if (e && e instanceof RegExp)
    if (e.global)
      t = e;
    else {
      let n = "g";
      e.ignoreCase && (n += "i"), e.multiline && (n += "m"), e.unicode && (n += "u"), t = new RegExp(e.source, n);
    }
  return t.lastIndex = 0, t;
}
const ia = new Ma();
ia.unshift({
  maxLen: 1e3,
  windowSize: 15,
  timeBudget: 150
});
function Rr(e, t, n, r, i) {
  if (i || (i = mn.first(ia)), n.length > i.maxLen) {
    let u = e - i.maxLen / 2;
    return u < 0 ? u = 0 : r += u, n = n.substring(u, e + i.maxLen / 2), Rr(e, t, n, r, i);
  }
  const s = Date.now(), a = e - 1 - r;
  let o = -1, l = null;
  for (let u = 1; !(Date.now() - s >= i.timeBudget); u++) {
    const c = a - i.windowSize * u;
    t.lastIndex = Math.max(0, c);
    const f = $o(t, n, a, o);
    if (!f && l || (l = f, c <= 0))
      break;
    o = c;
  }
  if (l) {
    const u = {
      word: l[0],
      startColumn: r + 1 + l.index,
      endColumn: r + 1 + l.index + l[0].length
    };
    return t.lastIndex = 0, u;
  }
  return null;
}
function $o(e, t, n, r) {
  let i;
  for (; i = e.exec(t); ) {
    const s = i.index || 0;
    if (s <= n && e.lastIndex >= n)
      return i;
    if (r > 0 && s > r)
      return null;
  }
  return null;
}
class Tr {
  constructor(t) {
    const n = Qr(t);
    this._defaultValue = n, this._asciiMap = Tr._createAsciiMap(n), this._map = /* @__PURE__ */ new Map();
  }
  static _createAsciiMap(t) {
    const n = new Uint8Array(256);
    return n.fill(t), n;
  }
  set(t, n) {
    const r = Qr(n);
    t >= 0 && t < 256 ? this._asciiMap[t] = r : this._map.set(t, r);
  }
  get(t) {
    return t >= 0 && t < 256 ? this._asciiMap[t] : this._map.get(t) || this._defaultValue;
  }
  clear() {
    this._asciiMap.fill(this._defaultValue), this._map.clear();
  }
}
class Wo {
  constructor(t, n, r) {
    const i = new Uint8Array(t * n);
    for (let s = 0, a = t * n; s < a; s++)
      i[s] = r;
    this._data = i, this.rows = t, this.cols = n;
  }
  get(t, n) {
    return this._data[t * this.cols + n];
  }
  set(t, n, r) {
    this._data[t * this.cols + n] = r;
  }
}
class Ho {
  constructor(t) {
    let n = 0, r = 0;
    for (let s = 0, a = t.length; s < a; s++) {
      const [o, l, u] = t[s];
      l > n && (n = l), o > r && (r = o), u > r && (r = u);
    }
    n++, r++;
    const i = new Wo(
      r,
      n,
      0
      /* State.Invalid */
    );
    for (let s = 0, a = t.length; s < a; s++) {
      const [o, l, u] = t[s];
      i.set(o, l, u);
    }
    this._states = i, this._maxCharCode = n;
  }
  nextState(t, n) {
    return n < 0 || n >= this._maxCharCode ? 0 : this._states.get(t, n);
  }
}
let On = null;
function zo() {
  return On === null && (On = new Ho([
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
  ])), On;
}
let Mt = null;
function Go() {
  if (Mt === null) {
    Mt = new Tr(
      0
      /* CharacterClass.None */
    );
    const e = ` 	<>'"、。｡､，．：；‘〈「『〔（［｛｢｣｝］）〕』」〉’｀～…`;
    for (let n = 0; n < e.length; n++)
      Mt.set(
        e.charCodeAt(n),
        1
        /* CharacterClass.ForceTermination */
      );
    const t = ".,;:";
    for (let n = 0; n < t.length; n++)
      Mt.set(
        t.charCodeAt(n),
        2
        /* CharacterClass.CannotEndIn */
      );
  }
  return Mt;
}
class bn {
  static _createLink(t, n, r, i, s) {
    let a = s - 1;
    do {
      const o = n.charCodeAt(a);
      if (t.get(o) !== 2)
        break;
      a--;
    } while (a > i);
    if (i > 0) {
      const o = n.charCodeAt(i - 1), l = n.charCodeAt(a);
      (o === 40 && l === 41 || o === 91 && l === 93 || o === 123 && l === 125) && a--;
    }
    return {
      range: {
        startLineNumber: r,
        startColumn: i + 1,
        endLineNumber: r,
        endColumn: a + 2
      },
      url: n.substring(i, a + 1)
    };
  }
  static computeLinks(t, n = zo()) {
    const r = Go(), i = [];
    for (let s = 1, a = t.getLineCount(); s <= a; s++) {
      const o = t.getLineContent(s), l = o.length;
      let u = 0, c = 0, f = 0, d = 1, g = !1, m = !1, p = !1, v = !1;
      for (; u < l; ) {
        let x = !1;
        const b = o.charCodeAt(u);
        if (d === 13) {
          let y;
          switch (b) {
            case 40:
              g = !0, y = 0;
              break;
            case 41:
              y = g ? 0 : 1;
              break;
            case 91:
              p = !0, m = !0, y = 0;
              break;
            case 93:
              p = !1, y = m ? 0 : 1;
              break;
            case 123:
              v = !0, y = 0;
              break;
            case 125:
              y = v ? 0 : 1;
              break;
            case 39:
            case 34:
            case 96:
              f === b ? y = 1 : f === 39 || f === 34 || f === 96 ? y = 0 : y = 1;
              break;
            case 42:
              y = f === 42 ? 1 : 0;
              break;
            case 124:
              y = f === 124 ? 1 : 0;
              break;
            case 32:
              y = p ? 0 : 1;
              break;
            default:
              y = r.get(b);
          }
          y === 1 && (i.push(bn._createLink(r, o, s, c, u)), x = !0);
        } else if (d === 12) {
          let y;
          b === 91 ? (m = !0, y = 0) : y = r.get(b), y === 1 ? x = !0 : d = 13;
        } else
          d = n.nextState(d, b), d === 0 && (x = !0);
        x && (d = 1, g = !1, m = !1, v = !1, c = u + 1, f = b), u++;
      }
      d === 13 && i.push(bn._createLink(r, o, s, c, l));
    }
    return i;
  }
}
function Jo(e) {
  return !e || typeof e.getLineCount != "function" || typeof e.getLineContent != "function" ? [] : bn.computeLinks(e);
}
class rr {
  constructor() {
    this._defaultValueSet = [
      ["true", "false"],
      ["True", "False"],
      ["Private", "Public", "Friend", "ReadOnly", "Partial", "Protected", "WriteOnly"],
      ["public", "protected", "private"]
    ];
  }
  navigateValueSet(t, n, r, i, s) {
    if (t && n) {
      const a = this.doNavigateValueSet(n, s);
      if (a)
        return {
          range: t,
          value: a
        };
    }
    if (r && i) {
      const a = this.doNavigateValueSet(i, s);
      if (a)
        return {
          range: r,
          value: a
        };
    }
    return null;
  }
  doNavigateValueSet(t, n) {
    const r = this.numberReplace(t, n);
    return r !== null ? r : this.textReplace(t, n);
  }
  numberReplace(t, n) {
    const r = Math.pow(10, t.length - (t.lastIndexOf(".") + 1));
    let i = Number(t);
    const s = parseFloat(t);
    return !isNaN(i) && !isNaN(s) && i === s ? i === 0 && !n ? null : (i = Math.floor(i * r), i += n ? r : -r, String(i / r)) : null;
  }
  textReplace(t, n) {
    return this.valueSetsReplace(this._defaultValueSet, t, n);
  }
  valueSetsReplace(t, n, r) {
    let i = null;
    for (let s = 0, a = t.length; i === null && s < a; s++)
      i = this.valueSetReplace(t[s], n, r);
    return i;
  }
  valueSetReplace(t, n, r) {
    let i = t.indexOf(n);
    return i >= 0 ? (i += r ? 1 : -1, i < 0 ? i = t.length - 1 : i %= t.length, t[i]) : null;
  }
}
rr.INSTANCE = new rr();
const sa = Object.freeze(function(e, t) {
  const n = setTimeout(e.bind(t), 0);
  return { dispose() {
    clearTimeout(n);
  } };
});
var yn;
(function(e) {
  function t(n) {
    return n === e.None || n === e.Cancelled || n instanceof hn ? !0 : !n || typeof n != "object" ? !1 : typeof n.isCancellationRequested == "boolean" && typeof n.onCancellationRequested == "function";
  }
  e.isCancellationToken = t, e.None = Object.freeze({
    isCancellationRequested: !1,
    onCancellationRequested: Xn.None
  }), e.Cancelled = Object.freeze({
    isCancellationRequested: !0,
    onCancellationRequested: sa
  });
})(yn || (yn = {}));
class hn {
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
    return this._isCancelled ? sa : (this._emitter || (this._emitter = new Ae()), this._emitter.event);
  }
  dispose() {
    this._emitter && (this._emitter.dispose(), this._emitter = null);
  }
}
class Xo {
  constructor(t) {
    this._token = void 0, this._parentListener = void 0, this._parentListener = t && t.onCancellationRequested(this.cancel, this);
  }
  get token() {
    return this._token || (this._token = new hn()), this._token;
  }
  cancel() {
    this._token ? this._token instanceof hn && this._token.cancel() : this._token = yn.Cancelled;
  }
  dispose(t = !1) {
    var n;
    t && this.cancel(), (n = this._parentListener) === null || n === void 0 || n.dispose(), this._token ? this._token instanceof hn && this._token.dispose() : this._token = yn.None;
  }
}
class Pr {
  constructor() {
    this._keyCodeToStr = [], this._strToKeyCode = /* @__PURE__ */ Object.create(null);
  }
  define(t, n) {
    this._keyCodeToStr[t] = n, this._strToKeyCode[n.toLowerCase()] = t;
  }
  keyCodeToStr(t) {
    return this._keyCodeToStr[t];
  }
  strToKeyCode(t) {
    return this._strToKeyCode[t.toLowerCase()] || 0;
  }
}
const dn = new Pr(), ir = new Pr(), sr = new Pr(), Qo = new Array(230), Zo = /* @__PURE__ */ Object.create(null), Yo = /* @__PURE__ */ Object.create(null);
(function() {
  const e = "", t = [
    // immutable, scanCode, scanCodeStr, keyCode, keyCodeStr, eventKeyCode, vkey, usUserSettingsLabel, generalUserSettingsLabel
    [1, 0, "None", 0, "unknown", 0, "VK_UNKNOWN", e, e],
    [1, 1, "Hyper", 0, e, 0, e, e, e],
    [1, 2, "Super", 0, e, 0, e, e, e],
    [1, 3, "Fn", 0, e, 0, e, e, e],
    [1, 4, "FnLock", 0, e, 0, e, e, e],
    [1, 5, "Suspend", 0, e, 0, e, e, e],
    [1, 6, "Resume", 0, e, 0, e, e, e],
    [1, 7, "Turbo", 0, e, 0, e, e, e],
    [1, 8, "Sleep", 0, e, 0, "VK_SLEEP", e, e],
    [1, 9, "WakeUp", 0, e, 0, e, e, e],
    [0, 10, "KeyA", 31, "A", 65, "VK_A", e, e],
    [0, 11, "KeyB", 32, "B", 66, "VK_B", e, e],
    [0, 12, "KeyC", 33, "C", 67, "VK_C", e, e],
    [0, 13, "KeyD", 34, "D", 68, "VK_D", e, e],
    [0, 14, "KeyE", 35, "E", 69, "VK_E", e, e],
    [0, 15, "KeyF", 36, "F", 70, "VK_F", e, e],
    [0, 16, "KeyG", 37, "G", 71, "VK_G", e, e],
    [0, 17, "KeyH", 38, "H", 72, "VK_H", e, e],
    [0, 18, "KeyI", 39, "I", 73, "VK_I", e, e],
    [0, 19, "KeyJ", 40, "J", 74, "VK_J", e, e],
    [0, 20, "KeyK", 41, "K", 75, "VK_K", e, e],
    [0, 21, "KeyL", 42, "L", 76, "VK_L", e, e],
    [0, 22, "KeyM", 43, "M", 77, "VK_M", e, e],
    [0, 23, "KeyN", 44, "N", 78, "VK_N", e, e],
    [0, 24, "KeyO", 45, "O", 79, "VK_O", e, e],
    [0, 25, "KeyP", 46, "P", 80, "VK_P", e, e],
    [0, 26, "KeyQ", 47, "Q", 81, "VK_Q", e, e],
    [0, 27, "KeyR", 48, "R", 82, "VK_R", e, e],
    [0, 28, "KeyS", 49, "S", 83, "VK_S", e, e],
    [0, 29, "KeyT", 50, "T", 84, "VK_T", e, e],
    [0, 30, "KeyU", 51, "U", 85, "VK_U", e, e],
    [0, 31, "KeyV", 52, "V", 86, "VK_V", e, e],
    [0, 32, "KeyW", 53, "W", 87, "VK_W", e, e],
    [0, 33, "KeyX", 54, "X", 88, "VK_X", e, e],
    [0, 34, "KeyY", 55, "Y", 89, "VK_Y", e, e],
    [0, 35, "KeyZ", 56, "Z", 90, "VK_Z", e, e],
    [0, 36, "Digit1", 22, "1", 49, "VK_1", e, e],
    [0, 37, "Digit2", 23, "2", 50, "VK_2", e, e],
    [0, 38, "Digit3", 24, "3", 51, "VK_3", e, e],
    [0, 39, "Digit4", 25, "4", 52, "VK_4", e, e],
    [0, 40, "Digit5", 26, "5", 53, "VK_5", e, e],
    [0, 41, "Digit6", 27, "6", 54, "VK_6", e, e],
    [0, 42, "Digit7", 28, "7", 55, "VK_7", e, e],
    [0, 43, "Digit8", 29, "8", 56, "VK_8", e, e],
    [0, 44, "Digit9", 30, "9", 57, "VK_9", e, e],
    [0, 45, "Digit0", 21, "0", 48, "VK_0", e, e],
    [1, 46, "Enter", 3, "Enter", 13, "VK_RETURN", e, e],
    [1, 47, "Escape", 9, "Escape", 27, "VK_ESCAPE", e, e],
    [1, 48, "Backspace", 1, "Backspace", 8, "VK_BACK", e, e],
    [1, 49, "Tab", 2, "Tab", 9, "VK_TAB", e, e],
    [1, 50, "Space", 10, "Space", 32, "VK_SPACE", e, e],
    [0, 51, "Minus", 88, "-", 189, "VK_OEM_MINUS", "-", "OEM_MINUS"],
    [0, 52, "Equal", 86, "=", 187, "VK_OEM_PLUS", "=", "OEM_PLUS"],
    [0, 53, "BracketLeft", 92, "[", 219, "VK_OEM_4", "[", "OEM_4"],
    [0, 54, "BracketRight", 94, "]", 221, "VK_OEM_6", "]", "OEM_6"],
    [0, 55, "Backslash", 93, "\\", 220, "VK_OEM_5", "\\", "OEM_5"],
    [0, 56, "IntlHash", 0, e, 0, e, e, e],
    // has been dropped from the w3c spec
    [0, 57, "Semicolon", 85, ";", 186, "VK_OEM_1", ";", "OEM_1"],
    [0, 58, "Quote", 95, "'", 222, "VK_OEM_7", "'", "OEM_7"],
    [0, 59, "Backquote", 91, "`", 192, "VK_OEM_3", "`", "OEM_3"],
    [0, 60, "Comma", 87, ",", 188, "VK_OEM_COMMA", ",", "OEM_COMMA"],
    [0, 61, "Period", 89, ".", 190, "VK_OEM_PERIOD", ".", "OEM_PERIOD"],
    [0, 62, "Slash", 90, "/", 191, "VK_OEM_2", "/", "OEM_2"],
    [1, 63, "CapsLock", 8, "CapsLock", 20, "VK_CAPITAL", e, e],
    [1, 64, "F1", 59, "F1", 112, "VK_F1", e, e],
    [1, 65, "F2", 60, "F2", 113, "VK_F2", e, e],
    [1, 66, "F3", 61, "F3", 114, "VK_F3", e, e],
    [1, 67, "F4", 62, "F4", 115, "VK_F4", e, e],
    [1, 68, "F5", 63, "F5", 116, "VK_F5", e, e],
    [1, 69, "F6", 64, "F6", 117, "VK_F6", e, e],
    [1, 70, "F7", 65, "F7", 118, "VK_F7", e, e],
    [1, 71, "F8", 66, "F8", 119, "VK_F8", e, e],
    [1, 72, "F9", 67, "F9", 120, "VK_F9", e, e],
    [1, 73, "F10", 68, "F10", 121, "VK_F10", e, e],
    [1, 74, "F11", 69, "F11", 122, "VK_F11", e, e],
    [1, 75, "F12", 70, "F12", 123, "VK_F12", e, e],
    [1, 76, "PrintScreen", 0, e, 0, e, e, e],
    [1, 77, "ScrollLock", 84, "ScrollLock", 145, "VK_SCROLL", e, e],
    [1, 78, "Pause", 7, "PauseBreak", 19, "VK_PAUSE", e, e],
    [1, 79, "Insert", 19, "Insert", 45, "VK_INSERT", e, e],
    [1, 80, "Home", 14, "Home", 36, "VK_HOME", e, e],
    [1, 81, "PageUp", 11, "PageUp", 33, "VK_PRIOR", e, e],
    [1, 82, "Delete", 20, "Delete", 46, "VK_DELETE", e, e],
    [1, 83, "End", 13, "End", 35, "VK_END", e, e],
    [1, 84, "PageDown", 12, "PageDown", 34, "VK_NEXT", e, e],
    [1, 85, "ArrowRight", 17, "RightArrow", 39, "VK_RIGHT", "Right", e],
    [1, 86, "ArrowLeft", 15, "LeftArrow", 37, "VK_LEFT", "Left", e],
    [1, 87, "ArrowDown", 18, "DownArrow", 40, "VK_DOWN", "Down", e],
    [1, 88, "ArrowUp", 16, "UpArrow", 38, "VK_UP", "Up", e],
    [1, 89, "NumLock", 83, "NumLock", 144, "VK_NUMLOCK", e, e],
    [1, 90, "NumpadDivide", 113, "NumPad_Divide", 111, "VK_DIVIDE", e, e],
    [1, 91, "NumpadMultiply", 108, "NumPad_Multiply", 106, "VK_MULTIPLY", e, e],
    [1, 92, "NumpadSubtract", 111, "NumPad_Subtract", 109, "VK_SUBTRACT", e, e],
    [1, 93, "NumpadAdd", 109, "NumPad_Add", 107, "VK_ADD", e, e],
    [1, 94, "NumpadEnter", 3, e, 0, e, e, e],
    [1, 95, "Numpad1", 99, "NumPad1", 97, "VK_NUMPAD1", e, e],
    [1, 96, "Numpad2", 100, "NumPad2", 98, "VK_NUMPAD2", e, e],
    [1, 97, "Numpad3", 101, "NumPad3", 99, "VK_NUMPAD3", e, e],
    [1, 98, "Numpad4", 102, "NumPad4", 100, "VK_NUMPAD4", e, e],
    [1, 99, "Numpad5", 103, "NumPad5", 101, "VK_NUMPAD5", e, e],
    [1, 100, "Numpad6", 104, "NumPad6", 102, "VK_NUMPAD6", e, e],
    [1, 101, "Numpad7", 105, "NumPad7", 103, "VK_NUMPAD7", e, e],
    [1, 102, "Numpad8", 106, "NumPad8", 104, "VK_NUMPAD8", e, e],
    [1, 103, "Numpad9", 107, "NumPad9", 105, "VK_NUMPAD9", e, e],
    [1, 104, "Numpad0", 98, "NumPad0", 96, "VK_NUMPAD0", e, e],
    [1, 105, "NumpadDecimal", 112, "NumPad_Decimal", 110, "VK_DECIMAL", e, e],
    [0, 106, "IntlBackslash", 97, "OEM_102", 226, "VK_OEM_102", e, e],
    [1, 107, "ContextMenu", 58, "ContextMenu", 93, e, e, e],
    [1, 108, "Power", 0, e, 0, e, e, e],
    [1, 109, "NumpadEqual", 0, e, 0, e, e, e],
    [1, 110, "F13", 71, "F13", 124, "VK_F13", e, e],
    [1, 111, "F14", 72, "F14", 125, "VK_F14", e, e],
    [1, 112, "F15", 73, "F15", 126, "VK_F15", e, e],
    [1, 113, "F16", 74, "F16", 127, "VK_F16", e, e],
    [1, 114, "F17", 75, "F17", 128, "VK_F17", e, e],
    [1, 115, "F18", 76, "F18", 129, "VK_F18", e, e],
    [1, 116, "F19", 77, "F19", 130, "VK_F19", e, e],
    [1, 117, "F20", 78, "F20", 131, "VK_F20", e, e],
    [1, 118, "F21", 79, "F21", 132, "VK_F21", e, e],
    [1, 119, "F22", 80, "F22", 133, "VK_F22", e, e],
    [1, 120, "F23", 81, "F23", 134, "VK_F23", e, e],
    [1, 121, "F24", 82, "F24", 135, "VK_F24", e, e],
    [1, 122, "Open", 0, e, 0, e, e, e],
    [1, 123, "Help", 0, e, 0, e, e, e],
    [1, 124, "Select", 0, e, 0, e, e, e],
    [1, 125, "Again", 0, e, 0, e, e, e],
    [1, 126, "Undo", 0, e, 0, e, e, e],
    [1, 127, "Cut", 0, e, 0, e, e, e],
    [1, 128, "Copy", 0, e, 0, e, e, e],
    [1, 129, "Paste", 0, e, 0, e, e, e],
    [1, 130, "Find", 0, e, 0, e, e, e],
    [1, 131, "AudioVolumeMute", 117, "AudioVolumeMute", 173, "VK_VOLUME_MUTE", e, e],
    [1, 132, "AudioVolumeUp", 118, "AudioVolumeUp", 175, "VK_VOLUME_UP", e, e],
    [1, 133, "AudioVolumeDown", 119, "AudioVolumeDown", 174, "VK_VOLUME_DOWN", e, e],
    [1, 134, "NumpadComma", 110, "NumPad_Separator", 108, "VK_SEPARATOR", e, e],
    [0, 135, "IntlRo", 115, "ABNT_C1", 193, "VK_ABNT_C1", e, e],
    [1, 136, "KanaMode", 0, e, 0, e, e, e],
    [0, 137, "IntlYen", 0, e, 0, e, e, e],
    [1, 138, "Convert", 0, e, 0, e, e, e],
    [1, 139, "NonConvert", 0, e, 0, e, e, e],
    [1, 140, "Lang1", 0, e, 0, e, e, e],
    [1, 141, "Lang2", 0, e, 0, e, e, e],
    [1, 142, "Lang3", 0, e, 0, e, e, e],
    [1, 143, "Lang4", 0, e, 0, e, e, e],
    [1, 144, "Lang5", 0, e, 0, e, e, e],
    [1, 145, "Abort", 0, e, 0, e, e, e],
    [1, 146, "Props", 0, e, 0, e, e, e],
    [1, 147, "NumpadParenLeft", 0, e, 0, e, e, e],
    [1, 148, "NumpadParenRight", 0, e, 0, e, e, e],
    [1, 149, "NumpadBackspace", 0, e, 0, e, e, e],
    [1, 150, "NumpadMemoryStore", 0, e, 0, e, e, e],
    [1, 151, "NumpadMemoryRecall", 0, e, 0, e, e, e],
    [1, 152, "NumpadMemoryClear", 0, e, 0, e, e, e],
    [1, 153, "NumpadMemoryAdd", 0, e, 0, e, e, e],
    [1, 154, "NumpadMemorySubtract", 0, e, 0, e, e, e],
    [1, 155, "NumpadClear", 131, "Clear", 12, "VK_CLEAR", e, e],
    [1, 156, "NumpadClearEntry", 0, e, 0, e, e, e],
    [1, 0, e, 5, "Ctrl", 17, "VK_CONTROL", e, e],
    [1, 0, e, 4, "Shift", 16, "VK_SHIFT", e, e],
    [1, 0, e, 6, "Alt", 18, "VK_MENU", e, e],
    [1, 0, e, 57, "Meta", 91, "VK_COMMAND", e, e],
    [1, 157, "ControlLeft", 5, e, 0, "VK_LCONTROL", e, e],
    [1, 158, "ShiftLeft", 4, e, 0, "VK_LSHIFT", e, e],
    [1, 159, "AltLeft", 6, e, 0, "VK_LMENU", e, e],
    [1, 160, "MetaLeft", 57, e, 0, "VK_LWIN", e, e],
    [1, 161, "ControlRight", 5, e, 0, "VK_RCONTROL", e, e],
    [1, 162, "ShiftRight", 4, e, 0, "VK_RSHIFT", e, e],
    [1, 163, "AltRight", 6, e, 0, "VK_RMENU", e, e],
    [1, 164, "MetaRight", 57, e, 0, "VK_RWIN", e, e],
    [1, 165, "BrightnessUp", 0, e, 0, e, e, e],
    [1, 166, "BrightnessDown", 0, e, 0, e, e, e],
    [1, 167, "MediaPlay", 0, e, 0, e, e, e],
    [1, 168, "MediaRecord", 0, e, 0, e, e, e],
    [1, 169, "MediaFastForward", 0, e, 0, e, e, e],
    [1, 170, "MediaRewind", 0, e, 0, e, e, e],
    [1, 171, "MediaTrackNext", 124, "MediaTrackNext", 176, "VK_MEDIA_NEXT_TRACK", e, e],
    [1, 172, "MediaTrackPrevious", 125, "MediaTrackPrevious", 177, "VK_MEDIA_PREV_TRACK", e, e],
    [1, 173, "MediaStop", 126, "MediaStop", 178, "VK_MEDIA_STOP", e, e],
    [1, 174, "Eject", 0, e, 0, e, e, e],
    [1, 175, "MediaPlayPause", 127, "MediaPlayPause", 179, "VK_MEDIA_PLAY_PAUSE", e, e],
    [1, 176, "MediaSelect", 128, "LaunchMediaPlayer", 181, "VK_MEDIA_LAUNCH_MEDIA_SELECT", e, e],
    [1, 177, "LaunchMail", 129, "LaunchMail", 180, "VK_MEDIA_LAUNCH_MAIL", e, e],
    [1, 178, "LaunchApp2", 130, "LaunchApp2", 183, "VK_MEDIA_LAUNCH_APP2", e, e],
    [1, 179, "LaunchApp1", 0, e, 0, "VK_MEDIA_LAUNCH_APP1", e, e],
    [1, 180, "SelectTask", 0, e, 0, e, e, e],
    [1, 181, "LaunchScreenSaver", 0, e, 0, e, e, e],
    [1, 182, "BrowserSearch", 120, "BrowserSearch", 170, "VK_BROWSER_SEARCH", e, e],
    [1, 183, "BrowserHome", 121, "BrowserHome", 172, "VK_BROWSER_HOME", e, e],
    [1, 184, "BrowserBack", 122, "BrowserBack", 166, "VK_BROWSER_BACK", e, e],
    [1, 185, "BrowserForward", 123, "BrowserForward", 167, "VK_BROWSER_FORWARD", e, e],
    [1, 186, "BrowserStop", 0, e, 0, "VK_BROWSER_STOP", e, e],
    [1, 187, "BrowserRefresh", 0, e, 0, "VK_BROWSER_REFRESH", e, e],
    [1, 188, "BrowserFavorites", 0, e, 0, "VK_BROWSER_FAVORITES", e, e],
    [1, 189, "ZoomToggle", 0, e, 0, e, e, e],
    [1, 190, "MailReply", 0, e, 0, e, e, e],
    [1, 191, "MailForward", 0, e, 0, e, e, e],
    [1, 192, "MailSend", 0, e, 0, e, e, e],
    // See https://lists.w3.org/Archives/Public/www-dom/2010JulSep/att-0182/keyCode-spec.html
    // If an Input Method Editor is processing key input and the event is keydown, return 229.
    [1, 0, e, 114, "KeyInComposition", 229, e, e, e],
    [1, 0, e, 116, "ABNT_C2", 194, "VK_ABNT_C2", e, e],
    [1, 0, e, 96, "OEM_8", 223, "VK_OEM_8", e, e],
    [1, 0, e, 0, e, 0, "VK_KANA", e, e],
    [1, 0, e, 0, e, 0, "VK_HANGUL", e, e],
    [1, 0, e, 0, e, 0, "VK_JUNJA", e, e],
    [1, 0, e, 0, e, 0, "VK_FINAL", e, e],
    [1, 0, e, 0, e, 0, "VK_HANJA", e, e],
    [1, 0, e, 0, e, 0, "VK_KANJI", e, e],
    [1, 0, e, 0, e, 0, "VK_CONVERT", e, e],
    [1, 0, e, 0, e, 0, "VK_NONCONVERT", e, e],
    [1, 0, e, 0, e, 0, "VK_ACCEPT", e, e],
    [1, 0, e, 0, e, 0, "VK_MODECHANGE", e, e],
    [1, 0, e, 0, e, 0, "VK_SELECT", e, e],
    [1, 0, e, 0, e, 0, "VK_PRINT", e, e],
    [1, 0, e, 0, e, 0, "VK_EXECUTE", e, e],
    [1, 0, e, 0, e, 0, "VK_SNAPSHOT", e, e],
    [1, 0, e, 0, e, 0, "VK_HELP", e, e],
    [1, 0, e, 0, e, 0, "VK_APPS", e, e],
    [1, 0, e, 0, e, 0, "VK_PROCESSKEY", e, e],
    [1, 0, e, 0, e, 0, "VK_PACKET", e, e],
    [1, 0, e, 0, e, 0, "VK_DBE_SBCSCHAR", e, e],
    [1, 0, e, 0, e, 0, "VK_DBE_DBCSCHAR", e, e],
    [1, 0, e, 0, e, 0, "VK_ATTN", e, e],
    [1, 0, e, 0, e, 0, "VK_CRSEL", e, e],
    [1, 0, e, 0, e, 0, "VK_EXSEL", e, e],
    [1, 0, e, 0, e, 0, "VK_EREOF", e, e],
    [1, 0, e, 0, e, 0, "VK_PLAY", e, e],
    [1, 0, e, 0, e, 0, "VK_ZOOM", e, e],
    [1, 0, e, 0, e, 0, "VK_NONAME", e, e],
    [1, 0, e, 0, e, 0, "VK_PA1", e, e],
    [1, 0, e, 0, e, 0, "VK_OEM_CLEAR", e, e]
  ], n = [], r = [];
  for (const i of t) {
    const [s, a, o, l, u, c, f, d, g] = i;
    if (r[a] || (r[a] = !0, Zo[o] = a, Yo[o.toLowerCase()] = a), !n[l]) {
      if (n[l] = !0, !u)
        throw new Error(`String representation missing for key code ${l} around scan code ${o}`);
      dn.define(l, u), ir.define(l, d || u), sr.define(l, g || d || u);
    }
    c && (Qo[c] = l);
  }
})();
var Zr;
(function(e) {
  function t(o) {
    return dn.keyCodeToStr(o);
  }
  e.toString = t;
  function n(o) {
    return dn.strToKeyCode(o);
  }
  e.fromString = n;
  function r(o) {
    return ir.keyCodeToStr(o);
  }
  e.toUserSettingsUS = r;
  function i(o) {
    return sr.keyCodeToStr(o);
  }
  e.toUserSettingsGeneral = i;
  function s(o) {
    return ir.strToKeyCode(o) || sr.strToKeyCode(o);
  }
  e.fromUserSettings = s;
  function a(o) {
    if (o >= 98 && o <= 113)
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
    return dn.keyCodeToStr(o);
  }
  e.toElectronAccelerator = a;
})(Zr || (Zr = {}));
function Ko(e, t) {
  const n = (t & 65535) << 16 >>> 0;
  return (e | n) >>> 0;
}
class ye extends ae {
  constructor(t, n, r, i) {
    super(t, n, r, i), this.selectionStartLineNumber = t, this.selectionStartColumn = n, this.positionLineNumber = r, this.positionColumn = i;
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
    return ye.selectionsEqual(this, t);
  }
  /**
   * Test if the two selections are equal.
   */
  static selectionsEqual(t, n) {
    return t.selectionStartLineNumber === n.selectionStartLineNumber && t.selectionStartColumn === n.selectionStartColumn && t.positionLineNumber === n.positionLineNumber && t.positionColumn === n.positionColumn;
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
  setEndPosition(t, n) {
    return this.getDirection() === 0 ? new ye(this.startLineNumber, this.startColumn, t, n) : new ye(t, n, this.startLineNumber, this.startColumn);
  }
  /**
   * Get the position at `positionLineNumber` and `positionColumn`.
   */
  getPosition() {
    return new Oe(this.positionLineNumber, this.positionColumn);
  }
  /**
   * Get the position at the start of the selection.
  */
  getSelectionStart() {
    return new Oe(this.selectionStartLineNumber, this.selectionStartColumn);
  }
  /**
   * Create a new selection with a different `selectionStartLineNumber` and `selectionStartColumn`.
   */
  setStartPosition(t, n) {
    return this.getDirection() === 0 ? new ye(t, n, this.endLineNumber, this.endColumn) : new ye(this.endLineNumber, this.endColumn, t, n);
  }
  // ----
  /**
   * Create a `Selection` from one or two positions
   */
  static fromPositions(t, n = t) {
    return new ye(t.lineNumber, t.column, n.lineNumber, n.column);
  }
  /**
   * Creates a `Selection` from a range, given a direction.
   */
  static fromRange(t, n) {
    return n === 0 ? new ye(t.startLineNumber, t.startColumn, t.endLineNumber, t.endColumn) : new ye(t.endLineNumber, t.endColumn, t.startLineNumber, t.startColumn);
  }
  /**
   * Create a `Selection` from an `ISelection`.
   */
  static liftSelection(t) {
    return new ye(t.selectionStartLineNumber, t.selectionStartColumn, t.positionLineNumber, t.positionColumn);
  }
  /**
   * `a` equals `b`.
   */
  static selectionsArrEqual(t, n) {
    if (t && !n || !t && n)
      return !1;
    if (!t && !n)
      return !0;
    if (t.length !== n.length)
      return !1;
    for (let r = 0, i = t.length; r < i; r++)
      if (!this.selectionsEqual(t[r], n[r]))
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
  static createWithDirection(t, n, r, i, s) {
    return s === 0 ? new ye(t, n, r, i) : new ye(r, i, t, n);
  }
}
const Yr = /* @__PURE__ */ Object.create(null);
function h(e, t) {
  if (Da(t)) {
    const n = Yr[t];
    if (n === void 0)
      throw new Error(`${e} references an unknown codicon: ${t}`);
    t = n;
  }
  return Yr[e] = t, { id: e };
}
const q = {
  // built-in icons, with image name
  add: h("add", 6e4),
  plus: h("plus", 6e4),
  gistNew: h("gist-new", 6e4),
  repoCreate: h("repo-create", 6e4),
  lightbulb: h("lightbulb", 60001),
  lightBulb: h("light-bulb", 60001),
  repo: h("repo", 60002),
  repoDelete: h("repo-delete", 60002),
  gistFork: h("gist-fork", 60003),
  repoForked: h("repo-forked", 60003),
  gitPullRequest: h("git-pull-request", 60004),
  gitPullRequestAbandoned: h("git-pull-request-abandoned", 60004),
  recordKeys: h("record-keys", 60005),
  keyboard: h("keyboard", 60005),
  tag: h("tag", 60006),
  tagAdd: h("tag-add", 60006),
  tagRemove: h("tag-remove", 60006),
  gitPullRequestLabel: h("git-pull-request-label", 60006),
  person: h("person", 60007),
  personFollow: h("person-follow", 60007),
  personOutline: h("person-outline", 60007),
  personFilled: h("person-filled", 60007),
  gitBranch: h("git-branch", 60008),
  gitBranchCreate: h("git-branch-create", 60008),
  gitBranchDelete: h("git-branch-delete", 60008),
  sourceControl: h("source-control", 60008),
  mirror: h("mirror", 60009),
  mirrorPublic: h("mirror-public", 60009),
  star: h("star", 60010),
  starAdd: h("star-add", 60010),
  starDelete: h("star-delete", 60010),
  starEmpty: h("star-empty", 60010),
  comment: h("comment", 60011),
  commentAdd: h("comment-add", 60011),
  alert: h("alert", 60012),
  warning: h("warning", 60012),
  search: h("search", 60013),
  searchSave: h("search-save", 60013),
  logOut: h("log-out", 60014),
  signOut: h("sign-out", 60014),
  logIn: h("log-in", 60015),
  signIn: h("sign-in", 60015),
  eye: h("eye", 60016),
  eyeUnwatch: h("eye-unwatch", 60016),
  eyeWatch: h("eye-watch", 60016),
  circleFilled: h("circle-filled", 60017),
  primitiveDot: h("primitive-dot", 60017),
  closeDirty: h("close-dirty", 60017),
  debugBreakpoint: h("debug-breakpoint", 60017),
  debugBreakpointDisabled: h("debug-breakpoint-disabled", 60017),
  debugHint: h("debug-hint", 60017),
  primitiveSquare: h("primitive-square", 60018),
  edit: h("edit", 60019),
  pencil: h("pencil", 60019),
  info: h("info", 60020),
  issueOpened: h("issue-opened", 60020),
  gistPrivate: h("gist-private", 60021),
  gitForkPrivate: h("git-fork-private", 60021),
  lock: h("lock", 60021),
  mirrorPrivate: h("mirror-private", 60021),
  close: h("close", 60022),
  removeClose: h("remove-close", 60022),
  x: h("x", 60022),
  repoSync: h("repo-sync", 60023),
  sync: h("sync", 60023),
  clone: h("clone", 60024),
  desktopDownload: h("desktop-download", 60024),
  beaker: h("beaker", 60025),
  microscope: h("microscope", 60025),
  vm: h("vm", 60026),
  deviceDesktop: h("device-desktop", 60026),
  file: h("file", 60027),
  fileText: h("file-text", 60027),
  more: h("more", 60028),
  ellipsis: h("ellipsis", 60028),
  kebabHorizontal: h("kebab-horizontal", 60028),
  mailReply: h("mail-reply", 60029),
  reply: h("reply", 60029),
  organization: h("organization", 60030),
  organizationFilled: h("organization-filled", 60030),
  organizationOutline: h("organization-outline", 60030),
  newFile: h("new-file", 60031),
  fileAdd: h("file-add", 60031),
  newFolder: h("new-folder", 60032),
  fileDirectoryCreate: h("file-directory-create", 60032),
  trash: h("trash", 60033),
  trashcan: h("trashcan", 60033),
  history: h("history", 60034),
  clock: h("clock", 60034),
  folder: h("folder", 60035),
  fileDirectory: h("file-directory", 60035),
  symbolFolder: h("symbol-folder", 60035),
  logoGithub: h("logo-github", 60036),
  markGithub: h("mark-github", 60036),
  github: h("github", 60036),
  terminal: h("terminal", 60037),
  console: h("console", 60037),
  repl: h("repl", 60037),
  zap: h("zap", 60038),
  symbolEvent: h("symbol-event", 60038),
  error: h("error", 60039),
  stop: h("stop", 60039),
  variable: h("variable", 60040),
  symbolVariable: h("symbol-variable", 60040),
  array: h("array", 60042),
  symbolArray: h("symbol-array", 60042),
  symbolModule: h("symbol-module", 60043),
  symbolPackage: h("symbol-package", 60043),
  symbolNamespace: h("symbol-namespace", 60043),
  symbolObject: h("symbol-object", 60043),
  symbolMethod: h("symbol-method", 60044),
  symbolFunction: h("symbol-function", 60044),
  symbolConstructor: h("symbol-constructor", 60044),
  symbolBoolean: h("symbol-boolean", 60047),
  symbolNull: h("symbol-null", 60047),
  symbolNumeric: h("symbol-numeric", 60048),
  symbolNumber: h("symbol-number", 60048),
  symbolStructure: h("symbol-structure", 60049),
  symbolStruct: h("symbol-struct", 60049),
  symbolParameter: h("symbol-parameter", 60050),
  symbolTypeParameter: h("symbol-type-parameter", 60050),
  symbolKey: h("symbol-key", 60051),
  symbolText: h("symbol-text", 60051),
  symbolReference: h("symbol-reference", 60052),
  goToFile: h("go-to-file", 60052),
  symbolEnum: h("symbol-enum", 60053),
  symbolValue: h("symbol-value", 60053),
  symbolRuler: h("symbol-ruler", 60054),
  symbolUnit: h("symbol-unit", 60054),
  activateBreakpoints: h("activate-breakpoints", 60055),
  archive: h("archive", 60056),
  arrowBoth: h("arrow-both", 60057),
  arrowDown: h("arrow-down", 60058),
  arrowLeft: h("arrow-left", 60059),
  arrowRight: h("arrow-right", 60060),
  arrowSmallDown: h("arrow-small-down", 60061),
  arrowSmallLeft: h("arrow-small-left", 60062),
  arrowSmallRight: h("arrow-small-right", 60063),
  arrowSmallUp: h("arrow-small-up", 60064),
  arrowUp: h("arrow-up", 60065),
  bell: h("bell", 60066),
  bold: h("bold", 60067),
  book: h("book", 60068),
  bookmark: h("bookmark", 60069),
  debugBreakpointConditionalUnverified: h("debug-breakpoint-conditional-unverified", 60070),
  debugBreakpointConditional: h("debug-breakpoint-conditional", 60071),
  debugBreakpointConditionalDisabled: h("debug-breakpoint-conditional-disabled", 60071),
  debugBreakpointDataUnverified: h("debug-breakpoint-data-unverified", 60072),
  debugBreakpointData: h("debug-breakpoint-data", 60073),
  debugBreakpointDataDisabled: h("debug-breakpoint-data-disabled", 60073),
  debugBreakpointLogUnverified: h("debug-breakpoint-log-unverified", 60074),
  debugBreakpointLog: h("debug-breakpoint-log", 60075),
  debugBreakpointLogDisabled: h("debug-breakpoint-log-disabled", 60075),
  briefcase: h("briefcase", 60076),
  broadcast: h("broadcast", 60077),
  browser: h("browser", 60078),
  bug: h("bug", 60079),
  calendar: h("calendar", 60080),
  caseSensitive: h("case-sensitive", 60081),
  check: h("check", 60082),
  checklist: h("checklist", 60083),
  chevronDown: h("chevron-down", 60084),
  dropDownButton: h("drop-down-button", 60084),
  chevronLeft: h("chevron-left", 60085),
  chevronRight: h("chevron-right", 60086),
  chevronUp: h("chevron-up", 60087),
  chromeClose: h("chrome-close", 60088),
  chromeMaximize: h("chrome-maximize", 60089),
  chromeMinimize: h("chrome-minimize", 60090),
  chromeRestore: h("chrome-restore", 60091),
  circle: h("circle", 60092),
  circleOutline: h("circle-outline", 60092),
  debugBreakpointUnverified: h("debug-breakpoint-unverified", 60092),
  circleSlash: h("circle-slash", 60093),
  circuitBoard: h("circuit-board", 60094),
  clearAll: h("clear-all", 60095),
  clippy: h("clippy", 60096),
  closeAll: h("close-all", 60097),
  cloudDownload: h("cloud-download", 60098),
  cloudUpload: h("cloud-upload", 60099),
  code: h("code", 60100),
  collapseAll: h("collapse-all", 60101),
  colorMode: h("color-mode", 60102),
  commentDiscussion: h("comment-discussion", 60103),
  compareChanges: h("compare-changes", 60157),
  creditCard: h("credit-card", 60105),
  dash: h("dash", 60108),
  dashboard: h("dashboard", 60109),
  database: h("database", 60110),
  debugContinue: h("debug-continue", 60111),
  debugDisconnect: h("debug-disconnect", 60112),
  debugPause: h("debug-pause", 60113),
  debugRestart: h("debug-restart", 60114),
  debugStart: h("debug-start", 60115),
  debugStepInto: h("debug-step-into", 60116),
  debugStepOut: h("debug-step-out", 60117),
  debugStepOver: h("debug-step-over", 60118),
  debugStop: h("debug-stop", 60119),
  debug: h("debug", 60120),
  deviceCameraVideo: h("device-camera-video", 60121),
  deviceCamera: h("device-camera", 60122),
  deviceMobile: h("device-mobile", 60123),
  diffAdded: h("diff-added", 60124),
  diffIgnored: h("diff-ignored", 60125),
  diffModified: h("diff-modified", 60126),
  diffRemoved: h("diff-removed", 60127),
  diffRenamed: h("diff-renamed", 60128),
  diff: h("diff", 60129),
  discard: h("discard", 60130),
  editorLayout: h("editor-layout", 60131),
  emptyWindow: h("empty-window", 60132),
  exclude: h("exclude", 60133),
  extensions: h("extensions", 60134),
  eyeClosed: h("eye-closed", 60135),
  fileBinary: h("file-binary", 60136),
  fileCode: h("file-code", 60137),
  fileMedia: h("file-media", 60138),
  filePdf: h("file-pdf", 60139),
  fileSubmodule: h("file-submodule", 60140),
  fileSymlinkDirectory: h("file-symlink-directory", 60141),
  fileSymlinkFile: h("file-symlink-file", 60142),
  fileZip: h("file-zip", 60143),
  files: h("files", 60144),
  filter: h("filter", 60145),
  flame: h("flame", 60146),
  foldDown: h("fold-down", 60147),
  foldUp: h("fold-up", 60148),
  fold: h("fold", 60149),
  folderActive: h("folder-active", 60150),
  folderOpened: h("folder-opened", 60151),
  gear: h("gear", 60152),
  gift: h("gift", 60153),
  gistSecret: h("gist-secret", 60154),
  gist: h("gist", 60155),
  gitCommit: h("git-commit", 60156),
  gitCompare: h("git-compare", 60157),
  gitMerge: h("git-merge", 60158),
  githubAction: h("github-action", 60159),
  githubAlt: h("github-alt", 60160),
  globe: h("globe", 60161),
  grabber: h("grabber", 60162),
  graph: h("graph", 60163),
  gripper: h("gripper", 60164),
  heart: h("heart", 60165),
  home: h("home", 60166),
  horizontalRule: h("horizontal-rule", 60167),
  hubot: h("hubot", 60168),
  inbox: h("inbox", 60169),
  issueClosed: h("issue-closed", 60324),
  issueReopened: h("issue-reopened", 60171),
  issues: h("issues", 60172),
  italic: h("italic", 60173),
  jersey: h("jersey", 60174),
  json: h("json", 60175),
  bracket: h("bracket", 60175),
  kebabVertical: h("kebab-vertical", 60176),
  key: h("key", 60177),
  law: h("law", 60178),
  lightbulbAutofix: h("lightbulb-autofix", 60179),
  linkExternal: h("link-external", 60180),
  link: h("link", 60181),
  listOrdered: h("list-ordered", 60182),
  listUnordered: h("list-unordered", 60183),
  liveShare: h("live-share", 60184),
  loading: h("loading", 60185),
  location: h("location", 60186),
  mailRead: h("mail-read", 60187),
  mail: h("mail", 60188),
  markdown: h("markdown", 60189),
  megaphone: h("megaphone", 60190),
  mention: h("mention", 60191),
  milestone: h("milestone", 60192),
  gitPullRequestMilestone: h("git-pull-request-milestone", 60192),
  mortarBoard: h("mortar-board", 60193),
  move: h("move", 60194),
  multipleWindows: h("multiple-windows", 60195),
  mute: h("mute", 60196),
  noNewline: h("no-newline", 60197),
  note: h("note", 60198),
  octoface: h("octoface", 60199),
  openPreview: h("open-preview", 60200),
  package: h("package", 60201),
  paintcan: h("paintcan", 60202),
  pin: h("pin", 60203),
  play: h("play", 60204),
  run: h("run", 60204),
  plug: h("plug", 60205),
  preserveCase: h("preserve-case", 60206),
  preview: h("preview", 60207),
  project: h("project", 60208),
  pulse: h("pulse", 60209),
  question: h("question", 60210),
  quote: h("quote", 60211),
  radioTower: h("radio-tower", 60212),
  reactions: h("reactions", 60213),
  references: h("references", 60214),
  refresh: h("refresh", 60215),
  regex: h("regex", 60216),
  remoteExplorer: h("remote-explorer", 60217),
  remote: h("remote", 60218),
  remove: h("remove", 60219),
  replaceAll: h("replace-all", 60220),
  replace: h("replace", 60221),
  repoClone: h("repo-clone", 60222),
  repoForcePush: h("repo-force-push", 60223),
  repoPull: h("repo-pull", 60224),
  repoPush: h("repo-push", 60225),
  report: h("report", 60226),
  requestChanges: h("request-changes", 60227),
  rocket: h("rocket", 60228),
  rootFolderOpened: h("root-folder-opened", 60229),
  rootFolder: h("root-folder", 60230),
  rss: h("rss", 60231),
  ruby: h("ruby", 60232),
  saveAll: h("save-all", 60233),
  saveAs: h("save-as", 60234),
  save: h("save", 60235),
  screenFull: h("screen-full", 60236),
  screenNormal: h("screen-normal", 60237),
  searchStop: h("search-stop", 60238),
  server: h("server", 60240),
  settingsGear: h("settings-gear", 60241),
  settings: h("settings", 60242),
  shield: h("shield", 60243),
  smiley: h("smiley", 60244),
  sortPrecedence: h("sort-precedence", 60245),
  splitHorizontal: h("split-horizontal", 60246),
  splitVertical: h("split-vertical", 60247),
  squirrel: h("squirrel", 60248),
  starFull: h("star-full", 60249),
  starHalf: h("star-half", 60250),
  symbolClass: h("symbol-class", 60251),
  symbolColor: h("symbol-color", 60252),
  symbolCustomColor: h("symbol-customcolor", 60252),
  symbolConstant: h("symbol-constant", 60253),
  symbolEnumMember: h("symbol-enum-member", 60254),
  symbolField: h("symbol-field", 60255),
  symbolFile: h("symbol-file", 60256),
  symbolInterface: h("symbol-interface", 60257),
  symbolKeyword: h("symbol-keyword", 60258),
  symbolMisc: h("symbol-misc", 60259),
  symbolOperator: h("symbol-operator", 60260),
  symbolProperty: h("symbol-property", 60261),
  wrench: h("wrench", 60261),
  wrenchSubaction: h("wrench-subaction", 60261),
  symbolSnippet: h("symbol-snippet", 60262),
  tasklist: h("tasklist", 60263),
  telescope: h("telescope", 60264),
  textSize: h("text-size", 60265),
  threeBars: h("three-bars", 60266),
  thumbsdown: h("thumbsdown", 60267),
  thumbsup: h("thumbsup", 60268),
  tools: h("tools", 60269),
  triangleDown: h("triangle-down", 60270),
  triangleLeft: h("triangle-left", 60271),
  triangleRight: h("triangle-right", 60272),
  triangleUp: h("triangle-up", 60273),
  twitter: h("twitter", 60274),
  unfold: h("unfold", 60275),
  unlock: h("unlock", 60276),
  unmute: h("unmute", 60277),
  unverified: h("unverified", 60278),
  verified: h("verified", 60279),
  versions: h("versions", 60280),
  vmActive: h("vm-active", 60281),
  vmOutline: h("vm-outline", 60282),
  vmRunning: h("vm-running", 60283),
  watch: h("watch", 60284),
  whitespace: h("whitespace", 60285),
  wholeWord: h("whole-word", 60286),
  window: h("window", 60287),
  wordWrap: h("word-wrap", 60288),
  zoomIn: h("zoom-in", 60289),
  zoomOut: h("zoom-out", 60290),
  listFilter: h("list-filter", 60291),
  listFlat: h("list-flat", 60292),
  listSelection: h("list-selection", 60293),
  selection: h("selection", 60293),
  listTree: h("list-tree", 60294),
  debugBreakpointFunctionUnverified: h("debug-breakpoint-function-unverified", 60295),
  debugBreakpointFunction: h("debug-breakpoint-function", 60296),
  debugBreakpointFunctionDisabled: h("debug-breakpoint-function-disabled", 60296),
  debugStackframeActive: h("debug-stackframe-active", 60297),
  circleSmallFilled: h("circle-small-filled", 60298),
  debugStackframeDot: h("debug-stackframe-dot", 60298),
  debugStackframe: h("debug-stackframe", 60299),
  debugStackframeFocused: h("debug-stackframe-focused", 60299),
  debugBreakpointUnsupported: h("debug-breakpoint-unsupported", 60300),
  symbolString: h("symbol-string", 60301),
  debugReverseContinue: h("debug-reverse-continue", 60302),
  debugStepBack: h("debug-step-back", 60303),
  debugRestartFrame: h("debug-restart-frame", 60304),
  callIncoming: h("call-incoming", 60306),
  callOutgoing: h("call-outgoing", 60307),
  menu: h("menu", 60308),
  expandAll: h("expand-all", 60309),
  feedback: h("feedback", 60310),
  gitPullRequestReviewer: h("git-pull-request-reviewer", 60310),
  groupByRefType: h("group-by-ref-type", 60311),
  ungroupByRefType: h("ungroup-by-ref-type", 60312),
  account: h("account", 60313),
  gitPullRequestAssignee: h("git-pull-request-assignee", 60313),
  bellDot: h("bell-dot", 60314),
  debugConsole: h("debug-console", 60315),
  library: h("library", 60316),
  output: h("output", 60317),
  runAll: h("run-all", 60318),
  syncIgnored: h("sync-ignored", 60319),
  pinned: h("pinned", 60320),
  githubInverted: h("github-inverted", 60321),
  debugAlt: h("debug-alt", 60305),
  serverProcess: h("server-process", 60322),
  serverEnvironment: h("server-environment", 60323),
  pass: h("pass", 60324),
  stopCircle: h("stop-circle", 60325),
  playCircle: h("play-circle", 60326),
  record: h("record", 60327),
  debugAltSmall: h("debug-alt-small", 60328),
  vmConnect: h("vm-connect", 60329),
  cloud: h("cloud", 60330),
  merge: h("merge", 60331),
  exportIcon: h("export", 60332),
  graphLeft: h("graph-left", 60333),
  magnet: h("magnet", 60334),
  notebook: h("notebook", 60335),
  redo: h("redo", 60336),
  checkAll: h("check-all", 60337),
  pinnedDirty: h("pinned-dirty", 60338),
  passFilled: h("pass-filled", 60339),
  circleLargeFilled: h("circle-large-filled", 60340),
  circleLarge: h("circle-large", 60341),
  circleLargeOutline: h("circle-large-outline", 60341),
  combine: h("combine", 60342),
  gather: h("gather", 60342),
  table: h("table", 60343),
  variableGroup: h("variable-group", 60344),
  typeHierarchy: h("type-hierarchy", 60345),
  typeHierarchySub: h("type-hierarchy-sub", 60346),
  typeHierarchySuper: h("type-hierarchy-super", 60347),
  gitPullRequestCreate: h("git-pull-request-create", 60348),
  runAbove: h("run-above", 60349),
  runBelow: h("run-below", 60350),
  notebookTemplate: h("notebook-template", 60351),
  debugRerun: h("debug-rerun", 60352),
  workspaceTrusted: h("workspace-trusted", 60353),
  workspaceUntrusted: h("workspace-untrusted", 60354),
  workspaceUnspecified: h("workspace-unspecified", 60355),
  terminalCmd: h("terminal-cmd", 60356),
  terminalDebian: h("terminal-debian", 60357),
  terminalLinux: h("terminal-linux", 60358),
  terminalPowershell: h("terminal-powershell", 60359),
  terminalTmux: h("terminal-tmux", 60360),
  terminalUbuntu: h("terminal-ubuntu", 60361),
  terminalBash: h("terminal-bash", 60362),
  arrowSwap: h("arrow-swap", 60363),
  copy: h("copy", 60364),
  personAdd: h("person-add", 60365),
  filterFilled: h("filter-filled", 60366),
  wand: h("wand", 60367),
  debugLineByLine: h("debug-line-by-line", 60368),
  inspect: h("inspect", 60369),
  layers: h("layers", 60370),
  layersDot: h("layers-dot", 60371),
  layersActive: h("layers-active", 60372),
  compass: h("compass", 60373),
  compassDot: h("compass-dot", 60374),
  compassActive: h("compass-active", 60375),
  azure: h("azure", 60376),
  issueDraft: h("issue-draft", 60377),
  gitPullRequestClosed: h("git-pull-request-closed", 60378),
  gitPullRequestDraft: h("git-pull-request-draft", 60379),
  debugAll: h("debug-all", 60380),
  debugCoverage: h("debug-coverage", 60381),
  runErrors: h("run-errors", 60382),
  folderLibrary: h("folder-library", 60383),
  debugContinueSmall: h("debug-continue-small", 60384),
  beakerStop: h("beaker-stop", 60385),
  graphLine: h("graph-line", 60386),
  graphScatter: h("graph-scatter", 60387),
  pieChart: h("pie-chart", 60388),
  bracketDot: h("bracket-dot", 60389),
  bracketError: h("bracket-error", 60390),
  lockSmall: h("lock-small", 60391),
  azureDevops: h("azure-devops", 60392),
  verifiedFilled: h("verified-filled", 60393),
  newLine: h("newline", 60394),
  layout: h("layout", 60395),
  layoutActivitybarLeft: h("layout-activitybar-left", 60396),
  layoutActivitybarRight: h("layout-activitybar-right", 60397),
  layoutPanelLeft: h("layout-panel-left", 60398),
  layoutPanelCenter: h("layout-panel-center", 60399),
  layoutPanelJustify: h("layout-panel-justify", 60400),
  layoutPanelRight: h("layout-panel-right", 60401),
  layoutPanel: h("layout-panel", 60402),
  layoutSidebarLeft: h("layout-sidebar-left", 60403),
  layoutSidebarRight: h("layout-sidebar-right", 60404),
  layoutStatusbar: h("layout-statusbar", 60405),
  layoutMenubar: h("layout-menubar", 60406),
  layoutCentered: h("layout-centered", 60407),
  layoutSidebarRightOff: h("layout-sidebar-right-off", 60416),
  layoutPanelOff: h("layout-panel-off", 60417),
  layoutSidebarLeftOff: h("layout-sidebar-left-off", 60418),
  target: h("target", 60408),
  indent: h("indent", 60409),
  recordSmall: h("record-small", 60410),
  errorSmall: h("error-small", 60411),
  arrowCircleDown: h("arrow-circle-down", 60412),
  arrowCircleLeft: h("arrow-circle-left", 60413),
  arrowCircleRight: h("arrow-circle-right", 60414),
  arrowCircleUp: h("arrow-circle-up", 60415),
  heartFilled: h("heart-filled", 60420),
  map: h("map", 60421),
  mapFilled: h("map-filled", 60422),
  circleSmall: h("circle-small", 60423),
  bellSlash: h("bell-slash", 60424),
  bellSlashDot: h("bell-slash-dot", 60425),
  commentUnresolved: h("comment-unresolved", 60426),
  gitPullRequestGoToChanges: h("git-pull-request-go-to-changes", 60427),
  gitPullRequestNewChanges: h("git-pull-request-new-changes", 60428),
  searchFuzzy: h("search-fuzzy", 60429),
  commentDraft: h("comment-draft", 60430),
  send: h("send", 60431),
  sparkle: h("sparkle", 60432),
  insert: h("insert", 60433),
  mic: h("mic", 60434),
  // derived icons, that could become separate icons
  dialogError: h("dialog-error", "error"),
  dialogWarning: h("dialog-warning", "warning"),
  dialogInfo: h("dialog-info", "info"),
  dialogClose: h("dialog-close", "close"),
  treeItemExpanded: h("tree-item-expanded", "chevron-down"),
  // collapsed is done with rotation
  treeFilterOnTypeOn: h("tree-filter-on-type-on", "list-filter"),
  treeFilterOnTypeOff: h("tree-filter-on-type-off", "list-selection"),
  treeFilterClear: h("tree-filter-clear", "close"),
  treeItemLoading: h("tree-item-loading", "loading"),
  menuSelection: h("menu-selection", "check"),
  menuSubmenu: h("menu-submenu", "chevron-right"),
  menuBarMore: h("menubar-more", "more"),
  scrollbarButtonLeft: h("scrollbar-button-left", "triangle-left"),
  scrollbarButtonRight: h("scrollbar-button-right", "triangle-right"),
  scrollbarButtonUp: h("scrollbar-button-up", "triangle-up"),
  scrollbarButtonDown: h("scrollbar-button-down", "triangle-down"),
  toolBarMore: h("toolbar-more", "more"),
  quickInputBack: h("quick-input-back", "arrow-left")
};
var ar = globalThis && globalThis.__awaiter || function(e, t, n, r) {
  function i(s) {
    return s instanceof n ? s : new n(function(a) {
      a(s);
    });
  }
  return new (n || (n = Promise))(function(s, a) {
    function o(c) {
      try {
        u(r.next(c));
      } catch (f) {
        a(f);
      }
    }
    function l(c) {
      try {
        u(r.throw(c));
      } catch (f) {
        a(f);
      }
    }
    function u(c) {
      c.done ? s(c.value) : i(c.value).then(o, l);
    }
    u((r = r.apply(e, t || [])).next());
  });
};
class el {
  constructor() {
    this._tokenizationSupports = /* @__PURE__ */ new Map(), this._factories = /* @__PURE__ */ new Map(), this._onDidChange = new Ae(), this.onDidChange = this._onDidChange.event, this._colorMap = null;
  }
  handleChange(t) {
    this._onDidChange.fire({
      changedLanguages: t,
      changedColorMap: !1
    });
  }
  register(t, n) {
    return this._tokenizationSupports.set(t, n), this.handleChange([t]), Ot(() => {
      this._tokenizationSupports.get(t) === n && (this._tokenizationSupports.delete(t), this.handleChange([t]));
    });
  }
  get(t) {
    return this._tokenizationSupports.get(t) || null;
  }
  registerFactory(t, n) {
    var r;
    (r = this._factories.get(t)) === null || r === void 0 || r.dispose();
    const i = new tl(this, t, n);
    return this._factories.set(t, i), Ot(() => {
      const s = this._factories.get(t);
      !s || s !== i || (this._factories.delete(t), s.dispose());
    });
  }
  getOrCreate(t) {
    return ar(this, void 0, void 0, function* () {
      const n = this.get(t);
      if (n)
        return n;
      const r = this._factories.get(t);
      return !r || r.isResolved ? null : (yield r.resolve(), this.get(t));
    });
  }
  isResolved(t) {
    if (this.get(t))
      return !0;
    const r = this._factories.get(t);
    return !!(!r || r.isResolved);
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
class tl extends jt {
  get isResolved() {
    return this._isResolved;
  }
  constructor(t, n, r) {
    super(), this._registry = t, this._languageId = n, this._factory = r, this._isDisposed = !1, this._resolvePromise = null, this._isResolved = !1;
  }
  dispose() {
    this._isDisposed = !0, super.dispose();
  }
  resolve() {
    return ar(this, void 0, void 0, function* () {
      return this._resolvePromise || (this._resolvePromise = this._create()), this._resolvePromise;
    });
  }
  _create() {
    return ar(this, void 0, void 0, function* () {
      const t = yield this._factory.tokenizationSupport;
      this._isResolved = !0, t && !this._isDisposed && this._register(this._registry.register(this._languageId, t));
    });
  }
}
class nl {
  constructor(t, n, r) {
    this.offset = t, this.type = n, this.language = r, this._tokenBrand = void 0;
  }
  toString() {
    return "(" + this.offset + ", " + this.type + ")";
  }
}
var Kr;
(function(e) {
  const t = /* @__PURE__ */ new Map();
  t.set(0, q.symbolMethod), t.set(1, q.symbolFunction), t.set(2, q.symbolConstructor), t.set(3, q.symbolField), t.set(4, q.symbolVariable), t.set(5, q.symbolClass), t.set(6, q.symbolStruct), t.set(7, q.symbolInterface), t.set(8, q.symbolModule), t.set(9, q.symbolProperty), t.set(10, q.symbolEvent), t.set(11, q.symbolOperator), t.set(12, q.symbolUnit), t.set(13, q.symbolValue), t.set(15, q.symbolEnum), t.set(14, q.symbolConstant), t.set(15, q.symbolEnum), t.set(16, q.symbolEnumMember), t.set(17, q.symbolKeyword), t.set(27, q.symbolSnippet), t.set(18, q.symbolText), t.set(19, q.symbolColor), t.set(20, q.symbolFile), t.set(21, q.symbolReference), t.set(22, q.symbolCustomColor), t.set(23, q.symbolFolder), t.set(24, q.symbolTypeParameter), t.set(25, q.account), t.set(26, q.issues);
  function n(s) {
    let a = t.get(s);
    return a || (console.info("No codicon found for CompletionItemKind " + s), a = q.symbolProperty), a;
  }
  e.toIcon = n;
  const r = /* @__PURE__ */ new Map();
  r.set(
    "method",
    0
    /* CompletionItemKind.Method */
  ), r.set(
    "function",
    1
    /* CompletionItemKind.Function */
  ), r.set(
    "constructor",
    2
    /* CompletionItemKind.Constructor */
  ), r.set(
    "field",
    3
    /* CompletionItemKind.Field */
  ), r.set(
    "variable",
    4
    /* CompletionItemKind.Variable */
  ), r.set(
    "class",
    5
    /* CompletionItemKind.Class */
  ), r.set(
    "struct",
    6
    /* CompletionItemKind.Struct */
  ), r.set(
    "interface",
    7
    /* CompletionItemKind.Interface */
  ), r.set(
    "module",
    8
    /* CompletionItemKind.Module */
  ), r.set(
    "property",
    9
    /* CompletionItemKind.Property */
  ), r.set(
    "event",
    10
    /* CompletionItemKind.Event */
  ), r.set(
    "operator",
    11
    /* CompletionItemKind.Operator */
  ), r.set(
    "unit",
    12
    /* CompletionItemKind.Unit */
  ), r.set(
    "value",
    13
    /* CompletionItemKind.Value */
  ), r.set(
    "constant",
    14
    /* CompletionItemKind.Constant */
  ), r.set(
    "enum",
    15
    /* CompletionItemKind.Enum */
  ), r.set(
    "enum-member",
    16
    /* CompletionItemKind.EnumMember */
  ), r.set(
    "enumMember",
    16
    /* CompletionItemKind.EnumMember */
  ), r.set(
    "keyword",
    17
    /* CompletionItemKind.Keyword */
  ), r.set(
    "snippet",
    27
    /* CompletionItemKind.Snippet */
  ), r.set(
    "text",
    18
    /* CompletionItemKind.Text */
  ), r.set(
    "color",
    19
    /* CompletionItemKind.Color */
  ), r.set(
    "file",
    20
    /* CompletionItemKind.File */
  ), r.set(
    "reference",
    21
    /* CompletionItemKind.Reference */
  ), r.set(
    "customcolor",
    22
    /* CompletionItemKind.Customcolor */
  ), r.set(
    "folder",
    23
    /* CompletionItemKind.Folder */
  ), r.set(
    "type-parameter",
    24
    /* CompletionItemKind.TypeParameter */
  ), r.set(
    "typeParameter",
    24
    /* CompletionItemKind.TypeParameter */
  ), r.set(
    "account",
    25
    /* CompletionItemKind.User */
  ), r.set(
    "issue",
    26
    /* CompletionItemKind.Issue */
  );
  function i(s, a) {
    let o = r.get(s);
    return typeof o > "u" && !a && (o = 9), o;
  }
  e.fromString = i;
})(Kr || (Kr = {}));
var ei;
(function(e) {
  e[e.Automatic = 0] = "Automatic", e[e.Explicit = 1] = "Explicit";
})(ei || (ei = {}));
var ti;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.TriggerCharacter = 2] = "TriggerCharacter", e[e.ContentChange = 3] = "ContentChange";
})(ti || (ti = {}));
var ni;
(function(e) {
  e[e.Text = 0] = "Text", e[e.Read = 1] = "Read", e[e.Write = 2] = "Write";
})(ni || (ni = {}));
Y("Array", "array"), Y("Boolean", "boolean"), Y("Class", "class"), Y("Constant", "constant"), Y("Constructor", "constructor"), Y("Enum", "enumeration"), Y("EnumMember", "enumeration member"), Y("Event", "event"), Y("Field", "field"), Y("File", "file"), Y("Function", "function"), Y("Interface", "interface"), Y("Key", "key"), Y("Method", "method"), Y("Module", "module"), Y("Namespace", "namespace"), Y("Null", "null"), Y("Number", "number"), Y("Object", "object"), Y("Operator", "operator"), Y("Package", "package"), Y("Property", "property"), Y("String", "string"), Y("Struct", "struct"), Y("TypeParameter", "type parameter"), Y("Variable", "variable");
var ri;
(function(e) {
  const t = /* @__PURE__ */ new Map();
  t.set(0, q.symbolFile), t.set(1, q.symbolModule), t.set(2, q.symbolNamespace), t.set(3, q.symbolPackage), t.set(4, q.symbolClass), t.set(5, q.symbolMethod), t.set(6, q.symbolProperty), t.set(7, q.symbolField), t.set(8, q.symbolConstructor), t.set(9, q.symbolEnum), t.set(10, q.symbolInterface), t.set(11, q.symbolFunction), t.set(12, q.symbolVariable), t.set(13, q.symbolConstant), t.set(14, q.symbolString), t.set(15, q.symbolNumber), t.set(16, q.symbolBoolean), t.set(17, q.symbolArray), t.set(18, q.symbolObject), t.set(19, q.symbolKey), t.set(20, q.symbolNull), t.set(21, q.symbolEnumMember), t.set(22, q.symbolStruct), t.set(23, q.symbolEvent), t.set(24, q.symbolOperator), t.set(25, q.symbolTypeParameter);
  function n(r) {
    let i = t.get(r);
    return i || (console.info("No codicon found for SymbolKind " + r), i = q.symbolProperty), i;
  }
  e.toIcon = n;
})(ri || (ri = {}));
var ii;
(function(e) {
  function t(n) {
    return !n || typeof n != "object" ? !1 : typeof n.id == "string" && typeof n.title == "string";
  }
  e.is = t;
})(ii || (ii = {}));
var si;
(function(e) {
  e[e.Type = 1] = "Type", e[e.Parameter = 2] = "Parameter";
})(si || (si = {}));
new el();
var ai;
(function(e) {
  e[e.Unknown = 0] = "Unknown", e[e.Disabled = 1] = "Disabled", e[e.Enabled = 2] = "Enabled";
})(ai || (ai = {}));
var oi;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.Auto = 2] = "Auto";
})(oi || (oi = {}));
var li;
(function(e) {
  e[e.None = 0] = "None", e[e.KeepWhitespace = 1] = "KeepWhitespace", e[e.InsertAsSnippet = 4] = "InsertAsSnippet";
})(li || (li = {}));
var ui;
(function(e) {
  e[e.Method = 0] = "Method", e[e.Function = 1] = "Function", e[e.Constructor = 2] = "Constructor", e[e.Field = 3] = "Field", e[e.Variable = 4] = "Variable", e[e.Class = 5] = "Class", e[e.Struct = 6] = "Struct", e[e.Interface = 7] = "Interface", e[e.Module = 8] = "Module", e[e.Property = 9] = "Property", e[e.Event = 10] = "Event", e[e.Operator = 11] = "Operator", e[e.Unit = 12] = "Unit", e[e.Value = 13] = "Value", e[e.Constant = 14] = "Constant", e[e.Enum = 15] = "Enum", e[e.EnumMember = 16] = "EnumMember", e[e.Keyword = 17] = "Keyword", e[e.Text = 18] = "Text", e[e.Color = 19] = "Color", e[e.File = 20] = "File", e[e.Reference = 21] = "Reference", e[e.Customcolor = 22] = "Customcolor", e[e.Folder = 23] = "Folder", e[e.TypeParameter = 24] = "TypeParameter", e[e.User = 25] = "User", e[e.Issue = 26] = "Issue", e[e.Snippet = 27] = "Snippet";
})(ui || (ui = {}));
var ci;
(function(e) {
  e[e.Deprecated = 1] = "Deprecated";
})(ci || (ci = {}));
var fi;
(function(e) {
  e[e.Invoke = 0] = "Invoke", e[e.TriggerCharacter = 1] = "TriggerCharacter", e[e.TriggerForIncompleteCompletions = 2] = "TriggerForIncompleteCompletions";
})(fi || (fi = {}));
var hi;
(function(e) {
  e[e.EXACT = 0] = "EXACT", e[e.ABOVE = 1] = "ABOVE", e[e.BELOW = 2] = "BELOW";
})(hi || (hi = {}));
var di;
(function(e) {
  e[e.NotSet = 0] = "NotSet", e[e.ContentFlush = 1] = "ContentFlush", e[e.RecoverFromMarkers = 2] = "RecoverFromMarkers", e[e.Explicit = 3] = "Explicit", e[e.Paste = 4] = "Paste", e[e.Undo = 5] = "Undo", e[e.Redo = 6] = "Redo";
})(di || (di = {}));
var gi;
(function(e) {
  e[e.LF = 1] = "LF", e[e.CRLF = 2] = "CRLF";
})(gi || (gi = {}));
var mi;
(function(e) {
  e[e.Text = 0] = "Text", e[e.Read = 1] = "Read", e[e.Write = 2] = "Write";
})(mi || (mi = {}));
var pi;
(function(e) {
  e[e.None = 0] = "None", e[e.Keep = 1] = "Keep", e[e.Brackets = 2] = "Brackets", e[e.Advanced = 3] = "Advanced", e[e.Full = 4] = "Full";
})(pi || (pi = {}));
var vi;
(function(e) {
  e[e.acceptSuggestionOnCommitCharacter = 0] = "acceptSuggestionOnCommitCharacter", e[e.acceptSuggestionOnEnter = 1] = "acceptSuggestionOnEnter", e[e.accessibilitySupport = 2] = "accessibilitySupport", e[e.accessibilityPageSize = 3] = "accessibilityPageSize", e[e.ariaLabel = 4] = "ariaLabel", e[e.ariaRequired = 5] = "ariaRequired", e[e.autoClosingBrackets = 6] = "autoClosingBrackets", e[e.autoClosingComments = 7] = "autoClosingComments", e[e.screenReaderAnnounceInlineSuggestion = 8] = "screenReaderAnnounceInlineSuggestion", e[e.autoClosingDelete = 9] = "autoClosingDelete", e[e.autoClosingOvertype = 10] = "autoClosingOvertype", e[e.autoClosingQuotes = 11] = "autoClosingQuotes", e[e.autoIndent = 12] = "autoIndent", e[e.automaticLayout = 13] = "automaticLayout", e[e.autoSurround = 14] = "autoSurround", e[e.bracketPairColorization = 15] = "bracketPairColorization", e[e.guides = 16] = "guides", e[e.codeLens = 17] = "codeLens", e[e.codeLensFontFamily = 18] = "codeLensFontFamily", e[e.codeLensFontSize = 19] = "codeLensFontSize", e[e.colorDecorators = 20] = "colorDecorators", e[e.colorDecoratorsLimit = 21] = "colorDecoratorsLimit", e[e.columnSelection = 22] = "columnSelection", e[e.comments = 23] = "comments", e[e.contextmenu = 24] = "contextmenu", e[e.copyWithSyntaxHighlighting = 25] = "copyWithSyntaxHighlighting", e[e.cursorBlinking = 26] = "cursorBlinking", e[e.cursorSmoothCaretAnimation = 27] = "cursorSmoothCaretAnimation", e[e.cursorStyle = 28] = "cursorStyle", e[e.cursorSurroundingLines = 29] = "cursorSurroundingLines", e[e.cursorSurroundingLinesStyle = 30] = "cursorSurroundingLinesStyle", e[e.cursorWidth = 31] = "cursorWidth", e[e.disableLayerHinting = 32] = "disableLayerHinting", e[e.disableMonospaceOptimizations = 33] = "disableMonospaceOptimizations", e[e.domReadOnly = 34] = "domReadOnly", e[e.dragAndDrop = 35] = "dragAndDrop", e[e.dropIntoEditor = 36] = "dropIntoEditor", e[e.emptySelectionClipboard = 37] = "emptySelectionClipboard", e[e.experimentalWhitespaceRendering = 38] = "experimentalWhitespaceRendering", e[e.extraEditorClassName = 39] = "extraEditorClassName", e[e.fastScrollSensitivity = 40] = "fastScrollSensitivity", e[e.find = 41] = "find", e[e.fixedOverflowWidgets = 42] = "fixedOverflowWidgets", e[e.folding = 43] = "folding", e[e.foldingStrategy = 44] = "foldingStrategy", e[e.foldingHighlight = 45] = "foldingHighlight", e[e.foldingImportsByDefault = 46] = "foldingImportsByDefault", e[e.foldingMaximumRegions = 47] = "foldingMaximumRegions", e[e.unfoldOnClickAfterEndOfLine = 48] = "unfoldOnClickAfterEndOfLine", e[e.fontFamily = 49] = "fontFamily", e[e.fontInfo = 50] = "fontInfo", e[e.fontLigatures = 51] = "fontLigatures", e[e.fontSize = 52] = "fontSize", e[e.fontWeight = 53] = "fontWeight", e[e.fontVariations = 54] = "fontVariations", e[e.formatOnPaste = 55] = "formatOnPaste", e[e.formatOnType = 56] = "formatOnType", e[e.glyphMargin = 57] = "glyphMargin", e[e.gotoLocation = 58] = "gotoLocation", e[e.hideCursorInOverviewRuler = 59] = "hideCursorInOverviewRuler", e[e.hover = 60] = "hover", e[e.inDiffEditor = 61] = "inDiffEditor", e[e.inlineSuggest = 62] = "inlineSuggest", e[e.letterSpacing = 63] = "letterSpacing", e[e.lightbulb = 64] = "lightbulb", e[e.lineDecorationsWidth = 65] = "lineDecorationsWidth", e[e.lineHeight = 66] = "lineHeight", e[e.lineNumbers = 67] = "lineNumbers", e[e.lineNumbersMinChars = 68] = "lineNumbersMinChars", e[e.linkedEditing = 69] = "linkedEditing", e[e.links = 70] = "links", e[e.matchBrackets = 71] = "matchBrackets", e[e.minimap = 72] = "minimap", e[e.mouseStyle = 73] = "mouseStyle", e[e.mouseWheelScrollSensitivity = 74] = "mouseWheelScrollSensitivity", e[e.mouseWheelZoom = 75] = "mouseWheelZoom", e[e.multiCursorMergeOverlapping = 76] = "multiCursorMergeOverlapping", e[e.multiCursorModifier = 77] = "multiCursorModifier", e[e.multiCursorPaste = 78] = "multiCursorPaste", e[e.multiCursorLimit = 79] = "multiCursorLimit", e[e.occurrencesHighlight = 80] = "occurrencesHighlight", e[e.overviewRulerBorder = 81] = "overviewRulerBorder", e[e.overviewRulerLanes = 82] = "overviewRulerLanes", e[e.padding = 83] = "padding", e[e.pasteAs = 84] = "pasteAs", e[e.parameterHints = 85] = "parameterHints", e[e.peekWidgetDefaultFocus = 86] = "peekWidgetDefaultFocus", e[e.definitionLinkOpensInPeek = 87] = "definitionLinkOpensInPeek", e[e.quickSuggestions = 88] = "quickSuggestions", e[e.quickSuggestionsDelay = 89] = "quickSuggestionsDelay", e[e.readOnly = 90] = "readOnly", e[e.readOnlyMessage = 91] = "readOnlyMessage", e[e.renameOnType = 92] = "renameOnType", e[e.renderControlCharacters = 93] = "renderControlCharacters", e[e.renderFinalNewline = 94] = "renderFinalNewline", e[e.renderLineHighlight = 95] = "renderLineHighlight", e[e.renderLineHighlightOnlyWhenFocus = 96] = "renderLineHighlightOnlyWhenFocus", e[e.renderValidationDecorations = 97] = "renderValidationDecorations", e[e.renderWhitespace = 98] = "renderWhitespace", e[e.revealHorizontalRightPadding = 99] = "revealHorizontalRightPadding", e[e.roundedSelection = 100] = "roundedSelection", e[e.rulers = 101] = "rulers", e[e.scrollbar = 102] = "scrollbar", e[e.scrollBeyondLastColumn = 103] = "scrollBeyondLastColumn", e[e.scrollBeyondLastLine = 104] = "scrollBeyondLastLine", e[e.scrollPredominantAxis = 105] = "scrollPredominantAxis", e[e.selectionClipboard = 106] = "selectionClipboard", e[e.selectionHighlight = 107] = "selectionHighlight", e[e.selectOnLineNumbers = 108] = "selectOnLineNumbers", e[e.showFoldingControls = 109] = "showFoldingControls", e[e.showUnused = 110] = "showUnused", e[e.snippetSuggestions = 111] = "snippetSuggestions", e[e.smartSelect = 112] = "smartSelect", e[e.smoothScrolling = 113] = "smoothScrolling", e[e.stickyScroll = 114] = "stickyScroll", e[e.stickyTabStops = 115] = "stickyTabStops", e[e.stopRenderingLineAfter = 116] = "stopRenderingLineAfter", e[e.suggest = 117] = "suggest", e[e.suggestFontSize = 118] = "suggestFontSize", e[e.suggestLineHeight = 119] = "suggestLineHeight", e[e.suggestOnTriggerCharacters = 120] = "suggestOnTriggerCharacters", e[e.suggestSelection = 121] = "suggestSelection", e[e.tabCompletion = 122] = "tabCompletion", e[e.tabIndex = 123] = "tabIndex", e[e.unicodeHighlighting = 124] = "unicodeHighlighting", e[e.unusualLineTerminators = 125] = "unusualLineTerminators", e[e.useShadowDOM = 126] = "useShadowDOM", e[e.useTabStops = 127] = "useTabStops", e[e.wordBreak = 128] = "wordBreak", e[e.wordSeparators = 129] = "wordSeparators", e[e.wordWrap = 130] = "wordWrap", e[e.wordWrapBreakAfterCharacters = 131] = "wordWrapBreakAfterCharacters", e[e.wordWrapBreakBeforeCharacters = 132] = "wordWrapBreakBeforeCharacters", e[e.wordWrapColumn = 133] = "wordWrapColumn", e[e.wordWrapOverride1 = 134] = "wordWrapOverride1", e[e.wordWrapOverride2 = 135] = "wordWrapOverride2", e[e.wrappingIndent = 136] = "wrappingIndent", e[e.wrappingStrategy = 137] = "wrappingStrategy", e[e.showDeprecated = 138] = "showDeprecated", e[e.inlayHints = 139] = "inlayHints", e[e.editorClassName = 140] = "editorClassName", e[e.pixelRatio = 141] = "pixelRatio", e[e.tabFocusMode = 142] = "tabFocusMode", e[e.layoutInfo = 143] = "layoutInfo", e[e.wrappingInfo = 144] = "wrappingInfo", e[e.defaultColorDecorators = 145] = "defaultColorDecorators", e[e.colorDecoratorsActivatedOn = 146] = "colorDecoratorsActivatedOn", e[e.inlineCompletionsAccessibilityVerbose = 147] = "inlineCompletionsAccessibilityVerbose";
})(vi || (vi = {}));
var bi;
(function(e) {
  e[e.TextDefined = 0] = "TextDefined", e[e.LF = 1] = "LF", e[e.CRLF = 2] = "CRLF";
})(bi || (bi = {}));
var yi;
(function(e) {
  e[e.LF = 0] = "LF", e[e.CRLF = 1] = "CRLF";
})(yi || (yi = {}));
var xi;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Right = 2] = "Right";
})(xi || (xi = {}));
var wi;
(function(e) {
  e[e.None = 0] = "None", e[e.Indent = 1] = "Indent", e[e.IndentOutdent = 2] = "IndentOutdent", e[e.Outdent = 3] = "Outdent";
})(wi || (wi = {}));
var _i;
(function(e) {
  e[e.Both = 0] = "Both", e[e.Right = 1] = "Right", e[e.Left = 2] = "Left", e[e.None = 3] = "None";
})(_i || (_i = {}));
var Si;
(function(e) {
  e[e.Type = 1] = "Type", e[e.Parameter = 2] = "Parameter";
})(Si || (Si = {}));
var Li;
(function(e) {
  e[e.Automatic = 0] = "Automatic", e[e.Explicit = 1] = "Explicit";
})(Li || (Li = {}));
var or;
(function(e) {
  e[e.DependsOnKbLayout = -1] = "DependsOnKbLayout", e[e.Unknown = 0] = "Unknown", e[e.Backspace = 1] = "Backspace", e[e.Tab = 2] = "Tab", e[e.Enter = 3] = "Enter", e[e.Shift = 4] = "Shift", e[e.Ctrl = 5] = "Ctrl", e[e.Alt = 6] = "Alt", e[e.PauseBreak = 7] = "PauseBreak", e[e.CapsLock = 8] = "CapsLock", e[e.Escape = 9] = "Escape", e[e.Space = 10] = "Space", e[e.PageUp = 11] = "PageUp", e[e.PageDown = 12] = "PageDown", e[e.End = 13] = "End", e[e.Home = 14] = "Home", e[e.LeftArrow = 15] = "LeftArrow", e[e.UpArrow = 16] = "UpArrow", e[e.RightArrow = 17] = "RightArrow", e[e.DownArrow = 18] = "DownArrow", e[e.Insert = 19] = "Insert", e[e.Delete = 20] = "Delete", e[e.Digit0 = 21] = "Digit0", e[e.Digit1 = 22] = "Digit1", e[e.Digit2 = 23] = "Digit2", e[e.Digit3 = 24] = "Digit3", e[e.Digit4 = 25] = "Digit4", e[e.Digit5 = 26] = "Digit5", e[e.Digit6 = 27] = "Digit6", e[e.Digit7 = 28] = "Digit7", e[e.Digit8 = 29] = "Digit8", e[e.Digit9 = 30] = "Digit9", e[e.KeyA = 31] = "KeyA", e[e.KeyB = 32] = "KeyB", e[e.KeyC = 33] = "KeyC", e[e.KeyD = 34] = "KeyD", e[e.KeyE = 35] = "KeyE", e[e.KeyF = 36] = "KeyF", e[e.KeyG = 37] = "KeyG", e[e.KeyH = 38] = "KeyH", e[e.KeyI = 39] = "KeyI", e[e.KeyJ = 40] = "KeyJ", e[e.KeyK = 41] = "KeyK", e[e.KeyL = 42] = "KeyL", e[e.KeyM = 43] = "KeyM", e[e.KeyN = 44] = "KeyN", e[e.KeyO = 45] = "KeyO", e[e.KeyP = 46] = "KeyP", e[e.KeyQ = 47] = "KeyQ", e[e.KeyR = 48] = "KeyR", e[e.KeyS = 49] = "KeyS", e[e.KeyT = 50] = "KeyT", e[e.KeyU = 51] = "KeyU", e[e.KeyV = 52] = "KeyV", e[e.KeyW = 53] = "KeyW", e[e.KeyX = 54] = "KeyX", e[e.KeyY = 55] = "KeyY", e[e.KeyZ = 56] = "KeyZ", e[e.Meta = 57] = "Meta", e[e.ContextMenu = 58] = "ContextMenu", e[e.F1 = 59] = "F1", e[e.F2 = 60] = "F2", e[e.F3 = 61] = "F3", e[e.F4 = 62] = "F4", e[e.F5 = 63] = "F5", e[e.F6 = 64] = "F6", e[e.F7 = 65] = "F7", e[e.F8 = 66] = "F8", e[e.F9 = 67] = "F9", e[e.F10 = 68] = "F10", e[e.F11 = 69] = "F11", e[e.F12 = 70] = "F12", e[e.F13 = 71] = "F13", e[e.F14 = 72] = "F14", e[e.F15 = 73] = "F15", e[e.F16 = 74] = "F16", e[e.F17 = 75] = "F17", e[e.F18 = 76] = "F18", e[e.F19 = 77] = "F19", e[e.F20 = 78] = "F20", e[e.F21 = 79] = "F21", e[e.F22 = 80] = "F22", e[e.F23 = 81] = "F23", e[e.F24 = 82] = "F24", e[e.NumLock = 83] = "NumLock", e[e.ScrollLock = 84] = "ScrollLock", e[e.Semicolon = 85] = "Semicolon", e[e.Equal = 86] = "Equal", e[e.Comma = 87] = "Comma", e[e.Minus = 88] = "Minus", e[e.Period = 89] = "Period", e[e.Slash = 90] = "Slash", e[e.Backquote = 91] = "Backquote", e[e.BracketLeft = 92] = "BracketLeft", e[e.Backslash = 93] = "Backslash", e[e.BracketRight = 94] = "BracketRight", e[e.Quote = 95] = "Quote", e[e.OEM_8 = 96] = "OEM_8", e[e.IntlBackslash = 97] = "IntlBackslash", e[e.Numpad0 = 98] = "Numpad0", e[e.Numpad1 = 99] = "Numpad1", e[e.Numpad2 = 100] = "Numpad2", e[e.Numpad3 = 101] = "Numpad3", e[e.Numpad4 = 102] = "Numpad4", e[e.Numpad5 = 103] = "Numpad5", e[e.Numpad6 = 104] = "Numpad6", e[e.Numpad7 = 105] = "Numpad7", e[e.Numpad8 = 106] = "Numpad8", e[e.Numpad9 = 107] = "Numpad9", e[e.NumpadMultiply = 108] = "NumpadMultiply", e[e.NumpadAdd = 109] = "NumpadAdd", e[e.NUMPAD_SEPARATOR = 110] = "NUMPAD_SEPARATOR", e[e.NumpadSubtract = 111] = "NumpadSubtract", e[e.NumpadDecimal = 112] = "NumpadDecimal", e[e.NumpadDivide = 113] = "NumpadDivide", e[e.KEY_IN_COMPOSITION = 114] = "KEY_IN_COMPOSITION", e[e.ABNT_C1 = 115] = "ABNT_C1", e[e.ABNT_C2 = 116] = "ABNT_C2", e[e.AudioVolumeMute = 117] = "AudioVolumeMute", e[e.AudioVolumeUp = 118] = "AudioVolumeUp", e[e.AudioVolumeDown = 119] = "AudioVolumeDown", e[e.BrowserSearch = 120] = "BrowserSearch", e[e.BrowserHome = 121] = "BrowserHome", e[e.BrowserBack = 122] = "BrowserBack", e[e.BrowserForward = 123] = "BrowserForward", e[e.MediaTrackNext = 124] = "MediaTrackNext", e[e.MediaTrackPrevious = 125] = "MediaTrackPrevious", e[e.MediaStop = 126] = "MediaStop", e[e.MediaPlayPause = 127] = "MediaPlayPause", e[e.LaunchMediaPlayer = 128] = "LaunchMediaPlayer", e[e.LaunchMail = 129] = "LaunchMail", e[e.LaunchApp2 = 130] = "LaunchApp2", e[e.Clear = 131] = "Clear", e[e.MAX_VALUE = 132] = "MAX_VALUE";
})(or || (or = {}));
var lr;
(function(e) {
  e[e.Hint = 1] = "Hint", e[e.Info = 2] = "Info", e[e.Warning = 4] = "Warning", e[e.Error = 8] = "Error";
})(lr || (lr = {}));
var ur;
(function(e) {
  e[e.Unnecessary = 1] = "Unnecessary", e[e.Deprecated = 2] = "Deprecated";
})(ur || (ur = {}));
var Ni;
(function(e) {
  e[e.Inline = 1] = "Inline", e[e.Gutter = 2] = "Gutter";
})(Ni || (Ni = {}));
var Ai;
(function(e) {
  e[e.UNKNOWN = 0] = "UNKNOWN", e[e.TEXTAREA = 1] = "TEXTAREA", e[e.GUTTER_GLYPH_MARGIN = 2] = "GUTTER_GLYPH_MARGIN", e[e.GUTTER_LINE_NUMBERS = 3] = "GUTTER_LINE_NUMBERS", e[e.GUTTER_LINE_DECORATIONS = 4] = "GUTTER_LINE_DECORATIONS", e[e.GUTTER_VIEW_ZONE = 5] = "GUTTER_VIEW_ZONE", e[e.CONTENT_TEXT = 6] = "CONTENT_TEXT", e[e.CONTENT_EMPTY = 7] = "CONTENT_EMPTY", e[e.CONTENT_VIEW_ZONE = 8] = "CONTENT_VIEW_ZONE", e[e.CONTENT_WIDGET = 9] = "CONTENT_WIDGET", e[e.OVERVIEW_RULER = 10] = "OVERVIEW_RULER", e[e.SCROLLBAR = 11] = "SCROLLBAR", e[e.OVERLAY_WIDGET = 12] = "OVERLAY_WIDGET", e[e.OUTSIDE_EDITOR = 13] = "OUTSIDE_EDITOR";
})(Ai || (Ai = {}));
var Ci;
(function(e) {
  e[e.TOP_RIGHT_CORNER = 0] = "TOP_RIGHT_CORNER", e[e.BOTTOM_RIGHT_CORNER = 1] = "BOTTOM_RIGHT_CORNER", e[e.TOP_CENTER = 2] = "TOP_CENTER";
})(Ci || (Ci = {}));
var ki;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Center = 2] = "Center", e[e.Right = 4] = "Right", e[e.Full = 7] = "Full";
})(ki || (ki = {}));
var Ei;
(function(e) {
  e[e.Left = 0] = "Left", e[e.Right = 1] = "Right", e[e.None = 2] = "None", e[e.LeftOfInjectedText = 3] = "LeftOfInjectedText", e[e.RightOfInjectedText = 4] = "RightOfInjectedText";
})(Ei || (Ei = {}));
var Mi;
(function(e) {
  e[e.Off = 0] = "Off", e[e.On = 1] = "On", e[e.Relative = 2] = "Relative", e[e.Interval = 3] = "Interval", e[e.Custom = 4] = "Custom";
})(Mi || (Mi = {}));
var Ri;
(function(e) {
  e[e.None = 0] = "None", e[e.Text = 1] = "Text", e[e.Blocks = 2] = "Blocks";
})(Ri || (Ri = {}));
var Ti;
(function(e) {
  e[e.Smooth = 0] = "Smooth", e[e.Immediate = 1] = "Immediate";
})(Ti || (Ti = {}));
var Pi;
(function(e) {
  e[e.Auto = 1] = "Auto", e[e.Hidden = 2] = "Hidden", e[e.Visible = 3] = "Visible";
})(Pi || (Pi = {}));
var cr;
(function(e) {
  e[e.LTR = 0] = "LTR", e[e.RTL = 1] = "RTL";
})(cr || (cr = {}));
var Fi;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.TriggerCharacter = 2] = "TriggerCharacter", e[e.ContentChange = 3] = "ContentChange";
})(Fi || (Fi = {}));
var Ii;
(function(e) {
  e[e.File = 0] = "File", e[e.Module = 1] = "Module", e[e.Namespace = 2] = "Namespace", e[e.Package = 3] = "Package", e[e.Class = 4] = "Class", e[e.Method = 5] = "Method", e[e.Property = 6] = "Property", e[e.Field = 7] = "Field", e[e.Constructor = 8] = "Constructor", e[e.Enum = 9] = "Enum", e[e.Interface = 10] = "Interface", e[e.Function = 11] = "Function", e[e.Variable = 12] = "Variable", e[e.Constant = 13] = "Constant", e[e.String = 14] = "String", e[e.Number = 15] = "Number", e[e.Boolean = 16] = "Boolean", e[e.Array = 17] = "Array", e[e.Object = 18] = "Object", e[e.Key = 19] = "Key", e[e.Null = 20] = "Null", e[e.EnumMember = 21] = "EnumMember", e[e.Struct = 22] = "Struct", e[e.Event = 23] = "Event", e[e.Operator = 24] = "Operator", e[e.TypeParameter = 25] = "TypeParameter";
})(Ii || (Ii = {}));
var Di;
(function(e) {
  e[e.Deprecated = 1] = "Deprecated";
})(Di || (Di = {}));
var Vi;
(function(e) {
  e[e.Hidden = 0] = "Hidden", e[e.Blink = 1] = "Blink", e[e.Smooth = 2] = "Smooth", e[e.Phase = 3] = "Phase", e[e.Expand = 4] = "Expand", e[e.Solid = 5] = "Solid";
})(Vi || (Vi = {}));
var Oi;
(function(e) {
  e[e.Line = 1] = "Line", e[e.Block = 2] = "Block", e[e.Underline = 3] = "Underline", e[e.LineThin = 4] = "LineThin", e[e.BlockOutline = 5] = "BlockOutline", e[e.UnderlineThin = 6] = "UnderlineThin";
})(Oi || (Oi = {}));
var ji;
(function(e) {
  e[e.AlwaysGrowsWhenTypingAtEdges = 0] = "AlwaysGrowsWhenTypingAtEdges", e[e.NeverGrowsWhenTypingAtEdges = 1] = "NeverGrowsWhenTypingAtEdges", e[e.GrowsOnlyWhenTypingBefore = 2] = "GrowsOnlyWhenTypingBefore", e[e.GrowsOnlyWhenTypingAfter = 3] = "GrowsOnlyWhenTypingAfter";
})(ji || (ji = {}));
var qi;
(function(e) {
  e[e.None = 0] = "None", e[e.Same = 1] = "Same", e[e.Indent = 2] = "Indent", e[e.DeepIndent = 3] = "DeepIndent";
})(qi || (qi = {}));
class Zt {
  static chord(t, n) {
    return Ko(t, n);
  }
}
Zt.CtrlCmd = 2048;
Zt.Shift = 1024;
Zt.Alt = 512;
Zt.WinCtrl = 256;
function rl() {
  return {
    editor: void 0,
    // undefined override expected here
    languages: void 0,
    // undefined override expected here
    CancellationTokenSource: Xo,
    Emitter: Ae,
    KeyCode: or,
    KeyMod: Zt,
    Position: Oe,
    Range: ae,
    Selection: ye,
    SelectionDirection: cr,
    MarkerSeverity: lr,
    MarkerTag: ur,
    Uri: Mr,
    Token: nl
  };
}
var Bi;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Center = 2] = "Center", e[e.Right = 4] = "Right", e[e.Full = 7] = "Full";
})(Bi || (Bi = {}));
var Ui;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Right = 2] = "Right";
})(Ui || (Ui = {}));
var $i;
(function(e) {
  e[e.Inline = 1] = "Inline", e[e.Gutter = 2] = "Gutter";
})($i || ($i = {}));
var Wi;
(function(e) {
  e[e.Both = 0] = "Both", e[e.Right = 1] = "Right", e[e.Left = 2] = "Left", e[e.None = 3] = "None";
})(Wi || (Wi = {}));
function il(e, t, n, r, i) {
  if (r === 0)
    return !0;
  const s = t.charCodeAt(r - 1);
  if (e.get(s) !== 0 || s === 13 || s === 10)
    return !0;
  if (i > 0) {
    const a = t.charCodeAt(r);
    if (e.get(a) !== 0)
      return !0;
  }
  return !1;
}
function sl(e, t, n, r, i) {
  if (r + i === n)
    return !0;
  const s = t.charCodeAt(r + i);
  if (e.get(s) !== 0 || s === 13 || s === 10)
    return !0;
  if (i > 0) {
    const a = t.charCodeAt(r + i - 1);
    if (e.get(a) !== 0)
      return !0;
  }
  return !1;
}
function al(e, t, n, r, i) {
  return il(e, t, n, r, i) && sl(e, t, n, r, i);
}
class ol {
  constructor(t, n) {
    this._wordSeparators = t, this._searchRegex = n, this._prevMatchStartIndex = -1, this._prevMatchLength = 0;
  }
  reset(t) {
    this._searchRegex.lastIndex = t, this._prevMatchStartIndex = -1, this._prevMatchLength = 0;
  }
  next(t) {
    const n = t.length;
    let r;
    do {
      if (this._prevMatchStartIndex + this._prevMatchLength === n || (r = this._searchRegex.exec(t), !r))
        return null;
      const i = r.index, s = r[0].length;
      if (i === this._prevMatchStartIndex && s === this._prevMatchLength) {
        if (s === 0) {
          eo(t, n, this._searchRegex.lastIndex) > 65535 ? this._searchRegex.lastIndex += 2 : this._searchRegex.lastIndex += 1;
          continue;
        }
        return null;
      }
      if (this._prevMatchStartIndex = i, this._prevMatchLength = s, !this._wordSeparators || al(this._wordSeparators, t, n, i, s))
        return r;
    } while (r);
    return null;
  }
}
function ll(e, t = "Unreachable") {
  throw new Error(t);
}
function xn(e) {
  if (!e()) {
    debugger;
    e(), Hs(new Ke("Assertion Failed"));
  }
}
function aa(e, t) {
  let n = 0;
  for (; n < e.length - 1; ) {
    const r = e[n], i = e[n + 1];
    if (!t(r, i))
      return !1;
    n++;
  }
  return !0;
}
class ul {
  static computeUnicodeHighlights(t, n, r) {
    const i = r ? r.startLineNumber : 1, s = r ? r.endLineNumber : t.getLineCount(), a = new Hi(n), o = a.getCandidateCodePoints();
    let l;
    o === "allNonBasicAscii" ? l = new RegExp("[^\\t\\n\\r\\x20-\\x7E]", "g") : l = new RegExp(`${cl(Array.from(o))}`, "g");
    const u = new ol(null, l), c = [];
    let f = !1, d, g = 0, m = 0, p = 0;
    e:
      for (let v = i, x = s; v <= x; v++) {
        const b = t.getLineContent(v), y = b.length;
        u.reset(0);
        do
          if (d = u.next(b), d) {
            let _ = d.index, N = d.index + d[0].length;
            if (_ > 0) {
              const M = b.charCodeAt(_ - 1);
              Kn(M) && _--;
            }
            if (N + 1 < y) {
              const M = b.charCodeAt(N - 1);
              Kn(M) && N++;
            }
            const w = b.substring(_, N);
            let S = Rr(_ + 1, ra, b, 0);
            S && S.endColumn <= _ + 1 && (S = null);
            const C = a.shouldHighlightNonBasicASCII(w, S ? S.word : null);
            if (C !== 0) {
              C === 3 ? g++ : C === 2 ? m++ : C === 1 ? p++ : ll();
              const M = 1e3;
              if (c.length >= M) {
                f = !0;
                break e;
              }
              c.push(new ae(v, _ + 1, v, N + 1));
            }
          }
        while (d);
      }
    return {
      ranges: c,
      hasMore: f,
      ambiguousCharacterCount: g,
      invisibleCharacterCount: m,
      nonBasicAsciiCharacterCount: p
    };
  }
  static computeUnicodeHighlightReason(t, n) {
    const r = new Hi(n);
    switch (r.shouldHighlightNonBasicASCII(t, null)) {
      case 0:
        return null;
      case 2:
        return {
          kind: 1
          /* UnicodeHighlighterReasonKind.Invisible */
        };
      case 3: {
        const s = t.codePointAt(0), a = r.ambiguousCharacters.getPrimaryConfusable(s), o = lt.getLocales().filter((l) => !lt.getInstance(/* @__PURE__ */ new Set([...n.allowedLocales, l])).isAmbiguous(s));
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
function cl(e, t) {
  return `[${Ja(e.map((r) => String.fromCodePoint(r)).join(""))}]`;
}
class Hi {
  constructor(t) {
    this.options = t, this.allowedCodePoints = new Set(t.allowedCodePoints), this.ambiguousCharacters = lt.getInstance(new Set(t.allowedLocales));
  }
  getCandidateCodePoints() {
    if (this.options.nonBasicASCII)
      return "allNonBasicAscii";
    const t = /* @__PURE__ */ new Set();
    if (this.options.invisibleCharacters)
      for (const n of et.codePoints)
        zi(String.fromCodePoint(n)) || t.add(n);
    if (this.options.ambiguousCharacters)
      for (const n of this.ambiguousCharacters.getConfusableCodePoints())
        t.add(n);
    for (const n of this.allowedCodePoints)
      t.delete(n);
    return t;
  }
  shouldHighlightNonBasicASCII(t, n) {
    const r = t.codePointAt(0);
    if (this.allowedCodePoints.has(r))
      return 0;
    if (this.options.nonBasicASCII)
      return 1;
    let i = !1, s = !1;
    if (n)
      for (const a of n) {
        const o = a.codePointAt(0), l = no(a);
        i = i || l, !l && !this.ambiguousCharacters.isAmbiguous(o) && !et.isInvisibleCharacter(o) && (s = !0);
      }
    return (
      /* Don't allow mixing weird looking characters with ASCII */
      !i && /* Is there an obviously weird looking character? */
      s ? 0 : this.options.invisibleCharacters && !zi(t) && et.isInvisibleCharacter(r) ? 2 : this.options.ambiguousCharacters && this.ambiguousCharacters.isAmbiguous(r) ? 3 : 0
    );
  }
}
function zi(e) {
  return e === " " || e === `
` || e === "	";
}
class gn {
  constructor(t, n, r) {
    this.changes = t, this.moves = n, this.hitTimeout = r;
  }
}
class fl {
  constructor(t, n) {
    this.lineRangeMapping = t, this.changes = n;
  }
}
class z {
  static addRange(t, n) {
    let r = 0;
    for (; r < n.length && n[r].endExclusive < t.start; )
      r++;
    let i = r;
    for (; i < n.length && n[i].start <= t.endExclusive; )
      i++;
    if (r === i)
      n.splice(r, 0, t);
    else {
      const s = Math.min(t.start, n[r].start), a = Math.max(t.endExclusive, n[i - 1].endExclusive);
      n.splice(r, i - r, new z(s, a));
    }
  }
  static tryCreate(t, n) {
    if (!(t > n))
      return new z(t, n);
  }
  static ofLength(t) {
    return new z(0, t);
  }
  constructor(t, n) {
    if (this.start = t, this.endExclusive = n, t > n)
      throw new Ke(`Invalid range: ${this.toString()}`);
  }
  get isEmpty() {
    return this.start === this.endExclusive;
  }
  delta(t) {
    return new z(this.start + t, this.endExclusive + t);
  }
  deltaStart(t) {
    return new z(this.start + t, this.endExclusive);
  }
  deltaEnd(t) {
    return new z(this.start, this.endExclusive + t);
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
    return new z(Math.min(this.start, t.start), Math.max(this.endExclusive, t.endExclusive));
  }
  /**
   * for all numbers n: range1.contains(n) and range2.contains(n) <=> range1.intersect(range2).contains(n)
   *
   * The resulting range is empty if the ranges do not intersect, but touch.
   * If the ranges don't even touch, the result is undefined.
   */
  intersect(t) {
    const n = Math.max(this.start, t.start), r = Math.min(this.endExclusive, t.endExclusive);
    if (n <= r)
      return new z(n, r);
  }
  slice(t) {
    return t.slice(this.start, this.endExclusive);
  }
  /**
   * Returns the given value if it is contained in this instance, otherwise the closest value that is contained.
   * The range must not be empty.
   */
  clip(t) {
    if (this.isEmpty)
      throw new Ke(`Invalid clipping range: ${this.toString()}`);
    return Math.max(this.start, Math.min(this.endExclusive - 1, t));
  }
  /**
   * Returns `r := value + k * length` such that `r` is contained in this range.
   * The range must not be empty.
   *
   * E.g. `[5, 10).clipCyclic(10) === 5`, `[5, 10).clipCyclic(11) === 6` and `[5, 10).clipCyclic(4) === 9`.
   */
  clipCyclic(t) {
    if (this.isEmpty)
      throw new Ke(`Invalid clipping range: ${this.toString()}`);
    return t < this.start ? this.endExclusive - (this.start - t) % this.length : t >= this.endExclusive ? this.start + (t - this.start) % this.length : t;
  }
  forEach(t) {
    for (let n = this.start; n < this.endExclusive; n++)
      t(n);
  }
}
function Bt(e, t) {
  const n = Ut(e, t);
  return n === -1 ? void 0 : e[n];
}
function Ut(e, t, n = 0, r = e.length) {
  let i = n, s = r;
  for (; i < s; ) {
    const a = Math.floor((i + s) / 2);
    t(e[a]) ? i = a + 1 : s = a;
  }
  return i - 1;
}
function hl(e, t) {
  const n = fr(e, t);
  return n === e.length ? void 0 : e[n];
}
function fr(e, t, n = 0, r = e.length) {
  let i = n, s = r;
  for (; i < s; ) {
    const a = Math.floor((i + s) / 2);
    t(e[a]) ? s = a : i = a + 1;
  }
  return i;
}
class Yt {
  constructor(t) {
    this._array = t, this._findLastMonotonousLastIdx = 0;
  }
  /**
   * The predicate must be monotonous, i.e. `arr.map(predicate)` must be like `[true, ..., true, false, ..., false]`!
   * For subsequent calls, current predicate must be weaker than (or equal to) the previous predicate, i.e. more entries must be `true`.
   */
  findLastMonotonous(t) {
    if (Yt.assertInvariants) {
      if (this._prevFindLastPredicate) {
        for (const r of this._array)
          if (this._prevFindLastPredicate(r) && !t(r))
            throw new Error("MonotonousArray: current predicate must be weaker than (or equal to) the previous predicate.");
      }
      this._prevFindLastPredicate = t;
    }
    const n = Ut(this._array, t, this._findLastMonotonousLastIdx);
    return this._findLastMonotonousLastIdx = n + 1, n === -1 ? void 0 : this._array[n];
  }
}
Yt.assertInvariants = !1;
class J {
  static fromRange(t) {
    return new J(t.startLineNumber, t.endLineNumber);
  }
  /**
   * @param lineRanges An array of sorted line ranges.
   */
  static joinMany(t) {
    if (t.length === 0)
      return [];
    let n = new Fe(t[0].slice());
    for (let r = 1; r < t.length; r++)
      n = n.getUnion(new Fe(t[r].slice()));
    return n.ranges;
  }
  static ofLength(t, n) {
    return new J(t, t + n);
  }
  /**
   * @internal
   */
  static deserialize(t) {
    return new J(t[0], t[1]);
  }
  constructor(t, n) {
    if (t > n)
      throw new Ke(`startLineNumber ${t} cannot be after endLineNumberExclusive ${n}`);
    this.startLineNumber = t, this.endLineNumberExclusive = n;
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
    return new J(this.startLineNumber + t, this.endLineNumberExclusive + t);
  }
  deltaLength(t) {
    return new J(this.startLineNumber, this.endLineNumberExclusive + t);
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
    return new J(Math.min(this.startLineNumber, t.startLineNumber), Math.max(this.endLineNumberExclusive, t.endLineNumberExclusive));
  }
  toString() {
    return `[${this.startLineNumber},${this.endLineNumberExclusive})`;
  }
  /**
   * The resulting range is empty if the ranges do not intersect, but touch.
   * If the ranges don't even touch, the result is undefined.
   */
  intersect(t) {
    const n = Math.max(this.startLineNumber, t.startLineNumber), r = Math.min(this.endLineNumberExclusive, t.endLineNumberExclusive);
    if (n <= r)
      return new J(n, r);
  }
  intersectsStrict(t) {
    return this.startLineNumber < t.endLineNumberExclusive && t.startLineNumber < this.endLineNumberExclusive;
  }
  overlapOrTouch(t) {
    return this.startLineNumber <= t.endLineNumberExclusive && t.startLineNumber <= this.endLineNumberExclusive;
  }
  equals(t) {
    return this.startLineNumber === t.startLineNumber && this.endLineNumberExclusive === t.endLineNumberExclusive;
  }
  toInclusiveRange() {
    return this.isEmpty ? null : new ae(this.startLineNumber, 1, this.endLineNumberExclusive - 1, Number.MAX_SAFE_INTEGER);
  }
  toExclusiveRange() {
    return new ae(this.startLineNumber, 1, this.endLineNumberExclusive, 1);
  }
  mapToLineArray(t) {
    const n = [];
    for (let r = this.startLineNumber; r < this.endLineNumberExclusive; r++)
      n.push(t(r));
    return n;
  }
  forEach(t) {
    for (let n = this.startLineNumber; n < this.endLineNumberExclusive; n++)
      t(n);
  }
  /**
   * @internal
   */
  serialize() {
    return [this.startLineNumber, this.endLineNumberExclusive];
  }
  includes(t) {
    return this.startLineNumber <= t && t < this.endLineNumberExclusive;
  }
  /**
   * Converts this 1-based line range to a 0-based offset range (subtracts 1!).
   * @internal
   */
  toOffsetRange() {
    return new z(this.startLineNumber - 1, this.endLineNumberExclusive - 1);
  }
}
class Fe {
  constructor(t = []) {
    this._normalizedRanges = t;
  }
  get ranges() {
    return this._normalizedRanges;
  }
  addRange(t) {
    if (t.length === 0)
      return;
    const n = fr(this._normalizedRanges, (i) => i.endLineNumberExclusive >= t.startLineNumber), r = Ut(this._normalizedRanges, (i) => i.startLineNumber <= t.endLineNumberExclusive) + 1;
    if (n === r)
      this._normalizedRanges.splice(n, 0, t);
    else if (n === r - 1) {
      const i = this._normalizedRanges[n];
      this._normalizedRanges[n] = i.join(t);
    } else {
      const i = this._normalizedRanges[n].join(this._normalizedRanges[r - 1]).join(t);
      this._normalizedRanges.splice(n, r - n, i);
    }
  }
  contains(t) {
    const n = Bt(this._normalizedRanges, (r) => r.startLineNumber <= t);
    return !!n && n.endLineNumberExclusive > t;
  }
  getUnion(t) {
    if (this._normalizedRanges.length === 0)
      return t;
    if (t._normalizedRanges.length === 0)
      return this;
    const n = [];
    let r = 0, i = 0, s = null;
    for (; r < this._normalizedRanges.length || i < t._normalizedRanges.length; ) {
      let a = null;
      if (r < this._normalizedRanges.length && i < t._normalizedRanges.length) {
        const o = this._normalizedRanges[r], l = t._normalizedRanges[i];
        o.startLineNumber < l.startLineNumber ? (a = o, r++) : (a = l, i++);
      } else
        r < this._normalizedRanges.length ? (a = this._normalizedRanges[r], r++) : (a = t._normalizedRanges[i], i++);
      s === null ? s = a : s.endLineNumberExclusive >= a.startLineNumber ? s = new J(s.startLineNumber, Math.max(s.endLineNumberExclusive, a.endLineNumberExclusive)) : (n.push(s), s = a);
    }
    return s !== null && n.push(s), new Fe(n);
  }
  /**
   * Subtracts all ranges in this set from `range` and returns the result.
   */
  subtractFrom(t) {
    const n = fr(this._normalizedRanges, (a) => a.endLineNumberExclusive >= t.startLineNumber), r = Ut(this._normalizedRanges, (a) => a.startLineNumber <= t.endLineNumberExclusive) + 1;
    if (n === r)
      return new Fe([t]);
    const i = [];
    let s = t.startLineNumber;
    for (let a = n; a < r; a++) {
      const o = this._normalizedRanges[a];
      o.startLineNumber > s && i.push(new J(s, o.startLineNumber)), s = o.endLineNumberExclusive;
    }
    return s < t.endLineNumberExclusive && i.push(new J(s, t.endLineNumberExclusive)), new Fe(i);
  }
  toString() {
    return this._normalizedRanges.map((t) => t.toString()).join(", ");
  }
  getIntersection(t) {
    const n = [];
    let r = 0, i = 0;
    for (; r < this._normalizedRanges.length && i < t._normalizedRanges.length; ) {
      const s = this._normalizedRanges[r], a = t._normalizedRanges[i], o = s.intersect(a);
      o && !o.isEmpty && n.push(o), s.endLineNumberExclusive < a.endLineNumberExclusive ? r++ : i++;
    }
    return new Fe(n);
  }
  getWithDelta(t) {
    return new Fe(this._normalizedRanges.map((n) => n.delta(t)));
  }
}
class rt {
  static inverse(t, n, r) {
    const i = [];
    let s = 1, a = 1;
    for (const l of t) {
      const u = new Ue(new J(s, l.original.startLineNumber), new J(a, l.modified.startLineNumber), void 0);
      u.modified.isEmpty || i.push(u), s = l.original.endLineNumberExclusive, a = l.modified.endLineNumberExclusive;
    }
    const o = new Ue(new J(s, n + 1), new J(a, r + 1), void 0);
    return o.modified.isEmpty || i.push(o), i;
  }
  constructor(t, n) {
    this.original = t, this.modified = n;
  }
  toString() {
    return `{${this.original.toString()}->${this.modified.toString()}}`;
  }
  flip() {
    return new rt(this.modified, this.original);
  }
  join(t) {
    return new rt(this.original.join(t.original), this.modified.join(t.modified));
  }
}
class Ue extends rt {
  constructor(t, n, r) {
    super(t, n), this.innerChanges = r;
  }
  flip() {
    var t;
    return new Ue(this.modified, this.original, (t = this.innerChanges) === null || t === void 0 ? void 0 : t.map((n) => n.flip()));
  }
}
class $t {
  constructor(t, n) {
    this.originalRange = t, this.modifiedRange = n;
  }
  toString() {
    return `{${this.originalRange.toString()}->${this.modifiedRange.toString()}}`;
  }
  flip() {
    return new $t(this.modifiedRange, this.originalRange);
  }
}
const dl = 3;
class gl {
  computeDiff(t, n, r) {
    var i;
    const a = new vl(t, n, {
      maxComputationTime: r.maxComputationTimeMs,
      shouldIgnoreTrimWhitespace: r.ignoreTrimWhitespace,
      shouldComputeCharChanges: !0,
      shouldMakePrettyDiff: !0,
      shouldPostProcessCharChanges: !0
    }).computeDiff(), o = [];
    let l = null;
    for (const u of a.changes) {
      let c;
      u.originalEndLineNumber === 0 ? c = new J(u.originalStartLineNumber + 1, u.originalStartLineNumber + 1) : c = new J(u.originalStartLineNumber, u.originalEndLineNumber + 1);
      let f;
      u.modifiedEndLineNumber === 0 ? f = new J(u.modifiedStartLineNumber + 1, u.modifiedStartLineNumber + 1) : f = new J(u.modifiedStartLineNumber, u.modifiedEndLineNumber + 1);
      let d = new Ue(c, f, (i = u.charChanges) === null || i === void 0 ? void 0 : i.map((g) => new $t(new ae(g.originalStartLineNumber, g.originalStartColumn, g.originalEndLineNumber, g.originalEndColumn), new ae(g.modifiedStartLineNumber, g.modifiedStartColumn, g.modifiedEndLineNumber, g.modifiedEndColumn))));
      l && (l.modified.endLineNumberExclusive === d.modified.startLineNumber || l.original.endLineNumberExclusive === d.original.startLineNumber) && (d = new Ue(l.original.join(d.original), l.modified.join(d.modified), l.innerChanges && d.innerChanges ? l.innerChanges.concat(d.innerChanges) : void 0), o.pop()), o.push(d), l = d;
    }
    return xn(() => aa(o, (u, c) => c.original.startLineNumber - u.original.endLineNumberExclusive === c.modified.startLineNumber - u.modified.endLineNumberExclusive && // There has to be an unchanged line in between (otherwise both diffs should have been joined)
    u.original.endLineNumberExclusive < c.original.startLineNumber && u.modified.endLineNumberExclusive < c.modified.startLineNumber)), new gn(o, [], a.quitEarly);
  }
}
function oa(e, t, n, r) {
  return new Ye(e, t, n).ComputeDiff(r);
}
let Gi = class {
  constructor(t) {
    const n = [], r = [];
    for (let i = 0, s = t.length; i < s; i++)
      n[i] = hr(t[i], 1), r[i] = dr(t[i], 1);
    this.lines = t, this._startColumns = n, this._endColumns = r;
  }
  getElements() {
    const t = [];
    for (let n = 0, r = this.lines.length; n < r; n++)
      t[n] = this.lines[n].substring(this._startColumns[n] - 1, this._endColumns[n] - 1);
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
  createCharSequence(t, n, r) {
    const i = [], s = [], a = [];
    let o = 0;
    for (let l = n; l <= r; l++) {
      const u = this.lines[l], c = t ? this._startColumns[l] : 1, f = t ? this._endColumns[l] : u.length + 1;
      for (let d = c; d < f; d++)
        i[o] = u.charCodeAt(d - 1), s[o] = l + 1, a[o] = d, o++;
      !t && l < r && (i[o] = 10, s[o] = l + 1, a[o] = u.length + 1, o++);
    }
    return new ml(i, s, a);
  }
};
class ml {
  constructor(t, n, r) {
    this._charCodes = t, this._lineNumbers = n, this._columns = r;
  }
  toString() {
    return "[" + this._charCodes.map((t, n) => (t === 10 ? "\\n" : String.fromCharCode(t)) + `-(${this._lineNumbers[n]},${this._columns[n]})`).join(", ") + "]";
  }
  _assertIndex(t, n) {
    if (t < 0 || t >= n.length)
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
class xt {
  constructor(t, n, r, i, s, a, o, l) {
    this.originalStartLineNumber = t, this.originalStartColumn = n, this.originalEndLineNumber = r, this.originalEndColumn = i, this.modifiedStartLineNumber = s, this.modifiedStartColumn = a, this.modifiedEndLineNumber = o, this.modifiedEndColumn = l;
  }
  static createFromDiffChange(t, n, r) {
    const i = n.getStartLineNumber(t.originalStart), s = n.getStartColumn(t.originalStart), a = n.getEndLineNumber(t.originalStart + t.originalLength - 1), o = n.getEndColumn(t.originalStart + t.originalLength - 1), l = r.getStartLineNumber(t.modifiedStart), u = r.getStartColumn(t.modifiedStart), c = r.getEndLineNumber(t.modifiedStart + t.modifiedLength - 1), f = r.getEndColumn(t.modifiedStart + t.modifiedLength - 1);
    return new xt(i, s, a, o, l, u, c, f);
  }
}
function pl(e) {
  if (e.length <= 1)
    return e;
  const t = [e[0]];
  let n = t[0];
  for (let r = 1, i = e.length; r < i; r++) {
    const s = e[r], a = s.originalStart - (n.originalStart + n.originalLength), o = s.modifiedStart - (n.modifiedStart + n.modifiedLength);
    Math.min(a, o) < dl ? (n.originalLength = s.originalStart + s.originalLength - n.originalStart, n.modifiedLength = s.modifiedStart + s.modifiedLength - n.modifiedStart) : (t.push(s), n = s);
  }
  return t;
}
class It {
  constructor(t, n, r, i, s) {
    this.originalStartLineNumber = t, this.originalEndLineNumber = n, this.modifiedStartLineNumber = r, this.modifiedEndLineNumber = i, this.charChanges = s;
  }
  static createFromDiffResult(t, n, r, i, s, a, o) {
    let l, u, c, f, d;
    if (n.originalLength === 0 ? (l = r.getStartLineNumber(n.originalStart) - 1, u = 0) : (l = r.getStartLineNumber(n.originalStart), u = r.getEndLineNumber(n.originalStart + n.originalLength - 1)), n.modifiedLength === 0 ? (c = i.getStartLineNumber(n.modifiedStart) - 1, f = 0) : (c = i.getStartLineNumber(n.modifiedStart), f = i.getEndLineNumber(n.modifiedStart + n.modifiedLength - 1)), a && n.originalLength > 0 && n.originalLength < 20 && n.modifiedLength > 0 && n.modifiedLength < 20 && s()) {
      const g = r.createCharSequence(t, n.originalStart, n.originalStart + n.originalLength - 1), m = i.createCharSequence(t, n.modifiedStart, n.modifiedStart + n.modifiedLength - 1);
      if (g.getElements().length > 0 && m.getElements().length > 0) {
        let p = oa(g, m, s, !0).changes;
        o && (p = pl(p)), d = [];
        for (let v = 0, x = p.length; v < x; v++)
          d.push(xt.createFromDiffChange(p[v], g, m));
      }
    }
    return new It(l, u, c, f, d);
  }
}
class vl {
  constructor(t, n, r) {
    this.shouldComputeCharChanges = r.shouldComputeCharChanges, this.shouldPostProcessCharChanges = r.shouldPostProcessCharChanges, this.shouldIgnoreTrimWhitespace = r.shouldIgnoreTrimWhitespace, this.shouldMakePrettyDiff = r.shouldMakePrettyDiff, this.originalLines = t, this.modifiedLines = n, this.original = new Gi(t), this.modified = new Gi(n), this.continueLineDiff = Ji(r.maxComputationTime), this.continueCharDiff = Ji(r.maxComputationTime === 0 ? 0 : Math.min(r.maxComputationTime, 5e3));
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
    const t = oa(this.original, this.modified, this.continueLineDiff, this.shouldMakePrettyDiff), n = t.changes, r = t.quitEarly;
    if (this.shouldIgnoreTrimWhitespace) {
      const o = [];
      for (let l = 0, u = n.length; l < u; l++)
        o.push(It.createFromDiffResult(this.shouldIgnoreTrimWhitespace, n[l], this.original, this.modified, this.continueCharDiff, this.shouldComputeCharChanges, this.shouldPostProcessCharChanges));
      return {
        quitEarly: r,
        changes: o
      };
    }
    const i = [];
    let s = 0, a = 0;
    for (let o = -1, l = n.length; o < l; o++) {
      const u = o + 1 < l ? n[o + 1] : null, c = u ? u.originalStart : this.originalLines.length, f = u ? u.modifiedStart : this.modifiedLines.length;
      for (; s < c && a < f; ) {
        const d = this.originalLines[s], g = this.modifiedLines[a];
        if (d !== g) {
          {
            let m = hr(d, 1), p = hr(g, 1);
            for (; m > 1 && p > 1; ) {
              const v = d.charCodeAt(m - 2), x = g.charCodeAt(p - 2);
              if (v !== x)
                break;
              m--, p--;
            }
            (m > 1 || p > 1) && this._pushTrimWhitespaceCharChange(i, s + 1, 1, m, a + 1, 1, p);
          }
          {
            let m = dr(d, 1), p = dr(g, 1);
            const v = d.length + 1, x = g.length + 1;
            for (; m < v && p < x; ) {
              const b = d.charCodeAt(m - 1), y = d.charCodeAt(p - 1);
              if (b !== y)
                break;
              m++, p++;
            }
            (m < v || p < x) && this._pushTrimWhitespaceCharChange(i, s + 1, m, v, a + 1, p, x);
          }
        }
        s++, a++;
      }
      u && (i.push(It.createFromDiffResult(this.shouldIgnoreTrimWhitespace, u, this.original, this.modified, this.continueCharDiff, this.shouldComputeCharChanges, this.shouldPostProcessCharChanges)), s += u.originalLength, a += u.modifiedLength);
    }
    return {
      quitEarly: r,
      changes: i
    };
  }
  _pushTrimWhitespaceCharChange(t, n, r, i, s, a, o) {
    if (this._mergeTrimWhitespaceCharChange(t, n, r, i, s, a, o))
      return;
    let l;
    this.shouldComputeCharChanges && (l = [new xt(n, r, n, i, s, a, s, o)]), t.push(new It(n, n, s, s, l));
  }
  _mergeTrimWhitespaceCharChange(t, n, r, i, s, a, o) {
    const l = t.length;
    if (l === 0)
      return !1;
    const u = t[l - 1];
    return u.originalEndLineNumber === 0 || u.modifiedEndLineNumber === 0 ? !1 : u.originalEndLineNumber === n && u.modifiedEndLineNumber === s ? (this.shouldComputeCharChanges && u.charChanges && u.charChanges.push(new xt(n, r, n, i, s, a, s, o)), !0) : u.originalEndLineNumber + 1 === n && u.modifiedEndLineNumber + 1 === s ? (u.originalEndLineNumber = n, u.modifiedEndLineNumber = s, this.shouldComputeCharChanges && u.charChanges && u.charChanges.push(new xt(n, r, n, i, s, a, s, o)), !0) : !1;
  }
}
function hr(e, t) {
  const n = Qa(e);
  return n === -1 ? t : n + 1;
}
function dr(e, t) {
  const n = Za(e);
  return n === -1 ? t : n + 2;
}
function Ji(e) {
  if (e === 0)
    return () => !0;
  const t = Date.now();
  return () => Date.now() - t < e;
}
class qe {
  static trivial(t, n) {
    return new qe([new re(z.ofLength(t.length), z.ofLength(n.length))], !1);
  }
  static trivialTimedOut(t, n) {
    return new qe([new re(z.ofLength(t.length), z.ofLength(n.length))], !0);
  }
  constructor(t, n) {
    this.diffs = t, this.hitTimeout = n;
  }
}
class re {
  static invert(t, n) {
    const r = [];
    return Po(t, (i, s) => {
      r.push(re.fromOffsetPairs(i ? i.getEndExclusives() : Ie.zero, s ? s.getStarts() : new Ie(n, (i ? i.seq2Range.endExclusive - i.seq1Range.endExclusive : 0) + n)));
    }), r;
  }
  static fromOffsetPairs(t, n) {
    return new re(new z(t.offset1, n.offset1), new z(t.offset2, n.offset2));
  }
  constructor(t, n) {
    this.seq1Range = t, this.seq2Range = n;
  }
  swap() {
    return new re(this.seq2Range, this.seq1Range);
  }
  toString() {
    return `${this.seq1Range} <-> ${this.seq2Range}`;
  }
  join(t) {
    return new re(this.seq1Range.join(t.seq1Range), this.seq2Range.join(t.seq2Range));
  }
  delta(t) {
    return t === 0 ? this : new re(this.seq1Range.delta(t), this.seq2Range.delta(t));
  }
  deltaStart(t) {
    return t === 0 ? this : new re(this.seq1Range.deltaStart(t), this.seq2Range.deltaStart(t));
  }
  deltaEnd(t) {
    return t === 0 ? this : new re(this.seq1Range.deltaEnd(t), this.seq2Range.deltaEnd(t));
  }
  intersect(t) {
    const n = this.seq1Range.intersect(t.seq1Range), r = this.seq2Range.intersect(t.seq2Range);
    if (!(!n || !r))
      return new re(n, r);
  }
  getStarts() {
    return new Ie(this.seq1Range.start, this.seq2Range.start);
  }
  getEndExclusives() {
    return new Ie(this.seq1Range.endExclusive, this.seq2Range.endExclusive);
  }
}
class Ie {
  constructor(t, n) {
    this.offset1 = t, this.offset2 = n;
  }
  toString() {
    return `${this.offset1} <-> ${this.offset2}`;
  }
}
Ie.zero = new Ie(0, 0);
Ie.max = new Ie(Number.MAX_SAFE_INTEGER, Number.MAX_SAFE_INTEGER);
class Wt {
  isValid() {
    return !0;
  }
}
Wt.instance = new Wt();
class bl {
  constructor(t) {
    if (this.timeout = t, this.startTime = Date.now(), this.valid = !0, t <= 0)
      throw new Ke("timeout must be positive");
  }
  // Recommendation: Set a log-point `{this.disable()}` in the body
  isValid() {
    if (!(Date.now() - this.startTime < this.timeout) && this.valid) {
      this.valid = !1;
      debugger;
    }
    return this.valid;
  }
}
class jn {
  constructor(t, n) {
    this.width = t, this.height = n, this.array = [], this.array = new Array(t * n);
  }
  get(t, n) {
    return this.array[t + n * this.width];
  }
  set(t, n, r) {
    this.array[t + n * this.width] = r;
  }
}
function gr(e) {
  return e === 32 || e === 9;
}
class At {
  static getKey(t) {
    let n = this.chrKeys.get(t);
    return n === void 0 && (n = this.chrKeys.size, this.chrKeys.set(t, n)), n;
  }
  constructor(t, n, r) {
    this.range = t, this.lines = n, this.source = r, this.histogram = [];
    let i = 0;
    for (let s = t.startLineNumber - 1; s < t.endLineNumberExclusive - 1; s++) {
      const a = n[s];
      for (let l = 0; l < a.length; l++) {
        i++;
        const u = a[l], c = At.getKey(u);
        this.histogram[c] = (this.histogram[c] || 0) + 1;
      }
      i++;
      const o = At.getKey(`
`);
      this.histogram[o] = (this.histogram[o] || 0) + 1;
    }
    this.totalCount = i;
  }
  computeSimilarity(t) {
    var n, r;
    let i = 0;
    const s = Math.max(this.histogram.length, t.histogram.length);
    for (let a = 0; a < s; a++)
      i += Math.abs(((n = this.histogram[a]) !== null && n !== void 0 ? n : 0) - ((r = t.histogram[a]) !== null && r !== void 0 ? r : 0));
    return 1 - i / (this.totalCount + t.totalCount);
  }
}
At.chrKeys = /* @__PURE__ */ new Map();
class yl {
  compute(t, n, r = Wt.instance, i) {
    if (t.length === 0 || n.length === 0)
      return qe.trivial(t, n);
    const s = new jn(t.length, n.length), a = new jn(t.length, n.length), o = new jn(t.length, n.length);
    for (let m = 0; m < t.length; m++)
      for (let p = 0; p < n.length; p++) {
        if (!r.isValid())
          return qe.trivialTimedOut(t, n);
        const v = m === 0 ? 0 : s.get(m - 1, p), x = p === 0 ? 0 : s.get(m, p - 1);
        let b;
        t.getElement(m) === n.getElement(p) ? (m === 0 || p === 0 ? b = 0 : b = s.get(m - 1, p - 1), m > 0 && p > 0 && a.get(m - 1, p - 1) === 3 && (b += o.get(m - 1, p - 1)), b += i ? i(m, p) : 1) : b = -1;
        const y = Math.max(v, x, b);
        if (y === b) {
          const _ = m > 0 && p > 0 ? o.get(m - 1, p - 1) : 0;
          o.set(m, p, _ + 1), a.set(m, p, 3);
        } else
          y === v ? (o.set(m, p, 0), a.set(m, p, 1)) : y === x && (o.set(m, p, 0), a.set(m, p, 2));
        s.set(m, p, y);
      }
    const l = [];
    let u = t.length, c = n.length;
    function f(m, p) {
      (m + 1 !== u || p + 1 !== c) && l.push(new re(new z(m + 1, u), new z(p + 1, c))), u = m, c = p;
    }
    let d = t.length - 1, g = n.length - 1;
    for (; d >= 0 && g >= 0; )
      a.get(d, g) === 3 ? (f(d, g), d--, g--) : a.get(d, g) === 1 ? d-- : g--;
    return f(-1, -1), l.reverse(), new qe(l, !1);
  }
}
class la {
  compute(t, n, r = Wt.instance) {
    if (t.length === 0 || n.length === 0)
      return qe.trivial(t, n);
    const i = t, s = n;
    function a(p, v) {
      for (; p < i.length && v < s.length && i.getElement(p) === s.getElement(v); )
        p++, v++;
      return p;
    }
    let o = 0;
    const l = new xl();
    l.set(0, a(0, 0));
    const u = new wl();
    u.set(0, l.get(0) === 0 ? null : new Xi(null, 0, 0, l.get(0)));
    let c = 0;
    e:
      for (; ; ) {
        if (o++, !r.isValid())
          return qe.trivialTimedOut(i, s);
        const p = -Math.min(o, s.length + o % 2), v = Math.min(o, i.length + o % 2);
        for (c = p; c <= v; c += 2) {
          const x = c === v ? -1 : l.get(c + 1), b = c === p ? -1 : l.get(c - 1) + 1, y = Math.min(Math.max(x, b), i.length), _ = y - c;
          if (y > i.length || _ > s.length)
            continue;
          const N = a(y, _);
          l.set(c, N);
          const w = y === x ? u.get(c + 1) : u.get(c - 1);
          if (u.set(c, N !== y ? new Xi(w, y, _, N - y) : w), l.get(c) === i.length && l.get(c) - c === s.length)
            break e;
        }
      }
    let f = u.get(c);
    const d = [];
    let g = i.length, m = s.length;
    for (; ; ) {
      const p = f ? f.x + f.length : 0, v = f ? f.y + f.length : 0;
      if ((p !== g || v !== m) && d.push(new re(new z(p, g), new z(v, m))), !f)
        break;
      g = f.x, m = f.y, f = f.prev;
    }
    return d.reverse(), new qe(d, !1);
  }
}
class Xi {
  constructor(t, n, r, i) {
    this.prev = t, this.x = n, this.y = r, this.length = i;
  }
}
class xl {
  constructor() {
    this.positiveArr = new Int32Array(10), this.negativeArr = new Int32Array(10);
  }
  get(t) {
    return t < 0 ? (t = -t - 1, this.negativeArr[t]) : this.positiveArr[t];
  }
  set(t, n) {
    if (t < 0) {
      if (t = -t - 1, t >= this.negativeArr.length) {
        const r = this.negativeArr;
        this.negativeArr = new Int32Array(r.length * 2), this.negativeArr.set(r);
      }
      this.negativeArr[t] = n;
    } else {
      if (t >= this.positiveArr.length) {
        const r = this.positiveArr;
        this.positiveArr = new Int32Array(r.length * 2), this.positiveArr.set(r);
      }
      this.positiveArr[t] = n;
    }
  }
}
class wl {
  constructor() {
    this.positiveArr = [], this.negativeArr = [];
  }
  get(t) {
    return t < 0 ? (t = -t - 1, this.negativeArr[t]) : this.positiveArr[t];
  }
  set(t, n) {
    t < 0 ? (t = -t - 1, this.negativeArr[t] = n) : this.positiveArr[t] = n;
  }
}
class _l {
  constructor() {
    this.map = /* @__PURE__ */ new Map();
  }
  add(t, n) {
    let r = this.map.get(t);
    r || (r = /* @__PURE__ */ new Set(), this.map.set(t, r)), r.add(n);
  }
  delete(t, n) {
    const r = this.map.get(t);
    r && (r.delete(n), r.size === 0 && this.map.delete(t));
  }
  forEach(t, n) {
    const r = this.map.get(t);
    r && r.forEach(n);
  }
  get(t) {
    const n = this.map.get(t);
    return n || /* @__PURE__ */ new Set();
  }
}
class wn {
  constructor(t, n, r) {
    this.lines = t, this.considerWhitespaceChanges = r, this.elements = [], this.firstCharOffsetByLine = [], this.additionalOffsetByLine = [];
    let i = !1;
    n.start > 0 && n.endExclusive >= t.length && (n = new z(n.start - 1, n.endExclusive), i = !0), this.lineRange = n, this.firstCharOffsetByLine[0] = 0;
    for (let s = this.lineRange.start; s < this.lineRange.endExclusive; s++) {
      let a = t[s], o = 0;
      if (i)
        o = a.length, a = "", i = !1;
      else if (!r) {
        const l = a.trimStart();
        o = a.length - l.length, a = l.trimEnd();
      }
      this.additionalOffsetByLine.push(o);
      for (let l = 0; l < a.length; l++)
        this.elements.push(a.charCodeAt(l));
      s < t.length - 1 && (this.elements.push(`
`.charCodeAt(0)), this.firstCharOffsetByLine[s - this.lineRange.start + 1] = this.elements.length);
    }
    this.additionalOffsetByLine.push(0);
  }
  toString() {
    return `Slice: "${this.text}"`;
  }
  get text() {
    return this.getText(new z(0, this.length));
  }
  getText(t) {
    return this.elements.slice(t.start, t.endExclusive).map((n) => String.fromCharCode(n)).join("");
  }
  getElement(t) {
    return this.elements[t];
  }
  get length() {
    return this.elements.length;
  }
  getBoundaryScore(t) {
    const n = Zi(t > 0 ? this.elements[t - 1] : -1), r = Zi(t < this.elements.length ? this.elements[t] : -1);
    if (n === 6 && r === 7)
      return 0;
    let i = 0;
    return n !== r && (i += 10, n === 0 && r === 1 && (i += 1)), i += Qi(n), i += Qi(r), i;
  }
  translateOffset(t) {
    if (this.lineRange.isEmpty)
      return new Oe(this.lineRange.start + 1, 1);
    const n = Ut(this.firstCharOffsetByLine, (r) => r <= t);
    return new Oe(this.lineRange.start + n + 1, t - this.firstCharOffsetByLine[n] + this.additionalOffsetByLine[n] + 1);
  }
  translateRange(t) {
    return ae.fromPositions(this.translateOffset(t.start), this.translateOffset(t.endExclusive));
  }
  /**
   * Finds the word that contains the character at the given offset
   */
  findWordContaining(t) {
    if (t < 0 || t >= this.elements.length || !qn(this.elements[t]))
      return;
    let n = t;
    for (; n > 0 && qn(this.elements[n - 1]); )
      n--;
    let r = t;
    for (; r < this.elements.length && qn(this.elements[r]); )
      r++;
    return new z(n, r);
  }
  countLinesIn(t) {
    return this.translateOffset(t.endExclusive).lineNumber - this.translateOffset(t.start).lineNumber;
  }
  isStronglyEqual(t, n) {
    return this.elements[t] === this.elements[n];
  }
  extendToFullLines(t) {
    var n, r;
    const i = (n = Bt(this.firstCharOffsetByLine, (a) => a <= t.start)) !== null && n !== void 0 ? n : 0, s = (r = hl(this.firstCharOffsetByLine, (a) => t.endExclusive <= a)) !== null && r !== void 0 ? r : this.elements.length;
    return new z(i, s);
  }
}
function qn(e) {
  return e >= 97 && e <= 122 || e >= 65 && e <= 90 || e >= 48 && e <= 57;
}
const Sl = {
  0: 0,
  1: 0,
  2: 0,
  3: 10,
  4: 2,
  5: 3,
  6: 10,
  7: 10
};
function Qi(e) {
  return Sl[e];
}
function Zi(e) {
  return e === 10 ? 7 : e === 13 ? 6 : gr(e) ? 5 : e >= 97 && e <= 122 ? 0 : e >= 65 && e <= 90 ? 1 : e >= 48 && e <= 57 ? 2 : e === -1 ? 3 : 4;
}
function Ll(e, t, n, r, i, s) {
  let { moves: a, excludedChanges: o } = Nl(e, t, n, s);
  if (!s.isValid())
    return [];
  const l = e.filter((c) => !o.has(c)), u = Al(l, r, i, t, n, s);
  return Io(a, u), a = Cl(a), a = a.filter((c) => c.original.toOffsetRange().slice(t).map((d) => d.trim()).join(`
`).length >= 10), a = kl(e, a), a;
}
function Nl(e, t, n, r) {
  const i = [], s = e.filter((l) => l.modified.isEmpty && l.original.length >= 3).map((l) => new At(l.original, t, l)), a = new Set(e.filter((l) => l.original.isEmpty && l.modified.length >= 3).map((l) => new At(l.modified, n, l))), o = /* @__PURE__ */ new Set();
  for (const l of s) {
    let u = -1, c;
    for (const f of a) {
      const d = l.computeSimilarity(f);
      d > u && (u = d, c = f);
    }
    if (u > 0.9 && c && (a.delete(c), i.push(new rt(l.range, c.range)), o.add(l.source), o.add(c.source)), !r.isValid())
      return { moves: i, excludedChanges: o };
  }
  return { moves: i, excludedChanges: o };
}
function Al(e, t, n, r, i, s) {
  const a = [], o = new _l();
  for (const d of e)
    for (let g = d.original.startLineNumber; g < d.original.endLineNumberExclusive - 2; g++) {
      const m = `${t[g - 1]}:${t[g + 1 - 1]}:${t[g + 2 - 1]}`;
      o.add(m, { range: new J(g, g + 3) });
    }
  const l = [];
  e.sort(cn((d) => d.modified.startLineNumber, fn));
  for (const d of e) {
    let g = [];
    for (let m = d.modified.startLineNumber; m < d.modified.endLineNumberExclusive - 2; m++) {
      const p = `${n[m - 1]}:${n[m + 1 - 1]}:${n[m + 2 - 1]}`, v = new J(m, m + 3), x = [];
      o.forEach(p, ({ range: b }) => {
        for (const _ of g)
          if (_.originalLineRange.endLineNumberExclusive + 1 === b.endLineNumberExclusive && _.modifiedLineRange.endLineNumberExclusive + 1 === v.endLineNumberExclusive) {
            _.originalLineRange = new J(_.originalLineRange.startLineNumber, b.endLineNumberExclusive), _.modifiedLineRange = new J(_.modifiedLineRange.startLineNumber, v.endLineNumberExclusive), x.push(_);
            return;
          }
        const y = {
          modifiedLineRange: v,
          originalLineRange: b
        };
        l.push(y), x.push(y);
      }), g = x;
    }
    if (!s.isValid())
      return [];
  }
  l.sort(Do(cn((d) => d.modifiedLineRange.length, fn)));
  const u = new Fe(), c = new Fe();
  for (const d of l) {
    const g = d.modifiedLineRange.startLineNumber - d.originalLineRange.startLineNumber, m = u.subtractFrom(d.modifiedLineRange), p = c.subtractFrom(d.originalLineRange).getWithDelta(g), v = m.getIntersection(p);
    for (const x of v.ranges) {
      if (x.length < 3)
        continue;
      const b = x, y = x.delta(-g);
      a.push(new rt(y, b)), u.addRange(b), c.addRange(y);
    }
  }
  a.sort(cn((d) => d.original.startLineNumber, fn));
  const f = new Yt(e);
  for (let d = 0; d < a.length; d++) {
    const g = a[d], m = f.findLastMonotonous((w) => w.original.startLineNumber <= g.original.startLineNumber), p = Bt(e, (w) => w.modified.startLineNumber <= g.modified.startLineNumber), v = Math.max(g.original.startLineNumber - m.original.startLineNumber, g.modified.startLineNumber - p.modified.startLineNumber), x = f.findLastMonotonous((w) => w.original.startLineNumber < g.original.endLineNumberExclusive), b = Bt(e, (w) => w.modified.startLineNumber < g.modified.endLineNumberExclusive), y = Math.max(x.original.endLineNumberExclusive - g.original.endLineNumberExclusive, b.modified.endLineNumberExclusive - g.modified.endLineNumberExclusive);
    let _;
    for (_ = 0; _ < v; _++) {
      const w = g.original.startLineNumber - _ - 1, S = g.modified.startLineNumber - _ - 1;
      if (w > r.length || S > i.length || u.contains(S) || c.contains(w) || !Yi(r[w - 1], i[S - 1], s))
        break;
    }
    _ > 0 && (c.addRange(new J(g.original.startLineNumber - _, g.original.startLineNumber)), u.addRange(new J(g.modified.startLineNumber - _, g.modified.startLineNumber)));
    let N;
    for (N = 0; N < y; N++) {
      const w = g.original.endLineNumberExclusive + N, S = g.modified.endLineNumberExclusive + N;
      if (w > r.length || S > i.length || u.contains(S) || c.contains(w) || !Yi(r[w - 1], i[S - 1], s))
        break;
    }
    N > 0 && (c.addRange(new J(g.original.endLineNumberExclusive, g.original.endLineNumberExclusive + N)), u.addRange(new J(g.modified.endLineNumberExclusive, g.modified.endLineNumberExclusive + N))), (_ > 0 || N > 0) && (a[d] = new rt(new J(g.original.startLineNumber - _, g.original.endLineNumberExclusive + N), new J(g.modified.startLineNumber - _, g.modified.endLineNumberExclusive + N)));
  }
  return a;
}
function Yi(e, t, n) {
  if (e.trim() === t.trim())
    return !0;
  if (e.length > 300 && t.length > 300)
    return !1;
  const i = new la().compute(new wn([e], new z(0, 1), !1), new wn([t], new z(0, 1), !1), n);
  let s = 0;
  const a = re.invert(i.diffs, e.length);
  for (const c of a)
    c.seq1Range.forEach((f) => {
      gr(e.charCodeAt(f)) || s++;
    });
  function o(c) {
    let f = 0;
    for (let d = 0; d < e.length; d++)
      gr(c.charCodeAt(d)) || f++;
    return f;
  }
  const l = o(e.length > t.length ? e : t);
  return s / l > 0.6 && l > 10;
}
function Cl(e) {
  if (e.length === 0)
    return e;
  e.sort(cn((n) => n.original.startLineNumber, fn));
  const t = [e[0]];
  for (let n = 1; n < e.length; n++) {
    const r = t[t.length - 1], i = e[n], s = i.original.startLineNumber - r.original.endLineNumberExclusive, a = i.modified.startLineNumber - r.modified.endLineNumberExclusive;
    if (s >= 0 && a >= 0 && s + a <= 2) {
      t[t.length - 1] = r.join(i);
      continue;
    }
    t.push(i);
  }
  return t;
}
function kl(e, t) {
  const n = new Yt(e);
  return t = t.filter((r) => {
    const i = n.findLastMonotonous((o) => o.original.endLineNumberExclusive < r.original.endLineNumberExclusive) || new rt(new J(1, 1), new J(1, 1)), s = Bt(e, (o) => o.modified.endLineNumberExclusive < r.modified.endLineNumberExclusive);
    return i !== s;
  }), t;
}
function Ki(e, t, n) {
  let r = n;
  return r = El(e, t, r), r = Ml(e, t, r), r;
}
function El(e, t, n) {
  if (n.length === 0)
    return n;
  const r = [];
  r.push(n[0]);
  for (let s = 1; s < n.length; s++) {
    const a = r[r.length - 1];
    let o = n[s];
    if (o.seq1Range.isEmpty || o.seq2Range.isEmpty) {
      const l = o.seq1Range.start - a.seq1Range.endExclusive;
      let u;
      for (u = 1; u <= l && !(e.getElement(o.seq1Range.start - u) !== e.getElement(o.seq1Range.endExclusive - u) || t.getElement(o.seq2Range.start - u) !== t.getElement(o.seq2Range.endExclusive - u)); u++)
        ;
      if (u--, u === l) {
        r[r.length - 1] = new re(new z(a.seq1Range.start, o.seq1Range.endExclusive - l), new z(a.seq2Range.start, o.seq2Range.endExclusive - l));
        continue;
      }
      o = o.delta(-u);
    }
    r.push(o);
  }
  const i = [];
  for (let s = 0; s < r.length - 1; s++) {
    const a = r[s + 1];
    let o = r[s];
    if (o.seq1Range.isEmpty || o.seq2Range.isEmpty) {
      const l = a.seq1Range.start - o.seq1Range.endExclusive;
      let u;
      for (u = 0; u < l && !(!e.isStronglyEqual(o.seq1Range.start + u, o.seq1Range.endExclusive + u) || !t.isStronglyEqual(o.seq2Range.start + u, o.seq2Range.endExclusive + u)); u++)
        ;
      if (u === l) {
        r[s + 1] = new re(new z(o.seq1Range.start + l, a.seq1Range.endExclusive), new z(o.seq2Range.start + l, a.seq2Range.endExclusive));
        continue;
      }
      u > 0 && (o = o.delta(u));
    }
    i.push(o);
  }
  return r.length > 0 && i.push(r[r.length - 1]), i;
}
function Ml(e, t, n) {
  if (!e.getBoundaryScore || !t.getBoundaryScore)
    return n;
  for (let r = 0; r < n.length; r++) {
    const i = r > 0 ? n[r - 1] : void 0, s = n[r], a = r + 1 < n.length ? n[r + 1] : void 0, o = new z(i ? i.seq1Range.start + 1 : 0, a ? a.seq1Range.endExclusive - 1 : e.length), l = new z(i ? i.seq2Range.start + 1 : 0, a ? a.seq2Range.endExclusive - 1 : t.length);
    s.seq1Range.isEmpty ? n[r] = es(s, e, t, o, l) : s.seq2Range.isEmpty && (n[r] = es(s.swap(), t, e, l, o).swap());
  }
  return n;
}
function es(e, t, n, r, i) {
  let a = 1;
  for (; e.seq1Range.start - a >= r.start && e.seq2Range.start - a >= i.start && n.isStronglyEqual(e.seq2Range.start - a, e.seq2Range.endExclusive - a) && a < 100; )
    a++;
  a--;
  let o = 0;
  for (; e.seq1Range.start + o < r.endExclusive && e.seq2Range.endExclusive + o < i.endExclusive && n.isStronglyEqual(e.seq2Range.start + o, e.seq2Range.endExclusive + o) && o < 100; )
    o++;
  if (a === 0 && o === 0)
    return e;
  let l = 0, u = -1;
  for (let c = -a; c <= o; c++) {
    const f = e.seq2Range.start + c, d = e.seq2Range.endExclusive + c, g = e.seq1Range.start + c, m = t.getBoundaryScore(g) + n.getBoundaryScore(f) + n.getBoundaryScore(d);
    m > u && (u = m, l = c);
  }
  return e.delta(l);
}
function Rl(e, t, n) {
  const r = [];
  for (const i of n) {
    const s = r[r.length - 1];
    if (!s) {
      r.push(i);
      continue;
    }
    i.seq1Range.start - s.seq1Range.endExclusive <= 2 || i.seq2Range.start - s.seq2Range.endExclusive <= 2 ? r[r.length - 1] = new re(s.seq1Range.join(i.seq1Range), s.seq2Range.join(i.seq2Range)) : r.push(i);
  }
  return r;
}
function Tl(e, t, n) {
  const r = [];
  let i;
  function s() {
    if (!i)
      return;
    const l = i.s1Range.length - i.deleted;
    i.s2Range.length - i.added, Math.max(i.deleted, i.added) + (i.count - 1) > l && r.push(new re(i.s1Range, i.s2Range)), i = void 0;
  }
  for (const l of n) {
    let u = function(m, p) {
      var v, x, b, y;
      if (!i || !i.s1Range.containsRange(m) || !i.s2Range.containsRange(p))
        if (i && !(i.s1Range.endExclusive < m.start && i.s2Range.endExclusive < p.start)) {
          const w = z.tryCreate(i.s1Range.endExclusive, m.start), S = z.tryCreate(i.s2Range.endExclusive, p.start);
          i.deleted += (v = w == null ? void 0 : w.length) !== null && v !== void 0 ? v : 0, i.added += (x = S == null ? void 0 : S.length) !== null && x !== void 0 ? x : 0, i.s1Range = i.s1Range.join(m), i.s2Range = i.s2Range.join(p);
        } else
          s(), i = { added: 0, deleted: 0, count: 0, s1Range: m, s2Range: p };
      const _ = m.intersect(l.seq1Range), N = p.intersect(l.seq2Range);
      i.count++, i.deleted += (b = _ == null ? void 0 : _.length) !== null && b !== void 0 ? b : 0, i.added += (y = N == null ? void 0 : N.length) !== null && y !== void 0 ? y : 0;
    };
    var o = u;
    const c = e.findWordContaining(l.seq1Range.start - 1), f = t.findWordContaining(l.seq2Range.start - 1), d = e.findWordContaining(l.seq1Range.endExclusive), g = t.findWordContaining(l.seq2Range.endExclusive);
    c && d && f && g && c.equals(d) && f.equals(g) ? u(c, f) : (c && f && u(c, f), d && g && u(d, g));
  }
  return s(), Pl(n, r);
}
function Pl(e, t) {
  const n = [];
  for (; e.length > 0 || t.length > 0; ) {
    const r = e[0], i = t[0];
    let s;
    r && (!i || r.seq1Range.start < i.seq1Range.start) ? s = e.shift() : s = t.shift(), n.length > 0 && n[n.length - 1].seq1Range.endExclusive >= s.seq1Range.start ? n[n.length - 1] = n[n.length - 1].join(s) : n.push(s);
  }
  return n;
}
function Fl(e, t, n) {
  let r = n;
  if (r.length === 0)
    return r;
  let i = 0, s;
  do {
    s = !1;
    const o = [
      r[0]
    ];
    for (let l = 1; l < r.length; l++) {
      let f = function(g, m) {
        const p = new z(c.seq1Range.endExclusive, u.seq1Range.start);
        return e.getText(p).replace(/\s/g, "").length <= 4 && (g.seq1Range.length + g.seq2Range.length > 5 || m.seq1Range.length + m.seq2Range.length > 5);
      };
      var a = f;
      const u = r[l], c = o[o.length - 1];
      f(c, u) ? (s = !0, o[o.length - 1] = o[o.length - 1].join(u)) : o.push(u);
    }
    r = o;
  } while (i++ < 10 && s);
  return r;
}
function Il(e, t, n) {
  let r = n;
  if (r.length === 0)
    return r;
  let i = 0, s;
  do {
    s = !1;
    const l = [
      r[0]
    ];
    for (let u = 1; u < r.length; u++) {
      let d = function(m, p) {
        const v = new z(f.seq1Range.endExclusive, c.seq1Range.start);
        if (e.countLinesIn(v) > 5 || v.length > 500)
          return !1;
        const b = e.getText(v).trim();
        if (b.length > 20 || b.split(/\r\n|\r|\n/).length > 1)
          return !1;
        const y = e.countLinesIn(m.seq1Range), _ = m.seq1Range.length, N = t.countLinesIn(m.seq2Range), w = m.seq2Range.length, S = e.countLinesIn(p.seq1Range), C = p.seq1Range.length, M = t.countLinesIn(p.seq2Range), I = p.seq2Range.length, D = 2 * 40 + 50;
        function L(E) {
          return Math.min(E, D);
        }
        return Math.pow(Math.pow(L(y * 40 + _), 1.5) + Math.pow(L(N * 40 + w), 1.5), 1.5) + Math.pow(Math.pow(L(S * 40 + C), 1.5) + Math.pow(L(M * 40 + I), 1.5), 1.5) > Math.pow(Math.pow(D, 1.5), 1.5) * 1.3;
      };
      var o = d;
      const c = r[u], f = l[l.length - 1];
      d(f, c) ? (s = !0, l[l.length - 1] = l[l.length - 1].join(c)) : l.push(c);
    }
    r = l;
  } while (i++ < 10 && s);
  const a = [];
  return Fo(r, (l, u, c) => {
    let f = u;
    function d(b) {
      return b.length > 0 && b.trim().length <= 3 && u.seq1Range.length + u.seq2Range.length > 100;
    }
    const g = e.extendToFullLines(u.seq1Range), m = e.getText(new z(g.start, u.seq1Range.start));
    d(m) && (f = f.deltaStart(-m.length));
    const p = e.getText(new z(u.seq1Range.endExclusive, g.endExclusive));
    d(p) && (f = f.deltaEnd(p.length));
    const v = re.fromOffsetPairs(l ? l.getEndExclusives() : Ie.zero, c ? c.getStarts() : Ie.max), x = f.intersect(v);
    a.push(x);
  }), a;
}
class ts {
  constructor(t, n) {
    this.trimmedHash = t, this.lines = n;
  }
  getElement(t) {
    return this.trimmedHash[t];
  }
  get length() {
    return this.trimmedHash.length;
  }
  getBoundaryScore(t) {
    const n = t === 0 ? 0 : ns(this.lines[t - 1]), r = t === this.lines.length ? 0 : ns(this.lines[t]);
    return 1e3 - (n + r);
  }
  getText(t) {
    return this.lines.slice(t.start, t.endExclusive).join(`
`);
  }
  isStronglyEqual(t, n) {
    return this.lines[t] === this.lines[n];
  }
}
function ns(e) {
  let t = 0;
  for (; t < e.length && (e.charCodeAt(t) === 32 || e.charCodeAt(t) === 9); )
    t++;
  return t;
}
class Dl {
  constructor() {
    this.dynamicProgrammingDiffing = new yl(), this.myersDiffingAlgorithm = new la();
  }
  computeDiff(t, n, r) {
    if (t.length <= 1 && Ro(t, n, (N, w) => N === w))
      return new gn([], [], !1);
    if (t.length === 1 && t[0].length === 0 || n.length === 1 && n[0].length === 0)
      return new gn([
        new Ue(new J(1, t.length + 1), new J(1, n.length + 1), [
          new $t(new ae(1, 1, t.length, t[0].length + 1), new ae(1, 1, n.length, n[0].length + 1))
        ])
      ], [], !1);
    const i = r.maxComputationTimeMs === 0 ? Wt.instance : new bl(r.maxComputationTimeMs), s = !r.ignoreTrimWhitespace, a = /* @__PURE__ */ new Map();
    function o(N) {
      let w = a.get(N);
      return w === void 0 && (w = a.size, a.set(N, w)), w;
    }
    const l = t.map((N) => o(N.trim())), u = n.map((N) => o(N.trim())), c = new ts(l, t), f = new ts(u, n), d = (() => c.length + f.length < 1700 ? this.dynamicProgrammingDiffing.compute(c, f, i, (N, w) => t[N] === n[w] ? n[w].length === 0 ? 0.1 : 1 + Math.log(1 + n[w].length) : 0.99) : this.myersDiffingAlgorithm.compute(c, f))();
    let g = d.diffs, m = d.hitTimeout;
    g = Ki(c, f, g), g = Fl(c, f, g);
    const p = [], v = (N) => {
      if (s)
        for (let w = 0; w < N; w++) {
          const S = x + w, C = b + w;
          if (t[S] !== n[C]) {
            const M = this.refineDiff(t, n, new re(new z(S, S + 1), new z(C, C + 1)), i, s);
            for (const I of M.mappings)
              p.push(I);
            M.hitTimeout && (m = !0);
          }
        }
    };
    let x = 0, b = 0;
    for (const N of g) {
      xn(() => N.seq1Range.start - x === N.seq2Range.start - b);
      const w = N.seq1Range.start - x;
      v(w), x = N.seq1Range.endExclusive, b = N.seq2Range.endExclusive;
      const S = this.refineDiff(t, n, N, i, s);
      S.hitTimeout && (m = !0);
      for (const C of S.mappings)
        p.push(C);
    }
    v(t.length - x);
    const y = rs(p, t, n);
    let _ = [];
    return r.computeMoves && (_ = this.computeMoves(y, t, n, l, u, i, s)), xn(() => {
      function N(S, C) {
        if (S.lineNumber < 1 || S.lineNumber > C.length)
          return !1;
        const M = C[S.lineNumber - 1];
        return !(S.column < 1 || S.column > M.length + 1);
      }
      function w(S, C) {
        return !(S.startLineNumber < 1 || S.startLineNumber > C.length + 1 || S.endLineNumberExclusive < 1 || S.endLineNumberExclusive > C.length + 1);
      }
      for (const S of y) {
        if (!S.innerChanges)
          return !1;
        for (const C of S.innerChanges)
          if (!(N(C.modifiedRange.getStartPosition(), n) && N(C.modifiedRange.getEndPosition(), n) && N(C.originalRange.getStartPosition(), t) && N(C.originalRange.getEndPosition(), t)))
            return !1;
        if (!w(S.modified, n) || !w(S.original, t))
          return !1;
      }
      return !0;
    }), new gn(y, _, m);
  }
  computeMoves(t, n, r, i, s, a, o) {
    return Ll(t, n, r, i, s, a).map((c) => {
      const f = this.refineDiff(n, r, new re(c.original.toOffsetRange(), c.modified.toOffsetRange()), a, o), d = rs(f.mappings, n, r, !0);
      return new fl(c, d);
    });
  }
  refineDiff(t, n, r, i, s) {
    const a = new wn(t, r.seq1Range, s), o = new wn(n, r.seq2Range, s), l = a.length + o.length < 500 ? this.dynamicProgrammingDiffing.compute(a, o, i) : this.myersDiffingAlgorithm.compute(a, o, i);
    let u = l.diffs;
    return u = Ki(a, o, u), u = Tl(a, o, u), u = Rl(a, o, u), u = Il(a, o, u), {
      mappings: u.map((f) => new $t(a.translateRange(f.seq1Range), o.translateRange(f.seq2Range))),
      hitTimeout: l.hitTimeout
    };
  }
}
function rs(e, t, n, r = !1) {
  const i = [];
  for (const s of To(e.map((a) => Vl(a, t, n)), (a, o) => a.original.overlapOrTouch(o.original) || a.modified.overlapOrTouch(o.modified))) {
    const a = s[0], o = s[s.length - 1];
    i.push(new Ue(a.original.join(o.original), a.modified.join(o.modified), s.map((l) => l.innerChanges[0])));
  }
  return xn(() => !r && i.length > 0 && i[0].original.startLineNumber !== i[0].modified.startLineNumber ? !1 : aa(i, (s, a) => a.original.startLineNumber - s.original.endLineNumberExclusive === a.modified.startLineNumber - s.modified.endLineNumberExclusive && // There has to be an unchanged line in between (otherwise both diffs should have been joined)
  s.original.endLineNumberExclusive < a.original.startLineNumber && s.modified.endLineNumberExclusive < a.modified.startLineNumber)), i;
}
function Vl(e, t, n) {
  let r = 0, i = 0;
  e.modifiedRange.endColumn === 1 && e.originalRange.endColumn === 1 && e.originalRange.startLineNumber + r <= e.originalRange.endLineNumber && e.modifiedRange.startLineNumber + r <= e.modifiedRange.endLineNumber && (i = -1), e.modifiedRange.startColumn - 1 >= n[e.modifiedRange.startLineNumber - 1].length && e.originalRange.startColumn - 1 >= t[e.originalRange.startLineNumber - 1].length && e.originalRange.startLineNumber <= e.originalRange.endLineNumber + i && e.modifiedRange.startLineNumber <= e.modifiedRange.endLineNumber + i && (r = 1);
  const s = new J(e.originalRange.startLineNumber + r, e.originalRange.endLineNumber + 1 + i), a = new J(e.modifiedRange.startLineNumber + r, e.modifiedRange.endLineNumber + 1 + i);
  return new Ue(s, a, [e]);
}
const is = {
  getLegacy: () => new gl(),
  getDefault: () => new Dl()
};
function nt(e, t) {
  const n = Math.pow(10, t);
  return Math.round(e * n) / n;
}
class oe {
  constructor(t, n, r, i = 1) {
    this._rgbaBrand = void 0, this.r = Math.min(255, Math.max(0, t)) | 0, this.g = Math.min(255, Math.max(0, n)) | 0, this.b = Math.min(255, Math.max(0, r)) | 0, this.a = nt(Math.max(Math.min(1, i), 0), 3);
  }
  static equals(t, n) {
    return t.r === n.r && t.g === n.g && t.b === n.b && t.a === n.a;
  }
}
class Le {
  constructor(t, n, r, i) {
    this._hslaBrand = void 0, this.h = Math.max(Math.min(360, t), 0) | 0, this.s = nt(Math.max(Math.min(1, n), 0), 3), this.l = nt(Math.max(Math.min(1, r), 0), 3), this.a = nt(Math.max(Math.min(1, i), 0), 3);
  }
  static equals(t, n) {
    return t.h === n.h && t.s === n.s && t.l === n.l && t.a === n.a;
  }
  /**
   * Converts an RGB color value to HSL. Conversion formula
   * adapted from http://en.wikipedia.org/wiki/HSL_color_space.
   * Assumes r, g, and b are contained in the set [0, 255] and
   * returns h in the set [0, 360], s, and l in the set [0, 1].
   */
  static fromRGBA(t) {
    const n = t.r / 255, r = t.g / 255, i = t.b / 255, s = t.a, a = Math.max(n, r, i), o = Math.min(n, r, i);
    let l = 0, u = 0;
    const c = (o + a) / 2, f = a - o;
    if (f > 0) {
      switch (u = Math.min(c <= 0.5 ? f / (2 * c) : f / (2 - 2 * c), 1), a) {
        case n:
          l = (r - i) / f + (r < i ? 6 : 0);
          break;
        case r:
          l = (i - n) / f + 2;
          break;
        case i:
          l = (n - r) / f + 4;
          break;
      }
      l *= 60, l = Math.round(l);
    }
    return new Le(l, u, c, s);
  }
  static _hue2rgb(t, n, r) {
    return r < 0 && (r += 1), r > 1 && (r -= 1), r < 1 / 6 ? t + (n - t) * 6 * r : r < 1 / 2 ? n : r < 2 / 3 ? t + (n - t) * (2 / 3 - r) * 6 : t;
  }
  /**
   * Converts an HSL color value to RGB. Conversion formula
   * adapted from http://en.wikipedia.org/wiki/HSL_color_space.
   * Assumes h in the set [0, 360] s, and l are contained in the set [0, 1] and
   * returns r, g, and b in the set [0, 255].
   */
  static toRGBA(t) {
    const n = t.h / 360, { s: r, l: i, a: s } = t;
    let a, o, l;
    if (r === 0)
      a = o = l = i;
    else {
      const u = i < 0.5 ? i * (1 + r) : i + r - i * r, c = 2 * i - u;
      a = Le._hue2rgb(c, u, n + 1 / 3), o = Le._hue2rgb(c, u, n), l = Le._hue2rgb(c, u, n - 1 / 3);
    }
    return new oe(Math.round(a * 255), Math.round(o * 255), Math.round(l * 255), s);
  }
}
class bt {
  constructor(t, n, r, i) {
    this._hsvaBrand = void 0, this.h = Math.max(Math.min(360, t), 0) | 0, this.s = nt(Math.max(Math.min(1, n), 0), 3), this.v = nt(Math.max(Math.min(1, r), 0), 3), this.a = nt(Math.max(Math.min(1, i), 0), 3);
  }
  static equals(t, n) {
    return t.h === n.h && t.s === n.s && t.v === n.v && t.a === n.a;
  }
  // from http://www.rapidtables.com/convert/color/rgb-to-hsv.htm
  static fromRGBA(t) {
    const n = t.r / 255, r = t.g / 255, i = t.b / 255, s = Math.max(n, r, i), a = Math.min(n, r, i), o = s - a, l = s === 0 ? 0 : o / s;
    let u;
    return o === 0 ? u = 0 : s === n ? u = ((r - i) / o % 6 + 6) % 6 : s === r ? u = (i - n) / o + 2 : u = (n - r) / o + 4, new bt(Math.round(u * 60), l, s, t.a);
  }
  // from http://www.rapidtables.com/convert/color/hsv-to-rgb.htm
  static toRGBA(t) {
    const { h: n, s: r, v: i, a: s } = t, a = i * r, o = a * (1 - Math.abs(n / 60 % 2 - 1)), l = i - a;
    let [u, c, f] = [0, 0, 0];
    return n < 60 ? (u = a, c = o) : n < 120 ? (u = o, c = a) : n < 180 ? (c = a, f = o) : n < 240 ? (c = o, f = a) : n < 300 ? (u = o, f = a) : n <= 360 && (u = a, f = o), u = Math.round((u + l) * 255), c = Math.round((c + l) * 255), f = Math.round((f + l) * 255), new oe(u, c, f, s);
  }
}
let se = class Se {
  static fromHex(t) {
    return Se.Format.CSS.parseHex(t) || Se.red;
  }
  static equals(t, n) {
    return !t && !n ? !0 : !t || !n ? !1 : t.equals(n);
  }
  get hsla() {
    return this._hsla ? this._hsla : Le.fromRGBA(this.rgba);
  }
  get hsva() {
    return this._hsva ? this._hsva : bt.fromRGBA(this.rgba);
  }
  constructor(t) {
    if (t)
      if (t instanceof oe)
        this.rgba = t;
      else if (t instanceof Le)
        this._hsla = t, this.rgba = Le.toRGBA(t);
      else if (t instanceof bt)
        this._hsva = t, this.rgba = bt.toRGBA(t);
      else
        throw new Error("Invalid color ctor argument");
    else
      throw new Error("Color needs a value");
  }
  equals(t) {
    return !!t && oe.equals(this.rgba, t.rgba) && Le.equals(this.hsla, t.hsla) && bt.equals(this.hsva, t.hsva);
  }
  /**
   * http://www.w3.org/TR/WCAG20/#relativeluminancedef
   * Returns the number in the set [0, 1]. O => Darkest Black. 1 => Lightest white.
   */
  getRelativeLuminance() {
    const t = Se._relativeLuminanceForComponent(this.rgba.r), n = Se._relativeLuminanceForComponent(this.rgba.g), r = Se._relativeLuminanceForComponent(this.rgba.b), i = 0.2126 * t + 0.7152 * n + 0.0722 * r;
    return nt(i, 4);
  }
  static _relativeLuminanceForComponent(t) {
    const n = t / 255;
    return n <= 0.03928 ? n / 12.92 : Math.pow((n + 0.055) / 1.055, 2.4);
  }
  /**
   *	http://24ways.org/2010/calculating-color-contrast
   *  Return 'true' if lighter color otherwise 'false'
   */
  isLighter() {
    return (this.rgba.r * 299 + this.rgba.g * 587 + this.rgba.b * 114) / 1e3 >= 128;
  }
  isLighterThan(t) {
    const n = this.getRelativeLuminance(), r = t.getRelativeLuminance();
    return n > r;
  }
  isDarkerThan(t) {
    const n = this.getRelativeLuminance(), r = t.getRelativeLuminance();
    return n < r;
  }
  lighten(t) {
    return new Se(new Le(this.hsla.h, this.hsla.s, this.hsla.l + this.hsla.l * t, this.hsla.a));
  }
  darken(t) {
    return new Se(new Le(this.hsla.h, this.hsla.s, this.hsla.l - this.hsla.l * t, this.hsla.a));
  }
  transparent(t) {
    const { r: n, g: r, b: i, a: s } = this.rgba;
    return new Se(new oe(n, r, i, s * t));
  }
  isTransparent() {
    return this.rgba.a === 0;
  }
  isOpaque() {
    return this.rgba.a === 1;
  }
  opposite() {
    return new Se(new oe(255 - this.rgba.r, 255 - this.rgba.g, 255 - this.rgba.b, this.rgba.a));
  }
  makeOpaque(t) {
    if (this.isOpaque() || t.rgba.a !== 1)
      return this;
    const { r: n, g: r, b: i, a: s } = this.rgba;
    return new Se(new oe(t.rgba.r - s * (t.rgba.r - n), t.rgba.g - s * (t.rgba.g - r), t.rgba.b - s * (t.rgba.b - i), 1));
  }
  toString() {
    return this._toString || (this._toString = Se.Format.CSS.format(this)), this._toString;
  }
  static getLighterColor(t, n, r) {
    if (t.isLighterThan(n))
      return t;
    r = r || 0.5;
    const i = t.getRelativeLuminance(), s = n.getRelativeLuminance();
    return r = r * (s - i) / s, t.lighten(r);
  }
  static getDarkerColor(t, n, r) {
    if (t.isDarkerThan(n))
      return t;
    r = r || 0.5;
    const i = t.getRelativeLuminance(), s = n.getRelativeLuminance();
    return r = r * (i - s) / i, t.darken(r);
  }
};
se.white = new se(new oe(255, 255, 255, 1));
se.black = new se(new oe(0, 0, 0, 1));
se.red = new se(new oe(255, 0, 0, 1));
se.blue = new se(new oe(0, 0, 255, 1));
se.green = new se(new oe(0, 255, 0, 1));
se.cyan = new se(new oe(0, 255, 255, 1));
se.lightgrey = new se(new oe(211, 211, 211, 1));
se.transparent = new se(new oe(0, 0, 0, 0));
(function(e) {
  (function(t) {
    (function(n) {
      function r(g) {
        return g.rgba.a === 1 ? `rgb(${g.rgba.r}, ${g.rgba.g}, ${g.rgba.b})` : e.Format.CSS.formatRGBA(g);
      }
      n.formatRGB = r;
      function i(g) {
        return `rgba(${g.rgba.r}, ${g.rgba.g}, ${g.rgba.b}, ${+g.rgba.a.toFixed(2)})`;
      }
      n.formatRGBA = i;
      function s(g) {
        return g.hsla.a === 1 ? `hsl(${g.hsla.h}, ${(g.hsla.s * 100).toFixed(2)}%, ${(g.hsla.l * 100).toFixed(2)}%)` : e.Format.CSS.formatHSLA(g);
      }
      n.formatHSL = s;
      function a(g) {
        return `hsla(${g.hsla.h}, ${(g.hsla.s * 100).toFixed(2)}%, ${(g.hsla.l * 100).toFixed(2)}%, ${g.hsla.a.toFixed(2)})`;
      }
      n.formatHSLA = a;
      function o(g) {
        const m = g.toString(16);
        return m.length !== 2 ? "0" + m : m;
      }
      function l(g) {
        return `#${o(g.rgba.r)}${o(g.rgba.g)}${o(g.rgba.b)}`;
      }
      n.formatHex = l;
      function u(g, m = !1) {
        return m && g.rgba.a === 1 ? e.Format.CSS.formatHex(g) : `#${o(g.rgba.r)}${o(g.rgba.g)}${o(g.rgba.b)}${o(Math.round(g.rgba.a * 255))}`;
      }
      n.formatHexA = u;
      function c(g) {
        return g.isOpaque() ? e.Format.CSS.formatHex(g) : e.Format.CSS.formatRGBA(g);
      }
      n.format = c;
      function f(g) {
        const m = g.length;
        if (m === 0 || g.charCodeAt(0) !== 35)
          return null;
        if (m === 7) {
          const p = 16 * d(g.charCodeAt(1)) + d(g.charCodeAt(2)), v = 16 * d(g.charCodeAt(3)) + d(g.charCodeAt(4)), x = 16 * d(g.charCodeAt(5)) + d(g.charCodeAt(6));
          return new e(new oe(p, v, x, 1));
        }
        if (m === 9) {
          const p = 16 * d(g.charCodeAt(1)) + d(g.charCodeAt(2)), v = 16 * d(g.charCodeAt(3)) + d(g.charCodeAt(4)), x = 16 * d(g.charCodeAt(5)) + d(g.charCodeAt(6)), b = 16 * d(g.charCodeAt(7)) + d(g.charCodeAt(8));
          return new e(new oe(p, v, x, b / 255));
        }
        if (m === 4) {
          const p = d(g.charCodeAt(1)), v = d(g.charCodeAt(2)), x = d(g.charCodeAt(3));
          return new e(new oe(16 * p + p, 16 * v + v, 16 * x + x));
        }
        if (m === 5) {
          const p = d(g.charCodeAt(1)), v = d(g.charCodeAt(2)), x = d(g.charCodeAt(3)), b = d(g.charCodeAt(4));
          return new e(new oe(16 * p + p, 16 * v + v, 16 * x + x, (16 * b + b) / 255));
        }
        return null;
      }
      n.parseHex = f;
      function d(g) {
        switch (g) {
          case 48:
            return 0;
          case 49:
            return 1;
          case 50:
            return 2;
          case 51:
            return 3;
          case 52:
            return 4;
          case 53:
            return 5;
          case 54:
            return 6;
          case 55:
            return 7;
          case 56:
            return 8;
          case 57:
            return 9;
          case 97:
            return 10;
          case 65:
            return 10;
          case 98:
            return 11;
          case 66:
            return 11;
          case 99:
            return 12;
          case 67:
            return 12;
          case 100:
            return 13;
          case 68:
            return 13;
          case 101:
            return 14;
          case 69:
            return 14;
          case 102:
            return 15;
          case 70:
            return 15;
        }
        return 0;
      }
    })(t.CSS || (t.CSS = {}));
  })(e.Format || (e.Format = {}));
})(se || (se = {}));
function ua(e) {
  const t = [];
  for (const n of e) {
    const r = Number(n);
    (r || r === 0 && n.replace(/\s/g, "") !== "") && t.push(r);
  }
  return t;
}
function Fr(e, t, n, r) {
  return {
    red: e / 255,
    blue: n / 255,
    green: t / 255,
    alpha: r
  };
}
function Rt(e, t) {
  const n = t.index, r = t[0].length;
  if (!n)
    return;
  const i = e.positionAt(n);
  return {
    startLineNumber: i.lineNumber,
    startColumn: i.column,
    endLineNumber: i.lineNumber,
    endColumn: i.column + r
  };
}
function Ol(e, t) {
  if (!e)
    return;
  const n = se.Format.CSS.parseHex(t);
  if (n)
    return {
      range: e,
      color: Fr(n.rgba.r, n.rgba.g, n.rgba.b, n.rgba.a)
    };
}
function ss(e, t, n) {
  if (!e || t.length !== 1)
    return;
  const i = t[0].values(), s = ua(i);
  return {
    range: e,
    color: Fr(s[0], s[1], s[2], n ? s[3] : 1)
  };
}
function as(e, t, n) {
  if (!e || t.length !== 1)
    return;
  const i = t[0].values(), s = ua(i), a = new se(new Le(s[0], s[1] / 100, s[2] / 100, n ? s[3] : 1));
  return {
    range: e,
    color: Fr(a.rgba.r, a.rgba.g, a.rgba.b, a.rgba.a)
  };
}
function Tt(e, t) {
  return typeof e == "string" ? [...e.matchAll(t)] : e.findMatches(t);
}
function jl(e) {
  const t = [], r = Tt(e, /\b(rgb|rgba|hsl|hsla)(\([0-9\s,.\%]*\))|(#)([A-Fa-f0-9]{3})\b|(#)([A-Fa-f0-9]{4})\b|(#)([A-Fa-f0-9]{6})\b|(#)([A-Fa-f0-9]{8})\b/gm);
  if (r.length > 0)
    for (const i of r) {
      const s = i.filter((u) => u !== void 0), a = s[1], o = s[2];
      if (!o)
        continue;
      let l;
      if (a === "rgb") {
        const u = /^\(\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*\)$/gm;
        l = ss(Rt(e, i), Tt(o, u), !1);
      } else if (a === "rgba") {
        const u = /^\(\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(0[.][0-9]+|[.][0-9]+|[01][.]|[01])\s*\)$/gm;
        l = ss(Rt(e, i), Tt(o, u), !0);
      } else if (a === "hsl") {
        const u = /^\(\s*(36[0]|3[0-5][0-9]|[12][0-9][0-9]|[1-9]?[0-9])\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*\)$/gm;
        l = as(Rt(e, i), Tt(o, u), !1);
      } else if (a === "hsla") {
        const u = /^\(\s*(36[0]|3[0-5][0-9]|[12][0-9][0-9]|[1-9]?[0-9])\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*,\s*(0[.][0-9]+|[.][0-9]+|[01][.]|[01])\s*\)$/gm;
        l = as(Rt(e, i), Tt(o, u), !0);
      } else
        a === "#" && (l = Ol(Rt(e, i), a + o));
      l && t.push(l);
    }
  return t;
}
function ql(e) {
  return !e || typeof e.getValue != "function" || typeof e.positionAt != "function" ? [] : jl(e);
}
var Ge = globalThis && globalThis.__awaiter || function(e, t, n, r) {
  function i(s) {
    return s instanceof n ? s : new n(function(a) {
      a(s);
    });
  }
  return new (n || (n = Promise))(function(s, a) {
    function o(c) {
      try {
        u(r.next(c));
      } catch (f) {
        a(f);
      }
    }
    function l(c) {
      try {
        u(r.throw(c));
      } catch (f) {
        a(f);
      }
    }
    function u(c) {
      c.done ? s(c.value) : i(c.value).then(o, l);
    }
    u((r = r.apply(e, t || [])).next());
  });
};
class Bl extends jo {
  get uri() {
    return this._uri;
  }
  get eol() {
    return this._eol;
  }
  getValue() {
    return this.getText();
  }
  findMatches(t) {
    const n = [];
    for (let r = 0; r < this._lines.length; r++) {
      const i = this._lines[r], s = this.offsetAt(new Oe(r + 1, 1)), a = i.matchAll(t);
      for (const o of a)
        (o.index || o.index === 0) && (o.index = o.index + s), n.push(o);
    }
    return n;
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
  getWordAtPosition(t, n) {
    const r = Rr(t.column, Uo(n), this._lines[t.lineNumber - 1], 0);
    return r ? new ae(t.lineNumber, r.startColumn, t.lineNumber, r.endColumn) : null;
  }
  words(t) {
    const n = this._lines, r = this._wordenize.bind(this);
    let i = 0, s = "", a = 0, o = [];
    return {
      *[Symbol.iterator]() {
        for (; ; )
          if (a < o.length) {
            const l = s.substring(o[a].start, o[a].end);
            a += 1, yield l;
          } else if (i < n.length)
            s = n[i], o = r(s, t), a = 0, i += 1;
          else
            break;
      }
    };
  }
  getLineWords(t, n) {
    const r = this._lines[t - 1], i = this._wordenize(r, n), s = [];
    for (const a of i)
      s.push({
        word: r.substring(a.start, a.end),
        startColumn: a.start + 1,
        endColumn: a.end + 1
      });
    return s;
  }
  _wordenize(t, n) {
    const r = [];
    let i;
    for (n.lastIndex = 0; (i = n.exec(t)) && i[0].length !== 0; )
      r.push({ start: i.index, end: i.index + i[0].length });
    return r;
  }
  getValueInRange(t) {
    if (t = this._validateRange(t), t.startLineNumber === t.endLineNumber)
      return this._lines[t.startLineNumber - 1].substring(t.startColumn - 1, t.endColumn - 1);
    const n = this._eol, r = t.startLineNumber - 1, i = t.endLineNumber - 1, s = [];
    s.push(this._lines[r].substring(t.startColumn - 1));
    for (let a = r + 1; a < i; a++)
      s.push(this._lines[a]);
    return s.push(this._lines[i].substring(0, t.endColumn - 1)), s.join(n);
  }
  offsetAt(t) {
    return t = this._validatePosition(t), this._ensureLineStarts(), this._lineStarts.getPrefixSum(t.lineNumber - 2) + (t.column - 1);
  }
  positionAt(t) {
    t = Math.floor(t), t = Math.max(0, t), this._ensureLineStarts();
    const n = this._lineStarts.getIndexOf(t), r = this._lines[n.index].length;
    return {
      lineNumber: 1 + n.index,
      column: 1 + Math.min(n.remainder, r)
    };
  }
  _validateRange(t) {
    const n = this._validatePosition({ lineNumber: t.startLineNumber, column: t.startColumn }), r = this._validatePosition({ lineNumber: t.endLineNumber, column: t.endColumn });
    return n.lineNumber !== t.startLineNumber || n.column !== t.startColumn || r.lineNumber !== t.endLineNumber || r.column !== t.endColumn ? {
      startLineNumber: n.lineNumber,
      startColumn: n.column,
      endLineNumber: r.lineNumber,
      endColumn: r.column
    } : t;
  }
  _validatePosition(t) {
    if (!Oe.isIPosition(t))
      throw new Error("bad position");
    let { lineNumber: n, column: r } = t, i = !1;
    if (n < 1)
      n = 1, r = 1, i = !0;
    else if (n > this._lines.length)
      n = this._lines.length, r = this._lines[n - 1].length + 1, i = !0;
    else {
      const s = this._lines[n - 1].length + 1;
      r < 1 ? (r = 1, i = !0) : r > s && (r = s, i = !0);
    }
    return i ? { lineNumber: n, column: r } : t;
  }
}
class at {
  constructor(t, n) {
    this._host = t, this._models = /* @__PURE__ */ Object.create(null), this._foreignModuleFactory = n, this._foreignModule = null;
  }
  dispose() {
    this._models = /* @__PURE__ */ Object.create(null);
  }
  _getModel(t) {
    return this._models[t];
  }
  _getModels() {
    const t = [];
    return Object.keys(this._models).forEach((n) => t.push(this._models[n])), t;
  }
  acceptNewModel(t) {
    this._models[t.url] = new Bl(Mr.parse(t.url), t.lines, t.EOL, t.versionId);
  }
  acceptModelChanged(t, n) {
    if (!this._models[t])
      return;
    this._models[t].onEvents(n);
  }
  acceptRemovedModel(t) {
    this._models[t] && delete this._models[t];
  }
  computeUnicodeHighlights(t, n, r) {
    return Ge(this, void 0, void 0, function* () {
      const i = this._getModel(t);
      return i ? ul.computeUnicodeHighlights(i, n, r) : { ranges: [], hasMore: !1, ambiguousCharacterCount: 0, invisibleCharacterCount: 0, nonBasicAsciiCharacterCount: 0 };
    });
  }
  // ---- BEGIN diff --------------------------------------------------------------------------
  computeDiff(t, n, r, i) {
    return Ge(this, void 0, void 0, function* () {
      const s = this._getModel(t), a = this._getModel(n);
      return !s || !a ? null : at.computeDiff(s, a, r, i);
    });
  }
  static computeDiff(t, n, r, i) {
    const s = i === "advanced" ? is.getDefault() : is.getLegacy(), a = t.getLinesContent(), o = n.getLinesContent(), l = s.computeDiff(a, o, r), u = l.changes.length > 0 ? !1 : this._modelsAreIdentical(t, n);
    function c(f) {
      return f.map((d) => {
        var g;
        return [d.original.startLineNumber, d.original.endLineNumberExclusive, d.modified.startLineNumber, d.modified.endLineNumberExclusive, (g = d.innerChanges) === null || g === void 0 ? void 0 : g.map((m) => [
          m.originalRange.startLineNumber,
          m.originalRange.startColumn,
          m.originalRange.endLineNumber,
          m.originalRange.endColumn,
          m.modifiedRange.startLineNumber,
          m.modifiedRange.startColumn,
          m.modifiedRange.endLineNumber,
          m.modifiedRange.endColumn
        ])];
      });
    }
    return {
      identical: u,
      quitEarly: l.hitTimeout,
      changes: c(l.changes),
      moves: l.moves.map((f) => [
        f.lineRangeMapping.original.startLineNumber,
        f.lineRangeMapping.original.endLineNumberExclusive,
        f.lineRangeMapping.modified.startLineNumber,
        f.lineRangeMapping.modified.endLineNumberExclusive,
        c(f.changes)
      ])
    };
  }
  static _modelsAreIdentical(t, n) {
    const r = t.getLineCount(), i = n.getLineCount();
    if (r !== i)
      return !1;
    for (let s = 1; s <= r; s++) {
      const a = t.getLineContent(s), o = n.getLineContent(s);
      if (a !== o)
        return !1;
    }
    return !0;
  }
  computeMoreMinimalEdits(t, n, r) {
    return Ge(this, void 0, void 0, function* () {
      const i = this._getModel(t);
      if (!i)
        return n;
      const s = [];
      let a;
      n = n.slice(0).sort((l, u) => {
        if (l.range && u.range)
          return ae.compareRangesUsingStarts(l.range, u.range);
        const c = l.range ? 0 : 1, f = u.range ? 0 : 1;
        return c - f;
      });
      let o = 0;
      for (let l = 1; l < n.length; l++)
        ae.getEndPosition(n[o].range).equals(ae.getStartPosition(n[l].range)) ? (n[o].range = ae.fromPositions(ae.getStartPosition(n[o].range), ae.getEndPosition(n[l].range)), n[o].text += n[l].text) : (o++, n[o] = n[l]);
      n.length = o + 1;
      for (let { range: l, text: u, eol: c } of n) {
        if (typeof c == "number" && (a = c), ae.isEmpty(l) && !u)
          continue;
        const f = i.getValueInRange(l);
        if (u = u.replace(/\r\n|\n|\r/g, i.eol), f === u)
          continue;
        if (Math.max(u.length, f.length) > at._diffLimit) {
          s.push({ range: l, text: u });
          continue;
        }
        const d = ho(f, u, r), g = i.offsetAt(ae.lift(l).getStartPosition());
        for (const m of d) {
          const p = i.positionAt(g + m.originalStart), v = i.positionAt(g + m.originalStart + m.originalLength), x = {
            text: u.substr(m.modifiedStart, m.modifiedLength),
            range: { startLineNumber: p.lineNumber, startColumn: p.column, endLineNumber: v.lineNumber, endColumn: v.column }
          };
          i.getValueInRange(x.range) !== x.text && s.push(x);
        }
      }
      return typeof a == "number" && s.push({ eol: a, text: "", range: { startLineNumber: 0, startColumn: 0, endLineNumber: 0, endColumn: 0 } }), s;
    });
  }
  // ---- END minimal edits ---------------------------------------------------------------
  computeLinks(t) {
    return Ge(this, void 0, void 0, function* () {
      const n = this._getModel(t);
      return n ? Jo(n) : null;
    });
  }
  // --- BEGIN default document colors -----------------------------------------------------------
  computeDefaultDocumentColors(t) {
    return Ge(this, void 0, void 0, function* () {
      const n = this._getModel(t);
      return n ? ql(n) : null;
    });
  }
  textualSuggest(t, n, r, i) {
    return Ge(this, void 0, void 0, function* () {
      const s = new Mn(), a = new RegExp(r, i), o = /* @__PURE__ */ new Set();
      e:
        for (const l of t) {
          const u = this._getModel(l);
          if (u) {
            for (const c of u.words(a))
              if (!(c === n || !isNaN(Number(c))) && (o.add(c), o.size > at._suggestionsLimit))
                break e;
          }
        }
      return { words: Array.from(o), duration: s.elapsed() };
    });
  }
  // ---- END suggest --------------------------------------------------------------------------
  //#region -- word ranges --
  computeWordRanges(t, n, r, i) {
    return Ge(this, void 0, void 0, function* () {
      const s = this._getModel(t);
      if (!s)
        return /* @__PURE__ */ Object.create(null);
      const a = new RegExp(r, i), o = /* @__PURE__ */ Object.create(null);
      for (let l = n.startLineNumber; l < n.endLineNumber; l++) {
        const u = s.getLineWords(l, a);
        for (const c of u) {
          if (!isNaN(Number(c.word)))
            continue;
          let f = o[c.word];
          f || (f = [], o[c.word] = f), f.push({
            startLineNumber: l,
            startColumn: c.startColumn,
            endLineNumber: l,
            endColumn: c.endColumn
          });
        }
      }
      return o;
    });
  }
  //#endregion
  navigateValueSet(t, n, r, i, s) {
    return Ge(this, void 0, void 0, function* () {
      const a = this._getModel(t);
      if (!a)
        return null;
      const o = new RegExp(i, s);
      n.startColumn === n.endColumn && (n = {
        startLineNumber: n.startLineNumber,
        startColumn: n.startColumn,
        endLineNumber: n.endLineNumber,
        endColumn: n.endColumn + 1
      });
      const l = a.getValueInRange(n), u = a.getWordAtPosition({ lineNumber: n.startLineNumber, column: n.startColumn }, o);
      if (!u)
        return null;
      const c = a.getValueInRange(u);
      return rr.INSTANCE.navigateValueSet(n, l, u, c, r);
    });
  }
  // ---- BEGIN foreign module support --------------------------------------------------------------------------
  loadForeignModule(t, n, r) {
    const a = {
      host: Oa(r, (o, l) => this._host.fhr(o, l)),
      getMirrorModels: () => this._getModels()
    };
    return this._foreignModuleFactory ? (this._foreignModule = this._foreignModuleFactory(a, n), Promise.resolve(Qn(this._foreignModule))) : Promise.reject(new Error("Unexpected usage"));
  }
  // foreign method request
  fmr(t, n) {
    if (!this._foreignModule || typeof this._foreignModule[t] != "function")
      return Promise.reject(new Error("Missing requestHandler or method: " + t));
    try {
      return Promise.resolve(this._foreignModule[t].apply(this._foreignModule, n));
    } catch (r) {
      return Promise.reject(r);
    }
  }
}
at._diffLimit = 1e5;
at._suggestionsLimit = 1e4;
typeof importScripts == "function" && (globalThis.monaco = rl());
let mr = !1;
function ca(e) {
  if (mr)
    return;
  mr = !0;
  const t = new co((n) => {
    globalThis.postMessage(n);
  }, (n) => new at(n, e));
  globalThis.onmessage = (n) => {
    t.onmessage(n.data);
  };
}
globalThis.onmessage = (e) => {
  mr || ca(null);
};
/*!-----------------------------------------------------------------------------
 * Copyright (c) Microsoft Corporation. All rights reserved.
 * Version: 0.44.0(3e047efd345ff102c8c61b5398fb30845aaac166)
 * Released under the MIT license
 * https://github.com/microsoft/monaco-editor/blob/main/LICENSE.txt
 *-----------------------------------------------------------------------------*/
function Ir(e, t) {
  t === void 0 && (t = !1);
  var n = e.length, r = 0, i = "", s = 0, a = 16, o = 0, l = 0, u = 0, c = 0, f = 0;
  function d(y, _) {
    for (var N = 0, w = 0; N < y || !_; ) {
      var S = e.charCodeAt(r);
      if (S >= 48 && S <= 57)
        w = w * 16 + S - 48;
      else if (S >= 65 && S <= 70)
        w = w * 16 + S - 65 + 10;
      else if (S >= 97 && S <= 102)
        w = w * 16 + S - 97 + 10;
      else
        break;
      r++, N++;
    }
    return N < y && (w = -1), w;
  }
  function g(y) {
    r = y, i = "", s = 0, a = 16, f = 0;
  }
  function m() {
    var y = r;
    if (e.charCodeAt(r) === 48)
      r++;
    else
      for (r++; r < e.length && mt(e.charCodeAt(r)); )
        r++;
    if (r < e.length && e.charCodeAt(r) === 46)
      if (r++, r < e.length && mt(e.charCodeAt(r)))
        for (r++; r < e.length && mt(e.charCodeAt(r)); )
          r++;
      else
        return f = 3, e.substring(y, r);
    var _ = r;
    if (r < e.length && (e.charCodeAt(r) === 69 || e.charCodeAt(r) === 101))
      if (r++, (r < e.length && e.charCodeAt(r) === 43 || e.charCodeAt(r) === 45) && r++, r < e.length && mt(e.charCodeAt(r))) {
        for (r++; r < e.length && mt(e.charCodeAt(r)); )
          r++;
        _ = r;
      } else
        f = 3;
    return e.substring(y, _);
  }
  function p() {
    for (var y = "", _ = r; ; ) {
      if (r >= n) {
        y += e.substring(_, r), f = 2;
        break;
      }
      var N = e.charCodeAt(r);
      if (N === 34) {
        y += e.substring(_, r), r++;
        break;
      }
      if (N === 92) {
        if (y += e.substring(_, r), r++, r >= n) {
          f = 2;
          break;
        }
        var w = e.charCodeAt(r++);
        switch (w) {
          case 34:
            y += '"';
            break;
          case 92:
            y += "\\";
            break;
          case 47:
            y += "/";
            break;
          case 98:
            y += "\b";
            break;
          case 102:
            y += "\f";
            break;
          case 110:
            y += `
`;
            break;
          case 114:
            y += "\r";
            break;
          case 116:
            y += "	";
            break;
          case 117:
            var S = d(4, !0);
            S >= 0 ? y += String.fromCharCode(S) : f = 4;
            break;
          default:
            f = 5;
        }
        _ = r;
        continue;
      }
      if (N >= 0 && N <= 31)
        if (Pt(N)) {
          y += e.substring(_, r), f = 2;
          break;
        } else
          f = 6;
      r++;
    }
    return y;
  }
  function v() {
    if (i = "", f = 0, s = r, l = o, c = u, r >= n)
      return s = n, a = 17;
    var y = e.charCodeAt(r);
    if (Bn(y)) {
      do
        r++, i += String.fromCharCode(y), y = e.charCodeAt(r);
      while (Bn(y));
      return a = 15;
    }
    if (Pt(y))
      return r++, i += String.fromCharCode(y), y === 13 && e.charCodeAt(r) === 10 && (r++, i += `
`), o++, u = r, a = 14;
    switch (y) {
      case 123:
        return r++, a = 1;
      case 125:
        return r++, a = 2;
      case 91:
        return r++, a = 3;
      case 93:
        return r++, a = 4;
      case 58:
        return r++, a = 6;
      case 44:
        return r++, a = 5;
      case 34:
        return r++, i = p(), a = 10;
      case 47:
        var _ = r - 1;
        if (e.charCodeAt(r + 1) === 47) {
          for (r += 2; r < n && !Pt(e.charCodeAt(r)); )
            r++;
          return i = e.substring(_, r), a = 12;
        }
        if (e.charCodeAt(r + 1) === 42) {
          r += 2;
          for (var N = n - 1, w = !1; r < N; ) {
            var S = e.charCodeAt(r);
            if (S === 42 && e.charCodeAt(r + 1) === 47) {
              r += 2, w = !0;
              break;
            }
            r++, Pt(S) && (S === 13 && e.charCodeAt(r) === 10 && r++, o++, u = r);
          }
          return w || (r++, f = 1), i = e.substring(_, r), a = 13;
        }
        return i += String.fromCharCode(y), r++, a = 16;
      case 45:
        if (i += String.fromCharCode(y), r++, r === n || !mt(e.charCodeAt(r)))
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
        return i += m(), a = 11;
      default:
        for (; r < n && x(y); )
          r++, y = e.charCodeAt(r);
        if (s !== r) {
          switch (i = e.substring(s, r), i) {
            case "true":
              return a = 8;
            case "false":
              return a = 9;
            case "null":
              return a = 7;
          }
          return a = 16;
        }
        return i += String.fromCharCode(y), r++, a = 16;
    }
  }
  function x(y) {
    if (Bn(y) || Pt(y))
      return !1;
    switch (y) {
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
  function b() {
    var y;
    do
      y = v();
    while (y >= 12 && y <= 15);
    return y;
  }
  return {
    setPosition: g,
    getPosition: function() {
      return r;
    },
    scan: t ? b : v,
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
      return r - s;
    },
    getTokenStartLine: function() {
      return l;
    },
    getTokenStartCharacter: function() {
      return s - c;
    },
    getTokenError: function() {
      return f;
    }
  };
}
function Bn(e) {
  return e === 32 || e === 9 || e === 11 || e === 12 || e === 160 || e === 5760 || e >= 8192 && e <= 8203 || e === 8239 || e === 8287 || e === 12288 || e === 65279;
}
function Pt(e) {
  return e === 10 || e === 13 || e === 8232 || e === 8233;
}
function mt(e) {
  return e >= 48 && e <= 57;
}
function Ul(e, t, n) {
  var r, i, s, a, o;
  if (t) {
    for (a = t.offset, o = a + t.length, s = a; s > 0 && !os(e, s - 1); )
      s--;
    for (var l = o; l < e.length && !os(e, l); )
      l++;
    i = e.substring(s, l), r = $l(i, n);
  } else
    i = e, r = 0, s = 0, a = 0, o = e.length;
  var u = Wl(n, e), c = !1, f = 0, d;
  n.insertSpaces ? d = Un(" ", n.tabSize || 4) : d = "	";
  var g = Ir(i, !1), m = !1;
  function p() {
    return u + Un(d, r + f);
  }
  function v() {
    var L = g.scan();
    for (c = !1; L === 15 || L === 14; )
      c = c || L === 14, L = g.scan();
    return m = L === 16 || g.getTokenError() !== 0, L;
  }
  var x = [];
  function b(L, E, P) {
    !m && (!t || E < o && P > a) && e.substring(E, P) !== L && x.push({ offset: E, length: P - E, content: L });
  }
  var y = v();
  if (y !== 17) {
    var _ = g.getTokenOffset() + s, N = Un(d, r);
    b(N, s, _);
  }
  for (; y !== 17; ) {
    for (var w = g.getTokenOffset() + g.getTokenLength() + s, S = v(), C = "", M = !1; !c && (S === 12 || S === 13); ) {
      var I = g.getTokenOffset() + s;
      b(" ", w, I), w = g.getTokenOffset() + g.getTokenLength() + s, M = S === 12, C = M ? p() : "", S = v();
    }
    if (S === 2)
      y !== 1 && (f--, C = p());
    else if (S === 4)
      y !== 3 && (f--, C = p());
    else {
      switch (y) {
        case 3:
        case 1:
          f++, C = p();
          break;
        case 5:
        case 12:
          C = p();
          break;
        case 13:
          c ? C = p() : M || (C = " ");
          break;
        case 6:
          M || (C = " ");
          break;
        case 10:
          if (S === 6) {
            M || (C = "");
            break;
          }
        case 7:
        case 8:
        case 9:
        case 11:
        case 2:
        case 4:
          S === 12 || S === 13 ? M || (C = " ") : S !== 5 && S !== 17 && (m = !0);
          break;
        case 16:
          m = !0;
          break;
      }
      c && (S === 12 || S === 13) && (C = p());
    }
    S === 17 && (C = n.insertFinalNewline ? u : "");
    var D = g.getTokenOffset() + s;
    b(C, w, D), y = S;
  }
  return x;
}
function Un(e, t) {
  for (var n = "", r = 0; r < t; r++)
    n += e;
  return n;
}
function $l(e, t) {
  for (var n = 0, r = 0, i = t.tabSize || 4; n < e.length; ) {
    var s = e.charAt(n);
    if (s === " ")
      r++;
    else if (s === "	")
      r += i;
    else
      break;
    n++;
  }
  return Math.floor(r / i);
}
function Wl(e, t) {
  for (var n = 0; n < t.length; n++) {
    var r = t.charAt(n);
    if (r === "\r")
      return n + 1 < t.length && t.charAt(n + 1) === `
` ? `\r
` : "\r";
    if (r === `
`)
      return `
`;
  }
  return e && e.eol || `
`;
}
function os(e, t) {
  return `\r
`.indexOf(e.charAt(t)) !== -1;
}
var _n;
(function(e) {
  e.DEFAULT = {
    allowTrailingComma: !1
  };
})(_n || (_n = {}));
function Hl(e, t, n) {
  t === void 0 && (t = []), n === void 0 && (n = _n.DEFAULT);
  var r = null, i = [], s = [];
  function a(l) {
    Array.isArray(i) ? i.push(l) : r !== null && (i[r] = l);
  }
  var o = {
    onObjectBegin: function() {
      var l = {};
      a(l), s.push(i), i = l, r = null;
    },
    onObjectProperty: function(l) {
      r = l;
    },
    onObjectEnd: function() {
      i = s.pop();
    },
    onArrayBegin: function() {
      var l = [];
      a(l), s.push(i), i = l, r = null;
    },
    onArrayEnd: function() {
      i = s.pop();
    },
    onLiteralValue: a,
    onError: function(l, u, c) {
      t.push({ error: l, offset: u, length: c });
    }
  };
  return Gl(e, o, n), i[0];
}
function fa(e) {
  if (!e.parent || !e.parent.children)
    return [];
  var t = fa(e.parent);
  if (e.parent.type === "property") {
    var n = e.parent.children[0].value;
    t.push(n);
  } else if (e.parent.type === "array") {
    var r = e.parent.children.indexOf(e);
    r !== -1 && t.push(r);
  }
  return t;
}
function pr(e) {
  switch (e.type) {
    case "array":
      return e.children.map(pr);
    case "object":
      for (var t = /* @__PURE__ */ Object.create(null), n = 0, r = e.children; n < r.length; n++) {
        var i = r[n], s = i.children[1];
        s && (t[i.children[0].value] = pr(s));
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
function zl(e, t, n) {
  return n === void 0 && (n = !1), t >= e.offset && t < e.offset + e.length || n && t === e.offset + e.length;
}
function ha(e, t, n) {
  if (n === void 0 && (n = !1), zl(e, t, n)) {
    var r = e.children;
    if (Array.isArray(r))
      for (var i = 0; i < r.length && r[i].offset <= t; i++) {
        var s = ha(r[i], t, n);
        if (s)
          return s;
      }
    return e;
  }
}
function Gl(e, t, n) {
  n === void 0 && (n = _n.DEFAULT);
  var r = Ir(e, !1);
  function i(M) {
    return M ? function() {
      return M(r.getTokenOffset(), r.getTokenLength(), r.getTokenStartLine(), r.getTokenStartCharacter());
    } : function() {
      return !0;
    };
  }
  function s(M) {
    return M ? function(I) {
      return M(I, r.getTokenOffset(), r.getTokenLength(), r.getTokenStartLine(), r.getTokenStartCharacter());
    } : function() {
      return !0;
    };
  }
  var a = i(t.onObjectBegin), o = s(t.onObjectProperty), l = i(t.onObjectEnd), u = i(t.onArrayBegin), c = i(t.onArrayEnd), f = s(t.onLiteralValue), d = s(t.onSeparator), g = i(t.onComment), m = s(t.onError), p = n && n.disallowComments, v = n && n.allowTrailingComma;
  function x() {
    for (; ; ) {
      var M = r.scan();
      switch (r.getTokenError()) {
        case 4:
          b(14);
          break;
        case 5:
          b(15);
          break;
        case 3:
          b(13);
          break;
        case 1:
          p || b(11);
          break;
        case 2:
          b(12);
          break;
        case 6:
          b(16);
          break;
      }
      switch (M) {
        case 12:
        case 13:
          p ? b(10) : g();
          break;
        case 16:
          b(1);
          break;
        case 15:
        case 14:
          break;
        default:
          return M;
      }
    }
  }
  function b(M, I, D) {
    if (I === void 0 && (I = []), D === void 0 && (D = []), m(M), I.length + D.length > 0)
      for (var L = r.getToken(); L !== 17; ) {
        if (I.indexOf(L) !== -1) {
          x();
          break;
        } else if (D.indexOf(L) !== -1)
          break;
        L = x();
      }
  }
  function y(M) {
    var I = r.getTokenValue();
    return M ? f(I) : o(I), x(), !0;
  }
  function _() {
    switch (r.getToken()) {
      case 11:
        var M = r.getTokenValue(), I = Number(M);
        isNaN(I) && (b(2), I = 0), f(I);
        break;
      case 7:
        f(null);
        break;
      case 8:
        f(!0);
        break;
      case 9:
        f(!1);
        break;
      default:
        return !1;
    }
    return x(), !0;
  }
  function N() {
    return r.getToken() !== 10 ? (b(3, [], [2, 5]), !1) : (y(!1), r.getToken() === 6 ? (d(":"), x(), C() || b(4, [], [2, 5])) : b(5, [], [2, 5]), !0);
  }
  function w() {
    a(), x();
    for (var M = !1; r.getToken() !== 2 && r.getToken() !== 17; ) {
      if (r.getToken() === 5) {
        if (M || b(4, [], []), d(","), x(), r.getToken() === 2 && v)
          break;
      } else
        M && b(6, [], []);
      N() || b(4, [], [2, 5]), M = !0;
    }
    return l(), r.getToken() !== 2 ? b(7, [2], []) : x(), !0;
  }
  function S() {
    u(), x();
    for (var M = !1; r.getToken() !== 4 && r.getToken() !== 17; ) {
      if (r.getToken() === 5) {
        if (M || b(4, [], []), d(","), x(), r.getToken() === 4 && v)
          break;
      } else
        M && b(6, [], []);
      C() || b(4, [], [4, 5]), M = !0;
    }
    return c(), r.getToken() !== 4 ? b(8, [4], []) : x(), !0;
  }
  function C() {
    switch (r.getToken()) {
      case 3:
        return S();
      case 1:
        return w();
      case 10:
        return y(!0);
      default:
        return _();
    }
  }
  return x(), r.getToken() === 17 ? n.allowEmptyContent ? !0 : (b(4, [], []), !1) : C() ? (r.getToken() !== 17 && b(9, [], []), !0) : (b(4, [], []), !1);
}
var wt = Ir, Jl = Hl, Xl = ha, Ql = fa, Zl = pr;
function Yl(e, t, n) {
  return Ul(e, t, n);
}
function Dt(e, t) {
  if (e === t)
    return !0;
  if (e == null || t === null || t === void 0 || typeof e != typeof t || typeof e != "object" || Array.isArray(e) !== Array.isArray(t))
    return !1;
  var n, r;
  if (Array.isArray(e)) {
    if (e.length !== t.length)
      return !1;
    for (n = 0; n < e.length; n++)
      if (!Dt(e[n], t[n]))
        return !1;
  } else {
    var i = [];
    for (r in e)
      i.push(r);
    i.sort();
    var s = [];
    for (r in t)
      s.push(r);
    if (s.sort(), !Dt(i, s))
      return !1;
    for (n = 0; n < i.length; n++)
      if (!Dt(e[i[n]], t[i[n]]))
        return !1;
  }
  return !0;
}
function be(e) {
  return typeof e == "number";
}
function je(e) {
  return typeof e < "u";
}
function De(e) {
  return typeof e == "boolean";
}
function Kl(e) {
  return typeof e == "string";
}
function eu(e, t) {
  if (e.length < t.length)
    return !1;
  for (var n = 0; n < t.length; n++)
    if (e[n] !== t[n])
      return !1;
  return !0;
}
function Ht(e, t) {
  var n = e.length - t.length;
  return n > 0 ? e.lastIndexOf(t) === n : n === 0 ? e === t : !1;
}
function Sn(e) {
  var t = "";
  eu(e, "(?i)") && (e = e.substring(4), t = "i");
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
var ls;
(function(e) {
  e.MIN_VALUE = -2147483648, e.MAX_VALUE = 2147483647;
})(ls || (ls = {}));
var Ln;
(function(e) {
  e.MIN_VALUE = 0, e.MAX_VALUE = 2147483647;
})(Ln || (Ln = {}));
var ke;
(function(e) {
  function t(r, i) {
    return r === Number.MAX_VALUE && (r = Ln.MAX_VALUE), i === Number.MAX_VALUE && (i = Ln.MAX_VALUE), { line: r, character: i };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return R.objectLiteral(i) && R.uinteger(i.line) && R.uinteger(i.character);
  }
  e.is = n;
})(ke || (ke = {}));
var X;
(function(e) {
  function t(r, i, s, a) {
    if (R.uinteger(r) && R.uinteger(i) && R.uinteger(s) && R.uinteger(a))
      return { start: ke.create(r, i), end: ke.create(s, a) };
    if (ke.is(r) && ke.is(i))
      return { start: r, end: i };
    throw new Error("Range#create called with invalid arguments[" + r + ", " + i + ", " + s + ", " + a + "]");
  }
  e.create = t;
  function n(r) {
    var i = r;
    return R.objectLiteral(i) && ke.is(i.start) && ke.is(i.end);
  }
  e.is = n;
})(X || (X = {}));
var zt;
(function(e) {
  function t(r, i) {
    return { uri: r, range: i };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return R.defined(i) && X.is(i.range) && (R.string(i.uri) || R.undefined(i.uri));
  }
  e.is = n;
})(zt || (zt = {}));
var us;
(function(e) {
  function t(r, i, s, a) {
    return { targetUri: r, targetRange: i, targetSelectionRange: s, originSelectionRange: a };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return R.defined(i) && X.is(i.targetRange) && R.string(i.targetUri) && (X.is(i.targetSelectionRange) || R.undefined(i.targetSelectionRange)) && (X.is(i.originSelectionRange) || R.undefined(i.originSelectionRange));
  }
  e.is = n;
})(us || (us = {}));
var vr;
(function(e) {
  function t(r, i, s, a) {
    return {
      red: r,
      green: i,
      blue: s,
      alpha: a
    };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return R.numberRange(i.red, 0, 1) && R.numberRange(i.green, 0, 1) && R.numberRange(i.blue, 0, 1) && R.numberRange(i.alpha, 0, 1);
  }
  e.is = n;
})(vr || (vr = {}));
var cs;
(function(e) {
  function t(r, i) {
    return {
      range: r,
      color: i
    };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return X.is(i.range) && vr.is(i.color);
  }
  e.is = n;
})(cs || (cs = {}));
var fs;
(function(e) {
  function t(r, i, s) {
    return {
      label: r,
      textEdit: i,
      additionalTextEdits: s
    };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return R.string(i.label) && (R.undefined(i.textEdit) || Me.is(i)) && (R.undefined(i.additionalTextEdits) || R.typedArray(i.additionalTextEdits, Me.is));
  }
  e.is = n;
})(fs || (fs = {}));
var Vt;
(function(e) {
  e.Comment = "comment", e.Imports = "imports", e.Region = "region";
})(Vt || (Vt = {}));
var hs;
(function(e) {
  function t(r, i, s, a, o) {
    var l = {
      startLine: r,
      endLine: i
    };
    return R.defined(s) && (l.startCharacter = s), R.defined(a) && (l.endCharacter = a), R.defined(o) && (l.kind = o), l;
  }
  e.create = t;
  function n(r) {
    var i = r;
    return R.uinteger(i.startLine) && R.uinteger(i.startLine) && (R.undefined(i.startCharacter) || R.uinteger(i.startCharacter)) && (R.undefined(i.endCharacter) || R.uinteger(i.endCharacter)) && (R.undefined(i.kind) || R.string(i.kind));
  }
  e.is = n;
})(hs || (hs = {}));
var br;
(function(e) {
  function t(r, i) {
    return {
      location: r,
      message: i
    };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return R.defined(i) && zt.is(i.location) && R.string(i.message);
  }
  e.is = n;
})(br || (br = {}));
var we;
(function(e) {
  e.Error = 1, e.Warning = 2, e.Information = 3, e.Hint = 4;
})(we || (we = {}));
var ds;
(function(e) {
  e.Unnecessary = 1, e.Deprecated = 2;
})(ds || (ds = {}));
var gs;
(function(e) {
  function t(n) {
    var r = n;
    return r != null && R.string(r.href);
  }
  e.is = t;
})(gs || (gs = {}));
var Be;
(function(e) {
  function t(r, i, s, a, o, l) {
    var u = { range: r, message: i };
    return R.defined(s) && (u.severity = s), R.defined(a) && (u.code = a), R.defined(o) && (u.source = o), R.defined(l) && (u.relatedInformation = l), u;
  }
  e.create = t;
  function n(r) {
    var i, s = r;
    return R.defined(s) && X.is(s.range) && R.string(s.message) && (R.number(s.severity) || R.undefined(s.severity)) && (R.integer(s.code) || R.string(s.code) || R.undefined(s.code)) && (R.undefined(s.codeDescription) || R.string((i = s.codeDescription) === null || i === void 0 ? void 0 : i.href)) && (R.string(s.source) || R.undefined(s.source)) && (R.undefined(s.relatedInformation) || R.typedArray(s.relatedInformation, br.is));
  }
  e.is = n;
})(Be || (Be = {}));
var Gt;
(function(e) {
  function t(r, i) {
    for (var s = [], a = 2; a < arguments.length; a++)
      s[a - 2] = arguments[a];
    var o = { title: r, command: i };
    return R.defined(s) && s.length > 0 && (o.arguments = s), o;
  }
  e.create = t;
  function n(r) {
    var i = r;
    return R.defined(i) && R.string(i.title) && R.string(i.command);
  }
  e.is = n;
})(Gt || (Gt = {}));
var Me;
(function(e) {
  function t(s, a) {
    return { range: s, newText: a };
  }
  e.replace = t;
  function n(s, a) {
    return { range: { start: s, end: s }, newText: a };
  }
  e.insert = n;
  function r(s) {
    return { range: s, newText: "" };
  }
  e.del = r;
  function i(s) {
    var a = s;
    return R.objectLiteral(a) && R.string(a.newText) && X.is(a.range);
  }
  e.is = i;
})(Me || (Me = {}));
var _t;
(function(e) {
  function t(r, i, s) {
    var a = { label: r };
    return i !== void 0 && (a.needsConfirmation = i), s !== void 0 && (a.description = s), a;
  }
  e.create = t;
  function n(r) {
    var i = r;
    return i !== void 0 && R.objectLiteral(i) && R.string(i.label) && (R.boolean(i.needsConfirmation) || i.needsConfirmation === void 0) && (R.string(i.description) || i.description === void 0);
  }
  e.is = n;
})(_t || (_t = {}));
var fe;
(function(e) {
  function t(n) {
    var r = n;
    return typeof r == "string";
  }
  e.is = t;
})(fe || (fe = {}));
var Ze;
(function(e) {
  function t(s, a, o) {
    return { range: s, newText: a, annotationId: o };
  }
  e.replace = t;
  function n(s, a, o) {
    return { range: { start: s, end: s }, newText: a, annotationId: o };
  }
  e.insert = n;
  function r(s, a) {
    return { range: s, newText: "", annotationId: a };
  }
  e.del = r;
  function i(s) {
    var a = s;
    return Me.is(a) && (_t.is(a.annotationId) || fe.is(a.annotationId));
  }
  e.is = i;
})(Ze || (Ze = {}));
var Nn;
(function(e) {
  function t(r, i) {
    return { textDocument: r, edits: i };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return R.defined(i) && An.is(i.textDocument) && Array.isArray(i.edits);
  }
  e.is = n;
})(Nn || (Nn = {}));
var Jt;
(function(e) {
  function t(r, i, s) {
    var a = {
      kind: "create",
      uri: r
    };
    return i !== void 0 && (i.overwrite !== void 0 || i.ignoreIfExists !== void 0) && (a.options = i), s !== void 0 && (a.annotationId = s), a;
  }
  e.create = t;
  function n(r) {
    var i = r;
    return i && i.kind === "create" && R.string(i.uri) && (i.options === void 0 || (i.options.overwrite === void 0 || R.boolean(i.options.overwrite)) && (i.options.ignoreIfExists === void 0 || R.boolean(i.options.ignoreIfExists))) && (i.annotationId === void 0 || fe.is(i.annotationId));
  }
  e.is = n;
})(Jt || (Jt = {}));
var Xt;
(function(e) {
  function t(r, i, s, a) {
    var o = {
      kind: "rename",
      oldUri: r,
      newUri: i
    };
    return s !== void 0 && (s.overwrite !== void 0 || s.ignoreIfExists !== void 0) && (o.options = s), a !== void 0 && (o.annotationId = a), o;
  }
  e.create = t;
  function n(r) {
    var i = r;
    return i && i.kind === "rename" && R.string(i.oldUri) && R.string(i.newUri) && (i.options === void 0 || (i.options.overwrite === void 0 || R.boolean(i.options.overwrite)) && (i.options.ignoreIfExists === void 0 || R.boolean(i.options.ignoreIfExists))) && (i.annotationId === void 0 || fe.is(i.annotationId));
  }
  e.is = n;
})(Xt || (Xt = {}));
var Qt;
(function(e) {
  function t(r, i, s) {
    var a = {
      kind: "delete",
      uri: r
    };
    return i !== void 0 && (i.recursive !== void 0 || i.ignoreIfNotExists !== void 0) && (a.options = i), s !== void 0 && (a.annotationId = s), a;
  }
  e.create = t;
  function n(r) {
    var i = r;
    return i && i.kind === "delete" && R.string(i.uri) && (i.options === void 0 || (i.options.recursive === void 0 || R.boolean(i.options.recursive)) && (i.options.ignoreIfNotExists === void 0 || R.boolean(i.options.ignoreIfNotExists))) && (i.annotationId === void 0 || fe.is(i.annotationId));
  }
  e.is = n;
})(Qt || (Qt = {}));
var yr;
(function(e) {
  function t(n) {
    var r = n;
    return r && (r.changes !== void 0 || r.documentChanges !== void 0) && (r.documentChanges === void 0 || r.documentChanges.every(function(i) {
      return R.string(i.kind) ? Jt.is(i) || Xt.is(i) || Qt.is(i) : Nn.is(i);
    }));
  }
  e.is = t;
})(yr || (yr = {}));
var nn = function() {
  function e(t, n) {
    this.edits = t, this.changeAnnotations = n;
  }
  return e.prototype.insert = function(t, n, r) {
    var i, s;
    if (r === void 0 ? i = Me.insert(t, n) : fe.is(r) ? (s = r, i = Ze.insert(t, n, r)) : (this.assertChangeAnnotations(this.changeAnnotations), s = this.changeAnnotations.manage(r), i = Ze.insert(t, n, s)), this.edits.push(i), s !== void 0)
      return s;
  }, e.prototype.replace = function(t, n, r) {
    var i, s;
    if (r === void 0 ? i = Me.replace(t, n) : fe.is(r) ? (s = r, i = Ze.replace(t, n, r)) : (this.assertChangeAnnotations(this.changeAnnotations), s = this.changeAnnotations.manage(r), i = Ze.replace(t, n, s)), this.edits.push(i), s !== void 0)
      return s;
  }, e.prototype.delete = function(t, n) {
    var r, i;
    if (n === void 0 ? r = Me.del(t) : fe.is(n) ? (i = n, r = Ze.del(t, n)) : (this.assertChangeAnnotations(this.changeAnnotations), i = this.changeAnnotations.manage(n), r = Ze.del(t, i)), this.edits.push(r), i !== void 0)
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
}(), ms = function() {
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
  }), e.prototype.manage = function(t, n) {
    var r;
    if (fe.is(t) ? r = t : (r = this.nextId(), n = t), this._annotations[r] !== void 0)
      throw new Error("Id " + r + " is already in use.");
    if (n === void 0)
      throw new Error("No annotation provided for id " + r);
    return this._annotations[r] = n, this._size++, r;
  }, e.prototype.nextId = function() {
    return this._counter++, this._counter.toString();
  }, e;
}();
(function() {
  function e(t) {
    var n = this;
    this._textEditChanges = /* @__PURE__ */ Object.create(null), t !== void 0 ? (this._workspaceEdit = t, t.documentChanges ? (this._changeAnnotations = new ms(t.changeAnnotations), t.changeAnnotations = this._changeAnnotations.all(), t.documentChanges.forEach(function(r) {
      if (Nn.is(r)) {
        var i = new nn(r.edits, n._changeAnnotations);
        n._textEditChanges[r.textDocument.uri] = i;
      }
    })) : t.changes && Object.keys(t.changes).forEach(function(r) {
      var i = new nn(t.changes[r]);
      n._textEditChanges[r] = i;
    })) : this._workspaceEdit = {};
  }
  return Object.defineProperty(e.prototype, "edit", {
    get: function() {
      return this.initDocumentChanges(), this._changeAnnotations !== void 0 && (this._changeAnnotations.size === 0 ? this._workspaceEdit.changeAnnotations = void 0 : this._workspaceEdit.changeAnnotations = this._changeAnnotations.all()), this._workspaceEdit;
    },
    enumerable: !1,
    configurable: !0
  }), e.prototype.getTextEditChange = function(t) {
    if (An.is(t)) {
      if (this.initDocumentChanges(), this._workspaceEdit.documentChanges === void 0)
        throw new Error("Workspace edit is not configured for document changes.");
      var n = { uri: t.uri, version: t.version }, r = this._textEditChanges[n.uri];
      if (!r) {
        var i = [], s = {
          textDocument: n,
          edits: i
        };
        this._workspaceEdit.documentChanges.push(s), r = new nn(i, this._changeAnnotations), this._textEditChanges[n.uri] = r;
      }
      return r;
    } else {
      if (this.initChanges(), this._workspaceEdit.changes === void 0)
        throw new Error("Workspace edit is not configured for normal text edit changes.");
      var r = this._textEditChanges[t];
      if (!r) {
        var i = [];
        this._workspaceEdit.changes[t] = i, r = new nn(i), this._textEditChanges[t] = r;
      }
      return r;
    }
  }, e.prototype.initDocumentChanges = function() {
    this._workspaceEdit.documentChanges === void 0 && this._workspaceEdit.changes === void 0 && (this._changeAnnotations = new ms(), this._workspaceEdit.documentChanges = [], this._workspaceEdit.changeAnnotations = this._changeAnnotations.all());
  }, e.prototype.initChanges = function() {
    this._workspaceEdit.documentChanges === void 0 && this._workspaceEdit.changes === void 0 && (this._workspaceEdit.changes = /* @__PURE__ */ Object.create(null));
  }, e.prototype.createFile = function(t, n, r) {
    if (this.initDocumentChanges(), this._workspaceEdit.documentChanges === void 0)
      throw new Error("Workspace edit is not configured for document changes.");
    var i;
    _t.is(n) || fe.is(n) ? i = n : r = n;
    var s, a;
    if (i === void 0 ? s = Jt.create(t, r) : (a = fe.is(i) ? i : this._changeAnnotations.manage(i), s = Jt.create(t, r, a)), this._workspaceEdit.documentChanges.push(s), a !== void 0)
      return a;
  }, e.prototype.renameFile = function(t, n, r, i) {
    if (this.initDocumentChanges(), this._workspaceEdit.documentChanges === void 0)
      throw new Error("Workspace edit is not configured for document changes.");
    var s;
    _t.is(r) || fe.is(r) ? s = r : i = r;
    var a, o;
    if (s === void 0 ? a = Xt.create(t, n, i) : (o = fe.is(s) ? s : this._changeAnnotations.manage(s), a = Xt.create(t, n, i, o)), this._workspaceEdit.documentChanges.push(a), o !== void 0)
      return o;
  }, e.prototype.deleteFile = function(t, n, r) {
    if (this.initDocumentChanges(), this._workspaceEdit.documentChanges === void 0)
      throw new Error("Workspace edit is not configured for document changes.");
    var i;
    _t.is(n) || fe.is(n) ? i = n : r = n;
    var s, a;
    if (i === void 0 ? s = Qt.create(t, r) : (a = fe.is(i) ? i : this._changeAnnotations.manage(i), s = Qt.create(t, r, a)), this._workspaceEdit.documentChanges.push(s), a !== void 0)
      return a;
  }, e;
})();
var ps;
(function(e) {
  function t(r) {
    return { uri: r };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return R.defined(i) && R.string(i.uri);
  }
  e.is = n;
})(ps || (ps = {}));
var vs;
(function(e) {
  function t(r, i) {
    return { uri: r, version: i };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return R.defined(i) && R.string(i.uri) && R.integer(i.version);
  }
  e.is = n;
})(vs || (vs = {}));
var An;
(function(e) {
  function t(r, i) {
    return { uri: r, version: i };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return R.defined(i) && R.string(i.uri) && (i.version === null || R.integer(i.version));
  }
  e.is = n;
})(An || (An = {}));
var bs;
(function(e) {
  function t(r, i, s, a) {
    return { uri: r, languageId: i, version: s, text: a };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return R.defined(i) && R.string(i.uri) && R.string(i.languageId) && R.integer(i.version) && R.string(i.text);
  }
  e.is = n;
})(bs || (bs = {}));
var $e;
(function(e) {
  e.PlainText = "plaintext", e.Markdown = "markdown";
})($e || ($e = {}));
(function(e) {
  function t(n) {
    var r = n;
    return r === e.PlainText || r === e.Markdown;
  }
  e.is = t;
})($e || ($e = {}));
var xr;
(function(e) {
  function t(n) {
    var r = n;
    return R.objectLiteral(n) && $e.is(r.kind) && R.string(r.value);
  }
  e.is = t;
})(xr || (xr = {}));
var xe;
(function(e) {
  e.Text = 1, e.Method = 2, e.Function = 3, e.Constructor = 4, e.Field = 5, e.Variable = 6, e.Class = 7, e.Interface = 8, e.Module = 9, e.Property = 10, e.Unit = 11, e.Value = 12, e.Enum = 13, e.Keyword = 14, e.Snippet = 15, e.Color = 16, e.File = 17, e.Reference = 18, e.Folder = 19, e.EnumMember = 20, e.Constant = 21, e.Struct = 22, e.Event = 23, e.Operator = 24, e.TypeParameter = 25;
})(xe || (xe = {}));
var ie;
(function(e) {
  e.PlainText = 1, e.Snippet = 2;
})(ie || (ie = {}));
var ys;
(function(e) {
  e.Deprecated = 1;
})(ys || (ys = {}));
var xs;
(function(e) {
  function t(r, i, s) {
    return { newText: r, insert: i, replace: s };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return i && R.string(i.newText) && X.is(i.insert) && X.is(i.replace);
  }
  e.is = n;
})(xs || (xs = {}));
var ws;
(function(e) {
  e.asIs = 1, e.adjustIndentation = 2;
})(ws || (ws = {}));
var wr;
(function(e) {
  function t(n) {
    return { label: n };
  }
  e.create = t;
})(wr || (wr = {}));
var _s;
(function(e) {
  function t(n, r) {
    return { items: n || [], isIncomplete: !!r };
  }
  e.create = t;
})(_s || (_s = {}));
var Cn;
(function(e) {
  function t(r) {
    return r.replace(/[\\`*_{}[\]()#+\-.!]/g, "\\$&");
  }
  e.fromPlainText = t;
  function n(r) {
    var i = r;
    return R.string(i) || R.objectLiteral(i) && R.string(i.language) && R.string(i.value);
  }
  e.is = n;
})(Cn || (Cn = {}));
var Ss;
(function(e) {
  function t(n) {
    var r = n;
    return !!r && R.objectLiteral(r) && (xr.is(r.contents) || Cn.is(r.contents) || R.typedArray(r.contents, Cn.is)) && (n.range === void 0 || X.is(n.range));
  }
  e.is = t;
})(Ss || (Ss = {}));
var Ls;
(function(e) {
  function t(n, r) {
    return r ? { label: n, documentation: r } : { label: n };
  }
  e.create = t;
})(Ls || (Ls = {}));
var Ns;
(function(e) {
  function t(n, r) {
    for (var i = [], s = 2; s < arguments.length; s++)
      i[s - 2] = arguments[s];
    var a = { label: n };
    return R.defined(r) && (a.documentation = r), R.defined(i) ? a.parameters = i : a.parameters = [], a;
  }
  e.create = t;
})(Ns || (Ns = {}));
var As;
(function(e) {
  e.Text = 1, e.Read = 2, e.Write = 3;
})(As || (As = {}));
var Cs;
(function(e) {
  function t(n, r) {
    var i = { range: n };
    return R.number(r) && (i.kind = r), i;
  }
  e.create = t;
})(Cs || (Cs = {}));
var Pe;
(function(e) {
  e.File = 1, e.Module = 2, e.Namespace = 3, e.Package = 4, e.Class = 5, e.Method = 6, e.Property = 7, e.Field = 8, e.Constructor = 9, e.Enum = 10, e.Interface = 11, e.Function = 12, e.Variable = 13, e.Constant = 14, e.String = 15, e.Number = 16, e.Boolean = 17, e.Array = 18, e.Object = 19, e.Key = 20, e.Null = 21, e.EnumMember = 22, e.Struct = 23, e.Event = 24, e.Operator = 25, e.TypeParameter = 26;
})(Pe || (Pe = {}));
var ks;
(function(e) {
  e.Deprecated = 1;
})(ks || (ks = {}));
var Es;
(function(e) {
  function t(n, r, i, s, a) {
    var o = {
      name: n,
      kind: r,
      location: { uri: s, range: i }
    };
    return a && (o.containerName = a), o;
  }
  e.create = t;
})(Es || (Es = {}));
var Ms;
(function(e) {
  function t(r, i, s, a, o, l) {
    var u = {
      name: r,
      detail: i,
      kind: s,
      range: a,
      selectionRange: o
    };
    return l !== void 0 && (u.children = l), u;
  }
  e.create = t;
  function n(r) {
    var i = r;
    return i && R.string(i.name) && R.number(i.kind) && X.is(i.range) && X.is(i.selectionRange) && (i.detail === void 0 || R.string(i.detail)) && (i.deprecated === void 0 || R.boolean(i.deprecated)) && (i.children === void 0 || Array.isArray(i.children)) && (i.tags === void 0 || Array.isArray(i.tags));
  }
  e.is = n;
})(Ms || (Ms = {}));
var Rs;
(function(e) {
  e.Empty = "", e.QuickFix = "quickfix", e.Refactor = "refactor", e.RefactorExtract = "refactor.extract", e.RefactorInline = "refactor.inline", e.RefactorRewrite = "refactor.rewrite", e.Source = "source", e.SourceOrganizeImports = "source.organizeImports", e.SourceFixAll = "source.fixAll";
})(Rs || (Rs = {}));
var Ts;
(function(e) {
  function t(r, i) {
    var s = { diagnostics: r };
    return i != null && (s.only = i), s;
  }
  e.create = t;
  function n(r) {
    var i = r;
    return R.defined(i) && R.typedArray(i.diagnostics, Be.is) && (i.only === void 0 || R.typedArray(i.only, R.string));
  }
  e.is = n;
})(Ts || (Ts = {}));
var Ps;
(function(e) {
  function t(r, i, s) {
    var a = { title: r }, o = !0;
    return typeof i == "string" ? (o = !1, a.kind = i) : Gt.is(i) ? a.command = i : a.edit = i, o && s !== void 0 && (a.kind = s), a;
  }
  e.create = t;
  function n(r) {
    var i = r;
    return i && R.string(i.title) && (i.diagnostics === void 0 || R.typedArray(i.diagnostics, Be.is)) && (i.kind === void 0 || R.string(i.kind)) && (i.edit !== void 0 || i.command !== void 0) && (i.command === void 0 || Gt.is(i.command)) && (i.isPreferred === void 0 || R.boolean(i.isPreferred)) && (i.edit === void 0 || yr.is(i.edit));
  }
  e.is = n;
})(Ps || (Ps = {}));
var Fs;
(function(e) {
  function t(r, i) {
    var s = { range: r };
    return R.defined(i) && (s.data = i), s;
  }
  e.create = t;
  function n(r) {
    var i = r;
    return R.defined(i) && X.is(i.range) && (R.undefined(i.command) || Gt.is(i.command));
  }
  e.is = n;
})(Fs || (Fs = {}));
var Is;
(function(e) {
  function t(r, i) {
    return { tabSize: r, insertSpaces: i };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return R.defined(i) && R.uinteger(i.tabSize) && R.boolean(i.insertSpaces);
  }
  e.is = n;
})(Is || (Is = {}));
var Ds;
(function(e) {
  function t(r, i, s) {
    return { range: r, target: i, data: s };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return R.defined(i) && X.is(i.range) && (R.undefined(i.target) || R.string(i.target));
  }
  e.is = n;
})(Ds || (Ds = {}));
var kn;
(function(e) {
  function t(r, i) {
    return { range: r, parent: i };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return i !== void 0 && X.is(i.range) && (i.parent === void 0 || e.is(i.parent));
  }
  e.is = n;
})(kn || (kn = {}));
var Vs;
(function(e) {
  function t(s, a, o, l) {
    return new tu(s, a, o, l);
  }
  e.create = t;
  function n(s) {
    var a = s;
    return !!(R.defined(a) && R.string(a.uri) && (R.undefined(a.languageId) || R.string(a.languageId)) && R.uinteger(a.lineCount) && R.func(a.getText) && R.func(a.positionAt) && R.func(a.offsetAt));
  }
  e.is = n;
  function r(s, a) {
    for (var o = s.getText(), l = i(a, function(m, p) {
      var v = m.range.start.line - p.range.start.line;
      return v === 0 ? m.range.start.character - p.range.start.character : v;
    }), u = o.length, c = l.length - 1; c >= 0; c--) {
      var f = l[c], d = s.offsetAt(f.range.start), g = s.offsetAt(f.range.end);
      if (g <= u)
        o = o.substring(0, d) + f.newText + o.substring(g, o.length);
      else
        throw new Error("Overlapping edit");
      u = d;
    }
    return o;
  }
  e.applyEdits = r;
  function i(s, a) {
    if (s.length <= 1)
      return s;
    var o = s.length / 2 | 0, l = s.slice(0, o), u = s.slice(o);
    i(l, a), i(u, a);
    for (var c = 0, f = 0, d = 0; c < l.length && f < u.length; ) {
      var g = a(l[c], u[f]);
      g <= 0 ? s[d++] = l[c++] : s[d++] = u[f++];
    }
    for (; c < l.length; )
      s[d++] = l[c++];
    for (; f < u.length; )
      s[d++] = u[f++];
    return s;
  }
})(Vs || (Vs = {}));
var tu = function() {
  function e(t, n, r, i) {
    this._uri = t, this._languageId = n, this._version = r, this._content = i, this._lineOffsets = void 0;
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
      var n = this.offsetAt(t.start), r = this.offsetAt(t.end);
      return this._content.substring(n, r);
    }
    return this._content;
  }, e.prototype.update = function(t, n) {
    this._content = t.text, this._version = n, this._lineOffsets = void 0;
  }, e.prototype.getLineOffsets = function() {
    if (this._lineOffsets === void 0) {
      for (var t = [], n = this._content, r = !0, i = 0; i < n.length; i++) {
        r && (t.push(i), r = !1);
        var s = n.charAt(i);
        r = s === "\r" || s === `
`, s === "\r" && i + 1 < n.length && n.charAt(i + 1) === `
` && i++;
      }
      r && n.length > 0 && t.push(n.length), this._lineOffsets = t;
    }
    return this._lineOffsets;
  }, e.prototype.positionAt = function(t) {
    t = Math.max(Math.min(t, this._content.length), 0);
    var n = this.getLineOffsets(), r = 0, i = n.length;
    if (i === 0)
      return ke.create(0, t);
    for (; r < i; ) {
      var s = Math.floor((r + i) / 2);
      n[s] > t ? i = s : r = s + 1;
    }
    var a = r - 1;
    return ke.create(a, t - n[a]);
  }, e.prototype.offsetAt = function(t) {
    var n = this.getLineOffsets();
    if (t.line >= n.length)
      return this._content.length;
    if (t.line < 0)
      return 0;
    var r = n[t.line], i = t.line + 1 < n.length ? n[t.line + 1] : this._content.length;
    return Math.max(Math.min(r + t.character, i), r);
  }, Object.defineProperty(e.prototype, "lineCount", {
    get: function() {
      return this.getLineOffsets().length;
    },
    enumerable: !1,
    configurable: !0
  }), e;
}(), R;
(function(e) {
  var t = Object.prototype.toString;
  function n(g) {
    return typeof g < "u";
  }
  e.defined = n;
  function r(g) {
    return typeof g > "u";
  }
  e.undefined = r;
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
  function o(g, m, p) {
    return t.call(g) === "[object Number]" && m <= g && g <= p;
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
  function c(g) {
    return t.call(g) === "[object Function]";
  }
  e.func = c;
  function f(g) {
    return g !== null && typeof g == "object";
  }
  e.objectLiteral = f;
  function d(g, m) {
    return Array.isArray(g) && g.every(m);
  }
  e.typedArray = d;
})(R || (R = {}));
var En = class {
  constructor(e, t, n, r) {
    this._uri = e, this._languageId = t, this._version = n, this._content = r, this._lineOffsets = void 0;
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
      const t = this.offsetAt(e.start), n = this.offsetAt(e.end);
      return this._content.substring(t, n);
    }
    return this._content;
  }
  update(e, t) {
    for (let n of e)
      if (En.isIncremental(n)) {
        const r = da(n.range), i = this.offsetAt(r.start), s = this.offsetAt(r.end);
        this._content = this._content.substring(0, i) + n.text + this._content.substring(s, this._content.length);
        const a = Math.max(r.start.line, 0), o = Math.max(r.end.line, 0);
        let l = this._lineOffsets;
        const u = Os(n.text, !1, i);
        if (o - a === u.length)
          for (let f = 0, d = u.length; f < d; f++)
            l[f + a + 1] = u[f];
        else
          u.length < 1e4 ? l.splice(a + 1, o - a, ...u) : this._lineOffsets = l = l.slice(0, a + 1).concat(u, l.slice(o + 1));
        const c = n.text.length - (s - i);
        if (c !== 0)
          for (let f = a + 1 + u.length, d = l.length; f < d; f++)
            l[f] = l[f] + c;
      } else if (En.isFull(n))
        this._content = n.text, this._lineOffsets = void 0;
      else
        throw new Error("Unknown change event received");
    this._version = t;
  }
  getLineOffsets() {
    return this._lineOffsets === void 0 && (this._lineOffsets = Os(this._content, !0)), this._lineOffsets;
  }
  positionAt(e) {
    e = Math.max(Math.min(e, this._content.length), 0);
    let t = this.getLineOffsets(), n = 0, r = t.length;
    if (r === 0)
      return { line: 0, character: e };
    for (; n < r; ) {
      let s = Math.floor((n + r) / 2);
      t[s] > e ? r = s : n = s + 1;
    }
    let i = n - 1;
    return { line: i, character: e - t[i] };
  }
  offsetAt(e) {
    let t = this.getLineOffsets();
    if (e.line >= t.length)
      return this._content.length;
    if (e.line < 0)
      return 0;
    let n = t[e.line], r = e.line + 1 < t.length ? t[e.line + 1] : this._content.length;
    return Math.max(Math.min(n + e.character, r), n);
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
}, _r;
(function(e) {
  function t(i, s, a, o) {
    return new En(i, s, a, o);
  }
  e.create = t;
  function n(i, s, a) {
    if (i instanceof En)
      return i.update(s, a), i;
    throw new Error("TextDocument.update: document must be created by TextDocument.create");
  }
  e.update = n;
  function r(i, s) {
    let a = i.getText(), o = Sr(s.map(nu), (c, f) => {
      let d = c.range.start.line - f.range.start.line;
      return d === 0 ? c.range.start.character - f.range.start.character : d;
    }), l = 0;
    const u = [];
    for (const c of o) {
      let f = i.offsetAt(c.range.start);
      if (f < l)
        throw new Error("Overlapping edit");
      f > l && u.push(a.substring(l, f)), c.newText.length && u.push(c.newText), l = i.offsetAt(c.range.end);
    }
    return u.push(a.substr(l)), u.join("");
  }
  e.applyEdits = r;
})(_r || (_r = {}));
function Sr(e, t) {
  if (e.length <= 1)
    return e;
  const n = e.length / 2 | 0, r = e.slice(0, n), i = e.slice(n);
  Sr(r, t), Sr(i, t);
  let s = 0, a = 0, o = 0;
  for (; s < r.length && a < i.length; )
    t(r[s], i[a]) <= 0 ? e[o++] = r[s++] : e[o++] = i[a++];
  for (; s < r.length; )
    e[o++] = r[s++];
  for (; a < i.length; )
    e[o++] = i[a++];
  return e;
}
function Os(e, t, n = 0) {
  const r = t ? [n] : [];
  for (let i = 0; i < e.length; i++) {
    let s = e.charCodeAt(i);
    (s === 13 || s === 10) && (s === 13 && i + 1 < e.length && e.charCodeAt(i + 1) === 10 && i++, r.push(n + i + 1));
  }
  return r;
}
function da(e) {
  const t = e.start, n = e.end;
  return t.line > n.line || t.line === n.line && t.character > n.character ? { start: n, end: t } : e;
}
function nu(e) {
  const t = da(e.range);
  return t !== e.range ? { newText: e.newText, range: t } : e;
}
var G;
(function(e) {
  e[e.Undefined = 0] = "Undefined", e[e.EnumValueMismatch = 1] = "EnumValueMismatch", e[e.Deprecated = 2] = "Deprecated", e[e.UnexpectedEndOfComment = 257] = "UnexpectedEndOfComment", e[e.UnexpectedEndOfString = 258] = "UnexpectedEndOfString", e[e.UnexpectedEndOfNumber = 259] = "UnexpectedEndOfNumber", e[e.InvalidUnicode = 260] = "InvalidUnicode", e[e.InvalidEscapeCharacter = 261] = "InvalidEscapeCharacter", e[e.InvalidCharacter = 262] = "InvalidCharacter", e[e.PropertyExpected = 513] = "PropertyExpected", e[e.CommaExpected = 514] = "CommaExpected", e[e.ColonExpected = 515] = "ColonExpected", e[e.ValueExpected = 516] = "ValueExpected", e[e.CommaOrCloseBacketExpected = 517] = "CommaOrCloseBacketExpected", e[e.CommaOrCloseBraceExpected = 518] = "CommaOrCloseBraceExpected", e[e.TrailingComma = 519] = "TrailingComma", e[e.DuplicateKey = 520] = "DuplicateKey", e[e.CommentNotPermitted = 521] = "CommentNotPermitted", e[e.SchemaResolveError = 768] = "SchemaResolveError";
})(G || (G = {}));
var js;
(function(e) {
  e.LATEST = {
    textDocument: {
      completion: {
        completionItem: {
          documentationFormat: [$e.Markdown, $e.PlainText],
          commitCharactersSupport: !0
        }
      }
    }
  };
})(js || (js = {}));
function ru(e, t) {
  let n;
  return t.length === 0 ? n = e : n = e.replace(/\{(\d+)\}/g, (r, i) => {
    let s = i[0];
    return typeof t[s] < "u" ? t[s] : r;
  }), n;
}
function iu(e, t, ...n) {
  return ru(t, n);
}
function Kt(e) {
  return iu;
}
var ut = function() {
  var e = function(t, n) {
    return e = Object.setPrototypeOf || { __proto__: [] } instanceof Array && function(r, i) {
      r.__proto__ = i;
    } || function(r, i) {
      for (var s in i)
        Object.prototype.hasOwnProperty.call(i, s) && (r[s] = i[s]);
    }, e(t, n);
  };
  return function(t, n) {
    if (typeof n != "function" && n !== null)
      throw new TypeError("Class extends value " + String(n) + " is not a constructor or null");
    e(t, n);
    function r() {
      this.constructor = t;
    }
    t.prototype = n === null ? Object.create(n) : (r.prototype = n.prototype, new r());
  };
}(), j = Kt(), su = {
  "color-hex": { errorMessage: j("colorHexFormatWarning", "Invalid color format. Use #RGB, #RGBA, #RRGGBB or #RRGGBBAA."), pattern: /^#([0-9A-Fa-f]{3,4}|([0-9A-Fa-f]{2}){3,4})$/ },
  "date-time": { errorMessage: j("dateTimeFormatWarning", "String is not a RFC3339 date-time."), pattern: /^(\d{4})-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])T([01][0-9]|2[0-3]):([0-5][0-9]):([0-5][0-9]|60)(\.[0-9]+)?(Z|(\+|-)([01][0-9]|2[0-3]):([0-5][0-9]))$/i },
  date: { errorMessage: j("dateFormatWarning", "String is not a RFC3339 date."), pattern: /^(\d{4})-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])$/i },
  time: { errorMessage: j("timeFormatWarning", "String is not a RFC3339 time."), pattern: /^([01][0-9]|2[0-3]):([0-5][0-9]):([0-5][0-9]|60)(\.[0-9]+)?(Z|(\+|-)([01][0-9]|2[0-3]):([0-5][0-9]))$/i },
  email: { errorMessage: j("emailFormatWarning", "String is not an e-mail address."), pattern: /^(([^<>()\[\]\\.,;:\s@"]+(\.[^<>()\[\]\\.,;:\s@"]+)*)|(".+"))@((\[[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}])|(([a-zA-Z0-9-]+\.)+[a-zA-Z]{2,}))$/ },
  hostname: { errorMessage: j("hostnameFormatWarning", "String is not a hostname."), pattern: /^(?=.{1,253}\.?$)[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[-0-9a-z]{0,61}[0-9a-z])?)*\.?$/i },
  ipv4: { errorMessage: j("ipv4FormatWarning", "String is not an IPv4 address."), pattern: /^(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)$/ },
  ipv6: { errorMessage: j("ipv6FormatWarning", "String is not an IPv6 address."), pattern: /^((([0-9a-f]{1,4}:){7}([0-9a-f]{1,4}|:))|(([0-9a-f]{1,4}:){6}(:[0-9a-f]{1,4}|((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3})|:))|(([0-9a-f]{1,4}:){5}(((:[0-9a-f]{1,4}){1,2})|:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3})|:))|(([0-9a-f]{1,4}:){4}(((:[0-9a-f]{1,4}){1,3})|((:[0-9a-f]{1,4})?:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9a-f]{1,4}:){3}(((:[0-9a-f]{1,4}){1,4})|((:[0-9a-f]{1,4}){0,2}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9a-f]{1,4}:){2}(((:[0-9a-f]{1,4}){1,5})|((:[0-9a-f]{1,4}){0,3}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9a-f]{1,4}:){1}(((:[0-9a-f]{1,4}){1,6})|((:[0-9a-f]{1,4}){0,4}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(:(((:[0-9a-f]{1,4}){1,7})|((:[0-9a-f]{1,4}){0,5}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:)))$/i }
}, ct = function() {
  function e(t, n, r) {
    r === void 0 && (r = 0), this.offset = n, this.length = r, this.parent = t;
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
}(), au = function(e) {
  ut(t, e);
  function t(n, r) {
    var i = e.call(this, n, r) || this;
    return i.type = "null", i.value = null, i;
  }
  return t;
}(ct), qs = function(e) {
  ut(t, e);
  function t(n, r, i) {
    var s = e.call(this, n, i) || this;
    return s.type = "boolean", s.value = r, s;
  }
  return t;
}(ct), ou = function(e) {
  ut(t, e);
  function t(n, r) {
    var i = e.call(this, n, r) || this;
    return i.type = "array", i.items = [], i;
  }
  return Object.defineProperty(t.prototype, "children", {
    get: function() {
      return this.items;
    },
    enumerable: !1,
    configurable: !0
  }), t;
}(ct), lu = function(e) {
  ut(t, e);
  function t(n, r) {
    var i = e.call(this, n, r) || this;
    return i.type = "number", i.isInteger = !0, i.value = Number.NaN, i;
  }
  return t;
}(ct), $n = function(e) {
  ut(t, e);
  function t(n, r, i) {
    var s = e.call(this, n, r, i) || this;
    return s.type = "string", s.value = "", s;
  }
  return t;
}(ct), uu = function(e) {
  ut(t, e);
  function t(n, r, i) {
    var s = e.call(this, n, r) || this;
    return s.type = "property", s.colonOffset = -1, s.keyNode = i, s;
  }
  return Object.defineProperty(t.prototype, "children", {
    get: function() {
      return this.valueNode ? [this.keyNode, this.valueNode] : [this.keyNode];
    },
    enumerable: !1,
    configurable: !0
  }), t;
}(ct), cu = function(e) {
  ut(t, e);
  function t(n, r) {
    var i = e.call(this, n, r) || this;
    return i.type = "object", i.properties = [], i;
  }
  return Object.defineProperty(t.prototype, "children", {
    get: function() {
      return this.properties;
    },
    enumerable: !1,
    configurable: !0
  }), t;
}(ct);
function me(e) {
  return De(e) ? e ? {} : { not: {} } : e;
}
var Bs;
(function(e) {
  e[e.Key = 0] = "Key", e[e.Enum = 1] = "Enum";
})(Bs || (Bs = {}));
var fu = function() {
  function e(t, n) {
    t === void 0 && (t = -1), this.focusOffset = t, this.exclude = n, this.schemas = [];
  }
  return e.prototype.add = function(t) {
    this.schemas.push(t);
  }, e.prototype.merge = function(t) {
    Array.prototype.push.apply(this.schemas, t.schemas);
  }, e.prototype.include = function(t) {
    return (this.focusOffset === -1 || ga(t, this.focusOffset)) && t !== this.exclude;
  }, e.prototype.newSub = function() {
    return new e(-1, this.exclude);
  }, e;
}(), Lr = function() {
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
}(), pe = function() {
  function e() {
    this.problems = [], this.propertiesMatches = 0, this.propertiesValueMatches = 0, this.primaryValueMatches = 0, this.enumValueMatch = !1, this.enumValues = void 0;
  }
  return e.prototype.hasProblems = function() {
    return !!this.problems.length;
  }, e.prototype.mergeAll = function(t) {
    for (var n = 0, r = t; n < r.length; n++) {
      var i = r[n];
      this.merge(i);
    }
  }, e.prototype.merge = function(t) {
    this.problems = this.problems.concat(t.problems);
  }, e.prototype.mergeEnumValues = function(t) {
    if (!this.enumValueMatch && !t.enumValueMatch && this.enumValues && t.enumValues) {
      this.enumValues = this.enumValues.concat(t.enumValues);
      for (var n = 0, r = this.problems; n < r.length; n++) {
        var i = r[n];
        i.code === G.EnumValueMismatch && (i.message = j("enumWarning", "Value is not accepted. Valid values: {0}.", this.enumValues.map(function(s) {
          return JSON.stringify(s);
        }).join(", ")));
      }
    }
  }, e.prototype.mergePropertyMatch = function(t) {
    this.merge(t), this.propertiesMatches++, (t.enumValueMatch || !t.hasProblems() && t.propertiesMatches) && this.propertiesValueMatches++, t.enumValueMatch && t.enumValues && t.enumValues.length === 1 && this.primaryValueMatches++;
  }, e.prototype.compare = function(t) {
    var n = this.hasProblems();
    return n !== t.hasProblems() ? n ? -1 : 1 : this.enumValueMatch !== t.enumValueMatch ? t.enumValueMatch ? -1 : 1 : this.primaryValueMatches !== t.primaryValueMatches ? this.primaryValueMatches - t.primaryValueMatches : this.propertiesValueMatches !== t.propertiesValueMatches ? this.propertiesValueMatches - t.propertiesValueMatches : this.propertiesMatches - t.propertiesMatches;
  }, e;
}();
function hu(e, t) {
  return t === void 0 && (t = []), new ma(e, t, []);
}
function ot(e) {
  return Zl(e);
}
function Nr(e) {
  return Ql(e);
}
function ga(e, t, n) {
  return n === void 0 && (n = !1), t >= e.offset && t < e.offset + e.length || n && t === e.offset + e.length;
}
var ma = function() {
  function e(t, n, r) {
    n === void 0 && (n = []), r === void 0 && (r = []), this.root = t, this.syntaxErrors = n, this.comments = r;
  }
  return e.prototype.getNodeFromOffset = function(t, n) {
    if (n === void 0 && (n = !1), this.root)
      return Xl(this.root, t, n);
  }, e.prototype.visit = function(t) {
    if (this.root) {
      var n = function(r) {
        var i = t(r), s = r.children;
        if (Array.isArray(s))
          for (var a = 0; a < s.length && i; a++)
            i = n(s[a]);
        return i;
      };
      n(this.root);
    }
  }, e.prototype.validate = function(t, n, r) {
    if (r === void 0 && (r = we.Warning), this.root && n) {
      var i = new pe();
      return ue(this.root, n, i, Lr.instance), i.problems.map(function(s) {
        var a, o = X.create(t.positionAt(s.location.offset), t.positionAt(s.location.offset + s.location.length));
        return Be.create(o, s.message, (a = s.severity) !== null && a !== void 0 ? a : r, s.code);
      });
    }
  }, e.prototype.getMatchingSchemas = function(t, n, r) {
    n === void 0 && (n = -1);
    var i = new fu(n, r);
    return this.root && t && ue(this.root, t, new pe(), i), i.schemas;
  }, e;
}();
function ue(e, t, n, r) {
  if (!e || !r.include(e))
    return;
  var i = e;
  switch (i.type) {
    case "object":
      u(i, t, n, r);
      break;
    case "array":
      l(i, t, n, r);
      break;
    case "string":
      o(i, t, n);
      break;
    case "number":
      a(i, t, n);
      break;
    case "property":
      return ue(i.valueNode, t, n, r);
  }
  s(), r.add({ node: i, schema: t });
  function s() {
    function c(E) {
      return i.type === E || E === "integer" && i.type === "number" && i.isInteger;
    }
    if (Array.isArray(t.type) ? t.type.some(c) || n.problems.push({
      location: { offset: i.offset, length: i.length },
      message: t.errorMessage || j("typeArrayMismatchWarning", "Incorrect type. Expected one of {0}.", t.type.join(", "))
    }) : t.type && (c(t.type) || n.problems.push({
      location: { offset: i.offset, length: i.length },
      message: t.errorMessage || j("typeMismatchWarning", 'Incorrect type. Expected "{0}".', t.type)
    })), Array.isArray(t.allOf))
      for (var f = 0, d = t.allOf; f < d.length; f++) {
        var g = d[f];
        ue(i, me(g), n, r);
      }
    var m = me(t.not);
    if (m) {
      var p = new pe(), v = r.newSub();
      ue(i, m, p, v), p.hasProblems() || n.problems.push({
        location: { offset: i.offset, length: i.length },
        message: j("notSchemaWarning", "Matches a schema that is not allowed.")
      });
      for (var x = 0, b = v.schemas; x < b.length; x++) {
        var y = b[x];
        y.inverted = !y.inverted, r.add(y);
      }
    }
    var _ = function(E, P) {
      for (var O = [], V = void 0, k = 0, A = E; k < A.length; k++) {
        var T = A[k], F = me(T), B = new pe(), U = r.newSub();
        if (ue(i, F, B, U), B.hasProblems() || O.push(F), !V)
          V = { schema: F, validationResult: B, matchingSchemas: U };
        else if (!P && !B.hasProblems() && !V.validationResult.hasProblems())
          V.matchingSchemas.merge(U), V.validationResult.propertiesMatches += B.propertiesMatches, V.validationResult.propertiesValueMatches += B.propertiesValueMatches;
        else {
          var W = B.compare(V.validationResult);
          W > 0 ? V = { schema: F, validationResult: B, matchingSchemas: U } : W === 0 && (V.matchingSchemas.merge(U), V.validationResult.mergeEnumValues(B));
        }
      }
      return O.length > 1 && P && n.problems.push({
        location: { offset: i.offset, length: 1 },
        message: j("oneOfWarning", "Matches multiple schemas when only one must validate.")
      }), V && (n.merge(V.validationResult), n.propertiesMatches += V.validationResult.propertiesMatches, n.propertiesValueMatches += V.validationResult.propertiesValueMatches, r.merge(V.matchingSchemas)), O.length;
    };
    Array.isArray(t.anyOf) && _(t.anyOf, !1), Array.isArray(t.oneOf) && _(t.oneOf, !0);
    var N = function(E) {
      var P = new pe(), O = r.newSub();
      ue(i, me(E), P, O), n.merge(P), n.propertiesMatches += P.propertiesMatches, n.propertiesValueMatches += P.propertiesValueMatches, r.merge(O);
    }, w = function(E, P, O) {
      var V = me(E), k = new pe(), A = r.newSub();
      ue(i, V, k, A), r.merge(A), k.hasProblems() ? O && N(O) : P && N(P);
    }, S = me(t.if);
    if (S && w(S, me(t.then), me(t.else)), Array.isArray(t.enum)) {
      for (var C = ot(i), M = !1, I = 0, D = t.enum; I < D.length; I++) {
        var L = D[I];
        if (Dt(C, L)) {
          M = !0;
          break;
        }
      }
      n.enumValues = t.enum, n.enumValueMatch = M, M || n.problems.push({
        location: { offset: i.offset, length: i.length },
        code: G.EnumValueMismatch,
        message: t.errorMessage || j("enumWarning", "Value is not accepted. Valid values: {0}.", t.enum.map(function(E) {
          return JSON.stringify(E);
        }).join(", "))
      });
    }
    if (je(t.const)) {
      var C = ot(i);
      Dt(C, t.const) ? n.enumValueMatch = !0 : (n.problems.push({
        location: { offset: i.offset, length: i.length },
        code: G.EnumValueMismatch,
        message: t.errorMessage || j("constWarning", "Value must be {0}.", JSON.stringify(t.const))
      }), n.enumValueMatch = !1), n.enumValues = [t.const];
    }
    t.deprecationMessage && i.parent && n.problems.push({
      location: { offset: i.parent.offset, length: i.parent.length },
      severity: we.Warning,
      message: t.deprecationMessage,
      code: G.Deprecated
    });
  }
  function a(c, f, d, g) {
    var m = c.value;
    function p(I) {
      var D, L = /^(-?\d+)(?:\.(\d+))?(?:e([-+]\d+))?$/.exec(I.toString());
      return L && {
        value: Number(L[1] + (L[2] || "")),
        multiplier: (((D = L[2]) === null || D === void 0 ? void 0 : D.length) || 0) - (parseInt(L[3]) || 0)
      };
    }
    if (be(f.multipleOf)) {
      var v = -1;
      if (Number.isInteger(f.multipleOf))
        v = m % f.multipleOf;
      else {
        var x = p(f.multipleOf), b = p(m);
        if (x && b) {
          var y = Math.pow(10, Math.abs(b.multiplier - x.multiplier));
          b.multiplier < x.multiplier ? b.value *= y : x.value *= y, v = b.value % x.value;
        }
      }
      v !== 0 && d.problems.push({
        location: { offset: c.offset, length: c.length },
        message: j("multipleOfWarning", "Value is not divisible by {0}.", f.multipleOf)
      });
    }
    function _(I, D) {
      if (be(D))
        return D;
      if (De(D) && D)
        return I;
    }
    function N(I, D) {
      if (!De(D) || !D)
        return I;
    }
    var w = _(f.minimum, f.exclusiveMinimum);
    be(w) && m <= w && d.problems.push({
      location: { offset: c.offset, length: c.length },
      message: j("exclusiveMinimumWarning", "Value is below the exclusive minimum of {0}.", w)
    });
    var S = _(f.maximum, f.exclusiveMaximum);
    be(S) && m >= S && d.problems.push({
      location: { offset: c.offset, length: c.length },
      message: j("exclusiveMaximumWarning", "Value is above the exclusive maximum of {0}.", S)
    });
    var C = N(f.minimum, f.exclusiveMinimum);
    be(C) && m < C && d.problems.push({
      location: { offset: c.offset, length: c.length },
      message: j("minimumWarning", "Value is below the minimum of {0}.", C)
    });
    var M = N(f.maximum, f.exclusiveMaximum);
    be(M) && m > M && d.problems.push({
      location: { offset: c.offset, length: c.length },
      message: j("maximumWarning", "Value is above the maximum of {0}.", M)
    });
  }
  function o(c, f, d, g) {
    if (be(f.minLength) && c.value.length < f.minLength && d.problems.push({
      location: { offset: c.offset, length: c.length },
      message: j("minLengthWarning", "String is shorter than the minimum length of {0}.", f.minLength)
    }), be(f.maxLength) && c.value.length > f.maxLength && d.problems.push({
      location: { offset: c.offset, length: c.length },
      message: j("maxLengthWarning", "String is longer than the maximum length of {0}.", f.maxLength)
    }), Kl(f.pattern)) {
      var m = Sn(f.pattern);
      m != null && m.test(c.value) || d.problems.push({
        location: { offset: c.offset, length: c.length },
        message: f.patternErrorMessage || f.errorMessage || j("patternWarning", 'String does not match the pattern of "{0}".', f.pattern)
      });
    }
    if (f.format)
      switch (f.format) {
        case "uri":
        case "uri-reference":
          {
            var p = void 0;
            if (!c.value)
              p = j("uriEmpty", "URI expected.");
            else {
              var v = /^(([^:/?#]+?):)?(\/\/([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?/.exec(c.value);
              v ? !v[2] && f.format === "uri" && (p = j("uriSchemeMissing", "URI with a scheme is expected.")) : p = j("uriMissing", "URI is expected.");
            }
            p && d.problems.push({
              location: { offset: c.offset, length: c.length },
              message: f.patternErrorMessage || f.errorMessage || j("uriFormatWarning", "String is not a URI: {0}", p)
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
          var x = su[f.format];
          (!c.value || !x.pattern.exec(c.value)) && d.problems.push({
            location: { offset: c.offset, length: c.length },
            message: f.patternErrorMessage || f.errorMessage || x.errorMessage
          });
      }
  }
  function l(c, f, d, g) {
    if (Array.isArray(f.items)) {
      for (var m = f.items, p = 0; p < m.length; p++) {
        var v = m[p], x = me(v), b = new pe(), y = c.items[p];
        y ? (ue(y, x, b, g), d.mergePropertyMatch(b)) : c.items.length >= m.length && d.propertiesValueMatches++;
      }
      if (c.items.length > m.length)
        if (typeof f.additionalItems == "object")
          for (var _ = m.length; _ < c.items.length; _++) {
            var b = new pe();
            ue(c.items[_], f.additionalItems, b, g), d.mergePropertyMatch(b);
          }
        else
          f.additionalItems === !1 && d.problems.push({
            location: { offset: c.offset, length: c.length },
            message: j("additionalItemsWarning", "Array has too many items according to schema. Expected {0} or fewer.", m.length)
          });
    } else {
      var N = me(f.items);
      if (N)
        for (var w = 0, S = c.items; w < S.length; w++) {
          var y = S[w], b = new pe();
          ue(y, N, b, g), d.mergePropertyMatch(b);
        }
    }
    var C = me(f.contains);
    if (C) {
      var M = c.items.some(function(L) {
        var E = new pe();
        return ue(L, C, E, Lr.instance), !E.hasProblems();
      });
      M || d.problems.push({
        location: { offset: c.offset, length: c.length },
        message: f.errorMessage || j("requiredItemMissingWarning", "Array does not contain required item.")
      });
    }
    if (be(f.minItems) && c.items.length < f.minItems && d.problems.push({
      location: { offset: c.offset, length: c.length },
      message: j("minItemsWarning", "Array has too few items. Expected {0} or more.", f.minItems)
    }), be(f.maxItems) && c.items.length > f.maxItems && d.problems.push({
      location: { offset: c.offset, length: c.length },
      message: j("maxItemsWarning", "Array has too many items. Expected {0} or fewer.", f.maxItems)
    }), f.uniqueItems === !0) {
      var I = ot(c), D = I.some(function(L, E) {
        return E !== I.lastIndexOf(L);
      });
      D && d.problems.push({
        location: { offset: c.offset, length: c.length },
        message: j("uniqueItemsWarning", "Array has duplicate items.")
      });
    }
  }
  function u(c, f, d, g) {
    for (var m = /* @__PURE__ */ Object.create(null), p = [], v = 0, x = c.properties; v < x.length; v++) {
      var b = x[v], y = b.keyNode.value;
      m[y] = b.valueNode, p.push(y);
    }
    if (Array.isArray(f.required))
      for (var _ = 0, N = f.required; _ < N.length; _++) {
        var w = N[_];
        if (!m[w]) {
          var S = c.parent && c.parent.type === "property" && c.parent.keyNode, C = S ? { offset: S.offset, length: S.length } : { offset: c.offset, length: 1 };
          d.problems.push({
            location: C,
            message: j("MissingRequiredPropWarning", 'Missing property "{0}".', w)
          });
        }
      }
    var M = function(qr) {
      for (var Pn = p.indexOf(qr); Pn >= 0; )
        p.splice(Pn, 1), Pn = p.indexOf(qr);
    };
    if (f.properties)
      for (var I = 0, D = Object.keys(f.properties); I < D.length; I++) {
        var w = D[I];
        M(w);
        var L = f.properties[w], E = m[w];
        if (E)
          if (De(L))
            if (L)
              d.propertiesMatches++, d.propertiesValueMatches++;
            else {
              var b = E.parent;
              d.problems.push({
                location: { offset: b.keyNode.offset, length: b.keyNode.length },
                message: f.errorMessage || j("DisallowedExtraPropWarning", "Property {0} is not allowed.", w)
              });
            }
          else {
            var P = new pe();
            ue(E, L, P, g), d.mergePropertyMatch(P);
          }
      }
    if (f.patternProperties)
      for (var O = 0, V = Object.keys(f.patternProperties); O < V.length; O++)
        for (var k = V[O], A = Sn(k), T = 0, F = p.slice(0); T < F.length; T++) {
          var w = F[T];
          if (A != null && A.test(w)) {
            M(w);
            var E = m[w];
            if (E) {
              var L = f.patternProperties[k];
              if (De(L))
                if (L)
                  d.propertiesMatches++, d.propertiesValueMatches++;
                else {
                  var b = E.parent;
                  d.problems.push({
                    location: { offset: b.keyNode.offset, length: b.keyNode.length },
                    message: f.errorMessage || j("DisallowedExtraPropWarning", "Property {0} is not allowed.", w)
                  });
                }
              else {
                var P = new pe();
                ue(E, L, P, g), d.mergePropertyMatch(P);
              }
            }
          }
        }
    if (typeof f.additionalProperties == "object")
      for (var B = 0, U = p; B < U.length; B++) {
        var w = U[B], E = m[w];
        if (E) {
          var P = new pe();
          ue(E, f.additionalProperties, P, g), d.mergePropertyMatch(P);
        }
      }
    else if (f.additionalProperties === !1 && p.length > 0)
      for (var W = 0, _e = p; W < _e.length; W++) {
        var w = _e[W], E = m[w];
        if (E) {
          var b = E.parent;
          d.problems.push({
            location: { offset: b.keyNode.offset, length: b.keyNode.length },
            message: f.errorMessage || j("DisallowedExtraPropWarning", "Property {0} is not allowed.", w)
          });
        }
      }
    if (be(f.maxProperties) && c.properties.length > f.maxProperties && d.problems.push({
      location: { offset: c.offset, length: c.length },
      message: j("MaxPropWarning", "Object has more properties than limit of {0}.", f.maxProperties)
    }), be(f.minProperties) && c.properties.length < f.minProperties && d.problems.push({
      location: { offset: c.offset, length: c.length },
      message: j("MinPropWarning", "Object has fewer properties than the required number of {0}", f.minProperties)
    }), f.dependencies)
      for (var ee = 0, de = Object.keys(f.dependencies); ee < de.length; ee++) {
        var y = de[ee], We = m[y];
        if (We) {
          var Re = f.dependencies[y];
          if (Array.isArray(Re))
            for (var Rn = 0, Dr = Re; Rn < Dr.length; Rn++) {
              var Vr = Dr[Rn];
              m[Vr] ? d.propertiesValueMatches++ : d.problems.push({
                location: { offset: c.offset, length: c.length },
                message: j("RequiredDependentPropWarning", "Object is missing property {0} required by property {1}.", Vr, y)
              });
            }
          else {
            var L = me(Re);
            if (L) {
              var P = new pe();
              ue(c, L, P, g), d.mergePropertyMatch(P);
            }
          }
        }
      }
    var Or = me(f.propertyNames);
    if (Or)
      for (var Tn = 0, jr = c.properties; Tn < jr.length; Tn++) {
        var wa = jr[Tn], y = wa.keyNode;
        y && ue(y, Or, d, Lr.instance);
      }
  }
}
function du(e, t) {
  var n = [], r = -1, i = e.getText(), s = wt(i, !1), a = t && t.collectComments ? [] : void 0;
  function o() {
    for (; ; ) {
      var w = s.scan();
      switch (c(), w) {
        case 12:
        case 13:
          Array.isArray(a) && a.push(X.create(e.positionAt(s.getTokenOffset()), e.positionAt(s.getTokenOffset() + s.getTokenLength())));
          break;
        case 15:
        case 14:
          break;
        default:
          return w;
      }
    }
  }
  function l(w, S, C, M, I) {
    if (I === void 0 && (I = we.Error), n.length === 0 || C !== r) {
      var D = X.create(e.positionAt(C), e.positionAt(M));
      n.push(Be.create(D, w, I, S, e.languageId)), r = C;
    }
  }
  function u(w, S, C, M, I) {
    C === void 0 && (C = void 0), M === void 0 && (M = []), I === void 0 && (I = []);
    var D = s.getTokenOffset(), L = s.getTokenOffset() + s.getTokenLength();
    if (D === L && D > 0) {
      for (D--; D > 0 && /\s/.test(i.charAt(D)); )
        D--;
      L = D + 1;
    }
    if (l(w, S, D, L), C && f(C, !1), M.length + I.length > 0)
      for (var E = s.getToken(); E !== 17; ) {
        if (M.indexOf(E) !== -1) {
          o();
          break;
        } else if (I.indexOf(E) !== -1)
          break;
        E = o();
      }
    return C;
  }
  function c() {
    switch (s.getTokenError()) {
      case 4:
        return u(j("InvalidUnicode", "Invalid unicode sequence in string."), G.InvalidUnicode), !0;
      case 5:
        return u(j("InvalidEscapeCharacter", "Invalid escape character in string."), G.InvalidEscapeCharacter), !0;
      case 3:
        return u(j("UnexpectedEndOfNumber", "Unexpected end of number."), G.UnexpectedEndOfNumber), !0;
      case 1:
        return u(j("UnexpectedEndOfComment", "Unexpected end of comment."), G.UnexpectedEndOfComment), !0;
      case 2:
        return u(j("UnexpectedEndOfString", "Unexpected end of string."), G.UnexpectedEndOfString), !0;
      case 6:
        return u(j("InvalidCharacter", "Invalid characters in string. Control characters must be escaped."), G.InvalidCharacter), !0;
    }
    return !1;
  }
  function f(w, S) {
    return w.length = s.getTokenOffset() + s.getTokenLength() - w.offset, S && o(), w;
  }
  function d(w) {
    if (s.getToken() === 3) {
      var S = new ou(w, s.getTokenOffset());
      o();
      for (var C = !1; s.getToken() !== 4 && s.getToken() !== 17; ) {
        if (s.getToken() === 5) {
          C || u(j("ValueExpected", "Value expected"), G.ValueExpected);
          var M = s.getTokenOffset();
          if (o(), s.getToken() === 4) {
            C && l(j("TrailingComma", "Trailing comma"), G.TrailingComma, M, M + 1);
            continue;
          }
        } else
          C && u(j("ExpectedComma", "Expected comma"), G.CommaExpected);
        var I = y(S);
        I ? S.items.push(I) : u(j("PropertyExpected", "Value expected"), G.ValueExpected, void 0, [], [4, 5]), C = !0;
      }
      return s.getToken() !== 4 ? u(j("ExpectedCloseBracket", "Expected comma or closing bracket"), G.CommaOrCloseBacketExpected, S) : f(S, !0);
    }
  }
  var g = new $n(void 0, 0, 0);
  function m(w, S) {
    var C = new uu(w, s.getTokenOffset(), g), M = v(C);
    if (!M)
      if (s.getToken() === 16) {
        u(j("DoubleQuotesExpected", "Property keys must be doublequoted"), G.Undefined);
        var I = new $n(C, s.getTokenOffset(), s.getTokenLength());
        I.value = s.getTokenValue(), M = I, o();
      } else
        return;
    C.keyNode = M;
    var D = S[M.value];
    if (D ? (l(j("DuplicateKeyWarning", "Duplicate object key"), G.DuplicateKey, C.keyNode.offset, C.keyNode.offset + C.keyNode.length, we.Warning), typeof D == "object" && l(j("DuplicateKeyWarning", "Duplicate object key"), G.DuplicateKey, D.keyNode.offset, D.keyNode.offset + D.keyNode.length, we.Warning), S[M.value] = !0) : S[M.value] = C, s.getToken() === 6)
      C.colonOffset = s.getTokenOffset(), o();
    else if (u(j("ColonExpected", "Colon expected"), G.ColonExpected), s.getToken() === 10 && e.positionAt(M.offset + M.length).line < e.positionAt(s.getTokenOffset()).line)
      return C.length = M.length, C;
    var L = y(C);
    return L ? (C.valueNode = L, C.length = L.offset + L.length - C.offset, C) : u(j("ValueExpected", "Value expected"), G.ValueExpected, C, [], [2, 5]);
  }
  function p(w) {
    if (s.getToken() === 1) {
      var S = new cu(w, s.getTokenOffset()), C = /* @__PURE__ */ Object.create(null);
      o();
      for (var M = !1; s.getToken() !== 2 && s.getToken() !== 17; ) {
        if (s.getToken() === 5) {
          M || u(j("PropertyExpected", "Property expected"), G.PropertyExpected);
          var I = s.getTokenOffset();
          if (o(), s.getToken() === 2) {
            M && l(j("TrailingComma", "Trailing comma"), G.TrailingComma, I, I + 1);
            continue;
          }
        } else
          M && u(j("ExpectedComma", "Expected comma"), G.CommaExpected);
        var D = m(S, C);
        D ? S.properties.push(D) : u(j("PropertyExpected", "Property expected"), G.PropertyExpected, void 0, [], [2, 5]), M = !0;
      }
      return s.getToken() !== 2 ? u(j("ExpectedCloseBrace", "Expected comma or closing brace"), G.CommaOrCloseBraceExpected, S) : f(S, !0);
    }
  }
  function v(w) {
    if (s.getToken() === 10) {
      var S = new $n(w, s.getTokenOffset());
      return S.value = s.getTokenValue(), f(S, !0);
    }
  }
  function x(w) {
    if (s.getToken() === 11) {
      var S = new lu(w, s.getTokenOffset());
      if (s.getTokenError() === 0) {
        var C = s.getTokenValue();
        try {
          var M = JSON.parse(C);
          if (!be(M))
            return u(j("InvalidNumberFormat", "Invalid number format."), G.Undefined, S);
          S.value = M;
        } catch {
          return u(j("InvalidNumberFormat", "Invalid number format."), G.Undefined, S);
        }
        S.isInteger = C.indexOf(".") === -1;
      }
      return f(S, !0);
    }
  }
  function b(w) {
    switch (s.getToken()) {
      case 7:
        return f(new au(w, s.getTokenOffset()), !0);
      case 8:
        return f(new qs(w, !0, s.getTokenOffset()), !0);
      case 9:
        return f(new qs(w, !1, s.getTokenOffset()), !0);
      default:
        return;
    }
  }
  function y(w) {
    return d(w) || p(w) || v(w) || x(w) || b(w);
  }
  var _ = void 0, N = o();
  return N !== 17 && (_ = y(_), _ ? s.getToken() !== 17 && u(j("End of file expected", "End of file expected."), G.Undefined) : u(j("Invalid symbol", "Expected a JSON object, array or literal."), G.Undefined)), new ma(_, n, a);
}
function Ar(e, t, n) {
  if (e !== null && typeof e == "object") {
    var r = t + "	";
    if (Array.isArray(e)) {
      if (e.length === 0)
        return "[]";
      for (var i = `[
`, s = 0; s < e.length; s++)
        i += r + Ar(e[s], r, n), s < e.length - 1 && (i += ","), i += `
`;
      return i += t + "]", i;
    } else {
      var a = Object.keys(e);
      if (a.length === 0)
        return "{}";
      for (var i = `{
`, s = 0; s < a.length; s++) {
        var o = a[s];
        i += r + JSON.stringify(o) + ": " + Ar(e[o], r, n), s < a.length - 1 && (i += ","), i += `
`;
      }
      return i += t + "}", i;
    }
  }
  return n(e);
}
var Wn = Kt(), gu = function() {
  function e(t, n, r, i) {
    n === void 0 && (n = []), r === void 0 && (r = Promise), i === void 0 && (i = {}), this.schemaService = t, this.contributions = n, this.promiseConstructor = r, this.clientCapabilities = i;
  }
  return e.prototype.doResolve = function(t) {
    for (var n = this.contributions.length - 1; n >= 0; n--) {
      var r = this.contributions[n].resolveCompletion;
      if (r) {
        var i = r(t);
        if (i)
          return i;
      }
    }
    return this.promiseConstructor.resolve(t);
  }, e.prototype.doComplete = function(t, n, r) {
    var i = this, s = {
      items: [],
      isIncomplete: !1
    }, a = t.getText(), o = t.offsetAt(n), l = r.getNodeFromOffset(o, !0);
    if (this.isInComment(t, l ? l.offset : 0, o))
      return Promise.resolve(s);
    if (l && o === l.offset + l.length && o > 0) {
      var u = a[o - 1];
      (l.type === "object" && u === "}" || l.type === "array" && u === "]") && (l = l.parent);
    }
    var c = this.getCurrentWord(t, o), f;
    if (l && (l.type === "string" || l.type === "number" || l.type === "boolean" || l.type === "null"))
      f = X.create(t.positionAt(l.offset), t.positionAt(l.offset + l.length));
    else {
      var d = o - c.length;
      d > 0 && a[d - 1] === '"' && d--, f = X.create(t.positionAt(d), n);
    }
    var g = {}, m = {
      add: function(p) {
        var v = p.label, x = g[v];
        if (x)
          x.documentation || (x.documentation = p.documentation), x.detail || (x.detail = p.detail);
        else {
          if (v = v.replace(/[\n]/g, "↵"), v.length > 60) {
            var b = v.substr(0, 57).trim() + "...";
            g[b] || (v = b);
          }
          f && p.insertText !== void 0 && (p.textEdit = Me.replace(f, p.insertText)), p.label = v, g[v] = p, s.items.push(p);
        }
      },
      setAsIncomplete: function() {
        s.isIncomplete = !0;
      },
      error: function(p) {
        console.error(p);
      },
      log: function(p) {
        console.log(p);
      },
      getNumberOfProposals: function() {
        return s.items.length;
      }
    };
    return this.schemaService.getSchemaForResource(t.uri, r).then(function(p) {
      var v = [], x = !0, b = "", y = void 0;
      if (l && l.type === "string") {
        var _ = l.parent;
        _ && _.type === "property" && _.keyNode === l && (x = !_.valueNode, y = _, b = a.substr(l.offset + 1, l.length - 2), _ && (l = _.parent));
      }
      if (l && l.type === "object") {
        if (l.offset === o)
          return s;
        var N = l.properties;
        N.forEach(function(M) {
          (!y || y !== M) && (g[M.keyNode.value] = wr.create("__"));
        });
        var w = "";
        x && (w = i.evaluateSeparatorAfter(t, t.offsetAt(f.end))), p ? i.getPropertyCompletions(p, r, l, x, w, m) : i.getSchemaLessPropertyCompletions(r, l, b, m);
        var S = Nr(l);
        i.contributions.forEach(function(M) {
          var I = M.collectPropertyCompletions(t.uri, S, c, x, w === "", m);
          I && v.push(I);
        }), !p && c.length > 0 && a.charAt(o - c.length - 1) !== '"' && (m.add({
          kind: xe.Property,
          label: i.getLabelForValue(c),
          insertText: i.getInsertTextForProperty(c, void 0, !1, w),
          insertTextFormat: ie.Snippet,
          documentation: ""
        }), m.setAsIncomplete());
      }
      var C = {};
      return p ? i.getValueCompletions(p, r, l, o, t, m, C) : i.getSchemaLessValueCompletions(r, l, o, t, m), i.contributions.length > 0 && i.getContributedValueCompletions(r, l, o, t, m, v), i.promiseConstructor.all(v).then(function() {
        if (m.getNumberOfProposals() === 0) {
          var M = o;
          l && (l.type === "string" || l.type === "number" || l.type === "boolean" || l.type === "null") && (M = l.offset + l.length);
          var I = i.evaluateSeparatorAfter(t, M);
          i.addFillerValueCompletions(C, I, m);
        }
        return s;
      });
    });
  }, e.prototype.getPropertyCompletions = function(t, n, r, i, s, a) {
    var o = this, l = n.getMatchingSchemas(t.schema, r.offset);
    l.forEach(function(u) {
      if (u.node === r && !u.inverted) {
        var c = u.schema.properties;
        c && Object.keys(c).forEach(function(p) {
          var v = c[p];
          if (typeof v == "object" && !v.deprecationMessage && !v.doNotSuggest) {
            var x = {
              kind: xe.Property,
              label: p,
              insertText: o.getInsertTextForProperty(p, v, i, s),
              insertTextFormat: ie.Snippet,
              filterText: o.getFilterTextForValue(p),
              documentation: o.fromMarkup(v.markdownDescription) || v.description || ""
            };
            v.suggestSortText !== void 0 && (x.sortText = v.suggestSortText), x.insertText && Ht(x.insertText, "$1".concat(s)) && (x.command = {
              title: "Suggest",
              command: "editor.action.triggerSuggest"
            }), a.add(x);
          }
        });
        var f = u.schema.propertyNames;
        if (typeof f == "object" && !f.deprecationMessage && !f.doNotSuggest) {
          var d = function(p, v) {
            v === void 0 && (v = void 0);
            var x = {
              kind: xe.Property,
              label: p,
              insertText: o.getInsertTextForProperty(p, void 0, i, s),
              insertTextFormat: ie.Snippet,
              filterText: o.getFilterTextForValue(p),
              documentation: v || o.fromMarkup(f.markdownDescription) || f.description || ""
            };
            f.suggestSortText !== void 0 && (x.sortText = f.suggestSortText), x.insertText && Ht(x.insertText, "$1".concat(s)) && (x.command = {
              title: "Suggest",
              command: "editor.action.triggerSuggest"
            }), a.add(x);
          };
          if (f.enum)
            for (var g = 0; g < f.enum.length; g++) {
              var m = void 0;
              f.markdownEnumDescriptions && g < f.markdownEnumDescriptions.length ? m = o.fromMarkup(f.markdownEnumDescriptions[g]) : f.enumDescriptions && g < f.enumDescriptions.length && (m = f.enumDescriptions[g]), d(f.enum[g], m);
            }
          f.const && d(f.const);
        }
      }
    });
  }, e.prototype.getSchemaLessPropertyCompletions = function(t, n, r, i) {
    var s = this, a = function(l) {
      l.properties.forEach(function(u) {
        var c = u.keyNode.value;
        i.add({
          kind: xe.Property,
          label: c,
          insertText: s.getInsertTextForValue(c, ""),
          insertTextFormat: ie.Snippet,
          filterText: s.getFilterTextForValue(c),
          documentation: ""
        });
      });
    };
    if (n.parent)
      if (n.parent.type === "property") {
        var o = n.parent.keyNode.value;
        t.visit(function(l) {
          return l.type === "property" && l !== n.parent && l.keyNode.value === o && l.valueNode && l.valueNode.type === "object" && a(l.valueNode), !0;
        });
      } else
        n.parent.type === "array" && n.parent.items.forEach(function(l) {
          l.type === "object" && l !== n && a(l);
        });
    else
      n.type === "object" && i.add({
        kind: xe.Property,
        label: "$schema",
        insertText: this.getInsertTextForProperty("$schema", void 0, !0, ""),
        insertTextFormat: ie.Snippet,
        documentation: "",
        filterText: this.getFilterTextForValue("$schema")
      });
  }, e.prototype.getSchemaLessValueCompletions = function(t, n, r, i, s) {
    var a = this, o = r;
    if (n && (n.type === "string" || n.type === "number" || n.type === "boolean" || n.type === "null") && (o = n.offset + n.length, n = n.parent), !n) {
      s.add({
        kind: this.getSuggestionKind("object"),
        label: "Empty object",
        insertText: this.getInsertTextForValue({}, ""),
        insertTextFormat: ie.Snippet,
        documentation: ""
      }), s.add({
        kind: this.getSuggestionKind("array"),
        label: "Empty array",
        insertText: this.getInsertTextForValue([], ""),
        insertTextFormat: ie.Snippet,
        documentation: ""
      });
      return;
    }
    var l = this.evaluateSeparatorAfter(i, o), u = function(g) {
      g.parent && !ga(g.parent, r, !0) && s.add({
        kind: a.getSuggestionKind(g.type),
        label: a.getLabelTextForMatchingNode(g, i),
        insertText: a.getInsertTextForMatchingNode(g, i, l),
        insertTextFormat: ie.Snippet,
        documentation: ""
      }), g.type === "boolean" && a.addBooleanValueCompletion(!g.value, l, s);
    };
    if (n.type === "property" && r > (n.colonOffset || 0)) {
      var c = n.valueNode;
      if (c && (r > c.offset + c.length || c.type === "object" || c.type === "array"))
        return;
      var f = n.keyNode.value;
      t.visit(function(g) {
        return g.type === "property" && g.keyNode.value === f && g.valueNode && u(g.valueNode), !0;
      }), f === "$schema" && n.parent && !n.parent.parent && this.addDollarSchemaCompletions(l, s);
    }
    if (n.type === "array")
      if (n.parent && n.parent.type === "property") {
        var d = n.parent.keyNode.value;
        t.visit(function(g) {
          return g.type === "property" && g.keyNode.value === d && g.valueNode && g.valueNode.type === "array" && g.valueNode.items.forEach(u), !0;
        });
      } else
        n.items.forEach(u);
  }, e.prototype.getValueCompletions = function(t, n, r, i, s, a, o) {
    var l = i, u = void 0, c = void 0;
    if (r && (r.type === "string" || r.type === "number" || r.type === "boolean" || r.type === "null") && (l = r.offset + r.length, c = r, r = r.parent), !r) {
      this.addSchemaValueCompletions(t.schema, "", a, o);
      return;
    }
    if (r.type === "property" && i > (r.colonOffset || 0)) {
      var f = r.valueNode;
      if (f && i > f.offset + f.length)
        return;
      u = r.keyNode.value, r = r.parent;
    }
    if (r && (u !== void 0 || r.type === "array")) {
      for (var d = this.evaluateSeparatorAfter(s, l), g = n.getMatchingSchemas(t.schema, r.offset, c), m = 0, p = g; m < p.length; m++) {
        var v = p[m];
        if (v.node === r && !v.inverted && v.schema) {
          if (r.type === "array" && v.schema.items)
            if (Array.isArray(v.schema.items)) {
              var x = this.findItemAtOffset(r, s, i);
              x < v.schema.items.length && this.addSchemaValueCompletions(v.schema.items[x], d, a, o);
            } else
              this.addSchemaValueCompletions(v.schema.items, d, a, o);
          if (u !== void 0) {
            var b = !1;
            if (v.schema.properties) {
              var y = v.schema.properties[u];
              y && (b = !0, this.addSchemaValueCompletions(y, d, a, o));
            }
            if (v.schema.patternProperties && !b)
              for (var _ = 0, N = Object.keys(v.schema.patternProperties); _ < N.length; _++) {
                var w = N[_], S = Sn(w);
                if (S != null && S.test(u)) {
                  b = !0;
                  var y = v.schema.patternProperties[w];
                  this.addSchemaValueCompletions(y, d, a, o);
                }
              }
            if (v.schema.additionalProperties && !b) {
              var y = v.schema.additionalProperties;
              this.addSchemaValueCompletions(y, d, a, o);
            }
          }
        }
      }
      u === "$schema" && !r.parent && this.addDollarSchemaCompletions(d, a), o.boolean && (this.addBooleanValueCompletion(!0, d, a), this.addBooleanValueCompletion(!1, d, a)), o.null && this.addNullValueCompletion(d, a);
    }
  }, e.prototype.getContributedValueCompletions = function(t, n, r, i, s, a) {
    if (!n)
      this.contributions.forEach(function(c) {
        var f = c.collectDefaultCompletions(i.uri, s);
        f && a.push(f);
      });
    else if ((n.type === "string" || n.type === "number" || n.type === "boolean" || n.type === "null") && (n = n.parent), n && n.type === "property" && r > (n.colonOffset || 0)) {
      var o = n.keyNode.value, l = n.valueNode;
      if ((!l || r <= l.offset + l.length) && n.parent) {
        var u = Nr(n.parent);
        this.contributions.forEach(function(c) {
          var f = c.collectValueCompletions(i.uri, u, o, s);
          f && a.push(f);
        });
      }
    }
  }, e.prototype.addSchemaValueCompletions = function(t, n, r, i) {
    var s = this;
    typeof t == "object" && (this.addEnumValueCompletions(t, n, r), this.addDefaultValueCompletions(t, n, r), this.collectTypes(t, i), Array.isArray(t.allOf) && t.allOf.forEach(function(a) {
      return s.addSchemaValueCompletions(a, n, r, i);
    }), Array.isArray(t.anyOf) && t.anyOf.forEach(function(a) {
      return s.addSchemaValueCompletions(a, n, r, i);
    }), Array.isArray(t.oneOf) && t.oneOf.forEach(function(a) {
      return s.addSchemaValueCompletions(a, n, r, i);
    }));
  }, e.prototype.addDefaultValueCompletions = function(t, n, r, i) {
    var s = this;
    i === void 0 && (i = 0);
    var a = !1;
    if (je(t.default)) {
      for (var o = t.type, l = t.default, u = i; u > 0; u--)
        l = [l], o = "array";
      r.add({
        kind: this.getSuggestionKind(o),
        label: this.getLabelForValue(l),
        insertText: this.getInsertTextForValue(l, n),
        insertTextFormat: ie.Snippet,
        detail: Wn("json.suggest.default", "Default value")
      }), a = !0;
    }
    Array.isArray(t.examples) && t.examples.forEach(function(c) {
      for (var f = t.type, d = c, g = i; g > 0; g--)
        d = [d], f = "array";
      r.add({
        kind: s.getSuggestionKind(f),
        label: s.getLabelForValue(d),
        insertText: s.getInsertTextForValue(d, n),
        insertTextFormat: ie.Snippet
      }), a = !0;
    }), Array.isArray(t.defaultSnippets) && t.defaultSnippets.forEach(function(c) {
      var f = t.type, d = c.body, g = c.label, m, p;
      if (je(d)) {
        t.type;
        for (var v = i; v > 0; v--)
          d = [d];
        m = s.getInsertTextForSnippetValue(d, n), p = s.getFilterTextForSnippetValue(d), g = g || s.getLabelForSnippetValue(d);
      } else if (typeof c.bodyText == "string") {
        for (var x = "", b = "", y = "", v = i; v > 0; v--)
          x = x + y + `[
`, b = b + `
` + y + "]", y += "	", f = "array";
        m = x + y + c.bodyText.split(`
`).join(`
` + y) + b + n, g = g || m, p = m.replace(/[\n]/g, "");
      } else
        return;
      r.add({
        kind: s.getSuggestionKind(f),
        label: g,
        documentation: s.fromMarkup(c.markdownDescription) || c.description,
        insertText: m,
        insertTextFormat: ie.Snippet,
        filterText: p
      }), a = !0;
    }), !a && typeof t.items == "object" && !Array.isArray(t.items) && i < 5 && this.addDefaultValueCompletions(t.items, n, r, i + 1);
  }, e.prototype.addEnumValueCompletions = function(t, n, r) {
    if (je(t.const) && r.add({
      kind: this.getSuggestionKind(t.type),
      label: this.getLabelForValue(t.const),
      insertText: this.getInsertTextForValue(t.const, n),
      insertTextFormat: ie.Snippet,
      documentation: this.fromMarkup(t.markdownDescription) || t.description
    }), Array.isArray(t.enum))
      for (var i = 0, s = t.enum.length; i < s; i++) {
        var a = t.enum[i], o = this.fromMarkup(t.markdownDescription) || t.description;
        t.markdownEnumDescriptions && i < t.markdownEnumDescriptions.length && this.doesSupportMarkdown() ? o = this.fromMarkup(t.markdownEnumDescriptions[i]) : t.enumDescriptions && i < t.enumDescriptions.length && (o = t.enumDescriptions[i]), r.add({
          kind: this.getSuggestionKind(t.type),
          label: this.getLabelForValue(a),
          insertText: this.getInsertTextForValue(a, n),
          insertTextFormat: ie.Snippet,
          documentation: o
        });
      }
  }, e.prototype.collectTypes = function(t, n) {
    if (!(Array.isArray(t.enum) || je(t.const))) {
      var r = t.type;
      Array.isArray(r) ? r.forEach(function(i) {
        return n[i] = !0;
      }) : r && (n[r] = !0);
    }
  }, e.prototype.addFillerValueCompletions = function(t, n, r) {
    t.object && r.add({
      kind: this.getSuggestionKind("object"),
      label: "{}",
      insertText: this.getInsertTextForGuessedValue({}, n),
      insertTextFormat: ie.Snippet,
      detail: Wn("defaults.object", "New object"),
      documentation: ""
    }), t.array && r.add({
      kind: this.getSuggestionKind("array"),
      label: "[]",
      insertText: this.getInsertTextForGuessedValue([], n),
      insertTextFormat: ie.Snippet,
      detail: Wn("defaults.array", "New array"),
      documentation: ""
    });
  }, e.prototype.addBooleanValueCompletion = function(t, n, r) {
    r.add({
      kind: this.getSuggestionKind("boolean"),
      label: t ? "true" : "false",
      insertText: this.getInsertTextForValue(t, n),
      insertTextFormat: ie.Snippet,
      documentation: ""
    });
  }, e.prototype.addNullValueCompletion = function(t, n) {
    n.add({
      kind: this.getSuggestionKind("null"),
      label: "null",
      insertText: "null" + t,
      insertTextFormat: ie.Snippet,
      documentation: ""
    });
  }, e.prototype.addDollarSchemaCompletions = function(t, n) {
    var r = this, i = this.schemaService.getRegisteredSchemaIds(function(s) {
      return s === "http" || s === "https";
    });
    i.forEach(function(s) {
      return n.add({
        kind: xe.Module,
        label: r.getLabelForValue(s),
        filterText: r.getFilterTextForValue(s),
        insertText: r.getInsertTextForValue(s, t),
        insertTextFormat: ie.Snippet,
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
    var n = JSON.stringify(t);
    return n.replace(/\$\{\d+:([^}]+)\}|\$\d+/g, "$1");
  }, e.prototype.getInsertTextForPlainText = function(t) {
    return t.replace(/[\\\$\}]/g, "\\$&");
  }, e.prototype.getInsertTextForValue = function(t, n) {
    var r = JSON.stringify(t, null, "	");
    return r === "{}" ? "{$1}" + n : r === "[]" ? "[$1]" + n : this.getInsertTextForPlainText(r + n);
  }, e.prototype.getInsertTextForSnippetValue = function(t, n) {
    var r = function(i) {
      return typeof i == "string" && i[0] === "^" ? i.substr(1) : JSON.stringify(i);
    };
    return Ar(t, "", r) + n;
  }, e.prototype.getInsertTextForGuessedValue = function(t, n) {
    switch (typeof t) {
      case "object":
        return t === null ? "${1:null}" + n : this.getInsertTextForValue(t, n);
      case "string":
        var r = JSON.stringify(t);
        return r = r.substr(1, r.length - 2), r = this.getInsertTextForPlainText(r), '"${1:' + r + '}"' + n;
      case "number":
      case "boolean":
        return "${1:" + JSON.stringify(t) + "}" + n;
    }
    return this.getInsertTextForValue(t, n);
  }, e.prototype.getSuggestionKind = function(t) {
    if (Array.isArray(t)) {
      var n = t;
      t = n.length > 0 ? n[0] : void 0;
    }
    if (!t)
      return xe.Value;
    switch (t) {
      case "string":
        return xe.Value;
      case "object":
        return xe.Module;
      case "property":
        return xe.Property;
      default:
        return xe.Value;
    }
  }, e.prototype.getLabelTextForMatchingNode = function(t, n) {
    switch (t.type) {
      case "array":
        return "[]";
      case "object":
        return "{}";
      default:
        var r = n.getText().substr(t.offset, t.length);
        return r;
    }
  }, e.prototype.getInsertTextForMatchingNode = function(t, n, r) {
    switch (t.type) {
      case "array":
        return this.getInsertTextForValue([], r);
      case "object":
        return this.getInsertTextForValue({}, r);
      default:
        var i = n.getText().substr(t.offset, t.length) + r;
        return this.getInsertTextForPlainText(i);
    }
  }, e.prototype.getInsertTextForProperty = function(t, n, r, i) {
    var s = this.getInsertTextForValue(t, "");
    if (!r)
      return s;
    var a = s + ": ", o, l = 0;
    if (n) {
      if (Array.isArray(n.defaultSnippets)) {
        if (n.defaultSnippets.length === 1) {
          var u = n.defaultSnippets[0].body;
          je(u) && (o = this.getInsertTextForSnippetValue(u, ""));
        }
        l += n.defaultSnippets.length;
      }
      if (n.enum && (!o && n.enum.length === 1 && (o = this.getInsertTextForGuessedValue(n.enum[0], "")), l += n.enum.length), je(n.default) && (o || (o = this.getInsertTextForGuessedValue(n.default, "")), l++), Array.isArray(n.examples) && n.examples.length && (o || (o = this.getInsertTextForGuessedValue(n.examples[0], "")), l += n.examples.length), l === 0) {
        var c = Array.isArray(n.type) ? n.type[0] : n.type;
        switch (c || (n.properties ? c = "object" : n.items && (c = "array")), c) {
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
  }, e.prototype.getCurrentWord = function(t, n) {
    for (var r = n - 1, i = t.getText(); r >= 0 && ` 	
\r\v":{[,]}`.indexOf(i.charAt(r)) === -1; )
      r--;
    return i.substring(r + 1, n);
  }, e.prototype.evaluateSeparatorAfter = function(t, n) {
    var r = wt(t.getText(), !0);
    r.setPosition(n);
    var i = r.scan();
    switch (i) {
      case 5:
      case 2:
      case 4:
      case 17:
        return "";
      default:
        return ",";
    }
  }, e.prototype.findItemAtOffset = function(t, n, r) {
    for (var i = wt(n.getText(), !0), s = t.items, a = s.length - 1; a >= 0; a--) {
      var o = s[a];
      if (r > o.offset + o.length) {
        i.setPosition(o.offset + o.length);
        var l = i.scan();
        return l === 5 && r >= i.getTokenOffset() + i.getTokenLength() ? a + 1 : a;
      } else if (r >= o.offset)
        return a;
    }
    return 0;
  }, e.prototype.isInComment = function(t, n, r) {
    var i = wt(t.getText(), !1);
    i.setPosition(n);
    for (var s = i.scan(); s !== 17 && i.getTokenOffset() + i.getTokenLength() < r; )
      s = i.scan();
    return (s === 12 || s === 13) && i.getTokenOffset() <= r;
  }, e.prototype.fromMarkup = function(t) {
    if (t && this.doesSupportMarkdown())
      return {
        kind: $e.Markdown,
        value: t
      };
  }, e.prototype.doesSupportMarkdown = function() {
    if (!je(this.supportsMarkdown)) {
      var t = this.clientCapabilities.textDocument && this.clientCapabilities.textDocument.completion;
      this.supportsMarkdown = t && t.completionItem && Array.isArray(t.completionItem.documentationFormat) && t.completionItem.documentationFormat.indexOf($e.Markdown) !== -1;
    }
    return this.supportsMarkdown;
  }, e.prototype.doesSupportsCommitCharacters = function() {
    if (!je(this.supportsCommitCharacters)) {
      var t = this.clientCapabilities.textDocument && this.clientCapabilities.textDocument.completion;
      this.supportsCommitCharacters = t && t.completionItem && !!t.completionItem.commitCharactersSupport;
    }
    return this.supportsCommitCharacters;
  }, e;
}(), mu = function() {
  function e(t, n, r) {
    n === void 0 && (n = []), this.schemaService = t, this.contributions = n, this.promise = r || Promise;
  }
  return e.prototype.doHover = function(t, n, r) {
    var i = t.offsetAt(n), s = r.getNodeFromOffset(i);
    if (!s || (s.type === "object" || s.type === "array") && i > s.offset + 1 && i < s.offset + s.length - 1)
      return this.promise.resolve(null);
    var a = s;
    if (s.type === "string") {
      var o = s.parent;
      if (o && o.type === "property" && o.keyNode === s && (s = o.valueNode, !s))
        return this.promise.resolve(null);
    }
    for (var l = X.create(t.positionAt(a.offset), t.positionAt(a.offset + a.length)), u = function(m) {
      var p = {
        contents: m,
        range: l
      };
      return p;
    }, c = Nr(s), f = this.contributions.length - 1; f >= 0; f--) {
      var d = this.contributions[f], g = d.getInfoContribution(t.uri, c);
      if (g)
        return g.then(function(m) {
          return u(m);
        });
    }
    return this.schemaService.getSchemaForResource(t.uri, r).then(function(m) {
      if (m && s) {
        var p = r.getMatchingSchemas(m.schema, s.offset), v = void 0, x = void 0, b = void 0, y = void 0;
        p.every(function(N) {
          if (N.node === s && !N.inverted && N.schema && (v = v || N.schema.title, x = x || N.schema.markdownDescription || Hn(N.schema.description), N.schema.enum)) {
            var w = N.schema.enum.indexOf(ot(s));
            N.schema.markdownEnumDescriptions ? b = N.schema.markdownEnumDescriptions[w] : N.schema.enumDescriptions && (b = Hn(N.schema.enumDescriptions[w])), b && (y = N.schema.enum[w], typeof y != "string" && (y = JSON.stringify(y)));
          }
          return !0;
        });
        var _ = "";
        return v && (_ = Hn(v)), x && (_.length > 0 && (_ += `

`), _ += x), b && (_.length > 0 && (_ += `

`), _ += "`".concat(pu(y), "`: ").concat(b)), u([_]);
      }
      return null;
    });
  }, e;
}();
function Hn(e) {
  if (e) {
    var t = e.replace(/([^\n\r])(\r?\n)([^\n\r])/gm, `$1

$3`);
    return t.replace(/[\\`*_{}[\]()#+\-.!]/g, "\\$&");
  }
}
function pu(e) {
  return e.indexOf("`") !== -1 ? "`` " + e + " ``" : e;
}
var vu = Kt(), bu = function() {
  function e(t, n) {
    this.jsonSchemaService = t, this.promise = n, this.validationEnabled = !0;
  }
  return e.prototype.configure = function(t) {
    t && (this.validationEnabled = t.validate !== !1, this.commentSeverity = t.allowComments ? void 0 : we.Error);
  }, e.prototype.doValidation = function(t, n, r, i) {
    var s = this;
    if (!this.validationEnabled)
      return this.promise.resolve([]);
    var a = [], o = {}, l = function(d) {
      var g = d.range.start.line + " " + d.range.start.character + " " + d.message;
      o[g] || (o[g] = !0, a.push(d));
    }, u = function(d) {
      var g = r != null && r.trailingCommas ? rn(r.trailingCommas) : we.Error, m = r != null && r.comments ? rn(r.comments) : s.commentSeverity, p = r != null && r.schemaValidation ? rn(r.schemaValidation) : we.Warning, v = r != null && r.schemaRequest ? rn(r.schemaRequest) : we.Warning;
      if (d) {
        if (d.errors.length && n.root && v) {
          var x = n.root, b = x.type === "object" ? x.properties[0] : void 0;
          if (b && b.keyNode.value === "$schema") {
            var y = b.valueNode || b, _ = X.create(t.positionAt(y.offset), t.positionAt(y.offset + y.length));
            l(Be.create(_, d.errors[0], v, G.SchemaResolveError));
          } else {
            var _ = X.create(t.positionAt(x.offset), t.positionAt(x.offset + 1));
            l(Be.create(_, d.errors[0], v, G.SchemaResolveError));
          }
        } else if (p) {
          var N = n.validate(t, d.schema, p);
          N && N.forEach(l);
        }
        pa(d.schema) && (m = void 0), va(d.schema) && (g = void 0);
      }
      for (var w = 0, S = n.syntaxErrors; w < S.length; w++) {
        var C = S[w];
        if (C.code === G.TrailingComma) {
          if (typeof g != "number")
            continue;
          C.severity = g;
        }
        l(C);
      }
      if (typeof m == "number") {
        var M = vu("InvalidCommentToken", "Comments are not permitted in JSON.");
        n.comments.forEach(function(I) {
          l(Be.create(I, M, m, G.CommentNotPermitted));
        });
      }
      return a;
    };
    if (i) {
      var c = i.id || "schemaservice://untitled/" + yu++, f = this.jsonSchemaService.registerExternalSchema(c, [], i);
      return f.getResolvedSchema().then(function(d) {
        return u(d);
      });
    }
    return this.jsonSchemaService.getSchemaForResource(t.uri, n).then(function(d) {
      return u(d);
    });
  }, e.prototype.getLanguageStatus = function(t, n) {
    return { schemas: this.jsonSchemaService.getSchemaURIsForResource(t.uri, n) };
  }, e;
}(), yu = 0;
function pa(e) {
  if (e && typeof e == "object") {
    if (De(e.allowComments))
      return e.allowComments;
    if (e.allOf)
      for (var t = 0, n = e.allOf; t < n.length; t++) {
        var r = n[t], i = pa(r);
        if (De(i))
          return i;
      }
  }
}
function va(e) {
  if (e && typeof e == "object") {
    if (De(e.allowTrailingCommas))
      return e.allowTrailingCommas;
    var t = e;
    if (De(t.allowsTrailingCommas))
      return t.allowsTrailingCommas;
    if (e.allOf)
      for (var n = 0, r = e.allOf; n < r.length; n++) {
        var i = r[n], s = va(i);
        if (De(s))
          return s;
      }
  }
}
function rn(e) {
  switch (e) {
    case "error":
      return we.Error;
    case "warning":
      return we.Warning;
    case "ignore":
      return;
  }
}
var Us = 48, xu = 57, wu = 65, sn = 97, _u = 102;
function te(e) {
  return e < Us ? 0 : e <= xu ? e - Us : (e < sn && (e += sn - wu), e >= sn && e <= _u ? e - sn + 10 : 0);
}
function Su(e) {
  if (e[0] === "#")
    switch (e.length) {
      case 4:
        return {
          red: te(e.charCodeAt(1)) * 17 / 255,
          green: te(e.charCodeAt(2)) * 17 / 255,
          blue: te(e.charCodeAt(3)) * 17 / 255,
          alpha: 1
        };
      case 5:
        return {
          red: te(e.charCodeAt(1)) * 17 / 255,
          green: te(e.charCodeAt(2)) * 17 / 255,
          blue: te(e.charCodeAt(3)) * 17 / 255,
          alpha: te(e.charCodeAt(4)) * 17 / 255
        };
      case 7:
        return {
          red: (te(e.charCodeAt(1)) * 16 + te(e.charCodeAt(2))) / 255,
          green: (te(e.charCodeAt(3)) * 16 + te(e.charCodeAt(4))) / 255,
          blue: (te(e.charCodeAt(5)) * 16 + te(e.charCodeAt(6))) / 255,
          alpha: 1
        };
      case 9:
        return {
          red: (te(e.charCodeAt(1)) * 16 + te(e.charCodeAt(2))) / 255,
          green: (te(e.charCodeAt(3)) * 16 + te(e.charCodeAt(4))) / 255,
          blue: (te(e.charCodeAt(5)) * 16 + te(e.charCodeAt(6))) / 255,
          alpha: (te(e.charCodeAt(7)) * 16 + te(e.charCodeAt(8))) / 255
        };
    }
}
var Lu = function() {
  function e(t) {
    this.schemaService = t;
  }
  return e.prototype.findDocumentSymbols = function(t, n, r) {
    var i = this;
    r === void 0 && (r = { resultLimit: Number.MAX_VALUE });
    var s = n.root;
    if (!s)
      return [];
    var a = r.resultLimit || Number.MAX_VALUE, o = t.uri;
    if ((o === "vscode://defaultsettings/keybindings.json" || Ht(o.toLowerCase(), "/user/keybindings.json")) && s.type === "array") {
      for (var l = [], u = 0, c = s.items; u < c.length; u++) {
        var f = c[u];
        if (f.type === "object")
          for (var d = 0, g = f.properties; d < g.length; d++) {
            var m = g[d];
            if (m.keyNode.value === "key" && m.valueNode) {
              var p = zt.create(t.uri, Je(t, f));
              if (l.push({ name: ot(m.valueNode), kind: Pe.Function, location: p }), a--, a <= 0)
                return r && r.onResultLimitExceeded && r.onResultLimitExceeded(o), l;
            }
          }
      }
      return l;
    }
    for (var v = [
      { node: s, containerName: "" }
    ], x = 0, b = !1, y = [], _ = function(w, S) {
      w.type === "array" ? w.items.forEach(function(C) {
        C && v.push({ node: C, containerName: S });
      }) : w.type === "object" && w.properties.forEach(function(C) {
        var M = C.valueNode;
        if (M)
          if (a > 0) {
            a--;
            var I = zt.create(t.uri, Je(t, C)), D = S ? S + "." + C.keyNode.value : C.keyNode.value;
            y.push({ name: i.getKeyLabel(C), kind: i.getSymbolKind(M.type), location: I, containerName: S }), v.push({ node: M, containerName: D });
          } else
            b = !0;
      });
    }; x < v.length; ) {
      var N = v[x++];
      _(N.node, N.containerName);
    }
    return b && r && r.onResultLimitExceeded && r.onResultLimitExceeded(o), y;
  }, e.prototype.findDocumentSymbols2 = function(t, n, r) {
    var i = this;
    r === void 0 && (r = { resultLimit: Number.MAX_VALUE });
    var s = n.root;
    if (!s)
      return [];
    var a = r.resultLimit || Number.MAX_VALUE, o = t.uri;
    if ((o === "vscode://defaultsettings/keybindings.json" || Ht(o.toLowerCase(), "/user/keybindings.json")) && s.type === "array") {
      for (var l = [], u = 0, c = s.items; u < c.length; u++) {
        var f = c[u];
        if (f.type === "object")
          for (var d = 0, g = f.properties; d < g.length; d++) {
            var m = g[d];
            if (m.keyNode.value === "key" && m.valueNode) {
              var p = Je(t, f), v = Je(t, m.keyNode);
              if (l.push({ name: ot(m.valueNode), kind: Pe.Function, range: p, selectionRange: v }), a--, a <= 0)
                return r && r.onResultLimitExceeded && r.onResultLimitExceeded(o), l;
            }
          }
      }
      return l;
    }
    for (var x = [], b = [
      { node: s, result: x }
    ], y = 0, _ = !1, N = function(S, C) {
      S.type === "array" ? S.items.forEach(function(M, I) {
        if (M)
          if (a > 0) {
            a--;
            var D = Je(t, M), L = D, E = String(I), P = { name: E, kind: i.getSymbolKind(M.type), range: D, selectionRange: L, children: [] };
            C.push(P), b.push({ result: P.children, node: M });
          } else
            _ = !0;
      }) : S.type === "object" && S.properties.forEach(function(M) {
        var I = M.valueNode;
        if (I)
          if (a > 0) {
            a--;
            var D = Je(t, M), L = Je(t, M.keyNode), E = [], P = { name: i.getKeyLabel(M), kind: i.getSymbolKind(I.type), range: D, selectionRange: L, children: E, detail: i.getDetail(I) };
            C.push(P), b.push({ result: E, node: I });
          } else
            _ = !0;
      });
    }; y < b.length; ) {
      var w = b[y++];
      N(w.node, w.result);
    }
    return _ && r && r.onResultLimitExceeded && r.onResultLimitExceeded(o), x;
  }, e.prototype.getSymbolKind = function(t) {
    switch (t) {
      case "object":
        return Pe.Module;
      case "string":
        return Pe.String;
      case "number":
        return Pe.Number;
      case "array":
        return Pe.Array;
      case "boolean":
        return Pe.Boolean;
      default:
        return Pe.Variable;
    }
  }, e.prototype.getKeyLabel = function(t) {
    var n = t.keyNode.value;
    return n && (n = n.replace(/[\n]/g, "↵")), n && n.trim() ? n : '"'.concat(n, '"');
  }, e.prototype.getDetail = function(t) {
    if (t) {
      if (t.type === "boolean" || t.type === "number" || t.type === "null" || t.type === "string")
        return String(t.value);
      if (t.type === "array")
        return t.children.length ? void 0 : "[]";
      if (t.type === "object")
        return t.children.length ? void 0 : "{}";
    }
  }, e.prototype.findDocumentColors = function(t, n, r) {
    return this.schemaService.getSchemaForResource(t.uri, n).then(function(i) {
      var s = [];
      if (i)
        for (var a = r && typeof r.resultLimit == "number" ? r.resultLimit : Number.MAX_VALUE, o = n.getMatchingSchemas(i.schema), l = {}, u = 0, c = o; u < c.length; u++) {
          var f = c[u];
          if (!f.inverted && f.schema && (f.schema.format === "color" || f.schema.format === "color-hex") && f.node && f.node.type === "string") {
            var d = String(f.node.offset);
            if (!l[d]) {
              var g = Su(ot(f.node));
              if (g) {
                var m = Je(t, f.node);
                s.push({ color: g, range: m });
              }
              if (l[d] = !0, a--, a <= 0)
                return r && r.onResultLimitExceeded && r.onResultLimitExceeded(t.uri), s;
            }
          }
        }
      return s;
    });
  }, e.prototype.getColorPresentations = function(t, n, r, i) {
    var s = [], a = Math.round(r.red * 255), o = Math.round(r.green * 255), l = Math.round(r.blue * 255);
    function u(f) {
      var d = f.toString(16);
      return d.length !== 2 ? "0" + d : d;
    }
    var c;
    return r.alpha === 1 ? c = "#".concat(u(a)).concat(u(o)).concat(u(l)) : c = "#".concat(u(a)).concat(u(o)).concat(u(l)).concat(u(Math.round(r.alpha * 255))), s.push({ label: c, textEdit: Me.replace(i, JSON.stringify(c)) }), s;
  }, e;
}();
function Je(e, t) {
  return X.create(e.positionAt(t.offset), e.positionAt(t.offset + t.length));
}
var $ = Kt(), Cr = {
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
}, Nu = {
  id: $("schema.json.id", "A unique identifier for the schema."),
  $schema: $("schema.json.$schema", "The schema to verify this document against."),
  title: $("schema.json.title", "A descriptive title of the element."),
  description: $("schema.json.description", "A long description of the element. Used in hover menus and suggestions."),
  default: $("schema.json.default", "A default value. Used by suggestions."),
  multipleOf: $("schema.json.multipleOf", "A number that should cleanly divide the current value (i.e. have no remainder)."),
  maximum: $("schema.json.maximum", "The maximum numerical value, inclusive by default."),
  exclusiveMaximum: $("schema.json.exclusiveMaximum", "Makes the maximum property exclusive."),
  minimum: $("schema.json.minimum", "The minimum numerical value, inclusive by default."),
  exclusiveMinimum: $("schema.json.exclusiveMininum", "Makes the minimum property exclusive."),
  maxLength: $("schema.json.maxLength", "The maximum length of a string."),
  minLength: $("schema.json.minLength", "The minimum length of a string."),
  pattern: $("schema.json.pattern", "A regular expression to match the string against. It is not implicitly anchored."),
  additionalItems: $("schema.json.additionalItems", "For arrays, only when items is set as an array. If it is a schema, then this schema validates items after the ones specified by the items array. If it is false, then additional items will cause validation to fail."),
  items: $("schema.json.items", "For arrays. Can either be a schema to validate every element against or an array of schemas to validate each item against in order (the first schema will validate the first element, the second schema will validate the second element, and so on."),
  maxItems: $("schema.json.maxItems", "The maximum number of items that can be inside an array. Inclusive."),
  minItems: $("schema.json.minItems", "The minimum number of items that can be inside an array. Inclusive."),
  uniqueItems: $("schema.json.uniqueItems", "If all of the items in the array must be unique. Defaults to false."),
  maxProperties: $("schema.json.maxProperties", "The maximum number of properties an object can have. Inclusive."),
  minProperties: $("schema.json.minProperties", "The minimum number of properties an object can have. Inclusive."),
  required: $("schema.json.required", "An array of strings that lists the names of all properties required on this object."),
  additionalProperties: $("schema.json.additionalProperties", "Either a schema or a boolean. If a schema, then used to validate all properties not matched by 'properties' or 'patternProperties'. If false, then any properties not matched by either will cause this schema to fail."),
  definitions: $("schema.json.definitions", "Not used for validation. Place subschemas here that you wish to reference inline with $ref."),
  properties: $("schema.json.properties", "A map of property names to schemas for each property."),
  patternProperties: $("schema.json.patternProperties", "A map of regular expressions on property names to schemas for matching properties."),
  dependencies: $("schema.json.dependencies", "A map of property names to either an array of property names or a schema. An array of property names means the property named in the key depends on the properties in the array being present in the object in order to be valid. If the value is a schema, then the schema is only applied to the object if the property in the key exists on the object."),
  enum: $("schema.json.enum", "The set of literal values that are valid."),
  type: $("schema.json.type", "Either a string of one of the basic schema types (number, integer, null, array, object, boolean, string) or an array of strings specifying a subset of those types."),
  format: $("schema.json.format", "Describes the format expected for the value."),
  allOf: $("schema.json.allOf", "An array of schemas, all of which must match."),
  anyOf: $("schema.json.anyOf", "An array of schemas, where at least one must match."),
  oneOf: $("schema.json.oneOf", "An array of schemas, exactly one of which must match."),
  not: $("schema.json.not", "A schema which must not match."),
  $id: $("schema.json.$id", "A unique identifier for the schema."),
  $ref: $("schema.json.$ref", "Reference a definition hosted on any location."),
  $comment: $("schema.json.$comment", "Comments from schema authors to readers or maintainers of the schema."),
  readOnly: $("schema.json.readOnly", "Indicates that the value of the instance is managed exclusively by the owning authority."),
  examples: $("schema.json.examples", "Sample JSON values associated with a particular schema, for the purpose of illustrating usage."),
  contains: $("schema.json.contains", 'An array instance is valid against "contains" if at least one of its elements is valid against the given schema.'),
  propertyNames: $("schema.json.propertyNames", "If the instance is an object, this keyword validates if every property name in the instance validates against the provided schema."),
  const: $("schema.json.const", "An instance validates successfully against this keyword if its value is equal to the value of the keyword."),
  contentMediaType: $("schema.json.contentMediaType", "Describes the media type of a string property."),
  contentEncoding: $("schema.json.contentEncoding", "Describes the content encoding of a string property."),
  if: $("schema.json.if", 'The validation outcome of the "if" subschema controls which of the "then" or "else" keywords are evaluated.'),
  then: $("schema.json.then", 'The "if" subschema is used for validation when the "if" subschema succeeds.'),
  else: $("schema.json.else", 'The "else" subschema is used for validation when the "if" subschema fails.')
};
for ($s in Cr.schemas) {
  an = Cr.schemas[$s];
  for (pt in an.properties)
    on = an.properties[pt], typeof on == "boolean" && (on = an.properties[pt] = {}), zn = Nu[pt], zn ? on.description = zn : console.log("".concat(pt, ": localize('schema.json.").concat(pt, `', "")`));
}
var an, on, zn, pt, $s, ba;
ba = (() => {
  var e = { 470: (r) => {
    function i(o) {
      if (typeof o != "string")
        throw new TypeError("Path must be a string. Received " + JSON.stringify(o));
    }
    function s(o, l) {
      for (var u, c = "", f = 0, d = -1, g = 0, m = 0; m <= o.length; ++m) {
        if (m < o.length)
          u = o.charCodeAt(m);
        else {
          if (u === 47)
            break;
          u = 47;
        }
        if (u === 47) {
          if (!(d === m - 1 || g === 1))
            if (d !== m - 1 && g === 2) {
              if (c.length < 2 || f !== 2 || c.charCodeAt(c.length - 1) !== 46 || c.charCodeAt(c.length - 2) !== 46) {
                if (c.length > 2) {
                  var p = c.lastIndexOf("/");
                  if (p !== c.length - 1) {
                    p === -1 ? (c = "", f = 0) : f = (c = c.slice(0, p)).length - 1 - c.lastIndexOf("/"), d = m, g = 0;
                    continue;
                  }
                } else if (c.length === 2 || c.length === 1) {
                  c = "", f = 0, d = m, g = 0;
                  continue;
                }
              }
              l && (c.length > 0 ? c += "/.." : c = "..", f = 2);
            } else
              c.length > 0 ? c += "/" + o.slice(d + 1, m) : c = o.slice(d + 1, m), f = m - d - 1;
          d = m, g = 0;
        } else
          u === 46 && g !== -1 ? ++g : g = -1;
      }
      return c;
    }
    var a = { resolve: function() {
      for (var o, l = "", u = !1, c = arguments.length - 1; c >= -1 && !u; c--) {
        var f;
        c >= 0 ? f = arguments[c] : (o === void 0 && (o = process.cwd()), f = o), i(f), f.length !== 0 && (l = f + "/" + l, u = f.charCodeAt(0) === 47);
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
      for (var c = o.length, f = c - u, d = 1; d < l.length && l.charCodeAt(d) === 47; ++d)
        ;
      for (var g = l.length - d, m = f < g ? f : g, p = -1, v = 0; v <= m; ++v) {
        if (v === m) {
          if (g > m) {
            if (l.charCodeAt(d + v) === 47)
              return l.slice(d + v + 1);
            if (v === 0)
              return l.slice(d + v);
          } else
            f > m && (o.charCodeAt(u + v) === 47 ? p = v : v === 0 && (p = 0));
          break;
        }
        var x = o.charCodeAt(u + v);
        if (x !== l.charCodeAt(d + v))
          break;
        x === 47 && (p = v);
      }
      var b = "";
      for (v = u + p + 1; v <= c; ++v)
        v !== c && o.charCodeAt(v) !== 47 || (b.length === 0 ? b += ".." : b += "/..");
      return b.length > 0 ? b + l.slice(d + p) : (d += p, l.charCodeAt(d) === 47 && ++d, l.slice(d));
    }, _makeLong: function(o) {
      return o;
    }, dirname: function(o) {
      if (i(o), o.length === 0)
        return ".";
      for (var l = o.charCodeAt(0), u = l === 47, c = -1, f = !0, d = o.length - 1; d >= 1; --d)
        if ((l = o.charCodeAt(d)) === 47) {
          if (!f) {
            c = d;
            break;
          }
        } else
          f = !1;
      return c === -1 ? u ? "/" : "." : u && c === 1 ? "//" : o.slice(0, c);
    }, basename: function(o, l) {
      if (l !== void 0 && typeof l != "string")
        throw new TypeError('"ext" argument must be a string');
      i(o);
      var u, c = 0, f = -1, d = !0;
      if (l !== void 0 && l.length > 0 && l.length <= o.length) {
        if (l.length === o.length && l === o)
          return "";
        var g = l.length - 1, m = -1;
        for (u = o.length - 1; u >= 0; --u) {
          var p = o.charCodeAt(u);
          if (p === 47) {
            if (!d) {
              c = u + 1;
              break;
            }
          } else
            m === -1 && (d = !1, m = u + 1), g >= 0 && (p === l.charCodeAt(g) ? --g == -1 && (f = u) : (g = -1, f = m));
        }
        return c === f ? f = m : f === -1 && (f = o.length), o.slice(c, f);
      }
      for (u = o.length - 1; u >= 0; --u)
        if (o.charCodeAt(u) === 47) {
          if (!d) {
            c = u + 1;
            break;
          }
        } else
          f === -1 && (d = !1, f = u + 1);
      return f === -1 ? "" : o.slice(c, f);
    }, extname: function(o) {
      i(o);
      for (var l = -1, u = 0, c = -1, f = !0, d = 0, g = o.length - 1; g >= 0; --g) {
        var m = o.charCodeAt(g);
        if (m !== 47)
          c === -1 && (f = !1, c = g + 1), m === 46 ? l === -1 ? l = g : d !== 1 && (d = 1) : l !== -1 && (d = -1);
        else if (!f) {
          u = g + 1;
          break;
        }
      }
      return l === -1 || c === -1 || d === 0 || d === 1 && l === c - 1 && l === u + 1 ? "" : o.slice(l, c);
    }, format: function(o) {
      if (o === null || typeof o != "object")
        throw new TypeError('The "pathObject" argument must be of type Object. Received type ' + typeof o);
      return function(l, u) {
        var c = u.dir || u.root, f = u.base || (u.name || "") + (u.ext || "");
        return c ? c === u.root ? c + f : c + "/" + f : f;
      }(0, o);
    }, parse: function(o) {
      i(o);
      var l = { root: "", dir: "", base: "", ext: "", name: "" };
      if (o.length === 0)
        return l;
      var u, c = o.charCodeAt(0), f = c === 47;
      f ? (l.root = "/", u = 1) : u = 0;
      for (var d = -1, g = 0, m = -1, p = !0, v = o.length - 1, x = 0; v >= u; --v)
        if ((c = o.charCodeAt(v)) !== 47)
          m === -1 && (p = !1, m = v + 1), c === 46 ? d === -1 ? d = v : x !== 1 && (x = 1) : d !== -1 && (x = -1);
        else if (!p) {
          g = v + 1;
          break;
        }
      return d === -1 || m === -1 || x === 0 || x === 1 && d === m - 1 && d === g + 1 ? m !== -1 && (l.base = l.name = g === 0 && f ? o.slice(1, m) : o.slice(g, m)) : (g === 0 && f ? (l.name = o.slice(1, d), l.base = o.slice(1, m)) : (l.name = o.slice(g, d), l.base = o.slice(g, m)), l.ext = o.slice(d, m)), g > 0 ? l.dir = o.slice(0, g - 1) : f && (l.dir = "/"), l;
    }, sep: "/", delimiter: ":", win32: null, posix: null };
    a.posix = a, r.exports = a;
  }, 447: (r, i, s) => {
    var a;
    if (s.r(i), s.d(i, { URI: () => b, Utils: () => E }), typeof process == "object")
      a = process.platform === "win32";
    else if (typeof navigator == "object") {
      var o = navigator.userAgent;
      a = o.indexOf("Windows") >= 0;
    }
    var l, u, c = (l = function(k, A) {
      return (l = Object.setPrototypeOf || { __proto__: [] } instanceof Array && function(T, F) {
        T.__proto__ = F;
      } || function(T, F) {
        for (var B in F)
          Object.prototype.hasOwnProperty.call(F, B) && (T[B] = F[B]);
      })(k, A);
    }, function(k, A) {
      if (typeof A != "function" && A !== null)
        throw new TypeError("Class extends value " + String(A) + " is not a constructor or null");
      function T() {
        this.constructor = k;
      }
      l(k, A), k.prototype = A === null ? Object.create(A) : (T.prototype = A.prototype, new T());
    }), f = /^\w[\w\d+.-]*$/, d = /^\//, g = /^\/\//;
    function m(k, A) {
      if (!k.scheme && A)
        throw new Error('[UriError]: Scheme is missing: {scheme: "", authority: "'.concat(k.authority, '", path: "').concat(k.path, '", query: "').concat(k.query, '", fragment: "').concat(k.fragment, '"}'));
      if (k.scheme && !f.test(k.scheme))
        throw new Error("[UriError]: Scheme contains illegal characters.");
      if (k.path) {
        if (k.authority) {
          if (!d.test(k.path))
            throw new Error('[UriError]: If a URI contains an authority component, then the path component must either be empty or begin with a slash ("/") character');
        } else if (g.test(k.path))
          throw new Error('[UriError]: If a URI does not contain an authority component, then the path cannot begin with two slash characters ("//")');
      }
    }
    var p = "", v = "/", x = /^(([^:/?#]+?):)?(\/\/([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?/, b = function() {
      function k(A, T, F, B, U, W) {
        W === void 0 && (W = !1), typeof A == "object" ? (this.scheme = A.scheme || p, this.authority = A.authority || p, this.path = A.path || p, this.query = A.query || p, this.fragment = A.fragment || p) : (this.scheme = function(_e, ee) {
          return _e || ee ? _e : "file";
        }(A, W), this.authority = T || p, this.path = function(_e, ee) {
          switch (_e) {
            case "https":
            case "http":
            case "file":
              ee ? ee[0] !== v && (ee = v + ee) : ee = v;
          }
          return ee;
        }(this.scheme, F || p), this.query = B || p, this.fragment = U || p, m(this, W));
      }
      return k.isUri = function(A) {
        return A instanceof k || !!A && typeof A.authority == "string" && typeof A.fragment == "string" && typeof A.path == "string" && typeof A.query == "string" && typeof A.scheme == "string" && typeof A.fsPath == "string" && typeof A.with == "function" && typeof A.toString == "function";
      }, Object.defineProperty(k.prototype, "fsPath", { get: function() {
        return C(this, !1);
      }, enumerable: !1, configurable: !0 }), k.prototype.with = function(A) {
        if (!A)
          return this;
        var T = A.scheme, F = A.authority, B = A.path, U = A.query, W = A.fragment;
        return T === void 0 ? T = this.scheme : T === null && (T = p), F === void 0 ? F = this.authority : F === null && (F = p), B === void 0 ? B = this.path : B === null && (B = p), U === void 0 ? U = this.query : U === null && (U = p), W === void 0 ? W = this.fragment : W === null && (W = p), T === this.scheme && F === this.authority && B === this.path && U === this.query && W === this.fragment ? this : new _(T, F, B, U, W);
      }, k.parse = function(A, T) {
        T === void 0 && (T = !1);
        var F = x.exec(A);
        return F ? new _(F[2] || p, L(F[4] || p), L(F[5] || p), L(F[7] || p), L(F[9] || p), T) : new _(p, p, p, p, p);
      }, k.file = function(A) {
        var T = p;
        if (a && (A = A.replace(/\\/g, v)), A[0] === v && A[1] === v) {
          var F = A.indexOf(v, 2);
          F === -1 ? (T = A.substring(2), A = v) : (T = A.substring(2, F), A = A.substring(F) || v);
        }
        return new _("file", T, A, p, p);
      }, k.from = function(A) {
        var T = new _(A.scheme, A.authority, A.path, A.query, A.fragment);
        return m(T, !0), T;
      }, k.prototype.toString = function(A) {
        return A === void 0 && (A = !1), M(this, A);
      }, k.prototype.toJSON = function() {
        return this;
      }, k.revive = function(A) {
        if (A) {
          if (A instanceof k)
            return A;
          var T = new _(A);
          return T._formatted = A.external, T._fsPath = A._sep === y ? A.fsPath : null, T;
        }
        return A;
      }, k;
    }(), y = a ? 1 : void 0, _ = function(k) {
      function A() {
        var T = k !== null && k.apply(this, arguments) || this;
        return T._formatted = null, T._fsPath = null, T;
      }
      return c(A, k), Object.defineProperty(A.prototype, "fsPath", { get: function() {
        return this._fsPath || (this._fsPath = C(this, !1)), this._fsPath;
      }, enumerable: !1, configurable: !0 }), A.prototype.toString = function(T) {
        return T === void 0 && (T = !1), T ? M(this, !0) : (this._formatted || (this._formatted = M(this, !1)), this._formatted);
      }, A.prototype.toJSON = function() {
        var T = { $mid: 1 };
        return this._fsPath && (T.fsPath = this._fsPath, T._sep = y), this._formatted && (T.external = this._formatted), this.path && (T.path = this.path), this.scheme && (T.scheme = this.scheme), this.authority && (T.authority = this.authority), this.query && (T.query = this.query), this.fragment && (T.fragment = this.fragment), T;
      }, A;
    }(b), N = ((u = {})[58] = "%3A", u[47] = "%2F", u[63] = "%3F", u[35] = "%23", u[91] = "%5B", u[93] = "%5D", u[64] = "%40", u[33] = "%21", u[36] = "%24", u[38] = "%26", u[39] = "%27", u[40] = "%28", u[41] = "%29", u[42] = "%2A", u[43] = "%2B", u[44] = "%2C", u[59] = "%3B", u[61] = "%3D", u[32] = "%20", u);
    function w(k, A) {
      for (var T = void 0, F = -1, B = 0; B < k.length; B++) {
        var U = k.charCodeAt(B);
        if (U >= 97 && U <= 122 || U >= 65 && U <= 90 || U >= 48 && U <= 57 || U === 45 || U === 46 || U === 95 || U === 126 || A && U === 47)
          F !== -1 && (T += encodeURIComponent(k.substring(F, B)), F = -1), T !== void 0 && (T += k.charAt(B));
        else {
          T === void 0 && (T = k.substr(0, B));
          var W = N[U];
          W !== void 0 ? (F !== -1 && (T += encodeURIComponent(k.substring(F, B)), F = -1), T += W) : F === -1 && (F = B);
        }
      }
      return F !== -1 && (T += encodeURIComponent(k.substring(F))), T !== void 0 ? T : k;
    }
    function S(k) {
      for (var A = void 0, T = 0; T < k.length; T++) {
        var F = k.charCodeAt(T);
        F === 35 || F === 63 ? (A === void 0 && (A = k.substr(0, T)), A += N[F]) : A !== void 0 && (A += k[T]);
      }
      return A !== void 0 ? A : k;
    }
    function C(k, A) {
      var T;
      return T = k.authority && k.path.length > 1 && k.scheme === "file" ? "//".concat(k.authority).concat(k.path) : k.path.charCodeAt(0) === 47 && (k.path.charCodeAt(1) >= 65 && k.path.charCodeAt(1) <= 90 || k.path.charCodeAt(1) >= 97 && k.path.charCodeAt(1) <= 122) && k.path.charCodeAt(2) === 58 ? A ? k.path.substr(1) : k.path[1].toLowerCase() + k.path.substr(2) : k.path, a && (T = T.replace(/\//g, "\\")), T;
    }
    function M(k, A) {
      var T = A ? S : w, F = "", B = k.scheme, U = k.authority, W = k.path, _e = k.query, ee = k.fragment;
      if (B && (F += B, F += ":"), (U || B === "file") && (F += v, F += v), U) {
        var de = U.indexOf("@");
        if (de !== -1) {
          var We = U.substr(0, de);
          U = U.substr(de + 1), (de = We.indexOf(":")) === -1 ? F += T(We, !1) : (F += T(We.substr(0, de), !1), F += ":", F += T(We.substr(de + 1), !1)), F += "@";
        }
        (de = (U = U.toLowerCase()).indexOf(":")) === -1 ? F += T(U, !1) : (F += T(U.substr(0, de), !1), F += U.substr(de));
      }
      if (W) {
        if (W.length >= 3 && W.charCodeAt(0) === 47 && W.charCodeAt(2) === 58)
          (Re = W.charCodeAt(1)) >= 65 && Re <= 90 && (W = "/".concat(String.fromCharCode(Re + 32), ":").concat(W.substr(3)));
        else if (W.length >= 2 && W.charCodeAt(1) === 58) {
          var Re;
          (Re = W.charCodeAt(0)) >= 65 && Re <= 90 && (W = "".concat(String.fromCharCode(Re + 32), ":").concat(W.substr(2)));
        }
        F += T(W, !0);
      }
      return _e && (F += "?", F += T(_e, !1)), ee && (F += "#", F += A ? ee : w(ee, !1)), F;
    }
    function I(k) {
      try {
        return decodeURIComponent(k);
      } catch {
        return k.length > 3 ? k.substr(0, 3) + I(k.substr(3)) : k;
      }
    }
    var D = /(%[0-9A-Za-z][0-9A-Za-z])+/g;
    function L(k) {
      return k.match(D) ? k.replace(D, function(A) {
        return I(A);
      }) : k;
    }
    var E, P = s(470), O = function(k, A, T) {
      if (T || arguments.length === 2)
        for (var F, B = 0, U = A.length; B < U; B++)
          !F && B in A || (F || (F = Array.prototype.slice.call(A, 0, B)), F[B] = A[B]);
      return k.concat(F || Array.prototype.slice.call(A));
    }, V = P.posix || P;
    (function(k) {
      k.joinPath = function(A) {
        for (var T = [], F = 1; F < arguments.length; F++)
          T[F - 1] = arguments[F];
        return A.with({ path: V.join.apply(V, O([A.path], T, !1)) });
      }, k.resolvePath = function(A) {
        for (var T = [], F = 1; F < arguments.length; F++)
          T[F - 1] = arguments[F];
        var B = A.path || "/";
        return A.with({ path: V.resolve.apply(V, O([B], T, !1)) });
      }, k.dirname = function(A) {
        var T = V.dirname(A.path);
        return T.length === 1 && T.charCodeAt(0) === 46 ? A : A.with({ path: T });
      }, k.basename = function(A) {
        return V.basename(A.path);
      }, k.extname = function(A) {
        return V.extname(A.path);
      };
    })(E || (E = {}));
  } }, t = {};
  function n(r) {
    if (t[r])
      return t[r].exports;
    var i = t[r] = { exports: {} };
    return e[r](i, i.exports, n), i.exports;
  }
  return n.d = (r, i) => {
    for (var s in i)
      n.o(i, s) && !n.o(r, s) && Object.defineProperty(r, s, { enumerable: !0, get: i[s] });
  }, n.o = (r, i) => Object.prototype.hasOwnProperty.call(r, i), n.r = (r) => {
    typeof Symbol < "u" && Symbol.toStringTag && Object.defineProperty(r, Symbol.toStringTag, { value: "Module" }), Object.defineProperty(r, "__esModule", { value: !0 });
  }, n(447);
})();
var { URI: kt, Utils: Xu } = ba;
function Au(e, t) {
  if (typeof e != "string")
    throw new TypeError("Expected a string");
  for (var n = String(e), r = "", i = t ? !!t.extended : !1, s = t ? !!t.globstar : !1, a = !1, o = t && typeof t.flags == "string" ? t.flags : "", l, u = 0, c = n.length; u < c; u++)
    switch (l = n[u], l) {
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
        r += "\\" + l;
        break;
      case "?":
        if (i) {
          r += ".";
          break;
        }
      case "[":
      case "]":
        if (i) {
          r += l;
          break;
        }
      case "{":
        if (i) {
          a = !0, r += "(";
          break;
        }
      case "}":
        if (i) {
          a = !1, r += ")";
          break;
        }
      case ",":
        if (a) {
          r += "|";
          break;
        }
        r += "\\" + l;
        break;
      case "*":
        for (var f = n[u - 1], d = 1; n[u + 1] === "*"; )
          d++, u++;
        var g = n[u + 1];
        if (!s)
          r += ".*";
        else {
          var m = d > 1 && (f === "/" || f === void 0 || f === "{" || f === ",") && (g === "/" || g === void 0 || g === "," || g === "}");
          m ? (g === "/" ? u++ : f === "/" && r.endsWith("\\/") && (r = r.substr(0, r.length - 2)), r += "((?:[^/]*(?:/|$))*)") : r += "([^/]*)";
        }
        break;
      default:
        r += l;
    }
  return (!o || !~o.indexOf("g")) && (r = "^" + r + "$"), new RegExp(r, o);
}
var Te = Kt(), Cu = "!", ku = "/", Eu = function() {
  function e(t, n) {
    this.globWrappers = [];
    try {
      for (var r = 0, i = t; r < i.length; r++) {
        var s = i[r], a = s[0] !== Cu;
        a || (s = s.substring(1)), s.length > 0 && (s[0] === ku && (s = s.substring(1)), this.globWrappers.push({
          regexp: Au("**/" + s, { extended: !0, globstar: !0 }),
          include: a
        }));
      }
      this.uris = n;
    } catch {
      this.globWrappers.length = 0, this.uris = [];
    }
  }
  return e.prototype.matchesPattern = function(t) {
    for (var n = !1, r = 0, i = this.globWrappers; r < i.length; r++) {
      var s = i[r], a = s.regexp, o = s.include;
      a.test(t) && (n = o);
    }
    return n;
  }, e.prototype.getURIs = function() {
    return this.uris;
  }, e;
}(), Mu = function() {
  function e(t, n, r) {
    this.service = t, this.uri = n, this.dependencies = /* @__PURE__ */ new Set(), this.anchors = void 0, r && (this.unresolvedSchema = this.service.promise.resolve(new Ft(r)));
  }
  return e.prototype.getUnresolvedSchema = function() {
    return this.unresolvedSchema || (this.unresolvedSchema = this.service.loadSchema(this.uri)), this.unresolvedSchema;
  }, e.prototype.getResolvedSchema = function() {
    var t = this;
    return this.resolvedSchema || (this.resolvedSchema = this.getUnresolvedSchema().then(function(n) {
      return t.service.resolveSchemaContent(n, t);
    })), this.resolvedSchema;
  }, e.prototype.clearSchema = function() {
    var t = !!this.unresolvedSchema;
    return this.resolvedSchema = void 0, this.unresolvedSchema = void 0, this.dependencies.clear(), this.anchors = void 0, t;
  }, e;
}(), Ft = function() {
  function e(t, n) {
    n === void 0 && (n = []), this.schema = t, this.errors = n;
  }
  return e;
}(), Ws = function() {
  function e(t, n) {
    n === void 0 && (n = []), this.schema = t, this.errors = n;
  }
  return e.prototype.getSection = function(t) {
    var n = this.getSectionRecursive(t, this.schema);
    if (n)
      return me(n);
  }, e.prototype.getSectionRecursive = function(t, n) {
    if (!n || typeof n == "boolean" || t.length === 0)
      return n;
    var r = t.shift();
    if (n.properties && typeof n.properties[r])
      return this.getSectionRecursive(t, n.properties[r]);
    if (n.patternProperties)
      for (var i = 0, s = Object.keys(n.patternProperties); i < s.length; i++) {
        var a = s[i], o = Sn(a);
        if (o != null && o.test(r))
          return this.getSectionRecursive(t, n.patternProperties[a]);
      }
    else {
      if (typeof n.additionalProperties == "object")
        return this.getSectionRecursive(t, n.additionalProperties);
      if (r.match("[0-9]+")) {
        if (Array.isArray(n.items)) {
          var l = parseInt(r, 10);
          if (!isNaN(l) && n.items[l])
            return this.getSectionRecursive(t, n.items[l]);
        } else if (n.items)
          return this.getSectionRecursive(t, n.items);
      }
    }
  }, e;
}(), Ru = function() {
  function e(t, n, r) {
    this.contextService = n, this.requestService = t, this.promiseConstructor = r || Promise, this.callOnDispose = [], this.contributionSchemas = {}, this.contributionAssociations = [], this.schemasById = {}, this.filePatternAssociations = [], this.registeredSchemasIds = {};
  }
  return e.prototype.getRegisteredSchemaIds = function(t) {
    return Object.keys(this.registeredSchemasIds).filter(function(n) {
      var r = kt.parse(n).scheme;
      return r !== "schemaservice" && (!t || t(r));
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
    var n = this;
    this.cachedSchemaForResource = void 0;
    var r = !1;
    t = Xe(t);
    for (var i = [t], s = Object.keys(this.schemasById).map(function(u) {
      return n.schemasById[u];
    }); i.length; )
      for (var a = i.pop(), o = 0; o < s.length; o++) {
        var l = s[o];
        l && (l.uri === a || l.dependencies.has(a)) && (l.uri !== a && i.push(l.uri), l.clearSchema() && (r = !0), s[o] = void 0);
      }
    return r;
  }, e.prototype.setSchemaContributions = function(t) {
    if (t.schemas) {
      var n = t.schemas;
      for (var r in n) {
        var i = Xe(r);
        this.contributionSchemas[i] = this.addSchemaHandle(i, n[r]);
      }
    }
    if (Array.isArray(t.schemaAssociations))
      for (var s = t.schemaAssociations, a = 0, o = s; a < o.length; a++) {
        var l = o[a], u = l.uris.map(Xe), c = this.addFilePatternAssociation(l.pattern, u);
        this.contributionAssociations.push(c);
      }
  }, e.prototype.addSchemaHandle = function(t, n) {
    var r = new Mu(this, t, n);
    return this.schemasById[t] = r, r;
  }, e.prototype.getOrAddSchemaHandle = function(t, n) {
    return this.schemasById[t] || this.addSchemaHandle(t, n);
  }, e.prototype.addFilePatternAssociation = function(t, n) {
    var r = new Eu(t, n);
    return this.filePatternAssociations.push(r), r;
  }, e.prototype.registerExternalSchema = function(t, n, r) {
    var i = Xe(t);
    return this.registeredSchemasIds[i] = !0, this.cachedSchemaForResource = void 0, n && this.addFilePatternAssociation(n, [i]), r ? this.addSchemaHandle(i, r) : this.getOrAddSchemaHandle(i);
  }, e.prototype.clearExternalSchemas = function() {
    this.schemasById = {}, this.filePatternAssociations = [], this.registeredSchemasIds = {}, this.cachedSchemaForResource = void 0;
    for (var t in this.contributionSchemas)
      this.schemasById[t] = this.contributionSchemas[t], this.registeredSchemasIds[t] = !0;
    for (var n = 0, r = this.contributionAssociations; n < r.length; n++) {
      var i = r[n];
      this.filePatternAssociations.push(i);
    }
  }, e.prototype.getResolvedSchema = function(t) {
    var n = Xe(t), r = this.schemasById[n];
    return r ? r.getResolvedSchema() : this.promise.resolve(void 0);
  }, e.prototype.loadSchema = function(t) {
    if (!this.requestService) {
      var n = Te("json.schema.norequestservice", "Unable to load schema from '{0}'. No schema request service available", ln(t));
      return this.promise.resolve(new Ft({}, [n]));
    }
    return this.requestService(t).then(function(r) {
      if (!r) {
        var i = Te("json.schema.nocontent", "Unable to load schema from '{0}': No content.", ln(t));
        return new Ft({}, [i]);
      }
      var s = {}, a = [];
      s = Jl(r, a);
      var o = a.length ? [Te("json.schema.invalidFormat", "Unable to parse content from '{0}': Parse error at offset {1}.", ln(t), a[0].offset)] : [];
      return new Ft(s, o);
    }, function(r) {
      var i = r.toString(), s = r.toString().split("Error: ");
      return s.length > 1 && (i = s[1]), Ht(i, ".") && (i = i.substr(0, i.length - 1)), new Ft({}, [Te("json.schema.nocontent", "Unable to load schema from '{0}': {1}.", ln(t), i)]);
    });
  }, e.prototype.resolveSchemaContent = function(t, n) {
    var r = this, i = t.errors.slice(0), s = t.schema;
    if (s.$schema) {
      var a = Xe(s.$schema);
      if (a === "http://json-schema.org/draft-03/schema")
        return this.promise.resolve(new Ws({}, [Te("json.schema.draft03.notsupported", "Draft-03 schemas are not supported.")]));
      a === "https://json-schema.org/draft/2019-09/schema" ? i.push(Te("json.schema.draft201909.notsupported", "Draft 2019-09 schemas are not yet fully supported.")) : a === "https://json-schema.org/draft/2020-12/schema" && i.push(Te("json.schema.draft202012.notsupported", "Draft 2020-12 schemas are not yet fully supported."));
    }
    var o = this.contextService, l = function(p, v) {
      v = decodeURIComponent(v);
      var x = p;
      return v[0] === "/" && (v = v.substring(1)), v.split("/").some(function(b) {
        return b = b.replace(/~1/g, "/").replace(/~0/g, "~"), x = x[b], !x;
      }), x;
    }, u = function(p, v, x) {
      return v.anchors || (v.anchors = m(p)), v.anchors.get(x);
    }, c = function(p, v) {
      for (var x in v)
        v.hasOwnProperty(x) && !p.hasOwnProperty(x) && x !== "id" && x !== "$id" && (p[x] = v[x]);
    }, f = function(p, v, x, b) {
      var y;
      b === void 0 || b.length === 0 ? y = v : b.charAt(0) === "/" ? y = l(v, b) : y = u(v, x, b), y ? c(p, y) : i.push(Te("json.schema.invalidid", "$ref '{0}' in '{1}' can not be resolved.", b, x.uri));
    }, d = function(p, v, x, b) {
      o && !/^[A-Za-z][A-Za-z0-9+\-.+]*:\/\/.*/.test(v) && (v = o.resolveRelativePath(v, b.uri)), v = Xe(v);
      var y = r.getOrAddSchemaHandle(v);
      return y.getUnresolvedSchema().then(function(_) {
        if (b.dependencies.add(v), _.errors.length) {
          var N = x ? v + "#" + x : v;
          i.push(Te("json.schema.problemloadingref", "Problems loading reference '{0}': {1}", N, _.errors[0]));
        }
        return f(p, _.schema, y, x), g(p, _.schema, y);
      });
    }, g = function(p, v, x) {
      var b = [];
      return r.traverseNodes(p, function(y) {
        for (var _ = /* @__PURE__ */ new Set(); y.$ref; ) {
          var N = y.$ref, w = N.split("#", 2);
          if (delete y.$ref, w[0].length > 0) {
            b.push(d(y, w[0], w[1], x));
            return;
          } else if (!_.has(N)) {
            var S = w[1];
            f(y, v, x, S), _.add(N);
          }
        }
      }), r.promise.all(b);
    }, m = function(p) {
      var v = /* @__PURE__ */ new Map();
      return r.traverseNodes(p, function(x) {
        var b = x.$id || x.id;
        if (typeof b == "string" && b.charAt(0) === "#") {
          var y = b.substring(1);
          v.has(y) ? i.push(Te("json.schema.duplicateid", "Duplicate id declaration: '{0}'", b)) : v.set(y, x);
        }
      }), v;
    };
    return g(s, s, n).then(function(p) {
      return new Ws(s, i);
    });
  }, e.prototype.traverseNodes = function(t, n) {
    if (!t || typeof t != "object")
      return Promise.resolve(null);
    for (var r = /* @__PURE__ */ new Set(), i = function() {
      for (var u = [], c = 0; c < arguments.length; c++)
        u[c] = arguments[c];
      for (var f = 0, d = u; f < d.length; f++) {
        var g = d[f];
        typeof g == "object" && o.push(g);
      }
    }, s = function() {
      for (var u = [], c = 0; c < arguments.length; c++)
        u[c] = arguments[c];
      for (var f = 0, d = u; f < d.length; f++) {
        var g = d[f];
        if (typeof g == "object")
          for (var m in g) {
            var p = m, v = g[p];
            typeof v == "object" && o.push(v);
          }
      }
    }, a = function() {
      for (var u = [], c = 0; c < arguments.length; c++)
        u[c] = arguments[c];
      for (var f = 0, d = u; f < d.length; f++) {
        var g = d[f];
        if (Array.isArray(g))
          for (var m = 0, p = g; m < p.length; m++) {
            var v = p[m];
            typeof v == "object" && o.push(v);
          }
      }
    }, o = [t], l = o.pop(); l; )
      r.has(l) || (r.add(l), n(l), i(l.items, l.additionalItems, l.additionalProperties, l.not, l.contains, l.propertyNames, l.if, l.then, l.else), s(l.definitions, l.properties, l.patternProperties, l.dependencies), a(l.anyOf, l.allOf, l.oneOf, l.items)), l = o.pop();
  }, e.prototype.getSchemaFromProperty = function(t, n) {
    var r, i;
    if (((r = n.root) === null || r === void 0 ? void 0 : r.type) === "object")
      for (var s = 0, a = n.root.properties; s < a.length; s++) {
        var o = a[s];
        if (o.keyNode.value === "$schema" && ((i = o.valueNode) === null || i === void 0 ? void 0 : i.type) === "string") {
          var l = o.valueNode.value;
          return this.contextService && !/^\w[\w\d+.-]*:/.test(l) && (l = this.contextService.resolveRelativePath(l, t)), l;
        }
      }
  }, e.prototype.getAssociatedSchemas = function(t) {
    for (var n = /* @__PURE__ */ Object.create(null), r = [], i = Pu(t), s = 0, a = this.filePatternAssociations; s < a.length; s++) {
      var o = a[s];
      if (o.matchesPattern(i))
        for (var l = 0, u = o.getURIs(); l < u.length; l++) {
          var c = u[l];
          n[c] || (r.push(c), n[c] = !0);
        }
    }
    return r;
  }, e.prototype.getSchemaURIsForResource = function(t, n) {
    var r = n && this.getSchemaFromProperty(t, n);
    return r ? [r] : this.getAssociatedSchemas(t);
  }, e.prototype.getSchemaForResource = function(t, n) {
    if (n) {
      var r = this.getSchemaFromProperty(t, n);
      if (r) {
        var i = Xe(r);
        return this.getOrAddSchemaHandle(i).getResolvedSchema();
      }
    }
    if (this.cachedSchemaForResource && this.cachedSchemaForResource.resource === t)
      return this.cachedSchemaForResource.resolvedSchema;
    var s = this.getAssociatedSchemas(t), a = s.length > 0 ? this.createCombinedSchema(t, s).getResolvedSchema() : this.promise.resolve(void 0);
    return this.cachedSchemaForResource = { resource: t, resolvedSchema: a }, a;
  }, e.prototype.createCombinedSchema = function(t, n) {
    if (n.length === 1)
      return this.getOrAddSchemaHandle(n[0]);
    var r = "schemaservice://combinedSchema/" + encodeURIComponent(t), i = {
      allOf: n.map(function(s) {
        return { $ref: s };
      })
    };
    return this.addSchemaHandle(r, i);
  }, e.prototype.getMatchingSchemas = function(t, n, r) {
    if (r) {
      var i = r.id || "schemaservice://untitled/matchingSchemas/" + Tu++, s = this.addSchemaHandle(i, r);
      return s.getResolvedSchema().then(function(a) {
        return n.getMatchingSchemas(a.schema).filter(function(o) {
          return !o.inverted;
        });
      });
    }
    return this.getSchemaForResource(t.uri, n).then(function(a) {
      return a ? n.getMatchingSchemas(a.schema).filter(function(o) {
        return !o.inverted;
      }) : [];
    });
  }, e;
}(), Tu = 0;
function Xe(e) {
  try {
    return kt.parse(e).toString(!0);
  } catch {
    return e;
  }
}
function Pu(e) {
  try {
    return kt.parse(e).with({ fragment: null, query: null }).toString(!0);
  } catch {
    return e;
  }
}
function ln(e) {
  try {
    var t = kt.parse(e);
    if (t.scheme === "file")
      return t.fsPath;
  } catch {
  }
  return e;
}
function Fu(e, t) {
  var n = [], r = [], i = [], s = -1, a = wt(e.getText(), !1), o = a.scan();
  function l(I) {
    n.push(I), r.push(i.length);
  }
  for (; o !== 17; ) {
    switch (o) {
      case 1:
      case 3: {
        var u = e.positionAt(a.getTokenOffset()).line, c = { startLine: u, endLine: u, kind: o === 1 ? "object" : "array" };
        i.push(c);
        break;
      }
      case 2:
      case 4: {
        var f = o === 2 ? "object" : "array";
        if (i.length > 0 && i[i.length - 1].kind === f) {
          var c = i.pop(), d = e.positionAt(a.getTokenOffset()).line;
          c && d > c.startLine + 1 && s !== c.startLine && (c.endLine = d - 1, l(c), s = c.startLine);
        }
        break;
      }
      case 13: {
        var u = e.positionAt(a.getTokenOffset()).line, g = e.positionAt(a.getTokenOffset() + a.getTokenLength()).line;
        a.getTokenError() === 1 && u + 1 < e.lineCount ? a.setPosition(e.offsetAt(ke.create(u + 1, 0))) : u < g && (l({ startLine: u, endLine: g, kind: Vt.Comment }), s = u);
        break;
      }
      case 12: {
        var m = e.getText().substr(a.getTokenOffset(), a.getTokenLength()), p = m.match(/^\/\/\s*#(region\b)|(endregion\b)/);
        if (p) {
          var d = e.positionAt(a.getTokenOffset()).line;
          if (p[1]) {
            var c = { startLine: d, endLine: d, kind: Vt.Region };
            i.push(c);
          } else {
            for (var v = i.length - 1; v >= 0 && i[v].kind !== Vt.Region; )
              v--;
            if (v >= 0) {
              var c = i[v];
              i.length = v, d > c.startLine && s !== c.startLine && (c.endLine = d, l(c), s = c.startLine);
            }
          }
        }
        break;
      }
    }
    o = a.scan();
  }
  var x = t && t.rangeLimit;
  if (typeof x != "number" || n.length <= x)
    return n;
  t && t.onRangeLimitExceeded && t.onRangeLimitExceeded(e.uri);
  for (var b = [], y = 0, _ = r; y < _.length; y++) {
    var N = _[y];
    N < 30 && (b[N] = (b[N] || 0) + 1);
  }
  for (var w = 0, S = 0, v = 0; v < b.length; v++) {
    var C = b[v];
    if (C) {
      if (C + w > x) {
        S = v;
        break;
      }
      w += C;
    }
  }
  for (var M = [], v = 0; v < n.length; v++) {
    var N = r[v];
    typeof N == "number" && (N < S || N === S && w++ < x) && M.push(n[v]);
  }
  return M;
}
function Iu(e, t, n) {
  function r(o) {
    for (var l = e.offsetAt(o), u = n.getNodeFromOffset(l, !0), c = []; u; ) {
      switch (u.type) {
        case "string":
        case "object":
        case "array":
          var f = u.offset + 1, d = u.offset + u.length - 1;
          f < d && l >= f && l <= d && c.push(i(f, d)), c.push(i(u.offset, u.offset + u.length));
          break;
        case "number":
        case "boolean":
        case "null":
        case "property":
          c.push(i(u.offset, u.offset + u.length));
          break;
      }
      if (u.type === "property" || u.parent && u.parent.type === "array") {
        var g = a(u.offset + u.length, 5);
        g !== -1 && c.push(i(u.offset, g));
      }
      u = u.parent;
    }
    for (var m = void 0, p = c.length - 1; p >= 0; p--)
      m = kn.create(c[p], m);
    return m || (m = kn.create(X.create(o, o))), m;
  }
  function i(o, l) {
    return X.create(e.positionAt(o), e.positionAt(l));
  }
  var s = wt(e.getText(), !0);
  function a(o, l) {
    s.setPosition(o);
    var u = s.scan();
    return u === l ? s.getTokenOffset() + s.getTokenLength() : -1;
  }
  return t.map(r);
}
function Du(e, t) {
  var n = [];
  return t.visit(function(r) {
    var i;
    if (r.type === "property" && r.keyNode.value === "$ref" && ((i = r.valueNode) === null || i === void 0 ? void 0 : i.type) === "string") {
      var s = r.valueNode.value, a = Ou(t, s);
      if (a) {
        var o = e.positionAt(a.offset);
        n.push({
          target: "".concat(e.uri, "#").concat(o.line + 1, ",").concat(o.character + 1),
          range: Vu(e, r.valueNode)
        });
      }
    }
    return !0;
  }), Promise.resolve(n);
}
function Vu(e, t) {
  return X.create(e.positionAt(t.offset + 1), e.positionAt(t.offset + t.length - 1));
}
function Ou(e, t) {
  var n = ju(t);
  return n ? kr(n, e.root) : null;
}
function kr(e, t) {
  if (!t)
    return null;
  if (e.length === 0)
    return t;
  var n = e.shift();
  if (t && t.type === "object") {
    var r = t.properties.find(function(a) {
      return a.keyNode.value === n;
    });
    return r ? kr(e, r.valueNode) : null;
  } else if (t && t.type === "array" && n.match(/^(0|[1-9][0-9]*)$/)) {
    var i = Number.parseInt(n), s = t.items[i];
    return s ? kr(e, s) : null;
  }
  return null;
}
function ju(e) {
  return e === "#" ? [] : e[0] !== "#" || e[1] !== "/" ? null : e.substring(2).split(/\//).map(qu);
}
function qu(e) {
  return e.replace(/~1/g, "/").replace(/~0/g, "~");
}
function Bu(e) {
  var t = e.promiseConstructor || Promise, n = new Ru(e.schemaRequestService, e.workspaceContext, t);
  n.setSchemaContributions(Cr);
  var r = new gu(n, e.contributions, t, e.clientCapabilities), i = new mu(n, e.contributions, t), s = new Lu(n), a = new bu(n, t);
  return {
    configure: function(o) {
      n.clearExternalSchemas(), o.schemas && o.schemas.forEach(function(l) {
        n.registerExternalSchema(l.uri, l.fileMatch, l.schema);
      }), a.configure(o);
    },
    resetSchema: function(o) {
      return n.onResourceChange(o);
    },
    doValidation: a.doValidation.bind(a),
    getLanguageStatus: a.getLanguageStatus.bind(a),
    parseJSONDocument: function(o) {
      return du(o, { collectComments: !0 });
    },
    newJSONDocument: function(o, l) {
      return hu(o, l);
    },
    getMatchingSchemas: n.getMatchingSchemas.bind(n),
    doResolve: r.doResolve.bind(r),
    doComplete: r.doComplete.bind(r),
    findDocumentSymbols: s.findDocumentSymbols.bind(s),
    findDocumentSymbols2: s.findDocumentSymbols2.bind(s),
    findDocumentColors: s.findDocumentColors.bind(s),
    getColorPresentations: s.getColorPresentations.bind(s),
    doHover: i.doHover.bind(i),
    getFoldingRanges: Fu,
    getSelectionRanges: Iu,
    findDefinition: function() {
      return Promise.resolve([]);
    },
    findLinks: Du,
    format: function(o, l, u) {
      var c = void 0;
      if (l) {
        var f = o.offsetAt(l.start), d = o.offsetAt(l.end) - f;
        c = { offset: f, length: d };
      }
      var g = { tabSize: u ? u.tabSize : 4, insertSpaces: (u == null ? void 0 : u.insertSpaces) === !0, insertFinalNewline: (u == null ? void 0 : u.insertFinalNewline) === !0, eol: `
` };
      return Yl(o.getText(), c, g).map(function(m) {
        return Me.replace(X.create(o.positionAt(m.offset), o.positionAt(m.offset + m.length)), m.content);
      });
    }
  };
}
var ya;
typeof fetch < "u" && (ya = function(e) {
  return fetch(e).then((t) => t.text());
});
var Uu = class {
  constructor(e, t) {
    Et(this, "_ctx");
    Et(this, "_languageService");
    Et(this, "_languageSettings");
    Et(this, "_languageId");
    this._ctx = e, this._languageSettings = t.languageSettings, this._languageId = t.languageId, this._languageService = Bu({
      workspaceContext: {
        resolveRelativePath: (n, r) => {
          const i = r.substr(0, r.lastIndexOf("/") + 1);
          return Hu(i, n);
        }
      },
      schemaRequestService: t.enableSchemaRequest ? ya : void 0
    }), this._languageService.configure(this._languageSettings);
  }
  async doValidation(e) {
    let t = this._getTextDocument(e);
    if (t) {
      let n = this._languageService.parseJSONDocument(t);
      return this._languageService.doValidation(t, n, this._languageSettings);
    }
    return Promise.resolve([]);
  }
  async doComplete(e, t) {
    let n = this._getTextDocument(e);
    if (!n)
      return null;
    let r = this._languageService.parseJSONDocument(n);
    return this._languageService.doComplete(n, t, r);
  }
  async doResolve(e) {
    return this._languageService.doResolve(e);
  }
  async doHover(e, t) {
    let n = this._getTextDocument(e);
    if (!n)
      return null;
    let r = this._languageService.parseJSONDocument(n);
    return this._languageService.doHover(n, t, r);
  }
  async format(e, t, n) {
    let r = this._getTextDocument(e);
    if (!r)
      return [];
    let i = this._languageService.format(r, t, n);
    return Promise.resolve(i);
  }
  async resetSchema(e) {
    return Promise.resolve(this._languageService.resetSchema(e));
  }
  async findDocumentSymbols(e) {
    let t = this._getTextDocument(e);
    if (!t)
      return [];
    let n = this._languageService.parseJSONDocument(t), r = this._languageService.findDocumentSymbols(t, n);
    return Promise.resolve(r);
  }
  async findDocumentColors(e) {
    let t = this._getTextDocument(e);
    if (!t)
      return [];
    let n = this._languageService.parseJSONDocument(t), r = this._languageService.findDocumentColors(t, n);
    return Promise.resolve(r);
  }
  async getColorPresentations(e, t, n) {
    let r = this._getTextDocument(e);
    if (!r)
      return [];
    let i = this._languageService.parseJSONDocument(r), s = this._languageService.getColorPresentations(r, i, t, n);
    return Promise.resolve(s);
  }
  async getFoldingRanges(e, t) {
    let n = this._getTextDocument(e);
    if (!n)
      return [];
    let r = this._languageService.getFoldingRanges(n, t);
    return Promise.resolve(r);
  }
  async getSelectionRanges(e, t) {
    let n = this._getTextDocument(e);
    if (!n)
      return [];
    let r = this._languageService.parseJSONDocument(n), i = this._languageService.getSelectionRanges(n, t, r);
    return Promise.resolve(i);
  }
  _getTextDocument(e) {
    let t = this._ctx.getMirrorModels();
    for (let n of t)
      if (n.uri.toString() === e)
        return _r.create(e, this._languageId, n.version, n.getValue());
    return null;
  }
}, $u = "/".charCodeAt(0), Gn = ".".charCodeAt(0);
function Wu(e) {
  return e.charCodeAt(0) === $u;
}
function Hu(e, t) {
  if (Wu(t)) {
    const n = kt.parse(e), r = t.split("/");
    return n.with({ path: xa(r) }).toString();
  }
  return zu(e, t);
}
function xa(e) {
  const t = [];
  for (const r of e)
    r.length === 0 || r.length === 1 && r.charCodeAt(0) === Gn || (r.length === 2 && r.charCodeAt(0) === Gn && r.charCodeAt(1) === Gn ? t.pop() : t.push(r));
  e.length > 1 && e[e.length - 1].length === 0 && t.push("");
  let n = t.join("/");
  return e[0].length === 0 && (n = "/" + n), n;
}
function zu(e, ...t) {
  const n = kt.parse(e), r = n.path.split("/");
  for (let i of t)
    r.push(...i.split("/"));
  return n.with({ path: xa(r) }).toString();
}
self.onmessage = () => {
  ca((e, t) => new Uu(e, t));
};
