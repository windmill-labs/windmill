import { readDir, stat, createReadableStream, createWritableStream } from '../fs.js';
import { resolve, basename, join } from '../path.js';
import Tarball from './Tarball.js';

async function tar(src, dest = {}, options = {}) {
    var _a, _b;
    let _dest = undefined;
    if (typeof dest === "string") {
        _dest = options.root ? dest : resolve(dest);
    }
    else if (typeof dest === "object") {
        if (typeof FileSystemFileHandle === "function" && dest instanceof FileSystemFileHandle) {
            _dest = dest;
        }
        else {
            options = dest;
        }
    }
    src = typeof src === "string" && !options.root ? resolve(src) : src;
    const { signal } = options;
    const baseDir = typeof src === "string" ? basename(src) : src.name;
    const entries = readDir(src, { ...options, recursive: true });
    const tarball = new Tarball();
    for await (const entry of entries) {
        let filename;
        let info;
        let stream = null;
        if (entry.handle) {
            filename = entry.relativePath;
            info = await stat(entry.handle);
        }
        else if (typeof src === "string") {
            filename = join(src, entry.relativePath);
            info = await stat(filename, options);
        }
        else {
            filename = entry.relativePath;
            info = await stat(entry.relativePath, options);
        }
        if (info.kind !== "directory") {
            stream = createReadableStream((_a = entry.handle) !== null && _a !== void 0 ? _a : filename, options);
            signal === null || signal === void 0 ? void 0 : signal.addEventListener("abort", () => {
                stream.cancel(signal.reason).catch(() => { });
            }, { once: true });
        }
        let kind;
        if (info.kind === "directory") {
            kind = "directory";
        }
        else if (info.kind === "symlink") {
            kind = "symlink";
        }
        else if (info.isBlockDevice) {
            kind = "block-device";
        }
        else if (info.isCharDevice) {
            kind = "character-device";
        }
        else if (info.isFIFO || info.isSocket) {
            kind = "fifo";
        }
        else {
            kind = "file";
        }
        tarball.append(stream, {
            name: entry.name,
            kind,
            relativePath: baseDir ? baseDir + "/" + entry.relativePath : entry.relativePath,
            size: entry.kind === "directory" ? 0 : info.size,
            mtime: (_b = info.mtime) !== null && _b !== void 0 ? _b : new Date(),
            mode: info.mode,
            uid: info.uid,
            gid: info.gid,
        });
    }
    if (!_dest) {
        return tarball;
    }
    const output = createWritableStream(_dest, options);
    const stream = tarball.stream(options);
    signal === null || signal === void 0 ? void 0 : signal.addEventListener("abort", () => {
        output.abort(signal.reason).catch(() => { });
    }, { once: true });
    await stream.pipeTo(output);
}

export { tar as default };
//# sourceMappingURL=tar.js.map
