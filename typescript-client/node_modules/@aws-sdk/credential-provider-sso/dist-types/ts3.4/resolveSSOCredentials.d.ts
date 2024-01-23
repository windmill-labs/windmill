import { AwsCredentialIdentity } from "@smithy/types";
import { FromSSOInit, SsoCredentialsParameters } from "./fromSSO";
export declare const resolveSSOCredentials: ({
  ssoStartUrl,
  ssoSession,
  ssoAccountId,
  ssoRegion,
  ssoRoleName,
  ssoClient,
  profile,
}: FromSSOInit & SsoCredentialsParameters) => Promise<AwsCredentialIdentity>;
