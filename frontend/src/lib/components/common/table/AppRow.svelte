<script lang="ts">
	import { base } from '$lib/base'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import type MoveDrawer from '$lib/components/MoveDrawer.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import DraftBadge from '$lib/components/DraftBadge.svelte'
	import type ShareModal from '$lib/components/ShareModal.svelte'
	import { AppService, type ListableApp } from '$lib/gen'
	import { userStore, userWorkspaces, workspaceStore } from '$lib/stores'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import { createEventDispatcher } from 'svelte'
	import Button from '../button/Button.svelte'
	import Row from './Row.svelte'
	import InheritedLabels from '$lib/components/InheritedLabels.svelte'
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
		Shield,
		Trash,
		Copy
	} from 'lucide-svelte'
	import { goto as gotoUrl } from '$app/navigation'
	import { page } from '$app/state'
	import type DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import { copyToClipboard } from '$lib/utils'
	import AppDeploymentHistory from '$lib/components/apps/editor/AppDeploymentHistory.svelte'
	import { isDeployable } from '$lib/utils_deployable'
	import { getDeployUiSettings } from '$lib/components/home/deploy_ui'
	import { buildForkEditUrl, editInForkAllowed, editInForkLabel } from '$lib/utils/editInFork'
	import { isCloudHosted } from '$lib/cloud'

	interface Props {
		app: ListableApp & { draft_only?: boolean; canWrite: boolean }
		marked: string | undefined
		shareModal: ShareModal
		moveDrawer: MoveDrawer
		deploymentDrawer: DeployWorkspaceDrawer
		deleteConfirmedCallback: (() => void) | undefined
		depth?: number
		menuOpen?: boolean
		showEditButton?: boolean
		keyboardSelected?: boolean
	}

	let {
		app,
		marked,
		shareModal,
		moveDrawer,
		deploymentDrawer,
		deleteConfirmedCallback = $bindable(),
		depth = 0,
		menuOpen = $bindable(false),
		showEditButton = $bindable(true),
		keyboardSelected = false
	}: Props = $props()

	const dispatch = createEventDispatcher()

	let appExport: { open: (path: string, rawApp?: boolean) => void } | undefined = $state(undefined)
	let appDeploymentHistory: AppDeploymentHistory | undefined = $state(undefined)

	async function loadAppJson() {
		// Thread the row's `raw_app` flag so the JSON drawer's backend
		// fetch picks the right draft kind on draft-only items (no
		// deployed row to read the kind from server-side).
		appExport?.open(app.path, !!app.raw_app)
	}

	async function deleteApp(path: string): Promise<void> {
		// Draft-only items have no deployed row — the regular route would
		// 404. Route the delete through the syncer instead; the `app` vs
		// `raw_app` choice mirrors the row's own `raw_app` flag.
		if (app.draft_only) {
			await UserDraftDbSyncer.save({
				workspace: $workspaceStore ?? '',
				itemKind: app.raw_app ? 'raw_app' : 'app',
				path,
				value: null,
				immediate: true
			})
		} else {
			await AppService.deleteApp({ workspace: $workspaceStore ?? '', path })
		}
	}
</script>

{#if menuOpen}
	{#await import('$lib/components/apps/editor/AppJsonEditor.svelte') then Module}
		<Module.default on:change bind:this={appExport} />
	{/await}
	<AppDeploymentHistory bind:this={appDeploymentHistory} appPath={app.path} />
{/if}

<Row
	href={app.draft_only
		? `${base}/apps${app.raw_app ? '_raw' : ''}/edit/${app.path}`
		: `${base}/apps${app.raw_app ? '_raw' : ''}/get/${app.path}`}
	kind="app"
	{keyboardSelected}
	{marked}
	path={(app as any).draft_path ?? app.path}
	summary={app.is_draft ? `${app.summary || (app as any).draft_path || app.path}*` : app.summary}
	workspaceId={app.workspace_id ?? $workspaceStore ?? ''}
	canFavorite={!app.draft_only}
	{depth}
>
	{#snippet badges()}
		{#if app.execution_mode == 'anonymous'}
			<Badge small icon={{ icon: Eye }}>Public</Badge>
		{/if}
		{#if app.raw_app}
			<Badge small icon={{ icon: FileJson }}>Raw</Badge>
		{/if}
		<SharedBadge canWrite={app.canWrite} extraPerms={app.extra_perms} />
		<DraftBadge
			is_draft={app.is_draft}
			draft_only={app.draft_only}
			draft_users={app.draft_users}
			currentUsername={$userStore?.username}
			workspace={$workspaceStore ?? undefined}
			itemKind={app.raw_app ? 'raw_app' : 'app'}
			path={app.path}
			onMigrated={() => dispatch('change')}
		/>
		{#if app.labels?.length}
			<div class="flex items-center gap-0.5">
				{#each app.labels.slice(0, 3) as label}
					<Badge color="blue" small class="px-1" title="Label: {label}">{label}</Badge>
				{/each}
				{#if app.labels.length > 3}
					<Badge
						color="blue"
						small
						class="px-1"
						title={app.labels
							.slice(3)
							.map((l) => 'Label: ' + l)
							.join('\n')}>+{app.labels.length - 3}</Badge
					>
				{/if}
			</div>
		{/if}
		<InheritedLabels labels={app.inherited_labels} />
		<div class="w-8 center-center"></div>
	{/snippet}
	{#snippet actions()}
		<span class="hidden md:inline-flex gap-x-1">
			{#if !$userStore?.operator}
				{#if showEditButton && app.canWrite}
					<div>
						<Button
							aiId={`edit-app-button-${app.summary?.length > 0 ? app.summary : app.path}`}
							aiDescription={`Edits the app ${app.summary?.length > 0 ? app.summary : app.path}`}
							variant="subtle"
							wrapperClasses="w-20"
							startIcon={{ icon: Pen }}
							href="{base}/apps{app.raw_app ? '_raw' : ''}/edit/{app.path}"
						>
							Edit
						</Button>
					</div>
				{/if}
				{#if !isCloudHosted() && editInForkAllowed($workspaceStore, $userWorkspaces) && (!showEditButton || !app.canWrite)}
					<div>
						<Button
							variant={!showEditButton ? 'default' : 'subtle'}
							wrapperClasses="w-32"
							unifiedSize="md"
							startIcon={{ icon: GitFork }}
							href={buildForkEditUrl(app.raw_app ? 'raw_app' : 'app', app.path)}
						>
							{editInForkLabel($workspaceStore, $userWorkspaces)}
						</Button>
					</div>
				{/if}
			{/if}
		</span>
		<Dropdown
			aiId={`app-row-dropdown-${app.summary?.length > 0 ? app.summary : app.path}`}
			aiDescription={`Open dropdown for app ${app.summary?.length > 0 ? app.summary : app.path} options`}
			items={async () => {
				let { draft_only, canWrite, summary, execution_mode, path } = app

				const canEdit = canWrite && showEditButton
				if (draft_only) {
					return [
						{
							displayName: 'Delete',
							icon: Trash,
							action: async (event) => {
								// TODO
								// @ts-ignore
								if (event?.shiftKey) {
									await deleteApp(path)
									dispatch('change')
								} else {
									deleteConfirmedCallback = async () => {
										await deleteApp(path)
										dispatch('change')
									}
								}
							},
							type: 'delete',
							// A draft-only row is always the authed user's own draft (the
							// list endpoint only surfaces own/legacy draft-only rows), so
							// discarding it never requires write permission on the path.
							disabled: !showEditButton,
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
						disabled: !showEditButton,
						hide: $userStore?.operator
					},
					{
						displayName: editInForkLabel($workspaceStore, $userWorkspaces),
						icon: GitFork,
						href: buildForkEditUrl(app.raw_app ? 'raw_app' : 'app', path),
						hide:
							$userStore?.operator ||
							isCloudHosted() ||
							!editInForkAllowed($workspaceStore, $userWorkspaces)
					},
					{
						displayName: 'Move/Rename',
						icon: FolderOpen,
						action: () => {
							moveDrawer.openDrawer(path, summary, 'app')
						},
						disabled: !canEdit,
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
						displayName: 'Permissions',
						icon: Shield,
						action: () => {
							shareModal.openDrawer && shareModal.openDrawer(path, 'app')
						},
						hide: $userStore?.operator
					},
					{
						displayName: 'Copy path',
						icon: Copy,
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
											page.url.protocol +
											'//' +
											`${page.url.hostname}/public/${$workspaceStore}/${secretUrl}`
										gotoUrl(url)
									}
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
								await deleteApp(path)
								dispatch('change')
							} else {
								deleteConfirmedCallback = async () => {
									await deleteApp(path)
									dispatch('change')
								}
							}
						},
						type: 'delete',
						disabled: !canEdit,
						hide: $userStore?.operator
					}
				]
			}}
			on:open={() => {
				menuOpen = true
			}}
		/>
	{/snippet}
</Row>
