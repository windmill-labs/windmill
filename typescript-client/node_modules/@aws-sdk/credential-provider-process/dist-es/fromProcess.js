import { getProfileName, parseKnownFiles } from "@smithy/shared-ini-file-loader";
import { resolveProcessCredentials } from "./resolveProcessCredentials";
export const fromProcess = (init = {}) => async () => {
    const profiles = await parseKnownFiles(init);
    return resolveProcessCredentials(getProfileName(init), profiles);
};
