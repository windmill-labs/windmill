import { SignatureV4S3Express } from "@aws-sdk/middleware-sdk-s3";
import { signatureV4CrtContainer } from "./signature-v4-crt-container";
export class SignatureV4MultiRegion {
    constructor(options) {
        this.sigv4Signer = new SignatureV4S3Express(options);
        this.signerOptions = options;
    }
    async sign(requestToSign, options = {}) {
        if (options.signingRegion === "*") {
            if (this.signerOptions.runtime !== "node")
                throw new Error("This request requires signing with SigV4Asymmetric algorithm. It's only available in Node.js");
            return this.getSigv4aSigner().sign(requestToSign, options);
        }
        return this.sigv4Signer.sign(requestToSign, options);
    }
    async signWithCredentials(requestToSign, credentials, options = {}) {
        if (options.signingRegion === "*") {
            if (this.signerOptions.runtime !== "node")
                throw new Error("This request requires signing with SigV4Asymmetric algorithm. It's only available in Node.js");
            return this.getSigv4aSigner().signWithCredentials(requestToSign, credentials, options);
        }
        return this.sigv4Signer.signWithCredentials(requestToSign, credentials, options);
    }
    async presign(originalRequest, options = {}) {
        if (options.signingRegion === "*") {
            if (this.signerOptions.runtime !== "node")
                throw new Error("This request requires signing with SigV4Asymmetric algorithm. It's only available in Node.js");
            return this.getSigv4aSigner().presign(originalRequest, options);
        }
        return this.sigv4Signer.presign(originalRequest, options);
    }
    async presignWithCredentials(originalRequest, credentials, options = {}) {
        if (options.signingRegion === "*") {
            throw new Error("Method presignWithCredentials is not supported for [signingRegion=*].");
        }
        return this.sigv4Signer.presignWithCredentials(originalRequest, credentials, options);
    }
    getSigv4aSigner() {
        if (!this.sigv4aSigner) {
            let CrtSignerV4 = null;
            try {
                CrtSignerV4 = signatureV4CrtContainer.CrtSignerV4;
                if (typeof CrtSignerV4 !== "function")
                    throw new Error();
            }
            catch (e) {
                e.message =
                    `${e.message}\n` +
                        `Please check whether you have installed the "@aws-sdk/signature-v4-crt" package explicitly. \n` +
                        `You must also register the package by calling [require("@aws-sdk/signature-v4-crt");] ` +
                        `or an ESM equivalent such as [import "@aws-sdk/signature-v4-crt";]. \n` +
                        "For more information please go to " +
                        "https://github.com/aws/aws-sdk-js-v3#functionality-requiring-aws-common-runtime-crt";
                throw e;
            }
            this.sigv4aSigner = new CrtSignerV4({
                ...this.signerOptions,
                signingAlgorithm: 1,
            });
        }
        return this.sigv4aSigner;
    }
}
