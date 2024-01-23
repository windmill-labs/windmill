import { Transform } from "stream";
export function createReadStreamOnBuffer(buffer) {
    const stream = new Transform();
    stream.push(buffer);
    stream.push(null);
    return stream;
}
