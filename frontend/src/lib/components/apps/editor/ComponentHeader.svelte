<script lang="ts">
	import { classNames } from '$lib/utils'
	import type { AppComponent } from '../types'
	import { Anchor, Move } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'

	export let component: AppComponent
	export let selected: boolean
	export let locked: boolean = false
	export let pointerdown: boolean = false
	export let hover: boolean = false

	const dispatch = createEventDispatcher()
</script>

<span
	class={classNames(
		'px-2 text-2xs font-bold w-fit absolute z-50',
		selected ? 'bg-indigo-500/90 text-white' : 'bg-gray-200/60 text-gray-500'
	)}
	style="padding-top: 1px; padding-bottom: 1px;"
>
	{component.id}
</span>

{#if pointerdown || selected || hover}
	<button
		class={classNames(
			'text-gray-800 px-1 text-2xs py-0.5 font-bold w-fit absolute right-1 top-1 z-50 cursor-pointer',
			' hover:bg-gray-300',
			selected ? 'bg-gray-200/80' : 'bg-gray-200/60'
		)}
		on:click={() => dispatch('lock')}
	>
		{#if locked}
			<Anchor size={14} class="text-orange-500" />
		{:else}
			<Anchor size={14} />
		{/if}
	</button>
{/if}

{#if selected || hover}
	<span
		on:mousedown|stopPropagation|capture
		class={classNames(
			'text-gray-600 px-1 text-2xs py-0.5 font-bold w-fit absolute right-7 top-1 z-50 cursor-move',
			'bg-gray-200/60'
		)}
	>
		<Move size={14} />
	</span>
{/if}
