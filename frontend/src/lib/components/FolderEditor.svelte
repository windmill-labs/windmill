<script lang="ts">
	import { userStore, workspaceStore } from '$lib/stores'
	import {
		type Folder,
		FolderService,
		UserService,
		GranularAclService,
		GroupService
	} from '$lib/gen'
	import AutoComplete from 'simple-svelte-autocomplete'
	import TableCustom from './TableCustom.svelte'
	import { Alert, Button, ToggleButton, ToggleButtonGroup } from './common'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import Tooltip from './Tooltip.svelte'

	export let name: string
	let can_write = false

	type Role = 'viewer' | 'writer' | 'admin'
	let folder: Folder | undefined
	let perms: { owner_name: string; role: Role }[] | undefined = undefined
	let managing_folders: string[] = []
	let usernames: string[] = []
	let groups: string[] = []
	let ownerItem: string = ''
	let folders: string[] = []

	async function loadUsernames(): Promise<void> {
		usernames = await UserService.listUsernames({ workspace: $workspaceStore! })
	}

	async function loadGroups(): Promise<void> {
		groups = await GroupService.listGroupNames({ workspace: $workspaceStore! })
	}

	$: {
		if ($workspaceStore && $userStore) {
			load()
		}
	}

	async function load() {
		loadUsernames()
		loadGroups()
		loadFolders()
		await loadFolder()
	}

	async function loadFolders(): Promise<void> {
		folders = (await FolderService.listFolders({ workspace: $workspaceStore! })).map((x) => x.name)
	}

	async function addToFolder() {
		await GranularAclService.addGranularAcls({
			workspace: $workspaceStore ?? '',
			path: name,
			kind: 'folder',
			requestBody: {
				owner: (ownerKind == 'user' ? 'u/' : 'g/') + ownerItem
			}
		})
		loadFolder()
	}

	async function loadFolder(): Promise<void> {
		folder = await FolderService.getFolder({ workspace: $workspaceStore!, name })
		can_write =
			$userStore != undefined &&
			(folder?.owners.includes('u/' + $userStore.username) ||
				($userStore.is_admin ?? false) ||
				$userStore.pgroups.findIndex((x) => folder?.owners.includes(x)) != -1)

		perms = Array.from(
			new Set(
				Object.entries(folder?.extra_perms ?? {})
					.map((x) => x[0])
					.concat(folder?.owners ?? [])
			)
		).map((x) => {
			return {
				owner_name: x,
				role: getRole(x)
			}
		})
		managing_folders = Object.entries(folder?.extra_perms ?? {})
			.filter(([k, v]) => k.startsWith('g/') && v)
			.map(([k, v]) => k)
	}

	function getRole(x: string): Role {
		const viewer = x in (folder?.extra_perms ?? {})
		const writer = viewer && (folder?.extra_perms ?? {})[x]
		const owner = folder?.owners?.includes(x)

		if (owner) {
			return 'admin'
		} else if (writer) {
			return 'writer'
		} else {
			return 'viewer'
		}
	}
	let ownerKind: 'user' | 'group' = 'user'
</script>

<div class="flex flex-col gap-6">
	<h1>{name}</h1>
	<h2>Permissions ({perms?.length ?? 0})</h2>
	{#if can_write}
		<Alert role="info" title="New permissions may take up to 60s to apply"
			><span class="text-xs text-gray-500">Due to permissions cache invalidation</span></Alert
		>
		<div class="flex items-center gap-1">
			<div>
				<ToggleButtonGroup bind:selected={ownerKind} on:selected={() => (ownerItem = '')}>
					<ToggleButton position="left" value="user" size="xs">User</ToggleButton>
					<ToggleButton position="right" value="group" size="xs">Group</ToggleButton>
				</ToggleButtonGroup>
			</div>
			{#key ownerKind}
				<AutoComplete
					items={ownerKind === 'user' ? usernames : groups}
					bind:selectedItem={ownerItem}
				/>
			{/key}
			<Button
				disabled={ownerItem == ''}
				variant="contained"
				color="blue"
				size="sm"
				on:click={addToFolder}
			>
				Grant permission to folder
			</Button>
		</div>
	{/if}
	{#if perms}
		<TableCustom>
			<tr slot="header-row">
				<th>user/group</th>
				<th />
				<th />
			</tr>
			<tbody slot="body">
				{#each perms as { owner_name, role }}<tr>
						<td>{owner_name}</td>
						<td>
							{#if can_write}
								<div>
									<ToggleButtonGroup
										selected={role}
										on:selected={async (e) => {
											const role = e.detail
											// const wasInFolder = (folder?.owners ?? []).includes(folder)
											// const inAcl = (
											// 	folder?.extra_perms ? Object.keys(folder?.extra_perms) : []
											// ).includes(folder)
											if (role == 'admin') {
												await FolderService.addOwnerToFolder({
													workspace: $workspaceStore ?? '',
													name,
													requestBody: {
														owner: owner_name
													}
												})
												await GranularAclService.addGranularAcls({
													workspace: $workspaceStore ?? '',
													path: name,
													kind: 'folder',
													requestBody: {
														owner: owner_name
													}
												})
											} else if (role == 'writer') {
												await FolderService.removeOwnerToFolder({
													workspace: $workspaceStore ?? '',
													name,
													requestBody: {
														owner: owner_name
													}
												})
												await GranularAclService.addGranularAcls({
													workspace: $workspaceStore ?? '',
													path: name,
													kind: 'folder',
													requestBody: {
														owner: owner_name,
														write: true
													}
												})
											} else if (role == 'viewer') {
												await FolderService.removeOwnerToFolder({
													workspace: $workspaceStore ?? '',
													name,
													requestBody: {
														owner: owner_name
													}
												})
												await GranularAclService.addGranularAcls({
													workspace: $workspaceStore ?? '',
													path: name,
													kind: 'folder',
													requestBody: {
														owner: owner_name,
														write: false
													}
												})
											}
											loadFolder()
										}}
									>
										<ToggleButton position="left" value="viewer" size="xs"
											>Viewer&nbsp;<Tooltip
												>A viewer of a folder has read-only access to all the elements
												(scripts/flows/apps/schedules/resources/variables) inside the folder</Tooltip
											></ToggleButton
										>
										<ToggleButton position="center" value="writer" size="xs"
											>Writer&nbsp;<Tooltip
												>A viewer of a folder has read AND write access to all the elements
												(scripts/flows/apps/schedules/resources/variables) inside the folder</Tooltip
											></ToggleButton
										>
										<ToggleButton position="right" value="admin" size="xs"
											>Admin&nbsp;<Tooltip
												>An admin of a folder has read AND write access to all the elements inside
												the folders and can manage the permissions as well as add new admins</Tooltip
											></ToggleButton
										>
									</ToggleButtonGroup>
								</div>
							{:else}
								{role}
							{/if}</td
						>
						<td>
							{#if can_write}
								<button
									class="ml-2 text-red-500"
									on:click={async () => {
										await FolderService.removeOwnerToFolder({
											workspace: $workspaceStore ?? '',
											name,
											requestBody: { owner: owner_name }
										})
										await GranularAclService.removeGranularAcls({
											workspace: $workspaceStore ?? '',
											path: name,
											kind: 'folder',
											requestBody: {
												owner: owner_name
											}
										})
										loadFolder()
									}}>remove</button
								>
							{/if}</td
						>
					</tr>{/each}
			</tbody>
		</TableCustom>
		<!-- <h2 class="mt-10"
			>Folders managing this folder <Tooltip
				>Any owner of those folders can manage this folder</Tooltip
			></h2
		>
		{#if can_write}
			<div class="flex items-start">
				<AutoComplete items={folders} bind:selectedItem={new_managing_folder} />
				<Button
					variant="contained"
					color="blue"
					size="sm"
					btnClasses="!ml-4"
					on:click={addToManagingFolder}
				>
					Add folder managing this folder
				</Button>
			</div>
		{/if} -->
		<!-- {#if managing_folders.length == 0}
			<p class="text-gray-600 text-sm">No folder is managing this folder</p>
		{:else}
			<TableCustom>
				<tr slot="header-row">
					<th>folder</th>
					<th />
				</tr>
				<tbody slot="body">
					{#each managing_folders as managing_folder}<tr>
							<td>{managing_folder.split('/')[1]}</td>
							<td>
								{#if can_write}
									<button
										class="ml-2 text-red-500"
										on:click={async () => {
											await GranularAclService.removeGranularAcls({
												workspace: $workspaceStore ?? '',
												path: name,
												kind: 'folder',
												requestBody: {
													owner: managing_folder
												}
											})
											loadFolder()
										}}>remove</button
									>
								{/if}</td
							>
						</tr>{/each}
				</tbody>
			</TableCustom>
		{/if} -->
	{:else}
		<div class="flex flex-col">
			{#each new Array(6) as _}
				<Skeleton layout={[[2], 0.7]} />
			{/each}
		</div>
	{/if}
</div>
