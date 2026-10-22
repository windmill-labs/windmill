import { AppService, type Policy } from '$lib/gen'

/** Writes who can open the app to the deployed app at once. The rest of the policy comes from the
 *  deployed row, not the editor's: sandbox, SDK scopes and the run-as user wait for a deploy, and
 *  the runnable whitelist describes the deployed value. */
export async function publishAccessMode(
	workspace: string,
	path: string,
	mode: Policy['execution_mode']
): Promise<void> {
	const row = await AppService.getAppLiteByPath({ workspace, path })
	await AppService.updateApp({
		workspace,
		path,
		requestBody: {
			policy: { ...row.policy, execution_mode: mode },
			// Without it the backend runs the app as whoever clicked.
			preserve_on_behalf_of: true,
			// The clicker's draft holds edits this write does not deploy.
			skip_draft_deletion: true
		}
	})
}
