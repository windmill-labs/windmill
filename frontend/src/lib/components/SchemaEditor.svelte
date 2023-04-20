<script lang="ts">
	import type { Schema, SchemaProperty, PropertyDisplayInfo } from '$lib/common'
	import { emptySchema, emptyString, sendUserToast } from '$lib/utils'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import { Button } from './common'
	import { createEventDispatcher } from 'svelte'
	import type { ModalSchemaProperty } from './SchemaModal.svelte'
	import SchemaModal, { DEFAULT_PROPERTY, schemaToModal } from './SchemaModal.svelte'
	import PropertyRow from './PropertyRow.svelte'
	import SimpleEditor from './SimpleEditor.svelte'
	import TableCustom from './TableCustom.svelte'
	import Toggle from './Toggle.svelte'
	import Tooltip from './Tooltip.svelte'
	import { flip } from 'svelte/animate'
	import Portal from 'svelte-portal'

	const dispatch = createEventDispatcher()

	const moveAnimationDuration = 500

	export let schema: Schema = emptySchema()
	if (!schema) {
		schema = emptySchema()
	}

	let schemaModal: SchemaModal
	let schemaString: string = ''

	// Internal state: bound to args builder modal
	let modalProperty: ModalSchemaProperty = Object.assign({}, DEFAULT_PROPERTY)
	let argError = ''
	let editing = false
	let oldArgName: string | undefined // when editing argument and changing name

	let viewJsonSchema = false

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

	function modalToSchema(schema: ModalSchemaProperty): SchemaProperty {
		return {
			type: schema.selectedType,
			description: schema.description,
			pattern: schema.pattern,
			default: schema.default,
			enum: schema.enum_,
			items: schema.items,
			contentEncoding: schema.contentEncoding,
			format: schema.format
		}
	}
	function handleAddOrEditArgument(): void {
		// If editing the arg's name, oldName containing the old argument name must be provided
		argError = ''
		if (modalProperty.name.length === 0) {
			argError = 'Arguments need to have a name'
		} else if (Object.keys(schema.properties).includes(modalProperty.name) && !editing) {
			argError = 'There is already an argument with this name'
		} else {
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
				handleDeleteArgument(oldArgName)
			}
			modalProperty = Object.assign({}, DEFAULT_PROPERTY)
			editing = false
			oldArgName = undefined

			schemaModal.closeDrawer()
		}
		schema = schema
		schemaString = JSON.stringify(schema, null, '\t')
		dispatch('change', schema)
	}

	function handleStartEditEvent(event: CustomEvent): void {
		startEditArgument(event.detail)
	}

	function startEditArgument(argName: string): void {
		argError = ''
		if (Object.keys(schema.properties).includes(argName)) {
			editing = true
			modalProperty = schemaToModal(
				schema.properties[argName],
				argName,
				schema.required.includes(argName)
			)
			oldArgName = argName
			schemaModal.openDrawer()
		} else {
			sendUserToast(`This argument does not exist and can't be edited`, true)
		}
	}

	function handleDeleteEvent(event: CustomEvent): void {
		handleDeleteArgument(event.detail)
	}

	function handleDeleteArgument(argName: string): void {
		try {
			if (Object.keys(schema.properties).includes(argName)) {
				delete schema.properties[argName]

				schema.required = schema.required.filter((arg) => arg !== argName)

				schema = schema
				schemaString = JSON.stringify(schema, null, '\t')
				dispatch('change', schema)
			} else {
				throw Error('Argument not found!')
			}
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

	function handleChangePositionEvent(event: CustomEvent): void {
		changePosition(event.detail.i, event.detail.up)
	}

	function changePosition(i: number, up: boolean): any {
		isAnimated = true
		setTimeout(() => {
			isAnimated = false
		}, moveAnimationDuration)
		const entries = Object.entries(schema.properties)
		var element = entries[i]
		entries.splice(i, 1)
		entries.splice(up ? i - 1 : i + 1, 0, element)
		schema.properties = Object.fromEntries(entries)
	}
	let isAnimated = false
	let error = ''

	function schemaPropertiesToDisplay(schema: Schema): PropertyDisplayInfo[] {
		return propertiesToDisplay(schema.properties, schema.required, 0)
	}

	function propertiesToDisplay(
		properties: { [name: string]: SchemaProperty },
		required: string[],
		depth: number
	): PropertyDisplayInfo[] {
		return Object.entries(properties)
			.map(([name, property], index) => {
				const isRequired = required.includes(name)
				const displayProperty = {
					property,
					name,
					isRequired,
					depth,
					index,
					propertiesNumber: Object.entries(properties).length
				}
				if (property.type === 'object') {
					return [
						displayProperty,
						...propertiesToDisplay(property.properties || {}, property.required || [], depth + 1)
					]
				} else {
					return [displayProperty]
				}
			})
			.flat()
	}
</script>

<div class="flex flex-col">
	<div class="flex justify-between gap-x-2">
		<div>
			<Button
				variant="contained"
				color="dark"
				size="sm"
				startIcon={{ icon: faPlus }}
				on:click={() => {
					modalProperty = Object.assign({}, DEFAULT_PROPERTY)
					schemaModal.openDrawer()
				}}
			>
				Add Property
			</Button>
		</div>

		<div class="flex items-center">
			<Toggle
				on:change={() => switchTab()}
				options={{
					right: 'As JSON'
				}}
			/>
			<div class="ml-2">
				<Tooltip>
					Arguments can be edited either using the wizard, or by editing their JSON Schema,
					<a href="https://docs.windmill.dev/docs/reference/#script-parameters-to-json-schema"
						>see docs</a
					>
				</Tooltip>
			</div>
		</div>
	</div>

	<!--json schema or table view-->
	<div class="h-full">
		{#if !viewJsonSchema}
			<div class="h-full">
				{#if schema.properties && Object.keys(schema.properties).length > 0 && schema.required}
					<TableCustom>
						<tr slot="header-row">
							<th>Name</th>
							<th>Type</th>
							<th>Description</th>
							<th>Default</th>
							<th>Required</th>
							<th />
						</tr>
						<tbody slot="body">
							{#each schemaPropertiesToDisplay(schema) as displayInfo, i (displayInfo.name)}
								<tr animate:flip={{duration:moveAnimationDuration}}>
									<PropertyRow
										{displayInfo}
										{isAnimated}
										on:startEditArgument={handleStartEditEvent}
										on:deleteArgument={handleDeleteEvent}
										on:changePosition={handleChangePositionEvent}
									/>
								</tr>
							{/each}
						</tbody>
					</TableCustom>
				{:else}
					<div class="text-gray-700 text-xs italic mt-2">This schema has no arguments.</div>
				{/if}
			</div>
		{:else}
			{#if !emptyString(error)}<span class="text-red-400">{error}</span>{:else}<div
					class="py-6"
				/>{/if}
			<div class="border rounded p-2">
				<SimpleEditor
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
		{/if}
	</div>
</div>

<Portal>
	<SchemaModal
		bind:this={schemaModal}
		bind:property={modalProperty}
		bind:error={argError}
		on:save={handleAddOrEditArgument}
		bind:editing
		bind:oldArgName
	/>
</Portal>
