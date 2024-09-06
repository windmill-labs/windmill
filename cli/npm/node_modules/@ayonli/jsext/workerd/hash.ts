import { text } from "../bytes.ts";
import {
    type BufferSource,
    type DataSource,
    hash,
    hmac,
    toBytes,
    sha1,
    sha256,
    sha512,
} from "../hash/web.ts";

export type { BufferSource, DataSource };

export default hash;
export { hmac, sha1, sha256, sha512 };

export function md5(data: DataSource): Promise<ArrayBuffer>;
export function md5(data: DataSource, encoding: "hex" | "base64"): Promise<string>;
export async function md5(
    data: DataSource,
    encoding: "hex" | "base64" | undefined = undefined
): Promise<string | ArrayBuffer> {
    let bytes = await toBytes(data);
    const hash = await crypto.subtle.digest("MD5", bytes);

    if (encoding === "hex") {
        return text(new Uint8Array(hash), "hex");
    } else if (encoding === "base64") {
        return text(new Uint8Array(hash), "base64");
    } else {
        return hash;
    }
}
