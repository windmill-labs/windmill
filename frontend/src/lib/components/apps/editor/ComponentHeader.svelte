<script lang="ts">
	import { classNames } from '$lib/utils'
	import type { AppComponent } from '../types'
	import { Anchor, Lock, Move, X } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'

	export let component: AppComponent
	export let selected: boolean
	export let locked: boolean = false
	export let pointerdown: boolean = false

	const dispatch = createEventDispatcher()
</script>

<span
	class={classNames(
		'px-2 text-2xs py-0.5 font-bold rounded-t-sm w-fit absolute z-50',
		selected ? 'bg-indigo-500/90 text-white' : 'bg-gray-200/60 text-gray-500'
	)}
>
	{component.id}
</span>

{#if pointerdown || selected}
	<button
		class={classNames(
			'text-white px-1 text-2xs py-0.5 font-bold rounded-t-sm w-fit absolute  right-10 z-50 cursor-pointer',
			' hover:bg-gray-800',
			selected ? 'bg-gray-600/90' : 'bg-gray-600/80'
		)}
		on:click={() => {
			dispatch('lock')
		}}
	>
		{#if locked}
			<Anchor size={16} class="text-orange-500" />
		{:else}
			<Anchor size={16} />
		{/if}
	</button>
{/if}

{#if selected}
	<span
		on:mousedown|stopPropagation|capture
		class={classNames(
			'text-white px-1 text-2xs py-0.5 font-bold rounded-t-sm w-fit absolute  right-20 z-50 cursor-move',
			'bg-gray-600/80'
		)}><Move size={16} /></span
	>
{/if}
