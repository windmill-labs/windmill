<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import { WorkerService, type WorkerPing } from '$lib/gen'
	import { displayDate, elapsedSinceSecs, groupBy, sendUserToast } from '$lib/utils'
	import { onDestroy, onMount } from 'svelte'

	let workers: WorkerPing[] = []
	let filteredWorkers: WorkerPing[] = []
	let groupedWorkers: [string, WorkerPing[]][] = []
	let intervalId: NodeJS.Timer | undefined

	$: filteredWorkers = workers.filter((x) => elapsedSinceSecs(x.ping_at) < 300)
	$: groupedWorkers = groupBy(filteredWorkers, (wp: WorkerPing) => wp.worker_instance)

	async function loadWorkers(): Promise<void> {
		try {
			workers = await WorkerService.listWorkers({ perPage: 100 })
		} catch (err) {
			sendUserToast(`Could not load workers: ${err}`, true)
		}
	}

	onMount(() => {
		loadWorkers()
		intervalId = setInterval(loadWorkers, 5000)
	})

	onDestroy(() => {
		if (intervalId) {
			clearInterval(intervalId)
		}
	})
</script>

<CenteredPage>
	<PageHeader
		title="Workers"
		tooltip="The workers are the dutiful servants that execute your scripts.
		 This page enables you to know their IP in case you need whitelisting and also display liveness information"
	/>

	{#if groupedWorkers.length == 0}
		<p>No workers seems to be available</p>
	{/if}
	{#each groupedWorkers as [section, workers]}
		<div class="mt-6">
			Instance: {section} | IP:
			<Badge large color="gray">{workers[0].ip}</Badge>
		</div>

		<TableCustom>
			<tr slot="header-row">
				<th>Worker</th>
				<th>Last ping at</th>
				<th>Worker start</th>
				<th>Nb of jobs executed</th>
				<th>Liveness</th>
			</tr>
			<tbody slot="body">
				{#if workers}
					{#each workers as { worker, ping_at, started_at, jobs_executed }}
						<tr>
							<td class="py-1">{worker}</td>
							<td class="py-1">{elapsedSinceSecs(ping_at)}s ago</td>
							<td class="py-1">{displayDate(started_at)}</td>
							<td class="py-1">{jobs_executed}</td>
							<td class="py-1">{elapsedSinceSecs(ping_at) < 60 ? 'Alive' : 'Dead'}</td>
						</tr>
					{/each}
				{/if}
			</tbody>
		</TableCustom>
	{/each}
</CenteredPage>
