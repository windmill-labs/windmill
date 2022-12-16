<script context="module">
	export function load() {
		return {
			stuff: { title: 'Groups' }
		}
	}
</script>

<script lang="ts">
	import type { Group } from '$lib/gen'
	import { GroupService } from '$lib/gen'
	import { canWrite } from '$lib/utils'

	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import GroupEditor from '$lib/components/GroupEditor.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { faEdit, faPlus, faShare, faTrash } from '@fortawesome/free-solid-svg-icons'
	import { Button, Drawer, DrawerContent } from '$lib/components/common'

	type GroupW = Group & { canWrite: boolean }

	let newGroupName: string = ''
	let groups: GroupW[] = []
	let shareModal: ShareModal
	let groupDrawer: Drawer

	async function loadGroups(): Promise<void> {
		groups = (await GroupService.listGroups({ workspace: $workspaceStore! })).map((x) => {
			return { canWrite: canWrite(x.name, x.extra_perms ?? {}, $userStore), ...x }
		})
	}

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key
		if (key === 'Enter') {
			event.preventDefault()
			addGroup()
		}
	}
	async function addGroup() {
		await GroupService.createGroup({
			workspace: $workspaceStore ?? '',
			requestBody: { name: newGroupName }
		})
		loadGroups()
	}

	$: {
		if ($workspaceStore && $userStore) {
			loadGroups()
		}
	}

	let editGroupName: string = ''
</script>

<ShareModal
	bind:this={shareModal}
	kind="group_"
	on:change={() => {
		loadGroups()
	}}
/>

<Drawer bind:this={groupDrawer}>
	<DrawerContent title="Group {editGroupName}" on:close={groupDrawer.closeDrawer}>
		<GroupEditor name={editGroupName} />
	</DrawerContent>
</Drawer>

<CenteredPage>
	<PageHeader
		title="Groups"
		tooltip="Group users together to grant roles and homegenous permissions. Same users can be in many groups at the same time."
	>
		<div class="flex flex-row">
			<input
				class="mr-2"
				on:keyup={handleKeyUp}
				placeholder="New group name"
				bind:value={newGroupName}
			/>
			<Button size="md" startIcon={{ icon: faPlus }} disabled={!newGroupName} on:click={addGroup}>
				New&nbsp;group
			</Button>
		</div>
	</PageHeader>

	<div class="relative mb-20">
		<TableCustom>
			<tr slot="header-row">
				<th>Name</th>
				<th>Summary</th>
				<th />
			</tr>
			<tbody slot="body">
				{#each groups as { name, summary, extra_perms, canWrite }}
					<tr>
						<td>
							<a
								href="#{name}"
								on:click={() => {
									editGroupName = name
									groupDrawer.openDrawer()
								}}
								>{name}
							</a>
							<div>
								<SharedBadge {canWrite} extraPerms={extra_perms} />
							</div>
						</td>
						<td>{summary ?? ''}</td>
						<td>
							<Dropdown
								placement="bottom-end"
								dropdownItems={[
									{
										displayName: 'Manage members',
										icon: faEdit,
										disabled: !canWrite,
										action: () => {
											editGroupName = name
											groupDrawer.openDrawer()
										}
									},
									{
										displayName: 'Manage ACL of the group',
										icon: faShare,
										disabled: !canWrite,
										action: () => {
											shareModal.openDrawer(name)
										}
									},
									{
										displayName: 'Delete',

										icon: faTrash,
										type: 'delete',
										disabled: !canWrite,
										action: async () => {
											await GroupService.deleteGroup({ workspace: $workspaceStore ?? '', name })
											loadGroups()
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
