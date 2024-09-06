import { isAsyncGenerator, isGenerator } from '../external/check-iterable/index.js';
import { unwrapChannel } from './channel.js';
import { resolveModule } from './module.js';
import { isPlainObject } from '../object.js';
import { fromObject, isDOMException, isAggregateError, toObject } from '../error.js';
import Exception from '../error/Exception.js';

const pendingTasks = new Map();
/**
 * For some reason, in Node.js and Bun, when import expression throws an
 * module/package not found error, the error can not be serialized and sent to
 * the other thread properly. We need to check this situation and sent the error
 * as plain object instead.
 */
function isModuleResolveError(value) {
    var _a;
    if (typeof value === "object" &&
        typeof (value === null || value === void 0 ? void 0 : value.message) === "string" &&
        /Cannot find (module|package)/.test(value === null || value === void 0 ? void 0 : value.message)) {
        return (value instanceof Error) // Node.js (possibly bug)
            || ((_a = value.constructor) === null || _a === void 0 ? void 0 : _a.name) === "Error"; // Bun (doesn't inherit from Error)
    }
    return false;
}
function removeUnserializableProperties(obj) {
    const _obj = {};
    for (const key of Reflect.ownKeys(obj)) {
        if (typeof obj[key] !== "bigint" && typeof obj[key] !== "function") {
            _obj[key] = obj[key];
        }
    }
    return _obj;
}
function unwrapArgs(args, channelWrite) {
    return args.map(arg => {
        if (isPlainObject(arg)) {
            if (arg["@@type"] === "Channel" && typeof arg["@@id"] === "number") {
                return unwrapChannel(arg, channelWrite);
            }
            else if (arg["@@type"] === "Exception"
                || arg["@@type"] === "DOMException"
                || arg["@@type"] === "AggregateError") {
                return fromObject(arg);
            }
        }
        return arg;
    });
}
function wrapReturnValue(value) {
    const transferable = [];
    if (value instanceof ArrayBuffer) {
        transferable.push(value);
    }
    else if ((value instanceof Exception)
        || isDOMException(value)
        || isAggregateError(value)
        || isModuleResolveError(value)) {
        value = toObject(value);
    }
    else if (isPlainObject(value)) {
        for (const key of Object.getOwnPropertyNames(value)) {
            const _value = value[key];
            if (_value instanceof ArrayBuffer) {
                transferable.push(_value);
            }
            else if ((_value instanceof Exception)
                || isDOMException(_value)
                || isAggregateError(_value)
                || isModuleResolveError(_value)) {
                value[key] = toObject(_value);
            }
        }
    }
    else if (Array.isArray(value)) {
        value = value.map(item => {
            if (item instanceof ArrayBuffer) {
                transferable.push(item);
                return item;
            }
            else if ((item instanceof Exception)
                || isDOMException(item)
                || isAggregateError(item)
                || isModuleResolveError(item)) {
                return toObject(item);
            }
            else {
                return item;
            }
        });
    }
    return { value, transferable };
}
/**
 * @ignore
 * @internal
 */
function isCallRequest(msg) {
    return msg && typeof msg === "object"
        && ((msg.type === "call" && typeof msg.module === "string" && typeof msg.fn === "string") ||
            (["next", "return", "throw"].includes(msg.type) && typeof msg.taskId === "number"))
        && Array.isArray(msg.args);
}
/**
 * @ignore
 * @internal
 */
async function handleCallRequest(msg, reply) {
    const _reply = reply;
    reply = (res) => {
        if (res.type === "error") {
            if ((res.error instanceof Exception) ||
                isDOMException(res.error) ||
                isAggregateError(res.error) ||
                isModuleResolveError(res.error)) {
                return _reply({
                    ...res,
                    error: removeUnserializableProperties(toObject(res.error)),
                });
            }
            try {
                return _reply(res);
            }
            catch (_a) {
                // In case the error cannot be cloned directly, fallback to
                // transferring it as an object and rebuild in the main thread.
                return _reply({
                    ...res,
                    error: removeUnserializableProperties(toObject(res.error)),
                });
            }
        }
        else {
            return _reply(res);
        }
    };
    msg.args = unwrapArgs(msg.args, (type, msg, channelId) => {
        reply({ type, value: msg, channelId });
    });
    try {
        if (msg.taskId && ["next", "return", "throw"].includes(msg.type)) {
            const req = msg;
            const task = pendingTasks.get(req.taskId);
            if (task) {
                if (req.type === "throw") {
                    try {
                        await task.throw(req.args[0]);
                    }
                    catch (error) {
                        reply({ type: "error", error, taskId: req.taskId });
                    }
                }
                else if (req.type === "return") {
                    try {
                        const res = await task.return(req.args[0]);
                        const { value, transferable } = wrapReturnValue(res.value);
                        reply({
                            type: "yield",
                            value,
                            done: res.done,
                            taskId: req.taskId,
                        }, transferable);
                    }
                    catch (error) {
                        reply({ type: "error", error, taskId: req.taskId });
                    }
                }
                else { // req.type === "next"
                    try {
                        const res = await task.next(req.args[0]);
                        const { value, transferable } = wrapReturnValue(res.value);
                        reply({
                            type: "yield",
                            value,
                            done: res.done,
                            taskId: req.taskId,
                        }, transferable);
                    }
                    catch (error) {
                        reply({ type: "error", error, taskId: req.taskId });
                    }
                }
            }
            else {
                reply({
                    type: "error",
                    error: new ReferenceError(`task (${req.taskId}) doesn't exists`),
                    taskId: req.taskId,
                });
            }
            return;
        }
        const req = msg;
        const module = await resolveModule(req.module);
        const returns = await module[req.fn](...req.args);
        if (isAsyncGenerator(returns) || isGenerator(returns)) {
            if (req.taskId) {
                pendingTasks.set(req.taskId, returns);
                reply({ type: "gen", taskId: req.taskId });
            }
            else {
                while (true) {
                    try {
                        const res = await returns.next();
                        const { value, transferable } = wrapReturnValue(res.value);
                        reply({ type: "yield", value, done: res.done }, transferable);
                        if (res.done) {
                            break;
                        }
                    }
                    catch (error) {
                        reply({ type: "error", error });
                        break;
                    }
                }
            }
        }
        else {
            const { value, transferable } = wrapReturnValue(returns);
            reply({ type: "return", value, taskId: req.taskId }, transferable);
        }
    }
    catch (error) {
        reply({ type: "error", error, taskId: msg.taskId });
    }
}

export { handleCallRequest, isCallRequest };
//# sourceMappingURL=worker.js.map
