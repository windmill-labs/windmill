<script lang="ts">
	import { type Schema, modalToSchema, type ModalSchemaProperty } from '$lib/common'
	import { emptySchema, sendUserToast } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import SimpleEditor from '../SimpleEditor.svelte'
	import AddPropertyForm from './AddPropertyForm.svelte'

	export let schema: Schema | any = emptySchema()

	export const DEFAULT_PROPERTY: ModalSchemaProperty = {
		selectedType: 'string',
		description: '',
		name: '',
		required: false
	}

	const dispatch = createEventDispatcher()

	if (!schema) {
		schema = emptySchema()
	}

	let schemaString: string = ''

	// Internal state: bound to args builder modal
	let argError = ''
	let editing = false
	let oldArgName: string | undefined // when editing argument and changing name
	let jsonEditor: SimpleEditor | undefined

	reorder(schema)

	function reorder(s: Schema) {
		if (s.order && Array.isArray(s.order)) {
			const n = {}

			;(s.order as string[]).forEach((x) => {
				if (s.properties && s.properties[x] != undefined) {
					n[x] = s.properties[x]
				}
			})

			Object.keys(s.properties ?? {})
				.filter((x) => !s.order?.includes(x))
				.forEach((x) => {
					n[x] = s.properties[x]
				})
			schema.properties = n
		}
	}

	function syncOrders(s: Schema) {
		if (s) {
			s.order = Object.keys(s.properties ?? {})
		}
	}

	function handleAddOrEditArgument(modalProperty: ModalSchemaProperty): void {
		// If editing the arg's name, oldName containing the old argument name must be provided
		argError = ''
		modalProperty.name = modalProperty.name.trim()
		if (modalProperty.name.length === 0) {
			argError = 'Arguments need to have a name'
		} else if (
			Object.keys(schema.properties ?? {}).includes(modalProperty.name) &&
			(!editing || (editing && oldArgName && oldArgName !== modalProperty.name))
		) {
			argError = 'There is already an argument with this name'
		} else {
			let newSchema = { ...schema }
			if (!newSchema.properties) {
				newSchema.properties = {}
			}
			if (!newSchema.required) {
				newSchema.required = []
			}
			if (!newSchema.order || !Array.isArray(newSchema.order)) {
				syncOrders(newSchema)
			}
			newSchema.properties = {
				...newSchema.properties,
				[modalProperty.name]: modalToSchema(modalProperty)
			}
			if (modalProperty.required) {
				if (!newSchema.required.includes(modalProperty.name)) {
					newSchema.required.push(modalProperty.name)
				}
			} else if (newSchema.required.includes(modalProperty.name)) {
				const index = newSchema.required.indexOf(modalProperty.name, 0)
				if (index > -1) {
					newSchema.required.splice(index, 1)
				}
			}

			if (editing && oldArgName && oldArgName !== modalProperty.name) {
				let oldPosition = newSchema.order.indexOf(oldArgName)
				newSchema.order[oldPosition] = modalProperty.name
				reorder(newSchema)
				handleDeleteArgument([oldArgName], newSchema)
			}

			if (!newSchema.order?.includes(modalProperty.name)) {
				newSchema.order.push(modalProperty.name)
			}
			modalProperty = Object.assign({}, DEFAULT_PROPERTY)
			editing = false
			oldArgName = undefined

			schema = newSchema
			schemaString = JSON.stringify(schema, null, '\t')
			jsonEditor?.setCode(schemaString)
		}

		if (argError !== '') {
			sendUserToast(argError, true)
		}
		dispatch('change', schema)
	}

	export function handleDeleteArgument(argPath: string[], nschema?: Schema): void {
		try {
			let modifiedObject: Schema = { ...(nschema ?? schema) }
			let modifiedProperties = modifiedObject.properties as object
			let argName = argPath.pop() as string

			argPath.forEach((property) => {
				if (Object.keys(modifiedProperties).includes(property)) {
					modifiedObject = modifiedProperties[property]
					modifiedProperties = modifiedObject.properties as object
				} else {
					throw Error('Nested argument not found!')
				}
			})

			if (Object.keys(modifiedProperties).includes(argName)) {
				delete modifiedProperties[argName]

				if (modifiedObject.required) {
					modifiedObject.required = schema.required.filter((arg) => arg !== argName)
				}
				if (modifiedObject.order) {
					modifiedObject.order = modifiedObject.order.filter((arg) => arg !== argName)
				}
				schema = modifiedObject
				schemaString = JSON.stringify(schema, null, '\t')
				dispatch('change', schema)
			} else {
				throw Error('Argument not found!')
			}
			dispatch('change', schema)
		} catch (err) {
			sendUserToast(`Could not delete argument: ${err}`, true)
		}
	}
</script>

<div class="flex">
	<AddPropertyForm
		on:add={(e) => {
			try {
				handleAddOrEditArgument({
					...DEFAULT_PROPERTY,
					selectedType: 'string',
					name: e.detail.name
				})
			} catch (err) {
				sendUserToast(`Could not add argument: ${err}`, true)
				console.log('Could not add argument', err)
			}
		}}
	/>
</div>
