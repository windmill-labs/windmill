import type { Schema } from '$lib/common'
import { schemaToTsType } from '$lib/schema'

export function buildExtraLibForBatchReruns(schema: Schema) {
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
  * job input as an object
  */
  input: ${schemaToTsType(schema)},
  /**
   * original scheduled date of the job
   */
  scheduled_for: Date
}
`
}
