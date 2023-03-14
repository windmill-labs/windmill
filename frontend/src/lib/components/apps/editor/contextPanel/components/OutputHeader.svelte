<script lang="ts">
	import { classNames } from '$lib/utils'
	import { ChevronDown, ChevronUp } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import { slide } from 'svelte/transition'

	export let id: string
	export let name: string
	export let first: boolean = false
	export let nested: boolean = false
	export let color: 'blue' | 'indigo' = 'indigo'
	export let expanded: boolean = false
	export let shouldOpen: boolean = false

	$: open = shouldOpen
	let manuallyOpen = false

	const dispatch = createEventDispatcher()

	$: if (expanded) {
		manuallyOpen = true
	} else {
		manuallyOpen = false
	}

	const hoverColor = {
		blue: 'hover:bg-blue-300 hover:text-blue-600',
		indigo: 'hover:bg-indigo-300 hover:text-indigo-600'
	}

	const openBackground = {
		blue: 'bg-blue-50',
		indigo: 'bg-indigo-50'
	}

	const manuallyOpenColor = {
		blue: 'text-blue-600',
		indigo: 'text-indigo-600'
	}

	const idClass = {
		blue: 'bg-blue-500 text-white',
		indigo: 'bg-indigo-500 text-white'
	}
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div
	class={classNames(
		'flex items-center justify-between p-1 cursor-pointer border-b',
		hoverColor[color],
		open && !manuallyOpen ? openBackground[color] : 'bg-white',
		first ? 'border-t' : '',
		nested ? 'border-l' : ''
	)}
	on:click={() => {
		dispatch('handleClick', { manuallyOpen })
		manuallyOpen = !manuallyOpen
	}}
>
	<div
		class={classNames(
			'text-2xs ml-0.5 font-bold px-2 py-0.5 rounded-sm',
			open ? idClass[color] : ' bg-gray-100'
		)}
	>
		{id}
	</div>
	<div class="text-2xs font-bold flex flex-row gap-2 items-center">
		{name}
		{#if !open && !manuallyOpen}
			<ChevronDown size={14} />
		{:else if manuallyOpen}
			<ChevronUp size={14} class={manuallyOpenColor[color]} strokeWidth={4} />
		{:else}
			<ChevronUp size={14} />
		{/if}
	</div>
</div>
{#if open || manuallyOpen}
	<div class="py-1 border-b" transition:slide|local>
		<div class={classNames(nested ? 'border-l ml-2' : '')}>
			<slot />
		</div>
	</div>
{/if}
