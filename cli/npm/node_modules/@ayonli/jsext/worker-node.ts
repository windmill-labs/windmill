import { isMainThread, parentPort } from "worker_threads";
import { isNode } from "./env.ts";
import { isChannelMessage, handleChannelMessage } from "./parallel/channel.ts";
import { isCallRequest, handleCallRequest } from "./parallel/worker.ts";

if (isNode) {
    if (!isMainThread && parentPort) {
        parentPort.on("message", async (msg) => {
            if (isCallRequest(msg)) {
                await handleCallRequest(msg, (res, transferable = []) => {
                    parentPort!.postMessage(res, transferable);
                });
            } else if (isChannelMessage(msg)) {
                await handleChannelMessage(msg);
            }
        });
    } else if (process.send) {
        // notify the parent process that the worker is ready;
        process.send("ready");
        process.on("message", async (msg) => {
            if (isCallRequest(msg)) {
                await handleCallRequest(msg, (res, _ = []) => {
                    process.send!(res);
                });
            } else if (isChannelMessage(msg)) {
                await handleChannelMessage(msg);
            }
        });
    }
}
