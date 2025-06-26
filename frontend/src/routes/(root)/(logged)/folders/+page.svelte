<script lang="ts">
	import type { Folder } from '$lib/gen'
	import { FolderService } from '$lib/gen'

	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import FolderEditor from '$lib/components/FolderEditor.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { userStore, workspaceStore, userWorkspaces } from '$lib/stores'
	import { Button, Drawer, DrawerContent, Skeleton } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import FolderInfo from '$lib/components/FolderInfo.svelte'
	import FolderUsageInfo from '$lib/components/FolderUsageInfo.svelte'
	import { sendUserToast } from '$lib/utils'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import { Pen, Trash, Plus } from 'lucide-svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import { untrack } from 'svelte'

	type FolderW = Folder & { canWrite: boolean }

	let newFolderName: string = $state('')
	let folders: FolderW[] | undefined = $state(undefined)
	let folderDrawer: Drawer | undefined = $state()

	async function loadFolders(): Promise<void> {
		folders = (await FolderService.listFolders({ workspace: $workspaceStore! })).map((x) => {
			return {
				canWrite:
					$userStore != undefined &&
					($userStore?.is_admin ||
						$userStore?.is_super_admin ||
						$userStore?.folders_owners.includes(x.name)),
				...x
			}
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
		folderDrawer?.openDrawer()
	}

	$effect(() => {
		if ($workspaceStore && $userStore) {
			untrack(() => {
				loadFolders()
			})
		}
	})

	let editFolderName: string = $state('')

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

{#if $userStore?.operator && $workspaceStore && !$userWorkspaces.find((_) => _.id === $workspaceStore)?.operator_settings?.folders}
	<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4 m-4 mt-12" role="alert">
		<p class="font-bold">Unauthorized</p>
		<p>Page not available for operators</p>
	</div>
{:else}
	<CenteredPage>
		<PageHeader
			title="Folders"
			tooltip="Folders allow to group items such as scripts/flows/resources/schedule together and to grant homogenous RBAC permissions to groups and individual users towards them."
			documentationLink="https://www.windmill.dev/docs/core_concepts/groups_and_folders"
		>
			<div class="flex flex-row">
				<Popover
					floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
					contentClasses="flex flex-col gap-2 p-4"
				>
					{#snippet trigger()}
						<Button size="md" startIcon={{ icon: Plus }} nonCaptureEvent>New folder</Button>
					{/snippet}
					{#snippet content({ close })}
						<input
							class="mr-2"
							onkeyup={(e) => handleKeyUp(e, () => close())}
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
									close()
								}}
							>
								Create
							</Button>
						</div>
					{/snippet}
				</Popover>
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

						{#each folders as { name, extra_perms, owners, canWrite, summary } (name)}
							<Row
								hoverable
								on:click={() => {
									editFolderName = name
									folderDrawer?.openDrawer()
								}}
							>
								<Cell first>
									<span class="text-blue-500">{name}</span>
									{#if summary}
										<br />
										<span class="text-gray-500">{summary}</span>
									{/if}
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
													folderDrawer?.openDrawer()
												}
											},
											{
												displayName: `Delete${canWrite ? '' : ' (require owner permissions)'}`,
												icon: Trash,
												type: 'delete',
												disabled: !canWrite,
												action: async () => {
													try {
														await FolderService.deleteFolder({
															workspace: $workspaceStore ?? '',
															name
														})
													} catch (e) {
														sendUserToast(e.body, true)
														loadFolders()
													}
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
{/if}
