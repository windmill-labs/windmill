<script lang="ts" module>
	export type FilterSchemaRec = Record<string, FilterSchema>
	export type FilterSchema = (
		| {
				type: 'string' | 'number' | 'boolean' | 'date'
		  }
		| {
				type: 'oneof'
				options: { value: string; label?: string; description?: string }[]
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
</script>

<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import { inputBaseClass, inputBorderClass, inputSizeClasses } from './text_input/TextInput.svelte'
	import { SearchIcon } from 'lucide-svelte'
	import { type IconType } from '$lib/utils'
	import GenericDropdown from './select/GenericDropdown.svelte'
	import DateTimeInput from './DateTimeInput.svelte'
	import TaggedTextInput from './TaggedTextInput.svelte'
	import { transformedSync } from '$lib/svelte5Utils.svelte'

	type Props<SchemaT extends FilterSchemaRec> = {
		schema: SchemaT
		value: Partial<FilterInstanceRec<SchemaT>>
		class?: string
	}

	type SchemaT = FilterSchemaRec // TODO: Generic
	let { schema, value, class: className }: Props<SchemaT> = $props()

	let editingKey: keyof SchemaT | undefined = $state()
	let open = $state(false)
	let inputElement: HTMLDivElement | undefined = $state()

	let tags = $derived(
		Object.entries(schema).map(([key, filterSchema]) => ({
			regex: new RegExp(`\\b${key}:[^\\s]*`, 'g')
		}))
	)

	const kvRegex = /\b(\w+):([^\s]*)/g

	function parseFromText(text: string): Partial<FilterInstanceRec<SchemaT>> {
		const parsed: Record<string, string> = {}
		let match
		while ((match = kvRegex.exec(text)) !== null) {
			const [_, key, val] = match
			if (key in schema) {
				parsed[key] = val
			}
		}
		return parsed
	}

	let asText = transformedSync(
		[
			() => (Object.keys(value), value),
			(v) => {
				for (const key in value) delete value[key]
				for (const key in v) value[key] = v[key]
			}
		],
		(value) =>
			Object.entries(value)
				.map(([key, val]) => `${key}:${val}`)
				.join(' '),
		parseFromText
	)
</script>

<svelte:window on:click={() => (open = false)} />

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
		class={twMerge(
			'bg-surface-input flex justify-start gap-0.5 items-center outline-none overflow-x-auto scrollbar-hidden text-nowrap',
			inputBaseClass,
			inputBorderClass(),
			inputSizeClasses.md,
			className
		)}
	/>
</div>

<GenericDropdown
	{open}
	getInputRect={() => inputElement?.getBoundingClientRect() ?? new DOMRect()}
	innerClass="!max-h-[40rem]"
>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="py-1 p-2 overflow-y-auto" onclick={(e) => e.stopPropagation()}>
		{#if !editingKey}
			<div class="text-xs px-2 my-2 font-bold">Filters</div>
			{#each Object.entries(schema).filter(([k, _]) => !(k in value)) as [key, filterSchema]}
				{@render menuItem({
					Icon: filterSchema.icon || SearchIcon,
					onClick: () => {
						asText.val += ` ${key}:`
					},
					label: filterSchema.label || key
				})}
			{/each}
		{:else}
			{@render suggestion(schema[editingKey])}
		{/if}
	</div>
</GenericDropdown>

{#snippet suggestion(filter: FilterSchema)}
	{#if filter.description}
		<div class="text-xs text-hint px-2 my-2">{filter.description}</div>
	{/if}
	{#if filter.type === 'oneof'}
		{#each filter.options.filter((o) => !value[editingKey!] || o.value.includes(value[editingKey!] ?? '')) as option}
			{@render menuItem({
				onClick: () => {
					value[editingKey!] = option.value
				},
				label: option.label || option.value
			})}
		{/each}
	{:else if filter.type === 'date'}
		<DateTimeInput bind:value={value[editingKey!]} />
	{/if}
{/snippet}

{#snippet menuItem({
	Icon,
	onClick,
	label
}: {
	Icon?: IconType
	onClick: () => void
	label: string
})}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="py-2 px-2 rounded-md hover:bg-surface-hover cursor-pointer text-sm" onclick={onClick}>
		{#if Icon}
			<Icon size={16} class="mr-2 inline" />
		{/if}
		{label}
	</div>
{/snippet}
