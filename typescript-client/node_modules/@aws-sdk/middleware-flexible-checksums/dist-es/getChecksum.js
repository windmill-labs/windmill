import { isStreaming } from "./isStreaming";
import { stringHasher } from "./stringHasher";
export const getChecksum = async (body, { streamHasher, checksumAlgorithmFn, base64Encoder }) => {
    const digest = isStreaming(body) ? streamHasher(checksumAlgorithmFn, body) : stringHasher(checksumAlgorithmFn, body);
    return base64Encoder(await digest);
};
