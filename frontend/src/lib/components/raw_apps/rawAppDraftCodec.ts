import { DEFAULT_DATA, type RawAppData } from './dataTableRefUtils'

// The raw-app draft shape stored under `UserDraft<RawAppDraft>`.
export type RawAppDraft = {
	files: Record<string, string>
	runnables: Record<string, any>
	data: RawAppData
	summary: string
	policy?: any
	custom_path?: string
}

function normalizeRawAppData(value: Record<string, any>): RawAppData {
	if (value.data) {
		if (value.data.creation) {
			return {
				tables: value.data.tables ?? [],
				datatable: value.data.creation.datatable,
				schema: value.data.creation.schema
			}
		}
		return value.data
	}
	if (value.datatables) {
		return { ...DEFAULT_DATA, tables: value.datatables }
	}
	if (value.dataTableRefs) {
		return { ...DEFAULT_DATA, tables: value.dataTableRefs }
	}
	return { ...DEFAULT_DATA }
}

export function appSourceToRawAppDraft(app: any, fallback?: any): RawAppDraft {
	const value = (app.value ?? {}) as Record<string, any>
	return {
		summary: app.summary ?? '',
		files: { ...(value.files ?? {}) },
		runnables: { ...(value.runnables ?? {}) },
		data: normalizeRawAppData(value),
		policy: app.policy ?? fallback?.policy,
		custom_path: app.custom_path ?? fallback?.custom_path
	}
}
