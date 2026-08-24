/**
 * The `changes` column of `trigger_history`, in the three shapes the backend
 * writes it (see `windmill-common/src/trigger_history.rs`):
 *
 *   null                                 no field-level diff — a delete
 *   { truncated_fields: [...] }          the diff exceeded the 32 KiB cap
 *   { <field>: { old?, new } }           the fields that changed
 *
 * Parsing it into this closed set up front is what lets the viewer lay every
 * entry out the same way, instead of rendering whatever tree came back.
 */

/**
 * How one field moved. A missing or null side is treated as the field having no
 * value: the writer omits `old` where it never read one, and the backend writes
 * `new: null` for a cleared column, so both collapse to added/removed.
 */
export type FieldChange =
	| { kind: 'added'; field: string; next: unknown }
	| { kind: 'removed'; field: string; prev: unknown | undefined }
	| { kind: 'changed'; field: string; prev: unknown; next: unknown }

export type ParsedChanges =
	| { kind: 'none' }
	| { kind: 'truncated'; fields: string[] }
	| { kind: 'fields'; changes: FieldChange[] }

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value)
}

export function parseChanges(changes: unknown): ParsedChanges {
	if (!isRecord(changes)) return { kind: 'none' }

	const truncated = changes['truncated_fields']
	if (Array.isArray(truncated)) {
		return { kind: 'truncated', fields: truncated.map(String) }
	}

	const parsed: FieldChange[] = []
	for (const [field, entry] of Object.entries(changes)) {
		if (!isRecord(entry)) continue
		// A null on either side is the field having no value, which reads as
		// added/removed rather than as a change to or from `null`.
		const prev = entry['old'] ?? undefined
		const next = entry['new'] ?? undefined
		if (next === undefined) {
			parsed.push({ kind: 'removed', field, prev })
		} else if (prev === undefined) {
			parsed.push({ kind: 'added', field, next })
		} else {
			parsed.push({ kind: 'changed', field, prev, next })
		}
	}
	// Alphabetical: jsonb hands back its own key order, which is by length then
	// bytes and would shuffle as values change.
	parsed.sort((a, b) => a.field.localeCompare(b.field))

	return parsed.length === 0 ? { kind: 'none' } : { kind: 'fields', changes: parsed }
}

/** True when a value needs the tree viewer rather than a one-line rendering. */
export function isComplex(value: unknown): boolean {
	return typeof value === 'object' && value !== null
}

/** One-line rendering of a scalar, quoted so `""` and `"0"` stay visible. */
export function formatScalar(value: unknown): string {
	return typeof value === 'string' ? JSON.stringify(value) : String(value)
}
