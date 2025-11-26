<script lang="ts">
	import AssignableTags from '$lib/components/AssignableTags.svelte'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Alert, Button, Skeleton, Tab, Tabs } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
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
		devopsRole,
		userStore,
		workspaceStore,
		userWorkspaces
	} from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { displayDate, groupBy, pluralize, retrieveCommonWorkerPrefix, truncate } from '$lib/utils'
	import { AlertTriangle, LineChart, List, Plus, Search, Terminal } from 'lucide-svelte'
	import { getContext, onDestroy, onMount, untrack } from 'svelte'

	import YAML from 'yaml'
	import { DEFAULT_TAGS_WORKSPACES_SETTING } from '$lib/consts'
	import AutoscalingEvents from '$lib/components/AutoscalingEvents.svelte'
	import HttpAgentWorkerDrawer from '$lib/components/HttpAgentWorkerDrawer.svelte'
	import WorkerRepl from '$lib/components/WorkerRepl.svelte'
	import Select from '$lib/components/select/Select.svelte'

	let workers: WorkerPing[] | undefined = $state(undefined)
	let workerGroups: Record<string, any> | undefined = $state(undefined)
	let groupedWorkers = $derived.by(() => groupWorkers(workers, workerGroups))
	let intervalId: number | undefined
	const splitter = '_%%%_'
	let customTags: string[] | undefined = $state(undefined)

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
	let timeSinceLastPing = $state(0)

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

	let secondInterval: number | undefined = undefined
	async function loadCustomTags() {
		try {
			customTags = (await WorkerService.getCustomTags()) ?? []
		} catch (err) {
			sendUserToast(`Could not load global cache: ${err}`, true)
		}
	}

	let defaultTagPerWorkspace: boolean | undefined = $state(undefined)
	let defaultTagWorkspaces: string[] = $state([])
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

	onDestroy(() => {
		if (intervalId) {
			clearInterval(intervalId)
		}
		if (secondInterval) {
			clearInterval(secondInterval)
		}
	})

	let newConfigName = $state('')

	async function addConfig() {
		await ConfigService.updateConfig({ name: 'worker__' + newConfigName, requestBody: {} })
		newGroupPopover?.close()
		loadWorkerGroups()
	}

	let importConfigDrawer: Drawer | undefined = $state(undefined)
	let importConfigCode = $state('')
	let tag: string = $state('')
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

	let queueMetricsDrawer: QueueMetricsDrawer | undefined = $state(undefined)
	let selectedTab: string = $state('default')

	function updateSelectedTabIfDefaultDoesNotExist() {
		if (
			selectedTab == 'default' &&
			!groupedWorkers.some((x) => x[0] == 'default') &&
			Object.keys(workerGroups ?? {})[0]
		) {
			selectedTab = Object.keys(workerGroups ?? {})[0]
		}
	}

	let search: string = $state('')

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
	let newHttpAgentWorkerDrawer: Drawer | undefined = $state(undefined)
	let replForWorkerDrawer: Drawer | undefined = $state(undefined)

	function isWorkerMaybeAlive(last_ping: number | undefined): boolean | undefined {
		return last_ping != undefined ? last_ping < 60 : undefined
	}

	let newGroupPopover: Popover | undefined = $state(undefined)

	$effect(() => {
		;($superadmin || $devopsRole) && loadDefaultTagsPerWorkspace()
	})

	$effect(() => {
		groupedWorkers &&
			selectedTab == 'default' &&
			untrack(() => updateSelectedTabIfDefaultDoesNotExist())
	})
	let worker_group = $derived(
		filterWorkerGroupByNames(
			groupedWorkers?.find((x) => x?.[0] == selectedTab),
			search
		)
	)
</script>

{#if $superadmin || $devopsRole}
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
		{#snippet actions()}
			<Button
				unifiedSize="md"
				variant="accent"
				onClick={importConfigFromYaml}
				disabled={!importConfigCode}>Import</Button
			>
		{/snippet}
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

<Drawer bind:this={replForWorkerDrawer} size="1000px">
	<DrawerContent
		title="Repl"
		on:close={() => {
			tag = ''
			replForWorkerDrawer?.closeDrawer?.()
		}}
	>
		<div class="flex flex-col gap-2">
			<Alert title="Info" type="info" size="xs">
				If no command has been run in the past 2 minutes, the next one may take up to 15 seconds to
				start.
			</Alert>
			<WorkerRepl {tag} />
		</div>
	</DrawerContent>
</Drawer>

{#if $userStore?.operator && $workspaceStore && !$userWorkspaces.find((_) => _.id === $workspaceStore)?.operator_settings?.workers}
	<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4 m-4 mt-12" role="alert">
		<p class="font-bold">Unauthorized</p>
		<p>Page not available for operators</p>
	</div>
{:else}
	<CenteredPage>
		{#snippet children({ width })}
			<PageHeader
				title="Workers"
				tooltip="The workers are the dutiful servants that execute the jobs."
				documentationLink="https://www.windmill.dev/docs/core_concepts/worker_groups"
			>
				{#if $superadmin || $devopsRole}
					<div class="flex flex-row-reverse w-full pb-2 items-center gap-4">
						<div>
							<AssignableTags
								on:refresh={() => {
									loadCustomTags()
								}}
								variant="default"
							/>
						</div>
						<div>
							<DefaultTags bind:defaultTagPerWorkspace bind:defaultTagWorkspaces />
						</div>
						<div>
							<Button
								unifiedSize="md"
								variant="default"
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
								unifiedSize="md"
								variant="default"
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
							>Worker groups are groups of workers that share a config and are meant to be
							identical. Worker groups are meant to be used with tags. Tags can be assigned to
							scripts and flows and can be seen as dedicated queues. Only the corresponding
						</Tooltip></h4
					>
					<div></div>

					{#if $superadmin || $devopsRole}
						<div class="flex flex-row gap-4 items-center">
							<Button
								unifiedSize="md"
								variant="default"
								startIcon={{ icon: Plus }}
								on:click={() => {
									newHttpAgentWorkerDrawer?.toggleDrawer?.()
								}}>New agent worker</Button
							>
							<Popover
								floatingConfig={{ strategy: 'absolute', placement: 'bottom-start' }}
								containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
								onKeyDown={(e) => {
									if (
										e.key === 'Enter' &&
										newConfigName &&
										newConfigName.trim() !== '' &&
										$enterpriseLicense
									) {
										addConfig()
									}
								}}
								onClose={() => {
									newConfigName = ''
								}}
								bind:this={newGroupPopover}
								targetId="new-group-popover-trigger"
							>
								{#snippet trigger()}
									<div class="flex items-center gap-2">
										<Button
											id="new-group-popover-trigger"
											variant="accent"
											unifiedSize="md"
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
								{/snippet}
								{#snippet content()}
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
											unifiedSize="md"
											variant="accent"
											startIcon={{ icon: Plus }}
											disabled={!newConfigName || !$enterpriseLicense}
											on:click={addConfig}
										>
											Create
										</Button>
									</div>
								{/snippet}
							</Popover>
						</div>
					{/if}</div
				>

				{#if (groupedWorkers ?? []).length > 5}
					<div class="flex gap-2 items-center">
						<div class="text-secondary text-sm">Worker group:</div>
						<Select items={groupedWorkers.map((x) => ({ value: x[0] }))} bind:value={selectedTab} />
					</div>
				{:else}
					<Tabs bind:selected={selectedTab}>
						{#each groupedWorkers.map((x) => x[0]) as name (name)}
							{@const worker_group = groupedWorkers.find((x) => x[0] == name)}

							{#if worker_group}
								{@const activeWorkers = worker_group?.[1].flatMap((x) =>
									x[1]?.filter((y) => (y.last_ping ?? 0) < 15)
								)}
								<Tab
									value={worker_group[0]}
									label={`${worker_group[0]} - ${pluralize(activeWorkers?.length, 'worker')}`}
								>
									{#snippet extra()}
										<Tooltip>Number of workers active in the last 15s</Tooltip>
									{/snippet}
								</Tab>
							{:else}
								<Tab value={name} label={`${name} (0 worker)`} />
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
							{width}
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
							<div class="text-xs text-primary">
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
										{#if (!config || config?.dedicated_worker == undefined) && ($superadmin || $devopsRole)}
											<Cell head>Last job</Cell>
											<Cell head>Occupancy rate<br />(15s/5m/30m/ever)</Cell>
										{/if}
										<Cell head>Memory usage<br />(Windmill)</Cell>
										<Cell head>Limits</Cell>
										<Cell head>Version</Cell>
										<Cell head>Liveness</Cell>
										{#if $superadmin || $devopsRole}
											<Cell head>
												Live Shell
												<Tooltip>
													<p class="text-sm">
														Open a live shell to execute bash commands on the machine where the
														worker runs — useful for quick access, inspection, and real-time
														debugging
													</p>
												</Tooltip>
											</Cell>
										{/if}
									</tr>
								</Head>
								<tbody>
									{#each worker_group[1] as [section, workers], groupIdx}
										{@const hostname = section?.split(splitter)?.[0]}
										<tr>
											<Cell
												first
												colspan={(!config || config?.dedicated_worker == undefined) &&
												($superadmin || $devopsRole)
													? 12
													: 9}
												scope="colgroup"
												class="!text-xs {groupIdx % 2 == 1 ? 'bg-surface-secondary/50' : ''}"
											>
												<div class="flex flex-row w-full text-2xs text-hint">
													<div class="min-w-64">
														Host:
														<span class="">{hostname}</span>
													</div>
													<span class="ml-4">IP: </span>
													<span class="">{workers[0].ip}</span>

													{#if workers?.length > 1}
														<span class="ml-8">{workers?.length} Workers</span>
													{/if}
												</div>
											</Cell>
										</tr>
										{#if workers}
											{#each workers as { worker, custom_tags, last_ping, started_at, jobs_executed, last_job_id, last_job_workspace_id, occupancy_rate_15s, occupancy_rate_5m, occupancy_rate_30m, occupancy_rate, wm_version, vcpus, memory, memory_usage, wm_memory_usage }}
												{@const isWorkerAlive = isWorkerMaybeAlive(last_ping)}
												<tr class={groupIdx % 2 == 1 ? 'bg-surface-secondary/50' : ''}>
													<Cell class="py-6 text-primary" first>
														{@const underscorePos = worker.search('_')}
														{#if underscorePos === -1}
															{worker}
														{:else}
															{truncate(worker, underscorePos)}
															<Tooltip>{worker}</Tooltip>
														{/if}
													</Cell>
													<Cell class="text-secondary">
														{#if custom_tags && custom_tags?.length > 2}
															{truncate(custom_tags?.join(', ') ?? '', 10)}
															<Tooltip>{custom_tags?.join(', ')}</Tooltip>
														{:else}
															{custom_tags?.join(', ') ?? ''}
														{/if}
													</Cell>
													<Cell class="text-secondary"
														>{last_ping != undefined ? last_ping + timeSinceLastPing : -1}s ago</Cell
													>
													<Cell class="text-secondary">{displayDate(started_at)}</Cell>
													<Cell class="text-secondary">{jobs_executed}</Cell>
													{#if (!config || config?.dedicated_worker == undefined) && ($superadmin || $devopsRole)}
														<Cell class="text-secondary">
															{#if last_job_id}
																<a href={`/run/${last_job_id}?workspace=${last_job_workspace_id}`}>
																	View last job
																</a>
																<br />
																(workspace {last_job_workspace_id})
															{/if}
														</Cell>
														<Cell class="text-secondary">
															{displayOccupancyRate(occupancy_rate_15s)}/{displayOccupancyRate(
																occupancy_rate_5m
															)}/{displayOccupancyRate(occupancy_rate_30m)}/{displayOccupancyRate(
																occupancy_rate
															)}
														</Cell>
													{/if}
													<Cell class="text-secondary">
														<div class="flex flex-col gap-1">
															<div>
																{memory_usage
																	? Math.round(memory_usage / 1024 / 1024) + 'MB'
																	: '--'}
															</div>
															<div>
																({wm_memory_usage
																	? Math.round(wm_memory_usage / 1024 / 1024) + 'MB'
																	: '--'})
															</div>
														</div>
													</Cell>
													<Cell class="text-secondary">
														<div class="flex flex-col gap-1">
															<div>
																{vcpus ? (vcpus / 100000).toFixed(2) + ' vCPUs' : '--'}
															</div>
															<div>
																{memory ? Math.round(memory / 1024 / 1024) + 'MB' : '--'}
															</div>
														</div>
													</Cell>
													<Cell class="text-secondary">
														<div class="!text-2xs">
															{wm_version.split('-')[0]}<Tooltip>{wm_version}</Tooltip>
														</div>
													</Cell>
													<Cell class="text-secondary">
														<Badge
															color={isWorkerAlive != undefined
																? isWorkerAlive
																	? 'green'
																	: 'red'
																: 'gray'}
														>
															{isWorkerAlive != undefined
																? isWorkerAlive
																	? 'Alive'
																	: 'Dead'
																: 'Unknown'}
														</Badge>
													</Cell>
													{#if $superadmin || $devopsRole}
														<Cell class="text-secondary">
															<Button
																unifiedSize="sm"
																color="light"
																on:click={() => {
																	if (isWorkerAlive === false) {
																		sendUserToast('Worker must be alive', true)
																		return
																	}
																	tag = retrieveCommonWorkerPrefix(worker)
																	replForWorkerDrawer?.openDrawer()
																}}
																startIcon={{ icon: Terminal }}
															>
																ssh
															</Button>
														</Cell>
													{/if}
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
								{width}
							/>
							<div class="text-xs text-primary"> No workers currently in this worker group </div>
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
		{/snippet}
	</CenteredPage>
{/if}
