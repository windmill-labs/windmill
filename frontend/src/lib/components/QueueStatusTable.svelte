<script lang="ts">
	import { WorkerService, type GetQueueStatusResponse } from '$lib/gen'
	import { RefreshCw, TriangleAlert } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { Alert, Button, Section, Skeleton } from './common'
	import DataTable from './table/DataTable.svelte'
	import Head from './table/Head.svelte'
	import Cell from './table/Cell.svelte'
	import { msToReadableTime } from '$lib/utils'
	import { onMount } from 'svelte'

	const REFRESH_MS = 10_000

	let status = $state<GetQueueStatusResponse>()
	let loading = $state(false)
	let error = $state<string>()

	async function load() {
		loading = true
		try {
			status = await WorkerService.getQueueStatus()
			error = undefined
		} catch (e) {
			error = e instanceof Error ? e.message : String(e)
		} finally {
			loading = false
		}
	}

	onMount(() => {
		load()
		const interval = setInterval(load, REFRESH_MS)
		return () => clearInterval(interval)
	})

	type TagStatus = GetQueueStatusResponse[number]

	// A backlog nobody listens to never drains, so it leads the table.
	function unserved(s: TagStatus) {
		return s.waiting > 0 && s.workers === 0
	}

	const rows = $derived(
		[...(status ?? [])].sort(
			(a, b) =>
				Number(unserved(b)) - Number(unserved(a)) ||
				(b.delay ?? -1) - (a.delay ?? -1) ||
				b.running - a.running ||
				a.tag.localeCompare(b.tag)
		)
	)
</script>

<Section
	label="Queue status"
	tooltip="Waiting counts jobs due for more than 3 seconds that no worker has picked up. Next job's wait is how long the job the next pull would take has been waiting. Workers counts the workers that pinged in the last minute and pull the tag."
>
	{#snippet action()}
		<Button
			variant="subtle"
			unifiedSize="sm"
			startIcon={{ icon: RefreshCw, classes: twMerge(loading ? 'animate-spin' : '') }}
			iconOnly
			onclick={load}
			disabled={loading}
		/>
	{/snippet}

	{#if error}
		<Alert type="error" title="Failed to load the queue status">{error}</Alert>
	{:else if status === undefined}
		<Skeleton layout={[[6]]} />
	{:else if rows.length === 0}
		<p class="text-secondary text-xs">No jobs are waiting or running.</p>
	{:else}
		<DataTable size="sm" noBorder={false} rounded={true}>
			<Head>
				<tr>
					<Cell head first>Tag</Cell>
					<Cell head numeric>Waiting</Cell>
					<Cell head numeric>Next job's wait</Cell>
					<Cell head numeric>Running</Cell>
					<Cell head last>Workers</Cell>
				</tr>
			</Head>
			<tbody>
				{#each rows as s (s.tag)}
					<tr class="border-b last:border-b-0">
						<Cell first class="text-xs font-mono text-primary">{s.tag}</Cell>
						<Cell numeric class="text-xs text-primary">{s.waiting}</Cell>
						<Cell numeric class="text-xs text-primary">
							{s.delay === undefined ? '-' : msToReadableTime(s.delay * 1000, 0)}
						</Cell>
						<Cell numeric class="text-xs text-primary">{s.running}</Cell>
						<Cell last class="text-xs text-primary">
							{#if unserved(s)}
								<span class="inline-flex items-center gap-1 text-yellow-600 dark:text-yellow-400">
									<TriangleAlert size={14} />
									None: add the tag to a worker group, or cancel its jobs
								</span>
							{:else}
								{s.workers}
							{/if}
						</Cell>
					</tr>
				{/each}
			</tbody>
		</DataTable>
	{/if}
</Section>
