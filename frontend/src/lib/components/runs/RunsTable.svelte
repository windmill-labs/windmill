<script lang="ts">
	import Cell from '$lib/components/table/Cell.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import type { Job } from '$lib/gen'
	import RunRow from './RunRow.svelte'

	export let jobs: Job[] = []
	export let selectedId: string | undefined = undefined
	export let nbObJobs: number = 30
	export let loadMoreQuantity: number = 30

	function groupJobsByDay(jobs: Job[]): Record<string, Job[]> {
		const groupedLogs: Record<string, Job[]> = {}

		if (!jobs) return groupedLogs

		for (const job of jobs) {
			if ('started_at' in job && job.started_at) {
				const date = new Date(job.started_at)
				date.setMilliseconds(date.getMilliseconds() + (job['duration_ms'] ?? 0))

				const day = date.toLocaleDateString('en-US', {
					year: 'numeric',
					month: 'long',
					day: 'numeric'
				})

				if (!groupedLogs[day]) {
					groupedLogs[day] = []
				}

				groupedLogs[day].push(job)
			}
		}

		for (const day in groupedLogs) {
			groupedLogs[day].sort((a, b) => {
				return new Date(b.started_at!).getTime() - new Date(a.started_at!).getTime()
			})
		}

		const sortedLogs: Record<string, Job[]> = {}
		Object.keys(groupedLogs)
			.sort((a, b) => {
				return new Date(b).getTime() - new Date(a).getTime()
			})
			.forEach((key) => {
				sortedLogs[key] = groupedLogs[key]
			})

		return sortedLogs
	}

	$: groupedJobs = groupJobsByDay(jobs.slice(0, nbObJobs))
</script>

<DataTable
	rounded={false}
	size="sm"
	on:loadMore={() => console.log('load more')}
	loadMore={loadMoreQuantity}
	shouldLoadMore={nbObJobs < jobs.length}
	on:loadMore={() => (nbObJobs += loadMoreQuantity)}
>
	<Head>
		<Cell first head class="w-8" />
		<Cell head>Timestamp</Cell>
		<Cell head>Path</Cell>
		<Cell head last>Triggered by</Cell>
	</Head>

	<tbody class="divide-y">
		{#each Object.entries(groupedJobs) as [date, jobsByDay]}
			<tr class="border-t">
				<Cell
					first
					colspan="6"
					scope="colgroup"
					class="bg-surface-secondary/30 py-2 border-b font-semibold"
				>
					{date}
				</Cell>
			</tr>
			{#each jobsByDay as job (job.id)}
				<RunRow {job} bind:selectedId on:select />
			{/each}
		{/each}
	</tbody>
	{#if jobs.length == 0}
		<tr>
			<td colspan="4" class="text-center py-8">
				<div class="text-xs text-secondary"> No jobs found for the selected filters. </div>
			</td>
		</tr>
	{/if}
</DataTable>
