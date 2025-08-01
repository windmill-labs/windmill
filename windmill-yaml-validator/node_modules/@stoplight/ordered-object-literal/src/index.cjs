'use strict';

Object.defineProperty(exports, '__esModule', { value: true });

const TIMESTAMP = Math.floor(Date.now() / 3600000); // 1-day should be sufficient in most circumstances

const ORDER_KEY_ID = `__object_order_${TIMESTAMP}__`;

const ORDER_KEY = Symbol.for(ORDER_KEY_ID);
const STRINGIFIED_ORDER_KEY = String(ORDER_KEY);

const traps = {
  defineProperty(target, key, descriptor) {
    const hasKey = Object.prototype.hasOwnProperty.call(target, key);
    if (!hasKey && ORDER_KEY in target) {
      target[ORDER_KEY].push(key);
    } else if (
      'value' in descriptor &&
      key === ORDER_KEY &&
      descriptor.value.lastIndexOf(ORDER_KEY) === -1
    ) {
      descriptor.value.push(ORDER_KEY);
    }

    return Reflect.defineProperty(target, key, descriptor);
  },

  deleteProperty(target, key) {
    const hasKey = Object.prototype.hasOwnProperty.call(target, key);
    const deleted = Reflect.deleteProperty(target, key);

    if (deleted && hasKey && ORDER_KEY in target) {
      const index = target[ORDER_KEY].indexOf(key);
      if (index !== -1) {
        target[ORDER_KEY].splice(index, 1);
      }
    }

    return deleted;
  },

  ownKeys(target) {
    if (ORDER_KEY in target) {
      return target[ORDER_KEY];
    }

    return Reflect.ownKeys(target);
  },

  set(target, key, value) {
    const hasKey = Object.prototype.hasOwnProperty.call(target, key);
    const set = Reflect.set(target, key, value);

    if (set && !hasKey && ORDER_KEY in target) {
      target[ORDER_KEY].push(key);
    }

    return set;
  },
};

function createObj(target, order = Reflect.ownKeys(target)) {
  assertObjectLiteral(target);
  const t = new Proxy(target, traps);
  setOrder(t, order);
  return t;
}

function setOrder(target, order) {
  if (ORDER_KEY in target) {
    target[ORDER_KEY].length = 0;
    target[ORDER_KEY].push(...order);
    return true;
  } else {
    return Reflect.defineProperty(target, ORDER_KEY, {
      configurable: true,
      value: order,
    });
  }
}

function getOrder(target) {
  return target[ORDER_KEY];
}

function serializeArray(target) {
  const newTarget = target.slice();

  for (let i = 0; i < newTarget.length; i += 1) {
    const value = newTarget[i];
    if (isObject(value)) {
      newTarget[i] = Array.isArray(value)
        ? serializeArray(value)
        : serialize(value, true);
    }
  }

  return newTarget;
}

function serialize(target, deep) {
  assertObjectLiteral(target, 'Invalid target provided');

  const newTarget = { ...target };

  if (ORDER_KEY in target) {
    Object.defineProperty(newTarget, STRINGIFIED_ORDER_KEY, {
      enumerable: true,
      value: target[ORDER_KEY].filter((item) => item !== ORDER_KEY),
    });
  }

  if (deep) {
    for (const key of Object.keys(target)) {
      if (key === STRINGIFIED_ORDER_KEY) continue;
      const value = target[key];
      if (isObject(value)) {
        newTarget[key] = Array.isArray(value)
          ? serializeArray(value)
          : serialize(value, true);
      }
    }
  }

  return newTarget;
}

function deserializeArray(target) {
  for (let i = 0; i < target.length; i += 1) {
    const value = target[i];
    if (isObject(value)) {
      target[i] = Array.isArray(value)
        ? deserializeArray(value)
        : deserialize(value, true);
    }
  }

  return target;
}

function deserialize(target, deep) {
  assertObjectLiteral(target, 'Invalid target provided');

  const newTarget = createObj(
    target,
    STRINGIFIED_ORDER_KEY in target
      ? target[STRINGIFIED_ORDER_KEY]
      : Reflect.ownKeys(target),
  );

  delete newTarget[STRINGIFIED_ORDER_KEY];

  if (deep) {
    for (const key of Object.keys(target)) {
      const value = target[key];
      if (isObject(value)) {
        target[key] = Array.isArray(value)
          ? deserializeArray(value)
          : deserialize(value, true);
      }
    }
  }

  return newTarget;
}

function isOrderedObject(target) {
  return ORDER_KEY in target;
}

function isObject(maybeObj) {
  return maybeObj !== null && typeof maybeObj === 'object';
}

function isObjectLiteral(obj) {
  if (!isObject(obj)) return false;
  if (obj[Symbol.toStringTag] !== void 0) {
    const proto = Object.getPrototypeOf(obj);
    return proto === null || proto === Object.prototype;
  }

  return toStringTag(obj) === 'Object';
}

function toStringTag(obj) {
  const tag = obj[Symbol.toStringTag];
  if (typeof tag === 'string') {
    return tag;
  }

  const name = Reflect.apply(Object.prototype.toString, obj, []);
  return name.slice(8, name.length - 1);
}

function assertObjectLiteral(maybeObj, message) {
  if (isDevEnv() && !isObjectLiteral(maybeObj)) {
    throw new TypeError(message);
  }
}

function isDevEnv() {
  if (
    typeof process === 'undefined' ||
    !isObject(process) ||
    !isObject(process.env)
  ) {
    return false;
  }

  return (
    process.env.NODE_ENV === 'development' || process.env.NODE_ENV === 'test'
  );
}

exports.ORDER_KEY_ID = ORDER_KEY_ID;
exports.default = createObj;
exports.deserialize = deserialize;
exports.getOrder = getOrder;
exports.isOrderedObject = isOrderedObject;
exports.serialize = serialize;
exports.setOrder = setOrder;
