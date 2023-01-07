<script lang="ts">
	import Dropdown from '$lib/components/Dropdown.svelte'
	import type MoveDrawer from '$lib/components/MoveDrawer.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import type ShareModal from '$lib/components/ShareModal.svelte'
	import { AppService, type ListableApp } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import {
		faCodeFork,
		faEdit,
		faEye,
		faFileExport,
		faPen,
		faShare,
		faTrashAlt
	} from '@fortawesome/free-solid-svg-icons'
	import { MoreVertical } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import Button from '../button/Button.svelte'
	import Row from './Row.svelte'

	export let app: ListableApp & { canWrite: boolean }
	export let marked: string | undefined
	export let starred: boolean
	export let shareModal: ShareModal
	export let moveDrawer: MoveDrawer

	let { summary, path, extra_perms, canWrite, workspace_id } = app

	const dispatch = createEventDispatcher()
</script>

<Row
	href={`/apps/get/${path}`}
	kind="app"
	{marked}
	{path}
	{summary}
	workspaceId={workspace_id ?? $workspaceStore ?? ''}
	{starred}
	on:change
>
	<svelte:fragment slot="badges">
		<SharedBadge {canWrite} extraPerms={extra_perms} />
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
							href="/apps/edit/{path}?nodraft=true"
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
							href="/apps/add?template={path}"
						>
							Fork
						</Button>
					</div>
				{/if}
			{/if}

			<Button
				href="/apps/get/{path}"
				color="dark"
				size="xs"
				spacingSize="md"
				startIcon={{ icon: faEye }}
			>
				View
			</Button>
		</span>
		<Dropdown
			placement="bottom-end"
			btnClasses="!text-gray-700 !bg-transparent hover:!bg-gray-400/20 !p-[6px]"
			dropdownItems={[
				{
					displayName: 'View',
					icon: faEye,
					href: `/apps/get/${path}`
				},
				{
					displayName: 'Edit',
					icon: faPen,
					href: `/apps/edit/${path}?nodraft=true`
				},
				{
					displayName: 'Move/Rename',
					icon: faFileExport,
					action: () => {
						moveDrawer.openDrawer(path, summary, 'app')
					},
					disabled: !canWrite
				},
				{
					displayName: canWrite ? 'Share' : 'See Permissions',
					icon: faShare,
					action: () => {
						shareModal.openDrawer && shareModal.openDrawer(path, 'app')
					}
				},
				{
					displayName: 'Delete',
					icon: faTrashAlt,
					action: async () => {
						await AppService.deleteApp({ workspace: $workspaceStore ?? '', path })
						dispatch('change')
					},
					type: 'delete',
					disabled: !canWrite
				}
			]}
		>
			<MoreVertical size={20} />
		</Dropdown>
	</svelte:fragment>
</Row>
