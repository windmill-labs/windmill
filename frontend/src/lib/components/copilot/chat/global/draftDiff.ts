import { createTwoFilesPatch } from 'diff'
import YAML from 'yaml'

// Drop null/undefined object properties so `field: null` on one side never
// diffs against the field being absent on the other (backend rows spell unset
// as null, draft payloads omit the key). A real change keeps showing: the side
// that has a value still emits its line. Array elements are kept (recursed
// into, not removed) so positions stay aligned.
function pruneNulls(value: unknown): unknown {
	if (Array.isArray(value)) return value.map(pruneNulls)
	if (value !== null && typeof value === 'object') {
		return Object.fromEntries(
			Object.entries(value as Record<string, unknown>)
				.filter(([, v]) => v != null)
				.map(([k, v]) => [k, pruneNulls(v)])
		)
	}
	return value
}

// Serialize a draft/deployed diff value to deterministic YAML: sorted keys so
// field order never shows as a change, no anchors/aliases (a `*a` reference in
// a diff is unreadable), no line folding (wrapping would split one logical
// change across hunks). Multiline strings (script content, app files) render
// as block scalars, so code diffs read line-by-line instead of as one escaped
// JSON string.
export function toStableYaml(value: unknown): string {
	if (value == null) return ''
	return (
		YAML.stringify(pruneNulls(value), {
			sortMapEntries: true,
			aliasDuplicateObjects: false,
			lineWidth: 0
		}) ?? ''
	)
}

/** Unified patch between two sides of a workspace item.
 * Returns '' when the two sides are identical. */
export function yamlValuePatch(
	before: unknown,
	after: unknown,
	beforeLabel: string,
	afterLabel: string
): string {
	const beforeYaml = toStableYaml(before)
	const afterYaml = toStableYaml(after)
	if (beforeYaml === afterYaml) return ''
	return createTwoFilesPatch(beforeLabel, afterLabel, beforeYaml, afterYaml, '', '', { context: 3 })
}

/** Unified patch between the deployed and draft sides of a workspace item.
 * Returns '' when the two sides are identical. */
export function draftDeployedPatch(deployed: unknown, draft: unknown): string {
	return yamlValuePatch(deployed, draft, 'deployed', 'draft')
}

/** Indices of the real changed lines (+/-) in a unified patch. Hunk-aware:
 * the `---`/`+++` file-label lines are structure, but they only occur before
 * the first `@@` marker — a changed SOURCE line may itself start with ++ or
 * -- (e.g. `++counter`), so prefix-matching `+++`/`---` would drop it. */
export function changedLineIndices(patch: string): number[] {
	const indices: number[] = []
	const lines = patch.split('\n')
	let inHunk = false
	for (let i = 0; i < lines.length; i++) {
		if (lines[i].startsWith('@@')) {
			inHunk = true
			continue
		}
		if (!inHunk) continue
		if (lines[i].startsWith('+') || lines[i].startsWith('-')) indices.push(i)
	}
	return indices
}

/** Unified patch between two raw text files (no YAML wrapping — file contents
 * diff line-by-line as-is). Returns '' when identical; an absent side is
 * treated as empty, so a one-sided file reads as all additions/removals. */
export function textFilePatch(
	before: string | undefined,
	after: string | undefined,
	beforeLabel: string,
	afterLabel: string
): string {
	const beforeText = before ?? ''
	const afterText = after ?? ''
	if (beforeText === afterText) return ''
	return createTwoFilesPatch(beforeLabel, afterLabel, beforeText, afterText, '', '', { context: 3 })
}
