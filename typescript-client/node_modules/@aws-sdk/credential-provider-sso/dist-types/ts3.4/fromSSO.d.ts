import { SSOClient } from "@aws-sdk/client-sso";
import { SourceProfileInit } from "@smithy/shared-ini-file-loader";
import { AwsCredentialIdentityProvider } from "@smithy/types";
export interface SsoCredentialsParameters {
  ssoStartUrl: string;
  ssoSession?: string;
  ssoAccountId: string;
  ssoRegion: string;
  ssoRoleName: string;
}
export interface FromSSOInit extends SourceProfileInit {
  ssoClient?: SSOClient;
}
export declare const fromSSO: (
  init?: FromSSOInit & Partial<SsoCredentialsParameters>
) => AwsCredentialIdentityProvider;
