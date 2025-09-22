<script lang="ts">
	import { base } from '$lib/base'
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
	import { goto } from '$lib/navigation'
	import type DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import { FolderOpen, Globe, Pen, Share, Trash } from 'lucide-svelte'
	import { isDeployable } from '$lib/utils_deployable'
	import { getDeployUiSettings } from '$lib/components/home/deploy_ui'

	interface Props {
		app: ListableRawApp & { canWrite: boolean }
		marked: string | undefined
		starred: boolean
		shareModal: ShareModal
		moveDrawer: MoveDrawer
		deleteConfirmedCallback: (() => void) | undefined
		deploymentDrawer: DeployWorkspaceDrawer
		depth?: number
		menuOpen?: boolean
	}

	let {
		app,
		marked,
		starred,
		shareModal,
		moveDrawer,
		deleteConfirmedCallback = $bindable(),
		deploymentDrawer,
		depth = 0,
		menuOpen = $bindable(false)
	}: Props = $props()

	let updateAppDrawer: Drawer | undefined = $state(undefined)

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
						path: app.path,
						requestBody: { value: detail?.[0] }
					})
					goto(`/apps/get_raw/${app.version + 1}/${app.path}`)
				}}
			/>
		</DrawerContent>
	</Drawer>
{/if}
<Row
	href="{base}/apps/get_raw/{app.version}/{app.path}"
	kind="raw_app"
	{marked}
	path={app.path}
	summary={app.summary}
	workspaceId={app.workspace_id ?? $workspaceStore ?? ''}
	{starred}
	on:change
	canFavorite={true}
	{depth}
>
	{#snippet badges()}
		<SharedBadge canWrite={app.canWrite} extraPerms={app.extra_perms} />
	{/snippet}
	{#snippet actions()}
		<span class="hidden md:inline-flex gap-x-1">
			{#if !$userStore?.operator}
				{#if app.canWrite}
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
			items={async () => {
				let { summary, path, canWrite } = app

				return [
					{
						displayName: 'Move/Rename',
						icon: FolderOpen,
						action: () => {
							moveDrawer.openDrawer(path, summary, 'raw_app')
						},
						disabled: !canWrite
					},
					...(isDeployable('app', path, await getDeployUiSettings())
						? [
								{
									displayName: 'Deploy to prod/staging',
									icon: Globe,
									action: () => {
										deploymentDrawer.openDrawer(path, 'raw_app')
									}
								}
							]
						: []),
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
	{/snippet}
</Row>
