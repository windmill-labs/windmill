import { TokenProviderError } from "@smithy/property-provider";
export const fromStatic = ({ token }) => async () => {
    if (!token || !token.token) {
        throw new TokenProviderError(`Please pass a valid token to fromStatic`, false);
    }
    return token;
};
