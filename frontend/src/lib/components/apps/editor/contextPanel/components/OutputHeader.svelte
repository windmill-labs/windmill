<script lang="ts">
	import { classNames } from '$lib/utils'
	import { ChevronRight, ChevronDown } from 'lucide-svelte'

	export let open: boolean = false
	export let manuallyOpen: boolean = open
	export let id: string
	export let name: string
	export let first: boolean = false
	export let nested: boolean = false
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div
	class={classNames(
		'flex items-center justify-between p-1 cursor-pointer hover:bg-indigo-100 hover:text-indigo-500 border-b',
		open ? 'bg-indigo-200' : 'bg-white',
		first ? 'border-t' : '',
		nested ? 'border-l' : ''
	)}
	on:click
>
	<div
		class={classNames(
			'text-2xs ml-0.5 font-bold px-2 py-0.5 rounded-sm',
			open ? 'bg-indigo-500 text-white' : ' bg-indigo-50'
		)}
	>
		{id}
	</div>
	<div class="text-2xs font-bold flex flex-row gap-2 items-center">
		{name}
		{#if !open && !manuallyOpen}
			<ChevronRight size={14} />
		{:else if manuallyOpen}
			<ChevronDown size={14} class="text-indigo-600" strokeWidth={4} />
		{:else}
			<ChevronDown size={14} />
		{/if}
	</div>
</div>
