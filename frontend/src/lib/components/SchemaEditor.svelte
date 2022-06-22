<script lang="ts">
	import SchemaModal, { DEFAULT_PROPERTY, modalToSchema, schemaToModal } from './SchemaModal.svelte'
	import type { ModalSchemaProperty } from './SchemaModal.svelte'
	import type { Schema } from '$lib/common'
	import Editor from './Editor.svelte'
	import { emptySchema, sendUserToast } from '$lib/utils'
	import Tooltip from './Tooltip.svelte'
	import TableCustom from './TableCustom.svelte'

	export let schema: Schema = emptySchema()

	let schemaModal: SchemaModal
	let schemaString: string = ''

	// Internal state: bound to args builder modal
	let modalProperty: ModalSchemaProperty = Object.assign({}, DEFAULT_PROPERTY)
	let argError = ''
	let editing = false
	let oldArgName: string | undefined // when editing argument and changing name

	let viewJsonSchema = false
	let editor: Editor

	$: schemaString = JSON.stringify(schema, null, '\t')

	export function getEditor(): Editor {
		return editor
	}
	// Binding is not enough because monaco Editor does not support two-way binding
	export function getSchema(): Schema {
		if (viewJsonSchema) {
			try {
				schema = JSON.parse(editor.getCode())
				return schema
			} catch (err) {
				throw Error(`Error: input is not a valid schema: ${err}`)
			}
		} else {
			try {
				editor.setCode(JSON.stringify(schema, null, '\t'))
				return schema
			} catch (err) {
				throw Error(`Error: input is not a valid schema: ${err}`)
			}
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
				schema.required = [...schema.required, modalProperty.name]
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
			schemaModal.closeModal()
		}
	}

	function startEditArgument(argName: string): void {
		argError = ''
		if (Object.keys(schema.properties).includes(argName)) {
			schemaModal.openModal()
			editing = true
			modalProperty = schemaToModal(
				schema.properties[argName],
				argName,
				schema.required.includes(argName)
			)
			oldArgName = argName
		} else {
			sendUserToast(`This argument does not exist and can't be edited`, true)
		}
	}

	function handleDeleteArgument(argName: string): void {
		try {
			if (Object.keys(schema.properties).includes(argName)) {
				delete schema.properties[argName]
				schema = schema //needed for reactivity, see https://svelte.dev/tutorial/updating-arrays-and-objects
			} else {
				throw Error('Argument not found!')
			}
		} catch (err) {
			console.error(err)
			sendUserToast(`Could not delete argument: ${err}`, true)
		}
	}

	function switchTab(): void {
		if (viewJsonSchema) {
			let schemaString = editor.getCode()
			if (schemaString === '') {
				schemaString = JSON.stringify(emptySchema(), null, 4)
			}
			try {
				schema = JSON.parse(schemaString)
				viewJsonSchema = false
			} catch (err) {
				sendUserToast(err, true)
			}
		} else {
			try {
				editor.setCode(JSON.stringify(schema, null, '\t'))
				viewJsonSchema = true
			} catch (err) {
				sendUserToast(err, true)
			}
		}
	}
</script>

<div class="flex flex-col">
	<div class="w-full">
		<div class="flex flex-row text-base">
			<button
				class="text-xs sm:text-base py-1 px-6 block hover:text-blue-500 focus:outline-noneborder-gray-200  {viewJsonSchema
					? 'text-gray-500 '
					: 'text-gray-700 font-semibold  '}"
				on:click={() => (viewJsonSchema ? switchTab() : null)}
			>
				arguments
			</button><button
				class="py-1 px-6 block hover:text-blue-500 focus:outline-none border-gray-200  {viewJsonSchema
					? 'text-gray-700 font-semibold '
					: 'text-gray-500'}"
				on:click={() => (viewJsonSchema ? null : switchTab())}
			>
				advanced <Tooltip
					>Arguments can be edited either using the wizard, or by editing their json-schema <a
						href="https://docs.windmill.dev/docs/reference/script_arguments_reference">docs</a
					></Tooltip
				>
			</button>
			<button
				class="default-button-secondary grow"
				on:click={() => {
					modalProperty = Object.assign({}, DEFAULT_PROPERTY)
					schemaModal.openModal()
				}}>Add argument</button
			>
		</div>
	</div>
	<!--json schema or table view-->
	<div class="border-t py-1  h-full overflow-y-auto">
		<div class="h-full {viewJsonSchema ? 'hidden' : ''}">
			{#if schema.properties && Object.keys(schema.properties).length > 0 && schema.required}
				<TableCustom class="w-full min-h-full">
					<tr slot="header-row" class="underline">
						<th>name</th>
						<th>type</th>
						<th>description</th>
						<th>default</th>
						<th>required</th>
					</tr>
					<tbody slot="body">
						{#each Object.entries(schema.properties) as [name, property] (name)}
							<tr>
								<td>{name}</td>
								<td
									>{#if !property.type} any {:else} {property.type} {/if}</td
								>
								<td>{property.description}</td>
								<td>{JSON.stringify(property.default) ?? ''}</td>
								<td>{schema.required.includes(name) ? 'required' : 'optional'}</td>
								<td class="">
									<button class="mr-2" on:click={() => handleDeleteArgument(name)}
										><svg
											class="w-4 h-4"
											fill="none"
											stroke="currentColor"
											viewBox="0 0 24 14"
											xmlns="http://www.w3.org/2000/svg"
										>
											<path
												stroke-linecap="round"
												stroke-linejoin="round"
												stroke-width="2"
												d="M6 18L18 6M6 6l12 12"
											/>
										</svg></button
									>
									<button
										class="default-button-secondary text-xs inline-flex"
										on:click={() => {
											startEditArgument(name)
										}}>edit</button
									></td
								>
							</tr>
						{/each}
					</tbody>
				</TableCustom>
			{:else}
				<div class="text-gray-700 text-xs italic">This script has no argument</div>
			{/if}
		</div>
		<div class={viewJsonSchema ? '' : 'hidden'}>
			<Editor code={schemaString} bind:this={editor} lang={'json'} class="small-editor" />
		</div>
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
