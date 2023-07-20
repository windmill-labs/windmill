<script lang="ts">
	import { goto } from '$app/navigation'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import type MoveDrawer from '$lib/components/MoveDrawer.svelte'
	import ScheduleEditor from '$lib/components/ScheduleEditor.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import type ShareModal from '$lib/components/ShareModal.svelte'

	import { ScriptService, type Script, DraftService } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import {
		faArchive,
		faCalendarAlt,
		faCodeFork,
		faCopy,
		faEdit,
		faFileExport,
		faList,
		faShare,
		faTrashAlt
	} from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Badge from '../badge/Badge.svelte'
	import Button from '../button/Button.svelte'
	import Row from './Row.svelte'
	import DraftBadge from '$lib/components/DraftBadge.svelte'
	import { sendUserToast } from '$lib/toast'
	import { copyToClipboard, DELETE, isOwner } from '$lib/utils'
	import type DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import { LanguageIcon } from '../languageIcons'

	export let script: Script & { canWrite: boolean }
	export let marked: string | undefined
	export let starred: boolean
	export let shareModal: ShareModal
	export let moveDrawer: MoveDrawer
	export let deploymentDrawer: DeployWorkspaceDrawer
	export let deleteConfirmedCallback: (() => void) | undefined

	let {
		summary,
		path,
		hash,
		language,
		extra_perms,
		canWrite,
		lock_error_logs,
		archived,
		has_draft,
		draft_only
	} = script

	const dispatch = createEventDispatcher()

	async function archiveScript(path: string): Promise<void> {
		await ScriptService.archiveScriptByPath({ workspace: $workspaceStore!, path })
		dispatch('change')
		sendUserToast(`Archived script ${path}`)
	}

	async function unarchiveScript(path: string): Promise<void> {
		const r = await ScriptService.getScriptByPath({ workspace: $workspaceStore!, path })
		await ScriptService.createScript({
			workspace: $workspaceStore!,
			requestBody: {
				...r,
				parent_hash: r.hash,
				lock: r.lock?.split('\n')
			}
		})
		dispatch('change')
		sendUserToast(`Unarchived script ${path}`)
	}

	async function deleteScript(path: string): Promise<void> {
		await ScriptService.deleteScriptByPath({ workspace: $workspaceStore!, path })
		dispatch('change')
		sendUserToast(`Delete script ${path}`)
	}
	let scheduleEditor: ScheduleEditor

	const dlt: 'delete' = 'delete'
</script>

<ScheduleEditor on:update={() => goto('/schedules')} bind:this={scheduleEditor} />

<Row
	href="/scripts/get/{hash}?workspace={$workspaceStore}"
	kind="script"
	{marked}
	{path}
	{summary}
	{starred}
	workspaceId={$workspaceStore ?? ''}
	on:change
	canFavorite={!draft_only}
>
	<svelte:fragment slot="badges">
		{#if lock_error_logs}
			<Badge color="red" baseClass="border border-red-200">Deployment failed</Badge>
		{/if}

		{#if archived}
			<Badge color="red" baseClass="border">archived</Badge>
		{/if}

		<SharedBadge {canWrite} extraPerms={extra_perms} />
		<DraftBadge {has_draft} {draft_only} />
		<div class="w-8 center-center">
			<LanguageIcon lang={language} width={12} height={12} />
		</div>
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
							href="/scripts/edit/{path}"
						>
							Edit
						</Button>
					</div>
				{:else if !draft_only}
					<div>
						<Button
							color="light"
							size="xs"
							variant="border"
							startIcon={{ icon: faCodeFork }}
							href="/scripts/add?template={path}"
						>
							Fork
						</Button>
					</div>
				{/if}
			{/if}
		</span>
		<Dropdown
			placement="bottom-end"
			dropdownItems={() => {
				let owner = isOwner(path, $userStore, $workspaceStore)
				if (draft_only) {
					return [
						{
							displayName: 'Delete',
							icon: faTrashAlt,
							action: (event) => {
								if (event?.shiftKey) {
									deleteScript(path)
								} else {
									deleteConfirmedCallback = () => {
										deleteScript(path)
									}
								}
							},
							type: dlt,
							disabled: !canWrite
						}
					]
				}
				return [
					{
						displayName: 'Duplicate/Fork',
						icon: faCodeFork,
						href: `/scripts/add?template=${path}`
					},
					{
						displayName: 'Move/Rename',
						icon: faFileExport,
						action: () => {
							moveDrawer.openDrawer(path, summary, 'script')
						},
						disabled: !owner || archived
					},
					{
						displayName: 'Deploy to staging/prod',
						icon: faFileExport,
						action: () => {
							deploymentDrawer.openDrawer(path, 'script')
						},
						disabled: archived
					},
					{
						displayName: 'View runs',
						icon: faList,
						href: `/runs/${path}`
					},
					{
						displayName: 'Schedule',
						icon: faCalendarAlt,
						action: () => {
							scheduleEditor.openNew(false, path)
						},
						disabled: archived
					},
					{
						displayName: owner ? 'Share' : 'See Permissions',
						icon: faShare,
						action: () => {
							shareModal.openDrawer && shareModal.openDrawer(path, 'script')
						},
						disabled: archived
					},
					{
						displayName: 'Copy path',
						icon: faCopy,
						action: () => {
							copyToClipboard(path)
						}
					},
					{
						displayName: archived ? 'Unarchive' : 'Archive',
						icon: faArchive,
						action: () => {
							archived ? path && unarchiveScript(path) : path && archiveScript(path)
						},
						type: 'delete',
						disabled: !owner
					},

					...(has_draft
						? [
								{
									displayName: 'Delete Draft',
									icon: faTrashAlt,
									action: async () => {
										await DraftService.deleteDraft({
											workspace: $workspaceStore ?? '',
											path,
											kind: 'script'
										})
										dispatch('change')
									},
									type: DELETE,
									disabled: !owner
								}
						  ]
						: []),
					...($userStore?.is_admin || $userStore?.is_super_admin
						? [
								{
									displayName: 'Delete',
									icon: faTrashAlt,
									action: (event) => {
										if (event?.shiftKey) {
											deleteScript(path)
										} else {
											deleteConfirmedCallback = () => {
												deleteScript(path)
											}
										}
									},
									type: dlt,
									disabled: !canWrite
								}
						  ]
						: [])
				]
			}}
		/>
	</svelte:fragment>
</Row>
