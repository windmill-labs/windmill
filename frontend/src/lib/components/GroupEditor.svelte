<script lang="ts">
	import {
		GranularAclService,
		GroupService,
		UserService,
		type Group,
		type InstanceGroup
	} from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import AutoComplete from 'simple-svelte-autocomplete'
	import { createEventDispatcher } from 'svelte'
	import autosize from 'svelte-autosize'
	import { Button } from './common'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import TableCustom from './TableCustom.svelte'
	import { sendUserToast } from '$lib/toast'
	import { canWrite } from '$lib/utils'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'

	export let name: string
	let can_write = false

	type Role = 'member' | 'manager' | 'admin'
	let group: Group | undefined
	let instance_group: InstanceGroup | undefined
	let members: { member_name: string; role: Role }[] | undefined = undefined
	let usernames: string[] | undefined = []
	let username: string = ''

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
		return Promise.all([loadGroup(), loadInstanceGroup(), loadUsernames()])
	}

	async function addToGroup() {
		await GroupService.addUserToGroup({
			workspace: $workspaceStore ?? '',
			name,
			requestBody: { username }
		})
		loadGroup()
	}

	async function loadInstanceGroup(): Promise<void> {
		try {
			instance_group = await GroupService.getInstanceGroup({ name })
		} catch (e) {
			instance_group = undefined
		}
	}

	async function loadGroup(): Promise<void> {
		try {
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
		} catch (e) {
			can_write = false
			members = []
			group = {
				name
			}
		}
	}

	function getRole(x: string): Role {
		const writer = 'u/' + x in (group?.extra_perms ?? {}) && (group?.extra_perms ?? {})['u/' + x]
		const member = group?.members?.includes(x)

		if (writer && member) {
			return 'admin'
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
			<div class="flex justify-end">
				<Button
					disabled={!can_write}
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
		</div>
	{:else}
		<Skeleton layout={[[4]]} />
	{/if}
	<h2>Members ({members?.length ?? 0})</h2>
	{#if can_write}
		<div class="flex items-start">
			<AutoComplete required noInputStyles items={usernames} bind:selectedItem={username} />
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
											} else if (role == 'admin') {
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
										<ToggleButton
											value="member"
											size="xs"
											label="Member"
											tooltip="A Member of a group can see everything the group can see, write to everything the group can write, and generally act on behalf of the group"
										/>

										<!-- <ToggleButton position="center" value="manager" size="xs"
											>Manager <Tooltip
												>A manager of a group can manage the group, adding and removing users and
												change their roles. Being a manager does not make you a member.</Tooltip
											></ToggleButton
										> -->
										<ToggleButton
											position="right"
											value="admin"
											size="xs"
											label="Admin"
											tooltip="An admin of a group is a member of a group that can also add and remove members to the group, or make them admin."
										/>
									</ToggleButtonGroup>
								</div>
							{:else}
								{role}
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
										await GranularAclService.removeGranularAcls({
											workspace: $workspaceStore ?? '',
											path: name,
											kind: 'group_',
											requestBody: {
												owner: 'u/' + member_name
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

		{#if instance_group?.emails}
			<h2 class="mt-10">Members from the instance group</h2>
			<TableCustom>
				<tr slot="header-row">
					<th>user</th>
				</tr>
				<tbody slot="body">
					{#each instance_group?.emails ?? [] as email}<tr>
							<td>{email}</td>
						</tr>{/each}
				</tbody>
			</TableCustom>
		{/if}

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
			<p class="text-tertiary text-sm">No group is managing this group</p>
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
