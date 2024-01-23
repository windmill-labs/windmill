import { SsoProfile } from "@aws-sdk/credential-provider-sso";
/**
 * @internal
 */
export { isSsoProfile } from "@aws-sdk/credential-provider-sso";
/**
 * @internal
 */
export declare const resolveSsoCredentials: (data: Partial<SsoProfile>) => Promise<import("@smithy/types").AwsCredentialIdentity>;
