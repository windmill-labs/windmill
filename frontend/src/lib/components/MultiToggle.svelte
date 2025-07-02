<!-- A component similar to Toggle but with more than two values -->
<script lang="ts" generics="T">
	import { twMerge } from 'tailwind-merge'
	import { getLabel } from './select/utils.svelte'

	let {
		items,
		value = $bindable(),
		class: className,
		error,
		disabled
	}: {
		items: { label?: string; value: T }[]
		value: T
		class?: string
		error?: boolean
		disabled?: boolean
	} = $props()
</script>

<ul
	class={twMerge(
		'flex text-nowrap bg-gray-300 rounded-full items-center select-none border',
		error ? 'border-red-300 bg-red-100' : '',
		className
	)}
>
	{#each items as item}
		<li class="h-6 mx-[1px] text-xs flex items-center">
			<button
				class={twMerge(
					'rounded-full px-1.5 h-[22px] min-w-[22px] flex items-center justify-center',
					value === item.value ? 'bg-white' : ''
				)}
				{disabled}
				onclick={() => !disabled && (value = item.value)}
			>
				{getLabel(item)}
			</button>
		</li>
	{/each}
</ul>
