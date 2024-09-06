"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.metrics = void 0;
const metrics = function metrics() {
    return {
        opsDispatched: 0,
        opsDispatchedSync: 0,
        opsDispatchedAsync: 0,
        opsDispatchedAsyncUnref: 0,
        opsCompleted: 0,
        opsCompletedSync: 0,
        opsCompletedAsync: 0,
        opsCompletedAsyncUnref: 0,
        bytesSentControl: 0,
        bytesSentData: 0,
        bytesReceived: 0,
        ops: {},
    };
};
exports.metrics = metrics;
