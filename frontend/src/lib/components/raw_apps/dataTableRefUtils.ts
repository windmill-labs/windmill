/** Internal representation of a data table reference */
export interface DataTableRef {
	/** The datatable name from workspace settings */
	datatable: string
	/** Optional schema filter */
	schema?: string
	/** Optional table filter */
	table?: string
}

/** Top-level data configuration for raw apps */
export interface RawAppData {
	/** Table references for the app */
	tables: string[]
	/** The datatable name for table creation (if specified) */
	datatable: string | undefined
	/** The schema for table creation (if specified) */
	schema: string | undefined
}

/** Default data configuration */
export const DEFAULT_DATA: RawAppData = {
	tables: [],
	datatable: undefined,
	schema: undefined
}

/**
 * Parse a string ref into a DataTableRef object
 * Format: <datatableName>/<schema>:<table> or <datatableName>/<table> (for public schema)
 */
export function parseDataTableRef(ref: string): DataTableRef {
	const slashIdx = ref.indexOf('/')
	if (slashIdx === -1) {
		return { datatable: ref }
	}
	const datatable = ref.slice(0, slashIdx)
	const rest = ref.slice(slashIdx + 1)

	const colonIdx = rest.indexOf(':')
	if (colonIdx === -1) {
		// No colon means public schema: <datatableName>/<table>
		return { datatable, schema: 'public', table: rest }
	}
	// Has colon: <datatableName>/<schema>:<table>
	const schema = rest.slice(0, colonIdx)
	const table = rest.slice(colonIdx + 1)
	return { datatable, schema, table }
}

/**
 * Format a DataTableRef object into a string
 * Format: <datatableName>/<schema>:<table> or <datatableName>/<table> (for public schema)
 */
export function formatDataTableRef(ref: DataTableRef): string {
	if (!ref.table) {
		return ref.datatable
	}
	if (!ref.schema || ref.schema === 'public') {
		return `${ref.datatable}/${ref.table}`
	}
	return `${ref.datatable}/${ref.schema}:${ref.table}`
}
