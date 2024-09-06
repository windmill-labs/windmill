/**
 * Declares a class that combines all methods from the base classes.
 * @module
 */

import type { Constructor } from "./types.ts";

export type UnionToIntersection<U> = (
    U extends any ? (k: U) => void : never) extends ((k: infer I) => void) ? I : never;

/**
 * Merges properties and methods only if they're missing in the class. 
 */
function mergeIfNotExists(proto: object, source: object, mergeSuper = false) {
    const props = Reflect.ownKeys(source);

    for (const prop of props) {
        if (prop === "constructor") {
            continue;
        } else if (mergeSuper) {
            // When merging properties from super classes, the properties in the
            // base class has the first priority and shall not be overwrite.
            if (!(prop in proto)) {
                setProp(proto, source, <string | symbol>prop);
            }
        } else if (!Object.prototype.hasOwnProperty.call(proto, prop)) {
            setProp(proto, source, <string | symbol>prop);
        }
    }

    return proto;
}

/**
 * Merges properties and methods across the prototype chain.
 */
function mergeHierarchy(ctor: Constructor, mixin: Constructor, mergeSuper = false) {
    mergeIfNotExists(ctor.prototype, mixin.prototype, mergeSuper);

    const _super = Object.getPrototypeOf(mixin);

    // Every user defined class or functions that can be instantiated have their
    // own names, if no name appears, that means the function has traveled to 
    // the root of the hierarchical tree.
    if (_super.name) {
        mergeHierarchy(ctor, _super, true);
    }
}

/**
 * Sets property for prototype based on the given source and prop name properly.
 */
function setProp(proto: any, source: any, prop: string | symbol) {
    const desc = Object.getOwnPropertyDescriptor(source, prop);

    if (desc) {
        Object.defineProperty(proto, prop, desc);
    } else {
        proto[prop] = source[prop];
    }
}

/**
 * Creates a class that combines all methods from the given base class and mixin
 * classes.
 *  
 * @example
 * ```ts
 * import mixin from "@ayonli/jsext/mixin";
 * import { isSubclassOf } from "@ayonli/jsext/class";
 * 
 * class Log {
 *     log(text: string) {
 *         console.log(text);
 *     }
 * }
 * 
 * class View {
 *     display(data: Record<string, any>[]) {
 *         console.table(data);
 *     }
 * }
 * 
 * class Controller extends mixin(View, Log) {
 *     constructor(readonly topic: string) {
 *         super();
 *     }
 * }
 * 
 * const ctrl = new Controller("foo");
 * ctrl.log("something is happening");
 * ctrl.display([{ topic: ctrl.topic, content: "something is happening" }]);
 * 
 * console.assert(isSubclassOf(Controller, View));
 * console.assert(!isSubclassOf(Controller, Log));
 * ```
 */
export default function mixin<T extends Constructor<any>, M extends any[]>(
    base: T,
    ...mixins: { [X in keyof M]: Constructor<M[X]> }
): T & Constructor<UnionToIntersection<FlatArray<M, 1>>>;
export default function mixin<T extends Constructor<any>, M extends any[]>(
    base: T,
    ...mixins: M
): T & Constructor<UnionToIntersection<FlatArray<M, 1>>>;
export default function mixin(base: Constructor<any>, ...mixins: any[]) {
    const obj = { ctor: null as any as Constructor<any> };
    obj.ctor = class extends (<any>base) { }; // make sure this class has no name

    for (const mixin of mixins) {
        if (typeof mixin === "function") {
            mergeHierarchy(obj.ctor, mixin);
        } else if (mixin && typeof mixin === "object") {
            mergeIfNotExists(obj.ctor.prototype, mixin);
        } else {
            throw new TypeError("mixin must be a constructor or an object");
        }
    }

    return obj.ctor as Constructor<any>;
}
