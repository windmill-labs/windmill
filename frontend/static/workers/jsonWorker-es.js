var xa = Object.defineProperty;
var wa = (e, t, n) => t in e ? xa(e, t, { enumerable: !0, configurable: !0, writable: !0, value: n }) : e[t] = n;
var Nt = (e, t, n) => (wa(e, typeof t != "symbol" ? t + "" : t, n), n);
class _a {
  constructor() {
    this.listeners = [], this.unexpectedErrorHandler = function(t) {
      setTimeout(() => {
        throw t.stack ? wt.isErrorNoTelemetry(t) ? new wt(t.message + `

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
const Sa = new _a();
function Ws(e) {
  La(e) || Sa.onUnexpectedError(e);
}
function Vr(e) {
  if (e instanceof Error) {
    const { name: t, message: n } = e, r = e.stacktrace || e.stack;
    return {
      $isError: !0,
      name: t,
      message: n,
      stack: r,
      noTelemetry: wt.isErrorNoTelemetry(e)
    };
  }
  return e;
}
const $n = "Canceled";
function La(e) {
  return e instanceof Na ? !0 : e instanceof Error && e.name === $n && e.message === $n;
}
class Na extends Error {
  constructor() {
    super($n), this.name = this.message;
  }
}
class wt extends Error {
  constructor(t) {
    super(t), this.name = "CodeExpectedError";
  }
  static fromError(t) {
    if (t instanceof wt)
      return t;
    const n = new wt();
    return n.message = t.message, n.stack = t.stack, n;
  }
  static isErrorNoTelemetry(t) {
    return t.name === "CodeExpectedError";
  }
}
class je extends Error {
  constructor(t) {
    super(t || "An unexpected bug occurred."), Object.setPrototypeOf(this, je.prototype);
  }
}
function Aa(e) {
  const t = this;
  let n = !1, r;
  return function() {
    return n || (n = !0, r = e.apply(t, arguments)), r;
  };
}
var ln;
(function(e) {
  function t(y) {
    return y && typeof y == "object" && typeof y[Symbol.iterator] == "function";
  }
  e.is = t;
  const n = Object.freeze([]);
  function r() {
    return n;
  }
  e.empty = r;
  function* i(y) {
    yield y;
  }
  e.single = i;
  function s(y) {
    return t(y) ? y : i(y);
  }
  e.wrap = s;
  function a(y) {
    return y || n;
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
    for (const v of y)
      if (x(v))
        return !0;
    return !1;
  }
  e.some = u;
  function h(y, x) {
    for (const v of y)
      if (x(v))
        return v;
  }
  e.find = h;
  function* f(y, x) {
    for (const v of y)
      x(v) && (yield v);
  }
  e.filter = f;
  function* d(y, x) {
    let v = 0;
    for (const C of y)
      yield x(C, v++);
  }
  e.map = d;
  function* g(...y) {
    for (const x of y)
      for (const v of x)
        yield v;
  }
  e.concat = g;
  function m(y, x, v) {
    let C = v;
    for (const L of y)
      C = x(C, L);
    return C;
  }
  e.reduce = m;
  function* p(y, x, v = y.length) {
    for (x < 0 && (x += y.length), v < 0 ? v += y.length : v > y.length && (v = y.length); x < v; x++)
      yield y[x];
  }
  e.slice = p;
  function b(y, x = Number.POSITIVE_INFINITY) {
    const v = [];
    if (x === 0)
      return [v, y];
    const C = y[Symbol.iterator]();
    for (let L = 0; L < x; L++) {
      const _ = C.next();
      if (_.done)
        return [v, e.empty()];
      v.push(_.value);
    }
    return [v, { [Symbol.iterator]() {
      return C;
    } }];
  }
  e.consume = b;
})(ln || (ln = {}));
function Hs(e) {
  if (ln.is(e)) {
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
function Ca(...e) {
  return Ft(() => Hs(e));
}
function Ft(e) {
  return {
    dispose: Aa(() => {
      e();
    })
  };
}
class it {
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
        Hs(this._toDispose);
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
    return this._isDisposed ? it.DISABLE_DISPOSED_WARNING || console.warn(new Error("Trying to add a disposable to a DisposableStore that has already been disposed of. The added object will be leaked!").stack) : this._toDispose.add(t), t;
  }
}
it.DISABLE_DISPOSED_WARNING = !1;
class It {
  constructor() {
    this._store = new it(), this._store;
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
It.None = Object.freeze({ dispose() {
} });
class Q {
  constructor(t) {
    this.element = t, this.next = Q.Undefined, this.prev = Q.Undefined;
  }
}
Q.Undefined = new Q(void 0);
class ka {
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
class Sn {
  static create(t) {
    return new Sn(t);
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
var Wn;
(function(e) {
  e.None = () => It.None;
  function t(k, w) {
    return h(k, () => {
    }, 0, void 0, !0, void 0, w);
  }
  e.defer = t;
  function n(k) {
    return (w, N = null, T) => {
      let V = !1, q;
      return q = k((B) => {
        if (!V)
          return q ? q.dispose() : V = !0, w.call(N, B);
      }, null, T), V && q.dispose(), q;
    };
  }
  e.once = n;
  function r(k, w, N) {
    return u((T, V = null, q) => k((B) => T.call(V, w(B)), null, q), N);
  }
  e.map = r;
  function i(k, w, N) {
    return u((T, V = null, q) => k((B) => {
      w(B), T.call(V, B);
    }, null, q), N);
  }
  e.forEach = i;
  function s(k, w, N) {
    return u((T, V = null, q) => k((B) => w(B) && T.call(V, B), null, q), N);
  }
  e.filter = s;
  function a(k) {
    return k;
  }
  e.signal = a;
  function o(...k) {
    return (w, N = null, T) => Ca(...k.map((V) => V((q) => w.call(N, q), null, T)));
  }
  e.any = o;
  function l(k, w, N, T) {
    let V = N;
    return r(k, (q) => (V = w(V, q), V), T);
  }
  e.reduce = l;
  function u(k, w) {
    let N;
    const T = {
      onWillAddFirstListener() {
        N = k(V.fire, V);
      },
      onDidRemoveLastListener() {
        N == null || N.dispose();
      }
    }, V = new Ae(T);
    return w == null || w.add(V), V.event;
  }
  function h(k, w, N = 100, T = !1, V = !1, q, B) {
    let P, M, F, I = 0, j;
    const U = {
      leakWarningThreshold: q,
      onWillAddFirstListener() {
        P = k((we) => {
          I++, M = w(M, we), T && !F && (H.fire(M), M = void 0), j = () => {
            const se = M;
            M = void 0, F = void 0, (!T || I > 1) && H.fire(se), I = 0;
          }, typeof N == "number" ? (clearTimeout(F), F = setTimeout(j, N)) : F === void 0 && (F = 0, queueMicrotask(j));
        });
      },
      onWillRemoveListener() {
        V && I > 0 && (j == null || j());
      },
      onDidRemoveLastListener() {
        j = void 0, P.dispose();
      }
    }, H = new Ae(U);
    return B == null || B.add(H), H.event;
  }
  e.debounce = h;
  function f(k, w = 0, N) {
    return e.debounce(k, (T, V) => T ? (T.push(V), T) : [V], w, void 0, !0, void 0, N);
  }
  e.accumulate = f;
  function d(k, w = (T, V) => T === V, N) {
    let T = !0, V;
    return s(k, (q) => {
      const B = T || !w(q, V);
      return T = !1, V = q, B;
    }, N);
  }
  e.latch = d;
  function g(k, w, N) {
    return [
      e.filter(k, w, N),
      e.filter(k, (T) => !w(T), N)
    ];
  }
  e.split = g;
  function m(k, w = !1, N = []) {
    let T = N.slice(), V = k((P) => {
      T ? T.push(P) : B.fire(P);
    });
    const q = () => {
      T == null || T.forEach((P) => B.fire(P)), T = null;
    }, B = new Ae({
      onWillAddFirstListener() {
        V || (V = k((P) => B.fire(P)));
      },
      onDidAddFirstListener() {
        T && (w ? setTimeout(q) : q());
      },
      onDidRemoveLastListener() {
        V && V.dispose(), V = null;
      }
    });
    return B.event;
  }
  e.buffer = m;
  class p {
    constructor(w) {
      this.event = w, this.disposables = new it();
    }
    /** @see {@link Event.map} */
    map(w) {
      return new p(r(this.event, w, this.disposables));
    }
    /** @see {@link Event.forEach} */
    forEach(w) {
      return new p(i(this.event, w, this.disposables));
    }
    filter(w) {
      return new p(s(this.event, w, this.disposables));
    }
    /** @see {@link Event.reduce} */
    reduce(w, N) {
      return new p(l(this.event, w, N, this.disposables));
    }
    /** @see {@link Event.reduce} */
    latch() {
      return new p(d(this.event, void 0, this.disposables));
    }
    debounce(w, N = 100, T = !1, V = !1, q) {
      return new p(h(this.event, w, N, T, V, q, this.disposables));
    }
    /**
     * Attach a listener to the event.
     */
    on(w, N, T) {
      return this.event(w, N, T);
    }
    /** @see {@link Event.once} */
    once(w, N, T) {
      return n(this.event)(w, N, T);
    }
    dispose() {
      this.disposables.dispose();
    }
  }
  function b(k) {
    return new p(k);
  }
  e.chain = b;
  function y(k, w, N = (T) => T) {
    const T = (...P) => B.fire(N(...P)), V = () => k.on(w, T), q = () => k.removeListener(w, T), B = new Ae({ onWillAddFirstListener: V, onDidRemoveLastListener: q });
    return B.event;
  }
  e.fromNodeEventEmitter = y;
  function x(k, w, N = (T) => T) {
    const T = (...P) => B.fire(N(...P)), V = () => k.addEventListener(w, T), q = () => k.removeEventListener(w, T), B = new Ae({ onWillAddFirstListener: V, onDidRemoveLastListener: q });
    return B.event;
  }
  e.fromDOMEventEmitter = x;
  function v(k) {
    return new Promise((w) => n(k)(w));
  }
  e.toPromise = v;
  function C(k) {
    const w = new Ae();
    return k.then((N) => {
      w.fire(N);
    }, () => {
      w.fire(void 0);
    }).finally(() => {
      w.dispose();
    }), w.event;
  }
  e.fromPromise = C;
  function L(k, w) {
    return w(void 0), k((N) => w(N));
  }
  e.runAndSubscribe = L;
  function _(k, w) {
    let N = null;
    function T(q) {
      N == null || N.dispose(), N = new it(), w(q, N);
    }
    T(void 0);
    const V = k((q) => T(q));
    return Ft(() => {
      V.dispose(), N == null || N.dispose();
    });
  }
  e.runAndSubscribeWithStore = _;
  class S {
    constructor(w, N) {
      this._observable = w, this._counter = 0, this._hasChanged = !1;
      const T = {
        onWillAddFirstListener: () => {
          w.addObserver(this);
        },
        onDidRemoveLastListener: () => {
          w.removeObserver(this);
        }
      };
      this.emitter = new Ae(T), N && N.add(this.emitter);
    }
    beginUpdate(w) {
      this._counter++;
    }
    handlePossibleChange(w) {
    }
    handleChange(w, N) {
      this._hasChanged = !0;
    }
    endUpdate(w) {
      this._counter--, this._counter === 0 && (this._observable.reportChanges(), this._hasChanged && (this._hasChanged = !1, this.emitter.fire(this._observable.get())));
    }
  }
  function A(k, w) {
    return new S(k, w).emitter.event;
  }
  e.fromObservable = A;
  function R(k) {
    return (w) => {
      let N = 0, T = !1;
      const V = {
        beginUpdate() {
          N++;
        },
        endUpdate() {
          N--, N === 0 && (k.reportChanges(), T && (T = !1, w()));
        },
        handlePossibleChange() {
        },
        handleChange() {
          T = !0;
        }
      };
      return k.addObserver(V), k.reportChanges(), {
        dispose() {
          k.removeObserver(V);
        }
      };
    };
  }
  e.fromObservableLight = R;
})(Wn || (Wn = {}));
class _t {
  constructor(t) {
    this.listenerCount = 0, this.invocationCount = 0, this.elapsedOverall = 0, this.durations = [], this.name = `${t}_${_t._idPool++}`, _t.all.add(this);
  }
  start(t) {
    this._stopWatch = new Sn(), this.listenerCount = t;
  }
  stop() {
    if (this._stopWatch) {
      const t = this._stopWatch.elapsed();
      this.durations.push(t), this.elapsedOverall += t, this.invocationCount += 1, this._stopWatch = void 0;
    }
  }
}
_t.all = /* @__PURE__ */ new Set();
_t._idPool = 0;
let Ea = -1;
class Ma {
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
class _r {
  static create() {
    var t;
    return new _r((t = new Error().stack) !== null && t !== void 0 ? t : "");
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
class Cn {
  constructor(t) {
    this.value = t;
  }
}
const Ta = 2;
class Ae {
  constructor(t) {
    var n, r, i, s, a;
    this._size = 0, this._options = t, this._leakageMon = !((n = this._options) === null || n === void 0) && n.leakWarningThreshold ? new Ma((i = (r = this._options) === null || r === void 0 ? void 0 : r.leakWarningThreshold) !== null && i !== void 0 ? i : Ea) : void 0, this._perfMon = !((s = this._options) === null || s === void 0) && s._profName ? new _t(this._options._profName) : void 0, this._deliveryQueue = (a = this._options) === null || a === void 0 ? void 0 : a.deliveryQueue;
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
        return console.warn(`[${this._leakageMon.name}] REFUSES to accept new listeners because it exceeded its threshold by far`), It.None;
      if (this._disposed)
        return It.None;
      r && (n = n.bind(r));
      const h = new Cn(n);
      let f;
      this._leakageMon && this._size >= Math.ceil(this._leakageMon.threshold * 0.2) && (h.stack = _r.create(), f = this._leakageMon.check(h.stack, this._size + 1)), this._listeners ? this._listeners instanceof Cn ? ((u = this._deliveryQueue) !== null && u !== void 0 || (this._deliveryQueue = new Pa()), this._listeners = [this._listeners, h]) : this._listeners.push(h) : ((a = (s = this._options) === null || s === void 0 ? void 0 : s.onWillAddFirstListener) === null || a === void 0 || a.call(s, this), this._listeners = h, (l = (o = this._options) === null || o === void 0 ? void 0 : o.onDidAddFirstListener) === null || l === void 0 || l.call(o, this)), this._size++;
      const d = Ft(() => {
        f == null || f(), this._removeListener(h);
      });
      return i instanceof it ? i.add(d) : Array.isArray(i) && i.push(d), d;
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
    if (this._size * Ta <= a.length) {
      let u = 0;
      for (let h = 0; h < a.length; h++)
        a[h] ? a[u++] = a[h] : l && (this._deliveryQueue.end--, u < this._deliveryQueue.i && this._deliveryQueue.i--);
      a.length = u;
    }
  }
  _deliver(t, n) {
    var r;
    if (!t)
      return;
    const i = ((r = this._options) === null || r === void 0 ? void 0 : r.onListenerError) || Ws;
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
      if (this._listeners instanceof Cn)
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
class Pa {
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
function Fa(e) {
  return typeof e == "string";
}
function Ia(e) {
  let t = [];
  for (; Object.prototype !== e; )
    t = t.concat(Object.getOwnPropertyNames(e)), e = Object.getPrototypeOf(e);
  return t;
}
function Hn(e) {
  const t = [];
  for (const n of Ia(e))
    typeof e[n] == "function" && t.push(n);
  return t;
}
function Va(e, t) {
  const n = (i) => function() {
    const s = Array.prototype.slice.call(arguments, 0);
    return t(i, s);
  }, r = {};
  for (const i of e)
    r[i] = n(i);
  return r;
}
globalThis && globalThis.__awaiter;
let Da = typeof document < "u" && document.location && document.location.hash.indexOf("pseudo=true") >= 0;
function Oa(e, t) {
  let n;
  return t.length === 0 ? n = e : n = e.replace(/\{(\d+)\}/g, (r, i) => {
    const s = i[0], a = t[s];
    let o = r;
    return typeof a == "string" ? o = a : (typeof a == "number" || typeof a == "boolean" || a === void 0 || a === null) && (o = String(a)), o;
  }), Da && (n = "［" + n.replace(/[aouei]/g, "$&$&") + "］"), n;
}
function Y(e, t, ...n) {
  return Oa(t, n);
}
var kn;
const mt = "en";
let zn = !1, Gn = !1, Rn = !1, zs = !1, Jt, En = mt, Dr = mt, ja, Ne;
const Re = typeof self == "object" ? self : typeof global == "object" ? global : {};
let le;
typeof Re.vscode < "u" && typeof Re.vscode.process < "u" ? le = Re.vscode.process : typeof process < "u" && (le = process);
const Ba = typeof ((kn = le == null ? void 0 : le.versions) === null || kn === void 0 ? void 0 : kn.electron) == "string", qa = Ba && (le == null ? void 0 : le.type) === "renderer";
if (typeof navigator == "object" && !qa)
  Ne = navigator.userAgent, zn = Ne.indexOf("Windows") >= 0, Gn = Ne.indexOf("Macintosh") >= 0, (Ne.indexOf("Macintosh") >= 0 || Ne.indexOf("iPad") >= 0 || Ne.indexOf("iPhone") >= 0) && navigator.maxTouchPoints && navigator.maxTouchPoints > 0, Rn = Ne.indexOf("Linux") >= 0, (Ne == null ? void 0 : Ne.indexOf("Mobi")) >= 0, zs = !0, // This call _must_ be done in the file that calls `nls.getConfiguredDefaultLocale`
  // to ensure that the NLS AMD Loader plugin has been loaded and configured.
  // This is because the loader plugin decides what the default locale is based on
  // how it's able to resolve the strings.
  Y({ key: "ensureLoaderPluginIsLoaded", comment: ["{Locked}"] }, "_"), Jt = mt, En = Jt, Dr = navigator.language;
else if (typeof le == "object") {
  zn = le.platform === "win32", Gn = le.platform === "darwin", Rn = le.platform === "linux", Rn && le.env.SNAP && le.env.SNAP_REVISION, le.env.CI || le.env.BUILD_ARTIFACTSTAGINGDIRECTORY, Jt = mt, En = mt;
  const e = le.env.VSCODE_NLS_CONFIG;
  if (e)
    try {
      const t = JSON.parse(e), n = t.availableLanguages["*"];
      Jt = t.locale, Dr = t.osLocale, En = n || mt, ja = t._translationsConfigFile;
    } catch {
    }
} else
  console.error("Unable to resolve platform.");
const Vt = zn, Ua = Gn;
zs && Re.importScripts;
const Ve = Ne, $a = typeof Re.postMessage == "function" && !Re.importScripts;
(() => {
  if ($a) {
    const e = [];
    Re.addEventListener("message", (n) => {
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
      }), Re.postMessage({ vscodeScheduleAsyncWork: r }, "*");
    };
  }
  return (e) => setTimeout(e);
})();
const Wa = !!(Ve && Ve.indexOf("Chrome") >= 0);
Ve && Ve.indexOf("Firefox") >= 0;
!Wa && Ve && Ve.indexOf("Safari") >= 0;
Ve && Ve.indexOf("Edg/") >= 0;
Ve && Ve.indexOf("Android") >= 0;
class Ha {
  constructor(t) {
    this.fn = t, this.lastCache = void 0, this.lastArgKey = void 0;
  }
  get(t) {
    const n = JSON.stringify(t);
    return this.lastArgKey !== n && (this.lastArgKey = n, this.lastCache = this.fn(t)), this.lastCache;
  }
}
class Gs {
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
var St;
function za(e) {
  return e.replace(/[\\\{\}\*\+\?\|\^\$\.\[\]\(\)]/g, "\\$&");
}
function Ga(e) {
  return e.split(/\r\n|\r|\n/);
}
function Ja(e) {
  for (let t = 0, n = e.length; t < n; t++) {
    const r = e.charCodeAt(t);
    if (r !== 32 && r !== 9)
      return t;
  }
  return -1;
}
function Xa(e, t = e.length - 1) {
  for (let n = t; n >= 0; n--) {
    const r = e.charCodeAt(n);
    if (r !== 32 && r !== 9)
      return n;
  }
  return -1;
}
function Js(e) {
  return e >= 65 && e <= 90;
}
function Jn(e) {
  return 55296 <= e && e <= 56319;
}
function Qa(e) {
  return 56320 <= e && e <= 57343;
}
function Za(e, t) {
  return (e - 55296 << 10) + (t - 56320) + 65536;
}
function Ya(e, t, n) {
  const r = e.charCodeAt(n);
  if (Jn(r) && n + 1 < t) {
    const i = e.charCodeAt(n + 1);
    if (Qa(i))
      return Za(r, i);
  }
  return r;
}
const Ka = /^[\t\n\r\x20-\x7E]*$/;
function eo(e) {
  return Ka.test(e);
}
class st {
  static getInstance(t) {
    return St.cache.get(Array.from(t));
  }
  static getLocales() {
    return St._locales.value;
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
St = st;
st.ambiguousCharacterData = new Gs(() => JSON.parse('{"_common":[8232,32,8233,32,5760,32,8192,32,8193,32,8194,32,8195,32,8196,32,8197,32,8198,32,8200,32,8201,32,8202,32,8287,32,8199,32,8239,32,2042,95,65101,95,65102,95,65103,95,8208,45,8209,45,8210,45,65112,45,1748,45,8259,45,727,45,8722,45,10134,45,11450,45,1549,44,1643,44,8218,44,184,44,42233,44,894,59,2307,58,2691,58,1417,58,1795,58,1796,58,5868,58,65072,58,6147,58,6153,58,8282,58,1475,58,760,58,42889,58,8758,58,720,58,42237,58,451,33,11601,33,660,63,577,63,2429,63,5038,63,42731,63,119149,46,8228,46,1793,46,1794,46,42510,46,68176,46,1632,46,1776,46,42232,46,1373,96,65287,96,8219,96,8242,96,1370,96,1523,96,8175,96,65344,96,900,96,8189,96,8125,96,8127,96,8190,96,697,96,884,96,712,96,714,96,715,96,756,96,699,96,701,96,700,96,702,96,42892,96,1497,96,2036,96,2037,96,5194,96,5836,96,94033,96,94034,96,65339,91,10088,40,10098,40,12308,40,64830,40,65341,93,10089,41,10099,41,12309,41,64831,41,10100,123,119060,123,10101,125,65342,94,8270,42,1645,42,8727,42,66335,42,5941,47,8257,47,8725,47,8260,47,9585,47,10187,47,10744,47,119354,47,12755,47,12339,47,11462,47,20031,47,12035,47,65340,92,65128,92,8726,92,10189,92,10741,92,10745,92,119311,92,119355,92,12756,92,20022,92,12034,92,42872,38,708,94,710,94,5869,43,10133,43,66203,43,8249,60,10094,60,706,60,119350,60,5176,60,5810,60,5120,61,11840,61,12448,61,42239,61,8250,62,10095,62,707,62,119351,62,5171,62,94015,62,8275,126,732,126,8128,126,8764,126,65372,124,65293,45,120784,50,120794,50,120804,50,120814,50,120824,50,130034,50,42842,50,423,50,1000,50,42564,50,5311,50,42735,50,119302,51,120785,51,120795,51,120805,51,120815,51,120825,51,130035,51,42923,51,540,51,439,51,42858,51,11468,51,1248,51,94011,51,71882,51,120786,52,120796,52,120806,52,120816,52,120826,52,130036,52,5070,52,71855,52,120787,53,120797,53,120807,53,120817,53,120827,53,130037,53,444,53,71867,53,120788,54,120798,54,120808,54,120818,54,120828,54,130038,54,11474,54,5102,54,71893,54,119314,55,120789,55,120799,55,120809,55,120819,55,120829,55,130039,55,66770,55,71878,55,2819,56,2538,56,2666,56,125131,56,120790,56,120800,56,120810,56,120820,56,120830,56,130040,56,547,56,546,56,66330,56,2663,57,2920,57,2541,57,3437,57,120791,57,120801,57,120811,57,120821,57,120831,57,130041,57,42862,57,11466,57,71884,57,71852,57,71894,57,9082,97,65345,97,119834,97,119886,97,119938,97,119990,97,120042,97,120094,97,120146,97,120198,97,120250,97,120302,97,120354,97,120406,97,120458,97,593,97,945,97,120514,97,120572,97,120630,97,120688,97,120746,97,65313,65,119808,65,119860,65,119912,65,119964,65,120016,65,120068,65,120120,65,120172,65,120224,65,120276,65,120328,65,120380,65,120432,65,913,65,120488,65,120546,65,120604,65,120662,65,120720,65,5034,65,5573,65,42222,65,94016,65,66208,65,119835,98,119887,98,119939,98,119991,98,120043,98,120095,98,120147,98,120199,98,120251,98,120303,98,120355,98,120407,98,120459,98,388,98,5071,98,5234,98,5551,98,65314,66,8492,66,119809,66,119861,66,119913,66,120017,66,120069,66,120121,66,120173,66,120225,66,120277,66,120329,66,120381,66,120433,66,42932,66,914,66,120489,66,120547,66,120605,66,120663,66,120721,66,5108,66,5623,66,42192,66,66178,66,66209,66,66305,66,65347,99,8573,99,119836,99,119888,99,119940,99,119992,99,120044,99,120096,99,120148,99,120200,99,120252,99,120304,99,120356,99,120408,99,120460,99,7428,99,1010,99,11429,99,43951,99,66621,99,128844,67,71922,67,71913,67,65315,67,8557,67,8450,67,8493,67,119810,67,119862,67,119914,67,119966,67,120018,67,120174,67,120226,67,120278,67,120330,67,120382,67,120434,67,1017,67,11428,67,5087,67,42202,67,66210,67,66306,67,66581,67,66844,67,8574,100,8518,100,119837,100,119889,100,119941,100,119993,100,120045,100,120097,100,120149,100,120201,100,120253,100,120305,100,120357,100,120409,100,120461,100,1281,100,5095,100,5231,100,42194,100,8558,68,8517,68,119811,68,119863,68,119915,68,119967,68,120019,68,120071,68,120123,68,120175,68,120227,68,120279,68,120331,68,120383,68,120435,68,5024,68,5598,68,5610,68,42195,68,8494,101,65349,101,8495,101,8519,101,119838,101,119890,101,119942,101,120046,101,120098,101,120150,101,120202,101,120254,101,120306,101,120358,101,120410,101,120462,101,43826,101,1213,101,8959,69,65317,69,8496,69,119812,69,119864,69,119916,69,120020,69,120072,69,120124,69,120176,69,120228,69,120280,69,120332,69,120384,69,120436,69,917,69,120492,69,120550,69,120608,69,120666,69,120724,69,11577,69,5036,69,42224,69,71846,69,71854,69,66182,69,119839,102,119891,102,119943,102,119995,102,120047,102,120099,102,120151,102,120203,102,120255,102,120307,102,120359,102,120411,102,120463,102,43829,102,42905,102,383,102,7837,102,1412,102,119315,70,8497,70,119813,70,119865,70,119917,70,120021,70,120073,70,120125,70,120177,70,120229,70,120281,70,120333,70,120385,70,120437,70,42904,70,988,70,120778,70,5556,70,42205,70,71874,70,71842,70,66183,70,66213,70,66853,70,65351,103,8458,103,119840,103,119892,103,119944,103,120048,103,120100,103,120152,103,120204,103,120256,103,120308,103,120360,103,120412,103,120464,103,609,103,7555,103,397,103,1409,103,119814,71,119866,71,119918,71,119970,71,120022,71,120074,71,120126,71,120178,71,120230,71,120282,71,120334,71,120386,71,120438,71,1292,71,5056,71,5107,71,42198,71,65352,104,8462,104,119841,104,119945,104,119997,104,120049,104,120101,104,120153,104,120205,104,120257,104,120309,104,120361,104,120413,104,120465,104,1211,104,1392,104,5058,104,65320,72,8459,72,8460,72,8461,72,119815,72,119867,72,119919,72,120023,72,120179,72,120231,72,120283,72,120335,72,120387,72,120439,72,919,72,120494,72,120552,72,120610,72,120668,72,120726,72,11406,72,5051,72,5500,72,42215,72,66255,72,731,105,9075,105,65353,105,8560,105,8505,105,8520,105,119842,105,119894,105,119946,105,119998,105,120050,105,120102,105,120154,105,120206,105,120258,105,120310,105,120362,105,120414,105,120466,105,120484,105,618,105,617,105,953,105,8126,105,890,105,120522,105,120580,105,120638,105,120696,105,120754,105,1110,105,42567,105,1231,105,43893,105,5029,105,71875,105,65354,106,8521,106,119843,106,119895,106,119947,106,119999,106,120051,106,120103,106,120155,106,120207,106,120259,106,120311,106,120363,106,120415,106,120467,106,1011,106,1112,106,65322,74,119817,74,119869,74,119921,74,119973,74,120025,74,120077,74,120129,74,120181,74,120233,74,120285,74,120337,74,120389,74,120441,74,42930,74,895,74,1032,74,5035,74,5261,74,42201,74,119844,107,119896,107,119948,107,120000,107,120052,107,120104,107,120156,107,120208,107,120260,107,120312,107,120364,107,120416,107,120468,107,8490,75,65323,75,119818,75,119870,75,119922,75,119974,75,120026,75,120078,75,120130,75,120182,75,120234,75,120286,75,120338,75,120390,75,120442,75,922,75,120497,75,120555,75,120613,75,120671,75,120729,75,11412,75,5094,75,5845,75,42199,75,66840,75,1472,108,8739,73,9213,73,65512,73,1633,108,1777,73,66336,108,125127,108,120783,73,120793,73,120803,73,120813,73,120823,73,130033,73,65321,73,8544,73,8464,73,8465,73,119816,73,119868,73,119920,73,120024,73,120128,73,120180,73,120232,73,120284,73,120336,73,120388,73,120440,73,65356,108,8572,73,8467,108,119845,108,119897,108,119949,108,120001,108,120053,108,120105,73,120157,73,120209,73,120261,73,120313,73,120365,73,120417,73,120469,73,448,73,120496,73,120554,73,120612,73,120670,73,120728,73,11410,73,1030,73,1216,73,1493,108,1503,108,1575,108,126464,108,126592,108,65166,108,65165,108,1994,108,11599,73,5825,73,42226,73,93992,73,66186,124,66313,124,119338,76,8556,76,8466,76,119819,76,119871,76,119923,76,120027,76,120079,76,120131,76,120183,76,120235,76,120287,76,120339,76,120391,76,120443,76,11472,76,5086,76,5290,76,42209,76,93974,76,71843,76,71858,76,66587,76,66854,76,65325,77,8559,77,8499,77,119820,77,119872,77,119924,77,120028,77,120080,77,120132,77,120184,77,120236,77,120288,77,120340,77,120392,77,120444,77,924,77,120499,77,120557,77,120615,77,120673,77,120731,77,1018,77,11416,77,5047,77,5616,77,5846,77,42207,77,66224,77,66321,77,119847,110,119899,110,119951,110,120003,110,120055,110,120107,110,120159,110,120211,110,120263,110,120315,110,120367,110,120419,110,120471,110,1400,110,1404,110,65326,78,8469,78,119821,78,119873,78,119925,78,119977,78,120029,78,120081,78,120185,78,120237,78,120289,78,120341,78,120393,78,120445,78,925,78,120500,78,120558,78,120616,78,120674,78,120732,78,11418,78,42208,78,66835,78,3074,111,3202,111,3330,111,3458,111,2406,111,2662,111,2790,111,3046,111,3174,111,3302,111,3430,111,3664,111,3792,111,4160,111,1637,111,1781,111,65359,111,8500,111,119848,111,119900,111,119952,111,120056,111,120108,111,120160,111,120212,111,120264,111,120316,111,120368,111,120420,111,120472,111,7439,111,7441,111,43837,111,959,111,120528,111,120586,111,120644,111,120702,111,120760,111,963,111,120532,111,120590,111,120648,111,120706,111,120764,111,11423,111,4351,111,1413,111,1505,111,1607,111,126500,111,126564,111,126596,111,65259,111,65260,111,65258,111,65257,111,1726,111,64428,111,64429,111,64427,111,64426,111,1729,111,64424,111,64425,111,64423,111,64422,111,1749,111,3360,111,4125,111,66794,111,71880,111,71895,111,66604,111,1984,79,2534,79,2918,79,12295,79,70864,79,71904,79,120782,79,120792,79,120802,79,120812,79,120822,79,130032,79,65327,79,119822,79,119874,79,119926,79,119978,79,120030,79,120082,79,120134,79,120186,79,120238,79,120290,79,120342,79,120394,79,120446,79,927,79,120502,79,120560,79,120618,79,120676,79,120734,79,11422,79,1365,79,11604,79,4816,79,2848,79,66754,79,42227,79,71861,79,66194,79,66219,79,66564,79,66838,79,9076,112,65360,112,119849,112,119901,112,119953,112,120005,112,120057,112,120109,112,120161,112,120213,112,120265,112,120317,112,120369,112,120421,112,120473,112,961,112,120530,112,120544,112,120588,112,120602,112,120646,112,120660,112,120704,112,120718,112,120762,112,120776,112,11427,112,65328,80,8473,80,119823,80,119875,80,119927,80,119979,80,120031,80,120083,80,120187,80,120239,80,120291,80,120343,80,120395,80,120447,80,929,80,120504,80,120562,80,120620,80,120678,80,120736,80,11426,80,5090,80,5229,80,42193,80,66197,80,119850,113,119902,113,119954,113,120006,113,120058,113,120110,113,120162,113,120214,113,120266,113,120318,113,120370,113,120422,113,120474,113,1307,113,1379,113,1382,113,8474,81,119824,81,119876,81,119928,81,119980,81,120032,81,120084,81,120188,81,120240,81,120292,81,120344,81,120396,81,120448,81,11605,81,119851,114,119903,114,119955,114,120007,114,120059,114,120111,114,120163,114,120215,114,120267,114,120319,114,120371,114,120423,114,120475,114,43847,114,43848,114,7462,114,11397,114,43905,114,119318,82,8475,82,8476,82,8477,82,119825,82,119877,82,119929,82,120033,82,120189,82,120241,82,120293,82,120345,82,120397,82,120449,82,422,82,5025,82,5074,82,66740,82,5511,82,42211,82,94005,82,65363,115,119852,115,119904,115,119956,115,120008,115,120060,115,120112,115,120164,115,120216,115,120268,115,120320,115,120372,115,120424,115,120476,115,42801,115,445,115,1109,115,43946,115,71873,115,66632,115,65331,83,119826,83,119878,83,119930,83,119982,83,120034,83,120086,83,120138,83,120190,83,120242,83,120294,83,120346,83,120398,83,120450,83,1029,83,1359,83,5077,83,5082,83,42210,83,94010,83,66198,83,66592,83,119853,116,119905,116,119957,116,120009,116,120061,116,120113,116,120165,116,120217,116,120269,116,120321,116,120373,116,120425,116,120477,116,8868,84,10201,84,128872,84,65332,84,119827,84,119879,84,119931,84,119983,84,120035,84,120087,84,120139,84,120191,84,120243,84,120295,84,120347,84,120399,84,120451,84,932,84,120507,84,120565,84,120623,84,120681,84,120739,84,11430,84,5026,84,42196,84,93962,84,71868,84,66199,84,66225,84,66325,84,119854,117,119906,117,119958,117,120010,117,120062,117,120114,117,120166,117,120218,117,120270,117,120322,117,120374,117,120426,117,120478,117,42911,117,7452,117,43854,117,43858,117,651,117,965,117,120534,117,120592,117,120650,117,120708,117,120766,117,1405,117,66806,117,71896,117,8746,85,8899,85,119828,85,119880,85,119932,85,119984,85,120036,85,120088,85,120140,85,120192,85,120244,85,120296,85,120348,85,120400,85,120452,85,1357,85,4608,85,66766,85,5196,85,42228,85,94018,85,71864,85,8744,118,8897,118,65366,118,8564,118,119855,118,119907,118,119959,118,120011,118,120063,118,120115,118,120167,118,120219,118,120271,118,120323,118,120375,118,120427,118,120479,118,7456,118,957,118,120526,118,120584,118,120642,118,120700,118,120758,118,1141,118,1496,118,71430,118,43945,118,71872,118,119309,86,1639,86,1783,86,8548,86,119829,86,119881,86,119933,86,119985,86,120037,86,120089,86,120141,86,120193,86,120245,86,120297,86,120349,86,120401,86,120453,86,1140,86,11576,86,5081,86,5167,86,42719,86,42214,86,93960,86,71840,86,66845,86,623,119,119856,119,119908,119,119960,119,120012,119,120064,119,120116,119,120168,119,120220,119,120272,119,120324,119,120376,119,120428,119,120480,119,7457,119,1121,119,1309,119,1377,119,71434,119,71438,119,71439,119,43907,119,71919,87,71910,87,119830,87,119882,87,119934,87,119986,87,120038,87,120090,87,120142,87,120194,87,120246,87,120298,87,120350,87,120402,87,120454,87,1308,87,5043,87,5076,87,42218,87,5742,120,10539,120,10540,120,10799,120,65368,120,8569,120,119857,120,119909,120,119961,120,120013,120,120065,120,120117,120,120169,120,120221,120,120273,120,120325,120,120377,120,120429,120,120481,120,5441,120,5501,120,5741,88,9587,88,66338,88,71916,88,65336,88,8553,88,119831,88,119883,88,119935,88,119987,88,120039,88,120091,88,120143,88,120195,88,120247,88,120299,88,120351,88,120403,88,120455,88,42931,88,935,88,120510,88,120568,88,120626,88,120684,88,120742,88,11436,88,11613,88,5815,88,42219,88,66192,88,66228,88,66327,88,66855,88,611,121,7564,121,65369,121,119858,121,119910,121,119962,121,120014,121,120066,121,120118,121,120170,121,120222,121,120274,121,120326,121,120378,121,120430,121,120482,121,655,121,7935,121,43866,121,947,121,8509,121,120516,121,120574,121,120632,121,120690,121,120748,121,1199,121,4327,121,71900,121,65337,89,119832,89,119884,89,119936,89,119988,89,120040,89,120092,89,120144,89,120196,89,120248,89,120300,89,120352,89,120404,89,120456,89,933,89,978,89,120508,89,120566,89,120624,89,120682,89,120740,89,11432,89,1198,89,5033,89,5053,89,42220,89,94019,89,71844,89,66226,89,119859,122,119911,122,119963,122,120015,122,120067,122,120119,122,120171,122,120223,122,120275,122,120327,122,120379,122,120431,122,120483,122,7458,122,43923,122,71876,122,66293,90,71909,90,65338,90,8484,90,8488,90,119833,90,119885,90,119937,90,119989,90,120041,90,120197,90,120249,90,120301,90,120353,90,120405,90,120457,90,918,90,120493,90,120551,90,120609,90,120667,90,120725,90,5059,90,42204,90,71849,90,65282,34,65284,36,65285,37,65286,38,65290,42,65291,43,65294,46,65295,47,65296,48,65297,49,65298,50,65299,51,65300,52,65301,53,65302,54,65303,55,65304,56,65305,57,65308,60,65309,61,65310,62,65312,64,65316,68,65318,70,65319,71,65324,76,65329,81,65330,82,65333,85,65334,86,65335,87,65343,95,65346,98,65348,100,65350,102,65355,107,65357,109,65358,110,65361,113,65362,114,65364,116,65365,117,65367,119,65370,122,65371,123,65373,125,119846,109],"_default":[160,32,8211,45,65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"cs":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"de":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"es":[8211,45,65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"fr":[65374,126,65306,58,65281,33,8216,96,8245,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"it":[160,32,8211,45,65374,126,65306,58,65281,33,8216,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"ja":[8211,45,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65292,44,65307,59],"ko":[8211,45,65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"pl":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"pt-BR":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"qps-ploc":[160,32,8211,45,65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"ru":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,305,105,921,73,1009,112,215,120,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"tr":[160,32,8211,45,65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"zh-hans":[65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65288,40,65289,41],"zh-hant":[8211,45,65374,126,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65307,59]}'));
st.cache = new Ha((e) => {
  function t(u) {
    const h = /* @__PURE__ */ new Map();
    for (let f = 0; f < u.length; f += 2)
      h.set(u[f], u[f + 1]);
    return h;
  }
  function n(u, h) {
    const f = new Map(u);
    for (const [d, g] of h)
      f.set(d, g);
    return f;
  }
  function r(u, h) {
    if (!u)
      return h;
    const f = /* @__PURE__ */ new Map();
    for (const [d, g] of u)
      h.has(d) && f.set(d, g);
    return f;
  }
  const i = St.ambiguousCharacterData.value;
  let s = e.filter((u) => !u.startsWith("_") && u in i);
  s.length === 0 && (s = ["_default"]);
  let a;
  for (const u of s) {
    const h = t(i[u]);
    a = r(a, h);
  }
  const o = t(i._common), l = n(o, a);
  return new St(l);
});
st._locales = new Gs(() => Object.keys(St.ambiguousCharacterData.value).filter((e) => !e.startsWith("_")));
class Ze {
  static getRawData() {
    return JSON.parse("[9,10,11,12,13,32,127,160,173,847,1564,4447,4448,6068,6069,6155,6156,6157,6158,7355,7356,8192,8193,8194,8195,8196,8197,8198,8199,8200,8201,8202,8203,8204,8205,8206,8207,8234,8235,8236,8237,8238,8239,8287,8288,8289,8290,8291,8292,8293,8294,8295,8296,8297,8298,8299,8300,8301,8302,8303,10240,12288,12644,65024,65025,65026,65027,65028,65029,65030,65031,65032,65033,65034,65035,65036,65037,65038,65039,65279,65440,65520,65521,65522,65523,65524,65525,65526,65527,65528,65532,78844,119155,119156,119157,119158,119159,119160,119161,119162,917504,917505,917506,917507,917508,917509,917510,917511,917512,917513,917514,917515,917516,917517,917518,917519,917520,917521,917522,917523,917524,917525,917526,917527,917528,917529,917530,917531,917532,917533,917534,917535,917536,917537,917538,917539,917540,917541,917542,917543,917544,917545,917546,917547,917548,917549,917550,917551,917552,917553,917554,917555,917556,917557,917558,917559,917560,917561,917562,917563,917564,917565,917566,917567,917568,917569,917570,917571,917572,917573,917574,917575,917576,917577,917578,917579,917580,917581,917582,917583,917584,917585,917586,917587,917588,917589,917590,917591,917592,917593,917594,917595,917596,917597,917598,917599,917600,917601,917602,917603,917604,917605,917606,917607,917608,917609,917610,917611,917612,917613,917614,917615,917616,917617,917618,917619,917620,917621,917622,917623,917624,917625,917626,917627,917628,917629,917630,917631,917760,917761,917762,917763,917764,917765,917766,917767,917768,917769,917770,917771,917772,917773,917774,917775,917776,917777,917778,917779,917780,917781,917782,917783,917784,917785,917786,917787,917788,917789,917790,917791,917792,917793,917794,917795,917796,917797,917798,917799,917800,917801,917802,917803,917804,917805,917806,917807,917808,917809,917810,917811,917812,917813,917814,917815,917816,917817,917818,917819,917820,917821,917822,917823,917824,917825,917826,917827,917828,917829,917830,917831,917832,917833,917834,917835,917836,917837,917838,917839,917840,917841,917842,917843,917844,917845,917846,917847,917848,917849,917850,917851,917852,917853,917854,917855,917856,917857,917858,917859,917860,917861,917862,917863,917864,917865,917866,917867,917868,917869,917870,917871,917872,917873,917874,917875,917876,917877,917878,917879,917880,917881,917882,917883,917884,917885,917886,917887,917888,917889,917890,917891,917892,917893,917894,917895,917896,917897,917898,917899,917900,917901,917902,917903,917904,917905,917906,917907,917908,917909,917910,917911,917912,917913,917914,917915,917916,917917,917918,917919,917920,917921,917922,917923,917924,917925,917926,917927,917928,917929,917930,917931,917932,917933,917934,917935,917936,917937,917938,917939,917940,917941,917942,917943,917944,917945,917946,917947,917948,917949,917950,917951,917952,917953,917954,917955,917956,917957,917958,917959,917960,917961,917962,917963,917964,917965,917966,917967,917968,917969,917970,917971,917972,917973,917974,917975,917976,917977,917978,917979,917980,917981,917982,917983,917984,917985,917986,917987,917988,917989,917990,917991,917992,917993,917994,917995,917996,917997,917998,917999]");
  }
  static getData() {
    return this._data || (this._data = new Set(Ze.getRawData())), this._data;
  }
  static isInvisibleCharacter(t) {
    return Ze.getData().has(t);
  }
  static get codePoints() {
    return Ze.getData();
  }
}
Ze._data = void 0;
const to = "$initialize";
class no {
  constructor(t, n, r, i) {
    this.vsWorker = t, this.req = n, this.method = r, this.args = i, this.type = 0;
  }
}
class Or {
  constructor(t, n, r, i) {
    this.vsWorker = t, this.seq = n, this.res = r, this.err = i, this.type = 1;
  }
}
class ro {
  constructor(t, n, r, i) {
    this.vsWorker = t, this.req = n, this.eventName = r, this.arg = i, this.type = 2;
  }
}
class io {
  constructor(t, n, r) {
    this.vsWorker = t, this.req = n, this.event = r, this.type = 3;
  }
}
class so {
  constructor(t, n) {
    this.vsWorker = t, this.req = n, this.type = 4;
  }
}
class ao {
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
      }, this._send(new no(this._workerId, r, t, n));
    });
  }
  listen(t, n) {
    let r = null;
    const i = new Ae({
      onWillAddFirstListener: () => {
        r = String(++this._lastSentReq), this._pendingEmitters.set(r, i), this._send(new ro(this._workerId, r, t, n));
      },
      onDidRemoveLastListener: () => {
        this._pendingEmitters.delete(r), this._send(new so(this._workerId, r)), r = null;
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
      this._send(new Or(this._workerId, n, i, void 0));
    }, (i) => {
      i.detail instanceof Error && (i.detail = Vr(i.detail)), this._send(new Or(this._workerId, n, void 0, Vr(i)));
    });
  }
  _handleSubscribeEventMessage(t) {
    const n = t.req, r = this._handler.handleEvent(t.eventName, t.arg)((i) => {
      this._send(new io(this._workerId, n, i));
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
function Xs(e) {
  return e[0] === "o" && e[1] === "n" && Js(e.charCodeAt(2));
}
function Qs(e) {
  return /^onDynamic/.test(e) && Js(e.charCodeAt(9));
}
function oo(e, t, n) {
  const r = (a) => function() {
    const o = Array.prototype.slice.call(arguments, 0);
    return t(a, o);
  }, i = (a) => function(o) {
    return n(a, o);
  }, s = {};
  for (const a of e) {
    if (Qs(a)) {
      s[a] = i(a);
      continue;
    }
    if (Xs(a)) {
      s[a] = n(a, void 0);
      continue;
    }
    s[a] = r(a);
  }
  return s;
}
class lo {
  constructor(t, n) {
    this._requestHandlerFactory = n, this._requestHandler = null, this._protocol = new ao({
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
    if (t === to)
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
    if (Qs(t)) {
      const r = this._requestHandler[t].call(this._requestHandler, n);
      if (typeof r != "function")
        throw new Error(`Missing dynamic event ${t} on request handler.`);
      return r;
    }
    if (Xs(t)) {
      const r = this._requestHandler[t];
      if (typeof r != "function")
        throw new Error(`Missing event ${t} on request handler.`);
      return r;
    }
    throw new Error(`Malformed event name ${t}`);
  }
  initialize(t, n, r, i) {
    this._protocol.setWorkerId(t);
    const o = oo(i, (l, u) => this._protocol.sendMessage(l, u), (l, u) => this._protocol.listen(l, u));
    return this._requestHandlerFactory ? (this._requestHandler = this._requestHandlerFactory(o), Promise.resolve(Hn(this._requestHandler))) : (n && (typeof n.baseUrl < "u" && delete n.baseUrl, typeof n.paths < "u" && typeof n.paths.vs < "u" && delete n.paths.vs, typeof n.trustedTypesPolicy !== void 0 && delete n.trustedTypesPolicy, n.catchError = !0, globalThis.require.config(n)), new Promise((l, u) => {
      const h = globalThis.require;
      h([r], (f) => {
        if (this._requestHandler = f.create(o), !this._requestHandler) {
          u(new Error("No RequestHandler!"));
          return;
        }
        l(Hn(this._requestHandler));
      }, u);
    }));
  }
}
class Je {
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
function jr(e, t) {
  return (t << 5) - t + e | 0;
}
function uo(e, t) {
  t = jr(149417, t);
  for (let n = 0, r = e.length; n < r; n++)
    t = jr(e.charCodeAt(n), t);
  return t;
}
class Br {
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
function co(e, t, n) {
  return new Qe(new Br(e), new Br(t)).ComputeDiff(n).changes;
}
class ut {
  static Assert(t, n) {
    if (!t)
      throw new Error(n);
  }
}
class ct {
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
class qr {
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
    (this.m_originalCount > 0 || this.m_modifiedCount > 0) && this.m_changes.push(new Je(this.m_originalStart, this.m_originalCount, this.m_modifiedStart, this.m_modifiedCount)), this.m_originalCount = 0, this.m_modifiedCount = 0, this.m_originalStart = 1073741824, this.m_modifiedStart = 1073741824;
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
class Qe {
  /**
   * Constructs the DiffFinder
   */
  constructor(t, n, r = null) {
    this.ContinueProcessingPredicate = r, this._originalSequence = t, this._modifiedSequence = n;
    const [i, s, a] = Qe._getElements(t), [o, l, u] = Qe._getElements(n);
    this._hasStrings = a && u, this._originalStringElements = i, this._originalElementsOrHash = s, this._modifiedStringElements = o, this._modifiedElementsOrHash = l, this.m_forwardHistory = [], this.m_reverseHistory = [];
  }
  static _isStringArray(t) {
    return t.length > 0 && typeof t[0] == "string";
  }
  static _getElements(t) {
    const n = t.getElements();
    if (Qe._isStringArray(n)) {
      const r = new Int32Array(n.length);
      for (let i = 0, s = n.length; i < s; i++)
        r[i] = uo(n[i], 0);
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
    const r = Qe._getStrictElement(this._originalSequence, t), i = Qe._getStrictElement(this._modifiedSequence, n);
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
      return r <= i ? (ut.Assert(t === n + 1, "originalStart should only be one more than originalEnd"), f = [
        new Je(t, 0, r, i - r + 1)
      ]) : t <= n ? (ut.Assert(r === i + 1, "modifiedStart should only be one more than modifiedEnd"), f = [
        new Je(t, n - t + 1, r, 0)
      ]) : (ut.Assert(t === n + 1, "originalStart should only be one more than originalEnd"), ut.Assert(r === i + 1, "modifiedStart should only be one more than modifiedEnd"), f = []), f;
    }
    const a = [0], o = [0], l = this.ComputeRecursionPoint(t, n, r, i, a, o, s), u = a[0], h = o[0];
    if (l !== null)
      return l;
    if (!s[0]) {
      const f = this.ComputeDiffRecursive(t, u, r, h, s);
      let d = [];
      return s[0] ? d = [
        new Je(u + 1, n - (u + 1) + 1, h + 1, i - (h + 1) + 1)
      ] : d = this.ComputeDiffRecursive(u + 1, n, h + 1, i, s), this.ConcatenateChanges(f, d);
    }
    return [
      new Je(t, n - t + 1, r, i - r + 1)
    ];
  }
  WALKTRACE(t, n, r, i, s, a, o, l, u, h, f, d, g, m, p, b, y, x) {
    let v = null, C = null, L = new qr(), _ = n, S = r, A = g[0] - b[0] - i, R = -1073741824, k = this.m_forwardHistory.length - 1;
    do {
      const w = A + t;
      w === _ || w < S && u[w - 1] < u[w + 1] ? (f = u[w + 1], m = f - A - i, f < R && L.MarkNextChange(), R = f, L.AddModifiedElement(f + 1, m), A = w + 1 - t) : (f = u[w - 1] + 1, m = f - A - i, f < R && L.MarkNextChange(), R = f - 1, L.AddOriginalElement(f, m + 1), A = w - 1 - t), k >= 0 && (u = this.m_forwardHistory[k], t = u[0], _ = 1, S = u.length - 1);
    } while (--k >= -1);
    if (v = L.getReverseChanges(), x[0]) {
      let w = g[0] + 1, N = b[0] + 1;
      if (v !== null && v.length > 0) {
        const T = v[v.length - 1];
        w = Math.max(w, T.getOriginalEnd()), N = Math.max(N, T.getModifiedEnd());
      }
      C = [
        new Je(w, d - w + 1, N, p - N + 1)
      ];
    } else {
      L = new qr(), _ = a, S = o, A = g[0] - b[0] - l, R = 1073741824, k = y ? this.m_reverseHistory.length - 1 : this.m_reverseHistory.length - 2;
      do {
        const w = A + s;
        w === _ || w < S && h[w - 1] >= h[w + 1] ? (f = h[w + 1] - 1, m = f - A - l, f > R && L.MarkNextChange(), R = f + 1, L.AddOriginalElement(f + 1, m + 1), A = w + 1 - s) : (f = h[w - 1], m = f - A - l, f > R && L.MarkNextChange(), R = f, L.AddModifiedElement(f + 1, m + 1), A = w - 1 - s), k >= 0 && (h = this.m_reverseHistory[k], s = h[0], _ = 1, S = h.length - 1);
      } while (--k >= -1);
      C = L.getChanges();
    }
    return this.ConcatenateChanges(v, C);
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
    let l = 0, u = 0, h = 0, f = 0, d = 0, g = 0;
    t--, r--, s[0] = 0, a[0] = 0, this.m_forwardHistory = [], this.m_reverseHistory = [];
    const m = n - t + (i - r), p = m + 1, b = new Int32Array(p), y = new Int32Array(p), x = i - r, v = n - t, C = t - r, L = n - i, S = (v - x) % 2 === 0;
    b[x] = t, y[v] = n, o[0] = !1;
    for (let A = 1; A <= m / 2 + 1; A++) {
      let R = 0, k = 0;
      h = this.ClipDiagonalBound(x - A, A, x, p), f = this.ClipDiagonalBound(x + A, A, x, p);
      for (let N = h; N <= f; N += 2) {
        N === h || N < f && b[N - 1] < b[N + 1] ? l = b[N + 1] : l = b[N - 1] + 1, u = l - (N - x) - C;
        const T = l;
        for (; l < n && u < i && this.ElementsAreEqual(l + 1, u + 1); )
          l++, u++;
        if (b[N] = l, l + u > R + k && (R = l, k = u), !S && Math.abs(N - v) <= A - 1 && l >= y[N])
          return s[0] = l, a[0] = u, T <= y[N] && 1447 > 0 && A <= 1447 + 1 ? this.WALKTRACE(x, h, f, C, v, d, g, L, b, y, l, n, s, u, i, a, S, o) : null;
      }
      const w = (R - t + (k - r) - A) / 2;
      if (this.ContinueProcessingPredicate !== null && !this.ContinueProcessingPredicate(R, w))
        return o[0] = !0, s[0] = R, a[0] = k, w > 0 && 1447 > 0 && A <= 1447 + 1 ? this.WALKTRACE(x, h, f, C, v, d, g, L, b, y, l, n, s, u, i, a, S, o) : (t++, r++, [
          new Je(t, n - t + 1, r, i - r + 1)
        ]);
      d = this.ClipDiagonalBound(v - A, A, v, p), g = this.ClipDiagonalBound(v + A, A, v, p);
      for (let N = d; N <= g; N += 2) {
        N === d || N < g && y[N - 1] >= y[N + 1] ? l = y[N + 1] - 1 : l = y[N - 1], u = l - (N - v) - L;
        const T = l;
        for (; l > t && u > r && this.ElementsAreEqual(l, u); )
          l--, u--;
        if (y[N] = l, S && Math.abs(N - x) <= A && l <= b[N])
          return s[0] = l, a[0] = u, T >= b[N] && 1447 > 0 && A <= 1447 + 1 ? this.WALKTRACE(x, h, f, C, v, d, g, L, b, y, l, n, s, u, i, a, S, o) : null;
      }
      if (A <= 1447) {
        let N = new Int32Array(f - h + 2);
        N[0] = x - h + 1, ct.Copy2(b, h, N, 1, f - h + 1), this.m_forwardHistory.push(N), N = new Int32Array(g - d + 2), N[0] = v - d + 1, ct.Copy2(y, d, N, 1, g - d + 1), this.m_reverseHistory.push(N);
      }
    }
    return this.WALKTRACE(x, h, f, C, v, d, g, L, b, y, l, n, s, u, i, a, S, o);
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
      const h = [null];
      if (n > 0 && this.ChangesOverlap(t[n - 1], t[n], h)) {
        t[n - 1] = h[0], t.splice(n, 1), n++;
        continue;
      }
    }
    if (this._hasStrings)
      for (let n = 1, r = t.length; n < r; n++) {
        const i = t[n - 1], s = t[n], a = s.originalStart - i.originalStart - i.originalLength, o = i.originalStart, l = s.originalStart + s.originalLength, u = l - o, h = i.modifiedStart, f = s.modifiedStart + s.modifiedLength, d = f - h;
        if (a < 5 && u < 20 && d < 20) {
          const g = this._findBetterContiguousSequence(o, u, h, d, a);
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
    let l = 0, u = 0, h = 0;
    for (let f = t; f < a; f++)
      for (let d = r; d < o; d++) {
        const g = this._contiguousSequenceScore(f, d, s);
        g > 0 && g > l && (l = g, u = f, h = d);
      }
    return l > 0 ? [u, h] : null;
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
      return ct.Copy(t, 0, i, 0, t.length - 1), i[t.length - 1] = r[0], ct.Copy(n, 1, i, t.length, n.length - 1), i;
    } else {
      const i = new Array(t.length + n.length);
      return ct.Copy(t, 0, i, 0, t.length), ct.Copy(n, 0, i, t.length, n.length), i;
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
    if (ut.Assert(t.originalStart <= n.originalStart, "Left change is not less than or equal to right change"), ut.Assert(t.modifiedStart <= n.modifiedStart, "Left change is not less than or equal to right change"), t.originalStart + t.originalLength >= n.originalStart || t.modifiedStart + t.modifiedLength >= n.modifiedStart) {
      const i = t.originalStart;
      let s = t.originalLength;
      const a = t.modifiedStart;
      let o = t.modifiedLength;
      return t.originalStart + t.originalLength >= n.originalStart && (s = n.originalStart + n.originalLength - t.originalStart), t.modifiedStart + t.modifiedLength >= n.modifiedStart && (o = n.modifiedStart + n.modifiedLength - t.modifiedStart), r[0] = new Je(i, s, a, o), !0;
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
let vt;
if (typeof Re.vscode < "u" && typeof Re.vscode.process < "u") {
  const e = Re.vscode.process;
  vt = {
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
  typeof process < "u" ? vt = {
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
  } : vt = {
    // Supported
    get platform() {
      return Vt ? "win32" : Ua ? "darwin" : "linux";
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
const un = vt.cwd, fo = vt.env, ho = vt.platform, go = 65, mo = 97, po = 90, vo = 122, Ye = 46, ae = 47, he = 92, $e = 58, bo = 63;
class Zs extends Error {
  constructor(t, n, r) {
    let i;
    typeof n == "string" && n.indexOf("not ") === 0 ? (i = "must not be", n = n.replace(/^not /, "")) : i = "must be";
    const s = t.indexOf(".") !== -1 ? "property" : "argument";
    let a = `The "${t}" ${s} ${i} of type ${n}`;
    a += `. Received type ${typeof r}`, super(a), this.code = "ERR_INVALID_ARG_TYPE";
  }
}
function yo(e, t) {
  if (e === null || typeof e != "object")
    throw new Zs(t, "Object", e);
}
function K(e, t) {
  if (typeof e != "string")
    throw new Zs(t, "string", e);
}
const et = ho === "win32";
function W(e) {
  return e === ae || e === he;
}
function Xn(e) {
  return e === ae;
}
function We(e) {
  return e >= go && e <= po || e >= mo && e <= vo;
}
function cn(e, t, n, r) {
  let i = "", s = 0, a = -1, o = 0, l = 0;
  for (let u = 0; u <= e.length; ++u) {
    if (u < e.length)
      l = e.charCodeAt(u);
    else {
      if (r(l))
        break;
      l = ae;
    }
    if (r(l)) {
      if (!(a === u - 1 || o === 1))
        if (o === 2) {
          if (i.length < 2 || s !== 2 || i.charCodeAt(i.length - 1) !== Ye || i.charCodeAt(i.length - 2) !== Ye) {
            if (i.length > 2) {
              const h = i.lastIndexOf(n);
              h === -1 ? (i = "", s = 0) : (i = i.slice(0, h), s = i.length - 1 - i.lastIndexOf(n)), a = u, o = 0;
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
      l === Ye && o !== -1 ? ++o : o = -1;
  }
  return i;
}
function Ys(e, t) {
  yo(t, "pathObject");
  const n = t.dir || t.root, r = t.base || `${t.name || ""}${t.ext || ""}`;
  return n ? n === t.root ? `${n}${r}` : `${n}${e}${r}` : r;
}
const ce = {
  // path.resolve([from ...], to)
  resolve(...e) {
    let t = "", n = "", r = !1;
    for (let i = e.length - 1; i >= -1; i--) {
      let s;
      if (i >= 0) {
        if (s = e[i], K(s, "path"), s.length === 0)
          continue;
      } else
        t.length === 0 ? s = un() : (s = fo[`=${t}`] || un(), (s === void 0 || s.slice(0, 2).toLowerCase() !== t.toLowerCase() && s.charCodeAt(2) === he) && (s = `${t}\\`));
      const a = s.length;
      let o = 0, l = "", u = !1;
      const h = s.charCodeAt(0);
      if (a === 1)
        W(h) && (o = 1, u = !0);
      else if (W(h))
        if (u = !0, W(s.charCodeAt(1))) {
          let f = 2, d = f;
          for (; f < a && !W(s.charCodeAt(f)); )
            f++;
          if (f < a && f !== d) {
            const g = s.slice(d, f);
            for (d = f; f < a && W(s.charCodeAt(f)); )
              f++;
            if (f < a && f !== d) {
              for (d = f; f < a && !W(s.charCodeAt(f)); )
                f++;
              (f === a || f !== d) && (l = `\\\\${g}\\${s.slice(d, f)}`, o = f);
            }
          }
        } else
          o = 1;
      else
        We(h) && s.charCodeAt(1) === $e && (l = s.slice(0, 2), o = 2, a > 2 && W(s.charCodeAt(2)) && (u = !0, o = 3));
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
    return n = cn(n, !r, "\\", W), r ? `${t}\\${n}` : `${t}${n}` || ".";
  },
  normalize(e) {
    K(e, "path");
    const t = e.length;
    if (t === 0)
      return ".";
    let n = 0, r, i = !1;
    const s = e.charCodeAt(0);
    if (t === 1)
      return Xn(s) ? "\\" : e;
    if (W(s))
      if (i = !0, W(e.charCodeAt(1))) {
        let o = 2, l = o;
        for (; o < t && !W(e.charCodeAt(o)); )
          o++;
        if (o < t && o !== l) {
          const u = e.slice(l, o);
          for (l = o; o < t && W(e.charCodeAt(o)); )
            o++;
          if (o < t && o !== l) {
            for (l = o; o < t && !W(e.charCodeAt(o)); )
              o++;
            if (o === t)
              return `\\\\${u}\\${e.slice(l)}\\`;
            o !== l && (r = `\\\\${u}\\${e.slice(l, o)}`, n = o);
          }
        }
      } else
        n = 1;
    else
      We(s) && e.charCodeAt(1) === $e && (r = e.slice(0, 2), n = 2, t > 2 && W(e.charCodeAt(2)) && (i = !0, n = 3));
    let a = n < t ? cn(e.slice(n), !i, "\\", W) : "";
    return a.length === 0 && !i && (a = "."), a.length > 0 && W(e.charCodeAt(t - 1)) && (a += "\\"), r === void 0 ? i ? `\\${a}` : a : i ? `${r}\\${a}` : `${r}${a}`;
  },
  isAbsolute(e) {
    K(e, "path");
    const t = e.length;
    if (t === 0)
      return !1;
    const n = e.charCodeAt(0);
    return W(n) || // Possible device root
    t > 2 && We(n) && e.charCodeAt(1) === $e && W(e.charCodeAt(2));
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
    if (typeof n == "string" && W(n.charCodeAt(0))) {
      ++i;
      const s = n.length;
      s > 1 && W(n.charCodeAt(1)) && (++i, s > 2 && (W(n.charCodeAt(2)) ? ++i : r = !1));
    }
    if (r) {
      for (; i < t.length && W(t.charCodeAt(i)); )
        i++;
      i >= 2 && (t = `\\${t.slice(i)}`);
    }
    return ce.normalize(t);
  },
  // It will solve the relative path from `from` to `to`, for instance:
  //  from = 'C:\\orandea\\test\\aaa'
  //  to = 'C:\\orandea\\impl\\bbb'
  // The output of the function should be: '..\\..\\impl\\bbb'
  relative(e, t) {
    if (K(e, "from"), K(t, "to"), e === t)
      return "";
    const n = ce.resolve(e), r = ce.resolve(t);
    if (n === r || (e = n.toLowerCase(), t = r.toLowerCase(), e === t))
      return "";
    let i = 0;
    for (; i < e.length && e.charCodeAt(i) === he; )
      i++;
    let s = e.length;
    for (; s - 1 > i && e.charCodeAt(s - 1) === he; )
      s--;
    const a = s - i;
    let o = 0;
    for (; o < t.length && t.charCodeAt(o) === he; )
      o++;
    let l = t.length;
    for (; l - 1 > o && t.charCodeAt(l - 1) === he; )
      l--;
    const u = l - o, h = a < u ? a : u;
    let f = -1, d = 0;
    for (; d < h; d++) {
      const m = e.charCodeAt(i + d);
      if (m !== t.charCodeAt(o + d))
        break;
      m === he && (f = d);
    }
    if (d !== h) {
      if (f === -1)
        return r;
    } else {
      if (u > h) {
        if (t.charCodeAt(o + d) === he)
          return r.slice(o + d + 1);
        if (d === 2)
          return r.slice(o + d);
      }
      a > h && (e.charCodeAt(i + d) === he ? f = d : d === 2 && (f = 3)), f === -1 && (f = 0);
    }
    let g = "";
    for (d = i + f + 1; d <= s; ++d)
      (d === s || e.charCodeAt(d) === he) && (g += g.length === 0 ? ".." : "\\..");
    return o += f, g.length > 0 ? `${g}${r.slice(o, l)}` : (r.charCodeAt(o) === he && ++o, r.slice(o, l));
  },
  toNamespacedPath(e) {
    if (typeof e != "string" || e.length === 0)
      return e;
    const t = ce.resolve(e);
    if (t.length <= 2)
      return e;
    if (t.charCodeAt(0) === he) {
      if (t.charCodeAt(1) === he) {
        const n = t.charCodeAt(2);
        if (n !== bo && n !== Ye)
          return `\\\\?\\UNC\\${t.slice(2)}`;
      }
    } else if (We(t.charCodeAt(0)) && t.charCodeAt(1) === $e && t.charCodeAt(2) === he)
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
      return W(i) ? e : ".";
    if (W(i)) {
      if (n = r = 1, W(e.charCodeAt(1))) {
        let o = 2, l = o;
        for (; o < t && !W(e.charCodeAt(o)); )
          o++;
        if (o < t && o !== l) {
          for (l = o; o < t && W(e.charCodeAt(o)); )
            o++;
          if (o < t && o !== l) {
            for (l = o; o < t && !W(e.charCodeAt(o)); )
              o++;
            if (o === t)
              return e;
            o !== l && (n = r = o + 1);
          }
        }
      }
    } else
      We(i) && e.charCodeAt(1) === $e && (n = t > 2 && W(e.charCodeAt(2)) ? 3 : 2, r = n);
    let s = -1, a = !0;
    for (let o = t - 1; o >= r; --o)
      if (W(e.charCodeAt(o))) {
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
    if (e.length >= 2 && We(e.charCodeAt(0)) && e.charCodeAt(1) === $e && (n = 2), t !== void 0 && t.length > 0 && t.length <= e.length) {
      if (t === e)
        return "";
      let a = t.length - 1, o = -1;
      for (s = e.length - 1; s >= n; --s) {
        const l = e.charCodeAt(s);
        if (W(l)) {
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
      if (W(e.charCodeAt(s))) {
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
    e.length >= 2 && e.charCodeAt(1) === $e && We(e.charCodeAt(0)) && (t = r = 2);
    for (let o = e.length - 1; o >= t; --o) {
      const l = e.charCodeAt(o);
      if (W(l)) {
        if (!s) {
          r = o + 1;
          break;
        }
        continue;
      }
      i === -1 && (s = !1, i = o + 1), l === Ye ? n === -1 ? n = o : a !== 1 && (a = 1) : n !== -1 && (a = -1);
    }
    return n === -1 || i === -1 || // We saw a non-dot character immediately before the dot
    a === 0 || // The (right-most) trimmed path component is exactly '..'
    a === 1 && n === i - 1 && n === r + 1 ? "" : e.slice(n, i);
  },
  format: Ys.bind(null, "\\"),
  parse(e) {
    K(e, "path");
    const t = { root: "", dir: "", base: "", ext: "", name: "" };
    if (e.length === 0)
      return t;
    const n = e.length;
    let r = 0, i = e.charCodeAt(0);
    if (n === 1)
      return W(i) ? (t.root = t.dir = e, t) : (t.base = t.name = e, t);
    if (W(i)) {
      if (r = 1, W(e.charCodeAt(1))) {
        let f = 2, d = f;
        for (; f < n && !W(e.charCodeAt(f)); )
          f++;
        if (f < n && f !== d) {
          for (d = f; f < n && W(e.charCodeAt(f)); )
            f++;
          if (f < n && f !== d) {
            for (d = f; f < n && !W(e.charCodeAt(f)); )
              f++;
            f === n ? r = f : f !== d && (r = f + 1);
          }
        }
      }
    } else if (We(i) && e.charCodeAt(1) === $e) {
      if (n <= 2)
        return t.root = t.dir = e, t;
      if (r = 2, W(e.charCodeAt(2))) {
        if (n === 3)
          return t.root = t.dir = e, t;
        r = 3;
      }
    }
    r > 0 && (t.root = e.slice(0, r));
    let s = -1, a = r, o = -1, l = !0, u = e.length - 1, h = 0;
    for (; u >= r; --u) {
      if (i = e.charCodeAt(u), W(i)) {
        if (!l) {
          a = u + 1;
          break;
        }
        continue;
      }
      o === -1 && (l = !1, o = u + 1), i === Ye ? s === -1 ? s = u : h !== 1 && (h = 1) : s !== -1 && (h = -1);
    }
    return o !== -1 && (s === -1 || // We saw a non-dot character immediately before the dot
    h === 0 || // The (right-most) trimmed path component is exactly '..'
    h === 1 && s === o - 1 && s === a + 1 ? t.base = t.name = e.slice(a, o) : (t.name = e.slice(a, s), t.base = e.slice(a, o), t.ext = e.slice(s, o))), a > 0 && a !== r ? t.dir = e.slice(0, a - 1) : t.dir = t.root, t;
  },
  sep: "\\",
  delimiter: ";",
  win32: null,
  posix: null
}, xo = (() => {
  if (et) {
    const e = /\\/g;
    return () => {
      const t = un().replace(e, "/");
      return t.slice(t.indexOf("/"));
    };
  }
  return () => un();
})(), me = {
  // path.resolve([from ...], to)
  resolve(...e) {
    let t = "", n = !1;
    for (let r = e.length - 1; r >= -1 && !n; r--) {
      const i = r >= 0 ? e[r] : xo();
      K(i, "path"), i.length !== 0 && (t = `${i}/${t}`, n = i.charCodeAt(0) === ae);
    }
    return t = cn(t, !n, "/", Xn), n ? `/${t}` : t.length > 0 ? t : ".";
  },
  normalize(e) {
    if (K(e, "path"), e.length === 0)
      return ".";
    const t = e.charCodeAt(0) === ae, n = e.charCodeAt(e.length - 1) === ae;
    return e = cn(e, !t, "/", Xn), e.length === 0 ? t ? "/" : n ? "./" : "." : (n && (e += "/"), t ? `/${e}` : e);
  },
  isAbsolute(e) {
    return K(e, "path"), e.length > 0 && e.charCodeAt(0) === ae;
  },
  join(...e) {
    if (e.length === 0)
      return ".";
    let t;
    for (let n = 0; n < e.length; ++n) {
      const r = e[n];
      K(r, "path"), r.length > 0 && (t === void 0 ? t = r : t += `/${r}`);
    }
    return t === void 0 ? "." : me.normalize(t);
  },
  relative(e, t) {
    if (K(e, "from"), K(t, "to"), e === t || (e = me.resolve(e), t = me.resolve(t), e === t))
      return "";
    const n = 1, r = e.length, i = r - n, s = 1, a = t.length - s, o = i < a ? i : a;
    let l = -1, u = 0;
    for (; u < o; u++) {
      const f = e.charCodeAt(n + u);
      if (f !== t.charCodeAt(s + u))
        break;
      f === ae && (l = u);
    }
    if (u === o)
      if (a > o) {
        if (t.charCodeAt(s + u) === ae)
          return t.slice(s + u + 1);
        if (u === 0)
          return t.slice(s + u);
      } else
        i > o && (e.charCodeAt(n + u) === ae ? l = u : u === 0 && (l = 0));
    let h = "";
    for (u = n + l + 1; u <= r; ++u)
      (u === r || e.charCodeAt(u) === ae) && (h += h.length === 0 ? ".." : "/..");
    return `${h}${t.slice(s + l)}`;
  },
  toNamespacedPath(e) {
    return e;
  },
  dirname(e) {
    if (K(e, "path"), e.length === 0)
      return ".";
    const t = e.charCodeAt(0) === ae;
    let n = -1, r = !0;
    for (let i = e.length - 1; i >= 1; --i)
      if (e.charCodeAt(i) === ae) {
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
        if (l === ae) {
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
      if (e.charCodeAt(s) === ae) {
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
      if (o === ae) {
        if (!i) {
          n = a + 1;
          break;
        }
        continue;
      }
      r === -1 && (i = !1, r = a + 1), o === Ye ? t === -1 ? t = a : s !== 1 && (s = 1) : t !== -1 && (s = -1);
    }
    return t === -1 || r === -1 || // We saw a non-dot character immediately before the dot
    s === 0 || // The (right-most) trimmed path component is exactly '..'
    s === 1 && t === r - 1 && t === n + 1 ? "" : e.slice(t, r);
  },
  format: Ys.bind(null, "/"),
  parse(e) {
    K(e, "path");
    const t = { root: "", dir: "", base: "", ext: "", name: "" };
    if (e.length === 0)
      return t;
    const n = e.charCodeAt(0) === ae;
    let r;
    n ? (t.root = "/", r = 1) : r = 0;
    let i = -1, s = 0, a = -1, o = !0, l = e.length - 1, u = 0;
    for (; l >= r; --l) {
      const h = e.charCodeAt(l);
      if (h === ae) {
        if (!o) {
          s = l + 1;
          break;
        }
        continue;
      }
      a === -1 && (o = !1, a = l + 1), h === Ye ? i === -1 ? i = l : u !== 1 && (u = 1) : i !== -1 && (u = -1);
    }
    if (a !== -1) {
      const h = s === 0 && n ? 1 : s;
      i === -1 || // We saw a non-dot character immediately before the dot
      u === 0 || // The (right-most) trimmed path component is exactly '..'
      u === 1 && i === a - 1 && i === s + 1 ? t.base = t.name = e.slice(h, a) : (t.name = e.slice(h, i), t.base = e.slice(h, a), t.ext = e.slice(i, a));
    }
    return s > 0 ? t.dir = e.slice(0, s - 1) : n && (t.dir = "/"), t;
  },
  sep: "/",
  delimiter: ":",
  win32: null,
  posix: null
};
me.win32 = ce.win32 = ce;
me.posix = ce.posix = me;
et ? ce.normalize : me.normalize;
et ? ce.resolve : me.resolve;
et ? ce.relative : me.relative;
et ? ce.dirname : me.dirname;
et ? ce.basename : me.basename;
et ? ce.extname : me.extname;
et ? ce.sep : me.sep;
const wo = /^\w[\w\d+.-]*$/, _o = /^\//, So = /^\/\//;
function Lo(e, t) {
  if (!e.scheme && t)
    throw new Error(`[UriError]: Scheme is missing: {scheme: "", authority: "${e.authority}", path: "${e.path}", query: "${e.query}", fragment: "${e.fragment}"}`);
  if (e.scheme && !wo.test(e.scheme))
    throw new Error("[UriError]: Scheme contains illegal characters.");
  if (e.path) {
    if (e.authority) {
      if (!_o.test(e.path))
        throw new Error('[UriError]: If a URI contains an authority component, then the path component must either be empty or begin with a slash ("/") character');
    } else if (So.test(e.path))
      throw new Error('[UriError]: If a URI does not contain an authority component, then the path cannot begin with two slash characters ("//")');
  }
}
function No(e, t) {
  return !e && !t ? "file" : e;
}
function Ao(e, t) {
  switch (e) {
    case "https":
    case "http":
    case "file":
      t ? t[0] !== Ce && (t = Ce + t) : t = Ce;
      break;
  }
  return t;
}
const Z = "", Ce = "/", Co = /^(([^:/?#]+?):)?(\/\/([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?/;
let Sr = class rn {
  static isUri(t) {
    return t instanceof rn ? !0 : t ? typeof t.authority == "string" && typeof t.fragment == "string" && typeof t.path == "string" && typeof t.query == "string" && typeof t.scheme == "string" && typeof t.fsPath == "string" && typeof t.with == "function" && typeof t.toString == "function" : !1;
  }
  /**
   * @internal
   */
  constructor(t, n, r, i, s, a = !1) {
    typeof t == "object" ? (this.scheme = t.scheme || Z, this.authority = t.authority || Z, this.path = t.path || Z, this.query = t.query || Z, this.fragment = t.fragment || Z) : (this.scheme = No(t, a), this.authority = n || Z, this.path = Ao(this.scheme, r || Z), this.query = i || Z, this.fragment = s || Z, Lo(this, a));
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
    return Qn(this, !1);
  }
  // ---- modify to new -------------------------
  with(t) {
    if (!t)
      return this;
    let { scheme: n, authority: r, path: i, query: s, fragment: a } = t;
    return n === void 0 ? n = this.scheme : n === null && (n = Z), r === void 0 ? r = this.authority : r === null && (r = Z), i === void 0 ? i = this.path : i === null && (i = Z), s === void 0 ? s = this.query : s === null && (s = Z), a === void 0 ? a = this.fragment : a === null && (a = Z), n === this.scheme && r === this.authority && i === this.path && s === this.query && a === this.fragment ? this : new ft(n, r, i, s, a);
  }
  // ---- parse & validate ------------------------
  /**
   * Creates a new URI from a string, e.g. `http://www.example.com/some/path`,
   * `file:///usr/home`, or `scheme:with/path`.
   *
   * @param value A string which represents an URI (see `URI#toString`).
   */
  static parse(t, n = !1) {
    const r = Co.exec(t);
    return r ? new ft(r[2] || Z, Xt(r[4] || Z), Xt(r[5] || Z), Xt(r[7] || Z), Xt(r[9] || Z), n) : new ft(Z, Z, Z, Z, Z);
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
    if (Vt && (t = t.replace(/\\/g, Ce)), t[0] === Ce && t[1] === Ce) {
      const r = t.indexOf(Ce, 2);
      r === -1 ? (n = t.substring(2), t = Ce) : (n = t.substring(2, r), t = t.substring(r) || Ce);
    }
    return new ft("file", n, t, Z, Z);
  }
  /**
   * Creates new URI from uri components.
   *
   * Unless `strict` is `true` the scheme is defaults to be `file`. This function performs
   * validation and should be used for untrusted uri components retrieved from storage,
   * user input, command arguments etc
   */
  static from(t, n) {
    return new ft(t.scheme, t.authority, t.path, t.query, t.fragment, n);
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
    return Vt && t.scheme === "file" ? r = rn.file(ce.join(Qn(t, !0), ...n)).path : r = me.join(t.path, ...n), t.with({ path: r });
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
    return Zn(this, t);
  }
  toJSON() {
    return this;
  }
  static revive(t) {
    var n, r;
    if (t) {
      if (t instanceof rn)
        return t;
      {
        const i = new ft(t);
        return i._formatted = (n = t.external) !== null && n !== void 0 ? n : null, i._fsPath = t._sep === Ks && (r = t.fsPath) !== null && r !== void 0 ? r : null, i;
      }
    } else
      return t;
  }
};
const Ks = Vt ? 1 : void 0;
class ft extends Sr {
  constructor() {
    super(...arguments), this._formatted = null, this._fsPath = null;
  }
  get fsPath() {
    return this._fsPath || (this._fsPath = Qn(this, !1)), this._fsPath;
  }
  toString(t = !1) {
    return t ? Zn(this, !0) : (this._formatted || (this._formatted = Zn(this, !1)), this._formatted);
  }
  toJSON() {
    const t = {
      $mid: 1
      /* MarshalledId.Uri */
    };
    return this._fsPath && (t.fsPath = this._fsPath, t._sep = Ks), this._formatted && (t.external = this._formatted), this.path && (t.path = this.path), this.scheme && (t.scheme = this.scheme), this.authority && (t.authority = this.authority), this.query && (t.query = this.query), this.fragment && (t.fragment = this.fragment), t;
  }
}
const ea = {
  58: "%3A",
  47: "%2F",
  63: "%3F",
  35: "%23",
  91: "%5B",
  93: "%5D",
  64: "%40",
  33: "%21",
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
function Ur(e, t, n) {
  let r, i = -1;
  for (let s = 0; s < e.length; s++) {
    const a = e.charCodeAt(s);
    if (a >= 97 && a <= 122 || a >= 65 && a <= 90 || a >= 48 && a <= 57 || a === 45 || a === 46 || a === 95 || a === 126 || t && a === 47 || n && a === 91 || n && a === 93 || n && a === 58)
      i !== -1 && (r += encodeURIComponent(e.substring(i, s)), i = -1), r !== void 0 && (r += e.charAt(s));
    else {
      r === void 0 && (r = e.substr(0, s));
      const o = ea[a];
      o !== void 0 ? (i !== -1 && (r += encodeURIComponent(e.substring(i, s)), i = -1), r += o) : i === -1 && (i = s);
    }
  }
  return i !== -1 && (r += encodeURIComponent(e.substring(i))), r !== void 0 ? r : e;
}
function ko(e) {
  let t;
  for (let n = 0; n < e.length; n++) {
    const r = e.charCodeAt(n);
    r === 35 || r === 63 ? (t === void 0 && (t = e.substr(0, n)), t += ea[r]) : t !== void 0 && (t += e[n]);
  }
  return t !== void 0 ? t : e;
}
function Qn(e, t) {
  let n;
  return e.authority && e.path.length > 1 && e.scheme === "file" ? n = `//${e.authority}${e.path}` : e.path.charCodeAt(0) === 47 && (e.path.charCodeAt(1) >= 65 && e.path.charCodeAt(1) <= 90 || e.path.charCodeAt(1) >= 97 && e.path.charCodeAt(1) <= 122) && e.path.charCodeAt(2) === 58 ? t ? n = e.path.substr(1) : n = e.path[1].toLowerCase() + e.path.substr(2) : n = e.path, Vt && (n = n.replace(/\//g, "\\")), n;
}
function Zn(e, t) {
  const n = t ? ko : Ur;
  let r = "", { scheme: i, authority: s, path: a, query: o, fragment: l } = e;
  if (i && (r += i, r += ":"), (s || i === "file") && (r += Ce, r += Ce), s) {
    let u = s.indexOf("@");
    if (u !== -1) {
      const h = s.substr(0, u);
      s = s.substr(u + 1), u = h.lastIndexOf(":"), u === -1 ? r += n(h, !1, !1) : (r += n(h.substr(0, u), !1, !1), r += ":", r += n(h.substr(u + 1), !1, !0)), r += "@";
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
  return o && (r += "?", r += n(o, !1, !1)), l && (r += "#", r += t ? l : Ur(l, !1, !1)), r;
}
function ta(e) {
  try {
    return decodeURIComponent(e);
  } catch {
    return e.length > 3 ? e.substr(0, 3) + ta(e.substr(3)) : e;
  }
}
const $r = /(%[0-9A-Za-z][0-9A-Za-z])+/g;
function Xt(e) {
  return e.match($r) ? e.replace($r, (t) => ta(t)) : e;
}
let De = class tt {
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
    return t === this.lineNumber && n === this.column ? this : new tt(t, n);
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
    return tt.equals(this, t);
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
    return tt.isBefore(this, t);
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
    return tt.isBeforeOrEqual(this, t);
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
    return new tt(this.lineNumber, this.column);
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
    return new tt(t.lineNumber, t.column);
  }
  /**
   * Test if `obj` is an `IPosition`.
   */
  static isIPosition(t) {
    return t && typeof t.lineNumber == "number" && typeof t.column == "number";
  }
}, pe = class te {
  constructor(t, n, r, i) {
    t > r || t === r && n > i ? (this.startLineNumber = r, this.startColumn = i, this.endLineNumber = t, this.endColumn = n) : (this.startLineNumber = t, this.startColumn = n, this.endLineNumber = r, this.endColumn = i);
  }
  /**
   * Test if this range is empty.
   */
  isEmpty() {
    return te.isEmpty(this);
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
    return te.containsPosition(this, t);
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
    return te.containsRange(this, t);
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
    return te.strictContainsRange(this, t);
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
    return te.plusRange(this, t);
  }
  /**
   * A reunion of the two ranges.
   * The smallest position will be used as the start point, and the largest one as the end point.
   */
  static plusRange(t, n) {
    let r, i, s, a;
    return n.startLineNumber < t.startLineNumber ? (r = n.startLineNumber, i = n.startColumn) : n.startLineNumber === t.startLineNumber ? (r = n.startLineNumber, i = Math.min(n.startColumn, t.startColumn)) : (r = t.startLineNumber, i = t.startColumn), n.endLineNumber > t.endLineNumber ? (s = n.endLineNumber, a = n.endColumn) : n.endLineNumber === t.endLineNumber ? (s = n.endLineNumber, a = Math.max(n.endColumn, t.endColumn)) : (s = t.endLineNumber, a = t.endColumn), new te(r, i, s, a);
  }
  /**
   * A intersection of the two ranges.
   */
  intersectRanges(t) {
    return te.intersectRanges(this, t);
  }
  /**
   * A intersection of the two ranges.
   */
  static intersectRanges(t, n) {
    let r = t.startLineNumber, i = t.startColumn, s = t.endLineNumber, a = t.endColumn;
    const o = n.startLineNumber, l = n.startColumn, u = n.endLineNumber, h = n.endColumn;
    return r < o ? (r = o, i = l) : r === o && (i = Math.max(i, l)), s > u ? (s = u, a = h) : s === u && (a = Math.min(a, h)), r > s || r === s && i > a ? null : new te(r, i, s, a);
  }
  /**
   * Test if this range equals other.
   */
  equalsRange(t) {
    return te.equalsRange(this, t);
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
    return te.getEndPosition(this);
  }
  /**
   * Return the end position (which will be after or equal to the start position)
   */
  static getEndPosition(t) {
    return new De(t.endLineNumber, t.endColumn);
  }
  /**
   * Return the start position (which will be before or equal to the end position)
   */
  getStartPosition() {
    return te.getStartPosition(this);
  }
  /**
   * Return the start position (which will be before or equal to the end position)
   */
  static getStartPosition(t) {
    return new De(t.startLineNumber, t.startColumn);
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
    return new te(this.startLineNumber, this.startColumn, t, n);
  }
  /**
   * Create a new range using this range's end position, and using startLineNumber and startColumn as the start position.
   */
  setStartPosition(t, n) {
    return new te(t, n, this.endLineNumber, this.endColumn);
  }
  /**
   * Create a new empty range using this range's start position.
   */
  collapseToStart() {
    return te.collapseToStart(this);
  }
  /**
   * Create a new empty range using this range's start position.
   */
  static collapseToStart(t) {
    return new te(t.startLineNumber, t.startColumn, t.startLineNumber, t.startColumn);
  }
  /**
   * Create a new empty range using this range's end position.
   */
  collapseToEnd() {
    return te.collapseToEnd(this);
  }
  /**
   * Create a new empty range using this range's end position.
   */
  static collapseToEnd(t) {
    return new te(t.endLineNumber, t.endColumn, t.endLineNumber, t.endColumn);
  }
  /**
   * Moves the range by the given amount of lines.
   */
  delta(t) {
    return new te(this.startLineNumber + t, this.startColumn, this.endLineNumber + t, this.endColumn);
  }
  // ---
  static fromPositions(t, n = t) {
    return new te(t.lineNumber, t.column, n.lineNumber, n.column);
  }
  static lift(t) {
    return t ? new te(t.startLineNumber, t.startColumn, t.endLineNumber, t.endColumn) : null;
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
          const u = t.endLineNumber | 0, h = n.endLineNumber | 0;
          if (u === h) {
            const f = t.endColumn | 0, d = n.endColumn | 0;
            return f - d;
          }
          return u - h;
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
function Wr(e, t) {
  for (let n = e.length - 1; n >= 0; n--) {
    const r = e[n];
    if (t(r))
      return n;
  }
  return -1;
}
var fn;
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
})(fn || (fn = {}));
function Mn(e, t) {
  return (n, r) => t(e(n), e(r));
}
const Qt = (e, t) => e - t;
function Eo(e) {
  return (t, n) => -e(t, n);
}
function Hr(e) {
  return e < 0 ? 0 : e > 255 ? 255 : e | 0;
}
function ht(e) {
  return e < 0 ? 0 : e > 4294967295 ? 4294967295 : e | 0;
}
class Mo {
  constructor(t) {
    this.values = t, this.prefixSum = new Uint32Array(t.length), this.prefixSumValidIndex = new Int32Array(1), this.prefixSumValidIndex[0] = -1;
  }
  insertValues(t, n) {
    t = ht(t);
    const r = this.values, i = this.prefixSum, s = n.length;
    return s === 0 ? !1 : (this.values = new Uint32Array(r.length + s), this.values.set(r.subarray(0, t), 0), this.values.set(r.subarray(t), t + s), this.values.set(n, t), t - 1 < this.prefixSumValidIndex[0] && (this.prefixSumValidIndex[0] = t - 1), this.prefixSum = new Uint32Array(this.values.length), this.prefixSumValidIndex[0] >= 0 && this.prefixSum.set(i.subarray(0, this.prefixSumValidIndex[0] + 1)), !0);
  }
  setValue(t, n) {
    return t = ht(t), n = ht(n), this.values[t] === n ? !1 : (this.values[t] = n, t - 1 < this.prefixSumValidIndex[0] && (this.prefixSumValidIndex[0] = t - 1), !0);
  }
  removeValues(t, n) {
    t = ht(t), n = ht(n);
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
    return t < 0 ? 0 : (t = ht(t), this._getPrefixSum(t));
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
    return new To(i, t - a);
  }
}
class To {
  constructor(t, n) {
    this.index = t, this.remainder = n, this._prefixSumIndexOfResultBrand = void 0, this.index = t, this.remainder = n;
  }
}
class Po {
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
      this._acceptDeleteRange(r.range), this._acceptInsertText(new De(r.range.startLineNumber, r.range.startColumn), r.text);
    this._versionId = t.versionId, this._cachedTextValue = null;
  }
  _ensureLineStarts() {
    if (!this._lineStarts) {
      const t = this._eol.length, n = this._lines.length, r = new Uint32Array(n);
      for (let i = 0; i < n; i++)
        r[i] = this._lines[i].length + t;
      this._lineStarts = new Mo(r);
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
    const r = Ga(n);
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
const Fo = "`~!@#$%^&*()-=+[{]}\\|;:'\",.<>/?";
function Io(e = "") {
  let t = "(-?\\d*\\.\\d\\w*)|([^";
  for (const n of Fo)
    e.indexOf(n) >= 0 || (t += "\\" + n);
  return t += "\\s]+)", new RegExp(t, "g");
}
const na = Io();
function Vo(e) {
  let t = na;
  if (e && e instanceof RegExp)
    if (e.global)
      t = e;
    else {
      let n = "g";
      e.ignoreCase && (n += "i"), e.multiline && (n += "m"), e.unicode && (n += "u"), t = new RegExp(e.source, n);
    }
  return t.lastIndex = 0, t;
}
const ra = new ka();
ra.unshift({
  maxLen: 1e3,
  windowSize: 15,
  timeBudget: 150
});
function Lr(e, t, n, r, i) {
  if (i || (i = ln.first(ra)), n.length > i.maxLen) {
    let u = e - i.maxLen / 2;
    return u < 0 ? u = 0 : r += u, n = n.substring(u, e + i.maxLen / 2), Lr(e, t, n, r, i);
  }
  const s = Date.now(), a = e - 1 - r;
  let o = -1, l = null;
  for (let u = 1; !(Date.now() - s >= i.timeBudget); u++) {
    const h = a - i.windowSize * u;
    t.lastIndex = Math.max(0, h);
    const f = Do(t, n, a, o);
    if (!f && l || (l = f, h <= 0))
      break;
    o = h;
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
function Do(e, t, n, r) {
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
class Nr {
  constructor(t) {
    const n = Hr(t);
    this._defaultValue = n, this._asciiMap = Nr._createAsciiMap(n), this._map = /* @__PURE__ */ new Map();
  }
  static _createAsciiMap(t) {
    const n = new Uint8Array(256);
    return n.fill(t), n;
  }
  set(t, n) {
    const r = Hr(n);
    t >= 0 && t < 256 ? this._asciiMap[t] = r : this._map.set(t, r);
  }
  get(t) {
    return t >= 0 && t < 256 ? this._asciiMap[t] : this._map.get(t) || this._defaultValue;
  }
  clear() {
    this._asciiMap.fill(this._defaultValue), this._map.clear();
  }
}
class Oo {
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
class jo {
  constructor(t) {
    let n = 0, r = 0;
    for (let s = 0, a = t.length; s < a; s++) {
      const [o, l, u] = t[s];
      l > n && (n = l), o > r && (r = o), u > r && (r = u);
    }
    n++, r++;
    const i = new Oo(
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
let Tn = null;
function Bo() {
  return Tn === null && (Tn = new jo([
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
  ])), Tn;
}
let At = null;
function qo() {
  if (At === null) {
    At = new Nr(
      0
      /* CharacterClass.None */
    );
    const e = ` 	<>'"、。｡､，．：；‘〈「『〔（［｛｢｣｝］）〕』」〉’｀～…`;
    for (let n = 0; n < e.length; n++)
      At.set(
        e.charCodeAt(n),
        1
        /* CharacterClass.ForceTermination */
      );
    const t = ".,;:";
    for (let n = 0; n < t.length; n++)
      At.set(
        t.charCodeAt(n),
        2
        /* CharacterClass.CannotEndIn */
      );
  }
  return At;
}
class hn {
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
  static computeLinks(t, n = Bo()) {
    const r = qo(), i = [];
    for (let s = 1, a = t.getLineCount(); s <= a; s++) {
      const o = t.getLineContent(s), l = o.length;
      let u = 0, h = 0, f = 0, d = 1, g = !1, m = !1, p = !1, b = !1;
      for (; u < l; ) {
        let y = !1;
        const x = o.charCodeAt(u);
        if (d === 13) {
          let v;
          switch (x) {
            case 40:
              g = !0, v = 0;
              break;
            case 41:
              v = g ? 0 : 1;
              break;
            case 91:
              p = !0, m = !0, v = 0;
              break;
            case 93:
              p = !1, v = m ? 0 : 1;
              break;
            case 123:
              b = !0, v = 0;
              break;
            case 125:
              v = b ? 0 : 1;
              break;
            case 39:
            case 34:
            case 96:
              f === x ? v = 1 : f === 39 || f === 34 || f === 96 ? v = 0 : v = 1;
              break;
            case 42:
              v = f === 42 ? 1 : 0;
              break;
            case 124:
              v = f === 124 ? 1 : 0;
              break;
            case 32:
              v = p ? 0 : 1;
              break;
            default:
              v = r.get(x);
          }
          v === 1 && (i.push(hn._createLink(r, o, s, h, u)), y = !0);
        } else if (d === 12) {
          let v;
          x === 91 ? (m = !0, v = 0) : v = r.get(x), v === 1 ? y = !0 : d = 13;
        } else
          d = n.nextState(d, x), d === 0 && (y = !0);
        y && (d = 1, g = !1, m = !1, b = !1, h = u + 1, f = x), u++;
      }
      d === 13 && i.push(hn._createLink(r, o, s, h, l));
    }
    return i;
  }
}
function Uo(e) {
  return !e || typeof e.getLineCount != "function" || typeof e.getLineContent != "function" ? [] : hn.computeLinks(e);
}
class Yn {
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
Yn.INSTANCE = new Yn();
const ia = Object.freeze(function(e, t) {
  const n = setTimeout(e.bind(t), 0);
  return { dispose() {
    clearTimeout(n);
  } };
});
var dn;
(function(e) {
  function t(n) {
    return n === e.None || n === e.Cancelled || n instanceof sn ? !0 : !n || typeof n != "object" ? !1 : typeof n.isCancellationRequested == "boolean" && typeof n.onCancellationRequested == "function";
  }
  e.isCancellationToken = t, e.None = Object.freeze({
    isCancellationRequested: !1,
    onCancellationRequested: Wn.None
  }), e.Cancelled = Object.freeze({
    isCancellationRequested: !0,
    onCancellationRequested: ia
  });
})(dn || (dn = {}));
class sn {
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
    return this._isCancelled ? ia : (this._emitter || (this._emitter = new Ae()), this._emitter.event);
  }
  dispose() {
    this._emitter && (this._emitter.dispose(), this._emitter = null);
  }
}
class $o {
  constructor(t) {
    this._token = void 0, this._parentListener = void 0, this._parentListener = t && t.onCancellationRequested(this.cancel, this);
  }
  get token() {
    return this._token || (this._token = new sn()), this._token;
  }
  cancel() {
    this._token ? this._token instanceof sn && this._token.cancel() : this._token = dn.Cancelled;
  }
  dispose(t = !1) {
    var n;
    t && this.cancel(), (n = this._parentListener) === null || n === void 0 || n.dispose(), this._token ? this._token instanceof sn && this._token.dispose() : this._token = dn.None;
  }
}
class Ar {
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
const an = new Ar(), Kn = new Ar(), er = new Ar(), Wo = new Array(230), Ho = /* @__PURE__ */ Object.create(null), zo = /* @__PURE__ */ Object.create(null);
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
    const [s, a, o, l, u, h, f, d, g] = i;
    if (r[a] || (r[a] = !0, Ho[o] = a, zo[o.toLowerCase()] = a), !n[l]) {
      if (n[l] = !0, !u)
        throw new Error(`String representation missing for key code ${l} around scan code ${o}`);
      an.define(l, u), Kn.define(l, d || u), er.define(l, g || d || u);
    }
    h && (Wo[h] = l);
  }
})();
var zr;
(function(e) {
  function t(o) {
    return an.keyCodeToStr(o);
  }
  e.toString = t;
  function n(o) {
    return an.strToKeyCode(o);
  }
  e.fromString = n;
  function r(o) {
    return Kn.keyCodeToStr(o);
  }
  e.toUserSettingsUS = r;
  function i(o) {
    return er.keyCodeToStr(o);
  }
  e.toUserSettingsGeneral = i;
  function s(o) {
    return Kn.strToKeyCode(o) || er.strToKeyCode(o);
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
    return an.keyCodeToStr(o);
  }
  e.toElectronAccelerator = a;
})(zr || (zr = {}));
function Go(e, t) {
  const n = (t & 65535) << 16 >>> 0;
  return (e | n) >>> 0;
}
class be extends pe {
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
    return be.selectionsEqual(this, t);
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
    return this.getDirection() === 0 ? new be(this.startLineNumber, this.startColumn, t, n) : new be(t, n, this.startLineNumber, this.startColumn);
  }
  /**
   * Get the position at `positionLineNumber` and `positionColumn`.
   */
  getPosition() {
    return new De(this.positionLineNumber, this.positionColumn);
  }
  /**
   * Get the position at the start of the selection.
  */
  getSelectionStart() {
    return new De(this.selectionStartLineNumber, this.selectionStartColumn);
  }
  /**
   * Create a new selection with a different `selectionStartLineNumber` and `selectionStartColumn`.
   */
  setStartPosition(t, n) {
    return this.getDirection() === 0 ? new be(t, n, this.endLineNumber, this.endColumn) : new be(this.endLineNumber, this.endColumn, t, n);
  }
  // ----
  /**
   * Create a `Selection` from one or two positions
   */
  static fromPositions(t, n = t) {
    return new be(t.lineNumber, t.column, n.lineNumber, n.column);
  }
  /**
   * Creates a `Selection` from a range, given a direction.
   */
  static fromRange(t, n) {
    return n === 0 ? new be(t.startLineNumber, t.startColumn, t.endLineNumber, t.endColumn) : new be(t.endLineNumber, t.endColumn, t.startLineNumber, t.startColumn);
  }
  /**
   * Create a `Selection` from an `ISelection`.
   */
  static liftSelection(t) {
    return new be(t.selectionStartLineNumber, t.selectionStartColumn, t.positionLineNumber, t.positionColumn);
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
    return s === 0 ? new be(t, n, r, i) : new be(r, i, t, n);
  }
}
const Gr = /* @__PURE__ */ Object.create(null);
function c(e, t) {
  if (Fa(t)) {
    const n = Gr[t];
    if (n === void 0)
      throw new Error(`${e} references an unknown codicon: ${t}`);
    t = n;
  }
  return Gr[e] = t, { id: e };
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
  gitPullRequestLabel: c("git-pull-request-label", 60006),
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
  gitPullRequestMilestone: c("git-pull-request-milestone", 60192),
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
  gitPullRequestReviewer: c("git-pull-request-reviewer", 60310),
  groupByRefType: c("group-by-ref-type", 60311),
  ungroupByRefType: c("ungroup-by-ref-type", 60312),
  account: c("account", 60313),
  gitPullRequestAssignee: c("git-pull-request-assignee", 60313),
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
  mic: c("mic", 60434),
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
var tr = globalThis && globalThis.__awaiter || function(e, t, n, r) {
  function i(s) {
    return s instanceof n ? s : new n(function(a) {
      a(s);
    });
  }
  return new (n || (n = Promise))(function(s, a) {
    function o(h) {
      try {
        u(r.next(h));
      } catch (f) {
        a(f);
      }
    }
    function l(h) {
      try {
        u(r.throw(h));
      } catch (f) {
        a(f);
      }
    }
    function u(h) {
      h.done ? s(h.value) : i(h.value).then(o, l);
    }
    u((r = r.apply(e, t || [])).next());
  });
};
class Jo {
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
    return this._tokenizationSupports.set(t, n), this.handleChange([t]), Ft(() => {
      this._tokenizationSupports.get(t) === n && (this._tokenizationSupports.delete(t), this.handleChange([t]));
    });
  }
  get(t) {
    return this._tokenizationSupports.get(t) || null;
  }
  registerFactory(t, n) {
    var r;
    (r = this._factories.get(t)) === null || r === void 0 || r.dispose();
    const i = new Xo(this, t, n);
    return this._factories.set(t, i), Ft(() => {
      const s = this._factories.get(t);
      !s || s !== i || (this._factories.delete(t), s.dispose());
    });
  }
  getOrCreate(t) {
    return tr(this, void 0, void 0, function* () {
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
class Xo extends It {
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
    return tr(this, void 0, void 0, function* () {
      return this._resolvePromise || (this._resolvePromise = this._create()), this._resolvePromise;
    });
  }
  _create() {
    return tr(this, void 0, void 0, function* () {
      const t = yield this._factory.tokenizationSupport;
      this._isResolved = !0, t && !this._isDisposed && this._register(this._registry.register(this._languageId, t));
    });
  }
}
class Qo {
  constructor(t, n, r) {
    this.offset = t, this.type = n, this.language = r, this._tokenBrand = void 0;
  }
  toString() {
    return "(" + this.offset + ", " + this.type + ")";
  }
}
var Jr;
(function(e) {
  const t = /* @__PURE__ */ new Map();
  t.set(0, O.symbolMethod), t.set(1, O.symbolFunction), t.set(2, O.symbolConstructor), t.set(3, O.symbolField), t.set(4, O.symbolVariable), t.set(5, O.symbolClass), t.set(6, O.symbolStruct), t.set(7, O.symbolInterface), t.set(8, O.symbolModule), t.set(9, O.symbolProperty), t.set(10, O.symbolEvent), t.set(11, O.symbolOperator), t.set(12, O.symbolUnit), t.set(13, O.symbolValue), t.set(15, O.symbolEnum), t.set(14, O.symbolConstant), t.set(15, O.symbolEnum), t.set(16, O.symbolEnumMember), t.set(17, O.symbolKeyword), t.set(27, O.symbolSnippet), t.set(18, O.symbolText), t.set(19, O.symbolColor), t.set(20, O.symbolFile), t.set(21, O.symbolReference), t.set(22, O.symbolCustomColor), t.set(23, O.symbolFolder), t.set(24, O.symbolTypeParameter), t.set(25, O.account), t.set(26, O.issues);
  function n(s) {
    let a = t.get(s);
    return a || (console.info("No codicon found for CompletionItemKind " + s), a = O.symbolProperty), a;
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
})(Jr || (Jr = {}));
var Xr;
(function(e) {
  e[e.Automatic = 0] = "Automatic", e[e.Explicit = 1] = "Explicit";
})(Xr || (Xr = {}));
var Qr;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.TriggerCharacter = 2] = "TriggerCharacter", e[e.ContentChange = 3] = "ContentChange";
})(Qr || (Qr = {}));
var Zr;
(function(e) {
  e[e.Text = 0] = "Text", e[e.Read = 1] = "Read", e[e.Write = 2] = "Write";
})(Zr || (Zr = {}));
Y("Array", "array"), Y("Boolean", "boolean"), Y("Class", "class"), Y("Constant", "constant"), Y("Constructor", "constructor"), Y("Enum", "enumeration"), Y("EnumMember", "enumeration member"), Y("Event", "event"), Y("Field", "field"), Y("File", "file"), Y("Function", "function"), Y("Interface", "interface"), Y("Key", "key"), Y("Method", "method"), Y("Module", "module"), Y("Namespace", "namespace"), Y("Null", "null"), Y("Number", "number"), Y("Object", "object"), Y("Operator", "operator"), Y("Package", "package"), Y("Property", "property"), Y("String", "string"), Y("Struct", "struct"), Y("TypeParameter", "type parameter"), Y("Variable", "variable");
var Yr;
(function(e) {
  const t = /* @__PURE__ */ new Map();
  t.set(0, O.symbolFile), t.set(1, O.symbolModule), t.set(2, O.symbolNamespace), t.set(3, O.symbolPackage), t.set(4, O.symbolClass), t.set(5, O.symbolMethod), t.set(6, O.symbolProperty), t.set(7, O.symbolField), t.set(8, O.symbolConstructor), t.set(9, O.symbolEnum), t.set(10, O.symbolInterface), t.set(11, O.symbolFunction), t.set(12, O.symbolVariable), t.set(13, O.symbolConstant), t.set(14, O.symbolString), t.set(15, O.symbolNumber), t.set(16, O.symbolBoolean), t.set(17, O.symbolArray), t.set(18, O.symbolObject), t.set(19, O.symbolKey), t.set(20, O.symbolNull), t.set(21, O.symbolEnumMember), t.set(22, O.symbolStruct), t.set(23, O.symbolEvent), t.set(24, O.symbolOperator), t.set(25, O.symbolTypeParameter);
  function n(r) {
    let i = t.get(r);
    return i || (console.info("No codicon found for SymbolKind " + r), i = O.symbolProperty), i;
  }
  e.toIcon = n;
})(Yr || (Yr = {}));
var Kr;
(function(e) {
  function t(n) {
    return !n || typeof n != "object" ? !1 : typeof n.id == "string" && typeof n.title == "string";
  }
  e.is = t;
})(Kr || (Kr = {}));
var ei;
(function(e) {
  e[e.Type = 1] = "Type", e[e.Parameter = 2] = "Parameter";
})(ei || (ei = {}));
new Jo();
var ti;
(function(e) {
  e[e.Unknown = 0] = "Unknown", e[e.Disabled = 1] = "Disabled", e[e.Enabled = 2] = "Enabled";
})(ti || (ti = {}));
var ni;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.Auto = 2] = "Auto";
})(ni || (ni = {}));
var ri;
(function(e) {
  e[e.None = 0] = "None", e[e.KeepWhitespace = 1] = "KeepWhitespace", e[e.InsertAsSnippet = 4] = "InsertAsSnippet";
})(ri || (ri = {}));
var ii;
(function(e) {
  e[e.Method = 0] = "Method", e[e.Function = 1] = "Function", e[e.Constructor = 2] = "Constructor", e[e.Field = 3] = "Field", e[e.Variable = 4] = "Variable", e[e.Class = 5] = "Class", e[e.Struct = 6] = "Struct", e[e.Interface = 7] = "Interface", e[e.Module = 8] = "Module", e[e.Property = 9] = "Property", e[e.Event = 10] = "Event", e[e.Operator = 11] = "Operator", e[e.Unit = 12] = "Unit", e[e.Value = 13] = "Value", e[e.Constant = 14] = "Constant", e[e.Enum = 15] = "Enum", e[e.EnumMember = 16] = "EnumMember", e[e.Keyword = 17] = "Keyword", e[e.Text = 18] = "Text", e[e.Color = 19] = "Color", e[e.File = 20] = "File", e[e.Reference = 21] = "Reference", e[e.Customcolor = 22] = "Customcolor", e[e.Folder = 23] = "Folder", e[e.TypeParameter = 24] = "TypeParameter", e[e.User = 25] = "User", e[e.Issue = 26] = "Issue", e[e.Snippet = 27] = "Snippet";
})(ii || (ii = {}));
var si;
(function(e) {
  e[e.Deprecated = 1] = "Deprecated";
})(si || (si = {}));
var ai;
(function(e) {
  e[e.Invoke = 0] = "Invoke", e[e.TriggerCharacter = 1] = "TriggerCharacter", e[e.TriggerForIncompleteCompletions = 2] = "TriggerForIncompleteCompletions";
})(ai || (ai = {}));
var oi;
(function(e) {
  e[e.EXACT = 0] = "EXACT", e[e.ABOVE = 1] = "ABOVE", e[e.BELOW = 2] = "BELOW";
})(oi || (oi = {}));
var li;
(function(e) {
  e[e.NotSet = 0] = "NotSet", e[e.ContentFlush = 1] = "ContentFlush", e[e.RecoverFromMarkers = 2] = "RecoverFromMarkers", e[e.Explicit = 3] = "Explicit", e[e.Paste = 4] = "Paste", e[e.Undo = 5] = "Undo", e[e.Redo = 6] = "Redo";
})(li || (li = {}));
var ui;
(function(e) {
  e[e.LF = 1] = "LF", e[e.CRLF = 2] = "CRLF";
})(ui || (ui = {}));
var ci;
(function(e) {
  e[e.Text = 0] = "Text", e[e.Read = 1] = "Read", e[e.Write = 2] = "Write";
})(ci || (ci = {}));
var fi;
(function(e) {
  e[e.None = 0] = "None", e[e.Keep = 1] = "Keep", e[e.Brackets = 2] = "Brackets", e[e.Advanced = 3] = "Advanced", e[e.Full = 4] = "Full";
})(fi || (fi = {}));
var hi;
(function(e) {
  e[e.acceptSuggestionOnCommitCharacter = 0] = "acceptSuggestionOnCommitCharacter", e[e.acceptSuggestionOnEnter = 1] = "acceptSuggestionOnEnter", e[e.accessibilitySupport = 2] = "accessibilitySupport", e[e.accessibilityPageSize = 3] = "accessibilityPageSize", e[e.ariaLabel = 4] = "ariaLabel", e[e.ariaRequired = 5] = "ariaRequired", e[e.autoClosingBrackets = 6] = "autoClosingBrackets", e[e.screenReaderAnnounceInlineSuggestion = 7] = "screenReaderAnnounceInlineSuggestion", e[e.autoClosingDelete = 8] = "autoClosingDelete", e[e.autoClosingOvertype = 9] = "autoClosingOvertype", e[e.autoClosingQuotes = 10] = "autoClosingQuotes", e[e.autoIndent = 11] = "autoIndent", e[e.automaticLayout = 12] = "automaticLayout", e[e.autoSurround = 13] = "autoSurround", e[e.bracketPairColorization = 14] = "bracketPairColorization", e[e.guides = 15] = "guides", e[e.codeLens = 16] = "codeLens", e[e.codeLensFontFamily = 17] = "codeLensFontFamily", e[e.codeLensFontSize = 18] = "codeLensFontSize", e[e.colorDecorators = 19] = "colorDecorators", e[e.colorDecoratorsLimit = 20] = "colorDecoratorsLimit", e[e.columnSelection = 21] = "columnSelection", e[e.comments = 22] = "comments", e[e.contextmenu = 23] = "contextmenu", e[e.copyWithSyntaxHighlighting = 24] = "copyWithSyntaxHighlighting", e[e.cursorBlinking = 25] = "cursorBlinking", e[e.cursorSmoothCaretAnimation = 26] = "cursorSmoothCaretAnimation", e[e.cursorStyle = 27] = "cursorStyle", e[e.cursorSurroundingLines = 28] = "cursorSurroundingLines", e[e.cursorSurroundingLinesStyle = 29] = "cursorSurroundingLinesStyle", e[e.cursorWidth = 30] = "cursorWidth", e[e.disableLayerHinting = 31] = "disableLayerHinting", e[e.disableMonospaceOptimizations = 32] = "disableMonospaceOptimizations", e[e.domReadOnly = 33] = "domReadOnly", e[e.dragAndDrop = 34] = "dragAndDrop", e[e.dropIntoEditor = 35] = "dropIntoEditor", e[e.emptySelectionClipboard = 36] = "emptySelectionClipboard", e[e.experimentalWhitespaceRendering = 37] = "experimentalWhitespaceRendering", e[e.extraEditorClassName = 38] = "extraEditorClassName", e[e.fastScrollSensitivity = 39] = "fastScrollSensitivity", e[e.find = 40] = "find", e[e.fixedOverflowWidgets = 41] = "fixedOverflowWidgets", e[e.folding = 42] = "folding", e[e.foldingStrategy = 43] = "foldingStrategy", e[e.foldingHighlight = 44] = "foldingHighlight", e[e.foldingImportsByDefault = 45] = "foldingImportsByDefault", e[e.foldingMaximumRegions = 46] = "foldingMaximumRegions", e[e.unfoldOnClickAfterEndOfLine = 47] = "unfoldOnClickAfterEndOfLine", e[e.fontFamily = 48] = "fontFamily", e[e.fontInfo = 49] = "fontInfo", e[e.fontLigatures = 50] = "fontLigatures", e[e.fontSize = 51] = "fontSize", e[e.fontWeight = 52] = "fontWeight", e[e.fontVariations = 53] = "fontVariations", e[e.formatOnPaste = 54] = "formatOnPaste", e[e.formatOnType = 55] = "formatOnType", e[e.glyphMargin = 56] = "glyphMargin", e[e.gotoLocation = 57] = "gotoLocation", e[e.hideCursorInOverviewRuler = 58] = "hideCursorInOverviewRuler", e[e.hover = 59] = "hover", e[e.inDiffEditor = 60] = "inDiffEditor", e[e.inlineSuggest = 61] = "inlineSuggest", e[e.letterSpacing = 62] = "letterSpacing", e[e.lightbulb = 63] = "lightbulb", e[e.lineDecorationsWidth = 64] = "lineDecorationsWidth", e[e.lineHeight = 65] = "lineHeight", e[e.lineNumbers = 66] = "lineNumbers", e[e.lineNumbersMinChars = 67] = "lineNumbersMinChars", e[e.linkedEditing = 68] = "linkedEditing", e[e.links = 69] = "links", e[e.matchBrackets = 70] = "matchBrackets", e[e.minimap = 71] = "minimap", e[e.mouseStyle = 72] = "mouseStyle", e[e.mouseWheelScrollSensitivity = 73] = "mouseWheelScrollSensitivity", e[e.mouseWheelZoom = 74] = "mouseWheelZoom", e[e.multiCursorMergeOverlapping = 75] = "multiCursorMergeOverlapping", e[e.multiCursorModifier = 76] = "multiCursorModifier", e[e.multiCursorPaste = 77] = "multiCursorPaste", e[e.multiCursorLimit = 78] = "multiCursorLimit", e[e.occurrencesHighlight = 79] = "occurrencesHighlight", e[e.overviewRulerBorder = 80] = "overviewRulerBorder", e[e.overviewRulerLanes = 81] = "overviewRulerLanes", e[e.padding = 82] = "padding", e[e.pasteAs = 83] = "pasteAs", e[e.parameterHints = 84] = "parameterHints", e[e.peekWidgetDefaultFocus = 85] = "peekWidgetDefaultFocus", e[e.definitionLinkOpensInPeek = 86] = "definitionLinkOpensInPeek", e[e.quickSuggestions = 87] = "quickSuggestions", e[e.quickSuggestionsDelay = 88] = "quickSuggestionsDelay", e[e.readOnly = 89] = "readOnly", e[e.readOnlyMessage = 90] = "readOnlyMessage", e[e.renameOnType = 91] = "renameOnType", e[e.renderControlCharacters = 92] = "renderControlCharacters", e[e.renderFinalNewline = 93] = "renderFinalNewline", e[e.renderLineHighlight = 94] = "renderLineHighlight", e[e.renderLineHighlightOnlyWhenFocus = 95] = "renderLineHighlightOnlyWhenFocus", e[e.renderValidationDecorations = 96] = "renderValidationDecorations", e[e.renderWhitespace = 97] = "renderWhitespace", e[e.revealHorizontalRightPadding = 98] = "revealHorizontalRightPadding", e[e.roundedSelection = 99] = "roundedSelection", e[e.rulers = 100] = "rulers", e[e.scrollbar = 101] = "scrollbar", e[e.scrollBeyondLastColumn = 102] = "scrollBeyondLastColumn", e[e.scrollBeyondLastLine = 103] = "scrollBeyondLastLine", e[e.scrollPredominantAxis = 104] = "scrollPredominantAxis", e[e.selectionClipboard = 105] = "selectionClipboard", e[e.selectionHighlight = 106] = "selectionHighlight", e[e.selectOnLineNumbers = 107] = "selectOnLineNumbers", e[e.showFoldingControls = 108] = "showFoldingControls", e[e.showUnused = 109] = "showUnused", e[e.snippetSuggestions = 110] = "snippetSuggestions", e[e.smartSelect = 111] = "smartSelect", e[e.smoothScrolling = 112] = "smoothScrolling", e[e.stickyScroll = 113] = "stickyScroll", e[e.stickyTabStops = 114] = "stickyTabStops", e[e.stopRenderingLineAfter = 115] = "stopRenderingLineAfter", e[e.suggest = 116] = "suggest", e[e.suggestFontSize = 117] = "suggestFontSize", e[e.suggestLineHeight = 118] = "suggestLineHeight", e[e.suggestOnTriggerCharacters = 119] = "suggestOnTriggerCharacters", e[e.suggestSelection = 120] = "suggestSelection", e[e.tabCompletion = 121] = "tabCompletion", e[e.tabIndex = 122] = "tabIndex", e[e.unicodeHighlighting = 123] = "unicodeHighlighting", e[e.unusualLineTerminators = 124] = "unusualLineTerminators", e[e.useShadowDOM = 125] = "useShadowDOM", e[e.useTabStops = 126] = "useTabStops", e[e.wordBreak = 127] = "wordBreak", e[e.wordSeparators = 128] = "wordSeparators", e[e.wordWrap = 129] = "wordWrap", e[e.wordWrapBreakAfterCharacters = 130] = "wordWrapBreakAfterCharacters", e[e.wordWrapBreakBeforeCharacters = 131] = "wordWrapBreakBeforeCharacters", e[e.wordWrapColumn = 132] = "wordWrapColumn", e[e.wordWrapOverride1 = 133] = "wordWrapOverride1", e[e.wordWrapOverride2 = 134] = "wordWrapOverride2", e[e.wrappingIndent = 135] = "wrappingIndent", e[e.wrappingStrategy = 136] = "wrappingStrategy", e[e.showDeprecated = 137] = "showDeprecated", e[e.inlayHints = 138] = "inlayHints", e[e.editorClassName = 139] = "editorClassName", e[e.pixelRatio = 140] = "pixelRatio", e[e.tabFocusMode = 141] = "tabFocusMode", e[e.layoutInfo = 142] = "layoutInfo", e[e.wrappingInfo = 143] = "wrappingInfo", e[e.defaultColorDecorators = 144] = "defaultColorDecorators", e[e.colorDecoratorsActivatedOn = 145] = "colorDecoratorsActivatedOn", e[e.inlineCompletionsAccessibilityVerbose = 146] = "inlineCompletionsAccessibilityVerbose";
})(hi || (hi = {}));
var di;
(function(e) {
  e[e.TextDefined = 0] = "TextDefined", e[e.LF = 1] = "LF", e[e.CRLF = 2] = "CRLF";
})(di || (di = {}));
var gi;
(function(e) {
  e[e.LF = 0] = "LF", e[e.CRLF = 1] = "CRLF";
})(gi || (gi = {}));
var mi;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Right = 2] = "Right";
})(mi || (mi = {}));
var pi;
(function(e) {
  e[e.None = 0] = "None", e[e.Indent = 1] = "Indent", e[e.IndentOutdent = 2] = "IndentOutdent", e[e.Outdent = 3] = "Outdent";
})(pi || (pi = {}));
var vi;
(function(e) {
  e[e.Both = 0] = "Both", e[e.Right = 1] = "Right", e[e.Left = 2] = "Left", e[e.None = 3] = "None";
})(vi || (vi = {}));
var bi;
(function(e) {
  e[e.Type = 1] = "Type", e[e.Parameter = 2] = "Parameter";
})(bi || (bi = {}));
var yi;
(function(e) {
  e[e.Automatic = 0] = "Automatic", e[e.Explicit = 1] = "Explicit";
})(yi || (yi = {}));
var nr;
(function(e) {
  e[e.DependsOnKbLayout = -1] = "DependsOnKbLayout", e[e.Unknown = 0] = "Unknown", e[e.Backspace = 1] = "Backspace", e[e.Tab = 2] = "Tab", e[e.Enter = 3] = "Enter", e[e.Shift = 4] = "Shift", e[e.Ctrl = 5] = "Ctrl", e[e.Alt = 6] = "Alt", e[e.PauseBreak = 7] = "PauseBreak", e[e.CapsLock = 8] = "CapsLock", e[e.Escape = 9] = "Escape", e[e.Space = 10] = "Space", e[e.PageUp = 11] = "PageUp", e[e.PageDown = 12] = "PageDown", e[e.End = 13] = "End", e[e.Home = 14] = "Home", e[e.LeftArrow = 15] = "LeftArrow", e[e.UpArrow = 16] = "UpArrow", e[e.RightArrow = 17] = "RightArrow", e[e.DownArrow = 18] = "DownArrow", e[e.Insert = 19] = "Insert", e[e.Delete = 20] = "Delete", e[e.Digit0 = 21] = "Digit0", e[e.Digit1 = 22] = "Digit1", e[e.Digit2 = 23] = "Digit2", e[e.Digit3 = 24] = "Digit3", e[e.Digit4 = 25] = "Digit4", e[e.Digit5 = 26] = "Digit5", e[e.Digit6 = 27] = "Digit6", e[e.Digit7 = 28] = "Digit7", e[e.Digit8 = 29] = "Digit8", e[e.Digit9 = 30] = "Digit9", e[e.KeyA = 31] = "KeyA", e[e.KeyB = 32] = "KeyB", e[e.KeyC = 33] = "KeyC", e[e.KeyD = 34] = "KeyD", e[e.KeyE = 35] = "KeyE", e[e.KeyF = 36] = "KeyF", e[e.KeyG = 37] = "KeyG", e[e.KeyH = 38] = "KeyH", e[e.KeyI = 39] = "KeyI", e[e.KeyJ = 40] = "KeyJ", e[e.KeyK = 41] = "KeyK", e[e.KeyL = 42] = "KeyL", e[e.KeyM = 43] = "KeyM", e[e.KeyN = 44] = "KeyN", e[e.KeyO = 45] = "KeyO", e[e.KeyP = 46] = "KeyP", e[e.KeyQ = 47] = "KeyQ", e[e.KeyR = 48] = "KeyR", e[e.KeyS = 49] = "KeyS", e[e.KeyT = 50] = "KeyT", e[e.KeyU = 51] = "KeyU", e[e.KeyV = 52] = "KeyV", e[e.KeyW = 53] = "KeyW", e[e.KeyX = 54] = "KeyX", e[e.KeyY = 55] = "KeyY", e[e.KeyZ = 56] = "KeyZ", e[e.Meta = 57] = "Meta", e[e.ContextMenu = 58] = "ContextMenu", e[e.F1 = 59] = "F1", e[e.F2 = 60] = "F2", e[e.F3 = 61] = "F3", e[e.F4 = 62] = "F4", e[e.F5 = 63] = "F5", e[e.F6 = 64] = "F6", e[e.F7 = 65] = "F7", e[e.F8 = 66] = "F8", e[e.F9 = 67] = "F9", e[e.F10 = 68] = "F10", e[e.F11 = 69] = "F11", e[e.F12 = 70] = "F12", e[e.F13 = 71] = "F13", e[e.F14 = 72] = "F14", e[e.F15 = 73] = "F15", e[e.F16 = 74] = "F16", e[e.F17 = 75] = "F17", e[e.F18 = 76] = "F18", e[e.F19 = 77] = "F19", e[e.F20 = 78] = "F20", e[e.F21 = 79] = "F21", e[e.F22 = 80] = "F22", e[e.F23 = 81] = "F23", e[e.F24 = 82] = "F24", e[e.NumLock = 83] = "NumLock", e[e.ScrollLock = 84] = "ScrollLock", e[e.Semicolon = 85] = "Semicolon", e[e.Equal = 86] = "Equal", e[e.Comma = 87] = "Comma", e[e.Minus = 88] = "Minus", e[e.Period = 89] = "Period", e[e.Slash = 90] = "Slash", e[e.Backquote = 91] = "Backquote", e[e.BracketLeft = 92] = "BracketLeft", e[e.Backslash = 93] = "Backslash", e[e.BracketRight = 94] = "BracketRight", e[e.Quote = 95] = "Quote", e[e.OEM_8 = 96] = "OEM_8", e[e.IntlBackslash = 97] = "IntlBackslash", e[e.Numpad0 = 98] = "Numpad0", e[e.Numpad1 = 99] = "Numpad1", e[e.Numpad2 = 100] = "Numpad2", e[e.Numpad3 = 101] = "Numpad3", e[e.Numpad4 = 102] = "Numpad4", e[e.Numpad5 = 103] = "Numpad5", e[e.Numpad6 = 104] = "Numpad6", e[e.Numpad7 = 105] = "Numpad7", e[e.Numpad8 = 106] = "Numpad8", e[e.Numpad9 = 107] = "Numpad9", e[e.NumpadMultiply = 108] = "NumpadMultiply", e[e.NumpadAdd = 109] = "NumpadAdd", e[e.NUMPAD_SEPARATOR = 110] = "NUMPAD_SEPARATOR", e[e.NumpadSubtract = 111] = "NumpadSubtract", e[e.NumpadDecimal = 112] = "NumpadDecimal", e[e.NumpadDivide = 113] = "NumpadDivide", e[e.KEY_IN_COMPOSITION = 114] = "KEY_IN_COMPOSITION", e[e.ABNT_C1 = 115] = "ABNT_C1", e[e.ABNT_C2 = 116] = "ABNT_C2", e[e.AudioVolumeMute = 117] = "AudioVolumeMute", e[e.AudioVolumeUp = 118] = "AudioVolumeUp", e[e.AudioVolumeDown = 119] = "AudioVolumeDown", e[e.BrowserSearch = 120] = "BrowserSearch", e[e.BrowserHome = 121] = "BrowserHome", e[e.BrowserBack = 122] = "BrowserBack", e[e.BrowserForward = 123] = "BrowserForward", e[e.MediaTrackNext = 124] = "MediaTrackNext", e[e.MediaTrackPrevious = 125] = "MediaTrackPrevious", e[e.MediaStop = 126] = "MediaStop", e[e.MediaPlayPause = 127] = "MediaPlayPause", e[e.LaunchMediaPlayer = 128] = "LaunchMediaPlayer", e[e.LaunchMail = 129] = "LaunchMail", e[e.LaunchApp2 = 130] = "LaunchApp2", e[e.Clear = 131] = "Clear", e[e.MAX_VALUE = 132] = "MAX_VALUE";
})(nr || (nr = {}));
var rr;
(function(e) {
  e[e.Hint = 1] = "Hint", e[e.Info = 2] = "Info", e[e.Warning = 4] = "Warning", e[e.Error = 8] = "Error";
})(rr || (rr = {}));
var ir;
(function(e) {
  e[e.Unnecessary = 1] = "Unnecessary", e[e.Deprecated = 2] = "Deprecated";
})(ir || (ir = {}));
var xi;
(function(e) {
  e[e.Inline = 1] = "Inline", e[e.Gutter = 2] = "Gutter";
})(xi || (xi = {}));
var wi;
(function(e) {
  e[e.UNKNOWN = 0] = "UNKNOWN", e[e.TEXTAREA = 1] = "TEXTAREA", e[e.GUTTER_GLYPH_MARGIN = 2] = "GUTTER_GLYPH_MARGIN", e[e.GUTTER_LINE_NUMBERS = 3] = "GUTTER_LINE_NUMBERS", e[e.GUTTER_LINE_DECORATIONS = 4] = "GUTTER_LINE_DECORATIONS", e[e.GUTTER_VIEW_ZONE = 5] = "GUTTER_VIEW_ZONE", e[e.CONTENT_TEXT = 6] = "CONTENT_TEXT", e[e.CONTENT_EMPTY = 7] = "CONTENT_EMPTY", e[e.CONTENT_VIEW_ZONE = 8] = "CONTENT_VIEW_ZONE", e[e.CONTENT_WIDGET = 9] = "CONTENT_WIDGET", e[e.OVERVIEW_RULER = 10] = "OVERVIEW_RULER", e[e.SCROLLBAR = 11] = "SCROLLBAR", e[e.OVERLAY_WIDGET = 12] = "OVERLAY_WIDGET", e[e.OUTSIDE_EDITOR = 13] = "OUTSIDE_EDITOR";
})(wi || (wi = {}));
var _i;
(function(e) {
  e[e.TOP_RIGHT_CORNER = 0] = "TOP_RIGHT_CORNER", e[e.BOTTOM_RIGHT_CORNER = 1] = "BOTTOM_RIGHT_CORNER", e[e.TOP_CENTER = 2] = "TOP_CENTER";
})(_i || (_i = {}));
var Si;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Center = 2] = "Center", e[e.Right = 4] = "Right", e[e.Full = 7] = "Full";
})(Si || (Si = {}));
var Li;
(function(e) {
  e[e.Left = 0] = "Left", e[e.Right = 1] = "Right", e[e.None = 2] = "None", e[e.LeftOfInjectedText = 3] = "LeftOfInjectedText", e[e.RightOfInjectedText = 4] = "RightOfInjectedText";
})(Li || (Li = {}));
var Ni;
(function(e) {
  e[e.Off = 0] = "Off", e[e.On = 1] = "On", e[e.Relative = 2] = "Relative", e[e.Interval = 3] = "Interval", e[e.Custom = 4] = "Custom";
})(Ni || (Ni = {}));
var Ai;
(function(e) {
  e[e.None = 0] = "None", e[e.Text = 1] = "Text", e[e.Blocks = 2] = "Blocks";
})(Ai || (Ai = {}));
var Ci;
(function(e) {
  e[e.Smooth = 0] = "Smooth", e[e.Immediate = 1] = "Immediate";
})(Ci || (Ci = {}));
var ki;
(function(e) {
  e[e.Auto = 1] = "Auto", e[e.Hidden = 2] = "Hidden", e[e.Visible = 3] = "Visible";
})(ki || (ki = {}));
var sr;
(function(e) {
  e[e.LTR = 0] = "LTR", e[e.RTL = 1] = "RTL";
})(sr || (sr = {}));
var Ri;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.TriggerCharacter = 2] = "TriggerCharacter", e[e.ContentChange = 3] = "ContentChange";
})(Ri || (Ri = {}));
var Ei;
(function(e) {
  e[e.File = 0] = "File", e[e.Module = 1] = "Module", e[e.Namespace = 2] = "Namespace", e[e.Package = 3] = "Package", e[e.Class = 4] = "Class", e[e.Method = 5] = "Method", e[e.Property = 6] = "Property", e[e.Field = 7] = "Field", e[e.Constructor = 8] = "Constructor", e[e.Enum = 9] = "Enum", e[e.Interface = 10] = "Interface", e[e.Function = 11] = "Function", e[e.Variable = 12] = "Variable", e[e.Constant = 13] = "Constant", e[e.String = 14] = "String", e[e.Number = 15] = "Number", e[e.Boolean = 16] = "Boolean", e[e.Array = 17] = "Array", e[e.Object = 18] = "Object", e[e.Key = 19] = "Key", e[e.Null = 20] = "Null", e[e.EnumMember = 21] = "EnumMember", e[e.Struct = 22] = "Struct", e[e.Event = 23] = "Event", e[e.Operator = 24] = "Operator", e[e.TypeParameter = 25] = "TypeParameter";
})(Ei || (Ei = {}));
var Mi;
(function(e) {
  e[e.Deprecated = 1] = "Deprecated";
})(Mi || (Mi = {}));
var Ti;
(function(e) {
  e[e.Hidden = 0] = "Hidden", e[e.Blink = 1] = "Blink", e[e.Smooth = 2] = "Smooth", e[e.Phase = 3] = "Phase", e[e.Expand = 4] = "Expand", e[e.Solid = 5] = "Solid";
})(Ti || (Ti = {}));
var Pi;
(function(e) {
  e[e.Line = 1] = "Line", e[e.Block = 2] = "Block", e[e.Underline = 3] = "Underline", e[e.LineThin = 4] = "LineThin", e[e.BlockOutline = 5] = "BlockOutline", e[e.UnderlineThin = 6] = "UnderlineThin";
})(Pi || (Pi = {}));
var Fi;
(function(e) {
  e[e.AlwaysGrowsWhenTypingAtEdges = 0] = "AlwaysGrowsWhenTypingAtEdges", e[e.NeverGrowsWhenTypingAtEdges = 1] = "NeverGrowsWhenTypingAtEdges", e[e.GrowsOnlyWhenTypingBefore = 2] = "GrowsOnlyWhenTypingBefore", e[e.GrowsOnlyWhenTypingAfter = 3] = "GrowsOnlyWhenTypingAfter";
})(Fi || (Fi = {}));
var Ii;
(function(e) {
  e[e.None = 0] = "None", e[e.Same = 1] = "Same", e[e.Indent = 2] = "Indent", e[e.DeepIndent = 3] = "DeepIndent";
})(Ii || (Ii = {}));
class zt {
  static chord(t, n) {
    return Go(t, n);
  }
}
zt.CtrlCmd = 2048;
zt.Shift = 1024;
zt.Alt = 512;
zt.WinCtrl = 256;
function Zo() {
  return {
    editor: void 0,
    languages: void 0,
    CancellationTokenSource: $o,
    Emitter: Ae,
    KeyCode: nr,
    KeyMod: zt,
    Position: De,
    Range: pe,
    Selection: be,
    SelectionDirection: sr,
    MarkerSeverity: rr,
    MarkerTag: ir,
    Uri: Sr,
    Token: Qo
  };
}
var Vi;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Center = 2] = "Center", e[e.Right = 4] = "Right", e[e.Full = 7] = "Full";
})(Vi || (Vi = {}));
var Di;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Right = 2] = "Right";
})(Di || (Di = {}));
var Oi;
(function(e) {
  e[e.Inline = 1] = "Inline", e[e.Gutter = 2] = "Gutter";
})(Oi || (Oi = {}));
var ji;
(function(e) {
  e[e.Both = 0] = "Both", e[e.Right = 1] = "Right", e[e.Left = 2] = "Left", e[e.None = 3] = "None";
})(ji || (ji = {}));
function Yo(e, t, n, r, i) {
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
function Ko(e, t, n, r, i) {
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
function el(e, t, n, r, i) {
  return Yo(e, t, n, r, i) && Ko(e, t, n, r, i);
}
class tl {
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
          Ya(t, n, this._searchRegex.lastIndex) > 65535 ? this._searchRegex.lastIndex += 2 : this._searchRegex.lastIndex += 1;
          continue;
        }
        return null;
      }
      if (this._prevMatchStartIndex = i, this._prevMatchLength = s, !this._wordSeparators || el(this._wordSeparators, t, n, i, s))
        return r;
    } while (r);
    return null;
  }
}
function nl(e, t = "Unreachable") {
  throw new Error(t);
}
function gn(e) {
  if (!e()) {
    debugger;
    e(), Ws(new je("Assertion Failed"));
  }
}
function sa(e, t) {
  let n = 0;
  for (; n < e.length - 1; ) {
    const r = e[n], i = e[n + 1];
    if (!t(r, i))
      return !1;
    n++;
  }
  return !0;
}
class rl {
  static computeUnicodeHighlights(t, n, r) {
    const i = r ? r.startLineNumber : 1, s = r ? r.endLineNumber : t.getLineCount(), a = new Bi(n), o = a.getCandidateCodePoints();
    let l;
    o === "allNonBasicAscii" ? l = new RegExp("[^\\t\\n\\r\\x20-\\x7E]", "g") : l = new RegExp(`${il(Array.from(o))}`, "g");
    const u = new tl(null, l), h = [];
    let f = !1, d, g = 0, m = 0, p = 0;
    e:
      for (let b = i, y = s; b <= y; b++) {
        const x = t.getLineContent(b), v = x.length;
        u.reset(0);
        do
          if (d = u.next(x), d) {
            let C = d.index, L = d.index + d[0].length;
            if (C > 0) {
              const R = x.charCodeAt(C - 1);
              Jn(R) && C--;
            }
            if (L + 1 < v) {
              const R = x.charCodeAt(L - 1);
              Jn(R) && L++;
            }
            const _ = x.substring(C, L);
            let S = Lr(C + 1, na, x, 0);
            S && S.endColumn <= C + 1 && (S = null);
            const A = a.shouldHighlightNonBasicASCII(_, S ? S.word : null);
            if (A !== 0) {
              A === 3 ? g++ : A === 2 ? m++ : A === 1 ? p++ : nl();
              const R = 1e3;
              if (h.length >= R) {
                f = !0;
                break e;
              }
              h.push(new pe(b, C + 1, b, L + 1));
            }
          }
        while (d);
      }
    return {
      ranges: h,
      hasMore: f,
      ambiguousCharacterCount: g,
      invisibleCharacterCount: m,
      nonBasicAsciiCharacterCount: p
    };
  }
  static computeUnicodeHighlightReason(t, n) {
    const r = new Bi(n);
    switch (r.shouldHighlightNonBasicASCII(t, null)) {
      case 0:
        return null;
      case 2:
        return {
          kind: 1
          /* UnicodeHighlighterReasonKind.Invisible */
        };
      case 3: {
        const s = t.codePointAt(0), a = r.ambiguousCharacters.getPrimaryConfusable(s), o = st.getLocales().filter((l) => !st.getInstance(/* @__PURE__ */ new Set([...n.allowedLocales, l])).isAmbiguous(s));
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
function il(e, t) {
  return `[${za(e.map((r) => String.fromCodePoint(r)).join(""))}]`;
}
class Bi {
  constructor(t) {
    this.options = t, this.allowedCodePoints = new Set(t.allowedCodePoints), this.ambiguousCharacters = st.getInstance(new Set(t.allowedLocales));
  }
  getCandidateCodePoints() {
    if (this.options.nonBasicASCII)
      return "allNonBasicAscii";
    const t = /* @__PURE__ */ new Set();
    if (this.options.invisibleCharacters)
      for (const n of Ze.codePoints)
        qi(String.fromCodePoint(n)) || t.add(n);
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
        const o = a.codePointAt(0), l = eo(a);
        i = i || l, !l && !this.ambiguousCharacters.isAmbiguous(o) && !Ze.isInvisibleCharacter(o) && (s = !0);
      }
    return (
      /* Don't allow mixing weird looking characters with ASCII */
      !i && /* Is there an obviously weird looking character? */
      s ? 0 : this.options.invisibleCharacters && !qi(t) && Ze.isInvisibleCharacter(r) ? 2 : this.options.ambiguousCharacters && this.ambiguousCharacters.isAmbiguous(r) ? 3 : 0
    );
  }
}
function qi(e) {
  return e === " " || e === `
` || e === "	";
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
      throw new je(`Invalid range: ${this.toString()}`);
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
      throw new je(`Invalid clipping range: ${this.toString()}`);
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
      throw new je(`Invalid clipping range: ${this.toString()}`);
    return t < this.start ? this.endExclusive - (this.start - t) % this.length : t >= this.endExclusive ? this.start + (t - this.start) % this.length : t;
  }
}
class J {
  static fromRange(t) {
    return new J(t.startLineNumber, t.endLineNumber);
  }
  static subtract(t, n) {
    return n ? t.startLineNumber < n.startLineNumber && n.endLineNumberExclusive < t.endLineNumberExclusive ? [
      new J(t.startLineNumber, n.startLineNumber),
      new J(n.endLineNumberExclusive, t.endLineNumberExclusive)
    ] : n.startLineNumber <= t.startLineNumber && t.endLineNumberExclusive <= n.endLineNumberExclusive ? [] : n.endLineNumberExclusive < t.endLineNumberExclusive ? [new J(Math.max(n.endLineNumberExclusive, t.startLineNumber), t.endLineNumberExclusive)] : [new J(t.startLineNumber, Math.min(n.startLineNumber, t.endLineNumberExclusive))] : [t];
  }
  /**
   * @param lineRanges An array of sorted line ranges.
   */
  static joinMany(t) {
    if (t.length === 0)
      return [];
    let n = t[0];
    for (let r = 1; r < t.length; r++)
      n = this.join(n, t[r]);
    return n;
  }
  /**
   * @param lineRanges1 Must be sorted.
   * @param lineRanges2 Must be sorted.
   */
  static join(t, n) {
    if (t.length === 0)
      return n;
    if (n.length === 0)
      return t;
    const r = [];
    let i = 0, s = 0, a = null;
    for (; i < t.length || s < n.length; ) {
      let o = null;
      if (i < t.length && s < n.length) {
        const l = t[i], u = n[s];
        l.startLineNumber < u.startLineNumber ? (o = l, i++) : (o = u, s++);
      } else
        i < t.length ? (o = t[i], i++) : (o = n[s], s++);
      a === null ? a = o : a.endLineNumberExclusive >= o.startLineNumber ? a = new J(a.startLineNumber, Math.max(a.endLineNumberExclusive, o.endLineNumberExclusive)) : (r.push(a), a = o);
    }
    return a !== null && r.push(a), r;
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
      throw new je(`startLineNumber ${t} cannot be after endLineNumberExclusive ${n}`);
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
    return this.isEmpty ? null : new pe(this.startLineNumber, 1, this.endLineNumberExclusive - 1, Number.MAX_SAFE_INTEGER);
  }
  toExclusiveRange() {
    return new pe(this.startLineNumber, 1, this.endLineNumberExclusive, 1);
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
class on {
  constructor(t, n, r) {
    this.changes = t, this.moves = n, this.hitTimeout = r;
  }
}
class Ie {
  static inverse(t, n, r) {
    const i = [];
    let s = 1, a = 1;
    for (const l of t) {
      const u = new Ie(new J(s, l.originalRange.startLineNumber), new J(a, l.modifiedRange.startLineNumber), void 0);
      u.modifiedRange.isEmpty || i.push(u), s = l.originalRange.endLineNumberExclusive, a = l.modifiedRange.endLineNumberExclusive;
    }
    const o = new Ie(new J(s, n + 1), new J(a, r + 1), void 0);
    return o.modifiedRange.isEmpty || i.push(o), i;
  }
  constructor(t, n, r) {
    this.originalRange = t, this.modifiedRange = n, this.innerChanges = r;
  }
  toString() {
    return `{${this.originalRange.toString()}->${this.modifiedRange.toString()}}`;
  }
  get changedLineCount() {
    return Math.max(this.originalRange.length, this.modifiedRange.length);
  }
  flip() {
    var t;
    return new Ie(this.modifiedRange, this.originalRange, (t = this.innerChanges) === null || t === void 0 ? void 0 : t.map((n) => n.flip()));
  }
}
class Dt {
  constructor(t, n) {
    this.originalRange = t, this.modifiedRange = n;
  }
  toString() {
    return `{${this.originalRange.toString()}->${this.modifiedRange.toString()}}`;
  }
  flip() {
    return new Dt(this.modifiedRange, this.originalRange);
  }
}
class Ot {
  constructor(t, n) {
    this.original = t, this.modified = n;
  }
  toString() {
    return `{${this.original.toString()}->${this.modified.toString()}}`;
  }
  flip() {
    return new Ot(this.modified, this.original);
  }
  join(t) {
    return new Ot(this.original.join(t.original), this.modified.join(t.modified));
  }
}
class Cr {
  constructor(t, n) {
    this.lineRangeMapping = t, this.changes = n;
  }
  flip() {
    return new Cr(this.lineRangeMapping.flip(), this.changes.map((t) => t.flip()));
  }
}
const sl = 3;
class al {
  computeDiff(t, n, r) {
    var i;
    const a = new ul(t, n, {
      maxComputationTime: r.maxComputationTimeMs,
      shouldIgnoreTrimWhitespace: r.ignoreTrimWhitespace,
      shouldComputeCharChanges: !0,
      shouldMakePrettyDiff: !0,
      shouldPostProcessCharChanges: !0
    }).computeDiff(), o = [];
    let l = null;
    for (const u of a.changes) {
      let h;
      u.originalEndLineNumber === 0 ? h = new J(u.originalStartLineNumber + 1, u.originalStartLineNumber + 1) : h = new J(u.originalStartLineNumber, u.originalEndLineNumber + 1);
      let f;
      u.modifiedEndLineNumber === 0 ? f = new J(u.modifiedStartLineNumber + 1, u.modifiedStartLineNumber + 1) : f = new J(u.modifiedStartLineNumber, u.modifiedEndLineNumber + 1);
      let d = new Ie(h, f, (i = u.charChanges) === null || i === void 0 ? void 0 : i.map((g) => new Dt(new pe(g.originalStartLineNumber, g.originalStartColumn, g.originalEndLineNumber, g.originalEndColumn), new pe(g.modifiedStartLineNumber, g.modifiedStartColumn, g.modifiedEndLineNumber, g.modifiedEndColumn))));
      l && (l.modifiedRange.endLineNumberExclusive === d.modifiedRange.startLineNumber || l.originalRange.endLineNumberExclusive === d.originalRange.startLineNumber) && (d = new Ie(l.originalRange.join(d.originalRange), l.modifiedRange.join(d.modifiedRange), l.innerChanges && d.innerChanges ? l.innerChanges.concat(d.innerChanges) : void 0), o.pop()), o.push(d), l = d;
    }
    return gn(() => sa(o, (u, h) => h.originalRange.startLineNumber - u.originalRange.endLineNumberExclusive === h.modifiedRange.startLineNumber - u.modifiedRange.endLineNumberExclusive && // There has to be an unchanged line in between (otherwise both diffs should have been joined)
    u.originalRange.endLineNumberExclusive < h.originalRange.startLineNumber && u.modifiedRange.endLineNumberExclusive < h.modifiedRange.startLineNumber)), new on(o, [], a.quitEarly);
  }
}
function aa(e, t, n, r) {
  return new Qe(e, t, n).ComputeDiff(r);
}
let Ui = class {
  constructor(t) {
    const n = [], r = [];
    for (let i = 0, s = t.length; i < s; i++)
      n[i] = ar(t[i], 1), r[i] = or(t[i], 1);
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
      const u = this.lines[l], h = t ? this._startColumns[l] : 1, f = t ? this._endColumns[l] : u.length + 1;
      for (let d = h; d < f; d++)
        i[o] = u.charCodeAt(d - 1), s[o] = l + 1, a[o] = d, o++;
      !t && l < r && (i[o] = 10, s[o] = l + 1, a[o] = u.length + 1, o++);
    }
    return new ol(i, s, a);
  }
};
class ol {
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
class bt {
  constructor(t, n, r, i, s, a, o, l) {
    this.originalStartLineNumber = t, this.originalStartColumn = n, this.originalEndLineNumber = r, this.originalEndColumn = i, this.modifiedStartLineNumber = s, this.modifiedStartColumn = a, this.modifiedEndLineNumber = o, this.modifiedEndColumn = l;
  }
  static createFromDiffChange(t, n, r) {
    const i = n.getStartLineNumber(t.originalStart), s = n.getStartColumn(t.originalStart), a = n.getEndLineNumber(t.originalStart + t.originalLength - 1), o = n.getEndColumn(t.originalStart + t.originalLength - 1), l = r.getStartLineNumber(t.modifiedStart), u = r.getStartColumn(t.modifiedStart), h = r.getEndLineNumber(t.modifiedStart + t.modifiedLength - 1), f = r.getEndColumn(t.modifiedStart + t.modifiedLength - 1);
    return new bt(i, s, a, o, l, u, h, f);
  }
}
function ll(e) {
  if (e.length <= 1)
    return e;
  const t = [e[0]];
  let n = t[0];
  for (let r = 1, i = e.length; r < i; r++) {
    const s = e[r], a = s.originalStart - (n.originalStart + n.originalLength), o = s.modifiedStart - (n.modifiedStart + n.modifiedLength);
    Math.min(a, o) < sl ? (n.originalLength = s.originalStart + s.originalLength - n.originalStart, n.modifiedLength = s.modifiedStart + s.modifiedLength - n.modifiedStart) : (t.push(s), n = s);
  }
  return t;
}
class Mt {
  constructor(t, n, r, i, s) {
    this.originalStartLineNumber = t, this.originalEndLineNumber = n, this.modifiedStartLineNumber = r, this.modifiedEndLineNumber = i, this.charChanges = s;
  }
  static createFromDiffResult(t, n, r, i, s, a, o) {
    let l, u, h, f, d;
    if (n.originalLength === 0 ? (l = r.getStartLineNumber(n.originalStart) - 1, u = 0) : (l = r.getStartLineNumber(n.originalStart), u = r.getEndLineNumber(n.originalStart + n.originalLength - 1)), n.modifiedLength === 0 ? (h = i.getStartLineNumber(n.modifiedStart) - 1, f = 0) : (h = i.getStartLineNumber(n.modifiedStart), f = i.getEndLineNumber(n.modifiedStart + n.modifiedLength - 1)), a && n.originalLength > 0 && n.originalLength < 20 && n.modifiedLength > 0 && n.modifiedLength < 20 && s()) {
      const g = r.createCharSequence(t, n.originalStart, n.originalStart + n.originalLength - 1), m = i.createCharSequence(t, n.modifiedStart, n.modifiedStart + n.modifiedLength - 1);
      if (g.getElements().length > 0 && m.getElements().length > 0) {
        let p = aa(g, m, s, !0).changes;
        o && (p = ll(p)), d = [];
        for (let b = 0, y = p.length; b < y; b++)
          d.push(bt.createFromDiffChange(p[b], g, m));
      }
    }
    return new Mt(l, u, h, f, d);
  }
}
class ul {
  constructor(t, n, r) {
    this.shouldComputeCharChanges = r.shouldComputeCharChanges, this.shouldPostProcessCharChanges = r.shouldPostProcessCharChanges, this.shouldIgnoreTrimWhitespace = r.shouldIgnoreTrimWhitespace, this.shouldMakePrettyDiff = r.shouldMakePrettyDiff, this.originalLines = t, this.modifiedLines = n, this.original = new Ui(t), this.modified = new Ui(n), this.continueLineDiff = $i(r.maxComputationTime), this.continueCharDiff = $i(r.maxComputationTime === 0 ? 0 : Math.min(r.maxComputationTime, 5e3));
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
    const t = aa(this.original, this.modified, this.continueLineDiff, this.shouldMakePrettyDiff), n = t.changes, r = t.quitEarly;
    if (this.shouldIgnoreTrimWhitespace) {
      const o = [];
      for (let l = 0, u = n.length; l < u; l++)
        o.push(Mt.createFromDiffResult(this.shouldIgnoreTrimWhitespace, n[l], this.original, this.modified, this.continueCharDiff, this.shouldComputeCharChanges, this.shouldPostProcessCharChanges));
      return {
        quitEarly: r,
        changes: o
      };
    }
    const i = [];
    let s = 0, a = 0;
    for (let o = -1, l = n.length; o < l; o++) {
      const u = o + 1 < l ? n[o + 1] : null, h = u ? u.originalStart : this.originalLines.length, f = u ? u.modifiedStart : this.modifiedLines.length;
      for (; s < h && a < f; ) {
        const d = this.originalLines[s], g = this.modifiedLines[a];
        if (d !== g) {
          {
            let m = ar(d, 1), p = ar(g, 1);
            for (; m > 1 && p > 1; ) {
              const b = d.charCodeAt(m - 2), y = g.charCodeAt(p - 2);
              if (b !== y)
                break;
              m--, p--;
            }
            (m > 1 || p > 1) && this._pushTrimWhitespaceCharChange(i, s + 1, 1, m, a + 1, 1, p);
          }
          {
            let m = or(d, 1), p = or(g, 1);
            const b = d.length + 1, y = g.length + 1;
            for (; m < b && p < y; ) {
              const x = d.charCodeAt(m - 1), v = d.charCodeAt(p - 1);
              if (x !== v)
                break;
              m++, p++;
            }
            (m < b || p < y) && this._pushTrimWhitespaceCharChange(i, s + 1, m, b, a + 1, p, y);
          }
        }
        s++, a++;
      }
      u && (i.push(Mt.createFromDiffResult(this.shouldIgnoreTrimWhitespace, u, this.original, this.modified, this.continueCharDiff, this.shouldComputeCharChanges, this.shouldPostProcessCharChanges)), s += u.originalLength, a += u.modifiedLength);
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
    this.shouldComputeCharChanges && (l = [new bt(n, r, n, i, s, a, s, o)]), t.push(new Mt(n, n, s, s, l));
  }
  _mergeTrimWhitespaceCharChange(t, n, r, i, s, a, o) {
    const l = t.length;
    if (l === 0)
      return !1;
    const u = t[l - 1];
    return u.originalEndLineNumber === 0 || u.modifiedEndLineNumber === 0 ? !1 : u.originalEndLineNumber === n && u.modifiedEndLineNumber === s ? (this.shouldComputeCharChanges && u.charChanges && u.charChanges.push(new bt(n, r, n, i, s, a, s, o)), !0) : u.originalEndLineNumber + 1 === n && u.modifiedEndLineNumber + 1 === s ? (u.originalEndLineNumber = n, u.modifiedEndLineNumber = s, this.shouldComputeCharChanges && u.charChanges && u.charChanges.push(new bt(n, r, n, i, s, a, s, o)), !0) : !1;
  }
}
function ar(e, t) {
  const n = Ja(e);
  return n === -1 ? t : n + 1;
}
function or(e, t) {
  const n = Xa(e);
  return n === -1 ? t : n + 2;
}
function $i(e) {
  if (e === 0)
    return () => !0;
  const t = Date.now();
  return () => Date.now() - t < e;
}
class cl {
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
class Be {
  static trivial(t, n) {
    return new Be([new fe(new z(0, t.length), new z(0, n.length))], !1);
  }
  static trivialTimedOut(t, n) {
    return new Be([new fe(new z(0, t.length), new z(0, n.length))], !0);
  }
  constructor(t, n) {
    this.diffs = t, this.hitTimeout = n;
  }
}
class fe {
  constructor(t, n) {
    this.seq1Range = t, this.seq2Range = n;
  }
  reverse() {
    return new fe(this.seq2Range, this.seq1Range);
  }
  toString() {
    return `${this.seq1Range} <-> ${this.seq2Range}`;
  }
  join(t) {
    return new fe(this.seq1Range.join(t.seq1Range), this.seq2Range.join(t.seq2Range));
  }
  delta(t) {
    return t === 0 ? this : new fe(this.seq1Range.delta(t), this.seq2Range.delta(t));
  }
}
class jt {
  isValid() {
    return !0;
  }
}
jt.instance = new jt();
class fl {
  constructor(t) {
    if (this.timeout = t, this.startTime = Date.now(), this.valid = !0, t <= 0)
      throw new je("timeout must be positive");
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
class Pn {
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
class hl {
  compute(t, n, r = jt.instance, i) {
    if (t.length === 0 || n.length === 0)
      return Be.trivial(t, n);
    const s = new Pn(t.length, n.length), a = new Pn(t.length, n.length), o = new Pn(t.length, n.length);
    for (let m = 0; m < t.length; m++)
      for (let p = 0; p < n.length; p++) {
        if (!r.isValid())
          return Be.trivialTimedOut(t, n);
        const b = m === 0 ? 0 : s.get(m - 1, p), y = p === 0 ? 0 : s.get(m, p - 1);
        let x;
        t.getElement(m) === n.getElement(p) ? (m === 0 || p === 0 ? x = 0 : x = s.get(m - 1, p - 1), m > 0 && p > 0 && a.get(m - 1, p - 1) === 3 && (x += o.get(m - 1, p - 1)), x += i ? i(m, p) : 1) : x = -1;
        const v = Math.max(b, y, x);
        if (v === x) {
          const C = m > 0 && p > 0 ? o.get(m - 1, p - 1) : 0;
          o.set(m, p, C + 1), a.set(m, p, 3);
        } else
          v === b ? (o.set(m, p, 0), a.set(m, p, 1)) : v === y && (o.set(m, p, 0), a.set(m, p, 2));
        s.set(m, p, v);
      }
    const l = [];
    let u = t.length, h = n.length;
    function f(m, p) {
      (m + 1 !== u || p + 1 !== h) && l.push(new fe(new z(m + 1, u), new z(p + 1, h))), u = m, h = p;
    }
    let d = t.length - 1, g = n.length - 1;
    for (; d >= 0 && g >= 0; )
      a.get(d, g) === 3 ? (f(d, g), d--, g--) : a.get(d, g) === 1 ? d-- : g--;
    return f(-1, -1), l.reverse(), new Be(l, !1);
  }
}
function Wi(e, t, n) {
  let r = n;
  return r = pl(e, t, r), r = vl(e, t, r), r;
}
function dl(e, t, n) {
  const r = [];
  for (const i of n) {
    const s = r[r.length - 1];
    if (!s) {
      r.push(i);
      continue;
    }
    i.seq1Range.start - s.seq1Range.endExclusive <= 2 || i.seq2Range.start - s.seq2Range.endExclusive <= 2 ? r[r.length - 1] = new fe(s.seq1Range.join(i.seq1Range), s.seq2Range.join(i.seq2Range)) : r.push(i);
  }
  return r;
}
function gl(e, t, n) {
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
        const p = new z(h.seq1Range.endExclusive, u.seq1Range.start);
        return e.getText(p).replace(/\s/g, "").length <= 4 && (g.seq1Range.length + g.seq2Range.length > 5 || m.seq1Range.length + m.seq2Range.length > 5);
      };
      var a = f;
      const u = r[l], h = o[o.length - 1];
      f(h, u) ? (s = !0, o[o.length - 1] = o[o.length - 1].join(u)) : o.push(u);
    }
    r = o;
  } while (i++ < 10 && s);
  return r;
}
function ml(e, t, n) {
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
        const p = new z(h.seq1Range.endExclusive, u.seq1Range.start);
        if (e.countLinesIn(p) > 5 || p.length > 500)
          return !1;
        const y = e.getText(p).trim();
        if (y.length > 20 || y.split(/\r\n|\r|\n/).length > 1)
          return !1;
        const x = e.countLinesIn(g.seq1Range), v = g.seq1Range.length, C = t.countLinesIn(g.seq2Range), L = g.seq2Range.length, _ = e.countLinesIn(m.seq1Range), S = m.seq1Range.length, A = t.countLinesIn(m.seq2Range), R = m.seq2Range.length, k = 2 * 40 + 50;
        function w(N) {
          return Math.min(N, k);
        }
        return Math.pow(Math.pow(w(x * 40 + v), 1.5) + Math.pow(w(C * 40 + L), 1.5), 1.5) + Math.pow(Math.pow(w(_ * 40 + S), 1.5) + Math.pow(w(A * 40 + R), 1.5), 1.5) > Math.pow(Math.pow(k, 1.5), 1.5) * 1.3;
      };
      var a = f;
      const u = r[l], h = o[o.length - 1];
      f(h, u) ? (s = !0, o[o.length - 1] = o[o.length - 1].join(u)) : o.push(u);
    }
    r = o;
  } while (i++ < 10 && s);
  for (let o = 0; o < r.length; o++) {
    const l = r[o];
    let u = l.seq1Range, h = l.seq2Range;
    const f = e.extendToFullLines(l.seq1Range), d = e.getText(new z(f.start, l.seq1Range.start));
    d.length > 0 && d.trim().length <= 3 && l.seq1Range.length + l.seq2Range.length > 100 && (u = l.seq1Range.deltaStart(-d.length), h = l.seq2Range.deltaStart(-d.length));
    const g = e.getText(new z(l.seq1Range.endExclusive, f.endExclusive));
    g.length > 0 && g.trim().length <= 3 && l.seq1Range.length + l.seq2Range.length > 150 && (u = u.deltaEnd(g.length), h = h.deltaEnd(g.length)), r[o] = new fe(u, h);
  }
  return r;
}
function pl(e, t, n) {
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
        r[r.length - 1] = new fe(new z(a.seq1Range.start, o.seq1Range.endExclusive - l), new z(a.seq2Range.start, o.seq2Range.endExclusive - l));
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
      for (u = 0; u < l && !(e.getElement(o.seq1Range.start + u) !== e.getElement(o.seq1Range.endExclusive + u) || t.getElement(o.seq2Range.start + u) !== t.getElement(o.seq2Range.endExclusive + u)); u++)
        ;
      if (u === l) {
        r[s + 1] = new fe(new z(o.seq1Range.start + l, a.seq1Range.endExclusive), new z(o.seq2Range.start + l, a.seq2Range.endExclusive));
        continue;
      }
      u > 0 && (o = o.delta(u));
    }
    i.push(o);
  }
  return r.length > 0 && i.push(r[r.length - 1]), i;
}
function vl(e, t, n) {
  if (!e.getBoundaryScore || !t.getBoundaryScore)
    return n;
  for (let r = 0; r < n.length; r++) {
    const i = r > 0 ? n[r - 1] : void 0, s = n[r], a = r + 1 < n.length ? n[r + 1] : void 0, o = new z(i ? i.seq1Range.start + 1 : 0, a ? a.seq1Range.endExclusive - 1 : e.length), l = new z(i ? i.seq2Range.start + 1 : 0, a ? a.seq2Range.endExclusive - 1 : t.length);
    s.seq1Range.isEmpty ? n[r] = Hi(s, e, t, o, l) : s.seq2Range.isEmpty && (n[r] = Hi(s.reverse(), t, e, l, o).reverse());
  }
  return n;
}
function Hi(e, t, n, r, i) {
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
  for (let h = -a; h <= o; h++) {
    const f = e.seq2Range.start + h, d = e.seq2Range.endExclusive + h, g = e.seq1Range.start + h, m = t.getBoundaryScore(g) + n.getBoundaryScore(f) + n.getBoundaryScore(d);
    m > u && (u = m, l = h);
  }
  return e.delta(l);
}
class bl {
  compute(t, n, r = jt.instance) {
    if (t.length === 0 || n.length === 0)
      return Be.trivial(t, n);
    function i(g, m) {
      for (; g < t.length && m < n.length && t.getElement(g) === n.getElement(m); )
        g++, m++;
      return g;
    }
    let s = 0;
    const a = new yl();
    a.set(0, i(0, 0));
    const o = new xl();
    o.set(0, a.get(0) === 0 ? null : new zi(null, 0, 0, a.get(0)));
    let l = 0;
    e:
      for (; ; ) {
        if (s++, !r.isValid())
          return Be.trivialTimedOut(t, n);
        const g = -Math.min(s, n.length + s % 2), m = Math.min(s, t.length + s % 2);
        for (l = g; l <= m; l += 2) {
          const p = l === m ? -1 : a.get(l + 1), b = l === g ? -1 : a.get(l - 1) + 1, y = Math.min(Math.max(p, b), t.length), x = y - l;
          if (y > t.length || x > n.length)
            continue;
          const v = i(y, x);
          a.set(l, v);
          const C = y === p ? o.get(l + 1) : o.get(l - 1);
          if (o.set(l, v !== y ? new zi(C, y, x, v - y) : C), a.get(l) === t.length && a.get(l) - l === n.length)
            break e;
        }
      }
    let u = o.get(l);
    const h = [];
    let f = t.length, d = n.length;
    for (; ; ) {
      const g = u ? u.x + u.length : 0, m = u ? u.y + u.length : 0;
      if ((g !== f || m !== d) && h.push(new fe(new z(g, f), new z(m, d))), !u)
        break;
      f = u.x, d = u.y, u = u.prev;
    }
    return h.reverse(), new Be(h, !1);
  }
}
class zi {
  constructor(t, n, r, i) {
    this.prev = t, this.x = n, this.y = r, this.length = i;
  }
}
class yl {
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
class xl {
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
class wl {
  constructor() {
    this.dynamicProgrammingDiffing = new hl(), this.myersDiffingAlgorithm = new bl();
  }
  computeDiff(t, n, r) {
    if (t.length <= 1 && Ro(t, n, (L, _) => L === _))
      return new on([], [], !1);
    if (t.length === 1 && t[0].length === 0 || n.length === 1 && n[0].length === 0)
      return new on([
        new Ie(new J(1, t.length + 1), new J(1, n.length + 1), [
          new Dt(new pe(1, 1, t.length, t[0].length + 1), new pe(1, 1, n.length, n[0].length + 1))
        ])
      ], [], !1);
    const i = r.maxComputationTimeMs === 0 ? jt.instance : new fl(r.maxComputationTimeMs), s = !r.ignoreTrimWhitespace, a = /* @__PURE__ */ new Map();
    function o(L) {
      let _ = a.get(L);
      return _ === void 0 && (_ = a.size, a.set(L, _)), _;
    }
    const l = t.map((L) => o(L.trim())), u = n.map((L) => o(L.trim())), h = new Qi(l, t), f = new Qi(u, n), d = (() => h.length + f.length < 1700 ? this.dynamicProgrammingDiffing.compute(h, f, i, (L, _) => t[L] === n[_] ? n[_].length === 0 ? 0.1 : 1 + Math.log(1 + n[_].length) : 0.99) : this.myersDiffingAlgorithm.compute(h, f))();
    let g = d.diffs, m = d.hitTimeout;
    g = Wi(h, f, g), g = gl(h, f, g);
    const p = [], b = (L) => {
      if (s)
        for (let _ = 0; _ < L; _++) {
          const S = y + _, A = x + _;
          if (t[S] !== n[A]) {
            const R = this.refineDiff(t, n, new fe(new z(S, S + 1), new z(A, A + 1)), i, s);
            for (const k of R.mappings)
              p.push(k);
            R.hitTimeout && (m = !0);
          }
        }
    };
    let y = 0, x = 0;
    for (const L of g) {
      gn(() => L.seq1Range.start - y === L.seq2Range.start - x);
      const _ = L.seq1Range.start - y;
      b(_), y = L.seq1Range.endExclusive, x = L.seq2Range.endExclusive;
      const S = this.refineDiff(t, n, L, i, s);
      S.hitTimeout && (m = !0);
      for (const A of S.mappings)
        p.push(A);
    }
    b(t.length - y);
    const v = Xi(p, t, n);
    let C = [];
    return r.computeMoves && (C = this.computeMoves(v, t, n, l, u, i, s)), gn(() => {
      function L(S, A) {
        if (S.lineNumber < 1 || S.lineNumber > A.length)
          return !1;
        const R = A[S.lineNumber - 1];
        return !(S.column < 1 || S.column > R.length + 1);
      }
      function _(S, A) {
        return !(S.startLineNumber < 1 || S.startLineNumber > A.length + 1 || S.endLineNumberExclusive < 1 || S.endLineNumberExclusive > A.length + 1);
      }
      for (const S of v) {
        if (!S.innerChanges)
          return !1;
        for (const A of S.innerChanges)
          if (!(L(A.modifiedRange.getStartPosition(), n) && L(A.modifiedRange.getEndPosition(), n) && L(A.originalRange.getStartPosition(), t) && L(A.originalRange.getEndPosition(), t)))
            return !1;
        if (!_(S.modifiedRange, n) || !_(S.originalRange, t))
          return !1;
      }
      return !0;
    }), new on(v, C, m);
  }
  computeMoves(t, n, r, i, s, a, o) {
    const l = [], u = t.filter((v) => v.modifiedRange.isEmpty && v.originalRange.length >= 3).map((v) => new ns(v.originalRange, n, v)), h = new Set(t.filter((v) => v.originalRange.isEmpty && v.modifiedRange.length >= 3).map((v) => new ns(v.modifiedRange, r, v))), f = /* @__PURE__ */ new Set();
    for (const v of u) {
      let C = -1, L;
      for (const _ of h) {
        const S = v.computeSimilarity(_);
        S > C && (C = S, L = _);
      }
      if (C > 0.9 && L && (h.delete(L), l.push(new Ot(v.range, L.range)), f.add(v.source), f.add(L.source)), !a.isValid())
        return [];
    }
    const d = new cl();
    for (const v of t)
      if (!f.has(v))
        for (let C = v.originalRange.startLineNumber; C < v.originalRange.endLineNumberExclusive - 2; C++) {
          const L = `${i[C - 1]}:${i[C + 1 - 1]}:${i[C + 2 - 1]}`;
          d.add(L, { range: new J(C, C + 3) });
        }
    const g = [];
    t.sort(Mn((v) => v.modifiedRange.startLineNumber, Qt));
    for (const v of t) {
      if (f.has(v))
        continue;
      let C = [];
      for (let L = v.modifiedRange.startLineNumber; L < v.modifiedRange.endLineNumberExclusive - 2; L++) {
        const _ = `${s[L - 1]}:${s[L + 1 - 1]}:${s[L + 2 - 1]}`, S = new J(L, L + 3), A = [];
        d.forEach(_, ({ range: R }) => {
          for (const w of C)
            if (w.originalLineRange.endLineNumberExclusive + 1 === R.endLineNumberExclusive && w.modifiedLineRange.endLineNumberExclusive + 1 === S.endLineNumberExclusive) {
              w.originalLineRange = new J(w.originalLineRange.startLineNumber, R.endLineNumberExclusive), w.modifiedLineRange = new J(w.modifiedLineRange.startLineNumber, S.endLineNumberExclusive), A.push(w);
              return;
            }
          const k = {
            modifiedLineRange: S,
            originalLineRange: R
          };
          g.push(k), A.push(k);
        }), C = A;
      }
      if (!a.isValid())
        return [];
    }
    g.sort(Eo(Mn((v) => v.modifiedLineRange.length, Qt)));
    const m = new Gi(), p = new Gi();
    for (const v of g) {
      const C = v.modifiedLineRange.startLineNumber - v.originalLineRange.startLineNumber, L = m.subtractFrom(v.modifiedLineRange), _ = p.subtractFrom(v.originalLineRange).map((A) => A.delta(C)), S = _l(L, _);
      for (const A of S) {
        if (A.length < 3)
          continue;
        const R = A, k = A.delta(-C);
        l.push(new Ot(k, R)), m.addRange(R), p.addRange(k);
      }
    }
    if (l.sort(Mn((v) => v.original.startLineNumber, Qt)), l.length === 0)
      return [];
    let b = [l[0]];
    for (let v = 1; v < l.length; v++) {
      const C = b[b.length - 1], L = l[v], _ = L.original.startLineNumber - C.original.endLineNumberExclusive, S = L.modified.startLineNumber - C.modified.endLineNumberExclusive;
      if (_ >= 0 && S >= 0 && _ + S <= 2) {
        b[b.length - 1] = C.join(L);
        continue;
      }
      L.original.toOffsetRange().slice(n).map((k) => k.trim()).join(`
`).length <= 10 || b.push(L);
    }
    const y = kr.createOfSorted(t, (v) => v.originalRange.endLineNumberExclusive, Qt);
    return b = b.filter((v) => {
      const C = y.findLastItemBeforeOrEqual(v.original.startLineNumber) || new Ie(new J(1, 1), new J(1, 1), []), L = v.modified.startLineNumber - C.modifiedRange.endLineNumberExclusive, _ = v.original.startLineNumber - C.originalRange.endLineNumberExclusive;
      return L !== _;
    }), b.map((v) => {
      const C = this.refineDiff(n, r, new fe(v.original.toOffsetRange(), v.modified.toOffsetRange()), a, o), L = Xi(C.mappings, n, r, !0);
      return new Cr(v, L);
    });
  }
  refineDiff(t, n, r, i, s) {
    const a = new Yi(t, r.seq1Range, s), o = new Yi(n, r.seq2Range, s), l = a.length + o.length < 500 ? this.dynamicProgrammingDiffing.compute(a, o, i) : this.myersDiffingAlgorithm.compute(a, o, i);
    let u = l.diffs;
    return u = Wi(a, o, u), u = Sl(a, o, u), u = dl(a, o, u), u = ml(a, o, u), {
      mappings: u.map((f) => new Dt(a.translateRange(f.seq1Range), o.translateRange(f.seq2Range))),
      hitTimeout: l.hitTimeout
    };
  }
}
class kr {
  static createOfSorted(t, n, r) {
    return new kr(t, n, r);
  }
  constructor(t, n, r) {
    this._items = t, this._itemToDomain = n, this._domainComparator = r, this._currentIdx = 0, this._lastValue = void 0, this._hasLastValue = !1;
  }
  /**
   * Assumes the values are monotonously increasing.
   */
  findLastItemBeforeOrEqual(t) {
    if (this._hasLastValue && fn.isLessThan(this._domainComparator(t, this._lastValue)))
      throw new je();
    for (this._lastValue = t, this._hasLastValue = !0; this._currentIdx < this._items.length && fn.isLessThanOrEqual(this._domainComparator(this._itemToDomain(this._items[this._currentIdx]), t)); )
      this._currentIdx++;
    return this._currentIdx === 0 ? void 0 : this._items[this._currentIdx - 1];
  }
}
function _l(e, t) {
  const n = [];
  let r = 0, i = 0;
  for (; r < e.length && i < t.length; ) {
    const s = e[r], a = t[i], o = s.intersect(a);
    o && !o.isEmpty && n.push(o), s.endLineNumberExclusive < a.endLineNumberExclusive ? r++ : i++;
  }
  return n;
}
class Gi {
  constructor() {
    this._normalizedRanges = [];
  }
  addRange(t) {
    const n = Ji(this._normalizedRanges.findIndex((i) => i.endLineNumberExclusive >= t.startLineNumber), this._normalizedRanges.length), r = Wr(this._normalizedRanges, (i) => i.startLineNumber <= t.endLineNumberExclusive) + 1;
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
  /**
   * Subtracts all ranges in this set from `range` and returns the result.
   */
  subtractFrom(t) {
    const n = Ji(this._normalizedRanges.findIndex((a) => a.endLineNumberExclusive >= t.startLineNumber), this._normalizedRanges.length), r = Wr(this._normalizedRanges, (a) => a.startLineNumber <= t.endLineNumberExclusive) + 1;
    if (n === r)
      return [t];
    const i = [];
    let s = t.startLineNumber;
    for (let a = n; a < r; a++) {
      const o = this._normalizedRanges[a];
      o.startLineNumber > s && i.push(new J(s, o.startLineNumber)), s = o.endLineNumberExclusive;
    }
    return s < t.endLineNumberExclusive && i.push(new J(s, t.endLineNumberExclusive)), i;
  }
}
function Ji(e, t) {
  return e === -1 ? t : e;
}
function Sl(e, t, n) {
  const r = [];
  let i;
  function s() {
    if (!i)
      return;
    const l = i.s1Range.length - i.deleted;
    i.s2Range.length - i.added, Math.max(i.deleted, i.added) + (i.count - 1) > l && r.push(new fe(i.s1Range, i.s2Range)), i = void 0;
  }
  for (const l of n) {
    let u = function(m, p) {
      var b, y, x, v;
      if (!i || !i.s1Range.containsRange(m) || !i.s2Range.containsRange(p))
        if (i && !(i.s1Range.endExclusive < m.start && i.s2Range.endExclusive < p.start)) {
          const _ = z.tryCreate(i.s1Range.endExclusive, m.start), S = z.tryCreate(i.s2Range.endExclusive, p.start);
          i.deleted += (b = _ == null ? void 0 : _.length) !== null && b !== void 0 ? b : 0, i.added += (y = S == null ? void 0 : S.length) !== null && y !== void 0 ? y : 0, i.s1Range = i.s1Range.join(m), i.s2Range = i.s2Range.join(p);
        } else
          s(), i = { added: 0, deleted: 0, count: 0, s1Range: m, s2Range: p };
      const C = m.intersect(l.seq1Range), L = p.intersect(l.seq2Range);
      i.count++, i.deleted += (x = C == null ? void 0 : C.length) !== null && x !== void 0 ? x : 0, i.added += (v = L == null ? void 0 : L.length) !== null && v !== void 0 ? v : 0;
    };
    var o = u;
    const h = e.findWordContaining(l.seq1Range.start - 1), f = t.findWordContaining(l.seq2Range.start - 1), d = e.findWordContaining(l.seq1Range.endExclusive), g = t.findWordContaining(l.seq2Range.endExclusive);
    h && d && f && g && h.equals(d) && f.equals(g) ? u(h, f) : (h && f && u(h, f), d && g && u(d, g));
  }
  return s(), Ll(n, r);
}
function Ll(e, t) {
  const n = [];
  for (; e.length > 0 || t.length > 0; ) {
    const r = e[0], i = t[0];
    let s;
    r && (!i || r.seq1Range.start < i.seq1Range.start) ? s = e.shift() : s = t.shift(), n.length > 0 && n[n.length - 1].seq1Range.endExclusive >= s.seq1Range.start ? n[n.length - 1] = n[n.length - 1].join(s) : n.push(s);
  }
  return n;
}
function Xi(e, t, n, r = !1) {
  const i = [];
  for (const s of Al(e.map((a) => Nl(a, t, n)), (a, o) => a.originalRange.overlapOrTouch(o.originalRange) || a.modifiedRange.overlapOrTouch(o.modifiedRange))) {
    const a = s[0], o = s[s.length - 1];
    i.push(new Ie(a.originalRange.join(o.originalRange), a.modifiedRange.join(o.modifiedRange), s.map((l) => l.innerChanges[0])));
  }
  return gn(() => !r && i.length > 0 && i[0].originalRange.startLineNumber !== i[0].modifiedRange.startLineNumber ? !1 : sa(i, (s, a) => a.originalRange.startLineNumber - s.originalRange.endLineNumberExclusive === a.modifiedRange.startLineNumber - s.modifiedRange.endLineNumberExclusive && // There has to be an unchanged line in between (otherwise both diffs should have been joined)
  s.originalRange.endLineNumberExclusive < a.originalRange.startLineNumber && s.modifiedRange.endLineNumberExclusive < a.modifiedRange.startLineNumber)), i;
}
function Nl(e, t, n) {
  let r = 0, i = 0;
  e.modifiedRange.endColumn === 1 && e.originalRange.endColumn === 1 && e.originalRange.startLineNumber + r <= e.originalRange.endLineNumber && e.modifiedRange.startLineNumber + r <= e.modifiedRange.endLineNumber && (i = -1), e.modifiedRange.startColumn - 1 >= n[e.modifiedRange.startLineNumber - 1].length && e.originalRange.startColumn - 1 >= t[e.originalRange.startLineNumber - 1].length && e.originalRange.startLineNumber <= e.originalRange.endLineNumber + i && e.modifiedRange.startLineNumber <= e.modifiedRange.endLineNumber + i && (r = 1);
  const s = new J(e.originalRange.startLineNumber + r, e.originalRange.endLineNumber + 1 + i), a = new J(e.modifiedRange.startLineNumber + r, e.modifiedRange.endLineNumber + 1 + i);
  return new Ie(s, a, [e]);
}
function* Al(e, t) {
  let n, r;
  for (const i of e)
    r !== void 0 && t(r, i) ? n.push(i) : (n && (yield n), n = [i]), r = i;
  n && (yield n);
}
class Qi {
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
    const n = t === 0 ? 0 : Zi(this.lines[t - 1]), r = t === this.lines.length ? 0 : Zi(this.lines[t]);
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
function Zi(e) {
  let t = 0;
  for (; t < e.length && (e.charCodeAt(t) === 32 || e.charCodeAt(t) === 9); )
    t++;
  return t;
}
class Yi {
  constructor(t, n, r) {
    this.lines = t, this.considerWhitespaceChanges = r, this.elements = [], this.firstCharOffsetByLineMinusOne = [], this.additionalOffsetByLine = [];
    let i = !1;
    n.start > 0 && n.endExclusive >= t.length && (n = new z(n.start - 1, n.endExclusive), i = !0), this.lineRange = n;
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
`.charCodeAt(0)), this.firstCharOffsetByLineMinusOne[s - this.lineRange.start] = this.elements.length);
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
    const n = es(t > 0 ? this.elements[t - 1] : -1), r = es(t < this.elements.length ? this.elements[t] : -1);
    if (n === 6 && r === 7)
      return 0;
    let i = 0;
    return n !== r && (i += 10, r === 1 && (i += 1)), i += Ki(n), i += Ki(r), i;
  }
  translateOffset(t) {
    if (this.lineRange.isEmpty)
      return new De(this.lineRange.start + 1, 1);
    let n = 0, r = this.firstCharOffsetByLineMinusOne.length;
    for (; n < r; ) {
      const s = Math.floor((n + r) / 2);
      this.firstCharOffsetByLineMinusOne[s] > t ? r = s : n = s + 1;
    }
    const i = n === 0 ? 0 : this.firstCharOffsetByLineMinusOne[n - 1];
    return new De(this.lineRange.start + n + 1, t - i + 1 + this.additionalOffsetByLine[n]);
  }
  translateRange(t) {
    return pe.fromPositions(this.translateOffset(t.start), this.translateOffset(t.endExclusive));
  }
  /**
   * Finds the word that contains the character at the given offset
   */
  findWordContaining(t) {
    if (t < 0 || t >= this.elements.length || !Fn(this.elements[t]))
      return;
    let n = t;
    for (; n > 0 && Fn(this.elements[n - 1]); )
      n--;
    let r = t;
    for (; r < this.elements.length && Fn(this.elements[r]); )
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
    const i = (n = kl(this.firstCharOffsetByLineMinusOne, (a) => a <= t.start)) !== null && n !== void 0 ? n : 0, s = (r = El(this.firstCharOffsetByLineMinusOne, (a) => t.endExclusive <= a)) !== null && r !== void 0 ? r : this.elements.length;
    return new z(i, s);
  }
}
function Cl(e, t) {
  let n = 0, r = e.length;
  for (; n < r; ) {
    const i = Math.floor((n + r) / 2);
    t(e[i]) ? n = i + 1 : r = i;
  }
  return n - 1;
}
function kl(e, t) {
  const n = Cl(e, t);
  return n === -1 ? void 0 : e[n];
}
function Rl(e, t) {
  let n = 0, r = e.length;
  for (; n < r; ) {
    const i = Math.floor((n + r) / 2);
    t(e[i]) ? r = i : n = i + 1;
  }
  return n;
}
function El(e, t) {
  const n = Rl(e, t);
  return n === e.length ? void 0 : e[n];
}
function Fn(e) {
  return e >= 97 && e <= 122 || e >= 65 && e <= 90 || e >= 48 && e <= 57;
}
const Ml = {
  0: 0,
  1: 0,
  2: 0,
  3: 10,
  4: 2,
  5: 3,
  6: 10,
  7: 10
};
function Ki(e) {
  return Ml[e];
}
function es(e) {
  return e === 10 ? 7 : e === 13 ? 6 : Tl(e) ? 5 : e >= 97 && e <= 122 ? 0 : e >= 65 && e <= 90 ? 1 : e >= 48 && e <= 57 ? 2 : e === -1 ? 3 : 4;
}
function Tl(e) {
  return e === 32 || e === 9;
}
const In = /* @__PURE__ */ new Map();
function ts(e) {
  let t = In.get(e);
  return t === void 0 && (t = In.size, In.set(e, t)), t;
}
class ns {
  constructor(t, n, r) {
    this.range = t, this.lines = n, this.source = r, this.histogram = [];
    let i = 0;
    for (let s = t.startLineNumber - 1; s < t.endLineNumberExclusive - 1; s++) {
      const a = n[s];
      for (let l = 0; l < a.length; l++) {
        i++;
        const u = a[l], h = ts(u);
        this.histogram[h] = (this.histogram[h] || 0) + 1;
      }
      i++;
      const o = ts(`
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
const rs = {
  getLegacy: () => new al(),
  getAdvanced: () => new wl()
};
function Ke(e, t) {
  const n = Math.pow(10, t);
  return Math.round(e * n) / n;
}
class ie {
  constructor(t, n, r, i = 1) {
    this._rgbaBrand = void 0, this.r = Math.min(255, Math.max(0, t)) | 0, this.g = Math.min(255, Math.max(0, n)) | 0, this.b = Math.min(255, Math.max(0, r)) | 0, this.a = Ke(Math.max(Math.min(1, i), 0), 3);
  }
  static equals(t, n) {
    return t.r === n.r && t.g === n.g && t.b === n.b && t.a === n.a;
  }
}
class Le {
  constructor(t, n, r, i) {
    this._hslaBrand = void 0, this.h = Math.max(Math.min(360, t), 0) | 0, this.s = Ke(Math.max(Math.min(1, n), 0), 3), this.l = Ke(Math.max(Math.min(1, r), 0), 3), this.a = Ke(Math.max(Math.min(1, i), 0), 3);
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
    const h = (o + a) / 2, f = a - o;
    if (f > 0) {
      switch (u = Math.min(h <= 0.5 ? f / (2 * h) : f / (2 - 2 * h), 1), a) {
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
    return new Le(l, u, h, s);
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
      const u = i < 0.5 ? i * (1 + r) : i + r - i * r, h = 2 * i - u;
      a = Le._hue2rgb(h, u, n + 1 / 3), o = Le._hue2rgb(h, u, n), l = Le._hue2rgb(h, u, n - 1 / 3);
    }
    return new ie(Math.round(a * 255), Math.round(o * 255), Math.round(l * 255), s);
  }
}
class pt {
  constructor(t, n, r, i) {
    this._hsvaBrand = void 0, this.h = Math.max(Math.min(360, t), 0) | 0, this.s = Ke(Math.max(Math.min(1, n), 0), 3), this.v = Ke(Math.max(Math.min(1, r), 0), 3), this.a = Ke(Math.max(Math.min(1, i), 0), 3);
  }
  static equals(t, n) {
    return t.h === n.h && t.s === n.s && t.v === n.v && t.a === n.a;
  }
  // from http://www.rapidtables.com/convert/color/rgb-to-hsv.htm
  static fromRGBA(t) {
    const n = t.r / 255, r = t.g / 255, i = t.b / 255, s = Math.max(n, r, i), a = Math.min(n, r, i), o = s - a, l = s === 0 ? 0 : o / s;
    let u;
    return o === 0 ? u = 0 : s === n ? u = ((r - i) / o % 6 + 6) % 6 : s === r ? u = (i - n) / o + 2 : u = (n - r) / o + 4, new pt(Math.round(u * 60), l, s, t.a);
  }
  // from http://www.rapidtables.com/convert/color/hsv-to-rgb.htm
  static toRGBA(t) {
    const { h: n, s: r, v: i, a: s } = t, a = i * r, o = a * (1 - Math.abs(n / 60 % 2 - 1)), l = i - a;
    let [u, h, f] = [0, 0, 0];
    return n < 60 ? (u = a, h = o) : n < 120 ? (u = o, h = a) : n < 180 ? (h = a, f = o) : n < 240 ? (h = o, f = a) : n < 300 ? (u = o, f = a) : n <= 360 && (u = a, f = o), u = Math.round((u + l) * 255), h = Math.round((h + l) * 255), f = Math.round((f + l) * 255), new ie(u, h, f, s);
  }
}
let re = class Se {
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
    return this._hsva ? this._hsva : pt.fromRGBA(this.rgba);
  }
  constructor(t) {
    if (t)
      if (t instanceof ie)
        this.rgba = t;
      else if (t instanceof Le)
        this._hsla = t, this.rgba = Le.toRGBA(t);
      else if (t instanceof pt)
        this._hsva = t, this.rgba = pt.toRGBA(t);
      else
        throw new Error("Invalid color ctor argument");
    else
      throw new Error("Color needs a value");
  }
  equals(t) {
    return !!t && ie.equals(this.rgba, t.rgba) && Le.equals(this.hsla, t.hsla) && pt.equals(this.hsva, t.hsva);
  }
  /**
   * http://www.w3.org/TR/WCAG20/#relativeluminancedef
   * Returns the number in the set [0, 1]. O => Darkest Black. 1 => Lightest white.
   */
  getRelativeLuminance() {
    const t = Se._relativeLuminanceForComponent(this.rgba.r), n = Se._relativeLuminanceForComponent(this.rgba.g), r = Se._relativeLuminanceForComponent(this.rgba.b), i = 0.2126 * t + 0.7152 * n + 0.0722 * r;
    return Ke(i, 4);
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
    return new Se(new ie(n, r, i, s * t));
  }
  isTransparent() {
    return this.rgba.a === 0;
  }
  isOpaque() {
    return this.rgba.a === 1;
  }
  opposite() {
    return new Se(new ie(255 - this.rgba.r, 255 - this.rgba.g, 255 - this.rgba.b, this.rgba.a));
  }
  makeOpaque(t) {
    if (this.isOpaque() || t.rgba.a !== 1)
      return this;
    const { r: n, g: r, b: i, a: s } = this.rgba;
    return new Se(new ie(t.rgba.r - s * (t.rgba.r - n), t.rgba.g - s * (t.rgba.g - r), t.rgba.b - s * (t.rgba.b - i), 1));
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
re.white = new re(new ie(255, 255, 255, 1));
re.black = new re(new ie(0, 0, 0, 1));
re.red = new re(new ie(255, 0, 0, 1));
re.blue = new re(new ie(0, 0, 255, 1));
re.green = new re(new ie(0, 255, 0, 1));
re.cyan = new re(new ie(0, 255, 255, 1));
re.lightgrey = new re(new ie(211, 211, 211, 1));
re.transparent = new re(new ie(0, 0, 0, 0));
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
      function h(g) {
        return g.isOpaque() ? e.Format.CSS.formatHex(g) : e.Format.CSS.formatRGBA(g);
      }
      n.format = h;
      function f(g) {
        const m = g.length;
        if (m === 0 || g.charCodeAt(0) !== 35)
          return null;
        if (m === 7) {
          const p = 16 * d(g.charCodeAt(1)) + d(g.charCodeAt(2)), b = 16 * d(g.charCodeAt(3)) + d(g.charCodeAt(4)), y = 16 * d(g.charCodeAt(5)) + d(g.charCodeAt(6));
          return new e(new ie(p, b, y, 1));
        }
        if (m === 9) {
          const p = 16 * d(g.charCodeAt(1)) + d(g.charCodeAt(2)), b = 16 * d(g.charCodeAt(3)) + d(g.charCodeAt(4)), y = 16 * d(g.charCodeAt(5)) + d(g.charCodeAt(6)), x = 16 * d(g.charCodeAt(7)) + d(g.charCodeAt(8));
          return new e(new ie(p, b, y, x / 255));
        }
        if (m === 4) {
          const p = d(g.charCodeAt(1)), b = d(g.charCodeAt(2)), y = d(g.charCodeAt(3));
          return new e(new ie(16 * p + p, 16 * b + b, 16 * y + y));
        }
        if (m === 5) {
          const p = d(g.charCodeAt(1)), b = d(g.charCodeAt(2)), y = d(g.charCodeAt(3)), x = d(g.charCodeAt(4));
          return new e(new ie(16 * p + p, 16 * b + b, 16 * y + y, (16 * x + x) / 255));
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
})(re || (re = {}));
function oa(e) {
  const t = [];
  for (const n of e) {
    const r = Number(n);
    (r || r === 0 && n.replace(/\s/g, "") !== "") && t.push(r);
  }
  return t;
}
function Rr(e, t, n, r) {
  return {
    red: e / 255,
    blue: n / 255,
    green: t / 255,
    alpha: r
  };
}
function Ct(e, t) {
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
function Pl(e, t) {
  if (!e)
    return;
  const n = re.Format.CSS.parseHex(t);
  if (n)
    return {
      range: e,
      color: Rr(n.rgba.r, n.rgba.g, n.rgba.b, n.rgba.a)
    };
}
function is(e, t, n) {
  if (!e || t.length !== 1)
    return;
  const i = t[0].values(), s = oa(i);
  return {
    range: e,
    color: Rr(s[0], s[1], s[2], n ? s[3] : 1)
  };
}
function ss(e, t, n) {
  if (!e || t.length !== 1)
    return;
  const i = t[0].values(), s = oa(i), a = new re(new Le(s[0], s[1] / 100, s[2] / 100, n ? s[3] : 1));
  return {
    range: e,
    color: Rr(a.rgba.r, a.rgba.g, a.rgba.b, a.rgba.a)
  };
}
function kt(e, t) {
  return typeof e == "string" ? [...e.matchAll(t)] : e.findMatches(t);
}
function Fl(e) {
  const t = [], r = kt(e, /\b(rgb|rgba|hsl|hsla)(\([0-9\s,.\%]*\))|(#)([A-Fa-f0-9]{3})\b|(#)([A-Fa-f0-9]{4})\b|(#)([A-Fa-f0-9]{6})\b|(#)([A-Fa-f0-9]{8})\b/gm);
  if (r.length > 0)
    for (const i of r) {
      const s = i.filter((u) => u !== void 0), a = s[1], o = s[2];
      if (!o)
        continue;
      let l;
      if (a === "rgb") {
        const u = /^\(\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*\)$/gm;
        l = is(Ct(e, i), kt(o, u), !1);
      } else if (a === "rgba") {
        const u = /^\(\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(0[.][0-9]+|[.][0-9]+|[01][.]|[01])\s*\)$/gm;
        l = is(Ct(e, i), kt(o, u), !0);
      } else if (a === "hsl") {
        const u = /^\(\s*(36[0]|3[0-5][0-9]|[12][0-9][0-9]|[1-9]?[0-9])\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*\)$/gm;
        l = ss(Ct(e, i), kt(o, u), !1);
      } else if (a === "hsla") {
        const u = /^\(\s*(36[0]|3[0-5][0-9]|[12][0-9][0-9]|[1-9]?[0-9])\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*,\s*(0[.][0-9]+|[.][0-9]+|[01][.]|[01])\s*\)$/gm;
        l = ss(Ct(e, i), kt(o, u), !0);
      } else
        a === "#" && (l = Pl(Ct(e, i), a + o));
      l && t.push(l);
    }
  return t;
}
function Il(e) {
  return !e || typeof e.getValue != "function" || typeof e.positionAt != "function" ? [] : Fl(e);
}
var He = globalThis && globalThis.__awaiter || function(e, t, n, r) {
  function i(s) {
    return s instanceof n ? s : new n(function(a) {
      a(s);
    });
  }
  return new (n || (n = Promise))(function(s, a) {
    function o(h) {
      try {
        u(r.next(h));
      } catch (f) {
        a(f);
      }
    }
    function l(h) {
      try {
        u(r.throw(h));
      } catch (f) {
        a(f);
      }
    }
    function u(h) {
      h.done ? s(h.value) : i(h.value).then(o, l);
    }
    u((r = r.apply(e, t || [])).next());
  });
};
class Vl extends Po {
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
      const i = this._lines[r], s = this.offsetAt(new De(r + 1, 1)), a = i.matchAll(t);
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
    const r = Lr(t.column, Vo(n), this._lines[t.lineNumber - 1], 0);
    return r ? new pe(t.lineNumber, r.startColumn, t.lineNumber, r.endColumn) : null;
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
    if (!De.isIPosition(t))
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
class nt {
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
    this._models[t.url] = new Vl(Sr.parse(t.url), t.lines, t.EOL, t.versionId);
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
    return He(this, void 0, void 0, function* () {
      const i = this._getModel(t);
      return i ? rl.computeUnicodeHighlights(i, n, r) : { ranges: [], hasMore: !1, ambiguousCharacterCount: 0, invisibleCharacterCount: 0, nonBasicAsciiCharacterCount: 0 };
    });
  }
  // ---- BEGIN diff --------------------------------------------------------------------------
  computeDiff(t, n, r, i) {
    return He(this, void 0, void 0, function* () {
      const s = this._getModel(t), a = this._getModel(n);
      return !s || !a ? null : nt.computeDiff(s, a, r, i);
    });
  }
  static computeDiff(t, n, r, i) {
    const s = i === "advanced" ? rs.getAdvanced() : rs.getLegacy(), a = t.getLinesContent(), o = n.getLinesContent(), l = s.computeDiff(a, o, r), u = l.changes.length > 0 ? !1 : this._modelsAreIdentical(t, n);
    function h(f) {
      return f.map((d) => {
        var g;
        return [d.originalRange.startLineNumber, d.originalRange.endLineNumberExclusive, d.modifiedRange.startLineNumber, d.modifiedRange.endLineNumberExclusive, (g = d.innerChanges) === null || g === void 0 ? void 0 : g.map((m) => [
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
      changes: h(l.changes),
      moves: l.moves.map((f) => [
        f.lineRangeMapping.original.startLineNumber,
        f.lineRangeMapping.original.endLineNumberExclusive,
        f.lineRangeMapping.modified.startLineNumber,
        f.lineRangeMapping.modified.endLineNumberExclusive,
        h(f.changes)
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
    return He(this, void 0, void 0, function* () {
      const i = this._getModel(t);
      if (!i)
        return n;
      const s = [];
      let a;
      n = n.slice(0).sort((o, l) => {
        if (o.range && l.range)
          return pe.compareRangesUsingStarts(o.range, l.range);
        const u = o.range ? 0 : 1, h = l.range ? 0 : 1;
        return u - h;
      });
      for (let { range: o, text: l, eol: u } of n) {
        if (typeof u == "number" && (a = u), pe.isEmpty(o) && !l)
          continue;
        const h = i.getValueInRange(o);
        if (l = l.replace(/\r\n|\n|\r/g, i.eol), h === l)
          continue;
        if (Math.max(l.length, h.length) > nt._diffLimit) {
          s.push({ range: o, text: l });
          continue;
        }
        const f = co(h, l, r), d = i.offsetAt(pe.lift(o).getStartPosition());
        for (const g of f) {
          const m = i.positionAt(d + g.originalStart), p = i.positionAt(d + g.originalStart + g.originalLength), b = {
            text: l.substr(g.modifiedStart, g.modifiedLength),
            range: { startLineNumber: m.lineNumber, startColumn: m.column, endLineNumber: p.lineNumber, endColumn: p.column }
          };
          i.getValueInRange(b.range) !== b.text && s.push(b);
        }
      }
      return typeof a == "number" && s.push({ eol: a, text: "", range: { startLineNumber: 0, startColumn: 0, endLineNumber: 0, endColumn: 0 } }), s;
    });
  }
  // ---- END minimal edits ---------------------------------------------------------------
  computeLinks(t) {
    return He(this, void 0, void 0, function* () {
      const n = this._getModel(t);
      return n ? Uo(n) : null;
    });
  }
  // --- BEGIN default document colors -----------------------------------------------------------
  computeDefaultDocumentColors(t) {
    return He(this, void 0, void 0, function* () {
      const n = this._getModel(t);
      return n ? Il(n) : null;
    });
  }
  textualSuggest(t, n, r, i) {
    return He(this, void 0, void 0, function* () {
      const s = new Sn(), a = new RegExp(r, i), o = /* @__PURE__ */ new Set();
      e:
        for (const l of t) {
          const u = this._getModel(l);
          if (u) {
            for (const h of u.words(a))
              if (!(h === n || !isNaN(Number(h))) && (o.add(h), o.size > nt._suggestionsLimit))
                break e;
          }
        }
      return { words: Array.from(o), duration: s.elapsed() };
    });
  }
  // ---- END suggest --------------------------------------------------------------------------
  //#region -- word ranges --
  computeWordRanges(t, n, r, i) {
    return He(this, void 0, void 0, function* () {
      const s = this._getModel(t);
      if (!s)
        return /* @__PURE__ */ Object.create(null);
      const a = new RegExp(r, i), o = /* @__PURE__ */ Object.create(null);
      for (let l = n.startLineNumber; l < n.endLineNumber; l++) {
        const u = s.getLineWords(l, a);
        for (const h of u) {
          if (!isNaN(Number(h.word)))
            continue;
          let f = o[h.word];
          f || (f = [], o[h.word] = f), f.push({
            startLineNumber: l,
            startColumn: h.startColumn,
            endLineNumber: l,
            endColumn: h.endColumn
          });
        }
      }
      return o;
    });
  }
  //#endregion
  navigateValueSet(t, n, r, i, s) {
    return He(this, void 0, void 0, function* () {
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
      const h = a.getValueInRange(u);
      return Yn.INSTANCE.navigateValueSet(n, l, u, h, r);
    });
  }
  // ---- BEGIN foreign module support --------------------------------------------------------------------------
  loadForeignModule(t, n, r) {
    const a = {
      host: Va(r, (o, l) => this._host.fhr(o, l)),
      getMirrorModels: () => this._getModels()
    };
    return this._foreignModuleFactory ? (this._foreignModule = this._foreignModuleFactory(a, n), Promise.resolve(Hn(this._foreignModule))) : Promise.reject(new Error("Unexpected usage"));
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
nt._diffLimit = 1e5;
nt._suggestionsLimit = 1e4;
typeof importScripts == "function" && (globalThis.monaco = Zo());
let lr = !1;
function la(e) {
  if (lr)
    return;
  lr = !0;
  const t = new lo((n) => {
    globalThis.postMessage(n);
  }, (n) => new nt(n, e));
  globalThis.onmessage = (n) => {
    t.onmessage(n.data);
  };
}
globalThis.onmessage = (e) => {
  lr || la(null);
};
/*!-----------------------------------------------------------------------------
 * Copyright (c) Microsoft Corporation. All rights reserved.
 * Version: 0.43.0(94c055bcbdd49f04a0fa15515e848542a79fb948)
 * Released under the MIT license
 * https://github.com/microsoft/monaco-editor/blob/main/LICENSE.txt
 *-----------------------------------------------------------------------------*/
function Er(e, t) {
  t === void 0 && (t = !1);
  var n = e.length, r = 0, i = "", s = 0, a = 16, o = 0, l = 0, u = 0, h = 0, f = 0;
  function d(v, C) {
    for (var L = 0, _ = 0; L < v || !C; ) {
      var S = e.charCodeAt(r);
      if (S >= 48 && S <= 57)
        _ = _ * 16 + S - 48;
      else if (S >= 65 && S <= 70)
        _ = _ * 16 + S - 65 + 10;
      else if (S >= 97 && S <= 102)
        _ = _ * 16 + S - 97 + 10;
      else
        break;
      r++, L++;
    }
    return L < v && (_ = -1), _;
  }
  function g(v) {
    r = v, i = "", s = 0, a = 16, f = 0;
  }
  function m() {
    var v = r;
    if (e.charCodeAt(r) === 48)
      r++;
    else
      for (r++; r < e.length && dt(e.charCodeAt(r)); )
        r++;
    if (r < e.length && e.charCodeAt(r) === 46)
      if (r++, r < e.length && dt(e.charCodeAt(r)))
        for (r++; r < e.length && dt(e.charCodeAt(r)); )
          r++;
      else
        return f = 3, e.substring(v, r);
    var C = r;
    if (r < e.length && (e.charCodeAt(r) === 69 || e.charCodeAt(r) === 101))
      if (r++, (r < e.length && e.charCodeAt(r) === 43 || e.charCodeAt(r) === 45) && r++, r < e.length && dt(e.charCodeAt(r))) {
        for (r++; r < e.length && dt(e.charCodeAt(r)); )
          r++;
        C = r;
      } else
        f = 3;
    return e.substring(v, C);
  }
  function p() {
    for (var v = "", C = r; ; ) {
      if (r >= n) {
        v += e.substring(C, r), f = 2;
        break;
      }
      var L = e.charCodeAt(r);
      if (L === 34) {
        v += e.substring(C, r), r++;
        break;
      }
      if (L === 92) {
        if (v += e.substring(C, r), r++, r >= n) {
          f = 2;
          break;
        }
        var _ = e.charCodeAt(r++);
        switch (_) {
          case 34:
            v += '"';
            break;
          case 92:
            v += "\\";
            break;
          case 47:
            v += "/";
            break;
          case 98:
            v += "\b";
            break;
          case 102:
            v += "\f";
            break;
          case 110:
            v += `
`;
            break;
          case 114:
            v += "\r";
            break;
          case 116:
            v += "	";
            break;
          case 117:
            var S = d(4, !0);
            S >= 0 ? v += String.fromCharCode(S) : f = 4;
            break;
          default:
            f = 5;
        }
        C = r;
        continue;
      }
      if (L >= 0 && L <= 31)
        if (Rt(L)) {
          v += e.substring(C, r), f = 2;
          break;
        } else
          f = 6;
      r++;
    }
    return v;
  }
  function b() {
    if (i = "", f = 0, s = r, l = o, h = u, r >= n)
      return s = n, a = 17;
    var v = e.charCodeAt(r);
    if (Vn(v)) {
      do
        r++, i += String.fromCharCode(v), v = e.charCodeAt(r);
      while (Vn(v));
      return a = 15;
    }
    if (Rt(v))
      return r++, i += String.fromCharCode(v), v === 13 && e.charCodeAt(r) === 10 && (r++, i += `
`), o++, u = r, a = 14;
    switch (v) {
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
        var C = r - 1;
        if (e.charCodeAt(r + 1) === 47) {
          for (r += 2; r < n && !Rt(e.charCodeAt(r)); )
            r++;
          return i = e.substring(C, r), a = 12;
        }
        if (e.charCodeAt(r + 1) === 42) {
          r += 2;
          for (var L = n - 1, _ = !1; r < L; ) {
            var S = e.charCodeAt(r);
            if (S === 42 && e.charCodeAt(r + 1) === 47) {
              r += 2, _ = !0;
              break;
            }
            r++, Rt(S) && (S === 13 && e.charCodeAt(r) === 10 && r++, o++, u = r);
          }
          return _ || (r++, f = 1), i = e.substring(C, r), a = 13;
        }
        return i += String.fromCharCode(v), r++, a = 16;
      case 45:
        if (i += String.fromCharCode(v), r++, r === n || !dt(e.charCodeAt(r)))
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
        for (; r < n && y(v); )
          r++, v = e.charCodeAt(r);
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
        return i += String.fromCharCode(v), r++, a = 16;
    }
  }
  function y(v) {
    if (Vn(v) || Rt(v))
      return !1;
    switch (v) {
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
    var v;
    do
      v = b();
    while (v >= 12 && v <= 15);
    return v;
  }
  return {
    setPosition: g,
    getPosition: function() {
      return r;
    },
    scan: t ? x : b,
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
      return s - h;
    },
    getTokenError: function() {
      return f;
    }
  };
}
function Vn(e) {
  return e === 32 || e === 9 || e === 11 || e === 12 || e === 160 || e === 5760 || e >= 8192 && e <= 8203 || e === 8239 || e === 8287 || e === 12288 || e === 65279;
}
function Rt(e) {
  return e === 10 || e === 13 || e === 8232 || e === 8233;
}
function dt(e) {
  return e >= 48 && e <= 57;
}
function Dl(e, t, n) {
  var r, i, s, a, o;
  if (t) {
    for (a = t.offset, o = a + t.length, s = a; s > 0 && !as(e, s - 1); )
      s--;
    for (var l = o; l < e.length && !as(e, l); )
      l++;
    i = e.substring(s, l), r = Ol(i, n);
  } else
    i = e, r = 0, s = 0, a = 0, o = e.length;
  var u = jl(n, e), h = !1, f = 0, d;
  n.insertSpaces ? d = Dn(" ", n.tabSize || 4) : d = "	";
  var g = Er(i, !1), m = !1;
  function p() {
    return u + Dn(d, r + f);
  }
  function b() {
    var N = g.scan();
    for (h = !1; N === 15 || N === 14; )
      h = h || N === 14, N = g.scan();
    return m = N === 16 || g.getTokenError() !== 0, N;
  }
  var y = [];
  function x(N, T, V) {
    !m && (!t || T < o && V > a) && e.substring(T, V) !== N && y.push({ offset: T, length: V - T, content: N });
  }
  var v = b();
  if (v !== 17) {
    var C = g.getTokenOffset() + s, L = Dn(d, r);
    x(L, s, C);
  }
  for (; v !== 17; ) {
    for (var _ = g.getTokenOffset() + g.getTokenLength() + s, S = b(), A = "", R = !1; !h && (S === 12 || S === 13); ) {
      var k = g.getTokenOffset() + s;
      x(" ", _, k), _ = g.getTokenOffset() + g.getTokenLength() + s, R = S === 12, A = R ? p() : "", S = b();
    }
    if (S === 2)
      v !== 1 && (f--, A = p());
    else if (S === 4)
      v !== 3 && (f--, A = p());
    else {
      switch (v) {
        case 3:
        case 1:
          f++, A = p();
          break;
        case 5:
        case 12:
          A = p();
          break;
        case 13:
          h ? A = p() : R || (A = " ");
          break;
        case 6:
          R || (A = " ");
          break;
        case 10:
          if (S === 6) {
            R || (A = "");
            break;
          }
        case 7:
        case 8:
        case 9:
        case 11:
        case 2:
        case 4:
          S === 12 || S === 13 ? R || (A = " ") : S !== 5 && S !== 17 && (m = !0);
          break;
        case 16:
          m = !0;
          break;
      }
      h && (S === 12 || S === 13) && (A = p());
    }
    S === 17 && (A = n.insertFinalNewline ? u : "");
    var w = g.getTokenOffset() + s;
    x(A, _, w), v = S;
  }
  return y;
}
function Dn(e, t) {
  for (var n = "", r = 0; r < t; r++)
    n += e;
  return n;
}
function Ol(e, t) {
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
function jl(e, t) {
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
function as(e, t) {
  return `\r
`.indexOf(e.charAt(t)) !== -1;
}
var mn;
(function(e) {
  e.DEFAULT = {
    allowTrailingComma: !1
  };
})(mn || (mn = {}));
function Bl(e, t, n) {
  t === void 0 && (t = []), n === void 0 && (n = mn.DEFAULT);
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
    onError: function(l, u, h) {
      t.push({ error: l, offset: u, length: h });
    }
  };
  return Ul(e, o, n), i[0];
}
function ua(e) {
  if (!e.parent || !e.parent.children)
    return [];
  var t = ua(e.parent);
  if (e.parent.type === "property") {
    var n = e.parent.children[0].value;
    t.push(n);
  } else if (e.parent.type === "array") {
    var r = e.parent.children.indexOf(e);
    r !== -1 && t.push(r);
  }
  return t;
}
function ur(e) {
  switch (e.type) {
    case "array":
      return e.children.map(ur);
    case "object":
      for (var t = /* @__PURE__ */ Object.create(null), n = 0, r = e.children; n < r.length; n++) {
        var i = r[n], s = i.children[1];
        s && (t[i.children[0].value] = ur(s));
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
function ql(e, t, n) {
  return n === void 0 && (n = !1), t >= e.offset && t < e.offset + e.length || n && t === e.offset + e.length;
}
function ca(e, t, n) {
  if (n === void 0 && (n = !1), ql(e, t, n)) {
    var r = e.children;
    if (Array.isArray(r))
      for (var i = 0; i < r.length && r[i].offset <= t; i++) {
        var s = ca(r[i], t, n);
        if (s)
          return s;
      }
    return e;
  }
}
function Ul(e, t, n) {
  n === void 0 && (n = mn.DEFAULT);
  var r = Er(e, !1);
  function i(R) {
    return R ? function() {
      return R(r.getTokenOffset(), r.getTokenLength(), r.getTokenStartLine(), r.getTokenStartCharacter());
    } : function() {
      return !0;
    };
  }
  function s(R) {
    return R ? function(k) {
      return R(k, r.getTokenOffset(), r.getTokenLength(), r.getTokenStartLine(), r.getTokenStartCharacter());
    } : function() {
      return !0;
    };
  }
  var a = i(t.onObjectBegin), o = s(t.onObjectProperty), l = i(t.onObjectEnd), u = i(t.onArrayBegin), h = i(t.onArrayEnd), f = s(t.onLiteralValue), d = s(t.onSeparator), g = i(t.onComment), m = s(t.onError), p = n && n.disallowComments, b = n && n.allowTrailingComma;
  function y() {
    for (; ; ) {
      var R = r.scan();
      switch (r.getTokenError()) {
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
          p || x(11);
          break;
        case 2:
          x(12);
          break;
        case 6:
          x(16);
          break;
      }
      switch (R) {
        case 12:
        case 13:
          p ? x(10) : g();
          break;
        case 16:
          x(1);
          break;
        case 15:
        case 14:
          break;
        default:
          return R;
      }
    }
  }
  function x(R, k, w) {
    if (k === void 0 && (k = []), w === void 0 && (w = []), m(R), k.length + w.length > 0)
      for (var N = r.getToken(); N !== 17; ) {
        if (k.indexOf(N) !== -1) {
          y();
          break;
        } else if (w.indexOf(N) !== -1)
          break;
        N = y();
      }
  }
  function v(R) {
    var k = r.getTokenValue();
    return R ? f(k) : o(k), y(), !0;
  }
  function C() {
    switch (r.getToken()) {
      case 11:
        var R = r.getTokenValue(), k = Number(R);
        isNaN(k) && (x(2), k = 0), f(k);
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
    return y(), !0;
  }
  function L() {
    return r.getToken() !== 10 ? (x(3, [], [2, 5]), !1) : (v(!1), r.getToken() === 6 ? (d(":"), y(), A() || x(4, [], [2, 5])) : x(5, [], [2, 5]), !0);
  }
  function _() {
    a(), y();
    for (var R = !1; r.getToken() !== 2 && r.getToken() !== 17; ) {
      if (r.getToken() === 5) {
        if (R || x(4, [], []), d(","), y(), r.getToken() === 2 && b)
          break;
      } else
        R && x(6, [], []);
      L() || x(4, [], [2, 5]), R = !0;
    }
    return l(), r.getToken() !== 2 ? x(7, [2], []) : y(), !0;
  }
  function S() {
    u(), y();
    for (var R = !1; r.getToken() !== 4 && r.getToken() !== 17; ) {
      if (r.getToken() === 5) {
        if (R || x(4, [], []), d(","), y(), r.getToken() === 4 && b)
          break;
      } else
        R && x(6, [], []);
      A() || x(4, [], [4, 5]), R = !0;
    }
    return h(), r.getToken() !== 4 ? x(8, [4], []) : y(), !0;
  }
  function A() {
    switch (r.getToken()) {
      case 3:
        return S();
      case 1:
        return _();
      case 10:
        return v(!0);
      default:
        return C();
    }
  }
  return y(), r.getToken() === 17 ? n.allowEmptyContent ? !0 : (x(4, [], []), !1) : A() ? (r.getToken() !== 17 && x(9, [], []), !0) : (x(4, [], []), !1);
}
var yt = Er, $l = Bl, Wl = ca, Hl = ua, zl = ur;
function Gl(e, t, n) {
  return Dl(e, t, n);
}
function Tt(e, t) {
  if (e === t)
    return !0;
  if (e == null || t === null || t === void 0 || typeof e != typeof t || typeof e != "object" || Array.isArray(e) !== Array.isArray(t))
    return !1;
  var n, r;
  if (Array.isArray(e)) {
    if (e.length !== t.length)
      return !1;
    for (n = 0; n < e.length; n++)
      if (!Tt(e[n], t[n]))
        return !1;
  } else {
    var i = [];
    for (r in e)
      i.push(r);
    i.sort();
    var s = [];
    for (r in t)
      s.push(r);
    if (s.sort(), !Tt(i, s))
      return !1;
    for (n = 0; n < i.length; n++)
      if (!Tt(e[i[n]], t[i[n]]))
        return !1;
  }
  return !0;
}
function ve(e) {
  return typeof e == "number";
}
function Oe(e) {
  return typeof e < "u";
}
function Fe(e) {
  return typeof e == "boolean";
}
function Jl(e) {
  return typeof e == "string";
}
function Xl(e, t) {
  if (e.length < t.length)
    return !1;
  for (var n = 0; n < t.length; n++)
    if (e[n] !== t[n])
      return !1;
  return !0;
}
function Bt(e, t) {
  var n = e.length - t.length;
  return n > 0 ? e.lastIndexOf(t) === n : n === 0 ? e === t : !1;
}
function pn(e) {
  var t = "";
  Xl(e, "(?i)") && (e = e.substring(4), t = "i");
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
var os;
(function(e) {
  e.MIN_VALUE = -2147483648, e.MAX_VALUE = 2147483647;
})(os || (os = {}));
var vn;
(function(e) {
  e.MIN_VALUE = 0, e.MAX_VALUE = 2147483647;
})(vn || (vn = {}));
var ke;
(function(e) {
  function t(r, i) {
    return r === Number.MAX_VALUE && (r = vn.MAX_VALUE), i === Number.MAX_VALUE && (i = vn.MAX_VALUE), { line: r, character: i };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return E.objectLiteral(i) && E.uinteger(i.line) && E.uinteger(i.character);
  }
  e.is = n;
})(ke || (ke = {}));
var X;
(function(e) {
  function t(r, i, s, a) {
    if (E.uinteger(r) && E.uinteger(i) && E.uinteger(s) && E.uinteger(a))
      return { start: ke.create(r, i), end: ke.create(s, a) };
    if (ke.is(r) && ke.is(i))
      return { start: r, end: i };
    throw new Error("Range#create called with invalid arguments[" + r + ", " + i + ", " + s + ", " + a + "]");
  }
  e.create = t;
  function n(r) {
    var i = r;
    return E.objectLiteral(i) && ke.is(i.start) && ke.is(i.end);
  }
  e.is = n;
})(X || (X = {}));
var qt;
(function(e) {
  function t(r, i) {
    return { uri: r, range: i };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return E.defined(i) && X.is(i.range) && (E.string(i.uri) || E.undefined(i.uri));
  }
  e.is = n;
})(qt || (qt = {}));
var ls;
(function(e) {
  function t(r, i, s, a) {
    return { targetUri: r, targetRange: i, targetSelectionRange: s, originSelectionRange: a };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return E.defined(i) && X.is(i.targetRange) && E.string(i.targetUri) && (X.is(i.targetSelectionRange) || E.undefined(i.targetSelectionRange)) && (X.is(i.originSelectionRange) || E.undefined(i.originSelectionRange));
  }
  e.is = n;
})(ls || (ls = {}));
var cr;
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
    return E.numberRange(i.red, 0, 1) && E.numberRange(i.green, 0, 1) && E.numberRange(i.blue, 0, 1) && E.numberRange(i.alpha, 0, 1);
  }
  e.is = n;
})(cr || (cr = {}));
var us;
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
    return X.is(i.range) && cr.is(i.color);
  }
  e.is = n;
})(us || (us = {}));
var cs;
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
    return E.string(i.label) && (E.undefined(i.textEdit) || Ee.is(i)) && (E.undefined(i.additionalTextEdits) || E.typedArray(i.additionalTextEdits, Ee.is));
  }
  e.is = n;
})(cs || (cs = {}));
var Pt;
(function(e) {
  e.Comment = "comment", e.Imports = "imports", e.Region = "region";
})(Pt || (Pt = {}));
var fs;
(function(e) {
  function t(r, i, s, a, o) {
    var l = {
      startLine: r,
      endLine: i
    };
    return E.defined(s) && (l.startCharacter = s), E.defined(a) && (l.endCharacter = a), E.defined(o) && (l.kind = o), l;
  }
  e.create = t;
  function n(r) {
    var i = r;
    return E.uinteger(i.startLine) && E.uinteger(i.startLine) && (E.undefined(i.startCharacter) || E.uinteger(i.startCharacter)) && (E.undefined(i.endCharacter) || E.uinteger(i.endCharacter)) && (E.undefined(i.kind) || E.string(i.kind));
  }
  e.is = n;
})(fs || (fs = {}));
var fr;
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
    return E.defined(i) && qt.is(i.location) && E.string(i.message);
  }
  e.is = n;
})(fr || (fr = {}));
var xe;
(function(e) {
  e.Error = 1, e.Warning = 2, e.Information = 3, e.Hint = 4;
})(xe || (xe = {}));
var hs;
(function(e) {
  e.Unnecessary = 1, e.Deprecated = 2;
})(hs || (hs = {}));
var ds;
(function(e) {
  function t(n) {
    var r = n;
    return r != null && E.string(r.href);
  }
  e.is = t;
})(ds || (ds = {}));
var qe;
(function(e) {
  function t(r, i, s, a, o, l) {
    var u = { range: r, message: i };
    return E.defined(s) && (u.severity = s), E.defined(a) && (u.code = a), E.defined(o) && (u.source = o), E.defined(l) && (u.relatedInformation = l), u;
  }
  e.create = t;
  function n(r) {
    var i, s = r;
    return E.defined(s) && X.is(s.range) && E.string(s.message) && (E.number(s.severity) || E.undefined(s.severity)) && (E.integer(s.code) || E.string(s.code) || E.undefined(s.code)) && (E.undefined(s.codeDescription) || E.string((i = s.codeDescription) === null || i === void 0 ? void 0 : i.href)) && (E.string(s.source) || E.undefined(s.source)) && (E.undefined(s.relatedInformation) || E.typedArray(s.relatedInformation, fr.is));
  }
  e.is = n;
})(qe || (qe = {}));
var Ut;
(function(e) {
  function t(r, i) {
    for (var s = [], a = 2; a < arguments.length; a++)
      s[a - 2] = arguments[a];
    var o = { title: r, command: i };
    return E.defined(s) && s.length > 0 && (o.arguments = s), o;
  }
  e.create = t;
  function n(r) {
    var i = r;
    return E.defined(i) && E.string(i.title) && E.string(i.command);
  }
  e.is = n;
})(Ut || (Ut = {}));
var Ee;
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
    return E.objectLiteral(a) && E.string(a.newText) && X.is(a.range);
  }
  e.is = i;
})(Ee || (Ee = {}));
var xt;
(function(e) {
  function t(r, i, s) {
    var a = { label: r };
    return i !== void 0 && (a.needsConfirmation = i), s !== void 0 && (a.description = s), a;
  }
  e.create = t;
  function n(r) {
    var i = r;
    return i !== void 0 && E.objectLiteral(i) && E.string(i.label) && (E.boolean(i.needsConfirmation) || i.needsConfirmation === void 0) && (E.string(i.description) || i.description === void 0);
  }
  e.is = n;
})(xt || (xt = {}));
var ue;
(function(e) {
  function t(n) {
    var r = n;
    return typeof r == "string";
  }
  e.is = t;
})(ue || (ue = {}));
var Xe;
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
    return Ee.is(a) && (xt.is(a.annotationId) || ue.is(a.annotationId));
  }
  e.is = i;
})(Xe || (Xe = {}));
var bn;
(function(e) {
  function t(r, i) {
    return { textDocument: r, edits: i };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return E.defined(i) && yn.is(i.textDocument) && Array.isArray(i.edits);
  }
  e.is = n;
})(bn || (bn = {}));
var $t;
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
    return i && i.kind === "create" && E.string(i.uri) && (i.options === void 0 || (i.options.overwrite === void 0 || E.boolean(i.options.overwrite)) && (i.options.ignoreIfExists === void 0 || E.boolean(i.options.ignoreIfExists))) && (i.annotationId === void 0 || ue.is(i.annotationId));
  }
  e.is = n;
})($t || ($t = {}));
var Wt;
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
    return i && i.kind === "rename" && E.string(i.oldUri) && E.string(i.newUri) && (i.options === void 0 || (i.options.overwrite === void 0 || E.boolean(i.options.overwrite)) && (i.options.ignoreIfExists === void 0 || E.boolean(i.options.ignoreIfExists))) && (i.annotationId === void 0 || ue.is(i.annotationId));
  }
  e.is = n;
})(Wt || (Wt = {}));
var Ht;
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
    return i && i.kind === "delete" && E.string(i.uri) && (i.options === void 0 || (i.options.recursive === void 0 || E.boolean(i.options.recursive)) && (i.options.ignoreIfNotExists === void 0 || E.boolean(i.options.ignoreIfNotExists))) && (i.annotationId === void 0 || ue.is(i.annotationId));
  }
  e.is = n;
})(Ht || (Ht = {}));
var hr;
(function(e) {
  function t(n) {
    var r = n;
    return r && (r.changes !== void 0 || r.documentChanges !== void 0) && (r.documentChanges === void 0 || r.documentChanges.every(function(i) {
      return E.string(i.kind) ? $t.is(i) || Wt.is(i) || Ht.is(i) : bn.is(i);
    }));
  }
  e.is = t;
})(hr || (hr = {}));
var Zt = function() {
  function e(t, n) {
    this.edits = t, this.changeAnnotations = n;
  }
  return e.prototype.insert = function(t, n, r) {
    var i, s;
    if (r === void 0 ? i = Ee.insert(t, n) : ue.is(r) ? (s = r, i = Xe.insert(t, n, r)) : (this.assertChangeAnnotations(this.changeAnnotations), s = this.changeAnnotations.manage(r), i = Xe.insert(t, n, s)), this.edits.push(i), s !== void 0)
      return s;
  }, e.prototype.replace = function(t, n, r) {
    var i, s;
    if (r === void 0 ? i = Ee.replace(t, n) : ue.is(r) ? (s = r, i = Xe.replace(t, n, r)) : (this.assertChangeAnnotations(this.changeAnnotations), s = this.changeAnnotations.manage(r), i = Xe.replace(t, n, s)), this.edits.push(i), s !== void 0)
      return s;
  }, e.prototype.delete = function(t, n) {
    var r, i;
    if (n === void 0 ? r = Ee.del(t) : ue.is(n) ? (i = n, r = Xe.del(t, n)) : (this.assertChangeAnnotations(this.changeAnnotations), i = this.changeAnnotations.manage(n), r = Xe.del(t, i)), this.edits.push(r), i !== void 0)
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
}(), gs = function() {
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
    if (ue.is(t) ? r = t : (r = this.nextId(), n = t), this._annotations[r] !== void 0)
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
    this._textEditChanges = /* @__PURE__ */ Object.create(null), t !== void 0 ? (this._workspaceEdit = t, t.documentChanges ? (this._changeAnnotations = new gs(t.changeAnnotations), t.changeAnnotations = this._changeAnnotations.all(), t.documentChanges.forEach(function(r) {
      if (bn.is(r)) {
        var i = new Zt(r.edits, n._changeAnnotations);
        n._textEditChanges[r.textDocument.uri] = i;
      }
    })) : t.changes && Object.keys(t.changes).forEach(function(r) {
      var i = new Zt(t.changes[r]);
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
    if (yn.is(t)) {
      if (this.initDocumentChanges(), this._workspaceEdit.documentChanges === void 0)
        throw new Error("Workspace edit is not configured for document changes.");
      var n = { uri: t.uri, version: t.version }, r = this._textEditChanges[n.uri];
      if (!r) {
        var i = [], s = {
          textDocument: n,
          edits: i
        };
        this._workspaceEdit.documentChanges.push(s), r = new Zt(i, this._changeAnnotations), this._textEditChanges[n.uri] = r;
      }
      return r;
    } else {
      if (this.initChanges(), this._workspaceEdit.changes === void 0)
        throw new Error("Workspace edit is not configured for normal text edit changes.");
      var r = this._textEditChanges[t];
      if (!r) {
        var i = [];
        this._workspaceEdit.changes[t] = i, r = new Zt(i), this._textEditChanges[t] = r;
      }
      return r;
    }
  }, e.prototype.initDocumentChanges = function() {
    this._workspaceEdit.documentChanges === void 0 && this._workspaceEdit.changes === void 0 && (this._changeAnnotations = new gs(), this._workspaceEdit.documentChanges = [], this._workspaceEdit.changeAnnotations = this._changeAnnotations.all());
  }, e.prototype.initChanges = function() {
    this._workspaceEdit.documentChanges === void 0 && this._workspaceEdit.changes === void 0 && (this._workspaceEdit.changes = /* @__PURE__ */ Object.create(null));
  }, e.prototype.createFile = function(t, n, r) {
    if (this.initDocumentChanges(), this._workspaceEdit.documentChanges === void 0)
      throw new Error("Workspace edit is not configured for document changes.");
    var i;
    xt.is(n) || ue.is(n) ? i = n : r = n;
    var s, a;
    if (i === void 0 ? s = $t.create(t, r) : (a = ue.is(i) ? i : this._changeAnnotations.manage(i), s = $t.create(t, r, a)), this._workspaceEdit.documentChanges.push(s), a !== void 0)
      return a;
  }, e.prototype.renameFile = function(t, n, r, i) {
    if (this.initDocumentChanges(), this._workspaceEdit.documentChanges === void 0)
      throw new Error("Workspace edit is not configured for document changes.");
    var s;
    xt.is(r) || ue.is(r) ? s = r : i = r;
    var a, o;
    if (s === void 0 ? a = Wt.create(t, n, i) : (o = ue.is(s) ? s : this._changeAnnotations.manage(s), a = Wt.create(t, n, i, o)), this._workspaceEdit.documentChanges.push(a), o !== void 0)
      return o;
  }, e.prototype.deleteFile = function(t, n, r) {
    if (this.initDocumentChanges(), this._workspaceEdit.documentChanges === void 0)
      throw new Error("Workspace edit is not configured for document changes.");
    var i;
    xt.is(n) || ue.is(n) ? i = n : r = n;
    var s, a;
    if (i === void 0 ? s = Ht.create(t, r) : (a = ue.is(i) ? i : this._changeAnnotations.manage(i), s = Ht.create(t, r, a)), this._workspaceEdit.documentChanges.push(s), a !== void 0)
      return a;
  }, e;
})();
var ms;
(function(e) {
  function t(r) {
    return { uri: r };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return E.defined(i) && E.string(i.uri);
  }
  e.is = n;
})(ms || (ms = {}));
var ps;
(function(e) {
  function t(r, i) {
    return { uri: r, version: i };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return E.defined(i) && E.string(i.uri) && E.integer(i.version);
  }
  e.is = n;
})(ps || (ps = {}));
var yn;
(function(e) {
  function t(r, i) {
    return { uri: r, version: i };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return E.defined(i) && E.string(i.uri) && (i.version === null || E.integer(i.version));
  }
  e.is = n;
})(yn || (yn = {}));
var vs;
(function(e) {
  function t(r, i, s, a) {
    return { uri: r, languageId: i, version: s, text: a };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return E.defined(i) && E.string(i.uri) && E.string(i.languageId) && E.integer(i.version) && E.string(i.text);
  }
  e.is = n;
})(vs || (vs = {}));
var Ue;
(function(e) {
  e.PlainText = "plaintext", e.Markdown = "markdown";
})(Ue || (Ue = {}));
(function(e) {
  function t(n) {
    var r = n;
    return r === e.PlainText || r === e.Markdown;
  }
  e.is = t;
})(Ue || (Ue = {}));
var dr;
(function(e) {
  function t(n) {
    var r = n;
    return E.objectLiteral(n) && Ue.is(r.kind) && E.string(r.value);
  }
  e.is = t;
})(dr || (dr = {}));
var ye;
(function(e) {
  e.Text = 1, e.Method = 2, e.Function = 3, e.Constructor = 4, e.Field = 5, e.Variable = 6, e.Class = 7, e.Interface = 8, e.Module = 9, e.Property = 10, e.Unit = 11, e.Value = 12, e.Enum = 13, e.Keyword = 14, e.Snippet = 15, e.Color = 16, e.File = 17, e.Reference = 18, e.Folder = 19, e.EnumMember = 20, e.Constant = 21, e.Struct = 22, e.Event = 23, e.Operator = 24, e.TypeParameter = 25;
})(ye || (ye = {}));
var ne;
(function(e) {
  e.PlainText = 1, e.Snippet = 2;
})(ne || (ne = {}));
var bs;
(function(e) {
  e.Deprecated = 1;
})(bs || (bs = {}));
var ys;
(function(e) {
  function t(r, i, s) {
    return { newText: r, insert: i, replace: s };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return i && E.string(i.newText) && X.is(i.insert) && X.is(i.replace);
  }
  e.is = n;
})(ys || (ys = {}));
var xs;
(function(e) {
  e.asIs = 1, e.adjustIndentation = 2;
})(xs || (xs = {}));
var gr;
(function(e) {
  function t(n) {
    return { label: n };
  }
  e.create = t;
})(gr || (gr = {}));
var ws;
(function(e) {
  function t(n, r) {
    return { items: n || [], isIncomplete: !!r };
  }
  e.create = t;
})(ws || (ws = {}));
var xn;
(function(e) {
  function t(r) {
    return r.replace(/[\\`*_{}[\]()#+\-.!]/g, "\\$&");
  }
  e.fromPlainText = t;
  function n(r) {
    var i = r;
    return E.string(i) || E.objectLiteral(i) && E.string(i.language) && E.string(i.value);
  }
  e.is = n;
})(xn || (xn = {}));
var _s;
(function(e) {
  function t(n) {
    var r = n;
    return !!r && E.objectLiteral(r) && (dr.is(r.contents) || xn.is(r.contents) || E.typedArray(r.contents, xn.is)) && (n.range === void 0 || X.is(n.range));
  }
  e.is = t;
})(_s || (_s = {}));
var Ss;
(function(e) {
  function t(n, r) {
    return r ? { label: n, documentation: r } : { label: n };
  }
  e.create = t;
})(Ss || (Ss = {}));
var Ls;
(function(e) {
  function t(n, r) {
    for (var i = [], s = 2; s < arguments.length; s++)
      i[s - 2] = arguments[s];
    var a = { label: n };
    return E.defined(r) && (a.documentation = r), E.defined(i) ? a.parameters = i : a.parameters = [], a;
  }
  e.create = t;
})(Ls || (Ls = {}));
var Ns;
(function(e) {
  e.Text = 1, e.Read = 2, e.Write = 3;
})(Ns || (Ns = {}));
var As;
(function(e) {
  function t(n, r) {
    var i = { range: n };
    return E.number(r) && (i.kind = r), i;
  }
  e.create = t;
})(As || (As = {}));
var Pe;
(function(e) {
  e.File = 1, e.Module = 2, e.Namespace = 3, e.Package = 4, e.Class = 5, e.Method = 6, e.Property = 7, e.Field = 8, e.Constructor = 9, e.Enum = 10, e.Interface = 11, e.Function = 12, e.Variable = 13, e.Constant = 14, e.String = 15, e.Number = 16, e.Boolean = 17, e.Array = 18, e.Object = 19, e.Key = 20, e.Null = 21, e.EnumMember = 22, e.Struct = 23, e.Event = 24, e.Operator = 25, e.TypeParameter = 26;
})(Pe || (Pe = {}));
var Cs;
(function(e) {
  e.Deprecated = 1;
})(Cs || (Cs = {}));
var ks;
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
})(ks || (ks = {}));
var Rs;
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
    return i && E.string(i.name) && E.number(i.kind) && X.is(i.range) && X.is(i.selectionRange) && (i.detail === void 0 || E.string(i.detail)) && (i.deprecated === void 0 || E.boolean(i.deprecated)) && (i.children === void 0 || Array.isArray(i.children)) && (i.tags === void 0 || Array.isArray(i.tags));
  }
  e.is = n;
})(Rs || (Rs = {}));
var Es;
(function(e) {
  e.Empty = "", e.QuickFix = "quickfix", e.Refactor = "refactor", e.RefactorExtract = "refactor.extract", e.RefactorInline = "refactor.inline", e.RefactorRewrite = "refactor.rewrite", e.Source = "source", e.SourceOrganizeImports = "source.organizeImports", e.SourceFixAll = "source.fixAll";
})(Es || (Es = {}));
var Ms;
(function(e) {
  function t(r, i) {
    var s = { diagnostics: r };
    return i != null && (s.only = i), s;
  }
  e.create = t;
  function n(r) {
    var i = r;
    return E.defined(i) && E.typedArray(i.diagnostics, qe.is) && (i.only === void 0 || E.typedArray(i.only, E.string));
  }
  e.is = n;
})(Ms || (Ms = {}));
var Ts;
(function(e) {
  function t(r, i, s) {
    var a = { title: r }, o = !0;
    return typeof i == "string" ? (o = !1, a.kind = i) : Ut.is(i) ? a.command = i : a.edit = i, o && s !== void 0 && (a.kind = s), a;
  }
  e.create = t;
  function n(r) {
    var i = r;
    return i && E.string(i.title) && (i.diagnostics === void 0 || E.typedArray(i.diagnostics, qe.is)) && (i.kind === void 0 || E.string(i.kind)) && (i.edit !== void 0 || i.command !== void 0) && (i.command === void 0 || Ut.is(i.command)) && (i.isPreferred === void 0 || E.boolean(i.isPreferred)) && (i.edit === void 0 || hr.is(i.edit));
  }
  e.is = n;
})(Ts || (Ts = {}));
var Ps;
(function(e) {
  function t(r, i) {
    var s = { range: r };
    return E.defined(i) && (s.data = i), s;
  }
  e.create = t;
  function n(r) {
    var i = r;
    return E.defined(i) && X.is(i.range) && (E.undefined(i.command) || Ut.is(i.command));
  }
  e.is = n;
})(Ps || (Ps = {}));
var Fs;
(function(e) {
  function t(r, i) {
    return { tabSize: r, insertSpaces: i };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return E.defined(i) && E.uinteger(i.tabSize) && E.boolean(i.insertSpaces);
  }
  e.is = n;
})(Fs || (Fs = {}));
var Is;
(function(e) {
  function t(r, i, s) {
    return { range: r, target: i, data: s };
  }
  e.create = t;
  function n(r) {
    var i = r;
    return E.defined(i) && X.is(i.range) && (E.undefined(i.target) || E.string(i.target));
  }
  e.is = n;
})(Is || (Is = {}));
var wn;
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
})(wn || (wn = {}));
var Vs;
(function(e) {
  function t(s, a, o, l) {
    return new Ql(s, a, o, l);
  }
  e.create = t;
  function n(s) {
    var a = s;
    return !!(E.defined(a) && E.string(a.uri) && (E.undefined(a.languageId) || E.string(a.languageId)) && E.uinteger(a.lineCount) && E.func(a.getText) && E.func(a.positionAt) && E.func(a.offsetAt));
  }
  e.is = n;
  function r(s, a) {
    for (var o = s.getText(), l = i(a, function(m, p) {
      var b = m.range.start.line - p.range.start.line;
      return b === 0 ? m.range.start.character - p.range.start.character : b;
    }), u = o.length, h = l.length - 1; h >= 0; h--) {
      var f = l[h], d = s.offsetAt(f.range.start), g = s.offsetAt(f.range.end);
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
    for (var h = 0, f = 0, d = 0; h < l.length && f < u.length; ) {
      var g = a(l[h], u[f]);
      g <= 0 ? s[d++] = l[h++] : s[d++] = u[f++];
    }
    for (; h < l.length; )
      s[d++] = l[h++];
    for (; f < u.length; )
      s[d++] = u[f++];
    return s;
  }
})(Vs || (Vs = {}));
var Ql = function() {
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
}(), E;
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
  function h(g) {
    return t.call(g) === "[object Function]";
  }
  e.func = h;
  function f(g) {
    return g !== null && typeof g == "object";
  }
  e.objectLiteral = f;
  function d(g, m) {
    return Array.isArray(g) && g.every(m);
  }
  e.typedArray = d;
})(E || (E = {}));
var _n = class {
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
      if (_n.isIncremental(n)) {
        const r = fa(n.range), i = this.offsetAt(r.start), s = this.offsetAt(r.end);
        this._content = this._content.substring(0, i) + n.text + this._content.substring(s, this._content.length);
        const a = Math.max(r.start.line, 0), o = Math.max(r.end.line, 0);
        let l = this._lineOffsets;
        const u = Ds(n.text, !1, i);
        if (o - a === u.length)
          for (let f = 0, d = u.length; f < d; f++)
            l[f + a + 1] = u[f];
        else
          u.length < 1e4 ? l.splice(a + 1, o - a, ...u) : this._lineOffsets = l = l.slice(0, a + 1).concat(u, l.slice(o + 1));
        const h = n.text.length - (s - i);
        if (h !== 0)
          for (let f = a + 1 + u.length, d = l.length; f < d; f++)
            l[f] = l[f] + h;
      } else if (_n.isFull(n))
        this._content = n.text, this._lineOffsets = void 0;
      else
        throw new Error("Unknown change event received");
    this._version = t;
  }
  getLineOffsets() {
    return this._lineOffsets === void 0 && (this._lineOffsets = Ds(this._content, !0)), this._lineOffsets;
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
}, mr;
(function(e) {
  function t(i, s, a, o) {
    return new _n(i, s, a, o);
  }
  e.create = t;
  function n(i, s, a) {
    if (i instanceof _n)
      return i.update(s, a), i;
    throw new Error("TextDocument.update: document must be created by TextDocument.create");
  }
  e.update = n;
  function r(i, s) {
    let a = i.getText(), o = pr(s.map(Zl), (h, f) => {
      let d = h.range.start.line - f.range.start.line;
      return d === 0 ? h.range.start.character - f.range.start.character : d;
    }), l = 0;
    const u = [];
    for (const h of o) {
      let f = i.offsetAt(h.range.start);
      if (f < l)
        throw new Error("Overlapping edit");
      f > l && u.push(a.substring(l, f)), h.newText.length && u.push(h.newText), l = i.offsetAt(h.range.end);
    }
    return u.push(a.substr(l)), u.join("");
  }
  e.applyEdits = r;
})(mr || (mr = {}));
function pr(e, t) {
  if (e.length <= 1)
    return e;
  const n = e.length / 2 | 0, r = e.slice(0, n), i = e.slice(n);
  pr(r, t), pr(i, t);
  let s = 0, a = 0, o = 0;
  for (; s < r.length && a < i.length; )
    t(r[s], i[a]) <= 0 ? e[o++] = r[s++] : e[o++] = i[a++];
  for (; s < r.length; )
    e[o++] = r[s++];
  for (; a < i.length; )
    e[o++] = i[a++];
  return e;
}
function Ds(e, t, n = 0) {
  const r = t ? [n] : [];
  for (let i = 0; i < e.length; i++) {
    let s = e.charCodeAt(i);
    (s === 13 || s === 10) && (s === 13 && i + 1 < e.length && e.charCodeAt(i + 1) === 10 && i++, r.push(n + i + 1));
  }
  return r;
}
function fa(e) {
  const t = e.start, n = e.end;
  return t.line > n.line || t.line === n.line && t.character > n.character ? { start: n, end: t } : e;
}
function Zl(e) {
  const t = fa(e.range);
  return t !== e.range ? { newText: e.newText, range: t } : e;
}
var G;
(function(e) {
  e[e.Undefined = 0] = "Undefined", e[e.EnumValueMismatch = 1] = "EnumValueMismatch", e[e.Deprecated = 2] = "Deprecated", e[e.UnexpectedEndOfComment = 257] = "UnexpectedEndOfComment", e[e.UnexpectedEndOfString = 258] = "UnexpectedEndOfString", e[e.UnexpectedEndOfNumber = 259] = "UnexpectedEndOfNumber", e[e.InvalidUnicode = 260] = "InvalidUnicode", e[e.InvalidEscapeCharacter = 261] = "InvalidEscapeCharacter", e[e.InvalidCharacter = 262] = "InvalidCharacter", e[e.PropertyExpected = 513] = "PropertyExpected", e[e.CommaExpected = 514] = "CommaExpected", e[e.ColonExpected = 515] = "ColonExpected", e[e.ValueExpected = 516] = "ValueExpected", e[e.CommaOrCloseBacketExpected = 517] = "CommaOrCloseBacketExpected", e[e.CommaOrCloseBraceExpected = 518] = "CommaOrCloseBraceExpected", e[e.TrailingComma = 519] = "TrailingComma", e[e.DuplicateKey = 520] = "DuplicateKey", e[e.CommentNotPermitted = 521] = "CommentNotPermitted", e[e.SchemaResolveError = 768] = "SchemaResolveError";
})(G || (G = {}));
var Os;
(function(e) {
  e.LATEST = {
    textDocument: {
      completion: {
        completionItem: {
          documentationFormat: [Ue.Markdown, Ue.PlainText],
          commitCharactersSupport: !0
        }
      }
    }
  };
})(Os || (Os = {}));
function Yl(e, t) {
  let n;
  return t.length === 0 ? n = e : n = e.replace(/\{(\d+)\}/g, (r, i) => {
    let s = i[0];
    return typeof t[s] < "u" ? t[s] : r;
  }), n;
}
function Kl(e, t, ...n) {
  return Yl(t, n);
}
function Gt(e) {
  return Kl;
}
var at = function() {
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
}(), D = Gt(), eu = {
  "color-hex": { errorMessage: D("colorHexFormatWarning", "Invalid color format. Use #RGB, #RGBA, #RRGGBB or #RRGGBBAA."), pattern: /^#([0-9A-Fa-f]{3,4}|([0-9A-Fa-f]{2}){3,4})$/ },
  "date-time": { errorMessage: D("dateTimeFormatWarning", "String is not a RFC3339 date-time."), pattern: /^(\d{4})-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])T([01][0-9]|2[0-3]):([0-5][0-9]):([0-5][0-9]|60)(\.[0-9]+)?(Z|(\+|-)([01][0-9]|2[0-3]):([0-5][0-9]))$/i },
  date: { errorMessage: D("dateFormatWarning", "String is not a RFC3339 date."), pattern: /^(\d{4})-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])$/i },
  time: { errorMessage: D("timeFormatWarning", "String is not a RFC3339 time."), pattern: /^([01][0-9]|2[0-3]):([0-5][0-9]):([0-5][0-9]|60)(\.[0-9]+)?(Z|(\+|-)([01][0-9]|2[0-3]):([0-5][0-9]))$/i },
  email: { errorMessage: D("emailFormatWarning", "String is not an e-mail address."), pattern: /^(([^<>()\[\]\\.,;:\s@"]+(\.[^<>()\[\]\\.,;:\s@"]+)*)|(".+"))@((\[[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}])|(([a-zA-Z0-9-]+\.)+[a-zA-Z]{2,}))$/ },
  hostname: { errorMessage: D("hostnameFormatWarning", "String is not a hostname."), pattern: /^(?=.{1,253}\.?$)[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[-0-9a-z]{0,61}[0-9a-z])?)*\.?$/i },
  ipv4: { errorMessage: D("ipv4FormatWarning", "String is not an IPv4 address."), pattern: /^(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)$/ },
  ipv6: { errorMessage: D("ipv6FormatWarning", "String is not an IPv6 address."), pattern: /^((([0-9a-f]{1,4}:){7}([0-9a-f]{1,4}|:))|(([0-9a-f]{1,4}:){6}(:[0-9a-f]{1,4}|((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3})|:))|(([0-9a-f]{1,4}:){5}(((:[0-9a-f]{1,4}){1,2})|:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3})|:))|(([0-9a-f]{1,4}:){4}(((:[0-9a-f]{1,4}){1,3})|((:[0-9a-f]{1,4})?:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9a-f]{1,4}:){3}(((:[0-9a-f]{1,4}){1,4})|((:[0-9a-f]{1,4}){0,2}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9a-f]{1,4}:){2}(((:[0-9a-f]{1,4}){1,5})|((:[0-9a-f]{1,4}){0,3}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9a-f]{1,4}:){1}(((:[0-9a-f]{1,4}){1,6})|((:[0-9a-f]{1,4}){0,4}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(:(((:[0-9a-f]{1,4}){1,7})|((:[0-9a-f]{1,4}){0,5}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:)))$/i }
}, ot = function() {
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
}(), tu = function(e) {
  at(t, e);
  function t(n, r) {
    var i = e.call(this, n, r) || this;
    return i.type = "null", i.value = null, i;
  }
  return t;
}(ot), js = function(e) {
  at(t, e);
  function t(n, r, i) {
    var s = e.call(this, n, i) || this;
    return s.type = "boolean", s.value = r, s;
  }
  return t;
}(ot), nu = function(e) {
  at(t, e);
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
}(ot), ru = function(e) {
  at(t, e);
  function t(n, r) {
    var i = e.call(this, n, r) || this;
    return i.type = "number", i.isInteger = !0, i.value = Number.NaN, i;
  }
  return t;
}(ot), On = function(e) {
  at(t, e);
  function t(n, r, i) {
    var s = e.call(this, n, r, i) || this;
    return s.type = "string", s.value = "", s;
  }
  return t;
}(ot), iu = function(e) {
  at(t, e);
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
}(ot), su = function(e) {
  at(t, e);
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
}(ot);
function de(e) {
  return Fe(e) ? e ? {} : { not: {} } : e;
}
var Bs;
(function(e) {
  e[e.Key = 0] = "Key", e[e.Enum = 1] = "Enum";
})(Bs || (Bs = {}));
var au = function() {
  function e(t, n) {
    t === void 0 && (t = -1), this.focusOffset = t, this.exclude = n, this.schemas = [];
  }
  return e.prototype.add = function(t) {
    this.schemas.push(t);
  }, e.prototype.merge = function(t) {
    Array.prototype.push.apply(this.schemas, t.schemas);
  }, e.prototype.include = function(t) {
    return (this.focusOffset === -1 || ha(t, this.focusOffset)) && t !== this.exclude;
  }, e.prototype.newSub = function() {
    return new e(-1, this.exclude);
  }, e;
}(), vr = function() {
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
}(), ge = function() {
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
        i.code === G.EnumValueMismatch && (i.message = D("enumWarning", "Value is not accepted. Valid values: {0}.", this.enumValues.map(function(s) {
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
function ou(e, t) {
  return t === void 0 && (t = []), new da(e, t, []);
}
function rt(e) {
  return zl(e);
}
function br(e) {
  return Hl(e);
}
function ha(e, t, n) {
  return n === void 0 && (n = !1), t >= e.offset && t < e.offset + e.length || n && t === e.offset + e.length;
}
var da = function() {
  function e(t, n, r) {
    n === void 0 && (n = []), r === void 0 && (r = []), this.root = t, this.syntaxErrors = n, this.comments = r;
  }
  return e.prototype.getNodeFromOffset = function(t, n) {
    if (n === void 0 && (n = !1), this.root)
      return Wl(this.root, t, n);
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
    if (r === void 0 && (r = xe.Warning), this.root && n) {
      var i = new ge();
      return oe(this.root, n, i, vr.instance), i.problems.map(function(s) {
        var a, o = X.create(t.positionAt(s.location.offset), t.positionAt(s.location.offset + s.location.length));
        return qe.create(o, s.message, (a = s.severity) !== null && a !== void 0 ? a : r, s.code);
      });
    }
  }, e.prototype.getMatchingSchemas = function(t, n, r) {
    n === void 0 && (n = -1);
    var i = new au(n, r);
    return this.root && t && oe(this.root, t, new ge(), i), i.schemas;
  }, e;
}();
function oe(e, t, n, r) {
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
      return oe(i.valueNode, t, n, r);
  }
  s(), r.add({ node: i, schema: t });
  function s() {
    function h(T) {
      return i.type === T || T === "integer" && i.type === "number" && i.isInteger;
    }
    if (Array.isArray(t.type) ? t.type.some(h) || n.problems.push({
      location: { offset: i.offset, length: i.length },
      message: t.errorMessage || D("typeArrayMismatchWarning", "Incorrect type. Expected one of {0}.", t.type.join(", "))
    }) : t.type && (h(t.type) || n.problems.push({
      location: { offset: i.offset, length: i.length },
      message: t.errorMessage || D("typeMismatchWarning", 'Incorrect type. Expected "{0}".', t.type)
    })), Array.isArray(t.allOf))
      for (var f = 0, d = t.allOf; f < d.length; f++) {
        var g = d[f];
        oe(i, de(g), n, r);
      }
    var m = de(t.not);
    if (m) {
      var p = new ge(), b = r.newSub();
      oe(i, m, p, b), p.hasProblems() || n.problems.push({
        location: { offset: i.offset, length: i.length },
        message: D("notSchemaWarning", "Matches a schema that is not allowed.")
      });
      for (var y = 0, x = b.schemas; y < x.length; y++) {
        var v = x[y];
        v.inverted = !v.inverted, r.add(v);
      }
    }
    var C = function(T, V) {
      for (var q = [], B = void 0, P = 0, M = T; P < M.length; P++) {
        var F = M[P], I = de(F), j = new ge(), U = r.newSub();
        if (oe(i, I, j, U), j.hasProblems() || q.push(I), !B)
          B = { schema: I, validationResult: j, matchingSchemas: U };
        else if (!V && !j.hasProblems() && !B.validationResult.hasProblems())
          B.matchingSchemas.merge(U), B.validationResult.propertiesMatches += j.propertiesMatches, B.validationResult.propertiesValueMatches += j.propertiesValueMatches;
        else {
          var H = j.compare(B.validationResult);
          H > 0 ? B = { schema: I, validationResult: j, matchingSchemas: U } : H === 0 && (B.matchingSchemas.merge(U), B.validationResult.mergeEnumValues(j));
        }
      }
      return q.length > 1 && V && n.problems.push({
        location: { offset: i.offset, length: 1 },
        message: D("oneOfWarning", "Matches multiple schemas when only one must validate.")
      }), B && (n.merge(B.validationResult), n.propertiesMatches += B.validationResult.propertiesMatches, n.propertiesValueMatches += B.validationResult.propertiesValueMatches, r.merge(B.matchingSchemas)), q.length;
    };
    Array.isArray(t.anyOf) && C(t.anyOf, !1), Array.isArray(t.oneOf) && C(t.oneOf, !0);
    var L = function(T) {
      var V = new ge(), q = r.newSub();
      oe(i, de(T), V, q), n.merge(V), n.propertiesMatches += V.propertiesMatches, n.propertiesValueMatches += V.propertiesValueMatches, r.merge(q);
    }, _ = function(T, V, q) {
      var B = de(T), P = new ge(), M = r.newSub();
      oe(i, B, P, M), r.merge(M), P.hasProblems() ? q && L(q) : V && L(V);
    }, S = de(t.if);
    if (S && _(S, de(t.then), de(t.else)), Array.isArray(t.enum)) {
      for (var A = rt(i), R = !1, k = 0, w = t.enum; k < w.length; k++) {
        var N = w[k];
        if (Tt(A, N)) {
          R = !0;
          break;
        }
      }
      n.enumValues = t.enum, n.enumValueMatch = R, R || n.problems.push({
        location: { offset: i.offset, length: i.length },
        code: G.EnumValueMismatch,
        message: t.errorMessage || D("enumWarning", "Value is not accepted. Valid values: {0}.", t.enum.map(function(T) {
          return JSON.stringify(T);
        }).join(", "))
      });
    }
    if (Oe(t.const)) {
      var A = rt(i);
      Tt(A, t.const) ? n.enumValueMatch = !0 : (n.problems.push({
        location: { offset: i.offset, length: i.length },
        code: G.EnumValueMismatch,
        message: t.errorMessage || D("constWarning", "Value must be {0}.", JSON.stringify(t.const))
      }), n.enumValueMatch = !1), n.enumValues = [t.const];
    }
    t.deprecationMessage && i.parent && n.problems.push({
      location: { offset: i.parent.offset, length: i.parent.length },
      severity: xe.Warning,
      message: t.deprecationMessage,
      code: G.Deprecated
    });
  }
  function a(h, f, d, g) {
    var m = h.value;
    function p(k) {
      var w, N = /^(-?\d+)(?:\.(\d+))?(?:e([-+]\d+))?$/.exec(k.toString());
      return N && {
        value: Number(N[1] + (N[2] || "")),
        multiplier: (((w = N[2]) === null || w === void 0 ? void 0 : w.length) || 0) - (parseInt(N[3]) || 0)
      };
    }
    if (ve(f.multipleOf)) {
      var b = -1;
      if (Number.isInteger(f.multipleOf))
        b = m % f.multipleOf;
      else {
        var y = p(f.multipleOf), x = p(m);
        if (y && x) {
          var v = Math.pow(10, Math.abs(x.multiplier - y.multiplier));
          x.multiplier < y.multiplier ? x.value *= v : y.value *= v, b = x.value % y.value;
        }
      }
      b !== 0 && d.problems.push({
        location: { offset: h.offset, length: h.length },
        message: D("multipleOfWarning", "Value is not divisible by {0}.", f.multipleOf)
      });
    }
    function C(k, w) {
      if (ve(w))
        return w;
      if (Fe(w) && w)
        return k;
    }
    function L(k, w) {
      if (!Fe(w) || !w)
        return k;
    }
    var _ = C(f.minimum, f.exclusiveMinimum);
    ve(_) && m <= _ && d.problems.push({
      location: { offset: h.offset, length: h.length },
      message: D("exclusiveMinimumWarning", "Value is below the exclusive minimum of {0}.", _)
    });
    var S = C(f.maximum, f.exclusiveMaximum);
    ve(S) && m >= S && d.problems.push({
      location: { offset: h.offset, length: h.length },
      message: D("exclusiveMaximumWarning", "Value is above the exclusive maximum of {0}.", S)
    });
    var A = L(f.minimum, f.exclusiveMinimum);
    ve(A) && m < A && d.problems.push({
      location: { offset: h.offset, length: h.length },
      message: D("minimumWarning", "Value is below the minimum of {0}.", A)
    });
    var R = L(f.maximum, f.exclusiveMaximum);
    ve(R) && m > R && d.problems.push({
      location: { offset: h.offset, length: h.length },
      message: D("maximumWarning", "Value is above the maximum of {0}.", R)
    });
  }
  function o(h, f, d, g) {
    if (ve(f.minLength) && h.value.length < f.minLength && d.problems.push({
      location: { offset: h.offset, length: h.length },
      message: D("minLengthWarning", "String is shorter than the minimum length of {0}.", f.minLength)
    }), ve(f.maxLength) && h.value.length > f.maxLength && d.problems.push({
      location: { offset: h.offset, length: h.length },
      message: D("maxLengthWarning", "String is longer than the maximum length of {0}.", f.maxLength)
    }), Jl(f.pattern)) {
      var m = pn(f.pattern);
      m != null && m.test(h.value) || d.problems.push({
        location: { offset: h.offset, length: h.length },
        message: f.patternErrorMessage || f.errorMessage || D("patternWarning", 'String does not match the pattern of "{0}".', f.pattern)
      });
    }
    if (f.format)
      switch (f.format) {
        case "uri":
        case "uri-reference":
          {
            var p = void 0;
            if (!h.value)
              p = D("uriEmpty", "URI expected.");
            else {
              var b = /^(([^:/?#]+?):)?(\/\/([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?/.exec(h.value);
              b ? !b[2] && f.format === "uri" && (p = D("uriSchemeMissing", "URI with a scheme is expected.")) : p = D("uriMissing", "URI is expected.");
            }
            p && d.problems.push({
              location: { offset: h.offset, length: h.length },
              message: f.patternErrorMessage || f.errorMessage || D("uriFormatWarning", "String is not a URI: {0}", p)
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
          var y = eu[f.format];
          (!h.value || !y.pattern.exec(h.value)) && d.problems.push({
            location: { offset: h.offset, length: h.length },
            message: f.patternErrorMessage || f.errorMessage || y.errorMessage
          });
      }
  }
  function l(h, f, d, g) {
    if (Array.isArray(f.items)) {
      for (var m = f.items, p = 0; p < m.length; p++) {
        var b = m[p], y = de(b), x = new ge(), v = h.items[p];
        v ? (oe(v, y, x, g), d.mergePropertyMatch(x)) : h.items.length >= m.length && d.propertiesValueMatches++;
      }
      if (h.items.length > m.length)
        if (typeof f.additionalItems == "object")
          for (var C = m.length; C < h.items.length; C++) {
            var x = new ge();
            oe(h.items[C], f.additionalItems, x, g), d.mergePropertyMatch(x);
          }
        else
          f.additionalItems === !1 && d.problems.push({
            location: { offset: h.offset, length: h.length },
            message: D("additionalItemsWarning", "Array has too many items according to schema. Expected {0} or fewer.", m.length)
          });
    } else {
      var L = de(f.items);
      if (L)
        for (var _ = 0, S = h.items; _ < S.length; _++) {
          var v = S[_], x = new ge();
          oe(v, L, x, g), d.mergePropertyMatch(x);
        }
    }
    var A = de(f.contains);
    if (A) {
      var R = h.items.some(function(N) {
        var T = new ge();
        return oe(N, A, T, vr.instance), !T.hasProblems();
      });
      R || d.problems.push({
        location: { offset: h.offset, length: h.length },
        message: f.errorMessage || D("requiredItemMissingWarning", "Array does not contain required item.")
      });
    }
    if (ve(f.minItems) && h.items.length < f.minItems && d.problems.push({
      location: { offset: h.offset, length: h.length },
      message: D("minItemsWarning", "Array has too few items. Expected {0} or more.", f.minItems)
    }), ve(f.maxItems) && h.items.length > f.maxItems && d.problems.push({
      location: { offset: h.offset, length: h.length },
      message: D("maxItemsWarning", "Array has too many items. Expected {0} or fewer.", f.maxItems)
    }), f.uniqueItems === !0) {
      var k = rt(h), w = k.some(function(N, T) {
        return T !== k.lastIndexOf(N);
      });
      w && d.problems.push({
        location: { offset: h.offset, length: h.length },
        message: D("uniqueItemsWarning", "Array has duplicate items.")
      });
    }
  }
  function u(h, f, d, g) {
    for (var m = /* @__PURE__ */ Object.create(null), p = [], b = 0, y = h.properties; b < y.length; b++) {
      var x = y[b], v = x.keyNode.value;
      m[v] = x.valueNode, p.push(v);
    }
    if (Array.isArray(f.required))
      for (var C = 0, L = f.required; C < L.length; C++) {
        var _ = L[C];
        if (!m[_]) {
          var S = h.parent && h.parent.type === "property" && h.parent.keyNode, A = S ? { offset: S.offset, length: S.length } : { offset: h.offset, length: 1 };
          d.problems.push({
            location: A,
            message: D("MissingRequiredPropWarning", 'Missing property "{0}".', _)
          });
        }
      }
    var R = function(Ir) {
      for (var An = p.indexOf(Ir); An >= 0; )
        p.splice(An, 1), An = p.indexOf(Ir);
    };
    if (f.properties)
      for (var k = 0, w = Object.keys(f.properties); k < w.length; k++) {
        var _ = w[k];
        R(_);
        var N = f.properties[_], T = m[_];
        if (T)
          if (Fe(N))
            if (N)
              d.propertiesMatches++, d.propertiesValueMatches++;
            else {
              var x = T.parent;
              d.problems.push({
                location: { offset: x.keyNode.offset, length: x.keyNode.length },
                message: f.errorMessage || D("DisallowedExtraPropWarning", "Property {0} is not allowed.", _)
              });
            }
          else {
            var V = new ge();
            oe(T, N, V, g), d.mergePropertyMatch(V);
          }
      }
    if (f.patternProperties)
      for (var q = 0, B = Object.keys(f.patternProperties); q < B.length; q++)
        for (var P = B[q], M = pn(P), F = 0, I = p.slice(0); F < I.length; F++) {
          var _ = I[F];
          if (M != null && M.test(_)) {
            R(_);
            var T = m[_];
            if (T) {
              var N = f.patternProperties[P];
              if (Fe(N))
                if (N)
                  d.propertiesMatches++, d.propertiesValueMatches++;
                else {
                  var x = T.parent;
                  d.problems.push({
                    location: { offset: x.keyNode.offset, length: x.keyNode.length },
                    message: f.errorMessage || D("DisallowedExtraPropWarning", "Property {0} is not allowed.", _)
                  });
                }
              else {
                var V = new ge();
                oe(T, N, V, g), d.mergePropertyMatch(V);
              }
            }
          }
        }
    if (typeof f.additionalProperties == "object")
      for (var j = 0, U = p; j < U.length; j++) {
        var _ = U[j], T = m[_];
        if (T) {
          var V = new ge();
          oe(T, f.additionalProperties, V, g), d.mergePropertyMatch(V);
        }
      }
    else if (f.additionalProperties === !1 && p.length > 0)
      for (var H = 0, we = p; H < we.length; H++) {
        var _ = we[H], T = m[_];
        if (T) {
          var x = T.parent;
          d.problems.push({
            location: { offset: x.keyNode.offset, length: x.keyNode.length },
            message: f.errorMessage || D("DisallowedExtraPropWarning", "Property {0} is not allowed.", _)
          });
        }
      }
    if (ve(f.maxProperties) && h.properties.length > f.maxProperties && d.problems.push({
      location: { offset: h.offset, length: h.length },
      message: D("MaxPropWarning", "Object has more properties than limit of {0}.", f.maxProperties)
    }), ve(f.minProperties) && h.properties.length < f.minProperties && d.problems.push({
      location: { offset: h.offset, length: h.length },
      message: D("MinPropWarning", "Object has fewer properties than the required number of {0}", f.minProperties)
    }), f.dependencies)
      for (var se = 0, _e = Object.keys(f.dependencies); se < _e.length; se++) {
        var v = _e[se], lt = m[v];
        if (lt) {
          var Me = f.dependencies[v];
          if (Array.isArray(Me))
            for (var Ln = 0, Mr = Me; Ln < Mr.length; Ln++) {
              var Tr = Mr[Ln];
              m[Tr] ? d.propertiesValueMatches++ : d.problems.push({
                location: { offset: h.offset, length: h.length },
                message: D("RequiredDependentPropWarning", "Object is missing property {0} required by property {1}.", Tr, v)
              });
            }
          else {
            var N = de(Me);
            if (N) {
              var V = new ge();
              oe(h, N, V, g), d.mergePropertyMatch(V);
            }
          }
        }
      }
    var Pr = de(f.propertyNames);
    if (Pr)
      for (var Nn = 0, Fr = h.properties; Nn < Fr.length; Nn++) {
        var ya = Fr[Nn], v = ya.keyNode;
        v && oe(v, Pr, d, vr.instance);
      }
  }
}
function lu(e, t) {
  var n = [], r = -1, i = e.getText(), s = yt(i, !1), a = t && t.collectComments ? [] : void 0;
  function o() {
    for (; ; ) {
      var _ = s.scan();
      switch (h(), _) {
        case 12:
        case 13:
          Array.isArray(a) && a.push(X.create(e.positionAt(s.getTokenOffset()), e.positionAt(s.getTokenOffset() + s.getTokenLength())));
          break;
        case 15:
        case 14:
          break;
        default:
          return _;
      }
    }
  }
  function l(_, S, A, R, k) {
    if (k === void 0 && (k = xe.Error), n.length === 0 || A !== r) {
      var w = X.create(e.positionAt(A), e.positionAt(R));
      n.push(qe.create(w, _, k, S, e.languageId)), r = A;
    }
  }
  function u(_, S, A, R, k) {
    A === void 0 && (A = void 0), R === void 0 && (R = []), k === void 0 && (k = []);
    var w = s.getTokenOffset(), N = s.getTokenOffset() + s.getTokenLength();
    if (w === N && w > 0) {
      for (w--; w > 0 && /\s/.test(i.charAt(w)); )
        w--;
      N = w + 1;
    }
    if (l(_, S, w, N), A && f(A, !1), R.length + k.length > 0)
      for (var T = s.getToken(); T !== 17; ) {
        if (R.indexOf(T) !== -1) {
          o();
          break;
        } else if (k.indexOf(T) !== -1)
          break;
        T = o();
      }
    return A;
  }
  function h() {
    switch (s.getTokenError()) {
      case 4:
        return u(D("InvalidUnicode", "Invalid unicode sequence in string."), G.InvalidUnicode), !0;
      case 5:
        return u(D("InvalidEscapeCharacter", "Invalid escape character in string."), G.InvalidEscapeCharacter), !0;
      case 3:
        return u(D("UnexpectedEndOfNumber", "Unexpected end of number."), G.UnexpectedEndOfNumber), !0;
      case 1:
        return u(D("UnexpectedEndOfComment", "Unexpected end of comment."), G.UnexpectedEndOfComment), !0;
      case 2:
        return u(D("UnexpectedEndOfString", "Unexpected end of string."), G.UnexpectedEndOfString), !0;
      case 6:
        return u(D("InvalidCharacter", "Invalid characters in string. Control characters must be escaped."), G.InvalidCharacter), !0;
    }
    return !1;
  }
  function f(_, S) {
    return _.length = s.getTokenOffset() + s.getTokenLength() - _.offset, S && o(), _;
  }
  function d(_) {
    if (s.getToken() === 3) {
      var S = new nu(_, s.getTokenOffset());
      o();
      for (var A = !1; s.getToken() !== 4 && s.getToken() !== 17; ) {
        if (s.getToken() === 5) {
          A || u(D("ValueExpected", "Value expected"), G.ValueExpected);
          var R = s.getTokenOffset();
          if (o(), s.getToken() === 4) {
            A && l(D("TrailingComma", "Trailing comma"), G.TrailingComma, R, R + 1);
            continue;
          }
        } else
          A && u(D("ExpectedComma", "Expected comma"), G.CommaExpected);
        var k = v(S);
        k ? S.items.push(k) : u(D("PropertyExpected", "Value expected"), G.ValueExpected, void 0, [], [4, 5]), A = !0;
      }
      return s.getToken() !== 4 ? u(D("ExpectedCloseBracket", "Expected comma or closing bracket"), G.CommaOrCloseBacketExpected, S) : f(S, !0);
    }
  }
  var g = new On(void 0, 0, 0);
  function m(_, S) {
    var A = new iu(_, s.getTokenOffset(), g), R = b(A);
    if (!R)
      if (s.getToken() === 16) {
        u(D("DoubleQuotesExpected", "Property keys must be doublequoted"), G.Undefined);
        var k = new On(A, s.getTokenOffset(), s.getTokenLength());
        k.value = s.getTokenValue(), R = k, o();
      } else
        return;
    A.keyNode = R;
    var w = S[R.value];
    if (w ? (l(D("DuplicateKeyWarning", "Duplicate object key"), G.DuplicateKey, A.keyNode.offset, A.keyNode.offset + A.keyNode.length, xe.Warning), typeof w == "object" && l(D("DuplicateKeyWarning", "Duplicate object key"), G.DuplicateKey, w.keyNode.offset, w.keyNode.offset + w.keyNode.length, xe.Warning), S[R.value] = !0) : S[R.value] = A, s.getToken() === 6)
      A.colonOffset = s.getTokenOffset(), o();
    else if (u(D("ColonExpected", "Colon expected"), G.ColonExpected), s.getToken() === 10 && e.positionAt(R.offset + R.length).line < e.positionAt(s.getTokenOffset()).line)
      return A.length = R.length, A;
    var N = v(A);
    return N ? (A.valueNode = N, A.length = N.offset + N.length - A.offset, A) : u(D("ValueExpected", "Value expected"), G.ValueExpected, A, [], [2, 5]);
  }
  function p(_) {
    if (s.getToken() === 1) {
      var S = new su(_, s.getTokenOffset()), A = /* @__PURE__ */ Object.create(null);
      o();
      for (var R = !1; s.getToken() !== 2 && s.getToken() !== 17; ) {
        if (s.getToken() === 5) {
          R || u(D("PropertyExpected", "Property expected"), G.PropertyExpected);
          var k = s.getTokenOffset();
          if (o(), s.getToken() === 2) {
            R && l(D("TrailingComma", "Trailing comma"), G.TrailingComma, k, k + 1);
            continue;
          }
        } else
          R && u(D("ExpectedComma", "Expected comma"), G.CommaExpected);
        var w = m(S, A);
        w ? S.properties.push(w) : u(D("PropertyExpected", "Property expected"), G.PropertyExpected, void 0, [], [2, 5]), R = !0;
      }
      return s.getToken() !== 2 ? u(D("ExpectedCloseBrace", "Expected comma or closing brace"), G.CommaOrCloseBraceExpected, S) : f(S, !0);
    }
  }
  function b(_) {
    if (s.getToken() === 10) {
      var S = new On(_, s.getTokenOffset());
      return S.value = s.getTokenValue(), f(S, !0);
    }
  }
  function y(_) {
    if (s.getToken() === 11) {
      var S = new ru(_, s.getTokenOffset());
      if (s.getTokenError() === 0) {
        var A = s.getTokenValue();
        try {
          var R = JSON.parse(A);
          if (!ve(R))
            return u(D("InvalidNumberFormat", "Invalid number format."), G.Undefined, S);
          S.value = R;
        } catch {
          return u(D("InvalidNumberFormat", "Invalid number format."), G.Undefined, S);
        }
        S.isInteger = A.indexOf(".") === -1;
      }
      return f(S, !0);
    }
  }
  function x(_) {
    switch (s.getToken()) {
      case 7:
        return f(new tu(_, s.getTokenOffset()), !0);
      case 8:
        return f(new js(_, !0, s.getTokenOffset()), !0);
      case 9:
        return f(new js(_, !1, s.getTokenOffset()), !0);
      default:
        return;
    }
  }
  function v(_) {
    return d(_) || p(_) || b(_) || y(_) || x(_);
  }
  var C = void 0, L = o();
  return L !== 17 && (C = v(C), C ? s.getToken() !== 17 && u(D("End of file expected", "End of file expected."), G.Undefined) : u(D("Invalid symbol", "Expected a JSON object, array or literal."), G.Undefined)), new da(C, n, a);
}
function yr(e, t, n) {
  if (e !== null && typeof e == "object") {
    var r = t + "	";
    if (Array.isArray(e)) {
      if (e.length === 0)
        return "[]";
      for (var i = `[
`, s = 0; s < e.length; s++)
        i += r + yr(e[s], r, n), s < e.length - 1 && (i += ","), i += `
`;
      return i += t + "]", i;
    } else {
      var a = Object.keys(e);
      if (a.length === 0)
        return "{}";
      for (var i = `{
`, s = 0; s < a.length; s++) {
        var o = a[s];
        i += r + JSON.stringify(o) + ": " + yr(e[o], r, n), s < a.length - 1 && (i += ","), i += `
`;
      }
      return i += t + "}", i;
    }
  }
  return n(e);
}
var jn = Gt(), uu = function() {
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
    var h = this.getCurrentWord(t, o), f;
    if (l && (l.type === "string" || l.type === "number" || l.type === "boolean" || l.type === "null"))
      f = X.create(t.positionAt(l.offset), t.positionAt(l.offset + l.length));
    else {
      var d = o - h.length;
      d > 0 && a[d - 1] === '"' && d--, f = X.create(t.positionAt(d), n);
    }
    var g = {}, m = {
      add: function(p) {
        var b = p.label, y = g[b];
        if (y)
          y.documentation || (y.documentation = p.documentation), y.detail || (y.detail = p.detail);
        else {
          if (b = b.replace(/[\n]/g, "↵"), b.length > 60) {
            var x = b.substr(0, 57).trim() + "...";
            g[x] || (b = x);
          }
          f && p.insertText !== void 0 && (p.textEdit = Ee.replace(f, p.insertText)), p.label = b, g[b] = p, s.items.push(p);
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
      var b = [], y = !0, x = "", v = void 0;
      if (l && l.type === "string") {
        var C = l.parent;
        C && C.type === "property" && C.keyNode === l && (y = !C.valueNode, v = C, x = a.substr(l.offset + 1, l.length - 2), C && (l = C.parent));
      }
      if (l && l.type === "object") {
        if (l.offset === o)
          return s;
        var L = l.properties;
        L.forEach(function(R) {
          (!v || v !== R) && (g[R.keyNode.value] = gr.create("__"));
        });
        var _ = "";
        y && (_ = i.evaluateSeparatorAfter(t, t.offsetAt(f.end))), p ? i.getPropertyCompletions(p, r, l, y, _, m) : i.getSchemaLessPropertyCompletions(r, l, x, m);
        var S = br(l);
        i.contributions.forEach(function(R) {
          var k = R.collectPropertyCompletions(t.uri, S, h, y, _ === "", m);
          k && b.push(k);
        }), !p && h.length > 0 && a.charAt(o - h.length - 1) !== '"' && (m.add({
          kind: ye.Property,
          label: i.getLabelForValue(h),
          insertText: i.getInsertTextForProperty(h, void 0, !1, _),
          insertTextFormat: ne.Snippet,
          documentation: ""
        }), m.setAsIncomplete());
      }
      var A = {};
      return p ? i.getValueCompletions(p, r, l, o, t, m, A) : i.getSchemaLessValueCompletions(r, l, o, t, m), i.contributions.length > 0 && i.getContributedValueCompletions(r, l, o, t, m, b), i.promiseConstructor.all(b).then(function() {
        if (m.getNumberOfProposals() === 0) {
          var R = o;
          l && (l.type === "string" || l.type === "number" || l.type === "boolean" || l.type === "null") && (R = l.offset + l.length);
          var k = i.evaluateSeparatorAfter(t, R);
          i.addFillerValueCompletions(A, k, m);
        }
        return s;
      });
    });
  }, e.prototype.getPropertyCompletions = function(t, n, r, i, s, a) {
    var o = this, l = n.getMatchingSchemas(t.schema, r.offset);
    l.forEach(function(u) {
      if (u.node === r && !u.inverted) {
        var h = u.schema.properties;
        h && Object.keys(h).forEach(function(p) {
          var b = h[p];
          if (typeof b == "object" && !b.deprecationMessage && !b.doNotSuggest) {
            var y = {
              kind: ye.Property,
              label: p,
              insertText: o.getInsertTextForProperty(p, b, i, s),
              insertTextFormat: ne.Snippet,
              filterText: o.getFilterTextForValue(p),
              documentation: o.fromMarkup(b.markdownDescription) || b.description || ""
            };
            b.suggestSortText !== void 0 && (y.sortText = b.suggestSortText), y.insertText && Bt(y.insertText, "$1".concat(s)) && (y.command = {
              title: "Suggest",
              command: "editor.action.triggerSuggest"
            }), a.add(y);
          }
        });
        var f = u.schema.propertyNames;
        if (typeof f == "object" && !f.deprecationMessage && !f.doNotSuggest) {
          var d = function(p, b) {
            b === void 0 && (b = void 0);
            var y = {
              kind: ye.Property,
              label: p,
              insertText: o.getInsertTextForProperty(p, void 0, i, s),
              insertTextFormat: ne.Snippet,
              filterText: o.getFilterTextForValue(p),
              documentation: b || o.fromMarkup(f.markdownDescription) || f.description || ""
            };
            f.suggestSortText !== void 0 && (y.sortText = f.suggestSortText), y.insertText && Bt(y.insertText, "$1".concat(s)) && (y.command = {
              title: "Suggest",
              command: "editor.action.triggerSuggest"
            }), a.add(y);
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
        var h = u.keyNode.value;
        i.add({
          kind: ye.Property,
          label: h,
          insertText: s.getInsertTextForValue(h, ""),
          insertTextFormat: ne.Snippet,
          filterText: s.getFilterTextForValue(h),
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
        kind: ye.Property,
        label: "$schema",
        insertText: this.getInsertTextForProperty("$schema", void 0, !0, ""),
        insertTextFormat: ne.Snippet,
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
        insertTextFormat: ne.Snippet,
        documentation: ""
      }), s.add({
        kind: this.getSuggestionKind("array"),
        label: "Empty array",
        insertText: this.getInsertTextForValue([], ""),
        insertTextFormat: ne.Snippet,
        documentation: ""
      });
      return;
    }
    var l = this.evaluateSeparatorAfter(i, o), u = function(g) {
      g.parent && !ha(g.parent, r, !0) && s.add({
        kind: a.getSuggestionKind(g.type),
        label: a.getLabelTextForMatchingNode(g, i),
        insertText: a.getInsertTextForMatchingNode(g, i, l),
        insertTextFormat: ne.Snippet,
        documentation: ""
      }), g.type === "boolean" && a.addBooleanValueCompletion(!g.value, l, s);
    };
    if (n.type === "property" && r > (n.colonOffset || 0)) {
      var h = n.valueNode;
      if (h && (r > h.offset + h.length || h.type === "object" || h.type === "array"))
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
    var l = i, u = void 0, h = void 0;
    if (r && (r.type === "string" || r.type === "number" || r.type === "boolean" || r.type === "null") && (l = r.offset + r.length, h = r, r = r.parent), !r) {
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
      for (var d = this.evaluateSeparatorAfter(s, l), g = n.getMatchingSchemas(t.schema, r.offset, h), m = 0, p = g; m < p.length; m++) {
        var b = p[m];
        if (b.node === r && !b.inverted && b.schema) {
          if (r.type === "array" && b.schema.items)
            if (Array.isArray(b.schema.items)) {
              var y = this.findItemAtOffset(r, s, i);
              y < b.schema.items.length && this.addSchemaValueCompletions(b.schema.items[y], d, a, o);
            } else
              this.addSchemaValueCompletions(b.schema.items, d, a, o);
          if (u !== void 0) {
            var x = !1;
            if (b.schema.properties) {
              var v = b.schema.properties[u];
              v && (x = !0, this.addSchemaValueCompletions(v, d, a, o));
            }
            if (b.schema.patternProperties && !x)
              for (var C = 0, L = Object.keys(b.schema.patternProperties); C < L.length; C++) {
                var _ = L[C], S = pn(_);
                if (S != null && S.test(u)) {
                  x = !0;
                  var v = b.schema.patternProperties[_];
                  this.addSchemaValueCompletions(v, d, a, o);
                }
              }
            if (b.schema.additionalProperties && !x) {
              var v = b.schema.additionalProperties;
              this.addSchemaValueCompletions(v, d, a, o);
            }
          }
        }
      }
      u === "$schema" && !r.parent && this.addDollarSchemaCompletions(d, a), o.boolean && (this.addBooleanValueCompletion(!0, d, a), this.addBooleanValueCompletion(!1, d, a)), o.null && this.addNullValueCompletion(d, a);
    }
  }, e.prototype.getContributedValueCompletions = function(t, n, r, i, s, a) {
    if (!n)
      this.contributions.forEach(function(h) {
        var f = h.collectDefaultCompletions(i.uri, s);
        f && a.push(f);
      });
    else if ((n.type === "string" || n.type === "number" || n.type === "boolean" || n.type === "null") && (n = n.parent), n && n.type === "property" && r > (n.colonOffset || 0)) {
      var o = n.keyNode.value, l = n.valueNode;
      if ((!l || r <= l.offset + l.length) && n.parent) {
        var u = br(n.parent);
        this.contributions.forEach(function(h) {
          var f = h.collectValueCompletions(i.uri, u, o, s);
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
    if (Oe(t.default)) {
      for (var o = t.type, l = t.default, u = i; u > 0; u--)
        l = [l], o = "array";
      r.add({
        kind: this.getSuggestionKind(o),
        label: this.getLabelForValue(l),
        insertText: this.getInsertTextForValue(l, n),
        insertTextFormat: ne.Snippet,
        detail: jn("json.suggest.default", "Default value")
      }), a = !0;
    }
    Array.isArray(t.examples) && t.examples.forEach(function(h) {
      for (var f = t.type, d = h, g = i; g > 0; g--)
        d = [d], f = "array";
      r.add({
        kind: s.getSuggestionKind(f),
        label: s.getLabelForValue(d),
        insertText: s.getInsertTextForValue(d, n),
        insertTextFormat: ne.Snippet
      }), a = !0;
    }), Array.isArray(t.defaultSnippets) && t.defaultSnippets.forEach(function(h) {
      var f = t.type, d = h.body, g = h.label, m, p;
      if (Oe(d)) {
        t.type;
        for (var b = i; b > 0; b--)
          d = [d];
        m = s.getInsertTextForSnippetValue(d, n), p = s.getFilterTextForSnippetValue(d), g = g || s.getLabelForSnippetValue(d);
      } else if (typeof h.bodyText == "string") {
        for (var y = "", x = "", v = "", b = i; b > 0; b--)
          y = y + v + `[
`, x = x + `
` + v + "]", v += "	", f = "array";
        m = y + v + h.bodyText.split(`
`).join(`
` + v) + x + n, g = g || m, p = m.replace(/[\n]/g, "");
      } else
        return;
      r.add({
        kind: s.getSuggestionKind(f),
        label: g,
        documentation: s.fromMarkup(h.markdownDescription) || h.description,
        insertText: m,
        insertTextFormat: ne.Snippet,
        filterText: p
      }), a = !0;
    }), !a && typeof t.items == "object" && !Array.isArray(t.items) && i < 5 && this.addDefaultValueCompletions(t.items, n, r, i + 1);
  }, e.prototype.addEnumValueCompletions = function(t, n, r) {
    if (Oe(t.const) && r.add({
      kind: this.getSuggestionKind(t.type),
      label: this.getLabelForValue(t.const),
      insertText: this.getInsertTextForValue(t.const, n),
      insertTextFormat: ne.Snippet,
      documentation: this.fromMarkup(t.markdownDescription) || t.description
    }), Array.isArray(t.enum))
      for (var i = 0, s = t.enum.length; i < s; i++) {
        var a = t.enum[i], o = this.fromMarkup(t.markdownDescription) || t.description;
        t.markdownEnumDescriptions && i < t.markdownEnumDescriptions.length && this.doesSupportMarkdown() ? o = this.fromMarkup(t.markdownEnumDescriptions[i]) : t.enumDescriptions && i < t.enumDescriptions.length && (o = t.enumDescriptions[i]), r.add({
          kind: this.getSuggestionKind(t.type),
          label: this.getLabelForValue(a),
          insertText: this.getInsertTextForValue(a, n),
          insertTextFormat: ne.Snippet,
          documentation: o
        });
      }
  }, e.prototype.collectTypes = function(t, n) {
    if (!(Array.isArray(t.enum) || Oe(t.const))) {
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
      insertTextFormat: ne.Snippet,
      detail: jn("defaults.object", "New object"),
      documentation: ""
    }), t.array && r.add({
      kind: this.getSuggestionKind("array"),
      label: "[]",
      insertText: this.getInsertTextForGuessedValue([], n),
      insertTextFormat: ne.Snippet,
      detail: jn("defaults.array", "New array"),
      documentation: ""
    });
  }, e.prototype.addBooleanValueCompletion = function(t, n, r) {
    r.add({
      kind: this.getSuggestionKind("boolean"),
      label: t ? "true" : "false",
      insertText: this.getInsertTextForValue(t, n),
      insertTextFormat: ne.Snippet,
      documentation: ""
    });
  }, e.prototype.addNullValueCompletion = function(t, n) {
    n.add({
      kind: this.getSuggestionKind("null"),
      label: "null",
      insertText: "null" + t,
      insertTextFormat: ne.Snippet,
      documentation: ""
    });
  }, e.prototype.addDollarSchemaCompletions = function(t, n) {
    var r = this, i = this.schemaService.getRegisteredSchemaIds(function(s) {
      return s === "http" || s === "https";
    });
    i.forEach(function(s) {
      return n.add({
        kind: ye.Module,
        label: r.getLabelForValue(s),
        filterText: r.getFilterTextForValue(s),
        insertText: r.getInsertTextForValue(s, t),
        insertTextFormat: ne.Snippet,
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
    return yr(t, "", r) + n;
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
      return ye.Value;
    switch (t) {
      case "string":
        return ye.Value;
      case "object":
        return ye.Module;
      case "property":
        return ye.Property;
      default:
        return ye.Value;
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
          Oe(u) && (o = this.getInsertTextForSnippetValue(u, ""));
        }
        l += n.defaultSnippets.length;
      }
      if (n.enum && (!o && n.enum.length === 1 && (o = this.getInsertTextForGuessedValue(n.enum[0], "")), l += n.enum.length), Oe(n.default) && (o || (o = this.getInsertTextForGuessedValue(n.default, "")), l++), Array.isArray(n.examples) && n.examples.length && (o || (o = this.getInsertTextForGuessedValue(n.examples[0], "")), l += n.examples.length), l === 0) {
        var h = Array.isArray(n.type) ? n.type[0] : n.type;
        switch (h || (n.properties ? h = "object" : n.items && (h = "array")), h) {
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
    var r = yt(t.getText(), !0);
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
    for (var i = yt(n.getText(), !0), s = t.items, a = s.length - 1; a >= 0; a--) {
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
    var i = yt(t.getText(), !1);
    i.setPosition(n);
    for (var s = i.scan(); s !== 17 && i.getTokenOffset() + i.getTokenLength() < r; )
      s = i.scan();
    return (s === 12 || s === 13) && i.getTokenOffset() <= r;
  }, e.prototype.fromMarkup = function(t) {
    if (t && this.doesSupportMarkdown())
      return {
        kind: Ue.Markdown,
        value: t
      };
  }, e.prototype.doesSupportMarkdown = function() {
    if (!Oe(this.supportsMarkdown)) {
      var t = this.clientCapabilities.textDocument && this.clientCapabilities.textDocument.completion;
      this.supportsMarkdown = t && t.completionItem && Array.isArray(t.completionItem.documentationFormat) && t.completionItem.documentationFormat.indexOf(Ue.Markdown) !== -1;
    }
    return this.supportsMarkdown;
  }, e.prototype.doesSupportsCommitCharacters = function() {
    if (!Oe(this.supportsCommitCharacters)) {
      var t = this.clientCapabilities.textDocument && this.clientCapabilities.textDocument.completion;
      this.supportsCommitCharacters = t && t.completionItem && !!t.completionItem.commitCharactersSupport;
    }
    return this.supportsCommitCharacters;
  }, e;
}(), cu = function() {
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
    }, h = br(s), f = this.contributions.length - 1; f >= 0; f--) {
      var d = this.contributions[f], g = d.getInfoContribution(t.uri, h);
      if (g)
        return g.then(function(m) {
          return u(m);
        });
    }
    return this.schemaService.getSchemaForResource(t.uri, r).then(function(m) {
      if (m && s) {
        var p = r.getMatchingSchemas(m.schema, s.offset), b = void 0, y = void 0, x = void 0, v = void 0;
        p.every(function(L) {
          if (L.node === s && !L.inverted && L.schema && (b = b || L.schema.title, y = y || L.schema.markdownDescription || Bn(L.schema.description), L.schema.enum)) {
            var _ = L.schema.enum.indexOf(rt(s));
            L.schema.markdownEnumDescriptions ? x = L.schema.markdownEnumDescriptions[_] : L.schema.enumDescriptions && (x = Bn(L.schema.enumDescriptions[_])), x && (v = L.schema.enum[_], typeof v != "string" && (v = JSON.stringify(v)));
          }
          return !0;
        });
        var C = "";
        return b && (C = Bn(b)), y && (C.length > 0 && (C += `

`), C += y), x && (C.length > 0 && (C += `

`), C += "`".concat(fu(v), "`: ").concat(x)), u([C]);
      }
      return null;
    });
  }, e;
}();
function Bn(e) {
  if (e) {
    var t = e.replace(/([^\n\r])(\r?\n)([^\n\r])/gm, `$1

$3`);
    return t.replace(/[\\`*_{}[\]()#+\-.!]/g, "\\$&");
  }
}
function fu(e) {
  return e.indexOf("`") !== -1 ? "`` " + e + " ``" : e;
}
var hu = Gt(), du = function() {
  function e(t, n) {
    this.jsonSchemaService = t, this.promise = n, this.validationEnabled = !0;
  }
  return e.prototype.configure = function(t) {
    t && (this.validationEnabled = t.validate !== !1, this.commentSeverity = t.allowComments ? void 0 : xe.Error);
  }, e.prototype.doValidation = function(t, n, r, i) {
    var s = this;
    if (!this.validationEnabled)
      return this.promise.resolve([]);
    var a = [], o = {}, l = function(d) {
      var g = d.range.start.line + " " + d.range.start.character + " " + d.message;
      o[g] || (o[g] = !0, a.push(d));
    }, u = function(d) {
      var g = r != null && r.trailingCommas ? Yt(r.trailingCommas) : xe.Error, m = r != null && r.comments ? Yt(r.comments) : s.commentSeverity, p = r != null && r.schemaValidation ? Yt(r.schemaValidation) : xe.Warning, b = r != null && r.schemaRequest ? Yt(r.schemaRequest) : xe.Warning;
      if (d) {
        if (d.errors.length && n.root && b) {
          var y = n.root, x = y.type === "object" ? y.properties[0] : void 0;
          if (x && x.keyNode.value === "$schema") {
            var v = x.valueNode || x, C = X.create(t.positionAt(v.offset), t.positionAt(v.offset + v.length));
            l(qe.create(C, d.errors[0], b, G.SchemaResolveError));
          } else {
            var C = X.create(t.positionAt(y.offset), t.positionAt(y.offset + 1));
            l(qe.create(C, d.errors[0], b, G.SchemaResolveError));
          }
        } else if (p) {
          var L = n.validate(t, d.schema, p);
          L && L.forEach(l);
        }
        ga(d.schema) && (m = void 0), ma(d.schema) && (g = void 0);
      }
      for (var _ = 0, S = n.syntaxErrors; _ < S.length; _++) {
        var A = S[_];
        if (A.code === G.TrailingComma) {
          if (typeof g != "number")
            continue;
          A.severity = g;
        }
        l(A);
      }
      if (typeof m == "number") {
        var R = hu("InvalidCommentToken", "Comments are not permitted in JSON.");
        n.comments.forEach(function(k) {
          l(qe.create(k, R, m, G.CommentNotPermitted));
        });
      }
      return a;
    };
    if (i) {
      var h = i.id || "schemaservice://untitled/" + gu++, f = this.jsonSchemaService.registerExternalSchema(h, [], i);
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
}(), gu = 0;
function ga(e) {
  if (e && typeof e == "object") {
    if (Fe(e.allowComments))
      return e.allowComments;
    if (e.allOf)
      for (var t = 0, n = e.allOf; t < n.length; t++) {
        var r = n[t], i = ga(r);
        if (Fe(i))
          return i;
      }
  }
}
function ma(e) {
  if (e && typeof e == "object") {
    if (Fe(e.allowTrailingCommas))
      return e.allowTrailingCommas;
    var t = e;
    if (Fe(t.allowsTrailingCommas))
      return t.allowsTrailingCommas;
    if (e.allOf)
      for (var n = 0, r = e.allOf; n < r.length; n++) {
        var i = r[n], s = ma(i);
        if (Fe(s))
          return s;
      }
  }
}
function Yt(e) {
  switch (e) {
    case "error":
      return xe.Error;
    case "warning":
      return xe.Warning;
    case "ignore":
      return;
  }
}
var qs = 48, mu = 57, pu = 65, Kt = 97, vu = 102;
function ee(e) {
  return e < qs ? 0 : e <= mu ? e - qs : (e < Kt && (e += Kt - pu), e >= Kt && e <= vu ? e - Kt + 10 : 0);
}
function bu(e) {
  if (e[0] === "#")
    switch (e.length) {
      case 4:
        return {
          red: ee(e.charCodeAt(1)) * 17 / 255,
          green: ee(e.charCodeAt(2)) * 17 / 255,
          blue: ee(e.charCodeAt(3)) * 17 / 255,
          alpha: 1
        };
      case 5:
        return {
          red: ee(e.charCodeAt(1)) * 17 / 255,
          green: ee(e.charCodeAt(2)) * 17 / 255,
          blue: ee(e.charCodeAt(3)) * 17 / 255,
          alpha: ee(e.charCodeAt(4)) * 17 / 255
        };
      case 7:
        return {
          red: (ee(e.charCodeAt(1)) * 16 + ee(e.charCodeAt(2))) / 255,
          green: (ee(e.charCodeAt(3)) * 16 + ee(e.charCodeAt(4))) / 255,
          blue: (ee(e.charCodeAt(5)) * 16 + ee(e.charCodeAt(6))) / 255,
          alpha: 1
        };
      case 9:
        return {
          red: (ee(e.charCodeAt(1)) * 16 + ee(e.charCodeAt(2))) / 255,
          green: (ee(e.charCodeAt(3)) * 16 + ee(e.charCodeAt(4))) / 255,
          blue: (ee(e.charCodeAt(5)) * 16 + ee(e.charCodeAt(6))) / 255,
          alpha: (ee(e.charCodeAt(7)) * 16 + ee(e.charCodeAt(8))) / 255
        };
    }
}
var yu = function() {
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
    if ((o === "vscode://defaultsettings/keybindings.json" || Bt(o.toLowerCase(), "/user/keybindings.json")) && s.type === "array") {
      for (var l = [], u = 0, h = s.items; u < h.length; u++) {
        var f = h[u];
        if (f.type === "object")
          for (var d = 0, g = f.properties; d < g.length; d++) {
            var m = g[d];
            if (m.keyNode.value === "key" && m.valueNode) {
              var p = qt.create(t.uri, ze(t, f));
              if (l.push({ name: rt(m.valueNode), kind: Pe.Function, location: p }), a--, a <= 0)
                return r && r.onResultLimitExceeded && r.onResultLimitExceeded(o), l;
            }
          }
      }
      return l;
    }
    for (var b = [
      { node: s, containerName: "" }
    ], y = 0, x = !1, v = [], C = function(_, S) {
      _.type === "array" ? _.items.forEach(function(A) {
        A && b.push({ node: A, containerName: S });
      }) : _.type === "object" && _.properties.forEach(function(A) {
        var R = A.valueNode;
        if (R)
          if (a > 0) {
            a--;
            var k = qt.create(t.uri, ze(t, A)), w = S ? S + "." + A.keyNode.value : A.keyNode.value;
            v.push({ name: i.getKeyLabel(A), kind: i.getSymbolKind(R.type), location: k, containerName: S }), b.push({ node: R, containerName: w });
          } else
            x = !0;
      });
    }; y < b.length; ) {
      var L = b[y++];
      C(L.node, L.containerName);
    }
    return x && r && r.onResultLimitExceeded && r.onResultLimitExceeded(o), v;
  }, e.prototype.findDocumentSymbols2 = function(t, n, r) {
    var i = this;
    r === void 0 && (r = { resultLimit: Number.MAX_VALUE });
    var s = n.root;
    if (!s)
      return [];
    var a = r.resultLimit || Number.MAX_VALUE, o = t.uri;
    if ((o === "vscode://defaultsettings/keybindings.json" || Bt(o.toLowerCase(), "/user/keybindings.json")) && s.type === "array") {
      for (var l = [], u = 0, h = s.items; u < h.length; u++) {
        var f = h[u];
        if (f.type === "object")
          for (var d = 0, g = f.properties; d < g.length; d++) {
            var m = g[d];
            if (m.keyNode.value === "key" && m.valueNode) {
              var p = ze(t, f), b = ze(t, m.keyNode);
              if (l.push({ name: rt(m.valueNode), kind: Pe.Function, range: p, selectionRange: b }), a--, a <= 0)
                return r && r.onResultLimitExceeded && r.onResultLimitExceeded(o), l;
            }
          }
      }
      return l;
    }
    for (var y = [], x = [
      { node: s, result: y }
    ], v = 0, C = !1, L = function(S, A) {
      S.type === "array" ? S.items.forEach(function(R, k) {
        if (R)
          if (a > 0) {
            a--;
            var w = ze(t, R), N = w, T = String(k), V = { name: T, kind: i.getSymbolKind(R.type), range: w, selectionRange: N, children: [] };
            A.push(V), x.push({ result: V.children, node: R });
          } else
            C = !0;
      }) : S.type === "object" && S.properties.forEach(function(R) {
        var k = R.valueNode;
        if (k)
          if (a > 0) {
            a--;
            var w = ze(t, R), N = ze(t, R.keyNode), T = [], V = { name: i.getKeyLabel(R), kind: i.getSymbolKind(k.type), range: w, selectionRange: N, children: T, detail: i.getDetail(k) };
            A.push(V), x.push({ result: T, node: k });
          } else
            C = !0;
      });
    }; v < x.length; ) {
      var _ = x[v++];
      L(_.node, _.result);
    }
    return C && r && r.onResultLimitExceeded && r.onResultLimitExceeded(o), y;
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
        for (var a = r && typeof r.resultLimit == "number" ? r.resultLimit : Number.MAX_VALUE, o = n.getMatchingSchemas(i.schema), l = {}, u = 0, h = o; u < h.length; u++) {
          var f = h[u];
          if (!f.inverted && f.schema && (f.schema.format === "color" || f.schema.format === "color-hex") && f.node && f.node.type === "string") {
            var d = String(f.node.offset);
            if (!l[d]) {
              var g = bu(rt(f.node));
              if (g) {
                var m = ze(t, f.node);
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
    var h;
    return r.alpha === 1 ? h = "#".concat(u(a)).concat(u(o)).concat(u(l)) : h = "#".concat(u(a)).concat(u(o)).concat(u(l)).concat(u(Math.round(r.alpha * 255))), s.push({ label: h, textEdit: Ee.replace(i, JSON.stringify(h)) }), s;
  }, e;
}();
function ze(e, t) {
  return X.create(e.positionAt(t.offset), e.positionAt(t.offset + t.length));
}
var $ = Gt(), xr = {
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
}, xu = {
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
for (Us in xr.schemas) {
  en = xr.schemas[Us];
  for (gt in en.properties)
    tn = en.properties[gt], typeof tn == "boolean" && (tn = en.properties[gt] = {}), qn = xu[gt], qn ? tn.description = qn : console.log("".concat(gt, ": localize('schema.json.").concat(gt, `', "")`));
}
var en, tn, qn, gt, Us, pa;
pa = (() => {
  var e = { 470: (r) => {
    function i(o) {
      if (typeof o != "string")
        throw new TypeError("Path must be a string. Received " + JSON.stringify(o));
    }
    function s(o, l) {
      for (var u, h = "", f = 0, d = -1, g = 0, m = 0; m <= o.length; ++m) {
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
              if (h.length < 2 || f !== 2 || h.charCodeAt(h.length - 1) !== 46 || h.charCodeAt(h.length - 2) !== 46) {
                if (h.length > 2) {
                  var p = h.lastIndexOf("/");
                  if (p !== h.length - 1) {
                    p === -1 ? (h = "", f = 0) : f = (h = h.slice(0, p)).length - 1 - h.lastIndexOf("/"), d = m, g = 0;
                    continue;
                  }
                } else if (h.length === 2 || h.length === 1) {
                  h = "", f = 0, d = m, g = 0;
                  continue;
                }
              }
              l && (h.length > 0 ? h += "/.." : h = "..", f = 2);
            } else
              h.length > 0 ? h += "/" + o.slice(d + 1, m) : h = o.slice(d + 1, m), f = m - d - 1;
          d = m, g = 0;
        } else
          u === 46 && g !== -1 ? ++g : g = -1;
      }
      return h;
    }
    var a = { resolve: function() {
      for (var o, l = "", u = !1, h = arguments.length - 1; h >= -1 && !u; h--) {
        var f;
        h >= 0 ? f = arguments[h] : (o === void 0 && (o = process.cwd()), f = o), i(f), f.length !== 0 && (l = f + "/" + l, u = f.charCodeAt(0) === 47);
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
      for (var h = o.length, f = h - u, d = 1; d < l.length && l.charCodeAt(d) === 47; ++d)
        ;
      for (var g = l.length - d, m = f < g ? f : g, p = -1, b = 0; b <= m; ++b) {
        if (b === m) {
          if (g > m) {
            if (l.charCodeAt(d + b) === 47)
              return l.slice(d + b + 1);
            if (b === 0)
              return l.slice(d + b);
          } else
            f > m && (o.charCodeAt(u + b) === 47 ? p = b : b === 0 && (p = 0));
          break;
        }
        var y = o.charCodeAt(u + b);
        if (y !== l.charCodeAt(d + b))
          break;
        y === 47 && (p = b);
      }
      var x = "";
      for (b = u + p + 1; b <= h; ++b)
        b !== h && o.charCodeAt(b) !== 47 || (x.length === 0 ? x += ".." : x += "/..");
      return x.length > 0 ? x + l.slice(d + p) : (d += p, l.charCodeAt(d) === 47 && ++d, l.slice(d));
    }, _makeLong: function(o) {
      return o;
    }, dirname: function(o) {
      if (i(o), o.length === 0)
        return ".";
      for (var l = o.charCodeAt(0), u = l === 47, h = -1, f = !0, d = o.length - 1; d >= 1; --d)
        if ((l = o.charCodeAt(d)) === 47) {
          if (!f) {
            h = d;
            break;
          }
        } else
          f = !1;
      return h === -1 ? u ? "/" : "." : u && h === 1 ? "//" : o.slice(0, h);
    }, basename: function(o, l) {
      if (l !== void 0 && typeof l != "string")
        throw new TypeError('"ext" argument must be a string');
      i(o);
      var u, h = 0, f = -1, d = !0;
      if (l !== void 0 && l.length > 0 && l.length <= o.length) {
        if (l.length === o.length && l === o)
          return "";
        var g = l.length - 1, m = -1;
        for (u = o.length - 1; u >= 0; --u) {
          var p = o.charCodeAt(u);
          if (p === 47) {
            if (!d) {
              h = u + 1;
              break;
            }
          } else
            m === -1 && (d = !1, m = u + 1), g >= 0 && (p === l.charCodeAt(g) ? --g == -1 && (f = u) : (g = -1, f = m));
        }
        return h === f ? f = m : f === -1 && (f = o.length), o.slice(h, f);
      }
      for (u = o.length - 1; u >= 0; --u)
        if (o.charCodeAt(u) === 47) {
          if (!d) {
            h = u + 1;
            break;
          }
        } else
          f === -1 && (d = !1, f = u + 1);
      return f === -1 ? "" : o.slice(h, f);
    }, extname: function(o) {
      i(o);
      for (var l = -1, u = 0, h = -1, f = !0, d = 0, g = o.length - 1; g >= 0; --g) {
        var m = o.charCodeAt(g);
        if (m !== 47)
          h === -1 && (f = !1, h = g + 1), m === 46 ? l === -1 ? l = g : d !== 1 && (d = 1) : l !== -1 && (d = -1);
        else if (!f) {
          u = g + 1;
          break;
        }
      }
      return l === -1 || h === -1 || d === 0 || d === 1 && l === h - 1 && l === u + 1 ? "" : o.slice(l, h);
    }, format: function(o) {
      if (o === null || typeof o != "object")
        throw new TypeError('The "pathObject" argument must be of type Object. Received type ' + typeof o);
      return function(l, u) {
        var h = u.dir || u.root, f = u.base || (u.name || "") + (u.ext || "");
        return h ? h === u.root ? h + f : h + "/" + f : f;
      }(0, o);
    }, parse: function(o) {
      i(o);
      var l = { root: "", dir: "", base: "", ext: "", name: "" };
      if (o.length === 0)
        return l;
      var u, h = o.charCodeAt(0), f = h === 47;
      f ? (l.root = "/", u = 1) : u = 0;
      for (var d = -1, g = 0, m = -1, p = !0, b = o.length - 1, y = 0; b >= u; --b)
        if ((h = o.charCodeAt(b)) !== 47)
          m === -1 && (p = !1, m = b + 1), h === 46 ? d === -1 ? d = b : y !== 1 && (y = 1) : d !== -1 && (y = -1);
        else if (!p) {
          g = b + 1;
          break;
        }
      return d === -1 || m === -1 || y === 0 || y === 1 && d === m - 1 && d === g + 1 ? m !== -1 && (l.base = l.name = g === 0 && f ? o.slice(1, m) : o.slice(g, m)) : (g === 0 && f ? (l.name = o.slice(1, d), l.base = o.slice(1, m)) : (l.name = o.slice(g, d), l.base = o.slice(g, m)), l.ext = o.slice(d, m)), g > 0 ? l.dir = o.slice(0, g - 1) : f && (l.dir = "/"), l;
    }, sep: "/", delimiter: ":", win32: null, posix: null };
    a.posix = a, r.exports = a;
  }, 447: (r, i, s) => {
    var a;
    if (s.r(i), s.d(i, { URI: () => x, Utils: () => T }), typeof process == "object")
      a = process.platform === "win32";
    else if (typeof navigator == "object") {
      var o = navigator.userAgent;
      a = o.indexOf("Windows") >= 0;
    }
    var l, u, h = (l = function(P, M) {
      return (l = Object.setPrototypeOf || { __proto__: [] } instanceof Array && function(F, I) {
        F.__proto__ = I;
      } || function(F, I) {
        for (var j in I)
          Object.prototype.hasOwnProperty.call(I, j) && (F[j] = I[j]);
      })(P, M);
    }, function(P, M) {
      if (typeof M != "function" && M !== null)
        throw new TypeError("Class extends value " + String(M) + " is not a constructor or null");
      function F() {
        this.constructor = P;
      }
      l(P, M), P.prototype = M === null ? Object.create(M) : (F.prototype = M.prototype, new F());
    }), f = /^\w[\w\d+.-]*$/, d = /^\//, g = /^\/\//;
    function m(P, M) {
      if (!P.scheme && M)
        throw new Error('[UriError]: Scheme is missing: {scheme: "", authority: "'.concat(P.authority, '", path: "').concat(P.path, '", query: "').concat(P.query, '", fragment: "').concat(P.fragment, '"}'));
      if (P.scheme && !f.test(P.scheme))
        throw new Error("[UriError]: Scheme contains illegal characters.");
      if (P.path) {
        if (P.authority) {
          if (!d.test(P.path))
            throw new Error('[UriError]: If a URI contains an authority component, then the path component must either be empty or begin with a slash ("/") character');
        } else if (g.test(P.path))
          throw new Error('[UriError]: If a URI does not contain an authority component, then the path cannot begin with two slash characters ("//")');
      }
    }
    var p = "", b = "/", y = /^(([^:/?#]+?):)?(\/\/([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?/, x = function() {
      function P(M, F, I, j, U, H) {
        H === void 0 && (H = !1), typeof M == "object" ? (this.scheme = M.scheme || p, this.authority = M.authority || p, this.path = M.path || p, this.query = M.query || p, this.fragment = M.fragment || p) : (this.scheme = function(we, se) {
          return we || se ? we : "file";
        }(M, H), this.authority = F || p, this.path = function(we, se) {
          switch (we) {
            case "https":
            case "http":
            case "file":
              se ? se[0] !== b && (se = b + se) : se = b;
          }
          return se;
        }(this.scheme, I || p), this.query = j || p, this.fragment = U || p, m(this, H));
      }
      return P.isUri = function(M) {
        return M instanceof P || !!M && typeof M.authority == "string" && typeof M.fragment == "string" && typeof M.path == "string" && typeof M.query == "string" && typeof M.scheme == "string" && typeof M.fsPath == "string" && typeof M.with == "function" && typeof M.toString == "function";
      }, Object.defineProperty(P.prototype, "fsPath", { get: function() {
        return A(this, !1);
      }, enumerable: !1, configurable: !0 }), P.prototype.with = function(M) {
        if (!M)
          return this;
        var F = M.scheme, I = M.authority, j = M.path, U = M.query, H = M.fragment;
        return F === void 0 ? F = this.scheme : F === null && (F = p), I === void 0 ? I = this.authority : I === null && (I = p), j === void 0 ? j = this.path : j === null && (j = p), U === void 0 ? U = this.query : U === null && (U = p), H === void 0 ? H = this.fragment : H === null && (H = p), F === this.scheme && I === this.authority && j === this.path && U === this.query && H === this.fragment ? this : new C(F, I, j, U, H);
      }, P.parse = function(M, F) {
        F === void 0 && (F = !1);
        var I = y.exec(M);
        return I ? new C(I[2] || p, N(I[4] || p), N(I[5] || p), N(I[7] || p), N(I[9] || p), F) : new C(p, p, p, p, p);
      }, P.file = function(M) {
        var F = p;
        if (a && (M = M.replace(/\\/g, b)), M[0] === b && M[1] === b) {
          var I = M.indexOf(b, 2);
          I === -1 ? (F = M.substring(2), M = b) : (F = M.substring(2, I), M = M.substring(I) || b);
        }
        return new C("file", F, M, p, p);
      }, P.from = function(M) {
        var F = new C(M.scheme, M.authority, M.path, M.query, M.fragment);
        return m(F, !0), F;
      }, P.prototype.toString = function(M) {
        return M === void 0 && (M = !1), R(this, M);
      }, P.prototype.toJSON = function() {
        return this;
      }, P.revive = function(M) {
        if (M) {
          if (M instanceof P)
            return M;
          var F = new C(M);
          return F._formatted = M.external, F._fsPath = M._sep === v ? M.fsPath : null, F;
        }
        return M;
      }, P;
    }(), v = a ? 1 : void 0, C = function(P) {
      function M() {
        var F = P !== null && P.apply(this, arguments) || this;
        return F._formatted = null, F._fsPath = null, F;
      }
      return h(M, P), Object.defineProperty(M.prototype, "fsPath", { get: function() {
        return this._fsPath || (this._fsPath = A(this, !1)), this._fsPath;
      }, enumerable: !1, configurable: !0 }), M.prototype.toString = function(F) {
        return F === void 0 && (F = !1), F ? R(this, !0) : (this._formatted || (this._formatted = R(this, !1)), this._formatted);
      }, M.prototype.toJSON = function() {
        var F = { $mid: 1 };
        return this._fsPath && (F.fsPath = this._fsPath, F._sep = v), this._formatted && (F.external = this._formatted), this.path && (F.path = this.path), this.scheme && (F.scheme = this.scheme), this.authority && (F.authority = this.authority), this.query && (F.query = this.query), this.fragment && (F.fragment = this.fragment), F;
      }, M;
    }(x), L = ((u = {})[58] = "%3A", u[47] = "%2F", u[63] = "%3F", u[35] = "%23", u[91] = "%5B", u[93] = "%5D", u[64] = "%40", u[33] = "%21", u[36] = "%24", u[38] = "%26", u[39] = "%27", u[40] = "%28", u[41] = "%29", u[42] = "%2A", u[43] = "%2B", u[44] = "%2C", u[59] = "%3B", u[61] = "%3D", u[32] = "%20", u);
    function _(P, M) {
      for (var F = void 0, I = -1, j = 0; j < P.length; j++) {
        var U = P.charCodeAt(j);
        if (U >= 97 && U <= 122 || U >= 65 && U <= 90 || U >= 48 && U <= 57 || U === 45 || U === 46 || U === 95 || U === 126 || M && U === 47)
          I !== -1 && (F += encodeURIComponent(P.substring(I, j)), I = -1), F !== void 0 && (F += P.charAt(j));
        else {
          F === void 0 && (F = P.substr(0, j));
          var H = L[U];
          H !== void 0 ? (I !== -1 && (F += encodeURIComponent(P.substring(I, j)), I = -1), F += H) : I === -1 && (I = j);
        }
      }
      return I !== -1 && (F += encodeURIComponent(P.substring(I))), F !== void 0 ? F : P;
    }
    function S(P) {
      for (var M = void 0, F = 0; F < P.length; F++) {
        var I = P.charCodeAt(F);
        I === 35 || I === 63 ? (M === void 0 && (M = P.substr(0, F)), M += L[I]) : M !== void 0 && (M += P[F]);
      }
      return M !== void 0 ? M : P;
    }
    function A(P, M) {
      var F;
      return F = P.authority && P.path.length > 1 && P.scheme === "file" ? "//".concat(P.authority).concat(P.path) : P.path.charCodeAt(0) === 47 && (P.path.charCodeAt(1) >= 65 && P.path.charCodeAt(1) <= 90 || P.path.charCodeAt(1) >= 97 && P.path.charCodeAt(1) <= 122) && P.path.charCodeAt(2) === 58 ? M ? P.path.substr(1) : P.path[1].toLowerCase() + P.path.substr(2) : P.path, a && (F = F.replace(/\//g, "\\")), F;
    }
    function R(P, M) {
      var F = M ? S : _, I = "", j = P.scheme, U = P.authority, H = P.path, we = P.query, se = P.fragment;
      if (j && (I += j, I += ":"), (U || j === "file") && (I += b, I += b), U) {
        var _e = U.indexOf("@");
        if (_e !== -1) {
          var lt = U.substr(0, _e);
          U = U.substr(_e + 1), (_e = lt.indexOf(":")) === -1 ? I += F(lt, !1) : (I += F(lt.substr(0, _e), !1), I += ":", I += F(lt.substr(_e + 1), !1)), I += "@";
        }
        (_e = (U = U.toLowerCase()).indexOf(":")) === -1 ? I += F(U, !1) : (I += F(U.substr(0, _e), !1), I += U.substr(_e));
      }
      if (H) {
        if (H.length >= 3 && H.charCodeAt(0) === 47 && H.charCodeAt(2) === 58)
          (Me = H.charCodeAt(1)) >= 65 && Me <= 90 && (H = "/".concat(String.fromCharCode(Me + 32), ":").concat(H.substr(3)));
        else if (H.length >= 2 && H.charCodeAt(1) === 58) {
          var Me;
          (Me = H.charCodeAt(0)) >= 65 && Me <= 90 && (H = "".concat(String.fromCharCode(Me + 32), ":").concat(H.substr(2)));
        }
        I += F(H, !0);
      }
      return we && (I += "?", I += F(we, !1)), se && (I += "#", I += M ? se : _(se, !1)), I;
    }
    function k(P) {
      try {
        return decodeURIComponent(P);
      } catch {
        return P.length > 3 ? P.substr(0, 3) + k(P.substr(3)) : P;
      }
    }
    var w = /(%[0-9A-Za-z][0-9A-Za-z])+/g;
    function N(P) {
      return P.match(w) ? P.replace(w, function(M) {
        return k(M);
      }) : P;
    }
    var T, V = s(470), q = function(P, M, F) {
      if (F || arguments.length === 2)
        for (var I, j = 0, U = M.length; j < U; j++)
          !I && j in M || (I || (I = Array.prototype.slice.call(M, 0, j)), I[j] = M[j]);
      return P.concat(I || Array.prototype.slice.call(M));
    }, B = V.posix || V;
    (function(P) {
      P.joinPath = function(M) {
        for (var F = [], I = 1; I < arguments.length; I++)
          F[I - 1] = arguments[I];
        return M.with({ path: B.join.apply(B, q([M.path], F, !1)) });
      }, P.resolvePath = function(M) {
        for (var F = [], I = 1; I < arguments.length; I++)
          F[I - 1] = arguments[I];
        var j = M.path || "/";
        return M.with({ path: B.resolve.apply(B, q([j], F, !1)) });
      }, P.dirname = function(M) {
        var F = B.dirname(M.path);
        return F.length === 1 && F.charCodeAt(0) === 46 ? M : M.with({ path: F });
      }, P.basename = function(M) {
        return B.basename(M.path);
      }, P.extname = function(M) {
        return B.extname(M.path);
      };
    })(T || (T = {}));
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
var { URI: Lt, Utils: Wu } = pa;
function wu(e, t) {
  if (typeof e != "string")
    throw new TypeError("Expected a string");
  for (var n = String(e), r = "", i = t ? !!t.extended : !1, s = t ? !!t.globstar : !1, a = !1, o = t && typeof t.flags == "string" ? t.flags : "", l, u = 0, h = n.length; u < h; u++)
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
var Te = Gt(), _u = "!", Su = "/", Lu = function() {
  function e(t, n) {
    this.globWrappers = [];
    try {
      for (var r = 0, i = t; r < i.length; r++) {
        var s = i[r], a = s[0] !== _u;
        a || (s = s.substring(1)), s.length > 0 && (s[0] === Su && (s = s.substring(1)), this.globWrappers.push({
          regexp: wu("**/" + s, { extended: !0, globstar: !0 }),
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
}(), Nu = function() {
  function e(t, n, r) {
    this.service = t, this.uri = n, this.dependencies = /* @__PURE__ */ new Set(), this.anchors = void 0, r && (this.unresolvedSchema = this.service.promise.resolve(new Et(r)));
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
}(), Et = function() {
  function e(t, n) {
    n === void 0 && (n = []), this.schema = t, this.errors = n;
  }
  return e;
}(), $s = function() {
  function e(t, n) {
    n === void 0 && (n = []), this.schema = t, this.errors = n;
  }
  return e.prototype.getSection = function(t) {
    var n = this.getSectionRecursive(t, this.schema);
    if (n)
      return de(n);
  }, e.prototype.getSectionRecursive = function(t, n) {
    if (!n || typeof n == "boolean" || t.length === 0)
      return n;
    var r = t.shift();
    if (n.properties && typeof n.properties[r])
      return this.getSectionRecursive(t, n.properties[r]);
    if (n.patternProperties)
      for (var i = 0, s = Object.keys(n.patternProperties); i < s.length; i++) {
        var a = s[i], o = pn(a);
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
}(), Au = function() {
  function e(t, n, r) {
    this.contextService = n, this.requestService = t, this.promiseConstructor = r || Promise, this.callOnDispose = [], this.contributionSchemas = {}, this.contributionAssociations = [], this.schemasById = {}, this.filePatternAssociations = [], this.registeredSchemasIds = {};
  }
  return e.prototype.getRegisteredSchemaIds = function(t) {
    return Object.keys(this.registeredSchemasIds).filter(function(n) {
      var r = Lt.parse(n).scheme;
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
    t = Ge(t);
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
        var i = Ge(r);
        this.contributionSchemas[i] = this.addSchemaHandle(i, n[r]);
      }
    }
    if (Array.isArray(t.schemaAssociations))
      for (var s = t.schemaAssociations, a = 0, o = s; a < o.length; a++) {
        var l = o[a], u = l.uris.map(Ge), h = this.addFilePatternAssociation(l.pattern, u);
        this.contributionAssociations.push(h);
      }
  }, e.prototype.addSchemaHandle = function(t, n) {
    var r = new Nu(this, t, n);
    return this.schemasById[t] = r, r;
  }, e.prototype.getOrAddSchemaHandle = function(t, n) {
    return this.schemasById[t] || this.addSchemaHandle(t, n);
  }, e.prototype.addFilePatternAssociation = function(t, n) {
    var r = new Lu(t, n);
    return this.filePatternAssociations.push(r), r;
  }, e.prototype.registerExternalSchema = function(t, n, r) {
    var i = Ge(t);
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
    var n = Ge(t), r = this.schemasById[n];
    return r ? r.getResolvedSchema() : this.promise.resolve(void 0);
  }, e.prototype.loadSchema = function(t) {
    if (!this.requestService) {
      var n = Te("json.schema.norequestservice", "Unable to load schema from '{0}'. No schema request service available", nn(t));
      return this.promise.resolve(new Et({}, [n]));
    }
    return this.requestService(t).then(function(r) {
      if (!r) {
        var i = Te("json.schema.nocontent", "Unable to load schema from '{0}': No content.", nn(t));
        return new Et({}, [i]);
      }
      var s = {}, a = [];
      s = $l(r, a);
      var o = a.length ? [Te("json.schema.invalidFormat", "Unable to parse content from '{0}': Parse error at offset {1}.", nn(t), a[0].offset)] : [];
      return new Et(s, o);
    }, function(r) {
      var i = r.toString(), s = r.toString().split("Error: ");
      return s.length > 1 && (i = s[1]), Bt(i, ".") && (i = i.substr(0, i.length - 1)), new Et({}, [Te("json.schema.nocontent", "Unable to load schema from '{0}': {1}.", nn(t), i)]);
    });
  }, e.prototype.resolveSchemaContent = function(t, n) {
    var r = this, i = t.errors.slice(0), s = t.schema;
    if (s.$schema) {
      var a = Ge(s.$schema);
      if (a === "http://json-schema.org/draft-03/schema")
        return this.promise.resolve(new $s({}, [Te("json.schema.draft03.notsupported", "Draft-03 schemas are not supported.")]));
      a === "https://json-schema.org/draft/2019-09/schema" ? i.push(Te("json.schema.draft201909.notsupported", "Draft 2019-09 schemas are not yet fully supported.")) : a === "https://json-schema.org/draft/2020-12/schema" && i.push(Te("json.schema.draft202012.notsupported", "Draft 2020-12 schemas are not yet fully supported."));
    }
    var o = this.contextService, l = function(p, b) {
      b = decodeURIComponent(b);
      var y = p;
      return b[0] === "/" && (b = b.substring(1)), b.split("/").some(function(x) {
        return x = x.replace(/~1/g, "/").replace(/~0/g, "~"), y = y[x], !y;
      }), y;
    }, u = function(p, b, y) {
      return b.anchors || (b.anchors = m(p)), b.anchors.get(y);
    }, h = function(p, b) {
      for (var y in b)
        b.hasOwnProperty(y) && !p.hasOwnProperty(y) && y !== "id" && y !== "$id" && (p[y] = b[y]);
    }, f = function(p, b, y, x) {
      var v;
      x === void 0 || x.length === 0 ? v = b : x.charAt(0) === "/" ? v = l(b, x) : v = u(b, y, x), v ? h(p, v) : i.push(Te("json.schema.invalidid", "$ref '{0}' in '{1}' can not be resolved.", x, y.uri));
    }, d = function(p, b, y, x) {
      o && !/^[A-Za-z][A-Za-z0-9+\-.+]*:\/\/.*/.test(b) && (b = o.resolveRelativePath(b, x.uri)), b = Ge(b);
      var v = r.getOrAddSchemaHandle(b);
      return v.getUnresolvedSchema().then(function(C) {
        if (x.dependencies.add(b), C.errors.length) {
          var L = y ? b + "#" + y : b;
          i.push(Te("json.schema.problemloadingref", "Problems loading reference '{0}': {1}", L, C.errors[0]));
        }
        return f(p, C.schema, v, y), g(p, C.schema, v);
      });
    }, g = function(p, b, y) {
      var x = [];
      return r.traverseNodes(p, function(v) {
        for (var C = /* @__PURE__ */ new Set(); v.$ref; ) {
          var L = v.$ref, _ = L.split("#", 2);
          if (delete v.$ref, _[0].length > 0) {
            x.push(d(v, _[0], _[1], y));
            return;
          } else if (!C.has(L)) {
            var S = _[1];
            f(v, b, y, S), C.add(L);
          }
        }
      }), r.promise.all(x);
    }, m = function(p) {
      var b = /* @__PURE__ */ new Map();
      return r.traverseNodes(p, function(y) {
        var x = y.$id || y.id;
        if (typeof x == "string" && x.charAt(0) === "#") {
          var v = x.substring(1);
          b.has(v) ? i.push(Te("json.schema.duplicateid", "Duplicate id declaration: '{0}'", x)) : b.set(v, y);
        }
      }), b;
    };
    return g(s, s, n).then(function(p) {
      return new $s(s, i);
    });
  }, e.prototype.traverseNodes = function(t, n) {
    if (!t || typeof t != "object")
      return Promise.resolve(null);
    for (var r = /* @__PURE__ */ new Set(), i = function() {
      for (var u = [], h = 0; h < arguments.length; h++)
        u[h] = arguments[h];
      for (var f = 0, d = u; f < d.length; f++) {
        var g = d[f];
        typeof g == "object" && o.push(g);
      }
    }, s = function() {
      for (var u = [], h = 0; h < arguments.length; h++)
        u[h] = arguments[h];
      for (var f = 0, d = u; f < d.length; f++) {
        var g = d[f];
        if (typeof g == "object")
          for (var m in g) {
            var p = m, b = g[p];
            typeof b == "object" && o.push(b);
          }
      }
    }, a = function() {
      for (var u = [], h = 0; h < arguments.length; h++)
        u[h] = arguments[h];
      for (var f = 0, d = u; f < d.length; f++) {
        var g = d[f];
        if (Array.isArray(g))
          for (var m = 0, p = g; m < p.length; m++) {
            var b = p[m];
            typeof b == "object" && o.push(b);
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
    for (var n = /* @__PURE__ */ Object.create(null), r = [], i = ku(t), s = 0, a = this.filePatternAssociations; s < a.length; s++) {
      var o = a[s];
      if (o.matchesPattern(i))
        for (var l = 0, u = o.getURIs(); l < u.length; l++) {
          var h = u[l];
          n[h] || (r.push(h), n[h] = !0);
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
        var i = Ge(r);
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
      var i = r.id || "schemaservice://untitled/matchingSchemas/" + Cu++, s = this.addSchemaHandle(i, r);
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
}(), Cu = 0;
function Ge(e) {
  try {
    return Lt.parse(e).toString(!0);
  } catch {
    return e;
  }
}
function ku(e) {
  try {
    return Lt.parse(e).with({ fragment: null, query: null }).toString(!0);
  } catch {
    return e;
  }
}
function nn(e) {
  try {
    var t = Lt.parse(e);
    if (t.scheme === "file")
      return t.fsPath;
  } catch {
  }
  return e;
}
function Ru(e, t) {
  var n = [], r = [], i = [], s = -1, a = yt(e.getText(), !1), o = a.scan();
  function l(k) {
    n.push(k), r.push(i.length);
  }
  for (; o !== 17; ) {
    switch (o) {
      case 1:
      case 3: {
        var u = e.positionAt(a.getTokenOffset()).line, h = { startLine: u, endLine: u, kind: o === 1 ? "object" : "array" };
        i.push(h);
        break;
      }
      case 2:
      case 4: {
        var f = o === 2 ? "object" : "array";
        if (i.length > 0 && i[i.length - 1].kind === f) {
          var h = i.pop(), d = e.positionAt(a.getTokenOffset()).line;
          h && d > h.startLine + 1 && s !== h.startLine && (h.endLine = d - 1, l(h), s = h.startLine);
        }
        break;
      }
      case 13: {
        var u = e.positionAt(a.getTokenOffset()).line, g = e.positionAt(a.getTokenOffset() + a.getTokenLength()).line;
        a.getTokenError() === 1 && u + 1 < e.lineCount ? a.setPosition(e.offsetAt(ke.create(u + 1, 0))) : u < g && (l({ startLine: u, endLine: g, kind: Pt.Comment }), s = u);
        break;
      }
      case 12: {
        var m = e.getText().substr(a.getTokenOffset(), a.getTokenLength()), p = m.match(/^\/\/\s*#(region\b)|(endregion\b)/);
        if (p) {
          var d = e.positionAt(a.getTokenOffset()).line;
          if (p[1]) {
            var h = { startLine: d, endLine: d, kind: Pt.Region };
            i.push(h);
          } else {
            for (var b = i.length - 1; b >= 0 && i[b].kind !== Pt.Region; )
              b--;
            if (b >= 0) {
              var h = i[b];
              i.length = b, d > h.startLine && s !== h.startLine && (h.endLine = d, l(h), s = h.startLine);
            }
          }
        }
        break;
      }
    }
    o = a.scan();
  }
  var y = t && t.rangeLimit;
  if (typeof y != "number" || n.length <= y)
    return n;
  t && t.onRangeLimitExceeded && t.onRangeLimitExceeded(e.uri);
  for (var x = [], v = 0, C = r; v < C.length; v++) {
    var L = C[v];
    L < 30 && (x[L] = (x[L] || 0) + 1);
  }
  for (var _ = 0, S = 0, b = 0; b < x.length; b++) {
    var A = x[b];
    if (A) {
      if (A + _ > y) {
        S = b;
        break;
      }
      _ += A;
    }
  }
  for (var R = [], b = 0; b < n.length; b++) {
    var L = r[b];
    typeof L == "number" && (L < S || L === S && _++ < y) && R.push(n[b]);
  }
  return R;
}
function Eu(e, t, n) {
  function r(o) {
    for (var l = e.offsetAt(o), u = n.getNodeFromOffset(l, !0), h = []; u; ) {
      switch (u.type) {
        case "string":
        case "object":
        case "array":
          var f = u.offset + 1, d = u.offset + u.length - 1;
          f < d && l >= f && l <= d && h.push(i(f, d)), h.push(i(u.offset, u.offset + u.length));
          break;
        case "number":
        case "boolean":
        case "null":
        case "property":
          h.push(i(u.offset, u.offset + u.length));
          break;
      }
      if (u.type === "property" || u.parent && u.parent.type === "array") {
        var g = a(u.offset + u.length, 5);
        g !== -1 && h.push(i(u.offset, g));
      }
      u = u.parent;
    }
    for (var m = void 0, p = h.length - 1; p >= 0; p--)
      m = wn.create(h[p], m);
    return m || (m = wn.create(X.create(o, o))), m;
  }
  function i(o, l) {
    return X.create(e.positionAt(o), e.positionAt(l));
  }
  var s = yt(e.getText(), !0);
  function a(o, l) {
    s.setPosition(o);
    var u = s.scan();
    return u === l ? s.getTokenOffset() + s.getTokenLength() : -1;
  }
  return t.map(r);
}
function Mu(e, t) {
  var n = [];
  return t.visit(function(r) {
    var i;
    if (r.type === "property" && r.keyNode.value === "$ref" && ((i = r.valueNode) === null || i === void 0 ? void 0 : i.type) === "string") {
      var s = r.valueNode.value, a = Pu(t, s);
      if (a) {
        var o = e.positionAt(a.offset);
        n.push({
          target: "".concat(e.uri, "#").concat(o.line + 1, ",").concat(o.character + 1),
          range: Tu(e, r.valueNode)
        });
      }
    }
    return !0;
  }), Promise.resolve(n);
}
function Tu(e, t) {
  return X.create(e.positionAt(t.offset + 1), e.positionAt(t.offset + t.length - 1));
}
function Pu(e, t) {
  var n = Fu(t);
  return n ? wr(n, e.root) : null;
}
function wr(e, t) {
  if (!t)
    return null;
  if (e.length === 0)
    return t;
  var n = e.shift();
  if (t && t.type === "object") {
    var r = t.properties.find(function(a) {
      return a.keyNode.value === n;
    });
    return r ? wr(e, r.valueNode) : null;
  } else if (t && t.type === "array" && n.match(/^(0|[1-9][0-9]*)$/)) {
    var i = Number.parseInt(n), s = t.items[i];
    return s ? wr(e, s) : null;
  }
  return null;
}
function Fu(e) {
  return e === "#" ? [] : e[0] !== "#" || e[1] !== "/" ? null : e.substring(2).split(/\//).map(Iu);
}
function Iu(e) {
  return e.replace(/~1/g, "/").replace(/~0/g, "~");
}
function Vu(e) {
  var t = e.promiseConstructor || Promise, n = new Au(e.schemaRequestService, e.workspaceContext, t);
  n.setSchemaContributions(xr);
  var r = new uu(n, e.contributions, t, e.clientCapabilities), i = new cu(n, e.contributions, t), s = new yu(n), a = new du(n, t);
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
      return lu(o, { collectComments: !0 });
    },
    newJSONDocument: function(o, l) {
      return ou(o, l);
    },
    getMatchingSchemas: n.getMatchingSchemas.bind(n),
    doResolve: r.doResolve.bind(r),
    doComplete: r.doComplete.bind(r),
    findDocumentSymbols: s.findDocumentSymbols.bind(s),
    findDocumentSymbols2: s.findDocumentSymbols2.bind(s),
    findDocumentColors: s.findDocumentColors.bind(s),
    getColorPresentations: s.getColorPresentations.bind(s),
    doHover: i.doHover.bind(i),
    getFoldingRanges: Ru,
    getSelectionRanges: Eu,
    findDefinition: function() {
      return Promise.resolve([]);
    },
    findLinks: Mu,
    format: function(o, l, u) {
      var h = void 0;
      if (l) {
        var f = o.offsetAt(l.start), d = o.offsetAt(l.end) - f;
        h = { offset: f, length: d };
      }
      var g = { tabSize: u ? u.tabSize : 4, insertSpaces: (u == null ? void 0 : u.insertSpaces) === !0, insertFinalNewline: (u == null ? void 0 : u.insertFinalNewline) === !0, eol: `
` };
      return Gl(o.getText(), h, g).map(function(m) {
        return Ee.replace(X.create(o.positionAt(m.offset), o.positionAt(m.offset + m.length)), m.content);
      });
    }
  };
}
var va;
typeof fetch < "u" && (va = function(e) {
  return fetch(e).then((t) => t.text());
});
var Du = class {
  constructor(e, t) {
    Nt(this, "_ctx");
    Nt(this, "_languageService");
    Nt(this, "_languageSettings");
    Nt(this, "_languageId");
    this._ctx = e, this._languageSettings = t.languageSettings, this._languageId = t.languageId, this._languageService = Vu({
      workspaceContext: {
        resolveRelativePath: (n, r) => {
          const i = r.substr(0, r.lastIndexOf("/") + 1);
          return Bu(i, n);
        }
      },
      schemaRequestService: t.enableSchemaRequest ? va : void 0
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
        return mr.create(e, this._languageId, n.version, n.getValue());
    return null;
  }
}, Ou = "/".charCodeAt(0), Un = ".".charCodeAt(0);
function ju(e) {
  return e.charCodeAt(0) === Ou;
}
function Bu(e, t) {
  if (ju(t)) {
    const n = Lt.parse(e), r = t.split("/");
    return n.with({ path: ba(r) }).toString();
  }
  return qu(e, t);
}
function ba(e) {
  const t = [];
  for (const r of e)
    r.length === 0 || r.length === 1 && r.charCodeAt(0) === Un || (r.length === 2 && r.charCodeAt(0) === Un && r.charCodeAt(1) === Un ? t.pop() : t.push(r));
  e.length > 1 && e[e.length - 1].length === 0 && t.push("");
  let n = t.join("/");
  return e[0].length === 0 && (n = "/" + n), n;
}
function qu(e, ...t) {
  const n = Lt.parse(e), r = n.path.split("/");
  for (let i of t)
    r.push(...i.split("/"));
  return n.with({ path: ba(r) }).toString();
}
self.onmessage = () => {
  la((e, t) => new Du(e, t));
};
