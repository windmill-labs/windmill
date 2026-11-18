<script lang="ts">
	import { GroupService, type InstanceGroupWithWorkspaces } from '$lib/gen'
	import { createEventDispatcher } from 'svelte'
	import { Button } from './common'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import DataTable from './table/DataTable.svelte'
	import Head from './table/Head.svelte'
	import Row from './table/Row.svelte'
	import Cell from './table/Cell.svelte'
	import { sendUserToast } from '$lib/toast'
	import { Plus, Trash } from 'lucide-svelte'
	import { superadmin } from '$lib/stores'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import Label from './Label.svelte'
	import CollapseLink from './CollapseLink.svelte'
	import IdpGroupCell from './IdpGroupCell.svelte'

	interface Props {
		name: string
	}

	let { name }: Props = $props()

	let email = $state('')
	let instance_group: InstanceGroupWithWorkspaces | undefined = $state()
	let summary = $state('')
	let externalId = $state('')

	const dispatch = createEventDispatcher()

	async function loadInstanceGroup(): Promise<void> {
		instance_group = await GroupService.getInstanceGroup({ name })
		summary = instance_group.summary ?? ''
		externalId = instance_group.external_id ?? ''
	}
	$effect(() => {
		loadInstanceGroup()
	})

	async function update(
		body: { instance_role?: string; external_id?: string; new_summary?: string },
		toast: string
	) {
		await GroupService.updateInstanceGroup({
			name,
			requestBody: {
				new_summary: body.new_summary ?? instance_group?.summary ?? '',
				instance_role: body.instance_role ?? instance_group?.instance_role ?? 'user',
				external_id: body.external_id
			}
		})
		await loadInstanceGroup()
		dispatch('update')
		sendUserToast(toast)
	}

	async function addMember() {
		await GroupService.addUserToInstanceGroup({ name, requestBody: { email } })
		email = ''
		await loadInstanceGroup()
		dispatch('update')
		sendUserToast('User added')
	}

	async function removeMember(member_email: string) {
		await GroupService.removeUserFromInstanceGroup({ name, requestBody: { email: member_email } })
		await loadInstanceGroup()
		dispatch('update')
		sendUserToast('User removed')
	}

	let members = $derived(instance_group?.emails ?? [])
	let linked = $derived(!!instance_group?.external_id)
</script>

{#if instance_group !== undefined}
	<div class="flex flex-col gap-6">
		{#if $superadmin}
			<Label label="Summary" for="igroup-summary">
				<div class="flex gap-2 items-center">
					<TextInput
						bind:value={summary}
						class="grow"
						size="md"
						inputProps={{ id: 'igroup-summary', placeholder: 'Short summary of the group' }}
					/>
					<Button
						variant="default"
						unifiedSize="md"
						disabled={summary === (instance_group.summary ?? '')}
						onclick={() => update({ new_summary: summary }, 'Summary saved')}
					>
						Save
					</Button>
				</div>
			</Label>
		{:else}
			{#if instance_group.summary}
				<Label label="Summary">
					<span class="text-xs text-primary">{instance_group.summary}</span>
				</Label>
			{/if}
			{#if instance_group.instance_role}
				<Label label="Instance role">
					<span class="text-xs text-primary">{instance_group.instance_role}</span>
				</Label>
			{/if}
			{#if linked}
				<Label label="IdP group">
					<IdpGroupCell
						external_id={instance_group.external_id}
						scim_managed={instance_group.scim_managed}
					/>
				</Label>
			{/if}
		{/if}

		{#if $superadmin}
			<Label label="Instance role">
				<span class="text-xs text-secondary">
					Every member gets this role on the instance. Superadmin grants full admin access, devops
					grants read-only admin visibility.
				</span>
				<ToggleButtonGroup
					selected={instance_group.instance_role ?? 'user'}
					on:selected={(e) => update({ instance_role: e.detail }, 'Instance role updated')}
				>
					{#snippet children({ item })}
						<ToggleButton value="user" size="sm" label="User" {item} />
						<ToggleButton
							value="devops"
							size="sm"
							label="Devops"
							tooltip="Devops is a role that grants visibility similar to that of a super admin, but without giving all rights."
							{item}
						/>
						<ToggleButton value="superadmin" size="sm" label="Superadmin" {item} />
					{/snippet}
				</ToggleButtonGroup>
			</Label>

			{#snippet idpGroupField(group: InstanceGroupWithWorkspaces)}
				<Label label="IdP group" for="igroup-external-id">
					<span class="text-xs text-secondary">
						{#if group.scim_managed}
							SCIM provisioned this group and keeps its IdP group in sync.
						{:else}
							The IdP group as the SSO groups claim names it: an id or a full path such as
							/operators/network-team. Each SSO login then adds or removes the user. Leave empty to
							manage members by hand.
						{/if}
					</span>
					<div class="flex gap-2 items-center">
						<TextInput
							bind:value={externalId}
							class="grow"
							size="md"
							inputProps={{
								id: 'igroup-external-id',
								placeholder: '/operators',
								disabled: group.scim_managed
							}}
						/>
						{#if !group.scim_managed}
							<Button
								variant="default"
								unifiedSize="md"
								disabled={externalId.trim() === (group.external_id ?? '')}
								onclick={() => update({ external_id: externalId }, 'IdP group saved')}
							>
								Save
							</Button>
						{/if}
					</div>
				</Label>
			{/snippet}

			{#if linked || instance_group.scim_managed}
				{@render idpGroupField(instance_group)}
			{:else}
				<CollapseLink text="Link to an IdP group">
					{@render idpGroupField(instance_group)}
				</CollapseLink>
			{/if}
		{/if}

		{#if instance_group.workspaces && instance_group.workspaces.length > 0}
			<Label label="Workspaces">
				<DataTable size="sm">
					<Head>
						<tr>
							<Cell head first class="text-secondary">Workspace</Cell>
							<Cell head last class="text-secondary">Role</Cell>
						</tr>
					</Head>
					<tbody class="divide-y">
						{#each instance_group.workspaces as ws (ws.workspace_id)}
							<Row>
								<Cell first>{ws.workspace_name ?? ws.workspace_id}</Cell>
								<Cell last>{ws.role}</Cell>
							</Row>
						{/each}
					</tbody>
				</DataTable>
			</Label>
		{/if}

		<Label label={`Members (${members.length})`}>
			{#if instance_group.scim_managed}
				<span class="text-xs text-secondary">SCIM keeps these members in sync with the IdP.</span>
			{:else if linked && $superadmin}
				<span class="text-xs text-secondary">
					Members follow the IdP group at each SSO login, so someone added here leaves at their next
					login unless the IdP lists them.
				</span>
			{:else if linked}
				<span class="text-xs text-secondary">Members follow the IdP group at each SSO login.</span>
			{/if}
			{#if $superadmin}
				<div class="flex gap-2 items-center">
					<TextInput
						bind:value={email}
						class="grow"
						size="md"
						inputProps={{ type: 'email', placeholder: 'john@acme.com' }}
					/>
					<Button
						variant="default"
						unifiedSize="md"
						startIcon={{ icon: Plus }}
						disabled={!email.trim()}
						onclick={addMember}
					>
						Add member
					</Button>
				</div>
			{/if}
			{#if members.length > 0}
				<DataTable size="sm">
					<Head>
						<tr>
							<Cell head first last={!$superadmin} class="text-secondary">Email</Cell>
							{#if $superadmin}
								<Cell head last actions class="text-secondary w-20">Actions</Cell>
							{/if}
						</tr>
					</Head>
					<tbody class="divide-y">
						{#each members as member_email (member_email)}
							<Row>
								<Cell first last={!$superadmin}>
									<span class="block truncate text-emphasis font-medium">{member_email}</span>
								</Cell>
								{#if $superadmin}
									<Cell last actions>
										<div class="flex items-center justify-end">
											<Button
												variant="subtle"
												destructive
												unifiedSize="sm"
												startIcon={{ icon: Trash }}
												iconOnly
												title="Remove member"
												aria-label="Remove {member_email}"
												onclick={() => removeMember(member_email)}
											/>
										</div>
									</Cell>
								{/if}
							</Row>
						{/each}
					</tbody>
				</DataTable>
			{/if}
		</Label>
	</div>
{:else}
	<div class="flex flex-col">
		{#each new Array(6) as _, i (i)}
			<Skeleton layout={[[2], 0.7]} />
		{/each}
	</div>
{/if}
