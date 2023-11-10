<script lang="ts" context="module">
	const lastMetaUsed = writable<Meta | undefined>(undefined)
</script>

<script lang="ts">
	import { pathToMeta, type Meta } from '$lib/common'

	import {
		AppService,
		FlowService,
		FolderService,
		ResourceService,
		ScheduleService,
		ScriptService,
		VariableService
	} from '$lib/gen'
	import { superadmin, userStore, workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import { writable } from 'svelte/store'
	import { Button, Drawer, DrawerContent } from './common'
	import Badge from './common/badge/Badge.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import FolderEditor from './FolderEditor.svelte'
	import { random_adj } from './random_positive_adjetive'
	import Required from './Required.svelte'
	import Tooltip from './Tooltip.svelte'
	import { Eye, Folder, Plus, User } from 'lucide-svelte'

	type PathKind = 'resource' | 'script' | 'variable' | 'flow' | 'schedule' | 'app' | 'raw_app'
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
					ownerKind: 'user',
					name: fullNamePlaceholder ?? random_adj() + '_' + namePlaceholder,
					owner: ''
				}
				if ($userStore?.username?.includes('@')) {
					meta.owner = $userStore!.username.split('@')[0].replace(/[^a-zA-Z0-9_]/g, '')
				} else {
					meta.owner = $userStore!.username!
				}
			} else {
				meta = {
					...$lastMetaUsed,
					name: fullNamePlaceholder ?? random_adj() + '_' + namePlaceholder
				}
			}
			let newMeta = { ...meta }
			while (await pathExists(metaToPath(newMeta), kind)) {
				disabled = true
				error = 'finding an available name...'
				newMeta.name = random_adj() + '_' + fullNamePlaceholder ?? namePlaceholder
			}
			error = ''
			disabled = false
			meta = newMeta
			path = metaToPath(meta)
		} else {
			meta = pathToMeta(path)
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
						($userStore?.folders?.includes(x) == true ?? false) ||
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
			error = 'Folder need to be chosen'
			return false
		} else if (meta.owner == '' && meta.ownerKind == 'group') {
			error = 'Group need to be chosen'
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
			meta = pathToMeta(path)
			onMetaChange()
			return
		}
		if (initialPath == undefined || initialPath == '') {
			reset()
		} else {
			meta = pathToMeta(initialPath)
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
			<div class="flex flex-row">
				<input class="mr-2" placeholder="New folder name" bind:value={newFolderName} />
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
			<div class="flex gap-x-4 shrink">
				<!-- svelte-ignore a11y-label-has-associated-control -->
				<label class="block">
					<span class="text-secondary text-sm whitespace-nowrap">&nbsp;</span>

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
					>
						<ToggleButton
							icon={User}
							{disabled}
							light
							size="xs"
							value="user"
							position="left"
							label="User"
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
						/>
					</ToggleButtonGroup>
				</label>
				{#if meta.ownerKind === 'user'}
					<label class="block shrink min-w-0">
						<span class="text-secondary text-sm">User</span>
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
						<span class="text-secondary text-sm">
							Folder
							<Tooltip
								documentationLink="https://www.windmill.dev/docs/core_concepts/groups_and_folders"
							>
								Read and write permissions are given to groups and users at the folder level and
								shared by all items inside the folder.
							</Tooltip>
						</span>

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
			<label class="block grow w-full max-w-md">
				<span class="text-secondary text-sm">
					Name
					<Required required={true} />
				</span>
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
		<div class="text-red-600 dark:text-red-400 text-2xs">{error}</div>
	</div>
</div>

<style>
	input:disabled {
		background: rgba(200, 200, 200, 0.267);
	}
</style>
