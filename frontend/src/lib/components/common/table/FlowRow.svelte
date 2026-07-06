<script lang="ts">
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import type MoveDrawer from '$lib/components/MoveDrawer.svelte'
	import ScheduleEditor from '$lib/components/triggers/schedules/ScheduleEditor.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import DraftBadge from '$lib/components/DraftBadge.svelte'
	import type ShareModal from '$lib/components/ShareModal.svelte'
	import { FlowService, type Flow } from '$lib/gen'
	import { userStore, userWorkspaces, workspaceStore } from '$lib/stores'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import { createEventDispatcher } from 'svelte'
	import Badge from '../badge/Badge.svelte'
	import Button from '../button/Button.svelte'
	import Row from './Row.svelte'
	import { sendUserToast } from '$lib/toast'
	import { copyToClipboard, isOwner } from '$lib/utils'
	import { isDeployable } from '$lib/utils_deployable'

	import type DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import {
		Pen,
		GitFork,
		Trash,
		List,
		FolderOpen,
		ChevronUpSquare,
		Calendar,
		Shield,
		Archive,
		Copy,
		Eye,
		HistoryIcon
	} from 'lucide-svelte'
	import FlowHistory from '$lib/components/flows/FlowHistory.svelte'
	import InheritedLabels from '$lib/components/InheritedLabels.svelte'
	import { getDeployUiSettings } from '$lib/components/home/deploy_ui'
	import { buildForkEditUrl, editInForkAllowed, editInForkLabel } from '$lib/utils/editInFork'
	import { isCloudHosted } from '$lib/cloud'

	interface Props {
		flow: Flow & {
			draft_only?: boolean
			is_draft?: boolean
			draft_path?: string
			draft_users?: { username?: string | null }[]
			canWrite: boolean
		}
		marked: string | undefined
		shareModal: ShareModal
		moveDrawer: MoveDrawer
		deleteConfirmedCallback: (() => void) | undefined
		deploymentDrawer: DeployWorkspaceDrawer
		errorHandlerMuted: boolean
		depth?: number
		menuOpen?: boolean
		showEditButton?: boolean
		keyboardSelected?: boolean
	}

	let {
		flow,
		marked,
		shareModal,
		moveDrawer,
		deleteConfirmedCallback = $bindable(),
		deploymentDrawer,
		errorHandlerMuted,
		depth = 0,
		menuOpen = $bindable(false),
		showEditButton = $bindable(true),
		keyboardSelected = false
	}: Props = $props()

	const dispatch = createEventDispatcher()

	async function archiveFlow(path: string, archived: boolean): Promise<void> {
		try {
			await FlowService.archiveFlowByPath({
				workspace: $workspaceStore!,
				path,
				requestBody: { archived }
			})
			dispatch('change')
			sendUserToast(`Archived flow ${path}`)
		} catch (err) {
			sendUserToast(`Could not archive this flow ${err.body}`, true)
		}
	}

	async function deleteFlow(path: string): Promise<void> {
		try {
			// Draft-only items have no deployed row to delete — the regular
			// route would 404. Route the delete through the syncer so the
			// per-user draft row is removed instead.
			if (flow.draft_only) {
				await UserDraftDbSyncer.save({
					workspace: $workspaceStore!,
					itemKind: 'flow',
					path,
					value: null,
					immediate: true
				})
			} else {
				await FlowService.deleteFlowByPath({ workspace: $workspaceStore!, path })
			}
			dispatch('change')
			sendUserToast(`Deleted flow ${path}`)
		} catch (err) {
			sendUserToast(`Could not delete this flow ${err.body}`, true)
		}
	}
	let scheduleEditor: ScheduleEditor | undefined = $state(undefined)
	let flowHistory: FlowHistory | undefined = $state(undefined)
</script>

{#if menuOpen}
	<ScheduleEditor onUpdate={() => goto('/schedules')} bind:this={scheduleEditor} />
	<FlowHistory bind:this={flowHistory} path={flow.path} />
{/if}

<Row
	aiId={`flow-row-${flow.path}`}
	aiDescription={`Button to access the form to run the flow ${flow.summary ?? flow.path}`}
	href={flow.draft_only
		? `${base}/flows/edit/${flow.path}`
		: `${base}/flows/get/${flow.path}?workspace=${$workspaceStore}`}
	kind="flow"
	workspaceId={flow.workspace_id ?? $workspaceStore ?? ''}
	{marked}
	path={flow.draft_path ?? flow.path}
	summary={flow.is_draft ? `${flow.summary || flow.draft_path || flow.path}*` : flow.summary}
	{errorHandlerMuted}
	canFavorite={!flow.draft_only}
	{depth}
	{keyboardSelected}
>
	{#snippet badges()}
		{#if flow.archived}
			<Badge color="red" baseClass="border">archived</Badge>
		{/if}
		<SharedBadge canWrite={flow.canWrite} extraPerms={flow.extra_perms} />
		<DraftBadge
			is_draft={flow.is_draft}
			draft_only={flow.draft_only}
			draft_users={flow.draft_users}
			currentUsername={$userStore?.username}
			workspace={$workspaceStore ?? undefined}
			itemKind="flow"
			path={flow.path}
			onMigrated={() => dispatch('change')}
		/>
		{#if flow.labels?.length}
			<div class="flex items-center gap-0.5">
				{#each flow.labels.slice(0, 3) as label}
					<Badge color="blue" small class="px-1" title="Label: {label}">{label}</Badge>
				{/each}
				{#if flow.labels.length > 3}
					<Badge
						color="blue"
						small
						class="px-1"
						title={flow.labels
							.slice(3)
							.map((l) => 'Label: ' + l)
							.join('\n')}>+{flow.labels.length - 3}</Badge
					>
				{/if}
			</div>
		{/if}
		<InheritedLabels labels={flow.inherited_labels} />
		<div class="w-8 center-center"></div>
	{/snippet}
	{#snippet actions()}
		<span class="hidden md:inline-flex gap-x-1">
			{#if !$userStore?.operator}
				{#if showEditButton && flow.canWrite && !flow.archived}
					<div>
						<Button
							variant="subtle"
							wrapperClasses="w-20"
							unifiedSize="md"
							startIcon={{ icon: Pen }}
							href="{base}/flows/edit/{flow.path}"
							aiId={`edit-flow-button-${flow.summary?.length > 0 ? flow.summary : flow.path}`}
							aiDescription={`Edits the flow ${flow.summary?.length > 0 ? flow.summary : flow.path}`}
						>
							Edit
						</Button>
					</div>
				{/if}
				{#if !isCloudHosted() && editInForkAllowed($workspaceStore, $userWorkspaces) && (!showEditButton || !flow.canWrite)}
					<div>
						<Button
							variant={!showEditButton ? 'default' : 'subtle'}
							wrapperClasses="w-32"
							unifiedSize="md"
							startIcon={{ icon: GitFork }}
							href={buildForkEditUrl('flow', flow.path)}
						>
							{editInForkLabel($workspaceStore, $userWorkspaces)}
						</Button>
					</div>
				{/if}
			{/if}
		</span>

		<Dropdown
			aiId={`flow-row-dropdown-${flow.summary?.length > 0 ? flow.summary : flow.path}`}
			aiDescription={`Open dropdown for flow ${flow.summary?.length > 0 ? flow.summary : flow.path} options`}
			items={async () => {
				let { draft_only, path, archived } = flow
				let owner = isOwner(path, $userStore, $workspaceStore)
				const canEdit = flow.canWrite && showEditButton
				if (draft_only) {
					return [
						{
							displayName: 'Delete',
							icon: Trash,
							action: (event) => {
								// @ts-ignore
								if (event?.shiftKey) {
									deleteFlow(path)
								} else {
									deleteConfirmedCallback = () => {
										deleteFlow(path)
									}
								}
							},
							type: 'delete',
							// A draft-only row is always the authed user's own draft (the
							// list endpoint only surfaces own/legacy draft-only rows), so
							// discarding it never requires write permission on the path.
							disabled: !showEditButton,
							hide: $userStore?.operator
						}
					]
				}
				return [
					{
						displayName: 'View runs',
						icon: List,
						href: `${base}/runs/${path}`
					},
					{
						displayName: 'Duplicate/Fork',
						icon: GitFork,
						href: `${base}/flows/add?template=${path}`,
						disabled: !showEditButton,
						hide: $userStore?.operator
					},
					{
						displayName: editInForkLabel($workspaceStore, $userWorkspaces),
						icon: GitFork,
						href: buildForkEditUrl('flow', path),
						hide:
							$userStore?.operator ||
							isCloudHosted() ||
							!editInForkAllowed($workspaceStore, $userWorkspaces)
					},
					{
						displayName: 'Audit logs',
						icon: Eye,
						href: `${base}/audit_logs?resource=${path}`,
						hide: $userStore?.operator
					},
					{
						displayName: 'Move/Rename',
						icon: FolderOpen,
						action: () => {
							moveDrawer.openDrawer(path, flow.summary, 'flow')
						},
						disabled: !owner || archived || !canEdit,
						hide: $userStore?.operator
					},
					{
						displayName: 'Copy path',
						icon: Copy,
						action: () => {
							copyToClipboard(path)
						}
					},
					...(isDeployable('flow', path, await getDeployUiSettings())
						? [
								{
									displayName: 'Deploy to staging/prod',
									icon: ChevronUpSquare,
									action: () => {
										deploymentDrawer.openDrawer(path, 'flow')
									},
									disabled: archived,
									hide: $userStore?.operator
								}
							]
						: []),
					{
						displayName: 'Deployments',
						icon: HistoryIcon,
						action: () => {
							flowHistory?.open()
						},
						hide: $userStore?.operator
					},
					{
						displayName: 'Schedule',
						icon: Calendar,
						action: () => {
							scheduleEditor?.openNew(true, path)
						},
						disabled: archived,
						hide: $userStore?.operator
					},
					{
						displayName: 'Permissions',
						icon: Shield,
						action: () => {
							shareModal.openDrawer && shareModal.openDrawer(path, 'flow')
						},
						hide: $userStore?.operator
					},
					{
						displayName: archived ? 'Unarchive' : 'Archive',
						icon: Archive,
						action: () => {
							path && archiveFlow(path, !archived)
						},
						type: 'delete',
						disabled: !owner || !canEdit,
						hide: $userStore?.operator
					},
					{
						displayName: 'Delete',
						icon: Trash,
						action: (event) => {
							// @ts-ignore
							if (event?.shiftKey) {
								deleteFlow(path)
							} else {
								deleteConfirmedCallback = () => {
									deleteFlow(path)
								}
							}
						},
						type: 'delete',
						disabled: !owner || !canEdit,
						hide: $userStore?.operator
					}
				]
			}}
			on:open={() => {
				menuOpen = true
			}}
		/>
	{/snippet}
</Row>
