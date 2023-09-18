import type { InputType } from '$lib/components/apps/inputType'

export interface Setting {
	label: string
	placeholder?: string
	tooltip?: string
	key: string
	fieldType: InputType | undefined
	subFieldType?: InputType
}
