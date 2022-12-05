<script lang="ts" context="module">
	import type { SchemaProperty } from '$lib/common'

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
	import { createEventDispatcher } from 'svelte'
	import ArgInput from './ArgInput.svelte'
	import StringTypeNarrowing from './StringTypeNarrowing.svelte'
	import Required from './Required.svelte'
	import ObjectTypeNarrowing from './ObjectTypeNarrowing.svelte'
	import { Button } from './common'
	import Toggle from '$lib/components/Toggle.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Drawer from './common/drawer/Drawer.svelte'

	export let property: ModalSchemaProperty = DEFAULT_PROPERTY
	export let error = ''
	export let editing = false
	export let oldArgName: string | undefined = undefined

	let resource_type: string | undefined = undefined

	const dispatch = createEventDispatcher()
	let drawer: Drawer

	export function openDrawer(): void {
		drawer.openDrawer()
		resource_type = property.format?.substring(5)
	}

	export function closeDrawer(): void {
		drawer.closeDrawer()
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
		drawer.closeDrawer()
	}

	$: if (property.selectedType == 'object' && resource_type) {
		property.format = resource_type ? `$res:${resource_type}` : undefined
	}

	$: if (property.name == '') {
		error = 'Name is required'
	} else {
		error = ''
	}
</script>

<Drawer bind:this={drawer} placement="right">
	<DrawerContent on:close={clearModal} title="Add an argument">
		<div class="flex flex-col gap-6">
			<div>
				<label class="block">
					<div class="mb-1 font-semibold text-gray-700">
						Name
						<Required required={true} />
					</div>
					<input
						autofocus
						autocomplete="off"
						type="text"
						placeholder="Enter a name"
						bind:value={property.name}
						class={error === ''
							? ''
							: 'border border-red-700 bg-red-100 border-opacity-30 focus:border-red-700 focus:border-opacity-30 focus-visible:ring-red-700 focus-visible:ring-opacity-25 focus-visible:border-red-700'}
					/>
				</label>
				<div class="text-red-600 text-2xs">{error}</div>
			</div>

			<label class="block">
				<div class="mb-1 font-semibold text-gray-700">
					Description
					<Required required={false} />
				</div>
				<textarea placeholder="Enter a description" rows="3" bind:value={property.description} />
			</label>
			<div>
				<div class="mb-1 font-semibold text-gray-700">Type<Required required={true} /></div>
				<div class="grid sm:grid-cols-3 md:grid-cols-4 gap-x-2 gap-y-1 items-center mb-2 w-full">
					{#each ARG_TYPES as argType}
						{@const isSelected = argType == property.selectedType}
						<Button
							size="sm"
							variant="border"
							color={isSelected ? 'blue' : 'dark'}
							btnClasses={isSelected ? '!border-2 !bg-blue-50/75' : 'm-[1px]'}
							on:click={() => {
								property.selectedType = argType
								property.format = undefined
								property.contentEncoding = undefined
								property.enum_ = undefined
								property.pattern = undefined
							}}
						>
							{argType}
						</Button>
					{/each}
					<Button
						size="sm"
						variant="border"
						color={!property.selectedType ? 'blue' : 'dark'}
						btnClasses={!property.selectedType ? '!border-2 !bg-blue-50/75' : 'm-[1px]'}
						on:click={() => {
							property.selectedType = undefined
						}}
					>
						any
					</Button>
				</div>
			</div>
			<div class="flex flex-row gap-x-4">
				<ArgInput
					label="Default"
					bind:value={property.default}
					type={property.selectedType}
					pattern={property.pattern}
				/>
				<Toggle
					options={{ right: 'Required' }}
					class="mt-5 !justify-start"
					bind:checked={property.required}
				/>
			</div>
			{#if property.selectedType !== 'boolean'}
				<div>
					<div class="font-semibold text-gray-700 mb-1">Advanced</div>
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
				</div>
			{/if}
		</div>

		<svelte:fragment slot="actions">
			<Button
				color="blue"
				disabled={!property.name || !property.selectedType || error != ''}
				on:click={() => {
					dispatch('save')
				}}
			>
				Save
			</Button>
		</svelte:fragment>
	</DrawerContent>
</Drawer>
