<script lang="ts">
	// Inline run viewer for the producer scripts of an asset, bundled with
	// a popover-driven history selector. Mounted in the bottom split of the
	// asset detail pane: header bar + history popover at the top, full
	// inline detail (args, logs, result) below.
	//
	// While *any* in-flight run is observed, we poll the listing every few
	// seconds so the user sees status transitions without refreshing —
	// JobLoader handles streaming for the *selected* job.
	import { JobService, type Job } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { onDestroy, untrack } from 'svelte'
	import { displayDate } from '$lib/utils'
	import { CheckCircle2, Clock, History, Loader2, XCircle, Ban } from 'lucide-svelte'
	import JobLoader from '$lib/components/JobLoader.svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import { base } from '$lib/base'
	import { Badge } from '$lib/components/common'
	import { Popover } from '$lib/components/meltComponents'
	import { twMerge } from 'tailwind-merge'
	import DispatchEventsButton from '$lib/components/runs/DispatchEventsButton.svelte'

	interface Props {
		// Producers of this asset. Each contributes its own listExtendedJobs
		// call keyed on path; results are merged and sorted desc by
		// created_at. We DO query for unsaved/draft producers too — running
		// a draft via runScriptPreview creates a `preview`-kind job under
		// the same path, so the listing surfaces those as soon as the user
		// hits play, even before the script is deployed.
		producers: Array<{ kind: 'script' | 'flow'; path: string; unsaved?: boolean }>
		// Bump this to force an immediate listing refresh — the parent page
		// uses it after dispatching a run so the new job appears without
		// waiting for the next poll tick.
		refreshKey?: any
		// Most-recently-dispatched job id. When this changes, the panel
		// switches selection to it so the user lands on their just-started
		// run without an extra click. Independent of refreshKey so callers
		// can refresh without forcing a selection change.
		pendingJobId?: string | undefined
		// Fires when the watched job transitions to terminal (success or
		// failure). The asset detail pane uses this to re-check the
		// asset's preview — a successful run materializes the asset, so
		// "not yet materialized" can move to the actual preview without
		// the user re-selecting.
		onRunCompleted?: () => void
	}
	let { producers, refreshKey, pendingJobId, onRunCompleted }: Props = $props()

	let runnableProducers = $derived(producers.filter((p) => p.kind === 'script'))
	// Stable string key for the producer set. The parent re-derives
	// `selectionProducers` (and therefore `producers`) on every change to
	// `graphWithDraft` — including every keystroke in an open draft — so
	// the array identity churns even when the path set is unchanged.
	// $derived returns the same string when contents match, and Svelte's
	// === comparison stops the change from propagating. Without this
	// debounce the listing was being re-fetched several times per second
	// and the spinner appeared frozen.
	let producerKey = $derived(
		runnableProducers
			.map((p) => p.path)
			.sort()
			.join('|')
	)

	const PER_PRODUCER = 20

	let jobs = $state<Job[]>([])
	let loading = $state(false)
	let loadError = $state<string | undefined>(undefined)
	let selectedId = $state<string | undefined>(undefined)
	let historyOpen = $state(false)

	// Live job streamed from the JobLoader whenever a run is selected.
	let selectedJob = $state<(Job & { result_stream?: string }) | undefined>(undefined)
	let jobLoader: JobLoader | undefined = $state()
	let jobLoaderLoading = $state(false)

	let pollTimer: number | undefined
	// Guards against overlapping refreshes — earlier the spinner was on
	// almost continuously because each refresh kicked another off mid-flight.
	// We keep the latest in-flight promise; concurrent calls await the same
	// promise rather than spawning duplicates.
	let refreshInFlight: Promise<void> | undefined

	$effect(() => {
		// Run only when the *content* of the producer set changes (string
		// key) or when the parent explicitly bumps `refreshKey` after
		// dispatching a new run. The producerKey debounce alone isn't
		// enough: refresh() synchronously reads $workspaceStore and
		// runnableProducers before the first await, and Svelte 5 records
		// those as deps of the surrounding $effect — which would then
		// re-fire whenever the parent re-derives `producers` (i.e. on
		// every keystroke in an open draft, since graphWithDraft cascades
		// through selectionProducers). untrack() isolates refresh()'s
		// reads so only producerKey + refreshKey gate this effect.
		void producerKey
		void refreshKey
		untrack(() => {
			void refresh()
		})
	})

	$effect(() => {
		// Auto-select the latest run on first load / producer change so the
		// bottom pane has something useful to show without an extra click.
		if (!selectedId && jobs.length > 0) {
			selectedId = jobs[0].id
		}
	})

	// Track the last pendingJobId we honored. Without this guard, manually
	// selecting a *different* run from the history popover would be undone
	// by a re-run of this effect (e.g. on parent re-derivation) — every
	// trip would slam selectedId back to pendingJobId.
	let appliedPendingJobId = $state<string | undefined>(undefined)
	$effect(() => {
		const id = pendingJobId
		if (id && id !== appliedPendingJobId) {
			appliedPendingJobId = id
			selectedId = id
		}
	})

	// Track the id we last started watching so we don't restart the
	// JobLoader stream on every effect re-run. The earlier flood (~4400
	// `/jobs_u/get/<id>` requests per second) came from JobLoader.watchJob
	// being called on every effect tick: each call resets internal state
	// and fires a fresh `getJob`, so a few extra effect runs per second
	// snowballed into thousands of fetches.
	let lastWatchedId = $state<string | undefined>(undefined)

	$effect(() => {
		const id = selectedId
		if (id === lastWatchedId) return
		lastWatchedId = id
		if (!id) {
			// untrack: clearCurrentJob reads JobLoader internals (tracked
			// state in another component); without untrack, those reads
			// become deps of this effect and any change to them
			// (e.g. JobLoader's own bind:isLoading writes) would re-run
			// it.
			untrack(() => {
				selectedJob = undefined
				void jobLoader?.clearCurrentJob?.()
			})
			return
		}
		untrack(() => {
			void jobLoader?.watchJob(id, {
				done: () => {
					// When a run we're watching finishes, refresh the listing
					// so its row shows the terminal status, and notify the
					// parent so the asset preview can re-check existence —
					// a successful run materializes the asset.
					void refresh()
					onRunCompleted?.()
				}
			})
		})
	})

	$effect(() => {
		// Poll the listing while any visible job is still running.
		const anyRunning = jobs.some((j) => isRunning(j))
		clearTimeout(pollTimer)
		if (anyRunning) {
			pollTimer = window.setTimeout(() => void refresh(), 3000)
		}
	})

	onDestroy(() => clearTimeout(pollTimer))

	async function refresh(): Promise<void> {
		if (refreshInFlight) return refreshInFlight
		if (!$workspaceStore || runnableProducers.length === 0) {
			jobs = []
			return
		}
		const ws = $workspaceStore
		// Capture the producer paths *now* — using runnableProducers
		// directly inside the await would re-read after the array
		// identity churned, defeating the in-flight guard.
		const paths = runnableProducers.map((p) => p.path)
		loading = true
		loadError = undefined
		const promise = (async () => {
			try {
				const results = await Promise.all(
					paths.map((path) =>
						JobService.listExtendedJobs({
							workspace: ws,
							scriptPathExact: path,
							// Include `preview` jobs so runs of unsaved drafts
							// (dispatched via runScriptPreview) appear in the
							// list. Same set the flow editor's StepHistory uses.
							jobKinds: ['preview', 'script', 'flowpreview', 'flow', 'flowscript'].join(','),
							perPage: PER_PRODUCER,
							page: 1
						})
					)
				)
				const merged = results.flatMap((r) => r.jobs ?? [])
				merged.sort((a, b) => (b.created_at ?? '').localeCompare(a.created_at ?? ''))
				jobs = merged
			} catch (err: any) {
				loadError = err?.body ?? err?.message ?? String(err)
			} finally {
				loading = false
				refreshInFlight = undefined
			}
		})()
		refreshInFlight = promise
		return promise
	}

	function isRunning(j: Job): boolean {
		return j.type === 'QueuedJob' && (j as any).running === true
	}
	function isQueued(j: Job): boolean {
		return j.type === 'QueuedJob' && !(j as any).running
	}
	function statusOf(j: Job): 'running' | 'queued' | 'success' | 'failure' | 'canceled' {
		if (isRunning(j)) return 'running'
		if (isQueued(j)) return 'queued'
		if ((j as any).canceled) return 'canceled'
		if ((j as any).success === false) return 'failure'
		return 'success'
	}

	function durationMs(j: Job): number | undefined {
		const started = (j as any).started_at as string | undefined
		const created = (j as any).created_at as string | undefined
		if (isRunning(j) && started) {
			return Date.now() - new Date(started).getTime()
		}
		const dms = (j as any).duration_ms as number | undefined
		if (dms != undefined) return dms
		const ended = (j as any).ended_at as string | undefined
		if (started && ended) {
			return new Date(ended).getTime() - new Date(started).getTime()
		}
		if (created && ended) {
			return new Date(ended).getTime() - new Date(created).getTime()
		}
		return undefined
	}

	function fmtDuration(ms: number | undefined): string {
		if (ms == undefined) return ''
		if (ms < 1000) return `${ms}ms`
		const s = ms / 1000
		if (s < 60) return `${s.toFixed(1)}s`
		const m = Math.floor(s / 60)
		return `${m}m ${Math.floor(s % 60)}s`
	}

	let selectedStatus = $derived(selectedJob ? statusOf(selectedJob) : undefined)
</script>

<JobLoader bind:this={jobLoader} bind:job={selectedJob} bind:isLoading={jobLoaderLoading} />

<div class="flex flex-col h-full min-h-0 bg-surface">
	<!-- Header bar: status of the currently-selected run + history popover.
	     Always visible (even with no producers) so users have a clear
	     anchor for where the runs live. -->
	<div
		class="flex items-center gap-2 px-3 py-1.5 border-b shrink-0 min-h-9 text-xs whitespace-nowrap"
	>
		{#if selectedJob && selectedStatus}
			<Badge
				color={selectedStatus === 'success'
					? 'green'
					: selectedStatus === 'failure'
						? 'red'
						: selectedStatus === 'running'
							? 'blue'
							: 'gray'}>{selectedStatus}</Badge
			>
			<span class="font-mono text-tertiary truncate min-w-0 flex-1">
				{selectedJob.script_path ?? selectedJob.id}
			</span>
			<span class="text-3xs text-tertiary shrink-0">
				{fmtDuration(durationMs(selectedJob))}
			</span>
			<a
				class="text-3xs text-blue-600 hover:underline shrink-0"
				href={`${base}/run/${selectedJob.id}?workspace=${$workspaceStore}`}
				target="_blank">Open ↗</a
			>
			{#if $workspaceStore}
				<DispatchEventsButton
					workspace={selectedJob.workspace_id ?? $workspaceStore}
					jobId={selectedJob.id}
				/>
			{/if}
		{:else if runnableProducers.length === 0}
			<span class="text-tertiary">No producer for this asset</span>
		{:else if loading && jobs.length === 0}
			<span class="text-tertiary inline-flex items-center gap-1">
				<Loader2 size={12} class="animate-spin" /> Loading runs…
			</span>
		{:else if jobs.length === 0}
			<span class="text-tertiary">No runs yet — click the play button next to the asset</span>
		{:else}
			<span class="text-tertiary">Select a run from history</span>
		{/if}

		<!-- History popover trigger: same affordance the flow editor exposes
		     on each module so users don't have to leave the page to dig
		     into prior runs. -->
		<Popover
			floatingConfig={{ strategy: 'fixed', placement: 'bottom-end', offset: { mainAxis: 8 } }}
			contentClasses="w-[340px] max-h-[420px] overflow-hidden"
			bind:isOpen={historyOpen}
		>
			{#snippet trigger()}
				<button
					type="button"
					class="ml-auto inline-flex items-center gap-1 px-1.5 py-0.5 rounded-sm hover:bg-surface-hover text-tertiary"
					title="Run history"
				>
					<History size={12} />
					{#if jobs.length > 0}<span class="text-3xs">{jobs.length}</span>{/if}
				</button>
			{/snippet}
			{#snippet content({ close })}
				<div class="flex flex-col max-h-[420px] overflow-auto">
					{#if loading && jobs.length === 0}
						<div class="flex items-center gap-1 p-3 text-xs text-tertiary">
							<Loader2 size={12} class="animate-spin" /> Loading runs…
						</div>
					{:else if loadError}
						<div class="p-3 text-xs text-red-600">Failed to load runs: {loadError}</div>
					{:else if jobs.length === 0}
						<div class="p-3 text-xs text-tertiary">
							No runs yet. Click the play button next to the asset to start one.
						</div>
					{:else}
						{#each jobs as j (j.id)}
							{@const s = statusOf(j)}
							<button
								type="button"
								onclick={() => {
									selectedId = j.id
									close()
								}}
								class={twMerge(
									'flex items-center gap-2 px-3 py-1.5 border-b last:border-b-0 hover:bg-surface-hover text-left text-xs',
									selectedId === j.id && 'bg-surface-selected'
								)}
							>
								<span class="shrink-0">
									{#if s === 'running'}
										<Loader2 size={12} class="animate-spin text-blue-500" />
									{:else if s === 'queued'}
										<Clock size={12} class="text-amber-500" />
									{:else if s === 'success'}
										<CheckCircle2 size={12} class="text-emerald-600" />
									{:else if s === 'failure'}
										<XCircle size={12} class="text-red-600" />
									{:else}
										<Ban size={12} class="text-gray-500" />
									{/if}
								</span>
								<span class="font-mono truncate flex-1 min-w-0">{j.script_path ?? j.id}</span>
								<span class="text-tertiary text-3xs whitespace-nowrap">
									{fmtDuration(durationMs(j))}
								</span>
								<span class="text-tertiary text-3xs whitespace-nowrap">
									{displayDate((j as any).created_at)}
								</span>
							</button>
						{/each}
					{/if}
				</div>
			{/snippet}
		</Popover>
	</div>

	<!-- Body: full inline detail of the selected run. -->
	<div class="flex-1 min-h-0 overflow-auto">
		{#if !selectedId}
			<div class="p-3 text-xs text-tertiary">
				{runnableProducers.length === 0
					? 'No producer is declared for this asset.'
					: jobs.length === 0
						? 'Click the play button next to the asset to start a run.'
						: 'Open the history popover to pick a run.'}
			</div>
		{:else if !selectedJob && jobLoaderLoading}
			<div class="flex items-center gap-1 p-3 text-xs text-tertiary">
				<Loader2 size={12} class="animate-spin" /> Loading run…
			</div>
		{:else if selectedJob}
			<div class="flex flex-col gap-3 p-3">
				<JobArgs
					id={selectedJob.id}
					workspace={selectedJob.workspace_id ?? $workspaceStore ?? ''}
					args={selectedJob.args}
				/>
				<div class="flex flex-col gap-1">
					<div class="text-3xs uppercase tracking-wide text-tertiary">Logs</div>
					<LogViewer
						jobId={selectedJob.id}
						tag={selectedJob.tag}
						duration={durationMs(selectedJob)}
						mem={(selectedJob as any).mem_peak}
						isLoading={jobLoaderLoading && !(selectedJob as any).logs}
						content={(selectedJob as any).logs}
						download={false}
					/>
				</div>
				{#if selectedStatus === 'success' || selectedStatus === 'failure'}
					<div class="flex flex-col gap-1">
						<div class="text-3xs uppercase tracking-wide text-tertiary">
							{selectedStatus === 'failure' ? 'Error' : 'Result'}
						</div>
						<DisplayResult
							workspaceId={selectedJob.workspace_id}
							jobId={selectedJob.id}
							result={(selectedJob as any).result}
						/>
					</div>
				{/if}
			</div>
		{/if}
	</div>
</div>
