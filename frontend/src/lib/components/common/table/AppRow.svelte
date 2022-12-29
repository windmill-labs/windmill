<script lang="ts">
	import Dropdown from '$lib/components/Dropdown.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import { AppService, type ListableApp } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { faEdit, faEye, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { MoreVertical } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import Button from '../button/Button.svelte'
	import Row from './Row.svelte'

	export let app: ListableApp & { canWrite: boolean }
	export let marked: string | undefined
	export let starred: boolean

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
							href="/apps/edit/{path}"
						>
							Edit
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
					displayName: 'View app',
					icon: faEye,
					href: `/apps/get/${path}`
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
