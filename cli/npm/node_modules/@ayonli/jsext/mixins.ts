/**
 * @deprecated use `mixin` and `class` module instead.
 * @module
 */
import mixin from "./mixin.ts";
import { isSubclassOf } from "./class.ts";

export {
    /**
     * @deprecated This is a redundant re-export of the `isSubclassOf` from `@ayonli/jsext/class`,
     *  will be removed in the next major version.
     */
    isSubclassOf,

    /**
     * @deprecated This is a redundant re-export of the `mixin` module,
     *  will be removed in the next major version.
     */
    mixin as default,
};
