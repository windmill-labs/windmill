<script lang="ts">
	import { msToSec } from '$lib/utils'
	import { ExternalLink } from 'lucide-svelte'
	import Popover from './Popover.svelte'

	export let total: number
	export let min: number | undefined
	export let started_at: number | undefined
	export let len: number
	export let id: string
	export let running: boolean
</script>

{#if min && started_at}
	<div class="flex w-full">
		<div style="width: {((started_at - min) / total) * 100}%" class="h-4" />
		<Popover
			style="width: {(len / total) * 100}%"
			class="h-4 {running
				? 'bg-blue-400/90'
				: 'bg-blue-500/90'} rounded-sm center-center text-white text-2xs whitespace-nowrap hover:outline outline-1 outline-black"
		>
			<svelte:fragment slot="text"
				><a href="/run/{id}" class="inline-flex items-center gap-1" target="_blank"
					>{id} <ExternalLink size={14} /></a
				></svelte:fragment
			>
			<span class={len / total < 0.09 ? '-ml-14 text-primary' : ''}
				>{#if len}{msToSec(len, 1)}s{/if}</span
			>
		</Popover>
	</div>
{/if}
