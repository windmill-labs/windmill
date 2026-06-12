// Index management types and transforms.
//
// Indexes are managed imperatively (create / drop happen immediately), so unlike
// foreign keys we do not round-trip a structured column model back from the DB —
// the listing shows the canonical definition string returned by the database.
// Currently PostgreSQL-only; the backend markers reject other dialects.

/** A single key of a to-be-created index. */
export type IndexColumnInput = {
	value: string
	/** When true, `value` is an arbitrary SQL expression (e.g. `lower(email)`)
	 * rather than a plain column name. */
	isExpression?: boolean
}

/** User-provided values for creating an index. */
export type CreateIndexInput = {
	name?: string
	columns: IndexColumnInput[]
	unique?: boolean
	/** Index access method: btree (default) / hash / gin / gist / brin / spgist. */
	method?: string
	/** Partial-index predicate (raw SQL, placed after WHERE). */
	where?: string
	/** Covering (INCLUDE) columns. */
	include?: string[]
	/** Build the index without locking the table against writes. */
	concurrent?: boolean
}

/** Raw row shape returned by the LIST_INDEXES marker (PostgreSQL). */
export type RawIndex = {
	index_name: string
	index_def: string
	is_unique: boolean | string | number
	is_primary: boolean | string | number
	is_valid: boolean | string | number
	method: string
	size_bytes: number | string | null
	backs_constraint: boolean | string | number
}

/** Display model for an existing index. */
export type DbIndex = {
	name: string
	definition: string
	isUnique: boolean
	isPrimary: boolean
	isValid: boolean
	method: string
	sizeBytes: number
	/** Backs a PK/unique constraint — cannot be dropped directly (read-only). */
	backsConstraint: boolean
}

// Postgres booleans can arrive as true/false, 't'/'f', or 1/0 depending on how
// the driver serializes the result — coerce all of them.
function coerceBool(v: boolean | string | number | null | undefined): boolean {
	if (typeof v === 'boolean') return v
	if (typeof v === 'number') return v !== 0
	if (typeof v === 'string') return v === 't' || v.toLowerCase() === 'true' || v === '1'
	return false
}

function coerceNumber(v: number | string | null | undefined): number {
	if (typeof v === 'number') return v
	if (typeof v === 'string') {
		const n = Number(v)
		return Number.isFinite(n) ? n : 0
	}
	return 0
}

export function transformIndexes(raw: RawIndex[]): DbIndex[] {
	if (!raw || !Array.isArray(raw)) return []
	return raw.map((r) => ({
		name: r.index_name,
		definition: r.index_def,
		isUnique: coerceBool(r.is_unique),
		isPrimary: coerceBool(r.is_primary),
		isValid: coerceBool(r.is_valid),
		method: r.method,
		sizeBytes: coerceNumber(r.size_bytes),
		backsConstraint: coerceBool(r.backs_constraint)
	}))
}

/** Human-readable byte size for display (e.g. "12 kB"). */
export function formatIndexSize(bytes: number): string {
	if (!bytes || bytes <= 0) return '0 B'
	const units = ['B', 'kB', 'MB', 'GB', 'TB']
	let i = 0
	let n = bytes
	while (n >= 1024 && i < units.length - 1) {
		n /= 1024
		i++
	}
	return `${i === 0 ? n : n.toFixed(1)} ${units[i]}`
}
