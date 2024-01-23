/// <reference types="node" />
import { Readable } from "stream";
/**
 * Convert a buffer to a readable stream.
 */
export declare function createReadStreamOnBuffer(buffer: Uint8Array): Readable;
