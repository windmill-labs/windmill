/**
 * The backend rejects "enable" requests on triggers/schedules in a fork when
 * the parent workspace has the same path enabled. The error body is shaped as
 *   `fork-conflict:<kind>:<parent_workspace_id>`
 * so the UI can show a tailored confirm-to-proceed dialog and re-issue the
 * call with `force: true` if the user agrees.
 */
export interface ForkConflict {
	kind: string
	parentWorkspaceId: string
}

export function detectForkConflict(e: unknown): ForkConflict | null {
	const body = (e as any)?.body
	const raw =
		typeof body === 'string'
			? body
			: ((body as any)?.error?.message ?? (body as any)?.message ?? (e as any)?.message ?? '')
	const m = String(raw).match(/fork-conflict:([^:]+):(.+)/)
	if (!m) return null
	return { kind: m[1], parentWorkspaceId: m[2].trim() }
}

/**
 * Catches a fork-conflict error from `fn(false)`, asks the user to confirm,
 * and retries with `fn(true)` when accepted. Re-throws every other error.
 *
 * `kindLabel` is shown to the user — pass a friendly name like "kafka trigger"
 * or "schedule" so the dialog reads naturally.
 */
export async function withForkConflictRetry<T>(
	fn: (force: boolean) => Promise<T>,
	kindLabel: string
): Promise<T> {
	try {
		return await fn(false)
	} catch (e) {
		const conflict = detectForkConflict(e)
		if (!conflict) throw e
		const proceed = window.confirm(
			`This ${kindLabel} is also enabled in the parent workspace ` +
				`(${conflict.parentWorkspaceId}). Both will run at the same time and may ` +
				`compete for the same upstream events or duplicate side effects.\n\n` +
				`Enable in this fork anyway?`
		)
		if (!proceed) {
			throw new Error(
				`Enable cancelled because of fork conflict with ${conflict.parentWorkspaceId}`
			)
		}
		return await fn(true)
	}
}
