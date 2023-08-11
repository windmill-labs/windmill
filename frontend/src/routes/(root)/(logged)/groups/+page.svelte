<script lang="ts">
	import type { Group } from '$lib/gen'
	import type { InstanceGroup } from '$lib/gen'
	import { GroupService } from '$lib/gen'

	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Button, Drawer, DrawerContent, Skeleton } from '$lib/components/common'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import GroupEditor from '$lib/components/GroupEditor.svelte'
	import GroupInfo from '$lib/components/GroupInfo.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { faEdit, faPlus, faTrash } from '@fortawesome/free-solid-svg-icons'
	import { canWrite } from '$lib/utils'

	type GroupW = Group & { canWrite: boolean }

	let newGroupName: string = ''
	let groups: GroupW[] | undefined = undefined
	let instanceGroups: InstanceGroup[] | undefined = undefined
	let groupDrawer: Drawer

	async function loadGroups(): Promise<void> {
		groups = (await GroupService.listGroups({ workspace: $workspaceStore! })).map((x) => {
			return { canWrite: canWrite(x.name, x.extra_perms ?? {}, $userStore), ...x }
		})
	}

	async function loadInstanceGroups(): Promise<void> {
		try {
			instanceGroups = await GroupService.listInstanceGroups()
		} catch (e) {
			instanceGroups = undefined
		}
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
		loadInstanceGroups()
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
		documentationLink="https://www.windmill.dev/docs/core_concepts/groups_and_folders"
	>
		<div class="flex flex-row">
			<input
				class="mr-2"
				on:keyup={handleKeyUp}
				placeholder="New group name"
				bind:value={newGroupName}
			/>
			<div>
				<Button size="md" startIcon={{ icon: faPlus }} disabled={!newGroupName} on:click={addGroup}>
					New&nbsp;group
				</Button>
			</div>
		</div>
	</PageHeader>

	<div class="relative mb-20 pt-8">
		<TableCustom>
			<tr slot="header-row">
				<th class="!px-0" />
				<th>Name</th>
				<th>Summary</th>
				<th>Members</th>
				<th />
			</tr>
			<tbody slot="body">
				{#if groups === undefined}
					{#each new Array(4) as _}
						<tr>
							<td colspan="5">
								<Skeleton layout={[[2]]} />
							</td>
						</tr>
					{/each}
				{:else}
					{#each groups as { name, summary, extra_perms, canWrite } (name)}
						<tr>
							<td class="!px-0 text-center">
								<SharedBadge {canWrite} extraPerms={extra_perms} />
							</td>
							<td>
								<a
									href="#{name}"
									on:click={() => {
										editGroupName = name
										groupDrawer.openDrawer()
									}}
									>{name}
								</a>
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

	{#if instanceGroups && instanceGroups.length > 0}
		<PageHeader
			title="Instance Groups"
			tooltip="Instance Groups are managed by SCIM and are groups shared by every workspaces"
			documentationLink="https://www.windmill.dev/docs/misc/saml_and_scim#scim"
		/>
		<div class="relative mb-20 pt-8">
			<TableCustom>
				<tr slot="header-row">
					<th>Name</th>
					<th>Members</th>
				</tr>
				<tbody slot="body">
					{#each instanceGroups as { name, emails }}
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
							</td>
							<td>{emails?.length ?? 0} members</td>
						</tr>
					{/each}
				</tbody>
			</TableCustom>
		</div>
	{/if}
</CenteredPage>

<style>
</style>
