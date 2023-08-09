<script lang="ts">
	import Cell from '$lib/components/table/Cell.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import type { Job } from '$lib/gen'
	import NoItemFound from '../home/NoItemFound.svelte'
	import JobDetail from '../jobs/JobDetail.svelte'

	export let jobs: Job[] = []
	export let selectedId: string | undefined = undefined
	export let nbObJobs: number = 30
	export let loadMoreQuantity: number = 30
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
		<Cell first head />
		<Cell head>Timestamp</Cell>
		<Cell head>Path</Cell>
		<Cell head>User</Cell>
		<Cell head last>Schedule</Cell>
	</Head>

	<tbody class="divide-y">
		{#each jobs.slice(0, nbObJobs) as job (job.id)}
			<JobDetail {job} bind:selectedId />
		{/each}
	</tbody>
	{#if jobs.length == 0}
		<NoItemFound />
	{/if}
</DataTable>
