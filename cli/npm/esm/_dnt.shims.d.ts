import { Deno } from "@deno/shim-deno";
export { Deno } from "@deno/shim-deno";
export declare const dntGlobalThis: Omit<typeof globalThis, "Deno"> & {
    Deno: typeof Deno;
};
//# sourceMappingURL=_dnt.shims.d.ts.map