<script lang="ts">
	import { userStore, workspaceStore } from '$lib/stores'
	import Modal from './Modal.svelte'
	import { type Group, GroupService, UserService } from '$lib/gen'
	import AutoComplete from 'simple-svelte-autocomplete'
	import PageHeader from './PageHeader.svelte'
	import TableCustom from './TableCustom.svelte'
	import { canWrite } from '$lib/utils'
	import { Button } from './common'

	let name = ''
	let modal: Modal
	let can_write = false

	let group: Group | undefined
	let members: { name: string; isAdmin: boolean }[] = []
	let usernames: string[] = []
	let username: string = ''

	async function loadUsernames(): Promise<void> {
		usernames = await UserService.listUsernames({ workspace: $workspaceStore! })
	}

	$: {
		if (group && $workspaceStore && userStore) {
			members = (group.members ?? []).map((x) => {
				return {
					name: x,
					isAdmin: x in (group?.extra_perms ?? {}) && (group?.extra_perms ?? {})[name]
				}
			})
		}
	}
	export function openModal(newName: string): void {
		name = newName
		loadGroup()
		loadUsernames()
		modal.openModal()
	}

	async function addToGroup() {
		await GroupService.addUserToGroup({
			workspace: $workspaceStore ?? '',
			name,
			requestBody: { username }
		})
		loadGroup()
	}

	async function loadGroup(): Promise<void> {
		group = await GroupService.getGroup({ workspace: $workspaceStore!, name })
		can_write = canWrite(group.name!, group.extra_perms ?? {}, $userStore)
	}
</script>

<Modal bind:this={modal}>
	<div slot="title">group {name}</div>
	<div slot="content">
		<PageHeader title="Summary" />
		<p>{group?.summary ?? 'No summary'}</p>
		<PageHeader title="Members">
			{#if can_write}
				<div>
					<AutoComplete items={usernames} bind:selectedItem={username} />
					<Button
						variant="contained"
						color="blue"
						size="sm"
						btnClasses="!ml-4"
						on:click={addToGroup}
					>
						Add member
					</Button>
				</div>
			{/if}
		</PageHeader>
		<TableCustom>
			<tr slot="header-row">
				<th>user</th>
				<th>admin of group</th>
				<th />
			</tr>
			<tbody slot="body">
				{#each members as { name, isAdmin }}<tr>
						<td>{name}</td>
						<td> {isAdmin ? 'admin' : ''} </td>
						<td>
							{#if can_write}
								<button
									class="ml-2 text-red-500"
									on:click={async () => {
										await GroupService.removeUserToGroup({
											workspace: $workspaceStore ?? '',
											name: group?.name ?? '',
											requestBody: { username: name }
										})
										loadGroup()
									}}>remove</button
								>
							{/if}</td
						>
					</tr>{/each}
			</tbody></TableCustom
		>
	</div>
</Modal>
