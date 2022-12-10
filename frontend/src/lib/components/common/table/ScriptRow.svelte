<script lang="ts">
	import Dropdown from '$lib/components/Dropdown.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import type ShareModal from '$lib/components/ShareModal.svelte'

	import { ScriptService, type Script } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { capitalize, sendUserToast } from '$lib/utils'
	import {
		faArchive,
		faCalendarAlt,
		faCodeFork,
		faEdit,
		faEye,
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
		sendUserToast(`Successfully archived script ${path}`)
	}
</script>

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
		<Dropdown
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
					displayName: 'View runs',
					icon: faList,
					href: `/runs/${path}`
				},
				{
					displayName: 'Schedule',
					icon: faCalendarAlt,
					href: `/schedule/add?path=${path}`
				},
				{
					displayName: 'Share',
					icon: faShare,
					action: () => {
						shareModal.openDrawer && shareModal.openDrawer(path)
					},
					disabled: !canWrite
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
	</svelte:fragment>
</Row>
