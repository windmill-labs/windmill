import { AppService, type Policy } from '$lib/gen'

/** Writes who can open the app to the deployed app at once. The server applies the mode alone to
 *  the deployed policy under the deploy's row lock: sandbox, SDK scopes and the run-as user wait
 *  for a deploy, and a concurrent deploy neither reverts the mode nor is reverted by it. */
export async function publishAccessMode(
	workspace: string,
	path: string,
	mode: NonNullable<Policy['execution_mode']>
): Promise<void> {
	await AppService.updateApp({
		workspace,
		path,
		requestBody: {
			execution_mode: mode,
			// The clicker's draft holds edits this write does not deploy.
			skip_draft_deletion: true
		}
	})
}

/** The policy an update deploy sends. The mode changes only through `publishAccessMode`, so an
 *  update states none and the server keeps the deployed one, read under the row lock: a policy read
 *  earlier would otherwise put back a mode changed since. */
export function policyForUpdate<P extends Partial<Policy>>(policy: P): P {
	const sent = { ...policy }
	delete sent.execution_mode
	return sent
}
