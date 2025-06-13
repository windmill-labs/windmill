<script lang="ts">
	import autosize from '$lib/autosize'
	import { shouldDisplayPlaceholder } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
	import ArrayTypeNarrowing from '../ArrayTypeNarrowing.svelte'
	import Label from '../Label.svelte'
	import NumberTypeNarrowing from '../NumberTypeNarrowing.svelte'
	import StringTypeNarrowing from '../StringTypeNarrowing.svelte'
	import Tooltip from '../Tooltip.svelte'

	import EditableSchemaForm from '../EditableSchemaForm.svelte'
	import { deepEqual } from 'fast-equals'
	import type { EnumType } from '$lib/common'
	import type { SchemaProperty } from '$lib/common'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import { createEventDispatcher, onMount } from 'svelte'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'

	export let description: string = ''
	export let format: string | undefined = undefined
	export let contentEncoding: 'base64' | 'binary' | undefined = undefined
	export let type: string | undefined = undefined
	export let oneOf: SchemaProperty[] | undefined = undefined
	export let pattern: undefined | string = undefined
	export let enum_: EnumType = undefined
	export let extra: Record<string, any> = {}
	export let minW = true
	export let customErrorMessage: string | undefined = undefined
	export let title: string | undefined = undefined
	export let placeholder: string | undefined = undefined
	export let properties: Record<string, any> | undefined = undefined
	export let isFlowInput: boolean = false
	export let isAppInput: boolean = false
	export let order: string[] | undefined = undefined
	export let itemsType:
		| {
				type?: 'string' | 'number' | 'bytes' | 'object' | 'resource'
				contentEncoding?: 'base64'
				enum?: string[]
				resourceType?: string
				multiselect?: string[]
		  }
		| undefined = undefined

	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)
	let el: HTMLTextAreaElement | undefined = undefined

	let oneOfSelected: string | undefined =
		(oneOf && oneOf.length >= 2 && oneOf[0]['title']) || undefined
	let oneOfSchemas: any[] | undefined = undefined
	function updateOneOfSchemas(oneOf: SchemaProperty[] | undefined) {
		if (oneOf && oneOf.length >= 2) {
			oneOfSchemas = oneOf.map((obj) => {
				return {
					properties: obj.properties
						? Object.fromEntries(
								Object.entries(obj.properties).filter(([k, v]) => k !== 'label' && k !== 'kind')
							)
						: {},
					order: obj.order
				}
			})
		}
	}
	$: updateOneOfSchemas(oneOf)

	export function focus() {
		el?.focus()
		if (el) {
			el.style.height = '5px'
			el.style.height = el.scrollHeight + 50 + 'px'
		}
	}

	function onKeyDown(e: KeyboardEvent) {
		if ((e.ctrlKey || e.metaKey) && e.key == 'Enter') {
			return
		}
		e.stopPropagation()
	}

	let schema = {
		properties,
		order
	}

	let initialExtra: any = structuredClone({ order: undefined, properties: undefined, ...extra })

	let mounted = false
	let firstOnContentChange = true
	onMount(() => {
		setTimeout(() => {
			mounted = true
		}, 500)
	})

	$: extra && mounted && onContentChange()

	function onContentChange() {
		if (firstOnContentChange) {
			firstOnContentChange = false
			return
		}
		if (!deepEqual(extra, initialExtra)) {
			initialExtra = structuredClone(extra)
			console.debug('property content updated')
			dispatchIfMounted('change')
		}
	}

	$: (properties || order) && updateSchema()

	function updateSchema() {
		if (!deepEqual(schema.properties, properties) || !deepEqual(schema.order, order)) {
			schema = {
				properties,
				order
			}
			console.debug('property schema updated')
			dispatchIfMounted('change')
		}
	}
</script>

<div class="flex flex-row items-center justify-between w-full gap-2">
	<!-- svelte-ignore a11y-autofocus -->
	<div class={twMerge('flex flex-col w-full', 'gap-4', minW ? 'min-w-[250px]' : '')}>
		<slot name="typeeditor" />

		<Label label="Description">
			<textarea
				use:autosize
				rows="2"
				bind:value={description}
				on:keydown={onKeyDown}
				placeholder="Field description"
			></textarea>
		</Label>

		<Label label="Custom title" class="w-full">
			<svelte:fragment slot="header">
				<Tooltip light>Will be displayed in the UI instead of the field name.</Tooltip>
			</svelte:fragment>
			<input bind:value={title} on:keydown={onKeyDown} placeholder="Field title" />
		</Label>

		<Label label="Placeholder">
			<svelte:fragment slot="header">
				<Tooltip light>
					Will be displayed in the input field when the field is empty. If not set, the default
					value will be used. The placeholder is disabled depending on the field type, format, etc.
				</Tooltip>
			</svelte:fragment>

			<textarea
				placeholder="Enter a placeholder"
				rows="1"
				bind:value={placeholder}
				on:change={() => dispatch('change')}
				disabled={!shouldDisplayPlaceholder(type, format, enum_, contentEncoding, pattern, extra)}
			></textarea>
		</Label>

		{#if type == 'array'}
			<ArrayTypeNarrowing
				originalType={extra['originalType']}
				bind:itemsType
				canEditResourceType={isFlowInput || isAppInput}
			/>
		{:else if type == 'string' || ['number', 'integer', 'object'].includes(type ?? '')}
			<div>
				<Label label="Field settings">
					<div>
						{#if type == 'string'}
							<StringTypeNarrowing
								bind:customErrorMessage
								bind:format
								bind:pattern
								bind:enum_
								bind:contentEncoding
								bind:password={extra['password']}
								bind:minRows={extra['minRows']}
								bind:disableCreate={extra['disableCreate']}
								bind:disableVariablePicker={extra['disableVariablePicker']}
								bind:dateFormat={extra['dateFormat']}
								bind:enumLabels={extra['enumLabels']}
								originalType={extra['originalType']}
								overrideAllowKindChange={isFlowInput || isAppInput}
							/>
						{:else if type == 'number' || type == 'integer'}
							<NumberTypeNarrowing
								bind:min={extra['min']}
								bind:max={extra['max']}
								bind:currency={extra['currency']}
								bind:currencyLocale={extra['currencyLocale']}
							/>
						{:else if type == 'object' && oneOf && oneOf.length >= 2 && !isFlowInput && !isAppInput}
							<ToggleButtonGroup bind:selected={oneOfSelected} class="mb-2" let:item>
								{#each oneOf as obj}
									<ToggleButton value={obj.title ?? ''} label={obj.title} {item} />
								{/each}
							</ToggleButtonGroup>
							{#if oneOfSelected && oneOfSchemas}
								{@const idx = oneOf.findIndex((obj) => obj.title === oneOfSelected)}
								<div class="border">
									<EditableSchemaForm
										on:change
										noPreview
										bind:schema={oneOfSchemas[idx]}
										uiOnly
										jsonEnabled={false}
										editTab="inputEditor"
									/>
								</div>
							{/if}
						{:else if type == 'object' && format?.startsWith('dynselect-')}
							<div class="text-tertiary text-xs">No settings available for Dynamic Select</div>
						{:else if type == 'object' && !format?.startsWith('resource-') && !isFlowInput && !isAppInput}
							<div class="border">
								<EditableSchemaForm
									on:change
									noPreview
									bind:schema
									uiOnly
									jsonEnabled={false}
									editTab="inputEditor"
								/>
							</div>
						{:else}
							<div class="text-tertiary text-xs">No settings available for this field type</div>
						{/if}
					</div>
				</Label>
			</div>
		{/if}

		<slot />
	</div>
</div>

<style>
	input::-webkit-outer-spin-button,
	input::-webkit-inner-spin-button {
		-webkit-appearance: none !important;
		margin: 0;
	}
</style>
