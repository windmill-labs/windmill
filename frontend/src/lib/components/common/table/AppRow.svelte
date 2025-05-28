<script lang="ts">
	import { base } from '$lib/base'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import type MoveDrawer from '$lib/components/MoveDrawer.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import type ShareModal from '$lib/components/ShareModal.svelte'
	import { AppService, DraftService, type ListableApp } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import Button from '../button/Button.svelte'
	import Row from './Row.svelte'
	import DraftBadge from '$lib/components/DraftBadge.svelte'
	import Badge from '../badge/Badge.svelte'
	import {
		ExternalLink,
		Eye,
		File,
		FileJson,
		FolderOpen,
		GitFork,
		ChevronUpSquare,
		History,
		Pen,
		Share,
		Trash,
		Clipboard
	} from 'lucide-svelte'
	import { goto as gotoUrl } from '$app/navigation'
	import { page } from '$app/stores'
	import type DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import { DELETE, copyToClipboard } from '$lib/utils'
	import AppDeploymentHistory from '$lib/components/apps/editor/AppDeploymentHistory.svelte'
	import { isDeployable } from '$lib/utils_deployable'
	import { getDeployUiSettings } from '$lib/components/home/deploy_ui'

	export let app: ListableApp & { has_draft?: boolean; draft_only?: boolean; canWrite: boolean }
	export let marked: string | undefined
	export let starred: boolean
	export let shareModal: ShareModal
	export let moveDrawer: MoveDrawer
	export let deploymentDrawer: DeployWorkspaceDrawer
	export let deleteConfirmedCallback: (() => void) | undefined
	export let depth: number = 0
	export let menuOpen: boolean = false

	const dispatch = createEventDispatcher()

	let appExport: { open: (path: string) => void } | undefined = undefined
	let appDeploymentHistory: AppDeploymentHistory | undefined = undefined

	async function loadAppJson() {
		appExport?.open(app.path)
	}
</script>

{#if menuOpen}
	{#await import('$lib/components/apps/editor/AppJsonEditor.svelte') then Module}
		<Module.default on:change bind:this={appExport} />
	{/await}
	<AppDeploymentHistory bind:this={appDeploymentHistory} appPath={app.path} />
{/if}

<Row
	href="{base}/apps{app.raw_app ? '_raw' : ''}/get/{app.path}"
	kind="app"
	{marked}
	path={app.path}
	summary={app.summary}
	workspaceId={app.workspace_id ?? $workspaceStore ?? ''}
	{starred}
	on:change
	canFavorite={!app.draft_only}
	{depth}
>
	<svelte:fragment slot="badges">
		{#if app.execution_mode == 'anonymous'}
			<Badge small>
				<div class="flex gap-1 items-center">
					<Eye size={14} />
					Public
				</div></Badge
			>
		{/if}
		{#if app.raw_app}
			<Badge small>
				<div class="flex gap-1 items-center">
					<FileJson size={14} />
					Raw
				</div></Badge
			>
		{/if}
		<SharedBadge canWrite={app.canWrite} extraPerms={app.extra_perms} />
		<DraftBadge has_draft={app.has_draft} draft_only={app.draft_only} />
		<div class="w-8 center-center"></div>
	</svelte:fragment>
	<svelte:fragment slot="actions">
		<span class="hidden md:inline-flex gap-x-1">
			{#if !$userStore?.operator}
				{#if app.canWrite}
					<div>
						<Button
							aiId={`edit-app-button-${app.summary?.length > 0 ? app.summary : app.path}`}
							aiDescription={`Edits the app ${app.summary?.length > 0 ? app.summary : app.path}`}
							color="light"
							size="xs"
							variant="border"
							startIcon={{ icon: Pen }}
							href="{base}/apps{app.raw_app ? '_raw' : ''}/edit/{app.path}?nodraft=true"
						>
							Edit
						</Button>
					</div>
				{:else}
					<div>
						<Button
							aiId={`fork-app-button-${app.summary?.length > 0 ? app.summary : app.path}`}
							aiDescription={`Fork the app ${app.summary?.length > 0 ? app.summary : app.path}`}
							color="light"
							size="xs"
							variant="border"
							startIcon={{ icon: GitFork }}
							href="{base}/apps{app.raw_app ? '_raw' : ''}/add?template={app.path}"
						>
							Fork
						</Button>
					</div>
				{/if}
			{/if}
		</span>
		<Dropdown
			aiId={`app-row-dropdown-${app.summary?.length > 0 ? app.summary : app.path}`}
			aiDescription={`Open dropdown for app ${app.summary?.length > 0 ? app.summary : app.path} options`}
			items={async () => {
				let { draft_only, canWrite, summary, execution_mode, path, has_draft } = app

				if (draft_only) {
					return [
						{
							displayName: 'Delete',
							icon: Trash,
							action: async (event) => {
								// TODO
								// @ts-ignore
								if (event?.shiftKey) {
									await AppService.deleteApp({ workspace: $workspaceStore ?? '', path })
									dispatch('change')
								} else {
									deleteConfirmedCallback = async () => {
										await AppService.deleteApp({ workspace: $workspaceStore ?? '', path })
										dispatch('change')
									}
								}
							},
							type: 'delete',
							disabled: !canWrite,
							hide: $userStore?.operator
						},
						{
							displayName: $userStore?.operator ? 'View JSON' : 'View/Edit JSON',
							icon: File,
							action: () => {
								loadAppJson()
							}
						}
					]
				}
				return [
					{
						displayName: 'Duplicate/Fork',
						icon: GitFork,
						href: `${base}/apps${app.raw_app ? '_raw' : ''}/add?template=${path}`,
						hide: $userStore?.operator
					},
					{
						displayName: 'Move/Rename',
						icon: FolderOpen,
						action: () => {
							moveDrawer.openDrawer(path, summary, 'app')
						},
						disabled: !canWrite,
						hide: $userStore?.operator
					},
					...(isDeployable('app', path, await getDeployUiSettings())
						? [
								{
									displayName: 'Deploy to staging/prod',
									icon: ChevronUpSquare,
									action: () => {
										deploymentDrawer.openDrawer(path, 'app')
									},
									hide: $userStore?.operator
								}
							]
						: []),
					{
						displayName: $userStore?.operator ? 'View JSON' : 'View/Edit JSON',
						icon: FileJson,
						action: () => {
							loadAppJson()
						}
					},
					{
						displayName: 'Deployments',
						icon: History,
						action: () => appDeploymentHistory?.open(),
						hide: $userStore?.operator
					},
					{
						displayName: canWrite ? 'Share' : 'See Permissions',
						icon: Share,
						action: () => {
							shareModal.openDrawer && shareModal.openDrawer(path, 'app')
						},
						hide: $userStore?.operator
					},
					{
						displayName: 'Copy path',
						icon: Clipboard,
						action: () => {
							copyToClipboard(path)
						}
					},
					...(execution_mode == 'anonymous'
						? [
								{
									displayName: 'Go to public page',
									icon: ExternalLink,
									action: async () => {
										let secretUrl = await AppService.getPublicSecretOfApp({
											workspace: $workspaceStore ?? '',
											path
										})
										let url =
											$page.url.protocol +
											'//' +
											`${$page.url.hostname}/public/${$workspaceStore}/${secretUrl}`
										gotoUrl(url)
									}
								}
							]
						: []),
					...(has_draft
						? [
								{
									displayName: 'Delete Draft',
									icon: Trash,
									action: async () => {
										await DraftService.deleteDraft({
											workspace: $workspaceStore ?? '',
											path,
											kind: 'app'
										})
										dispatch('change')
									},
									type: DELETE,
									disabled: !canWrite,
									hide: $userStore?.operator
								}
							]
						: []),
					{
						displayName: 'Delete',
						icon: Trash,
						action: async (event) => {
							// TODO
							// @ts-ignore
							if (event?.shiftKey) {
								await AppService.deleteApp({ workspace: $workspaceStore ?? '', path })
								dispatch('change')
							} else {
								deleteConfirmedCallback = async () => {
									await AppService.deleteApp({ workspace: $workspaceStore ?? '', path })
									dispatch('change')
								}
							}
						},
						type: 'delete',
						disabled: !canWrite,
						hide: $userStore?.operator
					}
				]
			}}
			on:open={() => {
				menuOpen = true
			}}
		/>
	</svelte:fragment>
</Row>
