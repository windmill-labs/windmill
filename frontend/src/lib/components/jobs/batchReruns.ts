export function buildExtraLibForBatchReruns(jobParameters: string[]) {
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

declare const job = {
  /**
  * job input as an object
  */
  input: ${JSON.stringify(Object.fromEntries(jobParameters.map((p) => [p, null as any])))},
  /**
   * original scheduled date of the job
   */
  scheduled_for = null as Date
}
`
}
