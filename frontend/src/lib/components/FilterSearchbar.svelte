<script lang="ts" module>
	export type FilterSchemaRec = Record<string, FilterSchema>
	export type FilterSchema = (
		| {
				type: 'string' | 'number' | 'boolean' | 'date'
		  }
		| {
				type: 'oneof'
				options: readonly { value: string; label?: string; description?: string }[]
				allowCustomValue?: boolean
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
					: T extends { type: 'oneof'; options: infer O; allowCustomValue?: infer A }
						? A extends true
							? string
							: O extends readonly string[]
								? O[number]
								: never
						: never

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
			if (schema.allowCustomValue) return text
			return schema.options.find((o) => o.value === text)?.value ?? null
		}
		return null
	}
</script>

<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import { inputBaseClass, inputBorderClass, inputSizeClasses } from './text_input/TextInput.svelte'
	import { SearchIcon } from 'lucide-svelte'
	import { assignObjInPlace, formatDatePretty, parsePrettyDate, type IconType } from '$lib/utils'
	import GenericDropdown from './select/GenericDropdown.svelte'
	import DateTimeInput from './DateTimeInput.svelte'
	import TaggedTextInput from './TaggedTextInput.svelte'
	import { useTransformedSyncedValue } from '$lib/svelte5Utils.svelte'

	type Props<SchemaT extends FilterSchemaRec> = {
		schema: SchemaT
		value: Partial<FilterInstanceRec<SchemaT>>
		class?: string
	}

	type SchemaT = FilterSchemaRec // TODO: Generic
	let { schema, value = $bindable(), class: className }: Props<SchemaT> = $props()

	let currentTag: keyof SchemaT | undefined = $state()
	let open = $state(false)
	let inputElement: HTMLDivElement | undefined = $state()
	let highlightedIndex = $state(0)

	let tags = $derived(
		Object.entries(schema).map(([key, filterSchema]) => ({
			regex: new RegExp(`\\b${key}:(?:\\\\.|[^\\s])*`, 'g'),
			id: key
		}))
	)

	let menuItems = $derived.by(() => {
		if (!currentTag) {
			return Object.entries(schema)
				.filter(([k, _]) => !(k in value))
				.map(([key, filterSchema]) => ({
					type: 'filter' as const,
					key,
					filterSchema,
					onClick: () => {
						asText.val.replaceAll('\u00A0', ' ')
						if (!asText.val.endsWith(' ')) asText.val += ' '
						asText.val += `${key}:\\ `
					}
				}))
		} else {
			const filter = schema[currentTag]
			if (filter.type === 'oneof') {
				return filter.options
					.filter((o) => !value[currentTag!] || o.value.includes(String(value[currentTag!] ?? '')))
					.map((option) => ({
						type: 'option' as const,
						option,
						onClick: () => setValueForCurrentTag(option.value)
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
				val = val.replace(/\\(.)/g, '$1') // Unescape escaped characters
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
					`${key}: ${filterToText(val as any, schema[key])}`.replace(/ /g, '\\ ')
				)
				.join(' ') + '\u00A0'
		)
	}

	let asText = useTransformedSyncedValue(
		[() => (Object.entries(value), value), (v) => assignObjInPlace(value, v)],
		parseToText,
		parseFromText
	)

	function onTagComplete() {
		asText.val = parseToText(value)
	}

	function setValueForCurrentTag(val: any) {
		value[currentTag!] = val
		onTagComplete()
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
			if (menuItems[highlightedIndex]) menuItems[highlightedIndex].onClick()
			else setValueForCurrentTag(value[currentTag!])
		} else {
			return
		}
		e.preventDefault()
	}
</script>

<svelte:window on:click={() => (open = false)} onkeydown={handleKeyDown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
	class="relative"
	onclick={(e) => {
		open = true
		e.stopPropagation()
	}}
	bind:this={inputElement}
>
	<TaggedTextInput
		bind:value={asText.val}
		{tags}
		onCurrentTagChange={(tag) => (currentTag = tag ? (tag.id as keyof SchemaT) : undefined)}
		class={twMerge(
			'bg-surface-input flex justify-start gap-0.5 items-center outline-none overflow-x-auto scrollbar-hidden text-nowrap',
			inputBaseClass,
			inputBorderClass(),
			inputSizeClasses.md,
			className
		)}
		placeholder="Filter runs..."
	/>
</div>

<GenericDropdown
	{open}
	getInputRect={() => inputElement?.getBoundingClientRect() ?? new DOMRect()}
	innerClass="!max-h-[30rem]"
	strictWidth
>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="py-1 p-2 overflow-y-auto" onclick={(e) => e.stopPropagation()}>
		{#if !currentTag}
			<div class="text-xs px-2 my-2 font-bold">Filters</div>
			{#each menuItems as item, index}
				{#if item.type === 'filter'}
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
			{@render suggestion(schema[currentTag])}
		{/if}
	</div>
</GenericDropdown>

{#snippet suggestion(filter: FilterSchema)}
	{#if filter.description}
		<div class="text-xs text-hint px-2 my-2">{filter.description}</div>
	{/if}
	{#if filter.type === 'oneof'}
		<div class="max-h-60 overflow-y-auto">
			{#each menuItems as item, index}
				{#if item.type === 'option'}
					{@render menuItem({
						onClick: item.onClick,
						label: item.option.label || item.option.value,
						highlighted: index === highlightedIndex
					})}
				{/if}
			{/each}
		</div>
	{:else if filter.type === 'boolean'}
		{#each menuItems as item, index}
			{#if item.type === 'boolean'}
				{@render menuItem({
					onClick: item.onClick,
					label: item.label,
					highlighted: index === highlightedIndex
				})}
			{/if}
		{/each}
	{:else if filter.type === 'date'}
		<DateTimeInput
			bind:value={() => value[currentTag!], (d) => d && setValueForCurrentTag(new Date(d))}
		/>
	{/if}
{/snippet}

{#snippet menuItem({
	Icon,
	onClick,
	label,
	description,
	highlighted = false
}: {
	Icon?: IconType
	onClick: () => void
	label: string
	description?: string
	highlighted?: boolean
})}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		class={twMerge(
			'py-1.5 px-2 rounded-md hover:bg-surface-hover cursor-pointer text-sm flex items-center',
			highlighted && 'bg-surface-hover'
		)}
		onclick={onClick}
	>
		{#if Icon}
			<Icon size={16} class="mr-3 inline" />
		{/if}
		<div class="inline">
			<span class="text-sm">{label}</span>
			{#if description}
				<div class="text-xs text-hint">{description}</div>
			{/if}
		</div>
	</div>
{/snippet}
