class Hr {
  constructor() {
    this.listeners = [], this.unexpectedErrorHandler = function(t) {
      setTimeout(() => {
        throw t.stack ? Be.isErrorNoTelemetry(t) ? new Be(t.message + `

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
const Wr = new Hr();
function pr(e) {
  zr(e) || Wr.onUnexpectedError(e);
}
function Zt(e) {
  if (e instanceof Error) {
    const { name: t, message: n } = e, r = e.stacktrace || e.stack;
    return {
      $isError: !0,
      name: t,
      message: n,
      stack: r,
      noTelemetry: Be.isErrorNoTelemetry(e)
    };
  }
  return e;
}
const Ct = "Canceled";
function zr(e) {
  return e instanceof $r ? !0 : e instanceof Error && e.name === Ct && e.message === Ct;
}
class $r extends Error {
  constructor() {
    super(Ct), this.name = this.message;
  }
}
class Be extends Error {
  constructor(t) {
    super(t), this.name = "CodeExpectedError";
  }
  static fromError(t) {
    if (t instanceof Be)
      return t;
    const n = new Be();
    return n.message = t.message, n.stack = t.stack, n;
  }
  static isErrorNoTelemetry(t) {
    return t.name === "CodeExpectedError";
  }
}
class xe extends Error {
  constructor(t) {
    super(t || "An unexpected bug occurred."), Object.setPrototypeOf(this, xe.prototype);
  }
}
function Or(e) {
  const t = this;
  let n = !1, r;
  return function() {
    return n || (n = !0, r = e.apply(t, arguments)), r;
  };
}
var ct;
(function(e) {
  function t(_) {
    return _ && typeof _ == "object" && typeof _[Symbol.iterator] == "function";
  }
  e.is = t;
  const n = Object.freeze([]);
  function r() {
    return n;
  }
  e.empty = r;
  function* s(_) {
    yield _;
  }
  e.single = s;
  function i(_) {
    return t(_) ? _ : s(_);
  }
  e.wrap = i;
  function a(_) {
    return _ || n;
  }
  e.from = a;
  function* o(_) {
    for (let x = _.length - 1; x >= 0; x--)
      yield _[x];
  }
  e.reverse = o;
  function c(_) {
    return !_ || _[Symbol.iterator]().next().done === !0;
  }
  e.isEmpty = c;
  function u(_) {
    return _[Symbol.iterator]().next().value;
  }
  e.first = u;
  function h(_, x) {
    for (const S of _)
      if (x(S))
        return !0;
    return !1;
  }
  e.some = h;
  function f(_, x) {
    for (const S of _)
      if (x(S))
        return S;
  }
  e.find = f;
  function* m(_, x) {
    for (const S of _)
      x(S) && (yield S);
  }
  e.filter = m;
  function* d(_, x) {
    let S = 0;
    for (const N of _)
      yield x(N, S++);
  }
  e.map = d;
  function* g(..._) {
    for (const x of _)
      for (const S of x)
        yield S;
  }
  e.concat = g;
  function b(_, x, S) {
    let N = S;
    for (const R of _)
      N = x(N, R);
    return N;
  }
  e.reduce = b;
  function* w(_, x, S = _.length) {
    for (x < 0 && (x += _.length), S < 0 ? S += _.length : S > _.length && (S = _.length); x < S; x++)
      yield _[x];
  }
  e.slice = w;
  function L(_, x = Number.POSITIVE_INFINITY) {
    const S = [];
    if (x === 0)
      return [S, _];
    const N = _[Symbol.iterator]();
    for (let R = 0; R < x; R++) {
      const M = N.next();
      if (M.done)
        return [S, e.empty()];
      S.push(M.value);
    }
    return [S, { [Symbol.iterator]() {
      return N;
    } }];
  }
  e.consume = L;
})(ct || (ct = {}));
function vr(e) {
  if (ct.is(e)) {
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
function Gr(...e) {
  return Ge(() => vr(e));
}
function Ge(e) {
  return {
    dispose: Or(() => {
      e();
    })
  };
}
class He {
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
        vr(this._toDispose);
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
    return this._isDisposed ? He.DISABLE_DISPOSED_WARNING || console.warn(new Error("Trying to add a disposable to a DisposableStore that has already been disposed of. The added object will be leaked!").stack) : this._toDispose.add(t), t;
  }
  /**
   * Deletes the value from the store, but does not dispose it.
   */
  deleteAndLeak(t) {
    t && this._toDispose.has(t) && this._toDispose.delete(t);
  }
}
He.DISABLE_DISPOSED_WARNING = !1;
class je {
  constructor() {
    this._store = new He(), this._store;
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
je.None = Object.freeze({ dispose() {
} });
class H {
  constructor(t) {
    this.element = t, this.next = H.Undefined, this.prev = H.Undefined;
  }
}
H.Undefined = new H(void 0);
class jr {
  constructor() {
    this._first = H.Undefined, this._last = H.Undefined, this._size = 0;
  }
  get size() {
    return this._size;
  }
  isEmpty() {
    return this._first === H.Undefined;
  }
  clear() {
    let t = this._first;
    for (; t !== H.Undefined; ) {
      const n = t.next;
      t.prev = H.Undefined, t.next = H.Undefined, t = n;
    }
    this._first = H.Undefined, this._last = H.Undefined, this._size = 0;
  }
  unshift(t) {
    return this._insert(t, !1);
  }
  push(t) {
    return this._insert(t, !0);
  }
  _insert(t, n) {
    const r = new H(t);
    if (this._first === H.Undefined)
      this._first = r, this._last = r;
    else if (n) {
      const i = this._last;
      this._last = r, r.prev = i, i.next = r;
    } else {
      const i = this._first;
      this._first = r, r.next = i, i.prev = r;
    }
    this._size += 1;
    let s = !1;
    return () => {
      s || (s = !0, this._remove(r));
    };
  }
  shift() {
    if (this._first !== H.Undefined) {
      const t = this._first.element;
      return this._remove(this._first), t;
    }
  }
  pop() {
    if (this._last !== H.Undefined) {
      const t = this._last.element;
      return this._remove(this._last), t;
    }
  }
  _remove(t) {
    if (t.prev !== H.Undefined && t.next !== H.Undefined) {
      const n = t.prev;
      n.next = t.next, t.next.prev = n;
    } else
      t.prev === H.Undefined && t.next === H.Undefined ? (this._first = H.Undefined, this._last = H.Undefined) : t.next === H.Undefined ? (this._last = this._last.prev, this._last.next = H.Undefined) : t.prev === H.Undefined && (this._first = this._first.next, this._first.prev = H.Undefined);
    this._size -= 1;
  }
  *[Symbol.iterator]() {
    let t = this._first;
    for (; t !== H.Undefined; )
      yield t.element, t = t.next;
  }
}
const Qr = globalThis.performance && typeof globalThis.performance.now == "function";
class _t {
  static create(t) {
    return new _t(t);
  }
  constructor(t) {
    this._now = Qr && t === !1 ? Date.now : globalThis.performance.now.bind(globalThis.performance), this._startTime = this._now(), this._stopTime = -1;
  }
  stop() {
    this._stopTime = this._now();
  }
  elapsed() {
    return this._stopTime !== -1 ? this._stopTime - this._startTime : this._now() - this._startTime;
  }
}
var At;
(function(e) {
  e.None = () => je.None;
  function t(p, v) {
    return f(p, () => {
    }, 0, void 0, !0, void 0, v);
  }
  e.defer = t;
  function n(p) {
    return (v, C = null, A) => {
      let k = !1, F;
      return F = p((U) => {
        if (!k)
          return F ? F.dispose() : k = !0, v.call(C, U);
      }, null, A), k && F.dispose(), F;
    };
  }
  e.once = n;
  function r(p, v, C) {
    return u((A, k = null, F) => p((U) => A.call(k, v(U)), null, F), C);
  }
  e.map = r;
  function s(p, v, C) {
    return u((A, k = null, F) => p((U) => {
      v(U), A.call(k, U);
    }, null, F), C);
  }
  e.forEach = s;
  function i(p, v, C) {
    return u((A, k = null, F) => p((U) => v(U) && A.call(k, U), null, F), C);
  }
  e.filter = i;
  function a(p) {
    return p;
  }
  e.signal = a;
  function o(...p) {
    return (v, C = null, A) => {
      const k = Gr(...p.map((F) => F((U) => v.call(C, U))));
      return h(k, A);
    };
  }
  e.any = o;
  function c(p, v, C, A) {
    let k = C;
    return r(p, (F) => (k = v(k, F), k), A);
  }
  e.reduce = c;
  function u(p, v) {
    let C;
    const A = {
      onWillAddFirstListener() {
        C = p(k.fire, k);
      },
      onDidRemoveLastListener() {
        C == null || C.dispose();
      }
    }, k = new ie(A);
    return v == null || v.add(k), k.event;
  }
  function h(p, v) {
    return v instanceof Array ? v.push(p) : v && v.add(p), p;
  }
  function f(p, v, C = 100, A = !1, k = !1, F, U) {
    let K, Y, ye, tt = 0, Se;
    const qr = {
      leakWarningThreshold: F,
      onWillAddFirstListener() {
        K = p((Ir) => {
          tt++, Y = v(Y, Ir), A && !ye && (nt.fire(Y), Y = void 0), Se = () => {
            const Ur = Y;
            Y = void 0, ye = void 0, (!A || tt > 1) && nt.fire(Ur), tt = 0;
          }, typeof C == "number" ? (clearTimeout(ye), ye = setTimeout(Se, C)) : ye === void 0 && (ye = 0, queueMicrotask(Se));
        });
      },
      onWillRemoveListener() {
        k && tt > 0 && (Se == null || Se());
      },
      onDidRemoveLastListener() {
        Se = void 0, K.dispose();
      }
    }, nt = new ie(qr);
    return U == null || U.add(nt), nt.event;
  }
  e.debounce = f;
  function m(p, v = 0, C) {
    return e.debounce(p, (A, k) => A ? (A.push(k), A) : [k], v, void 0, !0, void 0, C);
  }
  e.accumulate = m;
  function d(p, v = (A, k) => A === k, C) {
    let A = !0, k;
    return i(p, (F) => {
      const U = A || !v(F, k);
      return A = !1, k = F, U;
    }, C);
  }
  e.latch = d;
  function g(p, v, C) {
    return [
      e.filter(p, v, C),
      e.filter(p, (A) => !v(A), C)
    ];
  }
  e.split = g;
  function b(p, v = !1, C = [], A) {
    let k = C.slice(), F = p((Y) => {
      k ? k.push(Y) : K.fire(Y);
    });
    A && A.add(F);
    const U = () => {
      k == null || k.forEach((Y) => K.fire(Y)), k = null;
    }, K = new ie({
      onWillAddFirstListener() {
        F || (F = p((Y) => K.fire(Y)), A && A.add(F));
      },
      onDidAddFirstListener() {
        k && (v ? setTimeout(U) : U());
      },
      onDidRemoveLastListener() {
        F && F.dispose(), F = null;
      }
    });
    return A && A.add(K), K.event;
  }
  e.buffer = b;
  function w(p, v) {
    return (A, k, F) => {
      const U = v(new _());
      return p(function(K) {
        const Y = U.evaluate(K);
        Y !== L && A.call(k, Y);
      }, void 0, F);
    };
  }
  e.chain = w;
  const L = Symbol("HaltChainable");
  class _ {
    constructor() {
      this.steps = [];
    }
    map(v) {
      return this.steps.push(v), this;
    }
    forEach(v) {
      return this.steps.push((C) => (v(C), C)), this;
    }
    filter(v) {
      return this.steps.push((C) => v(C) ? C : L), this;
    }
    reduce(v, C) {
      let A = C;
      return this.steps.push((k) => (A = v(A, k), A)), this;
    }
    latch(v = (C, A) => C === A) {
      let C = !0, A;
      return this.steps.push((k) => {
        const F = C || !v(k, A);
        return C = !1, A = k, F ? k : L;
      }), this;
    }
    evaluate(v) {
      for (const C of this.steps)
        if (v = C(v), v === L)
          break;
      return v;
    }
  }
  function x(p, v, C = (A) => A) {
    const A = (...K) => U.fire(C(...K)), k = () => p.on(v, A), F = () => p.removeListener(v, A), U = new ie({ onWillAddFirstListener: k, onDidRemoveLastListener: F });
    return U.event;
  }
  e.fromNodeEventEmitter = x;
  function S(p, v, C = (A) => A) {
    const A = (...K) => U.fire(C(...K)), k = () => p.addEventListener(v, A), F = () => p.removeEventListener(v, A), U = new ie({ onWillAddFirstListener: k, onDidRemoveLastListener: F });
    return U.event;
  }
  e.fromDOMEventEmitter = S;
  function N(p) {
    return new Promise((v) => n(p)(v));
  }
  e.toPromise = N;
  function R(p) {
    const v = new ie();
    return p.then((C) => {
      v.fire(C);
    }, () => {
      v.fire(void 0);
    }).finally(() => {
      v.dispose();
    }), v.event;
  }
  e.fromPromise = R;
  function M(p, v) {
    return v(void 0), p((C) => v(C));
  }
  e.runAndSubscribe = M;
  function E(p, v) {
    let C = null;
    function A(F) {
      C == null || C.dispose(), C = new He(), v(F, C);
    }
    A(void 0);
    const k = p((F) => A(F));
    return Ge(() => {
      k.dispose(), C == null || C.dispose();
    });
  }
  e.runAndSubscribeWithStore = E;
  class I {
    constructor(v, C) {
      this._observable = v, this._counter = 0, this._hasChanged = !1;
      const A = {
        onWillAddFirstListener: () => {
          v.addObserver(this);
        },
        onDidRemoveLastListener: () => {
          v.removeObserver(this);
        }
      };
      this.emitter = new ie(A), C && C.add(this.emitter);
    }
    beginUpdate(v) {
      this._counter++;
    }
    handlePossibleChange(v) {
    }
    handleChange(v, C) {
      this._hasChanged = !0;
    }
    endUpdate(v) {
      this._counter--, this._counter === 0 && (this._observable.reportChanges(), this._hasChanged && (this._hasChanged = !1, this.emitter.fire(this._observable.get())));
    }
  }
  function G(p, v) {
    return new I(p, v).emitter.event;
  }
  e.fromObservable = G;
  function B(p) {
    return (v) => {
      let C = 0, A = !1;
      const k = {
        beginUpdate() {
          C++;
        },
        endUpdate() {
          C--, C === 0 && (p.reportChanges(), A && (A = !1, v()));
        },
        handlePossibleChange() {
        },
        handleChange() {
          A = !0;
        }
      };
      return p.addObserver(k), p.reportChanges(), {
        dispose() {
          p.removeObserver(k);
        }
      };
    };
  }
  e.fromObservableLight = B;
})(At || (At = {}));
class qe {
  constructor(t) {
    this.listenerCount = 0, this.invocationCount = 0, this.elapsedOverall = 0, this.durations = [], this.name = `${t}_${qe._idPool++}`, qe.all.add(this);
  }
  start(t) {
    this._stopWatch = new _t(), this.listenerCount = t;
  }
  stop() {
    if (this._stopWatch) {
      const t = this._stopWatch.elapsed();
      this.durations.push(t), this.elapsedOverall += t, this.invocationCount += 1, this._stopWatch = void 0;
    }
  }
}
qe.all = /* @__PURE__ */ new Set();
qe._idPool = 0;
let Xr = -1;
class Jr {
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
    const s = this._stacks.get(t.value) || 0;
    if (this._stacks.set(t.value, s + 1), this._warnCountdown -= 1, this._warnCountdown <= 0) {
      this._warnCountdown = r * 0.5;
      let i, a = 0;
      for (const [o, c] of this._stacks)
        (!i || a < c) && (i = o, a = c);
      console.warn(`[${this.name}] potential listener LEAK detected, having ${n} listeners already. MOST frequent listener (${a}):`), console.warn(i);
    }
    return () => {
      const i = this._stacks.get(t.value) || 0;
      this._stacks.set(t.value, i - 1);
    };
  }
}
class jt {
  static create() {
    var t;
    return new jt((t = new Error().stack) !== null && t !== void 0 ? t : "");
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
class xt {
  constructor(t) {
    this.value = t;
  }
}
const Yr = 2;
class ie {
  constructor(t) {
    var n, r, s, i, a;
    this._size = 0, this._options = t, this._leakageMon = !((n = this._options) === null || n === void 0) && n.leakWarningThreshold ? new Jr((s = (r = this._options) === null || r === void 0 ? void 0 : r.leakWarningThreshold) !== null && s !== void 0 ? s : Xr) : void 0, this._perfMon = !((i = this._options) === null || i === void 0) && i._profName ? new qe(this._options._profName) : void 0, this._deliveryQueue = (a = this._options) === null || a === void 0 ? void 0 : a.deliveryQueue;
  }
  dispose() {
    var t, n, r, s;
    this._disposed || (this._disposed = !0, ((t = this._deliveryQueue) === null || t === void 0 ? void 0 : t.current) === this && this._deliveryQueue.reset(), this._listeners && (this._listeners = void 0, this._size = 0), (r = (n = this._options) === null || n === void 0 ? void 0 : n.onDidRemoveLastListener) === null || r === void 0 || r.call(n), (s = this._leakageMon) === null || s === void 0 || s.dispose());
  }
  /**
   * For the public to allow to subscribe
   * to events from this Emitter
   */
  get event() {
    var t;
    return (t = this._event) !== null && t !== void 0 || (this._event = (n, r, s) => {
      var i, a, o, c, u;
      if (this._leakageMon && this._size > this._leakageMon.threshold * 3)
        return console.warn(`[${this._leakageMon.name}] REFUSES to accept new listeners because it exceeded its threshold by far`), je.None;
      if (this._disposed)
        return je.None;
      r && (n = n.bind(r));
      const h = new xt(n);
      let f;
      this._leakageMon && this._size >= Math.ceil(this._leakageMon.threshold * 0.2) && (h.stack = jt.create(), f = this._leakageMon.check(h.stack, this._size + 1)), this._listeners ? this._listeners instanceof xt ? ((u = this._deliveryQueue) !== null && u !== void 0 || (this._deliveryQueue = new Zr()), this._listeners = [this._listeners, h]) : this._listeners.push(h) : ((a = (i = this._options) === null || i === void 0 ? void 0 : i.onWillAddFirstListener) === null || a === void 0 || a.call(i, this), this._listeners = h, (c = (o = this._options) === null || o === void 0 ? void 0 : o.onDidAddFirstListener) === null || c === void 0 || c.call(o, this)), this._size++;
      const m = Ge(() => {
        f == null || f(), this._removeListener(h);
      });
      return s instanceof He ? s.add(m) : Array.isArray(s) && s.push(m), m;
    }), this._event;
  }
  _removeListener(t) {
    var n, r, s, i;
    if ((r = (n = this._options) === null || n === void 0 ? void 0 : n.onWillRemoveListener) === null || r === void 0 || r.call(n, this), !this._listeners)
      return;
    if (this._size === 1) {
      this._listeners = void 0, (i = (s = this._options) === null || s === void 0 ? void 0 : s.onDidRemoveLastListener) === null || i === void 0 || i.call(s, this), this._size = 0;
      return;
    }
    const a = this._listeners, o = a.indexOf(t);
    if (o === -1)
      throw console.log("disposed?", this._disposed), console.log("size?", this._size), console.log("arr?", JSON.stringify(this._listeners)), new Error("Attempted to dispose unknown listener");
    this._size--, a[o] = void 0;
    const c = this._deliveryQueue.current === this;
    if (this._size * Yr <= a.length) {
      let u = 0;
      for (let h = 0; h < a.length; h++)
        a[h] ? a[u++] = a[h] : c && (this._deliveryQueue.end--, u < this._deliveryQueue.i && this._deliveryQueue.i--);
      a.length = u;
    }
  }
  _deliver(t, n) {
    var r;
    if (!t)
      return;
    const s = ((r = this._options) === null || r === void 0 ? void 0 : r.onListenerError) || pr;
    if (!s) {
      t.value(n);
      return;
    }
    try {
      t.value(n);
    } catch (i) {
      s(i);
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
    var n, r, s, i;
    if (!((n = this._deliveryQueue) === null || n === void 0) && n.current && (this._deliverQueue(this._deliveryQueue), (r = this._perfMon) === null || r === void 0 || r.stop()), (s = this._perfMon) === null || s === void 0 || s.start(this._size), this._listeners)
      if (this._listeners instanceof xt)
        this._deliver(this._listeners, t);
      else {
        const a = this._deliveryQueue;
        a.enqueue(this, t, this._listeners.length), this._deliverQueue(a);
      }
    (i = this._perfMon) === null || i === void 0 || i.stop();
  }
  hasListeners() {
    return this._size > 0;
  }
}
class Zr {
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
function Kr(e) {
  return typeof e == "string";
}
function es(e) {
  let t = [];
  for (; Object.prototype !== e; )
    t = t.concat(Object.getOwnPropertyNames(e)), e = Object.getPrototypeOf(e);
  return t;
}
function Rt(e) {
  const t = [];
  for (const n of es(e))
    typeof e[n] == "function" && t.push(n);
  return t;
}
function ts(e, t) {
  const n = (s) => function() {
    const i = Array.prototype.slice.call(arguments, 0);
    return t(s, i);
  }, r = {};
  for (const s of e)
    r[s] = n(s);
  return r;
}
globalThis && globalThis.__awaiter;
let ns = typeof document < "u" && document.location && document.location.hash.indexOf("pseudo=true") >= 0;
function rs(e, t) {
  let n;
  return t.length === 0 ? n = e : n = e.replace(/\{(\d+)\}/g, (r, s) => {
    const i = s[0], a = t[i];
    let o = r;
    return typeof a == "string" ? o = a : (typeof a == "number" || typeof a == "boolean" || a === void 0 || a === null) && (o = String(a)), o;
  }), ns && (n = "［" + n.replace(/[aouei]/g, "$&$&") + "］"), n;
}
function z(e, t, ...n) {
  return rs(t, n);
}
var pt;
const Fe = "en";
let yt = !1, Mt = !1, vt = !1, wr = !1, rt, wt = Fe, Kt = Fe, ss, se;
const le = typeof self == "object" ? self : typeof global == "object" ? global : {};
let X;
typeof le.vscode < "u" && typeof le.vscode.process < "u" ? X = le.vscode.process : typeof process < "u" && (X = process);
const is = typeof ((pt = X == null ? void 0 : X.versions) === null || pt === void 0 ? void 0 : pt.electron) == "string", as = is && (X == null ? void 0 : X.type) === "renderer";
if (typeof navigator == "object" && !as)
  se = navigator.userAgent, yt = se.indexOf("Windows") >= 0, Mt = se.indexOf("Macintosh") >= 0, (se.indexOf("Macintosh") >= 0 || se.indexOf("iPad") >= 0 || se.indexOf("iPhone") >= 0) && navigator.maxTouchPoints && navigator.maxTouchPoints > 0, vt = se.indexOf("Linux") >= 0, (se == null ? void 0 : se.indexOf("Mobi")) >= 0, wr = !0, // This call _must_ be done in the file that calls `nls.getConfiguredDefaultLocale`
  // to ensure that the NLS AMD Loader plugin has been loaded and configured.
  // This is because the loader plugin decides what the default locale is based on
  // how it's able to resolve the strings.
  z({ key: "ensureLoaderPluginIsLoaded", comment: ["{Locked}"] }, "_"), rt = Fe, wt = rt, Kt = navigator.language;
else if (typeof X == "object") {
  yt = X.platform === "win32", Mt = X.platform === "darwin", vt = X.platform === "linux", vt && X.env.SNAP && X.env.SNAP_REVISION, X.env.CI || X.env.BUILD_ARTIFACTSTAGINGDIRECTORY, rt = Fe, wt = Fe;
  const e = X.env.VSCODE_NLS_CONFIG;
  if (e)
    try {
      const t = JSON.parse(e), n = t.availableLanguages["*"];
      rt = t.locale, Kt = t.osLocale, wt = n || Fe, ss = t._translationsConfigFile;
    } catch {
    }
} else
  console.error("Unable to resolve platform.");
const Qe = yt, ls = Mt;
wr && le.importScripts;
const ce = se, os = typeof le.postMessage == "function" && !le.importScripts;
(() => {
  if (os) {
    const e = [];
    le.addEventListener("message", (n) => {
      if (n.data && n.data.vscodeScheduleAsyncWork)
        for (let r = 0, s = e.length; r < s; r++) {
          const i = e[r];
          if (i.id === n.data.vscodeScheduleAsyncWork) {
            e.splice(r, 1), i.callback();
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
      }), le.postMessage({ vscodeScheduleAsyncWork: r }, "*");
    };
  }
  return (e) => setTimeout(e);
})();
const us = !!(ce && ce.indexOf("Chrome") >= 0);
ce && ce.indexOf("Firefox") >= 0;
!us && ce && ce.indexOf("Safari") >= 0;
ce && ce.indexOf("Edg/") >= 0;
ce && ce.indexOf("Android") >= 0;
class cs {
  constructor(t) {
    this.fn = t, this.lastCache = void 0, this.lastArgKey = void 0;
  }
  get(t) {
    const n = JSON.stringify(t);
    return this.lastArgKey !== n && (this.lastArgKey = n, this.lastCache = this.fn(t)), this.lastCache;
  }
}
class Lr {
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
var Ie;
function hs(e) {
  return e.replace(/[\\\{\}\*\+\?\|\^\$\.\[\]\(\)]/g, "\\$&");
}
function fs(e) {
  return e.split(/\r\n|\r|\n/);
}
function ds(e) {
  for (let t = 0, n = e.length; t < n; t++) {
    const r = e.charCodeAt(t);
    if (r !== 32 && r !== 9)
      return t;
  }
  return -1;
}
function ms(e, t = e.length - 1) {
  for (let n = t; n >= 0; n--) {
    const r = e.charCodeAt(n);
    if (r !== 32 && r !== 9)
      return n;
  }
  return -1;
}
function Nr(e) {
  return e >= 65 && e <= 90;
}
function Et(e) {
  return 55296 <= e && e <= 56319;
}
function gs(e) {
  return 56320 <= e && e <= 57343;
}
function bs(e, t) {
  return (e - 55296 << 10) + (t - 56320) + 65536;
}
function _s(e, t, n) {
  const r = e.charCodeAt(n);
  if (Et(r) && n + 1 < t) {
    const s = e.charCodeAt(n + 1);
    if (gs(s))
      return bs(r, s);
  }
  return r;
}
const xs = /^[\t\n\r\x20-\x7E]*$/;
function ps(e) {
  return xs.test(e);
}
class Re {
  static getInstance(t) {
    return Ie.cache.get(Array.from(t));
  }
  static getLocales() {
    return Ie._locales.value;
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
Ie = Re;
Re.ambiguousCharacterData = new Lr(() => JSON.parse('{"_common":[8232,32,8233,32,5760,32,8192,32,8193,32,8194,32,8195,32,8196,32,8197,32,8198,32,8200,32,8201,32,8202,32,8287,32,8199,32,8239,32,2042,95,65101,95,65102,95,65103,95,8208,45,8209,45,8210,45,65112,45,1748,45,8259,45,727,45,8722,45,10134,45,11450,45,1549,44,1643,44,8218,44,184,44,42233,44,894,59,2307,58,2691,58,1417,58,1795,58,1796,58,5868,58,65072,58,6147,58,6153,58,8282,58,1475,58,760,58,42889,58,8758,58,720,58,42237,58,451,33,11601,33,660,63,577,63,2429,63,5038,63,42731,63,119149,46,8228,46,1793,46,1794,46,42510,46,68176,46,1632,46,1776,46,42232,46,1373,96,65287,96,8219,96,8242,96,1370,96,1523,96,8175,96,65344,96,900,96,8189,96,8125,96,8127,96,8190,96,697,96,884,96,712,96,714,96,715,96,756,96,699,96,701,96,700,96,702,96,42892,96,1497,96,2036,96,2037,96,5194,96,5836,96,94033,96,94034,96,65339,91,10088,40,10098,40,12308,40,64830,40,65341,93,10089,41,10099,41,12309,41,64831,41,10100,123,119060,123,10101,125,65342,94,8270,42,1645,42,8727,42,66335,42,5941,47,8257,47,8725,47,8260,47,9585,47,10187,47,10744,47,119354,47,12755,47,12339,47,11462,47,20031,47,12035,47,65340,92,65128,92,8726,92,10189,92,10741,92,10745,92,119311,92,119355,92,12756,92,20022,92,12034,92,42872,38,708,94,710,94,5869,43,10133,43,66203,43,8249,60,10094,60,706,60,119350,60,5176,60,5810,60,5120,61,11840,61,12448,61,42239,61,8250,62,10095,62,707,62,119351,62,5171,62,94015,62,8275,126,732,126,8128,126,8764,126,65372,124,65293,45,120784,50,120794,50,120804,50,120814,50,120824,50,130034,50,42842,50,423,50,1000,50,42564,50,5311,50,42735,50,119302,51,120785,51,120795,51,120805,51,120815,51,120825,51,130035,51,42923,51,540,51,439,51,42858,51,11468,51,1248,51,94011,51,71882,51,120786,52,120796,52,120806,52,120816,52,120826,52,130036,52,5070,52,71855,52,120787,53,120797,53,120807,53,120817,53,120827,53,130037,53,444,53,71867,53,120788,54,120798,54,120808,54,120818,54,120828,54,130038,54,11474,54,5102,54,71893,54,119314,55,120789,55,120799,55,120809,55,120819,55,120829,55,130039,55,66770,55,71878,55,2819,56,2538,56,2666,56,125131,56,120790,56,120800,56,120810,56,120820,56,120830,56,130040,56,547,56,546,56,66330,56,2663,57,2920,57,2541,57,3437,57,120791,57,120801,57,120811,57,120821,57,120831,57,130041,57,42862,57,11466,57,71884,57,71852,57,71894,57,9082,97,65345,97,119834,97,119886,97,119938,97,119990,97,120042,97,120094,97,120146,97,120198,97,120250,97,120302,97,120354,97,120406,97,120458,97,593,97,945,97,120514,97,120572,97,120630,97,120688,97,120746,97,65313,65,119808,65,119860,65,119912,65,119964,65,120016,65,120068,65,120120,65,120172,65,120224,65,120276,65,120328,65,120380,65,120432,65,913,65,120488,65,120546,65,120604,65,120662,65,120720,65,5034,65,5573,65,42222,65,94016,65,66208,65,119835,98,119887,98,119939,98,119991,98,120043,98,120095,98,120147,98,120199,98,120251,98,120303,98,120355,98,120407,98,120459,98,388,98,5071,98,5234,98,5551,98,65314,66,8492,66,119809,66,119861,66,119913,66,120017,66,120069,66,120121,66,120173,66,120225,66,120277,66,120329,66,120381,66,120433,66,42932,66,914,66,120489,66,120547,66,120605,66,120663,66,120721,66,5108,66,5623,66,42192,66,66178,66,66209,66,66305,66,65347,99,8573,99,119836,99,119888,99,119940,99,119992,99,120044,99,120096,99,120148,99,120200,99,120252,99,120304,99,120356,99,120408,99,120460,99,7428,99,1010,99,11429,99,43951,99,66621,99,128844,67,71922,67,71913,67,65315,67,8557,67,8450,67,8493,67,119810,67,119862,67,119914,67,119966,67,120018,67,120174,67,120226,67,120278,67,120330,67,120382,67,120434,67,1017,67,11428,67,5087,67,42202,67,66210,67,66306,67,66581,67,66844,67,8574,100,8518,100,119837,100,119889,100,119941,100,119993,100,120045,100,120097,100,120149,100,120201,100,120253,100,120305,100,120357,100,120409,100,120461,100,1281,100,5095,100,5231,100,42194,100,8558,68,8517,68,119811,68,119863,68,119915,68,119967,68,120019,68,120071,68,120123,68,120175,68,120227,68,120279,68,120331,68,120383,68,120435,68,5024,68,5598,68,5610,68,42195,68,8494,101,65349,101,8495,101,8519,101,119838,101,119890,101,119942,101,120046,101,120098,101,120150,101,120202,101,120254,101,120306,101,120358,101,120410,101,120462,101,43826,101,1213,101,8959,69,65317,69,8496,69,119812,69,119864,69,119916,69,120020,69,120072,69,120124,69,120176,69,120228,69,120280,69,120332,69,120384,69,120436,69,917,69,120492,69,120550,69,120608,69,120666,69,120724,69,11577,69,5036,69,42224,69,71846,69,71854,69,66182,69,119839,102,119891,102,119943,102,119995,102,120047,102,120099,102,120151,102,120203,102,120255,102,120307,102,120359,102,120411,102,120463,102,43829,102,42905,102,383,102,7837,102,1412,102,119315,70,8497,70,119813,70,119865,70,119917,70,120021,70,120073,70,120125,70,120177,70,120229,70,120281,70,120333,70,120385,70,120437,70,42904,70,988,70,120778,70,5556,70,42205,70,71874,70,71842,70,66183,70,66213,70,66853,70,65351,103,8458,103,119840,103,119892,103,119944,103,120048,103,120100,103,120152,103,120204,103,120256,103,120308,103,120360,103,120412,103,120464,103,609,103,7555,103,397,103,1409,103,119814,71,119866,71,119918,71,119970,71,120022,71,120074,71,120126,71,120178,71,120230,71,120282,71,120334,71,120386,71,120438,71,1292,71,5056,71,5107,71,42198,71,65352,104,8462,104,119841,104,119945,104,119997,104,120049,104,120101,104,120153,104,120205,104,120257,104,120309,104,120361,104,120413,104,120465,104,1211,104,1392,104,5058,104,65320,72,8459,72,8460,72,8461,72,119815,72,119867,72,119919,72,120023,72,120179,72,120231,72,120283,72,120335,72,120387,72,120439,72,919,72,120494,72,120552,72,120610,72,120668,72,120726,72,11406,72,5051,72,5500,72,42215,72,66255,72,731,105,9075,105,65353,105,8560,105,8505,105,8520,105,119842,105,119894,105,119946,105,119998,105,120050,105,120102,105,120154,105,120206,105,120258,105,120310,105,120362,105,120414,105,120466,105,120484,105,618,105,617,105,953,105,8126,105,890,105,120522,105,120580,105,120638,105,120696,105,120754,105,1110,105,42567,105,1231,105,43893,105,5029,105,71875,105,65354,106,8521,106,119843,106,119895,106,119947,106,119999,106,120051,106,120103,106,120155,106,120207,106,120259,106,120311,106,120363,106,120415,106,120467,106,1011,106,1112,106,65322,74,119817,74,119869,74,119921,74,119973,74,120025,74,120077,74,120129,74,120181,74,120233,74,120285,74,120337,74,120389,74,120441,74,42930,74,895,74,1032,74,5035,74,5261,74,42201,74,119844,107,119896,107,119948,107,120000,107,120052,107,120104,107,120156,107,120208,107,120260,107,120312,107,120364,107,120416,107,120468,107,8490,75,65323,75,119818,75,119870,75,119922,75,119974,75,120026,75,120078,75,120130,75,120182,75,120234,75,120286,75,120338,75,120390,75,120442,75,922,75,120497,75,120555,75,120613,75,120671,75,120729,75,11412,75,5094,75,5845,75,42199,75,66840,75,1472,108,8739,73,9213,73,65512,73,1633,108,1777,73,66336,108,125127,108,120783,73,120793,73,120803,73,120813,73,120823,73,130033,73,65321,73,8544,73,8464,73,8465,73,119816,73,119868,73,119920,73,120024,73,120128,73,120180,73,120232,73,120284,73,120336,73,120388,73,120440,73,65356,108,8572,73,8467,108,119845,108,119897,108,119949,108,120001,108,120053,108,120105,73,120157,73,120209,73,120261,73,120313,73,120365,73,120417,73,120469,73,448,73,120496,73,120554,73,120612,73,120670,73,120728,73,11410,73,1030,73,1216,73,1493,108,1503,108,1575,108,126464,108,126592,108,65166,108,65165,108,1994,108,11599,73,5825,73,42226,73,93992,73,66186,124,66313,124,119338,76,8556,76,8466,76,119819,76,119871,76,119923,76,120027,76,120079,76,120131,76,120183,76,120235,76,120287,76,120339,76,120391,76,120443,76,11472,76,5086,76,5290,76,42209,76,93974,76,71843,76,71858,76,66587,76,66854,76,65325,77,8559,77,8499,77,119820,77,119872,77,119924,77,120028,77,120080,77,120132,77,120184,77,120236,77,120288,77,120340,77,120392,77,120444,77,924,77,120499,77,120557,77,120615,77,120673,77,120731,77,1018,77,11416,77,5047,77,5616,77,5846,77,42207,77,66224,77,66321,77,119847,110,119899,110,119951,110,120003,110,120055,110,120107,110,120159,110,120211,110,120263,110,120315,110,120367,110,120419,110,120471,110,1400,110,1404,110,65326,78,8469,78,119821,78,119873,78,119925,78,119977,78,120029,78,120081,78,120185,78,120237,78,120289,78,120341,78,120393,78,120445,78,925,78,120500,78,120558,78,120616,78,120674,78,120732,78,11418,78,42208,78,66835,78,3074,111,3202,111,3330,111,3458,111,2406,111,2662,111,2790,111,3046,111,3174,111,3302,111,3430,111,3664,111,3792,111,4160,111,1637,111,1781,111,65359,111,8500,111,119848,111,119900,111,119952,111,120056,111,120108,111,120160,111,120212,111,120264,111,120316,111,120368,111,120420,111,120472,111,7439,111,7441,111,43837,111,959,111,120528,111,120586,111,120644,111,120702,111,120760,111,963,111,120532,111,120590,111,120648,111,120706,111,120764,111,11423,111,4351,111,1413,111,1505,111,1607,111,126500,111,126564,111,126596,111,65259,111,65260,111,65258,111,65257,111,1726,111,64428,111,64429,111,64427,111,64426,111,1729,111,64424,111,64425,111,64423,111,64422,111,1749,111,3360,111,4125,111,66794,111,71880,111,71895,111,66604,111,1984,79,2534,79,2918,79,12295,79,70864,79,71904,79,120782,79,120792,79,120802,79,120812,79,120822,79,130032,79,65327,79,119822,79,119874,79,119926,79,119978,79,120030,79,120082,79,120134,79,120186,79,120238,79,120290,79,120342,79,120394,79,120446,79,927,79,120502,79,120560,79,120618,79,120676,79,120734,79,11422,79,1365,79,11604,79,4816,79,2848,79,66754,79,42227,79,71861,79,66194,79,66219,79,66564,79,66838,79,9076,112,65360,112,119849,112,119901,112,119953,112,120005,112,120057,112,120109,112,120161,112,120213,112,120265,112,120317,112,120369,112,120421,112,120473,112,961,112,120530,112,120544,112,120588,112,120602,112,120646,112,120660,112,120704,112,120718,112,120762,112,120776,112,11427,112,65328,80,8473,80,119823,80,119875,80,119927,80,119979,80,120031,80,120083,80,120187,80,120239,80,120291,80,120343,80,120395,80,120447,80,929,80,120504,80,120562,80,120620,80,120678,80,120736,80,11426,80,5090,80,5229,80,42193,80,66197,80,119850,113,119902,113,119954,113,120006,113,120058,113,120110,113,120162,113,120214,113,120266,113,120318,113,120370,113,120422,113,120474,113,1307,113,1379,113,1382,113,8474,81,119824,81,119876,81,119928,81,119980,81,120032,81,120084,81,120188,81,120240,81,120292,81,120344,81,120396,81,120448,81,11605,81,119851,114,119903,114,119955,114,120007,114,120059,114,120111,114,120163,114,120215,114,120267,114,120319,114,120371,114,120423,114,120475,114,43847,114,43848,114,7462,114,11397,114,43905,114,119318,82,8475,82,8476,82,8477,82,119825,82,119877,82,119929,82,120033,82,120189,82,120241,82,120293,82,120345,82,120397,82,120449,82,422,82,5025,82,5074,82,66740,82,5511,82,42211,82,94005,82,65363,115,119852,115,119904,115,119956,115,120008,115,120060,115,120112,115,120164,115,120216,115,120268,115,120320,115,120372,115,120424,115,120476,115,42801,115,445,115,1109,115,43946,115,71873,115,66632,115,65331,83,119826,83,119878,83,119930,83,119982,83,120034,83,120086,83,120138,83,120190,83,120242,83,120294,83,120346,83,120398,83,120450,83,1029,83,1359,83,5077,83,5082,83,42210,83,94010,83,66198,83,66592,83,119853,116,119905,116,119957,116,120009,116,120061,116,120113,116,120165,116,120217,116,120269,116,120321,116,120373,116,120425,116,120477,116,8868,84,10201,84,128872,84,65332,84,119827,84,119879,84,119931,84,119983,84,120035,84,120087,84,120139,84,120191,84,120243,84,120295,84,120347,84,120399,84,120451,84,932,84,120507,84,120565,84,120623,84,120681,84,120739,84,11430,84,5026,84,42196,84,93962,84,71868,84,66199,84,66225,84,66325,84,119854,117,119906,117,119958,117,120010,117,120062,117,120114,117,120166,117,120218,117,120270,117,120322,117,120374,117,120426,117,120478,117,42911,117,7452,117,43854,117,43858,117,651,117,965,117,120534,117,120592,117,120650,117,120708,117,120766,117,1405,117,66806,117,71896,117,8746,85,8899,85,119828,85,119880,85,119932,85,119984,85,120036,85,120088,85,120140,85,120192,85,120244,85,120296,85,120348,85,120400,85,120452,85,1357,85,4608,85,66766,85,5196,85,42228,85,94018,85,71864,85,8744,118,8897,118,65366,118,8564,118,119855,118,119907,118,119959,118,120011,118,120063,118,120115,118,120167,118,120219,118,120271,118,120323,118,120375,118,120427,118,120479,118,7456,118,957,118,120526,118,120584,118,120642,118,120700,118,120758,118,1141,118,1496,118,71430,118,43945,118,71872,118,119309,86,1639,86,1783,86,8548,86,119829,86,119881,86,119933,86,119985,86,120037,86,120089,86,120141,86,120193,86,120245,86,120297,86,120349,86,120401,86,120453,86,1140,86,11576,86,5081,86,5167,86,42719,86,42214,86,93960,86,71840,86,66845,86,623,119,119856,119,119908,119,119960,119,120012,119,120064,119,120116,119,120168,119,120220,119,120272,119,120324,119,120376,119,120428,119,120480,119,7457,119,1121,119,1309,119,1377,119,71434,119,71438,119,71439,119,43907,119,71919,87,71910,87,119830,87,119882,87,119934,87,119986,87,120038,87,120090,87,120142,87,120194,87,120246,87,120298,87,120350,87,120402,87,120454,87,1308,87,5043,87,5076,87,42218,87,5742,120,10539,120,10540,120,10799,120,65368,120,8569,120,119857,120,119909,120,119961,120,120013,120,120065,120,120117,120,120169,120,120221,120,120273,120,120325,120,120377,120,120429,120,120481,120,5441,120,5501,120,5741,88,9587,88,66338,88,71916,88,65336,88,8553,88,119831,88,119883,88,119935,88,119987,88,120039,88,120091,88,120143,88,120195,88,120247,88,120299,88,120351,88,120403,88,120455,88,42931,88,935,88,120510,88,120568,88,120626,88,120684,88,120742,88,11436,88,11613,88,5815,88,42219,88,66192,88,66228,88,66327,88,66855,88,611,121,7564,121,65369,121,119858,121,119910,121,119962,121,120014,121,120066,121,120118,121,120170,121,120222,121,120274,121,120326,121,120378,121,120430,121,120482,121,655,121,7935,121,43866,121,947,121,8509,121,120516,121,120574,121,120632,121,120690,121,120748,121,1199,121,4327,121,71900,121,65337,89,119832,89,119884,89,119936,89,119988,89,120040,89,120092,89,120144,89,120196,89,120248,89,120300,89,120352,89,120404,89,120456,89,933,89,978,89,120508,89,120566,89,120624,89,120682,89,120740,89,11432,89,1198,89,5033,89,5053,89,42220,89,94019,89,71844,89,66226,89,119859,122,119911,122,119963,122,120015,122,120067,122,120119,122,120171,122,120223,122,120275,122,120327,122,120379,122,120431,122,120483,122,7458,122,43923,122,71876,122,66293,90,71909,90,65338,90,8484,90,8488,90,119833,90,119885,90,119937,90,119989,90,120041,90,120197,90,120249,90,120301,90,120353,90,120405,90,120457,90,918,90,120493,90,120551,90,120609,90,120667,90,120725,90,5059,90,42204,90,71849,90,65282,34,65284,36,65285,37,65286,38,65290,42,65291,43,65294,46,65295,47,65296,48,65297,49,65298,50,65299,51,65300,52,65301,53,65302,54,65303,55,65304,56,65305,57,65308,60,65309,61,65310,62,65312,64,65316,68,65318,70,65319,71,65324,76,65329,81,65330,82,65333,85,65334,86,65335,87,65343,95,65346,98,65348,100,65350,102,65355,107,65357,109,65358,110,65361,113,65362,114,65364,116,65365,117,65367,119,65370,122,65371,123,65373,125,119846,109],"_default":[160,32,8211,45,65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"cs":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"de":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"es":[8211,45,65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"fr":[65374,126,65306,58,65281,33,8216,96,8245,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"it":[160,32,8211,45,65374,126,65306,58,65281,33,8216,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"ja":[8211,45,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65292,44,65307,59],"ko":[8211,45,65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"pl":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"pt-BR":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"qps-ploc":[160,32,8211,45,65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"ru":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,305,105,921,73,1009,112,215,120,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"tr":[160,32,8211,45,65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"zh-hans":[65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65288,40,65289,41],"zh-hant":[8211,45,65374,126,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65307,59]}'));
Re.cache = new cs((e) => {
  function t(u) {
    const h = /* @__PURE__ */ new Map();
    for (let f = 0; f < u.length; f += 2)
      h.set(u[f], u[f + 1]);
    return h;
  }
  function n(u, h) {
    const f = new Map(u);
    for (const [m, d] of h)
      f.set(m, d);
    return f;
  }
  function r(u, h) {
    if (!u)
      return h;
    const f = /* @__PURE__ */ new Map();
    for (const [m, d] of u)
      h.has(m) && f.set(m, d);
    return f;
  }
  const s = Ie.ambiguousCharacterData.value;
  let i = e.filter((u) => !u.startsWith("_") && u in s);
  i.length === 0 && (i = ["_default"]);
  let a;
  for (const u of i) {
    const h = t(s[u]);
    a = r(a, h);
  }
  const o = t(s._common), c = n(o, a);
  return new Ie(c);
});
Re._locales = new Lr(() => Object.keys(Ie.ambiguousCharacterData.value).filter((e) => !e.startsWith("_")));
class pe {
  static getRawData() {
    return JSON.parse("[9,10,11,12,13,32,127,160,173,847,1564,4447,4448,6068,6069,6155,6156,6157,6158,7355,7356,8192,8193,8194,8195,8196,8197,8198,8199,8200,8201,8202,8203,8204,8205,8206,8207,8234,8235,8236,8237,8238,8239,8287,8288,8289,8290,8291,8292,8293,8294,8295,8296,8297,8298,8299,8300,8301,8302,8303,10240,12288,12644,65024,65025,65026,65027,65028,65029,65030,65031,65032,65033,65034,65035,65036,65037,65038,65039,65279,65440,65520,65521,65522,65523,65524,65525,65526,65527,65528,65532,78844,119155,119156,119157,119158,119159,119160,119161,119162,917504,917505,917506,917507,917508,917509,917510,917511,917512,917513,917514,917515,917516,917517,917518,917519,917520,917521,917522,917523,917524,917525,917526,917527,917528,917529,917530,917531,917532,917533,917534,917535,917536,917537,917538,917539,917540,917541,917542,917543,917544,917545,917546,917547,917548,917549,917550,917551,917552,917553,917554,917555,917556,917557,917558,917559,917560,917561,917562,917563,917564,917565,917566,917567,917568,917569,917570,917571,917572,917573,917574,917575,917576,917577,917578,917579,917580,917581,917582,917583,917584,917585,917586,917587,917588,917589,917590,917591,917592,917593,917594,917595,917596,917597,917598,917599,917600,917601,917602,917603,917604,917605,917606,917607,917608,917609,917610,917611,917612,917613,917614,917615,917616,917617,917618,917619,917620,917621,917622,917623,917624,917625,917626,917627,917628,917629,917630,917631,917760,917761,917762,917763,917764,917765,917766,917767,917768,917769,917770,917771,917772,917773,917774,917775,917776,917777,917778,917779,917780,917781,917782,917783,917784,917785,917786,917787,917788,917789,917790,917791,917792,917793,917794,917795,917796,917797,917798,917799,917800,917801,917802,917803,917804,917805,917806,917807,917808,917809,917810,917811,917812,917813,917814,917815,917816,917817,917818,917819,917820,917821,917822,917823,917824,917825,917826,917827,917828,917829,917830,917831,917832,917833,917834,917835,917836,917837,917838,917839,917840,917841,917842,917843,917844,917845,917846,917847,917848,917849,917850,917851,917852,917853,917854,917855,917856,917857,917858,917859,917860,917861,917862,917863,917864,917865,917866,917867,917868,917869,917870,917871,917872,917873,917874,917875,917876,917877,917878,917879,917880,917881,917882,917883,917884,917885,917886,917887,917888,917889,917890,917891,917892,917893,917894,917895,917896,917897,917898,917899,917900,917901,917902,917903,917904,917905,917906,917907,917908,917909,917910,917911,917912,917913,917914,917915,917916,917917,917918,917919,917920,917921,917922,917923,917924,917925,917926,917927,917928,917929,917930,917931,917932,917933,917934,917935,917936,917937,917938,917939,917940,917941,917942,917943,917944,917945,917946,917947,917948,917949,917950,917951,917952,917953,917954,917955,917956,917957,917958,917959,917960,917961,917962,917963,917964,917965,917966,917967,917968,917969,917970,917971,917972,917973,917974,917975,917976,917977,917978,917979,917980,917981,917982,917983,917984,917985,917986,917987,917988,917989,917990,917991,917992,917993,917994,917995,917996,917997,917998,917999]");
  }
  static getData() {
    return this._data || (this._data = new Set(pe.getRawData())), this._data;
  }
  static isInvisibleCharacter(t) {
    return pe.getData().has(t);
  }
  static get codePoints() {
    return pe.getData();
  }
}
pe._data = void 0;
const vs = "$initialize";
class ws {
  constructor(t, n, r, s) {
    this.vsWorker = t, this.req = n, this.method = r, this.args = s, this.type = 0;
  }
}
class en {
  constructor(t, n, r, s) {
    this.vsWorker = t, this.seq = n, this.res = r, this.err = s, this.type = 1;
  }
}
class Ls {
  constructor(t, n, r, s) {
    this.vsWorker = t, this.req = n, this.eventName = r, this.arg = s, this.type = 2;
  }
}
class Ns {
  constructor(t, n, r) {
    this.vsWorker = t, this.req = n, this.event = r, this.type = 3;
  }
}
class Ss {
  constructor(t, n) {
    this.vsWorker = t, this.req = n, this.type = 4;
  }
}
class Cs {
  constructor(t) {
    this._workerId = -1, this._handler = t, this._lastSentReq = 0, this._pendingReplies = /* @__PURE__ */ Object.create(null), this._pendingEmitters = /* @__PURE__ */ new Map(), this._pendingEvents = /* @__PURE__ */ new Map();
  }
  setWorkerId(t) {
    this._workerId = t;
  }
  sendMessage(t, n) {
    const r = String(++this._lastSentReq);
    return new Promise((s, i) => {
      this._pendingReplies[r] = {
        resolve: s,
        reject: i
      }, this._send(new ws(this._workerId, r, t, n));
    });
  }
  listen(t, n) {
    let r = null;
    const s = new ie({
      onWillAddFirstListener: () => {
        r = String(++this._lastSentReq), this._pendingEmitters.set(r, s), this._send(new Ls(this._workerId, r, t, n));
      },
      onDidRemoveLastListener: () => {
        this._pendingEmitters.delete(r), this._send(new Ss(this._workerId, r)), r = null;
      }
    });
    return s.event;
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
    this._handler.handleMessage(t.method, t.args).then((s) => {
      this._send(new en(this._workerId, n, s, void 0));
    }, (s) => {
      s.detail instanceof Error && (s.detail = Zt(s.detail)), this._send(new en(this._workerId, n, void 0, Zt(s)));
    });
  }
  _handleSubscribeEventMessage(t) {
    const n = t.req, r = this._handler.handleEvent(t.eventName, t.arg)((s) => {
      this._send(new Ns(this._workerId, n, s));
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
function Sr(e) {
  return e[0] === "o" && e[1] === "n" && Nr(e.charCodeAt(2));
}
function Cr(e) {
  return /^onDynamic/.test(e) && Nr(e.charCodeAt(9));
}
function As(e, t, n) {
  const r = (a) => function() {
    const o = Array.prototype.slice.call(arguments, 0);
    return t(a, o);
  }, s = (a) => function(o) {
    return n(a, o);
  }, i = {};
  for (const a of e) {
    if (Cr(a)) {
      i[a] = s(a);
      continue;
    }
    if (Sr(a)) {
      i[a] = n(a, void 0);
      continue;
    }
    i[a] = r(a);
  }
  return i;
}
class Rs {
  constructor(t, n) {
    this._requestHandlerFactory = n, this._requestHandler = null, this._protocol = new Cs({
      sendMessage: (r, s) => {
        t(r, s);
      },
      handleMessage: (r, s) => this._handleMessage(r, s),
      handleEvent: (r, s) => this._handleEvent(r, s)
    });
  }
  onmessage(t) {
    this._protocol.handleMessage(t);
  }
  _handleMessage(t, n) {
    if (t === vs)
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
    if (Cr(t)) {
      const r = this._requestHandler[t].call(this._requestHandler, n);
      if (typeof r != "function")
        throw new Error(`Missing dynamic event ${t} on request handler.`);
      return r;
    }
    if (Sr(t)) {
      const r = this._requestHandler[t];
      if (typeof r != "function")
        throw new Error(`Missing event ${t} on request handler.`);
      return r;
    }
    throw new Error(`Malformed event name ${t}`);
  }
  initialize(t, n, r, s) {
    this._protocol.setWorkerId(t);
    const o = As(s, (c, u) => this._protocol.sendMessage(c, u), (c, u) => this._protocol.listen(c, u));
    return this._requestHandlerFactory ? (this._requestHandler = this._requestHandlerFactory(o), Promise.resolve(Rt(this._requestHandler))) : (n && (typeof n.baseUrl < "u" && delete n.baseUrl, typeof n.paths < "u" && typeof n.paths.vs < "u" && delete n.paths.vs, typeof n.trustedTypesPolicy !== void 0 && delete n.trustedTypesPolicy, n.catchError = !0, globalThis.require.config(n)), new Promise((c, u) => {
      const h = globalThis.require;
      h([r], (f) => {
        if (this._requestHandler = f.create(o), !this._requestHandler) {
          u(new Error("No RequestHandler!"));
          return;
        }
        c(Rt(this._requestHandler));
      }, u);
    }));
  }
}
class be {
  /**
   * Constructs a new DiffChange with the given sequence information
   * and content.
   */
  constructor(t, n, r, s) {
    this.originalStart = t, this.originalLength = n, this.modifiedStart = r, this.modifiedLength = s;
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
function tn(e, t) {
  return (t << 5) - t + e | 0;
}
function ys(e, t) {
  t = tn(149417, t);
  for (let n = 0, r = e.length; n < r; n++)
    t = tn(e.charCodeAt(n), t);
  return t;
}
class nn {
  constructor(t) {
    this.source = t;
  }
  getElements() {
    const t = this.source, n = new Int32Array(t.length);
    for (let r = 0, s = t.length; r < s; r++)
      n[r] = t.charCodeAt(r);
    return n;
  }
}
function Ms(e, t, n) {
  return new _e(new nn(e), new nn(t)).ComputeDiff(n).changes;
}
class Me {
  static Assert(t, n) {
    if (!t)
      throw new Error(n);
  }
}
class Ee {
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
  static Copy(t, n, r, s, i) {
    for (let a = 0; a < i; a++)
      r[s + a] = t[n + a];
  }
  static Copy2(t, n, r, s, i) {
    for (let a = 0; a < i; a++)
      r[s + a] = t[n + a];
  }
}
class rn {
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
    (this.m_originalCount > 0 || this.m_modifiedCount > 0) && this.m_changes.push(new be(this.m_originalStart, this.m_originalCount, this.m_modifiedStart, this.m_modifiedCount)), this.m_originalCount = 0, this.m_modifiedCount = 0, this.m_originalStart = 1073741824, this.m_modifiedStart = 1073741824;
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
class _e {
  /**
   * Constructs the DiffFinder
   */
  constructor(t, n, r = null) {
    this.ContinueProcessingPredicate = r, this._originalSequence = t, this._modifiedSequence = n;
    const [s, i, a] = _e._getElements(t), [o, c, u] = _e._getElements(n);
    this._hasStrings = a && u, this._originalStringElements = s, this._originalElementsOrHash = i, this._modifiedStringElements = o, this._modifiedElementsOrHash = c, this.m_forwardHistory = [], this.m_reverseHistory = [];
  }
  static _isStringArray(t) {
    return t.length > 0 && typeof t[0] == "string";
  }
  static _getElements(t) {
    const n = t.getElements();
    if (_e._isStringArray(n)) {
      const r = new Int32Array(n.length);
      for (let s = 0, i = n.length; s < i; s++)
        r[s] = ys(n[s], 0);
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
    const r = _e._getStrictElement(this._originalSequence, t), s = _e._getStrictElement(this._modifiedSequence, n);
    return r === s;
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
  _ComputeDiff(t, n, r, s, i) {
    const a = [!1];
    let o = this.ComputeDiffRecursive(t, n, r, s, a);
    return i && (o = this.PrettifyChanges(o)), {
      quitEarly: a[0],
      changes: o
    };
  }
  /**
   * Private helper method which computes the differences on the bounded range
   * recursively.
   * @returns An array of the differences between the two input sequences.
   */
  ComputeDiffRecursive(t, n, r, s, i) {
    for (i[0] = !1; t <= n && r <= s && this.ElementsAreEqual(t, r); )
      t++, r++;
    for (; n >= t && s >= r && this.ElementsAreEqual(n, s); )
      n--, s--;
    if (t > n || r > s) {
      let f;
      return r <= s ? (Me.Assert(t === n + 1, "originalStart should only be one more than originalEnd"), f = [
        new be(t, 0, r, s - r + 1)
      ]) : t <= n ? (Me.Assert(r === s + 1, "modifiedStart should only be one more than modifiedEnd"), f = [
        new be(t, n - t + 1, r, 0)
      ]) : (Me.Assert(t === n + 1, "originalStart should only be one more than originalEnd"), Me.Assert(r === s + 1, "modifiedStart should only be one more than modifiedEnd"), f = []), f;
    }
    const a = [0], o = [0], c = this.ComputeRecursionPoint(t, n, r, s, a, o, i), u = a[0], h = o[0];
    if (c !== null)
      return c;
    if (!i[0]) {
      const f = this.ComputeDiffRecursive(t, u, r, h, i);
      let m = [];
      return i[0] ? m = [
        new be(u + 1, n - (u + 1) + 1, h + 1, s - (h + 1) + 1)
      ] : m = this.ComputeDiffRecursive(u + 1, n, h + 1, s, i), this.ConcatenateChanges(f, m);
    }
    return [
      new be(t, n - t + 1, r, s - r + 1)
    ];
  }
  WALKTRACE(t, n, r, s, i, a, o, c, u, h, f, m, d, g, b, w, L, _) {
    let x = null, S = null, N = new rn(), R = n, M = r, E = d[0] - w[0] - s, I = -1073741824, G = this.m_forwardHistory.length - 1;
    do {
      const B = E + t;
      B === R || B < M && u[B - 1] < u[B + 1] ? (f = u[B + 1], g = f - E - s, f < I && N.MarkNextChange(), I = f, N.AddModifiedElement(f + 1, g), E = B + 1 - t) : (f = u[B - 1] + 1, g = f - E - s, f < I && N.MarkNextChange(), I = f - 1, N.AddOriginalElement(f, g + 1), E = B - 1 - t), G >= 0 && (u = this.m_forwardHistory[G], t = u[0], R = 1, M = u.length - 1);
    } while (--G >= -1);
    if (x = N.getReverseChanges(), _[0]) {
      let B = d[0] + 1, p = w[0] + 1;
      if (x !== null && x.length > 0) {
        const v = x[x.length - 1];
        B = Math.max(B, v.getOriginalEnd()), p = Math.max(p, v.getModifiedEnd());
      }
      S = [
        new be(B, m - B + 1, p, b - p + 1)
      ];
    } else {
      N = new rn(), R = a, M = o, E = d[0] - w[0] - c, I = 1073741824, G = L ? this.m_reverseHistory.length - 1 : this.m_reverseHistory.length - 2;
      do {
        const B = E + i;
        B === R || B < M && h[B - 1] >= h[B + 1] ? (f = h[B + 1] - 1, g = f - E - c, f > I && N.MarkNextChange(), I = f + 1, N.AddOriginalElement(f + 1, g + 1), E = B + 1 - i) : (f = h[B - 1], g = f - E - c, f > I && N.MarkNextChange(), I = f, N.AddModifiedElement(f + 1, g + 1), E = B - 1 - i), G >= 0 && (h = this.m_reverseHistory[G], i = h[0], R = 1, M = h.length - 1);
      } while (--G >= -1);
      S = N.getChanges();
    }
    return this.ConcatenateChanges(x, S);
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
  ComputeRecursionPoint(t, n, r, s, i, a, o) {
    let c = 0, u = 0, h = 0, f = 0, m = 0, d = 0;
    t--, r--, i[0] = 0, a[0] = 0, this.m_forwardHistory = [], this.m_reverseHistory = [];
    const g = n - t + (s - r), b = g + 1, w = new Int32Array(b), L = new Int32Array(b), _ = s - r, x = n - t, S = t - r, N = n - s, M = (x - _) % 2 === 0;
    w[_] = t, L[x] = n, o[0] = !1;
    for (let E = 1; E <= g / 2 + 1; E++) {
      let I = 0, G = 0;
      h = this.ClipDiagonalBound(_ - E, E, _, b), f = this.ClipDiagonalBound(_ + E, E, _, b);
      for (let p = h; p <= f; p += 2) {
        p === h || p < f && w[p - 1] < w[p + 1] ? c = w[p + 1] : c = w[p - 1] + 1, u = c - (p - _) - S;
        const v = c;
        for (; c < n && u < s && this.ElementsAreEqual(c + 1, u + 1); )
          c++, u++;
        if (w[p] = c, c + u > I + G && (I = c, G = u), !M && Math.abs(p - x) <= E - 1 && c >= L[p])
          return i[0] = c, a[0] = u, v <= L[p] && 1447 > 0 && E <= 1447 + 1 ? this.WALKTRACE(_, h, f, S, x, m, d, N, w, L, c, n, i, u, s, a, M, o) : null;
      }
      const B = (I - t + (G - r) - E) / 2;
      if (this.ContinueProcessingPredicate !== null && !this.ContinueProcessingPredicate(I, B))
        return o[0] = !0, i[0] = I, a[0] = G, B > 0 && 1447 > 0 && E <= 1447 + 1 ? this.WALKTRACE(_, h, f, S, x, m, d, N, w, L, c, n, i, u, s, a, M, o) : (t++, r++, [
          new be(t, n - t + 1, r, s - r + 1)
        ]);
      m = this.ClipDiagonalBound(x - E, E, x, b), d = this.ClipDiagonalBound(x + E, E, x, b);
      for (let p = m; p <= d; p += 2) {
        p === m || p < d && L[p - 1] >= L[p + 1] ? c = L[p + 1] - 1 : c = L[p - 1], u = c - (p - x) - N;
        const v = c;
        for (; c > t && u > r && this.ElementsAreEqual(c, u); )
          c--, u--;
        if (L[p] = c, M && Math.abs(p - _) <= E && c <= w[p])
          return i[0] = c, a[0] = u, v >= w[p] && 1447 > 0 && E <= 1447 + 1 ? this.WALKTRACE(_, h, f, S, x, m, d, N, w, L, c, n, i, u, s, a, M, o) : null;
      }
      if (E <= 1447) {
        let p = new Int32Array(f - h + 2);
        p[0] = _ - h + 1, Ee.Copy2(w, h, p, 1, f - h + 1), this.m_forwardHistory.push(p), p = new Int32Array(d - m + 2), p[0] = x - m + 1, Ee.Copy2(L, m, p, 1, d - m + 1), this.m_reverseHistory.push(p);
      }
    }
    return this.WALKTRACE(_, h, f, S, x, m, d, N, w, L, c, n, i, u, s, a, M, o);
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
      const r = t[n], s = n < t.length - 1 ? t[n + 1].originalStart : this._originalElementsOrHash.length, i = n < t.length - 1 ? t[n + 1].modifiedStart : this._modifiedElementsOrHash.length, a = r.originalLength > 0, o = r.modifiedLength > 0;
      for (; r.originalStart + r.originalLength < s && r.modifiedStart + r.modifiedLength < i && (!a || this.OriginalElementsAreEqual(r.originalStart, r.originalStart + r.originalLength)) && (!o || this.ModifiedElementsAreEqual(r.modifiedStart, r.modifiedStart + r.modifiedLength)); ) {
        const u = this.ElementsAreStrictEqual(r.originalStart, r.modifiedStart);
        if (this.ElementsAreStrictEqual(r.originalStart + r.originalLength, r.modifiedStart + r.modifiedLength) && !u)
          break;
        r.originalStart++, r.modifiedStart++;
      }
      const c = [null];
      if (n < t.length - 1 && this.ChangesOverlap(t[n], t[n + 1], c)) {
        t[n] = c[0], t.splice(n + 1, 1), n--;
        continue;
      }
    }
    for (let n = t.length - 1; n >= 0; n--) {
      const r = t[n];
      let s = 0, i = 0;
      if (n > 0) {
        const f = t[n - 1];
        s = f.originalStart + f.originalLength, i = f.modifiedStart + f.modifiedLength;
      }
      const a = r.originalLength > 0, o = r.modifiedLength > 0;
      let c = 0, u = this._boundaryScore(r.originalStart, r.originalLength, r.modifiedStart, r.modifiedLength);
      for (let f = 1; ; f++) {
        const m = r.originalStart - f, d = r.modifiedStart - f;
        if (m < s || d < i || a && !this.OriginalElementsAreEqual(m, m + r.originalLength) || o && !this.ModifiedElementsAreEqual(d, d + r.modifiedLength))
          break;
        const b = (m === s && d === i ? 5 : 0) + this._boundaryScore(m, r.originalLength, d, r.modifiedLength);
        b > u && (u = b, c = f);
      }
      r.originalStart -= c, r.modifiedStart -= c;
      const h = [null];
      if (n > 0 && this.ChangesOverlap(t[n - 1], t[n], h)) {
        t[n - 1] = h[0], t.splice(n, 1), n++;
        continue;
      }
    }
    if (this._hasStrings)
      for (let n = 1, r = t.length; n < r; n++) {
        const s = t[n - 1], i = t[n], a = i.originalStart - s.originalStart - s.originalLength, o = s.originalStart, c = i.originalStart + i.originalLength, u = c - o, h = s.modifiedStart, f = i.modifiedStart + i.modifiedLength, m = f - h;
        if (a < 5 && u < 20 && m < 20) {
          const d = this._findBetterContiguousSequence(o, u, h, m, a);
          if (d) {
            const [g, b] = d;
            (g !== s.originalStart + s.originalLength || b !== s.modifiedStart + s.modifiedLength) && (s.originalLength = g - s.originalStart, s.modifiedLength = b - s.modifiedStart, i.originalStart = g + a, i.modifiedStart = b + a, i.originalLength = c - i.originalStart, i.modifiedLength = f - i.modifiedStart);
          }
        }
      }
    return t;
  }
  _findBetterContiguousSequence(t, n, r, s, i) {
    if (n < i || s < i)
      return null;
    const a = t + n - i + 1, o = r + s - i + 1;
    let c = 0, u = 0, h = 0;
    for (let f = t; f < a; f++)
      for (let m = r; m < o; m++) {
        const d = this._contiguousSequenceScore(f, m, i);
        d > 0 && d > c && (c = d, u = f, h = m);
      }
    return c > 0 ? [u, h] : null;
  }
  _contiguousSequenceScore(t, n, r) {
    let s = 0;
    for (let i = 0; i < r; i++) {
      if (!this.ElementsAreEqual(t + i, n + i))
        return 0;
      s += this._originalStringElements[t + i].length;
    }
    return s;
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
  _boundaryScore(t, n, r, s) {
    const i = this._OriginalRegionIsBoundary(t, n) ? 1 : 0, a = this._ModifiedRegionIsBoundary(r, s) ? 1 : 0;
    return i + a;
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
      const s = new Array(t.length + n.length - 1);
      return Ee.Copy(t, 0, s, 0, t.length - 1), s[t.length - 1] = r[0], Ee.Copy(n, 1, s, t.length, n.length - 1), s;
    } else {
      const s = new Array(t.length + n.length);
      return Ee.Copy(t, 0, s, 0, t.length), Ee.Copy(n, 0, s, t.length, n.length), s;
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
    if (Me.Assert(t.originalStart <= n.originalStart, "Left change is not less than or equal to right change"), Me.Assert(t.modifiedStart <= n.modifiedStart, "Left change is not less than or equal to right change"), t.originalStart + t.originalLength >= n.originalStart || t.modifiedStart + t.modifiedLength >= n.modifiedStart) {
      const s = t.originalStart;
      let i = t.originalLength;
      const a = t.modifiedStart;
      let o = t.modifiedLength;
      return t.originalStart + t.originalLength >= n.originalStart && (i = n.originalStart + n.originalLength - t.originalStart), t.modifiedStart + t.modifiedLength >= n.modifiedStart && (o = n.modifiedStart + n.modifiedLength - t.modifiedStart), r[0] = new be(s, i, a, o), !0;
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
  ClipDiagonalBound(t, n, r, s) {
    if (t >= 0 && t < s)
      return t;
    const i = r, a = s - r - 1, o = n % 2 === 0;
    if (t < 0) {
      const c = i % 2 === 0;
      return o === c ? 0 : 1;
    } else {
      const c = a % 2 === 0;
      return o === c ? s - 1 : s - 2;
    }
  }
}
let Te;
if (typeof le.vscode < "u" && typeof le.vscode.process < "u") {
  const e = le.vscode.process;
  Te = {
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
  typeof process < "u" ? Te = {
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
  } : Te = {
    // Supported
    get platform() {
      return Qe ? "win32" : ls ? "darwin" : "linux";
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
const ht = Te.cwd, Es = Te.env, ks = Te.platform, Ps = 65, Fs = 97, Ds = 90, Ts = 122, ve = 46, Q = 47, ee = 92, de = 58, Vs = 63;
class Ar extends Error {
  constructor(t, n, r) {
    let s;
    typeof n == "string" && n.indexOf("not ") === 0 ? (s = "must not be", n = n.replace(/^not /, "")) : s = "must be";
    const i = t.indexOf(".") !== -1 ? "property" : "argument";
    let a = `The "${t}" ${i} ${s} of type ${n}`;
    a += `. Received type ${typeof r}`, super(a), this.code = "ERR_INVALID_ARG_TYPE";
  }
}
function Bs(e, t) {
  if (e === null || typeof e != "object")
    throw new Ar(t, "Object", e);
}
function $(e, t) {
  if (typeof e != "string")
    throw new Ar(t, "string", e);
}
const Ne = ks === "win32";
function P(e) {
  return e === Q || e === ee;
}
function kt(e) {
  return e === Q;
}
function me(e) {
  return e >= Ps && e <= Ds || e >= Fs && e <= Ts;
}
function ft(e, t, n, r) {
  let s = "", i = 0, a = -1, o = 0, c = 0;
  for (let u = 0; u <= e.length; ++u) {
    if (u < e.length)
      c = e.charCodeAt(u);
    else {
      if (r(c))
        break;
      c = Q;
    }
    if (r(c)) {
      if (!(a === u - 1 || o === 1))
        if (o === 2) {
          if (s.length < 2 || i !== 2 || s.charCodeAt(s.length - 1) !== ve || s.charCodeAt(s.length - 2) !== ve) {
            if (s.length > 2) {
              const h = s.lastIndexOf(n);
              h === -1 ? (s = "", i = 0) : (s = s.slice(0, h), i = s.length - 1 - s.lastIndexOf(n)), a = u, o = 0;
              continue;
            } else if (s.length !== 0) {
              s = "", i = 0, a = u, o = 0;
              continue;
            }
          }
          t && (s += s.length > 0 ? `${n}..` : "..", i = 2);
        } else
          s.length > 0 ? s += `${n}${e.slice(a + 1, u)}` : s = e.slice(a + 1, u), i = u - a - 1;
      a = u, o = 0;
    } else
      c === ve && o !== -1 ? ++o : o = -1;
  }
  return s;
}
function Rr(e, t) {
  Bs(t, "pathObject");
  const n = t.dir || t.root, r = t.base || `${t.name || ""}${t.ext || ""}`;
  return n ? n === t.root ? `${n}${r}` : `${n}${e}${r}` : r;
}
const Z = {
  // path.resolve([from ...], to)
  resolve(...e) {
    let t = "", n = "", r = !1;
    for (let s = e.length - 1; s >= -1; s--) {
      let i;
      if (s >= 0) {
        if (i = e[s], $(i, "path"), i.length === 0)
          continue;
      } else
        t.length === 0 ? i = ht() : (i = Es[`=${t}`] || ht(), (i === void 0 || i.slice(0, 2).toLowerCase() !== t.toLowerCase() && i.charCodeAt(2) === ee) && (i = `${t}\\`));
      const a = i.length;
      let o = 0, c = "", u = !1;
      const h = i.charCodeAt(0);
      if (a === 1)
        P(h) && (o = 1, u = !0);
      else if (P(h))
        if (u = !0, P(i.charCodeAt(1))) {
          let f = 2, m = f;
          for (; f < a && !P(i.charCodeAt(f)); )
            f++;
          if (f < a && f !== m) {
            const d = i.slice(m, f);
            for (m = f; f < a && P(i.charCodeAt(f)); )
              f++;
            if (f < a && f !== m) {
              for (m = f; f < a && !P(i.charCodeAt(f)); )
                f++;
              (f === a || f !== m) && (c = `\\\\${d}\\${i.slice(m, f)}`, o = f);
            }
          }
        } else
          o = 1;
      else
        me(h) && i.charCodeAt(1) === de && (c = i.slice(0, 2), o = 2, a > 2 && P(i.charCodeAt(2)) && (u = !0, o = 3));
      if (c.length > 0)
        if (t.length > 0) {
          if (c.toLowerCase() !== t.toLowerCase())
            continue;
        } else
          t = c;
      if (r) {
        if (t.length > 0)
          break;
      } else if (n = `${i.slice(o)}\\${n}`, r = u, u && t.length > 0)
        break;
    }
    return n = ft(n, !r, "\\", P), r ? `${t}\\${n}` : `${t}${n}` || ".";
  },
  normalize(e) {
    $(e, "path");
    const t = e.length;
    if (t === 0)
      return ".";
    let n = 0, r, s = !1;
    const i = e.charCodeAt(0);
    if (t === 1)
      return kt(i) ? "\\" : e;
    if (P(i))
      if (s = !0, P(e.charCodeAt(1))) {
        let o = 2, c = o;
        for (; o < t && !P(e.charCodeAt(o)); )
          o++;
        if (o < t && o !== c) {
          const u = e.slice(c, o);
          for (c = o; o < t && P(e.charCodeAt(o)); )
            o++;
          if (o < t && o !== c) {
            for (c = o; o < t && !P(e.charCodeAt(o)); )
              o++;
            if (o === t)
              return `\\\\${u}\\${e.slice(c)}\\`;
            o !== c && (r = `\\\\${u}\\${e.slice(c, o)}`, n = o);
          }
        }
      } else
        n = 1;
    else
      me(i) && e.charCodeAt(1) === de && (r = e.slice(0, 2), n = 2, t > 2 && P(e.charCodeAt(2)) && (s = !0, n = 3));
    let a = n < t ? ft(e.slice(n), !s, "\\", P) : "";
    return a.length === 0 && !s && (a = "."), a.length > 0 && P(e.charCodeAt(t - 1)) && (a += "\\"), r === void 0 ? s ? `\\${a}` : a : s ? `${r}\\${a}` : `${r}${a}`;
  },
  isAbsolute(e) {
    $(e, "path");
    const t = e.length;
    if (t === 0)
      return !1;
    const n = e.charCodeAt(0);
    return P(n) || // Possible device root
    t > 2 && me(n) && e.charCodeAt(1) === de && P(e.charCodeAt(2));
  },
  join(...e) {
    if (e.length === 0)
      return ".";
    let t, n;
    for (let i = 0; i < e.length; ++i) {
      const a = e[i];
      $(a, "path"), a.length > 0 && (t === void 0 ? t = n = a : t += `\\${a}`);
    }
    if (t === void 0)
      return ".";
    let r = !0, s = 0;
    if (typeof n == "string" && P(n.charCodeAt(0))) {
      ++s;
      const i = n.length;
      i > 1 && P(n.charCodeAt(1)) && (++s, i > 2 && (P(n.charCodeAt(2)) ? ++s : r = !1));
    }
    if (r) {
      for (; s < t.length && P(t.charCodeAt(s)); )
        s++;
      s >= 2 && (t = `\\${t.slice(s)}`);
    }
    return Z.normalize(t);
  },
  // It will solve the relative path from `from` to `to`, for instance:
  //  from = 'C:\\orandea\\test\\aaa'
  //  to = 'C:\\orandea\\impl\\bbb'
  // The output of the function should be: '..\\..\\impl\\bbb'
  relative(e, t) {
    if ($(e, "from"), $(t, "to"), e === t)
      return "";
    const n = Z.resolve(e), r = Z.resolve(t);
    if (n === r || (e = n.toLowerCase(), t = r.toLowerCase(), e === t))
      return "";
    let s = 0;
    for (; s < e.length && e.charCodeAt(s) === ee; )
      s++;
    let i = e.length;
    for (; i - 1 > s && e.charCodeAt(i - 1) === ee; )
      i--;
    const a = i - s;
    let o = 0;
    for (; o < t.length && t.charCodeAt(o) === ee; )
      o++;
    let c = t.length;
    for (; c - 1 > o && t.charCodeAt(c - 1) === ee; )
      c--;
    const u = c - o, h = a < u ? a : u;
    let f = -1, m = 0;
    for (; m < h; m++) {
      const g = e.charCodeAt(s + m);
      if (g !== t.charCodeAt(o + m))
        break;
      g === ee && (f = m);
    }
    if (m !== h) {
      if (f === -1)
        return r;
    } else {
      if (u > h) {
        if (t.charCodeAt(o + m) === ee)
          return r.slice(o + m + 1);
        if (m === 2)
          return r.slice(o + m);
      }
      a > h && (e.charCodeAt(s + m) === ee ? f = m : m === 2 && (f = 3)), f === -1 && (f = 0);
    }
    let d = "";
    for (m = s + f + 1; m <= i; ++m)
      (m === i || e.charCodeAt(m) === ee) && (d += d.length === 0 ? ".." : "\\..");
    return o += f, d.length > 0 ? `${d}${r.slice(o, c)}` : (r.charCodeAt(o) === ee && ++o, r.slice(o, c));
  },
  toNamespacedPath(e) {
    if (typeof e != "string" || e.length === 0)
      return e;
    const t = Z.resolve(e);
    if (t.length <= 2)
      return e;
    if (t.charCodeAt(0) === ee) {
      if (t.charCodeAt(1) === ee) {
        const n = t.charCodeAt(2);
        if (n !== Vs && n !== ve)
          return `\\\\?\\UNC\\${t.slice(2)}`;
      }
    } else if (me(t.charCodeAt(0)) && t.charCodeAt(1) === de && t.charCodeAt(2) === ee)
      return `\\\\?\\${t}`;
    return e;
  },
  dirname(e) {
    $(e, "path");
    const t = e.length;
    if (t === 0)
      return ".";
    let n = -1, r = 0;
    const s = e.charCodeAt(0);
    if (t === 1)
      return P(s) ? e : ".";
    if (P(s)) {
      if (n = r = 1, P(e.charCodeAt(1))) {
        let o = 2, c = o;
        for (; o < t && !P(e.charCodeAt(o)); )
          o++;
        if (o < t && o !== c) {
          for (c = o; o < t && P(e.charCodeAt(o)); )
            o++;
          if (o < t && o !== c) {
            for (c = o; o < t && !P(e.charCodeAt(o)); )
              o++;
            if (o === t)
              return e;
            o !== c && (n = r = o + 1);
          }
        }
      }
    } else
      me(s) && e.charCodeAt(1) === de && (n = t > 2 && P(e.charCodeAt(2)) ? 3 : 2, r = n);
    let i = -1, a = !0;
    for (let o = t - 1; o >= r; --o)
      if (P(e.charCodeAt(o))) {
        if (!a) {
          i = o;
          break;
        }
      } else
        a = !1;
    if (i === -1) {
      if (n === -1)
        return ".";
      i = n;
    }
    return e.slice(0, i);
  },
  basename(e, t) {
    t !== void 0 && $(t, "ext"), $(e, "path");
    let n = 0, r = -1, s = !0, i;
    if (e.length >= 2 && me(e.charCodeAt(0)) && e.charCodeAt(1) === de && (n = 2), t !== void 0 && t.length > 0 && t.length <= e.length) {
      if (t === e)
        return "";
      let a = t.length - 1, o = -1;
      for (i = e.length - 1; i >= n; --i) {
        const c = e.charCodeAt(i);
        if (P(c)) {
          if (!s) {
            n = i + 1;
            break;
          }
        } else
          o === -1 && (s = !1, o = i + 1), a >= 0 && (c === t.charCodeAt(a) ? --a === -1 && (r = i) : (a = -1, r = o));
      }
      return n === r ? r = o : r === -1 && (r = e.length), e.slice(n, r);
    }
    for (i = e.length - 1; i >= n; --i)
      if (P(e.charCodeAt(i))) {
        if (!s) {
          n = i + 1;
          break;
        }
      } else
        r === -1 && (s = !1, r = i + 1);
    return r === -1 ? "" : e.slice(n, r);
  },
  extname(e) {
    $(e, "path");
    let t = 0, n = -1, r = 0, s = -1, i = !0, a = 0;
    e.length >= 2 && e.charCodeAt(1) === de && me(e.charCodeAt(0)) && (t = r = 2);
    for (let o = e.length - 1; o >= t; --o) {
      const c = e.charCodeAt(o);
      if (P(c)) {
        if (!i) {
          r = o + 1;
          break;
        }
        continue;
      }
      s === -1 && (i = !1, s = o + 1), c === ve ? n === -1 ? n = o : a !== 1 && (a = 1) : n !== -1 && (a = -1);
    }
    return n === -1 || s === -1 || // We saw a non-dot character immediately before the dot
    a === 0 || // The (right-most) trimmed path component is exactly '..'
    a === 1 && n === s - 1 && n === r + 1 ? "" : e.slice(n, s);
  },
  format: Rr.bind(null, "\\"),
  parse(e) {
    $(e, "path");
    const t = { root: "", dir: "", base: "", ext: "", name: "" };
    if (e.length === 0)
      return t;
    const n = e.length;
    let r = 0, s = e.charCodeAt(0);
    if (n === 1)
      return P(s) ? (t.root = t.dir = e, t) : (t.base = t.name = e, t);
    if (P(s)) {
      if (r = 1, P(e.charCodeAt(1))) {
        let f = 2, m = f;
        for (; f < n && !P(e.charCodeAt(f)); )
          f++;
        if (f < n && f !== m) {
          for (m = f; f < n && P(e.charCodeAt(f)); )
            f++;
          if (f < n && f !== m) {
            for (m = f; f < n && !P(e.charCodeAt(f)); )
              f++;
            f === n ? r = f : f !== m && (r = f + 1);
          }
        }
      }
    } else if (me(s) && e.charCodeAt(1) === de) {
      if (n <= 2)
        return t.root = t.dir = e, t;
      if (r = 2, P(e.charCodeAt(2))) {
        if (n === 3)
          return t.root = t.dir = e, t;
        r = 3;
      }
    }
    r > 0 && (t.root = e.slice(0, r));
    let i = -1, a = r, o = -1, c = !0, u = e.length - 1, h = 0;
    for (; u >= r; --u) {
      if (s = e.charCodeAt(u), P(s)) {
        if (!c) {
          a = u + 1;
          break;
        }
        continue;
      }
      o === -1 && (c = !1, o = u + 1), s === ve ? i === -1 ? i = u : h !== 1 && (h = 1) : i !== -1 && (h = -1);
    }
    return o !== -1 && (i === -1 || // We saw a non-dot character immediately before the dot
    h === 0 || // The (right-most) trimmed path component is exactly '..'
    h === 1 && i === o - 1 && i === a + 1 ? t.base = t.name = e.slice(a, o) : (t.name = e.slice(a, i), t.base = e.slice(a, o), t.ext = e.slice(i, o))), a > 0 && a !== r ? t.dir = e.slice(0, a - 1) : t.dir = t.root, t;
  },
  sep: "\\",
  delimiter: ";",
  win32: null,
  posix: null
}, qs = (() => {
  if (Ne) {
    const e = /\\/g;
    return () => {
      const t = ht().replace(e, "/");
      return t.slice(t.indexOf("/"));
    };
  }
  return () => ht();
})(), te = {
  // path.resolve([from ...], to)
  resolve(...e) {
    let t = "", n = !1;
    for (let r = e.length - 1; r >= -1 && !n; r--) {
      const s = r >= 0 ? e[r] : qs();
      $(s, "path"), s.length !== 0 && (t = `${s}/${t}`, n = s.charCodeAt(0) === Q);
    }
    return t = ft(t, !n, "/", kt), n ? `/${t}` : t.length > 0 ? t : ".";
  },
  normalize(e) {
    if ($(e, "path"), e.length === 0)
      return ".";
    const t = e.charCodeAt(0) === Q, n = e.charCodeAt(e.length - 1) === Q;
    return e = ft(e, !t, "/", kt), e.length === 0 ? t ? "/" : n ? "./" : "." : (n && (e += "/"), t ? `/${e}` : e);
  },
  isAbsolute(e) {
    return $(e, "path"), e.length > 0 && e.charCodeAt(0) === Q;
  },
  join(...e) {
    if (e.length === 0)
      return ".";
    let t;
    for (let n = 0; n < e.length; ++n) {
      const r = e[n];
      $(r, "path"), r.length > 0 && (t === void 0 ? t = r : t += `/${r}`);
    }
    return t === void 0 ? "." : te.normalize(t);
  },
  relative(e, t) {
    if ($(e, "from"), $(t, "to"), e === t || (e = te.resolve(e), t = te.resolve(t), e === t))
      return "";
    const n = 1, r = e.length, s = r - n, i = 1, a = t.length - i, o = s < a ? s : a;
    let c = -1, u = 0;
    for (; u < o; u++) {
      const f = e.charCodeAt(n + u);
      if (f !== t.charCodeAt(i + u))
        break;
      f === Q && (c = u);
    }
    if (u === o)
      if (a > o) {
        if (t.charCodeAt(i + u) === Q)
          return t.slice(i + u + 1);
        if (u === 0)
          return t.slice(i + u);
      } else
        s > o && (e.charCodeAt(n + u) === Q ? c = u : u === 0 && (c = 0));
    let h = "";
    for (u = n + c + 1; u <= r; ++u)
      (u === r || e.charCodeAt(u) === Q) && (h += h.length === 0 ? ".." : "/..");
    return `${h}${t.slice(i + c)}`;
  },
  toNamespacedPath(e) {
    return e;
  },
  dirname(e) {
    if ($(e, "path"), e.length === 0)
      return ".";
    const t = e.charCodeAt(0) === Q;
    let n = -1, r = !0;
    for (let s = e.length - 1; s >= 1; --s)
      if (e.charCodeAt(s) === Q) {
        if (!r) {
          n = s;
          break;
        }
      } else
        r = !1;
    return n === -1 ? t ? "/" : "." : t && n === 1 ? "//" : e.slice(0, n);
  },
  basename(e, t) {
    t !== void 0 && $(t, "ext"), $(e, "path");
    let n = 0, r = -1, s = !0, i;
    if (t !== void 0 && t.length > 0 && t.length <= e.length) {
      if (t === e)
        return "";
      let a = t.length - 1, o = -1;
      for (i = e.length - 1; i >= 0; --i) {
        const c = e.charCodeAt(i);
        if (c === Q) {
          if (!s) {
            n = i + 1;
            break;
          }
        } else
          o === -1 && (s = !1, o = i + 1), a >= 0 && (c === t.charCodeAt(a) ? --a === -1 && (r = i) : (a = -1, r = o));
      }
      return n === r ? r = o : r === -1 && (r = e.length), e.slice(n, r);
    }
    for (i = e.length - 1; i >= 0; --i)
      if (e.charCodeAt(i) === Q) {
        if (!s) {
          n = i + 1;
          break;
        }
      } else
        r === -1 && (s = !1, r = i + 1);
    return r === -1 ? "" : e.slice(n, r);
  },
  extname(e) {
    $(e, "path");
    let t = -1, n = 0, r = -1, s = !0, i = 0;
    for (let a = e.length - 1; a >= 0; --a) {
      const o = e.charCodeAt(a);
      if (o === Q) {
        if (!s) {
          n = a + 1;
          break;
        }
        continue;
      }
      r === -1 && (s = !1, r = a + 1), o === ve ? t === -1 ? t = a : i !== 1 && (i = 1) : t !== -1 && (i = -1);
    }
    return t === -1 || r === -1 || // We saw a non-dot character immediately before the dot
    i === 0 || // The (right-most) trimmed path component is exactly '..'
    i === 1 && t === r - 1 && t === n + 1 ? "" : e.slice(t, r);
  },
  format: Rr.bind(null, "/"),
  parse(e) {
    $(e, "path");
    const t = { root: "", dir: "", base: "", ext: "", name: "" };
    if (e.length === 0)
      return t;
    const n = e.charCodeAt(0) === Q;
    let r;
    n ? (t.root = "/", r = 1) : r = 0;
    let s = -1, i = 0, a = -1, o = !0, c = e.length - 1, u = 0;
    for (; c >= r; --c) {
      const h = e.charCodeAt(c);
      if (h === Q) {
        if (!o) {
          i = c + 1;
          break;
        }
        continue;
      }
      a === -1 && (o = !1, a = c + 1), h === ve ? s === -1 ? s = c : u !== 1 && (u = 1) : s !== -1 && (u = -1);
    }
    if (a !== -1) {
      const h = i === 0 && n ? 1 : i;
      s === -1 || // We saw a non-dot character immediately before the dot
      u === 0 || // The (right-most) trimmed path component is exactly '..'
      u === 1 && s === a - 1 && s === i + 1 ? t.base = t.name = e.slice(h, a) : (t.name = e.slice(h, s), t.base = e.slice(h, a), t.ext = e.slice(s, a));
    }
    return i > 0 ? t.dir = e.slice(0, i - 1) : n && (t.dir = "/"), t;
  },
  sep: "/",
  delimiter: ":",
  win32: null,
  posix: null
};
te.win32 = Z.win32 = Z;
te.posix = Z.posix = te;
Ne ? Z.normalize : te.normalize;
Ne ? Z.resolve : te.resolve;
Ne ? Z.relative : te.relative;
Ne ? Z.dirname : te.dirname;
Ne ? Z.basename : te.basename;
Ne ? Z.extname : te.extname;
Ne ? Z.sep : te.sep;
const Is = /^\w[\w\d+.-]*$/, Us = /^\//, Hs = /^\/\//;
function Ws(e, t) {
  if (!e.scheme && t)
    throw new Error(`[UriError]: Scheme is missing: {scheme: "", authority: "${e.authority}", path: "${e.path}", query: "${e.query}", fragment: "${e.fragment}"}`);
  if (e.scheme && !Is.test(e.scheme))
    throw new Error("[UriError]: Scheme contains illegal characters.");
  if (e.path) {
    if (e.authority) {
      if (!Us.test(e.path))
        throw new Error('[UriError]: If a URI contains an authority component, then the path component must either be empty or begin with a slash ("/") character');
    } else if (Hs.test(e.path))
      throw new Error('[UriError]: If a URI does not contain an authority component, then the path cannot begin with two slash characters ("//")');
  }
}
function zs(e, t) {
  return !e && !t ? "file" : e;
}
function $s(e, t) {
  switch (e) {
    case "https":
    case "http":
    case "file":
      t ? t[0] !== ae && (t = ae + t) : t = ae;
      break;
  }
  return t;
}
const W = "", ae = "/", Os = /^(([^:/?#]+?):)?(\/\/([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?/;
class Ce {
  static isUri(t) {
    return t instanceof Ce ? !0 : t ? typeof t.authority == "string" && typeof t.fragment == "string" && typeof t.path == "string" && typeof t.query == "string" && typeof t.scheme == "string" && typeof t.fsPath == "string" && typeof t.with == "function" && typeof t.toString == "function" : !1;
  }
  /**
   * @internal
   */
  constructor(t, n, r, s, i, a = !1) {
    typeof t == "object" ? (this.scheme = t.scheme || W, this.authority = t.authority || W, this.path = t.path || W, this.query = t.query || W, this.fragment = t.fragment || W) : (this.scheme = zs(t, a), this.authority = n || W, this.path = $s(this.scheme, r || W), this.query = s || W, this.fragment = i || W, Ws(this, a));
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
    return Pt(this, !1);
  }
  // ---- modify to new -------------------------
  with(t) {
    if (!t)
      return this;
    let { scheme: n, authority: r, path: s, query: i, fragment: a } = t;
    return n === void 0 ? n = this.scheme : n === null && (n = W), r === void 0 ? r = this.authority : r === null && (r = W), s === void 0 ? s = this.path : s === null && (s = W), i === void 0 ? i = this.query : i === null && (i = W), a === void 0 ? a = this.fragment : a === null && (a = W), n === this.scheme && r === this.authority && s === this.path && i === this.query && a === this.fragment ? this : new ke(n, r, s, i, a);
  }
  // ---- parse & validate ------------------------
  /**
   * Creates a new URI from a string, e.g. `http://www.example.com/some/path`,
   * `file:///usr/home`, or `scheme:with/path`.
   *
   * @param value A string which represents an URI (see `URI#toString`).
   */
  static parse(t, n = !1) {
    const r = Os.exec(t);
    return r ? new ke(r[2] || W, st(r[4] || W), st(r[5] || W), st(r[7] || W), st(r[9] || W), n) : new ke(W, W, W, W, W);
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
    let n = W;
    if (Qe && (t = t.replace(/\\/g, ae)), t[0] === ae && t[1] === ae) {
      const r = t.indexOf(ae, 2);
      r === -1 ? (n = t.substring(2), t = ae) : (n = t.substring(2, r), t = t.substring(r) || ae);
    }
    return new ke("file", n, t, W, W);
  }
  /**
   * Creates new URI from uri components.
   *
   * Unless `strict` is `true` the scheme is defaults to be `file`. This function performs
   * validation and should be used for untrusted uri components retrieved from storage,
   * user input, command arguments etc
   */
  static from(t, n) {
    return new ke(t.scheme, t.authority, t.path, t.query, t.fragment, n);
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
    return Qe && t.scheme === "file" ? r = Ce.file(Z.join(Pt(t, !0), ...n)).path : r = te.join(t.path, ...n), t.with({ path: r });
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
    return Ft(this, t);
  }
  toJSON() {
    return this;
  }
  static revive(t) {
    var n, r;
    if (t) {
      if (t instanceof Ce)
        return t;
      {
        const s = new ke(t);
        return s._formatted = (n = t.external) !== null && n !== void 0 ? n : null, s._fsPath = t._sep === yr && (r = t.fsPath) !== null && r !== void 0 ? r : null, s;
      }
    } else
      return t;
  }
}
const yr = Qe ? 1 : void 0;
class ke extends Ce {
  constructor() {
    super(...arguments), this._formatted = null, this._fsPath = null;
  }
  get fsPath() {
    return this._fsPath || (this._fsPath = Pt(this, !1)), this._fsPath;
  }
  toString(t = !1) {
    return t ? Ft(this, !0) : (this._formatted || (this._formatted = Ft(this, !1)), this._formatted);
  }
  toJSON() {
    const t = {
      $mid: 1
      /* MarshalledId.Uri */
    };
    return this._fsPath && (t.fsPath = this._fsPath, t._sep = yr), this._formatted && (t.external = this._formatted), this.path && (t.path = this.path), this.scheme && (t.scheme = this.scheme), this.authority && (t.authority = this.authority), this.query && (t.query = this.query), this.fragment && (t.fragment = this.fragment), t;
  }
}
const Mr = {
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
function sn(e, t, n) {
  let r, s = -1;
  for (let i = 0; i < e.length; i++) {
    const a = e.charCodeAt(i);
    if (a >= 97 && a <= 122 || a >= 65 && a <= 90 || a >= 48 && a <= 57 || a === 45 || a === 46 || a === 95 || a === 126 || t && a === 47 || n && a === 91 || n && a === 93 || n && a === 58)
      s !== -1 && (r += encodeURIComponent(e.substring(s, i)), s = -1), r !== void 0 && (r += e.charAt(i));
    else {
      r === void 0 && (r = e.substr(0, i));
      const o = Mr[a];
      o !== void 0 ? (s !== -1 && (r += encodeURIComponent(e.substring(s, i)), s = -1), r += o) : s === -1 && (s = i);
    }
  }
  return s !== -1 && (r += encodeURIComponent(e.substring(s))), r !== void 0 ? r : e;
}
function Gs(e) {
  let t;
  for (let n = 0; n < e.length; n++) {
    const r = e.charCodeAt(n);
    r === 35 || r === 63 ? (t === void 0 && (t = e.substr(0, n)), t += Mr[r]) : t !== void 0 && (t += e[n]);
  }
  return t !== void 0 ? t : e;
}
function Pt(e, t) {
  let n;
  return e.authority && e.path.length > 1 && e.scheme === "file" ? n = `//${e.authority}${e.path}` : e.path.charCodeAt(0) === 47 && (e.path.charCodeAt(1) >= 65 && e.path.charCodeAt(1) <= 90 || e.path.charCodeAt(1) >= 97 && e.path.charCodeAt(1) <= 122) && e.path.charCodeAt(2) === 58 ? t ? n = e.path.substr(1) : n = e.path[1].toLowerCase() + e.path.substr(2) : n = e.path, Qe && (n = n.replace(/\//g, "\\")), n;
}
function Ft(e, t) {
  const n = t ? Gs : sn;
  let r = "", { scheme: s, authority: i, path: a, query: o, fragment: c } = e;
  if (s && (r += s, r += ":"), (i || s === "file") && (r += ae, r += ae), i) {
    let u = i.indexOf("@");
    if (u !== -1) {
      const h = i.substr(0, u);
      i = i.substr(u + 1), u = h.lastIndexOf(":"), u === -1 ? r += n(h, !1, !1) : (r += n(h.substr(0, u), !1, !1), r += ":", r += n(h.substr(u + 1), !1, !0)), r += "@";
    }
    i = i.toLowerCase(), u = i.lastIndexOf(":"), u === -1 ? r += n(i, !1, !0) : (r += n(i.substr(0, u), !1, !0), r += i.substr(u));
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
  return o && (r += "?", r += n(o, !1, !1)), c && (r += "#", r += t ? c : sn(c, !1, !1)), r;
}
function Er(e) {
  try {
    return decodeURIComponent(e);
  } catch {
    return e.length > 3 ? e.substr(0, 3) + Er(e.substr(3)) : e;
  }
}
const an = /(%[0-9A-Za-z][0-9A-Za-z])+/g;
function st(e) {
  return e.match(an) ? e.replace(an, (t) => Er(t)) : e;
}
class J {
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
    return t === this.lineNumber && n === this.column ? this : new J(t, n);
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
    return J.equals(this, t);
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
    return J.isBefore(this, t);
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
    return J.isBeforeOrEqual(this, t);
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
    const r = t.lineNumber | 0, s = n.lineNumber | 0;
    if (r === s) {
      const i = t.column | 0, a = n.column | 0;
      return i - a;
    }
    return r - s;
  }
  /**
   * Clone this position.
   */
  clone() {
    return new J(this.lineNumber, this.column);
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
    return new J(t.lineNumber, t.column);
  }
  /**
   * Test if `obj` is an `IPosition`.
   */
  static isIPosition(t) {
    return t && typeof t.lineNumber == "number" && typeof t.column == "number";
  }
}
class D {
  constructor(t, n, r, s) {
    t > r || t === r && n > s ? (this.startLineNumber = r, this.startColumn = s, this.endLineNumber = t, this.endColumn = n) : (this.startLineNumber = t, this.startColumn = n, this.endLineNumber = r, this.endColumn = s);
  }
  /**
   * Test if this range is empty.
   */
  isEmpty() {
    return D.isEmpty(this);
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
    return D.containsPosition(this, t);
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
    return D.containsRange(this, t);
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
    return D.strictContainsRange(this, t);
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
    return D.plusRange(this, t);
  }
  /**
   * A reunion of the two ranges.
   * The smallest position will be used as the start point, and the largest one as the end point.
   */
  static plusRange(t, n) {
    let r, s, i, a;
    return n.startLineNumber < t.startLineNumber ? (r = n.startLineNumber, s = n.startColumn) : n.startLineNumber === t.startLineNumber ? (r = n.startLineNumber, s = Math.min(n.startColumn, t.startColumn)) : (r = t.startLineNumber, s = t.startColumn), n.endLineNumber > t.endLineNumber ? (i = n.endLineNumber, a = n.endColumn) : n.endLineNumber === t.endLineNumber ? (i = n.endLineNumber, a = Math.max(n.endColumn, t.endColumn)) : (i = t.endLineNumber, a = t.endColumn), new D(r, s, i, a);
  }
  /**
   * A intersection of the two ranges.
   */
  intersectRanges(t) {
    return D.intersectRanges(this, t);
  }
  /**
   * A intersection of the two ranges.
   */
  static intersectRanges(t, n) {
    let r = t.startLineNumber, s = t.startColumn, i = t.endLineNumber, a = t.endColumn;
    const o = n.startLineNumber, c = n.startColumn, u = n.endLineNumber, h = n.endColumn;
    return r < o ? (r = o, s = c) : r === o && (s = Math.max(s, c)), i > u ? (i = u, a = h) : i === u && (a = Math.min(a, h)), r > i || r === i && s > a ? null : new D(r, s, i, a);
  }
  /**
   * Test if this range equals other.
   */
  equalsRange(t) {
    return D.equalsRange(this, t);
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
    return D.getEndPosition(this);
  }
  /**
   * Return the end position (which will be after or equal to the start position)
   */
  static getEndPosition(t) {
    return new J(t.endLineNumber, t.endColumn);
  }
  /**
   * Return the start position (which will be before or equal to the end position)
   */
  getStartPosition() {
    return D.getStartPosition(this);
  }
  /**
   * Return the start position (which will be before or equal to the end position)
   */
  static getStartPosition(t) {
    return new J(t.startLineNumber, t.startColumn);
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
    return new D(this.startLineNumber, this.startColumn, t, n);
  }
  /**
   * Create a new range using this range's end position, and using startLineNumber and startColumn as the start position.
   */
  setStartPosition(t, n) {
    return new D(t, n, this.endLineNumber, this.endColumn);
  }
  /**
   * Create a new empty range using this range's start position.
   */
  collapseToStart() {
    return D.collapseToStart(this);
  }
  /**
   * Create a new empty range using this range's start position.
   */
  static collapseToStart(t) {
    return new D(t.startLineNumber, t.startColumn, t.startLineNumber, t.startColumn);
  }
  /**
   * Create a new empty range using this range's end position.
   */
  collapseToEnd() {
    return D.collapseToEnd(this);
  }
  /**
   * Create a new empty range using this range's end position.
   */
  static collapseToEnd(t) {
    return new D(t.endLineNumber, t.endColumn, t.endLineNumber, t.endColumn);
  }
  /**
   * Moves the range by the given amount of lines.
   */
  delta(t) {
    return new D(this.startLineNumber + t, this.startColumn, this.endLineNumber + t, this.endColumn);
  }
  // ---
  static fromPositions(t, n = t) {
    return new D(t.lineNumber, t.column, n.lineNumber, n.column);
  }
  static lift(t) {
    return t ? new D(t.startLineNumber, t.startColumn, t.endLineNumber, t.endColumn) : null;
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
      const i = t.startLineNumber | 0, a = n.startLineNumber | 0;
      if (i === a) {
        const o = t.startColumn | 0, c = n.startColumn | 0;
        if (o === c) {
          const u = t.endLineNumber | 0, h = n.endLineNumber | 0;
          if (u === h) {
            const f = t.endColumn | 0, m = n.endColumn | 0;
            return f - m;
          }
          return u - h;
        }
        return o - c;
      }
      return i - a;
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
}
function js(e, t, n = (r, s) => r === s) {
  if (e === t)
    return !0;
  if (!e || !t || e.length !== t.length)
    return !1;
  for (let r = 0, s = e.length; r < s; r++)
    if (!n(e[r], t[r]))
      return !1;
  return !0;
}
function* Qs(e, t) {
  let n, r;
  for (const s of e)
    r !== void 0 && t(r, s) ? n.push(s) : (n && (yield n), n = [s]), r = s;
  n && (yield n);
}
function Xs(e, t) {
  for (let n = 0; n <= e.length; n++)
    t(n === 0 ? void 0 : e[n - 1], n === e.length ? void 0 : e[n]);
}
function Js(e, t) {
  for (let n = 0; n < e.length; n++)
    t(n === 0 ? void 0 : e[n - 1], e[n], n + 1 === e.length ? void 0 : e[n + 1]);
}
function Ys(e, t) {
  for (const n of t)
    e.push(n);
}
var ln;
(function(e) {
  function t(i) {
    return i < 0;
  }
  e.isLessThan = t;
  function n(i) {
    return i <= 0;
  }
  e.isLessThanOrEqual = n;
  function r(i) {
    return i > 0;
  }
  e.isGreaterThan = r;
  function s(i) {
    return i === 0;
  }
  e.isNeitherLessOrGreaterThan = s, e.greaterThan = 1, e.lessThan = -1, e.neitherLessOrGreaterThan = 0;
})(ln || (ln = {}));
function it(e, t) {
  return (n, r) => t(e(n), e(r));
}
const at = (e, t) => e - t;
function Zs(e) {
  return (t, n) => -e(t, n);
}
function on(e) {
  return e < 0 ? 0 : e > 255 ? 255 : e | 0;
}
function Pe(e) {
  return e < 0 ? 0 : e > 4294967295 ? 4294967295 : e | 0;
}
class Ks {
  constructor(t) {
    this.values = t, this.prefixSum = new Uint32Array(t.length), this.prefixSumValidIndex = new Int32Array(1), this.prefixSumValidIndex[0] = -1;
  }
  insertValues(t, n) {
    t = Pe(t);
    const r = this.values, s = this.prefixSum, i = n.length;
    return i === 0 ? !1 : (this.values = new Uint32Array(r.length + i), this.values.set(r.subarray(0, t), 0), this.values.set(r.subarray(t), t + i), this.values.set(n, t), t - 1 < this.prefixSumValidIndex[0] && (this.prefixSumValidIndex[0] = t - 1), this.prefixSum = new Uint32Array(this.values.length), this.prefixSumValidIndex[0] >= 0 && this.prefixSum.set(s.subarray(0, this.prefixSumValidIndex[0] + 1)), !0);
  }
  setValue(t, n) {
    return t = Pe(t), n = Pe(n), this.values[t] === n ? !1 : (this.values[t] = n, t - 1 < this.prefixSumValidIndex[0] && (this.prefixSumValidIndex[0] = t - 1), !0);
  }
  removeValues(t, n) {
    t = Pe(t), n = Pe(n);
    const r = this.values, s = this.prefixSum;
    if (t >= r.length)
      return !1;
    const i = r.length - t;
    return n >= i && (n = i), n === 0 ? !1 : (this.values = new Uint32Array(r.length - n), this.values.set(r.subarray(0, t), 0), this.values.set(r.subarray(t + n), t), this.prefixSum = new Uint32Array(this.values.length), t - 1 < this.prefixSumValidIndex[0] && (this.prefixSumValidIndex[0] = t - 1), this.prefixSumValidIndex[0] >= 0 && this.prefixSum.set(s.subarray(0, this.prefixSumValidIndex[0] + 1)), !0);
  }
  getTotalSum() {
    return this.values.length === 0 ? 0 : this._getPrefixSum(this.values.length - 1);
  }
  /**
   * Returns the sum of the first `index + 1` many items.
   * @returns `SUM(0 <= j <= index, values[j])`.
   */
  getPrefixSum(t) {
    return t < 0 ? 0 : (t = Pe(t), this._getPrefixSum(t));
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
    let n = 0, r = this.values.length - 1, s = 0, i = 0, a = 0;
    for (; n <= r; )
      if (s = n + (r - n) / 2 | 0, i = this.prefixSum[s], a = i - this.values[s], t < a)
        r = s - 1;
      else if (t >= i)
        n = s + 1;
      else
        break;
    return new ei(s, t - a);
  }
}
class ei {
  constructor(t, n) {
    this.index = t, this.remainder = n, this._prefixSumIndexOfResultBrand = void 0, this.index = t, this.remainder = n;
  }
}
class ti {
  constructor(t, n, r, s) {
    this._uri = t, this._lines = n, this._eol = r, this._versionId = s, this._lineStarts = null, this._cachedTextValue = null;
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
      this._acceptDeleteRange(r.range), this._acceptInsertText(new J(r.range.startLineNumber, r.range.startColumn), r.text);
    this._versionId = t.versionId, this._cachedTextValue = null;
  }
  _ensureLineStarts() {
    if (!this._lineStarts) {
      const t = this._eol.length, n = this._lines.length, r = new Uint32Array(n);
      for (let s = 0; s < n; s++)
        r[s] = this._lines[s].length + t;
      this._lineStarts = new Ks(r);
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
    const r = fs(n);
    if (r.length === 1) {
      this._setLineText(t.lineNumber - 1, this._lines[t.lineNumber - 1].substring(0, t.column - 1) + r[0] + this._lines[t.lineNumber - 1].substring(t.column - 1));
      return;
    }
    r[r.length - 1] += this._lines[t.lineNumber - 1].substring(t.column - 1), this._setLineText(t.lineNumber - 1, this._lines[t.lineNumber - 1].substring(0, t.column - 1) + r[0]);
    const s = new Uint32Array(r.length - 1);
    for (let i = 1; i < r.length; i++)
      this._lines.splice(t.lineNumber + i - 1, 0, r[i]), s[i - 1] = r[i].length + this._eol.length;
    this._lineStarts && this._lineStarts.insertValues(t.lineNumber, s);
  }
}
const ni = "`~!@#$%^&*()-=+[{]}\\|;:'\",.<>/?";
function ri(e = "") {
  let t = "(-?\\d*\\.\\d\\w*)|([^";
  for (const n of ni)
    e.indexOf(n) >= 0 || (t += "\\" + n);
  return t += "\\s]+)", new RegExp(t, "g");
}
const kr = ri();
function si(e) {
  let t = kr;
  if (e && e instanceof RegExp)
    if (e.global)
      t = e;
    else {
      let n = "g";
      e.ignoreCase && (n += "i"), e.multiline && (n += "m"), e.unicode && (n += "u"), t = new RegExp(e.source, n);
    }
  return t.lastIndex = 0, t;
}
const Pr = new jr();
Pr.unshift({
  maxLen: 1e3,
  windowSize: 15,
  timeBudget: 150
});
function Qt(e, t, n, r, s) {
  if (s || (s = ct.first(Pr)), n.length > s.maxLen) {
    let u = e - s.maxLen / 2;
    return u < 0 ? u = 0 : r += u, n = n.substring(u, e + s.maxLen / 2), Qt(e, t, n, r, s);
  }
  const i = Date.now(), a = e - 1 - r;
  let o = -1, c = null;
  for (let u = 1; !(Date.now() - i >= s.timeBudget); u++) {
    const h = a - s.windowSize * u;
    t.lastIndex = Math.max(0, h);
    const f = ii(t, n, a, o);
    if (!f && c || (c = f, h <= 0))
      break;
    o = h;
  }
  if (c) {
    const u = {
      word: c[0],
      startColumn: r + 1 + c.index,
      endColumn: r + 1 + c.index + c[0].length
    };
    return t.lastIndex = 0, u;
  }
  return null;
}
function ii(e, t, n, r) {
  let s;
  for (; s = e.exec(t); ) {
    const i = s.index || 0;
    if (i <= n && e.lastIndex >= n)
      return s;
    if (r > 0 && i > r)
      return null;
  }
  return null;
}
class Xt {
  constructor(t) {
    const n = on(t);
    this._defaultValue = n, this._asciiMap = Xt._createAsciiMap(n), this._map = /* @__PURE__ */ new Map();
  }
  static _createAsciiMap(t) {
    const n = new Uint8Array(256);
    return n.fill(t), n;
  }
  set(t, n) {
    const r = on(n);
    t >= 0 && t < 256 ? this._asciiMap[t] = r : this._map.set(t, r);
  }
  get(t) {
    return t >= 0 && t < 256 ? this._asciiMap[t] : this._map.get(t) || this._defaultValue;
  }
  clear() {
    this._asciiMap.fill(this._defaultValue), this._map.clear();
  }
}
class ai {
  constructor(t, n, r) {
    const s = new Uint8Array(t * n);
    for (let i = 0, a = t * n; i < a; i++)
      s[i] = r;
    this._data = s, this.rows = t, this.cols = n;
  }
  get(t, n) {
    return this._data[t * this.cols + n];
  }
  set(t, n, r) {
    this._data[t * this.cols + n] = r;
  }
}
class li {
  constructor(t) {
    let n = 0, r = 0;
    for (let i = 0, a = t.length; i < a; i++) {
      const [o, c, u] = t[i];
      c > n && (n = c), o > r && (r = o), u > r && (r = u);
    }
    n++, r++;
    const s = new ai(
      r,
      n,
      0
      /* State.Invalid */
    );
    for (let i = 0, a = t.length; i < a; i++) {
      const [o, c, u] = t[i];
      s.set(o, c, u);
    }
    this._states = s, this._maxCharCode = n;
  }
  nextState(t, n) {
    return n < 0 || n >= this._maxCharCode ? 0 : this._states.get(t, n);
  }
}
let Lt = null;
function oi() {
  return Lt === null && (Lt = new li([
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
  ])), Lt;
}
let We = null;
function ui() {
  if (We === null) {
    We = new Xt(
      0
      /* CharacterClass.None */
    );
    const e = ` 	<>'"、。｡､，．：；‘〈「『〔（［｛｢｣｝］）〕』」〉’｀～…`;
    for (let n = 0; n < e.length; n++)
      We.set(
        e.charCodeAt(n),
        1
        /* CharacterClass.ForceTermination */
      );
    const t = ".,;:";
    for (let n = 0; n < t.length; n++)
      We.set(
        t.charCodeAt(n),
        2
        /* CharacterClass.CannotEndIn */
      );
  }
  return We;
}
class dt {
  static _createLink(t, n, r, s, i) {
    let a = i - 1;
    do {
      const o = n.charCodeAt(a);
      if (t.get(o) !== 2)
        break;
      a--;
    } while (a > s);
    if (s > 0) {
      const o = n.charCodeAt(s - 1), c = n.charCodeAt(a);
      (o === 40 && c === 41 || o === 91 && c === 93 || o === 123 && c === 125) && a--;
    }
    return {
      range: {
        startLineNumber: r,
        startColumn: s + 1,
        endLineNumber: r,
        endColumn: a + 2
      },
      url: n.substring(s, a + 1)
    };
  }
  static computeLinks(t, n = oi()) {
    const r = ui(), s = [];
    for (let i = 1, a = t.getLineCount(); i <= a; i++) {
      const o = t.getLineContent(i), c = o.length;
      let u = 0, h = 0, f = 0, m = 1, d = !1, g = !1, b = !1, w = !1;
      for (; u < c; ) {
        let L = !1;
        const _ = o.charCodeAt(u);
        if (m === 13) {
          let x;
          switch (_) {
            case 40:
              d = !0, x = 0;
              break;
            case 41:
              x = d ? 0 : 1;
              break;
            case 91:
              b = !0, g = !0, x = 0;
              break;
            case 93:
              b = !1, x = g ? 0 : 1;
              break;
            case 123:
              w = !0, x = 0;
              break;
            case 125:
              x = w ? 0 : 1;
              break;
            case 39:
            case 34:
            case 96:
              f === _ ? x = 1 : f === 39 || f === 34 || f === 96 ? x = 0 : x = 1;
              break;
            case 42:
              x = f === 42 ? 1 : 0;
              break;
            case 124:
              x = f === 124 ? 1 : 0;
              break;
            case 32:
              x = b ? 0 : 1;
              break;
            default:
              x = r.get(_);
          }
          x === 1 && (s.push(dt._createLink(r, o, i, h, u)), L = !0);
        } else if (m === 12) {
          let x;
          _ === 91 ? (g = !0, x = 0) : x = r.get(_), x === 1 ? L = !0 : m = 13;
        } else
          m = n.nextState(m, _), m === 0 && (L = !0);
        L && (m = 1, d = !1, g = !1, w = !1, h = u + 1, f = _), u++;
      }
      m === 13 && s.push(dt._createLink(r, o, i, h, c));
    }
    return s;
  }
}
function ci(e) {
  return !e || typeof e.getLineCount != "function" || typeof e.getLineContent != "function" ? [] : dt.computeLinks(e);
}
class Dt {
  constructor() {
    this._defaultValueSet = [
      ["true", "false"],
      ["True", "False"],
      ["Private", "Public", "Friend", "ReadOnly", "Partial", "Protected", "WriteOnly"],
      ["public", "protected", "private"]
    ];
  }
  navigateValueSet(t, n, r, s, i) {
    if (t && n) {
      const a = this.doNavigateValueSet(n, i);
      if (a)
        return {
          range: t,
          value: a
        };
    }
    if (r && s) {
      const a = this.doNavigateValueSet(s, i);
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
    let s = Number(t);
    const i = parseFloat(t);
    return !isNaN(s) && !isNaN(i) && s === i ? s === 0 && !n ? null : (s = Math.floor(s * r), s += n ? r : -r, String(s / r)) : null;
  }
  textReplace(t, n) {
    return this.valueSetsReplace(this._defaultValueSet, t, n);
  }
  valueSetsReplace(t, n, r) {
    let s = null;
    for (let i = 0, a = t.length; s === null && i < a; i++)
      s = this.valueSetReplace(t[i], n, r);
    return s;
  }
  valueSetReplace(t, n, r) {
    let s = t.indexOf(n);
    return s >= 0 ? (s += r ? 1 : -1, s < 0 ? s = t.length - 1 : s %= t.length, t[s]) : null;
  }
}
Dt.INSTANCE = new Dt();
const Fr = Object.freeze(function(e, t) {
  const n = setTimeout(e.bind(t), 0);
  return { dispose() {
    clearTimeout(n);
  } };
});
var mt;
(function(e) {
  function t(n) {
    return n === e.None || n === e.Cancelled || n instanceof lt ? !0 : !n || typeof n != "object" ? !1 : typeof n.isCancellationRequested == "boolean" && typeof n.onCancellationRequested == "function";
  }
  e.isCancellationToken = t, e.None = Object.freeze({
    isCancellationRequested: !1,
    onCancellationRequested: At.None
  }), e.Cancelled = Object.freeze({
    isCancellationRequested: !0,
    onCancellationRequested: Fr
  });
})(mt || (mt = {}));
class lt {
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
    return this._isCancelled ? Fr : (this._emitter || (this._emitter = new ie()), this._emitter.event);
  }
  dispose() {
    this._emitter && (this._emitter.dispose(), this._emitter = null);
  }
}
class hi {
  constructor(t) {
    this._token = void 0, this._parentListener = void 0, this._parentListener = t && t.onCancellationRequested(this.cancel, this);
  }
  get token() {
    return this._token || (this._token = new lt()), this._token;
  }
  cancel() {
    this._token ? this._token instanceof lt && this._token.cancel() : this._token = mt.Cancelled;
  }
  dispose(t = !1) {
    var n;
    t && this.cancel(), (n = this._parentListener) === null || n === void 0 || n.dispose(), this._token ? this._token instanceof lt && this._token.dispose() : this._token = mt.None;
  }
}
class Jt {
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
const ot = new Jt(), Tt = new Jt(), Vt = new Jt(), fi = new Array(230), di = /* @__PURE__ */ Object.create(null), mi = /* @__PURE__ */ Object.create(null);
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
  for (const s of t) {
    const [i, a, o, c, u, h, f, m, d] = s;
    if (r[a] || (r[a] = !0, di[o] = a, mi[o.toLowerCase()] = a), !n[c]) {
      if (n[c] = !0, !u)
        throw new Error(`String representation missing for key code ${c} around scan code ${o}`);
      ot.define(c, u), Tt.define(c, m || u), Vt.define(c, d || m || u);
    }
    h && (fi[h] = c);
  }
})();
var un;
(function(e) {
  function t(o) {
    return ot.keyCodeToStr(o);
  }
  e.toString = t;
  function n(o) {
    return ot.strToKeyCode(o);
  }
  e.fromString = n;
  function r(o) {
    return Tt.keyCodeToStr(o);
  }
  e.toUserSettingsUS = r;
  function s(o) {
    return Vt.keyCodeToStr(o);
  }
  e.toUserSettingsGeneral = s;
  function i(o) {
    return Tt.strToKeyCode(o) || Vt.strToKeyCode(o);
  }
  e.fromUserSettings = i;
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
    return ot.keyCodeToStr(o);
  }
  e.toElectronAccelerator = a;
})(un || (un = {}));
function gi(e, t) {
  const n = (t & 65535) << 16 >>> 0;
  return (e | n) >>> 0;
}
class ne extends D {
  constructor(t, n, r, s) {
    super(t, n, r, s), this.selectionStartLineNumber = t, this.selectionStartColumn = n, this.positionLineNumber = r, this.positionColumn = s;
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
    return ne.selectionsEqual(this, t);
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
    return this.getDirection() === 0 ? new ne(this.startLineNumber, this.startColumn, t, n) : new ne(t, n, this.startLineNumber, this.startColumn);
  }
  /**
   * Get the position at `positionLineNumber` and `positionColumn`.
   */
  getPosition() {
    return new J(this.positionLineNumber, this.positionColumn);
  }
  /**
   * Get the position at the start of the selection.
  */
  getSelectionStart() {
    return new J(this.selectionStartLineNumber, this.selectionStartColumn);
  }
  /**
   * Create a new selection with a different `selectionStartLineNumber` and `selectionStartColumn`.
   */
  setStartPosition(t, n) {
    return this.getDirection() === 0 ? new ne(t, n, this.endLineNumber, this.endColumn) : new ne(this.endLineNumber, this.endColumn, t, n);
  }
  // ----
  /**
   * Create a `Selection` from one or two positions
   */
  static fromPositions(t, n = t) {
    return new ne(t.lineNumber, t.column, n.lineNumber, n.column);
  }
  /**
   * Creates a `Selection` from a range, given a direction.
   */
  static fromRange(t, n) {
    return n === 0 ? new ne(t.startLineNumber, t.startColumn, t.endLineNumber, t.endColumn) : new ne(t.endLineNumber, t.endColumn, t.startLineNumber, t.startColumn);
  }
  /**
   * Create a `Selection` from an `ISelection`.
   */
  static liftSelection(t) {
    return new ne(t.selectionStartLineNumber, t.selectionStartColumn, t.positionLineNumber, t.positionColumn);
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
    for (let r = 0, s = t.length; r < s; r++)
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
  static createWithDirection(t, n, r, s, i) {
    return i === 0 ? new ne(t, n, r, s) : new ne(r, s, t, n);
  }
}
const cn = /* @__PURE__ */ Object.create(null);
function l(e, t) {
  if (Kr(t)) {
    const n = cn[t];
    if (n === void 0)
      throw new Error(`${e} references an unknown codicon: ${t}`);
    t = n;
  }
  return cn[e] = t, { id: e };
}
const y = {
  // built-in icons, with image name
  add: l("add", 6e4),
  plus: l("plus", 6e4),
  gistNew: l("gist-new", 6e4),
  repoCreate: l("repo-create", 6e4),
  lightbulb: l("lightbulb", 60001),
  lightBulb: l("light-bulb", 60001),
  repo: l("repo", 60002),
  repoDelete: l("repo-delete", 60002),
  gistFork: l("gist-fork", 60003),
  repoForked: l("repo-forked", 60003),
  gitPullRequest: l("git-pull-request", 60004),
  gitPullRequestAbandoned: l("git-pull-request-abandoned", 60004),
  recordKeys: l("record-keys", 60005),
  keyboard: l("keyboard", 60005),
  tag: l("tag", 60006),
  tagAdd: l("tag-add", 60006),
  tagRemove: l("tag-remove", 60006),
  gitPullRequestLabel: l("git-pull-request-label", 60006),
  person: l("person", 60007),
  personFollow: l("person-follow", 60007),
  personOutline: l("person-outline", 60007),
  personFilled: l("person-filled", 60007),
  gitBranch: l("git-branch", 60008),
  gitBranchCreate: l("git-branch-create", 60008),
  gitBranchDelete: l("git-branch-delete", 60008),
  sourceControl: l("source-control", 60008),
  mirror: l("mirror", 60009),
  mirrorPublic: l("mirror-public", 60009),
  star: l("star", 60010),
  starAdd: l("star-add", 60010),
  starDelete: l("star-delete", 60010),
  starEmpty: l("star-empty", 60010),
  comment: l("comment", 60011),
  commentAdd: l("comment-add", 60011),
  alert: l("alert", 60012),
  warning: l("warning", 60012),
  search: l("search", 60013),
  searchSave: l("search-save", 60013),
  logOut: l("log-out", 60014),
  signOut: l("sign-out", 60014),
  logIn: l("log-in", 60015),
  signIn: l("sign-in", 60015),
  eye: l("eye", 60016),
  eyeUnwatch: l("eye-unwatch", 60016),
  eyeWatch: l("eye-watch", 60016),
  circleFilled: l("circle-filled", 60017),
  primitiveDot: l("primitive-dot", 60017),
  closeDirty: l("close-dirty", 60017),
  debugBreakpoint: l("debug-breakpoint", 60017),
  debugBreakpointDisabled: l("debug-breakpoint-disabled", 60017),
  debugHint: l("debug-hint", 60017),
  primitiveSquare: l("primitive-square", 60018),
  edit: l("edit", 60019),
  pencil: l("pencil", 60019),
  info: l("info", 60020),
  issueOpened: l("issue-opened", 60020),
  gistPrivate: l("gist-private", 60021),
  gitForkPrivate: l("git-fork-private", 60021),
  lock: l("lock", 60021),
  mirrorPrivate: l("mirror-private", 60021),
  close: l("close", 60022),
  removeClose: l("remove-close", 60022),
  x: l("x", 60022),
  repoSync: l("repo-sync", 60023),
  sync: l("sync", 60023),
  clone: l("clone", 60024),
  desktopDownload: l("desktop-download", 60024),
  beaker: l("beaker", 60025),
  microscope: l("microscope", 60025),
  vm: l("vm", 60026),
  deviceDesktop: l("device-desktop", 60026),
  file: l("file", 60027),
  fileText: l("file-text", 60027),
  more: l("more", 60028),
  ellipsis: l("ellipsis", 60028),
  kebabHorizontal: l("kebab-horizontal", 60028),
  mailReply: l("mail-reply", 60029),
  reply: l("reply", 60029),
  organization: l("organization", 60030),
  organizationFilled: l("organization-filled", 60030),
  organizationOutline: l("organization-outline", 60030),
  newFile: l("new-file", 60031),
  fileAdd: l("file-add", 60031),
  newFolder: l("new-folder", 60032),
  fileDirectoryCreate: l("file-directory-create", 60032),
  trash: l("trash", 60033),
  trashcan: l("trashcan", 60033),
  history: l("history", 60034),
  clock: l("clock", 60034),
  folder: l("folder", 60035),
  fileDirectory: l("file-directory", 60035),
  symbolFolder: l("symbol-folder", 60035),
  logoGithub: l("logo-github", 60036),
  markGithub: l("mark-github", 60036),
  github: l("github", 60036),
  terminal: l("terminal", 60037),
  console: l("console", 60037),
  repl: l("repl", 60037),
  zap: l("zap", 60038),
  symbolEvent: l("symbol-event", 60038),
  error: l("error", 60039),
  stop: l("stop", 60039),
  variable: l("variable", 60040),
  symbolVariable: l("symbol-variable", 60040),
  array: l("array", 60042),
  symbolArray: l("symbol-array", 60042),
  symbolModule: l("symbol-module", 60043),
  symbolPackage: l("symbol-package", 60043),
  symbolNamespace: l("symbol-namespace", 60043),
  symbolObject: l("symbol-object", 60043),
  symbolMethod: l("symbol-method", 60044),
  symbolFunction: l("symbol-function", 60044),
  symbolConstructor: l("symbol-constructor", 60044),
  symbolBoolean: l("symbol-boolean", 60047),
  symbolNull: l("symbol-null", 60047),
  symbolNumeric: l("symbol-numeric", 60048),
  symbolNumber: l("symbol-number", 60048),
  symbolStructure: l("symbol-structure", 60049),
  symbolStruct: l("symbol-struct", 60049),
  symbolParameter: l("symbol-parameter", 60050),
  symbolTypeParameter: l("symbol-type-parameter", 60050),
  symbolKey: l("symbol-key", 60051),
  symbolText: l("symbol-text", 60051),
  symbolReference: l("symbol-reference", 60052),
  goToFile: l("go-to-file", 60052),
  symbolEnum: l("symbol-enum", 60053),
  symbolValue: l("symbol-value", 60053),
  symbolRuler: l("symbol-ruler", 60054),
  symbolUnit: l("symbol-unit", 60054),
  activateBreakpoints: l("activate-breakpoints", 60055),
  archive: l("archive", 60056),
  arrowBoth: l("arrow-both", 60057),
  arrowDown: l("arrow-down", 60058),
  arrowLeft: l("arrow-left", 60059),
  arrowRight: l("arrow-right", 60060),
  arrowSmallDown: l("arrow-small-down", 60061),
  arrowSmallLeft: l("arrow-small-left", 60062),
  arrowSmallRight: l("arrow-small-right", 60063),
  arrowSmallUp: l("arrow-small-up", 60064),
  arrowUp: l("arrow-up", 60065),
  bell: l("bell", 60066),
  bold: l("bold", 60067),
  book: l("book", 60068),
  bookmark: l("bookmark", 60069),
  debugBreakpointConditionalUnverified: l("debug-breakpoint-conditional-unverified", 60070),
  debugBreakpointConditional: l("debug-breakpoint-conditional", 60071),
  debugBreakpointConditionalDisabled: l("debug-breakpoint-conditional-disabled", 60071),
  debugBreakpointDataUnverified: l("debug-breakpoint-data-unverified", 60072),
  debugBreakpointData: l("debug-breakpoint-data", 60073),
  debugBreakpointDataDisabled: l("debug-breakpoint-data-disabled", 60073),
  debugBreakpointLogUnverified: l("debug-breakpoint-log-unverified", 60074),
  debugBreakpointLog: l("debug-breakpoint-log", 60075),
  debugBreakpointLogDisabled: l("debug-breakpoint-log-disabled", 60075),
  briefcase: l("briefcase", 60076),
  broadcast: l("broadcast", 60077),
  browser: l("browser", 60078),
  bug: l("bug", 60079),
  calendar: l("calendar", 60080),
  caseSensitive: l("case-sensitive", 60081),
  check: l("check", 60082),
  checklist: l("checklist", 60083),
  chevronDown: l("chevron-down", 60084),
  dropDownButton: l("drop-down-button", 60084),
  chevronLeft: l("chevron-left", 60085),
  chevronRight: l("chevron-right", 60086),
  chevronUp: l("chevron-up", 60087),
  chromeClose: l("chrome-close", 60088),
  chromeMaximize: l("chrome-maximize", 60089),
  chromeMinimize: l("chrome-minimize", 60090),
  chromeRestore: l("chrome-restore", 60091),
  circle: l("circle", 60092),
  circleOutline: l("circle-outline", 60092),
  debugBreakpointUnverified: l("debug-breakpoint-unverified", 60092),
  circleSlash: l("circle-slash", 60093),
  circuitBoard: l("circuit-board", 60094),
  clearAll: l("clear-all", 60095),
  clippy: l("clippy", 60096),
  closeAll: l("close-all", 60097),
  cloudDownload: l("cloud-download", 60098),
  cloudUpload: l("cloud-upload", 60099),
  code: l("code", 60100),
  collapseAll: l("collapse-all", 60101),
  colorMode: l("color-mode", 60102),
  commentDiscussion: l("comment-discussion", 60103),
  compareChanges: l("compare-changes", 60157),
  creditCard: l("credit-card", 60105),
  dash: l("dash", 60108),
  dashboard: l("dashboard", 60109),
  database: l("database", 60110),
  debugContinue: l("debug-continue", 60111),
  debugDisconnect: l("debug-disconnect", 60112),
  debugPause: l("debug-pause", 60113),
  debugRestart: l("debug-restart", 60114),
  debugStart: l("debug-start", 60115),
  debugStepInto: l("debug-step-into", 60116),
  debugStepOut: l("debug-step-out", 60117),
  debugStepOver: l("debug-step-over", 60118),
  debugStop: l("debug-stop", 60119),
  debug: l("debug", 60120),
  deviceCameraVideo: l("device-camera-video", 60121),
  deviceCamera: l("device-camera", 60122),
  deviceMobile: l("device-mobile", 60123),
  diffAdded: l("diff-added", 60124),
  diffIgnored: l("diff-ignored", 60125),
  diffModified: l("diff-modified", 60126),
  diffRemoved: l("diff-removed", 60127),
  diffRenamed: l("diff-renamed", 60128),
  diff: l("diff", 60129),
  discard: l("discard", 60130),
  editorLayout: l("editor-layout", 60131),
  emptyWindow: l("empty-window", 60132),
  exclude: l("exclude", 60133),
  extensions: l("extensions", 60134),
  eyeClosed: l("eye-closed", 60135),
  fileBinary: l("file-binary", 60136),
  fileCode: l("file-code", 60137),
  fileMedia: l("file-media", 60138),
  filePdf: l("file-pdf", 60139),
  fileSubmodule: l("file-submodule", 60140),
  fileSymlinkDirectory: l("file-symlink-directory", 60141),
  fileSymlinkFile: l("file-symlink-file", 60142),
  fileZip: l("file-zip", 60143),
  files: l("files", 60144),
  filter: l("filter", 60145),
  flame: l("flame", 60146),
  foldDown: l("fold-down", 60147),
  foldUp: l("fold-up", 60148),
  fold: l("fold", 60149),
  folderActive: l("folder-active", 60150),
  folderOpened: l("folder-opened", 60151),
  gear: l("gear", 60152),
  gift: l("gift", 60153),
  gistSecret: l("gist-secret", 60154),
  gist: l("gist", 60155),
  gitCommit: l("git-commit", 60156),
  gitCompare: l("git-compare", 60157),
  gitMerge: l("git-merge", 60158),
  githubAction: l("github-action", 60159),
  githubAlt: l("github-alt", 60160),
  globe: l("globe", 60161),
  grabber: l("grabber", 60162),
  graph: l("graph", 60163),
  gripper: l("gripper", 60164),
  heart: l("heart", 60165),
  home: l("home", 60166),
  horizontalRule: l("horizontal-rule", 60167),
  hubot: l("hubot", 60168),
  inbox: l("inbox", 60169),
  issueClosed: l("issue-closed", 60324),
  issueReopened: l("issue-reopened", 60171),
  issues: l("issues", 60172),
  italic: l("italic", 60173),
  jersey: l("jersey", 60174),
  json: l("json", 60175),
  bracket: l("bracket", 60175),
  kebabVertical: l("kebab-vertical", 60176),
  key: l("key", 60177),
  law: l("law", 60178),
  lightbulbAutofix: l("lightbulb-autofix", 60179),
  linkExternal: l("link-external", 60180),
  link: l("link", 60181),
  listOrdered: l("list-ordered", 60182),
  listUnordered: l("list-unordered", 60183),
  liveShare: l("live-share", 60184),
  loading: l("loading", 60185),
  location: l("location", 60186),
  mailRead: l("mail-read", 60187),
  mail: l("mail", 60188),
  markdown: l("markdown", 60189),
  megaphone: l("megaphone", 60190),
  mention: l("mention", 60191),
  milestone: l("milestone", 60192),
  gitPullRequestMilestone: l("git-pull-request-milestone", 60192),
  mortarBoard: l("mortar-board", 60193),
  move: l("move", 60194),
  multipleWindows: l("multiple-windows", 60195),
  mute: l("mute", 60196),
  noNewline: l("no-newline", 60197),
  note: l("note", 60198),
  octoface: l("octoface", 60199),
  openPreview: l("open-preview", 60200),
  package: l("package", 60201),
  paintcan: l("paintcan", 60202),
  pin: l("pin", 60203),
  play: l("play", 60204),
  run: l("run", 60204),
  plug: l("plug", 60205),
  preserveCase: l("preserve-case", 60206),
  preview: l("preview", 60207),
  project: l("project", 60208),
  pulse: l("pulse", 60209),
  question: l("question", 60210),
  quote: l("quote", 60211),
  radioTower: l("radio-tower", 60212),
  reactions: l("reactions", 60213),
  references: l("references", 60214),
  refresh: l("refresh", 60215),
  regex: l("regex", 60216),
  remoteExplorer: l("remote-explorer", 60217),
  remote: l("remote", 60218),
  remove: l("remove", 60219),
  replaceAll: l("replace-all", 60220),
  replace: l("replace", 60221),
  repoClone: l("repo-clone", 60222),
  repoForcePush: l("repo-force-push", 60223),
  repoPull: l("repo-pull", 60224),
  repoPush: l("repo-push", 60225),
  report: l("report", 60226),
  requestChanges: l("request-changes", 60227),
  rocket: l("rocket", 60228),
  rootFolderOpened: l("root-folder-opened", 60229),
  rootFolder: l("root-folder", 60230),
  rss: l("rss", 60231),
  ruby: l("ruby", 60232),
  saveAll: l("save-all", 60233),
  saveAs: l("save-as", 60234),
  save: l("save", 60235),
  screenFull: l("screen-full", 60236),
  screenNormal: l("screen-normal", 60237),
  searchStop: l("search-stop", 60238),
  server: l("server", 60240),
  settingsGear: l("settings-gear", 60241),
  settings: l("settings", 60242),
  shield: l("shield", 60243),
  smiley: l("smiley", 60244),
  sortPrecedence: l("sort-precedence", 60245),
  splitHorizontal: l("split-horizontal", 60246),
  splitVertical: l("split-vertical", 60247),
  squirrel: l("squirrel", 60248),
  starFull: l("star-full", 60249),
  starHalf: l("star-half", 60250),
  symbolClass: l("symbol-class", 60251),
  symbolColor: l("symbol-color", 60252),
  symbolCustomColor: l("symbol-customcolor", 60252),
  symbolConstant: l("symbol-constant", 60253),
  symbolEnumMember: l("symbol-enum-member", 60254),
  symbolField: l("symbol-field", 60255),
  symbolFile: l("symbol-file", 60256),
  symbolInterface: l("symbol-interface", 60257),
  symbolKeyword: l("symbol-keyword", 60258),
  symbolMisc: l("symbol-misc", 60259),
  symbolOperator: l("symbol-operator", 60260),
  symbolProperty: l("symbol-property", 60261),
  wrench: l("wrench", 60261),
  wrenchSubaction: l("wrench-subaction", 60261),
  symbolSnippet: l("symbol-snippet", 60262),
  tasklist: l("tasklist", 60263),
  telescope: l("telescope", 60264),
  textSize: l("text-size", 60265),
  threeBars: l("three-bars", 60266),
  thumbsdown: l("thumbsdown", 60267),
  thumbsup: l("thumbsup", 60268),
  tools: l("tools", 60269),
  triangleDown: l("triangle-down", 60270),
  triangleLeft: l("triangle-left", 60271),
  triangleRight: l("triangle-right", 60272),
  triangleUp: l("triangle-up", 60273),
  twitter: l("twitter", 60274),
  unfold: l("unfold", 60275),
  unlock: l("unlock", 60276),
  unmute: l("unmute", 60277),
  unverified: l("unverified", 60278),
  verified: l("verified", 60279),
  versions: l("versions", 60280),
  vmActive: l("vm-active", 60281),
  vmOutline: l("vm-outline", 60282),
  vmRunning: l("vm-running", 60283),
  watch: l("watch", 60284),
  whitespace: l("whitespace", 60285),
  wholeWord: l("whole-word", 60286),
  window: l("window", 60287),
  wordWrap: l("word-wrap", 60288),
  zoomIn: l("zoom-in", 60289),
  zoomOut: l("zoom-out", 60290),
  listFilter: l("list-filter", 60291),
  listFlat: l("list-flat", 60292),
  listSelection: l("list-selection", 60293),
  selection: l("selection", 60293),
  listTree: l("list-tree", 60294),
  debugBreakpointFunctionUnverified: l("debug-breakpoint-function-unverified", 60295),
  debugBreakpointFunction: l("debug-breakpoint-function", 60296),
  debugBreakpointFunctionDisabled: l("debug-breakpoint-function-disabled", 60296),
  debugStackframeActive: l("debug-stackframe-active", 60297),
  circleSmallFilled: l("circle-small-filled", 60298),
  debugStackframeDot: l("debug-stackframe-dot", 60298),
  debugStackframe: l("debug-stackframe", 60299),
  debugStackframeFocused: l("debug-stackframe-focused", 60299),
  debugBreakpointUnsupported: l("debug-breakpoint-unsupported", 60300),
  symbolString: l("symbol-string", 60301),
  debugReverseContinue: l("debug-reverse-continue", 60302),
  debugStepBack: l("debug-step-back", 60303),
  debugRestartFrame: l("debug-restart-frame", 60304),
  callIncoming: l("call-incoming", 60306),
  callOutgoing: l("call-outgoing", 60307),
  menu: l("menu", 60308),
  expandAll: l("expand-all", 60309),
  feedback: l("feedback", 60310),
  gitPullRequestReviewer: l("git-pull-request-reviewer", 60310),
  groupByRefType: l("group-by-ref-type", 60311),
  ungroupByRefType: l("ungroup-by-ref-type", 60312),
  account: l("account", 60313),
  gitPullRequestAssignee: l("git-pull-request-assignee", 60313),
  bellDot: l("bell-dot", 60314),
  debugConsole: l("debug-console", 60315),
  library: l("library", 60316),
  output: l("output", 60317),
  runAll: l("run-all", 60318),
  syncIgnored: l("sync-ignored", 60319),
  pinned: l("pinned", 60320),
  githubInverted: l("github-inverted", 60321),
  debugAlt: l("debug-alt", 60305),
  serverProcess: l("server-process", 60322),
  serverEnvironment: l("server-environment", 60323),
  pass: l("pass", 60324),
  stopCircle: l("stop-circle", 60325),
  playCircle: l("play-circle", 60326),
  record: l("record", 60327),
  debugAltSmall: l("debug-alt-small", 60328),
  vmConnect: l("vm-connect", 60329),
  cloud: l("cloud", 60330),
  merge: l("merge", 60331),
  exportIcon: l("export", 60332),
  graphLeft: l("graph-left", 60333),
  magnet: l("magnet", 60334),
  notebook: l("notebook", 60335),
  redo: l("redo", 60336),
  checkAll: l("check-all", 60337),
  pinnedDirty: l("pinned-dirty", 60338),
  passFilled: l("pass-filled", 60339),
  circleLargeFilled: l("circle-large-filled", 60340),
  circleLarge: l("circle-large", 60341),
  circleLargeOutline: l("circle-large-outline", 60341),
  combine: l("combine", 60342),
  gather: l("gather", 60342),
  table: l("table", 60343),
  variableGroup: l("variable-group", 60344),
  typeHierarchy: l("type-hierarchy", 60345),
  typeHierarchySub: l("type-hierarchy-sub", 60346),
  typeHierarchySuper: l("type-hierarchy-super", 60347),
  gitPullRequestCreate: l("git-pull-request-create", 60348),
  runAbove: l("run-above", 60349),
  runBelow: l("run-below", 60350),
  notebookTemplate: l("notebook-template", 60351),
  debugRerun: l("debug-rerun", 60352),
  workspaceTrusted: l("workspace-trusted", 60353),
  workspaceUntrusted: l("workspace-untrusted", 60354),
  workspaceUnspecified: l("workspace-unspecified", 60355),
  terminalCmd: l("terminal-cmd", 60356),
  terminalDebian: l("terminal-debian", 60357),
  terminalLinux: l("terminal-linux", 60358),
  terminalPowershell: l("terminal-powershell", 60359),
  terminalTmux: l("terminal-tmux", 60360),
  terminalUbuntu: l("terminal-ubuntu", 60361),
  terminalBash: l("terminal-bash", 60362),
  arrowSwap: l("arrow-swap", 60363),
  copy: l("copy", 60364),
  personAdd: l("person-add", 60365),
  filterFilled: l("filter-filled", 60366),
  wand: l("wand", 60367),
  debugLineByLine: l("debug-line-by-line", 60368),
  inspect: l("inspect", 60369),
  layers: l("layers", 60370),
  layersDot: l("layers-dot", 60371),
  layersActive: l("layers-active", 60372),
  compass: l("compass", 60373),
  compassDot: l("compass-dot", 60374),
  compassActive: l("compass-active", 60375),
  azure: l("azure", 60376),
  issueDraft: l("issue-draft", 60377),
  gitPullRequestClosed: l("git-pull-request-closed", 60378),
  gitPullRequestDraft: l("git-pull-request-draft", 60379),
  debugAll: l("debug-all", 60380),
  debugCoverage: l("debug-coverage", 60381),
  runErrors: l("run-errors", 60382),
  folderLibrary: l("folder-library", 60383),
  debugContinueSmall: l("debug-continue-small", 60384),
  beakerStop: l("beaker-stop", 60385),
  graphLine: l("graph-line", 60386),
  graphScatter: l("graph-scatter", 60387),
  pieChart: l("pie-chart", 60388),
  bracketDot: l("bracket-dot", 60389),
  bracketError: l("bracket-error", 60390),
  lockSmall: l("lock-small", 60391),
  azureDevops: l("azure-devops", 60392),
  verifiedFilled: l("verified-filled", 60393),
  newLine: l("newline", 60394),
  layout: l("layout", 60395),
  layoutActivitybarLeft: l("layout-activitybar-left", 60396),
  layoutActivitybarRight: l("layout-activitybar-right", 60397),
  layoutPanelLeft: l("layout-panel-left", 60398),
  layoutPanelCenter: l("layout-panel-center", 60399),
  layoutPanelJustify: l("layout-panel-justify", 60400),
  layoutPanelRight: l("layout-panel-right", 60401),
  layoutPanel: l("layout-panel", 60402),
  layoutSidebarLeft: l("layout-sidebar-left", 60403),
  layoutSidebarRight: l("layout-sidebar-right", 60404),
  layoutStatusbar: l("layout-statusbar", 60405),
  layoutMenubar: l("layout-menubar", 60406),
  layoutCentered: l("layout-centered", 60407),
  layoutSidebarRightOff: l("layout-sidebar-right-off", 60416),
  layoutPanelOff: l("layout-panel-off", 60417),
  layoutSidebarLeftOff: l("layout-sidebar-left-off", 60418),
  target: l("target", 60408),
  indent: l("indent", 60409),
  recordSmall: l("record-small", 60410),
  errorSmall: l("error-small", 60411),
  arrowCircleDown: l("arrow-circle-down", 60412),
  arrowCircleLeft: l("arrow-circle-left", 60413),
  arrowCircleRight: l("arrow-circle-right", 60414),
  arrowCircleUp: l("arrow-circle-up", 60415),
  heartFilled: l("heart-filled", 60420),
  map: l("map", 60421),
  mapFilled: l("map-filled", 60422),
  circleSmall: l("circle-small", 60423),
  bellSlash: l("bell-slash", 60424),
  bellSlashDot: l("bell-slash-dot", 60425),
  commentUnresolved: l("comment-unresolved", 60426),
  gitPullRequestGoToChanges: l("git-pull-request-go-to-changes", 60427),
  gitPullRequestNewChanges: l("git-pull-request-new-changes", 60428),
  searchFuzzy: l("search-fuzzy", 60429),
  commentDraft: l("comment-draft", 60430),
  send: l("send", 60431),
  sparkle: l("sparkle", 60432),
  insert: l("insert", 60433),
  mic: l("mic", 60434),
  // derived icons, that could become separate icons
  dialogError: l("dialog-error", "error"),
  dialogWarning: l("dialog-warning", "warning"),
  dialogInfo: l("dialog-info", "info"),
  dialogClose: l("dialog-close", "close"),
  treeItemExpanded: l("tree-item-expanded", "chevron-down"),
  // collapsed is done with rotation
  treeFilterOnTypeOn: l("tree-filter-on-type-on", "list-filter"),
  treeFilterOnTypeOff: l("tree-filter-on-type-off", "list-selection"),
  treeFilterClear: l("tree-filter-clear", "close"),
  treeItemLoading: l("tree-item-loading", "loading"),
  menuSelection: l("menu-selection", "check"),
  menuSubmenu: l("menu-submenu", "chevron-right"),
  menuBarMore: l("menubar-more", "more"),
  scrollbarButtonLeft: l("scrollbar-button-left", "triangle-left"),
  scrollbarButtonRight: l("scrollbar-button-right", "triangle-right"),
  scrollbarButtonUp: l("scrollbar-button-up", "triangle-up"),
  scrollbarButtonDown: l("scrollbar-button-down", "triangle-down"),
  toolBarMore: l("toolbar-more", "more"),
  quickInputBack: l("quick-input-back", "arrow-left")
};
var Bt = globalThis && globalThis.__awaiter || function(e, t, n, r) {
  function s(i) {
    return i instanceof n ? i : new n(function(a) {
      a(i);
    });
  }
  return new (n || (n = Promise))(function(i, a) {
    function o(h) {
      try {
        u(r.next(h));
      } catch (f) {
        a(f);
      }
    }
    function c(h) {
      try {
        u(r.throw(h));
      } catch (f) {
        a(f);
      }
    }
    function u(h) {
      h.done ? i(h.value) : s(h.value).then(o, c);
    }
    u((r = r.apply(e, t || [])).next());
  });
};
class bi {
  constructor() {
    this._tokenizationSupports = /* @__PURE__ */ new Map(), this._factories = /* @__PURE__ */ new Map(), this._onDidChange = new ie(), this.onDidChange = this._onDidChange.event, this._colorMap = null;
  }
  handleChange(t) {
    this._onDidChange.fire({
      changedLanguages: t,
      changedColorMap: !1
    });
  }
  register(t, n) {
    return this._tokenizationSupports.set(t, n), this.handleChange([t]), Ge(() => {
      this._tokenizationSupports.get(t) === n && (this._tokenizationSupports.delete(t), this.handleChange([t]));
    });
  }
  get(t) {
    return this._tokenizationSupports.get(t) || null;
  }
  registerFactory(t, n) {
    var r;
    (r = this._factories.get(t)) === null || r === void 0 || r.dispose();
    const s = new _i(this, t, n);
    return this._factories.set(t, s), Ge(() => {
      const i = this._factories.get(t);
      !i || i !== s || (this._factories.delete(t), i.dispose());
    });
  }
  getOrCreate(t) {
    return Bt(this, void 0, void 0, function* () {
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
class _i extends je {
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
    return Bt(this, void 0, void 0, function* () {
      return this._resolvePromise || (this._resolvePromise = this._create()), this._resolvePromise;
    });
  }
  _create() {
    return Bt(this, void 0, void 0, function* () {
      const t = yield this._factory.tokenizationSupport;
      this._isResolved = !0, t && !this._isDisposed && this._register(this._registry.register(this._languageId, t));
    });
  }
}
class xi {
  constructor(t, n, r) {
    this.offset = t, this.type = n, this.language = r, this._tokenBrand = void 0;
  }
  toString() {
    return "(" + this.offset + ", " + this.type + ")";
  }
}
var hn;
(function(e) {
  const t = /* @__PURE__ */ new Map();
  t.set(0, y.symbolMethod), t.set(1, y.symbolFunction), t.set(2, y.symbolConstructor), t.set(3, y.symbolField), t.set(4, y.symbolVariable), t.set(5, y.symbolClass), t.set(6, y.symbolStruct), t.set(7, y.symbolInterface), t.set(8, y.symbolModule), t.set(9, y.symbolProperty), t.set(10, y.symbolEvent), t.set(11, y.symbolOperator), t.set(12, y.symbolUnit), t.set(13, y.symbolValue), t.set(15, y.symbolEnum), t.set(14, y.symbolConstant), t.set(15, y.symbolEnum), t.set(16, y.symbolEnumMember), t.set(17, y.symbolKeyword), t.set(27, y.symbolSnippet), t.set(18, y.symbolText), t.set(19, y.symbolColor), t.set(20, y.symbolFile), t.set(21, y.symbolReference), t.set(22, y.symbolCustomColor), t.set(23, y.symbolFolder), t.set(24, y.symbolTypeParameter), t.set(25, y.account), t.set(26, y.issues);
  function n(i) {
    let a = t.get(i);
    return a || (console.info("No codicon found for CompletionItemKind " + i), a = y.symbolProperty), a;
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
  function s(i, a) {
    let o = r.get(i);
    return typeof o > "u" && !a && (o = 9), o;
  }
  e.fromString = s;
})(hn || (hn = {}));
var fn;
(function(e) {
  e[e.Automatic = 0] = "Automatic", e[e.Explicit = 1] = "Explicit";
})(fn || (fn = {}));
var dn;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.TriggerCharacter = 2] = "TriggerCharacter", e[e.ContentChange = 3] = "ContentChange";
})(dn || (dn = {}));
var mn;
(function(e) {
  e[e.Text = 0] = "Text", e[e.Read = 1] = "Read", e[e.Write = 2] = "Write";
})(mn || (mn = {}));
z("Array", "array"), z("Boolean", "boolean"), z("Class", "class"), z("Constant", "constant"), z("Constructor", "constructor"), z("Enum", "enumeration"), z("EnumMember", "enumeration member"), z("Event", "event"), z("Field", "field"), z("File", "file"), z("Function", "function"), z("Interface", "interface"), z("Key", "key"), z("Method", "method"), z("Module", "module"), z("Namespace", "namespace"), z("Null", "null"), z("Number", "number"), z("Object", "object"), z("Operator", "operator"), z("Package", "package"), z("Property", "property"), z("String", "string"), z("Struct", "struct"), z("TypeParameter", "type parameter"), z("Variable", "variable");
var gn;
(function(e) {
  const t = /* @__PURE__ */ new Map();
  t.set(0, y.symbolFile), t.set(1, y.symbolModule), t.set(2, y.symbolNamespace), t.set(3, y.symbolPackage), t.set(4, y.symbolClass), t.set(5, y.symbolMethod), t.set(6, y.symbolProperty), t.set(7, y.symbolField), t.set(8, y.symbolConstructor), t.set(9, y.symbolEnum), t.set(10, y.symbolInterface), t.set(11, y.symbolFunction), t.set(12, y.symbolVariable), t.set(13, y.symbolConstant), t.set(14, y.symbolString), t.set(15, y.symbolNumber), t.set(16, y.symbolBoolean), t.set(17, y.symbolArray), t.set(18, y.symbolObject), t.set(19, y.symbolKey), t.set(20, y.symbolNull), t.set(21, y.symbolEnumMember), t.set(22, y.symbolStruct), t.set(23, y.symbolEvent), t.set(24, y.symbolOperator), t.set(25, y.symbolTypeParameter);
  function n(r) {
    let s = t.get(r);
    return s || (console.info("No codicon found for SymbolKind " + r), s = y.symbolProperty), s;
  }
  e.toIcon = n;
})(gn || (gn = {}));
var bn;
(function(e) {
  function t(n) {
    return !n || typeof n != "object" ? !1 : typeof n.id == "string" && typeof n.title == "string";
  }
  e.is = t;
})(bn || (bn = {}));
var _n;
(function(e) {
  e[e.Type = 1] = "Type", e[e.Parameter = 2] = "Parameter";
})(_n || (_n = {}));
new bi();
var xn;
(function(e) {
  e[e.Unknown = 0] = "Unknown", e[e.Disabled = 1] = "Disabled", e[e.Enabled = 2] = "Enabled";
})(xn || (xn = {}));
var pn;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.Auto = 2] = "Auto";
})(pn || (pn = {}));
var vn;
(function(e) {
  e[e.None = 0] = "None", e[e.KeepWhitespace = 1] = "KeepWhitespace", e[e.InsertAsSnippet = 4] = "InsertAsSnippet";
})(vn || (vn = {}));
var wn;
(function(e) {
  e[e.Method = 0] = "Method", e[e.Function = 1] = "Function", e[e.Constructor = 2] = "Constructor", e[e.Field = 3] = "Field", e[e.Variable = 4] = "Variable", e[e.Class = 5] = "Class", e[e.Struct = 6] = "Struct", e[e.Interface = 7] = "Interface", e[e.Module = 8] = "Module", e[e.Property = 9] = "Property", e[e.Event = 10] = "Event", e[e.Operator = 11] = "Operator", e[e.Unit = 12] = "Unit", e[e.Value = 13] = "Value", e[e.Constant = 14] = "Constant", e[e.Enum = 15] = "Enum", e[e.EnumMember = 16] = "EnumMember", e[e.Keyword = 17] = "Keyword", e[e.Text = 18] = "Text", e[e.Color = 19] = "Color", e[e.File = 20] = "File", e[e.Reference = 21] = "Reference", e[e.Customcolor = 22] = "Customcolor", e[e.Folder = 23] = "Folder", e[e.TypeParameter = 24] = "TypeParameter", e[e.User = 25] = "User", e[e.Issue = 26] = "Issue", e[e.Snippet = 27] = "Snippet";
})(wn || (wn = {}));
var Ln;
(function(e) {
  e[e.Deprecated = 1] = "Deprecated";
})(Ln || (Ln = {}));
var Nn;
(function(e) {
  e[e.Invoke = 0] = "Invoke", e[e.TriggerCharacter = 1] = "TriggerCharacter", e[e.TriggerForIncompleteCompletions = 2] = "TriggerForIncompleteCompletions";
})(Nn || (Nn = {}));
var Sn;
(function(e) {
  e[e.EXACT = 0] = "EXACT", e[e.ABOVE = 1] = "ABOVE", e[e.BELOW = 2] = "BELOW";
})(Sn || (Sn = {}));
var Cn;
(function(e) {
  e[e.NotSet = 0] = "NotSet", e[e.ContentFlush = 1] = "ContentFlush", e[e.RecoverFromMarkers = 2] = "RecoverFromMarkers", e[e.Explicit = 3] = "Explicit", e[e.Paste = 4] = "Paste", e[e.Undo = 5] = "Undo", e[e.Redo = 6] = "Redo";
})(Cn || (Cn = {}));
var An;
(function(e) {
  e[e.LF = 1] = "LF", e[e.CRLF = 2] = "CRLF";
})(An || (An = {}));
var Rn;
(function(e) {
  e[e.Text = 0] = "Text", e[e.Read = 1] = "Read", e[e.Write = 2] = "Write";
})(Rn || (Rn = {}));
var yn;
(function(e) {
  e[e.None = 0] = "None", e[e.Keep = 1] = "Keep", e[e.Brackets = 2] = "Brackets", e[e.Advanced = 3] = "Advanced", e[e.Full = 4] = "Full";
})(yn || (yn = {}));
var Mn;
(function(e) {
  e[e.acceptSuggestionOnCommitCharacter = 0] = "acceptSuggestionOnCommitCharacter", e[e.acceptSuggestionOnEnter = 1] = "acceptSuggestionOnEnter", e[e.accessibilitySupport = 2] = "accessibilitySupport", e[e.accessibilityPageSize = 3] = "accessibilityPageSize", e[e.ariaLabel = 4] = "ariaLabel", e[e.ariaRequired = 5] = "ariaRequired", e[e.autoClosingBrackets = 6] = "autoClosingBrackets", e[e.autoClosingComments = 7] = "autoClosingComments", e[e.screenReaderAnnounceInlineSuggestion = 8] = "screenReaderAnnounceInlineSuggestion", e[e.autoClosingDelete = 9] = "autoClosingDelete", e[e.autoClosingOvertype = 10] = "autoClosingOvertype", e[e.autoClosingQuotes = 11] = "autoClosingQuotes", e[e.autoIndent = 12] = "autoIndent", e[e.automaticLayout = 13] = "automaticLayout", e[e.autoSurround = 14] = "autoSurround", e[e.bracketPairColorization = 15] = "bracketPairColorization", e[e.guides = 16] = "guides", e[e.codeLens = 17] = "codeLens", e[e.codeLensFontFamily = 18] = "codeLensFontFamily", e[e.codeLensFontSize = 19] = "codeLensFontSize", e[e.colorDecorators = 20] = "colorDecorators", e[e.colorDecoratorsLimit = 21] = "colorDecoratorsLimit", e[e.columnSelection = 22] = "columnSelection", e[e.comments = 23] = "comments", e[e.contextmenu = 24] = "contextmenu", e[e.copyWithSyntaxHighlighting = 25] = "copyWithSyntaxHighlighting", e[e.cursorBlinking = 26] = "cursorBlinking", e[e.cursorSmoothCaretAnimation = 27] = "cursorSmoothCaretAnimation", e[e.cursorStyle = 28] = "cursorStyle", e[e.cursorSurroundingLines = 29] = "cursorSurroundingLines", e[e.cursorSurroundingLinesStyle = 30] = "cursorSurroundingLinesStyle", e[e.cursorWidth = 31] = "cursorWidth", e[e.disableLayerHinting = 32] = "disableLayerHinting", e[e.disableMonospaceOptimizations = 33] = "disableMonospaceOptimizations", e[e.domReadOnly = 34] = "domReadOnly", e[e.dragAndDrop = 35] = "dragAndDrop", e[e.dropIntoEditor = 36] = "dropIntoEditor", e[e.emptySelectionClipboard = 37] = "emptySelectionClipboard", e[e.experimentalWhitespaceRendering = 38] = "experimentalWhitespaceRendering", e[e.extraEditorClassName = 39] = "extraEditorClassName", e[e.fastScrollSensitivity = 40] = "fastScrollSensitivity", e[e.find = 41] = "find", e[e.fixedOverflowWidgets = 42] = "fixedOverflowWidgets", e[e.folding = 43] = "folding", e[e.foldingStrategy = 44] = "foldingStrategy", e[e.foldingHighlight = 45] = "foldingHighlight", e[e.foldingImportsByDefault = 46] = "foldingImportsByDefault", e[e.foldingMaximumRegions = 47] = "foldingMaximumRegions", e[e.unfoldOnClickAfterEndOfLine = 48] = "unfoldOnClickAfterEndOfLine", e[e.fontFamily = 49] = "fontFamily", e[e.fontInfo = 50] = "fontInfo", e[e.fontLigatures = 51] = "fontLigatures", e[e.fontSize = 52] = "fontSize", e[e.fontWeight = 53] = "fontWeight", e[e.fontVariations = 54] = "fontVariations", e[e.formatOnPaste = 55] = "formatOnPaste", e[e.formatOnType = 56] = "formatOnType", e[e.glyphMargin = 57] = "glyphMargin", e[e.gotoLocation = 58] = "gotoLocation", e[e.hideCursorInOverviewRuler = 59] = "hideCursorInOverviewRuler", e[e.hover = 60] = "hover", e[e.inDiffEditor = 61] = "inDiffEditor", e[e.inlineSuggest = 62] = "inlineSuggest", e[e.letterSpacing = 63] = "letterSpacing", e[e.lightbulb = 64] = "lightbulb", e[e.lineDecorationsWidth = 65] = "lineDecorationsWidth", e[e.lineHeight = 66] = "lineHeight", e[e.lineNumbers = 67] = "lineNumbers", e[e.lineNumbersMinChars = 68] = "lineNumbersMinChars", e[e.linkedEditing = 69] = "linkedEditing", e[e.links = 70] = "links", e[e.matchBrackets = 71] = "matchBrackets", e[e.minimap = 72] = "minimap", e[e.mouseStyle = 73] = "mouseStyle", e[e.mouseWheelScrollSensitivity = 74] = "mouseWheelScrollSensitivity", e[e.mouseWheelZoom = 75] = "mouseWheelZoom", e[e.multiCursorMergeOverlapping = 76] = "multiCursorMergeOverlapping", e[e.multiCursorModifier = 77] = "multiCursorModifier", e[e.multiCursorPaste = 78] = "multiCursorPaste", e[e.multiCursorLimit = 79] = "multiCursorLimit", e[e.occurrencesHighlight = 80] = "occurrencesHighlight", e[e.overviewRulerBorder = 81] = "overviewRulerBorder", e[e.overviewRulerLanes = 82] = "overviewRulerLanes", e[e.padding = 83] = "padding", e[e.pasteAs = 84] = "pasteAs", e[e.parameterHints = 85] = "parameterHints", e[e.peekWidgetDefaultFocus = 86] = "peekWidgetDefaultFocus", e[e.definitionLinkOpensInPeek = 87] = "definitionLinkOpensInPeek", e[e.quickSuggestions = 88] = "quickSuggestions", e[e.quickSuggestionsDelay = 89] = "quickSuggestionsDelay", e[e.readOnly = 90] = "readOnly", e[e.readOnlyMessage = 91] = "readOnlyMessage", e[e.renameOnType = 92] = "renameOnType", e[e.renderControlCharacters = 93] = "renderControlCharacters", e[e.renderFinalNewline = 94] = "renderFinalNewline", e[e.renderLineHighlight = 95] = "renderLineHighlight", e[e.renderLineHighlightOnlyWhenFocus = 96] = "renderLineHighlightOnlyWhenFocus", e[e.renderValidationDecorations = 97] = "renderValidationDecorations", e[e.renderWhitespace = 98] = "renderWhitespace", e[e.revealHorizontalRightPadding = 99] = "revealHorizontalRightPadding", e[e.roundedSelection = 100] = "roundedSelection", e[e.rulers = 101] = "rulers", e[e.scrollbar = 102] = "scrollbar", e[e.scrollBeyondLastColumn = 103] = "scrollBeyondLastColumn", e[e.scrollBeyondLastLine = 104] = "scrollBeyondLastLine", e[e.scrollPredominantAxis = 105] = "scrollPredominantAxis", e[e.selectionClipboard = 106] = "selectionClipboard", e[e.selectionHighlight = 107] = "selectionHighlight", e[e.selectOnLineNumbers = 108] = "selectOnLineNumbers", e[e.showFoldingControls = 109] = "showFoldingControls", e[e.showUnused = 110] = "showUnused", e[e.snippetSuggestions = 111] = "snippetSuggestions", e[e.smartSelect = 112] = "smartSelect", e[e.smoothScrolling = 113] = "smoothScrolling", e[e.stickyScroll = 114] = "stickyScroll", e[e.stickyTabStops = 115] = "stickyTabStops", e[e.stopRenderingLineAfter = 116] = "stopRenderingLineAfter", e[e.suggest = 117] = "suggest", e[e.suggestFontSize = 118] = "suggestFontSize", e[e.suggestLineHeight = 119] = "suggestLineHeight", e[e.suggestOnTriggerCharacters = 120] = "suggestOnTriggerCharacters", e[e.suggestSelection = 121] = "suggestSelection", e[e.tabCompletion = 122] = "tabCompletion", e[e.tabIndex = 123] = "tabIndex", e[e.unicodeHighlighting = 124] = "unicodeHighlighting", e[e.unusualLineTerminators = 125] = "unusualLineTerminators", e[e.useShadowDOM = 126] = "useShadowDOM", e[e.useTabStops = 127] = "useTabStops", e[e.wordBreak = 128] = "wordBreak", e[e.wordSeparators = 129] = "wordSeparators", e[e.wordWrap = 130] = "wordWrap", e[e.wordWrapBreakAfterCharacters = 131] = "wordWrapBreakAfterCharacters", e[e.wordWrapBreakBeforeCharacters = 132] = "wordWrapBreakBeforeCharacters", e[e.wordWrapColumn = 133] = "wordWrapColumn", e[e.wordWrapOverride1 = 134] = "wordWrapOverride1", e[e.wordWrapOverride2 = 135] = "wordWrapOverride2", e[e.wrappingIndent = 136] = "wrappingIndent", e[e.wrappingStrategy = 137] = "wrappingStrategy", e[e.showDeprecated = 138] = "showDeprecated", e[e.inlayHints = 139] = "inlayHints", e[e.editorClassName = 140] = "editorClassName", e[e.pixelRatio = 141] = "pixelRatio", e[e.tabFocusMode = 142] = "tabFocusMode", e[e.layoutInfo = 143] = "layoutInfo", e[e.wrappingInfo = 144] = "wrappingInfo", e[e.defaultColorDecorators = 145] = "defaultColorDecorators", e[e.colorDecoratorsActivatedOn = 146] = "colorDecoratorsActivatedOn", e[e.inlineCompletionsAccessibilityVerbose = 147] = "inlineCompletionsAccessibilityVerbose";
})(Mn || (Mn = {}));
var En;
(function(e) {
  e[e.TextDefined = 0] = "TextDefined", e[e.LF = 1] = "LF", e[e.CRLF = 2] = "CRLF";
})(En || (En = {}));
var kn;
(function(e) {
  e[e.LF = 0] = "LF", e[e.CRLF = 1] = "CRLF";
})(kn || (kn = {}));
var Pn;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Right = 2] = "Right";
})(Pn || (Pn = {}));
var Fn;
(function(e) {
  e[e.None = 0] = "None", e[e.Indent = 1] = "Indent", e[e.IndentOutdent = 2] = "IndentOutdent", e[e.Outdent = 3] = "Outdent";
})(Fn || (Fn = {}));
var Dn;
(function(e) {
  e[e.Both = 0] = "Both", e[e.Right = 1] = "Right", e[e.Left = 2] = "Left", e[e.None = 3] = "None";
})(Dn || (Dn = {}));
var Tn;
(function(e) {
  e[e.Type = 1] = "Type", e[e.Parameter = 2] = "Parameter";
})(Tn || (Tn = {}));
var Vn;
(function(e) {
  e[e.Automatic = 0] = "Automatic", e[e.Explicit = 1] = "Explicit";
})(Vn || (Vn = {}));
var qt;
(function(e) {
  e[e.DependsOnKbLayout = -1] = "DependsOnKbLayout", e[e.Unknown = 0] = "Unknown", e[e.Backspace = 1] = "Backspace", e[e.Tab = 2] = "Tab", e[e.Enter = 3] = "Enter", e[e.Shift = 4] = "Shift", e[e.Ctrl = 5] = "Ctrl", e[e.Alt = 6] = "Alt", e[e.PauseBreak = 7] = "PauseBreak", e[e.CapsLock = 8] = "CapsLock", e[e.Escape = 9] = "Escape", e[e.Space = 10] = "Space", e[e.PageUp = 11] = "PageUp", e[e.PageDown = 12] = "PageDown", e[e.End = 13] = "End", e[e.Home = 14] = "Home", e[e.LeftArrow = 15] = "LeftArrow", e[e.UpArrow = 16] = "UpArrow", e[e.RightArrow = 17] = "RightArrow", e[e.DownArrow = 18] = "DownArrow", e[e.Insert = 19] = "Insert", e[e.Delete = 20] = "Delete", e[e.Digit0 = 21] = "Digit0", e[e.Digit1 = 22] = "Digit1", e[e.Digit2 = 23] = "Digit2", e[e.Digit3 = 24] = "Digit3", e[e.Digit4 = 25] = "Digit4", e[e.Digit5 = 26] = "Digit5", e[e.Digit6 = 27] = "Digit6", e[e.Digit7 = 28] = "Digit7", e[e.Digit8 = 29] = "Digit8", e[e.Digit9 = 30] = "Digit9", e[e.KeyA = 31] = "KeyA", e[e.KeyB = 32] = "KeyB", e[e.KeyC = 33] = "KeyC", e[e.KeyD = 34] = "KeyD", e[e.KeyE = 35] = "KeyE", e[e.KeyF = 36] = "KeyF", e[e.KeyG = 37] = "KeyG", e[e.KeyH = 38] = "KeyH", e[e.KeyI = 39] = "KeyI", e[e.KeyJ = 40] = "KeyJ", e[e.KeyK = 41] = "KeyK", e[e.KeyL = 42] = "KeyL", e[e.KeyM = 43] = "KeyM", e[e.KeyN = 44] = "KeyN", e[e.KeyO = 45] = "KeyO", e[e.KeyP = 46] = "KeyP", e[e.KeyQ = 47] = "KeyQ", e[e.KeyR = 48] = "KeyR", e[e.KeyS = 49] = "KeyS", e[e.KeyT = 50] = "KeyT", e[e.KeyU = 51] = "KeyU", e[e.KeyV = 52] = "KeyV", e[e.KeyW = 53] = "KeyW", e[e.KeyX = 54] = "KeyX", e[e.KeyY = 55] = "KeyY", e[e.KeyZ = 56] = "KeyZ", e[e.Meta = 57] = "Meta", e[e.ContextMenu = 58] = "ContextMenu", e[e.F1 = 59] = "F1", e[e.F2 = 60] = "F2", e[e.F3 = 61] = "F3", e[e.F4 = 62] = "F4", e[e.F5 = 63] = "F5", e[e.F6 = 64] = "F6", e[e.F7 = 65] = "F7", e[e.F8 = 66] = "F8", e[e.F9 = 67] = "F9", e[e.F10 = 68] = "F10", e[e.F11 = 69] = "F11", e[e.F12 = 70] = "F12", e[e.F13 = 71] = "F13", e[e.F14 = 72] = "F14", e[e.F15 = 73] = "F15", e[e.F16 = 74] = "F16", e[e.F17 = 75] = "F17", e[e.F18 = 76] = "F18", e[e.F19 = 77] = "F19", e[e.F20 = 78] = "F20", e[e.F21 = 79] = "F21", e[e.F22 = 80] = "F22", e[e.F23 = 81] = "F23", e[e.F24 = 82] = "F24", e[e.NumLock = 83] = "NumLock", e[e.ScrollLock = 84] = "ScrollLock", e[e.Semicolon = 85] = "Semicolon", e[e.Equal = 86] = "Equal", e[e.Comma = 87] = "Comma", e[e.Minus = 88] = "Minus", e[e.Period = 89] = "Period", e[e.Slash = 90] = "Slash", e[e.Backquote = 91] = "Backquote", e[e.BracketLeft = 92] = "BracketLeft", e[e.Backslash = 93] = "Backslash", e[e.BracketRight = 94] = "BracketRight", e[e.Quote = 95] = "Quote", e[e.OEM_8 = 96] = "OEM_8", e[e.IntlBackslash = 97] = "IntlBackslash", e[e.Numpad0 = 98] = "Numpad0", e[e.Numpad1 = 99] = "Numpad1", e[e.Numpad2 = 100] = "Numpad2", e[e.Numpad3 = 101] = "Numpad3", e[e.Numpad4 = 102] = "Numpad4", e[e.Numpad5 = 103] = "Numpad5", e[e.Numpad6 = 104] = "Numpad6", e[e.Numpad7 = 105] = "Numpad7", e[e.Numpad8 = 106] = "Numpad8", e[e.Numpad9 = 107] = "Numpad9", e[e.NumpadMultiply = 108] = "NumpadMultiply", e[e.NumpadAdd = 109] = "NumpadAdd", e[e.NUMPAD_SEPARATOR = 110] = "NUMPAD_SEPARATOR", e[e.NumpadSubtract = 111] = "NumpadSubtract", e[e.NumpadDecimal = 112] = "NumpadDecimal", e[e.NumpadDivide = 113] = "NumpadDivide", e[e.KEY_IN_COMPOSITION = 114] = "KEY_IN_COMPOSITION", e[e.ABNT_C1 = 115] = "ABNT_C1", e[e.ABNT_C2 = 116] = "ABNT_C2", e[e.AudioVolumeMute = 117] = "AudioVolumeMute", e[e.AudioVolumeUp = 118] = "AudioVolumeUp", e[e.AudioVolumeDown = 119] = "AudioVolumeDown", e[e.BrowserSearch = 120] = "BrowserSearch", e[e.BrowserHome = 121] = "BrowserHome", e[e.BrowserBack = 122] = "BrowserBack", e[e.BrowserForward = 123] = "BrowserForward", e[e.MediaTrackNext = 124] = "MediaTrackNext", e[e.MediaTrackPrevious = 125] = "MediaTrackPrevious", e[e.MediaStop = 126] = "MediaStop", e[e.MediaPlayPause = 127] = "MediaPlayPause", e[e.LaunchMediaPlayer = 128] = "LaunchMediaPlayer", e[e.LaunchMail = 129] = "LaunchMail", e[e.LaunchApp2 = 130] = "LaunchApp2", e[e.Clear = 131] = "Clear", e[e.MAX_VALUE = 132] = "MAX_VALUE";
})(qt || (qt = {}));
var It;
(function(e) {
  e[e.Hint = 1] = "Hint", e[e.Info = 2] = "Info", e[e.Warning = 4] = "Warning", e[e.Error = 8] = "Error";
})(It || (It = {}));
var Ut;
(function(e) {
  e[e.Unnecessary = 1] = "Unnecessary", e[e.Deprecated = 2] = "Deprecated";
})(Ut || (Ut = {}));
var Bn;
(function(e) {
  e[e.Inline = 1] = "Inline", e[e.Gutter = 2] = "Gutter";
})(Bn || (Bn = {}));
var qn;
(function(e) {
  e[e.UNKNOWN = 0] = "UNKNOWN", e[e.TEXTAREA = 1] = "TEXTAREA", e[e.GUTTER_GLYPH_MARGIN = 2] = "GUTTER_GLYPH_MARGIN", e[e.GUTTER_LINE_NUMBERS = 3] = "GUTTER_LINE_NUMBERS", e[e.GUTTER_LINE_DECORATIONS = 4] = "GUTTER_LINE_DECORATIONS", e[e.GUTTER_VIEW_ZONE = 5] = "GUTTER_VIEW_ZONE", e[e.CONTENT_TEXT = 6] = "CONTENT_TEXT", e[e.CONTENT_EMPTY = 7] = "CONTENT_EMPTY", e[e.CONTENT_VIEW_ZONE = 8] = "CONTENT_VIEW_ZONE", e[e.CONTENT_WIDGET = 9] = "CONTENT_WIDGET", e[e.OVERVIEW_RULER = 10] = "OVERVIEW_RULER", e[e.SCROLLBAR = 11] = "SCROLLBAR", e[e.OVERLAY_WIDGET = 12] = "OVERLAY_WIDGET", e[e.OUTSIDE_EDITOR = 13] = "OUTSIDE_EDITOR";
})(qn || (qn = {}));
var In;
(function(e) {
  e[e.TOP_RIGHT_CORNER = 0] = "TOP_RIGHT_CORNER", e[e.BOTTOM_RIGHT_CORNER = 1] = "BOTTOM_RIGHT_CORNER", e[e.TOP_CENTER = 2] = "TOP_CENTER";
})(In || (In = {}));
var Un;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Center = 2] = "Center", e[e.Right = 4] = "Right", e[e.Full = 7] = "Full";
})(Un || (Un = {}));
var Hn;
(function(e) {
  e[e.Left = 0] = "Left", e[e.Right = 1] = "Right", e[e.None = 2] = "None", e[e.LeftOfInjectedText = 3] = "LeftOfInjectedText", e[e.RightOfInjectedText = 4] = "RightOfInjectedText";
})(Hn || (Hn = {}));
var Wn;
(function(e) {
  e[e.Off = 0] = "Off", e[e.On = 1] = "On", e[e.Relative = 2] = "Relative", e[e.Interval = 3] = "Interval", e[e.Custom = 4] = "Custom";
})(Wn || (Wn = {}));
var zn;
(function(e) {
  e[e.None = 0] = "None", e[e.Text = 1] = "Text", e[e.Blocks = 2] = "Blocks";
})(zn || (zn = {}));
var $n;
(function(e) {
  e[e.Smooth = 0] = "Smooth", e[e.Immediate = 1] = "Immediate";
})($n || ($n = {}));
var On;
(function(e) {
  e[e.Auto = 1] = "Auto", e[e.Hidden = 2] = "Hidden", e[e.Visible = 3] = "Visible";
})(On || (On = {}));
var Ht;
(function(e) {
  e[e.LTR = 0] = "LTR", e[e.RTL = 1] = "RTL";
})(Ht || (Ht = {}));
var Gn;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.TriggerCharacter = 2] = "TriggerCharacter", e[e.ContentChange = 3] = "ContentChange";
})(Gn || (Gn = {}));
var jn;
(function(e) {
  e[e.File = 0] = "File", e[e.Module = 1] = "Module", e[e.Namespace = 2] = "Namespace", e[e.Package = 3] = "Package", e[e.Class = 4] = "Class", e[e.Method = 5] = "Method", e[e.Property = 6] = "Property", e[e.Field = 7] = "Field", e[e.Constructor = 8] = "Constructor", e[e.Enum = 9] = "Enum", e[e.Interface = 10] = "Interface", e[e.Function = 11] = "Function", e[e.Variable = 12] = "Variable", e[e.Constant = 13] = "Constant", e[e.String = 14] = "String", e[e.Number = 15] = "Number", e[e.Boolean = 16] = "Boolean", e[e.Array = 17] = "Array", e[e.Object = 18] = "Object", e[e.Key = 19] = "Key", e[e.Null = 20] = "Null", e[e.EnumMember = 21] = "EnumMember", e[e.Struct = 22] = "Struct", e[e.Event = 23] = "Event", e[e.Operator = 24] = "Operator", e[e.TypeParameter = 25] = "TypeParameter";
})(jn || (jn = {}));
var Qn;
(function(e) {
  e[e.Deprecated = 1] = "Deprecated";
})(Qn || (Qn = {}));
var Xn;
(function(e) {
  e[e.Hidden = 0] = "Hidden", e[e.Blink = 1] = "Blink", e[e.Smooth = 2] = "Smooth", e[e.Phase = 3] = "Phase", e[e.Expand = 4] = "Expand", e[e.Solid = 5] = "Solid";
})(Xn || (Xn = {}));
var Jn;
(function(e) {
  e[e.Line = 1] = "Line", e[e.Block = 2] = "Block", e[e.Underline = 3] = "Underline", e[e.LineThin = 4] = "LineThin", e[e.BlockOutline = 5] = "BlockOutline", e[e.UnderlineThin = 6] = "UnderlineThin";
})(Jn || (Jn = {}));
var Yn;
(function(e) {
  e[e.AlwaysGrowsWhenTypingAtEdges = 0] = "AlwaysGrowsWhenTypingAtEdges", e[e.NeverGrowsWhenTypingAtEdges = 1] = "NeverGrowsWhenTypingAtEdges", e[e.GrowsOnlyWhenTypingBefore = 2] = "GrowsOnlyWhenTypingBefore", e[e.GrowsOnlyWhenTypingAfter = 3] = "GrowsOnlyWhenTypingAfter";
})(Yn || (Yn = {}));
var Zn;
(function(e) {
  e[e.None = 0] = "None", e[e.Same = 1] = "Same", e[e.Indent = 2] = "Indent", e[e.DeepIndent = 3] = "DeepIndent";
})(Zn || (Zn = {}));
class Ke {
  static chord(t, n) {
    return gi(t, n);
  }
}
Ke.CtrlCmd = 2048;
Ke.Shift = 1024;
Ke.Alt = 512;
Ke.WinCtrl = 256;
function pi() {
  return {
    editor: void 0,
    // undefined override expected here
    languages: void 0,
    // undefined override expected here
    CancellationTokenSource: hi,
    Emitter: ie,
    KeyCode: qt,
    KeyMod: Ke,
    Position: J,
    Range: D,
    Selection: ne,
    SelectionDirection: Ht,
    MarkerSeverity: It,
    MarkerTag: Ut,
    Uri: Ce,
    Token: xi
  };
}
var Kn;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Center = 2] = "Center", e[e.Right = 4] = "Right", e[e.Full = 7] = "Full";
})(Kn || (Kn = {}));
var er;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Right = 2] = "Right";
})(er || (er = {}));
var tr;
(function(e) {
  e[e.Inline = 1] = "Inline", e[e.Gutter = 2] = "Gutter";
})(tr || (tr = {}));
var nr;
(function(e) {
  e[e.Both = 0] = "Both", e[e.Right = 1] = "Right", e[e.Left = 2] = "Left", e[e.None = 3] = "None";
})(nr || (nr = {}));
function vi(e, t, n, r, s) {
  if (r === 0)
    return !0;
  const i = t.charCodeAt(r - 1);
  if (e.get(i) !== 0 || i === 13 || i === 10)
    return !0;
  if (s > 0) {
    const a = t.charCodeAt(r);
    if (e.get(a) !== 0)
      return !0;
  }
  return !1;
}
function wi(e, t, n, r, s) {
  if (r + s === n)
    return !0;
  const i = t.charCodeAt(r + s);
  if (e.get(i) !== 0 || i === 13 || i === 10)
    return !0;
  if (s > 0) {
    const a = t.charCodeAt(r + s - 1);
    if (e.get(a) !== 0)
      return !0;
  }
  return !1;
}
function Li(e, t, n, r, s) {
  return vi(e, t, n, r, s) && wi(e, t, n, r, s);
}
class Ni {
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
      const s = r.index, i = r[0].length;
      if (s === this._prevMatchStartIndex && i === this._prevMatchLength) {
        if (i === 0) {
          _s(t, n, this._searchRegex.lastIndex) > 65535 ? this._searchRegex.lastIndex += 2 : this._searchRegex.lastIndex += 1;
          continue;
        }
        return null;
      }
      if (this._prevMatchStartIndex = s, this._prevMatchLength = i, !this._wordSeparators || Li(this._wordSeparators, t, n, s, i))
        return r;
    } while (r);
    return null;
  }
}
function Si(e, t = "Unreachable") {
  throw new Error(t);
}
function gt(e) {
  if (!e()) {
    debugger;
    e(), pr(new xe("Assertion Failed"));
  }
}
function Dr(e, t) {
  let n = 0;
  for (; n < e.length - 1; ) {
    const r = e[n], s = e[n + 1];
    if (!t(r, s))
      return !1;
    n++;
  }
  return !0;
}
class Ci {
  static computeUnicodeHighlights(t, n, r) {
    const s = r ? r.startLineNumber : 1, i = r ? r.endLineNumber : t.getLineCount(), a = new rr(n), o = a.getCandidateCodePoints();
    let c;
    o === "allNonBasicAscii" ? c = new RegExp("[^\\t\\n\\r\\x20-\\x7E]", "g") : c = new RegExp(`${Ai(Array.from(o))}`, "g");
    const u = new Ni(null, c), h = [];
    let f = !1, m, d = 0, g = 0, b = 0;
    e:
      for (let w = s, L = i; w <= L; w++) {
        const _ = t.getLineContent(w), x = _.length;
        u.reset(0);
        do
          if (m = u.next(_), m) {
            let S = m.index, N = m.index + m[0].length;
            if (S > 0) {
              const I = _.charCodeAt(S - 1);
              Et(I) && S--;
            }
            if (N + 1 < x) {
              const I = _.charCodeAt(N - 1);
              Et(I) && N++;
            }
            const R = _.substring(S, N);
            let M = Qt(S + 1, kr, _, 0);
            M && M.endColumn <= S + 1 && (M = null);
            const E = a.shouldHighlightNonBasicASCII(R, M ? M.word : null);
            if (E !== 0) {
              E === 3 ? d++ : E === 2 ? g++ : E === 1 ? b++ : Si();
              const I = 1e3;
              if (h.length >= I) {
                f = !0;
                break e;
              }
              h.push(new D(w, S + 1, w, N + 1));
            }
          }
        while (m);
      }
    return {
      ranges: h,
      hasMore: f,
      ambiguousCharacterCount: d,
      invisibleCharacterCount: g,
      nonBasicAsciiCharacterCount: b
    };
  }
  static computeUnicodeHighlightReason(t, n) {
    const r = new rr(n);
    switch (r.shouldHighlightNonBasicASCII(t, null)) {
      case 0:
        return null;
      case 2:
        return {
          kind: 1
          /* UnicodeHighlighterReasonKind.Invisible */
        };
      case 3: {
        const i = t.codePointAt(0), a = r.ambiguousCharacters.getPrimaryConfusable(i), o = Re.getLocales().filter((c) => !Re.getInstance(/* @__PURE__ */ new Set([...n.allowedLocales, c])).isAmbiguous(i));
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
function Ai(e, t) {
  return `[${hs(e.map((r) => String.fromCodePoint(r)).join(""))}]`;
}
class rr {
  constructor(t) {
    this.options = t, this.allowedCodePoints = new Set(t.allowedCodePoints), this.ambiguousCharacters = Re.getInstance(new Set(t.allowedLocales));
  }
  getCandidateCodePoints() {
    if (this.options.nonBasicASCII)
      return "allNonBasicAscii";
    const t = /* @__PURE__ */ new Set();
    if (this.options.invisibleCharacters)
      for (const n of pe.codePoints)
        sr(String.fromCodePoint(n)) || t.add(n);
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
    let s = !1, i = !1;
    if (n)
      for (const a of n) {
        const o = a.codePointAt(0), c = ps(a);
        s = s || c, !c && !this.ambiguousCharacters.isAmbiguous(o) && !pe.isInvisibleCharacter(o) && (i = !0);
      }
    return (
      /* Don't allow mixing weird looking characters with ASCII */
      !s && /* Is there an obviously weird looking character? */
      i ? 0 : this.options.invisibleCharacters && !sr(t) && pe.isInvisibleCharacter(r) ? 2 : this.options.ambiguousCharacters && this.ambiguousCharacters.isAmbiguous(r) ? 3 : 0
    );
  }
}
function sr(e) {
  return e === " " || e === `
` || e === "	";
}
class ut {
  constructor(t, n, r) {
    this.changes = t, this.moves = n, this.hitTimeout = r;
  }
}
class Ri {
  constructor(t, n) {
    this.lineRangeMapping = t, this.changes = n;
  }
}
class T {
  static addRange(t, n) {
    let r = 0;
    for (; r < n.length && n[r].endExclusive < t.start; )
      r++;
    let s = r;
    for (; s < n.length && n[s].start <= t.endExclusive; )
      s++;
    if (r === s)
      n.splice(r, 0, t);
    else {
      const i = Math.min(t.start, n[r].start), a = Math.max(t.endExclusive, n[s - 1].endExclusive);
      n.splice(r, s - r, new T(i, a));
    }
  }
  static tryCreate(t, n) {
    if (!(t > n))
      return new T(t, n);
  }
  static ofLength(t) {
    return new T(0, t);
  }
  constructor(t, n) {
    if (this.start = t, this.endExclusive = n, t > n)
      throw new xe(`Invalid range: ${this.toString()}`);
  }
  get isEmpty() {
    return this.start === this.endExclusive;
  }
  delta(t) {
    return new T(this.start + t, this.endExclusive + t);
  }
  deltaStart(t) {
    return new T(this.start + t, this.endExclusive);
  }
  deltaEnd(t) {
    return new T(this.start, this.endExclusive + t);
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
    return new T(Math.min(this.start, t.start), Math.max(this.endExclusive, t.endExclusive));
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
      return new T(n, r);
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
      throw new xe(`Invalid clipping range: ${this.toString()}`);
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
      throw new xe(`Invalid clipping range: ${this.toString()}`);
    return t < this.start ? this.endExclusive - (this.start - t) % this.length : t >= this.endExclusive ? this.start + (t - this.start) % this.length : t;
  }
  forEach(t) {
    for (let n = this.start; n < this.endExclusive; n++)
      t(n);
  }
}
function Xe(e, t) {
  const n = Je(e, t);
  return n === -1 ? void 0 : e[n];
}
function Je(e, t, n = 0, r = e.length) {
  let s = n, i = r;
  for (; s < i; ) {
    const a = Math.floor((s + i) / 2);
    t(e[a]) ? s = a + 1 : i = a;
  }
  return s - 1;
}
function yi(e, t) {
  const n = Wt(e, t);
  return n === e.length ? void 0 : e[n];
}
function Wt(e, t, n = 0, r = e.length) {
  let s = n, i = r;
  for (; s < i; ) {
    const a = Math.floor((s + i) / 2);
    t(e[a]) ? i = a : s = a + 1;
  }
  return s;
}
class et {
  constructor(t) {
    this._array = t, this._findLastMonotonousLastIdx = 0;
  }
  /**
   * The predicate must be monotonous, i.e. `arr.map(predicate)` must be like `[true, ..., true, false, ..., false]`!
   * For subsequent calls, current predicate must be weaker than (or equal to) the previous predicate, i.e. more entries must be `true`.
   */
  findLastMonotonous(t) {
    if (et.assertInvariants) {
      if (this._prevFindLastPredicate) {
        for (const r of this._array)
          if (this._prevFindLastPredicate(r) && !t(r))
            throw new Error("MonotonousArray: current predicate must be weaker than (or equal to) the previous predicate.");
      }
      this._prevFindLastPredicate = t;
    }
    const n = Je(this._array, t, this._findLastMonotonousLastIdx);
    return this._findLastMonotonousLastIdx = n + 1, n === -1 ? void 0 : this._array[n];
  }
}
et.assertInvariants = !1;
class V {
  static fromRange(t) {
    return new V(t.startLineNumber, t.endLineNumber);
  }
  /**
   * @param lineRanges An array of sorted line ranges.
   */
  static joinMany(t) {
    if (t.length === 0)
      return [];
    let n = new oe(t[0].slice());
    for (let r = 1; r < t.length; r++)
      n = n.getUnion(new oe(t[r].slice()));
    return n.ranges;
  }
  static ofLength(t, n) {
    return new V(t, t + n);
  }
  /**
   * @internal
   */
  static deserialize(t) {
    return new V(t[0], t[1]);
  }
  constructor(t, n) {
    if (t > n)
      throw new xe(`startLineNumber ${t} cannot be after endLineNumberExclusive ${n}`);
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
    return new V(this.startLineNumber + t, this.endLineNumberExclusive + t);
  }
  deltaLength(t) {
    return new V(this.startLineNumber, this.endLineNumberExclusive + t);
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
    return new V(Math.min(this.startLineNumber, t.startLineNumber), Math.max(this.endLineNumberExclusive, t.endLineNumberExclusive));
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
      return new V(n, r);
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
    return this.isEmpty ? null : new D(this.startLineNumber, 1, this.endLineNumberExclusive - 1, Number.MAX_SAFE_INTEGER);
  }
  toExclusiveRange() {
    return new D(this.startLineNumber, 1, this.endLineNumberExclusive, 1);
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
    return new T(this.startLineNumber - 1, this.endLineNumberExclusive - 1);
  }
}
class oe {
  constructor(t = []) {
    this._normalizedRanges = t;
  }
  get ranges() {
    return this._normalizedRanges;
  }
  addRange(t) {
    if (t.length === 0)
      return;
    const n = Wt(this._normalizedRanges, (s) => s.endLineNumberExclusive >= t.startLineNumber), r = Je(this._normalizedRanges, (s) => s.startLineNumber <= t.endLineNumberExclusive) + 1;
    if (n === r)
      this._normalizedRanges.splice(n, 0, t);
    else if (n === r - 1) {
      const s = this._normalizedRanges[n];
      this._normalizedRanges[n] = s.join(t);
    } else {
      const s = this._normalizedRanges[n].join(this._normalizedRanges[r - 1]).join(t);
      this._normalizedRanges.splice(n, r - n, s);
    }
  }
  contains(t) {
    const n = Xe(this._normalizedRanges, (r) => r.startLineNumber <= t);
    return !!n && n.endLineNumberExclusive > t;
  }
  getUnion(t) {
    if (this._normalizedRanges.length === 0)
      return t;
    if (t._normalizedRanges.length === 0)
      return this;
    const n = [];
    let r = 0, s = 0, i = null;
    for (; r < this._normalizedRanges.length || s < t._normalizedRanges.length; ) {
      let a = null;
      if (r < this._normalizedRanges.length && s < t._normalizedRanges.length) {
        const o = this._normalizedRanges[r], c = t._normalizedRanges[s];
        o.startLineNumber < c.startLineNumber ? (a = o, r++) : (a = c, s++);
      } else
        r < this._normalizedRanges.length ? (a = this._normalizedRanges[r], r++) : (a = t._normalizedRanges[s], s++);
      i === null ? i = a : i.endLineNumberExclusive >= a.startLineNumber ? i = new V(i.startLineNumber, Math.max(i.endLineNumberExclusive, a.endLineNumberExclusive)) : (n.push(i), i = a);
    }
    return i !== null && n.push(i), new oe(n);
  }
  /**
   * Subtracts all ranges in this set from `range` and returns the result.
   */
  subtractFrom(t) {
    const n = Wt(this._normalizedRanges, (a) => a.endLineNumberExclusive >= t.startLineNumber), r = Je(this._normalizedRanges, (a) => a.startLineNumber <= t.endLineNumberExclusive) + 1;
    if (n === r)
      return new oe([t]);
    const s = [];
    let i = t.startLineNumber;
    for (let a = n; a < r; a++) {
      const o = this._normalizedRanges[a];
      o.startLineNumber > i && s.push(new V(i, o.startLineNumber)), i = o.endLineNumberExclusive;
    }
    return i < t.endLineNumberExclusive && s.push(new V(i, t.endLineNumberExclusive)), new oe(s);
  }
  toString() {
    return this._normalizedRanges.map((t) => t.toString()).join(", ");
  }
  getIntersection(t) {
    const n = [];
    let r = 0, s = 0;
    for (; r < this._normalizedRanges.length && s < t._normalizedRanges.length; ) {
      const i = this._normalizedRanges[r], a = t._normalizedRanges[s], o = i.intersect(a);
      o && !o.isEmpty && n.push(o), i.endLineNumberExclusive < a.endLineNumberExclusive ? r++ : s++;
    }
    return new oe(n);
  }
  getWithDelta(t) {
    return new oe(this._normalizedRanges.map((n) => n.delta(t)));
  }
}
class Le {
  static inverse(t, n, r) {
    const s = [];
    let i = 1, a = 1;
    for (const c of t) {
      const u = new fe(new V(i, c.original.startLineNumber), new V(a, c.modified.startLineNumber), void 0);
      u.modified.isEmpty || s.push(u), i = c.original.endLineNumberExclusive, a = c.modified.endLineNumberExclusive;
    }
    const o = new fe(new V(i, n + 1), new V(a, r + 1), void 0);
    return o.modified.isEmpty || s.push(o), s;
  }
  constructor(t, n) {
    this.original = t, this.modified = n;
  }
  toString() {
    return `{${this.original.toString()}->${this.modified.toString()}}`;
  }
  flip() {
    return new Le(this.modified, this.original);
  }
  join(t) {
    return new Le(this.original.join(t.original), this.modified.join(t.modified));
  }
}
class fe extends Le {
  constructor(t, n, r) {
    super(t, n), this.innerChanges = r;
  }
  flip() {
    var t;
    return new fe(this.modified, this.original, (t = this.innerChanges) === null || t === void 0 ? void 0 : t.map((n) => n.flip()));
  }
}
class Ye {
  constructor(t, n) {
    this.originalRange = t, this.modifiedRange = n;
  }
  toString() {
    return `{${this.originalRange.toString()}->${this.modifiedRange.toString()}}`;
  }
  flip() {
    return new Ye(this.modifiedRange, this.originalRange);
  }
}
const Mi = 3;
class Ei {
  computeDiff(t, n, r) {
    var s;
    const a = new Fi(t, n, {
      maxComputationTime: r.maxComputationTimeMs,
      shouldIgnoreTrimWhitespace: r.ignoreTrimWhitespace,
      shouldComputeCharChanges: !0,
      shouldMakePrettyDiff: !0,
      shouldPostProcessCharChanges: !0
    }).computeDiff(), o = [];
    let c = null;
    for (const u of a.changes) {
      let h;
      u.originalEndLineNumber === 0 ? h = new V(u.originalStartLineNumber + 1, u.originalStartLineNumber + 1) : h = new V(u.originalStartLineNumber, u.originalEndLineNumber + 1);
      let f;
      u.modifiedEndLineNumber === 0 ? f = new V(u.modifiedStartLineNumber + 1, u.modifiedStartLineNumber + 1) : f = new V(u.modifiedStartLineNumber, u.modifiedEndLineNumber + 1);
      let m = new fe(h, f, (s = u.charChanges) === null || s === void 0 ? void 0 : s.map((d) => new Ye(new D(d.originalStartLineNumber, d.originalStartColumn, d.originalEndLineNumber, d.originalEndColumn), new D(d.modifiedStartLineNumber, d.modifiedStartColumn, d.modifiedEndLineNumber, d.modifiedEndColumn))));
      c && (c.modified.endLineNumberExclusive === m.modified.startLineNumber || c.original.endLineNumberExclusive === m.original.startLineNumber) && (m = new fe(c.original.join(m.original), c.modified.join(m.modified), c.innerChanges && m.innerChanges ? c.innerChanges.concat(m.innerChanges) : void 0), o.pop()), o.push(m), c = m;
    }
    return gt(() => Dr(o, (u, h) => h.original.startLineNumber - u.original.endLineNumberExclusive === h.modified.startLineNumber - u.modified.endLineNumberExclusive && // There has to be an unchanged line in between (otherwise both diffs should have been joined)
    u.original.endLineNumberExclusive < h.original.startLineNumber && u.modified.endLineNumberExclusive < h.modified.startLineNumber)), new ut(o, [], a.quitEarly);
  }
}
function Tr(e, t, n, r) {
  return new _e(e, t, n).ComputeDiff(r);
}
let ir = class {
  constructor(t) {
    const n = [], r = [];
    for (let s = 0, i = t.length; s < i; s++)
      n[s] = zt(t[s], 1), r[s] = $t(t[s], 1);
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
    const s = [], i = [], a = [];
    let o = 0;
    for (let c = n; c <= r; c++) {
      const u = this.lines[c], h = t ? this._startColumns[c] : 1, f = t ? this._endColumns[c] : u.length + 1;
      for (let m = h; m < f; m++)
        s[o] = u.charCodeAt(m - 1), i[o] = c + 1, a[o] = m, o++;
      !t && c < r && (s[o] = 10, i[o] = c + 1, a[o] = u.length + 1, o++);
    }
    return new ki(s, i, a);
  }
};
class ki {
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
class Ve {
  constructor(t, n, r, s, i, a, o, c) {
    this.originalStartLineNumber = t, this.originalStartColumn = n, this.originalEndLineNumber = r, this.originalEndColumn = s, this.modifiedStartLineNumber = i, this.modifiedStartColumn = a, this.modifiedEndLineNumber = o, this.modifiedEndColumn = c;
  }
  static createFromDiffChange(t, n, r) {
    const s = n.getStartLineNumber(t.originalStart), i = n.getStartColumn(t.originalStart), a = n.getEndLineNumber(t.originalStart + t.originalLength - 1), o = n.getEndColumn(t.originalStart + t.originalLength - 1), c = r.getStartLineNumber(t.modifiedStart), u = r.getStartColumn(t.modifiedStart), h = r.getEndLineNumber(t.modifiedStart + t.modifiedLength - 1), f = r.getEndColumn(t.modifiedStart + t.modifiedLength - 1);
    return new Ve(s, i, a, o, c, u, h, f);
  }
}
function Pi(e) {
  if (e.length <= 1)
    return e;
  const t = [e[0]];
  let n = t[0];
  for (let r = 1, s = e.length; r < s; r++) {
    const i = e[r], a = i.originalStart - (n.originalStart + n.originalLength), o = i.modifiedStart - (n.modifiedStart + n.modifiedLength);
    Math.min(a, o) < Mi ? (n.originalLength = i.originalStart + i.originalLength - n.originalStart, n.modifiedLength = i.modifiedStart + i.modifiedLength - n.modifiedStart) : (t.push(i), n = i);
  }
  return t;
}
class Oe {
  constructor(t, n, r, s, i) {
    this.originalStartLineNumber = t, this.originalEndLineNumber = n, this.modifiedStartLineNumber = r, this.modifiedEndLineNumber = s, this.charChanges = i;
  }
  static createFromDiffResult(t, n, r, s, i, a, o) {
    let c, u, h, f, m;
    if (n.originalLength === 0 ? (c = r.getStartLineNumber(n.originalStart) - 1, u = 0) : (c = r.getStartLineNumber(n.originalStart), u = r.getEndLineNumber(n.originalStart + n.originalLength - 1)), n.modifiedLength === 0 ? (h = s.getStartLineNumber(n.modifiedStart) - 1, f = 0) : (h = s.getStartLineNumber(n.modifiedStart), f = s.getEndLineNumber(n.modifiedStart + n.modifiedLength - 1)), a && n.originalLength > 0 && n.originalLength < 20 && n.modifiedLength > 0 && n.modifiedLength < 20 && i()) {
      const d = r.createCharSequence(t, n.originalStart, n.originalStart + n.originalLength - 1), g = s.createCharSequence(t, n.modifiedStart, n.modifiedStart + n.modifiedLength - 1);
      if (d.getElements().length > 0 && g.getElements().length > 0) {
        let b = Tr(d, g, i, !0).changes;
        o && (b = Pi(b)), m = [];
        for (let w = 0, L = b.length; w < L; w++)
          m.push(Ve.createFromDiffChange(b[w], d, g));
      }
    }
    return new Oe(c, u, h, f, m);
  }
}
class Fi {
  constructor(t, n, r) {
    this.shouldComputeCharChanges = r.shouldComputeCharChanges, this.shouldPostProcessCharChanges = r.shouldPostProcessCharChanges, this.shouldIgnoreTrimWhitespace = r.shouldIgnoreTrimWhitespace, this.shouldMakePrettyDiff = r.shouldMakePrettyDiff, this.originalLines = t, this.modifiedLines = n, this.original = new ir(t), this.modified = new ir(n), this.continueLineDiff = ar(r.maxComputationTime), this.continueCharDiff = ar(r.maxComputationTime === 0 ? 0 : Math.min(r.maxComputationTime, 5e3));
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
    const t = Tr(this.original, this.modified, this.continueLineDiff, this.shouldMakePrettyDiff), n = t.changes, r = t.quitEarly;
    if (this.shouldIgnoreTrimWhitespace) {
      const o = [];
      for (let c = 0, u = n.length; c < u; c++)
        o.push(Oe.createFromDiffResult(this.shouldIgnoreTrimWhitespace, n[c], this.original, this.modified, this.continueCharDiff, this.shouldComputeCharChanges, this.shouldPostProcessCharChanges));
      return {
        quitEarly: r,
        changes: o
      };
    }
    const s = [];
    let i = 0, a = 0;
    for (let o = -1, c = n.length; o < c; o++) {
      const u = o + 1 < c ? n[o + 1] : null, h = u ? u.originalStart : this.originalLines.length, f = u ? u.modifiedStart : this.modifiedLines.length;
      for (; i < h && a < f; ) {
        const m = this.originalLines[i], d = this.modifiedLines[a];
        if (m !== d) {
          {
            let g = zt(m, 1), b = zt(d, 1);
            for (; g > 1 && b > 1; ) {
              const w = m.charCodeAt(g - 2), L = d.charCodeAt(b - 2);
              if (w !== L)
                break;
              g--, b--;
            }
            (g > 1 || b > 1) && this._pushTrimWhitespaceCharChange(s, i + 1, 1, g, a + 1, 1, b);
          }
          {
            let g = $t(m, 1), b = $t(d, 1);
            const w = m.length + 1, L = d.length + 1;
            for (; g < w && b < L; ) {
              const _ = m.charCodeAt(g - 1), x = m.charCodeAt(b - 1);
              if (_ !== x)
                break;
              g++, b++;
            }
            (g < w || b < L) && this._pushTrimWhitespaceCharChange(s, i + 1, g, w, a + 1, b, L);
          }
        }
        i++, a++;
      }
      u && (s.push(Oe.createFromDiffResult(this.shouldIgnoreTrimWhitespace, u, this.original, this.modified, this.continueCharDiff, this.shouldComputeCharChanges, this.shouldPostProcessCharChanges)), i += u.originalLength, a += u.modifiedLength);
    }
    return {
      quitEarly: r,
      changes: s
    };
  }
  _pushTrimWhitespaceCharChange(t, n, r, s, i, a, o) {
    if (this._mergeTrimWhitespaceCharChange(t, n, r, s, i, a, o))
      return;
    let c;
    this.shouldComputeCharChanges && (c = [new Ve(n, r, n, s, i, a, i, o)]), t.push(new Oe(n, n, i, i, c));
  }
  _mergeTrimWhitespaceCharChange(t, n, r, s, i, a, o) {
    const c = t.length;
    if (c === 0)
      return !1;
    const u = t[c - 1];
    return u.originalEndLineNumber === 0 || u.modifiedEndLineNumber === 0 ? !1 : u.originalEndLineNumber === n && u.modifiedEndLineNumber === i ? (this.shouldComputeCharChanges && u.charChanges && u.charChanges.push(new Ve(n, r, n, s, i, a, i, o)), !0) : u.originalEndLineNumber + 1 === n && u.modifiedEndLineNumber + 1 === i ? (u.originalEndLineNumber = n, u.modifiedEndLineNumber = i, this.shouldComputeCharChanges && u.charChanges && u.charChanges.push(new Ve(n, r, n, s, i, a, i, o)), !0) : !1;
  }
}
function zt(e, t) {
  const n = ds(e);
  return n === -1 ? t : n + 1;
}
function $t(e, t) {
  const n = ms(e);
  return n === -1 ? t : n + 2;
}
function ar(e) {
  if (e === 0)
    return () => !0;
  const t = Date.now();
  return () => Date.now() - t < e;
}
class he {
  static trivial(t, n) {
    return new he([new O(T.ofLength(t.length), T.ofLength(n.length))], !1);
  }
  static trivialTimedOut(t, n) {
    return new he([new O(T.ofLength(t.length), T.ofLength(n.length))], !0);
  }
  constructor(t, n) {
    this.diffs = t, this.hitTimeout = n;
  }
}
class O {
  static invert(t, n) {
    const r = [];
    return Xs(t, (s, i) => {
      r.push(O.fromOffsetPairs(s ? s.getEndExclusives() : ue.zero, i ? i.getStarts() : new ue(n, (s ? s.seq2Range.endExclusive - s.seq1Range.endExclusive : 0) + n)));
    }), r;
  }
  static fromOffsetPairs(t, n) {
    return new O(new T(t.offset1, n.offset1), new T(t.offset2, n.offset2));
  }
  constructor(t, n) {
    this.seq1Range = t, this.seq2Range = n;
  }
  swap() {
    return new O(this.seq2Range, this.seq1Range);
  }
  toString() {
    return `${this.seq1Range} <-> ${this.seq2Range}`;
  }
  join(t) {
    return new O(this.seq1Range.join(t.seq1Range), this.seq2Range.join(t.seq2Range));
  }
  delta(t) {
    return t === 0 ? this : new O(this.seq1Range.delta(t), this.seq2Range.delta(t));
  }
  deltaStart(t) {
    return t === 0 ? this : new O(this.seq1Range.deltaStart(t), this.seq2Range.deltaStart(t));
  }
  deltaEnd(t) {
    return t === 0 ? this : new O(this.seq1Range.deltaEnd(t), this.seq2Range.deltaEnd(t));
  }
  intersect(t) {
    const n = this.seq1Range.intersect(t.seq1Range), r = this.seq2Range.intersect(t.seq2Range);
    if (!(!n || !r))
      return new O(n, r);
  }
  getStarts() {
    return new ue(this.seq1Range.start, this.seq2Range.start);
  }
  getEndExclusives() {
    return new ue(this.seq1Range.endExclusive, this.seq2Range.endExclusive);
  }
}
class ue {
  constructor(t, n) {
    this.offset1 = t, this.offset2 = n;
  }
  toString() {
    return `${this.offset1} <-> ${this.offset2}`;
  }
}
ue.zero = new ue(0, 0);
ue.max = new ue(Number.MAX_SAFE_INTEGER, Number.MAX_SAFE_INTEGER);
class Ze {
  isValid() {
    return !0;
  }
}
Ze.instance = new Ze();
class Di {
  constructor(t) {
    if (this.timeout = t, this.startTime = Date.now(), this.valid = !0, t <= 0)
      throw new xe("timeout must be positive");
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
class Nt {
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
function Ot(e) {
  return e === 32 || e === 9;
}
class Ue {
  static getKey(t) {
    let n = this.chrKeys.get(t);
    return n === void 0 && (n = this.chrKeys.size, this.chrKeys.set(t, n)), n;
  }
  constructor(t, n, r) {
    this.range = t, this.lines = n, this.source = r, this.histogram = [];
    let s = 0;
    for (let i = t.startLineNumber - 1; i < t.endLineNumberExclusive - 1; i++) {
      const a = n[i];
      for (let c = 0; c < a.length; c++) {
        s++;
        const u = a[c], h = Ue.getKey(u);
        this.histogram[h] = (this.histogram[h] || 0) + 1;
      }
      s++;
      const o = Ue.getKey(`
`);
      this.histogram[o] = (this.histogram[o] || 0) + 1;
    }
    this.totalCount = s;
  }
  computeSimilarity(t) {
    var n, r;
    let s = 0;
    const i = Math.max(this.histogram.length, t.histogram.length);
    for (let a = 0; a < i; a++)
      s += Math.abs(((n = this.histogram[a]) !== null && n !== void 0 ? n : 0) - ((r = t.histogram[a]) !== null && r !== void 0 ? r : 0));
    return 1 - s / (this.totalCount + t.totalCount);
  }
}
Ue.chrKeys = /* @__PURE__ */ new Map();
class Ti {
  compute(t, n, r = Ze.instance, s) {
    if (t.length === 0 || n.length === 0)
      return he.trivial(t, n);
    const i = new Nt(t.length, n.length), a = new Nt(t.length, n.length), o = new Nt(t.length, n.length);
    for (let g = 0; g < t.length; g++)
      for (let b = 0; b < n.length; b++) {
        if (!r.isValid())
          return he.trivialTimedOut(t, n);
        const w = g === 0 ? 0 : i.get(g - 1, b), L = b === 0 ? 0 : i.get(g, b - 1);
        let _;
        t.getElement(g) === n.getElement(b) ? (g === 0 || b === 0 ? _ = 0 : _ = i.get(g - 1, b - 1), g > 0 && b > 0 && a.get(g - 1, b - 1) === 3 && (_ += o.get(g - 1, b - 1)), _ += s ? s(g, b) : 1) : _ = -1;
        const x = Math.max(w, L, _);
        if (x === _) {
          const S = g > 0 && b > 0 ? o.get(g - 1, b - 1) : 0;
          o.set(g, b, S + 1), a.set(g, b, 3);
        } else
          x === w ? (o.set(g, b, 0), a.set(g, b, 1)) : x === L && (o.set(g, b, 0), a.set(g, b, 2));
        i.set(g, b, x);
      }
    const c = [];
    let u = t.length, h = n.length;
    function f(g, b) {
      (g + 1 !== u || b + 1 !== h) && c.push(new O(new T(g + 1, u), new T(b + 1, h))), u = g, h = b;
    }
    let m = t.length - 1, d = n.length - 1;
    for (; m >= 0 && d >= 0; )
      a.get(m, d) === 3 ? (f(m, d), m--, d--) : a.get(m, d) === 1 ? m-- : d--;
    return f(-1, -1), c.reverse(), new he(c, !1);
  }
}
class Vr {
  compute(t, n, r = Ze.instance) {
    if (t.length === 0 || n.length === 0)
      return he.trivial(t, n);
    const s = t, i = n;
    function a(b, w) {
      for (; b < s.length && w < i.length && s.getElement(b) === i.getElement(w); )
        b++, w++;
      return b;
    }
    let o = 0;
    const c = new Vi();
    c.set(0, a(0, 0));
    const u = new Bi();
    u.set(0, c.get(0) === 0 ? null : new lr(null, 0, 0, c.get(0)));
    let h = 0;
    e:
      for (; ; ) {
        if (o++, !r.isValid())
          return he.trivialTimedOut(s, i);
        const b = -Math.min(o, i.length + o % 2), w = Math.min(o, s.length + o % 2);
        for (h = b; h <= w; h += 2) {
          const L = h === w ? -1 : c.get(h + 1), _ = h === b ? -1 : c.get(h - 1) + 1, x = Math.min(Math.max(L, _), s.length), S = x - h;
          if (x > s.length || S > i.length)
            continue;
          const N = a(x, S);
          c.set(h, N);
          const R = x === L ? u.get(h + 1) : u.get(h - 1);
          if (u.set(h, N !== x ? new lr(R, x, S, N - x) : R), c.get(h) === s.length && c.get(h) - h === i.length)
            break e;
        }
      }
    let f = u.get(h);
    const m = [];
    let d = s.length, g = i.length;
    for (; ; ) {
      const b = f ? f.x + f.length : 0, w = f ? f.y + f.length : 0;
      if ((b !== d || w !== g) && m.push(new O(new T(b, d), new T(w, g))), !f)
        break;
      d = f.x, g = f.y, f = f.prev;
    }
    return m.reverse(), new he(m, !1);
  }
}
class lr {
  constructor(t, n, r, s) {
    this.prev = t, this.x = n, this.y = r, this.length = s;
  }
}
class Vi {
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
class Bi {
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
class qi {
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
class bt {
  constructor(t, n, r) {
    this.lines = t, this.considerWhitespaceChanges = r, this.elements = [], this.firstCharOffsetByLine = [], this.additionalOffsetByLine = [];
    let s = !1;
    n.start > 0 && n.endExclusive >= t.length && (n = new T(n.start - 1, n.endExclusive), s = !0), this.lineRange = n, this.firstCharOffsetByLine[0] = 0;
    for (let i = this.lineRange.start; i < this.lineRange.endExclusive; i++) {
      let a = t[i], o = 0;
      if (s)
        o = a.length, a = "", s = !1;
      else if (!r) {
        const c = a.trimStart();
        o = a.length - c.length, a = c.trimEnd();
      }
      this.additionalOffsetByLine.push(o);
      for (let c = 0; c < a.length; c++)
        this.elements.push(a.charCodeAt(c));
      i < t.length - 1 && (this.elements.push(`
`.charCodeAt(0)), this.firstCharOffsetByLine[i - this.lineRange.start + 1] = this.elements.length);
    }
    this.additionalOffsetByLine.push(0);
  }
  toString() {
    return `Slice: "${this.text}"`;
  }
  get text() {
    return this.getText(new T(0, this.length));
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
    const n = ur(t > 0 ? this.elements[t - 1] : -1), r = ur(t < this.elements.length ? this.elements[t] : -1);
    if (n === 6 && r === 7)
      return 0;
    let s = 0;
    return n !== r && (s += 10, n === 0 && r === 1 && (s += 1)), s += or(n), s += or(r), s;
  }
  translateOffset(t) {
    if (this.lineRange.isEmpty)
      return new J(this.lineRange.start + 1, 1);
    const n = Je(this.firstCharOffsetByLine, (r) => r <= t);
    return new J(this.lineRange.start + n + 1, t - this.firstCharOffsetByLine[n] + this.additionalOffsetByLine[n] + 1);
  }
  translateRange(t) {
    return D.fromPositions(this.translateOffset(t.start), this.translateOffset(t.endExclusive));
  }
  /**
   * Finds the word that contains the character at the given offset
   */
  findWordContaining(t) {
    if (t < 0 || t >= this.elements.length || !St(this.elements[t]))
      return;
    let n = t;
    for (; n > 0 && St(this.elements[n - 1]); )
      n--;
    let r = t;
    for (; r < this.elements.length && St(this.elements[r]); )
      r++;
    return new T(n, r);
  }
  countLinesIn(t) {
    return this.translateOffset(t.endExclusive).lineNumber - this.translateOffset(t.start).lineNumber;
  }
  isStronglyEqual(t, n) {
    return this.elements[t] === this.elements[n];
  }
  extendToFullLines(t) {
    var n, r;
    const s = (n = Xe(this.firstCharOffsetByLine, (a) => a <= t.start)) !== null && n !== void 0 ? n : 0, i = (r = yi(this.firstCharOffsetByLine, (a) => t.endExclusive <= a)) !== null && r !== void 0 ? r : this.elements.length;
    return new T(s, i);
  }
}
function St(e) {
  return e >= 97 && e <= 122 || e >= 65 && e <= 90 || e >= 48 && e <= 57;
}
const Ii = {
  0: 0,
  1: 0,
  2: 0,
  3: 10,
  4: 2,
  5: 3,
  6: 10,
  7: 10
};
function or(e) {
  return Ii[e];
}
function ur(e) {
  return e === 10 ? 7 : e === 13 ? 6 : Ot(e) ? 5 : e >= 97 && e <= 122 ? 0 : e >= 65 && e <= 90 ? 1 : e >= 48 && e <= 57 ? 2 : e === -1 ? 3 : 4;
}
function Ui(e, t, n, r, s, i) {
  let { moves: a, excludedChanges: o } = Hi(e, t, n, i);
  if (!i.isValid())
    return [];
  const c = e.filter((h) => !o.has(h)), u = Wi(c, r, s, t, n, i);
  return Ys(a, u), a = zi(a), a = a.filter((h) => h.original.toOffsetRange().slice(t).map((m) => m.trim()).join(`
`).length >= 10), a = $i(e, a), a;
}
function Hi(e, t, n, r) {
  const s = [], i = e.filter((c) => c.modified.isEmpty && c.original.length >= 3).map((c) => new Ue(c.original, t, c)), a = new Set(e.filter((c) => c.original.isEmpty && c.modified.length >= 3).map((c) => new Ue(c.modified, n, c))), o = /* @__PURE__ */ new Set();
  for (const c of i) {
    let u = -1, h;
    for (const f of a) {
      const m = c.computeSimilarity(f);
      m > u && (u = m, h = f);
    }
    if (u > 0.9 && h && (a.delete(h), s.push(new Le(c.range, h.range)), o.add(c.source), o.add(h.source)), !r.isValid())
      return { moves: s, excludedChanges: o };
  }
  return { moves: s, excludedChanges: o };
}
function Wi(e, t, n, r, s, i) {
  const a = [], o = new qi();
  for (const m of e)
    for (let d = m.original.startLineNumber; d < m.original.endLineNumberExclusive - 2; d++) {
      const g = `${t[d - 1]}:${t[d + 1 - 1]}:${t[d + 2 - 1]}`;
      o.add(g, { range: new V(d, d + 3) });
    }
  const c = [];
  e.sort(it((m) => m.modified.startLineNumber, at));
  for (const m of e) {
    let d = [];
    for (let g = m.modified.startLineNumber; g < m.modified.endLineNumberExclusive - 2; g++) {
      const b = `${n[g - 1]}:${n[g + 1 - 1]}:${n[g + 2 - 1]}`, w = new V(g, g + 3), L = [];
      o.forEach(b, ({ range: _ }) => {
        for (const S of d)
          if (S.originalLineRange.endLineNumberExclusive + 1 === _.endLineNumberExclusive && S.modifiedLineRange.endLineNumberExclusive + 1 === w.endLineNumberExclusive) {
            S.originalLineRange = new V(S.originalLineRange.startLineNumber, _.endLineNumberExclusive), S.modifiedLineRange = new V(S.modifiedLineRange.startLineNumber, w.endLineNumberExclusive), L.push(S);
            return;
          }
        const x = {
          modifiedLineRange: w,
          originalLineRange: _
        };
        c.push(x), L.push(x);
      }), d = L;
    }
    if (!i.isValid())
      return [];
  }
  c.sort(Zs(it((m) => m.modifiedLineRange.length, at)));
  const u = new oe(), h = new oe();
  for (const m of c) {
    const d = m.modifiedLineRange.startLineNumber - m.originalLineRange.startLineNumber, g = u.subtractFrom(m.modifiedLineRange), b = h.subtractFrom(m.originalLineRange).getWithDelta(d), w = g.getIntersection(b);
    for (const L of w.ranges) {
      if (L.length < 3)
        continue;
      const _ = L, x = L.delta(-d);
      a.push(new Le(x, _)), u.addRange(_), h.addRange(x);
    }
  }
  a.sort(it((m) => m.original.startLineNumber, at));
  const f = new et(e);
  for (let m = 0; m < a.length; m++) {
    const d = a[m], g = f.findLastMonotonous((R) => R.original.startLineNumber <= d.original.startLineNumber), b = Xe(e, (R) => R.modified.startLineNumber <= d.modified.startLineNumber), w = Math.max(d.original.startLineNumber - g.original.startLineNumber, d.modified.startLineNumber - b.modified.startLineNumber), L = f.findLastMonotonous((R) => R.original.startLineNumber < d.original.endLineNumberExclusive), _ = Xe(e, (R) => R.modified.startLineNumber < d.modified.endLineNumberExclusive), x = Math.max(L.original.endLineNumberExclusive - d.original.endLineNumberExclusive, _.modified.endLineNumberExclusive - d.modified.endLineNumberExclusive);
    let S;
    for (S = 0; S < w; S++) {
      const R = d.original.startLineNumber - S - 1, M = d.modified.startLineNumber - S - 1;
      if (R > r.length || M > s.length || u.contains(M) || h.contains(R) || !cr(r[R - 1], s[M - 1], i))
        break;
    }
    S > 0 && (h.addRange(new V(d.original.startLineNumber - S, d.original.startLineNumber)), u.addRange(new V(d.modified.startLineNumber - S, d.modified.startLineNumber)));
    let N;
    for (N = 0; N < x; N++) {
      const R = d.original.endLineNumberExclusive + N, M = d.modified.endLineNumberExclusive + N;
      if (R > r.length || M > s.length || u.contains(M) || h.contains(R) || !cr(r[R - 1], s[M - 1], i))
        break;
    }
    N > 0 && (h.addRange(new V(d.original.endLineNumberExclusive, d.original.endLineNumberExclusive + N)), u.addRange(new V(d.modified.endLineNumberExclusive, d.modified.endLineNumberExclusive + N))), (S > 0 || N > 0) && (a[m] = new Le(new V(d.original.startLineNumber - S, d.original.endLineNumberExclusive + N), new V(d.modified.startLineNumber - S, d.modified.endLineNumberExclusive + N)));
  }
  return a;
}
function cr(e, t, n) {
  if (e.trim() === t.trim())
    return !0;
  if (e.length > 300 && t.length > 300)
    return !1;
  const s = new Vr().compute(new bt([e], new T(0, 1), !1), new bt([t], new T(0, 1), !1), n);
  let i = 0;
  const a = O.invert(s.diffs, e.length);
  for (const h of a)
    h.seq1Range.forEach((f) => {
      Ot(e.charCodeAt(f)) || i++;
    });
  function o(h) {
    let f = 0;
    for (let m = 0; m < e.length; m++)
      Ot(h.charCodeAt(m)) || f++;
    return f;
  }
  const c = o(e.length > t.length ? e : t);
  return i / c > 0.6 && c > 10;
}
function zi(e) {
  if (e.length === 0)
    return e;
  e.sort(it((n) => n.original.startLineNumber, at));
  const t = [e[0]];
  for (let n = 1; n < e.length; n++) {
    const r = t[t.length - 1], s = e[n], i = s.original.startLineNumber - r.original.endLineNumberExclusive, a = s.modified.startLineNumber - r.modified.endLineNumberExclusive;
    if (i >= 0 && a >= 0 && i + a <= 2) {
      t[t.length - 1] = r.join(s);
      continue;
    }
    t.push(s);
  }
  return t;
}
function $i(e, t) {
  const n = new et(e);
  return t = t.filter((r) => {
    const s = n.findLastMonotonous((o) => o.original.endLineNumberExclusive < r.original.endLineNumberExclusive) || new Le(new V(1, 1), new V(1, 1)), i = Xe(e, (o) => o.modified.endLineNumberExclusive < r.modified.endLineNumberExclusive);
    return s !== i;
  }), t;
}
function hr(e, t, n) {
  let r = n;
  return r = Oi(e, t, r), r = Gi(e, t, r), r;
}
function Oi(e, t, n) {
  if (n.length === 0)
    return n;
  const r = [];
  r.push(n[0]);
  for (let i = 1; i < n.length; i++) {
    const a = r[r.length - 1];
    let o = n[i];
    if (o.seq1Range.isEmpty || o.seq2Range.isEmpty) {
      const c = o.seq1Range.start - a.seq1Range.endExclusive;
      let u;
      for (u = 1; u <= c && !(e.getElement(o.seq1Range.start - u) !== e.getElement(o.seq1Range.endExclusive - u) || t.getElement(o.seq2Range.start - u) !== t.getElement(o.seq2Range.endExclusive - u)); u++)
        ;
      if (u--, u === c) {
        r[r.length - 1] = new O(new T(a.seq1Range.start, o.seq1Range.endExclusive - c), new T(a.seq2Range.start, o.seq2Range.endExclusive - c));
        continue;
      }
      o = o.delta(-u);
    }
    r.push(o);
  }
  const s = [];
  for (let i = 0; i < r.length - 1; i++) {
    const a = r[i + 1];
    let o = r[i];
    if (o.seq1Range.isEmpty || o.seq2Range.isEmpty) {
      const c = a.seq1Range.start - o.seq1Range.endExclusive;
      let u;
      for (u = 0; u < c && !(!e.isStronglyEqual(o.seq1Range.start + u, o.seq1Range.endExclusive + u) || !t.isStronglyEqual(o.seq2Range.start + u, o.seq2Range.endExclusive + u)); u++)
        ;
      if (u === c) {
        r[i + 1] = new O(new T(o.seq1Range.start + c, a.seq1Range.endExclusive), new T(o.seq2Range.start + c, a.seq2Range.endExclusive));
        continue;
      }
      u > 0 && (o = o.delta(u));
    }
    s.push(o);
  }
  return r.length > 0 && s.push(r[r.length - 1]), s;
}
function Gi(e, t, n) {
  if (!e.getBoundaryScore || !t.getBoundaryScore)
    return n;
  for (let r = 0; r < n.length; r++) {
    const s = r > 0 ? n[r - 1] : void 0, i = n[r], a = r + 1 < n.length ? n[r + 1] : void 0, o = new T(s ? s.seq1Range.start + 1 : 0, a ? a.seq1Range.endExclusive - 1 : e.length), c = new T(s ? s.seq2Range.start + 1 : 0, a ? a.seq2Range.endExclusive - 1 : t.length);
    i.seq1Range.isEmpty ? n[r] = fr(i, e, t, o, c) : i.seq2Range.isEmpty && (n[r] = fr(i.swap(), t, e, c, o).swap());
  }
  return n;
}
function fr(e, t, n, r, s) {
  let a = 1;
  for (; e.seq1Range.start - a >= r.start && e.seq2Range.start - a >= s.start && n.isStronglyEqual(e.seq2Range.start - a, e.seq2Range.endExclusive - a) && a < 100; )
    a++;
  a--;
  let o = 0;
  for (; e.seq1Range.start + o < r.endExclusive && e.seq2Range.endExclusive + o < s.endExclusive && n.isStronglyEqual(e.seq2Range.start + o, e.seq2Range.endExclusive + o) && o < 100; )
    o++;
  if (a === 0 && o === 0)
    return e;
  let c = 0, u = -1;
  for (let h = -a; h <= o; h++) {
    const f = e.seq2Range.start + h, m = e.seq2Range.endExclusive + h, d = e.seq1Range.start + h, g = t.getBoundaryScore(d) + n.getBoundaryScore(f) + n.getBoundaryScore(m);
    g > u && (u = g, c = h);
  }
  return e.delta(c);
}
function ji(e, t, n) {
  const r = [];
  for (const s of n) {
    const i = r[r.length - 1];
    if (!i) {
      r.push(s);
      continue;
    }
    s.seq1Range.start - i.seq1Range.endExclusive <= 2 || s.seq2Range.start - i.seq2Range.endExclusive <= 2 ? r[r.length - 1] = new O(i.seq1Range.join(s.seq1Range), i.seq2Range.join(s.seq2Range)) : r.push(s);
  }
  return r;
}
function Qi(e, t, n) {
  const r = [];
  let s;
  function i() {
    if (!s)
      return;
    const o = s.s1Range.length - s.deleted;
    s.s2Range.length - s.added, Math.max(s.deleted, s.added) + (s.count - 1) > o && r.push(new O(s.s1Range, s.s2Range)), s = void 0;
  }
  for (const o of n) {
    let c = function(d, g) {
      var b, w, L, _;
      if (!s || !s.s1Range.containsRange(d) || !s.s2Range.containsRange(g))
        if (s && !(s.s1Range.endExclusive < d.start && s.s2Range.endExclusive < g.start)) {
          const N = T.tryCreate(s.s1Range.endExclusive, d.start), R = T.tryCreate(s.s2Range.endExclusive, g.start);
          s.deleted += (b = N == null ? void 0 : N.length) !== null && b !== void 0 ? b : 0, s.added += (w = R == null ? void 0 : R.length) !== null && w !== void 0 ? w : 0, s.s1Range = s.s1Range.join(d), s.s2Range = s.s2Range.join(g);
        } else
          i(), s = { added: 0, deleted: 0, count: 0, s1Range: d, s2Range: g };
      const x = d.intersect(o.seq1Range), S = g.intersect(o.seq2Range);
      s.count++, s.deleted += (L = x == null ? void 0 : x.length) !== null && L !== void 0 ? L : 0, s.added += (_ = S == null ? void 0 : S.length) !== null && _ !== void 0 ? _ : 0;
    };
    const u = e.findWordContaining(o.seq1Range.start - 1), h = t.findWordContaining(o.seq2Range.start - 1), f = e.findWordContaining(o.seq1Range.endExclusive), m = t.findWordContaining(o.seq2Range.endExclusive);
    u && f && h && m && u.equals(f) && h.equals(m) ? c(u, h) : (u && h && c(u, h), f && m && c(f, m));
  }
  return i(), Xi(n, r);
}
function Xi(e, t) {
  const n = [];
  for (; e.length > 0 || t.length > 0; ) {
    const r = e[0], s = t[0];
    let i;
    r && (!s || r.seq1Range.start < s.seq1Range.start) ? i = e.shift() : i = t.shift(), n.length > 0 && n[n.length - 1].seq1Range.endExclusive >= i.seq1Range.start ? n[n.length - 1] = n[n.length - 1].join(i) : n.push(i);
  }
  return n;
}
function Ji(e, t, n) {
  let r = n;
  if (r.length === 0)
    return r;
  let s = 0, i;
  do {
    i = !1;
    const a = [
      r[0]
    ];
    for (let o = 1; o < r.length; o++) {
      let h = function(m, d) {
        const g = new T(u.seq1Range.endExclusive, c.seq1Range.start);
        return e.getText(g).replace(/\s/g, "").length <= 4 && (m.seq1Range.length + m.seq2Range.length > 5 || d.seq1Range.length + d.seq2Range.length > 5);
      };
      const c = r[o], u = a[a.length - 1];
      h(u, c) ? (i = !0, a[a.length - 1] = a[a.length - 1].join(c)) : a.push(c);
    }
    r = a;
  } while (s++ < 10 && i);
  return r;
}
function Yi(e, t, n) {
  let r = n;
  if (r.length === 0)
    return r;
  let s = 0, i;
  do {
    i = !1;
    const o = [
      r[0]
    ];
    for (let c = 1; c < r.length; c++) {
      let f = function(d, g) {
        const b = new T(h.seq1Range.endExclusive, u.seq1Range.start);
        if (e.countLinesIn(b) > 5 || b.length > 500)
          return !1;
        const L = e.getText(b).trim();
        if (L.length > 20 || L.split(/\r\n|\r|\n/).length > 1)
          return !1;
        const _ = e.countLinesIn(d.seq1Range), x = d.seq1Range.length, S = t.countLinesIn(d.seq2Range), N = d.seq2Range.length, R = e.countLinesIn(g.seq1Range), M = g.seq1Range.length, E = t.countLinesIn(g.seq2Range), I = g.seq2Range.length, G = 2 * 40 + 50;
        function B(p) {
          return Math.min(p, G);
        }
        return Math.pow(Math.pow(B(_ * 40 + x), 1.5) + Math.pow(B(S * 40 + N), 1.5), 1.5) + Math.pow(Math.pow(B(R * 40 + M), 1.5) + Math.pow(B(E * 40 + I), 1.5), 1.5) > Math.pow(Math.pow(G, 1.5), 1.5) * 1.3;
      };
      const u = r[c], h = o[o.length - 1];
      f(h, u) ? (i = !0, o[o.length - 1] = o[o.length - 1].join(u)) : o.push(u);
    }
    r = o;
  } while (s++ < 10 && i);
  const a = [];
  return Js(r, (o, c, u) => {
    let h = c;
    function f(L) {
      return L.length > 0 && L.trim().length <= 3 && c.seq1Range.length + c.seq2Range.length > 100;
    }
    const m = e.extendToFullLines(c.seq1Range), d = e.getText(new T(m.start, c.seq1Range.start));
    f(d) && (h = h.deltaStart(-d.length));
    const g = e.getText(new T(c.seq1Range.endExclusive, m.endExclusive));
    f(g) && (h = h.deltaEnd(g.length));
    const b = O.fromOffsetPairs(o ? o.getEndExclusives() : ue.zero, u ? u.getStarts() : ue.max), w = h.intersect(b);
    a.push(w);
  }), a;
}
class dr {
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
    const n = t === 0 ? 0 : mr(this.lines[t - 1]), r = t === this.lines.length ? 0 : mr(this.lines[t]);
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
function mr(e) {
  let t = 0;
  for (; t < e.length && (e.charCodeAt(t) === 32 || e.charCodeAt(t) === 9); )
    t++;
  return t;
}
class Zi {
  constructor() {
    this.dynamicProgrammingDiffing = new Ti(), this.myersDiffingAlgorithm = new Vr();
  }
  computeDiff(t, n, r) {
    if (t.length <= 1 && js(t, n, (N, R) => N === R))
      return new ut([], [], !1);
    if (t.length === 1 && t[0].length === 0 || n.length === 1 && n[0].length === 0)
      return new ut([
        new fe(new V(1, t.length + 1), new V(1, n.length + 1), [
          new Ye(new D(1, 1, t.length, t[0].length + 1), new D(1, 1, n.length, n[0].length + 1))
        ])
      ], [], !1);
    const s = r.maxComputationTimeMs === 0 ? Ze.instance : new Di(r.maxComputationTimeMs), i = !r.ignoreTrimWhitespace, a = /* @__PURE__ */ new Map();
    function o(N) {
      let R = a.get(N);
      return R === void 0 && (R = a.size, a.set(N, R)), R;
    }
    const c = t.map((N) => o(N.trim())), u = n.map((N) => o(N.trim())), h = new dr(c, t), f = new dr(u, n), m = (() => h.length + f.length < 1700 ? this.dynamicProgrammingDiffing.compute(h, f, s, (N, R) => t[N] === n[R] ? n[R].length === 0 ? 0.1 : 1 + Math.log(1 + n[R].length) : 0.99) : this.myersDiffingAlgorithm.compute(h, f))();
    let d = m.diffs, g = m.hitTimeout;
    d = hr(h, f, d), d = Ji(h, f, d);
    const b = [], w = (N) => {
      if (i)
        for (let R = 0; R < N; R++) {
          const M = L + R, E = _ + R;
          if (t[M] !== n[E]) {
            const I = this.refineDiff(t, n, new O(new T(M, M + 1), new T(E, E + 1)), s, i);
            for (const G of I.mappings)
              b.push(G);
            I.hitTimeout && (g = !0);
          }
        }
    };
    let L = 0, _ = 0;
    for (const N of d) {
      gt(() => N.seq1Range.start - L === N.seq2Range.start - _);
      const R = N.seq1Range.start - L;
      w(R), L = N.seq1Range.endExclusive, _ = N.seq2Range.endExclusive;
      const M = this.refineDiff(t, n, N, s, i);
      M.hitTimeout && (g = !0);
      for (const E of M.mappings)
        b.push(E);
    }
    w(t.length - L);
    const x = gr(b, t, n);
    let S = [];
    return r.computeMoves && (S = this.computeMoves(x, t, n, c, u, s, i)), gt(() => {
      function N(M, E) {
        if (M.lineNumber < 1 || M.lineNumber > E.length)
          return !1;
        const I = E[M.lineNumber - 1];
        return !(M.column < 1 || M.column > I.length + 1);
      }
      function R(M, E) {
        return !(M.startLineNumber < 1 || M.startLineNumber > E.length + 1 || M.endLineNumberExclusive < 1 || M.endLineNumberExclusive > E.length + 1);
      }
      for (const M of x) {
        if (!M.innerChanges)
          return !1;
        for (const E of M.innerChanges)
          if (!(N(E.modifiedRange.getStartPosition(), n) && N(E.modifiedRange.getEndPosition(), n) && N(E.originalRange.getStartPosition(), t) && N(E.originalRange.getEndPosition(), t)))
            return !1;
        if (!R(M.modified, n) || !R(M.original, t))
          return !1;
      }
      return !0;
    }), new ut(x, S, g);
  }
  computeMoves(t, n, r, s, i, a, o) {
    return Ui(t, n, r, s, i, a).map((h) => {
      const f = this.refineDiff(n, r, new O(h.original.toOffsetRange(), h.modified.toOffsetRange()), a, o), m = gr(f.mappings, n, r, !0);
      return new Ri(h, m);
    });
  }
  refineDiff(t, n, r, s, i) {
    const a = new bt(t, r.seq1Range, i), o = new bt(n, r.seq2Range, i), c = a.length + o.length < 500 ? this.dynamicProgrammingDiffing.compute(a, o, s) : this.myersDiffingAlgorithm.compute(a, o, s);
    let u = c.diffs;
    return u = hr(a, o, u), u = Qi(a, o, u), u = ji(a, o, u), u = Yi(a, o, u), {
      mappings: u.map((f) => new Ye(a.translateRange(f.seq1Range), o.translateRange(f.seq2Range))),
      hitTimeout: c.hitTimeout
    };
  }
}
function gr(e, t, n, r = !1) {
  const s = [];
  for (const i of Qs(e.map((a) => Ki(a, t, n)), (a, o) => a.original.overlapOrTouch(o.original) || a.modified.overlapOrTouch(o.modified))) {
    const a = i[0], o = i[i.length - 1];
    s.push(new fe(a.original.join(o.original), a.modified.join(o.modified), i.map((c) => c.innerChanges[0])));
  }
  return gt(() => !r && s.length > 0 && s[0].original.startLineNumber !== s[0].modified.startLineNumber ? !1 : Dr(s, (i, a) => a.original.startLineNumber - i.original.endLineNumberExclusive === a.modified.startLineNumber - i.modified.endLineNumberExclusive && // There has to be an unchanged line in between (otherwise both diffs should have been joined)
  i.original.endLineNumberExclusive < a.original.startLineNumber && i.modified.endLineNumberExclusive < a.modified.startLineNumber)), s;
}
function Ki(e, t, n) {
  let r = 0, s = 0;
  e.modifiedRange.endColumn === 1 && e.originalRange.endColumn === 1 && e.originalRange.startLineNumber + r <= e.originalRange.endLineNumber && e.modifiedRange.startLineNumber + r <= e.modifiedRange.endLineNumber && (s = -1), e.modifiedRange.startColumn - 1 >= n[e.modifiedRange.startLineNumber - 1].length && e.originalRange.startColumn - 1 >= t[e.originalRange.startLineNumber - 1].length && e.originalRange.startLineNumber <= e.originalRange.endLineNumber + s && e.modifiedRange.startLineNumber <= e.modifiedRange.endLineNumber + s && (r = 1);
  const i = new V(e.originalRange.startLineNumber + r, e.originalRange.endLineNumber + 1 + s), a = new V(e.modifiedRange.startLineNumber + r, e.modifiedRange.endLineNumber + 1 + s);
  return new fe(i, a, [e]);
}
const br = {
  getLegacy: () => new Ei(),
  getDefault: () => new Zi()
};
function we(e, t) {
  const n = Math.pow(10, t);
  return Math.round(e * n) / n;
}
class j {
  constructor(t, n, r, s = 1) {
    this._rgbaBrand = void 0, this.r = Math.min(255, Math.max(0, t)) | 0, this.g = Math.min(255, Math.max(0, n)) | 0, this.b = Math.min(255, Math.max(0, r)) | 0, this.a = we(Math.max(Math.min(1, s), 0), 3);
  }
  static equals(t, n) {
    return t.r === n.r && t.g === n.g && t.b === n.b && t.a === n.a;
  }
}
class re {
  constructor(t, n, r, s) {
    this._hslaBrand = void 0, this.h = Math.max(Math.min(360, t), 0) | 0, this.s = we(Math.max(Math.min(1, n), 0), 3), this.l = we(Math.max(Math.min(1, r), 0), 3), this.a = we(Math.max(Math.min(1, s), 0), 3);
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
    const n = t.r / 255, r = t.g / 255, s = t.b / 255, i = t.a, a = Math.max(n, r, s), o = Math.min(n, r, s);
    let c = 0, u = 0;
    const h = (o + a) / 2, f = a - o;
    if (f > 0) {
      switch (u = Math.min(h <= 0.5 ? f / (2 * h) : f / (2 - 2 * h), 1), a) {
        case n:
          c = (r - s) / f + (r < s ? 6 : 0);
          break;
        case r:
          c = (s - n) / f + 2;
          break;
        case s:
          c = (n - r) / f + 4;
          break;
      }
      c *= 60, c = Math.round(c);
    }
    return new re(c, u, h, i);
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
    const n = t.h / 360, { s: r, l: s, a: i } = t;
    let a, o, c;
    if (r === 0)
      a = o = c = s;
    else {
      const u = s < 0.5 ? s * (1 + r) : s + r - s * r, h = 2 * s - u;
      a = re._hue2rgb(h, u, n + 1 / 3), o = re._hue2rgb(h, u, n), c = re._hue2rgb(h, u, n - 1 / 3);
    }
    return new j(Math.round(a * 255), Math.round(o * 255), Math.round(c * 255), i);
  }
}
class De {
  constructor(t, n, r, s) {
    this._hsvaBrand = void 0, this.h = Math.max(Math.min(360, t), 0) | 0, this.s = we(Math.max(Math.min(1, n), 0), 3), this.v = we(Math.max(Math.min(1, r), 0), 3), this.a = we(Math.max(Math.min(1, s), 0), 3);
  }
  static equals(t, n) {
    return t.h === n.h && t.s === n.s && t.v === n.v && t.a === n.a;
  }
  // from http://www.rapidtables.com/convert/color/rgb-to-hsv.htm
  static fromRGBA(t) {
    const n = t.r / 255, r = t.g / 255, s = t.b / 255, i = Math.max(n, r, s), a = Math.min(n, r, s), o = i - a, c = i === 0 ? 0 : o / i;
    let u;
    return o === 0 ? u = 0 : i === n ? u = ((r - s) / o % 6 + 6) % 6 : i === r ? u = (s - n) / o + 2 : u = (n - r) / o + 4, new De(Math.round(u * 60), c, i, t.a);
  }
  // from http://www.rapidtables.com/convert/color/hsv-to-rgb.htm
  static toRGBA(t) {
    const { h: n, s: r, v: s, a: i } = t, a = s * r, o = a * (1 - Math.abs(n / 60 % 2 - 1)), c = s - a;
    let [u, h, f] = [0, 0, 0];
    return n < 60 ? (u = a, h = o) : n < 120 ? (u = o, h = a) : n < 180 ? (h = a, f = o) : n < 240 ? (h = o, f = a) : n < 300 ? (u = o, f = a) : n <= 360 && (u = a, f = o), u = Math.round((u + c) * 255), h = Math.round((h + c) * 255), f = Math.round((f + c) * 255), new j(u, h, f, i);
  }
}
class q {
  static fromHex(t) {
    return q.Format.CSS.parseHex(t) || q.red;
  }
  static equals(t, n) {
    return !t && !n ? !0 : !t || !n ? !1 : t.equals(n);
  }
  get hsla() {
    return this._hsla ? this._hsla : re.fromRGBA(this.rgba);
  }
  get hsva() {
    return this._hsva ? this._hsva : De.fromRGBA(this.rgba);
  }
  constructor(t) {
    if (t)
      if (t instanceof j)
        this.rgba = t;
      else if (t instanceof re)
        this._hsla = t, this.rgba = re.toRGBA(t);
      else if (t instanceof De)
        this._hsva = t, this.rgba = De.toRGBA(t);
      else
        throw new Error("Invalid color ctor argument");
    else
      throw new Error("Color needs a value");
  }
  equals(t) {
    return !!t && j.equals(this.rgba, t.rgba) && re.equals(this.hsla, t.hsla) && De.equals(this.hsva, t.hsva);
  }
  /**
   * http://www.w3.org/TR/WCAG20/#relativeluminancedef
   * Returns the number in the set [0, 1]. O => Darkest Black. 1 => Lightest white.
   */
  getRelativeLuminance() {
    const t = q._relativeLuminanceForComponent(this.rgba.r), n = q._relativeLuminanceForComponent(this.rgba.g), r = q._relativeLuminanceForComponent(this.rgba.b), s = 0.2126 * t + 0.7152 * n + 0.0722 * r;
    return we(s, 4);
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
    return new q(new re(this.hsla.h, this.hsla.s, this.hsla.l + this.hsla.l * t, this.hsla.a));
  }
  darken(t) {
    return new q(new re(this.hsla.h, this.hsla.s, this.hsla.l - this.hsla.l * t, this.hsla.a));
  }
  transparent(t) {
    const { r: n, g: r, b: s, a: i } = this.rgba;
    return new q(new j(n, r, s, i * t));
  }
  isTransparent() {
    return this.rgba.a === 0;
  }
  isOpaque() {
    return this.rgba.a === 1;
  }
  opposite() {
    return new q(new j(255 - this.rgba.r, 255 - this.rgba.g, 255 - this.rgba.b, this.rgba.a));
  }
  makeOpaque(t) {
    if (this.isOpaque() || t.rgba.a !== 1)
      return this;
    const { r: n, g: r, b: s, a: i } = this.rgba;
    return new q(new j(t.rgba.r - i * (t.rgba.r - n), t.rgba.g - i * (t.rgba.g - r), t.rgba.b - i * (t.rgba.b - s), 1));
  }
  toString() {
    return this._toString || (this._toString = q.Format.CSS.format(this)), this._toString;
  }
  static getLighterColor(t, n, r) {
    if (t.isLighterThan(n))
      return t;
    r = r || 0.5;
    const s = t.getRelativeLuminance(), i = n.getRelativeLuminance();
    return r = r * (i - s) / i, t.lighten(r);
  }
  static getDarkerColor(t, n, r) {
    if (t.isDarkerThan(n))
      return t;
    r = r || 0.5;
    const s = t.getRelativeLuminance(), i = n.getRelativeLuminance();
    return r = r * (s - i) / s, t.darken(r);
  }
}
q.white = new q(new j(255, 255, 255, 1));
q.black = new q(new j(0, 0, 0, 1));
q.red = new q(new j(255, 0, 0, 1));
q.blue = new q(new j(0, 0, 255, 1));
q.green = new q(new j(0, 255, 0, 1));
q.cyan = new q(new j(0, 255, 255, 1));
q.lightgrey = new q(new j(211, 211, 211, 1));
q.transparent = new q(new j(0, 0, 0, 0));
(function(e) {
  (function(t) {
    (function(n) {
      function r(d) {
        return d.rgba.a === 1 ? `rgb(${d.rgba.r}, ${d.rgba.g}, ${d.rgba.b})` : e.Format.CSS.formatRGBA(d);
      }
      n.formatRGB = r;
      function s(d) {
        return `rgba(${d.rgba.r}, ${d.rgba.g}, ${d.rgba.b}, ${+d.rgba.a.toFixed(2)})`;
      }
      n.formatRGBA = s;
      function i(d) {
        return d.hsla.a === 1 ? `hsl(${d.hsla.h}, ${(d.hsla.s * 100).toFixed(2)}%, ${(d.hsla.l * 100).toFixed(2)}%)` : e.Format.CSS.formatHSLA(d);
      }
      n.formatHSL = i;
      function a(d) {
        return `hsla(${d.hsla.h}, ${(d.hsla.s * 100).toFixed(2)}%, ${(d.hsla.l * 100).toFixed(2)}%, ${d.hsla.a.toFixed(2)})`;
      }
      n.formatHSLA = a;
      function o(d) {
        const g = d.toString(16);
        return g.length !== 2 ? "0" + g : g;
      }
      function c(d) {
        return `#${o(d.rgba.r)}${o(d.rgba.g)}${o(d.rgba.b)}`;
      }
      n.formatHex = c;
      function u(d, g = !1) {
        return g && d.rgba.a === 1 ? e.Format.CSS.formatHex(d) : `#${o(d.rgba.r)}${o(d.rgba.g)}${o(d.rgba.b)}${o(Math.round(d.rgba.a * 255))}`;
      }
      n.formatHexA = u;
      function h(d) {
        return d.isOpaque() ? e.Format.CSS.formatHex(d) : e.Format.CSS.formatRGBA(d);
      }
      n.format = h;
      function f(d) {
        const g = d.length;
        if (g === 0 || d.charCodeAt(0) !== 35)
          return null;
        if (g === 7) {
          const b = 16 * m(d.charCodeAt(1)) + m(d.charCodeAt(2)), w = 16 * m(d.charCodeAt(3)) + m(d.charCodeAt(4)), L = 16 * m(d.charCodeAt(5)) + m(d.charCodeAt(6));
          return new e(new j(b, w, L, 1));
        }
        if (g === 9) {
          const b = 16 * m(d.charCodeAt(1)) + m(d.charCodeAt(2)), w = 16 * m(d.charCodeAt(3)) + m(d.charCodeAt(4)), L = 16 * m(d.charCodeAt(5)) + m(d.charCodeAt(6)), _ = 16 * m(d.charCodeAt(7)) + m(d.charCodeAt(8));
          return new e(new j(b, w, L, _ / 255));
        }
        if (g === 4) {
          const b = m(d.charCodeAt(1)), w = m(d.charCodeAt(2)), L = m(d.charCodeAt(3));
          return new e(new j(16 * b + b, 16 * w + w, 16 * L + L));
        }
        if (g === 5) {
          const b = m(d.charCodeAt(1)), w = m(d.charCodeAt(2)), L = m(d.charCodeAt(3)), _ = m(d.charCodeAt(4));
          return new e(new j(16 * b + b, 16 * w + w, 16 * L + L, (16 * _ + _) / 255));
        }
        return null;
      }
      n.parseHex = f;
      function m(d) {
        switch (d) {
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
})(q || (q = {}));
function Br(e) {
  const t = [];
  for (const n of e) {
    const r = Number(n);
    (r || r === 0 && n.replace(/\s/g, "") !== "") && t.push(r);
  }
  return t;
}
function Yt(e, t, n, r) {
  return {
    red: e / 255,
    blue: n / 255,
    green: t / 255,
    alpha: r
  };
}
function ze(e, t) {
  const n = t.index, r = t[0].length;
  if (!n)
    return;
  const s = e.positionAt(n);
  return {
    startLineNumber: s.lineNumber,
    startColumn: s.column,
    endLineNumber: s.lineNumber,
    endColumn: s.column + r
  };
}
function e1(e, t) {
  if (!e)
    return;
  const n = q.Format.CSS.parseHex(t);
  if (n)
    return {
      range: e,
      color: Yt(n.rgba.r, n.rgba.g, n.rgba.b, n.rgba.a)
    };
}
function _r(e, t, n) {
  if (!e || t.length !== 1)
    return;
  const s = t[0].values(), i = Br(s);
  return {
    range: e,
    color: Yt(i[0], i[1], i[2], n ? i[3] : 1)
  };
}
function xr(e, t, n) {
  if (!e || t.length !== 1)
    return;
  const s = t[0].values(), i = Br(s), a = new q(new re(i[0], i[1] / 100, i[2] / 100, n ? i[3] : 1));
  return {
    range: e,
    color: Yt(a.rgba.r, a.rgba.g, a.rgba.b, a.rgba.a)
  };
}
function $e(e, t) {
  return typeof e == "string" ? [...e.matchAll(t)] : e.findMatches(t);
}
function t1(e) {
  const t = [], r = $e(e, /\b(rgb|rgba|hsl|hsla)(\([0-9\s,.\%]*\))|(#)([A-Fa-f0-9]{3})\b|(#)([A-Fa-f0-9]{4})\b|(#)([A-Fa-f0-9]{6})\b|(#)([A-Fa-f0-9]{8})\b/gm);
  if (r.length > 0)
    for (const s of r) {
      const i = s.filter((u) => u !== void 0), a = i[1], o = i[2];
      if (!o)
        continue;
      let c;
      if (a === "rgb") {
        const u = /^\(\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*\)$/gm;
        c = _r(ze(e, s), $e(o, u), !1);
      } else if (a === "rgba") {
        const u = /^\(\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(0[.][0-9]+|[.][0-9]+|[01][.]|[01])\s*\)$/gm;
        c = _r(ze(e, s), $e(o, u), !0);
      } else if (a === "hsl") {
        const u = /^\(\s*(36[0]|3[0-5][0-9]|[12][0-9][0-9]|[1-9]?[0-9])\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*\)$/gm;
        c = xr(ze(e, s), $e(o, u), !1);
      } else if (a === "hsla") {
        const u = /^\(\s*(36[0]|3[0-5][0-9]|[12][0-9][0-9]|[1-9]?[0-9])\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*,\s*(0[.][0-9]+|[.][0-9]+|[01][.]|[01])\s*\)$/gm;
        c = xr(ze(e, s), $e(o, u), !0);
      } else
        a === "#" && (c = e1(ze(e, s), a + o));
      c && t.push(c);
    }
  return t;
}
function n1(e) {
  return !e || typeof e.getValue != "function" || typeof e.positionAt != "function" ? [] : t1(e);
}
var ge = globalThis && globalThis.__awaiter || function(e, t, n, r) {
  function s(i) {
    return i instanceof n ? i : new n(function(a) {
      a(i);
    });
  }
  return new (n || (n = Promise))(function(i, a) {
    function o(h) {
      try {
        u(r.next(h));
      } catch (f) {
        a(f);
      }
    }
    function c(h) {
      try {
        u(r.throw(h));
      } catch (f) {
        a(f);
      }
    }
    function u(h) {
      h.done ? i(h.value) : s(h.value).then(o, c);
    }
    u((r = r.apply(e, t || [])).next());
  });
};
class r1 extends ti {
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
      const s = this._lines[r], i = this.offsetAt(new J(r + 1, 1)), a = s.matchAll(t);
      for (const o of a)
        (o.index || o.index === 0) && (o.index = o.index + i), n.push(o);
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
    const r = Qt(t.column, si(n), this._lines[t.lineNumber - 1], 0);
    return r ? new D(t.lineNumber, r.startColumn, t.lineNumber, r.endColumn) : null;
  }
  words(t) {
    const n = this._lines, r = this._wordenize.bind(this);
    let s = 0, i = "", a = 0, o = [];
    return {
      *[Symbol.iterator]() {
        for (; ; )
          if (a < o.length) {
            const c = i.substring(o[a].start, o[a].end);
            a += 1, yield c;
          } else if (s < n.length)
            i = n[s], o = r(i, t), a = 0, s += 1;
          else
            break;
      }
    };
  }
  getLineWords(t, n) {
    const r = this._lines[t - 1], s = this._wordenize(r, n), i = [];
    for (const a of s)
      i.push({
        word: r.substring(a.start, a.end),
        startColumn: a.start + 1,
        endColumn: a.end + 1
      });
    return i;
  }
  _wordenize(t, n) {
    const r = [];
    let s;
    for (n.lastIndex = 0; (s = n.exec(t)) && s[0].length !== 0; )
      r.push({ start: s.index, end: s.index + s[0].length });
    return r;
  }
  getValueInRange(t) {
    if (t = this._validateRange(t), t.startLineNumber === t.endLineNumber)
      return this._lines[t.startLineNumber - 1].substring(t.startColumn - 1, t.endColumn - 1);
    const n = this._eol, r = t.startLineNumber - 1, s = t.endLineNumber - 1, i = [];
    i.push(this._lines[r].substring(t.startColumn - 1));
    for (let a = r + 1; a < s; a++)
      i.push(this._lines[a]);
    return i.push(this._lines[s].substring(0, t.endColumn - 1)), i.join(n);
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
    if (!J.isIPosition(t))
      throw new Error("bad position");
    let { lineNumber: n, column: r } = t, s = !1;
    if (n < 1)
      n = 1, r = 1, s = !0;
    else if (n > this._lines.length)
      n = this._lines.length, r = this._lines[n - 1].length + 1, s = !0;
    else {
      const i = this._lines[n - 1].length + 1;
      r < 1 ? (r = 1, s = !0) : r > i && (r = i, s = !0);
    }
    return s ? { lineNumber: n, column: r } : t;
  }
}
class Ae {
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
    this._models[t.url] = new r1(Ce.parse(t.url), t.lines, t.EOL, t.versionId);
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
    return ge(this, void 0, void 0, function* () {
      const s = this._getModel(t);
      return s ? Ci.computeUnicodeHighlights(s, n, r) : { ranges: [], hasMore: !1, ambiguousCharacterCount: 0, invisibleCharacterCount: 0, nonBasicAsciiCharacterCount: 0 };
    });
  }
  // ---- BEGIN diff --------------------------------------------------------------------------
  computeDiff(t, n, r, s) {
    return ge(this, void 0, void 0, function* () {
      const i = this._getModel(t), a = this._getModel(n);
      return !i || !a ? null : Ae.computeDiff(i, a, r, s);
    });
  }
  static computeDiff(t, n, r, s) {
    const i = s === "advanced" ? br.getDefault() : br.getLegacy(), a = t.getLinesContent(), o = n.getLinesContent(), c = i.computeDiff(a, o, r), u = c.changes.length > 0 ? !1 : this._modelsAreIdentical(t, n);
    function h(f) {
      return f.map((m) => {
        var d;
        return [m.original.startLineNumber, m.original.endLineNumberExclusive, m.modified.startLineNumber, m.modified.endLineNumberExclusive, (d = m.innerChanges) === null || d === void 0 ? void 0 : d.map((g) => [
          g.originalRange.startLineNumber,
          g.originalRange.startColumn,
          g.originalRange.endLineNumber,
          g.originalRange.endColumn,
          g.modifiedRange.startLineNumber,
          g.modifiedRange.startColumn,
          g.modifiedRange.endLineNumber,
          g.modifiedRange.endColumn
        ])];
      });
    }
    return {
      identical: u,
      quitEarly: c.hitTimeout,
      changes: h(c.changes),
      moves: c.moves.map((f) => [
        f.lineRangeMapping.original.startLineNumber,
        f.lineRangeMapping.original.endLineNumberExclusive,
        f.lineRangeMapping.modified.startLineNumber,
        f.lineRangeMapping.modified.endLineNumberExclusive,
        h(f.changes)
      ])
    };
  }
  static _modelsAreIdentical(t, n) {
    const r = t.getLineCount(), s = n.getLineCount();
    if (r !== s)
      return !1;
    for (let i = 1; i <= r; i++) {
      const a = t.getLineContent(i), o = n.getLineContent(i);
      if (a !== o)
        return !1;
    }
    return !0;
  }
  computeMoreMinimalEdits(t, n, r) {
    return ge(this, void 0, void 0, function* () {
      const s = this._getModel(t);
      if (!s)
        return n;
      const i = [];
      let a;
      n = n.slice(0).sort((c, u) => {
        if (c.range && u.range)
          return D.compareRangesUsingStarts(c.range, u.range);
        const h = c.range ? 0 : 1, f = u.range ? 0 : 1;
        return h - f;
      });
      let o = 0;
      for (let c = 1; c < n.length; c++)
        D.getEndPosition(n[o].range).equals(D.getStartPosition(n[c].range)) ? (n[o].range = D.fromPositions(D.getStartPosition(n[o].range), D.getEndPosition(n[c].range)), n[o].text += n[c].text) : (o++, n[o] = n[c]);
      n.length = o + 1;
      for (let { range: c, text: u, eol: h } of n) {
        if (typeof h == "number" && (a = h), D.isEmpty(c) && !u)
          continue;
        const f = s.getValueInRange(c);
        if (u = u.replace(/\r\n|\n|\r/g, s.eol), f === u)
          continue;
        if (Math.max(u.length, f.length) > Ae._diffLimit) {
          i.push({ range: c, text: u });
          continue;
        }
        const m = Ms(f, u, r), d = s.offsetAt(D.lift(c).getStartPosition());
        for (const g of m) {
          const b = s.positionAt(d + g.originalStart), w = s.positionAt(d + g.originalStart + g.originalLength), L = {
            text: u.substr(g.modifiedStart, g.modifiedLength),
            range: { startLineNumber: b.lineNumber, startColumn: b.column, endLineNumber: w.lineNumber, endColumn: w.column }
          };
          s.getValueInRange(L.range) !== L.text && i.push(L);
        }
      }
      return typeof a == "number" && i.push({ eol: a, text: "", range: { startLineNumber: 0, startColumn: 0, endLineNumber: 0, endColumn: 0 } }), i;
    });
  }
  // ---- END minimal edits ---------------------------------------------------------------
  computeLinks(t) {
    return ge(this, void 0, void 0, function* () {
      const n = this._getModel(t);
      return n ? ci(n) : null;
    });
  }
  // --- BEGIN default document colors -----------------------------------------------------------
  computeDefaultDocumentColors(t) {
    return ge(this, void 0, void 0, function* () {
      const n = this._getModel(t);
      return n ? n1(n) : null;
    });
  }
  textualSuggest(t, n, r, s) {
    return ge(this, void 0, void 0, function* () {
      const i = new _t(), a = new RegExp(r, s), o = /* @__PURE__ */ new Set();
      e:
        for (const c of t) {
          const u = this._getModel(c);
          if (u) {
            for (const h of u.words(a))
              if (!(h === n || !isNaN(Number(h))) && (o.add(h), o.size > Ae._suggestionsLimit))
                break e;
          }
        }
      return { words: Array.from(o), duration: i.elapsed() };
    });
  }
  // ---- END suggest --------------------------------------------------------------------------
  //#region -- word ranges --
  computeWordRanges(t, n, r, s) {
    return ge(this, void 0, void 0, function* () {
      const i = this._getModel(t);
      if (!i)
        return /* @__PURE__ */ Object.create(null);
      const a = new RegExp(r, s), o = /* @__PURE__ */ Object.create(null);
      for (let c = n.startLineNumber; c < n.endLineNumber; c++) {
        const u = i.getLineWords(c, a);
        for (const h of u) {
          if (!isNaN(Number(h.word)))
            continue;
          let f = o[h.word];
          f || (f = [], o[h.word] = f), f.push({
            startLineNumber: c,
            startColumn: h.startColumn,
            endLineNumber: c,
            endColumn: h.endColumn
          });
        }
      }
      return o;
    });
  }
  //#endregion
  navigateValueSet(t, n, r, s, i) {
    return ge(this, void 0, void 0, function* () {
      const a = this._getModel(t);
      if (!a)
        return null;
      const o = new RegExp(s, i);
      n.startColumn === n.endColumn && (n = {
        startLineNumber: n.startLineNumber,
        startColumn: n.startColumn,
        endLineNumber: n.endLineNumber,
        endColumn: n.endColumn + 1
      });
      const c = a.getValueInRange(n), u = a.getWordAtPosition({ lineNumber: n.startLineNumber, column: n.startColumn }, o);
      if (!u)
        return null;
      const h = a.getValueInRange(u);
      return Dt.INSTANCE.navigateValueSet(n, c, u, h, r);
    });
  }
  // ---- BEGIN foreign module support --------------------------------------------------------------------------
  loadForeignModule(t, n, r) {
    const a = {
      host: ts(r, (o, c) => this._host.fhr(o, c)),
      getMirrorModels: () => this._getModels()
    };
    return this._foreignModuleFactory ? (this._foreignModule = this._foreignModuleFactory(a, n), Promise.resolve(Rt(this._foreignModule))) : Promise.reject(new Error("Unexpected usage"));
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
Ae._diffLimit = 1e5;
Ae._suggestionsLimit = 1e4;
typeof importScripts == "function" && (globalThis.monaco = pi());
let Gt = !1;
function s1(e) {
  if (Gt)
    return;
  Gt = !0;
  const t = new Rs((n) => {
    globalThis.postMessage(n);
  }, (n) => new Ae(n, e));
  globalThis.onmessage = (n) => {
    t.onmessage(n.data);
  };
}
globalThis.onmessage = (e) => {
  Gt || s1(null);
};
export {
  s1 as initialize
};
