<script lang="ts">
	import { userStore, workspaceStore } from '$lib/stores'
	import { type Group, GroupService, UserService } from '$lib/gen'
	import AutoComplete from 'simple-svelte-autocomplete'
	import TableCustom from './TableCustom.svelte'
	import { canWrite } from '$lib/utils'
	import { Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'

	let name = ''
	let drawer: Drawer
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
	export function openDrawer(newName: string): void {
		name = newName
		loadGroup()
		loadUsernames()
		drawer.openDrawer()
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

<Drawer bind:this={drawer}>
	<DrawerContent title="Group {name}" on:close={drawer.closeDrawer}>
		<div class="flex flex-col gap-6">
			<h2>Summary</h2>
			<p>{group?.summary ?? 'No summary'}</p>
			<h2>Members</h2>
			{#if can_write}
				<div class="flex items-start">
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
				</tbody>
			</TableCustom>
		</div>
	</DrawerContent>
</Drawer>
