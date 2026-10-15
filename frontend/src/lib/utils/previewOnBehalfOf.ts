type ActingUser = { is_admin?: boolean; is_super_admin?: boolean; groups?: string[] } | undefined

/**
 * The identity an editor preview should run as: the runnable's configured run-as identity, so a
 * test behaves like the deployed run. Only admins and `wm_deployers` may preview as someone else
 * (the server refuses anyone else), and for them a save keeps that identity too; anyone else
 * previews as themselves, which is also what their own save would record.
 */
export function previewOnBehalfOf(
	user: ActingUser,
	onBehalfOf: string | null | undefined,
	onBehalfOfEmail?: string | null
): string | undefined {
	const canPreserve =
		!!user?.is_admin || !!user?.is_super_admin || !!user?.groups?.includes('wm_deployers')
	if (!canPreserve) return undefined
	return onBehalfOf || onBehalfOfEmail || undefined
}

/** An app's runnables run as its policy identity unless it is in viewer mode, which runs them as
 * whoever views it — the editor's user, in a preview. */
export function appPreviewOnBehalfOf(
	user: ActingUser,
	policy:
		| { execution_mode?: string; on_behalf_of?: string; on_behalf_of_email?: string }
		| undefined
): string | undefined {
	if (!policy || policy.execution_mode === 'viewer') return undefined
	return previewOnBehalfOf(user, policy.on_behalf_of, policy.on_behalf_of_email)
}
