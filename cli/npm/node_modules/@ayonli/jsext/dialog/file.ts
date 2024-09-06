import { isBrowserWindow, isDeno, isNodeLike } from "../env.ts";
import { platform } from "../runtime.ts";
import { readAsObjectURL } from "../reader.ts";
import { asyncTask } from "../async.ts";
import { getExtensions } from "../filetype.ts";
import { readDir, readFileAsFile, writeFile } from "../fs.ts";
import { fixFileType } from "../fs/util.ts";
import { as, pick } from "../object.ts";
import { basename, join } from "../path.ts";
import { createProgressEvent } from "../event.ts";
import progress, { ProgressState } from "./progress.ts";

/**
 * Options for file dialog functions, such as {@link pickFile} and
 * {@link openFile}.
 */
export interface FileDialogOptions {
    /**
     * Customize the dialog's title. This option is ignored in the browser.
     */
    title?: string | undefined;
    /**
     * Filter files by providing a MIME type or suffix, multiple types can be
     * separated via `,`.
     */
    type?: string | undefined;
}

/**
 * Options for the {@link pickFile} function.
 */
export interface PickFileOptions extends FileDialogOptions {
    /** Open the dialog in save mode. */
    forSave?: boolean;
    /** The default name of the file to save when `forSave` is set. */
    defaultName?: string | undefined;
}

/**
 * Opens the file picker dialog and pick a file, this function returns the
 * file's path or a `FileSystemFileHandle` in the browser.
 * 
 * NOTE: Browser support is limited to the chromium-based browsers.
 * 
 * @example
 * ```ts
 * // default usage
 * import { pickFile } from "@ayonli/jsext/dialog";
 * 
 * // Node.js, Deno, Bun
 * const filename = await pickFile() as string | null;
 * 
 * // Browser (Chrome)
 * const handle = await pickFile() as FileSystemFileHandle | null;
 * ```
 * 
 * @example
 * ```ts
 * // filter by MIME type
 * import { pickFile } from "@ayonli/jsext/dialog";
 * 
 * // Node.js, Deno, Bun
 * const filename = await pickFile({ type: "image/*" }) as string | null;
 * 
 * // Browser (Chrome)
 * const handle = await pickFile({ type: "image/*" }) as FileSystemFileHandle | null;
 * ```
 * 
 * @example
 * ```ts
 * // pick for save
 * import { pickFile } from "@ayonli/jsext/dialog";
 * 
 * // Node.js, Deno, Bun
 * const filename = await pickFile({
 *     forSave: true,
 *     defaultName: "hello.txt",
 * }) as string | null;
 * 
 * // Browser (Chrome)
 * const handle = await pickFile({
 *     forSave: true,
 *     defaultName: "hello.txt",
 * }) as FileSystemFileHandle | null;
 * ```
 */
export async function pickFile(
    options: PickFileOptions = {}
): Promise<string | FileSystemFileHandle | null> {
    if (typeof (globalThis as any)["showOpenFilePicker"] === "function") {
        const { browserPickFile } = await import("./browser/file.ts");
        return await browserPickFile(options.type, {
            forSave: options.forSave,
            defaultName: options.defaultName,
        });
    } else if (isDeno || isNodeLike) {
        const { isWSL, which } = await import("../cli.ts");
        const _platform = platform();

        if (_platform === "darwin") {
            const { macPickFile } = await import("./terminal/file/mac.ts");
            return await macPickFile(options.title, {
                type: options.type,
                forSave: options?.forSave,
                defaultName: options?.defaultName,
            });
        } else if (_platform === "windows" || isWSL()) {
            const { windowsPickFile } = await import("./terminal/file/windows.ts");
            return await windowsPickFile(options.title, {
                type: options.type,
                forSave: options?.forSave,
                defaultName: options?.defaultName,
            });
        } else if (_platform === "linux" || await which("zenity")) {
            const { linuxPickFile } = await import("./terminal/file/linux.ts");
            return await linuxPickFile(options.title, {
                type: options.type,
                forSave: options?.forSave,
                defaultName: options?.defaultName,
            });
        }
    }

    throw new Error("Unsupported platform");
}

/**
 * Opens the file picker dialog and pick multiple files, this function returns
 * the paths or `FileSystemFileHandle` objects in the browser of the files
 * selected.
 * 
 * NOTE: Browser support is limited to the chromium-based browsers.
 * 
 * @example
 * ```ts
 * // default usage
 * import { pickFiles } from "@ayonli/jsext/dialog";
 * 
 * // Node.js, Deno, Bun
 * const filenames = await pickFiles() as string[];
 * 
 * // Browser (Chrome)
 * const handles = await pickFiles() as FileSystemFileHandle[];
 * ```
 * 
 * @example
 * ```ts
 * // filter by MIME type
 * import { pickFiles } from "@ayonli/jsext/dialog";
 * 
 * // Node.js, Deno, Bun
 * const filenames = await pickFiles({ type: "image/*" }) as string[];
 * 
 * // Browser (Chrome)
 * const handles = await pickFiles({ type: "image/*" }) as FileSystemFileHandle[];
 * ```
 */
export async function pickFiles(
    options: FileDialogOptions = {}
): Promise<string[] | FileSystemFileHandle[]> {
    if (typeof (globalThis as any)["showOpenFilePicker"] === "function") {
        const { browserPickFiles } = await import("./browser/file.ts");
        return await browserPickFiles(options.type);
    } else if (isDeno || isNodeLike) {
        const { isWSL, which } = await import("../cli.ts");
        const _platform = platform();

        if (_platform === "darwin") {
            const { macPickFiles } = await import("./terminal/file/mac.ts");
            return await macPickFiles(options.title, options.type);
        } else if (_platform === "windows" || isWSL()) {
            const { windowsPickFiles } = await import("./terminal/file/windows");
            return await windowsPickFiles(options.title, options.type);
        } else if (_platform === "linux" || await which("zenity")) {
            const { linuxPickFiles } = await import("./terminal/file/linux.ts");
            return await linuxPickFiles(options.title, options.type);
        }
    }

    throw new Error("Unsupported platform");
}

/**
 * Opens the file picker dialog and pick a directory, this function returns the
 * directory's path or `FileSystemDirectoryHandle` in the browser.
 * 
 * NOTE: Browser support is limited to the chromium-based browsers.
 * 
 * @example
 * ```ts
 * import { pickDirectory } from "@ayonli/jsext/dialog";
 * 
 * // Node.js, Deno, Bun
 * const dirname = await pickDirectory() as string | null;
 * 
 * // Browser (Chrome)
 * const handle = await pickDirectory() as FileSystemDirectoryHandle | null;
 * ```
 */
export async function pickDirectory(
    options: Pick<FileDialogOptions, "title"> = {}
): Promise<string | FileSystemDirectoryHandle | null> {
    if (typeof (globalThis as any)["showDirectoryPicker"] === "function") {
        const { browserPickFolder } = await import("./browser/file.ts");
        return await browserPickFolder();
    } else if (isDeno || isNodeLike) {
        const { isWSL, which } = await import("../cli.ts");
        const _platform = platform();

        if (_platform === "darwin") {
            const { macPickFolder } = await import("./terminal/file/mac.ts");
            return await macPickFolder(options.title);
        } else if (_platform === "windows" || isWSL()) {
            const { windowsPickFolder } = await import("./terminal/file/windows.ts");
            return await windowsPickFolder(options.title);
        } else if (_platform === "linux" || await which("zenity")) {
            const { linuxPickFolder } = await import("./terminal/file/linux.ts");
            return await linuxPickFolder(options.title);
        }
    }

    throw new Error("Unsupported platform");
}

/**
 * Opens the file picker dialog and selects a file to open.
 * 
 * @example
 * ```ts
 * // default usage
 * import { openFile } from "@ayonli/jsext/dialog";
 * 
 * const file = await openFile();
 * 
 * if (file) {
 *     console.log(`You selected: ${file.name}`);
 * }
 * ```
 * 
 * @example
 * ```ts
 * // filter by MIME type
 * import { openFile } from "@ayonli/jsext/dialog";
 * 
 * const file = await openFile({ type: "image/*" });
 * 
 * if (file) {
 *     console.log(`You selected: ${file.name}`);
 *     console.assert(file.type.startsWith("image/"));
 * }
 * ```
 */
export function openFile(options?: FileDialogOptions): Promise<File | null>;
/**
 * Opens the file picker dialog and selects multiple files to open.
 * 
 * @deprecated use {@link openFiles} instead.
 */
export function openFile(options: FileDialogOptions & {
    multiple: true;
}): Promise<File[]>;
/**
 * Opens the directory picker dialog and selects all its files.
 * 
 * @deprecated use {@link openDirectory} instead.
 */
export function openFile(options: Pick<FileDialogOptions, "title"> & {
    directory: true;
}): Promise<File[]>;
export async function openFile(options: FileDialogOptions & {
    multiple?: boolean;
    directory?: boolean;
} = {}): Promise<File | File[] | null> {
    const { title = "", type = "", multiple = false, directory = false } = options;

    if (directory) {
        return await openDirectory({ title });
    } else if (multiple) {
        return await openFiles({ title, type });
    }

    if (typeof (globalThis as any)["showOpenFilePicker"] === "function") {
        const { browserPickFile } = await import("./browser/file.ts");
        const handle = await browserPickFile(type);
        return handle ? await handle.getFile().then(fixFileType) : null;
    } else if (isBrowserWindow) {
        const input = document.createElement("input");
        input.type = "file";
        input.accept = type ?? "";

        return await new Promise<File | File[] | null>(resolve => {
            input.onchange = () => {
                const file = input.files?.[0];
                resolve(file ? fixFileType(file) : null);
            };
            input.oncancel = () => {
                resolve(null);
            };

            if (typeof input.showPicker === "function") {
                input.showPicker();
            } else {
                input.click();
            }
        });
    } else if (isDeno || isNodeLike) {
        let filename = await pickFile({ title, type }) as string | null;

        if (filename) {
            return await readFileAsFile(filename);
        } else {
            return null;
        }
    } else {
        throw new Error("Unsupported runtime");
    }
}

/**
 * Opens the file picker dialog and selects multiple files to open.
 * 
 * @example
 * ```ts
 * // default usage
 * import { openFiles } from "@ayonli/jsext/dialog";
 * 
 * const files = await openFiles();
 * 
 * if (files.length > 0) {
 *     console.log(`You selected: ${files.map(file => file.name).join(", ")}`);
 * }
 * ```
 * 
 * @example
 * ```ts
 * // filter by MIME type
 * import { openFiles } from "@ayonli/jsext/dialog";
 * 
 * const files = await openFiles({ type: "image/*" });
 * 
 * if (files.length > 0) {
 *     console.log(`You selected: ${files.map(file => file.name).join(", ")}`);
 *     console.assert(files.every(file => file.type.startsWith("image/")));
 * }
 * ```
 */
export async function openFiles(options: FileDialogOptions = {}): Promise<File[]> {
    if (typeof (globalThis as any)["showOpenFilePicker"] === "function") {
        const { browserPickFiles } = await import("./browser/file.ts");
        const handles = await browserPickFiles(options.type);
        const files: File[] = [];

        for (const handle of handles) {
            const file = await handle.getFile();
            files.push(fixFileType(file));
        }

        return files;
    } else if (isBrowserWindow) {
        const input = document.createElement("input");
        input.type = "file";
        input.multiple = true;
        input.accept = options.type || "";

        return await new Promise<File[]>(resolve => {
            input.onchange = () => {
                const files = input.files;
                resolve(files ? [...files].map(fixFileType) : []);
            };
            input.oncancel = () => {
                resolve([]);
            };

            if (typeof input.showPicker === "function") {
                input.showPicker();
            } else {
                input.click();
            }
        });
    } else if (isDeno || isNodeLike) {
        const filenames = await pickFiles(options) as string[];
        return await Promise.all(filenames.map(path => readFileAsFile(path)));
    } else {
        throw new Error("Unsupported runtime");
    }
}

/**
 * Opens the directory picker dialog and selects all its files to open.
 * 
 * @example
 * ```ts
 * import { openDirectory } from "@ayonli/jsext/dialog";
 * 
 * const files = await openDirectory();
 * 
 * for (const file of files) {
 *     console.log(`File name: ${file.name}, path: ${file.webkitRelativePath}`);
 * }
 * ```
 */
export async function openDirectory(
    options: Pick<FileDialogOptions, "title"> = {}
): Promise<File[]> {
    if (typeof (globalThis as any)["showDirectoryPicker"] === "function") {
        const { browserPickFolder } = await import("./browser/file.ts");
        const dir = await browserPickFolder();
        const files: File[] = [];

        if (!dir) {
            return files;
        }

        for await (const entry of readDir(dir, { recursive: true })) {
            if (entry.kind === "file") {
                const file = await (entry.handle as FileSystemFileHandle).getFile();

                Object.defineProperty(file, "webkitRelativePath", {
                    configurable: true,
                    enumerable: true,
                    writable: false,
                    value: entry.relativePath.replace(/\\/g, "/"),
                });

                files.push(fixFileType(file));
            }
        }

        return files;
    } else if (isBrowserWindow) {
        const input = document.createElement("input");
        input.type = "file";
        input.webkitdirectory = true;

        return await new Promise<File[]>(resolve => {
            input.onchange = () => {
                const files = input.files;
                resolve(files ? [...files].map(fixFileType) : []);
            };
            input.oncancel = () => {
                resolve([]);
            };

            if (typeof input.showPicker === "function") {
                input.showPicker();
            } else {
                input.click();
            }
        });
    } else if (isDeno || isNodeLike) {
        const dirname = await pickDirectory(options) as string | null;

        if (dirname) {
            const files: File[] = [];

            for await (const entry of readDir(dirname, { recursive: true })) {
                if (entry.kind === "file") {
                    const path = join(dirname, entry.relativePath);
                    const file = await readFileAsFile(path);

                    Object.defineProperty(file, "webkitRelativePath", {
                        configurable: true,
                        enumerable: true,
                        writable: false,
                        value: entry.relativePath.replace(/\\/g, "/"),
                    });

                    files.push(fixFileType(file));
                }
            }

            return files;
        } else {
            return [];
        }
    } else {
        throw new Error("Unsupported runtime");
    }
}

/**
 * Options for the {@link saveFile} function.
 */
export interface SaveFileOptions {
    /**
     * Customize the dialog's title. This option is ignored in the browser.
     */
    title?: string;
    /** The suggested name of the file. */
    name?: string;
    /** The MIME type of the file. */
    type?: string;
    signal?: AbortSignal;
}

/**
 * Saves a file to the file system.
 * 
 * In the CLI, this function will open a dialog to let the user choose the
 * location where the file will be saved. In the browser, the file will be saved
 * to the default download location, or the browser will prompt the user to
 * choose a location.
 * 
 * @example
 * ```ts
 * import { saveFile } from "@ayonli/jsext/dialog";
 * 
 * const file = new File(["Hello, World!"], "hello.txt", { type: "text/plain" });
 * 
 * await saveFile(file);
 * ```
 */
export async function saveFile(file: File, options?: Pick<SaveFileOptions, "title">): Promise<void>;
/**
 * @example
 * ```ts
 * import { saveFile } from "@ayonli/jsext/dialog";
 * import bytes from "@ayonli/jsext/bytes";
 * 
 * const data = bytes("Hello, World!");
 * 
 * await saveFile(data, { name: "hello.txt", type: "text/plain" });
 * ```
 */
export async function saveFile(
    file: Blob | ArrayBuffer | ArrayBufferView | ReadableStream<Uint8Array>,
    options?: SaveFileOptions
): Promise<void>;
export async function saveFile(
    file: File | Blob | ArrayBuffer | ArrayBufferView | ReadableStream<Uint8Array>,
    options: SaveFileOptions = {}
): Promise<void> {
    if (isBrowserWindow) {
        const a = document.createElement("a");

        if (file instanceof ReadableStream) {
            const type = options.type || "application/octet-stream";
            a.href = await readAsObjectURL(file, type);
            a.download = options.name || "Unnamed" + (getExtensions(type)[0] || "");
        } else if (file instanceof File) {
            a.href = URL.createObjectURL(file);
            a.download = options.name || file.name || "Unnamed" + (getExtensions(file.type)[0] || "");
        } else if (file instanceof Blob) {
            a.href = URL.createObjectURL(file);
            a.download = options.name || "Unnamed" + (getExtensions(file.type)[0] || "");
        } else {
            const type = options.type || "application/octet-stream";
            const blob = new Blob([file], { type });
            a.href = URL.createObjectURL(blob);
            a.download = options.name || "Unnamed" + (getExtensions(type)[0] || "");
        }

        a.click();
    } else if (isDeno || isNodeLike) {
        const { title } = options;
        let filename: string | null | undefined;

        if (typeof Blob === "function" && file instanceof Blob) {
            filename = await pickFile({
                title,
                type: options.type || file.type,
                forSave: true,
                defaultName: options.name || as(file, File)?.name,
            }) as string | null;
        } else {
            filename = await pickFile({
                title,
                type: options.type,
                forSave: true,
                defaultName: options.name,
            }) as string | null;
        }

        if (filename) {
            await writeFile(filename, file, pick(options, ["signal"]));
        }
    } else {
        throw new Error("Unsupported runtime");
    }
}

/**
 * Options for the {@link downloadFile} function.
 */
export interface DownloadFileOptions extends SaveFileOptions {
    /**
     * A callback function that will be called when the download progress
     * changes.
     */
    onProgress?: (event: ProgressEvent) => void;
    /**
     * Displays a progress bar during the download process. This option shadows
     * the `signal` option if provided, as the progress bar has its own
     * cancellation mechanism.
     */
    showProgress?: boolean;
}

/**
 * Downloads the file of the given URL to the file system.
 * 
 * In the CLI, this function will open a dialog to let the user choose the
 * location where the file will be saved. In the browser, the file will be saved
 * to the default download location, or the browser will prompt the user to
 * choose a location.
 * 
 * NOTE: This function depends on the Fetch API and Web Streams API, in Node.js,
 * it requires Node.js v18.0 or above.
 * 
 * @example
 * ```ts
 * import { downloadFile } from "@ayonli/jsext/dialog";
 * 
 * await downloadFile("https://ayonli.github.io/jsext/README.md");
 * ```
 */
export async function downloadFile(
    url: string | URL,
    options: DownloadFileOptions = {}
): Promise<void> {
    const src = typeof url === "object" ? url.href : url;
    const name = options.name || basename(src);

    if (isBrowserWindow) {
        const a = document.createElement("a");
        a.href = src;
        a.download = name;
        a.click();
        return;
    } else if (!isDeno && !isNodeLike || typeof fetch !== "function") {
        throw new Error("Unsupported runtime");
    }

    const dest = await pickFile({
        title: options.title,
        type: options.type,
        forSave: true,
        defaultName: name,
    }) as string | null;

    if (!dest) // user canceled
        return;

    const task = asyncTask<void>();
    let signal = options.signal ?? null;
    let result: Promise<void | null>;
    let updateProgress: ((state: ProgressState) => void) | undefined;

    if (options.showProgress) {
        const ctrl = new AbortController();
        signal = ctrl.signal;

        result = progress("Downloading...", async (set) => {
            updateProgress = set;
            return await task;
        }, () => {
            ctrl.abort();
            throw new Error("Download canceled");
        });
    } else {
        result = task;
    }

    const res = await fetch(src, { signal });

    if (!res.ok) {
        throw new Error(`Failed to download: ${src}`);
    }

    const size = parseInt(res.headers.get("Content-Length") || "0", 10);
    let stream = res.body!;

    if (options.onProgress || options.showProgress) {
        const { onProgress } = options;
        let loaded = 0;

        const transform = new TransformStream<Uint8Array, Uint8Array>({
            transform(chunk, controller) {
                controller.enqueue(chunk);
                loaded += chunk.byteLength;

                if (onProgress) {
                    try {
                        onProgress?.(createProgressEvent("progress", {
                            lengthComputable: !!size,
                            loaded,
                            total: size ?? 0,
                        }));
                    } catch {
                        // ignore
                    }
                }

                if (updateProgress && size) {
                    updateProgress({
                        percent: loaded / size,
                    });
                }
            },
        });

        stream = stream.pipeThrough(transform);
    }

    writeFile(dest, stream, { signal: signal! }).then(() => {
        task.resolve();
    }).catch(err => {
        task.reject(err);
    });

    await result;
}
