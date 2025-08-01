import Ajv, { ErrorObject } from 'ajv';
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
 * Validates a flow document against the OpenFlow schema.
 * @param doc - The YAML flow document as a string
 * @returns Object containing the parsed document and any validation errors
 */
export function validateFlow(doc: string): {
  parsed: YamlParserResult<unknown>;
  errors: ErrorObject[];
} {
  const parsed = parseWithPointers(doc);
  const { data } = parsed;
  const ajv = new Ajv({ strict: false, allErrors: true, discriminator: true });
  removeMapping(openFlowSchema);
  for (const [n, s] of Object.entries(openFlowSchema.components.schemas)) {
    ajv.addSchema(s, `#/components/schemas/${n}`);
  }
  const validate = ajv.getSchema('#/components/schemas/OpenFlow')!;
  const ok = validate(data);
  if (ok) return {
    parsed,
    errors: [],
  }
  return {
    parsed,
    errors: validate.errors!,
  };
}