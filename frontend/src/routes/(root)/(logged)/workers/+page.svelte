<script lang="ts">
	import AssignableTags from '$lib/components/AssignableTags.svelte'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Button, Skeleton, Tab, Tabs } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import DefaultTags from '$lib/components/DefaultTags.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import QueueMetricsDrawer from '$lib/components/QueueMetricsDrawer.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import WorkspaceGroup from '$lib/components/WorkerGroup.svelte'
	import { WorkerService, type WorkerPing, ConfigService, SettingService } from '$lib/gen'
	import {
		enterpriseLicense,
		superadmin,
		userStore,
		workspaceStore,
		userWorkspaces
	} from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { displayDate, groupBy, pluralize, truncate } from '$lib/utils'
	import { AlertTriangle, LineChart, List, Plus, Search, Terminal, Unplug } from 'lucide-svelte'
	import { getContext, onDestroy, onMount } from 'svelte'
	import AutoComplete from 'simple-svelte-autocomplete'

	import YAML from 'yaml'
	import { DEFAULT_TAGS_WORKSPACES_SETTING } from '$lib/consts'
	import AutoscalingEvents from '$lib/components/AutoscalingEvents.svelte'
	import HttpAgentWorkerDrawer from '$lib/components/HttpAgentWorkerDrawer.svelte'

	let workers: WorkerPing[] | undefined = undefined
	let workerGroups: Record<string, any> | undefined = undefined
	let groupedWorkers: [string, [string, WorkerPing[]][]][] = []
	let intervalId: NodeJS.Timeout | undefined

	const splitter = '_%%%_'
	let customTags: string[] | undefined = undefined

	$: groupedWorkers = groupWorkers(workers, workerGroups)

	function groupWorkers(
		workers: WorkerPing[] | undefined,
		workerGroups: Record<string, any> | undefined
	): [string, [string, WorkerPing[]][]][] {
		if (!workers && !workerGroups) {
			return []
		}

		let grouped = groupBy(
			groupBy(
				workers ?? [],
				(wp: WorkerPing) => wp.worker_instance + splitter + wp.worker_group,
				(wp: WorkerPing) => wp.worker
			),
			(x) => x[0]?.split(splitter)?.[1],
			(x) => x[0]?.split(splitter)?.[0]
		).sort((a, b) => b[1].length - a[1].length)

		Object.keys(workerGroups ?? {}).forEach((group) => {
			if (!grouped.some((x) => x[0] == group)) {
				grouped.push([group, []])
			}
		})
		return grouped
	}
	let timeSinceLastPing = 0

	async function loadWorkers(): Promise<void> {
		try {
			workers = await WorkerService.listWorkers({ perPage: 1000, pingSince: 300 })
			timeSinceLastPing = 0
		} catch (err) {
			sendUserToast(`Could not load workers: ${err}`, true)
		}
	}

	async function loadWorkerGroups(): Promise<void> {
		try {
			workerGroups = Object.fromEntries(
				(await ConfigService.listWorkerGroups()).map((x) => [x.name, x.config])
			)
		} catch (err) {
			sendUserToast(`Could not load worker groups: ${err}`, true)
		}
	}

	let secondInterval: NodeJS.Timeout | undefined = undefined
	async function loadCustomTags() {
		try {
			customTags = (await WorkerService.getCustomTags()) ?? []
		} catch (err) {
			sendUserToast(`Could not load global cache: ${err}`, true)
		}
	}

	let defaultTagPerWorkspace: boolean | undefined = undefined
	let defaultTagWorkspaces: string[] | undefined = undefined
	async function loadDefaultTagsPerWorkspace() {
		try {
			defaultTagPerWorkspace = await WorkerService.isDefaultTagsPerWorkspace()
			defaultTagWorkspaces = (await SettingService.getGlobal({
				key: DEFAULT_TAGS_WORKSPACES_SETTING
			})) as any
		} catch (err) {
			sendUserToast(`Could not load default tag per workspace setting: ${err}`, true)
		}
	}

	onMount(() => {
		intervalId = setInterval(() => {
			loadWorkers()
			loadWorkerGroups()
		}, 5000)
		secondInterval = setInterval(() => {
			timeSinceLastPing += 1
		}, 1000)
	})

	loadWorkers()
	loadWorkerGroups()
	loadCustomTags()
	$: $superadmin && loadDefaultTagsPerWorkspace()

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

	let importConfigDrawer: Drawer | undefined = undefined
	let importConfigCode = ''

	async function importSingleWorkerConfig(c: any) {
		if (typeof c === 'object' && c !== null) {
			if (!c.name || typeof c.name !== 'string') {
				throw new Error('Invalid worker group config name')
			}

			if (workerGroups?.hasOwnProperty(c.name)) {
				throw new Error(`Worker group config with the name ${c.name} already exists`)
			}

			await ConfigService.updateConfig({
				name: 'worker__' + c.name,
				requestBody: { ...c, name: undefined }
			})
		} else {
			throw new Error('Invalid worker group config')
		}
	}

	async function importConfigFromYaml() {
		const config = YAML.parse(importConfigCode)

		try {
			if (Array.isArray(config)) {
				for (const c of config) {
					await importSingleWorkerConfig(c)
				}
			} else {
				await importSingleWorkerConfig(config)
			}
		} catch (err) {
			if (err instanceof Error) {
				sendUserToast(err.message, true)
			} else {
				console.error(err)
				sendUserToast('Could not import worker group config', true)
			}
			return
		}

		importConfigDrawer?.toggleDrawer?.()

		importConfigCode = ''

		sendUserToast('Worker group(s) config successfully imported')

		await loadWorkerGroups()
	}

	let queueMetricsDrawer: QueueMetricsDrawer
	let selectedTab: string = 'default'

	$: groupedWorkers && selectedTab == 'default' && updateSelectedTabIfDefaultDoesNotExist()

	function updateSelectedTabIfDefaultDoesNotExist() {
		if (selectedTab == 'default' && !groupedWorkers.some((x) => x[0] == 'default')) {
			selectedTab = Object.keys(workerGroups ?? {})[0] ?? 'default'
		}
	}

	let search: string = ''

	$: worker_group = filterWorkerGroupByNames(
		groupedWorkers?.find((x) => x?.[0] == selectedTab),
		search
	)

	function filterWorkerGroupByNames(
		worker_group: [string, [string, WorkerPing[]][]] | undefined,
		search: string
	): [string, [string, WorkerPing[]][]] | undefined {
		if (!worker_group) {
			return undefined
		}

		if (!search) {
			return worker_group
		}

		if (search === '') {
			return worker_group
		}

		const filteredWorkerGroup: [string, WorkerPing[]][] = worker_group[1]
			.map(
				([section, workers]) =>
					[
						section,
						workers.filter(
							(worker) =>
								worker.worker.toLowerCase().includes(search.toLowerCase()) ||
								worker.worker_instance.toLowerCase().includes(search.toLowerCase()) ||
								worker.ip.toLowerCase().includes(search.toLowerCase())
						)
					] as [string, WorkerPing[]]
			)
			.filter(([section, workers]) => workers.length > 0)

		return [worker_group[0], filteredWorkerGroup]
	}
	const openSearchWithPrefilledText: (t?: string) => void = getContext(
		'openSearchWithPrefilledText'
	)

	function displayOccupancyRate(occupancy_rate: number | undefined) {
		if (occupancy_rate == undefined) {
			return '--'
		}

		return Math.ceil(occupancy_rate * 100) + '%'
	}

	let newHttpAgentWorkerDrawer: Drawer | undefined = undefined
</script>

{#if $superadmin}
	<QueueMetricsDrawer bind:this={queueMetricsDrawer} />
{/if}

<Drawer bind:this={importConfigDrawer} size="800px">
	<DrawerContent
		title="Import groups config from YAML"
		on:close={() => importConfigDrawer?.toggleDrawer?.()}
	>
		<SimpleEditor
			bind:code={importConfigCode}
			lang="yaml"
			class="h-full"
			fixedOverflowWidgets={false}
		/>
		<svelte:fragment slot="actions">
			<Button size="sm" on:click={importConfigFromYaml} disabled={!importConfigCode}>Import</Button>
		</svelte:fragment>
	</DrawerContent>
</Drawer>

<Drawer bind:this={newHttpAgentWorkerDrawer} size="800px">
	<DrawerContent
		title="New HTTP agent worker"
		on:close={() => newHttpAgentWorkerDrawer?.toggleDrawer?.()}
	>
		<HttpAgentWorkerDrawer {customTags} />
	</DrawerContent>
</Drawer>

{#if $userStore?.operator && $workspaceStore && !$userWorkspaces.find((_) => _.id === $workspaceStore)?.operator_settings?.workers}
	<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4 m-4 mt-12" role="alert">
		<p class="font-bold">Unauthorized</p>
		<p>Page not available for operators</p>
	</div>
{:else}
	<CenteredPage>
		<PageHeader
			title="Workers"
			tooltip="The workers are the dutiful servants that execute the jobs."
			documentationLink="https://www.windmill.dev/docs/core_concepts/worker_groups"
		>
			{#if $superadmin}
				<div class="flex flex-row-reverse w-full pb-2 items-center gap-4">
					<div>
						<AssignableTags
							showWorkspaceRestriction
							on:refresh={() => {
								loadCustomTags()
							}}
						/>
					</div>
					<div>
						<DefaultTags bind:defaultTagPerWorkspace bind:defaultTagWorkspaces />
					</div>
					<div>
						<Button
							size="xs"
							color="dark"
							startIcon={{
								icon: LineChart
							}}
							on:click={() => {
								queueMetricsDrawer?.openDrawer()
							}}
						>
							Queue metrics
						</Button>
					</div>
					<div>
						<Button
							size="xs"
							color="dark"
							startIcon={{
								icon: List
							}}
							on:click={() => {
								openSearchWithPrefilledText('!')
							}}
						>
							Service logs
						</Button>
					</div>
				</div>
			{/if}
		</PageHeader>

		{#if workers != undefined}
			{#if groupedWorkers.length == 0}
				<p>No workers seem to be available</p>
			{/if}

			<div class="pt-4 pb-8 w-full flex justify-between items-center"
				><h4
					>{groupWorkers?.length} Worker Groups <Tooltip
						documentationLink="https://www.windmill.dev/docs/core_concepts/worker_groups"
						>Worker groups are groups of workers that share a config and are meant to be identical.
						Worker groups are meant to be used with tags. Tags can be assigned to scripts and flows
						and can be seen as dedicated queues. Only the corresponding
					</Tooltip></h4
				>
				<div></div>

				{#if $superadmin}
					<div class="flex flex-row gap-4 items-center">
						<Button
							size="sm"
							color="light"
							variant="border"
							startIcon={{ icon: Plus }}
							on:click={() => {
								newHttpAgentWorkerDrawer?.toggleDrawer?.()
							}}>New agent worker</Button
						>
						<Popover
							floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
							containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
						>
							<svelte:fragment slot="trigger">
								<div class="flex items-center gap-2">
									<Button
										size="sm"
										startIcon={{ icon: Plus }}
										nonCaptureEvent
										disabled={!$enterpriseLicense}
										dropdownItems={$enterpriseLicense
											? [
													{
														label: 'Copy groups config as YAML',
														onClick: () => {
															if (!workerGroups) {
																return sendUserToast('No worker groups found', true)
															}

															const workersConfig = Object.entries(workerGroups).map(
																([name, config]) => ({
																	name,
																	...config
																})
															)
															navigator.clipboard.writeText(YAML.stringify(workersConfig))
															sendUserToast('Worker groups config copied to clipboard as YAML')
														}
													},
													{
														label: 'Import groups config from YAML',
														onClick: () => {
															importConfigDrawer?.toggleDrawer?.()
														}
													}
												]
											: undefined}
									>
										<span class="hidden md:block"
											>New group config {!$enterpriseLicense ? '(EE)' : ''}</span
										>

										<Tooltip light>
											Worker Group configs are propagated to every workers in the worker group
										</Tooltip>
									</Button>
								</div>
							</svelte:fragment>
							<svelte:fragment slot="content">
								<div class="flex flex-col gap-2 p-4">
									<input
										class="mr-2 h-full"
										placeholder="New group name"
										bind:value={newConfigName}
									/>

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
							</svelte:fragment>
						</Popover>
					</div>
				{/if}</div
			>

			{#if (groupedWorkers ?? []).length > 5}
				<div class="flex gap-2 items-center">
					<div class="text-secondary text-sm">Worker group:</div>
					<AutoComplete
						noInputStyles
						items={groupedWorkers.map((x) => x[0])}
						bind:selectedItem={selectedTab}
						hideArrow={true}
						inputClassName={'flex !font-gray-600 !font-primary !bg-surface-primary"'}
						dropdownClassName="!text-sm !py-2 !rounded-sm  !border-gray-200 !border !shadow-md"
						className="!font-gray-600 !font-primary !bg-surface-primary"
					/>

					<!-- <select
					class="max-w-64"
					bind:value={selectedTab}
					on:change={() => {
						search = ''
					}}
				>
					{#each groupedWorkers.map((x) => x[0]) as name (name)}
						<option value={name}
							>{name} ({pluralize(
								groupedWorkers.find((x) => x[0] == name)?.[1].length ?? 0,
								'worker'
							)})
						</option>
					{/each}
				</select> -->
				</div>
			{:else}
				<Tabs bind:selected={selectedTab}>
					{#each groupedWorkers.map((x) => x[0]) as name (name)}
						{@const worker_group = groupedWorkers.find((x) => x[0] == name)}

						{#if worker_group}
							{@const activeWorkers = worker_group?.[1].flatMap((x) =>
								x[1]?.filter((y) => (y.last_ping ?? 0) < 15)
							)}
							<Tab value={worker_group[0]}>
								{`${worker_group[0]} - ${pluralize(activeWorkers?.length, 'worker')}`}
								<Tooltip>Number of workers active in the last 15s</Tooltip>
							</Tab>
						{:else}
							<Tab value={name}>
								{name} (0 worker)
							</Tab>
						{/if}
					{/each}
				</Tabs>
			{/if}

			<div>
				{#if worker_group}
					{@const config = (workerGroups ?? {})[worker_group[0]]}
					{@const activeWorkers = worker_group?.[1].flatMap((x) =>
						x[1]?.filter((y) => (y.last_ping ?? 0) < 15)
					)}
					<WorkspaceGroup
						{customTags}
						name={worker_group[0]}
						workers={worker_group[1]}
						{config}
						on:reload={() => {
							loadWorkerGroups()
						}}
						activeWorkers={activeWorkers?.length ?? 0}
						{defaultTagPerWorkspace}
					/>

					<div class="flex flex-row items-center gap-2 relative my-2">
						<input
							class="max-w-80 border rounded-md !pl-8"
							placeholder="Search workers by name..."
							autocomplete="off"
							bind:value={search}
						/>
						<Search class="absolute left-2 " size={14} />
					</div>
					{#if worker_group?.[1].length == 0 && search}
						<div class="text-xs text-tertiary">
							No workers found. Reset the search to see all workers.
						</div>
					{:else}
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
									<Cell head>Jobs ran</Cell>
									{#if (!config || config?.dedicated_worker == undefined) && $superadmin}
										<Cell head>Last job</Cell>
										<Cell head>Occupancy rate<br />(15s/5m/30m/ever)</Cell>
									{/if}
									<Cell head>Memory usage<br />(Windmill)</Cell>
									<Cell head>Limits</Cell>
									<Cell head>Version</Cell>
									<Cell head>Liveness</Cell>
									<Cell head last>Live Shell</Cell>
								</tr>
							</Head>
							<tbody class="divide-y">
								{#each worker_group[1] as [section, workers]}
									<tr class="border-t">
										<Cell
											first
											colspan={(!config || config?.dedicated_worker == undefined) && $superadmin
												? 11
												: 9}
											scope="colgroup"
											class="bg-surface-secondary/30 !py-1 border-b !text-xs"
										>
											<div class="flex flex-row w-full">
												<div class="min-w-64">
													Host:
													<span class="font-semibold">{section?.split(splitter)?.[0]}</span>
												</div>
												<span class="ml-4">IP: </span>
												<span class="font-semibold">{workers[0].ip}</span>

												{#if workers?.length > 1}
													<span class="font-semibold ml-8">{workers?.length} Workers</span>
												{/if}
											</div>
										</Cell>
									</tr>

									{#if workers}
										{#each workers as { worker, custom_tags, last_ping, started_at, jobs_executed, last_job_id, last_job_workspace_id, occupancy_rate_15s, occupancy_rate_5m, occupancy_rate_30m, occupancy_rate, wm_version, vcpus, memory, memory_usage, wm_memory_usage }}
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
												<Cell
													>{last_ping != undefined ? last_ping + timeSinceLastPing : -1}s ago</Cell
												>
												<Cell>{displayDate(started_at)}</Cell>
												<Cell>{jobs_executed}</Cell>
												{#if (!config || config?.dedicated_worker == undefined) && $superadmin}
													<Cell>
														{#if last_job_id}
															<a href={`/run/${last_job_id}?workspace=${last_job_workspace_id}`}>
																View last job
															</a>
															<br />
															(workspace {last_job_workspace_id})
														{/if}
													</Cell>
													<Cell>
														{displayOccupancyRate(occupancy_rate_15s)}/{displayOccupancyRate(
															occupancy_rate_5m
														)}/{displayOccupancyRate(occupancy_rate_30m)}/{displayOccupancyRate(
															occupancy_rate
														)}
													</Cell>
												{/if}
												<Cell>
													<div class="flex flex-col gap-1">
														<div>
															{memory_usage ? Math.round(memory_usage / 1024 / 1024) + 'MB' : '--'}
														</div>
														<div>
															({wm_memory_usage
																? Math.round(wm_memory_usage / 1024 / 1024) + 'MB'
																: '--'})
														</div>
													</div>
												</Cell>
												<Cell>
													<div class="flex flex-col gap-1">
														<div>
															{vcpus ? (vcpus / 100000).toFixed(2) + ' vCPUs' : '--'}
														</div>
														<div>
															{memory ? Math.round(memory / 1024 / 1024) + 'MB' : '--'}
														</div>
													</div>
												</Cell>
												<Cell>
													<div class="!text-2xs">
														{wm_version.split('-')[0]}<Tooltip>{wm_version}</Tooltip>
													</div>
												</Cell>
												<Cell>
													<Badge
														color={last_ping != undefined
															? last_ping < 60
																? 'green'
																: 'red'
															: 'gray'}
													>
														{last_ping != undefined
															? last_ping < 60
																? 'Alive'
																: 'Dead'
															: 'Unknown'}
													</Badge>
												</Cell>
												<Cell last>
													<Button
														size="xs"
														color="light"
														on:click={() => {}}
														startIcon={{ icon: Terminal }}
													>
														Execute Command
														<Tooltip>
															<p class="text-sm">
																Open a live shell to execute bash commands on the worker — useful
																for quick access, inspection, and real-time debugging
															</p>
														</Tooltip>
													</Button>
												</Cell>
											</tr>
										{/each}
									{/if}
								{/each}
							</tbody>
						</DataTable>
					{/if}
				{:else}
					{@const worker_group = Object.entries(workerGroups ?? {})
						.filter((x) => !groupedWorkers.some((y) => y[0] == x[0]))
						.find((x) => x[0] == selectedTab)}

					{#if worker_group}
						<WorkspaceGroup
							{customTags}
							workers={worker_group[1]}
							on:reload={() => {
								loadWorkerGroups()
							}}
							name={worker_group[0]}
							config={worker_group[1]}
							activeWorkers={0}
						/>
						<div class="text-xs text-tertiary"> No workers currently in this worker group </div>
					{/if}
				{/if}
			</div>
			<div class="pb-20"></div>
			<AutoscalingEvents worker_group={selectedTab} />
		{:else}
			<div class="flex flex-col">
				{#each new Array(4) as _}
					<Skeleton layout={[[8], 1]} />
				{/each}
			</div>
		{/if}
	</CenteredPage>
{/if}
