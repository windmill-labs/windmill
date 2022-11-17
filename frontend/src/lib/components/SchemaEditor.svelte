<script lang="ts">
	import type { Schema, SchemaProperty } from '$lib/common'
	import { emptySchema, sendUserToast } from '$lib/utils'
	import { faPen, faPlus, faTrash } from '@fortawesome/free-solid-svg-icons'
	import { Button } from './common'
	import { createEventDispatcher } from 'svelte'
	import SchemaEditorProperty from './SchemaEditorProperty.svelte'
	import type { ModalSchemaProperty } from './SchemaModal.svelte'
	import SchemaModal, { DEFAULT_PROPERTY, schemaToModal } from './SchemaModal.svelte'
	import SimpleEditor from './SimpleEditor.svelte'
	import TableCustom from './TableCustom.svelte'
	import Toggle from './Toggle.svelte'
	import Tooltip from './Tooltip.svelte'
	import { flip } from 'svelte/animate'

	const dispatch = createEventDispatcher()

	export let schema: Schema = emptySchema()

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

	function changePosition(i: number, up: boolean): any {
		const entries = Object.entries(schema.properties)
		var element = entries[i]
		entries.splice(i, 1)
		entries.splice(up ? i - 1 : i + 1, 0, element)
		schema.properties = Object.fromEntries(entries)
	}
</script>

<div class="flex flex-col">
	<div class="flex justify-between gap-x-2">
		<div>
			<Button
				variant="contained"
				color="blue"
				size="md"
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
					right: 'Json Schema Editor'
				}}
			/>
			<div class="ml-2">
				<Tooltip>
					Arguments can be edited either using the wizard, or by editing their json-schema
					<a href="https://docs.windmill.dev/docs/reference/script_arguments_reference">docs</a>
				</Tooltip>
			</div>
		</div>
	</div>

	<!--json schema or table view-->
	<div class="h-full overflow-y-auto">
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
							{#each Object.entries(schema.properties) as [name, property], i (name)}
								<tr animate:flip={{ duration: 50 }}>
									<td class="font-bold">{name}</td>
									<td>
										<SchemaEditorProperty {property} />
									</td>
									<td>{property.description ?? ''}</td>
									<td>{property.default ? JSON.stringify(property.default) : ''}</td>
									<td
										>{#if schema.required.includes(name)}
											<span class="text-red-600 font-bold text-lg">*</span>
										{/if}</td
									>
									<td class="justify-end flex">
										{#if i > 0}
											<button on:click={() => changePosition(i, true)} class="text-lg mr-2"
												>&uparrow;</button
											>
										{/if}
										{#if i < Object.keys(schema.properties).length - 1}
											<button on:click={() => changePosition(i, false)} class="text-lg mr-2"
												>&downarrow;</button
											>
										{/if}

										<Button
											color="red"
											variant="border"
											btnClasses="mx-2"
											size="sm"
											startIcon={{ icon: faTrash }}
											on:click={() => handleDeleteArgument(name)}
										>
											Delete
										</Button>
										<Button
											color="light"
											variant="border"
											size="sm"
											startIcon={{ icon: faPen }}
											on:click={() => startEditArgument(name)}
										>
											Edit
										</Button>
									</td>
								</tr>
							{/each}
						</tbody>
					</TableCustom>
				{:else}
					<div class="text-gray-700 text-xs italic mt-2">This schema has no arguments.</div>
				{/if}
			</div>
		{:else}
			<div class="border rounded mt-4 p-2">
				<SimpleEditor
					on:change={() => {
						try {
							schema = JSON.parse(schemaString)
						} catch (err) {
							sendUserToast(err.message, true)
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

<SchemaModal
	bind:this={schemaModal}
	bind:property={modalProperty}
	bind:error={argError}
	on:save={handleAddOrEditArgument}
	bind:editing
	bind:oldArgName
/>
