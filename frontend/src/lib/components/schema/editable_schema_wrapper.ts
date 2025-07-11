import type { Schema } from '$lib/common'

export type EditableSchemaWrapperProps = {
	schema: Schema | undefined | any
	uiOnly?: boolean
	noPreview?: boolean
	fullHeight?: boolean
	formatExtension?: string | undefined
	customUi?: {
		noAddPopover?: boolean
	}
	onSchemaChange?: ({ schema }: { schema: Schema }) => void
}
