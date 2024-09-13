class qr {
  constructor() {
    this.listeners = [], this.unexpectedErrorHandler = function(t) {
      setTimeout(() => {
        throw t.stack ? ct.isErrorNoTelemetry(t) ? new ct(t.message + `

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
    this.listeners.forEach((n) => {
      n(t);
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
  onUnexpectedExternalError(t) {
    this.unexpectedErrorHandler(t);
  }
}
const Wr = new qr();
function yt(e) {
  Gr(e) || Wr.onUnexpectedError(e);
}
function Zn(e) {
  if (e instanceof Error) {
    const { name: t, message: n } = e, i = e.stacktrace || e.stack;
    return {
      $isError: !0,
      name: t,
      message: n,
      stack: i,
      noTelemetry: ct.isErrorNoTelemetry(e)
    };
  }
  return e;
}
const dn = "Canceled";
function Gr(e) {
  return e instanceof $r ? !0 : e instanceof Error && e.name === dn && e.message === dn;
}
class $r extends Error {
  constructor() {
    super(dn), this.name = this.message;
  }
}
class ct extends Error {
  constructor(t) {
    super(t), this.name = "CodeExpectedError";
  }
  static fromError(t) {
    if (t instanceof ct)
      return t;
    const n = new ct();
    return n.message = t.message, n.stack = t.stack, n;
  }
  static isErrorNoTelemetry(t) {
    return t.name === "CodeExpectedError";
  }
}
class me extends Error {
  constructor(t) {
    super(t || "An unexpected bug occurred."), Object.setPrototypeOf(this, me.prototype);
  }
}
function zr(e, t) {
  const n = this;
  let i = !1, s;
  return function() {
    return i || (i = !0, s = e.apply(n, arguments)), s;
  };
}
function ft(e, t) {
  const n = vt(e, t);
  return n === -1 ? void 0 : e[n];
}
function vt(e, t, n = 0, i = e.length) {
  let s = n, r = i;
  for (; s < r; ) {
    const a = Math.floor((s + r) / 2);
    t(e[a]) ? s = a + 1 : r = a;
  }
  return s - 1;
}
function jr(e, t) {
  const n = Ln(e, t);
  return n === e.length ? void 0 : e[n];
}
function Ln(e, t, n = 0, i = e.length) {
  let s = n, r = i;
  for (; s < r; ) {
    const a = Math.floor((s + r) / 2);
    t(e[a]) ? r = a : s = a + 1;
  }
  return s;
}
const tn = class tn {
  constructor(t) {
    this._array = t, this._findLastMonotonousLastIdx = 0;
  }
  findLastMonotonous(t) {
    if (tn.assertInvariants) {
      if (this._prevFindLastPredicate) {
        for (const i of this._array)
          if (this._prevFindLastPredicate(i) && !t(i))
            throw new Error(
              "MonotonousArray: current predicate must be weaker than (or equal to) the previous predicate."
            );
      }
      this._prevFindLastPredicate = t;
    }
    const n = vt(this._array, t, this._findLastMonotonousLastIdx);
    return this._findLastMonotonousLastIdx = n + 1, n === -1 ? void 0 : this._array[n];
  }
};
tn.assertInvariants = !1;
let Gt = tn;
function Xr(e, t, n = (i, s) => i === s) {
  if (e === t)
    return !0;
  if (!e || !t || e.length !== t.length)
    return !1;
  for (let i = 0, s = e.length; i < s; i++)
    if (!n(e[i], t[i]))
      return !1;
  return !0;
}
function* Yr(e, t) {
  let n, i;
  for (const s of e)
    i !== void 0 && t(i, s) ? n.push(s) : (n && (yield n), n = [s]), i = s;
  n && (yield n);
}
function Qr(e, t) {
  for (let n = 0; n <= e.length; n++)
    t(n === 0 ? void 0 : e[n - 1], n === e.length ? void 0 : e[n]);
}
function Zr(e, t) {
  for (let n = 0; n < e.length; n++)
    t(n === 0 ? void 0 : e[n - 1], e[n], n + 1 === e.length ? void 0 : e[n + 1]);
}
function Jr(e, t) {
  for (const n of t)
    e.push(n);
}
var Nn;
(function(e) {
  function t(r) {
    return r < 0;
  }
  e.isLessThan = t;
  function n(r) {
    return r <= 0;
  }
  e.isLessThanOrEqual = n;
  function i(r) {
    return r > 0;
  }
  e.isGreaterThan = i;
  function s(r) {
    return r === 0;
  }
  e.isNeitherLessOrGreaterThan = s, e.greaterThan = 1, e.lessThan = -1, e.neitherLessOrGreaterThan = 0;
})(Nn || (Nn = {}));
function Nt(e, t) {
  return (n, i) => t(e(n), e(i));
}
const pt = (e, t) => e - t;
function Kr(e) {
  return (t, n) => -e(t, n);
}
const st = class st {
  constructor(t) {
    this.iterate = t;
  }
  forEach(t) {
    this.iterate((n) => (t(n), !0));
  }
  toArray() {
    const t = [];
    return this.iterate((n) => (t.push(n), !0)), t;
  }
  filter(t) {
    return new st((n) => this.iterate((i) => t(i) ? n(i) : !0));
  }
  map(t) {
    return new st((n) => this.iterate((i) => n(t(i))));
  }
  some(t) {
    let n = !1;
    return this.iterate((i) => (n = t(i), !n)), n;
  }
  findFirst(t) {
    let n;
    return this.iterate((i) => t(i) ? (n = i, !1) : !0), n;
  }
  findLast(t) {
    let n;
    return this.iterate((i) => (t(i) && (n = i), !0)), n;
  }
  findLastMaxBy(t) {
    let n, i = !0;
    return this.iterate((s) => ((i || Nn.isGreaterThan(t(s, n))) && (i = !1, n = s), !0)), n;
  }
};
st.empty = new st((t) => {
});
let Jn = st;
function Cr(e, t) {
  const n = /* @__PURE__ */ Object.create(null);
  for (const i of e) {
    const s = t(i);
    let r = n[s];
    r || (r = n[s] = []), r.push(i);
  }
  return n;
}
var Kn, Cn;
class el {
  constructor(t, n) {
    this.uri = t, this.value = n;
  }
}
function tl(e) {
  return Array.isArray(e);
}
const Ge = class Ge {
  constructor(t, n) {
    if (this[Kn] = "ResourceMap", t instanceof Ge)
      this.map = new Map(t.map), this.toKey = n ?? Ge.defaultToKey;
    else if (tl(t)) {
      this.map = /* @__PURE__ */ new Map(), this.toKey = n ?? Ge.defaultToKey;
      for (const [i, s] of t)
        this.set(i, s);
    } else
      this.map = /* @__PURE__ */ new Map(), this.toKey = t ?? Ge.defaultToKey;
  }
  set(t, n) {
    return this.map.set(this.toKey(t), new el(t, n)), this;
  }
  get(t) {
    var n;
    return (n = this.map.get(this.toKey(t))) == null ? void 0 : n.value;
  }
  has(t) {
    return this.map.has(this.toKey(t));
  }
  get size() {
    return this.map.size;
  }
  clear() {
    this.map.clear();
  }
  delete(t) {
    return this.map.delete(this.toKey(t));
  }
  forEach(t, n) {
    typeof n < "u" && (t = t.bind(n));
    for (const [i, s] of this.map)
      t(s.value, s.uri, this);
  }
  *values() {
    for (const t of this.map.values())
      yield t.value;
  }
  *keys() {
    for (const t of this.map.values())
      yield t.uri;
  }
  *entries() {
    for (const t of this.map.values())
      yield [t.uri, t.value];
  }
  *[(Kn = Symbol.toStringTag, Symbol.iterator)]() {
    for (const [, t] of this.map)
      yield [t.uri, t.value];
  }
};
Ge.defaultToKey = (t) => t.toString();
let ei = Ge;
var oe;
(function(e) {
  e[e.None = 0] = "None", e[e.AsOld = 1] = "AsOld", e[e.AsNew = 2] = "AsNew";
})(oe || (oe = {}));
class nl {
  constructor() {
    this[Cn] = "LinkedMap", this._map = /* @__PURE__ */ new Map(), this._head = void 0, this._tail = void 0, this._size = 0, this._state = 0;
  }
  clear() {
    this._map.clear(), this._head = void 0, this._tail = void 0, this._size = 0, this._state++;
  }
  isEmpty() {
    return !this._head && !this._tail;
  }
  get size() {
    return this._size;
  }
  get first() {
    var t;
    return (t = this._head) == null ? void 0 : t.value;
  }
  get last() {
    var t;
    return (t = this._tail) == null ? void 0 : t.value;
  }
  has(t) {
    return this._map.has(t);
  }
  get(t, n = oe.None) {
    const i = this._map.get(t);
    if (i)
      return n !== oe.None && this.touch(i, n), i.value;
  }
  set(t, n, i = oe.None) {
    let s = this._map.get(t);
    if (s)
      s.value = n, i !== oe.None && this.touch(s, i);
    else {
      switch (s = { key: t, value: n, next: void 0, previous: void 0 }, i) {
        case oe.None:
          this.addItemLast(s);
          break;
        case oe.AsOld:
          this.addItemFirst(s);
          break;
        case oe.AsNew:
          this.addItemLast(s);
          break;
        default:
          this.addItemLast(s);
          break;
      }
      this._map.set(t, s), this._size++;
    }
    return this;
  }
  delete(t) {
    return !!this.remove(t);
  }
  remove(t) {
    const n = this._map.get(t);
    if (n)
      return this._map.delete(t), this.removeItem(n), this._size--, n.value;
  }
  shift() {
    if (!this._head && !this._tail)
      return;
    if (!this._head || !this._tail)
      throw new Error("Invalid list");
    const t = this._head;
    return this._map.delete(t.key), this.removeItem(t), this._size--, t.value;
  }
  forEach(t, n) {
    const i = this._state;
    let s = this._head;
    for (; s; ) {
      if (n ? t.bind(n)(s.value, s.key, this) : t(s.value, s.key, this), this._state !== i)
        throw new Error("LinkedMap got modified during iteration.");
      s = s.next;
    }
  }
  keys() {
    const t = this, n = this._state;
    let i = this._head;
    const s = {
      [Symbol.iterator]() {
        return s;
      },
      next() {
        if (t._state !== n)
          throw new Error("LinkedMap got modified during iteration.");
        if (i) {
          const r = { value: i.key, done: !1 };
          return i = i.next, r;
        } else
          return { value: void 0, done: !0 };
      }
    };
    return s;
  }
  values() {
    const t = this, n = this._state;
    let i = this._head;
    const s = {
      [Symbol.iterator]() {
        return s;
      },
      next() {
        if (t._state !== n)
          throw new Error("LinkedMap got modified during iteration.");
        if (i) {
          const r = { value: i.value, done: !1 };
          return i = i.next, r;
        } else
          return { value: void 0, done: !0 };
      }
    };
    return s;
  }
  entries() {
    const t = this, n = this._state;
    let i = this._head;
    const s = {
      [Symbol.iterator]() {
        return s;
      },
      next() {
        if (t._state !== n)
          throw new Error("LinkedMap got modified during iteration.");
        if (i) {
          const r = { value: [i.key, i.value], done: !1 };
          return i = i.next, r;
        } else
          return { value: void 0, done: !0 };
      }
    };
    return s;
  }
  [(Cn = Symbol.toStringTag, Symbol.iterator)]() {
    return this.entries();
  }
  trimOld(t) {
    if (t >= this.size)
      return;
    if (t === 0) {
      this.clear();
      return;
    }
    let n = this._head, i = this.size;
    for (; n && i > t; )
      this._map.delete(n.key), n = n.next, i--;
    this._head = n, this._size = i, n && (n.previous = void 0), this._state++;
  }
  trimNew(t) {
    if (t >= this.size)
      return;
    if (t === 0) {
      this.clear();
      return;
    }
    let n = this._tail, i = this.size;
    for (; n && i > t; )
      this._map.delete(n.key), n = n.previous, i--;
    this._tail = n, this._size = i, n && (n.next = void 0), this._state++;
  }
  addItemFirst(t) {
    if (!this._head && !this._tail)
      this._tail = t;
    else if (this._head)
      t.next = this._head, this._head.previous = t;
    else
      throw new Error("Invalid list");
    this._head = t, this._state++;
  }
  addItemLast(t) {
    if (!this._head && !this._tail)
      this._head = t;
    else if (this._tail)
      t.previous = this._tail, this._tail.next = t;
    else
      throw new Error("Invalid list");
    this._tail = t, this._state++;
  }
  removeItem(t) {
    if (t === this._head && t === this._tail)
      this._head = void 0, this._tail = void 0;
    else if (t === this._head) {
      if (!t.next)
        throw new Error("Invalid list");
      t.next.previous = void 0, this._head = t.next;
    } else if (t === this._tail) {
      if (!t.previous)
        throw new Error("Invalid list");
      t.previous.next = void 0, this._tail = t.previous;
    } else {
      const n = t.next, i = t.previous;
      if (!n || !i)
        throw new Error("Invalid list");
      n.previous = i, i.next = n;
    }
    t.next = void 0, t.previous = void 0, this._state++;
  }
  touch(t, n) {
    if (!this._head || !this._tail)
      throw new Error("Invalid list");
    if (!(n !== oe.AsOld && n !== oe.AsNew)) {
      if (n === oe.AsOld) {
        if (t === this._head)
          return;
        const i = t.next, s = t.previous;
        t === this._tail ? (s.next = void 0, this._tail = s) : (i.previous = s, s.next = i), t.previous = void 0, t.next = this._head, this._head.previous = t, this._head = t, this._state++;
      } else if (n === oe.AsNew) {
        if (t === this._tail)
          return;
        const i = t.next, s = t.previous;
        t === this._head ? (i.previous = void 0, this._head = i) : (i.previous = s, s.next = i), t.next = void 0, t.previous = this._tail, this._tail.next = t, this._tail = t, this._state++;
      }
    }
  }
  toJSON() {
    const t = [];
    return this.forEach((n, i) => {
      t.push([i, n]);
    }), t;
  }
  fromJSON(t) {
    this.clear();
    for (const [n, i] of t)
      this.set(n, i);
  }
}
class il extends nl {
  constructor(t, n = 1) {
    super(), this._limit = t, this._ratio = Math.min(Math.max(0, n), 1);
  }
  get limit() {
    return this._limit;
  }
  set limit(t) {
    this._limit = t, this.checkTrim();
  }
  get ratio() {
    return this._ratio;
  }
  set ratio(t) {
    this._ratio = Math.min(Math.max(0, t), 1), this.checkTrim();
  }
  get(t, n = oe.AsNew) {
    return super.get(t, n);
  }
  peek(t) {
    return super.get(t, oe.None);
  }
  set(t, n) {
    return super.set(t, n, oe.AsNew), this;
  }
  checkTrim() {
    this.size > this._limit && this.trim(Math.round(this._limit * this._ratio));
  }
}
class sl extends il {
  constructor(t, n = 1) {
    super(t, n);
  }
  trim(t) {
    this.trimOld(t);
  }
  set(t, n) {
    return super.set(t, n), this.checkTrim(), this;
  }
}
class br {
  constructor() {
    this.map = /* @__PURE__ */ new Map();
  }
  add(t, n) {
    let i = this.map.get(t);
    i || (i = /* @__PURE__ */ new Set(), this.map.set(t, i)), i.add(n);
  }
  delete(t, n) {
    const i = this.map.get(t);
    i && (i.delete(n), i.size === 0 && this.map.delete(t));
  }
  forEach(t, n) {
    const i = this.map.get(t);
    i && i.forEach(n);
  }
  get(t) {
    const n = this.map.get(t);
    return n || /* @__PURE__ */ new Set();
  }
}
var $t;
(function(e) {
  function t(p) {
    return p && typeof p == "object" && typeof p[Symbol.iterator] == "function";
  }
  e.is = t;
  const n = Object.freeze([]);
  function i() {
    return n;
  }
  e.empty = i;
  function* s(p) {
    yield p;
  }
  e.single = s;
  function r(p) {
    return t(p) ? p : s(p);
  }
  e.wrap = r;
  function a(p) {
    return p || n;
  }
  e.from = a;
  function* u(p) {
    for (let w = p.length - 1; w >= 0; w--)
      yield p[w];
  }
  e.reverse = u;
  function o(p) {
    return !p || p[Symbol.iterator]().next().done === !0;
  }
  e.isEmpty = o;
  function c(p) {
    return p[Symbol.iterator]().next().value;
  }
  e.first = c;
  function h(p, w) {
    let D = 0;
    for (const T of p)
      if (w(T, D++))
        return !0;
    return !1;
  }
  e.some = h;
  function f(p, w) {
    for (const D of p)
      if (w(D))
        return D;
  }
  e.find = f;
  function* g(p, w) {
    for (const D of p)
      w(D) && (yield D);
  }
  e.filter = g;
  function* b(p, w) {
    let D = 0;
    for (const T of p)
      yield w(T, D++);
  }
  e.map = b;
  function* L(p, w) {
    let D = 0;
    for (const T of p)
      yield* w(T, D++);
  }
  e.flatMap = L;
  function* N(...p) {
    for (const w of p)
      yield* w;
  }
  e.concat = N;
  function A(p, w, D) {
    let T = D;
    for (const I of p)
      T = w(T, I);
    return T;
  }
  e.reduce = A;
  function* x(p, w, D = p.length) {
    for (w < 0 && (w += p.length), D < 0 ? D += p.length : D > p.length && (D = p.length); w < D; w++)
      yield p[w];
  }
  e.slice = x;
  function R(p, w = Number.POSITIVE_INFINITY) {
    const D = [];
    if (w === 0)
      return [D, p];
    const T = p[Symbol.iterator]();
    for (let I = 0; I < w; I++) {
      const $ = T.next();
      if ($.done)
        return [D, e.empty()];
      D.push($.value);
    }
    return [D, { [Symbol.iterator]() {
      return T;
    } }];
  }
  e.consume = R;
  async function U(p) {
    const w = [];
    for await (const D of p)
      w.push(D);
    return Promise.resolve(w);
  }
  e.asyncToArray = U;
})($t || ($t = {}));
const nn = class nn {
  constructor() {
    this.livingDisposables = /* @__PURE__ */ new Map();
  }
  getDisposableData(t) {
    let n = this.livingDisposables.get(t);
    return n || (n = { parent: null, source: null, isSingleton: !1, value: t, idx: nn.idx++ }, this.livingDisposables.set(t, n)), n;
  }
  trackDisposable(t) {
    const n = this.getDisposableData(t);
    n.source || (n.source = new Error().stack);
  }
  setParent(t, n) {
    const i = this.getDisposableData(t);
    i.parent = n;
  }
  markAsDisposed(t) {
    this.livingDisposables.delete(t);
  }
  markAsSingleton(t) {
    this.getDisposableData(t).isSingleton = !0;
  }
  getRootParent(t, n) {
    const i = n.get(t);
    if (i)
      return i;
    const s = t.parent ? this.getRootParent(this.getDisposableData(t.parent), n) : t;
    return n.set(t, s), s;
  }
  getTrackedDisposables() {
    const t = /* @__PURE__ */ new Map();
    return [...this.livingDisposables.entries()].filter(([, i]) => i.source !== null && !this.getRootParent(i, t).isSingleton).flatMap(([i]) => i);
  }
  computeLeakingDisposables(t = 10, n) {
    let i;
    if (n)
      i = n;
    else {
      const o = /* @__PURE__ */ new Map(), c = [...this.livingDisposables.values()].filter((f) => f.source !== null && !this.getRootParent(f, o).isSingleton);
      if (c.length === 0)
        return;
      const h = new Set(c.map((f) => f.value));
      if (i = c.filter((f) => !(f.parent && h.has(f.parent))), i.length === 0)
        throw new Error("There are cyclic diposable chains!");
    }
    if (!i)
      return;
    function s(o) {
      function c(f, g) {
        for (; f.length > 0 && g.some(
          (b) => typeof b == "string" ? b === f[0] : f[0].match(b)
        ); )
          f.shift();
      }
      const h = o.source.split(`
`).map((f) => f.trim().replace("at ", "")).filter((f) => f !== "");
      return c(h, ["Error", /^trackDisposable \(.*\)$/, /^DisposableTracker.trackDisposable \(.*\)$/]), h.reverse();
    }
    const r = new br();
    for (const o of i) {
      const c = s(o);
      for (let h = 0; h <= c.length; h++)
        r.add(c.slice(0, h).join(`
`), o);
    }
    i.sort(Nt((o) => o.idx, pt));
    let a = "", u = 0;
    for (const o of i.slice(0, t)) {
      u++;
      const c = s(o), h = [];
      for (let f = 0; f < c.length; f++) {
        let g = c[f];
        g = `(shared with ${r.get(c.slice(0, f + 1).join(`
`)).size}/${i.length} leaks) at ${g}`;
        const L = r.get(c.slice(0, f).join(`
`)), N = Cr([...L].map((A) => s(A)[f]), (A) => A);
        delete N[c[f]];
        for (const [A, x] of Object.entries(N))
          h.unshift(`    - stacktraces of ${x.length} other leaks continue with ${A}`);
        h.unshift(g);
      }
      a += `


==================== Leaking disposable ${u}/${i.length}: ${o.value.constructor.name} ====================
${h.join(`
`)}
============================================================

`;
    }
    return i.length > t && (a += `


... and ${i.length - t} more leaking disposables

`), { leaks: i, details: a };
  }
};
nn.idx = 0;
let ti = nn;
function dr(e) {
  if ($t.is(e)) {
    const t = [];
    for (const n of e)
      if (n)
        try {
          n.dispose();
        } catch (i) {
          t.push(i);
        }
    if (t.length === 1)
      throw t[0];
    if (t.length > 1)
      throw new AggregateError(t, "Encountered errors while disposing of store");
    return Array.isArray(e) ? [] : e;
  } else if (e)
    return e.dispose(), e;
}
function rl(...e) {
  return zt(() => dr(e));
}
function zt(e) {
  return {
    dispose: zr(() => {
      e();
    })
  };
}
const sn = class sn {
  constructor() {
    this._toDispose = /* @__PURE__ */ new Set(), this._isDisposed = !1;
  }
  dispose() {
    this._isDisposed || (this._isDisposed = !0, this.clear());
  }
  get isDisposed() {
    return this._isDisposed;
  }
  clear() {
    if (this._toDispose.size !== 0)
      try {
        dr(this._toDispose);
      } finally {
        this._toDispose.clear();
      }
  }
  add(t) {
    if (!t)
      return t;
    if (t === this)
      throw new Error("Cannot register a disposable on itself!");
    return this._isDisposed ? sn.DISABLE_DISPOSED_WARNING || console.warn(new Error(
      "Trying to add a disposable to a DisposableStore that has already been disposed of. The added object will be leaked!"
    ).stack) : this._toDispose.add(t), t;
  }
  delete(t) {
    if (t) {
      if (t === this)
        throw new Error("Cannot dispose a disposable on itself!");
      this._toDispose.delete(t), t.dispose();
    }
  }
  deleteAndLeak(t) {
    t && this._toDispose.has(t) && this._toDispose.delete(t);
  }
};
sn.DISABLE_DISPOSED_WARNING = !1;
let Ut = sn;
const Qn = class Qn {
  constructor() {
    this._store = new Ut(), this._store;
  }
  dispose() {
    this._store.dispose();
  }
  _register(t) {
    if (t === this)
      throw new Error("Cannot register a disposable on itself!");
    return this._store.add(t);
  }
};
Qn.None = Object.freeze({ dispose() {
} });
let ht = Qn;
const rt = class rt {
  constructor(t) {
    this.element = t, this.next = rt.Undefined, this.prev = rt.Undefined;
  }
};
rt.Undefined = new rt(void 0);
let J = rt;
class ll {
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
      const n = t.next;
      t.prev = J.Undefined, t.next = J.Undefined, t = n;
    }
    this._first = J.Undefined, this._last = J.Undefined, this._size = 0;
  }
  unshift(t) {
    return this._insert(t, !1);
  }
  push(t) {
    return this._insert(t, !0);
  }
  _insert(t, n) {
    const i = new J(t);
    if (this._first === J.Undefined)
      this._first = i, this._last = i;
    else if (n) {
      const r = this._last;
      this._last = i, i.prev = r, r.next = i;
    } else {
      const r = this._first;
      this._first = i, i.next = r, r.prev = i;
    }
    this._size += 1;
    let s = !1;
    return () => {
      s || (s = !0, this._remove(i));
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
      const n = t.prev;
      n.next = t.next, t.next.prev = n;
    } else t.prev === J.Undefined && t.next === J.Undefined ? (this._first = J.Undefined, this._last = J.Undefined) : t.next === J.Undefined ? (this._last = this._last.prev, this._last.next = J.Undefined) : t.prev === J.Undefined && (this._first = this._first.next, this._first.prev = J.Undefined);
    this._size -= 1;
  }
  *[Symbol.iterator]() {
    let t = this._first;
    for (; t !== J.Undefined; )
      yield t.element, t = t.next;
  }
}
const al = globalThis.performance && typeof globalThis.performance.now == "function";
class on {
  static create(t) {
    return new on(t);
  }
  constructor(t) {
    this._now = al && t === !1 ? Date.now : globalThis.performance.now.bind(globalThis.performance), this._startTime = this._now(), this._stopTime = -1;
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
}
var pn;
(function(e) {
  e.None = () => ht.None;
  function t(E, v) {
    return f(E, () => {
    }, 0, void 0, !0, void 0, v);
  }
  e.defer = t;
  function n(E) {
    return (v, k = null, M) => {
      let y = !1, q;
      return q = E((j) => {
        if (!y)
          return q ? q.dispose() : y = !0, v.call(k, j);
      }, null, M), y && q.dispose(), q;
    };
  }
  e.once = n;
  function i(E, v, k) {
    return c((M, y = null, q) => E((j) => M.call(y, v(j)), null, q), k);
  }
  e.map = i;
  function s(E, v, k) {
    return c((M, y = null, q) => E((j) => {
      v(j), M.call(y, j);
    }, null, q), k);
  }
  e.forEach = s;
  function r(E, v, k) {
    return c((M, y = null, q) => E((j) => v(j) && M.call(y, j), null, q), k);
  }
  e.filter = r;
  function a(E) {
    return E;
  }
  e.signal = a;
  function u(...E) {
    return (v, k = null, M) => {
      const y = rl(...E.map((q) => q((j) => v.call(k, j))));
      return h(y, M);
    };
  }
  e.any = u;
  function o(E, v, k, M) {
    let y = k;
    return i(E, (q) => (y = v(y, q), y), M);
  }
  e.reduce = o;
  function c(E, v) {
    let k;
    const M = {
      onWillAddFirstListener() {
        k = E(y.fire, y);
      },
      onDidRemoveLastListener() {
        k == null || k.dispose();
      }
    }, y = new Ae(M);
    return v == null || v.add(y), y.event;
  }
  function h(E, v) {
    return v instanceof Array ? v.push(E) : v && v.add(E), E;
  }
  function f(E, v, k = 100, M = !1, y = !1, q, j) {
    let se, ue, Qe, It = 0, We;
    const Or = {
      leakWarningThreshold: q,
      onWillAddFirstListener() {
        se = E((Vr) => {
          It++, ue = v(ue, Vr), M && !Qe && (St.fire(ue), ue = void 0), We = () => {
            const Hr = ue;
            ue = void 0, Qe = void 0, (!M || It > 1) && St.fire(Hr), It = 0;
          }, typeof k == "number" ? (clearTimeout(Qe), Qe = setTimeout(We, k)) : Qe === void 0 && (Qe = 0, queueMicrotask(We));
        });
      },
      onWillRemoveListener() {
        y && It > 0 && (We == null || We());
      },
      onDidRemoveLastListener() {
        We = void 0, se.dispose();
      }
    }, St = new Ae(Or);
    return j == null || j.add(St), St.event;
  }
  e.debounce = f;
  function g(E, v = 0, k) {
    return e.debounce(E, (M, y) => M ? (M.push(y), M) : [y], v, void 0, !0, void 0, k);
  }
  e.accumulate = g;
  function b(E, v = (M, y) => M === y, k) {
    let M = !0, y;
    return r(E, (q) => {
      const j = M || !v(q, y);
      return M = !1, y = q, j;
    }, k);
  }
  e.latch = b;
  function L(E, v, k) {
    return [
      e.filter(E, v, k),
      e.filter(E, (M) => !v(M), k)
    ];
  }
  e.split = L;
  function N(E, v = !1, k = [], M) {
    let y = k.slice(), q = E((ue) => {
      y ? y.push(ue) : se.fire(ue);
    });
    M && M.add(q);
    const j = () => {
      y == null || y.forEach((ue) => se.fire(ue)), y = null;
    }, se = new Ae({
      onWillAddFirstListener() {
        q || (q = E((ue) => se.fire(ue)), M && M.add(q));
      },
      onDidAddFirstListener() {
        y && (v ? setTimeout(j) : j());
      },
      onDidRemoveLastListener() {
        q && q.dispose(), q = null;
      }
    });
    return M && M.add(se), se.event;
  }
  e.buffer = N;
  function A(E, v) {
    return (M, y, q) => {
      const j = v(new R());
      return E(function(se) {
        const ue = j.evaluate(se);
        ue !== x && M.call(y, ue);
      }, void 0, q);
    };
  }
  e.chain = A;
  const x = Symbol("HaltChainable");
  class R {
    constructor() {
      this.steps = [];
    }
    map(v) {
      return this.steps.push(v), this;
    }
    forEach(v) {
      return this.steps.push((k) => (v(k), k)), this;
    }
    filter(v) {
      return this.steps.push((k) => v(k) ? k : x), this;
    }
    reduce(v, k) {
      let M = k;
      return this.steps.push((y) => (M = v(M, y), M)), this;
    }
    latch(v = (k, M) => k === M) {
      let k = !0, M;
      return this.steps.push((y) => {
        const q = k || !v(y, M);
        return k = !1, M = y, q ? y : x;
      }), this;
    }
    evaluate(v) {
      for (const k of this.steps)
        if (v = k(v), v === x)
          break;
      return v;
    }
  }
  function U(E, v, k = (M) => M) {
    const M = (...se) => j.fire(k(...se)), y = () => E.on(v, M), q = () => E.removeListener(v, M), j = new Ae(
      { onWillAddFirstListener: y, onDidRemoveLastListener: q }
    );
    return j.event;
  }
  e.fromNodeEventEmitter = U;
  function p(E, v, k = (M) => M) {
    const M = (...se) => j.fire(k(...se)), y = () => E.addEventListener(v, M), q = () => E.removeEventListener(v, M), j = new Ae(
      { onWillAddFirstListener: y, onDidRemoveLastListener: q }
    );
    return j.event;
  }
  e.fromDOMEventEmitter = p;
  function w(E) {
    return new Promise((v) => n(E)(v));
  }
  e.toPromise = w;
  function D(E) {
    const v = new Ae();
    return E.then((k) => {
      v.fire(k);
    }, () => {
      v.fire(void 0);
    }).finally(() => {
      v.dispose();
    }), v.event;
  }
  e.fromPromise = D;
  function T(E, v) {
    return E((k) => v.fire(k));
  }
  e.forward = T;
  function I(E, v, k) {
    return v(k), E((M) => v(M));
  }
  e.runAndSubscribe = I;
  class $ {
    constructor(v, k) {
      this._observable = v, this._counter = 0, this._hasChanged = !1;
      const M = {
        onWillAddFirstListener: () => {
          v.addObserver(this);
        },
        onDidRemoveLastListener: () => {
          v.removeObserver(this);
        }
      };
      this.emitter = new Ae(M), k && k.add(this.emitter);
    }
    beginUpdate(v) {
      this._counter++;
    }
    handlePossibleChange(v) {
    }
    handleChange(v, k) {
      this._hasChanged = !0;
    }
    endUpdate(v) {
      this._counter--, this._counter === 0 && (this._observable.reportChanges(), this._hasChanged && (this._hasChanged = !1, this.emitter.fire(this._observable.get())));
    }
  }
  function ne(E, v) {
    return new $(E, v).emitter.event;
  }
  e.fromObservable = ne;
  function z(E) {
    return (v, k, M) => {
      let y = 0, q = !1;
      const j = {
        beginUpdate() {
          y++;
        },
        endUpdate() {
          y--, y === 0 && (E.reportChanges(), q && (q = !1, v.call(k)));
        },
        handlePossibleChange() {
        },
        handleChange() {
          q = !0;
        }
      };
      E.addObserver(j), E.reportChanges();
      const se = {
        dispose() {
          E.removeObserver(j);
        }
      };
      return M instanceof Ut ? M.add(se) : Array.isArray(M) && M.push(se), se;
    };
  }
  e.fromObservableLight = z;
})(pn || (pn = {}));
const lt = class lt {
  constructor(t) {
    this.listenerCount = 0, this.invocationCount = 0, this.elapsedOverall = 0, this.durations = [], this.name = `${t}_${lt._idPool++}`, lt.all.add(this);
  }
  start(t) {
    this._stopWatch = new on(), this.listenerCount = t;
  }
  stop() {
    if (this._stopWatch) {
      const t = this._stopWatch.elapsed();
      this.durations.push(t), this.elapsedOverall += t, this.invocationCount += 1, this._stopWatch = void 0;
    }
  }
};
lt.all = /* @__PURE__ */ new Set(), lt._idPool = 0;
let wn = lt, ul = -1;
const rn = class rn {
  constructor(t, n, i = (rn._idPool++).toString(16).padStart(3, "0")) {
    this._errorHandler = t, this.threshold = n, this.name = i, this._warnCountdown = 0;
  }
  dispose() {
    var t;
    (t = this._stacks) == null || t.clear();
  }
  check(t, n) {
    const i = this.threshold;
    if (i <= 0 || n < i)
      return;
    this._stacks || (this._stacks = /* @__PURE__ */ new Map());
    const s = this._stacks.get(t.value) || 0;
    if (this._stacks.set(t.value, s + 1), this._warnCountdown -= 1, this._warnCountdown <= 0) {
      this._warnCountdown = i * 0.5;
      const [r, a] = this.getMostFrequentStack(), u = `[${this.name}] potential listener LEAK detected, having ${n} listeners already. MOST frequent listener (${a}):`;
      console.warn(u), console.warn(r);
      const o = new ol(u, r);
      this._errorHandler(o);
    }
    return () => {
      const r = this._stacks.get(t.value) || 0;
      this._stacks.set(t.value, r - 1);
    };
  }
  getMostFrequentStack() {
    if (!this._stacks)
      return;
    let t, n = 0;
    for (const [i, s] of this._stacks)
      (!t || n < s) && (t = [i, s], n = s);
    return t;
  }
};
rn._idPool = 1;
let En = rn;
class Gn {
  static create() {
    const t = new Error();
    return new Gn(t.stack ?? "");
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
class ol extends Error {
  constructor(t, n) {
    super(t), this.name = "ListenerLeakError", this.stack = n;
  }
}
class cl extends Error {
  constructor(t, n) {
    super(t), this.name = "ListenerRefusalError", this.stack = n;
  }
}
let fl = 0;
class cn {
  constructor(t) {
    this.value = t, this.id = fl++;
  }
}
const hl = 2;
class Ae {
  constructor(t) {
    var n, i, s, r;
    this._size = 0, this._options = t, this._leakageMon = (n = this._options) != null && n.leakWarningThreshold ? new En(
      (t == null ? void 0 : t.onListenerError) ?? yt,
      ((i = this._options) == null ? void 0 : i.leakWarningThreshold) ?? ul
    ) : void 0, this._perfMon = (s = this._options) != null && s._profName ? new wn(this._options._profName) : void 0, this._deliveryQueue = (r = this._options) == null ? void 0 : r.deliveryQueue;
  }
  dispose() {
    var t, n, i, s;
    this._disposed || (this._disposed = !0, ((t = this._deliveryQueue) == null ? void 0 : t.current) === this && this._deliveryQueue.reset(), this._listeners && (this._listeners = void 0, this._size = 0), (i = (n = this._options) == null ? void 0 : n.onDidRemoveLastListener) == null || i.call(n), (s = this._leakageMon) == null || s.dispose());
  }
  get event() {
    return this._event ?? (this._event = (t, n, i) => {
      var u, o, c, h, f;
      if (this._leakageMon && this._size > this._leakageMon.threshold ** 2) {
        const g = `[${this._leakageMon.name}] REFUSES to accept new listeners because it exceeded its threshold by far (${this._size} vs ${this._leakageMon.threshold})`;
        console.warn(g);
        const b = this._leakageMon.getMostFrequentStack() ?? ["UNKNOWN stack", -1], L = new cl(
          `${g}. HINT: Stack shows most frequent listener (${b[1]}-times)`,
          b[0]
        );
        return (((u = this._options) == null ? void 0 : u.onListenerError) || yt)(L), ht.None;
      }
      if (this._disposed)
        return ht.None;
      n && (t = t.bind(n));
      const s = new cn(t);
      let r;
      this._leakageMon && this._size >= Math.ceil(this._leakageMon.threshold * 0.2) && (s.stack = Gn.create(), r = this._leakageMon.check(s.stack, this._size + 1)), this._listeners ? this._listeners instanceof cn ? (this._deliveryQueue ?? (this._deliveryQueue = new ml()), this._listeners = [this._listeners, s]) : this._listeners.push(s) : ((c = (o = this._options) == null ? void 0 : o.onWillAddFirstListener) == null || c.call(o, this), this._listeners = s, (f = (h = this._options) == null ? void 0 : h.onDidAddFirstListener) == null || f.call(h, this)), this._size++;
      const a = zt(() => {
        r == null || r(), this._removeListener(s);
      });
      return i instanceof Ut ? i.add(a) : Array.isArray(i) && i.push(a), a;
    }), this._event;
  }
  _removeListener(t) {
    var r, a, u, o;
    if ((a = (r = this._options) == null ? void 0 : r.onWillRemoveListener) == null || a.call(r, this), !this._listeners)
      return;
    if (this._size === 1) {
      this._listeners = void 0, (o = (u = this._options) == null ? void 0 : u.onDidRemoveLastListener) == null || o.call(u, this), this._size = 0;
      return;
    }
    const n = this._listeners, i = n.indexOf(t);
    if (i === -1)
      throw console.log("disposed?", this._disposed), console.log("size?", this._size), console.log("arr?", JSON.stringify(this._listeners)), new Error("Attempted to dispose unknown listener");
    this._size--, n[i] = void 0;
    const s = this._deliveryQueue.current === this;
    if (this._size * hl <= n.length) {
      let c = 0;
      for (let h = 0; h < n.length; h++)
        n[h] ? n[c++] = n[h] : s && (this._deliveryQueue.end--, c < this._deliveryQueue.i && this._deliveryQueue.i--);
      n.length = c;
    }
  }
  _deliver(t, n) {
    var s;
    if (!t)
      return;
    const i = ((s = this._options) == null ? void 0 : s.onListenerError) || yt;
    if (!i) {
      t.value(n);
      return;
    }
    try {
      t.value(n);
    } catch (r) {
      i(r);
    }
  }
  _deliverQueue(t) {
    const n = t.current._listeners;
    for (; t.i < t.end; )
      this._deliver(n[t.i++], t.value);
    t.reset();
  }
  fire(t) {
    var n, i, s, r;
    if ((n = this._deliveryQueue) != null && n.current && (this._deliverQueue(this._deliveryQueue), (i = this._perfMon) == null || i.stop()), (s = this._perfMon) == null || s.start(this._size), this._listeners) if (this._listeners instanceof cn)
      this._deliver(this._listeners, t);
    else {
      const a = this._deliveryQueue;
      a.enqueue(this, t, this._listeners.length), this._deliverQueue(a);
    }
    (r = this._perfMon) == null || r.stop();
  }
  hasListeners() {
    return this._size > 0;
  }
}
class ml {
  constructor() {
    this.i = -1, this.end = 0;
  }
  enqueue(t, n, i) {
    this.i = 0, this.end = i, this.current = t, this.value = n;
  }
  reset() {
    this.i = this.end, this.current = void 0, this.value = void 0;
  }
}
function gl(e) {
  return typeof e == "string";
}
function _l(e) {
  let t = [];
  for (; Object.prototype !== e; )
    t = t.concat(Object.getOwnPropertyNames(e)), e = Object.getPrototypeOf(e);
  return t;
}
function An(e) {
  const t = [];
  for (const n of _l(e))
    typeof e[n] == "function" && t.push(n);
  return t;
}
function bl(e, t) {
  const n = (s) => function() {
    const r = Array.prototype.slice.call(arguments, 0);
    return t(s, r);
  }, i = {};
  for (const s of e)
    i[s] = n(s);
  return i;
}
const tt = "en";
let Dt = !1, Tt = !1, Ot = !1, Lr = !1, Bt, Vt = tt, ni = tt, dl, Ee;
const je = globalThis;
let ae;
var hr;
typeof je.vscode < "u" && typeof je.vscode.process < "u" ? ae = je.vscode.process : typeof process < "u" && typeof ((hr = process == null ? void 0 : process.versions) == null ? void 0 : hr.node) == "string" && (ae = process);
var mr;
const Ll = typeof ((mr = ae == null ? void 0 : ae.versions) == null ? void 0 : mr.electron) == "string", Nl = Ll && (ae == null ? void 0 : ae.type) === "renderer";
var gr;
if (typeof ae == "object") {
  Dt = ae.platform === "win32", Tt = ae.platform === "darwin", Ot = ae.platform === "linux", Ot && ae.env.SNAP && ae.env.SNAP_REVISION, ae.env.CI || ae.env.BUILD_ARTIFACTSTAGINGDIRECTORY, Bt = tt, Vt = tt;
  const e = ae.env.VSCODE_NLS_CONFIG;
  if (e)
    try {
      const t = JSON.parse(e);
      Bt = t.userLocale, ni = t.osLocale, Vt = t.resolvedLanguage || tt, dl = (gr = t.languagePack) == null ? void 0 : gr.translationsConfigFile;
    } catch {
    }
} else typeof navigator == "object" && !Nl ? (Ee = navigator.userAgent, Dt = Ee.indexOf("Windows") >= 0, Tt = Ee.indexOf("Macintosh") >= 0, Lr = (Ee.indexOf("Macintosh") >= 0 || Ee.indexOf("iPad") >= 0 || Ee.indexOf("iPhone") >= 0) && !!navigator.maxTouchPoints && navigator.maxTouchPoints > 0, Ot = Ee.indexOf("Linux") >= 0, (Ee == null ? void 0 : Ee.indexOf("Mobi")) >= 0, Vt = globalThis._VSCODE_NLS_LANGUAGE || tt, Bt = navigator.language.toLowerCase(), ni = Bt) : console.error("Unable to resolve platform.");
var ut;
(function(e) {
  e[e.Web = 0] = "Web", e[e.Mac = 1] = "Mac", e[e.Linux = 2] = "Linux", e[e.Windows = 3] = "Windows";
})(ut || (ut = {}));
ut.Web;
Tt ? ut.Mac : Dt ? ut.Windows : Ot && ut.Linux;
const Mt = Dt, pl = Tt, De = Ee, Ie = Vt;
var ii;
(function(e) {
  function t() {
    return Ie;
  }
  e.value = t;
  function n() {
    return Ie.length === 2 ? Ie === "en" : Ie.length >= 3 ? Ie[0] === "e" && Ie[1] === "n" && Ie[2] === "-" : !1;
  }
  e.isDefaultVariant = n;
  function i() {
    return Ie === "en";
  }
  e.isDefault = i;
})(ii || (ii = {}));
const wl = typeof je.postMessage == "function" && !je.importScripts;
(() => {
  if (wl) {
    const e = [];
    je.addEventListener("message", (n) => {
      if (n.data && n.data.vscodeScheduleAsyncWork)
        for (let i = 0, s = e.length; i < s; i++) {
          const r = e[i];
          if (r.id === n.data.vscodeScheduleAsyncWork) {
            e.splice(i, 1), r.callback();
            return;
          }
        }
    });
    let t = 0;
    return (n) => {
      const i = ++t;
      e.push({
        id: i,
        callback: n
      }), je.postMessage({ vscodeScheduleAsyncWork: i }, "*");
    };
  }
  return (e) => setTimeout(e);
})();
var wt;
(function(e) {
  e[e.Windows = 1] = "Windows", e[e.Macintosh = 2] = "Macintosh", e[e.Linux = 3] = "Linux";
})(wt || (wt = {}));
Tt || Lr ? wt.Macintosh : Dt ? wt.Windows : wt.Linux;
const El = !!(De && De.indexOf("Chrome") >= 0);
De && De.indexOf("Firefox") >= 0;
!El && De && De.indexOf("Safari") >= 0;
De && De.indexOf("Edg/") >= 0;
De && De.indexOf("Android") >= 0;
const Nr = Object.freeze(function(e, t) {
  const n = setTimeout(e.bind(t), 0);
  return { dispose() {
    clearTimeout(n);
  } };
});
var jt;
(function(e) {
  function t(n) {
    return n === e.None || n === e.Cancelled || n instanceof Ht ? !0 : !n || typeof n != "object" ? !1 : typeof n.isCancellationRequested == "boolean" && typeof n.onCancellationRequested == "function";
  }
  e.isCancellationToken = t, e.None = Object.freeze({
    isCancellationRequested: !1,
    onCancellationRequested: pn.None
  }), e.Cancelled = Object.freeze({
    isCancellationRequested: !0,
    onCancellationRequested: Nr
  });
})(jt || (jt = {}));
class Ht {
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
    return this._isCancelled ? Nr : (this._emitter || (this._emitter = new Ae()), this._emitter.event);
  }
  dispose() {
    this._emitter && (this._emitter.dispose(), this._emitter = null);
  }
}
class Al {
  constructor(t) {
    this._token = void 0, this._parentListener = void 0, this._parentListener = t && t.onCancellationRequested(this.cancel, this);
  }
  get token() {
    return this._token || (this._token = new Ht()), this._token;
  }
  cancel() {
    this._token ? this._token instanceof Ht && this._token.cancel() : this._token = jt.Cancelled;
  }
  dispose(t = !1) {
    var n;
    t && this.cancel(), (n = this._parentListener) == null || n.dispose(), this._token ? this._token instanceof Ht && this._token.dispose() : this._token = jt.None;
  }
}
function xl(e) {
  return e;
}
class Rl {
  constructor(t, n) {
    this.lastCache = void 0, this.lastArgKey = void 0, typeof t == "function" ? (this._fn = t, this._computeKey = xl) : (this._fn = n, this._computeKey = t.getCacheKey);
  }
  get(t) {
    const n = this._computeKey(t);
    return this.lastArgKey !== n && (this.lastArgKey = n, this.lastCache = this._fn(t)), this.lastCache;
  }
}
var d;
(function(e) {
  e[e.Null = 0] = "Null", e[e.Backspace = 8] = "Backspace", e[e.Tab = 9] = "Tab", e[e.LineFeed = 10] = "LineFeed", e[e.CarriageReturn = 13] = "CarriageReturn", e[e.Space = 32] = "Space", e[e.ExclamationMark = 33] = "ExclamationMark", e[e.DoubleQuote = 34] = "DoubleQuote", e[e.Hash = 35] = "Hash", e[e.DollarSign = 36] = "DollarSign", e[e.PercentSign = 37] = "PercentSign", e[e.Ampersand = 38] = "Ampersand", e[e.SingleQuote = 39] = "SingleQuote", e[e.OpenParen = 40] = "OpenParen", e[e.CloseParen = 41] = "CloseParen", e[e.Asterisk = 42] = "Asterisk", e[e.Plus = 43] = "Plus", e[e.Comma = 44] = "Comma", e[e.Dash = 45] = "Dash", e[e.Period = 46] = "Period", e[e.Slash = 47] = "Slash", e[e.Digit0 = 48] = "Digit0", e[e.Digit1 = 49] = "Digit1", e[e.Digit2 = 50] = "Digit2", e[e.Digit3 = 51] = "Digit3", e[e.Digit4 = 52] = "Digit4", e[e.Digit5 = 53] = "Digit5", e[e.Digit6 = 54] = "Digit6", e[e.Digit7 = 55] = "Digit7", e[e.Digit8 = 56] = "Digit8", e[e.Digit9 = 57] = "Digit9", e[e.Colon = 58] = "Colon", e[e.Semicolon = 59] = "Semicolon", e[e.LessThan = 60] = "LessThan", e[e.Equals = 61] = "Equals", e[e.GreaterThan = 62] = "GreaterThan", e[e.QuestionMark = 63] = "QuestionMark", e[e.AtSign = 64] = "AtSign", e[e.A = 65] = "A", e[e.B = 66] = "B", e[e.C = 67] = "C", e[e.D = 68] = "D", e[e.E = 69] = "E", e[e.F = 70] = "F", e[e.G = 71] = "G", e[e.H = 72] = "H", e[e.I = 73] = "I", e[e.J = 74] = "J", e[e.K = 75] = "K", e[e.L = 76] = "L", e[e.M = 77] = "M", e[e.N = 78] = "N", e[e.O = 79] = "O", e[e.P = 80] = "P", e[e.Q = 81] = "Q", e[e.R = 82] = "R", e[e.S = 83] = "S", e[e.T = 84] = "T", e[e.U = 85] = "U", e[e.V = 86] = "V", e[e.W = 87] = "W", e[e.X = 88] = "X", e[e.Y = 89] = "Y", e[e.Z = 90] = "Z", e[e.OpenSquareBracket = 91] = "OpenSquareBracket", e[e.Backslash = 92] = "Backslash", e[e.CloseSquareBracket = 93] = "CloseSquareBracket", e[e.Caret = 94] = "Caret", e[e.Underline = 95] = "Underline", e[e.BackTick = 96] = "BackTick", e[e.a = 97] = "a", e[e.b = 98] = "b", e[e.c = 99] = "c", e[e.d = 100] = "d", e[e.e = 101] = "e", e[e.f = 102] = "f", e[e.g = 103] = "g", e[e.h = 104] = "h", e[e.i = 105] = "i", e[e.j = 106] = "j", e[e.k = 107] = "k", e[e.l = 108] = "l", e[e.m = 109] = "m", e[e.n = 110] = "n", e[e.o = 111] = "o", e[e.p = 112] = "p", e[e.q = 113] = "q", e[e.r = 114] = "r", e[e.s = 115] = "s", e[e.t = 116] = "t", e[e.u = 117] = "u", e[e.v = 118] = "v", e[e.w = 119] = "w", e[e.x = 120] = "x", e[e.y = 121] = "y", e[e.z = 122] = "z", e[e.OpenCurlyBrace = 123] = "OpenCurlyBrace", e[e.Pipe = 124] = "Pipe", e[e.CloseCurlyBrace = 125] = "CloseCurlyBrace", e[e.Tilde = 126] = "Tilde", e[e.NoBreakSpace = 160] = "NoBreakSpace", e[e.U_Combining_Grave_Accent = 768] = "U_Combining_Grave_Accent", e[e.U_Combining_Acute_Accent = 769] = "U_Combining_Acute_Accent", e[e.U_Combining_Circumflex_Accent = 770] = "U_Combining_Circumflex_Accent", e[e.U_Combining_Tilde = 771] = "U_Combining_Tilde", e[e.U_Combining_Macron = 772] = "U_Combining_Macron", e[e.U_Combining_Overline = 773] = "U_Combining_Overline", e[e.U_Combining_Breve = 774] = "U_Combining_Breve", e[e.U_Combining_Dot_Above = 775] = "U_Combining_Dot_Above", e[e.U_Combining_Diaeresis = 776] = "U_Combining_Diaeresis", e[e.U_Combining_Hook_Above = 777] = "U_Combining_Hook_Above", e[e.U_Combining_Ring_Above = 778] = "U_Combining_Ring_Above", e[e.U_Combining_Double_Acute_Accent = 779] = "U_Combining_Double_Acute_Accent", e[e.U_Combining_Caron = 780] = "U_Combining_Caron", e[e.U_Combining_Vertical_Line_Above = 781] = "U_Combining_Vertical_Line_Above", e[e.U_Combining_Double_Vertical_Line_Above = 782] = "U_Combining_Double_Vertical_Line_Above", e[e.U_Combining_Double_Grave_Accent = 783] = "U_Combining_Double_Grave_Accent", e[e.U_Combining_Candrabindu = 784] = "U_Combining_Candrabindu", e[e.U_Combining_Inverted_Breve = 785] = "U_Combining_Inverted_Breve", e[e.U_Combining_Turned_Comma_Above = 786] = "U_Combining_Turned_Comma_Above", e[e.U_Combining_Comma_Above = 787] = "U_Combining_Comma_Above", e[e.U_Combining_Reversed_Comma_Above = 788] = "U_Combining_Reversed_Comma_Above", e[e.U_Combining_Comma_Above_Right = 789] = "U_Combining_Comma_Above_Right", e[e.U_Combining_Grave_Accent_Below = 790] = "U_Combining_Grave_Accent_Below", e[e.U_Combining_Acute_Accent_Below = 791] = "U_Combining_Acute_Accent_Below", e[e.U_Combining_Left_Tack_Below = 792] = "U_Combining_Left_Tack_Below", e[e.U_Combining_Right_Tack_Below = 793] = "U_Combining_Right_Tack_Below", e[e.U_Combining_Left_Angle_Above = 794] = "U_Combining_Left_Angle_Above", e[e.U_Combining_Horn = 795] = "U_Combining_Horn", e[e.U_Combining_Left_Half_Ring_Below = 796] = "U_Combining_Left_Half_Ring_Below", e[e.U_Combining_Up_Tack_Below = 797] = "U_Combining_Up_Tack_Below", e[e.U_Combining_Down_Tack_Below = 798] = "U_Combining_Down_Tack_Below", e[e.U_Combining_Plus_Sign_Below = 799] = "U_Combining_Plus_Sign_Below", e[e.U_Combining_Minus_Sign_Below = 800] = "U_Combining_Minus_Sign_Below", e[e.U_Combining_Palatalized_Hook_Below = 801] = "U_Combining_Palatalized_Hook_Below", e[e.U_Combining_Retroflex_Hook_Below = 802] = "U_Combining_Retroflex_Hook_Below", e[e.U_Combining_Dot_Below = 803] = "U_Combining_Dot_Below", e[e.U_Combining_Diaeresis_Below = 804] = "U_Combining_Diaeresis_Below", e[e.U_Combining_Ring_Below = 805] = "U_Combining_Ring_Below", e[e.U_Combining_Comma_Below = 806] = "U_Combining_Comma_Below", e[e.U_Combining_Cedilla = 807] = "U_Combining_Cedilla", e[e.U_Combining_Ogonek = 808] = "U_Combining_Ogonek", e[e.U_Combining_Vertical_Line_Below = 809] = "U_Combining_Vertical_Line_Below", e[e.U_Combining_Bridge_Below = 810] = "U_Combining_Bridge_Below", e[e.U_Combining_Inverted_Double_Arch_Below = 811] = "U_Combining_Inverted_Double_Arch_Below", e[e.U_Combining_Caron_Below = 812] = "U_Combining_Caron_Below", e[e.U_Combining_Circumflex_Accent_Below = 813] = "U_Combining_Circumflex_Accent_Below", e[e.U_Combining_Breve_Below = 814] = "U_Combining_Breve_Below", e[e.U_Combining_Inverted_Breve_Below = 815] = "U_Combining_Inverted_Breve_Below", e[e.U_Combining_Tilde_Below = 816] = "U_Combining_Tilde_Below", e[e.U_Combining_Macron_Below = 817] = "U_Combining_Macron_Below", e[e.U_Combining_Low_Line = 818] = "U_Combining_Low_Line", e[e.U_Combining_Double_Low_Line = 819] = "U_Combining_Double_Low_Line", e[e.U_Combining_Tilde_Overlay = 820] = "U_Combining_Tilde_Overlay", e[e.U_Combining_Short_Stroke_Overlay = 821] = "U_Combining_Short_Stroke_Overlay", e[e.U_Combining_Long_Stroke_Overlay = 822] = "U_Combining_Long_Stroke_Overlay", e[e.U_Combining_Short_Solidus_Overlay = 823] = "U_Combining_Short_Solidus_Overlay", e[e.U_Combining_Long_Solidus_Overlay = 824] = "U_Combining_Long_Solidus_Overlay", e[e.U_Combining_Right_Half_Ring_Below = 825] = "U_Combining_Right_Half_Ring_Below", e[e.U_Combining_Inverted_Bridge_Below = 826] = "U_Combining_Inverted_Bridge_Below", e[e.U_Combining_Square_Below = 827] = "U_Combining_Square_Below", e[e.U_Combining_Seagull_Below = 828] = "U_Combining_Seagull_Below", e[e.U_Combining_X_Above = 829] = "U_Combining_X_Above", e[e.U_Combining_Vertical_Tilde = 830] = "U_Combining_Vertical_Tilde", e[e.U_Combining_Double_Overline = 831] = "U_Combining_Double_Overline", e[e.U_Combining_Grave_Tone_Mark = 832] = "U_Combining_Grave_Tone_Mark", e[e.U_Combining_Acute_Tone_Mark = 833] = "U_Combining_Acute_Tone_Mark", e[e.U_Combining_Greek_Perispomeni = 834] = "U_Combining_Greek_Perispomeni", e[e.U_Combining_Greek_Koronis = 835] = "U_Combining_Greek_Koronis", e[e.U_Combining_Greek_Dialytika_Tonos = 836] = "U_Combining_Greek_Dialytika_Tonos", e[e.U_Combining_Greek_Ypogegrammeni = 837] = "U_Combining_Greek_Ypogegrammeni", e[e.U_Combining_Bridge_Above = 838] = "U_Combining_Bridge_Above", e[e.U_Combining_Equals_Sign_Below = 839] = "U_Combining_Equals_Sign_Below", e[e.U_Combining_Double_Vertical_Line_Below = 840] = "U_Combining_Double_Vertical_Line_Below", e[e.U_Combining_Left_Angle_Below = 841] = "U_Combining_Left_Angle_Below", e[e.U_Combining_Not_Tilde_Above = 842] = "U_Combining_Not_Tilde_Above", e[e.U_Combining_Homothetic_Above = 843] = "U_Combining_Homothetic_Above", e[e.U_Combining_Almost_Equal_To_Above = 844] = "U_Combining_Almost_Equal_To_Above", e[e.U_Combining_Left_Right_Arrow_Below = 845] = "U_Combining_Left_Right_Arrow_Below", e[e.U_Combining_Upwards_Arrow_Below = 846] = "U_Combining_Upwards_Arrow_Below", e[e.U_Combining_Grapheme_Joiner = 847] = "U_Combining_Grapheme_Joiner", e[e.U_Combining_Right_Arrowhead_Above = 848] = "U_Combining_Right_Arrowhead_Above", e[e.U_Combining_Left_Half_Ring_Above = 849] = "U_Combining_Left_Half_Ring_Above", e[e.U_Combining_Fermata = 850] = "U_Combining_Fermata", e[e.U_Combining_X_Below = 851] = "U_Combining_X_Below", e[e.U_Combining_Left_Arrowhead_Below = 852] = "U_Combining_Left_Arrowhead_Below", e[e.U_Combining_Right_Arrowhead_Below = 853] = "U_Combining_Right_Arrowhead_Below", e[e.U_Combining_Right_Arrowhead_And_Up_Arrowhead_Below = 854] = "U_Combining_Right_Arrowhead_And_Up_Arrowhead_Below", e[e.U_Combining_Right_Half_Ring_Above = 855] = "U_Combining_Right_Half_Ring_Above", e[e.U_Combining_Dot_Above_Right = 856] = "U_Combining_Dot_Above_Right", e[e.U_Combining_Asterisk_Below = 857] = "U_Combining_Asterisk_Below", e[e.U_Combining_Double_Ring_Below = 858] = "U_Combining_Double_Ring_Below", e[e.U_Combining_Zigzag_Above = 859] = "U_Combining_Zigzag_Above", e[e.U_Combining_Double_Breve_Below = 860] = "U_Combining_Double_Breve_Below", e[e.U_Combining_Double_Breve = 861] = "U_Combining_Double_Breve", e[e.U_Combining_Double_Macron = 862] = "U_Combining_Double_Macron", e[e.U_Combining_Double_Macron_Below = 863] = "U_Combining_Double_Macron_Below", e[e.U_Combining_Double_Tilde = 864] = "U_Combining_Double_Tilde", e[e.U_Combining_Double_Inverted_Breve = 865] = "U_Combining_Double_Inverted_Breve", e[e.U_Combining_Double_Rightwards_Arrow_Below = 866] = "U_Combining_Double_Rightwards_Arrow_Below", e[e.U_Combining_Latin_Small_Letter_A = 867] = "U_Combining_Latin_Small_Letter_A", e[e.U_Combining_Latin_Small_Letter_E = 868] = "U_Combining_Latin_Small_Letter_E", e[e.U_Combining_Latin_Small_Letter_I = 869] = "U_Combining_Latin_Small_Letter_I", e[e.U_Combining_Latin_Small_Letter_O = 870] = "U_Combining_Latin_Small_Letter_O", e[e.U_Combining_Latin_Small_Letter_U = 871] = "U_Combining_Latin_Small_Letter_U", e[e.U_Combining_Latin_Small_Letter_C = 872] = "U_Combining_Latin_Small_Letter_C", e[e.U_Combining_Latin_Small_Letter_D = 873] = "U_Combining_Latin_Small_Letter_D", e[e.U_Combining_Latin_Small_Letter_H = 874] = "U_Combining_Latin_Small_Letter_H", e[e.U_Combining_Latin_Small_Letter_M = 875] = "U_Combining_Latin_Small_Letter_M", e[e.U_Combining_Latin_Small_Letter_R = 876] = "U_Combining_Latin_Small_Letter_R", e[e.U_Combining_Latin_Small_Letter_T = 877] = "U_Combining_Latin_Small_Letter_T", e[e.U_Combining_Latin_Small_Letter_V = 878] = "U_Combining_Latin_Small_Letter_V", e[e.U_Combining_Latin_Small_Letter_X = 879] = "U_Combining_Latin_Small_Letter_X", e[e.LINE_SEPARATOR = 8232] = "LINE_SEPARATOR", e[e.PARAGRAPH_SEPARATOR = 8233] = "PARAGRAPH_SEPARATOR", e[e.NEXT_LINE = 133] = "NEXT_LINE", e[e.U_CIRCUMFLEX = 94] = "U_CIRCUMFLEX", e[e.U_GRAVE_ACCENT = 96] = "U_GRAVE_ACCENT", e[e.U_DIAERESIS = 168] = "U_DIAERESIS", e[e.U_MACRON = 175] = "U_MACRON", e[e.U_ACUTE_ACCENT = 180] = "U_ACUTE_ACCENT", e[e.U_CEDILLA = 184] = "U_CEDILLA", e[e.U_MODIFIER_LETTER_LEFT_ARROWHEAD = 706] = "U_MODIFIER_LETTER_LEFT_ARROWHEAD", e[e.U_MODIFIER_LETTER_RIGHT_ARROWHEAD = 707] = "U_MODIFIER_LETTER_RIGHT_ARROWHEAD", e[e.U_MODIFIER_LETTER_UP_ARROWHEAD = 708] = "U_MODIFIER_LETTER_UP_ARROWHEAD", e[e.U_MODIFIER_LETTER_DOWN_ARROWHEAD = 709] = "U_MODIFIER_LETTER_DOWN_ARROWHEAD", e[e.U_MODIFIER_LETTER_CENTRED_RIGHT_HALF_RING = 722] = "U_MODIFIER_LETTER_CENTRED_RIGHT_HALF_RING", e[e.U_MODIFIER_LETTER_CENTRED_LEFT_HALF_RING = 723] = "U_MODIFIER_LETTER_CENTRED_LEFT_HALF_RING", e[e.U_MODIFIER_LETTER_UP_TACK = 724] = "U_MODIFIER_LETTER_UP_TACK", e[e.U_MODIFIER_LETTER_DOWN_TACK = 725] = "U_MODIFIER_LETTER_DOWN_TACK", e[e.U_MODIFIER_LETTER_PLUS_SIGN = 726] = "U_MODIFIER_LETTER_PLUS_SIGN", e[e.U_MODIFIER_LETTER_MINUS_SIGN = 727] = "U_MODIFIER_LETTER_MINUS_SIGN", e[e.U_BREVE = 728] = "U_BREVE", e[e.U_DOT_ABOVE = 729] = "U_DOT_ABOVE", e[e.U_RING_ABOVE = 730] = "U_RING_ABOVE", e[e.U_OGONEK = 731] = "U_OGONEK", e[e.U_SMALL_TILDE = 732] = "U_SMALL_TILDE", e[e.U_DOUBLE_ACUTE_ACCENT = 733] = "U_DOUBLE_ACUTE_ACCENT", e[e.U_MODIFIER_LETTER_RHOTIC_HOOK = 734] = "U_MODIFIER_LETTER_RHOTIC_HOOK", e[e.U_MODIFIER_LETTER_CROSS_ACCENT = 735] = "U_MODIFIER_LETTER_CROSS_ACCENT", e[e.U_MODIFIER_LETTER_EXTRA_HIGH_TONE_BAR = 741] = "U_MODIFIER_LETTER_EXTRA_HIGH_TONE_BAR", e[e.U_MODIFIER_LETTER_HIGH_TONE_BAR = 742] = "U_MODIFIER_LETTER_HIGH_TONE_BAR", e[e.U_MODIFIER_LETTER_MID_TONE_BAR = 743] = "U_MODIFIER_LETTER_MID_TONE_BAR", e[e.U_MODIFIER_LETTER_LOW_TONE_BAR = 744] = "U_MODIFIER_LETTER_LOW_TONE_BAR", e[e.U_MODIFIER_LETTER_EXTRA_LOW_TONE_BAR = 745] = "U_MODIFIER_LETTER_EXTRA_LOW_TONE_BAR", e[e.U_MODIFIER_LETTER_YIN_DEPARTING_TONE_MARK = 746] = "U_MODIFIER_LETTER_YIN_DEPARTING_TONE_MARK", e[e.U_MODIFIER_LETTER_YANG_DEPARTING_TONE_MARK = 747] = "U_MODIFIER_LETTER_YANG_DEPARTING_TONE_MARK", e[e.U_MODIFIER_LETTER_UNASPIRATED = 749] = "U_MODIFIER_LETTER_UNASPIRATED", e[e.U_MODIFIER_LETTER_LOW_DOWN_ARROWHEAD = 751] = "U_MODIFIER_LETTER_LOW_DOWN_ARROWHEAD", e[e.U_MODIFIER_LETTER_LOW_UP_ARROWHEAD = 752] = "U_MODIFIER_LETTER_LOW_UP_ARROWHEAD", e[e.U_MODIFIER_LETTER_LOW_LEFT_ARROWHEAD = 753] = "U_MODIFIER_LETTER_LOW_LEFT_ARROWHEAD", e[e.U_MODIFIER_LETTER_LOW_RIGHT_ARROWHEAD = 754] = "U_MODIFIER_LETTER_LOW_RIGHT_ARROWHEAD", e[e.U_MODIFIER_LETTER_LOW_RING = 755] = "U_MODIFIER_LETTER_LOW_RING", e[e.U_MODIFIER_LETTER_MIDDLE_GRAVE_ACCENT = 756] = "U_MODIFIER_LETTER_MIDDLE_GRAVE_ACCENT", e[e.U_MODIFIER_LETTER_MIDDLE_DOUBLE_GRAVE_ACCENT = 757] = "U_MODIFIER_LETTER_MIDDLE_DOUBLE_GRAVE_ACCENT", e[e.U_MODIFIER_LETTER_MIDDLE_DOUBLE_ACUTE_ACCENT = 758] = "U_MODIFIER_LETTER_MIDDLE_DOUBLE_ACUTE_ACCENT", e[e.U_MODIFIER_LETTER_LOW_TILDE = 759] = "U_MODIFIER_LETTER_LOW_TILDE", e[e.U_MODIFIER_LETTER_RAISED_COLON = 760] = "U_MODIFIER_LETTER_RAISED_COLON", e[e.U_MODIFIER_LETTER_BEGIN_HIGH_TONE = 761] = "U_MODIFIER_LETTER_BEGIN_HIGH_TONE", e[e.U_MODIFIER_LETTER_END_HIGH_TONE = 762] = "U_MODIFIER_LETTER_END_HIGH_TONE", e[e.U_MODIFIER_LETTER_BEGIN_LOW_TONE = 763] = "U_MODIFIER_LETTER_BEGIN_LOW_TONE", e[e.U_MODIFIER_LETTER_END_LOW_TONE = 764] = "U_MODIFIER_LETTER_END_LOW_TONE", e[e.U_MODIFIER_LETTER_SHELF = 765] = "U_MODIFIER_LETTER_SHELF", e[e.U_MODIFIER_LETTER_OPEN_SHELF = 766] = "U_MODIFIER_LETTER_OPEN_SHELF", e[e.U_MODIFIER_LETTER_LOW_LEFT_ARROW = 767] = "U_MODIFIER_LETTER_LOW_LEFT_ARROW", e[e.U_GREEK_LOWER_NUMERAL_SIGN = 885] = "U_GREEK_LOWER_NUMERAL_SIGN", e[e.U_GREEK_TONOS = 900] = "U_GREEK_TONOS", e[e.U_GREEK_DIALYTIKA_TONOS = 901] = "U_GREEK_DIALYTIKA_TONOS", e[e.U_GREEK_KORONIS = 8125] = "U_GREEK_KORONIS", e[e.U_GREEK_PSILI = 8127] = "U_GREEK_PSILI", e[e.U_GREEK_PERISPOMENI = 8128] = "U_GREEK_PERISPOMENI", e[e.U_GREEK_DIALYTIKA_AND_PERISPOMENI = 8129] = "U_GREEK_DIALYTIKA_AND_PERISPOMENI", e[e.U_GREEK_PSILI_AND_VARIA = 8141] = "U_GREEK_PSILI_AND_VARIA", e[e.U_GREEK_PSILI_AND_OXIA = 8142] = "U_GREEK_PSILI_AND_OXIA", e[e.U_GREEK_PSILI_AND_PERISPOMENI = 8143] = "U_GREEK_PSILI_AND_PERISPOMENI", e[e.U_GREEK_DASIA_AND_VARIA = 8157] = "U_GREEK_DASIA_AND_VARIA", e[e.U_GREEK_DASIA_AND_OXIA = 8158] = "U_GREEK_DASIA_AND_OXIA", e[e.U_GREEK_DASIA_AND_PERISPOMENI = 8159] = "U_GREEK_DASIA_AND_PERISPOMENI", e[e.U_GREEK_DIALYTIKA_AND_VARIA = 8173] = "U_GREEK_DIALYTIKA_AND_VARIA", e[e.U_GREEK_DIALYTIKA_AND_OXIA = 8174] = "U_GREEK_DIALYTIKA_AND_OXIA", e[e.U_GREEK_VARIA = 8175] = "U_GREEK_VARIA", e[e.U_GREEK_OXIA = 8189] = "U_GREEK_OXIA", e[e.U_GREEK_DASIA = 8190] = "U_GREEK_DASIA", e[e.U_IDEOGRAPHIC_FULL_STOP = 12290] = "U_IDEOGRAPHIC_FULL_STOP", e[e.U_LEFT_CORNER_BRACKET = 12300] = "U_LEFT_CORNER_BRACKET", e[e.U_RIGHT_CORNER_BRACKET = 12301] = "U_RIGHT_CORNER_BRACKET", e[e.U_LEFT_BLACK_LENTICULAR_BRACKET = 12304] = "U_LEFT_BLACK_LENTICULAR_BRACKET", e[e.U_RIGHT_BLACK_LENTICULAR_BRACKET = 12305] = "U_RIGHT_BLACK_LENTICULAR_BRACKET", e[e.U_OVERLINE = 8254] = "U_OVERLINE", e[e.UTF8_BOM = 65279] = "UTF8_BOM", e[e.U_FULLWIDTH_SEMICOLON = 65307] = "U_FULLWIDTH_SEMICOLON", e[e.U_FULLWIDTH_COMMA = 65292] = "U_FULLWIDTH_COMMA";
})(d || (d = {}));
class si {
  constructor(t) {
    this.executor = t, this._didRun = !1;
  }
  get hasValue() {
    return this._didRun;
  }
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
  get rawValue() {
    return this._value;
  }
}
var pe;
(function(e) {
  e[e.MAX_SAFE_SMALL_INTEGER = 1073741824] = "MAX_SAFE_SMALL_INTEGER", e[e.MIN_SAFE_SMALL_INTEGER = -1073741824] = "MIN_SAFE_SMALL_INTEGER", e[e.MAX_UINT_8 = 255] = "MAX_UINT_8", e[e.MAX_UINT_16 = 65535] = "MAX_UINT_16", e[e.MAX_UINT_32 = 4294967295] = "MAX_UINT_32", e[e.UNICODE_SUPPLEMENTARY_PLANE_BEGIN = 65536] = "UNICODE_SUPPLEMENTARY_PLANE_BEGIN";
})(pe || (pe = {}));
function ri(e) {
  return e < 0 ? 0 : e > pe.MAX_UINT_8 ? pe.MAX_UINT_8 : e | 0;
}
function Ze(e) {
  return e < 0 ? 0 : e > pe.MAX_UINT_32 ? pe.MAX_UINT_32 : e | 0;
}
function vl(e) {
  return e.replace(/[\\\{\}\*\+\?\|\^\$\.\[\]\(\)]/g, "\\$&");
}
function Ul(e) {
  return e.split(/\r\n|\r|\n/);
}
function Dl(e) {
  for (let t = 0, n = e.length; t < n; t++) {
    const i = e.charCodeAt(t);
    if (i !== d.Space && i !== d.Tab)
      return t;
  }
  return -1;
}
function Tl(e, t = e.length - 1) {
  for (let n = t; n >= 0; n--) {
    const i = e.charCodeAt(n);
    if (i !== d.Space && i !== d.Tab)
      return n;
  }
  return -1;
}
function pr(e) {
  return e >= d.A && e <= d.Z;
}
function Xt(e) {
  return 55296 <= e && e <= 56319;
}
function xn(e) {
  return 56320 <= e && e <= 57343;
}
function wr(e, t) {
  return (e - 55296 << 10) + (t - 56320) + 65536;
}
function Ml(e, t, n) {
  const i = e.charCodeAt(n);
  if (Xt(i) && n + 1 < t) {
    const s = e.charCodeAt(n + 1);
    if (xn(s))
      return wr(i, s);
  }
  return i;
}
const Fl = /^[\t\n\r\x20-\x7E]*$/;
function kl(e) {
  return Fl.test(e);
}
String.fromCharCode(d.UTF8_BOM);
var li;
(function(e) {
  e[e.Other = 0] = "Other", e[e.Prepend = 1] = "Prepend", e[e.CR = 2] = "CR", e[e.LF = 3] = "LF", e[e.Control = 4] = "Control", e[e.Extend = 5] = "Extend", e[e.Regional_Indicator = 6] = "Regional_Indicator", e[e.SpacingMark = 7] = "SpacingMark", e[e.L = 8] = "L", e[e.V = 9] = "V", e[e.T = 10] = "T", e[e.LV = 11] = "LV", e[e.LVT = 12] = "LVT", e[e.ZWJ = 13] = "ZWJ", e[e.Extended_Pictographic = 14] = "Extended_Pictographic";
})(li || (li = {}));
var ai;
(function(e) {
  e[e.zwj = 8205] = "zwj", e[e.emojiVariantSelector = 65039] = "emojiVariantSelector", e[e.enclosingKeyCap = 8419] = "enclosingKeyCap";
})(ai || (ai = {}));
const ve = class ve {
  static getInstance(t) {
    return ve.cache.get(Array.from(t));
  }
  static getLocales() {
    return ve._locales.value;
  }
  constructor(t) {
    this.confusableDictionary = t;
  }
  isAmbiguous(t) {
    return this.confusableDictionary.has(t);
  }
  containsAmbiguousCharacter(t) {
    for (let n = 0; n < t.length; n++) {
      const i = t.codePointAt(n);
      if (typeof i == "number" && this.isAmbiguous(i))
        return !0;
    }
    return !1;
  }
  getPrimaryConfusable(t) {
    return this.confusableDictionary.get(t);
  }
  getConfusableCodePoints() {
    return new Set(this.confusableDictionary.keys());
  }
};
ve.ambiguousCharacterData = new si(() => JSON.parse('{"_common":[8232,32,8233,32,5760,32,8192,32,8193,32,8194,32,8195,32,8196,32,8197,32,8198,32,8200,32,8201,32,8202,32,8287,32,8199,32,8239,32,2042,95,65101,95,65102,95,65103,95,8208,45,8209,45,8210,45,65112,45,1748,45,8259,45,727,45,8722,45,10134,45,11450,45,1549,44,1643,44,8218,44,184,44,42233,44,894,59,2307,58,2691,58,1417,58,1795,58,1796,58,5868,58,65072,58,6147,58,6153,58,8282,58,1475,58,760,58,42889,58,8758,58,720,58,42237,58,451,33,11601,33,660,63,577,63,2429,63,5038,63,42731,63,119149,46,8228,46,1793,46,1794,46,42510,46,68176,46,1632,46,1776,46,42232,46,1373,96,65287,96,8219,96,8242,96,1370,96,1523,96,8175,96,65344,96,900,96,8189,96,8125,96,8127,96,8190,96,697,96,884,96,712,96,714,96,715,96,756,96,699,96,701,96,700,96,702,96,42892,96,1497,96,2036,96,2037,96,5194,96,5836,96,94033,96,94034,96,65339,91,10088,40,10098,40,12308,40,64830,40,65341,93,10089,41,10099,41,12309,41,64831,41,10100,123,119060,123,10101,125,65342,94,8270,42,1645,42,8727,42,66335,42,5941,47,8257,47,8725,47,8260,47,9585,47,10187,47,10744,47,119354,47,12755,47,12339,47,11462,47,20031,47,12035,47,65340,92,65128,92,8726,92,10189,92,10741,92,10745,92,119311,92,119355,92,12756,92,20022,92,12034,92,42872,38,708,94,710,94,5869,43,10133,43,66203,43,8249,60,10094,60,706,60,119350,60,5176,60,5810,60,5120,61,11840,61,12448,61,42239,61,8250,62,10095,62,707,62,119351,62,5171,62,94015,62,8275,126,732,126,8128,126,8764,126,65372,124,65293,45,120784,50,120794,50,120804,50,120814,50,120824,50,130034,50,42842,50,423,50,1000,50,42564,50,5311,50,42735,50,119302,51,120785,51,120795,51,120805,51,120815,51,120825,51,130035,51,42923,51,540,51,439,51,42858,51,11468,51,1248,51,94011,51,71882,51,120786,52,120796,52,120806,52,120816,52,120826,52,130036,52,5070,52,71855,52,120787,53,120797,53,120807,53,120817,53,120827,53,130037,53,444,53,71867,53,120788,54,120798,54,120808,54,120818,54,120828,54,130038,54,11474,54,5102,54,71893,54,119314,55,120789,55,120799,55,120809,55,120819,55,120829,55,130039,55,66770,55,71878,55,2819,56,2538,56,2666,56,125131,56,120790,56,120800,56,120810,56,120820,56,120830,56,130040,56,547,56,546,56,66330,56,2663,57,2920,57,2541,57,3437,57,120791,57,120801,57,120811,57,120821,57,120831,57,130041,57,42862,57,11466,57,71884,57,71852,57,71894,57,9082,97,65345,97,119834,97,119886,97,119938,97,119990,97,120042,97,120094,97,120146,97,120198,97,120250,97,120302,97,120354,97,120406,97,120458,97,593,97,945,97,120514,97,120572,97,120630,97,120688,97,120746,97,65313,65,119808,65,119860,65,119912,65,119964,65,120016,65,120068,65,120120,65,120172,65,120224,65,120276,65,120328,65,120380,65,120432,65,913,65,120488,65,120546,65,120604,65,120662,65,120720,65,5034,65,5573,65,42222,65,94016,65,66208,65,119835,98,119887,98,119939,98,119991,98,120043,98,120095,98,120147,98,120199,98,120251,98,120303,98,120355,98,120407,98,120459,98,388,98,5071,98,5234,98,5551,98,65314,66,8492,66,119809,66,119861,66,119913,66,120017,66,120069,66,120121,66,120173,66,120225,66,120277,66,120329,66,120381,66,120433,66,42932,66,914,66,120489,66,120547,66,120605,66,120663,66,120721,66,5108,66,5623,66,42192,66,66178,66,66209,66,66305,66,65347,99,8573,99,119836,99,119888,99,119940,99,119992,99,120044,99,120096,99,120148,99,120200,99,120252,99,120304,99,120356,99,120408,99,120460,99,7428,99,1010,99,11429,99,43951,99,66621,99,128844,67,71922,67,71913,67,65315,67,8557,67,8450,67,8493,67,119810,67,119862,67,119914,67,119966,67,120018,67,120174,67,120226,67,120278,67,120330,67,120382,67,120434,67,1017,67,11428,67,5087,67,42202,67,66210,67,66306,67,66581,67,66844,67,8574,100,8518,100,119837,100,119889,100,119941,100,119993,100,120045,100,120097,100,120149,100,120201,100,120253,100,120305,100,120357,100,120409,100,120461,100,1281,100,5095,100,5231,100,42194,100,8558,68,8517,68,119811,68,119863,68,119915,68,119967,68,120019,68,120071,68,120123,68,120175,68,120227,68,120279,68,120331,68,120383,68,120435,68,5024,68,5598,68,5610,68,42195,68,8494,101,65349,101,8495,101,8519,101,119838,101,119890,101,119942,101,120046,101,120098,101,120150,101,120202,101,120254,101,120306,101,120358,101,120410,101,120462,101,43826,101,1213,101,8959,69,65317,69,8496,69,119812,69,119864,69,119916,69,120020,69,120072,69,120124,69,120176,69,120228,69,120280,69,120332,69,120384,69,120436,69,917,69,120492,69,120550,69,120608,69,120666,69,120724,69,11577,69,5036,69,42224,69,71846,69,71854,69,66182,69,119839,102,119891,102,119943,102,119995,102,120047,102,120099,102,120151,102,120203,102,120255,102,120307,102,120359,102,120411,102,120463,102,43829,102,42905,102,383,102,7837,102,1412,102,119315,70,8497,70,119813,70,119865,70,119917,70,120021,70,120073,70,120125,70,120177,70,120229,70,120281,70,120333,70,120385,70,120437,70,42904,70,988,70,120778,70,5556,70,42205,70,71874,70,71842,70,66183,70,66213,70,66853,70,65351,103,8458,103,119840,103,119892,103,119944,103,120048,103,120100,103,120152,103,120204,103,120256,103,120308,103,120360,103,120412,103,120464,103,609,103,7555,103,397,103,1409,103,119814,71,119866,71,119918,71,119970,71,120022,71,120074,71,120126,71,120178,71,120230,71,120282,71,120334,71,120386,71,120438,71,1292,71,5056,71,5107,71,42198,71,65352,104,8462,104,119841,104,119945,104,119997,104,120049,104,120101,104,120153,104,120205,104,120257,104,120309,104,120361,104,120413,104,120465,104,1211,104,1392,104,5058,104,65320,72,8459,72,8460,72,8461,72,119815,72,119867,72,119919,72,120023,72,120179,72,120231,72,120283,72,120335,72,120387,72,120439,72,919,72,120494,72,120552,72,120610,72,120668,72,120726,72,11406,72,5051,72,5500,72,42215,72,66255,72,731,105,9075,105,65353,105,8560,105,8505,105,8520,105,119842,105,119894,105,119946,105,119998,105,120050,105,120102,105,120154,105,120206,105,120258,105,120310,105,120362,105,120414,105,120466,105,120484,105,618,105,617,105,953,105,8126,105,890,105,120522,105,120580,105,120638,105,120696,105,120754,105,1110,105,42567,105,1231,105,43893,105,5029,105,71875,105,65354,106,8521,106,119843,106,119895,106,119947,106,119999,106,120051,106,120103,106,120155,106,120207,106,120259,106,120311,106,120363,106,120415,106,120467,106,1011,106,1112,106,65322,74,119817,74,119869,74,119921,74,119973,74,120025,74,120077,74,120129,74,120181,74,120233,74,120285,74,120337,74,120389,74,120441,74,42930,74,895,74,1032,74,5035,74,5261,74,42201,74,119844,107,119896,107,119948,107,120000,107,120052,107,120104,107,120156,107,120208,107,120260,107,120312,107,120364,107,120416,107,120468,107,8490,75,65323,75,119818,75,119870,75,119922,75,119974,75,120026,75,120078,75,120130,75,120182,75,120234,75,120286,75,120338,75,120390,75,120442,75,922,75,120497,75,120555,75,120613,75,120671,75,120729,75,11412,75,5094,75,5845,75,42199,75,66840,75,1472,108,8739,73,9213,73,65512,73,1633,108,1777,73,66336,108,125127,108,120783,73,120793,73,120803,73,120813,73,120823,73,130033,73,65321,73,8544,73,8464,73,8465,73,119816,73,119868,73,119920,73,120024,73,120128,73,120180,73,120232,73,120284,73,120336,73,120388,73,120440,73,65356,108,8572,73,8467,108,119845,108,119897,108,119949,108,120001,108,120053,108,120105,73,120157,73,120209,73,120261,73,120313,73,120365,73,120417,73,120469,73,448,73,120496,73,120554,73,120612,73,120670,73,120728,73,11410,73,1030,73,1216,73,1493,108,1503,108,1575,108,126464,108,126592,108,65166,108,65165,108,1994,108,11599,73,5825,73,42226,73,93992,73,66186,124,66313,124,119338,76,8556,76,8466,76,119819,76,119871,76,119923,76,120027,76,120079,76,120131,76,120183,76,120235,76,120287,76,120339,76,120391,76,120443,76,11472,76,5086,76,5290,76,42209,76,93974,76,71843,76,71858,76,66587,76,66854,76,65325,77,8559,77,8499,77,119820,77,119872,77,119924,77,120028,77,120080,77,120132,77,120184,77,120236,77,120288,77,120340,77,120392,77,120444,77,924,77,120499,77,120557,77,120615,77,120673,77,120731,77,1018,77,11416,77,5047,77,5616,77,5846,77,42207,77,66224,77,66321,77,119847,110,119899,110,119951,110,120003,110,120055,110,120107,110,120159,110,120211,110,120263,110,120315,110,120367,110,120419,110,120471,110,1400,110,1404,110,65326,78,8469,78,119821,78,119873,78,119925,78,119977,78,120029,78,120081,78,120185,78,120237,78,120289,78,120341,78,120393,78,120445,78,925,78,120500,78,120558,78,120616,78,120674,78,120732,78,11418,78,42208,78,66835,78,3074,111,3202,111,3330,111,3458,111,2406,111,2662,111,2790,111,3046,111,3174,111,3302,111,3430,111,3664,111,3792,111,4160,111,1637,111,1781,111,65359,111,8500,111,119848,111,119900,111,119952,111,120056,111,120108,111,120160,111,120212,111,120264,111,120316,111,120368,111,120420,111,120472,111,7439,111,7441,111,43837,111,959,111,120528,111,120586,111,120644,111,120702,111,120760,111,963,111,120532,111,120590,111,120648,111,120706,111,120764,111,11423,111,4351,111,1413,111,1505,111,1607,111,126500,111,126564,111,126596,111,65259,111,65260,111,65258,111,65257,111,1726,111,64428,111,64429,111,64427,111,64426,111,1729,111,64424,111,64425,111,64423,111,64422,111,1749,111,3360,111,4125,111,66794,111,71880,111,71895,111,66604,111,1984,79,2534,79,2918,79,12295,79,70864,79,71904,79,120782,79,120792,79,120802,79,120812,79,120822,79,130032,79,65327,79,119822,79,119874,79,119926,79,119978,79,120030,79,120082,79,120134,79,120186,79,120238,79,120290,79,120342,79,120394,79,120446,79,927,79,120502,79,120560,79,120618,79,120676,79,120734,79,11422,79,1365,79,11604,79,4816,79,2848,79,66754,79,42227,79,71861,79,66194,79,66219,79,66564,79,66838,79,9076,112,65360,112,119849,112,119901,112,119953,112,120005,112,120057,112,120109,112,120161,112,120213,112,120265,112,120317,112,120369,112,120421,112,120473,112,961,112,120530,112,120544,112,120588,112,120602,112,120646,112,120660,112,120704,112,120718,112,120762,112,120776,112,11427,112,65328,80,8473,80,119823,80,119875,80,119927,80,119979,80,120031,80,120083,80,120187,80,120239,80,120291,80,120343,80,120395,80,120447,80,929,80,120504,80,120562,80,120620,80,120678,80,120736,80,11426,80,5090,80,5229,80,42193,80,66197,80,119850,113,119902,113,119954,113,120006,113,120058,113,120110,113,120162,113,120214,113,120266,113,120318,113,120370,113,120422,113,120474,113,1307,113,1379,113,1382,113,8474,81,119824,81,119876,81,119928,81,119980,81,120032,81,120084,81,120188,81,120240,81,120292,81,120344,81,120396,81,120448,81,11605,81,119851,114,119903,114,119955,114,120007,114,120059,114,120111,114,120163,114,120215,114,120267,114,120319,114,120371,114,120423,114,120475,114,43847,114,43848,114,7462,114,11397,114,43905,114,119318,82,8475,82,8476,82,8477,82,119825,82,119877,82,119929,82,120033,82,120189,82,120241,82,120293,82,120345,82,120397,82,120449,82,422,82,5025,82,5074,82,66740,82,5511,82,42211,82,94005,82,65363,115,119852,115,119904,115,119956,115,120008,115,120060,115,120112,115,120164,115,120216,115,120268,115,120320,115,120372,115,120424,115,120476,115,42801,115,445,115,1109,115,43946,115,71873,115,66632,115,65331,83,119826,83,119878,83,119930,83,119982,83,120034,83,120086,83,120138,83,120190,83,120242,83,120294,83,120346,83,120398,83,120450,83,1029,83,1359,83,5077,83,5082,83,42210,83,94010,83,66198,83,66592,83,119853,116,119905,116,119957,116,120009,116,120061,116,120113,116,120165,116,120217,116,120269,116,120321,116,120373,116,120425,116,120477,116,8868,84,10201,84,128872,84,65332,84,119827,84,119879,84,119931,84,119983,84,120035,84,120087,84,120139,84,120191,84,120243,84,120295,84,120347,84,120399,84,120451,84,932,84,120507,84,120565,84,120623,84,120681,84,120739,84,11430,84,5026,84,42196,84,93962,84,71868,84,66199,84,66225,84,66325,84,119854,117,119906,117,119958,117,120010,117,120062,117,120114,117,120166,117,120218,117,120270,117,120322,117,120374,117,120426,117,120478,117,42911,117,7452,117,43854,117,43858,117,651,117,965,117,120534,117,120592,117,120650,117,120708,117,120766,117,1405,117,66806,117,71896,117,8746,85,8899,85,119828,85,119880,85,119932,85,119984,85,120036,85,120088,85,120140,85,120192,85,120244,85,120296,85,120348,85,120400,85,120452,85,1357,85,4608,85,66766,85,5196,85,42228,85,94018,85,71864,85,8744,118,8897,118,65366,118,8564,118,119855,118,119907,118,119959,118,120011,118,120063,118,120115,118,120167,118,120219,118,120271,118,120323,118,120375,118,120427,118,120479,118,7456,118,957,118,120526,118,120584,118,120642,118,120700,118,120758,118,1141,118,1496,118,71430,118,43945,118,71872,118,119309,86,1639,86,1783,86,8548,86,119829,86,119881,86,119933,86,119985,86,120037,86,120089,86,120141,86,120193,86,120245,86,120297,86,120349,86,120401,86,120453,86,1140,86,11576,86,5081,86,5167,86,42719,86,42214,86,93960,86,71840,86,66845,86,623,119,119856,119,119908,119,119960,119,120012,119,120064,119,120116,119,120168,119,120220,119,120272,119,120324,119,120376,119,120428,119,120480,119,7457,119,1121,119,1309,119,1377,119,71434,119,71438,119,71439,119,43907,119,71919,87,71910,87,119830,87,119882,87,119934,87,119986,87,120038,87,120090,87,120142,87,120194,87,120246,87,120298,87,120350,87,120402,87,120454,87,1308,87,5043,87,5076,87,42218,87,5742,120,10539,120,10540,120,10799,120,65368,120,8569,120,119857,120,119909,120,119961,120,120013,120,120065,120,120117,120,120169,120,120221,120,120273,120,120325,120,120377,120,120429,120,120481,120,5441,120,5501,120,5741,88,9587,88,66338,88,71916,88,65336,88,8553,88,119831,88,119883,88,119935,88,119987,88,120039,88,120091,88,120143,88,120195,88,120247,88,120299,88,120351,88,120403,88,120455,88,42931,88,935,88,120510,88,120568,88,120626,88,120684,88,120742,88,11436,88,11613,88,5815,88,42219,88,66192,88,66228,88,66327,88,66855,88,611,121,7564,121,65369,121,119858,121,119910,121,119962,121,120014,121,120066,121,120118,121,120170,121,120222,121,120274,121,120326,121,120378,121,120430,121,120482,121,655,121,7935,121,43866,121,947,121,8509,121,120516,121,120574,121,120632,121,120690,121,120748,121,1199,121,4327,121,71900,121,65337,89,119832,89,119884,89,119936,89,119988,89,120040,89,120092,89,120144,89,120196,89,120248,89,120300,89,120352,89,120404,89,120456,89,933,89,978,89,120508,89,120566,89,120624,89,120682,89,120740,89,11432,89,1198,89,5033,89,5053,89,42220,89,94019,89,71844,89,66226,89,119859,122,119911,122,119963,122,120015,122,120067,122,120119,122,120171,122,120223,122,120275,122,120327,122,120379,122,120431,122,120483,122,7458,122,43923,122,71876,122,66293,90,71909,90,65338,90,8484,90,8488,90,119833,90,119885,90,119937,90,119989,90,120041,90,120197,90,120249,90,120301,90,120353,90,120405,90,120457,90,918,90,120493,90,120551,90,120609,90,120667,90,120725,90,5059,90,42204,90,71849,90,65282,34,65284,36,65285,37,65286,38,65290,42,65291,43,65294,46,65295,47,65296,48,65297,49,65298,50,65299,51,65300,52,65301,53,65302,54,65303,55,65304,56,65305,57,65308,60,65309,61,65310,62,65312,64,65316,68,65318,70,65319,71,65324,76,65329,81,65330,82,65333,85,65334,86,65335,87,65343,95,65346,98,65348,100,65350,102,65355,107,65357,109,65358,110,65361,113,65362,114,65364,116,65365,117,65367,119,65370,122,65371,123,65373,125,119846,109],"_default":[160,32,8211,45,65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"cs":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"de":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"es":[8211,45,65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"fr":[65374,126,65306,58,65281,33,8216,96,8245,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"it":[160,32,8211,45,65374,126,65306,58,65281,33,8216,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"ja":[8211,45,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65292,44,65307,59],"ko":[8211,45,65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"pl":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"pt-BR":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"qps-ploc":[160,32,8211,45,65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"ru":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,305,105,921,73,1009,112,215,120,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"tr":[160,32,8211,45,65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"zh-hans":[65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65288,40,65289,41],"zh-hant":[8211,45,65374,126,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65307,59]}')), ve.cache = new Rl({ getCacheKey: JSON.stringify }, (t) => {
  function n(h) {
    const f = /* @__PURE__ */ new Map();
    for (let g = 0; g < h.length; g += 2)
      f.set(h[g], h[g + 1]);
    return f;
  }
  function i(h, f) {
    const g = new Map(h);
    for (const [b, L] of f)
      g.set(b, L);
    return g;
  }
  function s(h, f) {
    if (!h)
      return f;
    const g = /* @__PURE__ */ new Map();
    for (const [b, L] of h)
      f.has(b) && g.set(b, L);
    return g;
  }
  const r = ve.ambiguousCharacterData.value;
  let a = t.filter((h) => !h.startsWith("_") && h in r);
  a.length === 0 && (a = ["_default"]);
  let u;
  for (const h of a) {
    const f = n(r[h]);
    u = s(u, f);
  }
  const o = n(r._common), c = i(o, u);
  return new ve(c);
}), ve._locales = new si(() => Object.keys(ve.ambiguousCharacterData.value).filter((t) => !t.startsWith("_")));
let Ft = ve;
const $e = class $e {
  static getRawData() {
    return JSON.parse("[9,10,11,12,13,32,127,160,173,847,1564,4447,4448,6068,6069,6155,6156,6157,6158,7355,7356,8192,8193,8194,8195,8196,8197,8198,8199,8200,8201,8202,8203,8204,8205,8206,8207,8234,8235,8236,8237,8238,8239,8287,8288,8289,8290,8291,8292,8293,8294,8295,8296,8297,8298,8299,8300,8301,8302,8303,10240,12288,12644,65024,65025,65026,65027,65028,65029,65030,65031,65032,65033,65034,65035,65036,65037,65038,65039,65279,65440,65520,65521,65522,65523,65524,65525,65526,65527,65528,65532,78844,119155,119156,119157,119158,119159,119160,119161,119162,917504,917505,917506,917507,917508,917509,917510,917511,917512,917513,917514,917515,917516,917517,917518,917519,917520,917521,917522,917523,917524,917525,917526,917527,917528,917529,917530,917531,917532,917533,917534,917535,917536,917537,917538,917539,917540,917541,917542,917543,917544,917545,917546,917547,917548,917549,917550,917551,917552,917553,917554,917555,917556,917557,917558,917559,917560,917561,917562,917563,917564,917565,917566,917567,917568,917569,917570,917571,917572,917573,917574,917575,917576,917577,917578,917579,917580,917581,917582,917583,917584,917585,917586,917587,917588,917589,917590,917591,917592,917593,917594,917595,917596,917597,917598,917599,917600,917601,917602,917603,917604,917605,917606,917607,917608,917609,917610,917611,917612,917613,917614,917615,917616,917617,917618,917619,917620,917621,917622,917623,917624,917625,917626,917627,917628,917629,917630,917631,917760,917761,917762,917763,917764,917765,917766,917767,917768,917769,917770,917771,917772,917773,917774,917775,917776,917777,917778,917779,917780,917781,917782,917783,917784,917785,917786,917787,917788,917789,917790,917791,917792,917793,917794,917795,917796,917797,917798,917799,917800,917801,917802,917803,917804,917805,917806,917807,917808,917809,917810,917811,917812,917813,917814,917815,917816,917817,917818,917819,917820,917821,917822,917823,917824,917825,917826,917827,917828,917829,917830,917831,917832,917833,917834,917835,917836,917837,917838,917839,917840,917841,917842,917843,917844,917845,917846,917847,917848,917849,917850,917851,917852,917853,917854,917855,917856,917857,917858,917859,917860,917861,917862,917863,917864,917865,917866,917867,917868,917869,917870,917871,917872,917873,917874,917875,917876,917877,917878,917879,917880,917881,917882,917883,917884,917885,917886,917887,917888,917889,917890,917891,917892,917893,917894,917895,917896,917897,917898,917899,917900,917901,917902,917903,917904,917905,917906,917907,917908,917909,917910,917911,917912,917913,917914,917915,917916,917917,917918,917919,917920,917921,917922,917923,917924,917925,917926,917927,917928,917929,917930,917931,917932,917933,917934,917935,917936,917937,917938,917939,917940,917941,917942,917943,917944,917945,917946,917947,917948,917949,917950,917951,917952,917953,917954,917955,917956,917957,917958,917959,917960,917961,917962,917963,917964,917965,917966,917967,917968,917969,917970,917971,917972,917973,917974,917975,917976,917977,917978,917979,917980,917981,917982,917983,917984,917985,917986,917987,917988,917989,917990,917991,917992,917993,917994,917995,917996,917997,917998,917999]");
  }
  static getData() {
    return this._data || (this._data = new Set($e.getRawData())), this._data;
  }
  static isInvisibleCharacter(t) {
    return $e.getData().has(t);
  }
  static containsInvisibleCharacter(t) {
    for (let n = 0; n < t.length; n++) {
      const i = t.codePointAt(n);
      if (typeof i == "number" && $e.isInvisibleCharacter(i))
        return !0;
    }
    return !1;
  }
  static get codePoints() {
    return $e.getData();
  }
};
$e._data = void 0;
let Et = $e;
const Il = "$initialize";
var he;
(function(e) {
  e[e.Request = 0] = "Request", e[e.Reply = 1] = "Reply", e[e.SubscribeEvent = 2] = "SubscribeEvent", e[e.Event = 3] = "Event", e[e.UnsubscribeEvent = 4] = "UnsubscribeEvent";
})(he || (he = {}));
class Sl {
  constructor(t, n, i, s) {
    this.vsWorker = t, this.req = n, this.method = i, this.args = s, this.type = he.Request;
  }
}
class ui {
  constructor(t, n, i, s) {
    this.vsWorker = t, this.seq = n, this.res = i, this.err = s, this.type = he.Reply;
  }
}
class Bl {
  constructor(t, n, i, s) {
    this.vsWorker = t, this.req = n, this.eventName = i, this.arg = s, this.type = he.SubscribeEvent;
  }
}
class Pl {
  constructor(t, n, i) {
    this.vsWorker = t, this.req = n, this.event = i, this.type = he.Event;
  }
}
class yl {
  constructor(t, n) {
    this.vsWorker = t, this.req = n, this.type = he.UnsubscribeEvent;
  }
}
class Ol {
  constructor(t) {
    this._workerId = -1, this._handler = t, this._lastSentReq = 0, this._pendingReplies = /* @__PURE__ */ Object.create(null), this._pendingEmitters = /* @__PURE__ */ new Map(), this._pendingEvents = /* @__PURE__ */ new Map();
  }
  setWorkerId(t) {
    this._workerId = t;
  }
  sendMessage(t, n) {
    const i = String(++this._lastSentReq);
    return new Promise((s, r) => {
      this._pendingReplies[i] = {
        resolve: s,
        reject: r
      }, this._send(new Sl(this._workerId, i, t, n));
    });
  }
  listen(t, n) {
    let i = null;
    const s = new Ae({
      onWillAddFirstListener: () => {
        i = String(++this._lastSentReq), this._pendingEmitters.set(i, s), this._send(new Bl(this._workerId, i, t, n));
      },
      onDidRemoveLastListener: () => {
        this._pendingEmitters.delete(i), this._send(new yl(this._workerId, i)), i = null;
      }
    });
    return s.event;
  }
  handleMessage(t) {
    !t || !t.vsWorker || this._workerId !== -1 && t.vsWorker !== this._workerId || this._handleMessage(t);
  }
  _handleMessage(t) {
    switch (t.type) {
      case he.Reply:
        return this._handleReplyMessage(t);
      case he.Request:
        return this._handleRequestMessage(t);
      case he.SubscribeEvent:
        return this._handleSubscribeEventMessage(t);
      case he.Event:
        return this._handleEventMessage(t);
      case he.UnsubscribeEvent:
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
      let i = t.err;
      t.err.$isError && (i = new Error(), i.name = t.err.name, i.message = t.err.message, i.stack = t.err.stack), n.reject(i);
      return;
    }
    n.resolve(t.res);
  }
  _handleRequestMessage(t) {
    const n = t.req;
    this._handler.handleMessage(t.method, t.args).then((s) => {
      this._send(new ui(this._workerId, n, s, void 0));
    }, (s) => {
      s.detail instanceof Error && (s.detail = Zn(s.detail)), this._send(new ui(this._workerId, n, void 0, Zn(s)));
    });
  }
  _handleSubscribeEventMessage(t) {
    const n = t.req, i = this._handler.handleEvent(t.eventName, t.arg)((s) => {
      this._send(new Pl(this._workerId, n, s));
    });
    this._pendingEvents.set(n, i);
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
    if (t.type === he.Request)
      for (let i = 0; i < t.args.length; i++)
        t.args[i] instanceof ArrayBuffer && n.push(t.args[i]);
    else t.type === he.Reply && t.res instanceof ArrayBuffer && n.push(t.res);
    this._handler.sendMessage(t, n);
  }
}
function Er(e) {
  return e[0] === "o" && e[1] === "n" && pr(e.charCodeAt(2));
}
function Ar(e) {
  return /^onDynamic/.test(e) && pr(e.charCodeAt(9));
}
function Vl(e, t, n) {
  const i = (a) => function() {
    const u = Array.prototype.slice.call(arguments, 0);
    return t(a, u);
  }, s = (a) => function(u) {
    return n(a, u);
  }, r = {};
  for (const a of e) {
    if (Ar(a)) {
      r[a] = s(a);
      continue;
    }
    if (Er(a)) {
      r[a] = n(a, void 0);
      continue;
    }
    r[a] = i(a);
  }
  return r;
}
class Hl {
  constructor(t, n) {
    this._requestHandlerFactory = n, this._requestHandler = null, this._protocol = new Ol({
      sendMessage: (i, s) => {
        t(i, s);
      },
      handleMessage: (i, s) => this._handleMessage(i, s),
      handleEvent: (i, s) => this._handleEvent(i, s)
    });
  }
  onmessage(t) {
    this._protocol.handleMessage(t);
  }
  _handleMessage(t, n) {
    if (t === Il)
      return this.initialize(n[0], n[1], n[2], n[3]);
    if (!this._requestHandler || typeof this._requestHandler[t] != "function")
      return Promise.reject(new Error("Missing requestHandler or method: " + t));
    try {
      return Promise.resolve(this._requestHandler[t].apply(this._requestHandler, n));
    } catch (i) {
      return Promise.reject(i);
    }
  }
  _handleEvent(t, n) {
    if (!this._requestHandler)
      throw new Error("Missing requestHandler");
    if (Ar(t)) {
      const i = this._requestHandler[t].call(this._requestHandler, n);
      if (typeof i != "function")
        throw new Error(`Missing dynamic event ${t} on request handler.`);
      return i;
    }
    if (Er(t)) {
      const i = this._requestHandler[t];
      if (typeof i != "function")
        throw new Error(`Missing event ${t} on request handler.`);
      return i;
    }
    throw new Error(`Malformed event name ${t}`);
  }
  initialize(t, n, i, s) {
    this._protocol.setWorkerId(t);
    const u = Vl(s, (o, c) => this._protocol.sendMessage(o, c), (o, c) => this._protocol.listen(o, c));
    return this._requestHandlerFactory ? (this._requestHandler = this._requestHandlerFactory(u), Promise.resolve(An(this._requestHandler))) : (n && (typeof n.baseUrl < "u" && delete n.baseUrl, typeof n.paths < "u" && typeof n.paths.vs < "u" && delete n.paths.vs, typeof n.trustedTypesPolicy < "u" && delete n.trustedTypesPolicy, n.catchError = !0, globalThis.require.config(n)), new Promise((o, c) => {
      (void 0)([i], (f) => {
        if (this._requestHandler = f.create(u), !this._requestHandler) {
          c(new Error("No RequestHandler!"));
          return;
        }
        o(An(this._requestHandler));
      }, c);
    }));
  }
}
class Pe {
  constructor(t, n, i, s) {
    this.originalStart = t, this.originalLength = n, this.modifiedStart = i, this.modifiedLength = s;
  }
  getOriginalEnd() {
    return this.originalStart + this.originalLength;
  }
  getModifiedEnd() {
    return this.modifiedStart + this.modifiedLength;
  }
}
function oi(e, t) {
  return (t << 5) - t + e | 0;
}
function ql(e, t) {
  t = oi(149417, t);
  for (let n = 0, i = e.length; n < i; n++)
    t = oi(e.charCodeAt(n), t);
  return t;
}
var be;
(function(e) {
  e[e.BLOCK_SIZE = 64] = "BLOCK_SIZE", e[e.UNICODE_REPLACEMENT = 65533] = "UNICODE_REPLACEMENT";
})(be || (be = {}));
function fn(e, t, n = 32) {
  const i = n - t, s = ~((1 << i) - 1);
  return (e << t | (s & e) >>> i) >>> 0;
}
function ci(e, t = 0, n = e.byteLength, i = 0) {
  for (let s = 0; s < n; s++)
    e[t + s] = i;
}
function Wl(e, t, n = "0") {
  for (; e.length < t; )
    e = n + e;
  return e;
}
function _t(e, t = 32) {
  return e instanceof ArrayBuffer ? Array.from(new Uint8Array(e)).map((n) => n.toString(16).padStart(2, "0")).join("") : Wl((e >>> 0).toString(16), t / 4);
}
const ln = class ln {
  constructor() {
    this._h0 = 1732584193, this._h1 = 4023233417, this._h2 = 2562383102, this._h3 = 271733878, this._h4 = 3285377520, this._buff = new Uint8Array(be.BLOCK_SIZE + 3), this._buffDV = new DataView(this._buff.buffer), this._buffLen = 0, this._totalLen = 0, this._leftoverHighSurrogate = 0, this._finished = !1;
  }
  update(t) {
    const n = t.length;
    if (n === 0)
      return;
    const i = this._buff;
    let s = this._buffLen, r = this._leftoverHighSurrogate, a, u;
    for (r !== 0 ? (a = r, u = -1, r = 0) : (a = t.charCodeAt(0), u = 0); ; ) {
      let o = a;
      if (Xt(a))
        if (u + 1 < n) {
          const c = t.charCodeAt(u + 1);
          xn(c) ? (u++, o = wr(a, c)) : o = be.UNICODE_REPLACEMENT;
        } else {
          r = a;
          break;
        }
      else xn(a) && (o = be.UNICODE_REPLACEMENT);
      if (s = this._push(i, s, o), u++, u < n)
        a = t.charCodeAt(u);
      else
        break;
    }
    this._buffLen = s, this._leftoverHighSurrogate = r;
  }
  _push(t, n, i) {
    return i < 128 ? t[n++] = i : i < 2048 ? (t[n++] = 192 | (i & 1984) >>> 6, t[n++] = 128 | (i & 63) >>> 0) : i < 65536 ? (t[n++] = 224 | (i & 61440) >>> 12, t[n++] = 128 | (i & 4032) >>> 6, t[n++] = 128 | (i & 63) >>> 0) : (t[n++] = 240 | (i & 1835008) >>> 18, t[n++] = 128 | (i & 258048) >>> 12, t[n++] = 128 | (i & 4032) >>> 6, t[n++] = 128 | (i & 63) >>> 0), n >= be.BLOCK_SIZE && (this._step(), n -= be.BLOCK_SIZE, this._totalLen += be.BLOCK_SIZE, t[0] = t[be.BLOCK_SIZE + 0], t[1] = t[be.BLOCK_SIZE + 1], t[2] = t[be.BLOCK_SIZE + 2]), n;
  }
  digest() {
    return this._finished || (this._finished = !0, this._leftoverHighSurrogate && (this._leftoverHighSurrogate = 0, this._buffLen = this._push(this._buff, this._buffLen, be.UNICODE_REPLACEMENT)), this._totalLen += this._buffLen, this._wrapUp()), _t(this._h0) + _t(this._h1) + _t(this._h2) + _t(this._h3) + _t(this._h4);
  }
  _wrapUp() {
    this._buff[this._buffLen++] = 128, ci(this._buff, this._buffLen), this._buffLen > 56 && (this._step(), ci(this._buff));
    const t = 8 * this._totalLen;
    this._buffDV.setUint32(56, Math.floor(t / 4294967296), !1), this._buffDV.setUint32(60, t % 4294967296, !1), this._step();
  }
  _step() {
    const t = ln._bigBlock32, n = this._buffDV;
    for (let f = 0; f < 64; f += 4)
      t.setUint32(f, n.getUint32(f, !1), !1);
    for (let f = 64; f < 320; f += 4)
      t.setUint32(f, fn(t.getUint32(f - 12, !1) ^ t.getUint32(f - 32, !1) ^ t.getUint32(f - 56, !1) ^ t.getUint32(f - 64, !1), 1), !1);
    let i = this._h0, s = this._h1, r = this._h2, a = this._h3, u = this._h4, o, c, h;
    for (let f = 0; f < 80; f++)
      f < 20 ? (o = s & r | ~s & a, c = 1518500249) : f < 40 ? (o = s ^ r ^ a, c = 1859775393) : f < 60 ? (o = s & r | s & a | r & a, c = 2400959708) : (o = s ^ r ^ a, c = 3395469782), h = fn(i, 5) + o + u + c + t.getUint32(f * 4, !1) & 4294967295, u = a, a = r, r = fn(s, 30), s = i, i = h;
    this._h0 = this._h0 + i & 4294967295, this._h1 = this._h1 + s & 4294967295, this._h2 = this._h2 + r & 4294967295, this._h3 = this._h3 + a & 4294967295, this._h4 = this._h4 + u & 4294967295;
  }
};
ln._bigBlock32 = new DataView(new ArrayBuffer(320));
let fi = ln;
class hi {
  constructor(t) {
    this.source = t;
  }
  getElements() {
    const t = this.source, n = new Int32Array(t.length);
    for (let i = 0, s = t.length; i < s; i++)
      n[i] = t.charCodeAt(i);
    return n;
  }
}
function Gl(e, t, n) {
  return new Ve(new hi(e), new hi(t)).ComputeDiff(n).changes;
}
class Je {
  static Assert(t, n) {
    if (!t)
      throw new Error(n);
  }
}
class Ke {
  static Copy(t, n, i, s, r) {
    for (let a = 0; a < r; a++)
      i[s + a] = t[n + a];
  }
  static Copy2(t, n, i, s, r) {
    for (let a = 0; a < r; a++)
      i[s + a] = t[n + a];
  }
}
var Te;
(function(e) {
  e[e.MaxDifferencesHistory = 1447] = "MaxDifferencesHistory";
})(Te || (Te = {}));
class mi {
  constructor() {
    this.m_changes = [], this.m_originalStart = pe.MAX_SAFE_SMALL_INTEGER, this.m_modifiedStart = pe.MAX_SAFE_SMALL_INTEGER, this.m_originalCount = 0, this.m_modifiedCount = 0;
  }
  MarkNextChange() {
    (this.m_originalCount > 0 || this.m_modifiedCount > 0) && this.m_changes.push(new Pe(
      this.m_originalStart,
      this.m_originalCount,
      this.m_modifiedStart,
      this.m_modifiedCount
    )), this.m_originalCount = 0, this.m_modifiedCount = 0, this.m_originalStart = pe.MAX_SAFE_SMALL_INTEGER, this.m_modifiedStart = pe.MAX_SAFE_SMALL_INTEGER;
  }
  AddOriginalElement(t, n) {
    this.m_originalStart = Math.min(this.m_originalStart, t), this.m_modifiedStart = Math.min(this.m_modifiedStart, n), this.m_originalCount++;
  }
  AddModifiedElement(t, n) {
    this.m_originalStart = Math.min(this.m_originalStart, t), this.m_modifiedStart = Math.min(this.m_modifiedStart, n), this.m_modifiedCount++;
  }
  getChanges() {
    return (this.m_originalCount > 0 || this.m_modifiedCount > 0) && this.MarkNextChange(), this.m_changes;
  }
  getReverseChanges() {
    return (this.m_originalCount > 0 || this.m_modifiedCount > 0) && this.MarkNextChange(), this.m_changes.reverse(), this.m_changes;
  }
}
class Ve {
  constructor(t, n, i = null) {
    this.ContinueProcessingPredicate = i, this._originalSequence = t, this._modifiedSequence = n;
    const [s, r, a] = Ve._getElements(t), [u, o, c] = Ve._getElements(n);
    this._hasStrings = a && c, this._originalStringElements = s, this._originalElementsOrHash = r, this._modifiedStringElements = u, this._modifiedElementsOrHash = o, this.m_forwardHistory = [], this.m_reverseHistory = [];
  }
  static _isStringArray(t) {
    return t.length > 0 && typeof t[0] == "string";
  }
  static _getElements(t) {
    const n = t.getElements();
    if (Ve._isStringArray(n)) {
      const i = new Int32Array(n.length);
      for (let s = 0, r = n.length; s < r; s++)
        i[s] = ql(n[s], 0);
      return [n, i, !0];
    }
    return n instanceof Int32Array ? [[], n, !1] : [[], new Int32Array(n), !1];
  }
  ElementsAreEqual(t, n) {
    return this._originalElementsOrHash[t] !== this._modifiedElementsOrHash[n] ? !1 : this._hasStrings ? this._originalStringElements[t] === this._modifiedStringElements[n] : !0;
  }
  ElementsAreStrictEqual(t, n) {
    if (!this.ElementsAreEqual(t, n))
      return !1;
    const i = Ve._getStrictElement(this._originalSequence, t), s = Ve._getStrictElement(this._modifiedSequence, n);
    return i === s;
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
  _ComputeDiff(t, n, i, s, r) {
    const a = [!1];
    let u = this.ComputeDiffRecursive(t, n, i, s, a);
    return r && (u = this.PrettifyChanges(u)), {
      quitEarly: a[0],
      changes: u
    };
  }
  ComputeDiffRecursive(t, n, i, s, r) {
    for (r[0] = !1; t <= n && i <= s && this.ElementsAreEqual(t, i); )
      t++, i++;
    for (; n >= t && s >= i && this.ElementsAreEqual(n, s); )
      n--, s--;
    if (t > n || i > s) {
      let f;
      return i <= s ? (Je.Assert(t === n + 1, "originalStart should only be one more than originalEnd"), f = [
        new Pe(t, 0, i, s - i + 1)
      ]) : t <= n ? (Je.Assert(i === s + 1, "modifiedStart should only be one more than modifiedEnd"), f = [
        new Pe(t, n - t + 1, i, 0)
      ]) : (Je.Assert(t === n + 1, "originalStart should only be one more than originalEnd"), Je.Assert(i === s + 1, "modifiedStart should only be one more than modifiedEnd"), f = []), f;
    }
    const a = [0], u = [0], o = this.ComputeRecursionPoint(t, n, i, s, a, u, r), c = a[0], h = u[0];
    if (o !== null)
      return o;
    if (!r[0]) {
      const f = this.ComputeDiffRecursive(t, c, i, h, r);
      let g = [];
      return r[0] ? g = [
        new Pe(
          c + 1,
          n - (c + 1) + 1,
          h + 1,
          s - (h + 1) + 1
        )
      ] : g = this.ComputeDiffRecursive(c + 1, n, h + 1, s, r), this.ConcatenateChanges(f, g);
    }
    return [
      new Pe(
        t,
        n - t + 1,
        i,
        s - i + 1
      )
    ];
  }
  WALKTRACE(t, n, i, s, r, a, u, o, c, h, f, g, b, L, N, A, x, R) {
    let U = null, p = null, w = new mi(), D = n, T = i, I = b[0] - A[0] - s, $ = pe.MIN_SAFE_SMALL_INTEGER, ne = this.m_forwardHistory.length - 1;
    do {
      const z = I + t;
      z === D || z < T && c[z - 1] < c[z + 1] ? (f = c[z + 1], L = f - I - s, f < $ && w.MarkNextChange(), $ = f, w.AddModifiedElement(f + 1, L), I = z + 1 - t) : (f = c[z - 1] + 1, L = f - I - s, f < $ && w.MarkNextChange(), $ = f - 1, w.AddOriginalElement(f, L + 1), I = z - 1 - t), ne >= 0 && (c = this.m_forwardHistory[ne], t = c[0], D = 1, T = c.length - 1);
    } while (--ne >= -1);
    if (U = w.getReverseChanges(), R[0]) {
      let z = b[0] + 1, E = A[0] + 1;
      if (U !== null && U.length > 0) {
        const v = U[U.length - 1];
        z = Math.max(z, v.getOriginalEnd()), E = Math.max(E, v.getModifiedEnd());
      }
      p = [
        new Pe(
          z,
          g - z + 1,
          E,
          N - E + 1
        )
      ];
    } else {
      w = new mi(), D = a, T = u, I = b[0] - A[0] - o, $ = pe.MAX_SAFE_SMALL_INTEGER, ne = x ? this.m_reverseHistory.length - 1 : this.m_reverseHistory.length - 2;
      do {
        const z = I + r;
        z === D || z < T && h[z - 1] >= h[z + 1] ? (f = h[z + 1] - 1, L = f - I - o, f > $ && w.MarkNextChange(), $ = f + 1, w.AddOriginalElement(f + 1, L + 1), I = z + 1 - r) : (f = h[z - 1], L = f - I - o, f > $ && w.MarkNextChange(), $ = f, w.AddModifiedElement(f + 1, L + 1), I = z - 1 - r), ne >= 0 && (h = this.m_reverseHistory[ne], r = h[0], D = 1, T = h.length - 1);
      } while (--ne >= -1);
      p = w.getChanges();
    }
    return this.ConcatenateChanges(U, p);
  }
  ComputeRecursionPoint(t, n, i, s, r, a, u) {
    let o = 0, c = 0, h = 0, f = 0, g = 0, b = 0;
    t--, i--, r[0] = 0, a[0] = 0, this.m_forwardHistory = [], this.m_reverseHistory = [];
    const L = n - t + (s - i), N = L + 1, A = new Int32Array(N), x = new Int32Array(N), R = s - i, U = n - t, p = t - i, w = n - s, T = (U - R) % 2 === 0;
    A[R] = t, x[U] = n, u[0] = !1;
    for (let I = 1; I <= L / 2 + 1; I++) {
      let $ = 0, ne = 0;
      h = this.ClipDiagonalBound(R - I, I, R, N), f = this.ClipDiagonalBound(R + I, I, R, N);
      for (let E = h; E <= f; E += 2) {
        E === h || E < f && A[E - 1] < A[E + 1] ? o = A[E + 1] : o = A[E - 1] + 1, c = o - (E - R) - p;
        const v = o;
        for (; o < n && c < s && this.ElementsAreEqual(o + 1, c + 1); )
          o++, c++;
        if (A[E] = o, o + c > $ + ne && ($ = o, ne = c), !T && Math.abs(E - U) <= I - 1 && o >= x[E])
          return r[0] = o, a[0] = c, v <= x[E] && Te.MaxDifferencesHistory > 0 && I <= Te.MaxDifferencesHistory + 1 ? this.WALKTRACE(R, h, f, p, U, g, b, w, A, x, o, n, r, c, s, a, T, u) : null;
      }
      const z = ($ - t + (ne - i) - I) / 2;
      if (this.ContinueProcessingPredicate !== null && !this.ContinueProcessingPredicate($, z))
        return u[0] = !0, r[0] = $, a[0] = ne, z > 0 && Te.MaxDifferencesHistory > 0 && I <= Te.MaxDifferencesHistory + 1 ? this.WALKTRACE(R, h, f, p, U, g, b, w, A, x, o, n, r, c, s, a, T, u) : (t++, i++, [
          new Pe(
            t,
            n - t + 1,
            i,
            s - i + 1
          )
        ]);
      g = this.ClipDiagonalBound(U - I, I, U, N), b = this.ClipDiagonalBound(U + I, I, U, N);
      for (let E = g; E <= b; E += 2) {
        E === g || E < b && x[E - 1] >= x[E + 1] ? o = x[E + 1] - 1 : o = x[E - 1], c = o - (E - U) - w;
        const v = o;
        for (; o > t && c > i && this.ElementsAreEqual(o, c); )
          o--, c--;
        if (x[E] = o, T && Math.abs(E - R) <= I && o <= A[E])
          return r[0] = o, a[0] = c, v >= A[E] && Te.MaxDifferencesHistory > 0 && I <= Te.MaxDifferencesHistory + 1 ? this.WALKTRACE(R, h, f, p, U, g, b, w, A, x, o, n, r, c, s, a, T, u) : null;
      }
      if (I <= Te.MaxDifferencesHistory) {
        let E = new Int32Array(f - h + 2);
        E[0] = R - h + 1, Ke.Copy2(A, h, E, 1, f - h + 1), this.m_forwardHistory.push(E), E = new Int32Array(b - g + 2), E[0] = U - g + 1, Ke.Copy2(x, g, E, 1, b - g + 1), this.m_reverseHistory.push(E);
      }
    }
    return this.WALKTRACE(R, h, f, p, U, g, b, w, A, x, o, n, r, c, s, a, T, u);
  }
  PrettifyChanges(t) {
    for (let n = 0; n < t.length; n++) {
      const i = t[n], s = n < t.length - 1 ? t[n + 1].originalStart : this._originalElementsOrHash.length, r = n < t.length - 1 ? t[n + 1].modifiedStart : this._modifiedElementsOrHash.length, a = i.originalLength > 0, u = i.modifiedLength > 0;
      for (; i.originalStart + i.originalLength < s && i.modifiedStart + i.modifiedLength < r && (!a || this.OriginalElementsAreEqual(i.originalStart, i.originalStart + i.originalLength)) && (!u || this.ModifiedElementsAreEqual(i.modifiedStart, i.modifiedStart + i.modifiedLength)); ) {
        const c = this.ElementsAreStrictEqual(i.originalStart, i.modifiedStart);
        if (this.ElementsAreStrictEqual(i.originalStart + i.originalLength, i.modifiedStart + i.modifiedLength) && !c)
          break;
        i.originalStart++, i.modifiedStart++;
      }
      const o = [null];
      if (n < t.length - 1 && this.ChangesOverlap(t[n], t[n + 1], o)) {
        t[n] = o[0], t.splice(n + 1, 1), n--;
        continue;
      }
    }
    for (let n = t.length - 1; n >= 0; n--) {
      const i = t[n];
      let s = 0, r = 0;
      if (n > 0) {
        const f = t[n - 1];
        s = f.originalStart + f.originalLength, r = f.modifiedStart + f.modifiedLength;
      }
      const a = i.originalLength > 0, u = i.modifiedLength > 0;
      let o = 0, c = this._boundaryScore(i.originalStart, i.originalLength, i.modifiedStart, i.modifiedLength);
      for (let f = 1; ; f++) {
        const g = i.originalStart - f, b = i.modifiedStart - f;
        if (g < s || b < r || a && !this.OriginalElementsAreEqual(g, g + i.originalLength) || u && !this.ModifiedElementsAreEqual(b, b + i.modifiedLength))
          break;
        const N = (g === s && b === r ? 5 : 0) + this._boundaryScore(g, i.originalLength, b, i.modifiedLength);
        N > c && (c = N, o = f);
      }
      i.originalStart -= o, i.modifiedStart -= o;
      const h = [null];
      if (n > 0 && this.ChangesOverlap(t[n - 1], t[n], h)) {
        t[n - 1] = h[0], t.splice(n, 1), n++;
        continue;
      }
    }
    if (this._hasStrings)
      for (let n = 1, i = t.length; n < i; n++) {
        const s = t[n - 1], r = t[n], a = r.originalStart - s.originalStart - s.originalLength, u = s.originalStart, o = r.originalStart + r.originalLength, c = o - u, h = s.modifiedStart, f = r.modifiedStart + r.modifiedLength, g = f - h;
        if (a < 5 && c < 20 && g < 20) {
          const b = this._findBetterContiguousSequence(u, c, h, g, a);
          if (b) {
            const [L, N] = b;
            (L !== s.originalStart + s.originalLength || N !== s.modifiedStart + s.modifiedLength) && (s.originalLength = L - s.originalStart, s.modifiedLength = N - s.modifiedStart, r.originalStart = L + a, r.modifiedStart = N + a, r.originalLength = o - r.originalStart, r.modifiedLength = f - r.modifiedStart);
          }
        }
      }
    return t;
  }
  _findBetterContiguousSequence(t, n, i, s, r) {
    if (n < r || s < r)
      return null;
    const a = t + n - r + 1, u = i + s - r + 1;
    let o = 0, c = 0, h = 0;
    for (let f = t; f < a; f++)
      for (let g = i; g < u; g++) {
        const b = this._contiguousSequenceScore(f, g, r);
        b > 0 && b > o && (o = b, c = f, h = g);
      }
    return o > 0 ? [c, h] : null;
  }
  _contiguousSequenceScore(t, n, i) {
    let s = 0;
    for (let r = 0; r < i; r++) {
      if (!this.ElementsAreEqual(t + r, n + r))
        return 0;
      s += this._originalStringElements[t + r].length;
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
      const i = t + n;
      if (this._OriginalIsBoundary(i - 1) || this._OriginalIsBoundary(i))
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
      const i = t + n;
      if (this._ModifiedIsBoundary(i - 1) || this._ModifiedIsBoundary(i))
        return !0;
    }
    return !1;
  }
  _boundaryScore(t, n, i, s) {
    const r = this._OriginalRegionIsBoundary(t, n) ? 1 : 0, a = this._ModifiedRegionIsBoundary(i, s) ? 1 : 0;
    return r + a;
  }
  ConcatenateChanges(t, n) {
    const i = [];
    if (t.length === 0 || n.length === 0)
      return n.length > 0 ? n : t;
    if (this.ChangesOverlap(t[t.length - 1], n[0], i)) {
      const s = new Array(t.length + n.length - 1);
      return Ke.Copy(t, 0, s, 0, t.length - 1), s[t.length - 1] = i[0], Ke.Copy(n, 1, s, t.length, n.length - 1), s;
    } else {
      const s = new Array(t.length + n.length);
      return Ke.Copy(t, 0, s, 0, t.length), Ke.Copy(n, 0, s, t.length, n.length), s;
    }
  }
  ChangesOverlap(t, n, i) {
    if (Je.Assert(t.originalStart <= n.originalStart, "Left change is not less than or equal to right change"), Je.Assert(t.modifiedStart <= n.modifiedStart, "Left change is not less than or equal to right change"), t.originalStart + t.originalLength >= n.originalStart || t.modifiedStart + t.modifiedLength >= n.modifiedStart) {
      const s = t.originalStart;
      let r = t.originalLength;
      const a = t.modifiedStart;
      let u = t.modifiedLength;
      return t.originalStart + t.originalLength >= n.originalStart && (r = n.originalStart + n.originalLength - t.originalStart), t.modifiedStart + t.modifiedLength >= n.modifiedStart && (u = n.modifiedStart + n.modifiedLength - t.modifiedStart), i[0] = new Pe(s, r, a, u), !0;
    } else
      return i[0] = null, !1;
  }
  ClipDiagonalBound(t, n, i, s) {
    if (t >= 0 && t < s)
      return t;
    const r = i, a = s - i - 1, u = n % 2 === 0;
    if (t < 0) {
      const o = r % 2 === 0;
      return u === o ? 0 : 1;
    } else {
      const o = a % 2 === 0;
      return u === o ? s - 1 : s - 2;
    }
  }
}
var Rn;
(function(e) {
  e[e.Uri = 1] = "Uri", e[e.Regexp = 2] = "Regexp", e[e.ScmResource = 3] = "ScmResource", e[e.ScmResourceGroup = 4] = "ScmResourceGroup", e[e.ScmProvider = 5] = "ScmProvider", e[e.CommentController = 6] = "CommentController", e[e.CommentThread = 7] = "CommentThread", e[e.CommentThreadInstance = 8] = "CommentThreadInstance", e[e.CommentThreadReply = 9] = "CommentThreadReply", e[e.CommentNode = 10] = "CommentNode", e[e.CommentThreadNode = 11] = "CommentThreadNode", e[e.TimelineActionContext = 12] = "TimelineActionContext", e[e.NotebookCellActionContext = 13] = "NotebookCellActionContext", e[e.NotebookActionContext = 14] = "NotebookActionContext", e[e.TerminalContext = 15] = "TerminalContext", e[e.TestItemContext = 16] = "TestItemContext", e[e.Date = 17] = "Date", e[e.TestMessageMenuArgs = 18] = "TestMessageMenuArgs";
})(Rn || (Rn = {}));
let Xe;
const hn = globalThis.vscode;
var _r;
if (typeof hn < "u" && typeof hn.process < "u") {
  const e = hn.process;
  Xe = {
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
} else typeof process < "u" && typeof ((_r = process == null ? void 0 : process.versions) == null ? void 0 : _r.node) == "string" ? Xe = {
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
} : Xe = {
  get platform() {
    return Mt ? "win32" : pl ? "darwin" : "linux";
  },
  get arch() {
  },
  get env() {
    return {};
  },
  cwd() {
    return "/";
  }
};
const Yt = Xe.cwd, $l = Xe.env, zl = Xe.platform;
Xe.arch;
const jl = 65, Xl = 97, Yl = 90, Ql = 122, He = 46, re = 47, fe = 92, Se = 58, Zl = 63;
class xr extends Error {
  constructor(t, n, i) {
    let s;
    typeof n == "string" && n.indexOf("not ") === 0 ? (s = "must not be", n = n.replace(/^not /, "")) : s = "must be";
    const r = t.indexOf(".") !== -1 ? "property" : "argument";
    let a = `The "${t}" ${r} ${s} of type ${n}`;
    a += `. Received type ${typeof i}`, super(a), this.code = "ERR_INVALID_ARG_TYPE";
  }
}
function Jl(e, t) {
  if (e === null || typeof e != "object")
    throw new xr(t, "Object", e);
}
function ee(e, t) {
  if (typeof e != "string")
    throw new xr(t, "string", e);
}
const we = zl === "win32";
function H(e) {
  return e === re || e === fe;
}
function vn(e) {
  return e === re;
}
function Be(e) {
  return e >= jl && e <= Yl || e >= Xl && e <= Ql;
}
function Qt(e, t, n, i) {
  let s = "", r = 0, a = -1, u = 0, o = 0;
  for (let c = 0; c <= e.length; ++c) {
    if (c < e.length)
      o = e.charCodeAt(c);
    else {
      if (i(o))
        break;
      o = re;
    }
    if (i(o)) {
      if (!(a === c - 1 || u === 1)) if (u === 2) {
        if (s.length < 2 || r !== 2 || s.charCodeAt(s.length - 1) !== He || s.charCodeAt(s.length - 2) !== He) {
          if (s.length > 2) {
            const h = s.lastIndexOf(n);
            h === -1 ? (s = "", r = 0) : (s = s.slice(0, h), r = s.length - 1 - s.lastIndexOf(n)), a = c, u = 0;
            continue;
          } else if (s.length !== 0) {
            s = "", r = 0, a = c, u = 0;
            continue;
          }
        }
        t && (s += s.length > 0 ? `${n}..` : "..", r = 2);
      } else
        s.length > 0 ? s += `${n}${e.slice(a + 1, c)}` : s = e.slice(a + 1, c), r = c - a - 1;
      a = c, u = 0;
    } else o === He && u !== -1 ? ++u : u = -1;
  }
  return s;
}
function Kl(e) {
  return e ? `${e[0] === "." ? "" : "."}${e}` : "";
}
function Rr(e, t) {
  Jl(t, "pathObject");
  const n = t.dir || t.root, i = t.base || `${t.name || ""}${Kl(t.ext)}`;
  return n ? n === t.root ? `${n}${i}` : `${n}${e}${i}` : i;
}
const ie = {
  resolve(...e) {
    let t = "", n = "", i = !1;
    for (let s = e.length - 1; s >= -1; s--) {
      let r;
      if (s >= 0) {
        if (r = e[s], ee(r, `paths[${s}]`), r.length === 0)
          continue;
      } else t.length === 0 ? r = Yt() : (r = $l[`=${t}`] || Yt(), (r === void 0 || r.slice(0, 2).toLowerCase() !== t.toLowerCase() && r.charCodeAt(2) === fe) && (r = `${t}\\`));
      const a = r.length;
      let u = 0, o = "", c = !1;
      const h = r.charCodeAt(0);
      if (a === 1)
        H(h) && (u = 1, c = !0);
      else if (H(h))
        if (c = !0, H(r.charCodeAt(1))) {
          let f = 2, g = f;
          for (; f < a && !H(r.charCodeAt(f)); )
            f++;
          if (f < a && f !== g) {
            const b = r.slice(g, f);
            for (g = f; f < a && H(r.charCodeAt(f)); )
              f++;
            if (f < a && f !== g) {
              for (g = f; f < a && !H(r.charCodeAt(f)); )
                f++;
              (f === a || f !== g) && (o = `\\\\${b}\\${r.slice(g, f)}`, u = f);
            }
          }
        } else
          u = 1;
      else Be(h) && r.charCodeAt(1) === Se && (o = r.slice(0, 2), u = 2, a > 2 && H(r.charCodeAt(2)) && (c = !0, u = 3));
      if (o.length > 0)
        if (t.length > 0) {
          if (o.toLowerCase() !== t.toLowerCase())
            continue;
        } else
          t = o;
      if (i) {
        if (t.length > 0)
          break;
      } else if (n = `${r.slice(u)}\\${n}`, i = c, c && t.length > 0)
        break;
    }
    return n = Qt(n, !i, "\\", H), i ? `${t}\\${n}` : `${t}${n}` || ".";
  },
  normalize(e) {
    ee(e, "path");
    const t = e.length;
    if (t === 0)
      return ".";
    let n = 0, i, s = !1;
    const r = e.charCodeAt(0);
    if (t === 1)
      return vn(r) ? "\\" : e;
    if (H(r))
      if (s = !0, H(e.charCodeAt(1))) {
        let u = 2, o = u;
        for (; u < t && !H(e.charCodeAt(u)); )
          u++;
        if (u < t && u !== o) {
          const c = e.slice(o, u);
          for (o = u; u < t && H(e.charCodeAt(u)); )
            u++;
          if (u < t && u !== o) {
            for (o = u; u < t && !H(e.charCodeAt(u)); )
              u++;
            if (u === t)
              return `\\\\${c}\\${e.slice(o)}\\`;
            u !== o && (i = `\\\\${c}\\${e.slice(o, u)}`, n = u);
          }
        }
      } else
        n = 1;
    else Be(r) && e.charCodeAt(1) === Se && (i = e.slice(0, 2), n = 2, t > 2 && H(e.charCodeAt(2)) && (s = !0, n = 3));
    let a = n < t ? Qt(e.slice(n), !s, "\\", H) : "";
    return a.length === 0 && !s && (a = "."), a.length > 0 && H(e.charCodeAt(t - 1)) && (a += "\\"), i === void 0 ? s ? `\\${a}` : a : s ? `${i}\\${a}` : `${i}${a}`;
  },
  isAbsolute(e) {
    ee(e, "path");
    const t = e.length;
    if (t === 0)
      return !1;
    const n = e.charCodeAt(0);
    return H(n) || t > 2 && Be(n) && e.charCodeAt(1) === Se && H(e.charCodeAt(2));
  },
  join(...e) {
    if (e.length === 0)
      return ".";
    let t, n;
    for (let r = 0; r < e.length; ++r) {
      const a = e[r];
      ee(a, "path"), a.length > 0 && (t === void 0 ? t = n = a : t += `\\${a}`);
    }
    if (t === void 0)
      return ".";
    let i = !0, s = 0;
    if (typeof n == "string" && H(n.charCodeAt(0))) {
      ++s;
      const r = n.length;
      r > 1 && H(n.charCodeAt(1)) && (++s, r > 2 && (H(n.charCodeAt(2)) ? ++s : i = !1));
    }
    if (i) {
      for (; s < t.length && H(t.charCodeAt(s)); )
        s++;
      s >= 2 && (t = `\\${t.slice(s)}`);
    }
    return ie.normalize(t);
  },
  relative(e, t) {
    if (ee(e, "from"), ee(t, "to"), e === t)
      return "";
    const n = ie.resolve(e), i = ie.resolve(t);
    if (n === i || (e = n.toLowerCase(), t = i.toLowerCase(), e === t))
      return "";
    let s = 0;
    for (; s < e.length && e.charCodeAt(s) === fe; )
      s++;
    let r = e.length;
    for (; r - 1 > s && e.charCodeAt(r - 1) === fe; )
      r--;
    const a = r - s;
    let u = 0;
    for (; u < t.length && t.charCodeAt(u) === fe; )
      u++;
    let o = t.length;
    for (; o - 1 > u && t.charCodeAt(o - 1) === fe; )
      o--;
    const c = o - u, h = a < c ? a : c;
    let f = -1, g = 0;
    for (; g < h; g++) {
      const L = e.charCodeAt(s + g);
      if (L !== t.charCodeAt(u + g))
        break;
      L === fe && (f = g);
    }
    if (g !== h) {
      if (f === -1)
        return i;
    } else {
      if (c > h) {
        if (t.charCodeAt(u + g) === fe)
          return i.slice(u + g + 1);
        if (g === 2)
          return i.slice(u + g);
      }
      a > h && (e.charCodeAt(s + g) === fe ? f = g : g === 2 && (f = 3)), f === -1 && (f = 0);
    }
    let b = "";
    for (g = s + f + 1; g <= r; ++g)
      (g === r || e.charCodeAt(g) === fe) && (b += b.length === 0 ? ".." : "\\..");
    return u += f, b.length > 0 ? `${b}${i.slice(u, o)}` : (i.charCodeAt(u) === fe && ++u, i.slice(u, o));
  },
  toNamespacedPath(e) {
    if (typeof e != "string" || e.length === 0)
      return e;
    const t = ie.resolve(e);
    if (t.length <= 2)
      return e;
    if (t.charCodeAt(0) === fe) {
      if (t.charCodeAt(1) === fe) {
        const n = t.charCodeAt(2);
        if (n !== Zl && n !== He)
          return `\\\\?\\UNC\\${t.slice(2)}`;
      }
    } else if (Be(t.charCodeAt(0)) && t.charCodeAt(1) === Se && t.charCodeAt(2) === fe)
      return `\\\\?\\${t}`;
    return e;
  },
  dirname(e) {
    ee(e, "path");
    const t = e.length;
    if (t === 0)
      return ".";
    let n = -1, i = 0;
    const s = e.charCodeAt(0);
    if (t === 1)
      return H(s) ? e : ".";
    if (H(s)) {
      if (n = i = 1, H(e.charCodeAt(1))) {
        let u = 2, o = u;
        for (; u < t && !H(e.charCodeAt(u)); )
          u++;
        if (u < t && u !== o) {
          for (o = u; u < t && H(e.charCodeAt(u)); )
            u++;
          if (u < t && u !== o) {
            for (o = u; u < t && !H(e.charCodeAt(u)); )
              u++;
            if (u === t)
              return e;
            u !== o && (n = i = u + 1);
          }
        }
      }
    } else Be(s) && e.charCodeAt(1) === Se && (n = t > 2 && H(e.charCodeAt(2)) ? 3 : 2, i = n);
    let r = -1, a = !0;
    for (let u = t - 1; u >= i; --u)
      if (H(e.charCodeAt(u))) {
        if (!a) {
          r = u;
          break;
        }
      } else
        a = !1;
    if (r === -1) {
      if (n === -1)
        return ".";
      r = n;
    }
    return e.slice(0, r);
  },
  basename(e, t) {
    t !== void 0 && ee(t, "suffix"), ee(e, "path");
    let n = 0, i = -1, s = !0, r;
    if (e.length >= 2 && Be(e.charCodeAt(0)) && e.charCodeAt(1) === Se && (n = 2), t !== void 0 && t.length > 0 && t.length <= e.length) {
      if (t === e)
        return "";
      let a = t.length - 1, u = -1;
      for (r = e.length - 1; r >= n; --r) {
        const o = e.charCodeAt(r);
        if (H(o)) {
          if (!s) {
            n = r + 1;
            break;
          }
        } else
          u === -1 && (s = !1, u = r + 1), a >= 0 && (o === t.charCodeAt(a) ? --a === -1 && (i = r) : (a = -1, i = u));
      }
      return n === i ? i = u : i === -1 && (i = e.length), e.slice(n, i);
    }
    for (r = e.length - 1; r >= n; --r)
      if (H(e.charCodeAt(r))) {
        if (!s) {
          n = r + 1;
          break;
        }
      } else i === -1 && (s = !1, i = r + 1);
    return i === -1 ? "" : e.slice(n, i);
  },
  extname(e) {
    ee(e, "path");
    let t = 0, n = -1, i = 0, s = -1, r = !0, a = 0;
    e.length >= 2 && e.charCodeAt(1) === Se && Be(e.charCodeAt(0)) && (t = i = 2);
    for (let u = e.length - 1; u >= t; --u) {
      const o = e.charCodeAt(u);
      if (H(o)) {
        if (!r) {
          i = u + 1;
          break;
        }
        continue;
      }
      s === -1 && (r = !1, s = u + 1), o === He ? n === -1 ? n = u : a !== 1 && (a = 1) : n !== -1 && (a = -1);
    }
    return n === -1 || s === -1 || a === 0 || a === 1 && n === s - 1 && n === i + 1 ? "" : e.slice(n, s);
  },
  format: Rr.bind(null, "\\"),
  parse(e) {
    ee(e, "path");
    const t = { root: "", dir: "", base: "", ext: "", name: "" };
    if (e.length === 0)
      return t;
    const n = e.length;
    let i = 0, s = e.charCodeAt(0);
    if (n === 1)
      return H(s) ? (t.root = t.dir = e, t) : (t.base = t.name = e, t);
    if (H(s)) {
      if (i = 1, H(e.charCodeAt(1))) {
        let f = 2, g = f;
        for (; f < n && !H(e.charCodeAt(f)); )
          f++;
        if (f < n && f !== g) {
          for (g = f; f < n && H(e.charCodeAt(f)); )
            f++;
          if (f < n && f !== g) {
            for (g = f; f < n && !H(e.charCodeAt(f)); )
              f++;
            f === n ? i = f : f !== g && (i = f + 1);
          }
        }
      }
    } else if (Be(s) && e.charCodeAt(1) === Se) {
      if (n <= 2)
        return t.root = t.dir = e, t;
      if (i = 2, H(e.charCodeAt(2))) {
        if (n === 3)
          return t.root = t.dir = e, t;
        i = 3;
      }
    }
    i > 0 && (t.root = e.slice(0, i));
    let r = -1, a = i, u = -1, o = !0, c = e.length - 1, h = 0;
    for (; c >= i; --c) {
      if (s = e.charCodeAt(c), H(s)) {
        if (!o) {
          a = c + 1;
          break;
        }
        continue;
      }
      u === -1 && (o = !1, u = c + 1), s === He ? r === -1 ? r = c : h !== 1 && (h = 1) : r !== -1 && (h = -1);
    }
    return u !== -1 && (r === -1 || h === 0 || h === 1 && r === u - 1 && r === a + 1 ? t.base = t.name = e.slice(a, u) : (t.name = e.slice(a, r), t.base = e.slice(a, u), t.ext = e.slice(r, u))), a > 0 && a !== i ? t.dir = e.slice(0, a - 1) : t.dir = t.root, t;
  },
  sep: "\\",
  delimiter: ";",
  win32: null,
  posix: null
}, Cl = (() => {
  if (we) {
    const e = /\\/g;
    return () => {
      const t = Yt().replace(e, "/");
      return t.slice(t.indexOf("/"));
    };
  }
  return () => Yt();
})(), le = {
  resolve(...e) {
    let t = "", n = !1;
    for (let i = e.length - 1; i >= -1 && !n; i--) {
      const s = i >= 0 ? e[i] : Cl();
      ee(s, `paths[${i}]`), s.length !== 0 && (t = `${s}/${t}`, n = s.charCodeAt(0) === re);
    }
    return t = Qt(t, !n, "/", vn), n ? `/${t}` : t.length > 0 ? t : ".";
  },
  normalize(e) {
    if (ee(e, "path"), e.length === 0)
      return ".";
    const t = e.charCodeAt(0) === re, n = e.charCodeAt(e.length - 1) === re;
    return e = Qt(e, !t, "/", vn), e.length === 0 ? t ? "/" : n ? "./" : "." : (n && (e += "/"), t ? `/${e}` : e);
  },
  isAbsolute(e) {
    return ee(e, "path"), e.length > 0 && e.charCodeAt(0) === re;
  },
  join(...e) {
    if (e.length === 0)
      return ".";
    let t;
    for (let n = 0; n < e.length; ++n) {
      const i = e[n];
      ee(i, "path"), i.length > 0 && (t === void 0 ? t = i : t += `/${i}`);
    }
    return t === void 0 ? "." : le.normalize(t);
  },
  relative(e, t) {
    if (ee(e, "from"), ee(t, "to"), e === t || (e = le.resolve(e), t = le.resolve(t), e === t))
      return "";
    const n = 1, i = e.length, s = i - n, r = 1, a = t.length - r, u = s < a ? s : a;
    let o = -1, c = 0;
    for (; c < u; c++) {
      const f = e.charCodeAt(n + c);
      if (f !== t.charCodeAt(r + c))
        break;
      f === re && (o = c);
    }
    if (c === u)
      if (a > u) {
        if (t.charCodeAt(r + c) === re)
          return t.slice(r + c + 1);
        if (c === 0)
          return t.slice(r + c);
      } else s > u && (e.charCodeAt(n + c) === re ? o = c : c === 0 && (o = 0));
    let h = "";
    for (c = n + o + 1; c <= i; ++c)
      (c === i || e.charCodeAt(c) === re) && (h += h.length === 0 ? ".." : "/..");
    return `${h}${t.slice(r + o)}`;
  },
  toNamespacedPath(e) {
    return e;
  },
  dirname(e) {
    if (ee(e, "path"), e.length === 0)
      return ".";
    const t = e.charCodeAt(0) === re;
    let n = -1, i = !0;
    for (let s = e.length - 1; s >= 1; --s)
      if (e.charCodeAt(s) === re) {
        if (!i) {
          n = s;
          break;
        }
      } else
        i = !1;
    return n === -1 ? t ? "/" : "." : t && n === 1 ? "//" : e.slice(0, n);
  },
  basename(e, t) {
    t !== void 0 && ee(t, "ext"), ee(e, "path");
    let n = 0, i = -1, s = !0, r;
    if (t !== void 0 && t.length > 0 && t.length <= e.length) {
      if (t === e)
        return "";
      let a = t.length - 1, u = -1;
      for (r = e.length - 1; r >= 0; --r) {
        const o = e.charCodeAt(r);
        if (o === re) {
          if (!s) {
            n = r + 1;
            break;
          }
        } else
          u === -1 && (s = !1, u = r + 1), a >= 0 && (o === t.charCodeAt(a) ? --a === -1 && (i = r) : (a = -1, i = u));
      }
      return n === i ? i = u : i === -1 && (i = e.length), e.slice(n, i);
    }
    for (r = e.length - 1; r >= 0; --r)
      if (e.charCodeAt(r) === re) {
        if (!s) {
          n = r + 1;
          break;
        }
      } else i === -1 && (s = !1, i = r + 1);
    return i === -1 ? "" : e.slice(n, i);
  },
  extname(e) {
    ee(e, "path");
    let t = -1, n = 0, i = -1, s = !0, r = 0;
    for (let a = e.length - 1; a >= 0; --a) {
      const u = e.charCodeAt(a);
      if (u === re) {
        if (!s) {
          n = a + 1;
          break;
        }
        continue;
      }
      i === -1 && (s = !1, i = a + 1), u === He ? t === -1 ? t = a : r !== 1 && (r = 1) : t !== -1 && (r = -1);
    }
    return t === -1 || i === -1 || r === 0 || r === 1 && t === i - 1 && t === n + 1 ? "" : e.slice(t, i);
  },
  format: Rr.bind(null, "/"),
  parse(e) {
    ee(e, "path");
    const t = { root: "", dir: "", base: "", ext: "", name: "" };
    if (e.length === 0)
      return t;
    const n = e.charCodeAt(0) === re;
    let i;
    n ? (t.root = "/", i = 1) : i = 0;
    let s = -1, r = 0, a = -1, u = !0, o = e.length - 1, c = 0;
    for (; o >= i; --o) {
      const h = e.charCodeAt(o);
      if (h === re) {
        if (!u) {
          r = o + 1;
          break;
        }
        continue;
      }
      a === -1 && (u = !1, a = o + 1), h === He ? s === -1 ? s = o : c !== 1 && (c = 1) : s !== -1 && (c = -1);
    }
    if (a !== -1) {
      const h = r === 0 && n ? 1 : r;
      s === -1 || c === 0 || c === 1 && s === a - 1 && s === r + 1 ? t.base = t.name = e.slice(h, a) : (t.name = e.slice(h, s), t.base = e.slice(h, a), t.ext = e.slice(s, a));
    }
    return r > 0 ? t.dir = e.slice(0, r - 1) : n && (t.dir = "/"), t;
  },
  sep: "/",
  delimiter: ":",
  win32: null,
  posix: null
};
le.win32 = ie.win32 = ie;
le.posix = ie.posix = le;
we ? ie.normalize : le.normalize;
we ? ie.isAbsolute : le.isAbsolute;
we ? ie.join : le.join;
we ? ie.resolve : le.resolve;
we ? ie.relative : le.relative;
we ? ie.dirname : le.dirname;
we ? ie.basename : le.basename;
we ? ie.extname : le.extname;
we ? ie.parse : le.parse;
we ? ie.sep : le.sep;
we ? ie.delimiter : le.delimiter;
const e1 = /^\w[\w\d+.-]*$/, t1 = /^\//, n1 = /^\/\//;
function i1(e, t) {
  if (!e.scheme && t)
    throw new Error(
      `[UriError]: Scheme is missing: {scheme: "", authority: "${e.authority}", path: "${e.path}", query: "${e.query}", fragment: "${e.fragment}"}`
    );
  if (e.scheme && !e1.test(e.scheme))
    throw new Error("[UriError]: Scheme contains illegal characters.");
  if (e.path) {
    if (e.authority) {
      if (!t1.test(e.path))
        throw new Error(
          '[UriError]: If a URI contains an authority component, then the path component must either be empty or begin with a slash ("/") character'
        );
    } else if (n1.test(e.path))
      throw new Error(
        '[UriError]: If a URI does not contain an authority component, then the path cannot begin with two slash characters ("//")'
      );
  }
}
function s1(e, t) {
  return !e && !t ? "file" : e;
}
function r1(e, t) {
  switch (e) {
    case "https":
    case "http":
    case "file":
      t ? t[0] !== xe && (t = xe + t) : t = xe;
      break;
  }
  return t;
}
const Y = "", xe = "/", l1 = /^(([^:/?#]+?):)?(\/\/([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?/;
class Ye {
  static isUri(t) {
    return t instanceof Ye ? !0 : t ? typeof t.authority == "string" && typeof t.fragment == "string" && typeof t.path == "string" && typeof t.query == "string" && typeof t.scheme == "string" && typeof t.fsPath == "string" && typeof t.with == "function" && typeof t.toString == "function" : !1;
  }
  constructor(t, n, i, s, r, a = !1) {
    typeof t == "object" ? (this.scheme = t.scheme || Y, this.authority = t.authority || Y, this.path = t.path || Y, this.query = t.query || Y, this.fragment = t.fragment || Y) : (this.scheme = s1(t, a), this.authority = n || Y, this.path = r1(this.scheme, i || Y), this.query = s || Y, this.fragment = r || Y, i1(this, a));
  }
  get fsPath() {
    return Un(this, !1);
  }
  with(t) {
    if (!t)
      return this;
    let { scheme: n, authority: i, path: s, query: r, fragment: a } = t;
    return n === void 0 ? n = this.scheme : n === null && (n = Y), i === void 0 ? i = this.authority : i === null && (i = Y), s === void 0 ? s = this.path : s === null && (s = Y), r === void 0 ? r = this.query : r === null && (r = Y), a === void 0 ? a = this.fragment : a === null && (a = Y), n === this.scheme && i === this.authority && s === this.path && r === this.query && a === this.fragment ? this : new Ce(n, i, s, r, a);
  }
  static parse(t, n = !1) {
    const i = l1.exec(t);
    return i ? new Ce(
      i[2] || Y,
      Pt(i[4] || Y),
      Pt(i[5] || Y),
      Pt(i[7] || Y),
      Pt(i[9] || Y),
      n
    ) : new Ce(Y, Y, Y, Y, Y);
  }
  static file(t) {
    let n = Y;
    if (Mt && (t = t.replace(/\\/g, xe)), t[0] === xe && t[1] === xe) {
      const i = t.indexOf(xe, 2);
      i === -1 ? (n = t.substring(2), t = xe) : (n = t.substring(2, i), t = t.substring(i) || xe);
    }
    return new Ce("file", n, t, Y, Y);
  }
  static from(t, n) {
    return new Ce(
      t.scheme,
      t.authority,
      t.path,
      t.query,
      t.fragment,
      n
    );
  }
  static joinPath(t, ...n) {
    if (!t.path)
      throw new Error("[UriError]: cannot call joinPath on URI without path");
    let i;
    return Mt && t.scheme === "file" ? i = Ye.file(ie.join(Un(t, !0), ...n)).path : i = le.join(t.path, ...n), t.with({ path: i });
  }
  toString(t = !1) {
    return Dn(this, t);
  }
  toJSON() {
    return this;
  }
  static revive(t) {
    if (t) {
      if (t instanceof Ye)
        return t;
      {
        const n = new Ce(t);
        return n._formatted = t.external ?? null, n._fsPath = t._sep === vr ? t.fsPath ?? null : null, n;
      }
    } else return t;
  }
  [Symbol.for("debug.description")]() {
    return `URI(${this.toString()})`;
  }
}
const vr = Mt ? 1 : void 0;
class Ce extends Ye {
  constructor() {
    super(...arguments), this._formatted = null, this._fsPath = null;
  }
  get fsPath() {
    return this._fsPath || (this._fsPath = Un(this, !1)), this._fsPath;
  }
  toString(t = !1) {
    return t ? Dn(this, !0) : (this._formatted || (this._formatted = Dn(this, !1)), this._formatted);
  }
  toJSON() {
    const t = {
      $mid: Rn.Uri
    };
    return this._fsPath && (t.fsPath = this._fsPath, t._sep = vr), this._formatted && (t.external = this._formatted), this.path && (t.path = this.path), this.scheme && (t.scheme = this.scheme), this.authority && (t.authority = this.authority), this.query && (t.query = this.query), this.fragment && (t.fragment = this.fragment), t;
  }
}
const Ur = {
  [d.Colon]: "%3A",
  [d.Slash]: "%2F",
  [d.QuestionMark]: "%3F",
  [d.Hash]: "%23",
  [d.OpenSquareBracket]: "%5B",
  [d.CloseSquareBracket]: "%5D",
  [d.AtSign]: "%40",
  [d.ExclamationMark]: "%21",
  [d.DollarSign]: "%24",
  [d.Ampersand]: "%26",
  [d.SingleQuote]: "%27",
  [d.OpenParen]: "%28",
  [d.CloseParen]: "%29",
  [d.Asterisk]: "%2A",
  [d.Plus]: "%2B",
  [d.Comma]: "%2C",
  [d.Semicolon]: "%3B",
  [d.Equals]: "%3D",
  [d.Space]: "%20"
};
function gi(e, t, n) {
  let i, s = -1;
  for (let r = 0; r < e.length; r++) {
    const a = e.charCodeAt(r);
    if (a >= d.a && a <= d.z || a >= d.A && a <= d.Z || a >= d.Digit0 && a <= d.Digit9 || a === d.Dash || a === d.Period || a === d.Underline || a === d.Tilde || t && a === d.Slash || n && a === d.OpenSquareBracket || n && a === d.CloseSquareBracket || n && a === d.Colon)
      s !== -1 && (i += encodeURIComponent(e.substring(s, r)), s = -1), i !== void 0 && (i += e.charAt(r));
    else {
      i === void 0 && (i = e.substr(0, r));
      const u = Ur[a];
      u !== void 0 ? (s !== -1 && (i += encodeURIComponent(e.substring(s, r)), s = -1), i += u) : s === -1 && (s = r);
    }
  }
  return s !== -1 && (i += encodeURIComponent(e.substring(s))), i !== void 0 ? i : e;
}
function a1(e) {
  let t;
  for (let n = 0; n < e.length; n++) {
    const i = e.charCodeAt(n);
    i === d.Hash || i === d.QuestionMark ? (t === void 0 && (t = e.substr(0, n)), t += Ur[i]) : t !== void 0 && (t += e[n]);
  }
  return t !== void 0 ? t : e;
}
function Un(e, t) {
  let n;
  return e.authority && e.path.length > 1 && e.scheme === "file" ? n = `//${e.authority}${e.path}` : e.path.charCodeAt(0) === d.Slash && (e.path.charCodeAt(1) >= d.A && e.path.charCodeAt(1) <= d.Z || e.path.charCodeAt(1) >= d.a && e.path.charCodeAt(1) <= d.z) && e.path.charCodeAt(2) === d.Colon ? t ? n = e.path.substr(1) : n = e.path[1].toLowerCase() + e.path.substr(2) : n = e.path, Mt && (n = n.replace(/\//g, "\\")), n;
}
function Dn(e, t) {
  const n = t ? a1 : gi;
  let i = "", { scheme: s, authority: r, path: a, query: u, fragment: o } = e;
  if (s && (i += s, i += ":"), (r || s === "file") && (i += xe, i += xe), r) {
    let c = r.indexOf("@");
    if (c !== -1) {
      const h = r.substr(0, c);
      r = r.substr(c + 1), c = h.lastIndexOf(":"), c === -1 ? i += n(h, !1, !1) : (i += n(h.substr(0, c), !1, !1), i += ":", i += n(h.substr(c + 1), !1, !0)), i += "@";
    }
    r = r.toLowerCase(), c = r.lastIndexOf(":"), c === -1 ? i += n(r, !1, !0) : (i += n(r.substr(0, c), !1, !0), i += r.substr(c));
  }
  if (a) {
    if (a.length >= 3 && a.charCodeAt(0) === d.Slash && a.charCodeAt(2) === d.Colon) {
      const c = a.charCodeAt(1);
      c >= d.A && c <= d.Z && (a = `/${String.fromCharCode(c + 32)}:${a.substr(3)}`);
    } else if (a.length >= 2 && a.charCodeAt(1) === d.Colon) {
      const c = a.charCodeAt(0);
      c >= d.A && c <= d.Z && (a = `${String.fromCharCode(c + 32)}:${a.substr(2)}`);
    }
    i += n(a, !0, !1);
  }
  return u && (i += "?", i += n(u, !1, !1)), o && (i += "#", i += t ? o : gi(o, !1, !1)), i;
}
function Dr(e) {
  try {
    return decodeURIComponent(e);
  } catch {
    return e.length > 3 ? e.substr(0, 3) + Dr(e.substr(3)) : e;
  }
}
const _i = /(%[0-9A-Za-z][0-9A-Za-z])+/g;
function Pt(e) {
  return e.match(_i) ? e.replace(_i, (t) => Dr(t)) : e;
}
class X {
  constructor(t, n) {
    this.lineNumber = t, this.column = n;
  }
  with(t = this.lineNumber, n = this.column) {
    return t === this.lineNumber && n === this.column ? this : new X(t, n);
  }
  delta(t = 0, n = 0) {
    return this.with(this.lineNumber + t, this.column + n);
  }
  equals(t) {
    return X.equals(this, t);
  }
  static equals(t, n) {
    return !t && !n ? !0 : !!t && !!n && t.lineNumber === n.lineNumber && t.column === n.column;
  }
  isBefore(t) {
    return X.isBefore(this, t);
  }
  static isBefore(t, n) {
    return t.lineNumber < n.lineNumber ? !0 : n.lineNumber < t.lineNumber ? !1 : t.column < n.column;
  }
  isBeforeOrEqual(t) {
    return X.isBeforeOrEqual(this, t);
  }
  static isBeforeOrEqual(t, n) {
    return t.lineNumber < n.lineNumber ? !0 : n.lineNumber < t.lineNumber ? !1 : t.column <= n.column;
  }
  static compare(t, n) {
    const i = t.lineNumber | 0, s = n.lineNumber | 0;
    if (i === s) {
      const r = t.column | 0, a = n.column | 0;
      return r - a;
    }
    return i - s;
  }
  clone() {
    return new X(this.lineNumber, this.column);
  }
  toString() {
    return "(" + this.lineNumber + "," + this.column + ")";
  }
  static lift(t) {
    return new X(t.lineNumber, t.column);
  }
  static isIPosition(t) {
    return t && typeof t.lineNumber == "number" && typeof t.column == "number";
  }
  toJSON() {
    return {
      lineNumber: this.lineNumber,
      column: this.column
    };
  }
}
class S {
  constructor(t, n, i, s) {
    t > i || t === i && n > s ? (this.startLineNumber = i, this.startColumn = s, this.endLineNumber = t, this.endColumn = n) : (this.startLineNumber = t, this.startColumn = n, this.endLineNumber = i, this.endColumn = s);
  }
  isEmpty() {
    return S.isEmpty(this);
  }
  static isEmpty(t) {
    return t.startLineNumber === t.endLineNumber && t.startColumn === t.endColumn;
  }
  containsPosition(t) {
    return S.containsPosition(this, t);
  }
  static containsPosition(t, n) {
    return !(n.lineNumber < t.startLineNumber || n.lineNumber > t.endLineNumber || n.lineNumber === t.startLineNumber && n.column < t.startColumn || n.lineNumber === t.endLineNumber && n.column > t.endColumn);
  }
  static strictContainsPosition(t, n) {
    return !(n.lineNumber < t.startLineNumber || n.lineNumber > t.endLineNumber || n.lineNumber === t.startLineNumber && n.column <= t.startColumn || n.lineNumber === t.endLineNumber && n.column >= t.endColumn);
  }
  containsRange(t) {
    return S.containsRange(this, t);
  }
  static containsRange(t, n) {
    return !(n.startLineNumber < t.startLineNumber || n.endLineNumber < t.startLineNumber || n.startLineNumber > t.endLineNumber || n.endLineNumber > t.endLineNumber || n.startLineNumber === t.startLineNumber && n.startColumn < t.startColumn || n.endLineNumber === t.endLineNumber && n.endColumn > t.endColumn);
  }
  strictContainsRange(t) {
    return S.strictContainsRange(this, t);
  }
  static strictContainsRange(t, n) {
    return !(n.startLineNumber < t.startLineNumber || n.endLineNumber < t.startLineNumber || n.startLineNumber > t.endLineNumber || n.endLineNumber > t.endLineNumber || n.startLineNumber === t.startLineNumber && n.startColumn <= t.startColumn || n.endLineNumber === t.endLineNumber && n.endColumn >= t.endColumn);
  }
  plusRange(t) {
    return S.plusRange(this, t);
  }
  static plusRange(t, n) {
    let i, s, r, a;
    return n.startLineNumber < t.startLineNumber ? (i = n.startLineNumber, s = n.startColumn) : n.startLineNumber === t.startLineNumber ? (i = n.startLineNumber, s = Math.min(n.startColumn, t.startColumn)) : (i = t.startLineNumber, s = t.startColumn), n.endLineNumber > t.endLineNumber ? (r = n.endLineNumber, a = n.endColumn) : n.endLineNumber === t.endLineNumber ? (r = n.endLineNumber, a = Math.max(n.endColumn, t.endColumn)) : (r = t.endLineNumber, a = t.endColumn), new S(i, s, r, a);
  }
  intersectRanges(t) {
    return S.intersectRanges(this, t);
  }
  static intersectRanges(t, n) {
    let i = t.startLineNumber, s = t.startColumn, r = t.endLineNumber, a = t.endColumn;
    const u = n.startLineNumber, o = n.startColumn, c = n.endLineNumber, h = n.endColumn;
    return i < u ? (i = u, s = o) : i === u && (s = Math.max(s, o)), r > c ? (r = c, a = h) : r === c && (a = Math.min(a, h)), i > r || i === r && s > a ? null : new S(
      i,
      s,
      r,
      a
    );
  }
  equalsRange(t) {
    return S.equalsRange(this, t);
  }
  static equalsRange(t, n) {
    return !t && !n ? !0 : !!t && !!n && t.startLineNumber === n.startLineNumber && t.startColumn === n.startColumn && t.endLineNumber === n.endLineNumber && t.endColumn === n.endColumn;
  }
  getEndPosition() {
    return S.getEndPosition(this);
  }
  static getEndPosition(t) {
    return new X(t.endLineNumber, t.endColumn);
  }
  getStartPosition() {
    return S.getStartPosition(this);
  }
  static getStartPosition(t) {
    return new X(t.startLineNumber, t.startColumn);
  }
  toString() {
    return "[" + this.startLineNumber + "," + this.startColumn + " -> " + this.endLineNumber + "," + this.endColumn + "]";
  }
  setEndPosition(t, n) {
    return new S(this.startLineNumber, this.startColumn, t, n);
  }
  setStartPosition(t, n) {
    return new S(t, n, this.endLineNumber, this.endColumn);
  }
  collapseToStart() {
    return S.collapseToStart(this);
  }
  static collapseToStart(t) {
    return new S(
      t.startLineNumber,
      t.startColumn,
      t.startLineNumber,
      t.startColumn
    );
  }
  collapseToEnd() {
    return S.collapseToEnd(this);
  }
  static collapseToEnd(t) {
    return new S(t.endLineNumber, t.endColumn, t.endLineNumber, t.endColumn);
  }
  delta(t) {
    return new S(
      this.startLineNumber + t,
      this.startColumn,
      this.endLineNumber + t,
      this.endColumn
    );
  }
  static fromPositions(t, n = t) {
    return new S(t.lineNumber, t.column, n.lineNumber, n.column);
  }
  static lift(t) {
    return t ? new S(
      t.startLineNumber,
      t.startColumn,
      t.endLineNumber,
      t.endColumn
    ) : null;
  }
  static isIRange(t) {
    return t && typeof t.startLineNumber == "number" && typeof t.startColumn == "number" && typeof t.endLineNumber == "number" && typeof t.endColumn == "number";
  }
  static areIntersectingOrTouching(t, n) {
    return !(t.endLineNumber < n.startLineNumber || t.endLineNumber === n.startLineNumber && t.endColumn < n.startColumn || n.endLineNumber < t.startLineNumber || n.endLineNumber === t.startLineNumber && n.endColumn < t.startColumn);
  }
  static areIntersecting(t, n) {
    return !(t.endLineNumber < n.startLineNumber || t.endLineNumber === n.startLineNumber && t.endColumn <= n.startColumn || n.endLineNumber < t.startLineNumber || n.endLineNumber === t.startLineNumber && n.endColumn <= t.startColumn);
  }
  static compareRangesUsingStarts(t, n) {
    if (t && n) {
      const r = t.startLineNumber | 0, a = n.startLineNumber | 0;
      if (r === a) {
        const u = t.startColumn | 0, o = n.startColumn | 0;
        if (u === o) {
          const c = t.endLineNumber | 0, h = n.endLineNumber | 0;
          if (c === h) {
            const f = t.endColumn | 0, g = n.endColumn | 0;
            return f - g;
          }
          return c - h;
        }
        return u - o;
      }
      return r - a;
    }
    return (t ? 1 : 0) - (n ? 1 : 0);
  }
  static compareRangesUsingEnds(t, n) {
    return t.endLineNumber === n.endLineNumber ? t.endColumn === n.endColumn ? t.startLineNumber === n.startLineNumber ? t.startColumn - n.startColumn : t.startLineNumber - n.startLineNumber : t.endColumn - n.endColumn : t.endLineNumber - n.endLineNumber;
  }
  static spansMultipleLines(t) {
    return t.endLineNumber > t.startLineNumber;
  }
  toJSON() {
    return this;
  }
}
class u1 {
  constructor(t) {
    this.values = t, this.prefixSum = new Uint32Array(t.length), this.prefixSumValidIndex = new Int32Array(1), this.prefixSumValidIndex[0] = -1;
  }
  getCount() {
    return this.values.length;
  }
  insertValues(t, n) {
    t = Ze(t);
    const i = this.values, s = this.prefixSum, r = n.length;
    return r === 0 ? !1 : (this.values = new Uint32Array(i.length + r), this.values.set(i.subarray(0, t), 0), this.values.set(i.subarray(t), t + r), this.values.set(n, t), t - 1 < this.prefixSumValidIndex[0] && (this.prefixSumValidIndex[0] = t - 1), this.prefixSum = new Uint32Array(this.values.length), this.prefixSumValidIndex[0] >= 0 && this.prefixSum.set(s.subarray(0, this.prefixSumValidIndex[0] + 1)), !0);
  }
  setValue(t, n) {
    return t = Ze(t), n = Ze(n), this.values[t] === n ? !1 : (this.values[t] = n, t - 1 < this.prefixSumValidIndex[0] && (this.prefixSumValidIndex[0] = t - 1), !0);
  }
  removeValues(t, n) {
    t = Ze(t), n = Ze(n);
    const i = this.values, s = this.prefixSum;
    if (t >= i.length)
      return !1;
    const r = i.length - t;
    return n >= r && (n = r), n === 0 ? !1 : (this.values = new Uint32Array(i.length - n), this.values.set(i.subarray(0, t), 0), this.values.set(i.subarray(t + n), t), this.prefixSum = new Uint32Array(this.values.length), t - 1 < this.prefixSumValidIndex[0] && (this.prefixSumValidIndex[0] = t - 1), this.prefixSumValidIndex[0] >= 0 && this.prefixSum.set(s.subarray(0, this.prefixSumValidIndex[0] + 1)), !0);
  }
  getTotalSum() {
    return this.values.length === 0 ? 0 : this._getPrefixSum(this.values.length - 1);
  }
  getPrefixSum(t) {
    return t < 0 ? 0 : (t = Ze(t), this._getPrefixSum(t));
  }
  _getPrefixSum(t) {
    if (t <= this.prefixSumValidIndex[0])
      return this.prefixSum[t];
    let n = this.prefixSumValidIndex[0] + 1;
    n === 0 && (this.prefixSum[0] = this.values[0], n++), t >= this.values.length && (t = this.values.length - 1);
    for (let i = n; i <= t; i++)
      this.prefixSum[i] = this.prefixSum[i - 1] + this.values[i];
    return this.prefixSumValidIndex[0] = Math.max(this.prefixSumValidIndex[0], t), this.prefixSum[t];
  }
  getIndexOf(t) {
    t = Math.floor(t), this.getTotalSum();
    let n = 0, i = this.values.length - 1, s = 0, r = 0, a = 0;
    for (; n <= i; )
      if (s = n + (i - n) / 2 | 0, r = this.prefixSum[s], a = r - this.values[s], t < a)
        i = s - 1;
      else if (t >= r)
        n = s + 1;
      else
        break;
    return new o1(s, t - a);
  }
}
class o1 {
  constructor(t, n) {
    this.index = t, this.remainder = n, this._prefixSumIndexOfResultBrand = void 0, this.index = t, this.remainder = n;
  }
}
class c1 {
  constructor(t, n, i, s) {
    this._uri = t, this._lines = n, this._eol = i, this._versionId = s, this._lineStarts = null, this._cachedTextValue = null;
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
    for (const i of n)
      this._acceptDeleteRange(i.range), this._acceptInsertText(new X(i.range.startLineNumber, i.range.startColumn), i.text);
    this._versionId = t.versionId, this._cachedTextValue = null;
  }
  _ensureLineStarts() {
    if (!this._lineStarts) {
      const t = this._eol.length, n = this._lines.length, i = new Uint32Array(n);
      for (let s = 0; s < n; s++)
        i[s] = this._lines[s].length + t;
      this._lineStarts = new u1(i);
    }
  }
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
    const i = Ul(n);
    if (i.length === 1) {
      this._setLineText(t.lineNumber - 1, this._lines[t.lineNumber - 1].substring(0, t.column - 1) + i[0] + this._lines[t.lineNumber - 1].substring(t.column - 1));
      return;
    }
    i[i.length - 1] += this._lines[t.lineNumber - 1].substring(t.column - 1), this._setLineText(t.lineNumber - 1, this._lines[t.lineNumber - 1].substring(0, t.column - 1) + i[0]);
    const s = new Uint32Array(i.length - 1);
    for (let r = 1; r < i.length; r++)
      this._lines.splice(t.lineNumber + r - 1, 0, i[r]), s[r - 1] = i[r].length + this._eol.length;
    this._lineStarts && this._lineStarts.insertValues(t.lineNumber, s);
  }
}
const f1 = "`~!@#$%^&*()-=+[{]}\\|;:'\",.<>/?";
function h1(e = "") {
  let t = "(-?\\d*\\.\\d\\w*)|([^";
  for (const n of f1)
    e.indexOf(n) >= 0 || (t += "\\" + n);
  return t += "\\s]+)", new RegExp(t, "g");
}
const Tr = h1();
function Mr(e) {
  let t = Tr;
  if (e && e instanceof RegExp)
    if (e.global)
      t = e;
    else {
      let n = "g";
      e.ignoreCase && (n += "i"), e.multiline && (n += "m"), e.unicode && (n += "u"), t = new RegExp(e.source, n);
    }
  return t.lastIndex = 0, t;
}
const Fr = new ll();
Fr.unshift({
  maxLen: 1e3,
  windowSize: 15,
  timeBudget: 150
});
function $n(e, t, n, i, s) {
  if (t = Mr(t), s || (s = $t.first(Fr)), n.length > s.maxLen) {
    let c = e - s.maxLen / 2;
    return c < 0 ? c = 0 : i += c, n = n.substring(c, e + s.maxLen / 2), $n(e, t, n, i, s);
  }
  const r = Date.now(), a = e - 1 - i;
  let u = -1, o = null;
  for (let c = 1; !(Date.now() - r >= s.timeBudget); c++) {
    const h = a - s.windowSize * c;
    t.lastIndex = Math.max(0, h);
    const f = m1(t, n, a, u);
    if (!f && o || (o = f, h <= 0))
      break;
    u = h;
  }
  if (o) {
    const c = {
      word: o[0],
      startColumn: i + 1 + o.index,
      endColumn: i + 1 + o.index + o[0].length
    };
    return t.lastIndex = 0, c;
  }
  return null;
}
function m1(e, t, n, i) {
  let s;
  for (; s = e.exec(t); ) {
    const r = s.index || 0;
    if (r <= n && e.lastIndex >= n)
      return s;
    if (i > 0 && r > i)
      return null;
  }
  return null;
}
class zn {
  constructor(t) {
    const n = ri(t);
    this._defaultValue = n, this._asciiMap = zn._createAsciiMap(n), this._map = /* @__PURE__ */ new Map();
  }
  static _createAsciiMap(t) {
    const n = new Uint8Array(256);
    return n.fill(t), n;
  }
  set(t, n) {
    const i = ri(n);
    t >= 0 && t < 256 ? this._asciiMap[t] = i : this._map.set(t, i);
  }
  get(t) {
    return t >= 0 && t < 256 ? this._asciiMap[t] : this._map.get(t) || this._defaultValue;
  }
  clear() {
    this._asciiMap.fill(this._defaultValue), this._map.clear();
  }
}
var bi;
(function(e) {
  e[e.False = 0] = "False", e[e.True = 1] = "True";
})(bi || (bi = {}));
var P;
(function(e) {
  e[e.Invalid = 0] = "Invalid", e[e.Start = 1] = "Start", e[e.H = 2] = "H", e[e.HT = 3] = "HT", e[e.HTT = 4] = "HTT", e[e.HTTP = 5] = "HTTP", e[e.F = 6] = "F", e[e.FI = 7] = "FI", e[e.FIL = 8] = "FIL", e[e.BeforeColon = 9] = "BeforeColon", e[e.AfterColon = 10] = "AfterColon", e[e.AlmostThere = 11] = "AlmostThere", e[e.End = 12] = "End", e[e.Accept = 13] = "Accept", e[e.LastKnownState = 14] = "LastKnownState";
})(P || (P = {}));
class g1 {
  constructor(t, n, i) {
    const s = new Uint8Array(t * n);
    for (let r = 0, a = t * n; r < a; r++)
      s[r] = i;
    this._data = s, this.rows = t, this.cols = n;
  }
  get(t, n) {
    return this._data[t * this.cols + n];
  }
  set(t, n, i) {
    this._data[t * this.cols + n] = i;
  }
}
class _1 {
  constructor(t) {
    let n = 0, i = P.Invalid;
    for (let r = 0, a = t.length; r < a; r++) {
      const [u, o, c] = t[r];
      o > n && (n = o), u > i && (i = u), c > i && (i = c);
    }
    n++, i++;
    const s = new g1(i, n, P.Invalid);
    for (let r = 0, a = t.length; r < a; r++) {
      const [u, o, c] = t[r];
      s.set(u, o, c);
    }
    this._states = s, this._maxCharCode = n;
  }
  nextState(t, n) {
    return n < 0 || n >= this._maxCharCode ? P.Invalid : this._states.get(t, n);
  }
}
let mn = null;
function b1() {
  return mn === null && (mn = new _1([
    [P.Start, d.h, P.H],
    [P.Start, d.H, P.H],
    [P.Start, d.f, P.F],
    [P.Start, d.F, P.F],
    [P.H, d.t, P.HT],
    [P.H, d.T, P.HT],
    [P.HT, d.t, P.HTT],
    [P.HT, d.T, P.HTT],
    [P.HTT, d.p, P.HTTP],
    [P.HTT, d.P, P.HTTP],
    [P.HTTP, d.s, P.BeforeColon],
    [P.HTTP, d.S, P.BeforeColon],
    [P.HTTP, d.Colon, P.AfterColon],
    [P.F, d.i, P.FI],
    [P.F, d.I, P.FI],
    [P.FI, d.l, P.FIL],
    [P.FI, d.L, P.FIL],
    [P.FIL, d.e, P.BeforeColon],
    [P.FIL, d.E, P.BeforeColon],
    [P.BeforeColon, d.Colon, P.AfterColon],
    [P.AfterColon, d.Slash, P.AlmostThere],
    [P.AlmostThere, d.Slash, P.End]
  ])), mn;
}
var Q;
(function(e) {
  e[e.None = 0] = "None", e[e.ForceTermination = 1] = "ForceTermination", e[e.CannotEndIn = 2] = "CannotEndIn";
})(Q || (Q = {}));
let bt = null;
function d1() {
  if (bt === null) {
    bt = new zn(Q.None);
    const e = ` 	<>'"、。｡､，．：；‘〈「『〔（［｛｢｣｝］）〕』」〉’｀～…`;
    for (let n = 0; n < e.length; n++)
      bt.set(e.charCodeAt(n), Q.ForceTermination);
    const t = ".,;:";
    for (let n = 0; n < t.length; n++)
      bt.set(t.charCodeAt(n), Q.CannotEndIn);
  }
  return bt;
}
class Zt {
  static _createLink(t, n, i, s, r) {
    let a = r - 1;
    do {
      const u = n.charCodeAt(a);
      if (t.get(u) !== Q.CannotEndIn)
        break;
      a--;
    } while (a > s);
    if (s > 0) {
      const u = n.charCodeAt(s - 1), o = n.charCodeAt(a);
      (u === d.OpenParen && o === d.CloseParen || u === d.OpenSquareBracket && o === d.CloseSquareBracket || u === d.OpenCurlyBrace && o === d.CloseCurlyBrace) && a--;
    }
    return {
      range: {
        startLineNumber: i,
        startColumn: s + 1,
        endLineNumber: i,
        endColumn: a + 2
      },
      url: n.substring(s, a + 1)
    };
  }
  static computeLinks(t, n = b1()) {
    const i = d1(), s = [];
    for (let r = 1, a = t.getLineCount(); r <= a; r++) {
      const u = t.getLineContent(r), o = u.length;
      let c = 0, h = 0, f = 0, g = P.Start, b = !1, L = !1, N = !1, A = !1;
      for (; c < o; ) {
        let x = !1;
        const R = u.charCodeAt(c);
        if (g === P.Accept) {
          let U;
          switch (R) {
            case d.OpenParen:
              b = !0, U = Q.None;
              break;
            case d.CloseParen:
              U = b ? Q.None : Q.ForceTermination;
              break;
            case d.OpenSquareBracket:
              N = !0, L = !0, U = Q.None;
              break;
            case d.CloseSquareBracket:
              N = !1, U = L ? Q.None : Q.ForceTermination;
              break;
            case d.OpenCurlyBrace:
              A = !0, U = Q.None;
              break;
            case d.CloseCurlyBrace:
              U = A ? Q.None : Q.ForceTermination;
              break;
            case d.SingleQuote:
            case d.DoubleQuote:
            case d.BackTick:
              f === R ? U = Q.ForceTermination : f === d.SingleQuote || f === d.DoubleQuote || f === d.BackTick ? U = Q.None : U = Q.ForceTermination;
              break;
            case d.Asterisk:
              U = f === d.Asterisk ? Q.ForceTermination : Q.None;
              break;
            case d.Pipe:
              U = f === d.Pipe ? Q.ForceTermination : Q.None;
              break;
            case d.Space:
              U = N ? Q.None : Q.ForceTermination;
              break;
            default:
              U = i.get(R);
          }
          U === Q.ForceTermination && (s.push(Zt._createLink(i, u, r, h, c)), x = !0);
        } else if (g === P.End) {
          let U;
          R === d.OpenSquareBracket ? (L = !0, U = Q.None) : U = i.get(R), U === Q.ForceTermination ? x = !0 : g = P.Accept;
        } else
          g = n.nextState(g, R), g === P.Invalid && (x = !0);
        x && (g = P.Start, b = !1, L = !1, A = !1, h = c + 1, f = R), c++;
      }
      g === P.Accept && s.push(Zt._createLink(i, u, r, h, o));
    }
    return s;
  }
}
function L1(e) {
  return !e || typeof e.getLineCount != "function" || typeof e.getLineContent != "function" ? [] : Zt.computeLinks(e);
}
const an = class an {
  constructor() {
    this._defaultValueSet = [
      ["true", "false"],
      ["True", "False"],
      ["Private", "Public", "Friend", "ReadOnly", "Partial", "Protected", "WriteOnly"],
      ["public", "protected", "private"]
    ];
  }
  navigateValueSet(t, n, i, s, r) {
    if (t && n) {
      const a = this.doNavigateValueSet(n, r);
      if (a)
        return {
          range: t,
          value: a
        };
    }
    if (i && s) {
      const a = this.doNavigateValueSet(s, r);
      if (a)
        return {
          range: i,
          value: a
        };
    }
    return null;
  }
  doNavigateValueSet(t, n) {
    const i = this.numberReplace(t, n);
    return i !== null ? i : this.textReplace(t, n);
  }
  numberReplace(t, n) {
    const i = Math.pow(10, t.length - (t.lastIndexOf(".") + 1));
    let s = Number(t);
    const r = parseFloat(t);
    return !isNaN(s) && !isNaN(r) && s === r ? s === 0 && !n ? null : (s = Math.floor(s * i), s += n ? i : -i, String(s / i)) : null;
  }
  textReplace(t, n) {
    return this.valueSetsReplace(this._defaultValueSet, t, n);
  }
  valueSetsReplace(t, n, i) {
    let s = null;
    for (let r = 0, a = t.length; s === null && r < a; r++)
      s = this.valueSetReplace(t[r], n, i);
    return s;
  }
  valueSetReplace(t, n, i) {
    let s = t.indexOf(n);
    return s >= 0 ? (s += i ? 1 : -1, s < 0 ? s = t.length - 1 : s %= t.length, t[s]) : null;
  }
};
an.INSTANCE = new an();
let Tn = an;
var m;
(function(e) {
  e[e.DependsOnKbLayout = -1] = "DependsOnKbLayout", e[e.Unknown = 0] = "Unknown", e[e.Backspace = 1] = "Backspace", e[e.Tab = 2] = "Tab", e[e.Enter = 3] = "Enter", e[e.Shift = 4] = "Shift", e[e.Ctrl = 5] = "Ctrl", e[e.Alt = 6] = "Alt", e[e.PauseBreak = 7] = "PauseBreak", e[e.CapsLock = 8] = "CapsLock", e[e.Escape = 9] = "Escape", e[e.Space = 10] = "Space", e[e.PageUp = 11] = "PageUp", e[e.PageDown = 12] = "PageDown", e[e.End = 13] = "End", e[e.Home = 14] = "Home", e[e.LeftArrow = 15] = "LeftArrow", e[e.UpArrow = 16] = "UpArrow", e[e.RightArrow = 17] = "RightArrow", e[e.DownArrow = 18] = "DownArrow", e[e.Insert = 19] = "Insert", e[e.Delete = 20] = "Delete", e[e.Digit0 = 21] = "Digit0", e[e.Digit1 = 22] = "Digit1", e[e.Digit2 = 23] = "Digit2", e[e.Digit3 = 24] = "Digit3", e[e.Digit4 = 25] = "Digit4", e[e.Digit5 = 26] = "Digit5", e[e.Digit6 = 27] = "Digit6", e[e.Digit7 = 28] = "Digit7", e[e.Digit8 = 29] = "Digit8", e[e.Digit9 = 30] = "Digit9", e[e.KeyA = 31] = "KeyA", e[e.KeyB = 32] = "KeyB", e[e.KeyC = 33] = "KeyC", e[e.KeyD = 34] = "KeyD", e[e.KeyE = 35] = "KeyE", e[e.KeyF = 36] = "KeyF", e[e.KeyG = 37] = "KeyG", e[e.KeyH = 38] = "KeyH", e[e.KeyI = 39] = "KeyI", e[e.KeyJ = 40] = "KeyJ", e[e.KeyK = 41] = "KeyK", e[e.KeyL = 42] = "KeyL", e[e.KeyM = 43] = "KeyM", e[e.KeyN = 44] = "KeyN", e[e.KeyO = 45] = "KeyO", e[e.KeyP = 46] = "KeyP", e[e.KeyQ = 47] = "KeyQ", e[e.KeyR = 48] = "KeyR", e[e.KeyS = 49] = "KeyS", e[e.KeyT = 50] = "KeyT", e[e.KeyU = 51] = "KeyU", e[e.KeyV = 52] = "KeyV", e[e.KeyW = 53] = "KeyW", e[e.KeyX = 54] = "KeyX", e[e.KeyY = 55] = "KeyY", e[e.KeyZ = 56] = "KeyZ", e[e.Meta = 57] = "Meta", e[e.ContextMenu = 58] = "ContextMenu", e[e.F1 = 59] = "F1", e[e.F2 = 60] = "F2", e[e.F3 = 61] = "F3", e[e.F4 = 62] = "F4", e[e.F5 = 63] = "F5", e[e.F6 = 64] = "F6", e[e.F7 = 65] = "F7", e[e.F8 = 66] = "F8", e[e.F9 = 67] = "F9", e[e.F10 = 68] = "F10", e[e.F11 = 69] = "F11", e[e.F12 = 70] = "F12", e[e.F13 = 71] = "F13", e[e.F14 = 72] = "F14", e[e.F15 = 73] = "F15", e[e.F16 = 74] = "F16", e[e.F17 = 75] = "F17", e[e.F18 = 76] = "F18", e[e.F19 = 77] = "F19", e[e.F20 = 78] = "F20", e[e.F21 = 79] = "F21", e[e.F22 = 80] = "F22", e[e.F23 = 81] = "F23", e[e.F24 = 82] = "F24", e[e.NumLock = 83] = "NumLock", e[e.ScrollLock = 84] = "ScrollLock", e[e.Semicolon = 85] = "Semicolon", e[e.Equal = 86] = "Equal", e[e.Comma = 87] = "Comma", e[e.Minus = 88] = "Minus", e[e.Period = 89] = "Period", e[e.Slash = 90] = "Slash", e[e.Backquote = 91] = "Backquote", e[e.BracketLeft = 92] = "BracketLeft", e[e.Backslash = 93] = "Backslash", e[e.BracketRight = 94] = "BracketRight", e[e.Quote = 95] = "Quote", e[e.OEM_8 = 96] = "OEM_8", e[e.IntlBackslash = 97] = "IntlBackslash", e[e.Numpad0 = 98] = "Numpad0", e[e.Numpad1 = 99] = "Numpad1", e[e.Numpad2 = 100] = "Numpad2", e[e.Numpad3 = 101] = "Numpad3", e[e.Numpad4 = 102] = "Numpad4", e[e.Numpad5 = 103] = "Numpad5", e[e.Numpad6 = 104] = "Numpad6", e[e.Numpad7 = 105] = "Numpad7", e[e.Numpad8 = 106] = "Numpad8", e[e.Numpad9 = 107] = "Numpad9", e[e.NumpadMultiply = 108] = "NumpadMultiply", e[e.NumpadAdd = 109] = "NumpadAdd", e[e.NUMPAD_SEPARATOR = 110] = "NUMPAD_SEPARATOR", e[e.NumpadSubtract = 111] = "NumpadSubtract", e[e.NumpadDecimal = 112] = "NumpadDecimal", e[e.NumpadDivide = 113] = "NumpadDivide", e[e.KEY_IN_COMPOSITION = 114] = "KEY_IN_COMPOSITION", e[e.ABNT_C1 = 115] = "ABNT_C1", e[e.ABNT_C2 = 116] = "ABNT_C2", e[e.AudioVolumeMute = 117] = "AudioVolumeMute", e[e.AudioVolumeUp = 118] = "AudioVolumeUp", e[e.AudioVolumeDown = 119] = "AudioVolumeDown", e[e.BrowserSearch = 120] = "BrowserSearch", e[e.BrowserHome = 121] = "BrowserHome", e[e.BrowserBack = 122] = "BrowserBack", e[e.BrowserForward = 123] = "BrowserForward", e[e.MediaTrackNext = 124] = "MediaTrackNext", e[e.MediaTrackPrevious = 125] = "MediaTrackPrevious", e[e.MediaStop = 126] = "MediaStop", e[e.MediaPlayPause = 127] = "MediaPlayPause", e[e.LaunchMediaPlayer = 128] = "LaunchMediaPlayer", e[e.LaunchMail = 129] = "LaunchMail", e[e.LaunchApp2 = 130] = "LaunchApp2", e[e.Clear = 131] = "Clear", e[e.MAX_VALUE = 132] = "MAX_VALUE";
})(m || (m = {}));
var _;
(function(e) {
  e[e.DependsOnKbLayout = -1] = "DependsOnKbLayout", e[e.None = 0] = "None", e[e.Hyper = 1] = "Hyper", e[e.Super = 2] = "Super", e[e.Fn = 3] = "Fn", e[e.FnLock = 4] = "FnLock", e[e.Suspend = 5] = "Suspend", e[e.Resume = 6] = "Resume", e[e.Turbo = 7] = "Turbo", e[e.Sleep = 8] = "Sleep", e[e.WakeUp = 9] = "WakeUp", e[e.KeyA = 10] = "KeyA", e[e.KeyB = 11] = "KeyB", e[e.KeyC = 12] = "KeyC", e[e.KeyD = 13] = "KeyD", e[e.KeyE = 14] = "KeyE", e[e.KeyF = 15] = "KeyF", e[e.KeyG = 16] = "KeyG", e[e.KeyH = 17] = "KeyH", e[e.KeyI = 18] = "KeyI", e[e.KeyJ = 19] = "KeyJ", e[e.KeyK = 20] = "KeyK", e[e.KeyL = 21] = "KeyL", e[e.KeyM = 22] = "KeyM", e[e.KeyN = 23] = "KeyN", e[e.KeyO = 24] = "KeyO", e[e.KeyP = 25] = "KeyP", e[e.KeyQ = 26] = "KeyQ", e[e.KeyR = 27] = "KeyR", e[e.KeyS = 28] = "KeyS", e[e.KeyT = 29] = "KeyT", e[e.KeyU = 30] = "KeyU", e[e.KeyV = 31] = "KeyV", e[e.KeyW = 32] = "KeyW", e[e.KeyX = 33] = "KeyX", e[e.KeyY = 34] = "KeyY", e[e.KeyZ = 35] = "KeyZ", e[e.Digit1 = 36] = "Digit1", e[e.Digit2 = 37] = "Digit2", e[e.Digit3 = 38] = "Digit3", e[e.Digit4 = 39] = "Digit4", e[e.Digit5 = 40] = "Digit5", e[e.Digit6 = 41] = "Digit6", e[e.Digit7 = 42] = "Digit7", e[e.Digit8 = 43] = "Digit8", e[e.Digit9 = 44] = "Digit9", e[e.Digit0 = 45] = "Digit0", e[e.Enter = 46] = "Enter", e[e.Escape = 47] = "Escape", e[e.Backspace = 48] = "Backspace", e[e.Tab = 49] = "Tab", e[e.Space = 50] = "Space", e[e.Minus = 51] = "Minus", e[e.Equal = 52] = "Equal", e[e.BracketLeft = 53] = "BracketLeft", e[e.BracketRight = 54] = "BracketRight", e[e.Backslash = 55] = "Backslash", e[e.IntlHash = 56] = "IntlHash", e[e.Semicolon = 57] = "Semicolon", e[e.Quote = 58] = "Quote", e[e.Backquote = 59] = "Backquote", e[e.Comma = 60] = "Comma", e[e.Period = 61] = "Period", e[e.Slash = 62] = "Slash", e[e.CapsLock = 63] = "CapsLock", e[e.F1 = 64] = "F1", e[e.F2 = 65] = "F2", e[e.F3 = 66] = "F3", e[e.F4 = 67] = "F4", e[e.F5 = 68] = "F5", e[e.F6 = 69] = "F6", e[e.F7 = 70] = "F7", e[e.F8 = 71] = "F8", e[e.F9 = 72] = "F9", e[e.F10 = 73] = "F10", e[e.F11 = 74] = "F11", e[e.F12 = 75] = "F12", e[e.PrintScreen = 76] = "PrintScreen", e[e.ScrollLock = 77] = "ScrollLock", e[e.Pause = 78] = "Pause", e[e.Insert = 79] = "Insert", e[e.Home = 80] = "Home", e[e.PageUp = 81] = "PageUp", e[e.Delete = 82] = "Delete", e[e.End = 83] = "End", e[e.PageDown = 84] = "PageDown", e[e.ArrowRight = 85] = "ArrowRight", e[e.ArrowLeft = 86] = "ArrowLeft", e[e.ArrowDown = 87] = "ArrowDown", e[e.ArrowUp = 88] = "ArrowUp", e[e.NumLock = 89] = "NumLock", e[e.NumpadDivide = 90] = "NumpadDivide", e[e.NumpadMultiply = 91] = "NumpadMultiply", e[e.NumpadSubtract = 92] = "NumpadSubtract", e[e.NumpadAdd = 93] = "NumpadAdd", e[e.NumpadEnter = 94] = "NumpadEnter", e[e.Numpad1 = 95] = "Numpad1", e[e.Numpad2 = 96] = "Numpad2", e[e.Numpad3 = 97] = "Numpad3", e[e.Numpad4 = 98] = "Numpad4", e[e.Numpad5 = 99] = "Numpad5", e[e.Numpad6 = 100] = "Numpad6", e[e.Numpad7 = 101] = "Numpad7", e[e.Numpad8 = 102] = "Numpad8", e[e.Numpad9 = 103] = "Numpad9", e[e.Numpad0 = 104] = "Numpad0", e[e.NumpadDecimal = 105] = "NumpadDecimal", e[e.IntlBackslash = 106] = "IntlBackslash", e[e.ContextMenu = 107] = "ContextMenu", e[e.Power = 108] = "Power", e[e.NumpadEqual = 109] = "NumpadEqual", e[e.F13 = 110] = "F13", e[e.F14 = 111] = "F14", e[e.F15 = 112] = "F15", e[e.F16 = 113] = "F16", e[e.F17 = 114] = "F17", e[e.F18 = 115] = "F18", e[e.F19 = 116] = "F19", e[e.F20 = 117] = "F20", e[e.F21 = 118] = "F21", e[e.F22 = 119] = "F22", e[e.F23 = 120] = "F23", e[e.F24 = 121] = "F24", e[e.Open = 122] = "Open", e[e.Help = 123] = "Help", e[e.Select = 124] = "Select", e[e.Again = 125] = "Again", e[e.Undo = 126] = "Undo", e[e.Cut = 127] = "Cut", e[e.Copy = 128] = "Copy", e[e.Paste = 129] = "Paste", e[e.Find = 130] = "Find", e[e.AudioVolumeMute = 131] = "AudioVolumeMute", e[e.AudioVolumeUp = 132] = "AudioVolumeUp", e[e.AudioVolumeDown = 133] = "AudioVolumeDown", e[e.NumpadComma = 134] = "NumpadComma", e[e.IntlRo = 135] = "IntlRo", e[e.KanaMode = 136] = "KanaMode", e[e.IntlYen = 137] = "IntlYen", e[e.Convert = 138] = "Convert", e[e.NonConvert = 139] = "NonConvert", e[e.Lang1 = 140] = "Lang1", e[e.Lang2 = 141] = "Lang2", e[e.Lang3 = 142] = "Lang3", e[e.Lang4 = 143] = "Lang4", e[e.Lang5 = 144] = "Lang5", e[e.Abort = 145] = "Abort", e[e.Props = 146] = "Props", e[e.NumpadParenLeft = 147] = "NumpadParenLeft", e[e.NumpadParenRight = 148] = "NumpadParenRight", e[e.NumpadBackspace = 149] = "NumpadBackspace", e[e.NumpadMemoryStore = 150] = "NumpadMemoryStore", e[e.NumpadMemoryRecall = 151] = "NumpadMemoryRecall", e[e.NumpadMemoryClear = 152] = "NumpadMemoryClear", e[e.NumpadMemoryAdd = 153] = "NumpadMemoryAdd", e[e.NumpadMemorySubtract = 154] = "NumpadMemorySubtract", e[e.NumpadClear = 155] = "NumpadClear", e[e.NumpadClearEntry = 156] = "NumpadClearEntry", e[e.ControlLeft = 157] = "ControlLeft", e[e.ShiftLeft = 158] = "ShiftLeft", e[e.AltLeft = 159] = "AltLeft", e[e.MetaLeft = 160] = "MetaLeft", e[e.ControlRight = 161] = "ControlRight", e[e.ShiftRight = 162] = "ShiftRight", e[e.AltRight = 163] = "AltRight", e[e.MetaRight = 164] = "MetaRight", e[e.BrightnessUp = 165] = "BrightnessUp", e[e.BrightnessDown = 166] = "BrightnessDown", e[e.MediaPlay = 167] = "MediaPlay", e[e.MediaRecord = 168] = "MediaRecord", e[e.MediaFastForward = 169] = "MediaFastForward", e[e.MediaRewind = 170] = "MediaRewind", e[e.MediaTrackNext = 171] = "MediaTrackNext", e[e.MediaTrackPrevious = 172] = "MediaTrackPrevious", e[e.MediaStop = 173] = "MediaStop", e[e.Eject = 174] = "Eject", e[e.MediaPlayPause = 175] = "MediaPlayPause", e[e.MediaSelect = 176] = "MediaSelect", e[e.LaunchMail = 177] = "LaunchMail", e[e.LaunchApp2 = 178] = "LaunchApp2", e[e.LaunchApp1 = 179] = "LaunchApp1", e[e.SelectTask = 180] = "SelectTask", e[e.LaunchScreenSaver = 181] = "LaunchScreenSaver", e[e.BrowserSearch = 182] = "BrowserSearch", e[e.BrowserHome = 183] = "BrowserHome", e[e.BrowserBack = 184] = "BrowserBack", e[e.BrowserForward = 185] = "BrowserForward", e[e.BrowserStop = 186] = "BrowserStop", e[e.BrowserRefresh = 187] = "BrowserRefresh", e[e.BrowserFavorites = 188] = "BrowserFavorites", e[e.ZoomToggle = 189] = "ZoomToggle", e[e.MailReply = 190] = "MailReply", e[e.MailForward = 191] = "MailForward", e[e.MailSend = 192] = "MailSend", e[e.MAX_VALUE = 193] = "MAX_VALUE";
})(_ || (_ = {}));
class jn {
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
    return this._strToKeyCode[t.toLowerCase()] || m.Unknown;
  }
}
const qt = new jn(), Mn = new jn(), Fn = new jn(), N1 = new Array(230), p1 = /* @__PURE__ */ Object.create(null), w1 = /* @__PURE__ */ Object.create(null), kn = [];
for (let e = 0; e <= _.MAX_VALUE; e++)
  m.DependsOnKbLayout;
for (let e = 0; e <= m.MAX_VALUE; e++)
  kn[e] = _.DependsOnKbLayout;
(function() {
  const e = "", t = [
    [1, _.None, "None", m.Unknown, "unknown", 0, "VK_UNKNOWN", e, e],
    [1, _.Hyper, "Hyper", m.Unknown, e, 0, e, e, e],
    [1, _.Super, "Super", m.Unknown, e, 0, e, e, e],
    [1, _.Fn, "Fn", m.Unknown, e, 0, e, e, e],
    [1, _.FnLock, "FnLock", m.Unknown, e, 0, e, e, e],
    [1, _.Suspend, "Suspend", m.Unknown, e, 0, e, e, e],
    [1, _.Resume, "Resume", m.Unknown, e, 0, e, e, e],
    [1, _.Turbo, "Turbo", m.Unknown, e, 0, e, e, e],
    [1, _.Sleep, "Sleep", m.Unknown, e, 0, "VK_SLEEP", e, e],
    [1, _.WakeUp, "WakeUp", m.Unknown, e, 0, e, e, e],
    [0, _.KeyA, "KeyA", m.KeyA, "A", 65, "VK_A", e, e],
    [0, _.KeyB, "KeyB", m.KeyB, "B", 66, "VK_B", e, e],
    [0, _.KeyC, "KeyC", m.KeyC, "C", 67, "VK_C", e, e],
    [0, _.KeyD, "KeyD", m.KeyD, "D", 68, "VK_D", e, e],
    [0, _.KeyE, "KeyE", m.KeyE, "E", 69, "VK_E", e, e],
    [0, _.KeyF, "KeyF", m.KeyF, "F", 70, "VK_F", e, e],
    [0, _.KeyG, "KeyG", m.KeyG, "G", 71, "VK_G", e, e],
    [0, _.KeyH, "KeyH", m.KeyH, "H", 72, "VK_H", e, e],
    [0, _.KeyI, "KeyI", m.KeyI, "I", 73, "VK_I", e, e],
    [0, _.KeyJ, "KeyJ", m.KeyJ, "J", 74, "VK_J", e, e],
    [0, _.KeyK, "KeyK", m.KeyK, "K", 75, "VK_K", e, e],
    [0, _.KeyL, "KeyL", m.KeyL, "L", 76, "VK_L", e, e],
    [0, _.KeyM, "KeyM", m.KeyM, "M", 77, "VK_M", e, e],
    [0, _.KeyN, "KeyN", m.KeyN, "N", 78, "VK_N", e, e],
    [0, _.KeyO, "KeyO", m.KeyO, "O", 79, "VK_O", e, e],
    [0, _.KeyP, "KeyP", m.KeyP, "P", 80, "VK_P", e, e],
    [0, _.KeyQ, "KeyQ", m.KeyQ, "Q", 81, "VK_Q", e, e],
    [0, _.KeyR, "KeyR", m.KeyR, "R", 82, "VK_R", e, e],
    [0, _.KeyS, "KeyS", m.KeyS, "S", 83, "VK_S", e, e],
    [0, _.KeyT, "KeyT", m.KeyT, "T", 84, "VK_T", e, e],
    [0, _.KeyU, "KeyU", m.KeyU, "U", 85, "VK_U", e, e],
    [0, _.KeyV, "KeyV", m.KeyV, "V", 86, "VK_V", e, e],
    [0, _.KeyW, "KeyW", m.KeyW, "W", 87, "VK_W", e, e],
    [0, _.KeyX, "KeyX", m.KeyX, "X", 88, "VK_X", e, e],
    [0, _.KeyY, "KeyY", m.KeyY, "Y", 89, "VK_Y", e, e],
    [0, _.KeyZ, "KeyZ", m.KeyZ, "Z", 90, "VK_Z", e, e],
    [0, _.Digit1, "Digit1", m.Digit1, "1", 49, "VK_1", e, e],
    [0, _.Digit2, "Digit2", m.Digit2, "2", 50, "VK_2", e, e],
    [0, _.Digit3, "Digit3", m.Digit3, "3", 51, "VK_3", e, e],
    [0, _.Digit4, "Digit4", m.Digit4, "4", 52, "VK_4", e, e],
    [0, _.Digit5, "Digit5", m.Digit5, "5", 53, "VK_5", e, e],
    [0, _.Digit6, "Digit6", m.Digit6, "6", 54, "VK_6", e, e],
    [0, _.Digit7, "Digit7", m.Digit7, "7", 55, "VK_7", e, e],
    [0, _.Digit8, "Digit8", m.Digit8, "8", 56, "VK_8", e, e],
    [0, _.Digit9, "Digit9", m.Digit9, "9", 57, "VK_9", e, e],
    [0, _.Digit0, "Digit0", m.Digit0, "0", 48, "VK_0", e, e],
    [1, _.Enter, "Enter", m.Enter, "Enter", 13, "VK_RETURN", e, e],
    [1, _.Escape, "Escape", m.Escape, "Escape", 27, "VK_ESCAPE", e, e],
    [1, _.Backspace, "Backspace", m.Backspace, "Backspace", 8, "VK_BACK", e, e],
    [1, _.Tab, "Tab", m.Tab, "Tab", 9, "VK_TAB", e, e],
    [1, _.Space, "Space", m.Space, "Space", 32, "VK_SPACE", e, e],
    [0, _.Minus, "Minus", m.Minus, "-", 189, "VK_OEM_MINUS", "-", "OEM_MINUS"],
    [0, _.Equal, "Equal", m.Equal, "=", 187, "VK_OEM_PLUS", "=", "OEM_PLUS"],
    [0, _.BracketLeft, "BracketLeft", m.BracketLeft, "[", 219, "VK_OEM_4", "[", "OEM_4"],
    [0, _.BracketRight, "BracketRight", m.BracketRight, "]", 221, "VK_OEM_6", "]", "OEM_6"],
    [0, _.Backslash, "Backslash", m.Backslash, "\\", 220, "VK_OEM_5", "\\", "OEM_5"],
    [0, _.IntlHash, "IntlHash", m.Unknown, e, 0, e, e, e],
    [0, _.Semicolon, "Semicolon", m.Semicolon, ";", 186, "VK_OEM_1", ";", "OEM_1"],
    [0, _.Quote, "Quote", m.Quote, "'", 222, "VK_OEM_7", "'", "OEM_7"],
    [0, _.Backquote, "Backquote", m.Backquote, "`", 192, "VK_OEM_3", "`", "OEM_3"],
    [0, _.Comma, "Comma", m.Comma, ",", 188, "VK_OEM_COMMA", ",", "OEM_COMMA"],
    [0, _.Period, "Period", m.Period, ".", 190, "VK_OEM_PERIOD", ".", "OEM_PERIOD"],
    [0, _.Slash, "Slash", m.Slash, "/", 191, "VK_OEM_2", "/", "OEM_2"],
    [1, _.CapsLock, "CapsLock", m.CapsLock, "CapsLock", 20, "VK_CAPITAL", e, e],
    [1, _.F1, "F1", m.F1, "F1", 112, "VK_F1", e, e],
    [1, _.F2, "F2", m.F2, "F2", 113, "VK_F2", e, e],
    [1, _.F3, "F3", m.F3, "F3", 114, "VK_F3", e, e],
    [1, _.F4, "F4", m.F4, "F4", 115, "VK_F4", e, e],
    [1, _.F5, "F5", m.F5, "F5", 116, "VK_F5", e, e],
    [1, _.F6, "F6", m.F6, "F6", 117, "VK_F6", e, e],
    [1, _.F7, "F7", m.F7, "F7", 118, "VK_F7", e, e],
    [1, _.F8, "F8", m.F8, "F8", 119, "VK_F8", e, e],
    [1, _.F9, "F9", m.F9, "F9", 120, "VK_F9", e, e],
    [1, _.F10, "F10", m.F10, "F10", 121, "VK_F10", e, e],
    [1, _.F11, "F11", m.F11, "F11", 122, "VK_F11", e, e],
    [1, _.F12, "F12", m.F12, "F12", 123, "VK_F12", e, e],
    [1, _.PrintScreen, "PrintScreen", m.Unknown, e, 0, e, e, e],
    [1, _.ScrollLock, "ScrollLock", m.ScrollLock, "ScrollLock", 145, "VK_SCROLL", e, e],
    [1, _.Pause, "Pause", m.PauseBreak, "PauseBreak", 19, "VK_PAUSE", e, e],
    [1, _.Insert, "Insert", m.Insert, "Insert", 45, "VK_INSERT", e, e],
    [1, _.Home, "Home", m.Home, "Home", 36, "VK_HOME", e, e],
    [1, _.PageUp, "PageUp", m.PageUp, "PageUp", 33, "VK_PRIOR", e, e],
    [1, _.Delete, "Delete", m.Delete, "Delete", 46, "VK_DELETE", e, e],
    [1, _.End, "End", m.End, "End", 35, "VK_END", e, e],
    [1, _.PageDown, "PageDown", m.PageDown, "PageDown", 34, "VK_NEXT", e, e],
    [1, _.ArrowRight, "ArrowRight", m.RightArrow, "RightArrow", 39, "VK_RIGHT", "Right", e],
    [1, _.ArrowLeft, "ArrowLeft", m.LeftArrow, "LeftArrow", 37, "VK_LEFT", "Left", e],
    [1, _.ArrowDown, "ArrowDown", m.DownArrow, "DownArrow", 40, "VK_DOWN", "Down", e],
    [1, _.ArrowUp, "ArrowUp", m.UpArrow, "UpArrow", 38, "VK_UP", "Up", e],
    [1, _.NumLock, "NumLock", m.NumLock, "NumLock", 144, "VK_NUMLOCK", e, e],
    [1, _.NumpadDivide, "NumpadDivide", m.NumpadDivide, "NumPad_Divide", 111, "VK_DIVIDE", e, e],
    [1, _.NumpadMultiply, "NumpadMultiply", m.NumpadMultiply, "NumPad_Multiply", 106, "VK_MULTIPLY", e, e],
    [1, _.NumpadSubtract, "NumpadSubtract", m.NumpadSubtract, "NumPad_Subtract", 109, "VK_SUBTRACT", e, e],
    [1, _.NumpadAdd, "NumpadAdd", m.NumpadAdd, "NumPad_Add", 107, "VK_ADD", e, e],
    [1, _.NumpadEnter, "NumpadEnter", m.Enter, e, 0, e, e, e],
    [1, _.Numpad1, "Numpad1", m.Numpad1, "NumPad1", 97, "VK_NUMPAD1", e, e],
    [1, _.Numpad2, "Numpad2", m.Numpad2, "NumPad2", 98, "VK_NUMPAD2", e, e],
    [1, _.Numpad3, "Numpad3", m.Numpad3, "NumPad3", 99, "VK_NUMPAD3", e, e],
    [1, _.Numpad4, "Numpad4", m.Numpad4, "NumPad4", 100, "VK_NUMPAD4", e, e],
    [1, _.Numpad5, "Numpad5", m.Numpad5, "NumPad5", 101, "VK_NUMPAD5", e, e],
    [1, _.Numpad6, "Numpad6", m.Numpad6, "NumPad6", 102, "VK_NUMPAD6", e, e],
    [1, _.Numpad7, "Numpad7", m.Numpad7, "NumPad7", 103, "VK_NUMPAD7", e, e],
    [1, _.Numpad8, "Numpad8", m.Numpad8, "NumPad8", 104, "VK_NUMPAD8", e, e],
    [1, _.Numpad9, "Numpad9", m.Numpad9, "NumPad9", 105, "VK_NUMPAD9", e, e],
    [1, _.Numpad0, "Numpad0", m.Numpad0, "NumPad0", 96, "VK_NUMPAD0", e, e],
    [1, _.NumpadDecimal, "NumpadDecimal", m.NumpadDecimal, "NumPad_Decimal", 110, "VK_DECIMAL", e, e],
    [0, _.IntlBackslash, "IntlBackslash", m.IntlBackslash, "OEM_102", 226, "VK_OEM_102", e, e],
    [1, _.ContextMenu, "ContextMenu", m.ContextMenu, "ContextMenu", 93, e, e, e],
    [1, _.Power, "Power", m.Unknown, e, 0, e, e, e],
    [1, _.NumpadEqual, "NumpadEqual", m.Unknown, e, 0, e, e, e],
    [1, _.F13, "F13", m.F13, "F13", 124, "VK_F13", e, e],
    [1, _.F14, "F14", m.F14, "F14", 125, "VK_F14", e, e],
    [1, _.F15, "F15", m.F15, "F15", 126, "VK_F15", e, e],
    [1, _.F16, "F16", m.F16, "F16", 127, "VK_F16", e, e],
    [1, _.F17, "F17", m.F17, "F17", 128, "VK_F17", e, e],
    [1, _.F18, "F18", m.F18, "F18", 129, "VK_F18", e, e],
    [1, _.F19, "F19", m.F19, "F19", 130, "VK_F19", e, e],
    [1, _.F20, "F20", m.F20, "F20", 131, "VK_F20", e, e],
    [1, _.F21, "F21", m.F21, "F21", 132, "VK_F21", e, e],
    [1, _.F22, "F22", m.F22, "F22", 133, "VK_F22", e, e],
    [1, _.F23, "F23", m.F23, "F23", 134, "VK_F23", e, e],
    [1, _.F24, "F24", m.F24, "F24", 135, "VK_F24", e, e],
    [1, _.Open, "Open", m.Unknown, e, 0, e, e, e],
    [1, _.Help, "Help", m.Unknown, e, 0, e, e, e],
    [1, _.Select, "Select", m.Unknown, e, 0, e, e, e],
    [1, _.Again, "Again", m.Unknown, e, 0, e, e, e],
    [1, _.Undo, "Undo", m.Unknown, e, 0, e, e, e],
    [1, _.Cut, "Cut", m.Unknown, e, 0, e, e, e],
    [1, _.Copy, "Copy", m.Unknown, e, 0, e, e, e],
    [1, _.Paste, "Paste", m.Unknown, e, 0, e, e, e],
    [1, _.Find, "Find", m.Unknown, e, 0, e, e, e],
    [1, _.AudioVolumeMute, "AudioVolumeMute", m.AudioVolumeMute, "AudioVolumeMute", 173, "VK_VOLUME_MUTE", e, e],
    [1, _.AudioVolumeUp, "AudioVolumeUp", m.AudioVolumeUp, "AudioVolumeUp", 175, "VK_VOLUME_UP", e, e],
    [1, _.AudioVolumeDown, "AudioVolumeDown", m.AudioVolumeDown, "AudioVolumeDown", 174, "VK_VOLUME_DOWN", e, e],
    [1, _.NumpadComma, "NumpadComma", m.NUMPAD_SEPARATOR, "NumPad_Separator", 108, "VK_SEPARATOR", e, e],
    [0, _.IntlRo, "IntlRo", m.ABNT_C1, "ABNT_C1", 193, "VK_ABNT_C1", e, e],
    [1, _.KanaMode, "KanaMode", m.Unknown, e, 0, e, e, e],
    [0, _.IntlYen, "IntlYen", m.Unknown, e, 0, e, e, e],
    [1, _.Convert, "Convert", m.Unknown, e, 0, e, e, e],
    [1, _.NonConvert, "NonConvert", m.Unknown, e, 0, e, e, e],
    [1, _.Lang1, "Lang1", m.Unknown, e, 0, e, e, e],
    [1, _.Lang2, "Lang2", m.Unknown, e, 0, e, e, e],
    [1, _.Lang3, "Lang3", m.Unknown, e, 0, e, e, e],
    [1, _.Lang4, "Lang4", m.Unknown, e, 0, e, e, e],
    [1, _.Lang5, "Lang5", m.Unknown, e, 0, e, e, e],
    [1, _.Abort, "Abort", m.Unknown, e, 0, e, e, e],
    [1, _.Props, "Props", m.Unknown, e, 0, e, e, e],
    [1, _.NumpadParenLeft, "NumpadParenLeft", m.Unknown, e, 0, e, e, e],
    [1, _.NumpadParenRight, "NumpadParenRight", m.Unknown, e, 0, e, e, e],
    [1, _.NumpadBackspace, "NumpadBackspace", m.Unknown, e, 0, e, e, e],
    [1, _.NumpadMemoryStore, "NumpadMemoryStore", m.Unknown, e, 0, e, e, e],
    [1, _.NumpadMemoryRecall, "NumpadMemoryRecall", m.Unknown, e, 0, e, e, e],
    [1, _.NumpadMemoryClear, "NumpadMemoryClear", m.Unknown, e, 0, e, e, e],
    [1, _.NumpadMemoryAdd, "NumpadMemoryAdd", m.Unknown, e, 0, e, e, e],
    [1, _.NumpadMemorySubtract, "NumpadMemorySubtract", m.Unknown, e, 0, e, e, e],
    [1, _.NumpadClear, "NumpadClear", m.Clear, "Clear", 12, "VK_CLEAR", e, e],
    [1, _.NumpadClearEntry, "NumpadClearEntry", m.Unknown, e, 0, e, e, e],
    [1, _.None, e, m.Ctrl, "Ctrl", 17, "VK_CONTROL", e, e],
    [1, _.None, e, m.Shift, "Shift", 16, "VK_SHIFT", e, e],
    [1, _.None, e, m.Alt, "Alt", 18, "VK_MENU", e, e],
    [1, _.None, e, m.Meta, "Meta", 91, "VK_COMMAND", e, e],
    [1, _.ControlLeft, "ControlLeft", m.Ctrl, e, 0, "VK_LCONTROL", e, e],
    [1, _.ShiftLeft, "ShiftLeft", m.Shift, e, 0, "VK_LSHIFT", e, e],
    [1, _.AltLeft, "AltLeft", m.Alt, e, 0, "VK_LMENU", e, e],
    [1, _.MetaLeft, "MetaLeft", m.Meta, e, 0, "VK_LWIN", e, e],
    [1, _.ControlRight, "ControlRight", m.Ctrl, e, 0, "VK_RCONTROL", e, e],
    [1, _.ShiftRight, "ShiftRight", m.Shift, e, 0, "VK_RSHIFT", e, e],
    [1, _.AltRight, "AltRight", m.Alt, e, 0, "VK_RMENU", e, e],
    [1, _.MetaRight, "MetaRight", m.Meta, e, 0, "VK_RWIN", e, e],
    [1, _.BrightnessUp, "BrightnessUp", m.Unknown, e, 0, e, e, e],
    [1, _.BrightnessDown, "BrightnessDown", m.Unknown, e, 0, e, e, e],
    [1, _.MediaPlay, "MediaPlay", m.Unknown, e, 0, e, e, e],
    [1, _.MediaRecord, "MediaRecord", m.Unknown, e, 0, e, e, e],
    [1, _.MediaFastForward, "MediaFastForward", m.Unknown, e, 0, e, e, e],
    [1, _.MediaRewind, "MediaRewind", m.Unknown, e, 0, e, e, e],
    [1, _.MediaTrackNext, "MediaTrackNext", m.MediaTrackNext, "MediaTrackNext", 176, "VK_MEDIA_NEXT_TRACK", e, e],
    [1, _.MediaTrackPrevious, "MediaTrackPrevious", m.MediaTrackPrevious, "MediaTrackPrevious", 177, "VK_MEDIA_PREV_TRACK", e, e],
    [1, _.MediaStop, "MediaStop", m.MediaStop, "MediaStop", 178, "VK_MEDIA_STOP", e, e],
    [1, _.Eject, "Eject", m.Unknown, e, 0, e, e, e],
    [1, _.MediaPlayPause, "MediaPlayPause", m.MediaPlayPause, "MediaPlayPause", 179, "VK_MEDIA_PLAY_PAUSE", e, e],
    [1, _.MediaSelect, "MediaSelect", m.LaunchMediaPlayer, "LaunchMediaPlayer", 181, "VK_MEDIA_LAUNCH_MEDIA_SELECT", e, e],
    [1, _.LaunchMail, "LaunchMail", m.LaunchMail, "LaunchMail", 180, "VK_MEDIA_LAUNCH_MAIL", e, e],
    [1, _.LaunchApp2, "LaunchApp2", m.LaunchApp2, "LaunchApp2", 183, "VK_MEDIA_LAUNCH_APP2", e, e],
    [1, _.LaunchApp1, "LaunchApp1", m.Unknown, e, 0, "VK_MEDIA_LAUNCH_APP1", e, e],
    [1, _.SelectTask, "SelectTask", m.Unknown, e, 0, e, e, e],
    [1, _.LaunchScreenSaver, "LaunchScreenSaver", m.Unknown, e, 0, e, e, e],
    [1, _.BrowserSearch, "BrowserSearch", m.BrowserSearch, "BrowserSearch", 170, "VK_BROWSER_SEARCH", e, e],
    [1, _.BrowserHome, "BrowserHome", m.BrowserHome, "BrowserHome", 172, "VK_BROWSER_HOME", e, e],
    [1, _.BrowserBack, "BrowserBack", m.BrowserBack, "BrowserBack", 166, "VK_BROWSER_BACK", e, e],
    [1, _.BrowserForward, "BrowserForward", m.BrowserForward, "BrowserForward", 167, "VK_BROWSER_FORWARD", e, e],
    [1, _.BrowserStop, "BrowserStop", m.Unknown, e, 0, "VK_BROWSER_STOP", e, e],
    [1, _.BrowserRefresh, "BrowserRefresh", m.Unknown, e, 0, "VK_BROWSER_REFRESH", e, e],
    [1, _.BrowserFavorites, "BrowserFavorites", m.Unknown, e, 0, "VK_BROWSER_FAVORITES", e, e],
    [1, _.ZoomToggle, "ZoomToggle", m.Unknown, e, 0, e, e, e],
    [1, _.MailReply, "MailReply", m.Unknown, e, 0, e, e, e],
    [1, _.MailForward, "MailForward", m.Unknown, e, 0, e, e, e],
    [1, _.MailSend, "MailSend", m.Unknown, e, 0, e, e, e],
    [1, _.None, e, m.KEY_IN_COMPOSITION, "KeyInComposition", 229, e, e, e],
    [1, _.None, e, m.ABNT_C2, "ABNT_C2", 194, "VK_ABNT_C2", e, e],
    [1, _.None, e, m.OEM_8, "OEM_8", 223, "VK_OEM_8", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_KANA", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_HANGUL", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_JUNJA", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_FINAL", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_HANJA", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_KANJI", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_CONVERT", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_NONCONVERT", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_ACCEPT", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_MODECHANGE", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_SELECT", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_PRINT", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_EXECUTE", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_SNAPSHOT", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_HELP", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_APPS", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_PROCESSKEY", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_PACKET", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_DBE_SBCSCHAR", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_DBE_DBCSCHAR", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_ATTN", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_CRSEL", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_EXSEL", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_EREOF", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_PLAY", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_ZOOM", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_NONAME", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_PA1", e, e],
    [1, _.None, e, m.Unknown, e, 0, "VK_OEM_CLEAR", e, e]
  ], n = [], i = [];
  for (const s of t) {
    const [r, a, u, o, c, h, f, g, b] = s;
    if (i[a] || (i[a] = !0, p1[u] = a, w1[u.toLowerCase()] = a, r && o !== m.Unknown && o !== m.Enter && o !== m.Ctrl && o !== m.Shift && o !== m.Alt && o !== m.Meta && (kn[o] = a)), !n[o]) {
      if (n[o] = !0, !c)
        throw new Error(
          `String representation missing for key code ${o} around scan code ${u}`
        );
      qt.define(o, c), Mn.define(o, g || c), Fn.define(o, b || g || c);
    }
    h && (N1[h] = o);
  }
  kn[m.Enter] = _.Enter;
})();
var di;
(function(e) {
  function t(u) {
    return qt.keyCodeToStr(u);
  }
  e.toString = t;
  function n(u) {
    return qt.strToKeyCode(u);
  }
  e.fromString = n;
  function i(u) {
    return Mn.keyCodeToStr(u);
  }
  e.toUserSettingsUS = i;
  function s(u) {
    return Fn.keyCodeToStr(u);
  }
  e.toUserSettingsGeneral = s;
  function r(u) {
    return Mn.strToKeyCode(u) || Fn.strToKeyCode(u);
  }
  e.fromUserSettings = r;
  function a(u) {
    if (u >= m.Numpad0 && u <= m.NumpadDivide)
      return null;
    switch (u) {
      case m.UpArrow:
        return "Up";
      case m.DownArrow:
        return "Down";
      case m.LeftArrow:
        return "Left";
      case m.RightArrow:
        return "Right";
    }
    return qt.keyCodeToStr(u);
  }
  e.toElectronAccelerator = a;
})(di || (di = {}));
var nt;
(function(e) {
  e[e.CtrlCmd = 2048] = "CtrlCmd", e[e.Shift = 1024] = "Shift", e[e.Alt = 512] = "Alt", e[e.WinCtrl = 256] = "WinCtrl";
})(nt || (nt = {}));
function E1(e, t) {
  const n = (t & 65535) << 16 >>> 0;
  return (e | n) >>> 0;
}
var ye;
(function(e) {
  e[e.LTR = 0] = "LTR", e[e.RTL = 1] = "RTL";
})(ye || (ye = {}));
class ge extends S {
  constructor(t, n, i, s) {
    super(t, n, i, s), this.selectionStartLineNumber = t, this.selectionStartColumn = n, this.positionLineNumber = i, this.positionColumn = s;
  }
  toString() {
    return "[" + this.selectionStartLineNumber + "," + this.selectionStartColumn + " -> " + this.positionLineNumber + "," + this.positionColumn + "]";
  }
  equalsSelection(t) {
    return ge.selectionsEqual(this, t);
  }
  static selectionsEqual(t, n) {
    return t.selectionStartLineNumber === n.selectionStartLineNumber && t.selectionStartColumn === n.selectionStartColumn && t.positionLineNumber === n.positionLineNumber && t.positionColumn === n.positionColumn;
  }
  getDirection() {
    return this.selectionStartLineNumber === this.startLineNumber && this.selectionStartColumn === this.startColumn ? ye.LTR : ye.RTL;
  }
  setEndPosition(t, n) {
    return this.getDirection() === ye.LTR ? new ge(this.startLineNumber, this.startColumn, t, n) : new ge(t, n, this.startLineNumber, this.startColumn);
  }
  getPosition() {
    return new X(this.positionLineNumber, this.positionColumn);
  }
  getSelectionStart() {
    return new X(this.selectionStartLineNumber, this.selectionStartColumn);
  }
  setStartPosition(t, n) {
    return this.getDirection() === ye.LTR ? new ge(t, n, this.endLineNumber, this.endColumn) : new ge(this.endLineNumber, this.endColumn, t, n);
  }
  static fromPositions(t, n = t) {
    return new ge(t.lineNumber, t.column, n.lineNumber, n.column);
  }
  static fromRange(t, n) {
    return n === ye.LTR ? new ge(
      t.startLineNumber,
      t.startColumn,
      t.endLineNumber,
      t.endColumn
    ) : new ge(
      t.endLineNumber,
      t.endColumn,
      t.startLineNumber,
      t.startColumn
    );
  }
  static liftSelection(t) {
    return new ge(
      t.selectionStartLineNumber,
      t.selectionStartColumn,
      t.positionLineNumber,
      t.positionColumn
    );
  }
  static selectionsArrEqual(t, n) {
    if (t && !n || !t && n)
      return !1;
    if (!t && !n)
      return !0;
    if (t.length !== n.length)
      return !1;
    for (let i = 0, s = t.length; i < s; i++)
      if (!this.selectionsEqual(t[i], n[i]))
        return !1;
    return !0;
  }
  static isISelection(t) {
    return t && typeof t.selectionStartLineNumber == "number" && typeof t.selectionStartColumn == "number" && typeof t.positionLineNumber == "number" && typeof t.positionColumn == "number";
  }
  static createWithDirection(t, n, i, s, r) {
    return r === ye.LTR ? new ge(t, n, i, s) : new ge(i, s, t, n);
  }
}
const Li = /* @__PURE__ */ Object.create(null);
function l(e, t) {
  if (gl(t)) {
    const n = Li[t];
    if (n === void 0)
      throw new Error(`${e} references an unknown codicon: ${t}`);
    t = n;
  }
  return Li[e] = t, { id: e };
}
const A1 = {
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
  gitPullRequestLabel: l("git-pull-request-label", 60006),
  tagAdd: l("tag-add", 60006),
  tagRemove: l("tag-remove", 60006),
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
  terminalDecorationSuccess: l("terminal-decoration-success", 60017),
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
  chevronLeft: l("chevron-left", 60085),
  chevronRight: l("chevron-right", 60086),
  chevronUp: l("chevron-up", 60087),
  chromeClose: l("chrome-close", 60088),
  chromeMaximize: l("chrome-maximize", 60089),
  chromeMinimize: l("chrome-minimize", 60090),
  chromeRestore: l("chrome-restore", 60091),
  circleOutline: l("circle-outline", 60092),
  circle: l("circle", 60092),
  debugBreakpointUnverified: l("debug-breakpoint-unverified", 60092),
  terminalDecorationIncomplete: l("terminal-decoration-incomplete", 60092),
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
  diffSidebyside: l("diff-sidebyside", 60129),
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
  compareChanges: l("compare-changes", 60157),
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
  issueReopened: l("issue-reopened", 60171),
  issues: l("issues", 60172),
  italic: l("italic", 60173),
  jersey: l("jersey", 60174),
  json: l("json", 60175),
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
  terminalDecorationMark: l("terminal-decoration-mark", 60298),
  debugStackframe: l("debug-stackframe", 60299),
  debugStackframeFocused: l("debug-stackframe-focused", 60299),
  debugBreakpointUnsupported: l("debug-breakpoint-unsupported", 60300),
  symbolString: l("symbol-string", 60301),
  debugReverseContinue: l("debug-reverse-continue", 60302),
  debugStepBack: l("debug-step-back", 60303),
  debugRestartFrame: l("debug-restart-frame", 60304),
  debugAlt: l("debug-alt", 60305),
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
  serverProcess: l("server-process", 60322),
  serverEnvironment: l("server-environment", 60323),
  pass: l("pass", 60324),
  issueClosed: l("issue-closed", 60324),
  stopCircle: l("stop-circle", 60325),
  playCircle: l("play-circle", 60326),
  record: l("record", 60327),
  debugAltSmall: l("debug-alt-small", 60328),
  vmConnect: l("vm-connect", 60329),
  cloud: l("cloud", 60330),
  merge: l("merge", 60331),
  export: l("export", 60332),
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
  workspaceUnknown: l("workspace-unknown", 60355),
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
  bracket: l("bracket", 60175),
  bracketDot: l("bracket-dot", 60389),
  bracketError: l("bracket-error", 60390),
  lockSmall: l("lock-small", 60391),
  azureDevops: l("azure-devops", 60392),
  verifiedFilled: l("verified-filled", 60393),
  newline: l("newline", 60394),
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
  target: l("target", 60408),
  indent: l("indent", 60409),
  recordSmall: l("record-small", 60410),
  errorSmall: l("error-small", 60411),
  terminalDecorationError: l("terminal-decoration-error", 60411),
  arrowCircleDown: l("arrow-circle-down", 60412),
  arrowCircleLeft: l("arrow-circle-left", 60413),
  arrowCircleRight: l("arrow-circle-right", 60414),
  arrowCircleUp: l("arrow-circle-up", 60415),
  layoutSidebarRightOff: l("layout-sidebar-right-off", 60416),
  layoutPanelOff: l("layout-panel-off", 60417),
  layoutSidebarLeftOff: l("layout-sidebar-left-off", 60418),
  blank: l("blank", 60419),
  heartFilled: l("heart-filled", 60420),
  map: l("map", 60421),
  mapHorizontal: l("map-horizontal", 60421),
  foldHorizontal: l("fold-horizontal", 60421),
  mapFilled: l("map-filled", 60422),
  mapHorizontalFilled: l("map-horizontal-filled", 60422),
  foldHorizontalFilled: l("fold-horizontal-filled", 60422),
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
  thumbsdownFilled: l("thumbsdown-filled", 60435),
  thumbsupFilled: l("thumbsup-filled", 60436),
  coffee: l("coffee", 60437),
  snake: l("snake", 60438),
  game: l("game", 60439),
  vr: l("vr", 60440),
  chip: l("chip", 60441),
  piano: l("piano", 60442),
  music: l("music", 60443),
  micFilled: l("mic-filled", 60444),
  repoFetch: l("repo-fetch", 60445),
  copilot: l("copilot", 60446),
  lightbulbSparkle: l("lightbulb-sparkle", 60447),
  robot: l("robot", 60448),
  sparkleFilled: l("sparkle-filled", 60449),
  diffSingle: l("diff-single", 60450),
  diffMultiple: l("diff-multiple", 60451),
  surroundWith: l("surround-with", 60452),
  share: l("share", 60453),
  gitStash: l("git-stash", 60454),
  gitStashApply: l("git-stash-apply", 60455),
  gitStashPop: l("git-stash-pop", 60456),
  vscode: l("vscode", 60457),
  vscodeInsiders: l("vscode-insiders", 60458),
  codeOss: l("code-oss", 60459),
  runCoverage: l("run-coverage", 60460),
  runAllCoverage: l("run-all-coverage", 60461),
  coverage: l("coverage", 60462),
  githubProject: l("github-project", 60463),
  mapVertical: l("map-vertical", 60464),
  foldVertical: l("fold-vertical", 60464),
  mapVerticalFilled: l("map-vertical-filled", 60465),
  foldVerticalFilled: l("fold-vertical-filled", 60465),
  goToSearch: l("go-to-search", 60466),
  percentage: l("percentage", 60467),
  sortPercentage: l("sort-percentage", 60467),
  attach: l("attach", 60468)
}, x1 = {
  dialogError: l("dialog-error", "error"),
  dialogWarning: l("dialog-warning", "warning"),
  dialogInfo: l("dialog-info", "info"),
  dialogClose: l("dialog-close", "close"),
  treeItemExpanded: l("tree-item-expanded", "chevron-down"),
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
  quickInputBack: l("quick-input-back", "arrow-left"),
  dropDownButton: l("drop-down-button", 60084),
  symbolCustomColor: l("symbol-customcolor", 60252),
  exportIcon: l("export", 60332),
  workspaceUnspecified: l("workspace-unspecified", 60355),
  newLine: l("newline", 60394),
  thumbsDownFilled: l("thumbsdown-filled", 60435),
  thumbsUpFilled: l("thumbsup-filled", 60436),
  gitFetch: l("git-fetch", 60445),
  lightbulbSparkleAutofix: l("lightbulb-sparkle-autofix", 60447),
  debugBreakpointPending: l("debug-breakpoint-pending", 60377)
}, B = {
  ...A1,
  ...x1
};
var Ni;
(function(e) {
  e[e.Null = 0] = "Null", e[e.PlainText = 1] = "PlainText";
})(Ni || (Ni = {}));
var pi;
(function(e) {
  e[e.NotSet = -1] = "NotSet", e[e.None = 0] = "None", e[e.Italic = 1] = "Italic", e[e.Bold = 2] = "Bold", e[e.Underline = 4] = "Underline", e[e.Strikethrough = 8] = "Strikethrough";
})(pi || (pi = {}));
var Jt;
(function(e) {
  e[e.None = 0] = "None", e[e.DefaultForeground = 1] = "DefaultForeground", e[e.DefaultBackground = 2] = "DefaultBackground";
})(Jt || (Jt = {}));
var wi;
(function(e) {
  e[e.Other = 0] = "Other", e[e.Comment = 1] = "Comment", e[e.String = 2] = "String", e[e.RegEx = 3] = "RegEx";
})(wi || (wi = {}));
var Ei;
(function(e) {
  e[e.LANGUAGEID_MASK = 255] = "LANGUAGEID_MASK", e[e.TOKEN_TYPE_MASK = 768] = "TOKEN_TYPE_MASK", e[e.BALANCED_BRACKETS_MASK = 1024] = "BALANCED_BRACKETS_MASK", e[e.FONT_STYLE_MASK = 30720] = "FONT_STYLE_MASK", e[e.FOREGROUND_MASK = 16744448] = "FOREGROUND_MASK", e[e.BACKGROUND_MASK = 4278190080] = "BACKGROUND_MASK", e[e.ITALIC_MASK = 2048] = "ITALIC_MASK", e[e.BOLD_MASK = 4096] = "BOLD_MASK", e[e.UNDERLINE_MASK = 8192] = "UNDERLINE_MASK", e[e.STRIKETHROUGH_MASK = 16384] = "STRIKETHROUGH_MASK", e[e.SEMANTIC_USE_ITALIC = 1] = "SEMANTIC_USE_ITALIC", e[e.SEMANTIC_USE_BOLD = 2] = "SEMANTIC_USE_BOLD", e[e.SEMANTIC_USE_UNDERLINE = 4] = "SEMANTIC_USE_UNDERLINE", e[e.SEMANTIC_USE_STRIKETHROUGH = 8] = "SEMANTIC_USE_STRIKETHROUGH", e[e.SEMANTIC_USE_FOREGROUND = 16] = "SEMANTIC_USE_FOREGROUND", e[e.SEMANTIC_USE_BACKGROUND = 32] = "SEMANTIC_USE_BACKGROUND", e[e.LANGUAGEID_OFFSET = 0] = "LANGUAGEID_OFFSET", e[e.TOKEN_TYPE_OFFSET = 8] = "TOKEN_TYPE_OFFSET", e[e.BALANCED_BRACKETS_OFFSET = 10] = "BALANCED_BRACKETS_OFFSET", e[e.FONT_STYLE_OFFSET = 11] = "FONT_STYLE_OFFSET", e[e.FOREGROUND_OFFSET = 15] = "FOREGROUND_OFFSET", e[e.BACKGROUND_OFFSET = 24] = "BACKGROUND_OFFSET";
})(Ei || (Ei = {}));
class R1 {
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
    return this._tokenizationSupports.set(t, n), this.handleChange([t]), zt(() => {
      this._tokenizationSupports.get(t) === n && (this._tokenizationSupports.delete(t), this.handleChange([t]));
    });
  }
  get(t) {
    return this._tokenizationSupports.get(t) || null;
  }
  registerFactory(t, n) {
    var s;
    (s = this._factories.get(t)) == null || s.dispose();
    const i = new v1(this, t, n);
    return this._factories.set(t, i), zt(() => {
      const r = this._factories.get(t);
      !r || r !== i || (this._factories.delete(t), r.dispose());
    });
  }
  async getOrCreate(t) {
    const n = this.get(t);
    if (n)
      return n;
    const i = this._factories.get(t);
    return !i || i.isResolved ? null : (await i.resolve(), this.get(t));
  }
  isResolved(t) {
    if (this.get(t))
      return !0;
    const i = this._factories.get(t);
    return !!(!i || i.isResolved);
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
    return this._colorMap && this._colorMap.length > Jt.DefaultBackground ? this._colorMap[Jt.DefaultBackground] : null;
  }
}
class v1 extends ht {
  get isResolved() {
    return this._isResolved;
  }
  constructor(t, n, i) {
    super(), this._registry = t, this._languageId = n, this._factory = i, this._isDisposed = !1, this._resolvePromise = null, this._isResolved = !1;
  }
  dispose() {
    this._isDisposed = !0, super.dispose();
  }
  async resolve() {
    return this._resolvePromise || (this._resolvePromise = this._create()), this._resolvePromise;
  }
  async _create() {
    const t = await this._factory.tokenizationSupport;
    this._isResolved = !0, t && !this._isDisposed && this._register(this._registry.register(this._languageId, t));
  }
}
const U1 = globalThis._VSCODE_NLS_LANGUAGE === "pseudo" || typeof document < "u" && document.location && document.location.hash.indexOf("pseudo=true") >= 0;
function Ai(e, t) {
  let n;
  return t.length === 0 ? n = e : n = e.replace(/\{(\d+)\}/g, (i, s) => {
    const r = s[0], a = t[r];
    let u = i;
    return typeof a == "string" ? u = a : (typeof a == "number" || typeof a == "boolean" || a === void 0 || a === null) && (u = String(a)), u;
  }), U1 && (n = "［" + n.replace(/[aouei]/g, "$&$&") + "］"), n;
}
function Z(e, t, ...n) {
  return Ai(typeof e == "number" ? D1(e, t) : t, n);
}
function D1(e, t) {
  var i;
  const n = (i = globalThis._VSCODE_NLS_MESSAGES) == null ? void 0 : i[e];
  if (typeof n != "string") {
    if (typeof t == "string")
      return t;
    throw new Error(`!!! NLS MISSING: ${e} !!!`);
  }
  return n;
}
class T1 {
  constructor(t, n, i) {
    this.offset = t, this.type = n, this.language = i, this._tokenBrand = void 0;
  }
  toString() {
    return "(" + this.offset + ", " + this.type + ")";
  }
}
var xi;
(function(e) {
  e[e.Increase = 0] = "Increase", e[e.Decrease = 1] = "Decrease";
})(xi || (xi = {}));
var F;
(function(e) {
  e[e.Method = 0] = "Method", e[e.Function = 1] = "Function", e[e.Constructor = 2] = "Constructor", e[e.Field = 3] = "Field", e[e.Variable = 4] = "Variable", e[e.Class = 5] = "Class", e[e.Struct = 6] = "Struct", e[e.Interface = 7] = "Interface", e[e.Module = 8] = "Module", e[e.Property = 9] = "Property", e[e.Event = 10] = "Event", e[e.Operator = 11] = "Operator", e[e.Unit = 12] = "Unit", e[e.Value = 13] = "Value", e[e.Constant = 14] = "Constant", e[e.Enum = 15] = "Enum", e[e.EnumMember = 16] = "EnumMember", e[e.Keyword = 17] = "Keyword", e[e.Text = 18] = "Text", e[e.Color = 19] = "Color", e[e.File = 20] = "File", e[e.Reference = 21] = "Reference", e[e.Customcolor = 22] = "Customcolor", e[e.Folder = 23] = "Folder", e[e.TypeParameter = 24] = "TypeParameter", e[e.User = 25] = "User", e[e.Issue = 26] = "Issue", e[e.Snippet = 27] = "Snippet";
})(F || (F = {}));
var Ri;
(function(e) {
  const t = /* @__PURE__ */ new Map();
  t.set(F.Method, B.symbolMethod), t.set(F.Function, B.symbolFunction), t.set(F.Constructor, B.symbolConstructor), t.set(F.Field, B.symbolField), t.set(F.Variable, B.symbolVariable), t.set(F.Class, B.symbolClass), t.set(F.Struct, B.symbolStruct), t.set(F.Interface, B.symbolInterface), t.set(F.Module, B.symbolModule), t.set(F.Property, B.symbolProperty), t.set(F.Event, B.symbolEvent), t.set(F.Operator, B.symbolOperator), t.set(F.Unit, B.symbolUnit), t.set(F.Value, B.symbolValue), t.set(F.Enum, B.symbolEnum), t.set(F.Constant, B.symbolConstant), t.set(F.Enum, B.symbolEnum), t.set(F.EnumMember, B.symbolEnumMember), t.set(F.Keyword, B.symbolKeyword), t.set(F.Snippet, B.symbolSnippet), t.set(F.Text, B.symbolText), t.set(F.Color, B.symbolColor), t.set(F.File, B.symbolFile), t.set(F.Reference, B.symbolReference), t.set(F.Customcolor, B.symbolCustomColor), t.set(F.Folder, B.symbolFolder), t.set(F.TypeParameter, B.symbolTypeParameter), t.set(F.User, B.account), t.set(F.Issue, B.issues);
  function n(r) {
    let a = t.get(r);
    return a || (console.info("No codicon found for CompletionItemKind " + r), a = B.symbolProperty), a;
  }
  e.toIcon = n;
  const i = /* @__PURE__ */ new Map();
  i.set("method", F.Method), i.set("function", F.Function), i.set("constructor", F.Constructor), i.set("field", F.Field), i.set("variable", F.Variable), i.set("class", F.Class), i.set("struct", F.Struct), i.set("interface", F.Interface), i.set("module", F.Module), i.set("property", F.Property), i.set("event", F.Event), i.set("operator", F.Operator), i.set("unit", F.Unit), i.set("value", F.Value), i.set("constant", F.Constant), i.set("enum", F.Enum), i.set("enum-member", F.EnumMember), i.set("enumMember", F.EnumMember), i.set("keyword", F.Keyword), i.set("snippet", F.Snippet), i.set("text", F.Text), i.set("color", F.Color), i.set("file", F.File), i.set("reference", F.Reference), i.set("customcolor", F.Customcolor), i.set("folder", F.Folder), i.set("type-parameter", F.TypeParameter), i.set("typeParameter", F.TypeParameter), i.set("account", F.User), i.set("issue", F.Issue);
  function s(r, a) {
    let u = i.get(r);
    return typeof u > "u" && !a && (u = F.Property), u;
  }
  e.fromString = s;
})(Ri || (Ri = {}));
var vi;
(function(e) {
  e[e.Deprecated = 1] = "Deprecated";
})(vi || (vi = {}));
var Ui;
(function(e) {
  e[e.None = 0] = "None", e[e.KeepWhitespace = 1] = "KeepWhitespace", e[e.InsertAsSnippet = 4] = "InsertAsSnippet";
})(Ui || (Ui = {}));
var Di;
(function(e) {
  e[e.Word = 0] = "Word", e[e.Line = 1] = "Line", e[e.Suggest = 2] = "Suggest";
})(Di || (Di = {}));
var Ti;
(function(e) {
  e[e.Invoke = 0] = "Invoke", e[e.TriggerCharacter = 1] = "TriggerCharacter", e[e.TriggerForIncompleteCompletions = 2] = "TriggerForIncompleteCompletions";
})(Ti || (Ti = {}));
var Mi;
(function(e) {
  e[e.Automatic = 0] = "Automatic", e[e.Explicit = 1] = "Explicit";
})(Mi || (Mi = {}));
var Fi;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.Auto = 2] = "Auto";
})(Fi || (Fi = {}));
var ki;
(function(e) {
  e[e.Automatic = 0] = "Automatic", e[e.PasteAs = 1] = "PasteAs";
})(ki || (ki = {}));
var Ii;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.TriggerCharacter = 2] = "TriggerCharacter", e[e.ContentChange = 3] = "ContentChange";
})(Ii || (Ii = {}));
var Si;
(function(e) {
  e[e.Text = 0] = "Text", e[e.Read = 1] = "Read", e[e.Write = 2] = "Write";
})(Si || (Si = {}));
var O;
(function(e) {
  e[e.File = 0] = "File", e[e.Module = 1] = "Module", e[e.Namespace = 2] = "Namespace", e[e.Package = 3] = "Package", e[e.Class = 4] = "Class", e[e.Method = 5] = "Method", e[e.Property = 6] = "Property", e[e.Field = 7] = "Field", e[e.Constructor = 8] = "Constructor", e[e.Enum = 9] = "Enum", e[e.Interface = 10] = "Interface", e[e.Function = 11] = "Function", e[e.Variable = 12] = "Variable", e[e.Constant = 13] = "Constant", e[e.String = 14] = "String", e[e.Number = 15] = "Number", e[e.Boolean = 16] = "Boolean", e[e.Array = 17] = "Array", e[e.Object = 18] = "Object", e[e.Key = 19] = "Key", e[e.Null = 20] = "Null", e[e.EnumMember = 21] = "EnumMember", e[e.Struct = 22] = "Struct", e[e.Event = 23] = "Event", e[e.Operator = 24] = "Operator", e[e.TypeParameter = 25] = "TypeParameter";
})(O || (O = {}));
O.Array + "", Z(1658, "array"), O.Boolean + "", Z(1659, "boolean"), O.Class + "", Z(1660, "class"), O.Constant + "", Z(1661, "constant"), O.Constructor + "", Z(1662, "constructor"), O.Enum + "", Z(1663, "enumeration"), O.EnumMember + "", Z(1664, "enumeration member"), O.Event + "", Z(1665, "event"), O.Field + "", Z(1666, "field"), O.File + "", Z(1667, "file"), O.Function + "", Z(1668, "function"), O.Interface + "", Z(1669, "interface"), O.Key + "", Z(1670, "key"), O.Method + "", Z(1671, "method"), O.Module + "", Z(1672, "module"), O.Namespace + "", Z(1673, "namespace"), O.Null + "", Z(1674, "null"), O.Number + "", Z(1675, "number"), O.Object + "", Z(1676, "object"), O.Operator + "", Z(1677, "operator"), O.Package + "", Z(1678, "package"), O.Property + "", Z(1679, "property"), O.String + "", Z(1680, "string"), O.Struct + "", Z(1681, "struct"), O.TypeParameter + "", Z(1682, "type parameter"), O.Variable + "", Z(1683, "variable");
var Bi;
(function(e) {
  e[e.Deprecated = 1] = "Deprecated";
})(Bi || (Bi = {}));
var Pi;
(function(e) {
  const t = /* @__PURE__ */ new Map();
  t.set(O.File, B.symbolFile), t.set(O.Module, B.symbolModule), t.set(O.Namespace, B.symbolNamespace), t.set(O.Package, B.symbolPackage), t.set(O.Class, B.symbolClass), t.set(O.Method, B.symbolMethod), t.set(O.Property, B.symbolProperty), t.set(O.Field, B.symbolField), t.set(O.Constructor, B.symbolConstructor), t.set(O.Enum, B.symbolEnum), t.set(O.Interface, B.symbolInterface), t.set(O.Function, B.symbolFunction), t.set(O.Variable, B.symbolVariable), t.set(O.Constant, B.symbolConstant), t.set(O.String, B.symbolString), t.set(O.Number, B.symbolNumber), t.set(O.Boolean, B.symbolBoolean), t.set(O.Array, B.symbolArray), t.set(O.Object, B.symbolObject), t.set(O.Key, B.symbolKey), t.set(O.Null, B.symbolNull), t.set(O.EnumMember, B.symbolEnumMember), t.set(O.Struct, B.symbolStruct), t.set(O.Event, B.symbolEvent), t.set(O.Operator, B.symbolOperator), t.set(O.TypeParameter, B.symbolTypeParameter);
  function n(i) {
    let s = t.get(i);
    return s || (console.info("No codicon found for SymbolKind " + i), s = B.symbolProperty), s;
  }
  e.toIcon = n;
})(Pi || (Pi = {}));
const de = class de {
  static fromValue(t) {
    switch (t) {
      case "comment":
        return de.Comment;
      case "imports":
        return de.Imports;
      case "region":
        return de.Region;
    }
    return new de(t);
  }
  constructor(t) {
    this.value = t;
  }
};
de.Comment = new de("comment"), de.Imports = new de("imports"), de.Region = new de("region");
let yi = de;
var Oi;
(function(e) {
  e[e.AIGenerated = 1] = "AIGenerated";
})(Oi || (Oi = {}));
var Vi;
(function(e) {
  e[e.Invoke = 0] = "Invoke", e[e.Automatic = 1] = "Automatic";
})(Vi || (Vi = {}));
var Hi;
(function(e) {
  function t(n) {
    return !n || typeof n != "object" ? !1 : typeof n.id == "string" && typeof n.title == "string";
  }
  e.is = t;
})(Hi || (Hi = {}));
var qi;
(function(e) {
  e[e.Collapsed = 0] = "Collapsed", e[e.Expanded = 1] = "Expanded";
})(qi || (qi = {}));
var Wi;
(function(e) {
  e[e.Unresolved = 0] = "Unresolved", e[e.Resolved = 1] = "Resolved";
})(Wi || (Wi = {}));
var Gi;
(function(e) {
  e[e.Current = 0] = "Current", e[e.Outdated = 1] = "Outdated";
})(Gi || (Gi = {}));
var $i;
(function(e) {
  e[e.Editing = 0] = "Editing", e[e.Preview = 1] = "Preview";
})($i || ($i = {}));
var zi;
(function(e) {
  e[e.Type = 1] = "Type", e[e.Parameter = 2] = "Parameter";
})(zi || (zi = {}));
new R1();
var ji;
(function(e) {
  e[e.None = 0] = "None", e[e.Option = 1] = "Option", e[e.Default = 2] = "Default", e[e.Preferred = 3] = "Preferred";
})(ji || (ji = {}));
var Xi;
(function(e) {
  e[e.Invoke = 0] = "Invoke", e[e.Automatic = 1] = "Automatic";
})(Xi || (Xi = {}));
var Yi;
(function(e) {
  e[e.Unknown = 0] = "Unknown", e[e.Disabled = 1] = "Disabled", e[e.Enabled = 2] = "Enabled";
})(Yi || (Yi = {}));
var Qi;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.Auto = 2] = "Auto";
})(Qi || (Qi = {}));
var Zi;
(function(e) {
  e[e.None = 0] = "None", e[e.KeepWhitespace = 1] = "KeepWhitespace", e[e.InsertAsSnippet = 4] = "InsertAsSnippet";
})(Zi || (Zi = {}));
var Ji;
(function(e) {
  e[e.Method = 0] = "Method", e[e.Function = 1] = "Function", e[e.Constructor = 2] = "Constructor", e[e.Field = 3] = "Field", e[e.Variable = 4] = "Variable", e[e.Class = 5] = "Class", e[e.Struct = 6] = "Struct", e[e.Interface = 7] = "Interface", e[e.Module = 8] = "Module", e[e.Property = 9] = "Property", e[e.Event = 10] = "Event", e[e.Operator = 11] = "Operator", e[e.Unit = 12] = "Unit", e[e.Value = 13] = "Value", e[e.Constant = 14] = "Constant", e[e.Enum = 15] = "Enum", e[e.EnumMember = 16] = "EnumMember", e[e.Keyword = 17] = "Keyword", e[e.Text = 18] = "Text", e[e.Color = 19] = "Color", e[e.File = 20] = "File", e[e.Reference = 21] = "Reference", e[e.Customcolor = 22] = "Customcolor", e[e.Folder = 23] = "Folder", e[e.TypeParameter = 24] = "TypeParameter", e[e.User = 25] = "User", e[e.Issue = 26] = "Issue", e[e.Snippet = 27] = "Snippet";
})(Ji || (Ji = {}));
var Ki;
(function(e) {
  e[e.Deprecated = 1] = "Deprecated";
})(Ki || (Ki = {}));
var Ci;
(function(e) {
  e[e.Invoke = 0] = "Invoke", e[e.TriggerCharacter = 1] = "TriggerCharacter", e[e.TriggerForIncompleteCompletions = 2] = "TriggerForIncompleteCompletions";
})(Ci || (Ci = {}));
var es;
(function(e) {
  e[e.EXACT = 0] = "EXACT", e[e.ABOVE = 1] = "ABOVE", e[e.BELOW = 2] = "BELOW";
})(es || (es = {}));
var ts;
(function(e) {
  e[e.NotSet = 0] = "NotSet", e[e.ContentFlush = 1] = "ContentFlush", e[e.RecoverFromMarkers = 2] = "RecoverFromMarkers", e[e.Explicit = 3] = "Explicit", e[e.Paste = 4] = "Paste", e[e.Undo = 5] = "Undo", e[e.Redo = 6] = "Redo";
})(ts || (ts = {}));
var ns;
(function(e) {
  e[e.LF = 1] = "LF", e[e.CRLF = 2] = "CRLF";
})(ns || (ns = {}));
var is;
(function(e) {
  e[e.Text = 0] = "Text", e[e.Read = 1] = "Read", e[e.Write = 2] = "Write";
})(is || (is = {}));
var ss;
(function(e) {
  e[e.None = 0] = "None", e[e.Keep = 1] = "Keep", e[e.Brackets = 2] = "Brackets", e[e.Advanced = 3] = "Advanced", e[e.Full = 4] = "Full";
})(ss || (ss = {}));
var rs;
(function(e) {
  e[e.acceptSuggestionOnCommitCharacter = 0] = "acceptSuggestionOnCommitCharacter", e[e.acceptSuggestionOnEnter = 1] = "acceptSuggestionOnEnter", e[e.accessibilitySupport = 2] = "accessibilitySupport", e[e.accessibilityPageSize = 3] = "accessibilityPageSize", e[e.ariaLabel = 4] = "ariaLabel", e[e.ariaRequired = 5] = "ariaRequired", e[e.autoClosingBrackets = 6] = "autoClosingBrackets", e[e.autoClosingComments = 7] = "autoClosingComments", e[e.screenReaderAnnounceInlineSuggestion = 8] = "screenReaderAnnounceInlineSuggestion", e[e.autoClosingDelete = 9] = "autoClosingDelete", e[e.autoClosingOvertype = 10] = "autoClosingOvertype", e[e.autoClosingQuotes = 11] = "autoClosingQuotes", e[e.autoIndent = 12] = "autoIndent", e[e.automaticLayout = 13] = "automaticLayout", e[e.autoSurround = 14] = "autoSurround", e[e.bracketPairColorization = 15] = "bracketPairColorization", e[e.guides = 16] = "guides", e[e.codeLens = 17] = "codeLens", e[e.codeLensFontFamily = 18] = "codeLensFontFamily", e[e.codeLensFontSize = 19] = "codeLensFontSize", e[e.colorDecorators = 20] = "colorDecorators", e[e.colorDecoratorsLimit = 21] = "colorDecoratorsLimit", e[e.columnSelection = 22] = "columnSelection", e[e.comments = 23] = "comments", e[e.contextmenu = 24] = "contextmenu", e[e.copyWithSyntaxHighlighting = 25] = "copyWithSyntaxHighlighting", e[e.cursorBlinking = 26] = "cursorBlinking", e[e.cursorSmoothCaretAnimation = 27] = "cursorSmoothCaretAnimation", e[e.cursorStyle = 28] = "cursorStyle", e[e.cursorSurroundingLines = 29] = "cursorSurroundingLines", e[e.cursorSurroundingLinesStyle = 30] = "cursorSurroundingLinesStyle", e[e.cursorWidth = 31] = "cursorWidth", e[e.disableLayerHinting = 32] = "disableLayerHinting", e[e.disableMonospaceOptimizations = 33] = "disableMonospaceOptimizations", e[e.domReadOnly = 34] = "domReadOnly", e[e.dragAndDrop = 35] = "dragAndDrop", e[e.dropIntoEditor = 36] = "dropIntoEditor", e[e.emptySelectionClipboard = 37] = "emptySelectionClipboard", e[e.experimentalWhitespaceRendering = 38] = "experimentalWhitespaceRendering", e[e.extraEditorClassName = 39] = "extraEditorClassName", e[e.fastScrollSensitivity = 40] = "fastScrollSensitivity", e[e.find = 41] = "find", e[e.fixedOverflowWidgets = 42] = "fixedOverflowWidgets", e[e.folding = 43] = "folding", e[e.foldingStrategy = 44] = "foldingStrategy", e[e.foldingHighlight = 45] = "foldingHighlight", e[e.foldingImportsByDefault = 46] = "foldingImportsByDefault", e[e.foldingMaximumRegions = 47] = "foldingMaximumRegions", e[e.unfoldOnClickAfterEndOfLine = 48] = "unfoldOnClickAfterEndOfLine", e[e.fontFamily = 49] = "fontFamily", e[e.fontInfo = 50] = "fontInfo", e[e.fontLigatures = 51] = "fontLigatures", e[e.fontSize = 52] = "fontSize", e[e.fontWeight = 53] = "fontWeight", e[e.fontVariations = 54] = "fontVariations", e[e.formatOnPaste = 55] = "formatOnPaste", e[e.formatOnType = 56] = "formatOnType", e[e.glyphMargin = 57] = "glyphMargin", e[e.gotoLocation = 58] = "gotoLocation", e[e.hideCursorInOverviewRuler = 59] = "hideCursorInOverviewRuler", e[e.hover = 60] = "hover", e[e.inDiffEditor = 61] = "inDiffEditor", e[e.inlineSuggest = 62] = "inlineSuggest", e[e.inlineEdit = 63] = "inlineEdit", e[e.letterSpacing = 64] = "letterSpacing", e[e.lightbulb = 65] = "lightbulb", e[e.lineDecorationsWidth = 66] = "lineDecorationsWidth", e[e.lineHeight = 67] = "lineHeight", e[e.lineNumbers = 68] = "lineNumbers", e[e.lineNumbersMinChars = 69] = "lineNumbersMinChars", e[e.linkedEditing = 70] = "linkedEditing", e[e.links = 71] = "links", e[e.matchBrackets = 72] = "matchBrackets", e[e.minimap = 73] = "minimap", e[e.mouseStyle = 74] = "mouseStyle", e[e.mouseWheelScrollSensitivity = 75] = "mouseWheelScrollSensitivity", e[e.mouseWheelZoom = 76] = "mouseWheelZoom", e[e.multiCursorMergeOverlapping = 77] = "multiCursorMergeOverlapping", e[e.multiCursorModifier = 78] = "multiCursorModifier", e[e.multiCursorPaste = 79] = "multiCursorPaste", e[e.multiCursorLimit = 80] = "multiCursorLimit", e[e.occurrencesHighlight = 81] = "occurrencesHighlight", e[e.overviewRulerBorder = 82] = "overviewRulerBorder", e[e.overviewRulerLanes = 83] = "overviewRulerLanes", e[e.padding = 84] = "padding", e[e.pasteAs = 85] = "pasteAs", e[e.parameterHints = 86] = "parameterHints", e[e.peekWidgetDefaultFocus = 87] = "peekWidgetDefaultFocus", e[e.placeholder = 88] = "placeholder", e[e.definitionLinkOpensInPeek = 89] = "definitionLinkOpensInPeek", e[e.quickSuggestions = 90] = "quickSuggestions", e[e.quickSuggestionsDelay = 91] = "quickSuggestionsDelay", e[e.readOnly = 92] = "readOnly", e[e.readOnlyMessage = 93] = "readOnlyMessage", e[e.renameOnType = 94] = "renameOnType", e[e.renderControlCharacters = 95] = "renderControlCharacters", e[e.renderFinalNewline = 96] = "renderFinalNewline", e[e.renderLineHighlight = 97] = "renderLineHighlight", e[e.renderLineHighlightOnlyWhenFocus = 98] = "renderLineHighlightOnlyWhenFocus", e[e.renderValidationDecorations = 99] = "renderValidationDecorations", e[e.renderWhitespace = 100] = "renderWhitespace", e[e.revealHorizontalRightPadding = 101] = "revealHorizontalRightPadding", e[e.roundedSelection = 102] = "roundedSelection", e[e.rulers = 103] = "rulers", e[e.scrollbar = 104] = "scrollbar", e[e.scrollBeyondLastColumn = 105] = "scrollBeyondLastColumn", e[e.scrollBeyondLastLine = 106] = "scrollBeyondLastLine", e[e.scrollPredominantAxis = 107] = "scrollPredominantAxis", e[e.selectionClipboard = 108] = "selectionClipboard", e[e.selectionHighlight = 109] = "selectionHighlight", e[e.selectOnLineNumbers = 110] = "selectOnLineNumbers", e[e.showFoldingControls = 111] = "showFoldingControls", e[e.showUnused = 112] = "showUnused", e[e.snippetSuggestions = 113] = "snippetSuggestions", e[e.smartSelect = 114] = "smartSelect", e[e.smoothScrolling = 115] = "smoothScrolling", e[e.stickyScroll = 116] = "stickyScroll", e[e.stickyTabStops = 117] = "stickyTabStops", e[e.stopRenderingLineAfter = 118] = "stopRenderingLineAfter", e[e.suggest = 119] = "suggest", e[e.suggestFontSize = 120] = "suggestFontSize", e[e.suggestLineHeight = 121] = "suggestLineHeight", e[e.suggestOnTriggerCharacters = 122] = "suggestOnTriggerCharacters", e[e.suggestSelection = 123] = "suggestSelection", e[e.tabCompletion = 124] = "tabCompletion", e[e.tabIndex = 125] = "tabIndex", e[e.unicodeHighlighting = 126] = "unicodeHighlighting", e[e.unusualLineTerminators = 127] = "unusualLineTerminators", e[e.useShadowDOM = 128] = "useShadowDOM", e[e.useTabStops = 129] = "useTabStops", e[e.wordBreak = 130] = "wordBreak", e[e.wordSegmenterLocales = 131] = "wordSegmenterLocales", e[e.wordSeparators = 132] = "wordSeparators", e[e.wordWrap = 133] = "wordWrap", e[e.wordWrapBreakAfterCharacters = 134] = "wordWrapBreakAfterCharacters", e[e.wordWrapBreakBeforeCharacters = 135] = "wordWrapBreakBeforeCharacters", e[e.wordWrapColumn = 136] = "wordWrapColumn", e[e.wordWrapOverride1 = 137] = "wordWrapOverride1", e[e.wordWrapOverride2 = 138] = "wordWrapOverride2", e[e.wrappingIndent = 139] = "wrappingIndent", e[e.wrappingStrategy = 140] = "wrappingStrategy", e[e.showDeprecated = 141] = "showDeprecated", e[e.inlayHints = 142] = "inlayHints", e[e.editorClassName = 143] = "editorClassName", e[e.pixelRatio = 144] = "pixelRatio", e[e.tabFocusMode = 145] = "tabFocusMode", e[e.layoutInfo = 146] = "layoutInfo", e[e.wrappingInfo = 147] = "wrappingInfo", e[e.defaultColorDecorators = 148] = "defaultColorDecorators", e[e.colorDecoratorsActivatedOn = 149] = "colorDecoratorsActivatedOn", e[e.inlineCompletionsAccessibilityVerbose = 150] = "inlineCompletionsAccessibilityVerbose";
})(rs || (rs = {}));
var ls;
(function(e) {
  e[e.TextDefined = 0] = "TextDefined", e[e.LF = 1] = "LF", e[e.CRLF = 2] = "CRLF";
})(ls || (ls = {}));
var as;
(function(e) {
  e[e.LF = 0] = "LF", e[e.CRLF = 1] = "CRLF";
})(as || (as = {}));
var us;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Center = 2] = "Center", e[e.Right = 3] = "Right";
})(us || (us = {}));
var os;
(function(e) {
  e[e.Increase = 0] = "Increase", e[e.Decrease = 1] = "Decrease";
})(os || (os = {}));
var cs;
(function(e) {
  e[e.None = 0] = "None", e[e.Indent = 1] = "Indent", e[e.IndentOutdent = 2] = "IndentOutdent", e[e.Outdent = 3] = "Outdent";
})(cs || (cs = {}));
var fs;
(function(e) {
  e[e.Both = 0] = "Both", e[e.Right = 1] = "Right", e[e.Left = 2] = "Left", e[e.None = 3] = "None";
})(fs || (fs = {}));
var hs;
(function(e) {
  e[e.Type = 1] = "Type", e[e.Parameter = 2] = "Parameter";
})(hs || (hs = {}));
var ms;
(function(e) {
  e[e.Automatic = 0] = "Automatic", e[e.Explicit = 1] = "Explicit";
})(ms || (ms = {}));
var gs;
(function(e) {
  e[e.Invoke = 0] = "Invoke", e[e.Automatic = 1] = "Automatic";
})(gs || (gs = {}));
var In;
(function(e) {
  e[e.DependsOnKbLayout = -1] = "DependsOnKbLayout", e[e.Unknown = 0] = "Unknown", e[e.Backspace = 1] = "Backspace", e[e.Tab = 2] = "Tab", e[e.Enter = 3] = "Enter", e[e.Shift = 4] = "Shift", e[e.Ctrl = 5] = "Ctrl", e[e.Alt = 6] = "Alt", e[e.PauseBreak = 7] = "PauseBreak", e[e.CapsLock = 8] = "CapsLock", e[e.Escape = 9] = "Escape", e[e.Space = 10] = "Space", e[e.PageUp = 11] = "PageUp", e[e.PageDown = 12] = "PageDown", e[e.End = 13] = "End", e[e.Home = 14] = "Home", e[e.LeftArrow = 15] = "LeftArrow", e[e.UpArrow = 16] = "UpArrow", e[e.RightArrow = 17] = "RightArrow", e[e.DownArrow = 18] = "DownArrow", e[e.Insert = 19] = "Insert", e[e.Delete = 20] = "Delete", e[e.Digit0 = 21] = "Digit0", e[e.Digit1 = 22] = "Digit1", e[e.Digit2 = 23] = "Digit2", e[e.Digit3 = 24] = "Digit3", e[e.Digit4 = 25] = "Digit4", e[e.Digit5 = 26] = "Digit5", e[e.Digit6 = 27] = "Digit6", e[e.Digit7 = 28] = "Digit7", e[e.Digit8 = 29] = "Digit8", e[e.Digit9 = 30] = "Digit9", e[e.KeyA = 31] = "KeyA", e[e.KeyB = 32] = "KeyB", e[e.KeyC = 33] = "KeyC", e[e.KeyD = 34] = "KeyD", e[e.KeyE = 35] = "KeyE", e[e.KeyF = 36] = "KeyF", e[e.KeyG = 37] = "KeyG", e[e.KeyH = 38] = "KeyH", e[e.KeyI = 39] = "KeyI", e[e.KeyJ = 40] = "KeyJ", e[e.KeyK = 41] = "KeyK", e[e.KeyL = 42] = "KeyL", e[e.KeyM = 43] = "KeyM", e[e.KeyN = 44] = "KeyN", e[e.KeyO = 45] = "KeyO", e[e.KeyP = 46] = "KeyP", e[e.KeyQ = 47] = "KeyQ", e[e.KeyR = 48] = "KeyR", e[e.KeyS = 49] = "KeyS", e[e.KeyT = 50] = "KeyT", e[e.KeyU = 51] = "KeyU", e[e.KeyV = 52] = "KeyV", e[e.KeyW = 53] = "KeyW", e[e.KeyX = 54] = "KeyX", e[e.KeyY = 55] = "KeyY", e[e.KeyZ = 56] = "KeyZ", e[e.Meta = 57] = "Meta", e[e.ContextMenu = 58] = "ContextMenu", e[e.F1 = 59] = "F1", e[e.F2 = 60] = "F2", e[e.F3 = 61] = "F3", e[e.F4 = 62] = "F4", e[e.F5 = 63] = "F5", e[e.F6 = 64] = "F6", e[e.F7 = 65] = "F7", e[e.F8 = 66] = "F8", e[e.F9 = 67] = "F9", e[e.F10 = 68] = "F10", e[e.F11 = 69] = "F11", e[e.F12 = 70] = "F12", e[e.F13 = 71] = "F13", e[e.F14 = 72] = "F14", e[e.F15 = 73] = "F15", e[e.F16 = 74] = "F16", e[e.F17 = 75] = "F17", e[e.F18 = 76] = "F18", e[e.F19 = 77] = "F19", e[e.F20 = 78] = "F20", e[e.F21 = 79] = "F21", e[e.F22 = 80] = "F22", e[e.F23 = 81] = "F23", e[e.F24 = 82] = "F24", e[e.NumLock = 83] = "NumLock", e[e.ScrollLock = 84] = "ScrollLock", e[e.Semicolon = 85] = "Semicolon", e[e.Equal = 86] = "Equal", e[e.Comma = 87] = "Comma", e[e.Minus = 88] = "Minus", e[e.Period = 89] = "Period", e[e.Slash = 90] = "Slash", e[e.Backquote = 91] = "Backquote", e[e.BracketLeft = 92] = "BracketLeft", e[e.Backslash = 93] = "Backslash", e[e.BracketRight = 94] = "BracketRight", e[e.Quote = 95] = "Quote", e[e.OEM_8 = 96] = "OEM_8", e[e.IntlBackslash = 97] = "IntlBackslash", e[e.Numpad0 = 98] = "Numpad0", e[e.Numpad1 = 99] = "Numpad1", e[e.Numpad2 = 100] = "Numpad2", e[e.Numpad3 = 101] = "Numpad3", e[e.Numpad4 = 102] = "Numpad4", e[e.Numpad5 = 103] = "Numpad5", e[e.Numpad6 = 104] = "Numpad6", e[e.Numpad7 = 105] = "Numpad7", e[e.Numpad8 = 106] = "Numpad8", e[e.Numpad9 = 107] = "Numpad9", e[e.NumpadMultiply = 108] = "NumpadMultiply", e[e.NumpadAdd = 109] = "NumpadAdd", e[e.NUMPAD_SEPARATOR = 110] = "NUMPAD_SEPARATOR", e[e.NumpadSubtract = 111] = "NumpadSubtract", e[e.NumpadDecimal = 112] = "NumpadDecimal", e[e.NumpadDivide = 113] = "NumpadDivide", e[e.KEY_IN_COMPOSITION = 114] = "KEY_IN_COMPOSITION", e[e.ABNT_C1 = 115] = "ABNT_C1", e[e.ABNT_C2 = 116] = "ABNT_C2", e[e.AudioVolumeMute = 117] = "AudioVolumeMute", e[e.AudioVolumeUp = 118] = "AudioVolumeUp", e[e.AudioVolumeDown = 119] = "AudioVolumeDown", e[e.BrowserSearch = 120] = "BrowserSearch", e[e.BrowserHome = 121] = "BrowserHome", e[e.BrowserBack = 122] = "BrowserBack", e[e.BrowserForward = 123] = "BrowserForward", e[e.MediaTrackNext = 124] = "MediaTrackNext", e[e.MediaTrackPrevious = 125] = "MediaTrackPrevious", e[e.MediaStop = 126] = "MediaStop", e[e.MediaPlayPause = 127] = "MediaPlayPause", e[e.LaunchMediaPlayer = 128] = "LaunchMediaPlayer", e[e.LaunchMail = 129] = "LaunchMail", e[e.LaunchApp2 = 130] = "LaunchApp2", e[e.Clear = 131] = "Clear", e[e.MAX_VALUE = 132] = "MAX_VALUE";
})(In || (In = {}));
var Sn;
(function(e) {
  e[e.Hint = 1] = "Hint", e[e.Info = 2] = "Info", e[e.Warning = 4] = "Warning", e[e.Error = 8] = "Error";
})(Sn || (Sn = {}));
var Bn;
(function(e) {
  e[e.Unnecessary = 1] = "Unnecessary", e[e.Deprecated = 2] = "Deprecated";
})(Bn || (Bn = {}));
var _s;
(function(e) {
  e[e.Inline = 1] = "Inline", e[e.Gutter = 2] = "Gutter";
})(_s || (_s = {}));
var bs;
(function(e) {
  e[e.Normal = 1] = "Normal", e[e.Underlined = 2] = "Underlined";
})(bs || (bs = {}));
var ds;
(function(e) {
  e[e.UNKNOWN = 0] = "UNKNOWN", e[e.TEXTAREA = 1] = "TEXTAREA", e[e.GUTTER_GLYPH_MARGIN = 2] = "GUTTER_GLYPH_MARGIN", e[e.GUTTER_LINE_NUMBERS = 3] = "GUTTER_LINE_NUMBERS", e[e.GUTTER_LINE_DECORATIONS = 4] = "GUTTER_LINE_DECORATIONS", e[e.GUTTER_VIEW_ZONE = 5] = "GUTTER_VIEW_ZONE", e[e.CONTENT_TEXT = 6] = "CONTENT_TEXT", e[e.CONTENT_EMPTY = 7] = "CONTENT_EMPTY", e[e.CONTENT_VIEW_ZONE = 8] = "CONTENT_VIEW_ZONE", e[e.CONTENT_WIDGET = 9] = "CONTENT_WIDGET", e[e.OVERVIEW_RULER = 10] = "OVERVIEW_RULER", e[e.SCROLLBAR = 11] = "SCROLLBAR", e[e.OVERLAY_WIDGET = 12] = "OVERLAY_WIDGET", e[e.OUTSIDE_EDITOR = 13] = "OUTSIDE_EDITOR";
})(ds || (ds = {}));
var Ls;
(function(e) {
  e[e.AIGenerated = 1] = "AIGenerated";
})(Ls || (Ls = {}));
var Ns;
(function(e) {
  e[e.Invoke = 0] = "Invoke", e[e.Automatic = 1] = "Automatic";
})(Ns || (Ns = {}));
var ps;
(function(e) {
  e[e.TOP_RIGHT_CORNER = 0] = "TOP_RIGHT_CORNER", e[e.BOTTOM_RIGHT_CORNER = 1] = "BOTTOM_RIGHT_CORNER", e[e.TOP_CENTER = 2] = "TOP_CENTER";
})(ps || (ps = {}));
var ws;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Center = 2] = "Center", e[e.Right = 4] = "Right", e[e.Full = 7] = "Full";
})(ws || (ws = {}));
var Es;
(function(e) {
  e[e.Word = 0] = "Word", e[e.Line = 1] = "Line", e[e.Suggest = 2] = "Suggest";
})(Es || (Es = {}));
var As;
(function(e) {
  e[e.Left = 0] = "Left", e[e.Right = 1] = "Right", e[e.None = 2] = "None", e[e.LeftOfInjectedText = 3] = "LeftOfInjectedText", e[e.RightOfInjectedText = 4] = "RightOfInjectedText";
})(As || (As = {}));
var xs;
(function(e) {
  e[e.Off = 0] = "Off", e[e.On = 1] = "On", e[e.Relative = 2] = "Relative", e[e.Interval = 3] = "Interval", e[e.Custom = 4] = "Custom";
})(xs || (xs = {}));
var Rs;
(function(e) {
  e[e.None = 0] = "None", e[e.Text = 1] = "Text", e[e.Blocks = 2] = "Blocks";
})(Rs || (Rs = {}));
var vs;
(function(e) {
  e[e.Smooth = 0] = "Smooth", e[e.Immediate = 1] = "Immediate";
})(vs || (vs = {}));
var Us;
(function(e) {
  e[e.Auto = 1] = "Auto", e[e.Hidden = 2] = "Hidden", e[e.Visible = 3] = "Visible";
})(Us || (Us = {}));
var Pn;
(function(e) {
  e[e.LTR = 0] = "LTR", e[e.RTL = 1] = "RTL";
})(Pn || (Pn = {}));
var Ds;
(function(e) {
  e.Off = "off", e.OnCode = "onCode", e.On = "on";
})(Ds || (Ds = {}));
var Ts;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.TriggerCharacter = 2] = "TriggerCharacter", e[e.ContentChange = 3] = "ContentChange";
})(Ts || (Ts = {}));
var Ms;
(function(e) {
  e[e.File = 0] = "File", e[e.Module = 1] = "Module", e[e.Namespace = 2] = "Namespace", e[e.Package = 3] = "Package", e[e.Class = 4] = "Class", e[e.Method = 5] = "Method", e[e.Property = 6] = "Property", e[e.Field = 7] = "Field", e[e.Constructor = 8] = "Constructor", e[e.Enum = 9] = "Enum", e[e.Interface = 10] = "Interface", e[e.Function = 11] = "Function", e[e.Variable = 12] = "Variable", e[e.Constant = 13] = "Constant", e[e.String = 14] = "String", e[e.Number = 15] = "Number", e[e.Boolean = 16] = "Boolean", e[e.Array = 17] = "Array", e[e.Object = 18] = "Object", e[e.Key = 19] = "Key", e[e.Null = 20] = "Null", e[e.EnumMember = 21] = "EnumMember", e[e.Struct = 22] = "Struct", e[e.Event = 23] = "Event", e[e.Operator = 24] = "Operator", e[e.TypeParameter = 25] = "TypeParameter";
})(Ms || (Ms = {}));
var Fs;
(function(e) {
  e[e.Deprecated = 1] = "Deprecated";
})(Fs || (Fs = {}));
var ks;
(function(e) {
  e[e.Hidden = 0] = "Hidden", e[e.Blink = 1] = "Blink", e[e.Smooth = 2] = "Smooth", e[e.Phase = 3] = "Phase", e[e.Expand = 4] = "Expand", e[e.Solid = 5] = "Solid";
})(ks || (ks = {}));
var Is;
(function(e) {
  e[e.Line = 1] = "Line", e[e.Block = 2] = "Block", e[e.Underline = 3] = "Underline", e[e.LineThin = 4] = "LineThin", e[e.BlockOutline = 5] = "BlockOutline", e[e.UnderlineThin = 6] = "UnderlineThin";
})(Is || (Is = {}));
var Ss;
(function(e) {
  e[e.AlwaysGrowsWhenTypingAtEdges = 0] = "AlwaysGrowsWhenTypingAtEdges", e[e.NeverGrowsWhenTypingAtEdges = 1] = "NeverGrowsWhenTypingAtEdges", e[e.GrowsOnlyWhenTypingBefore = 2] = "GrowsOnlyWhenTypingBefore", e[e.GrowsOnlyWhenTypingAfter = 3] = "GrowsOnlyWhenTypingAfter";
})(Ss || (Ss = {}));
var Bs;
(function(e) {
  e[e.None = 0] = "None", e[e.Same = 1] = "Same", e[e.Indent = 2] = "Indent", e[e.DeepIndent = 3] = "DeepIndent";
})(Bs || (Bs = {}));
const at = class at {
  static chord(t, n) {
    return E1(t, n);
  }
};
at.CtrlCmd = nt.CtrlCmd, at.Shift = nt.Shift, at.Alt = nt.Alt, at.WinCtrl = nt.WinCtrl;
let yn = at;
function M1() {
  return {
    editor: void 0,
    languages: void 0,
    CancellationTokenSource: Al,
    Emitter: Ae,
    KeyCode: In,
    KeyMod: yn,
    Position: X,
    Range: S,
    Selection: ge,
    SelectionDirection: Pn,
    MarkerSeverity: Sn,
    MarkerTag: Bn,
    Uri: Ye,
    Token: T1
  };
}
var mt;
(function(e) {
  e[e.Regular = 0] = "Regular", e[e.Whitespace = 1] = "Whitespace", e[e.WordSeparator = 2] = "WordSeparator";
})(mt || (mt = {}));
new sl(10);
var Ps;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Center = 2] = "Center", e[e.Right = 4] = "Right", e[e.Full = 7] = "Full";
})(Ps || (Ps = {}));
var ys;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Center = 2] = "Center", e[e.Right = 3] = "Right";
})(ys || (ys = {}));
var Os;
(function(e) {
  e[e.Inline = 1] = "Inline", e[e.Gutter = 2] = "Gutter";
})(Os || (Os = {}));
var Vs;
(function(e) {
  e[e.Normal = 1] = "Normal", e[e.Underlined = 2] = "Underlined";
})(Vs || (Vs = {}));
var Hs;
(function(e) {
  e[e.Both = 0] = "Both", e[e.Right = 1] = "Right", e[e.Left = 2] = "Left", e[e.None = 3] = "None";
})(Hs || (Hs = {}));
var qs;
(function(e) {
  e[e.TextDefined = 0] = "TextDefined", e[e.LF = 1] = "LF", e[e.CRLF = 2] = "CRLF";
})(qs || (qs = {}));
var Ws;
(function(e) {
  e[e.LF = 1] = "LF", e[e.CRLF = 2] = "CRLF";
})(Ws || (Ws = {}));
var Gs;
(function(e) {
  e[e.LF = 0] = "LF", e[e.CRLF = 1] = "CRLF";
})(Gs || (Gs = {}));
var $s;
(function(e) {
  e[e.AlwaysGrowsWhenTypingAtEdges = 0] = "AlwaysGrowsWhenTypingAtEdges", e[e.NeverGrowsWhenTypingAtEdges = 1] = "NeverGrowsWhenTypingAtEdges", e[e.GrowsOnlyWhenTypingBefore = 2] = "GrowsOnlyWhenTypingBefore", e[e.GrowsOnlyWhenTypingAfter = 3] = "GrowsOnlyWhenTypingAfter";
})($s || ($s = {}));
var zs;
(function(e) {
  e[e.Left = 0] = "Left", e[e.Right = 1] = "Right", e[e.None = 2] = "None", e[e.LeftOfInjectedText = 3] = "LeftOfInjectedText", e[e.RightOfInjectedText = 4] = "RightOfInjectedText";
})(zs || (zs = {}));
var js;
(function(e) {
  e[e.FIRST_LINE_DETECTION_LENGTH_LIMIT = 1e3] = "FIRST_LINE_DETECTION_LENGTH_LIMIT";
})(js || (js = {}));
function F1(e, t, n, i, s) {
  if (i === 0)
    return !0;
  const r = t.charCodeAt(i - 1);
  if (e.get(r) !== mt.Regular || r === d.CarriageReturn || r === d.LineFeed)
    return !0;
  if (s > 0) {
    const a = t.charCodeAt(i);
    if (e.get(a) !== mt.Regular)
      return !0;
  }
  return !1;
}
function k1(e, t, n, i, s) {
  if (i + s === n)
    return !0;
  const r = t.charCodeAt(i + s);
  if (e.get(r) !== mt.Regular || r === d.CarriageReturn || r === d.LineFeed)
    return !0;
  if (s > 0) {
    const a = t.charCodeAt(i + s - 1);
    if (e.get(a) !== mt.Regular)
      return !0;
  }
  return !1;
}
function I1(e, t, n, i, s) {
  return F1(e, t, n, i, s) && k1(e, t, n, i, s);
}
class S1 {
  constructor(t, n) {
    this._wordSeparators = t, this._searchRegex = n, this._prevMatchStartIndex = -1, this._prevMatchLength = 0;
  }
  reset(t) {
    this._searchRegex.lastIndex = t, this._prevMatchStartIndex = -1, this._prevMatchLength = 0;
  }
  next(t) {
    const n = t.length;
    let i;
    do {
      if (this._prevMatchStartIndex + this._prevMatchLength === n || (i = this._searchRegex.exec(t), !i))
        return null;
      const s = i.index, r = i[0].length;
      if (s === this._prevMatchStartIndex && r === this._prevMatchLength) {
        if (r === 0) {
          Ml(t, n, this._searchRegex.lastIndex) > 65535 ? this._searchRegex.lastIndex += 2 : this._searchRegex.lastIndex += 1;
          continue;
        }
        return null;
      }
      if (this._prevMatchStartIndex = s, this._prevMatchLength = r, !this._wordSeparators || I1(this._wordSeparators, t, n, s, r))
        return i;
    } while (i);
    return null;
  }
}
function B1(e, t = "Unreachable") {
  throw new Error(t);
}
function Kt(e) {
  if (!e()) {
    debugger;
    e(), yt(new me("Assertion Failed"));
  }
}
function kr(e, t) {
  let n = 0;
  for (; n < e.length - 1; ) {
    const i = e[n], s = e[n + 1];
    if (!t(i, s))
      return !1;
    n++;
  }
  return !0;
}
class P1 {
  static computeUnicodeHighlights(t, n, i) {
    const s = i ? i.startLineNumber : 1, r = i ? i.endLineNumber : t.getLineCount(), a = new Xs(n), u = a.getCandidateCodePoints();
    let o;
    u === "allNonBasicAscii" ? o = new RegExp("[^\\t\\n\\r\\x20-\\x7E]", "g") : o = new RegExp(`${y1(Array.from(u))}`, "g");
    const c = new S1(null, o), h = [];
    let f = !1, g, b = 0, L = 0, N = 0;
    e: for (let A = s, x = r; A <= x; A++) {
      const R = t.getLineContent(A), U = R.length;
      c.reset(0);
      do
        if (g = c.next(R), g) {
          let p = g.index, w = g.index + g[0].length;
          if (p > 0) {
            const $ = R.charCodeAt(p - 1);
            Xt($) && p--;
          }
          if (w + 1 < U) {
            const $ = R.charCodeAt(w - 1);
            Xt($) && w++;
          }
          const D = R.substring(p, w);
          let T = $n(p + 1, Tr, R, 0);
          T && T.endColumn <= p + 1 && (T = null);
          const I = a.shouldHighlightNonBasicASCII(D, T ? T.word : null);
          if (I !== ce.None) {
            if (I === ce.Ambiguous ? b++ : I === ce.Invisible ? L++ : I === ce.NonBasicASCII ? N++ : B1(), h.length >= 1e3) {
              f = !0;
              break e;
            }
            h.push(new S(A, p + 1, A, w + 1));
          }
        }
      while (g);
    }
    return {
      ranges: h,
      hasMore: f,
      ambiguousCharacterCount: b,
      invisibleCharacterCount: L,
      nonBasicAsciiCharacterCount: N
    };
  }
  static computeUnicodeHighlightReason(t, n) {
    const i = new Xs(n);
    switch (i.shouldHighlightNonBasicASCII(t, null)) {
      case ce.None:
        return null;
      case ce.Invisible:
        return { kind: At.Invisible };
      case ce.Ambiguous: {
        const r = t.codePointAt(0), a = i.ambiguousCharacters.getPrimaryConfusable(r), u = Ft.getLocales().filter((o) => !Ft.getInstance(/* @__PURE__ */ new Set([...n.allowedLocales, o])).isAmbiguous(r));
        return { kind: At.Ambiguous, confusableWith: String.fromCodePoint(a), notAmbiguousInLocales: u };
      }
      case ce.NonBasicASCII:
        return { kind: At.NonBasicAscii };
    }
  }
}
function y1(e, t) {
  return `[${vl(e.map((i) => String.fromCodePoint(i)).join(""))}]`;
}
var At;
(function(e) {
  e[e.Ambiguous = 0] = "Ambiguous", e[e.Invisible = 1] = "Invisible", e[e.NonBasicAscii = 2] = "NonBasicAscii";
})(At || (At = {}));
class Xs {
  constructor(t) {
    this.options = t, this.allowedCodePoints = new Set(t.allowedCodePoints), this.ambiguousCharacters = Ft.getInstance(new Set(t.allowedLocales));
  }
  getCandidateCodePoints() {
    if (this.options.nonBasicASCII)
      return "allNonBasicAscii";
    const t = /* @__PURE__ */ new Set();
    if (this.options.invisibleCharacters)
      for (const n of Et.codePoints)
        Ys(String.fromCodePoint(n)) || t.add(n);
    if (this.options.ambiguousCharacters)
      for (const n of this.ambiguousCharacters.getConfusableCodePoints())
        t.add(n);
    for (const n of this.allowedCodePoints)
      t.delete(n);
    return t;
  }
  shouldHighlightNonBasicASCII(t, n) {
    const i = t.codePointAt(0);
    if (this.allowedCodePoints.has(i))
      return ce.None;
    if (this.options.nonBasicASCII)
      return ce.NonBasicASCII;
    let s = !1, r = !1;
    if (n)
      for (const a of n) {
        const u = a.codePointAt(0), o = kl(a);
        s = s || o, !o && !this.ambiguousCharacters.isAmbiguous(u) && !Et.isInvisibleCharacter(u) && (r = !0);
      }
    return !s && r ? ce.None : this.options.invisibleCharacters && !Ys(t) && Et.isInvisibleCharacter(i) ? ce.Invisible : this.options.ambiguousCharacters && this.ambiguousCharacters.isAmbiguous(i) ? ce.Ambiguous : ce.None;
  }
}
function Ys(e) {
  return e === " " || e === `
` || e === "	";
}
var ce;
(function(e) {
  e[e.None = 0] = "None", e[e.NonBasicASCII = 1] = "NonBasicASCII", e[e.Invisible = 2] = "Invisible", e[e.Ambiguous = 3] = "Ambiguous";
})(ce || (ce = {}));
class Wt {
  constructor(t, n, i) {
    this.changes = t, this.moves = n, this.hitTimeout = i;
  }
}
class Xn {
  constructor(t, n) {
    this.lineRangeMapping = t, this.changes = n;
  }
  flip() {
    return new Xn(this.lineRangeMapping.flip(), this.changes.map((t) => t.flip()));
  }
}
class W {
  static addRange(t, n) {
    let i = 0;
    for (; i < n.length && n[i].endExclusive < t.start; )
      i++;
    let s = i;
    for (; s < n.length && n[s].start <= t.endExclusive; )
      s++;
    if (i === s)
      n.splice(i, 0, t);
    else {
      const r = Math.min(t.start, n[i].start), a = Math.max(t.endExclusive, n[s - 1].endExclusive);
      n.splice(i, s - i, new W(r, a));
    }
  }
  static tryCreate(t, n) {
    if (!(t > n))
      return new W(t, n);
  }
  static ofLength(t) {
    return new W(0, t);
  }
  static ofStartAndLength(t, n) {
    return new W(t, t + n);
  }
  constructor(t, n) {
    if (this.start = t, this.endExclusive = n, t > n)
      throw new me(`Invalid range: ${this.toString()}`);
  }
  get isEmpty() {
    return this.start === this.endExclusive;
  }
  delta(t) {
    return new W(this.start + t, this.endExclusive + t);
  }
  deltaStart(t) {
    return new W(this.start + t, this.endExclusive);
  }
  deltaEnd(t) {
    return new W(this.start, this.endExclusive + t);
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
  join(t) {
    return new W(
      Math.min(this.start, t.start),
      Math.max(this.endExclusive, t.endExclusive)
    );
  }
  intersect(t) {
    const n = Math.max(this.start, t.start), i = Math.min(this.endExclusive, t.endExclusive);
    if (n <= i)
      return new W(n, i);
  }
  intersects(t) {
    const n = Math.max(this.start, t.start), i = Math.min(this.endExclusive, t.endExclusive);
    return n < i;
  }
  intersectsOrTouches(t) {
    const n = Math.max(this.start, t.start), i = Math.min(this.endExclusive, t.endExclusive);
    return n <= i;
  }
  isBefore(t) {
    return this.endExclusive <= t.start;
  }
  isAfter(t) {
    return this.start >= t.endExclusive;
  }
  slice(t) {
    return t.slice(this.start, this.endExclusive);
  }
  substring(t) {
    return t.substring(this.start, this.endExclusive);
  }
  clip(t) {
    if (this.isEmpty)
      throw new me(`Invalid clipping range: ${this.toString()}`);
    return Math.max(this.start, Math.min(this.endExclusive - 1, t));
  }
  clipCyclic(t) {
    if (this.isEmpty)
      throw new me(`Invalid clipping range: ${this.toString()}`);
    return t < this.start ? this.endExclusive - (this.start - t) % this.length : t >= this.endExclusive ? this.start + (t - this.start) % this.length : t;
  }
  map(t) {
    const n = [];
    for (let i = this.start; i < this.endExclusive; i++)
      n.push(t(i));
    return n;
  }
  forEach(t) {
    for (let n = this.start; n < this.endExclusive; n++)
      t(n);
  }
}
class V {
  static fromRange(t) {
    return new V(t.startLineNumber, t.endLineNumber);
  }
  static fromRangeInclusive(t) {
    return new V(t.startLineNumber, t.endLineNumber + 1);
  }
  static subtract(t, n) {
    return n ? t.startLineNumber < n.startLineNumber && n.endLineNumberExclusive < t.endLineNumberExclusive ? [
      new V(t.startLineNumber, n.startLineNumber),
      new V(n.endLineNumberExclusive, t.endLineNumberExclusive)
    ] : n.startLineNumber <= t.startLineNumber && t.endLineNumberExclusive <= n.endLineNumberExclusive ? [] : n.endLineNumberExclusive < t.endLineNumberExclusive ? [new V(
      Math.max(n.endLineNumberExclusive, t.startLineNumber),
      t.endLineNumberExclusive
    )] : [new V(t.startLineNumber, Math.min(n.startLineNumber, t.endLineNumberExclusive))] : [t];
  }
  static joinMany(t) {
    if (t.length === 0)
      return [];
    let n = new Ue(t[0].slice());
    for (let i = 1; i < t.length; i++)
      n = n.getUnion(new Ue(t[i].slice()));
    return n.ranges;
  }
  static join(t) {
    if (t.length === 0)
      throw new me("lineRanges cannot be empty");
    let n = t[0].startLineNumber, i = t[0].endLineNumberExclusive;
    for (let s = 1; s < t.length; s++)
      n = Math.min(n, t[s].startLineNumber), i = Math.max(i, t[s].endLineNumberExclusive);
    return new V(n, i);
  }
  static ofLength(t, n) {
    return new V(t, t + n);
  }
  static deserialize(t) {
    return new V(t[0], t[1]);
  }
  constructor(t, n) {
    if (t > n)
      throw new me(
        `startLineNumber ${t} cannot be after endLineNumberExclusive ${n}`
      );
    this.startLineNumber = t, this.endLineNumberExclusive = n;
  }
  contains(t) {
    return this.startLineNumber <= t && t < this.endLineNumberExclusive;
  }
  get isEmpty() {
    return this.startLineNumber === this.endLineNumberExclusive;
  }
  delta(t) {
    return new V(this.startLineNumber + t, this.endLineNumberExclusive + t);
  }
  deltaLength(t) {
    return new V(this.startLineNumber, this.endLineNumberExclusive + t);
  }
  get length() {
    return this.endLineNumberExclusive - this.startLineNumber;
  }
  join(t) {
    return new V(
      Math.min(this.startLineNumber, t.startLineNumber),
      Math.max(this.endLineNumberExclusive, t.endLineNumberExclusive)
    );
  }
  toString() {
    return `[${this.startLineNumber},${this.endLineNumberExclusive})`;
  }
  intersect(t) {
    const n = Math.max(this.startLineNumber, t.startLineNumber), i = Math.min(this.endLineNumberExclusive, t.endLineNumberExclusive);
    if (n <= i)
      return new V(n, i);
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
    return this.isEmpty ? null : new S(
      this.startLineNumber,
      1,
      this.endLineNumberExclusive - 1,
      Number.MAX_SAFE_INTEGER
    );
  }
  toExclusiveRange() {
    return new S(this.startLineNumber, 1, this.endLineNumberExclusive, 1);
  }
  mapToLineArray(t) {
    const n = [];
    for (let i = this.startLineNumber; i < this.endLineNumberExclusive; i++)
      n.push(t(i));
    return n;
  }
  forEach(t) {
    for (let n = this.startLineNumber; n < this.endLineNumberExclusive; n++)
      t(n);
  }
  serialize() {
    return [this.startLineNumber, this.endLineNumberExclusive];
  }
  includes(t) {
    return this.startLineNumber <= t && t < this.endLineNumberExclusive;
  }
  toOffsetRange() {
    return new W(this.startLineNumber - 1, this.endLineNumberExclusive - 1);
  }
}
class Ue {
  constructor(t = []) {
    this._normalizedRanges = t;
  }
  get ranges() {
    return this._normalizedRanges;
  }
  addRange(t) {
    if (t.length === 0)
      return;
    const n = Ln(this._normalizedRanges, (s) => s.endLineNumberExclusive >= t.startLineNumber), i = vt(this._normalizedRanges, (s) => s.startLineNumber <= t.endLineNumberExclusive) + 1;
    if (n === i)
      this._normalizedRanges.splice(n, 0, t);
    else if (n === i - 1) {
      const s = this._normalizedRanges[n];
      this._normalizedRanges[n] = s.join(t);
    } else {
      const s = this._normalizedRanges[n].join(this._normalizedRanges[i - 1]).join(t);
      this._normalizedRanges.splice(n, i - n, s);
    }
  }
  contains(t) {
    const n = ft(this._normalizedRanges, (i) => i.startLineNumber <= t);
    return !!n && n.endLineNumberExclusive > t;
  }
  intersects(t) {
    const n = ft(this._normalizedRanges, (i) => i.startLineNumber < t.endLineNumberExclusive);
    return !!n && n.endLineNumberExclusive > t.startLineNumber;
  }
  getUnion(t) {
    if (this._normalizedRanges.length === 0)
      return t;
    if (t._normalizedRanges.length === 0)
      return this;
    const n = [];
    let i = 0, s = 0, r = null;
    for (; i < this._normalizedRanges.length || s < t._normalizedRanges.length; ) {
      let a = null;
      if (i < this._normalizedRanges.length && s < t._normalizedRanges.length) {
        const u = this._normalizedRanges[i], o = t._normalizedRanges[s];
        u.startLineNumber < o.startLineNumber ? (a = u, i++) : (a = o, s++);
      } else i < this._normalizedRanges.length ? (a = this._normalizedRanges[i], i++) : (a = t._normalizedRanges[s], s++);
      r === null ? r = a : r.endLineNumberExclusive >= a.startLineNumber ? r = new V(
        r.startLineNumber,
        Math.max(r.endLineNumberExclusive, a.endLineNumberExclusive)
      ) : (n.push(r), r = a);
    }
    return r !== null && n.push(r), new Ue(n);
  }
  subtractFrom(t) {
    const n = Ln(this._normalizedRanges, (a) => a.endLineNumberExclusive >= t.startLineNumber), i = vt(this._normalizedRanges, (a) => a.startLineNumber <= t.endLineNumberExclusive) + 1;
    if (n === i)
      return new Ue([t]);
    const s = [];
    let r = t.startLineNumber;
    for (let a = n; a < i; a++) {
      const u = this._normalizedRanges[a];
      u.startLineNumber > r && s.push(new V(r, u.startLineNumber)), r = u.endLineNumberExclusive;
    }
    return r < t.endLineNumberExclusive && s.push(new V(r, t.endLineNumberExclusive)), new Ue(s);
  }
  toString() {
    return this._normalizedRanges.map((t) => t.toString()).join(", ");
  }
  getIntersection(t) {
    const n = [];
    let i = 0, s = 0;
    for (; i < this._normalizedRanges.length && s < t._normalizedRanges.length; ) {
      const r = this._normalizedRanges[i], a = t._normalizedRanges[s], u = r.intersect(a);
      u && !u.isEmpty && n.push(u), r.endLineNumberExclusive < a.endLineNumberExclusive ? i++ : s++;
    }
    return new Ue(n);
  }
  getWithDelta(t) {
    return new Ue(this._normalizedRanges.map((n) => n.delta(t)));
  }
}
const _e = class _e {
  static lengthDiffNonNegative(t, n) {
    return n.isLessThan(t) ? _e.zero : t.lineCount === n.lineCount ? new _e(0, n.columnCount - t.columnCount) : new _e(n.lineCount - t.lineCount, n.columnCount);
  }
  static betweenPositions(t, n) {
    return t.lineNumber === n.lineNumber ? new _e(0, n.column - t.column) : new _e(n.lineNumber - t.lineNumber, n.column - 1);
  }
  static ofRange(t) {
    return _e.betweenPositions(t.getStartPosition(), t.getEndPosition());
  }
  static ofText(t) {
    let n = 0, i = 0;
    for (const s of t)
      s === `
` ? (n++, i = 0) : i++;
    return new _e(n, i);
  }
  constructor(t, n) {
    this.lineCount = t, this.columnCount = n;
  }
  isZero() {
    return this.lineCount === 0 && this.columnCount === 0;
  }
  isLessThan(t) {
    return this.lineCount !== t.lineCount ? this.lineCount < t.lineCount : this.columnCount < t.columnCount;
  }
  isGreaterThan(t) {
    return this.lineCount !== t.lineCount ? this.lineCount > t.lineCount : this.columnCount > t.columnCount;
  }
  isGreaterThanOrEqualTo(t) {
    return this.lineCount !== t.lineCount ? this.lineCount > t.lineCount : this.columnCount >= t.columnCount;
  }
  equals(t) {
    return this.lineCount === t.lineCount && this.columnCount === t.columnCount;
  }
  compare(t) {
    return this.lineCount !== t.lineCount ? this.lineCount - t.lineCount : this.columnCount - t.columnCount;
  }
  add(t) {
    return t.lineCount === 0 ? new _e(this.lineCount, this.columnCount + t.columnCount) : new _e(this.lineCount + t.lineCount, t.columnCount);
  }
  createRange(t) {
    return this.lineCount === 0 ? new S(
      t.lineNumber,
      t.column,
      t.lineNumber,
      t.column + this.columnCount
    ) : new S(
      t.lineNumber,
      t.column,
      t.lineNumber + this.lineCount,
      this.columnCount + 1
    );
  }
  toRange() {
    return new S(1, 1, this.lineCount + 1, this.columnCount + 1);
  }
  addToPosition(t) {
    return this.lineCount === 0 ? new X(t.lineNumber, t.column + this.columnCount) : new X(t.lineNumber + this.lineCount, this.columnCount + 1);
  }
  toString() {
    return `${this.lineCount},${this.columnCount}`;
  }
};
_e.zero = new _e(0, 0);
let Qs = _e;
class O1 {
  constructor(t, n) {
    this.range = t, this.text = n;
  }
  get isEmpty() {
    return this.range.isEmpty() && this.text.length === 0;
  }
  static equals(t, n) {
    return t.range.equalsRange(n.range) && t.text === n.text;
  }
  toSingleEditOperation() {
    return {
      range: this.range,
      text: this.text
    };
  }
}
class Ne {
  static inverse(t, n, i) {
    const s = [];
    let r = 1, a = 1;
    for (const o of t) {
      const c = new Ne(new V(r, o.original.startLineNumber), new V(a, o.modified.startLineNumber));
      c.modified.isEmpty || s.push(c), r = o.original.endLineNumberExclusive, a = o.modified.endLineNumberExclusive;
    }
    const u = new Ne(new V(r, n + 1), new V(a, i + 1));
    return u.modified.isEmpty || s.push(u), s;
  }
  static clip(t, n, i) {
    const s = [];
    for (const r of t) {
      const a = r.original.intersect(n), u = r.modified.intersect(i);
      a && !a.isEmpty && u && !u.isEmpty && s.push(new Ne(a, u));
    }
    return s;
  }
  constructor(t, n) {
    this.original = t, this.modified = n;
  }
  toString() {
    return `{${this.original.toString()}->${this.modified.toString()}}`;
  }
  flip() {
    return new Ne(this.modified, this.original);
  }
  join(t) {
    return new Ne(this.original.join(t.original), this.modified.join(t.modified));
  }
  get changedLineCount() {
    return Math.max(this.original.length, this.modified.length);
  }
  toRangeMapping() {
    const t = this.original.toInclusiveRange(), n = this.modified.toInclusiveRange();
    if (t && n)
      return new Re(t, n);
    if (this.original.startLineNumber === 1 || this.modified.startLineNumber === 1) {
      if (!(this.modified.startLineNumber === 1 && this.original.startLineNumber === 1))
        throw new me("not a valid diff");
      return new Re(new S(this.original.startLineNumber, 1, this.original.endLineNumberExclusive, 1), new S(this.modified.startLineNumber, 1, this.modified.endLineNumberExclusive, 1));
    } else
      return new Re(new S(
        this.original.startLineNumber - 1,
        Number.MAX_SAFE_INTEGER,
        this.original.endLineNumberExclusive - 1,
        Number.MAX_SAFE_INTEGER
      ), new S(
        this.modified.startLineNumber - 1,
        Number.MAX_SAFE_INTEGER,
        this.modified.endLineNumberExclusive - 1,
        Number.MAX_SAFE_INTEGER
      ));
  }
  toRangeMapping2(t, n) {
    if (Zs(this.original.endLineNumberExclusive, t) && Zs(this.modified.endLineNumberExclusive, n))
      return new Re(new S(this.original.startLineNumber, 1, this.original.endLineNumberExclusive, 1), new S(this.modified.startLineNumber, 1, this.modified.endLineNumberExclusive, 1));
    if (!this.original.isEmpty && !this.modified.isEmpty)
      return new Re(S.fromPositions(new X(this.original.startLineNumber, 1), et(new X(this.original.endLineNumberExclusive - 1, Number.MAX_SAFE_INTEGER), t)), S.fromPositions(new X(this.modified.startLineNumber, 1), et(new X(this.modified.endLineNumberExclusive - 1, Number.MAX_SAFE_INTEGER), n)));
    if (this.original.startLineNumber > 1 && this.modified.startLineNumber > 1)
      return new Re(S.fromPositions(et(new X(this.original.startLineNumber - 1, Number.MAX_SAFE_INTEGER), t), et(new X(this.original.endLineNumberExclusive - 1, Number.MAX_SAFE_INTEGER), t)), S.fromPositions(et(new X(this.modified.startLineNumber - 1, Number.MAX_SAFE_INTEGER), n), et(new X(this.modified.endLineNumberExclusive - 1, Number.MAX_SAFE_INTEGER), n)));
    throw new me();
  }
}
function et(e, t) {
  if (e.lineNumber < 1)
    return new X(1, 1);
  if (e.lineNumber > t.length)
    return new X(t.length, t[t.length - 1].length + 1);
  const n = t[e.lineNumber - 1];
  return e.column > n.length + 1 ? new X(e.lineNumber, n.length + 1) : e;
}
function Zs(e, t) {
  return e >= 1 && e <= t.length;
}
class Fe extends Ne {
  static fromRangeMappings(t) {
    const n = V.join(t.map((s) => V.fromRangeInclusive(s.originalRange))), i = V.join(t.map((s) => V.fromRangeInclusive(s.modifiedRange)));
    return new Fe(n, i, t);
  }
  constructor(t, n, i) {
    super(t, n), this.innerChanges = i;
  }
  flip() {
    var t;
    return new Fe(this.modified, this.original, (t = this.innerChanges) == null ? void 0 : t.map((n) => n.flip()));
  }
  withInnerChangesFromLineRanges() {
    return new Fe(this.original, this.modified, [this.toRangeMapping()]);
  }
}
class Re {
  static assertSorted(t) {
    for (let n = 1; n < t.length; n++) {
      const i = t[n - 1], s = t[n];
      if (!(i.originalRange.getEndPosition().isBeforeOrEqual(s.originalRange.getStartPosition()) && i.modifiedRange.getEndPosition().isBeforeOrEqual(s.modifiedRange.getStartPosition())))
        throw new me("Range mappings must be sorted");
    }
  }
  constructor(t, n) {
    this.originalRange = t, this.modifiedRange = n;
  }
  toString() {
    return `{${this.originalRange.toString()}->${this.modifiedRange.toString()}}`;
  }
  flip() {
    return new Re(this.modifiedRange, this.originalRange);
  }
  toTextEdit(t) {
    const n = t.getValueOfRange(this.modifiedRange);
    return new O1(this.originalRange, n);
  }
}
const V1 = 3;
class H1 {
  computeDiff(t, n, i) {
    var o;
    const r = new Sr(t, n, {
      maxComputationTime: i.maxComputationTimeMs,
      shouldIgnoreTrimWhitespace: i.ignoreTrimWhitespace,
      shouldComputeCharChanges: !0,
      shouldMakePrettyDiff: !0,
      shouldPostProcessCharChanges: !0
    }).computeDiff(), a = [];
    let u = null;
    for (const c of r.changes) {
      let h;
      c.originalEndLineNumber === 0 ? h = new V(c.originalStartLineNumber + 1, c.originalStartLineNumber + 1) : h = new V(c.originalStartLineNumber, c.originalEndLineNumber + 1);
      let f;
      c.modifiedEndLineNumber === 0 ? f = new V(c.modifiedStartLineNumber + 1, c.modifiedStartLineNumber + 1) : f = new V(c.modifiedStartLineNumber, c.modifiedEndLineNumber + 1);
      let g = new Fe(h, f, (o = c.charChanges) == null ? void 0 : o.map((b) => new Re(new S(
        b.originalStartLineNumber,
        b.originalStartColumn,
        b.originalEndLineNumber,
        b.originalEndColumn
      ), new S(
        b.modifiedStartLineNumber,
        b.modifiedStartColumn,
        b.modifiedEndLineNumber,
        b.modifiedEndColumn
      ))));
      u && (u.modified.endLineNumberExclusive === g.modified.startLineNumber || u.original.endLineNumberExclusive === g.original.startLineNumber) && (g = new Fe(
        u.original.join(g.original),
        u.modified.join(g.modified),
        u.innerChanges && g.innerChanges ? u.innerChanges.concat(g.innerChanges) : void 0
      ), a.pop()), a.push(g), u = g;
    }
    return Kt(() => kr(a, (c, h) => h.original.startLineNumber - c.original.endLineNumberExclusive === h.modified.startLineNumber - c.modified.endLineNumberExclusive && c.original.endLineNumberExclusive < h.original.startLineNumber && c.modified.endLineNumberExclusive < h.modified.startLineNumber)), new Wt(a, [], r.quitEarly);
  }
}
function Ir(e, t, n, i) {
  return new Ve(e, t, n).ComputeDiff(i);
}
let Js = class {
  constructor(t) {
    const n = [], i = [];
    for (let s = 0, r = t.length; s < r; s++)
      n[s] = On(t[s], 1), i[s] = Vn(t[s], 1);
    this.lines = t, this._startColumns = n, this._endColumns = i;
  }
  getElements() {
    const t = [];
    for (let n = 0, i = this.lines.length; n < i; n++)
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
  createCharSequence(t, n, i) {
    const s = [], r = [], a = [];
    let u = 0;
    for (let o = n; o <= i; o++) {
      const c = this.lines[o], h = t ? this._startColumns[o] : 1, f = t ? this._endColumns[o] : c.length + 1;
      for (let g = h; g < f; g++)
        s[u] = c.charCodeAt(g - 1), r[u] = o + 1, a[u] = g, u++;
      !t && o < i && (s[u] = d.LineFeed, r[u] = o + 1, a[u] = c.length + 1, u++);
    }
    return new q1(s, r, a);
  }
};
class q1 {
  constructor(t, n, i) {
    this._charCodes = t, this._lineNumbers = n, this._columns = i;
  }
  toString() {
    return "[" + this._charCodes.map(
      (t, n) => (t === d.LineFeed ? "\\n" : String.fromCharCode(t)) + `-(${this._lineNumbers[n]},${this._columns[n]})`
    ).join(", ") + "]";
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
    return t === -1 ? this.getStartLineNumber(t + 1) : (this._assertIndex(t, this._lineNumbers), this._charCodes[t] === d.LineFeed ? this._lineNumbers[t] + 1 : this._lineNumbers[t]);
  }
  getStartColumn(t) {
    return t > 0 && t === this._columns.length ? this.getEndColumn(t - 1) : (this._assertIndex(t, this._columns), this._columns[t]);
  }
  getEndColumn(t) {
    return t === -1 ? this.getStartColumn(t + 1) : (this._assertIndex(t, this._columns), this._charCodes[t] === d.LineFeed ? 1 : this._columns[t] + 1);
  }
}
class ot {
  constructor(t, n, i, s, r, a, u, o) {
    this.originalStartLineNumber = t, this.originalStartColumn = n, this.originalEndLineNumber = i, this.originalEndColumn = s, this.modifiedStartLineNumber = r, this.modifiedStartColumn = a, this.modifiedEndLineNumber = u, this.modifiedEndColumn = o;
  }
  static createFromDiffChange(t, n, i) {
    const s = n.getStartLineNumber(t.originalStart), r = n.getStartColumn(t.originalStart), a = n.getEndLineNumber(t.originalStart + t.originalLength - 1), u = n.getEndColumn(t.originalStart + t.originalLength - 1), o = i.getStartLineNumber(t.modifiedStart), c = i.getStartColumn(t.modifiedStart), h = i.getEndLineNumber(t.modifiedStart + t.modifiedLength - 1), f = i.getEndColumn(t.modifiedStart + t.modifiedLength - 1);
    return new ot(
      s,
      r,
      a,
      u,
      o,
      c,
      h,
      f
    );
  }
}
function W1(e) {
  if (e.length <= 1)
    return e;
  const t = [e[0]];
  let n = t[0];
  for (let i = 1, s = e.length; i < s; i++) {
    const r = e[i], a = r.originalStart - (n.originalStart + n.originalLength), u = r.modifiedStart - (n.modifiedStart + n.modifiedLength);
    Math.min(a, u) < V1 ? (n.originalLength = r.originalStart + r.originalLength - n.originalStart, n.modifiedLength = r.modifiedStart + r.modifiedLength - n.modifiedStart) : (t.push(r), n = r);
  }
  return t;
}
class xt {
  constructor(t, n, i, s, r) {
    this.originalStartLineNumber = t, this.originalEndLineNumber = n, this.modifiedStartLineNumber = i, this.modifiedEndLineNumber = s, this.charChanges = r;
  }
  static createFromDiffResult(t, n, i, s, r, a, u) {
    let o, c, h, f, g;
    if (n.originalLength === 0 ? (o = i.getStartLineNumber(n.originalStart) - 1, c = 0) : (o = i.getStartLineNumber(n.originalStart), c = i.getEndLineNumber(n.originalStart + n.originalLength - 1)), n.modifiedLength === 0 ? (h = s.getStartLineNumber(n.modifiedStart) - 1, f = 0) : (h = s.getStartLineNumber(n.modifiedStart), f = s.getEndLineNumber(n.modifiedStart + n.modifiedLength - 1)), a && n.originalLength > 0 && n.originalLength < 20 && n.modifiedLength > 0 && n.modifiedLength < 20 && r()) {
      const b = i.createCharSequence(t, n.originalStart, n.originalStart + n.originalLength - 1), L = s.createCharSequence(t, n.modifiedStart, n.modifiedStart + n.modifiedLength - 1);
      if (b.getElements().length > 0 && L.getElements().length > 0) {
        let N = Ir(b, L, r, !0).changes;
        u && (N = W1(N)), g = [];
        for (let A = 0, x = N.length; A < x; A++)
          g.push(ot.createFromDiffChange(N[A], b, L));
      }
    }
    return new xt(
      o,
      c,
      h,
      f,
      g
    );
  }
}
class Sr {
  constructor(t, n, i) {
    this.shouldComputeCharChanges = i.shouldComputeCharChanges, this.shouldPostProcessCharChanges = i.shouldPostProcessCharChanges, this.shouldIgnoreTrimWhitespace = i.shouldIgnoreTrimWhitespace, this.shouldMakePrettyDiff = i.shouldMakePrettyDiff, this.originalLines = t, this.modifiedLines = n, this.original = new Js(t), this.modified = new Js(n), this.continueLineDiff = Ks(i.maxComputationTime), this.continueCharDiff = Ks(i.maxComputationTime === 0 ? 0 : Math.min(i.maxComputationTime, 5e3));
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
    const t = Ir(this.original, this.modified, this.continueLineDiff, this.shouldMakePrettyDiff), n = t.changes, i = t.quitEarly;
    if (this.shouldIgnoreTrimWhitespace) {
      const u = [];
      for (let o = 0, c = n.length; o < c; o++)
        u.push(xt.createFromDiffResult(this.shouldIgnoreTrimWhitespace, n[o], this.original, this.modified, this.continueCharDiff, this.shouldComputeCharChanges, this.shouldPostProcessCharChanges));
      return {
        quitEarly: i,
        changes: u
      };
    }
    const s = [];
    let r = 0, a = 0;
    for (let u = -1, o = n.length; u < o; u++) {
      const c = u + 1 < o ? n[u + 1] : null, h = c ? c.originalStart : this.originalLines.length, f = c ? c.modifiedStart : this.modifiedLines.length;
      for (; r < h && a < f; ) {
        const g = this.originalLines[r], b = this.modifiedLines[a];
        if (g !== b) {
          {
            let L = On(g, 1), N = On(b, 1);
            for (; L > 1 && N > 1; ) {
              const A = g.charCodeAt(L - 2), x = b.charCodeAt(N - 2);
              if (A !== x)
                break;
              L--, N--;
            }
            (L > 1 || N > 1) && this._pushTrimWhitespaceCharChange(s, r + 1, 1, L, a + 1, 1, N);
          }
          {
            let L = Vn(g, 1), N = Vn(b, 1);
            const A = g.length + 1, x = b.length + 1;
            for (; L < A && N < x; ) {
              const R = g.charCodeAt(L - 1), U = g.charCodeAt(N - 1);
              if (R !== U)
                break;
              L++, N++;
            }
            (L < A || N < x) && this._pushTrimWhitespaceCharChange(s, r + 1, L, A, a + 1, N, x);
          }
        }
        r++, a++;
      }
      c && (s.push(xt.createFromDiffResult(this.shouldIgnoreTrimWhitespace, c, this.original, this.modified, this.continueCharDiff, this.shouldComputeCharChanges, this.shouldPostProcessCharChanges)), r += c.originalLength, a += c.modifiedLength);
    }
    return {
      quitEarly: i,
      changes: s
    };
  }
  _pushTrimWhitespaceCharChange(t, n, i, s, r, a, u) {
    if (this._mergeTrimWhitespaceCharChange(t, n, i, s, r, a, u))
      return;
    let o;
    this.shouldComputeCharChanges && (o = [new ot(
      n,
      i,
      n,
      s,
      r,
      a,
      r,
      u
    )]), t.push(new xt(
      n,
      n,
      r,
      r,
      o
    ));
  }
  _mergeTrimWhitespaceCharChange(t, n, i, s, r, a, u) {
    const o = t.length;
    if (o === 0)
      return !1;
    const c = t[o - 1];
    return c.originalEndLineNumber === 0 || c.modifiedEndLineNumber === 0 ? !1 : c.originalEndLineNumber === n && c.modifiedEndLineNumber === r ? (this.shouldComputeCharChanges && c.charChanges && c.charChanges.push(new ot(
      n,
      i,
      n,
      s,
      r,
      a,
      r,
      u
    )), !0) : c.originalEndLineNumber + 1 === n && c.modifiedEndLineNumber + 1 === r ? (c.originalEndLineNumber = n, c.modifiedEndLineNumber = r, this.shouldComputeCharChanges && c.charChanges && c.charChanges.push(new ot(
      n,
      i,
      n,
      s,
      r,
      a,
      r,
      u
    )), !0) : !1;
  }
}
function On(e, t) {
  const n = Dl(e);
  return n === -1 ? t : n + 1;
}
function Vn(e, t) {
  const n = Tl(e);
  return n === -1 ? t : n + 2;
}
function Ks(e) {
  if (e === 0)
    return () => !0;
  const t = Date.now();
  return () => Date.now() - t < e;
}
class ke {
  static trivial(t, n) {
    return new ke([new C(W.ofLength(t.length), W.ofLength(n.length))], !1);
  }
  static trivialTimedOut(t, n) {
    return new ke([new C(W.ofLength(t.length), W.ofLength(n.length))], !0);
  }
  constructor(t, n) {
    this.diffs = t, this.hitTimeout = n;
  }
}
class C {
  static invert(t, n) {
    const i = [];
    return Qr(t, (s, r) => {
      i.push(C.fromOffsetPairs(s ? s.getEndExclusives() : Me.zero, r ? r.getStarts() : new Me(
        n,
        (s ? s.seq2Range.endExclusive - s.seq1Range.endExclusive : 0) + n
      )));
    }), i;
  }
  static fromOffsetPairs(t, n) {
    return new C(new W(t.offset1, n.offset1), new W(t.offset2, n.offset2));
  }
  static assertSorted(t) {
    let n;
    for (const i of t) {
      if (n && !(n.seq1Range.endExclusive <= i.seq1Range.start && n.seq2Range.endExclusive <= i.seq2Range.start))
        throw new me("Sequence diffs must be sorted");
      n = i;
    }
  }
  constructor(t, n) {
    this.seq1Range = t, this.seq2Range = n;
  }
  swap() {
    return new C(this.seq2Range, this.seq1Range);
  }
  toString() {
    return `${this.seq1Range} <-> ${this.seq2Range}`;
  }
  join(t) {
    return new C(this.seq1Range.join(t.seq1Range), this.seq2Range.join(t.seq2Range));
  }
  delta(t) {
    return t === 0 ? this : new C(this.seq1Range.delta(t), this.seq2Range.delta(t));
  }
  deltaStart(t) {
    return t === 0 ? this : new C(this.seq1Range.deltaStart(t), this.seq2Range.deltaStart(t));
  }
  deltaEnd(t) {
    return t === 0 ? this : new C(this.seq1Range.deltaEnd(t), this.seq2Range.deltaEnd(t));
  }
  intersectsOrTouches(t) {
    return this.seq1Range.intersectsOrTouches(t.seq1Range) || this.seq2Range.intersectsOrTouches(t.seq2Range);
  }
  intersect(t) {
    const n = this.seq1Range.intersect(t.seq1Range), i = this.seq2Range.intersect(t.seq2Range);
    if (!(!n || !i))
      return new C(n, i);
  }
  getStarts() {
    return new Me(this.seq1Range.start, this.seq2Range.start);
  }
  getEndExclusives() {
    return new Me(this.seq1Range.endExclusive, this.seq2Range.endExclusive);
  }
}
const ze = class ze {
  constructor(t, n) {
    this.offset1 = t, this.offset2 = n;
  }
  toString() {
    return `${this.offset1} <-> ${this.offset2}`;
  }
  delta(t) {
    return t === 0 ? this : new ze(this.offset1 + t, this.offset2 + t);
  }
  equals(t) {
    return this.offset1 === t.offset1 && this.offset2 === t.offset2;
  }
};
ze.zero = new ze(0, 0), ze.max = new ze(Number.MAX_SAFE_INTEGER, Number.MAX_SAFE_INTEGER);
let Me = ze;
const un = class un {
  isValid() {
    return !0;
  }
};
un.instance = new un();
let kt = un;
class G1 {
  constructor(t) {
    if (this.timeout = t, this.startTime = Date.now(), this.valid = !0, t <= 0)
      throw new me("timeout must be positive");
  }
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
class gn {
  constructor(t, n) {
    this.width = t, this.height = n, this.array = [], this.array = new Array(t * n);
  }
  get(t, n) {
    return this.array[t + n * this.width];
  }
  set(t, n, i) {
    this.array[t + n * this.width] = i;
  }
}
function Hn(e) {
  return e === d.Space || e === d.Tab;
}
const Rt = class Rt {
  static getKey(t) {
    let n = this.chrKeys.get(t);
    return n === void 0 && (n = this.chrKeys.size, this.chrKeys.set(t, n)), n;
  }
  constructor(t, n, i) {
    this.range = t, this.lines = n, this.source = i, this.histogram = [];
    let s = 0;
    for (let r = t.startLineNumber - 1; r < t.endLineNumberExclusive - 1; r++) {
      const a = n[r];
      for (let o = 0; o < a.length; o++) {
        s++;
        const c = a[o], h = Rt.getKey(c);
        this.histogram[h] = (this.histogram[h] || 0) + 1;
      }
      s++;
      const u = Rt.getKey(`
`);
      this.histogram[u] = (this.histogram[u] || 0) + 1;
    }
    this.totalCount = s;
  }
  computeSimilarity(t) {
    let n = 0;
    const i = Math.max(this.histogram.length, t.histogram.length);
    for (let s = 0; s < i; s++)
      n += Math.abs((this.histogram[s] ?? 0) - (t.histogram[s] ?? 0));
    return 1 - n / (this.totalCount + t.totalCount);
  }
};
Rt.chrKeys = /* @__PURE__ */ new Map();
let Ct = Rt;
class $1 {
  compute(t, n, i = kt.instance, s) {
    if (t.length === 0 || n.length === 0)
      return ke.trivial(t, n);
    const r = new gn(t.length, n.length), a = new gn(t.length, n.length), u = new gn(t.length, n.length);
    for (let L = 0; L < t.length; L++)
      for (let N = 0; N < n.length; N++) {
        if (!i.isValid())
          return ke.trivialTimedOut(t, n);
        const A = L === 0 ? 0 : r.get(L - 1, N), x = N === 0 ? 0 : r.get(L, N - 1);
        let R;
        t.getElement(L) === n.getElement(N) ? (L === 0 || N === 0 ? R = 0 : R = r.get(L - 1, N - 1), L > 0 && N > 0 && a.get(L - 1, N - 1) === 3 && (R += u.get(L - 1, N - 1)), R += s ? s(L, N) : 1) : R = -1;
        const U = Math.max(A, x, R);
        if (U === R) {
          const p = L > 0 && N > 0 ? u.get(L - 1, N - 1) : 0;
          u.set(L, N, p + 1), a.set(L, N, 3);
        } else U === A ? (u.set(L, N, 0), a.set(L, N, 1)) : U === x && (u.set(L, N, 0), a.set(L, N, 2));
        r.set(L, N, U);
      }
    const o = [];
    let c = t.length, h = n.length;
    function f(L, N) {
      (L + 1 !== c || N + 1 !== h) && o.push(new C(new W(L + 1, c), new W(N + 1, h))), c = L, h = N;
    }
    let g = t.length - 1, b = n.length - 1;
    for (; g >= 0 && b >= 0; )
      a.get(g, b) === 3 ? (f(g, b), g--, b--) : a.get(g, b) === 1 ? g-- : b--;
    return f(-1, -1), o.reverse(), new ke(o, !1);
  }
}
class Br {
  compute(t, n, i = kt.instance) {
    if (t.length === 0 || n.length === 0)
      return ke.trivial(t, n);
    const s = t, r = n;
    function a(N, A) {
      for (; N < s.length && A < r.length && s.getElement(N) === r.getElement(A); )
        N++, A++;
      return N;
    }
    let u = 0;
    const o = new z1();
    o.set(0, a(0, 0));
    const c = new j1();
    c.set(0, o.get(0) === 0 ? null : new Cs(null, 0, 0, o.get(0)));
    let h = 0;
    e: for (; ; ) {
      if (u++, !i.isValid())
        return ke.trivialTimedOut(s, r);
      const N = -Math.min(u, r.length + u % 2), A = Math.min(u, s.length + u % 2);
      for (h = N; h <= A; h += 2) {
        const x = h === A ? -1 : o.get(h + 1), R = h === N ? -1 : o.get(h - 1) + 1, U = Math.min(Math.max(x, R), s.length), p = U - h;
        if (U > s.length || p > r.length)
          continue;
        const w = a(U, p);
        o.set(h, w);
        const D = U === x ? c.get(h + 1) : c.get(h - 1);
        if (c.set(h, w !== U ? new Cs(D, U, p, w - U) : D), o.get(h) === s.length && o.get(h) - h === r.length)
          break e;
      }
    }
    let f = c.get(h);
    const g = [];
    let b = s.length, L = r.length;
    for (; ; ) {
      const N = f ? f.x + f.length : 0, A = f ? f.y + f.length : 0;
      if ((N !== b || A !== L) && g.push(new C(new W(N, b), new W(A, L))), !f)
        break;
      b = f.x, L = f.y, f = f.prev;
    }
    return g.reverse(), new ke(g, !1);
  }
}
class Cs {
  constructor(t, n, i, s) {
    this.prev = t, this.x = n, this.y = i, this.length = s;
  }
}
class z1 {
  constructor() {
    this.positiveArr = new Int32Array(10), this.negativeArr = new Int32Array(10);
  }
  get(t) {
    return t < 0 ? (t = -t - 1, this.negativeArr[t]) : this.positiveArr[t];
  }
  set(t, n) {
    if (t < 0) {
      if (t = -t - 1, t >= this.negativeArr.length) {
        const i = this.negativeArr;
        this.negativeArr = new Int32Array(i.length * 2), this.negativeArr.set(i);
      }
      this.negativeArr[t] = n;
    } else {
      if (t >= this.positiveArr.length) {
        const i = this.positiveArr;
        this.positiveArr = new Int32Array(i.length * 2), this.positiveArr.set(i);
      }
      this.positiveArr[t] = n;
    }
  }
}
class j1 {
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
class en {
  constructor(t, n, i) {
    this.lines = t, this.range = n, this.considerWhitespaceChanges = i, this.elements = [], this.firstElementOffsetByLineIdx = [], this.lineStartOffsets = [], this.trimmedWsLengthsByLineIdx = [], this.firstElementOffsetByLineIdx.push(0);
    for (let s = this.range.startLineNumber; s <= this.range.endLineNumber; s++) {
      let r = t[s - 1], a = 0;
      s === this.range.startLineNumber && this.range.startColumn > 1 && (a = this.range.startColumn - 1, r = r.substring(a)), this.lineStartOffsets.push(a);
      let u = 0;
      if (!i) {
        const c = r.trimStart();
        u = r.length - c.length, r = c.trimEnd();
      }
      this.trimmedWsLengthsByLineIdx.push(u);
      const o = s === this.range.endLineNumber ? Math.min(this.range.endColumn - 1 - a - u, r.length) : r.length;
      for (let c = 0; c < o; c++)
        this.elements.push(r.charCodeAt(c));
      s < this.range.endLineNumber && (this.elements.push(10), this.firstElementOffsetByLineIdx.push(this.elements.length));
    }
  }
  toString() {
    return `Slice: "${this.text}"`;
  }
  get text() {
    return this.getText(new W(0, this.length));
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
    const n = tr(t > 0 ? this.elements[t - 1] : -1), i = tr(t < this.elements.length ? this.elements[t] : -1);
    if (n === K.LineBreakCR && i === K.LineBreakLF)
      return 0;
    if (n === K.LineBreakLF)
      return 150;
    let s = 0;
    return n !== i && (s += 10, n === K.WordLower && i === K.WordUpper && (s += 1)), s += er(n), s += er(i), s;
  }
  translateOffset(t, n = "right") {
    const i = vt(this.firstElementOffsetByLineIdx, (r) => r <= t), s = t - this.firstElementOffsetByLineIdx[i];
    return new X(
      this.range.startLineNumber + i,
      1 + this.lineStartOffsets[i] + s + (s === 0 && n === "left" ? 0 : this.trimmedWsLengthsByLineIdx[i])
    );
  }
  translateRange(t) {
    const n = this.translateOffset(t.start, "right"), i = this.translateOffset(t.endExclusive, "left");
    return i.isBefore(n) ? S.fromPositions(i, i) : S.fromPositions(n, i);
  }
  findWordContaining(t) {
    if (t < 0 || t >= this.elements.length || !_n(this.elements[t]))
      return;
    let n = t;
    for (; n > 0 && _n(this.elements[n - 1]); )
      n--;
    let i = t;
    for (; i < this.elements.length && _n(this.elements[i]); )
      i++;
    return new W(n, i);
  }
  countLinesIn(t) {
    return this.translateOffset(t.endExclusive).lineNumber - this.translateOffset(t.start).lineNumber;
  }
  isStronglyEqual(t, n) {
    return this.elements[t] === this.elements[n];
  }
  extendToFullLines(t) {
    const n = ft(this.firstElementOffsetByLineIdx, (s) => s <= t.start) ?? 0, i = jr(this.firstElementOffsetByLineIdx, (s) => t.endExclusive <= s) ?? this.elements.length;
    return new W(n, i);
  }
}
function _n(e) {
  return e >= d.a && e <= d.z || e >= d.A && e <= d.Z || e >= d.Digit0 && e <= d.Digit9;
}
var K;
(function(e) {
  e[e.WordLower = 0] = "WordLower", e[e.WordUpper = 1] = "WordUpper", e[e.WordNumber = 2] = "WordNumber", e[e.End = 3] = "End", e[e.Other = 4] = "Other", e[e.Separator = 5] = "Separator", e[e.Space = 6] = "Space", e[e.LineBreakCR = 7] = "LineBreakCR", e[e.LineBreakLF = 8] = "LineBreakLF";
})(K || (K = {}));
const X1 = {
  [K.WordLower]: 0,
  [K.WordUpper]: 0,
  [K.WordNumber]: 0,
  [K.End]: 10,
  [K.Other]: 2,
  [K.Separator]: 30,
  [K.Space]: 3,
  [K.LineBreakCR]: 10,
  [K.LineBreakLF]: 10
};
function er(e) {
  return X1[e];
}
function tr(e) {
  return e === d.LineFeed ? K.LineBreakLF : e === d.CarriageReturn ? K.LineBreakCR : Hn(e) ? K.Space : e >= d.a && e <= d.z ? K.WordLower : e >= d.A && e <= d.Z ? K.WordUpper : e >= d.Digit0 && e <= d.Digit9 ? K.WordNumber : e === -1 ? K.End : e === d.Comma || e === d.Semicolon ? K.Separator : K.Other;
}
function Y1(e, t, n, i, s, r) {
  let { moves: a, excludedChanges: u } = Z1(e, t, n, r);
  if (!r.isValid())
    return [];
  const o = e.filter((h) => !u.has(h)), c = J1(o, i, s, t, n, r);
  return Jr(a, c), a = K1(a), a = a.filter((h) => {
    const f = h.original.toOffsetRange().slice(t).map((b) => b.trim());
    return f.join(`
`).length >= 15 && Q1(f, (b) => b.length >= 2) >= 2;
  }), a = C1(e, a), a;
}
function Q1(e, t) {
  let n = 0;
  for (const i of e)
    t(i) && n++;
  return n;
}
function Z1(e, t, n, i) {
  const s = [], r = e.filter((o) => o.modified.isEmpty && o.original.length >= 3).map((o) => new Ct(o.original, t, o)), a = new Set(e.filter((o) => o.original.isEmpty && o.modified.length >= 3).map((o) => new Ct(o.modified, n, o))), u = /* @__PURE__ */ new Set();
  for (const o of r) {
    let c = -1, h;
    for (const f of a) {
      const g = o.computeSimilarity(f);
      g > c && (c = g, h = f);
    }
    if (c > 0.9 && h && (a.delete(h), s.push(new Ne(o.range, h.range)), u.add(o.source), u.add(h.source)), !i.isValid())
      return { moves: s, excludedChanges: u };
  }
  return { moves: s, excludedChanges: u };
}
function J1(e, t, n, i, s, r) {
  const a = [], u = new br();
  for (const g of e)
    for (let b = g.original.startLineNumber; b < g.original.endLineNumberExclusive - 2; b++) {
      const L = `${t[b - 1]}:${t[b + 1 - 1]}:${t[b + 2 - 1]}`;
      u.add(L, { range: new V(b, b + 3) });
    }
  const o = [];
  e.sort(Nt((g) => g.modified.startLineNumber, pt));
  for (const g of e) {
    let b = [];
    for (let L = g.modified.startLineNumber; L < g.modified.endLineNumberExclusive - 2; L++) {
      const N = `${n[L - 1]}:${n[L + 1 - 1]}:${n[L + 2 - 1]}`, A = new V(L, L + 3), x = [];
      u.forEach(N, ({ range: R }) => {
        for (const p of b)
          if (p.originalLineRange.endLineNumberExclusive + 1 === R.endLineNumberExclusive && p.modifiedLineRange.endLineNumberExclusive + 1 === A.endLineNumberExclusive) {
            p.originalLineRange = new V(
              p.originalLineRange.startLineNumber,
              R.endLineNumberExclusive
            ), p.modifiedLineRange = new V(
              p.modifiedLineRange.startLineNumber,
              A.endLineNumberExclusive
            ), x.push(p);
            return;
          }
        const U = {
          modifiedLineRange: A,
          originalLineRange: R
        };
        o.push(U), x.push(U);
      }), b = x;
    }
    if (!r.isValid())
      return [];
  }
  o.sort(Kr(Nt((g) => g.modifiedLineRange.length, pt)));
  const c = new Ue(), h = new Ue();
  for (const g of o) {
    const b = g.modifiedLineRange.startLineNumber - g.originalLineRange.startLineNumber, L = c.subtractFrom(g.modifiedLineRange), N = h.subtractFrom(g.originalLineRange).getWithDelta(b), A = L.getIntersection(N);
    for (const x of A.ranges) {
      if (x.length < 3)
        continue;
      const R = x, U = x.delta(-b);
      a.push(new Ne(U, R)), c.addRange(R), h.addRange(U);
    }
  }
  a.sort(Nt((g) => g.original.startLineNumber, pt));
  const f = new Gt(e);
  for (let g = 0; g < a.length; g++) {
    const b = a[g], L = f.findLastMonotonous((D) => D.original.startLineNumber <= b.original.startLineNumber), N = ft(e, (D) => D.modified.startLineNumber <= b.modified.startLineNumber), A = Math.max(b.original.startLineNumber - L.original.startLineNumber, b.modified.startLineNumber - N.modified.startLineNumber), x = f.findLastMonotonous((D) => D.original.startLineNumber < b.original.endLineNumberExclusive), R = ft(e, (D) => D.modified.startLineNumber < b.modified.endLineNumberExclusive), U = Math.max(x.original.endLineNumberExclusive - b.original.endLineNumberExclusive, R.modified.endLineNumberExclusive - b.modified.endLineNumberExclusive);
    let p;
    for (p = 0; p < A; p++) {
      const D = b.original.startLineNumber - p - 1, T = b.modified.startLineNumber - p - 1;
      if (D > i.length || T > s.length || c.contains(T) || h.contains(D) || !nr(i[D - 1], s[T - 1], r))
        break;
    }
    p > 0 && (h.addRange(new V(b.original.startLineNumber - p, b.original.startLineNumber)), c.addRange(new V(b.modified.startLineNumber - p, b.modified.startLineNumber)));
    let w;
    for (w = 0; w < U; w++) {
      const D = b.original.endLineNumberExclusive + w, T = b.modified.endLineNumberExclusive + w;
      if (D > i.length || T > s.length || c.contains(T) || h.contains(D) || !nr(i[D - 1], s[T - 1], r))
        break;
    }
    w > 0 && (h.addRange(new V(
      b.original.endLineNumberExclusive,
      b.original.endLineNumberExclusive + w
    )), c.addRange(new V(
      b.modified.endLineNumberExclusive,
      b.modified.endLineNumberExclusive + w
    ))), (p > 0 || w > 0) && (a[g] = new Ne(new V(
      b.original.startLineNumber - p,
      b.original.endLineNumberExclusive + w
    ), new V(
      b.modified.startLineNumber - p,
      b.modified.endLineNumberExclusive + w
    )));
  }
  return a;
}
function nr(e, t, n) {
  if (e.trim() === t.trim())
    return !0;
  if (e.length > 300 && t.length > 300)
    return !1;
  const s = new Br().compute(new en([e], new S(1, 1, 1, e.length), !1), new en([t], new S(1, 1, 1, t.length), !1), n);
  let r = 0;
  const a = C.invert(s.diffs, e.length);
  for (const h of a)
    h.seq1Range.forEach((f) => {
      Hn(e.charCodeAt(f)) || r++;
    });
  function u(h) {
    let f = 0;
    for (let g = 0; g < e.length; g++)
      Hn(h.charCodeAt(g)) || f++;
    return f;
  }
  const o = u(e.length > t.length ? e : t);
  return r / o > 0.6 && o > 10;
}
function K1(e) {
  if (e.length === 0)
    return e;
  e.sort(Nt((n) => n.original.startLineNumber, pt));
  const t = [e[0]];
  for (let n = 1; n < e.length; n++) {
    const i = t[t.length - 1], s = e[n], r = s.original.startLineNumber - i.original.endLineNumberExclusive, a = s.modified.startLineNumber - i.modified.endLineNumberExclusive;
    if (r >= 0 && a >= 0 && r + a <= 2) {
      t[t.length - 1] = i.join(s);
      continue;
    }
    t.push(s);
  }
  return t;
}
function C1(e, t) {
  const n = new Gt(e);
  return t = t.filter((i) => {
    const s = n.findLastMonotonous((u) => u.original.startLineNumber < i.original.endLineNumberExclusive) || new Ne(new V(1, 1), new V(1, 1)), r = ft(e, (u) => u.modified.startLineNumber < i.modified.endLineNumberExclusive);
    return s !== r;
  }), t;
}
function ir(e, t, n) {
  let i = n;
  return i = sr(e, t, i), i = sr(e, t, i), i = ea(e, t, i), i;
}
function sr(e, t, n) {
  if (n.length === 0)
    return n;
  const i = [];
  i.push(n[0]);
  for (let r = 1; r < n.length; r++) {
    const a = i[i.length - 1];
    let u = n[r];
    if (u.seq1Range.isEmpty || u.seq2Range.isEmpty) {
      const o = u.seq1Range.start - a.seq1Range.endExclusive;
      let c;
      for (c = 1; c <= o && !(e.getElement(u.seq1Range.start - c) !== e.getElement(u.seq1Range.endExclusive - c) || t.getElement(u.seq2Range.start - c) !== t.getElement(u.seq2Range.endExclusive - c)); c++)
        ;
      if (c--, c === o) {
        i[i.length - 1] = new C(new W(a.seq1Range.start, u.seq1Range.endExclusive - o), new W(a.seq2Range.start, u.seq2Range.endExclusive - o));
        continue;
      }
      u = u.delta(-c);
    }
    i.push(u);
  }
  const s = [];
  for (let r = 0; r < i.length - 1; r++) {
    const a = i[r + 1];
    let u = i[r];
    if (u.seq1Range.isEmpty || u.seq2Range.isEmpty) {
      const o = a.seq1Range.start - u.seq1Range.endExclusive;
      let c;
      for (c = 0; c < o && !(!e.isStronglyEqual(u.seq1Range.start + c, u.seq1Range.endExclusive + c) || !t.isStronglyEqual(u.seq2Range.start + c, u.seq2Range.endExclusive + c)); c++)
        ;
      if (c === o) {
        i[r + 1] = new C(new W(u.seq1Range.start + o, a.seq1Range.endExclusive), new W(u.seq2Range.start + o, a.seq2Range.endExclusive));
        continue;
      }
      c > 0 && (u = u.delta(c));
    }
    s.push(u);
  }
  return i.length > 0 && s.push(i[i.length - 1]), s;
}
function ea(e, t, n) {
  if (!e.getBoundaryScore || !t.getBoundaryScore)
    return n;
  for (let i = 0; i < n.length; i++) {
    const s = i > 0 ? n[i - 1] : void 0, r = n[i], a = i + 1 < n.length ? n[i + 1] : void 0, u = new W(
      s ? s.seq1Range.endExclusive + 1 : 0,
      a ? a.seq1Range.start - 1 : e.length
    ), o = new W(
      s ? s.seq2Range.endExclusive + 1 : 0,
      a ? a.seq2Range.start - 1 : t.length
    );
    r.seq1Range.isEmpty ? n[i] = rr(r, e, t, u, o) : r.seq2Range.isEmpty && (n[i] = rr(r.swap(), t, e, o, u).swap());
  }
  return n;
}
function rr(e, t, n, i, s) {
  let a = 1;
  for (; e.seq1Range.start - a >= i.start && e.seq2Range.start - a >= s.start && n.isStronglyEqual(e.seq2Range.start - a, e.seq2Range.endExclusive - a) && a < 100; )
    a++;
  a--;
  let u = 0;
  for (; e.seq1Range.start + u < i.endExclusive && e.seq2Range.endExclusive + u < s.endExclusive && n.isStronglyEqual(e.seq2Range.start + u, e.seq2Range.endExclusive + u) && u < 100; )
    u++;
  if (a === 0 && u === 0)
    return e;
  let o = 0, c = -1;
  for (let h = -a; h <= u; h++) {
    const f = e.seq2Range.start + h, g = e.seq2Range.endExclusive + h, b = e.seq1Range.start + h, L = t.getBoundaryScore(b) + n.getBoundaryScore(f) + n.getBoundaryScore(g);
    L > c && (c = L, o = h);
  }
  return e.delta(o);
}
function ta(e, t, n) {
  const i = [];
  for (const s of n) {
    const r = i[i.length - 1];
    if (!r) {
      i.push(s);
      continue;
    }
    s.seq1Range.start - r.seq1Range.endExclusive <= 2 || s.seq2Range.start - r.seq2Range.endExclusive <= 2 ? i[i.length - 1] = new C(r.seq1Range.join(s.seq1Range), r.seq2Range.join(s.seq2Range)) : i.push(s);
  }
  return i;
}
function na(e, t, n) {
  const i = C.invert(n, e.length), s = [];
  let r = new Me(0, 0);
  function a(o, c) {
    if (o.offset1 < r.offset1 || o.offset2 < r.offset2)
      return;
    const h = e.findWordContaining(o.offset1), f = t.findWordContaining(o.offset2);
    if (!h || !f)
      return;
    let g = new C(h, f);
    const b = g.intersect(c);
    let L = b.seq1Range.length, N = b.seq2Range.length;
    for (; i.length > 0; ) {
      const A = i[0];
      if (!(A.seq1Range.intersects(g.seq1Range) || A.seq2Range.intersects(g.seq2Range)))
        break;
      const R = e.findWordContaining(A.seq1Range.start), U = t.findWordContaining(A.seq2Range.start), p = new C(R, U), w = p.intersect(A);
      if (L += w.seq1Range.length, N += w.seq2Range.length, g = g.join(p), g.seq1Range.endExclusive >= A.seq1Range.endExclusive)
        i.shift();
      else
        break;
    }
    L + N < (g.seq1Range.length + g.seq2Range.length) * 2 / 3 && s.push(g), r = g.getEndExclusives();
  }
  for (; i.length > 0; ) {
    const o = i.shift();
    o.seq1Range.isEmpty || (a(o.getStarts(), o), a(o.getEndExclusives().delta(-1), o));
  }
  return ia(n, s);
}
function ia(e, t) {
  const n = [];
  for (; e.length > 0 || t.length > 0; ) {
    const i = e[0], s = t[0];
    let r;
    i && (!s || i.seq1Range.start < s.seq1Range.start) ? r = e.shift() : r = t.shift(), n.length > 0 && n[n.length - 1].seq1Range.endExclusive >= r.seq1Range.start ? n[n.length - 1] = n[n.length - 1].join(r) : n.push(r);
  }
  return n;
}
function sa(e, t, n) {
  let i = n;
  if (i.length === 0)
    return i;
  let s = 0, r;
  do {
    r = !1;
    const a = [
      i[0]
    ];
    for (let u = 1; u < i.length; u++) {
      let h = function(g, b) {
        const L = new W(c.seq1Range.endExclusive, o.seq1Range.start);
        return e.getText(L).replace(/\s/g, "").length <= 4 && (g.seq1Range.length + g.seq2Range.length > 5 || b.seq1Range.length + b.seq2Range.length > 5);
      };
      const o = i[u], c = a[a.length - 1];
      h(c, o) ? (r = !0, a[a.length - 1] = a[a.length - 1].join(o)) : a.push(o);
    }
    i = a;
  } while (s++ < 10 && r);
  return i;
}
function ra(e, t, n) {
  let i = n;
  if (i.length === 0)
    return i;
  let s = 0, r;
  do {
    r = !1;
    const u = [
      i[0]
    ];
    for (let o = 1; o < i.length; o++) {
      let f = function(b, L) {
        const N = new W(h.seq1Range.endExclusive, c.seq1Range.start);
        if (e.countLinesIn(N) > 5 || N.length > 500)
          return !1;
        const x = e.getText(N).trim();
        if (x.length > 20 || x.split(/\r\n|\r|\n/).length > 1)
          return !1;
        const R = e.countLinesIn(b.seq1Range), U = b.seq1Range.length, p = t.countLinesIn(b.seq2Range), w = b.seq2Range.length, D = e.countLinesIn(L.seq1Range), T = L.seq1Range.length, I = t.countLinesIn(L.seq2Range), $ = L.seq2Range.length, ne = 2 * 40 + 50;
        function z(E) {
          return Math.min(E, ne);
        }
        return Math.pow(Math.pow(z(R * 40 + U), 1.5) + Math.pow(z(p * 40 + w), 1.5), 1.5) + Math.pow(Math.pow(z(D * 40 + T), 1.5) + Math.pow(z(I * 40 + $), 1.5), 1.5) > (ne ** 1.5) ** 1.5 * 1.3;
      };
      const c = i[o], h = u[u.length - 1];
      f(h, c) ? (r = !0, u[u.length - 1] = u[u.length - 1].join(c)) : u.push(c);
    }
    i = u;
  } while (s++ < 10 && r);
  const a = [];
  return Zr(i, (u, o, c) => {
    let h = o;
    function f(x) {
      return x.length > 0 && x.trim().length <= 3 && o.seq1Range.length + o.seq2Range.length > 100;
    }
    const g = e.extendToFullLines(o.seq1Range), b = e.getText(new W(g.start, o.seq1Range.start));
    f(b) && (h = h.deltaStart(-b.length));
    const L = e.getText(new W(o.seq1Range.endExclusive, g.endExclusive));
    f(L) && (h = h.deltaEnd(L.length));
    const N = C.fromOffsetPairs(u ? u.getEndExclusives() : Me.zero, c ? c.getStarts() : Me.max), A = h.intersect(N);
    a.length > 0 && A.getStarts().equals(a[a.length - 1].getEndExclusives()) ? a[a.length - 1] = a[a.length - 1].join(A) : a.push(A);
  }), a;
}
class lr {
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
    const n = t === 0 ? 0 : ar(this.lines[t - 1]), i = t === this.lines.length ? 0 : ar(this.lines[t]);
    return 1e3 - (n + i);
  }
  getText(t) {
    return this.lines.slice(t.start, t.endExclusive).join(`
`);
  }
  isStronglyEqual(t, n) {
    return this.lines[t] === this.lines[n];
  }
}
function ar(e) {
  let t = 0;
  for (; t < e.length && (e.charCodeAt(t) === d.Space || e.charCodeAt(t) === d.Tab); )
    t++;
  return t;
}
class la {
  constructor() {
    this.dynamicProgrammingDiffing = new $1(), this.myersDiffingAlgorithm = new Br();
  }
  computeDiff(t, n, i) {
    if (t.length <= 1 && Xr(t, n, (w, D) => w === D))
      return new Wt([], [], !1);
    if (t.length === 1 && t[0].length === 0 || n.length === 1 && n[0].length === 0)
      return new Wt([
        new Fe(new V(1, t.length + 1), new V(1, n.length + 1), [
          new Re(new S(
            1,
            1,
            t.length,
            t[t.length - 1].length + 1
          ), new S(
            1,
            1,
            n.length,
            n[n.length - 1].length + 1
          ))
        ])
      ], [], !1);
    const s = i.maxComputationTimeMs === 0 ? kt.instance : new G1(i.maxComputationTimeMs), r = !i.ignoreTrimWhitespace, a = /* @__PURE__ */ new Map();
    function u(w) {
      let D = a.get(w);
      return D === void 0 && (D = a.size, a.set(w, D)), D;
    }
    const o = t.map((w) => u(w.trim())), c = n.map((w) => u(w.trim())), h = new lr(o, t), f = new lr(c, n), g = h.length + f.length < 1700 ? this.dynamicProgrammingDiffing.compute(h, f, s, (w, D) => t[w] === n[D] ? n[D].length === 0 ? 0.1 : 1 + Math.log(1 + n[D].length) : 0.99) : this.myersDiffingAlgorithm.compute(h, f, s);
    let b = g.diffs, L = g.hitTimeout;
    b = ir(h, f, b), b = sa(h, f, b);
    const N = [], A = (w) => {
      if (r)
        for (let D = 0; D < w; D++) {
          const T = x + D, I = R + D;
          if (t[T] !== n[I]) {
            const $ = this.refineDiff(t, n, new C(new W(T, T + 1), new W(I, I + 1)), s, r);
            for (const ne of $.mappings)
              N.push(ne);
            $.hitTimeout && (L = !0);
          }
        }
    };
    let x = 0, R = 0;
    for (const w of b) {
      Kt(() => w.seq1Range.start - x === w.seq2Range.start - R);
      const D = w.seq1Range.start - x;
      A(D), x = w.seq1Range.endExclusive, R = w.seq2Range.endExclusive;
      const T = this.refineDiff(t, n, w, s, r);
      T.hitTimeout && (L = !0);
      for (const I of T.mappings)
        N.push(I);
    }
    A(t.length - x);
    const U = ur(N, t, n);
    let p = [];
    return i.computeMoves && (p = this.computeMoves(U, t, n, o, c, s, r)), Kt(() => {
      function w(T, I) {
        if (T.lineNumber < 1 || T.lineNumber > I.length)
          return !1;
        const $ = I[T.lineNumber - 1];
        return !(T.column < 1 || T.column > $.length + 1);
      }
      function D(T, I) {
        return !(T.startLineNumber < 1 || T.startLineNumber > I.length + 1 || T.endLineNumberExclusive < 1 || T.endLineNumberExclusive > I.length + 1);
      }
      for (const T of U) {
        if (!T.innerChanges)
          return !1;
        for (const I of T.innerChanges)
          if (!(w(I.modifiedRange.getStartPosition(), n) && w(I.modifiedRange.getEndPosition(), n) && w(I.originalRange.getStartPosition(), t) && w(I.originalRange.getEndPosition(), t)))
            return !1;
        if (!D(T.modified, n) || !D(T.original, t))
          return !1;
      }
      return !0;
    }), new Wt(U, p, L);
  }
  computeMoves(t, n, i, s, r, a, u) {
    return Y1(t, n, i, s, r, a).map((h) => {
      const f = this.refineDiff(n, i, new C(h.original.toOffsetRange(), h.modified.toOffsetRange()), a, u), g = ur(f.mappings, n, i, !0);
      return new Xn(h, g);
    });
  }
  refineDiff(t, n, i, s, r) {
    const u = ua(i).toRangeMapping2(t, n), o = new en(t, u.originalRange, r), c = new en(n, u.modifiedRange, r), h = o.length + c.length < 500 ? this.dynamicProgrammingDiffing.compute(o, c, s) : this.myersDiffingAlgorithm.compute(o, c, s);
    let f = h.diffs;
    return f = ir(o, c, f), f = na(o, c, f), f = ta(o, c, f), f = ra(o, c, f), {
      mappings: f.map((b) => new Re(o.translateRange(b.seq1Range), c.translateRange(b.seq2Range))),
      hitTimeout: h.hitTimeout
    };
  }
}
function ur(e, t, n, i = !1) {
  const s = [];
  for (const r of Yr(e.map((a) => aa(a, t, n)), (a, u) => a.original.overlapOrTouch(u.original) || a.modified.overlapOrTouch(u.modified))) {
    const a = r[0], u = r[r.length - 1];
    s.push(new Fe(
      a.original.join(u.original),
      a.modified.join(u.modified),
      r.map((o) => o.innerChanges[0])
    ));
  }
  return Kt(() => !i && s.length > 0 && (s[0].modified.startLineNumber !== s[0].original.startLineNumber || n.length - s[s.length - 1].modified.endLineNumberExclusive !== t.length - s[s.length - 1].original.endLineNumberExclusive) ? !1 : kr(s, (r, a) => a.original.startLineNumber - r.original.endLineNumberExclusive === a.modified.startLineNumber - r.modified.endLineNumberExclusive && r.original.endLineNumberExclusive < a.original.startLineNumber && r.modified.endLineNumberExclusive < a.modified.startLineNumber)), s;
}
function aa(e, t, n) {
  let i = 0, s = 0;
  e.modifiedRange.endColumn === 1 && e.originalRange.endColumn === 1 && e.originalRange.startLineNumber + i <= e.originalRange.endLineNumber && e.modifiedRange.startLineNumber + i <= e.modifiedRange.endLineNumber && (s = -1), e.modifiedRange.startColumn - 1 >= n[e.modifiedRange.startLineNumber - 1].length && e.originalRange.startColumn - 1 >= t[e.originalRange.startLineNumber - 1].length && e.originalRange.startLineNumber <= e.originalRange.endLineNumber + s && e.modifiedRange.startLineNumber <= e.modifiedRange.endLineNumber + s && (i = 1);
  const r = new V(
    e.originalRange.startLineNumber + i,
    e.originalRange.endLineNumber + 1 + s
  ), a = new V(
    e.modifiedRange.startLineNumber + i,
    e.modifiedRange.endLineNumber + 1 + s
  );
  return new Fe(r, a, [e]);
}
function ua(e) {
  return new Ne(new V(e.seq1Range.start + 1, e.seq1Range.endExclusive + 1), new V(e.seq2Range.start + 1, e.seq2Range.endExclusive + 1));
}
const bn = {
  getLegacy: () => new H1(),
  getDefault: () => new la()
};
function qe(e, t) {
  const n = Math.pow(10, t);
  return Math.round(e * n) / n;
}
class te {
  constructor(t, n, i, s = 1) {
    this._rgbaBrand = void 0, this.r = Math.min(255, Math.max(0, t)) | 0, this.g = Math.min(255, Math.max(0, n)) | 0, this.b = Math.min(255, Math.max(0, i)) | 0, this.a = qe(Math.max(Math.min(1, s), 0), 3);
  }
  static equals(t, n) {
    return t.r === n.r && t.g === n.g && t.b === n.b && t.a === n.a;
  }
}
class Le {
  constructor(t, n, i, s) {
    this._hslaBrand = void 0, this.h = Math.max(Math.min(360, t), 0) | 0, this.s = qe(Math.max(Math.min(1, n), 0), 3), this.l = qe(Math.max(Math.min(1, i), 0), 3), this.a = qe(Math.max(Math.min(1, s), 0), 3);
  }
  static equals(t, n) {
    return t.h === n.h && t.s === n.s && t.l === n.l && t.a === n.a;
  }
  static fromRGBA(t) {
    const n = t.r / 255, i = t.g / 255, s = t.b / 255, r = t.a, a = Math.max(n, i, s), u = Math.min(n, i, s);
    let o = 0, c = 0;
    const h = (u + a) / 2, f = a - u;
    if (f > 0) {
      switch (c = Math.min(h <= 0.5 ? f / (2 * h) : f / (2 - 2 * h), 1), a) {
        case n:
          o = (i - s) / f + (i < s ? 6 : 0);
          break;
        case i:
          o = (s - n) / f + 2;
          break;
        case s:
          o = (n - i) / f + 4;
          break;
      }
      o *= 60, o = Math.round(o);
    }
    return new Le(o, c, h, r);
  }
  static _hue2rgb(t, n, i) {
    return i < 0 && (i += 1), i > 1 && (i -= 1), i < 1 / 6 ? t + (n - t) * 6 * i : i < 1 / 2 ? n : i < 2 / 3 ? t + (n - t) * (2 / 3 - i) * 6 : t;
  }
  static toRGBA(t) {
    const n = t.h / 360, { s: i, l: s, a: r } = t;
    let a, u, o;
    if (i === 0)
      a = u = o = s;
    else {
      const c = s < 0.5 ? s * (1 + i) : s + i - s * i, h = 2 * s - c;
      a = Le._hue2rgb(h, c, n + 1 / 3), u = Le._hue2rgb(h, c, n), o = Le._hue2rgb(h, c, n - 1 / 3);
    }
    return new te(Math.round(a * 255), Math.round(u * 255), Math.round(o * 255), r);
  }
}
class it {
  constructor(t, n, i, s) {
    this._hsvaBrand = void 0, this.h = Math.max(Math.min(360, t), 0) | 0, this.s = qe(Math.max(Math.min(1, n), 0), 3), this.v = qe(Math.max(Math.min(1, i), 0), 3), this.a = qe(Math.max(Math.min(1, s), 0), 3);
  }
  static equals(t, n) {
    return t.h === n.h && t.s === n.s && t.v === n.v && t.a === n.a;
  }
  static fromRGBA(t) {
    const n = t.r / 255, i = t.g / 255, s = t.b / 255, r = Math.max(n, i, s), a = Math.min(n, i, s), u = r - a, o = r === 0 ? 0 : u / r;
    let c;
    return u === 0 ? c = 0 : r === n ? c = ((i - s) / u % 6 + 6) % 6 : r === i ? c = (s - n) / u + 2 : c = (n - i) / u + 4, new it(Math.round(c * 60), o, r, t.a);
  }
  static toRGBA(t) {
    const { h: n, s: i, v: s, a: r } = t, a = s * i, u = a * (1 - Math.abs(n / 60 % 2 - 1)), o = s - a;
    let [c, h, f] = [0, 0, 0];
    return n < 60 ? (c = a, h = u) : n < 120 ? (c = u, h = a) : n < 180 ? (h = a, f = u) : n < 240 ? (h = u, f = a) : n < 300 ? (c = u, f = a) : n <= 360 && (c = a, f = u), c = Math.round((c + o) * 255), h = Math.round((h + o) * 255), f = Math.round((f + o) * 255), new te(c, h, f, r);
  }
}
const G = class G {
  static fromHex(t) {
    return G.Format.CSS.parseHex(t) || G.red;
  }
  static equals(t, n) {
    return !t && !n ? !0 : !t || !n ? !1 : t.equals(n);
  }
  get hsla() {
    return this._hsla ? this._hsla : Le.fromRGBA(this.rgba);
  }
  get hsva() {
    return this._hsva ? this._hsva : it.fromRGBA(this.rgba);
  }
  constructor(t) {
    if (t)
      if (t instanceof te)
        this.rgba = t;
      else if (t instanceof Le)
        this._hsla = t, this.rgba = Le.toRGBA(t);
      else if (t instanceof it)
        this._hsva = t, this.rgba = it.toRGBA(t);
      else
        throw new Error("Invalid color ctor argument");
    else throw new Error("Color needs a value");
  }
  equals(t) {
    return !!t && te.equals(this.rgba, t.rgba) && Le.equals(this.hsla, t.hsla) && it.equals(this.hsva, t.hsva);
  }
  getRelativeLuminance() {
    const t = G._relativeLuminanceForComponent(this.rgba.r), n = G._relativeLuminanceForComponent(this.rgba.g), i = G._relativeLuminanceForComponent(this.rgba.b), s = 0.2126 * t + 0.7152 * n + 0.0722 * i;
    return qe(s, 4);
  }
  static _relativeLuminanceForComponent(t) {
    const n = t / 255;
    return n <= 0.03928 ? n / 12.92 : Math.pow((n + 0.055) / 1.055, 2.4);
  }
  getContrastRatio(t) {
    const n = this.getRelativeLuminance(), i = t.getRelativeLuminance();
    return n > i ? (n + 0.05) / (i + 0.05) : (i + 0.05) / (n + 0.05);
  }
  isDarker() {
    return (this.rgba.r * 299 + this.rgba.g * 587 + this.rgba.b * 114) / 1e3 < 128;
  }
  isLighter() {
    return (this.rgba.r * 299 + this.rgba.g * 587 + this.rgba.b * 114) / 1e3 >= 128;
  }
  isLighterThan(t) {
    const n = this.getRelativeLuminance(), i = t.getRelativeLuminance();
    return n > i;
  }
  isDarkerThan(t) {
    const n = this.getRelativeLuminance(), i = t.getRelativeLuminance();
    return n < i;
  }
  lighten(t) {
    return new G(new Le(this.hsla.h, this.hsla.s, this.hsla.l + this.hsla.l * t, this.hsla.a));
  }
  darken(t) {
    return new G(new Le(this.hsla.h, this.hsla.s, this.hsla.l - this.hsla.l * t, this.hsla.a));
  }
  transparent(t) {
    const { r: n, g: i, b: s, a: r } = this.rgba;
    return new G(new te(n, i, s, r * t));
  }
  isTransparent() {
    return this.rgba.a === 0;
  }
  isOpaque() {
    return this.rgba.a === 1;
  }
  opposite() {
    return new G(new te(255 - this.rgba.r, 255 - this.rgba.g, 255 - this.rgba.b, this.rgba.a));
  }
  blend(t) {
    const n = t.rgba, i = this.rgba.a, s = n.a, r = i + s * (1 - i);
    if (r < 1e-6)
      return G.transparent;
    const a = this.rgba.r * i / r + n.r * s * (1 - i) / r, u = this.rgba.g * i / r + n.g * s * (1 - i) / r, o = this.rgba.b * i / r + n.b * s * (1 - i) / r;
    return new G(new te(a, u, o, r));
  }
  makeOpaque(t) {
    if (this.isOpaque() || t.rgba.a !== 1)
      return this;
    const { r: n, g: i, b: s, a: r } = this.rgba;
    return new G(new te(
      t.rgba.r - r * (t.rgba.r - n),
      t.rgba.g - r * (t.rgba.g - i),
      t.rgba.b - r * (t.rgba.b - s),
      1
    ));
  }
  flatten(...t) {
    const n = t.reduceRight((i, s) => G._flatten(s, i));
    return G._flatten(this, n);
  }
  static _flatten(t, n) {
    const i = 1 - t.rgba.a;
    return new G(new te(
      i * n.rgba.r + t.rgba.a * t.rgba.r,
      i * n.rgba.g + t.rgba.a * t.rgba.g,
      i * n.rgba.b + t.rgba.a * t.rgba.b
    ));
  }
  toString() {
    return this._toString || (this._toString = G.Format.CSS.format(this)), this._toString;
  }
  static getLighterColor(t, n, i) {
    if (t.isLighterThan(n))
      return t;
    i = i || 0.5;
    const s = t.getRelativeLuminance(), r = n.getRelativeLuminance();
    return i = i * (r - s) / r, t.lighten(i);
  }
  static getDarkerColor(t, n, i) {
    if (t.isDarkerThan(n))
      return t;
    i = i || 0.5;
    const s = t.getRelativeLuminance(), r = n.getRelativeLuminance();
    return i = i * (s - r) / s, t.darken(i);
  }
};
G.white = new G(new te(255, 255, 255, 1)), G.black = new G(new te(0, 0, 0, 1)), G.red = new G(new te(255, 0, 0, 1)), G.blue = new G(new te(0, 0, 255, 1)), G.green = new G(new te(0, 255, 0, 1)), G.cyan = new G(new te(0, 255, 255, 1)), G.lightgrey = new G(new te(211, 211, 211, 1)), G.transparent = new G(new te(0, 0, 0, 0));
let gt = G;
(function(e) {
  (function(t) {
    (function(n) {
      function i(b) {
        return b.rgba.a === 1 ? `rgb(${b.rgba.r}, ${b.rgba.g}, ${b.rgba.b})` : e.Format.CSS.formatRGBA(b);
      }
      n.formatRGB = i;
      function s(b) {
        return `rgba(${b.rgba.r}, ${b.rgba.g}, ${b.rgba.b}, ${+b.rgba.a.toFixed(2)})`;
      }
      n.formatRGBA = s;
      function r(b) {
        return b.hsla.a === 1 ? `hsl(${b.hsla.h}, ${(b.hsla.s * 100).toFixed(2)}%, ${(b.hsla.l * 100).toFixed(2)}%)` : e.Format.CSS.formatHSLA(b);
      }
      n.formatHSL = r;
      function a(b) {
        return `hsla(${b.hsla.h}, ${(b.hsla.s * 100).toFixed(2)}%, ${(b.hsla.l * 100).toFixed(2)}%, ${b.hsla.a.toFixed(2)})`;
      }
      n.formatHSLA = a;
      function u(b) {
        const L = b.toString(16);
        return L.length !== 2 ? "0" + L : L;
      }
      function o(b) {
        return `#${u(b.rgba.r)}${u(b.rgba.g)}${u(b.rgba.b)}`;
      }
      n.formatHex = o;
      function c(b, L = !1) {
        return L && b.rgba.a === 1 ? e.Format.CSS.formatHex(b) : `#${u(b.rgba.r)}${u(b.rgba.g)}${u(b.rgba.b)}${u(Math.round(b.rgba.a * 255))}`;
      }
      n.formatHexA = c;
      function h(b) {
        return b.isOpaque() ? e.Format.CSS.formatHex(b) : e.Format.CSS.formatRGBA(b);
      }
      n.format = h;
      function f(b) {
        const L = b.length;
        if (L === 0 || b.charCodeAt(0) !== d.Hash)
          return null;
        if (L === 7) {
          const N = 16 * g(b.charCodeAt(1)) + g(b.charCodeAt(2)), A = 16 * g(b.charCodeAt(3)) + g(b.charCodeAt(4)), x = 16 * g(b.charCodeAt(5)) + g(b.charCodeAt(6));
          return new e(new te(N, A, x, 1));
        }
        if (L === 9) {
          const N = 16 * g(b.charCodeAt(1)) + g(b.charCodeAt(2)), A = 16 * g(b.charCodeAt(3)) + g(b.charCodeAt(4)), x = 16 * g(b.charCodeAt(5)) + g(b.charCodeAt(6)), R = 16 * g(b.charCodeAt(7)) + g(b.charCodeAt(8));
          return new e(new te(N, A, x, R / 255));
        }
        if (L === 4) {
          const N = g(b.charCodeAt(1)), A = g(b.charCodeAt(2)), x = g(b.charCodeAt(3));
          return new e(new te(16 * N + N, 16 * A + A, 16 * x + x));
        }
        if (L === 5) {
          const N = g(b.charCodeAt(1)), A = g(b.charCodeAt(2)), x = g(b.charCodeAt(3)), R = g(b.charCodeAt(4));
          return new e(new te(16 * N + N, 16 * A + A, 16 * x + x, (16 * R + R) / 255));
        }
        return null;
      }
      n.parseHex = f;
      function g(b) {
        switch (b) {
          case d.Digit0:
            return 0;
          case d.Digit1:
            return 1;
          case d.Digit2:
            return 2;
          case d.Digit3:
            return 3;
          case d.Digit4:
            return 4;
          case d.Digit5:
            return 5;
          case d.Digit6:
            return 6;
          case d.Digit7:
            return 7;
          case d.Digit8:
            return 8;
          case d.Digit9:
            return 9;
          case d.a:
            return 10;
          case d.A:
            return 10;
          case d.b:
            return 11;
          case d.B:
            return 11;
          case d.c:
            return 12;
          case d.C:
            return 12;
          case d.d:
            return 13;
          case d.D:
            return 13;
          case d.e:
            return 14;
          case d.E:
            return 14;
          case d.f:
            return 15;
          case d.F:
            return 15;
        }
        return 0;
      }
    })(t.CSS || (t.CSS = {}));
  })(e.Format || (e.Format = {}));
})(gt || (gt = {}));
function Pr(e) {
  const t = [];
  for (const n of e) {
    const i = Number(n);
    (i || i === 0 && n.replace(/\s/g, "") !== "") && t.push(i);
  }
  return t;
}
function Yn(e, t, n, i) {
  return {
    red: e / 255,
    blue: n / 255,
    green: t / 255,
    alpha: i
  };
}
function dt(e, t) {
  const n = t.index, i = t[0].length;
  if (!n)
    return;
  const s = e.positionAt(n);
  return {
    startLineNumber: s.lineNumber,
    startColumn: s.column,
    endLineNumber: s.lineNumber,
    endColumn: s.column + i
  };
}
function oa(e, t) {
  if (!e)
    return;
  const n = gt.Format.CSS.parseHex(t);
  if (n)
    return {
      range: e,
      color: Yn(n.rgba.r, n.rgba.g, n.rgba.b, n.rgba.a)
    };
}
function or(e, t, n) {
  if (!e || t.length !== 1)
    return;
  const s = t[0].values(), r = Pr(s);
  return {
    range: e,
    color: Yn(r[0], r[1], r[2], n ? r[3] : 1)
  };
}
function cr(e, t, n) {
  if (!e || t.length !== 1)
    return;
  const s = t[0].values(), r = Pr(s), a = new gt(new Le(
    r[0],
    r[1] / 100,
    r[2] / 100,
    n ? r[3] : 1
  ));
  return {
    range: e,
    color: Yn(a.rgba.r, a.rgba.g, a.rgba.b, a.rgba.a)
  };
}
function Lt(e, t) {
  return typeof e == "string" ? [...e.matchAll(t)] : e.findMatches(t);
}
function ca(e) {
  const t = [], i = Lt(e, /\b(rgb|rgba|hsl|hsla)(\([0-9\s,.\%]*\))|(#)([A-Fa-f0-9]{3})\b|(#)([A-Fa-f0-9]{4})\b|(#)([A-Fa-f0-9]{6})\b|(#)([A-Fa-f0-9]{8})\b/gm);
  if (i.length > 0)
    for (const s of i) {
      const r = s.filter((c) => c !== void 0), a = r[1], u = r[2];
      if (!u)
        continue;
      let o;
      if (a === "rgb") {
        const c = /^\(\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*\)$/gm;
        o = or(dt(e, s), Lt(u, c), !1);
      } else if (a === "rgba") {
        const c = /^\(\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(0[.][0-9]+|[.][0-9]+|[01][.]|[01])\s*\)$/gm;
        o = or(dt(e, s), Lt(u, c), !0);
      } else if (a === "hsl") {
        const c = /^\(\s*(36[0]|3[0-5][0-9]|[12][0-9][0-9]|[1-9]?[0-9])\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*\)$/gm;
        o = cr(dt(e, s), Lt(u, c), !1);
      } else if (a === "hsla") {
        const c = /^\(\s*(36[0]|3[0-5][0-9]|[12][0-9][0-9]|[1-9]?[0-9])\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*,\s*(0[.][0-9]+|[.][0-9]+|[01][.]|[01])\s*\)$/gm;
        o = cr(dt(e, s), Lt(u, c), !0);
      } else a === "#" && (o = oa(dt(e, s), a + u));
      o && t.push(o);
    }
  return t;
}
function fa(e) {
  return !e || typeof e.getValue != "function" || typeof e.positionAt != "function" ? [] : ca(e);
}
const fr = new RegExp("\\bMARK:\\s*(.*)$", "d"), ha = /^-+|-+$/g;
function ma(e, t) {
  var i;
  let n = [];
  if (t.findRegionSectionHeaders && ((i = t.foldingRules) != null && i.markers)) {
    const s = ga(e, t);
    n = n.concat(s);
  }
  if (t.findMarkSectionHeaders) {
    const s = _a(e);
    n = n.concat(s);
  }
  return n;
}
function ga(e, t) {
  const n = [], i = e.getLineCount();
  for (let s = 1; s <= i; s++) {
    const r = e.getLineContent(s), a = r.match(t.foldingRules.markers.start);
    if (a) {
      const u = { startLineNumber: s, startColumn: a[0].length + 1, endLineNumber: s, endColumn: r.length + 1 };
      if (u.endColumn > u.startColumn) {
        const o = {
          range: u,
          ...yr(r.substring(a[0].length)),
          shouldBeInComments: !1
        };
        (o.text || o.hasSeparatorLine) && n.push(o);
      }
    }
  }
  return n;
}
function _a(e) {
  const t = [], n = e.getLineCount();
  for (let i = 1; i <= n; i++) {
    const s = e.getLineContent(i);
    ba(s, i, t);
  }
  return t;
}
function ba(e, t, n) {
  fr.lastIndex = 0;
  const i = fr.exec(e);
  if (i) {
    const s = i.indices[1][0] + 1, r = i.indices[1][1] + 1, a = { startLineNumber: t, startColumn: s, endLineNumber: t, endColumn: r };
    if (a.endColumn > a.startColumn) {
      const u = {
        range: a,
        ...yr(i[1]),
        shouldBeInComments: !0
      };
      (u.text || u.hasSeparatorLine) && n.push(u);
    }
  }
}
function yr(e) {
  e = e.trim();
  const t = e.startsWith("-");
  return e = e.replace(ha, ""), { text: e, hasSeparatorLine: t };
}
class da extends c1 {
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
    for (let i = 0; i < this._lines.length; i++) {
      const s = this._lines[i], r = this.offsetAt(new X(i + 1, 1)), a = s.matchAll(t);
      for (const u of a)
        (u.index || u.index === 0) && (u.index = u.index + r), n.push(u);
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
    const i = $n(t.column, Mr(n), this._lines[t.lineNumber - 1], 0);
    return i ? new S(
      t.lineNumber,
      i.startColumn,
      t.lineNumber,
      i.endColumn
    ) : null;
  }
  getWordUntilPosition(t, n) {
    const i = this.getWordAtPosition(t, n);
    return i ? {
      word: this._lines[t.lineNumber - 1].substring(i.startColumn - 1, t.column - 1),
      startColumn: i.startColumn,
      endColumn: t.column
    } : {
      word: "",
      startColumn: t.column,
      endColumn: t.column
    };
  }
  words(t) {
    const n = this._lines, i = this._wordenize.bind(this);
    let s = 0, r = "", a = 0, u = [];
    return {
      *[Symbol.iterator]() {
        for (; ; )
          if (a < u.length) {
            const o = r.substring(u[a].start, u[a].end);
            a += 1, yield o;
          } else if (s < n.length)
            r = n[s], u = i(r, t), a = 0, s += 1;
          else
            break;
      }
    };
  }
  getLineWords(t, n) {
    const i = this._lines[t - 1], s = this._wordenize(i, n), r = [];
    for (const a of s)
      r.push({
        word: i.substring(a.start, a.end),
        startColumn: a.start + 1,
        endColumn: a.end + 1
      });
    return r;
  }
  _wordenize(t, n) {
    const i = [];
    let s;
    for (n.lastIndex = 0; (s = n.exec(t)) && s[0].length !== 0; )
      i.push({ start: s.index, end: s.index + s[0].length });
    return i;
  }
  getValueInRange(t) {
    if (t = this._validateRange(t), t.startLineNumber === t.endLineNumber)
      return this._lines[t.startLineNumber - 1].substring(t.startColumn - 1, t.endColumn - 1);
    const n = this._eol, i = t.startLineNumber - 1, s = t.endLineNumber - 1, r = [];
    r.push(this._lines[i].substring(t.startColumn - 1));
    for (let a = i + 1; a < s; a++)
      r.push(this._lines[a]);
    return r.push(this._lines[s].substring(0, t.endColumn - 1)), r.join(n);
  }
  offsetAt(t) {
    return t = this._validatePosition(t), this._ensureLineStarts(), this._lineStarts.getPrefixSum(t.lineNumber - 2) + (t.column - 1);
  }
  positionAt(t) {
    t = Math.floor(t), t = Math.max(0, t), this._ensureLineStarts();
    const n = this._lineStarts.getIndexOf(t), i = this._lines[n.index].length;
    return {
      lineNumber: 1 + n.index,
      column: 1 + Math.min(n.remainder, i)
    };
  }
  _validateRange(t) {
    const n = this._validatePosition({ lineNumber: t.startLineNumber, column: t.startColumn }), i = this._validatePosition({ lineNumber: t.endLineNumber, column: t.endColumn });
    return n.lineNumber !== t.startLineNumber || n.column !== t.startColumn || i.lineNumber !== t.endLineNumber || i.column !== t.endColumn ? {
      startLineNumber: n.lineNumber,
      startColumn: n.column,
      endLineNumber: i.lineNumber,
      endColumn: i.column
    } : t;
  }
  _validatePosition(t) {
    if (!X.isIPosition(t))
      throw new Error("bad position");
    let { lineNumber: n, column: i } = t, s = !1;
    if (n < 1)
      n = 1, i = 1, s = !0;
    else if (n > this._lines.length)
      n = this._lines.length, i = this._lines[n - 1].length + 1, s = !0;
    else {
      const r = this._lines[n - 1].length + 1;
      i < 1 ? (i = 1, s = !0) : i > r && (i = r, s = !0);
    }
    return s ? { lineNumber: n, column: i } : t;
  }
}
const Oe = class Oe {
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
    this._models[t.url] = new da(Ye.parse(t.url), t.lines, t.EOL, t.versionId);
  }
  acceptModelChanged(t, n) {
    if (!this._models[t])
      return;
    this._models[t].onEvents(n);
  }
  acceptRemovedModel(t) {
    this._models[t] && delete this._models[t];
  }
  async computeUnicodeHighlights(t, n, i) {
    const s = this._getModel(t);
    return s ? P1.computeUnicodeHighlights(s, n, i) : { ranges: [], hasMore: !1, ambiguousCharacterCount: 0, invisibleCharacterCount: 0, nonBasicAsciiCharacterCount: 0 };
  }
  async findSectionHeaders(t, n) {
    const i = this._getModel(t);
    return i ? ma(i, n) : [];
  }
  async computeDiff(t, n, i, s) {
    const r = this._getModel(t), a = this._getModel(n);
    return !r || !a ? null : Oe.computeDiff(r, a, i, s);
  }
  static computeDiff(t, n, i, s) {
    const r = s === "advanced" ? bn.getDefault() : bn.getLegacy(), a = t.getLinesContent(), u = n.getLinesContent(), o = r.computeDiff(a, u, i), c = o.changes.length > 0 ? !1 : this._modelsAreIdentical(t, n);
    function h(f) {
      return f.map(
        (g) => {
          var b;
          return [g.original.startLineNumber, g.original.endLineNumberExclusive, g.modified.startLineNumber, g.modified.endLineNumberExclusive, (b = g.innerChanges) == null ? void 0 : b.map((L) => [
            L.originalRange.startLineNumber,
            L.originalRange.startColumn,
            L.originalRange.endLineNumber,
            L.originalRange.endColumn,
            L.modifiedRange.startLineNumber,
            L.modifiedRange.startColumn,
            L.modifiedRange.endLineNumber,
            L.modifiedRange.endColumn
          ])];
        }
      );
    }
    return {
      identical: c,
      quitEarly: o.hitTimeout,
      changes: h(o.changes),
      moves: o.moves.map((f) => [
        f.lineRangeMapping.original.startLineNumber,
        f.lineRangeMapping.original.endLineNumberExclusive,
        f.lineRangeMapping.modified.startLineNumber,
        f.lineRangeMapping.modified.endLineNumberExclusive,
        h(f.changes)
      ])
    };
  }
  static _modelsAreIdentical(t, n) {
    const i = t.getLineCount(), s = n.getLineCount();
    if (i !== s)
      return !1;
    for (let r = 1; r <= i; r++) {
      const a = t.getLineContent(r), u = n.getLineContent(r);
      if (a !== u)
        return !1;
    }
    return !0;
  }
  async computeDirtyDiff(t, n, i) {
    const s = this._getModel(t), r = this._getModel(n);
    if (!s || !r)
      return null;
    const a = s.getLinesContent(), u = r.getLinesContent();
    return new Sr(a, u, {
      shouldComputeCharChanges: !1,
      shouldPostProcessCharChanges: !1,
      shouldIgnoreTrimWhitespace: i,
      shouldMakePrettyDiff: !0,
      maxComputationTime: 1e3
    }).computeDiff().changes;
  }
  async computeMoreMinimalEdits(t, n, i) {
    const s = this._getModel(t);
    if (!s)
      return n;
    const r = [];
    let a;
    n = n.slice(0).sort((o, c) => {
      if (o.range && c.range)
        return S.compareRangesUsingStarts(o.range, c.range);
      const h = o.range ? 0 : 1, f = c.range ? 0 : 1;
      return h - f;
    });
    let u = 0;
    for (let o = 1; o < n.length; o++)
      S.getEndPosition(n[u].range).equals(S.getStartPosition(n[o].range)) ? (n[u].range = S.fromPositions(S.getStartPosition(n[u].range), S.getEndPosition(n[o].range)), n[u].text += n[o].text) : (u++, n[u] = n[o]);
    n.length = u + 1;
    for (let { range: o, text: c, eol: h } of n) {
      if (typeof h == "number" && (a = h), S.isEmpty(o) && !c)
        continue;
      const f = s.getValueInRange(o);
      if (c = c.replace(/\r\n|\n|\r/g, s.eol), f === c)
        continue;
      if (Math.max(c.length, f.length) > Oe._diffLimit) {
        r.push({ range: o, text: c });
        continue;
      }
      const g = Gl(f, c, i), b = s.offsetAt(S.lift(o).getStartPosition());
      for (const L of g) {
        const N = s.positionAt(b + L.originalStart), A = s.positionAt(b + L.originalStart + L.originalLength), x = {
          text: c.substr(L.modifiedStart, L.modifiedLength),
          range: { startLineNumber: N.lineNumber, startColumn: N.column, endLineNumber: A.lineNumber, endColumn: A.column }
        };
        s.getValueInRange(x.range) !== x.text && r.push(x);
      }
    }
    return typeof a == "number" && r.push({ eol: a, text: "", range: { startLineNumber: 0, startColumn: 0, endLineNumber: 0, endColumn: 0 } }), r;
  }
  computeHumanReadableDiff(t, n, i) {
    const s = this._getModel(t);
    if (!s)
      return n;
    const r = [];
    let a;
    n = n.slice(0).sort((u, o) => {
      if (u.range && o.range)
        return S.compareRangesUsingStarts(u.range, o.range);
      const c = u.range ? 0 : 1, h = o.range ? 0 : 1;
      return c - h;
    });
    for (let { range: u, text: o, eol: c } of n) {
      let N = function(x, R) {
        return new X(
          x.lineNumber + R.lineNumber - 1,
          R.lineNumber === 1 ? x.column + R.column - 1 : R.column
        );
      }, A = function(x, R) {
        const U = [];
        for (let p = R.startLineNumber; p <= R.endLineNumber; p++) {
          const w = x[p - 1];
          p === R.startLineNumber && p === R.endLineNumber ? U.push(w.substring(R.startColumn - 1, R.endColumn - 1)) : p === R.startLineNumber ? U.push(w.substring(R.startColumn - 1)) : p === R.endLineNumber ? U.push(w.substring(0, R.endColumn - 1)) : U.push(w);
        }
        return U;
      };
      if (typeof c == "number" && (a = c), S.isEmpty(u) && !o)
        continue;
      const h = s.getValueInRange(u);
      if (o = o.replace(/\r\n|\n|\r/g, s.eol), h === o)
        continue;
      if (Math.max(o.length, h.length) > Oe._diffLimit) {
        r.push({ range: u, text: o });
        continue;
      }
      const f = h.split(/\r\n|\n|\r/), g = o.split(/\r\n|\n|\r/), b = bn.getDefault().computeDiff(f, g, i), L = S.lift(u).getStartPosition();
      for (const x of b.changes)
        if (x.innerChanges)
          for (const R of x.innerChanges)
            r.push({
              range: S.fromPositions(N(L, R.originalRange.getStartPosition()), N(L, R.originalRange.getEndPosition())),
              text: A(g, R.modifiedRange).join(s.eol)
            });
        else
          throw new me("The experimental diff algorithm always produces inner changes");
    }
    return typeof a == "number" && r.push({ eol: a, text: "", range: { startLineNumber: 0, startColumn: 0, endLineNumber: 0, endColumn: 0 } }), r;
  }
  async computeLinks(t) {
    const n = this._getModel(t);
    return n ? L1(n) : null;
  }
  async computeDefaultDocumentColors(t) {
    const n = this._getModel(t);
    return n ? fa(n) : null;
  }
  async textualSuggest(t, n, i, s) {
    const r = new on(), a = new RegExp(i, s), u = /* @__PURE__ */ new Set();
    e: for (const o of t) {
      const c = this._getModel(o);
      if (c) {
        for (const h of c.words(a))
          if (!(h === n || !isNaN(Number(h))) && (u.add(h), u.size > Oe._suggestionsLimit))
            break e;
      }
    }
    return { words: Array.from(u), duration: r.elapsed() };
  }
  async computeWordRanges(t, n, i, s) {
    const r = this._getModel(t);
    if (!r)
      return /* @__PURE__ */ Object.create(null);
    const a = new RegExp(i, s), u = /* @__PURE__ */ Object.create(null);
    for (let o = n.startLineNumber; o < n.endLineNumber; o++) {
      const c = r.getLineWords(o, a);
      for (const h of c) {
        if (!isNaN(Number(h.word)))
          continue;
        let f = u[h.word];
        f || (f = [], u[h.word] = f), f.push({
          startLineNumber: o,
          startColumn: h.startColumn,
          endLineNumber: o,
          endColumn: h.endColumn
        });
      }
    }
    return u;
  }
  async navigateValueSet(t, n, i, s, r) {
    const a = this._getModel(t);
    if (!a)
      return null;
    const u = new RegExp(s, r);
    n.startColumn === n.endColumn && (n = {
      startLineNumber: n.startLineNumber,
      startColumn: n.startColumn,
      endLineNumber: n.endLineNumber,
      endColumn: n.endColumn + 1
    });
    const o = a.getValueInRange(n), c = a.getWordAtPosition({ lineNumber: n.startLineNumber, column: n.startColumn }, u);
    if (!c)
      return null;
    const h = a.getValueInRange(c);
    return Tn.INSTANCE.navigateValueSet(n, o, c, h, i);
  }
  loadForeignModule(t, n, i) {
    const a = {
      host: bl(i, (u, o) => this._host.fhr(u, o)),
      getMirrorModels: () => this._getModels()
    };
    return this._foreignModuleFactory ? (this._foreignModule = this._foreignModuleFactory(a, n), Promise.resolve(An(this._foreignModule))) : Promise.reject(new Error("Unexpected usage"));
  }
  fmr(t, n) {
    if (!this._foreignModule || typeof this._foreignModule[t] != "function")
      return Promise.reject(new Error("Missing requestHandler or method: " + t));
    try {
      return Promise.resolve(this._foreignModule[t].apply(this._foreignModule, n));
    } catch (i) {
      return Promise.reject(i);
    }
  }
};
Oe._diffLimit = 1e5, Oe._suggestionsLimit = 1e4;
let qn = Oe;
typeof importScripts == "function" && (globalThis.monaco = M1());
let Wn = !1;
function La(e) {
  if (Wn)
    return;
  Wn = !0;
  const t = new Hl((n) => {
    globalThis.postMessage(n);
  }, (n) => new qn(n, e));
  globalThis.onmessage = (n) => {
    t.onmessage(n.data);
  };
}
globalThis.onmessage = (e) => {
  Wn || La(null);
};
export {
  La as initialize
};
