<script lang="ts">
	import { userStore, workspaceStore } from '$lib/stores'
	import { type Group, GroupService, UserService, GranularAclService } from '$lib/gen'
	import AutoComplete from 'simple-svelte-autocomplete'
	import TableCustom from './TableCustom.svelte'
	import { canWrite } from '$lib/utils'
	import { Button, ToggleButton, ToggleButtonGroup } from './common'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import Tooltip from './Tooltip.svelte'

	export let name: string
	let can_write = false

	type Role = 'viewer' | 'member' | 'manager' | 'member_manager'
	let group: Group | undefined
	let members: { name: string; role: Role }[] | undefined = undefined
	let usernames: string[] | undefined = []
	let username: string = ''

	async function loadUsernames(): Promise<void> {
		usernames = await UserService.listUsernames({ workspace: $workspaceStore! })
	}

	$: {
		if ($workspaceStore && $userStore) {
			load()
		}
	}

	async function load() {
		await loadGroup()
		await loadUsernames()
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
		members = Object.keys(group?.extra_perms ?? {})
			.concat(group?.members ?? [])
			.map((x) => {
				return {
					name: x,
					role: getRole(x)
				}
			})
	}

	function getRole(x: string): Role {
		const writer = x in (group?.extra_perms ?? {}) && (group?.extra_perms ?? {})[name]
		const member = group?.members?.includes(x)
		if (writer && member) {
			return 'member_manager'
		} else if (writer) {
			return 'manager'
		} else if (member) {
			return 'member'
		}
		return 'viewer'
	}
</script>

<div class="flex flex-col gap-6">
	<h2>Summary</h2>
	{#if group}
		<p>{group?.summary ?? 'No summary'}</p>
	{:else}
		<Skeleton layout={[[4]]} />
	{/if}
	<h2>Members</h2>
	{#if can_write}
		<div class="flex items-start">
			<AutoComplete items={usernames} bind:selectedItem={username} />
			<Button variant="contained" color="blue" size="sm" btnClasses="!ml-4" on:click={addToGroup}>
				Add member
			</Button>
		</div>
	{/if}
	{#if members}
		<TableCustom>
			<tr slot="header-row">
				<th>user</th>
				<th />
				<th />
			</tr>
			<tbody slot="body">
				{#each members as { name, role }}<tr>
						<td>{name}</td>
						<td>
							<ToggleButtonGroup
								selected={role}
								on:selected={async (e) => {
									const group = e.detail
									// const wasInGroup = (group?.members ?? []).includes(group)
									// const inAcl = (
									// 	group?.extra_perms ? Object.keys(group?.extra_perms) : []
									// ).includes(group)
									if (group == 'member') {
										GroupService.addUserToGroup(group)

										GranularAclService.removeGranularAcls({
											workspace: $workspaceStore ?? '',
											path: name,
											kind: 'group_',
											requestBody: $userStore.
										})
									}
									loadGroup()
								}}
							>
								<ToggleButton position="left" value="member" size="xs"
									>Viewer <Tooltip
										>A viewer can see who are the members and managers of a group</Tooltip
									></ToggleButton
								>
								<ToggleButton position="left" value="member" size="xs"
									>Member <Tooltip
										>A Member of a group can see everything the group can see, write to everything
										the group can write, and generally act on behalf of the group</Tooltip
									></ToggleButton
								>
								<ToggleButton position="center" value="manager" size="xs"
									>Manager <Tooltip
										>A manager of a group can manage the group, adding and removing users and change
										their roles. He is not necessarily a member of the group.</Tooltip
									></ToggleButton
								>
								<ToggleButton position="right" value="member_manager" size="xs"
									>Member & Manager</ToggleButton
								>
							</ToggleButtonGroup></td
						>
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
	{:else}
		<div class="flex flex-col">
			{#each new Array(6) as _}
				<Skeleton layout={[[2], 0.7]} />
			{/each}
		</div>
	{/if}
</div>
