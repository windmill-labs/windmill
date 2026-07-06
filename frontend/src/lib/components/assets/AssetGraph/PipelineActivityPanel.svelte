<script lang="ts">
	import { base } from '$lib/base'
	import { workspaceStore } from '$lib/stores'
	import type { Job } from '$lib/gen'
	import Select from '$lib/components/select/Select.svelte'
	import JobLoader from '$lib/components/JobLoader.svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import {
		CheckCircle2,
		ChevronDown,
		ChevronRight,
		Clock,
		Code2,
		GitBranch,
		History,
		Loader2,
		XCircle
	} from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { untrack } from 'svelte'
	import { isActiveEvent, type EventStatus, type PipelineEvent } from './activeRunnables.svelte'
	import type { DispatchEdge } from './pipelineHistory.svelte'
	import { RotateCcw, Workflow } from 'lucide-svelte'
	import ActivityHistogram from './ActivityHistogram.svelte'

	interface Props {
		// Merged history + live events, newest-first (page owns the merge —
		// the live poll wins on id collisions since it carries fresher status).
		events: PipelineEvent[]
		// Asset-cascade producer→child edges (by job id) used to group the
		// runs of one cascade together.
		edges?: DispatchEdge[]
		loading?: boolean
		// History preload hit its page cap before the days cutoff.
		truncated?: boolean
		error?: string | undefined
		days?: number
		onDaysChange?: (days: number) => void
		// Live-only surfaces (local-dev preview) have no historical fetch: hide
		// the day-range Select + histogram (both are history-window concepts) and
		// show only the live run stream.
		liveOnly?: boolean
		// Hover a run row → emphasize its node(s) on the canvas; a group header
		// passes the whole cascade's paths. `undefined` clears.
		onHoverRun?: (paths: string[] | undefined) => void
		// Expand/collapse a run → sticky node emphasis (undefined clears).
		onSelectRun?: (paths: string[] | undefined) => void
	}

	let {
		events,
		edges = [],
		loading = false,
		truncated = false,
		error,
		days = 30,
		onDaysChange,
		liveOnly = false,
		onHoverRun,
		onSelectRun
	}: Props = $props()

	type CascadeGroup = {
		// Component representative id (stable per cascade).
		key: string
		// Earliest originating (non-asset) run — the group header. Undefined when
		// the originating run fell outside the loaded window.
		root: PipelineEvent | undefined
		// Additional originating triggers beyond `root` (joins fed by >1 source).
		extraTriggers: number
		// All loaded runs of this cascade: roots first, then newest-first.
		members: PipelineEvent[]
		// Worth grouping (more than one run, or an edge reaches outside window).
		isCascade: boolean
		// Aggregate status across members (running > failure > success).
		status: EventStatus
		// Newest member time — drives group ordering.
		latestAt: string
	}

	// Histogram window (last `days`) and the brushed sub-range. The histogram
	// always shows the full window; the list + groups filter to the brush.
	let windowBounds = $derived.by(() => {
		void events // refresh as new runs arrive
		const to = Date.now()
		return { from: to - days * 86400000, to }
	})
	let selectedRange = $state<{ from: number; to: number } | undefined>(undefined)
	// Drop a stale brush when the day window changes under it.
	$effect(() => {
		void days
		untrack(() => (selectedRange = undefined))
	})
	let filteredEvents = $derived.by(() => {
		const r = selectedRange
		if (!r) return events
		return events.filter((e) => {
			const t = Date.parse(e.at)
			return !isNaN(t) && t >= r.from && t <= r.to
		})
	})
	// Quick reset: drop the brush and return to the default 30-day window.
	function resetWindow() {
		selectedRange = undefined
		if (days !== 30) onDaysChange?.(30)
	}
	function fmtRange(r: { from: number; to: number }): string {
		const opt: Intl.DateTimeFormatOptions = {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		}
		return `${new Date(r.from).toLocaleString(undefined, opt)} – ${new Date(r.to).toLocaleString(undefined, opt)}`
	}

	// Group cascades by the CONNECTED COMPONENT of the dispatch graph, so an
	// AND-join (fed by several trigger chains) folds into one group rather than
	// fragmenting by whichever input completed it last. Edges come by job id
	// from `dispatched` rows; `join_pending` inputs (no child yet) are linked to
	// the next firing of the same subscriber so the pre-completion contributors
	// join the component too.
	let groups = $derived.by<CascadeGroup[]>(() => {
		// Producer→child edges (job id). `childSet` = jobs with an incoming edge
		// (i.e. not an originating run).
		const childEdges: Array<[string, string]> = []
		const childSet = new Set<string>()
		const inEdges = new Set<string>()
		// Firings per subscriber path (a `dispatched` row), time-sorted, to map
		// each join_pending input forward to the child it contributed to.
		const firingsBySub = new Map<string, Array<{ child: string; at: number }>>()
		for (const e of edges) {
			if (e.outcome === 'dispatched' && e.child_job_id) {
				childEdges.push([e.producer_job_id, e.child_job_id])
				childSet.add(e.child_job_id)
				const arr = firingsBySub.get(e.subscriber_path)
				const f = { child: e.child_job_id, at: Date.parse(e.created_at) }
				if (arr) arr.push(f)
				else firingsBySub.set(e.subscriber_path, [f])
			}
		}
		for (const arr of firingsBySub.values()) arr.sort((a, b) => a.at - b.at)
		for (const e of edges) {
			if (e.outcome !== 'join_pending') continue
			const firings = firingsBySub.get(e.subscriber_path)
			if (!firings) continue
			const t = Date.parse(e.created_at)
			const firing = firings.find((f) => f.at >= t)
			if (firing) {
				childEdges.push([e.producer_job_id, firing.child])
				childSet.add(firing.child)
			}
		}
		for (const [p, c] of childEdges) {
			inEdges.add(p)
			inEdges.add(c)
		}
		// Union-find over the edges.
		const parent = new Map<string, string>()
		const find = (x: string): string => {
			let r = x
			while (parent.has(r) && parent.get(r) !== r) r = parent.get(r)!
			return r
		}
		const union = (a: string, b: string) => {
			if (!parent.has(a)) parent.set(a, a)
			if (!parent.has(b)) parent.set(b, b)
			const ra = find(a)
			const rb = find(b)
			if (ra !== rb) parent.set(ra, rb)
		}
		for (const [p, c] of childEdges) union(p, c)

		const byComponent = new Map<string, PipelineEvent[]>()
		for (const e of filteredEvents) {
			const k = parent.has(e.id) ? find(e.id) : e.id
			const arr = byComponent.get(k)
			if (arr) arr.push(e)
			else byComponent.set(k, [e])
		}
		const out: CascadeGroup[] = []
		for (const [key, members] of byComponent) {
			const roots = members
				.filter((m) => !childSet.has(m.id))
				.sort((a, b) => (a.at < b.at ? -1 : a.at > b.at ? 1 : 0))
			members.sort((a, b) => {
				const aRoot = !childSet.has(a.id)
				const bRoot = !childSet.has(b.id)
				if (aRoot !== bRoot) return aRoot ? -1 : 1
				if (aRoot) return a.at < b.at ? -1 : a.at > b.at ? 1 : 0 // roots earliest-first
				return a.at < b.at ? 1 : a.at > b.at ? -1 : 0 // hops newest-first
			})
			const isCascade = members.length > 1 || members.some((m) => inEdges.has(m.id))
			const anyActive = members.some((m) => m.status === 'running' || m.status === 'queued')
			const anyFail = members.some((m) => m.status === 'failure')
			const status: EventStatus = anyActive ? 'running' : anyFail ? 'failure' : 'success'
			const latestAt = members.reduce((mx, m) => (m.at > mx ? m.at : mx), members[0].at)
			out.push({
				key,
				root: roots[0],
				extraTriggers: Math.max(0, roots.length - 1),
				members,
				isCascade,
				status,
				latestAt
			})
		}
		out.sort((a, b) => (a.latestAt < b.latestAt ? 1 : a.latestAt > b.latestAt ? -1 : 0))
		return out
	})

	// Collapsed cascade groups (members hidden). Default expanded.
	let collapsedGroups = $state<Set<string>>(new Set())
	function toggleGroup(key: string) {
		const next = new Set(collapsedGroups)
		if (next.has(key)) next.delete(key)
		else next.add(key)
		collapsedGroups = next
	}

	// The cascade's trigger label, shown on the group header — the earliest
	// originating run's source. A loaded root is never asset-triggered (an asset
	// hop always has a producer), so its `source` is the real origin; an
	// unloaded root means the originating run fell outside the window.
	function groupTrigger(g: CascadeGroup): string {
		if (!g.root) return 'cascade'
		return g.root.source === 'schedule' ? 'schedule' : 'run'
	}
	function groupTitle(g: CascadeGroup): string {
		return g.root?.path ?? 'cascade (upstream outside window)'
	}

	// Window options as fractions of a day (the fetch cutoff is `days * 1d`).
	const DAY_OPTIONS = [
		{ label: 'Last hour', value: 1 / 24 },
		{ label: 'Last 24 hours', value: 1 },
		{ label: 'Last 48 hours', value: 2 },
		{ label: 'Last 7 days', value: 7 },
		{ label: 'Last 30 days', value: 30 },
		{ label: 'Last 90 days', value: 90 }
	]
	let windowLabel = $derived(
		DAY_OPTIONS.find((o) => o.value === days)?.label ?? `Last ${days} days`
	)

	// Excludes future-scheduled queued jobs (a schedule's next planned run
	// is not activity) — see isActiveEvent.
	let runningCount = $derived(events.filter((e) => isActiveEvent(e)).length)

	// Accordion: one expanded run at a time, streamed via JobLoader (live
	// logs while running, terminal result when done) — same pattern as
	// AssetRunsPanel's selected job.
	let expandedId = $state<string | undefined>(undefined)
	let expandedJob = $state<(Job & { logs?: string; result?: any }) | undefined>(undefined)
	let jobLoader: JobLoader | undefined = $state()
	let jobLoaderLoading = $state(false)
	// Only (re)start the JobLoader stream when the expanded id actually
	// changes — watchJob resets internal state and refetches on every call
	// (cf. the request-flood note in AssetRunsPanel).
	let lastWatchedId = $state<string | undefined>(undefined)
	$effect(() => {
		const id = expandedId
		if (id === lastWatchedId) return
		lastWatchedId = id
		if (!id) {
			untrack(() => {
				expandedJob = undefined
				void jobLoader?.clearCurrentJob?.()
			})
			return
		}
		untrack(() => {
			void jobLoader?.watchJob(id)
		})
	})

	function toggle(id: string, path: string) {
		const willExpand = expandedId !== id
		expandedId = willExpand ? id : undefined
		// Expanding a run pins its node emphasis; collapsing clears it.
		onSelectRun?.(willExpand ? [path] : undefined)
	}

	function isRunningJob(j: Job): boolean {
		return j.type === 'QueuedJob' && (j as any).running === true
	}
	function jobStatus(j: Job): 'running' | 'queued' | 'success' | 'failure' | 'canceled' {
		if (isRunningJob(j)) return 'running'
		if (j.type === 'QueuedJob') return 'queued'
		if ((j as any).canceled) return 'canceled'
		if ((j as any).success === false) return 'failure'
		return 'success'
	}
	function durationMs(j: Job): number | undefined {
		const started = (j as any).started_at as string | undefined
		const created = (j as any).created_at as string | undefined
		if (isRunningJob(j) && started) {
			return Date.now() - new Date(started).getTime()
		}
		const dms = (j as any).duration_ms as number | undefined
		if (dms != undefined) return dms
		const ended = (j as any).ended_at as string | undefined
		if (started && ended) return new Date(ended).getTime() - new Date(started).getTime()
		if (created && ended) return new Date(ended).getTime() - new Date(created).getTime()
		return undefined
	}

	function ago(iso: string): string {
		const s = Math.max(0, Math.floor((Date.now() - new Date(iso).getTime()) / 1000))
		if (s < 60) return `${s}s ago`
		if (s < 3600) return `${Math.floor(s / 60)}m ago`
		if (s < 86400) return `${Math.floor(s / 3600)}h ago`
		return `${Math.floor(s / 86400)}d ago`
	}

	// A schedule's next planned run sits in the queue with a future
	// scheduled_for — show when it's due instead of a misleading "ago"
	// (its `at` is the enqueue time).
	function isFutureScheduled(e: PipelineEvent): boolean {
		return e.status === 'queued' && !isActiveEvent(e)
	}
	function inTxt(iso: string): string {
		const s = Math.max(0, Math.floor((new Date(iso).getTime() - Date.now()) / 1000))
		if (s < 60) return `in ${s}s`
		if (s < 3600) return `in ${Math.floor(s / 60)}m`
		if (s < 86400) return `in ${Math.floor(s / 3600)}h`
		return `in ${Math.floor(s / 86400)}d`
	}
</script>

<JobLoader bind:this={jobLoader} bind:job={expandedJob} bind:isLoading={jobLoaderLoading} />

<div class="flex flex-col h-full bg-surface">
	<div class="flex items-center justify-between gap-2 px-3 py-2 border-b shrink-0 min-h-10">
		<span class="flex items-center gap-2 text-xs font-semibold text-emphasis">
			<History size={14} />
			Activity
			{#if runningCount > 0}
				<span
					class="flex items-center gap-1 px-1.5 py-0.5 rounded-full bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300 text-2xs font-normal"
				>
					<Loader2 size={9} class="animate-spin" />
					{runningCount} running
				</span>
			{:else if loading}
				<Loader2 size={12} class="animate-spin text-tertiary" />
			{/if}
		</span>
		{#if !liveOnly}
			<div class="w-36 shrink-0">
				<Select
					items={DAY_OPTIONS}
					size="sm"
					clearable={false}
					bind:value={() => days, (v) => onDaysChange?.(v ?? 30)}
				/>
			</div>
		{/if}
	</div>

	{#if events.length > 0 && !liveOnly}
		<div class="border-b shrink-0">
			<ActivityHistogram
				{events}
				from={windowBounds.from}
				to={windowBounds.to}
				selected={selectedRange}
				onSelect={(r) => (selectedRange = r)}
			/>
			{#if selectedRange || days !== 30}
				<div class="flex items-center justify-between px-2 pb-1.5 text-2xs text-tertiary">
					<span class="truncate">
						{selectedRange ? fmtRange(selectedRange) : windowLabel}
					</span>
					<button
						type="button"
						class="flex items-center gap-1 px-1.5 py-0.5 rounded-sm hover:bg-surface-hover text-secondary shrink-0"
						onclick={resetWindow}
						title="Reset to the last 30 days"
					>
						<RotateCcw size={11} /> Reset
					</button>
				</div>
			{/if}
		</div>
	{/if}

	<div class="flex-1 min-h-0 overflow-y-auto px-1 py-1">
		{#if error}
			<div class="px-3 py-4 text-xs text-red-600 dark:text-red-400">
				Failed to load activity: {error}
			</div>
		{:else if events.length === 0}
			{#if loading}
				<div class="flex items-center justify-center gap-2 px-3 py-6 text-xs text-tertiary">
					<Loader2 size={13} class="animate-spin" /> Loading activity…
				</div>
			{:else}
				<div class="px-3 py-6 text-center text-xs text-tertiary">
					{#if liveOnly}
						No runs yet — executions of this pipeline will appear here live.
					{:else}
						No runs in this window ({windowLabel.toLowerCase()}) — executions of this pipeline will
						appear here live.
					{/if}
				</div>
			{/if}
		{:else}
			{#snippet runRow(e: PipelineEvent, isMember: boolean, showSource: boolean)}
				{@const expanded = expandedId === e.id}
				<button
					type="button"
					onclick={() => toggle(e.id, e.path)}
					onmouseenter={() => onHoverRun?.([e.path])}
					onmouseleave={() => onHoverRun?.(undefined)}
					class={twMerge(
						'w-full flex items-center gap-2 px-2 py-1 rounded-sm hover:bg-surface-hover text-xs text-left',
						isMember && 'pl-7',
						expanded && 'bg-surface-selected'
					)}
					title={expanded ? 'Collapse run details' : 'Expand run details'}
				>
					<span class="shrink-0 text-tertiary">
						{#if expanded}
							<ChevronDown size={12} />
						{:else}
							<ChevronRight size={12} />
						{/if}
					</span>
					<span class="shrink-0">
						{#if e.status === 'running'}
							<Loader2 size={12} class="animate-spin text-blue-600 dark:text-blue-400" />
						{:else if e.status === 'queued'}
							<Clock size={12} class="text-gray-500" />
						{:else if e.status === 'success'}
							<CheckCircle2 size={12} class="text-emerald-600 dark:text-emerald-400" />
						{:else}
							<XCircle size={12} class="text-red-600 dark:text-red-400" />
						{/if}
					</span>
					{#if e.kind === 'flow'}
						<GitBranch size={12} class="shrink-0 text-tertiary" />
					{:else}
						<Code2 size={12} class="shrink-0 text-tertiary" />
					{/if}
					<span class="flex-1 min-w-0 truncate font-mono text-2xs" title={e.path}>
						{e.path}
					</span>
					{#if showSource}
						<span
							class={twMerge(
								'shrink-0 px-1 py-0.5 rounded-sm text-3xs leading-none',
								e.source === 'schedule'
									? 'bg-amber-100 dark:bg-amber-900/40 text-amber-700 dark:text-amber-300'
									: 'bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400'
							)}
						>
							{e.source}
						</span>
					{/if}
					<span class="shrink-0 text-3xs text-tertiary tabular-nums w-14 text-right">
						{isFutureScheduled(e) && e.scheduledFor ? inTxt(e.scheduledFor) : ago(e.at)}
					</span>
				</button>
				{#if expanded}
					<div class="mx-2 mb-1 border rounded-md bg-surface-secondary/30">
						{#if !expandedJob && jobLoaderLoading}
							<div class="flex items-center gap-1 p-3 text-xs text-tertiary">
								<Loader2 size={12} class="animate-spin" /> Loading run…
							</div>
						{:else if expandedJob}
							{@const status = jobStatus(expandedJob)}
							<div class="flex flex-col gap-2 p-2">
								<div class="flex items-center gap-2 text-2xs text-tertiary">
									<span class="font-mono truncate flex-1 min-w-0">{expandedJob.id}</span>
									<a
										class="text-blue-600 hover:underline shrink-0"
										href={`${base}/run/${expandedJob.id}?workspace=${$workspaceStore}`}
										target="_blank">Open ↗</a
									>
								</div>
								<JobArgs
									id={expandedJob.id}
									workspace={expandedJob.workspace_id ?? $workspaceStore ?? ''}
									args={expandedJob.args}
								/>
								<div class="flex flex-col gap-1">
									<div class="text-3xs uppercase tracking-wide text-tertiary">Logs</div>
									<LogViewer
										jobId={expandedJob.id}
										duration={durationMs(expandedJob)}
										mem={(expandedJob as any).mem_peak}
										tag={(expandedJob as any).tag}
										isLoading={jobLoaderLoading && !expandedJob.logs}
										content={expandedJob.logs}
										download={false}
									/>
								</div>
								{#if status === 'success' || status === 'failure'}
									<div class="flex flex-col gap-1">
										<div class="text-3xs uppercase tracking-wide text-tertiary">
											{status === 'failure' ? 'Error' : 'Result'}
										</div>
										<DisplayResult
											workspaceId={expandedJob.workspace_id}
											jobId={expandedJob.id}
											result={expandedJob.result}
										/>
									</div>
								{/if}
							</div>
						{:else}
							<div class="p-3 text-xs text-tertiary">Could not load this run.</div>
						{/if}
					</div>
				{/if}
			{/snippet}

			{#if groups.length === 0}
				<div class="px-3 py-6 text-center text-xs text-tertiary">
					No runs in the selected time range.
				</div>
			{/if}
			{#each groups as g (g.key)}
				{#if g.isCascade}
					{@const collapsed = collapsedGroups.has(g.key)}
					<!-- Cascade group header: the originating run + its trigger, the
					     aggregate status, and a member count. Collapses the hops. -->
					<button
						type="button"
						onclick={() => toggleGroup(g.key)}
						onmouseenter={() => onHoverRun?.(g.members.map((m) => m.path))}
						onmouseleave={() => onHoverRun?.(undefined)}
						class="w-full flex items-center gap-2 px-2 py-1 rounded-sm hover:bg-surface-hover text-xs text-left"
						title={collapsed ? 'Expand cascade runs' : 'Collapse cascade runs'}
					>
						<span class="shrink-0 text-tertiary">
							{#if collapsed}<ChevronRight size={12} />{:else}<ChevronDown size={12} />{/if}
						</span>
						<span class="shrink-0">
							{#if g.status === 'running'}
								<Loader2 size={12} class="animate-spin text-blue-600 dark:text-blue-400" />
							{:else if g.status === 'failure'}
								<XCircle size={12} class="text-red-600 dark:text-red-400" />
							{:else}
								<CheckCircle2 size={12} class="text-emerald-600 dark:text-emerald-400" />
							{/if}
						</span>
						<Workflow size={12} class="shrink-0 text-indigo-500" />
						<span class="flex-1 min-w-0 truncate font-mono text-2xs" title={groupTitle(g)}>
							{groupTitle(g)}
						</span>
						<span
							class={twMerge(
								'shrink-0 px-1 py-0.5 rounded-sm text-3xs leading-none',
								groupTrigger(g) === 'schedule'
									? 'bg-amber-100 dark:bg-amber-900/40 text-amber-700 dark:text-amber-300'
									: 'bg-indigo-100 dark:bg-indigo-900/40 text-indigo-700 dark:text-indigo-300'
							)}
						>
							{groupTrigger(g)}
						</span>
						{#if g.extraTriggers > 0}
							<span
								class="shrink-0 px-1 py-0.5 rounded-sm text-3xs leading-none bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400"
								title={`Join fed by ${g.extraTriggers + 1} triggers`}>+{g.extraTriggers}</span
							>
						{/if}
						<span class="shrink-0 text-3xs text-tertiary tabular-nums">{g.members.length} runs</span
						>
						<span class="shrink-0 text-3xs text-tertiary tabular-nums w-14 text-right">
							{ago(g.latestAt)}
						</span>
					</button>
					{#if !collapsed}
						{#each g.members as m (m.id)}
							{@render runRow(m, true, false)}
						{/each}
					{/if}
				{:else}
					{@render runRow(g.members[0], false, true)}
				{/if}
			{/each}
			{#if truncated}
				<div class="px-3 py-2 text-center text-2xs text-tertiary">
					Showing the latest {events.length} runs — older runs in this window are not listed.
				</div>
			{/if}
		{/if}
	</div>
</div>
