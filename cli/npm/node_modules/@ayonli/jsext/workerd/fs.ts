import { Exception } from "../error.ts";
import { getMIME } from "../filetype.ts";
import type {
    CopyOptions,
    GetDirOptions,
    GetFileOptions,
    LinkOptions,
    MkdirOptions,
    ReadDirOptions,
    ReadFileOptions,
    RemoveOptions,
    StatOptions,
    WriteFileOptions,
} from "../fs.ts";
import type { FileSystemOptions, FileInfo, DirEntry, DirTree } from "../fs/types.ts";
import { makeTree } from "../fs/util.ts";
import { basename, extname, join } from "../path.ts";
import { readAsArray, readAsArrayBuffer } from "../reader.ts";
import { KVNamespace } from "./types.ts";

export type {
    CopyOptions,
    GetDirOptions,
    GetFileOptions,
    LinkOptions,
    MkdirOptions,
    ReadDirOptions,
    ReadFileOptions,
    RemoveOptions,
    StatOptions,
    WriteFileOptions,
};
export type { FileSystemOptions, FileInfo, DirEntry, DirTree };

export const EOL: "\n" | "\r\n" = "\n";

export async function getDirHandle(
    path: string,
    options: GetDirOptions = {}
): Promise<FileSystemDirectoryHandle> {
    void path, options;
    throw new Error("Unsupported runtime");
}

export async function getFileHandle(
    path: string,
    options: GetFileOptions = {}
): Promise<FileSystemFileHandle> {
    void path, options;
    throw new Error("Unsupported runtime");
}

function getKVStore(options: FileSystemOptions): KVNamespace {
    // @ts-ignore
    const kv = (options.root ?? globalThis["__STATIC_CONTENT"]) as KVNamespace | undefined;

    if (!kv) {
        throw new Error("Must set the `options.root` a KVNamespace object");
    }

    return kv as KVNamespace;
}

// @ts-ignore
const loadManifest: Promise<{ [filename: string]: string; }> = (async () => {
    // @ts-ignore
    if (globalThis["__STATIC_CONTENT_MANIFEST"]) {
        // @ts-ignore
        return globalThis["__STATIC_CONTENT_MANIFEST"];
    }

    // @ts-ignore
    return import("__STATIC_CONTENT_MANIFEST")
        .then((mod) => JSON.parse(mod.default))
        .catch(() => ({}));
})();

function throwNotFoundError(filename: string, kind: "file" | "directory" = "file"): never {
    throw new Exception(`${kind === "file" ? "File" : "Directory"} '${filename}' does not exist`, {
        name: "NotFoundError",
        code: 404,
    });
}

export async function exists(path: string, options: FileSystemOptions = {}): Promise<boolean> {
    void getKVStore(options);
    path = join(path);

    const manifest = await loadManifest;
    const filenames = Object.keys(manifest);

    if (filenames.includes(path)) {
        return true;
    } else {
        const dirPath = path + "/";
        return filenames.some(filename => filename.startsWith(dirPath));
    }
}

export async function stat(
    target: string | FileSystemFileHandle | FileSystemDirectoryHandle,
    options: StatOptions = {}
): Promise<FileInfo> {
    const filename = join(target as string);
    const kv = getKVStore(options);

    const manifest = await loadManifest;
    const filenames = Object.keys(manifest);

    if (filenames.includes(filename)) {
        const buffer = await kv.get(filename, { type: "arrayBuffer" }) as ArrayBuffer;
        return {
            name: basename(filename),
            kind: "file",
            size: buffer.byteLength,
            type: getMIME(extname(filename)) ?? "",
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
    } else {
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
        } else {
            throwNotFoundError(filename);
        }
    }
}

export async function mkdir(path: string, options: MkdirOptions = {}): Promise<void> {
    void path, options;
    throw new Error("Unsupported runtime");
}

export async function ensureDir(
    path: string,
    options: Omit<MkdirOptions, "recursive"> = {}
): Promise<void> {
    void path, options;
    throw new Error("Unsupported runtime");
}

export async function* readDir(
    target: string | FileSystemDirectoryHandle,
    options: ReadDirOptions = {}
): AsyncIterableIterator<DirEntry> {
    void getKVStore(options);

    const manifest = await loadManifest;
    const StaticFilenames = Object.keys(manifest);

    let dirPath = target as string;

    if (dirPath === "." || dirPath.endsWith("/")) {
        dirPath = dirPath.slice(0, -1);
    }

    let dirPaths = new Set<string>();
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
                        name: parts.slice(-2, -1)[0]!,
                        kind: "directory",
                        relativePath: dirPath,
                    };
                }

                yield {
                    name: parts[0]!,
                    kind: "file",
                    relativePath,
                };
            } else if (parts.length === 1) { // direct file
                hasFiles = true;
                yield {
                    name: parts[0]!,
                    kind: "file",
                    relativePath,
                };
            }
        }

        if (!hasFiles) {
            throwNotFoundError(dirPath, "directory");
        }
    } else {
        const allEntries = await readAsArray(readDir(target, { ...options, recursive: true }));

        for (const entry of allEntries) {
            if (!entry.relativePath.includes("/")) {
                yield entry;
            }
        }
    }
}

export async function readTree(
    target: string | FileSystemDirectoryHandle,
    options: FileSystemOptions = {}
): Promise<DirTree> {
    const entries = (await readAsArray(readDir(target, { ...options, recursive: true })));
    return makeTree<DirEntry, DirTree>(target, entries, true);
}

export async function readFile(
    target: string | FileSystemFileHandle,
    options: ReadFileOptions = {}
): Promise<Uint8Array> {
    const filename = target as string;
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

export async function readFileAsText(
    target: string | FileSystemFileHandle,
    options: ReadFileOptions = {}
): Promise<string> {
    const filename = target as string;
    const kv = getKVStore(options);
    const text = await kv.get(filename, { type: "text" });

    if (text === null) {
        throwNotFoundError(filename);
    } else {
        return text;
    }
}

export async function readFileAsFile(
    target: string | FileSystemFileHandle,
    options: ReadFileOptions = {}
): Promise<File> {
    const filename = target as string;
    const kv = getKVStore(options);
    const buffer = await kv.get(filename, { type: "arrayBuffer" });

    if (!buffer) {
        throwNotFoundError(filename);
    }

    const file = new File([buffer], filename, {
        type: getMIME(extname(filename)) ?? "",
    });

    Object.defineProperty(file, "webkitRelativePath", {
        configurable: true,
        enumerable: true,
        writable: false,
        value: "",
    });

    return file;
}

export async function writeFile(
    target: string | FileSystemFileHandle,
    data: string | ArrayBuffer | ArrayBufferView | ReadableStream<Uint8Array> | Blob,
    options: WriteFileOptions = {}
): Promise<void> {
    void target, data, options;
    throw new Error("Unsupported runtime");
}

export async function writeLines(
    target: string | FileSystemFileHandle,
    lines: string[],
    options: WriteFileOptions = {}
): Promise<void> {
    void target, lines, options;
    throw new Error("Unsupported runtime");
}

export async function truncate(
    target: string | FileSystemFileHandle,
    size = 0,
    options: FileSystemOptions = {}
): Promise<void> {
    void target, size, options;
    throw new Error("Unsupported runtime");
}

export async function remove(path: string, options: RemoveOptions = {}): Promise<void> {
    void path, options;
    throw new Error("Unsupported runtime");
}

export async function rename(
    oldPath: string,
    newPath: string,
    options: FileSystemOptions = {}
): Promise<void> {
    void oldPath, newPath, options;
    throw new Error("Unsupported runtime");
}

export async function copy(src: string, dest: string, options?: CopyOptions): Promise<void>;
export async function copy(
    src: FileSystemFileHandle,
    dest: FileSystemFileHandle | FileSystemDirectoryHandle
): Promise<void>;
export async function copy(
    src: FileSystemDirectoryHandle,
    dest: FileSystemDirectoryHandle,
    options?: Pick<CopyOptions, "recursive">
): Promise<void>;
export async function copy(
    src: string | FileSystemFileHandle | FileSystemDirectoryHandle,
    dest: string | FileSystemFileHandle | FileSystemDirectoryHandle,
    options: CopyOptions = {}
): Promise<void> {
    void src, dest, options;
    throw new Error("Unsupported runtime");
}

export async function link(src: string, dest: string, options: LinkOptions = {}): Promise<void> {
    void src, dest, options;
    throw new Error("Unsupported runtime");
}

export async function readLink(path: string): Promise<string> {
    void path;
    throw new Error("Unsupported runtime");
}

export async function chmod(path: string, mode: number): Promise<void> {
    void path, mode;
}

export async function chown(path: string, uid: number, gid: number): Promise<void> {
    void path, uid, gid;
}

export async function utimes(
    path: string,
    atime: number | Date,
    mtime: number | Date
): Promise<void> {
    void path, atime, mtime;
}

export function createReadableStream(
    target: string | FileSystemFileHandle,
    options: FileSystemOptions = {}
): ReadableStream<Uint8Array> {
    void target, options;
    throw new Error("Unsupported runtime");
}

export function createWritableStream(
    target: string | FileSystemFileHandle,
    options: Omit<WriteFileOptions, "signal"> = {}
): WritableStream<Uint8Array> {
    void target, options;
    throw new Error("Unsupported runtime");
}
