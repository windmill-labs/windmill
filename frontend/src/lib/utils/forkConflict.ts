import { forkConflictModal } from '$lib/stores'

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
 * Opens the global ForkConflictModal and awaits the user's choice. Resolves
 * to true when the user clicks "Enable anyway", false when they cancel or
 * dismiss. If a previous modal is still pending (e.g. user clicked toggles
 * on two rows in quick succession), resolve the older promise to false so
 * the prior caller doesn't hang.
 */
function askForkConflictConfirm(kind: string, kindLabel: string, parentWorkspaceId: string) {
	return new Promise<boolean>((resolve) => {
		const previous = forkConflictModal.val
		previous?.resolve(false)
		forkConflictModal.val = { kind, kindLabel, parentWorkspaceId, resolve }
	})
}

/**
 * Catches a fork-conflict error from `fn(false)`, shows the confirmation
 * dialog, and retries with `fn(true)` when the user accepts. Re-throws every
 * other error.
 *
 * Returns `true` when the call committed (no conflict, or user confirmed and
 * retry succeeded) and `false` when the user dismissed the modal. Callers
 * should bail on `false` to skip success toasts and revert any optimistic UI
 * state.
 *
 * `kindLabel` is shown to the user — pass a friendly name like "kafka trigger"
 * or "schedule" so the dialog reads naturally.
 */
export async function withForkConflictRetry(
	fn: (force: boolean) => Promise<unknown>,
	kindLabel: string
): Promise<boolean> {
	try {
		await fn(false)
		return true
	} catch (e) {
		const conflict = detectForkConflict(e)
		if (!conflict) throw e
		const proceed = await askForkConflictConfirm(
			conflict.kind,
			kindLabel,
			conflict.parentWorkspaceId
		)
		// User explicitly dismissed the modal — treat as a silent no-op so the
		// caller's catch block doesn't pop a redundant error toast.
		if (!proceed) return false
		await fn(true)
		return true
	}
}
