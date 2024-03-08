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
		Eye,
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
				lock: r.lock
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
	href={script.draft_only
		? `/scripts/edit/${script.path}`
		: `/scripts/get/${script.hash}?workspace=${$workspaceStore}`}
	kind="script"
	{marked}
	path={script.path}
	summary={script.summary}
	{starred}
	{errorHandlerMuted}
	workspaceId={$workspaceStore ?? ''}
	on:change
	canFavorite={!script.draft_only}
	{depth}
>
	<svelte:fragment slot="badges">
		{#if script.lock_error_logs}
			<Badge color="red" baseClass="border border-red-200">Deployment failed</Badge>
		{/if}

		{#if script.archived}
			<Badge color="red" baseClass="border">archived</Badge>
		{/if}

		<SharedBadge canWrite={script.canWrite} extraPerms={script.extra_perms} />
		<DraftBadge has_draft={script.has_draft} draft_only={script.draft_only} />
		<div class="w-8 center-center">
			<LanguageIcon lang={script.language} width={12} height={12} />
		</div>
	</svelte:fragment>

	<svelte:fragment slot="actions">
		<span class="hidden md:inline-flex gap-x-1">
			{#if !$userStore?.operator}
				{#if script.canWrite && !script.archived}
					<div>
						<Button
							color="light"
							size="xs"
							variant="border"
							startIcon={{ icon: Pen }}
							href="/scripts/edit/{script.path}"
						>
							Edit
						</Button>
					</div>
				{:else if !script.draft_only}
					<div>
						<Button
							color="light"
							size="xs"
							variant="border"
							startIcon={{ icon: GitFork }}
							href="/scripts/add?template={script.path}"
						>
							Fork
						</Button>
					</div>
				{/if}
			{/if}
		</span>
		<Dropdown
			items={() => {
				let owner = isOwner(script.path, $userStore, $workspaceStore)
				if (script.draft_only) {
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
									deleteScript(script.path)
								} else {
									deleteConfirmedCallback = () => {
										deleteScript(script.path)
									}
								}
							},
							type: dlt,
							disabled: !script.canWrite
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
						href: `/scripts/add?template=${script.path}`,
						hide: $userStore?.operator
					},
					{
						displayName: 'Move/Rename',
						icon: FileUp,
						action: () => {
							moveDrawer.openDrawer(script.path, script.summary, 'script')
						},
						disabled: !owner || script.archived,
						hide: $userStore?.operator
					},
					{
						displayName: 'Deploy to staging/prod',
						icon: FileUp,
						action: () => {
							deploymentDrawer.openDrawer(script.path, 'script')
						},
						disabled: script.archived,
						hide: $userStore?.operator
					},
					{
						displayName: 'View runs',
						icon: List,
						href: `/runs/${script.path}`
					},
					{
						displayName: 'Audit logs',
						icon: Eye,
						href: `/audit_logs?resource=${script.path}`,
						hide: $userStore?.operator
					},
					{
						displayName: 'Schedule',
						icon: Calendar,
						action: () => {
							scheduleEditor.openNew(false, script.path)
						},
						disabled: script.archived,
						hide: $userStore?.operator
					},
					{
						displayName: owner ? 'Share' : 'See Permissions',
						icon: Share,
						action: () => {
							shareModal.openDrawer && shareModal.openDrawer(script.path, 'script')
						},
						disabled: script.archived,
						hide: $userStore?.operator
					},
					{
						displayName: 'Copy path',
						icon: Copy,
						action: () => {
							copyToClipboard(script.path)
						}
					},
					{
						displayName: script.archived ? 'Unarchive' : 'Archive',
						icon: Archive,
						action: () => {
							script.archived
								? script.path && unarchiveScript(script.path)
								: script.path && archiveScript(script.path)
						},
						type: 'delete',
						disabled: !owner,
						hide: $userStore?.operator
					},

					...(script.has_draft
						? [
								{
									displayName: 'Delete Draft',
									icon: Trash,
									action: async () => {
										await DraftService.deleteDraft({
											workspace: $workspaceStore ?? '',
											path: script.path,
											kind: 'script'
										})
										dispatch('change')
									},
									type: DELETE,
									disabled: !owner,
									hide: $userStore?.operator
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
											deleteScript(script.path)
										} else {
											deleteConfirmedCallback = () => {
												deleteScript(script.path)
											}
										}
									},
									type: dlt,
									disabled: !script.canWrite,
									hide: $userStore?.operator
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
