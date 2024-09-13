var jl = Object.defineProperty;
var Gl = (e, t, n) => t in e ? jl(e, t, { enumerable: !0, configurable: !0, writable: !0, value: n }) : e[t] = n;
var it = (e, t, n) => Gl(e, typeof t != "symbol" ? t + "" : t, n);
class zl {
  constructor() {
    this.listeners = [], this.unexpectedErrorHandler = function(t) {
      setTimeout(() => {
        throw t.stack ? Gt.isErrorNoTelemetry(t) ? new Gt(t.message + `

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
const Xl = new zl();
function In(e) {
  Jl(e) || Xl.onUnexpectedError(e);
}
function yr(e) {
  if (e instanceof Error) {
    const { name: t, message: n } = e, i = e.stacktrace || e.stack;
    return {
      $isError: !0,
      name: t,
      message: n,
      stack: i,
      noTelemetry: Gt.isErrorNoTelemetry(e)
    };
  }
  return e;
}
const Ai = "Canceled";
function Jl(e) {
  return e instanceof Yl ? !0 : e instanceof Error && e.name === Ai && e.message === Ai;
}
class Yl extends Error {
  constructor() {
    super(Ai), this.name = this.message;
  }
}
class Gt extends Error {
  constructor(t) {
    super(t), this.name = "CodeExpectedError";
  }
  static fromError(t) {
    if (t instanceof Gt)
      return t;
    const n = new Gt();
    return n.message = t.message, n.stack = t.stack, n;
  }
  static isErrorNoTelemetry(t) {
    return t.name === "CodeExpectedError";
  }
}
class ye extends Error {
  constructor(t) {
    super(t || "An unexpected bug occurred."), Object.setPrototypeOf(this, ye.prototype);
  }
}
function Ql(e, t) {
  const n = this;
  let i = !1, r;
  return function() {
    return i || (i = !0, r = e.apply(n, arguments)), r;
  };
}
function zt(e, t) {
  const n = dn(e, t);
  return n === -1 ? void 0 : e[n];
}
function dn(e, t, n = 0, i = e.length) {
  let r = n, s = i;
  for (; r < s; ) {
    const a = Math.floor((r + s) / 2);
    t(e[a]) ? r = a + 1 : s = a;
  }
  return r - 1;
}
function Zl(e, t) {
  const n = xi(e, t);
  return n === e.length ? void 0 : e[n];
}
function xi(e, t, n = 0, i = e.length) {
  let r = n, s = i;
  for (; r < s; ) {
    const a = Math.floor((r + s) / 2);
    t(e[a]) ? s = a : r = a + 1;
  }
  return r;
}
const ri = class ri {
  constructor(t) {
    this._array = t, this._findLastMonotonousLastIdx = 0;
  }
  findLastMonotonous(t) {
    if (ri.assertInvariants) {
      if (this._prevFindLastPredicate) {
        for (const i of this._array)
          if (this._prevFindLastPredicate(i) && !t(i))
            throw new Error(
              "MonotonousArray: current predicate must be weaker than (or equal to) the previous predicate."
            );
      }
      this._prevFindLastPredicate = t;
    }
    const n = dn(this._array, t, this._findLastMonotonousLastIdx);
    return this._findLastMonotonousLastIdx = n + 1, n === -1 ? void 0 : this._array[n];
  }
};
ri.assertInvariants = !1;
let Vn = ri;
function Kl(e, t, n = (i, r) => i === r) {
  if (e === t)
    return !0;
  if (!e || !t || e.length !== t.length)
    return !1;
  for (let i = 0, r = e.length; i < r; i++)
    if (!n(e[i], t[i]))
      return !1;
  return !0;
}
function* Cl(e, t) {
  let n, i;
  for (const r of e)
    i !== void 0 && t(i, r) ? n.push(r) : (n && (yield n), n = [r]), i = r;
  n && (yield n);
}
function eu(e, t) {
  for (let n = 0; n <= e.length; n++)
    t(n === 0 ? void 0 : e[n - 1], n === e.length ? void 0 : e[n]);
}
function tu(e, t) {
  for (let n = 0; n < e.length; n++)
    t(n === 0 ? void 0 : e[n - 1], e[n], n + 1 === e.length ? void 0 : e[n + 1]);
}
function nu(e, t) {
  for (const n of t)
    e.push(n);
}
var Ei;
(function(e) {
  function t(s) {
    return s < 0;
  }
  e.isLessThan = t;
  function n(s) {
    return s <= 0;
  }
  e.isLessThanOrEqual = n;
  function i(s) {
    return s > 0;
  }
  e.isGreaterThan = i;
  function r(s) {
    return s === 0;
  }
  e.isNeitherLessOrGreaterThan = r, e.greaterThan = 1, e.lessThan = -1, e.neitherLessOrGreaterThan = 0;
})(Ei || (Ei = {}));
function on(e, t) {
  return (n, i) => t(e(n), e(i));
}
const ln = (e, t) => e - t;
function iu(e) {
  return (t, n) => -e(t, n);
}
const Bt = class Bt {
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
    return new Bt((n) => this.iterate((i) => t(i) ? n(i) : !0));
  }
  map(t) {
    return new Bt((n) => this.iterate((i) => n(t(i))));
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
    return this.iterate((r) => ((i || Ei.isGreaterThan(t(r, n))) && (i = !1, n = r), !0)), n;
  }
};
Bt.empty = new Bt((t) => {
});
let Rr = Bt;
function ru(e, t) {
  const n = /* @__PURE__ */ Object.create(null);
  for (const i of e) {
    const r = t(i);
    let s = n[r];
    s || (s = n[r] = []), s.push(i);
  }
  return n;
}
var kr, Tr;
class su {
  constructor(t, n) {
    this.uri = t, this.value = n;
  }
}
function au(e) {
  return Array.isArray(e);
}
const _t = class _t {
  constructor(t, n) {
    if (this[kr] = "ResourceMap", t instanceof _t)
      this.map = new Map(t.map), this.toKey = n ?? _t.defaultToKey;
    else if (au(t)) {
      this.map = /* @__PURE__ */ new Map(), this.toKey = n ?? _t.defaultToKey;
      for (const [i, r] of t)
        this.set(i, r);
    } else
      this.map = /* @__PURE__ */ new Map(), this.toKey = t ?? _t.defaultToKey;
  }
  set(t, n) {
    return this.map.set(this.toKey(t), new su(t, n)), this;
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
    for (const [i, r] of this.map)
      t(r.value, r.uri, this);
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
  *[(kr = Symbol.toStringTag, Symbol.iterator)]() {
    for (const [, t] of this.map)
      yield [t.uri, t.value];
  }
};
_t.defaultToKey = (t) => t.toString();
let Sr = _t;
var Ne;
(function(e) {
  e[e.None = 0] = "None", e[e.AsOld = 1] = "AsOld", e[e.AsNew = 2] = "AsNew";
})(Ne || (Ne = {}));
class ou {
  constructor() {
    this[Tr] = "LinkedMap", this._map = /* @__PURE__ */ new Map(), this._head = void 0, this._tail = void 0, this._size = 0, this._state = 0;
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
  get(t, n = Ne.None) {
    const i = this._map.get(t);
    if (i)
      return n !== Ne.None && this.touch(i, n), i.value;
  }
  set(t, n, i = Ne.None) {
    let r = this._map.get(t);
    if (r)
      r.value = n, i !== Ne.None && this.touch(r, i);
    else {
      switch (r = { key: t, value: n, next: void 0, previous: void 0 }, i) {
        case Ne.None:
          this.addItemLast(r);
          break;
        case Ne.AsOld:
          this.addItemFirst(r);
          break;
        case Ne.AsNew:
          this.addItemLast(r);
          break;
        default:
          this.addItemLast(r);
          break;
      }
      this._map.set(t, r), this._size++;
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
    let r = this._head;
    for (; r; ) {
      if (n ? t.bind(n)(r.value, r.key, this) : t(r.value, r.key, this), this._state !== i)
        throw new Error("LinkedMap got modified during iteration.");
      r = r.next;
    }
  }
  keys() {
    const t = this, n = this._state;
    let i = this._head;
    const r = {
      [Symbol.iterator]() {
        return r;
      },
      next() {
        if (t._state !== n)
          throw new Error("LinkedMap got modified during iteration.");
        if (i) {
          const s = { value: i.key, done: !1 };
          return i = i.next, s;
        } else
          return { value: void 0, done: !0 };
      }
    };
    return r;
  }
  values() {
    const t = this, n = this._state;
    let i = this._head;
    const r = {
      [Symbol.iterator]() {
        return r;
      },
      next() {
        if (t._state !== n)
          throw new Error("LinkedMap got modified during iteration.");
        if (i) {
          const s = { value: i.value, done: !1 };
          return i = i.next, s;
        } else
          return { value: void 0, done: !0 };
      }
    };
    return r;
  }
  entries() {
    const t = this, n = this._state;
    let i = this._head;
    const r = {
      [Symbol.iterator]() {
        return r;
      },
      next() {
        if (t._state !== n)
          throw new Error("LinkedMap got modified during iteration.");
        if (i) {
          const s = { value: [i.key, i.value], done: !1 };
          return i = i.next, s;
        } else
          return { value: void 0, done: !0 };
      }
    };
    return r;
  }
  [(Tr = Symbol.toStringTag, Symbol.iterator)]() {
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
    if (!(n !== Ne.AsOld && n !== Ne.AsNew)) {
      if (n === Ne.AsOld) {
        if (t === this._head)
          return;
        const i = t.next, r = t.previous;
        t === this._tail ? (r.next = void 0, this._tail = r) : (i.previous = r, r.next = i), t.previous = void 0, t.next = this._head, this._head.previous = t, this._head = t, this._state++;
      } else if (n === Ne.AsNew) {
        if (t === this._tail)
          return;
        const i = t.next, r = t.previous;
        t === this._head ? (i.previous = void 0, this._head = i) : (i.previous = r, r.next = i), t.next = void 0, t.previous = this._tail, this._tail.next = t, this._tail = t, this._state++;
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
class lu extends ou {
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
  get(t, n = Ne.AsNew) {
    return super.get(t, n);
  }
  peek(t) {
    return super.get(t, Ne.None);
  }
  set(t, n) {
    return super.set(t, n, Ne.AsNew), this;
  }
  checkTrim() {
    this.size > this._limit && this.trim(Math.round(this._limit * this._ratio));
  }
}
class uu extends lu {
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
class il {
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
var qn;
(function(e) {
  function t(L) {
    return L && typeof L == "object" && typeof L[Symbol.iterator] == "function";
  }
  e.is = t;
  const n = Object.freeze([]);
  function i() {
    return n;
  }
  e.empty = i;
  function* r(L) {
    yield L;
  }
  e.single = r;
  function s(L) {
    return t(L) ? L : r(L);
  }
  e.wrap = s;
  function a(L) {
    return L || n;
  }
  e.from = a;
  function* l(L) {
    for (let b = L.length - 1; b >= 0; b--)
      yield L[b];
  }
  e.reverse = l;
  function o(L) {
    return !L || L[Symbol.iterator]().next().done === !0;
  }
  e.isEmpty = o;
  function u(L) {
    return L[Symbol.iterator]().next().value;
  }
  e.first = u;
  function h(L, b) {
    let E = 0;
    for (const k of L)
      if (b(k, E++))
        return !0;
    return !1;
  }
  e.some = h;
  function c(L, b) {
    for (const E of L)
      if (b(E))
        return E;
  }
  e.find = c;
  function* m(L, b) {
    for (const E of L)
      b(E) && (yield E);
  }
  e.filter = m;
  function* g(L, b) {
    let E = 0;
    for (const k of L)
      yield b(k, E++);
  }
  e.map = g;
  function* d(L, b) {
    let E = 0;
    for (const k of L)
      yield* b(k, E++);
  }
  e.flatMap = d;
  function* p(...L) {
    for (const b of L)
      yield* b;
  }
  e.concat = p;
  function _(L, b, E) {
    let k = E;
    for (const D of L)
      k = b(k, D);
    return k;
  }
  e.reduce = _;
  function* x(L, b, E = L.length) {
    for (b < 0 && (b += L.length), E < 0 ? E += L.length : E > L.length && (E = L.length); b < E; b++)
      yield L[b];
  }
  e.slice = x;
  function R(L, b = Number.POSITIVE_INFINITY) {
    const E = [];
    if (b === 0)
      return [E, L];
    const k = L[Symbol.iterator]();
    for (let D = 0; D < b; D++) {
      const O = k.next();
      if (O.done)
        return [E, e.empty()];
      E.push(O.value);
    }
    return [E, { [Symbol.iterator]() {
      return k;
    } }];
  }
  e.consume = R;
  async function v(L) {
    const b = [];
    for await (const E of L)
      b.push(E);
    return Promise.resolve(b);
  }
  e.asyncToArray = v;
})(qn || (qn = {}));
const si = class si {
  constructor() {
    this.livingDisposables = /* @__PURE__ */ new Map();
  }
  getDisposableData(t) {
    let n = this.livingDisposables.get(t);
    return n || (n = { parent: null, source: null, isSingleton: !1, value: t, idx: si.idx++ }, this.livingDisposables.set(t, n)), n;
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
    const r = t.parent ? this.getRootParent(this.getDisposableData(t.parent), n) : t;
    return n.set(t, r), r;
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
      const o = /* @__PURE__ */ new Map(), u = [...this.livingDisposables.values()].filter((c) => c.source !== null && !this.getRootParent(c, o).isSingleton);
      if (u.length === 0)
        return;
      const h = new Set(u.map((c) => c.value));
      if (i = u.filter((c) => !(c.parent && h.has(c.parent))), i.length === 0)
        throw new Error("There are cyclic diposable chains!");
    }
    if (!i)
      return;
    function r(o) {
      function u(c, m) {
        for (; c.length > 0 && m.some(
          (g) => typeof g == "string" ? g === c[0] : c[0].match(g)
        ); )
          c.shift();
      }
      const h = o.source.split(`
`).map((c) => c.trim().replace("at ", "")).filter((c) => c !== "");
      return u(h, ["Error", /^trackDisposable \(.*\)$/, /^DisposableTracker.trackDisposable \(.*\)$/]), h.reverse();
    }
    const s = new il();
    for (const o of i) {
      const u = r(o);
      for (let h = 0; h <= u.length; h++)
        s.add(u.slice(0, h).join(`
`), o);
    }
    i.sort(on((o) => o.idx, ln));
    let a = "", l = 0;
    for (const o of i.slice(0, t)) {
      l++;
      const u = r(o), h = [];
      for (let c = 0; c < u.length; c++) {
        let m = u[c];
        m = `(shared with ${s.get(u.slice(0, c + 1).join(`
`)).size}/${i.length} leaks) at ${m}`;
        const d = s.get(u.slice(0, c).join(`
`)), p = ru([...d].map((_) => r(_)[c]), (_) => _);
        delete p[u[c]];
        for (const [_, x] of Object.entries(p))
          h.unshift(`    - stacktraces of ${x.length} other leaks continue with ${_}`);
        h.unshift(m);
      }
      a += `


==================== Leaking disposable ${l}/${i.length}: ${o.value.constructor.name} ====================
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
si.idx = 0;
let Mr = si;
function rl(e) {
  if (qn.is(e)) {
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
function cu(...e) {
  return Hn(() => rl(e));
}
function Hn(e) {
  return {
    dispose: Ql(() => {
      e();
    })
  };
}
const ai = class ai {
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
        rl(this._toDispose);
      } finally {
        this._toDispose.clear();
      }
  }
  add(t) {
    if (!t)
      return t;
    if (t === this)
      throw new Error("Cannot register a disposable on itself!");
    return this._isDisposed ? ai.DISABLE_DISPOSED_WARNING || console.warn(new Error(
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
ai.DISABLE_DISPOSED_WARNING = !1;
let pn = ai;
const Er = class Er {
  constructor() {
    this._store = new pn(), this._store;
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
Er.None = Object.freeze({ dispose() {
} });
let Xt = Er;
const Vt = class Vt {
  constructor(t) {
    this.element = t, this.next = Vt.Undefined, this.prev = Vt.Undefined;
  }
};
Vt.Undefined = new Vt(void 0);
let ie = Vt;
class fu {
  constructor() {
    this._first = ie.Undefined, this._last = ie.Undefined, this._size = 0;
  }
  get size() {
    return this._size;
  }
  isEmpty() {
    return this._first === ie.Undefined;
  }
  clear() {
    let t = this._first;
    for (; t !== ie.Undefined; ) {
      const n = t.next;
      t.prev = ie.Undefined, t.next = ie.Undefined, t = n;
    }
    this._first = ie.Undefined, this._last = ie.Undefined, this._size = 0;
  }
  unshift(t) {
    return this._insert(t, !1);
  }
  push(t) {
    return this._insert(t, !0);
  }
  _insert(t, n) {
    const i = new ie(t);
    if (this._first === ie.Undefined)
      this._first = i, this._last = i;
    else if (n) {
      const s = this._last;
      this._last = i, i.prev = s, s.next = i;
    } else {
      const s = this._first;
      this._first = i, i.next = s, s.prev = i;
    }
    this._size += 1;
    let r = !1;
    return () => {
      r || (r = !0, this._remove(i));
    };
  }
  shift() {
    if (this._first !== ie.Undefined) {
      const t = this._first.element;
      return this._remove(this._first), t;
    }
  }
  pop() {
    if (this._last !== ie.Undefined) {
      const t = this._last.element;
      return this._remove(this._last), t;
    }
  }
  _remove(t) {
    if (t.prev !== ie.Undefined && t.next !== ie.Undefined) {
      const n = t.prev;
      n.next = t.next, t.next.prev = n;
    } else t.prev === ie.Undefined && t.next === ie.Undefined ? (this._first = ie.Undefined, this._last = ie.Undefined) : t.next === ie.Undefined ? (this._last = this._last.prev, this._last.next = ie.Undefined) : t.prev === ie.Undefined && (this._first = this._first.next, this._first.prev = ie.Undefined);
    this._size -= 1;
  }
  *[Symbol.iterator]() {
    let t = this._first;
    for (; t !== ie.Undefined; )
      yield t.element, t = t.next;
  }
}
const hu = globalThis.performance && typeof globalThis.performance.now == "function";
class fi {
  static create(t) {
    return new fi(t);
  }
  constructor(t) {
    this._now = hu && t === !1 ? Date.now : globalThis.performance.now.bind(globalThis.performance), this._startTime = this._now(), this._stopTime = -1;
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
var yi;
(function(e) {
  e.None = () => Xt.None;
  function t(A, y) {
    return c(A, () => {
    }, 0, void 0, !0, void 0, y);
  }
  e.defer = t;
  function n(A) {
    return (y, I = null, F) => {
      let P = !1, B;
      return B = A((G) => {
        if (!P)
          return B ? B.dispose() : P = !0, y.call(I, G);
      }, null, F), P && B.dispose(), B;
    };
  }
  e.once = n;
  function i(A, y, I) {
    return u((F, P = null, B) => A((G) => F.call(P, y(G)), null, B), I);
  }
  e.map = i;
  function r(A, y, I) {
    return u((F, P = null, B) => A((G) => {
      y(G), F.call(P, G);
    }, null, B), I);
  }
  e.forEach = r;
  function s(A, y, I) {
    return u((F, P = null, B) => A((G) => y(G) && F.call(P, G), null, B), I);
  }
  e.filter = s;
  function a(A) {
    return A;
  }
  e.signal = a;
  function l(...A) {
    return (y, I = null, F) => {
      const P = cu(...A.map((B) => B((G) => y.call(I, G))));
      return h(P, F);
    };
  }
  e.any = l;
  function o(A, y, I, F) {
    let P = I;
    return i(A, (B) => (P = y(P, B), P), F);
  }
  e.reduce = o;
  function u(A, y) {
    let I;
    const F = {
      onWillAddFirstListener() {
        I = A(P.fire, P);
      },
      onDidRemoveLastListener() {
        I == null || I.dispose();
      }
    }, P = new He(F);
    return y == null || y.add(P), P.event;
  }
  function h(A, y) {
    return y instanceof Array ? y.push(A) : y && y.add(A), A;
  }
  function c(A, y, I = 100, F = !1, P = !1, B, G) {
    let C, Q, Be, En = 0, pt;
    const Hl = {
      leakWarningThreshold: B,
      onWillAddFirstListener() {
        C = A(($l) => {
          En++, Q = y(Q, $l), F && !Be && (yn.fire(Q), Q = void 0), pt = () => {
            const Wl = Q;
            Q = void 0, Be = void 0, (!F || En > 1) && yn.fire(Wl), En = 0;
          }, typeof I == "number" ? (clearTimeout(Be), Be = setTimeout(pt, I)) : Be === void 0 && (Be = 0, queueMicrotask(pt));
        });
      },
      onWillRemoveListener() {
        P && En > 0 && (pt == null || pt());
      },
      onDidRemoveLastListener() {
        pt = void 0, C.dispose();
      }
    }, yn = new He(Hl);
    return G == null || G.add(yn), yn.event;
  }
  e.debounce = c;
  function m(A, y = 0, I) {
    return e.debounce(A, (F, P) => F ? (F.push(P), F) : [P], y, void 0, !0, void 0, I);
  }
  e.accumulate = m;
  function g(A, y = (F, P) => F === P, I) {
    let F = !0, P;
    return s(A, (B) => {
      const G = F || !y(B, P);
      return F = !1, P = B, G;
    }, I);
  }
  e.latch = g;
  function d(A, y, I) {
    return [
      e.filter(A, y, I),
      e.filter(A, (F) => !y(F), I)
    ];
  }
  e.split = d;
  function p(A, y = !1, I = [], F) {
    let P = I.slice(), B = A((Q) => {
      P ? P.push(Q) : C.fire(Q);
    });
    F && F.add(B);
    const G = () => {
      P == null || P.forEach((Q) => C.fire(Q)), P = null;
    }, C = new He({
      onWillAddFirstListener() {
        B || (B = A((Q) => C.fire(Q)), F && F.add(B));
      },
      onDidAddFirstListener() {
        P && (y ? setTimeout(G) : G());
      },
      onDidRemoveLastListener() {
        B && B.dispose(), B = null;
      }
    });
    return F && F.add(C), C.event;
  }
  e.buffer = p;
  function _(A, y) {
    return (F, P, B) => {
      const G = y(new R());
      return A(function(C) {
        const Q = G.evaluate(C);
        Q !== x && F.call(P, Q);
      }, void 0, B);
    };
  }
  e.chain = _;
  const x = Symbol("HaltChainable");
  class R {
    constructor() {
      this.steps = [];
    }
    map(y) {
      return this.steps.push(y), this;
    }
    forEach(y) {
      return this.steps.push((I) => (y(I), I)), this;
    }
    filter(y) {
      return this.steps.push((I) => y(I) ? I : x), this;
    }
    reduce(y, I) {
      let F = I;
      return this.steps.push((P) => (F = y(F, P), F)), this;
    }
    latch(y = (I, F) => I === F) {
      let I = !0, F;
      return this.steps.push((P) => {
        const B = I || !y(P, F);
        return I = !1, F = P, B ? P : x;
      }), this;
    }
    evaluate(y) {
      for (const I of this.steps)
        if (y = I(y), y === x)
          break;
      return y;
    }
  }
  function v(A, y, I = (F) => F) {
    const F = (...C) => G.fire(I(...C)), P = () => A.on(y, F), B = () => A.removeListener(y, F), G = new He(
      { onWillAddFirstListener: P, onDidRemoveLastListener: B }
    );
    return G.event;
  }
  e.fromNodeEventEmitter = v;
  function L(A, y, I = (F) => F) {
    const F = (...C) => G.fire(I(...C)), P = () => A.addEventListener(y, F), B = () => A.removeEventListener(y, F), G = new He(
      { onWillAddFirstListener: P, onDidRemoveLastListener: B }
    );
    return G.event;
  }
  e.fromDOMEventEmitter = L;
  function b(A) {
    return new Promise((y) => n(A)(y));
  }
  e.toPromise = b;
  function E(A) {
    const y = new He();
    return A.then((I) => {
      y.fire(I);
    }, () => {
      y.fire(void 0);
    }).finally(() => {
      y.dispose();
    }), y.event;
  }
  e.fromPromise = E;
  function k(A, y) {
    return A((I) => y.fire(I));
  }
  e.forward = k;
  function D(A, y, I) {
    return y(I), A((F) => y(F));
  }
  e.runAndSubscribe = D;
  class O {
    constructor(y, I) {
      this._observable = y, this._counter = 0, this._hasChanged = !1;
      const F = {
        onWillAddFirstListener: () => {
          y.addObserver(this);
        },
        onDidRemoveLastListener: () => {
          y.removeObserver(this);
        }
      };
      this.emitter = new He(F), I && I.add(this.emitter);
    }
    beginUpdate(y) {
      this._counter++;
    }
    handlePossibleChange(y) {
    }
    handleChange(y, I) {
      this._hasChanged = !0;
    }
    endUpdate(y) {
      this._counter--, this._counter === 0 && (this._observable.reportChanges(), this._hasChanged && (this._hasChanged = !1, this.emitter.fire(this._observable.get())));
    }
  }
  function W(A, y) {
    return new O(A, y).emitter.event;
  }
  e.fromObservable = W;
  function U(A) {
    return (y, I, F) => {
      let P = 0, B = !1;
      const G = {
        beginUpdate() {
          P++;
        },
        endUpdate() {
          P--, P === 0 && (A.reportChanges(), B && (B = !1, y.call(I)));
        },
        handlePossibleChange() {
        },
        handleChange() {
          B = !0;
        }
      };
      A.addObserver(G), A.reportChanges();
      const C = {
        dispose() {
          A.removeObserver(G);
        }
      };
      return F instanceof pn ? F.add(C) : Array.isArray(F) && F.push(C), C;
    };
  }
  e.fromObservableLight = U;
})(yi || (yi = {}));
const qt = class qt {
  constructor(t) {
    this.listenerCount = 0, this.invocationCount = 0, this.elapsedOverall = 0, this.durations = [], this.name = `${t}_${qt._idPool++}`, qt.all.add(this);
  }
  start(t) {
    this._stopWatch = new fi(), this.listenerCount = t;
  }
  stop() {
    if (this._stopWatch) {
      const t = this._stopWatch.elapsed();
      this.durations.push(t), this.elapsedOverall += t, this.invocationCount += 1, this._stopWatch = void 0;
    }
  }
};
qt.all = /* @__PURE__ */ new Set(), qt._idPool = 0;
let Ri = qt, mu = -1;
const oi = class oi {
  constructor(t, n, i = (oi._idPool++).toString(16).padStart(3, "0")) {
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
    const r = this._stacks.get(t.value) || 0;
    if (this._stacks.set(t.value, r + 1), this._warnCountdown -= 1, this._warnCountdown <= 0) {
      this._warnCountdown = i * 0.5;
      const [s, a] = this.getMostFrequentStack(), l = `[${this.name}] potential listener LEAK detected, having ${n} listeners already. MOST frequent listener (${a}):`;
      console.warn(l), console.warn(s);
      const o = new gu(l, s);
      this._errorHandler(o);
    }
    return () => {
      const s = this._stacks.get(t.value) || 0;
      this._stacks.set(t.value, s - 1);
    };
  }
  getMostFrequentStack() {
    if (!this._stacks)
      return;
    let t, n = 0;
    for (const [i, r] of this._stacks)
      (!t || n < r) && (t = [i, r], n = r);
    return t;
  }
};
oi._idPool = 1;
let ki = oi;
class br {
  static create() {
    const t = new Error();
    return new br(t.stack ?? "");
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
class gu extends Error {
  constructor(t, n) {
    super(t), this.name = "ListenerLeakError", this.stack = n;
  }
}
class du extends Error {
  constructor(t, n) {
    super(t), this.name = "ListenerRefusalError", this.stack = n;
  }
}
let pu = 0;
class hi {
  constructor(t) {
    this.value = t, this.id = pu++;
  }
}
const bu = 2;
class He {
  constructor(t) {
    var n, i, r, s;
    this._size = 0, this._options = t, this._leakageMon = (n = this._options) != null && n.leakWarningThreshold ? new ki(
      (t == null ? void 0 : t.onListenerError) ?? In,
      ((i = this._options) == null ? void 0 : i.leakWarningThreshold) ?? mu
    ) : void 0, this._perfMon = (r = this._options) != null && r._profName ? new Ri(this._options._profName) : void 0, this._deliveryQueue = (s = this._options) == null ? void 0 : s.deliveryQueue;
  }
  dispose() {
    var t, n, i, r;
    this._disposed || (this._disposed = !0, ((t = this._deliveryQueue) == null ? void 0 : t.current) === this && this._deliveryQueue.reset(), this._listeners && (this._listeners = void 0, this._size = 0), (i = (n = this._options) == null ? void 0 : n.onDidRemoveLastListener) == null || i.call(n), (r = this._leakageMon) == null || r.dispose());
  }
  get event() {
    return this._event ?? (this._event = (t, n, i) => {
      var l, o, u, h, c;
      if (this._leakageMon && this._size > this._leakageMon.threshold ** 2) {
        const m = `[${this._leakageMon.name}] REFUSES to accept new listeners because it exceeded its threshold by far (${this._size} vs ${this._leakageMon.threshold})`;
        console.warn(m);
        const g = this._leakageMon.getMostFrequentStack() ?? ["UNKNOWN stack", -1], d = new du(
          `${m}. HINT: Stack shows most frequent listener (${g[1]}-times)`,
          g[0]
        );
        return (((l = this._options) == null ? void 0 : l.onListenerError) || In)(d), Xt.None;
      }
      if (this._disposed)
        return Xt.None;
      n && (t = t.bind(n));
      const r = new hi(t);
      let s;
      this._leakageMon && this._size >= Math.ceil(this._leakageMon.threshold * 0.2) && (r.stack = br.create(), s = this._leakageMon.check(r.stack, this._size + 1)), this._listeners ? this._listeners instanceof hi ? (this._deliveryQueue ?? (this._deliveryQueue = new _u()), this._listeners = [this._listeners, r]) : this._listeners.push(r) : ((u = (o = this._options) == null ? void 0 : o.onWillAddFirstListener) == null || u.call(o, this), this._listeners = r, (c = (h = this._options) == null ? void 0 : h.onDidAddFirstListener) == null || c.call(h, this)), this._size++;
      const a = Hn(() => {
        s == null || s(), this._removeListener(r);
      });
      return i instanceof pn ? i.add(a) : Array.isArray(i) && i.push(a), a;
    }), this._event;
  }
  _removeListener(t) {
    var s, a, l, o;
    if ((a = (s = this._options) == null ? void 0 : s.onWillRemoveListener) == null || a.call(s, this), !this._listeners)
      return;
    if (this._size === 1) {
      this._listeners = void 0, (o = (l = this._options) == null ? void 0 : l.onDidRemoveLastListener) == null || o.call(l, this), this._size = 0;
      return;
    }
    const n = this._listeners, i = n.indexOf(t);
    if (i === -1)
      throw console.log("disposed?", this._disposed), console.log("size?", this._size), console.log("arr?", JSON.stringify(this._listeners)), new Error("Attempted to dispose unknown listener");
    this._size--, n[i] = void 0;
    const r = this._deliveryQueue.current === this;
    if (this._size * bu <= n.length) {
      let u = 0;
      for (let h = 0; h < n.length; h++)
        n[h] ? n[u++] = n[h] : r && (this._deliveryQueue.end--, u < this._deliveryQueue.i && this._deliveryQueue.i--);
      n.length = u;
    }
  }
  _deliver(t, n) {
    var r;
    if (!t)
      return;
    const i = ((r = this._options) == null ? void 0 : r.onListenerError) || In;
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
  _deliverQueue(t) {
    const n = t.current._listeners;
    for (; t.i < t.end; )
      this._deliver(n[t.i++], t.value);
    t.reset();
  }
  fire(t) {
    var n, i, r, s;
    if ((n = this._deliveryQueue) != null && n.current && (this._deliverQueue(this._deliveryQueue), (i = this._perfMon) == null || i.stop()), (r = this._perfMon) == null || r.start(this._size), this._listeners) if (this._listeners instanceof hi)
      this._deliver(this._listeners, t);
    else {
      const a = this._deliveryQueue;
      a.enqueue(this, t, this._listeners.length), this._deliverQueue(a);
    }
    (s = this._perfMon) == null || s.stop();
  }
  hasListeners() {
    return this._size > 0;
  }
}
class _u {
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
function vu(e) {
  return typeof e == "string";
}
function Lu(e) {
  let t = [];
  for (; Object.prototype !== e; )
    t = t.concat(Object.getOwnPropertyNames(e)), e = Object.getPrototypeOf(e);
  return t;
}
function Ti(e) {
  const t = [];
  for (const n of Lu(e))
    typeof e[n] == "function" && t.push(n);
  return t;
}
function Nu(e, t) {
  const n = (r) => function() {
    const s = Array.prototype.slice.call(arguments, 0);
    return t(r, s);
  }, i = {};
  for (const r of e)
    i[r] = n(r);
  return i;
}
const Ft = "en";
let bn = !1, _n = !1, Dn = !1, sl = !1, Rn, Un = Ft, Ir = Ft, wu, Ve;
const Nt = globalThis;
let _e;
var Co;
typeof Nt.vscode < "u" && typeof Nt.vscode.process < "u" ? _e = Nt.vscode.process : typeof process < "u" && typeof ((Co = process == null ? void 0 : process.versions) == null ? void 0 : Co.node) == "string" && (_e = process);
var el;
const Au = typeof ((el = _e == null ? void 0 : _e.versions) == null ? void 0 : el.electron) == "string", xu = Au && (_e == null ? void 0 : _e.type) === "renderer";
var tl;
if (typeof _e == "object") {
  bn = _e.platform === "win32", _n = _e.platform === "darwin", Dn = _e.platform === "linux", Dn && _e.env.SNAP && _e.env.SNAP_REVISION, _e.env.CI || _e.env.BUILD_ARTIFACTSTAGINGDIRECTORY, Rn = Ft, Un = Ft;
  const e = _e.env.VSCODE_NLS_CONFIG;
  if (e)
    try {
      const t = JSON.parse(e);
      Rn = t.userLocale, Ir = t.osLocale, Un = t.resolvedLanguage || Ft, wu = (tl = t.languagePack) == null ? void 0 : tl.translationsConfigFile;
    } catch {
    }
} else typeof navigator == "object" && !xu ? (Ve = navigator.userAgent, bn = Ve.indexOf("Windows") >= 0, _n = Ve.indexOf("Macintosh") >= 0, sl = (Ve.indexOf("Macintosh") >= 0 || Ve.indexOf("iPad") >= 0 || Ve.indexOf("iPhone") >= 0) && !!navigator.maxTouchPoints && navigator.maxTouchPoints > 0, Dn = Ve.indexOf("Linux") >= 0, (Ve == null ? void 0 : Ve.indexOf("Mobi")) >= 0, Un = globalThis._VSCODE_NLS_LANGUAGE || Ft, Rn = navigator.language.toLowerCase(), Ir = Rn) : console.error("Unable to resolve platform.");
var $t;
(function(e) {
  e[e.Web = 0] = "Web", e[e.Mac = 1] = "Mac", e[e.Linux = 2] = "Linux", e[e.Windows = 3] = "Windows";
})($t || ($t = {}));
$t.Web;
_n ? $t.Mac : bn ? $t.Windows : Dn && $t.Linux;
const vn = bn, Eu = _n, Qe = Ve, rt = Un;
var Dr;
(function(e) {
  function t() {
    return rt;
  }
  e.value = t;
  function n() {
    return rt.length === 2 ? rt === "en" : rt.length >= 3 ? rt[0] === "e" && rt[1] === "n" && rt[2] === "-" : !1;
  }
  e.isDefaultVariant = n;
  function i() {
    return rt === "en";
  }
  e.isDefault = i;
})(Dr || (Dr = {}));
const yu = typeof Nt.postMessage == "function" && !Nt.importScripts;
(() => {
  if (yu) {
    const e = [];
    Nt.addEventListener("message", (n) => {
      if (n.data && n.data.vscodeScheduleAsyncWork)
        for (let i = 0, r = e.length; i < r; i++) {
          const s = e[i];
          if (s.id === n.data.vscodeScheduleAsyncWork) {
            e.splice(i, 1), s.callback();
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
      }), Nt.postMessage({ vscodeScheduleAsyncWork: i }, "*");
    };
  }
  return (e) => setTimeout(e);
})();
var un;
(function(e) {
  e[e.Windows = 1] = "Windows", e[e.Macintosh = 2] = "Macintosh", e[e.Linux = 3] = "Linux";
})(un || (un = {}));
_n || sl ? un.Macintosh : bn ? un.Windows : un.Linux;
const Ru = !!(Qe && Qe.indexOf("Chrome") >= 0);
Qe && Qe.indexOf("Firefox") >= 0;
!Ru && Qe && Qe.indexOf("Safari") >= 0;
Qe && Qe.indexOf("Edg/") >= 0;
Qe && Qe.indexOf("Android") >= 0;
const al = Object.freeze(function(e, t) {
  const n = setTimeout(e.bind(t), 0);
  return { dispose() {
    clearTimeout(n);
  } };
});
var $n;
(function(e) {
  function t(n) {
    return n === e.None || n === e.Cancelled || n instanceof Fn ? !0 : !n || typeof n != "object" ? !1 : typeof n.isCancellationRequested == "boolean" && typeof n.onCancellationRequested == "function";
  }
  e.isCancellationToken = t, e.None = Object.freeze({
    isCancellationRequested: !1,
    onCancellationRequested: yi.None
  }), e.Cancelled = Object.freeze({
    isCancellationRequested: !0,
    onCancellationRequested: al
  });
})($n || ($n = {}));
class Fn {
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
    return this._isCancelled ? al : (this._emitter || (this._emitter = new He()), this._emitter.event);
  }
  dispose() {
    this._emitter && (this._emitter.dispose(), this._emitter = null);
  }
}
class ku {
  constructor(t) {
    this._token = void 0, this._parentListener = void 0, this._parentListener = t && t.onCancellationRequested(this.cancel, this);
  }
  get token() {
    return this._token || (this._token = new Fn()), this._token;
  }
  cancel() {
    this._token ? this._token instanceof Fn && this._token.cancel() : this._token = $n.Cancelled;
  }
  dispose(t = !1) {
    var n;
    t && this.cancel(), (n = this._parentListener) == null || n.dispose(), this._token ? this._token instanceof Fn && this._token.dispose() : this._token = $n.None;
  }
}
function Tu(e) {
  return e;
}
class Su {
  constructor(t, n) {
    this.lastCache = void 0, this.lastArgKey = void 0, typeof t == "function" ? (this._fn = t, this._computeKey = Tu) : (this._fn = n, this._computeKey = t.getCacheKey);
  }
  get(t) {
    const n = this._computeKey(t);
    return this.lastArgKey !== n && (this.lastArgKey = n, this.lastCache = this._fn(t)), this.lastCache;
  }
}
var S;
(function(e) {
  e[e.Null = 0] = "Null", e[e.Backspace = 8] = "Backspace", e[e.Tab = 9] = "Tab", e[e.LineFeed = 10] = "LineFeed", e[e.CarriageReturn = 13] = "CarriageReturn", e[e.Space = 32] = "Space", e[e.ExclamationMark = 33] = "ExclamationMark", e[e.DoubleQuote = 34] = "DoubleQuote", e[e.Hash = 35] = "Hash", e[e.DollarSign = 36] = "DollarSign", e[e.PercentSign = 37] = "PercentSign", e[e.Ampersand = 38] = "Ampersand", e[e.SingleQuote = 39] = "SingleQuote", e[e.OpenParen = 40] = "OpenParen", e[e.CloseParen = 41] = "CloseParen", e[e.Asterisk = 42] = "Asterisk", e[e.Plus = 43] = "Plus", e[e.Comma = 44] = "Comma", e[e.Dash = 45] = "Dash", e[e.Period = 46] = "Period", e[e.Slash = 47] = "Slash", e[e.Digit0 = 48] = "Digit0", e[e.Digit1 = 49] = "Digit1", e[e.Digit2 = 50] = "Digit2", e[e.Digit3 = 51] = "Digit3", e[e.Digit4 = 52] = "Digit4", e[e.Digit5 = 53] = "Digit5", e[e.Digit6 = 54] = "Digit6", e[e.Digit7 = 55] = "Digit7", e[e.Digit8 = 56] = "Digit8", e[e.Digit9 = 57] = "Digit9", e[e.Colon = 58] = "Colon", e[e.Semicolon = 59] = "Semicolon", e[e.LessThan = 60] = "LessThan", e[e.Equals = 61] = "Equals", e[e.GreaterThan = 62] = "GreaterThan", e[e.QuestionMark = 63] = "QuestionMark", e[e.AtSign = 64] = "AtSign", e[e.A = 65] = "A", e[e.B = 66] = "B", e[e.C = 67] = "C", e[e.D = 68] = "D", e[e.E = 69] = "E", e[e.F = 70] = "F", e[e.G = 71] = "G", e[e.H = 72] = "H", e[e.I = 73] = "I", e[e.J = 74] = "J", e[e.K = 75] = "K", e[e.L = 76] = "L", e[e.M = 77] = "M", e[e.N = 78] = "N", e[e.O = 79] = "O", e[e.P = 80] = "P", e[e.Q = 81] = "Q", e[e.R = 82] = "R", e[e.S = 83] = "S", e[e.T = 84] = "T", e[e.U = 85] = "U", e[e.V = 86] = "V", e[e.W = 87] = "W", e[e.X = 88] = "X", e[e.Y = 89] = "Y", e[e.Z = 90] = "Z", e[e.OpenSquareBracket = 91] = "OpenSquareBracket", e[e.Backslash = 92] = "Backslash", e[e.CloseSquareBracket = 93] = "CloseSquareBracket", e[e.Caret = 94] = "Caret", e[e.Underline = 95] = "Underline", e[e.BackTick = 96] = "BackTick", e[e.a = 97] = "a", e[e.b = 98] = "b", e[e.c = 99] = "c", e[e.d = 100] = "d", e[e.e = 101] = "e", e[e.f = 102] = "f", e[e.g = 103] = "g", e[e.h = 104] = "h", e[e.i = 105] = "i", e[e.j = 106] = "j", e[e.k = 107] = "k", e[e.l = 108] = "l", e[e.m = 109] = "m", e[e.n = 110] = "n", e[e.o = 111] = "o", e[e.p = 112] = "p", e[e.q = 113] = "q", e[e.r = 114] = "r", e[e.s = 115] = "s", e[e.t = 116] = "t", e[e.u = 117] = "u", e[e.v = 118] = "v", e[e.w = 119] = "w", e[e.x = 120] = "x", e[e.y = 121] = "y", e[e.z = 122] = "z", e[e.OpenCurlyBrace = 123] = "OpenCurlyBrace", e[e.Pipe = 124] = "Pipe", e[e.CloseCurlyBrace = 125] = "CloseCurlyBrace", e[e.Tilde = 126] = "Tilde", e[e.NoBreakSpace = 160] = "NoBreakSpace", e[e.U_Combining_Grave_Accent = 768] = "U_Combining_Grave_Accent", e[e.U_Combining_Acute_Accent = 769] = "U_Combining_Acute_Accent", e[e.U_Combining_Circumflex_Accent = 770] = "U_Combining_Circumflex_Accent", e[e.U_Combining_Tilde = 771] = "U_Combining_Tilde", e[e.U_Combining_Macron = 772] = "U_Combining_Macron", e[e.U_Combining_Overline = 773] = "U_Combining_Overline", e[e.U_Combining_Breve = 774] = "U_Combining_Breve", e[e.U_Combining_Dot_Above = 775] = "U_Combining_Dot_Above", e[e.U_Combining_Diaeresis = 776] = "U_Combining_Diaeresis", e[e.U_Combining_Hook_Above = 777] = "U_Combining_Hook_Above", e[e.U_Combining_Ring_Above = 778] = "U_Combining_Ring_Above", e[e.U_Combining_Double_Acute_Accent = 779] = "U_Combining_Double_Acute_Accent", e[e.U_Combining_Caron = 780] = "U_Combining_Caron", e[e.U_Combining_Vertical_Line_Above = 781] = "U_Combining_Vertical_Line_Above", e[e.U_Combining_Double_Vertical_Line_Above = 782] = "U_Combining_Double_Vertical_Line_Above", e[e.U_Combining_Double_Grave_Accent = 783] = "U_Combining_Double_Grave_Accent", e[e.U_Combining_Candrabindu = 784] = "U_Combining_Candrabindu", e[e.U_Combining_Inverted_Breve = 785] = "U_Combining_Inverted_Breve", e[e.U_Combining_Turned_Comma_Above = 786] = "U_Combining_Turned_Comma_Above", e[e.U_Combining_Comma_Above = 787] = "U_Combining_Comma_Above", e[e.U_Combining_Reversed_Comma_Above = 788] = "U_Combining_Reversed_Comma_Above", e[e.U_Combining_Comma_Above_Right = 789] = "U_Combining_Comma_Above_Right", e[e.U_Combining_Grave_Accent_Below = 790] = "U_Combining_Grave_Accent_Below", e[e.U_Combining_Acute_Accent_Below = 791] = "U_Combining_Acute_Accent_Below", e[e.U_Combining_Left_Tack_Below = 792] = "U_Combining_Left_Tack_Below", e[e.U_Combining_Right_Tack_Below = 793] = "U_Combining_Right_Tack_Below", e[e.U_Combining_Left_Angle_Above = 794] = "U_Combining_Left_Angle_Above", e[e.U_Combining_Horn = 795] = "U_Combining_Horn", e[e.U_Combining_Left_Half_Ring_Below = 796] = "U_Combining_Left_Half_Ring_Below", e[e.U_Combining_Up_Tack_Below = 797] = "U_Combining_Up_Tack_Below", e[e.U_Combining_Down_Tack_Below = 798] = "U_Combining_Down_Tack_Below", e[e.U_Combining_Plus_Sign_Below = 799] = "U_Combining_Plus_Sign_Below", e[e.U_Combining_Minus_Sign_Below = 800] = "U_Combining_Minus_Sign_Below", e[e.U_Combining_Palatalized_Hook_Below = 801] = "U_Combining_Palatalized_Hook_Below", e[e.U_Combining_Retroflex_Hook_Below = 802] = "U_Combining_Retroflex_Hook_Below", e[e.U_Combining_Dot_Below = 803] = "U_Combining_Dot_Below", e[e.U_Combining_Diaeresis_Below = 804] = "U_Combining_Diaeresis_Below", e[e.U_Combining_Ring_Below = 805] = "U_Combining_Ring_Below", e[e.U_Combining_Comma_Below = 806] = "U_Combining_Comma_Below", e[e.U_Combining_Cedilla = 807] = "U_Combining_Cedilla", e[e.U_Combining_Ogonek = 808] = "U_Combining_Ogonek", e[e.U_Combining_Vertical_Line_Below = 809] = "U_Combining_Vertical_Line_Below", e[e.U_Combining_Bridge_Below = 810] = "U_Combining_Bridge_Below", e[e.U_Combining_Inverted_Double_Arch_Below = 811] = "U_Combining_Inverted_Double_Arch_Below", e[e.U_Combining_Caron_Below = 812] = "U_Combining_Caron_Below", e[e.U_Combining_Circumflex_Accent_Below = 813] = "U_Combining_Circumflex_Accent_Below", e[e.U_Combining_Breve_Below = 814] = "U_Combining_Breve_Below", e[e.U_Combining_Inverted_Breve_Below = 815] = "U_Combining_Inverted_Breve_Below", e[e.U_Combining_Tilde_Below = 816] = "U_Combining_Tilde_Below", e[e.U_Combining_Macron_Below = 817] = "U_Combining_Macron_Below", e[e.U_Combining_Low_Line = 818] = "U_Combining_Low_Line", e[e.U_Combining_Double_Low_Line = 819] = "U_Combining_Double_Low_Line", e[e.U_Combining_Tilde_Overlay = 820] = "U_Combining_Tilde_Overlay", e[e.U_Combining_Short_Stroke_Overlay = 821] = "U_Combining_Short_Stroke_Overlay", e[e.U_Combining_Long_Stroke_Overlay = 822] = "U_Combining_Long_Stroke_Overlay", e[e.U_Combining_Short_Solidus_Overlay = 823] = "U_Combining_Short_Solidus_Overlay", e[e.U_Combining_Long_Solidus_Overlay = 824] = "U_Combining_Long_Solidus_Overlay", e[e.U_Combining_Right_Half_Ring_Below = 825] = "U_Combining_Right_Half_Ring_Below", e[e.U_Combining_Inverted_Bridge_Below = 826] = "U_Combining_Inverted_Bridge_Below", e[e.U_Combining_Square_Below = 827] = "U_Combining_Square_Below", e[e.U_Combining_Seagull_Below = 828] = "U_Combining_Seagull_Below", e[e.U_Combining_X_Above = 829] = "U_Combining_X_Above", e[e.U_Combining_Vertical_Tilde = 830] = "U_Combining_Vertical_Tilde", e[e.U_Combining_Double_Overline = 831] = "U_Combining_Double_Overline", e[e.U_Combining_Grave_Tone_Mark = 832] = "U_Combining_Grave_Tone_Mark", e[e.U_Combining_Acute_Tone_Mark = 833] = "U_Combining_Acute_Tone_Mark", e[e.U_Combining_Greek_Perispomeni = 834] = "U_Combining_Greek_Perispomeni", e[e.U_Combining_Greek_Koronis = 835] = "U_Combining_Greek_Koronis", e[e.U_Combining_Greek_Dialytika_Tonos = 836] = "U_Combining_Greek_Dialytika_Tonos", e[e.U_Combining_Greek_Ypogegrammeni = 837] = "U_Combining_Greek_Ypogegrammeni", e[e.U_Combining_Bridge_Above = 838] = "U_Combining_Bridge_Above", e[e.U_Combining_Equals_Sign_Below = 839] = "U_Combining_Equals_Sign_Below", e[e.U_Combining_Double_Vertical_Line_Below = 840] = "U_Combining_Double_Vertical_Line_Below", e[e.U_Combining_Left_Angle_Below = 841] = "U_Combining_Left_Angle_Below", e[e.U_Combining_Not_Tilde_Above = 842] = "U_Combining_Not_Tilde_Above", e[e.U_Combining_Homothetic_Above = 843] = "U_Combining_Homothetic_Above", e[e.U_Combining_Almost_Equal_To_Above = 844] = "U_Combining_Almost_Equal_To_Above", e[e.U_Combining_Left_Right_Arrow_Below = 845] = "U_Combining_Left_Right_Arrow_Below", e[e.U_Combining_Upwards_Arrow_Below = 846] = "U_Combining_Upwards_Arrow_Below", e[e.U_Combining_Grapheme_Joiner = 847] = "U_Combining_Grapheme_Joiner", e[e.U_Combining_Right_Arrowhead_Above = 848] = "U_Combining_Right_Arrowhead_Above", e[e.U_Combining_Left_Half_Ring_Above = 849] = "U_Combining_Left_Half_Ring_Above", e[e.U_Combining_Fermata = 850] = "U_Combining_Fermata", e[e.U_Combining_X_Below = 851] = "U_Combining_X_Below", e[e.U_Combining_Left_Arrowhead_Below = 852] = "U_Combining_Left_Arrowhead_Below", e[e.U_Combining_Right_Arrowhead_Below = 853] = "U_Combining_Right_Arrowhead_Below", e[e.U_Combining_Right_Arrowhead_And_Up_Arrowhead_Below = 854] = "U_Combining_Right_Arrowhead_And_Up_Arrowhead_Below", e[e.U_Combining_Right_Half_Ring_Above = 855] = "U_Combining_Right_Half_Ring_Above", e[e.U_Combining_Dot_Above_Right = 856] = "U_Combining_Dot_Above_Right", e[e.U_Combining_Asterisk_Below = 857] = "U_Combining_Asterisk_Below", e[e.U_Combining_Double_Ring_Below = 858] = "U_Combining_Double_Ring_Below", e[e.U_Combining_Zigzag_Above = 859] = "U_Combining_Zigzag_Above", e[e.U_Combining_Double_Breve_Below = 860] = "U_Combining_Double_Breve_Below", e[e.U_Combining_Double_Breve = 861] = "U_Combining_Double_Breve", e[e.U_Combining_Double_Macron = 862] = "U_Combining_Double_Macron", e[e.U_Combining_Double_Macron_Below = 863] = "U_Combining_Double_Macron_Below", e[e.U_Combining_Double_Tilde = 864] = "U_Combining_Double_Tilde", e[e.U_Combining_Double_Inverted_Breve = 865] = "U_Combining_Double_Inverted_Breve", e[e.U_Combining_Double_Rightwards_Arrow_Below = 866] = "U_Combining_Double_Rightwards_Arrow_Below", e[e.U_Combining_Latin_Small_Letter_A = 867] = "U_Combining_Latin_Small_Letter_A", e[e.U_Combining_Latin_Small_Letter_E = 868] = "U_Combining_Latin_Small_Letter_E", e[e.U_Combining_Latin_Small_Letter_I = 869] = "U_Combining_Latin_Small_Letter_I", e[e.U_Combining_Latin_Small_Letter_O = 870] = "U_Combining_Latin_Small_Letter_O", e[e.U_Combining_Latin_Small_Letter_U = 871] = "U_Combining_Latin_Small_Letter_U", e[e.U_Combining_Latin_Small_Letter_C = 872] = "U_Combining_Latin_Small_Letter_C", e[e.U_Combining_Latin_Small_Letter_D = 873] = "U_Combining_Latin_Small_Letter_D", e[e.U_Combining_Latin_Small_Letter_H = 874] = "U_Combining_Latin_Small_Letter_H", e[e.U_Combining_Latin_Small_Letter_M = 875] = "U_Combining_Latin_Small_Letter_M", e[e.U_Combining_Latin_Small_Letter_R = 876] = "U_Combining_Latin_Small_Letter_R", e[e.U_Combining_Latin_Small_Letter_T = 877] = "U_Combining_Latin_Small_Letter_T", e[e.U_Combining_Latin_Small_Letter_V = 878] = "U_Combining_Latin_Small_Letter_V", e[e.U_Combining_Latin_Small_Letter_X = 879] = "U_Combining_Latin_Small_Letter_X", e[e.LINE_SEPARATOR = 8232] = "LINE_SEPARATOR", e[e.PARAGRAPH_SEPARATOR = 8233] = "PARAGRAPH_SEPARATOR", e[e.NEXT_LINE = 133] = "NEXT_LINE", e[e.U_CIRCUMFLEX = 94] = "U_CIRCUMFLEX", e[e.U_GRAVE_ACCENT = 96] = "U_GRAVE_ACCENT", e[e.U_DIAERESIS = 168] = "U_DIAERESIS", e[e.U_MACRON = 175] = "U_MACRON", e[e.U_ACUTE_ACCENT = 180] = "U_ACUTE_ACCENT", e[e.U_CEDILLA = 184] = "U_CEDILLA", e[e.U_MODIFIER_LETTER_LEFT_ARROWHEAD = 706] = "U_MODIFIER_LETTER_LEFT_ARROWHEAD", e[e.U_MODIFIER_LETTER_RIGHT_ARROWHEAD = 707] = "U_MODIFIER_LETTER_RIGHT_ARROWHEAD", e[e.U_MODIFIER_LETTER_UP_ARROWHEAD = 708] = "U_MODIFIER_LETTER_UP_ARROWHEAD", e[e.U_MODIFIER_LETTER_DOWN_ARROWHEAD = 709] = "U_MODIFIER_LETTER_DOWN_ARROWHEAD", e[e.U_MODIFIER_LETTER_CENTRED_RIGHT_HALF_RING = 722] = "U_MODIFIER_LETTER_CENTRED_RIGHT_HALF_RING", e[e.U_MODIFIER_LETTER_CENTRED_LEFT_HALF_RING = 723] = "U_MODIFIER_LETTER_CENTRED_LEFT_HALF_RING", e[e.U_MODIFIER_LETTER_UP_TACK = 724] = "U_MODIFIER_LETTER_UP_TACK", e[e.U_MODIFIER_LETTER_DOWN_TACK = 725] = "U_MODIFIER_LETTER_DOWN_TACK", e[e.U_MODIFIER_LETTER_PLUS_SIGN = 726] = "U_MODIFIER_LETTER_PLUS_SIGN", e[e.U_MODIFIER_LETTER_MINUS_SIGN = 727] = "U_MODIFIER_LETTER_MINUS_SIGN", e[e.U_BREVE = 728] = "U_BREVE", e[e.U_DOT_ABOVE = 729] = "U_DOT_ABOVE", e[e.U_RING_ABOVE = 730] = "U_RING_ABOVE", e[e.U_OGONEK = 731] = "U_OGONEK", e[e.U_SMALL_TILDE = 732] = "U_SMALL_TILDE", e[e.U_DOUBLE_ACUTE_ACCENT = 733] = "U_DOUBLE_ACUTE_ACCENT", e[e.U_MODIFIER_LETTER_RHOTIC_HOOK = 734] = "U_MODIFIER_LETTER_RHOTIC_HOOK", e[e.U_MODIFIER_LETTER_CROSS_ACCENT = 735] = "U_MODIFIER_LETTER_CROSS_ACCENT", e[e.U_MODIFIER_LETTER_EXTRA_HIGH_TONE_BAR = 741] = "U_MODIFIER_LETTER_EXTRA_HIGH_TONE_BAR", e[e.U_MODIFIER_LETTER_HIGH_TONE_BAR = 742] = "U_MODIFIER_LETTER_HIGH_TONE_BAR", e[e.U_MODIFIER_LETTER_MID_TONE_BAR = 743] = "U_MODIFIER_LETTER_MID_TONE_BAR", e[e.U_MODIFIER_LETTER_LOW_TONE_BAR = 744] = "U_MODIFIER_LETTER_LOW_TONE_BAR", e[e.U_MODIFIER_LETTER_EXTRA_LOW_TONE_BAR = 745] = "U_MODIFIER_LETTER_EXTRA_LOW_TONE_BAR", e[e.U_MODIFIER_LETTER_YIN_DEPARTING_TONE_MARK = 746] = "U_MODIFIER_LETTER_YIN_DEPARTING_TONE_MARK", e[e.U_MODIFIER_LETTER_YANG_DEPARTING_TONE_MARK = 747] = "U_MODIFIER_LETTER_YANG_DEPARTING_TONE_MARK", e[e.U_MODIFIER_LETTER_UNASPIRATED = 749] = "U_MODIFIER_LETTER_UNASPIRATED", e[e.U_MODIFIER_LETTER_LOW_DOWN_ARROWHEAD = 751] = "U_MODIFIER_LETTER_LOW_DOWN_ARROWHEAD", e[e.U_MODIFIER_LETTER_LOW_UP_ARROWHEAD = 752] = "U_MODIFIER_LETTER_LOW_UP_ARROWHEAD", e[e.U_MODIFIER_LETTER_LOW_LEFT_ARROWHEAD = 753] = "U_MODIFIER_LETTER_LOW_LEFT_ARROWHEAD", e[e.U_MODIFIER_LETTER_LOW_RIGHT_ARROWHEAD = 754] = "U_MODIFIER_LETTER_LOW_RIGHT_ARROWHEAD", e[e.U_MODIFIER_LETTER_LOW_RING = 755] = "U_MODIFIER_LETTER_LOW_RING", e[e.U_MODIFIER_LETTER_MIDDLE_GRAVE_ACCENT = 756] = "U_MODIFIER_LETTER_MIDDLE_GRAVE_ACCENT", e[e.U_MODIFIER_LETTER_MIDDLE_DOUBLE_GRAVE_ACCENT = 757] = "U_MODIFIER_LETTER_MIDDLE_DOUBLE_GRAVE_ACCENT", e[e.U_MODIFIER_LETTER_MIDDLE_DOUBLE_ACUTE_ACCENT = 758] = "U_MODIFIER_LETTER_MIDDLE_DOUBLE_ACUTE_ACCENT", e[e.U_MODIFIER_LETTER_LOW_TILDE = 759] = "U_MODIFIER_LETTER_LOW_TILDE", e[e.U_MODIFIER_LETTER_RAISED_COLON = 760] = "U_MODIFIER_LETTER_RAISED_COLON", e[e.U_MODIFIER_LETTER_BEGIN_HIGH_TONE = 761] = "U_MODIFIER_LETTER_BEGIN_HIGH_TONE", e[e.U_MODIFIER_LETTER_END_HIGH_TONE = 762] = "U_MODIFIER_LETTER_END_HIGH_TONE", e[e.U_MODIFIER_LETTER_BEGIN_LOW_TONE = 763] = "U_MODIFIER_LETTER_BEGIN_LOW_TONE", e[e.U_MODIFIER_LETTER_END_LOW_TONE = 764] = "U_MODIFIER_LETTER_END_LOW_TONE", e[e.U_MODIFIER_LETTER_SHELF = 765] = "U_MODIFIER_LETTER_SHELF", e[e.U_MODIFIER_LETTER_OPEN_SHELF = 766] = "U_MODIFIER_LETTER_OPEN_SHELF", e[e.U_MODIFIER_LETTER_LOW_LEFT_ARROW = 767] = "U_MODIFIER_LETTER_LOW_LEFT_ARROW", e[e.U_GREEK_LOWER_NUMERAL_SIGN = 885] = "U_GREEK_LOWER_NUMERAL_SIGN", e[e.U_GREEK_TONOS = 900] = "U_GREEK_TONOS", e[e.U_GREEK_DIALYTIKA_TONOS = 901] = "U_GREEK_DIALYTIKA_TONOS", e[e.U_GREEK_KORONIS = 8125] = "U_GREEK_KORONIS", e[e.U_GREEK_PSILI = 8127] = "U_GREEK_PSILI", e[e.U_GREEK_PERISPOMENI = 8128] = "U_GREEK_PERISPOMENI", e[e.U_GREEK_DIALYTIKA_AND_PERISPOMENI = 8129] = "U_GREEK_DIALYTIKA_AND_PERISPOMENI", e[e.U_GREEK_PSILI_AND_VARIA = 8141] = "U_GREEK_PSILI_AND_VARIA", e[e.U_GREEK_PSILI_AND_OXIA = 8142] = "U_GREEK_PSILI_AND_OXIA", e[e.U_GREEK_PSILI_AND_PERISPOMENI = 8143] = "U_GREEK_PSILI_AND_PERISPOMENI", e[e.U_GREEK_DASIA_AND_VARIA = 8157] = "U_GREEK_DASIA_AND_VARIA", e[e.U_GREEK_DASIA_AND_OXIA = 8158] = "U_GREEK_DASIA_AND_OXIA", e[e.U_GREEK_DASIA_AND_PERISPOMENI = 8159] = "U_GREEK_DASIA_AND_PERISPOMENI", e[e.U_GREEK_DIALYTIKA_AND_VARIA = 8173] = "U_GREEK_DIALYTIKA_AND_VARIA", e[e.U_GREEK_DIALYTIKA_AND_OXIA = 8174] = "U_GREEK_DIALYTIKA_AND_OXIA", e[e.U_GREEK_VARIA = 8175] = "U_GREEK_VARIA", e[e.U_GREEK_OXIA = 8189] = "U_GREEK_OXIA", e[e.U_GREEK_DASIA = 8190] = "U_GREEK_DASIA", e[e.U_IDEOGRAPHIC_FULL_STOP = 12290] = "U_IDEOGRAPHIC_FULL_STOP", e[e.U_LEFT_CORNER_BRACKET = 12300] = "U_LEFT_CORNER_BRACKET", e[e.U_RIGHT_CORNER_BRACKET = 12301] = "U_RIGHT_CORNER_BRACKET", e[e.U_LEFT_BLACK_LENTICULAR_BRACKET = 12304] = "U_LEFT_BLACK_LENTICULAR_BRACKET", e[e.U_RIGHT_BLACK_LENTICULAR_BRACKET = 12305] = "U_RIGHT_BLACK_LENTICULAR_BRACKET", e[e.U_OVERLINE = 8254] = "U_OVERLINE", e[e.UTF8_BOM = 65279] = "UTF8_BOM", e[e.U_FULLWIDTH_SEMICOLON = 65307] = "U_FULLWIDTH_SEMICOLON", e[e.U_FULLWIDTH_COMMA = 65292] = "U_FULLWIDTH_COMMA";
})(S || (S = {}));
class Ur {
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
var Pe;
(function(e) {
  e[e.MAX_SAFE_SMALL_INTEGER = 1073741824] = "MAX_SAFE_SMALL_INTEGER", e[e.MIN_SAFE_SMALL_INTEGER = -1073741824] = "MIN_SAFE_SMALL_INTEGER", e[e.MAX_UINT_8 = 255] = "MAX_UINT_8", e[e.MAX_UINT_16 = 65535] = "MAX_UINT_16", e[e.MAX_UINT_32 = 4294967295] = "MAX_UINT_32", e[e.UNICODE_SUPPLEMENTARY_PLANE_BEGIN = 65536] = "UNICODE_SUPPLEMENTARY_PLANE_BEGIN";
})(Pe || (Pe = {}));
function Fr(e) {
  return e < 0 ? 0 : e > Pe.MAX_UINT_8 ? Pe.MAX_UINT_8 : e | 0;
}
function Rt(e) {
  return e < 0 ? 0 : e > Pe.MAX_UINT_32 ? Pe.MAX_UINT_32 : e | 0;
}
function Mu(e) {
  return e.replace(/[\\\{\}\*\+\?\|\^\$\.\[\]\(\)]/g, "\\$&");
}
function Iu(e) {
  return e.split(/\r\n|\r|\n/);
}
function Du(e) {
  for (let t = 0, n = e.length; t < n; t++) {
    const i = e.charCodeAt(t);
    if (i !== S.Space && i !== S.Tab)
      return t;
  }
  return -1;
}
function Uu(e, t = e.length - 1) {
  for (let n = t; n >= 0; n--) {
    const i = e.charCodeAt(n);
    if (i !== S.Space && i !== S.Tab)
      return n;
  }
  return -1;
}
function ol(e) {
  return e >= S.A && e <= S.Z;
}
function Wn(e) {
  return 55296 <= e && e <= 56319;
}
function Si(e) {
  return 56320 <= e && e <= 57343;
}
function ll(e, t) {
  return (e - 55296 << 10) + (t - 56320) + 65536;
}
function Fu(e, t, n) {
  const i = e.charCodeAt(n);
  if (Wn(i) && n + 1 < t) {
    const r = e.charCodeAt(n + 1);
    if (Si(r))
      return ll(i, r);
  }
  return i;
}
const Pu = /^[\t\n\r\x20-\x7E]*$/;
function Ou(e) {
  return Pu.test(e);
}
String.fromCharCode(S.UTF8_BOM);
var Pr;
(function(e) {
  e[e.Other = 0] = "Other", e[e.Prepend = 1] = "Prepend", e[e.CR = 2] = "CR", e[e.LF = 3] = "LF", e[e.Control = 4] = "Control", e[e.Extend = 5] = "Extend", e[e.Regional_Indicator = 6] = "Regional_Indicator", e[e.SpacingMark = 7] = "SpacingMark", e[e.L = 8] = "L", e[e.V = 9] = "V", e[e.T = 10] = "T", e[e.LV = 11] = "LV", e[e.LVT = 12] = "LVT", e[e.ZWJ = 13] = "ZWJ", e[e.Extended_Pictographic = 14] = "Extended_Pictographic";
})(Pr || (Pr = {}));
var Or;
(function(e) {
  e[e.zwj = 8205] = "zwj", e[e.emojiVariantSelector = 65039] = "emojiVariantSelector", e[e.enclosingKeyCap = 8419] = "enclosingKeyCap";
})(Or || (Or = {}));
const Xe = class Xe {
  static getInstance(t) {
    return Xe.cache.get(Array.from(t));
  }
  static getLocales() {
    return Xe._locales.value;
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
Xe.ambiguousCharacterData = new Ur(() => JSON.parse('{"_common":[8232,32,8233,32,5760,32,8192,32,8193,32,8194,32,8195,32,8196,32,8197,32,8198,32,8200,32,8201,32,8202,32,8287,32,8199,32,8239,32,2042,95,65101,95,65102,95,65103,95,8208,45,8209,45,8210,45,65112,45,1748,45,8259,45,727,45,8722,45,10134,45,11450,45,1549,44,1643,44,8218,44,184,44,42233,44,894,59,2307,58,2691,58,1417,58,1795,58,1796,58,5868,58,65072,58,6147,58,6153,58,8282,58,1475,58,760,58,42889,58,8758,58,720,58,42237,58,451,33,11601,33,660,63,577,63,2429,63,5038,63,42731,63,119149,46,8228,46,1793,46,1794,46,42510,46,68176,46,1632,46,1776,46,42232,46,1373,96,65287,96,8219,96,8242,96,1370,96,1523,96,8175,96,65344,96,900,96,8189,96,8125,96,8127,96,8190,96,697,96,884,96,712,96,714,96,715,96,756,96,699,96,701,96,700,96,702,96,42892,96,1497,96,2036,96,2037,96,5194,96,5836,96,94033,96,94034,96,65339,91,10088,40,10098,40,12308,40,64830,40,65341,93,10089,41,10099,41,12309,41,64831,41,10100,123,119060,123,10101,125,65342,94,8270,42,1645,42,8727,42,66335,42,5941,47,8257,47,8725,47,8260,47,9585,47,10187,47,10744,47,119354,47,12755,47,12339,47,11462,47,20031,47,12035,47,65340,92,65128,92,8726,92,10189,92,10741,92,10745,92,119311,92,119355,92,12756,92,20022,92,12034,92,42872,38,708,94,710,94,5869,43,10133,43,66203,43,8249,60,10094,60,706,60,119350,60,5176,60,5810,60,5120,61,11840,61,12448,61,42239,61,8250,62,10095,62,707,62,119351,62,5171,62,94015,62,8275,126,732,126,8128,126,8764,126,65372,124,65293,45,120784,50,120794,50,120804,50,120814,50,120824,50,130034,50,42842,50,423,50,1000,50,42564,50,5311,50,42735,50,119302,51,120785,51,120795,51,120805,51,120815,51,120825,51,130035,51,42923,51,540,51,439,51,42858,51,11468,51,1248,51,94011,51,71882,51,120786,52,120796,52,120806,52,120816,52,120826,52,130036,52,5070,52,71855,52,120787,53,120797,53,120807,53,120817,53,120827,53,130037,53,444,53,71867,53,120788,54,120798,54,120808,54,120818,54,120828,54,130038,54,11474,54,5102,54,71893,54,119314,55,120789,55,120799,55,120809,55,120819,55,120829,55,130039,55,66770,55,71878,55,2819,56,2538,56,2666,56,125131,56,120790,56,120800,56,120810,56,120820,56,120830,56,130040,56,547,56,546,56,66330,56,2663,57,2920,57,2541,57,3437,57,120791,57,120801,57,120811,57,120821,57,120831,57,130041,57,42862,57,11466,57,71884,57,71852,57,71894,57,9082,97,65345,97,119834,97,119886,97,119938,97,119990,97,120042,97,120094,97,120146,97,120198,97,120250,97,120302,97,120354,97,120406,97,120458,97,593,97,945,97,120514,97,120572,97,120630,97,120688,97,120746,97,65313,65,119808,65,119860,65,119912,65,119964,65,120016,65,120068,65,120120,65,120172,65,120224,65,120276,65,120328,65,120380,65,120432,65,913,65,120488,65,120546,65,120604,65,120662,65,120720,65,5034,65,5573,65,42222,65,94016,65,66208,65,119835,98,119887,98,119939,98,119991,98,120043,98,120095,98,120147,98,120199,98,120251,98,120303,98,120355,98,120407,98,120459,98,388,98,5071,98,5234,98,5551,98,65314,66,8492,66,119809,66,119861,66,119913,66,120017,66,120069,66,120121,66,120173,66,120225,66,120277,66,120329,66,120381,66,120433,66,42932,66,914,66,120489,66,120547,66,120605,66,120663,66,120721,66,5108,66,5623,66,42192,66,66178,66,66209,66,66305,66,65347,99,8573,99,119836,99,119888,99,119940,99,119992,99,120044,99,120096,99,120148,99,120200,99,120252,99,120304,99,120356,99,120408,99,120460,99,7428,99,1010,99,11429,99,43951,99,66621,99,128844,67,71922,67,71913,67,65315,67,8557,67,8450,67,8493,67,119810,67,119862,67,119914,67,119966,67,120018,67,120174,67,120226,67,120278,67,120330,67,120382,67,120434,67,1017,67,11428,67,5087,67,42202,67,66210,67,66306,67,66581,67,66844,67,8574,100,8518,100,119837,100,119889,100,119941,100,119993,100,120045,100,120097,100,120149,100,120201,100,120253,100,120305,100,120357,100,120409,100,120461,100,1281,100,5095,100,5231,100,42194,100,8558,68,8517,68,119811,68,119863,68,119915,68,119967,68,120019,68,120071,68,120123,68,120175,68,120227,68,120279,68,120331,68,120383,68,120435,68,5024,68,5598,68,5610,68,42195,68,8494,101,65349,101,8495,101,8519,101,119838,101,119890,101,119942,101,120046,101,120098,101,120150,101,120202,101,120254,101,120306,101,120358,101,120410,101,120462,101,43826,101,1213,101,8959,69,65317,69,8496,69,119812,69,119864,69,119916,69,120020,69,120072,69,120124,69,120176,69,120228,69,120280,69,120332,69,120384,69,120436,69,917,69,120492,69,120550,69,120608,69,120666,69,120724,69,11577,69,5036,69,42224,69,71846,69,71854,69,66182,69,119839,102,119891,102,119943,102,119995,102,120047,102,120099,102,120151,102,120203,102,120255,102,120307,102,120359,102,120411,102,120463,102,43829,102,42905,102,383,102,7837,102,1412,102,119315,70,8497,70,119813,70,119865,70,119917,70,120021,70,120073,70,120125,70,120177,70,120229,70,120281,70,120333,70,120385,70,120437,70,42904,70,988,70,120778,70,5556,70,42205,70,71874,70,71842,70,66183,70,66213,70,66853,70,65351,103,8458,103,119840,103,119892,103,119944,103,120048,103,120100,103,120152,103,120204,103,120256,103,120308,103,120360,103,120412,103,120464,103,609,103,7555,103,397,103,1409,103,119814,71,119866,71,119918,71,119970,71,120022,71,120074,71,120126,71,120178,71,120230,71,120282,71,120334,71,120386,71,120438,71,1292,71,5056,71,5107,71,42198,71,65352,104,8462,104,119841,104,119945,104,119997,104,120049,104,120101,104,120153,104,120205,104,120257,104,120309,104,120361,104,120413,104,120465,104,1211,104,1392,104,5058,104,65320,72,8459,72,8460,72,8461,72,119815,72,119867,72,119919,72,120023,72,120179,72,120231,72,120283,72,120335,72,120387,72,120439,72,919,72,120494,72,120552,72,120610,72,120668,72,120726,72,11406,72,5051,72,5500,72,42215,72,66255,72,731,105,9075,105,65353,105,8560,105,8505,105,8520,105,119842,105,119894,105,119946,105,119998,105,120050,105,120102,105,120154,105,120206,105,120258,105,120310,105,120362,105,120414,105,120466,105,120484,105,618,105,617,105,953,105,8126,105,890,105,120522,105,120580,105,120638,105,120696,105,120754,105,1110,105,42567,105,1231,105,43893,105,5029,105,71875,105,65354,106,8521,106,119843,106,119895,106,119947,106,119999,106,120051,106,120103,106,120155,106,120207,106,120259,106,120311,106,120363,106,120415,106,120467,106,1011,106,1112,106,65322,74,119817,74,119869,74,119921,74,119973,74,120025,74,120077,74,120129,74,120181,74,120233,74,120285,74,120337,74,120389,74,120441,74,42930,74,895,74,1032,74,5035,74,5261,74,42201,74,119844,107,119896,107,119948,107,120000,107,120052,107,120104,107,120156,107,120208,107,120260,107,120312,107,120364,107,120416,107,120468,107,8490,75,65323,75,119818,75,119870,75,119922,75,119974,75,120026,75,120078,75,120130,75,120182,75,120234,75,120286,75,120338,75,120390,75,120442,75,922,75,120497,75,120555,75,120613,75,120671,75,120729,75,11412,75,5094,75,5845,75,42199,75,66840,75,1472,108,8739,73,9213,73,65512,73,1633,108,1777,73,66336,108,125127,108,120783,73,120793,73,120803,73,120813,73,120823,73,130033,73,65321,73,8544,73,8464,73,8465,73,119816,73,119868,73,119920,73,120024,73,120128,73,120180,73,120232,73,120284,73,120336,73,120388,73,120440,73,65356,108,8572,73,8467,108,119845,108,119897,108,119949,108,120001,108,120053,108,120105,73,120157,73,120209,73,120261,73,120313,73,120365,73,120417,73,120469,73,448,73,120496,73,120554,73,120612,73,120670,73,120728,73,11410,73,1030,73,1216,73,1493,108,1503,108,1575,108,126464,108,126592,108,65166,108,65165,108,1994,108,11599,73,5825,73,42226,73,93992,73,66186,124,66313,124,119338,76,8556,76,8466,76,119819,76,119871,76,119923,76,120027,76,120079,76,120131,76,120183,76,120235,76,120287,76,120339,76,120391,76,120443,76,11472,76,5086,76,5290,76,42209,76,93974,76,71843,76,71858,76,66587,76,66854,76,65325,77,8559,77,8499,77,119820,77,119872,77,119924,77,120028,77,120080,77,120132,77,120184,77,120236,77,120288,77,120340,77,120392,77,120444,77,924,77,120499,77,120557,77,120615,77,120673,77,120731,77,1018,77,11416,77,5047,77,5616,77,5846,77,42207,77,66224,77,66321,77,119847,110,119899,110,119951,110,120003,110,120055,110,120107,110,120159,110,120211,110,120263,110,120315,110,120367,110,120419,110,120471,110,1400,110,1404,110,65326,78,8469,78,119821,78,119873,78,119925,78,119977,78,120029,78,120081,78,120185,78,120237,78,120289,78,120341,78,120393,78,120445,78,925,78,120500,78,120558,78,120616,78,120674,78,120732,78,11418,78,42208,78,66835,78,3074,111,3202,111,3330,111,3458,111,2406,111,2662,111,2790,111,3046,111,3174,111,3302,111,3430,111,3664,111,3792,111,4160,111,1637,111,1781,111,65359,111,8500,111,119848,111,119900,111,119952,111,120056,111,120108,111,120160,111,120212,111,120264,111,120316,111,120368,111,120420,111,120472,111,7439,111,7441,111,43837,111,959,111,120528,111,120586,111,120644,111,120702,111,120760,111,963,111,120532,111,120590,111,120648,111,120706,111,120764,111,11423,111,4351,111,1413,111,1505,111,1607,111,126500,111,126564,111,126596,111,65259,111,65260,111,65258,111,65257,111,1726,111,64428,111,64429,111,64427,111,64426,111,1729,111,64424,111,64425,111,64423,111,64422,111,1749,111,3360,111,4125,111,66794,111,71880,111,71895,111,66604,111,1984,79,2534,79,2918,79,12295,79,70864,79,71904,79,120782,79,120792,79,120802,79,120812,79,120822,79,130032,79,65327,79,119822,79,119874,79,119926,79,119978,79,120030,79,120082,79,120134,79,120186,79,120238,79,120290,79,120342,79,120394,79,120446,79,927,79,120502,79,120560,79,120618,79,120676,79,120734,79,11422,79,1365,79,11604,79,4816,79,2848,79,66754,79,42227,79,71861,79,66194,79,66219,79,66564,79,66838,79,9076,112,65360,112,119849,112,119901,112,119953,112,120005,112,120057,112,120109,112,120161,112,120213,112,120265,112,120317,112,120369,112,120421,112,120473,112,961,112,120530,112,120544,112,120588,112,120602,112,120646,112,120660,112,120704,112,120718,112,120762,112,120776,112,11427,112,65328,80,8473,80,119823,80,119875,80,119927,80,119979,80,120031,80,120083,80,120187,80,120239,80,120291,80,120343,80,120395,80,120447,80,929,80,120504,80,120562,80,120620,80,120678,80,120736,80,11426,80,5090,80,5229,80,42193,80,66197,80,119850,113,119902,113,119954,113,120006,113,120058,113,120110,113,120162,113,120214,113,120266,113,120318,113,120370,113,120422,113,120474,113,1307,113,1379,113,1382,113,8474,81,119824,81,119876,81,119928,81,119980,81,120032,81,120084,81,120188,81,120240,81,120292,81,120344,81,120396,81,120448,81,11605,81,119851,114,119903,114,119955,114,120007,114,120059,114,120111,114,120163,114,120215,114,120267,114,120319,114,120371,114,120423,114,120475,114,43847,114,43848,114,7462,114,11397,114,43905,114,119318,82,8475,82,8476,82,8477,82,119825,82,119877,82,119929,82,120033,82,120189,82,120241,82,120293,82,120345,82,120397,82,120449,82,422,82,5025,82,5074,82,66740,82,5511,82,42211,82,94005,82,65363,115,119852,115,119904,115,119956,115,120008,115,120060,115,120112,115,120164,115,120216,115,120268,115,120320,115,120372,115,120424,115,120476,115,42801,115,445,115,1109,115,43946,115,71873,115,66632,115,65331,83,119826,83,119878,83,119930,83,119982,83,120034,83,120086,83,120138,83,120190,83,120242,83,120294,83,120346,83,120398,83,120450,83,1029,83,1359,83,5077,83,5082,83,42210,83,94010,83,66198,83,66592,83,119853,116,119905,116,119957,116,120009,116,120061,116,120113,116,120165,116,120217,116,120269,116,120321,116,120373,116,120425,116,120477,116,8868,84,10201,84,128872,84,65332,84,119827,84,119879,84,119931,84,119983,84,120035,84,120087,84,120139,84,120191,84,120243,84,120295,84,120347,84,120399,84,120451,84,932,84,120507,84,120565,84,120623,84,120681,84,120739,84,11430,84,5026,84,42196,84,93962,84,71868,84,66199,84,66225,84,66325,84,119854,117,119906,117,119958,117,120010,117,120062,117,120114,117,120166,117,120218,117,120270,117,120322,117,120374,117,120426,117,120478,117,42911,117,7452,117,43854,117,43858,117,651,117,965,117,120534,117,120592,117,120650,117,120708,117,120766,117,1405,117,66806,117,71896,117,8746,85,8899,85,119828,85,119880,85,119932,85,119984,85,120036,85,120088,85,120140,85,120192,85,120244,85,120296,85,120348,85,120400,85,120452,85,1357,85,4608,85,66766,85,5196,85,42228,85,94018,85,71864,85,8744,118,8897,118,65366,118,8564,118,119855,118,119907,118,119959,118,120011,118,120063,118,120115,118,120167,118,120219,118,120271,118,120323,118,120375,118,120427,118,120479,118,7456,118,957,118,120526,118,120584,118,120642,118,120700,118,120758,118,1141,118,1496,118,71430,118,43945,118,71872,118,119309,86,1639,86,1783,86,8548,86,119829,86,119881,86,119933,86,119985,86,120037,86,120089,86,120141,86,120193,86,120245,86,120297,86,120349,86,120401,86,120453,86,1140,86,11576,86,5081,86,5167,86,42719,86,42214,86,93960,86,71840,86,66845,86,623,119,119856,119,119908,119,119960,119,120012,119,120064,119,120116,119,120168,119,120220,119,120272,119,120324,119,120376,119,120428,119,120480,119,7457,119,1121,119,1309,119,1377,119,71434,119,71438,119,71439,119,43907,119,71919,87,71910,87,119830,87,119882,87,119934,87,119986,87,120038,87,120090,87,120142,87,120194,87,120246,87,120298,87,120350,87,120402,87,120454,87,1308,87,5043,87,5076,87,42218,87,5742,120,10539,120,10540,120,10799,120,65368,120,8569,120,119857,120,119909,120,119961,120,120013,120,120065,120,120117,120,120169,120,120221,120,120273,120,120325,120,120377,120,120429,120,120481,120,5441,120,5501,120,5741,88,9587,88,66338,88,71916,88,65336,88,8553,88,119831,88,119883,88,119935,88,119987,88,120039,88,120091,88,120143,88,120195,88,120247,88,120299,88,120351,88,120403,88,120455,88,42931,88,935,88,120510,88,120568,88,120626,88,120684,88,120742,88,11436,88,11613,88,5815,88,42219,88,66192,88,66228,88,66327,88,66855,88,611,121,7564,121,65369,121,119858,121,119910,121,119962,121,120014,121,120066,121,120118,121,120170,121,120222,121,120274,121,120326,121,120378,121,120430,121,120482,121,655,121,7935,121,43866,121,947,121,8509,121,120516,121,120574,121,120632,121,120690,121,120748,121,1199,121,4327,121,71900,121,65337,89,119832,89,119884,89,119936,89,119988,89,120040,89,120092,89,120144,89,120196,89,120248,89,120300,89,120352,89,120404,89,120456,89,933,89,978,89,120508,89,120566,89,120624,89,120682,89,120740,89,11432,89,1198,89,5033,89,5053,89,42220,89,94019,89,71844,89,66226,89,119859,122,119911,122,119963,122,120015,122,120067,122,120119,122,120171,122,120223,122,120275,122,120327,122,120379,122,120431,122,120483,122,7458,122,43923,122,71876,122,66293,90,71909,90,65338,90,8484,90,8488,90,119833,90,119885,90,119937,90,119989,90,120041,90,120197,90,120249,90,120301,90,120353,90,120405,90,120457,90,918,90,120493,90,120551,90,120609,90,120667,90,120725,90,5059,90,42204,90,71849,90,65282,34,65284,36,65285,37,65286,38,65290,42,65291,43,65294,46,65295,47,65296,48,65297,49,65298,50,65299,51,65300,52,65301,53,65302,54,65303,55,65304,56,65305,57,65308,60,65309,61,65310,62,65312,64,65316,68,65318,70,65319,71,65324,76,65329,81,65330,82,65333,85,65334,86,65335,87,65343,95,65346,98,65348,100,65350,102,65355,107,65357,109,65358,110,65361,113,65362,114,65364,116,65365,117,65367,119,65370,122,65371,123,65373,125,119846,109],"_default":[160,32,8211,45,65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"cs":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"de":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"es":[8211,45,65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"fr":[65374,126,65306,58,65281,33,8216,96,8245,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"it":[160,32,8211,45,65374,126,65306,58,65281,33,8216,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"ja":[8211,45,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65292,44,65307,59],"ko":[8211,45,65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"pl":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"pt-BR":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"qps-ploc":[160,32,8211,45,65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"ru":[65374,126,65306,58,65281,33,8216,96,8217,96,8245,96,180,96,12494,47,305,105,921,73,1009,112,215,120,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"tr":[160,32,8211,45,65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65288,40,65289,41,65292,44,65307,59,65311,63],"zh-hans":[65374,126,65306,58,65281,33,8245,96,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65288,40,65289,41],"zh-hant":[8211,45,65374,126,180,96,12494,47,1047,51,1073,54,1072,97,1040,65,1068,98,1042,66,1089,99,1057,67,1077,101,1045,69,1053,72,305,105,1050,75,921,73,1052,77,1086,111,1054,79,1009,112,1088,112,1056,80,1075,114,1058,84,215,120,1093,120,1061,88,1091,121,1059,89,65283,35,65307,59]}')), Xe.cache = new Su({ getCacheKey: JSON.stringify }, (t) => {
  function n(h) {
    const c = /* @__PURE__ */ new Map();
    for (let m = 0; m < h.length; m += 2)
      c.set(h[m], h[m + 1]);
    return c;
  }
  function i(h, c) {
    const m = new Map(h);
    for (const [g, d] of c)
      m.set(g, d);
    return m;
  }
  function r(h, c) {
    if (!h)
      return c;
    const m = /* @__PURE__ */ new Map();
    for (const [g, d] of h)
      c.has(g) && m.set(g, d);
    return m;
  }
  const s = Xe.ambiguousCharacterData.value;
  let a = t.filter((h) => !h.startsWith("_") && h in s);
  a.length === 0 && (a = ["_default"]);
  let l;
  for (const h of a) {
    const c = n(s[h]);
    l = r(l, c);
  }
  const o = n(s._common), u = i(o, l);
  return new Xe(u);
}), Xe._locales = new Ur(() => Object.keys(Xe.ambiguousCharacterData.value).filter((t) => !t.startsWith("_")));
let Ln = Xe;
const vt = class vt {
  static getRawData() {
    return JSON.parse("[9,10,11,12,13,32,127,160,173,847,1564,4447,4448,6068,6069,6155,6156,6157,6158,7355,7356,8192,8193,8194,8195,8196,8197,8198,8199,8200,8201,8202,8203,8204,8205,8206,8207,8234,8235,8236,8237,8238,8239,8287,8288,8289,8290,8291,8292,8293,8294,8295,8296,8297,8298,8299,8300,8301,8302,8303,10240,12288,12644,65024,65025,65026,65027,65028,65029,65030,65031,65032,65033,65034,65035,65036,65037,65038,65039,65279,65440,65520,65521,65522,65523,65524,65525,65526,65527,65528,65532,78844,119155,119156,119157,119158,119159,119160,119161,119162,917504,917505,917506,917507,917508,917509,917510,917511,917512,917513,917514,917515,917516,917517,917518,917519,917520,917521,917522,917523,917524,917525,917526,917527,917528,917529,917530,917531,917532,917533,917534,917535,917536,917537,917538,917539,917540,917541,917542,917543,917544,917545,917546,917547,917548,917549,917550,917551,917552,917553,917554,917555,917556,917557,917558,917559,917560,917561,917562,917563,917564,917565,917566,917567,917568,917569,917570,917571,917572,917573,917574,917575,917576,917577,917578,917579,917580,917581,917582,917583,917584,917585,917586,917587,917588,917589,917590,917591,917592,917593,917594,917595,917596,917597,917598,917599,917600,917601,917602,917603,917604,917605,917606,917607,917608,917609,917610,917611,917612,917613,917614,917615,917616,917617,917618,917619,917620,917621,917622,917623,917624,917625,917626,917627,917628,917629,917630,917631,917760,917761,917762,917763,917764,917765,917766,917767,917768,917769,917770,917771,917772,917773,917774,917775,917776,917777,917778,917779,917780,917781,917782,917783,917784,917785,917786,917787,917788,917789,917790,917791,917792,917793,917794,917795,917796,917797,917798,917799,917800,917801,917802,917803,917804,917805,917806,917807,917808,917809,917810,917811,917812,917813,917814,917815,917816,917817,917818,917819,917820,917821,917822,917823,917824,917825,917826,917827,917828,917829,917830,917831,917832,917833,917834,917835,917836,917837,917838,917839,917840,917841,917842,917843,917844,917845,917846,917847,917848,917849,917850,917851,917852,917853,917854,917855,917856,917857,917858,917859,917860,917861,917862,917863,917864,917865,917866,917867,917868,917869,917870,917871,917872,917873,917874,917875,917876,917877,917878,917879,917880,917881,917882,917883,917884,917885,917886,917887,917888,917889,917890,917891,917892,917893,917894,917895,917896,917897,917898,917899,917900,917901,917902,917903,917904,917905,917906,917907,917908,917909,917910,917911,917912,917913,917914,917915,917916,917917,917918,917919,917920,917921,917922,917923,917924,917925,917926,917927,917928,917929,917930,917931,917932,917933,917934,917935,917936,917937,917938,917939,917940,917941,917942,917943,917944,917945,917946,917947,917948,917949,917950,917951,917952,917953,917954,917955,917956,917957,917958,917959,917960,917961,917962,917963,917964,917965,917966,917967,917968,917969,917970,917971,917972,917973,917974,917975,917976,917977,917978,917979,917980,917981,917982,917983,917984,917985,917986,917987,917988,917989,917990,917991,917992,917993,917994,917995,917996,917997,917998,917999]");
  }
  static getData() {
    return this._data || (this._data = new Set(vt.getRawData())), this._data;
  }
  static isInvisibleCharacter(t) {
    return vt.getData().has(t);
  }
  static containsInvisibleCharacter(t) {
    for (let n = 0; n < t.length; n++) {
      const i = t.codePointAt(n);
      if (typeof i == "number" && vt.isInvisibleCharacter(i))
        return !0;
    }
    return !1;
  }
  static get codePoints() {
    return vt.getData();
  }
};
vt._data = void 0;
let cn = vt;
const Bu = "$initialize";
var Ee;
(function(e) {
  e[e.Request = 0] = "Request", e[e.Reply = 1] = "Reply", e[e.SubscribeEvent = 2] = "SubscribeEvent", e[e.Event = 3] = "Event", e[e.UnsubscribeEvent = 4] = "UnsubscribeEvent";
})(Ee || (Ee = {}));
class Vu {
  constructor(t, n, i, r) {
    this.vsWorker = t, this.req = n, this.method = i, this.args = r, this.type = Ee.Request;
  }
}
class Br {
  constructor(t, n, i, r) {
    this.vsWorker = t, this.seq = n, this.res = i, this.err = r, this.type = Ee.Reply;
  }
}
class qu {
  constructor(t, n, i, r) {
    this.vsWorker = t, this.req = n, this.eventName = i, this.arg = r, this.type = Ee.SubscribeEvent;
  }
}
class Hu {
  constructor(t, n, i) {
    this.vsWorker = t, this.req = n, this.event = i, this.type = Ee.Event;
  }
}
class $u {
  constructor(t, n) {
    this.vsWorker = t, this.req = n, this.type = Ee.UnsubscribeEvent;
  }
}
class Wu {
  constructor(t) {
    this._workerId = -1, this._handler = t, this._lastSentReq = 0, this._pendingReplies = /* @__PURE__ */ Object.create(null), this._pendingEmitters = /* @__PURE__ */ new Map(), this._pendingEvents = /* @__PURE__ */ new Map();
  }
  setWorkerId(t) {
    this._workerId = t;
  }
  sendMessage(t, n) {
    const i = String(++this._lastSentReq);
    return new Promise((r, s) => {
      this._pendingReplies[i] = {
        resolve: r,
        reject: s
      }, this._send(new Vu(this._workerId, i, t, n));
    });
  }
  listen(t, n) {
    let i = null;
    const r = new He({
      onWillAddFirstListener: () => {
        i = String(++this._lastSentReq), this._pendingEmitters.set(i, r), this._send(new qu(this._workerId, i, t, n));
      },
      onDidRemoveLastListener: () => {
        this._pendingEmitters.delete(i), this._send(new $u(this._workerId, i)), i = null;
      }
    });
    return r.event;
  }
  handleMessage(t) {
    !t || !t.vsWorker || this._workerId !== -1 && t.vsWorker !== this._workerId || this._handleMessage(t);
  }
  _handleMessage(t) {
    switch (t.type) {
      case Ee.Reply:
        return this._handleReplyMessage(t);
      case Ee.Request:
        return this._handleRequestMessage(t);
      case Ee.SubscribeEvent:
        return this._handleSubscribeEventMessage(t);
      case Ee.Event:
        return this._handleEventMessage(t);
      case Ee.UnsubscribeEvent:
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
    this._handler.handleMessage(t.method, t.args).then((r) => {
      this._send(new Br(this._workerId, n, r, void 0));
    }, (r) => {
      r.detail instanceof Error && (r.detail = yr(r.detail)), this._send(new Br(this._workerId, n, void 0, yr(r)));
    });
  }
  _handleSubscribeEventMessage(t) {
    const n = t.req, i = this._handler.handleEvent(t.eventName, t.arg)((r) => {
      this._send(new Hu(this._workerId, n, r));
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
    if (t.type === Ee.Request)
      for (let i = 0; i < t.args.length; i++)
        t.args[i] instanceof ArrayBuffer && n.push(t.args[i]);
    else t.type === Ee.Reply && t.res instanceof ArrayBuffer && n.push(t.res);
    this._handler.sendMessage(t, n);
  }
}
function ul(e) {
  return e[0] === "o" && e[1] === "n" && ol(e.charCodeAt(2));
}
function cl(e) {
  return /^onDynamic/.test(e) && ol(e.charCodeAt(9));
}
function ju(e, t, n) {
  const i = (a) => function() {
    const l = Array.prototype.slice.call(arguments, 0);
    return t(a, l);
  }, r = (a) => function(l) {
    return n(a, l);
  }, s = {};
  for (const a of e) {
    if (cl(a)) {
      s[a] = r(a);
      continue;
    }
    if (ul(a)) {
      s[a] = n(a, void 0);
      continue;
    }
    s[a] = i(a);
  }
  return s;
}
class Gu {
  constructor(t, n) {
    this._requestHandlerFactory = n, this._requestHandler = null, this._protocol = new Wu({
      sendMessage: (i, r) => {
        t(i, r);
      },
      handleMessage: (i, r) => this._handleMessage(i, r),
      handleEvent: (i, r) => this._handleEvent(i, r)
    });
  }
  onmessage(t) {
    this._protocol.handleMessage(t);
  }
  _handleMessage(t, n) {
    if (t === Bu)
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
    if (cl(t)) {
      const i = this._requestHandler[t].call(this._requestHandler, n);
      if (typeof i != "function")
        throw new Error(`Missing dynamic event ${t} on request handler.`);
      return i;
    }
    if (ul(t)) {
      const i = this._requestHandler[t];
      if (typeof i != "function")
        throw new Error(`Missing event ${t} on request handler.`);
      return i;
    }
    throw new Error(`Malformed event name ${t}`);
  }
  initialize(t, n, i, r) {
    this._protocol.setWorkerId(t);
    const l = ju(r, (o, u) => this._protocol.sendMessage(o, u), (o, u) => this._protocol.listen(o, u));
    return this._requestHandlerFactory ? (this._requestHandler = this._requestHandlerFactory(l), Promise.resolve(Ti(this._requestHandler))) : (n && (typeof n.baseUrl < "u" && delete n.baseUrl, typeof n.paths < "u" && typeof n.paths.vs < "u" && delete n.paths.vs, typeof n.trustedTypesPolicy < "u" && delete n.trustedTypesPolicy, n.catchError = !0, globalThis.require.config(n)), new Promise((o, u) => {
      (void 0)([i], (c) => {
        if (this._requestHandler = c.create(l), !this._requestHandler) {
          u(new Error("No RequestHandler!"));
          return;
        }
        o(Ti(this._requestHandler));
      }, u);
    }));
  }
}
class ut {
  constructor(t, n, i, r) {
    this.originalStart = t, this.originalLength = n, this.modifiedStart = i, this.modifiedLength = r;
  }
  getOriginalEnd() {
    return this.originalStart + this.originalLength;
  }
  getModifiedEnd() {
    return this.modifiedStart + this.modifiedLength;
  }
}
function Vr(e, t) {
  return (t << 5) - t + e | 0;
}
function zu(e, t) {
  t = Vr(149417, t);
  for (let n = 0, i = e.length; n < i; n++)
    t = Vr(e.charCodeAt(n), t);
  return t;
}
var De;
(function(e) {
  e[e.BLOCK_SIZE = 64] = "BLOCK_SIZE", e[e.UNICODE_REPLACEMENT = 65533] = "UNICODE_REPLACEMENT";
})(De || (De = {}));
function mi(e, t, n = 32) {
  const i = n - t, r = ~((1 << i) - 1);
  return (e << t | (r & e) >>> i) >>> 0;
}
function qr(e, t = 0, n = e.byteLength, i = 0) {
  for (let r = 0; r < n; r++)
    e[t + r] = i;
}
function Xu(e, t, n = "0") {
  for (; e.length < t; )
    e = n + e;
  return e;
}
function Ct(e, t = 32) {
  return e instanceof ArrayBuffer ? Array.from(new Uint8Array(e)).map((n) => n.toString(16).padStart(2, "0")).join("") : Xu((e >>> 0).toString(16), t / 4);
}
const li = class li {
  constructor() {
    this._h0 = 1732584193, this._h1 = 4023233417, this._h2 = 2562383102, this._h3 = 271733878, this._h4 = 3285377520, this._buff = new Uint8Array(De.BLOCK_SIZE + 3), this._buffDV = new DataView(this._buff.buffer), this._buffLen = 0, this._totalLen = 0, this._leftoverHighSurrogate = 0, this._finished = !1;
  }
  update(t) {
    const n = t.length;
    if (n === 0)
      return;
    const i = this._buff;
    let r = this._buffLen, s = this._leftoverHighSurrogate, a, l;
    for (s !== 0 ? (a = s, l = -1, s = 0) : (a = t.charCodeAt(0), l = 0); ; ) {
      let o = a;
      if (Wn(a))
        if (l + 1 < n) {
          const u = t.charCodeAt(l + 1);
          Si(u) ? (l++, o = ll(a, u)) : o = De.UNICODE_REPLACEMENT;
        } else {
          s = a;
          break;
        }
      else Si(a) && (o = De.UNICODE_REPLACEMENT);
      if (r = this._push(i, r, o), l++, l < n)
        a = t.charCodeAt(l);
      else
        break;
    }
    this._buffLen = r, this._leftoverHighSurrogate = s;
  }
  _push(t, n, i) {
    return i < 128 ? t[n++] = i : i < 2048 ? (t[n++] = 192 | (i & 1984) >>> 6, t[n++] = 128 | (i & 63) >>> 0) : i < 65536 ? (t[n++] = 224 | (i & 61440) >>> 12, t[n++] = 128 | (i & 4032) >>> 6, t[n++] = 128 | (i & 63) >>> 0) : (t[n++] = 240 | (i & 1835008) >>> 18, t[n++] = 128 | (i & 258048) >>> 12, t[n++] = 128 | (i & 4032) >>> 6, t[n++] = 128 | (i & 63) >>> 0), n >= De.BLOCK_SIZE && (this._step(), n -= De.BLOCK_SIZE, this._totalLen += De.BLOCK_SIZE, t[0] = t[De.BLOCK_SIZE + 0], t[1] = t[De.BLOCK_SIZE + 1], t[2] = t[De.BLOCK_SIZE + 2]), n;
  }
  digest() {
    return this._finished || (this._finished = !0, this._leftoverHighSurrogate && (this._leftoverHighSurrogate = 0, this._buffLen = this._push(this._buff, this._buffLen, De.UNICODE_REPLACEMENT)), this._totalLen += this._buffLen, this._wrapUp()), Ct(this._h0) + Ct(this._h1) + Ct(this._h2) + Ct(this._h3) + Ct(this._h4);
  }
  _wrapUp() {
    this._buff[this._buffLen++] = 128, qr(this._buff, this._buffLen), this._buffLen > 56 && (this._step(), qr(this._buff));
    const t = 8 * this._totalLen;
    this._buffDV.setUint32(56, Math.floor(t / 4294967296), !1), this._buffDV.setUint32(60, t % 4294967296, !1), this._step();
  }
  _step() {
    const t = li._bigBlock32, n = this._buffDV;
    for (let c = 0; c < 64; c += 4)
      t.setUint32(c, n.getUint32(c, !1), !1);
    for (let c = 64; c < 320; c += 4)
      t.setUint32(c, mi(t.getUint32(c - 12, !1) ^ t.getUint32(c - 32, !1) ^ t.getUint32(c - 56, !1) ^ t.getUint32(c - 64, !1), 1), !1);
    let i = this._h0, r = this._h1, s = this._h2, a = this._h3, l = this._h4, o, u, h;
    for (let c = 0; c < 80; c++)
      c < 20 ? (o = r & s | ~r & a, u = 1518500249) : c < 40 ? (o = r ^ s ^ a, u = 1859775393) : c < 60 ? (o = r & s | r & a | s & a, u = 2400959708) : (o = r ^ s ^ a, u = 3395469782), h = mi(i, 5) + o + l + u + t.getUint32(c * 4, !1) & 4294967295, l = a, a = s, s = mi(r, 30), r = i, i = h;
    this._h0 = this._h0 + i & 4294967295, this._h1 = this._h1 + r & 4294967295, this._h2 = this._h2 + s & 4294967295, this._h3 = this._h3 + a & 4294967295, this._h4 = this._h4 + l & 4294967295;
  }
};
li._bigBlock32 = new DataView(new ArrayBuffer(320));
let Hr = li;
class $r {
  constructor(t) {
    this.source = t;
  }
  getElements() {
    const t = this.source, n = new Int32Array(t.length);
    for (let i = 0, r = t.length; i < r; i++)
      n[i] = t.charCodeAt(i);
    return n;
  }
}
function Ju(e, t, n) {
  return new mt(new $r(e), new $r(t)).ComputeDiff(n).changes;
}
class kt {
  static Assert(t, n) {
    if (!t)
      throw new Error(n);
  }
}
class Tt {
  static Copy(t, n, i, r, s) {
    for (let a = 0; a < s; a++)
      i[r + a] = t[n + a];
  }
  static Copy2(t, n, i, r, s) {
    for (let a = 0; a < s; a++)
      i[r + a] = t[n + a];
  }
}
var Ke;
(function(e) {
  e[e.MaxDifferencesHistory = 1447] = "MaxDifferencesHistory";
})(Ke || (Ke = {}));
class Wr {
  constructor() {
    this.m_changes = [], this.m_originalStart = Pe.MAX_SAFE_SMALL_INTEGER, this.m_modifiedStart = Pe.MAX_SAFE_SMALL_INTEGER, this.m_originalCount = 0, this.m_modifiedCount = 0;
  }
  MarkNextChange() {
    (this.m_originalCount > 0 || this.m_modifiedCount > 0) && this.m_changes.push(new ut(
      this.m_originalStart,
      this.m_originalCount,
      this.m_modifiedStart,
      this.m_modifiedCount
    )), this.m_originalCount = 0, this.m_modifiedCount = 0, this.m_originalStart = Pe.MAX_SAFE_SMALL_INTEGER, this.m_modifiedStart = Pe.MAX_SAFE_SMALL_INTEGER;
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
class mt {
  constructor(t, n, i = null) {
    this.ContinueProcessingPredicate = i, this._originalSequence = t, this._modifiedSequence = n;
    const [r, s, a] = mt._getElements(t), [l, o, u] = mt._getElements(n);
    this._hasStrings = a && u, this._originalStringElements = r, this._originalElementsOrHash = s, this._modifiedStringElements = l, this._modifiedElementsOrHash = o, this.m_forwardHistory = [], this.m_reverseHistory = [];
  }
  static _isStringArray(t) {
    return t.length > 0 && typeof t[0] == "string";
  }
  static _getElements(t) {
    const n = t.getElements();
    if (mt._isStringArray(n)) {
      const i = new Int32Array(n.length);
      for (let r = 0, s = n.length; r < s; r++)
        i[r] = zu(n[r], 0);
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
    const i = mt._getStrictElement(this._originalSequence, t), r = mt._getStrictElement(this._modifiedSequence, n);
    return i === r;
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
  _ComputeDiff(t, n, i, r, s) {
    const a = [!1];
    let l = this.ComputeDiffRecursive(t, n, i, r, a);
    return s && (l = this.PrettifyChanges(l)), {
      quitEarly: a[0],
      changes: l
    };
  }
  ComputeDiffRecursive(t, n, i, r, s) {
    for (s[0] = !1; t <= n && i <= r && this.ElementsAreEqual(t, i); )
      t++, i++;
    for (; n >= t && r >= i && this.ElementsAreEqual(n, r); )
      n--, r--;
    if (t > n || i > r) {
      let c;
      return i <= r ? (kt.Assert(t === n + 1, "originalStart should only be one more than originalEnd"), c = [
        new ut(t, 0, i, r - i + 1)
      ]) : t <= n ? (kt.Assert(i === r + 1, "modifiedStart should only be one more than modifiedEnd"), c = [
        new ut(t, n - t + 1, i, 0)
      ]) : (kt.Assert(t === n + 1, "originalStart should only be one more than originalEnd"), kt.Assert(i === r + 1, "modifiedStart should only be one more than modifiedEnd"), c = []), c;
    }
    const a = [0], l = [0], o = this.ComputeRecursionPoint(t, n, i, r, a, l, s), u = a[0], h = l[0];
    if (o !== null)
      return o;
    if (!s[0]) {
      const c = this.ComputeDiffRecursive(t, u, i, h, s);
      let m = [];
      return s[0] ? m = [
        new ut(
          u + 1,
          n - (u + 1) + 1,
          h + 1,
          r - (h + 1) + 1
        )
      ] : m = this.ComputeDiffRecursive(u + 1, n, h + 1, r, s), this.ConcatenateChanges(c, m);
    }
    return [
      new ut(
        t,
        n - t + 1,
        i,
        r - i + 1
      )
    ];
  }
  WALKTRACE(t, n, i, r, s, a, l, o, u, h, c, m, g, d, p, _, x, R) {
    let v = null, L = null, b = new Wr(), E = n, k = i, D = g[0] - _[0] - r, O = Pe.MIN_SAFE_SMALL_INTEGER, W = this.m_forwardHistory.length - 1;
    do {
      const U = D + t;
      U === E || U < k && u[U - 1] < u[U + 1] ? (c = u[U + 1], d = c - D - r, c < O && b.MarkNextChange(), O = c, b.AddModifiedElement(c + 1, d), D = U + 1 - t) : (c = u[U - 1] + 1, d = c - D - r, c < O && b.MarkNextChange(), O = c - 1, b.AddOriginalElement(c, d + 1), D = U - 1 - t), W >= 0 && (u = this.m_forwardHistory[W], t = u[0], E = 1, k = u.length - 1);
    } while (--W >= -1);
    if (v = b.getReverseChanges(), R[0]) {
      let U = g[0] + 1, A = _[0] + 1;
      if (v !== null && v.length > 0) {
        const y = v[v.length - 1];
        U = Math.max(U, y.getOriginalEnd()), A = Math.max(A, y.getModifiedEnd());
      }
      L = [
        new ut(
          U,
          m - U + 1,
          A,
          p - A + 1
        )
      ];
    } else {
      b = new Wr(), E = a, k = l, D = g[0] - _[0] - o, O = Pe.MAX_SAFE_SMALL_INTEGER, W = x ? this.m_reverseHistory.length - 1 : this.m_reverseHistory.length - 2;
      do {
        const U = D + s;
        U === E || U < k && h[U - 1] >= h[U + 1] ? (c = h[U + 1] - 1, d = c - D - o, c > O && b.MarkNextChange(), O = c + 1, b.AddOriginalElement(c + 1, d + 1), D = U + 1 - s) : (c = h[U - 1], d = c - D - o, c > O && b.MarkNextChange(), O = c, b.AddModifiedElement(c + 1, d + 1), D = U - 1 - s), W >= 0 && (h = this.m_reverseHistory[W], s = h[0], E = 1, k = h.length - 1);
      } while (--W >= -1);
      L = b.getChanges();
    }
    return this.ConcatenateChanges(v, L);
  }
  ComputeRecursionPoint(t, n, i, r, s, a, l) {
    let o = 0, u = 0, h = 0, c = 0, m = 0, g = 0;
    t--, i--, s[0] = 0, a[0] = 0, this.m_forwardHistory = [], this.m_reverseHistory = [];
    const d = n - t + (r - i), p = d + 1, _ = new Int32Array(p), x = new Int32Array(p), R = r - i, v = n - t, L = t - i, b = n - r, k = (v - R) % 2 === 0;
    _[R] = t, x[v] = n, l[0] = !1;
    for (let D = 1; D <= d / 2 + 1; D++) {
      let O = 0, W = 0;
      h = this.ClipDiagonalBound(R - D, D, R, p), c = this.ClipDiagonalBound(R + D, D, R, p);
      for (let A = h; A <= c; A += 2) {
        A === h || A < c && _[A - 1] < _[A + 1] ? o = _[A + 1] : o = _[A - 1] + 1, u = o - (A - R) - L;
        const y = o;
        for (; o < n && u < r && this.ElementsAreEqual(o + 1, u + 1); )
          o++, u++;
        if (_[A] = o, o + u > O + W && (O = o, W = u), !k && Math.abs(A - v) <= D - 1 && o >= x[A])
          return s[0] = o, a[0] = u, y <= x[A] && Ke.MaxDifferencesHistory > 0 && D <= Ke.MaxDifferencesHistory + 1 ? this.WALKTRACE(R, h, c, L, v, m, g, b, _, x, o, n, s, u, r, a, k, l) : null;
      }
      const U = (O - t + (W - i) - D) / 2;
      if (this.ContinueProcessingPredicate !== null && !this.ContinueProcessingPredicate(O, U))
        return l[0] = !0, s[0] = O, a[0] = W, U > 0 && Ke.MaxDifferencesHistory > 0 && D <= Ke.MaxDifferencesHistory + 1 ? this.WALKTRACE(R, h, c, L, v, m, g, b, _, x, o, n, s, u, r, a, k, l) : (t++, i++, [
          new ut(
            t,
            n - t + 1,
            i,
            r - i + 1
          )
        ]);
      m = this.ClipDiagonalBound(v - D, D, v, p), g = this.ClipDiagonalBound(v + D, D, v, p);
      for (let A = m; A <= g; A += 2) {
        A === m || A < g && x[A - 1] >= x[A + 1] ? o = x[A + 1] - 1 : o = x[A - 1], u = o - (A - v) - b;
        const y = o;
        for (; o > t && u > i && this.ElementsAreEqual(o, u); )
          o--, u--;
        if (x[A] = o, k && Math.abs(A - R) <= D && o <= _[A])
          return s[0] = o, a[0] = u, y >= _[A] && Ke.MaxDifferencesHistory > 0 && D <= Ke.MaxDifferencesHistory + 1 ? this.WALKTRACE(R, h, c, L, v, m, g, b, _, x, o, n, s, u, r, a, k, l) : null;
      }
      if (D <= Ke.MaxDifferencesHistory) {
        let A = new Int32Array(c - h + 2);
        A[0] = R - h + 1, Tt.Copy2(_, h, A, 1, c - h + 1), this.m_forwardHistory.push(A), A = new Int32Array(g - m + 2), A[0] = v - m + 1, Tt.Copy2(x, m, A, 1, g - m + 1), this.m_reverseHistory.push(A);
      }
    }
    return this.WALKTRACE(R, h, c, L, v, m, g, b, _, x, o, n, s, u, r, a, k, l);
  }
  PrettifyChanges(t) {
    for (let n = 0; n < t.length; n++) {
      const i = t[n], r = n < t.length - 1 ? t[n + 1].originalStart : this._originalElementsOrHash.length, s = n < t.length - 1 ? t[n + 1].modifiedStart : this._modifiedElementsOrHash.length, a = i.originalLength > 0, l = i.modifiedLength > 0;
      for (; i.originalStart + i.originalLength < r && i.modifiedStart + i.modifiedLength < s && (!a || this.OriginalElementsAreEqual(i.originalStart, i.originalStart + i.originalLength)) && (!l || this.ModifiedElementsAreEqual(i.modifiedStart, i.modifiedStart + i.modifiedLength)); ) {
        const u = this.ElementsAreStrictEqual(i.originalStart, i.modifiedStart);
        if (this.ElementsAreStrictEqual(i.originalStart + i.originalLength, i.modifiedStart + i.modifiedLength) && !u)
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
      let r = 0, s = 0;
      if (n > 0) {
        const c = t[n - 1];
        r = c.originalStart + c.originalLength, s = c.modifiedStart + c.modifiedLength;
      }
      const a = i.originalLength > 0, l = i.modifiedLength > 0;
      let o = 0, u = this._boundaryScore(i.originalStart, i.originalLength, i.modifiedStart, i.modifiedLength);
      for (let c = 1; ; c++) {
        const m = i.originalStart - c, g = i.modifiedStart - c;
        if (m < r || g < s || a && !this.OriginalElementsAreEqual(m, m + i.originalLength) || l && !this.ModifiedElementsAreEqual(g, g + i.modifiedLength))
          break;
        const p = (m === r && g === s ? 5 : 0) + this._boundaryScore(m, i.originalLength, g, i.modifiedLength);
        p > u && (u = p, o = c);
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
        const r = t[n - 1], s = t[n], a = s.originalStart - r.originalStart - r.originalLength, l = r.originalStart, o = s.originalStart + s.originalLength, u = o - l, h = r.modifiedStart, c = s.modifiedStart + s.modifiedLength, m = c - h;
        if (a < 5 && u < 20 && m < 20) {
          const g = this._findBetterContiguousSequence(l, u, h, m, a);
          if (g) {
            const [d, p] = g;
            (d !== r.originalStart + r.originalLength || p !== r.modifiedStart + r.modifiedLength) && (r.originalLength = d - r.originalStart, r.modifiedLength = p - r.modifiedStart, s.originalStart = d + a, s.modifiedStart = p + a, s.originalLength = o - s.originalStart, s.modifiedLength = c - s.modifiedStart);
          }
        }
      }
    return t;
  }
  _findBetterContiguousSequence(t, n, i, r, s) {
    if (n < s || r < s)
      return null;
    const a = t + n - s + 1, l = i + r - s + 1;
    let o = 0, u = 0, h = 0;
    for (let c = t; c < a; c++)
      for (let m = i; m < l; m++) {
        const g = this._contiguousSequenceScore(c, m, s);
        g > 0 && g > o && (o = g, u = c, h = m);
      }
    return o > 0 ? [u, h] : null;
  }
  _contiguousSequenceScore(t, n, i) {
    let r = 0;
    for (let s = 0; s < i; s++) {
      if (!this.ElementsAreEqual(t + s, n + s))
        return 0;
      r += this._originalStringElements[t + s].length;
    }
    return r;
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
  _boundaryScore(t, n, i, r) {
    const s = this._OriginalRegionIsBoundary(t, n) ? 1 : 0, a = this._ModifiedRegionIsBoundary(i, r) ? 1 : 0;
    return s + a;
  }
  ConcatenateChanges(t, n) {
    const i = [];
    if (t.length === 0 || n.length === 0)
      return n.length > 0 ? n : t;
    if (this.ChangesOverlap(t[t.length - 1], n[0], i)) {
      const r = new Array(t.length + n.length - 1);
      return Tt.Copy(t, 0, r, 0, t.length - 1), r[t.length - 1] = i[0], Tt.Copy(n, 1, r, t.length, n.length - 1), r;
    } else {
      const r = new Array(t.length + n.length);
      return Tt.Copy(t, 0, r, 0, t.length), Tt.Copy(n, 0, r, t.length, n.length), r;
    }
  }
  ChangesOverlap(t, n, i) {
    if (kt.Assert(t.originalStart <= n.originalStart, "Left change is not less than or equal to right change"), kt.Assert(t.modifiedStart <= n.modifiedStart, "Left change is not less than or equal to right change"), t.originalStart + t.originalLength >= n.originalStart || t.modifiedStart + t.modifiedLength >= n.modifiedStart) {
      const r = t.originalStart;
      let s = t.originalLength;
      const a = t.modifiedStart;
      let l = t.modifiedLength;
      return t.originalStart + t.originalLength >= n.originalStart && (s = n.originalStart + n.originalLength - t.originalStart), t.modifiedStart + t.modifiedLength >= n.modifiedStart && (l = n.modifiedStart + n.modifiedLength - t.modifiedStart), i[0] = new ut(r, s, a, l), !0;
    } else
      return i[0] = null, !1;
  }
  ClipDiagonalBound(t, n, i, r) {
    if (t >= 0 && t < r)
      return t;
    const s = i, a = r - i - 1, l = n % 2 === 0;
    if (t < 0) {
      const o = s % 2 === 0;
      return l === o ? 0 : 1;
    } else {
      const o = a % 2 === 0;
      return l === o ? r - 1 : r - 2;
    }
  }
}
var Mi;
(function(e) {
  e[e.Uri = 1] = "Uri", e[e.Regexp = 2] = "Regexp", e[e.ScmResource = 3] = "ScmResource", e[e.ScmResourceGroup = 4] = "ScmResourceGroup", e[e.ScmProvider = 5] = "ScmProvider", e[e.CommentController = 6] = "CommentController", e[e.CommentThread = 7] = "CommentThread", e[e.CommentThreadInstance = 8] = "CommentThreadInstance", e[e.CommentThreadReply = 9] = "CommentThreadReply", e[e.CommentNode = 10] = "CommentNode", e[e.CommentThreadNode = 11] = "CommentThreadNode", e[e.TimelineActionContext = 12] = "TimelineActionContext", e[e.NotebookCellActionContext = 13] = "NotebookCellActionContext", e[e.NotebookActionContext = 14] = "NotebookActionContext", e[e.TerminalContext = 15] = "TerminalContext", e[e.TestItemContext = 16] = "TestItemContext", e[e.Date = 17] = "Date", e[e.TestMessageMenuArgs = 18] = "TestMessageMenuArgs";
})(Mi || (Mi = {}));
let wt;
const gi = globalThis.vscode;
var nl;
if (typeof gi < "u" && typeof gi.process < "u") {
  const e = gi.process;
  wt = {
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
} else typeof process < "u" && typeof ((nl = process == null ? void 0 : process.versions) == null ? void 0 : nl.node) == "string" ? wt = {
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
} : wt = {
  get platform() {
    return vn ? "win32" : Eu ? "darwin" : "linux";
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
const jn = wt.cwd, Yu = wt.env, Qu = wt.platform;
wt.arch;
const Zu = 65, Ku = 97, Cu = 90, ec = 122, gt = 46, pe = 47, xe = 92, st = 58, tc = 63;
class fl extends Error {
  constructor(t, n, i) {
    let r;
    typeof n == "string" && n.indexOf("not ") === 0 ? (r = "must not be", n = n.replace(/^not /, "")) : r = "must be";
    const s = t.indexOf(".") !== -1 ? "property" : "argument";
    let a = `The "${t}" ${s} ${r} of type ${n}`;
    a += `. Received type ${typeof i}`, super(a), this.code = "ERR_INVALID_ARG_TYPE";
  }
}
function nc(e, t) {
  if (e === null || typeof e != "object")
    throw new fl(t, "Object", e);
}
function le(e, t) {
  if (typeof e != "string")
    throw new fl(t, "string", e);
}
const Oe = Qu === "win32";
function z(e) {
  return e === pe || e === xe;
}
function Ii(e) {
  return e === pe;
}
function at(e) {
  return e >= Zu && e <= Cu || e >= Ku && e <= ec;
}
function Gn(e, t, n, i) {
  let r = "", s = 0, a = -1, l = 0, o = 0;
  for (let u = 0; u <= e.length; ++u) {
    if (u < e.length)
      o = e.charCodeAt(u);
    else {
      if (i(o))
        break;
      o = pe;
    }
    if (i(o)) {
      if (!(a === u - 1 || l === 1)) if (l === 2) {
        if (r.length < 2 || s !== 2 || r.charCodeAt(r.length - 1) !== gt || r.charCodeAt(r.length - 2) !== gt) {
          if (r.length > 2) {
            const h = r.lastIndexOf(n);
            h === -1 ? (r = "", s = 0) : (r = r.slice(0, h), s = r.length - 1 - r.lastIndexOf(n)), a = u, l = 0;
            continue;
          } else if (r.length !== 0) {
            r = "", s = 0, a = u, l = 0;
            continue;
          }
        }
        t && (r += r.length > 0 ? `${n}..` : "..", s = 2);
      } else
        r.length > 0 ? r += `${n}${e.slice(a + 1, u)}` : r = e.slice(a + 1, u), s = u - a - 1;
      a = u, l = 0;
    } else o === gt && l !== -1 ? ++l : l = -1;
  }
  return r;
}
function ic(e) {
  return e ? `${e[0] === "." ? "" : "."}${e}` : "";
}
function hl(e, t) {
  nc(t, "pathObject");
  const n = t.dir || t.root, i = t.base || `${t.name || ""}${ic(t.ext)}`;
  return n ? n === t.root ? `${n}${i}` : `${n}${e}${i}` : i;
}
const ge = {
  resolve(...e) {
    let t = "", n = "", i = !1;
    for (let r = e.length - 1; r >= -1; r--) {
      let s;
      if (r >= 0) {
        if (s = e[r], le(s, `paths[${r}]`), s.length === 0)
          continue;
      } else t.length === 0 ? s = jn() : (s = Yu[`=${t}`] || jn(), (s === void 0 || s.slice(0, 2).toLowerCase() !== t.toLowerCase() && s.charCodeAt(2) === xe) && (s = `${t}\\`));
      const a = s.length;
      let l = 0, o = "", u = !1;
      const h = s.charCodeAt(0);
      if (a === 1)
        z(h) && (l = 1, u = !0);
      else if (z(h))
        if (u = !0, z(s.charCodeAt(1))) {
          let c = 2, m = c;
          for (; c < a && !z(s.charCodeAt(c)); )
            c++;
          if (c < a && c !== m) {
            const g = s.slice(m, c);
            for (m = c; c < a && z(s.charCodeAt(c)); )
              c++;
            if (c < a && c !== m) {
              for (m = c; c < a && !z(s.charCodeAt(c)); )
                c++;
              (c === a || c !== m) && (o = `\\\\${g}\\${s.slice(m, c)}`, l = c);
            }
          }
        } else
          l = 1;
      else at(h) && s.charCodeAt(1) === st && (o = s.slice(0, 2), l = 2, a > 2 && z(s.charCodeAt(2)) && (u = !0, l = 3));
      if (o.length > 0)
        if (t.length > 0) {
          if (o.toLowerCase() !== t.toLowerCase())
            continue;
        } else
          t = o;
      if (i) {
        if (t.length > 0)
          break;
      } else if (n = `${s.slice(l)}\\${n}`, i = u, u && t.length > 0)
        break;
    }
    return n = Gn(n, !i, "\\", z), i ? `${t}\\${n}` : `${t}${n}` || ".";
  },
  normalize(e) {
    le(e, "path");
    const t = e.length;
    if (t === 0)
      return ".";
    let n = 0, i, r = !1;
    const s = e.charCodeAt(0);
    if (t === 1)
      return Ii(s) ? "\\" : e;
    if (z(s))
      if (r = !0, z(e.charCodeAt(1))) {
        let l = 2, o = l;
        for (; l < t && !z(e.charCodeAt(l)); )
          l++;
        if (l < t && l !== o) {
          const u = e.slice(o, l);
          for (o = l; l < t && z(e.charCodeAt(l)); )
            l++;
          if (l < t && l !== o) {
            for (o = l; l < t && !z(e.charCodeAt(l)); )
              l++;
            if (l === t)
              return `\\\\${u}\\${e.slice(o)}\\`;
            l !== o && (i = `\\\\${u}\\${e.slice(o, l)}`, n = l);
          }
        }
      } else
        n = 1;
    else at(s) && e.charCodeAt(1) === st && (i = e.slice(0, 2), n = 2, t > 2 && z(e.charCodeAt(2)) && (r = !0, n = 3));
    let a = n < t ? Gn(e.slice(n), !r, "\\", z) : "";
    return a.length === 0 && !r && (a = "."), a.length > 0 && z(e.charCodeAt(t - 1)) && (a += "\\"), i === void 0 ? r ? `\\${a}` : a : r ? `${i}\\${a}` : `${i}${a}`;
  },
  isAbsolute(e) {
    le(e, "path");
    const t = e.length;
    if (t === 0)
      return !1;
    const n = e.charCodeAt(0);
    return z(n) || t > 2 && at(n) && e.charCodeAt(1) === st && z(e.charCodeAt(2));
  },
  join(...e) {
    if (e.length === 0)
      return ".";
    let t, n;
    for (let s = 0; s < e.length; ++s) {
      const a = e[s];
      le(a, "path"), a.length > 0 && (t === void 0 ? t = n = a : t += `\\${a}`);
    }
    if (t === void 0)
      return ".";
    let i = !0, r = 0;
    if (typeof n == "string" && z(n.charCodeAt(0))) {
      ++r;
      const s = n.length;
      s > 1 && z(n.charCodeAt(1)) && (++r, s > 2 && (z(n.charCodeAt(2)) ? ++r : i = !1));
    }
    if (i) {
      for (; r < t.length && z(t.charCodeAt(r)); )
        r++;
      r >= 2 && (t = `\\${t.slice(r)}`);
    }
    return ge.normalize(t);
  },
  relative(e, t) {
    if (le(e, "from"), le(t, "to"), e === t)
      return "";
    const n = ge.resolve(e), i = ge.resolve(t);
    if (n === i || (e = n.toLowerCase(), t = i.toLowerCase(), e === t))
      return "";
    let r = 0;
    for (; r < e.length && e.charCodeAt(r) === xe; )
      r++;
    let s = e.length;
    for (; s - 1 > r && e.charCodeAt(s - 1) === xe; )
      s--;
    const a = s - r;
    let l = 0;
    for (; l < t.length && t.charCodeAt(l) === xe; )
      l++;
    let o = t.length;
    for (; o - 1 > l && t.charCodeAt(o - 1) === xe; )
      o--;
    const u = o - l, h = a < u ? a : u;
    let c = -1, m = 0;
    for (; m < h; m++) {
      const d = e.charCodeAt(r + m);
      if (d !== t.charCodeAt(l + m))
        break;
      d === xe && (c = m);
    }
    if (m !== h) {
      if (c === -1)
        return i;
    } else {
      if (u > h) {
        if (t.charCodeAt(l + m) === xe)
          return i.slice(l + m + 1);
        if (m === 2)
          return i.slice(l + m);
      }
      a > h && (e.charCodeAt(r + m) === xe ? c = m : m === 2 && (c = 3)), c === -1 && (c = 0);
    }
    let g = "";
    for (m = r + c + 1; m <= s; ++m)
      (m === s || e.charCodeAt(m) === xe) && (g += g.length === 0 ? ".." : "\\..");
    return l += c, g.length > 0 ? `${g}${i.slice(l, o)}` : (i.charCodeAt(l) === xe && ++l, i.slice(l, o));
  },
  toNamespacedPath(e) {
    if (typeof e != "string" || e.length === 0)
      return e;
    const t = ge.resolve(e);
    if (t.length <= 2)
      return e;
    if (t.charCodeAt(0) === xe) {
      if (t.charCodeAt(1) === xe) {
        const n = t.charCodeAt(2);
        if (n !== tc && n !== gt)
          return `\\\\?\\UNC\\${t.slice(2)}`;
      }
    } else if (at(t.charCodeAt(0)) && t.charCodeAt(1) === st && t.charCodeAt(2) === xe)
      return `\\\\?\\${t}`;
    return e;
  },
  dirname(e) {
    le(e, "path");
    const t = e.length;
    if (t === 0)
      return ".";
    let n = -1, i = 0;
    const r = e.charCodeAt(0);
    if (t === 1)
      return z(r) ? e : ".";
    if (z(r)) {
      if (n = i = 1, z(e.charCodeAt(1))) {
        let l = 2, o = l;
        for (; l < t && !z(e.charCodeAt(l)); )
          l++;
        if (l < t && l !== o) {
          for (o = l; l < t && z(e.charCodeAt(l)); )
            l++;
          if (l < t && l !== o) {
            for (o = l; l < t && !z(e.charCodeAt(l)); )
              l++;
            if (l === t)
              return e;
            l !== o && (n = i = l + 1);
          }
        }
      }
    } else at(r) && e.charCodeAt(1) === st && (n = t > 2 && z(e.charCodeAt(2)) ? 3 : 2, i = n);
    let s = -1, a = !0;
    for (let l = t - 1; l >= i; --l)
      if (z(e.charCodeAt(l))) {
        if (!a) {
          s = l;
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
    t !== void 0 && le(t, "suffix"), le(e, "path");
    let n = 0, i = -1, r = !0, s;
    if (e.length >= 2 && at(e.charCodeAt(0)) && e.charCodeAt(1) === st && (n = 2), t !== void 0 && t.length > 0 && t.length <= e.length) {
      if (t === e)
        return "";
      let a = t.length - 1, l = -1;
      for (s = e.length - 1; s >= n; --s) {
        const o = e.charCodeAt(s);
        if (z(o)) {
          if (!r) {
            n = s + 1;
            break;
          }
        } else
          l === -1 && (r = !1, l = s + 1), a >= 0 && (o === t.charCodeAt(a) ? --a === -1 && (i = s) : (a = -1, i = l));
      }
      return n === i ? i = l : i === -1 && (i = e.length), e.slice(n, i);
    }
    for (s = e.length - 1; s >= n; --s)
      if (z(e.charCodeAt(s))) {
        if (!r) {
          n = s + 1;
          break;
        }
      } else i === -1 && (r = !1, i = s + 1);
    return i === -1 ? "" : e.slice(n, i);
  },
  extname(e) {
    le(e, "path");
    let t = 0, n = -1, i = 0, r = -1, s = !0, a = 0;
    e.length >= 2 && e.charCodeAt(1) === st && at(e.charCodeAt(0)) && (t = i = 2);
    for (let l = e.length - 1; l >= t; --l) {
      const o = e.charCodeAt(l);
      if (z(o)) {
        if (!s) {
          i = l + 1;
          break;
        }
        continue;
      }
      r === -1 && (s = !1, r = l + 1), o === gt ? n === -1 ? n = l : a !== 1 && (a = 1) : n !== -1 && (a = -1);
    }
    return n === -1 || r === -1 || a === 0 || a === 1 && n === r - 1 && n === i + 1 ? "" : e.slice(n, r);
  },
  format: hl.bind(null, "\\"),
  parse(e) {
    le(e, "path");
    const t = { root: "", dir: "", base: "", ext: "", name: "" };
    if (e.length === 0)
      return t;
    const n = e.length;
    let i = 0, r = e.charCodeAt(0);
    if (n === 1)
      return z(r) ? (t.root = t.dir = e, t) : (t.base = t.name = e, t);
    if (z(r)) {
      if (i = 1, z(e.charCodeAt(1))) {
        let c = 2, m = c;
        for (; c < n && !z(e.charCodeAt(c)); )
          c++;
        if (c < n && c !== m) {
          for (m = c; c < n && z(e.charCodeAt(c)); )
            c++;
          if (c < n && c !== m) {
            for (m = c; c < n && !z(e.charCodeAt(c)); )
              c++;
            c === n ? i = c : c !== m && (i = c + 1);
          }
        }
      }
    } else if (at(r) && e.charCodeAt(1) === st) {
      if (n <= 2)
        return t.root = t.dir = e, t;
      if (i = 2, z(e.charCodeAt(2))) {
        if (n === 3)
          return t.root = t.dir = e, t;
        i = 3;
      }
    }
    i > 0 && (t.root = e.slice(0, i));
    let s = -1, a = i, l = -1, o = !0, u = e.length - 1, h = 0;
    for (; u >= i; --u) {
      if (r = e.charCodeAt(u), z(r)) {
        if (!o) {
          a = u + 1;
          break;
        }
        continue;
      }
      l === -1 && (o = !1, l = u + 1), r === gt ? s === -1 ? s = u : h !== 1 && (h = 1) : s !== -1 && (h = -1);
    }
    return l !== -1 && (s === -1 || h === 0 || h === 1 && s === l - 1 && s === a + 1 ? t.base = t.name = e.slice(a, l) : (t.name = e.slice(a, s), t.base = e.slice(a, l), t.ext = e.slice(s, l))), a > 0 && a !== i ? t.dir = e.slice(0, a - 1) : t.dir = t.root, t;
  },
  sep: "\\",
  delimiter: ";",
  win32: null,
  posix: null
}, rc = (() => {
  if (Oe) {
    const e = /\\/g;
    return () => {
      const t = jn().replace(e, "/");
      return t.slice(t.indexOf("/"));
    };
  }
  return () => jn();
})(), be = {
  resolve(...e) {
    let t = "", n = !1;
    for (let i = e.length - 1; i >= -1 && !n; i--) {
      const r = i >= 0 ? e[i] : rc();
      le(r, `paths[${i}]`), r.length !== 0 && (t = `${r}/${t}`, n = r.charCodeAt(0) === pe);
    }
    return t = Gn(t, !n, "/", Ii), n ? `/${t}` : t.length > 0 ? t : ".";
  },
  normalize(e) {
    if (le(e, "path"), e.length === 0)
      return ".";
    const t = e.charCodeAt(0) === pe, n = e.charCodeAt(e.length - 1) === pe;
    return e = Gn(e, !t, "/", Ii), e.length === 0 ? t ? "/" : n ? "./" : "." : (n && (e += "/"), t ? `/${e}` : e);
  },
  isAbsolute(e) {
    return le(e, "path"), e.length > 0 && e.charCodeAt(0) === pe;
  },
  join(...e) {
    if (e.length === 0)
      return ".";
    let t;
    for (let n = 0; n < e.length; ++n) {
      const i = e[n];
      le(i, "path"), i.length > 0 && (t === void 0 ? t = i : t += `/${i}`);
    }
    return t === void 0 ? "." : be.normalize(t);
  },
  relative(e, t) {
    if (le(e, "from"), le(t, "to"), e === t || (e = be.resolve(e), t = be.resolve(t), e === t))
      return "";
    const n = 1, i = e.length, r = i - n, s = 1, a = t.length - s, l = r < a ? r : a;
    let o = -1, u = 0;
    for (; u < l; u++) {
      const c = e.charCodeAt(n + u);
      if (c !== t.charCodeAt(s + u))
        break;
      c === pe && (o = u);
    }
    if (u === l)
      if (a > l) {
        if (t.charCodeAt(s + u) === pe)
          return t.slice(s + u + 1);
        if (u === 0)
          return t.slice(s + u);
      } else r > l && (e.charCodeAt(n + u) === pe ? o = u : u === 0 && (o = 0));
    let h = "";
    for (u = n + o + 1; u <= i; ++u)
      (u === i || e.charCodeAt(u) === pe) && (h += h.length === 0 ? ".." : "/..");
    return `${h}${t.slice(s + o)}`;
  },
  toNamespacedPath(e) {
    return e;
  },
  dirname(e) {
    if (le(e, "path"), e.length === 0)
      return ".";
    const t = e.charCodeAt(0) === pe;
    let n = -1, i = !0;
    for (let r = e.length - 1; r >= 1; --r)
      if (e.charCodeAt(r) === pe) {
        if (!i) {
          n = r;
          break;
        }
      } else
        i = !1;
    return n === -1 ? t ? "/" : "." : t && n === 1 ? "//" : e.slice(0, n);
  },
  basename(e, t) {
    t !== void 0 && le(t, "ext"), le(e, "path");
    let n = 0, i = -1, r = !0, s;
    if (t !== void 0 && t.length > 0 && t.length <= e.length) {
      if (t === e)
        return "";
      let a = t.length - 1, l = -1;
      for (s = e.length - 1; s >= 0; --s) {
        const o = e.charCodeAt(s);
        if (o === pe) {
          if (!r) {
            n = s + 1;
            break;
          }
        } else
          l === -1 && (r = !1, l = s + 1), a >= 0 && (o === t.charCodeAt(a) ? --a === -1 && (i = s) : (a = -1, i = l));
      }
      return n === i ? i = l : i === -1 && (i = e.length), e.slice(n, i);
    }
    for (s = e.length - 1; s >= 0; --s)
      if (e.charCodeAt(s) === pe) {
        if (!r) {
          n = s + 1;
          break;
        }
      } else i === -1 && (r = !1, i = s + 1);
    return i === -1 ? "" : e.slice(n, i);
  },
  extname(e) {
    le(e, "path");
    let t = -1, n = 0, i = -1, r = !0, s = 0;
    for (let a = e.length - 1; a >= 0; --a) {
      const l = e.charCodeAt(a);
      if (l === pe) {
        if (!r) {
          n = a + 1;
          break;
        }
        continue;
      }
      i === -1 && (r = !1, i = a + 1), l === gt ? t === -1 ? t = a : s !== 1 && (s = 1) : t !== -1 && (s = -1);
    }
    return t === -1 || i === -1 || s === 0 || s === 1 && t === i - 1 && t === n + 1 ? "" : e.slice(t, i);
  },
  format: hl.bind(null, "/"),
  parse(e) {
    le(e, "path");
    const t = { root: "", dir: "", base: "", ext: "", name: "" };
    if (e.length === 0)
      return t;
    const n = e.charCodeAt(0) === pe;
    let i;
    n ? (t.root = "/", i = 1) : i = 0;
    let r = -1, s = 0, a = -1, l = !0, o = e.length - 1, u = 0;
    for (; o >= i; --o) {
      const h = e.charCodeAt(o);
      if (h === pe) {
        if (!l) {
          s = o + 1;
          break;
        }
        continue;
      }
      a === -1 && (l = !1, a = o + 1), h === gt ? r === -1 ? r = o : u !== 1 && (u = 1) : r !== -1 && (u = -1);
    }
    if (a !== -1) {
      const h = s === 0 && n ? 1 : s;
      r === -1 || u === 0 || u === 1 && r === a - 1 && r === s + 1 ? t.base = t.name = e.slice(h, a) : (t.name = e.slice(h, r), t.base = e.slice(h, a), t.ext = e.slice(r, a));
    }
    return s > 0 ? t.dir = e.slice(0, s - 1) : n && (t.dir = "/"), t;
  },
  sep: "/",
  delimiter: ":",
  win32: null,
  posix: null
};
be.win32 = ge.win32 = ge;
be.posix = ge.posix = be;
Oe ? ge.normalize : be.normalize;
Oe ? ge.isAbsolute : be.isAbsolute;
Oe ? ge.join : be.join;
Oe ? ge.resolve : be.resolve;
Oe ? ge.relative : be.relative;
Oe ? ge.dirname : be.dirname;
Oe ? ge.basename : be.basename;
Oe ? ge.extname : be.extname;
Oe ? ge.parse : be.parse;
Oe ? ge.sep : be.sep;
Oe ? ge.delimiter : be.delimiter;
const sc = /^\w[\w\d+.-]*$/, ac = /^\//, oc = /^\/\//;
function lc(e, t) {
  if (!e.scheme && t)
    throw new Error(
      `[UriError]: Scheme is missing: {scheme: "", authority: "${e.authority}", path: "${e.path}", query: "${e.query}", fragment: "${e.fragment}"}`
    );
  if (e.scheme && !sc.test(e.scheme))
    throw new Error("[UriError]: Scheme contains illegal characters.");
  if (e.path) {
    if (e.authority) {
      if (!ac.test(e.path))
        throw new Error(
          '[UriError]: If a URI contains an authority component, then the path component must either be empty or begin with a slash ("/") character'
        );
    } else if (oc.test(e.path))
      throw new Error(
        '[UriError]: If a URI does not contain an authority component, then the path cannot begin with two slash characters ("//")'
      );
  }
}
function uc(e, t) {
  return !e && !t ? "file" : e;
}
function cc(e, t) {
  switch (e) {
    case "https":
    case "http":
    case "file":
      t ? t[0] !== $e && (t = $e + t) : t = $e;
      break;
  }
  return t;
}
const ee = "", $e = "/", fc = /^(([^:/?#]+?):)?(\/\/([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?/;
let _r = class Pn {
  static isUri(t) {
    return t instanceof Pn ? !0 : t ? typeof t.authority == "string" && typeof t.fragment == "string" && typeof t.path == "string" && typeof t.query == "string" && typeof t.scheme == "string" && typeof t.fsPath == "string" && typeof t.with == "function" && typeof t.toString == "function" : !1;
  }
  constructor(t, n, i, r, s, a = !1) {
    typeof t == "object" ? (this.scheme = t.scheme || ee, this.authority = t.authority || ee, this.path = t.path || ee, this.query = t.query || ee, this.fragment = t.fragment || ee) : (this.scheme = uc(t, a), this.authority = n || ee, this.path = cc(this.scheme, i || ee), this.query = r || ee, this.fragment = s || ee, lc(this, a));
  }
  get fsPath() {
    return Di(this, !1);
  }
  with(t) {
    if (!t)
      return this;
    let { scheme: n, authority: i, path: r, query: s, fragment: a } = t;
    return n === void 0 ? n = this.scheme : n === null && (n = ee), i === void 0 ? i = this.authority : i === null && (i = ee), r === void 0 ? r = this.path : r === null && (r = ee), s === void 0 ? s = this.query : s === null && (s = ee), a === void 0 ? a = this.fragment : a === null && (a = ee), n === this.scheme && i === this.authority && r === this.path && s === this.query && a === this.fragment ? this : new St(n, i, r, s, a);
  }
  static parse(t, n = !1) {
    const i = fc.exec(t);
    return i ? new St(
      i[2] || ee,
      kn(i[4] || ee),
      kn(i[5] || ee),
      kn(i[7] || ee),
      kn(i[9] || ee),
      n
    ) : new St(ee, ee, ee, ee, ee);
  }
  static file(t) {
    let n = ee;
    if (vn && (t = t.replace(/\\/g, $e)), t[0] === $e && t[1] === $e) {
      const i = t.indexOf($e, 2);
      i === -1 ? (n = t.substring(2), t = $e) : (n = t.substring(2, i), t = t.substring(i) || $e);
    }
    return new St("file", n, t, ee, ee);
  }
  static from(t, n) {
    return new St(
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
    return vn && t.scheme === "file" ? i = Pn.file(ge.join(Di(t, !0), ...n)).path : i = be.join(t.path, ...n), t.with({ path: i });
  }
  toString(t = !1) {
    return Ui(this, t);
  }
  toJSON() {
    return this;
  }
  static revive(t) {
    if (t) {
      if (t instanceof Pn)
        return t;
      {
        const n = new St(t);
        return n._formatted = t.external ?? null, n._fsPath = t._sep === ml ? t.fsPath ?? null : null, n;
      }
    } else return t;
  }
  [Symbol.for("debug.description")]() {
    return `URI(${this.toString()})`;
  }
};
const ml = vn ? 1 : void 0;
class St extends _r {
  constructor() {
    super(...arguments), this._formatted = null, this._fsPath = null;
  }
  get fsPath() {
    return this._fsPath || (this._fsPath = Di(this, !1)), this._fsPath;
  }
  toString(t = !1) {
    return t ? Ui(this, !0) : (this._formatted || (this._formatted = Ui(this, !1)), this._formatted);
  }
  toJSON() {
    const t = {
      $mid: Mi.Uri
    };
    return this._fsPath && (t.fsPath = this._fsPath, t._sep = ml), this._formatted && (t.external = this._formatted), this.path && (t.path = this.path), this.scheme && (t.scheme = this.scheme), this.authority && (t.authority = this.authority), this.query && (t.query = this.query), this.fragment && (t.fragment = this.fragment), t;
  }
}
const gl = {
  [S.Colon]: "%3A",
  [S.Slash]: "%2F",
  [S.QuestionMark]: "%3F",
  [S.Hash]: "%23",
  [S.OpenSquareBracket]: "%5B",
  [S.CloseSquareBracket]: "%5D",
  [S.AtSign]: "%40",
  [S.ExclamationMark]: "%21",
  [S.DollarSign]: "%24",
  [S.Ampersand]: "%26",
  [S.SingleQuote]: "%27",
  [S.OpenParen]: "%28",
  [S.CloseParen]: "%29",
  [S.Asterisk]: "%2A",
  [S.Plus]: "%2B",
  [S.Comma]: "%2C",
  [S.Semicolon]: "%3B",
  [S.Equals]: "%3D",
  [S.Space]: "%20"
};
function jr(e, t, n) {
  let i, r = -1;
  for (let s = 0; s < e.length; s++) {
    const a = e.charCodeAt(s);
    if (a >= S.a && a <= S.z || a >= S.A && a <= S.Z || a >= S.Digit0 && a <= S.Digit9 || a === S.Dash || a === S.Period || a === S.Underline || a === S.Tilde || t && a === S.Slash || n && a === S.OpenSquareBracket || n && a === S.CloseSquareBracket || n && a === S.Colon)
      r !== -1 && (i += encodeURIComponent(e.substring(r, s)), r = -1), i !== void 0 && (i += e.charAt(s));
    else {
      i === void 0 && (i = e.substr(0, s));
      const l = gl[a];
      l !== void 0 ? (r !== -1 && (i += encodeURIComponent(e.substring(r, s)), r = -1), i += l) : r === -1 && (r = s);
    }
  }
  return r !== -1 && (i += encodeURIComponent(e.substring(r))), i !== void 0 ? i : e;
}
function hc(e) {
  let t;
  for (let n = 0; n < e.length; n++) {
    const i = e.charCodeAt(n);
    i === S.Hash || i === S.QuestionMark ? (t === void 0 && (t = e.substr(0, n)), t += gl[i]) : t !== void 0 && (t += e[n]);
  }
  return t !== void 0 ? t : e;
}
function Di(e, t) {
  let n;
  return e.authority && e.path.length > 1 && e.scheme === "file" ? n = `//${e.authority}${e.path}` : e.path.charCodeAt(0) === S.Slash && (e.path.charCodeAt(1) >= S.A && e.path.charCodeAt(1) <= S.Z || e.path.charCodeAt(1) >= S.a && e.path.charCodeAt(1) <= S.z) && e.path.charCodeAt(2) === S.Colon ? t ? n = e.path.substr(1) : n = e.path[1].toLowerCase() + e.path.substr(2) : n = e.path, vn && (n = n.replace(/\//g, "\\")), n;
}
function Ui(e, t) {
  const n = t ? hc : jr;
  let i = "", { scheme: r, authority: s, path: a, query: l, fragment: o } = e;
  if (r && (i += r, i += ":"), (s || r === "file") && (i += $e, i += $e), s) {
    let u = s.indexOf("@");
    if (u !== -1) {
      const h = s.substr(0, u);
      s = s.substr(u + 1), u = h.lastIndexOf(":"), u === -1 ? i += n(h, !1, !1) : (i += n(h.substr(0, u), !1, !1), i += ":", i += n(h.substr(u + 1), !1, !0)), i += "@";
    }
    s = s.toLowerCase(), u = s.lastIndexOf(":"), u === -1 ? i += n(s, !1, !0) : (i += n(s.substr(0, u), !1, !0), i += s.substr(u));
  }
  if (a) {
    if (a.length >= 3 && a.charCodeAt(0) === S.Slash && a.charCodeAt(2) === S.Colon) {
      const u = a.charCodeAt(1);
      u >= S.A && u <= S.Z && (a = `/${String.fromCharCode(u + 32)}:${a.substr(3)}`);
    } else if (a.length >= 2 && a.charCodeAt(1) === S.Colon) {
      const u = a.charCodeAt(0);
      u >= S.A && u <= S.Z && (a = `${String.fromCharCode(u + 32)}:${a.substr(2)}`);
    }
    i += n(a, !0, !1);
  }
  return l && (i += "?", i += n(l, !1, !1)), o && (i += "#", i += t ? o : jr(o, !1, !1)), i;
}
function dl(e) {
  try {
    return decodeURIComponent(e);
  } catch {
    return e.length > 3 ? e.substr(0, 3) + dl(e.substr(3)) : e;
  }
}
const Gr = /(%[0-9A-Za-z][0-9A-Za-z])+/g;
function kn(e) {
  return e.match(Gr) ? e.replace(Gr, (t) => dl(t)) : e;
}
let se = class bt {
  constructor(t, n) {
    this.lineNumber = t, this.column = n;
  }
  with(t = this.lineNumber, n = this.column) {
    return t === this.lineNumber && n === this.column ? this : new bt(t, n);
  }
  delta(t = 0, n = 0) {
    return this.with(this.lineNumber + t, this.column + n);
  }
  equals(t) {
    return bt.equals(this, t);
  }
  static equals(t, n) {
    return !t && !n ? !0 : !!t && !!n && t.lineNumber === n.lineNumber && t.column === n.column;
  }
  isBefore(t) {
    return bt.isBefore(this, t);
  }
  static isBefore(t, n) {
    return t.lineNumber < n.lineNumber ? !0 : n.lineNumber < t.lineNumber ? !1 : t.column < n.column;
  }
  isBeforeOrEqual(t) {
    return bt.isBeforeOrEqual(this, t);
  }
  static isBeforeOrEqual(t, n) {
    return t.lineNumber < n.lineNumber ? !0 : n.lineNumber < t.lineNumber ? !1 : t.column <= n.column;
  }
  static compare(t, n) {
    const i = t.lineNumber | 0, r = n.lineNumber | 0;
    if (i === r) {
      const s = t.column | 0, a = n.column | 0;
      return s - a;
    }
    return i - r;
  }
  clone() {
    return new bt(this.lineNumber, this.column);
  }
  toString() {
    return "(" + this.lineNumber + "," + this.column + ")";
  }
  static lift(t) {
    return new bt(t.lineNumber, t.column);
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
}, J = class ce {
  constructor(t, n, i, r) {
    t > i || t === i && n > r ? (this.startLineNumber = i, this.startColumn = r, this.endLineNumber = t, this.endColumn = n) : (this.startLineNumber = t, this.startColumn = n, this.endLineNumber = i, this.endColumn = r);
  }
  isEmpty() {
    return ce.isEmpty(this);
  }
  static isEmpty(t) {
    return t.startLineNumber === t.endLineNumber && t.startColumn === t.endColumn;
  }
  containsPosition(t) {
    return ce.containsPosition(this, t);
  }
  static containsPosition(t, n) {
    return !(n.lineNumber < t.startLineNumber || n.lineNumber > t.endLineNumber || n.lineNumber === t.startLineNumber && n.column < t.startColumn || n.lineNumber === t.endLineNumber && n.column > t.endColumn);
  }
  static strictContainsPosition(t, n) {
    return !(n.lineNumber < t.startLineNumber || n.lineNumber > t.endLineNumber || n.lineNumber === t.startLineNumber && n.column <= t.startColumn || n.lineNumber === t.endLineNumber && n.column >= t.endColumn);
  }
  containsRange(t) {
    return ce.containsRange(this, t);
  }
  static containsRange(t, n) {
    return !(n.startLineNumber < t.startLineNumber || n.endLineNumber < t.startLineNumber || n.startLineNumber > t.endLineNumber || n.endLineNumber > t.endLineNumber || n.startLineNumber === t.startLineNumber && n.startColumn < t.startColumn || n.endLineNumber === t.endLineNumber && n.endColumn > t.endColumn);
  }
  strictContainsRange(t) {
    return ce.strictContainsRange(this, t);
  }
  static strictContainsRange(t, n) {
    return !(n.startLineNumber < t.startLineNumber || n.endLineNumber < t.startLineNumber || n.startLineNumber > t.endLineNumber || n.endLineNumber > t.endLineNumber || n.startLineNumber === t.startLineNumber && n.startColumn <= t.startColumn || n.endLineNumber === t.endLineNumber && n.endColumn >= t.endColumn);
  }
  plusRange(t) {
    return ce.plusRange(this, t);
  }
  static plusRange(t, n) {
    let i, r, s, a;
    return n.startLineNumber < t.startLineNumber ? (i = n.startLineNumber, r = n.startColumn) : n.startLineNumber === t.startLineNumber ? (i = n.startLineNumber, r = Math.min(n.startColumn, t.startColumn)) : (i = t.startLineNumber, r = t.startColumn), n.endLineNumber > t.endLineNumber ? (s = n.endLineNumber, a = n.endColumn) : n.endLineNumber === t.endLineNumber ? (s = n.endLineNumber, a = Math.max(n.endColumn, t.endColumn)) : (s = t.endLineNumber, a = t.endColumn), new ce(i, r, s, a);
  }
  intersectRanges(t) {
    return ce.intersectRanges(this, t);
  }
  static intersectRanges(t, n) {
    let i = t.startLineNumber, r = t.startColumn, s = t.endLineNumber, a = t.endColumn;
    const l = n.startLineNumber, o = n.startColumn, u = n.endLineNumber, h = n.endColumn;
    return i < l ? (i = l, r = o) : i === l && (r = Math.max(r, o)), s > u ? (s = u, a = h) : s === u && (a = Math.min(a, h)), i > s || i === s && r > a ? null : new ce(
      i,
      r,
      s,
      a
    );
  }
  equalsRange(t) {
    return ce.equalsRange(this, t);
  }
  static equalsRange(t, n) {
    return !t && !n ? !0 : !!t && !!n && t.startLineNumber === n.startLineNumber && t.startColumn === n.startColumn && t.endLineNumber === n.endLineNumber && t.endColumn === n.endColumn;
  }
  getEndPosition() {
    return ce.getEndPosition(this);
  }
  static getEndPosition(t) {
    return new se(t.endLineNumber, t.endColumn);
  }
  getStartPosition() {
    return ce.getStartPosition(this);
  }
  static getStartPosition(t) {
    return new se(t.startLineNumber, t.startColumn);
  }
  toString() {
    return "[" + this.startLineNumber + "," + this.startColumn + " -> " + this.endLineNumber + "," + this.endColumn + "]";
  }
  setEndPosition(t, n) {
    return new ce(this.startLineNumber, this.startColumn, t, n);
  }
  setStartPosition(t, n) {
    return new ce(t, n, this.endLineNumber, this.endColumn);
  }
  collapseToStart() {
    return ce.collapseToStart(this);
  }
  static collapseToStart(t) {
    return new ce(
      t.startLineNumber,
      t.startColumn,
      t.startLineNumber,
      t.startColumn
    );
  }
  collapseToEnd() {
    return ce.collapseToEnd(this);
  }
  static collapseToEnd(t) {
    return new ce(t.endLineNumber, t.endColumn, t.endLineNumber, t.endColumn);
  }
  delta(t) {
    return new ce(
      this.startLineNumber + t,
      this.startColumn,
      this.endLineNumber + t,
      this.endColumn
    );
  }
  static fromPositions(t, n = t) {
    return new ce(t.lineNumber, t.column, n.lineNumber, n.column);
  }
  static lift(t) {
    return t ? new ce(
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
      const s = t.startLineNumber | 0, a = n.startLineNumber | 0;
      if (s === a) {
        const l = t.startColumn | 0, o = n.startColumn | 0;
        if (l === o) {
          const u = t.endLineNumber | 0, h = n.endLineNumber | 0;
          if (u === h) {
            const c = t.endColumn | 0, m = n.endColumn | 0;
            return c - m;
          }
          return u - h;
        }
        return l - o;
      }
      return s - a;
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
};
class mc {
  constructor(t) {
    this.values = t, this.prefixSum = new Uint32Array(t.length), this.prefixSumValidIndex = new Int32Array(1), this.prefixSumValidIndex[0] = -1;
  }
  getCount() {
    return this.values.length;
  }
  insertValues(t, n) {
    t = Rt(t);
    const i = this.values, r = this.prefixSum, s = n.length;
    return s === 0 ? !1 : (this.values = new Uint32Array(i.length + s), this.values.set(i.subarray(0, t), 0), this.values.set(i.subarray(t), t + s), this.values.set(n, t), t - 1 < this.prefixSumValidIndex[0] && (this.prefixSumValidIndex[0] = t - 1), this.prefixSum = new Uint32Array(this.values.length), this.prefixSumValidIndex[0] >= 0 && this.prefixSum.set(r.subarray(0, this.prefixSumValidIndex[0] + 1)), !0);
  }
  setValue(t, n) {
    return t = Rt(t), n = Rt(n), this.values[t] === n ? !1 : (this.values[t] = n, t - 1 < this.prefixSumValidIndex[0] && (this.prefixSumValidIndex[0] = t - 1), !0);
  }
  removeValues(t, n) {
    t = Rt(t), n = Rt(n);
    const i = this.values, r = this.prefixSum;
    if (t >= i.length)
      return !1;
    const s = i.length - t;
    return n >= s && (n = s), n === 0 ? !1 : (this.values = new Uint32Array(i.length - n), this.values.set(i.subarray(0, t), 0), this.values.set(i.subarray(t + n), t), this.prefixSum = new Uint32Array(this.values.length), t - 1 < this.prefixSumValidIndex[0] && (this.prefixSumValidIndex[0] = t - 1), this.prefixSumValidIndex[0] >= 0 && this.prefixSum.set(r.subarray(0, this.prefixSumValidIndex[0] + 1)), !0);
  }
  getTotalSum() {
    return this.values.length === 0 ? 0 : this._getPrefixSum(this.values.length - 1);
  }
  getPrefixSum(t) {
    return t < 0 ? 0 : (t = Rt(t), this._getPrefixSum(t));
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
    let n = 0, i = this.values.length - 1, r = 0, s = 0, a = 0;
    for (; n <= i; )
      if (r = n + (i - n) / 2 | 0, s = this.prefixSum[r], a = s - this.values[r], t < a)
        i = r - 1;
      else if (t >= s)
        n = r + 1;
      else
        break;
    return new gc(r, t - a);
  }
}
class gc {
  constructor(t, n) {
    this.index = t, this.remainder = n, this._prefixSumIndexOfResultBrand = void 0, this.index = t, this.remainder = n;
  }
}
class dc {
  constructor(t, n, i, r) {
    this._uri = t, this._lines = n, this._eol = i, this._versionId = r, this._lineStarts = null, this._cachedTextValue = null;
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
      this._acceptDeleteRange(i.range), this._acceptInsertText(new se(i.range.startLineNumber, i.range.startColumn), i.text);
    this._versionId = t.versionId, this._cachedTextValue = null;
  }
  _ensureLineStarts() {
    if (!this._lineStarts) {
      const t = this._eol.length, n = this._lines.length, i = new Uint32Array(n);
      for (let r = 0; r < n; r++)
        i[r] = this._lines[r].length + t;
      this._lineStarts = new mc(i);
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
    const i = Iu(n);
    if (i.length === 1) {
      this._setLineText(t.lineNumber - 1, this._lines[t.lineNumber - 1].substring(0, t.column - 1) + i[0] + this._lines[t.lineNumber - 1].substring(t.column - 1));
      return;
    }
    i[i.length - 1] += this._lines[t.lineNumber - 1].substring(t.column - 1), this._setLineText(t.lineNumber - 1, this._lines[t.lineNumber - 1].substring(0, t.column - 1) + i[0]);
    const r = new Uint32Array(i.length - 1);
    for (let s = 1; s < i.length; s++)
      this._lines.splice(t.lineNumber + s - 1, 0, i[s]), r[s - 1] = i[s].length + this._eol.length;
    this._lineStarts && this._lineStarts.insertValues(t.lineNumber, r);
  }
}
const pc = "`~!@#$%^&*()-=+[{]}\\|;:'\",.<>/?";
function bc(e = "") {
  let t = "(-?\\d*\\.\\d\\w*)|([^";
  for (const n of pc)
    e.indexOf(n) >= 0 || (t += "\\" + n);
  return t += "\\s]+)", new RegExp(t, "g");
}
const pl = bc();
function bl(e) {
  let t = pl;
  if (e && e instanceof RegExp)
    if (e.global)
      t = e;
    else {
      let n = "g";
      e.ignoreCase && (n += "i"), e.multiline && (n += "m"), e.unicode && (n += "u"), t = new RegExp(e.source, n);
    }
  return t.lastIndex = 0, t;
}
const _l = new fu();
_l.unshift({
  maxLen: 1e3,
  windowSize: 15,
  timeBudget: 150
});
function vr(e, t, n, i, r) {
  if (t = bl(t), r || (r = qn.first(_l)), n.length > r.maxLen) {
    let u = e - r.maxLen / 2;
    return u < 0 ? u = 0 : i += u, n = n.substring(u, e + r.maxLen / 2), vr(e, t, n, i, r);
  }
  const s = Date.now(), a = e - 1 - i;
  let l = -1, o = null;
  for (let u = 1; !(Date.now() - s >= r.timeBudget); u++) {
    const h = a - r.windowSize * u;
    t.lastIndex = Math.max(0, h);
    const c = _c(t, n, a, l);
    if (!c && o || (o = c, h <= 0))
      break;
    l = h;
  }
  if (o) {
    const u = {
      word: o[0],
      startColumn: i + 1 + o.index,
      endColumn: i + 1 + o.index + o[0].length
    };
    return t.lastIndex = 0, u;
  }
  return null;
}
function _c(e, t, n, i) {
  let r;
  for (; r = e.exec(t); ) {
    const s = r.index || 0;
    if (s <= n && e.lastIndex >= n)
      return r;
    if (i > 0 && s > i)
      return null;
  }
  return null;
}
class Lr {
  constructor(t) {
    const n = Fr(t);
    this._defaultValue = n, this._asciiMap = Lr._createAsciiMap(n), this._map = /* @__PURE__ */ new Map();
  }
  static _createAsciiMap(t) {
    const n = new Uint8Array(256);
    return n.fill(t), n;
  }
  set(t, n) {
    const i = Fr(n);
    t >= 0 && t < 256 ? this._asciiMap[t] = i : this._map.set(t, i);
  }
  get(t) {
    return t >= 0 && t < 256 ? this._asciiMap[t] : this._map.get(t) || this._defaultValue;
  }
  clear() {
    this._asciiMap.fill(this._defaultValue), this._map.clear();
  }
}
var zr;
(function(e) {
  e[e.False = 0] = "False", e[e.True = 1] = "True";
})(zr || (zr = {}));
var H;
(function(e) {
  e[e.Invalid = 0] = "Invalid", e[e.Start = 1] = "Start", e[e.H = 2] = "H", e[e.HT = 3] = "HT", e[e.HTT = 4] = "HTT", e[e.HTTP = 5] = "HTTP", e[e.F = 6] = "F", e[e.FI = 7] = "FI", e[e.FIL = 8] = "FIL", e[e.BeforeColon = 9] = "BeforeColon", e[e.AfterColon = 10] = "AfterColon", e[e.AlmostThere = 11] = "AlmostThere", e[e.End = 12] = "End", e[e.Accept = 13] = "Accept", e[e.LastKnownState = 14] = "LastKnownState";
})(H || (H = {}));
class vc {
  constructor(t, n, i) {
    const r = new Uint8Array(t * n);
    for (let s = 0, a = t * n; s < a; s++)
      r[s] = i;
    this._data = r, this.rows = t, this.cols = n;
  }
  get(t, n) {
    return this._data[t * this.cols + n];
  }
  set(t, n, i) {
    this._data[t * this.cols + n] = i;
  }
}
class Lc {
  constructor(t) {
    let n = 0, i = H.Invalid;
    for (let s = 0, a = t.length; s < a; s++) {
      const [l, o, u] = t[s];
      o > n && (n = o), l > i && (i = l), u > i && (i = u);
    }
    n++, i++;
    const r = new vc(i, n, H.Invalid);
    for (let s = 0, a = t.length; s < a; s++) {
      const [l, o, u] = t[s];
      r.set(l, o, u);
    }
    this._states = r, this._maxCharCode = n;
  }
  nextState(t, n) {
    return n < 0 || n >= this._maxCharCode ? H.Invalid : this._states.get(t, n);
  }
}
let di = null;
function Nc() {
  return di === null && (di = new Lc([
    [H.Start, S.h, H.H],
    [H.Start, S.H, H.H],
    [H.Start, S.f, H.F],
    [H.Start, S.F, H.F],
    [H.H, S.t, H.HT],
    [H.H, S.T, H.HT],
    [H.HT, S.t, H.HTT],
    [H.HT, S.T, H.HTT],
    [H.HTT, S.p, H.HTTP],
    [H.HTT, S.P, H.HTTP],
    [H.HTTP, S.s, H.BeforeColon],
    [H.HTTP, S.S, H.BeforeColon],
    [H.HTTP, S.Colon, H.AfterColon],
    [H.F, S.i, H.FI],
    [H.F, S.I, H.FI],
    [H.FI, S.l, H.FIL],
    [H.FI, S.L, H.FIL],
    [H.FIL, S.e, H.BeforeColon],
    [H.FIL, S.E, H.BeforeColon],
    [H.BeforeColon, S.Colon, H.AfterColon],
    [H.AfterColon, S.Slash, H.AlmostThere],
    [H.AlmostThere, S.Slash, H.End]
  ])), di;
}
var te;
(function(e) {
  e[e.None = 0] = "None", e[e.ForceTermination = 1] = "ForceTermination", e[e.CannotEndIn = 2] = "CannotEndIn";
})(te || (te = {}));
let en = null;
function wc() {
  if (en === null) {
    en = new Lr(te.None);
    const e = ` 	<>'"、。｡､，．：；‘〈「『〔（［｛｢｣｝］）〕』」〉’｀～…`;
    for (let n = 0; n < e.length; n++)
      en.set(e.charCodeAt(n), te.ForceTermination);
    const t = ".,;:";
    for (let n = 0; n < t.length; n++)
      en.set(t.charCodeAt(n), te.CannotEndIn);
  }
  return en;
}
class zn {
  static _createLink(t, n, i, r, s) {
    let a = s - 1;
    do {
      const l = n.charCodeAt(a);
      if (t.get(l) !== te.CannotEndIn)
        break;
      a--;
    } while (a > r);
    if (r > 0) {
      const l = n.charCodeAt(r - 1), o = n.charCodeAt(a);
      (l === S.OpenParen && o === S.CloseParen || l === S.OpenSquareBracket && o === S.CloseSquareBracket || l === S.OpenCurlyBrace && o === S.CloseCurlyBrace) && a--;
    }
    return {
      range: {
        startLineNumber: i,
        startColumn: r + 1,
        endLineNumber: i,
        endColumn: a + 2
      },
      url: n.substring(r, a + 1)
    };
  }
  static computeLinks(t, n = Nc()) {
    const i = wc(), r = [];
    for (let s = 1, a = t.getLineCount(); s <= a; s++) {
      const l = t.getLineContent(s), o = l.length;
      let u = 0, h = 0, c = 0, m = H.Start, g = !1, d = !1, p = !1, _ = !1;
      for (; u < o; ) {
        let x = !1;
        const R = l.charCodeAt(u);
        if (m === H.Accept) {
          let v;
          switch (R) {
            case S.OpenParen:
              g = !0, v = te.None;
              break;
            case S.CloseParen:
              v = g ? te.None : te.ForceTermination;
              break;
            case S.OpenSquareBracket:
              p = !0, d = !0, v = te.None;
              break;
            case S.CloseSquareBracket:
              p = !1, v = d ? te.None : te.ForceTermination;
              break;
            case S.OpenCurlyBrace:
              _ = !0, v = te.None;
              break;
            case S.CloseCurlyBrace:
              v = _ ? te.None : te.ForceTermination;
              break;
            case S.SingleQuote:
            case S.DoubleQuote:
            case S.BackTick:
              c === R ? v = te.ForceTermination : c === S.SingleQuote || c === S.DoubleQuote || c === S.BackTick ? v = te.None : v = te.ForceTermination;
              break;
            case S.Asterisk:
              v = c === S.Asterisk ? te.ForceTermination : te.None;
              break;
            case S.Pipe:
              v = c === S.Pipe ? te.ForceTermination : te.None;
              break;
            case S.Space:
              v = p ? te.None : te.ForceTermination;
              break;
            default:
              v = i.get(R);
          }
          v === te.ForceTermination && (r.push(zn._createLink(i, l, s, h, u)), x = !0);
        } else if (m === H.End) {
          let v;
          R === S.OpenSquareBracket ? (d = !0, v = te.None) : v = i.get(R), v === te.ForceTermination ? x = !0 : m = H.Accept;
        } else
          m = n.nextState(m, R), m === H.Invalid && (x = !0);
        x && (m = H.Start, g = !1, d = !1, _ = !1, h = u + 1, c = R), u++;
      }
      m === H.Accept && r.push(zn._createLink(i, l, s, h, o));
    }
    return r;
  }
}
function Ac(e) {
  return !e || typeof e.getLineCount != "function" || typeof e.getLineContent != "function" ? [] : zn.computeLinks(e);
}
const ui = class ui {
  constructor() {
    this._defaultValueSet = [
      ["true", "false"],
      ["True", "False"],
      ["Private", "Public", "Friend", "ReadOnly", "Partial", "Protected", "WriteOnly"],
      ["public", "protected", "private"]
    ];
  }
  navigateValueSet(t, n, i, r, s) {
    if (t && n) {
      const a = this.doNavigateValueSet(n, s);
      if (a)
        return {
          range: t,
          value: a
        };
    }
    if (i && r) {
      const a = this.doNavigateValueSet(r, s);
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
    let r = Number(t);
    const s = parseFloat(t);
    return !isNaN(r) && !isNaN(s) && r === s ? r === 0 && !n ? null : (r = Math.floor(r * i), r += n ? i : -i, String(r / i)) : null;
  }
  textReplace(t, n) {
    return this.valueSetsReplace(this._defaultValueSet, t, n);
  }
  valueSetsReplace(t, n, i) {
    let r = null;
    for (let s = 0, a = t.length; r === null && s < a; s++)
      r = this.valueSetReplace(t[s], n, i);
    return r;
  }
  valueSetReplace(t, n, i) {
    let r = t.indexOf(n);
    return r >= 0 ? (r += i ? 1 : -1, r < 0 ? r = t.length - 1 : r %= t.length, t[r]) : null;
  }
};
ui.INSTANCE = new ui();
let Fi = ui;
var N;
(function(e) {
  e[e.DependsOnKbLayout = -1] = "DependsOnKbLayout", e[e.Unknown = 0] = "Unknown", e[e.Backspace = 1] = "Backspace", e[e.Tab = 2] = "Tab", e[e.Enter = 3] = "Enter", e[e.Shift = 4] = "Shift", e[e.Ctrl = 5] = "Ctrl", e[e.Alt = 6] = "Alt", e[e.PauseBreak = 7] = "PauseBreak", e[e.CapsLock = 8] = "CapsLock", e[e.Escape = 9] = "Escape", e[e.Space = 10] = "Space", e[e.PageUp = 11] = "PageUp", e[e.PageDown = 12] = "PageDown", e[e.End = 13] = "End", e[e.Home = 14] = "Home", e[e.LeftArrow = 15] = "LeftArrow", e[e.UpArrow = 16] = "UpArrow", e[e.RightArrow = 17] = "RightArrow", e[e.DownArrow = 18] = "DownArrow", e[e.Insert = 19] = "Insert", e[e.Delete = 20] = "Delete", e[e.Digit0 = 21] = "Digit0", e[e.Digit1 = 22] = "Digit1", e[e.Digit2 = 23] = "Digit2", e[e.Digit3 = 24] = "Digit3", e[e.Digit4 = 25] = "Digit4", e[e.Digit5 = 26] = "Digit5", e[e.Digit6 = 27] = "Digit6", e[e.Digit7 = 28] = "Digit7", e[e.Digit8 = 29] = "Digit8", e[e.Digit9 = 30] = "Digit9", e[e.KeyA = 31] = "KeyA", e[e.KeyB = 32] = "KeyB", e[e.KeyC = 33] = "KeyC", e[e.KeyD = 34] = "KeyD", e[e.KeyE = 35] = "KeyE", e[e.KeyF = 36] = "KeyF", e[e.KeyG = 37] = "KeyG", e[e.KeyH = 38] = "KeyH", e[e.KeyI = 39] = "KeyI", e[e.KeyJ = 40] = "KeyJ", e[e.KeyK = 41] = "KeyK", e[e.KeyL = 42] = "KeyL", e[e.KeyM = 43] = "KeyM", e[e.KeyN = 44] = "KeyN", e[e.KeyO = 45] = "KeyO", e[e.KeyP = 46] = "KeyP", e[e.KeyQ = 47] = "KeyQ", e[e.KeyR = 48] = "KeyR", e[e.KeyS = 49] = "KeyS", e[e.KeyT = 50] = "KeyT", e[e.KeyU = 51] = "KeyU", e[e.KeyV = 52] = "KeyV", e[e.KeyW = 53] = "KeyW", e[e.KeyX = 54] = "KeyX", e[e.KeyY = 55] = "KeyY", e[e.KeyZ = 56] = "KeyZ", e[e.Meta = 57] = "Meta", e[e.ContextMenu = 58] = "ContextMenu", e[e.F1 = 59] = "F1", e[e.F2 = 60] = "F2", e[e.F3 = 61] = "F3", e[e.F4 = 62] = "F4", e[e.F5 = 63] = "F5", e[e.F6 = 64] = "F6", e[e.F7 = 65] = "F7", e[e.F8 = 66] = "F8", e[e.F9 = 67] = "F9", e[e.F10 = 68] = "F10", e[e.F11 = 69] = "F11", e[e.F12 = 70] = "F12", e[e.F13 = 71] = "F13", e[e.F14 = 72] = "F14", e[e.F15 = 73] = "F15", e[e.F16 = 74] = "F16", e[e.F17 = 75] = "F17", e[e.F18 = 76] = "F18", e[e.F19 = 77] = "F19", e[e.F20 = 78] = "F20", e[e.F21 = 79] = "F21", e[e.F22 = 80] = "F22", e[e.F23 = 81] = "F23", e[e.F24 = 82] = "F24", e[e.NumLock = 83] = "NumLock", e[e.ScrollLock = 84] = "ScrollLock", e[e.Semicolon = 85] = "Semicolon", e[e.Equal = 86] = "Equal", e[e.Comma = 87] = "Comma", e[e.Minus = 88] = "Minus", e[e.Period = 89] = "Period", e[e.Slash = 90] = "Slash", e[e.Backquote = 91] = "Backquote", e[e.BracketLeft = 92] = "BracketLeft", e[e.Backslash = 93] = "Backslash", e[e.BracketRight = 94] = "BracketRight", e[e.Quote = 95] = "Quote", e[e.OEM_8 = 96] = "OEM_8", e[e.IntlBackslash = 97] = "IntlBackslash", e[e.Numpad0 = 98] = "Numpad0", e[e.Numpad1 = 99] = "Numpad1", e[e.Numpad2 = 100] = "Numpad2", e[e.Numpad3 = 101] = "Numpad3", e[e.Numpad4 = 102] = "Numpad4", e[e.Numpad5 = 103] = "Numpad5", e[e.Numpad6 = 104] = "Numpad6", e[e.Numpad7 = 105] = "Numpad7", e[e.Numpad8 = 106] = "Numpad8", e[e.Numpad9 = 107] = "Numpad9", e[e.NumpadMultiply = 108] = "NumpadMultiply", e[e.NumpadAdd = 109] = "NumpadAdd", e[e.NUMPAD_SEPARATOR = 110] = "NUMPAD_SEPARATOR", e[e.NumpadSubtract = 111] = "NumpadSubtract", e[e.NumpadDecimal = 112] = "NumpadDecimal", e[e.NumpadDivide = 113] = "NumpadDivide", e[e.KEY_IN_COMPOSITION = 114] = "KEY_IN_COMPOSITION", e[e.ABNT_C1 = 115] = "ABNT_C1", e[e.ABNT_C2 = 116] = "ABNT_C2", e[e.AudioVolumeMute = 117] = "AudioVolumeMute", e[e.AudioVolumeUp = 118] = "AudioVolumeUp", e[e.AudioVolumeDown = 119] = "AudioVolumeDown", e[e.BrowserSearch = 120] = "BrowserSearch", e[e.BrowserHome = 121] = "BrowserHome", e[e.BrowserBack = 122] = "BrowserBack", e[e.BrowserForward = 123] = "BrowserForward", e[e.MediaTrackNext = 124] = "MediaTrackNext", e[e.MediaTrackPrevious = 125] = "MediaTrackPrevious", e[e.MediaStop = 126] = "MediaStop", e[e.MediaPlayPause = 127] = "MediaPlayPause", e[e.LaunchMediaPlayer = 128] = "LaunchMediaPlayer", e[e.LaunchMail = 129] = "LaunchMail", e[e.LaunchApp2 = 130] = "LaunchApp2", e[e.Clear = 131] = "Clear", e[e.MAX_VALUE = 132] = "MAX_VALUE";
})(N || (N = {}));
var w;
(function(e) {
  e[e.DependsOnKbLayout = -1] = "DependsOnKbLayout", e[e.None = 0] = "None", e[e.Hyper = 1] = "Hyper", e[e.Super = 2] = "Super", e[e.Fn = 3] = "Fn", e[e.FnLock = 4] = "FnLock", e[e.Suspend = 5] = "Suspend", e[e.Resume = 6] = "Resume", e[e.Turbo = 7] = "Turbo", e[e.Sleep = 8] = "Sleep", e[e.WakeUp = 9] = "WakeUp", e[e.KeyA = 10] = "KeyA", e[e.KeyB = 11] = "KeyB", e[e.KeyC = 12] = "KeyC", e[e.KeyD = 13] = "KeyD", e[e.KeyE = 14] = "KeyE", e[e.KeyF = 15] = "KeyF", e[e.KeyG = 16] = "KeyG", e[e.KeyH = 17] = "KeyH", e[e.KeyI = 18] = "KeyI", e[e.KeyJ = 19] = "KeyJ", e[e.KeyK = 20] = "KeyK", e[e.KeyL = 21] = "KeyL", e[e.KeyM = 22] = "KeyM", e[e.KeyN = 23] = "KeyN", e[e.KeyO = 24] = "KeyO", e[e.KeyP = 25] = "KeyP", e[e.KeyQ = 26] = "KeyQ", e[e.KeyR = 27] = "KeyR", e[e.KeyS = 28] = "KeyS", e[e.KeyT = 29] = "KeyT", e[e.KeyU = 30] = "KeyU", e[e.KeyV = 31] = "KeyV", e[e.KeyW = 32] = "KeyW", e[e.KeyX = 33] = "KeyX", e[e.KeyY = 34] = "KeyY", e[e.KeyZ = 35] = "KeyZ", e[e.Digit1 = 36] = "Digit1", e[e.Digit2 = 37] = "Digit2", e[e.Digit3 = 38] = "Digit3", e[e.Digit4 = 39] = "Digit4", e[e.Digit5 = 40] = "Digit5", e[e.Digit6 = 41] = "Digit6", e[e.Digit7 = 42] = "Digit7", e[e.Digit8 = 43] = "Digit8", e[e.Digit9 = 44] = "Digit9", e[e.Digit0 = 45] = "Digit0", e[e.Enter = 46] = "Enter", e[e.Escape = 47] = "Escape", e[e.Backspace = 48] = "Backspace", e[e.Tab = 49] = "Tab", e[e.Space = 50] = "Space", e[e.Minus = 51] = "Minus", e[e.Equal = 52] = "Equal", e[e.BracketLeft = 53] = "BracketLeft", e[e.BracketRight = 54] = "BracketRight", e[e.Backslash = 55] = "Backslash", e[e.IntlHash = 56] = "IntlHash", e[e.Semicolon = 57] = "Semicolon", e[e.Quote = 58] = "Quote", e[e.Backquote = 59] = "Backquote", e[e.Comma = 60] = "Comma", e[e.Period = 61] = "Period", e[e.Slash = 62] = "Slash", e[e.CapsLock = 63] = "CapsLock", e[e.F1 = 64] = "F1", e[e.F2 = 65] = "F2", e[e.F3 = 66] = "F3", e[e.F4 = 67] = "F4", e[e.F5 = 68] = "F5", e[e.F6 = 69] = "F6", e[e.F7 = 70] = "F7", e[e.F8 = 71] = "F8", e[e.F9 = 72] = "F9", e[e.F10 = 73] = "F10", e[e.F11 = 74] = "F11", e[e.F12 = 75] = "F12", e[e.PrintScreen = 76] = "PrintScreen", e[e.ScrollLock = 77] = "ScrollLock", e[e.Pause = 78] = "Pause", e[e.Insert = 79] = "Insert", e[e.Home = 80] = "Home", e[e.PageUp = 81] = "PageUp", e[e.Delete = 82] = "Delete", e[e.End = 83] = "End", e[e.PageDown = 84] = "PageDown", e[e.ArrowRight = 85] = "ArrowRight", e[e.ArrowLeft = 86] = "ArrowLeft", e[e.ArrowDown = 87] = "ArrowDown", e[e.ArrowUp = 88] = "ArrowUp", e[e.NumLock = 89] = "NumLock", e[e.NumpadDivide = 90] = "NumpadDivide", e[e.NumpadMultiply = 91] = "NumpadMultiply", e[e.NumpadSubtract = 92] = "NumpadSubtract", e[e.NumpadAdd = 93] = "NumpadAdd", e[e.NumpadEnter = 94] = "NumpadEnter", e[e.Numpad1 = 95] = "Numpad1", e[e.Numpad2 = 96] = "Numpad2", e[e.Numpad3 = 97] = "Numpad3", e[e.Numpad4 = 98] = "Numpad4", e[e.Numpad5 = 99] = "Numpad5", e[e.Numpad6 = 100] = "Numpad6", e[e.Numpad7 = 101] = "Numpad7", e[e.Numpad8 = 102] = "Numpad8", e[e.Numpad9 = 103] = "Numpad9", e[e.Numpad0 = 104] = "Numpad0", e[e.NumpadDecimal = 105] = "NumpadDecimal", e[e.IntlBackslash = 106] = "IntlBackslash", e[e.ContextMenu = 107] = "ContextMenu", e[e.Power = 108] = "Power", e[e.NumpadEqual = 109] = "NumpadEqual", e[e.F13 = 110] = "F13", e[e.F14 = 111] = "F14", e[e.F15 = 112] = "F15", e[e.F16 = 113] = "F16", e[e.F17 = 114] = "F17", e[e.F18 = 115] = "F18", e[e.F19 = 116] = "F19", e[e.F20 = 117] = "F20", e[e.F21 = 118] = "F21", e[e.F22 = 119] = "F22", e[e.F23 = 120] = "F23", e[e.F24 = 121] = "F24", e[e.Open = 122] = "Open", e[e.Help = 123] = "Help", e[e.Select = 124] = "Select", e[e.Again = 125] = "Again", e[e.Undo = 126] = "Undo", e[e.Cut = 127] = "Cut", e[e.Copy = 128] = "Copy", e[e.Paste = 129] = "Paste", e[e.Find = 130] = "Find", e[e.AudioVolumeMute = 131] = "AudioVolumeMute", e[e.AudioVolumeUp = 132] = "AudioVolumeUp", e[e.AudioVolumeDown = 133] = "AudioVolumeDown", e[e.NumpadComma = 134] = "NumpadComma", e[e.IntlRo = 135] = "IntlRo", e[e.KanaMode = 136] = "KanaMode", e[e.IntlYen = 137] = "IntlYen", e[e.Convert = 138] = "Convert", e[e.NonConvert = 139] = "NonConvert", e[e.Lang1 = 140] = "Lang1", e[e.Lang2 = 141] = "Lang2", e[e.Lang3 = 142] = "Lang3", e[e.Lang4 = 143] = "Lang4", e[e.Lang5 = 144] = "Lang5", e[e.Abort = 145] = "Abort", e[e.Props = 146] = "Props", e[e.NumpadParenLeft = 147] = "NumpadParenLeft", e[e.NumpadParenRight = 148] = "NumpadParenRight", e[e.NumpadBackspace = 149] = "NumpadBackspace", e[e.NumpadMemoryStore = 150] = "NumpadMemoryStore", e[e.NumpadMemoryRecall = 151] = "NumpadMemoryRecall", e[e.NumpadMemoryClear = 152] = "NumpadMemoryClear", e[e.NumpadMemoryAdd = 153] = "NumpadMemoryAdd", e[e.NumpadMemorySubtract = 154] = "NumpadMemorySubtract", e[e.NumpadClear = 155] = "NumpadClear", e[e.NumpadClearEntry = 156] = "NumpadClearEntry", e[e.ControlLeft = 157] = "ControlLeft", e[e.ShiftLeft = 158] = "ShiftLeft", e[e.AltLeft = 159] = "AltLeft", e[e.MetaLeft = 160] = "MetaLeft", e[e.ControlRight = 161] = "ControlRight", e[e.ShiftRight = 162] = "ShiftRight", e[e.AltRight = 163] = "AltRight", e[e.MetaRight = 164] = "MetaRight", e[e.BrightnessUp = 165] = "BrightnessUp", e[e.BrightnessDown = 166] = "BrightnessDown", e[e.MediaPlay = 167] = "MediaPlay", e[e.MediaRecord = 168] = "MediaRecord", e[e.MediaFastForward = 169] = "MediaFastForward", e[e.MediaRewind = 170] = "MediaRewind", e[e.MediaTrackNext = 171] = "MediaTrackNext", e[e.MediaTrackPrevious = 172] = "MediaTrackPrevious", e[e.MediaStop = 173] = "MediaStop", e[e.Eject = 174] = "Eject", e[e.MediaPlayPause = 175] = "MediaPlayPause", e[e.MediaSelect = 176] = "MediaSelect", e[e.LaunchMail = 177] = "LaunchMail", e[e.LaunchApp2 = 178] = "LaunchApp2", e[e.LaunchApp1 = 179] = "LaunchApp1", e[e.SelectTask = 180] = "SelectTask", e[e.LaunchScreenSaver = 181] = "LaunchScreenSaver", e[e.BrowserSearch = 182] = "BrowserSearch", e[e.BrowserHome = 183] = "BrowserHome", e[e.BrowserBack = 184] = "BrowserBack", e[e.BrowserForward = 185] = "BrowserForward", e[e.BrowserStop = 186] = "BrowserStop", e[e.BrowserRefresh = 187] = "BrowserRefresh", e[e.BrowserFavorites = 188] = "BrowserFavorites", e[e.ZoomToggle = 189] = "ZoomToggle", e[e.MailReply = 190] = "MailReply", e[e.MailForward = 191] = "MailForward", e[e.MailSend = 192] = "MailSend", e[e.MAX_VALUE = 193] = "MAX_VALUE";
})(w || (w = {}));
class Nr {
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
    return this._strToKeyCode[t.toLowerCase()] || N.Unknown;
  }
}
const On = new Nr(), Pi = new Nr(), Oi = new Nr(), xc = new Array(230), Ec = /* @__PURE__ */ Object.create(null), yc = /* @__PURE__ */ Object.create(null), Bi = [];
for (let e = 0; e <= w.MAX_VALUE; e++)
  N.DependsOnKbLayout;
for (let e = 0; e <= N.MAX_VALUE; e++)
  Bi[e] = w.DependsOnKbLayout;
(function() {
  const e = "", t = [
    [1, w.None, "None", N.Unknown, "unknown", 0, "VK_UNKNOWN", e, e],
    [1, w.Hyper, "Hyper", N.Unknown, e, 0, e, e, e],
    [1, w.Super, "Super", N.Unknown, e, 0, e, e, e],
    [1, w.Fn, "Fn", N.Unknown, e, 0, e, e, e],
    [1, w.FnLock, "FnLock", N.Unknown, e, 0, e, e, e],
    [1, w.Suspend, "Suspend", N.Unknown, e, 0, e, e, e],
    [1, w.Resume, "Resume", N.Unknown, e, 0, e, e, e],
    [1, w.Turbo, "Turbo", N.Unknown, e, 0, e, e, e],
    [1, w.Sleep, "Sleep", N.Unknown, e, 0, "VK_SLEEP", e, e],
    [1, w.WakeUp, "WakeUp", N.Unknown, e, 0, e, e, e],
    [0, w.KeyA, "KeyA", N.KeyA, "A", 65, "VK_A", e, e],
    [0, w.KeyB, "KeyB", N.KeyB, "B", 66, "VK_B", e, e],
    [0, w.KeyC, "KeyC", N.KeyC, "C", 67, "VK_C", e, e],
    [0, w.KeyD, "KeyD", N.KeyD, "D", 68, "VK_D", e, e],
    [0, w.KeyE, "KeyE", N.KeyE, "E", 69, "VK_E", e, e],
    [0, w.KeyF, "KeyF", N.KeyF, "F", 70, "VK_F", e, e],
    [0, w.KeyG, "KeyG", N.KeyG, "G", 71, "VK_G", e, e],
    [0, w.KeyH, "KeyH", N.KeyH, "H", 72, "VK_H", e, e],
    [0, w.KeyI, "KeyI", N.KeyI, "I", 73, "VK_I", e, e],
    [0, w.KeyJ, "KeyJ", N.KeyJ, "J", 74, "VK_J", e, e],
    [0, w.KeyK, "KeyK", N.KeyK, "K", 75, "VK_K", e, e],
    [0, w.KeyL, "KeyL", N.KeyL, "L", 76, "VK_L", e, e],
    [0, w.KeyM, "KeyM", N.KeyM, "M", 77, "VK_M", e, e],
    [0, w.KeyN, "KeyN", N.KeyN, "N", 78, "VK_N", e, e],
    [0, w.KeyO, "KeyO", N.KeyO, "O", 79, "VK_O", e, e],
    [0, w.KeyP, "KeyP", N.KeyP, "P", 80, "VK_P", e, e],
    [0, w.KeyQ, "KeyQ", N.KeyQ, "Q", 81, "VK_Q", e, e],
    [0, w.KeyR, "KeyR", N.KeyR, "R", 82, "VK_R", e, e],
    [0, w.KeyS, "KeyS", N.KeyS, "S", 83, "VK_S", e, e],
    [0, w.KeyT, "KeyT", N.KeyT, "T", 84, "VK_T", e, e],
    [0, w.KeyU, "KeyU", N.KeyU, "U", 85, "VK_U", e, e],
    [0, w.KeyV, "KeyV", N.KeyV, "V", 86, "VK_V", e, e],
    [0, w.KeyW, "KeyW", N.KeyW, "W", 87, "VK_W", e, e],
    [0, w.KeyX, "KeyX", N.KeyX, "X", 88, "VK_X", e, e],
    [0, w.KeyY, "KeyY", N.KeyY, "Y", 89, "VK_Y", e, e],
    [0, w.KeyZ, "KeyZ", N.KeyZ, "Z", 90, "VK_Z", e, e],
    [0, w.Digit1, "Digit1", N.Digit1, "1", 49, "VK_1", e, e],
    [0, w.Digit2, "Digit2", N.Digit2, "2", 50, "VK_2", e, e],
    [0, w.Digit3, "Digit3", N.Digit3, "3", 51, "VK_3", e, e],
    [0, w.Digit4, "Digit4", N.Digit4, "4", 52, "VK_4", e, e],
    [0, w.Digit5, "Digit5", N.Digit5, "5", 53, "VK_5", e, e],
    [0, w.Digit6, "Digit6", N.Digit6, "6", 54, "VK_6", e, e],
    [0, w.Digit7, "Digit7", N.Digit7, "7", 55, "VK_7", e, e],
    [0, w.Digit8, "Digit8", N.Digit8, "8", 56, "VK_8", e, e],
    [0, w.Digit9, "Digit9", N.Digit9, "9", 57, "VK_9", e, e],
    [0, w.Digit0, "Digit0", N.Digit0, "0", 48, "VK_0", e, e],
    [1, w.Enter, "Enter", N.Enter, "Enter", 13, "VK_RETURN", e, e],
    [1, w.Escape, "Escape", N.Escape, "Escape", 27, "VK_ESCAPE", e, e],
    [1, w.Backspace, "Backspace", N.Backspace, "Backspace", 8, "VK_BACK", e, e],
    [1, w.Tab, "Tab", N.Tab, "Tab", 9, "VK_TAB", e, e],
    [1, w.Space, "Space", N.Space, "Space", 32, "VK_SPACE", e, e],
    [0, w.Minus, "Minus", N.Minus, "-", 189, "VK_OEM_MINUS", "-", "OEM_MINUS"],
    [0, w.Equal, "Equal", N.Equal, "=", 187, "VK_OEM_PLUS", "=", "OEM_PLUS"],
    [0, w.BracketLeft, "BracketLeft", N.BracketLeft, "[", 219, "VK_OEM_4", "[", "OEM_4"],
    [0, w.BracketRight, "BracketRight", N.BracketRight, "]", 221, "VK_OEM_6", "]", "OEM_6"],
    [0, w.Backslash, "Backslash", N.Backslash, "\\", 220, "VK_OEM_5", "\\", "OEM_5"],
    [0, w.IntlHash, "IntlHash", N.Unknown, e, 0, e, e, e],
    [0, w.Semicolon, "Semicolon", N.Semicolon, ";", 186, "VK_OEM_1", ";", "OEM_1"],
    [0, w.Quote, "Quote", N.Quote, "'", 222, "VK_OEM_7", "'", "OEM_7"],
    [0, w.Backquote, "Backquote", N.Backquote, "`", 192, "VK_OEM_3", "`", "OEM_3"],
    [0, w.Comma, "Comma", N.Comma, ",", 188, "VK_OEM_COMMA", ",", "OEM_COMMA"],
    [0, w.Period, "Period", N.Period, ".", 190, "VK_OEM_PERIOD", ".", "OEM_PERIOD"],
    [0, w.Slash, "Slash", N.Slash, "/", 191, "VK_OEM_2", "/", "OEM_2"],
    [1, w.CapsLock, "CapsLock", N.CapsLock, "CapsLock", 20, "VK_CAPITAL", e, e],
    [1, w.F1, "F1", N.F1, "F1", 112, "VK_F1", e, e],
    [1, w.F2, "F2", N.F2, "F2", 113, "VK_F2", e, e],
    [1, w.F3, "F3", N.F3, "F3", 114, "VK_F3", e, e],
    [1, w.F4, "F4", N.F4, "F4", 115, "VK_F4", e, e],
    [1, w.F5, "F5", N.F5, "F5", 116, "VK_F5", e, e],
    [1, w.F6, "F6", N.F6, "F6", 117, "VK_F6", e, e],
    [1, w.F7, "F7", N.F7, "F7", 118, "VK_F7", e, e],
    [1, w.F8, "F8", N.F8, "F8", 119, "VK_F8", e, e],
    [1, w.F9, "F9", N.F9, "F9", 120, "VK_F9", e, e],
    [1, w.F10, "F10", N.F10, "F10", 121, "VK_F10", e, e],
    [1, w.F11, "F11", N.F11, "F11", 122, "VK_F11", e, e],
    [1, w.F12, "F12", N.F12, "F12", 123, "VK_F12", e, e],
    [1, w.PrintScreen, "PrintScreen", N.Unknown, e, 0, e, e, e],
    [1, w.ScrollLock, "ScrollLock", N.ScrollLock, "ScrollLock", 145, "VK_SCROLL", e, e],
    [1, w.Pause, "Pause", N.PauseBreak, "PauseBreak", 19, "VK_PAUSE", e, e],
    [1, w.Insert, "Insert", N.Insert, "Insert", 45, "VK_INSERT", e, e],
    [1, w.Home, "Home", N.Home, "Home", 36, "VK_HOME", e, e],
    [1, w.PageUp, "PageUp", N.PageUp, "PageUp", 33, "VK_PRIOR", e, e],
    [1, w.Delete, "Delete", N.Delete, "Delete", 46, "VK_DELETE", e, e],
    [1, w.End, "End", N.End, "End", 35, "VK_END", e, e],
    [1, w.PageDown, "PageDown", N.PageDown, "PageDown", 34, "VK_NEXT", e, e],
    [1, w.ArrowRight, "ArrowRight", N.RightArrow, "RightArrow", 39, "VK_RIGHT", "Right", e],
    [1, w.ArrowLeft, "ArrowLeft", N.LeftArrow, "LeftArrow", 37, "VK_LEFT", "Left", e],
    [1, w.ArrowDown, "ArrowDown", N.DownArrow, "DownArrow", 40, "VK_DOWN", "Down", e],
    [1, w.ArrowUp, "ArrowUp", N.UpArrow, "UpArrow", 38, "VK_UP", "Up", e],
    [1, w.NumLock, "NumLock", N.NumLock, "NumLock", 144, "VK_NUMLOCK", e, e],
    [1, w.NumpadDivide, "NumpadDivide", N.NumpadDivide, "NumPad_Divide", 111, "VK_DIVIDE", e, e],
    [1, w.NumpadMultiply, "NumpadMultiply", N.NumpadMultiply, "NumPad_Multiply", 106, "VK_MULTIPLY", e, e],
    [1, w.NumpadSubtract, "NumpadSubtract", N.NumpadSubtract, "NumPad_Subtract", 109, "VK_SUBTRACT", e, e],
    [1, w.NumpadAdd, "NumpadAdd", N.NumpadAdd, "NumPad_Add", 107, "VK_ADD", e, e],
    [1, w.NumpadEnter, "NumpadEnter", N.Enter, e, 0, e, e, e],
    [1, w.Numpad1, "Numpad1", N.Numpad1, "NumPad1", 97, "VK_NUMPAD1", e, e],
    [1, w.Numpad2, "Numpad2", N.Numpad2, "NumPad2", 98, "VK_NUMPAD2", e, e],
    [1, w.Numpad3, "Numpad3", N.Numpad3, "NumPad3", 99, "VK_NUMPAD3", e, e],
    [1, w.Numpad4, "Numpad4", N.Numpad4, "NumPad4", 100, "VK_NUMPAD4", e, e],
    [1, w.Numpad5, "Numpad5", N.Numpad5, "NumPad5", 101, "VK_NUMPAD5", e, e],
    [1, w.Numpad6, "Numpad6", N.Numpad6, "NumPad6", 102, "VK_NUMPAD6", e, e],
    [1, w.Numpad7, "Numpad7", N.Numpad7, "NumPad7", 103, "VK_NUMPAD7", e, e],
    [1, w.Numpad8, "Numpad8", N.Numpad8, "NumPad8", 104, "VK_NUMPAD8", e, e],
    [1, w.Numpad9, "Numpad9", N.Numpad9, "NumPad9", 105, "VK_NUMPAD9", e, e],
    [1, w.Numpad0, "Numpad0", N.Numpad0, "NumPad0", 96, "VK_NUMPAD0", e, e],
    [1, w.NumpadDecimal, "NumpadDecimal", N.NumpadDecimal, "NumPad_Decimal", 110, "VK_DECIMAL", e, e],
    [0, w.IntlBackslash, "IntlBackslash", N.IntlBackslash, "OEM_102", 226, "VK_OEM_102", e, e],
    [1, w.ContextMenu, "ContextMenu", N.ContextMenu, "ContextMenu", 93, e, e, e],
    [1, w.Power, "Power", N.Unknown, e, 0, e, e, e],
    [1, w.NumpadEqual, "NumpadEqual", N.Unknown, e, 0, e, e, e],
    [1, w.F13, "F13", N.F13, "F13", 124, "VK_F13", e, e],
    [1, w.F14, "F14", N.F14, "F14", 125, "VK_F14", e, e],
    [1, w.F15, "F15", N.F15, "F15", 126, "VK_F15", e, e],
    [1, w.F16, "F16", N.F16, "F16", 127, "VK_F16", e, e],
    [1, w.F17, "F17", N.F17, "F17", 128, "VK_F17", e, e],
    [1, w.F18, "F18", N.F18, "F18", 129, "VK_F18", e, e],
    [1, w.F19, "F19", N.F19, "F19", 130, "VK_F19", e, e],
    [1, w.F20, "F20", N.F20, "F20", 131, "VK_F20", e, e],
    [1, w.F21, "F21", N.F21, "F21", 132, "VK_F21", e, e],
    [1, w.F22, "F22", N.F22, "F22", 133, "VK_F22", e, e],
    [1, w.F23, "F23", N.F23, "F23", 134, "VK_F23", e, e],
    [1, w.F24, "F24", N.F24, "F24", 135, "VK_F24", e, e],
    [1, w.Open, "Open", N.Unknown, e, 0, e, e, e],
    [1, w.Help, "Help", N.Unknown, e, 0, e, e, e],
    [1, w.Select, "Select", N.Unknown, e, 0, e, e, e],
    [1, w.Again, "Again", N.Unknown, e, 0, e, e, e],
    [1, w.Undo, "Undo", N.Unknown, e, 0, e, e, e],
    [1, w.Cut, "Cut", N.Unknown, e, 0, e, e, e],
    [1, w.Copy, "Copy", N.Unknown, e, 0, e, e, e],
    [1, w.Paste, "Paste", N.Unknown, e, 0, e, e, e],
    [1, w.Find, "Find", N.Unknown, e, 0, e, e, e],
    [1, w.AudioVolumeMute, "AudioVolumeMute", N.AudioVolumeMute, "AudioVolumeMute", 173, "VK_VOLUME_MUTE", e, e],
    [1, w.AudioVolumeUp, "AudioVolumeUp", N.AudioVolumeUp, "AudioVolumeUp", 175, "VK_VOLUME_UP", e, e],
    [1, w.AudioVolumeDown, "AudioVolumeDown", N.AudioVolumeDown, "AudioVolumeDown", 174, "VK_VOLUME_DOWN", e, e],
    [1, w.NumpadComma, "NumpadComma", N.NUMPAD_SEPARATOR, "NumPad_Separator", 108, "VK_SEPARATOR", e, e],
    [0, w.IntlRo, "IntlRo", N.ABNT_C1, "ABNT_C1", 193, "VK_ABNT_C1", e, e],
    [1, w.KanaMode, "KanaMode", N.Unknown, e, 0, e, e, e],
    [0, w.IntlYen, "IntlYen", N.Unknown, e, 0, e, e, e],
    [1, w.Convert, "Convert", N.Unknown, e, 0, e, e, e],
    [1, w.NonConvert, "NonConvert", N.Unknown, e, 0, e, e, e],
    [1, w.Lang1, "Lang1", N.Unknown, e, 0, e, e, e],
    [1, w.Lang2, "Lang2", N.Unknown, e, 0, e, e, e],
    [1, w.Lang3, "Lang3", N.Unknown, e, 0, e, e, e],
    [1, w.Lang4, "Lang4", N.Unknown, e, 0, e, e, e],
    [1, w.Lang5, "Lang5", N.Unknown, e, 0, e, e, e],
    [1, w.Abort, "Abort", N.Unknown, e, 0, e, e, e],
    [1, w.Props, "Props", N.Unknown, e, 0, e, e, e],
    [1, w.NumpadParenLeft, "NumpadParenLeft", N.Unknown, e, 0, e, e, e],
    [1, w.NumpadParenRight, "NumpadParenRight", N.Unknown, e, 0, e, e, e],
    [1, w.NumpadBackspace, "NumpadBackspace", N.Unknown, e, 0, e, e, e],
    [1, w.NumpadMemoryStore, "NumpadMemoryStore", N.Unknown, e, 0, e, e, e],
    [1, w.NumpadMemoryRecall, "NumpadMemoryRecall", N.Unknown, e, 0, e, e, e],
    [1, w.NumpadMemoryClear, "NumpadMemoryClear", N.Unknown, e, 0, e, e, e],
    [1, w.NumpadMemoryAdd, "NumpadMemoryAdd", N.Unknown, e, 0, e, e, e],
    [1, w.NumpadMemorySubtract, "NumpadMemorySubtract", N.Unknown, e, 0, e, e, e],
    [1, w.NumpadClear, "NumpadClear", N.Clear, "Clear", 12, "VK_CLEAR", e, e],
    [1, w.NumpadClearEntry, "NumpadClearEntry", N.Unknown, e, 0, e, e, e],
    [1, w.None, e, N.Ctrl, "Ctrl", 17, "VK_CONTROL", e, e],
    [1, w.None, e, N.Shift, "Shift", 16, "VK_SHIFT", e, e],
    [1, w.None, e, N.Alt, "Alt", 18, "VK_MENU", e, e],
    [1, w.None, e, N.Meta, "Meta", 91, "VK_COMMAND", e, e],
    [1, w.ControlLeft, "ControlLeft", N.Ctrl, e, 0, "VK_LCONTROL", e, e],
    [1, w.ShiftLeft, "ShiftLeft", N.Shift, e, 0, "VK_LSHIFT", e, e],
    [1, w.AltLeft, "AltLeft", N.Alt, e, 0, "VK_LMENU", e, e],
    [1, w.MetaLeft, "MetaLeft", N.Meta, e, 0, "VK_LWIN", e, e],
    [1, w.ControlRight, "ControlRight", N.Ctrl, e, 0, "VK_RCONTROL", e, e],
    [1, w.ShiftRight, "ShiftRight", N.Shift, e, 0, "VK_RSHIFT", e, e],
    [1, w.AltRight, "AltRight", N.Alt, e, 0, "VK_RMENU", e, e],
    [1, w.MetaRight, "MetaRight", N.Meta, e, 0, "VK_RWIN", e, e],
    [1, w.BrightnessUp, "BrightnessUp", N.Unknown, e, 0, e, e, e],
    [1, w.BrightnessDown, "BrightnessDown", N.Unknown, e, 0, e, e, e],
    [1, w.MediaPlay, "MediaPlay", N.Unknown, e, 0, e, e, e],
    [1, w.MediaRecord, "MediaRecord", N.Unknown, e, 0, e, e, e],
    [1, w.MediaFastForward, "MediaFastForward", N.Unknown, e, 0, e, e, e],
    [1, w.MediaRewind, "MediaRewind", N.Unknown, e, 0, e, e, e],
    [1, w.MediaTrackNext, "MediaTrackNext", N.MediaTrackNext, "MediaTrackNext", 176, "VK_MEDIA_NEXT_TRACK", e, e],
    [1, w.MediaTrackPrevious, "MediaTrackPrevious", N.MediaTrackPrevious, "MediaTrackPrevious", 177, "VK_MEDIA_PREV_TRACK", e, e],
    [1, w.MediaStop, "MediaStop", N.MediaStop, "MediaStop", 178, "VK_MEDIA_STOP", e, e],
    [1, w.Eject, "Eject", N.Unknown, e, 0, e, e, e],
    [1, w.MediaPlayPause, "MediaPlayPause", N.MediaPlayPause, "MediaPlayPause", 179, "VK_MEDIA_PLAY_PAUSE", e, e],
    [1, w.MediaSelect, "MediaSelect", N.LaunchMediaPlayer, "LaunchMediaPlayer", 181, "VK_MEDIA_LAUNCH_MEDIA_SELECT", e, e],
    [1, w.LaunchMail, "LaunchMail", N.LaunchMail, "LaunchMail", 180, "VK_MEDIA_LAUNCH_MAIL", e, e],
    [1, w.LaunchApp2, "LaunchApp2", N.LaunchApp2, "LaunchApp2", 183, "VK_MEDIA_LAUNCH_APP2", e, e],
    [1, w.LaunchApp1, "LaunchApp1", N.Unknown, e, 0, "VK_MEDIA_LAUNCH_APP1", e, e],
    [1, w.SelectTask, "SelectTask", N.Unknown, e, 0, e, e, e],
    [1, w.LaunchScreenSaver, "LaunchScreenSaver", N.Unknown, e, 0, e, e, e],
    [1, w.BrowserSearch, "BrowserSearch", N.BrowserSearch, "BrowserSearch", 170, "VK_BROWSER_SEARCH", e, e],
    [1, w.BrowserHome, "BrowserHome", N.BrowserHome, "BrowserHome", 172, "VK_BROWSER_HOME", e, e],
    [1, w.BrowserBack, "BrowserBack", N.BrowserBack, "BrowserBack", 166, "VK_BROWSER_BACK", e, e],
    [1, w.BrowserForward, "BrowserForward", N.BrowserForward, "BrowserForward", 167, "VK_BROWSER_FORWARD", e, e],
    [1, w.BrowserStop, "BrowserStop", N.Unknown, e, 0, "VK_BROWSER_STOP", e, e],
    [1, w.BrowserRefresh, "BrowserRefresh", N.Unknown, e, 0, "VK_BROWSER_REFRESH", e, e],
    [1, w.BrowserFavorites, "BrowserFavorites", N.Unknown, e, 0, "VK_BROWSER_FAVORITES", e, e],
    [1, w.ZoomToggle, "ZoomToggle", N.Unknown, e, 0, e, e, e],
    [1, w.MailReply, "MailReply", N.Unknown, e, 0, e, e, e],
    [1, w.MailForward, "MailForward", N.Unknown, e, 0, e, e, e],
    [1, w.MailSend, "MailSend", N.Unknown, e, 0, e, e, e],
    [1, w.None, e, N.KEY_IN_COMPOSITION, "KeyInComposition", 229, e, e, e],
    [1, w.None, e, N.ABNT_C2, "ABNT_C2", 194, "VK_ABNT_C2", e, e],
    [1, w.None, e, N.OEM_8, "OEM_8", 223, "VK_OEM_8", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_KANA", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_HANGUL", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_JUNJA", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_FINAL", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_HANJA", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_KANJI", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_CONVERT", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_NONCONVERT", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_ACCEPT", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_MODECHANGE", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_SELECT", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_PRINT", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_EXECUTE", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_SNAPSHOT", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_HELP", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_APPS", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_PROCESSKEY", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_PACKET", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_DBE_SBCSCHAR", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_DBE_DBCSCHAR", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_ATTN", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_CRSEL", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_EXSEL", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_EREOF", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_PLAY", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_ZOOM", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_NONAME", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_PA1", e, e],
    [1, w.None, e, N.Unknown, e, 0, "VK_OEM_CLEAR", e, e]
  ], n = [], i = [];
  for (const r of t) {
    const [s, a, l, o, u, h, c, m, g] = r;
    if (i[a] || (i[a] = !0, Ec[l] = a, yc[l.toLowerCase()] = a, s && o !== N.Unknown && o !== N.Enter && o !== N.Ctrl && o !== N.Shift && o !== N.Alt && o !== N.Meta && (Bi[o] = a)), !n[o]) {
      if (n[o] = !0, !u)
        throw new Error(
          `String representation missing for key code ${o} around scan code ${l}`
        );
      On.define(o, u), Pi.define(o, m || u), Oi.define(o, g || m || u);
    }
    h && (xc[h] = o);
  }
  Bi[N.Enter] = w.Enter;
})();
var Xr;
(function(e) {
  function t(l) {
    return On.keyCodeToStr(l);
  }
  e.toString = t;
  function n(l) {
    return On.strToKeyCode(l);
  }
  e.fromString = n;
  function i(l) {
    return Pi.keyCodeToStr(l);
  }
  e.toUserSettingsUS = i;
  function r(l) {
    return Oi.keyCodeToStr(l);
  }
  e.toUserSettingsGeneral = r;
  function s(l) {
    return Pi.strToKeyCode(l) || Oi.strToKeyCode(l);
  }
  e.fromUserSettings = s;
  function a(l) {
    if (l >= N.Numpad0 && l <= N.NumpadDivide)
      return null;
    switch (l) {
      case N.UpArrow:
        return "Up";
      case N.DownArrow:
        return "Down";
      case N.LeftArrow:
        return "Left";
      case N.RightArrow:
        return "Right";
    }
    return On.keyCodeToStr(l);
  }
  e.toElectronAccelerator = a;
})(Xr || (Xr = {}));
var Pt;
(function(e) {
  e[e.CtrlCmd = 2048] = "CtrlCmd", e[e.Shift = 1024] = "Shift", e[e.Alt = 512] = "Alt", e[e.WinCtrl = 256] = "WinCtrl";
})(Pt || (Pt = {}));
function Rc(e, t) {
  const n = (t & 65535) << 16 >>> 0;
  return (e | n) >>> 0;
}
var ct;
(function(e) {
  e[e.LTR = 0] = "LTR", e[e.RTL = 1] = "RTL";
})(ct || (ct = {}));
class ke extends J {
  constructor(t, n, i, r) {
    super(t, n, i, r), this.selectionStartLineNumber = t, this.selectionStartColumn = n, this.positionLineNumber = i, this.positionColumn = r;
  }
  toString() {
    return "[" + this.selectionStartLineNumber + "," + this.selectionStartColumn + " -> " + this.positionLineNumber + "," + this.positionColumn + "]";
  }
  equalsSelection(t) {
    return ke.selectionsEqual(this, t);
  }
  static selectionsEqual(t, n) {
    return t.selectionStartLineNumber === n.selectionStartLineNumber && t.selectionStartColumn === n.selectionStartColumn && t.positionLineNumber === n.positionLineNumber && t.positionColumn === n.positionColumn;
  }
  getDirection() {
    return this.selectionStartLineNumber === this.startLineNumber && this.selectionStartColumn === this.startColumn ? ct.LTR : ct.RTL;
  }
  setEndPosition(t, n) {
    return this.getDirection() === ct.LTR ? new ke(this.startLineNumber, this.startColumn, t, n) : new ke(t, n, this.startLineNumber, this.startColumn);
  }
  getPosition() {
    return new se(this.positionLineNumber, this.positionColumn);
  }
  getSelectionStart() {
    return new se(this.selectionStartLineNumber, this.selectionStartColumn);
  }
  setStartPosition(t, n) {
    return this.getDirection() === ct.LTR ? new ke(t, n, this.endLineNumber, this.endColumn) : new ke(this.endLineNumber, this.endColumn, t, n);
  }
  static fromPositions(t, n = t) {
    return new ke(t.lineNumber, t.column, n.lineNumber, n.column);
  }
  static fromRange(t, n) {
    return n === ct.LTR ? new ke(
      t.startLineNumber,
      t.startColumn,
      t.endLineNumber,
      t.endColumn
    ) : new ke(
      t.endLineNumber,
      t.endColumn,
      t.startLineNumber,
      t.startColumn
    );
  }
  static liftSelection(t) {
    return new ke(
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
    for (let i = 0, r = t.length; i < r; i++)
      if (!this.selectionsEqual(t[i], n[i]))
        return !1;
    return !0;
  }
  static isISelection(t) {
    return t && typeof t.selectionStartLineNumber == "number" && typeof t.selectionStartColumn == "number" && typeof t.positionLineNumber == "number" && typeof t.positionColumn == "number";
  }
  static createWithDirection(t, n, i, r, s) {
    return s === ct.LTR ? new ke(t, n, i, r) : new ke(i, r, t, n);
  }
}
const Jr = /* @__PURE__ */ Object.create(null);
function f(e, t) {
  if (vu(t)) {
    const n = Jr[t];
    if (n === void 0)
      throw new Error(`${e} references an unknown codicon: ${t}`);
    t = n;
  }
  return Jr[e] = t, { id: e };
}
const kc = {
  add: f("add", 6e4),
  plus: f("plus", 6e4),
  gistNew: f("gist-new", 6e4),
  repoCreate: f("repo-create", 6e4),
  lightbulb: f("lightbulb", 60001),
  lightBulb: f("light-bulb", 60001),
  repo: f("repo", 60002),
  repoDelete: f("repo-delete", 60002),
  gistFork: f("gist-fork", 60003),
  repoForked: f("repo-forked", 60003),
  gitPullRequest: f("git-pull-request", 60004),
  gitPullRequestAbandoned: f("git-pull-request-abandoned", 60004),
  recordKeys: f("record-keys", 60005),
  keyboard: f("keyboard", 60005),
  tag: f("tag", 60006),
  gitPullRequestLabel: f("git-pull-request-label", 60006),
  tagAdd: f("tag-add", 60006),
  tagRemove: f("tag-remove", 60006),
  person: f("person", 60007),
  personFollow: f("person-follow", 60007),
  personOutline: f("person-outline", 60007),
  personFilled: f("person-filled", 60007),
  gitBranch: f("git-branch", 60008),
  gitBranchCreate: f("git-branch-create", 60008),
  gitBranchDelete: f("git-branch-delete", 60008),
  sourceControl: f("source-control", 60008),
  mirror: f("mirror", 60009),
  mirrorPublic: f("mirror-public", 60009),
  star: f("star", 60010),
  starAdd: f("star-add", 60010),
  starDelete: f("star-delete", 60010),
  starEmpty: f("star-empty", 60010),
  comment: f("comment", 60011),
  commentAdd: f("comment-add", 60011),
  alert: f("alert", 60012),
  warning: f("warning", 60012),
  search: f("search", 60013),
  searchSave: f("search-save", 60013),
  logOut: f("log-out", 60014),
  signOut: f("sign-out", 60014),
  logIn: f("log-in", 60015),
  signIn: f("sign-in", 60015),
  eye: f("eye", 60016),
  eyeUnwatch: f("eye-unwatch", 60016),
  eyeWatch: f("eye-watch", 60016),
  circleFilled: f("circle-filled", 60017),
  primitiveDot: f("primitive-dot", 60017),
  closeDirty: f("close-dirty", 60017),
  debugBreakpoint: f("debug-breakpoint", 60017),
  debugBreakpointDisabled: f("debug-breakpoint-disabled", 60017),
  debugHint: f("debug-hint", 60017),
  terminalDecorationSuccess: f("terminal-decoration-success", 60017),
  primitiveSquare: f("primitive-square", 60018),
  edit: f("edit", 60019),
  pencil: f("pencil", 60019),
  info: f("info", 60020),
  issueOpened: f("issue-opened", 60020),
  gistPrivate: f("gist-private", 60021),
  gitForkPrivate: f("git-fork-private", 60021),
  lock: f("lock", 60021),
  mirrorPrivate: f("mirror-private", 60021),
  close: f("close", 60022),
  removeClose: f("remove-close", 60022),
  x: f("x", 60022),
  repoSync: f("repo-sync", 60023),
  sync: f("sync", 60023),
  clone: f("clone", 60024),
  desktopDownload: f("desktop-download", 60024),
  beaker: f("beaker", 60025),
  microscope: f("microscope", 60025),
  vm: f("vm", 60026),
  deviceDesktop: f("device-desktop", 60026),
  file: f("file", 60027),
  fileText: f("file-text", 60027),
  more: f("more", 60028),
  ellipsis: f("ellipsis", 60028),
  kebabHorizontal: f("kebab-horizontal", 60028),
  mailReply: f("mail-reply", 60029),
  reply: f("reply", 60029),
  organization: f("organization", 60030),
  organizationFilled: f("organization-filled", 60030),
  organizationOutline: f("organization-outline", 60030),
  newFile: f("new-file", 60031),
  fileAdd: f("file-add", 60031),
  newFolder: f("new-folder", 60032),
  fileDirectoryCreate: f("file-directory-create", 60032),
  trash: f("trash", 60033),
  trashcan: f("trashcan", 60033),
  history: f("history", 60034),
  clock: f("clock", 60034),
  folder: f("folder", 60035),
  fileDirectory: f("file-directory", 60035),
  symbolFolder: f("symbol-folder", 60035),
  logoGithub: f("logo-github", 60036),
  markGithub: f("mark-github", 60036),
  github: f("github", 60036),
  terminal: f("terminal", 60037),
  console: f("console", 60037),
  repl: f("repl", 60037),
  zap: f("zap", 60038),
  symbolEvent: f("symbol-event", 60038),
  error: f("error", 60039),
  stop: f("stop", 60039),
  variable: f("variable", 60040),
  symbolVariable: f("symbol-variable", 60040),
  array: f("array", 60042),
  symbolArray: f("symbol-array", 60042),
  symbolModule: f("symbol-module", 60043),
  symbolPackage: f("symbol-package", 60043),
  symbolNamespace: f("symbol-namespace", 60043),
  symbolObject: f("symbol-object", 60043),
  symbolMethod: f("symbol-method", 60044),
  symbolFunction: f("symbol-function", 60044),
  symbolConstructor: f("symbol-constructor", 60044),
  symbolBoolean: f("symbol-boolean", 60047),
  symbolNull: f("symbol-null", 60047),
  symbolNumeric: f("symbol-numeric", 60048),
  symbolNumber: f("symbol-number", 60048),
  symbolStructure: f("symbol-structure", 60049),
  symbolStruct: f("symbol-struct", 60049),
  symbolParameter: f("symbol-parameter", 60050),
  symbolTypeParameter: f("symbol-type-parameter", 60050),
  symbolKey: f("symbol-key", 60051),
  symbolText: f("symbol-text", 60051),
  symbolReference: f("symbol-reference", 60052),
  goToFile: f("go-to-file", 60052),
  symbolEnum: f("symbol-enum", 60053),
  symbolValue: f("symbol-value", 60053),
  symbolRuler: f("symbol-ruler", 60054),
  symbolUnit: f("symbol-unit", 60054),
  activateBreakpoints: f("activate-breakpoints", 60055),
  archive: f("archive", 60056),
  arrowBoth: f("arrow-both", 60057),
  arrowDown: f("arrow-down", 60058),
  arrowLeft: f("arrow-left", 60059),
  arrowRight: f("arrow-right", 60060),
  arrowSmallDown: f("arrow-small-down", 60061),
  arrowSmallLeft: f("arrow-small-left", 60062),
  arrowSmallRight: f("arrow-small-right", 60063),
  arrowSmallUp: f("arrow-small-up", 60064),
  arrowUp: f("arrow-up", 60065),
  bell: f("bell", 60066),
  bold: f("bold", 60067),
  book: f("book", 60068),
  bookmark: f("bookmark", 60069),
  debugBreakpointConditionalUnverified: f("debug-breakpoint-conditional-unverified", 60070),
  debugBreakpointConditional: f("debug-breakpoint-conditional", 60071),
  debugBreakpointConditionalDisabled: f("debug-breakpoint-conditional-disabled", 60071),
  debugBreakpointDataUnverified: f("debug-breakpoint-data-unverified", 60072),
  debugBreakpointData: f("debug-breakpoint-data", 60073),
  debugBreakpointDataDisabled: f("debug-breakpoint-data-disabled", 60073),
  debugBreakpointLogUnverified: f("debug-breakpoint-log-unverified", 60074),
  debugBreakpointLog: f("debug-breakpoint-log", 60075),
  debugBreakpointLogDisabled: f("debug-breakpoint-log-disabled", 60075),
  briefcase: f("briefcase", 60076),
  broadcast: f("broadcast", 60077),
  browser: f("browser", 60078),
  bug: f("bug", 60079),
  calendar: f("calendar", 60080),
  caseSensitive: f("case-sensitive", 60081),
  check: f("check", 60082),
  checklist: f("checklist", 60083),
  chevronDown: f("chevron-down", 60084),
  chevronLeft: f("chevron-left", 60085),
  chevronRight: f("chevron-right", 60086),
  chevronUp: f("chevron-up", 60087),
  chromeClose: f("chrome-close", 60088),
  chromeMaximize: f("chrome-maximize", 60089),
  chromeMinimize: f("chrome-minimize", 60090),
  chromeRestore: f("chrome-restore", 60091),
  circleOutline: f("circle-outline", 60092),
  circle: f("circle", 60092),
  debugBreakpointUnverified: f("debug-breakpoint-unverified", 60092),
  terminalDecorationIncomplete: f("terminal-decoration-incomplete", 60092),
  circleSlash: f("circle-slash", 60093),
  circuitBoard: f("circuit-board", 60094),
  clearAll: f("clear-all", 60095),
  clippy: f("clippy", 60096),
  closeAll: f("close-all", 60097),
  cloudDownload: f("cloud-download", 60098),
  cloudUpload: f("cloud-upload", 60099),
  code: f("code", 60100),
  collapseAll: f("collapse-all", 60101),
  colorMode: f("color-mode", 60102),
  commentDiscussion: f("comment-discussion", 60103),
  creditCard: f("credit-card", 60105),
  dash: f("dash", 60108),
  dashboard: f("dashboard", 60109),
  database: f("database", 60110),
  debugContinue: f("debug-continue", 60111),
  debugDisconnect: f("debug-disconnect", 60112),
  debugPause: f("debug-pause", 60113),
  debugRestart: f("debug-restart", 60114),
  debugStart: f("debug-start", 60115),
  debugStepInto: f("debug-step-into", 60116),
  debugStepOut: f("debug-step-out", 60117),
  debugStepOver: f("debug-step-over", 60118),
  debugStop: f("debug-stop", 60119),
  debug: f("debug", 60120),
  deviceCameraVideo: f("device-camera-video", 60121),
  deviceCamera: f("device-camera", 60122),
  deviceMobile: f("device-mobile", 60123),
  diffAdded: f("diff-added", 60124),
  diffIgnored: f("diff-ignored", 60125),
  diffModified: f("diff-modified", 60126),
  diffRemoved: f("diff-removed", 60127),
  diffRenamed: f("diff-renamed", 60128),
  diff: f("diff", 60129),
  diffSidebyside: f("diff-sidebyside", 60129),
  discard: f("discard", 60130),
  editorLayout: f("editor-layout", 60131),
  emptyWindow: f("empty-window", 60132),
  exclude: f("exclude", 60133),
  extensions: f("extensions", 60134),
  eyeClosed: f("eye-closed", 60135),
  fileBinary: f("file-binary", 60136),
  fileCode: f("file-code", 60137),
  fileMedia: f("file-media", 60138),
  filePdf: f("file-pdf", 60139),
  fileSubmodule: f("file-submodule", 60140),
  fileSymlinkDirectory: f("file-symlink-directory", 60141),
  fileSymlinkFile: f("file-symlink-file", 60142),
  fileZip: f("file-zip", 60143),
  files: f("files", 60144),
  filter: f("filter", 60145),
  flame: f("flame", 60146),
  foldDown: f("fold-down", 60147),
  foldUp: f("fold-up", 60148),
  fold: f("fold", 60149),
  folderActive: f("folder-active", 60150),
  folderOpened: f("folder-opened", 60151),
  gear: f("gear", 60152),
  gift: f("gift", 60153),
  gistSecret: f("gist-secret", 60154),
  gist: f("gist", 60155),
  gitCommit: f("git-commit", 60156),
  gitCompare: f("git-compare", 60157),
  compareChanges: f("compare-changes", 60157),
  gitMerge: f("git-merge", 60158),
  githubAction: f("github-action", 60159),
  githubAlt: f("github-alt", 60160),
  globe: f("globe", 60161),
  grabber: f("grabber", 60162),
  graph: f("graph", 60163),
  gripper: f("gripper", 60164),
  heart: f("heart", 60165),
  home: f("home", 60166),
  horizontalRule: f("horizontal-rule", 60167),
  hubot: f("hubot", 60168),
  inbox: f("inbox", 60169),
  issueReopened: f("issue-reopened", 60171),
  issues: f("issues", 60172),
  italic: f("italic", 60173),
  jersey: f("jersey", 60174),
  json: f("json", 60175),
  kebabVertical: f("kebab-vertical", 60176),
  key: f("key", 60177),
  law: f("law", 60178),
  lightbulbAutofix: f("lightbulb-autofix", 60179),
  linkExternal: f("link-external", 60180),
  link: f("link", 60181),
  listOrdered: f("list-ordered", 60182),
  listUnordered: f("list-unordered", 60183),
  liveShare: f("live-share", 60184),
  loading: f("loading", 60185),
  location: f("location", 60186),
  mailRead: f("mail-read", 60187),
  mail: f("mail", 60188),
  markdown: f("markdown", 60189),
  megaphone: f("megaphone", 60190),
  mention: f("mention", 60191),
  milestone: f("milestone", 60192),
  gitPullRequestMilestone: f("git-pull-request-milestone", 60192),
  mortarBoard: f("mortar-board", 60193),
  move: f("move", 60194),
  multipleWindows: f("multiple-windows", 60195),
  mute: f("mute", 60196),
  noNewline: f("no-newline", 60197),
  note: f("note", 60198),
  octoface: f("octoface", 60199),
  openPreview: f("open-preview", 60200),
  package: f("package", 60201),
  paintcan: f("paintcan", 60202),
  pin: f("pin", 60203),
  play: f("play", 60204),
  run: f("run", 60204),
  plug: f("plug", 60205),
  preserveCase: f("preserve-case", 60206),
  preview: f("preview", 60207),
  project: f("project", 60208),
  pulse: f("pulse", 60209),
  question: f("question", 60210),
  quote: f("quote", 60211),
  radioTower: f("radio-tower", 60212),
  reactions: f("reactions", 60213),
  references: f("references", 60214),
  refresh: f("refresh", 60215),
  regex: f("regex", 60216),
  remoteExplorer: f("remote-explorer", 60217),
  remote: f("remote", 60218),
  remove: f("remove", 60219),
  replaceAll: f("replace-all", 60220),
  replace: f("replace", 60221),
  repoClone: f("repo-clone", 60222),
  repoForcePush: f("repo-force-push", 60223),
  repoPull: f("repo-pull", 60224),
  repoPush: f("repo-push", 60225),
  report: f("report", 60226),
  requestChanges: f("request-changes", 60227),
  rocket: f("rocket", 60228),
  rootFolderOpened: f("root-folder-opened", 60229),
  rootFolder: f("root-folder", 60230),
  rss: f("rss", 60231),
  ruby: f("ruby", 60232),
  saveAll: f("save-all", 60233),
  saveAs: f("save-as", 60234),
  save: f("save", 60235),
  screenFull: f("screen-full", 60236),
  screenNormal: f("screen-normal", 60237),
  searchStop: f("search-stop", 60238),
  server: f("server", 60240),
  settingsGear: f("settings-gear", 60241),
  settings: f("settings", 60242),
  shield: f("shield", 60243),
  smiley: f("smiley", 60244),
  sortPrecedence: f("sort-precedence", 60245),
  splitHorizontal: f("split-horizontal", 60246),
  splitVertical: f("split-vertical", 60247),
  squirrel: f("squirrel", 60248),
  starFull: f("star-full", 60249),
  starHalf: f("star-half", 60250),
  symbolClass: f("symbol-class", 60251),
  symbolColor: f("symbol-color", 60252),
  symbolConstant: f("symbol-constant", 60253),
  symbolEnumMember: f("symbol-enum-member", 60254),
  symbolField: f("symbol-field", 60255),
  symbolFile: f("symbol-file", 60256),
  symbolInterface: f("symbol-interface", 60257),
  symbolKeyword: f("symbol-keyword", 60258),
  symbolMisc: f("symbol-misc", 60259),
  symbolOperator: f("symbol-operator", 60260),
  symbolProperty: f("symbol-property", 60261),
  wrench: f("wrench", 60261),
  wrenchSubaction: f("wrench-subaction", 60261),
  symbolSnippet: f("symbol-snippet", 60262),
  tasklist: f("tasklist", 60263),
  telescope: f("telescope", 60264),
  textSize: f("text-size", 60265),
  threeBars: f("three-bars", 60266),
  thumbsdown: f("thumbsdown", 60267),
  thumbsup: f("thumbsup", 60268),
  tools: f("tools", 60269),
  triangleDown: f("triangle-down", 60270),
  triangleLeft: f("triangle-left", 60271),
  triangleRight: f("triangle-right", 60272),
  triangleUp: f("triangle-up", 60273),
  twitter: f("twitter", 60274),
  unfold: f("unfold", 60275),
  unlock: f("unlock", 60276),
  unmute: f("unmute", 60277),
  unverified: f("unverified", 60278),
  verified: f("verified", 60279),
  versions: f("versions", 60280),
  vmActive: f("vm-active", 60281),
  vmOutline: f("vm-outline", 60282),
  vmRunning: f("vm-running", 60283),
  watch: f("watch", 60284),
  whitespace: f("whitespace", 60285),
  wholeWord: f("whole-word", 60286),
  window: f("window", 60287),
  wordWrap: f("word-wrap", 60288),
  zoomIn: f("zoom-in", 60289),
  zoomOut: f("zoom-out", 60290),
  listFilter: f("list-filter", 60291),
  listFlat: f("list-flat", 60292),
  listSelection: f("list-selection", 60293),
  selection: f("selection", 60293),
  listTree: f("list-tree", 60294),
  debugBreakpointFunctionUnverified: f("debug-breakpoint-function-unverified", 60295),
  debugBreakpointFunction: f("debug-breakpoint-function", 60296),
  debugBreakpointFunctionDisabled: f("debug-breakpoint-function-disabled", 60296),
  debugStackframeActive: f("debug-stackframe-active", 60297),
  circleSmallFilled: f("circle-small-filled", 60298),
  debugStackframeDot: f("debug-stackframe-dot", 60298),
  terminalDecorationMark: f("terminal-decoration-mark", 60298),
  debugStackframe: f("debug-stackframe", 60299),
  debugStackframeFocused: f("debug-stackframe-focused", 60299),
  debugBreakpointUnsupported: f("debug-breakpoint-unsupported", 60300),
  symbolString: f("symbol-string", 60301),
  debugReverseContinue: f("debug-reverse-continue", 60302),
  debugStepBack: f("debug-step-back", 60303),
  debugRestartFrame: f("debug-restart-frame", 60304),
  debugAlt: f("debug-alt", 60305),
  callIncoming: f("call-incoming", 60306),
  callOutgoing: f("call-outgoing", 60307),
  menu: f("menu", 60308),
  expandAll: f("expand-all", 60309),
  feedback: f("feedback", 60310),
  gitPullRequestReviewer: f("git-pull-request-reviewer", 60310),
  groupByRefType: f("group-by-ref-type", 60311),
  ungroupByRefType: f("ungroup-by-ref-type", 60312),
  account: f("account", 60313),
  gitPullRequestAssignee: f("git-pull-request-assignee", 60313),
  bellDot: f("bell-dot", 60314),
  debugConsole: f("debug-console", 60315),
  library: f("library", 60316),
  output: f("output", 60317),
  runAll: f("run-all", 60318),
  syncIgnored: f("sync-ignored", 60319),
  pinned: f("pinned", 60320),
  githubInverted: f("github-inverted", 60321),
  serverProcess: f("server-process", 60322),
  serverEnvironment: f("server-environment", 60323),
  pass: f("pass", 60324),
  issueClosed: f("issue-closed", 60324),
  stopCircle: f("stop-circle", 60325),
  playCircle: f("play-circle", 60326),
  record: f("record", 60327),
  debugAltSmall: f("debug-alt-small", 60328),
  vmConnect: f("vm-connect", 60329),
  cloud: f("cloud", 60330),
  merge: f("merge", 60331),
  export: f("export", 60332),
  graphLeft: f("graph-left", 60333),
  magnet: f("magnet", 60334),
  notebook: f("notebook", 60335),
  redo: f("redo", 60336),
  checkAll: f("check-all", 60337),
  pinnedDirty: f("pinned-dirty", 60338),
  passFilled: f("pass-filled", 60339),
  circleLargeFilled: f("circle-large-filled", 60340),
  circleLarge: f("circle-large", 60341),
  circleLargeOutline: f("circle-large-outline", 60341),
  combine: f("combine", 60342),
  gather: f("gather", 60342),
  table: f("table", 60343),
  variableGroup: f("variable-group", 60344),
  typeHierarchy: f("type-hierarchy", 60345),
  typeHierarchySub: f("type-hierarchy-sub", 60346),
  typeHierarchySuper: f("type-hierarchy-super", 60347),
  gitPullRequestCreate: f("git-pull-request-create", 60348),
  runAbove: f("run-above", 60349),
  runBelow: f("run-below", 60350),
  notebookTemplate: f("notebook-template", 60351),
  debugRerun: f("debug-rerun", 60352),
  workspaceTrusted: f("workspace-trusted", 60353),
  workspaceUntrusted: f("workspace-untrusted", 60354),
  workspaceUnknown: f("workspace-unknown", 60355),
  terminalCmd: f("terminal-cmd", 60356),
  terminalDebian: f("terminal-debian", 60357),
  terminalLinux: f("terminal-linux", 60358),
  terminalPowershell: f("terminal-powershell", 60359),
  terminalTmux: f("terminal-tmux", 60360),
  terminalUbuntu: f("terminal-ubuntu", 60361),
  terminalBash: f("terminal-bash", 60362),
  arrowSwap: f("arrow-swap", 60363),
  copy: f("copy", 60364),
  personAdd: f("person-add", 60365),
  filterFilled: f("filter-filled", 60366),
  wand: f("wand", 60367),
  debugLineByLine: f("debug-line-by-line", 60368),
  inspect: f("inspect", 60369),
  layers: f("layers", 60370),
  layersDot: f("layers-dot", 60371),
  layersActive: f("layers-active", 60372),
  compass: f("compass", 60373),
  compassDot: f("compass-dot", 60374),
  compassActive: f("compass-active", 60375),
  azure: f("azure", 60376),
  issueDraft: f("issue-draft", 60377),
  gitPullRequestClosed: f("git-pull-request-closed", 60378),
  gitPullRequestDraft: f("git-pull-request-draft", 60379),
  debugAll: f("debug-all", 60380),
  debugCoverage: f("debug-coverage", 60381),
  runErrors: f("run-errors", 60382),
  folderLibrary: f("folder-library", 60383),
  debugContinueSmall: f("debug-continue-small", 60384),
  beakerStop: f("beaker-stop", 60385),
  graphLine: f("graph-line", 60386),
  graphScatter: f("graph-scatter", 60387),
  pieChart: f("pie-chart", 60388),
  bracket: f("bracket", 60175),
  bracketDot: f("bracket-dot", 60389),
  bracketError: f("bracket-error", 60390),
  lockSmall: f("lock-small", 60391),
  azureDevops: f("azure-devops", 60392),
  verifiedFilled: f("verified-filled", 60393),
  newline: f("newline", 60394),
  layout: f("layout", 60395),
  layoutActivitybarLeft: f("layout-activitybar-left", 60396),
  layoutActivitybarRight: f("layout-activitybar-right", 60397),
  layoutPanelLeft: f("layout-panel-left", 60398),
  layoutPanelCenter: f("layout-panel-center", 60399),
  layoutPanelJustify: f("layout-panel-justify", 60400),
  layoutPanelRight: f("layout-panel-right", 60401),
  layoutPanel: f("layout-panel", 60402),
  layoutSidebarLeft: f("layout-sidebar-left", 60403),
  layoutSidebarRight: f("layout-sidebar-right", 60404),
  layoutStatusbar: f("layout-statusbar", 60405),
  layoutMenubar: f("layout-menubar", 60406),
  layoutCentered: f("layout-centered", 60407),
  target: f("target", 60408),
  indent: f("indent", 60409),
  recordSmall: f("record-small", 60410),
  errorSmall: f("error-small", 60411),
  terminalDecorationError: f("terminal-decoration-error", 60411),
  arrowCircleDown: f("arrow-circle-down", 60412),
  arrowCircleLeft: f("arrow-circle-left", 60413),
  arrowCircleRight: f("arrow-circle-right", 60414),
  arrowCircleUp: f("arrow-circle-up", 60415),
  layoutSidebarRightOff: f("layout-sidebar-right-off", 60416),
  layoutPanelOff: f("layout-panel-off", 60417),
  layoutSidebarLeftOff: f("layout-sidebar-left-off", 60418),
  blank: f("blank", 60419),
  heartFilled: f("heart-filled", 60420),
  map: f("map", 60421),
  mapHorizontal: f("map-horizontal", 60421),
  foldHorizontal: f("fold-horizontal", 60421),
  mapFilled: f("map-filled", 60422),
  mapHorizontalFilled: f("map-horizontal-filled", 60422),
  foldHorizontalFilled: f("fold-horizontal-filled", 60422),
  circleSmall: f("circle-small", 60423),
  bellSlash: f("bell-slash", 60424),
  bellSlashDot: f("bell-slash-dot", 60425),
  commentUnresolved: f("comment-unresolved", 60426),
  gitPullRequestGoToChanges: f("git-pull-request-go-to-changes", 60427),
  gitPullRequestNewChanges: f("git-pull-request-new-changes", 60428),
  searchFuzzy: f("search-fuzzy", 60429),
  commentDraft: f("comment-draft", 60430),
  send: f("send", 60431),
  sparkle: f("sparkle", 60432),
  insert: f("insert", 60433),
  mic: f("mic", 60434),
  thumbsdownFilled: f("thumbsdown-filled", 60435),
  thumbsupFilled: f("thumbsup-filled", 60436),
  coffee: f("coffee", 60437),
  snake: f("snake", 60438),
  game: f("game", 60439),
  vr: f("vr", 60440),
  chip: f("chip", 60441),
  piano: f("piano", 60442),
  music: f("music", 60443),
  micFilled: f("mic-filled", 60444),
  repoFetch: f("repo-fetch", 60445),
  copilot: f("copilot", 60446),
  lightbulbSparkle: f("lightbulb-sparkle", 60447),
  robot: f("robot", 60448),
  sparkleFilled: f("sparkle-filled", 60449),
  diffSingle: f("diff-single", 60450),
  diffMultiple: f("diff-multiple", 60451),
  surroundWith: f("surround-with", 60452),
  share: f("share", 60453),
  gitStash: f("git-stash", 60454),
  gitStashApply: f("git-stash-apply", 60455),
  gitStashPop: f("git-stash-pop", 60456),
  vscode: f("vscode", 60457),
  vscodeInsiders: f("vscode-insiders", 60458),
  codeOss: f("code-oss", 60459),
  runCoverage: f("run-coverage", 60460),
  runAllCoverage: f("run-all-coverage", 60461),
  coverage: f("coverage", 60462),
  githubProject: f("github-project", 60463),
  mapVertical: f("map-vertical", 60464),
  foldVertical: f("fold-vertical", 60464),
  mapVerticalFilled: f("map-vertical-filled", 60465),
  foldVerticalFilled: f("fold-vertical-filled", 60465),
  goToSearch: f("go-to-search", 60466),
  percentage: f("percentage", 60467),
  sortPercentage: f("sort-percentage", 60467),
  attach: f("attach", 60468)
}, Tc = {
  dialogError: f("dialog-error", "error"),
  dialogWarning: f("dialog-warning", "warning"),
  dialogInfo: f("dialog-info", "info"),
  dialogClose: f("dialog-close", "close"),
  treeItemExpanded: f("tree-item-expanded", "chevron-down"),
  treeFilterOnTypeOn: f("tree-filter-on-type-on", "list-filter"),
  treeFilterOnTypeOff: f("tree-filter-on-type-off", "list-selection"),
  treeFilterClear: f("tree-filter-clear", "close"),
  treeItemLoading: f("tree-item-loading", "loading"),
  menuSelection: f("menu-selection", "check"),
  menuSubmenu: f("menu-submenu", "chevron-right"),
  menuBarMore: f("menubar-more", "more"),
  scrollbarButtonLeft: f("scrollbar-button-left", "triangle-left"),
  scrollbarButtonRight: f("scrollbar-button-right", "triangle-right"),
  scrollbarButtonUp: f("scrollbar-button-up", "triangle-up"),
  scrollbarButtonDown: f("scrollbar-button-down", "triangle-down"),
  toolBarMore: f("toolbar-more", "more"),
  quickInputBack: f("quick-input-back", "arrow-left"),
  dropDownButton: f("drop-down-button", 60084),
  symbolCustomColor: f("symbol-customcolor", 60252),
  exportIcon: f("export", 60332),
  workspaceUnspecified: f("workspace-unspecified", 60355),
  newLine: f("newline", 60394),
  thumbsDownFilled: f("thumbsdown-filled", 60435),
  thumbsUpFilled: f("thumbsup-filled", 60436),
  gitFetch: f("git-fetch", 60445),
  lightbulbSparkleAutofix: f("lightbulb-sparkle-autofix", 60447),
  debugBreakpointPending: f("debug-breakpoint-pending", 60377)
}, q = {
  ...kc,
  ...Tc
};
var Yr;
(function(e) {
  e[e.Null = 0] = "Null", e[e.PlainText = 1] = "PlainText";
})(Yr || (Yr = {}));
var Qr;
(function(e) {
  e[e.NotSet = -1] = "NotSet", e[e.None = 0] = "None", e[e.Italic = 1] = "Italic", e[e.Bold = 2] = "Bold", e[e.Underline = 4] = "Underline", e[e.Strikethrough = 8] = "Strikethrough";
})(Qr || (Qr = {}));
var Xn;
(function(e) {
  e[e.None = 0] = "None", e[e.DefaultForeground = 1] = "DefaultForeground", e[e.DefaultBackground = 2] = "DefaultBackground";
})(Xn || (Xn = {}));
var Zr;
(function(e) {
  e[e.Other = 0] = "Other", e[e.Comment = 1] = "Comment", e[e.String = 2] = "String", e[e.RegEx = 3] = "RegEx";
})(Zr || (Zr = {}));
var Kr;
(function(e) {
  e[e.LANGUAGEID_MASK = 255] = "LANGUAGEID_MASK", e[e.TOKEN_TYPE_MASK = 768] = "TOKEN_TYPE_MASK", e[e.BALANCED_BRACKETS_MASK = 1024] = "BALANCED_BRACKETS_MASK", e[e.FONT_STYLE_MASK = 30720] = "FONT_STYLE_MASK", e[e.FOREGROUND_MASK = 16744448] = "FOREGROUND_MASK", e[e.BACKGROUND_MASK = 4278190080] = "BACKGROUND_MASK", e[e.ITALIC_MASK = 2048] = "ITALIC_MASK", e[e.BOLD_MASK = 4096] = "BOLD_MASK", e[e.UNDERLINE_MASK = 8192] = "UNDERLINE_MASK", e[e.STRIKETHROUGH_MASK = 16384] = "STRIKETHROUGH_MASK", e[e.SEMANTIC_USE_ITALIC = 1] = "SEMANTIC_USE_ITALIC", e[e.SEMANTIC_USE_BOLD = 2] = "SEMANTIC_USE_BOLD", e[e.SEMANTIC_USE_UNDERLINE = 4] = "SEMANTIC_USE_UNDERLINE", e[e.SEMANTIC_USE_STRIKETHROUGH = 8] = "SEMANTIC_USE_STRIKETHROUGH", e[e.SEMANTIC_USE_FOREGROUND = 16] = "SEMANTIC_USE_FOREGROUND", e[e.SEMANTIC_USE_BACKGROUND = 32] = "SEMANTIC_USE_BACKGROUND", e[e.LANGUAGEID_OFFSET = 0] = "LANGUAGEID_OFFSET", e[e.TOKEN_TYPE_OFFSET = 8] = "TOKEN_TYPE_OFFSET", e[e.BALANCED_BRACKETS_OFFSET = 10] = "BALANCED_BRACKETS_OFFSET", e[e.FONT_STYLE_OFFSET = 11] = "FONT_STYLE_OFFSET", e[e.FOREGROUND_OFFSET = 15] = "FOREGROUND_OFFSET", e[e.BACKGROUND_OFFSET = 24] = "BACKGROUND_OFFSET";
})(Kr || (Kr = {}));
class Sc {
  constructor() {
    this._tokenizationSupports = /* @__PURE__ */ new Map(), this._factories = /* @__PURE__ */ new Map(), this._onDidChange = new He(), this.onDidChange = this._onDidChange.event, this._colorMap = null;
  }
  handleChange(t) {
    this._onDidChange.fire({
      changedLanguages: t,
      changedColorMap: !1
    });
  }
  register(t, n) {
    return this._tokenizationSupports.set(t, n), this.handleChange([t]), Hn(() => {
      this._tokenizationSupports.get(t) === n && (this._tokenizationSupports.delete(t), this.handleChange([t]));
    });
  }
  get(t) {
    return this._tokenizationSupports.get(t) || null;
  }
  registerFactory(t, n) {
    var r;
    (r = this._factories.get(t)) == null || r.dispose();
    const i = new Mc(this, t, n);
    return this._factories.set(t, i), Hn(() => {
      const s = this._factories.get(t);
      !s || s !== i || (this._factories.delete(t), s.dispose());
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
    return this._colorMap && this._colorMap.length > Xn.DefaultBackground ? this._colorMap[Xn.DefaultBackground] : null;
  }
}
class Mc extends Xt {
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
const Ic = globalThis._VSCODE_NLS_LANGUAGE === "pseudo" || typeof document < "u" && document.location && document.location.hash.indexOf("pseudo=true") >= 0;
function Cr(e, t) {
  let n;
  return t.length === 0 ? n = e : n = e.replace(/\{(\d+)\}/g, (i, r) => {
    const s = r[0], a = t[s];
    let l = i;
    return typeof a == "string" ? l = a : (typeof a == "number" || typeof a == "boolean" || a === void 0 || a === null) && (l = String(a)), l;
  }), Ic && (n = "［" + n.replace(/[aouei]/g, "$&$&") + "］"), n;
}
function ne(e, t, ...n) {
  return Cr(typeof e == "number" ? Dc(e, t) : t, n);
}
function Dc(e, t) {
  var i;
  const n = (i = globalThis._VSCODE_NLS_MESSAGES) == null ? void 0 : i[e];
  if (typeof n != "string") {
    if (typeof t == "string")
      return t;
    throw new Error(`!!! NLS MISSING: ${e} !!!`);
  }
  return n;
}
class Uc {
  constructor(t, n, i) {
    this.offset = t, this.type = n, this.language = i, this._tokenBrand = void 0;
  }
  toString() {
    return "(" + this.offset + ", " + this.type + ")";
  }
}
var es;
(function(e) {
  e[e.Increase = 0] = "Increase", e[e.Decrease = 1] = "Decrease";
})(es || (es = {}));
var V;
(function(e) {
  e[e.Method = 0] = "Method", e[e.Function = 1] = "Function", e[e.Constructor = 2] = "Constructor", e[e.Field = 3] = "Field", e[e.Variable = 4] = "Variable", e[e.Class = 5] = "Class", e[e.Struct = 6] = "Struct", e[e.Interface = 7] = "Interface", e[e.Module = 8] = "Module", e[e.Property = 9] = "Property", e[e.Event = 10] = "Event", e[e.Operator = 11] = "Operator", e[e.Unit = 12] = "Unit", e[e.Value = 13] = "Value", e[e.Constant = 14] = "Constant", e[e.Enum = 15] = "Enum", e[e.EnumMember = 16] = "EnumMember", e[e.Keyword = 17] = "Keyword", e[e.Text = 18] = "Text", e[e.Color = 19] = "Color", e[e.File = 20] = "File", e[e.Reference = 21] = "Reference", e[e.Customcolor = 22] = "Customcolor", e[e.Folder = 23] = "Folder", e[e.TypeParameter = 24] = "TypeParameter", e[e.User = 25] = "User", e[e.Issue = 26] = "Issue", e[e.Snippet = 27] = "Snippet";
})(V || (V = {}));
var ts;
(function(e) {
  const t = /* @__PURE__ */ new Map();
  t.set(V.Method, q.symbolMethod), t.set(V.Function, q.symbolFunction), t.set(V.Constructor, q.symbolConstructor), t.set(V.Field, q.symbolField), t.set(V.Variable, q.symbolVariable), t.set(V.Class, q.symbolClass), t.set(V.Struct, q.symbolStruct), t.set(V.Interface, q.symbolInterface), t.set(V.Module, q.symbolModule), t.set(V.Property, q.symbolProperty), t.set(V.Event, q.symbolEvent), t.set(V.Operator, q.symbolOperator), t.set(V.Unit, q.symbolUnit), t.set(V.Value, q.symbolValue), t.set(V.Enum, q.symbolEnum), t.set(V.Constant, q.symbolConstant), t.set(V.Enum, q.symbolEnum), t.set(V.EnumMember, q.symbolEnumMember), t.set(V.Keyword, q.symbolKeyword), t.set(V.Snippet, q.symbolSnippet), t.set(V.Text, q.symbolText), t.set(V.Color, q.symbolColor), t.set(V.File, q.symbolFile), t.set(V.Reference, q.symbolReference), t.set(V.Customcolor, q.symbolCustomColor), t.set(V.Folder, q.symbolFolder), t.set(V.TypeParameter, q.symbolTypeParameter), t.set(V.User, q.account), t.set(V.Issue, q.issues);
  function n(s) {
    let a = t.get(s);
    return a || (console.info("No codicon found for CompletionItemKind " + s), a = q.symbolProperty), a;
  }
  e.toIcon = n;
  const i = /* @__PURE__ */ new Map();
  i.set("method", V.Method), i.set("function", V.Function), i.set("constructor", V.Constructor), i.set("field", V.Field), i.set("variable", V.Variable), i.set("class", V.Class), i.set("struct", V.Struct), i.set("interface", V.Interface), i.set("module", V.Module), i.set("property", V.Property), i.set("event", V.Event), i.set("operator", V.Operator), i.set("unit", V.Unit), i.set("value", V.Value), i.set("constant", V.Constant), i.set("enum", V.Enum), i.set("enum-member", V.EnumMember), i.set("enumMember", V.EnumMember), i.set("keyword", V.Keyword), i.set("snippet", V.Snippet), i.set("text", V.Text), i.set("color", V.Color), i.set("file", V.File), i.set("reference", V.Reference), i.set("customcolor", V.Customcolor), i.set("folder", V.Folder), i.set("type-parameter", V.TypeParameter), i.set("typeParameter", V.TypeParameter), i.set("account", V.User), i.set("issue", V.Issue);
  function r(s, a) {
    let l = i.get(s);
    return typeof l > "u" && !a && (l = V.Property), l;
  }
  e.fromString = r;
})(ts || (ts = {}));
var ns;
(function(e) {
  e[e.Deprecated = 1] = "Deprecated";
})(ns || (ns = {}));
var is;
(function(e) {
  e[e.None = 0] = "None", e[e.KeepWhitespace = 1] = "KeepWhitespace", e[e.InsertAsSnippet = 4] = "InsertAsSnippet";
})(is || (is = {}));
var rs;
(function(e) {
  e[e.Word = 0] = "Word", e[e.Line = 1] = "Line", e[e.Suggest = 2] = "Suggest";
})(rs || (rs = {}));
var ss;
(function(e) {
  e[e.Invoke = 0] = "Invoke", e[e.TriggerCharacter = 1] = "TriggerCharacter", e[e.TriggerForIncompleteCompletions = 2] = "TriggerForIncompleteCompletions";
})(ss || (ss = {}));
var as;
(function(e) {
  e[e.Automatic = 0] = "Automatic", e[e.Explicit = 1] = "Explicit";
})(as || (as = {}));
var os;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.Auto = 2] = "Auto";
})(os || (os = {}));
var ls;
(function(e) {
  e[e.Automatic = 0] = "Automatic", e[e.PasteAs = 1] = "PasteAs";
})(ls || (ls = {}));
var us;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.TriggerCharacter = 2] = "TriggerCharacter", e[e.ContentChange = 3] = "ContentChange";
})(us || (us = {}));
var cs;
(function(e) {
  e[e.Text = 0] = "Text", e[e.Read = 1] = "Read", e[e.Write = 2] = "Write";
})(cs || (cs = {}));
var $;
(function(e) {
  e[e.File = 0] = "File", e[e.Module = 1] = "Module", e[e.Namespace = 2] = "Namespace", e[e.Package = 3] = "Package", e[e.Class = 4] = "Class", e[e.Method = 5] = "Method", e[e.Property = 6] = "Property", e[e.Field = 7] = "Field", e[e.Constructor = 8] = "Constructor", e[e.Enum = 9] = "Enum", e[e.Interface = 10] = "Interface", e[e.Function = 11] = "Function", e[e.Variable = 12] = "Variable", e[e.Constant = 13] = "Constant", e[e.String = 14] = "String", e[e.Number = 15] = "Number", e[e.Boolean = 16] = "Boolean", e[e.Array = 17] = "Array", e[e.Object = 18] = "Object", e[e.Key = 19] = "Key", e[e.Null = 20] = "Null", e[e.EnumMember = 21] = "EnumMember", e[e.Struct = 22] = "Struct", e[e.Event = 23] = "Event", e[e.Operator = 24] = "Operator", e[e.TypeParameter = 25] = "TypeParameter";
})($ || ($ = {}));
$.Array + "", ne(1658, "array"), $.Boolean + "", ne(1659, "boolean"), $.Class + "", ne(1660, "class"), $.Constant + "", ne(1661, "constant"), $.Constructor + "", ne(1662, "constructor"), $.Enum + "", ne(1663, "enumeration"), $.EnumMember + "", ne(1664, "enumeration member"), $.Event + "", ne(1665, "event"), $.Field + "", ne(1666, "field"), $.File + "", ne(1667, "file"), $.Function + "", ne(1668, "function"), $.Interface + "", ne(1669, "interface"), $.Key + "", ne(1670, "key"), $.Method + "", ne(1671, "method"), $.Module + "", ne(1672, "module"), $.Namespace + "", ne(1673, "namespace"), $.Null + "", ne(1674, "null"), $.Number + "", ne(1675, "number"), $.Object + "", ne(1676, "object"), $.Operator + "", ne(1677, "operator"), $.Package + "", ne(1678, "package"), $.Property + "", ne(1679, "property"), $.String + "", ne(1680, "string"), $.Struct + "", ne(1681, "struct"), $.TypeParameter + "", ne(1682, "type parameter"), $.Variable + "", ne(1683, "variable");
var fs;
(function(e) {
  e[e.Deprecated = 1] = "Deprecated";
})(fs || (fs = {}));
var hs;
(function(e) {
  const t = /* @__PURE__ */ new Map();
  t.set($.File, q.symbolFile), t.set($.Module, q.symbolModule), t.set($.Namespace, q.symbolNamespace), t.set($.Package, q.symbolPackage), t.set($.Class, q.symbolClass), t.set($.Method, q.symbolMethod), t.set($.Property, q.symbolProperty), t.set($.Field, q.symbolField), t.set($.Constructor, q.symbolConstructor), t.set($.Enum, q.symbolEnum), t.set($.Interface, q.symbolInterface), t.set($.Function, q.symbolFunction), t.set($.Variable, q.symbolVariable), t.set($.Constant, q.symbolConstant), t.set($.String, q.symbolString), t.set($.Number, q.symbolNumber), t.set($.Boolean, q.symbolBoolean), t.set($.Array, q.symbolArray), t.set($.Object, q.symbolObject), t.set($.Key, q.symbolKey), t.set($.Null, q.symbolNull), t.set($.EnumMember, q.symbolEnumMember), t.set($.Struct, q.symbolStruct), t.set($.Event, q.symbolEvent), t.set($.Operator, q.symbolOperator), t.set($.TypeParameter, q.symbolTypeParameter);
  function n(i) {
    let r = t.get(i);
    return r || (console.info("No codicon found for SymbolKind " + i), r = q.symbolProperty), r;
  }
  e.toIcon = n;
})(hs || (hs = {}));
var Ae;
let Uf = (Ae = class {
  static fromValue(t) {
    switch (t) {
      case "comment":
        return Ae.Comment;
      case "imports":
        return Ae.Imports;
      case "region":
        return Ae.Region;
    }
    return new Ae(t);
  }
  constructor(t) {
    this.value = t;
  }
}, Ae.Comment = new Ae("comment"), Ae.Imports = new Ae("imports"), Ae.Region = new Ae("region"), Ae);
var ms;
(function(e) {
  e[e.AIGenerated = 1] = "AIGenerated";
})(ms || (ms = {}));
var gs;
(function(e) {
  e[e.Invoke = 0] = "Invoke", e[e.Automatic = 1] = "Automatic";
})(gs || (gs = {}));
var ds;
(function(e) {
  function t(n) {
    return !n || typeof n != "object" ? !1 : typeof n.id == "string" && typeof n.title == "string";
  }
  e.is = t;
})(ds || (ds = {}));
var ps;
(function(e) {
  e[e.Collapsed = 0] = "Collapsed", e[e.Expanded = 1] = "Expanded";
})(ps || (ps = {}));
var bs;
(function(e) {
  e[e.Unresolved = 0] = "Unresolved", e[e.Resolved = 1] = "Resolved";
})(bs || (bs = {}));
var _s;
(function(e) {
  e[e.Current = 0] = "Current", e[e.Outdated = 1] = "Outdated";
})(_s || (_s = {}));
var vs;
(function(e) {
  e[e.Editing = 0] = "Editing", e[e.Preview = 1] = "Preview";
})(vs || (vs = {}));
var Ls;
(function(e) {
  e[e.Type = 1] = "Type", e[e.Parameter = 2] = "Parameter";
})(Ls || (Ls = {}));
new Sc();
var Ns;
(function(e) {
  e[e.None = 0] = "None", e[e.Option = 1] = "Option", e[e.Default = 2] = "Default", e[e.Preferred = 3] = "Preferred";
})(Ns || (Ns = {}));
var ws;
(function(e) {
  e[e.Invoke = 0] = "Invoke", e[e.Automatic = 1] = "Automatic";
})(ws || (ws = {}));
var As;
(function(e) {
  e[e.Unknown = 0] = "Unknown", e[e.Disabled = 1] = "Disabled", e[e.Enabled = 2] = "Enabled";
})(As || (As = {}));
var xs;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.Auto = 2] = "Auto";
})(xs || (xs = {}));
var Es;
(function(e) {
  e[e.None = 0] = "None", e[e.KeepWhitespace = 1] = "KeepWhitespace", e[e.InsertAsSnippet = 4] = "InsertAsSnippet";
})(Es || (Es = {}));
var ys;
(function(e) {
  e[e.Method = 0] = "Method", e[e.Function = 1] = "Function", e[e.Constructor = 2] = "Constructor", e[e.Field = 3] = "Field", e[e.Variable = 4] = "Variable", e[e.Class = 5] = "Class", e[e.Struct = 6] = "Struct", e[e.Interface = 7] = "Interface", e[e.Module = 8] = "Module", e[e.Property = 9] = "Property", e[e.Event = 10] = "Event", e[e.Operator = 11] = "Operator", e[e.Unit = 12] = "Unit", e[e.Value = 13] = "Value", e[e.Constant = 14] = "Constant", e[e.Enum = 15] = "Enum", e[e.EnumMember = 16] = "EnumMember", e[e.Keyword = 17] = "Keyword", e[e.Text = 18] = "Text", e[e.Color = 19] = "Color", e[e.File = 20] = "File", e[e.Reference = 21] = "Reference", e[e.Customcolor = 22] = "Customcolor", e[e.Folder = 23] = "Folder", e[e.TypeParameter = 24] = "TypeParameter", e[e.User = 25] = "User", e[e.Issue = 26] = "Issue", e[e.Snippet = 27] = "Snippet";
})(ys || (ys = {}));
var Rs;
(function(e) {
  e[e.Deprecated = 1] = "Deprecated";
})(Rs || (Rs = {}));
var ks;
(function(e) {
  e[e.Invoke = 0] = "Invoke", e[e.TriggerCharacter = 1] = "TriggerCharacter", e[e.TriggerForIncompleteCompletions = 2] = "TriggerForIncompleteCompletions";
})(ks || (ks = {}));
var Ts;
(function(e) {
  e[e.EXACT = 0] = "EXACT", e[e.ABOVE = 1] = "ABOVE", e[e.BELOW = 2] = "BELOW";
})(Ts || (Ts = {}));
var Ss;
(function(e) {
  e[e.NotSet = 0] = "NotSet", e[e.ContentFlush = 1] = "ContentFlush", e[e.RecoverFromMarkers = 2] = "RecoverFromMarkers", e[e.Explicit = 3] = "Explicit", e[e.Paste = 4] = "Paste", e[e.Undo = 5] = "Undo", e[e.Redo = 6] = "Redo";
})(Ss || (Ss = {}));
var Ms;
(function(e) {
  e[e.LF = 1] = "LF", e[e.CRLF = 2] = "CRLF";
})(Ms || (Ms = {}));
var Is;
(function(e) {
  e[e.Text = 0] = "Text", e[e.Read = 1] = "Read", e[e.Write = 2] = "Write";
})(Is || (Is = {}));
var Ds;
(function(e) {
  e[e.None = 0] = "None", e[e.Keep = 1] = "Keep", e[e.Brackets = 2] = "Brackets", e[e.Advanced = 3] = "Advanced", e[e.Full = 4] = "Full";
})(Ds || (Ds = {}));
var Us;
(function(e) {
  e[e.acceptSuggestionOnCommitCharacter = 0] = "acceptSuggestionOnCommitCharacter", e[e.acceptSuggestionOnEnter = 1] = "acceptSuggestionOnEnter", e[e.accessibilitySupport = 2] = "accessibilitySupport", e[e.accessibilityPageSize = 3] = "accessibilityPageSize", e[e.ariaLabel = 4] = "ariaLabel", e[e.ariaRequired = 5] = "ariaRequired", e[e.autoClosingBrackets = 6] = "autoClosingBrackets", e[e.autoClosingComments = 7] = "autoClosingComments", e[e.screenReaderAnnounceInlineSuggestion = 8] = "screenReaderAnnounceInlineSuggestion", e[e.autoClosingDelete = 9] = "autoClosingDelete", e[e.autoClosingOvertype = 10] = "autoClosingOvertype", e[e.autoClosingQuotes = 11] = "autoClosingQuotes", e[e.autoIndent = 12] = "autoIndent", e[e.automaticLayout = 13] = "automaticLayout", e[e.autoSurround = 14] = "autoSurround", e[e.bracketPairColorization = 15] = "bracketPairColorization", e[e.guides = 16] = "guides", e[e.codeLens = 17] = "codeLens", e[e.codeLensFontFamily = 18] = "codeLensFontFamily", e[e.codeLensFontSize = 19] = "codeLensFontSize", e[e.colorDecorators = 20] = "colorDecorators", e[e.colorDecoratorsLimit = 21] = "colorDecoratorsLimit", e[e.columnSelection = 22] = "columnSelection", e[e.comments = 23] = "comments", e[e.contextmenu = 24] = "contextmenu", e[e.copyWithSyntaxHighlighting = 25] = "copyWithSyntaxHighlighting", e[e.cursorBlinking = 26] = "cursorBlinking", e[e.cursorSmoothCaretAnimation = 27] = "cursorSmoothCaretAnimation", e[e.cursorStyle = 28] = "cursorStyle", e[e.cursorSurroundingLines = 29] = "cursorSurroundingLines", e[e.cursorSurroundingLinesStyle = 30] = "cursorSurroundingLinesStyle", e[e.cursorWidth = 31] = "cursorWidth", e[e.disableLayerHinting = 32] = "disableLayerHinting", e[e.disableMonospaceOptimizations = 33] = "disableMonospaceOptimizations", e[e.domReadOnly = 34] = "domReadOnly", e[e.dragAndDrop = 35] = "dragAndDrop", e[e.dropIntoEditor = 36] = "dropIntoEditor", e[e.emptySelectionClipboard = 37] = "emptySelectionClipboard", e[e.experimentalWhitespaceRendering = 38] = "experimentalWhitespaceRendering", e[e.extraEditorClassName = 39] = "extraEditorClassName", e[e.fastScrollSensitivity = 40] = "fastScrollSensitivity", e[e.find = 41] = "find", e[e.fixedOverflowWidgets = 42] = "fixedOverflowWidgets", e[e.folding = 43] = "folding", e[e.foldingStrategy = 44] = "foldingStrategy", e[e.foldingHighlight = 45] = "foldingHighlight", e[e.foldingImportsByDefault = 46] = "foldingImportsByDefault", e[e.foldingMaximumRegions = 47] = "foldingMaximumRegions", e[e.unfoldOnClickAfterEndOfLine = 48] = "unfoldOnClickAfterEndOfLine", e[e.fontFamily = 49] = "fontFamily", e[e.fontInfo = 50] = "fontInfo", e[e.fontLigatures = 51] = "fontLigatures", e[e.fontSize = 52] = "fontSize", e[e.fontWeight = 53] = "fontWeight", e[e.fontVariations = 54] = "fontVariations", e[e.formatOnPaste = 55] = "formatOnPaste", e[e.formatOnType = 56] = "formatOnType", e[e.glyphMargin = 57] = "glyphMargin", e[e.gotoLocation = 58] = "gotoLocation", e[e.hideCursorInOverviewRuler = 59] = "hideCursorInOverviewRuler", e[e.hover = 60] = "hover", e[e.inDiffEditor = 61] = "inDiffEditor", e[e.inlineSuggest = 62] = "inlineSuggest", e[e.inlineEdit = 63] = "inlineEdit", e[e.letterSpacing = 64] = "letterSpacing", e[e.lightbulb = 65] = "lightbulb", e[e.lineDecorationsWidth = 66] = "lineDecorationsWidth", e[e.lineHeight = 67] = "lineHeight", e[e.lineNumbers = 68] = "lineNumbers", e[e.lineNumbersMinChars = 69] = "lineNumbersMinChars", e[e.linkedEditing = 70] = "linkedEditing", e[e.links = 71] = "links", e[e.matchBrackets = 72] = "matchBrackets", e[e.minimap = 73] = "minimap", e[e.mouseStyle = 74] = "mouseStyle", e[e.mouseWheelScrollSensitivity = 75] = "mouseWheelScrollSensitivity", e[e.mouseWheelZoom = 76] = "mouseWheelZoom", e[e.multiCursorMergeOverlapping = 77] = "multiCursorMergeOverlapping", e[e.multiCursorModifier = 78] = "multiCursorModifier", e[e.multiCursorPaste = 79] = "multiCursorPaste", e[e.multiCursorLimit = 80] = "multiCursorLimit", e[e.occurrencesHighlight = 81] = "occurrencesHighlight", e[e.overviewRulerBorder = 82] = "overviewRulerBorder", e[e.overviewRulerLanes = 83] = "overviewRulerLanes", e[e.padding = 84] = "padding", e[e.pasteAs = 85] = "pasteAs", e[e.parameterHints = 86] = "parameterHints", e[e.peekWidgetDefaultFocus = 87] = "peekWidgetDefaultFocus", e[e.placeholder = 88] = "placeholder", e[e.definitionLinkOpensInPeek = 89] = "definitionLinkOpensInPeek", e[e.quickSuggestions = 90] = "quickSuggestions", e[e.quickSuggestionsDelay = 91] = "quickSuggestionsDelay", e[e.readOnly = 92] = "readOnly", e[e.readOnlyMessage = 93] = "readOnlyMessage", e[e.renameOnType = 94] = "renameOnType", e[e.renderControlCharacters = 95] = "renderControlCharacters", e[e.renderFinalNewline = 96] = "renderFinalNewline", e[e.renderLineHighlight = 97] = "renderLineHighlight", e[e.renderLineHighlightOnlyWhenFocus = 98] = "renderLineHighlightOnlyWhenFocus", e[e.renderValidationDecorations = 99] = "renderValidationDecorations", e[e.renderWhitespace = 100] = "renderWhitespace", e[e.revealHorizontalRightPadding = 101] = "revealHorizontalRightPadding", e[e.roundedSelection = 102] = "roundedSelection", e[e.rulers = 103] = "rulers", e[e.scrollbar = 104] = "scrollbar", e[e.scrollBeyondLastColumn = 105] = "scrollBeyondLastColumn", e[e.scrollBeyondLastLine = 106] = "scrollBeyondLastLine", e[e.scrollPredominantAxis = 107] = "scrollPredominantAxis", e[e.selectionClipboard = 108] = "selectionClipboard", e[e.selectionHighlight = 109] = "selectionHighlight", e[e.selectOnLineNumbers = 110] = "selectOnLineNumbers", e[e.showFoldingControls = 111] = "showFoldingControls", e[e.showUnused = 112] = "showUnused", e[e.snippetSuggestions = 113] = "snippetSuggestions", e[e.smartSelect = 114] = "smartSelect", e[e.smoothScrolling = 115] = "smoothScrolling", e[e.stickyScroll = 116] = "stickyScroll", e[e.stickyTabStops = 117] = "stickyTabStops", e[e.stopRenderingLineAfter = 118] = "stopRenderingLineAfter", e[e.suggest = 119] = "suggest", e[e.suggestFontSize = 120] = "suggestFontSize", e[e.suggestLineHeight = 121] = "suggestLineHeight", e[e.suggestOnTriggerCharacters = 122] = "suggestOnTriggerCharacters", e[e.suggestSelection = 123] = "suggestSelection", e[e.tabCompletion = 124] = "tabCompletion", e[e.tabIndex = 125] = "tabIndex", e[e.unicodeHighlighting = 126] = "unicodeHighlighting", e[e.unusualLineTerminators = 127] = "unusualLineTerminators", e[e.useShadowDOM = 128] = "useShadowDOM", e[e.useTabStops = 129] = "useTabStops", e[e.wordBreak = 130] = "wordBreak", e[e.wordSegmenterLocales = 131] = "wordSegmenterLocales", e[e.wordSeparators = 132] = "wordSeparators", e[e.wordWrap = 133] = "wordWrap", e[e.wordWrapBreakAfterCharacters = 134] = "wordWrapBreakAfterCharacters", e[e.wordWrapBreakBeforeCharacters = 135] = "wordWrapBreakBeforeCharacters", e[e.wordWrapColumn = 136] = "wordWrapColumn", e[e.wordWrapOverride1 = 137] = "wordWrapOverride1", e[e.wordWrapOverride2 = 138] = "wordWrapOverride2", e[e.wrappingIndent = 139] = "wrappingIndent", e[e.wrappingStrategy = 140] = "wrappingStrategy", e[e.showDeprecated = 141] = "showDeprecated", e[e.inlayHints = 142] = "inlayHints", e[e.editorClassName = 143] = "editorClassName", e[e.pixelRatio = 144] = "pixelRatio", e[e.tabFocusMode = 145] = "tabFocusMode", e[e.layoutInfo = 146] = "layoutInfo", e[e.wrappingInfo = 147] = "wrappingInfo", e[e.defaultColorDecorators = 148] = "defaultColorDecorators", e[e.colorDecoratorsActivatedOn = 149] = "colorDecoratorsActivatedOn", e[e.inlineCompletionsAccessibilityVerbose = 150] = "inlineCompletionsAccessibilityVerbose";
})(Us || (Us = {}));
var Fs;
(function(e) {
  e[e.TextDefined = 0] = "TextDefined", e[e.LF = 1] = "LF", e[e.CRLF = 2] = "CRLF";
})(Fs || (Fs = {}));
var Ps;
(function(e) {
  e[e.LF = 0] = "LF", e[e.CRLF = 1] = "CRLF";
})(Ps || (Ps = {}));
var Os;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Center = 2] = "Center", e[e.Right = 3] = "Right";
})(Os || (Os = {}));
var Bs;
(function(e) {
  e[e.Increase = 0] = "Increase", e[e.Decrease = 1] = "Decrease";
})(Bs || (Bs = {}));
var Vs;
(function(e) {
  e[e.None = 0] = "None", e[e.Indent = 1] = "Indent", e[e.IndentOutdent = 2] = "IndentOutdent", e[e.Outdent = 3] = "Outdent";
})(Vs || (Vs = {}));
var qs;
(function(e) {
  e[e.Both = 0] = "Both", e[e.Right = 1] = "Right", e[e.Left = 2] = "Left", e[e.None = 3] = "None";
})(qs || (qs = {}));
var Hs;
(function(e) {
  e[e.Type = 1] = "Type", e[e.Parameter = 2] = "Parameter";
})(Hs || (Hs = {}));
var $s;
(function(e) {
  e[e.Automatic = 0] = "Automatic", e[e.Explicit = 1] = "Explicit";
})($s || ($s = {}));
var Ws;
(function(e) {
  e[e.Invoke = 0] = "Invoke", e[e.Automatic = 1] = "Automatic";
})(Ws || (Ws = {}));
var Vi;
(function(e) {
  e[e.DependsOnKbLayout = -1] = "DependsOnKbLayout", e[e.Unknown = 0] = "Unknown", e[e.Backspace = 1] = "Backspace", e[e.Tab = 2] = "Tab", e[e.Enter = 3] = "Enter", e[e.Shift = 4] = "Shift", e[e.Ctrl = 5] = "Ctrl", e[e.Alt = 6] = "Alt", e[e.PauseBreak = 7] = "PauseBreak", e[e.CapsLock = 8] = "CapsLock", e[e.Escape = 9] = "Escape", e[e.Space = 10] = "Space", e[e.PageUp = 11] = "PageUp", e[e.PageDown = 12] = "PageDown", e[e.End = 13] = "End", e[e.Home = 14] = "Home", e[e.LeftArrow = 15] = "LeftArrow", e[e.UpArrow = 16] = "UpArrow", e[e.RightArrow = 17] = "RightArrow", e[e.DownArrow = 18] = "DownArrow", e[e.Insert = 19] = "Insert", e[e.Delete = 20] = "Delete", e[e.Digit0 = 21] = "Digit0", e[e.Digit1 = 22] = "Digit1", e[e.Digit2 = 23] = "Digit2", e[e.Digit3 = 24] = "Digit3", e[e.Digit4 = 25] = "Digit4", e[e.Digit5 = 26] = "Digit5", e[e.Digit6 = 27] = "Digit6", e[e.Digit7 = 28] = "Digit7", e[e.Digit8 = 29] = "Digit8", e[e.Digit9 = 30] = "Digit9", e[e.KeyA = 31] = "KeyA", e[e.KeyB = 32] = "KeyB", e[e.KeyC = 33] = "KeyC", e[e.KeyD = 34] = "KeyD", e[e.KeyE = 35] = "KeyE", e[e.KeyF = 36] = "KeyF", e[e.KeyG = 37] = "KeyG", e[e.KeyH = 38] = "KeyH", e[e.KeyI = 39] = "KeyI", e[e.KeyJ = 40] = "KeyJ", e[e.KeyK = 41] = "KeyK", e[e.KeyL = 42] = "KeyL", e[e.KeyM = 43] = "KeyM", e[e.KeyN = 44] = "KeyN", e[e.KeyO = 45] = "KeyO", e[e.KeyP = 46] = "KeyP", e[e.KeyQ = 47] = "KeyQ", e[e.KeyR = 48] = "KeyR", e[e.KeyS = 49] = "KeyS", e[e.KeyT = 50] = "KeyT", e[e.KeyU = 51] = "KeyU", e[e.KeyV = 52] = "KeyV", e[e.KeyW = 53] = "KeyW", e[e.KeyX = 54] = "KeyX", e[e.KeyY = 55] = "KeyY", e[e.KeyZ = 56] = "KeyZ", e[e.Meta = 57] = "Meta", e[e.ContextMenu = 58] = "ContextMenu", e[e.F1 = 59] = "F1", e[e.F2 = 60] = "F2", e[e.F3 = 61] = "F3", e[e.F4 = 62] = "F4", e[e.F5 = 63] = "F5", e[e.F6 = 64] = "F6", e[e.F7 = 65] = "F7", e[e.F8 = 66] = "F8", e[e.F9 = 67] = "F9", e[e.F10 = 68] = "F10", e[e.F11 = 69] = "F11", e[e.F12 = 70] = "F12", e[e.F13 = 71] = "F13", e[e.F14 = 72] = "F14", e[e.F15 = 73] = "F15", e[e.F16 = 74] = "F16", e[e.F17 = 75] = "F17", e[e.F18 = 76] = "F18", e[e.F19 = 77] = "F19", e[e.F20 = 78] = "F20", e[e.F21 = 79] = "F21", e[e.F22 = 80] = "F22", e[e.F23 = 81] = "F23", e[e.F24 = 82] = "F24", e[e.NumLock = 83] = "NumLock", e[e.ScrollLock = 84] = "ScrollLock", e[e.Semicolon = 85] = "Semicolon", e[e.Equal = 86] = "Equal", e[e.Comma = 87] = "Comma", e[e.Minus = 88] = "Minus", e[e.Period = 89] = "Period", e[e.Slash = 90] = "Slash", e[e.Backquote = 91] = "Backquote", e[e.BracketLeft = 92] = "BracketLeft", e[e.Backslash = 93] = "Backslash", e[e.BracketRight = 94] = "BracketRight", e[e.Quote = 95] = "Quote", e[e.OEM_8 = 96] = "OEM_8", e[e.IntlBackslash = 97] = "IntlBackslash", e[e.Numpad0 = 98] = "Numpad0", e[e.Numpad1 = 99] = "Numpad1", e[e.Numpad2 = 100] = "Numpad2", e[e.Numpad3 = 101] = "Numpad3", e[e.Numpad4 = 102] = "Numpad4", e[e.Numpad5 = 103] = "Numpad5", e[e.Numpad6 = 104] = "Numpad6", e[e.Numpad7 = 105] = "Numpad7", e[e.Numpad8 = 106] = "Numpad8", e[e.Numpad9 = 107] = "Numpad9", e[e.NumpadMultiply = 108] = "NumpadMultiply", e[e.NumpadAdd = 109] = "NumpadAdd", e[e.NUMPAD_SEPARATOR = 110] = "NUMPAD_SEPARATOR", e[e.NumpadSubtract = 111] = "NumpadSubtract", e[e.NumpadDecimal = 112] = "NumpadDecimal", e[e.NumpadDivide = 113] = "NumpadDivide", e[e.KEY_IN_COMPOSITION = 114] = "KEY_IN_COMPOSITION", e[e.ABNT_C1 = 115] = "ABNT_C1", e[e.ABNT_C2 = 116] = "ABNT_C2", e[e.AudioVolumeMute = 117] = "AudioVolumeMute", e[e.AudioVolumeUp = 118] = "AudioVolumeUp", e[e.AudioVolumeDown = 119] = "AudioVolumeDown", e[e.BrowserSearch = 120] = "BrowserSearch", e[e.BrowserHome = 121] = "BrowserHome", e[e.BrowserBack = 122] = "BrowserBack", e[e.BrowserForward = 123] = "BrowserForward", e[e.MediaTrackNext = 124] = "MediaTrackNext", e[e.MediaTrackPrevious = 125] = "MediaTrackPrevious", e[e.MediaStop = 126] = "MediaStop", e[e.MediaPlayPause = 127] = "MediaPlayPause", e[e.LaunchMediaPlayer = 128] = "LaunchMediaPlayer", e[e.LaunchMail = 129] = "LaunchMail", e[e.LaunchApp2 = 130] = "LaunchApp2", e[e.Clear = 131] = "Clear", e[e.MAX_VALUE = 132] = "MAX_VALUE";
})(Vi || (Vi = {}));
var qi;
(function(e) {
  e[e.Hint = 1] = "Hint", e[e.Info = 2] = "Info", e[e.Warning = 4] = "Warning", e[e.Error = 8] = "Error";
})(qi || (qi = {}));
var Hi;
(function(e) {
  e[e.Unnecessary = 1] = "Unnecessary", e[e.Deprecated = 2] = "Deprecated";
})(Hi || (Hi = {}));
var js;
(function(e) {
  e[e.Inline = 1] = "Inline", e[e.Gutter = 2] = "Gutter";
})(js || (js = {}));
var Gs;
(function(e) {
  e[e.Normal = 1] = "Normal", e[e.Underlined = 2] = "Underlined";
})(Gs || (Gs = {}));
var zs;
(function(e) {
  e[e.UNKNOWN = 0] = "UNKNOWN", e[e.TEXTAREA = 1] = "TEXTAREA", e[e.GUTTER_GLYPH_MARGIN = 2] = "GUTTER_GLYPH_MARGIN", e[e.GUTTER_LINE_NUMBERS = 3] = "GUTTER_LINE_NUMBERS", e[e.GUTTER_LINE_DECORATIONS = 4] = "GUTTER_LINE_DECORATIONS", e[e.GUTTER_VIEW_ZONE = 5] = "GUTTER_VIEW_ZONE", e[e.CONTENT_TEXT = 6] = "CONTENT_TEXT", e[e.CONTENT_EMPTY = 7] = "CONTENT_EMPTY", e[e.CONTENT_VIEW_ZONE = 8] = "CONTENT_VIEW_ZONE", e[e.CONTENT_WIDGET = 9] = "CONTENT_WIDGET", e[e.OVERVIEW_RULER = 10] = "OVERVIEW_RULER", e[e.SCROLLBAR = 11] = "SCROLLBAR", e[e.OVERLAY_WIDGET = 12] = "OVERLAY_WIDGET", e[e.OUTSIDE_EDITOR = 13] = "OUTSIDE_EDITOR";
})(zs || (zs = {}));
var Xs;
(function(e) {
  e[e.AIGenerated = 1] = "AIGenerated";
})(Xs || (Xs = {}));
var Js;
(function(e) {
  e[e.Invoke = 0] = "Invoke", e[e.Automatic = 1] = "Automatic";
})(Js || (Js = {}));
var Ys;
(function(e) {
  e[e.TOP_RIGHT_CORNER = 0] = "TOP_RIGHT_CORNER", e[e.BOTTOM_RIGHT_CORNER = 1] = "BOTTOM_RIGHT_CORNER", e[e.TOP_CENTER = 2] = "TOP_CENTER";
})(Ys || (Ys = {}));
var Qs;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Center = 2] = "Center", e[e.Right = 4] = "Right", e[e.Full = 7] = "Full";
})(Qs || (Qs = {}));
var Zs;
(function(e) {
  e[e.Word = 0] = "Word", e[e.Line = 1] = "Line", e[e.Suggest = 2] = "Suggest";
})(Zs || (Zs = {}));
var Ks;
(function(e) {
  e[e.Left = 0] = "Left", e[e.Right = 1] = "Right", e[e.None = 2] = "None", e[e.LeftOfInjectedText = 3] = "LeftOfInjectedText", e[e.RightOfInjectedText = 4] = "RightOfInjectedText";
})(Ks || (Ks = {}));
var Cs;
(function(e) {
  e[e.Off = 0] = "Off", e[e.On = 1] = "On", e[e.Relative = 2] = "Relative", e[e.Interval = 3] = "Interval", e[e.Custom = 4] = "Custom";
})(Cs || (Cs = {}));
var ea;
(function(e) {
  e[e.None = 0] = "None", e[e.Text = 1] = "Text", e[e.Blocks = 2] = "Blocks";
})(ea || (ea = {}));
var ta;
(function(e) {
  e[e.Smooth = 0] = "Smooth", e[e.Immediate = 1] = "Immediate";
})(ta || (ta = {}));
var na;
(function(e) {
  e[e.Auto = 1] = "Auto", e[e.Hidden = 2] = "Hidden", e[e.Visible = 3] = "Visible";
})(na || (na = {}));
var $i;
(function(e) {
  e[e.LTR = 0] = "LTR", e[e.RTL = 1] = "RTL";
})($i || ($i = {}));
var ia;
(function(e) {
  e.Off = "off", e.OnCode = "onCode", e.On = "on";
})(ia || (ia = {}));
var ra;
(function(e) {
  e[e.Invoke = 1] = "Invoke", e[e.TriggerCharacter = 2] = "TriggerCharacter", e[e.ContentChange = 3] = "ContentChange";
})(ra || (ra = {}));
var sa;
(function(e) {
  e[e.File = 0] = "File", e[e.Module = 1] = "Module", e[e.Namespace = 2] = "Namespace", e[e.Package = 3] = "Package", e[e.Class = 4] = "Class", e[e.Method = 5] = "Method", e[e.Property = 6] = "Property", e[e.Field = 7] = "Field", e[e.Constructor = 8] = "Constructor", e[e.Enum = 9] = "Enum", e[e.Interface = 10] = "Interface", e[e.Function = 11] = "Function", e[e.Variable = 12] = "Variable", e[e.Constant = 13] = "Constant", e[e.String = 14] = "String", e[e.Number = 15] = "Number", e[e.Boolean = 16] = "Boolean", e[e.Array = 17] = "Array", e[e.Object = 18] = "Object", e[e.Key = 19] = "Key", e[e.Null = 20] = "Null", e[e.EnumMember = 21] = "EnumMember", e[e.Struct = 22] = "Struct", e[e.Event = 23] = "Event", e[e.Operator = 24] = "Operator", e[e.TypeParameter = 25] = "TypeParameter";
})(sa || (sa = {}));
var aa;
(function(e) {
  e[e.Deprecated = 1] = "Deprecated";
})(aa || (aa = {}));
var oa;
(function(e) {
  e[e.Hidden = 0] = "Hidden", e[e.Blink = 1] = "Blink", e[e.Smooth = 2] = "Smooth", e[e.Phase = 3] = "Phase", e[e.Expand = 4] = "Expand", e[e.Solid = 5] = "Solid";
})(oa || (oa = {}));
var la;
(function(e) {
  e[e.Line = 1] = "Line", e[e.Block = 2] = "Block", e[e.Underline = 3] = "Underline", e[e.LineThin = 4] = "LineThin", e[e.BlockOutline = 5] = "BlockOutline", e[e.UnderlineThin = 6] = "UnderlineThin";
})(la || (la = {}));
var ua;
(function(e) {
  e[e.AlwaysGrowsWhenTypingAtEdges = 0] = "AlwaysGrowsWhenTypingAtEdges", e[e.NeverGrowsWhenTypingAtEdges = 1] = "NeverGrowsWhenTypingAtEdges", e[e.GrowsOnlyWhenTypingBefore = 2] = "GrowsOnlyWhenTypingBefore", e[e.GrowsOnlyWhenTypingAfter = 3] = "GrowsOnlyWhenTypingAfter";
})(ua || (ua = {}));
var ca;
(function(e) {
  e[e.None = 0] = "None", e[e.Same = 1] = "Same", e[e.Indent = 2] = "Indent", e[e.DeepIndent = 3] = "DeepIndent";
})(ca || (ca = {}));
const Ht = class Ht {
  static chord(t, n) {
    return Rc(t, n);
  }
};
Ht.CtrlCmd = Pt.CtrlCmd, Ht.Shift = Pt.Shift, Ht.Alt = Pt.Alt, Ht.WinCtrl = Pt.WinCtrl;
let Wi = Ht;
function Fc() {
  return {
    editor: void 0,
    languages: void 0,
    CancellationTokenSource: ku,
    Emitter: He,
    KeyCode: Vi,
    KeyMod: Wi,
    Position: se,
    Range: J,
    Selection: ke,
    SelectionDirection: $i,
    MarkerSeverity: qi,
    MarkerTag: Hi,
    Uri: _r,
    Token: Uc
  };
}
var Jt;
(function(e) {
  e[e.Regular = 0] = "Regular", e[e.Whitespace = 1] = "Whitespace", e[e.WordSeparator = 2] = "WordSeparator";
})(Jt || (Jt = {}));
new uu(10);
var fa;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Center = 2] = "Center", e[e.Right = 4] = "Right", e[e.Full = 7] = "Full";
})(fa || (fa = {}));
var ha;
(function(e) {
  e[e.Left = 1] = "Left", e[e.Center = 2] = "Center", e[e.Right = 3] = "Right";
})(ha || (ha = {}));
var ma;
(function(e) {
  e[e.Inline = 1] = "Inline", e[e.Gutter = 2] = "Gutter";
})(ma || (ma = {}));
var ga;
(function(e) {
  e[e.Normal = 1] = "Normal", e[e.Underlined = 2] = "Underlined";
})(ga || (ga = {}));
var da;
(function(e) {
  e[e.Both = 0] = "Both", e[e.Right = 1] = "Right", e[e.Left = 2] = "Left", e[e.None = 3] = "None";
})(da || (da = {}));
var pa;
(function(e) {
  e[e.TextDefined = 0] = "TextDefined", e[e.LF = 1] = "LF", e[e.CRLF = 2] = "CRLF";
})(pa || (pa = {}));
var ba;
(function(e) {
  e[e.LF = 1] = "LF", e[e.CRLF = 2] = "CRLF";
})(ba || (ba = {}));
var _a;
(function(e) {
  e[e.LF = 0] = "LF", e[e.CRLF = 1] = "CRLF";
})(_a || (_a = {}));
var va;
(function(e) {
  e[e.AlwaysGrowsWhenTypingAtEdges = 0] = "AlwaysGrowsWhenTypingAtEdges", e[e.NeverGrowsWhenTypingAtEdges = 1] = "NeverGrowsWhenTypingAtEdges", e[e.GrowsOnlyWhenTypingBefore = 2] = "GrowsOnlyWhenTypingBefore", e[e.GrowsOnlyWhenTypingAfter = 3] = "GrowsOnlyWhenTypingAfter";
})(va || (va = {}));
var La;
(function(e) {
  e[e.Left = 0] = "Left", e[e.Right = 1] = "Right", e[e.None = 2] = "None", e[e.LeftOfInjectedText = 3] = "LeftOfInjectedText", e[e.RightOfInjectedText = 4] = "RightOfInjectedText";
})(La || (La = {}));
var Na;
(function(e) {
  e[e.FIRST_LINE_DETECTION_LENGTH_LIMIT = 1e3] = "FIRST_LINE_DETECTION_LENGTH_LIMIT";
})(Na || (Na = {}));
function Pc(e, t, n, i, r) {
  if (i === 0)
    return !0;
  const s = t.charCodeAt(i - 1);
  if (e.get(s) !== Jt.Regular || s === S.CarriageReturn || s === S.LineFeed)
    return !0;
  if (r > 0) {
    const a = t.charCodeAt(i);
    if (e.get(a) !== Jt.Regular)
      return !0;
  }
  return !1;
}
function Oc(e, t, n, i, r) {
  if (i + r === n)
    return !0;
  const s = t.charCodeAt(i + r);
  if (e.get(s) !== Jt.Regular || s === S.CarriageReturn || s === S.LineFeed)
    return !0;
  if (r > 0) {
    const a = t.charCodeAt(i + r - 1);
    if (e.get(a) !== Jt.Regular)
      return !0;
  }
  return !1;
}
function Bc(e, t, n, i, r) {
  return Pc(e, t, n, i, r) && Oc(e, t, n, i, r);
}
class Vc {
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
      const r = i.index, s = i[0].length;
      if (r === this._prevMatchStartIndex && s === this._prevMatchLength) {
        if (s === 0) {
          Fu(t, n, this._searchRegex.lastIndex) > 65535 ? this._searchRegex.lastIndex += 2 : this._searchRegex.lastIndex += 1;
          continue;
        }
        return null;
      }
      if (this._prevMatchStartIndex = r, this._prevMatchLength = s, !this._wordSeparators || Bc(this._wordSeparators, t, n, r, s))
        return i;
    } while (i);
    return null;
  }
}
function qc(e, t = "Unreachable") {
  throw new Error(t);
}
function Jn(e) {
  if (!e()) {
    debugger;
    e(), In(new ye("Assertion Failed"));
  }
}
function vl(e, t) {
  let n = 0;
  for (; n < e.length - 1; ) {
    const i = e[n], r = e[n + 1];
    if (!t(i, r))
      return !1;
    n++;
  }
  return !0;
}
class Hc {
  static computeUnicodeHighlights(t, n, i) {
    const r = i ? i.startLineNumber : 1, s = i ? i.endLineNumber : t.getLineCount(), a = new wa(n), l = a.getCandidateCodePoints();
    let o;
    l === "allNonBasicAscii" ? o = new RegExp("[^\\t\\n\\r\\x20-\\x7E]", "g") : o = new RegExp(`${$c(Array.from(l))}`, "g");
    const u = new Vc(null, o), h = [];
    let c = !1, m, g = 0, d = 0, p = 0;
    e: for (let _ = r, x = s; _ <= x; _++) {
      const R = t.getLineContent(_), v = R.length;
      u.reset(0);
      do
        if (m = u.next(R), m) {
          let L = m.index, b = m.index + m[0].length;
          if (L > 0) {
            const O = R.charCodeAt(L - 1);
            Wn(O) && L--;
          }
          if (b + 1 < v) {
            const O = R.charCodeAt(b - 1);
            Wn(O) && b++;
          }
          const E = R.substring(L, b);
          let k = vr(L + 1, pl, R, 0);
          k && k.endColumn <= L + 1 && (k = null);
          const D = a.shouldHighlightNonBasicASCII(E, k ? k.word : null);
          if (D !== we.None) {
            if (D === we.Ambiguous ? g++ : D === we.Invisible ? d++ : D === we.NonBasicASCII ? p++ : qc(), h.length >= 1e3) {
              c = !0;
              break e;
            }
            h.push(new J(_, L + 1, _, b + 1));
          }
        }
      while (m);
    }
    return {
      ranges: h,
      hasMore: c,
      ambiguousCharacterCount: g,
      invisibleCharacterCount: d,
      nonBasicAsciiCharacterCount: p
    };
  }
  static computeUnicodeHighlightReason(t, n) {
    const i = new wa(n);
    switch (i.shouldHighlightNonBasicASCII(t, null)) {
      case we.None:
        return null;
      case we.Invisible:
        return { kind: fn.Invisible };
      case we.Ambiguous: {
        const s = t.codePointAt(0), a = i.ambiguousCharacters.getPrimaryConfusable(s), l = Ln.getLocales().filter((o) => !Ln.getInstance(/* @__PURE__ */ new Set([...n.allowedLocales, o])).isAmbiguous(s));
        return { kind: fn.Ambiguous, confusableWith: String.fromCodePoint(a), notAmbiguousInLocales: l };
      }
      case we.NonBasicASCII:
        return { kind: fn.NonBasicAscii };
    }
  }
}
function $c(e, t) {
  return `[${Mu(e.map((i) => String.fromCodePoint(i)).join(""))}]`;
}
var fn;
(function(e) {
  e[e.Ambiguous = 0] = "Ambiguous", e[e.Invisible = 1] = "Invisible", e[e.NonBasicAscii = 2] = "NonBasicAscii";
})(fn || (fn = {}));
class wa {
  constructor(t) {
    this.options = t, this.allowedCodePoints = new Set(t.allowedCodePoints), this.ambiguousCharacters = Ln.getInstance(new Set(t.allowedLocales));
  }
  getCandidateCodePoints() {
    if (this.options.nonBasicASCII)
      return "allNonBasicAscii";
    const t = /* @__PURE__ */ new Set();
    if (this.options.invisibleCharacters)
      for (const n of cn.codePoints)
        Aa(String.fromCodePoint(n)) || t.add(n);
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
      return we.None;
    if (this.options.nonBasicASCII)
      return we.NonBasicASCII;
    let r = !1, s = !1;
    if (n)
      for (const a of n) {
        const l = a.codePointAt(0), o = Ou(a);
        r = r || o, !o && !this.ambiguousCharacters.isAmbiguous(l) && !cn.isInvisibleCharacter(l) && (s = !0);
      }
    return !r && s ? we.None : this.options.invisibleCharacters && !Aa(t) && cn.isInvisibleCharacter(i) ? we.Invisible : this.options.ambiguousCharacters && this.ambiguousCharacters.isAmbiguous(i) ? we.Ambiguous : we.None;
  }
}
function Aa(e) {
  return e === " " || e === `
` || e === "	";
}
var we;
(function(e) {
  e[e.None = 0] = "None", e[e.NonBasicASCII = 1] = "NonBasicASCII", e[e.Invisible = 2] = "Invisible", e[e.Ambiguous = 3] = "Ambiguous";
})(we || (we = {}));
class Bn {
  constructor(t, n, i) {
    this.changes = t, this.moves = n, this.hitTimeout = i;
  }
}
class wr {
  constructor(t, n) {
    this.lineRangeMapping = t, this.changes = n;
  }
  flip() {
    return new wr(this.lineRangeMapping.flip(), this.changes.map((t) => t.flip()));
  }
}
class Z {
  static addRange(t, n) {
    let i = 0;
    for (; i < n.length && n[i].endExclusive < t.start; )
      i++;
    let r = i;
    for (; r < n.length && n[r].start <= t.endExclusive; )
      r++;
    if (i === r)
      n.splice(i, 0, t);
    else {
      const s = Math.min(t.start, n[i].start), a = Math.max(t.endExclusive, n[r - 1].endExclusive);
      n.splice(i, r - i, new Z(s, a));
    }
  }
  static tryCreate(t, n) {
    if (!(t > n))
      return new Z(t, n);
  }
  static ofLength(t) {
    return new Z(0, t);
  }
  static ofStartAndLength(t, n) {
    return new Z(t, t + n);
  }
  constructor(t, n) {
    if (this.start = t, this.endExclusive = n, t > n)
      throw new ye(`Invalid range: ${this.toString()}`);
  }
  get isEmpty() {
    return this.start === this.endExclusive;
  }
  delta(t) {
    return new Z(this.start + t, this.endExclusive + t);
  }
  deltaStart(t) {
    return new Z(this.start + t, this.endExclusive);
  }
  deltaEnd(t) {
    return new Z(this.start, this.endExclusive + t);
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
    return new Z(
      Math.min(this.start, t.start),
      Math.max(this.endExclusive, t.endExclusive)
    );
  }
  intersect(t) {
    const n = Math.max(this.start, t.start), i = Math.min(this.endExclusive, t.endExclusive);
    if (n <= i)
      return new Z(n, i);
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
      throw new ye(`Invalid clipping range: ${this.toString()}`);
    return Math.max(this.start, Math.min(this.endExclusive - 1, t));
  }
  clipCyclic(t) {
    if (this.isEmpty)
      throw new ye(`Invalid clipping range: ${this.toString()}`);
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
class j {
  static fromRange(t) {
    return new j(t.startLineNumber, t.endLineNumber);
  }
  static fromRangeInclusive(t) {
    return new j(t.startLineNumber, t.endLineNumber + 1);
  }
  static subtract(t, n) {
    return n ? t.startLineNumber < n.startLineNumber && n.endLineNumberExclusive < t.endLineNumberExclusive ? [
      new j(t.startLineNumber, n.startLineNumber),
      new j(n.endLineNumberExclusive, t.endLineNumberExclusive)
    ] : n.startLineNumber <= t.startLineNumber && t.endLineNumberExclusive <= n.endLineNumberExclusive ? [] : n.endLineNumberExclusive < t.endLineNumberExclusive ? [new j(
      Math.max(n.endLineNumberExclusive, t.startLineNumber),
      t.endLineNumberExclusive
    )] : [new j(t.startLineNumber, Math.min(n.startLineNumber, t.endLineNumberExclusive))] : [t];
  }
  static joinMany(t) {
    if (t.length === 0)
      return [];
    let n = new Je(t[0].slice());
    for (let i = 1; i < t.length; i++)
      n = n.getUnion(new Je(t[i].slice()));
    return n.ranges;
  }
  static join(t) {
    if (t.length === 0)
      throw new ye("lineRanges cannot be empty");
    let n = t[0].startLineNumber, i = t[0].endLineNumberExclusive;
    for (let r = 1; r < t.length; r++)
      n = Math.min(n, t[r].startLineNumber), i = Math.max(i, t[r].endLineNumberExclusive);
    return new j(n, i);
  }
  static ofLength(t, n) {
    return new j(t, t + n);
  }
  static deserialize(t) {
    return new j(t[0], t[1]);
  }
  constructor(t, n) {
    if (t > n)
      throw new ye(
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
    return new j(this.startLineNumber + t, this.endLineNumberExclusive + t);
  }
  deltaLength(t) {
    return new j(this.startLineNumber, this.endLineNumberExclusive + t);
  }
  get length() {
    return this.endLineNumberExclusive - this.startLineNumber;
  }
  join(t) {
    return new j(
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
      return new j(n, i);
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
    return this.isEmpty ? null : new J(
      this.startLineNumber,
      1,
      this.endLineNumberExclusive - 1,
      Number.MAX_SAFE_INTEGER
    );
  }
  toExclusiveRange() {
    return new J(this.startLineNumber, 1, this.endLineNumberExclusive, 1);
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
    return new Z(this.startLineNumber - 1, this.endLineNumberExclusive - 1);
  }
}
class Je {
  constructor(t = []) {
    this._normalizedRanges = t;
  }
  get ranges() {
    return this._normalizedRanges;
  }
  addRange(t) {
    if (t.length === 0)
      return;
    const n = xi(this._normalizedRanges, (r) => r.endLineNumberExclusive >= t.startLineNumber), i = dn(this._normalizedRanges, (r) => r.startLineNumber <= t.endLineNumberExclusive) + 1;
    if (n === i)
      this._normalizedRanges.splice(n, 0, t);
    else if (n === i - 1) {
      const r = this._normalizedRanges[n];
      this._normalizedRanges[n] = r.join(t);
    } else {
      const r = this._normalizedRanges[n].join(this._normalizedRanges[i - 1]).join(t);
      this._normalizedRanges.splice(n, i - n, r);
    }
  }
  contains(t) {
    const n = zt(this._normalizedRanges, (i) => i.startLineNumber <= t);
    return !!n && n.endLineNumberExclusive > t;
  }
  intersects(t) {
    const n = zt(this._normalizedRanges, (i) => i.startLineNumber < t.endLineNumberExclusive);
    return !!n && n.endLineNumberExclusive > t.startLineNumber;
  }
  getUnion(t) {
    if (this._normalizedRanges.length === 0)
      return t;
    if (t._normalizedRanges.length === 0)
      return this;
    const n = [];
    let i = 0, r = 0, s = null;
    for (; i < this._normalizedRanges.length || r < t._normalizedRanges.length; ) {
      let a = null;
      if (i < this._normalizedRanges.length && r < t._normalizedRanges.length) {
        const l = this._normalizedRanges[i], o = t._normalizedRanges[r];
        l.startLineNumber < o.startLineNumber ? (a = l, i++) : (a = o, r++);
      } else i < this._normalizedRanges.length ? (a = this._normalizedRanges[i], i++) : (a = t._normalizedRanges[r], r++);
      s === null ? s = a : s.endLineNumberExclusive >= a.startLineNumber ? s = new j(
        s.startLineNumber,
        Math.max(s.endLineNumberExclusive, a.endLineNumberExclusive)
      ) : (n.push(s), s = a);
    }
    return s !== null && n.push(s), new Je(n);
  }
  subtractFrom(t) {
    const n = xi(this._normalizedRanges, (a) => a.endLineNumberExclusive >= t.startLineNumber), i = dn(this._normalizedRanges, (a) => a.startLineNumber <= t.endLineNumberExclusive) + 1;
    if (n === i)
      return new Je([t]);
    const r = [];
    let s = t.startLineNumber;
    for (let a = n; a < i; a++) {
      const l = this._normalizedRanges[a];
      l.startLineNumber > s && r.push(new j(s, l.startLineNumber)), s = l.endLineNumberExclusive;
    }
    return s < t.endLineNumberExclusive && r.push(new j(s, t.endLineNumberExclusive)), new Je(r);
  }
  toString() {
    return this._normalizedRanges.map((t) => t.toString()).join(", ");
  }
  getIntersection(t) {
    const n = [];
    let i = 0, r = 0;
    for (; i < this._normalizedRanges.length && r < t._normalizedRanges.length; ) {
      const s = this._normalizedRanges[i], a = t._normalizedRanges[r], l = s.intersect(a);
      l && !l.isEmpty && n.push(l), s.endLineNumberExclusive < a.endLineNumberExclusive ? i++ : r++;
    }
    return new Je(n);
  }
  getWithDelta(t) {
    return new Je(this._normalizedRanges.map((n) => n.delta(t)));
  }
}
const Se = class Se {
  static lengthDiffNonNegative(t, n) {
    return n.isLessThan(t) ? Se.zero : t.lineCount === n.lineCount ? new Se(0, n.columnCount - t.columnCount) : new Se(n.lineCount - t.lineCount, n.columnCount);
  }
  static betweenPositions(t, n) {
    return t.lineNumber === n.lineNumber ? new Se(0, n.column - t.column) : new Se(n.lineNumber - t.lineNumber, n.column - 1);
  }
  static ofRange(t) {
    return Se.betweenPositions(t.getStartPosition(), t.getEndPosition());
  }
  static ofText(t) {
    let n = 0, i = 0;
    for (const r of t)
      r === `
` ? (n++, i = 0) : i++;
    return new Se(n, i);
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
    return t.lineCount === 0 ? new Se(this.lineCount, this.columnCount + t.columnCount) : new Se(this.lineCount + t.lineCount, t.columnCount);
  }
  createRange(t) {
    return this.lineCount === 0 ? new J(
      t.lineNumber,
      t.column,
      t.lineNumber,
      t.column + this.columnCount
    ) : new J(
      t.lineNumber,
      t.column,
      t.lineNumber + this.lineCount,
      this.columnCount + 1
    );
  }
  toRange() {
    return new J(1, 1, this.lineCount + 1, this.columnCount + 1);
  }
  addToPosition(t) {
    return this.lineCount === 0 ? new se(t.lineNumber, t.column + this.columnCount) : new se(t.lineNumber + this.lineCount, this.columnCount + 1);
  }
  toString() {
    return `${this.lineCount},${this.columnCount}`;
  }
};
Se.zero = new Se(0, 0);
let xa = Se;
class Wc {
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
class Fe {
  static inverse(t, n, i) {
    const r = [];
    let s = 1, a = 1;
    for (const o of t) {
      const u = new Fe(new j(s, o.original.startLineNumber), new j(a, o.modified.startLineNumber));
      u.modified.isEmpty || r.push(u), s = o.original.endLineNumberExclusive, a = o.modified.endLineNumberExclusive;
    }
    const l = new Fe(new j(s, n + 1), new j(a, i + 1));
    return l.modified.isEmpty || r.push(l), r;
  }
  static clip(t, n, i) {
    const r = [];
    for (const s of t) {
      const a = s.original.intersect(n), l = s.modified.intersect(i);
      a && !a.isEmpty && l && !l.isEmpty && r.push(new Fe(a, l));
    }
    return r;
  }
  constructor(t, n) {
    this.original = t, this.modified = n;
  }
  toString() {
    return `{${this.original.toString()}->${this.modified.toString()}}`;
  }
  flip() {
    return new Fe(this.modified, this.original);
  }
  join(t) {
    return new Fe(this.original.join(t.original), this.modified.join(t.modified));
  }
  get changedLineCount() {
    return Math.max(this.original.length, this.modified.length);
  }
  toRangeMapping() {
    const t = this.original.toInclusiveRange(), n = this.modified.toInclusiveRange();
    if (t && n)
      return new We(t, n);
    if (this.original.startLineNumber === 1 || this.modified.startLineNumber === 1) {
      if (!(this.modified.startLineNumber === 1 && this.original.startLineNumber === 1))
        throw new ye("not a valid diff");
      return new We(new J(this.original.startLineNumber, 1, this.original.endLineNumberExclusive, 1), new J(this.modified.startLineNumber, 1, this.modified.endLineNumberExclusive, 1));
    } else
      return new We(new J(
        this.original.startLineNumber - 1,
        Number.MAX_SAFE_INTEGER,
        this.original.endLineNumberExclusive - 1,
        Number.MAX_SAFE_INTEGER
      ), new J(
        this.modified.startLineNumber - 1,
        Number.MAX_SAFE_INTEGER,
        this.modified.endLineNumberExclusive - 1,
        Number.MAX_SAFE_INTEGER
      ));
  }
  toRangeMapping2(t, n) {
    if (Ea(this.original.endLineNumberExclusive, t) && Ea(this.modified.endLineNumberExclusive, n))
      return new We(new J(this.original.startLineNumber, 1, this.original.endLineNumberExclusive, 1), new J(this.modified.startLineNumber, 1, this.modified.endLineNumberExclusive, 1));
    if (!this.original.isEmpty && !this.modified.isEmpty)
      return new We(J.fromPositions(new se(this.original.startLineNumber, 1), Mt(new se(this.original.endLineNumberExclusive - 1, Number.MAX_SAFE_INTEGER), t)), J.fromPositions(new se(this.modified.startLineNumber, 1), Mt(new se(this.modified.endLineNumberExclusive - 1, Number.MAX_SAFE_INTEGER), n)));
    if (this.original.startLineNumber > 1 && this.modified.startLineNumber > 1)
      return new We(J.fromPositions(Mt(new se(this.original.startLineNumber - 1, Number.MAX_SAFE_INTEGER), t), Mt(new se(this.original.endLineNumberExclusive - 1, Number.MAX_SAFE_INTEGER), t)), J.fromPositions(Mt(new se(this.modified.startLineNumber - 1, Number.MAX_SAFE_INTEGER), n), Mt(new se(this.modified.endLineNumberExclusive - 1, Number.MAX_SAFE_INTEGER), n)));
    throw new ye();
  }
}
function Mt(e, t) {
  if (e.lineNumber < 1)
    return new se(1, 1);
  if (e.lineNumber > t.length)
    return new se(t.length, t[t.length - 1].length + 1);
  const n = t[e.lineNumber - 1];
  return e.column > n.length + 1 ? new se(e.lineNumber, n.length + 1) : e;
}
function Ea(e, t) {
  return e >= 1 && e <= t.length;
}
class et extends Fe {
  static fromRangeMappings(t) {
    const n = j.join(t.map((r) => j.fromRangeInclusive(r.originalRange))), i = j.join(t.map((r) => j.fromRangeInclusive(r.modifiedRange)));
    return new et(n, i, t);
  }
  constructor(t, n, i) {
    super(t, n), this.innerChanges = i;
  }
  flip() {
    var t;
    return new et(this.modified, this.original, (t = this.innerChanges) == null ? void 0 : t.map((n) => n.flip()));
  }
  withInnerChangesFromLineRanges() {
    return new et(this.original, this.modified, [this.toRangeMapping()]);
  }
}
class We {
  static assertSorted(t) {
    for (let n = 1; n < t.length; n++) {
      const i = t[n - 1], r = t[n];
      if (!(i.originalRange.getEndPosition().isBeforeOrEqual(r.originalRange.getStartPosition()) && i.modifiedRange.getEndPosition().isBeforeOrEqual(r.modifiedRange.getStartPosition())))
        throw new ye("Range mappings must be sorted");
    }
  }
  constructor(t, n) {
    this.originalRange = t, this.modifiedRange = n;
  }
  toString() {
    return `{${this.originalRange.toString()}->${this.modifiedRange.toString()}}`;
  }
  flip() {
    return new We(this.modifiedRange, this.originalRange);
  }
  toTextEdit(t) {
    const n = t.getValueOfRange(this.modifiedRange);
    return new Wc(this.originalRange, n);
  }
}
const jc = 3;
class Gc {
  computeDiff(t, n, i) {
    var o;
    const s = new Nl(t, n, {
      maxComputationTime: i.maxComputationTimeMs,
      shouldIgnoreTrimWhitespace: i.ignoreTrimWhitespace,
      shouldComputeCharChanges: !0,
      shouldMakePrettyDiff: !0,
      shouldPostProcessCharChanges: !0
    }).computeDiff(), a = [];
    let l = null;
    for (const u of s.changes) {
      let h;
      u.originalEndLineNumber === 0 ? h = new j(u.originalStartLineNumber + 1, u.originalStartLineNumber + 1) : h = new j(u.originalStartLineNumber, u.originalEndLineNumber + 1);
      let c;
      u.modifiedEndLineNumber === 0 ? c = new j(u.modifiedStartLineNumber + 1, u.modifiedStartLineNumber + 1) : c = new j(u.modifiedStartLineNumber, u.modifiedEndLineNumber + 1);
      let m = new et(h, c, (o = u.charChanges) == null ? void 0 : o.map((g) => new We(new J(
        g.originalStartLineNumber,
        g.originalStartColumn,
        g.originalEndLineNumber,
        g.originalEndColumn
      ), new J(
        g.modifiedStartLineNumber,
        g.modifiedStartColumn,
        g.modifiedEndLineNumber,
        g.modifiedEndColumn
      ))));
      l && (l.modified.endLineNumberExclusive === m.modified.startLineNumber || l.original.endLineNumberExclusive === m.original.startLineNumber) && (m = new et(
        l.original.join(m.original),
        l.modified.join(m.modified),
        l.innerChanges && m.innerChanges ? l.innerChanges.concat(m.innerChanges) : void 0
      ), a.pop()), a.push(m), l = m;
    }
    return Jn(() => vl(a, (u, h) => h.original.startLineNumber - u.original.endLineNumberExclusive === h.modified.startLineNumber - u.modified.endLineNumberExclusive && u.original.endLineNumberExclusive < h.original.startLineNumber && u.modified.endLineNumberExclusive < h.modified.startLineNumber)), new Bn(a, [], s.quitEarly);
  }
}
function Ll(e, t, n, i) {
  return new mt(e, t, n).ComputeDiff(i);
}
let ya = class {
  constructor(t) {
    const n = [], i = [];
    for (let r = 0, s = t.length; r < s; r++)
      n[r] = ji(t[r], 1), i[r] = Gi(t[r], 1);
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
    const r = [], s = [], a = [];
    let l = 0;
    for (let o = n; o <= i; o++) {
      const u = this.lines[o], h = t ? this._startColumns[o] : 1, c = t ? this._endColumns[o] : u.length + 1;
      for (let m = h; m < c; m++)
        r[l] = u.charCodeAt(m - 1), s[l] = o + 1, a[l] = m, l++;
      !t && o < i && (r[l] = S.LineFeed, s[l] = o + 1, a[l] = u.length + 1, l++);
    }
    return new zc(r, s, a);
  }
};
class zc {
  constructor(t, n, i) {
    this._charCodes = t, this._lineNumbers = n, this._columns = i;
  }
  toString() {
    return "[" + this._charCodes.map(
      (t, n) => (t === S.LineFeed ? "\\n" : String.fromCharCode(t)) + `-(${this._lineNumbers[n]},${this._columns[n]})`
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
    return t === -1 ? this.getStartLineNumber(t + 1) : (this._assertIndex(t, this._lineNumbers), this._charCodes[t] === S.LineFeed ? this._lineNumbers[t] + 1 : this._lineNumbers[t]);
  }
  getStartColumn(t) {
    return t > 0 && t === this._columns.length ? this.getEndColumn(t - 1) : (this._assertIndex(t, this._columns), this._columns[t]);
  }
  getEndColumn(t) {
    return t === -1 ? this.getStartColumn(t + 1) : (this._assertIndex(t, this._columns), this._charCodes[t] === S.LineFeed ? 1 : this._columns[t] + 1);
  }
}
class Wt {
  constructor(t, n, i, r, s, a, l, o) {
    this.originalStartLineNumber = t, this.originalStartColumn = n, this.originalEndLineNumber = i, this.originalEndColumn = r, this.modifiedStartLineNumber = s, this.modifiedStartColumn = a, this.modifiedEndLineNumber = l, this.modifiedEndColumn = o;
  }
  static createFromDiffChange(t, n, i) {
    const r = n.getStartLineNumber(t.originalStart), s = n.getStartColumn(t.originalStart), a = n.getEndLineNumber(t.originalStart + t.originalLength - 1), l = n.getEndColumn(t.originalStart + t.originalLength - 1), o = i.getStartLineNumber(t.modifiedStart), u = i.getStartColumn(t.modifiedStart), h = i.getEndLineNumber(t.modifiedStart + t.modifiedLength - 1), c = i.getEndColumn(t.modifiedStart + t.modifiedLength - 1);
    return new Wt(
      r,
      s,
      a,
      l,
      o,
      u,
      h,
      c
    );
  }
}
function Xc(e) {
  if (e.length <= 1)
    return e;
  const t = [e[0]];
  let n = t[0];
  for (let i = 1, r = e.length; i < r; i++) {
    const s = e[i], a = s.originalStart - (n.originalStart + n.originalLength), l = s.modifiedStart - (n.modifiedStart + n.modifiedLength);
    Math.min(a, l) < jc ? (n.originalLength = s.originalStart + s.originalLength - n.originalStart, n.modifiedLength = s.modifiedStart + s.modifiedLength - n.modifiedStart) : (t.push(s), n = s);
  }
  return t;
}
class hn {
  constructor(t, n, i, r, s) {
    this.originalStartLineNumber = t, this.originalEndLineNumber = n, this.modifiedStartLineNumber = i, this.modifiedEndLineNumber = r, this.charChanges = s;
  }
  static createFromDiffResult(t, n, i, r, s, a, l) {
    let o, u, h, c, m;
    if (n.originalLength === 0 ? (o = i.getStartLineNumber(n.originalStart) - 1, u = 0) : (o = i.getStartLineNumber(n.originalStart), u = i.getEndLineNumber(n.originalStart + n.originalLength - 1)), n.modifiedLength === 0 ? (h = r.getStartLineNumber(n.modifiedStart) - 1, c = 0) : (h = r.getStartLineNumber(n.modifiedStart), c = r.getEndLineNumber(n.modifiedStart + n.modifiedLength - 1)), a && n.originalLength > 0 && n.originalLength < 20 && n.modifiedLength > 0 && n.modifiedLength < 20 && s()) {
      const g = i.createCharSequence(t, n.originalStart, n.originalStart + n.originalLength - 1), d = r.createCharSequence(t, n.modifiedStart, n.modifiedStart + n.modifiedLength - 1);
      if (g.getElements().length > 0 && d.getElements().length > 0) {
        let p = Ll(g, d, s, !0).changes;
        l && (p = Xc(p)), m = [];
        for (let _ = 0, x = p.length; _ < x; _++)
          m.push(Wt.createFromDiffChange(p[_], g, d));
      }
    }
    return new hn(
      o,
      u,
      h,
      c,
      m
    );
  }
}
class Nl {
  constructor(t, n, i) {
    this.shouldComputeCharChanges = i.shouldComputeCharChanges, this.shouldPostProcessCharChanges = i.shouldPostProcessCharChanges, this.shouldIgnoreTrimWhitespace = i.shouldIgnoreTrimWhitespace, this.shouldMakePrettyDiff = i.shouldMakePrettyDiff, this.originalLines = t, this.modifiedLines = n, this.original = new ya(t), this.modified = new ya(n), this.continueLineDiff = Ra(i.maxComputationTime), this.continueCharDiff = Ra(i.maxComputationTime === 0 ? 0 : Math.min(i.maxComputationTime, 5e3));
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
    const t = Ll(this.original, this.modified, this.continueLineDiff, this.shouldMakePrettyDiff), n = t.changes, i = t.quitEarly;
    if (this.shouldIgnoreTrimWhitespace) {
      const l = [];
      for (let o = 0, u = n.length; o < u; o++)
        l.push(hn.createFromDiffResult(this.shouldIgnoreTrimWhitespace, n[o], this.original, this.modified, this.continueCharDiff, this.shouldComputeCharChanges, this.shouldPostProcessCharChanges));
      return {
        quitEarly: i,
        changes: l
      };
    }
    const r = [];
    let s = 0, a = 0;
    for (let l = -1, o = n.length; l < o; l++) {
      const u = l + 1 < o ? n[l + 1] : null, h = u ? u.originalStart : this.originalLines.length, c = u ? u.modifiedStart : this.modifiedLines.length;
      for (; s < h && a < c; ) {
        const m = this.originalLines[s], g = this.modifiedLines[a];
        if (m !== g) {
          {
            let d = ji(m, 1), p = ji(g, 1);
            for (; d > 1 && p > 1; ) {
              const _ = m.charCodeAt(d - 2), x = g.charCodeAt(p - 2);
              if (_ !== x)
                break;
              d--, p--;
            }
            (d > 1 || p > 1) && this._pushTrimWhitespaceCharChange(r, s + 1, 1, d, a + 1, 1, p);
          }
          {
            let d = Gi(m, 1), p = Gi(g, 1);
            const _ = m.length + 1, x = g.length + 1;
            for (; d < _ && p < x; ) {
              const R = m.charCodeAt(d - 1), v = m.charCodeAt(p - 1);
              if (R !== v)
                break;
              d++, p++;
            }
            (d < _ || p < x) && this._pushTrimWhitespaceCharChange(r, s + 1, d, _, a + 1, p, x);
          }
        }
        s++, a++;
      }
      u && (r.push(hn.createFromDiffResult(this.shouldIgnoreTrimWhitespace, u, this.original, this.modified, this.continueCharDiff, this.shouldComputeCharChanges, this.shouldPostProcessCharChanges)), s += u.originalLength, a += u.modifiedLength);
    }
    return {
      quitEarly: i,
      changes: r
    };
  }
  _pushTrimWhitespaceCharChange(t, n, i, r, s, a, l) {
    if (this._mergeTrimWhitespaceCharChange(t, n, i, r, s, a, l))
      return;
    let o;
    this.shouldComputeCharChanges && (o = [new Wt(
      n,
      i,
      n,
      r,
      s,
      a,
      s,
      l
    )]), t.push(new hn(
      n,
      n,
      s,
      s,
      o
    ));
  }
  _mergeTrimWhitespaceCharChange(t, n, i, r, s, a, l) {
    const o = t.length;
    if (o === 0)
      return !1;
    const u = t[o - 1];
    return u.originalEndLineNumber === 0 || u.modifiedEndLineNumber === 0 ? !1 : u.originalEndLineNumber === n && u.modifiedEndLineNumber === s ? (this.shouldComputeCharChanges && u.charChanges && u.charChanges.push(new Wt(
      n,
      i,
      n,
      r,
      s,
      a,
      s,
      l
    )), !0) : u.originalEndLineNumber + 1 === n && u.modifiedEndLineNumber + 1 === s ? (u.originalEndLineNumber = n, u.modifiedEndLineNumber = s, this.shouldComputeCharChanges && u.charChanges && u.charChanges.push(new Wt(
      n,
      i,
      n,
      r,
      s,
      a,
      s,
      l
    )), !0) : !1;
  }
}
function ji(e, t) {
  const n = Du(e);
  return n === -1 ? t : n + 1;
}
function Gi(e, t) {
  const n = Uu(e);
  return n === -1 ? t : n + 2;
}
function Ra(e) {
  if (e === 0)
    return () => !0;
  const t = Date.now();
  return () => Date.now() - t < e;
}
class tt {
  static trivial(t, n) {
    return new tt([new ae(Z.ofLength(t.length), Z.ofLength(n.length))], !1);
  }
  static trivialTimedOut(t, n) {
    return new tt([new ae(Z.ofLength(t.length), Z.ofLength(n.length))], !0);
  }
  constructor(t, n) {
    this.diffs = t, this.hitTimeout = n;
  }
}
class ae {
  static invert(t, n) {
    const i = [];
    return eu(t, (r, s) => {
      i.push(ae.fromOffsetPairs(r ? r.getEndExclusives() : Ce.zero, s ? s.getStarts() : new Ce(
        n,
        (r ? r.seq2Range.endExclusive - r.seq1Range.endExclusive : 0) + n
      )));
    }), i;
  }
  static fromOffsetPairs(t, n) {
    return new ae(new Z(t.offset1, n.offset1), new Z(t.offset2, n.offset2));
  }
  static assertSorted(t) {
    let n;
    for (const i of t) {
      if (n && !(n.seq1Range.endExclusive <= i.seq1Range.start && n.seq2Range.endExclusive <= i.seq2Range.start))
        throw new ye("Sequence diffs must be sorted");
      n = i;
    }
  }
  constructor(t, n) {
    this.seq1Range = t, this.seq2Range = n;
  }
  swap() {
    return new ae(this.seq2Range, this.seq1Range);
  }
  toString() {
    return `${this.seq1Range} <-> ${this.seq2Range}`;
  }
  join(t) {
    return new ae(this.seq1Range.join(t.seq1Range), this.seq2Range.join(t.seq2Range));
  }
  delta(t) {
    return t === 0 ? this : new ae(this.seq1Range.delta(t), this.seq2Range.delta(t));
  }
  deltaStart(t) {
    return t === 0 ? this : new ae(this.seq1Range.deltaStart(t), this.seq2Range.deltaStart(t));
  }
  deltaEnd(t) {
    return t === 0 ? this : new ae(this.seq1Range.deltaEnd(t), this.seq2Range.deltaEnd(t));
  }
  intersectsOrTouches(t) {
    return this.seq1Range.intersectsOrTouches(t.seq1Range) || this.seq2Range.intersectsOrTouches(t.seq2Range);
  }
  intersect(t) {
    const n = this.seq1Range.intersect(t.seq1Range), i = this.seq2Range.intersect(t.seq2Range);
    if (!(!n || !i))
      return new ae(n, i);
  }
  getStarts() {
    return new Ce(this.seq1Range.start, this.seq2Range.start);
  }
  getEndExclusives() {
    return new Ce(this.seq1Range.endExclusive, this.seq2Range.endExclusive);
  }
}
const Lt = class Lt {
  constructor(t, n) {
    this.offset1 = t, this.offset2 = n;
  }
  toString() {
    return `${this.offset1} <-> ${this.offset2}`;
  }
  delta(t) {
    return t === 0 ? this : new Lt(this.offset1 + t, this.offset2 + t);
  }
  equals(t) {
    return this.offset1 === t.offset1 && this.offset2 === t.offset2;
  }
};
Lt.zero = new Lt(0, 0), Lt.max = new Lt(Number.MAX_SAFE_INTEGER, Number.MAX_SAFE_INTEGER);
let Ce = Lt;
const ci = class ci {
  isValid() {
    return !0;
  }
};
ci.instance = new ci();
let Nn = ci;
class Jc {
  constructor(t) {
    if (this.timeout = t, this.startTime = Date.now(), this.valid = !0, t <= 0)
      throw new ye("timeout must be positive");
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
class pi {
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
function zi(e) {
  return e === S.Space || e === S.Tab;
}
const gn = class gn {
  static getKey(t) {
    let n = this.chrKeys.get(t);
    return n === void 0 && (n = this.chrKeys.size, this.chrKeys.set(t, n)), n;
  }
  constructor(t, n, i) {
    this.range = t, this.lines = n, this.source = i, this.histogram = [];
    let r = 0;
    for (let s = t.startLineNumber - 1; s < t.endLineNumberExclusive - 1; s++) {
      const a = n[s];
      for (let o = 0; o < a.length; o++) {
        r++;
        const u = a[o], h = gn.getKey(u);
        this.histogram[h] = (this.histogram[h] || 0) + 1;
      }
      r++;
      const l = gn.getKey(`
`);
      this.histogram[l] = (this.histogram[l] || 0) + 1;
    }
    this.totalCount = r;
  }
  computeSimilarity(t) {
    let n = 0;
    const i = Math.max(this.histogram.length, t.histogram.length);
    for (let r = 0; r < i; r++)
      n += Math.abs((this.histogram[r] ?? 0) - (t.histogram[r] ?? 0));
    return 1 - n / (this.totalCount + t.totalCount);
  }
};
gn.chrKeys = /* @__PURE__ */ new Map();
let Yn = gn;
class Yc {
  compute(t, n, i = Nn.instance, r) {
    if (t.length === 0 || n.length === 0)
      return tt.trivial(t, n);
    const s = new pi(t.length, n.length), a = new pi(t.length, n.length), l = new pi(t.length, n.length);
    for (let d = 0; d < t.length; d++)
      for (let p = 0; p < n.length; p++) {
        if (!i.isValid())
          return tt.trivialTimedOut(t, n);
        const _ = d === 0 ? 0 : s.get(d - 1, p), x = p === 0 ? 0 : s.get(d, p - 1);
        let R;
        t.getElement(d) === n.getElement(p) ? (d === 0 || p === 0 ? R = 0 : R = s.get(d - 1, p - 1), d > 0 && p > 0 && a.get(d - 1, p - 1) === 3 && (R += l.get(d - 1, p - 1)), R += r ? r(d, p) : 1) : R = -1;
        const v = Math.max(_, x, R);
        if (v === R) {
          const L = d > 0 && p > 0 ? l.get(d - 1, p - 1) : 0;
          l.set(d, p, L + 1), a.set(d, p, 3);
        } else v === _ ? (l.set(d, p, 0), a.set(d, p, 1)) : v === x && (l.set(d, p, 0), a.set(d, p, 2));
        s.set(d, p, v);
      }
    const o = [];
    let u = t.length, h = n.length;
    function c(d, p) {
      (d + 1 !== u || p + 1 !== h) && o.push(new ae(new Z(d + 1, u), new Z(p + 1, h))), u = d, h = p;
    }
    let m = t.length - 1, g = n.length - 1;
    for (; m >= 0 && g >= 0; )
      a.get(m, g) === 3 ? (c(m, g), m--, g--) : a.get(m, g) === 1 ? m-- : g--;
    return c(-1, -1), o.reverse(), new tt(o, !1);
  }
}
class wl {
  compute(t, n, i = Nn.instance) {
    if (t.length === 0 || n.length === 0)
      return tt.trivial(t, n);
    const r = t, s = n;
    function a(p, _) {
      for (; p < r.length && _ < s.length && r.getElement(p) === s.getElement(_); )
        p++, _++;
      return p;
    }
    let l = 0;
    const o = new Qc();
    o.set(0, a(0, 0));
    const u = new Zc();
    u.set(0, o.get(0) === 0 ? null : new ka(null, 0, 0, o.get(0)));
    let h = 0;
    e: for (; ; ) {
      if (l++, !i.isValid())
        return tt.trivialTimedOut(r, s);
      const p = -Math.min(l, s.length + l % 2), _ = Math.min(l, r.length + l % 2);
      for (h = p; h <= _; h += 2) {
        const x = h === _ ? -1 : o.get(h + 1), R = h === p ? -1 : o.get(h - 1) + 1, v = Math.min(Math.max(x, R), r.length), L = v - h;
        if (v > r.length || L > s.length)
          continue;
        const b = a(v, L);
        o.set(h, b);
        const E = v === x ? u.get(h + 1) : u.get(h - 1);
        if (u.set(h, b !== v ? new ka(E, v, L, b - v) : E), o.get(h) === r.length && o.get(h) - h === s.length)
          break e;
      }
    }
    let c = u.get(h);
    const m = [];
    let g = r.length, d = s.length;
    for (; ; ) {
      const p = c ? c.x + c.length : 0, _ = c ? c.y + c.length : 0;
      if ((p !== g || _ !== d) && m.push(new ae(new Z(p, g), new Z(_, d))), !c)
        break;
      g = c.x, d = c.y, c = c.prev;
    }
    return m.reverse(), new tt(m, !1);
  }
}
class ka {
  constructor(t, n, i, r) {
    this.prev = t, this.x = n, this.y = i, this.length = r;
  }
}
class Qc {
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
class Zc {
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
class Qn {
  constructor(t, n, i) {
    this.lines = t, this.range = n, this.considerWhitespaceChanges = i, this.elements = [], this.firstElementOffsetByLineIdx = [], this.lineStartOffsets = [], this.trimmedWsLengthsByLineIdx = [], this.firstElementOffsetByLineIdx.push(0);
    for (let r = this.range.startLineNumber; r <= this.range.endLineNumber; r++) {
      let s = t[r - 1], a = 0;
      r === this.range.startLineNumber && this.range.startColumn > 1 && (a = this.range.startColumn - 1, s = s.substring(a)), this.lineStartOffsets.push(a);
      let l = 0;
      if (!i) {
        const u = s.trimStart();
        l = s.length - u.length, s = u.trimEnd();
      }
      this.trimmedWsLengthsByLineIdx.push(l);
      const o = r === this.range.endLineNumber ? Math.min(this.range.endColumn - 1 - a - l, s.length) : s.length;
      for (let u = 0; u < o; u++)
        this.elements.push(s.charCodeAt(u));
      r < this.range.endLineNumber && (this.elements.push(10), this.firstElementOffsetByLineIdx.push(this.elements.length));
    }
  }
  toString() {
    return `Slice: "${this.text}"`;
  }
  get text() {
    return this.getText(new Z(0, this.length));
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
    const n = Sa(t > 0 ? this.elements[t - 1] : -1), i = Sa(t < this.elements.length ? this.elements[t] : -1);
    if (n === re.LineBreakCR && i === re.LineBreakLF)
      return 0;
    if (n === re.LineBreakLF)
      return 150;
    let r = 0;
    return n !== i && (r += 10, n === re.WordLower && i === re.WordUpper && (r += 1)), r += Ta(n), r += Ta(i), r;
  }
  translateOffset(t, n = "right") {
    const i = dn(this.firstElementOffsetByLineIdx, (s) => s <= t), r = t - this.firstElementOffsetByLineIdx[i];
    return new se(
      this.range.startLineNumber + i,
      1 + this.lineStartOffsets[i] + r + (r === 0 && n === "left" ? 0 : this.trimmedWsLengthsByLineIdx[i])
    );
  }
  translateRange(t) {
    const n = this.translateOffset(t.start, "right"), i = this.translateOffset(t.endExclusive, "left");
    return i.isBefore(n) ? J.fromPositions(i, i) : J.fromPositions(n, i);
  }
  findWordContaining(t) {
    if (t < 0 || t >= this.elements.length || !bi(this.elements[t]))
      return;
    let n = t;
    for (; n > 0 && bi(this.elements[n - 1]); )
      n--;
    let i = t;
    for (; i < this.elements.length && bi(this.elements[i]); )
      i++;
    return new Z(n, i);
  }
  countLinesIn(t) {
    return this.translateOffset(t.endExclusive).lineNumber - this.translateOffset(t.start).lineNumber;
  }
  isStronglyEqual(t, n) {
    return this.elements[t] === this.elements[n];
  }
  extendToFullLines(t) {
    const n = zt(this.firstElementOffsetByLineIdx, (r) => r <= t.start) ?? 0, i = Zl(this.firstElementOffsetByLineIdx, (r) => t.endExclusive <= r) ?? this.elements.length;
    return new Z(n, i);
  }
}
function bi(e) {
  return e >= S.a && e <= S.z || e >= S.A && e <= S.Z || e >= S.Digit0 && e <= S.Digit9;
}
var re;
(function(e) {
  e[e.WordLower = 0] = "WordLower", e[e.WordUpper = 1] = "WordUpper", e[e.WordNumber = 2] = "WordNumber", e[e.End = 3] = "End", e[e.Other = 4] = "Other", e[e.Separator = 5] = "Separator", e[e.Space = 6] = "Space", e[e.LineBreakCR = 7] = "LineBreakCR", e[e.LineBreakLF = 8] = "LineBreakLF";
})(re || (re = {}));
const Kc = {
  [re.WordLower]: 0,
  [re.WordUpper]: 0,
  [re.WordNumber]: 0,
  [re.End]: 10,
  [re.Other]: 2,
  [re.Separator]: 30,
  [re.Space]: 3,
  [re.LineBreakCR]: 10,
  [re.LineBreakLF]: 10
};
function Ta(e) {
  return Kc[e];
}
function Sa(e) {
  return e === S.LineFeed ? re.LineBreakLF : e === S.CarriageReturn ? re.LineBreakCR : zi(e) ? re.Space : e >= S.a && e <= S.z ? re.WordLower : e >= S.A && e <= S.Z ? re.WordUpper : e >= S.Digit0 && e <= S.Digit9 ? re.WordNumber : e === -1 ? re.End : e === S.Comma || e === S.Semicolon ? re.Separator : re.Other;
}
function Cc(e, t, n, i, r, s) {
  let { moves: a, excludedChanges: l } = t1(e, t, n, s);
  if (!s.isValid())
    return [];
  const o = e.filter((h) => !l.has(h)), u = n1(o, i, r, t, n, s);
  return nu(a, u), a = i1(a), a = a.filter((h) => {
    const c = h.original.toOffsetRange().slice(t).map((g) => g.trim());
    return c.join(`
`).length >= 15 && e1(c, (g) => g.length >= 2) >= 2;
  }), a = r1(e, a), a;
}
function e1(e, t) {
  let n = 0;
  for (const i of e)
    t(i) && n++;
  return n;
}
function t1(e, t, n, i) {
  const r = [], s = e.filter((o) => o.modified.isEmpty && o.original.length >= 3).map((o) => new Yn(o.original, t, o)), a = new Set(e.filter((o) => o.original.isEmpty && o.modified.length >= 3).map((o) => new Yn(o.modified, n, o))), l = /* @__PURE__ */ new Set();
  for (const o of s) {
    let u = -1, h;
    for (const c of a) {
      const m = o.computeSimilarity(c);
      m > u && (u = m, h = c);
    }
    if (u > 0.9 && h && (a.delete(h), r.push(new Fe(o.range, h.range)), l.add(o.source), l.add(h.source)), !i.isValid())
      return { moves: r, excludedChanges: l };
  }
  return { moves: r, excludedChanges: l };
}
function n1(e, t, n, i, r, s) {
  const a = [], l = new il();
  for (const m of e)
    for (let g = m.original.startLineNumber; g < m.original.endLineNumberExclusive - 2; g++) {
      const d = `${t[g - 1]}:${t[g + 1 - 1]}:${t[g + 2 - 1]}`;
      l.add(d, { range: new j(g, g + 3) });
    }
  const o = [];
  e.sort(on((m) => m.modified.startLineNumber, ln));
  for (const m of e) {
    let g = [];
    for (let d = m.modified.startLineNumber; d < m.modified.endLineNumberExclusive - 2; d++) {
      const p = `${n[d - 1]}:${n[d + 1 - 1]}:${n[d + 2 - 1]}`, _ = new j(d, d + 3), x = [];
      l.forEach(p, ({ range: R }) => {
        for (const L of g)
          if (L.originalLineRange.endLineNumberExclusive + 1 === R.endLineNumberExclusive && L.modifiedLineRange.endLineNumberExclusive + 1 === _.endLineNumberExclusive) {
            L.originalLineRange = new j(
              L.originalLineRange.startLineNumber,
              R.endLineNumberExclusive
            ), L.modifiedLineRange = new j(
              L.modifiedLineRange.startLineNumber,
              _.endLineNumberExclusive
            ), x.push(L);
            return;
          }
        const v = {
          modifiedLineRange: _,
          originalLineRange: R
        };
        o.push(v), x.push(v);
      }), g = x;
    }
    if (!s.isValid())
      return [];
  }
  o.sort(iu(on((m) => m.modifiedLineRange.length, ln)));
  const u = new Je(), h = new Je();
  for (const m of o) {
    const g = m.modifiedLineRange.startLineNumber - m.originalLineRange.startLineNumber, d = u.subtractFrom(m.modifiedLineRange), p = h.subtractFrom(m.originalLineRange).getWithDelta(g), _ = d.getIntersection(p);
    for (const x of _.ranges) {
      if (x.length < 3)
        continue;
      const R = x, v = x.delta(-g);
      a.push(new Fe(v, R)), u.addRange(R), h.addRange(v);
    }
  }
  a.sort(on((m) => m.original.startLineNumber, ln));
  const c = new Vn(e);
  for (let m = 0; m < a.length; m++) {
    const g = a[m], d = c.findLastMonotonous((E) => E.original.startLineNumber <= g.original.startLineNumber), p = zt(e, (E) => E.modified.startLineNumber <= g.modified.startLineNumber), _ = Math.max(g.original.startLineNumber - d.original.startLineNumber, g.modified.startLineNumber - p.modified.startLineNumber), x = c.findLastMonotonous((E) => E.original.startLineNumber < g.original.endLineNumberExclusive), R = zt(e, (E) => E.modified.startLineNumber < g.modified.endLineNumberExclusive), v = Math.max(x.original.endLineNumberExclusive - g.original.endLineNumberExclusive, R.modified.endLineNumberExclusive - g.modified.endLineNumberExclusive);
    let L;
    for (L = 0; L < _; L++) {
      const E = g.original.startLineNumber - L - 1, k = g.modified.startLineNumber - L - 1;
      if (E > i.length || k > r.length || u.contains(k) || h.contains(E) || !Ma(i[E - 1], r[k - 1], s))
        break;
    }
    L > 0 && (h.addRange(new j(g.original.startLineNumber - L, g.original.startLineNumber)), u.addRange(new j(g.modified.startLineNumber - L, g.modified.startLineNumber)));
    let b;
    for (b = 0; b < v; b++) {
      const E = g.original.endLineNumberExclusive + b, k = g.modified.endLineNumberExclusive + b;
      if (E > i.length || k > r.length || u.contains(k) || h.contains(E) || !Ma(i[E - 1], r[k - 1], s))
        break;
    }
    b > 0 && (h.addRange(new j(
      g.original.endLineNumberExclusive,
      g.original.endLineNumberExclusive + b
    )), u.addRange(new j(
      g.modified.endLineNumberExclusive,
      g.modified.endLineNumberExclusive + b
    ))), (L > 0 || b > 0) && (a[m] = new Fe(new j(
      g.original.startLineNumber - L,
      g.original.endLineNumberExclusive + b
    ), new j(
      g.modified.startLineNumber - L,
      g.modified.endLineNumberExclusive + b
    )));
  }
  return a;
}
function Ma(e, t, n) {
  if (e.trim() === t.trim())
    return !0;
  if (e.length > 300 && t.length > 300)
    return !1;
  const r = new wl().compute(new Qn([e], new J(1, 1, 1, e.length), !1), new Qn([t], new J(1, 1, 1, t.length), !1), n);
  let s = 0;
  const a = ae.invert(r.diffs, e.length);
  for (const h of a)
    h.seq1Range.forEach((c) => {
      zi(e.charCodeAt(c)) || s++;
    });
  function l(h) {
    let c = 0;
    for (let m = 0; m < e.length; m++)
      zi(h.charCodeAt(m)) || c++;
    return c;
  }
  const o = l(e.length > t.length ? e : t);
  return s / o > 0.6 && o > 10;
}
function i1(e) {
  if (e.length === 0)
    return e;
  e.sort(on((n) => n.original.startLineNumber, ln));
  const t = [e[0]];
  for (let n = 1; n < e.length; n++) {
    const i = t[t.length - 1], r = e[n], s = r.original.startLineNumber - i.original.endLineNumberExclusive, a = r.modified.startLineNumber - i.modified.endLineNumberExclusive;
    if (s >= 0 && a >= 0 && s + a <= 2) {
      t[t.length - 1] = i.join(r);
      continue;
    }
    t.push(r);
  }
  return t;
}
function r1(e, t) {
  const n = new Vn(e);
  return t = t.filter((i) => {
    const r = n.findLastMonotonous((l) => l.original.startLineNumber < i.original.endLineNumberExclusive) || new Fe(new j(1, 1), new j(1, 1)), s = zt(e, (l) => l.modified.startLineNumber < i.modified.endLineNumberExclusive);
    return r !== s;
  }), t;
}
function Ia(e, t, n) {
  let i = n;
  return i = Da(e, t, i), i = Da(e, t, i), i = s1(e, t, i), i;
}
function Da(e, t, n) {
  if (n.length === 0)
    return n;
  const i = [];
  i.push(n[0]);
  for (let s = 1; s < n.length; s++) {
    const a = i[i.length - 1];
    let l = n[s];
    if (l.seq1Range.isEmpty || l.seq2Range.isEmpty) {
      const o = l.seq1Range.start - a.seq1Range.endExclusive;
      let u;
      for (u = 1; u <= o && !(e.getElement(l.seq1Range.start - u) !== e.getElement(l.seq1Range.endExclusive - u) || t.getElement(l.seq2Range.start - u) !== t.getElement(l.seq2Range.endExclusive - u)); u++)
        ;
      if (u--, u === o) {
        i[i.length - 1] = new ae(new Z(a.seq1Range.start, l.seq1Range.endExclusive - o), new Z(a.seq2Range.start, l.seq2Range.endExclusive - o));
        continue;
      }
      l = l.delta(-u);
    }
    i.push(l);
  }
  const r = [];
  for (let s = 0; s < i.length - 1; s++) {
    const a = i[s + 1];
    let l = i[s];
    if (l.seq1Range.isEmpty || l.seq2Range.isEmpty) {
      const o = a.seq1Range.start - l.seq1Range.endExclusive;
      let u;
      for (u = 0; u < o && !(!e.isStronglyEqual(l.seq1Range.start + u, l.seq1Range.endExclusive + u) || !t.isStronglyEqual(l.seq2Range.start + u, l.seq2Range.endExclusive + u)); u++)
        ;
      if (u === o) {
        i[s + 1] = new ae(new Z(l.seq1Range.start + o, a.seq1Range.endExclusive), new Z(l.seq2Range.start + o, a.seq2Range.endExclusive));
        continue;
      }
      u > 0 && (l = l.delta(u));
    }
    r.push(l);
  }
  return i.length > 0 && r.push(i[i.length - 1]), r;
}
function s1(e, t, n) {
  if (!e.getBoundaryScore || !t.getBoundaryScore)
    return n;
  for (let i = 0; i < n.length; i++) {
    const r = i > 0 ? n[i - 1] : void 0, s = n[i], a = i + 1 < n.length ? n[i + 1] : void 0, l = new Z(
      r ? r.seq1Range.endExclusive + 1 : 0,
      a ? a.seq1Range.start - 1 : e.length
    ), o = new Z(
      r ? r.seq2Range.endExclusive + 1 : 0,
      a ? a.seq2Range.start - 1 : t.length
    );
    s.seq1Range.isEmpty ? n[i] = Ua(s, e, t, l, o) : s.seq2Range.isEmpty && (n[i] = Ua(s.swap(), t, e, o, l).swap());
  }
  return n;
}
function Ua(e, t, n, i, r) {
  let a = 1;
  for (; e.seq1Range.start - a >= i.start && e.seq2Range.start - a >= r.start && n.isStronglyEqual(e.seq2Range.start - a, e.seq2Range.endExclusive - a) && a < 100; )
    a++;
  a--;
  let l = 0;
  for (; e.seq1Range.start + l < i.endExclusive && e.seq2Range.endExclusive + l < r.endExclusive && n.isStronglyEqual(e.seq2Range.start + l, e.seq2Range.endExclusive + l) && l < 100; )
    l++;
  if (a === 0 && l === 0)
    return e;
  let o = 0, u = -1;
  for (let h = -a; h <= l; h++) {
    const c = e.seq2Range.start + h, m = e.seq2Range.endExclusive + h, g = e.seq1Range.start + h, d = t.getBoundaryScore(g) + n.getBoundaryScore(c) + n.getBoundaryScore(m);
    d > u && (u = d, o = h);
  }
  return e.delta(o);
}
function a1(e, t, n) {
  const i = [];
  for (const r of n) {
    const s = i[i.length - 1];
    if (!s) {
      i.push(r);
      continue;
    }
    r.seq1Range.start - s.seq1Range.endExclusive <= 2 || r.seq2Range.start - s.seq2Range.endExclusive <= 2 ? i[i.length - 1] = new ae(s.seq1Range.join(r.seq1Range), s.seq2Range.join(r.seq2Range)) : i.push(r);
  }
  return i;
}
function o1(e, t, n) {
  const i = ae.invert(n, e.length), r = [];
  let s = new Ce(0, 0);
  function a(o, u) {
    if (o.offset1 < s.offset1 || o.offset2 < s.offset2)
      return;
    const h = e.findWordContaining(o.offset1), c = t.findWordContaining(o.offset2);
    if (!h || !c)
      return;
    let m = new ae(h, c);
    const g = m.intersect(u);
    let d = g.seq1Range.length, p = g.seq2Range.length;
    for (; i.length > 0; ) {
      const _ = i[0];
      if (!(_.seq1Range.intersects(m.seq1Range) || _.seq2Range.intersects(m.seq2Range)))
        break;
      const R = e.findWordContaining(_.seq1Range.start), v = t.findWordContaining(_.seq2Range.start), L = new ae(R, v), b = L.intersect(_);
      if (d += b.seq1Range.length, p += b.seq2Range.length, m = m.join(L), m.seq1Range.endExclusive >= _.seq1Range.endExclusive)
        i.shift();
      else
        break;
    }
    d + p < (m.seq1Range.length + m.seq2Range.length) * 2 / 3 && r.push(m), s = m.getEndExclusives();
  }
  for (; i.length > 0; ) {
    const o = i.shift();
    o.seq1Range.isEmpty || (a(o.getStarts(), o), a(o.getEndExclusives().delta(-1), o));
  }
  return l1(n, r);
}
function l1(e, t) {
  const n = [];
  for (; e.length > 0 || t.length > 0; ) {
    const i = e[0], r = t[0];
    let s;
    i && (!r || i.seq1Range.start < r.seq1Range.start) ? s = e.shift() : s = t.shift(), n.length > 0 && n[n.length - 1].seq1Range.endExclusive >= s.seq1Range.start ? n[n.length - 1] = n[n.length - 1].join(s) : n.push(s);
  }
  return n;
}
function u1(e, t, n) {
  let i = n;
  if (i.length === 0)
    return i;
  let r = 0, s;
  do {
    s = !1;
    const l = [
      i[0]
    ];
    for (let o = 1; o < i.length; o++) {
      let c = function(g, d) {
        const p = new Z(h.seq1Range.endExclusive, u.seq1Range.start);
        return e.getText(p).replace(/\s/g, "").length <= 4 && (g.seq1Range.length + g.seq2Range.length > 5 || d.seq1Range.length + d.seq2Range.length > 5);
      };
      var a = c;
      const u = i[o], h = l[l.length - 1];
      c(h, u) ? (s = !0, l[l.length - 1] = l[l.length - 1].join(u)) : l.push(u);
    }
    i = l;
  } while (r++ < 10 && s);
  return i;
}
function c1(e, t, n) {
  let i = n;
  if (i.length === 0)
    return i;
  let r = 0, s;
  do {
    s = !1;
    const o = [
      i[0]
    ];
    for (let u = 1; u < i.length; u++) {
      let m = function(d, p) {
        const _ = new Z(c.seq1Range.endExclusive, h.seq1Range.start);
        if (e.countLinesIn(_) > 5 || _.length > 500)
          return !1;
        const R = e.getText(_).trim();
        if (R.length > 20 || R.split(/\r\n|\r|\n/).length > 1)
          return !1;
        const v = e.countLinesIn(d.seq1Range), L = d.seq1Range.length, b = t.countLinesIn(d.seq2Range), E = d.seq2Range.length, k = e.countLinesIn(p.seq1Range), D = p.seq1Range.length, O = t.countLinesIn(p.seq2Range), W = p.seq2Range.length, U = 2 * 40 + 50;
        function A(y) {
          return Math.min(y, U);
        }
        return Math.pow(Math.pow(A(v * 40 + L), 1.5) + Math.pow(A(b * 40 + E), 1.5), 1.5) + Math.pow(Math.pow(A(k * 40 + D), 1.5) + Math.pow(A(O * 40 + W), 1.5), 1.5) > (U ** 1.5) ** 1.5 * 1.3;
      };
      var l = m;
      const h = i[u], c = o[o.length - 1];
      m(c, h) ? (s = !0, o[o.length - 1] = o[o.length - 1].join(h)) : o.push(h);
    }
    i = o;
  } while (r++ < 10 && s);
  const a = [];
  return tu(i, (o, u, h) => {
    let c = u;
    function m(R) {
      return R.length > 0 && R.trim().length <= 3 && u.seq1Range.length + u.seq2Range.length > 100;
    }
    const g = e.extendToFullLines(u.seq1Range), d = e.getText(new Z(g.start, u.seq1Range.start));
    m(d) && (c = c.deltaStart(-d.length));
    const p = e.getText(new Z(u.seq1Range.endExclusive, g.endExclusive));
    m(p) && (c = c.deltaEnd(p.length));
    const _ = ae.fromOffsetPairs(o ? o.getEndExclusives() : Ce.zero, h ? h.getStarts() : Ce.max), x = c.intersect(_);
    a.length > 0 && x.getStarts().equals(a[a.length - 1].getEndExclusives()) ? a[a.length - 1] = a[a.length - 1].join(x) : a.push(x);
  }), a;
}
class Fa {
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
    const n = t === 0 ? 0 : Pa(this.lines[t - 1]), i = t === this.lines.length ? 0 : Pa(this.lines[t]);
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
function Pa(e) {
  let t = 0;
  for (; t < e.length && (e.charCodeAt(t) === S.Space || e.charCodeAt(t) === S.Tab); )
    t++;
  return t;
}
class f1 {
  constructor() {
    this.dynamicProgrammingDiffing = new Yc(), this.myersDiffingAlgorithm = new wl();
  }
  computeDiff(t, n, i) {
    if (t.length <= 1 && Kl(t, n, (b, E) => b === E))
      return new Bn([], [], !1);
    if (t.length === 1 && t[0].length === 0 || n.length === 1 && n[0].length === 0)
      return new Bn([
        new et(new j(1, t.length + 1), new j(1, n.length + 1), [
          new We(new J(
            1,
            1,
            t.length,
            t[t.length - 1].length + 1
          ), new J(
            1,
            1,
            n.length,
            n[n.length - 1].length + 1
          ))
        ])
      ], [], !1);
    const r = i.maxComputationTimeMs === 0 ? Nn.instance : new Jc(i.maxComputationTimeMs), s = !i.ignoreTrimWhitespace, a = /* @__PURE__ */ new Map();
    function l(b) {
      let E = a.get(b);
      return E === void 0 && (E = a.size, a.set(b, E)), E;
    }
    const o = t.map((b) => l(b.trim())), u = n.map((b) => l(b.trim())), h = new Fa(o, t), c = new Fa(u, n), m = h.length + c.length < 1700 ? this.dynamicProgrammingDiffing.compute(h, c, r, (b, E) => t[b] === n[E] ? n[E].length === 0 ? 0.1 : 1 + Math.log(1 + n[E].length) : 0.99) : this.myersDiffingAlgorithm.compute(h, c, r);
    let g = m.diffs, d = m.hitTimeout;
    g = Ia(h, c, g), g = u1(h, c, g);
    const p = [], _ = (b) => {
      if (s)
        for (let E = 0; E < b; E++) {
          const k = x + E, D = R + E;
          if (t[k] !== n[D]) {
            const O = this.refineDiff(t, n, new ae(new Z(k, k + 1), new Z(D, D + 1)), r, s);
            for (const W of O.mappings)
              p.push(W);
            O.hitTimeout && (d = !0);
          }
        }
    };
    let x = 0, R = 0;
    for (const b of g) {
      Jn(() => b.seq1Range.start - x === b.seq2Range.start - R);
      const E = b.seq1Range.start - x;
      _(E), x = b.seq1Range.endExclusive, R = b.seq2Range.endExclusive;
      const k = this.refineDiff(t, n, b, r, s);
      k.hitTimeout && (d = !0);
      for (const D of k.mappings)
        p.push(D);
    }
    _(t.length - x);
    const v = Oa(p, t, n);
    let L = [];
    return i.computeMoves && (L = this.computeMoves(v, t, n, o, u, r, s)), Jn(() => {
      function b(k, D) {
        if (k.lineNumber < 1 || k.lineNumber > D.length)
          return !1;
        const O = D[k.lineNumber - 1];
        return !(k.column < 1 || k.column > O.length + 1);
      }
      function E(k, D) {
        return !(k.startLineNumber < 1 || k.startLineNumber > D.length + 1 || k.endLineNumberExclusive < 1 || k.endLineNumberExclusive > D.length + 1);
      }
      for (const k of v) {
        if (!k.innerChanges)
          return !1;
        for (const D of k.innerChanges)
          if (!(b(D.modifiedRange.getStartPosition(), n) && b(D.modifiedRange.getEndPosition(), n) && b(D.originalRange.getStartPosition(), t) && b(D.originalRange.getEndPosition(), t)))
            return !1;
        if (!E(k.modified, n) || !E(k.original, t))
          return !1;
      }
      return !0;
    }), new Bn(v, L, d);
  }
  computeMoves(t, n, i, r, s, a, l) {
    return Cc(t, n, i, r, s, a).map((h) => {
      const c = this.refineDiff(n, i, new ae(h.original.toOffsetRange(), h.modified.toOffsetRange()), a, l), m = Oa(c.mappings, n, i, !0);
      return new wr(h, m);
    });
  }
  refineDiff(t, n, i, r, s) {
    const l = m1(i).toRangeMapping2(t, n), o = new Qn(t, l.originalRange, s), u = new Qn(n, l.modifiedRange, s), h = o.length + u.length < 500 ? this.dynamicProgrammingDiffing.compute(o, u, r) : this.myersDiffingAlgorithm.compute(o, u, r);
    let c = h.diffs;
    return c = Ia(o, u, c), c = o1(o, u, c), c = a1(o, u, c), c = c1(o, u, c), {
      mappings: c.map((g) => new We(o.translateRange(g.seq1Range), u.translateRange(g.seq2Range))),
      hitTimeout: h.hitTimeout
    };
  }
}
function Oa(e, t, n, i = !1) {
  const r = [];
  for (const s of Cl(e.map((a) => h1(a, t, n)), (a, l) => a.original.overlapOrTouch(l.original) || a.modified.overlapOrTouch(l.modified))) {
    const a = s[0], l = s[s.length - 1];
    r.push(new et(
      a.original.join(l.original),
      a.modified.join(l.modified),
      s.map((o) => o.innerChanges[0])
    ));
  }
  return Jn(() => !i && r.length > 0 && (r[0].modified.startLineNumber !== r[0].original.startLineNumber || n.length - r[r.length - 1].modified.endLineNumberExclusive !== t.length - r[r.length - 1].original.endLineNumberExclusive) ? !1 : vl(r, (s, a) => a.original.startLineNumber - s.original.endLineNumberExclusive === a.modified.startLineNumber - s.modified.endLineNumberExclusive && s.original.endLineNumberExclusive < a.original.startLineNumber && s.modified.endLineNumberExclusive < a.modified.startLineNumber)), r;
}
function h1(e, t, n) {
  let i = 0, r = 0;
  e.modifiedRange.endColumn === 1 && e.originalRange.endColumn === 1 && e.originalRange.startLineNumber + i <= e.originalRange.endLineNumber && e.modifiedRange.startLineNumber + i <= e.modifiedRange.endLineNumber && (r = -1), e.modifiedRange.startColumn - 1 >= n[e.modifiedRange.startLineNumber - 1].length && e.originalRange.startColumn - 1 >= t[e.originalRange.startLineNumber - 1].length && e.originalRange.startLineNumber <= e.originalRange.endLineNumber + r && e.modifiedRange.startLineNumber <= e.modifiedRange.endLineNumber + r && (i = 1);
  const s = new j(
    e.originalRange.startLineNumber + i,
    e.originalRange.endLineNumber + 1 + r
  ), a = new j(
    e.modifiedRange.startLineNumber + i,
    e.modifiedRange.endLineNumber + 1 + r
  );
  return new et(s, a, [e]);
}
function m1(e) {
  return new Fe(new j(e.seq1Range.start + 1, e.seq1Range.endExclusive + 1), new j(e.seq2Range.start + 1, e.seq2Range.endExclusive + 1));
}
const _i = {
  getLegacy: () => new Gc(),
  getDefault: () => new f1()
};
function dt(e, t) {
  const n = Math.pow(10, t);
  return Math.round(e * n) / n;
}
class fe {
  constructor(t, n, i, r = 1) {
    this._rgbaBrand = void 0, this.r = Math.min(255, Math.max(0, t)) | 0, this.g = Math.min(255, Math.max(0, n)) | 0, this.b = Math.min(255, Math.max(0, i)) | 0, this.a = dt(Math.max(Math.min(1, r), 0), 3);
  }
  static equals(t, n) {
    return t.r === n.r && t.g === n.g && t.b === n.b && t.a === n.a;
  }
}
class Ue {
  constructor(t, n, i, r) {
    this._hslaBrand = void 0, this.h = Math.max(Math.min(360, t), 0) | 0, this.s = dt(Math.max(Math.min(1, n), 0), 3), this.l = dt(Math.max(Math.min(1, i), 0), 3), this.a = dt(Math.max(Math.min(1, r), 0), 3);
  }
  static equals(t, n) {
    return t.h === n.h && t.s === n.s && t.l === n.l && t.a === n.a;
  }
  static fromRGBA(t) {
    const n = t.r / 255, i = t.g / 255, r = t.b / 255, s = t.a, a = Math.max(n, i, r), l = Math.min(n, i, r);
    let o = 0, u = 0;
    const h = (l + a) / 2, c = a - l;
    if (c > 0) {
      switch (u = Math.min(h <= 0.5 ? c / (2 * h) : c / (2 - 2 * h), 1), a) {
        case n:
          o = (i - r) / c + (i < r ? 6 : 0);
          break;
        case i:
          o = (r - n) / c + 2;
          break;
        case r:
          o = (n - i) / c + 4;
          break;
      }
      o *= 60, o = Math.round(o);
    }
    return new Ue(o, u, h, s);
  }
  static _hue2rgb(t, n, i) {
    return i < 0 && (i += 1), i > 1 && (i -= 1), i < 1 / 6 ? t + (n - t) * 6 * i : i < 1 / 2 ? n : i < 2 / 3 ? t + (n - t) * (2 / 3 - i) * 6 : t;
  }
  static toRGBA(t) {
    const n = t.h / 360, { s: i, l: r, a: s } = t;
    let a, l, o;
    if (i === 0)
      a = l = o = r;
    else {
      const u = r < 0.5 ? r * (1 + i) : r + i - r * i, h = 2 * r - u;
      a = Ue._hue2rgb(h, u, n + 1 / 3), l = Ue._hue2rgb(h, u, n), o = Ue._hue2rgb(h, u, n - 1 / 3);
    }
    return new fe(Math.round(a * 255), Math.round(l * 255), Math.round(o * 255), s);
  }
}
class Ot {
  constructor(t, n, i, r) {
    this._hsvaBrand = void 0, this.h = Math.max(Math.min(360, t), 0) | 0, this.s = dt(Math.max(Math.min(1, n), 0), 3), this.v = dt(Math.max(Math.min(1, i), 0), 3), this.a = dt(Math.max(Math.min(1, r), 0), 3);
  }
  static equals(t, n) {
    return t.h === n.h && t.s === n.s && t.v === n.v && t.a === n.a;
  }
  static fromRGBA(t) {
    const n = t.r / 255, i = t.g / 255, r = t.b / 255, s = Math.max(n, i, r), a = Math.min(n, i, r), l = s - a, o = s === 0 ? 0 : l / s;
    let u;
    return l === 0 ? u = 0 : s === n ? u = ((i - r) / l % 6 + 6) % 6 : s === i ? u = (r - n) / l + 2 : u = (n - i) / l + 4, new Ot(Math.round(u * 60), o, s, t.a);
  }
  static toRGBA(t) {
    const { h: n, s: i, v: r, a: s } = t, a = r * i, l = a * (1 - Math.abs(n / 60 % 2 - 1)), o = r - a;
    let [u, h, c] = [0, 0, 0];
    return n < 60 ? (u = a, h = l) : n < 120 ? (u = l, h = a) : n < 180 ? (h = a, c = l) : n < 240 ? (h = l, c = a) : n < 300 ? (u = l, c = a) : n <= 360 && (u = a, c = l), u = Math.round((u + o) * 255), h = Math.round((h + o) * 255), c = Math.round((c + o) * 255), new fe(u, h, c, s);
  }
}
var Y;
let Zn = (Y = class {
  static fromHex(t) {
    return Y.Format.CSS.parseHex(t) || Y.red;
  }
  static equals(t, n) {
    return !t && !n ? !0 : !t || !n ? !1 : t.equals(n);
  }
  get hsla() {
    return this._hsla ? this._hsla : Ue.fromRGBA(this.rgba);
  }
  get hsva() {
    return this._hsva ? this._hsva : Ot.fromRGBA(this.rgba);
  }
  constructor(t) {
    if (t)
      if (t instanceof fe)
        this.rgba = t;
      else if (t instanceof Ue)
        this._hsla = t, this.rgba = Ue.toRGBA(t);
      else if (t instanceof Ot)
        this._hsva = t, this.rgba = Ot.toRGBA(t);
      else
        throw new Error("Invalid color ctor argument");
    else throw new Error("Color needs a value");
  }
  equals(t) {
    return !!t && fe.equals(this.rgba, t.rgba) && Ue.equals(this.hsla, t.hsla) && Ot.equals(this.hsva, t.hsva);
  }
  getRelativeLuminance() {
    const t = Y._relativeLuminanceForComponent(this.rgba.r), n = Y._relativeLuminanceForComponent(this.rgba.g), i = Y._relativeLuminanceForComponent(this.rgba.b), r = 0.2126 * t + 0.7152 * n + 0.0722 * i;
    return dt(r, 4);
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
    return new Y(new Ue(this.hsla.h, this.hsla.s, this.hsla.l + this.hsla.l * t, this.hsla.a));
  }
  darken(t) {
    return new Y(new Ue(this.hsla.h, this.hsla.s, this.hsla.l - this.hsla.l * t, this.hsla.a));
  }
  transparent(t) {
    const { r: n, g: i, b: r, a: s } = this.rgba;
    return new Y(new fe(n, i, r, s * t));
  }
  isTransparent() {
    return this.rgba.a === 0;
  }
  isOpaque() {
    return this.rgba.a === 1;
  }
  opposite() {
    return new Y(new fe(255 - this.rgba.r, 255 - this.rgba.g, 255 - this.rgba.b, this.rgba.a));
  }
  blend(t) {
    const n = t.rgba, i = this.rgba.a, r = n.a, s = i + r * (1 - i);
    if (s < 1e-6)
      return Y.transparent;
    const a = this.rgba.r * i / s + n.r * r * (1 - i) / s, l = this.rgba.g * i / s + n.g * r * (1 - i) / s, o = this.rgba.b * i / s + n.b * r * (1 - i) / s;
    return new Y(new fe(a, l, o, s));
  }
  makeOpaque(t) {
    if (this.isOpaque() || t.rgba.a !== 1)
      return this;
    const { r: n, g: i, b: r, a: s } = this.rgba;
    return new Y(new fe(
      t.rgba.r - s * (t.rgba.r - n),
      t.rgba.g - s * (t.rgba.g - i),
      t.rgba.b - s * (t.rgba.b - r),
      1
    ));
  }
  flatten(...t) {
    const n = t.reduceRight((i, r) => Y._flatten(r, i));
    return Y._flatten(this, n);
  }
  static _flatten(t, n) {
    const i = 1 - t.rgba.a;
    return new Y(new fe(
      i * n.rgba.r + t.rgba.a * t.rgba.r,
      i * n.rgba.g + t.rgba.a * t.rgba.g,
      i * n.rgba.b + t.rgba.a * t.rgba.b
    ));
  }
  toString() {
    return this._toString || (this._toString = Y.Format.CSS.format(this)), this._toString;
  }
  static getLighterColor(t, n, i) {
    if (t.isLighterThan(n))
      return t;
    i = i || 0.5;
    const r = t.getRelativeLuminance(), s = n.getRelativeLuminance();
    return i = i * (s - r) / s, t.lighten(i);
  }
  static getDarkerColor(t, n, i) {
    if (t.isDarkerThan(n))
      return t;
    i = i || 0.5;
    const r = t.getRelativeLuminance(), s = n.getRelativeLuminance();
    return i = i * (r - s) / r, t.darken(i);
  }
}, Y.white = new Y(new fe(255, 255, 255, 1)), Y.black = new Y(new fe(0, 0, 0, 1)), Y.red = new Y(new fe(255, 0, 0, 1)), Y.blue = new Y(new fe(0, 0, 255, 1)), Y.green = new Y(new fe(0, 255, 0, 1)), Y.cyan = new Y(new fe(0, 255, 255, 1)), Y.lightgrey = new Y(new fe(211, 211, 211, 1)), Y.transparent = new Y(new fe(0, 0, 0, 0)), Y);
(function(e) {
  (function(t) {
    (function(n) {
      function i(g) {
        return g.rgba.a === 1 ? `rgb(${g.rgba.r}, ${g.rgba.g}, ${g.rgba.b})` : e.Format.CSS.formatRGBA(g);
      }
      n.formatRGB = i;
      function r(g) {
        return `rgba(${g.rgba.r}, ${g.rgba.g}, ${g.rgba.b}, ${+g.rgba.a.toFixed(2)})`;
      }
      n.formatRGBA = r;
      function s(g) {
        return g.hsla.a === 1 ? `hsl(${g.hsla.h}, ${(g.hsla.s * 100).toFixed(2)}%, ${(g.hsla.l * 100).toFixed(2)}%)` : e.Format.CSS.formatHSLA(g);
      }
      n.formatHSL = s;
      function a(g) {
        return `hsla(${g.hsla.h}, ${(g.hsla.s * 100).toFixed(2)}%, ${(g.hsla.l * 100).toFixed(2)}%, ${g.hsla.a.toFixed(2)})`;
      }
      n.formatHSLA = a;
      function l(g) {
        const d = g.toString(16);
        return d.length !== 2 ? "0" + d : d;
      }
      function o(g) {
        return `#${l(g.rgba.r)}${l(g.rgba.g)}${l(g.rgba.b)}`;
      }
      n.formatHex = o;
      function u(g, d = !1) {
        return d && g.rgba.a === 1 ? e.Format.CSS.formatHex(g) : `#${l(g.rgba.r)}${l(g.rgba.g)}${l(g.rgba.b)}${l(Math.round(g.rgba.a * 255))}`;
      }
      n.formatHexA = u;
      function h(g) {
        return g.isOpaque() ? e.Format.CSS.formatHex(g) : e.Format.CSS.formatRGBA(g);
      }
      n.format = h;
      function c(g) {
        const d = g.length;
        if (d === 0 || g.charCodeAt(0) !== S.Hash)
          return null;
        if (d === 7) {
          const p = 16 * m(g.charCodeAt(1)) + m(g.charCodeAt(2)), _ = 16 * m(g.charCodeAt(3)) + m(g.charCodeAt(4)), x = 16 * m(g.charCodeAt(5)) + m(g.charCodeAt(6));
          return new e(new fe(p, _, x, 1));
        }
        if (d === 9) {
          const p = 16 * m(g.charCodeAt(1)) + m(g.charCodeAt(2)), _ = 16 * m(g.charCodeAt(3)) + m(g.charCodeAt(4)), x = 16 * m(g.charCodeAt(5)) + m(g.charCodeAt(6)), R = 16 * m(g.charCodeAt(7)) + m(g.charCodeAt(8));
          return new e(new fe(p, _, x, R / 255));
        }
        if (d === 4) {
          const p = m(g.charCodeAt(1)), _ = m(g.charCodeAt(2)), x = m(g.charCodeAt(3));
          return new e(new fe(16 * p + p, 16 * _ + _, 16 * x + x));
        }
        if (d === 5) {
          const p = m(g.charCodeAt(1)), _ = m(g.charCodeAt(2)), x = m(g.charCodeAt(3)), R = m(g.charCodeAt(4));
          return new e(new fe(16 * p + p, 16 * _ + _, 16 * x + x, (16 * R + R) / 255));
        }
        return null;
      }
      n.parseHex = c;
      function m(g) {
        switch (g) {
          case S.Digit0:
            return 0;
          case S.Digit1:
            return 1;
          case S.Digit2:
            return 2;
          case S.Digit3:
            return 3;
          case S.Digit4:
            return 4;
          case S.Digit5:
            return 5;
          case S.Digit6:
            return 6;
          case S.Digit7:
            return 7;
          case S.Digit8:
            return 8;
          case S.Digit9:
            return 9;
          case S.a:
            return 10;
          case S.A:
            return 10;
          case S.b:
            return 11;
          case S.B:
            return 11;
          case S.c:
            return 12;
          case S.C:
            return 12;
          case S.d:
            return 13;
          case S.D:
            return 13;
          case S.e:
            return 14;
          case S.E:
            return 14;
          case S.f:
            return 15;
          case S.F:
            return 15;
        }
        return 0;
      }
    })(t.CSS || (t.CSS = {}));
  })(e.Format || (e.Format = {}));
})(Zn || (Zn = {}));
function Al(e) {
  const t = [];
  for (const n of e) {
    const i = Number(n);
    (i || i === 0 && n.replace(/\s/g, "") !== "") && t.push(i);
  }
  return t;
}
function Ar(e, t, n, i) {
  return {
    red: e / 255,
    blue: n / 255,
    green: t / 255,
    alpha: i
  };
}
function tn(e, t) {
  const n = t.index, i = t[0].length;
  if (!n)
    return;
  const r = e.positionAt(n);
  return {
    startLineNumber: r.lineNumber,
    startColumn: r.column,
    endLineNumber: r.lineNumber,
    endColumn: r.column + i
  };
}
function g1(e, t) {
  if (!e)
    return;
  const n = Zn.Format.CSS.parseHex(t);
  if (n)
    return {
      range: e,
      color: Ar(n.rgba.r, n.rgba.g, n.rgba.b, n.rgba.a)
    };
}
function Ba(e, t, n) {
  if (!e || t.length !== 1)
    return;
  const r = t[0].values(), s = Al(r);
  return {
    range: e,
    color: Ar(s[0], s[1], s[2], n ? s[3] : 1)
  };
}
function Va(e, t, n) {
  if (!e || t.length !== 1)
    return;
  const r = t[0].values(), s = Al(r), a = new Zn(new Ue(
    s[0],
    s[1] / 100,
    s[2] / 100,
    n ? s[3] : 1
  ));
  return {
    range: e,
    color: Ar(a.rgba.r, a.rgba.g, a.rgba.b, a.rgba.a)
  };
}
function nn(e, t) {
  return typeof e == "string" ? [...e.matchAll(t)] : e.findMatches(t);
}
function d1(e) {
  const t = [], i = nn(e, /\b(rgb|rgba|hsl|hsla)(\([0-9\s,.\%]*\))|(#)([A-Fa-f0-9]{3})\b|(#)([A-Fa-f0-9]{4})\b|(#)([A-Fa-f0-9]{6})\b|(#)([A-Fa-f0-9]{8})\b/gm);
  if (i.length > 0)
    for (const r of i) {
      const s = r.filter((u) => u !== void 0), a = s[1], l = s[2];
      if (!l)
        continue;
      let o;
      if (a === "rgb") {
        const u = /^\(\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*\)$/gm;
        o = Ba(tn(e, r), nn(l, u), !1);
      } else if (a === "rgba") {
        const u = /^\(\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9][0-9]|[0-9])\s*,\s*(0[.][0-9]+|[.][0-9]+|[01][.]|[01])\s*\)$/gm;
        o = Ba(tn(e, r), nn(l, u), !0);
      } else if (a === "hsl") {
        const u = /^\(\s*(36[0]|3[0-5][0-9]|[12][0-9][0-9]|[1-9]?[0-9])\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*\)$/gm;
        o = Va(tn(e, r), nn(l, u), !1);
      } else if (a === "hsla") {
        const u = /^\(\s*(36[0]|3[0-5][0-9]|[12][0-9][0-9]|[1-9]?[0-9])\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*,\s*(100|\d{1,2}[.]\d*|\d{1,2})%\s*,\s*(0[.][0-9]+|[.][0-9]+|[01][.]|[01])\s*\)$/gm;
        o = Va(tn(e, r), nn(l, u), !0);
      } else a === "#" && (o = g1(tn(e, r), a + l));
      o && t.push(o);
    }
  return t;
}
function p1(e) {
  return !e || typeof e.getValue != "function" || typeof e.positionAt != "function" ? [] : d1(e);
}
const qa = new RegExp("\\bMARK:\\s*(.*)$", "d"), b1 = /^-+|-+$/g;
function _1(e, t) {
  var i;
  let n = [];
  if (t.findRegionSectionHeaders && ((i = t.foldingRules) != null && i.markers)) {
    const r = v1(e, t);
    n = n.concat(r);
  }
  if (t.findMarkSectionHeaders) {
    const r = L1(e);
    n = n.concat(r);
  }
  return n;
}
function v1(e, t) {
  const n = [], i = e.getLineCount();
  for (let r = 1; r <= i; r++) {
    const s = e.getLineContent(r), a = s.match(t.foldingRules.markers.start);
    if (a) {
      const l = { startLineNumber: r, startColumn: a[0].length + 1, endLineNumber: r, endColumn: s.length + 1 };
      if (l.endColumn > l.startColumn) {
        const o = {
          range: l,
          ...xl(s.substring(a[0].length)),
          shouldBeInComments: !1
        };
        (o.text || o.hasSeparatorLine) && n.push(o);
      }
    }
  }
  return n;
}
function L1(e) {
  const t = [], n = e.getLineCount();
  for (let i = 1; i <= n; i++) {
    const r = e.getLineContent(i);
    N1(r, i, t);
  }
  return t;
}
function N1(e, t, n) {
  qa.lastIndex = 0;
  const i = qa.exec(e);
  if (i) {
    const r = i.indices[1][0] + 1, s = i.indices[1][1] + 1, a = { startLineNumber: t, startColumn: r, endLineNumber: t, endColumn: s };
    if (a.endColumn > a.startColumn) {
      const l = {
        range: a,
        ...xl(i[1]),
        shouldBeInComments: !0
      };
      (l.text || l.hasSeparatorLine) && n.push(l);
    }
  }
}
function xl(e) {
  e = e.trim();
  const t = e.startsWith("-");
  return e = e.replace(b1, ""), { text: e, hasSeparatorLine: t };
}
class w1 extends dc {
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
      const r = this._lines[i], s = this.offsetAt(new se(i + 1, 1)), a = r.matchAll(t);
      for (const l of a)
        (l.index || l.index === 0) && (l.index = l.index + s), n.push(l);
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
    const i = vr(t.column, bl(n), this._lines[t.lineNumber - 1], 0);
    return i ? new J(
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
    let r = 0, s = "", a = 0, l = [];
    return {
      *[Symbol.iterator]() {
        for (; ; )
          if (a < l.length) {
            const o = s.substring(l[a].start, l[a].end);
            a += 1, yield o;
          } else if (r < n.length)
            s = n[r], l = i(s, t), a = 0, r += 1;
          else
            break;
      }
    };
  }
  getLineWords(t, n) {
    const i = this._lines[t - 1], r = this._wordenize(i, n), s = [];
    for (const a of r)
      s.push({
        word: i.substring(a.start, a.end),
        startColumn: a.start + 1,
        endColumn: a.end + 1
      });
    return s;
  }
  _wordenize(t, n) {
    const i = [];
    let r;
    for (n.lastIndex = 0; (r = n.exec(t)) && r[0].length !== 0; )
      i.push({ start: r.index, end: r.index + r[0].length });
    return i;
  }
  getValueInRange(t) {
    if (t = this._validateRange(t), t.startLineNumber === t.endLineNumber)
      return this._lines[t.startLineNumber - 1].substring(t.startColumn - 1, t.endColumn - 1);
    const n = this._eol, i = t.startLineNumber - 1, r = t.endLineNumber - 1, s = [];
    s.push(this._lines[i].substring(t.startColumn - 1));
    for (let a = i + 1; a < r; a++)
      s.push(this._lines[a]);
    return s.push(this._lines[r].substring(0, t.endColumn - 1)), s.join(n);
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
    if (!se.isIPosition(t))
      throw new Error("bad position");
    let { lineNumber: n, column: i } = t, r = !1;
    if (n < 1)
      n = 1, i = 1, r = !0;
    else if (n > this._lines.length)
      n = this._lines.length, i = this._lines[n - 1].length + 1, r = !0;
    else {
      const s = this._lines[n - 1].length + 1;
      i < 1 ? (i = 1, r = !0) : i > s && (i = s, r = !0);
    }
    return r ? { lineNumber: n, column: i } : t;
  }
}
const ht = class ht {
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
    this._models[t.url] = new w1(_r.parse(t.url), t.lines, t.EOL, t.versionId);
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
    const r = this._getModel(t);
    return r ? Hc.computeUnicodeHighlights(r, n, i) : { ranges: [], hasMore: !1, ambiguousCharacterCount: 0, invisibleCharacterCount: 0, nonBasicAsciiCharacterCount: 0 };
  }
  async findSectionHeaders(t, n) {
    const i = this._getModel(t);
    return i ? _1(i, n) : [];
  }
  async computeDiff(t, n, i, r) {
    const s = this._getModel(t), a = this._getModel(n);
    return !s || !a ? null : ht.computeDiff(s, a, i, r);
  }
  static computeDiff(t, n, i, r) {
    const s = r === "advanced" ? _i.getDefault() : _i.getLegacy(), a = t.getLinesContent(), l = n.getLinesContent(), o = s.computeDiff(a, l, i), u = o.changes.length > 0 ? !1 : this._modelsAreIdentical(t, n);
    function h(c) {
      return c.map(
        (m) => {
          var g;
          return [m.original.startLineNumber, m.original.endLineNumberExclusive, m.modified.startLineNumber, m.modified.endLineNumberExclusive, (g = m.innerChanges) == null ? void 0 : g.map((d) => [
            d.originalRange.startLineNumber,
            d.originalRange.startColumn,
            d.originalRange.endLineNumber,
            d.originalRange.endColumn,
            d.modifiedRange.startLineNumber,
            d.modifiedRange.startColumn,
            d.modifiedRange.endLineNumber,
            d.modifiedRange.endColumn
          ])];
        }
      );
    }
    return {
      identical: u,
      quitEarly: o.hitTimeout,
      changes: h(o.changes),
      moves: o.moves.map((c) => [
        c.lineRangeMapping.original.startLineNumber,
        c.lineRangeMapping.original.endLineNumberExclusive,
        c.lineRangeMapping.modified.startLineNumber,
        c.lineRangeMapping.modified.endLineNumberExclusive,
        h(c.changes)
      ])
    };
  }
  static _modelsAreIdentical(t, n) {
    const i = t.getLineCount(), r = n.getLineCount();
    if (i !== r)
      return !1;
    for (let s = 1; s <= i; s++) {
      const a = t.getLineContent(s), l = n.getLineContent(s);
      if (a !== l)
        return !1;
    }
    return !0;
  }
  async computeDirtyDiff(t, n, i) {
    const r = this._getModel(t), s = this._getModel(n);
    if (!r || !s)
      return null;
    const a = r.getLinesContent(), l = s.getLinesContent();
    return new Nl(a, l, {
      shouldComputeCharChanges: !1,
      shouldPostProcessCharChanges: !1,
      shouldIgnoreTrimWhitespace: i,
      shouldMakePrettyDiff: !0,
      maxComputationTime: 1e3
    }).computeDiff().changes;
  }
  async computeMoreMinimalEdits(t, n, i) {
    const r = this._getModel(t);
    if (!r)
      return n;
    const s = [];
    let a;
    n = n.slice(0).sort((o, u) => {
      if (o.range && u.range)
        return J.compareRangesUsingStarts(o.range, u.range);
      const h = o.range ? 0 : 1, c = u.range ? 0 : 1;
      return h - c;
    });
    let l = 0;
    for (let o = 1; o < n.length; o++)
      J.getEndPosition(n[l].range).equals(J.getStartPosition(n[o].range)) ? (n[l].range = J.fromPositions(J.getStartPosition(n[l].range), J.getEndPosition(n[o].range)), n[l].text += n[o].text) : (l++, n[l] = n[o]);
    n.length = l + 1;
    for (let { range: o, text: u, eol: h } of n) {
      if (typeof h == "number" && (a = h), J.isEmpty(o) && !u)
        continue;
      const c = r.getValueInRange(o);
      if (u = u.replace(/\r\n|\n|\r/g, r.eol), c === u)
        continue;
      if (Math.max(u.length, c.length) > ht._diffLimit) {
        s.push({ range: o, text: u });
        continue;
      }
      const m = Ju(c, u, i), g = r.offsetAt(J.lift(o).getStartPosition());
      for (const d of m) {
        const p = r.positionAt(g + d.originalStart), _ = r.positionAt(g + d.originalStart + d.originalLength), x = {
          text: u.substr(d.modifiedStart, d.modifiedLength),
          range: { startLineNumber: p.lineNumber, startColumn: p.column, endLineNumber: _.lineNumber, endColumn: _.column }
        };
        r.getValueInRange(x.range) !== x.text && s.push(x);
      }
    }
    return typeof a == "number" && s.push({ eol: a, text: "", range: { startLineNumber: 0, startColumn: 0, endLineNumber: 0, endColumn: 0 } }), s;
  }
  computeHumanReadableDiff(t, n, i) {
    const r = this._getModel(t);
    if (!r)
      return n;
    const s = [];
    let a;
    n = n.slice(0).sort((u, h) => {
      if (u.range && h.range)
        return J.compareRangesUsingStarts(u.range, h.range);
      const c = u.range ? 0 : 1, m = h.range ? 0 : 1;
      return c - m;
    });
    for (let { range: u, text: h, eol: c } of n) {
      let x = function(v, L) {
        return new se(
          v.lineNumber + L.lineNumber - 1,
          L.lineNumber === 1 ? v.column + L.column - 1 : L.column
        );
      }, R = function(v, L) {
        const b = [];
        for (let E = L.startLineNumber; E <= L.endLineNumber; E++) {
          const k = v[E - 1];
          E === L.startLineNumber && E === L.endLineNumber ? b.push(k.substring(L.startColumn - 1, L.endColumn - 1)) : E === L.startLineNumber ? b.push(k.substring(L.startColumn - 1)) : E === L.endLineNumber ? b.push(k.substring(0, L.endColumn - 1)) : b.push(k);
        }
        return b;
      };
      var l = x, o = R;
      if (typeof c == "number" && (a = c), J.isEmpty(u) && !h)
        continue;
      const m = r.getValueInRange(u);
      if (h = h.replace(/\r\n|\n|\r/g, r.eol), m === h)
        continue;
      if (Math.max(h.length, m.length) > ht._diffLimit) {
        s.push({ range: u, text: h });
        continue;
      }
      const g = m.split(/\r\n|\n|\r/), d = h.split(/\r\n|\n|\r/), p = _i.getDefault().computeDiff(g, d, i), _ = J.lift(u).getStartPosition();
      for (const v of p.changes)
        if (v.innerChanges)
          for (const L of v.innerChanges)
            s.push({
              range: J.fromPositions(x(_, L.originalRange.getStartPosition()), x(_, L.originalRange.getEndPosition())),
              text: R(d, L.modifiedRange).join(r.eol)
            });
        else
          throw new ye("The experimental diff algorithm always produces inner changes");
    }
    return typeof a == "number" && s.push({ eol: a, text: "", range: { startLineNumber: 0, startColumn: 0, endLineNumber: 0, endColumn: 0 } }), s;
  }
  async computeLinks(t) {
    const n = this._getModel(t);
    return n ? Ac(n) : null;
  }
  async computeDefaultDocumentColors(t) {
    const n = this._getModel(t);
    return n ? p1(n) : null;
  }
  async textualSuggest(t, n, i, r) {
    const s = new fi(), a = new RegExp(i, r), l = /* @__PURE__ */ new Set();
    e: for (const o of t) {
      const u = this._getModel(o);
      if (u) {
        for (const h of u.words(a))
          if (!(h === n || !isNaN(Number(h))) && (l.add(h), l.size > ht._suggestionsLimit))
            break e;
      }
    }
    return { words: Array.from(l), duration: s.elapsed() };
  }
  async computeWordRanges(t, n, i, r) {
    const s = this._getModel(t);
    if (!s)
      return /* @__PURE__ */ Object.create(null);
    const a = new RegExp(i, r), l = /* @__PURE__ */ Object.create(null);
    for (let o = n.startLineNumber; o < n.endLineNumber; o++) {
      const u = s.getLineWords(o, a);
      for (const h of u) {
        if (!isNaN(Number(h.word)))
          continue;
        let c = l[h.word];
        c || (c = [], l[h.word] = c), c.push({
          startLineNumber: o,
          startColumn: h.startColumn,
          endLineNumber: o,
          endColumn: h.endColumn
        });
      }
    }
    return l;
  }
  async navigateValueSet(t, n, i, r, s) {
    const a = this._getModel(t);
    if (!a)
      return null;
    const l = new RegExp(r, s);
    n.startColumn === n.endColumn && (n = {
      startLineNumber: n.startLineNumber,
      startColumn: n.startColumn,
      endLineNumber: n.endLineNumber,
      endColumn: n.endColumn + 1
    });
    const o = a.getValueInRange(n), u = a.getWordAtPosition({ lineNumber: n.startLineNumber, column: n.startColumn }, l);
    if (!u)
      return null;
    const h = a.getValueInRange(u);
    return Fi.INSTANCE.navigateValueSet(n, o, u, h, i);
  }
  loadForeignModule(t, n, i) {
    const a = {
      host: Nu(i, (l, o) => this._host.fhr(l, o)),
      getMirrorModels: () => this._getModels()
    };
    return this._foreignModuleFactory ? (this._foreignModule = this._foreignModuleFactory(a, n), Promise.resolve(Ti(this._foreignModule))) : Promise.reject(new Error("Unexpected usage"));
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
ht._diffLimit = 1e5, ht._suggestionsLimit = 1e4;
let Xi = ht;
typeof importScripts == "function" && (globalThis.monaco = Fc());
let Ji = !1;
function El(e) {
  if (Ji)
    return;
  Ji = !0;
  const t = new Gu((n) => {
    globalThis.postMessage(n);
  }, (n) => new Xi(n, e));
  globalThis.onmessage = (n) => {
    t.onmessage(n.data);
  };
}
globalThis.onmessage = (e) => {
  Ji || El(null);
};
/*!-----------------------------------------------------------------------------
 * Copyright (c) Microsoft Corporation. All rights reserved.
 * Version: 0.50.0(undefined)
 * Released under the MIT license
 * https://github.com/microsoft/monaco-editor/blob/main/LICENSE.txt
 *-----------------------------------------------------------------------------*/
function xr(e, t = !1) {
  const n = e.length;
  let i = 0, r = "", s = 0, a = 16, l = 0, o = 0, u = 0, h = 0, c = 0;
  function m(v, L) {
    let b = 0, E = 0;
    for (; b < v || !L; ) {
      let k = e.charCodeAt(i);
      if (k >= 48 && k <= 57)
        E = E * 16 + k - 48;
      else if (k >= 65 && k <= 70)
        E = E * 16 + k - 65 + 10;
      else if (k >= 97 && k <= 102)
        E = E * 16 + k - 97 + 10;
      else
        break;
      i++, b++;
    }
    return b < v && (E = -1), E;
  }
  function g(v) {
    i = v, r = "", s = 0, a = 16, c = 0;
  }
  function d() {
    let v = i;
    if (e.charCodeAt(i) === 48)
      i++;
    else
      for (i++; i < e.length && It(e.charCodeAt(i)); )
        i++;
    if (i < e.length && e.charCodeAt(i) === 46)
      if (i++, i < e.length && It(e.charCodeAt(i)))
        for (i++; i < e.length && It(e.charCodeAt(i)); )
          i++;
      else
        return c = 3, e.substring(v, i);
    let L = i;
    if (i < e.length && (e.charCodeAt(i) === 69 || e.charCodeAt(i) === 101))
      if (i++, (i < e.length && e.charCodeAt(i) === 43 || e.charCodeAt(i) === 45) && i++, i < e.length && It(e.charCodeAt(i))) {
        for (i++; i < e.length && It(e.charCodeAt(i)); )
          i++;
        L = i;
      } else
        c = 3;
    return e.substring(v, L);
  }
  function p() {
    let v = "", L = i;
    for (; ; ) {
      if (i >= n) {
        v += e.substring(L, i), c = 2;
        break;
      }
      const b = e.charCodeAt(i);
      if (b === 34) {
        v += e.substring(L, i), i++;
        break;
      }
      if (b === 92) {
        if (v += e.substring(L, i), i++, i >= n) {
          c = 2;
          break;
        }
        switch (e.charCodeAt(i++)) {
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
            const k = m(4, !0);
            k >= 0 ? v += String.fromCharCode(k) : c = 4;
            break;
          default:
            c = 5;
        }
        L = i;
        continue;
      }
      if (b >= 0 && b <= 31)
        if (rn(b)) {
          v += e.substring(L, i), c = 2;
          break;
        } else
          c = 6;
      i++;
    }
    return v;
  }
  function _() {
    if (r = "", c = 0, s = i, o = l, h = u, i >= n)
      return s = n, a = 17;
    let v = e.charCodeAt(i);
    if (vi(v)) {
      do
        i++, r += String.fromCharCode(v), v = e.charCodeAt(i);
      while (vi(v));
      return a = 15;
    }
    if (rn(v))
      return i++, r += String.fromCharCode(v), v === 13 && e.charCodeAt(i) === 10 && (i++, r += `
`), l++, u = i, a = 14;
    switch (v) {
      case 123:
        return i++, a = 1;
      case 125:
        return i++, a = 2;
      case 91:
        return i++, a = 3;
      case 93:
        return i++, a = 4;
      case 58:
        return i++, a = 6;
      case 44:
        return i++, a = 5;
      case 34:
        return i++, r = p(), a = 10;
      case 47:
        const L = i - 1;
        if (e.charCodeAt(i + 1) === 47) {
          for (i += 2; i < n && !rn(e.charCodeAt(i)); )
            i++;
          return r = e.substring(L, i), a = 12;
        }
        if (e.charCodeAt(i + 1) === 42) {
          i += 2;
          const b = n - 1;
          let E = !1;
          for (; i < b; ) {
            const k = e.charCodeAt(i);
            if (k === 42 && e.charCodeAt(i + 1) === 47) {
              i += 2, E = !0;
              break;
            }
            i++, rn(k) && (k === 13 && e.charCodeAt(i) === 10 && i++, l++, u = i);
          }
          return E || (i++, c = 1), r = e.substring(L, i), a = 13;
        }
        return r += String.fromCharCode(v), i++, a = 16;
      case 45:
        if (r += String.fromCharCode(v), i++, i === n || !It(e.charCodeAt(i)))
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
        return r += d(), a = 11;
      default:
        for (; i < n && x(v); )
          i++, v = e.charCodeAt(i);
        if (s !== i) {
          switch (r = e.substring(s, i), r) {
            case "true":
              return a = 8;
            case "false":
              return a = 9;
            case "null":
              return a = 7;
          }
          return a = 16;
        }
        return r += String.fromCharCode(v), i++, a = 16;
    }
  }
  function x(v) {
    if (vi(v) || rn(v))
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
  function R() {
    let v;
    do
      v = _();
    while (v >= 12 && v <= 15);
    return v;
  }
  return {
    setPosition: g,
    getPosition: () => i,
    scan: t ? R : _,
    getToken: () => a,
    getTokenValue: () => r,
    getTokenOffset: () => s,
    getTokenLength: () => i - s,
    getTokenStartLine: () => o,
    getTokenStartCharacter: () => s - h,
    getTokenError: () => c
  };
}
function vi(e) {
  return e === 32 || e === 9;
}
function rn(e) {
  return e === 10 || e === 13;
}
function It(e) {
  return e >= 48 && e <= 57;
}
var Ha;
(function(e) {
  e[e.lineFeed = 10] = "lineFeed", e[e.carriageReturn = 13] = "carriageReturn", e[e.space = 32] = "space", e[e._0 = 48] = "_0", e[e._1 = 49] = "_1", e[e._2 = 50] = "_2", e[e._3 = 51] = "_3", e[e._4 = 52] = "_4", e[e._5 = 53] = "_5", e[e._6 = 54] = "_6", e[e._7 = 55] = "_7", e[e._8 = 56] = "_8", e[e._9 = 57] = "_9", e[e.a = 97] = "a", e[e.b = 98] = "b", e[e.c = 99] = "c", e[e.d = 100] = "d", e[e.e = 101] = "e", e[e.f = 102] = "f", e[e.g = 103] = "g", e[e.h = 104] = "h", e[e.i = 105] = "i", e[e.j = 106] = "j", e[e.k = 107] = "k", e[e.l = 108] = "l", e[e.m = 109] = "m", e[e.n = 110] = "n", e[e.o = 111] = "o", e[e.p = 112] = "p", e[e.q = 113] = "q", e[e.r = 114] = "r", e[e.s = 115] = "s", e[e.t = 116] = "t", e[e.u = 117] = "u", e[e.v = 118] = "v", e[e.w = 119] = "w", e[e.x = 120] = "x", e[e.y = 121] = "y", e[e.z = 122] = "z", e[e.A = 65] = "A", e[e.B = 66] = "B", e[e.C = 67] = "C", e[e.D = 68] = "D", e[e.E = 69] = "E", e[e.F = 70] = "F", e[e.G = 71] = "G", e[e.H = 72] = "H", e[e.I = 73] = "I", e[e.J = 74] = "J", e[e.K = 75] = "K", e[e.L = 76] = "L", e[e.M = 77] = "M", e[e.N = 78] = "N", e[e.O = 79] = "O", e[e.P = 80] = "P", e[e.Q = 81] = "Q", e[e.R = 82] = "R", e[e.S = 83] = "S", e[e.T = 84] = "T", e[e.U = 85] = "U", e[e.V = 86] = "V", e[e.W = 87] = "W", e[e.X = 88] = "X", e[e.Y = 89] = "Y", e[e.Z = 90] = "Z", e[e.asterisk = 42] = "asterisk", e[e.backslash = 92] = "backslash", e[e.closeBrace = 125] = "closeBrace", e[e.closeBracket = 93] = "closeBracket", e[e.colon = 58] = "colon", e[e.comma = 44] = "comma", e[e.dot = 46] = "dot", e[e.doubleQuote = 34] = "doubleQuote", e[e.minus = 45] = "minus", e[e.openBrace = 123] = "openBrace", e[e.openBracket = 91] = "openBracket", e[e.plus = 43] = "plus", e[e.slash = 47] = "slash", e[e.formFeed = 12] = "formFeed", e[e.tab = 9] = "tab";
})(Ha || (Ha = {}));
var Ie = new Array(20).fill(0).map((e, t) => " ".repeat(t)), Dt = 200, $a = {
  " ": {
    "\n": new Array(Dt).fill(0).map((e, t) => `
` + " ".repeat(t)),
    "\r": new Array(Dt).fill(0).map((e, t) => "\r" + " ".repeat(t)),
    "\r\n": new Array(Dt).fill(0).map((e, t) => `\r
` + " ".repeat(t))
  },
  "	": {
    "\n": new Array(Dt).fill(0).map((e, t) => `
` + "	".repeat(t)),
    "\r": new Array(Dt).fill(0).map((e, t) => "\r" + "	".repeat(t)),
    "\r\n": new Array(Dt).fill(0).map((e, t) => `\r
` + "	".repeat(t))
  }
}, A1 = [`
`, "\r", `\r
`];
function x1(e, t, n) {
  let i, r, s, a, l;
  if (t) {
    for (a = t.offset, l = a + t.length, s = a; s > 0 && !Wa(e, s - 1); )
      s--;
    let b = l;
    for (; b < e.length && !Wa(e, b); )
      b++;
    r = e.substring(s, b), i = E1(r, n);
  } else
    r = e, i = 0, s = 0, a = 0, l = e.length;
  const o = y1(n, e), u = A1.includes(o);
  let h = 0, c = 0, m;
  n.insertSpaces ? m = Ie[n.tabSize || 4] ?? Ut(Ie[1], n.tabSize || 4) : m = "	";
  const g = m === "	" ? "	" : " ";
  let d = xr(r, !1), p = !1;
  function _() {
    if (h > 1)
      return Ut(o, h) + Ut(m, i + c);
    const b = m.length * (i + c);
    return !u || b > $a[g][o].length ? o + Ut(m, i + c) : b <= 0 ? o : $a[g][o][b];
  }
  function x() {
    let b = d.scan();
    for (h = 0; b === 15 || b === 14; )
      b === 14 && n.keepLines ? h += 1 : b === 14 && (h = 1), b = d.scan();
    return p = b === 16 || d.getTokenError() !== 0, b;
  }
  const R = [];
  function v(b, E, k) {
    !p && (!t || E < l && k > a) && e.substring(E, k) !== b && R.push({ offset: E, length: k - E, content: b });
  }
  let L = x();
  if (n.keepLines && h > 0 && v(Ut(o, h), 0, 0), L !== 17) {
    let b = d.getTokenOffset() + s, E = m.length * i < 20 && n.insertSpaces ? Ie[m.length * i] : Ut(m, i);
    v(E, s, b);
  }
  for (; L !== 17; ) {
    let b = d.getTokenOffset() + d.getTokenLength() + s, E = x(), k = "", D = !1;
    for (; h === 0 && (E === 12 || E === 13); ) {
      let W = d.getTokenOffset() + s;
      v(Ie[1], b, W), b = d.getTokenOffset() + d.getTokenLength() + s, D = E === 12, k = D ? _() : "", E = x();
    }
    if (E === 2)
      L !== 1 && c--, n.keepLines && h > 0 || !n.keepLines && L !== 1 ? k = _() : n.keepLines && (k = Ie[1]);
    else if (E === 4)
      L !== 3 && c--, n.keepLines && h > 0 || !n.keepLines && L !== 3 ? k = _() : n.keepLines && (k = Ie[1]);
    else {
      switch (L) {
        case 3:
        case 1:
          c++, n.keepLines && h > 0 || !n.keepLines ? k = _() : k = Ie[1];
          break;
        case 5:
          n.keepLines && h > 0 || !n.keepLines ? k = _() : k = Ie[1];
          break;
        case 12:
          k = _();
          break;
        case 13:
          h > 0 ? k = _() : D || (k = Ie[1]);
          break;
        case 6:
          n.keepLines && h > 0 ? k = _() : D || (k = Ie[1]);
          break;
        case 10:
          n.keepLines && h > 0 ? k = _() : E === 6 && !D && (k = "");
          break;
        case 7:
        case 8:
        case 9:
        case 11:
        case 2:
        case 4:
          n.keepLines && h > 0 ? k = _() : (E === 12 || E === 13) && !D ? k = Ie[1] : E !== 5 && E !== 17 && (p = !0);
          break;
        case 16:
          p = !0;
          break;
      }
      h > 0 && (E === 12 || E === 13) && (k = _());
    }
    E === 17 && (n.keepLines && h > 0 ? k = _() : k = n.insertFinalNewline ? o : "");
    const O = d.getTokenOffset() + s;
    v(k, b, O), L = E;
  }
  return R;
}
function Ut(e, t) {
  let n = "";
  for (let i = 0; i < t; i++)
    n += e;
  return n;
}
function E1(e, t) {
  let n = 0, i = 0;
  const r = t.tabSize || 4;
  for (; n < e.length; ) {
    let s = e.charAt(n);
    if (s === Ie[1])
      i++;
    else if (s === "	")
      i += r;
    else
      break;
    n++;
  }
  return Math.floor(i / r);
}
function y1(e, t) {
  for (let n = 0; n < t.length; n++) {
    const i = t.charAt(n);
    if (i === "\r")
      return n + 1 < t.length && t.charAt(n + 1) === `
` ? `\r
` : "\r";
    if (i === `
`)
      return `
`;
  }
  return e && e.eol || `
`;
}
function Wa(e, t) {
  return `\r
`.indexOf(e.charAt(t)) !== -1;
}
var Kn;
(function(e) {
  e.DEFAULT = {
    allowTrailingComma: !1
  };
})(Kn || (Kn = {}));
function R1(e, t = [], n = Kn.DEFAULT) {
  let i = null, r = [];
  const s = [];
  function a(o) {
    Array.isArray(r) ? r.push(o) : i !== null && (r[i] = o);
  }
  return T1(e, {
    onObjectBegin: () => {
      const o = {};
      a(o), s.push(r), r = o, i = null;
    },
    onObjectProperty: (o) => {
      i = o;
    },
    onObjectEnd: () => {
      r = s.pop();
    },
    onArrayBegin: () => {
      const o = [];
      a(o), s.push(r), r = o, i = null;
    },
    onArrayEnd: () => {
      r = s.pop();
    },
    onLiteralValue: a,
    onError: (o, u, h) => {
      t.push({ error: o, offset: u, length: h });
    }
  }, n), r[0];
}
function yl(e) {
  if (!e.parent || !e.parent.children)
    return [];
  const t = yl(e.parent);
  if (e.parent.type === "property") {
    const n = e.parent.children[0].value;
    t.push(n);
  } else if (e.parent.type === "array") {
    const n = e.parent.children.indexOf(e);
    n !== -1 && t.push(n);
  }
  return t;
}
function Yi(e) {
  switch (e.type) {
    case "array":
      return e.children.map(Yi);
    case "object":
      const t = /* @__PURE__ */ Object.create(null);
      for (let n of e.children) {
        const i = n.children[1];
        i && (t[n.children[0].value] = Yi(i));
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
function k1(e, t, n = !1) {
  return t >= e.offset && t < e.offset + e.length || n && t === e.offset + e.length;
}
function Rl(e, t, n = !1) {
  if (k1(e, t, n)) {
    const i = e.children;
    if (Array.isArray(i))
      for (let r = 0; r < i.length && i[r].offset <= t; r++) {
        const s = Rl(i[r], t, n);
        if (s)
          return s;
      }
    return e;
  }
}
function T1(e, t, n = Kn.DEFAULT) {
  const i = xr(e, !1), r = [];
  function s(A) {
    return A ? () => A(i.getTokenOffset(), i.getTokenLength(), i.getTokenStartLine(), i.getTokenStartCharacter()) : () => !0;
  }
  function a(A) {
    return A ? () => A(i.getTokenOffset(), i.getTokenLength(), i.getTokenStartLine(), i.getTokenStartCharacter(), () => r.slice()) : () => !0;
  }
  function l(A) {
    return A ? (y) => A(y, i.getTokenOffset(), i.getTokenLength(), i.getTokenStartLine(), i.getTokenStartCharacter()) : () => !0;
  }
  function o(A) {
    return A ? (y) => A(y, i.getTokenOffset(), i.getTokenLength(), i.getTokenStartLine(), i.getTokenStartCharacter(), () => r.slice()) : () => !0;
  }
  const u = a(t.onObjectBegin), h = o(t.onObjectProperty), c = s(t.onObjectEnd), m = a(t.onArrayBegin), g = s(t.onArrayEnd), d = o(t.onLiteralValue), p = l(t.onSeparator), _ = s(t.onComment), x = l(t.onError), R = n && n.disallowComments, v = n && n.allowTrailingComma;
  function L() {
    for (; ; ) {
      const A = i.scan();
      switch (i.getTokenError()) {
        case 4:
          b(
            14
            /* ParseErrorCode.InvalidUnicode */
          );
          break;
        case 5:
          b(
            15
            /* ParseErrorCode.InvalidEscapeCharacter */
          );
          break;
        case 3:
          b(
            13
            /* ParseErrorCode.UnexpectedEndOfNumber */
          );
          break;
        case 1:
          R || b(
            11
            /* ParseErrorCode.UnexpectedEndOfComment */
          );
          break;
        case 2:
          b(
            12
            /* ParseErrorCode.UnexpectedEndOfString */
          );
          break;
        case 6:
          b(
            16
            /* ParseErrorCode.InvalidCharacter */
          );
          break;
      }
      switch (A) {
        case 12:
        case 13:
          R ? b(
            10
            /* ParseErrorCode.InvalidCommentToken */
          ) : _();
          break;
        case 16:
          b(
            1
            /* ParseErrorCode.InvalidSymbol */
          );
          break;
        case 15:
        case 14:
          break;
        default:
          return A;
      }
    }
  }
  function b(A, y = [], I = []) {
    if (x(A), y.length + I.length > 0) {
      let F = i.getToken();
      for (; F !== 17; ) {
        if (y.indexOf(F) !== -1) {
          L();
          break;
        } else if (I.indexOf(F) !== -1)
          break;
        F = L();
      }
    }
  }
  function E(A) {
    const y = i.getTokenValue();
    return A ? d(y) : (h(y), r.push(y)), L(), !0;
  }
  function k() {
    switch (i.getToken()) {
      case 11:
        const A = i.getTokenValue();
        let y = Number(A);
        isNaN(y) && (b(
          2
          /* ParseErrorCode.InvalidNumberFormat */
        ), y = 0), d(y);
        break;
      case 7:
        d(null);
        break;
      case 8:
        d(!0);
        break;
      case 9:
        d(!1);
        break;
      default:
        return !1;
    }
    return L(), !0;
  }
  function D() {
    return i.getToken() !== 10 ? (b(3, [], [
      2,
      5
      /* SyntaxKind.CommaToken */
    ]), !1) : (E(!1), i.getToken() === 6 ? (p(":"), L(), U() || b(4, [], [
      2,
      5
      /* SyntaxKind.CommaToken */
    ])) : b(5, [], [
      2,
      5
      /* SyntaxKind.CommaToken */
    ]), r.pop(), !0);
  }
  function O() {
    u(), L();
    let A = !1;
    for (; i.getToken() !== 2 && i.getToken() !== 17; ) {
      if (i.getToken() === 5) {
        if (A || b(4, [], []), p(","), L(), i.getToken() === 2 && v)
          break;
      } else A && b(6, [], []);
      D() || b(4, [], [
        2,
        5
        /* SyntaxKind.CommaToken */
      ]), A = !0;
    }
    return c(), i.getToken() !== 2 ? b(7, [
      2
      /* SyntaxKind.CloseBraceToken */
    ], []) : L(), !0;
  }
  function W() {
    m(), L();
    let A = !0, y = !1;
    for (; i.getToken() !== 4 && i.getToken() !== 17; ) {
      if (i.getToken() === 5) {
        if (y || b(4, [], []), p(","), L(), i.getToken() === 4 && v)
          break;
      } else y && b(6, [], []);
      A ? (r.push(0), A = !1) : r[r.length - 1]++, U() || b(4, [], [
        4,
        5
        /* SyntaxKind.CommaToken */
      ]), y = !0;
    }
    return g(), A || r.pop(), i.getToken() !== 4 ? b(8, [
      4
      /* SyntaxKind.CloseBracketToken */
    ], []) : L(), !0;
  }
  function U() {
    switch (i.getToken()) {
      case 3:
        return W();
      case 1:
        return O();
      case 10:
        return E(!0);
      default:
        return k();
    }
  }
  return L(), i.getToken() === 17 ? n.allowEmptyContent ? !0 : (b(4, [], []), !1) : U() ? (i.getToken() !== 17 && b(9, [], []), !0) : (b(4, [], []), !1);
}
var At = xr, ja;
(function(e) {
  e[e.None = 0] = "None", e[e.UnexpectedEndOfComment = 1] = "UnexpectedEndOfComment", e[e.UnexpectedEndOfString = 2] = "UnexpectedEndOfString", e[e.UnexpectedEndOfNumber = 3] = "UnexpectedEndOfNumber", e[e.InvalidUnicode = 4] = "InvalidUnicode", e[e.InvalidEscapeCharacter = 5] = "InvalidEscapeCharacter", e[e.InvalidCharacter = 6] = "InvalidCharacter";
})(ja || (ja = {}));
var Ga;
(function(e) {
  e[e.OpenBraceToken = 1] = "OpenBraceToken", e[e.CloseBraceToken = 2] = "CloseBraceToken", e[e.OpenBracketToken = 3] = "OpenBracketToken", e[e.CloseBracketToken = 4] = "CloseBracketToken", e[e.CommaToken = 5] = "CommaToken", e[e.ColonToken = 6] = "ColonToken", e[e.NullKeyword = 7] = "NullKeyword", e[e.TrueKeyword = 8] = "TrueKeyword", e[e.FalseKeyword = 9] = "FalseKeyword", e[e.StringLiteral = 10] = "StringLiteral", e[e.NumericLiteral = 11] = "NumericLiteral", e[e.LineCommentTrivia = 12] = "LineCommentTrivia", e[e.BlockCommentTrivia = 13] = "BlockCommentTrivia", e[e.LineBreakTrivia = 14] = "LineBreakTrivia", e[e.Trivia = 15] = "Trivia", e[e.Unknown = 16] = "Unknown", e[e.EOF = 17] = "EOF";
})(Ga || (Ga = {}));
var S1 = R1, M1 = Rl, I1 = yl, D1 = Yi, za;
(function(e) {
  e[e.InvalidSymbol = 1] = "InvalidSymbol", e[e.InvalidNumberFormat = 2] = "InvalidNumberFormat", e[e.PropertyNameExpected = 3] = "PropertyNameExpected", e[e.ValueExpected = 4] = "ValueExpected", e[e.ColonExpected = 5] = "ColonExpected", e[e.CommaExpected = 6] = "CommaExpected", e[e.CloseBraceExpected = 7] = "CloseBraceExpected", e[e.CloseBracketExpected = 8] = "CloseBracketExpected", e[e.EndOfFileExpected = 9] = "EndOfFileExpected", e[e.InvalidCommentToken = 10] = "InvalidCommentToken", e[e.UnexpectedEndOfComment = 11] = "UnexpectedEndOfComment", e[e.UnexpectedEndOfString = 12] = "UnexpectedEndOfString", e[e.UnexpectedEndOfNumber = 13] = "UnexpectedEndOfNumber", e[e.InvalidUnicode = 14] = "InvalidUnicode", e[e.InvalidEscapeCharacter = 15] = "InvalidEscapeCharacter", e[e.InvalidCharacter = 16] = "InvalidCharacter";
})(za || (za = {}));
function U1(e, t, n) {
  return x1(e, t, n);
}
function jt(e, t) {
  if (e === t)
    return !0;
  if (e == null || t === null || t === void 0 || typeof e != typeof t || typeof e != "object" || Array.isArray(e) !== Array.isArray(t))
    return !1;
  let n, i;
  if (Array.isArray(e)) {
    if (e.length !== t.length)
      return !1;
    for (n = 0; n < e.length; n++)
      if (!jt(e[n], t[n]))
        return !1;
  } else {
    const r = [];
    for (i in e)
      r.push(i);
    r.sort();
    const s = [];
    for (i in t)
      s.push(i);
    if (s.sort(), !jt(r, s))
      return !1;
    for (n = 0; n < r.length; n++)
      if (!jt(e[r[n]], t[r[n]]))
        return !1;
  }
  return !0;
}
function ve(e) {
  return typeof e == "number";
}
function qe(e) {
  return typeof e < "u";
}
function Ye(e) {
  return typeof e == "boolean";
}
function kl(e) {
  return typeof e == "string";
}
function ft(e) {
  return typeof e == "object" && e !== null && !Array.isArray(e);
}
function F1(e, t) {
  if (e.length < t.length)
    return !1;
  for (let n = 0; n < t.length; n++)
    if (e[n] !== t[n])
      return !1;
  return !0;
}
function wn(e, t) {
  const n = e.length - t.length;
  return n > 0 ? e.lastIndexOf(t) === n : n === 0 ? e === t : !1;
}
function Cn(e) {
  let t = "";
  F1(e, "(?i)") && (e = e.substring(4), t = "i");
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
function Xa(e) {
  let t = 0;
  for (let n = 0; n < e.length; n++) {
    t++;
    const i = e.charCodeAt(n);
    55296 <= i && i <= 56319 && n++;
  }
  return t;
}
var Ja;
(function(e) {
  function t(n) {
    return typeof n == "string";
  }
  e.is = t;
})(Ja || (Ja = {}));
var Qi;
(function(e) {
  function t(n) {
    return typeof n == "string";
  }
  e.is = t;
})(Qi || (Qi = {}));
var Ya;
(function(e) {
  e.MIN_VALUE = -2147483648, e.MAX_VALUE = 2147483647;
  function t(n) {
    return typeof n == "number" && e.MIN_VALUE <= n && n <= e.MAX_VALUE;
  }
  e.is = t;
})(Ya || (Ya = {}));
var ei;
(function(e) {
  e.MIN_VALUE = 0, e.MAX_VALUE = 2147483647;
  function t(n) {
    return typeof n == "number" && e.MIN_VALUE <= n && n <= e.MAX_VALUE;
  }
  e.is = t;
})(ei || (ei = {}));
var oe;
(function(e) {
  function t(i, r) {
    return i === Number.MAX_VALUE && (i = ei.MAX_VALUE), r === Number.MAX_VALUE && (r = ei.MAX_VALUE), { line: i, character: r };
  }
  e.create = t;
  function n(i) {
    let r = i;
    return T.objectLiteral(r) && T.uinteger(r.line) && T.uinteger(r.character);
  }
  e.is = n;
})(oe || (oe = {}));
var X;
(function(e) {
  function t(i, r, s, a) {
    if (T.uinteger(i) && T.uinteger(r) && T.uinteger(s) && T.uinteger(a))
      return { start: oe.create(i, r), end: oe.create(s, a) };
    if (oe.is(i) && oe.is(r))
      return { start: i, end: r };
    throw new Error(`Range#create called with invalid arguments[${i}, ${r}, ${s}, ${a}]`);
  }
  e.create = t;
  function n(i) {
    let r = i;
    return T.objectLiteral(r) && oe.is(r.start) && oe.is(r.end);
  }
  e.is = n;
})(X || (X = {}));
var Yt;
(function(e) {
  function t(i, r) {
    return { uri: i, range: r };
  }
  e.create = t;
  function n(i) {
    let r = i;
    return T.objectLiteral(r) && X.is(r.range) && (T.string(r.uri) || T.undefined(r.uri));
  }
  e.is = n;
})(Yt || (Yt = {}));
var Qa;
(function(e) {
  function t(i, r, s, a) {
    return { targetUri: i, targetRange: r, targetSelectionRange: s, originSelectionRange: a };
  }
  e.create = t;
  function n(i) {
    let r = i;
    return T.objectLiteral(r) && X.is(r.targetRange) && T.string(r.targetUri) && X.is(r.targetSelectionRange) && (X.is(r.originSelectionRange) || T.undefined(r.originSelectionRange));
  }
  e.is = n;
})(Qa || (Qa = {}));
var Zi;
(function(e) {
  function t(i, r, s, a) {
    return {
      red: i,
      green: r,
      blue: s,
      alpha: a
    };
  }
  e.create = t;
  function n(i) {
    const r = i;
    return T.objectLiteral(r) && T.numberRange(r.red, 0, 1) && T.numberRange(r.green, 0, 1) && T.numberRange(r.blue, 0, 1) && T.numberRange(r.alpha, 0, 1);
  }
  e.is = n;
})(Zi || (Zi = {}));
var Za;
(function(e) {
  function t(i, r) {
    return {
      range: i,
      color: r
    };
  }
  e.create = t;
  function n(i) {
    const r = i;
    return T.objectLiteral(r) && X.is(r.range) && Zi.is(r.color);
  }
  e.is = n;
})(Za || (Za = {}));
var Ka;
(function(e) {
  function t(i, r, s) {
    return {
      label: i,
      textEdit: r,
      additionalTextEdits: s
    };
  }
  e.create = t;
  function n(i) {
    const r = i;
    return T.objectLiteral(r) && T.string(r.label) && (T.undefined(r.textEdit) || Ze.is(r)) && (T.undefined(r.additionalTextEdits) || T.typedArray(r.additionalTextEdits, Ze.is));
  }
  e.is = n;
})(Ka || (Ka = {}));
var mn;
(function(e) {
  e.Comment = "comment", e.Imports = "imports", e.Region = "region";
})(mn || (mn = {}));
var Ca;
(function(e) {
  function t(i, r, s, a, l, o) {
    const u = {
      startLine: i,
      endLine: r
    };
    return T.defined(s) && (u.startCharacter = s), T.defined(a) && (u.endCharacter = a), T.defined(l) && (u.kind = l), T.defined(o) && (u.collapsedText = o), u;
  }
  e.create = t;
  function n(i) {
    const r = i;
    return T.objectLiteral(r) && T.uinteger(r.startLine) && T.uinteger(r.startLine) && (T.undefined(r.startCharacter) || T.uinteger(r.startCharacter)) && (T.undefined(r.endCharacter) || T.uinteger(r.endCharacter)) && (T.undefined(r.kind) || T.string(r.kind));
  }
  e.is = n;
})(Ca || (Ca = {}));
var Ki;
(function(e) {
  function t(i, r) {
    return {
      location: i,
      message: r
    };
  }
  e.create = t;
  function n(i) {
    let r = i;
    return T.defined(r) && Yt.is(r.location) && T.string(r.message);
  }
  e.is = n;
})(Ki || (Ki = {}));
var Me;
(function(e) {
  e.Error = 1, e.Warning = 2, e.Information = 3, e.Hint = 4;
})(Me || (Me = {}));
var eo;
(function(e) {
  e.Unnecessary = 1, e.Deprecated = 2;
})(eo || (eo = {}));
var to;
(function(e) {
  function t(n) {
    const i = n;
    return T.objectLiteral(i) && T.string(i.href);
  }
  e.is = t;
})(to || (to = {}));
var nt;
(function(e) {
  function t(i, r, s, a, l, o) {
    let u = { range: i, message: r };
    return T.defined(s) && (u.severity = s), T.defined(a) && (u.code = a), T.defined(l) && (u.source = l), T.defined(o) && (u.relatedInformation = o), u;
  }
  e.create = t;
  function n(i) {
    var r;
    let s = i;
    return T.defined(s) && X.is(s.range) && T.string(s.message) && (T.number(s.severity) || T.undefined(s.severity)) && (T.integer(s.code) || T.string(s.code) || T.undefined(s.code)) && (T.undefined(s.codeDescription) || T.string((r = s.codeDescription) === null || r === void 0 ? void 0 : r.href)) && (T.string(s.source) || T.undefined(s.source)) && (T.undefined(s.relatedInformation) || T.typedArray(s.relatedInformation, Ki.is));
  }
  e.is = n;
})(nt || (nt = {}));
var Qt;
(function(e) {
  function t(i, r, ...s) {
    let a = { title: i, command: r };
    return T.defined(s) && s.length > 0 && (a.arguments = s), a;
  }
  e.create = t;
  function n(i) {
    let r = i;
    return T.defined(r) && T.string(r.title) && T.string(r.command);
  }
  e.is = n;
})(Qt || (Qt = {}));
var Ze;
(function(e) {
  function t(s, a) {
    return { range: s, newText: a };
  }
  e.replace = t;
  function n(s, a) {
    return { range: { start: s, end: s }, newText: a };
  }
  e.insert = n;
  function i(s) {
    return { range: s, newText: "" };
  }
  e.del = i;
  function r(s) {
    const a = s;
    return T.objectLiteral(a) && T.string(a.newText) && X.is(a.range);
  }
  e.is = r;
})(Ze || (Ze = {}));
var Ci;
(function(e) {
  function t(i, r, s) {
    const a = { label: i };
    return r !== void 0 && (a.needsConfirmation = r), s !== void 0 && (a.description = s), a;
  }
  e.create = t;
  function n(i) {
    const r = i;
    return T.objectLiteral(r) && T.string(r.label) && (T.boolean(r.needsConfirmation) || r.needsConfirmation === void 0) && (T.string(r.description) || r.description === void 0);
  }
  e.is = n;
})(Ci || (Ci = {}));
var Zt;
(function(e) {
  function t(n) {
    const i = n;
    return T.string(i);
  }
  e.is = t;
})(Zt || (Zt = {}));
var no;
(function(e) {
  function t(s, a, l) {
    return { range: s, newText: a, annotationId: l };
  }
  e.replace = t;
  function n(s, a, l) {
    return { range: { start: s, end: s }, newText: a, annotationId: l };
  }
  e.insert = n;
  function i(s, a) {
    return { range: s, newText: "", annotationId: a };
  }
  e.del = i;
  function r(s) {
    const a = s;
    return Ze.is(a) && (Ci.is(a.annotationId) || Zt.is(a.annotationId));
  }
  e.is = r;
})(no || (no = {}));
var er;
(function(e) {
  function t(i, r) {
    return { textDocument: i, edits: r };
  }
  e.create = t;
  function n(i) {
    let r = i;
    return T.defined(r) && sr.is(r.textDocument) && Array.isArray(r.edits);
  }
  e.is = n;
})(er || (er = {}));
var tr;
(function(e) {
  function t(i, r, s) {
    let a = {
      kind: "create",
      uri: i
    };
    return r !== void 0 && (r.overwrite !== void 0 || r.ignoreIfExists !== void 0) && (a.options = r), s !== void 0 && (a.annotationId = s), a;
  }
  e.create = t;
  function n(i) {
    let r = i;
    return r && r.kind === "create" && T.string(r.uri) && (r.options === void 0 || (r.options.overwrite === void 0 || T.boolean(r.options.overwrite)) && (r.options.ignoreIfExists === void 0 || T.boolean(r.options.ignoreIfExists))) && (r.annotationId === void 0 || Zt.is(r.annotationId));
  }
  e.is = n;
})(tr || (tr = {}));
var nr;
(function(e) {
  function t(i, r, s, a) {
    let l = {
      kind: "rename",
      oldUri: i,
      newUri: r
    };
    return s !== void 0 && (s.overwrite !== void 0 || s.ignoreIfExists !== void 0) && (l.options = s), a !== void 0 && (l.annotationId = a), l;
  }
  e.create = t;
  function n(i) {
    let r = i;
    return r && r.kind === "rename" && T.string(r.oldUri) && T.string(r.newUri) && (r.options === void 0 || (r.options.overwrite === void 0 || T.boolean(r.options.overwrite)) && (r.options.ignoreIfExists === void 0 || T.boolean(r.options.ignoreIfExists))) && (r.annotationId === void 0 || Zt.is(r.annotationId));
  }
  e.is = n;
})(nr || (nr = {}));
var ir;
(function(e) {
  function t(i, r, s) {
    let a = {
      kind: "delete",
      uri: i
    };
    return r !== void 0 && (r.recursive !== void 0 || r.ignoreIfNotExists !== void 0) && (a.options = r), s !== void 0 && (a.annotationId = s), a;
  }
  e.create = t;
  function n(i) {
    let r = i;
    return r && r.kind === "delete" && T.string(r.uri) && (r.options === void 0 || (r.options.recursive === void 0 || T.boolean(r.options.recursive)) && (r.options.ignoreIfNotExists === void 0 || T.boolean(r.options.ignoreIfNotExists))) && (r.annotationId === void 0 || Zt.is(r.annotationId));
  }
  e.is = n;
})(ir || (ir = {}));
var rr;
(function(e) {
  function t(n) {
    let i = n;
    return i && (i.changes !== void 0 || i.documentChanges !== void 0) && (i.documentChanges === void 0 || i.documentChanges.every((r) => T.string(r.kind) ? tr.is(r) || nr.is(r) || ir.is(r) : er.is(r)));
  }
  e.is = t;
})(rr || (rr = {}));
var io;
(function(e) {
  function t(i) {
    return { uri: i };
  }
  e.create = t;
  function n(i) {
    let r = i;
    return T.defined(r) && T.string(r.uri);
  }
  e.is = n;
})(io || (io = {}));
var ro;
(function(e) {
  function t(i, r) {
    return { uri: i, version: r };
  }
  e.create = t;
  function n(i) {
    let r = i;
    return T.defined(r) && T.string(r.uri) && T.integer(r.version);
  }
  e.is = n;
})(ro || (ro = {}));
var sr;
(function(e) {
  function t(i, r) {
    return { uri: i, version: r };
  }
  e.create = t;
  function n(i) {
    let r = i;
    return T.defined(r) && T.string(r.uri) && (r.version === null || T.integer(r.version));
  }
  e.is = n;
})(sr || (sr = {}));
var so;
(function(e) {
  function t(i, r, s, a) {
    return { uri: i, languageId: r, version: s, text: a };
  }
  e.create = t;
  function n(i) {
    let r = i;
    return T.defined(r) && T.string(r.uri) && T.string(r.languageId) && T.integer(r.version) && T.string(r.text);
  }
  e.is = n;
})(so || (so = {}));
var Et;
(function(e) {
  e.PlainText = "plaintext", e.Markdown = "markdown";
  function t(n) {
    const i = n;
    return i === e.PlainText || i === e.Markdown;
  }
  e.is = t;
})(Et || (Et = {}));
var An;
(function(e) {
  function t(n) {
    const i = n;
    return T.objectLiteral(n) && Et.is(i.kind) && T.string(i.value);
  }
  e.is = t;
})(An || (An = {}));
var Te;
(function(e) {
  e.Text = 1, e.Method = 2, e.Function = 3, e.Constructor = 4, e.Field = 5, e.Variable = 6, e.Class = 7, e.Interface = 8, e.Module = 9, e.Property = 10, e.Unit = 11, e.Value = 12, e.Enum = 13, e.Keyword = 14, e.Snippet = 15, e.Color = 16, e.File = 17, e.Reference = 18, e.Folder = 19, e.EnumMember = 20, e.Constant = 21, e.Struct = 22, e.Event = 23, e.Operator = 24, e.TypeParameter = 25;
})(Te || (Te = {}));
var he;
(function(e) {
  e.PlainText = 1, e.Snippet = 2;
})(he || (he = {}));
var ao;
(function(e) {
  e.Deprecated = 1;
})(ao || (ao = {}));
var oo;
(function(e) {
  function t(i, r, s) {
    return { newText: i, insert: r, replace: s };
  }
  e.create = t;
  function n(i) {
    const r = i;
    return r && T.string(r.newText) && X.is(r.insert) && X.is(r.replace);
  }
  e.is = n;
})(oo || (oo = {}));
var lo;
(function(e) {
  e.asIs = 1, e.adjustIndentation = 2;
})(lo || (lo = {}));
var uo;
(function(e) {
  function t(n) {
    const i = n;
    return i && (T.string(i.detail) || i.detail === void 0) && (T.string(i.description) || i.description === void 0);
  }
  e.is = t;
})(uo || (uo = {}));
var ar;
(function(e) {
  function t(n) {
    return { label: n };
  }
  e.create = t;
})(ar || (ar = {}));
var co;
(function(e) {
  function t(n, i) {
    return { items: n || [], isIncomplete: !!i };
  }
  e.create = t;
})(co || (co = {}));
var ti;
(function(e) {
  function t(i) {
    return i.replace(/[\\`*_{}[\]()#+\-.!]/g, "\\$&");
  }
  e.fromPlainText = t;
  function n(i) {
    const r = i;
    return T.string(r) || T.objectLiteral(r) && T.string(r.language) && T.string(r.value);
  }
  e.is = n;
})(ti || (ti = {}));
var fo;
(function(e) {
  function t(n) {
    let i = n;
    return !!i && T.objectLiteral(i) && (An.is(i.contents) || ti.is(i.contents) || T.typedArray(i.contents, ti.is)) && (n.range === void 0 || X.is(n.range));
  }
  e.is = t;
})(fo || (fo = {}));
var ho;
(function(e) {
  function t(n, i) {
    return i ? { label: n, documentation: i } : { label: n };
  }
  e.create = t;
})(ho || (ho = {}));
var mo;
(function(e) {
  function t(n, i, ...r) {
    let s = { label: n };
    return T.defined(i) && (s.documentation = i), T.defined(r) ? s.parameters = r : s.parameters = [], s;
  }
  e.create = t;
})(mo || (mo = {}));
var go;
(function(e) {
  e.Text = 1, e.Read = 2, e.Write = 3;
})(go || (go = {}));
var po;
(function(e) {
  function t(n, i) {
    let r = { range: n };
    return T.number(i) && (r.kind = i), r;
  }
  e.create = t;
})(po || (po = {}));
var ze;
(function(e) {
  e.File = 1, e.Module = 2, e.Namespace = 3, e.Package = 4, e.Class = 5, e.Method = 6, e.Property = 7, e.Field = 8, e.Constructor = 9, e.Enum = 10, e.Interface = 11, e.Function = 12, e.Variable = 13, e.Constant = 14, e.String = 15, e.Number = 16, e.Boolean = 17, e.Array = 18, e.Object = 19, e.Key = 20, e.Null = 21, e.EnumMember = 22, e.Struct = 23, e.Event = 24, e.Operator = 25, e.TypeParameter = 26;
})(ze || (ze = {}));
var bo;
(function(e) {
  e.Deprecated = 1;
})(bo || (bo = {}));
var _o;
(function(e) {
  function t(n, i, r, s, a) {
    let l = {
      name: n,
      kind: i,
      location: { uri: s, range: r }
    };
    return a && (l.containerName = a), l;
  }
  e.create = t;
})(_o || (_o = {}));
var vo;
(function(e) {
  function t(n, i, r, s) {
    return s !== void 0 ? { name: n, kind: i, location: { uri: r, range: s } } : { name: n, kind: i, location: { uri: r } };
  }
  e.create = t;
})(vo || (vo = {}));
var Lo;
(function(e) {
  function t(i, r, s, a, l, o) {
    let u = {
      name: i,
      detail: r,
      kind: s,
      range: a,
      selectionRange: l
    };
    return o !== void 0 && (u.children = o), u;
  }
  e.create = t;
  function n(i) {
    let r = i;
    return r && T.string(r.name) && T.number(r.kind) && X.is(r.range) && X.is(r.selectionRange) && (r.detail === void 0 || T.string(r.detail)) && (r.deprecated === void 0 || T.boolean(r.deprecated)) && (r.children === void 0 || Array.isArray(r.children)) && (r.tags === void 0 || Array.isArray(r.tags));
  }
  e.is = n;
})(Lo || (Lo = {}));
var No;
(function(e) {
  e.Empty = "", e.QuickFix = "quickfix", e.Refactor = "refactor", e.RefactorExtract = "refactor.extract", e.RefactorInline = "refactor.inline", e.RefactorRewrite = "refactor.rewrite", e.Source = "source", e.SourceOrganizeImports = "source.organizeImports", e.SourceFixAll = "source.fixAll";
})(No || (No = {}));
var ni;
(function(e) {
  e.Invoked = 1, e.Automatic = 2;
})(ni || (ni = {}));
var wo;
(function(e) {
  function t(i, r, s) {
    let a = { diagnostics: i };
    return r != null && (a.only = r), s != null && (a.triggerKind = s), a;
  }
  e.create = t;
  function n(i) {
    let r = i;
    return T.defined(r) && T.typedArray(r.diagnostics, nt.is) && (r.only === void 0 || T.typedArray(r.only, T.string)) && (r.triggerKind === void 0 || r.triggerKind === ni.Invoked || r.triggerKind === ni.Automatic);
  }
  e.is = n;
})(wo || (wo = {}));
var Ao;
(function(e) {
  function t(i, r, s) {
    let a = { title: i }, l = !0;
    return typeof r == "string" ? (l = !1, a.kind = r) : Qt.is(r) ? a.command = r : a.edit = r, l && s !== void 0 && (a.kind = s), a;
  }
  e.create = t;
  function n(i) {
    let r = i;
    return r && T.string(r.title) && (r.diagnostics === void 0 || T.typedArray(r.diagnostics, nt.is)) && (r.kind === void 0 || T.string(r.kind)) && (r.edit !== void 0 || r.command !== void 0) && (r.command === void 0 || Qt.is(r.command)) && (r.isPreferred === void 0 || T.boolean(r.isPreferred)) && (r.edit === void 0 || rr.is(r.edit));
  }
  e.is = n;
})(Ao || (Ao = {}));
var xo;
(function(e) {
  function t(i, r) {
    let s = { range: i };
    return T.defined(r) && (s.data = r), s;
  }
  e.create = t;
  function n(i) {
    let r = i;
    return T.defined(r) && X.is(r.range) && (T.undefined(r.command) || Qt.is(r.command));
  }
  e.is = n;
})(xo || (xo = {}));
var Eo;
(function(e) {
  function t(i, r) {
    return { tabSize: i, insertSpaces: r };
  }
  e.create = t;
  function n(i) {
    let r = i;
    return T.defined(r) && T.uinteger(r.tabSize) && T.boolean(r.insertSpaces);
  }
  e.is = n;
})(Eo || (Eo = {}));
var yo;
(function(e) {
  function t(i, r, s) {
    return { range: i, target: r, data: s };
  }
  e.create = t;
  function n(i) {
    let r = i;
    return T.defined(r) && X.is(r.range) && (T.undefined(r.target) || T.string(r.target));
  }
  e.is = n;
})(yo || (yo = {}));
var ii;
(function(e) {
  function t(i, r) {
    return { range: i, parent: r };
  }
  e.create = t;
  function n(i) {
    let r = i;
    return T.objectLiteral(r) && X.is(r.range) && (r.parent === void 0 || e.is(r.parent));
  }
  e.is = n;
})(ii || (ii = {}));
var Ro;
(function(e) {
  e.namespace = "namespace", e.type = "type", e.class = "class", e.enum = "enum", e.interface = "interface", e.struct = "struct", e.typeParameter = "typeParameter", e.parameter = "parameter", e.variable = "variable", e.property = "property", e.enumMember = "enumMember", e.event = "event", e.function = "function", e.method = "method", e.macro = "macro", e.keyword = "keyword", e.modifier = "modifier", e.comment = "comment", e.string = "string", e.number = "number", e.regexp = "regexp", e.operator = "operator", e.decorator = "decorator";
})(Ro || (Ro = {}));
var ko;
(function(e) {
  e.declaration = "declaration", e.definition = "definition", e.readonly = "readonly", e.static = "static", e.deprecated = "deprecated", e.abstract = "abstract", e.async = "async", e.modification = "modification", e.documentation = "documentation", e.defaultLibrary = "defaultLibrary";
})(ko || (ko = {}));
var To;
(function(e) {
  function t(n) {
    const i = n;
    return T.objectLiteral(i) && (i.resultId === void 0 || typeof i.resultId == "string") && Array.isArray(i.data) && (i.data.length === 0 || typeof i.data[0] == "number");
  }
  e.is = t;
})(To || (To = {}));
var So;
(function(e) {
  function t(i, r) {
    return { range: i, text: r };
  }
  e.create = t;
  function n(i) {
    const r = i;
    return r != null && X.is(r.range) && T.string(r.text);
  }
  e.is = n;
})(So || (So = {}));
var Mo;
(function(e) {
  function t(i, r, s) {
    return { range: i, variableName: r, caseSensitiveLookup: s };
  }
  e.create = t;
  function n(i) {
    const r = i;
    return r != null && X.is(r.range) && T.boolean(r.caseSensitiveLookup) && (T.string(r.variableName) || r.variableName === void 0);
  }
  e.is = n;
})(Mo || (Mo = {}));
var Io;
(function(e) {
  function t(i, r) {
    return { range: i, expression: r };
  }
  e.create = t;
  function n(i) {
    const r = i;
    return r != null && X.is(r.range) && (T.string(r.expression) || r.expression === void 0);
  }
  e.is = n;
})(Io || (Io = {}));
var Do;
(function(e) {
  function t(i, r) {
    return { frameId: i, stoppedLocation: r };
  }
  e.create = t;
  function n(i) {
    const r = i;
    return T.defined(r) && X.is(i.stoppedLocation);
  }
  e.is = n;
})(Do || (Do = {}));
var or;
(function(e) {
  e.Type = 1, e.Parameter = 2;
  function t(n) {
    return n === 1 || n === 2;
  }
  e.is = t;
})(or || (or = {}));
var lr;
(function(e) {
  function t(i) {
    return { value: i };
  }
  e.create = t;
  function n(i) {
    const r = i;
    return T.objectLiteral(r) && (r.tooltip === void 0 || T.string(r.tooltip) || An.is(r.tooltip)) && (r.location === void 0 || Yt.is(r.location)) && (r.command === void 0 || Qt.is(r.command));
  }
  e.is = n;
})(lr || (lr = {}));
var Uo;
(function(e) {
  function t(i, r, s) {
    const a = { position: i, label: r };
    return s !== void 0 && (a.kind = s), a;
  }
  e.create = t;
  function n(i) {
    const r = i;
    return T.objectLiteral(r) && oe.is(r.position) && (T.string(r.label) || T.typedArray(r.label, lr.is)) && (r.kind === void 0 || or.is(r.kind)) && r.textEdits === void 0 || T.typedArray(r.textEdits, Ze.is) && (r.tooltip === void 0 || T.string(r.tooltip) || An.is(r.tooltip)) && (r.paddingLeft === void 0 || T.boolean(r.paddingLeft)) && (r.paddingRight === void 0 || T.boolean(r.paddingRight));
  }
  e.is = n;
})(Uo || (Uo = {}));
var Fo;
(function(e) {
  function t(n) {
    return { kind: "snippet", value: n };
  }
  e.createSnippet = t;
})(Fo || (Fo = {}));
var Po;
(function(e) {
  function t(n, i, r, s) {
    return { insertText: n, filterText: i, range: r, command: s };
  }
  e.create = t;
})(Po || (Po = {}));
var Oo;
(function(e) {
  function t(n) {
    return { items: n };
  }
  e.create = t;
})(Oo || (Oo = {}));
var Bo;
(function(e) {
  e.Invoked = 0, e.Automatic = 1;
})(Bo || (Bo = {}));
var Vo;
(function(e) {
  function t(n, i) {
    return { range: n, text: i };
  }
  e.create = t;
})(Vo || (Vo = {}));
var qo;
(function(e) {
  function t(n, i) {
    return { triggerKind: n, selectedCompletionInfo: i };
  }
  e.create = t;
})(qo || (qo = {}));
var Ho;
(function(e) {
  function t(n) {
    const i = n;
    return T.objectLiteral(i) && Qi.is(i.uri) && T.string(i.name);
  }
  e.is = t;
})(Ho || (Ho = {}));
var $o;
(function(e) {
  function t(s, a, l, o) {
    return new P1(s, a, l, o);
  }
  e.create = t;
  function n(s) {
    let a = s;
    return !!(T.defined(a) && T.string(a.uri) && (T.undefined(a.languageId) || T.string(a.languageId)) && T.uinteger(a.lineCount) && T.func(a.getText) && T.func(a.positionAt) && T.func(a.offsetAt));
  }
  e.is = n;
  function i(s, a) {
    let l = s.getText(), o = r(a, (h, c) => {
      let m = h.range.start.line - c.range.start.line;
      return m === 0 ? h.range.start.character - c.range.start.character : m;
    }), u = l.length;
    for (let h = o.length - 1; h >= 0; h--) {
      let c = o[h], m = s.offsetAt(c.range.start), g = s.offsetAt(c.range.end);
      if (g <= u)
        l = l.substring(0, m) + c.newText + l.substring(g, l.length);
      else
        throw new Error("Overlapping edit");
      u = m;
    }
    return l;
  }
  e.applyEdits = i;
  function r(s, a) {
    if (s.length <= 1)
      return s;
    const l = s.length / 2 | 0, o = s.slice(0, l), u = s.slice(l);
    r(o, a), r(u, a);
    let h = 0, c = 0, m = 0;
    for (; h < o.length && c < u.length; )
      a(o[h], u[c]) <= 0 ? s[m++] = o[h++] : s[m++] = u[c++];
    for (; h < o.length; )
      s[m++] = o[h++];
    for (; c < u.length; )
      s[m++] = u[c++];
    return s;
  }
})($o || ($o = {}));
var P1 = class {
  constructor(e, t, n, i) {
    this._uri = e, this._languageId = t, this._version = n, this._content = i, this._lineOffsets = void 0;
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
      let t = this.offsetAt(e.start), n = this.offsetAt(e.end);
      return this._content.substring(t, n);
    }
    return this._content;
  }
  update(e, t) {
    this._content = e.text, this._version = t, this._lineOffsets = void 0;
  }
  getLineOffsets() {
    if (this._lineOffsets === void 0) {
      let e = [], t = this._content, n = !0;
      for (let i = 0; i < t.length; i++) {
        n && (e.push(i), n = !1);
        let r = t.charAt(i);
        n = r === "\r" || r === `
`, r === "\r" && i + 1 < t.length && t.charAt(i + 1) === `
` && i++;
      }
      n && t.length > 0 && e.push(t.length), this._lineOffsets = e;
    }
    return this._lineOffsets;
  }
  positionAt(e) {
    e = Math.max(Math.min(e, this._content.length), 0);
    let t = this.getLineOffsets(), n = 0, i = t.length;
    if (i === 0)
      return oe.create(0, e);
    for (; n < i; ) {
      let s = Math.floor((n + i) / 2);
      t[s] > e ? i = s : n = s + 1;
    }
    let r = n - 1;
    return oe.create(r, e - t[r]);
  }
  offsetAt(e) {
    let t = this.getLineOffsets();
    if (e.line >= t.length)
      return this._content.length;
    if (e.line < 0)
      return 0;
    let n = t[e.line], i = e.line + 1 < t.length ? t[e.line + 1] : this._content.length;
    return Math.max(Math.min(n + e.character, i), n);
  }
  get lineCount() {
    return this.getLineOffsets().length;
  }
}, T;
(function(e) {
  const t = Object.prototype.toString;
  function n(g) {
    return typeof g < "u";
  }
  e.defined = n;
  function i(g) {
    return typeof g > "u";
  }
  e.undefined = i;
  function r(g) {
    return g === !0 || g === !1;
  }
  e.boolean = r;
  function s(g) {
    return t.call(g) === "[object String]";
  }
  e.string = s;
  function a(g) {
    return t.call(g) === "[object Number]";
  }
  e.number = a;
  function l(g, d, p) {
    return t.call(g) === "[object Number]" && d <= g && g <= p;
  }
  e.numberRange = l;
  function o(g) {
    return t.call(g) === "[object Number]" && -2147483648 <= g && g <= 2147483647;
  }
  e.integer = o;
  function u(g) {
    return t.call(g) === "[object Number]" && 0 <= g && g <= 2147483647;
  }
  e.uinteger = u;
  function h(g) {
    return t.call(g) === "[object Function]";
  }
  e.func = h;
  function c(g) {
    return g !== null && typeof g == "object";
  }
  e.objectLiteral = c;
  function m(g, d) {
    return Array.isArray(g) && g.every(d);
  }
  e.typedArray = m;
})(T || (T = {}));
var Wo = class ur {
  constructor(t, n, i, r) {
    this._uri = t, this._languageId = n, this._version = i, this._content = r, this._lineOffsets = void 0;
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
  getText(t) {
    if (t) {
      const n = this.offsetAt(t.start), i = this.offsetAt(t.end);
      return this._content.substring(n, i);
    }
    return this._content;
  }
  update(t, n) {
    for (let i of t)
      if (ur.isIncremental(i)) {
        const r = Tl(i.range), s = this.offsetAt(r.start), a = this.offsetAt(r.end);
        this._content = this._content.substring(0, s) + i.text + this._content.substring(a, this._content.length);
        const l = Math.max(r.start.line, 0), o = Math.max(r.end.line, 0);
        let u = this._lineOffsets;
        const h = jo(i.text, !1, s);
        if (o - l === h.length)
          for (let m = 0, g = h.length; m < g; m++)
            u[m + l + 1] = h[m];
        else
          h.length < 1e4 ? u.splice(l + 1, o - l, ...h) : this._lineOffsets = u = u.slice(0, l + 1).concat(h, u.slice(o + 1));
        const c = i.text.length - (a - s);
        if (c !== 0)
          for (let m = l + 1 + h.length, g = u.length; m < g; m++)
            u[m] = u[m] + c;
      } else if (ur.isFull(i))
        this._content = i.text, this._lineOffsets = void 0;
      else
        throw new Error("Unknown change event received");
    this._version = n;
  }
  getLineOffsets() {
    return this._lineOffsets === void 0 && (this._lineOffsets = jo(this._content, !0)), this._lineOffsets;
  }
  positionAt(t) {
    t = Math.max(Math.min(t, this._content.length), 0);
    let n = this.getLineOffsets(), i = 0, r = n.length;
    if (r === 0)
      return { line: 0, character: t };
    for (; i < r; ) {
      let a = Math.floor((i + r) / 2);
      n[a] > t ? r = a : i = a + 1;
    }
    let s = i - 1;
    return { line: s, character: t - n[s] };
  }
  offsetAt(t) {
    let n = this.getLineOffsets();
    if (t.line >= n.length)
      return this._content.length;
    if (t.line < 0)
      return 0;
    let i = n[t.line], r = t.line + 1 < n.length ? n[t.line + 1] : this._content.length;
    return Math.max(Math.min(i + t.character, r), i);
  }
  get lineCount() {
    return this.getLineOffsets().length;
  }
  static isIncremental(t) {
    let n = t;
    return n != null && typeof n.text == "string" && n.range !== void 0 && (n.rangeLength === void 0 || typeof n.rangeLength == "number");
  }
  static isFull(t) {
    let n = t;
    return n != null && typeof n.text == "string" && n.range === void 0 && n.rangeLength === void 0;
  }
}, Ge;
(function(e) {
  function t(r, s, a, l) {
    return new Wo(r, s, a, l);
  }
  e.create = t;
  function n(r, s, a) {
    if (r instanceof Wo)
      return r.update(s, a), r;
    throw new Error("TextDocument.update: document must be created by TextDocument.create");
  }
  e.update = n;
  function i(r, s) {
    let a = r.getText(), l = cr(s.map(O1), (h, c) => {
      let m = h.range.start.line - c.range.start.line;
      return m === 0 ? h.range.start.character - c.range.start.character : m;
    }), o = 0;
    const u = [];
    for (const h of l) {
      let c = r.offsetAt(h.range.start);
      if (c < o)
        throw new Error("Overlapping edit");
      c > o && u.push(a.substring(o, c)), h.newText.length && u.push(h.newText), o = r.offsetAt(h.range.end);
    }
    return u.push(a.substr(o)), u.join("");
  }
  e.applyEdits = i;
})(Ge || (Ge = {}));
function cr(e, t) {
  if (e.length <= 1)
    return e;
  const n = e.length / 2 | 0, i = e.slice(0, n), r = e.slice(n);
  cr(i, t), cr(r, t);
  let s = 0, a = 0, l = 0;
  for (; s < i.length && a < r.length; )
    t(i[s], r[a]) <= 0 ? e[l++] = i[s++] : e[l++] = r[a++];
  for (; s < i.length; )
    e[l++] = i[s++];
  for (; a < r.length; )
    e[l++] = r[a++];
  return e;
}
function jo(e, t, n = 0) {
  const i = t ? [n] : [];
  for (let r = 0; r < e.length; r++) {
    let s = e.charCodeAt(r);
    (s === 13 || s === 10) && (s === 13 && r + 1 < e.length && e.charCodeAt(r + 1) === 10 && r++, i.push(n + r + 1));
  }
  return i;
}
function Tl(e) {
  const t = e.start, n = e.end;
  return t.line > n.line || t.line === n.line && t.character > n.character ? { start: n, end: t } : e;
}
function O1(e) {
  const t = Tl(e.range);
  return t !== e.range ? { newText: e.newText, range: t } : e;
}
var K;
(function(e) {
  e[e.Undefined = 0] = "Undefined", e[e.EnumValueMismatch = 1] = "EnumValueMismatch", e[e.Deprecated = 2] = "Deprecated", e[e.UnexpectedEndOfComment = 257] = "UnexpectedEndOfComment", e[e.UnexpectedEndOfString = 258] = "UnexpectedEndOfString", e[e.UnexpectedEndOfNumber = 259] = "UnexpectedEndOfNumber", e[e.InvalidUnicode = 260] = "InvalidUnicode", e[e.InvalidEscapeCharacter = 261] = "InvalidEscapeCharacter", e[e.InvalidCharacter = 262] = "InvalidCharacter", e[e.PropertyExpected = 513] = "PropertyExpected", e[e.CommaExpected = 514] = "CommaExpected", e[e.ColonExpected = 515] = "ColonExpected", e[e.ValueExpected = 516] = "ValueExpected", e[e.CommaOrCloseBacketExpected = 517] = "CommaOrCloseBacketExpected", e[e.CommaOrCloseBraceExpected = 518] = "CommaOrCloseBraceExpected", e[e.TrailingComma = 519] = "TrailingComma", e[e.DuplicateKey = 520] = "DuplicateKey", e[e.CommentNotPermitted = 521] = "CommentNotPermitted", e[e.PropertyKeysMustBeDoublequoted = 528] = "PropertyKeysMustBeDoublequoted", e[e.SchemaResolveError = 768] = "SchemaResolveError", e[e.SchemaUnsupportedFeature = 769] = "SchemaUnsupportedFeature";
})(K || (K = {}));
var je;
(function(e) {
  e[e.v3 = 3] = "v3", e[e.v4 = 4] = "v4", e[e.v6 = 6] = "v6", e[e.v7 = 7] = "v7", e[e.v2019_09 = 19] = "v2019_09", e[e.v2020_12 = 20] = "v2020_12";
})(je || (je = {}));
var fr;
(function(e) {
  e.LATEST = {
    textDocument: {
      completion: {
        completionItem: {
          documentationFormat: [Et.Markdown, Et.PlainText],
          commitCharactersSupport: !0,
          labelDetailsSupport: !0
        }
      }
    }
  };
})(fr || (fr = {}));
function M(...e) {
  const t = e[0];
  let n, i, r;
  if (typeof t == "string")
    n = t, i = t, e.splice(0, 1), r = !e || typeof e[0] != "object" ? e : e[0];
  else if (t instanceof Array) {
    const s = e.slice(1);
    if (t.length !== s.length + 1)
      throw new Error("expected a string as the first argument to l10n.t");
    let a = t[0];
    for (let l = 1; l < t.length; l++)
      a += `{${l - 1}}` + t[l];
    return M(a, ...s);
  } else
    i = t.message, n = i, t.comment && t.comment.length > 0 && (n += `/${Array.isArray(t.comment) ? t.comment.join("") : t.comment}`), r = t.args ?? {};
  return V1(i, r);
}
var B1 = /{([^}]+)}/g;
function V1(e, t) {
  return Object.keys(t).length === 0 ? e : e.replace(B1, (n, i) => t[i] ?? n);
}
var q1 = {
  "color-hex": { errorMessage: M("Invalid color format. Use #RGB, #RGBA, #RRGGBB or #RRGGBBAA."), pattern: /^#([0-9A-Fa-f]{3,4}|([0-9A-Fa-f]{2}){3,4})$/ },
  "date-time": { errorMessage: M("String is not a RFC3339 date-time."), pattern: /^(\d{4})-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])T([01][0-9]|2[0-3]):([0-5][0-9]):([0-5][0-9]|60)(\.[0-9]+)?(Z|(\+|-)([01][0-9]|2[0-3]):([0-5][0-9]))$/i },
  date: { errorMessage: M("String is not a RFC3339 date."), pattern: /^(\d{4})-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])$/i },
  time: { errorMessage: M("String is not a RFC3339 time."), pattern: /^([01][0-9]|2[0-3]):([0-5][0-9]):([0-5][0-9]|60)(\.[0-9]+)?(Z|(\+|-)([01][0-9]|2[0-3]):([0-5][0-9]))$/i },
  email: { errorMessage: M("String is not an e-mail address."), pattern: /^(([^<>()\[\]\\.,;:\s@"]+(\.[^<>()\[\]\\.,;:\s@"]+)*)|(".+"))@((\[[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}])|(([a-zA-Z0-9-]+\.)+[a-zA-Z]{2,}))$/ },
  hostname: { errorMessage: M("String is not a hostname."), pattern: /^(?=.{1,253}\.?$)[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[-0-9a-z]{0,61}[0-9a-z])?)*\.?$/i },
  ipv4: { errorMessage: M("String is not an IPv4 address."), pattern: /^(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)$/ },
  ipv6: { errorMessage: M("String is not an IPv6 address."), pattern: /^((([0-9a-f]{1,4}:){7}([0-9a-f]{1,4}|:))|(([0-9a-f]{1,4}:){6}(:[0-9a-f]{1,4}|((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3})|:))|(([0-9a-f]{1,4}:){5}(((:[0-9a-f]{1,4}){1,2})|:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3})|:))|(([0-9a-f]{1,4}:){4}(((:[0-9a-f]{1,4}){1,3})|((:[0-9a-f]{1,4})?:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9a-f]{1,4}:){3}(((:[0-9a-f]{1,4}){1,4})|((:[0-9a-f]{1,4}){0,2}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9a-f]{1,4}:){2}(((:[0-9a-f]{1,4}){1,5})|((:[0-9a-f]{1,4}){0,3}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9a-f]{1,4}:){1}(((:[0-9a-f]{1,4}){1,6})|((:[0-9a-f]{1,4}){0,4}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(:(((:[0-9a-f]{1,4}){1,7})|((:[0-9a-f]{1,4}){0,5}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:)))$/i }
}, yt = class {
  constructor(e, t, n = 0) {
    this.offset = t, this.length = n, this.parent = e;
  }
  get children() {
    return [];
  }
  toString() {
    return "type: " + this.type + " (" + this.offset + "/" + this.length + ")" + (this.parent ? " parent: {" + this.parent.toString() + "}" : "");
  }
}, H1 = class extends yt {
  constructor(e, t) {
    super(e, t), this.type = "null", this.value = null;
  }
}, Go = class extends yt {
  constructor(e, t, n) {
    super(e, n), this.type = "boolean", this.value = t;
  }
}, $1 = class extends yt {
  constructor(e, t) {
    super(e, t), this.type = "array", this.items = [];
  }
  get children() {
    return this.items;
  }
}, W1 = class extends yt {
  constructor(e, t) {
    super(e, t), this.type = "number", this.isInteger = !0, this.value = Number.NaN;
  }
}, Li = class extends yt {
  constructor(e, t, n) {
    super(e, t, n), this.type = "string", this.value = "";
  }
}, j1 = class extends yt {
  constructor(e, t, n) {
    super(e, t), this.type = "property", this.colonOffset = -1, this.keyNode = n;
  }
  get children() {
    return this.valueNode ? [this.keyNode, this.valueNode] : [this.keyNode];
  }
}, G1 = class extends yt {
  constructor(e, t) {
    super(e, t), this.type = "object", this.properties = [];
  }
  get children() {
    return this.properties;
  }
};
function Re(e) {
  return Ye(e) ? e ? {} : { not: {} } : e;
}
var zo;
(function(e) {
  e[e.Key = 0] = "Key", e[e.Enum = 1] = "Enum";
})(zo || (zo = {}));
var z1 = {
  "http://json-schema.org/draft-03/schema#": je.v3,
  "http://json-schema.org/draft-04/schema#": je.v4,
  "http://json-schema.org/draft-06/schema#": je.v6,
  "http://json-schema.org/draft-07/schema#": je.v7,
  "https://json-schema.org/draft/2019-09/schema": je.v2019_09,
  "https://json-schema.org/draft/2020-12/schema": je.v2020_12
}, Xo = class {
  constructor(e) {
    this.schemaDraft = e;
  }
}, X1 = class Sl {
  constructor(t = -1, n) {
    this.focusOffset = t, this.exclude = n, this.schemas = [];
  }
  add(t) {
    this.schemas.push(t);
  }
  merge(t) {
    Array.prototype.push.apply(this.schemas, t.schemas);
  }
  include(t) {
    return (this.focusOffset === -1 || Ml(t, this.focusOffset)) && t !== this.exclude;
  }
  newSub() {
    return new Sl(-1, this.exclude);
  }
}, xn = class {
  constructor() {
  }
  get schemas() {
    return [];
  }
  add(e) {
  }
  merge(e) {
  }
  include(e) {
    return !0;
  }
  newSub() {
    return this;
  }
};
xn.instance = new xn();
var Le = class {
  constructor() {
    this.problems = [], this.propertiesMatches = 0, this.processedProperties = /* @__PURE__ */ new Set(), this.propertiesValueMatches = 0, this.primaryValueMatches = 0, this.enumValueMatch = !1, this.enumValues = void 0;
  }
  hasProblems() {
    return !!this.problems.length;
  }
  merge(e) {
    this.problems = this.problems.concat(e.problems), this.propertiesMatches += e.propertiesMatches, this.propertiesValueMatches += e.propertiesValueMatches, this.mergeProcessedProperties(e);
  }
  mergeEnumValues(e) {
    if (!this.enumValueMatch && !e.enumValueMatch && this.enumValues && e.enumValues) {
      this.enumValues = this.enumValues.concat(e.enumValues);
      for (const t of this.problems)
        t.code === K.EnumValueMismatch && (t.message = M("Value is not accepted. Valid values: {0}.", this.enumValues.map((n) => JSON.stringify(n)).join(", ")));
    }
  }
  mergePropertyMatch(e) {
    this.problems = this.problems.concat(e.problems), this.propertiesMatches++, (e.enumValueMatch || !e.hasProblems() && e.propertiesMatches) && this.propertiesValueMatches++, e.enumValueMatch && e.enumValues && e.enumValues.length === 1 && this.primaryValueMatches++;
  }
  mergeProcessedProperties(e) {
    e.processedProperties.forEach((t) => this.processedProperties.add(t));
  }
  compare(e) {
    const t = this.hasProblems();
    return t !== e.hasProblems() ? t ? -1 : 1 : this.enumValueMatch !== e.enumValueMatch ? e.enumValueMatch ? -1 : 1 : this.primaryValueMatches !== e.primaryValueMatches ? this.primaryValueMatches - e.primaryValueMatches : this.propertiesValueMatches !== e.propertiesValueMatches ? this.propertiesValueMatches - e.propertiesValueMatches : this.propertiesMatches - e.propertiesMatches;
  }
};
function J1(e, t = []) {
  return new Il(e, t, []);
}
function xt(e) {
  return D1(e);
}
function hr(e) {
  return I1(e);
}
function Ml(e, t, n = !1) {
  return t >= e.offset && t < e.offset + e.length || n && t === e.offset + e.length;
}
var Il = class {
  constructor(e, t = [], n = []) {
    this.root = e, this.syntaxErrors = t, this.comments = n;
  }
  getNodeFromOffset(e, t = !1) {
    if (this.root)
      return M1(this.root, e, t);
  }
  visit(e) {
    if (this.root) {
      const t = (n) => {
        let i = e(n);
        const r = n.children;
        if (Array.isArray(r))
          for (let s = 0; s < r.length && i; s++)
            i = t(r[s]);
        return i;
      };
      t(this.root);
    }
  }
  validate(e, t, n = Me.Warning, i) {
    if (this.root && t) {
      const r = new Le();
      return de(this.root, t, r, xn.instance, new Xo(i ?? Jo(t))), r.problems.map((s) => {
        const a = X.create(e.positionAt(s.location.offset), e.positionAt(s.location.offset + s.location.length));
        return nt.create(a, s.message, s.severity ?? n, s.code);
      });
    }
  }
  getMatchingSchemas(e, t = -1, n) {
    if (this.root && e) {
      const i = new X1(t, n), r = Jo(e), s = new Xo(r);
      return de(this.root, e, new Le(), i, s), i.schemas;
    }
    return [];
  }
};
function Jo(e, t = je.v2020_12) {
  let n = e.$schema;
  return n ? z1[n] ?? t : t;
}
function de(e, t, n, i, r) {
  if (!e || !i.include(e))
    return;
  if (e.type === "property")
    return de(e.valueNode, t, n, i, r);
  const s = e;
  switch (a(), s.type) {
    case "object":
      h(s);
      break;
    case "array":
      u(s);
      break;
    case "string":
      o(s);
      break;
    case "number":
      l(s);
      break;
  }
  i.add({ node: s, schema: t });
  function a() {
    var R;
    function c(v) {
      return s.type === v || v === "integer" && s.type === "number" && s.isInteger;
    }
    if (Array.isArray(t.type) ? t.type.some(c) || n.problems.push({
      location: { offset: s.offset, length: s.length },
      message: t.errorMessage || M("Incorrect type. Expected one of {0}.", t.type.join(", "))
    }) : t.type && (c(t.type) || n.problems.push({
      location: { offset: s.offset, length: s.length },
      message: t.errorMessage || M('Incorrect type. Expected "{0}".', t.type)
    })), Array.isArray(t.allOf))
      for (const v of t.allOf) {
        const L = new Le(), b = i.newSub();
        de(s, Re(v), L, b, r), n.merge(L), i.merge(b);
      }
    const m = Re(t.not);
    if (m) {
      const v = new Le(), L = i.newSub();
      de(s, m, v, L, r), v.hasProblems() || n.problems.push({
        location: { offset: s.offset, length: s.length },
        message: t.errorMessage || M("Matches a schema that is not allowed.")
      });
      for (const b of L.schemas)
        b.inverted = !b.inverted, i.add(b);
    }
    const g = (v, L) => {
      const b = [];
      let E;
      for (const k of v) {
        const D = Re(k), O = new Le(), W = i.newSub();
        if (de(s, D, O, W, r), O.hasProblems() || b.push(D), !E)
          E = { schema: D, validationResult: O, matchingSchemas: W };
        else if (!L && !O.hasProblems() && !E.validationResult.hasProblems())
          E.matchingSchemas.merge(W), E.validationResult.propertiesMatches += O.propertiesMatches, E.validationResult.propertiesValueMatches += O.propertiesValueMatches, E.validationResult.mergeProcessedProperties(O);
        else {
          const U = O.compare(E.validationResult);
          U > 0 ? E = { schema: D, validationResult: O, matchingSchemas: W } : U === 0 && (E.matchingSchemas.merge(W), E.validationResult.mergeEnumValues(O));
        }
      }
      return b.length > 1 && L && n.problems.push({
        location: { offset: s.offset, length: 1 },
        message: M("Matches multiple schemas when only one must validate.")
      }), E && (n.merge(E.validationResult), i.merge(E.matchingSchemas)), b.length;
    };
    Array.isArray(t.anyOf) && g(t.anyOf, !1), Array.isArray(t.oneOf) && g(t.oneOf, !0);
    const d = (v) => {
      const L = new Le(), b = i.newSub();
      de(s, Re(v), L, b, r), n.merge(L), i.merge(b);
    }, p = (v, L, b) => {
      const E = Re(v), k = new Le(), D = i.newSub();
      de(s, E, k, D, r), i.merge(D), n.mergeProcessedProperties(k), k.hasProblems() ? b && d(b) : L && d(L);
    }, _ = Re(t.if);
    if (_ && p(_, Re(t.then), Re(t.else)), Array.isArray(t.enum)) {
      const v = xt(s);
      let L = !1;
      for (const b of t.enum)
        if (jt(v, b)) {
          L = !0;
          break;
        }
      n.enumValues = t.enum, n.enumValueMatch = L, L || n.problems.push({
        location: { offset: s.offset, length: s.length },
        code: K.EnumValueMismatch,
        message: t.errorMessage || M("Value is not accepted. Valid values: {0}.", t.enum.map((b) => JSON.stringify(b)).join(", "))
      });
    }
    if (qe(t.const)) {
      const v = xt(s);
      jt(v, t.const) ? n.enumValueMatch = !0 : (n.problems.push({
        location: { offset: s.offset, length: s.length },
        code: K.EnumValueMismatch,
        message: t.errorMessage || M("Value must be {0}.", JSON.stringify(t.const))
      }), n.enumValueMatch = !1), n.enumValues = [t.const];
    }
    let x = t.deprecationMessage;
    if (x || t.deprecated) {
      x = x || M("Value is deprecated");
      let v = ((R = s.parent) == null ? void 0 : R.type) === "property" ? s.parent : s;
      n.problems.push({
        location: { offset: v.offset, length: v.length },
        severity: Me.Warning,
        message: x,
        code: K.Deprecated
      });
    }
  }
  function l(c) {
    const m = c.value;
    function g(L) {
      var E;
      const b = /^(-?\d+)(?:\.(\d+))?(?:e([-+]\d+))?$/.exec(L.toString());
      return b && {
        value: Number(b[1] + (b[2] || "")),
        multiplier: (((E = b[2]) == null ? void 0 : E.length) || 0) - (parseInt(b[3]) || 0)
      };
    }
    if (ve(t.multipleOf)) {
      let L = -1;
      if (Number.isInteger(t.multipleOf))
        L = m % t.multipleOf;
      else {
        let b = g(t.multipleOf), E = g(m);
        if (b && E) {
          const k = 10 ** Math.abs(E.multiplier - b.multiplier);
          E.multiplier < b.multiplier ? E.value *= k : b.value *= k, L = E.value % b.value;
        }
      }
      L !== 0 && n.problems.push({
        location: { offset: c.offset, length: c.length },
        message: M("Value is not divisible by {0}.", t.multipleOf)
      });
    }
    function d(L, b) {
      if (ve(b))
        return b;
      if (Ye(b) && b)
        return L;
    }
    function p(L, b) {
      if (!Ye(b) || !b)
        return L;
    }
    const _ = d(t.minimum, t.exclusiveMinimum);
    ve(_) && m <= _ && n.problems.push({
      location: { offset: c.offset, length: c.length },
      message: M("Value is below the exclusive minimum of {0}.", _)
    });
    const x = d(t.maximum, t.exclusiveMaximum);
    ve(x) && m >= x && n.problems.push({
      location: { offset: c.offset, length: c.length },
      message: M("Value is above the exclusive maximum of {0}.", x)
    });
    const R = p(t.minimum, t.exclusiveMinimum);
    ve(R) && m < R && n.problems.push({
      location: { offset: c.offset, length: c.length },
      message: M("Value is below the minimum of {0}.", R)
    });
    const v = p(t.maximum, t.exclusiveMaximum);
    ve(v) && m > v && n.problems.push({
      location: { offset: c.offset, length: c.length },
      message: M("Value is above the maximum of {0}.", v)
    });
  }
  function o(c) {
    if (ve(t.minLength) && Xa(c.value) < t.minLength && n.problems.push({
      location: { offset: c.offset, length: c.length },
      message: M("String is shorter than the minimum length of {0}.", t.minLength)
    }), ve(t.maxLength) && Xa(c.value) > t.maxLength && n.problems.push({
      location: { offset: c.offset, length: c.length },
      message: M("String is longer than the maximum length of {0}.", t.maxLength)
    }), kl(t.pattern)) {
      const m = Cn(t.pattern);
      m != null && m.test(c.value) || n.problems.push({
        location: { offset: c.offset, length: c.length },
        message: t.patternErrorMessage || t.errorMessage || M('String does not match the pattern of "{0}".', t.pattern)
      });
    }
    if (t.format)
      switch (t.format) {
        case "uri":
        case "uri-reference":
          {
            let g;
            if (!c.value)
              g = M("URI expected.");
            else {
              const d = /^(([^:/?#]+?):)?(\/\/([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?/.exec(c.value);
              d ? !d[2] && t.format === "uri" && (g = M("URI with a scheme is expected.")) : g = M("URI is expected.");
            }
            g && n.problems.push({
              location: { offset: c.offset, length: c.length },
              message: t.patternErrorMessage || t.errorMessage || M("String is not a URI: {0}", g)
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
          const m = q1[t.format];
          (!c.value || !m.pattern.exec(c.value)) && n.problems.push({
            location: { offset: c.offset, length: c.length },
            message: t.patternErrorMessage || t.errorMessage || m.errorMessage
          });
      }
  }
  function u(c) {
    let m, g;
    r.schemaDraft >= je.v2020_12 ? (m = t.prefixItems, g = Array.isArray(t.items) ? void 0 : t.items) : (m = Array.isArray(t.items) ? t.items : void 0, g = Array.isArray(t.items) ? t.additionalItems : t.items);
    let d = 0;
    if (m !== void 0) {
      const x = Math.min(m.length, c.items.length);
      for (; d < x; d++) {
        const R = m[d], v = Re(R), L = new Le(), b = c.items[d];
        b && (de(b, v, L, i, r), n.mergePropertyMatch(L)), n.processedProperties.add(String(d));
      }
    }
    if (g !== void 0 && d < c.items.length)
      if (typeof g == "boolean")
        for (g === !1 && n.problems.push({
          location: { offset: c.offset, length: c.length },
          message: M("Array has too many items according to schema. Expected {0} or fewer.", d)
        }); d < c.items.length; d++)
          n.processedProperties.add(String(d)), n.propertiesValueMatches++;
      else
        for (; d < c.items.length; d++) {
          const x = new Le();
          de(c.items[d], g, x, i, r), n.mergePropertyMatch(x), n.processedProperties.add(String(d));
        }
    const p = Re(t.contains);
    if (p) {
      let x = 0;
      for (let R = 0; R < c.items.length; R++) {
        const v = c.items[R], L = new Le();
        de(v, p, L, xn.instance, r), L.hasProblems() || (x++, r.schemaDraft >= je.v2020_12 && n.processedProperties.add(String(R)));
      }
      x === 0 && !ve(t.minContains) && n.problems.push({
        location: { offset: c.offset, length: c.length },
        message: t.errorMessage || M("Array does not contain required item.")
      }), ve(t.minContains) && x < t.minContains && n.problems.push({
        location: { offset: c.offset, length: c.length },
        message: t.errorMessage || M("Array has too few items that match the contains contraint. Expected {0} or more.", t.minContains)
      }), ve(t.maxContains) && x > t.maxContains && n.problems.push({
        location: { offset: c.offset, length: c.length },
        message: t.errorMessage || M("Array has too many items that match the contains contraint. Expected {0} or less.", t.maxContains)
      });
    }
    const _ = t.unevaluatedItems;
    if (_ !== void 0)
      for (let x = 0; x < c.items.length; x++) {
        if (!n.processedProperties.has(String(x)))
          if (_ === !1)
            n.problems.push({
              location: { offset: c.offset, length: c.length },
              message: M("Item does not match any validation rule from the array.")
            });
          else {
            const R = new Le();
            de(c.items[x], t.unevaluatedItems, R, i, r), n.mergePropertyMatch(R);
          }
        n.processedProperties.add(String(x)), n.propertiesValueMatches++;
      }
    if (ve(t.minItems) && c.items.length < t.minItems && n.problems.push({
      location: { offset: c.offset, length: c.length },
      message: M("Array has too few items. Expected {0} or more.", t.minItems)
    }), ve(t.maxItems) && c.items.length > t.maxItems && n.problems.push({
      location: { offset: c.offset, length: c.length },
      message: M("Array has too many items. Expected {0} or fewer.", t.maxItems)
    }), t.uniqueItems === !0) {
      let x = function() {
        for (let v = 0; v < R.length - 1; v++) {
          const L = R[v];
          for (let b = v + 1; b < R.length; b++)
            if (jt(L, R[b]))
              return !0;
        }
        return !1;
      };
      const R = xt(c);
      x() && n.problems.push({
        location: { offset: c.offset, length: c.length },
        message: M("Array has duplicate items.")
      });
    }
  }
  function h(c) {
    const m = /* @__PURE__ */ Object.create(null), g = /* @__PURE__ */ new Set();
    for (const v of c.properties) {
      const L = v.keyNode.value;
      m[L] = v.valueNode, g.add(L);
    }
    if (Array.isArray(t.required)) {
      for (const v of t.required)
        if (!m[v]) {
          const L = c.parent && c.parent.type === "property" && c.parent.keyNode, b = L ? { offset: L.offset, length: L.length } : { offset: c.offset, length: 1 };
          n.problems.push({
            location: b,
            message: M('Missing property "{0}".', v)
          });
        }
    }
    const d = (v) => {
      g.delete(v), n.processedProperties.add(v);
    };
    if (t.properties)
      for (const v of Object.keys(t.properties)) {
        d(v);
        const L = t.properties[v], b = m[v];
        if (b)
          if (Ye(L))
            if (L)
              n.propertiesMatches++, n.propertiesValueMatches++;
            else {
              const E = b.parent;
              n.problems.push({
                location: { offset: E.keyNode.offset, length: E.keyNode.length },
                message: t.errorMessage || M("Property {0} is not allowed.", v)
              });
            }
          else {
            const E = new Le();
            de(b, L, E, i, r), n.mergePropertyMatch(E);
          }
      }
    if (t.patternProperties)
      for (const v of Object.keys(t.patternProperties)) {
        const L = Cn(v);
        if (L) {
          const b = [];
          for (const E of g)
            if (L.test(E)) {
              b.push(E);
              const k = m[E];
              if (k) {
                const D = t.patternProperties[v];
                if (Ye(D))
                  if (D)
                    n.propertiesMatches++, n.propertiesValueMatches++;
                  else {
                    const O = k.parent;
                    n.problems.push({
                      location: { offset: O.keyNode.offset, length: O.keyNode.length },
                      message: t.errorMessage || M("Property {0} is not allowed.", E)
                    });
                  }
                else {
                  const O = new Le();
                  de(k, D, O, i, r), n.mergePropertyMatch(O);
                }
              }
            }
          b.forEach(d);
        }
      }
    const p = t.additionalProperties;
    if (p !== void 0)
      for (const v of g) {
        d(v);
        const L = m[v];
        if (L) {
          if (p === !1) {
            const b = L.parent;
            n.problems.push({
              location: { offset: b.keyNode.offset, length: b.keyNode.length },
              message: t.errorMessage || M("Property {0} is not allowed.", v)
            });
          } else if (p !== !0) {
            const b = new Le();
            de(L, p, b, i, r), n.mergePropertyMatch(b);
          }
        }
      }
    const _ = t.unevaluatedProperties;
    if (_ !== void 0) {
      const v = [];
      for (const L of g)
        if (!n.processedProperties.has(L)) {
          v.push(L);
          const b = m[L];
          if (b) {
            if (_ === !1) {
              const E = b.parent;
              n.problems.push({
                location: { offset: E.keyNode.offset, length: E.keyNode.length },
                message: t.errorMessage || M("Property {0} is not allowed.", L)
              });
            } else if (_ !== !0) {
              const E = new Le();
              de(b, _, E, i, r), n.mergePropertyMatch(E);
            }
          }
        }
      v.forEach(d);
    }
    if (ve(t.maxProperties) && c.properties.length > t.maxProperties && n.problems.push({
      location: { offset: c.offset, length: c.length },
      message: M("Object has more properties than limit of {0}.", t.maxProperties)
    }), ve(t.minProperties) && c.properties.length < t.minProperties && n.problems.push({
      location: { offset: c.offset, length: c.length },
      message: M("Object has fewer properties than the required number of {0}", t.minProperties)
    }), t.dependentRequired)
      for (const v in t.dependentRequired) {
        const L = m[v], b = t.dependentRequired[v];
        L && Array.isArray(b) && R(v, b);
      }
    if (t.dependentSchemas)
      for (const v in t.dependentSchemas) {
        const L = m[v], b = t.dependentSchemas[v];
        L && ft(b) && R(v, b);
      }
    if (t.dependencies)
      for (const v in t.dependencies)
        m[v] && R(v, t.dependencies[v]);
    const x = Re(t.propertyNames);
    if (x)
      for (const v of c.properties) {
        const L = v.keyNode;
        L && de(L, x, n, xn.instance, r);
      }
    function R(v, L) {
      if (Array.isArray(L))
        for (const b of L)
          m[b] ? n.propertiesValueMatches++ : n.problems.push({
            location: { offset: c.offset, length: c.length },
            message: M("Object is missing property {0} required by property {1}.", b, v)
          });
      else {
        const b = Re(L);
        if (b) {
          const E = new Le();
          de(c, b, E, i, r), n.mergePropertyMatch(E);
        }
      }
    }
  }
}
function Y1(e, t) {
  const n = [];
  let i = -1;
  const r = e.getText(), s = At(r, !1), a = t && t.collectComments ? [] : void 0;
  function l() {
    for (; ; ) {
      const E = s.scan();
      switch (h(), E) {
        case 12:
        case 13:
          Array.isArray(a) && a.push(X.create(e.positionAt(s.getTokenOffset()), e.positionAt(s.getTokenOffset() + s.getTokenLength())));
          break;
        case 15:
        case 14:
          break;
        default:
          return E;
      }
    }
  }
  function o(E, k, D, O, W = Me.Error) {
    if (n.length === 0 || D !== i) {
      const U = X.create(e.positionAt(D), e.positionAt(O));
      n.push(nt.create(U, E, W, k, e.languageId)), i = D;
    }
  }
  function u(E, k, D = void 0, O = [], W = []) {
    let U = s.getTokenOffset(), A = s.getTokenOffset() + s.getTokenLength();
    if (U === A && U > 0) {
      for (U--; U > 0 && /\s/.test(r.charAt(U)); )
        U--;
      A = U + 1;
    }
    if (o(E, k, U, A), D && c(D, !1), O.length + W.length > 0) {
      let y = s.getToken();
      for (; y !== 17; ) {
        if (O.indexOf(y) !== -1) {
          l();
          break;
        } else if (W.indexOf(y) !== -1)
          break;
        y = l();
      }
    }
    return D;
  }
  function h() {
    switch (s.getTokenError()) {
      case 4:
        return u(M("Invalid unicode sequence in string."), K.InvalidUnicode), !0;
      case 5:
        return u(M("Invalid escape character in string."), K.InvalidEscapeCharacter), !0;
      case 3:
        return u(M("Unexpected end of number."), K.UnexpectedEndOfNumber), !0;
      case 1:
        return u(M("Unexpected end of comment."), K.UnexpectedEndOfComment), !0;
      case 2:
        return u(M("Unexpected end of string."), K.UnexpectedEndOfString), !0;
      case 6:
        return u(M("Invalid characters in string. Control characters must be escaped."), K.InvalidCharacter), !0;
    }
    return !1;
  }
  function c(E, k) {
    return E.length = s.getTokenOffset() + s.getTokenLength() - E.offset, k && l(), E;
  }
  function m(E) {
    if (s.getToken() !== 3)
      return;
    const k = new $1(E, s.getTokenOffset());
    l();
    let D = !1;
    for (; s.getToken() !== 4 && s.getToken() !== 17; ) {
      if (s.getToken() === 5) {
        D || u(M("Value expected"), K.ValueExpected);
        const W = s.getTokenOffset();
        if (l(), s.getToken() === 4) {
          D && o(M("Trailing comma"), K.TrailingComma, W, W + 1);
          continue;
        }
      } else D && u(M("Expected comma"), K.CommaExpected);
      const O = v(k);
      O ? k.items.push(O) : u(M("Value expected"), K.ValueExpected, void 0, [], [
        4,
        5
        /* Json.SyntaxKind.CommaToken */
      ]), D = !0;
    }
    return s.getToken() !== 4 ? u(M("Expected comma or closing bracket"), K.CommaOrCloseBacketExpected, k) : c(k, !0);
  }
  const g = new Li(void 0, 0, 0);
  function d(E, k) {
    const D = new j1(E, s.getTokenOffset(), g);
    let O = _(D);
    if (!O)
      if (s.getToken() === 16) {
        u(M("Property keys must be doublequoted"), K.PropertyKeysMustBeDoublequoted);
        const U = new Li(D, s.getTokenOffset(), s.getTokenLength());
        U.value = s.getTokenValue(), O = U, l();
      } else
        return;
    if (D.keyNode = O, O.value !== "//") {
      const U = k[O.value];
      U ? (o(M("Duplicate object key"), K.DuplicateKey, D.keyNode.offset, D.keyNode.offset + D.keyNode.length, Me.Warning), ft(U) && o(M("Duplicate object key"), K.DuplicateKey, U.keyNode.offset, U.keyNode.offset + U.keyNode.length, Me.Warning), k[O.value] = !0) : k[O.value] = D;
    }
    if (s.getToken() === 6)
      D.colonOffset = s.getTokenOffset(), l();
    else if (u(M("Colon expected"), K.ColonExpected), s.getToken() === 10 && e.positionAt(O.offset + O.length).line < e.positionAt(s.getTokenOffset()).line)
      return D.length = O.length, D;
    const W = v(D);
    return W ? (D.valueNode = W, D.length = W.offset + W.length - D.offset, D) : u(M("Value expected"), K.ValueExpected, D, [], [
      2,
      5
      /* Json.SyntaxKind.CommaToken */
    ]);
  }
  function p(E) {
    if (s.getToken() !== 1)
      return;
    const k = new G1(E, s.getTokenOffset()), D = /* @__PURE__ */ Object.create(null);
    l();
    let O = !1;
    for (; s.getToken() !== 2 && s.getToken() !== 17; ) {
      if (s.getToken() === 5) {
        O || u(M("Property expected"), K.PropertyExpected);
        const U = s.getTokenOffset();
        if (l(), s.getToken() === 2) {
          O && o(M("Trailing comma"), K.TrailingComma, U, U + 1);
          continue;
        }
      } else O && u(M("Expected comma"), K.CommaExpected);
      const W = d(k, D);
      W ? k.properties.push(W) : u(M("Property expected"), K.PropertyExpected, void 0, [], [
        2,
        5
        /* Json.SyntaxKind.CommaToken */
      ]), O = !0;
    }
    return s.getToken() !== 2 ? u(M("Expected comma or closing brace"), K.CommaOrCloseBraceExpected, k) : c(k, !0);
  }
  function _(E) {
    if (s.getToken() !== 10)
      return;
    const k = new Li(E, s.getTokenOffset());
    return k.value = s.getTokenValue(), c(k, !0);
  }
  function x(E) {
    if (s.getToken() !== 11)
      return;
    const k = new W1(E, s.getTokenOffset());
    if (s.getTokenError() === 0) {
      const D = s.getTokenValue();
      try {
        const O = JSON.parse(D);
        if (!ve(O))
          return u(M("Invalid number format."), K.Undefined, k);
        k.value = O;
      } catch {
        return u(M("Invalid number format."), K.Undefined, k);
      }
      k.isInteger = D.indexOf(".") === -1;
    }
    return c(k, !0);
  }
  function R(E) {
    switch (s.getToken()) {
      case 7:
        return c(new H1(E, s.getTokenOffset()), !0);
      case 8:
        return c(new Go(E, !0, s.getTokenOffset()), !0);
      case 9:
        return c(new Go(E, !1, s.getTokenOffset()), !0);
      default:
        return;
    }
  }
  function v(E) {
    return m(E) || p(E) || _(E) || x(E) || R(E);
  }
  let L;
  return l() !== 17 && (L = v(L), L ? s.getToken() !== 17 && u(M("End of file expected."), K.Undefined) : u(M("Expected a JSON object, array or literal."), K.Undefined)), new Il(L, n, a);
}
function mr(e, t, n) {
  if (e !== null && typeof e == "object") {
    const i = t + "	";
    if (Array.isArray(e)) {
      if (e.length === 0)
        return "[]";
      let r = `[
`;
      for (let s = 0; s < e.length; s++)
        r += i + mr(e[s], i, n), s < e.length - 1 && (r += ","), r += `
`;
      return r += t + "]", r;
    } else {
      const r = Object.keys(e);
      if (r.length === 0)
        return "{}";
      let s = `{
`;
      for (let a = 0; a < r.length; a++) {
        const l = r[a];
        s += i + JSON.stringify(l) + ": " + mr(e[l], i, n), a < r.length - 1 && (s += ","), s += `
`;
      }
      return s += t + "}", s;
    }
  }
  return n(e);
}
var Q1 = class {
  constructor(e, t = [], n = Promise, i = {}) {
    this.schemaService = e, this.contributions = t, this.promiseConstructor = n, this.clientCapabilities = i;
  }
  doResolve(e) {
    for (let t = this.contributions.length - 1; t >= 0; t--) {
      const n = this.contributions[t].resolveCompletion;
      if (n) {
        const i = n(e);
        if (i)
          return i;
      }
    }
    return this.promiseConstructor.resolve(e);
  }
  doComplete(e, t, n) {
    const i = {
      items: [],
      isIncomplete: !1
    }, r = e.getText(), s = e.offsetAt(t);
    let a = n.getNodeFromOffset(s, !0);
    if (this.isInComment(e, a ? a.offset : 0, s))
      return Promise.resolve(i);
    if (a && s === a.offset + a.length && s > 0) {
      const c = r[s - 1];
      (a.type === "object" && c === "}" || a.type === "array" && c === "]") && (a = a.parent);
    }
    const l = this.getCurrentWord(e, s);
    let o;
    if (a && (a.type === "string" || a.type === "number" || a.type === "boolean" || a.type === "null"))
      o = X.create(e.positionAt(a.offset), e.positionAt(a.offset + a.length));
    else {
      let c = s - l.length;
      c > 0 && r[c - 1] === '"' && c--, o = X.create(e.positionAt(c), t);
    }
    const u = /* @__PURE__ */ new Map(), h = {
      add: (c) => {
        let m = c.label;
        const g = u.get(m);
        if (g)
          g.documentation || (g.documentation = c.documentation), g.detail || (g.detail = c.detail), g.labelDetails || (g.labelDetails = c.labelDetails);
        else {
          if (m = m.replace(/[\n]/g, "↵"), m.length > 60) {
            const d = m.substr(0, 57).trim() + "...";
            u.has(d) || (m = d);
          }
          c.textEdit = Ze.replace(o, c.insertText), c.label = m, u.set(m, c), i.items.push(c);
        }
      },
      setAsIncomplete: () => {
        i.isIncomplete = !0;
      },
      error: (c) => {
        console.error(c);
      },
      getNumberOfProposals: () => i.items.length
    };
    return this.schemaService.getSchemaForResource(e.uri, n).then((c) => {
      const m = [];
      let g = !0, d = "", p;
      if (a && a.type === "string") {
        const x = a.parent;
        x && x.type === "property" && x.keyNode === a && (g = !x.valueNode, p = x, d = r.substr(a.offset + 1, a.length - 2), x && (a = x.parent));
      }
      if (a && a.type === "object") {
        if (a.offset === s)
          return i;
        a.properties.forEach((L) => {
          (!p || p !== L) && u.set(L.keyNode.value, ar.create("__"));
        });
        let R = "";
        g && (R = this.evaluateSeparatorAfter(e, e.offsetAt(o.end))), c ? this.getPropertyCompletions(c, n, a, g, R, h) : this.getSchemaLessPropertyCompletions(n, a, d, h);
        const v = hr(a);
        this.contributions.forEach((L) => {
          const b = L.collectPropertyCompletions(e.uri, v, l, g, R === "", h);
          b && m.push(b);
        }), !c && l.length > 0 && r.charAt(s - l.length - 1) !== '"' && (h.add({
          kind: Te.Property,
          label: this.getLabelForValue(l),
          insertText: this.getInsertTextForProperty(l, void 0, !1, R),
          insertTextFormat: he.Snippet,
          documentation: ""
        }), h.setAsIncomplete());
      }
      const _ = {};
      return c ? this.getValueCompletions(c, n, a, s, e, h, _) : this.getSchemaLessValueCompletions(n, a, s, e, h), this.contributions.length > 0 && this.getContributedValueCompletions(n, a, s, e, h, m), this.promiseConstructor.all(m).then(() => {
        if (h.getNumberOfProposals() === 0) {
          let x = s;
          a && (a.type === "string" || a.type === "number" || a.type === "boolean" || a.type === "null") && (x = a.offset + a.length);
          const R = this.evaluateSeparatorAfter(e, x);
          this.addFillerValueCompletions(_, R, h);
        }
        return i;
      });
    });
  }
  getPropertyCompletions(e, t, n, i, r, s) {
    t.getMatchingSchemas(e.schema, n.offset).forEach((l) => {
      if (l.node === n && !l.inverted) {
        const o = l.schema.properties;
        o && Object.keys(o).forEach((h) => {
          const c = o[h];
          if (typeof c == "object" && !c.deprecationMessage && !c.doNotSuggest) {
            const m = {
              kind: Te.Property,
              label: h,
              insertText: this.getInsertTextForProperty(h, c, i, r),
              insertTextFormat: he.Snippet,
              filterText: this.getFilterTextForValue(h),
              documentation: this.fromMarkup(c.markdownDescription) || c.description || ""
            };
            c.suggestSortText !== void 0 && (m.sortText = c.suggestSortText), m.insertText && wn(m.insertText, `$1${r}`) && (m.command = {
              title: "Suggest",
              command: "editor.action.triggerSuggest"
            }), s.add(m);
          }
        });
        const u = l.schema.propertyNames;
        if (typeof u == "object" && !u.deprecationMessage && !u.doNotSuggest) {
          const h = (c, m = void 0) => {
            const g = {
              kind: Te.Property,
              label: c,
              insertText: this.getInsertTextForProperty(c, void 0, i, r),
              insertTextFormat: he.Snippet,
              filterText: this.getFilterTextForValue(c),
              documentation: m || this.fromMarkup(u.markdownDescription) || u.description || ""
            };
            u.suggestSortText !== void 0 && (g.sortText = u.suggestSortText), g.insertText && wn(g.insertText, `$1${r}`) && (g.command = {
              title: "Suggest",
              command: "editor.action.triggerSuggest"
            }), s.add(g);
          };
          if (u.enum)
            for (let c = 0; c < u.enum.length; c++) {
              let m;
              u.markdownEnumDescriptions && c < u.markdownEnumDescriptions.length ? m = this.fromMarkup(u.markdownEnumDescriptions[c]) : u.enumDescriptions && c < u.enumDescriptions.length && (m = u.enumDescriptions[c]), h(u.enum[c], m);
            }
          u.const && h(u.const);
        }
      }
    });
  }
  getSchemaLessPropertyCompletions(e, t, n, i) {
    const r = (s) => {
      s.properties.forEach((a) => {
        const l = a.keyNode.value;
        i.add({
          kind: Te.Property,
          label: l,
          insertText: this.getInsertTextForValue(l, ""),
          insertTextFormat: he.Snippet,
          filterText: this.getFilterTextForValue(l),
          documentation: ""
        });
      });
    };
    if (t.parent)
      if (t.parent.type === "property") {
        const s = t.parent.keyNode.value;
        e.visit((a) => (a.type === "property" && a !== t.parent && a.keyNode.value === s && a.valueNode && a.valueNode.type === "object" && r(a.valueNode), !0));
      } else t.parent.type === "array" && t.parent.items.forEach((s) => {
        s.type === "object" && s !== t && r(s);
      });
    else t.type === "object" && i.add({
      kind: Te.Property,
      label: "$schema",
      insertText: this.getInsertTextForProperty("$schema", void 0, !0, ""),
      insertTextFormat: he.Snippet,
      documentation: "",
      filterText: this.getFilterTextForValue("$schema")
    });
  }
  getSchemaLessValueCompletions(e, t, n, i, r) {
    let s = n;
    if (t && (t.type === "string" || t.type === "number" || t.type === "boolean" || t.type === "null") && (s = t.offset + t.length, t = t.parent), !t) {
      r.add({
        kind: this.getSuggestionKind("object"),
        label: "Empty object",
        insertText: this.getInsertTextForValue({}, ""),
        insertTextFormat: he.Snippet,
        documentation: ""
      }), r.add({
        kind: this.getSuggestionKind("array"),
        label: "Empty array",
        insertText: this.getInsertTextForValue([], ""),
        insertTextFormat: he.Snippet,
        documentation: ""
      });
      return;
    }
    const a = this.evaluateSeparatorAfter(i, s), l = (o) => {
      o.parent && !Ml(o.parent, n, !0) && r.add({
        kind: this.getSuggestionKind(o.type),
        label: this.getLabelTextForMatchingNode(o, i),
        insertText: this.getInsertTextForMatchingNode(o, i, a),
        insertTextFormat: he.Snippet,
        documentation: ""
      }), o.type === "boolean" && this.addBooleanValueCompletion(!o.value, a, r);
    };
    if (t.type === "property" && n > (t.colonOffset || 0)) {
      const o = t.valueNode;
      if (o && (n > o.offset + o.length || o.type === "object" || o.type === "array"))
        return;
      const u = t.keyNode.value;
      e.visit((h) => (h.type === "property" && h.keyNode.value === u && h.valueNode && l(h.valueNode), !0)), u === "$schema" && t.parent && !t.parent.parent && this.addDollarSchemaCompletions(a, r);
    }
    if (t.type === "array")
      if (t.parent && t.parent.type === "property") {
        const o = t.parent.keyNode.value;
        e.visit((u) => (u.type === "property" && u.keyNode.value === o && u.valueNode && u.valueNode.type === "array" && u.valueNode.items.forEach(l), !0));
      } else
        t.items.forEach(l);
  }
  getValueCompletions(e, t, n, i, r, s, a) {
    let l = i, o, u;
    if (n && (n.type === "string" || n.type === "number" || n.type === "boolean" || n.type === "null") && (l = n.offset + n.length, u = n, n = n.parent), !n) {
      this.addSchemaValueCompletions(e.schema, "", s, a);
      return;
    }
    if (n.type === "property" && i > (n.colonOffset || 0)) {
      const h = n.valueNode;
      if (h && i > h.offset + h.length)
        return;
      o = n.keyNode.value, n = n.parent;
    }
    if (n && (o !== void 0 || n.type === "array")) {
      const h = this.evaluateSeparatorAfter(r, l), c = t.getMatchingSchemas(e.schema, n.offset, u);
      for (const m of c)
        if (m.node === n && !m.inverted && m.schema) {
          if (n.type === "array" && m.schema.items) {
            let g = s;
            if (m.schema.uniqueItems) {
              const d = /* @__PURE__ */ new Set();
              n.children.forEach((p) => {
                p.type !== "array" && p.type !== "object" && d.add(this.getLabelForValue(xt(p)));
              }), g = {
                ...s,
                add(p) {
                  d.has(p.label) || s.add(p);
                }
              };
            }
            if (Array.isArray(m.schema.items)) {
              const d = this.findItemAtOffset(n, r, i);
              d < m.schema.items.length && this.addSchemaValueCompletions(m.schema.items[d], h, g, a);
            } else
              this.addSchemaValueCompletions(m.schema.items, h, g, a);
          }
          if (o !== void 0) {
            let g = !1;
            if (m.schema.properties) {
              const d = m.schema.properties[o];
              d && (g = !0, this.addSchemaValueCompletions(d, h, s, a));
            }
            if (m.schema.patternProperties && !g)
              for (const d of Object.keys(m.schema.patternProperties)) {
                const p = Cn(d);
                if (p != null && p.test(o)) {
                  g = !0;
                  const _ = m.schema.patternProperties[d];
                  this.addSchemaValueCompletions(_, h, s, a);
                }
              }
            if (m.schema.additionalProperties && !g) {
              const d = m.schema.additionalProperties;
              this.addSchemaValueCompletions(d, h, s, a);
            }
          }
        }
      o === "$schema" && !n.parent && this.addDollarSchemaCompletions(h, s), a.boolean && (this.addBooleanValueCompletion(!0, h, s), this.addBooleanValueCompletion(!1, h, s)), a.null && this.addNullValueCompletion(h, s);
    }
  }
  getContributedValueCompletions(e, t, n, i, r, s) {
    if (!t)
      this.contributions.forEach((a) => {
        const l = a.collectDefaultCompletions(i.uri, r);
        l && s.push(l);
      });
    else if ((t.type === "string" || t.type === "number" || t.type === "boolean" || t.type === "null") && (t = t.parent), t && t.type === "property" && n > (t.colonOffset || 0)) {
      const a = t.keyNode.value, l = t.valueNode;
      if ((!l || n <= l.offset + l.length) && t.parent) {
        const o = hr(t.parent);
        this.contributions.forEach((u) => {
          const h = u.collectValueCompletions(i.uri, o, a, r);
          h && s.push(h);
        });
      }
    }
  }
  addSchemaValueCompletions(e, t, n, i) {
    typeof e == "object" && (this.addEnumValueCompletions(e, t, n), this.addDefaultValueCompletions(e, t, n), this.collectTypes(e, i), Array.isArray(e.allOf) && e.allOf.forEach((r) => this.addSchemaValueCompletions(r, t, n, i)), Array.isArray(e.anyOf) && e.anyOf.forEach((r) => this.addSchemaValueCompletions(r, t, n, i)), Array.isArray(e.oneOf) && e.oneOf.forEach((r) => this.addSchemaValueCompletions(r, t, n, i)));
  }
  addDefaultValueCompletions(e, t, n, i = 0) {
    let r = !1;
    if (qe(e.default)) {
      let s = e.type, a = e.default;
      for (let o = i; o > 0; o--)
        a = [a], s = "array";
      const l = {
        kind: this.getSuggestionKind(s),
        label: this.getLabelForValue(a),
        insertText: this.getInsertTextForValue(a, t),
        insertTextFormat: he.Snippet
      };
      this.doesSupportsLabelDetails() ? l.labelDetails = { description: M("Default value") } : l.detail = M("Default value"), n.add(l), r = !0;
    }
    Array.isArray(e.examples) && e.examples.forEach((s) => {
      let a = e.type, l = s;
      for (let o = i; o > 0; o--)
        l = [l], a = "array";
      n.add({
        kind: this.getSuggestionKind(a),
        label: this.getLabelForValue(l),
        insertText: this.getInsertTextForValue(l, t),
        insertTextFormat: he.Snippet
      }), r = !0;
    }), Array.isArray(e.defaultSnippets) && e.defaultSnippets.forEach((s) => {
      let a = e.type, l = s.body, o = s.label, u, h;
      if (qe(l)) {
        e.type;
        for (let c = i; c > 0; c--)
          l = [l];
        u = this.getInsertTextForSnippetValue(l, t), h = this.getFilterTextForSnippetValue(l), o = o || this.getLabelForSnippetValue(l);
      } else if (typeof s.bodyText == "string") {
        let c = "", m = "", g = "";
        for (let d = i; d > 0; d--)
          c = c + g + `[
`, m = m + `
` + g + "]", g += "	", a = "array";
        u = c + g + s.bodyText.split(`
`).join(`
` + g) + m + t, o = o || u, h = u.replace(/[\n]/g, "");
      } else
        return;
      n.add({
        kind: this.getSuggestionKind(a),
        label: o,
        documentation: this.fromMarkup(s.markdownDescription) || s.description,
        insertText: u,
        insertTextFormat: he.Snippet,
        filterText: h
      }), r = !0;
    }), !r && typeof e.items == "object" && !Array.isArray(e.items) && i < 5 && this.addDefaultValueCompletions(e.items, t, n, i + 1);
  }
  addEnumValueCompletions(e, t, n) {
    if (qe(e.const) && n.add({
      kind: this.getSuggestionKind(e.type),
      label: this.getLabelForValue(e.const),
      insertText: this.getInsertTextForValue(e.const, t),
      insertTextFormat: he.Snippet,
      documentation: this.fromMarkup(e.markdownDescription) || e.description
    }), Array.isArray(e.enum))
      for (let i = 0, r = e.enum.length; i < r; i++) {
        const s = e.enum[i];
        let a = this.fromMarkup(e.markdownDescription) || e.description;
        e.markdownEnumDescriptions && i < e.markdownEnumDescriptions.length && this.doesSupportMarkdown() ? a = this.fromMarkup(e.markdownEnumDescriptions[i]) : e.enumDescriptions && i < e.enumDescriptions.length && (a = e.enumDescriptions[i]), n.add({
          kind: this.getSuggestionKind(e.type),
          label: this.getLabelForValue(s),
          insertText: this.getInsertTextForValue(s, t),
          insertTextFormat: he.Snippet,
          documentation: a
        });
      }
  }
  collectTypes(e, t) {
    if (Array.isArray(e.enum) || qe(e.const))
      return;
    const n = e.type;
    Array.isArray(n) ? n.forEach((i) => t[i] = !0) : n && (t[n] = !0);
  }
  addFillerValueCompletions(e, t, n) {
    e.object && n.add({
      kind: this.getSuggestionKind("object"),
      label: "{}",
      insertText: this.getInsertTextForGuessedValue({}, t),
      insertTextFormat: he.Snippet,
      detail: M("New object"),
      documentation: ""
    }), e.array && n.add({
      kind: this.getSuggestionKind("array"),
      label: "[]",
      insertText: this.getInsertTextForGuessedValue([], t),
      insertTextFormat: he.Snippet,
      detail: M("New array"),
      documentation: ""
    });
  }
  addBooleanValueCompletion(e, t, n) {
    n.add({
      kind: this.getSuggestionKind("boolean"),
      label: e ? "true" : "false",
      insertText: this.getInsertTextForValue(e, t),
      insertTextFormat: he.Snippet,
      documentation: ""
    });
  }
  addNullValueCompletion(e, t) {
    t.add({
      kind: this.getSuggestionKind("null"),
      label: "null",
      insertText: "null" + e,
      insertTextFormat: he.Snippet,
      documentation: ""
    });
  }
  addDollarSchemaCompletions(e, t) {
    this.schemaService.getRegisteredSchemaIds((i) => i === "http" || i === "https").forEach((i) => {
      i.startsWith("http://json-schema.org/draft-") && (i = i + "#"), t.add({
        kind: Te.Module,
        label: this.getLabelForValue(i),
        filterText: this.getFilterTextForValue(i),
        insertText: this.getInsertTextForValue(i, e),
        insertTextFormat: he.Snippet,
        documentation: ""
      });
    });
  }
  getLabelForValue(e) {
    return JSON.stringify(e);
  }
  getValueFromLabel(e) {
    return JSON.parse(e);
  }
  getFilterTextForValue(e) {
    return JSON.stringify(e);
  }
  getFilterTextForSnippetValue(e) {
    return JSON.stringify(e).replace(/\$\{\d+:([^}]+)\}|\$\d+/g, "$1");
  }
  getLabelForSnippetValue(e) {
    return JSON.stringify(e).replace(/\$\{\d+:([^}]+)\}|\$\d+/g, "$1");
  }
  getInsertTextForPlainText(e) {
    return e.replace(/[\\\$\}]/g, "\\$&");
  }
  getInsertTextForValue(e, t) {
    const n = JSON.stringify(e, null, "	");
    return n === "{}" ? "{$1}" + t : n === "[]" ? "[$1]" + t : this.getInsertTextForPlainText(n + t);
  }
  getInsertTextForSnippetValue(e, t) {
    return mr(e, "", (i) => typeof i == "string" && i[0] === "^" ? i.substr(1) : JSON.stringify(i)) + t;
  }
  getInsertTextForGuessedValue(e, t) {
    switch (typeof e) {
      case "object":
        return e === null ? "${1:null}" + t : this.getInsertTextForValue(e, t);
      case "string":
        let n = JSON.stringify(e);
        return n = n.substr(1, n.length - 2), n = this.getInsertTextForPlainText(n), '"${1:' + n + '}"' + t;
      case "number":
      case "boolean":
        return "${1:" + JSON.stringify(e) + "}" + t;
    }
    return this.getInsertTextForValue(e, t);
  }
  getSuggestionKind(e) {
    if (Array.isArray(e)) {
      const t = e;
      e = t.length > 0 ? t[0] : void 0;
    }
    if (!e)
      return Te.Value;
    switch (e) {
      case "string":
        return Te.Value;
      case "object":
        return Te.Module;
      case "property":
        return Te.Property;
      default:
        return Te.Value;
    }
  }
  getLabelTextForMatchingNode(e, t) {
    switch (e.type) {
      case "array":
        return "[]";
      case "object":
        return "{}";
      default:
        return t.getText().substr(e.offset, e.length);
    }
  }
  getInsertTextForMatchingNode(e, t, n) {
    switch (e.type) {
      case "array":
        return this.getInsertTextForValue([], n);
      case "object":
        return this.getInsertTextForValue({}, n);
      default:
        const i = t.getText().substr(e.offset, e.length) + n;
        return this.getInsertTextForPlainText(i);
    }
  }
  getInsertTextForProperty(e, t, n, i) {
    const r = this.getInsertTextForValue(e, "");
    if (!n)
      return r;
    const s = r + ": ";
    let a, l = 0;
    if (t) {
      if (Array.isArray(t.defaultSnippets)) {
        if (t.defaultSnippets.length === 1) {
          const o = t.defaultSnippets[0].body;
          qe(o) && (a = this.getInsertTextForSnippetValue(o, ""));
        }
        l += t.defaultSnippets.length;
      }
      if (t.enum && (!a && t.enum.length === 1 && (a = this.getInsertTextForGuessedValue(t.enum[0], "")), l += t.enum.length), qe(t.const) && (a || (a = this.getInsertTextForGuessedValue(t.const, "")), l++), qe(t.default) && (a || (a = this.getInsertTextForGuessedValue(t.default, "")), l++), Array.isArray(t.examples) && t.examples.length && (a || (a = this.getInsertTextForGuessedValue(t.examples[0], "")), l += t.examples.length), l === 0) {
        let o = Array.isArray(t.type) ? t.type[0] : t.type;
        switch (o || (t.properties ? o = "object" : t.items && (o = "array")), o) {
          case "boolean":
            a = "$1";
            break;
          case "string":
            a = '"$1"';
            break;
          case "object":
            a = "{$1}";
            break;
          case "array":
            a = "[$1]";
            break;
          case "number":
          case "integer":
            a = "${1:0}";
            break;
          case "null":
            a = "${1:null}";
            break;
          default:
            return r;
        }
      }
    }
    return (!a || l > 1) && (a = "$1"), s + a + i;
  }
  getCurrentWord(e, t) {
    let n = t - 1;
    const i = e.getText();
    for (; n >= 0 && ` 	
\r\v":{[,]}`.indexOf(i.charAt(n)) === -1; )
      n--;
    return i.substring(n + 1, t);
  }
  evaluateSeparatorAfter(e, t) {
    const n = At(e.getText(), !0);
    switch (n.setPosition(t), n.scan()) {
      case 5:
      case 2:
      case 4:
      case 17:
        return "";
      default:
        return ",";
    }
  }
  findItemAtOffset(e, t, n) {
    const i = At(t.getText(), !0), r = e.items;
    for (let s = r.length - 1; s >= 0; s--) {
      const a = r[s];
      if (n > a.offset + a.length)
        return i.setPosition(a.offset + a.length), i.scan() === 5 && n >= i.getTokenOffset() + i.getTokenLength() ? s + 1 : s;
      if (n >= a.offset)
        return s;
    }
    return 0;
  }
  isInComment(e, t, n) {
    const i = At(e.getText(), !1);
    i.setPosition(t);
    let r = i.scan();
    for (; r !== 17 && i.getTokenOffset() + i.getTokenLength() < n; )
      r = i.scan();
    return (r === 12 || r === 13) && i.getTokenOffset() <= n;
  }
  fromMarkup(e) {
    if (e && this.doesSupportMarkdown())
      return {
        kind: Et.Markdown,
        value: e
      };
  }
  doesSupportMarkdown() {
    var e, t, n;
    if (!qe(this.supportsMarkdown)) {
      const i = (n = (t = (e = this.clientCapabilities.textDocument) == null ? void 0 : e.completion) == null ? void 0 : t.completionItem) == null ? void 0 : n.documentationFormat;
      this.supportsMarkdown = Array.isArray(i) && i.indexOf(Et.Markdown) !== -1;
    }
    return this.supportsMarkdown;
  }
  doesSupportsCommitCharacters() {
    var e, t, n;
    return qe(this.supportsCommitCharacters) || (this.labelDetailsSupport = (n = (t = (e = this.clientCapabilities.textDocument) == null ? void 0 : e.completion) == null ? void 0 : t.completionItem) == null ? void 0 : n.commitCharactersSupport), this.supportsCommitCharacters;
  }
  doesSupportsLabelDetails() {
    var e, t, n;
    return qe(this.labelDetailsSupport) || (this.labelDetailsSupport = (n = (t = (e = this.clientCapabilities.textDocument) == null ? void 0 : e.completion) == null ? void 0 : t.completionItem) == null ? void 0 : n.labelDetailsSupport), this.labelDetailsSupport;
  }
}, Z1 = class {
  constructor(e, t = [], n) {
    this.schemaService = e, this.contributions = t, this.promise = n || Promise;
  }
  doHover(e, t, n) {
    const i = e.offsetAt(t);
    let r = n.getNodeFromOffset(i);
    if (!r || (r.type === "object" || r.type === "array") && i > r.offset + 1 && i < r.offset + r.length - 1)
      return this.promise.resolve(null);
    const s = r;
    if (r.type === "string") {
      const u = r.parent;
      if (u && u.type === "property" && u.keyNode === r && (r = u.valueNode, !r))
        return this.promise.resolve(null);
    }
    const a = X.create(e.positionAt(s.offset), e.positionAt(s.offset + s.length)), l = (u) => ({
      contents: u,
      range: a
    }), o = hr(r);
    for (let u = this.contributions.length - 1; u >= 0; u--) {
      const c = this.contributions[u].getInfoContribution(e.uri, o);
      if (c)
        return c.then((m) => l(m));
    }
    return this.schemaService.getSchemaForResource(e.uri, n).then((u) => {
      if (u && r) {
        const h = n.getMatchingSchemas(u.schema, r.offset);
        let c, m, g, d;
        h.every((_) => {
          if (_.node === r && !_.inverted && _.schema && (c = c || _.schema.title, m = m || _.schema.markdownDescription || Ni(_.schema.description), _.schema.enum)) {
            const x = _.schema.enum.indexOf(xt(r));
            _.schema.markdownEnumDescriptions ? g = _.schema.markdownEnumDescriptions[x] : _.schema.enumDescriptions && (g = Ni(_.schema.enumDescriptions[x])), g && (d = _.schema.enum[x], typeof d != "string" && (d = JSON.stringify(d)));
          }
          return !0;
        });
        let p = "";
        return c && (p = Ni(c)), m && (p.length > 0 && (p += `

`), p += m), g && (p.length > 0 && (p += `

`), p += `\`${K1(d)}\`: ${g}`), l([p]);
      }
      return null;
    });
  }
};
function Ni(e) {
  if (e)
    return e.replace(/([^\n\r])(\r?\n)([^\n\r])/gm, `$1

$3`).replace(/[\\`*_{}[\]()#+\-.!]/g, "\\$&");
}
function K1(e) {
  return e.indexOf("`") !== -1 ? "`` " + e + " ``" : e;
}
var C1 = class {
  constructor(e, t) {
    this.jsonSchemaService = e, this.promise = t, this.validationEnabled = !0;
  }
  configure(e) {
    e && (this.validationEnabled = e.validate !== !1, this.commentSeverity = e.allowComments ? void 0 : Me.Error);
  }
  doValidation(e, t, n, i) {
    if (!this.validationEnabled)
      return this.promise.resolve([]);
    const r = [], s = {}, a = (o) => {
      const u = o.range.start.line + " " + o.range.start.character + " " + o.message;
      s[u] || (s[u] = !0, r.push(o));
    }, l = (o) => {
      let u = n != null && n.trailingCommas ? Tn(n.trailingCommas) : Me.Error, h = n != null && n.comments ? Tn(n.comments) : this.commentSeverity, c = n != null && n.schemaValidation ? Tn(n.schemaValidation) : Me.Warning, m = n != null && n.schemaRequest ? Tn(n.schemaRequest) : Me.Warning;
      if (o) {
        const g = (d, p) => {
          if (t.root && m) {
            const _ = t.root, x = _.type === "object" ? _.properties[0] : void 0;
            if (x && x.keyNode.value === "$schema") {
              const R = x.valueNode || x, v = X.create(e.positionAt(R.offset), e.positionAt(R.offset + R.length));
              a(nt.create(v, d, m, p));
            } else {
              const R = X.create(e.positionAt(_.offset), e.positionAt(_.offset + 1));
              a(nt.create(R, d, m, p));
            }
          }
        };
        if (o.errors.length)
          g(o.errors[0], K.SchemaResolveError);
        else if (c) {
          for (const p of o.warnings)
            g(p, K.SchemaUnsupportedFeature);
          const d = t.validate(e, o.schema, c, n == null ? void 0 : n.schemaDraft);
          d && d.forEach(a);
        }
        Dl(o.schema) && (h = void 0), Ul(o.schema) && (u = void 0);
      }
      for (const g of t.syntaxErrors) {
        if (g.code === K.TrailingComma) {
          if (typeof u != "number")
            continue;
          g.severity = u;
        }
        a(g);
      }
      if (typeof h == "number") {
        const g = M("Comments are not permitted in JSON.");
        t.comments.forEach((d) => {
          a(nt.create(d, g, h, K.CommentNotPermitted));
        });
      }
      return r;
    };
    if (i) {
      const o = i.id || "schemaservice://untitled/" + ef++;
      return this.jsonSchemaService.registerExternalSchema({ uri: o, schema: i }).getResolvedSchema().then((h) => l(h));
    }
    return this.jsonSchemaService.getSchemaForResource(e.uri, t).then((o) => l(o));
  }
  getLanguageStatus(e, t) {
    return { schemas: this.jsonSchemaService.getSchemaURIsForResource(e.uri, t) };
  }
}, ef = 0;
function Dl(e) {
  if (e && typeof e == "object") {
    if (Ye(e.allowComments))
      return e.allowComments;
    if (e.allOf)
      for (const t of e.allOf) {
        const n = Dl(t);
        if (Ye(n))
          return n;
      }
  }
}
function Ul(e) {
  if (e && typeof e == "object") {
    if (Ye(e.allowTrailingCommas))
      return e.allowTrailingCommas;
    const t = e;
    if (Ye(t.allowsTrailingCommas))
      return t.allowsTrailingCommas;
    if (e.allOf)
      for (const n of e.allOf) {
        const i = Ul(n);
        if (Ye(i))
          return i;
      }
  }
}
function Tn(e) {
  switch (e) {
    case "error":
      return Me.Error;
    case "warning":
      return Me.Warning;
    case "ignore":
      return;
  }
}
var Yo = 48, tf = 57, nf = 65, Sn = 97, rf = 102;
function ue(e) {
  return e < Yo ? 0 : e <= tf ? e - Yo : (e < Sn && (e += Sn - nf), e >= Sn && e <= rf ? e - Sn + 10 : 0);
}
function sf(e) {
  if (e[0] === "#")
    switch (e.length) {
      case 4:
        return {
          red: ue(e.charCodeAt(1)) * 17 / 255,
          green: ue(e.charCodeAt(2)) * 17 / 255,
          blue: ue(e.charCodeAt(3)) * 17 / 255,
          alpha: 1
        };
      case 5:
        return {
          red: ue(e.charCodeAt(1)) * 17 / 255,
          green: ue(e.charCodeAt(2)) * 17 / 255,
          blue: ue(e.charCodeAt(3)) * 17 / 255,
          alpha: ue(e.charCodeAt(4)) * 17 / 255
        };
      case 7:
        return {
          red: (ue(e.charCodeAt(1)) * 16 + ue(e.charCodeAt(2))) / 255,
          green: (ue(e.charCodeAt(3)) * 16 + ue(e.charCodeAt(4))) / 255,
          blue: (ue(e.charCodeAt(5)) * 16 + ue(e.charCodeAt(6))) / 255,
          alpha: 1
        };
      case 9:
        return {
          red: (ue(e.charCodeAt(1)) * 16 + ue(e.charCodeAt(2))) / 255,
          green: (ue(e.charCodeAt(3)) * 16 + ue(e.charCodeAt(4))) / 255,
          blue: (ue(e.charCodeAt(5)) * 16 + ue(e.charCodeAt(6))) / 255,
          alpha: (ue(e.charCodeAt(7)) * 16 + ue(e.charCodeAt(8))) / 255
        };
    }
}
var af = class {
  constructor(e) {
    this.schemaService = e;
  }
  findDocumentSymbols(e, t, n = { resultLimit: Number.MAX_VALUE }) {
    const i = t.root;
    if (!i)
      return [];
    let r = n.resultLimit || Number.MAX_VALUE;
    const s = e.uri;
    if ((s === "vscode://defaultsettings/keybindings.json" || wn(s.toLowerCase(), "/user/keybindings.json")) && i.type === "array") {
      const c = [];
      for (const m of i.items)
        if (m.type === "object") {
          for (const g of m.properties)
            if (g.keyNode.value === "key" && g.valueNode) {
              const d = Yt.create(e.uri, ot(e, m));
              if (c.push({ name: Qo(g.valueNode), kind: ze.Function, location: d }), r--, r <= 0)
                return n && n.onResultLimitExceeded && n.onResultLimitExceeded(s), c;
            }
        }
      return c;
    }
    const a = [
      { node: i, containerName: "" }
    ];
    let l = 0, o = !1;
    const u = [], h = (c, m) => {
      c.type === "array" ? c.items.forEach((g) => {
        g && a.push({ node: g, containerName: m });
      }) : c.type === "object" && c.properties.forEach((g) => {
        const d = g.valueNode;
        if (d)
          if (r > 0) {
            r--;
            const p = Yt.create(e.uri, ot(e, g)), _ = m ? m + "." + g.keyNode.value : g.keyNode.value;
            u.push({ name: this.getKeyLabel(g), kind: this.getSymbolKind(d.type), location: p, containerName: m }), a.push({ node: d, containerName: _ });
          } else
            o = !0;
      });
    };
    for (; l < a.length; ) {
      const c = a[l++];
      h(c.node, c.containerName);
    }
    return o && n && n.onResultLimitExceeded && n.onResultLimitExceeded(s), u;
  }
  findDocumentSymbols2(e, t, n = { resultLimit: Number.MAX_VALUE }) {
    const i = t.root;
    if (!i)
      return [];
    let r = n.resultLimit || Number.MAX_VALUE;
    const s = e.uri;
    if ((s === "vscode://defaultsettings/keybindings.json" || wn(s.toLowerCase(), "/user/keybindings.json")) && i.type === "array") {
      const c = [];
      for (const m of i.items)
        if (m.type === "object") {
          for (const g of m.properties)
            if (g.keyNode.value === "key" && g.valueNode) {
              const d = ot(e, m), p = ot(e, g.keyNode);
              if (c.push({ name: Qo(g.valueNode), kind: ze.Function, range: d, selectionRange: p }), r--, r <= 0)
                return n && n.onResultLimitExceeded && n.onResultLimitExceeded(s), c;
            }
        }
      return c;
    }
    const a = [], l = [
      { node: i, result: a }
    ];
    let o = 0, u = !1;
    const h = (c, m) => {
      c.type === "array" ? c.items.forEach((g, d) => {
        if (g)
          if (r > 0) {
            r--;
            const p = ot(e, g), _ = p, R = { name: String(d), kind: this.getSymbolKind(g.type), range: p, selectionRange: _, children: [] };
            m.push(R), l.push({ result: R.children, node: g });
          } else
            u = !0;
      }) : c.type === "object" && c.properties.forEach((g) => {
        const d = g.valueNode;
        if (d)
          if (r > 0) {
            r--;
            const p = ot(e, g), _ = ot(e, g.keyNode), x = [], R = { name: this.getKeyLabel(g), kind: this.getSymbolKind(d.type), range: p, selectionRange: _, children: x, detail: this.getDetail(d) };
            m.push(R), l.push({ result: x, node: d });
          } else
            u = !0;
      });
    };
    for (; o < l.length; ) {
      const c = l[o++];
      h(c.node, c.result);
    }
    return u && n && n.onResultLimitExceeded && n.onResultLimitExceeded(s), a;
  }
  getSymbolKind(e) {
    switch (e) {
      case "object":
        return ze.Module;
      case "string":
        return ze.String;
      case "number":
        return ze.Number;
      case "array":
        return ze.Array;
      case "boolean":
        return ze.Boolean;
      default:
        return ze.Variable;
    }
  }
  getKeyLabel(e) {
    let t = e.keyNode.value;
    return t && (t = t.replace(/[\n]/g, "↵")), t && t.trim() ? t : `"${t}"`;
  }
  getDetail(e) {
    if (e) {
      if (e.type === "boolean" || e.type === "number" || e.type === "null" || e.type === "string")
        return String(e.value);
      if (e.type === "array")
        return e.children.length ? void 0 : "[]";
      if (e.type === "object")
        return e.children.length ? void 0 : "{}";
    }
  }
  findDocumentColors(e, t, n) {
    return this.schemaService.getSchemaForResource(e.uri, t).then((i) => {
      const r = [];
      if (i) {
        let s = n && typeof n.resultLimit == "number" ? n.resultLimit : Number.MAX_VALUE;
        const a = t.getMatchingSchemas(i.schema), l = {};
        for (const o of a)
          if (!o.inverted && o.schema && (o.schema.format === "color" || o.schema.format === "color-hex") && o.node && o.node.type === "string") {
            const u = String(o.node.offset);
            if (!l[u]) {
              const h = sf(xt(o.node));
              if (h) {
                const c = ot(e, o.node);
                r.push({ color: h, range: c });
              }
              if (l[u] = !0, s--, s <= 0)
                return n && n.onResultLimitExceeded && n.onResultLimitExceeded(e.uri), r;
            }
          }
      }
      return r;
    });
  }
  getColorPresentations(e, t, n, i) {
    const r = [], s = Math.round(n.red * 255), a = Math.round(n.green * 255), l = Math.round(n.blue * 255);
    function o(h) {
      const c = h.toString(16);
      return c.length !== 2 ? "0" + c : c;
    }
    let u;
    return n.alpha === 1 ? u = `#${o(s)}${o(a)}${o(l)}` : u = `#${o(s)}${o(a)}${o(l)}${o(Math.round(n.alpha * 255))}`, r.push({ label: u, textEdit: Ze.replace(i, JSON.stringify(u)) }), r;
  }
};
function ot(e, t) {
  return X.create(e.positionAt(t.offset), e.positionAt(t.offset + t.length));
}
function Qo(e) {
  return xt(e) || M("<empty>");
}
var gr = {
  schemaAssociations: [],
  schemas: {
    // bundle the schema-schema to include (localized) descriptions
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
}, of = {
  id: M("A unique identifier for the schema."),
  $schema: M("The schema to verify this document against."),
  title: M("A descriptive title of the element."),
  description: M("A long description of the element. Used in hover menus and suggestions."),
  default: M("A default value. Used by suggestions."),
  multipleOf: M("A number that should cleanly divide the current value (i.e. have no remainder)."),
  maximum: M("The maximum numerical value, inclusive by default."),
  exclusiveMaximum: M("Makes the maximum property exclusive."),
  minimum: M("The minimum numerical value, inclusive by default."),
  exclusiveMinimum: M("Makes the minimum property exclusive."),
  maxLength: M("The maximum length of a string."),
  minLength: M("The minimum length of a string."),
  pattern: M("A regular expression to match the string against. It is not implicitly anchored."),
  additionalItems: M("For arrays, only when items is set as an array. If it is a schema, then this schema validates items after the ones specified by the items array. If it is false, then additional items will cause validation to fail."),
  items: M("For arrays. Can either be a schema to validate every element against or an array of schemas to validate each item against in order (the first schema will validate the first element, the second schema will validate the second element, and so on."),
  maxItems: M("The maximum number of items that can be inside an array. Inclusive."),
  minItems: M("The minimum number of items that can be inside an array. Inclusive."),
  uniqueItems: M("If all of the items in the array must be unique. Defaults to false."),
  maxProperties: M("The maximum number of properties an object can have. Inclusive."),
  minProperties: M("The minimum number of properties an object can have. Inclusive."),
  required: M("An array of strings that lists the names of all properties required on this object."),
  additionalProperties: M("Either a schema or a boolean. If a schema, then used to validate all properties not matched by 'properties' or 'patternProperties'. If false, then any properties not matched by either will cause this schema to fail."),
  definitions: M("Not used for validation. Place subschemas here that you wish to reference inline with $ref."),
  properties: M("A map of property names to schemas for each property."),
  patternProperties: M("A map of regular expressions on property names to schemas for matching properties."),
  dependencies: M("A map of property names to either an array of property names or a schema. An array of property names means the property named in the key depends on the properties in the array being present in the object in order to be valid. If the value is a schema, then the schema is only applied to the object if the property in the key exists on the object."),
  enum: M("The set of literal values that are valid."),
  type: M("Either a string of one of the basic schema types (number, integer, null, array, object, boolean, string) or an array of strings specifying a subset of those types."),
  format: M("Describes the format expected for the value."),
  allOf: M("An array of schemas, all of which must match."),
  anyOf: M("An array of schemas, where at least one must match."),
  oneOf: M("An array of schemas, exactly one of which must match."),
  not: M("A schema which must not match."),
  $id: M("A unique identifier for the schema."),
  $ref: M("Reference a definition hosted on any location."),
  $comment: M("Comments from schema authors to readers or maintainers of the schema."),
  readOnly: M("Indicates that the value of the instance is managed exclusively by the owning authority."),
  examples: M("Sample JSON values associated with a particular schema, for the purpose of illustrating usage."),
  contains: M('An array instance is valid against "contains" if at least one of its elements is valid against the given schema.'),
  propertyNames: M("If the instance is an object, this keyword validates if every property name in the instance validates against the provided schema."),
  const: M("An instance validates successfully against this keyword if its value is equal to the value of the keyword."),
  contentMediaType: M("Describes the media type of a string property."),
  contentEncoding: M("Describes the content encoding of a string property."),
  if: M('The validation outcome of the "if" subschema controls which of the "then" or "else" keywords are evaluated.'),
  then: M('The "if" subschema is used for validation when the "if" subschema succeeds.'),
  else: M('The "else" subschema is used for validation when the "if" subschema fails.')
};
for (const e in gr.schemas) {
  const t = gr.schemas[e];
  for (const n in t.properties) {
    let i = t.properties[n];
    typeof i == "boolean" && (i = t.properties[n] = {});
    const r = of[n];
    r && (i.description = r);
  }
}
var Fl;
(() => {
  var e = { 470: (r) => {
    function s(o) {
      if (typeof o != "string")
        throw new TypeError("Path must be a string. Received " + JSON.stringify(o));
    }
    function a(o, u) {
      for (var h, c = "", m = 0, g = -1, d = 0, p = 0; p <= o.length; ++p) {
        if (p < o.length)
          h = o.charCodeAt(p);
        else {
          if (h === 47)
            break;
          h = 47;
        }
        if (h === 47) {
          if (!(g === p - 1 || d === 1))
            if (g !== p - 1 && d === 2) {
              if (c.length < 2 || m !== 2 || c.charCodeAt(c.length - 1) !== 46 || c.charCodeAt(c.length - 2) !== 46) {
                if (c.length > 2) {
                  var _ = c.lastIndexOf("/");
                  if (_ !== c.length - 1) {
                    _ === -1 ? (c = "", m = 0) : m = (c = c.slice(0, _)).length - 1 - c.lastIndexOf("/"), g = p, d = 0;
                    continue;
                  }
                } else if (c.length === 2 || c.length === 1) {
                  c = "", m = 0, g = p, d = 0;
                  continue;
                }
              }
              u && (c.length > 0 ? c += "/.." : c = "..", m = 2);
            } else
              c.length > 0 ? c += "/" + o.slice(g + 1, p) : c = o.slice(g + 1, p), m = p - g - 1;
          g = p, d = 0;
        } else
          h === 46 && d !== -1 ? ++d : d = -1;
      }
      return c;
    }
    var l = { resolve: function() {
      for (var o, u = "", h = !1, c = arguments.length - 1; c >= -1 && !h; c--) {
        var m;
        c >= 0 ? m = arguments[c] : (o === void 0 && (o = process.cwd()), m = o), s(m), m.length !== 0 && (u = m + "/" + u, h = m.charCodeAt(0) === 47);
      }
      return u = a(u, !h), h ? u.length > 0 ? "/" + u : "/" : u.length > 0 ? u : ".";
    }, normalize: function(o) {
      if (s(o), o.length === 0)
        return ".";
      var u = o.charCodeAt(0) === 47, h = o.charCodeAt(o.length - 1) === 47;
      return (o = a(o, !u)).length !== 0 || u || (o = "."), o.length > 0 && h && (o += "/"), u ? "/" + o : o;
    }, isAbsolute: function(o) {
      return s(o), o.length > 0 && o.charCodeAt(0) === 47;
    }, join: function() {
      if (arguments.length === 0)
        return ".";
      for (var o, u = 0; u < arguments.length; ++u) {
        var h = arguments[u];
        s(h), h.length > 0 && (o === void 0 ? o = h : o += "/" + h);
      }
      return o === void 0 ? "." : l.normalize(o);
    }, relative: function(o, u) {
      if (s(o), s(u), o === u || (o = l.resolve(o)) === (u = l.resolve(u)))
        return "";
      for (var h = 1; h < o.length && o.charCodeAt(h) === 47; ++h)
        ;
      for (var c = o.length, m = c - h, g = 1; g < u.length && u.charCodeAt(g) === 47; ++g)
        ;
      for (var d = u.length - g, p = m < d ? m : d, _ = -1, x = 0; x <= p; ++x) {
        if (x === p) {
          if (d > p) {
            if (u.charCodeAt(g + x) === 47)
              return u.slice(g + x + 1);
            if (x === 0)
              return u.slice(g + x);
          } else
            m > p && (o.charCodeAt(h + x) === 47 ? _ = x : x === 0 && (_ = 0));
          break;
        }
        var R = o.charCodeAt(h + x);
        if (R !== u.charCodeAt(g + x))
          break;
        R === 47 && (_ = x);
      }
      var v = "";
      for (x = h + _ + 1; x <= c; ++x)
        x !== c && o.charCodeAt(x) !== 47 || (v.length === 0 ? v += ".." : v += "/..");
      return v.length > 0 ? v + u.slice(g + _) : (g += _, u.charCodeAt(g) === 47 && ++g, u.slice(g));
    }, _makeLong: function(o) {
      return o;
    }, dirname: function(o) {
      if (s(o), o.length === 0)
        return ".";
      for (var u = o.charCodeAt(0), h = u === 47, c = -1, m = !0, g = o.length - 1; g >= 1; --g)
        if ((u = o.charCodeAt(g)) === 47) {
          if (!m) {
            c = g;
            break;
          }
        } else
          m = !1;
      return c === -1 ? h ? "/" : "." : h && c === 1 ? "//" : o.slice(0, c);
    }, basename: function(o, u) {
      if (u !== void 0 && typeof u != "string")
        throw new TypeError('"ext" argument must be a string');
      s(o);
      var h, c = 0, m = -1, g = !0;
      if (u !== void 0 && u.length > 0 && u.length <= o.length) {
        if (u.length === o.length && u === o)
          return "";
        var d = u.length - 1, p = -1;
        for (h = o.length - 1; h >= 0; --h) {
          var _ = o.charCodeAt(h);
          if (_ === 47) {
            if (!g) {
              c = h + 1;
              break;
            }
          } else
            p === -1 && (g = !1, p = h + 1), d >= 0 && (_ === u.charCodeAt(d) ? --d == -1 && (m = h) : (d = -1, m = p));
        }
        return c === m ? m = p : m === -1 && (m = o.length), o.slice(c, m);
      }
      for (h = o.length - 1; h >= 0; --h)
        if (o.charCodeAt(h) === 47) {
          if (!g) {
            c = h + 1;
            break;
          }
        } else
          m === -1 && (g = !1, m = h + 1);
      return m === -1 ? "" : o.slice(c, m);
    }, extname: function(o) {
      s(o);
      for (var u = -1, h = 0, c = -1, m = !0, g = 0, d = o.length - 1; d >= 0; --d) {
        var p = o.charCodeAt(d);
        if (p !== 47)
          c === -1 && (m = !1, c = d + 1), p === 46 ? u === -1 ? u = d : g !== 1 && (g = 1) : u !== -1 && (g = -1);
        else if (!m) {
          h = d + 1;
          break;
        }
      }
      return u === -1 || c === -1 || g === 0 || g === 1 && u === c - 1 && u === h + 1 ? "" : o.slice(u, c);
    }, format: function(o) {
      if (o === null || typeof o != "object")
        throw new TypeError('The "pathObject" argument must be of type Object. Received type ' + typeof o);
      return function(u, h) {
        var c = h.dir || h.root, m = h.base || (h.name || "") + (h.ext || "");
        return c ? c === h.root ? c + m : c + "/" + m : m;
      }(0, o);
    }, parse: function(o) {
      s(o);
      var u = { root: "", dir: "", base: "", ext: "", name: "" };
      if (o.length === 0)
        return u;
      var h, c = o.charCodeAt(0), m = c === 47;
      m ? (u.root = "/", h = 1) : h = 0;
      for (var g = -1, d = 0, p = -1, _ = !0, x = o.length - 1, R = 0; x >= h; --x)
        if ((c = o.charCodeAt(x)) !== 47)
          p === -1 && (_ = !1, p = x + 1), c === 46 ? g === -1 ? g = x : R !== 1 && (R = 1) : g !== -1 && (R = -1);
        else if (!_) {
          d = x + 1;
          break;
        }
      return g === -1 || p === -1 || R === 0 || R === 1 && g === p - 1 && g === d + 1 ? p !== -1 && (u.base = u.name = d === 0 && m ? o.slice(1, p) : o.slice(d, p)) : (d === 0 && m ? (u.name = o.slice(1, g), u.base = o.slice(1, p)) : (u.name = o.slice(d, g), u.base = o.slice(d, p)), u.ext = o.slice(g, p)), d > 0 ? u.dir = o.slice(0, d - 1) : m && (u.dir = "/"), u;
    }, sep: "/", delimiter: ":", win32: null, posix: null };
    l.posix = l, r.exports = l;
  } }, t = {};
  function n(r) {
    var s = t[r];
    if (s !== void 0)
      return s.exports;
    var a = t[r] = { exports: {} };
    return e[r](a, a.exports, n), a.exports;
  }
  n.d = (r, s) => {
    for (var a in s)
      n.o(s, a) && !n.o(r, a) && Object.defineProperty(r, a, { enumerable: !0, get: s[a] });
  }, n.o = (r, s) => Object.prototype.hasOwnProperty.call(r, s), n.r = (r) => {
    typeof Symbol < "u" && Symbol.toStringTag && Object.defineProperty(r, Symbol.toStringTag, { value: "Module" }), Object.defineProperty(r, "__esModule", { value: !0 });
  };
  var i = {};
  (() => {
    let r;
    n.r(i), n.d(i, { URI: () => m, Utils: () => W }), typeof process == "object" ? r = process.platform === "win32" : typeof navigator == "object" && (r = navigator.userAgent.indexOf("Windows") >= 0);
    const s = /^\w[\w\d+.-]*$/, a = /^\//, l = /^\/\//;
    function o(U, A) {
      if (!U.scheme && A)
        throw new Error(`[UriError]: Scheme is missing: {scheme: "", authority: "${U.authority}", path: "${U.path}", query: "${U.query}", fragment: "${U.fragment}"}`);
      if (U.scheme && !s.test(U.scheme))
        throw new Error("[UriError]: Scheme contains illegal characters.");
      if (U.path) {
        if (U.authority) {
          if (!a.test(U.path))
            throw new Error('[UriError]: If a URI contains an authority component, then the path component must either be empty or begin with a slash ("/") character');
        } else if (l.test(U.path))
          throw new Error('[UriError]: If a URI does not contain an authority component, then the path cannot begin with two slash characters ("//")');
      }
    }
    const u = "", h = "/", c = /^(([^:/?#]+?):)?(\/\/([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?/;
    class m {
      constructor(A, y, I, F, P, B = !1) {
        it(this, "scheme");
        it(this, "authority");
        it(this, "path");
        it(this, "query");
        it(this, "fragment");
        typeof A == "object" ? (this.scheme = A.scheme || u, this.authority = A.authority || u, this.path = A.path || u, this.query = A.query || u, this.fragment = A.fragment || u) : (this.scheme = /* @__PURE__ */ function(G, C) {
          return G || C ? G : "file";
        }(A, B), this.authority = y || u, this.path = function(G, C) {
          switch (G) {
            case "https":
            case "http":
            case "file":
              C ? C[0] !== h && (C = h + C) : C = h;
          }
          return C;
        }(this.scheme, I || u), this.query = F || u, this.fragment = P || u, o(this, B));
      }
      static isUri(A) {
        return A instanceof m || !!A && typeof A.authority == "string" && typeof A.fragment == "string" && typeof A.path == "string" && typeof A.query == "string" && typeof A.scheme == "string" && typeof A.fsPath == "string" && typeof A.with == "function" && typeof A.toString == "function";
      }
      get fsPath() {
        return R(this);
      }
      with(A) {
        if (!A)
          return this;
        let { scheme: y, authority: I, path: F, query: P, fragment: B } = A;
        return y === void 0 ? y = this.scheme : y === null && (y = u), I === void 0 ? I = this.authority : I === null && (I = u), F === void 0 ? F = this.path : F === null && (F = u), P === void 0 ? P = this.query : P === null && (P = u), B === void 0 ? B = this.fragment : B === null && (B = u), y === this.scheme && I === this.authority && F === this.path && P === this.query && B === this.fragment ? this : new d(y, I, F, P, B);
      }
      static parse(A, y = !1) {
        const I = c.exec(A);
        return I ? new d(I[2] || u, E(I[4] || u), E(I[5] || u), E(I[7] || u), E(I[9] || u), y) : new d(u, u, u, u, u);
      }
      static file(A) {
        let y = u;
        if (r && (A = A.replace(/\\/g, h)), A[0] === h && A[1] === h) {
          const I = A.indexOf(h, 2);
          I === -1 ? (y = A.substring(2), A = h) : (y = A.substring(2, I), A = A.substring(I) || h);
        }
        return new d("file", y, A, u, u);
      }
      static from(A) {
        const y = new d(A.scheme, A.authority, A.path, A.query, A.fragment);
        return o(y, !0), y;
      }
      toString(A = !1) {
        return v(this, A);
      }
      toJSON() {
        return this;
      }
      static revive(A) {
        if (A) {
          if (A instanceof m)
            return A;
          {
            const y = new d(A);
            return y._formatted = A.external, y._fsPath = A._sep === g ? A.fsPath : null, y;
          }
        }
        return A;
      }
    }
    const g = r ? 1 : void 0;
    class d extends m {
      constructor() {
        super(...arguments);
        it(this, "_formatted", null);
        it(this, "_fsPath", null);
      }
      get fsPath() {
        return this._fsPath || (this._fsPath = R(this)), this._fsPath;
      }
      toString(y = !1) {
        return y ? v(this, !0) : (this._formatted || (this._formatted = v(this, !1)), this._formatted);
      }
      toJSON() {
        const y = { $mid: 1 };
        return this._fsPath && (y.fsPath = this._fsPath, y._sep = g), this._formatted && (y.external = this._formatted), this.path && (y.path = this.path), this.scheme && (y.scheme = this.scheme), this.authority && (y.authority = this.authority), this.query && (y.query = this.query), this.fragment && (y.fragment = this.fragment), y;
      }
    }
    const p = { 58: "%3A", 47: "%2F", 63: "%3F", 35: "%23", 91: "%5B", 93: "%5D", 64: "%40", 33: "%21", 36: "%24", 38: "%26", 39: "%27", 40: "%28", 41: "%29", 42: "%2A", 43: "%2B", 44: "%2C", 59: "%3B", 61: "%3D", 32: "%20" };
    function _(U, A, y) {
      let I, F = -1;
      for (let P = 0; P < U.length; P++) {
        const B = U.charCodeAt(P);
        if (B >= 97 && B <= 122 || B >= 65 && B <= 90 || B >= 48 && B <= 57 || B === 45 || B === 46 || B === 95 || B === 126 || A && B === 47 || y && B === 91 || y && B === 93 || y && B === 58)
          F !== -1 && (I += encodeURIComponent(U.substring(F, P)), F = -1), I !== void 0 && (I += U.charAt(P));
        else {
          I === void 0 && (I = U.substr(0, P));
          const G = p[B];
          G !== void 0 ? (F !== -1 && (I += encodeURIComponent(U.substring(F, P)), F = -1), I += G) : F === -1 && (F = P);
        }
      }
      return F !== -1 && (I += encodeURIComponent(U.substring(F))), I !== void 0 ? I : U;
    }
    function x(U) {
      let A;
      for (let y = 0; y < U.length; y++) {
        const I = U.charCodeAt(y);
        I === 35 || I === 63 ? (A === void 0 && (A = U.substr(0, y)), A += p[I]) : A !== void 0 && (A += U[y]);
      }
      return A !== void 0 ? A : U;
    }
    function R(U, A) {
      let y;
      return y = U.authority && U.path.length > 1 && U.scheme === "file" ? `//${U.authority}${U.path}` : U.path.charCodeAt(0) === 47 && (U.path.charCodeAt(1) >= 65 && U.path.charCodeAt(1) <= 90 || U.path.charCodeAt(1) >= 97 && U.path.charCodeAt(1) <= 122) && U.path.charCodeAt(2) === 58 ? U.path[1].toLowerCase() + U.path.substr(2) : U.path, r && (y = y.replace(/\//g, "\\")), y;
    }
    function v(U, A) {
      const y = A ? x : _;
      let I = "", { scheme: F, authority: P, path: B, query: G, fragment: C } = U;
      if (F && (I += F, I += ":"), (P || F === "file") && (I += h, I += h), P) {
        let Q = P.indexOf("@");
        if (Q !== -1) {
          const Be = P.substr(0, Q);
          P = P.substr(Q + 1), Q = Be.lastIndexOf(":"), Q === -1 ? I += y(Be, !1, !1) : (I += y(Be.substr(0, Q), !1, !1), I += ":", I += y(Be.substr(Q + 1), !1, !0)), I += "@";
        }
        P = P.toLowerCase(), Q = P.lastIndexOf(":"), Q === -1 ? I += y(P, !1, !0) : (I += y(P.substr(0, Q), !1, !0), I += P.substr(Q));
      }
      if (B) {
        if (B.length >= 3 && B.charCodeAt(0) === 47 && B.charCodeAt(2) === 58) {
          const Q = B.charCodeAt(1);
          Q >= 65 && Q <= 90 && (B = `/${String.fromCharCode(Q + 32)}:${B.substr(3)}`);
        } else if (B.length >= 2 && B.charCodeAt(1) === 58) {
          const Q = B.charCodeAt(0);
          Q >= 65 && Q <= 90 && (B = `${String.fromCharCode(Q + 32)}:${B.substr(2)}`);
        }
        I += y(B, !0, !1);
      }
      return G && (I += "?", I += y(G, !1, !1)), C && (I += "#", I += A ? C : _(C, !1, !1)), I;
    }
    function L(U) {
      try {
        return decodeURIComponent(U);
      } catch {
        return U.length > 3 ? U.substr(0, 3) + L(U.substr(3)) : U;
      }
    }
    const b = /(%[0-9A-Za-z][0-9A-Za-z])+/g;
    function E(U) {
      return U.match(b) ? U.replace(b, (A) => L(A)) : U;
    }
    var k = n(470);
    const D = k.posix || k, O = "/";
    var W;
    (function(U) {
      U.joinPath = function(A, ...y) {
        return A.with({ path: D.join(A.path, ...y) });
      }, U.resolvePath = function(A, ...y) {
        let I = A.path, F = !1;
        I[0] !== O && (I = O + I, F = !0);
        let P = D.resolve(I, ...y);
        return F && P[0] === O && !A.authority && (P = P.substring(1)), A.with({ path: P });
      }, U.dirname = function(A) {
        if (A.path.length === 0 || A.path === O)
          return A;
        let y = D.dirname(A.path);
        return y.length === 1 && y.charCodeAt(0) === 46 && (y = ""), A.with({ path: y });
      }, U.basename = function(A) {
        return D.basename(A.path);
      }, U.extname = function(A) {
        return D.extname(A.path);
      };
    })(W || (W = {}));
  })(), Fl = i;
})();
var { URI: Kt, Utils: Pf } = Fl;
function lf(e, t) {
  if (typeof e != "string")
    throw new TypeError("Expected a string");
  const n = String(e);
  let i = "";
  const r = t ? !!t.extended : !1, s = t ? !!t.globstar : !1;
  let a = !1;
  const l = t && typeof t.flags == "string" ? t.flags : "";
  let o;
  for (let u = 0, h = n.length; u < h; u++)
    switch (o = n[u], o) {
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
        i += "\\" + o;
        break;
      case "?":
        if (r) {
          i += ".";
          break;
        }
      case "[":
      case "]":
        if (r) {
          i += o;
          break;
        }
      case "{":
        if (r) {
          a = !0, i += "(";
          break;
        }
      case "}":
        if (r) {
          a = !1, i += ")";
          break;
        }
      case ",":
        if (a) {
          i += "|";
          break;
        }
        i += "\\" + o;
        break;
      case "*":
        const c = n[u - 1];
        let m = 1;
        for (; n[u + 1] === "*"; )
          m++, u++;
        const g = n[u + 1];
        s ? m > 1 && (c === "/" || c === void 0 || c === "{" || c === ",") && (g === "/" || g === void 0 || g === "," || g === "}") ? (g === "/" ? u++ : c === "/" && i.endsWith("\\/") && (i = i.substr(0, i.length - 2)), i += "((?:[^/]*(?:/|$))*)") : i += "([^/]*)" : i += ".*";
        break;
      default:
        i += o;
    }
  return (!l || !~l.indexOf("g")) && (i = "^" + i + "$"), new RegExp(i, l);
}
var uf = "!", cf = "/", ff = class {
  constructor(e, t, n) {
    this.folderUri = t, this.uris = n, this.globWrappers = [];
    try {
      for (let i of e) {
        const r = i[0] !== uf;
        r || (i = i.substring(1)), i.length > 0 && (i[0] === cf && (i = i.substring(1)), this.globWrappers.push({
          regexp: lf("**/" + i, { extended: !0, globstar: !0 }),
          include: r
        }));
      }
      t && (t = Pl(t), t.endsWith("/") || (t = t + "/"), this.folderUri = t);
    } catch {
      this.globWrappers.length = 0, this.uris = [];
    }
  }
  matchesPattern(e) {
    if (this.folderUri && !e.startsWith(this.folderUri))
      return !1;
    let t = !1;
    for (const { regexp: n, include: i } of this.globWrappers)
      n.test(e) && (t = i);
    return t;
  }
  getURIs() {
    return this.uris;
  }
}, hf = class {
  constructor(e, t, n) {
    this.service = e, this.uri = t, this.dependencies = /* @__PURE__ */ new Set(), this.anchors = void 0, n && (this.unresolvedSchema = this.service.promise.resolve(new an(n)));
  }
  getUnresolvedSchema() {
    return this.unresolvedSchema || (this.unresolvedSchema = this.service.loadSchema(this.uri)), this.unresolvedSchema;
  }
  getResolvedSchema() {
    return this.resolvedSchema || (this.resolvedSchema = this.getUnresolvedSchema().then((e) => this.service.resolveSchemaContent(e, this))), this.resolvedSchema;
  }
  clearSchema() {
    const e = !!this.unresolvedSchema;
    return this.resolvedSchema = void 0, this.unresolvedSchema = void 0, this.dependencies.clear(), this.anchors = void 0, e;
  }
}, an = class {
  constructor(e, t = []) {
    this.schema = e, this.errors = t;
  }
}, Zo = class {
  constructor(e, t = [], n = [], i) {
    this.schema = e, this.errors = t, this.warnings = n, this.schemaDraft = i;
  }
  getSection(e) {
    const t = this.getSectionRecursive(e, this.schema);
    if (t)
      return Re(t);
  }
  getSectionRecursive(e, t) {
    if (!t || typeof t == "boolean" || e.length === 0)
      return t;
    const n = e.shift();
    if (t.properties && typeof t.properties[n])
      return this.getSectionRecursive(e, t.properties[n]);
    if (t.patternProperties)
      for (const i of Object.keys(t.patternProperties)) {
        const r = Cn(i);
        if (r != null && r.test(n))
          return this.getSectionRecursive(e, t.patternProperties[i]);
      }
    else {
      if (typeof t.additionalProperties == "object")
        return this.getSectionRecursive(e, t.additionalProperties);
      if (n.match("[0-9]+")) {
        if (Array.isArray(t.items)) {
          const i = parseInt(n, 10);
          if (!isNaN(i) && t.items[i])
            return this.getSectionRecursive(e, t.items[i]);
        } else if (t.items)
          return this.getSectionRecursive(e, t.items);
      }
    }
  }
}, mf = class {
  constructor(e, t, n) {
    this.contextService = t, this.requestService = e, this.promiseConstructor = n || Promise, this.callOnDispose = [], this.contributionSchemas = {}, this.contributionAssociations = [], this.schemasById = {}, this.filePatternAssociations = [], this.registeredSchemasIds = {};
  }
  getRegisteredSchemaIds(e) {
    return Object.keys(this.registeredSchemasIds).filter((t) => {
      const n = Kt.parse(t).scheme;
      return n !== "schemaservice" && (!e || e(n));
    });
  }
  get promise() {
    return this.promiseConstructor;
  }
  dispose() {
    for (; this.callOnDispose.length > 0; )
      this.callOnDispose.pop()();
  }
  onResourceChange(e) {
    this.cachedSchemaForResource = void 0;
    let t = !1;
    e = lt(e);
    const n = [e], i = Object.keys(this.schemasById).map((r) => this.schemasById[r]);
    for (; n.length; ) {
      const r = n.pop();
      for (let s = 0; s < i.length; s++) {
        const a = i[s];
        a && (a.uri === r || a.dependencies.has(r)) && (a.uri !== r && n.push(a.uri), a.clearSchema() && (t = !0), i[s] = void 0);
      }
    }
    return t;
  }
  setSchemaContributions(e) {
    if (e.schemas) {
      const t = e.schemas;
      for (const n in t) {
        const i = lt(n);
        this.contributionSchemas[i] = this.addSchemaHandle(i, t[n]);
      }
    }
    if (Array.isArray(e.schemaAssociations)) {
      const t = e.schemaAssociations;
      for (let n of t) {
        const i = n.uris.map(lt), r = this.addFilePatternAssociation(n.pattern, n.folderUri, i);
        this.contributionAssociations.push(r);
      }
    }
  }
  addSchemaHandle(e, t) {
    const n = new hf(this, e, t);
    return this.schemasById[e] = n, n;
  }
  getOrAddSchemaHandle(e, t) {
    return this.schemasById[e] || this.addSchemaHandle(e, t);
  }
  addFilePatternAssociation(e, t, n) {
    const i = new ff(e, t, n);
    return this.filePatternAssociations.push(i), i;
  }
  registerExternalSchema(e) {
    const t = lt(e.uri);
    return this.registeredSchemasIds[t] = !0, this.cachedSchemaForResource = void 0, e.fileMatch && e.fileMatch.length && this.addFilePatternAssociation(e.fileMatch, e.folderUri, [t]), e.schema ? this.addSchemaHandle(t, e.schema) : this.getOrAddSchemaHandle(t);
  }
  clearExternalSchemas() {
    this.schemasById = {}, this.filePatternAssociations = [], this.registeredSchemasIds = {}, this.cachedSchemaForResource = void 0;
    for (const e in this.contributionSchemas)
      this.schemasById[e] = this.contributionSchemas[e], this.registeredSchemasIds[e] = !0;
    for (const e of this.contributionAssociations)
      this.filePatternAssociations.push(e);
  }
  getResolvedSchema(e) {
    const t = lt(e), n = this.schemasById[t];
    return n ? n.getResolvedSchema() : this.promise.resolve(void 0);
  }
  loadSchema(e) {
    if (!this.requestService) {
      const t = M("Unable to load schema from '{0}'. No schema request service available", sn(e));
      return this.promise.resolve(new an({}, [t]));
    }
    return e.startsWith("http://json-schema.org/") && (e = "https" + e.substring(4)), this.requestService(e).then((t) => {
      if (!t) {
        const s = M("Unable to load schema from '{0}': No content.", sn(e));
        return new an({}, [s]);
      }
      const n = [];
      t.charCodeAt(0) === 65279 && (n.push(M("Problem reading content from '{0}': UTF-8 with BOM detected, only UTF 8 is allowed.", sn(e))), t = t.trimStart());
      let i = {};
      const r = [];
      return i = S1(t, r), r.length && n.push(M("Unable to parse content from '{0}': Parse error at offset {1}.", sn(e), r[0].offset)), new an(i, n);
    }, (t) => {
      let n = t.toString();
      const i = t.toString().split("Error: ");
      return i.length > 1 && (n = i[1]), wn(n, ".") && (n = n.substr(0, n.length - 1)), new an({}, [M("Unable to load schema from '{0}': {1}.", sn(e), n)]);
    });
  }
  resolveSchemaContent(e, t) {
    const n = e.errors.slice(0), i = e.schema;
    let r = i.$schema ? lt(i.$schema) : void 0;
    if (r === "http://json-schema.org/draft-03/schema")
      return this.promise.resolve(new Zo({}, [M("Draft-03 schemas are not supported.")], [], r));
    let s = /* @__PURE__ */ new Set();
    const a = this.contextService, l = (d, p) => {
      p = decodeURIComponent(p);
      let _ = d;
      return p[0] === "/" && (p = p.substring(1)), p.split("/").some((x) => (x = x.replace(/~1/g, "/").replace(/~0/g, "~"), _ = _[x], !_)), _;
    }, o = (d, p, _) => (p.anchors || (p.anchors = g(d)), p.anchors.get(_)), u = (d, p) => {
      for (const _ in p)
        p.hasOwnProperty(_) && _ !== "id" && _ !== "$id" && (d[_] = p[_]);
    }, h = (d, p, _, x) => {
      let R;
      x === void 0 || x.length === 0 ? R = p : x.charAt(0) === "/" ? R = l(p, x) : R = o(p, _, x), R ? u(d, R) : n.push(M("$ref '{0}' in '{1}' can not be resolved.", x || "", _.uri));
    }, c = (d, p, _, x) => {
      a && !/^[A-Za-z][A-Za-z0-9+\-.+]*:\/\/.*/.test(p) && (p = a.resolveRelativePath(p, x.uri)), p = lt(p);
      const R = this.getOrAddSchemaHandle(p);
      return R.getUnresolvedSchema().then((v) => {
        if (x.dependencies.add(p), v.errors.length) {
          const L = _ ? p + "#" + _ : p;
          n.push(M("Problems loading reference '{0}': {1}", L, v.errors[0]));
        }
        return h(d, v.schema, R, _), m(d, v.schema, R);
      });
    }, m = (d, p, _) => {
      const x = [];
      return this.traverseNodes(d, (R) => {
        const v = /* @__PURE__ */ new Set();
        for (; R.$ref; ) {
          const L = R.$ref, b = L.split("#", 2);
          if (delete R.$ref, b[0].length > 0) {
            x.push(c(R, b[0], b[1], _));
            return;
          } else if (!v.has(L)) {
            const E = b[1];
            h(R, p, _, E), v.add(L);
          }
        }
        R.$recursiveRef && s.add("$recursiveRef"), R.$dynamicRef && s.add("$dynamicRef");
      }), this.promise.all(x);
    }, g = (d) => {
      const p = /* @__PURE__ */ new Map();
      return this.traverseNodes(d, (_) => {
        const x = _.$id || _.id, R = kl(x) && x.charAt(0) === "#" ? x.substring(1) : _.$anchor;
        R && (p.has(R) ? n.push(M("Duplicate anchor declaration: '{0}'", R)) : p.set(R, _)), _.$recursiveAnchor && s.add("$recursiveAnchor"), _.$dynamicAnchor && s.add("$dynamicAnchor");
      }), p;
    };
    return m(i, i, t).then((d) => {
      let p = [];
      return s.size && p.push(M("The schema uses meta-schema features ({0}) that are not yet supported by the validator.", Array.from(s.keys()).join(", "))), new Zo(i, n, p, r);
    });
  }
  traverseNodes(e, t) {
    if (!e || typeof e != "object")
      return Promise.resolve(null);
    const n = /* @__PURE__ */ new Set(), i = (...u) => {
      for (const h of u)
        ft(h) && l.push(h);
    }, r = (...u) => {
      for (const h of u)
        if (ft(h))
          for (const c in h) {
            const g = h[c];
            ft(g) && l.push(g);
          }
    }, s = (...u) => {
      for (const h of u)
        if (Array.isArray(h))
          for (const c of h)
            ft(c) && l.push(c);
    }, a = (u) => {
      if (Array.isArray(u))
        for (const h of u)
          ft(h) && l.push(h);
      else ft(u) && l.push(u);
    }, l = [e];
    let o = l.pop();
    for (; o; )
      n.has(o) || (n.add(o), t(o), i(o.additionalItems, o.additionalProperties, o.not, o.contains, o.propertyNames, o.if, o.then, o.else, o.unevaluatedItems, o.unevaluatedProperties), r(o.definitions, o.$defs, o.properties, o.patternProperties, o.dependencies, o.dependentSchemas), s(o.anyOf, o.allOf, o.oneOf, o.prefixItems), a(o.items)), o = l.pop();
  }
  getSchemaFromProperty(e, t) {
    var n, i;
    if (((n = t.root) == null ? void 0 : n.type) === "object") {
      for (const r of t.root.properties)
        if (r.keyNode.value === "$schema" && ((i = r.valueNode) == null ? void 0 : i.type) === "string") {
          let s = r.valueNode.value;
          return this.contextService && !/^\w[\w\d+.-]*:/.test(s) && (s = this.contextService.resolveRelativePath(s, e)), s;
        }
    }
  }
  getAssociatedSchemas(e) {
    const t = /* @__PURE__ */ Object.create(null), n = [], i = Pl(e);
    for (const r of this.filePatternAssociations)
      if (r.matchesPattern(i))
        for (const s of r.getURIs())
          t[s] || (n.push(s), t[s] = !0);
    return n;
  }
  getSchemaURIsForResource(e, t) {
    let n = t && this.getSchemaFromProperty(e, t);
    return n ? [n] : this.getAssociatedSchemas(e);
  }
  getSchemaForResource(e, t) {
    if (t) {
      let r = this.getSchemaFromProperty(e, t);
      if (r) {
        const s = lt(r);
        return this.getOrAddSchemaHandle(s).getResolvedSchema();
      }
    }
    if (this.cachedSchemaForResource && this.cachedSchemaForResource.resource === e)
      return this.cachedSchemaForResource.resolvedSchema;
    const n = this.getAssociatedSchemas(e), i = n.length > 0 ? this.createCombinedSchema(e, n).getResolvedSchema() : this.promise.resolve(void 0);
    return this.cachedSchemaForResource = { resource: e, resolvedSchema: i }, i;
  }
  createCombinedSchema(e, t) {
    if (t.length === 1)
      return this.getOrAddSchemaHandle(t[0]);
    {
      const n = "schemaservice://combinedSchema/" + encodeURIComponent(e), i = {
        allOf: t.map((r) => ({ $ref: r }))
      };
      return this.addSchemaHandle(n, i);
    }
  }
  getMatchingSchemas(e, t, n) {
    if (n) {
      const i = n.id || "schemaservice://untitled/matchingSchemas/" + gf++;
      return this.addSchemaHandle(i, n).getResolvedSchema().then((s) => t.getMatchingSchemas(s.schema).filter((a) => !a.inverted));
    }
    return this.getSchemaForResource(e.uri, t).then((i) => i ? t.getMatchingSchemas(i.schema).filter((r) => !r.inverted) : []);
  }
}, gf = 0;
function lt(e) {
  try {
    return Kt.parse(e).toString(!0);
  } catch {
    return e;
  }
}
function Pl(e) {
  try {
    return Kt.parse(e).with({ fragment: null, query: null }).toString(!0);
  } catch {
    return e;
  }
}
function sn(e) {
  try {
    const t = Kt.parse(e);
    if (t.scheme === "file")
      return t.fsPath;
  } catch {
  }
  return e;
}
function df(e, t) {
  const n = [], i = [], r = [];
  let s = -1;
  const a = At(e.getText(), !1);
  let l = a.scan();
  function o(d) {
    n.push(d), i.push(r.length);
  }
  for (; l !== 17; ) {
    switch (l) {
      case 1:
      case 3: {
        const d = e.positionAt(a.getTokenOffset()).line, p = { startLine: d, endLine: d, kind: l === 1 ? "object" : "array" };
        r.push(p);
        break;
      }
      case 2:
      case 4: {
        const d = l === 2 ? "object" : "array";
        if (r.length > 0 && r[r.length - 1].kind === d) {
          const p = r.pop(), _ = e.positionAt(a.getTokenOffset()).line;
          p && _ > p.startLine + 1 && s !== p.startLine && (p.endLine = _ - 1, o(p), s = p.startLine);
        }
        break;
      }
      case 13: {
        const d = e.positionAt(a.getTokenOffset()).line, p = e.positionAt(a.getTokenOffset() + a.getTokenLength()).line;
        a.getTokenError() === 1 && d + 1 < e.lineCount ? a.setPosition(e.offsetAt(oe.create(d + 1, 0))) : d < p && (o({ startLine: d, endLine: p, kind: mn.Comment }), s = d);
        break;
      }
      case 12: {
        const p = e.getText().substr(a.getTokenOffset(), a.getTokenLength()).match(/^\/\/\s*#(region\b)|(endregion\b)/);
        if (p) {
          const _ = e.positionAt(a.getTokenOffset()).line;
          if (p[1]) {
            const x = { startLine: _, endLine: _, kind: mn.Region };
            r.push(x);
          } else {
            let x = r.length - 1;
            for (; x >= 0 && r[x].kind !== mn.Region; )
              x--;
            if (x >= 0) {
              const R = r[x];
              r.length = x, _ > R.startLine && s !== R.startLine && (R.endLine = _, o(R), s = R.startLine);
            }
          }
        }
        break;
      }
    }
    l = a.scan();
  }
  const u = t && t.rangeLimit;
  if (typeof u != "number" || n.length <= u)
    return n;
  t && t.onRangeLimitExceeded && t.onRangeLimitExceeded(e.uri);
  const h = [];
  for (let d of i)
    d < 30 && (h[d] = (h[d] || 0) + 1);
  let c = 0, m = 0;
  for (let d = 0; d < h.length; d++) {
    const p = h[d];
    if (p) {
      if (p + c > u) {
        m = d;
        break;
      }
      c += p;
    }
  }
  const g = [];
  for (let d = 0; d < n.length; d++) {
    const p = i[d];
    typeof p == "number" && (p < m || p === m && c++ < u) && g.push(n[d]);
  }
  return g;
}
function pf(e, t, n) {
  function i(l) {
    let o = e.offsetAt(l), u = n.getNodeFromOffset(o, !0);
    const h = [];
    for (; u; ) {
      switch (u.type) {
        case "string":
        case "object":
        case "array":
          const m = u.offset + 1, g = u.offset + u.length - 1;
          m < g && o >= m && o <= g && h.push(r(m, g)), h.push(r(u.offset, u.offset + u.length));
          break;
        case "number":
        case "boolean":
        case "null":
        case "property":
          h.push(r(u.offset, u.offset + u.length));
          break;
      }
      if (u.type === "property" || u.parent && u.parent.type === "array") {
        const m = a(
          u.offset + u.length,
          5
          /* SyntaxKind.CommaToken */
        );
        m !== -1 && h.push(r(u.offset, m));
      }
      u = u.parent;
    }
    let c;
    for (let m = h.length - 1; m >= 0; m--)
      c = ii.create(h[m], c);
    return c || (c = ii.create(X.create(l, l))), c;
  }
  function r(l, o) {
    return X.create(e.positionAt(l), e.positionAt(o));
  }
  const s = At(e.getText(), !0);
  function a(l, o) {
    return s.setPosition(l), s.scan() === o ? s.getTokenOffset() + s.getTokenLength() : -1;
  }
  return t.map(i);
}
function dr(e, t, n) {
  let i;
  if (n) {
    const s = e.offsetAt(n.start), a = e.offsetAt(n.end) - s;
    i = { offset: s, length: a };
  }
  const r = {
    tabSize: t ? t.tabSize : 4,
    insertSpaces: (t == null ? void 0 : t.insertSpaces) === !0,
    insertFinalNewline: (t == null ? void 0 : t.insertFinalNewline) === !0,
    eol: `
`,
    keepLines: (t == null ? void 0 : t.keepLines) === !0
  };
  return U1(e.getText(), i, r).map((s) => Ze.replace(X.create(e.positionAt(s.offset), e.positionAt(s.offset + s.length)), s.content));
}
var me;
(function(e) {
  e[e.Object = 0] = "Object", e[e.Array = 1] = "Array";
})(me || (me = {}));
var Mn = class {
  constructor(e, t) {
    this.propertyName = e ?? "", this.beginningLineNumber = t, this.childrenProperties = [], this.lastProperty = !1, this.noKeyName = !1;
  }
  addChildProperty(e) {
    if (e.parent = this, this.childrenProperties.length > 0) {
      let t = 0;
      e.noKeyName ? t = this.childrenProperties.length : t = _f(this.childrenProperties, e, bf), t < 0 && (t = t * -1 - 1), this.childrenProperties.splice(t, 0, e);
    } else
      this.childrenProperties.push(e);
    return e;
  }
};
function bf(e, t) {
  const n = e.propertyName.toLowerCase(), i = t.propertyName.toLowerCase();
  return n < i ? -1 : n > i ? 1 : 0;
}
function _f(e, t, n) {
  const i = t.propertyName.toLowerCase(), r = e[0].propertyName.toLowerCase(), s = e[e.length - 1].propertyName.toLowerCase();
  if (i < r)
    return 0;
  if (i > s)
    return e.length;
  let a = 0, l = e.length - 1;
  for (; a <= l; ) {
    let o = l + a >> 1, u = n(t, e[o]);
    if (u > 0)
      a = o + 1;
    else if (u < 0)
      l = o - 1;
    else
      return o;
  }
  return -a - 1;
}
function vf(e, t) {
  const n = {
    ...t,
    keepLines: !1
    // keepLines must be false so that the properties are on separate lines for the sorting
  }, i = Ge.applyEdits(e, dr(e, n, void 0)), r = Ge.create("test://test.json", "json", 0, i), s = Lf(r), a = Nf(r, s), l = dr(a, n, void 0), o = Ge.applyEdits(a, l);
  return [Ze.replace(X.create(oe.create(0, 0), e.positionAt(e.getText().length)), o)];
}
function Lf(e) {
  const t = e.getText(), n = At(t, !1);
  let i = new Mn(), r = i, s = i, a = i, l, o = 0, u = 0, h, c, m = -1, g = -1, d = 0, p = 0, _ = [], x = !1, R = !1;
  for (; (l = n.scan()) !== 17; ) {
    if (x === !0 && l !== 14 && l !== 15 && l !== 12 && l !== 13 && s.endLineNumber === void 0) {
      let v = n.getTokenStartLine();
      c === 2 || c === 4 ? a.endLineNumber = v - 1 : s.endLineNumber = v - 1, d = v, x = !1;
    }
    if (R === !0 && l !== 14 && l !== 15 && l !== 12 && l !== 13 && (d = n.getTokenStartLine(), R = !1), n.getTokenStartLine() !== o) {
      for (let v = o; v < n.getTokenStartLine(); v++) {
        const L = e.getText(X.create(oe.create(v, 0), oe.create(v + 1, 0))).length;
        u = u + L;
      }
      o = n.getTokenStartLine();
    }
    switch (l) {
      case 10: {
        if (h === void 0 || h === 1 || h === 5 && _[_.length - 1] === me.Object) {
          const v = new Mn(n.getTokenValue(), d);
          a = s, s = r.addChildProperty(v);
        }
        break;
      }
      case 3: {
        if (i.beginningLineNumber === void 0 && (i.beginningLineNumber = n.getTokenStartLine()), _[_.length - 1] === me.Object)
          r = s;
        else if (_[_.length - 1] === me.Array) {
          const v = new Mn(n.getTokenValue(), d);
          v.noKeyName = !0, a = s, s = r.addChildProperty(v), r = s;
        }
        _.push(me.Array), s.type = me.Array, d = n.getTokenStartLine(), d++;
        break;
      }
      case 1: {
        if (i.beginningLineNumber === void 0)
          i.beginningLineNumber = n.getTokenStartLine();
        else if (_[_.length - 1] === me.Array) {
          const v = new Mn(n.getTokenValue(), d);
          v.noKeyName = !0, a = s, s = r.addChildProperty(v);
        }
        s.type = me.Object, _.push(me.Object), r = s, d = n.getTokenStartLine(), d++;
        break;
      }
      case 4: {
        p = n.getTokenStartLine(), _.pop(), s.endLineNumber === void 0 && (h === 2 || h === 4) && (s.endLineNumber = p - 1, s.lastProperty = !0, s.lineWhereToAddComma = m, s.indexWhereToAddComa = g, a = s, s = s ? s.parent : void 0, r = s), i.endLineNumber = p, d = p + 1;
        break;
      }
      case 2: {
        p = n.getTokenStartLine(), _.pop(), h !== 1 && (s.endLineNumber === void 0 && (s.endLineNumber = p - 1, s.lastProperty = !0, s.lineWhereToAddComma = m, s.indexWhereToAddComa = g), a = s, s = s ? s.parent : void 0, r = s), i.endLineNumber = n.getTokenStartLine(), d = p + 1;
        break;
      }
      case 5: {
        p = n.getTokenStartLine(), s.endLineNumber === void 0 && (_[_.length - 1] === me.Object || _[_.length - 1] === me.Array && (h === 2 || h === 4)) && (s.endLineNumber = p, s.commaIndex = n.getTokenOffset() - u, s.commaLine = p), (h === 2 || h === 4) && (a = s, s = s ? s.parent : void 0, r = s), d = p + 1;
        break;
      }
      case 13: {
        h === 5 && m === n.getTokenStartLine() && (_[_.length - 1] === me.Array && (c === 2 || c === 4) || _[_.length - 1] === me.Object) && (_[_.length - 1] === me.Array && (c === 2 || c === 4) || _[_.length - 1] === me.Object) && (s.endLineNumber = void 0, x = !0), (h === 1 || h === 3) && m === n.getTokenStartLine() && (R = !0);
        break;
      }
    }
    l !== 14 && l !== 13 && l !== 12 && l !== 15 && (c = h, h = l, m = n.getTokenStartLine(), g = n.getTokenOffset() + n.getTokenLength() - u);
  }
  return i;
}
function Nf(e, t) {
  if (t.childrenProperties.length === 0)
    return e;
  const n = Ge.create("test://test.json", "json", 0, e.getText()), i = [];
  for (Ko(i, t, t.beginningLineNumber); i.length > 0; ) {
    const r = i.shift(), s = r.propertyTreeArray;
    let a = r.beginningLineNumber;
    for (let l = 0; l < s.length; l++) {
      const o = s[l], u = X.create(oe.create(o.beginningLineNumber, 0), oe.create(o.endLineNumber + 1, 0)), h = e.getText(u), c = Ge.create("test://test.json", "json", 0, h);
      if (o.lastProperty === !0 && l !== s.length - 1) {
        const d = o.lineWhereToAddComma - o.beginningLineNumber, p = o.indexWhereToAddComa, _ = {
          range: X.create(oe.create(d, p), oe.create(d, p)),
          text: ","
        };
        Ge.update(c, [_], 1);
      } else if (o.lastProperty === !1 && l === s.length - 1) {
        const d = o.commaIndex, _ = o.commaLine - o.beginningLineNumber, x = {
          range: X.create(oe.create(_, d), oe.create(_, d + 1)),
          text: ""
        };
        Ge.update(c, [x], 1);
      }
      const m = o.endLineNumber - o.beginningLineNumber + 1, g = {
        range: X.create(oe.create(a, 0), oe.create(a + m, 0)),
        text: c.getText()
      };
      Ge.update(n, [g], 1), Ko(i, o, a), a = a + m;
    }
  }
  return n;
}
function Ko(e, t, n) {
  if (t.childrenProperties.length !== 0)
    if (t.type === me.Object) {
      let i = 1 / 0;
      for (const s of t.childrenProperties)
        s.beginningLineNumber < i && (i = s.beginningLineNumber);
      const r = i - t.beginningLineNumber;
      n = n + r, e.push(new Bl(n, t.childrenProperties));
    } else t.type === me.Array && Ol(e, t, n);
}
function Ol(e, t, n) {
  for (const i of t.childrenProperties) {
    if (i.type === me.Object) {
      let r = 1 / 0;
      for (const a of i.childrenProperties)
        a.beginningLineNumber < r && (r = a.beginningLineNumber);
      const s = r - i.beginningLineNumber;
      e.push(new Bl(n + i.beginningLineNumber - t.beginningLineNumber + s, i.childrenProperties));
    }
    i.type === me.Array && Ol(e, i, n + i.beginningLineNumber - t.beginningLineNumber);
  }
}
var Bl = class {
  constructor(e, t) {
    this.beginningLineNumber = e, this.propertyTreeArray = t;
  }
};
function wf(e, t) {
  const n = [];
  return t.visit((i) => {
    var r;
    if (i.type === "property" && i.keyNode.value === "$ref" && ((r = i.valueNode) == null ? void 0 : r.type) === "string") {
      const s = i.valueNode.value, a = xf(t, s);
      if (a) {
        const l = e.positionAt(a.offset);
        n.push({
          target: `${e.uri}#${l.line + 1},${l.character + 1}`,
          range: Af(e, i.valueNode)
        });
      }
    }
    return !0;
  }), Promise.resolve(n);
}
function Af(e, t) {
  return X.create(e.positionAt(t.offset + 1), e.positionAt(t.offset + t.length - 1));
}
function xf(e, t) {
  const n = Ef(t);
  return n ? pr(n, e.root) : null;
}
function pr(e, t) {
  if (!t)
    return null;
  if (e.length === 0)
    return t;
  const n = e.shift();
  if (t && t.type === "object") {
    const i = t.properties.find((r) => r.keyNode.value === n);
    return i ? pr(e, i.valueNode) : null;
  } else if (t && t.type === "array" && n.match(/^(0|[1-9][0-9]*)$/)) {
    const i = Number.parseInt(n), r = t.items[i];
    return r ? pr(e, r) : null;
  }
  return null;
}
function Ef(e) {
  return e === "#" ? [] : e[0] !== "#" || e[1] !== "/" ? null : e.substring(2).split(/\//).map(yf);
}
function yf(e) {
  return e.replace(/~1/g, "/").replace(/~0/g, "~");
}
function Rf(e) {
  const t = e.promiseConstructor || Promise, n = new mf(e.schemaRequestService, e.workspaceContext, t);
  n.setSchemaContributions(gr);
  const i = new Q1(n, e.contributions, t, e.clientCapabilities), r = new Z1(n, e.contributions, t), s = new af(n), a = new C1(n, t);
  return {
    configure: (l) => {
      var o;
      n.clearExternalSchemas(), (o = l.schemas) == null || o.forEach(n.registerExternalSchema.bind(n)), a.configure(l);
    },
    resetSchema: (l) => n.onResourceChange(l),
    doValidation: a.doValidation.bind(a),
    getLanguageStatus: a.getLanguageStatus.bind(a),
    parseJSONDocument: (l) => Y1(l, { collectComments: !0 }),
    newJSONDocument: (l, o) => J1(l, o),
    getMatchingSchemas: n.getMatchingSchemas.bind(n),
    doResolve: i.doResolve.bind(i),
    doComplete: i.doComplete.bind(i),
    findDocumentSymbols: s.findDocumentSymbols.bind(s),
    findDocumentSymbols2: s.findDocumentSymbols2.bind(s),
    findDocumentColors: s.findDocumentColors.bind(s),
    getColorPresentations: s.getColorPresentations.bind(s),
    doHover: r.doHover.bind(r),
    getFoldingRanges: df,
    getSelectionRanges: pf,
    findDefinition: () => Promise.resolve([]),
    findLinks: wf,
    format: (l, o, u) => dr(l, u, o),
    sort: (l, o) => vf(l, o)
  };
}
var Vl;
typeof fetch < "u" && (Vl = function(e) {
  return fetch(e).then((t) => t.text());
});
var kf = class {
  constructor(e, t) {
    this._ctx = e, this._languageSettings = t.languageSettings, this._languageId = t.languageId, this._languageService = Rf({
      workspaceContext: {
        resolveRelativePath: (n, i) => {
          const r = i.substr(0, i.lastIndexOf("/") + 1);
          return Mf(r, n);
        }
      },
      schemaRequestService: t.enableSchemaRequest ? Vl : void 0,
      clientCapabilities: fr.LATEST
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
    let i = this._languageService.parseJSONDocument(n);
    return this._languageService.doComplete(n, t, i);
  }
  async doResolve(e) {
    return this._languageService.doResolve(e);
  }
  async doHover(e, t) {
    let n = this._getTextDocument(e);
    if (!n)
      return null;
    let i = this._languageService.parseJSONDocument(n);
    return this._languageService.doHover(n, t, i);
  }
  async format(e, t, n) {
    let i = this._getTextDocument(e);
    if (!i)
      return [];
    let r = this._languageService.format(i, t, n);
    return Promise.resolve(r);
  }
  async resetSchema(e) {
    return Promise.resolve(this._languageService.resetSchema(e));
  }
  async findDocumentSymbols(e) {
    let t = this._getTextDocument(e);
    if (!t)
      return [];
    let n = this._languageService.parseJSONDocument(t), i = this._languageService.findDocumentSymbols2(t, n);
    return Promise.resolve(i);
  }
  async findDocumentColors(e) {
    let t = this._getTextDocument(e);
    if (!t)
      return [];
    let n = this._languageService.parseJSONDocument(t), i = this._languageService.findDocumentColors(t, n);
    return Promise.resolve(i);
  }
  async getColorPresentations(e, t, n) {
    let i = this._getTextDocument(e);
    if (!i)
      return [];
    let r = this._languageService.parseJSONDocument(i), s = this._languageService.getColorPresentations(
      i,
      r,
      t,
      n
    );
    return Promise.resolve(s);
  }
  async getFoldingRanges(e, t) {
    let n = this._getTextDocument(e);
    if (!n)
      return [];
    let i = this._languageService.getFoldingRanges(n, t);
    return Promise.resolve(i);
  }
  async getSelectionRanges(e, t) {
    let n = this._getTextDocument(e);
    if (!n)
      return [];
    let i = this._languageService.parseJSONDocument(n), r = this._languageService.getSelectionRanges(n, t, i);
    return Promise.resolve(r);
  }
  async parseJSONDocument(e) {
    let t = this._getTextDocument(e);
    if (!t)
      return null;
    let n = this._languageService.parseJSONDocument(t);
    return Promise.resolve(n);
  }
  async getMatchingSchemas(e) {
    let t = this._getTextDocument(e);
    if (!t)
      return [];
    let n = this._languageService.parseJSONDocument(t);
    return Promise.resolve(this._languageService.getMatchingSchemas(t, n));
  }
  _getTextDocument(e) {
    let t = this._ctx.getMirrorModels();
    for (let n of t)
      if (n.uri.toString() === e)
        return Ge.create(
          e,
          this._languageId,
          n.version,
          n.getValue()
        );
    return null;
  }
}, Tf = 47, wi = 46;
function Sf(e) {
  return e.charCodeAt(0) === Tf;
}
function Mf(e, t) {
  if (Sf(t)) {
    const n = Kt.parse(e), i = t.split("/");
    return n.with({ path: ql(i) }).toString();
  }
  return If(e, t);
}
function ql(e) {
  const t = [];
  for (const i of e)
    i.length === 0 || i.length === 1 && i.charCodeAt(0) === wi || (i.length === 2 && i.charCodeAt(0) === wi && i.charCodeAt(1) === wi ? t.pop() : t.push(i));
  e.length > 1 && e[e.length - 1].length === 0 && t.push("");
  let n = t.join("/");
  return e[0].length === 0 && (n = "/" + n), n;
}
function If(e, ...t) {
  const n = Kt.parse(e), i = n.path.split("/");
  for (let r of t)
    i.push(...r.split("/"));
  return n.with({ path: ql(i) }).toString();
}
self.onmessage = () => {
  El((e, t) => new kf(e, t));
};
