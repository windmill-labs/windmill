<script lang="ts">
	import { msToSec } from '$lib/utils'
	import { base } from '$lib/base'
	import { ExternalLink } from 'lucide-svelte'
	import Popover from './Popover.svelte'

	interface Props {
		position?: 'center' | 'left' | 'right'
		total: number
		min: number | undefined
		started_at: number | undefined
		len: number
		id: string
		running: boolean
		concat?: boolean
		gray?: boolean
		spacerClass?: string
	}

	let {
		position = 'center',
		total,
		min,
		started_at,
		len,
		id,
		running,
		concat = false,
		gray = false,
		spacerClass = ''
	}: Props = $props()
</script>

{#if min && started_at != undefined}
	{#if !concat}
		<div style="width: {((started_at - min) / total) * 100}%" class="h-5 {spacerClass}"></div>
	{/if}
	<Popover
		style="width: {(len / total) * 100}%"
		class="h-5 {gray
			? 'bg-gray-300 dark:bg-gray-600'
			: running
				? 'bg-blue-400/90'
				: 'bg-blue-500/90'} {position == 'left'
			? 'rounded-l-md'
			: position == 'right'
				? 'rounded-r-md'
				: 'rounded-md'} center-center text-white text-2xs whitespace-nowrap hover:outline outline-1 outline-black"
	>
		{#snippet text()}
			<a href="{base}/run/{id}" class="inline-flex items-center gap-1" target="_blank"
				>{id} <ExternalLink size={14} /></a
			>
		{/snippet}
		{#if len > 0}
			<span class={len / total < 0.09 ? '-ml-14 text-primary font-mono' : 'font-mono'}
				>{#if len}{msToSec(len, 1)}s{/if}</span
			>
		{/if}
	</Popover>
{/if}
