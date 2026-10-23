<script lang="ts">
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import type ShareModal from '$lib/components/ShareModal.svelte'
	import { type ListableRawApp } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import Row from './Row.svelte'
	import type DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import { Globe, UserPlus } from 'lucide-svelte'
	import { isDeployable } from '$lib/utils_deployable'
	import { cachedDeployUiSettings, warmDeployUiSettings } from '$lib/components/home/deploy_ui'

	interface Props {
		app: ListableRawApp & { canWrite: boolean }
		marked: string | undefined
		shareModal: ShareModal
		deploymentDrawer: DeployWorkspaceDrawer
		depth?: number
		menuOpen?: boolean
		keyboardSelected?: boolean
	}

	let {
		app,
		marked,
		shareModal,
		deploymentDrawer,
		depth = 0,
		menuOpen = $bindable(false),
		keyboardSelected = false
	}: Props = $props()

	warmDeployUiSettings()
</script>

<Row
	href="/apps_raw/get/{app.path}"
	kind="raw_app"
	{keyboardSelected}
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
			size="sm"
			items={() => {
				let { path } = app

				return [
					...(isDeployable('app', path, cachedDeployUiSettings())
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
						displayName: 'Share',
						icon: UserPlus,
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
