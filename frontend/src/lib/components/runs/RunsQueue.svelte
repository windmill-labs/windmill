<script lang="ts">
	import type { Tweened } from 'svelte/motion'
	import { Button } from '../common'
	import { Bot, Hourglass, ListFilterPlus, X } from 'lucide-svelte'
	import RunOption from './RunOption.svelte'
	import { Popover } from '../meltComponents'

	interface Props {
		queue_count?: Tweened<number> | undefined
		suspended_count?: Tweened<number> | undefined
		success: string | null
		small?: boolean
		onJobsWaiting?: () => void
		onJobsSuspended?: () => void
	}

	let {
		queue_count = undefined,
		suspended_count = undefined,
		success,
		small = false,
		onJobsWaiting,
		onJobsSuspended
	}: Props = $props()
</script>

<div class="flex flex-row gap-4 items-center">
	{#if small}
		<Popover contentClasses="p-4" openOnHover debounceDelay={100}>
			{#snippet trigger()}
				<div class="relative">
					<Bot size={16} />
					{#if queue_count && ($queue_count ?? 0) > 0}
						<div
							class="absolute top-0 right-0 translate-x-1/2 -translate-y-1/2 bg-yellow-500 rounded-full text-white text-2xs h-4 min-w-4 px-1"
						>
							{queue_count ? ($queue_count ?? 0).toFixed(0) : '...'}
						</div>
					{/if}
				</div>
			{/snippet}

			{#snippet content()}
				{@render queuedContent()}
			{/snippet}
		</Popover>
	{:else}
		{@render queuedContent()}
	{/if}

	{#if small}
		<Popover
			contentClasses="p-4"
			openOnHover
			debounceDelay={100}
			disablePopup={!suspended_count || ($suspended_count ?? 0) <= 0}
		>
			{#snippet trigger()}
				<div class="relative">
					<Hourglass size={16} />
					<div
						class="absolute top-0 right-0 translate-x-1/2 -translate-y-1/2 bg-surface-secondary-inverse rounded-full text-primary-inverse text-2xs h-4 min-w-4 px-1"
					>
						{suspended_count ? ($suspended_count ?? 0).toFixed(0) : '...'}
					</div>
				</div>
			{/snippet}
			{#snippet content()}
				{@render suspendedContent()}
			{/snippet}
		</Popover>
	{:else}
		{@render suspendedContent()}
	{/if}
</div>

{#snippet queuedContent()}
	<RunOption label="Waiting for workers">
		{#snippet tooltip()}
			Jobs waiting for a worker being available to be executed
		{/snippet}
		<button
			class={queue_count && ($queue_count ?? 0) > 0
				? 'bg-yellow-500 text-white rounded-full min-w-6 h-6 flex center-center'
				: ''}
			onclick={() => onJobsWaiting?.()}
		>
			{queue_count ? ($queue_count ?? 0).toFixed(0) : '...'}
		</button>
		<div class="truncate text-2xs !text-secondary mt-0.5">
			<Button variant="subtle" unifiedSize="md" on:click={() => onJobsWaiting?.()}>
				{#if success == 'waiting'}
					<div class="flex flex-row items-center gap-1">
						Reset filter
						<X size={12} />
					</div>
				{:else}
					<ListFilterPlus size={14} />
				{/if}
			</Button>
		</div>
	</RunOption>
{/snippet}

{#snippet suspendedContent()}
	{#if suspended_count && ($suspended_count ?? 0) > 0}
		<RunOption label="Suspended">
			{#snippet tooltip()}
				Jobs waiting for an event or approval before being resumed
			{/snippet}
			<div
				class={suspended_count && ($suspended_count ?? 0) > 0
					? 'bg-surface-secondary-inverse text-primary-inverse rounded-full min-w-6 h-6 flex center-center'
					: ''}>{suspended_count ? ($suspended_count ?? 0).toFixed(0) : '...'}</div
			>
			<div class="truncate text-2xs !text-secondary">
				<Button unifiedSize="md" variant="subtle" on:click={() => onJobsSuspended?.()}>
					{#if success == 'suspended'}
						<div class="flex flex-row items-center gap-1">
							Reset filter
							<X size={12} />
						</div>
					{:else}
						<ListFilterPlus size={14} />
					{/if}
				</Button>
			</div>
		</RunOption>
	{/if}
{/snippet}
