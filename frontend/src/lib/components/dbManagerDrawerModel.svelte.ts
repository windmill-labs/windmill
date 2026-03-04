import { z } from 'zod'
import { useSearchParams } from '$lib/svelte5UtilsKit.svelte'
import type { DbInput, DbType } from './dbTypes'
import { isDbType } from './dbTypes'

/**
 * Single URL param `dbm` encodes the full DB manager state:
 *   firstSegment~path~schema.table
 *
 * firstSegment:
 *   datatable              – database with datatable:// resource (resourceType always postgresql)
 *   ducklake               – ducklake connection
 *   postgresql, mysql, …   – regular database (the segment IS the resource type)
 *
 * schema.table (third segment, optional):
 *   schema.table  – both
 *   .table        – table only
 *   schema.       – schema only
 *   (omitted)     – neither
 *
 * Default schemas (omitted from URL, restored on parse):
 *   datatable → public
 *   ducklake  → main
 *
 * Examples:
 *   datatable~main~.customers          (schema "public" implied)
 *   ducklake~main~.orders              (schema "main" implied)
 *   postgresql~$res:u/user/my_pg~public.customers
 */

const dbManagerSchema = z.object({
	dbm: z.string().nullable()
})

interface ParsedDbm {
	type: 'database' | 'datatable' | 'ducklake'
	path: string
	resType?: string
	schema?: string
	table?: string
}

function parseDbm(raw: unknown): ParsedDbm | null {
	if (!raw || typeof raw !== 'string') return null
	const parts = raw.split('~')
	if (parts.length < 2 || !parts[1]) return null

	const firstSeg = parts[0]
	const path = parts[1]
	const schemaTable = parts[2] ?? ''

	let type: ParsedDbm['type']
	let resType: string | undefined
	if (firstSeg === 'datatable') {
		type = 'datatable'
	} else if (firstSeg === 'ducklake') {
		type = 'ducklake'
	} else if (isDbType(firstSeg)) {
		type = 'database'
		resType = firstSeg
	} else {
		return null
	}

	let schema: string | undefined
	let table: string | undefined
	if (schemaTable) {
		const dotIdx = schemaTable.indexOf('.')
		if (dotIdx === 0) {
			table = schemaTable.slice(1) || undefined
		} else if (dotIdx === schemaTable.length - 1) {
			schema = schemaTable.slice(0, -1) || undefined
		} else if (dotIdx > 0) {
			schema = schemaTable.slice(0, dotIdx)
			table = schemaTable.slice(dotIdx + 1)
		}
	}

	// Restore default schema when omitted for datatable/ducklake
	if (!schema && table && type in defaultSchemas) {
		schema = defaultSchemas[type]
	}

	return { type, path, resType, schema, table }
}

const defaultSchemas: Record<string, string> = { datatable: 'public', ducklake: 'main' }

function buildDbm(p: ParsedDbm): string {
	const firstSeg = p.type === 'database' ? p.resType! : p.type
	const schema = p.schema === defaultSchemas[p.type] ? undefined : p.schema
	let schemaTable = ''
	if (schema && p.table) {
		schemaTable = `${schema}.${p.table}`
	} else if (p.table) {
		schemaTable = `.${p.table}`
	} else if (schema) {
		schemaTable = `${schema}.`
	}
	return schemaTable ? `${firstSeg}~${p.path}~${schemaTable}` : `${firstSeg}~${p.path}`
}

export interface DbManagerUriState {
	readonly input: DbInput | undefined
	readonly effectiveInput: DbInput | undefined
	readonly isDatatableInput: boolean
	selectedDatatable: string | undefined
	selectedSchema: string | undefined
	selectedTable: string | undefined
	readonly open: boolean
	openDrawer: (nInput: DbInput) => void
	closeDrawer: () => void
}

export function useDbManagerUriState(): DbManagerUriState {
	const params = useSearchParams(dbManagerSchema)

	const parsed = $derived(parseDbm(params.dbm))

	let input: DbInput | undefined = $derived.by(() => {
		if (!parsed) return undefined
		if (parsed.type === 'ducklake') {
			return {
				type: 'ducklake' as const,
				ducklake: parsed.path,
				specificTable: parsed.table
			}
		}
		// datatable or database
		const resType = parsed.type === 'datatable' ? 'postgresql' : parsed.resType
		if (!isDbType(resType ?? undefined)) return undefined
		return {
			type: 'database' as const,
			resourceType: resType as DbType,
			resourcePath: parsed.type === 'datatable' ? `datatable://${parsed.path}` : parsed.path,
			specificSchema: parsed.schema,
			specificTable: parsed.table
		}
	})

	const isDatatableInput = $derived(parsed?.type === 'datatable')

	function updateField(updates: Partial<ParsedDbm>) {
		const p = parseDbm(params.dbm)
		if (!p) return
		Object.assign(p, updates)
		params.dbm = buildDbm(p)
	}

	function openDrawer(nInput: DbInput) {
		if (nInput.type === 'database') {
			const isDatatable = nInput.resourcePath.startsWith('datatable://')
			params.dbm = buildDbm({
				type: isDatatable ? 'datatable' : 'database',
				path: isDatatable ? nInput.resourcePath.slice('datatable://'.length) : nInput.resourcePath,
				resType: isDatatable ? undefined : nInput.resourceType,
				schema: nInput.specificSchema,
				table: nInput.specificTable
			})
		} else {
			params.dbm = buildDbm({
				type: 'ducklake',
				path: nInput.ducklake,
				table: nInput.specificTable
			})
		}
	}

	function closeDrawer() {
		params.dbm = null
	}

	return {
		get input() {
			return input
		},
		get effectiveInput() {
			return input
		},
		get isDatatableInput() {
			return isDatatableInput
		},
		get selectedDatatable() {
			return parsed?.type === 'datatable' ? parsed.path : undefined
		},
		set selectedDatatable(v: string | undefined) {
			if (v) updateField({ path: v })
		},
		get selectedSchema() {
			return parsed?.schema
		},
		set selectedSchema(v: string | undefined) {
			updateField({ schema: v })
		},
		get selectedTable() {
			return parsed?.table
		},
		set selectedTable(v: string | undefined) {
			updateField({ table: v })
		},
		get open() {
			return !!input
		},
		openDrawer,
		closeDrawer
	}
}
