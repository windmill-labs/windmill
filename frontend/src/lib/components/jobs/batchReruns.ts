import type { Schema } from '$lib/common'
import { schemaToTsType } from '$lib/schema'

export function buildExtraLibForBatchReruns({
	schema,
	script_path,
	script_hashes
}: {
	schema: Schema
	script_path: string
	script_hashes: string[]
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
  * original job arguments
  */
  input: ${schemaToTsType(schema)};
  /**
   * scheduled date of the original job
   */
  scheduled_for: Date;
  /**
   * id of the original job
   */
  id: string;
  script_path: ${JSON.stringify(script_path)};
  script_hash: ${script_hashes.map((h) => JSON.stringify(h)).join(' | ')};
}`
}
