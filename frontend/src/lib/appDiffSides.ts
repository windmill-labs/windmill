/** Split a CLASSIC app draft into comparable parts. The editor mirrors
 * summary/draft_path INTO the bare grid it autosaves, while a deployed row
 * keeps them outside `value` — compared raw, metadata diffs as grid noise.
 * Also unwraps legacy wrapped drafts ({summary/policy/custom_path, value}). */
export function classicAppDraftParts(json: unknown): {
	value: unknown
	summary?: string
	draftPath?: string
} {
	if (json === null || typeof json !== 'object' || Array.isArray(json)) {
		return { value: json }
	}
	const obj = json as Record<string, unknown>
	// A bare App always carries `grid` at top level (and the editor mirrors
	// summary into it), while a legacy wrapper never does — `grid` is the only
	// reliable discriminator; metadata keys appear on both shapes.
	const wrapped =
		!('grid' in obj) &&
		typeof obj.value === 'object' &&
		obj.value !== null &&
		'grid' in (obj.value as Record<string, unknown>)
	if (wrapped) {
		const inner = classicAppDraftParts(obj.value)
		return {
			value: inner.value,
			summary: (obj.summary as string | undefined) ?? inner.summary,
			draftPath: (obj.draft_path as string | undefined) ?? inner.draftPath
		}
	}
	const { parent_version: _pv, draft_path, summary, ...value } = obj
	return {
		value,
		summary: summary as string | undefined,
		draftPath: draft_path as string | undefined
	}
}
