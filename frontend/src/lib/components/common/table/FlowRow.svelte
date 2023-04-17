<script lang="ts">
	import { goto } from '$app/navigation'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import type MoveDrawer from '$lib/components/MoveDrawer.svelte'
	import ScheduleEditor from '$lib/components/ScheduleEditor.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import type ShareModal from '$lib/components/ShareModal.svelte'
	import { FlowService, type Flow } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { isOwner, sendUserToast } from '$lib/utils'
	import {
		faArchive,
		faCalendarAlt,
		faCodeFork,
		faEdit,
		faEye,
		faFileExport,
		faList,
		faPlay,
		faShare,
		faTrashAlt
	} from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Badge from '../badge/Badge.svelte'
	import Button from '../button/Button.svelte'
	import Row from './Row.svelte'

	export let flow: Flow & { canWrite: boolean }
	export let marked: string | undefined
	export let starred: boolean
	export let shareModal: ShareModal
	export let moveDrawer: MoveDrawer
	export let deleteConfirmedCallback: (() => void) | undefined

	let { summary, path, extra_perms, canWrite, workspace_id, archived } = flow

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
</script>

<ScheduleEditor on:update={() => goto('/schedules')} bind:this={scheduleEditor} />
<Row
	href={`/flows/run/${path}`}
	kind="flow"
	workspaceId={workspace_id ?? $workspaceStore ?? ''}
	{marked}
	{path}
	{summary}
	{starred}
	on:change
>
	<svelte:fragment slot="badges">
		<SharedBadge {canWrite} extraPerms={extra_perms} />

		{#if archived}
			<Badge color="red" baseClass="border">archived</Badge>
		{/if}
	</svelte:fragment>
	<svelte:fragment slot="actions">
		<span class="hidden md:inline-flex gap-x-1">
			{#if !$userStore?.operator}
				{#if canWrite && !archived}
					<div>
						<Button
							color="light"
							size="xs"
							variant="border"
							startIcon={{ icon: faEdit }}
							href="/flows/edit/{path}?nodraft=true"
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
							startIcon={{ icon: faCodeFork }}
							href="/flows/add?template={path}"
						>
							Fork
						</Button>
					</div>
				{/if}
			{/if}

			<Button
				href="/flows/get/{path}?workspace={$workspaceStore}"
				color="light"
				variant="border"
				size="xs"
				spacingSize="md"
				startIcon={{ icon: faEye }}
			>
				Detail
			</Button>
			<Button
				href="/flows/run/{path}"
				color="dark"
				size="xs"
				spacingSize="md"
				endIcon={{ icon: faPlay }}
			>
				Run
			</Button>
		</span>

		<Dropdown
			placement="bottom-end"
			dropdownItems={() => {
				let owner = isOwner(path, $userStore, $workspaceStore)
				return [
					{
						displayName: 'View flow',
						icon: faEye,
						href: `/flows/get/${path}?workspace=${$workspaceStore}`
					},
					{
						displayName: 'Edit',
						icon: faEdit,
						href: `/flows/edit/${path}?nodraft=true`,
						disabled: !canWrite || archived
					},
					{
						displayName: 'Use as template/Fork',
						icon: faCodeFork,
						href: `/flows/add?template=${path}`
					},
					{
						displayName: 'View runs',
						icon: faList,
						href: `/runs/${path}`
					},
					{
						displayName: 'Move/Rename',
						icon: faFileExport,
						action: () => {
							moveDrawer.openDrawer(path, summary, 'flow')
						},
						disabled: !owner || archived
					},
					{
						displayName: 'Schedule',
						icon: faCalendarAlt,
						action: () => {
							scheduleEditor.openNew(true, path)
						},
						disabled: archived
					},
					{
						displayName: owner ? 'Share' : 'See Permissions',
						icon: faShare,
						action: () => {
							shareModal.openDrawer && shareModal.openDrawer(path, 'flow')
						}
					},
					{
						displayName: archived ? 'Unarchive' : 'Archive',
						icon: faArchive,
						action: () => {
							path && archiveFlow(path, !archived)
						},
						type: 'delete',
						disabled: !owner
					},
					{
						displayName: 'Delete',
						icon: faTrashAlt,
						action: (event) => {
							if (event?.shiftKey) {
								deleteFlow(path)
							} else {
								deleteConfirmedCallback = () => {
									deleteFlow(path)
								}
							}
						},
						type: 'delete',
						disabled: !owner
					}
				]
			}}
		/>
	</svelte:fragment>
</Row>
