import type { Schema } from '$lib/common'

export type EditableSchemaWrapperProps = {
	schema: Schema | undefined | any
	uiOnly?: boolean
	noPreview?: boolean
	fullHeight?: boolean
	formatExtension?: string | undefined
	isFileset?: boolean | undefined
	showSensitiveToggle?: boolean
	customUi?: {
		noAddPopover?: boolean
	}
}
