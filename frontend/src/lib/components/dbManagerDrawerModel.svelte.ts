import { z } from 'zod'
import { useSearchParams } from '$lib/svelte5UtilsKit.svelte'
import type { DbInput, DbType } from './dbTypes'
import { isDbType } from './dbTypes'

const dbManagerSchema = z.object({
	db_type: z.string().nullable(),
	db_res_type: z.string().nullable(),
	db_res_path: z.string().nullable(),
	db_ducklake: z.string().nullable(),
	db_schema: z.string().nullable(),
	db_table: z.string().nullable(),
	db_datatable: z.string().nullable()
})

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

	let input: DbInput | undefined = $derived.by(() => {
		if (params.db_type === 'database') {
			const rt = params.db_res_type
			const rp = params.db_res_path
			if (!isDbType(rt ?? undefined) || !rp) return undefined
			return {
				type: 'database' as const,
				resourceType: rt as DbType,
				resourcePath: rp as string,
				specificSchema: (params.db_schema as string) ?? undefined,
				specificTable: (params.db_table as string) ?? undefined
			}
		}
		if (params.db_type === 'ducklake') {
			const dl = params.db_ducklake
			if (!dl) return undefined
			return {
				type: 'ducklake' as const,
				ducklake: dl as string,
				specificTable: (params.db_table as string) ?? undefined
			}
		}
		return undefined
	})

	const isDatatableInput = $derived(
		input?.type === 'database' && input.resourcePath.startsWith('datatable://')
	)

	// selectedDatatable reads directly from URL params (no separate $state needed)
	const selectedDatatable = $derived(
		params.db_datatable != null ? String(params.db_datatable) : undefined
	)

	const effectiveInput: DbInput | undefined = $derived.by(() => {
		if (!input) return undefined
		if (!isDatatableInput || !selectedDatatable) return input
		return {
			...input,
			resourcePath: `datatable://${selectedDatatable}`
		}
	})

	function clearAllParams() {
		params.db_type = null
		params.db_res_type = null
		params.db_res_path = null
		params.db_ducklake = null
		params.db_schema = null
		params.db_table = null
		params.db_datatable = null
	}

	function openDrawer(nInput: DbInput) {
		if (nInput.type === 'database') {
			params.db_type = 'database'
			params.db_res_type = nInput.resourceType
			params.db_res_path = nInput.resourcePath
			params.db_schema = nInput.specificSchema ?? null
			params.db_table = nInput.specificTable ?? null
			params.db_ducklake = null
			if (nInput.resourcePath.startsWith('datatable://')) {
				params.db_datatable = nInput.resourcePath.replace('datatable://', '')
			} else {
				params.db_datatable = null
			}
		} else {
			params.db_type = 'ducklake'
			params.db_ducklake = nInput.ducklake
			params.db_table = nInput.specificTable ?? null
			params.db_res_type = null
			params.db_res_path = null
			params.db_schema = null
			params.db_datatable = null
		}
	}

	function closeDrawer() {
		clearAllParams()
	}

	return {
		get input() {
			return input
		},
		get effectiveInput() {
			return effectiveInput
		},
		get isDatatableInput() {
			return isDatatableInput
		},
		get selectedDatatable() {
			return selectedDatatable
		},
		set selectedDatatable(v: string | undefined) {
			params.db_datatable = v ?? null
		},
		get selectedSchema() {
			return params.db_schema != null ? String(params.db_schema) : undefined
		},
		set selectedSchema(v: string | undefined) {
			params.db_schema = v ?? null
		},
		get selectedTable() {
			return params.db_table != null ? String(params.db_table) : undefined
		},
		set selectedTable(v: string | undefined) {
			params.db_table = v ?? null
		},
		get open() {
			return !!input
		},
		openDrawer,
		closeDrawer
	}
}
