<script lang="ts">
	import { base } from '$lib/base'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import DraftBadge from '$lib/components/DraftBadge.svelte'
	import type ShareModal from '$lib/components/ShareModal.svelte'
	import type DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import InheritedLabels from '$lib/components/InheritedLabels.svelte'
	import { ResourceService, type ListableResource } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { isDeployable } from '$lib/utils_deployable'
	import { getDeployUiSettings } from '$lib/components/home/deploy_ui'
	import { createEventDispatcher } from 'svelte'
	import { FileUp, Pen, Shield, Trash } from 'lucide-svelte'
	import Button from '../button/Button.svelte'
	import ChatFlowBadge from '$lib/components/flows/ChatFlowBadge.svelte'
	import { keepsManagedMemory } from '$lib/components/flows/agentFormFields'
	import Row from './Row.svelte'

	/**
	 * A home-page row for a saved agent. The agent is an `ai_agent` resource, so the row opens the
	 * agent editor the resources page hosts, and its menu carries the resource actions.
	 */
	interface Props {
		agent: ListableResource & { canWrite: boolean }
		marked: string | undefined
		shareModal: ShareModal
		deploymentDrawer: DeployWorkspaceDrawer
		deleteConfirmedCallback?: (() => void) | undefined
		depth?: number
		menuOpen: boolean
		keyboardSelected?: boolean
	}

	let {
		agent,
		marked,
		shareModal,
		deploymentDrawer,
		deleteConfirmedCallback = $bindable(),
		depth = 0,
		menuOpen = $bindable(),
		keyboardSelected = false
	}: Props = $props()

	const dispatch = createEventDispatcher()

	async function deleteAgent(path: string): Promise<void> {
		try {
			await ResourceService.deleteResource({ workspace: $workspaceStore!, path })
			sendUserToast(`Deleted agent ${path}`)
			dispatch('change')
		} catch (err) {
			sendUserToast(`Could not delete agent ${path}: ${err}`, true)
		}
	}

	let editHref = $derived(`${base}/agents/edit/${agent.path}`)
	// A draft-only agent has nothing deployed for its page to run, so it opens in the editor.
	let rowHref = $derived(agent.draft_only ? editHref : `${base}/agents/get/${agent.path}`)
</script>

{#snippet chatBadge()}
	<ChatFlowBadge title="Managed memory on: this agent opens as a conversation" />
{/snippet}

<Row
	href={rowHref}
	titleBadge={keepsManagedMemory(agent.agent_memory) ? chatBadge : undefined}
	kind="agent"
	{keyboardSelected}
	{marked}
	path={agent.draft_path ?? agent.path}
	summary={agent.description}
	workspaceId={agent.workspace_id ?? $workspaceStore ?? ''}
	canFavorite={false}
	{depth}
>
	{#snippet badges()}
		<SharedBadge canWrite={agent.canWrite} extraPerms={agent.extra_perms} />
		<DraftBadge
			is_draft={agent.is_draft}
			draft_only={agent.draft_only}
			currentUsername={$userStore?.username}
			workspace={$workspaceStore ?? undefined}
			itemKind="resource"
			path={agent.path}
			onMigrated={() => dispatch('change')}
		/>
		<InheritedLabels labels={agent.inherited_labels} />
	{/snippet}

	{#snippet actions()}
		{#if agent.canWrite}
			<span class="hidden md:inline-flex">
				<Button
					variant="subtle"
					wrapperClasses="w-20"
					unifiedSize="md"
					startIcon={{ icon: Pen }}
					href={editHref}
				>
					Edit
				</Button>
			</span>
		{/if}
		<Dropdown
			items={async () => {
				const { path } = agent
				return [
					{
						displayName: 'Permissions',
						icon: Shield,
						// A draft-only agent has no resource yet to hold permissions.
						disabled: !agent.canWrite || Boolean(agent.draft_only),
						action: () => {
							shareModal.openDrawer?.(path, 'resource')
						}
					},
					...(!agent.ws_specific &&
					!agent.draft_only &&
					isDeployable('resource', path, await getDeployUiSettings())
						? [
								{
									displayName: 'Deploy to prod/staging',
									icon: FileUp,
									action: () => {
										deploymentDrawer.openDrawer(path, 'resource')
									}
								}
							]
						: []),
					{
						displayName: 'Delete',
						icon: Trash,
						type: 'delete' as const,
						disabled: !agent.canWrite,
						action: (event) => {
							if (event?.shiftKey) {
								deleteAgent(path)
							} else {
								deleteConfirmedCallback = () => {
									deleteAgent(path)
								}
							}
						}
					}
				]
			}}
			on:open={() => {
				menuOpen = true
			}}
		/>
	{/snippet}
</Row>
