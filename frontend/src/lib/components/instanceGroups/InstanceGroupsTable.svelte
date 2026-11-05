<script lang="ts">
	import { GroupService, type InstanceGroupWithWorkspaces } from '$lib/gen'
	import { Drawer, DrawerContent, EmptyState, Skeleton } from '$lib/components/common'
	import InstanceGroupEditor from '$lib/components/InstanceGroupEditor.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from '$lib/components/common/confirmationModal/asyncConfirmationModal.svelte'
	import IdpGroupCell from '$lib/components/IdpGroupCell.svelte'
	import { superadmin } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Pen, Trash, Users } from 'lucide-svelte'
	import { onMount, type Snippet } from 'svelte'
	import { page } from '$app/state'
	import { isNewTabClick } from '$lib/navigation'

	interface Props {
		/** Renders nothing, header included, when there are no groups. */
		hideIfEmpty?: boolean
		/** Above the table, once the list has loaded. */
		header?: Snippet
	}

	let { hideIfEmpty = false, header }: Props = $props()

	let groups: InstanceGroupWithWorkspaces[] | undefined = $state(undefined)
	let drawer: Drawer | undefined = $state()
	let openName = $state('')
	let anyLinked = $derived.by(() => (groups ?? []).some((g) => !!g.external_id))
	let hidden = $derived.by(() => hideIfEmpty && groups?.length === 0)

	const confirmationModal = createAsyncConfirmationModal()

	export async function reload(): Promise<void> {
		try {
			groups = await GroupService.listInstanceGroupsWithWorkspaces()
		} catch (e) {
			groups = []
		}
	}

	export function open(name: string) {
		handledName = name
		openName = name
		drawer?.openDrawer()
	}

	// Deep link: #<name> opens that group's drawer. `open` pre-sets handledName, so a row
	// click that also sets the hash doesn't reopen it. Names are compared decoded since the
	// browser percent-encodes the hash.
	let handledName = ''
	$effect(() => {
		const hash = page.url.hash
		if (hash.length <= 1) {
			handledName = ''
			return
		}
		let name: string
		try {
			name = decodeURIComponent(hash.slice(1))
		} catch {
			return
		}
		if (name === handledName || !groups || !drawer) return
		if (groups.some((g) => g.name === name)) open(name)
	})

	export async function created(name: string) {
		await reload()
		open(name)
	}

	async function deleteGroup(name: string) {
		const confirmed = await confirmationModal.ask({
			title: `Delete instance group ${name}?`,
			confirmationText: 'Delete',
			children:
				'Its members leave every workspace they joined through it, and lose any instance role it gave them.'
		})
		if (!confirmed) return
		await GroupService.deleteInstanceGroup({ name })
		sendUserToast(`Instance group ${name} deleted`)
		reload()
	}

	onMount(() => {
		reload()
	})
</script>

<Drawer bind:this={drawer}>
	<DrawerContent title="Instance group {openName}" on:close={drawer?.closeDrawer}>
		<InstanceGroupEditor on:update={reload} name={openName} />
	</DrawerContent>
</Drawer>

<ConfirmationModal {...confirmationModal.props} />

{#if !hidden}
	{@render header?.()}
	{#if groups === undefined}
		<div class="flex flex-col">
			{#each new Array(4) as _, i (i)}
				<Skeleton layout={[[2]]} />
			{/each}
		</div>
	{:else if groups.length === 0}
		<EmptyState
			icon={Users}
			title="No instance groups yet"
			description="Instance groups are shared by every workspace. Create one here, link it to an IdP group, or let SCIM provision them."
		/>
	{:else}
		<DataTable>
			<Head>
				<tr>
					<Cell head first>Name</Cell>
					<Cell head>Members</Cell>
					<Cell head>Instance role</Cell>
					{#if anyLinked}<Cell head>IdP group</Cell>{/if}
					<Cell head last={!$superadmin}>Workspaces</Cell>
					{#if $superadmin}<Cell head last actions>Actions</Cell>{/if}
				</tr>
			</Head>
			<tbody class="divide-y">
				{#each groups as group (group.name)}
					{@const count = group.emails?.length ?? 0}
					<Row hoverable on:click={() => open(group.name)}>
						<Cell first>
							<div class="flex flex-col min-w-0">
								<!-- The row opens the drawer on any click; the name is the link a new-tab click
								     follows, so it keeps that click from the row. -->
								<a
									href="#{encodeURIComponent(group.name)}"
									class="text-emphasis text-xs font-semibold"
									onclick={(e) => {
										e.stopPropagation()
										if (isNewTabClick(e)) return
										open(group.name)
									}}>{group.name}</a
								>
								{#if group.summary}
									<span class="text-2xs font-normal text-secondary truncate" title={group.summary}>
										{group.summary}
									</span>
								{/if}
							</div>
						</Cell>
						<Cell>{count} {count === 1 ? 'member' : 'members'}</Cell>
						<Cell>{group.instance_role ?? '-'}</Cell>
						{#if anyLinked}
							<Cell>
								<IdpGroupCell external_id={group.external_id} scim_managed={group.scim_managed} />
							</Cell>
						{/if}
						<Cell last={!$superadmin}>
							{#if group.workspaces && group.workspaces.length > 0}
								{#each group.workspaces as workspace, index (workspace.workspace_id)}
									{#if index > 0},
									{/if}<a
										href="/workspace_settings?tab=users&workspace={workspace.workspace_id}"
										onclick={(e) => e.stopPropagation()}>{workspace.workspace_id}</a
									>
									({workspace.role})
								{/each}
							{:else}
								<span class="text-secondary">-</span>
							{/if}
						</Cell>
						{#if $superadmin}
							<Cell last actions shouldStopPropagation>
								<Dropdown
									items={[
										{ displayName: 'Manage group', icon: Pen, action: () => open(group.name) },
										{
											displayName: 'Delete',
											icon: Trash,
											type: 'delete',
											action: () => deleteGroup(group.name)
										}
									]}
								/>
							</Cell>
						{/if}
					</Row>
				{/each}
			</tbody>
		</DataTable>
	{/if}
{/if}
