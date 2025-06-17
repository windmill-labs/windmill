<script lang="ts">
	import { userStore, workspaceStore } from '$lib/stores'
	import {
		type Folder,
		FolderService,
		UserService,
		GranularAclService,
		GroupService
	} from '$lib/gen'
	import TableCustom from './TableCustom.svelte'
	import { Alert, Button, Drawer, DrawerContent } from './common'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import GroupEditor from './GroupEditor.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import Section from './Section.svelte'
	import { Eye, Plus } from 'lucide-svelte'
	import Label from './Label.svelte'
	import { sendUserToast } from '$lib/toast'
	import { createEventDispatcher } from 'svelte'
	import Select from './select/Select.svelte'

	export let name: string
	let can_write = false

	type Role = 'viewer' | 'writer' | 'admin'
	let folder: Folder | undefined
	let perms: { owner_name: string; role: Role }[] | undefined = undefined
	let usernames: string[] = []
	let groups: string[] = []
	let ownerItem: string = ''

	let newGroup: Drawer
	let viewGroup: Drawer

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
		await loadFolder()
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
		ownerItem = ''
		loadFolder()
	}

	let folderNotFound: boolean | undefined = undefined

	async function loadFolder(): Promise<void> {
		try {
			folder = await FolderService.getFolder({ workspace: $workspaceStore!, name })
			summary = folder.summary ?? ''
			can_write =
				$userStore != undefined &&
				(folder?.owners.includes('u/' + $userStore.username) ||
					($userStore.is_admin ?? false) ||
					($userStore.is_super_admin ?? false) ||
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
		} catch (e) {
			folderNotFound = true
		}
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
	let groupCreated: string | undefined = undefined
	let newGroupName: string = ''
	let summary: string = ''

	async function addGroup() {
		await GroupService.createGroup({
			workspace: $workspaceStore ?? '',
			requestBody: { name: newGroupName }
		})
		groupCreated = newGroupName
		$userStore?.folders?.push(newGroupName)
		loadGroups()
		ownerItem = newGroupName
	}

	const dispatch = createEventDispatcher()

	async function updateFolder() {
		await FolderService.updateFolder({
			workspace: $workspaceStore ?? '',
			name,
			requestBody: { summary }
		})
		sendUserToast('Folder summary updated')
		dispatch('update')
		loadFolder()
	}
</script>

<Drawer bind:this={newGroup}>
	<DrawerContent
		title="New Group"
		on:close={() => {
			newGroup.closeDrawer()
			groupCreated = undefined
		}}
	>
		{#if !groupCreated}
			<div class="flex flex-row">
				<input class="mr-2" placeholder="New group name" bind:value={newGroupName} />
				<Button size="md" startIcon={{ icon: Plus }} disabled={!newGroupName} on:click={addGroup}>
					New&nbsp;group
				</Button>
			</div>
		{:else}
			<GroupEditor name={groupCreated} />
		{/if}
	</DrawerContent>
</Drawer>

<Drawer bind:this={viewGroup}>
	<DrawerContent title="Group {ownerItem}" on:close={viewGroup.closeDrawer}>
		<GroupEditor name={ownerItem} />
	</DrawerContent>
</Drawer>

<Section label="Metadata" class="mb-4">
	<Label label="Summary">
		<div class="flex flex-row gap-2">
			<input placeholder="Short summary to be displayed when listed" bind:value={summary} />
			<Button size="sm" on:click={updateFolder} disabled={!can_write}>Save</Button>
		</div>
	</Label>
</Section>

<Section label={`Permissions (${perms?.length ?? 0})`}>
	<div class="flex flex-col gap-6">
		{#if can_write}
			<Alert role="info" title="New permissions may take up to 60s to apply">
				<span class="text-xs text-tertiary">Due to permissions cache invalidation </span>
			</Alert>
			<div class="flex items-center gap-1">
				<div>
					<ToggleButtonGroup bind:selected={ownerKind} on:selected={() => (ownerItem = '')}>
						{#snippet children({ item })}
							<ToggleButton value="user" label="User" {item} />
							<ToggleButton value="group" label="Group" {item} />
						{/snippet}
					</ToggleButtonGroup>
				</div>

				{#key ownerKind}
					{@const items =
						ownerKind === 'user'
							? usernames.filter((x) => !perms?.map((y) => y.owner_name).includes('u/' + x))
							: groups.filter((x) => !perms?.map((y) => y.owner_name).includes('g/' + x))}
					<Select items={items.map((x) => ({ label: x, value: x }))} bind:value={ownerItem} />
					{#if ownerKind == 'group'}
						<Button
							title="View Group"
							btnClasses="!p-1.5"
							variant="border"
							size="xs"
							disabled={!ownerItem || ownerItem == ''}
							on:click={viewGroup.openDrawer}
							startIcon={{ icon: Eye }}
							iconOnly
						/>
						<Button
							title="New Group"
							btnClasses="!p-1.5"
							variant="border"
							on:click={newGroup.openDrawer}
							size="xs"
							startIcon={{ icon: Plus }}
							iconOnly
						/>
					{/if}
				{/key}
				<Button
					disabled={ownerItem == ''}
					variant="contained"
					color="blue"
					size="sm"
					on:click={addToFolder}
				>
					Grant
				</Button>
			</div>
		{/if}

		{#if folderNotFound}
			<Alert type="warning" title="Folder not found" size="xs">
				The folder "{name}" does not exist in the workspace. You can create it by clicking the
				button below. An item can seemingly be in a folder given its path without the folder
				existing. A windmill folder has settable permissions that its children inherit. If an item
				is within a non-existing folders, only admins will see it.
			</Alert>
			<Button
				color="light"
				variant="border"
				wrapperClasses="w-min"
				startIcon={{ icon: Plus }}
				size="xs"
				on:click={() => {
					FolderService.createFolder({
						workspace: $workspaceStore ?? '',
						requestBody: { name }
					}).then(() => {
						loadFolder()
					})
				}}
			>
				Create folder "{name}"
			</Button>
		{/if}
		{#if perms}
			<TableCustom>
				<tr slot="header-row">
					<th>user/group</th>
					<th></th>
					<th></th>
				</tr>
				<tbody slot="body">
					{#each perms as { owner_name, role }}<tr>
							<td>{owner_name}</td>
							<td>
								{#if can_write}
									<div>
										<ToggleButtonGroup
											disabled={owner_name == 'u/' + $userStore?.username && !$userStore?.is_admin}
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
												} else if (role == 'writer') {
													await FolderService.removeOwnerToFolder({
														workspace: $workspaceStore ?? '',
														name,
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
															owner: owner_name,
															write: false
														}
													})
												}
												loadFolder()
											}}
										>
											{#snippet children({ item })}
												<ToggleButton
													value="viewer"
													label="Viewer"
													tooltip="A viewer of a folder has read-only access to all the elements (scripts/flows/apps/schedules/resources/variables) inside the folder"
													{item}
												/>

												<ToggleButton
													position="center"
													value="writer"
													label="Writer"
													tooltip="A writer of a folder has read AND write access to all the elements (scripts/flows/apps/schedules/resources/variables) inside the folder"
													{item}
												/>

												<ToggleButton
													position="right"
													value="admin"
													label="Admin"
													tooltip="An admin of a folder has read AND write access to all the elements inside the folders and can manage the permissions as well as add new admins"
													{item}
												/>
											{/snippet}
										</ToggleButtonGroup>
									</div>
								{:else}
									{role}
								{/if}</td
							>
							<td>
								{#if can_write && (owner_name != 'u/' + $userStore?.username || $userStore?.is_admin)}
									<button
										class="ml-2 text-red-500"
										on:click={async () => {
											await Promise.all([
												FolderService.removeOwnerToFolder({
													workspace: $workspaceStore ?? '',
													name,
													requestBody: { owner: owner_name }
												}),
												GranularAclService.removeGranularAcls({
													workspace: $workspaceStore ?? '',
													path: name,
													kind: 'folder',
													requestBody: {
														owner: owner_name
													}
												})
											])

											loadFolder()
										}}>remove</button
									>
								{:else}
									<span class="text-tertiary text-xs">cannot remove yourself</span>
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
				<p class="text-tertiary text-sm">No folder is managing this folder</p>
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
		{:else if folderNotFound === undefined}
			<div class="flex flex-col">
				{#each new Array(6) as _}
					<Skeleton layout={[[2], 0.7]} />
				{/each}
			</div>
		{/if}
	</div>
</Section>
