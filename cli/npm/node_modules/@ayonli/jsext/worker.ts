/**
 * This module is only used internally by the `parallel()` function to spawn
 * workers, DON'T use it in your own code.
 * @internal
 * @module
 */

import { isNode, isBun } from "./env.ts";
import { handleChannelMessage, isChannelMessage } from "./parallel/channel.ts";
import { handleCallRequest, isCallRequest } from "./parallel/worker.ts";

declare var Bun: any;

if (isBun
    && Bun.isMainThread
    && typeof process === "object"
    && typeof process.send === "function"
) { // Bun with child_process
    process.send("ready"); // notify the parent process that the worker is ready;
    process.on("message", async (msg) => {
        if (isCallRequest(msg)) {
            await handleCallRequest(msg, (res, _ = []) => {
                process.send!(res);
            });
        } else if (isChannelMessage(msg)) {
            await handleChannelMessage(msg);
        }
    });
} else if (!isNode && typeof self === "object") {
    self.onmessage = async ({ data: msg }) => {
        if (isCallRequest(msg)) {
            await handleCallRequest(msg, (res, transferable = []) => {
                self.postMessage(res, { transfer: transferable });
            });
        } else if (isChannelMessage(msg)) {
            await handleChannelMessage(msg);
        }
    };
}
