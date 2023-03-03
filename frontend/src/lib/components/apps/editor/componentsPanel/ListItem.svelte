<script lang="ts">
	import { slide } from 'svelte/transition'
	import { ChevronDown } from 'lucide-svelte'
	import { isOpenStore } from './store'

	export let title: string
	export let prefix: string | undefined = undefined

	$: storeTitle = prefix + title
	$: isOpen = prefix ? $isOpenStore[storeTitle] : true
</script>

<section class="pt-1 pb-2 px-1">
	{#if prefix !== undefined}
		<button
			on:click|preventDefault={() => isOpenStore.toggle(storeTitle)}
			class="w-full flex justify-between items-center text-gray-700 px-2 py-1 
			rounded-sm duration-200 hover:bg-gray-100"
		>
			<h1 class="text-sm font-semibold text-left">
				<slot name="title">
					{title}
				</slot>
			</h1>
			<ChevronDown class="rotate-0 duration-300 {isOpen ? '!rotate-180' : ''}" />
		</button>
		{#if isOpen}
			<div transition:slide|local={{ duration: 300 }} class="px-2">
				<slot />
			</div>
		{/if}
	{:else}
		<h1 class="text-sm font-semibold text-left px-2 py-1">
			<slot name="title">
				{title}
			</slot>
		</h1>
		<div class="px-2">
			<slot />
		</div>
	{/if}
</section>
