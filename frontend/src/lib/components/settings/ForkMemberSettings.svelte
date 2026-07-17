<script lang="ts">
	import { Alert, Badge, Button, Section, Skeleton } from '$lib/components/common'
	import SettingsPageHeader from '$lib/components/settings/SettingsPageHeader.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import type { User } from '$lib/gen'
	import { UserService, WorkspaceService } from '$lib/gen'
	import { userStore, userWorkspaces, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Bot, UserMinus, UserPlus } from 'lucide-svelte'
	import { untrack } from 'svelte'

	const parentWorkspaceId = $derived(
		$userWorkspaces.find((w) => w.id === $workspaceStore)?.parent_workspace_id ?? undefined
	)

	let members: User[] | undefined = $state(undefined)
	// Parent members the fork's creator is allowed to bring in: developers and admins there. The
	// backend enforces the same rule; this only keeps the picker from offering rejected candidates.
	let candidates: User[] = $state([])
	let selectedCandidate: string | undefined = $state(undefined)
	let adding = $state(false)
	let removeConfirmedCallback: (() => void) | undefined = $state(undefined)

	const candidateItems = $derived(
		candidates
			.filter((c) => !members?.some((m) => m.email === c.email))
			.map((c) => ({ value: c.email, label: `${c.username} (${c.email})` }))
	)

	async function loadMembers(): Promise<void> {
		members = await UserService.listUsers({ workspace: $workspaceStore! })
	}

	async function loadCandidates(): Promise<void> {
		if (!parentWorkspaceId) {
			candidates = []
			return
		}
		try {
			const parentMembers = await UserService.listUsers({ workspace: parentWorkspaceId })
			candidates = parentMembers.filter((u) => !u.operator && !u.disabled)
		} catch (e) {
			console.warn('Failed to list the parent workspace members:', e)
			candidates = []
		}
	}

	async function addCandidate(): Promise<void> {
		if (!selectedCandidate) return
		adding = true
		try {
			await WorkspaceService.addUser({
				workspace: $workspaceStore!,
				requestBody: { email: selectedCandidate, is_admin: false, operator: false }
			})
			sendUserToast(`Added ${selectedCandidate} as a developer of this fork`)
			selectedCandidate = undefined
			await loadMembers()
		} catch (e) {
			console.error('Failed to add member:', e)
			sendUserToast(`Failed to add member: ${e}`, true)
		} finally {
			adding = false
		}
	}

	async function removeMember(username: string): Promise<void> {
		try {
			await UserService.deleteUser({ workspace: $workspaceStore!, username })
			sendUserToast(`Removed ${username} from this fork`)
			await loadMembers()
		} catch (e) {
			console.error('Failed to remove member:', e)
			sendUserToast(`Failed to remove member: ${e}`, true)
		}
	}

	$effect(() => {
		;[$workspaceStore, parentWorkspaceId]
		untrack(() => {
			if ($workspaceStore && parentWorkspaceId) {
				loadMembers()
				loadCandidates()
			}
		})
	})
</script>

<SettingsPageHeader
	title="Members {members != undefined ? `(${members.length})` : ''}"
	description="Add collaborators to the fork you created."
	link="https://www.windmill.dev/docs/core_concepts/roles_and_permissions"
/>

<Alert type="info" title="You created this fork">
	You are not an admin of this workspace, so you can only add developers or admins of
	<b>{parentWorkspaceId}</b> as developers of this fork, and remove the ones who are not admins here.
	Ask an admin of this workspace for any other membership change.
</Alert>

<div class="pt-6"></div>

<Section>
	{#snippet action()}
		<Popover placement="bottom-end">
			{#snippet trigger()}
				<Button
					variant="accent"
					unifiedSize="md"
					nonCaptureEvent={true}
					startIcon={{ icon: UserPlus }}
				>
					Add collaborator
				</Button>
			{/snippet}
			{#snippet content()}
				<div class="flex flex-col w-[28rem] p-4 gap-2">
					<span class="text-sm leading-6 font-semibold">Add a collaborator</span>
					<span class="text-xs text-secondary">
						They join as a developer of this fork. Only members of
						<b>{parentWorkspaceId}</b> who are developers or admins there can be added.
					</span>
					<Select
						items={candidateItems}
						bind:value={selectedCandidate}
						placeholder={candidateItems.length > 0
							? 'Select a member'
							: 'No one left to add from the parent workspace'}
						clearable
					/>
					<Button
						variant="accent"
						unifiedSize="md"
						disabled={!selectedCandidate || adding}
						onClick={addCandidate}
					>
						Add as developer
					</Button>
				</div>
			{/snippet}
		</Popover>
	{/snippet}

	<DataTable>
		<Head>
			<tr>
				<Cell head first>Email</Cell>
				<Cell head>Username</Cell>
				<Cell head>Role</Cell>
				<Cell head last><span class="sr-only">Actions</span></Cell>
			</tr>
		</Head>
		<tbody>
			{#if members}
				{#each members as member (member.email)}
					{@const { email, username, is_admin, operator } = member}
					<tr class="bg-surface">
						<Cell first>
							{#if member.is_service_account}
								<span class="flex items-center gap-1.5 max-w-[220px]" title={email}>
									<Bot size={16} class="text-blue-500 shrink-0" />
									<span class="truncate">{email}</span>
								</span>
							{:else}
								<span class="block truncate max-w-[220px]" title={email}>{email}</span>
							{/if}
						</Cell>
						<Cell>
							<span class="block truncate max-w-[120px]" title={username}>{username}</span>
						</Cell>
						<Cell>
							<Badge color="blue">
								{is_admin ? 'Admin' : operator ? 'Operator' : 'Developer'}
							</Badge>
						</Cell>
						<Cell last>
							<Button
								unifiedSize="sm"
								variant="subtle"
								destructive
								disabled={is_admin || email === $userStore?.email}
								title={is_admin
									? 'Only an admin of this workspace can remove an admin'
									: email === $userStore?.email
										? 'Leave the workspace from the workspace menu instead'
										: undefined}
								startIcon={{ icon: UserMinus }}
								onClick={() => {
									removeConfirmedCallback = () => removeMember(username)
								}}
							>
								Remove
							</Button>
						</Cell>
					</tr>
				{/each}
			{:else}
				{#each new Array(4) as _, i (i)}
					<tr class="border">
						<td colspan={4}><Skeleton layout={[[4]]} /></td>
					</tr>
				{/each}
			{/if}
		</tbody>
	</DataTable>
</Section>

<ConfirmationModal
	open={Boolean(removeConfirmedCallback)}
	title="Remove member"
	confirmationText="Remove"
	on:canceled={() => {
		removeConfirmedCallback = undefined
	}}
	on:confirmed={() => {
		removeConfirmedCallback?.()
		removeConfirmedCallback = undefined
	}}
>
	<span>
		Are you sure you want to remove this member from the fork? Anything they own here stays behind
		under their user path.
	</span>
</ConfirmationModal>
