<script lang="ts">
	import {
		type Schema,
		type SchemaProperty,
		modalToSchema,
		type ModalSchemaProperty
	} from '$lib/common'
	import { emptySchema, emptyString, sendUserToast } from '$lib/utils'
	import { Button } from '../common'
	import { createEventDispatcher } from 'svelte'
	import SchemaModal, { DEFAULT_PROPERTY } from '../SchemaModal.svelte'
	import SimpleEditor from '../SimpleEditor.svelte'
	import Toggle from '../Toggle.svelte'
	import Tooltip from '../Tooltip.svelte'
	import Portal from 'svelte-portal'
	import { Plus } from 'lucide-svelte'
	import type VariableEditor from '../VariableEditor.svelte'

	export let isFlowInput = false
	export let variableEditor: VariableEditor | undefined = undefined
	export let schema: Schema | any = emptySchema()
	export let lightMode: boolean = false

	const dispatch = createEventDispatcher()

	if (!schema) {
		schema = emptySchema()
	}

	let schemaModal: SchemaModal
	let schemaString: string = ''

	// Internal state: bound to args builder modal
	let argError = ''
	let editing = false
	let oldArgName: string | undefined // when editing argument and changing name

	let viewJsonSchema = false
	let jsonEditor: SimpleEditor

	// Binding is not enough because monaco Editor does not support two-way binding
	export function getSchema(): Schema {
		try {
			if (viewJsonSchema) {
				schema = JSON.parse(schemaString)
				return schema
			} else {
				schemaString = JSON.stringify(schema, null, '\t')
				return schema
			}
		} catch (err) {
			throw Error(`Error: input is not a valid schema: ${err}`)
		}
	}

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

			schemaModal.closeDrawer()
		}

		schema = schema
		schemaString = JSON.stringify(schema, null, '\t')
		jsonEditor?.setCode(schemaString)

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

				modifiedObject.required = schema.required.filter((arg) => arg !== argName)
				if (modifiedObject.order) {
					modifiedObject.order = modifiedObject.order.filter((arg) => arg !== argName)
				}
				schema = schema
				schemaString = JSON.stringify(schema, null, '\t')
				dispatch('change', schema)
			} else {
				throw Error('Argument not found!')
			}
			syncOrders()
		} catch (err) {
			sendUserToast(`Could not delete argument: ${err}`, true)
		}
	}

	function switchTab(): void {
		if (viewJsonSchema) {
			if (schemaString === '') {
				schemaString = JSON.stringify(emptySchema(), null, 4)
			}
			viewJsonSchema = false
		} else {
			schemaString = JSON.stringify(schema, null, '\t')
			viewJsonSchema = true
		}
	}

	let error = ''
</script>

<div class="flex flex-col">
	<div class="flex justify-between items-center gap-x-2">
		<div>
			<Button
				variant="contained"
				color="dark"
				size="xs"
				startIcon={{ icon: Plus }}
				on:click={() => {
					schemaModal.openDrawer(Object.assign({}, DEFAULT_PROPERTY))
				}}
				id="flow-editor-add-property"
			>
				Add Argument
			</Button>
		</div>

		<div class="flex items-center">
			<Toggle
				size={lightMode ? 'xs' : 'sm'}
				on:change={() => switchTab()}
				options={{
					right: 'As JSON'
				}}
			/>
			<div class="ml-2">
				<Tooltip
					documentationLink="https://www.windmill.dev/docs/core_concepts/json_schema_and_parsing#script-parameters-to-json-schema"
				>
					Arguments can be edited either using the wizard, or by editing their JSON Schema.
				</Tooltip>
			</div>
		</div>
	</div>

	<!--json schema or table view-->
	<div class="h-full">
		{#if viewJsonSchema}
			<div class="border rounded p-2">
				<SimpleEditor
					small
					bind:this={jsonEditor}
					fixedOverflowWidgets={false}
					on:change={() => {
						try {
							schema = JSON.parse(schemaString)
							error = ''
						} catch (err) {
							error = err.message
						}
					}}
					bind:code={schemaString}
					lang="json"
					class="small-editor"
				/>
			</div>
			{#if !emptyString(error)}
				<div class="text-red-400">{error}</div>
			{:else}
				<div><br /></div>
			{/if}
		{/if}
	</div>
</div>

<Portal>
	<SchemaModal
		{isFlowInput}
		bind:this={schemaModal}
		bind:error={argError}
		on:save={(e) => handleAddOrEditArgument(e.detail)}
		bind:editing
		bind:oldArgName
		propsNames={Object.keys(schema.properties ?? {})}
		{variableEditor}
	/>
</Portal>
