<script lang="ts">
	import { canWrite, getUser } from '../utils';
	import { GroupService } from '../gen';
	import type { Group } from '../gen';

	import PageHeader from './components/PageHeader.svelte';
	import TableCustom from './components/TableCustom.svelte';
	import Dropdown from './components/Dropdown.svelte';
	import ShareModal from './components/ShareModal.svelte';
	import SharedBadge from './components/SharedBadge.svelte';
	import { faEdit, faPlus, faShare } from '@fortawesome/free-solid-svg-icons';
	import { workspaceStore } from '../stores';
	import CenteredPage from './components/CenteredPage.svelte';
	import Icon from 'svelte-awesome';
	import GroupModal from './components/GroupModal.svelte';

	type GroupW = Group & { canWrite: boolean };

	let newGroupName: string = '';
	let groups: GroupW[] = [];
	let shareModal: ShareModal;
	let groupModal: GroupModal;

	async function loadGroups(): Promise<void> {
		const user = await getUser($workspaceStore!);
		groups = (await GroupService.listGroups({ workspace: $workspaceStore! })).map((x) => {
			return { canWrite: canWrite(x.name, x.extra_perms ?? {}, user), ...x };
		});
	}

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key || event.keyCode;
		if (key === 13 || key === 'Enter') {
			event.preventDefault();
			addGroup();
		}
	}
	async function addGroup() {
		await GroupService.createGroup({
			workspace: $workspaceStore ?? '',
			requestBody: { name: newGroupName }
		});
		loadGroups();
	}

	$: {
		if ($workspaceStore) {
			loadGroups();
		}
	}
</script>

<ShareModal
	bind:this={shareModal}
	kind="group_"
	on:change={() => {
		loadGroups();
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
									groupModal.openModal(name);
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
											groupModal.openModal(name);
										}
									},
									{
										displayName: 'Manage ACL of the group itself',
										icon: faShare,
										disabled: !canWrite,
										action: () => {
											shareModal.openModal(name);
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
