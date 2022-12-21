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
		faShare
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

	let {
		summary,
		path,
		hash,
		language,
		extra_perms,
		canWrite,
		lock_error_logs,
		kind,
		workspace_id
	} = script

	const dispatch = createEventDispatcher()

	async function archiveScript(path: string): Promise<void> {
		await ScriptService.archiveScriptByPath({ workspace: $workspaceStore!, path })
		dispatch('change')
		sendUserToast(`Archived script ${path}`)
	}
	let scheduleEditor: ScheduleEditor
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
		<span class="hidden md:block">
			<SharedBadge {canWrite} extraPerms={extra_perms} />
			{#if lock_error_logs}
				<Badge color="red" baseClass="border border-red-200">Deployment failed</Badge>
			{/if}
			{#if kind !== 'script'}
				<Badge color="gray" baseClass="border">{capitalize(kind)}</Badge>
			{/if}

			<LanguageBadge {language} />
		</span>
	</svelte:fragment>

	<svelte:fragment slot="actions">
		<span class="hidden md:inline-flex gap-x-1">
			{#if !$userStore?.operator}
				{#if canWrite}
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
				href="/scripts/get/{hash}"
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
					href: `/scripts/get/${hash}`
				},

				{
					displayName: 'Edit',
					icon: faEdit,
					href: `/scripts/edit/${hash}`,
					disabled: !canWrite
				},
				{
					displayName: 'Edit code',
					icon: faEdit,
					href: `/scripts/edit/${hash}?step=2`,
					disabled: !canWrite
				},
				{
					displayName: 'Use as template',
					icon: faCodeFork,
					href: `/scripts/add?template=${path}`
				},
				{
					displayName: 'Move',
					icon: faFileExport,
					action: () => {
						moveDrawer.openDrawer(path, 'script')
					}
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
					}
				},
				{
					displayName: canWrite ? 'Share' : 'See Permissions',
					icon: faShare,
					action: () => {
						shareModal.openDrawer && shareModal.openDrawer(path, 'script')
					}
				},
				{
					displayName: 'Archive',
					icon: faArchive,
					action: () => {
						path ? archiveScript(path) : null
					},
					type: 'delete',
					disabled: !canWrite
				}
			]}
		/>
	</svelte:fragment>
</Row>
