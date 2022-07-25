<script context="module">
	export function load() {
		return {
			stuff: { title: 'Groups' }
		}
	}
</script>

<script lang="ts">
	import { canWrite } from '$lib/utils'
	import { GroupService } from '$lib/gen'
	import type { Group } from '$lib/gen'

	import PageHeader from '$lib/components/PageHeader.svelte'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import { faEdit, faPlus, faShare } from '@fortawesome/free-solid-svg-icons'
	import { userStore, workspaceStore } from '$lib/stores'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import Icon from 'svelte-awesome'
	import GroupModal from '$lib/components/GroupModal.svelte'

	type GroupW = Group & { canWrite: boolean }

	let newGroupName: string = ''
	let groups: GroupW[] = []
	let shareModal: ShareModal
	let groupModal: GroupModal

	async function loadGroups(): Promise<void> {
		groups = (await GroupService.listGroups({ workspace: $workspaceStore! })).map((x) => {
			return { canWrite: canWrite(x.name, x.extra_perms ?? {}, $userStore), ...x }
		})
	}

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key || event.keyCode
		if (key === 13 || key === 'Enter') {
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
</script>

<ShareModal
	bind:this={shareModal}
	kind="group_"
	on:change={() => {
		loadGroups()
	}}
/>

<GroupModal bind:this={groupModal} />

<CenteredPage>
	<PageHeader title="Groups">
		<div class="flex flex-row">
			<input
				class="mr-2"
				on:keyup={handleKeyUp}
				placeholder="new group name"
				bind:value={newGroupName}
			/>
			<button
				class={newGroupName ? 'default-button' : 'default-button-disabled'}
				on:click={addGroup}
				><Icon class="text-white mb-1" data={faPlus} scale={0.9} /> &nbsp; New group</button
			>
		</div>
	</PageHeader>

	<div class="bg-blue-100 border-l-4 border-blue-600 text-blue-700 p-4 m-4" role="alert">
		<p class="font-bold">Groups are a team or enterprise feature - Unlimited during beta</p>
		<p>
			Groups are a team or enterprise feature and the feature might be significantly different after
			beta in the community edition
		</p>
	</div>
	<div class="relative">
		<TableCustom>
			<tr slot="header-row">
				<th>Name</th>
				<th>Summary</th>
				<th />
			</tr>
			<tbody slot="body">
				{#each groups as { name, summary, extra_perms, canWrite }}
					<tr>
						<td
							><a
								href="#{name}"
								on:click={() => {
									groupModal.openModal(name)
								}}>{name}</a
							>
							<div>
								<SharedBadge {canWrite} extraPerms={extra_perms} />
							</div>
						</td>
						<td>{summary ?? ''}</td>
						<td
							><Dropdown
								dropdownItems={[
									{
										displayName: 'Manage members',
										icon: faEdit,
										disabled: !canWrite,
										action: () => {
											groupModal.openModal(name)
										}
									},
									{
										displayName: 'Manage ACL of the group itself',
										icon: faShare,
										disabled: !canWrite,
										action: () => {
											shareModal.openModal(name)
										}
									}
								]}
								relative={false}
							/></td
						>
					</tr>
				{/each}
			</tbody>
		</TableCustom>
	</div>
</CenteredPage>

<style>
</style>
