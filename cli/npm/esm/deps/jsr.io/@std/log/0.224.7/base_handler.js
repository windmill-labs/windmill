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
var _BaseHandler_levelName, _BaseHandler_level;
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { getLevelByName, getLevelName, } from "./levels.js";
const DEFAULT_FORMATTER = ({ levelName, msg }) => `${levelName} ${msg}`;
export class BaseHandler {
    constructor(levelName, options) {
        _BaseHandler_levelName.set(this, void 0);
        _BaseHandler_level.set(this, void 0);
        Object.defineProperty(this, "formatter", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        const { formatter = DEFAULT_FORMATTER } = options ?? {};
        __classPrivateFieldSet(this, _BaseHandler_levelName, levelName, "f");
        __classPrivateFieldSet(this, _BaseHandler_level, getLevelByName(levelName), "f");
        this.formatter = formatter;
    }
    get level() {
        return __classPrivateFieldGet(this, _BaseHandler_level, "f");
    }
    set level(level) {
        __classPrivateFieldSet(this, _BaseHandler_level, level, "f");
        __classPrivateFieldSet(this, _BaseHandler_levelName, getLevelName(level), "f");
    }
    get levelName() {
        return __classPrivateFieldGet(this, _BaseHandler_levelName, "f");
    }
    set levelName(levelName) {
        __classPrivateFieldSet(this, _BaseHandler_levelName, levelName, "f");
        __classPrivateFieldSet(this, _BaseHandler_level, getLevelByName(levelName), "f");
    }
    handle(logRecord) {
        if (this.level > logRecord.level)
            return;
        const msg = this.format(logRecord);
        this.log(msg);
    }
    format(logRecord) {
        return this.formatter(logRecord);
    }
    setup() { }
    destroy() { }
    [(_BaseHandler_levelName = new WeakMap(), _BaseHandler_level = new WeakMap(), Symbol.dispose)]() {
        this.destroy();
    }
}
