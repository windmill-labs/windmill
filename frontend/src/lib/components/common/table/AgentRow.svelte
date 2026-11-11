<script lang="ts">
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import DraftBadge from '$lib/components/DraftBadge.svelte'
	import type ShareModal from '$lib/components/ShareModal.svelte'
	import type DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import RowLabels from './RowLabels.svelte'
	import type { ListableResource } from '$lib/gen'
	import { roleCapabilities, userStore, workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import { Pen } from 'lucide-svelte'
	import Button from '../button/Button.svelte'
	import ChatFlowBadge from '$lib/components/flows/ChatFlowBadge.svelte'
	import { keepsManagedMemory } from '$lib/components/flows/agentFormFields'
	import { agentMenuItems, deleteAgent } from '$lib/components/flows/agentActions'
	import AgentHistory from '$lib/components/flows/AgentHistory.svelte'
	import { cachedDeployUiSettings, warmDeployUiSettings } from '$lib/components/home/deploy_ui'
	import Row from './Row.svelte'
	import { selectMenuItems, type RowSelection } from './rowSelection'

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
		rowSelection?: RowSelection
	}

	let {
		agent,
		marked,
		shareModal,
		deploymentDrawer,
		deleteConfirmedCallback = $bindable(),
		depth = 0,
		menuOpen = $bindable(),
		keyboardSelected = false,
		rowSelection = undefined
	}: Props = $props()

	const dispatch = createEventDispatcher()

	warmDeployUiSettings()

	async function remove(path: string) {
		if (await deleteAgent($workspaceStore!, path)) dispatch('change')
	}

	let editHref = $derived(`/agents/edit/${agent.path}`)
	// A draft-only agent has nothing deployed for its page to run, so it opens in the editor.
	let rowHref = $derived(agent.draft_only ? editHref : `/agents/get/${agent.path}`)
	let history: AgentHistory | undefined = $state(undefined)
</script>

<AgentHistory
	bind:this={history}
	path={agent.path}
	canRestore={agent.canWrite}
	onRestored={() => dispatch('change')}
/>

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
	canFavorite={!agent.draft_only}
	{depth}
	{rowSelection}
>
	{#snippet badges()}
		<SharedBadge canWrite={agent.canWrite} extraPerms={agent.extra_perms} />
		<RowLabels inherited={agent.inherited_labels} />
		<DraftBadge
			is_draft={agent.is_draft}
			draft_only={agent.draft_only}
			currentUsername={$userStore?.username}
			workspace={$workspaceStore ?? undefined}
			itemKind="resource"
			path={agent.path}
			onMigrated={() => dispatch('change')}
		/>
	{/snippet}

	{#snippet actions()}
		{#if agent.canWrite}
			<span class="hidden md:inline-flex">
				<Button variant="subtle" unifiedSize="sm" startIcon={{ icon: Pen }} href={editHref}>
					Edit
				</Button>
			</span>
		{/if}
		<Dropdown
			size="sm"
			items={() => [
				...selectMenuItems(rowSelection),
				...agentMenuItems({
					deployUiSettings: cachedDeployUiSettings(),
					path: agent.path,
					canWrite: agent.canWrite,
					draftOnly: Boolean(agent.draft_only),
					wsSpecific: agent.ws_specific,
					noCode: !$roleCapabilities.code,
					onShare: () => shareModal.openDrawer?.(agent.path, 'resource'),
					onDeploy: () => deploymentDrawer.openDrawer(agent.path, 'resource'),
					onHistory: () => history?.open(),
					onDelete: (event) => {
						const { path } = agent
						if (event?.shiftKey) remove(path)
						else deleteConfirmedCallback = () => remove(path)
					}
				})
			]}
			on:open={() => {
				menuOpen = true
			}}
		/>
	{/snippet}
</Row>
