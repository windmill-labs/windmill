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
	import { createEventDispatcher, onMount, untrack } from 'svelte'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'
	import TextInput, { inputBaseClass, inputBorderClass } from '../text_input/TextInput.svelte'

	interface Props {
		description?: string
		format?: string | undefined
		contentEncoding?: 'base64' | 'binary' | undefined
		type?: string | undefined
		oneOf?: SchemaProperty[] | undefined
		pattern?: undefined | string
		enum_?: EnumType
		extra?: Record<string, any>
		minW?: boolean
		customErrorMessage?: string | undefined
		title?: string | undefined
		placeholder?: string | undefined
		properties?: Record<string, any> | undefined
		nonEmpty?: boolean | undefined
		isFlowInput?: boolean
		isAppInput?: boolean
		order?: string[] | undefined
		itemsType?:
			| {
					type?: 'string' | 'number' | 'bytes' | 'object' | 'resource'
					contentEncoding?: 'base64'
					enum?: string[]
					resourceType?: string
					multiselect?: string[]
			  }
			| undefined
		typeeditor?: import('svelte').Snippet
		children?: import('svelte').Snippet
	}

	let {
		description = $bindable(undefined),
		format = $bindable(undefined),
		contentEncoding = $bindable(undefined),
		type = undefined,
		oneOf = $bindable(),
		pattern = $bindable(undefined),
		enum_ = $bindable(undefined),
		extra = $bindable({}),
		nonEmpty = $bindable(undefined),
		minW = true,
		customErrorMessage = $bindable(undefined),
		title = $bindable(undefined),
		placeholder = $bindable(undefined),
		properties = $bindable(),
		isFlowInput = false,
		isAppInput = false,
		order = $bindable(),
		itemsType = $bindable(undefined),
		typeeditor,
		children
	}: Props = $props()

	$effect.pre(() => {
		if (description == undefined) {
			description = ''
		}
	})

	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)
	let el: HTMLTextAreaElement | undefined = undefined

	let oneOfSelected: string | undefined = $state(
		(oneOf && oneOf.length >= 2 && oneOf[0]['title']) || undefined
	)
	let oneOfSchemas: any[] | undefined = $state(undefined)
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

	let schema = $state({
		properties,
		order
	})

	let initialExtra: any = structuredClone({
		order: undefined,
		properties: undefined,
		...$state.snapshot(extra)
	})

	let mounted = $state(false)
	let firstOnContentChange = true
	onMount(() => {
		setTimeout(() => {
			mounted = true
		}, 500)
	})

	function onContentChange() {
		if (firstOnContentChange) {
			firstOnContentChange = false
			return
		}
		if (!deepEqual(extra, initialExtra)) {
			initialExtra = structuredClone($state.snapshot(extra))
			console.debug('property content updated')
			dispatchIfMounted('change')
		}
	}

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
	$effect(() => {
		;[oneOf]
		untrack(() => updateOneOfSchemas(oneOf))
	})
	$effect(() => {
		extra && mounted && untrack(() => onContentChange())
	})
	$effect(() => {
		;(properties || order) && untrack(() => updateSchema())
	})
</script>

<div class="flex flex-row items-center justify-between w-full gap-2">
	<!-- svelte-ignore a11y_autofocus -->
	<div class={twMerge('flex flex-col w-full', 'gap-4', minW ? 'min-w-[250px]' : '')}>
		{@render typeeditor?.()}

		<Label label="Description">
			<textarea
				use:autosize
				rows="2"
				bind:value={description}
				onkeydown={onKeyDown}
				onchange={() => dispatch('change')}
				placeholder="Field description"
				class={twMerge(inputBorderClass(), inputBaseClass, 'w-full')}
			></textarea>
		</Label>

		<Label label="Custom title" class="w-full">
			{#snippet header()}
				<Tooltip light>Will be displayed in the UI instead of the field name.</Tooltip>
			{/snippet}
			<TextInput
				bind:value={title}
				inputProps={{ placeholder: 'Field title', onkeydown, onchange: () => dispatch('change') }}
			/>
		</Label>

		{#if shouldDisplayPlaceholder(type, format, enum_, contentEncoding, pattern, extra)}
			<Label label="Placeholder">
				{#snippet header()}
					<Tooltip light>
						Will be displayed in the input field when the field is empty. If not set, the default
						value will be used. The placeholder is disabled depending on the field type, format,
						etc.
					</Tooltip>
				{/snippet}

				<textarea
					placeholder="Enter a placeholder"
					rows="1"
					bind:value={placeholder}
					onchange={() => dispatch('change')}
					class={twMerge(inputBorderClass(), inputBaseClass, 'w-full')}
				></textarea>
			</Label>
		{/if}

		{#if type == 'array'}
			<ArrayTypeNarrowing
				originalType={extra['originalType']}
				bind:itemsType
				canEditResourceType={isFlowInput || isAppInput}
				bind:nonEmpty
			/>
		{:else if type == 'string' || ['number', 'integer'].includes(type ?? '')}
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
								bind:seconds={extra['seconds']}
							/>
						{/if}
					</div>
				</Label>
			</div>
		{:else if type == 'object' && oneOf && oneOf.length >= 2 && !isFlowInput && !isAppInput}
			<ToggleButtonGroup bind:selected={oneOfSelected} class="mb-2">
				{#snippet children({ item })}
					{#each oneOf as obj}
						<ToggleButton value={obj.title ?? ''} label={obj.title} {item} />
					{/each}
				{/snippet}
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
						{isFlowInput}
						{isAppInput}
					/>
				</div>
			{/if}
		{:else if type == 'object' && !format?.startsWith('resource-') && !isFlowInput && !isAppInput}
			<div class="border">
				<EditableSchemaForm
					on:change
					noPreview
					bind:schema
					uiOnly
					jsonEnabled={false}
					editTab="inputEditor"
					{isFlowInput}
					{isAppInput}
				/>
			</div>
		{/if}

		{@render children?.()}
	</div>
</div>
