export interface Setting {
	label: string
	placeholder?: string
	tooltip?: string
	key: string
	fieldType: 'text' | 'number' | 'boolean' | 'password' | 'select' | 'textarea' | 'seconds'
	storage: SettingStorage
}

export type SettingStorage = 'setting' | 'config' | 'smtp'
