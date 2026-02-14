<script lang="ts">
	import { base } from '$lib/base'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import type ShareModal from '$lib/components/ShareModal.svelte'
	import { type ListableRawApp } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import Row from './Row.svelte'
	import type DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import { Globe, Share } from 'lucide-svelte'
	import { isDeployable } from '$lib/utils_deployable'
	import { getDeployUiSettings } from '$lib/components/home/deploy_ui'

	interface Props {
		app: ListableRawApp & { canWrite: boolean }
		marked: string | undefined
		shareModal: ShareModal
		deploymentDrawer: DeployWorkspaceDrawer
		depth?: number
		menuOpen?: boolean
	}

	let {
		app,
		marked,
		shareModal,
		deploymentDrawer,
		depth = 0,
		menuOpen = $bindable(false)
	}: Props = $props()
</script>

<Row
	href="{base}/apps/get_raw/{app.version}/{app.path}"
	kind="raw_app"
	{marked}
	path={app.path}
	summary={app.summary}
	workspaceId={app.workspace_id ?? $workspaceStore ?? ''}
	canFavorite={true}
	{depth}
>
	{#snippet badges()}
		<SharedBadge canWrite={app.canWrite} extraPerms={app.extra_perms} />
	{/snippet}
	{#snippet actions()}
		<Dropdown
			items={async () => {
				let { path, canWrite } = app

				return [
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
					}
				]
			}}
			on:open={() => {
				menuOpen = true
			}}
		/>
	{/snippet}
</Row>
