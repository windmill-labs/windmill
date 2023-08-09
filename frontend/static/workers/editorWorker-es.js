class kr {
  constructor() {
    this.listeners = [], this.unexpectedErrorHandler = function(t) {
      setTimeout(() => {
        throw t.stack ? Ee.isErrorNoTelemetry(t) ? new Ee(t.message + `

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
const Er = new kr();
function lr(e) {
  Fr(e) || Er.onUnexpectedError(e);
}
function It(e) {
  if (e instanceof Error) {
    const { name: t, message: n } = e, r = e.stacktrace || e.stack;
    return {
      $isError: !0,
      name: t,
      message: n,
      stack: r,
      noTelemetry: Ee.isErrorNoTelemetry(e)
    };
  }
  return e;
}
const ht = "Canceled";
function Fr(e) {
  return e instanceof Pr ? !0 : e instanceof Error && e.name === ht && e.message === ht;
}
class Pr extends Error {
  constructor() {
    super(ht), this.name = this.message;
  }
}
class Ee extends Error {
  constructor(t) {
    super(t), this.name = "CodeExpectedError";
  }
  static fromError(t) {
    if (t instanceof Ee)
      return t;
    const n = new Ee();
    return n.message = t.message, n.stack = t.stack, n;
  }
  static isErrorNoTelemetry(t) {
    return t.name === "CodeExpectedError";
  }
}
class Pe extends Error {
  constructor(t) {
    super(t || "An unexpected bug occurred."), Object.setPrototypeOf(this, Pe.prototype);
  }
}
function Dr(e) {
  const t = this;
  let n = !1, r;
  return function() {
    return n || (n = !0, r = e.apply(t, arguments)), r;
  };
}
var Ye;
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
  function* s(b) {
    yield b;
  }
  e.single = s;
  function i(b) {
    return t(b) ? b : s(b);
  }
  e.wrap = i;
  function l(b) {
    return b || n;
  }
  e.from = l;
  function o(b) {
    return !b || b[Symbol.iterator]().next().done === !0;
  }
  e.isEmpty = o;
  function c(b) {
    return b[Symbol.iterator]().next().value;
  }
  e.first = c;
  function u(b, N) {
    for (const w of b)
      if (N(w))
        return !0;
    return !1;
  }
  e.some = u;
  function h(b, N) {
    for (const w of b)
      if (N(w))
        return w;
  }
  e.find = h;
  function* f(b, N) {
    for (const w of b)
      N(w) && (yield w);
  }
  e.filter = f;
  function* d(b, N) {
    let w = 0;
    for (const k of b)
      yield N(k, w++);
  }
  e.map = d;
  function* m(...b) {
    for (const N of b)
      for (const w of N)
        yield w;
  }
  e.concat = m;
  function g(b, N, w) {
    let k = w;
    for (const A of b)
      k = N(k, A);
    return k;
  }
  e.reduce = g;
  function* _(b, N, w = b.length) {
    for (N < 0 && (N += b.length), w < 0 ? w += b.length : w > b.length && (w = b.length); N < w; N++)
      yield b[N];
  }
  e.slice = _;
  function S(b, N = Number.POSITIVE_INFINITY) {
    const w = [];
    if (N === 0)
      return [w, b];
    const k = b[Symbol.iterator]();
    for (let A = 0; A < N; A++) {
      const E = k.next();
      if (E.done)
        return [w, e.empty()];
      w.push(E.value);
    }
    return [w, { [Symbol.iterator]() {
      return k;
    } }];
  }
  e.consume = S;
})(Ye || (Ye = {}));
function or(e) {
  if (Ye.is(e)) {
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
function Vr(...e) {
  return Ie(() => or(e));
}
function Ie(e) {
  return {
    dispose: Dr(() => {
      e();
    })
  };
}
class ve {
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
        or(this._toDispose);
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
    return this._isDisposed ? ve.DISABLE_DISPOSED_WARNING || console.warn(new Error("Trying to add a disposable to a DisposableStore that has already been disposed of. The added object will be leaked!").stack) : this._toDispose.add(t), t;
  }
}
ve.DISABLE_DISPOSED_WARNING = !1;
class Ue {
  constructor() {
    this._store = new ve(), this._store;
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
Ue.None = Object.freeze({ dispose() {
} });
class B {
  constructor(t) {
    this.element = t, this.next = B.Undefined, this.prev = B.Undefined;
  }
}
B.Undefined = new B(void 0);
class Tr {
  constructor() {
    this._first = B.Undefined, this._last = B.Undefined, this._size = 0;
  }
  get size() {
    return this._size;
  }
  isEmpty() {
    return this._first === B.Undefined;
  }
  clear() {
    let t = this._first;
    for (; t !== B.Undefined; ) {
      const n = t.next;
      t.prev = B.Undefined, t.next = B.Undefined, t = n;
    }
    this._first = B.Undefined, this._last = B.Undefined, this._size = 0;
  }
  unshift(t) {
    return this._insert(t, !1);
  }
  push(t) {
    return this._insert(t, !0);
  }
  _insert(t, n) {
    const r = new B(t);
    if (this._first === B.Undefined)
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
    if (this._first !== B.Undefined) {
      const t = this._first.element;
      return this._remove(this._first), t;
    }
  }
  pop() {
    if (this._last !== B.Undefined) {
      const t = this._last.element;
      return this._remove(this._last), t;
    }
  }
  _remove(t) {
    if (t.prev !== B.Undefined && t.next !== B.Undefined) {
      const n = t.prev;
      n.next = t.next, t.next.prev = n;
    } else
      t.prev === B.Undefined && t.next === B.Undefined ? (this._first = B.Undefined, this._last = B.Undefined) : t.next === B.Undefined ? (this._last = this._last.prev, this._last.next = B.Undefined) : t.prev === B.Undefined && (this._first = this._first.next, this._first.prev = B.Undefined);
    this._size -= 1;
  }
  *[Symbol.iterator]() {
    let t = this._first;
    for (; t !== B.Undefined; )
      yield t.element, t = t.next;
  }
}
const Br = globalThis.performance && typeof globalThis.performance.now == "function";
class nt {
  static create(t) {
    return new nt(t);
  }
  constructor(t) {
    this._now = Br && t === !1 ? Date.now : globalThis.performance.now.bind(globalThis.performance), this._startTime = this._now(), this._stopTime = -1;
  }
  stop() {
    this._stopTime = this._now();
  }
  elapsed() {
    return this._stopTime !== -1 ? this._stopTime - this._startTime : this._now() - this._startTime;
  }
}
var ft;
(function(e) {
  e.None = () => Ue.None;
  function t(L, p) {
    return h(L, () => {
    }, 0, void 0, !0, void 0, p);
  }
  e.defer = t;
  function n(L) {
    return (p, v = null, x) => {
      let M = !1, D;
      return D = L((q) => {
        if (!M)
          return D ? D.dispose() : M = !0, p.call(v, q);
      }, null, x), M && D.dispose(), D;
    };
  }
  e.once = n;
  function r(L, p, v) {
    return u((x, M = null, D) => L((q) => x.call(M, p(q)), null, D), v);
  }
  e.map = r;
  function s(L, p, v) {
    return u((x, M = null, D) => L((q) => {
      p(q), x.call(M, q);
    }, null, D), v);
  }
  e.forEach = s;
  function i(L, p, v) {
    return u((x, M = null, D) => L((q) => p(q) && x.call(M, q), null, D), v);
  }
  e.filter = i;
  function l(L) {
    return L;
  }
  e.signal = l;
  function o(...L) {
    return (p, v = null, x) => Vr(...L.map((M) => M((D) => p.call(v, D), null, x)));
  }
  e.any = o;
  function c(L, p, v, x) {
    let M = v;
    return r(L, (D) => (M = p(M, D), M), x);
  }
  e.reduce = c;
  function u(L, p) {
    let v;
    const x = {
      onWillAddFirstListener() {
        v = L(M.fire, M);
      },
      onDidRemoveLastListener() {
        v == null || v.dispose();
      }
    }, M = new se(x);
    return p == null || p.add(M), M.event;
  }
  function h(L, p, v = 100, x = !1, M = !1, D, q) {
    let J, we, Le, ze = 0, _e;
    const Rr = {
      leakWarningThreshold: D,
      onWillAddFirstListener() {
        J = L((yr) => {
          ze++, we = p(we, yr), x && !Le && (Ge.fire(we), we = void 0), _e = () => {
            const Mr = we;
            we = void 0, Le = void 0, (!x || ze > 1) && Ge.fire(Mr), ze = 0;
          }, typeof v == "number" ? (clearTimeout(Le), Le = setTimeout(_e, v)) : Le === void 0 && (Le = 0, queueMicrotask(_e));
        });
      },
      onWillRemoveListener() {
        M && ze > 0 && (_e == null || _e());
      },
      onDidRemoveLastListener() {
        _e = void 0, J.dispose();
      }
    }, Ge = new se(Rr);
    return q == null || q.add(Ge), Ge.event;
  }
  e.debounce = h;
  function f(L, p = 0, v) {
    return e.debounce(L, (x, M) => x ? (x.push(M), x) : [M], p, void 0, !0, void 0, v);
  }
  e.accumulate = f;
  function d(L, p = (x, M) => x === M, v) {
    let x = !0, M;
    return i(L, (D) => {
      const q = x || !p(D, M);
      return x = !1, M = D, q;
    }, v);
  }
  e.latch = d;
  function m(L, p, v) {
    return [
      e.filter(L, p, v),
      e.filter(L, (x) => !p(x), v)
    ];
  }
  e.split = m;
  function g(L, p = !1, v = []) {
    let x = v.slice(), M = L((J) => {
      x ? x.push(J) : q.fire(J);
    });
    const D = () => {
      x == null || x.forEach((J) => q.fire(J)), x = null;
    }, q = new se({
      onWillAddFirstListener() {
        M || (M = L((J) => q.fire(J)));
      },
      onDidAddFirstListener() {
        x && (p ? setTimeout(D) : D());
      },
      onDidRemoveLastListener() {
        M && M.dispose(), M = null;
      }
    });
    return q.event;
  }
  e.buffer = g;
  class _ {
    constructor(p) {
      this.event = p, this.disposables = new ve();
    }
    /** @see {@link Event.map} */
    map(p) {
      return new _(r(this.event, p, this.disposables));
    }
    /** @see {@link Event.forEach} */
    forEach(p) {
      return new _(s(this.event, p, this.disposables));
    }
    filter(p) {
      return new _(i(this.event, p, this.disposables));
    }
    /** @see {@link Event.reduce} */
    reduce(p, v) {
      return new _(c(this.event, p, v, this.disposables));
    }
    /** @see {@link Event.reduce} */
    latch() {
      return new _(d(this.event, void 0, this.disposables));
    }
    debounce(p, v = 100, x = !1, M = !1, D) {
      return new _(h(this.event, p, v, x, M, D, this.disposables));
    }
    /**
     * Attach a listener to the event.
     */
    on(p, v, x) {
      return this.event(p, v, x);
    }
    /** @see {@link Event.once} */
    once(p, v, x) {
      return n(this.event)(p, v, x);
    }
    dispose() {
      this.disposables.dispose();
    }
  }
  function S(L) {
    return new _(L);
  }
  e.chain = S;
  function b(L, p, v = (x) => x) {
    const x = (...J) => q.fire(v(...J)), M = () => L.on(p, x), D = () => L.removeListener(p, x), q = new se({ onWillAddFirstListener: M, onDidRemoveLastListener: D });
    return q.event;
  }
  e.fromNodeEventEmitter = b;
  function N(L, p, v = (x) => x) {
    const x = (...J) => q.fire(v(...J)), M = () => L.addEventListener(p, x), D = () => L.removeEventListener(p, x), q = new se({ onWillAddFirstListener: M, onDidRemoveLastListener: D });
    return q.event;
  }
  e.fromDOMEventEmitter = N;
  function w(L) {
    return new Promise((p) => n(L)(p));
  }
  e.toPromise = w;
  function k(L, p) {
    return p(void 0), L((v) => p(v));
  }
  e.runAndSubscribe = k;
  function A(L, p) {
    let v = null;
    function x(D) {
      v == null || v.dispose(), v = new ve(), p(D, v);
    }
    x(void 0);
    const M = L((D) => x(D));
    return Ie(() => {
      M.dispose(), v == null || v.dispose();
    });
  }
  e.runAndSubscribeWithStore = A;
  class E {
    constructor(p, v) {
      this._observable = p, this._counter = 0, this._hasChanged = !1;
      const x = {
        onWillAddFirstListener: () => {
          p.addObserver(this);
        },
        onDidRemoveLastListener: () => {
          p.removeObserver(this);
        }
      };
      this.emitter = new se(x), v && v.add(this.emitter);
    }
    beginUpdate(p) {
      this._counter++;
    }
    handlePossibleChange(p) {
    }
    handleChange(p, v) {
      this._hasChanged = !0;
    }
    endUpdate(p) {
      this._counter--, this._counter === 0 && (this._observable.reportChanges(), this._hasChanged && (this._hasChanged = !1, this.emitter.fire(this._observable.get())));
    }
  }
  function C(L, p) {
    return new E(L, p).emitter.event;
  }
  e.fromObservable = C;
  function R(L) {
    return (p) => {
      let v = 0, x = !1;
      const M = {
        beginUpdate() {
          v++;
        },
        endUpdate() {
          v--, v === 0 && (L.reportChanges(), x && (x = !1, p()));
        },
        handlePossibleChange() {
        },
        handleChange() {
          x = !0;
        }
      };
      return L.addObserver(M), L.reportChanges(), {
        dispose() {
          L.removeObserver(M);
        }
      };
    };
  }
  e.fromObservableLight = R;
})(ft || (ft = {}));
class Fe {
  constructor(t) {
    this.listenerCount = 0, this.invocationCount = 0, this.elapsedOverall = 0, this.durations = [], this.name = `${t}_${Fe._idPool++}`, Fe.all.add(this);
  }
  start(t) {
    this._stopWatch = new nt(), this.listenerCount = t;
  }
  stop() {
    if (this._stopWatch) {
      const t = this._stopWatch.elapsed();
      this.durations.push(t), this.elapsedOverall += t, this.invocationCount += 1, this._stopWatch = void 0;
    }
  }
}
Fe.all = /* @__PURE__ */ new Set();
Fe._idPool = 0;
let Ir = -1;
class Ur {
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
      let i, l = 0;
      for (const [o, c] of this._stacks)
        (!i || l < c) && (i = o, l = c);
      console.warn(`[${this.name}] potential listener LEAK detected, having ${n} listeners already. MOST frequent listener (${l}):`), console.warn(i);
    }
    return () => {
      const i = this._stacks.get(t.value) || 0;
      this._stacks.set(t.value, i - 1);
    };
  }
}
class Et {
  static create() {
    var t;
    return new Et((t = new Error().stack) !== null && t !== void 0 ? t : "");
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
class rt {
  constructor(t) {
    this.value = t;
  }
}
const qr = 2;
class se {
  constructor(t) {
    var n, r, s, i, l;
    this._size = 0, this._options = t, this._leakageMon = !((n = this._options) === null || n === void 0) && n.leakWarningThreshold ? new Ur((s = (r = this._options) === null || r === void 0 ? void 0 : r.leakWarningThreshold) !== null && s !== void 0 ? s : Ir) : void 0, this._perfMon = !((i = this._options) === null || i === void 0) && i._profName ? new Fe(this._options._profName) : void 0, this._deliveryQueue = (l = this._options) === null || l === void 0 ? void 0 : l.deliveryQueue;
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
      var i, l, o, c, u;
      if (this._leakageMon && this._size > this._leakageMon.threshold * 3)
        return console.warn(`[${this._leakageMon.name}] REFUSES to accept new listeners because it exceeded its threshold by far`), Ue.None;
      if (this._disposed)
        return Ue.None;
      r && (n = n.bind(r));
      const h = new rt(n);
      let f;
      this._leakageMon && this._size >= Math.ceil(this._leakageMon.threshold * 0.2) && (h.stack = Et.create(), f = this._leakageMon.check(h.stack, this._size + 1)), this._listeners ? this._listeners instanceof rt ? ((u = this._deliveryQueue) !== null && u !== void 0 || (this._deliveryQueue = new Hr()), this._listeners = [this._listeners, h]) : this._listeners.push(h) : ((l = (i = this._options) === null || i === void 0 ? void 0 : i.onWillAddFirstListener) === null || l === void 0 || l.call(i, this), this._listeners = h, (c = (o = this._options) === null || o === void 0 ? void 0 : o.onDidAddFirstListener) === null || c === void 0 || c.call(o, this)), this._size++;
      const d = Ie(() => {
        f == null || f(), this._removeListener(h);
      });
      return s instanceof ve ? s.add(d) : Array.isArray(s) && s.push(d), d;
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
    const l = this._listeners, o = l.indexOf(t);
    if (o === -1)
      throw console.log("disposed?", this._disposed), console.log("size?", this._size), console.log("arr?", JSON.stringify(this._listeners)), new Error("Attempted to dispose unknown listener");
    this._size--, l[o] = void 0;
    const c = this._deliveryQueue.current === this;
    if (this._size * qr <= l.length) {
      let u = 0;
      for (let h = 0; h < l.length; h++)
        l[h] ? l[u++] = l[h] : c && (this._deliveryQueue.end--, u < this._deliveryQueue.i && this._deliveryQueue.i--);
      l.length = u;
    }
  }
  _deliver(t, n) {
    var r;
    if (!t)
      return;
    const s = ((r = this._options) === null || r === void 0 ? void 0 : r.onListenerError) || lr;
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
      if (this._listeners instanceof rt)
        this._deliver(this._listeners, t);
      else {
        const l = this._deliveryQueue;
        l.enqueue(this, t, this._listeners.length), this._deliverQueue(l);
      }
    (i = this._perfMon) === null || i === void 0 || i.stop();
  }
  hasListeners() {
    return this._size > 0;
  }
}
class Hr {
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
function Wr(e) {
  return typeof e == "string";
}
function $r(e) {
  let t = [];
  for (; Object.prototype !== e; )
    t = t.concat(Object.getOwnPropertyNames(e)), e = Object.getPrototypeOf(e);
  return t;
}
function dt(e) {
  const t = [];
  for (const n of $r(e))
    typeof e[n] == "function" && t.push(n);
  return t;
}
function zr(e, t) {
  const n = (s) => function() {
    const i = Array.prototype.slice.call(arguments, 0);
    return t(s, i);
  }, r = {};
  for (const s of e)
    r[s] = n(s);
  return r;
}
globalThis && globalThis.__awaiter;
let Gr = typeof document < "u" && document.location && document.location.hash.indexOf("pseudo=true") >= 0;
function Or(e, t) {
  let n;
  return t.length === 0 ? n = e : n = e.replace(/\{(\d+)\}/g, (r, s) => {
    const i = s[0], l = t[i];
    let o = r;
    return typeof l == "string" ? o = l : (typeof l == "number" || typeof l == "boolean" || l === void 0 || l === null) && (o = String(l)), o;
  }), Gr && (n = "［" + n.replace(/[aouei]/g, "$&$&") + "］"), n;
}
function U(e, t, ...n) {
  return Or(t, n);
}
var st;
const Re = "en";
let mt = !1, gt = !1, it = !1, ur = !1, Oe, at = Re, Ut = Re, jr, te;
const re = typeof self == "object" ? self : typeof global == "object" ? global : {};
let G;
typeof re.vscode < "u" && typeof re.vscode.process < "u" ? G = re.vscode.process : typeof process < "u" && (G = process);
const Qr = typeof ((st = G == null ? void 0 : G.versions) === null || st === void 0 ? void 0 : st.electron) == "string", Xr = Qr && (G == null ? void 0 : G.type) === "renderer";
if (typeof navigator == "object" && !Xr)
  te = navigator.userAgent, mt = te.indexOf("Windows") >= 0, gt = te.indexOf("Macintosh") >= 0, (te.indexOf("Macintosh") >= 0 || te.indexOf("iPad") >= 0 || te.indexOf("iPhone") >= 0) && navigator.maxTouchPoints && navigator.maxTouchPoints > 0, it = te.indexOf("Linux") >= 0, (te == null ? void 0 : te.indexOf("Mobi")) >= 0, ur = !0, // This call _must_ be done in the file that calls `nls.getConfiguredDefaultLocale`
  // to ensure that the NLS AMD Loader plugin has been loaded and configured.
  // This is because the loader plugin decides what the default locale is based on
  // how it's able to resolve the strings.
  U({ key: "ensureLoaderPluginIsLoaded", comment: ["{Locked}"] }, "_"), Oe = Re, at = Oe, Ut = navigator.language;
else if (typeof G == "object") {
  mt = G.platform === "win32", gt = G.platform === "darwin", it = G.platform === "linux", it && G.env.SNAP && G.env.SNAP_REVISION, G.env.CI || G.env.BUILD_ARTIFACTSTAGINGDIRECTORY, Oe = Re, at = Re;
  const e = G.env.VSCODE_NLS_CONFIG;
  if (e)
    try {
      const t = JSON.parse(e), n = t.availableLanguages["*"];
      Oe = t.locale, Ut = t.osLocale, at = n || Re, jr = t._translationsConfigFile;
    } catch {
    }
} else
  console.error("Unable to resolve platform.");
const qe = mt, Yr = gt;
ur && re.importScripts;
const ie = te, Jr = typeof re.postMessage == "function" && !re.importScripts;
(() => {
  if (Jr) {
    const e = [];
    re.addEventListener("message", (n) => {
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
      }), re.postMessage({ vscodeScheduleAsyncWork: r }, "*");
    };
  }
  return (e) => setTimeout(e);
})();
const Zr = !!(ie && ie.indexOf("Chrome") >= 0);
ie && ie.indexOf("Firefox") >= 0;
!Zr && ie && ie.indexOf("Safari") >= 0;
ie && ie.indexOf("Edg/") >= 0;
ie && ie.indexOf("Android") >= 0;
class Kr {
  constructor(t) {
    this.fn = t, this.lastCache = void 0, this.lastArgKey = void 0;
  }
  get(t) {
    const n = JSON.stringify(t);
    return this.lastArgKey !== n && (this.lastArgKey = n, this.lastCache = this.fn(t)), this.lastCache;
  }
}
class cr {
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
var hr;
function es(e) {
  return e.replace(/[\\\{\}\*\+\?\|\^\$\.\[\]\(\)]/g, "\\$&");
}
function ts(e) {
  return e.split(/\r\n|\r|\n/);
}
function ns(e) {
  for (let t = 0, n = e.length; t < n; t++) {
    const r = e.charCodeAt(t);
    if (r !== 32 && r !== 9)
      return t;
  }
  return -1;
}
function rs(e, t = e.length - 1) {
  for (let n = t; n >= 0; n--) {
    const r = e.charCodeAt(n);
    if (r !== 32 && r !== 9)
      return n;
  }
  return -1;
}
function fr(e) {
  return e >= 65 && e <= 90;
}
function bt(e) {
  return 55296 <= e && e <= 56319;
}
function ss(e) {
  return 56320 <= e && e <= 57343;
}
function is(e, t) {
  return (e - 55296 << 10) + (t - 56320) + 65536;
}
function as(e, t, n) {
  const r = e.charCodeAt(n);
  if (bt(r) && n + 1 < t) {
    const s = e.charCodeAt(n + 1);
    if (ss(s))
      return is(r, s);
  }
  return r;
}
const ls = /^[\t\n\r\x20-\x7E]*$/;
function os(e) {
  return ls.test(e);
}
class ee {
  static getInstance(t) {
    return ee.cache.get(Array.from(t));
  }
  static getLocales() {
    return ee._locales.value;
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
hr = ee;
ee.ambiguousCharacterData = new cr(() => JSON.parse('{"_common":[8232,32,8233,32,5760,32,8192,32,8193,32,8194,32,8195,32,8196,32,8197,32,8198,32,8200,32,8201,32,8202,32,8287,32,8199,32,8239,32,2042,95,65101,95,65102,95,65103,95,8208,45,8209,45,8210,45,65112,45,1748,45,8259,45,727,45,8722,45,10134,45,11450,45,1549,44,1643,44,8218,44,184,44,42233,44,894,59,2307,58,2691,58,1417,58,1795,58,1796,58,5868,58,65072,58,6147,58,6153,58,8282,58,1475,58,760,58,42889,58,8758,58,720,58,42237,58,451,33,11601,33,660,63,577,63,2429,63,5038,63,42731,63,119149,46,8228,46,1793,46,1794,46,42510,46,68176,46,1632,46,1776,46,42232,46,1373,96,65287,96,8219,96,8242,96,1370,96,1523,96,8175,96,65344,96,900,96,8189,96,8125,96,8127,96,8190,96,697,96,884,96,712,96,714,96,715,96,756,96,699,96,701,96,700,96,702,96,42892,96,1497,96,2036,96,2037,96,5194,96,5836,96,94033,96,94034,96,65339,91,10088,40,10098,40,12308,40,64830,40,65341,93,10089,41,10099,41,12309,41,64831,41,10100,123,119060,123,10101,125,65342,94,8270,42,1645,42,8727,42,66335,42,5941,47,8257,47,8725,47,8260,47,9585,47,10187,47,10744,47,119354,47,12755,47,12339,47,11462,47,20031,47,12035,47,65340,92,65128,92,8726,92,10189,92,10741,92,10745,92,119311,92,119355,92,12756,92,20022,92,12034,92,42872,38,708,94,710,94,5869,43,10133,43,66203,43,8249,60,10094,60,706,60,119350,60,5176,60,5810,60,5120,61,11840,61,12448,61,42239,61,8250,62,10095,62,707,62,119351,62,5171,62,94015,62,8275,126,732,126,8128,126,8764,126,65372,124,65293,45,120784,50,120794,50,120804,50,120814,50,120824,50,130034,50,42842,50,423,50,1000,50,42564,50,5311,50,42735,50,119302,51,120785,51,120795,51,120805,51,120815,51,120825,51,130035,51,42923,51,540,51,439,51,42858,51,11468,51,1248,51,94011,51,71882,51,120786,52,120796,52,120806,52,120816,52,120826,52,130036,52,5070,52,71855,52,120787,53,120797,53,120807,53,120817,53,120827,53,130037,53,444,53,71867,53,120788,54,120798,54,120808,54,120818,54,120828,54,130038,54,11474,54,5102,54,71893,54,119314,55,120789,55,120799,55,120809,55,120819,55,120829,55,130039,55,66770,55,71878,55,2819,56,2538,56,2666,56,125131,56,120790,56,120800,56,120810,56,120820,56,120830,56,130040,56,547,56,546,56,66330,56,2663,57,2920,57,2541,57,3437,57,120791,57,120801,57,120811,57,120821,57,120831,57,130041,57,42862,57,11466,57,71884,57,71852,57,71894,57,9082,97,65345,97,119834,97,119886,97,119938,97,119990,97,120042,97,120094,97,120146,97,120198,97,120250,97,120302,97,120354,97,120406,97,120458,97,593,97,945,97,120514,97,120572,97,120630,97,120688,97,120746,97,65313,65,119808,65,119860,65,119912,65,119964,65,120016,65,120068,65,120120,65,120172,65,120224,65,120276,65,120328,65,120380,65,120432,65,913,65,120488,65,120546,65,120604,65,120662,65,120720,65,5034,65,5573,65,42222,65,94016,65,66208,65,119835,98,119887,98,119939,98,119991,98,120043,98,120095,98,120147,98,120199,98,120251,98,120303,98,120355,98,120407,98,120459,98,388,98,5071,98,5234,98,5551,98,65314,66,8492,66,119809,66,119861,66,119913,66,120017,66,120069,66,120121,66,120173,66,120225,66,120277,66,120329,66,120381,66,120433,66,42932,66,914,66,120489,66,120547,66,120605,66,120663,66,120721,66,5108,66,5623,66,42192,66,66178,66,66209,66,66305,66,65347,99,8573,99,119836,99,119888,99,119940,99,119992,99,120044,99,120096,99,120148,99,120200,99,120252,99,120304,99,120356,99,120408,99,120460,99,7428,99,1010,99,11429,99,43951,99,66621,99,128844,67,71922,67,71913,67,65315,67,8557,67,8450,67,8493,67,119810,67,119862,67,119914,67,119966,67,120018,67,120174,67,120226,67,120278,67,120330,67,120382,67,120434,67,1017,67,11428,67,5087,67,42202,67,66210,67,66306,67,66581,67,66844,67,8574,100,8518,100,119837,100,119889,100,119941,100,119993,100,120045,100,120097,100,120149,100,120201,100,120253,100,120305,100,120357,100,120409,100,120461,100,1281,100,5095,100,5231,100,42194,100,8558,68,8517,68,119811,68,119863,68,119915,68,119967,68,120019,68,120071,68,120123,68,120175,68,120227,68,120279,68,120331,68,120383,68,120435,68,5024,68,5598,68,5610,68,42195,68,8494,101,65349,101,8495,101,8519,101,119838,101,119890,101,119942,101,120046,101,120098,101,120150,101,120202,101,120254,101,120306,101,120358,101,120410,101,120462,101,43826,101,1213,101,8959,69,65317,69,8496,69,119812,69,119864,69,119916,69,120020,69,120072,69,120124,69,120176,69,120228,69,120280,69,120332,69,120384,69,120436,69,917,69,120492,69,120550,69,120608,69,120666,69,120724,69,11577,69,5036,69,42224,69,71846,69,71854,69,66182,69,119839,102,119891,102,119943,102,119995,102,120047,102,120099,102,120151,102,120203,102,120255,102,120307,102,120359,102,120411,102,120463,102,43829,102,42905,102,383,102,7837,102,1412,102,119315,70,8497,70,119813,70,119865,70,119917,70,120021,70,120073,70,120125,70,120177,70,120229,70,120281,70,120333,70,120385,70,120437,70,42904,70,988,70,120778,70,5556,70,42205,70,71874,70,71842,70,66183,70,66213,70,66853,70,65351,103,8458,103,119840,103,119892,103,119944,103,120048,103,120100,103,120152,103,120204,103,120256,103,120308,103,120360,103,120412,103,120464,103,609,103,7555,103,397,103,1409,103,119814,71,119866,71,119918,71,119970,71,120022,71,120074,71,120126,71,120178,71,120230,71,120282,71,120334,71,120386,71,120438,71,1292,71,5056,71,5107,71,42198,71,65352,104,8462,104,119841,104,119945,104,119997,104,120049,104,120101,104,120153,104,120205,104,120257,104,120309,104,120361,104,120413,104,120465,104,1211,104,1392,104,5058,104,65320,72,8459,72,8460,72,8461,72,119815,72,119867,72,119919,72,120023,72,120179,72,120231,72,120283,72,120335,72,120387,72,120439,72,919,72,120494,72,120552,72,120610,72,120668,72,120726,72,11406,72,5051,72,5500,72,42215,72,66255,72,731,105,9075,105,65353,105,8560,105,8505,105,8520,105,119842,105,119894,105,119946,105,119998,105,120050,105,120102,105,120154,105,120206,105,120258,105,120310,105,120362,105,120414,105,120466,105,120484,105,618,105,617,105,953,105,8126,105,890,105,120522,105,120580,105,120638,105,120696,105,120754,105,1110,105,42567,105,1231,105,43893,105,5029,105,71875,105,65354,106,8521,106,119843,106,119895,106,119947,106,119999,106,120051,106,120103,106,120155,106,120207,106,120259,106,120311,106,120363,106,120415,106,120467,106,1011,106,1112,106,65322,74,119817,74,119869,74,119921,74,119973,74,120025,74,120077,74,120129,74,120181,74,120233,74,120285,74,120337,74,120389,74,120441,74,42930,74,895,74,1032,74,5035,74,5261,74,42201,74,119844,107,119896,107,119948,107,120000,107,120052,107,120104,107,120156,107,120208,107,120260,107,120312,107,120364,107,120416,107,120468,107,8490,75,65323,75,119818,75,119870,75,119922,75,119974,75,120026,75,120078,75,120130,75,120182,75,120234,75,120286,75,120338,75,120390,75,120442,75,922,75,120497,75,120555,75,120613,75,120671,75,120729,75,11412,75,5094,75,5845,75,42199,75,66840,75,1472,108,8739,73,9213,73,65512,73,1633,108,1777,73,66336,108,125127,108,120783,73,120793,73,120803,73,120813,73,120823,73,130033,73,65321,73,8544,73,8464,73,8465,73,119816,73,119868,73,119920,73,120024,73,120128,73,120180,73,120232,73,120284,73,120336,73,120388,73,120440,73,65356,108,8572,73,8467,108,119845,108,119897,108,119949,108,120001,108,120053,108,120105,73,120157,73,120209,73,120261,73,120313,73,120365,73,120417,73,120469,73,448,73,120496,73,120554,73,120612,73,120670,73,120728,73,11410,73,1030,73,1216,73,1493,108,1503,108,1575,108,126464,108,126592,108,65166,108,65165,108,1994,108,11599,73,5825,73,42226,73,93992,73,66186,124,66313,124,119338,76,8556,76,8466,76,119819,76,119871,76,119923,76,120027,76,120079,76,120131,76,120183,76,120235,76,120287,76,120339,76,120391,76,120443,76,11472,76,5086,76,5290,76,42209,76,93974,76,71843,76,71858,76,66587,76,66854,76,65325,77,8559,77,8499,77,119820,77,119872,77,119924,77,120028,77,120080,77,120132,77,120184,77,120236,77,120288,77,120340,77,120392,77,120444,77,924,77,120499,77,120557,77,120615,77,120673,77,120731,77,1018,77,11416,77,5047,77,5616,77,5846,77,42207,77,66224,77,66321,77,119847,110,119899,110,119951,110,120003,110,120055,110,120107,110,120159,110,120211,110,120263,110,120315,110,120367,110,120419,110,120471,110,1400,110,1404,110,65326,78,8469,78,119821,78,119873,78,119925,78,119977,78,120029,78,120081,78,120185,78,120237,78,120289,78,120341,78,120393,78,120445,78,925,78,120500,78,120558,78,120616,78,120674,78,120732,78,11418,78,42208,78,66835,78,3074,111,3202,111,3330,111,3458,111,2406,111,2662,111,2790,111,3046,111,3174,111,3302,111,3430,111,3664,111,3792,111,4160,111,1637,111,1781,111,65359,111,8500,111,119848,111,119900,111,119952,111,120056,111,120108,111,120160,111,120212,111,120264,111,120316,111,120368,111,120420,111,120472,111,7439,111,7441,111,43837,111,959,111,120528,111,120586,111,120644,111,120702,111,120760,111,963,111,120532,111,120590,111,120648,111,120706,111,120764,111,11423,111,4351,111,1413,111,1505,111,1607,111,126500,111,126564,111,126596,111,65259,111,65260,111,65258,111,65257,111,1726,111,64428,111,64429,111,64427,111,64426,111,1729,111,64424,111,64425,111,64423,111,64422,111,1749,111,3360,111,4125,111,66794,111,71880,111,71895,111,66604,111,1984,79,2534,79,2918,79,12295,79,70864,79,71904,79,120782,79,120792,79,120802,79,120812,79,120822,79,130032,79,65327,79,119822,79,119874,79,119926,79,119978,79,120030,79,120082,79,120134,79,120186,79,120238,79,120290,79,120342,79,120394,79,120446,79,927,79,120502,79,120560,79,120618,79,120676,79,120734,79,11422,79,1365,79,11604,79,4816,79,2848,79,66754,79,42227,79,71861,79,66194,79,66219,79,66564,79,66838,79,9076,112,65360,112,119849,112,119901,112,119953,112,120005,112,120057,112,120109,112,120161,112,120213,112,120265,112,120317,112,120369,112,120421,112,120473,112,961,112,120530,112,120544,112,120588,112,120602,112,120646,112,120660,112,120704,112,120718,112,120762,112,120776,112,11427,112,65328,80,8473,80,119823,80,119875,80,119927,80,119979,80,120031,80,120083,80,120187,80,120239,80,120291,80,120343,80,120395,80,120447,80,929,80,120504,80,120562,80,120620,80,120678,80,120736,80,11426,80,5090,80,5229,80,42193,80,66197,80,119850,113,119902,113,119954,113,120006,113,120058,113,120110,113,120162,113,120214,113,120266,113,120318,113,120370,113,120422,113,120474,113,1307,113,1379,113,1382,113,8474,81,119824,81,119876,81,119928,81,119980,81,120032,81,120084,81,120188,81,120240,81,120292,81,120344,81,120396,81,120448,81,11605,81,119851,114,119903,114,119955,114,120007,114,120059,114,120111,114,120163,114,120215,114,120267,114,120319,114,120371,114,120423,114,120475,114,43847,114,43848,114,7462,114,11397,114,43905,114,119318,82,8475,82,8476,82,8477,82,119825,82,119877,82,119929,82,120033,82,120189,82,120241,82,120293,82,120345,82,120397,82,120449,82,422,82,5025,82,5074,82,66740,82,5511,82,42211,82,94005,82,65363,115,119852,115,119904,115,119956,115,120008,115,120060,115,120112,115,120164,115,120216,115,120268,115,120320,115,120372,115,120424,115,120476,115,42801,115,445,115,1109,115,43946,115,71873,115,66632,115,65331,83,119826,83,119878,83,119930,83,119982,83,120034,83,120086,83,120138,83,120190,83,120242,83,120294,83,120346,83,120398,83,120450,83,1029,83,1359,83,5077,83,5082,83,42210,83,94010,83,66198,83,66592,83,119853,116,119905,116,119957,116,120009,116,120061,116,120113,116,120165,116,120217,116,120269,116,120321,116,120373,116,120425,116,120477,116,8868,84,10201,84,128872,84,65332,84,119827,84,119879,84,119931,84,119983,84,120035,84,120087,84,120139,84,120191,84,120243,84,120295,84,120347,84,120399,84,120451,84,932,84,120507,84,120565,84,120623,84,120681,84,120739,84,11430,84,5026,84,42196,84,93962,84,71868,84,66199,84,66225,84,66325,84,119854,117,119906,117,119958,117,120010,117,120062,117,120114,117,120166,117,120218,117,120270,117,120322,117,120374,117,120426,117,120478,117,42911,117,7452,117,43854,117,43858,117,651,117,965,117,120534,117,120592,117,120650,117,120708,117,120766,117,1405,117,66806,117,71896,117,8746,85,8899,85,119828,85,119880,85,119932,85,119984,85,120036,85,120088,85,120140,85,120192,85,120244,85,120296,85,120348,85,120400,85,120452,85,1357,85,4608,85,66766,85,5196,85,42228,85,94018,85,71864,85,8744,118,8897,118,65366,118,8564,118,119855,118,119907,118,119959,118,120011,118,120063,118,120115,118,120167,118,120219,118,120271,118,120323,118,120375,118,120427,118,120479,118,7456,118,957,118,120526,118,120584,118,120642,118,120700,118,120758,118,1141,118,1496,118,71430,118,43945,118,71872,118,119309,86,1639,86,1783,86,8548,86,119829,86,119881,86,119933,86,119985,86,120037,86,120089,86,120141,86,120193,86,120245,86,120297,86,120349,86,120401,86,120453,86,1140,86,11576,86,5081,86,5167,86,42719,86,42214,86,93960,86,71840,86,66845,86,623,119,119856,119,119908,119,119960,119,120012,119,120064,119,120116,119,120168,119,120220,119,120272,119,120324,119,120376,119,120428,119,120480,119,7457,119,1121,119,1309,119,1377,119,71434,119,71438,119,71439,119,43907,119,71919,87,71910,87,119830,87,119882,87,119934,87,119986,87,120038,87,120090,87,120142,87,120194,87,120246,87,120298,87,120350,87,120402,87,120454,87,1308,87,5043,87,5076,87,42218,87,5742,120,10539,120,10540,120,10799,120,65368,120,8569,120,119857,120,119909,120,119961,120,120013,120,120065,120,120117,120,120169,120,120221,120,120273,120,120325,120,120377,120,120429,120,120481,120,5441,120,5501,120,5741,88,9587,88,66338,88,71916,88,65336,88,8553,88,119831,88,119883,88,119935,88,119987,88,120039,88,120091,88,120143,88,120195,88,120247,88,120299,88,120351,88,120403,88,120455,88,42931,88,935,88,120510,88,120568,88,120626,88,120684,88,120742,88,11436,88,11613,88,5815,88,42219,88,66192,88,66228,88,66327,88,66855,88,611,121,7564,121,65369,121,119858,121,119910,121,119962,121,120014,121,120066,121,120118,121,120170,121,120222,121,120274,121,120326,121,120378,121,120430,121,120482,121,655,121,7935,121,43866,121,947,121,8509,121,120516,121,120574,121,120632,121,120690,121,120748,121,1199,121,4327,121,71900,121,65337,89,119832,89,119884,89,119936,89,119988,89,120040,89,120092,89,120144,89,120196,89,120248,89,120300,89,120352,89,120404,89,120456,89,933,89,978,89,120508,89,120566,89,120624,89,120682,89,120740,89,11432,89,1198,89,5033,89,5053,89,42220,89,94019,89,71844,89,66226,89,119859,122,119911,122,119963,122,120015,122,120067,122,120119,122,120171,122,120223,122,120275,122,120327,122,120379,122,120431,122,120483,122,7458,122,43923,122,71876,122,66293,90,71909,90,65338,90,8484,90,8488,90,119833,90,119885,90,119937,90,119989,90,120041,90,120197,90,120249,90,120301,90,120353,90,120405,90,120457,90,918,90,120493,90,120551,90,120609,90,120667,90,120725,90,5059,90,42204,90,71849,90,65282,34,65284,36,65285,37,65286,38,65290,42,65291,43,65294,46,65295,47,65296,48,65297,49,65298,50,65299,51,65300,52,65301,53,65302,54,65303,55,65304,56,65305,57,65308,60,65309,61,65310,62,65312,64,65316,68,65318,70,65319,71,65324,76,65329,81,65330,82,65333,85,65334,86,65335,87,65343,95,65346,98,65348,100,65350,102,65355,107,65357,109,65358,110,65361,113,65362,114,65364,116,65365,117,65367,119,65370,122,65371,123,65373,125,119846,109],"_default":[160,32,8211,45,65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"cs":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"de":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"es":[8211,45,65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"fr":[65374,126,65306,58,65281,33,8216,96,8245,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"it":[160,32,8211,45,65374,126,65306,58,65281,33,8216,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"ja":[8211,45,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65292,44,65307,59],"ko":[8211,45,65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"pl":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"pt-BR":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"qps-ploc":[160,32,8211,45,65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"ru":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,305,105,921,73,1009,112,215,120,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"tr":[160,32,8211,45,65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"zh-hans":[65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65288,40,65289,41],"zh-hant":[8211,45,65374,126,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65307,59]}'));
ee.cache = new Kr((e) => {
  function t(u) {
    const h = /* @__PURE__ */ new Map();
    for (let f = 0; f < u.length; f += 2)
      h.set(u[f], u[f + 1]);
    return h;
  }
  function n(u, h) {
    const f = new Map(u);
    for (const [d, m] of h)
      f.set(d, m);
    return f;
  }
  function r(u, h) {
    if (!u)
      return h;
    const f = /* @__PURE__ */ new Map();
    for (const [d, m] of u)
      h.has(d) && f.set(d, m);
    return f;
  }
  const s = hr.ambiguousCharacterData.value;
  let i = e.filter((u) => !u.startsWith("_") && u in s);
  i.length === 0 && (i = ["_default"]);
  let l;
  for (const u of i) {
    const h = t(s[u]);
    l = r(l, h);
  }
  const o = t(s._common), c = n(o, l);
  return new ee(c);
});
ee._locales = new cr(() => Object.keys(ee.ambiguousCharacterData.value).filter((e) => !e.startsWith("_")));
class de {
  static getRawData() {
    return JSON.parse("[9,10,11,12,13,32,127,160,173,847,1564,4447,4448,6068,6069,6155,6156,6157,6158,7355,7356,8192,8193,8194,8195,8196,8197,8198,8199,8200,8201,8202,8203,8204,8205,8206,8207,8234,8235,8236,8237,8238,8239,8287,8288,8289,8290,8291,8292,8293,8294,8295,8296,8297,8298,8299,8300,8301,8302,8303,10240,12288,12644,65024,65025,65026,65027,65028,65029,65030,65031,65032,65033,65034,65035,65036,65037,65038,65039,65279,65440,65520,65521,65522,65523,65524,65525,65526,65527,65528,65532,78844,119155,119156,119157,119158,119159,119160,119161,119162,917504,917505,917506,917507,917508,917509,917510,917511,917512,917513,917514,917515,917516,917517,917518,917519,917520,917521,917522,917523,917524,917525,917526,917527,917528,917529,917530,917531,917532,917533,917534,917535,917536,917537,917538,917539,917540,917541,917542,917543,917544,917545,917546,917547,917548,917549,917550,917551,917552,917553,917554,917555,917556,917557,917558,917559,917560,917561,917562,917563,917564,917565,917566,917567,917568,917569,917570,917571,917572,917573,917574,917575,917576,917577,917578,917579,917580,917581,917582,917583,917584,917585,917586,917587,917588,917589,917590,917591,917592,917593,917594,917595,917596,917597,917598,917599,917600,917601,917602,917603,917604,917605,917606,917607,917608,917609,917610,917611,917612,917613,917614,917615,917616,917617,917618,917619,917620,917621,917622,917623,917624,917625,917626,917627,917628,917629,917630,917631,917760,917761,917762,917763,917764,917765,917766,917767,917768,917769,917770,917771,917772,917773,917774,917775,917776,917777,917778,917779,917780,917781,917782,917783,917784,917785,917786,917787,917788,917789,917790,917791,917792,917793,917794,917795,917796,917797,917798,917799,917800,917801,917802,917803,917804,917805,917806,917807,917808,917809,917810,917811,917812,917813,917814,917815,917816,917817,917818,917819,917820,917821,917822,917823,917824,917825,917826,917827,917828,917829,917830,917831,917832,917833,917834,917835,917836,917837,917838,917839,917840,917841,917842,917843,917844,917845,917846,917847,917848,917849,917850,917851,917852,917853,917854,917855,917856,917857,917858,917859,917860,917861,917862,917863,917864,917865,917866,917867,917868,917869,917870,917871,917872,917873,917874,917875,917876,917877,917878,917879,917880,917881,917882,917883,917884,917885,917886,917887,917888,917889,917890,917891,917892,917893,917894,917895,917896,917897,917898,917899,917900,917901,917902,917903,917904,917905,917906,917907,917908,917909,917910,917911,917912,917913,917914,917915,917916,917917,917918,917919,917920,917921,917922,917923,917924,917925,917926,917927,917928,917929,917930,917931,917932,917933,917934,917935,917936,917937,917938,917939,917940,917941,917942,917943,917944,917945,917946,917947,917948,917949,917950,917951,917952,917953,917954,917955,917956,917957,917958,917959,917960,917961,917962,917963,917964,917965,917966,917967,917968,917969,917970,917971,917972,917973,917974,917975,917976,917977,917978,917979,917980,917981,917982,917983,917984,917985,917986,917987,917988,917989,917990,917991,917992,917993,917994,917995,917996,917997,917998,917999]");
  }
  static getData() {
    return this._data || (this._data = new Set(de.getRawData())), this._data;
  }
  static isInvisibleCharacter(t) {
    return de.getData().has(t);
  }
  static get codePoints() {
    return de.getData();
  }
}
de._data = void 0;
const us = "$initialize";
class cs {
  constructor(t, n, r, s) {
    this.vsWorker = t, this.req = n, this.method = r, this.args = s, this.type = 0;
  }
}
class qt {
  constructor(t, n, r, s) {
    this.vsWorker = t, this.seq = n, this.res = r, this.err = s, this.type = 1;
  }
}
class hs {
  constructor(t, n, r, s) {
    this.vsWorker = t, this.req = n, this.eventName = r, this.arg = s, this.type = 2;
  }
}
class fs {
  constructor(t, n, r) {
    this.vsWorker = t, this.req = n, this.event = r, this.type = 3;
  }
}
class ds {
  constructor(t, n) {
    this.vsWorker = t, this.req = n, this.type = 4;
  }
}
class ms {
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
      }, this._send(new cs(this._workerId, r, t, n));
    });
  }
  listen(t, n) {
    let r = null;
    const s = new se({
      onWillAddFirstListener: () => {
        r = String(++this._lastSentReq), this._pendingEmitters.set(r, s), this._send(new hs(this._workerId, r, t, n));
      },
      onDidRemoveLastListener: () => {
        this._pendingEmitters.delete(r), this._send(new ds(this._workerId, r)), r = null;
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
      this._send(new qt(this._workerId, n, s, void 0));
    }, (s) => {
      s.detail instanceof Error && (s.detail = It(s.detail)), this._send(new qt(this._workerId, n, void 0, It(s)));
    });
  }
  _handleSubscribeEventMessage(t) {
    const n = t.req, r = this._handler.handleEvent(t.eventName, t.arg)((s) => {
      this._send(new fs(this._workerId, n, s));
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
function dr(e) {
  return e[0] === "o" && e[1] === "n" && fr(e.charCodeAt(2));
}
function mr(e) {
  return /^onDynamic/.test(e) && fr(e.charCodeAt(9));
}
function gs(e, t, n) {
  const r = (l) => function() {
    const o = Array.prototype.slice.call(arguments, 0);
    return t(l, o);
  }, s = (l) => function(o) {
    return n(l, o);
  }, i = {};
  for (const l of e) {
    if (mr(l)) {
      i[l] = s(l);
      continue;
    }
    if (dr(l)) {
      i[l] = n(l, void 0);
      continue;
    }
    i[l] = r(l);
  }
  return i;
}
class bs {
  constructor(t, n) {
    this._requestHandlerFactory = n, this._requestHandler = null, this._protocol = new ms({
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
    if (t === us)
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
    if (mr(t)) {
      const r = this._requestHandler[t].call(this._requestHandler, n);
      if (typeof r != "function")
        throw new Error(`Missing dynamic event ${t} on request handler.`);
      return r;
    }
    if (dr(t)) {
      const r = this._requestHandler[t];
      if (typeof r != "function")
        throw new Error(`Missing event ${t} on request handler.`);
      return r;
    }
    throw new Error(`Malformed event name ${t}`);
  }
  initialize(t, n, r, s) {
    this._protocol.setWorkerId(t);
    const o = gs(s, (c, u) => this._protocol.sendMessage(c, u), (c, u) => this._protocol.listen(c, u));
    return this._requestHandlerFactory ? (this._requestHandler = this._requestHandlerFactory(o), Promise.resolve(dt(this._requestHandler))) : (n && (typeof n.baseUrl < "u" && delete n.baseUrl, typeof n.paths < "u" && typeof n.paths.vs < "u" && delete n.paths.vs, typeof n.trustedTypesPolicy !== void 0 && delete n.trustedTypesPolicy, n.catchError = !0, globalThis.require.config(n)), new Promise((c, u) => {
      const h = globalThis.require;
      h([r], (f) => {
        if (this._requestHandler = f.create(o), !this._requestHandler) {
          u(new Error("No RequestHandler!"));
          return;
        }
        c(dt(this._requestHandler));
      }, u);
    }));
  }
}
class he {
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
function Ht(e, t) {
  return (t << 5) - t + e | 0;
}
function _s(e, t) {
  t = Ht(149417, t);
  for (let n = 0, r = e.length; n < r; n++)
    t = Ht(e.charCodeAt(n), t);
  return t;
}
class Wt {
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
function xs(e, t, n) {
  return new fe(new Wt(e), new Wt(t)).ComputeDiff(n).changes;
}
class Ne {
  static Assert(t, n) {
    if (!t)
      throw new Error(n);
  }
}
class Se {
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
    for (let l = 0; l < i; l++)
      r[s + l] = t[n + l];
  }
  static Copy2(t, n, r, s, i) {
    for (let l = 0; l < i; l++)
      r[s + l] = t[n + l];
  }
}
class $t {
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
    (this.m_originalCount > 0 || this.m_modifiedCount > 0) && this.m_changes.push(new he(this.m_originalStart, this.m_originalCount, this.m_modifiedStart, this.m_modifiedCount)), this.m_originalCount = 0, this.m_modifiedCount = 0, this.m_originalStart = 1073741824, this.m_modifiedStart = 1073741824;
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
class fe {
  /**
   * Constructs the DiffFinder
   */
  constructor(t, n, r = null) {
    this.ContinueProcessingPredicate = r, this._originalSequence = t, this._modifiedSequence = n;
    const [s, i, l] = fe._getElements(t), [o, c, u] = fe._getElements(n);
    this._hasStrings = l && u, this._originalStringElements = s, this._originalElementsOrHash = i, this._modifiedStringElements = o, this._modifiedElementsOrHash = c, this.m_forwardHistory = [], this.m_reverseHistory = [];
  }
  static _isStringArray(t) {
    return t.length > 0 && typeof t[0] == "string";
  }
  static _getElements(t) {
    const n = t.getElements();
    if (fe._isStringArray(n)) {
      const r = new Int32Array(n.length);
      for (let s = 0, i = n.length; s < i; s++)
        r[s] = _s(n[s], 0);
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
    const r = fe._getStrictElement(this._originalSequence, t), s = fe._getStrictElement(this._modifiedSequence, n);
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
    const l = [!1];
    let o = this.ComputeDiffRecursive(t, n, r, s, l);
    return i && (o = this.PrettifyChanges(o)), {
      quitEarly: l[0],
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
      return r <= s ? (Ne.Assert(t === n + 1, "originalStart should only be one more than originalEnd"), f = [
        new he(t, 0, r, s - r + 1)
      ]) : t <= n ? (Ne.Assert(r === s + 1, "modifiedStart should only be one more than modifiedEnd"), f = [
        new he(t, n - t + 1, r, 0)
      ]) : (Ne.Assert(t === n + 1, "originalStart should only be one more than originalEnd"), Ne.Assert(r === s + 1, "modifiedStart should only be one more than modifiedEnd"), f = []), f;
    }
    const l = [0], o = [0], c = this.ComputeRecursionPoint(t, n, r, s, l, o, i), u = l[0], h = o[0];
    if (c !== null)
      return c;
    if (!i[0]) {
      const f = this.ComputeDiffRecursive(t, u, r, h, i);
      let d = [];
      return i[0] ? d = [
        new he(u + 1, n - (u + 1) + 1, h + 1, s - (h + 1) + 1)
      ] : d = this.ComputeDiffRecursive(u + 1, n, h + 1, s, i), this.ConcatenateChanges(f, d);
    }
    return [
      new he(t, n - t + 1, r, s - r + 1)
    ];
  }
  WALKTRACE(t, n, r, s, i, l, o, c, u, h, f, d, m, g, _, S, b, N) {
    let w = null, k = null, A = new $t(), E = n, C = r, R = m[0] - S[0] - s, L = -1073741824, p = this.m_forwardHistory.length - 1;
    do {
      const v = R + t;
      v === E || v < C && u[v - 1] < u[v + 1] ? (f = u[v + 1], g = f - R - s, f < L && A.MarkNextChange(), L = f, A.AddModifiedElement(f + 1, g), R = v + 1 - t) : (f = u[v - 1] + 1, g = f - R - s, f < L && A.MarkNextChange(), L = f - 1, A.AddOriginalElement(f, g + 1), R = v - 1 - t), p >= 0 && (u = this.m_forwardHistory[p], t = u[0], E = 1, C = u.length - 1);
    } while (--p >= -1);
    if (w = A.getReverseChanges(), N[0]) {
      let v = m[0] + 1, x = S[0] + 1;
      if (w !== null && w.length > 0) {
        const M = w[w.length - 1];
        v = Math.max(v, M.getOriginalEnd()), x = Math.max(x, M.getModifiedEnd());
      }
      k = [
        new he(v, d - v + 1, x, _ - x + 1)
      ];
    } else {
      A = new $t(), E = l, C = o, R = m[0] - S[0] - c, L = 1073741824, p = b ? this.m_reverseHistory.length - 1 : this.m_reverseHistory.length - 2;
      do {
        const v = R + i;
        v === E || v < C && h[v - 1] >= h[v + 1] ? (f = h[v + 1] - 1, g = f - R - c, f > L && A.MarkNextChange(), L = f + 1, A.AddOriginalElement(f + 1, g + 1), R = v + 1 - i) : (f = h[v - 1], g = f - R - c, f > L && A.MarkNextChange(), L = f, A.AddModifiedElement(f + 1, g + 1), R = v - 1 - i), p >= 0 && (h = this.m_reverseHistory[p], i = h[0], E = 1, C = h.length - 1);
      } while (--p >= -1);
      k = A.getChanges();
    }
    return this.ConcatenateChanges(w, k);
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
  ComputeRecursionPoint(t, n, r, s, i, l, o) {
    let c = 0, u = 0, h = 0, f = 0, d = 0, m = 0;
    t--, r--, i[0] = 0, l[0] = 0, this.m_forwardHistory = [], this.m_reverseHistory = [];
    const g = n - t + (s - r), _ = g + 1, S = new Int32Array(_), b = new Int32Array(_), N = s - r, w = n - t, k = t - r, A = n - s, C = (w - N) % 2 === 0;
    S[N] = t, b[w] = n, o[0] = !1;
    for (let R = 1; R <= g / 2 + 1; R++) {
      let L = 0, p = 0;
      h = this.ClipDiagonalBound(N - R, R, N, _), f = this.ClipDiagonalBound(N + R, R, N, _);
      for (let x = h; x <= f; x += 2) {
        x === h || x < f && S[x - 1] < S[x + 1] ? c = S[x + 1] : c = S[x - 1] + 1, u = c - (x - N) - k;
        const M = c;
        for (; c < n && u < s && this.ElementsAreEqual(c + 1, u + 1); )
          c++, u++;
        if (S[x] = c, c + u > L + p && (L = c, p = u), !C && Math.abs(x - w) <= R - 1 && c >= b[x])
          return i[0] = c, l[0] = u, M <= b[x] && 1447 > 0 && R <= 1447 + 1 ? this.WALKTRACE(N, h, f, k, w, d, m, A, S, b, c, n, i, u, s, l, C, o) : null;
      }
      const v = (L - t + (p - r) - R) / 2;
      if (this.ContinueProcessingPredicate !== null && !this.ContinueProcessingPredicate(L, v))
        return o[0] = !0, i[0] = L, l[0] = p, v > 0 && 1447 > 0 && R <= 1447 + 1 ? this.WALKTRACE(N, h, f, k, w, d, m, A, S, b, c, n, i, u, s, l, C, o) : (t++, r++, [
          new he(t, n - t + 1, r, s - r + 1)
        ]);
      d = this.ClipDiagonalBound(w - R, R, w, _), m = this.ClipDiagonalBound(w + R, R, w, _);
      for (let x = d; x <= m; x += 2) {
        x === d || x < m && b[x - 1] >= b[x + 1] ? c = b[x + 1] - 1 : c = b[x - 1], u = c - (x - w) - A;
        const M = c;
        for (; c > t && u > r && this.ElementsAreEqual(c, u); )
          c--, u--;
        if (b[x] = c, C && Math.abs(x - N) <= R && c <= S[x])
          return i[0] = c, l[0] = u, M >= S[x] && 1447 > 0 && R <= 1447 + 1 ? this.WALKTRACE(N, h, f, k, w, d, m, A, S, b, c, n, i, u, s, l, C, o) : null;
      }
      if (R <= 1447) {
        let x = new Int32Array(f - h + 2);
        x[0] = N - h + 1, Se.Copy2(S, h, x, 1, f - h + 1), this.m_forwardHistory.push(x), x = new Int32Array(m - d + 2), x[0] = w - d + 1, Se.Copy2(b, d, x, 1, m - d + 1), this.m_reverseHistory.push(x);
      }
    }
    return this.WALKTRACE(N, h, f, k, w, d, m, A, S, b, c, n, i, u, s, l, C, o);
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
      const r = t[n], s = n < t.length - 1 ? t[n + 1].originalStart : this._originalElementsOrHash.length, i = n < t.length - 1 ? t[n + 1].modifiedStart : this._modifiedElementsOrHash.length, l = r.originalLength > 0, o = r.modifiedLength > 0;
      for (; r.originalStart + r.originalLength < s && r.modifiedStart + r.modifiedLength < i && (!l || this.OriginalElementsAreEqual(r.originalStart, r.originalStart + r.originalLength)) && (!o || this.ModifiedElementsAreEqual(r.modifiedStart, r.modifiedStart + r.modifiedLength)); ) {
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
      const l = r.originalLength > 0, o = r.modifiedLength > 0;
      let c = 0, u = this._boundaryScore(r.originalStart, r.originalLength, r.modifiedStart, r.modifiedLength);
      for (let f = 1; ; f++) {
        const d = r.originalStart - f, m = r.modifiedStart - f;
        if (d < s || m < i || l && !this.OriginalElementsAreEqual(d, d + r.originalLength) || o && !this.ModifiedElementsAreEqual(m, m + r.modifiedLength))
          break;
        const _ = (d === s && m === i ? 5 : 0) + this._boundaryScore(d, r.originalLength, m, r.modifiedLength);
        _ > u && (u = _, c = f);
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
        const s = t[n - 1], i = t[n], l = i.originalStart - s.originalStart - s.originalLength, o = s.originalStart, c = i.originalStart + i.originalLength, u = c - o, h = s.modifiedStart, f = i.modifiedStart + i.modifiedLength, d = f - h;
        if (l < 5 && u < 20 && d < 20) {
          const m = this._findBetterContiguousSequence(o, u, h, d, l);
          if (m) {
            const [g, _] = m;
            (g !== s.originalStart + s.originalLength || _ !== s.modifiedStart + s.modifiedLength) && (s.originalLength = g - s.originalStart, s.modifiedLength = _ - s.modifiedStart, i.originalStart = g + l, i.modifiedStart = _ + l, i.originalLength = c - i.originalStart, i.modifiedLength = f - i.modifiedStart);
          }
        }
      }
    return t;
  }
  _findBetterContiguousSequence(t, n, r, s, i) {
    if (n < i || s < i)
      return null;
    const l = t + n - i + 1, o = r + s - i + 1;
    let c = 0, u = 0, h = 0;
    for (let f = t; f < l; f++)
      for (let d = r; d < o; d++) {
        const m = this._contiguousSequenceScore(f, d, i);
        m > 0 && m > c && (c = m, u = f, h = d);
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
    const i = this._OriginalRegionIsBoundary(t, n) ? 1 : 0, l = this._ModifiedRegionIsBoundary(r, s) ? 1 : 0;
    return i + l;
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
      return Se.Copy(t, 0, s, 0, t.length - 1), s[t.length - 1] = r[0], Se.Copy(n, 1, s, t.length, n.length - 1), s;
    } else {
      const s = new Array(t.length + n.length);
      return Se.Copy(t, 0, s, 0, t.length), Se.Copy(n, 0, s, t.length, n.length), s;
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
    if (Ne.Assert(t.originalStart <= n.originalStart, "Left change is not less than or equal to right change"), Ne.Assert(t.modifiedStart <= n.modifiedStart, "Left change is not less than or equal to right change"), t.originalStart + t.originalLength >= n.originalStart || t.modifiedStart + t.modifiedLength >= n.modifiedStart) {
      const s = t.originalStart;
      let i = t.originalLength;
      const l = t.modifiedStart;
      let o = t.modifiedLength;
      return t.originalStart + t.originalLength >= n.originalStart && (i = n.originalStart + n.originalLength - t.originalStart), t.modifiedStart + t.modifiedLength >= n.modifiedStart && (o = n.modifiedStart + n.modifiedLength - t.modifiedStart), r[0] = new he(s, i, l, o), !0;
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
    const i = r, l = s - r - 1, o = n % 2 === 0;
    if (t < 0) {
      const c = i % 2 === 0;
      return o === c ? 0 : 1;
    } else {
      const c = l % 2 === 0;
      return o === c ? s - 1 : s - 2;
    }
  }
}
let Me;
if (typeof re.vscode < "u" && typeof re.vscode.process < "u") {
  const e = re.vscode.process;
  Me = {
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
  typeof process < "u" ? Me = {
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
  } : Me = {
    // Supported
    get platform() {
      return qe ? "win32" : Yr ? "darwin" : "linux";
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
const Je = Me.cwd, ps = Me.env, vs = Me.platform, ws = 65, Ls = 97, Ns = 90, Ss = 122, me = 46, z = 47, Q = 92, oe = 58, As = 63;
class gr extends Error {
  constructor(t, n, r) {
    let s;
    typeof n == "string" && n.indexOf("not ") === 0 ? (s = "must not be", n = n.replace(/^not /, "")) : s = "must be";
    const i = t.indexOf(".") !== -1 ? "property" : "argument";
    let l = `The "${t}" ${i} ${s} of type ${n}`;
    l += `. Received type ${typeof r}`, super(l), this.code = "ERR_INVALID_ARG_TYPE";
  }
}
function Cs(e, t) {
  if (e === null || typeof e != "object")
    throw new gr(t, "Object", e);
}
function W(e, t) {
  if (typeof e != "string")
    throw new gr(t, "string", e);
}
const be = vs === "win32";
function F(e) {
  return e === z || e === Q;
}
function _t(e) {
  return e === z;
}
function ue(e) {
  return e >= ws && e <= Ns || e >= Ls && e <= Ss;
}
function Ze(e, t, n, r) {
  let s = "", i = 0, l = -1, o = 0, c = 0;
  for (let u = 0; u <= e.length; ++u) {
    if (u < e.length)
      c = e.charCodeAt(u);
    else {
      if (r(c))
        break;
      c = z;
    }
    if (r(c)) {
      if (!(l === u - 1 || o === 1))
        if (o === 2) {
          if (s.length < 2 || i !== 2 || s.charCodeAt(s.length - 1) !== me || s.charCodeAt(s.length - 2) !== me) {
            if (s.length > 2) {
              const h = s.lastIndexOf(n);
              h === -1 ? (s = "", i = 0) : (s = s.slice(0, h), i = s.length - 1 - s.lastIndexOf(n)), l = u, o = 0;
              continue;
            } else if (s.length !== 0) {
              s = "", i = 0, l = u, o = 0;
              continue;
            }
          }
          t && (s += s.length > 0 ? `${n}..` : "..", i = 2);
        } else
          s.length > 0 ? s += `${n}${e.slice(l + 1, u)}` : s = e.slice(l + 1, u), i = u - l - 1;
      l = u, o = 0;
    } else
      c === me && o !== -1 ? ++o : o = -1;
  }
  return s;
}
function br(e, t) {
  Cs(t, "pathObject");
  const n = t.dir || t.root, r = t.base || `${t.name || ""}${t.ext || ""}`;
  return n ? n === t.root ? `${n}${r}` : `${n}${e}${r}` : r;
}
const j = {
  // path.resolve([from ...], to)
  resolve(...e) {
    let t = "", n = "", r = !1;
    for (let s = e.length - 1; s >= -1; s--) {
      let i;
      if (s >= 0) {
        if (i = e[s], W(i, "path"), i.length === 0)
          continue;
      } else
        t.length === 0 ? i = Je() : (i = ps[`=${t}`] || Je(), (i === void 0 || i.slice(0, 2).toLowerCase() !== t.toLowerCase() && i.charCodeAt(2) === Q) && (i = `${t}\\`));
      const l = i.length;
      let o = 0, c = "", u = !1;
      const h = i.charCodeAt(0);
      if (l === 1)
        F(h) && (o = 1, u = !0);
      else if (F(h))
        if (u = !0, F(i.charCodeAt(1))) {
          let f = 2, d = f;
          for (; f < l && !F(i.charCodeAt(f)); )
            f++;
          if (f < l && f !== d) {
            const m = i.slice(d, f);
            for (d = f; f < l && F(i.charCodeAt(f)); )
              f++;
            if (f < l && f !== d) {
              for (d = f; f < l && !F(i.charCodeAt(f)); )
                f++;
              (f === l || f !== d) && (c = `\\\\${m}\\${i.slice(d, f)}`, o = f);
            }
          }
        } else
          o = 1;
      else
        ue(h) && i.charCodeAt(1) === oe && (c = i.slice(0, 2), o = 2, l > 2 && F(i.charCodeAt(2)) && (u = !0, o = 3));
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
    return n = Ze(n, !r, "\\", F), r ? `${t}\\${n}` : `${t}${n}` || ".";
  },
  normalize(e) {
    W(e, "path");
    const t = e.length;
    if (t === 0)
      return ".";
    let n = 0, r, s = !1;
    const i = e.charCodeAt(0);
    if (t === 1)
      return _t(i) ? "\\" : e;
    if (F(i))
      if (s = !0, F(e.charCodeAt(1))) {
        let o = 2, c = o;
        for (; o < t && !F(e.charCodeAt(o)); )
          o++;
        if (o < t && o !== c) {
          const u = e.slice(c, o);
          for (c = o; o < t && F(e.charCodeAt(o)); )
            o++;
          if (o < t && o !== c) {
            for (c = o; o < t && !F(e.charCodeAt(o)); )
              o++;
            if (o === t)
              return `\\\\${u}\\${e.slice(c)}\\`;
            o !== c && (r = `\\\\${u}\\${e.slice(c, o)}`, n = o);
          }
        }
      } else
        n = 1;
    else
      ue(i) && e.charCodeAt(1) === oe && (r = e.slice(0, 2), n = 2, t > 2 && F(e.charCodeAt(2)) && (s = !0, n = 3));
    let l = n < t ? Ze(e.slice(n), !s, "\\", F) : "";
    return l.length === 0 && !s && (l = "."), l.length > 0 && F(e.charCodeAt(t - 1)) && (l += "\\"), r === void 0 ? s ? `\\${l}` : l : s ? `${r}\\${l}` : `${r}${l}`;
  },
  isAbsolute(e) {
    W(e, "path");
    const t = e.length;
    if (t === 0)
      return !1;
    const n = e.charCodeAt(0);
    return F(n) || // Possible device root
    t > 2 && ue(n) && e.charCodeAt(1) === oe && F(e.charCodeAt(2));
  },
  join(...e) {
    if (e.length === 0)
      return ".";
    let t, n;
    for (let i = 0; i < e.length; ++i) {
      const l = e[i];
      W(l, "path"), l.length > 0 && (t === void 0 ? t = n = l : t += `\\${l}`);
    }
    if (t === void 0)
      return ".";
    let r = !0, s = 0;
    if (typeof n == "string" && F(n.charCodeAt(0))) {
      ++s;
      const i = n.length;
      i > 1 && F(n.charCodeAt(1)) && (++s, i > 2 && (F(n.charCodeAt(2)) ? ++s : r = !1));
    }
    if (r) {
      for (; s < t.length && F(t.charCodeAt(s)); )
        s++;
      s >= 2 && (t = `\\${t.slice(s)}`);
    }
    return j.normalize(t);
  },
  // It will solve the relative path from `from` to `to`, for instance:
  //  from = 'C:\\orandea\\test\\aaa'
  //  to = 'C:\\orandea\\impl\\bbb'
  // The output of the function should be: '..\\..\\impl\\bbb'
  relative(e, t) {
    if (W(e, "from"), W(t, "to"), e === t)
      return "";
    const n = j.resolve(e), r = j.resolve(t);
    if (n === r || (e = n.toLowerCase(), t = r.toLowerCase(), e === t))
      return "";
    let s = 0;
    for (; s < e.length && e.charCodeAt(s) === Q; )
      s++;
    let i = e.length;
    for (; i - 1 > s && e.charCodeAt(i - 1) === Q; )
      i--;
    const l = i - s;
    let o = 0;
    for (; o < t.length && t.charCodeAt(o) === Q; )
      o++;
    let c = t.length;
    for (; c - 1 > o && t.charCodeAt(c - 1) === Q; )
      c--;
    const u = c - o, h = l < u ? l : u;
    let f = -1, d = 0;
    for (; d < h; d++) {
      const g = e.charCodeAt(s + d);
      if (g !== t.charCodeAt(o + d))
        break;
      g === Q && (f = d);
    }
    if (d !== h) {
      if (f === -1)
        return r;
    } else {
      if (u > h) {
        if (t.charCodeAt(o + d) === Q)
          return r.slice(o + d + 1);
        if (d === 2)
          return r.slice(o + d);
      }
      l > h && (e.charCodeAt(s + d) === Q ? f = d : d === 2 && (f = 3)), f === -1 && (f = 0);
    }
    let m = "";
    for (d = s + f + 1; d <= i; ++d)
      (d === i || e.charCodeAt(d) === Q) && (m += m.length === 0 ? ".." : "\\..");
    return o += f, m.length > 0 ? `${m}${r.slice(o, c)}` : (r.charCodeAt(o) === Q && ++o, r.slice(o, c));
  },
  toNamespacedPath(e) {
    if (typeof e != "string" || e.length === 0)
      return e;
    const t = j.resolve(e);
    if (t.length <= 2)
      return e;
    if (t.charCodeAt(0) === Q) {
      if (t.charCodeAt(1) === Q) {
        const n = t.charCodeAt(2);
        if (n !== As && n !== me)
          return `\\\\?\\UNC\\${t.slice(2)}`;
      }
    } else if (ue(t.charCodeAt(0)) && t.charCodeAt(1) === oe && t.charCodeAt(2) === Q)
      return `\\\\?\\${t}`;
    return e;
  },
  dirname(e) {
    W(e, "path");
    const t = e.length;
    if (t === 0)
      return ".";
    let n = -1, r = 0;
    const s = e.charCodeAt(0);
    if (t === 1)
      return F(s) ? e : ".";
    if (F(s)) {
      if (n = r = 1, F(e.charCodeAt(1))) {
        let o = 2, c = o;
        for (; o < t && !F(e.charCodeAt(o)); )
          o++;
        if (o < t && o !== c) {
          for (c = o; o < t && F(e.charCodeAt(o)); )
            o++;
          if (o < t && o !== c) {
            for (c = o; o < t && !F(e.charCodeAt(o)); )
              o++;
            if (o === t)
              return e;
            o !== c && (n = r = o + 1);
          }
        }
      }
    } else
      ue(s) && e.charCodeAt(1) === oe && (n = t > 2 && F(e.charCodeAt(2)) ? 3 : 2, r = n);
    let i = -1, l = !0;
    for (let o = t - 1; o >= r; --o)
      if (F(e.charCodeAt(o))) {
        if (!l) {
          i = o;
          break;
        }
      } else
        l = !1;
    if (i === -1) {
      if (n === -1)
        return ".";
      i = n;
    }
    return e.slice(0, i);
  },
  basename(e, t) {
    t !== void 0 && W(t, "ext"), W(e, "path");
    let n = 0, r = -1, s = !0, i;
    if (e.length >= 2 && ue(e.charCodeAt(0)) && e.charCodeAt(1) === oe && (n = 2), t !== void 0 && t.length > 0 && t.length <= e.length) {
      if (t === e)
        return "";
      let l = t.length - 1, o = -1;
      for (i = e.length - 1; i >= n; --i) {
        const c = e.charCodeAt(i);
        if (F(c)) {
          if (!s) {
            n = i + 1;
            break;
          }
        } else
          o === -1 && (s = !1, o = i + 1), l >= 0 && (c === t.charCodeAt(l) ? --l === -1 && (r = i) : (l = -1, r = o));
      }
      return n === r ? r = o : r === -1 && (r = e.length), e.slice(n, r);
    }
    for (i = e.length - 1; i >= n; --i)
      if (F(e.charCodeAt(i))) {
        if (!s) {
          n = i + 1;
          break;
        }
      } else
        r === -1 && (s = !1, r = i + 1);
    return r === -1 ? "" : e.slice(n, r);
  },
  extname(e) {
    W(e, "path");
    let t = 0, n = -1, r = 0, s = -1, i = !0, l = 0;
    e.length >= 2 && e.charCodeAt(1) === oe && ue(e.charCodeAt(0)) && (t = r = 2);
    for (let o = e.length - 1; o >= t; --o) {
      const c = e.charCodeAt(o);
      if (F(c)) {
        if (!i) {
          r = o + 1;
          break;
        }
        continue;
      }
      s === -1 && (i = !1, s = o + 1), c === me ? n === -1 ? n = o : l !== 1 && (l = 1) : n !== -1 && (l = -1);
    }
    return n === -1 || s === -1 || // We saw a non-dot character immediately before the dot
    l === 0 || // The (right-most) trimmed path component is exactly '..'
    l === 1 && n === s - 1 && n === r + 1 ? "" : e.slice(n, s);
  },
  format: br.bind(null, "\\"),
  parse(e) {
    W(e, "path");
    const t = { root: "", dir: "", base: "", ext: "", name: "" };
    if (e.length === 0)
      return t;
    const n = e.length;
    let r = 0, s = e.charCodeAt(0);
    if (n === 1)
      return F(s) ? (t.root = t.dir = e, t) : (t.base = t.name = e, t);
    if (F(s)) {
      if (r = 1, F(e.charCodeAt(1))) {
        let f = 2, d = f;
        for (; f < n && !F(e.charCodeAt(f)); )
          f++;
        if (f < n && f !== d) {
          for (d = f; f < n && F(e.charCodeAt(f)); )
            f++;
          if (f < n && f !== d) {
            for (d = f; f < n && !F(e.charCodeAt(f)); )
              f++;
            f === n ? r = f : f !== d && (r = f + 1);
          }
        }
      }
    } else if (ue(s) && e.charCodeAt(1) === oe) {
      if (n <= 2)
        return t.root = t.dir = e, t;
      if (r = 2, F(e.charCodeAt(2))) {
        if (n === 3)
          return t.root = t.dir = e, t;
        r = 3;
      }
    }
    r > 0 && (t.root = e.slice(0, r));
    let i = -1, l = r, o = -1, c = !0, u = e.length - 1, h = 0;
    for (; u >= r; --u) {
      if (s = e.charCodeAt(u), F(s)) {
        if (!c) {
          l = u + 1;
          break;
        }
        continue;
      }
      o === -1 && (c = !1, o = u + 1), s === me ? i === -1 ? i = u : h !== 1 && (h = 1) : i !== -1 && (h = -1);
    }
    return o !== -1 && (i === -1 || // We saw a non-dot character immediately before the dot
    h === 0 || // The (right-most) trimmed path component is exactly '..'
    h === 1 && i === o - 1 && i === l + 1 ? t.base = t.name = e.slice(l, o) : (t.name = e.slice(l, i), t.base = e.slice(l, o), t.ext = e.slice(i, o))), l > 0 && l !== r ? t.dir = e.slice(0, l - 1) : t.dir = t.root, t;
  },
  sep: "\\",
  delimiter: ";",
  win32: null,
  posix: null
}, Rs = (() => {
  if (be) {
    const e = /\\/g;
    return () => {
      const t = Je().replace(e, "/");
      return t.slice(t.indexOf("/"));
    };
  }
  return () => Je();
})(), X = {
  // path.resolve([from ...], to)
  resolve(...e) {
    let t = "", n = !1;
    for (let r = e.length - 1; r >= -1 && !n; r--) {
      const s = r >= 0 ? e[r] : Rs();
      W(s, "path"), s.length !== 0 && (t = `${s}/${t}`, n = s.charCodeAt(0) === z);
    }
    return t = Ze(t, !n, "/", _t), n ? `/${t}` : t.length > 0 ? t : ".";
  },
  normalize(e) {
    if (W(e, "path"), e.length === 0)
      return ".";
    const t = e.charCodeAt(0) === z, n = e.charCodeAt(e.length - 1) === z;
    return e = Ze(e, !t, "/", _t), e.length === 0 ? t ? "/" : n ? "./" : "." : (n && (e += "/"), t ? `/${e}` : e);
  },
  isAbsolute(e) {
    return W(e, "path"), e.length > 0 && e.charCodeAt(0) === z;
  },
  join(...e) {
    if (e.length === 0)
      return ".";
    let t;
    for (let n = 0; n < e.length; ++n) {
      const r = e[n];
      W(r, "path"), r.length > 0 && (t === void 0 ? t = r : t += `/${r}`);
    }
    return t === void 0 ? "." : X.normalize(t);
  },
  relative(e, t) {
    if (W(e, "from"), W(t, "to"), e === t || (e = X.resolve(e), t = X.resolve(t), e === t))
      return "";
    const n = 1, r = e.length, s = r - n, i = 1, l = t.length - i, o = s < l ? s : l;
    let c = -1, u = 0;
    for (; u < o; u++) {
      const f = e.charCodeAt(n + u);
      if (f !== t.charCodeAt(i + u))
        break;
      f === z && (c = u);
    }
    if (u === o)
      if (l > o) {
        if (t.charCodeAt(i + u) === z)
          return t.slice(i + u + 1);
        if (u === 0)
          return t.slice(i + u);
      } else
        s > o && (e.charCodeAt(n + u) === z ? c = u : u === 0 && (c = 0));
    let h = "";
    for (u = n + c + 1; u <= r; ++u)
      (u === r || e.charCodeAt(u) === z) && (h += h.length === 0 ? ".." : "/..");
    return `${h}${t.slice(i + c)}`;
  },
  toNamespacedPath(e) {
    return e;
  },
  dirname(e) {
    if (W(e, "path"), e.length === 0)
      return ".";
    const t = e.charCodeAt(0) === z;
    let n = -1, r = !0;
    for (let s = e.length - 1; s >= 1; --s)
      if (e.charCodeAt(s) === z) {
        if (!r) {
          n = s;
          break;
        }
      } else
        r = !1;
    return n === -1 ? t ? "/" : "." : t && n === 1 ? "//" : e.slice(0, n);
  },
  basename(e, t) {
    t !== void 0 && W(t, "ext"), W(e, "path");
    let n = 0, r = -1, s = !0, i;
    if (t !== void 0 && t.length > 0 && t.length <= e.length) {
      if (t === e)
        return "";
      let l = t.length - 1, o = -1;
      for (i = e.length - 1; i >= 0; --i) {
        const c = e.charCodeAt(i);
        if (c === z) {
          if (!s) {
            n = i + 1;
            break;
          }
        } else
          o === -1 && (s = !1, o = i + 1), l >= 0 && (c === t.charCodeAt(l) ? --l === -1 && (r = i) : (l = -1, r = o));
      }
      return n === r ? r = o : r === -1 && (r = e.length), e.slice(n, r);
    }
    for (i = e.length - 1; i >= 0; --i)
      if (e.charCodeAt(i) === z) {
        if (!s) {
          n = i + 1;
          break;
        }
      } else
        r === -1 && (s = !1, r = i + 1);
    return r === -1 ? "" : e.slice(n, r);
  },
  extname(e) {
    W(e, "path");
    let t = -1, n = 0, r = -1, s = !0, i = 0;
    for (let l = e.length - 1; l >= 0; --l) {
      const o = e.charCodeAt(l);
      if (o === z) {
        if (!s) {
          n = l + 1;
          break;
        }
        continue;
      }
      r === -1 && (s = !1, r = l + 1), o === me ? t === -1 ? t = l : i !== 1 && (i = 1) : t !== -1 && (i = -1);
    }
    return t === -1 || r === -1 || // We saw a non-dot character immediately before the dot
    i === 0 || // The (right-most) trimmed path component is exactly '..'
    i === 1 && t === r - 1 && t === n + 1 ? "" : e.slice(t, r);
  },
  format: br.bind(null, "/"),
  parse(e) {
    W(e, "path");
    const t = { root: "", dir: "", base: "", ext: "", name: "" };
    if (e.length === 0)
      return t;
    const n = e.charCodeAt(0) === z;
    let r;
    n ? (t.root = "/", r = 1) : r = 0;
    let s = -1, i = 0, l = -1, o = !0, c = e.length - 1, u = 0;
    for (; c >= r; --c) {
      const h = e.charCodeAt(c);
      if (h === z) {
        if (!o) {
          i = c + 1;
          break;
        }
        continue;
      }
      l === -1 && (o = !1, l = c + 1), h === me ? s === -1 ? s = c : u !== 1 && (u = 1) : s !== -1 && (u = -1);
    }
    if (l !== -1) {
      const h = i === 0 && n ? 1 : i;
      s === -1 || // We saw a non-dot character immediately before the dot
      u === 0 || // The (right-most) trimmed path component is exactly '..'
      u === 1 && s === l - 1 && s === i + 1 ? t.base = t.name = e.slice(h, l) : (t.name = e.slice(h, s), t.base = e.slice(h, l), t.ext = e.slice(s, l));
    }
    return i > 0 ? t.dir = e.slice(0, i - 1) : n && (t.dir = "/"), t;
  },
  sep: "/",
  delimiter: ":",
  win32: null,
  posix: null
};
X.win32 = j.win32 = j;
X.posix = j.posix = X;
be ? j.normalize : X.normalize;
be ? j.resolve : X.resolve;
be ? j.relative : X.relative;
be ? j.dirname : X.dirname;
be ? j.basename : X.basename;
be ? j.extname : X.extname;
be ? j.sep : X.sep;
const ys = /^\w[\w\d+.-]*$/, Ms = /^\//, ks = /^\/\//;
function Es(e, t) {
  if (!e.scheme && t)
    throw new Error(`[UriError]: Scheme is missing: {scheme: "", authority: "${e.authority}", path: "${e.path}", query: "${e.query}", fragment: "${e.fragment}"}`);
  if (e.scheme && !ys.test(e.scheme))
    throw new Error("[UriError]: Scheme contains illegal characters.");
  if (e.path) {
    if (e.authority) {
      if (!Ms.test(e.path))
        throw new Error('[UriError]: If a URI contains an authority component, then the path component must either be empty or begin with a slash ("/") character');
    } else if (ks.test(e.path))
      throw new Error('[UriError]: If a URI does not contain an authority component, then the path cannot begin with two slash characters ("//")');
  }
}
function Fs(e, t) {
  return !e && !t ? "file" : e;
}
function Ps(e, t) {
  switch (e) {
    case "https":
    case "http":
    case "file":
      t ? t[0] !== ne && (t = ne + t) : t = ne;
      break;
  }
  return t;
}
const I = "", ne = "/", Ds = /^(([^:/?#]+?):)?(\/\/([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?/;
class xe {
  static isUri(t) {
    return t instanceof xe ? !0 : t ? typeof t.authority == "string" && typeof t.fragment == "string" && typeof t.path == "string" && typeof t.query == "string" && typeof t.scheme == "string" && typeof t.fsPath == "string" && typeof t.with == "function" && typeof t.toString == "function" : !1;
  }
  /**
   * @internal
   */
  constructor(t, n, r, s, i, l = !1) {
    typeof t == "object" ? (this.scheme = t.scheme || I, this.authority = t.authority || I, this.path = t.path || I, this.query = t.query || I, this.fragment = t.fragment || I) : (this.scheme = Fs(t, l), this.authority = n || I, this.path = Ps(this.scheme, r || I), this.query = s || I, this.fragment = i || I, Es(this, l));
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
    return xt(this, !1);
  }
  // ---- modify to new -------------------------
  with(t) {
    if (!t)
      return this;
    let { scheme: n, authority: r, path: s, query: i, fragment: l } = t;
    return n === void 0 ? n = this.scheme : n === null && (n = I), r === void 0 ? r = this.authority : r === null && (r = I), s === void 0 ? s = this.path : s === null && (s = I), i === void 0 ? i = this.query : i === null && (i = I), l === void 0 ? l = this.fragment : l === null && (l = I), n === this.scheme && r === this.authority && s === this.path && i === this.query && l === this.fragment ? this : new Ae(n, r, s, i, l);
  }
  // ---- parse & validate ------------------------
  /**
   * Creates a new URI from a string, e.g. `http://www.example.com/some/path`,
   * `file:///usr/home`, or `scheme:with/path`.
   *
   * @param value A string which represents an URI (see `URI#toString`).
   */
  static parse(t, n = !1) {
    const r = Ds.exec(t);
    return r ? new Ae(r[2] || I, je(r[4] || I), je(r[5] || I), je(r[7] || I), je(r[9] || I), n) : new Ae(I, I, I, I, I);
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
    let n = I;
    if (qe && (t = t.replace(/\\/g, ne)), t[0] === ne && t[1] === ne) {
      const r = t.indexOf(ne, 2);
      r === -1 ? (n = t.substring(2), t = ne) : (n = t.substring(2, r), t = t.substring(r) || ne);
    }
    return new Ae("file", n, t, I, I);
  }
  /**
   * Creates new URI from uri components.
   *
   * Unless `strict` is `true` the scheme is defaults to be `file`. This function performs
   * validation and should be used for untrusted uri components retrieved from storage,
   * user input, command arguments etc
   */
  static from(t, n) {
    return new Ae(t.scheme, t.authority, t.path, t.query, t.fragment, n);
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
    return qe && t.scheme === "file" ? r = xe.file(j.join(xt(t, !0), ...n)).path : r = X.join(t.path, ...n), t.with({ path: r });
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
    return pt(this, t);
  }
  toJSON() {
    return this;
  }
  static revive(t) {
    var n, r;
    if (t) {
      if (t instanceof xe)
        return t;
      {
        const s = new Ae(t);
        return s._formatted = (n = t.external) !== null && n !== void 0 ? n : null, s._fsPath = t._sep === _r && (r = t.fsPath) !== null && r !== void 0 ? r : null, s;
      }
    } else
      return t;
  }
}
const _r = qe ? 1 : void 0;
class Ae extends xe {
  constructor() {
    super(...arguments), this._formatted = null, this._fsPath = null;
  }
  get fsPath() {
    return this._fsPath || (this._fsPath = xt(this, !1)), this._fsPath;
  }
  toString(t = !1) {
    return t ? pt(this, !0) : (this._formatted || (this._formatted = pt(this, !1)), this._formatted);
  }
  toJSON() {
    const t = {
      $mid: 1
      /* MarshalledId.Uri */
    };
    return this._fsPath && (t.fsPath = this._fsPath, t._sep = _r), this._formatted && (t.external = this._formatted), this.path && (t.path = this.path), this.scheme && (t.scheme = this.scheme), this.authority && (t.authority = this.authority), this.query && (t.query = this.query), this.fragment && (t.fragment = this.fragment), t;
  }
}
const xr = {
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
function zt(e, t, n) {
  let r, s = -1;
  for (let i = 0; i < e.length; i++) {
    const l = e.charCodeAt(i);
    if (l >= 97 && l <= 122 || l >= 65 && l <= 90 || l >= 48 && l <= 57 || l === 45 || l === 46 || l === 95 || l === 126 || t && l === 47 || n && l === 91 || n && l === 93 || n && l === 58)
      s !== -1 && (r += encodeURIComponent(e.substring(s, i)), s = -1), r !== void 0 && (r += e.charAt(i));
    else {
      r === void 0 && (r = e.substr(0, i));
      const o = xr[l];
      o !== void 0 ? (s !== -1 && (r += encodeURIComponent(e.substring(s, i)), s = -1), r += o) : s === -1 && (s = i);
    }
  }
  return s !== -1 && (r += encodeURIComponent(e.substring(s))), r !== void 0 ? r : e;
}
function Vs(e) {
  let t;
  for (let n = 0; n < e.length; n++) {
    const r = e.charCodeAt(n);
    r === 35 || r === 63 ? (t === void 0 && (t = e.substr(0, n)), t += xr[r]) : t !== void 0 && (t += e[n]);
  }
  return t !== void 0 ? t : e;
}
function xt(e, t) {
  let n;
  return e.authority && e.path.length > 1 && e.scheme === "file" ? n = `//${e.authority}${e.path}` : e.path.charCodeAt(0) === 47 && (e.path.charCodeAt(1) >= 65 && e.path.charCodeAt(1) <= 90 || e.path.charCodeAt(1) >= 97 && e.path.charCodeAt(1) <= 122) && e.path.charCodeAt(2) === 58 ? t ? n = e.path.substr(1) : n = e.path[1].toLowerCase() + e.path.substr(2) : n = e.path, qe && (n = n.replace(/\//g, "\\")), n;
}
function pt(e, t) {
  const n = t ? Vs : zt;
  let r = "", { scheme: s, authority: i, path: l, query: o, fragment: c } = e;
  if (s && (r += s, r += ":"), (i || s === "file") && (r += ne, r += ne), i) {
    let u = i.indexOf("@");
    if (u !== -1) {
      const h = i.substr(0, u);
      i = i.substr(u + 1), u = h.lastIndexOf(":"), u === -1 ? r += n(h, !1, !1) : (r += n(h.substr(0, u), !1, !1), r += ":", r += n(h.substr(u + 1), !1, !0)), r += "@";
    }
    i = i.toLowerCase(), u = i.lastIndexOf(":"), u === -1 ? r += n(i, !1, !0) : (r += n(i.substr(0, u), !1, !0), r += i.substr(u));
  }
  if (l) {
    if (l.length >= 3 && l.charCodeAt(0) === 47 && l.charCodeAt(2) === 58) {
      const u = l.charCodeAt(1);
      u >= 65 && u <= 90 && (l = `/${String.fromCharCode(u + 32)}:${l.substr(3)}`);
    } else if (l.length >= 2 && l.charCodeAt(1) === 58) {
      const u = l.charCodeAt(0);
      u >= 65 && u <= 90 && (l = `${String.fromCharCode(u + 32)}:${l.substr(2)}`);
    }
    r += n(l, !0, !1);
  }
  return o && (r += "?", r += n(o, !1, !1)), c && (r += "#", r += t ? c : zt(c, !1, !1)), r;
}
function pr(e) {
  try {
    return decodeURIComponent(e);
  } catch {
    return e.length > 3 ? e.substr(0, 3) + pr(e.substr(3)) : e;
  }
}
const Gt = /(%[0-9A-Za-z][0-9A-Za-z])+/g;
function je(e) {
  return e.match(Gt) ? e.replace(Gt, (t) => pr(t)) : e;
}
class O {
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
    return t === this.lineNumber && n === this.column ? this : new O(t, n);
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
    return O.equals(this, t);
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
    return O.isBefore(this, t);
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
    return O.isBeforeOrEqual(this, t);
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
      const i = t.column | 0, l = n.column | 0;
      return i - l;
    }
    return r - s;
  }
  /**
   * Clone this position.
   */
  clone() {
    return new O(this.lineNumber, this.column);
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
    return new O(t.lineNumber, t.column);
  }
  /**
   * Test if `obj` is an `IPosition`.
   */
  static isIPosition(t) {
    return t && typeof t.lineNumber == "number" && typeof t.column == "number";
  }
}
class P {
  constructor(t, n, r, s) {
    t > r || t === r && n > s ? (this.startLineNumber = r, this.startColumn = s, this.endLineNumber = t, this.endColumn = n) : (this.startLineNumber = t, this.startColumn = n, this.endLineNumber = r, this.endColumn = s);
  }
  /**
   * Test if this range is empty.
   */
  isEmpty() {
    return P.isEmpty(this);
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
    return P.containsPosition(this, t);
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
    return P.containsRange(this, t);
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
    return P.strictContainsRange(this, t);
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
    return P.plusRange(this, t);
  }
  /**
   * A reunion of the two ranges.
   * The smallest position will be used as the start point, and the largest one as the end point.
   */
  static plusRange(t, n) {
    let r, s, i, l;
    return n.startLineNumber < t.startLineNumber ? (r = n.startLineNumber, s = n.startColumn) : n.startLineNumber === t.startLineNumber ? (r = n.startLineNumber, s = Math.min(n.startColumn, t.startColumn)) : (r = t.startLineNumber, s = t.startColumn), n.endLineNumber > t.endLineNumber ? (i = n.endLineNumber, l = n.endColumn) : n.endLineNumber === t.endLineNumber ? (i = n.endLineNumber, l = Math.max(n.endColumn, t.endColumn)) : (i = t.endLineNumber, l = t.endColumn), new P(r, s, i, l);
  }
  /**
   * A intersection of the two ranges.
   */
  intersectRanges(t) {
    return P.intersectRanges(this, t);
  }
  /**
   * A intersection of the two ranges.
   */
  static intersectRanges(t, n) {
    let r = t.startLineNumber, s = t.startColumn, i = t.endLineNumber, l = t.endColumn;
    const o = n.startLineNumber, c = n.startColumn, u = n.endLineNumber, h = n.endColumn;
    return r < o ? (r = o, s = c) : r === o && (s = Math.max(s, c)), i > u ? (i = u, l = h) : i === u && (l = Math.min(l, h)), r > i || r === i && s > l ? null : new P(r, s, i, l);
  }
  /**
   * Test if this range equals other.
   */
  equalsRange(t) {
    return P.equalsRange(this, t);
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
    return P.getEndPosition(this);
  }
  /**
   * Return the end position (which will be after or equal to the start position)
   */
  static getEndPosition(t) {
    return new O(t.endLineNumber, t.endColumn);
  }
  /**
   * Return the start position (which will be before or equal to the end position)
   */
  getStartPosition() {
    return P.getStartPosition(this);
  }
  /**
   * Return the start position (which will be before or equal to the end position)
   */
  static getStartPosition(t) {
    return new O(t.startLineNumber, t.startColumn);
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
    return new P(this.startLineNumber, this.startColumn, t, n);
  }
  /**
   * Create a new range using this range's end position, and using startLineNumber and startColumn as the start position.
   */
  setStartPosition(t, n) {
    return new P(t, n, this.endLineNumber, this.endColumn);
  }
  /**
   * Create a new empty range using this range's start position.
   */
  collapseToStart() {
    return P.collapseToStart(this);
  }
  /**
   * Create a new empty range using this range's start position.
   */
  static collapseToStart(t) {
    return new P(t.startLineNumber, t.startColumn, t.startLineNumber, t.startColumn);
  }
  /**
   * Create a new empty range using this range's end position.
   */
  collapseToEnd() {
    return P.collapseToEnd(this);
  }
  /**
   * Create a new empty range using this range's end position.
   */
  static collapseToEnd(t) {
    return new P(t.endLineNumber, t.endColumn, t.endLineNumber, t.endColumn);
  }
  /**
   * Moves the range by the given amount of lines.
   */
  delta(t) {
    return new P(this.startLineNumber + t, this.startColumn, this.endLineNumber + t, this.endColumn);
  }
  // ---
  static fromPositions(t, n = t) {
    return new P(t.lineNumber, t.column, n.lineNumber, n.column);
  }
  static lift(t) {
    return t ? new P(t.startLineNumber, t.startColumn, t.endLineNumber, t.endColumn) : null;
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
      const i = t.startLineNumber | 0, l = n.startLineNumber | 0;
      if (i === l) {
        const o = t.startColumn | 0, c = n.startColumn | 0;
        if (o === c) {
          const u = t.endLineNumber | 0, h = n.endLineNumber | 0;
          if (u === h) {
            const f = t.endColumn | 0, d = n.endColumn | 0;
            return f - d;
          }
          return u - h;
        }
        return o - c;
      }
      return i - l;
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
var Ot;
(function(e) {
  function t(s) {
    return s < 0;
  }
  e.isLessThan = t;
  function n(s) {
    return s > 0;
  }
  e.isGreaterThan = n;
  function r(s) {
    return s === 0;
  }
  e.isNeitherLessOrGreaterThan = r, e.greaterThan = 1, e.lessThan = -1, e.neitherLessOrGreaterThan = 0;
})(Ot || (Ot = {}));
function jt(e) {
  return e < 0 ? 0 : e > 255 ? 255 : e | 0;
}
function Ce(e) {
  return e < 0 ? 0 : e > 4294967295 ? 4294967295 : e | 0;
}
class Ts {
  constructor(t) {
    this.values = t, this.prefixSum = new Uint32Array(t.length), this.prefixSumValidIndex = new Int32Array(1), this.prefixSumValidIndex[0] = -1;
  }
  insertValues(t, n) {
    t = Ce(t);
    const r = this.values, s = this.prefixSum, i = n.length;
    return i === 0 ? !1 : (this.values = new Uint32Array(r.length + i), this.values.set(r.subarray(0, t), 0), this.values.set(r.subarray(t), t + i), this.values.set(n, t), t - 1 < this.prefixSumValidIndex[0] && (this.prefixSumValidIndex[0] = t - 1), this.prefixSum = new Uint32Array(this.values.length), this.prefixSumValidIndex[0] >= 0 && this.prefixSum.set(s.subarray(0, this.prefixSumValidIndex[0] + 1)), !0);
  }
  setValue(t, n) {
    return t = Ce(t), n = Ce(n), this.values[t] === n ? !1 : (this.values[t] = n, t - 1 < this.prefixSumValidIndex[0] && (this.prefixSumValidIndex[0] = t - 1), !0);
  }
  removeValues(t, n) {
    t = Ce(t), n = Ce(n);
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
    return t < 0 ? 0 : (t = Ce(t), this._getPrefixSum(t));
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
    let n = 0, r = this.values.length - 1, s = 0, i = 0, l = 0;
    for (; n <= r; )
      if (s = n + (r - n) / 2 | 0, i = this.prefixSum[s], l = i - this.values[s], t < l)
        r = s - 1;
      else if (t >= i)
        n = s + 1;
      else
        break;
    return new Bs(s, t - l);
  }
}
class Bs {
  constructor(t, n) {
    this.index = t, this.remainder = n, this._prefixSumIndexOfResultBrand = void 0, this.index = t, this.remainder = n;
  }
}
class Is {
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
      this._acceptDeleteRange(r.range), this._acceptInsertText(new O(r.range.startLineNumber, r.range.startColumn), r.text);
    this._versionId = t.versionId, this._cachedTextValue = null;
  }
  _ensureLineStarts() {
    if (!this._lineStarts) {
      const t = this._eol.length, n = this._lines.length, r = new Uint32Array(n);
      for (let s = 0; s < n; s++)
        r[s] = this._lines[s].length + t;
      this._lineStarts = new Ts(r);
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
    const r = ts(n);
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
const Us = "`~!@#$%^&*()-=+[{]}\\|;:'\",.<>/?";
function qs(e = "") {
  let t = "(-?\\d*\\.\\d\\w*)|([^";
  for (const n of Us)
    e.indexOf(n) >= 0 || (t += "\\" + n);
  return t += "\\s]+)", new RegExp(t, "g");
}
const vr = qs();
function Hs(e) {
  let t = vr;
  if (e && e instanceof RegExp)
    if (e.global)
      t = e;
    else {
      let n = "g";
      e.ignoreCase && (n += "i"), e.multiline && (n += "m"), e.unicode && (n += "u"), t = new RegExp(e.source, n);
    }
  return t.lastIndex = 0, t;
}
const wr = new Tr();
wr.unshift({
  maxLen: 1e3,
  windowSize: 15,
  timeBudget: 150
});
function Ft(e, t, n, r, s) {
  if (s || (s = Ye.first(wr)), n.length > s.maxLen) {
    let u = e - s.maxLen / 2;
    return u < 0 ? u = 0 : r += u, n = n.substring(u, e + s.maxLen / 2), Ft(e, t, n, r, s);
  }
  const i = Date.now(), l = e - 1 - r;
  let o = -1, c = null;
  for (let u = 1; !(Date.now() - i >= s.timeBudget); u++) {
    const h = l - s.windowSize * u;
    t.lastIndex = Math.max(0, h);
    const f = Ws(t, n, l, o);
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
function Ws(e, t, n, r) {
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
class Pt {
  constructor(t) {
    const n = jt(t);
    this._defaultValue = n, this._asciiMap = Pt._createAsciiMap(n), this._map = /* @__PURE__ */ new Map();
  }
  static _createAsciiMap(t) {
    const n = new Uint8Array(256);
    return n.fill(t), n;
  }
  set(t, n) {
    const r = jt(n);
    t >= 0 && t < 256 ? this._asciiMap[t] = r : this._map.set(t, r);
  }
  get(t) {
    return t >= 0 && t < 256 ? this._asciiMap[t] : this._map.get(t) || this._defaultValue;
  }
  clear() {
    this._asciiMap.fill(this._defaultValue), this._map.clear();
  }
}
class $s {
  constructor(t, n, r) {
    const s = new Uint8Array(t * n);
    for (let i = 0, l = t * n; i < l; i++)
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
class zs {
  constructor(t) {
    let n = 0, r = 0;
    for (let i = 0, l = t.length; i < l; i++) {
      const [o, c, u] = t[i];
      c > n && (n = c), o > r && (r = o), u > r && (r = u);
    }
    n++, r++;
    const s = new $s(
      r,
      n,
      0
      /* State.Invalid */
    );
    for (let i = 0, l = t.length; i < l; i++) {
      const [o, c, u] = t[i];
      s.set(o, c, u);
    }
    this._states = s, this._maxCharCode = n;
  }
  nextState(t, n) {
    return n < 0 || n >= this._maxCharCode ? 0 : this._states.get(t, n);
  }
}
let lt = null;
function Gs() {
  return lt === null && (lt = new zs([
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
  ])), lt;
}
let De = null;
function Os() {
  if (De === null) {
    De = new Pt(
      0
      /* CharacterClass.None */
    );
    const e = ` 	<>'"、。｡､，．：；‘〈「『〔（［｛｢｣｝］）〕』」〉’｀～…`;
    for (let n = 0; n < e.length; n++)
      De.set(
        e.charCodeAt(n),
        1
        /* CharacterClass.ForceTermination */
      );
    const t = ".,;:";
    for (let n = 0; n < t.length; n++)
      De.set(
        t.charCodeAt(n),
        2
        /* CharacterClass.CannotEndIn */
      );
  }
  return De;
}
class Ke {
  static _createLink(t, n, r, s, i) {
    let l = i - 1;
    do {
      const o = n.charCodeAt(l);
      if (t.get(o) !== 2)
        break;
      l--;
    } while (l > s);
    if (s > 0) {
      const o = n.charCodeAt(s - 1), c = n.charCodeAt(l);
      (o === 40 && c === 41 || o === 91 && c === 93 || o === 123 && c === 125) && l--;
    }
    return {
      range: {
        startLineNumber: r,
        startColumn: s + 1,
        endLineNumber: r,
        endColumn: l + 2
      },
      url: n.substring(s, l + 1)
    };
  }
  static computeLinks(t, n = Gs()) {
    const r = Os(), s = [];
    for (let i = 1, l = t.getLineCount(); i <= l; i++) {
      const o = t.getLineContent(i), c = o.length;
      let u = 0, h = 0, f = 0, d = 1, m = !1, g = !1, _ = !1, S = !1;
      for (; u < c; ) {
        let b = !1;
        const N = o.charCodeAt(u);
        if (d === 13) {
          let w;
          switch (N) {
            case 40:
              m = !0, w = 0;
              break;
            case 41:
              w = m ? 0 : 1;
              break;
            case 91:
              _ = !0, g = !0, w = 0;
              break;
            case 93:
              _ = !1, w = g ? 0 : 1;
              break;
            case 123:
              S = !0, w = 0;
              break;
            case 125:
              w = S ? 0 : 1;
              break;
            case 39:
            case 34:
            case 96:
              f === N ? w = 1 : f === 39 || f === 34 || f === 96 ? w = 0 : w = 1;
              break;
            case 42:
              w = f === 42 ? 1 : 0;
              break;
            case 124:
              w = f === 124 ? 1 : 0;
              break;
            case 32:
              w = _ ? 0 : 1;
              break;
            default:
              w = r.get(N);
          }
          w === 1 && (s.push(Ke._createLink(r, o, i, h, u)), b = !0);
        } else if (d === 12) {
          let w;
          N === 91 ? (g = !0, w = 0) : w = r.get(N), w === 1 ? b = !0 : d = 13;
        } else
          d = n.nextState(d, N), d === 0 && (b = !0);
        b && (d = 1, m = !1, g = !1, S = !1, h = u + 1, f = N), u++;
      }
      d === 13 && s.push(Ke._createLink(r, o, i, h, c));
    }
    return s;
  }
}
function js(e) {
  return !e || typeof e.getLineCount != "function" || typeof e.getLineContent != "function" ? [] : Ke.computeLinks(e);
}
class vt {
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
      const l = this.doNavigateValueSet(n, i);
      if (l)
        return {
          range: t,
          value: l
        };
    }
    if (r && s) {
      const l = this.doNavigateValueSet(s, i);
      if (l)
        return {
          range: r,
          value: l
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
    for (let i = 0, l = t.length; s === null && i < l; i++)
      s = this.valueSetReplace(t[i], n, r);
    return s;
  }
  valueSetReplace(t, n, r) {
    let s = t.indexOf(n);
    return s >= 0 ? (s += r ? 1 : -1, s < 0 ? s = t.length - 1 : s %= t.length, t[s]) : null;
  }
}
vt.INSTANCE = new vt();
const Lr = Object.freeze(function(e, t) {
  const n = setTimeout(e.bind(t), 0);
  return { dispose() {
    clearTimeout(n);
  } };
});
var et;
(function(e) {
  function t(n) {
    return n === e.None || n === e.Cancelled || n instanceof Qe ? !0 : !n || typeof n != "object" ? !1 : typeof n.isCancellationRequested == "boolean" && typeof n.onCancellationRequested == "function";
  }
  e.isCancellationToken = t, e.None = Object.freeze({
    isCancellationRequested: !1,
    onCancellationRequested: ft.None
  }), e.Cancelled = Object.freeze({
    isCancellationRequested: !0,
    onCancellationRequested: Lr
  });
})(et || (et = {}));
class Qe {
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
    return this._isCancelled ? Lr : (this._emitter || (this._emitter = new se()), this._emitter.event);
  }
  dispose() {
    this._emitter && (this._emitter.dispose(), this._emitter = null);
  }
}
class Qs {
  constructor(t) {
    this._token = void 0, this._parentListener = void 0, this._parentListener = t && t.onCancellationRequested(this.cancel, this);
  }
  get token() {
    return this._token || (this._token = new Qe()), this._token;
  }
  cancel() {
    this._token ? this._token instanceof Qe && this._token.cancel() : this._token = et.Cancelled;
  }
  dispose(t = !1) {
    var n;
    t && this.cancel(), (n = this._parentListener) === null || n === void 0 || n.dispose(), this._token ? this._token instanceof Qe && this._token.dispose() : this._token = et.None;
  }
}
class Dt {
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
const Xe = new Dt(), wt = new Dt(), Lt = new Dt(), Xs = new Array(230), Ys = /* @__PURE__ */ Object.create(null), Js = /* @__PURE__ */ Object.create(null);
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
  for (const s of t) {
    const [i, l, o, c, u, h, f, d, m] = s;
    if (r[l] || (r[l] = !0, Ys[o] = l, Js[o.toLowerCase()] = l), !n[c]) {
      if (n[c] = !0, !u)
        throw new Error(`String representation missing for key code ${c} around scan code ${o}`);
      Xe.define(c, u), wt.define(c, d || u), Lt.define(c, m || d || u);
    }
    h && (Xs[h] = c);
  }
})();
var Qt;
(function(e) {
  function t(o) {
    return Xe.keyCodeToStr(o);
  }
  e.toString = t;
  function n(o) {
    return Xe.strToKeyCode(o);
  }
  e.fromString = n;
  function r(o) {
    return wt.keyCodeToStr(o);
  }
  e.toUserSettingsUS = r;
  function s(o) {
    return Lt.keyCodeToStr(o);
  }
  e.toUserSettingsGeneral = s;
  function i(o) {
    return wt.strToKeyCode(o) || Lt.strToKeyCode(o);
  }
  e.fromUserSettings = i;
  function l(o) {
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
    return Xe.keyCodeToStr(o);
  }
  e.toElectronAccelerator = l;
})(Qt || (Qt = {}));
function Zs(e, t) {
  const n = (t & 65535) << 16 >>> 0;
  return (e | n) >>> 0;
}
class Z extends P {
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
    return Z.selectionsEqual(this, t);
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
    return this.getDirection() === 0 ? new Z(this.startLineNumber, this.startColumn, t, n) : new Z(t, n, this.startLineNumber, this.startColumn);
  }
  /**
   * Get the position at `positionLineNumber` and `positionColumn`.
   */
  getPosition() {
    return new O(this.positionLineNumber, this.positionColumn);
  }
  /**
   * Get the position at the start of the selection.
  */
  getSelectionStart() {
    return new O(this.selectionStartLineNumber, this.selectionStartColumn);
  }
  /**
   * Create a new selection with a different `selectionStartLineNumber` and `selectionStartColumn`.
   */
  setStartPosition(t, n) {
    return this.getDirection() === 0 ? new Z(t, n, this.endLineNumber, this.endColumn) : new Z(this.endLineNumber, this.endColumn, t, n);
  }
  // ----
  /**
   * Create a `Selection` from one or two positions
   */
  static fromPositions(t, n = t) {
    return new Z(t.lineNumber, t.column, n.lineNumber, n.column);
  }
  /**
   * Creates a `Selection` from a range, given a direction.
   */
  static fromRange(t, n) {
    return n === 0 ? new Z(t.startLineNumber, t.startColumn, t.endLineNumber, t.endColumn) : new Z(t.endLineNumber, t.endColumn, t.startLineNumber, t.startColumn);
  }
  /**
   * Create a `Selection` from an `ISelection`.
   */
  static liftSelection(t) {
    return new Z(t.selectionStartLineNumber, t.selectionStartColumn, t.positionLineNumber, t.positionColumn);
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
    return i === 0 ? new Z(t, n, r, s) : new Z(r, s, t, n);
  }
}
const Xt = /* @__PURE__ */ Object.create(null);
function a(e, t) {
  if (Wr(t)) {
    const n = Xt[t];
    if (n === void 0)
      throw new Error(`${e} references an unknown codicon: ${t}`);
    t = n;
  }
  return Xt[e] = t, { id: e };
}
const y = {
  // built-in icons, with image name
  add: a("add", 6e4),
  plus: a("plus", 6e4),
  gistNew: a("gist-new", 6e4),
  repoCreate: a("repo-create", 6e4),
  lightbulb: a("lightbulb", 60001),
  lightBulb: a("light-bulb", 60001),
  repo: a("repo", 60002),
  repoDelete: a("repo-delete", 60002),
  gistFork: a("gist-fork", 60003),
  repoForked: a("repo-forked", 60003),
  gitPullRequest: a("git-pull-request", 60004),
  gitPullRequestAbandoned: a("git-pull-request-abandoned", 60004),
  recordKeys: a("record-keys", 60005),
  keyboard: a("keyboard", 60005),
  tag: a("tag", 60006),
  tagAdd: a("tag-add", 60006),
  tagRemove: a("tag-remove", 60006),
  person: a("person", 60007),
  personFollow: a("person-follow", 60007),
  personOutline: a("person-outline", 60007),
  personFilled: a("person-filled", 60007),
  gitBranch: a("git-branch", 60008),
  gitBranchCreate: a("git-branch-create", 60008),
  gitBranchDelete: a("git-branch-delete", 60008),
  sourceControl: a("source-control", 60008),
  mirror: a("mirror", 60009),
  mirrorPublic: a("mirror-public", 60009),
  star: a("star", 60010),
  starAdd: a("star-add", 60010),
  starDelete: a("star-delete", 60010),
  starEmpty: a("star-empty", 60010),
  comment: a("comment", 60011),
  commentAdd: a("comment-add", 60011),
  alert: a("alert", 60012),
  warning: a("warning", 60012),
  search: a("search", 60013),
  searchSave: a("search-save", 60013),
  logOut: a("log-out", 60014),
  signOut: a("sign-out", 60014),
  logIn: a("log-in", 60015),
  signIn: a("sign-in", 60015),
  eye: a("eye", 60016),
  eyeUnwatch: a("eye-unwatch", 60016),
  eyeWatch: a("eye-watch", 60016),
  circleFilled: a("circle-filled", 60017),
  primitiveDot: a("primitive-dot", 60017),
  closeDirty: a("close-dirty", 60017),
  debugBreakpoint: a("debug-breakpoint", 60017),
  debugBreakpointDisabled: a("debug-breakpoint-disabled", 60017),
  debugHint: a("debug-hint", 60017),
  primitiveSquare: a("primitive-square", 60018),
  edit: a("edit", 60019),
  pencil: a("pencil", 60019),
  info: a("info", 60020),
  issueOpened: a("issue-opened", 60020),
  gistPrivate: a("gist-private", 60021),
  gitForkPrivate: a("git-fork-private", 60021),
  lock: a("lock", 60021),
  mirrorPrivate: a("mirror-private", 60021),
  close: a("close", 60022),
  removeClose: a("remove-close", 60022),
  x: a("x", 60022),
  repoSync: a("repo-sync", 60023),
  sync: a("sync", 60023),
  clone: a("clone", 60024),
  desktopDownload: a("desktop-download", 60024),
  beaker: a("beaker", 60025),
  microscope: a("microscope", 60025),
  vm: a("vm", 60026),
  deviceDesktop: a("device-desktop", 60026),
  file: a("file", 60027),
  fileText: a("file-text", 60027),
  more: a("more", 60028),
  ellipsis: a("ellipsis", 60028),
  kebabHorizontal: a("kebab-horizontal", 60028),
  mailReply: a("mail-reply", 60029),
  reply: a("reply", 60029),
  organization: a("organization", 60030),
  organizationFilled: a("organization-filled", 60030),
  organizationOutline: a("organization-outline", 60030),
  newFile: a("new-file", 60031),
  fileAdd: a("file-add", 60031),
  newFolder: a("new-folder", 60032),
  fileDirectoryCreate: a("file-directory-create", 60032),
  trash: a("trash", 60033),
  trashcan: a("trashcan", 60033),
  history: a("history", 60034),
  clock: a("clock", 60034),
  folder: a("folder", 60035),
  fileDirectory: a("file-directory", 60035),
  symbolFolder: a("symbol-folder", 60035),
  logoGithub: a("logo-github", 60036),
  markGithub: a("mark-github", 60036),
  github: a("github", 60036),
  terminal: a("terminal", 60037),
  console: a("console", 60037),
  repl: a("repl", 60037),
  zap: a("zap", 60038),
  symbolEvent: a("symbol-event", 60038),
  error: a("error", 60039),
  stop: a("stop", 60039),
  variable: a("variable", 60040),
  symbolVariable: a("symbol-variable", 60040),
  array: a("array", 60042),
  symbolArray: a("symbol-array", 60042),
  symbolModule: a("symbol-module", 60043),
  symbolPackage: a("symbol-package", 60043),
  symbolNamespace: a("symbol-namespace", 60043),
  symbolObject: a("symbol-object", 60043),
  symbolMethod: a("symbol-method", 60044),
  symbolFunction: a("symbol-function", 60044),
  symbolConstructor: a("symbol-constructor", 60044),
  symbolBoolean: a("symbol-boolean", 60047),
  symbolNull: a("symbol-null", 60047),
  symbolNumeric: a("symbol-numeric", 60048),
  symbolNumber: a("symbol-number", 60048),
  symbolStructure: a("symbol-structure", 60049),
  symbolStruct: a("symbol-struct", 60049),
  symbolParameter: a("symbol-parameter", 60050),
  symbolTypeParameter: a("symbol-type-parameter", 60050),
  symbolKey: a("symbol-key", 60051),
  symbolText: a("symbol-text", 60051),
  symbolReference: a("symbol-reference", 60052),
  goToFile: a("go-to-file", 60052),
  symbolEnum: a("symbol-enum", 60053),
  symbolValue: a("symbol-value", 60053),
  symbolRuler: a("symbol-ruler", 60054),
  symbolUnit: a("symbol-unit", 60054),
  activateBreakpoints: a("activate-breakpoints", 60055),
  archive: a("archive", 60056),
  arrowBoth: a("arrow-both", 60057),
  arrowDown: a("arrow-down", 60058),
  arrowLeft: a("arrow-left", 60059),
  arrowRight: a("arrow-right", 60060),
  arrowSmallDown: a("arrow-small-down", 60061),
  arrowSmallLeft: a("arrow-small-left", 60062),
  arrowSmallRight: a("arrow-small-right", 60063),
  arrowSmallUp: a("arrow-small-up", 60064),
  arrowUp: a("arrow-up", 60065),
  bell: a("bell", 60066),
  bold: a("bold", 60067),
  book: a("book", 60068),
  bookmark: a("bookmark", 60069),
  debugBreakpointConditionalUnverified: a("debug-breakpoint-conditional-unverified", 60070),
  debugBreakpointConditional: a("debug-breakpoint-conditional", 60071),
  debugBreakpointConditionalDisabled: a("debug-breakpoint-conditional-disabled", 60071),
  debugBreakpointDataUnverified: a("debug-breakpoint-data-unverified", 60072),
  debugBreakpointData: a("debug-breakpoint-data", 60073),
  debugBreakpointDataDisabled: a("debug-breakpoint-data-disabled", 60073),
  debugBreakpointLogUnverified: a("debug-breakpoint-log-unverified", 60074),
  debugBreakpointLog: a("debug-breakpoint-log", 60075),
  debugBreakpointLogDisabled: a("debug-breakpoint-log-disabled", 60075),
  briefcase: a("briefcase", 60076),
  broadcast: a("broadcast", 60077),
  browser: a("browser", 60078),
  bug: a("bug", 60079),
  calendar: a("calendar", 60080),
  caseSensitive: a("case-sensitive", 60081),
  check: a("check", 60082),
  checklist: a("checklist", 60083),
  chevronDown: a("chevron-down", 60084),
  dropDownButton: a("drop-down-button", 60084),
  chevronLeft: a("chevron-left", 60085),
  chevronRight: a("chevron-right", 60086),
  chevronUp: a("chevron-up", 60087),
  chromeClose: a("chrome-close", 60088),
  chromeMaximize: a("chrome-maximize", 60089),
  chromeMinimize: a("chrome-minimize", 60090),
  chromeRestore: a("chrome-restore", 60091),
  circle: a("circle", 60092),
  circleOutline: a("circle-outline", 60092),
  debugBreakpointUnverified: a("debug-breakpoint-unverified", 60092),
  circleSlash: a("circle-slash", 60093),
  circuitBoard: a("circuit-board", 60094),
  clearAll: a("clear-all", 60095),
  clippy: a("clippy", 60096),
  closeAll: a("close-all", 60097),
  cloudDownload: a("cloud-download", 60098),
  cloudUpload: a("cloud-upload", 60099),
  code: a("code", 60100),
  collapseAll: a("collapse-all", 60101),
  colorMode: a("color-mode", 60102),
  commentDiscussion: a("comment-discussion", 60103),
  compareChanges: a("compare-changes", 60157),
  creditCard: a("credit-card", 60105),
  dash: a("dash", 60108),
  dashboard: a("dashboard", 60109),
  database: a("database", 60110),
  debugContinue: a("debug-continue", 60111),
  debugDisconnect: a("debug-disconnect", 60112),
  debugPause: a("debug-pause", 60113),
  debugRestart: a("debug-restart", 60114),
  debugStart: a("debug-start", 60115),
  debugStepInto: a("debug-step-into", 60116),
  debugStepOut: a("debug-step-out", 60117),
  debugStepOver: a("debug-step-over", 60118),
  debugStop: a("debug-stop", 60119),
  debug: a("debug", 60120),
  deviceCameraVideo: a("device-camera-video", 60121),
  deviceCamera: a("device-camera", 60122),
  deviceMobile: a("device-mobile", 60123),
  diffAdded: a("diff-added", 60124),
  diffIgnored: a("diff-ignored", 60125),
  diffModified: a("diff-modified", 60126),
  diffRemoved: a("diff-removed", 60127),
  diffRenamed: a("diff-renamed", 60128),
  diff: a("diff", 60129),
  discard: a("discard", 60130),
  editorLayout: a("editor-layout", 60131),
  emptyWindow: a("empty-window", 60132),
  exclude: a("exclude", 60133),
  extensions: a("extensions", 60134),
  eyeClosed: a("eye-closed", 60135),
  fileBinary: a("file-binary", 60136),
  fileCode: a("file-code", 60137),
  fileMedia: a("file-media", 60138),
  filePdf: a("file-pdf", 60139),
  fileSubmodule: a("file-submodule", 60140),
  fileSymlinkDirectory: a("file-symlink-directory", 60141),
  fileSymlinkFile: a("file-symlink-file", 60142),
  fileZip: a("file-zip", 60143),
  files: a("files", 60144),
  filter: a("filter", 60145),
  flame: a("flame", 60146),
  foldDown: a("fold-down", 60147),
  foldUp: a("fold-up", 60148),
  fold: a("fold", 60149),
  folderActive: a("folder-active", 60150),
  folderOpened: a("folder-opened", 60151),
  gear: a("gear", 60152),
  gift: a("gift", 60153),
  gistSecret: a("gist-secret", 60154),
  gist: a("gist", 60155),
  gitCommit: a("git-commit", 60156),
  gitCompare: a("git-compare", 60157),
  gitMerge: a("git-merge", 60158),
  githubAction: a("github-action", 60159),
  githubAlt: a("github-alt", 60160),
  globe: a("globe", 60161),
  grabber: a("grabber", 60162),
  graph: a("graph", 60163),
  gripper: a("gripper", 60164),
  heart: a("heart", 60165),
  home: a("home", 60166),
  horizontalRule: a("horizontal-rule", 60167),
  hubot: a("hubot", 60168),
  inbox: a("inbox", 60169),
  issueClosed: a("issue-closed", 60324),
  issueReopened: a("issue-reopened", 60171),
  issues: a("issues", 60172),
  italic: a("italic", 60173),
  jersey: a("jersey", 60174),
  json: a("json", 60175),
  bracket: a("bracket", 60175),
  kebabVertical: a("kebab-vertical", 60176),
  key: a("key", 60177),
  law: a("law", 60178),
  lightbulbAutofix: a("lightbulb-autofix", 60179),
  linkExternal: a("link-external", 60180),
  link: a("link", 60181),
  listOrdered: a("list-ordered", 60182),
  listUnordered: a("list-unordered", 60183),
  liveShare: a("live-share", 60184),
  loading: a("loading", 60185),
  location: a("location", 60186),
  mailRead: a("mail-read", 60187),
  mail: a("mail", 60188),
  markdown: a("markdown", 60189),
  megaphone: a("megaphone", 60190),
  mention: a("mention", 60191),
  milestone: a("milestone", 60192),
  mortarBoard: a("mortar-board", 60193),
  move: a("move", 60194),
  multipleWindows: a("multiple-windows", 60195),
  mute: a("mute", 60196),
  noNewline: a("no-newline", 60197),
  note: a("note", 60198),
  octoface: a("octoface", 60199),
  openPreview: a("open-preview", 60200),
  package_: a("package", 60201),
  paintcan: a("paintcan", 60202),
  pin: a("pin", 60203),
  play: a("play", 60204),
  run: a("run", 60204),
  plug: a("plug", 60205),
  preserveCase: a("preserve-case", 60206),
  preview: a("preview", 60207),
  project: a("project", 60208),
  pulse: a("pulse", 60209),
  question: a("question", 60210),
  quote: a("quote", 60211),
  radioTower: a("radio-tower", 60212),
  reactions: a("reactions", 60213),
  references: a("references", 60214),
  refresh: a("refresh", 60215),
  regex: a("regex", 60216),
  remoteExplorer: a("remote-explorer", 60217),
  remote: a("remote", 60218),
  remove: a("remove", 60219),
  replaceAll: a("replace-all", 60220),
  replace: a("replace", 60221),
  repoClone: a("repo-clone", 60222),
  repoForcePush: a("repo-force-push", 60223),
  repoPull: a("repo-pull", 60224),
  repoPush: a("repo-push", 60225),
  report: a("report", 60226),
  requestChanges: a("request-changes", 60227),
  rocket: a("rocket", 60228),
  rootFolderOpened: a("root-folder-opened", 60229),
  rootFolder: a("root-folder", 60230),
  rss: a("rss", 60231),
  ruby: a("ruby", 60232),
  saveAll: a("save-all", 60233),
  saveAs: a("save-as", 60234),
  save: a("save", 60235),
  screenFull: a("screen-full", 60236),
  screenNormal: a("screen-normal", 60237),
  searchStop: a("search-stop", 60238),
  server: a("server", 60240),
  settingsGear: a("settings-gear", 60241),
  settings: a("settings", 60242),
  shield: a("shield", 60243),
  smiley: a("smiley", 60244),
  sortPrecedence: a("sort-precedence", 60245),
  splitHorizontal: a("split-horizontal", 60246),
  splitVertical: a("split-vertical", 60247),
  squirrel: a("squirrel", 60248),
  starFull: a("star-full", 60249),
  starHalf: a("star-half", 60250),
  symbolClass: a("symbol-class", 60251),
  symbolColor: a("symbol-color", 60252),
  symbolCustomColor: a("symbol-customcolor", 60252),
  symbolConstant: a("symbol-constant", 60253),
  symbolEnumMember: a("symbol-enum-member", 60254),
  symbolField: a("symbol-field", 60255),
  symbolFile: a("symbol-file", 60256),
  symbolInterface: a("symbol-interface", 60257),
  symbolKeyword: a("symbol-keyword", 60258),
  symbolMisc: a("symbol-misc", 60259),
  symbolOperator: a("symbol-operator", 60260),
  symbolProperty: a("symbol-property", 60261),
  wrench: a("wrench", 60261),
  wrenchSubaction: a("wrench-subaction", 60261),
  symbolSnippet: a("symbol-snippet", 60262),
  tasklist: a("tasklist", 60263),
  telescope: a("telescope", 60264),
  textSize: a("text-size", 60265),
  threeBars: a("three-bars", 60266),
  thumbsdown: a("thumbsdown", 60267),
  thumbsup: a("thumbsup", 60268),
  tools: a("tools", 60269),
  triangleDown: a("triangle-down", 60270),
  triangleLeft: a("triangle-left", 60271),
  triangleRight: a("triangle-right", 60272),
  triangleUp: a("triangle-up", 60273),
  twitter: a("twitter", 60274),
  unfold: a("unfold", 60275),
  unlock: a("unlock", 60276),
  unmute: a("unmute", 60277),
  unverified: a("unverified", 60278),
  verified: a("verified", 60279),
  versions: a("versions", 60280),
  vmActive: a("vm-active", 60281),
  vmOutline: a("vm-outline", 60282),
  vmRunning: a("vm-running", 60283),
  watch: a("watch", 60284),
  whitespace: a("whitespace", 60285),
  wholeWord: a("whole-word", 60286),
  window: a("window", 60287),
  wordWrap: a("word-wrap", 60288),
  zoomIn: a("zoom-in", 60289),
  zoomOut: a("zoom-out", 60290),
  listFilter: a("list-filter", 60291),
  listFlat: a("list-flat", 60292),
  listSelection: a("list-selection", 60293),
  selection: a("selection", 60293),
  listTree: a("list-tree", 60294),
  debugBreakpointFunctionUnverified: a("debug-breakpoint-function-unverified", 60295),
  debugBreakpointFunction: a("debug-breakpoint-function", 60296),
  debugBreakpointFunctionDisabled: a("debug-breakpoint-function-disabled", 60296),
  debugStackframeActive: a("debug-stackframe-active", 60297),
  circleSmallFilled: a("circle-small-filled", 60298),
  debugStackframeDot: a("debug-stackframe-dot", 60298),
  debugStackframe: a("debug-stackframe", 60299),
  debugStackframeFocused: a("debug-stackframe-focused", 60299),
  debugBreakpointUnsupported: a("debug-breakpoint-unsupported", 60300),
  symbolString: a("symbol-string", 60301),
  debugReverseContinue: a("debug-reverse-continue", 60302),
  debugStepBack: a("debug-step-back", 60303),
  debugRestartFrame: a("debug-restart-frame", 60304),
  callIncoming: a("call-incoming", 60306),
  callOutgoing: a("call-outgoing", 60307),
  menu: a("menu", 60308),
  expandAll: a("expand-all", 60309),
  feedback: a("feedback", 60310),
  groupByRefType: a("group-by-ref-type", 60311),
  ungroupByRefType: a("ungroup-by-ref-type", 60312),
  account: a("account", 60313),
  bellDot: a("bell-dot", 60314),
  debugConsole: a("debug-console", 60315),
  library: a("library", 60316),
  output: a("output", 60317),
  runAll: a("run-all", 60318),
  syncIgnored: a("sync-ignored", 60319),
  pinned: a("pinned", 60320),
  githubInverted: a("github-inverted", 60321),
  debugAlt: a("debug-alt", 60305),
  serverProcess: a("server-process", 60322),
  serverEnvironment: a("server-environment", 60323),
  pass: a("pass", 60324),
  stopCircle: a("stop-circle", 60325),
  playCircle: a("play-circle", 60326),
  record: a("record", 60327),
  debugAltSmall: a("debug-alt-small", 60328),
  vmConnect: a("vm-connect", 60329),
  cloud: a("cloud", 60330),
  merge: a("merge", 60331),
  exportIcon: a("export", 60332),
  graphLeft: a("graph-left", 60333),
  magnet: a("magnet", 60334),
  notebook: a("notebook", 60335),
  redo: a("redo", 60336),
  checkAll: a("check-all", 60337),
  pinnedDirty: a("pinned-dirty", 60338),
  passFilled: a("pass-filled", 60339),
  circleLargeFilled: a("circle-large-filled", 60340),
  circleLarge: a("circle-large", 60341),
  circleLargeOutline: a("circle-large-outline", 60341),
  combine: a("combine", 60342),
  gather: a("gather", 60342),
  table: a("table", 60343),
  variableGroup: a("variable-group", 60344),
  typeHierarchy: a("type-hierarchy", 60345),
  typeHierarchySub: a("type-hierarchy-sub", 60346),
  typeHierarchySuper: a("type-hierarchy-super", 60347),
  gitPullRequestCreate: a("git-pull-request-create", 60348),
  runAbove: a("run-above", 60349),
  runBelow: a("run-below", 60350),
  notebookTemplate: a("notebook-template", 60351),
  debugRerun: a("debug-rerun", 60352),
  workspaceTrusted: a("workspace-trusted", 60353),
  workspaceUntrusted: a("workspace-untrusted", 60354),
  workspaceUnspecified: a("workspace-unspecified", 60355),
  terminalCmd: a("terminal-cmd", 60356),
  terminalDebian: a("terminal-debian", 60357),
  terminalLinux: a("terminal-linux", 60358),
  terminalPowershell: a("terminal-powershell", 60359),
  terminalTmux: a("terminal-tmux", 60360),
  terminalUbuntu: a("terminal-ubuntu", 60361),
  terminalBash: a("terminal-bash", 60362),
  arrowSwap: a("arrow-swap", 60363),
  copy: a("copy", 60364),
  personAdd: a("person-add", 60365),
  filterFilled: a("filter-filled", 60366),
  wand: a("wand", 60367),
  debugLineByLine: a("debug-line-by-line", 60368),
  inspect: a("inspect", 60369),
  layers: a("layers", 60370),
  layersDot: a("layers-dot", 60371),
  layersActive: a("layers-active", 60372),
  compass: a("compass", 60373),
  compassDot: a("compass-dot", 60374),
  compassActive: a("compass-active", 60375),
  azure: a("azure", 60376),
  issueDraft: a("issue-draft", 60377),
  gitPullRequestClosed: a("git-pull-request-closed", 60378),
  gitPullRequestDraft: a("git-pull-request-draft", 60379),
  debugAll: a("debug-all", 60380),
  debugCoverage: a("debug-coverage", 60381),
  runErrors: a("run-errors", 60382),
  folderLibrary: a("folder-library", 60383),
  debugContinueSmall: a("debug-continue-small", 60384),
  beakerStop: a("beaker-stop", 60385),
  graphLine: a("graph-line", 60386),
  graphScatter: a("graph-scatter", 60387),
  pieChart: a("pie-chart", 60388),
  bracketDot: a("bracket-dot", 60389),
  bracketError: a("bracket-error", 60390),
  lockSmall: a("lock-small", 60391),
  azureDevops: a("azure-devops", 60392),
  verifiedFilled: a("verified-filled", 60393),
  newLine: a("newline", 60394),
  layout: a("layout", 60395),
  layoutActivitybarLeft: a("layout-activitybar-left", 60396),
  layoutActivitybarRight: a("layout-activitybar-right", 60397),
  layoutPanelLeft: a("layout-panel-left", 60398),
  layoutPanelCenter: a("layout-panel-center", 60399),
  layoutPanelJustify: a("layout-panel-justify", 60400),
  layoutPanelRight: a("layout-panel-right", 60401),
  layoutPanel: a("layout-panel", 60402),
  layoutSidebarLeft: a("layout-sidebar-left", 60403),
  layoutSidebarRight: a("layout-sidebar-right", 60404),
  layoutStatusbar: a("layout-statusbar", 60405),
  layoutMenubar: a("layout-menubar", 60406),
  layoutCentered: a("layout-centered", 60407),
  layoutSidebarRightOff: a("layout-sidebar-right-off", 60416),
  layoutPanelOff: a("layout-panel-off", 60417),
  layoutSidebarLeftOff: a("layout-sidebar-left-off", 60418),
  target: a("target", 60408),
  indent: a("indent", 60409),
  recordSmall: a("record-small", 60410),
  errorSmall: a("error-small", 60411),
  arrowCircleDown: a("arrow-circle-down", 60412),
  arrowCircleLeft: a("arrow-circle-left", 60413),
  arrowCircleRight: a("arrow-circle-right", 60414),
  arrowCircleUp: a("arrow-circle-up", 60415),
  heartFilled: a("heart-filled", 60420),
  map: a("map", 60421),
  mapFilled: a("map-filled", 60422),
  circleSmall: a("circle-small", 60423),
  bellSlash: a("bell-slash", 60424),
  bellSlashDot: a("bell-slash-dot", 60425),
  commentUnresolved: a("comment-unresolved", 60426),
  gitPullRequestGoToChanges: a("git-pull-request-go-to-changes", 60427),
  gitPullRequestNewChanges: a("git-pull-request-new-changes", 60428),
  searchFuzzy: a("search-fuzzy", 60429),
  commentDraft: a("comment-draft", 60430),
  send: a("send", 60431),
  sparkle: a("sparkle", 60432),
  insert: a("insert", 60433),
  // derived icons, that could become separate icons
  dialogError: a("dialog-error", "error"),
  dialogWarning: a("dialog-warning", "warning"),
  dialogInfo: a("dialog-info", "info"),
  dialogClose: a("dialog-close", "close"),
  treeItemExpanded: a("tree-item-expanded", "chevron-down"),
  treeFilterOnTypeOn: a("tree-filter-on-type-on", "list-filter"),
  treeFilterOnTypeOff: a("tree-filter-on-type-off", "list-selection"),
  treeFilterClear: a("tree-filter-clear", "close"),
  treeItemLoading: a("tree-item-loading", "loading"),
  menuSelection: a("menu-selection", "check"),
  menuSubmenu: a("menu-submenu", "chevron-right"),
  menuBarMore: a("menubar-more", "more"),
  scrollbarButtonLeft: a("scrollbar-button-left", "triangle-left"),
  scrollbarButtonRight: a("scrollbar-button-right", "triangle-right"),
  scrollbarButtonUp: a("scrollbar-button-up", "triangle-up"),
  scrollbarButtonDown: a("scrollbar-button-down", "triangle-down"),
  toolBarMore: a("toolbar-more", "more"),
  quickInputBack: a("quick-input-back", "arrow-left")
};
var Nt = globalThis && globalThis.__awaiter || function(e, t, n, r) {
  function s(i) {
    return i instanceof n ? i : new n(function(l) {
      l(i);
    });
  }
  return new (n || (n = Promise))(function(i, l) {
    function o(h) {
      try {
        u(r.next(h));
      } catch (f) {
        l(f);
      }
    }
    function c(h) {
      try {
        u(r.throw(h));
      } catch (f) {
        l(f);
      }
    }
    function u(h) {
      h.done ? i(h.value) : s(h.value).then(o, c);
    }
    u((r = r.apply(e, t || [])).next());
  });
};
class Ks {
  constructor() {
    this._tokenizationSupports = /* @__PURE__ */ new Map(), this._factories = /* @__PURE__ */ new Map(), this._onDidChange = new se(), this.onDidChange = this._onDidChange.event, this._colorMap = null;
  }
  handleChange(t) {
    this._onDidChange.fire({
      changedLanguages: t,
      changedColorMap: !1
    });
  }
  register(t, n) {
    return this._tokenizationSupports.set(t, n), this.handleChange([t]), Ie(() => {
      this._tokenizationSupports.get(t) === n && (this._tokenizationSupports.delete(t), this.handleChange([t]));
    });
  }
  get(t) {
    return this._tokenizationSupports.get(t) || null;
  }
  registerFactory(t, n) {
    var r;
    (r = this._factories.get(t)) === null || r === void 0 || r.dispose();
    const s = new ei(this, t, n);
    return this._factories.set(t, s), Ie(() => {
      const i = this._factories.get(t);
      !i || i !== s || (this._factories.delete(t), i.dispose());
    });
  }
  getOrCreate(t) {
    return Nt(this, void 0, void 0, function* () {
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
class ei extends Ue {
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
    return Nt(this, void 0, void 0, function* () {
      return this._resolvePromise || (this._resolvePromise = this._create()), this._resolvePromise;
    });
  }
  _create() {
    return Nt(this, void 0, void 0, function* () {
      const t = yield this._factory.tokenizationSupport;
      this._isResolved = !0, t && !this._isDisposed && this._register(this._registry.register(this._languageId, t));
    });
  }
}
class ti {
  constructor(t, n, r) {
    this.offset = t, this.type = n, this.language = r, this._tokenBrand = void 0;
  }
  toString() {
    return "(" + this.offset + ", " + this.type + ")";
  }
}
var Yt;
(function(e) {
  const t = /* @__PURE__ */ new Map();
  t.set(0, y.symbolMethod), t.set(1, y.symbolFunction), t.set(2, y.symbolConstructor), t.set(3, y.symbolField), t.set(4, y.symbolVariable), t.set(5, y.symbolClass), t.set(6, y.symbolStruct), t.set(7, y.symbolInterface), t.set(8, y.symbolModule), t.set(9, y.symbolProperty), t.set(10, y.symbolEvent), t.set(11, y.symbolOperator), t.set(12, y.symbolUnit), t.set(13, y.symbolValue), t.set(15, y.symbolEnum), t.set(14, y.symbolConstant), t.set(15, y.symbolEnum), t.set(16, y.symbolEnumMember), t.set(17, y.symbolKeyword), t.set(27, y.symbolSnippet), t.set(18, y.symbolText), t.set(19, y.symbolColor), t.set(20, y.symbolFile), t.set(21, y.symbolReference), t.set(22, y.symbolCustomColor), t.set(23, y.symbolFolder), t.set(24, y.symbolTypeParameter), t.set(25, y.account), t.set(26, y.issues);
  function n(i) {
    let l = t.get(i);
    return l || (console.info("No codicon found for CompletionItemKind " + i), l = y.symbolProperty), l;
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
  function s(i, l) {
    let o = r.get(i);
    return typeof o > "u" && !l && (o = 9), o;
  }
  e.fromString = s;
})(Yt || (Yt = {}));
var Jt;
(function(e) {
  e[e.Automatic = 0] = "Automatic", e[e.Explicit = 1] = "Explicit";
})(Jt || (Jt = {}));
var Zt;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.TriggerCharacter = 2] = "TriggerCharacter", e[e.ContentChange = 3] = "ContentChange";
})(Zt || (Zt = {}));
var Kt;
(function(e) {
  e[e.Text = 0] = "Text", e[e.Read = 1] = "Read", e[e.Write = 2] = "Write";
})(Kt || (Kt = {}));
U("Array", "array"), U("Boolean", "boolean"), U("Class", "class"), U("Constant", "constant"), U("Constructor", "constructor"), U("Enum", "enumeration"), U("EnumMember", "enumeration member"), U("Event", "event"), U("Field", "field"), U("File", "file"), U("Function", "function"), U("Interface", "interface"), U("Key", "key"), U("Method", "method"), U("Module", "module"), U("Namespace", "namespace"), U("Null", "null"), U("Number", "number"), U("Object", "object"), U("Operator", "operator"), U("Package", "package"), U("Property", "property"), U("String", "string"), U("Struct", "struct"), U("TypeParameter", "type parameter"), U("Variable", "variable");
var en;
(function(e) {
  const t = /* @__PURE__ */ new Map();
  t.set(0, y.symbolFile), t.set(1, y.symbolModule), t.set(2, y.symbolNamespace), t.set(3, y.symbolPackage), t.set(4, y.symbolClass), t.set(5, y.symbolMethod), t.set(6, y.symbolProperty), t.set(7, y.symbolField), t.set(8, y.symbolConstructor), t.set(9, y.symbolEnum), t.set(10, y.symbolInterface), t.set(11, y.symbolFunction), t.set(12, y.symbolVariable), t.set(13, y.symbolConstant), t.set(14, y.symbolString), t.set(15, y.symbolNumber), t.set(16, y.symbolBoolean), t.set(17, y.symbolArray), t.set(18, y.symbolObject), t.set(19, y.symbolKey), t.set(20, y.symbolNull), t.set(21, y.symbolEnumMember), t.set(22, y.symbolStruct), t.set(23, y.symbolEvent), t.set(24, y.symbolOperator), t.set(25, y.symbolTypeParameter);
  function n(r) {
    let s = t.get(r);
    return s || (console.info("No codicon found for SymbolKind " + r), s = y.symbolProperty), s;
  }
  e.toIcon = n;
})(en || (en = {}));
var tn;
(function(e) {
  function t(n) {
    return !n || typeof n != "object" ? !1 : typeof n.id == "string" && typeof n.title == "string";
  }
  e.is = t;
})(tn || (tn = {}));
var nn;
(function(e) {
  e[e.Type = 1] = "Type", e[e.Parameter = 2] = "Parameter";
})(nn || (nn = {}));
new Ks();
var rn;
(function(e) {
  e[e.Unknown = 0] = "Unknown", e[e.Disabled = 1] = "Disabled", e[e.Enabled = 2] = "Enabled";
})(rn || (rn = {}));
var sn;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.Auto = 2] = "Auto";
})(sn || (sn = {}));
var an;
(function(e) {
  e[e.None = 0] = "None", e[e.KeepWhitespace = 1] = "KeepWhitespace", e[e.InsertAsSnippet = 4] = "InsertAsSnippet";
})(an || (an = {}));
var ln;
(function(e) {
  e[e.Method = 0] = "Method", e[e.Function = 1] = "Function", e[e.Constructor = 2] = "Constructor", e[e.Field = 3] = "Field", e[e.Variable = 4] = "Variable", e[e.Class = 5] = "Class", e[e.Struct = 6] = "Struct", e[e.Interface = 7] = "Interface", e[e.Module = 8] = "Module", e[e.Property = 9] = "Property", e[e.Event = 10] = "Event", e[e.Operator = 11] = "Operator", e[e.Unit = 12] = "Unit", e[e.Value = 13] = "Value", e[e.Constant = 14] = "Constant", e[e.Enum = 15] = "Enum", e[e.EnumMember = 16] = "EnumMember", e[e.Keyword = 17] = "Keyword", e[e.Text = 18] = "Text", e[e.Color = 19] = "Color", e[e.File = 20] = "File", e[e.Reference = 21] = "Reference", e[e.Customcolor = 22] = "Customcolor", e[e.Folder = 23] = "Folder", e[e.TypeParameter = 24] = "TypeParameter", e[e.User = 25] = "User", e[e.Issue = 26] = "Issue", e[e.Snippet = 27] = "Snippet";
})(ln || (ln = {}));
var on;
(function(e) {
  e[e.Deprecated = 1] = "Deprecated";
})(on || (on = {}));
var un;
(function(e) {
  e[e.Invoke = 0] = "Invoke", e[e.TriggerCharacter = 1] = "TriggerCharacter", e[e.TriggerForIncompleteCompletions = 2] = "TriggerForIncompleteCompletions";
})(un || (un = {}));
var cn;
(function(e) {
  e[e.EXACT = 0] = "EXACT", e[e.ABOVE = 1] = "ABOVE", e[e.BELOW = 2] = "BELOW";
})(cn || (cn = {}));
var hn;
(function(e) {
  e[e.NotSet = 0] = "NotSet", e[e.ContentFlush = 1] = "ContentFlush", e[e.RecoverFromMarkers = 2] = "RecoverFromMarkers", e[e.Explicit = 3] = "Explicit", e[e.Paste = 4] = "Paste", e[e.Undo = 5] = "Undo", e[e.Redo = 6] = "Redo";
})(hn || (hn = {}));
var fn;
(function(e) {
  e[e.LF = 1] = "LF", e[e.CRLF = 2] = "CRLF";
})(fn || (fn = {}));
var dn;
(function(e) {
  e[e.Text = 0] = "Text", e[e.Read = 1] = "Read", e[e.Write = 2] = "Write";
})(dn || (dn = {}));
var mn;
(function(e) {
  e[e.None = 0] = "None", e[e.Keep = 1] = "Keep", e[e.Brackets = 2] = "Brackets", e[e.Advanced = 3] = "Advanced", e[e.Full = 4] = "Full";
})(mn || (mn = {}));
var gn;
(function(e) {
  e[e.acceptSuggestionOnCommitCharacter = 0] = "acceptSuggestionOnCommitCharacter", e[e.acceptSuggestionOnEnter = 1] = "acceptSuggestionOnEnter", e[e.accessibilitySupport = 2] = "accessibilitySupport", e[e.accessibilityPageSize = 3] = "accessibilityPageSize", e[e.ariaLabel = 4] = "ariaLabel", e[e.ariaRequired = 5] = "ariaRequired", e[e.autoClosingBrackets = 6] = "autoClosingBrackets", e[e.screenReaderAnnounceInlineSuggestion = 7] = "screenReaderAnnounceInlineSuggestion", e[e.autoClosingDelete = 8] = "autoClosingDelete", e[e.autoClosingOvertype = 9] = "autoClosingOvertype", e[e.autoClosingQuotes = 10] = "autoClosingQuotes", e[e.autoIndent = 11] = "autoIndent", e[e.automaticLayout = 12] = "automaticLayout", e[e.autoSurround = 13] = "autoSurround", e[e.bracketPairColorization = 14] = "bracketPairColorization", e[e.guides = 15] = "guides", e[e.codeLens = 16] = "codeLens", e[e.codeLensFontFamily = 17] = "codeLensFontFamily", e[e.codeLensFontSize = 18] = "codeLensFontSize", e[e.colorDecorators = 19] = "colorDecorators", e[e.colorDecoratorsLimit = 20] = "colorDecoratorsLimit", e[e.columnSelection = 21] = "columnSelection", e[e.comments = 22] = "comments", e[e.contextmenu = 23] = "contextmenu", e[e.copyWithSyntaxHighlighting = 24] = "copyWithSyntaxHighlighting", e[e.cursorBlinking = 25] = "cursorBlinking", e[e.cursorSmoothCaretAnimation = 26] = "cursorSmoothCaretAnimation", e[e.cursorStyle = 27] = "cursorStyle", e[e.cursorSurroundingLines = 28] = "cursorSurroundingLines", e[e.cursorSurroundingLinesStyle = 29] = "cursorSurroundingLinesStyle", e[e.cursorWidth = 30] = "cursorWidth", e[e.disableLayerHinting = 31] = "disableLayerHinting", e[e.disableMonospaceOptimizations = 32] = "disableMonospaceOptimizations", e[e.domReadOnly = 33] = "domReadOnly", e[e.dragAndDrop = 34] = "dragAndDrop", e[e.dropIntoEditor = 35] = "dropIntoEditor", e[e.emptySelectionClipboard = 36] = "emptySelectionClipboard", e[e.experimentalWhitespaceRendering = 37] = "experimentalWhitespaceRendering", e[e.extraEditorClassName = 38] = "extraEditorClassName", e[e.fastScrollSensitivity = 39] = "fastScrollSensitivity", e[e.find = 40] = "find", e[e.fixedOverflowWidgets = 41] = "fixedOverflowWidgets", e[e.folding = 42] = "folding", e[e.foldingStrategy = 43] = "foldingStrategy", e[e.foldingHighlight = 44] = "foldingHighlight", e[e.foldingImportsByDefault = 45] = "foldingImportsByDefault", e[e.foldingMaximumRegions = 46] = "foldingMaximumRegions", e[e.unfoldOnClickAfterEndOfLine = 47] = "unfoldOnClickAfterEndOfLine", e[e.fontFamily = 48] = "fontFamily", e[e.fontInfo = 49] = "fontInfo", e[e.fontLigatures = 50] = "fontLigatures", e[e.fontSize = 51] = "fontSize", e[e.fontWeight = 52] = "fontWeight", e[e.fontVariations = 53] = "fontVariations", e[e.formatOnPaste = 54] = "formatOnPaste", e[e.formatOnType = 55] = "formatOnType", e[e.glyphMargin = 56] = "glyphMargin", e[e.gotoLocation = 57] = "gotoLocation", e[e.hideCursorInOverviewRuler = 58] = "hideCursorInOverviewRuler", e[e.hover = 59] = "hover", e[e.inDiffEditor = 60] = "inDiffEditor", e[e.inlineSuggest = 61] = "inlineSuggest", e[e.letterSpacing = 62] = "letterSpacing", e[e.lightbulb = 63] = "lightbulb", e[e.lineDecorationsWidth = 64] = "lineDecorationsWidth", e[e.lineHeight = 65] = "lineHeight", e[e.lineNumbers = 66] = "lineNumbers", e[e.lineNumbersMinChars = 67] = "lineNumbersMinChars", e[e.linkedEditing = 68] = "linkedEditing", e[e.links = 69] = "links", e[e.matchBrackets = 70] = "matchBrackets", e[e.minimap = 71] = "minimap", e[e.mouseStyle = 72] = "mouseStyle", e[e.mouseWheelScrollSensitivity = 73] = "mouseWheelScrollSensitivity", e[e.mouseWheelZoom = 74] = "mouseWheelZoom", e[e.multiCursorMergeOverlapping = 75] = "multiCursorMergeOverlapping", e[e.multiCursorModifier = 76] = "multiCursorModifier", e[e.multiCursorPaste = 77] = "multiCursorPaste", e[e.multiCursorLimit = 78] = "multiCursorLimit", e[e.occurrencesHighlight = 79] = "occurrencesHighlight", e[e.overviewRulerBorder = 80] = "overviewRulerBorder", e[e.overviewRulerLanes = 81] = "overviewRulerLanes", e[e.padding = 82] = "padding", e[e.pasteAs = 83] = "pasteAs", e[e.parameterHints = 84] = "parameterHints", e[e.peekWidgetDefaultFocus = 85] = "peekWidgetDefaultFocus", e[e.definitionLinkOpensInPeek = 86] = "definitionLinkOpensInPeek", e[e.quickSuggestions = 87] = "quickSuggestions", e[e.quickSuggestionsDelay = 88] = "quickSuggestionsDelay", e[e.readOnly = 89] = "readOnly", e[e.readOnlyMessage = 90] = "readOnlyMessage", e[e.renameOnType = 91] = "renameOnType", e[e.renderControlCharacters = 92] = "renderControlCharacters", e[e.renderFinalNewline = 93] = "renderFinalNewline", e[e.renderLineHighlight = 94] = "renderLineHighlight", e[e.renderLineHighlightOnlyWhenFocus = 95] = "renderLineHighlightOnlyWhenFocus", e[e.renderValidationDecorations = 96] = "renderValidationDecorations", e[e.renderWhitespace = 97] = "renderWhitespace", e[e.revealHorizontalRightPadding = 98] = "revealHorizontalRightPadding", e[e.roundedSelection = 99] = "roundedSelection", e[e.rulers = 100] = "rulers", e[e.scrollbar = 101] = "scrollbar", e[e.scrollBeyondLastColumn = 102] = "scrollBeyondLastColumn", e[e.scrollBeyondLastLine = 103] = "scrollBeyondLastLine", e[e.scrollPredominantAxis = 104] = "scrollPredominantAxis", e[e.selectionClipboard = 105] = "selectionClipboard", e[e.selectionHighlight = 106] = "selectionHighlight", e[e.selectOnLineNumbers = 107] = "selectOnLineNumbers", e[e.showFoldingControls = 108] = "showFoldingControls", e[e.showUnused = 109] = "showUnused", e[e.snippetSuggestions = 110] = "snippetSuggestions", e[e.smartSelect = 111] = "smartSelect", e[e.smoothScrolling = 112] = "smoothScrolling", e[e.stickyScroll = 113] = "stickyScroll", e[e.stickyTabStops = 114] = "stickyTabStops", e[e.stopRenderingLineAfter = 115] = "stopRenderingLineAfter", e[e.suggest = 116] = "suggest", e[e.suggestFontSize = 117] = "suggestFontSize", e[e.suggestLineHeight = 118] = "suggestLineHeight", e[e.suggestOnTriggerCharacters = 119] = "suggestOnTriggerCharacters", e[e.suggestSelection = 120] = "suggestSelection", e[e.tabCompletion = 121] = "tabCompletion", e[e.tabIndex = 122] = "tabIndex", e[e.unicodeHighlighting = 123] = "unicodeHighlighting", e[e.unusualLineTerminators = 124] = "unusualLineTerminators", e[e.useShadowDOM = 125] = "useShadowDOM", e[e.useTabStops = 126] = "useTabStops", e[e.wordBreak = 127] = "wordBreak", e[e.wordSeparators = 128] = "wordSeparators", e[e.wordWrap = 129] = "wordWrap", e[e.wordWrapBreakAfterCharacters = 130] = "wordWrapBreakAfterCharacters", e[e.wordWrapBreakBeforeCharacters = 131] = "wordWrapBreakBeforeCharacters", e[e.wordWrapColumn = 132] = "wordWrapColumn", e[e.wordWrapOverride1 = 133] = "wordWrapOverride1", e[e.wordWrapOverride2 = 134] = "wordWrapOverride2", e[e.wrappingIndent = 135] = "wrappingIndent", e[e.wrappingStrategy = 136] = "wrappingStrategy", e[e.showDeprecated = 137] = "showDeprecated", e[e.inlayHints = 138] = "inlayHints", e[e.editorClassName = 139] = "editorClassName", e[e.pixelRatio = 140] = "pixelRatio", e[e.tabFocusMode = 141] = "tabFocusMode", e[e.layoutInfo = 142] = "layoutInfo", e[e.wrappingInfo = 143] = "wrappingInfo", e[e.defaultColorDecorators = 144] = "defaultColorDecorators", e[e.colorDecoratorsActivatedOn = 145] = "colorDecoratorsActivatedOn";
})(gn || (gn = {}));
var bn;
(function(e) {
  e[e.TextDefined = 0] = "TextDefined", e[e.LF = 1] = "LF", e[e.CRLF = 2] = "CRLF";
})(bn || (bn = {}));
var _n;
(function(e) {
  e[e.LF = 0] = "LF", e[e.CRLF = 1] = "CRLF";
})(_n || (_n = {}));
var xn;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Right = 2] = "Right";
})(xn || (xn = {}));
var pn;
(function(e) {
  e[e.None = 0] = "None", e[e.Indent = 1] = "Indent", e[e.IndentOutdent = 2] = "IndentOutdent", e[e.Outdent = 3] = "Outdent";
})(pn || (pn = {}));
var vn;
(function(e) {
  e[e.Both = 0] = "Both", e[e.Right = 1] = "Right", e[e.Left = 2] = "Left", e[e.None = 3] = "None";
})(vn || (vn = {}));
var wn;
(function(e) {
  e[e.Type = 1] = "Type", e[e.Parameter = 2] = "Parameter";
})(wn || (wn = {}));
var Ln;
(function(e) {
  e[e.Automatic = 0] = "Automatic", e[e.Explicit = 1] = "Explicit";
})(Ln || (Ln = {}));
var St;
(function(e) {
  e[e.DependsOnKbLayout = -1] = "DependsOnKbLayout", e[e.Unknown = 0] = "Unknown", e[e.Backspace = 1] = "Backspace", e[e.Tab = 2] = "Tab", e[e.Enter = 3] = "Enter", e[e.Shift = 4] = "Shift", e[e.Ctrl = 5] = "Ctrl", e[e.Alt = 6] = "Alt", e[e.PauseBreak = 7] = "PauseBreak", e[e.CapsLock = 8] = "CapsLock", e[e.Escape = 9] = "Escape", e[e.Space = 10] = "Space", e[e.PageUp = 11] = "PageUp", e[e.PageDown = 12] = "PageDown", e[e.End = 13] = "End", e[e.Home = 14] = "Home", e[e.LeftArrow = 15] = "LeftArrow", e[e.UpArrow = 16] = "UpArrow", e[e.RightArrow = 17] = "RightArrow", e[e.DownArrow = 18] = "DownArrow", e[e.Insert = 19] = "Insert", e[e.Delete = 20] = "Delete", e[e.Digit0 = 21] = "Digit0", e[e.Digit1 = 22] = "Digit1", e[e.Digit2 = 23] = "Digit2", e[e.Digit3 = 24] = "Digit3", e[e.Digit4 = 25] = "Digit4", e[e.Digit5 = 26] = "Digit5", e[e.Digit6 = 27] = "Digit6", e[e.Digit7 = 28] = "Digit7", e[e.Digit8 = 29] = "Digit8", e[e.Digit9 = 30] = "Digit9", e[e.KeyA = 31] = "KeyA", e[e.KeyB = 32] = "KeyB", e[e.KeyC = 33] = "KeyC", e[e.KeyD = 34] = "KeyD", e[e.KeyE = 35] = "KeyE", e[e.KeyF = 36] = "KeyF", e[e.KeyG = 37] = "KeyG", e[e.KeyH = 38] = "KeyH", e[e.KeyI = 39] = "KeyI", e[e.KeyJ = 40] = "KeyJ", e[e.KeyK = 41] = "KeyK", e[e.KeyL = 42] = "KeyL", e[e.KeyM = 43] = "KeyM", e[e.KeyN = 44] = "KeyN", e[e.KeyO = 45] = "KeyO", e[e.KeyP = 46] = "KeyP", e[e.KeyQ = 47] = "KeyQ", e[e.KeyR = 48] = "KeyR", e[e.KeyS = 49] = "KeyS", e[e.KeyT = 50] = "KeyT", e[e.KeyU = 51] = "KeyU", e[e.KeyV = 52] = "KeyV", e[e.KeyW = 53] = "KeyW", e[e.KeyX = 54] = "KeyX", e[e.KeyY = 55] = "KeyY", e[e.KeyZ = 56] = "KeyZ", e[e.Meta = 57] = "Meta", e[e.ContextMenu = 58] = "ContextMenu", e[e.F1 = 59] = "F1", e[e.F2 = 60] = "F2", e[e.F3 = 61] = "F3", e[e.F4 = 62] = "F4", e[e.F5 = 63] = "F5", e[e.F6 = 64] = "F6", e[e.F7 = 65] = "F7", e[e.F8 = 66] = "F8", e[e.F9 = 67] = "F9", e[e.F10 = 68] = "F10", e[e.F11 = 69] = "F11", e[e.F12 = 70] = "F12", e[e.F13 = 71] = "F13", e[e.F14 = 72] = "F14", e[e.F15 = 73] = "F15", e[e.F16 = 74] = "F16", e[e.F17 = 75] = "F17", e[e.F18 = 76] = "F18", e[e.F19 = 77] = "F19", e[e.F20 = 78] = "F20", e[e.F21 = 79] = "F21", e[e.F22 = 80] = "F22", e[e.F23 = 81] = "F23", e[e.F24 = 82] = "F24", e[e.NumLock = 83] = "NumLock", e[e.ScrollLock = 84] = "ScrollLock", e[e.Semicolon = 85] = "Semicolon", e[e.Equal = 86] = "Equal", e[e.Comma = 87] = "Comma", e[e.Minus = 88] = "Minus", e[e.Period = 89] = "Period", e[e.Slash = 90] = "Slash", e[e.Backquote = 91] = "Backquote", e[e.BracketLeft = 92] = "BracketLeft", e[e.Backslash = 93] = "Backslash", e[e.BracketRight = 94] = "BracketRight", e[e.Quote = 95] = "Quote", e[e.OEM_8 = 96] = "OEM_8", e[e.IntlBackslash = 97] = "IntlBackslash", e[e.Numpad0 = 98] = "Numpad0", e[e.Numpad1 = 99] = "Numpad1", e[e.Numpad2 = 100] = "Numpad2", e[e.Numpad3 = 101] = "Numpad3", e[e.Numpad4 = 102] = "Numpad4", e[e.Numpad5 = 103] = "Numpad5", e[e.Numpad6 = 104] = "Numpad6", e[e.Numpad7 = 105] = "Numpad7", e[e.Numpad8 = 106] = "Numpad8", e[e.Numpad9 = 107] = "Numpad9", e[e.NumpadMultiply = 108] = "NumpadMultiply", e[e.NumpadAdd = 109] = "NumpadAdd", e[e.NUMPAD_SEPARATOR = 110] = "NUMPAD_SEPARATOR", e[e.NumpadSubtract = 111] = "NumpadSubtract", e[e.NumpadDecimal = 112] = "NumpadDecimal", e[e.NumpadDivide = 113] = "NumpadDivide", e[e.KEY_IN_COMPOSITION = 114] = "KEY_IN_COMPOSITION", e[e.ABNT_C1 = 115] = "ABNT_C1", e[e.ABNT_C2 = 116] = "ABNT_C2", e[e.AudioVolumeMute = 117] = "AudioVolumeMute", e[e.AudioVolumeUp = 118] = "AudioVolumeUp", e[e.AudioVolumeDown = 119] = "AudioVolumeDown", e[e.BrowserSearch = 120] = "BrowserSearch", e[e.BrowserHome = 121] = "BrowserHome", e[e.BrowserBack = 122] = "BrowserBack", e[e.BrowserForward = 123] = "BrowserForward", e[e.MediaTrackNext = 124] = "MediaTrackNext", e[e.MediaTrackPrevious = 125] = "MediaTrackPrevious", e[e.MediaStop = 126] = "MediaStop", e[e.MediaPlayPause = 127] = "MediaPlayPause", e[e.LaunchMediaPlayer = 128] = "LaunchMediaPlayer", e[e.LaunchMail = 129] = "LaunchMail", e[e.LaunchApp2 = 130] = "LaunchApp2", e[e.Clear = 131] = "Clear", e[e.MAX_VALUE = 132] = "MAX_VALUE";
})(St || (St = {}));
var At;
(function(e) {
  e[e.Hint = 1] = "Hint", e[e.Info = 2] = "Info", e[e.Warning = 4] = "Warning", e[e.Error = 8] = "Error";
})(At || (At = {}));
var Ct;
(function(e) {
  e[e.Unnecessary = 1] = "Unnecessary", e[e.Deprecated = 2] = "Deprecated";
})(Ct || (Ct = {}));
var Nn;
(function(e) {
  e[e.Inline = 1] = "Inline", e[e.Gutter = 2] = "Gutter";
})(Nn || (Nn = {}));
var Sn;
(function(e) {
  e[e.UNKNOWN = 0] = "UNKNOWN", e[e.TEXTAREA = 1] = "TEXTAREA", e[e.GUTTER_GLYPH_MARGIN = 2] = "GUTTER_GLYPH_MARGIN", e[e.GUTTER_LINE_NUMBERS = 3] = "GUTTER_LINE_NUMBERS", e[e.GUTTER_LINE_DECORATIONS = 4] = "GUTTER_LINE_DECORATIONS", e[e.GUTTER_VIEW_ZONE = 5] = "GUTTER_VIEW_ZONE", e[e.CONTENT_TEXT = 6] = "CONTENT_TEXT", e[e.CONTENT_EMPTY = 7] = "CONTENT_EMPTY", e[e.CONTENT_VIEW_ZONE = 8] = "CONTENT_VIEW_ZONE", e[e.CONTENT_WIDGET = 9] = "CONTENT_WIDGET", e[e.OVERVIEW_RULER = 10] = "OVERVIEW_RULER", e[e.SCROLLBAR = 11] = "SCROLLBAR", e[e.OVERLAY_WIDGET = 12] = "OVERLAY_WIDGET", e[e.OUTSIDE_EDITOR = 13] = "OUTSIDE_EDITOR";
})(Sn || (Sn = {}));
var An;
(function(e) {
  e[e.TOP_RIGHT_CORNER = 0] = "TOP_RIGHT_CORNER", e[e.BOTTOM_RIGHT_CORNER = 1] = "BOTTOM_RIGHT_CORNER", e[e.TOP_CENTER = 2] = "TOP_CENTER";
})(An || (An = {}));
var Cn;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Center = 2] = "Center", e[e.Right = 4] = "Right", e[e.Full = 7] = "Full";
})(Cn || (Cn = {}));
var Rn;
(function(e) {
  e[e.Left = 0] = "Left", e[e.Right = 1] = "Right", e[e.None = 2] = "None", e[e.LeftOfInjectedText = 3] = "LeftOfInjectedText", e[e.RightOfInjectedText = 4] = "RightOfInjectedText";
})(Rn || (Rn = {}));
var yn;
(function(e) {
  e[e.Off = 0] = "Off", e[e.On = 1] = "On", e[e.Relative = 2] = "Relative", e[e.Interval = 3] = "Interval", e[e.Custom = 4] = "Custom";
})(yn || (yn = {}));
var Mn;
(function(e) {
  e[e.None = 0] = "None", e[e.Text = 1] = "Text", e[e.Blocks = 2] = "Blocks";
})(Mn || (Mn = {}));
var kn;
(function(e) {
  e[e.Smooth = 0] = "Smooth", e[e.Immediate = 1] = "Immediate";
})(kn || (kn = {}));
var En;
(function(e) {
  e[e.Auto = 1] = "Auto", e[e.Hidden = 2] = "Hidden", e[e.Visible = 3] = "Visible";
})(En || (En = {}));
var Rt;
(function(e) {
  e[e.LTR = 0] = "LTR", e[e.RTL = 1] = "RTL";
})(Rt || (Rt = {}));
var Fn;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.TriggerCharacter = 2] = "TriggerCharacter", e[e.ContentChange = 3] = "ContentChange";
})(Fn || (Fn = {}));
var Pn;
(function(e) {
  e[e.File = 0] = "File", e[e.Module = 1] = "Module", e[e.Namespace = 2] = "Namespace", e[e.Package = 3] = "Package", e[e.Class = 4] = "Class", e[e.Method = 5] = "Method", e[e.Property = 6] = "Property", e[e.Field = 7] = "Field", e[e.Constructor = 8] = "Constructor", e[e.Enum = 9] = "Enum", e[e.Interface = 10] = "Interface", e[e.Function = 11] = "Function", e[e.Variable = 12] = "Variable", e[e.Constant = 13] = "Constant", e[e.String = 14] = "String", e[e.Number = 15] = "Number", e[e.Boolean = 16] = "Boolean", e[e.Array = 17] = "Array", e[e.Object = 18] = "Object", e[e.Key = 19] = "Key", e[e.Null = 20] = "Null", e[e.EnumMember = 21] = "EnumMember", e[e.Struct = 22] = "Struct", e[e.Event = 23] = "Event", e[e.Operator = 24] = "Operator", e[e.TypeParameter = 25] = "TypeParameter";
})(Pn || (Pn = {}));
var Dn;
(function(e) {
  e[e.Deprecated = 1] = "Deprecated";
})(Dn || (Dn = {}));
var Vn;
(function(e) {
  e[e.Hidden = 0] = "Hidden", e[e.Blink = 1] = "Blink", e[e.Smooth = 2] = "Smooth", e[e.Phase = 3] = "Phase", e[e.Expand = 4] = "Expand", e[e.Solid = 5] = "Solid";
})(Vn || (Vn = {}));
var Tn;
(function(e) {
  e[e.Line = 1] = "Line", e[e.Block = 2] = "Block", e[e.Underline = 3] = "Underline", e[e.LineThin = 4] = "LineThin", e[e.BlockOutline = 5] = "BlockOutline", e[e.UnderlineThin = 6] = "UnderlineThin";
})(Tn || (Tn = {}));
var Bn;
(function(e) {
  e[e.AlwaysGrowsWhenTypingAtEdges = 0] = "AlwaysGrowsWhenTypingAtEdges", e[e.NeverGrowsWhenTypingAtEdges = 1] = "NeverGrowsWhenTypingAtEdges", e[e.GrowsOnlyWhenTypingBefore = 2] = "GrowsOnlyWhenTypingBefore", e[e.GrowsOnlyWhenTypingAfter = 3] = "GrowsOnlyWhenTypingAfter";
})(Bn || (Bn = {}));
var In;
(function(e) {
  e[e.None = 0] = "None", e[e.Same = 1] = "Same", e[e.Indent = 2] = "Indent", e[e.DeepIndent = 3] = "DeepIndent";
})(In || (In = {}));
class $e {
  static chord(t, n) {
    return Zs(t, n);
  }
}
$e.CtrlCmd = 2048;
$e.Shift = 1024;
$e.Alt = 512;
$e.WinCtrl = 256;
function ni() {
  return {
    editor: void 0,
    languages: void 0,
    CancellationTokenSource: Qs,
    Emitter: se,
    KeyCode: St,
    KeyMod: $e,
    Position: O,
    Range: P,
    Selection: Z,
    SelectionDirection: Rt,
    MarkerSeverity: At,
    MarkerTag: Ct,
    Uri: xe,
    Token: ti
  };
}
var Un;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Center = 2] = "Center", e[e.Right = 4] = "Right", e[e.Full = 7] = "Full";
})(Un || (Un = {}));
var qn;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Right = 2] = "Right";
})(qn || (qn = {}));
var Hn;
(function(e) {
  e[e.Inline = 1] = "Inline", e[e.Gutter = 2] = "Gutter";
})(Hn || (Hn = {}));
var Wn;
(function(e) {
  e[e.Both = 0] = "Both", e[e.Right = 1] = "Right", e[e.Left = 2] = "Left", e[e.None = 3] = "None";
})(Wn || (Wn = {}));
function ri(e, t, n, r, s) {
  if (r === 0)
    return !0;
  const i = t.charCodeAt(r - 1);
  if (e.get(i) !== 0 || i === 13 || i === 10)
    return !0;
  if (s > 0) {
    const l = t.charCodeAt(r);
    if (e.get(l) !== 0)
      return !0;
  }
  return !1;
}
function si(e, t, n, r, s) {
  if (r + s === n)
    return !0;
  const i = t.charCodeAt(r + s);
  if (e.get(i) !== 0 || i === 13 || i === 10)
    return !0;
  if (s > 0) {
    const l = t.charCodeAt(r + s - 1);
    if (e.get(l) !== 0)
      return !0;
  }
  return !1;
}
function ii(e, t, n, r, s) {
  return ri(e, t, n, r, s) && si(e, t, n, r, s);
}
class ai {
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
          as(t, n, this._searchRegex.lastIndex) > 65535 ? this._searchRegex.lastIndex += 2 : this._searchRegex.lastIndex += 1;
          continue;
        }
        return null;
      }
      if (this._prevMatchStartIndex = s, this._prevMatchLength = i, !this._wordSeparators || ii(this._wordSeparators, t, n, s, i))
        return r;
    } while (r);
    return null;
  }
}
function li(e, t = "Unreachable") {
  throw new Error(t);
}
function tt(e) {
  if (!e()) {
    debugger;
    e(), lr(new Pe("Assertion Failed"));
  }
}
function Nr(e, t) {
  let n = 0;
  for (; n < e.length - 1; ) {
    const r = e[n], s = e[n + 1];
    if (!t(r, s))
      return !1;
    n++;
  }
  return !0;
}
class oi {
  static computeUnicodeHighlights(t, n, r) {
    const s = r ? r.startLineNumber : 1, i = r ? r.endLineNumber : t.getLineCount(), l = new $n(n), o = l.getCandidateCodePoints();
    let c;
    o === "allNonBasicAscii" ? c = new RegExp("[^\\t\\n\\r\\x20-\\x7E]", "g") : c = new RegExp(`${ui(Array.from(o))}`, "g");
    const u = new ai(null, c), h = [];
    let f = !1, d, m = 0, g = 0, _ = 0;
    e:
      for (let S = s, b = i; S <= b; S++) {
        const N = t.getLineContent(S), w = N.length;
        u.reset(0);
        do
          if (d = u.next(N), d) {
            let k = d.index, A = d.index + d[0].length;
            if (k > 0) {
              const L = N.charCodeAt(k - 1);
              bt(L) && k--;
            }
            if (A + 1 < w) {
              const L = N.charCodeAt(A - 1);
              bt(L) && A++;
            }
            const E = N.substring(k, A);
            let C = Ft(k + 1, vr, N, 0);
            C && C.endColumn <= k + 1 && (C = null);
            const R = l.shouldHighlightNonBasicASCII(E, C ? C.word : null);
            if (R !== 0) {
              R === 3 ? m++ : R === 2 ? g++ : R === 1 ? _++ : li();
              const L = 1e3;
              if (h.length >= L) {
                f = !0;
                break e;
              }
              h.push(new P(S, k + 1, S, A + 1));
            }
          }
        while (d);
      }
    return {
      ranges: h,
      hasMore: f,
      ambiguousCharacterCount: m,
      invisibleCharacterCount: g,
      nonBasicAsciiCharacterCount: _
    };
  }
  static computeUnicodeHighlightReason(t, n) {
    const r = new $n(n);
    switch (r.shouldHighlightNonBasicASCII(t, null)) {
      case 0:
        return null;
      case 2:
        return {
          kind: 1
          /* UnicodeHighlighterReasonKind.Invisible */
        };
      case 3: {
        const i = t.codePointAt(0), l = r.ambiguousCharacters.getPrimaryConfusable(i), o = ee.getLocales().filter((c) => !ee.getInstance(/* @__PURE__ */ new Set([...n.allowedLocales, c])).isAmbiguous(i));
        return { kind: 0, confusableWith: String.fromCodePoint(l), notAmbiguousInLocales: o };
      }
      case 1:
        return {
          kind: 2
          /* UnicodeHighlighterReasonKind.NonBasicAscii */
        };
    }
  }
}
function ui(e, t) {
  return `[${es(e.map((r) => String.fromCodePoint(r)).join(""))}]`;
}
class $n {
  constructor(t) {
    this.options = t, this.allowedCodePoints = new Set(t.allowedCodePoints), this.ambiguousCharacters = ee.getInstance(new Set(t.allowedLocales));
  }
  getCandidateCodePoints() {
    if (this.options.nonBasicASCII)
      return "allNonBasicAscii";
    const t = /* @__PURE__ */ new Set();
    if (this.options.invisibleCharacters)
      for (const n of de.codePoints)
        zn(String.fromCodePoint(n)) || t.add(n);
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
      for (const l of n) {
        const o = l.codePointAt(0), c = os(l);
        s = s || c, !c && !this.ambiguousCharacters.isAmbiguous(o) && !de.isInvisibleCharacter(o) && (i = !0);
      }
    return (
      /* Don't allow mixing weird looking characters with ASCII */
      !s && /* Is there an obviously weird looking character? */
      i ? 0 : this.options.invisibleCharacters && !zn(t) && de.isInvisibleCharacter(r) ? 2 : this.options.ambiguousCharacters && this.ambiguousCharacters.isAmbiguous(r) ? 3 : 0
    );
  }
}
function zn(e) {
  return e === " " || e === `
` || e === "	";
}
class H {
  static fromRange(t) {
    return new H(t.startLineNumber, t.endLineNumber);
  }
  static subtract(t, n) {
    return n ? t.startLineNumber < n.startLineNumber && n.endLineNumberExclusive < t.endLineNumberExclusive ? [
      new H(t.startLineNumber, n.startLineNumber),
      new H(n.endLineNumberExclusive, t.endLineNumberExclusive)
    ] : n.startLineNumber <= t.startLineNumber && t.endLineNumberExclusive <= n.endLineNumberExclusive ? [] : n.endLineNumberExclusive < t.endLineNumberExclusive ? [new H(Math.max(n.endLineNumberExclusive, t.startLineNumber), t.endLineNumberExclusive)] : [new H(t.startLineNumber, Math.min(n.startLineNumber, t.endLineNumberExclusive))] : [t];
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
    let s = 0, i = 0, l = null;
    for (; s < t.length || i < n.length; ) {
      let o = null;
      if (s < t.length && i < n.length) {
        const c = t[s], u = n[i];
        c.startLineNumber < u.startLineNumber ? (o = c, s++) : (o = u, i++);
      } else
        s < t.length ? (o = t[s], s++) : (o = n[i], i++);
      l === null ? l = o : l.endLineNumberExclusive >= o.startLineNumber ? l = new H(l.startLineNumber, Math.max(l.endLineNumberExclusive, o.endLineNumberExclusive)) : (r.push(l), l = o);
    }
    return l !== null && r.push(l), r;
  }
  static ofLength(t, n) {
    return new H(t, t + n);
  }
  /**
   * @internal
   */
  static deserialize(t) {
    return new H(t[0], t[1]);
  }
  constructor(t, n) {
    if (t > n)
      throw new Pe(`startLineNumber ${t} cannot be after endLineNumberExclusive ${n}`);
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
    return new H(this.startLineNumber + t, this.endLineNumberExclusive + t);
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
    return new H(Math.min(this.startLineNumber, t.startLineNumber), Math.max(this.endLineNumberExclusive, t.endLineNumberExclusive));
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
      return new H(n, r);
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
    return this.isEmpty ? null : new P(this.startLineNumber, 1, this.endLineNumberExclusive - 1, Number.MAX_SAFE_INTEGER);
  }
  toExclusiveRange() {
    return new P(this.startLineNumber, 1, this.endLineNumberExclusive, 1);
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
}
class Sr {
  constructor(t, n, r) {
    this.changes = t, this.moves = n, this.hitTimeout = r;
  }
}
class ae {
  static inverse(t, n, r) {
    const s = [];
    let i = 1, l = 1;
    for (const c of t) {
      const u = new ae(new H(i, c.originalRange.startLineNumber), new H(l, c.modifiedRange.startLineNumber), void 0);
      u.modifiedRange.isEmpty || s.push(u), i = c.originalRange.endLineNumberExclusive, l = c.modifiedRange.endLineNumberExclusive;
    }
    const o = new ae(new H(i, n + 1), new H(l, r + 1), void 0);
    return o.modifiedRange.isEmpty || s.push(o), s;
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
    return new ae(this.modifiedRange, this.originalRange, (t = this.innerChanges) === null || t === void 0 ? void 0 : t.map((n) => n.flip()));
  }
}
class He {
  constructor(t, n) {
    this.originalRange = t, this.modifiedRange = n;
  }
  toString() {
    return `{${this.originalRange.toString()}->${this.modifiedRange.toString()}}`;
  }
  flip() {
    return new He(this.modifiedRange, this.originalRange);
  }
}
class Vt {
  constructor(t, n) {
    this.original = t, this.modified = n;
  }
  toString() {
    return `{${this.original.toString()}->${this.modified.toString()}}`;
  }
  flip() {
    return new Vt(this.modified, this.original);
  }
}
class Tt {
  constructor(t, n) {
    this.lineRangeMapping = t, this.changes = n;
  }
  flip() {
    return new Tt(this.lineRangeMapping.flip(), this.changes.map((t) => t.flip()));
  }
}
const ci = 3;
class hi {
  computeDiff(t, n, r) {
    var s;
    const l = new mi(t, n, {
      maxComputationTime: r.maxComputationTimeMs,
      shouldIgnoreTrimWhitespace: r.ignoreTrimWhitespace,
      shouldComputeCharChanges: !0,
      shouldMakePrettyDiff: !0,
      shouldPostProcessCharChanges: !0
    }).computeDiff(), o = [];
    let c = null;
    for (const u of l.changes) {
      let h;
      u.originalEndLineNumber === 0 ? h = new H(u.originalStartLineNumber + 1, u.originalStartLineNumber + 1) : h = new H(u.originalStartLineNumber, u.originalEndLineNumber + 1);
      let f;
      u.modifiedEndLineNumber === 0 ? f = new H(u.modifiedStartLineNumber + 1, u.modifiedStartLineNumber + 1) : f = new H(u.modifiedStartLineNumber, u.modifiedEndLineNumber + 1);
      let d = new ae(h, f, (s = u.charChanges) === null || s === void 0 ? void 0 : s.map((m) => new He(new P(m.originalStartLineNumber, m.originalStartColumn, m.originalEndLineNumber, m.originalEndColumn), new P(m.modifiedStartLineNumber, m.modifiedStartColumn, m.modifiedEndLineNumber, m.modifiedEndColumn))));
      c && (c.modifiedRange.endLineNumberExclusive === d.modifiedRange.startLineNumber || c.originalRange.endLineNumberExclusive === d.originalRange.startLineNumber) && (d = new ae(c.originalRange.join(d.originalRange), c.modifiedRange.join(d.modifiedRange), c.innerChanges && d.innerChanges ? c.innerChanges.concat(d.innerChanges) : void 0), o.pop()), o.push(d), c = d;
    }
    return tt(() => Nr(o, (u, h) => h.originalRange.startLineNumber - u.originalRange.endLineNumberExclusive === h.modifiedRange.startLineNumber - u.modifiedRange.endLineNumberExclusive && // There has to be an unchanged line in between (otherwise both diffs should have been joined)
    u.originalRange.endLineNumberExclusive < h.originalRange.startLineNumber && u.modifiedRange.endLineNumberExclusive < h.modifiedRange.startLineNumber)), new Sr(o, [], l.quitEarly);
  }
}
function Ar(e, t, n, r) {
  return new fe(e, t, n).ComputeDiff(r);
}
let Gn = class {
  constructor(t) {
    const n = [], r = [];
    for (let s = 0, i = t.length; s < i; s++)
      n[s] = yt(t[s], 1), r[s] = Mt(t[s], 1);
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
    const s = [], i = [], l = [];
    let o = 0;
    for (let c = n; c <= r; c++) {
      const u = this.lines[c], h = t ? this._startColumns[c] : 1, f = t ? this._endColumns[c] : u.length + 1;
      for (let d = h; d < f; d++)
        s[o] = u.charCodeAt(d - 1), i[o] = c + 1, l[o] = d, o++;
      !t && c < r && (s[o] = 10, i[o] = c + 1, l[o] = u.length + 1, o++);
    }
    return new fi(s, i, l);
  }
};
class fi {
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
class ke {
  constructor(t, n, r, s, i, l, o, c) {
    this.originalStartLineNumber = t, this.originalStartColumn = n, this.originalEndLineNumber = r, this.originalEndColumn = s, this.modifiedStartLineNumber = i, this.modifiedStartColumn = l, this.modifiedEndLineNumber = o, this.modifiedEndColumn = c;
  }
  static createFromDiffChange(t, n, r) {
    const s = n.getStartLineNumber(t.originalStart), i = n.getStartColumn(t.originalStart), l = n.getEndLineNumber(t.originalStart + t.originalLength - 1), o = n.getEndColumn(t.originalStart + t.originalLength - 1), c = r.getStartLineNumber(t.modifiedStart), u = r.getStartColumn(t.modifiedStart), h = r.getEndLineNumber(t.modifiedStart + t.modifiedLength - 1), f = r.getEndColumn(t.modifiedStart + t.modifiedLength - 1);
    return new ke(s, i, l, o, c, u, h, f);
  }
}
function di(e) {
  if (e.length <= 1)
    return e;
  const t = [e[0]];
  let n = t[0];
  for (let r = 1, s = e.length; r < s; r++) {
    const i = e[r], l = i.originalStart - (n.originalStart + n.originalLength), o = i.modifiedStart - (n.modifiedStart + n.modifiedLength);
    Math.min(l, o) < ci ? (n.originalLength = i.originalStart + i.originalLength - n.originalStart, n.modifiedLength = i.modifiedStart + i.modifiedLength - n.modifiedStart) : (t.push(i), n = i);
  }
  return t;
}
class Be {
  constructor(t, n, r, s, i) {
    this.originalStartLineNumber = t, this.originalEndLineNumber = n, this.modifiedStartLineNumber = r, this.modifiedEndLineNumber = s, this.charChanges = i;
  }
  static createFromDiffResult(t, n, r, s, i, l, o) {
    let c, u, h, f, d;
    if (n.originalLength === 0 ? (c = r.getStartLineNumber(n.originalStart) - 1, u = 0) : (c = r.getStartLineNumber(n.originalStart), u = r.getEndLineNumber(n.originalStart + n.originalLength - 1)), n.modifiedLength === 0 ? (h = s.getStartLineNumber(n.modifiedStart) - 1, f = 0) : (h = s.getStartLineNumber(n.modifiedStart), f = s.getEndLineNumber(n.modifiedStart + n.modifiedLength - 1)), l && n.originalLength > 0 && n.originalLength < 20 && n.modifiedLength > 0 && n.modifiedLength < 20 && i()) {
      const m = r.createCharSequence(t, n.originalStart, n.originalStart + n.originalLength - 1), g = s.createCharSequence(t, n.modifiedStart, n.modifiedStart + n.modifiedLength - 1);
      if (m.getElements().length > 0 && g.getElements().length > 0) {
        let _ = Ar(m, g, i, !0).changes;
        o && (_ = di(_)), d = [];
        for (let S = 0, b = _.length; S < b; S++)
          d.push(ke.createFromDiffChange(_[S], m, g));
      }
    }
    return new Be(c, u, h, f, d);
  }
}
class mi {
  constructor(t, n, r) {
    this.shouldComputeCharChanges = r.shouldComputeCharChanges, this.shouldPostProcessCharChanges = r.shouldPostProcessCharChanges, this.shouldIgnoreTrimWhitespace = r.shouldIgnoreTrimWhitespace, this.shouldMakePrettyDiff = r.shouldMakePrettyDiff, this.originalLines = t, this.modifiedLines = n, this.original = new Gn(t), this.modified = new Gn(n), this.continueLineDiff = On(r.maxComputationTime), this.continueCharDiff = On(r.maxComputationTime === 0 ? 0 : Math.min(r.maxComputationTime, 5e3));
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
    const t = Ar(this.original, this.modified, this.continueLineDiff, this.shouldMakePrettyDiff), n = t.changes, r = t.quitEarly;
    if (this.shouldIgnoreTrimWhitespace) {
      const o = [];
      for (let c = 0, u = n.length; c < u; c++)
        o.push(Be.createFromDiffResult(this.shouldIgnoreTrimWhitespace, n[c], this.original, this.modified, this.continueCharDiff, this.shouldComputeCharChanges, this.shouldPostProcessCharChanges));
      return {
        quitEarly: r,
        changes: o
      };
    }
    const s = [];
    let i = 0, l = 0;
    for (let o = -1, c = n.length; o < c; o++) {
      const u = o + 1 < c ? n[o + 1] : null, h = u ? u.originalStart : this.originalLines.length, f = u ? u.modifiedStart : this.modifiedLines.length;
      for (; i < h && l < f; ) {
        const d = this.originalLines[i], m = this.modifiedLines[l];
        if (d !== m) {
          {
            let g = yt(d, 1), _ = yt(m, 1);
            for (; g > 1 && _ > 1; ) {
              const S = d.charCodeAt(g - 2), b = m.charCodeAt(_ - 2);
              if (S !== b)
                break;
              g--, _--;
            }
            (g > 1 || _ > 1) && this._pushTrimWhitespaceCharChange(s, i + 1, 1, g, l + 1, 1, _);
          }
          {
            let g = Mt(d, 1), _ = Mt(m, 1);
            const S = d.length + 1, b = m.length + 1;
            for (; g < S && _ < b; ) {
              const N = d.charCodeAt(g - 1), w = d.charCodeAt(_ - 1);
              if (N !== w)
                break;
              g++, _++;
            }
            (g < S || _ < b) && this._pushTrimWhitespaceCharChange(s, i + 1, g, S, l + 1, _, b);
          }
        }
        i++, l++;
      }
      u && (s.push(Be.createFromDiffResult(this.shouldIgnoreTrimWhitespace, u, this.original, this.modified, this.continueCharDiff, this.shouldComputeCharChanges, this.shouldPostProcessCharChanges)), i += u.originalLength, l += u.modifiedLength);
    }
    return {
      quitEarly: r,
      changes: s
    };
  }
  _pushTrimWhitespaceCharChange(t, n, r, s, i, l, o) {
    if (this._mergeTrimWhitespaceCharChange(t, n, r, s, i, l, o))
      return;
    let c;
    this.shouldComputeCharChanges && (c = [new ke(n, r, n, s, i, l, i, o)]), t.push(new Be(n, n, i, i, c));
  }
  _mergeTrimWhitespaceCharChange(t, n, r, s, i, l, o) {
    const c = t.length;
    if (c === 0)
      return !1;
    const u = t[c - 1];
    return u.originalEndLineNumber === 0 || u.modifiedEndLineNumber === 0 ? !1 : u.originalEndLineNumber === n && u.modifiedEndLineNumber === i ? (this.shouldComputeCharChanges && u.charChanges && u.charChanges.push(new ke(n, r, n, s, i, l, i, o)), !0) : u.originalEndLineNumber + 1 === n && u.modifiedEndLineNumber + 1 === i ? (u.originalEndLineNumber = n, u.modifiedEndLineNumber = i, this.shouldComputeCharChanges && u.charChanges && u.charChanges.push(new ke(n, r, n, s, i, l, i, o)), !0) : !1;
  }
}
function yt(e, t) {
  const n = ns(e);
  return n === -1 ? t : n + 1;
}
function Mt(e, t) {
  const n = rs(e);
  return n === -1 ? t : n + 2;
}
function On(e) {
  if (e === 0)
    return () => !0;
  const t = Date.now();
  return () => Date.now() - t < e;
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
      const i = Math.min(t.start, n[r].start), l = Math.max(t.endExclusive, n[s - 1].endExclusive);
      n.splice(r, s - r, new T(i, l));
    }
  }
  static tryCreate(t, n) {
    if (!(t > n))
      return new T(t, n);
  }
  constructor(t, n) {
    if (this.start = t, this.endExclusive = n, t > n)
      throw new Pe(`Invalid range: ${this.toString()}`);
  }
  get isEmpty() {
    return this.start === this.endExclusive;
  }
  delta(t) {
    return new T(this.start + t, this.endExclusive + t);
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
}
class le {
  static trivial(t, n) {
    return new le([new Y(new T(0, t.length), new T(0, n.length))], !1);
  }
  static trivialTimedOut(t, n) {
    return new le([new Y(new T(0, t.length), new T(0, n.length))], !0);
  }
  constructor(t, n) {
    this.diffs = t, this.hitTimeout = n;
  }
}
class Y {
  constructor(t, n) {
    this.seq1Range = t, this.seq2Range = n;
  }
  reverse() {
    return new Y(this.seq2Range, this.seq1Range);
  }
  toString() {
    return `${this.seq1Range} <-> ${this.seq2Range}`;
  }
  join(t) {
    return new Y(this.seq1Range.join(t.seq1Range), this.seq2Range.join(t.seq2Range));
  }
  delta(t) {
    return t === 0 ? this : new Y(this.seq1Range.delta(t), this.seq2Range.delta(t));
  }
}
class We {
  isValid() {
    return !0;
  }
}
We.instance = new We();
class gi {
  constructor(t) {
    if (this.timeout = t, this.startTime = Date.now(), this.valid = !0, t <= 0)
      throw new Pe("timeout must be positive");
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
class ot {
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
class bi {
  compute(t, n, r = We.instance, s) {
    if (t.length === 0 || n.length === 0)
      return le.trivial(t, n);
    const i = new ot(t.length, n.length), l = new ot(t.length, n.length), o = new ot(t.length, n.length);
    for (let g = 0; g < t.length; g++)
      for (let _ = 0; _ < n.length; _++) {
        if (!r.isValid())
          return le.trivialTimedOut(t, n);
        const S = g === 0 ? 0 : i.get(g - 1, _), b = _ === 0 ? 0 : i.get(g, _ - 1);
        let N;
        t.getElement(g) === n.getElement(_) ? (g === 0 || _ === 0 ? N = 0 : N = i.get(g - 1, _ - 1), g > 0 && _ > 0 && l.get(g - 1, _ - 1) === 3 && (N += o.get(g - 1, _ - 1)), N += s ? s(g, _) : 1) : N = -1;
        const w = Math.max(S, b, N);
        if (w === N) {
          const k = g > 0 && _ > 0 ? o.get(g - 1, _ - 1) : 0;
          o.set(g, _, k + 1), l.set(g, _, 3);
        } else
          w === S ? (o.set(g, _, 0), l.set(g, _, 1)) : w === b && (o.set(g, _, 0), l.set(g, _, 2));
        i.set(g, _, w);
      }
    const c = [];
    let u = t.length, h = n.length;
    function f(g, _) {
      (g + 1 !== u || _ + 1 !== h) && c.push(new Y(new T(g + 1, u), new T(_ + 1, h))), u = g, h = _;
    }
    let d = t.length - 1, m = n.length - 1;
    for (; d >= 0 && m >= 0; )
      l.get(d, m) === 3 ? (f(d, m), d--, m--) : l.get(d, m) === 1 ? d-- : m--;
    return f(-1, -1), c.reverse(), new le(c, !1);
  }
}
function jn(e, t, n) {
  let r = n;
  return r = pi(e, t, r), r = vi(e, t, r), r;
}
function _i(e, t, n) {
  const r = [];
  for (const s of n) {
    const i = r[r.length - 1];
    if (!i) {
      r.push(s);
      continue;
    }
    s.seq1Range.start - i.seq1Range.endExclusive <= 2 || s.seq2Range.start - i.seq2Range.endExclusive <= 2 ? r[r.length - 1] = new Y(i.seq1Range.join(s.seq1Range), i.seq2Range.join(s.seq2Range)) : r.push(s);
  }
  return r;
}
function xi(e, t, n) {
  let r = n;
  if (r.length === 0)
    return r;
  let s = 0, i;
  do {
    i = !1;
    const l = [
      r[0]
    ];
    for (let o = 1; o < r.length; o++) {
      let h = function(d, m) {
        const g = new T(u.seq1Range.endExclusive, c.seq1Range.start);
        if (e.countLinesIn(g) > 5 || g.length > 500)
          return !1;
        const S = e.getText(g).trim();
        if (S.length > 20 || S.split(/\r\n|\r|\n/).length > 1)
          return !1;
        const b = e.countLinesIn(d.seq1Range), N = d.seq1Range.length, w = t.countLinesIn(d.seq2Range), k = d.seq2Range.length, A = e.countLinesIn(m.seq1Range), E = m.seq1Range.length, C = t.countLinesIn(m.seq2Range), R = m.seq2Range.length, L = 2 * 40 + 50;
        function p(v) {
          return Math.min(v, L);
        }
        return Math.pow(Math.pow(p(b * 40 + N), 1.5) + Math.pow(p(w * 40 + k), 1.5), 1.5) + Math.pow(Math.pow(p(A * 40 + E), 1.5) + Math.pow(p(C * 40 + R), 1.5), 1.5) > Math.pow(Math.pow(L, 1.5), 1.5) * 1.3;
      };
      const c = r[o], u = l[l.length - 1];
      h(u, c) ? (i = !0, l[l.length - 1] = l[l.length - 1].join(c)) : l.push(c);
    }
    r = l;
  } while (s++ < 10 && i);
  return r;
}
function pi(e, t, n) {
  if (n.length === 0)
    return n;
  const r = [];
  r.push(n[0]);
  for (let i = 1; i < n.length; i++) {
    const l = r[r.length - 1];
    let o = n[i];
    if (o.seq1Range.isEmpty || o.seq2Range.isEmpty) {
      const c = o.seq1Range.start - l.seq1Range.endExclusive;
      let u;
      for (u = 1; u <= c && !(e.getElement(o.seq1Range.start - u) !== e.getElement(o.seq1Range.endExclusive - u) || t.getElement(o.seq2Range.start - u) !== t.getElement(o.seq2Range.endExclusive - u)); u++)
        ;
      if (u--, u === c) {
        r[r.length - 1] = new Y(new T(l.seq1Range.start, o.seq1Range.endExclusive - c), new T(l.seq2Range.start, o.seq2Range.endExclusive - c));
        continue;
      }
      o = o.delta(-u);
    }
    r.push(o);
  }
  const s = [];
  for (let i = 0; i < r.length - 1; i++) {
    const l = r[i + 1];
    let o = r[i];
    if (o.seq1Range.isEmpty || o.seq2Range.isEmpty) {
      const c = l.seq1Range.start - o.seq1Range.endExclusive;
      let u;
      for (u = 0; u < c && !(e.getElement(o.seq1Range.start + u) !== e.getElement(o.seq1Range.endExclusive + u) || t.getElement(o.seq2Range.start + u) !== t.getElement(o.seq2Range.endExclusive + u)); u++)
        ;
      if (u === c) {
        r[i + 1] = new Y(new T(o.seq1Range.start + c, l.seq1Range.endExclusive), new T(o.seq2Range.start + c, l.seq2Range.endExclusive));
        continue;
      }
      u > 0 && (o = o.delta(u));
    }
    s.push(o);
  }
  return r.length > 0 && s.push(r[r.length - 1]), s;
}
function vi(e, t, n) {
  if (!e.getBoundaryScore || !t.getBoundaryScore)
    return n;
  for (let r = 0; r < n.length; r++) {
    const s = r > 0 ? n[r - 1] : void 0, i = n[r], l = r + 1 < n.length ? n[r + 1] : void 0, o = new T(s ? s.seq1Range.start + 1 : 0, l ? l.seq1Range.endExclusive - 1 : e.length), c = new T(s ? s.seq2Range.start + 1 : 0, l ? l.seq2Range.endExclusive - 1 : t.length);
    i.seq1Range.isEmpty ? n[r] = Qn(i, e, t, o, c) : i.seq2Range.isEmpty && (n[r] = Qn(i.reverse(), t, e, c, o).reverse());
  }
  return n;
}
function Qn(e, t, n, r, s) {
  let l = 1;
  for (; e.seq1Range.start - l >= r.start && e.seq2Range.start - l >= s.start && n.getElement(e.seq2Range.start - l) === n.getElement(e.seq2Range.endExclusive - l) && l < 100; )
    l++;
  l--;
  let o = 0;
  for (; e.seq1Range.start + o < r.endExclusive && e.seq2Range.endExclusive + o < s.endExclusive && n.getElement(e.seq2Range.start + o) === n.getElement(e.seq2Range.endExclusive + o) && o < 100; )
    o++;
  if (l === 0 && o === 0)
    return e;
  let c = 0, u = -1;
  for (let h = -l; h <= o; h++) {
    const f = e.seq2Range.start + h, d = e.seq2Range.endExclusive + h, m = e.seq1Range.start + h, g = t.getBoundaryScore(m) + n.getBoundaryScore(f) + n.getBoundaryScore(d);
    g > u && (u = g, c = h);
  }
  return e.delta(c);
}
class wi {
  compute(t, n, r = We.instance) {
    if (t.length === 0 || n.length === 0)
      return le.trivial(t, n);
    function s(m, g) {
      for (; m < t.length && g < n.length && t.getElement(m) === n.getElement(g); )
        m++, g++;
      return m;
    }
    let i = 0;
    const l = new Li();
    l.set(0, s(0, 0));
    const o = new Ni();
    o.set(0, l.get(0) === 0 ? null : new Xn(null, 0, 0, l.get(0)));
    let c = 0;
    e:
      for (; ; ) {
        if (i++, !r.isValid())
          return le.trivialTimedOut(t, n);
        const m = -Math.min(i, n.length + i % 2), g = Math.min(i, t.length + i % 2);
        for (c = m; c <= g; c += 2) {
          const _ = c === g ? -1 : l.get(c + 1), S = c === m ? -1 : l.get(c - 1) + 1, b = Math.min(Math.max(_, S), t.length), N = b - c;
          if (b > t.length || N > n.length)
            continue;
          const w = s(b, N);
          l.set(c, w);
          const k = b === _ ? o.get(c + 1) : o.get(c - 1);
          if (o.set(c, w !== b ? new Xn(k, b, N, w - b) : k), l.get(c) === t.length && l.get(c) - c === n.length)
            break e;
        }
      }
    let u = o.get(c);
    const h = [];
    let f = t.length, d = n.length;
    for (; ; ) {
      const m = u ? u.x + u.length : 0, g = u ? u.y + u.length : 0;
      if ((m !== f || g !== d) && h.push(new Y(new T(m, f), new T(g, d))), !u)
        break;
      f = u.x, d = u.y, u = u.prev;
    }
    return h.reverse(), new le(h, !1);
  }
}
class Xn {
  constructor(t, n, r, s) {
    this.prev = t, this.x = n, this.y = r, this.length = s;
  }
}
class Li {
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
class Ni {
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
class Si {
  constructor() {
    this.dynamicProgrammingDiffing = new bi(), this.myersDiffingAlgorithm = new wi();
  }
  computeDiff(t, n, r) {
    if (t.length === 1 && t[0].length === 0 || n.length === 1 && n[0].length === 0)
      return {
        changes: [
          new ae(new H(1, t.length + 1), new H(1, n.length + 1), [
            new He(new P(1, 1, t.length, t[0].length + 1), new P(1, 1, n.length, n[0].length + 1))
          ])
        ],
        hitTimeout: !1,
        moves: []
      };
    const s = r.maxComputationTimeMs === 0 ? We.instance : new gi(r.maxComputationTimeMs), i = !r.ignoreTrimWhitespace, l = /* @__PURE__ */ new Map();
    function o(A) {
      let E = l.get(A);
      return E === void 0 && (E = l.size, l.set(A, E)), E;
    }
    const c = t.map((A) => o(A.trim())), u = n.map((A) => o(A.trim())), h = new Jn(c, t), f = new Jn(u, n), d = (() => h.length + f.length < 1500 ? this.dynamicProgrammingDiffing.compute(h, f, s, (A, E) => t[A] === n[E] ? n[E].length === 0 ? 0.1 : 1 + Math.log(1 + n[E].length) : 0.99) : this.myersDiffingAlgorithm.compute(h, f))();
    let m = d.diffs, g = d.hitTimeout;
    m = jn(h, f, m);
    const _ = [], S = (A) => {
      if (i)
        for (let E = 0; E < A; E++) {
          const C = b + E, R = N + E;
          if (t[C] !== n[R]) {
            const L = this.refineDiff(t, n, new Y(new T(C, C + 1), new T(R, R + 1)), s, i);
            for (const p of L.mappings)
              _.push(p);
            L.hitTimeout && (g = !0);
          }
        }
    };
    let b = 0, N = 0;
    for (const A of m) {
      tt(() => A.seq1Range.start - b === A.seq2Range.start - N);
      const E = A.seq1Range.start - b;
      S(E), b = A.seq1Range.endExclusive, N = A.seq2Range.endExclusive;
      const C = this.refineDiff(t, n, A, s, i);
      C.hitTimeout && (g = !0);
      for (const R of C.mappings)
        _.push(R);
    }
    S(t.length - b);
    const w = Yn(_, t, n), k = [];
    if (r.computeMoves) {
      const A = w.filter((C) => C.modifiedRange.isEmpty && C.originalRange.length >= 3).map((C) => new rr(C.originalRange, t)), E = new Set(w.filter((C) => C.originalRange.isEmpty && C.modifiedRange.length >= 3).map((C) => new rr(C.modifiedRange, n)));
      for (const C of A) {
        let R = -1, L;
        for (const p of E) {
          const v = C.computeSimilarity(p);
          v > R && (R = v, L = p);
        }
        if (R > 0.9 && L) {
          const p = this.refineDiff(t, n, new Y(new T(C.range.startLineNumber - 1, C.range.endLineNumberExclusive - 1), new T(L.range.startLineNumber - 1, L.range.endLineNumberExclusive - 1)), s, i), v = Yn(p.mappings, t, n, !0);
          E.delete(L), k.push(new Tt(new Vt(C.range, L.range), v));
        }
      }
    }
    return tt(() => {
      function A(C, R) {
        if (C.lineNumber < 1 || C.lineNumber > R.length)
          return !1;
        const L = R[C.lineNumber - 1];
        return !(C.column < 1 || C.column > L.length + 1);
      }
      function E(C, R) {
        return !(C.startLineNumber < 1 || C.startLineNumber > R.length + 1 || C.endLineNumberExclusive < 1 || C.endLineNumberExclusive > R.length + 1);
      }
      for (const C of w) {
        if (!C.innerChanges)
          return !1;
        for (const R of C.innerChanges)
          if (!(A(R.modifiedRange.getStartPosition(), n) && A(R.modifiedRange.getEndPosition(), n) && A(R.originalRange.getStartPosition(), t) && A(R.originalRange.getEndPosition(), t)))
            return !1;
        if (!E(C.modifiedRange, n) || !E(C.originalRange, t))
          return !1;
      }
      return !0;
    }), new Sr(w, k, g);
  }
  refineDiff(t, n, r, s, i) {
    const l = new Kn(t, r.seq1Range, i), o = new Kn(n, r.seq2Range, i), c = l.length + o.length < 500 ? this.dynamicProgrammingDiffing.compute(l, o, s) : this.myersDiffingAlgorithm.compute(l, o, s);
    let u = c.diffs;
    return u = jn(l, o, u), u = Ai(l, o, u), u = _i(l, o, u), u = xi(l, o, u), {
      mappings: u.map((f) => new He(l.translateRange(f.seq1Range), o.translateRange(f.seq2Range))),
      hitTimeout: c.hitTimeout
    };
  }
}
function Ai(e, t, n) {
  const r = [];
  let s;
  function i() {
    if (!s)
      return;
    const o = s.s1Range.length - s.deleted;
    s.s2Range.length - s.added, Math.max(s.deleted, s.added) + (s.count - 1) > o && r.push(new Y(s.s1Range, s.s2Range)), s = void 0;
  }
  for (const o of n) {
    let c = function(m, g) {
      var _, S, b, N;
      if (!s || !s.s1Range.containsRange(m) || !s.s2Range.containsRange(g))
        if (s && !(s.s1Range.endExclusive < m.start && s.s2Range.endExclusive < g.start)) {
          const A = T.tryCreate(s.s1Range.endExclusive, m.start), E = T.tryCreate(s.s2Range.endExclusive, g.start);
          s.deleted += (_ = A == null ? void 0 : A.length) !== null && _ !== void 0 ? _ : 0, s.added += (S = E == null ? void 0 : E.length) !== null && S !== void 0 ? S : 0, s.s1Range = s.s1Range.join(m), s.s2Range = s.s2Range.join(g);
        } else
          i(), s = { added: 0, deleted: 0, count: 0, s1Range: m, s2Range: g };
      const w = m.intersect(o.seq1Range), k = g.intersect(o.seq2Range);
      s.count++, s.deleted += (b = w == null ? void 0 : w.length) !== null && b !== void 0 ? b : 0, s.added += (N = k == null ? void 0 : k.length) !== null && N !== void 0 ? N : 0;
    };
    const u = e.findWordContaining(o.seq1Range.start - 1), h = t.findWordContaining(o.seq2Range.start - 1), f = e.findWordContaining(o.seq1Range.endExclusive), d = t.findWordContaining(o.seq2Range.endExclusive);
    u && f && h && d && u.equals(f) && h.equals(d) ? c(u, h) : (u && h && c(u, h), f && d && c(f, d));
  }
  return i(), Ci(n, r);
}
function Ci(e, t) {
  const n = [];
  for (; e.length > 0 || t.length > 0; ) {
    const r = e[0], s = t[0];
    let i;
    r && (!s || r.seq1Range.start < s.seq1Range.start) ? i = e.shift() : i = t.shift(), n.length > 0 && n[n.length - 1].seq1Range.endExclusive >= i.seq1Range.start ? n[n.length - 1] = n[n.length - 1].join(i) : n.push(i);
  }
  return n;
}
function Yn(e, t, n, r = !1) {
  const s = [];
  for (const i of yi(e.map((l) => Ri(l, t, n)), (l, o) => l.originalRange.overlapOrTouch(o.originalRange) || l.modifiedRange.overlapOrTouch(o.modifiedRange))) {
    const l = i[0], o = i[i.length - 1];
    s.push(new ae(l.originalRange.join(o.originalRange), l.modifiedRange.join(o.modifiedRange), i.map((c) => c.innerChanges[0])));
  }
  return tt(() => !r && s.length > 0 && s[0].originalRange.startLineNumber !== s[0].modifiedRange.startLineNumber ? !1 : Nr(s, (i, l) => l.originalRange.startLineNumber - i.originalRange.endLineNumberExclusive === l.modifiedRange.startLineNumber - i.modifiedRange.endLineNumberExclusive && // There has to be an unchanged line in between (otherwise both diffs should have been joined)
  i.originalRange.endLineNumberExclusive < l.originalRange.startLineNumber && i.modifiedRange.endLineNumberExclusive < l.modifiedRange.startLineNumber)), s;
}
function Ri(e, t, n) {
  let r = 0, s = 0;
  e.modifiedRange.endColumn === 1 && e.originalRange.endColumn === 1 && e.originalRange.startLineNumber + r <= e.originalRange.endLineNumber && e.modifiedRange.startLineNumber + r <= e.modifiedRange.endLineNumber && (s = -1), e.modifiedRange.startColumn - 1 >= n[e.modifiedRange.startLineNumber - 1].length && e.originalRange.startColumn - 1 >= t[e.originalRange.startLineNumber - 1].length && e.originalRange.startLineNumber <= e.originalRange.endLineNumber + s && e.modifiedRange.startLineNumber <= e.modifiedRange.endLineNumber + s && (r = 1);
  const i = new H(e.originalRange.startLineNumber + r, e.originalRange.endLineNumber + 1 + s), l = new H(e.modifiedRange.startLineNumber + r, e.modifiedRange.endLineNumber + 1 + s);
  return new ae(i, l, [e]);
}
function* yi(e, t) {
  let n, r;
  for (const s of e)
    r !== void 0 && t(r, s) ? n.push(s) : (n && (yield n), n = [s]), r = s;
  n && (yield n);
}
class Jn {
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
    const n = t === 0 ? 0 : Zn(this.lines[t - 1]), r = t === this.lines.length ? 0 : Zn(this.lines[t]);
    return 1e3 - (n + r);
  }
}
function Zn(e) {
  let t = 0;
  for (; t < e.length && (e.charCodeAt(t) === 32 || e.charCodeAt(t) === 9); )
    t++;
  return t;
}
class Kn {
  constructor(t, n, r) {
    this.lines = t, this.considerWhitespaceChanges = r, this.elements = [], this.firstCharOffsetByLineMinusOne = [], this.offsetByLine = [];
    let s = !1;
    n.start > 0 && n.endExclusive >= t.length && (n = new T(n.start - 1, n.endExclusive), s = !0), this.lineRange = n;
    for (let i = this.lineRange.start; i < this.lineRange.endExclusive; i++) {
      let l = t[i], o = 0;
      if (s)
        o = l.length, l = "", s = !1;
      else if (!r) {
        const c = l.trimStart();
        o = l.length - c.length, l = c.trimEnd();
      }
      this.offsetByLine.push(o);
      for (let c = 0; c < l.length; c++)
        this.elements.push(l.charCodeAt(c));
      i < t.length - 1 && (this.elements.push(`
`.charCodeAt(0)), this.firstCharOffsetByLineMinusOne[i - this.lineRange.start] = this.elements.length);
    }
    this.offsetByLine.push(0);
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
    const n = tr(t > 0 ? this.elements[t - 1] : -1), r = tr(t < this.elements.length ? this.elements[t] : -1);
    if (n === 6 && r === 7)
      return 0;
    let s = 0;
    return n !== r && (s += 10, r === 1 && (s += 1)), s += er(n), s += er(r), s;
  }
  translateOffset(t) {
    if (this.lineRange.isEmpty)
      return new O(this.lineRange.start + 1, 1);
    let n = 0, r = this.firstCharOffsetByLineMinusOne.length;
    for (; n < r; ) {
      const i = Math.floor((n + r) / 2);
      this.firstCharOffsetByLineMinusOne[i] > t ? r = i : n = i + 1;
    }
    const s = n === 0 ? 0 : this.firstCharOffsetByLineMinusOne[n - 1];
    return new O(this.lineRange.start + n + 1, t - s + 1 + this.offsetByLine[n]);
  }
  translateRange(t) {
    return P.fromPositions(this.translateOffset(t.start), this.translateOffset(t.endExclusive));
  }
  /**
   * Finds the word that contains the character at the given offset
   */
  findWordContaining(t) {
    if (t < 0 || t >= this.elements.length || !ut(this.elements[t]))
      return;
    let n = t;
    for (; n > 0 && ut(this.elements[n - 1]); )
      n--;
    let r = t;
    for (; r < this.elements.length && ut(this.elements[r]); )
      r++;
    return new T(n, r);
  }
  countLinesIn(t) {
    return this.translateOffset(t.endExclusive).lineNumber - this.translateOffset(t.start).lineNumber;
  }
}
function ut(e) {
  return e >= 97 && e <= 122 || e >= 65 && e <= 90 || e >= 48 && e <= 57;
}
const Mi = {
  0: 0,
  1: 0,
  2: 0,
  3: 10,
  4: 2,
  5: 3,
  6: 10,
  7: 10
};
function er(e) {
  return Mi[e];
}
function tr(e) {
  return e === 10 ? 7 : e === 13 ? 6 : ki(e) ? 5 : e >= 97 && e <= 122 ? 0 : e >= 65 && e <= 90 ? 1 : e >= 48 && e <= 57 ? 2 : e === -1 ? 3 : 4;
}
function ki(e) {
  return e === 32 || e === 9;
}
const ct = /* @__PURE__ */ new Map();
function nr(e) {
  let t = ct.get(e);
  return t === void 0 && (t = ct.size, ct.set(e, t)), t;
}
class rr {
  constructor(t, n) {
    this.range = t, this.lines = n, this.histogram = [];
    let r = 0;
    for (let s = t.startLineNumber - 1; s < t.endLineNumberExclusive - 1; s++) {
      const i = n[s];
      for (let o = 0; o < i.length; o++) {
        r++;
        const c = i[o], u = nr(c);
        this.histogram[u] = (this.histogram[u] || 0) + 1;
      }
      r++;
      const l = nr(`
`);
      this.histogram[l] = (this.histogram[l] || 0) + 1;
    }
    this.totalCount = r;
  }
  computeSimilarity(t) {
    var n, r;
    let s = 0;
    const i = Math.max(this.histogram.length, t.histogram.length);
    for (let l = 0; l < i; l++)
      s += Math.abs(((n = this.histogram[l]) !== null && n !== void 0 ? n : 0) - ((r = t.histogram[l]) !== null && r !== void 0 ? r : 0));
    return 1 - s / (this.totalCount + t.totalCount);
  }
}
const sr = {
  getLegacy: () => new hi(),
  getAdvanced: () => new Si()
};
function ge(e, t) {
  const n = Math.pow(10, t);
  return Math.round(e * n) / n;
}
class $ {
  constructor(t, n, r, s = 1) {
    this._rgbaBrand = void 0, this.r = Math.min(255, Math.max(0, t)) | 0, this.g = Math.min(255, Math.max(0, n)) | 0, this.b = Math.min(255, Math.max(0, r)) | 0, this.a = ge(Math.max(Math.min(1, s), 0), 3);
  }
  static equals(t, n) {
    return t.r === n.r && t.g === n.g && t.b === n.b && t.a === n.a;
  }
}
class K {
  constructor(t, n, r, s) {
    this._hslaBrand = void 0, this.h = Math.max(Math.min(360, t), 0) | 0, this.s = ge(Math.max(Math.min(1, n), 0), 3), this.l = ge(Math.max(Math.min(1, r), 0), 3), this.a = ge(Math.max(Math.min(1, s), 0), 3);
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
    const n = t.r / 255, r = t.g / 255, s = t.b / 255, i = t.a, l = Math.max(n, r, s), o = Math.min(n, r, s);
    let c = 0, u = 0;
    const h = (o + l) / 2, f = l - o;
    if (f > 0) {
      switch (u = Math.min(h <= 0.5 ? f / (2 * h) : f / (2 - 2 * h), 1), l) {
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
    return new K(c, u, h, i);
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
    let l, o, c;
    if (r === 0)
      l = o = c = s;
    else {
      const u = s < 0.5 ? s * (1 + r) : s + r - s * r, h = 2 * s - u;
      l = K._hue2rgb(h, u, n + 1 / 3), o = K._hue2rgb(h, u, n), c = K._hue2rgb(h, u, n - 1 / 3);
    }
    return new $(Math.round(l * 255), Math.round(o * 255), Math.round(c * 255), i);
  }
}
class ye {
  constructor(t, n, r, s) {
    this._hsvaBrand = void 0, this.h = Math.max(Math.min(360, t), 0) | 0, this.s = ge(Math.max(Math.min(1, n), 0), 3), this.v = ge(Math.max(Math.min(1, r), 0), 3), this.a = ge(Math.max(Math.min(1, s), 0), 3);
  }
  static equals(t, n) {
    return t.h === n.h && t.s === n.s && t.v === n.v && t.a === n.a;
  }
  // from http://www.rapidtables.com/convert/color/rgb-to-hsv.htm
  static fromRGBA(t) {
    const n = t.r / 255, r = t.g / 255, s = t.b / 255, i = Math.max(n, r, s), l = Math.min(n, r, s), o = i - l, c = i === 0 ? 0 : o / i;
    let u;
    return o === 0 ? u = 0 : i === n ? u = ((r - s) / o % 6 + 6) % 6 : i === r ? u = (s - n) / o + 2 : u = (n - r) / o + 4, new ye(Math.round(u * 60), c, i, t.a);
  }
  // from http://www.rapidtables.com/convert/color/hsv-to-rgb.htm
  static toRGBA(t) {
    const { h: n, s: r, v: s, a: i } = t, l = s * r, o = l * (1 - Math.abs(n / 60 % 2 - 1)), c = s - l;
    let [u, h, f] = [0, 0, 0];
    return n < 60 ? (u = l, h = o) : n < 120 ? (u = o, h = l) : n < 180 ? (h = l, f = o) : n < 240 ? (h = o, f = l) : n < 300 ? (u = o, f = l) : n <= 360 && (u = l, f = o), u = Math.round((u + c) * 255), h = Math.round((h + c) * 255), f = Math.round((f + c) * 255), new $(u, h, f, i);
  }
}
class V {
  static fromHex(t) {
    return V.Format.CSS.parseHex(t) || V.red;
  }
  static equals(t, n) {
    return !t && !n ? !0 : !t || !n ? !1 : t.equals(n);
  }
  get hsla() {
    return this._hsla ? this._hsla : K.fromRGBA(this.rgba);
  }
  get hsva() {
    return this._hsva ? this._hsva : ye.fromRGBA(this.rgba);
  }
  constructor(t) {
    if (t)
      if (t instanceof $)
        this.rgba = t;
      else if (t instanceof K)
        this._hsla = t, this.rgba = K.toRGBA(t);
      else if (t instanceof ye)
        this._hsva = t, this.rgba = ye.toRGBA(t);
      else
        throw new Error("Invalid color ctor argument");
    else
      throw new Error("Color needs a value");
  }
  equals(t) {
    return !!t && $.equals(this.rgba, t.rgba) && K.equals(this.hsla, t.hsla) && ye.equals(this.hsva, t.hsva);
  }
  /**
   * http://www.w3.org/TR/WCAG20/#relativeluminancedef
   * Returns the number in the set [0, 1]. O => Darkest Black. 1 => Lightest white.
   */
  getRelativeLuminance() {
    const t = V._relativeLuminanceForComponent(this.rgba.r), n = V._relativeLuminanceForComponent(this.rgba.g), r = V._relativeLuminanceForComponent(this.rgba.b), s = 0.2126 * t + 0.7152 * n + 0.0722 * r;
    return ge(s, 4);
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
    return new V(new K(this.hsla.h, this.hsla.s, this.hsla.l + this.hsla.l * t, this.hsla.a));
  }
  darken(t) {
    return new V(new K(this.hsla.h, this.hsla.s, this.hsla.l - this.hsla.l * t, this.hsla.a));
  }
  transparent(t) {
    const { r: n, g: r, b: s, a: i } = this.rgba;
    return new V(new $(n, r, s, i * t));
  }
  isTransparent() {
    return this.rgba.a === 0;
  }
  isOpaque() {
    return this.rgba.a === 1;
  }
  opposite() {
    return new V(new $(255 - this.rgba.r, 255 - this.rgba.g, 255 - this.rgba.b, this.rgba.a));
  }
  makeOpaque(t) {
    if (this.isOpaque() || t.rgba.a !== 1)
      return this;
    const { r: n, g: r, b: s, a: i } = this.rgba;
    return new V(new $(t.rgba.r - i * (t.rgba.r - n), t.rgba.g - i * (t.rgba.g - r), t.rgba.b - i * (t.rgba.b - s), 1));
  }
  toString() {
    return this._toString || (this._toString = V.Format.CSS.format(this)), this._toString;
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
V.white = new V(new $(255, 255, 255, 1));
V.black = new V(new $(0, 0, 0, 1));
V.red = new V(new $(255, 0, 0, 1));
V.blue = new V(new $(0, 0, 255, 1));
V.green = new V(new $(0, 255, 0, 1));
V.cyan = new V(new $(0, 255, 255, 1));
V.lightgrey = new V(new $(211, 211, 211, 1));
V.transparent = new V(new $(0, 0, 0, 0));
(function(e) {
  (function(t) {
    (function(n) {
      function r(m) {
        return m.rgba.a === 1 ? `rgb(${m.rgba.r}, ${m.rgba.g}, ${m.rgba.b})` : e.Format.CSS.formatRGBA(m);
      }
      n.formatRGB = r;
      function s(m) {
        return `rgba(${m.rgba.r}, ${m.rgba.g}, ${m.rgba.b}, ${+m.rgba.a.toFixed(2)})`;
      }
      n.formatRGBA = s;
      function i(m) {
        return m.hsla.a === 1 ? `hsl(${m.hsla.h}, ${(m.hsla.s * 100).toFixed(2)}%, ${(m.hsla.l * 100).toFixed(2)}%)` : e.Format.CSS.formatHSLA(m);
      }
      n.formatHSL = i;
      function l(m) {
        return `hsla(${m.hsla.h}, ${(m.hsla.s * 100).toFixed(2)}%, ${(m.hsla.l * 100).toFixed(2)}%, ${m.hsla.a.toFixed(2)})`;
      }
      n.formatHSLA = l;
      function o(m) {
        const g = m.toString(16);
        return g.length !== 2 ? "0" + g : g;
      }
      function c(m) {
        return `#${o(m.rgba.r)}${o(m.rgba.g)}${o(m.rgba.b)}`;
      }
      n.formatHex = c;
      function u(m, g = !1) {
        return g && m.rgba.a === 1 ? e.Format.CSS.formatHex(m) : `#${o(m.rgba.r)}${o(m.rgba.g)}${o(m.rgba.b)}${o(Math.round(m.rgba.a * 255))}`;
      }
      n.formatHexA = u;
      function h(m) {
        return m.isOpaque() ? e.Format.CSS.formatHex(m) : e.Format.CSS.formatRGBA(m);
      }
      n.format = h;
      function f(m) {
        const g = m.length;
        if (g === 0 || m.charCodeAt(0) !== 35)
          return null;
        if (g === 7) {
          const _ = 16 * d(m.charCodeAt(1)) + d(m.charCodeAt(2)), S = 16 * d(m.charCodeAt(3)) + d(m.charCodeAt(4)), b = 16 * d(m.charCodeAt(5)) + d(m.charCodeAt(6));
          return new e(new $(_, S, b, 1));
        }
        if (g === 9) {
          const _ = 16 * d(m.charCodeAt(1)) + d(m.charCodeAt(2)), S = 16 * d(m.charCodeAt(3)) + d(m.charCodeAt(4)), b = 16 * d(m.charCodeAt(5)) + d(m.charCodeAt(6)), N = 16 * d(m.charCodeAt(7)) + d(m.charCodeAt(8));
          return new e(new $(_, S, b, N / 255));
        }
        if (g === 4) {
          const _ = d(m.charCodeAt(1)), S = d(m.charCodeAt(2)), b = d(m.charCodeAt(3));
          return new e(new $(16 * _ + _, 16 * S + S, 16 * b + b));
        }
        if (g === 5) {
          const _ = d(m.charCodeAt(1)), S = d(m.charCodeAt(2)), b = d(m.charCodeAt(3)), N = d(m.charCodeAt(4));
          return new e(new $(16 * _ + _, 16 * S + S, 16 * b + b, (16 * N + N) / 255));
        }
        return null;
      }
      n.parseHex = f;
      function d(m) {
        switch (m) {
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
})(V || (V = {}));
function Cr(e) {
  const t = [];
  for (const n of e) {
    const r = Number(n);
    (r || r === 0 && n.replace(/\s/g, "") !== "") && t.push(r);
  }
  return t;
}
function Bt(e, t, n, r) {
  return {
    red: e / 255,
    blue: n / 255,
    green: t / 255,
    alpha: r
  };
}
function Ve(e, t) {
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
function Ei(e, t) {
  if (!e)
    return;
  const n = V.Format.CSS.parseHex(t);
  if (n)
    return {
      range: e,
      color: Bt(n.rgba.r, n.rgba.g, n.rgba.b, n.rgba.a)
    };
}
function ir(e, t, n) {
  if (!e || t.length !== 1)
    return;
  const s = t[0].values(), i = Cr(s);
  return {
    range: e,
    color: Bt(i[0], i[1], i[2], n ? i[3] : 1)
  };
}
function ar(e, t, n) {
  if (!e || t.length !== 1)
    return;
  const s = t[0].values(), i = Cr(s), l = new V(new K(i[0], i[1] / 100, i[2] / 100, n ? i[3] : 1));
  return {
    range: e,
    color: Bt(l.rgba.r, l.rgba.g, l.rgba.b, l.rgba.a)
  };
}
function Te(e, t) {
  return typeof e == "string" ? [...e.matchAll(t)] : e.findMatches(t);
}
function Fi(e) {
  const t = [], r = Te(e, /\b(rgb|rgba|hsl|hsla)(\([0-9\s,.\%]*\))|(#)([A-Fa-f0-9]{3})\b|(#)([A-Fa-f0-9]{4})\b|(#)([A-Fa-f0-9]{6})\b|(#)([A-Fa-f0-9]{8})\b/gm);
  if (r.length > 0)
    for (const s of r) {
      const i = s.filter((u) => u !== void 0), l = i[1], o = i[2];
      if (!o)
        continue;
      let c;
      if (l === "rgb") {
        const u = /^\(\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*\)$/gm;
        c = ir(Ve(e, s), Te(o, u), !1);
      } else if (l === "rgba") {
        const u = /^\(\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(0[.][0-9]+|[.][0-9]+|[01][.]|[01])\s*\)$/gm;
        c = ir(Ve(e, s), Te(o, u), !0);
      } else if (l === "hsl") {
        const u = /^\(\s*(36[0]|3[0-5][0-9]|[12][0-9][0-9]|[1-9]?[0-9])\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*\)$/gm;
        c = ar(Ve(e, s), Te(o, u), !1);
      } else if (l === "hsla") {
        const u = /^\(\s*(36[0]|3[0-5][0-9]|[12][0-9][0-9]|[1-9]?[0-9])\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*,\s*(0[.][0-9]+|[.][0-9]+|[01][.]|[01])\s*\)$/gm;
        c = ar(Ve(e, s), Te(o, u), !0);
      } else
        l === "#" && (c = Ei(Ve(e, s), l + o));
      c && t.push(c);
    }
  return t;
}
function Pi(e) {
  return !e || typeof e.getValue != "function" || typeof e.positionAt != "function" ? [] : Fi(e);
}
var ce = globalThis && globalThis.__awaiter || function(e, t, n, r) {
  function s(i) {
    return i instanceof n ? i : new n(function(l) {
      l(i);
    });
  }
  return new (n || (n = Promise))(function(i, l) {
    function o(h) {
      try {
        u(r.next(h));
      } catch (f) {
        l(f);
      }
    }
    function c(h) {
      try {
        u(r.throw(h));
      } catch (f) {
        l(f);
      }
    }
    function u(h) {
      h.done ? i(h.value) : s(h.value).then(o, c);
    }
    u((r = r.apply(e, t || [])).next());
  });
};
class Di extends Is {
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
      const s = this._lines[r], i = this.offsetAt(new O(r + 1, 1)), l = s.matchAll(t);
      for (const o of l)
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
    const r = Ft(t.column, Hs(n), this._lines[t.lineNumber - 1], 0);
    return r ? new P(t.lineNumber, r.startColumn, t.lineNumber, r.endColumn) : null;
  }
  words(t) {
    const n = this._lines, r = this._wordenize.bind(this);
    let s = 0, i = "", l = 0, o = [];
    return {
      *[Symbol.iterator]() {
        for (; ; )
          if (l < o.length) {
            const c = i.substring(o[l].start, o[l].end);
            l += 1, yield c;
          } else if (s < n.length)
            i = n[s], o = r(i, t), l = 0, s += 1;
          else
            break;
      }
    };
  }
  getLineWords(t, n) {
    const r = this._lines[t - 1], s = this._wordenize(r, n), i = [];
    for (const l of s)
      i.push({
        word: r.substring(l.start, l.end),
        startColumn: l.start + 1,
        endColumn: l.end + 1
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
    for (let l = r + 1; l < s; l++)
      i.push(this._lines[l]);
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
    if (!O.isIPosition(t))
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
class pe {
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
    this._models[t.url] = new Di(xe.parse(t.url), t.lines, t.EOL, t.versionId);
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
    return ce(this, void 0, void 0, function* () {
      const s = this._getModel(t);
      return s ? oi.computeUnicodeHighlights(s, n, r) : { ranges: [], hasMore: !1, ambiguousCharacterCount: 0, invisibleCharacterCount: 0, nonBasicAsciiCharacterCount: 0 };
    });
  }
  // ---- BEGIN diff --------------------------------------------------------------------------
  computeDiff(t, n, r, s) {
    return ce(this, void 0, void 0, function* () {
      const i = this._getModel(t), l = this._getModel(n);
      return !i || !l ? null : pe.computeDiff(i, l, r, s);
    });
  }
  static computeDiff(t, n, r, s) {
    const i = s === "advanced" ? sr.getAdvanced() : sr.getLegacy(), l = t.getLinesContent(), o = n.getLinesContent(), c = i.computeDiff(l, o, r), u = c.changes.length > 0 ? !1 : this._modelsAreIdentical(t, n);
    function h(f) {
      return f.map((d) => {
        var m;
        return [d.originalRange.startLineNumber, d.originalRange.endLineNumberExclusive, d.modifiedRange.startLineNumber, d.modifiedRange.endLineNumberExclusive, (m = d.innerChanges) === null || m === void 0 ? void 0 : m.map((g) => [
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
      const l = t.getLineContent(i), o = n.getLineContent(i);
      if (l !== o)
        return !1;
    }
    return !0;
  }
  computeMoreMinimalEdits(t, n, r) {
    return ce(this, void 0, void 0, function* () {
      const s = this._getModel(t);
      if (!s)
        return n;
      const i = [];
      let l;
      n = n.slice(0).sort((o, c) => {
        if (o.range && c.range)
          return P.compareRangesUsingStarts(o.range, c.range);
        const u = o.range ? 0 : 1, h = c.range ? 0 : 1;
        return u - h;
      });
      for (let { range: o, text: c, eol: u } of n) {
        if (typeof u == "number" && (l = u), P.isEmpty(o) && !c)
          continue;
        const h = s.getValueInRange(o);
        if (c = c.replace(/\r\n|\n|\r/g, s.eol), h === c)
          continue;
        if (Math.max(c.length, h.length) > pe._diffLimit) {
          i.push({ range: o, text: c });
          continue;
        }
        const f = xs(h, c, r), d = s.offsetAt(P.lift(o).getStartPosition());
        for (const m of f) {
          const g = s.positionAt(d + m.originalStart), _ = s.positionAt(d + m.originalStart + m.originalLength), S = {
            text: c.substr(m.modifiedStart, m.modifiedLength),
            range: { startLineNumber: g.lineNumber, startColumn: g.column, endLineNumber: _.lineNumber, endColumn: _.column }
          };
          s.getValueInRange(S.range) !== S.text && i.push(S);
        }
      }
      return typeof l == "number" && i.push({ eol: l, text: "", range: { startLineNumber: 0, startColumn: 0, endLineNumber: 0, endColumn: 0 } }), i;
    });
  }
  // ---- END minimal edits ---------------------------------------------------------------
  computeLinks(t) {
    return ce(this, void 0, void 0, function* () {
      const n = this._getModel(t);
      return n ? js(n) : null;
    });
  }
  // --- BEGIN default document colors -----------------------------------------------------------
  computeDefaultDocumentColors(t) {
    return ce(this, void 0, void 0, function* () {
      const n = this._getModel(t);
      return n ? Pi(n) : null;
    });
  }
  textualSuggest(t, n, r, s) {
    return ce(this, void 0, void 0, function* () {
      const i = new nt(), l = new RegExp(r, s), o = /* @__PURE__ */ new Set();
      e:
        for (const c of t) {
          const u = this._getModel(c);
          if (u) {
            for (const h of u.words(l))
              if (!(h === n || !isNaN(Number(h))) && (o.add(h), o.size > pe._suggestionsLimit))
                break e;
          }
        }
      return { words: Array.from(o), duration: i.elapsed() };
    });
  }
  // ---- END suggest --------------------------------------------------------------------------
  //#region -- word ranges --
  computeWordRanges(t, n, r, s) {
    return ce(this, void 0, void 0, function* () {
      const i = this._getModel(t);
      if (!i)
        return /* @__PURE__ */ Object.create(null);
      const l = new RegExp(r, s), o = /* @__PURE__ */ Object.create(null);
      for (let c = n.startLineNumber; c < n.endLineNumber; c++) {
        const u = i.getLineWords(c, l);
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
    return ce(this, void 0, void 0, function* () {
      const l = this._getModel(t);
      if (!l)
        return null;
      const o = new RegExp(s, i);
      n.startColumn === n.endColumn && (n = {
        startLineNumber: n.startLineNumber,
        startColumn: n.startColumn,
        endLineNumber: n.endLineNumber,
        endColumn: n.endColumn + 1
      });
      const c = l.getValueInRange(n), u = l.getWordAtPosition({ lineNumber: n.startLineNumber, column: n.startColumn }, o);
      if (!u)
        return null;
      const h = l.getValueInRange(u);
      return vt.INSTANCE.navigateValueSet(n, c, u, h, r);
    });
  }
  // ---- BEGIN foreign module support --------------------------------------------------------------------------
  loadForeignModule(t, n, r) {
    const l = {
      host: zr(r, (o, c) => this._host.fhr(o, c)),
      getMirrorModels: () => this._getModels()
    };
    return this._foreignModuleFactory ? (this._foreignModule = this._foreignModuleFactory(l, n), Promise.resolve(dt(this._foreignModule))) : Promise.reject(new Error("Unexpected usage"));
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
pe._diffLimit = 1e5;
pe._suggestionsLimit = 1e4;
typeof importScripts == "function" && (globalThis.monaco = ni());
let kt = !1;
function Vi(e) {
  if (kt)
    return;
  kt = !0;
  const t = new bs((n) => {
    globalThis.postMessage(n);
  }, (n) => new pe(n, e));
  globalThis.onmessage = (n) => {
    t.onmessage(n.data);
  };
}
globalThis.onmessage = (e) => {
  kt || Vi(null);
};
export {
  Vi as initialize
};
