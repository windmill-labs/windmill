<script lang="ts">
	import { goto } from '$app/navigation'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import type MoveDrawer from '$lib/components/MoveDrawer.svelte'
	import ScheduleEditor from '$lib/components/ScheduleEditor.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import type ShareModal from '$lib/components/ShareModal.svelte'

	import { ScriptService, type Script } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { capitalize, sendUserToast } from '$lib/utils'
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
	import LanguageBadge from './LanguageBadge.svelte'
	import Row from './Row.svelte'

	export let script: Script & { canWrite: boolean }
	export let marked: string | undefined
	export let starred: boolean
	export let shareModal: ShareModal
	export let moveDrawer: MoveDrawer
	export let deleteConfirmedCallback: (() => void) | undefined

	let {
		summary,
		path,
		hash,
		language,
		extra_perms,
		canWrite,
		lock_error_logs,
		kind,
		workspace_id,
		archived
	} = script

	const dispatch = createEventDispatcher()

	async function archiveScript(path: string): Promise<void> {
		await ScriptService.archiveScriptByPath({ workspace: $workspaceStore!, path })
		dispatch('change')
		sendUserToast(`Archived script ${path}`)
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
	href={`/scripts/run/${hash}`}
	kind="script"
	{marked}
	{path}
	{summary}
	{starred}
	workspaceId={workspace_id ?? $workspaceStore ?? ''}
	on:change
>
	<svelte:fragment slot="badges">
		<SharedBadge {canWrite} extraPerms={extra_perms} />
		{#if lock_error_logs}
			<Badge color="red" baseClass="border border-red-200">Deployment failed</Badge>
		{/if}
		{#if kind !== 'script'}
			<Badge color="gray" baseClass="border">{capitalize(kind)}</Badge>
		{/if}

		{#if archived}
			<Badge color="red" baseClass="border">archived</Badge>
		{/if}

		<LanguageBadge {language} />
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
							href="/scripts/edit/{hash}?step=2"
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
							href="/scripts/add?template={path}"
						>
							Fork
						</Button>
					</div>
				{/if}
			{/if}

			<Button
				href="/scripts/get/{hash}?workspace_id={$workspaceStore}"
				color="light"
				variant="border"
				size="xs"
				spacingSize="md"
				startIcon={{ icon: faEye }}
			>
				Detail
			</Button>
			<Button
				href="/scripts/run/{hash}"
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
			dropdownItems={[
				{
					displayName: 'View script',
					icon: faEye,
					href: `/scripts/get/${hash}?workspace_id=${$workspaceStore}`
				},

				{
					displayName: 'Edit',
					icon: faEdit,
					href: `/scripts/edit/${hash}`,
					disabled: !canWrite || archived
				},
				{
					displayName: 'Edit code',
					icon: faEdit,
					href: `/scripts/edit/${hash}?step=2`,
					disabled: !canWrite || archived
				},
				{
					displayName: 'Use as template',
					icon: faCodeFork,
					href: `/scripts/add?template=${path}`
				},
				{
					displayName: 'Move/Rename',
					icon: faFileExport,
					action: () => {
						moveDrawer.openDrawer(path, summary, 'script')
					},
					disabled: !canWrite || archived
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
					displayName: canWrite ? 'Share' : 'See Permissions',
					icon: faShare,
					action: () => {
						shareModal.openDrawer && shareModal.openDrawer(path, 'script')
					},
					disabled: archived
				},
				{
					displayName: 'Archive',
					icon: faArchive,
					action: () => {
						path ? archiveScript(path) : null
					},
					type: 'delete',
					disabled: !canWrite || archived
				},
				...($userStore?.is_admin || $userStore?.is_super_admin
					? []
					: [
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
					  ])
			]}
		/>
	</svelte:fragment>
</Row>
