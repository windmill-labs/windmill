<script lang="ts" context="module">
	import { type SchemaProperty, type ModalSchemaProperty, modalToSchema } from '$lib/common'
	import Tab from './common/tabs/Tab.svelte'
	import TabContent from './common/tabs/TabContent.svelte'
	import Tabs from './common/tabs/Tabs.svelte'
	import SchemaEditor from './SchemaEditor.svelte'

	export const ARG_TYPES = ['integer', 'number', 'string', 'boolean', 'object', 'array'] as const
	export type ArgType = (typeof ARG_TYPES)[number]

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
			min: schema.min,
			max: schema.max,
			currency: schema.currency,
			currencyLocale: schema.currencyLocale,
			multiselect: schema.multiselect,
			items: schema.items?.type
				? { type: schema.items.type as 'string' | 'number' | undefined, enum: schema.items.enum }
				: undefined,
			required,
			schema:
				schema.type == 'object'
					? {
							$schema: undefined,
							type: schema.type,
							properties: schema.properties ?? {},
							required: schema.required ?? []
					  }
					: undefined,
			showExpr: schema.showExpr
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
	import LightweightSchemaForm from './LightweightSchemaForm.svelte'
	import NumberTypeNarrowing from './NumberTypeNarrowing.svelte'
	import SimpleEditor from './SimpleEditor.svelte'

	export let error = ''
	export let editing = false
	export let oldArgName: string | undefined = undefined
	export let isFlowInput = false
	export let propsNames: string[] = []

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
		property.min = undefined
		property.max = undefined
		property.currency = undefined
		property.currencyLocale = undefined
		property.multiselect = undefined
		property.items = undefined
		property.showExpr = undefined
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
						id="schema-modal-name"
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
							color={isSelected ? 'blue' : 'light'}
							btnClasses={isSelected ? '!border-2' : 'm-[1px]'}
							on:click={() => {
								property.selectedType = argType
								property.format = undefined
								property.contentEncoding = undefined
								property.enum_ = undefined
								property.pattern = undefined
								property.default = undefined
								property.min = undefined
								property.max = undefined
								property.currency = undefined
								property.currencyLocale = undefined
								property.multiselect = undefined
								if (argType == 'array') {
									property.items = { type: 'string' }
								} else {
									property.items = undefined
								}
								property.showExpr = undefined
							}}
							id={`schema-modal-type-${argType}`}
						>
							{argType}
						</Button>
					{/each}
					<Button
						size="sm"
						variant="border"
						color={!property.selectedType ? 'blue' : 'light'}
						btnClasses={!property.selectedType ? '!border-2' : 'm-[1px]'}
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
						customErrorMessage={property.customErrorMessage}
						itemsType={property.items}
						contentEncoding={property.contentEncoding}
						format={property.format}
						extra={property}
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
			<div>
				<div class="font-semibold text-secondary mb-1">Advanced</div>
				{#if property.selectedType == 'string'}
					<StringTypeNarrowing
						bind:customErrorMessage={property.customErrorMessage}
						bind:format={property.format}
						bind:pattern={property.pattern}
						bind:enum_={property.enum_}
						bind:contentEncoding={property.contentEncoding}
						noExtra
					/>
				{:else if property.selectedType == 'array'}
					<ArrayTypeNarrowing bind:itemsType={property.items} />
				{:else if property.selectedType == 'number' || property.selectedType == 'integer'}
					<NumberTypeNarrowing
						bind:min={property.min}
						bind:max={property.max}
						bind:currency={property.currency}
						bind:currencyLocale={property.currencyLocale}
					/>
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
				{/if}
				<div class="pt-2">
					<Toggle
						options={{ right: 'Show this field only when conditions are met' }}
						class="!justify-start"
						checked={Boolean(property.showExpr)}
						on:change={() => {
							property.showExpr = property.showExpr ? undefined : 'true //fields.foo == 42'
						}}
					/>
					{#if property.showExpr != undefined}
						<div class="border">
							<SimpleEditor
								extraLib={`declare const fields: Record<${propsNames
									.filter((x) => x != property.name)
									.map((x) => `"${x}"`)
									.join(' | ')}, any>;\n`}
								lang="javascript"
								bind:code={property.showExpr}
								shouldBindKey={false}
								fixedOverflowWidgets={false}
								autoHeight
							/>
						</div>
						<div class="flex flex-row-reverse text-2xs text-tertiary"
							><div
								>Other fields are available under <code>fields</code> (e.g:
								<code>fields.foo == 42</code>)</div
							></div
						>
					{/if}
				</div>
			</div>
		</div>
		<div class="font-semibold text-secondary mb-1 pt-4">Preview</div>
		<LightweightSchemaForm
			displayType={false}
			schema={{
				properties: {
					[property.name]: modalToSchema(property)
				},
				required: property.required ? [property.name] : []
			}}
		/>
		<svelte:fragment slot="actions">
			<div class="h-10" />
			<Button
				color="dark"
				disabled={!property.name || error != ''}
				on:click={() => {
					dispatch('save', property)
				}}
				id="schema-modal-save"
			>
				Save
			</Button>
		</svelte:fragment>
	</DrawerContent>
</Drawer>
