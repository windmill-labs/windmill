import { fromArrayBuffer } from "@smithy/util-buffer-from";
export const toBase64 = (input) => fromArrayBuffer(input.buffer, input.byteOffset, input.byteLength).toString("base64");
