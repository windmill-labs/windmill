import { ErrorObject } from 'ajv';
import { YamlParserResult } from '@stoplight/yaml';
/**
 * Flow validator class that initializes AJV once and reuses it for validation.
 */
export declare class FlowValidator {
    private readonly validate;
    constructor();
    /**
     * Validates a flow document against the OpenFlow schema.
     * @param doc - The YAML flow document as a string
     * @returns Object containing the parsed document and any validation errors
     */
    validateFlow(doc: string): {
        parsed: YamlParserResult<unknown>;
        errors: ErrorObject[];
    };
}
