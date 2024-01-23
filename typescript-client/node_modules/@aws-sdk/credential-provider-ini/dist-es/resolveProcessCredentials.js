import { fromProcess } from "@aws-sdk/credential-provider-process";
export const isProcessProfile = (arg) => Boolean(arg) && typeof arg === "object" && typeof arg.credential_process === "string";
export const resolveProcessCredentials = async (options, profile) => fromProcess({
    ...options,
    profile,
})();
