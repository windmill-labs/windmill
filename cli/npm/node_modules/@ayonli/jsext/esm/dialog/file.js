import { isDeno, isNodeLike, isBrowserWindow } from '../env.js';
import { platform } from '../runtime.js';
import { readAsObjectURL } from '../reader.js';
import { asyncTask } from '../async.js';
import { getExtensions } from '../filetype.js';
import { readFileAsFile, readDir, writeFile } from '../fs.js';
import { fixFileType } from '../fs/util.js';
import { as, pick } from '../object.js';
import { join, basename } from '../path.js';
import { createProgressEvent } from '../event.js';
import progress from './progress.js';

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
async function pickFile(options = {}) {
    if (typeof globalThis["showOpenFilePicker"] === "function") {
        const { browserPickFile } = await import('./browser/file.js');
        return await browserPickFile(options.type, {
            forSave: options.forSave,
            defaultName: options.defaultName,
        });
    }
    else if (isDeno || isNodeLike) {
        const { isWSL, which } = await import('../cli.js');
        const _platform = platform();
        if (_platform === "darwin") {
            const { macPickFile } = await import('./terminal/file/mac.js');
            return await macPickFile(options.title, {
                type: options.type,
                forSave: options === null || options === void 0 ? void 0 : options.forSave,
                defaultName: options === null || options === void 0 ? void 0 : options.defaultName,
            });
        }
        else if (_platform === "windows" || isWSL()) {
            const { windowsPickFile } = await import('./terminal/file/windows.js');
            return await windowsPickFile(options.title, {
                type: options.type,
                forSave: options === null || options === void 0 ? void 0 : options.forSave,
                defaultName: options === null || options === void 0 ? void 0 : options.defaultName,
            });
        }
        else if (_platform === "linux" || await which("zenity")) {
            const { linuxPickFile } = await import('./terminal/file/linux.js');
            return await linuxPickFile(options.title, {
                type: options.type,
                forSave: options === null || options === void 0 ? void 0 : options.forSave,
                defaultName: options === null || options === void 0 ? void 0 : options.defaultName,
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
async function pickFiles(options = {}) {
    if (typeof globalThis["showOpenFilePicker"] === "function") {
        const { browserPickFiles } = await import('./browser/file.js');
        return await browserPickFiles(options.type);
    }
    else if (isDeno || isNodeLike) {
        const { isWSL, which } = await import('../cli.js');
        const _platform = platform();
        if (_platform === "darwin") {
            const { macPickFiles } = await import('./terminal/file/mac.js');
            return await macPickFiles(options.title, options.type);
        }
        else if (_platform === "windows" || isWSL()) {
            const { windowsPickFiles } = await import('./terminal/file/windows.js');
            return await windowsPickFiles(options.title, options.type);
        }
        else if (_platform === "linux" || await which("zenity")) {
            const { linuxPickFiles } = await import('./terminal/file/linux.js');
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
async function pickDirectory(options = {}) {
    if (typeof globalThis["showDirectoryPicker"] === "function") {
        const { browserPickFolder } = await import('./browser/file.js');
        return await browserPickFolder();
    }
    else if (isDeno || isNodeLike) {
        const { isWSL, which } = await import('../cli.js');
        const _platform = platform();
        if (_platform === "darwin") {
            const { macPickFolder } = await import('./terminal/file/mac.js');
            return await macPickFolder(options.title);
        }
        else if (_platform === "windows" || isWSL()) {
            const { windowsPickFolder } = await import('./terminal/file/windows.js');
            return await windowsPickFolder(options.title);
        }
        else if (_platform === "linux" || await which("zenity")) {
            const { linuxPickFolder } = await import('./terminal/file/linux.js');
            return await linuxPickFolder(options.title);
        }
    }
    throw new Error("Unsupported platform");
}
async function openFile(options = {}) {
    const { title = "", type = "", multiple = false, directory = false } = options;
    if (directory) {
        return await openDirectory({ title });
    }
    else if (multiple) {
        return await openFiles({ title, type });
    }
    if (typeof globalThis["showOpenFilePicker"] === "function") {
        const { browserPickFile } = await import('./browser/file.js');
        const handle = await browserPickFile(type);
        return handle ? await handle.getFile().then(fixFileType) : null;
    }
    else if (isBrowserWindow) {
        const input = document.createElement("input");
        input.type = "file";
        input.accept = type !== null && type !== void 0 ? type : "";
        return await new Promise(resolve => {
            input.onchange = () => {
                var _a;
                const file = (_a = input.files) === null || _a === void 0 ? void 0 : _a[0];
                resolve(file ? fixFileType(file) : null);
            };
            input.oncancel = () => {
                resolve(null);
            };
            if (typeof input.showPicker === "function") {
                input.showPicker();
            }
            else {
                input.click();
            }
        });
    }
    else if (isDeno || isNodeLike) {
        let filename = await pickFile({ title, type });
        if (filename) {
            return await readFileAsFile(filename);
        }
        else {
            return null;
        }
    }
    else {
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
async function openFiles(options = {}) {
    if (typeof globalThis["showOpenFilePicker"] === "function") {
        const { browserPickFiles } = await import('./browser/file.js');
        const handles = await browserPickFiles(options.type);
        const files = [];
        for (const handle of handles) {
            const file = await handle.getFile();
            files.push(fixFileType(file));
        }
        return files;
    }
    else if (isBrowserWindow) {
        const input = document.createElement("input");
        input.type = "file";
        input.multiple = true;
        input.accept = options.type || "";
        return await new Promise(resolve => {
            input.onchange = () => {
                const files = input.files;
                resolve(files ? [...files].map(fixFileType) : []);
            };
            input.oncancel = () => {
                resolve([]);
            };
            if (typeof input.showPicker === "function") {
                input.showPicker();
            }
            else {
                input.click();
            }
        });
    }
    else if (isDeno || isNodeLike) {
        const filenames = await pickFiles(options);
        return await Promise.all(filenames.map(path => readFileAsFile(path)));
    }
    else {
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
async function openDirectory(options = {}) {
    if (typeof globalThis["showDirectoryPicker"] === "function") {
        const { browserPickFolder } = await import('./browser/file.js');
        const dir = await browserPickFolder();
        const files = [];
        if (!dir) {
            return files;
        }
        for await (const entry of readDir(dir, { recursive: true })) {
            if (entry.kind === "file") {
                const file = await entry.handle.getFile();
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
    }
    else if (isBrowserWindow) {
        const input = document.createElement("input");
        input.type = "file";
        input.webkitdirectory = true;
        return await new Promise(resolve => {
            input.onchange = () => {
                const files = input.files;
                resolve(files ? [...files].map(fixFileType) : []);
            };
            input.oncancel = () => {
                resolve([]);
            };
            if (typeof input.showPicker === "function") {
                input.showPicker();
            }
            else {
                input.click();
            }
        });
    }
    else if (isDeno || isNodeLike) {
        const dirname = await pickDirectory(options);
        if (dirname) {
            const files = [];
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
        }
        else {
            return [];
        }
    }
    else {
        throw new Error("Unsupported runtime");
    }
}
async function saveFile(file, options = {}) {
    var _a;
    if (isBrowserWindow) {
        const a = document.createElement("a");
        if (file instanceof ReadableStream) {
            const type = options.type || "application/octet-stream";
            a.href = await readAsObjectURL(file, type);
            a.download = options.name || "Unnamed" + (getExtensions(type)[0] || "");
        }
        else if (file instanceof File) {
            a.href = URL.createObjectURL(file);
            a.download = options.name || file.name || "Unnamed" + (getExtensions(file.type)[0] || "");
        }
        else if (file instanceof Blob) {
            a.href = URL.createObjectURL(file);
            a.download = options.name || "Unnamed" + (getExtensions(file.type)[0] || "");
        }
        else {
            const type = options.type || "application/octet-stream";
            const blob = new Blob([file], { type });
            a.href = URL.createObjectURL(blob);
            a.download = options.name || "Unnamed" + (getExtensions(type)[0] || "");
        }
        a.click();
    }
    else if (isDeno || isNodeLike) {
        const { title } = options;
        let filename;
        if (typeof Blob === "function" && file instanceof Blob) {
            filename = await pickFile({
                title,
                type: options.type || file.type,
                forSave: true,
                defaultName: options.name || ((_a = as(file, File)) === null || _a === void 0 ? void 0 : _a.name),
            });
        }
        else {
            filename = await pickFile({
                title,
                type: options.type,
                forSave: true,
                defaultName: options.name,
            });
        }
        if (filename) {
            await writeFile(filename, file, pick(options, ["signal"]));
        }
    }
    else {
        throw new Error("Unsupported runtime");
    }
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
async function downloadFile(url, options = {}) {
    var _a;
    const src = typeof url === "object" ? url.href : url;
    const name = options.name || basename(src);
    if (isBrowserWindow) {
        const a = document.createElement("a");
        a.href = src;
        a.download = name;
        a.click();
        return;
    }
    else if (!isDeno && !isNodeLike || typeof fetch !== "function") {
        throw new Error("Unsupported runtime");
    }
    const dest = await pickFile({
        title: options.title,
        type: options.type,
        forSave: true,
        defaultName: name,
    });
    if (!dest) // user canceled
        return;
    const task = asyncTask();
    let signal = (_a = options.signal) !== null && _a !== void 0 ? _a : null;
    let result;
    let updateProgress;
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
    }
    else {
        result = task;
    }
    const res = await fetch(src, { signal });
    if (!res.ok) {
        throw new Error(`Failed to download: ${src}`);
    }
    const size = parseInt(res.headers.get("Content-Length") || "0", 10);
    let stream = res.body;
    if (options.onProgress || options.showProgress) {
        const { onProgress } = options;
        let loaded = 0;
        const transform = new TransformStream({
            transform(chunk, controller) {
                controller.enqueue(chunk);
                loaded += chunk.byteLength;
                if (onProgress) {
                    try {
                        onProgress === null || onProgress === void 0 ? void 0 : onProgress(createProgressEvent("progress", {
                            lengthComputable: !!size,
                            loaded,
                            total: size !== null && size !== void 0 ? size : 0,
                        }));
                    }
                    catch (_a) {
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
    writeFile(dest, stream, { signal: signal }).then(() => {
        task.resolve();
    }).catch(err => {
        task.reject(err);
    });
    await result;
}

export { downloadFile, openDirectory, openFile, openFiles, pickDirectory, pickFile, pickFiles, saveFile };
//# sourceMappingURL=file.js.map
