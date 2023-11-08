<script lang="ts">
	import type { Folder } from '$lib/gen'
	import { FolderService } from '$lib/gen'

	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import FolderEditor from '$lib/components/FolderEditor.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Button, Drawer, DrawerContent, Popup, Skeleton } from '$lib/components/common'
	import FolderInfo from '$lib/components/FolderInfo.svelte'
	import FolderUsageInfo from '$lib/components/FolderUsageInfo.svelte'
	import { canWrite } from '$lib/utils'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import { Pen, Trash, Plus } from 'lucide-svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Row from '$lib/components/table/Row.svelte'

	type FolderW = Folder & { canWrite: boolean }

	let newFolderName: string = ''
	let folders: FolderW[] | undefined = undefined
	let folderDrawer: Drawer

	async function loadFolders(): Promise<void> {
		folders = (await FolderService.listFolders({ workspace: $workspaceStore! })).map((x) => {
			return { canWrite: canWrite('f/' + x.name, x.extra_perms ?? {}, $userStore), ...x }
		})
	}

	function handleKeyUp(event: KeyboardEvent, close: () => void) {
		const key = event.key
		if (key === 'Enter') {
			event.preventDefault()
			addFolder()
			close()
		}
	}
	async function addFolder() {
		await FolderService.createFolder({
			workspace: $workspaceStore ?? '',
			requestBody: { name: newFolderName }
		})
		$userStore?.folders.push(newFolderName)
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

	function computeMembers(owners: string[], extra_perms: Record<string, any>) {
		const members = new Set(owners)
		for (const [user, _] of Object.entries(extra_perms)) {
			members.add(user)
		}
		return Array.from(members)
	}
</script>

<Drawer bind:this={folderDrawer}>
	<DrawerContent title="Folder {editFolderName}" on:close={folderDrawer.closeDrawer}>
		<FolderEditor on:update={loadFolders} name={editFolderName} />
	</DrawerContent>
</Drawer>

<CenteredPage>
	<PageHeader
		title="Folders"
		tooltip="Folders allow to group items such as scripts/flows/resources/schedule together and to grant homogenous RBAC permissions to groups and individual users towards them."
		documentationLink="https://www.windmill.dev/docs/core_concepts/groups_and_folders"
	>
		<div class="flex flex-row">
			<Popup
				let:close
				floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
				containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
			>
				<svelte:fragment slot="button">
					<Button size="md" startIcon={{ icon: Plus }} nonCaptureEvent>New folder name</Button>
				</svelte:fragment>
				<div class="flex flex-col gap-2">
					<input
						class="mr-2"
						on:keyup={(e) => handleKeyUp(e, () => close(null))}
						placeholder="New folder name"
						bind:value={newFolderName}
					/>

					<div>
						<Button
							size="md"
							startIcon={{ icon: Plus }}
							disabled={!newFolderName}
							on:click={() => {
								addFolder()
								close(null)
							}}
						>
							Create
						</Button>
					</div>
				</div>
			</Popup>
		</div>
	</PageHeader>

	<div class="relative mb-20 pt-8">
		<DataTable>
			<Head>
				<tr>
					<Cell head first>Name</Cell>
					<Cell head class="w-20">Scripts</Cell>
					<Cell head class="w-20">Flows</Cell>
					<Cell head class="w-20">Apps</Cell>
					<Cell head class="w-20">Schedules</Cell>
					<Cell head class="w-20">Variables</Cell>
					<Cell head class="w-20">Resources</Cell>
					<Cell head class="w-20">Participants</Cell>
					<Cell head last />
				</tr>
			</Head>
			<tbody class="divide-y">
				{#if folders === undefined}
					{#each new Array(4) as _}
						<tr>
							<td colspan="9">
								<Skeleton layout={[[2]]} />
							</td>
						</tr>
					{/each}
				{:else}
					{#if folders.length === 0}
						<tr>
							<td colspan="4" class="text-tertiary mt-2">No folders yet, create one!</td>
						</tr>
					{/if}

					{#each folders as { name, extra_perms, owners, canWrite } (name)}
						<Row
							hoverable
							on:click={() => {
								editFolderName = name
								folderDrawer.openDrawer()
							}}
						>
							<Cell first>
								<span class="text-blue-500">{name}</span>
							</Cell>
							<FolderUsageInfo {name} tabular />

							<Cell><FolderInfo members={computeMembers(owners, extra_perms)} /></Cell>
							<Cell shouldStopPropagation>
								<Dropdown
									items={[
										{
											displayName: 'Manage folder',
											icon: Pen,
											disabled: !canWrite,
											action: () => {
												editFolderName = name
												folderDrawer.openDrawer()
											}
										},
										{
											displayName: 'Delete',
											icon: Trash,
											type: 'delete',
											disabled: !canWrite,
											action: async () => {
												await FolderService.deleteFolder({
													workspace: $workspaceStore ?? '',
													name
												})
												loadFolders()
											}
										}
									]}
								/>
							</Cell>
						</Row>
					{/each}
				{/if}
			</tbody>
		</DataTable>
	</div>
</CenteredPage>
