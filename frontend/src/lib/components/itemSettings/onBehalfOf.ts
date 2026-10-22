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

/** `preserve_on_behalf_of` for a deploy. The settings leave `preserve` undefined until a user is
 *  picked there, and until then a redeploy keeps the deployed identity: the settings may never be
 *  opened, and without the flag the backend runs the item as the deployer. The backend ignores the
 *  flag for a deployer who may not preserve. */
export function preserveOnBehalfOfOnDeploy(
	preserve: boolean | undefined,
	hasDeployedIdentity: boolean
): true | undefined {
	return (preserve ?? hasDeployedIdentity) || undefined
}

/** The `u/username` a deploy leaves the item running on behalf of: the submitted user when the
 *  deployer may preserve it and the request asks to, the deployer otherwise. */
export function deployedPermissionedAs(
	user: DeployingUser,
	permissionedAs: string | undefined,
	preserve: boolean | undefined
): string | undefined {
	if (canPreserveOnBehalfOf(user) && preserve && permissionedAs) return permissionedAs
	return user?.username ? `u/${user.username}` : undefined
}

/** Every viewer of an app uses the access of the user it runs on behalf of, so the deploys that
 *  set that user ask first: the first deploy, and any that changes it. */
export function appDeployAsksOnBehalfOf(
	firstDeploy: boolean,
	savedPermissionedAs: string | undefined,
	runsAs: string | undefined
): boolean {
	return firstDeploy || (!!savedPermissionedAs && !!runsAs && runsAs !== savedPermissionedAs)
}

/** Writes into an app's policy the user its deploy will run on behalf of. The editor keeps the
 *  policy it sent as the deployed one, so it has to say what the backend stores, or the next
 *  deploy would see a change that already happened. */
export function setDeployedOnBehalfOf(
	policy: { on_behalf_of?: string; on_behalf_of_email?: string },
	user: DeployingUser,
	preserve: boolean | undefined
): void {
	const runsAs = deployedPermissionedAs(user, policy.on_behalf_of, preserve)
	if (!runsAs || runsAs === policy.on_behalf_of) return
	policy.on_behalf_of = runsAs
	policy.on_behalf_of_email = user?.email
}

/** What keeps lending the publisher's access to an app's viewers safe. */
export function appOnBehalfOfTooltip(rawApp: boolean): string {
	return `To keep this safe, the app can call only the runnables it was deployed with, and viewers can't override their static inputs.${rawApp ? " Inline runnables can instead be set to read data tables and object storage with the viewer's own access." : ''}`
}
