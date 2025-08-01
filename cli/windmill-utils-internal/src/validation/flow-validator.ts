import Ajv, { AnySchema, ErrorObject, ValidateFunction } from 'ajv';
import { parseWithPointers, YamlParserResult } from '@stoplight/yaml';
import openFlowSchema from '../gen/openflow.json';

/**
 * Recursively removes discriminator mapping from a schema object.
 * This is necessary because AJV doesn't support discriminator mapping.
 * @param obj - The schema object to process
 */
function removeMapping(obj: any) {
  if (obj && typeof obj === 'object') {
    if (obj.discriminator?.mapping) delete obj.discriminator.mapping;
    for (const v of Object.values(obj)) removeMapping(v);
  }
}

/**
 * Flow validator class that initializes AJV once and reuses it for validation.
 */
export class FlowValidator {
  private readonly validate: ValidateFunction;

  constructor() {
    const ajv = new Ajv({ strict: false, allErrors: true, discriminator: true });
    const sanitizedSchema = JSON.parse(JSON.stringify(openFlowSchema));
    removeMapping(sanitizedSchema);
    
    for (const [n, s] of Object.entries(sanitizedSchema.components.schemas)) {
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