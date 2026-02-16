<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Alert, Button, Skeleton, Tab, Tabs } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import OccupancyBars from '$lib/components/OccupancyBars.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import QueueMetricsDrawer from '$lib/components/QueueMetricsDrawer.svelte'
	import ManageTagsDrawer from '$lib/components/ManageTagsDrawer.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import WorkspaceGroup from '$lib/components/WorkerGroup.svelte'
	import { WorkerService, type WorkerPing, ConfigService, SettingService, SettingsService } from '$lib/gen'
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
	import {
		ExternalLink,
		LineChart,
		Loader2,
		Plus,
		Search,
		Tags,
		Terminal,
		TriangleAlert
	} from 'lucide-svelte'
	import { onDestroy, onMount, untrack } from 'svelte'

	import YAML from 'yaml'
	import { cleanWorkerGroupConfig } from '$lib/components/worker_group'
	import { DEFAULT_TAGS_WORKSPACES_SETTING } from '$lib/consts'
	import AutoscalingEvents from '$lib/components/AutoscalingEvents.svelte'
	import HttpAgentWorkerDrawer from '$lib/components/HttpAgentWorkerDrawer.svelte'
	import WorkerRepl from '$lib/components/WorkerRepl.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import TagList from '$lib/components/TagList.svelte'
	import MeltTooltip from '$lib/components/meltComponents/Tooltip.svelte'

	let workers: WorkerPing[] | undefined = $state(undefined)
	let workerGroups: Record<string, any> | undefined = $state(undefined)
	let groupedWorkers = $derived.by(() => groupWorkers(workers, workerGroups))
	let intervalId: number | undefined
	const splitter = '_%%%_'
	let customTags: string[] | undefined = $state(undefined)
	let serverVersion: string | undefined = $state(undefined)
	let minKeepAliveVersion: string | undefined = $state(undefined)
	let agentMinKeepAliveVersion: string | undefined = $state(undefined)

	type VersionWarning = 'none' | 'note' | 'warning' | 'critical' | 'newer'

	// TODO: add util in frontend or update backend endpoint with flag to return clean version
	function parseVersion(v: string): [number, number, number] | null {
		let clean = v.replace(/^(CE|EE)\s+/, '').replace(/^v/, '')
		const parts = clean.split(/[.-]/)
		const [major, minor, patch] = parts.map((p) => parseInt(p, 10))
		if (isNaN(major) || isNaN(minor)) return null
		return [major, minor || 0, patch || 0]
	}

	function getVersionWarning(workerVersion: string, isAgent: boolean = false): VersionWarning {
		if (!serverVersion) return 'none'
		const server = parseVersion(serverVersion)
		const worker = parseVersion(workerVersion)
		if (!server || !worker) return 'none'

		// Worker newer than server
		if (worker[0] > server[0] || (worker[0] === server[0] && worker[1] > server[1])) {
			return 'newer'
		}

		const minorLag = server[1] - worker[1]
		if (minorLag <= 0) return 'none'

		// Check against min keep alive (different for agents vs normal workers)
		const minVersion = isAgent ? agentMinKeepAliveVersion : minKeepAliveVersion
		if (minVersion) {
			const minKeepAlive = parseVersion(minVersion)
			if (minKeepAlive && (worker[0] < minKeepAlive[0] ||
				(worker[0] === minKeepAlive[0] && worker[1] < minKeepAlive[1]))) {
				return 'critical'
			}
		}

		// Agent workers: no warning for version lag, only critical if below min keep alive
		if (isAgent) return 'none'

		if (minorLag > 50) return 'warning'
		if (minorLag > 0) return 'note'
		return 'none'
	}

	// Compute the worst version warning across alive workers only
	let worstVersionWarning = $derived.by(() => {
		if (!workers) return 'none' as VersionWarning
		const priority: Record<VersionWarning, number> = { none: 0, note: 1, newer: 2, warning: 3, critical: 4 }
		let worst: VersionWarning = 'none'
		for (const w of workers) {
			// Only check alive workers (pinged within last 60 seconds, accounting for time since refresh)
			if (w.last_ping == null || w.last_ping + timeSinceLastPing >= 60) continue
			const isAgent = w.worker.startsWith('ag-')
			const warning = getVersionWarning(w.wm_version, isAgent)
			if (priority[warning] > priority[worst]) worst = warning
		}
		return worst
	})

	function groupWorkers(
		workers: WorkerPing[] | undefined,
		workerGroups: Record<string, any> | undefined
	): [string, [string, WorkerPing[]][], boolean][] {
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

		// Add isAgent detection to each group
		return grouped.map(([groupName, workerInstances]) => {
			const isAgent = workerInstances.some(([_, pings]) =>
				pings.some((ping) => ping.worker.startsWith('ag-'))
			)
			return [groupName, workerInstances, isAgent]
		})
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

	function parseLicenseKey(key: string): {
		valid: boolean
		expiration?: Date
	} {
		let splitted = key.split('.')
		if (splitted.length >= 3) {
			try {
				let i = parseInt(splitted[1])
				let date = new Date(i * 1000)
				const stringDate = date.toLocaleDateString()
				if (stringDate !== 'Invalid Date') {
					return {
						valid: date.getTime() > Date.now(),
						expiration: date
					}
				}
			} catch {}
		}
		return {
			valid: false
		}
	}

	async function checkLicenseExpiration() {
		if (!$superadmin || !$enterpriseLicense) {
			return
		}

		try {
			const licenseKey = (await SettingService.getGlobal({
				key: 'license_key'
			})) as string

			if (!licenseKey) {
				return
			}

			const { valid, expiration } = parseLicenseKey(licenseKey)

			if (!valid && expiration) {
				// License is expired
				sendUserToast(
					`Enterprise license key expired on ${expiration.toLocaleDateString()}. Please renew your license key to continue using Windmill.`,
					true
				)
			} else if (expiration) {
				// Check if expires within 7 days
				const daysUntilExpiration = Math.floor(
					(expiration.getTime() - Date.now()) / (1000 * 60 * 60 * 24)
				)

				if (daysUntilExpiration <= 7 && daysUntilExpiration >= 0) {
					sendUserToast(
						`Enterprise license key expires in ${daysUntilExpiration} day${daysUntilExpiration !== 1 ? 's' : ''} on ${expiration.toLocaleDateString()}. Please renew your license key to continue using Windmill.`,
						true
					)
				}
			}
		} catch (err) {
			// Silently fail - don't show errors for license check
			console.error('Failed to check license expiration:', err)
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
	SettingsService.backendVersion().then((v) => (serverVersion = v)).catch((e) => console.error('Failed to fetch server version:', e))
	SettingService.getMinKeepAliveVersion().then((v) => {
		minKeepAliveVersion = v.worker
		agentMinKeepAliveVersion = v.agent
	}).catch((e) => console.error('Failed to fetch min keep-alive version:', e))

	onDestroy(() => {
		if (intervalId) {
			clearInterval(intervalId)
		}
		if (secondInterval) {
			clearInterval(secondInterval)
		}
	})

	let newConfigName = $state('')
	let shouldAutoOpenDrawer = $state('')

	function handleWorkerGroupDeleted(deletedGroupName: string) {
		// If the deleted group was the currently selected one, check if group has workers
		if (selectedTab === deletedGroupName) {
			// Find the current group to check if it has active workers
			const currentGroup = groupedWorkers.find((group) => group[0] === deletedGroupName)
			const hasActiveWorkers =
				currentGroup && currentGroup[1].some(([_, workers]) => workers.length > 0)

			// Only select a new group if the current group has no active workers
			if (!hasActiveWorkers) {
				// Find available groups (after deletion)
				const availableGroups = groupedWorkers
					.map((group) => group[0])
					.filter((group) => group !== deletedGroupName)

				if (availableGroups.length > 0) {
					// Prioritize 'default' group if available, otherwise pick the first one
					selectedTab = availableGroups.includes('default') ? 'default' : availableGroups[0]
				} else {
					// No groups left, fall back to 'default'
					selectedTab = 'default'
				}
			}
			// If group has active workers, stay on the same group (workers remain visible without config)
		}
	}

	async function addConfig() {
		const configName = newConfigName
		try {
			await ConfigService.updateConfig({ name: 'worker__' + configName, requestBody: {} })
			newGroupPopover?.close()
			await loadWorkerGroups()
			// Select the new worker group and signal it should auto-open drawer
			selectedTab = configName
			shouldAutoOpenDrawer = configName
			sendUserToast(`Worker group ${configName} created`)
		} catch (err) {
			sendUserToast(`Could not create worker group: ${err}`, true)
		}
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

	let yamlConfigDrawer: Drawer | undefined = $state(undefined)
	let yamlConfigCode = $state('')
	let yamlConfigOriginal = $state('')
	let yamlDiffMode = $state(false)
	let yamlSaving = $state(false)

	function serializeWorkerGroupsAsYaml(groups: Record<string, any>): string {
		const priorityGroups = ['default', 'native']
		const sorted: Record<string, any> = {}
		const entries = Object.entries(groups).sort(([a], [b]) => {
			const ai = priorityGroups.indexOf(a)
			const bi = priorityGroups.indexOf(b)
			if (ai !== -1 && bi !== -1) return ai - bi
			if (ai !== -1) return -1
			if (bi !== -1) return 1
			return a.localeCompare(b)
		})
		for (const [name, config] of entries) {
			sorted[name] = cleanWorkerGroupConfig(config)
		}
		return YAML.stringify(sorted)
	}

	function openYamlDrawer() {
		if (!workerGroups) {
			return sendUserToast('No worker groups found', true)
		}
		yamlConfigCode = serializeWorkerGroupsAsYaml(workerGroups)
		yamlConfigOriginal = yamlConfigCode
		yamlDiffMode = false
		yamlConfigDrawer?.toggleDrawer?.()
	}

	async function saveYamlConfig() {
		yamlSaving = true
		try {
			const parsed = YAML.parse(yamlConfigCode)
			if (typeof parsed !== 'object' || parsed == null || Array.isArray(parsed)) {
				sendUserToast('YAML must be a map of worker group name to config', true)
				return
			}

			const newGroups = new Map<string, any>()
			for (const [name, config] of Object.entries(parsed)) {
				if (typeof name !== 'string' || name.length === 0) {
					sendUserToast('Worker group names must be non-empty strings', true)
					return
				}
				newGroups.set(name, config ?? {})
			}

			const oldNames = new Set(Object.keys(workerGroups ?? {}))
			const newNames = new Set(newGroups.keys())

			// Protect well-known groups from accidental deletion
			const protectedGroups = ['default', 'native']
			const deletedProtected = protectedGroups.filter(
				(g) => oldNames.has(g) && !newNames.has(g)
			)
			if (deletedProtected.length > 0) {
				sendUserToast(
					`Cannot remove well-known groups: ${deletedProtected.join(', ')}. Add them back to the YAML.`,
					true
				)
				return
			}

			// Delete removed groups
			for (const name of oldNames) {
				if (!newNames.has(name)) {
					await ConfigService.deleteConfig({ name: 'worker__' + name })
				}
			}

			// Create or update groups
			for (const [name, config] of newGroups) {
				await ConfigService.updateConfig({
					name: 'worker__' + name,
					requestBody: config
				})
			}

			yamlDiffMode = false
			yamlConfigDrawer?.toggleDrawer?.()
			sendUserToast('Worker group configs saved')
			await loadWorkerGroups()
		} catch (err) {
			if (err instanceof Error) {
				sendUserToast(err.message, true)
			} else {
				sendUserToast('Could not save worker group configs', true)
			}
		} finally {
			yamlSaving = false
		}
	}

	let queueMetricsDrawer: QueueMetricsDrawer | undefined = $state(undefined)
	let manageTagsDrawer: ManageTagsDrawer | undefined = $state(undefined)
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
		worker_group: [string, [string, WorkerPing[]][], boolean] | undefined,
		search: string
	): [string, [string, WorkerPing[]][], boolean] | undefined {
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

		return [worker_group[0], filteredWorkerGroup, worker_group[2]]
	}

	let newHttpAgentWorkerDrawer: Drawer | undefined = $state(undefined)
	let replForWorkerDrawer: Drawer | undefined = $state(undefined)

	function isWorkerMaybeAlive(last_ping: number | undefined): boolean | undefined {
		return last_ping != undefined ? last_ping < 60 : undefined
	}

	function getTagMismatchInfo(
		workerTags: string[] | undefined,
		configTags: string[] | undefined
	): { hasMismatch: boolean; added: string[]; removed: string[] } {
		if (!configTags || configTags.length === 0) {
			return { hasMismatch: false, added: [], removed: [] }
		}

		const worker = new Set(workerTags || [])
		const config = new Set(configTags || [])

		const added = [...worker].filter((tag) => !config.has(tag))
		const removed = [...config].filter((tag) => !worker.has(tag))

		const hasMismatch = added.length > 0 || removed.length > 0

		return { hasMismatch, added, removed }
	}

	let newGroupPopover: Popover | undefined = $state(undefined)

	$effect(() => {
		;($superadmin || $devopsRole) && untrack(() => loadDefaultTagsPerWorkspace())
	})

	$effect(() => {
		groupedWorkers &&
			selectedTab == 'default' &&
			untrack(() => updateSelectedTabIfDefaultDoesNotExist())
	})

	$effect(() => {
		$superadmin && $enterpriseLicense && untrack(() => checkLicenseExpiration())
	})

	let worker_group = $derived(
		filterWorkerGroupByNames(
			groupedWorkers?.find((x) => x?.[0] == selectedTab),
			search
		)
	)

	let columnCount = $derived.by(() => {
		const config = (workerGroups ?? {})[selectedTab]
		let cols = 7 // Worker, Worker start, Jobs ran, Memory usage, Limits, Version, Status
		if (!config || !config.worker_tags || config.worker_tags.length === 0) cols += 1 // Worker tags
		if ((!config || config?.dedicated_worker == undefined) && ($superadmin || $devopsRole))
			cols += 2 // Last job, Occupancy rate
		if ($superadmin || $devopsRole) cols += 1 // Repl
		return cols
	})
</script>

{#if $superadmin || $devopsRole}
	<QueueMetricsDrawer bind:this={queueMetricsDrawer} />
	<ManageTagsDrawer
		bind:this={manageTagsDrawer}
		bind:defaultTagPerWorkspace
		bind:defaultTagWorkspaces
		onRefresh={() => {
			loadCustomTags()
		}}
	/>
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

<Drawer bind:this={yamlConfigDrawer} size="800px">
	<DrawerContent
		title="Worker groups config (YAML)"
		on:close={() => {
			yamlDiffMode = false
			yamlConfigDrawer?.toggleDrawer?.()
		}}
	>
		<p class="text-2xs text-tertiary mb-2">
			Use this YAML to manage worker group configs as code.
			<a
				href="https://www.windmill.dev/docs/advanced/instance_settings#kubernetes-operator"
				target="_blank"
				rel="noopener noreferrer"
			>Learn more <ExternalLink size={12} class="inline-block" /></a>
		</p>
		{#if yamlDiffMode}
			<div class="w-full h-full">
				{#await import('$lib/components/DiffEditor.svelte')}
					<div class="flex items-center justify-center h-full">
						<Loader2 class="animate-spin" />
					</div>
				{:then Module}
					<Module.default
						open={true}
						className="!h-full"
						defaultLang="yaml"
						defaultOriginal={yamlConfigOriginal}
						defaultModified={yamlConfigCode}
						readOnly
						inlineDiff={true}
					/>
				{/await}
			</div>
		{:else}
			<SimpleEditor
				bind:code={yamlConfigCode}
				lang="yaml"
				class="h-full"
				fixedOverflowWidgets={false}
			/>
		{/if}
		{#snippet actions()}
			{#if yamlDiffMode}
				<Button
					unifiedSize="md"
					variant="default"
					disabled={yamlSaving}
					onClick={() => { yamlDiffMode = false }}
				>Back to editor</Button>
				<Button
					unifiedSize="md"
					variant="accent"
					loading={yamlSaving}
					onClick={saveYamlConfig}
				>Save</Button>
			{:else}
				<Button
					unifiedSize="md"
					variant="accent"
					disabled={!yamlConfigCode || yamlConfigCode === yamlConfigOriginal}
					onClick={() => { yamlDiffMode = true }}
				>Review & Save</Button>
			{/if}
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
					<div class="flex flex-row w-full pb-2 items-center gap-4">
						<Button
							unifiedSize="md"
							variant="default"
							startIcon={{
								icon: Tags
							}}
							on:click={() => {
								manageTagsDrawer?.openDrawer()
							}}
						>
							Manage tags
						</Button>

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
									newConfigName.trim() !== ''
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
										dropdownItems={[
											...$enterpriseLicense
												? [
														{
															label: 'Copy groups config as YAML',
															onClick: () => {
																if (!workerGroups) {
																	return sendUserToast('No worker groups found', true)
																}
																navigator.clipboard.writeText(serializeWorkerGroupsAsYaml(workerGroups))
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
												: [],
											{
												label: 'Edit all configs as YAML',
												onClick: openYamlDrawer
											}
										]}
									>
										<span class="hidden md:block"
											>New group config</span
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

									<Button
										unifiedSize="md"
										variant="accent"
										startIcon={{ icon: Plus }}
										disabled={!newConfigName}
										on:click={addConfig}
									>
										Create
									</Button>
								</div>
							{/snippet}
						</Popover>
					</div>
				{/if}
			</PageHeader>

			{#if worstVersionWarning === 'critical'}
				<Alert type="error" title="Critical: Workers below minimum version" class="my-4">
					One or more workers are running below the minimum supported version.
					This may cause undefined behavior and cluster instability.
					Upgrade these workers immediately—running workers this old is untested and strongly discouraged.
				</Alert>
			{:else if worstVersionWarning === 'warning'}
				<Alert type="warning" title="Workers significantly behind" class="my-4">
					One or more workers are significantly behind the server ({serverVersion}) by more than 50 minor versions.
					While they should still function, the risk of issues is elevated.
					Further server upgrades may push these workers into a critical state. Upgrading is recommended.
				</Alert>
			{:else if worstVersionWarning === 'newer'}
				<Alert type="warning" title="Workers ahead of server" class="my-4">
					One or more workers are running a newer version than the server ({serverVersion}).
					Workers should be at or behind the server version. This may cause undefined behavior.
				</Alert>
			{/if}

			{#if workers != undefined}
				{#if groupedWorkers.length == 0}
					<p>No workers seem to be available</p>
				{/if}

				{#if (groupedWorkers ?? []).length < 6}
					<Tabs bind:selected={selectedTab}>
						{#each groupedWorkers.map((x) => x[0]) as name (name)}
							{@const worker_group = groupedWorkers.find((x) => x[0] == name)}

							{#if worker_group}
								{@const activeWorkers = worker_group?.[1].flatMap((x) =>
									x[1]?.filter((y) => (y.last_ping ?? 0) < 15)
								)}
								<Tab value={worker_group[0]} label={worker_group[0]}>
									{#snippet extra()}
										<span class="text-2xs text-hint">
											{pluralize(activeWorkers?.length, 'worker')}
										</span>
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
							isAgent={worker_group[2]}
							{config}
							on:reload={() => {
								loadWorkerGroups()
							}}
							activeWorkers={activeWorkers?.length ?? 0}
							{defaultTagPerWorkspace}
							{width}
							shouldAutoOpenDrawer={shouldAutoOpenDrawer === worker_group[0]}
							onDrawerOpened={() => {
								shouldAutoOpenDrawer = ''
							}}
							onDeleted={handleWorkerGroupDeleted}
							onOpenYamlEditor={openYamlDrawer}
							selectGroup={(groupedWorkers ?? []).length > 5 ? selectGroup : undefined}
						/>

						<div class="mt-6"></div>

						<div class="flex flex-row gap-2 items-center justify-between">
							<div class="text-emphasis font-semibold text-xs">Active workers</div>
							<div class="flex flex-row items-center gap-2 relative mb-2">
								<TextInput
									inputProps={{
										placeholder: `Search workers in group '${worker_group[0]}'`,
										autocomplete: 'off'
									}}
									class="pl-8 min-w-80"
									bind:value={search}
								/>
								<Search class="absolute left-2" size={14} />
							</div>
						</div>

						<DataTable>
							<Head>
								<tr>
									<Cell head first>Worker</Cell>
									{#if !config || !config.worker_tags || config.worker_tags.length === 0}
										<Cell head>
											<div class="flex flex-row items-center gap-1 min-w-32">
												Worker tags
												<Tooltip
													documentationLink="https://www.windmill.dev/docs/core_concepts/worker_groups#assign-custom-worker-groups"
												>
													If defined, the workers only pull jobs with the same corresponding tag
												</Tooltip>
											</div>
										</Cell>
									{/if}
									<Cell head>Worker start</Cell>
									<Cell head>Jobs ran</Cell>
									{#if (!config || config?.dedicated_worker == undefined) && ($superadmin || $devopsRole)}
										<Cell head>Last job</Cell>
										<Cell head>Occupancy rate<br />(15s/5m/30m/ever)</Cell>
									{/if}
									<Cell head>Memory usage<br />(Windmill)</Cell>
									<Cell head>Limits</Cell>
									<Cell head>Version</Cell>
									<Cell head>Status</Cell>
									{#if $superadmin || $devopsRole}
										<Cell head>
											Repl
											<Tooltip>
												<p class="text-sm">
													Open a live shell to execute bash commands on the machine where the worker
													runs — useful for quick access, inspection, and real-time debugging
												</p>
											</Tooltip>
										</Cell>
									{/if}
								</tr>
							</Head>
							<tbody>
								{#if worker_group?.[1].length > 0}
									{#each worker_group[1] as [section, workers], groupIdx}
										{@const hostname = section?.split(splitter)?.[0]}
										<tr>
											<Cell
												first
												colspan={columnCount}
												scope="colgroup"
												class="!text-xs !pb-0 {groupIdx % 2 == 1 ? 'bg-surface-secondary/50' : ''}"
											>
												<div class="flex flex-row w-full text-2xs text-hint">
													<div class="">
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
												{@const tagMismatchInfo = getTagMismatchInfo(
													custom_tags,
													config?.worker_tags
												)}
												<tr class={groupIdx % 2 == 1 ? 'bg-surface-secondary/50' : ''}>
													<Cell class="text-primary" first>
														{@const underscorePos = worker.search('_')}
														<div class="flex items-center gap-2">
															<span>
																{#if underscorePos === -1}
																	{worker}
																{:else}
																	{truncate(worker, underscorePos)}
																	<Tooltip>{worker}</Tooltip>
																{/if}
															</span>
															{#if config && tagMismatchInfo.hasMismatch}
																<MeltTooltip>
																	<Badge color="yellow">
																		<TriangleAlert size={14} />
																	</Badge>

																	{#snippet text()}
																		<div class="flex flex-col gap-2 text-xs max-w-md">
																			<div class="font-semibold text-emphasis"
																				>Tag configuration mismatch</div
																			>
																			<p
																				>This worker has tags that differ from the config. This
																				could be due to a recent update of the config or because the
																				worker has tags defined as environment variables.</p
																			>

																			{#if tagMismatchInfo.added.length > 0}
																				<div>
																					<div class="text-emphasis font-semibold"
																						>Additional tags:</div
																					>
																					<TagList
																						tags={tagMismatchInfo.added}
																						class="flex flex-wrap gap-1"
																					/>
																				</div>
																			{/if}

																			{#if tagMismatchInfo.removed.length > 0}
																				<div>
																					<div class="text-emphasis font-semibold"
																						>Missing tags:</div
																					>
																					<TagList
																						tags={tagMismatchInfo.removed}
																						class="flex flex-wrap gap-1"
																					/>
																				</div>
																			{/if}
																		</div>
																	{/snippet}
																</MeltTooltip>
															{/if}
														</div>
													</Cell>
													{#if !config || !config.worker_tags || config.worker_tags.length === 0}
														<Cell class="min-w-0 max-w-32">
															<TagList tags={custom_tags ?? []} maxVisible={1} />
														</Cell>
													{/if}
													<Cell class="text-secondary">{displayDate(started_at)}</Cell>
													<Cell class="text-secondary">{jobs_executed}</Cell>
													{#if (!config || config?.dedicated_worker == undefined) && ($superadmin || $devopsRole)}
														<Cell class="text-secondary">
															{#if last_job_id}
																<a href={`/run/${last_job_id}?workspace=${last_job_workspace_id}`}>
																	View last job
																	<ExternalLink size={12} class="inline-block" />
																</a>
																<br />
																{last_job_workspace_id}
															{/if}
														</Cell>
														<Cell class="text-secondary">
															<OccupancyBars
																rate_15s={occupancy_rate_15s}
																rate_5m={occupancy_rate_5m}
																rate_30m={occupancy_rate_30m}
																rate_ever={occupancy_rate}
															/>
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
														{@const versionWarning = getVersionWarning(wm_version, worker.startsWith('ag-'))}
														<div class="flex items-center gap-1">
															<div class="!text-2xs" title={wm_version}>
																{wm_version.split('-')[0]}
															</div>
															{#if versionWarning !== 'none'}
																<MeltTooltip>
																	<Badge color={versionWarning === 'critical' ? 'red' : versionWarning === 'warning' ? 'yellow' : versionWarning === 'newer' ? 'yellow' : 'blue'}>
																		<TriangleAlert size={12} />
																	</Badge>
																	{#snippet text()}
																		{@const isAgent = worker.startsWith('ag-')}
																		<div class="max-w-xs text-xs">
																			{#if versionWarning === 'critical'}
																				<strong>Critical:</strong> This {isAgent ? 'agent worker' : 'worker'} is running below the minimum supported version ({isAgent ? agentMinKeepAliveVersion : minKeepAliveVersion}).
																				This may cause undefined behavior and cluster instability.
																				Upgrade this {isAgent ? 'agent worker' : 'worker'} immediately—running {isAgent ? 'agent workers' : 'workers'} this old is untested and strongly discouraged.
																			{:else if versionWarning === 'warning'}
																				<strong>Warning:</strong> This worker is significantly behind the server ({serverVersion}) by more than 50 minor versions.
																				While it should still function, the risk of issues is elevated.
																				Further server upgrades may push this worker into a critical state. Upgrading is recommended.
																			{:else if versionWarning === 'newer'}
																				<strong>Warning:</strong> This worker is running a newer version than the server ({serverVersion}).
																				Workers should be at or behind the server version. This may cause undefined behavior.
																			{:else}
																				<strong>Note:</strong> This worker is behind the server version.
																				While generally fine, keeping all workers aligned with the server provides the best stability and is most thoroughly tested.
																			{/if}
																		</div>
																	{/snippet}
																</MeltTooltip>
															{/if}
														</div>
													</Cell>
													<Cell class="text-secondary">
														{@const pingText =
															last_ping != undefined
																? `${last_ping + timeSinceLastPing}s ago`
																: 'Unknown'}
														{@const statusText =
															isWorkerAlive != undefined
																? isWorkerAlive
																	? 'Alive'
																	: 'Dead'
																: 'Unknown'}
														<div class="min-w-24 flex flex-row gap-1 items-center">
															<Badge
																color={isWorkerAlive != undefined
																	? isWorkerAlive
																		? 'green'
																		: 'red'
																	: 'gray'}
															>
																{statusText}
															</Badge>
															<div class="text-2xs text-hint">{pingText}</div>
														</div>
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
																title="Open repl"
															></Button>
														</Cell>
													{/if}
												</tr>
											{/each}
										{/if}
									{/each}
								{:else if search}
									<tr>
										<Cell colspan={columnCount}>
											<div class="text-xs text-primary py-2 text-center">
												No active workers found matching the search query
											</div>
										</Cell>
									</tr>
								{:else}
									<tr>
										<Cell colspan={columnCount}>
											<div class="text-xs text-primary py-2 text-center">
												No active workers found for the group '{worker_group[0]}'
											</div>
										</Cell>
									</tr>
								{/if}
							</tbody>
						</DataTable>
					{:else}
						{@const worker_group = Object.entries(workerGroups ?? {})
							.filter((x) => !groupedWorkers.some((y) => y[0] == x[0]))
							.find((x) => x[0] == selectedTab)}

						{#if worker_group}
							<WorkspaceGroup
								{customTags}
								workers={worker_group[1]}
								isAgent={false}
								on:reload={() => {
									loadWorkerGroups()
								}}
								name={worker_group[0]}
								config={worker_group[1]}
								activeWorkers={0}
								{width}
								shouldAutoOpenDrawer={shouldAutoOpenDrawer === worker_group[0]}
								onDrawerOpened={() => {
									shouldAutoOpenDrawer = ''
								}}
								onDeleted={handleWorkerGroupDeleted}
								onOpenYamlEditor={openYamlDrawer}
							/>
							<div class="text-xs text-primary"> No workers currently in this worker group </div>
						{/if}
					{/if}
				</div>
				<div class="pb-8"></div>
				<!-- Agent worker groups don't have autoscaling -->
				{#if worker_group?.[2] === false}
					<AutoscalingEvents worker_group={selectedTab} />
				{/if}
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

{#snippet selectGroup()}
	<div class="flex gap-2 items-center">
		<div class="text-secondary text-xs"
			>Worker group
			<Tooltip
				documentationLink="https://www.windmill.dev/docs/core_concepts/worker_groups"
				wrapperClass="inline-block"
				>Worker groups are groups of workers that share a config and are meant to be identical.
				Worker groups are meant to be used with tags. Tags can be assigned to scripts and flows and
				can be seen as dedicated queues. Only the corresponding
			</Tooltip>
		</div>
		<Select
			inputClass="text-sm font-semibold text-emphasis"
			items={groupedWorkers.map((x) => ({
				value: x[0],
				subtitle: `${pluralize(x[1]?.flatMap((x) => x[1]?.filter((y) => (y.last_ping ?? 0) < 15))?.length ?? 0, 'worker')}`
			}))}
			bind:value={selectedTab}
		/>
	</div>
{/snippet}
