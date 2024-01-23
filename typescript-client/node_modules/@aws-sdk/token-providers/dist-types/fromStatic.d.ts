import { TokenIdentity, TokenIdentityProvider } from "@aws-sdk/types";
export interface FromStaticInit {
    token?: TokenIdentity;
}
/**
 * Creates a token provider that will read from static token.
 */
export declare const fromStatic: ({ token }: FromStaticInit) => TokenIdentityProvider;
