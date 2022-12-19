<script lang="ts">
	import { userStore, workspaceStore } from '$lib/stores'
	import { type Group, GroupService, UserService, GranularAclService } from '$lib/gen'
	import AutoComplete from 'simple-svelte-autocomplete'
	import TableCustom from './TableCustom.svelte'
	import { canWrite, sendUserToast } from '$lib/utils'
	import { Button, ToggleButton, ToggleButtonGroup } from './common'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import Tooltip from './Tooltip.svelte'
	import autosize from 'svelte-autosize'
	import { createEventDispatcher } from 'svelte'

	export let name: string
	let can_write = false

	type Role = 'member' | 'manager' | 'member_manager'
	let group: Group | undefined
	let members: { member_name: string; role: Role }[] | undefined = undefined
	let managing_groups: string[] = []
	let usernames: string[] | undefined = []
	let username: string = ''
	let groups: string[] = []

	const dispatch = createEventDispatcher()

	async function loadUsernames(): Promise<void> {
		usernames = await UserService.listUsernames({ workspace: $workspaceStore! })
	}

	$: {
		if ($workspaceStore && $userStore) {
			load()
		}
	}

	async function load() {
		loadGroups()
		await loadGroup()
		loadUsernames()
	}

	async function loadGroups(): Promise<void> {
		groups = (await GroupService.listGroups({ workspace: $workspaceStore! })).map((x) => x.name)
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
		can_write = canWrite(name!, group.extra_perms ?? {}, $userStore)
		members = Array.from(
			new Set(
				Object.entries(group?.extra_perms ?? {})
					.filter(([k, v]) => k.startsWith('u/') && v)
					.map(([k, _]) => k.split('/')[1])
					.concat(group?.members ?? [])
			)
		).map((x) => {
			return {
				member_name: x,
				role: getRole(x)
			}
		})
		managing_groups = Object.entries(group?.extra_perms ?? {})
			.filter(([k, v]) => k.startsWith('g/') && v)
			.map(([k, v]) => k)
	}

	function getRole(x: string): Role {
		const writer = 'u/' + x in (group?.extra_perms ?? {}) && (group?.extra_perms ?? {})['u/' + x]
		const member = group?.members?.includes(x)

		if (writer && member) {
			return 'member_manager'
		} else if (writer) {
			return 'manager'
		} else {
			return 'member'
		}
	}
</script>

<div class="flex flex-col gap-6">
	<h1>{name}</h1>
	{#if group}
		<div class="flex flex-col gap-1">
			<textarea
				disabled={!can_write}
				rows="2"
				type="text"
				use:autosize
				bind:value={group.summary}
				placeholder="Summary of the group"
			/>
			<Button
				size="xs"
				on:click={async () => {
					await GroupService.updateGroup({
						workspace: $workspaceStore ?? '',
						name,
						requestBody: { summary: group?.summary }
					})
					dispatch('update')
					sendUserToast('New summary saved')
				}}>Save Summary</Button
			>
		</div>
	{:else}
		<Skeleton layout={[[4]]} />
	{/if}
	<h2>Members & Managers</h2>
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
				{#each members as { member_name, role }}<tr>
						<td>{member_name}</td>
						<td>
							{#if can_write}
								<div>
									<ToggleButtonGroup
										selected={role}
										on:selected={async (e) => {
											const role = e.detail
											// const wasInGroup = (group?.members ?? []).includes(group)
											// const inAcl = (
											// 	group?.extra_perms ? Object.keys(group?.extra_perms) : []
											// ).includes(group)
											if (role == 'member') {
												await GroupService.addUserToGroup({
													workspace: $workspaceStore ?? '',
													name,
													requestBody: {
														username: member_name
													}
												})
												await GranularAclService.removeGranularAcls({
													workspace: $workspaceStore ?? '',
													path: name,
													kind: 'group_',
													requestBody: {
														owner: 'u/' + member_name
													}
												})
											} else if (role == 'manager') {
												await GroupService.removeUserToGroup({
													workspace: $workspaceStore ?? '',
													name,
													requestBody: {
														username: member_name
													}
												})
												await GranularAclService.addGranularAcls({
													workspace: $workspaceStore ?? '',
													path: name,
													kind: 'group_',
													requestBody: {
														owner: 'u/' + member_name,
														write: true
													}
												})
											} else if (role == 'member_manager') {
												await GroupService.addUserToGroup({
													workspace: $workspaceStore ?? '',
													name,
													requestBody: {
														username: member_name
													}
												})
												await GranularAclService.addGranularAcls({
													workspace: $workspaceStore ?? '',
													path: name,
													kind: 'group_',
													requestBody: {
														owner: 'u/' + member_name,
														write: true
													}
												})
											}
											loadGroup()
										}}
									>
										<ToggleButton position="left" value="member" size="xs"
											>Member <Tooltip
												>A Member of a group can see everything the group can see, write to
												everything the group can write, and generally act on behalf of the group</Tooltip
											></ToggleButton
										>
										<!-- <ToggleButton position="center" value="manager" size="xs"
											>Manager <Tooltip
												>A manager of a group can manage the group, adding and removing users and
												change their roles. Being a manager does not make you a member.</Tooltip
											></ToggleButton
										> -->
										<ToggleButton position="right" value="member_manager" size="xs"
											>Admin</ToggleButton
										>
									</ToggleButtonGroup>
								</div>
							{/if}</td
						>
						<td>
							{#if can_write}
								<button
									class="ml-2 text-red-500"
									on:click={async () => {
										await GroupService.removeUserToGroup({
											workspace: $workspaceStore ?? '',
											name,
											requestBody: { username: member_name }
										})
										loadGroup()
									}}>remove</button
								>
							{/if}</td
						>
					</tr>{/each}
			</tbody>
		</TableCustom>
		<!-- <h2 class="mt-10"
			>Groups managing this group <Tooltip>Any member of those groups can manage this group</Tooltip
			></h2
		>
		{#if can_write}
			<div class="flex items-start">
				<AutoComplete items={groups} bind:selectedItem={new_managing_group} />
				<Button
					variant="contained"
					color="blue"
					size="sm"
					btnClasses="!ml-4"
					on:click={addToManagingGroup}
				>
					Add group managing this group
				</Button>
			</div>
		{/if}
		{#if managing_groups.length == 0}
			<p class="text-gray-600 text-sm">No group is managing this group</p>
		{:else}
			<TableCustom>
				<tr slot="header-row">
					<th>group</th>
					<th />
				</tr>
				<tbody slot="body">
					{#each managing_groups as managing_group}<tr>
							<td>{managing_group.split('/')[1]}</td>
							<td>
								{#if can_write}
									<button
										class="ml-2 text-red-500"
										on:click={async () => {
											await GranularAclService.removeGranularAcls({
												workspace: $workspaceStore ?? '',
												path: name,
												kind: 'group_',
												requestBody: {
													owner: managing_group
												}
											})
											loadGroup()
										}}>remove</button
									>
								{/if}</td
							>
						</tr>{/each}
				</tbody>
			</TableCustom>
		{/if} -->
	{:else}
		<div class="flex flex-col">
			{#each new Array(6) as _}
				<Skeleton layout={[[2], 0.7]} />
			{/each}
		</div>
	{/if}
</div>
