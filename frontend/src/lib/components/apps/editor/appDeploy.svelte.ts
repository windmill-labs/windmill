import { AppService, type Policy } from '$lib/gen'
import { deployedPermissionedAs } from '$lib/components/itemSettings/onBehalfOf'

type RunAs = { on_behalf_of?: string; on_behalf_of_email?: string }
type DeployingUser = Parameters<typeof deployedPermissionedAs>[0]

export function computeSecretUrl(secretUrl: string, workspace: string | undefined) {
	return `${window.location.origin}/public/${workspace}/${secretUrl}`
}

/** The user the app at `path` runs on behalf of right now, read from its row: the editor's copy
 *  is the one it loaded, which another deploy may have replaced since. Undefined when the read
 *  fails. */
export async function fetchDeployedRunAs(
	workspace: string,
	path: string
): Promise<RunAs | undefined> {
	try {
		const { on_behalf_of, on_behalf_of_email } = (
			await AppService.getAppLiteByPath({ workspace, path })
		).policy
		return { on_behalf_of, on_behalf_of_email }
	} catch {
		return undefined
	}
}

/** Who can open the app at `path` right now. An update deploy states no mode, so the server keeps
 *  the deployed one, which someone may have changed since the editor loaded the app. Undefined
 *  when the read fails. */
export async function fetchDeployedAccessMode(
	workspace: string,
	path: string
): Promise<Policy['execution_mode'] | undefined> {
	try {
		return (await AppService.getAppLiteByPath({ workspace, path })).policy.execution_mode
	} catch {
		return undefined
	}
}

/** Writes into an app's policy the user its deploy will run on behalf of, so the request names
 *  it. The editor keeps the policy it sent as the deployed one, so it has to say what the backend
 *  stores, or the next deploy would see a change that already happened. */
export function setDeployedOnBehalfOf(
	policy: RunAs,
	user: DeployingUser,
	folderDefault?: string
): void {
	const runsAs = deployedPermissionedAs(user, policy.on_behalf_of, folderDefault)
	if (!runsAs || runsAs === policy.on_behalf_of) return
	policy.on_behalf_of = runsAs
	policy.on_behalf_of_email = runsAs === `u/${user?.username}` ? user?.email : undefined
}
