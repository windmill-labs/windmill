<script lang="ts">
	import { goto } from '$app/navigation'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import type MoveDrawer from '$lib/components/MoveDrawer.svelte'
	import ScheduleEditor from '$lib/components/ScheduleEditor.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import type ShareModal from '$lib/components/ShareModal.svelte'

	import { ScriptService, type Script, DraftService } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'

	import { createEventDispatcher } from 'svelte'
	import Badge from '../badge/Badge.svelte'
	import Button from '../button/Button.svelte'
	import Row from './Row.svelte'
	import DraftBadge from '$lib/components/DraftBadge.svelte'
	import { sendUserToast } from '$lib/toast'
	import { copyToClipboard, DELETE, isOwner } from '$lib/utils'
	import type DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import { LanguageIcon } from '../languageIcons'
	import {
		Archive,
		Calendar,
		Code,
		Copy,
		FileUp,
		GitFork,
		List,
		Pen,
		Share,
		Trash
	} from 'lucide-svelte'

	export let script: Script & { canWrite: boolean }
	export let marked: string | undefined
	export let starred: boolean
	export let shareModal: ShareModal
	export let moveDrawer: MoveDrawer
	export let deploymentDrawer: DeployWorkspaceDrawer
	export let deleteConfirmedCallback: (() => void) | undefined
	export let errorHandlerMuted: boolean
	export let showCode: (path: string, summary: string) => void
	export let depth: number = 0
	export let menuOpen: boolean = false

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

{#if menuOpen}
	<ScheduleEditor on:update={() => goto('/schedules')} bind:this={scheduleEditor} />
{/if}

<Row
	href={draft_only ? `/scripts/edit/${path}` : `/scripts/get/${hash}?workspace=${$workspaceStore}`}
	kind="script"
	{marked}
	{path}
	{summary}
	{starred}
	{errorHandlerMuted}
	workspaceId={$workspaceStore ?? ''}
	on:change
	canFavorite={!draft_only}
	{depth}
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
							startIcon={{ icon: Pen }}
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
							startIcon={{ icon: GitFork }}
							href="/scripts/add?template={path}"
						>
							Fork
						</Button>
					</div>
				{/if}
			{/if}
		</span>
		<Dropdown
			items={() => {
				let owner = isOwner(path, $userStore, $workspaceStore)
				if (draft_only) {
					return [
						{
							displayName: 'View code',
							icon: Code,
							action: () => {
								showCode(script.path, script.summary)
							}
						},
						{
							displayName: 'Delete',
							icon: Trash,
							action: (event) => {
								// TODO
								// @ts-ignore
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
						displayName: 'View code',
						icon: Code,
						action: () => {
							showCode(script.path, script.summary)
						}
					},
					{
						displayName: 'Duplicate/Fork',
						icon: GitFork,
						href: `/scripts/add?template=${path}`
					},
					{
						displayName: 'Move/Rename',
						icon: FileUp,
						action: () => {
							moveDrawer.openDrawer(path, summary, 'script')
						},
						disabled: !owner || archived
					},
					{
						displayName: 'Deploy to staging/prod',
						icon: FileUp,
						action: () => {
							deploymentDrawer.openDrawer(path, 'script')
						},
						disabled: archived
					},
					{
						displayName: 'View runs',
						icon: List,
						href: `/runs/${path}`
					},
					{
						displayName: 'Schedule',
						icon: Calendar,
						action: () => {
							scheduleEditor.openNew(false, path)
						},
						disabled: archived
					},
					{
						displayName: owner ? 'Share' : 'See Permissions',
						icon: Share,
						action: () => {
							shareModal.openDrawer && shareModal.openDrawer(path, 'script')
						},
						disabled: archived
					},
					{
						displayName: 'Copy path',
						icon: Copy,
						action: () => {
							copyToClipboard(path)
						}
					},
					{
						displayName: archived ? 'Unarchive' : 'Archive',
						icon: Archive,
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
									icon: Trash,
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
									icon: Trash,
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
			on:open={() => {
				menuOpen = true
			}}
		/>
	</svelte:fragment>
</Row>
