import { orderBy, startsWith } from '../array.js';
import { getMIME } from '../filetype.js';
import { omit } from '../object.js';
import { extname, basename } from '../path.js';
import { split } from '../path/util.js';

function fixDirEntry(entry) {
    Object.defineProperty(entry, "path", {
        get() {
            return entry.relativePath;
        },
    });
    return entry;
}
function fixFileType(file) {
    var _a;
    if (!file.type) {
        const ext = extname(file.name);
        if (ext) {
            Object.defineProperty(file, "type", {
                value: (_a = getMIME(ext)) !== null && _a !== void 0 ? _a : "",
                writable: false,
                configurable: true,
            });
        }
    }
    return file;
}
/**
 * @param addPathProp `DirEntry.prop` is deprecated, this option is for backward
 * compatibility.
 */
function makeTree(dir, entries, addPathProp = false) {
    const list = entries.map(entry => ({
        ...entry,
        paths: split(entry.relativePath),
    }));
    const nodes = (function walk(list, store) {
        // Order the entries first by kind, then by names alphabetically.
        list = [
            ...orderBy(list.filter(e => e.kind === "directory"), e => e.name, "asc"),
            ...orderBy(list.filter(e => e.kind === "file"), e => e.name, "asc"),
        ];
        const nodes = [];
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
                    };
                    addPathProp && fixDirEntry(_entry);
                    nodes.push(_entry);
                }
                else {
                    let _entry = {
                        ...omit(entry, ["paths"]),
                        children: [],
                    };
                    addPathProp && fixDirEntry(_entry);
                    nodes.push(_entry);
                }
            }
            else {
                const _entry = {
                    ...omit(entry, ["paths"]),
                };
                addPathProp && fixDirEntry(_entry);
                nodes.push(_entry);
            }
        }
        return nodes;
    })(list.filter(entry => entry.paths.length === 1), list.filter(entry => entry.paths.length > 1));
    let rootName;
    if (typeof dir === "object") {
        rootName = dir.name || "(root)";
    }
    else if (dir) {
        rootName = basename(dir);
        if (!rootName || rootName === ".") {
            rootName = "(root)";
        }
    }
    else {
        rootName = "(root)";
    }
    const rooEntry = {
        name: rootName,
        kind: "directory",
        relativePath: "",
        children: nodes,
    };
    if (typeof dir === "object") {
        rooEntry.handle = dir;
    }
    addPathProp && fixDirEntry(rooEntry);
    return rooEntry;
}

export { fixDirEntry, fixFileType, makeTree };
//# sourceMappingURL=util.js.map
