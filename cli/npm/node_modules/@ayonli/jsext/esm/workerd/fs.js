import Exception from '../error/Exception.js';
import '../external/event-target-polyfill/index.js';
import { getMIME } from '../filetype.js';
import { makeTree } from '../fs/util.js';
import { join, basename, extname } from '../path.js';
import { readAsArray, readAsArrayBuffer } from '../reader.js';

const EOL = "\n";
async function getDirHandle(path, options = {}) {
    throw new Error("Unsupported runtime");
}
async function getFileHandle(path, options = {}) {
    throw new Error("Unsupported runtime");
}
function getKVStore(options) {
    var _a;
    // @ts-ignore
    const kv = ((_a = options.root) !== null && _a !== void 0 ? _a : globalThis["__STATIC_CONTENT"]);
    if (!kv) {
        throw new Error("Must set the `options.root` a KVNamespace object");
    }
    return kv;
}
// @ts-ignore
const loadManifest = (async () => {
    // @ts-ignore
    if (globalThis["__STATIC_CONTENT_MANIFEST"]) {
        // @ts-ignore
        return globalThis["__STATIC_CONTENT_MANIFEST"];
    }
    // @ts-ignore
    return import('__STATIC_CONTENT_MANIFEST')
        .then((mod) => JSON.parse(mod.default))
        .catch(() => ({}));
})();
function throwNotFoundError(filename, kind = "file") {
    throw new Exception(`${kind === "file" ? "File" : "Directory"} '${filename}' does not exist`, {
        name: "NotFoundError",
        code: 404,
    });
}
async function exists(path, options = {}) {
    void getKVStore(options);
    path = join(path);
    const manifest = await loadManifest;
    const filenames = Object.keys(manifest);
    if (filenames.includes(path)) {
        return true;
    }
    else {
        const dirPath = path + "/";
        return filenames.some(filename => filename.startsWith(dirPath));
    }
}
async function stat(target, options = {}) {
    var _a;
    const filename = join(target);
    const kv = getKVStore(options);
    const manifest = await loadManifest;
    const filenames = Object.keys(manifest);
    if (filenames.includes(filename)) {
        const buffer = await kv.get(filename, { type: "arrayBuffer" });
        return {
            name: basename(filename),
            kind: "file",
            size: buffer.byteLength,
            type: (_a = getMIME(extname(filename))) !== null && _a !== void 0 ? _a : "",
            mtime: null,
            atime: null,
            birthtime: null,
            mode: 0,
            uid: 0,
            gid: 0,
            isBlockDevice: false,
            isCharDevice: false,
            isFIFO: false,
            isSocket: false,
        };
    }
    else {
        const dirPath = filename + "/";
        if (filenames.some(filename => filename.startsWith(dirPath))) {
            return {
                name: basename(filename),
                kind: "directory",
                size: 0,
                type: "",
                mtime: null,
                atime: null,
                birthtime: null,
                mode: 0,
                uid: 0,
                gid: 0,
                isBlockDevice: false,
                isCharDevice: false,
                isFIFO: false,
                isSocket: false,
            };
        }
        else {
            throwNotFoundError(filename);
        }
    }
}
async function mkdir(path, options = {}) {
    throw new Error("Unsupported runtime");
}
async function ensureDir(path, options = {}) {
    throw new Error("Unsupported runtime");
}
async function* readDir(target, options = {}) {
    void getKVStore(options);
    const manifest = await loadManifest;
    const StaticFilenames = Object.keys(manifest);
    let dirPath = target;
    if (dirPath === "." || dirPath.endsWith("/")) {
        dirPath = dirPath.slice(0, -1);
    }
    let dirPaths = new Set();
    let hasFiles = false;
    if (options.recursive) {
        const prefix = dirPath ? dirPath + "/" : "";
        const _filenames = prefix
            ? StaticFilenames.filter(filename => filename.startsWith(prefix))
            : StaticFilenames;
        if (!_filenames.length) {
            throwNotFoundError(dirPath, "directory");
        }
        for (let relativePath of _filenames) {
            relativePath = relativePath.slice(prefix.length);
            const parts = relativePath.split("/");
            if (parts.length >= 2) { // direct folder
                const dirPath = parts.slice(0, -1).join("/");
                if (!dirPaths.has(dirPath)) {
                    dirPaths.add(dirPath);
                    hasFiles = true;
                    yield {
                        name: parts.slice(-2, -1)[0],
                        kind: "directory",
                        relativePath: dirPath,
                    };
                }
                yield {
                    name: parts[0],
                    kind: "file",
                    relativePath,
                };
            }
            else if (parts.length === 1) { // direct file
                hasFiles = true;
                yield {
                    name: parts[0],
                    kind: "file",
                    relativePath,
                };
            }
        }
        if (!hasFiles) {
            throwNotFoundError(dirPath, "directory");
        }
    }
    else {
        const allEntries = await readAsArray(readDir(target, { ...options, recursive: true }));
        for (const entry of allEntries) {
            if (!entry.relativePath.includes("/")) {
                yield entry;
            }
        }
    }
}
async function readTree(target, options = {}) {
    const entries = (await readAsArray(readDir(target, { ...options, recursive: true })));
    return makeTree(target, entries, true);
}
async function readFile(target, options = {}) {
    const filename = target;
    const kv = getKVStore(options);
    const stream = await kv.get(filename, { type: "stream" });
    if (!stream) {
        throwNotFoundError(filename);
    }
    const ctrl = new AbortController();
    ctrl.signal.addEventListener("abort", () => stream.cancel());
    const buffer = await readAsArrayBuffer(stream);
    return new Uint8Array(buffer);
}
async function readFileAsText(target, options = {}) {
    const filename = target;
    const kv = getKVStore(options);
    const text = await kv.get(filename, { type: "text" });
    if (text === null) {
        throwNotFoundError(filename);
    }
    else {
        return text;
    }
}
async function readFileAsFile(target, options = {}) {
    var _a;
    const filename = target;
    const kv = getKVStore(options);
    const buffer = await kv.get(filename, { type: "arrayBuffer" });
    if (!buffer) {
        throwNotFoundError(filename);
    }
    const file = new File([buffer], filename, {
        type: (_a = getMIME(extname(filename))) !== null && _a !== void 0 ? _a : "",
    });
    Object.defineProperty(file, "webkitRelativePath", {
        configurable: true,
        enumerable: true,
        writable: false,
        value: "",
    });
    return file;
}
async function writeFile(target, data, options = {}) {
    throw new Error("Unsupported runtime");
}
async function writeLines(target, lines, options = {}) {
    throw new Error("Unsupported runtime");
}
async function truncate(target, size = 0, options = {}) {
    throw new Error("Unsupported runtime");
}
async function remove(path, options = {}) {
    throw new Error("Unsupported runtime");
}
async function rename(oldPath, newPath, options = {}) {
    throw new Error("Unsupported runtime");
}
async function copy(src, dest, options = {}) {
    throw new Error("Unsupported runtime");
}
async function link(src, dest, options = {}) {
    throw new Error("Unsupported runtime");
}
async function readLink(path) {
    throw new Error("Unsupported runtime");
}
async function chmod(path, mode) {
}
async function chown(path, uid, gid) {
}
async function utimes(path, atime, mtime) {
}
function createReadableStream(target, options = {}) {
    throw new Error("Unsupported runtime");
}
function createWritableStream(target, options = {}) {
    throw new Error("Unsupported runtime");
}

export { EOL, chmod, chown, copy, createReadableStream, createWritableStream, ensureDir, exists, getDirHandle, getFileHandle, link, mkdir, readDir, readFile, readFileAsFile, readFileAsText, readLink, readTree, remove, rename, stat, truncate, utimes, writeFile, writeLines };
//# sourceMappingURL=fs.js.map
