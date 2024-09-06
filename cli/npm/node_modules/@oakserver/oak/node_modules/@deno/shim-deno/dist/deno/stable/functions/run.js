"use strict";
/// <reference path="../lib.deno.d.ts" />
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
var __classPrivateFieldSet = (this && this.__classPrivateFieldSet) || function (receiver, state, value, kind, f) {
    if (kind === "m") throw new TypeError("Private method is not writable");
    if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a setter");
    if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot write private member to an object whose class did not declare it");
    return (kind === "a" ? f.call(receiver, value) : f ? f.value = value : state.set(receiver, value)), value;
};
var __classPrivateFieldGet = (this && this.__classPrivateFieldGet) || function (receiver, state, kind, f) {
    if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a getter");
    if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot read private member from an object whose class did not declare it");
    return kind === "m" ? f : kind === "a" ? f.call(receiver) : f ? f.value : state.get(receiver);
};
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
var _Process_process, _Process_stderr, _Process_stdout, _Process_stdin, _Process_status, _Process_receivedStatus, _ProcessReadStream_stream, _ProcessReadStream_bufferStreamReader, _ProcessReadStream_closed, _ProcessWriteStream_stream, _ProcessWriteStream_streamWriter, _ProcessWriteStream_closed;
Object.defineProperty(exports, "__esModule", { value: true });
exports.Process = exports.run = void 0;
const child_process_1 = __importDefault(require("child_process"));
const fs_1 = __importDefault(require("fs"));
const os_1 = __importDefault(require("os"));
const url_1 = __importDefault(require("url"));
const events_1 = require("events");
const which_1 = __importDefault(require("which"));
const streams_js_1 = require("../../internal/streams.js");
const errors = __importStar(require("../variables/errors.js"));
const run = function run(options) {
    const [cmd, ...args] = options.cmd;
    if (options.cwd && !fs_1.default.existsSync(options.cwd)) {
        throw new Error("The directory name is invalid.");
    }
    // childProcess.spawn will asynchronously check if the command exists, but
    // we need to do this synchronously
    const commandName = getCmd(cmd);
    if (!which_1.default.sync(commandName, { nothrow: true })) {
        throw new errors.NotFound("The system cannot find the file specified.");
    }
    const process = child_process_1.default.spawn(commandName, args, {
        cwd: options.cwd,
        env: getEnv(options),
        uid: options.uid,
        gid: options.gid,
        shell: false,
        stdio: [
            getStdio(options.stdin, "in"),
            getStdio(options.stdout, "out"),
            getStdio(options.stderr, "out"),
        ],
    });
    return new Process(process);
};
exports.run = run;
function getStdio(value, kind) {
    if (value === "inherit" || value == null) {
        return "inherit"; // default
    }
    else if (value === "piped") {
        return "pipe";
    }
    else if (value === "null") {
        return "ignore";
    }
    else if (typeof value === "number") {
        switch (kind) {
            case "in":
                return fs_1.default.createReadStream(null, { fd: value });
            case "out":
                return fs_1.default.createWriteStream(null, { fd: value });
            default: {
                const _assertNever = kind;
                throw new Error("Unreachable.");
            }
        }
    }
    else {
        const _assertNever = value;
        throw new Error("Unknown value.");
    }
}
function getCmd(firstArg) {
    if (firstArg instanceof URL) {
        return url_1.default.fileURLToPath(firstArg);
    }
    else {
        return firstArg;
    }
}
function getEnv(options) {
    var _a;
    const env = (_a = options.env) !== null && _a !== void 0 ? _a : {};
    for (const name in process.env) {
        if (!Object.prototype.hasOwnProperty.call(env, name)) {
            if (options.clearEnv) {
                if (os_1.default.platform() === "win32") {
                    env[name] = "";
                }
                else {
                    delete env[name];
                }
            }
            else {
                env[name] = process.env[name];
            }
        }
    }
    return env;
}
class Process {
    /** @internal */
    constructor(process) {
        var _a, _b, _c;
        _Process_process.set(this, void 0);
        _Process_stderr.set(this, void 0);
        _Process_stdout.set(this, void 0);
        _Process_stdin.set(this, void 0);
        _Process_status.set(this, void 0);
        _Process_receivedStatus.set(this, false);
        __classPrivateFieldSet(this, _Process_process, process, "f");
        __classPrivateFieldSet(this, _Process_stdout, (_a = ProcessReadStream.fromNullable(__classPrivateFieldGet(this, _Process_process, "f").stdout)) !== null && _a !== void 0 ? _a : null, "f");
        __classPrivateFieldSet(this, _Process_stderr, (_b = ProcessReadStream.fromNullable(__classPrivateFieldGet(this, _Process_process, "f").stderr)) !== null && _b !== void 0 ? _b : null, "f");
        __classPrivateFieldSet(this, _Process_stdin, (_c = ProcessWriteStream.fromNullable(__classPrivateFieldGet(this, _Process_process, "f").stdin)) !== null && _c !== void 0 ? _c : null, "f");
        __classPrivateFieldSet(this, _Process_status, (0, events_1.once)(process, "exit"), "f");
    }
    get rid() {
        // todo: useful to return something?
        return NaN;
    }
    get pid() {
        // only undefined when the process doesn't spawn, in which case this
        // will never be reached
        return __classPrivateFieldGet(this, _Process_process, "f").pid;
    }
    get stdin() {
        return __classPrivateFieldGet(this, _Process_stdin, "f");
    }
    get stdout() {
        return __classPrivateFieldGet(this, _Process_stdout, "f");
    }
    get stderr() {
        return __classPrivateFieldGet(this, _Process_stderr, "f");
    }
    async status() {
        const [receivedCode, signalName] = await __classPrivateFieldGet(this, _Process_status, "f");
        // when there is a signal, the exit code is 128 + signal code
        const signal = signalName
            ? os_1.default.constants.signals[signalName]
            : receivedCode > 128
                ? receivedCode - 128
                : undefined;
        const code = receivedCode != null
            ? receivedCode
            : signal != null
                ? 128 + signal
                : undefined;
        const success = code === 0;
        __classPrivateFieldSet(this, _Process_receivedStatus, true, "f");
        return { code, signal, success };
    }
    async output() {
        if (!__classPrivateFieldGet(this, _Process_stdout, "f")) {
            throw new TypeError("stdout was not piped");
        }
        const result = await __classPrivateFieldGet(this, _Process_stdout, "f").readAll();
        __classPrivateFieldGet(this, _Process_stdout, "f").close();
        return result;
    }
    async stderrOutput() {
        if (!__classPrivateFieldGet(this, _Process_stderr, "f")) {
            throw new TypeError("stderr was not piped");
        }
        const result = await __classPrivateFieldGet(this, _Process_stderr, "f").readAll();
        __classPrivateFieldGet(this, _Process_stderr, "f").close();
        return result;
    }
    close() {
        // Deno doesn't close any stdio streams here
        __classPrivateFieldGet(this, _Process_process, "f").unref();
        __classPrivateFieldGet(this, _Process_process, "f").kill();
    }
    kill(signo = "SIGTERM") {
        if (__classPrivateFieldGet(this, _Process_receivedStatus, "f")) {
            throw new errors.NotFound("entity not found");
        }
        __classPrivateFieldGet(this, _Process_process, "f").kill(signo);
    }
}
exports.Process = Process;
_Process_process = new WeakMap(), _Process_stderr = new WeakMap(), _Process_stdout = new WeakMap(), _Process_stdin = new WeakMap(), _Process_status = new WeakMap(), _Process_receivedStatus = new WeakMap();
class ProcessReadStream {
    constructor(stream) {
        _ProcessReadStream_stream.set(this, void 0);
        _ProcessReadStream_bufferStreamReader.set(this, void 0);
        _ProcessReadStream_closed.set(this, false);
        __classPrivateFieldSet(this, _ProcessReadStream_stream, stream, "f");
        __classPrivateFieldSet(this, _ProcessReadStream_bufferStreamReader, new streams_js_1.BufferStreamReader(stream), "f");
    }
    static fromNullable(stream) {
        return stream ? new ProcessReadStream(stream) : undefined;
    }
    readAll() {
        if (__classPrivateFieldGet(this, _ProcessReadStream_closed, "f")) {
            return Promise.resolve(new Uint8Array(0));
        }
        else {
            return __classPrivateFieldGet(this, _ProcessReadStream_bufferStreamReader, "f").readAll();
        }
    }
    read(p) {
        if (__classPrivateFieldGet(this, _ProcessReadStream_closed, "f")) {
            return Promise.resolve(null);
        }
        else {
            return __classPrivateFieldGet(this, _ProcessReadStream_bufferStreamReader, "f").read(p);
        }
    }
    close() {
        __classPrivateFieldSet(this, _ProcessReadStream_closed, true, "f");
        __classPrivateFieldGet(this, _ProcessReadStream_stream, "f").destroy();
    }
    get readable() {
        throw new Error("Not implemented.");
    }
    get writable() {
        throw new Error("Not implemented.");
    }
}
_ProcessReadStream_stream = new WeakMap(), _ProcessReadStream_bufferStreamReader = new WeakMap(), _ProcessReadStream_closed = new WeakMap();
class ProcessWriteStream {
    constructor(stream) {
        _ProcessWriteStream_stream.set(this, void 0);
        _ProcessWriteStream_streamWriter.set(this, void 0);
        _ProcessWriteStream_closed.set(this, false);
        __classPrivateFieldSet(this, _ProcessWriteStream_stream, stream, "f");
        __classPrivateFieldSet(this, _ProcessWriteStream_streamWriter, new streams_js_1.StreamWriter(stream), "f");
    }
    static fromNullable(stream) {
        return stream ? new ProcessWriteStream(stream) : undefined;
    }
    write(p) {
        if (__classPrivateFieldGet(this, _ProcessWriteStream_closed, "f")) {
            return Promise.resolve(0);
        }
        else {
            return __classPrivateFieldGet(this, _ProcessWriteStream_streamWriter, "f").write(p);
        }
    }
    close() {
        __classPrivateFieldSet(this, _ProcessWriteStream_closed, true, "f");
        __classPrivateFieldGet(this, _ProcessWriteStream_stream, "f").end();
    }
}
_ProcessWriteStream_stream = new WeakMap(), _ProcessWriteStream_streamWriter = new WeakMap(), _ProcessWriteStream_closed = new WeakMap();
