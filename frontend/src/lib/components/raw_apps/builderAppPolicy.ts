/**
 * Brings a raw-app policy in line with what the backend requires of a draft from an operator with
 * builder rights (see `docs/operator-builder-rights.md`): sandboxed, never in Viewer mode, and
 * running as nobody but its builder. A policy loaded from an app someone else deployed carries
 * their identity and may carry either of the other two, so every builder draft writer goes
 * through this before saving. Writes only what differs, so it is safe inside an `$effect`.
 */
export function conformBuilderAppPolicy(
	policy: Record<string, any>,
	user: { username?: string; email?: string } | undefined
): void {
	if (policy.sandbox !== true) policy.sandbox = true
	if (policy.execution_mode === 'viewer') policy.execution_mode = 'publisher'
	const username = user?.username
	const self = username?.includes('@') ? username : `u/${username}`
	if (
		(policy.on_behalf_of != undefined && policy.on_behalf_of !== self) ||
		(policy.on_behalf_of_email != undefined && policy.on_behalf_of_email !== user?.email)
	) {
		delete policy.on_behalf_of
		delete policy.on_behalf_of_email
	}
}
