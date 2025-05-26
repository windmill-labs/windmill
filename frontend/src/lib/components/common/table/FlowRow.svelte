<script lang="ts">
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import type MoveDrawer from '$lib/components/MoveDrawer.svelte'
	import ScheduleEditor from '$lib/components/triggers/schedules/ScheduleEditor.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import type ShareModal from '$lib/components/ShareModal.svelte'
	import { FlowService, type Flow, DraftService } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import Badge from '../badge/Badge.svelte'
	import Button from '../button/Button.svelte'
	import Row from './Row.svelte'
	import DraftBadge from '$lib/components/DraftBadge.svelte'
	import { sendUserToast } from '$lib/toast'
	import { DELETE, copyToClipboard, isOwner } from '$lib/utils'
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
		Share,
		Archive,
		Clipboard,
		Eye,
		HistoryIcon
	} from 'lucide-svelte'
	import FlowHistory from '$lib/components/flows/FlowHistory.svelte'
	import { getDeployUiSettings } from '$lib/components/home/deploy_ui'

	export let flow: Flow & { has_draft?: boolean; draft_only?: boolean; canWrite: boolean }
	export let marked: string | undefined
	export let starred: boolean
	export let shareModal: ShareModal
	export let moveDrawer: MoveDrawer
	export let deleteConfirmedCallback: (() => void) | undefined
	export let deploymentDrawer: DeployWorkspaceDrawer
	export let errorHandlerMuted: boolean
	export let depth: number = 0
	export let menuOpen: boolean = false

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
			await FlowService.deleteFlowByPath({ workspace: $workspaceStore!, path })
			dispatch('change')
			sendUserToast(`Deleted flow ${path}`)
		} catch (err) {
			sendUserToast(`Could not delete this flow ${err.body}`, true)
		}
	}
	let scheduleEditor: ScheduleEditor
	let flowHistory: FlowHistory
</script>

{#if menuOpen}
	<ScheduleEditor onUpdate={() => goto('/schedules')} bind:this={scheduleEditor} />
	<FlowHistory bind:this={flowHistory} path={flow.path} />
{/if}

<Row
	href={flow.draft_only
		? `${base}/flows/edit/${flow.path}?nodraft=true`
		: `${base}/flows/get/${flow.path}?workspace=${$workspaceStore}`}
	kind="flow"
	workspaceId={flow.workspace_id ?? $workspaceStore ?? ''}
	{marked}
	path={flow.path}
	summary={flow.summary}
	{starred}
	{errorHandlerMuted}
	on:change
	canFavorite={!flow.draft_only}
	{depth}
>
	<svelte:fragment slot="badges">
		{#if flow.archived}
			<Badge color="red" baseClass="border">archived</Badge>
		{/if}
		<SharedBadge canWrite={flow.canWrite} extraPerms={flow.extra_perms} />
		<DraftBadge has_draft={flow.has_draft} draft_only={flow.draft_only} />
		<div class="w-8 center-center"></div>
	</svelte:fragment>
	<svelte:fragment slot="actions">
		<span class="hidden md:inline-flex gap-x-1">
			{#if !$userStore?.operator}
				{#if flow.canWrite && !flow.archived}
					<div>
						<Button
							color="light"
							size="xs"
							variant="border"
							startIcon={{ icon: Pen }}
							href="{base}/flows/edit/{flow.path}?nodraft=true"
						>
							Edit
						</Button>
					</div>
				{:else}
					<div>
						<Button
							color="light"
							size="xs"
							variant="border"
							startIcon={{ icon: GitFork }}
							href="{base}/flows/add?template={flow.path}"
						>
							Fork
						</Button>
					</div>
				{/if}
			{/if}
		</span>

		<Dropdown
			items={async () => {
				let { draft_only, path, archived, has_draft } = flow
				let owner = isOwner(path, $userStore, $workspaceStore)
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
							disabled: !owner,
							hide: $userStore?.operator
						}
					]
				}
				return [
					{
						displayName: 'Duplicate/Fork',
						icon: GitFork,
						href: `${base}/flows/add?template=${path}`,
						hide: $userStore?.operator
					},
					{
						displayName: 'View runs',
						icon: List,
						href: `${base}/runs/${path}`
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
						disabled: !owner || archived,
						hide: $userStore?.operator
					},
					{
						displayName: 'Copy path',
						icon: Clipboard,
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
							flowHistory.open()
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
						displayName: owner ? 'Share' : 'See Permissions',
						icon: Share,
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
						disabled: !owner,
						hide: $userStore?.operator
					},
					...(has_draft
						? [
								{
									displayName: 'Delete Draft',
									icon: Trash,
									action: async () => {
										await DraftService.deleteDraft({
											workspace: $workspaceStore ?? '',
											path,
											kind: 'flow'
										})
										dispatch('change')
									},
									type: DELETE,
									disabled: !owner,
									hide: $userStore?.operator
								}
							]
						: []),
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
						disabled: !owner,
						hide: $userStore?.operator
					}
				]
			}}
			on:open={() => {
				menuOpen = true
			}}
		/>
	</svelte:fragment>
</Row>
