<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Button, Popup, Skeleton } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import WorkspaceGroup from '$lib/components/WorkspaceGroup.svelte'
	import { WorkerService, type WorkerPing, SettingService, ConfigService } from '$lib/gen'
	import { enterpriseLicense, superadmin } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { displayDate, groupBy, truncate } from '$lib/utils'
	import { AlertTriangle, Loader2, Pen, Plus, X } from 'lucide-svelte'
	import { onDestroy, onMount } from 'svelte'

	let workers: WorkerPing[] | undefined = undefined
	let filteredWorkers: WorkerPing[] = []
	let workerGroups: Record<string, any> | undefined = undefined
	let groupedWorkers: [string, [string, WorkerPing[]][]][] = []
	let intervalId: NodeJS.Timer | undefined

	const splitter = '_%%%_'
	let globalCache = false
	let customTags: string[] | undefined = []
	$: filteredWorkers = (workers ?? []).filter((x) => (x.last_ping ?? 0) < 300)
	$: groupedWorkers = groupBy(
		groupBy(
			filteredWorkers,
			(wp: WorkerPing) => wp.worker_instance + splitter + wp.worker_group,
			(wp: WorkerPing) => wp.worker
		),
		(x) => x[0]?.split(splitter)?.[1],
		(x) => x[0]?.split(splitter)?.[0]
	)

	const WORKER_S3_BUCKET_SYNC_SETTING = 'worker_s3_bucket_sync'
	const CUSTOM_TAGS_SETTING = 'custom_tags'

	let timeSinceLastPing = 0

	async function loadWorkers(): Promise<void> {
		try {
			workers = await WorkerService.listWorkers({ perPage: 1000 })
			timeSinceLastPing = 0
		} catch (err) {
			sendUserToast(`Could not load workers: ${err}`, true)
		}
	}

	async function loadWorkerGroups(): Promise<void> {
		try {
			workerGroups = Object.fromEntries(
				(await ConfigService.listWorkerGroups()).map((x) => [x.name.substring(8), x.config])
			)
		} catch (err) {
			sendUserToast(`Could not load worker groups: ${err}`, true)
		}
	}

	let secondInterval: NodeJS.Timer | undefined = undefined
	onMount(() => {
		loadWorkers()
		loadWorkerGroups()
		intervalId = setInterval(() => {
			loadWorkers()
			loadWorkerGroups()
		}, 5000)
		secondInterval = setInterval(() => {
			timeSinceLastPing += 1
		}, 1000)
	})

	$: if ($superadmin) {
		loadGlobalCache()
		loadCustomTags()
	}

	async function loadGlobalCache() {
		try {
			globalCache = (await SettingService.getGlobal({ key: WORKER_S3_BUCKET_SYNC_SETTING })) ?? true
		} catch (err) {
			sendUserToast(`Could not load global cache: ${err}`, true)
		}
	}

	async function loadCustomTags() {
		try {
			customTags = (await SettingService.getGlobal({ key: CUSTOM_TAGS_SETTING })) ?? []
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

	let newConfigName = ''

	async function addConfig() {
		await ConfigService.updateConfig({ name: 'worker__' + newConfigName, requestBody: {} })
		loadWorkerGroups()
	}

	let newTag: string = ''
</script>

<CenteredPage>
	<PageHeader
		title="Workers"
		tooltip="The workers are the dutiful servants that execute the jobs."
		documentationLink="https://www.windmill.dev/docs/core_concepts/worker_groups"
	>
		{#if $superadmin}
			<div class="flex flex-row-reverse w-full pb-2 items-center gap-4">
				<div class="flex gap-2 items-center">
					<Toggle
						checked={globalCache}
						on:change={async (e) => {
							try {
								console.log('Setting global cache to', e.detail)
								await SettingService.setGlobal({
									key: WORKER_S3_BUCKET_SYNC_SETTING,
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
					<Tooltip
						><p
							>global cache to s3 is an enterprise feature that enable workers to do fast cold start
							and share a single cache backed by s3 to ensure that even with a high number of
							workers, dependencies for python/deno/bun/go are only downloaded for the first time
							only once by the whole fleet.
						</p>require S3_CACHE_BUCKET to be set and has NO effect otherwise (even if this setting
						is on)</Tooltip
					>
				</div>
				<div
					><Popup
						floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
						containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
					>
						<svelte:fragment slot="button">
							<Button color="dark" size="xs" nonCaptureEvent={true}>
								<div class="flex flex-row gap-1 items-center"
									><Pen size={14} /> Assignable tags&nbsp;<Tooltip light
										>Tags are assigned to scripts and flows. Workers only accept jobs that
										correspond to their worker tags. Scripts have a default tag based on the
										language they are in but users can choose to override their tags with custom
										ones. This editor allow you to set the custom tags one can override the scripts
										and flows with.</Tooltip
									></div
								>
							</Button>
						</svelte:fragment>
						<div class="flex flex-col w-72 p-2 gap-2">
							{#if customTags == undefined}
								<Loader2 class="animate-spin" />
							{:else}
								<div class="flex flex-wrap gap-3 gap-y-2">
									{#each customTags as customTag}
										<div class="flex gap-0.5 items-center"
											><div class="text-2xs p-1 rounded border text-primary">{customTag}</div>
											<button
												class="z-10 rounded-full p-1 duration-200 hover:bg-gray-200"
												aria-label="Remove item"
												on:click|preventDefault|stopPropagation={async () => {
													await SettingService.setGlobal({
														key: CUSTOM_TAGS_SETTING,
														requestBody: { value: customTags?.filter((x) => x != customTag) }
													})
													loadCustomTags()
													sendUserToast('Tag removed')
												}}
											>
												<X size={12} />
											</button></div
										>
									{/each}
								</div>
								<input type="text" bind:value={newTag} />

								<Button
									variant="contained"
									color="blue"
									size="sm"
									on:click={async () => {
										await SettingService.setGlobal({
											key: CUSTOM_TAGS_SETTING,
											requestBody: {
												value: [...(customTags ?? []), newTag.trim().replaceAll(' ', '_')]
											}
										})
										loadCustomTags()
										sendUserToast('Tag added')
									}}
									disabled={newTag.trim() == ''}
								>
									Add
								</Button>
								<span class="text-2xs text-tertiary"
									>For tags specific to some workspaces, use <pre class="inline"
										>tag(workspace1+workspace2)</pre
									></span
								>
								<span class="text-2xs text-tertiary"
									>For dynamic tags based on the workspace, use <pre class="inline">$workspace</pre
									>, e.g:
									<pre class="inline">tag-$workspace</pre></span
								>
							{/if}
						</div>
					</Popup>
				</div>
			</div>
		{/if}
	</PageHeader>

	{#if workers != undefined}
		{#if groupedWorkers.length == 0}
			<p>No workers seems to be available</p>
		{/if}

		<div class="py-4 w-full flex justify-between"
			><h2
				>Worker Groups <Tooltip
					documentationLink="https://www.windmill.dev/docs/core_concepts/worker_groups"
					>Worker groups are groups of workers that share a config and are meant to be identical.
					Worker groups are meant to be used with tags. Tags can be assigned to scripts and flows
					and can be seen as dedicated queues. Only the corresponding
				</Tooltip></h2
			>
			<div />
			{#if $superadmin}
				<div class="flex flex-row items-center">
					<Popup
						floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
						containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
					>
						<svelte:fragment slot="button">
							<div class="flex items-center">
								<Button size="sm" startIcon={{ icon: Plus }} nonCaptureEvent>
									New worker group config
									<Tooltip light>
										Worker Group configs are propagated to every workers in the worker group
									</Tooltip>
								</Button>
							</div>
						</svelte:fragment>
						<div class="flex flex-col gap-2">
							<input class="mr-2 h-full" placeholder="New group name" bind:value={newConfigName} />

							{#if !$enterpriseLicense}
								<div class="flex items-center whitespace-nowrap text-yellow-600 gap-2">
									<AlertTriangle size={16} />
									EE only
								</div>
							{/if}
							<Button
								size="sm"
								startIcon={{ icon: Plus }}
								disabled={!newConfigName || !$enterpriseLicense}
								on:click={addConfig}
							>
								Create
							</Button>
						</div>
					</Popup>
				</div>
			{/if}</div
		>
		{#each groupedWorkers as worker_group (worker_group[0])}
			<WorkspaceGroup
				name={worker_group[0]}
				config={(workerGroups ?? {})[worker_group[0]]}
				on:reload={() => {
					loadWorkerGroups()
				}}
				activeWorkers={worker_group?.[1].flatMap((x) =>
					x[1]?.filter((y) => (y.last_ping ?? 0) < 15)
				)?.length ?? 0}
			/>

			<DataTable>
				<Head>
					<tr>
						<Cell head first>Worker</Cell>
						<Cell head>
							<div class="flex flex-row items-center gap-1">
								Worker Tags
								<Tooltip
									documentationLink="https://www.windmill.dev/docs/core_concepts/worker_groups#assign-custom-worker-groups"
								>
									If defined, the workers only pull jobs with the same corresponding tag
								</Tooltip>
							</div>
						</Cell>
						<Cell head>Last ping</Cell>
						<Cell head>Worker start</Cell>
						<Cell head>Nb of jobs executed</Cell>
						<Cell head>Version</Cell>
						<Cell head last>Liveness</Cell>
					</tr>
				</Head>
				<tbody>
					{#each worker_group[1] as [section, workers]}
						<tr class="border-t">
							<Cell
								first
								colspan="7"
								scope="colgroup"
								class="bg-surface-secondary/60 py-2 border-b"
							>
								Instance: <Badge color="gray">{section?.split(splitter)?.[0]}</Badge>
								IP: <Badge color="gray">{workers[0].ip}</Badge>
								{#if workers?.length > 1}
									{workers?.length} Workers
								{/if}
							</Cell>
						</tr>

						{#if workers}
							{#each workers as { worker, custom_tags, last_ping, started_at, jobs_executed, wm_version }}
								<tr>
									<Cell first>{worker}</Cell>
									<Cell>
										{#if custom_tags && custom_tags?.length > 2}
											{truncate(custom_tags?.join(', ') ?? '', 10)}
											<Tooltip>{custom_tags?.join(', ')}</Tooltip>
										{:else}
											{custom_tags?.join(', ') ?? ''}
										{/if}
									</Cell>
									<Cell>{last_ping != undefined ? last_ping + timeSinceLastPing : -1}s ago</Cell>
									<Cell>{displayDate(started_at)}</Cell>
									<Cell>{jobs_executed}</Cell>
									<Cell
										><div class="!text-2xs"
											>{wm_version.split('-')[0]}<Tooltip>{wm_version}</Tooltip></div
										></Cell
									>
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
			<div class="pb-4" />
		{/each}

		<div class="pb-4" />

		{#each Object.entries(workerGroups ?? {}).filter((x) => !groupedWorkers.some((y) => y[0] == x[0])) as worker_group (worker_group[0])}
			<WorkspaceGroup
				on:reload={() => {
					loadWorkerGroups()
				}}
				name={worker_group[0]}
				config={worker_group[1]}
				activeWorkers={0}
			/>
			<div class="text-xs text-tertiary"> No workers currently in this worker group </div>
		{/each}
	{:else}
		<div class="flex flex-col">
			{#each new Array(4) as _}
				<Skeleton layout={[[8], 1]} />
			{/each}
		</div>
	{/if}
</CenteredPage>
