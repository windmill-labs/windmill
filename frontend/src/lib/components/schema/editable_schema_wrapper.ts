import type { Schema } from '$lib/common'

export type EditableSchemaWrapperProps = {
    schema: Schema | undefined | any
    uiOnly?: boolean
    noPreview?: boolean
    fullHeight?: boolean
    formatExtension?: string | undefined
    onSchemaChange?: ({ schema }: { schema: Schema }) => void
}