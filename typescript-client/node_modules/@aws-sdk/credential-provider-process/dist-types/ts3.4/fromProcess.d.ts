import { SourceProfileInit } from "@smithy/shared-ini-file-loader";
import { AwsCredentialIdentityProvider } from "@smithy/types";
export interface FromProcessInit extends SourceProfileInit {}
export declare const fromProcess: (
  init?: FromProcessInit
) => AwsCredentialIdentityProvider;
