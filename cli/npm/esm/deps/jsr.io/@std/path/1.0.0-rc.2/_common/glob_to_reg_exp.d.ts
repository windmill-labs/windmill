/** Options for {@linkcode globToRegExp}. */
export interface GlobOptions {
    /** Extended glob syntax.
     * See https://www.linuxjournal.com/content/bash-extended-globbing.
     *
     * @default {true}
     */
    extended?: boolean;
    /** Globstar syntax.
     * See https://www.linuxjournal.com/content/globstar-new-bash-globbing-option.
     * If false, `**` is treated like `*`.
     *
     * @default {true}
     */
    globstar?: boolean;
    /**
     * Whether globstar should be case-insensitive.
     *
     * @default {false}
     */
    caseInsensitive?: boolean;
}
/** Options for {@linkcode globToRegExp}. */
export type GlobToRegExpOptions = GlobOptions;
export interface GlobConstants {
    sep: string;
    sepMaybe: string;
    seps: string[];
    globstar: string;
    wildcard: string;
    escapePrefix: string;
}
export declare function _globToRegExp(c: GlobConstants, glob: string, { extended, globstar: globstarOption, caseInsensitive, }?: GlobToRegExpOptions): RegExp;
//# sourceMappingURL=glob_to_reg_exp.d.ts.map