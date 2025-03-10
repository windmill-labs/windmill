<script lang="ts">
	import { msToSec } from '$lib/utils'
	import { base } from '$lib/base'
	import { ExternalLink } from 'lucide-svelte'
	import Popover from './Popover.svelte'

	export let position: 'center' | 'left' | 'right' = 'center'
	export let total: number
	export let min: number | undefined
	export let started_at: number | undefined
	export let len: number
	export let id: string
	export let running: boolean
	export let concat: boolean = false
	export let gray: boolean = false
</script>

{#if min && started_at != undefined}
	{#if !concat}
		<div style="width: {((started_at - min) / total) * 100}%" class="h-4"></div>
	{/if}
	<Popover
		style="width: {(len / total) * 100}%"
		class="h-4 {gray
			? 'bg-gray-300 dark:bg-gray-600'
			: running
			? 'bg-blue-400/90'
			: 'bg-blue-500/90'} {position == 'left'
			? 'rounded-l-sm'
			: position == 'right'
			? 'rounded-r-sm'
			: 'rounded-sm'} center-center text-white text-2xs whitespace-nowrap hover:outline outline-1 outline-black"
	>
		<svelte:fragment slot="text"
			><a href="{base}/run/{id}" class="inline-flex items-center gap-1" target="_blank"
				>{id} <ExternalLink size={14} /></a
			></svelte:fragment
		>
		{#if len > 0}
			<span class={len / total < 0.09 ? '-ml-14 text-primary font-mono' : 'font-mono'}
				>{#if len}{msToSec(len, 1)}s{/if}</span
			>
		{/if}
	</Popover>
{/if}
