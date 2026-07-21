/** Comparable parts of a CLASSIC (visual) app draft/deployed value.
 *
 * The classic editor autosaves the bare `App` grid with `summary` (and a
 * draft-only `draft_path`) mirrored INTO it so a draft round-trips them,
 * while a deployed row keeps `summary` as a column outside `value` — so the
 * metadata must be compared separately from the grid or it diffs as spurious
 * additions. Extracts `{ value, summary }`: value is the grid with
 * summary/draft_path/parent_version stripped (apply to BOTH sides so an older
 * deployed value that retained an inner summary stays comparable). Defensively
 * unwraps a legacy wrapped draft: a bare grid never carries summary/policy/
 * custom_path siblings next to an object `value`. */
export function classicAppDraftParts(json: unknown): { value: unknown; summary?: string } {
	if (json === null || typeof json !== 'object' || Array.isArray(json)) {
		return { value: json }
	}
	const obj = json as Record<string, unknown>
	const wrapped =
		typeof obj.value === 'object' &&
		obj.value !== null &&
		('summary' in obj || 'policy' in obj || 'custom_path' in obj)
	if (wrapped) {
		const inner = classicAppDraftParts(obj.value)
		return { value: inner.value, summary: (obj.summary as string | undefined) ?? inner.summary }
	}
	const { parent_version: _pv, draft_path: _dp, summary, ...value } = obj
	return { value, summary: summary as string | undefined }
}
