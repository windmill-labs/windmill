import { ENV_CMDS_FULL_URI, ENV_CMDS_RELATIVE_URI, fromContainerMetadata, fromInstanceMetadata, } from "@smithy/credential-provider-imds";
import { CredentialsProviderError } from "@smithy/property-provider";
export const ENV_IMDS_DISABLED = "AWS_EC2_METADATA_DISABLED";
export const remoteProvider = (init) => {
    if (process.env[ENV_CMDS_RELATIVE_URI] || process.env[ENV_CMDS_FULL_URI]) {
        return fromContainerMetadata(init);
    }
    if (process.env[ENV_IMDS_DISABLED]) {
        return async () => {
            throw new CredentialsProviderError("EC2 Instance Metadata Service access disabled");
        };
    }
    return fromInstanceMetadata(init);
};
