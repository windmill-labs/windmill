import { orderBy, startsWith } from "../array.ts";
import { getMIME } from "../filetype.ts";
import { omit } from "../object.ts";
import { basename, extname, split } from "../path.ts";
import { DirEntry } from "./types.ts";

export function fixDirEntry<T extends DirEntry>(entry: T): T {
    Object.defineProperty(entry, "path", {
        get() {
            return entry.relativePath;
        },
    });

    return entry;
}


export function fixFileType(file: File): File {
    if (!file.type) {
        const ext = extname(file.name);

        if (ext) {
            Object.defineProperty(file, "type", {
                value: getMIME(ext) ?? "",
                writable: false,
                configurable: true,
            });
        }
    }

    return file;
}

type CompatDirEntry = Omit<DirEntry, "kind"> & {
    kind: string;
};

type CompatDirTree = Omit<DirEntry, "kind"> & {
    kind: string;
    children?: CompatDirTree[];
};

/**
 * @param addPathProp `DirEntry.prop` is deprecated, this option is for backward
 * compatibility.
 */
export function makeTree<I extends CompatDirEntry, R extends CompatDirTree>(
    dir: string | FileSystemDirectoryHandle,
    entries: I[],
    addPathProp = false
): R {
    type CustomDirEntry = I & { paths: string[]; };
    const list: CustomDirEntry[] = entries.map(entry => ({
        ...entry,
        paths: split(entry.relativePath),
    }));

    const nodes = (function walk(list: CustomDirEntry[], store: CustomDirEntry[]): R[] {
        // Order the entries first by kind, then by names alphabetically.
        list = [
            ...orderBy(list.filter(e => e.kind === "directory"), e => e.name, "asc"),
            ...orderBy(list.filter(e => e.kind === "file"), e => e.name, "asc"),
        ];

        const nodes: R[] = [];

        for (const entry of list) {
            if (entry.kind === "directory") {
                const paths = entry.paths;
                const childEntries = store.filter(e => startsWith(e.paths, paths));
                const directChildren = childEntries
                    .filter(e => e.paths.length === paths.length + 1);

                if (directChildren.length) {
                    const indirectChildren = childEntries
                        .filter(e => !directChildren.includes(e));
                    const _entry = {
                        ...omit(entry, ["paths"]),
                        children: walk(directChildren, indirectChildren),
                    } as unknown as R;

                    addPathProp && fixDirEntry(_entry as DirEntry);
                    nodes.push(_entry);
                } else {
                    let _entry = {
                        ...omit(entry, ["paths"]),
                        children: [],
                    } as unknown as R;

                    addPathProp && fixDirEntry(_entry as DirEntry);
                    nodes.push(_entry);
                }
            } else {
                const _entry = {
                    ...omit(entry, ["paths"]),
                } as unknown as R;

                addPathProp && fixDirEntry(_entry as DirEntry);
                nodes.push(_entry);
            }
        }

        return nodes;
    })(list.filter(entry => entry.paths.length === 1),
        list.filter(entry => entry.paths.length > 1));

    let rootName: string;

    if (typeof dir === "object") {
        rootName = dir.name || "(root)";
    } else if (dir) {
        rootName = basename(dir);

        if (!rootName || rootName === ".") {
            rootName = "(root)";
        }
    } else {
        rootName = "(root)";
    }

    const rooEntry = {
        name: rootName,
        kind: "directory",
        relativePath: "",
        children: nodes,
    } as unknown as R;

    if (typeof dir === "object") {
        rooEntry.handle = dir;
    }

    addPathProp && fixDirEntry(rooEntry as DirEntry);
    return rooEntry;
}
