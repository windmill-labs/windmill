<script lang="ts" module>
	export type FilterSchemaRec = Record<string, FilterSchema>
	export type FilterSchema = (
		| {
				type: 'string' | 'number' | 'boolean' | 'date'
		  }
		| {
				type: 'oneof'
				options: string[]
				allowCustomValue?: boolean
		  }
	) & {
		label?: string
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
	import { SearchIcon, XIcon } from 'lucide-svelte'
	import { wait, type IconType } from '$lib/utils'
	import GenericDropdown from './select/GenericDropdown.svelte'

	type Props<SchemaT extends FilterSchemaRec> = {
		schema: SchemaT
		value: Partial<FilterInstanceRec<SchemaT>>
		class?: string
	}

	type SchemaT = FilterSchemaRec // TODO: Generic
	let { schema, value, class: className }: Props<SchemaT> = $props()

	let editingKey: keyof SchemaT | undefined = $state()
	async function updateEditingKeyOnCursorMoved() {
		await wait(0)
		if (!inputElement) return

		const selection = window.getSelection()
		if (!selection || selection.rangeCount === 0) {
			editingKey = undefined
			return
		}

		const range = selection.getRangeAt(0)
		const currentNode = range.startContainer

		// Find the parent .usercontent element if we're inside one
		let targetElement: HTMLElement | null =
			currentNode instanceof HTMLElement ? currentNode : currentNode.parentElement

		while (targetElement && targetElement !== inputElement) {
			if (targetElement.classList?.contains('usercontent')) {
				// Extract the key from the content
				const text = targetElement.textContent?.trim() || ''
				const colonIndex = text.indexOf(':')
				if (colonIndex > 0) {
					const key = text.slice(0, colonIndex).trim()
					editingKey = key as keyof SchemaT
					return
				}
			}
			targetElement = targetElement.parentElement
		}

		// Cursor is not inside any filter key element
		editingKey = undefined
	}

	async function onInputChanged() {
		await wait(0)
		if (!inputElement) return
		const allEntries: { key: string; rawValue: string }[] = []
		for (let i = 0; i < inputElement.childNodes.length; i++) {
			const child = inputElement.childNodes[i] as HTMLDivElement
			if (child?.classList?.contains?.('searchicon')) continue
			if (!child || !child?.classList?.contains?.('usercontent')) {
				continue
			}
			const str = child.textContent.trim()
			const idx = str.indexOf(':')
			const key = str.slice(0, idx).trim()
			const rawValue = str.slice(idx + 1).trim()
			allEntries.push({ key, rawValue })
		}

		// Delete all keys that are not present in the input anymore
		for (const key in value) {
			if (!allEntries.find((e) => e.key === key)) {
				delete value[key]
			}
		}

		await updateEditingKeyOnCursorMoved()
	}

	let open = $state(false)
	let inputElement: HTMLDivElement | undefined = $state()

	function clearValue() {
		for (const key in value) delete value[key]
	}
</script>

<svelte:window on:click={() => (open = false)} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="relative">
	<div
		contenteditable="true"
		onkeydown={(e) => onInputChanged()}
		onclick={(e) => {
			open = true
			e.stopPropagation()
			updateEditingKeyOnCursorMoved()
		}}
		class={twMerge(
			'bg-surface-input flex justify-start gap-0.5 items-center outline-none overflow-x-auto scrollbar-hidden !pr-8',
			inputBaseClass,
			inputBorderClass(),
			inputSizeClasses.md,
			className
		)}
		bind:this={inputElement}
	>
		&nbsp;
		{#each Object.entries(value) as [key, val]}
			{@const filterSchema = schema[key]}
			{@const Icon = filterSchema.icon}
			<div
				class="usercontent flex items-center bg-surface-sunken rounded px-1 text-sm flex-nowrap text-nowrap"
				contenteditable="true"
			>
				<Icon size={12} class="inline" /><div contenteditable="false">{key}:</div>
				&nbsp;{val}
			</div>
			&nbsp;
		{/each}
	</div>
	<div
		class="px-2 top-0.5 bottom-0.5 rounded-r-md center-center bg-surface-input right-0.5 absolute"
	>
		{#if Object.keys(value).length === 0}
			<SearchIcon size="16" class="searchicon" />
		{:else}
			<XIcon size="16" class="searchicon cursor-pointer" onclick={() => clearValue()} />
		{/if}
	</div>
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
				{@const Icon = filterSchema.icon || SearchIcon}
				<div
					class="py-2 px-2 rounded-md hover:bg-surface-hover cursor-pointer text-sm"
					onclick={() => {
						value[key] = ''
					}}
				>
					<Icon size={16} class="mr-2 inline" />
					{filterSchema.label}
				</div>
			{/each}
		{:else}
			{@render suggestion(schema[editingKey])}
		{/if}
	</div>
</GenericDropdown>

{#snippet suggestion(filter: FilterSchema)}
	{#if filter.type === 'oneof'}
		{#each filter.options as option}
			<div>{option}</div>
		{/each}
	{/if}
{/snippet}
