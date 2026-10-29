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

/** Whether a deploy asks first whose access the item will run with. A deploy asks when the item
 *  starts running on behalf of someone, changes who, or stops: a deploy that keeps the deployed
 *  user has nothing new to confirm. An app always runs on behalf of someone, so its first deploy
 *  always asks. `saved*` are the deployed version's, unset before the first deploy. */
export function deployAsksOnBehalfOf(
	kind: 'flow' | 'script' | 'app',
	deploy: {
		user: DeployingUser
		email: string | undefined
		permissionedAs: string | undefined
		savedEmail: string | undefined
		savedPermissionedAs: string | undefined
		folderDefault?: string
	}
): boolean {
	const deployedHoldsUser = !!(deploy.savedPermissionedAs ?? deploy.savedEmail)
	if (kind !== 'app' && !deploy.email && !deploy.folderDefault) return deployedHoldsUser
	if (!deployedHoldsUser) return true
	return (
		deployedPermissionedAs(deploy.user, deploy.permissionedAs, deploy.folderDefault) !==
		deploy.savedPermissionedAs
	)
}

/** What keeps lending the publisher's access to an app's viewers safe. */
export function appOnBehalfOfTooltip(rawApp: boolean): string {
	return `To keep this safe, the app can call only the runnables it was deployed with, and viewers can't override their static inputs.${rawApp ? " Inline runnables can instead be set to read data tables and object storage with the viewer's own access." : ''}`
}
