"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.read = exports.osUptime = exports.osRelease = exports.openSync = exports.open = exports.mkdirSync = exports.mkdir = exports.memoryUsage = exports.makeTempFileSync = exports.makeTempFile = exports.makeTempDirSync = exports.makeTempDir = exports.lstatSync = exports.lstat = exports.loadavg = exports.listenTls = exports.listen = exports.linkSync = exports.link = exports.kill = exports.inspect = exports.hostname = exports.gid = exports.ftruncateSync = exports.ftruncate = exports.fsyncSync = exports.fsync = exports.fstatSync = exports.fstat = exports.fdatasyncSync = exports.fdatasync = exports.exit = exports.execPath = exports.cwd = exports.createSync = exports.create = exports.copyFileSync = exports.copyFile = exports.copy = exports.consoleSize = exports.connectTls = exports.connect = exports.close = exports.chownSync = exports.chown = exports.chmodSync = exports.chmod = exports.chdir = exports.addSignalListener = exports.isatty = void 0;
exports.utimeSync = exports.utime = exports.futimeSync = exports.futime = exports.args = exports.writeTextFileSync = exports.writeTextFile = exports.writeSync = exports.writeFileSync = exports.writeFile = exports.write = exports.watchFs = exports.uid = exports.truncateSync = exports.truncate = exports.test = exports.symlinkSync = exports.symlink = exports.statSync = exports.stat = exports.shutdown = exports.run = exports.Process = exports.resolveDns = exports.renameSync = exports.rename = exports.removeSync = exports.removeSignalListener = exports.remove = exports.realPathSync = exports.realPath = exports.readTextFileSync = exports.readTextFile = exports.readSync = exports.readLinkSync = exports.readLink = exports.readFileSync = exports.readFile = exports.readDirSync = exports.readDir = void 0;
const fs_1 = __importDefault(require("fs"));
const errorMap_js_1 = __importDefault(require("../internal/errorMap.js"));
const variables_js_1 = require("./variables.js");
// todo(dsherret): collapse all these files into here
var tty_1 = require("tty");
Object.defineProperty(exports, "isatty", { enumerable: true, get: function () { return tty_1.isatty; } });
var addSignalListener_js_1 = require("./functions/addSignalListener.js");
Object.defineProperty(exports, "addSignalListener", { enumerable: true, get: function () { return addSignalListener_js_1.addSignalListener; } });
var chdir_js_1 = require("./functions/chdir.js");
Object.defineProperty(exports, "chdir", { enumerable: true, get: function () { return chdir_js_1.chdir; } });
var chmod_js_1 = require("./functions/chmod.js");
Object.defineProperty(exports, "chmod", { enumerable: true, get: function () { return chmod_js_1.chmod; } });
var chmodSync_js_1 = require("./functions/chmodSync.js");
Object.defineProperty(exports, "chmodSync", { enumerable: true, get: function () { return chmodSync_js_1.chmodSync; } });
var chown_js_1 = require("./functions/chown.js");
Object.defineProperty(exports, "chown", { enumerable: true, get: function () { return chown_js_1.chown; } });
var chownSync_js_1 = require("./functions/chownSync.js");
Object.defineProperty(exports, "chownSync", { enumerable: true, get: function () { return chownSync_js_1.chownSync; } });
var close_js_1 = require("./functions/close.js");
Object.defineProperty(exports, "close", { enumerable: true, get: function () { return close_js_1.close; } });
var connect_js_1 = require("./functions/connect.js");
Object.defineProperty(exports, "connect", { enumerable: true, get: function () { return connect_js_1.connect; } });
var connectTls_js_1 = require("./functions/connectTls.js");
Object.defineProperty(exports, "connectTls", { enumerable: true, get: function () { return connectTls_js_1.connectTls; } });
var consoleSize_js_1 = require("./functions/consoleSize.js");
Object.defineProperty(exports, "consoleSize", { enumerable: true, get: function () { return consoleSize_js_1.consoleSize; } });
var copy_js_1 = require("./functions/copy.js");
Object.defineProperty(exports, "copy", { enumerable: true, get: function () { return copy_js_1.copy; } });
var copyFile_js_1 = require("./functions/copyFile.js");
Object.defineProperty(exports, "copyFile", { enumerable: true, get: function () { return copyFile_js_1.copyFile; } });
var copyFileSync_js_1 = require("./functions/copyFileSync.js");
Object.defineProperty(exports, "copyFileSync", { enumerable: true, get: function () { return copyFileSync_js_1.copyFileSync; } });
var create_js_1 = require("./functions/create.js");
Object.defineProperty(exports, "create", { enumerable: true, get: function () { return create_js_1.create; } });
var createSync_js_1 = require("./functions/createSync.js");
Object.defineProperty(exports, "createSync", { enumerable: true, get: function () { return createSync_js_1.createSync; } });
var cwd_js_1 = require("./functions/cwd.js");
Object.defineProperty(exports, "cwd", { enumerable: true, get: function () { return cwd_js_1.cwd; } });
var execPath_js_1 = require("./functions/execPath.js");
Object.defineProperty(exports, "execPath", { enumerable: true, get: function () { return execPath_js_1.execPath; } });
var exit_js_1 = require("./functions/exit.js");
Object.defineProperty(exports, "exit", { enumerable: true, get: function () { return exit_js_1.exit; } });
var fdatasync_js_1 = require("./functions/fdatasync.js");
Object.defineProperty(exports, "fdatasync", { enumerable: true, get: function () { return fdatasync_js_1.fdatasync; } });
var fdatasyncSync_js_1 = require("./functions/fdatasyncSync.js");
Object.defineProperty(exports, "fdatasyncSync", { enumerable: true, get: function () { return fdatasyncSync_js_1.fdatasyncSync; } });
var fstat_js_1 = require("./functions/fstat.js");
Object.defineProperty(exports, "fstat", { enumerable: true, get: function () { return fstat_js_1.fstat; } });
var fstatSync_js_1 = require("./functions/fstatSync.js");
Object.defineProperty(exports, "fstatSync", { enumerable: true, get: function () { return fstatSync_js_1.fstatSync; } });
var fsync_js_1 = require("./functions/fsync.js");
Object.defineProperty(exports, "fsync", { enumerable: true, get: function () { return fsync_js_1.fsync; } });
var fsyncSync_js_1 = require("./functions/fsyncSync.js");
Object.defineProperty(exports, "fsyncSync", { enumerable: true, get: function () { return fsyncSync_js_1.fsyncSync; } });
var ftruncate_js_1 = require("./functions/ftruncate.js");
Object.defineProperty(exports, "ftruncate", { enumerable: true, get: function () { return ftruncate_js_1.ftruncate; } });
var ftruncateSync_js_1 = require("./functions/ftruncateSync.js");
Object.defineProperty(exports, "ftruncateSync", { enumerable: true, get: function () { return ftruncateSync_js_1.ftruncateSync; } });
var gid_js_1 = require("./functions/gid.js");
Object.defineProperty(exports, "gid", { enumerable: true, get: function () { return gid_js_1.gid; } });
var hostname_js_1 = require("./functions/hostname.js");
Object.defineProperty(exports, "hostname", { enumerable: true, get: function () { return hostname_js_1.hostname; } });
var inspect_js_1 = require("./functions/inspect.js");
Object.defineProperty(exports, "inspect", { enumerable: true, get: function () { return inspect_js_1.inspect; } });
var kill_js_1 = require("./functions/kill.js");
Object.defineProperty(exports, "kill", { enumerable: true, get: function () { return kill_js_1.kill; } });
var link_js_1 = require("./functions/link.js");
Object.defineProperty(exports, "link", { enumerable: true, get: function () { return link_js_1.link; } });
var linkSync_js_1 = require("./functions/linkSync.js");
Object.defineProperty(exports, "linkSync", { enumerable: true, get: function () { return linkSync_js_1.linkSync; } });
var listen_js_1 = require("./functions/listen.js");
Object.defineProperty(exports, "listen", { enumerable: true, get: function () { return listen_js_1.listen; } });
var listenTls_js_1 = require("./functions/listenTls.js");
Object.defineProperty(exports, "listenTls", { enumerable: true, get: function () { return listenTls_js_1.listenTls; } });
var loadavg_js_1 = require("./functions/loadavg.js");
Object.defineProperty(exports, "loadavg", { enumerable: true, get: function () { return loadavg_js_1.loadavg; } });
var lstat_js_1 = require("./functions/lstat.js");
Object.defineProperty(exports, "lstat", { enumerable: true, get: function () { return lstat_js_1.lstat; } });
var lstatSync_js_1 = require("./functions/lstatSync.js");
Object.defineProperty(exports, "lstatSync", { enumerable: true, get: function () { return lstatSync_js_1.lstatSync; } });
var makeTempDir_js_1 = require("./functions/makeTempDir.js");
Object.defineProperty(exports, "makeTempDir", { enumerable: true, get: function () { return makeTempDir_js_1.makeTempDir; } });
var makeTempDirSync_js_1 = require("./functions/makeTempDirSync.js");
Object.defineProperty(exports, "makeTempDirSync", { enumerable: true, get: function () { return makeTempDirSync_js_1.makeTempDirSync; } });
var makeTempFile_js_1 = require("./functions/makeTempFile.js");
Object.defineProperty(exports, "makeTempFile", { enumerable: true, get: function () { return makeTempFile_js_1.makeTempFile; } });
var makeTempFileSync_js_1 = require("./functions/makeTempFileSync.js");
Object.defineProperty(exports, "makeTempFileSync", { enumerable: true, get: function () { return makeTempFileSync_js_1.makeTempFileSync; } });
var memoryUsage_js_1 = require("./functions/memoryUsage.js");
Object.defineProperty(exports, "memoryUsage", { enumerable: true, get: function () { return memoryUsage_js_1.memoryUsage; } });
var mkdir_js_1 = require("./functions/mkdir.js");
Object.defineProperty(exports, "mkdir", { enumerable: true, get: function () { return mkdir_js_1.mkdir; } });
var mkdirSync_js_1 = require("./functions/mkdirSync.js");
Object.defineProperty(exports, "mkdirSync", { enumerable: true, get: function () { return mkdirSync_js_1.mkdirSync; } });
var open_js_1 = require("./functions/open.js");
Object.defineProperty(exports, "open", { enumerable: true, get: function () { return open_js_1.open; } });
var openSync_js_1 = require("./functions/openSync.js");
Object.defineProperty(exports, "openSync", { enumerable: true, get: function () { return openSync_js_1.openSync; } });
var osRelease_js_1 = require("./functions/osRelease.js");
Object.defineProperty(exports, "osRelease", { enumerable: true, get: function () { return osRelease_js_1.osRelease; } });
var osUptime_js_1 = require("./functions/osUptime.js");
Object.defineProperty(exports, "osUptime", { enumerable: true, get: function () { return osUptime_js_1.osUptime; } });
var read_js_1 = require("./functions/read.js");
Object.defineProperty(exports, "read", { enumerable: true, get: function () { return read_js_1.read; } });
var readDir_js_1 = require("./functions/readDir.js");
Object.defineProperty(exports, "readDir", { enumerable: true, get: function () { return readDir_js_1.readDir; } });
var readDirSync_js_1 = require("./functions/readDirSync.js");
Object.defineProperty(exports, "readDirSync", { enumerable: true, get: function () { return readDirSync_js_1.readDirSync; } });
var readFile_js_1 = require("./functions/readFile.js");
Object.defineProperty(exports, "readFile", { enumerable: true, get: function () { return readFile_js_1.readFile; } });
var readFileSync_js_1 = require("./functions/readFileSync.js");
Object.defineProperty(exports, "readFileSync", { enumerable: true, get: function () { return readFileSync_js_1.readFileSync; } });
var readLink_js_1 = require("./functions/readLink.js");
Object.defineProperty(exports, "readLink", { enumerable: true, get: function () { return readLink_js_1.readLink; } });
var readLinkSync_js_1 = require("./functions/readLinkSync.js");
Object.defineProperty(exports, "readLinkSync", { enumerable: true, get: function () { return readLinkSync_js_1.readLinkSync; } });
var readSync_js_1 = require("./functions/readSync.js");
Object.defineProperty(exports, "readSync", { enumerable: true, get: function () { return readSync_js_1.readSync; } });
var readTextFile_js_1 = require("./functions/readTextFile.js");
Object.defineProperty(exports, "readTextFile", { enumerable: true, get: function () { return readTextFile_js_1.readTextFile; } });
var readTextFileSync_js_1 = require("./functions/readTextFileSync.js");
Object.defineProperty(exports, "readTextFileSync", { enumerable: true, get: function () { return readTextFileSync_js_1.readTextFileSync; } });
var realPath_js_1 = require("./functions/realPath.js");
Object.defineProperty(exports, "realPath", { enumerable: true, get: function () { return realPath_js_1.realPath; } });
var realPathSync_js_1 = require("./functions/realPathSync.js");
Object.defineProperty(exports, "realPathSync", { enumerable: true, get: function () { return realPathSync_js_1.realPathSync; } });
var remove_js_1 = require("./functions/remove.js");
Object.defineProperty(exports, "remove", { enumerable: true, get: function () { return remove_js_1.remove; } });
var removeSignalListener_js_1 = require("./functions/removeSignalListener.js");
Object.defineProperty(exports, "removeSignalListener", { enumerable: true, get: function () { return removeSignalListener_js_1.removeSignalListener; } });
var removeSync_js_1 = require("./functions/removeSync.js");
Object.defineProperty(exports, "removeSync", { enumerable: true, get: function () { return removeSync_js_1.removeSync; } });
var rename_js_1 = require("./functions/rename.js");
Object.defineProperty(exports, "rename", { enumerable: true, get: function () { return rename_js_1.rename; } });
var renameSync_js_1 = require("./functions/renameSync.js");
Object.defineProperty(exports, "renameSync", { enumerable: true, get: function () { return renameSync_js_1.renameSync; } });
var resolveDns_js_1 = require("./functions/resolveDns.js");
Object.defineProperty(exports, "resolveDns", { enumerable: true, get: function () { return resolveDns_js_1.resolveDns; } });
var run_js_1 = require("./functions/run.js");
Object.defineProperty(exports, "Process", { enumerable: true, get: function () { return run_js_1.Process; } });
Object.defineProperty(exports, "run", { enumerable: true, get: function () { return run_js_1.run; } });
var shutdown_js_1 = require("./functions/shutdown.js");
Object.defineProperty(exports, "shutdown", { enumerable: true, get: function () { return shutdown_js_1.shutdown; } });
var stat_js_1 = require("./functions/stat.js");
Object.defineProperty(exports, "stat", { enumerable: true, get: function () { return stat_js_1.stat; } });
var statSync_js_1 = require("./functions/statSync.js");
Object.defineProperty(exports, "statSync", { enumerable: true, get: function () { return statSync_js_1.statSync; } });
var symlink_js_1 = require("./functions/symlink.js");
Object.defineProperty(exports, "symlink", { enumerable: true, get: function () { return symlink_js_1.symlink; } });
var symlinkSync_js_1 = require("./functions/symlinkSync.js");
Object.defineProperty(exports, "symlinkSync", { enumerable: true, get: function () { return symlinkSync_js_1.symlinkSync; } });
var test_js_1 = require("./functions/test.js");
Object.defineProperty(exports, "test", { enumerable: true, get: function () { return test_js_1.test; } });
var truncate_js_1 = require("./functions/truncate.js");
Object.defineProperty(exports, "truncate", { enumerable: true, get: function () { return truncate_js_1.truncate; } });
var truncateSync_js_1 = require("./functions/truncateSync.js");
Object.defineProperty(exports, "truncateSync", { enumerable: true, get: function () { return truncateSync_js_1.truncateSync; } });
var uid_js_1 = require("./functions/uid.js");
Object.defineProperty(exports, "uid", { enumerable: true, get: function () { return uid_js_1.uid; } });
var watchFs_js_1 = require("./functions/watchFs.js");
Object.defineProperty(exports, "watchFs", { enumerable: true, get: function () { return watchFs_js_1.watchFs; } });
var write_js_1 = require("./functions/write.js");
Object.defineProperty(exports, "write", { enumerable: true, get: function () { return write_js_1.write; } });
var writeFile_js_1 = require("./functions/writeFile.js");
Object.defineProperty(exports, "writeFile", { enumerable: true, get: function () { return writeFile_js_1.writeFile; } });
var writeFileSync_js_1 = require("./functions/writeFileSync.js");
Object.defineProperty(exports, "writeFileSync", { enumerable: true, get: function () { return writeFileSync_js_1.writeFileSync; } });
var writeSync_js_1 = require("./functions/writeSync.js");
Object.defineProperty(exports, "writeSync", { enumerable: true, get: function () { return writeSync_js_1.writeSync; } });
var writeTextFile_js_1 = require("./functions/writeTextFile.js");
Object.defineProperty(exports, "writeTextFile", { enumerable: true, get: function () { return writeTextFile_js_1.writeTextFile; } });
var writeTextFileSync_js_1 = require("./functions/writeTextFileSync.js");
Object.defineProperty(exports, "writeTextFileSync", { enumerable: true, get: function () { return writeTextFileSync_js_1.writeTextFileSync; } });
var args_js_1 = require("./variables/args.js");
Object.defineProperty(exports, "args", { enumerable: true, get: function () { return args_js_1.args; } });
const futime = async function (rid, atime, mtime) {
    try {
        await new Promise((resolve, reject) => {
            // doesn't exist in fs.promises
            fs_1.default.futimes(rid, atime, mtime, (err) => {
                if (err) {
                    reject(err);
                }
                else {
                    resolve();
                }
            });
        });
    }
    catch (error) {
        throw (0, errorMap_js_1.default)(error);
    }
};
exports.futime = futime;
const futimeSync = function (rid, atime, mtime) {
    try {
        fs_1.default.futimesSync(rid, atime, mtime);
    }
    catch (error) {
        throw (0, errorMap_js_1.default)(error);
    }
};
exports.futimeSync = futimeSync;
const utime = async function (path, atime, mtime) {
    try {
        await fs_1.default.promises.utimes(path, atime, mtime);
    }
    catch (error) {
        if ((error === null || error === void 0 ? void 0 : error.code) === "ENOENT") {
            throw new variables_js_1.errors.NotFound(`No such file or directory (os error 2), utime '${path}'`);
        }
        throw (0, errorMap_js_1.default)(error);
    }
};
exports.utime = utime;
const utimeSync = function (path, atime, mtime) {
    try {
        fs_1.default.utimesSync(path, atime, mtime);
    }
    catch (error) {
        if ((error === null || error === void 0 ? void 0 : error.code) === "ENOENT") {
            throw new variables_js_1.errors.NotFound(`No such file or directory (os error 2), utime '${path}'`);
        }
        throw (0, errorMap_js_1.default)(error);
    }
};
exports.utimeSync = utimeSync;
