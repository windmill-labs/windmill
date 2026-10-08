<script lang="ts">
	import { base } from '$lib/base'
	import { Tooltip } from '$lib/components/meltComponents'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import { triggerDisplayNamesMap, triggerIconMapMono } from '$lib/components/triggers/utils'
	import { displayDate, formatCron } from '$lib/utils'
	import { describeSchedule } from '$lib/utils/describeCron'
	import { twMerge } from 'tailwind-merge'
	import { ExternalLink } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { workspaceStore } from '$lib/stores'
	import { getHomeActivity, type ActivityKind } from './homeActivity.svelte'

	interface Props {
		/** Undefined for rows that have neither runs nor triggers (apps): the cells
		 * still take their width so the columns line up across the list. */
		kind: ActivityKind | undefined
		path: string
	}

	let { kind, path }: Props = $props()

	// Past this many triggers the rest collapse into a `+N` that lists them all on hover.
	const MAX_TRIGGERS_SHOWN = 2

	const homeActivity = getHomeActivity()

	$effect(() => {
		if (kind) homeActivity?.request(kind, path)
	})

	let activity = $derived(kind ? homeActivity?.get(kind, path) : undefined)
	let runs = $derived(activity?.recent_runs ?? [])
	let triggers = $derived(activity?.triggers ?? [])
	let lastRun = $derived(runs[0])
	let visibleTriggers = $derived(
		triggers.length > MAX_TRIGGERS_SHOWN + 1 ? triggers.slice(0, MAX_TRIGGERS_SHOWN) : triggers
	)
	let hiddenCount = $derived(triggers.length - visibleTriggers.length)

	type Trigger = (typeof triggers)[number]

	function triggerText(trigger: Trigger): string {
		if (trigger.kind === 'schedule' && trigger.schedule) {
			return (
				describeScheduleOf(trigger.schedule, trigger.timezone, trigger.cron_version) ??
				trigger.schedule
			)
		}
		if (trigger.kind === 'http') return 'HTTP route'
		return triggerLabel(trigger.kind)
	}

	// `describeCron` reads weekdays the v2 way (0 = Sunday); a v1 cron numbers them
	// from 1 = Sunday, so one that names weekdays is shown as the raw expression.
	function describeScheduleOf(
		cron: string,
		timezone: string | undefined,
		cronVersion: string | undefined
	): string | undefined {
		const dow = formatCron(cron.trim()).split(/\s+/)[5]
		if (cronVersion === 'v1' && dow !== undefined && dow !== '*' && dow !== '?') return undefined
		return describeSchedule(cron, timezone)
	}

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
	<div class="relative z-[1] hidden lg:flex items-center w-24 min-w-0 shrink-0 text-xs">
		{#if lastRun}
			<!-- The close delay lets the pointer cross into the popup to click a run. -->
			<Tooltip class="min-w-0" closeDelay={150}>
				<!-- A mouse click does nothing: the popover's "View runs" and run entries are the
				     ways in. Its links are out of the tab order, so the cell stays a real link for
				     keyboard Enter and screen-reader activation, which click with `detail === 0`. -->
				<a
					href="{base}/runs/{path}?workspace={$workspaceStore}"
					class="flex items-center gap-1.5 min-w-0 text-secondary hover:text-secondary cursor-default rounded"
					aria-label="Latest runs, open all runs"
					onclick={(e) => {
						if (e.detail > 0) e.preventDefault()
					}}
				>
					<span class={twMerge('w-1.5 h-1.5 rounded-full shrink-0', statusClass[lastRun.status])}
					></span>
					<span class="truncate">
						{#if lastRun.status === 'running'}
							Running
						{:else}
							<TimeAgo date={lastRun.created_at} compact /> ago
						{/if}
					</span>
				</a>
				{#snippet text()}
					<div class="flex flex-col gap-1 min-w-56">
						<div class="flex items-center justify-between gap-4">
							<span class="font-semibold">Latest {runs.length} runs</span>
							<Button
								variant="subtle"
								unifiedSize="xs"
								href="{base}/runs/{path}?workspace={$workspaceStore}"
							>
								View runs
							</Button>
						</div>
						{#each runs as run (run.id)}
							<a
								href="{base}/run/{run.id}?workspace={$workspaceStore}"
								class="group/run -mx-1.5 px-1.5 py-0.5 rounded flex items-center gap-2 text-primary hover:bg-surface-hover"
							>
								<div class={twMerge('w-2 h-2 rounded-full', statusClass[run.status])}></div>
								<span class="w-16">{statusLabel[run.status]}</span>
								<span class="text-hint grow">{displayDate(run.created_at, true)}</span>
								<ExternalLink size={12} class="text-hint group-hover/run:text-primary shrink-0" />
							</a>
						{/each}
					</div>
				{/snippet}
			</Tooltip>
		{:else if kind && activity}
			<span class="text-hint">Never run</span>
		{:else if !kind}
			<span class="text-hint text-2xs opacity-40">–</span>
		{/if}
	</div>

	<div class="relative z-[1] hidden lg:flex items-center w-52 min-w-0 shrink-0 text-xs">
		{#each visibleTriggers as trigger, i (i)}
			<Tooltip class="min-w-0 shrink">
				<span
					class={twMerge(
						'block truncate',
						trigger.mode === 'enabled' ? 'text-secondary' : 'text-hint line-through'
					)}>{triggerText(trigger)}</span
				>
				{#snippet text()}
					{@render triggerLine(trigger)}
				{/snippet}
			</Tooltip>
			{#if i < visibleTriggers.length - 1 || hiddenCount > 0}
				<span class="text-hint shrink-0 mr-1">,</span>
			{/if}
		{/each}
		{#if hiddenCount > 0}
			<Tooltip class="shrink-0">
				<span class="text-hint">+{hiddenCount}</span>
				{#snippet text()}
					<div class="flex flex-col gap-1">
						<span class="font-semibold">{triggers.length} triggers</span>
						{#each triggers as trigger, i (i)}
							{@render triggerLine(trigger)}
						{/each}
					</div>
				{/snippet}
			</Tooltip>
		{/if}
	</div>
{/if}

{#snippet triggerLine(trigger: Trigger)}
	{@const Icon = triggerIcon(trigger.kind)}
	<div class="flex items-center gap-2">
		{#if Icon}
			<Icon size={12} />
		{/if}
		<span class="font-semibold">{triggerText(trigger)}</span>
		<span>{trigger.path}</span>
		{#if trigger.schedule}
			<span class="text-hint font-mono">{trigger.schedule}</span>
		{/if}
		{#if trigger.mode !== 'enabled'}
			<span class="text-hint">({trigger.mode})</span>
		{/if}
	</div>
{/snippet}
