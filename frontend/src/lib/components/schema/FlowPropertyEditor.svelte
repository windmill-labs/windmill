<script lang="ts">
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Label from '../Label.svelte'
	import Toggle from '../Toggle.svelte'
	import SimpleEditor from '../SimpleEditor.svelte'
	import type ItemPicker from '../ItemPicker.svelte'
	import type VariableEditor from '../VariableEditor.svelte'
	import { createEventDispatcher } from 'svelte'
	import ArgInput from '../ArgInput.svelte'
	import ObjectTypeNarrowing from '../ObjectTypeNarrowing.svelte'
	import Tabs from '../common/tabs/Tabs.svelte'
	import { Tab, TabContent } from '../common'
	import EditableSchemaDrawer from './EditableSchemaDrawer.svelte'
	import type { SchemaProperty } from '$lib/common'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import Button from '../common/button/Button.svelte'
	import { Pen, Plus, Trash2 } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'

	interface Props {
		format?: string | undefined
		contentEncoding?: 'base64' | 'binary' | undefined
		type?: string | undefined
		oneOf?: SchemaProperty[] | undefined
		required?: boolean
		pattern?: undefined | string
		password?: undefined | boolean
		variableEditor?: VariableEditor | undefined
		itemPicker?: ItemPicker | undefined
		nullable?: boolean | undefined
		disabled?: boolean | undefined
		defaultValue?: any
		propsNames?: any
		showExpr?: string | undefined
		extra?: Record<string, any>
		customErrorMessage?: string | undefined
		itemsType?:
			| {
					type?: 'string' | 'number' | 'bytes' | 'object'
					contentEncoding?: 'base64'
					enum?: string[]
					multiselect?: string[]
			  }
			| undefined
		properties?: Record<string, any> | undefined
		order?: string[] | undefined
		requiredProperty?: string[] | undefined
		displayWebhookWarning?: boolean
	}

	let {
		format = $bindable(undefined),
		contentEncoding = undefined,
		type = undefined,
		oneOf = $bindable(undefined),
		required = false,
		pattern = undefined,
		password = undefined,
		variableEditor = undefined,
		itemPicker = undefined,
		nullable = $bindable(undefined),
		disabled = $bindable(undefined),
		defaultValue = $bindable(undefined),
		propsNames = [],
		showExpr = $bindable(undefined),
		extra = {},
		customErrorMessage = undefined,
		itemsType = undefined,
		properties = $bindable(undefined),
		order = $bindable(undefined),
		requiredProperty = $bindable(undefined),
		displayWebhookWarning = true
	}: Props = $props()

	let oneOfSelected: string | undefined = $state(undefined)

	const dispatch = createEventDispatcher()

	function getResourceTypesFromFormat(format: string | undefined): string[] {
		if (format?.startsWith('resource-')) {
			return [format.split('-')[1]]
		}

		return []
	}

	let variantName = $state('')
	function createVariant(name: string) {
		if (oneOf) {
			if (oneOf.some((obj) => obj.title === name)) {
				throw new Error('Variant name already exists')
			}
			const idx = oneOf.findIndex((obj) => obj.title === name)
			if (idx === -1) {
				oneOf = [
					...oneOf,
					{
						title: name,
						type: 'object',
						properties: {}
					}
				]
				oneOfSelected = name
			}
			variantName = ''
		}
	}

	function renameVariant(name: string, selected: string) {
		if (oneOf) {
			if (oneOf.some((obj) => obj.title === name)) {
				throw new Error('Variant name already exists')
			}
			const idx = oneOf.findIndex((obj) => obj.title === selected)
			if (idx !== -1) {
				oneOf[idx].title = name
				oneOfSelected = name
			}
			variantName = ''
		}
	}

	let initialObjectSelected = $state(
		Object.keys(properties ?? {}).length == 0 ? 'resource' : 'custom-object'
	)
</script>

<div class="flex flex-col gap-2">
	{#if type === 'object' && oneOf && oneOf.length >= 2}
		<div class="flex flex-row gap-1 items-center justify-start">
			<ToggleButtonGroup
				bind:selected={oneOfSelected}
				class="h-auto w-auto"
				tabListClass="flex-wrap"
			>
				{#snippet children({ item })}
					{#each oneOf ?? [] as obj}
						<ToggleButton value={obj.title ?? ''} label={obj.title} {item} />
					{/each}
				{/snippet}
			</ToggleButtonGroup>

			<Popover placement="bottom-end" closeButton>
				{#snippet trigger()}
					<Button size="xs2" color="light" nonCaptureEvent startIcon={{ icon: Plus }} />
				{/snippet}
				{#snippet content({ close })}
					<Label label="Label" class="p-2 flex flex-col gap-2">
						<input
							type="text"
							class="w-full !bg-surface"
							onkeydown={(event) => {
								if (event.key === 'Enter') {
									createVariant(variantName)
									close()
								}
							}}
							bind:value={variantName}
						/>
						<Button
							variant="border"
							color="light"
							size="xs"
							on:click={() => {
								createVariant(variantName)
								close()
							}}
							disabled={variantName.length === 0}
						>
							Add
						</Button>
					</Label>
				{/snippet}
			</Popover>
		</div>
		<div class="flex flex-row gap-2 items-center">
			<span class="font-semibold text-sm">{oneOfSelected}</span>

			<Popover
				floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
				containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
				closeButton
			>
				{#snippet trigger()}
					<Button
						size="xs2"
						color="light"
						startIcon={{ icon: Pen }}
						propagateEvent
						iconOnly={false}
						on:click={() => {
							if (oneOfSelected) {
								variantName = oneOfSelected
							}
						}}
					/>
				{/snippet}
				{#snippet content({ close })}
					<Label label="Label" class="p-2 flex flex-col gap-2">
						<input
							type="text"
							class="w-full !bg-surface"
							onkeydown={(event) => {
								if (event.key === 'Enter') {
									if (oneOfSelected) {
										renameVariant(variantName, oneOfSelected)
										close()
									}
								}
							}}
							bind:value={variantName}
						/>
						<Button
							variant="border"
							color="light"
							size="xs"
							on:click={() => {
								if (oneOfSelected) {
									renameVariant(variantName, oneOfSelected)
									close()
								}
							}}
							disabled={variantName.length === 0}
						>
							Rename
						</Button>
					</Label>
				{/snippet}
			</Popover>
			<Button
				size="xs2"
				color="red"
				startIcon={{ icon: Trash2 }}
				iconOnly
				disabled={(oneOf?.length ?? 0) <= 2}
				on:click={() => {
					if (oneOf && oneOfSelected) {
						const idx = oneOf.findIndex((obj) => obj.title === oneOfSelected)
						oneOf = oneOf.filter((_, i) => i !== idx)
						oneOfSelected = oneOf[0].title
					}
				}}
			/>
		</div>
		{#if oneOfSelected && oneOf}
			{@const idx = oneOf.findIndex((obj) => obj.title === oneOfSelected)}
			<EditableSchemaDrawer
				bind:schema={
					() => {
						if (oneOf?.[idx]) {
							return {
								...oneOf[idx],
								properties: Object.fromEntries(
									Object.entries(oneOf[idx].properties ?? {}).filter(
										([k]) => k !== 'label' && k !== 'kind'
									)
								)
							}
						}
					},
					(v) => {
						if (oneOf?.[idx]) {
							const tagKey = oneOf?.find((o) => Object.keys(o.properties ?? {}).includes('kind'))
								? 'kind'
								: 'label'

							oneOf[idx] = {
								...(v ?? {}),
								type: 'object',
								properties: {
									...(v?.properties ?? {}),
									[tagKey]: {
										type: 'string',
										enum: [v?.title ?? '']
									}
								}
							}
						}
					}
				}
				on:change={() => {
					dispatch('schemaChange')
				}}
			/>
		{/if}
	{:else if type === 'object' && format !== 'resource-s3_object'}
		<Tabs
			bind:selected={initialObjectSelected}
			on:selected={(e) => {
				if (e.detail === 'custom-object') {
					format = ''
				}
			}}
		>
			<Tab value="resource">Resource</Tab>
			<Tab value="custom-object">Custom Object</Tab>
			{#snippet content()}
				<div class="pt-2">
					<TabContent value="custom-object">
						<EditableSchemaDrawer
							bind:schema={
								() => {
									return {
										properties: properties,
										order: order,
										required: requiredProperty
									}
								},
								(v) => {
									properties = v.properties
									order = v.order
									requiredProperty = v.required
									dispatch('schemaChange')
								}
							}
						/>
					</TabContent>

					<TabContent value="resource">
						<ObjectTypeNarrowing on:change={() => dispatch('schemaChange')} bind:format />
					</TabContent>
				</div>
			{/snippet}
		</Tabs>
	{/if}

	{#if !(type === 'object' && oneOf && oneOf.length >= 2) && !(type == 'object' && initialObjectSelected == 'custom-object')}
		<Label label="Default">
			<ArgInput
				noDefaultOnSelectFirst
				{itemPicker}
				resourceTypes={getResourceTypesFromFormat(format)}
				bind:value={defaultValue}
				type={password ? 'string' : type}
				displayHeader={false}
				{pattern}
				{customErrorMessage}
				{itemsType}
				{contentEncoding}
				{format}
				{extra}
				{disabled}
				{nullable}
				{variableEditor}
				compact
				noMargin
			/>
		</Label>
	{/if}

	<div class="flex flex-row gap-2 flex-wrap h-auto">
		<Toggle
			options={{ right: 'Required' }}
			size="xs"
			disabled={type === 'boolean'}
			on:change={(event) => {
				dispatch('requiredChange', { required: event?.detail })
			}}
			checked={required}
			on:change={(event) => {
				if (event?.detail) {
					nullable = false
				}
			}}
		/>
		{#if type === 'string'}
			<Toggle
				options={{
					right: 'Nullable',
					rightTooltip: 'If enabled, the default value will be null and not an empty string.'
				}}
				lightMode
				size="xs"
				checked={nullable}
				on:change={(event) => {
					if (event?.detail) {
						nullable = true
					} else {
						nullable = undefined
					}
				}}
				disabled={required}
			/>
		{/if}
		<Toggle
			options={{
				right: 'Disabled',
				rightTooltip: 'Do not let user modify this field'
			}}
			lightMode
			size="xs"
			checked={disabled}
			on:change={(event) => {
				if (event?.detail) {
					disabled = true
				} else {
					disabled = undefined
				}
			}}
		/>
	</div>

	{#if displayWebhookWarning && !(type === 'object' && oneOf && oneOf.length >= 2)}
		<Alert type="info" title="Default not used by webhooks" size="xs" collapsible>
			If this flow is triggered by a webhook, the default value will not replace a missing value
			from the payload. It will only be used as the default value in the autogenerated UI. We
			recommend using default values in the signature of the steps where this value is used (using
			`x=default`) to have the desired behavior.
		</Alert>
	{/if}
</div>
<div>
	<Toggle
		size="xs"
		options={{ right: 'Show this field only when conditions are met' }}
		checked={Boolean(showExpr)}
		on:change={() => {
			showExpr = showExpr ? undefined : 'true //fields.foo == 42'
		}}
	/>
	{#if showExpr != undefined}
		<div class="border">
			<SimpleEditor
				extraLib={`declare const fields: Record<${propsNames
					?.filter((x) => x != name)
					.map((x) => `"${x}"`)
					.join(' | ')}, any>;\n`}
				lang="javascript"
				bind:code={showExpr}
				shouldBindKey={false}
				fixedOverflowWidgets={false}
				autoHeight
			/>
		</div>
		<div class="flex flex-row-reverse text-2xs text-tertiary">
			<div>
				Other fields are available under <code>fields</code> (e.g:
				<code>fields.foo == 42</code>)
			</div>
		</div>
	{/if}
</div>
