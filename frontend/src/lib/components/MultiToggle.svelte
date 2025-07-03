<!-- A component similar to Toggle but with more than two values -->
<script lang="ts" generics="T">
	import { twMerge } from 'tailwind-merge'
	import { getLabel } from './select/utils.svelte'
	import { fade } from 'svelte/transition'

	let {
		items,
		value = $bindable(),
		class: className,
		error,
		disabled,
		clearable = false
	}: {
		items: { label?: string; value: T }[]
		value: T
		class?: string
		error?: boolean
		disabled?: boolean
		clearable?: boolean
	} = $props()

	let btnRefs = $state<Record<number, HTMLButtonElement | undefined>>({})
	let btnWidths: { w: number; offsetX: number }[] = $state([])
	$effect(() => {
		btnRefs
		for (const item of items) {
			item.label
			item.value
		}
		setTimeout(() => {
			btnWidths = []
			for (let i = 0; i in btnRefs; i++) {
				btnWidths.push({
					w: btnRefs[i]?.getBoundingClientRect().width ?? 0,
					offsetX: btnRefs[i]?.offsetLeft ?? 0
				})
			}
		}, 0)
	})
	let selectedIdx = $derived(items.findIndex((item) => item.value === value))
	let selectedBtnBox: (typeof btnWidths)[number] | undefined = $derived(btnWidths[selectedIdx])
</script>

<ul
	class={twMerge(
		'flex text-nowrap bg-gray-300 rounded-full items-center border relative transition-colors',
		error ? 'border-red-300 bg-red-100' : '',
		className
	)}
>
	<!-- White animated selector -->
	{#if selectedBtnBox}
		<div
			class={twMerge('h-[22px] bg-white rounded-full absolute transition-all')}
			transition:fade={{ duration: 120 }}
			style={`width: ${selectedBtnBox?.w ?? 22}px; left: ${selectedBtnBox?.offsetX ?? 0}px;`}
		></div>
	{/if}

	{#each items as item, i}
		<li class="h-6 text-xs flex items-center">
			<button
				bind:this={btnRefs[i]}
				class="z-10 rounded-full px-1.5 h-6 min-w-6 flex items-center justify-center"
				{disabled}
				onclick={() => {
					if (disabled) return
					if (clearable && value === item.value) {
						value = undefined as any
					} else {
						value = item.value
					}
				}}
			>
				{getLabel(item)}
			</button>
		</li>
	{/each}
</ul>
