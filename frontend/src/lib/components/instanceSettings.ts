export interface Setting {
	label: string
	description?: string
	placeholder?: string
	cloudonly?: boolean
	ee_only?: string
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
		| 'license_key'
	storage: SettingStorage
	isValid?: (value: any) => boolean
}

export type SettingStorage = 'setting' | 'config'
