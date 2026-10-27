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

/** Whether the deployed user should be picked, as the run-as selector does once it shows with
 *  nothing picked. The Runtime tab applies it too, so a deploy that only changes a runtime
 *  setting keeps sending `preserve_on_behalf_of`. */
export function keepsDeployedOnBehalfOf(
	user: DeployingUser,
	choice: string | undefined,
	email: string | undefined,
	savedEmail: string | undefined
): boolean {
	return choice === undefined && !!email && !!savedEmail && canPreserveOnBehalfOf(user)
}

/** The `u/username` a deploy leaves the item running on behalf of: the held user for a deployer
 *  who may preserve, the deployer for anyone else. */
export function deployedPermissionedAs(
	user: DeployingUser,
	permissionedAs: string | undefined
): string | undefined {
	if (permissionedAs && canPreserveOnBehalfOf(user)) return permissionedAs
	return user?.username ? `u/${user.username}` : undefined
}
