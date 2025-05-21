<script lang="ts">
	import {
		PostgresTriggerService,
		WorkspaceService,
		type PostgresTrigger,
		type WorkspaceDeployUISettings
	} from '$lib/gen'
	import {
		canWrite,
		displayDate,
		getLocalSetting,
		sendUserToast,
		storeLocalSetting,
		removeTriggerKindIfUnused
	} from '$lib/utils'
	import { base } from '$app/paths'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Alert, Button, Skeleton } from '$lib/components/common'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { enterpriseLicense, usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { Code, Eye, Pen, Plus, Share, Trash, Circle, Database, FileUp } from 'lucide-svelte'
	import { goto } from '$lib/navigation'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import ListFilters from '$lib/components/home/ListFilters.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { setQuery } from '$lib/navigation'
	import { onDestroy, onMount } from 'svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import PostgresTriggerEditor from '$lib/components/triggers/postgres/PostgresTriggerEditor.svelte'
	import { ALL_DEPLOYABLE, isDeployable } from '$lib/utils_deployable'
	import DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'

	type TriggerD = PostgresTrigger & { canWrite: boolean }

	let triggers: TriggerD[] = []
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
		triggers = (
			await PostgresTriggerService.listPostgresTriggers({ workspace: $workspaceStore! })
		).map((x) => {
			return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
		})
		$usedTriggerKinds = removeTriggerKindIfUnused(triggers.length, 'postgres', $usedTriggerKinds)
		loading = false
	}

	let interval = setInterval(async () => {
		try {
			const newTriggers = await PostgresTriggerService.listPostgresTriggers({
				workspace: $workspaceStore!
			})
			for (let i = 0; i < triggers.length; i++) {
				const newTrigger = newTriggers.find((x) => x.path === triggers[i].path)
				if (newTrigger) {
					triggers[i] = {
						...triggers[i],
						error: newTrigger.error,
						last_server_ping: newTrigger.last_server_ping,
						enabled: newTrigger.enabled,
						server_id: newTrigger.server_id
					}
				}
			}
		} catch (err) {
			console.error(err)
		}
	}, 5000)

	onDestroy(() => {
		clearInterval(interval)
	})

	async function setTriggerEnabled(path: string, enabled: boolean): Promise<void> {
		try {
			await PostgresTriggerService.setPostgresTriggerEnabled({
				path,
				workspace: $workspaceStore!,
				requestBody: { enabled }
			})
		} catch (err) {
			sendUserToast(
				`Cannot ` + (enabled ? 'enable' : 'disable') + ` postgres trigger: ${err.body}`,
				true
			)
		} finally {
			loadTriggers()
		}
	}

	$: {
		if ($workspaceStore && $userStore) {
			loadTriggers()
		}
	}
	let postgresTriggerEditor: PostgresTriggerEditor

	let filteredItems: (TriggerD & { marked?: any })[] | undefined = []
	let items: typeof filteredItems | undefined = []
	let preFilteredItems: typeof filteredItems | undefined = []
	let filter = ''
	let ownerFilter: string | undefined = undefined
	let nbDisplayed = 15

	let isDeleting: boolean = false
	let deleteReplicationSlot: boolean = false
	let deletePublication: boolean = false
	let replicationSlotToDelete = ''
	let publicationToDelete = ''
	let deleteReplicationSlotCallback: (() => Promise<string>) | undefined = undefined
	let deletePublicationCallback: (() => Promise<string>) | undefined = undefined
	let deletePostgresTriggerCallback: (() => Promise<string>) | undefined = undefined

	const TRIGGER_PATH_KIND_FILTER_SETTING = 'filter_path_of'
	const FILTER_USER_FOLDER_SETTING_NAME = 'user_and_folders_only'
	let selectedFilterKind =
		(getLocalSetting(TRIGGER_PATH_KIND_FILTER_SETTING) as 'trigger' | 'script_flow') ?? 'trigger'
	let filterUserFolders = getLocalSetting(FILTER_USER_FOLDER_SETTING_NAME) == 'true'

	$: storeLocalSetting(TRIGGER_PATH_KIND_FILTER_SETTING, selectedFilterKind)
	$: storeLocalSetting(FILTER_USER_FOLDER_SETTING_NAME, filterUserFolders ? 'true' : undefined)

	function filterItemsPathsBaseOnUserFilters(
		item: TriggerD,
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
					new Set(filteredItems?.map((x) => x.script_path.split('/').slice(0, 2).join('/')) ?? [])
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

<ConfirmationModal
	open={Boolean(deletePostgresTriggerCallback)}
	title="Delete Postgres trigger"
	confirmationText="Remove"
	loading={isDeleting}
	on:canceled={() => {
		isDeleting = false
		deletePostgresTriggerCallback = undefined
	}}
	on:confirmed={async () => {
		if (deletePostgresTriggerCallback) {
			let msg: string
			isDeleting = true
			if (deleteReplicationSlot && deleteReplicationSlotCallback) {
				try {
					msg = await deleteReplicationSlotCallback()
					sendUserToast(msg)
				} catch (error) {
					isDeleting = false
					sendUserToast(error.body || error.message, true)
					return
				}
			}
			if (deletePublication && deletePublicationCallback) {
				try {
					msg = await deletePublicationCallback()
					sendUserToast(msg)
				} catch (error) {
					isDeleting = false
					sendUserToast(error.body || error.message, true)
					return
				}
			}
			msg = await deletePostgresTriggerCallback()
			sendUserToast(msg)
		}
		deletePostgresTriggerCallback = undefined
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>Are you sure you want to remove this trigger?</span>

		<Toggle
			options={{
				left: `Delete the associated replication slot named: ${replicationSlotToDelete} ?`
			}}
			bind:checked={deleteReplicationSlot}
		/>

		<Toggle
			options={{
				left: `Delete the associated publication named: ${publicationToDelete} ?`
			}}
			bind:checked={deletePublication}
		/>
	</div>
</ConfirmationModal>

<DeployWorkspaceDrawer bind:this={deploymentDrawer} />
<PostgresTriggerEditor onUpdate={loadTriggers} bind:this={postgresTriggerEditor} />

<SearchItems
	{filter}
	items={preFilteredItems}
	bind:filteredItems
	f={(x) => (x.summary ?? '') + ' ' + x.path + ' (' + x.script_path + ')'}
/>

<CenteredPage>
	<PageHeader
		title="Postgres triggers"
		tooltip="Windmill enables real-time responsiveness by listening to specific database transactions—such as inserts, updates, and deletes—and automatically triggering scripts or workflows in response."
	>
		<Button
			size="md"
			startIcon={{ icon: Plus }}
			on:click={() => postgresTriggerEditor.openNew(false)}
		>
			New&nbsp;Postgres trigger
		</Button>
	</PageHeader>

	{#if isCloudHosted()}
		<Alert title="Not compatible with multi-tenant cloud" type="warning">
			Postgres triggers are disabled in the multi-tenant cloud.
		</Alert>
		<div class="py-4"></div>
	{/if}
	<div class="w-full h-full flex flex-col">
		<div class="w-full pb-4 pt-6">
			<input
				type="text"
				placeholder="Search Postgres triggers"
				bind:value={filter}
				class="search-item"
			/>
			<div class="flex flex-row items-center gap-2 mt-6">
				<div class="text-sm shrink-0"> Filter by path of </div>
				<ToggleButtonGroup bind:selected={selectedFilterKind} let:item>
					<ToggleButton small value="trigger" label="Postgres trigger" icon={Database} {item} />
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
			<div class="text-center text-sm text-tertiary mt-2"> No postgres triggers </div>
		{:else if items?.length}
			<div class="border rounded-md divide-y">
				{#each items.slice(0, nbDisplayed) as { postgres_resource_path, publication_name, replication_slot_name, path, edited_by, error, edited_at, script_path, is_flow, extra_perms, canWrite, enabled, server_id } (path)}
					{@const href = `${is_flow ? '/flows/get' : '/scripts/get'}/${script_path}`}
					{@const ping = new Date()}
					{@const pinging = ping && ping.getTime() > new Date().getTime() - 15 * 1000}

					<div
						class="hover:bg-surface-hover w-full items-center px-4 py-2 gap-4 first-of-type:!border-t-0
				first-of-type:rounded-t-md last-of-type:rounded-b-md flex flex-col"
					>
						<div class="w-full flex gap-5 items-center">
							<RowIcon kind={is_flow ? 'flow' : 'script'} />

							<a
								href="#{path}"
								on:click={() => postgresTriggerEditor?.openEdit(path, is_flow)}
								class="min-w-0 grow hover:underline decoration-gray-400"
							>
								<div class="text-primary flex-wrap text-left text-md font-semibold mb-1 truncate">
									{path}
								</div>
								<div class="text-secondary text-xs truncate text-left font-light">
									{postgres_resource_path}
								</div>
								<div class="text-secondary text-xs truncate text-left font-light">
									runnable: {script_path}
								</div>
							</a>

							<div class="hidden lg:flex flex-row gap-1 items-center">
								<SharedBadge {canWrite} extraPerms={extra_perms} />
							</div>

							<div class="w-10">
								{#if (enabled && (!pinging || error)) || (!enabled && error) || (enabled && !server_id)}
									<Popover notClickable>
										<span class="flex h-4 w-4">
											<Circle
												class="text-red-600 animate-ping absolute inline-flex fill-current"
												size={12}
											/>
											<Circle class="text-red-600 relative inline-flex fill-current" size={12} />
										</span>
										<div slot="text">
											{#if enabled}
												{#if !server_id}
													Postgres trigger is starting...
												{:else}
													Could not connect to database{error ? ': ' + error : ''}
												{/if}
											{:else}
												Disabled because of an error: {error}
											{/if}
										</div>
									</Popover>
								{:else if enabled}
									<Popover notClickable>
										<span class="flex h-4 w-4">
											<Circle class="text-green-600 relative inline-flex fill-current" size={12} />
										</span>
										<div slot="text">
											Connected to database{!server_id ? ' (shutting down...)' : ''}</div
										>
									</Popover>
								{/if}
							</div>

							<Toggle
								checked={enabled}
								disabled={!canWrite}
								on:change={(e) => {
									setTriggerEnabled(path, e.detail)
								}}
							/>

							<div class="flex gap-2 items-center justify-end">
								<Button
									on:click={() => postgresTriggerEditor?.openEdit(path, is_flow)}
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
											disabled: !canWrite,
											action: async () => {
												publicationToDelete = publication_name
												replicationSlotToDelete = replication_slot_name
												isDeleting = false
												deletePublication = false
												deleteReplicationSlot = false
												deletePublicationCallback = async () => {
													return await PostgresTriggerService.deletePostgresPublication({
														workspace: $workspaceStore!,
														path: postgres_resource_path,
														publication: publication_name
													})
												}

												deleteReplicationSlotCallback = async () => {
													return await PostgresTriggerService.deletePostgresReplicationSlot({
														workspace: $workspaceStore!,
														path: postgres_resource_path,
														requestBody: {
															name: replication_slot_name
														}
													})
												}

												deletePostgresTriggerCallback = async () => {
													const msg = await PostgresTriggerService.deletePostgresTrigger({
														workspace: $workspaceStore ?? '',
														path
													})
													loadTriggers()
													return msg
												}
											}
										},
										{
											displayName: canWrite ? 'Edit' : 'View',
											icon: canWrite ? Pen : Eye,
											action: () => {
												postgresTriggerEditor?.openEdit(path, is_flow)
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
																	kind: 'postgres'
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
												shareModal.openDrawer(path, 'postgres_trigger')
											}
										}
									]}
								/>
							</div>
						</div>
						<div class="w-full flex justify-between items-baseline">
							<div
								class="flex flex-wrap text-[0.7em] text-tertiary gap-1 items-center justify-end truncate pr-2"
								><div class="truncate">edited by {edited_by}</div><div class="truncate"
									>the {displayDate(edited_at)}</div
								></div
							></div
						>
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

<ShareModal
	bind:this={shareModal}
	on:change={() => {
		loadTriggers()
	}}
/>
