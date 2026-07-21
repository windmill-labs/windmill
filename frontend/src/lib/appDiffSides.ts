/** Bare draft value of a CLASSIC (visual) app, for diffing against the
 * deployed row's `value`. The classic editor drafts only the grid (a bare
 * `App` JSON) while the deployed row nests it under `value` beside
 * summary/policy — diffed raw, a one-field edit reads as a whole-document
 * rewrite. Wrapper metadata cannot change through a classic draft, so both
 * diff sides reduce to `{ value }`. Defensively unwraps an already-wrapped
 * draft (legacy rows): a bare grid never carries summary/policy/custom_path
 * siblings next to an object `value`. */
export function classicAppDraftValue(draftJson: unknown): unknown {
	if (draftJson === null || typeof draftJson !== 'object' || Array.isArray(draftJson)) {
		return draftJson
	}
	const { parent_version: _pv, ...rest } = draftJson as Record<string, unknown>
	const wrapped =
		typeof rest.value === 'object' &&
		rest.value !== null &&
		('summary' in rest || 'policy' in rest || 'custom_path' in rest)
	return wrapped ? rest.value : rest
}
