import { TokenIdentity, TokenIdentityProvider } from "@aws-sdk/types";
export interface FromStaticInit {
  token?: TokenIdentity;
}
export declare const fromStatic: ({
  token,
}: FromStaticInit) => TokenIdentityProvider;
