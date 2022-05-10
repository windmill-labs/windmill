<script lang="ts" context="module">
	import type { SchemaProperty } from '../../common'
	import Modal from '../../routes/components/Modal.svelte'

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
	}

	export function modalToSchema(schema: ModalSchemaProperty): SchemaProperty {
		return {
			type: schema.selectedType,
			description: schema.description,
			pattern: schema.pattern,
			default: schema.default,
			enum: schema.enum_,
			items: schema.items
		}
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

	export let property: ModalSchemaProperty = DEFAULT_PROPERTY
	export let error = ''
	export let editing = false
	export let oldArgName: string | undefined

	const dispatch = createEventDispatcher()
	let modal: Modal

	export function openModal(): void {
		modal.openModal()
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
			<div class="grid sm:grid-cols-3 md:grid-cols-4 gap-x-2 gap-y-1 items-center mb-2">
				{#each ARG_TYPES as argType}
					<button
						class={argType == property.selectedType ? 'item-button-selected' : 'item-button'}
						on:click={() => {
							property.selectedType = argType
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
				<h2 class="mb-2">Advanced</h2>

				{#if property.selectedType == 'string'}
					<StringTypeNarrowing
						bind:format={property.format}
						bind:pattern={property.pattern}
						bind:enum_={property.enum_}
					/>
				{:else if property.selectedType == 'array'}
					<select bind:value={property.items}>
						<option value={undefined}>No specific item type</option>
						<option value={{ type: 'string' }}> Items are strings</option>
						<option value={{ type: 'number' }}>Items are numbers</option>
					</select>
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

<style>
	.item-button {
		@apply py-1;
		@apply border;
		@apply rounded-sm;
	}
	.item-button-selected {
		@apply underline;
		@apply font-bold;
		@apply py-1;
		@apply border border-blue-500;
		@apply bg-blue-50;
		@apply rounded-sm;
	}
</style>
