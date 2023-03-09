<script lang="ts">
	import { type Meta, pathToMeta } from '$lib/common'

	import {
		AppService,
		FlowService,
		FolderService,
		GroupService,
		ResourceService,
		ScheduleService,
		ScriptService,
		VariableService
	} from '$lib/gen'
	import { superadmin, userStore, workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import Required from './Required.svelte'
	import { Button, Drawer, DrawerContent } from './common'
	import { faEye, faPlus } from '@fortawesome/free-solid-svg-icons'
	import ToggleButtonGroup from './common/toggleButton/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton/ToggleButton.svelte'
	import { Icon } from 'svelte-awesome'
	import Tooltip from './Tooltip.svelte'
	import FolderEditor from './FolderEditor.svelte'
	import GroupEditor from './GroupEditor.svelte'
	import { random_adj } from './random_positive_adjetive'

	type PathKind = 'resource' | 'script' | 'variable' | 'flow' | 'schedule' | 'app'
	let meta: Meta | undefined = undefined
	export let namePlaceholder = ''
	export let initialPath: string
	export let path = ''
	export let error = ''
	export let disabled = false
	export let checkInitialPathExistence = false

	export let kind: PathKind

	let inputP: HTMLInputElement | undefined = undefined

	const dispatch = createEventDispatcher()

	let folders: string[] = []
	let groups: string[] = []

	$: meta && onMetaChange()

	function onMetaChange() {
		if (meta) {
			path = metaToPath(meta)
			validate(meta, path, kind)
		}
	}

	function metaToPath(meta: Meta): string {
		return [meta.ownerKind?.charAt(0) ?? '', meta.owner, meta.name].join('/')
	}

	export function focus() {
		inputP?.focus()
	}

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key

		if (key === 'Enter') {
			event.preventDefault()
			dispatch('enter')
		}
	}

	export async function reset() {
		if (path == '' || path == 'u//') {
			meta = {
				ownerKind: 'user',
				name: random_adj() + '_' + namePlaceholder,
				owner: ''
			}

			meta.owner = $userStore!.username.split('@')[0]

			let newMeta = { ...meta }
			while (await pathExists(metaToPath(newMeta), kind)) {
				disabled = true
				error = 'finding an available name...'
				newMeta.name = random_adj() + '_' + namePlaceholder
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
		let initialFolders: string[] = []
		let initialFolder = ''
		if (initialPath?.split('/')?.[0] == 'f') {
			initialFolder = initialPath?.split('/')?.[1]
			initialFolders.push(initialFolder)
		}
		folders = initialFolders.concat(
			(
				await FolderService.listFolderNames({
					workspace: $workspaceStore!
				})
			).filter((x) => x != initialFolder)
		)
	}

	async function loadGroups(): Promise<void> {
		let initialGroups: string[] = []
		if (initialPath?.split('/')?.[0] == 'f') {
			initialGroups.push(initialPath?.split('/')?.[1])
		}
		groups = initialGroups.concat(
			await GroupService.listGroupNames({
				workspace: $workspaceStore!
			})
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
			loadGroups()
			initPath()
		}
	}

	function initPath() {
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
		if (meta) {
			meta.owner = newFolderName
		}
		loadFolders()
	}

	let newGroup: Drawer
	let viewGroup: Drawer
	let newGroupName: string
	let groupCreated: string | undefined = undefined

	async function addGroup() {
		await GroupService.createGroup({
			workspace: $workspaceStore ?? '',
			requestBody: { name: newGroupName }
		})
		groupCreated = newGroupName
		if (meta) {
			meta.owner = newGroupName
		}
		loadGroups()
	}
</script>

<Drawer bind:this={newGroup}>
	<DrawerContent
		title="New Folder"
		on:close={() => {
			newGroup.closeDrawer()
			groupCreated = undefined
		}}
	>
		<div class="flex flex-row">
			<input class="mr-2" placeholder="New group name" bind:value={newGroupName} />
			<Button size="md" endIcon={{ icon: faPlus }} disabled={!newGroupName} on:click={addGroup}>
				New&nbsp;group
			</Button>
		</div>
		{#if groupCreated}
			<div class="mt-8" />
			<GroupEditor name={groupCreated} />
		{/if}
	</DrawerContent>
</Drawer>
<Drawer bind:this={viewGroup}>
	<DrawerContent title="Folder {meta?.owner}" on:close={viewGroup.closeDrawer}>
		<GroupEditor name={meta?.owner ?? ''} />
	</DrawerContent>
</Drawer>

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
				<Button
					size="md"
					startIcon={{ icon: faPlus }}
					disabled={!newFolderName}
					on:click={addFolder}
				>
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
	<div class="flex flex-col sm:flex-row items-center gap-2 sm:gap-4 pb-0 mb-1">
		{#if meta != undefined}
			<div class="flex gap-4 shrink">
				<label class="block">
					<span class="text-gray-700 text-sm whitespace-nowrap">&nbsp;</span>

					<ToggleButtonGroup
						class="mt-0.5"
						bind:selected={meta.ownerKind}
						on:selected={(e) => {
							const kind = e.detail
							if (meta) {
								if (kind === 'folder') {
									meta.owner = $userStore?.folders?.[0] ?? ''
								} else if (kind === 'group') {
									meta.owner = 'all'
								} else {
									meta.owner = $userStore?.username?.split('@')[0] ?? ''
								}
							}
						}}
					>
						<ToggleButton light size="xs" value="user" position="left">User</ToggleButton>
						<!-- <ToggleButton light size="xs" value="group" position="center">Group</ToggleButton> -->
						<ToggleButton light size="xs" value="folder" position="right">Folder</ToggleButton>
					</ToggleButtonGroup>
				</label>
				{#if meta.ownerKind === 'user'}
					<label class="block shrink min-w-0">
						<span class="text-gray-700 text-sm">User</span>
						<input
							class="!w-36"
							type="text"
							bind:value={meta.owner}
							placeholder={$userStore?.username ?? ''}
							disabled={!($superadmin || ($userStore?.is_admin ?? false))}
						/>
					</label>
				{:else if meta.ownerKind === 'folder'}
					<label class="block grow w-48">
						<span class="text-gray-700 text-sm"
							>Folder <Tooltip
								>Read and write permissions are given to groups and users at the folder level and
								shared by all items inside the folder.</Tooltip
							></span
						>

						<div class="flex flex-row gap-1 w-full">
							<select class="grow w-full" {disabled} bind:value={meta.owner}>
								{#each folders as f}
									<option>{f}</option>
								{/each}
							</select>
							<Button variant="border" size="xs" on:click={viewFolder.openDrawer}>
								<Icon scale={0.8} data={faEye} /></Button
							>
							<Button variant="border" size="xs" on:click={newFolder.openDrawer}>
								<Icon scale={0.8} data={faPlus} /></Button
							></div
						>
					</label>
				{:else if meta.ownerKind === 'group'}
					<label class="block grow w-48">
						<span class="text-gray-700 text-sm"
							>Group <Tooltip>Item will be owned by the group and hence all its member</Tooltip
							></span
						>

						<div class="flex flex-row gap-1">
							<select class="grow w-full" {disabled} bind:value={meta.owner}>
								{#each groups as g}
									<option>{g}</option>
								{/each}
							</select>
							<Button variant="border" size="xs" on:click={viewGroup.openDrawer}>
								<Icon scale={0.8} data={faEye} /></Button
							>
							<Button variant="border" size="xs" on:click={newGroup.openDrawer}>
								<Icon scale={0.8} data={faPlus} /></Button
							></div
						>
					</label>
				{/if}
			</div>
			<label class="block grow w-full max-w-md">
				<span class="text-gray-700 text-sm">
					Name
					<Required required={true} />
				</span>
				<input
					{disabled}
					type="text"
					id="path"
					autofocus
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

	<div class="flex-row flex justify-between w-full">
		<div><span class="font-mono text-sm break-all">{path}</span></div>
		<div class="text-red-600 text-2xs">{error}</div>
	</div>
</div>

<style>
	input:disabled {
		background: rgba(200, 200, 200, 0.267);
	}
</style>
