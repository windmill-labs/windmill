<script lang="ts">
	import { run } from 'svelte/legacy'

	import {
		GcpTriggerService,
		WorkspaceService,
		type GcpTrigger,
		type WorkspaceDeployUISettings
	} from '$lib/gen'
	import {
		canWrite,
		copyToClipboard,
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
	import { Code, Eye, Pen, Plus, Share, Trash, Circle, FileUp, ClipboardCopy } from 'lucide-svelte'
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
	import { ALL_DEPLOYABLE, isDeployable } from '$lib/utils_deployable'
	import DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import GcpTriggerEditor from '$lib/components/triggers/gcp/GcpTriggerEditor.svelte'
	import { GoogleCloudIcon } from '$lib/components/icons'
	import { getHttpRoute } from '$lib/components/triggers/http/utils'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'

	type TriggerD = GcpTrigger & { canWrite: boolean }

	let triggers: TriggerD[] = $state([])
	let shareModal: ShareModal | undefined = $state()
	let loading = $state(true)
	let deploymentDrawer: DeployWorkspaceDrawer | undefined = $state()
	let deployUiSettings: WorkspaceDeployUISettings | undefined = $state(undefined)
	let isDeleting = $state(false)
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
		triggers = (await GcpTriggerService.listGcpTriggers({ workspace: $workspaceStore! })).map(
			(x) => {
				return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
			}
		)
		$usedTriggerKinds = removeTriggerKindIfUnused(triggers.length, 'gcp', $usedTriggerKinds)
		loading = false
	}

	let interval = setInterval(async () => {
		try {
			const newTriggers = await GcpTriggerService.listGcpTriggers({
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
			await GcpTriggerService.setGcpTriggerEnabled({
				path,
				workspace: $workspaceStore!,
				requestBody: { enabled }
			})
		} catch (err) {
			sendUserToast(
				`Cannot ` + (enabled ? 'enable' : 'disable') + ` GCP Pub/Sub trigger: ${err.body}`,
				true
			)
		} finally {
			loadTriggers()
		}
	}

	run(() => {
		if ($workspaceStore && $userStore) {
			loadTriggers()
		}
	})
	let gcpTriggerEditor: GcpTriggerEditor | undefined = $state()

	let filteredItems: (TriggerD & { marked?: any })[] | undefined = $state([])
	let items: typeof filteredItems | undefined = $state([])
	let preFilteredItems: typeof filteredItems | undefined = $state([])
	let filter = $state('')
	let ownerFilter: string | undefined = $state(undefined)
	let nbDisplayed = $state(15)
	let deleteSubscription = $state(false)
	let deleteSubscriptionCallback: (() => Promise<void>) | undefined = $state(undefined)
	let deleteGcpTriggerCallback: (() => Promise<void>) | undefined = $state(undefined)
	let subscriptionToDelete: string = $state('')
	let currentTopic: string = $state('')
	const TRIGGER_PATH_KIND_FILTER_SETTING = 'filter_path_of'
	const FILTER_USER_FOLDER_SETTING_NAME = 'user_and_folders_only'
	let selectedFilterKind = $state(
		(getLocalSetting(TRIGGER_PATH_KIND_FILTER_SETTING) as 'trigger' | 'script_flow') ?? 'trigger'
	)
	let filterUserFolders = $state(getLocalSetting(FILTER_USER_FOLDER_SETTING_NAME) == 'true')

	run(() => {
		storeLocalSetting(TRIGGER_PATH_KIND_FILTER_SETTING, selectedFilterKind)
	})
	run(() => {
		storeLocalSetting(FILTER_USER_FOLDER_SETTING_NAME, filterUserFolders ? 'true' : undefined)
	})

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

	run(() => {
		preFilteredItems =
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
	})

	run(() => {
		if ($workspaceStore) {
			ownerFilter = undefined
		}
	})

	let owners = $derived(
		selectedFilterKind === 'trigger'
			? Array.from(
					new Set(filteredItems?.map((x) => x.path.split('/').slice(0, 2).join('/')) ?? [])
				).sort()
			: Array.from(
					new Set(filteredItems?.map((x) => x.script_path.split('/').slice(0, 2).join('/')) ?? [])
				).sort()
	)

	run(() => {
		items = filter !== '' ? filteredItems : preFilteredItems
	})

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

	run(() => {
		updateQueryFilters(selectedFilterKind, filterUserFolders)
	})
</script>

<ConfirmationModal
	open={Boolean(deleteGcpTriggerCallback)}
	title="Delete GCP Pub/Sub trigger"
	confirmationText="Remove"
	loading={isDeleting}
	on:canceled={() => {
		deleteGcpTriggerCallback = undefined
	}}
	on:confirmed={async () => {
		if (deleteGcpTriggerCallback) {
			isDeleting = true
			if (deleteSubscription && deleteSubscriptionCallback) {
				try {
					await deleteSubscriptionCallback()
				} catch (error) {
					isDeleting = false
					sendUserToast(error.body || error.message, true)
					return
				}
			}
			await deleteGcpTriggerCallback()
		}
		deleteGcpTriggerCallback = undefined
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>Are you sure you want to remove this trigger?</span>

		<Toggle
			options={{
				left: `Delete subscription "${subscriptionToDelete}" subscribed to topic "${currentTopic}"?`
			}}
			bind:checked={deleteSubscription}
		/>
	</div>
</ConfirmationModal>

<DeployWorkspaceDrawer bind:this={deploymentDrawer} />
<GcpTriggerEditor onUpdate={loadTriggers} bind:this={gcpTriggerEditor} />

<SearchItems
	{filter}
	items={preFilteredItems}
	bind:filteredItems
	f={(x) => (x.summary ?? '') + ' ' + x.path + ' (' + x.script_path + ')'}
/>

<CenteredPage>
	<PageHeader title="GCP Pub/Sub triggers" tooltip="GCP Pub/Sub trigger">
		<Button size="md" startIcon={{ icon: Plus }} on:click={() => gcpTriggerEditor?.openNew(false)}>
			New&nbsp;GCP Pub/Sub trigger
		</Button>
	</PageHeader>

	{#if isCloudHosted()}
		<Alert title="Not compatible with multi-tenant cloud" type="warning">
			GCP Pub/Sub triggers are disabled in the multi-tenant cloud.
		</Alert>
	{/if}
	<div class="w-full h-full flex flex-col">
		<div class="w-full pb-4 pt-6">
			<input type="text" placeholder="Search triggers" bind:value={filter} class="search-item" />
			<div class="flex flex-row items-center gap-2 mt-6">
				<div class="text-sm shrink-0"> Filter by path of </div>
				<ToggleButtonGroup bind:selected={selectedFilterKind}>
					{#snippet children({ item })}
						<ToggleButton
							small
							value="trigger"
							label="GCP Pub/Sub trigger"
							icon={GoogleCloudIcon}
							{item}
						/>
						<ToggleButton small value="script_flow" label="Script/Flow" icon={Code} {item} />
					{/snippet}
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
			<div class="text-center text-sm text-tertiary mt-2"> No GCP Pub/Sub triggers </div>
		{:else if items?.length}
			<div class="border rounded-md divide-y">
				{#each items.slice(0, nbDisplayed) as { gcp_resource_path, topic_id, workspace_id, delivery_type, path, edited_by, error, edited_at, script_path, is_flow, extra_perms, canWrite, enabled, server_id, subscription_id } (path)}
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
								onclick={() => gcpTriggerEditor?.openEdit(path, is_flow)}
								class="min-w-0 grow hover:underline decoration-gray-400"
							>
								<div class="text-primary flex-wrap text-left text-md font-semibold mb-1 truncate">
									{path} - {topic_id}
								</div>
								<div class="text-secondary text-xs truncate text-left font-light">
									runnable: {script_path}
								</div>
							</a>

							<div class="hidden lg:flex flex-row gap-1 items-center">
								<SharedBadge {canWrite} extraPerms={extra_perms} />
							</div>

							{#if delivery_type !== 'push'}
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
											{#snippet text()}
												<div>
													{#if enabled}
														{#if !server_id}
															GCP Pub/Sub trigger is starting...
														{:else}
															Could not connect to GCP Pub/Sub{error ? ': ' + error : ''}
														{/if}
													{:else}
														Disabled because of an error: {error}
													{/if}
												</div>
											{/snippet}
										</Popover>
									{:else if enabled}
										<Popover notClickable>
											<span class="flex h-4 w-4">
												<Circle
													class="text-green-600 relative inline-flex fill-current"
													size={12}
												/>
											</span>
											{#snippet text()}
												<div>
													Connected to GCP Pub/Sub{!server_id ? ' (shutting down...)' : ''}</div
												>
											{/snippet}
										</Popover>
									{/if}
								</div>
							{/if}

							{#if delivery_type !== 'push'}
								<Toggle
									checked={enabled}
									disabled={!canWrite}
									on:change={(e) => {
										setTriggerEnabled(path, e.detail)
									}}
								/>
							{/if}

							<div class="flex gap-2 items-center justify-end">
								{#if delivery_type === 'push'}
									<Button
										on:click={() =>
											copyToClipboard(getHttpRoute('gcp/w', path, true, workspace_id))}
										color="dark"
										size="xs"
										startIcon={{ icon: ClipboardCopy }}
									>
										Copy URL
									</Button>
								{/if}
								<Button
									on:click={() => gcpTriggerEditor?.openEdit(path, is_flow)}
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
												isDeleting = false
												deleteSubscription = false
												subscriptionToDelete = subscription_id
												currentTopic = topic_id
												deleteSubscriptionCallback = async () => {
													const message = await GcpTriggerService.deleteGcpSubscription({
														workspace: $workspaceStore ?? '',
														path: gcp_resource_path,
														requestBody: {
															subscription_id
														}
													})
													sendUserToast(message)
												}
												deleteGcpTriggerCallback = async () => {
													const message = await GcpTriggerService.deleteGcpTrigger({
														workspace: $workspaceStore ?? '',
														path
													})
													sendUserToast(message)
													loadTriggers()
												}
											}
										},
										{
											displayName: canWrite ? 'Edit' : 'View',
											icon: canWrite ? Pen : Eye,
											action: () => {
												gcpTriggerEditor?.openEdit(path, is_flow)
											}
										},
										...(isDeployable('trigger', path, deployUiSettings)
											? [
													{
														displayName: 'Deploy to prod/staging',
														icon: FileUp,
														action: () => {
															deploymentDrawer?.openDrawer(path, 'trigger', {
																triggers: {
																	kind: 'gcp'
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
												shareModal?.openDrawer(path, 'gcp_trigger')
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
			<button class="ml-4" onclick={() => (nbDisplayed += 30)}>load 30 more</button></span
		>
	{/if}
</CenteredPage>

<ShareModal
	bind:this={shareModal}
	on:change={() => {
		loadTriggers()
	}}
/>
