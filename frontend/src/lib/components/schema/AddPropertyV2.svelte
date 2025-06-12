<script lang="ts">
	import {
		type Schema,
		type SchemaProperty,
		modalToSchema,
		type ModalSchemaProperty
	} from '$lib/common'
	import { emptySchema, sendUserToast } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import AddPropertyFormV2 from './AddPropertyFormV2.svelte'
	interface Props {
		schema?: Schema | any
		trigger?: import('svelte').Snippet
	}

	let { schema = $bindable(emptySchema()), trigger }: Props = $props()

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

	// Internal state: bound to args builder modal
	let argError = ''
	let editing = false
	let oldArgName: string | undefined // when editing argument and changing name

	reorder()

	function reorder() {
		if (schema.order && Array.isArray(schema.order)) {
			const n = {}

			;(schema.order as string[]).forEach((x) => {
				if (schema.properties && schema.properties[x] != undefined) {
					n[x] = schema.properties[x]
				}
			})

			Object.keys(schema.properties ?? {})
				.filter((x) => !schema.order?.includes(x))
				.forEach((x) => {
					n[x] = schema.properties[x]
				})
			schema.properties = n
		}
	}

	function syncOrders() {
		if (schema) {
			schema.order = Object.keys(schema.properties ?? {})
		}
	}

	export function handleAddOrEditArgument(modalProperty: ModalSchemaProperty): void {
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
			if (!schema.properties) {
				schema.properties = {}
			}
			if (!schema.required) {
				schema.required = []
			}
			if (!schema.order || !Array.isArray(schema.order)) {
				syncOrders()
			}
			schema.properties[modalProperty.name] = modalToSchema(modalProperty)
			if (modalProperty.required) {
				if (!schema.required.includes(modalProperty.name)) {
					schema.required.push(modalProperty.name)
				}
			} else if (schema.required.includes(modalProperty.name)) {
				const index = schema.required.indexOf(modalProperty.name, 0)
				if (index > -1) {
					schema.required.splice(index, 1)
				}
			}

			if (editing && oldArgName && oldArgName !== modalProperty.name) {
				let oldPosition = schema.order.indexOf(oldArgName)
				schema.order[oldPosition] = modalProperty.name
				reorder()
				handleDeleteArgument([oldArgName])
			}

			if (!schema.order?.includes(modalProperty.name)) {
				schema.order.push(modalProperty.name)
			}
			modalProperty = Object.assign({}, DEFAULT_PROPERTY)
			editing = false
			oldArgName = undefined
		}

		schema = $state.snapshot(schema)

		if (argError !== '') {
			sendUserToast(argError, true)
		}

		dispatch('change', schema)
	}

	export function handleDeleteArgument(argPath: string[]): void {
		try {
			let modifiedObject: Schema | SchemaProperty = schema
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
				dispatch('change', schema)
			} else {
				throw Error('Argument not found!')
			}
			syncOrders()
			schema = $state.snapshot(schema)

			dispatch('change', schema)
		} catch (err) {
			sendUserToast(`Could not delete argument: ${err}`, true)
		}
	}

	const trigger_render = $derived(trigger)
</script>

<AddPropertyFormV2
	on:add={(e) => {
		try {
			handleAddOrEditArgument({
				...DEFAULT_PROPERTY,
				selectedType: 'string',
				name: e.detail.name
			})
			dispatch('addNew', e.detail.name)
		} catch (err) {
			sendUserToast(`Could not add argument: ${err}`, true)
		}
	}}
>
	{#snippet trigger()}
		{@render trigger_render?.()}
	{/snippet}
</AddPropertyFormV2>
