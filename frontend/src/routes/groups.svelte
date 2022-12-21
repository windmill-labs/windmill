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
	import TableCustom from '$lib/components/TableCustom.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { faEdit, faPlus, faShare, faTrash } from '@fortawesome/free-solid-svg-icons'
	import { Button, Drawer, DrawerContent, Skeleton } from '$lib/components/common'
	import GroupInfo from '$lib/components/GroupInfo.svelte'

	type GroupW = Group & { canWrite: boolean }

	let newGroupName: string = ''
	let groups: GroupW[] | undefined = undefined
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
		editGroupName = newGroupName
		groupDrawer.openDrawer()
	}

	$: {
		if ($workspaceStore && $userStore) {
			loadGroups()
		}
	}

	let editGroupName: string = ''
</script>

<Drawer bind:this={groupDrawer}>
	<DrawerContent title="Group {editGroupName}" on:close={groupDrawer.closeDrawer}>
		<GroupEditor on:update={loadGroups} name={editGroupName} />
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
				<th>Members</th>
				<th />
			</tr>
			<tbody slot="body">
				{#if groups === undefined}
					<tr>
						<td colspan="4">
							<Skeleton layout={[[2]]} />
						</td>
					</tr>
				{:else}
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
							<td><GroupInfo {name} /></td>
							<td>
								<Dropdown
									placement="bottom-end"
									dropdownItems={[
										{
											displayName: 'Manage group',
											icon: faEdit,
											disabled: !canWrite,
											action: () => {
												editGroupName = name
												groupDrawer.openDrawer()
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
				{/if}
			</tbody>
		</TableCustom>
	</div>
</CenteredPage>

<style>
</style>
