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
	title={`Id: ${component.id}`}
	class={classNames(
		'px-2 text-2xs font-bold w-fit absolute shadow  -top-1 -left-2 border z-50',
		selected
			? 'bg-indigo-500/90 border-blue-500 text-white'
			: 'bg-gray-200/90 border-gray-300  text-gray-500'
	)}
	style="padding-top: 1px; padding-bottom: 1px;"
>
	{component.id}
</span>

{#if pointerdown || selected || hover}
	<button
		title="Position locking"
		class={classNames(
			'text-gray-800 px-1 text-2xs py-0.5 font-bold w-fit shadow border border-gray-300 absolute  -top-1  right-[2.5rem] z-50 cursor-pointer',
			' hover:bg-gray-300',
			selected ? 'bg-gray-200/80' : 'bg-gray-200/80'
		)}
		on:click={() => dispatch('lock')}
	>
		{#if locked}
			<Anchor aria-label="Unlock position" size={14} class="text-orange-500" />
		{:else}
			<Anchor aria-label="Lock position" size={14} />
		{/if}
	</button>
{/if}

{#if selected || hover}
	<span
		title="Move"
		on:mousedown|stopPropagation|capture
		class={classNames(
			'text-gray-600 px-1 text-2xs py-0.5 font-bold rounded-t-sm w-fit absolute border border-gray-300 -top-1 shadow right-[4.5rem] z-50 cursor-move',
			'bg-gray-200/80'
		)}
	>
		<Move size={14} />
	</span>
{/if}
