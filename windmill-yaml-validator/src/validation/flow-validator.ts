import Ajv, { AnySchema, ErrorObject, ValidateFunction } from 'ajv';
import { parseWithPointers, YamlParserResult } from '@stoplight/yaml';
import openFlowSchema from '../gen/openflow.json';

/**
 * Flow validator class that initializes AJV once and reuses it for validation.
 */
export class FlowValidator {
  private readonly validate: ValidateFunction;

  constructor() {
    const ajv = new Ajv({ strict: false, allErrors: true, discriminator: true });
    
    for (const [n, s] of Object.entries(openFlowSchema.components.schemas)) {
      ajv.addSchema(s as AnySchema, `#/components/schemas/${n}`);
    }
    
    this.validate = ajv.getSchema('#/components/schemas/OpenFlow')!;
  }

  /**
   * Validates a flow document against the OpenFlow schema.
   * @param doc - The YAML flow document as a string
   * @returns Object containing the parsed document and any validation errors
   */
  validateFlow(doc: string): {
    parsed: YamlParserResult<unknown>;
    errors: ErrorObject[];
  } {
    if (typeof doc !== 'string') {
      throw new Error('Document must be a string');
    }

    const parsed = parseWithPointers(doc);
    const { data } = parsed;
    const ok = this.validate(data);
    
    if (ok) {
      return {
        parsed,
        errors: [],
      };
    }
    
    return {
      parsed,
      errors: this.validate.errors!,
    };
  }
}