"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.WriteZero = exports.UnexpectedEof = exports.TimedOut = exports.PermissionDenied = exports.NotFound = exports.NotConnected = exports.InvalidData = exports.Interrupted = exports.Http = exports.ConnectionReset = exports.ConnectionRefused = exports.ConnectionAborted = exports.Busy = exports.BrokenPipe = exports.BadResource = exports.AlreadyExists = exports.AddrNotAvailable = exports.AddrInUse = void 0;
// please keep sorted
class AddrInUse extends Error {
}
exports.AddrInUse = AddrInUse;
class AddrNotAvailable extends Error {
}
exports.AddrNotAvailable = AddrNotAvailable;
class AlreadyExists extends Error {
}
exports.AlreadyExists = AlreadyExists;
class BadResource extends Error {
}
exports.BadResource = BadResource;
class BrokenPipe extends Error {
}
exports.BrokenPipe = BrokenPipe;
class Busy extends Error {
}
exports.Busy = Busy;
class ConnectionAborted extends Error {
}
exports.ConnectionAborted = ConnectionAborted;
class ConnectionRefused extends Error {
}
exports.ConnectionRefused = ConnectionRefused;
class ConnectionReset extends Error {
}
exports.ConnectionReset = ConnectionReset;
class Http extends Error {
}
exports.Http = Http;
class Interrupted extends Error {
}
exports.Interrupted = Interrupted;
class InvalidData extends Error {
}
exports.InvalidData = InvalidData;
class NotConnected extends Error {
}
exports.NotConnected = NotConnected;
class NotFound extends Error {
    constructor() {
        super(...arguments);
        this.code = "ENOENT";
    }
}
exports.NotFound = NotFound;
class PermissionDenied extends Error {
}
exports.PermissionDenied = PermissionDenied;
class TimedOut extends Error {
}
exports.TimedOut = TimedOut;
class UnexpectedEof extends Error {
}
exports.UnexpectedEof = UnexpectedEof;
class WriteZero extends Error {
}
exports.WriteZero = WriteZero;
