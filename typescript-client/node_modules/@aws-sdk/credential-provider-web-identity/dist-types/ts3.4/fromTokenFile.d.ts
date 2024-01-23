import { AwsCredentialIdentityProvider } from "@smithy/types";
import { FromWebTokenInit } from "./fromWebToken";
export interface FromTokenFileInit
  extends Partial<
    Pick<FromWebTokenInit, Exclude<keyof FromWebTokenInit, "webIdentityToken">>
  > {
  webIdentityTokenFile?: string;
}
export declare const fromTokenFile: (
  init?: FromTokenFileInit
) => AwsCredentialIdentityProvider;
