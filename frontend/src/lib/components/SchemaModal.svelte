<script lang="ts" context="module">
	import type { SchemaProperty, Schema } from '$lib/common'
	import Tab from './common/tabs/Tab.svelte'
	import TabContent from './common/tabs/TabContent.svelte'
	import Tabs from './common/tabs/Tabs.svelte'
	import SchemaEditor from './SchemaEditor.svelte'

	export const ARG_TYPES = ['integer', 'number', 'string', 'boolean', 'object', 'array'] as const
	export type ArgType = (typeof ARG_TYPES)[number]

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
		schema?: Schema
	}

	export function schemaToModal(
		schema: SchemaProperty,
		name: string,
		required: boolean
	): ModalSchemaProperty {
		return {
			name,
			selectedType: schema.type,
			description: schema.description ?? '',
			pattern: schema.pattern,
			default: schema.default,
			contentEncoding: schema.contentEncoding,
			format: schema.format,
			enum_: schema.enum,
			required,
			schema:
				schema.type == 'object'
					? {
							$schema: undefined,
							type: schema.type,
							properties: schema.properties ?? {},
							required: schema.required ?? []
					  }
					: undefined
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
	import { Alert, Button } from './common'
	import Toggle from '$lib/components/Toggle.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Drawer from './common/drawer/Drawer.svelte'
	import ArrayTypeNarrowing from './ArrayTypeNarrowing.svelte'

	export let error = ''
	export let editing = false
	export let oldArgName: string | undefined = undefined
	export let isFlowInput = false

	const dispatch = createEventDispatcher()
	let drawer: Drawer

	let property: ModalSchemaProperty = DEFAULT_PROPERTY

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key
		if (key === 'Enter') {
			dispatch('save', property)
		}
	}

	export function openDrawer(nproperty: ModalSchemaProperty): void {
		drawer.openDrawer()
		property = nproperty
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
		property.schema = undefined

		drawer.closeDrawer()
	}

	$: if (property.name == '') {
		error = 'Name is required'
	} else {
		error = ''
	}

	let title = ''
	$: title = editing ? `Edit ${oldArgName} argument` : 'Add an argument'
</script>

<Drawer bind:this={drawer} placement="right">
	<DrawerContent on:close={clearModal} {title}>
		<div class="flex flex-col gap-6">
			<div>
				<label class="block">
					<div class="mb-1 font-semibold text-secondary">
						Name
						<Required required={true} />
					</div>
					<!-- svelte-ignore a11y-autofocus -->
					<input
						autofocus
						autocomplete="off"
						type="text"
						placeholder="Enter a name"
						bind:value={property.name}
						on:keyup={handleKeyUp}
						class={error === ''
							? ''
							: 'border border-red-700 bg-red-100 border-opacity-30 focus:border-red-700 focus:border-opacity-30 focus-visible:ring-red-700 focus-visible:ring-opacity-25 focus-visible:border-red-700'}
					/>
				</label>
				<div class="text-red-600 text-2xs">{error}</div>
			</div>

			<label class="block">
				<div class="mb-1 font-semibold text-secondary">
					Description
					<Required required={false} />
				</div>
				<textarea placeholder="Enter a description" rows="3" bind:value={property.description} />
			</label>
			<div>
				<div class="mb-1 font-semibold text-secondary">Type<Required required={true} /></div>
				<div class="grid sm:grid-cols-3 md:grid-cols-4 gap-x-2 gap-y-1 items-center mb-2 w-full">
					{#each ARG_TYPES as argType}
						{@const isSelected = argType == property.selectedType}
						<Button
							size="sm"
							variant="border"
							color={isSelected ? 'blue' : 'dark'}
							btnClasses={isSelected ? '!bg-blue-50/75 dark:!bg-frost-900/50 ' : ''}
							on:click={() => {
								property.selectedType = argType
								property.format = undefined
								property.contentEncoding = undefined
								property.enum_ = undefined
								property.pattern = undefined
								if (argType == 'array') {
									property.items = { type: 'string' }
								} else {
									property.items = undefined
								}
							}}
						>
							{argType}
						</Button>
					{/each}
					<Button
						size="sm"
						variant="border"
						color={!property.selectedType ? 'blue' : 'dark'}
						btnClasses={!property.selectedType ? '!bg-blue-50/75 dark:!bg-frost-900/50' : ']'}
						on:click={() => {
							property.selectedType = undefined
						}}
					>
						any
					</Button>
				</div>
			</div>
			<div>
				<div class="flex flex-row gap-x-4 items-center">
					<ArgInput
						resourceTypes={[]}
						label="Default"
						bind:value={property.default}
						type={property.selectedType}
						pattern={property.pattern}
						itemsType={property.items}
					/>
					<Toggle
						options={{ right: 'Required' }}
						class="!justify-start"
						bind:checked={property.required}
					/>
				</div>
				{#if isFlowInput}
					<Alert type="info" title="Default not used by webhooks">
						If this flow is triggered by a webhook, the default value will not replace a missing
						value from the payload. It will only be used as the default value in the autogenerated
						UI. We recommend using default values in the signature of the steps where this value is
						used (using `x=default`) to have the desired behavior.</Alert
					>
				{/if}
			</div>
			{#if property.selectedType !== 'boolean'}
				<div>
					<div class="font-semibold text-secondary mb-1">Advanced</div>
					{#if property.selectedType == 'string'}
						<StringTypeNarrowing
							bind:format={property.format}
							bind:pattern={property.pattern}
							bind:enum_={property.enum_}
							bind:contentEncoding={property.contentEncoding}
						/>
					{:else if property.selectedType == 'array'}
						<ArrayTypeNarrowing bind:itemsType={property.items} />
					{:else if property.selectedType == 'object'}
						<Tabs selected="custom-object">
							<Tab value="custom-object">Custom Object</Tab>
							<Tab value="resource">Resource</Tab>
							<svelte:fragment slot="content">
								<div class="overflow-auto pt-2">
									<TabContent value="custom-object">
										<SchemaEditor bind:schema={property.schema} />
									</TabContent>

									<TabContent value="resource">
										<h3 class="mb-2 font-bold mt-4">Resource type</h3>
										<ObjectTypeNarrowing bind:format={property.format} />
									</TabContent>
								</div>
							</svelte:fragment>
						</Tabs>
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
					dispatch('save', property)
				}}
			>
				Save
			</Button>
		</svelte:fragment>
	</DrawerContent>
</Drawer>
