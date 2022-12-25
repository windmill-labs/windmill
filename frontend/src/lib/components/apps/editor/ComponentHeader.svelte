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
		'text-white px-2 text-2xs py-0.5 font-bold rounded-t-sm w-fit absolute -top-5',
		selected ? 'bg-indigo-500' : 'bg-gray-500'
	)}
>
	{component.id}
</span>

<button
	class={classNames(
		'text-white px-1 text-2xs py-0.5 font-bold rounded-t-sm w-fit absolute -top-5 right-8  cursor-pointer',
		'bg-gray-600 hover:bg-gray-800'
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

<button
	class={classNames(
		'text-white px-1 text-2xs py-0.5 font-bold rounded-t-sm w-fit absolute -top-5 right-0  cursor-pointer',
		'bg-gray-600 hover:bg-gray-800'
	)}
	on:click={() => {
		dispatch('delete')
	}}
>
	<X size={16} />
</button>
