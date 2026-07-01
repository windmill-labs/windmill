<script lang="ts">
	import { resource } from 'runed'
	import { OpenAPI, AssetService, type MaterializedPartition } from '$lib/gen'
	import { Button } from '$lib/components/common'
	import { Loader2, RefreshCw, History } from 'lucide-svelte'
	import { sendUserToast } from '$lib/utils'
	import BackfillRangeDialog from './BackfillRangeDialog.svelte'

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

	const statusClass: Record<MaterializedPartition['status'], string> = {
		materialized: 'bg-green-100 text-green-800 dark:bg-green-900/40 dark:text-green-300',
		running: 'bg-blue-100 text-blue-800 dark:bg-blue-900/40 dark:text-blue-300',
		failed: 'bg-red-100 text-red-800 dark:bg-red-900/40 dark:text-red-300'
	}

	async function onBackfill(from: string, to: string) {
		// The fan-out runner is an enterprise feature; the dialog only submits
		// when licensed. This posts the intent to the (EE) backfill endpoint — a raw
		// fetch since it's not in the OSS OpenAPI, so carry the bearer token
		// explicitly (matches how `/pipeline_dev` authenticates: no session cookie).
		const token = OpenAPI.TOKEN
		const authHeader: Record<string, string> =
			typeof token === 'string' && token ? { Authorization: `Bearer ${token}` } : {}
		const res = await fetch(`${OpenAPI.BASE ?? ''}/w/${workspace}/assets/backfill`, {
			method: 'POST',
			credentials: 'include',
			headers: { 'content-type': 'application/json', ...authHeader },
			body: JSON.stringify({ path, from, to })
		})
		if (!res.ok) throw new Error(`backfill → ${res.status}`)
		sendUserToast(`Backfill queued for ${path} (${from} → ${to})`)
		await partitions.refetch()
	}
</script>

<div class="flex flex-col h-full">
	<div class="flex items-center justify-between gap-2 px-3 py-2 border-b shrink-0">
		<span class="text-xs font-semibold text-secondary">Materialized partitions</span>
		<div class="flex items-center gap-1">
			<Button
				variant="subtle"
				unifiedSize="sm"
				startIcon={{ icon: RefreshCw }}
				iconOnly
				onclick={() => partitions.refetch()}
				title="Refresh"
			/>
			<!-- The backfill range runner (POST /assets/backfill) is a planned
			     enterprise follow-up and not yet implemented on the backend, so the
			     button stays disabled — enabling it would 404. Re-enable when the
			     endpoint lands. -->
			<Button
				variant="default"
				unifiedSize="sm"
				startIcon={{ icon: History }}
				disabled
				onclick={() => (backfillOpen = true)}
				title="Backfill a range of partitions — coming soon (enterprise)"
			>
				Backfill
			</Button>
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

<BackfillRangeDialog bind:open={backfillOpen} assetPath={path} {onBackfill} />
