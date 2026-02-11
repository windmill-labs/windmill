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
		? { type: 'string'; value: string }
		: T extends { type: 'number' }
			? { type: 'number'; value: number }
			: T extends { type: 'boolean' }
				? { type: 'boolean'; value: boolean }
				: T extends { type: 'date' }
					? { type: 'date'; value: Date }
					: T extends { type: 'oneof'; options: infer O }
						? { type: 'oneof'; value: O extends string[] ? O[number] : never }
						: never
</script>

<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import { inputBaseClass, inputBorderClass, inputSizeClasses } from './text_input/TextInput.svelte'
	import { SearchIcon } from 'lucide-svelte'
	import { createDropdownMenu, melt } from '@melt-ui/svelte'
	import { fly } from 'svelte/transition'
	import type { IconType } from '$lib/utils'

	type Props = {
		schema: FilterSchemaRec
		value: Partial<FilterInstanceRec<FilterSchemaRec>>
		class?: string
	}

	const {
		elements: { menu, item, trigger },
		states: { open }
	} = createDropdownMenu({
		positioning: { sameWidth: true },
		closeFocus: null
	})

	let { schema, value, class: className }: Props = $props()

	function handleKeydown(e: KeyboardEvent) {
		console.log(e.currentTarget)
	}

	let inputElement: HTMLDivElement | null = null
</script>

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
		e.preventDefault() // Prevent default trigger behavior
		inputElement?.focus() // Keep focus on input
	}}
	use:melt={$trigger}
	bind:this={inputElement}
>
	<div contenteditable="true">abc</div>
	<div contenteditable="true">def</div>
	test
	<SearchIcon size="16" class="ml-auto" />
</div>

{#if $open}
	<div
		use:melt={$menu}
		transition:fly={{ y: -20, duration: 200 }}
		class="py-1 bg-surface-tertiary rounded-md shadow-lg p-2 z-[9999]"
	>
		<div class="text-xs px-2 my-2 font-bold">Filters</div>
		{#each Object.entries(schema) as [key, filterSchema]}
			{@const Icon = filterSchema.icon || SearchIcon}
			<div
				use:melt={$item}
				class="py-2 px-2 rounded-md hover:bg-surface-hover cursor-pointer text-sm"
			>
				<Icon size={16} class="mr-2 inline" />
				{filterSchema.label}
			</div>
		{/each}
	</div>
{/if}
