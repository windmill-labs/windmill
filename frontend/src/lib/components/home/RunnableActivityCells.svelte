<script lang="ts">
	import { base } from '$lib/base'
	import { Tooltip } from '$lib/components/meltComponents'
	import { triggerDisplayNamesMap, triggerIconMapMono } from '$lib/components/triggers/utils'
	import { displayDate } from '$lib/utils'
	import { describeSchedule } from '$lib/utils/describeCron'
	import { twMerge } from 'tailwind-merge'
	import { getHomeActivity, type ActivityKind } from './homeActivity.svelte'

	interface Props {
		/** Undefined for rows that have neither runs nor triggers (apps): the cells
		 * still take their width so the columns line up across the list. */
		kind: ActivityKind | undefined
		path: string
	}

	let { kind, path }: Props = $props()

	const GRID_SIZE = 9
	// Past this many triggers the last visible slot becomes the `+N` badge.
	const MAX_TRIGGER_ICONS = 3

	const homeActivity = getHomeActivity()

	$effect(() => {
		if (kind) homeActivity?.request(kind, path)
	})

	let activity = $derived(kind ? homeActivity?.get(kind, path) : undefined)
	let runs = $derived(activity?.recent_runs ?? [])
	let triggers = $derived(activity?.triggers ?? [])
	let visibleTriggers = $derived(
		triggers.length > MAX_TRIGGER_ICONS ? triggers.slice(0, MAX_TRIGGER_ICONS - 1) : triggers
	)
	let hiddenTriggers = $derived(triggers.slice(visibleTriggers.length))
	// A runnable whose only trigger is a schedule shows its cadence instead of an icon.
	let loneSchedule = $derived(
		triggers.length === 1 && triggers[0].kind === 'schedule' && triggers[0].schedule
			? triggers[0]
			: undefined
	)
	let loneScheduleText = $derived(
		loneSchedule?.schedule
			? (describeSchedule(loneSchedule.schedule, loneSchedule.timezone) ?? loneSchedule.schedule)
			: undefined
	)

	type RunStatus = (typeof runs)[number]['status']

	const statusClass: Record<RunStatus, string> = {
		success: 'bg-green-500',
		failure: 'bg-red-500',
		canceled: 'bg-gray-400 dark:bg-gray-500',
		skipped: 'bg-gray-300 dark:bg-gray-600',
		running: 'bg-blue-500 animate-pulse'
	}
	const statusLabel: Record<RunStatus, string> = {
		success: 'Success',
		failure: 'Failure',
		canceled: 'Canceled',
		skipped: 'Skipped',
		running: 'Running'
	}

	function triggerLabel(kind: string): string {
		return triggerDisplayNamesMap[kind as keyof typeof triggerDisplayNamesMap] ?? kind
	}
	function triggerIcon(kind: string) {
		return triggerIconMapMono[kind as keyof typeof triggerIconMapMono]
	}
</script>

{#if homeActivity}
	<div class="relative z-[1] hidden lg:flex w-5 shrink-0 justify-center">
		{#if kind}
			<Tooltip>
				<a
					href="{base}/runs/{path}"
					class="grid grid-cols-3 gap-px p-0.5 rounded hover:bg-surface-secondary"
					aria-label="Latest runs"
				>
					{#each Array(GRID_SIZE) as _, i (i)}
						{@const run = runs[i]}
						<div
							class={twMerge(
								'w-1 h-1 rounded-[1px]',
								run ? statusClass[run.status] : 'bg-surface-secondary'
							)}
						></div>
					{/each}
				</a>
				{#snippet text()}
					{#if runs.length === 0}
						<span>No runs yet</span>
					{:else}
						<div class="flex flex-col gap-1">
							<span class="font-semibold">Latest {runs.length} runs</span>
							{#each runs as run (run.id)}
								<div class="flex items-center gap-2">
									<div class={twMerge('w-2 h-2 rounded-[1px]', statusClass[run.status])}></div>
									<span class="w-16">{statusLabel[run.status]}</span>
									<span class="text-hint">{displayDate(run.created_at, true)}</span>
								</div>
							{/each}
						</div>
					{/if}
				{/snippet}
			</Tooltip>
		{/if}
	</div>

	<div class="relative z-[1] hidden lg:flex items-center justify-start gap-0.5 w-44 min-w-0 shrink-0">
		{#if loneSchedule}
			<Tooltip class="min-w-0">
				<div
					class={twMerge(
						'h-5 px-1.5 rounded-md bg-surface-secondary text-secondary text-2xs flex items-center gap-1 min-w-0',
						loneSchedule.mode === 'enabled' ? '' : 'opacity-50'
					)}
				>
					{@render triggerIconOnly('schedule')}
					<span class="truncate">{loneScheduleText}</span>
				</div>
				{#snippet text()}
					{@render triggerLine(loneSchedule)}
				{/snippet}
			</Tooltip>
		{:else}
			{#each visibleTriggers as trigger (trigger.kind + trigger.path)}
				<Tooltip>
					{@render triggerChip(trigger.kind, trigger.mode)}
					{#snippet text()}
						{@render triggerLine(trigger)}
					{/snippet}
				</Tooltip>
			{/each}
			{#if hiddenTriggers.length > 0}
				<Tooltip>
					<div
						class="h-5 min-w-5 px-1 rounded-md bg-surface-secondary text-3xs font-semibold text-secondary center-center"
					>
						+{hiddenTriggers.length}
					</div>
					{#snippet text()}
						<div class="flex flex-col gap-1">
							<span class="font-semibold">{triggers.length} triggers</span>
							{#each triggers as trigger (trigger.kind + trigger.path)}
								{@render triggerLine(trigger)}
							{/each}
						</div>
					{/snippet}
				</Tooltip>
			{/if}
		{/if}
	</div>
{/if}

{#snippet triggerChip(kind: string, mode: string)}
	{@const Icon = triggerIcon(kind)}
	<div
		class={twMerge(
			'h-5 w-5 rounded-md bg-surface-secondary text-secondary center-center',
			mode === 'enabled' ? '' : 'opacity-50'
		)}
	>
		{#if Icon}
			<Icon size={12} />
		{:else}
			<span class="text-3xs font-semibold">{kind.slice(0, 1).toUpperCase()}</span>
		{/if}
	</div>
{/snippet}

{#snippet triggerIconOnly(kind: string)}
	{@const Icon = triggerIcon(kind)}
	{#if Icon}
		<Icon size={12} />
	{/if}
{/snippet}

{#snippet triggerLine(trigger: { kind: string; path: string; mode: string })}
	{@const Icon = triggerIcon(trigger.kind)}
	<div class="flex items-center gap-2">
		{#if Icon}
			<Icon size={12} />
		{/if}
		<span class="font-semibold">{triggerLabel(trigger.kind)}</span>
		<span>{trigger.path}</span>
		{#if trigger.mode !== 'enabled'}
			<span class="text-hint">({trigger.mode})</span>
		{/if}
	</div>
{/snippet}
