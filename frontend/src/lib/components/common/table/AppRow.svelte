<script lang="ts">
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import type MoveDrawer from '$lib/components/MoveDrawer.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import type ShareModal from '$lib/components/ShareModal.svelte'
	import { AppService, AppWithLastVersion, DraftService, type ListableApp } from '$lib/gen'
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
		FileUp,
		GitFork,
		Globe,
		History,
		Pen,
		Share,
		Trash
	} from 'lucide-svelte'
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import type DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import { DELETE } from '$lib/utils'
	import AppDeploymentHistory from '$lib/components/apps/editor/AppDeploymentHistory.svelte'
	import AppJsonEditor from '$lib/components/apps/editor/AppJsonEditor.svelte'

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

	let appExport: AppJsonEditor
	let appDeploymentHistory: AppDeploymentHistory

	async function loadAppJson() {
		appExport.open(app.path)
	}

	async function loadDeployements() {
		const napp: AppWithLastVersion = (await AppService.getAppByPath({
			workspace: $workspaceStore!,
			path: app.path
		})) as unknown as AppWithLastVersion

		appDeploymentHistory.open(napp.path)
	}
</script>

{#if menuOpen}
	<AppJsonEditor on:change bind:this={appExport} />
	<AppDeploymentHistory bind:this={appDeploymentHistory} />
{/if}

<Row
	href={`/apps/get/${app.path}`}
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
		<SharedBadge canWrite={app.canWrite} extraPerms={app.extra_perms} />
		<DraftBadge has_draft={app.has_draft} draft_only={app.draft_only} />
		<div class="w-8 center-center" />
	</svelte:fragment>
	<svelte:fragment slot="actions">
		<span class="hidden md:inline-flex gap-x-1">
			{#if !$userStore?.operator}
				{#if app.canWrite}
					<div>
						<Button
							color="light"
							size="xs"
							variant="border"
							startIcon={{ icon: Pen }}
							href="/apps/edit/{app.path}?nodraft=true"
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
							startIcon={{ icon: GitFork }}
							href="/apps/add?template={app.path}"
						>
							Fork
						</Button>
					</div>
				{/if}
			{/if}
		</span>
		<Dropdown
			items={() => {
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
							disabled: !canWrite
						},
						{
							displayName: 'View/Edit JSON',
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
						href: `/apps/add?template=${path}`
					},
					{
						displayName: 'Move/Rename',
						icon: FileUp,
						action: () => {
							moveDrawer.openDrawer(path, summary, 'app')
						},
						disabled: !canWrite
					},
					{
						displayName: 'Deploy to staging/prod',
						icon: Globe,
						action: () => {
							deploymentDrawer.openDrawer(path, 'app')
						}
					},
					{
						displayName: 'View/Edit JSON',
						icon: FileJson,
						action: () => {
							loadAppJson()
						}
					},
					{
						displayName: 'Deployments',
						icon: History,
						action: () => loadDeployements()
					},
					{
						displayName: canWrite ? 'Share' : 'See Permissions',
						icon: Share,
						action: () => {
							shareModal.openDrawer && shareModal.openDrawer(path, 'app')
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
										goto(url)
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
									disabled: !canWrite
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
