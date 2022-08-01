<script lang="ts" context="module">
	import type { SchemaProperty } from '$lib/common'
	import Modal from './Modal.svelte'

	export const ARG_TYPES = ['integer', 'number', 'string', 'boolean', 'object', 'array'] as const
	export type ArgType = typeof ARG_TYPES[number]

	export interface ModalSchemaProperty {
		selectedType?: string
		description: string
		name: string
		required: boolean
		format?: string
		pattern?: string
		enum_?: string[]
		default?: any
		items?: { type?: 'string' | 'number' }
		contentEncoding?: 'base64' | 'binary'
	}

	export function schemaToModal(
		schema: SchemaProperty,
		name: string,
		required: boolean
	): ModalSchemaProperty {
		return {
			name,
			selectedType: schema.type,
			description: schema.description,
			pattern: schema.pattern,
			default: schema.default,
			contentEncoding: schema.contentEncoding,
			format: schema.format,
			required
		}
	}

	export const DEFAULT_PROPERTY: ModalSchemaProperty = {
		selectedType: 'string',
		description: '',
		name: '',
		required: false
	}
</script>

<script lang="ts">
	import Switch from './Switch.svelte'
	import { createEventDispatcher } from 'svelte'
	import ArgInput from './ArgInput.svelte'
	import StringTypeNarrowing from './StringTypeNarrowing.svelte'
	import Required from './Required.svelte'
	import ObjectTypeNarrowing from './ObjectTypeNarrowing.svelte'

	export let property: ModalSchemaProperty = DEFAULT_PROPERTY
	export let error = ''
	export let editing = false
	export let oldArgName: string | undefined = undefined

	let resource_type: string | undefined = undefined

	const dispatch = createEventDispatcher()
	let modal: Modal

	export function openModal(): void {
		modal.openModal()
		resource_type = property.format?.substring(5)
	}

	export function closeModal(): void {
		modal.closeModal()
	}

	function clearModal(): void {
		error = ''
		editing = false
		oldArgName = undefined
		property.name = DEFAULT_PROPERTY.name
		property.default = DEFAULT_PROPERTY.default
		property.description = DEFAULT_PROPERTY.description
		property.required = DEFAULT_PROPERTY.required
		property.selectedType = DEFAULT_PROPERTY.selectedType
		property.format = undefined
		resource_type = undefined
	}

	$: if (property.selectedType == 'object' && resource_type) {
		property.format = resource_type ? `$res:${resource_type}` : undefined
	}
</script>

<Modal bind:this={modal} on:close={clearModal}>
	<div slot="title">Add an argument</div>
	<div slot="content">
		<div class="flex flex-col px-6 py-3 bg-gray-50">
			<div class="text-purple-500 text-2xs grow">{error}</div>
			<label class="mb-2 font-semibold text-gray-700"
				>Name<Required required={true} />
				<input type="text" placeholder="Argument name" class="" bind:value={property.name} />
			</label>
			<label class="mb-2 font-semibold text-gray-700">
				Description
				<textarea
					class="mb-1"
					type="text"
					placeholder="Type message..."
					rows="3"
					bind:value={property.description}
				/>
			</label>
			<h3 class="font-semibold text-gray-700">Type<Required required={true} /></h3>
			<div class="grid sm:grid-cols-3 md:grid-cols-4 gap-x-2 gap-y-1 items-center mb-2 w-full">
				{#each ARG_TYPES as argType}
					<button
						class={argType == property.selectedType ? 'item-button-selected' : 'item-button'}
						on:click={() => {
							property.selectedType = argType
							property.format = undefined
							property.contentEncoding = undefined
							property.enum_ = undefined
							property.pattern = undefined
						}}>{argType}</button
					>
				{/each}
				<button
					class={!property.selectedType ? 'item-button-selected' : 'item-button'}
					on:click={() => {
						property.selectedType = undefined
					}}>any</button
				>
			</div>
			<Switch
				label={'Required'}
				textFormat={'text-md font-semibold text-gray-700'}
				class="my-2"
				bind:checked={property.required}
			/>
			<ArgInput
				label="Default"
				bind:value={property.default}
				type={property.selectedType}
				pattern={property.pattern}
			/>
			{#if property.selectedType !== 'boolean'}
				<h2 class="mb-2 mt-4">Advanced</h2>

				{#if property.selectedType == 'string'}
					<StringTypeNarrowing
						bind:format={property.format}
						bind:pattern={property.pattern}
						bind:enum_={property.enum_}
						bind:contentEncoding={property.contentEncoding}
					/>
				{:else if property.selectedType == 'array'}
					<select bind:value={property.items}>
						<option value={undefined}>No specific item type</option>
						<option value={{ type: 'string' }}> Items are strings</option>
						<option value={{ type: 'number' }}>Items are numbers</option>
					</select>
				{:else if property.selectedType == 'object'}
					<h3 class="mb-2 font-bold mt-4">Resource type</h3>
					<ObjectTypeNarrowing bind:format={property.format} />
				{:else}
					<p>No advanced configuration for this type</p>
				{/if}
			{/if}
		</div>
	</div>

	<button
		slot="submission"
		class="px-4 py-2 text-white font-semibold bg-blue-500 rounded"
		on:click={() => {
			dispatch('save')
		}}
	>
		Save
	</button>
</Modal>
