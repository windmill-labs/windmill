<script lang="ts">
	import {
		HttpTriggerService,
		WorkspaceService,
		type HttpTrigger,
		type WorkspaceDeployUISettings
	} from '$lib/gen'
	import {
		canWrite,
		copyToClipboard,
		displayDate,
		getLocalSetting,
		storeLocalSetting,
		removeTriggerKindIfUnused
	} from '$lib/utils'
	import { base } from '$app/paths'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Button, Skeleton } from '$lib/components/common'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import {
		userStore,
		workspaceStore,
		userWorkspaces,
		enterpriseLicense,
		usedTriggerKinds
	} from '$lib/stores'
	import { Route, Code, Eye, Pen, Plus, Share, Trash, FileUp, ClipboardCopy } from 'lucide-svelte'
	import { goto } from '$lib/navigation'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import ListFilters from '$lib/components/home/ListFilters.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { setQuery } from '$lib/navigation'
	import { onMount } from 'svelte'
	import RouteEditor from '$lib/components/triggers/http/RouteEditor.svelte'
	import DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import { ALL_DEPLOYABLE, isDeployable } from '$lib/utils_deployable'
	import { isCloudHosted } from '$lib/cloud'
	import { getHttpRoute } from '$lib/components/triggers/http/utils'

	type TriggerW = HttpTrigger & { canWrite: boolean }

	let triggers: TriggerW[] = []
	let shareModal: ShareModal
	let loading = true
	let deploymentDrawer: DeployWorkspaceDrawer
	let deployUiSettings: WorkspaceDeployUISettings | undefined = undefined

	async function getDeployUiSettings() {
		if (!$enterpriseLicense) {
			deployUiSettings = ALL_DEPLOYABLE
			return
		}
		let settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
		deployUiSettings = settings.deploy_ui ?? ALL_DEPLOYABLE
	}
	getDeployUiSettings()
	async function loadTriggers(): Promise<void> {
		triggers = (await HttpTriggerService.listHttpTriggers({ workspace: $workspaceStore! })).map(
			(x) => {
				return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
			}
		)
		$usedTriggerKinds = removeTriggerKindIfUnused(triggers.length, 'routes', $usedTriggerKinds)
		loading = false
	}

	$: {
		if ($workspaceStore && $userStore) {
			loadTriggers()
		}
	}
	let routeEditor: RouteEditor

	let filteredItems: (TriggerW & { marked?: any })[] | undefined = []
	let items: typeof filteredItems | undefined = []
	let preFilteredItems: typeof filteredItems | undefined = []
	let filter = ''
	let ownerFilter: string | undefined = undefined
	let nbDisplayed = 15

	const TRIGGER_PATH_KIND_FILTER_SETTING = 'filter_path_of'
	const FILTER_USER_FOLDER_SETTING_NAME = 'user_and_folders_only'
	let selectedFilterKind =
		(getLocalSetting(TRIGGER_PATH_KIND_FILTER_SETTING) as 'trigger' | 'script_flow') ?? 'trigger'
	let filterUserFolders = getLocalSetting(FILTER_USER_FOLDER_SETTING_NAME) == 'true'

	$: storeLocalSetting(TRIGGER_PATH_KIND_FILTER_SETTING, selectedFilterKind)
	$: storeLocalSetting(FILTER_USER_FOLDER_SETTING_NAME, filterUserFolders ? 'true' : undefined)

	function filterItemsPathsBaseOnUserFilters(
		item: TriggerW,
		selectedFilterKind: 'trigger' | 'script_flow',
		filterUserFolders: boolean
	) {
		if ($workspaceStore == 'admins') return true
		if (filterUserFolders) {
			if (selectedFilterKind === 'trigger') {
				return (
					!item.path.startsWith('u/') || item.path.startsWith('u/' + $userStore?.username + '/')
				)
			} else {
				return (
					!item.script_path.startsWith('u/') ||
					item.script_path.startsWith('u/' + $userStore?.username + '/')
				)
			}
		} else {
			return true
		}
	}

	$: preFilteredItems =
		ownerFilter != undefined
			? selectedFilterKind === 'trigger'
				? triggers?.filter(
						(x) =>
							x.path.startsWith(ownerFilter + '/') &&
							filterItemsPathsBaseOnUserFilters(x, selectedFilterKind, filterUserFolders)
					)
				: triggers?.filter(
						(x) =>
							x.script_path.startsWith(ownerFilter + '/') &&
							filterItemsPathsBaseOnUserFilters(x, selectedFilterKind, filterUserFolders)
					)
			: triggers?.filter((x) =>
					filterItemsPathsBaseOnUserFilters(x, selectedFilterKind, filterUserFolders)
				)

	$: if ($workspaceStore) {
		ownerFilter = undefined
	}

	$: owners =
		selectedFilterKind === 'trigger'
			? Array.from(
					new Set(filteredItems?.map((x) => x.path.split('/').slice(0, 2).join('/')) ?? [])
				).sort()
			: Array.from(
					new Set(
						filteredItems
							?.filter((x) => x.script_path)
							.map((x) => x.script_path.split('/').slice(0, 2).join('/')) ?? []
					)
				).sort()

	$: items = filter !== '' ? filteredItems : preFilteredItems

	function updateQueryFilters(selectedFilterKind, filterUserFolders) {
		setQuery(
			new URL(window.location.href),
			TRIGGER_PATH_KIND_FILTER_SETTING,
			selectedFilterKind
		).then(() => {
			setQuery(
				new URL(window.location.href),
				FILTER_USER_FOLDER_SETTING_NAME,
				String(filterUserFolders)
			)
		})
	}

	function loadQueryFilters() {
		let url = new URL(window.location.href)
		let queryFilterKind = url.searchParams.get(TRIGGER_PATH_KIND_FILTER_SETTING)
		let queryFilterUserFolders = url.searchParams.get(FILTER_USER_FOLDER_SETTING_NAME)
		if (queryFilterKind) {
			selectedFilterKind = queryFilterKind as 'trigger' | 'script_flow'
		}
		if (queryFilterUserFolders) {
			filterUserFolders = queryFilterUserFolders == 'true'
		}
	}

	onMount(() => {
		loadQueryFilters()
	})

	$: updateQueryFilters(selectedFilterKind, filterUserFolders)
</script>

<DeployWorkspaceDrawer bind:this={deploymentDrawer} />
<RouteEditor onUpdate={loadTriggers} bind:this={routeEditor} />

<SearchItems
	{filter}
	items={preFilteredItems}
	bind:filteredItems
	f={(x) => (x.summary ?? '') + ' ' + x.path + ' (' + x.script_path + ')'}
/>

{#if $userStore?.operator && $workspaceStore && !$userWorkspaces.find((_) => _.id === $workspaceStore)?.operator_settings?.triggers}
	<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4 m-4 mt-12" role="alert">
		<p class="font-bold">Unauthorized</p>
		<p>Page not available for operators</p>
	</div>
{:else}
	<CenteredPage>
		<PageHeader
			title="Custom HTTP routes"
			tooltip="Every script and flow already has a canonical HTTP API endpoint/webhook attached to it, this is to create additional parametrizable ones."
			documentationLink="https://www.windmill.dev/docs/core_concepts/http_routing"
		>
			{#if $userStore?.is_admin || $userStore?.is_super_admin}
				<Button size="md" startIcon={{ icon: Plus }} on:click={() => routeEditor.openNew(false)}>
					New&nbsp;route
				</Button>
			{/if}
		</PageHeader>
		<div class="w-full h-full flex flex-col">
			<div class="w-full pb-4 pt-6">
				<input type="text" placeholder="Search routes" bind:value={filter} class="search-item" />
				<div class="flex flex-row items-center gap-2 mt-6">
					<div class="text-sm shrink-0"> Filter by path of </div>
					<ToggleButtonGroup bind:selected={selectedFilterKind} let:item>
						<ToggleButton small value="trigger" label="Route" icon={Route} {item} />
						<ToggleButton small value="script_flow" label="Script/Flow" icon={Code} {item} />
					</ToggleButtonGroup>
				</div>
				<ListFilters syncQuery bind:selectedFilter={ownerFilter} filters={owners} />

				<div class="flex flex-row items-center justify-end gap-4">
					{#if $userStore?.is_super_admin && $userStore.username.includes('@')}
						<Toggle size="xs" bind:checked={filterUserFolders} options={{ right: 'Only f/*' }} />
					{:else if $userStore?.is_admin || $userStore?.is_super_admin}
						<Toggle
							size="xs"
							bind:checked={filterUserFolders}
							options={{ right: `Only u/${$userStore.username} and f/*` }}
						/>
					{/if}
				</div>
			</div>
			{#if loading}
				{#each new Array(6) as _}
					<Skeleton layout={[[6], 0.4]} />
				{/each}
			{:else if !triggers?.length}
				<div class="text-center text-sm text-tertiary mt-2"> No routes </div>
			{:else if items?.length}
				<div class="border rounded-md divide-y">
					{#each items.slice(0, nbDisplayed) as { workspace_id, workspaced_route, path, edited_by, edited_at, script_path, route_path, is_flow, extra_perms, canWrite, marked, http_method, static_asset_config } (path)}
						{@const href = `${is_flow ? '/flows/get' : '/scripts/get'}/${script_path}`}

						<div
							class="hover:bg-surface-hover w-full items-center px-4 py-2 gap-4 first-of-type:!border-t-0
				first-of-type:rounded-t-md last-of-type:rounded-b-md flex flex-col"
						>
							<div class="w-full flex gap-5 items-center">
								<RowIcon kind={is_flow ? 'flow' : 'script'} />

								<a
									href="#{path}"
									on:click={() => routeEditor?.openEdit(path, is_flow)}
									class="min-w-0 grow hover:underline decoration-gray-400"
								>
									<div class="text-primary flex-wrap text-left text-md font-semibold mb-1 truncate">
										{#if marked}
											<span class="text-xs">
												{@html marked}
											</span>
										{:else}
											{http_method.toUpperCase()}
											{isCloudHosted() || workspaced_route
												? workspace_id + '/' + route_path
												: route_path}
										{/if}
									</div>
									<div class="text-secondary text-xs truncate text-left font-light">
										{path}
									</div>
									<div class="text-secondary text-xs truncate text-left font-light">
										{#if static_asset_config}
											file: {static_asset_config.s3}
										{:else}
											runnable: {script_path}
										{/if}
									</div>
								</a>

								<div class="hidden lg:flex flex-row gap-1 items-center">
									<SharedBadge {canWrite} extraPerms={extra_perms} />
								</div>

								<div class="flex gap-2 items-center justify-end">
									<Button
										on:click={() =>
											copyToClipboard(
												getHttpRoute('r', route_path, workspaced_route ?? false, workspace_id)
											)}
										color="dark"
										size="xs"
										startIcon={{ icon: ClipboardCopy }}
									>
										Copy URL
									</Button>
									<Button
										on:click={() => routeEditor?.openEdit(path, is_flow)}
										size="xs"
										startIcon={canWrite
											? { icon: Pen }
											: {
													icon: Eye
												}}
										color="dark"
									>
										{canWrite ? 'Edit' : 'View'}
									</Button>
									<Dropdown
										items={[
											{
												displayName: `View ${is_flow ? 'Flow' : 'Script'}`,
												icon: Eye,
												action: () => {
													goto(href)
												}
											},
											{
												displayName: 'Delete',
												type: 'delete',
												icon: Trash,
												disabled:
													!canWrite || !($userStore?.is_admin || $userStore?.is_super_admin),
												action: async () => {
													await HttpTriggerService.deleteHttpTrigger({
														workspace: $workspaceStore ?? '',
														path
													})
													loadTriggers()
												}
											},
											{
												displayName: canWrite ? 'Edit' : 'View',
												icon: canWrite ? Pen : Eye,
												action: () => {
													routeEditor?.openEdit(path, is_flow)
												}
											},
											...(isDeployable('trigger', path, deployUiSettings)
												? [
														{
															displayName: 'Deploy to prod/staging',
															icon: FileUp,
															action: () => {
																deploymentDrawer.openDrawer(path, 'trigger', {
																	triggers: {
																		kind: 'routes'
																	}
																})
															}
														}
													]
												: []),
											{
												displayName: 'Audit logs',
												icon: Eye,
												href: `${base}/audit_logs?resource=${path}`
											},
											{
												displayName: canWrite ? 'Share' : 'See Permissions',
												icon: Share,
												action: () => {
													shareModal.openDrawer(path, 'http_trigger')
												}
											}
										]}
									/>
								</div>
							</div>
							<div class="w-full flex justify-between items-baseline">
								<div
									class="flex flex-wrap text-[0.7em] text-tertiary gap-1 items-center justify-end truncate pr-2"
								>
									<div class="truncate">edited by {edited_by}</div>
									<div class="truncate">at {displayDate(edited_at)}</div>
								</div>
							</div>
						</div>
					{/each}
				</div>
			{:else}
				<NoItemFound />
			{/if}
		</div>
		{#if items && items?.length > 15 && nbDisplayed < items.length}
			<span class="text-xs"
				>{nbDisplayed} items out of {items.length}
				<button class="ml-4" on:click={() => (nbDisplayed += 30)}>load 30 more</button></span
			>
		{/if}
	</CenteredPage>
{/if}

<ShareModal
	bind:this={shareModal}
	on:change={() => {
		loadTriggers()
	}}
/>
