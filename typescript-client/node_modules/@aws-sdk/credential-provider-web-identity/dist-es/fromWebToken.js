import { CredentialsProviderError } from "@smithy/property-provider";
export const fromWebToken = (init) => () => {
    const { roleArn, roleSessionName, webIdentityToken, providerId, policyArns, policy, durationSeconds, roleAssumerWithWebIdentity, } = init;
    if (!roleAssumerWithWebIdentity) {
        throw new CredentialsProviderError(`Role Arn '${roleArn}' needs to be assumed with web identity,` +
            ` but no role assumption callback was provided.`, false);
    }
    return roleAssumerWithWebIdentity({
        RoleArn: roleArn,
        RoleSessionName: roleSessionName ?? `aws-sdk-js-session-${Date.now()}`,
        WebIdentityToken: webIdentityToken,
        ProviderId: providerId,
        PolicyArns: policyArns,
        Policy: policy,
        DurationSeconds: durationSeconds,
    });
};
