import { Deno } from "@deno/shim-deno";
export { Deno } from "@deno/shim-deno";

const dntGlobals = {
  Deno,
};
export const dntGlobalThis = createMergeProxy(globalThis, dntGlobals);

function createMergeProxy<T extends object, U extends object>(
  baseObj: T,
  extObj: U,
): Omit<T, keyof U> & U {
  return new Proxy(baseObj, {
    get(_target, prop, _receiver) {
      if (prop in extObj) {
        return (extObj as any)[prop];
      } else {
        return (baseObj as any)[prop];
      }
    },
    set(_target, prop, value) {
      if (prop in extObj) {
        delete (extObj as any)[prop];
      }
      (baseObj as any)[prop] = value;
      return true;
    },
    deleteProperty(_target, prop) {
      let success = false;
      if (prop in extObj) {
        delete (extObj as any)[prop];
        success = true;
      }
      if (prop in baseObj) {
        delete (baseObj as any)[prop];
        success = true;
      }
      return success;
    },
    ownKeys(_target) {
      const baseKeys = Reflect.ownKeys(baseObj);
      const extKeys = Reflect.ownKeys(extObj);
      const extKeysSet = new Set(extKeys);
      return [...baseKeys.filter((k) => !extKeysSet.has(k)), ...extKeys];
    },
    defineProperty(_target, prop, desc) {
      if (prop in extObj) {
        delete (extObj as any)[prop];
      }
      Reflect.defineProperty(baseObj, prop, desc);
      return true;
    },
    getOwnPropertyDescriptor(_target, prop) {
      if (prop in extObj) {
        return Reflect.getOwnPropertyDescriptor(extObj, prop);
      } else {
        return Reflect.getOwnPropertyDescriptor(baseObj, prop);
      }
    },
    has(_target, prop) {
      return prop in extObj || prop in baseObj;
    },
  }) as any;
}
