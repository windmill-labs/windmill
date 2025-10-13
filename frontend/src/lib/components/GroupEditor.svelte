<script lang="ts">
	import {
		GranularAclService,
		GroupService,
		UserService,
		type Group,
		type InstanceGroup
	} from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { createEventDispatcher, untrack } from 'svelte'
	import { Button } from './common'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import TableCustom from './TableCustom.svelte'
	import { sendUserToast } from '$lib/toast'
	import { canWrite } from '$lib/utils'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import Section from './Section.svelte'
	import Label from './Label.svelte'
	import Select from './select/Select.svelte'
	import { safeSelectItems } from './select/utils.svelte'

	interface Props {
		name: string
	}

	let { name }: Props = $props()
	let can_write = $state(false)

	type Role = 'member' | 'manager' | 'admin'
	let group: Group | undefined
	let instance_group: InstanceGroup | undefined = $state()
	let members: { member_name: string; role: Role }[] | undefined = $state(undefined)
	let usernames: string[] | undefined = $state([])
	let username: string = $state('')
	let summary = $state('')

	const dispatch = createEventDispatcher()

	async function loadUsernames(): Promise<void> {
		usernames = await UserService.listUsernames({ workspace: $workspaceStore! })
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
			summary = group.summary ?? ''
		} catch (e) {
			can_write = false
			members = []
			summary = ''
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
	$effect.pre(() => {
		if ($workspaceStore && $userStore) {
			untrack(() => {
				load()
			})
		}
	})
</script>

<Section label="Metadata" class="mb-4">
	<Label label="Summary">
		<div class="flex flex-row gap-2">
			<input placeholder="Short summary to be displayed when listed" bind:value={summary} />
			<Button
				size="sm"
				on:click={async () => {
					await GroupService.updateGroup({
						workspace: $workspaceStore ?? '',
						name,
						requestBody: { summary }
					})
					dispatch('update')
					sendUserToast('Group summary updated')
					loadGroup()
				}}>Save</Button
			>
		</div>
	</Label>
</Section>

<Section label={`Members (${members?.length ?? 0})`}>
	{#if can_write}
		<div class="flex items-start">
			<Select items={safeSelectItems(usernames)} bind:value={username} />
			<Button variant="contained" color="blue" size="sm" btnClasses="!ml-4" on:click={addToGroup}>
				Add member
			</Button>
		</div>
	{/if}
	{#if members}
		<TableCustom>
			<!-- @migration-task: migrate this slot by hand, `header-row` is an invalid identifier -->
			<tr slot="header-row">
				<th>user</th>
				<th></th>
				<th></th>
			</tr>
			{#snippet body()}
				<tbody>
					{#each members ?? [] as { member_name, role }}<tr>
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
											{#snippet children({ item })}
												<ToggleButton
													value="member"
													small
													label="Member"
													tooltip="A Member of a group can see everything the group can see, write to everything the group can write, and generally act on behalf of the group"
													{item}
												/>
												<ToggleButton
													value="admin"
													small
													label="Admin"
													tooltip="An admin of a group is a member of a group that can also add and remove members to the group, or make them admin."
													{item}
												/>
												{#if role === 'manager'}
													<ToggleButton
														value="manager"
														small
														label="Manager"
														tooltip="A manager of a group can manage the group, adding and removing users and
													change their roles. Being a manager does not make you a member"
														{item}
													/>
												{/if}
											{/snippet}
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
										onclick={async () => {
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
			{/snippet}
		</TableCustom>

		{#if instance_group?.emails}
			<h2 class="mt-10">Members from the instance group</h2>
			<TableCustom>
				<!-- @migration-task: migrate this slot by hand, `header-row` is an invalid identifier -->
				<tr slot="header-row">
					<th>user</th>
				</tr>
				{#snippet body()}
					<tbody>
						{#each instance_group?.emails ?? [] as email}<tr>
								<td>{email}</td>
							</tr>{/each}
					</tbody>
				{/snippet}
			</TableCustom>
		{/if}
	{:else}
		<div class="flex flex-col">
			{#each new Array(6) as _}
				<Skeleton layout={[[2], 0.7]} />
			{/each}
		</div>
	{/if}
</Section>
