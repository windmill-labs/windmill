// Deterministic backend response formats from hub script and CLI
export interface SyncResponse {
	success: true
	changes: Array<{
		type: 'added' | 'edited' | 'deleted'
		path: string
		codebase_changed?: boolean
	}>
	total: number
	settingsDiffResult?: {
		hasChanges: boolean
		diff: Record<string, { from: any; to: any }>
		local: SettingsObject
		backend: SettingsObject
	}
}

export interface SettingsResponse {
	success: true
	hasChanges: boolean
	local: SettingsObject
	backend: SettingsObject
	diff: Record<string, { from: any; to: any }>
	repository: string
}

export interface SettingsObject {
	include_path: string[]
	exclude_path: string[]
	extra_include_path: string[]
	include_type: string[]
}
