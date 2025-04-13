<script lang="ts" context="module">
	const lastMetaUsed = writable<Meta | undefined>(undefined)
</script>

<script lang="ts">
	import { pathToMeta, type Meta } from '$lib/common'

	import { localeConcatAnd, pluralize } from '$lib/utils'
	import {
		AppService,
		FlowService,
		FolderService,
		ResourceService,
		ScheduleService,
		ScriptService,
		HttpTriggerService,
		VariableService,
		WebsocketTriggerService,
		KafkaTriggerService,
		PostgresTriggerService,
		NatsTriggerService,
		MqttTriggerService,
		SqsTriggerService,
		GcpTriggerService

	} from '$lib/gen'
	import { superadmin, userStore, workspaceStore } from '$lib/stores'
	import { createEventDispatcher, getContext } from 'svelte'
	import { writable } from 'svelte/store'
	import { Alert, Button, Drawer, DrawerContent } from './common'
	import Badge from './common/badge/Badge.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import FolderEditor from './FolderEditor.svelte'
	import { random_adj } from './random_positive_adjetive'
	import { Eye, Folder, Loader2, Plus, SearchCode, User } from 'lucide-svelte'
	import Tooltip from './Tooltip.svelte'

	type PathKind =
		| 'resource'
		| 'script'
		| 'variable'
		| 'flow'
		| 'schedule'
		| 'app'
		| 'raw_app'
		| 'http_trigger'
		| 'websocket_trigger'
		| 'kafka_trigger'
		| 'postgres_trigger'
		| 'nats_trigger'
		| 'mqtt_trigger'
		| 'sqs_trigger'
		| 'gcp_trigger'
	let meta: Meta | undefined = undefined
	export let fullNamePlaceholder: string | undefined = undefined
	export let namePlaceholder = ''
	export let initialPath: string
	export let path = ''
	export let error = ''
	export let disabled = false
	export let checkInitialPathExistence = false
	export let autofocus = true
	export let dirty = false
	export let kind: PathKind
	export let hideUser: boolean = false

	let inputP: HTMLInputElement | undefined = undefined

	const dispatch = createEventDispatcher()

	let folders: { name: string; write: boolean }[] = []

	$: meta && onMetaChange()

	function onMetaChange() {
		if (meta) {
			path = metaToPath(meta)
			validate(meta, path, kind)
			$lastMetaUsed = {
				...meta,
				name: ''
			}
		}
	}

	function metaToPath(meta: Meta): string {
		return [meta.ownerKind?.charAt(0) ?? '', meta.owner, meta.name].join('/')
	}

	export function focus() {
		inputP?.focus()
	}

	function handleKeyUp(event: KeyboardEvent) {
		setDirty()
		const key = event.key

		if (key === 'Enter') {
			event.preventDefault()
			dispatch('enter')
		}
	}

	export function setName(x: string) {
		if (meta) {
			meta.name = x
			onMetaChange()
		}
	}

	export async function reset() {
		if (path == '' || path == 'u//') {
			if ($lastMetaUsed == undefined || $lastMetaUsed.owner != $userStore?.username) {
				meta = {
					ownerKind: hideUser ? 'folder' : 'user',
					name: fullNamePlaceholder ?? random_adj() + '_' + namePlaceholder,
					owner: ''
				}
				if (!hideUser) {
					if ($userStore?.username?.includes('@')) {
						meta.owner = $userStore!.username.split('@')[0].replace(/[^a-zA-Z0-9_]/g, '')
					} else {
						meta.owner = $userStore!.username!
					}
				}
			} else {
				if ($lastMetaUsed.ownerKind == 'user' && hideUser) {
					meta = {
						ownerKind: 'folder',
						owner: '',
						name: fullNamePlaceholder ?? random_adj() + '_' + namePlaceholder
					}
				} else {
					meta = {
						...$lastMetaUsed,
						name: fullNamePlaceholder ?? random_adj() + '_' + namePlaceholder
					}
				}
			}
			let newMeta = { ...meta }
			while (await pathExists(metaToPath(newMeta), kind)) {
				disabled = true
				error = 'finding an available name...'
				newMeta.name = random_adj() + '_' + (fullNamePlaceholder ?? namePlaceholder)
			}
			error = ''
			disabled = false
			meta = newMeta
			path = metaToPath(meta)
		} else {
			meta = pathToMeta(path, hideUser)
		}
	}

	async function loadFolders(): Promise<void> {
		let initialFolders: { name: string; write: boolean }[] = []
		let initialFolder = ''
		if (initialPath?.split('/')?.[0] == 'f') {
			initialFolder = initialPath?.split('/')?.[1]
			initialFolders.push({ name: initialFolder, write: true })
		}
		folders = initialFolders.concat(
			(
				await FolderService.listFolderNames({
					workspace: $workspaceStore!
				})
			)
				.filter(
					(x) =>
						x != initialFolder &&
						x != 'app_groups' &&
						x != 'app_custom' &&
						x != 'app_themes' &&
						x != 'app_custom'
				)
				.map((x) => ({
					name: x,
					write:
						$userStore?.folders?.includes(x) == true ||
						($userStore?.is_admin ?? false) ||
						($userStore?.is_super_admin ?? false)
				}))
		)
	}

	async function validate(meta: Meta, path: string, kind: PathKind) {
		error = ''
		validateName(meta) && validatePath(path, kind)
	}

	let validateTimeout: NodeJS.Timeout | undefined = undefined

	async function validatePath(path: string, kind: PathKind): Promise<void> {
		if (validateTimeout) {
			clearTimeout(validateTimeout)
		}
		validateTimeout = setTimeout(async () => {
			if (
				(path == '' || checkInitialPathExistence || path != initialPath) &&
				(await pathExists(path, kind))
			) {
				error = 'path already used'
			} else if (meta && validateName(meta)) {
				error = ''
			}
			validateTimeout = undefined
		}, 500)
	}

	async function pathExists(path: string, kind: PathKind): Promise<boolean> {
		if (kind == 'flow') {
			return await FlowService.existsFlowByPath({ workspace: $workspaceStore!, path: path })
		} else if (kind == 'script') {
			return await ScriptService.existsScriptByPath({
				workspace: $workspaceStore!,
				path: path
			})
		} else if (kind == 'resource') {
			return await ResourceService.existsResource({
				workspace: $workspaceStore!,
				path: path
			})
		} else if (kind == 'variable') {
			return await VariableService.existsVariable({
				workspace: $workspaceStore!,
				path: path
			})
		} else if (kind == 'schedule') {
			return await ScheduleService.existsSchedule({ workspace: $workspaceStore!, path: path })
		} else if (kind == 'app') {
			return await AppService.existsApp({ workspace: $workspaceStore!, path: path })
		} else if (kind == 'http_trigger') {
			return await HttpTriggerService.existsHttpTrigger({
				workspace: $workspaceStore!,
				path: path
			})
		} else if (kind == 'websocket_trigger') {
			return await WebsocketTriggerService.existsWebsocketTrigger({
				workspace: $workspaceStore!,
				path: path
			})
		} else if (kind == 'kafka_trigger') {
			return await KafkaTriggerService.existsKafkaTrigger({
				workspace: $workspaceStore!,
				path: path
			})
		} else if (kind == 'postgres_trigger') {
			return await PostgresTriggerService.existsPostgresTrigger({
				workspace: $workspaceStore!,
				path: path
			})
		} else if (kind == 'nats_trigger') {
			return await NatsTriggerService.existsNatsTrigger({
				workspace: $workspaceStore!,
				path: path
			})
		} else if (kind === 'mqtt_trigger') {
			return await MqttTriggerService.existsMqttTrigger({
				workspace: $workspaceStore!,
				path: path
			})
		} else if (kind == 'sqs_trigger') {
			return await SqsTriggerService.existsSqsTrigger({
				workspace: $workspaceStore!,
				path: path
			})
		} else if (kind === 'gcp_trigger') {
			return await GcpTriggerService.existsGcpTrigger({
				workspace: $workspaceStore!,
				path: path
			})
		} else {
			return false
		}
	}

	function validateName(meta: Meta): boolean {
		if (meta.name == undefined || meta.name == '') {
			error = 'Choose a name'
			return false
		} else if (!/^[\w-]+(\/[\w-]+)*$/.test(meta.name)) {
			error = 'This name is not valid'
			return false
		} else if (meta.owner == '' && meta.ownerKind == 'folder') {
			error = 'Folder needs to be chosen'
			return false
		} else if (meta.owner == '' && meta.ownerKind == 'group') {
			error = 'Group needs to be chosen'
			return false
		} else {
			return true
		}
	}

	$: {
		if ($workspaceStore && $userStore) {
			loadFolders()
			initPath()
		}
	}

	function initPath() {
		if (path != undefined && path != '') {
			meta = pathToMeta(path, hideUser)
			onMetaChange()
			return
		}
		if (initialPath == undefined || initialPath == '') {
			reset()
		} else {
			meta = pathToMeta(initialPath, hideUser)
			onMetaChange()
			path = initialPath
		}
	}

	let newFolder: Drawer
	let viewFolder: Drawer
	let newFolderName: string
	let folderCreated: string | undefined = undefined

	async function addFolder() {
		await FolderService.createFolder({
			workspace: $workspaceStore ?? '',
			requestBody: { name: newFolderName }
		})
		folderCreated = newFolderName
		$userStore?.folders?.push(newFolderName)
		loadFolders()
		if (meta) {
			meta.owner = newFolderName
		}
	}

	function setDirty() {
		!dirty && (dirty = true)
	}

	const openSearchWithPrefilledText: (t?: string) => void = getContext(
		'openSearchWithPrefilledText'
	)

	$: displayPathChangedWarning =
		(['flow', 'script', 'resource', 'variable'] as PathKind[]).includes(kind) &&
		initialPath &&
		initialPath !== path

	$: pathUsageInFlowsPromise =
		(kind == 'script' || kind == 'flow') &&
		$workspaceStore &&
		initialPath &&
		FlowService.listFlowPathsFromWorkspaceRunnable({
			workspace: $workspaceStore,
			path: initialPath,
			runnableKind: kind
		})

	$: pathUsageInAppsPromise =
		(kind == 'script' || kind == 'flow') &&
		$workspaceStore &&
		initialPath &&
		AppService.listAppPathsFromWorkspaceRunnable({
			workspace: $workspaceStore,
			path: initialPath,
			runnableKind: kind
		})

	$: pathUsageInScriptsPromise =
		kind == 'script' &&
		$workspaceStore &&
		initialPath &&
		ScriptService.listScriptPathsFromWorkspaceRunnable({
			workspace: $workspaceStore,
			path: initialPath
		})
</script>

<Drawer bind:this={newFolder}>
	<DrawerContent
		title="New Folder"
		on:close={() => {
			newFolder.closeDrawer()
			folderCreated = undefined
		}}
	>
		{#if !folderCreated}
			<div class="flex flex-col gap-2">
				<input placeholder="New folder name" bind:value={newFolderName} />
				<Button size="md" startIcon={{ icon: Plus }} disabled={!newFolderName} on:click={addFolder}>
					New&nbsp;folder
				</Button>
			</div>
		{:else}
			<FolderEditor name={folderCreated} />
		{/if}
	</DrawerContent>
</Drawer>

<Drawer bind:this={viewFolder}>
	<DrawerContent title="Folder {meta?.owner}" on:close={viewFolder.closeDrawer}>
		<FolderEditor name={meta?.owner ?? ''} />
	</DrawerContent>
</Drawer>

<div>
	<div class="flex flex-col sm:flex-row sm:items-center gap-2 sm:gap-4 pb-0 mb-1">
		{#if meta != undefined}
			<!-- svelte-ignore a11y-label-has-associated-control -->
			{#if !hideUser}
				<div class="block">
					<ToggleButtonGroup
						class="mt-0.5"
						bind:selected={meta.ownerKind}
						on:selected={(e) => {
							setDirty()
							const kind = e.detail
							if (meta) {
								if (kind === 'folder') {
									meta.owner = folders?.[0]?.name ?? ''
								} else if (kind === 'group') {
									meta.owner = 'all'
								} else {
									meta.owner = $userStore?.username?.split('@')[0] ?? ''
								}
							}
						}}
						let:item
					>
						<ToggleButton
							icon={User}
							{disabled}
							light
							size="xs"
							value="user"
							position="left"
							label="User"
							{item}
						/>
						<!-- <ToggleButton light size="xs" value="group" position="center">Group</ToggleButton> -->
						<ToggleButton
							icon={Folder}
							{disabled}
							light
							size="xs"
							value="folder"
							position="right"
							label="Folder"
							{item}
						/>
					</ToggleButtonGroup>
				</div>
			{/if}
			{#if !hideUser}
				<div class="text-xl">/</div>
			{/if}
			<div>
				{#if meta.ownerKind === 'user'}
					<label class="block shrink min-w-0">
						<input
							class="!w-36"
							type="text"
							bind:value={meta.owner}
							placeholder={$userStore?.username ?? ''}
							disabled={disabled || !($superadmin || ($userStore?.is_admin ?? false))}
							on:keydown={setDirty}
						/>
					</label>
				{:else if meta.ownerKind === 'folder'}
					<label class="block grow w-48">
						<div class="flex flex-row items-center gap-1 w-full">
							<select class="grow w-full" {disabled} bind:value={meta.owner}>
								{#if folders?.length == 0}
									<option disabled>No folders</option>
								{/if}
								{#each folders as { name, write }}
									<option disabled={!write}>{name}{write ? '' : ' (read-only)'}</option>
								{/each}
							</select>
							<Button
								title="View folder"
								btnClasses="!p-1.5"
								variant="border"
								color="light"
								size="xs"
								disabled={!meta.owner || meta.owner == ''}
								on:click={viewFolder.openDrawer}
								iconOnly
								startIcon={{ icon: Eye }}
							/>
							<Button
								title="New folder"
								btnClasses="!p-1.5"
								variant="border"
								color="light"
								size="xs"
								{disabled}
								on:click={newFolder.openDrawer}
								iconOnly
								startIcon={{ icon: Plus }}
							/>
						</div>
					</label>
				{/if}
			</div>
			<span class="text-xl">/</span>
			<label class="block grow w-full max-w-md">
				<!-- svelte-ignore a11y-autofocus -->
				<input
					{disabled}
					type="text"
					id="path"
					{autofocus}
					bind:this={inputP}
					autocomplete="off"
					on:keyup={handleKeyUp}
					bind:value={meta.name}
					placeholder={namePlaceholder}
					class={error === ''
						? ''
						: 'border border-red-700 bg-red-100 border-opacity-30 focus:border-red-700 focus:border-opacity-30 focus-visible:ring-red-700 focus-visible:ring-opacity-25 focus-visible:border-red-700'}
				/>
			</label>
		{/if}
	</div>

	<div class="flex flex-col w-full mt-4">
		<div class="flex justify-start w-full">
			<Badge
				color="gray"
				class="center-center !bg-surface-secondary !text-tertiary !w-[70px] !h-[24px] rounded-r-none border"
			>
				Full path
			</Badge>
			<input
				type="text"
				readonly
				value={path}
				size={path?.length || 50}
				class="font-mono !text-xs max-w-[calc(100%-70px)] !w-auto !h-[24px] !py-0 !border-l-0 !rounded-l-none"
				on:focus={({ currentTarget }) => {
					currentTarget.select()
				}}
			/>
			<!-- <span class="font-mono text-sm break-all">{path}</span> -->
		</div>
		<div class="text-red-600 dark:text-red-400 text-2xs mt-1.5">{error}</div>
	</div>

	{#if pathUsageInFlowsPromise || pathUsageInAppsPromise || pathUsageInScriptsPromise}
		{#await Promise.all( [pathUsageInAppsPromise, pathUsageInFlowsPromise, pathUsageInScriptsPromise] )}
			<Loader2 class="animate-spin" size={16} />
		{:then [apps, flows, scripts]}
			{#if (apps && apps.length) || (flows && flows.length) || (scripts && scripts.length)}
				<p class="text-xs">
					Used by {localeConcatAnd([
						...(scripts && scripts.length ? [pluralize(scripts.length, 'script')] : []),
						...(flows && flows.length ? [pluralize(flows.length, 'flow')] : []),
						...(apps && apps.length ? [pluralize(apps.length, 'app')] : [])
					])}
					<Tooltip>
						<ul>
							{#each scripts || [] as path}
								<li><a target="_blank" href="/scripts/edit/{path}">{path}</a></li>
							{/each}
							{#each flows || [] as path}
								<li><a target="_blank" href="/flows/edit/{path}">{path}</a></li>
							{/each}
							{#each apps || [] as path}
								<li><a target="_blank" href="/apps/edit/{path}">{path}</a></li>
							{/each}
						</ul>
					</Tooltip>
				</p>
				{#if displayPathChangedWarning}
					<Alert
						type="warning"
						class="mt-4"
						title="Moving this item will break the following referencing it:"
					>
						<ul class="list-disc">
							{#each scripts || [] as scriptPath}
								<li>
									<a href={`/scripts/edit/${scriptPath}`} class="text-blue-400" target="_blank">
										{scriptPath}
									</a>
								</li>
							{/each}
							{#each flows || [] as flowPath}
								<li>
									<a href={`/flows/edit/${flowPath}`} class="text-blue-400" target="_blank">
										{flowPath}
									</a>
								</li>
							{/each}
							{#each apps || [] as appPath}
								<li>
									<a href={`/apps/edit/${appPath}`} class="text-blue-400" target="_blank">
										{appPath}
									</a>
								</li>
							{/each}
						</ul>
					</Alert>
				{/if}
			{/if}
		{/await}
	{:else if displayPathChangedWarning}
		<Alert type="warning" class="mt-4" title="Moving may break other items relying on it">
			You are renaming an item that may be depended upon by other items. This may break apps, flows
			or resources. Find if it used elsewhere using the content search. Note that linked variables
			and resources (having the same path) are automatically moved together.
			<div class="flex pt-2">
				<Button
					variant="border"
					color="dark"
					on:click={() => {
						openSearchWithPrefilledText('#')
					}}
					startIcon={{ icon: SearchCode }}
				>
					Search
				</Button>
			</div>
		</Alert>
	{/if}
</div>

<style>
	input:disabled {
		background: rgba(200, 200, 200, 0.267);
	}
</style>
