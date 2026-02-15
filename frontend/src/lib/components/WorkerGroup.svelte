<script lang="ts">
	import {
		TriangleAlert,
		Plus,
		RefreshCcwIcon,
		RotateCcw,
		Settings,
		Trash,
		X,
		ExternalLink,
		FileCode
	} from 'lucide-svelte'
	import { Alert, Button, Drawer } from './common'
	import Badge from './common/badge/Badge.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import { ConfigService, WorkspaceService, type WorkerPing, type Workspace } from '$lib/gen'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import { createEventDispatcher } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import {
		emptyString,
		pluralize,
		cleanValueProperties,
		orderedJsonStringify,
		replaceFalseWithUndefined
	} from '$lib/utils'
	import { enterpriseLicense, superadmin, devopsRole } from '$lib/stores'
	import Tooltip from './meltComponents/Tooltip.svelte'
	import Editor from './Editor.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Section from './Section.svelte'
	import Label from './Label.svelte'
	import Toggle from './Toggle.svelte'
	import { cleanWorkerGroupConfig, defaultTags, nativeTags, type AutoscalingConfig } from './worker_group'
	import AutoscalingConfigEditor from './AutoscalingConfigEditor.svelte'
	import TagsToListenTo from './TagsToListenTo.svelte'
	import Select from './select/Select.svelte'
	import MultiSelect from './select/MultiSelect.svelte'
	import { safeSelectItems } from './select/utils.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import Dropdown from './DropdownV2.svelte'
	import TagList from './TagList.svelte'
	import DedicatedWorkersSelector from './DedicatedWorkersSelector.svelte'
	import { computeHashedTag } from './dedicated_worker'

	function computeVCpuAndMemory(workers: [string, WorkerPing[]][]) {
		let vcpus = 0
		let memory = 0
		for (const [_, pings] of workers) {
			for (const ping of pings) {
				if (ping.vcpus) {
					vcpus += ping.vcpus
				}
				if (ping.memory) {
					memory += ping.memory
				}
			}
		}
		return { vcpus, memory }
	}

	function hasWorkersWithoutIsolation(workers: [string, WorkerPing[]][]): boolean {
		return workers.some(([_, pings]) =>
			pings.some((w) => !w.job_isolation || w.job_isolation === 'none')
		)
	}

	function getWorkersWithoutIsolation(workers: [string, WorkerPing[]][]): WorkerPing[] {
		return workers.flatMap(([_, pings]) =>
			pings.filter((w) => !w.job_isolation || w.job_isolation === 'none')
		)
	}

	let nconfig: {
		dedicated_worker?: string
		dedicated_workers?: string[]
		worker_tags?: string[]
		priority_tags?: Record<string, number>
		cache_clear?: number
		init_bash?: string
		periodic_script_bash?: string
		periodic_script_interval_seconds?: number
		env_vars_static?: Map<string, string>
		env_vars_allowlist?: string[]
		additional_python_paths?: string[]
		pip_local_dependencies?: string[]
		min_alive_workers_alert_threshold?: number
		autoscaling?: AutoscalingConfig
	} = $state({})

	function loadNConfig() {
		nconfig = config
			? config.worker_tags != undefined ||
				config.dedicated_worker != undefined ||
				config.dedicated_workers != undefined
				? config
				: {
						worker_tags: []
					}
			: {
					worker_tags: []
				}
		if (nconfig.priority_tags === undefined) {
			nconfig.priority_tags = {}
		}

		// Convert legacy dedicated_worker to dedicated_workers array
		if (nconfig.dedicated_worker && !nconfig.dedicated_workers?.length) {
			nconfig.dedicated_workers = [nconfig.dedicated_worker]
			nconfig.dedicated_worker = undefined
		}

		customEnvVars = []
		if (nconfig.env_vars_allowlist === undefined) {
			nconfig.env_vars_allowlist = []
		} else {
			for (const key of nconfig.env_vars_allowlist) {
				customEnvVars.push({ key, type: 'dynamic', value: undefined })
			}
		}
		if (nconfig.env_vars_static === undefined) {
			nconfig.env_vars_static = new Map<string, string>()
		} else {
			for (const [key, value] of Object.entries(nconfig.env_vars_static)) {
				customEnvVars.push({ key, type: 'static', value })
			}
		}
		customEnvVars.sort((a, b) => (a.key < b.key ? -1 : 1))
	}

	// Clean up priority_tags when worker_tags changes
	$effect(() => {
		// This effect ensures priority_tags only contains tags that exist in worker_tags
		if (nconfig.worker_tags && nconfig.priority_tags) {
			const validTags = new Set(nconfig.worker_tags)
			const currentPriorityKeys = Object.keys(nconfig.priority_tags)

			// Check if any priority tags are no longer valid
			const hasInvalidTags = currentPriorityKeys.some((tag) => !validTags.has(tag))

			if (hasInvalidTags) {
				// Filter out invalid tags
				const filteredPriorityTags: Record<string, number> = {}
				for (const [tag, priority] of Object.entries(nconfig.priority_tags)) {
					if (validTags.has(tag)) {
						filteredPriorityTags[tag] = priority
					}
				}
				nconfig.priority_tags = filteredPriorityTags
			}
		}
	})

	let customEnvVars: {
		key: string
		type: 'static' | 'dynamic'
		value: string | undefined
	}[] = $state([])

	const aws_env_vars_preset = [
		'AWS_REGION',
		'AWS_DEFAULT_REGION',
		'AWS_ACCESS_KEY_ID',
		'AWS_SECRET_ACCESS_KEY',
		'AWS_ENDPOINT_URL'
	]
	const ssl_env_vars_preset = [
		'DENO_CERT',
		'PIP_INDEX_CERT',
		'REQUESTS_CA_BUNDLE',
		'SSL_CERT_FILE',
		'SSL_CERT_DIR'
	]

	let workspaceTag = $state('')
	interface Props {
		name: string
		config:
			| undefined
			| {
					dedicated_worker?: string
					dedicated_workers?: string[]
					worker_tags?: string[]
					priority_tags?: Record<string, number>
					cache_clear?: number
					init_bash?: string
					additional_python_paths?: string[]
					pip_local_dependencies?: string[]
					min_alive_workers_alert_threshold?: number
					autoscaling?: AutoscalingConfig
					periodic_script_bash?: string
					periodic_script_interval_seconds?: number
			  }
		activeWorkers: number
		customTags: string[] | undefined
		workers: [string, WorkerPing[]][]
		isAgent?: boolean
		defaultTagPerWorkspace?: boolean | undefined
		width: number
		shouldAutoOpenDrawer?: boolean
		onDrawerOpened?: () => void
		onDeleted?: (deletedGroupName: string) => void
		onOpenYamlEditor?: () => void
		selectGroup?: import('svelte').Snippet
	}

	let {
		name,
		config = $bindable(),
		activeWorkers,
		customTags,
		workers,
		isAgent = false,
		defaultTagPerWorkspace = undefined,
		width = 0,
		shouldAutoOpenDrawer = false,
		onDrawerOpened = () => {},
		onDeleted = () => {},
		onOpenYamlEditor = undefined,
		selectGroup = undefined
	}: Props = $props()

	let workspaces: Workspace[] = $state([])
	async function listWorkspaces() {
		workspaces = await WorkspaceService.listWorkspacesAsSuperAdmin()
	}

	// Centralized permission logic
	let canEditConfig = $derived($superadmin || $devopsRole)
	let hasEnterpriseFeatures = $derived($enterpriseLicense)
	let canEditEEConfig = $derived(canEditConfig && hasEnterpriseFeatures)

	const dispatch = createEventDispatcher()

	async function deleteWorkerGroup() {
		await ConfigService.deleteConfig({ name: 'worker__' + name })
		dispatch('reload')
		onDeleted(name)
	}

	let hasChanges = $derived.by(() => {
		if (!config && !nconfig) return false
		if (!config || !nconfig) return true

		const cleaned1 = cleanValueProperties(config)
		const cleaned2 = cleanValueProperties(nconfig)

		return (
			orderedJsonStringify(replaceFalseWithUndefined(cleaned1)) !==
			orderedJsonStringify(replaceFalseWithUndefined(cleaned2))
		)
	})
	let openDelete = $state(false)
	let openClean = $state(false)

	// Compute hashed tags for display (actual tags used by the worker)
	let hashedDedicatedTags: Map<string, string> = $state(new Map())
	$effect(() => {
		const dws = config?.dedicated_workers ?? (config?.dedicated_worker ? [config.dedicated_worker] : [])
		if (dws.length > 0) {
			Promise.all(dws.map(async (dw) => [dw, await computeHashedTag(dw)] as const)).then(
				(entries) => {
					hashedDedicatedTags = new Map(entries)
				}
			)
		} else {
			hashedDedicatedTags = new Map()
		}
	})

	let drawer: Drawer | undefined = $state()
	let vcpus_memory = $derived(computeVCpuAndMemory(workers))
	let selected = $derived(
		nconfig?.dedicated_worker != undefined || nconfig?.dedicated_workers != undefined
			? 'dedicated'
			: 'normal'
	)
	$effect(() => {
		;($superadmin || $devopsRole) && listWorkspaces()
	})

	$effect(() => {
		if (shouldAutoOpenDrawer) {
			loadNConfig()
			drawer?.openDrawer()
			// Dispatch event to parent to clear the signal
			onDrawerOpened()
		}
	})
</script>

<ConfirmationModal
	open={openDelete}
	title="Delete worker group"
	confirmationText="Remove"
	on:canceled={() => {
		openDelete = false
	}}
	on:confirmed={async () => {
		deleteWorkerGroup()
		openDelete = false
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>Are you sure you want to remove this worker group {name}?</span>
	</div>
</ConfirmationModal>

<ConfirmationModal
	open={openClean}
	title="Clear cache"
	confirmationText="Remove"
	on:canceled={() => {
		openClean = false
	}}
	on:confirmed={async () => {
		const ndate = Math.floor(Date.now() / 1000)
		const withCacheConfig = { ...nconfig, cache_clear: ndate }
		await ConfigService.updateConfig({
			name: 'worker__' + name,
			requestBody: withCacheConfig
		})
		if (config) {
			config.cache_clear = ndate
		}
		sendUserToast('Worker caches clearing in 5s. Require a restart.')
		dispatch('reload')
		openClean = false
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span
			>Are you sure you want to clean the cache of all workers of this worker group (will also
			restart the workers and expect supervisor to restart them) ?</span
		>
	</div>
</ConfirmationModal>

<Drawer bind:this={drawer} size="800px">
	<DrawerContent
		on:close={() => drawer?.closeDrawer()}
		title={canEditConfig ? `Edit worker config '${name}'` : `Worker config '${name}'`}
	>
		{#if !hasEnterpriseFeatures}
			<Alert type="info" title="Enterprise features" class="mb-4">
				Some worker management features require the enterprise edition. In CE, you can configure
				worker tags.
			</Alert>
		{:else if !canEditConfig}
			<Alert type="info" title="Read-only mode" class="mb-4">
				Only superadmin or devops role can edit the worker config.
			</Alert>
		{/if}
		<Label label="Workers assignment" eeOnly={!hasEnterpriseFeatures}>
			<ToggleButtonGroup
				{selected}
				disabled={!canEditEEConfig}
				on:selected={(e) => {
					if (nconfig == undefined) {
						nconfig = {}
					}
					if (e.detail == 'dedicated') {
						nconfig.dedicated_workers = nconfig.dedicated_workers ?? []
						nconfig.dedicated_worker = undefined
						nconfig.worker_tags = undefined
					} else {
						nconfig.dedicated_worker = undefined
						nconfig.dedicated_workers = undefined
						nconfig.worker_tags = []
					}
				}}
			>
				{#snippet children({ item })}
					<ToggleButton value="normal" label="Any jobs within worker tags" {item} />
					<ToggleButton value="dedicated" label="Dedicated to scripts/flows" {item} />
				{/snippet}
			</ToggleButtonGroup>
		</Label>

		<div class="mt-8"></div>
		{#if selected == 'normal'}
			<Label label="Tags to listen to">
				{#snippet action()}
					{#if nconfig?.worker_tags != undefined}
						{@const dropdownResetToAllTags = [
							{
								label: 'Reset to all tags minus native ones',
								onClick: () => {
									if (nconfig != undefined) {
										nconfig.worker_tags = defaultTags
									}
								},
								disabled: !canEditConfig,
								tooltip: (defaultTagPerWorkspace
									? defaultTags.map((nt) => `${nt}-${workspaceTag}`)
									: defaultTags
								).join(', ')
							},
							{
								label: 'Reset to native tags',
								onClick: () => {
									if (nconfig != undefined) {
										nconfig.worker_tags = nativeTags
									}
								},
								disabled: !canEditConfig,
								tooltip: (defaultTagPerWorkspace && workspaceTag
									? nativeTags.map((nt) => `${nt}-${workspaceTag}`)
									: nativeTags
								).join(', ')
							},
							{
								label: 'Clear tags',
								onClick: () => {
									if (nconfig != undefined) {
										nconfig.worker_tags = []
									}
								},
								disabled: !canEditConfig
							}
						]}
						<div class="flex flex-wrap items-center gap-1">
							<Button
								variant="default"
								unifiedSize="sm"
								on:click={() => {
									if (nconfig != undefined) {
										nconfig.worker_tags =
											defaultTagPerWorkspace && workspaceTag
												? defaultTags.concat(nativeTags).map((nt) => `${nt}-${workspaceTag}`)
												: defaultTags.concat(nativeTags)
									}
								}}
								dropdownItems={dropdownResetToAllTags}
								dropdownWidth={300}
								startIcon={{ icon: RotateCcw }}
								disabled={!canEditConfig}
							>
								Reset to all tags <Tooltip>
									{#snippet text()}
										{(defaultTagPerWorkspace && workspaceTag
											? defaultTags.concat(nativeTags).map((nt) => `${nt}-${workspaceTag}`)
											: defaultTags.concat(nativeTags)
										).join(', ')}
									{/snippet}
								</Tooltip>
							</Button>

							{#if defaultTagPerWorkspace}
								<Select
									bind:value={workspaceTag}
									items={workspaces.map((w) => ({ value: w.id }))}
									onCreateItem={(c) => (workspaceTag = c)}
									placeholder="Workspace ID"
									disabled={!canEditConfig}
								/>
							{/if}
						</div>
					{/if}
				{/snippet}
				{#if nconfig?.worker_tags != undefined}
					<TagsToListenTo
						bind:worker_tags={nconfig.worker_tags}
						{customTags}
						disabled={!canEditConfig}
					/>
				{/if}
			</Label>

			{#if nconfig?.worker_tags !== undefined && nconfig?.worker_tags.length > 0}
				<div class="mt-8"></div>
				<Label label="High-priority tags" eeOnly={!hasEnterpriseFeatures}>
					{#snippet header()}
						<Tooltip>
							{#snippet text()}
								Jobs with the following high-priority tags will be picked up in priority by this
								worker.
								{#if !enterpriseLicense}
									This is a feature only available in enterprise edition.
								{/if}
							{/snippet}
						</Tooltip>
					{/snippet}
					<MultiSelect
						disabled={!canEditEEConfig}
						bind:value={
							() => (nconfig.priority_tags ? Object.keys(nconfig.priority_tags) : []),
							(v) => {
								nconfig.priority_tags = Object.fromEntries(v.map((k) => [k, 100]))
							}
						}
						items={safeSelectItems(nconfig?.worker_tags)}
					/>
				</Label>
			{/if}

			{#if nconfig !== undefined}
				<div class="mt-8"></div>
				<Label label="Alerts" tooltip="Alert is sent to the configured critical error channels" eeOnly={!hasEnterpriseFeatures}>
					<Toggle
						size="sm"
						options={{
							right: 'Send an alert when the number of alive workers falls below a given threshold'
						}}
						checked={nconfig?.min_alive_workers_alert_threshold !== undefined}
						on:change={(ev) => {
							if (nconfig !== undefined) {
								nconfig.min_alive_workers_alert_threshold = ev.detail ? 1 : undefined
							}
						}}
						disabled={!canEditEEConfig}
					/>
					{#if nconfig.min_alive_workers_alert_threshold !== undefined}
						<div class="flex flex-row items-center text-xs gap-2">
							<p>Triggered when number of workers in group is lower than</p>
							<input
								type="number"
								class="!w-14 text-center"
								disabled={!canEditEEConfig}
								min="1"
								bind:value={nconfig.min_alive_workers_alert_threshold}
							/>
						</div>
					{/if}
				</Label>
			{/if}
		{:else if selected == 'dedicated'}
			<div class="flex flex-col gap-4">
				{#if $superadmin || $devopsRole}
					<div class="py-2">
						<Alert
							size="xs"
							type="info"
							title="The 'dedicated worker' runtime setting of the runnables must be enabled to be selected here"
						/>
					</div>
				{/if}

				<p class="text-xs text-secondary"
					>Workers will get killed upon detecting changes. It is assumed they are in an environment
					where the supervisor will restart them.</p
				>

				{#if nconfig !== undefined}
					<DedicatedWorkersSelector
						selectedTags={nconfig.dedicated_workers ?? []}
						disabled={!canEditEEConfig}
						onchange={(tags) => {
							if (nconfig) {
								nconfig.dedicated_workers = tags
								nconfig.dedicated_worker = undefined
							}
						}}
					/>
				{/if}
			</div>
		{/if}

		<div class="mt-8"></div>

		<Label
			label="Environment variables passed to jobs"
			tooltip="Add static and dynamic environment variables that will be passed to jobs handled by this worker group. Dynamic environment variable values will be loaded from the worker host environment variables while static environment variables will be set directly from their values below."
			eeOnly={!hasEnterpriseFeatures}
		>
			<div class="flex flex-col gap-y-2 pb-2 max-w">
				{#each customEnvVars as envvar, i}
					<div class="flex gap-1 items-center">
						<input
							disabled={!canEditEEConfig}
							type="text"
							placeholder="ENV_VAR_NAME"
							bind:value={envvar.key}
							onkeypress={(e) => {}}
						/>
						<ToggleButtonGroup
							disabled={!canEditEEConfig}
							class="w-128"
							bind:selected={envvar.type}
							on:selected={(e) => {
								if (e.detail === 'dynamic') {
									envvar.value = undefined
								}
							}}
						>
							{#snippet children({ item })}
								<ToggleButton value="dynamic" label="Dynamic" {item} disabled={!canEditEEConfig} />
								<ToggleButton value="static" label="Static" {item} disabled={!canEditEEConfig} />
							{/snippet}
						</ToggleButtonGroup>
						<TextInput
							inputProps={{
								type: 'text',
								disabled: !canEditEEConfig || envvar.type === 'dynamic',
								placeholder:
									envvar.type === 'dynamic' ? 'value read from worker env var' : 'static value'
							}}
							bind:value={envvar.value}
						/>
						{#if canEditEEConfig}
							<Button
								wrapperClasses="ml-2"
								variant="subtle"
								unifiedSize="md"
								aria-label="Clear"
								onClick={() => {
									if (nconfig.env_vars_static?.[envvar.key] !== undefined) {
										delete nconfig.env_vars_static[envvar.key]
									}
									if (nconfig.env_vars_allowlist?.includes(envvar.key)) {
										nconfig.env_vars_allowlist = nconfig.env_vars_allowlist.filter(
											(k) => k != envvar.key
										)
									}
									customEnvVars.splice(i, 1)
									customEnvVars = [...customEnvVars]
								}}
								startIcon={{ icon: Trash }}
								iconOnly
								destructive
								disabled={!canEditEEConfig}
							/>
						{/if}
					</div>
				{/each}
				{#if canEditEEConfig}
					<div class="flex flex-col gap-2">
						<Button
							variant="default"
							unifiedSize="md"
							startIcon={{ icon: Plus }}
							on:click={() => {
								customEnvVars.push({ key: '', type: 'dynamic', value: undefined })
								customEnvVars = [...customEnvVars]
							}}
							disabled={!canEditEEConfig}
						>
							Add environment variable
						</Button>

						<span class="text-secondary text-xs">
							Set up env variables for AWS or SSL using our
							<Dropdown
								placement="bottom-start"
								class="inline-block"
								items={[
									{
										displayName: `AWS (${aws_env_vars_preset.join(', ')})`,
										action: () => {
											let updated = false
											aws_env_vars_preset.forEach((envvar) => {
												if (!customEnvVars.some((e) => e.key === envvar)) {
													updated = true
													customEnvVars.push({
														key: envvar,
														type: 'dynamic',
														value: undefined
													})
												}
											})
											if (updated) {
												customEnvVars = [...customEnvVars]
											}
										}
									},
									{
										displayName: `SSL (${ssl_env_vars_preset.join(', ')})`,
										action: () => {
											let updated = false
											ssl_env_vars_preset.forEach((envvar) => {
												if (!customEnvVars.some((e) => e.key === envvar)) {
													updated = true
													customEnvVars.push({
														key: envvar,
														type: 'dynamic',
														value: undefined
													})
												}
											})
											if (updated) {
												customEnvVars = [...customEnvVars]
											}
										}
									}
								]}
							>
								{#snippet buttonReplacement()}
									<button class="text-accent font-medium"> presets </button>
								{/snippet}
							</Dropdown>
						</span>
					</div>
				{/if}
			</div>
		</Label>
		<div class="mt-8"></div>

		<AutoscalingConfigEditor
			worker_tags={config?.worker_tags}
			bind:config={nconfig.autoscaling}
			disabled={!canEditEEConfig}
			eeOnly={!hasEnterpriseFeatures}
		/>

		<div class="mt-8"></div>
		<Section label="Python dependencies overrides" collapsable={true} class="flex flex-col gap-y-6" eeOnly={!hasEnterpriseFeatures}>
			<Label
				label="Additional Python paths"
				tooltip="Paths to add to the Python path for it to search dependencies, useful if you have packages pre-installed on the workers at a given path."
				class="mt-2"
			>
				{#if nconfig.additional_python_paths}
					{#each nconfig.additional_python_paths as _, i}
						<div class="flex gap-1 items-center">
							<input
								type="text"
								disabled={!canEditEEConfig}
								placeholder="/path/to/python3.X/site-packages"
								bind:value={nconfig.additional_python_paths![i]}
							/>
							{#if canEditEEConfig}
								<button
									class="rounded-full bg-surface/60 hover:bg-surface-hover"
									aria-label="Clear"
									onclick={() => {
										if (
											nconfig.additional_python_paths === undefined ||
											nconfig.additional_python_paths.length == 0
										) {
											return
										}
										nconfig.additional_python_paths.splice(i, 1)
										nconfig.additional_python_paths = [...nconfig.additional_python_paths]
									}}
								>
									<X size={14} />
								</button>
							{/if}
						</div>
					{/each}
				{/if}
				{#if canEditEEConfig}
					<div class="flex">
						<Button
							variant="default"
							size="xs"
							startIcon={{ icon: Plus }}
							on:click={() => {
								if (nconfig.additional_python_paths === undefined) {
									nconfig.additional_python_paths = []
								}
								nconfig.additional_python_paths.push('')
								nconfig.additional_python_paths = [...nconfig.additional_python_paths]
							}}
						>
							Add additional Python path
						</Button>
					</div>
				{/if}
			</Label>
			<Label
				label="Local dependencies import names to skip during resolution"
				tooltip="uv will not try to resolve dependencies for these packages, useful if you have packages pre-installed on the workers at a given path."
			>
				{#if nconfig.pip_local_dependencies}
					{#each nconfig.pip_local_dependencies as _, i}
						<div class="flex gap-1 items-center">
							<input
								disabled={!canEditEEConfig}
								type="text"
								placeholder="httpx"
								bind:value={nconfig.pip_local_dependencies[i]}
							/>
							{#if canEditEEConfig}
								<button
									class="rounded-full bg-surface/60 hover:bg-surface-hover"
									aria-label="Clear"
									onclick={() => {
										if (
											nconfig.pip_local_dependencies === undefined ||
											nconfig.pip_local_dependencies.length == 0
										) {
											return
										}
										nconfig.pip_local_dependencies.splice(i, 1)
										nconfig.pip_local_dependencies = [...nconfig.pip_local_dependencies]
									}}
								>
									<X size={14} />
								</button>
							{/if}
						</div>
					{/each}
				{/if}
				{#if canEditEEConfig}
					<div class="flex">
						<Button
							variant="default"
							size="xs"
							startIcon={{ icon: Plus }}
							on:click={() => {
								if (nconfig.pip_local_dependencies === undefined) {
									nconfig.pip_local_dependencies = []
								}
								nconfig.pip_local_dependencies.push('')
								nconfig.pip_local_dependencies = [...nconfig.pip_local_dependencies]
							}}
						>
							Add PIP local dependency
						</Button>
					</div>
				{/if}
			</Label>
		</Section>

		<div class="mt-8"></div>

		<Section
			label="Init script"
			tooltip="Bash script run at start of the workers. More lightweight than requiring custom worker images."
			collapsable
			initiallyCollapsed={nconfig.init_bash === undefined}
		>
			{#snippet header()}
				<div class="ml-4 flex flex-row gap-2 items-center">
					{#if nconfig.init_bash !== undefined}
						<Badge color="green">Enabled</Badge>
					{/if}
				</div>
			{/snippet}
			{#if (nconfig.init_bash ?? '') !== (config?.init_bash ?? '')}
				<div class="mb-2">
					<Alert size="xs" type="info" title="Worker restart required">
						Workers will get killed upon detecting changes to the init script. It is assumed they
						are in an environment where the supervisor will restart them.
					</Alert>
				</div>
			{/if}
			<p class="text-xs text-secondary mb-2">
				Execution logs are visible on the runs page of the admins workspace. Use the script editor
				with Bash to iterate on your script before setting it here.
			</p>
			<div class="border w-full h-40">
				<Editor
					fixedOverflowWidgets={true}
					disabled={!canEditConfig}
					class="flex flex-1 grow h-full w-full"
					automaticLayout
					scriptLang={'bash'}
					useWebsockets={false}
					code={config?.init_bash ?? ''}
					on:change={(e) => {
						if (config) {
							const code = e.detail
							if (code != '') {
								nconfig.init_bash = code?.replace(/\r\n/g, '\n')
							} else {
								nconfig.init_bash = undefined
							}
						}
					}}
				/>
			</div>
		</Section>

		<div class="mt-8"></div>

		<Section
			label="Periodic script"
			tooltip="Bash script run periodically at configurable intervals. Useful for maintenance tasks like cleaning disk space."
			collapsable
			eeOnly={!hasEnterpriseFeatures}
			initiallyCollapsed={nconfig.periodic_script_bash === undefined}
		>
			{#snippet header()}
				<div class="ml-4 flex flex-row gap-2 items-center">
					{#if nconfig.periodic_script_bash !== undefined}
						<Badge color="green">Enabled</Badge>
					{/if}
				</div>
			{/snippet}
			{#if (nconfig.periodic_script_bash ?? '') !== (config?.periodic_script_bash ?? '') || (nconfig.periodic_script_interval_seconds ?? 0) !== (config?.periodic_script_interval_seconds ?? 0)}
				<div class="mb-2">
					<Alert size="xs" type="info" title="Worker restart required">
						Workers will get killed upon detecting changes to the periodic script or interval.
						It is assumed they are in an environment where the supervisor will restart them.
					</Alert>
				</div>
			{/if}

			<div class="flex gap-4 items-center mb-4">
				<Label label="Execution interval (seconds)" for="periodic-script-interval-seconds">
					<TextInput
						inputProps={{
							disabled: !canEditEEConfig,
							type: 'number',
							min: '60',
							placeholder: '3600',
							class: '!w-24 '
						}}
						bind:value={nconfig.periodic_script_interval_seconds}
					/>
					<span class="text-2xs text-hint">Minimum: 60 seconds</span>
				</Label>
			</div>

			<div class="border w-full h-40">
				<Editor
					disabled={!canEditEEConfig}
					class="flex flex-1 grow h-full w-full"
					automaticLayout
					scriptLang={'bash'}
					useWebsockets={false}
					fixedOverflowWidgets={false}
					code={config?.periodic_script_bash ?? ''}
					on:change={(e) => {
						if (config) {
							const code = e.detail
							if (code != '') {
								nconfig.periodic_script_bash = code?.replace(/\r\n/g, '\n')
							} else {
								nconfig.periodic_script_bash = undefined
							}
						}
					}}
				/>
			</div>
		</Section>

		<div class="mt-8">
			<Section label="Set via API" collapsable headerClass="text-secondary">
				<p class="text-xs text-tertiary">Requires a superadmin token.</p>
				<pre class="mt-1 p-2 bg-surface-secondary rounded text-xs overflow-x-auto whitespace-pre-wrap break-all">curl -X POST '{window.location.origin}/api/configs/update/worker__{name}' \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer &lt;token&gt;' \
  -d '{JSON.stringify(cleanWorkerGroupConfig(nconfig ?? {}), null, 2)}'</pre>
			</Section>
		</div>
		{#snippet actions()}
			<div class="flex gap-4 items-center">
				{#if onOpenYamlEditor}
					<Button
						variant="default"
						unifiedSize="md"
						startIcon={{ icon: FileCode }}
						onClick={() => {
							drawer?.closeDrawer()
							onOpenYamlEditor?.()
						}}
					>
						YAML editor
					</Button>
				{/if}
				<div class="flex gap-2 items-center">
					{#if canEditConfig}
						<Button
							variant="accent"
							on:click={async () => {
								if (
									nconfig?.min_alive_workers_alert_threshold &&
									nconfig?.min_alive_workers_alert_threshold < 1
								) {
									sendUserToast('Minimum alive workers alert threshold must be at least 1', true)
									return
								}
								if (
									nconfig?.periodic_script_bash &&
									(!nconfig?.periodic_script_interval_seconds ||
										nconfig?.periodic_script_interval_seconds < 60)
								) {
									sendUserToast('Periodic script interval must be at least 60 seconds', true)
									return
								}
								// Remove duplicate env vars by keeping only the last occurrence of each key
								const seenKeys = new Set()
								customEnvVars = customEnvVars
									.reverse()
									.filter((envVar) => {
										if (envVar.key == '') {
											return false
										}
										if (seenKeys.has(envVar.key)) {
											return false
										}
										seenKeys.add(envVar.key)
										return true
									})
									.reverse()
								nconfig.env_vars_static = new Map()
								nconfig.env_vars_allowlist = []
								customEnvVars.forEach((envvar) => {
									if (
										nconfig.env_vars_static !== undefined &&
										nconfig.env_vars_allowlist !== undefined &&
										!emptyString(envvar.key)
									) {
										if (envvar.type === 'dynamic') {
											nconfig.env_vars_allowlist.push(envvar.key)
										} else {
											nconfig.env_vars_static[envvar.key] = envvar.value
										}
									}
								})

								await ConfigService.updateConfig({ name: 'worker__' + name, requestBody: nconfig })
								sendUserToast('Configuration set')
								dispatch('reload')
							}}
							disabled={(!hasChanges && nconfig?.dedicated_worker == undefined) || !canEditConfig}
						>
							Apply changes
						</Button>
					{:else}
						<span class="text-secondary text-xs">Read only</span>
					{/if}
				</div>
			</div>
		{/snippet}
	</DrawerContent>
</Drawer>

<div
	class="flex flex-col gap-6 items-center justify-between mt-2 rounded-md border border-light p-4 bg-surface-tertiary"
>
	<div class="flex gap-2 justify-between w-full">
		<div class="text-xs flex flex-row gap-2 items-center">
			<div class="flex flex-row gap-1 items-center">
				{#if selectGroup}
					{@render selectGroup()}
					<span class="ml-r"></span>
				{:else}
					<span class="text-secondary text-xs">Worker group:</span>
					<span class="text-emphasis font-semibold text-sm">{name} - </span>
				{/if}
				<span class="inline-flex">
					{`${pluralize(activeWorkers, 'worker')} `}
					<Tooltip
						>{#snippet text()}Number of active workers of this group in the last 15 seconds{/snippet}</Tooltip
					></span
				>
			</div>

			{#if vcpus_memory?.vcpus}
				- {(vcpus_memory?.vcpus / 100000).toFixed(2)} vCPUs{/if}
			{#if vcpus_memory?.memory}
				- {((vcpus_memory?.memory * 1.0) / 1024 / 1024 / 1024).toFixed(2)} GB{/if}
			{#if hasWorkersWithoutIsolation(workers)}
				{@const unsafeWorkers = getWorkersWithoutIsolation(workers)}
				<div class="flex justify-end">
					<Tooltip
						placement="bottom"
						closeButton
						containerClasses="border rounded-lg shadow-lg bg-surface"
					>
						<TriangleAlert size={14} class="text-yellow-600" />

						{#snippet text()}
							<div class="flex flex-col gap-2 text-xs max-w-md p-4">
								<div class="font-semibold text-emphasis">Workers without job isolation</div>
								<p class="text-primary">
									{unsafeWorkers.length}
									{unsafeWorkers.length === 1 ? 'worker' : 'workers'} in this group
									{unsafeWorkers.length === 1 ? 'is' : 'are'} running without job isolation (nsjail/unshare).
								</p>
								<div class="flex flex-wrap gap-1">
									{#each unsafeWorkers as worker}
										<Badge color="orange" small>
											{worker.worker}
										</Badge>
									{/each}
								</div>
								<a href="https://www.windmill.dev/docs/advanced/security_isolation" target="_blank">
									Learn more about job isolation <ExternalLink size={12} class="inline-block" />
								</a>
							</div>
						{/snippet}
					</Tooltip>
				</div>
			{/if}
		</div>
		{#if !isAgent}
			<div class="flex gap-4 items-center justify-end flex-row">
				{#if canEditConfig}
					{#if width > 1000}
						{#if config}
							<Button
								unifiedSize="sm"
								variant="subtle"
								on:click={() => {
									if (!hasEnterpriseFeatures) {
										sendUserToast('Worker Management UI is an EE feature', true)
									} else {
										openDelete = true
									}
								}}
								startIcon={{ icon: Trash }}
								destructive
							>
								Delete config
							</Button>
						{/if}

						<Button
							unifiedSize="sm"
							variant="subtle"
							on:click={() => {
								loadNConfig()
								openClean = true
							}}
							startIcon={{ icon: RefreshCcwIcon }}
							destructive
						>
							Clean cache
						</Button>
					{:else}
						<Dropdown
							items={[
								{
									displayName: 'Clean cache',
									action: () => {
										loadNConfig()
										openClean = true
									},
									disabled: !config,
									type: 'delete'
								},
								{
									displayName: 'Delete config',
									action: () => {
										if (!hasEnterpriseFeatures) {
											sendUserToast('Worker Management UI is an EE feature', true)
										} else {
											openDelete = true
										}
									},
									disabled: !config,
									type: 'delete'
								}
							]}
						/>
					{/if}
					<Button
						variant="accent"
						unifiedSize="sm"
						on:click={() => {
							loadNConfig()
							drawer?.openDrawer()
						}}
						startIcon={{ icon: config == undefined ? Plus : Settings }}
					>
						<div class="flex flex-row gap-1 items-center">
							{config == undefined ? 'Create' : 'Edit'} config
						</div>
					</Button>
				{:else if config}
					<Button
						unifiedSize="md"
						variant="accent"
						on:click={() => {
							loadNConfig()
							drawer?.openDrawer()
						}}
					>
						See config
					</Button>
				{/if}
			</div>
		{/if}
	</div>

	{#if config?.worker_tags && config.worker_tags.length > 0}
		<div class="flex flex-row items-start gap-2 w-full">
			<div class="text-secondary text-xs mt-1">Tags:</div>
			<TagList tags={config.worker_tags} maxVisible={25} class="flex-wrap" />
		</div>
	{:else if config?.dedicated_workers && config.dedicated_workers.length > 0}
		<div class="flex flex-row items-start gap-2 w-full min-w-0">
			<div class="text-secondary text-xs mt-1 flex-shrink-0">Dedicated to:</div>
			<div class="flex flex-wrap gap-1 min-w-0">
				{#each config.dedicated_workers as dw}
					<div
						class="text-xs bg-surface-secondary px-2 py-1 rounded text-primary font-mono truncate max-w-xs"
						title={hashedDedicatedTags.get(dw) ?? dw}
					>
						{hashedDedicatedTags.get(dw) ?? dw}
					</div>
				{/each}
			</div>
		</div>
	{:else if config?.dedicated_worker}
		<div class="flex flex-row items-start gap-2 w-full min-w-0">
			<div class="text-secondary text-xs mt-1 flex-shrink-0">Dedicated to:</div>
			<div
				class="text-xs bg-surface-secondary px-2 py-1 rounded text-primary font-mono truncate max-w-xs"
				title={hashedDedicatedTags.get(config.dedicated_worker) ?? config.dedicated_worker}
			>
				{hashedDedicatedTags.get(config.dedicated_worker) ?? config.dedicated_worker}
			</div>
		</div>
	{/if}
</div>

{#if isAgent}
	<div class="mt-4">
		<Alert type="info" title="Agent Worker Group" size="xs">
			{#snippet children()}
				This group is formed with agent workers, there is no associated config. {#if $superadmin || $devopsRole}
					To modify the tags, generate a new <a
						href="https://www.windmill.dev/docs/core_concepts/agent_workers#quickstart"
						target="_blank"
						class="underline">JWT token <ExternalLink size={12} class="inline-block" /></a
					>.{/if}
			{/snippet}
		</Alert>
	</div>
{/if}
