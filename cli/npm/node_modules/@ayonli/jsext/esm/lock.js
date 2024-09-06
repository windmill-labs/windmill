import BiMap from './collections/BiMap.js';

/**
 * A mutual exclusion mechanism for concurrent operations and protecting shared
 * data.
 * @module
 */
var _a;
if (typeof Symbol.dispose === "undefined") {
    Object.defineProperty(Symbol, "dispose", { value: Symbol("Symbol.dispose") });
}
const _queue = Symbol.for("queue");
const _value = Symbol.for("value");
const _mutex = Symbol.for("mutex");
const _unlocked = Symbol.for("unlocked");
/**
 * Mutual Exclusion prevents multiple coroutines from accessing the same shared
 * resource simultaneously.
 *
 * **NOTE:**
 * Currently, the Mutex instance can not be used across multiple threads, but is
 * considering adding support for `parallel` threads.
 *
 * @example
 * ```ts
 * import { Mutex } from "@ayonli/jsext/lock";
 * import { random } from "@ayonli/jsext/number";
 * import { sleep } from "@ayonli/jsext/async";
 *
 * const mutex = new Mutex(1);
 *
 * async function concurrentOperation() {
 *     using shared = await mutex.lock();
 *     const value1 = shared.value;
 *
 *     await otherAsyncOperations();
 *
 *     shared.value += 1
 *     const value2 = shared.value;
 *
 *     // Without mutex lock, the shared value may have been modified by other
 *     // calls during `await otherAsyncOperation()`, and the following
 *     // assertion will fail.
 *     console.assert(value1 + 1 === value2);
 * }
 *
 * async function otherAsyncOperations() {
 *     await sleep(100 * random(1, 10));
 * }
 *
 * await Promise.all([
 *     concurrentOperation(),
 *     concurrentOperation(),
 *     concurrentOperation(),
 *     concurrentOperation(),
 * ]);
 * ```
 */
class Mutex {
    /**
     * @param value The data associated to the mutex instance.
     */
    constructor(value) {
        this[_a] = [];
        this[_value] = value;
    }
    /**
     * Acquires the lock of the mutex, optionally for modifying the shared
     * resource.
     */
    async lock() {
        await new Promise(resolve => {
            if (this[_queue].length) {
                this[_queue].push(resolve);
            }
            else {
                this[_queue].push(resolve);
                resolve();
            }
        });
        const lock = Object.create(Mutex.Lock.prototype);
        lock[_mutex] = this;
        return lock;
    }
}
_a = _queue;
(function (Mutex) {
    var _b;
    class Lock {
        constructor(mutex) {
            this[_b] = false;
            this[_mutex] = mutex;
        }
        /** Accesses the data associated to the mutex instance. */
        get value() {
            if (this[_unlocked]) {
                throw new ReferenceError("trying to access data after unlocked");
            }
            return this[_mutex][_value];
        }
        set value(v) {
            if (this[_unlocked]) {
                throw new ReferenceError("trying to access data after unlocked");
            }
            this[_mutex][_value] = v;
        }
        /** Releases the current lock of the mutex. */
        unlock() {
            this[_unlocked] = true;
            const queue = this[_mutex][_queue];
            queue.shift();
            const next = queue[0];
            if (next) {
                next();
            }
            else if (registry.hasValue(this[_mutex])) {
                registry.deleteValue(this[_mutex]);
            }
        }
        [(_b = _unlocked, Symbol.dispose)]() {
            this.unlock();
        }
    }
    Mutex.Lock = Lock;
})(Mutex || (Mutex = {}));
const registry = new BiMap();
/**
 * Acquires a mutex lock for the given key in order to perform concurrent
 * operations and prevent conflicts.
 *
 * If the key is currently being locked by other coroutines, this function will
 * block until the lock becomes available again.
 *
 * @example
 * ```ts
 * import lock from "@ayonli/jsext/lock";
 *
 * const key = "lock_key";
 *
 * export async function concurrentOperation() {
 *     using ctx = await lock(key);
 *     void ctx;
 *
 *     // This block will never be run if there are other coroutines holding
 *     // the lock.
 *     //
 *     // Other coroutines trying to lock the same key will also never be run
 *     // before this function completes.
 * }
 * ```
 */
async function lock(key) {
    let mutex = registry.get(key);
    if (!mutex) {
        registry.set(key, mutex = new Mutex(void 0));
    }
    return await mutex.lock();
}

export { Mutex, lock as default };
//# sourceMappingURL=lock.js.map
