<script lang="ts">
	import { X } from 'lucide-svelte'
	import { Alert, Button, Drawer } from './common'
	import Multiselect from 'svelte-multiselect'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import { ConfigService } from '$lib/gen'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import { createEventDispatcher } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import { enterpriseLicense, superadmin } from '$lib/stores'
	import Tooltip from './Tooltip.svelte'
	import Editor from './Editor.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Section from './Section.svelte'
	import Label from './Label.svelte'

	export let name: string
	export let config:
		| undefined
		| {
				dedicated_worker?: string
				worker_tags?: string[]
				priority_tags?: Map<string, number>
				cache_clear?: number
				init_bash?: string
		  }
	export let activeWorkers: number

	let nconfig: {
		dedicated_worker?: string
		worker_tags?: string[]
		priority_tags?: Map<string, number>
		cache_clear?: number
		init_bash?: string
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
	}

	let selectedPriorityTags: string[] = []

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
	<DrawerContent on:close={() => drawer.closeDrawer()} title="Edit worker config '{name}'">
		{#if !$enterpriseLicense}
			<Alert type="warning" title="Worker management UI is EE only">
				Workers can still have their WORKER_TAGS passed as env. Dedicated workers are an enterprise
				only feature.
			</Alert>
			<div class="pb-4" />
		{/if}

		<ToggleButtonGroup
			{selected}
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
				label="Dedicated to a script"
			/>
		</ToggleButtonGroup>
		{#if selected == 'normal'}
			<Section label="Tags to listen to">
				{#if nconfig?.worker_tags != undefined}
					<div class="flex gap-3 gap-y-2 flex-wrap pb-2">
						{#each nconfig.worker_tags as tag}
							<div class="flex gap-0.5 items-center"
								><div class="text-2xs p-1 rounded border text-primary">{tag}</div>
								<button
									class="z-10 rounded-full p-1 duration-200 hover:bg-gray-200"
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
								</button></div
							>
						{/each}
					</div>
					<div class="max-w-md">
						<input type="text" placeholder="new tag" bind:value={newTag} />
						<div class="mt-1" />
						<Button
							variant="contained"
							color="blue"
							size="xs"
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
					<div class="max-w-md mt-2 items-center gap-1 pt-2">
						{#if nconfig?.worker_tags !== undefined && nconfig?.worker_tags.length > 0}
							<Label label="High-priority tags">
								<svelte:fragment slot="header">
									<Tooltip>
										Jobs with the following high-priority tags will be picked up in priority by this
										worker.
										{#if !enterpriseLicense}
											This is a feature only available in enterprise edition.
										{/if}
									</Tooltip>
								</svelte:fragment>
								<Multiselect
									outerDivClass="text-secondary"
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
									placeholder="High priority tags"
								/>
							</Label>
						{/if}
					</div>
				{/if}
			</Section>
		{:else if selected == 'dedicated'}
			{#if nconfig?.dedicated_worker != undefined}
				<input
					placeholder="<workspace>:<script path>"
					type="text"
					on:change={() => {
						dirtyCode = true
						dirty = true
					}}
					bind:value={nconfig.dedicated_worker}
				/>
				<div class="py-2"
					><Alert
						type="info"
						title="Script's runtime setting 'dedicated worker' must be toggled on as well"
					/></div
				>
				<p class="text-2xs text-tertiary mt-2"
					>Workers will get killed upon detecting this setting change. It is assumed they are in an
					environment where the supervisor will restart them. Upon restart, they will pick the new
					dedicated worker config.</p
				>
			{/if}
		{/if}
		<div class="mt-4" />

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
						class="flex flex-1 grow h-full w-full"
						automaticLayout
						lang="shell"
						deno={false}
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
							await ConfigService.updateConfig({ name: 'worker__' + name, requestBody: nconfig })
							sendUserToast('Configuration set')
							dispatch('reload')
							dirty = false
							dirtyCode = false
						}}
						disabled={(!dirty && nconfig?.dedicated_worker == undefined) || !$enterpriseLicense}
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
		<span class="text-xs text-secondary"
			>config <Tooltip>{JSON.stringify(config, null, 4)}</Tooltip></span
		>
	{/if}
	{#if activeWorkers > 1}
		<span class="ml-4 text-xs"
			>{activeWorkers} Workers <Tooltip>Number of workers active in the last 10s</Tooltip></span
		>
	{/if}
</div>
