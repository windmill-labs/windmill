/**
 * Reactive registry of drafts that `migrateUserDraftsToDb` could not push to
 * the server. The migration runs on every layout mount, so a persistently
 * un-migratable draft would re-fail (and re-report) each time — keying by the
 * LS key dedupes those repeats. A SINGLE toast fires on the empty→non-empty
 * transition (never per-failure, never when there's nothing wrong); its action
 * opens `DraftMigrationErrorModal`, which reads `list` live so failures that
 * surface while the modal is already open just appear in place.
 */
import { SvelteMap } from 'svelte/reactivity'
import type { UserDraftItemKind } from '$lib/gen'
import { sendUserToast } from './toast'

export type DraftMigrationError = {
	/** The source `userdraft/...` localStorage key — identity and delete target. */
	key: string
	workspace: string
	itemKind: UserDraftItemKind
	path: string
	/** The draft payload, surfaced verbatim by the modal's "View JSON". */
	value: unknown
}

const errors = new SvelteMap<string, DraftMigrationError>()
let modalOpen = $state(false)

export const draftMigrationErrors = {
	get list(): DraftMigrationError[] {
		return [...errors.values()]
	},
	get modalOpen(): boolean {
		return modalOpen
	},
	set modalOpen(open: boolean) {
		modalOpen = open
	}
}

/** Open the modal listing the failed migrations. */
export function openDraftMigrationErrorModal(): void {
	modalOpen = true
}

/**
 * Record a failed draft migration. Idempotent per `key`; the toast only fires
 * on the first failure of a batch (empty→non-empty) and is suppressed when the
 * modal is already open, since the user is already resolving issues there.
 */
export function reportDraftMigrationError(error: DraftMigrationError): void {
	if (errors.has(error.key)) return
	const wasEmpty = errors.size === 0
	errors.set(error.key, error)
	if (wasEmpty && !modalOpen) {
		sendUserToast('Some local storage drafts could not be migrated', 'error', [
			{ label: 'Resolve issues', callback: openDraftMigrationErrorModal }
		])
	}
}

/** Drop the un-migratable draft from localStorage and clear its error entry. */
export function deleteDraftMigrationError(key: string): void {
	try {
		localStorage.removeItem(key)
	} catch {
		// Best-effort; the entry leaves the list regardless.
	}
	errors.delete(key)
}

/** Drop every un-migratable draft at once. */
export function deleteAllDraftMigrationErrors(): void {
	for (const key of [...errors.keys()]) {
		deleteDraftMigrationError(key)
	}
}
