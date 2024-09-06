import * as dntShim from "../../../../../_dnt.shims.js";
/**
 * Walk entry for {@linkcode walk}, {@linkcode walkSync},
 * {@linkcode expandGlob} and {@linkcode expandGlobSync}.
 */
export interface WalkEntry extends dntShim.Deno.DirEntry {
    /** Full path of the entry. */
    path: string;
}
/** Create {@linkcode WalkEntry} for the `path` synchronously. */
export declare function createWalkEntrySync(path: string | URL): WalkEntry;
/** Create {@linkcode WalkEntry} for the `path` asynchronously. */
export declare function createWalkEntry(path: string | URL): Promise<WalkEntry>;
//# sourceMappingURL=_create_walk_entry.d.ts.map