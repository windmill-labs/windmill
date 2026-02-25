<script lang="ts" module>
	import { z } from 'zod'
	import { useSearchParams } from '$lib/svelte5UtilsKit.svelte'
	import { formatDatePretty, parsePrettyDate, type IconType } from '$lib/utils'

	export type FilterSchemaRec = Record<string, FilterSchema>
	export type FilterSchema = (
		| {
				type: 'string' | 'number' | 'boolean'
				allowMultiple?: boolean
				format?: 'json'
		  }
		| {
				type: 'date'
				mode?: 'single' | 'end' | 'start'
				otherField?: string // For range display
				allowMultiple?: undefined
		  }
		| {
				type: 'oneof'
				options: { value: string; label?: string; description?: string }[]
				allowCustomValue?: boolean
				allowNegative?: boolean
				allowMultiple?: boolean
		  }
	) & {
		label?: string
		description?: string
		icon?: IconType
	}
	export type FilterInstanceRec<T extends FilterSchemaRec> = {
		[K in keyof T]: FilterInstance<T[K]>
	}
	export type FilterInstance<T extends FilterSchema> = T extends { type: 'string' }
		? string
		: T extends { type: 'number' }
			? number
			: T extends { type: 'boolean' }
				? boolean
				: T extends { type: 'date' }
					? Date
					: T extends { type: 'oneof'; options: any; allowCustomValue?: infer A }
						? A extends true
							? string
							: T['options'] extends { value: string }[]
								? _NegativeFilterInstance<T>
								: never
						: never

	type _NegativeFilterInstance<T extends { options: { value: string }[] }> = T extends {
		allowNegative: true
	}
		? T['options'][number]['value'] | `!${T['options'][number]['value']}`
		: T['options'][number]['value']

	/**
	 * Converts a FilterSchemaRec to a Zod schema for validation
	 */
	export function filterSchemaRecToZodSchema<T extends FilterSchemaRec>(
		schemaRec: T
	): z.ZodObject<{
		[K in keyof T]: z.ZodType<FilterInstance<T[K]>>
	}> {
		const zodSchemaShape: Record<string, z.ZodType> = {}

		for (const [key, filterSchema] of Object.entries(schemaRec)) {
			let fieldSchema: z.ZodType

			if (filterSchema.type === 'string') {
				fieldSchema = z.string().nullable().default(null)
			} else if (filterSchema.type === 'number') {
				fieldSchema = z.number().nullable().default(null)
			} else if (filterSchema.type === 'boolean') {
				fieldSchema = z.boolean().nullable().default(null)
			} else if (filterSchema.type === 'date') {
				fieldSchema = z.string().nullable().default(null)
			} else if (filterSchema.type === 'oneof') {
				if (filterSchema.allowCustomValue) {
					// If custom values are allowed, accept any string
					fieldSchema = z.string().nullable().default(null)
				} else {
					// Extract the enum values from options
					const values = filterSchema.options.map((o) => o.value) as [string, ...string[]]
					fieldSchema = z.enum(values).nullable().default(null)
				}
			} else {
				// Fallback for unknown types
				fieldSchema = z.any().nullable().default(null)
			}

			zodSchemaShape[key] = fieldSchema
		}

		return z.object(zodSchemaShape) as any
	}

	/**
	 * Creates a URL-synced filter instance that automatically syncs with URL search parameters
	 */
	export function useUrlSyncedFilterInstance<T extends FilterSchemaRec>(
		schemaRec: T
	): { val: Partial<FilterInstanceRec<T>> } {
		// Build the Zod schema from the filter schema
		const zodSchema = filterSchemaRecToZodSchema(schemaRec)

		// Create URL-synced search params
		const urlFilter = useSearchParams(zodSchema) as Record<string, unknown>

		// Create the filter instance object
		const filterInstance: { val: Partial<FilterInstanceRec<T>> } = $state({ val: {} })

		// Sync URL params to filter instance on initialization and when URL changes
		for (const key of Object.keys(schemaRec)) {
			let urlValue = urlFilter[key]
			if (schemaRec[key].type === 'date' && typeof urlValue === 'string') {
				const d = new Date(urlValue)
				urlValue = isNaN(d.getTime()) ? null : d
			}
			if (urlValue !== undefined && urlValue !== null) {
				;(filterInstance.val as any)[key] = urlValue
			}
		}

		// Sync filter instance changes back to URL params
		for (const key of Object.keys(schemaRec)) {
			$effect(() => {
				let filterValue = (filterInstance.val as any)[key]
				if (schemaRec[key].type === 'date' && filterValue instanceof Date) {
					// Convert Date to ISO string for URL
					filterValue = filterValue.toISOString()
				}
				if (untrack(() => urlFilter[key]) == filterValue) return // Avoid unnecessary updates
				if (filterValue !== undefined && filterValue !== null) {
					urlFilter[key] = filterValue
				} else {
					urlFilter[key] = null
				}
			})
		}

		return filterInstance
	}

	function filterToText<F extends FilterSchema>(filter: FilterInstance<F>, schema: F): string {
		if (schema.type === 'date') {
			const date =
				typeof filter === 'string'
					? new Date(filter)
					: typeof filter === 'number'
						? new Date(filter)
						: (filter as Date)
			return formatDatePretty(date)
		}
		return String(filter)
	}

	function textToFilter(text: string, schema: FilterSchema): FilterInstance<FilterSchema> | null {
		if (schema.type === 'string') return text
		if (schema.type === 'number') {
			const num = Number(text)
			return isNaN(num) ? null : (num as any)
		}
		if (schema.type === 'boolean') {
			if (text.toLowerCase() === 'true') return true as any
			if (text.toLowerCase() === 'false') return false as any
			return null
		}
		if (schema.type === 'date') {
			const date = parsePrettyDate(text)
			return date ? (date as any) : null
		}
		if (schema.type === 'oneof') {
			return text
		}
		return null
	}

	export type FilterValidationError = { fields: string[]; error: string }

	/**
	 * Validates a filter instance against its schema.
	 * Returns a list of validation errors, each with the affected fields and an error message.
	 */
	export function validateFilterInstance<T extends FilterSchemaRec>(
		schemaRec: T,
		instance: Partial<FilterInstanceRec<T>>
	): FilterValidationError[] {
		const errors: FilterValidationError[] = []

		for (const [key, rawValue] of Object.entries(instance)) {
			const schema = schemaRec[key]
			if (!schema) continue

			if (schema.type === 'date') {
				if (!rawValue || !((rawValue as any) instanceof Date) || isNaN(rawValue.getTime())) {
					errors.push({ fields: [key], error: `Invalid date format` })
				}
			} else if (schema.type === 'oneof') {
				const strValue = String(rawValue)
				const elements = schema.allowMultiple ? strValue.split(',') : [strValue]
				const validValues = schema.options.map((o) => o.value)

				if (schema.allowMultiple && schema.allowNegative) {
					const hasPositive = elements.some((v) => !v.startsWith('!'))
					const hasNegative = elements.some((v) => v.startsWith('!'))
					if (hasPositive && hasNegative) {
						errors.push({
							fields: [key],
							error: `Cannot mix positive and negative values`
						})
						continue
					}
				}

				if (!schema.allowCustomValue) {
					const invalid = elements
						.map((v) => v.replace(/^!/, ''))
						.filter((v) => !validValues.includes(v))
					if (invalid.length > 0) {
						errors.push({
							fields: [key],
							error: `Invalid value${invalid.length > 1 ? 's' : ''}: ${invalid.join(', ')}`
						})
					}
				}
			} else if (schema.type === 'string' && schema.format === 'json') {
				try {
					JSON.parse(String(rawValue))
				} catch (e) {
					errors.push({ fields: [key], error: `Invalid JSON format` })
				}
			}
		}

		return errors
	}
</script>

<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import { inputBaseClass, inputBorderClass, inputSizeClasses } from './text_input/TextInput.svelte'
	import { MinusIcon, SearchIcon } from 'lucide-svelte'
	import { assignObjInPlace, clone } from '$lib/utils'
	import GenericDropdown from './select/GenericDropdown.svelte'
	import SimpleEditor from './SimpleEditor.svelte'
	import TaggedTextInput from './TaggedTextInput.svelte'
	import { DebouncedTempValue, useTransformedSyncedValue } from '$lib/svelte5Utils.svelte'
	import { untrack } from 'svelte'
	import CloseButton from './common/CloseButton.svelte'
	import Popover from './meltComponents/Popover.svelte'
	import Button from './common/button/Button.svelte'
	import Badge from './common/badge/Badge.svelte'
	import InlineCalendarInput, {
		fromCalendarDate,
		toCalendarDate
	} from './common/InlineCalendarInput.svelte'
	import { ButtonType } from './common'

	type Props<SchemaT extends FilterSchemaRec> = {
		schema: SchemaT
		value: Partial<FilterInstanceRec<SchemaT>>
		presets?: { name: string; value: string }[]
		class?: string
		placeholder?: string
	}

	type SchemaT = FilterSchemaRec // TODO: Generic
	let {
		schema,
		value: valueInput = $bindable(),
		presets: _presets = [],
		class: className,
		placeholder = 'Filter...'
	}: Props<SchemaT> = $props()

	let _value = new DebouncedTempValue(
		() => clone(valueInput),
		(v) => !errors.length && (valueInput = clone(v)),
		(t) => Object.entries(t)
	)
	let value = $derived(_value.current)
	let errors = $derived(validateFilterInstance(schema, value))

	let currentTag: keyof SchemaT | undefined = $state()
	let currentTextSegment = $state({ text: '', start: 0, end: 0 })
	let open = $state(false)
	let inputElement: HTMLDivElement | undefined = $state()
	let highlightedIndex = $state(0)
	let taggedTextInput: TaggedTextInput | undefined = $state()

	let tags = $derived(
		Object.entries(schema).map(([key, filterSchema]) => ({
			regex: new RegExp(`\\b${key}:(?:\\\\.|[^\\s])*`, 'g'),
			id: key,
			onClear: () => (delete value[key], asText.reparse())
		}))
	)

	let keyHighlightRegex = $derived(new RegExp(`\\b(${Object.keys(schema).join('|')}):`, 'g'))

	let errorKeys = $derived(new Set(errors.flatMap((e) => e.fields)))
	let errorHighlights = $derived(
		[...errorKeys].map((key) => ({
			regex: new RegExp(`(?<=\\b${key}:)(?:\\\\.|[^\\s])+`),
			classes: 'text-red-500 dark:text-red-400'
		}))
	)

	let menuItems = $derived.by(() => {
		if (!currentTag) {
			const searchText = currentTextSegment.text.trim().toLowerCase()
			return Object.entries(schema)
				.filter(([k, _]) => !(k in value))
				.filter(([k, filterSchema]) => {
					if (!searchText) return true
					const label = (filterSchema.label || k).toLowerCase()
					const key = k.toLowerCase()
					return label.includes(searchText) || key.includes(searchText)
				})
				.map(([key, filterSchema]) => ({
					type: 'filter' as const,
					key,
					filterSchema,
					onClick: () => {
						// Replace the text segment with the new filter tag
						const before = asText.val.slice(0, currentTextSegment.start)
						const after = asText.val.slice(currentTextSegment.end)
						asText.val =
							`${before}${before && !before.endsWith(' ') ? ' ' : ''}${key}:\\\u00A0${after}`.trim() +
							'\u00A0'
					}
				}))
		} else {
			const filter = schema[currentTag]
			if (filter.type === 'oneof') {
				// When allowMultiple, split on comma and match against the last segment
				const currentVal = String(value[currentTag!] ?? '')
				let searchSuffix: string
				if (filter.allowMultiple) {
					const parts = currentVal.split(',')
					searchSuffix = parts[parts.length - 1].replace(/^!/, '').trim()
				} else {
					searchSuffix = currentVal
				}

				// Already-selected values (for allowMultiple, to avoid suggesting duplicates)
				const selectedValues = filter.allowMultiple
					? currentVal
							.split(',')
							.slice(0, -1)
							.map((v) => v.replace(/^!/, '').trim())
					: []

				return filter.options
					.filter((o) => {
						if (selectedValues.includes(o.value)) return false
						if (!searchSuffix) return true
						return o.value.includes(searchSuffix)
					})
					.map((option) => ({
						type: 'option' as const,
						option,
						onClick: () =>
							appendOrSetValueForCurrentTag((currentVal.includes('!') ? '!' : '') + option.value),
						onNegativeClick: filter.allowNegative
							? () => appendOrSetValueForCurrentTag('!' + option.value)
							: undefined
					}))
			} else if (filter.type === 'boolean') {
				return [
					{
						type: 'boolean' as const,
						value: true,
						label: 'True',
						onClick: () => setValueForCurrentTag(true)
					},
					{
						type: 'boolean' as const,
						value: false,
						label: 'False',
						onClick: () => setValueForCurrentTag(false)
					}
				]
			}
		}
		return []
	})

	// Reset highlighted index when menu items change
	$effect(() => {
		menuItems
		open
		highlightedIndex = 0
	})

	const kvRegex = /\b(\w+):((?:[^\s\\]|\\.)*)/g

	function parseFromText(text: string): Partial<FilterInstanceRec<SchemaT>> {
		const parsed: Record<string, string> = {}
		let match
		while ((match = kvRegex.exec(text)) !== null) {
			let [_, key, val] = match
			if (key in schema) {
				val ??= ''
				val = val.replace(/\\(.)/g, (_: string, c: string) => {
					if (c === 'n') return '\n'
					if (c === 'r') return '\r'
					return c
				}) // Unescape escaped characters
				val = val.trim()
				parsed[key] = textToFilter(val, schema[key]) as any
			}
		}
		return parsed
	}

	function parseToText(v: Partial<FilterInstanceRec<SchemaT>>): string {
		return (
			Object.entries(v)
				.map(([key, val]) =>
					`${key}: ${filterToText(val as any, schema[key])}`
						.replace(/ /g, '\\ ')
						.replace(/\n/g, '\\n')
						.replace(/\r/g, '\\r')
				)
				.join(' ') + '\u00A0'
		)
	}

	let asText = useTransformedSyncedValue(
		[() => (Object.entries(value), value), (v) => assignObjInPlace(value, v)],
		parseToText,
		parseFromText
	)

	function setValueForCurrentTag(val: any) {
		if (!currentTag) return
		value[currentTag!] = val
		asText.reparse()
	}

	/**
	 * For allowMultiple fields: appends a new value to the existing comma-separated list,
	 * replacing the last (in-progress) segment. For non-allowMultiple fields, behaves like setValueForCurrentTag.
	 */
	function appendOrSetValueForCurrentTag(val: string) {
		if (!currentTag) return
		const filter = schema[currentTag]
		if (filter.allowMultiple) {
			const existing = String(value[currentTag!] ?? '')
			const parts = existing.split(',')
			// If any existing part is negative, force the new value to be negative too
			const isNegativeContext = parts.slice(0, -1).some((p) => p.startsWith('!'))
			if (isNegativeContext && !val.startsWith('!')) val = '!' + val
			// Replace the last in-progress segment with the selected value
			parts[parts.length - 1] = val
			value[currentTag!] = parts.join(',')
		} else {
			value[currentTag!] = val
		}
		asText.reparse()
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (!open) return
		if (e.key === 'Escape') {
			open = false
			return
		}

		if (menuItems.length && e.key === 'ArrowDown') {
			highlightedIndex = (highlightedIndex + 1) % menuItems.length
		} else if (menuItems.length && e.key === 'ArrowUp') {
			highlightedIndex = (highlightedIndex - 1 + menuItems.length) % menuItems.length
		} else if (e.key === 'Enter') {
			if (menuItems[highlightedIndex]) {
				menuItems[highlightedIndex].onClick()
			} else {
				setValueForCurrentTag(value[currentTag!])
				taggedTextInput?.focusAtEnd()
			}
			const currTagSchema = currentTag ? schema[currentTag] : undefined
			if (currTagSchema && 'format' in currTagSchema && currTagSchema.format === 'json') {
				return
			}
		} else {
			return
		}
		e.preventDefault()
	}

	type Preset = { name: string; value: string }
	let presets: Preset[] = $derived(
		_presets.filter((p) => {
			// Only show presets that aren't already applied in asText
			return !asText.val.includes(p.value)
		})
	)

	function appendFilterAsText(presetValue: string) {
		if (!asText.val.endsWith('\u00A0') && !asText.val.endsWith(' ')) asText.val += ' '
		asText.val += presetValue + '\u00A0'
	}
</script>

<svelte:window onmousedown={() => (open = false)} onkeydown={handleKeyDown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
	class={twMerge(
		'flex items-center rounded-md bg-surface-input overflow-clip',
		inputBorderClass({ error: errors.length > 0, forceFocus: open }),
		ButtonType.UnifiedHeightClasses.md,
		className
	)}
	onmousedown={(e) => {
		if (!open) {
			e.preventDefault()
			if (!asText.val.endsWith('\u00A0') && !asText.val.endsWith(' ')) asText.val += '\u00A0'
			taggedTextInput?.focusAtEnd()
		}
		open = true
		e.stopPropagation()
	}}
	bind:this={inputElement}
>
	<TaggedTextInput
		bind:this={taggedTextInput}
		bind:value={asText.val}
		{tags}
		highlights={[
			{ regex: /![a-zA-Z0-9_\-\/]+/, classes: 'text-yellow-600 dark:text-yellow-500' },
			{ regex: keyHighlightRegex, classes: 'text-hint' },
			{ regex: /,/, classes: 'text-hint mr-0.5' },
			...errorHighlights
		]}
		onCurrentTagChange={(tag) => (currentTag = tag ? (tag.id as keyof SchemaT) : undefined)}
		onTextSegmentAtCursorChange={(segment) => (currentTextSegment = segment)}
		class={twMerge(
			'overflow-x-auto !pr-24 bg-surface-input outline-none scrollbar-hidden text-nowrap flex-1 mr-2 mt-0.5',
			inputBaseClass,
			inputSizeClasses.md
		)}
		{placeholder}
	/>
	{#if asText.val}
		<CloseButton small class="mr-1.5" onClick={() => (_value.current = {})} />
	{:else}
		<div class="mr-3">
			<SearchIcon size={16} class="text-hint" />
		</div>
	{/if}
</div>

<GenericDropdown
	{open}
	getInputRect={() => inputElement?.getBoundingClientRect() ?? new DOMRect()}
	innerClass="!max-h-[30rem]"
	strictWidth
>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="py-1 p-2 overflow-y-auto" onmousedown={(e) => e.stopPropagation()}>
		{#if !currentTag || !schema[currentTag]}
			{#if presets.length}
				<div class="text-xs px-2 my-2 font-bold">Presets</div>
				<div class="mb-3 px-2 flex gap-2 flex-wrap">
					{#each presets as preset}
						{@render presetTag(preset)}
					{/each}
				</div>
			{/if}
			<div class="text-xs px-2 my-2 font-bold">Filters</div>
			{#each menuItems as item, index}
				{#if item.type === 'filter' && item.filterSchema}
					{@render menuItem({
						Icon: item.filterSchema.icon || SearchIcon,
						onClick: item.onClick,
						label: item.filterSchema.label || item.key,
						description: item.filterSchema.description,
						highlighted: index === highlightedIndex
					})}
				{/if}
			{/each}
		{:else}
			{#key currentTag}
				{@render suggestion(schema[currentTag])}
			{/key}
		{/if}
	</div>
</GenericDropdown>

{#snippet suggestion(filter: FilterSchema)}
	{#if filter.description}
		<div class="text-xs text-secondary px-2 my-2">{filter.description}</div>
	{/if}
	{#if filter.allowMultiple && (filter.type === 'string' || filter.type === 'oneof')}
		<div class="text-2xs text-hint px-2 -mt-1 mb-2">Separate multiple values with commas</div>
	{/if}
	{#if filter.type === 'oneof'}
		<div class="max-h-60 overflow-y-auto">
			{#each menuItems as item, index}
				{#if item.type === 'option' && item.option}
					{@render menuItem({
						onClick: item.onClick,
						label: item.option.label || item.option.value,
						highlighted: index === highlightedIndex,
						onNegativeClick: item.onNegativeClick
					})}
				{/if}
			{/each}
		</div>
	{:else if filter.type === 'boolean'}
		{#each menuItems as item, index}
			{#if item.type === 'boolean' && item.label}
				{@render menuItem({
					onClick: item.onClick,
					label: item.label,
					highlighted: index === highlightedIndex
				})}
			{/if}
		{/each}
	{:else if filter.type === 'date'}
		{@const filterMode = filter.mode}
		<div class="p-3 mb-1">
			{#if !filterMode || filterMode === 'single'}
				<InlineCalendarInput
					bind:value={
						() => toCalendarDate(value[currentTag!]),
						(v) => {
							setValueForCurrentTag(fromCalendarDate(v))
							taggedTextInput?.preventCursorMoveOnNextSync()
						}
					}
				/>
			{:else}
				{@const curr = toCalendarDate(value[currentTag!])}
				{@const obj =
					filterMode === 'end'
						? { start: toCalendarDate(value[filter.otherField as keyof SchemaT]), end: curr }
						: { end: toCalendarDate(value[filter.otherField as keyof SchemaT]), start: curr }}
				<InlineCalendarInput
					mode="range"
					onClickBehavior={`set-${filterMode}`}
					infiniteRange
					bind:value={
						() => obj,
						(v) => {
							setValueForCurrentTag(fromCalendarDate(v[filterMode]))
							taggedTextInput?.preventCursorMoveOnNextSync()
						}
					}
				/>
			{/if}
		</div>
	{:else if filter.type === 'string' && filter.format === 'json'}
		<div class="px-2 pb-2">
			<SimpleEditor
				autofocus={String(value[currentTag!] ?? '').length === 0}
				lang="json"
				autoHeight
				small
				bind:code={
					() => String(value[currentTag!] ?? ''),
					(v) => {
						setValueForCurrentTag(v ?? '')
						taggedTextInput?.preventCursorMoveOnNextSync()
					}
				}
				class="border border-border-light rounded min-h-[4rem]"
			/>
		</div>
	{/if}
{/snippet}

{#snippet menuItem({
	Icon,
	onClick,
	label,
	description,
	highlighted = false,
	onNegativeClick
}: {
	Icon?: IconType
	onClick: () => void
	label: string
	description?: string
	highlighted?: boolean
	onNegativeClick?: () => void
})}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		class={twMerge(
			'py-1.5 px-2 rounded-md hover:bg-surface-hover cursor-pointer text-sm flex items-center gap-3',
			highlighted && 'bg-surface-hover'
		)}
		onclick={onClick}
	>
		{#if Icon}
			<Icon size={16} class="inline" />
		{/if}
		<div class="inline flex-1 relative min-w-0">
			<div class="text-sm ellipsize">{label}</div>
			{#if description}
				<div class="text-xs text-hint">{description}</div>
			{/if}
		</div>
		{#if onNegativeClick}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<Popover openOnHover portal={null}>
				{#snippet trigger()}
					<Button
						onClick={(e) => (e?.stopPropagation(), onNegativeClick?.())}
						iconOnly
						endIcon={{ icon: MinusIcon }}
						unifiedSize="xs"
						destructive
					/>
				{/snippet}
				{#snippet content()}
					<div class="text-xs">Exclude {label}</div>
				{/snippet}
			</Popover>
		{/if}
	</div>
{/snippet}

{#snippet presetTag({ name, value }: Preset)}
	<Badge onclick={() => appendFilterAsText(value)} clickable>
		{name}
	</Badge>
{/snippet}
