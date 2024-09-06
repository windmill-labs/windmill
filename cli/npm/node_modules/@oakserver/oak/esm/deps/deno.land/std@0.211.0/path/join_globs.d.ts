import type { GlobOptions } from "./_common/glob_to_reg_exp.js";
/** Like join(), but doesn't collapse "**\/.." when `globstar` is true. */
export declare function joinGlobs(globs: string[], options?: GlobOptions): string;
