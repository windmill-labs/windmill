<script lang="ts">
	import Cell from '$lib/components/table/Cell.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import type { Job } from '$lib/gen'
	import NoItemFound from '../home/NoItemFound.svelte'
	import JobDetail from '../jobs/JobDetail.svelte'

	export let jobs: Job[] = []
	export let pageIndex: number = 1
	export let perPage: number = 100
	export let selectedId: string | undefined = undefined
</script>

<DataTable
	currentPage={pageIndex}
	paginated
	rounded={false}
	size="sm"
	bind:perPage
	on:next={() => pageIndex++}
	on:prev={() => pageIndex--}
	shouldHidePagination={jobs?.length === 0}
>
	<Head>
		<Cell first head>Path</Cell>
		<Cell head>User</Cell>
		<Cell head>Status</Cell>
		<Cell head last />
	</Head>

	<tbody class="divide-y">
		{#each jobs as job (job.id)}
			<JobDetail {job} bind:selectedId />
		{/each}
	</tbody>
	{#if jobs.length == 0}
		<NoItemFound />
	{/if}
</DataTable>
