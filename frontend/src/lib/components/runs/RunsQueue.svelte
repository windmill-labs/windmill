

<script lang="ts">
	import type { Tweened } from 'svelte/motion'

	import { createEventDispatcher } from 'svelte'
	import { Button } from '../common'
	import { FilterX, ListFilter } from 'lucide-svelte'
	import Tooltip from '../Tooltip.svelte'

	export let queue_count: Tweened<number> | undefined = undefined
	export let suspended_count: Tweened<number> | undefined = undefined
	export let success: string | undefined
	const dispatch = createEventDispatcher()
</script>

<div class="flex gap-1 relative max-w-36 min-w-[50px] items-baseline">
	<div class="text-xs absolute -top-4 truncate flex items-baseline gap-1">Waiting for workers <Tooltip small>Jobs waiting for a worker being available to be executed</Tooltip></div>
	<div class="mt-1">{queue_count ? ($queue_count ?? 0).toFixed(0) : '...'}</div>
	<div class="truncate text-2xs  !text-secondary mt-0.5">
		{#if queue_count && ($queue_count ?? 0) > 0}
				<Button size="xs2" color="light" on:click={() => dispatch('jobs_waiting')}>{#if success == 'waiting'}<FilterX size={12}></FilterX>{:else}<ListFilter size={12}></ListFilter>{/if}</Button>
		{/if}

	</div>
</div>

{#if suspended_count && ($suspended_count ?? 0) > 0}
	<div class="flex gap-1 relative max-w-36 min-w-[50px] items-baseline ml-20 mr-8">
		<div class="text-xs absolute -top-4 truncate flex items-baseline gap-1">Suspended <Tooltip small>Jobs waiting for an event or approval before being resumed</Tooltip></div>
		<div class="mt-1">{suspended_count ? ($suspended_count ?? 0).toFixed(0) : '...'}</div>
		<div class="truncate text-2xs !text-secondary mt-0.5">
					<Button size="xs2" color="light" on:click={() => dispatch('jobs_suspended')}>{#if success == 'waiting'}<FilterX size={12}></FilterX>{:else}<ListFilter size={12}></ListFilter>{/if}</Button>
		</div>
	</div>
{/if}
