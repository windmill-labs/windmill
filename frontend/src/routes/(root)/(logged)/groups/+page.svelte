<script lang="ts">
	import type { Group } from '$lib/gen'
	import type { InstanceGroupWithWorkspaces } from '$lib/gen'
	import { GroupService } from '$lib/gen'

	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Button, Drawer, DrawerContent, Skeleton } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import GroupEditor from '$lib/components/GroupEditor.svelte'
	import GroupInfo from '$lib/components/GroupInfo.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import { userStore, workspaceStore, userWorkspaces } from '$lib/stores'
	import { canWrite } from '$lib/utils'
	import { Pen, Plus, Trash } from 'lucide-svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import { untrack } from 'svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { Tooltip } from '$lib/components/meltComponents'

	type GroupW = Group & { canWrite: boolean }

	let newGroupName: string = $state('')
	let groups: GroupW[] | undefined = $state(undefined)
	let instanceGroups: InstanceGroupWithWorkspaces[] | undefined = $state(undefined)
	let groupDrawer: Drawer | undefined = $state()

	async function loadGroups(): Promise<void> {
		groups = (await GroupService.listGroups({ workspace: $workspaceStore! })).map((x) => {
			return { canWrite: canWrite(x.name, x.extra_perms ?? {}, $userStore), ...x }
		})
	}

	async function loadInstanceGroups(): Promise<void> {
		try {
			instanceGroups = await GroupService.listInstanceGroupsWithWorkspaces()
		} catch (e) {
			instanceGroups = undefined
		}
	}

	function handleKeyUp(event: KeyboardEvent, close: () => void) {
		const key = event.key
		if (key === 'Enter') {
			event.preventDefault()
			addGroup()
			close()
		}
	}
	async function addGroup() {
		await GroupService.createGroup({
			workspace: $workspaceStore ?? '',
			requestBody: { name: newGroupName }
		})
		loadGroups()
		editGroupName = newGroupName
		groupDrawer?.openDrawer()
	}

	$effect(() => {
		untrack(() => loadInstanceGroups())
		if ($workspaceStore && $userStore) {
			untrack(() => loadGroups())
		}
	})

	let editGroupName: string = $state('')
</script>

<Drawer bind:this={groupDrawer}>
	<DrawerContent title="Group {editGroupName}" on:close={groupDrawer.closeDrawer}>
		<GroupEditor on:update={loadGroups} name={editGroupName} />
	</DrawerContent>
</Drawer>

{#if $userStore?.operator && $workspaceStore && !$userWorkspaces.find((_) => _.id === $workspaceStore)?.operator_settings?.groups}
	<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4 m-4 mt-12" role="alert">
		<p class="font-bold">Unauthorized</p>
		<p>Page not available for operators</p>
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
					<Popover
						floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
						containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
					>
						{#snippet trigger()}
							<Button unifiedSize="md" variant="accent" startIcon={{ icon: Plus }} nonCaptureEvent
								>New&nbsp;group</Button
							>
						{/snippet}
						{#snippet content({ close })}
							<div class="flex-col flex gap-2 p-4">
								<TextInput
									size="md"
									inputProps={{
										placeholder: 'New group name',
										onkeyup: (e) => handleKeyUp(e, close)
									}}
									bind:value={newGroupName}
								/>
								<Button
									unifiedSize="md"
									variant="accent"
									startIcon={{ icon: Plus }}
									disabled={!newGroupName}
									on:click={() => {
										addGroup()
										close()
									}}
								>
									Create
								</Button>
							</div>
						{/snippet}
					</Popover>
				</div>
			</div>
		</PageHeader>

		<div class="relative mb-20 pt-8">
			<DataTable>
				<Head>
					<tr>
						<Cell head first>Name</Cell>
						<Cell head>Members</Cell>
						<Cell head last />
					</tr>
				</Head>
				<tbody class="divide-y">
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
							<Row
								hoverable
								on:click={() => {
									editGroupName = name
									groupDrawer?.openDrawer()
								}}
							>
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
								<Cell>
									<Dropdown
										items={[
											{
												displayName: 'Manage group',
												icon: Pen,
												disabled: !canWrite,
												action: (e) => {
													e?.stopPropagation()
													editGroupName = name
													groupDrawer?.openDrawer()
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

		{#if instanceGroups && instanceGroups.length > 0}
			<div class="flex flex-row gap-1 items-center mb-2">
				<span class="text-emphasis text-sm font-semibold">Instance groups</span>
				<Tooltip documentationLink="https://www.windmill.dev/docs/misc/saml_and_scim#scim">
					{#snippet text()}
						Instance Groups are managed by SCIM and are groups shared by every workspaces
					{/snippet}
				</Tooltip>
			</div>
			<div class="relative mb-20">
				<DataTable>
					<Head>
						<tr>
							<Cell head first>Name</Cell>
							<Cell head>Members</Cell>
							<Cell head last>Workspaces</Cell>
						</tr>
					</Head>
					<tbody class="divide-y">
						{#each instanceGroups ?? [] as { name, emails, workspaces }}
							<Row>
								<Cell first>
									<a
										href="#{name}"
										onclick={() => {
											if (name) {
												editGroupName = name
												groupDrawer?.openDrawer()
											}
										}}
										>{name}
									</a>
								</Cell>
								<Cell>{emails?.length ?? 0} members</Cell>
								<Cell last>
									{#if workspaces && workspaces.length > 0}
										{#each workspaces as workspace, index}
											{#if index > 0}${', '}{/if}<a
												href="/workspace_settings?tab=users&workspace={workspace.workspace_id}"
											>
												{workspace.workspace_id}
											</a>
											({workspace.role})
										{/each}
									{:else}
										<span class="text-emphasis font-semibold text-xs">No workspaces</span>
									{/if}
								</Cell>
							</Row>
						{/each}
					</tbody>
				</DataTable>
			</div>
		{/if}
	</CenteredPage>
{/if}

<style>
</style>
