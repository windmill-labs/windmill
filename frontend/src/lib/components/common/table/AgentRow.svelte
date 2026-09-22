<script lang="ts">
	import { base } from '$lib/base'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import type MoveDrawer from '$lib/components/MoveDrawer.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import DraftBadge from '$lib/components/DraftBadge.svelte'
	import type ShareModal from '$lib/components/ShareModal.svelte'
	import type DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import InheritedLabels from '$lib/components/InheritedLabels.svelte'
	import { DraftService, ResourceService, type ListableResource } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { isDeployable } from '$lib/utils_deployable'
	import { getDeployUiSettings } from '$lib/components/home/deploy_ui'
	import { createEventDispatcher } from 'svelte'
	import { Boxes, FileUp, FolderInput, Pen, Shield, Trash } from 'lucide-svelte'
	import Badge from '../badge/Badge.svelte'
	import Button from '../button/Button.svelte'
	import Row from './Row.svelte'

	/**
	 * A home-page row for a saved agent. The agent is an `ai_agent` resource; the row links to the
	 * agent editor page rather than the resources drawer, and its menu carries the resource actions
	 * (permissions, move, deploy, delete) since that is what the item is underneath.
	 */
	interface Props {
		agent: ListableResource & { canWrite: boolean }
		marked: string | undefined
		shareModal: ShareModal
		moveDrawer: MoveDrawer
		deploymentDrawer: DeployWorkspaceDrawer
		deleteConfirmedCallback?: (() => void) | undefined
		depth?: number
		menuOpen?: boolean
		showEditButton?: boolean
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
		menuOpen = $bindable(false),
		showEditButton = true,
		keyboardSelected = false
	}: Props = $props()

	const dispatch = createEventDispatcher()

	async function deleteAgent(path: string): Promise<void> {
		try {
			if (agent.draft_only) {
				// Nothing deployed to delete: the row is the caller's own draft, which the draft
				// endpoint discards on a null value.
				await DraftService.updateDraft({
					workspace: $workspaceStore!,
					kind: 'resource',
					path,
					requestBody: { value: null, force: true }
				})
			} else {
				await ResourceService.deleteResource({ workspace: $workspaceStore!, path })
			}
			sendUserToast(`Deleted agent ${path}`)
			dispatch('change')
		} catch (err) {
			sendUserToast(`Could not delete agent ${path}: ${err}`, true)
		}
	}

	let editHref = $derived(`${base}/agents/edit/${agent.path}`)
	// A draft-only agent has nothing deployed for a detail page to show, so it opens the editor.
	let rowHref = $derived(agent.draft_only ? editHref : `${base}/agents/get/${agent.path}`)
</script>

<Row
	href={rowHref}
	kind="agent"
	{keyboardSelected}
	{marked}
	path={agent.path}
	summary={agent.is_draft ? `${agent.description || agent.path}*` : agent.description}
	workspaceId={agent.workspace_id ?? $workspaceStore ?? ''}
	canFavorite={false}
	{depth}
>
	{#snippet badges()}
		<Badge color="violet" baseClass="border border-violet-200 dark:border-violet-800">Agent</Badge>
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
		{#if agent.labels?.length}
			<div class="flex items-center gap-0.5">
				{#each agent.labels.slice(0, 3) as label (label)}
					<Badge color="blue" small class="px-1" title="Label: {label}">{label}</Badge>
				{/each}
				{#if agent.labels.length > 3}
					<Badge
						color="blue"
						small
						class="px-1"
						title={agent.labels
							.slice(3)
							.map((l) => 'Label: ' + l)
							.join('\n')}>+{agent.labels.length - 3}</Badge
					>
				{/if}
			</div>
		{/if}
		<InheritedLabels labels={agent.inherited_labels} />
	{/snippet}

	{#snippet actions()}
		<span class="hidden md:inline-flex gap-x-1">
			{#if !$userStore?.operator && showEditButton && agent.canWrite}
				<div>
					<Button
						variant="subtle"
						wrapperClasses="w-20"
						unifiedSize="md"
						startIcon={{ icon: Pen }}
						href={editHref}
					>
						Edit
					</Button>
				</div>
			{/if}
		</span>
		<Dropdown
			items={async () => {
				const { path } = agent
				const canEdit = agent.canWrite && showEditButton
				return [
					{
						displayName: 'Open in resources',
						icon: Boxes,
						href: `${base}/resources#/resource/${path}`
					},
					{
						displayName: 'Permissions',
						icon: Shield,
						disabled: !canEdit,
						action: () => {
							shareModal.openDrawer?.(path, 'resource')
						}
					},
					{
						displayName: 'Move/Rename',
						icon: FolderInput,
						disabled: !canEdit || Boolean(agent.draft_only),
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
						disabled: !canEdit,
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
