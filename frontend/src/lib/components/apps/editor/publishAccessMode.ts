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

/** The policy an update deploy sends. The server keeps the deployed mode, read under the row lock,
 *  for a policy that states none, so a deploy states one only to change it: otherwise it would put
 *  back a mode changed since the policy was read. An editor passes the mode it loaded: holding
 *  another one means the settings drawer left that change for the deploy. */
export function policyForUpdate<P extends Partial<Policy>>(
	policy: P,
	deployedMode?: Policy['execution_mode']
): P {
	if (deployedMode !== undefined && policy.execution_mode !== deployedMode) return policy
	const sent = { ...policy }
	delete sent.execution_mode
	return sent
}
