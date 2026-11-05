<script lang="ts">
	import type { Group } from '$lib/gen'
	import { GroupService } from '$lib/gen'

	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Button, Skeleton } from '$lib/components/common'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import GroupEditorDrawer from '$lib/components/GroupEditorDrawer.svelte'
	import AccessDrawer from '$lib/components/AccessDrawer.svelte'
	import InstanceGroupsTable from '$lib/components/instanceGroups/InstanceGroupsTable.svelte'
	import NewInstanceGroupPopover from '$lib/components/instanceGroups/NewInstanceGroupPopover.svelte'
	import GroupInfo from '$lib/components/GroupInfo.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import { userStore, workspaceStore, roleCapabilities, superadmin } from '$lib/stores'
	import { useEditRights } from '$lib/operatorWriteRights'
	import { latestRequest } from '$lib/utils/latestRequest'
	import { Pen, Plus, ShieldCheck, Trash } from 'lucide-svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import { untrack } from 'svelte'
	import { Tooltip } from '$lib/components/meltComponents'
	import { DEMO_RESTRICTION_HINT, isDemoWorkspaceRestricted } from '$lib/cloud'

	const editRights = useEditRights()

	let restricted = $derived(
		isDemoWorkspaceRestricted($workspaceStore, $userStore?.is_admin, $userStore?.is_super_admin)
	)

	let groups: Group[] | undefined = $state(undefined)
	let instanceGroupsTable: InstanceGroupsTable | undefined = $state()
	let groupEditorDrawer: GroupEditorDrawer | undefined = $state()
	let accessDrawer: AccessDrawer | undefined = $state()

	const groupLoads = latestRequest()
	async function loadGroups(): Promise<void> {
		const isLatest = groupLoads.start()
		const result = await GroupService.listGroups({ workspace: $workspaceStore! })
		if (!isLatest()) return
		groups = result
	}

	$effect(() => {
		if ($workspaceStore) {
			untrack(() => loadGroups())
		}
	})
</script>

<GroupEditorDrawer bind:this={groupEditorDrawer} onSaved={loadGroups} />
<AccessDrawer bind:this={accessDrawer} />

{#if $workspaceStore && !$roleCapabilities.visibility.groups}
	<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4 m-4 mt-12" role="alert">
		<p class="font-bold">Unauthorized</p>
		<p>Page not available for your role</p>
	</div>
{:else}
	<CenteredPage>
		<PageHeader
			title="Groups"
			tooltip="Group users together to grant roles and homegenous permissions. Same users can be in many groups at the same time."
			documentationLink="https://www.windmill.dev/docs/core_concepts/groups_and_folders"
		>
			<div class="flex flex-row">
				<div>
					{#if restricted}
						<Button
							unifiedSize="md"
							variant="accent"
							startIcon={{ icon: Plus }}
							disabled
							title={DEMO_RESTRICTION_HINT}
						>
							New&nbsp;group
						</Button>
					{:else}
						<Button
							unifiedSize="md"
							variant="accent"
							startIcon={{ icon: Plus }}
							on:click={() => groupEditorDrawer?.initNew()}
						>
							New&nbsp;group
						</Button>
					{/if}
				</div>
			</div>
		</PageHeader>

		<div class="relative mb-20 pt-8">
			<DataTable>
				<Head>
					<tr>
						<Cell head first>Name</Cell>
						<Cell head>Members</Cell>
						<Cell head last actions>Actions</Cell>
					</tr>
				</Head>
				<tbody class="divide-y">
					{#if groups === undefined}
						{#each new Array(4) as _}
							<tr>
								<Cell first>
									<Skeleton layout={[[{ h: 0.75, w: 30 }], 0.25, [{ h: 0.75, w: 60 }], 0.25]} />
								</Cell>
								<Cell>
									<Skeleton layout={[[{ h: 0.75, w: 40 }]]} />
								</Cell>
								<Cell last actions />
							</tr>
						{/each}
					{:else}
						{#each groups as { name, summary, extra_perms } (name)}
							{@const canWrite = editRights.canEditItem('group', name, extra_perms)}
							<Row hoverable on:click={() => groupEditorDrawer?.initEdit(name)}>
								<Cell first>
									<div class="flex flex-row gap-2 justify-between">
										<div>
											<span class="text-emphasis text-xs font-semibold">{name}</span>
											{#if summary}
												<br />
												<span class="text-2xs font-normal text-secondary">{summary}</span>
											{/if}
										</div>
										<SharedBadge {canWrite} extraPerms={extra_perms} />
									</div>
								</Cell>
								<Cell>
									<GroupInfo {name} />
								</Cell>
								<Cell last actions shouldStopPropagation>
									<Dropdown
										items={[
											{
												displayName: 'Access',
												icon: ShieldCheck,
												hide: !(
													$userStore?.is_admin ||
													$userStore?.is_super_admin ||
													$userStore?.groups.includes(name)
												),
												action: () => accessDrawer?.open(`g/${name}`, 'group')
											},
											{
												displayName: 'Manage group',
												icon: Pen,
												disabled: !canWrite,
												action: (e) => {
													e?.stopPropagation()
													groupEditorDrawer?.initEdit(name)
												}
											},
											{
												displayName: 'Delete',

												icon: Trash,
												type: 'delete',
												disabled: !canWrite,
												action: async () => {
													await GroupService.deleteGroup({ workspace: $workspaceStore ?? '', name })
													loadGroups()
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

		<InstanceGroupsTable bind:this={instanceGroupsTable} hideIfEmpty={!$superadmin}>
			{#snippet header()}
				<div class="flex flex-row gap-1 items-center justify-between mb-2">
					<div class="flex flex-row gap-1 items-center">
						<span class="text-emphasis text-sm font-semibold">Instance groups</span>
						<Tooltip documentationLink="https://www.windmill.dev/docs/misc/saml_and_scim#scim">
							{#snippet text()}
								Shared by every workspace. Members come from SCIM, from a linked IdP group at SSO
								login, or are added by hand. Workspace admins choose which ones their workspace
								adds, with which role, in Settings → Users.
							{/snippet}
						</Tooltip>
					</div>
					{#if $superadmin}
						<NewInstanceGroupPopover
							variant="default"
							unifiedSize="sm"
							onCreated={(name) => instanceGroupsTable?.created(name)}
						/>
					{/if}
				</div>
			{/snippet}
		</InstanceGroupsTable>
		<div class="mb-20"></div>
	</CenteredPage>
{/if}

<style>
</style>
