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
	import { SearchIcon } from 'lucide-svelte'
	import type { IconType } from '$lib/utils'
	import GenericDropdown from './select/GenericDropdown.svelte'

	type Props = {
		schema: FilterSchemaRec
		value: Partial<FilterInstanceRec<FilterSchemaRec>>
		class?: string
	}

	let { schema, value, class: className }: Props = $props()

	function handleKeydown(
		e: KeyboardEvent & {
			currentTarget: EventTarget & HTMLDivElement
		}
	) {
		console.log(inputElement?.textContent)
	}

	let open = $state(false)
	let inputElement: HTMLDivElement | undefined = $state()
</script>

<svelte:window on:click={() => (open = false)} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	contenteditable="true"
	onkeydown={(e) => handleKeydown(e)}
	class={twMerge(
		'bg-surface-input flex gap-2 items-center outline-none',
		inputBaseClass,
		inputBorderClass(),
		inputSizeClasses.md,
		className
	)}
	onclick={(e) => {
		open = true
		e.stopPropagation()
	}}
	bind:this={inputElement}
>
	​<!-- zero-width space -->
	{#each Object.entries(value) as [key, val]}
		{@const filterSchema = schema[key]}
		{@const Icon = filterSchema.icon}
		<div
			class="flex items-center gap-1 bg-surface-hover rounded px-1 text-sm"
			contenteditable="true"
		>
			<div><Icon size={12} class="inline mr-0.5" /> {key}: &nbsp;</div>
			{val}
		</div>
	{/each}
	<SearchIcon size="16" class="ml-auto" />
</div>

<GenericDropdown {open} getInputRect={() => inputElement?.getBoundingClientRect() ?? new DOMRect()}>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div class="py-1 p-2" onclick={(e) => e.stopPropagation()}>
		<div class="text-xs px-2 my-2 font-bold">Filters</div>
		{#each Object.entries(schema) as [key, filterSchema]}
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
	</div>
</GenericDropdown>
