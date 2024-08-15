<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Badge, Button } from '$lib/components/common'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import { Plus } from 'lucide-svelte'

	const workers: {
		method: string
		route: string
		path: string
	}[] = [
		{
			method: 'GET',
			route: '/api/v1/{:id}/flows',
			path: 'flows'
		},
		{
			method: 'POST',
			route: '/api/v1/flows',
			path: 'flows'
		},
		{
			method: 'GET',
			route: '/api/v1/flows/:id',
			path: 'flows'
		},
		{
			method: 'PUT',
			route: '/api/v1/flows/:id',
			path: 'flows'
		},
		{
			method: 'DELETE',
			route: '/api/v1/flows/:id',
			path: 'flows'
		}
	]
</script>

<CenteredPage>
	<PageHeader
		title="Routing"
		tooltip="Trigger Scripts and Flows according to a cron schedule"
		documentationLink="https://www.windmill.dev/docs/core_concepts/routing"
	>
		<Button size="md" startIcon={{ icon: Plus }} on:click={() => {}}>New&nbsp;route</Button>
	</PageHeader>
	<div class="w-full h-full flex flex-col">
		<DataTable>
			<Head>
				<tr>
					<Cell head first class="w-24">Method</Cell>
					<Cell head>Path</Cell>
					<Cell head last>Flow/Script</Cell>
				</tr>
			</Head>
			<tbody class="divide-y">
				{#each workers as x}
					<tr>
						<Cell first class="!w-24">
							<select value={x.method} class="!w-24">
								<option value="GET" class="!text-green-500">GET</option>
								<option value="POST">POST</option>
								<option value="PUT">PUT</option>
								<option value="DELETE">DELETE</option>
							</select>
						</Cell>
						<Cell>
							<Badge small>
								{x.route}
							</Badge>
						</Cell>
						<Cell last>
							<Badge small>
								{x.path}
							</Badge>
						</Cell>
					</tr>
				{/each}
			</tbody>
		</DataTable>
	</div>
</CenteredPage>
