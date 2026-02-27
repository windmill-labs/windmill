import { z } from 'zod'
import { useSearchParams } from '$lib/svelte5UtilsKit.svelte'
import type { DbInput, DbType } from './dbTypes'
import { isDbType } from './dbTypes'

const dbManagerSchema = z.object({
	dbmanager_type: z.string().nullable(),
	dbmanager_resource_type: z.string().nullable(),
	dbmanager_resource_path: z.string().nullable(),
	dbmanager_ducklake: z.string().nullable(),
	dbmanager_schema: z.string().nullable(),
	dbmanager_table: z.string().nullable(),
	dbmanager_datatable: z.string().nullable()
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
		if (params.dbmanager_type === 'database') {
			const rt = params.dbmanager_resource_type
			const rp = params.dbmanager_resource_path
			if (!isDbType(rt ?? undefined) || !rp) return undefined
			return {
				type: 'database' as const,
				resourceType: rt as DbType,
				resourcePath: rp as string,
				specificSchema: (params.dbmanager_schema as string) ?? undefined,
				specificTable: (params.dbmanager_table as string) ?? undefined
			}
		}
		if (params.dbmanager_type === 'ducklake') {
			const dl = params.dbmanager_ducklake
			if (!dl) return undefined
			return {
				type: 'ducklake' as const,
				ducklake: dl as string,
				specificTable: (params.dbmanager_table as string) ?? undefined
			}
		}
		return undefined
	})

	const isDatatableInput = $derived(
		input?.type === 'database' && input.resourcePath.startsWith('datatable://')
	)

	// selectedDatatable reads directly from URL params (no separate $state needed)
	const selectedDatatable = $derived(
		params.dbmanager_datatable != null ? String(params.dbmanager_datatable) : undefined
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
		params.dbmanager_type = null
		params.dbmanager_resource_type = null
		params.dbmanager_resource_path = null
		params.dbmanager_ducklake = null
		params.dbmanager_schema = null
		params.dbmanager_table = null
		params.dbmanager_datatable = null
	}

	function openDrawer(nInput: DbInput) {
		if (nInput.type === 'database') {
			params.dbmanager_type = 'database'
			params.dbmanager_resource_type = nInput.resourceType
			params.dbmanager_resource_path = nInput.resourcePath
			params.dbmanager_schema = nInput.specificSchema ?? null
			params.dbmanager_table = nInput.specificTable ?? null
			params.dbmanager_ducklake = null
			if (nInput.resourcePath.startsWith('datatable://')) {
				params.dbmanager_datatable = nInput.resourcePath.replace('datatable://', '')
			} else {
				params.dbmanager_datatable = null
			}
		} else {
			params.dbmanager_type = 'ducklake'
			params.dbmanager_ducklake = nInput.ducklake
			params.dbmanager_table = nInput.specificTable ?? null
			params.dbmanager_resource_type = null
			params.dbmanager_resource_path = null
			params.dbmanager_schema = null
			params.dbmanager_datatable = null
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
			params.dbmanager_datatable = v ?? null
		},
		get selectedSchema() {
			return params.dbmanager_schema != null ? String(params.dbmanager_schema) : undefined
		},
		set selectedSchema(v: string | undefined) {
			params.dbmanager_schema = v ?? null
		},
		get selectedTable() {
			return params.dbmanager_table != null ? String(params.dbmanager_table) : undefined
		},
		set selectedTable(v: string | undefined) {
			params.dbmanager_table = v ?? null
		},
		get open() {
			return !!input
		},
		openDrawer,
		closeDrawer
	}
}
