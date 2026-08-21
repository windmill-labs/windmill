/**
 * What an import run has already created, kept across a reload.
 *
 * The plan lives in the URL (`./plan`) and everything the first two steps decide belongs
 * there — that is what makes the back button and shareable links work. This file is for the
 * one thing the URL cannot hold: a fact the run produced rather than the user, which the
 * next page load has no other way to learn. Creating the workspace is currently the only
 * one. Anything a user chose goes in the plan, not here.
 *
 * Kept out of the executor so a caller can ask what is parked without pulling the run in.
 */

const PARKED_KEY = 'import_wizard_parked'

export type ParkedImport = {
	/** The project this run is importing. */
	slug: string
	/** The workspace the run created, which a resumed run must not try to create again. */
	workspaceId: string
}

export function parkImport(parked: ParkedImport): void {
	try {
		sessionStorage.setItem(PARKED_KEY, JSON.stringify(parked))
	} catch {
		// Storage disabled or full. The run continues; only the resume is lost.
	}
}

export function clearParkedImport(): void {
	try {
		sessionStorage.removeItem(PARKED_KEY)
	} catch {}
}

/**
 * The parked run, when it is the one being asked about. Both fields have to match: an entry
 * left by another project would otherwise make this run skip a create it has not done, and
 * enter a workspace that belongs to a different import.
 */
export function resumableImport(slug: string, workspaceId: string): boolean {
	const parked = readParkedImport()
	return parked?.slug === slug && parked?.workspaceId === workspaceId
}

export function readParkedImport(): ParkedImport | undefined {
	let raw: string | null = null
	try {
		raw = sessionStorage.getItem(PARKED_KEY)
	} catch {
		return undefined
	}
	if (!raw) return undefined
	try {
		const parsed = JSON.parse(raw)
		return typeof parsed?.slug === 'string' && typeof parsed?.workspaceId === 'string'
			? { slug: parsed.slug, workspaceId: parsed.workspaceId }
			: undefined
	} catch {
		return undefined
	}
}
