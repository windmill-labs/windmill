export const WM_DEPLOYERS_GROUP = 'wm_deployers'

type DeployingUser =
	| {
			username?: string
			email?: string
			is_admin?: boolean
			is_super_admin?: boolean
			groups?: string[]
	  }
	| undefined

/** Who may keep, or pick, another user's identity on deploy. For anyone else the backend deploys
 *  the item on behalf of the deployer, whatever the request says. */
export function canPreserveOnBehalfOf(user: DeployingUser): boolean {
	return (
		!!user?.is_admin ||
		!!user?.is_super_admin ||
		(user?.groups?.includes(WM_DEPLOYERS_GROUP) ?? false)
	)
}

/** The `u/username` a deploy leaves the item running on behalf of. The editors send the user their
 *  deploy dialog confirmed, which the backend keeps for a deployer who may preserve and replaces
 *  with the deployer for anyone else. With no user held, a create takes the folder's default for
 *  the path. */
export function deployedPermissionedAs(
	user: DeployingUser,
	permissionedAs: string | undefined,
	folderDefault?: string
): string | undefined {
	const kept = permissionedAs ?? folderDefault
	if (kept && canPreserveOnBehalfOf(user)) return kept
	return user?.username ? `u/${user.username}` : undefined
}

/** Whether a deploy asks first whose access the item will run with: every app deploy, since all
 *  its viewers use that access, and any flow or script deploy that leaves it running on behalf of
 *  someone or stops it doing so. */
export function deployAsksOnBehalfOf(
	kind: 'flow' | 'script' | 'app',
	holdsUser: boolean,
	deployedHoldsUser: boolean,
	folderDefault?: string
): boolean {
	return kind === 'app' || holdsUser || deployedHoldsUser || !!folderDefault
}

/** Writes into an app's policy the user its deploy will run on behalf of, so the request names
 *  it. The editor keeps the policy it sent as the deployed one, so it has to say what the backend
 *  stores, or the next deploy would see a change that already happened. */
export function setDeployedOnBehalfOf(
	policy: { on_behalf_of?: string; on_behalf_of_email?: string },
	user: DeployingUser,
	folderDefault?: string
): void {
	const runsAs = deployedPermissionedAs(user, policy.on_behalf_of, folderDefault)
	if (!runsAs || runsAs === policy.on_behalf_of) return
	policy.on_behalf_of = runsAs
	policy.on_behalf_of_email = runsAs === `u/${user?.username}` ? user?.email : undefined
}

/** What keeps lending the publisher's access to an app's viewers safe. */
export function appOnBehalfOfTooltip(rawApp: boolean): string {
	return `To keep this safe, the app can call only the runnables it was deployed with, and viewers can't override their static inputs.${rawApp ? " Inline runnables can instead be set to read data tables and object storage with the viewer's own access." : ''}`
}
