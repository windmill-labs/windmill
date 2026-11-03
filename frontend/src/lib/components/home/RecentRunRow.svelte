<script lang="ts">
	import { goto } from '$app/navigation'
	import { workspaceStore } from '$lib/stores'
	import { CircleCheck, CircleSlash, CircleX, Loader2, Wrench } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { useRoleCapabilities } from '$lib/operatorWriteRights'
	import {
		askAI,
		canAskAI,
		itemHref,
		itemTitle,
		runHref,
		shortAge,
		type RecentRun,
		type RecentRunItem
	} from './recentRuns.svelte'

	interface Props {
		item: RecentRunItem
		now: number
		/** Shown before the title when another row carries the same title. */
		prefix?: string
		strip?: boolean
		/** What a click on the row does; without it the row opens the item's page. The run links
		 * open runs either way. */
		onSelect?: (item: RecentRunItem) => void
		/** Resolves the item's failed runs; without it the row offers no resolve action. */
		onResolve?: (item: RecentRunItem) => Promise<void>
	}

	let { item, now, prefix, strip = true, onSelect, onResolve }: Props = $props()

	const roleCaps = useRoleCapabilities()
	let resolving = $state(false)

	const STRIP_SLOTS = 10
	// The strip reads oldest to newest; the item's runs arrive newest first.
	let slots = $derived.by(() => {
		const runs = item.runs.slice(0, STRIP_SLOTS).reverse()
		return [...Array(STRIP_SLOTS - runs.length).fill(undefined), ...runs] as (
			| RecentRun
			| undefined
		)[]
	})
	let latest = $derived(item.runs[0])
	let failing = $derived(item.status === 'failure')
	let title = $derived(itemTitle(item))

	const statusLabel = {
		success: 'Succeeded',
		failure: 'Failed',
		resolved: 'Failed, resolved',
		canceled: 'Canceled',
		running: 'Running'
	}
	const barClass = {
		success: 'bg-green-500 dark:bg-green-600',
		failure: 'bg-red-500 dark:bg-red-600',
		resolved: 'bg-orange-300 dark:bg-orange-700',
		canceled: 'bg-gray-400 dark:bg-gray-500',
		running: 'bg-yellow-500'
	}

	function runTooltip(run: RecentRun) {
		const at = new Date(run.ran_at).toLocaleTimeString([], {
			hour: '2-digit',
			minute: '2-digit',
			hourCycle: 'h23'
		})
		const parts = [statusLabel[run.status], at]
		if (run.duration_ms != undefined) parts.push(formatDuration(run.duration_ms))
		parts.push('open run')
		return parts.join(' · ')
	}

	function formatDuration(ms: number) {
		if (ms < 1000) return `${ms}ms`
		const s = ms / 1000
		if (s < 60) return `${s.toFixed(s < 10 ? 1 : 0)}s`
		return `${Math.floor(s / 60)}m ${Math.round(s % 60)}s`
	}

	function open() {
		if (onSelect) onSelect(item)
		else goto(itemHref(item))
	}
</script>

<div
	role={onSelect ? 'button' : 'link'}
	tabindex="0"
	aria-label={onSelect ? `Show ${title} in the workspace list` : `Open ${title}`}
	class="group relative flex items-center gap-2.5 min-h-[42px] px-2.5 py-[5px] rounded-md cursor-pointer hover:bg-surface-hover focus-visible:bg-surface-hover outline-none"
	onclick={open}
	onkeydown={(e) => {
		if (e.key === 'Enter' && e.target === e.currentTarget) open()
	}}
>
	<span class="flex w-3.5 shrink-0 justify-center">
		{#if item.status === 'running'}
			<Loader2 size={14} class="animate-spin text-yellow-500" />
		{:else if failing}
			<CircleX size={14} class="text-red-500" />
		{:else if item.status === 'resolved'}
			<span class="text-orange-500" title="Failed, marked as resolved"><Wrench size={14} /></span>
		{:else if item.status === 'canceled'}
			<CircleSlash size={14} class="text-hint" />
		{:else}
			<CircleCheck size={14} class="text-green-500" />
		{/if}
	</span>

	<span class="flex flex-col min-w-0 flex-1">
		<span
			class="truncate text-[12.5px] {failing
				? 'font-semibold text-emphasis'
				: 'font-medium text-primary'}"
		>
			{#if prefix}<span class="text-hint font-normal">{prefix} / </span>{/if}{title}
		</span>
		<span class="flex items-center gap-1.5 min-w-0 text-[10.5px] text-hint">
			<span class="truncate">{item.path}</span>
			<!-- The header counts failed runs, not failing items: this per-item count shows where its
			     number comes from. -->
			{#if item.failed_runs > 0}
				<span class="shrink-0 text-red-600 dark:text-red-400">{item.failed_runs} failed</span>
				{#if onResolve && $roleCaps.code}
					<!-- Negative margins keep the button's height from growing the row on hover. -->
					<span
						class="hidden group-hover:inline-flex group-focus-within:inline-flex shrink-0 -my-[3px]"
					>
						<Button
							variant="subtle"
							unifiedSize="2xs"
							btnClasses="!text-accent !text-[10.5px] !px-1 font-medium"
							title="Mark the {item.failed_runs} failed runs of {title} as handled"
							loading={resolving}
							onClick={async (e) => {
								e?.stopPropagation()
								resolving = true
								try {
									await onResolve(item)
								} finally {
									resolving = false
								}
							}}
						>
							{item.failed_runs === 1 ? 'Resolve' : 'Resolve all'}
						</Button>
					</span>
				{/if}
			{/if}
			{#if failing && canAskAI() && latest}
				<button
					class="hidden group-hover:inline group-focus-within:inline shrink-0 font-medium text-ai hover:underline"
					onclick={(e) => {
						e.stopPropagation()
						askAI({ kind: 'run', path: item.path, jobId: latest.id }, 'Why did this run fail?')
					}}
				>
					Ask AI why it failed
				</button>
			{/if}
		</span>
	</span>

	{#if canAskAI() && latest}
		<!-- While Home's composer is open, on the hovered or focused row: the latest run as context,
		     like the cards' `+@`. -->
		<span
			class="hidden shrink-0 group-has-[[data-composer-open]]/home:group-hover:inline-flex group-has-[[data-composer-open]]/home:group-focus-within:inline-flex"
		>
			<Button
				variant="subtle"
				unifiedSize="sm"
				btnClasses="!text-ai font-medium"
				title="Add the latest run to AI session context"
				aria-label="Add the latest run of {title} to AI session context"
				onClick={(e) => {
					e?.stopPropagation()
					askAI({ kind: 'run', path: item.path, jobId: latest.id })
				}}
			>
				+@
			</Button>
		</span>
	{/if}

	{#if strip}
		<span class="flex items-center gap-0.5 shrink-0">
			{#each slots as run, i (run?.id ?? `empty-${i}`)}
				{#if run}
					<a
						href={runHref(run.id, $workspaceStore)}
						class="block w-1 h-3 rounded-[1px] {barClass[run.status]} hover:opacity-70"
						title={runTooltip(run)}
						aria-label={runTooltip(run)}
						onclick={(e) => e.stopPropagation()}
					></a>
				{:else}
					<span class="block w-1 h-3 rounded-[1px] bg-surface-sunken"></span>
				{/if}
			{/each}
		</span>
	{/if}

	<span class="relative w-[46px] shrink-0 text-right text-[11px] text-hint">
		<span class="group-hover:invisible group-focus-within:invisible">
			{latest ? shortAge(latest, now) : ''}
		</span>
		{#if latest}
			<a
				href={runHref(latest.id, $workspaceStore)}
				class="absolute inset-0 text-right font-medium text-accent opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 hover:underline"
				aria-label="Open the latest run of {title}"
				onclick={(e) => e.stopPropagation()}
			>
				Run ↗
			</a>
		{/if}
	</span>
</div>
