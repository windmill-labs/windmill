export interface Setting {
	label: string
	description?: string
	placeholder?: string
	cloudonly?: boolean
	tooltip?: string
	key: string
	fieldType:
		| 'text'
		| 'number'
		| 'boolean'
		| 'password'
		| 'select'
		| 'textarea'
		| 'seconds'
		| 'email'
	storage: SettingStorage
}

export type SettingStorage = 'setting' | 'config'
