import { TokenIdentityProvider } from "@aws-sdk/types";
import { SourceProfileInit } from "@smithy/shared-ini-file-loader";
export interface FromSsoInit extends SourceProfileInit {}
export declare const fromSso: (init?: FromSsoInit) => TokenIdentityProvider;
