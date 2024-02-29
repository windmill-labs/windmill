<script lang="ts">
	import { Plus, X } from 'lucide-svelte'
	import { Alert, Button, Drawer } from './common'
	import Multiselect from 'svelte-multiselect'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import { ConfigService, Preview } from '$lib/gen'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import { createEventDispatcher } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import { emptyString } from '$lib/utils'
	import { enterpriseLicense, superadmin } from '$lib/stores'
	import Tooltip from './Tooltip.svelte'
	import Editor from './Editor.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Section from './Section.svelte'
	import Label from './Label.svelte'
	import AutoComplete from 'simple-svelte-autocomplete'

	export let name: string
	export let config:
		| undefined
		| {
				dedicated_worker?: string
				worker_tags?: string[]
				priority_tags?: Map<string, number>
				cache_clear?: number
				init_bash?: string
				additional_python_paths?: string[]
				pip_local_dependencies?: string[]
		  }
	export let activeWorkers: number
	export let customTags: string[] | undefined

	let nconfig: {
		dedicated_worker?: string
		worker_tags?: string[]
		priority_tags?: Map<string, number>
		cache_clear?: number
		init_bash?: string
		env_vars_static?: Map<string, string>
		env_vars_allowlist?: string[]
		additional_python_paths?: string[]
		pip_local_dependencies?: string[]
	} = {}

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
			nconfig.priority_tags = new Map<string, number>()
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

	let selectedPriorityTags: string[] = []
	let customEnvVars: {
		key: string
		type: 'static' | 'dynamic'
		value: string | undefined
	}[] = []

	const defaultTags = [
		'deno',
		'python3',
		'go',
		'bash',
		'powershell',
		'dependency',
		'flow',
		'hub',
		'other',
		'bun'
	]
	const nativeTags = [
		'nativets',
		'postgresql',
		'mysql',
		'graphql',
		'snowflake',
		'mssql',
		'bigquery'
	]

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

	let newTag: string = ''
	$: selected = nconfig?.dedicated_worker != undefined ? 'dedicated' : 'normal'
	$: {
		selectedPriorityTags = []
		if (nconfig?.priority_tags !== undefined) {
			for (const [tag, _] of Object.entries(nconfig?.priority_tags)) {
				selectedPriorityTags.push(tag)
			}
		}
	}

	const dispatch = createEventDispatcher()

	async function deleteWorkerGroup() {
		await ConfigService.deleteConfig({ name: 'worker__' + name })
		dispatch('reload')
	}
	let dirty = false
	let dirtyCode = false
	let openDelete = false
	let openClean = false

	let drawer: Drawer

	let createdTags: string[] = []
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
		on:close={() => drawer.closeDrawer()}
		title={$superadmin ? `Edit worker config '${name}'` : `Worker config '${name}'`}
	>
		{#if !$enterpriseLicense}
			<Alert type="warning" title="Worker management UI is EE only">
				Workers can still have their WORKER_TAGS, INIT_SCRIPT and WHITELIST_ENVS passed as env.
				Dedicated workers are an enterprise only feature.
			</Alert>
			<div class="pb-4" />
		{/if}

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
			class="mb-4"
		>
			<ToggleButton position="left" value="normal" size="sm" label="Any jobs within worker tags" />
			<ToggleButton
				position="dedicated"
				value="dedicated"
				size="sm"
				label="Dedicated to a script/flow"
			/>
		</ToggleButtonGroup>
		{#if selected == 'normal'}
			<Section label="Tags to listen to">
				{#if nconfig?.worker_tags != undefined}
					<div class="flex gap-3 gap-y-2 flex-wrap pb-2">
						{#each nconfig.worker_tags as tag}
							<div class="flex gap-0.5 items-center"
								><div class="text-2xs p-1 rounded border text-primary">{tag}</div>
								{#if $superadmin}
									<button
										class={'z-10 rounded-full p-1 duration-200 hover:bg-gray-200'}
										aria-label="Remove item"
										on:click|preventDefault|stopPropagation={() => {
											if (nconfig != undefined) {
												dirty = true
												nconfig.worker_tags = nconfig?.worker_tags?.filter((t) => t != tag) ?? []
												if (nconfig.priority_tags) {
													delete nconfig.priority_tags[tag]
												}
												selectedPriorityTags = selectedPriorityTags.filter((t) => t != tag) ?? []
											}
										}}
									>
										<X size={12} />
									</button>
								{/if}</div
							>
						{/each}
					</div>
					{#if $superadmin}
						<div class="max-w-md">
							<AutoComplete
								noInputStyles
								items={[
									...(customTags ?? []),
									...createdTags,
									...defaultTags,
									...nativeTags
								].filter((x) => !nconfig?.worker_tags?.includes(x))}
								bind:selectedItem={newTag}
								hideArrow={true}
								inputClassName={'flex !font-gray-600 !font-primary !bg-surface-primary"'}
								dropdownClassName="!text-sm !py-2 !rounded-sm  !border-gray-200 !border !shadow-md"
								className="w-full !font-gray-600 !font-primary !bg-surface-primary"
								onFocus={() => {
									dispatch('focus')
								}}
								create
								onCreate={(c) => {
									createdTags.push(c)
									createdTags = [...createdTags]
									return c
								}}
								createText="Press enter to use this non-predefined value"
							/>

							<div class="mt-1" />
							<div class="flex">
								<Button
									variant="contained"
									color="blue"
									size="xs"
									startIcon={{ icon: Plus }}
									disabled={newTag == '' || nconfig.worker_tags?.includes(newTag)}
									on:click={() => {
										if (nconfig != undefined) {
											nconfig.worker_tags = [
												...(nconfig?.worker_tags ?? []),
												newTag.replaceAll(' ', '_')
											]
											newTag = ''
											dirty = true
										}
									}}
								>
									Add tag
								</Button>
							</div>
						</div>
						<div class="flex flex-wrap mt-2 items-center gap-1 pt-2">
							<Button
								variant="contained"
								color="light"
								size="xs"
								on:click={() => {
									if (nconfig != undefined) {
										nconfig.worker_tags = defaultTags.concat(nativeTags)
										dirty = true
									}
								}}
							>
								Reset to all tags <Tooltip>{defaultTags.concat(nativeTags).join(', ')}</Tooltip>
							</Button>
							<Button
								variant="contained"
								color="light"
								size="xs"
								on:click={() => {
									if (nconfig != undefined) {
										nconfig.worker_tags = defaultTags
										dirty = true
									}
								}}
							>
								Reset to all tags minus native ones <Tooltip>{defaultTags.join(', ')}</Tooltip>
							</Button>
							<Button
								variant="contained"
								color="light"
								size="xs"
								on:click={() => {
									if (nconfig != undefined) {
										nconfig.worker_tags = nativeTags
										dirty = true
									}
								}}
							>
								Reset to native tags <Tooltip>{nativeTags.join(', ')}</Tooltip>
							</Button>
						</div>
						<div class="max-w mt-2 items-center gap-1 pt-2">
							{#if nconfig?.worker_tags !== undefined && nconfig?.worker_tags.length > 0}
								<Label label="High-priority tags">
									<svelte:fragment slot="header">
										<Tooltip>
											Jobs with the following high-priority tags will be picked up in priority by
											this worker.
											{#if !enterpriseLicense}
												This is a feature only available in enterprise edition.
											{/if}
										</Tooltip>
									</svelte:fragment>
									<Multiselect
										outerDivClass="text-secondary !bg-surface-disabled"
										disabled={!$enterpriseLicense}
										bind:selected={selectedPriorityTags}
										on:change={(e) => {
											if (e.detail.type === 'add') {
												if (nconfig.priority_tags) {
													nconfig.priority_tags[e.detail.option] = 100
												}
												dirty = true
											} else if (e.detail.type === 'remove') {
												if (nconfig.priority_tags) {
													delete nconfig.priority_tags[e.detail.option]
												}
												dirty = true
											} else {
												console.error(
													`Priority tags multiselect - unknown event type: '${e.detail.type}'`
												)
											}
										}}
										options={nconfig?.worker_tags}
										selectedOptionsDraggable={false}
										ulOptionsClass={'!bg-surface-secondary'}
										placeholder="High priority tags"
									/>
								</Label>
							{/if}
						</div>
					{/if}
				{/if}
			</Section>
		{:else if selected == 'dedicated'}
			{#if nconfig?.dedicated_worker != undefined}
				<input
					disabled={!$superadmin}
					placeholder="<workspace>:<script path>"
					type="text"
					on:change={() => {
						dirtyCode = true
						dirty = true
					}}
					bind:value={nconfig.dedicated_worker}
				/>
				{#if $superadmin}
					<div class="py-2"
						><Alert
							type="info"
							title="Script's runtime setting 'dedicated worker' must be toggled on as well"
						/></div
					>
					<p class="text-2xs text-tertiary mt-2"
						>Workers will get killed upon detecting this setting change. It is assumed they are in
						an environment where the supervisor will restart them. Upon restart, they will pick the
						new dedicated worker config.</p
					>
				{/if}
			{/if}
		{/if}

		<div class="mt-8" />
		<Section
			label="Python runtime settings"
			collapsable={true}
			tooltip="Add Python runtime specific settings like additional python paths and PIP local dependencies"
		>
			<div class="flex flex-col gap-3 gap-y-2 pb-2 max-w">
				<span class="text-sm text-primary">Additional Python Paths</span>
				{#each nconfig.additional_python_paths ?? [] as additional_python_path, i}
					<div class="flex gap-1 items-center">
						<input
							type="text"
							disabled={!$superadmin}
							placeholder="/path/to/python3.X/site-packages"
							bind:value={additional_python_path}
						/>
						{#if $superadmin}
							<button
								class="rounded-full bg-surface/60 hover:bg-gray-200"
								aria-label="Clear"
								on:click={() => {
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
				{#if $superadmin}
					<div class="flex">
						<Button
							variant="contained"
							color="blue"
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
							Add Additional Python Path
						</Button>
					</div>
				{/if}

				<span class="text-sm text-primary">PIP local dependencies</span>
				{#each nconfig.pip_local_dependencies ?? [] as pip_local_dependency, i}
					<div class="flex gap-1 items-center">
						<input
							disabled={!$superadmin}
							type="text"
							placeholder="httpx"
							bind:value={pip_local_dependency}
						/>
						{#if $superadmin}
							<button
								class="rounded-full bg-surface/60 hover:bg-gray-200"
								aria-label="Clear"
								on:click={() => {
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
				{#if $superadmin}
					<div class="flex">
						<Button
							variant="contained"
							color="blue"
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
			</div>
		</Section>

		<div class="mt-8" />

		<Section
			label="Environment Variables passed to Jobs"
			collapsable={true}
			tooltip="Add static and dynamic environment variables that will be passed to jobs handled by this worker group. Dynamic environment variable values will be loaded from the worker host environment variables while static environment variables will be set directly from their values below."
		>
			<div class="flex flex-col gap-3 gap-y-2 pb-2 max-w">
				{#each customEnvVars as envvar, i}
					<div class="flex gap-1 items-center">
						<input
							disabled={!$superadmin}
							type="text"
							placeholder="ENV_VAR_NAME"
							bind:value={envvar.key}
						/>
						<ToggleButtonGroup
							disabled={!$superadmin}
							class="w-128"
							bind:selected={envvar.type}
							on:selected={(e) => {
								dirty = true
								if (e.detail === 'dynamic') {
									envvar.value = undefined
								}
							}}
						>
							<ToggleButton position="left" value="dynamic" label="Dynamic" />
							<ToggleButton position="right" value="static" label="Static" />
						</ToggleButtonGroup>
						<input
							type="text"
							disabled={!$superadmin || envvar.type === 'dynamic'}
							placeholder={envvar.type === 'dynamic'
								? 'value read from worker env var'
								: 'static value'}
							bind:value={envvar.value}
						/>
						{#if $superadmin}
							<button
								class="rounded-full bg-surface/60 hover:bg-gray-200"
								aria-label="Clear"
								on:click={() => {
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
							>
								<X size={14} />
							</button>
						{/if}
					</div>
				{/each}
				{#if $superadmin}
					<div class="flex">
						<Button
							variant="contained"
							color="blue"
							size="xs"
							startIcon={{ icon: Plus }}
							on:click={() => {
								customEnvVars.push({ key: '', type: 'dynamic', value: undefined })
								customEnvVars = [...customEnvVars]
								dirty = true
							}}
						>
							Add Environment Variable
						</Button>
					</div>
				{/if}
			</div>
			{#if !superadmin}
				<div class="flex flex-wrap items-center gap-1 pt-2">
					<Button
						variant="contained"
						color="light"
						size="xs"
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
						variant="contained"
						color="light"
						size="xs"
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
		</Section>
		<div class="mt-8" />

		<Section
			label="Init Script"
			tooltip="Bash scripts run at start of the workers. More lightweight than having to require the worker images at the cost of being run on every start."
		>
			<div class="flex gap-4 py-2 pb-6 items-baseline w-full">
				<div class="border w-full h-40">
					{#if dirtyCode}
						<div class="text-red-600 text-sm"
							>Init script has changed, once applied, the workers will restart to apply it.</div
						>
					{/if}
					<Editor
						disabled={!$superadmin}
						class="flex flex-1 grow h-full w-full"
						automaticLayout
						lang="shell"
						scriptLang={Preview.language.BASH}
						useWebsockets={false}
						fixedOverflowWidgets={false}
						listenEmptyChanges
						code={config?.init_bash ?? ''}
						on:change={(e) => {
							if (config) {
								dirty = true
								dirtyCode = true
								const code = e.detail
								if (code != '') {
									nconfig.init_bash = code
								} else {
									nconfig.init_bash = undefined
								}
							}
						}}
					/>
				</div>
			</div>
		</Section>
		<svelte:fragment slot="actions">
			<div class="flex gap-4 items-center">
				<div class="flex gap-2 items-center">
					{#if dirty}
						<div class="text-red-600 text-xs whitespace-nowrap">Non applied changes</div>
					{/if}
					<Button
						variant="contained"
						color="dark"
						on:click={async () => {
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
							dirtyCode = false
						}}
						disabled={(!dirty && nconfig?.dedicated_worker == undefined) ||
							!$enterpriseLicense ||
							!$superadmin}
					>
						Apply changes
					</Button>
				</div>
			</div>
		</svelte:fragment>
	</DrawerContent>
</Drawer>

<div class="flex gap-2 items-center"
	><h4 class="py-4 truncate w-40 text-primary">{name}</h4>
	{#if $superadmin}
		<Button
			color="light"
			size="xs"
			on:click={() => {
				dirty = false
				loadNConfig()
				drawer.openDrawer()
			}}
		>
			<div class="flex flex-row gap-1 items-center">
				{config == undefined ? 'create' : 'edit'} config
			</div>
		</Button>

		{#if config}
			<Button
				color="light"
				size="xs"
				on:click={() => {
					if (!$enterpriseLicense) {
						sendUserToast('Worker Management UI is an EE feature', true)
					} else {
						openDelete = true
					}
				}}
				btnClasses="text-red-400"
			>
				delete config
			</Button>
		{/if}

		<Button color="light" size="xs" on:click={() => (openClean = true)} btnClasses="text-red-400">
			clean cache
		</Button>
	{:else if config}
		<Button
			color="light"
			size="xs"
			on:click={() => {
				loadNConfig()
				drawer.openDrawer()
			}}
		>
			<div class="flex flex-row gap-1 items-center"> config </div>
		</Button>
	{/if}
	{#if activeWorkers > 1}
		<span class="ml-4 text-xs"
			>{activeWorkers} Workers <Tooltip>Number of workers active in the last 10s</Tooltip></span
		>
	{/if}
</div>
