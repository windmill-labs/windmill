export interface Setting {
	label: string
	placeholder?: string
	tooltip?: string
	key: string
	fieldType: 'text' | 'number' | 'boolean' | 'password' | 'select' | 'textarea' | 'seconds'
	storage: 'setting' | 'config'
}
