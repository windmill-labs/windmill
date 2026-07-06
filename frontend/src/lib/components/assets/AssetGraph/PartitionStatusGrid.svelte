<script lang="ts">
	import { resource } from 'runed'
	import { AssetService, JobService, type MaterializedPartition } from '$lib/gen'
	import { Button } from '$lib/components/common'
	import { Loader2, RefreshCw, History } from 'lucide-svelte'
	import { enterpriseLicense } from '$lib/stores'
	import BackfillRangeDialog from './BackfillRangeDialog.svelte'
	import { runBackfill, type BackfillSliceState } from './backfillRun'
	import { makeWaitJobTerminal } from './cascadeRun'

	interface Props {
		// The materialized ducklake asset path (`<ducklake>/<table>`).
		path: string
		workspace: string
	}
	let { path, workspace }: Props = $props()

	let partitions = resource([() => workspace, () => path], async ([ws, p]) => {
		if (!ws || !p) return [] as MaterializedPartition[]
		return await AssetService.listAssetPartitions({ workspace: ws, path: p })
	})

	let backfillOpen = $state(false)
	// Slice states of the in-flight (or last finished) backfill. Owned here —
	// not in the dialog — so progress keeps streaming in the header when the
	// dialog is closed mid-run.
	let backfillSlices = $state<BackfillSliceState[] | undefined>(undefined)
	let backfillRunning = $state(false)
	let backfillCancelRequested = $state(false)

	let backfillDone = $derived(
		backfillSlices?.filter((s) => s.status === 'success' || s.status === 'failure').length ?? 0
	)
	let backfillFailed = $derived(backfillSlices?.filter((s) => s.status === 'failure').length ?? 0)

	// Never throws: the job may already be terminal, and the sequential loop
	// stops on the cancel flag regardless.
	async function cancelJobById(jobId: string) {
		try {
			await JobService.cancelQueuedJob({
				workspace,
				id: jobId,
				requestBody: { reason: 'backfill cancelled' }
			})
		} catch {}
	}

	// Stop scheduling further slices and cancel the in-flight run (its slice
	// then reports failure); already-materialized slices are unaffected. A
	// cancel that lands while a launch is in flight (no job id yet) is
	// finished by the runner via its `cancelJob` hook.
	async function cancelBackfill() {
		backfillCancelRequested = true
		const running = backfillSlices?.find((s) => s.status === 'running')
		if (running?.jobId) {
			await cancelJobById(running.jobId)
		}
	}

	async function startBackfill(producerPath: string, worklist: string[]) {
		if (backfillRunning || worklist.length === 0) return
		backfillRunning = true
		backfillCancelRequested = false
		try {
			await runBackfill({
				partitions: worklist,
				// Backend asset dispatch stays ON (unlike client-orchestrated dev
				// cascades, which pass `_wmill_skip_asset_dispatch`): a backfilled
				// slice refreshes its deployed consumers like any deployed run,
				// carrying its partition down the chain.
				launch: async (partition) =>
					await JobService.runScriptByPath({
						workspace,
						path: producerPath,
						requestBody: { partition }
					}),
				waitTerminal: async (jobId) => {
					const term = await makeWaitJobTerminal(workspace)(jobId)
					// The run just recorded (or failed to record) its
					// materialized_partition row — stream it into the grid.
					partitions.refetch()
					return term
				},
				onUpdate: (s) => (backfillSlices = s),
				isCancelled: () => backfillCancelRequested,
				cancelJob: cancelJobById
			})
		} finally {
			backfillRunning = false
			partitions.refetch()
		}
	}

	const statusClass: Record<MaterializedPartition['status'], string> = {
		materialized: 'bg-green-100 text-green-800 dark:bg-green-900/40 dark:text-green-300',
		running: 'bg-blue-100 text-blue-800 dark:bg-blue-900/40 dark:text-blue-300',
		failed: 'bg-red-100 text-red-800 dark:bg-red-900/40 dark:text-red-300'
	}
</script>

<div class="flex flex-col h-full">
	<div class="flex items-center justify-between gap-2 px-3 py-2 border-b shrink-0">
		<span class="text-xs font-semibold text-secondary">Materialized partitions</span>
		<div class="flex items-center gap-2">
			{#if backfillRunning}
				<button
					class="flex items-center gap-1 text-3xs text-tertiary hover:text-primary"
					onclick={() => (backfillOpen = true)}
					title="Show backfill progress"
				>
					<Loader2 size={12} class="animate-spin" />
					Backfilling {backfillDone}/{backfillSlices?.length ?? 0}
				</button>
			{:else if backfillSlices?.length}
				<button
					class="text-3xs {backfillFailed > 0
						? 'text-red-600'
						: 'text-tertiary'} hover:text-primary"
					onclick={() => (backfillOpen = true)}
					title="Show backfill result"
				>
					Backfill: {backfillDone - backfillFailed}/{backfillSlices.length} ok{backfillFailed > 0
						? `, ${backfillFailed} failed`
						: ''}
				</button>
			{/if}
			<div class="flex items-center gap-1">
				<Button
					variant="subtle"
					unifiedSize="sm"
					startIcon={{ icon: RefreshCw }}
					iconOnly
					onclick={() => partitions.refetch()}
					title="Refresh"
				/>
				<Button
					variant="default"
					unifiedSize="sm"
					startIcon={{ icon: History }}
					disabled={!$enterpriseLicense}
					onclick={() => (backfillOpen = true)}
					title={$enterpriseLicense
						? 'Backfill a range of partitions'
						: 'Backfill a range of partitions (enterprise feature)'}
				>
					Backfill
				</Button>
			</div>
		</div>
	</div>

	<div class="flex-1 min-h-0 overflow-auto p-3">
		{#if partitions.loading}
			<div class="flex items-center gap-2 text-tertiary text-xs">
				<Loader2 size={14} class="animate-spin" /> Loading partitions…
			</div>
		{:else if partitions.error}
			<p class="text-xs text-red-600">Failed to load: {partitions.error.message}</p>
		{:else if !partitions.current?.length}
			<p class="text-xs text-secondary">
				No partitions materialized yet. They appear here after a <span class="font-mono"
					>// materialize</span
				> run.
			</p>
		{:else}
			<table class="w-full text-xs">
				<thead class="text-tertiary text-left">
					<tr>
						<th class="font-medium pb-1 pr-2">Partition</th>
						<th class="font-medium pb-1 pr-2">Status</th>
						<th class="font-medium pb-1 pr-2">Snapshot</th>
						<th class="font-medium pb-1 pr-2">Rows</th>
						<th class="font-medium pb-1">Materialized</th>
					</tr>
				</thead>
				<tbody>
					{#each partitions.current as p (p.partition)}
						<tr class="border-t">
							<td class="py-1 pr-2 font-mono">{p.partition || '(whole table)'}</td>
							<td class="py-1 pr-2">
								<span class="px-1.5 py-0.5 rounded text-3xs font-medium {statusClass[p.status]}">
									{p.status}
								</span>
							</td>
							<td class="py-1 pr-2 font-mono">{p.snapshot_id ?? '—'}</td>
							<td class="py-1 pr-2">{p.row_count ?? '—'}</td>
							<td class="py-1 text-tertiary">{new Date(p.materialized_at).toLocaleString()}</td>
						</tr>
						{#if p.error}
							<tr><td colspan="5" class="pb-1 text-3xs text-red-600 font-mono">{p.error}</td></tr>
						{/if}
					{/each}
				</tbody>
			</table>
		{/if}
	</div>
</div>

<BackfillRangeDialog
	bind:open={backfillOpen}
	assetPath={path}
	{workspace}
	slices={backfillSlices}
	running={backfillRunning}
	cancelRequested={backfillCancelRequested}
	onStart={startBackfill}
	onCancel={cancelBackfill}
	onReset={() => (backfillSlices = undefined)}
/>
