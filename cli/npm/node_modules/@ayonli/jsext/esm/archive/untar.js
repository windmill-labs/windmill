import { concat } from '../bytes.js';
import { isDeno, isNodeLike } from '../env.js';
import { createProgressEvent } from '../event.js';
import { createReadableStream, ensureDir, stat, createWritableStream, chmod, utimes } from '../fs.js';
import { makeTree } from '../fs/util.js';
import { resolve, join, dirname, basename } from '../path.js';
import { platform } from '../runtime.js';
import Tarball, { HEADER_LENGTH, parseHeader, createEntry } from './Tarball.js';

async function untar(src, dest = {}, options = {}) {
    var _a;
    let _dest = undefined;
    if (typeof dest === "string") {
        _dest = options.root ? dest : resolve(dest);
    }
    else if (typeof dest === "object") {
        if (typeof FileSystemDirectoryHandle === "function" &&
            dest instanceof FileSystemDirectoryHandle) {
            _dest = dest;
        }
        else {
            options = dest;
        }
    }
    src = typeof src === "string" && options.root ? resolve(src) : src;
    let input = src instanceof ReadableStream ? src : createReadableStream(src, options);
    if (options.gzip) {
        const gzip = new DecompressionStream("gzip");
        input = input.pipeThrough(gzip);
    }
    const { signal } = options;
    signal === null || signal === void 0 ? void 0 : signal.addEventListener("abort", () => {
        input.cancel(signal.reason);
    });
    if (!_dest) {
        return await Tarball.load(input);
    }
    else if (typeof _dest === "string") {
        await ensureDir(_dest, options);
    }
    let totalWrittenBytes = 0;
    let totalBytes = (_a = options.size) !== null && _a !== void 0 ? _a : 0;
    if (!totalBytes && options.onProgress) {
        if (typeof src === "string") {
            const info = await stat(src, options);
            totalBytes = info.size;
        }
        else if (typeof FileSystemFileHandle === "function"
            && src instanceof FileSystemFileHandle) {
            const info = await src.getFile();
            totalBytes = info.size;
        }
    }
    const entries = [];
    const reader = input.getReader();
    let lastChunk = new Uint8Array(0);
    let entry = null;
    let writer = null;
    let writtenBytes = 0;
    let paddingSize = 0;
    try {
        outer: while (true) {
            const { done, value } = await reader.read();
            if (done) {
                break;
            }
            lastChunk = lastChunk.byteLength ? concat(lastChunk, value) : value;
            while (true) {
                if (paddingSize > 0 && lastChunk.byteLength >= paddingSize) {
                    lastChunk = lastChunk.subarray(paddingSize);
                    paddingSize = 0;
                }
                if (!entry) {
                    if (lastChunk.byteLength >= HEADER_LENGTH) {
                        const _header = parseHeader(lastChunk);
                        if (_header) {
                            lastChunk = _header[2];
                            entry = createEntry(_header[0]);
                            entries.push(entry);
                        }
                        else {
                            lastChunk = new Uint8Array(0);
                            break outer;
                        }
                    }
                    else {
                        break;
                    }
                }
                const fileSize = entry.size;
                let filename = undefined;
                if (writer) {
                    const chunk = lastChunk.subarray(0, fileSize - writtenBytes);
                    await writer.write(chunk);
                    lastChunk = lastChunk.subarray(fileSize - writtenBytes);
                    writtenBytes += chunk.byteLength;
                    if (chunk.byteLength) {
                        totalWrittenBytes += chunk.byteLength;
                        if (options.onProgress) {
                            options.onProgress(createProgressEvent("progress", {
                                lengthComputable: !!totalBytes,
                                loaded: totalWrittenBytes,
                                total: totalBytes
                            }));
                        }
                    }
                }
                else if (entry.kind === "directory") {
                    if (typeof FileSystemDirectoryHandle === "function" &&
                        _dest instanceof FileSystemDirectoryHandle) {
                        await ensureDir(entry.relativePath, { ...options, root: _dest });
                    }
                    else {
                        filename = join(_dest, entry.relativePath);
                        await ensureDir(filename, options);
                    }
                }
                else {
                    let _options = options;
                    if (typeof FileSystemDirectoryHandle === "function" &&
                        _dest instanceof FileSystemDirectoryHandle) {
                        _options = { ...options, root: _dest };
                        filename = entry.relativePath;
                    }
                    else {
                        filename = join(_dest, entry.relativePath);
                    }
                    const dir = dirname(filename);
                    if (dir && dir !== "." && dir !== "/") {
                        await ensureDir(dir, _options);
                    }
                    const output = createWritableStream(filename, _options);
                    writer = output.getWriter();
                    continue;
                }
                if (writtenBytes === fileSize) {
                    paddingSize = HEADER_LENGTH - (fileSize % HEADER_LENGTH || HEADER_LENGTH);
                    writtenBytes = 0;
                    writer === null || writer === void 0 ? void 0 : writer.close();
                    writer = null;
                    entry = null;
                }
                else {
                    break;
                }
            }
        }
        if (lastChunk.byteLength) {
            throw new Error("The archive is corrupted");
        }
        else if (totalBytes && totalWrittenBytes < totalBytes && options.onProgress) {
            totalWrittenBytes = totalBytes;
            options.onProgress(createProgressEvent("progress", {
                lengthComputable: true,
                loaded: totalWrittenBytes,
                total: totalBytes,
            }));
        }
    }
    finally {
        reader.releaseLock();
    }
    if ((isDeno || isNodeLike) && typeof _dest === "string") {
        const isWindows = platform() === "windows";
        const tree = makeTree(basename(_dest), entries);
        await (async function restoreStats(nodes) {
            var _a;
            for (const entry of nodes) {
                const filename = join(_dest, entry.relativePath);
                if (entry.kind === "directory" && ((_a = entry.children) === null || _a === void 0 ? void 0 : _a.length)) {
                    // must restore contents' stats before the directory itself
                    await restoreStats(entry.children);
                }
                // Only restore the permission mode and the last modified time,
                // don't restore the owner and group, because they may not exist
                // and may cause an error.
                //
                // This behavior is consistent with `tar -xf archive.tar` in
                // Unix-like systems.
                if (entry.mode && !isWindows) {
                    await chmod(filename, entry.mode);
                }
                if (entry.mtime) {
                    await utimes(filename, new Date(), entry.mtime);
                }
            }
        })(tree.children);
    }
}

export { untar as default };
//# sourceMappingURL=untar.js.map
