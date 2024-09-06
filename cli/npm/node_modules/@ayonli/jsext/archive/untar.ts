import { concat as concatBytes } from "../bytes.ts";
import { isDeno, isNodeLike } from "../env.ts";
import { createProgressEvent } from "../event.ts";
import { chmod, createReadableStream, createWritableStream, ensureDir, stat, utimes } from "../fs.ts";
import { makeTree } from "../fs/util.ts";
import { basename, dirname, join, resolve } from "../path.ts";
import { platform } from "../runtime.ts";
import Tarball, { HEADER_LENGTH, TarEntry, TarTree, createEntry, parseHeader } from "./Tarball.ts";
import { TarOptions } from "./tar.ts";

/**
 * Options for the {@link untar} function.
 */
export interface UntarOptions extends TarOptions {
    /**
     * The size of the tarball file in bytes. When specified, the progress event
     * will be dispatched with the `lengthComputable` property set to `true` and
     * the `total` property set to this value.
     * 
     * This option is optional when the source is a file path or a file handle,
     * in which case the size will be determined by the file system.
     */
    size?: number;
    /**
     * A callback function that will be called when the extraction progress
     * changes.
     */
    onProgress?: (event: ProgressEvent) => void;
};

/**
 * Extracts files from a tarball file and writes them to the specified directory.
 * 
 * NOTE: If the destination directory does not exist, it will be created.
 * 
 * @example
 * ```ts
 * import { untar } from "@ayonli/jsext/archive";
 * 
 * await untar("/path/to/archive.tar", "/path/to/directory");
 * // with gzip
 * await untar("/path/to/archive.tar.gz", "/path/to/directory", { gzip: true });
 * ```
 */
export default function untar(
    src: string | FileSystemFileHandle | ReadableStream<Uint8Array>,
    dest: string | FileSystemDirectoryHandle,
    options?: UntarOptions
): Promise<void>;
/**
 * Loads the specified tarball file to a {@link Tarball} instance.
 * 
 * @example
 * ```ts
 * import { untar } from "@ayonli/jsext/archive";
 * 
 * const tarball = await untar("/path/to/archive.tar");
 * // with gzip
 * const tarball = await untar("/path/to/archive.tar.gz", { gzip: true });
 * ```
 */
export default function untar(
    src: string | FileSystemFileHandle | ReadableStream<Uint8Array>,
    options?: TarOptions
): Promise<Tarball>;
export default async function untar(
    src: string | FileSystemFileHandle | ReadableStream<Uint8Array>,
    dest: string | FileSystemDirectoryHandle | UntarOptions = {},
    options: UntarOptions = {}
): Promise<Tarball | void> {
    let _dest: string | FileSystemDirectoryHandle | undefined = undefined;

    if (typeof dest === "string") {
        _dest = options.root ? dest : resolve(dest);
    } else if (typeof dest === "object") {
        if (typeof FileSystemDirectoryHandle === "function" &&
            dest instanceof FileSystemDirectoryHandle
        ) {
            _dest = dest;
        } else {
            options = dest as UntarOptions;
        }
    }

    src = typeof src === "string" && options.root ? resolve(src) : src;
    let input = src instanceof ReadableStream ? src : createReadableStream(src, options);

    if (options.gzip) {
        const gzip = new DecompressionStream("gzip");
        input = input.pipeThrough<Uint8Array>(gzip);
    }

    const { signal } = options;
    signal?.addEventListener("abort", () => {
        input.cancel(signal.reason);
    });

    if (!_dest) {
        return await Tarball.load(input);
    } else if (typeof _dest === "string") {
        await ensureDir(_dest, options);
    }

    let totalWrittenBytes = 0;
    let totalBytes = options.size ?? 0;

    if (!totalBytes && options.onProgress) {
        if (typeof src === "string") {
            const info = await stat(src, options);
            totalBytes = info.size;
        } else if (typeof FileSystemFileHandle === "function"
            && src instanceof FileSystemFileHandle
        ) {
            const info = await src.getFile();
            totalBytes = info.size;
        }
    }

    const entries: TarEntry[] = [];
    const reader = input.getReader();
    let lastChunk: Uint8Array = new Uint8Array(0);
    let entry: TarEntry | null = null;
    let writer: WritableStreamDefaultWriter<Uint8Array> | null = null;
    let writtenBytes = 0;
    let paddingSize = 0;

    try {
        outer:
        while (true) {
            const { done, value } = await reader.read();

            if (done) {
                break;
            }

            lastChunk = lastChunk.byteLength ? concatBytes(lastChunk, value) : value;

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
                        } else {
                            lastChunk = new Uint8Array(0);
                            break outer;
                        }
                    } else {
                        break;
                    }
                }

                const fileSize = entry.size;
                let filename: string | undefined = undefined;

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
                } else if (entry.kind === "directory") {
                    if (typeof FileSystemDirectoryHandle === "function" &&
                        _dest instanceof FileSystemDirectoryHandle
                    ) {
                        await ensureDir(entry.relativePath, { ...options, root: _dest });
                    } else {
                        filename = join(_dest as string, entry.relativePath);
                        await ensureDir(filename, options);
                    }
                } else {
                    let _options = options;

                    if (typeof FileSystemDirectoryHandle === "function" &&
                        _dest instanceof FileSystemDirectoryHandle
                    ) {
                        _options = { ...options, root: _dest };
                        filename = entry.relativePath;
                    } else {
                        filename = join(_dest as string, entry.relativePath);
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
                    writer?.close();
                    writer = null;
                    entry = null;
                } else {
                    break;
                }
            }
        }

        if (lastChunk.byteLength) {
            throw new Error("The archive is corrupted");
        } else if (totalBytes && totalWrittenBytes < totalBytes && options.onProgress) {
            totalWrittenBytes = totalBytes;
            options.onProgress(createProgressEvent("progress", {
                lengthComputable: true,
                loaded: totalWrittenBytes,
                total: totalBytes,
            }));
        }
    } finally {
        reader.releaseLock();
    }

    if ((isDeno || isNodeLike) && typeof _dest === "string") {
        const isWindows = platform() === "windows";
        const tree = makeTree<TarEntry, TarTree>(basename(_dest), entries);

        await (async function restoreStats(nodes: TarTree[]) {
            for (const entry of nodes) {
                const filename = join(_dest, entry.relativePath);

                if (entry.kind === "directory" && entry.children?.length) {
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
        })(tree.children!);
    }
}
