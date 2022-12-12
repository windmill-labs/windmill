<script lang="ts">
	import Dropdown from '$lib/components/Dropdown.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import type ShareModal from '$lib/components/ShareModal.svelte'

	import { FlowService, type Flow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
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

	import Button from '../button/Button.svelte'
	import Row from './Row.svelte'

	export let flow: Flow & { canWrite: boolean }
	export let marked: string | undefined
	export let starred: boolean
	export let shareModal: ShareModal

	let { summary, path, extra_perms, canWrite, workspace_id } = flow

	const dispatch = createEventDispatcher()

	async function archiveFlow(path: string): Promise<void> {
		try {
			await FlowService.archiveFlowByPath({ workspace: $workspaceStore!, path })
			dispatch('change')
			sendUserToast(`Successfully archived flow ${path}`)
		} catch (err) {
			sendUserToast(`Could not archive this flow ${err.body}`, true)
		}
	}
</script>

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
	</svelte:fragment>
	<svelte:fragment slot="actions">
		<span class="hidden md:inline-flex gap-x-1">
			{#if canWrite}
				<div>
					<Button
						color="light"
						size="xs"
						variant="border"
						startIcon={{ icon: faEdit }}
						href="/flows/edit/{path}"
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

			<Button
				href="/flows/get/{path}"
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
			dropdownItems={[
				{
					displayName: 'View flow',
					icon: faEye,
					href: `/flows/get/${path}`
				},
				{
					displayName: 'Edit',
					icon: faEdit,
					href: `/flows/edit/${path}`,
					disabled: !canWrite
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
					displayName: 'Schedule',
					icon: faCalendarAlt,
					href: `/schedule/add?path=${path}&isFlow=true`
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
						path ? archiveFlow(path) : null
					},
					type: 'delete',
					disabled: !canWrite
				}
			]}
		/>
	</svelte:fragment>
</Row>
