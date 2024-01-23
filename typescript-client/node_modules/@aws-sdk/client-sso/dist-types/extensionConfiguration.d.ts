import { AwsRegionExtensionConfiguration } from "@aws-sdk/types";
import { HttpHandlerExtensionConfiguration } from "@smithy/protocol-http";
import { DefaultExtensionConfiguration } from "@smithy/types";
/**
 * @internal
 */
export interface SSOExtensionConfiguration extends HttpHandlerExtensionConfiguration, DefaultExtensionConfiguration, AwsRegionExtensionConfiguration {
}
