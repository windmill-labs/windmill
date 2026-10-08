<script lang="ts">
	import { base } from '$lib/base'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import DraftBadge from '$lib/components/DraftBadge.svelte'
	import type ShareModal from '$lib/components/ShareModal.svelte'
	import type DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import RowLabels from './RowLabels.svelte'
	import type { ListableResource } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import { Pen } from 'lucide-svelte'
	import Button from '../button/Button.svelte'
	import ChatFlowBadge from '$lib/components/flows/ChatFlowBadge.svelte'
	import { keepsManagedMemory } from '$lib/components/flows/agentFormFields'
	import { agentMenuItems, deleteAgent } from '$lib/components/flows/agentActions'
	import { getDeployUiSettings } from '$lib/components/home/deploy_ui'
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

	async function remove(path: string) {
		if (await deleteAgent($workspaceStore!, path)) dispatch('change')
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
	editedAt={agent.edited_at}
	{depth}
>
	{#snippet draftBadge()}
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
	{#snippet sharedBadge()}
		<SharedBadge canWrite={agent.canWrite} extraPerms={agent.extra_perms} />
	{/snippet}
	{#snippet labelBadges()}
		<RowLabels labels={undefined} inheritedLabels={agent.inherited_labels} />
	{/snippet}

	{#snippet actions()}
		{#if agent.canWrite}
			<span class="hidden md:inline-flex">
				<Button
					variant="accent"
					wrapperClasses="w-16 invisible group-hover/row:visible group-focus-within/row:visible group-data-[row-keyboard-selected=true]/row:visible"
					startIcon={{ icon: Pen }}
					unifiedSize="sm"
					href={editHref}
				>
					Edit
				</Button>
			</span>
		{/if}
		<Dropdown
			size="sm"
			fixedHeight={false}
			items={async () =>
				agentMenuItems({
					deployUiSettings: await getDeployUiSettings(),
					path: agent.path,
					canWrite: agent.canWrite,
					draftOnly: Boolean(agent.draft_only),
					wsSpecific: agent.ws_specific,
					onPermissions: () => shareModal.openDrawer?.(agent.path, 'resource'),
					onDeploy: () => deploymentDrawer.openDrawer(agent.path, 'resource'),
					onDelete: (event) => {
						const { path } = agent
						if (event?.shiftKey) remove(path)
						else deleteConfirmedCallback = () => remove(path)
					}
				})}
			on:open={() => {
				menuOpen = true
			}}
		/>
	{/snippet}
</Row>
