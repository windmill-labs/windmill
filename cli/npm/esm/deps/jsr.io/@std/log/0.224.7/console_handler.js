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
var _ConsoleHandler_useColors;
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { LogLevels } from "./levels.js";
import { blue, bold, red, yellow } from "../../fmt/1.0.2/colors.js";
import { BaseHandler } from "./base_handler.js";
/**
 * This is the default logger. It will output color coded log messages to the
 * console via `console.log()`.
 */
export class ConsoleHandler extends BaseHandler {
    constructor(levelName, options = {}) {
        super(levelName, options);
        _ConsoleHandler_useColors.set(this, void 0);
        __classPrivateFieldSet(this, _ConsoleHandler_useColors, options.useColors ?? true, "f");
    }
    format(logRecord) {
        let msg = super.format(logRecord);
        if (__classPrivateFieldGet(this, _ConsoleHandler_useColors, "f")) {
            msg = this.applyColors(msg, logRecord.level);
        }
        return msg;
    }
    applyColors(msg, level) {
        switch (level) {
            case LogLevels.INFO:
                msg = blue(msg);
                break;
            case LogLevels.WARN:
                msg = yellow(msg);
                break;
            case LogLevels.ERROR:
                msg = red(msg);
                break;
            case LogLevels.CRITICAL:
                msg = bold(red(msg));
                break;
            default:
                break;
        }
        return msg;
    }
    log(msg) {
        console.log(msg);
    }
}
_ConsoleHandler_useColors = new WeakMap();
