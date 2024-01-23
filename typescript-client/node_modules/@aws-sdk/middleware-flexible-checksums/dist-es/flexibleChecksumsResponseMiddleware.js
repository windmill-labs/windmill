import { HttpRequest } from "@smithy/protocol-http";
import { getChecksumAlgorithmListForResponse } from "./getChecksumAlgorithmListForResponse";
import { getChecksumLocationName } from "./getChecksumLocationName";
import { isChecksumWithPartNumber } from "./isChecksumWithPartNumber";
import { isStreaming } from "./isStreaming";
import { createReadStreamOnBuffer } from "./streams/create-read-stream-on-buffer";
import { validateChecksumFromResponse } from "./validateChecksumFromResponse";
export const flexibleChecksumsResponseMiddlewareOptions = {
    name: "flexibleChecksumsResponseMiddleware",
    toMiddleware: "deserializerMiddleware",
    relation: "after",
    tags: ["BODY_CHECKSUM"],
    override: true,
};
export const flexibleChecksumsResponseMiddleware = (config, middlewareConfig) => (next, context) => async (args) => {
    if (!HttpRequest.isInstance(args.request)) {
        return next(args);
    }
    const input = args.input;
    const result = await next(args);
    const response = result.response;
    let collectedStream = undefined;
    const { requestValidationModeMember, responseAlgorithms } = middlewareConfig;
    if (requestValidationModeMember && input[requestValidationModeMember] === "ENABLED") {
        const { clientName, commandName } = context;
        const isS3WholeObjectMultipartGetResponseChecksum = clientName === "S3Client" &&
            commandName === "GetObjectCommand" &&
            getChecksumAlgorithmListForResponse(responseAlgorithms).every((algorithm) => {
                const responseHeader = getChecksumLocationName(algorithm);
                const checksumFromResponse = response.headers[responseHeader];
                return !checksumFromResponse || isChecksumWithPartNumber(checksumFromResponse);
            });
        if (isS3WholeObjectMultipartGetResponseChecksum) {
            return result;
        }
        const isStreamingBody = isStreaming(response.body);
        if (isStreamingBody) {
            collectedStream = await config.streamCollector(response.body);
            response.body = createReadStreamOnBuffer(collectedStream);
        }
        await validateChecksumFromResponse(result.response, {
            config,
            responseAlgorithms,
        });
        if (isStreamingBody && collectedStream) {
            response.body = createReadStreamOnBuffer(collectedStream);
        }
    }
    return result;
};
