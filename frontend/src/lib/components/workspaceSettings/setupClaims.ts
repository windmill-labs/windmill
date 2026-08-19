/**
 * What a setup run created, and whether it is still there.
 *
 * Try again re-runs the whole plan, so every write meets what the previous attempt left behind
 * and has to answer one question: is the thing at this path the thing I made? Writing over its
 * own work is required; writing over another admin's destroys a password Supabase shows once.
 *
 * A claim therefore carries a **mark** — the discriminator to compare against the object as it
 * is now, rather than trusting that whatever sits at a remembered path is ours.
 *
 * Values, not runes, so the ownership matrix is testable without mounting a component.
 */

export type ClaimKind = 'secret' | 'resource' | 'row'

export type Claim = {
	kind: ClaimKind
	path: string
	/**
	 * Compared against the live object. It has to move whenever anyone else writes: `edited_at`
	 * for a secret and a resource — an author survives an edit and so cannot tell one from no
	 * edit at all — and the target for a row.
	 */
	mark: string
}

export type Claims = readonly Claim[]

export const noClaims: Claims = []

function sameObject(a: Claim, kind: ClaimKind, path: string): boolean {
	return a.kind === kind && a.path === path
}

/** Re-claiming an object replaces its mark. */
export function claim(claims: Claims, kind: ClaimKind, path: string, mark: string): Claims {
	return [...claims.filter((c) => !sameObject(c, kind, path)), { kind, path, mark }]
}

export function claimOf(claims: Claims, kind: ClaimKind, path: string): Claim | undefined {
	return claims.find((c) => sameObject(c, kind, path))
}

/** Given up when a run takes its own object back out, so the path is free again. */
export function release(claims: Claims, kind: ClaimKind, path: string): Claims {
	return claims.filter((c) => !sameObject(c, kind, path))
}

/**
 * Whether the object now at `path` is the one this run claimed. `observed` is the mark read back
 * from the live object; `undefined` means nothing is there.
 */
export function stillOurs(
	claims: Claims,
	kind: ClaimKind,
	path: string,
	observed: string | undefined
): boolean {
	const held = claimOf(claims, kind, path)
	return !!held && observed !== undefined && held.mark === observed
}

export function anythingClaimed(claims: Claims): boolean {
	return claims.length > 0
}

/**
 * Carried across the full-page redirect the blocked-popup Supabase leg falls back to. No secret
 * travels: a mark is a timestamp or a resource path.
 */
export function claimsToJSON(claims: Claims): Claim[] {
	return [...claims]
}

const KINDS: ClaimKind[] = ['secret', 'resource', 'row']

export function claimsFromJSON(value: unknown): Claims {
	if (!Array.isArray(value)) return noClaims
	return value.filter(
		(c): c is Claim =>
			!!c &&
			typeof c === 'object' &&
			typeof (c as Claim).path === 'string' &&
			typeof (c as Claim).mark === 'string' &&
			KINDS.includes((c as Claim).kind)
	)
}
