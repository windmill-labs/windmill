<script lang="ts">
	import type { Tweened } from 'svelte/motion'

	import { createEventDispatcher } from 'svelte'
	import { Button } from '../common'
	import { FilterX, Hourglass, ListFilterPlus } from 'lucide-svelte'
	import RunOption from './RunOption.svelte'
	import { Popover } from '../meltComponents'

	interface Props {
		queue_count?: Tweened<number> | undefined
		suspended_count?: Tweened<number> | undefined
		success: string | undefined
		small?: boolean
	}

	let {
		queue_count = undefined,
		suspended_count = undefined,
		success,
		small = false
	}: Props = $props()
	const dispatch = createEventDispatcher()
</script>

{#if small}
	<Popover contentClasses="p-4">
		{#snippet trigger()}
			<div class="relative">
				<Hourglass size={16} />
				<div
					class="absolute top-0 right-0 translate-x-1/2 -translate-y-1/2 bg-purple-500 rounded-full text-white text-xs h-4 w-4"
				>
					{queue_count && suspended_count
						? (($queue_count ?? 0) + ($suspended_count ?? 0)).toFixed(0)
						: '...'}
				</div>
			</div>
		{/snippet}
		{#snippet content()}
			{@render queuedContent()}
		{/snippet}
	</Popover>
{:else}
	{@render queuedContent()}
{/if}

{#snippet queuedContent()}
	<RunOption label="Waiting for workers">
		{#snippet tooltip()}
			Jobs waiting for a worker being available to be executed
		{/snippet}
		<div
			class={queue_count && ($queue_count ?? 0) > 0
				? 'bg-purple-500 text-white rounded-full w-5 h-5'
				: ''}>{queue_count ? ($queue_count ?? 0).toFixed(0) : '...'}</div
		>
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
			<div
				class={suspended_count && ($suspended_count ?? 0) > 0
					? 'bg-purple-500 text-white rounded-full w-6 h-6 flex center-center'
					: ''}>{suspended_count ? ($suspended_count ?? 0).toFixed(0) : '...'}</div
			>
			<div class="truncate text-2xs !text-secondary mt-0.5">
				<Button size="xs2" color="light" on:click={() => dispatch('jobs_suspended')}
					>{#if success == 'waiting'}<FilterX size={12}></FilterX>{:else}<ListFilterPlus size={14}
						></ListFilterPlus>{/if}</Button
				>
			</div>
		</RunOption>
	{/if}
{/snippet}
