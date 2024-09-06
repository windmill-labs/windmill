"use strict";
// getAccessFlag and getCreationFlag adapted from the original in Rust's std::fs
// <source path="https://github.com/rust-lang/rust/blob/304441960e7058fe97f09ef00b20739b4dc56d11/library/std/src/sys/unix/fs.rs#L694-L728" />
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
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.getFsFlag = exports.getCreationFlag = exports.getAccessFlag = void 0;
const errors = __importStar(require("../stable/variables/errors.js"));
const fs_1 = require("fs");
const os_1 = __importDefault(require("os"));
const { O_APPEND, O_CREAT, O_EXCL, O_RDONLY, O_RDWR, O_TRUNC, O_WRONLY } = fs_1.constants;
function getAccessFlag(opts) {
    if (opts.read && !opts.write && !opts.append)
        return O_RDONLY;
    if (!opts.read && opts.write && !opts.append)
        return O_WRONLY;
    if (opts.read && opts.write && !opts.append)
        return O_RDWR;
    if (!opts.read && opts.append)
        return O_WRONLY | O_APPEND;
    if (opts.read && opts.append)
        return O_RDWR | O_APPEND;
    if (!opts.read && !opts.write && !opts.append) {
        throw new errors.BadResource("EINVAL: One of 'read', 'write', 'append' is required to open file.");
    }
    throw new errors.BadResource("EINVAL: Invalid fs flags.");
}
exports.getAccessFlag = getAccessFlag;
function getCreationFlag(opts) {
    if (!opts.write && !opts.append) {
        if (opts.truncate || opts.create || opts.createNew) {
            throw new errors.BadResource("EINVAL: One of 'write', 'append' is required to 'truncate', 'create' or 'createNew' file.");
        }
    }
    if (opts.append) {
        if (opts.truncate && !opts.createNew) {
            throw new errors.BadResource("EINVAL: unexpected 'truncate': true and 'createNew': false when 'append' is true.");
        }
    }
    if (!opts.create && !opts.truncate && !opts.createNew)
        return 0;
    if (opts.create && !opts.truncate && !opts.createNew)
        return O_CREAT;
    if (!opts.create && opts.truncate && !opts.createNew) {
        if (os_1.default.platform() === "win32") {
            // for some reason only providing O_TRUNC on windows will
            // throw a "EINVAL: invalid argument", so to work around this
            // we relax the restriction here to also create the file if it
            // doesn't exist
            return O_CREAT | O_TRUNC;
        }
        else {
            return O_TRUNC;
        }
    }
    if (opts.create && opts.truncate && !opts.createNew) {
        return O_CREAT | O_TRUNC;
    }
    if (opts.createNew)
        return O_CREAT | O_EXCL;
    throw new errors.BadResource("EINVAL: Invalid fs flags.");
}
exports.getCreationFlag = getCreationFlag;
function getFsFlag(flags) {
    return getAccessFlag(flags) | getCreationFlag(flags);
}
exports.getFsFlag = getFsFlag;
