import type { Schema } from '$lib/common'
import { schemaToTsType } from '$lib/schema'
import { deepEqual } from 'fast-equals'

export function buildExtraLibForBatchReruns({
	schemas,
	script_path
}: {
	schemas: { schema: Schema | object; script_hash: string }[]
	script_path: string
}) {
	return `
/**
* get variable (including secret) at path
* @param {string} path - path of the variable (e.g: f/examples/secret)
*/
declare function variable(path: string): string;

/**
* get resource at path
* @param {string} path - path of the resource (e.g: f/examples/my_resource)
*/
declare function resource(path: string): any;

declare const job: {
  /**
   * scheduled date of the original job in ISO 8601 string format.
   * Use \`new Date(job.scheduled_for)\` to get a Date object
   */
  scheduled_for: string;
  /**
   * id of the original job
   */
  id: string;
  kind: 'flow' | 'script';
  script_path: ${JSON.stringify(script_path)};
} & (
  ${schemas
		.map(
			(s) => `
{
  script_hash: ${JSON.stringify(s.script_hash)};
  input: ${schemaToTsType(s.schema as Schema)};
}`
		)
		.join(' | ')}
)`
}

// Used for InputTransformForm
export function mergeSchemasForBatchReruns(schemas: Schema[]): Schema {
	const merged: Schema = { $schema: '', required: [], properties: {}, type: 'object' }

	for (const s of schemas) {
		for (const [propertyName, property] of Object.entries(s.properties)) {
			if (!(propertyName in merged.properties)) {
				merged.properties[propertyName] = { ...property }
			} else {
				if (!deepEqual(merged.properties[propertyName], property)) {
					if (merged.properties[propertyName].type === 'string' && property.type === 'string') {
						merged.properties[propertyName] = { type: 'string' }
					} else {
						merged.properties[propertyName] = { type: 'object' }
					}
				}
			}
		}
	}

	for (const propertyName of Object.keys(merged.properties)) {
		if (schemas.every((s) => s.required.includes(propertyName))) {
			merged.required.push(propertyName)
		}
	}

	return merged
}

export function batchReRunDefaultPropertyExpr(
	propertyName: string,
	schemas: { schema: Schema | object; script_hash: string }[]
): string {
	const typeMap = new Map<string, string[]>()
	for (const s of schemas) {
		const type = (s.schema as Schema).properties[propertyName]?.type ?? 'undefined'
		if (!typeMap.has(type)) typeMap.set(type, [])
		const entry = typeMap.get(type)
		entry?.push(s.script_hash)
	}

	const defaultExpr = /^[a-zA-Z_$][0-9a-zA-Z_$]*$/.test(propertyName)
		? `job.input.${propertyName}`
		: `job.input[${JSON.stringify(propertyName)}]`
	if (typeMap.size === 1) {
		return defaultExpr
	}
	if (typeMap.has('undefined') && typeMap.size === 2) {
		return `// ${propertyName} does not exist on all versions of the script\nif (${JSON.stringify(propertyName)} in job.input) {\n  ${defaultExpr}\n} else {\n  undefined\n}`
	}
	return (
		`// '${propertyName}' has different types depending on the script version\n` +
		[...typeMap.entries()]
			.map(([type, hashes]) => {
				const expr = type === 'undefined' ? 'undefined' : defaultExpr
				const condition = hashes.map((h) => `job.script_hash === ${JSON.stringify(h)}`).join(' || ')
				return `if (${condition}) {\n  ${expr} // ${type}\n}`
			})
			.join(' else ')
	)
}
