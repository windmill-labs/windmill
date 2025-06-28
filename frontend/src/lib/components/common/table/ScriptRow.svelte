<script lang="ts">
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import type MoveDrawer from '$lib/components/MoveDrawer.svelte'
	import ScheduleEditor from '$lib/components/triggers/schedules/ScheduleEditor.svelte'
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
	import { capitalize, copyToClipboard, DELETE, isOwner } from '$lib/utils'
	import { isDeployable } from '$lib/utils_deployable'

	import type DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import { LanguageIcon } from '../languageIcons'
	import {
		Archive,
		Calendar,
		Code,
		Copy,
		Eye,
		FolderOpen,
		ChevronUpSquare,
		GitFork,
		List,
		Pen,
		Share,
		Trash,
		History
	} from 'lucide-svelte'
	import ScriptVersionHistory from '$lib/components/ScriptVersionHistory.svelte'
	import { Drawer, DrawerContent } from '..'
	import NoMainFuncBadge from '$lib/components/NoMainFuncBadge.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { getDeployUiSettings } from '$lib/components/home/deploy_ui'

	interface Props {
		script: Script & { canWrite: boolean; use_codebase: boolean }
		marked: string | undefined
		starred: boolean
		shareModal: ShareModal
		moveDrawer: MoveDrawer
		deploymentDrawer: DeployWorkspaceDrawer
		deleteConfirmedCallback: (() => void) | undefined
		errorHandlerMuted: boolean
		showCode: (path: string, summary: string) => void
		depth?: number
		menuOpen?: boolean
	}

	let {
		script,
		marked,
		starred,
		shareModal,
		moveDrawer,
		deploymentDrawer,
		deleteConfirmedCallback = $bindable(),
		errorHandlerMuted,
		showCode,
		depth = 0,
		menuOpen = $bindable(false)
	}: Props = $props()

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
		sendUserToast(`Deleted script ${path}`)
	}
	let scheduleEditor: ScheduleEditor | undefined = $state(undefined)

	const dlt: 'delete' = 'delete'
	let versionsDrawerOpen: boolean = $state(false)
</script>

{#if menuOpen}
	<ScheduleEditor onUpdate={() => goto('/schedules')} bind:this={scheduleEditor} />
{/if}

<Row
	aiId={`script-run-button-${script.path}`}
	aiDescription={`Button to access the form to run the script ${script.summary ?? script.path}`}
	href={script.draft_only || script.kind !== 'script' || script.no_main_func
		? `${base}/scripts/edit/${script.path}`
		: `${base}/scripts/get/${script.hash}?workspace=${$workspaceStore}`}
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
	{#snippet badges()}
		{#if script.lock_error_logs}
			<Badge color="red" baseClass="border border-red-200">Deployment failed</Badge>
		{/if}

		{#if script.archived}
			<Badge color="red" baseClass="border">Archived</Badge>
		{/if}

		{#if script.no_main_func && script.kind !== 'preprocessor'}
			<NoMainFuncBadge />
		{/if}
		{#if script.kind !== 'script'}
			<Badge color="blue" baseClass="border">{capitalize(script.kind)}</Badge>
		{/if}
		<SharedBadge canWrite={script.canWrite} extraPerms={script.extra_perms} />
		<DraftBadge has_draft={script.has_draft} draft_only={script.draft_only} />
		<div class="w-8 center-center">
			<LanguageIcon lang={script.language} width={12} height={12} />
		</div>
	{/snippet}

	{#snippet actions()}
		<span class="hidden md:inline-flex gap-x-1">
			{#if !$userStore?.operator}
				{#if script.use_codebase}
					<Badge
						>bundle<Tooltip
							>This script is deployed as a bundle and can only be deployed from the CLI for now</Tooltip
						></Badge
					>
				{:else if script.canWrite && !script.archived}
					<div>
						<Button
							aiId={`edit-script-button-${script.summary?.length > 0 ? script.summary : script.path}`}
							aiDescription={`Edits the script ${script.summary?.length > 0 ? script.summary : script.path}`}
							color="light"
							size="xs"
							variant="border"
							startIcon={{ icon: Pen }}
							href="{base}/scripts/edit/{script.path}"
						>
							Edit
						</Button>
					</div>
				{:else if !script.draft_only}
					<div>
						<Button
							aiId={`fork-script-button-${script.summary ?? script.path}`}
							aiDescription={`Fork the script ${script.summary ?? script.path}`}
							color="light"
							size="xs"
							variant="border"
							startIcon={{ icon: GitFork }}
							href="{base}/scripts/add?template={script.path}"
						>
							Fork
						</Button>
					</div>
				{/if}
			{/if}
		</span>
		<Dropdown
			aiId={`script-row-dropdown-${script.summary?.length > 0 ? script.summary : script.path}`}
			aiDescription={`Open dropdown for script ${script.summary?.length > 0 ? script.summary : script.path} options`}
			items={async () => {
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
						href: `${base}/scripts/add?template=${script.path}`,
						hide: $userStore?.operator
					},
					{
						displayName: 'Move/Rename',
						icon: FolderOpen,
						action: () => {
							moveDrawer.openDrawer(script.path, script.summary, 'script')
						},
						disabled: !owner || script.archived,
						hide: $userStore?.operator
					},
					...(isDeployable('script', script.path, await getDeployUiSettings())
						? [
								{
									displayName: 'Deploy to staging/prod',
									icon: ChevronUpSquare,
									action: () => {
										deploymentDrawer.openDrawer(script.path, 'script')
									},
									disabled: script.archived,
									hide: $userStore?.operator
								}
							]
						: []),
					{
						displayName: 'View runs',
						icon: List,
						href: `${base}/runs/${script.path}`
					},
					{
						displayName: 'Versions',
						icon: History,
						action: () => {
							versionsDrawerOpen = true
						}
					},
					{
						displayName: 'Audit logs',
						icon: Eye,
						href: `${base}/audit_logs?resource=${script.path}`,
						hide: $userStore?.operator
					},
					{
						displayName: 'Schedule',
						icon: Calendar,
						action: () => {
							scheduleEditor?.openNew(false, script.path)
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
	{/snippet}
</Row>

{#if script}
	<Drawer bind:open={versionsDrawerOpen} size="1200px">
		<DrawerContent title="Versions History" on:close={() => (versionsDrawerOpen = false)}>
			<ScriptVersionHistory
				scriptPath={script.path}
				openDetails
				on:openDetails={(e) => {
					if (script) {
						goto(`/scripts/get/${e.detail.version}?workspace=${$workspaceStore}`)
					}
					versionsDrawerOpen = false
				}}
			/>
		</DrawerContent>
	</Drawer>
{/if}
