<script lang="ts">
	import { GroupService, type InstanceGroupWithWorkspaces } from '$lib/gen'
	import { createEventDispatcher } from 'svelte'
	import autosize from '$lib/autosize'
	import { Button } from './common'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import TableCustom from './TableCustom.svelte'
	import { sendUserToast } from '$lib/toast'
	import { Loader2 } from 'lucide-svelte'
	import { superadmin } from '$lib/stores'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'

	interface Props {
		name: string
	}

	let { name }: Props = $props()

	let email = $state('')
	let instance_group: InstanceGroupWithWorkspaces | undefined = $state()
	let members: { member_email: string }[] | undefined = $state(undefined)

	const dispatch = createEventDispatcher()

	async function load() {
		return Promise.all([loadInstanceGroup()])
	}

	async function loadInstanceGroup(): Promise<void> {
		instance_group = await GroupService.getInstanceGroup({ name })
		members = instance_group?.emails
			? instance_group.emails.map((x) => {
					return { member_email: x }
				})
			: []
	}
	$effect(() => {
		load()
	})
</script>

<div class="flex flex-col gap-6">
	<h1>{name}</h1>
	{#if instance_group !== undefined}
		<div class="flex flex-col gap-1">
			<textarea
				rows="2"
				use:autosize
				bind:value={instance_group.summary}
				placeholder="Summary of the group"
			></textarea>
			<div class="flex justify-end">
				<Button
					size="xs"
					on:click={async () => {
						await GroupService.updateInstanceGroup({
							name,
							requestBody: {
								new_summary: instance_group?.summary ?? '',
								instance_role: instance_group?.instance_role ?? 'user'
							}
						})
						dispatch('update')
						sendUserToast('New summary saved')
					}}>Save Summary</Button
				>
			</div>
		</div>

		{#if $superadmin}
			<div class="flex flex-col gap-2">
				<h2>Instance Role</h2>
				<p class="text-secondary text-xs"
					>Assign an instance-level role to all members of this group. Superadmin grants full admin
					access, devops grants read-only admin visibility.</p
				>
				<ToggleButtonGroup
					selected={instance_group.instance_role ?? 'user'}
					on:selected={async (e) => {
						let role = e.detail
						await GroupService.updateInstanceGroup({
							name,
							requestBody: {
								new_summary: instance_group?.summary ?? '',
								instance_role: role
							}
						})
						if (instance_group) {
							instance_group.instance_role =
								role === 'user' ? undefined : (role as 'superadmin' | 'devops')
						}
						dispatch('update')
						sendUserToast('Instance role updated')
					}}
				>
					{#snippet children({ item })}
						<ToggleButton value="user" small label="User" {item} />
						<ToggleButton
							value="devops"
							small
							label="Devops"
							tooltip="Devops is a role that grants visibility similar to that of a super admin, but without giving all rights."
							{item}
						/>
						<ToggleButton value="superadmin" small label="Superadmin" {item} />
					{/snippet}
				</ToggleButtonGroup>
			</div>
		{/if}

		{#if instance_group.workspaces && instance_group.workspaces.length > 0}
			<div class="flex flex-col gap-2">
				<h2>Workspace Membership</h2>
				<TableCustom>
					{#snippet headerRow()}
						<tr>
							<th>Workspace</th>
							<th>Role</th>
						</tr>
					{/snippet}
					{#snippet body()}
						<tbody>
							{#each instance_group?.workspaces ?? [] as ws (ws.workspace_id)}
								<tr>
									<td>{ws.workspace_name ?? ws.workspace_id}</td>
									<td>{ws.role}</td>
								</tr>
							{/each}
						</tbody>
					{/snippet}
				</TableCustom>
			</div>
		{/if}

		<h2
			>Members ({#if members?.length != undefined}{members?.length ?? 0}{:else}<Loader2
					class="animate-spin"
					size={10}
				/>{/if})</h2
		>
		<div class="flex items-start">
			<input type="text" placeholder="john@acme.com" bind:value={email} />
			<Button
				variant="accent"
				size="sm"
				btnClasses="!ml-4"
				on:click={async () => {
					await GroupService.addUserToInstanceGroup({
						name,
						requestBody: { email: email }
					})
					dispatch('update')
					sendUserToast('User added')
					loadInstanceGroup()
				}}
			>
				Add member
			</Button>
		</div>
		{#if members}
			<TableCustom>
				{#snippet headerRow()}
					<tr>
						<th>user</th>
						<th></th>
					</tr>
				{/snippet}
				{#snippet body()}
					<tbody>
						{#each members as { member_email } (member_email)}
							<tr>
								<td>{member_email}</td>
								<td>
									<button
										class="ml-2 text-red-500"
										onclick={async () => {
											await GroupService.removeUserFromInstanceGroup({
												name,
												requestBody: { email: member_email }
											})
											dispatch('update')
											sendUserToast('User removed')
											loadInstanceGroup()
										}}>remove</button
									>
								</td>
							</tr>
						{/each}
					</tbody>
				{/snippet}
			</TableCustom>
		{:else}
			<div class="flex flex-col">
				{#each new Array(6) as _, i (i)}
					<Skeleton layout={[[2], 0.7]} />
				{/each}
			</div>
		{/if}
	{/if}
</div>
