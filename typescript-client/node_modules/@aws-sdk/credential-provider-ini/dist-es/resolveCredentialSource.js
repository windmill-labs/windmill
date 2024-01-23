import { fromEnv } from "@aws-sdk/credential-provider-env";
import { fromContainerMetadata, fromInstanceMetadata } from "@smithy/credential-provider-imds";
import { CredentialsProviderError } from "@smithy/property-provider";
export const resolveCredentialSource = (credentialSource, profileName) => {
    const sourceProvidersMap = {
        EcsContainer: fromContainerMetadata,
        Ec2InstanceMetadata: fromInstanceMetadata,
        Environment: fromEnv,
    };
    if (credentialSource in sourceProvidersMap) {
        return sourceProvidersMap[credentialSource]();
    }
    else {
        throw new CredentialsProviderError(`Unsupported credential source in profile ${profileName}. Got ${credentialSource}, ` +
            `expected EcsContainer or Ec2InstanceMetadata or Environment.`);
    }
};
