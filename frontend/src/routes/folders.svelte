<script context="module">
	export function load() {
		return {
			stuff: { title: 'Folders' }
		}
	}
</script>

<script lang="ts">
	import type { Folder } from '$lib/gen'
	import { FolderService } from '$lib/gen'
	import { canWrite } from '$lib/utils'

	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import FolderEditor from '$lib/components/FolderEditor.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { faEdit, faPlus, faTrash } from '@fortawesome/free-solid-svg-icons'
	import { Button, Drawer, DrawerContent } from '$lib/components/common'

	type FolderW = Folder & { canWrite: boolean }

	let newFolderName: string = ''
	let folders: FolderW[] = []
	let folderDrawer: Drawer

	async function loadFolders(): Promise<void> {
		folders = (await FolderService.listFolders({ workspace: $workspaceStore! })).map((x) => {
			return { canWrite: canWrite(x.name, x.extra_perms ?? {}, $userStore), ...x }
		})
	}

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key
		if (key === 'Enter') {
			event.preventDefault()
			addFolder()
		}
	}
	async function addFolder() {
		await FolderService.createFolder({
			workspace: $workspaceStore ?? '',
			requestBody: { name: newFolderName }
		})
		loadFolders()
		editFolderName = newFolderName
		folderDrawer.openDrawer()
	}

	$: {
		if ($workspaceStore && $userStore) {
			loadFolders()
		}
	}

	let editFolderName: string = ''
</script>

<Drawer bind:this={folderDrawer}>
	<DrawerContent title="Folder {editFolderName}" on:close={folderDrawer.closeDrawer}>
		<FolderEditor on:update={loadFolders} name={editFolderName} />
	</DrawerContent>
</Drawer>

<CenteredPage>
	<PageHeader
		title="Folders"
		tooltip="Folder users together to grant roles and homegenous permissions. Same users can be in many folders at the same time."
	>
		<div class="flex flex-row">
			<input
				class="mr-2"
				on:keyup={handleKeyUp}
				placeholder="New folder name"
				bind:value={newFolderName}
			/>
			<Button size="md" startIcon={{ icon: faPlus }} disabled={!newFolderName} on:click={addFolder}>
				New&nbsp;folder
			</Button>
		</div>
	</PageHeader>

	<div class="relative mb-20">
		<TableCustom>
			<tr slot="header-row">
				<th>Name</th>
				<th />
			</tr>
			<tbody slot="body">
				{#if folders.length === 0}
					<tr>
						<td colspan="2" class="text-gray-600 mt-2"> No folders yet, create one! </td>
					</tr>
				{/if}

				{#each folders as { name, extra_perms, canWrite }}
					<tr>
						<td>
							<a
								href="#{name}"
								on:click={() => {
									editFolderName = name
									folderDrawer.openDrawer()
								}}
								>{name}
							</a>
							<div>
								<SharedBadge {canWrite} extraPerms={extra_perms} />
							</div>
						</td>
						<td>
							<Dropdown
								placement="bottom-end"
								dropdownItems={[
									{
										displayName: 'Manage folder',
										icon: faEdit,
										disabled: !canWrite,
										action: () => {
											editFolderName = name
											folderDrawer.openDrawer()
										}
									},
									{
										displayName: 'Delete',

										icon: faTrash,
										type: 'delete',
										disabled: !canWrite,
										action: async () => {
											await FolderService.deleteFolder({ workspace: $workspaceStore ?? '', name })
											loadFolders()
										}
									}
								]}
							/>
						</td>
					</tr>
				{/each}
			</tbody>
		</TableCustom>
	</div>
</CenteredPage>

<style>
</style>
