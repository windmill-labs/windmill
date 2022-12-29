<script lang="ts">
	import { classNames } from '$lib/utils'
	import type { AppComponent } from '../types'
	import { Anchor, Lock, X } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'

	export let component: AppComponent
	export let selected: boolean
	export let locked: boolean = false

	const dispatch = createEventDispatcher()
</script>

<span
	class={classNames(
		'px-2 text-2xs py-0.5 font-bold rounded-t-sm w-fit absolute z-50',
		selected ? 'bg-indigo-500 text-white' : 'bg-gray-200/60 text-gray-500'
	)}
>
	{component.id}
</span>

<button
	class={classNames(
		'text-white px-1 text-2xs py-0.5 font-bold rounded-t-sm w-fit absolute right-8 z-50 cursor-pointer',
		' hover:bg-gray-800',
		selected ? 'bg-gray-600/90' : 'bg-gray-600/60'
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
{#if selected}
	<button
		class={classNames(
			'text-white px-1 text-2xs py-0.5 font-bold rounded-t-sm w-fit absolute right-0 z-50 cursor-pointer',
			'bg-gray-600/80 hover:bg-gray-800'
		)}
		on:click={() => {
			dispatch('delete')
		}}
	>
		<X size={16} />
	</button>
{/if}
