<script lang="ts">
	import { base } from '$lib/base'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import type MoveDrawer from '$lib/components/MoveDrawer.svelte'
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
	import { RESOURCES_PATH } from '$lib/components/sessions/previewPaths'
	import { createEventDispatcher } from 'svelte'
	import { FileUp, FolderInput, Shield, Trash } from 'lucide-svelte'
	import Row from './Row.svelte'

	/**
	 * A home-page row for a saved agent. The agent is an `ai_agent` resource, so the row opens the
	 * agent editor the resources page hosts, and its menu carries the resource actions.
	 */
	interface Props {
		agent: ListableResource & { canWrite: boolean }
		marked: string | undefined
		shareModal: ShareModal
		moveDrawer: MoveDrawer
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
		moveDrawer,
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
</script>

<Row
	href="{base}{RESOURCES_PATH}#/resource/{agent.path}"
	kind="agent"
	{keyboardSelected}
	{marked}
	path={agent.path}
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
		<Dropdown
			items={async () => {
				const { path } = agent
				return [
					{
						displayName: 'Permissions',
						icon: Shield,
						disabled: !agent.canWrite,
						action: () => {
							shareModal.openDrawer?.(path, 'resource')
						}
					},
					{
						displayName: 'Move/Rename',
						icon: FolderInput,
						disabled: !agent.canWrite || Boolean(agent.draft_only),
						action: () => {
							moveDrawer.openDrawer(path, agent.description, 'resource')
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
