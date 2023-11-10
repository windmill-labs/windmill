<script lang="ts">
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import type MoveDrawer from '$lib/components/MoveDrawer.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import type ShareModal from '$lib/components/ShareModal.svelte'
	import { RawAppService, type ListableRawApp } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import Button from '../button/Button.svelte'
	import Row from './Row.svelte'
	import Drawer from '../drawer/Drawer.svelte'
	import DrawerContent from '../drawer/DrawerContent.svelte'
	import FileInput from '../fileInput/FileInput.svelte'
	import { goto } from '$app/navigation'
	import type DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import { FileUp, Globe, Pen, Share, Trash } from 'lucide-svelte'

	export let app: ListableRawApp & { canWrite: boolean }
	export let marked: string | undefined
	export let starred: boolean
	export let shareModal: ShareModal
	export let moveDrawer: MoveDrawer
	export let deleteConfirmedCallback: (() => void) | undefined
	export let deploymentDrawer: DeployWorkspaceDrawer
	export let depth: number = 0
	export let menuOpen: boolean = false

	let updateAppDrawer: Drawer

	let { summary, version, path, extra_perms, workspace_id, canWrite } = app

	const dispatch = createEventDispatcher()
</script>

{#if menuOpen}
	<Drawer bind:this={updateAppDrawer} size="800px">
		<DrawerContent title="Update app" on:close={() => updateAppDrawer?.toggleDrawer?.()}>
			<FileInput
				accept={'.js'}
				multiple={false}
				convertTo={'text'}
				iconSize={24}
				class="text-sm py-4"
				on:change={async ({ detail }) => {
					await RawAppService.updateRawApp({
						workspace: $workspaceStore ?? '',
						path,
						requestBody: { value: detail?.[0] }
					})
					goto(`/apps/get_raw/${version + 1}/${path}`)
				}}
			/>
		</DrawerContent>
	</Drawer>
{/if}
<Row
	href="/apps/get_raw/{version}/{path}"
	kind="raw_app"
	{marked}
	{path}
	{summary}
	workspaceId={workspace_id ?? $workspaceStore ?? ''}
	{starred}
	on:change
	canFavorite={true}
	{depth}
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
							startIcon={{ icon: Pen }}
							on:click={() => updateAppDrawer?.toggleDrawer?.()}
						>
							Edit
						</Button>
					</div>
				{/if}
			{/if}
		</span>
		<Dropdown
			items={() => {
				return [
					{
						displayName: 'Move/Rename',
						icon: FileUp,
						action: () => {
							moveDrawer.openDrawer(path, summary, 'raw_app')
						},
						disabled: !canWrite
					},
					{
						displayName: 'Deploy to prod/staging',
						icon: Globe,
						action: () => {
							deploymentDrawer.openDrawer(path, 'raw_app')
						}
					},
					{
						displayName: canWrite ? 'Share' : 'See Permissions',
						icon: Share,
						action: () => {
							shareModal.openDrawer && shareModal.openDrawer(path, 'raw_app')
						}
					},
					{
						displayName: 'Delete',
						icon: Trash,
						action: async (event) => {
							// TODO
							// @ts-ignore
							if (event?.shiftKey) {
								await RawAppService.deleteRawApp({ workspace: $workspaceStore ?? '', path })
								dispatch('change')
							} else {
								deleteConfirmedCallback = async () => {
									await RawAppService.deleteRawApp({ workspace: $workspaceStore ?? '', path })
									dispatch('change')
								}
							}
						},
						type: 'delete',
						disabled: !canWrite
					}
				]
			}}
			on:open={() => {
				menuOpen = true
			}}
		/>
	</svelte:fragment>
</Row>
