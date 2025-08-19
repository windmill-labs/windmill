<script lang="ts">
	import type { Tweened } from 'svelte/motion'

	import { createEventDispatcher } from 'svelte'
	import { Button } from '../common'
	import { FilterX, ListFilterPlus } from 'lucide-svelte'
	import RunOption from './RunOption.svelte'

	export let queue_count: Tweened<number> | undefined = undefined
	export let suspended_count: Tweened<number> | undefined = undefined
	export let success: string | undefined
	const dispatch = createEventDispatcher()
</script>

<RunOption label="Waiting for workers">
	{#snippet tooltip()}
		Jobs waiting for a worker being available to be executed
	{/snippet}
	<div>{queue_count ? ($queue_count ?? 0).toFixed(0) : '...'}</div>
	<div class="truncate text-2xs !text-secondary mt-0.5">
		<Button size="xs2" color="light" on:click={() => dispatch('jobs_waiting')}
			>{#if success == 'waiting'}<FilterX size={12}></FilterX>{:else}<ListFilterPlus size={14}
				></ListFilterPlus>{/if}</Button
		>
	</div>
</RunOption>

{#if suspended_count && ($suspended_count ?? 0) > 0}
	<RunOption label="Suspended">
		{#snippet tooltip()}
			Jobs waiting for an event or approval before being resumed
		{/snippet}
		<div>{suspended_count ? ($suspended_count ?? 0).toFixed(0) : '...'}</div>
		<div class="truncate text-2xs !text-secondary mt-0.5">
			<Button size="xs2" color="light" on:click={() => dispatch('jobs_suspended')}
				>{#if success == 'waiting'}<FilterX size={12}></FilterX>{:else}<ListFilterPlus size={14}
					></ListFilterPlus>{/if}</Button
			>
		</div>
	</RunOption>
{/if}
