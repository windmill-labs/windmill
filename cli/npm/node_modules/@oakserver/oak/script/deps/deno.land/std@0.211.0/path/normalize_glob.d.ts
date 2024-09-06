import type { GlobOptions } from "./_common/glob_to_reg_exp.js";
/** Like normalize(), but doesn't collapse "**\/.." when `globstar` is true. */
export declare function normalizeGlob(glob: string, options?: GlobOptions): string;
