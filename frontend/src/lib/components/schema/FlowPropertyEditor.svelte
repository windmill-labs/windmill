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
	import { deepEqual } from 'fast-equals'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'

	export let format: string | undefined = undefined
	export let contentEncoding: 'base64' | 'binary' | undefined = undefined
	export let type: string | undefined = undefined
	export let oneOf: SchemaProperty[] | undefined = undefined
	export let required = false
	export let pattern: undefined | string = undefined
	export let password: undefined | boolean = undefined
	export let variableEditor: VariableEditor | undefined = undefined
	export let itemPicker: ItemPicker | undefined = undefined
	export let nullable: boolean | undefined = undefined
	export let disabled: boolean | undefined = undefined
	export let defaultValue: any = undefined
	export let propsNames: any = []
	export let showExpr: string | undefined = undefined
	export let extra: Record<string, any> = {}
	export let customErrorMessage: string | undefined = undefined
	export let itemsType:
		| {
				type?: 'string' | 'number' | 'bytes' | 'object'
				contentEncoding?: 'base64'
				enum?: string[]
				multiselect?: string[]
		  }
		| undefined = undefined
	export let properties: Record<string, any> | undefined = undefined
	export let order: string[] | undefined = undefined
	export let requiredProperty: string[] | undefined = undefined
	export let displayWebhookWarning: boolean = true

	function getOneOfWithoutLabel(oneOf: SchemaProperty[]) {
		return oneOf.map((v) => ({
			...v,
			properties: Object.fromEntries(
				Object.entries(v.properties ?? {}).filter(([k, v]) => k !== 'label')
			)
		}))
	}

	let oneOfSelected: string | undefined = undefined
	function oneOfUpdate(oneOf: SchemaProperty[] | undefined) {
		if (oneOf && oneOf.length >= 2) {
			if (!oneOfSelected) {
				oneOfSelected = oneOf[0].title
			}

			if (
				!schema.oneOf ||
				!deepEqual(
					oneOf.map((v) => [v.title, v.order]),
					schema.oneOf.map((v) => [v.title, v.order])
				)
			) {
				// update schema if not exists or order changed
				schema.oneOf = getOneOfWithoutLabel(oneOf)
				schema = schema
			}
		} else if (!oneOf) {
			schema.oneOf = undefined
			schema = schema
		}
	}
	$: oneOfUpdate(oneOf)

	function orderUpdate(order) {
		if (order && !deepEqual(order, schema.order)) {
			// update from external reordering
			schema.order = order
			schema = schema
		}
	}
	$: orderUpdate(order)

	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)

	function getResourceTypesFromFormat(format: string | undefined): string[] {
		if (format?.startsWith('resource-')) {
			return [format.split('-')[1]]
		}

		return []
	}

	let schema = {
		properties,
		order,
		required: requiredProperty,
		oneOf: oneOf ? getOneOfWithoutLabel(oneOf) : undefined
	}

	function schemaUpdate(changedSchema: typeof schema) {
		if (
			!deepEqual(changedSchema, {
				properties,
				order,
				required: requiredProperty,
				oneOf: oneOf ? getOneOfWithoutLabel(oneOf) : undefined
			})
		) {
			properties = structuredClone(changedSchema.properties)
			order = structuredClone(changedSchema.order)
			requiredProperty = structuredClone(changedSchema.required)
			oneOf = changedSchema.oneOf?.map((v) => {
				return {
					...v,
					properties: {
						...(v.properties ?? {}),
						label: {
							type: 'string',
							enum: [v.title ?? '']
						}
					}
				}
			})
			dispatchIfMounted('schemaChange', { properties, order, requiredProperty, oneOf })
		}
	}

	$: schemaUpdate(schema)

	let variantName = ''
	function createVariant(name: string) {
		if (schema.oneOf) {
			if (schema.oneOf.some((obj) => obj.title === name)) {
				throw new Error('Variant name already exists')
			}
			const idx = schema.oneOf.findIndex((obj) => obj.title === name)
			if (idx === -1) {
				schema.oneOf = [
					...schema.oneOf,
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
		if (schema.oneOf) {
			if (schema.oneOf.some((obj) => obj.title === name)) {
				throw new Error('Variant name already exists')
			}
			const idx = schema.oneOf.findIndex((obj) => obj.title === selected)
			if (idx !== -1) {
				schema.oneOf[idx].title = name
				oneOfSelected = name
			}
			variantName = ''
		}
	}

	let initialObjectSelected =
		Object.keys(schema?.properties ?? {}).length == 0 ? 'resource' : 'custom-object'
</script>

<div class="flex flex-col gap-2">
	{#if type === 'object' && schema.oneOf && schema.oneOf.length >= 2}
		<div class="flex flex-row gap-1 items-center justify-start">
			<ToggleButtonGroup
				bind:selected={oneOfSelected}
				class="h-auto w-auto"
				tabListClass="flex-wrap"
				let:item
			>
				{#each schema.oneOf as obj}
					<ToggleButton value={obj.title ?? ''} label={obj.title} {item} />
				{/each}
			</ToggleButtonGroup>

			<Popover placement="bottom-end" closeButton>
				<svelte:fragment slot="trigger">
					<Button size="xs2" color="light" nonCaptureEvent startIcon={{ icon: Plus }} />
				</svelte:fragment>
				<svelte:fragment slot="content" let:close>
					<Label label="Label" class="p-2 flex flex-col gap-2">
						<input
							type="text"
							class="w-full !bg-surface"
							on:keydown={(event) => {
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
				</svelte:fragment>
			</Popover>
		</div>
		<div class="flex flex-row gap-2 items-center">
			<span class="font-semibold text-sm">{oneOfSelected}</span>

			<Popover
				floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
				containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
				closeButton
			>
				<svelte:fragment slot="trigger">
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
				</svelte:fragment>
				<svelte:fragment slot="content" let:close>
					<Label label="Label" class="p-2 flex flex-col gap-2">
						<input
							type="text"
							class="w-full !bg-surface"
							on:keydown={(event) => {
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
				</svelte:fragment>
			</Popover>
			<Button
				size="xs2"
				color="red"
				startIcon={{ icon: Trash2 }}
				iconOnly
				disabled={schema.oneOf.length <= 2}
				on:click={() => {
					if (schema.oneOf && oneOfSelected) {
						const idx = schema.oneOf.findIndex((obj) => obj.title === oneOfSelected)
						schema.oneOf = schema.oneOf.filter((_, i) => i !== idx)
						oneOfSelected = schema.oneOf[0].title
					}
				}}
			/>
		</div>
		{#if oneOfSelected && schema.oneOf}
			{@const idx = schema.oneOf.findIndex((obj) => obj.title === oneOfSelected)}
			<EditableSchemaDrawer
				bind:schema={schema.oneOf[idx]}
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
			<svelte:fragment slot="content">
				<div class="pt-2">
					<TabContent value="custom-object">
						<EditableSchemaDrawer bind:schema on:change={() => dispatch('schemaChange')} />
					</TabContent>

					<TabContent value="resource">
						<ObjectTypeNarrowing on:change={() => dispatch('schemaChange')} bind:format />
					</TabContent>
				</div>
			</svelte:fragment>
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
