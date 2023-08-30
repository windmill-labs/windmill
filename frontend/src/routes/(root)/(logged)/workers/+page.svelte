<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Skeleton } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { WorkerService, type WorkerPing, SettingService } from '$lib/gen'
	import { enterpriseLicense } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { displayDate, groupBy } from '$lib/utils'
	import { onDestroy, onMount } from 'svelte'

	let workers: WorkerPing[] | undefined = undefined
	let filteredWorkers: WorkerPing[] = []
	let groupedWorkers: [string, WorkerPing[]][] = []
	let intervalId: NodeJS.Timer | undefined

	let globalCache = false
	$: filteredWorkers = (workers ?? []).filter((x) => (x.last_ping ?? 0) < 300)
	$: groupedWorkers = groupBy(
		filteredWorkers,
		(wp: WorkerPing) => wp.worker_instance,
		(wp: WorkerPing) => wp.worker
	)

	const worker_s3_bucket_sync = 'worker_s3_bucket_sync'
	let timeSinceLastPing = 0

	async function loadWorkers(): Promise<void> {
		try {
			workers = await WorkerService.listWorkers({ perPage: 100 })
			timeSinceLastPing = 0
		} catch (err) {
			sendUserToast(`Could not load workers: ${err}`, true)
		}
	}

	let secondInterval: NodeJS.Timer | undefined = undefined
	onMount(() => {
		loadWorkers()
		intervalId = setInterval(loadWorkers, 5000)
		secondInterval = setInterval(() => {
			timeSinceLastPing += 1
		}, 1000)
		loadGlobalCache()
	})

	async function loadGlobalCache() {
		try {
			globalCache = (await SettingService.getGlobal({ key: worker_s3_bucket_sync })) ?? false
		} catch (err) {
			sendUserToast(`Could not load global cache: ${err}`, true)
		}
	}

	onDestroy(() => {
		if (intervalId) {
			clearInterval(intervalId)
		}
		if (secondInterval) {
			clearInterval(secondInterval)
		}
	})
</script>

<CenteredPage>
	<PageHeader
		title="Workers"
		tooltip="The workers are the dutiful servants that execute your scripts.
		 This page enables you to know their IP in case you need whitelisting and also display liveness information"
		documentationLink="https://www.windmill.dev/docs/core_concepts/worker_groups"
	/>

	<div class="flex flex-row-reverse w-full pb-2 items-center gap-2">
		<Tooltip
			>global cache to s3 is an enterprise feature that enable workers to do fast cold start and
			share a single cache backed by s3 to ensure that even with a high number of workers,
			dependencies for python/deno/bun/go are only downloaded for the first time only once by the
			whole fleet. require S3_CACHE_BUCKET to be set.</Tooltip
		>
		<Toggle
			checked={globalCache}
			on:change={async (e) => {
				try {
					console.log('Setting global cache to', e.detail)
					await SettingService.setGlobal({
						key: worker_s3_bucket_sync,
						requestBody: { value: e.detail }
					})
					globalCache = e.detail
				} catch (err) {
					sendUserToast(`Could not set global cache: ${err}`, true)
				}
			}}
			options={{ right: 'global cache to s3' }}
			disabled={!$enterpriseLicense}
		/>
	</div>
	{#if workers != undefined}
		{#if groupedWorkers.length == 0}
			<p>No workers seems to be available</p>
		{/if}

		<DataTable>
			<Head>
				<tr>
					<Cell head first>Worker</Cell>
					<Cell head>
						<div class="flex flex-row items-center gap-1">
							Custom Tags
							<Tooltip
								light
								documentationLink="https://www.windmill.dev/docs/core_concepts/worker_groups#assign-custom-worker-groups"
							>
								If defined, the workers only pull jobs with the same corresponding tag
							</Tooltip>
						</div>
					</Cell>
					<Cell head>Last ping</Cell>
					<Cell head>Worker start</Cell>
					<Cell head>Nb of jobs executed</Cell>
					<Cell head last>Liveness</Cell>
				</tr>
			</Head>
			<tbody>
				{#each groupedWorkers as [section, workers]}
					<tr class="border-t">
						<Cell first colspan="6" scope="colgroup" class="bg-surface-secondary/60 py-2 border-b">
							Instance: <Badge color="gray">{section}</Badge>
							IP: <Badge color="gray">{workers[0].ip}</Badge>
						</Cell>
					</tr>

					{#if workers}
						{#each workers as { worker, custom_tags, last_ping, started_at, jobs_executed }}
							<tr>
								<Cell first>{worker}</Cell>
								<Cell>{custom_tags?.join(', ') ?? ''}</Cell>
								<Cell>{last_ping != undefined ? last_ping + timeSinceLastPing : -1}s ago</Cell>
								<Cell>{displayDate(started_at)}</Cell>
								<Cell>{jobs_executed}</Cell>
								<Cell last>
									<Badge
										color={last_ping != undefined ? (last_ping < 60 ? 'green' : 'red') : 'gray'}
									>
										{last_ping != undefined ? (last_ping < 60 ? 'Alive' : 'Dead') : 'Unknown'}
									</Badge>
								</Cell>
							</tr>
						{/each}
					{/if}
				{/each}
			</tbody>
		</DataTable>
	{:else}
		<div class="flex flex-col">
			{#each new Array(4) as _}
				<Skeleton layout={[[8], 1]} />
			{/each}
		</div>
	{/if}
</CenteredPage>
