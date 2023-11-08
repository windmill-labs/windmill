<script lang="ts">
	import type { Schema, SchemaProperty, PropertyDisplayInfo } from '$lib/common'
	import { emptySchema, emptyString, sendUserToast } from '$lib/utils'
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
	import { Plus } from 'lucide-svelte'

	export let isFlowInput = false

	const dispatch = createEventDispatcher()

	export let lightMode: boolean = false

	const moveAnimationDuration = 300

	export let schema: Schema | any = emptySchema()

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
				n[x] = schema.properties[x]
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

	function modalToSchema(schema: ModalSchemaProperty): SchemaProperty {
		return {
			type: schema.selectedType,
			description: schema.description,
			pattern: schema.pattern,
			default: schema.default,
			enum: schema.enum_,
			items: schema.items,
			contentEncoding: schema.contentEncoding,
			format: schema.format,
			properties: schema.schema?.properties,
			required: schema.schema?.required
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
				handleDeleteArgument([oldArgName])
			}
			modalProperty = Object.assign({}, DEFAULT_PROPERTY)
			editing = false
			oldArgName = undefined

			schemaModal.closeDrawer()
		}

		schema = schema
		syncOrders()
		schemaString = JSON.stringify(schema, null, '\t')
		jsonEditor?.setCode(schemaString)
		dispatch('change', schema)
	}

	function handleStartEditEvent(event: CustomEvent): void {
		startEditArgument(event.detail)
	}

	function startEditArgument(argName: string): void {
		argError = ''
		if (Object.keys(schema.properties).includes(argName)) {
			editing = true
			const modalProperty = schemaToModal(
				schema.properties[argName],
				argName,
				schema.required.includes(argName)
			)
			oldArgName = argName
			schemaModal.openDrawer(modalProperty)
		} else {
			sendUserToast(`This argument does not exist and can't be edited`, true)
		}
	}

	function handleDeleteEvent(event: CustomEvent): void {
		handleDeleteArgument(event.detail)
	}

	function handleDeleteArgument(argPath: string[]): void {
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
		syncOrders()
	}

	let isAnimated = false
	let error = ''

	function schemaPropertiesToDisplay(schema: Schema): PropertyDisplayInfo[] {
		return propertiesToDisplay(schema.properties, schema.required, [])
	}

	function propertiesToDisplay(
		properties: { [name: string]: SchemaProperty },
		required: string[],
		path: string[]
	): PropertyDisplayInfo[] {
		return Object.entries(properties)
			.map(([name, property], index) => {
				const isRequired = required.includes(name)
				const displayInfo = {
					property: property,
					name: name,
					isRequired: isRequired,
					path: path,
					index: index,
					propertiesNumber: Object.entries(properties).length
				}
				if (property.type === 'object') {
					const newPath = [...path, name]
					return [
						displayInfo,
						...propertiesToDisplay(property.properties || {}, property.required || [], newPath)
					]
				} else {
					return [displayInfo]
				}
			})
			.flat()
	}

	/* Small hash function to generate a unique key for each property */
	function displayInfoKey(displayInfo: PropertyDisplayInfo): string {
		const pathLengthString = displayInfo.path.length.toString()
		return pathLengthString + [...displayInfo.path, displayInfo.name].join(pathLengthString)
	}
</script>

<div class="flex flex-col">
	<div class="flex justify-between gap-x-2">
		<div>
			<Button
				variant="contained"
				color="dark"
				size="sm"
				startIcon={{ icon: Plus }}
				on:click={() => {
					schemaModal.openDrawer(Object.assign({}, DEFAULT_PROPERTY))
				}}
				id="flow-editor-add-property"
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
		{#if !viewJsonSchema}
			<div class="h-full">
				{#if schema.properties && Object.keys(schema.properties).length > 0 && schema.required}
					<TableCustom>
						<tr slot="header-row">
							<th>Name</th>
							<th>Type</th>
							{#if !lightMode}
								<th>Default</th>
								<th>Description</th>
							{/if}
							<th />
						</tr>
						<tbody slot="body">
							{#key schema.required}
								{#each schemaPropertiesToDisplay(schema) as displayInfo (displayInfoKey(displayInfo))}
									<tr animate:flip={{ duration: moveAnimationDuration }}>
										<PropertyRow
											{displayInfo}
											{isAnimated}
											{lightMode}
											on:startEditArgument={handleStartEditEvent}
											on:deleteArgument={handleDeleteEvent}
											on:changePosition={handleChangePositionEvent}
										/>
									</tr>
								{/each}
							{/key}
						</tbody>
					</TableCustom>
				{:else}
					<div class="text-secondary text-xs italic mt-2">This schema has no arguments.</div>
				{/if}
			</div>
		{:else}
			<div class="border rounded p-2">
				<SimpleEditor
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
	/>
</Portal>
