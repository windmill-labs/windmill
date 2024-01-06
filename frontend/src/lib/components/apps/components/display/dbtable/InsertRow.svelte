<script lang="ts">
	import type { Schema, SchemaProperty } from '$lib/common'
	import LightweightSchemaForm from '$lib/components/LightweightSchemaForm.svelte'
	import { ColumnIdentity, type ColumnMetadata } from './utils'

	export let args: Record<string, any> = {}
	export let columnDefs: Array<
		{
			field: string
			ignored: boolean
			hideInsert: boolean
			overrideDefaultValue: boolean
			defaultUserValue: any
			defaultValueNull: boolean
		} & ColumnMetadata
	> = []

	type FieldMetadata = {
		type: string
		name: string
		isPrimaryKey: boolean
		defaultValue: string | undefined
		fieldType: 'text' | 'number' | 'checkbox' | 'date'
		identity: ColumnIdentity
		nullable: 'YES' | 'NO'
	}

	$: fields = columnDefs
		?.filter((t) => {
			const shouldFilter = t.isidentity !== ColumnIdentity.Always && t?.hideInsert === true

			return !shouldFilter
		})
		.map((column) => {
			const type = column.datatype
			const name = column.field
			const isPrimaryKey = column.isprimarykey
			const defaultValue = column.defaultValueNull ? null : column.defaultUserValue

			const baseType = type.split('(')[0]
			const validTextTypes = ['character varying', 'text']
			const validNumberTypes = ['integer', 'bigint', 'numeric', 'double precision']
			const validDateTypes = ['date', 'timestamp without time zone', 'timestamp with time zone']

			const fieldType = validTextTypes.includes(baseType)
				? 'text'
				: validNumberTypes.includes(baseType)
				? 'number'
				: baseType === 'boolean'
				? 'checkbox'
				: validDateTypes.includes(baseType)
				? 'date'
				: 'text'

			return {
				type,
				name,
				isPrimaryKey,
				defaultValue,
				fieldType,
				identity: column.isidentity,
				nullable: column.isnullable
			}
		}) as FieldMetadata[] | undefined

	function builtSchema(fields: FieldMetadata[]): Schema {
		const properties: { [name: string]: SchemaProperty } = {}
		const required: string[] = []

		fields.forEach((field) => {
			const schemaProperty: SchemaProperty = {
				type: field.fieldType
			}

			switch (field.fieldType) {
				case 'number':
					schemaProperty.type = 'number'
					const extractedDefaultValue = field.defaultValue
					schemaProperty.default = extractedDefaultValue ? Number(extractedDefaultValue) : undefined
					break
				case 'checkbox':
					schemaProperty.type = 'boolean'
					schemaProperty.default = field.defaultValue?.toLocaleLowerCase() === 'true'
					break
				case 'date':
					schemaProperty.type = 'string'
					schemaProperty.format = 'date-time'
					schemaProperty.default = field.defaultValue
					break
				case 'text':
				default:
					schemaProperty.type = 'string'
					schemaProperty.default = field.defaultValue
					break
			}

			properties[field.name] = schemaProperty

			const isRequired =
				(field.isPrimaryKey || field.defaultValue === undefined || field.defaultValue === null) &&
				field.nullable !== 'YES' &&
				![ColumnIdentity.Always, ColumnIdentity.ByDefault].includes(field.identity)

			if (isRequired) {
				required.push(field.name)
			}
		})

		return {
			$schema: 'http://json-schema.org/draft-07/schema#',
			type: 'object',
			properties,
			required
		}
	}

	$: schema = builtSchema(fields ?? []) as Schema

	export let isInsertable: boolean = false

	$: if (schema) {
		const requiredFields = schema.required ?? []
		const filledFields = Object.keys(args).filter(
			(key) => args[key] !== undefined && args[key] !== null
		)
		isInsertable = requiredFields.every((field) => filledFields.includes(field))
	}
</script>

<LightweightSchemaForm {schema} bind:args />
