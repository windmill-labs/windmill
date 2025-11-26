<script lang="ts">
	import {
		AlertTriangle,
		Copy,
		Plus,
		RefreshCcwIcon,
		RotateCcw,
		Settings,
		Trash,
		X
	} from 'lucide-svelte'
	import { Alert, Button, Drawer } from './common'
	import Badge from './common/badge/Badge.svelte'
	import Popover from './meltComponents/Popover.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import { ConfigService, WorkspaceService, type WorkerPing, type Workspace } from '$lib/gen'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import { createEventDispatcher } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import { emptyString, pluralize } from '$lib/utils'
	import { enterpriseLicense, superadmin, devopsRole } from '$lib/stores'
	import Tooltip from './Tooltip.svelte'
	import Editor from './Editor.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Section from './Section.svelte'
	import Label from './Label.svelte'
	import YAML from 'yaml'
	import Toggle from './Toggle.svelte'
	import { defaultTags, nativeTags, type AutoscalingConfig } from './worker_group'
	import AutoscalingConfigEditor from './AutoscalingConfigEditor.svelte'
	import TagsToListenTo from './TagsToListenTo.svelte'
	import Select from './select/Select.svelte'
	import MultiSelect from './select/MultiSelect.svelte'
	import { safeSelectItems } from './select/utils.svelte'
	import Subsection from './Subsection.svelte'
	import TextInput from './text_input/TextInput.svelte'

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
			? config.worker_tags != undefined || config.dedicated_worker != undefined
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
		defaultTagPerWorkspace?: boolean | undefined
	}

	let {
		name,
		config = $bindable(),
		activeWorkers,
		customTags,
		workers,
		defaultTagPerWorkspace = undefined
	}: Props = $props()

	let workspaces: Workspace[] = $state([])
	async function listWorkspaces() {
		workspaces = await WorkspaceService.listWorkspacesAsSuperAdmin()
	}

	const dispatch = createEventDispatcher()

	async function deleteWorkerGroup() {
		await ConfigService.deleteConfig({ name: 'worker__' + name })
		dispatch('reload')
	}
	let dirty = $state(false)
	let openDelete = $state(false)
	let openClean = $state(false)

	let drawer: Drawer | undefined = $state()
	let vcpus_memory = $derived(computeVCpuAndMemory(workers))
	let selected = $derived(nconfig?.dedicated_worker != undefined ? 'dedicated' : 'normal')
	$effect(() => {
		;($superadmin || $devopsRole) && listWorkspaces()
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
		title={$superadmin || $devopsRole ? `Edit worker config '${name}'` : `Worker config '${name}'`}
	>
		{#if !$enterpriseLicense}
			<Alert type="warning" title="Worker management UI is EE only">
				Workers can still have their WORKER_TAGS, INIT_SCRIPT and WHITELIST_ENVS passed as env.
				Dedicated workers are an enterprise only feature.
			</Alert>
		{/if}
		<Label label="Workers assignment">
			<ToggleButtonGroup
				{selected}
				disabled={!$superadmin}
				on:selected={(e) => {
					dirty = true
					if (nconfig == undefined) {
						nconfig = {}
					}
					if (e.detail == 'dedicated') {
						nconfig.dedicated_worker = ''
						nconfig.worker_tags = undefined
					} else {
						nconfig.dedicated_worker = undefined
						nconfig.worker_tags = []
					}
				}}
			>
				{#snippet children({ item })}
					<ToggleButton value="normal" label="Any jobs within worker tags" {item} />
					<ToggleButton value="dedicated" label="Dedicated to a script/flow" {item} />
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
										nconfig.worker_tags = defaultTags.concat(nativeTags)
									}
								},
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
								tooltip: (defaultTagPerWorkspace && workspaceTag
									? nativeTags.map((nt) => `${nt}-${workspaceTag}`)
									: nativeTags
								).join(', ')
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

										dirty = true
									}
								}}
								dropdownItems={dropdownResetToAllTags}
								dropdownWidth={300}
								startIcon={{ icon: RotateCcw }}
							>
								Reset to all tags <Tooltip
									>{(defaultTagPerWorkspace && workspaceTag
										? defaultTags.concat(nativeTags).map((nt) => `${nt}-${workspaceTag}`)
										: defaultTags.concat(nativeTags)
									).join(', ')}</Tooltip
								>
							</Button>

							{#if defaultTagPerWorkspace}
								<Select
									bind:value={workspaceTag}
									items={workspaces.map((w) => ({ value: w.id }))}
									onCreateItem={(c) => (workspaceTag = c)}
									placeholder="Workspace ID"
								/>
							{/if}
						</div>
					{/if}
				{/snippet}
				{#if nconfig?.worker_tags != undefined}
					<TagsToListenTo
						on:dirty={() => {
							dirty = true
						}}
						on:deletePriorityTag={(e) => {
							const tag = e.detail
							if (nconfig.priority_tags) {
								delete nconfig.priority_tags[tag]
							}
						}}
						bind:worker_tags={nconfig.worker_tags}
						{customTags}
					/>
				{/if}
			</Label>

			{#if nconfig?.worker_tags !== undefined && nconfig?.worker_tags.length > 0}
				<div class="mt-8"></div>
				<Label label="High-priority tags">
					{#snippet header()}
						<Tooltip>
							Jobs with the following high-priority tags will be picked up in priority by this
							worker.
							{#if !enterpriseLicense}
								This is a feature only available in enterprise edition.
							{/if}
						</Tooltip>
					{/snippet}
					<MultiSelect
						disabled={!$enterpriseLicense}
						bind:value={
							() => (nconfig.priority_tags ? Object.keys(nconfig.priority_tags) : []),
							(v) => {
								nconfig.priority_tags = Object.fromEntries(v.map((k) => [k, 100]))
								dirty = true
							}
						}
						items={safeSelectItems(nconfig?.worker_tags)}
					/>
				</Label>
			{/if}

			{#if nconfig !== undefined}
				<div class="mt-8"></div>
				<Label label="Alerts" tooltip="Alert is sent to the configured critical error channels">
					<Toggle
						size="sm"
						options={{
							right: 'Send an alert when the number of alive workers falls below a given threshold'
						}}
						checked={nconfig?.min_alive_workers_alert_threshold !== undefined}
						on:change={(ev) => {
							if (nconfig !== undefined) {
								nconfig.min_alive_workers_alert_threshold = ev.detail ? 1 : undefined
								dirty = true
							}
						}}
						disabled={!$enterpriseLicense}
					/>
					{#if nconfig.min_alive_workers_alert_threshold !== undefined}
						<div class="flex flex-row items-center text-xs gap-2">
							<p>Triggered when number of workers in group is lower than</p>
							<input
								type="number"
								class="!w-14 text-center"
								disabled={!$enterpriseLicense}
								min="1"
								bind:value={nconfig.min_alive_workers_alert_threshold}
								onchange={(ev) => {
									dirty = true
								}}
							/>
						</div>
					{/if}
				</Label>
			{/if}
		{:else if selected == 'dedicated'}
			<div class="flex flex-col gap-2">
				{#if $superadmin || $devopsRole}
					<div class="py-2">
						<Alert
							size="xs"
							type="info"
							title="Script's runtime setting 'dedicated worker' must be toggled on as well"
						/>
					</div>
				{/if}
				{#if nconfig?.dedicated_worker != undefined}
					<div
						><p class="text-xs text-secondary mb-2"
							>Workers will get killed upon detecting changes. It is assumed they are in an
							environment where the supervisor will restart them.</p
						>
						<input
							disabled={!($superadmin || $devopsRole)}
							placeholder="<workspace>:<script path>"
							type="text"
							onchange={() => {
								dirty = true
							}}
							bind:value={nconfig.dedicated_worker}
						/></div
					>
				{/if}
			</div>
		{/if}

		<div class="mt-8"></div>

		<Label
			label="Environment variables passed to jobs"
			tooltip="Add static and dynamic environment variables that will be passed to jobs handled by this worker group. Dynamic environment variable values will be loaded from the worker host environment variables while static environment variables will be set directly from their values below."
		>
			<div class="flex flex-col gap-y-2 pb-2 max-w">
				{#each customEnvVars as envvar, i}
					<div class="flex gap-1 items-center">
						<input
							disabled={!($superadmin || $devopsRole)}
							type="text"
							placeholder="ENV_VAR_NAME"
							bind:value={envvar.key}
							onkeypress={(e) => {
								dirty = true
							}}
						/>
						<ToggleButtonGroup
							disabled={!($superadmin || $devopsRole)}
							class="w-128"
							bind:selected={envvar.type}
							on:selected={(e) => {
								dirty = true
								if (e.detail === 'dynamic') {
									envvar.value = undefined
								}
							}}
						>
							{#snippet children({ item })}
								<ToggleButton value="dynamic" label="Dynamic" {item} />
								<ToggleButton value="static" label="Static" {item} />
							{/snippet}
						</ToggleButtonGroup>
						<TextInput
							inputProps={{
								type: 'text',
								disabled: !($superadmin || $devopsRole) || envvar.type === 'dynamic',
								placeholder:
									envvar.type === 'dynamic' ? 'value read from worker env var' : 'static value'
							}}
							bind:value={envvar.value}
						/>
						{#if $superadmin || $devopsRole}
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
									dirty = true
								}}
								startIcon={{ icon: Trash }}
								iconOnly
								destructive
							/>
						{/if}
					</div>
				{/each}
				{#if $superadmin || $devopsRole}
					<div class="flex">
						<Button
							variant="default"
							unifiedSize="md"
							startIcon={{ icon: Plus }}
							on:click={() => {
								customEnvVars.push({ key: '', type: 'dynamic', value: undefined })
								customEnvVars = [...customEnvVars]
								dirty = true
							}}
						>
							Add environment variable
						</Button>
					</div>
				{/if}
			</div>
			{#if !($superadmin || $devopsRole)}
				<div class="flex flex-wrap items-center gap-1 pt-2">
					<Button
						variant="subtle"
						unifiedSize="md"
						on:click={() => {
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
								dirty = true
							}
						}}
					>
						AWS env var preset <Tooltip
							>{`${aws_env_vars_preset.join(
								', '
							)} - see https://docs.aws.amazon.com/fr_fr/cli/latest/userguide/cli-configure-envvars.html for more options`}</Tooltip
						>
					</Button>
					<Button
						variant="subtle"
						unifiedSize="md"
						on:click={() => {
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
								dirty = true
							}
						}}
					>
						SSL env var preset <Tooltip>{`${ssl_env_vars_preset.join(', ')}`}</Tooltip>
					</Button>
				</div>
			{/if}
		</Label>
		<div class="mt-8"></div>

		<AutoscalingConfigEditor
			onDirty={() => (dirty = true)}
			worker_tags={config?.worker_tags}
			bind:config={nconfig.autoscaling}
		/>

		<div class="mt-8"></div>
		<Section label="Python dependencies overrides" collapsable={true} class="flex flex-col gap-y-6">
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
								disabled={!($superadmin || $devopsRole)}
								placeholder="/path/to/python3.X/site-packages"
								bind:value={nconfig.additional_python_paths![i]}
							/>
							{#if $superadmin || $devopsRole}
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
										dirty = true
									}}
								>
									<X size={14} />
								</button>
							{/if}
						</div>
					{/each}
				{/if}
				{#if $superadmin || $devopsRole}
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
								dirty = true
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
								disabled={!($superadmin || $devopsRole)}
								type="text"
								placeholder="httpx"
								bind:value={nconfig.pip_local_dependencies[i]}
							/>
							{#if $superadmin || $devopsRole}
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
										dirty = true
									}}
								>
									<X size={14} />
								</button>
							{/if}
						</div>
					{/each}
				{/if}
				{#if $superadmin || $devopsRole}
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
								dirty = true
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
			label="Worker scripts"
			tooltip="Bash scripts for worker initialization and maintenance. Init scripts run at worker start, periodic scripts run at configurable intervals."
		>
			{#if $superadmin || $devopsRole}
				<div class="mb-4">
					<Alert size="xs" type="info" title="Worker restart required">
						Workers will get killed upon detecting any changes in this section (scripts or
						interval). It is assumed they are in an environment where the supervisor will restart
						them.
					</Alert>
				</div>
			{/if}

			<div class="space-y-6">
				<div>
					<div class="text-xs text-secondary mb-1">
						Run at start of the workers. More lightweight than requiring custom worker images.
					</div>
					<Subsection
						label="Init script"
						collapsable
						openInitially={nconfig.init_bash !== undefined}
					>
						{#snippet header()}
							<div class="ml-4 flex flex-row gap-2 items-center">
								{#if nconfig.init_bash !== undefined}
									<Badge color="green">Enabled</Badge>
								{/if}
							</div>
						{/snippet}
						<div class="border w-full h-40">
							<Editor
								fixedOverflowWidgets={true}
								disabled={!($superadmin || $devopsRole)}
								class="flex flex-1 grow h-full w-full"
								automaticLayout
								scriptLang={'bash'}
								useWebsockets={false}
								code={config?.init_bash ?? ''}
								on:change={(e) => {
									if (config) {
										dirty = true
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
					</Subsection>
				</div>
				<div>
					<div class="text-xs text-secondary mb-1">
						Run periodically at configurable intervals. Useful for maintenance tasks like cleaning
						disk space.
					</div>
					<Subsection
						label="Periodic script"
						collapsable
						openInitially={nconfig.periodic_script_bash !== undefined}
					>
						{#snippet header()}
							<div class="ml-4 flex flex-row gap-2 items-center">
								{#if nconfig.periodic_script_bash !== undefined}
									<Badge color="green">Enabled</Badge>
								{/if}
							</div>
						{/snippet}

						<div class="flex gap-4 items-center mb-4">
							<Label class="text-sm">Execution interval (seconds):</Label>
							<input
								disabled={!($superadmin || $devopsRole)}
								type="number"
								min="60"
								placeholder="3600"
								class="!w-24 text-center"
								bind:value={nconfig.periodic_script_interval_seconds}
								onchange={() => {
									dirty = true
								}}
							/>
							<span class="text-xs text-hint">Minimum: 60 seconds</span>
						</div>

						<div class="border w-full h-40">
							<Editor
								disabled={!($superadmin || $devopsRole)}
								class="flex flex-1 grow h-full w-full"
								automaticLayout
								scriptLang={'bash'}
								useWebsockets={false}
								fixedOverflowWidgets={false}
								code={config?.periodic_script_bash ?? ''}
								on:change={(e) => {
									if (config) {
										dirty = true
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
					</Subsection>
				</div>
			</div>
		</Section>
		{#snippet actions()}
			<div class="flex gap-4 items-center">
				<div class="flex gap-2 items-center">
					{#if dirty}
						<div class="text-red-500 text-xs whitespace-nowrap">Non applied changes</div>
					{/if}
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
							dirty = false
						}}
						disabled={(!dirty && nconfig?.dedicated_worker == undefined) ||
							!$enterpriseLicense ||
							!($superadmin || $devopsRole)}
					>
						Apply changes
					</Button>
				</div>
			</div>
		{/snippet}
	</DrawerContent>
</Drawer>

<div class="flex flex-col gap-2">
	<div class="flex items-center justify-between mt-2">
		<div class="text-xs flex flex-row gap-2 items-center"
			>{pluralize(activeWorkers, 'worker')}
			{#if vcpus_memory?.vcpus}
				- {(vcpus_memory?.vcpus / 100000).toFixed(2)} vCPUs{/if}
			{#if vcpus_memory?.memory}
				- {((vcpus_memory?.memory * 1.0) / 1024 / 1024 / 1024).toFixed(2)} GB{/if}
			{#if hasWorkersWithoutIsolation(workers)}
				{@const unsafeWorkers = getWorkersWithoutIsolation(workers)}
				<div class="flex justify-end">
					<Popover
						placement="bottom"
						closeButton
						containerClasses="border rounded-lg shadow-lg bg-surface"
					>
						{#snippet trigger()}
							<Button
								nonCaptureEvent
								startIcon={{ icon: AlertTriangle, classes: 'text-yellow-600' }}
								unifiedSize="md"
								variant="subtle"
								btnClasses="text-3xs"
							>
								Workers without isolation
							</Button>
						{/snippet}
						{#snippet content()}
							<div class="flex flex-col gap-2 text-sm max-w-md p-4">
								<div class="font-semibold">Workers without job isolation</div>
								<p class="text-secondary">
									{unsafeWorkers.length}
									{unsafeWorkers.length === 1 ? 'worker' : 'workers'} in this group
									{unsafeWorkers.length === 1 ? 'is' : 'are'} running without job isolation (nsjail/unshare).
								</p>
								<div class="flex flex-wrap gap-1">
									{#each unsafeWorkers as worker}
										<Badge color="orange" verySmall={true}>
											{worker.worker}
										</Badge>
									{/each}
								</div>
								<a
									href="https://www.windmill.dev/docs/advanced/security_isolation"
									target="_blank"
									class="text-accent hover:underline text-xs"
								>
									Learn more about job isolation →
								</a>
							</div>
						{/snippet}
					</Popover>
				</div>
			{/if}
		</div>
		<div class="flex gap-2 items-center justify-end flex-row">
			{#if $superadmin}
				{#if config}
					<Button
						unifiedSize="md"
						variant="subtle"
						on:click={() => {
							if (!$enterpriseLicense) {
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
					unifiedSize="md"
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

				<Button
					unifiedSize="md"
					variant="subtle"
					on:click={() => {
						navigator.clipboard.writeText(
							YAML.stringify({
								name,
								...config
							})
						)
						sendUserToast('Worker config copied to clipboard as YAML')
					}}
					startIcon={{ icon: Copy }}
				>
					Copy config
				</Button>

				<Button
					variant="accent"
					unifiedSize="md"
					on:click={() => {
						dirty = false
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
					Config
				</Button>
			{/if}
		</div>
	</div>
</div>
