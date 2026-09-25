<script lang="ts">
	import { onDestroy, untrack } from 'svelte'
	import { base } from '$app/paths'
	import { page } from '$app/stores'
	import {
		AzureTriggerService,
		GcpTriggerService,
		PostgresTriggerService,
		SettingService,
		WorkspaceService,
		type TriggerMode,
		type WorkspaceDeployUISettings
	} from '$lib/gen'
	import {
		canWrite,
		capitalize,
		copyToClipboard,
		displayDate,
		getLocalSetting,
		removeTriggerKindIfUnused,
		sendUserToast,
		storeLocalSetting
	} from '$lib/utils'
	import { withForkConflictRetry } from '$lib/utils/forkConflict'
	import { getLocalDraftHint } from '$lib/localDraftHints.svelte'
	import { enterpriseLicense, usedTriggerKinds, userWorkspaces, workspaceStore } from '$lib/stores'
	import { goto, setQuery } from '$lib/navigation'
	import { isCloudHosted } from '$lib/cloud'
	import { ALL_DEPLOYABLE, isDeployable } from '$lib/utils_deployable'
	import {
		Circle,
		ClipboardCopy,
		Code,
		Database,
		Eye,
		FileUp,
		Mail,
		Pause,
		Pen,
		Plus,
		Route,
		Shield,
		Trash,
		Unplug
	} from 'lucide-svelte'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Alert, Badge, Button, EmptyState, Skeleton } from '$lib/components/common'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import DraftBadge from '$lib/components/DraftBadge.svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import ListFilters from '$lib/components/home/ListFilters.svelte'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import AmqpIcon from '$lib/components/icons/AmqpIcon.svelte'
	import AwsIcon from '$lib/components/icons/AwsIcon.svelte'
	import AzureIcon from '$lib/components/icons/AzureIcon.svelte'
	import GoogleCloudIcon from '$lib/components/icons/GoogleCloudIcon.svelte'
	import KafkaIcon from '$lib/components/icons/KafkaIcon.svelte'
	import MqttIcon from '$lib/components/icons/MqttIcon.svelte'
	import NatsIcon from '$lib/components/icons/NatsIcon.svelte'
	import TriggerModeToggle from './TriggerModeToggle.svelte'
	import { triggerLock } from '$lib/operatorWriteRights'
	import { getHttpRoute } from './http/utils'
	import { getEmailAddress, getEmailDomain } from './email/utils'
	import { TRIGGER_LIST_CONFIG, type TriggerRow } from './triggerListConfig'
	import type { TriggerKind } from '$lib/components/sessions/previewPaths'
	import { useHostedPage } from '$lib/components/hostedPage'
	import {
		useOperatingUser,
		useOperatingWorkspace
	} from '$lib/components/operatingWorkspace.svelte'

	let { triggerKind }: { triggerKind: TriggerKind } = $props()

	const operatingWorkspace = useOperatingWorkspace()
	const operatingUser = useOperatingUser()
	const hosted = useHostedPage()

	const config = $derived(TRIGGER_LIST_CONFIG[triggerKind])
	const isAdmin = $derived(
		!!(operatingUser.current?.is_admin || operatingUser.current?.is_super_admin)
	)

	type EditorHandle = {
		openNew: (isFlow: boolean) => void
		openEdit: (path: string, isFlow: boolean) => void
	}
	type Module = { default: any }
	const EDITORS: Record<TriggerKind, () => Promise<Module>> = {
		http: () => import('./http/RouteEditor.svelte'),
		websocket: () => import('./websocket/WebsocketTriggerEditor.svelte'),
		postgres: () => import('./postgres/PostgresTriggerEditor.svelte'),
		kafka: () => import('./kafka/KafkaTriggerEditor.svelte'),
		nats: () => import('./nats/NatsTriggerEditor.svelte'),
		sqs: () => import('./sqs/SqsTriggerEditor.svelte'),
		gcp: () => import('./gcp/GcpTriggerEditor.svelte'),
		azure: () => import('./azure/AzureTriggerEditor.svelte'),
		mqtt: () => import('./mqtt/MqttTriggerEditor.svelte'),
		amqp: () => import('./amqp/AmqpTriggerEditor.svelte'),
		email: () => import('./email/EmailTriggerEditor.svelte')
	}
	const ICONS: Record<TriggerKind, any> = {
		http: Route,
		websocket: Unplug,
		postgres: Database,
		kafka: KafkaIcon,
		nats: NatsIcon,
		sqs: AwsIcon,
		gcp: GoogleCloudIcon,
		azure: AzureIcon,
		mqtt: MqttIcon,
		amqp: AmqpIcon,
		email: Mail
	}
	const Icon = $derived(ICONS[triggerKind])

	type TriggerW = TriggerRow & { canWrite: boolean }

	let triggers: TriggerW[] = $state([])
	let loading = $state(true)
	let editor: EditorHandle | undefined = $state()
	let shareModal: ShareModal | undefined = $state()
	let deploymentDrawer: DeployWorkspaceDrawer | undefined = $state()
	let routesGenerator: { openDrawer: () => void } | undefined = $state()
	let openAPISpecGenerator: { openDrawer: () => void } | undefined = $state()
	let deployUiSettings: WorkspaceDeployUISettings | undefined = $state(undefined)
	let globalHttpWorkspacedRoute = $state(false)
	let emailDomain: string | null = $state(null)

	function openEdit(path: string, isFlow: boolean) {
		if (hosted) hosted.openItem(path)
		else editor?.openEdit(path, isFlow)
	}

	function openLink(href: string) {
		if (hosted) hosted.openLink(`${base}${href}`)
		else goto(href)
	}

	async function getDeployUiSettings() {
		if (!$enterpriseLicense) {
			deployUiSettings = ALL_DEPLOYABLE
			return
		}
		let settings = await WorkspaceService.getPublicSettings({ workspace: $operatingWorkspace! })
		deployUiSettings = settings.deploy_ui ?? ALL_DEPLOYABLE
	}
	getDeployUiSettings()

	if (untrack(() => triggerKind) === 'http') {
		SettingService.getGlobal({ key: 'http_route_workspaced_route' })
			.then((setting) => (globalHttpWorkspacedRoute = (setting as boolean) ?? false))
			.catch(() => (globalHttpWorkspacedRoute = false))
	}

	async function loadTriggers(): Promise<void> {
		const workspace = $operatingWorkspace!
		const listed = await config.list({ workspace, includeDraftOnly: true })
		triggers = listed.map((x) => ({
			canWrite: canWrite(x.path, x.extra_perms!, operatingUser.current),
			...x
		}))
		// The nav rail lists the trigger kinds of the navigation workspace only.
		if (workspace === $workspaceStore) {
			$usedTriggerKinds = removeTriggerKindIfUnused(
				triggers.length,
				config.usedKind,
				$usedTriggerKinds
			)
		}
		if (triggerKind === 'email') emailDomain = await getEmailDomain()
		loading = false
	}

	$effect(() => {
		if ($operatingWorkspace && operatingUser.current) {
			untrack(() => loadTriggers())
		}
	})

	let interval: ReturnType<typeof setInterval> | undefined = untrack(() => config.status)
		? setInterval(async () => {
				try {
					const newTriggers = await config.list({ workspace: $operatingWorkspace! })
					for (let i = 0; i < triggers.length; i++) {
						const newTrigger = newTriggers.find((x) => x.path === triggers[i].path)
						if (newTrigger) {
							triggers[i] = {
								...triggers[i],
								error: newTrigger.error,
								last_server_ping: newTrigger.last_server_ping,
								mode: newTrigger.mode,
								server_id: newTrigger.server_id
							}
						}
					}
				} catch (err) {
					console.error(err)
				}
			}, 5000)
		: undefined
	onDestroy(() => clearInterval(interval))

	function modeVerb(mode: TriggerMode) {
		return mode === 'enabled' ? 'enable' : mode === 'disabled' ? 'disable' : 'suspend'
	}

	async function onToggleMode(path: string, mode: TriggerMode): Promise<boolean> {
		const workspace = $operatingWorkspace!
		const label = config.forkConflictLabel
		if (label === undefined) {
			try {
				await config.setMode({ path, workspace, requestBody: { mode } })
			} catch (err) {
				sendUserToast(`${config.modeError(modeVerb(mode))}: ${err.body}`, true)
			} finally {
				loadTriggers()
			}
			return true
		}
		let committed = false
		try {
			const ok = await withForkConflictRetry(
				(force) => config.setMode({ path, workspace, requestBody: { mode, force } }),
				label
			)
			if (ok) {
				if (config.modeToastLabel) {
					sendUserToast(`${capitalize(mode)} ${config.modeToastLabel} ${path}`)
				}
				loadTriggers()
			}
			committed = ok
		} catch (err) {
			sendUserToast(`${config.modeError(modeVerb(mode))}: ${err.body}`, true)
			loadTriggers()
		}
		return committed
	}

	async function deleteTrigger(row: TriggerW) {
		const workspace = $operatingWorkspace ?? ''
		if (triggerKind === 'postgres' || triggerKind === 'gcp' || triggerKind === 'azure') {
			isDeleting = false
			deleteA = false
			deleteB = false
			pendingDelete = row
			return
		}
		if (!config.catchDelete) {
			await config.remove({ workspace, path: row.path })
			loadTriggers()
			return
		}
		try {
			await config.remove({ workspace, path: row.path })
			if (config.deleteToastLabel) {
				sendUserToast(`Successfully deleted ${config.deleteToastLabel}: ${row.path}`)
			}
			loadTriggers()
		} catch (error) {
			sendUserToast(error.body || error.message, true)
		}
	}

	// Kinds whose trigger owns an external subscription (a replication slot and publication, a
	// cloud subscription) confirm the delete, offering to drop those too.
	let pendingDelete: TriggerW | undefined = $state(undefined)
	let isDeleting = $state(false)
	let deleteA = $state(false)
	let deleteB = $state(false)

	async function confirmPendingDelete() {
		const row = pendingDelete
		if (!row) return
		const workspace = $operatingWorkspace ?? ''
		isDeleting = true
		const cleanups: (() => Promise<unknown>)[] = []
		if (triggerKind === 'postgres') {
			if (deleteA) {
				cleanups.push(() =>
					PostgresTriggerService.deletePostgresReplicationSlot({
						workspace,
						path: row.postgres_resource_path!,
						requestBody: { name: row.replication_slot_name! }
					})
				)
			}
			if (deleteB) {
				cleanups.push(() =>
					PostgresTriggerService.deletePostgresPublication({
						workspace,
						path: row.postgres_resource_path!,
						publication: row.publication_name!
					})
				)
			}
		} else if (triggerKind === 'gcp' && deleteA) {
			const requestBody = { subscription_id: row.subscription_id!, project_id: row.project_id }
			cleanups.push(() =>
				row.gcp_resource_path
					? GcpTriggerService.deleteGcpSubscription({
							workspace,
							path: row.gcp_resource_path,
							requestBody
						})
					: GcpTriggerService.deleteGcpSubscriptionWithDefaultCredentials({
							workspace,
							requestBody
						})
			)
		} else if (triggerKind === 'azure' && deleteA && row.subscription_name) {
			cleanups.push(() =>
				AzureTriggerService.deleteAzureSubscription({
					workspace,
					path: row.azure_resource_path!,
					requestBody: {
						azure_mode: row.azure_mode!,
						scope_resource_id: row.scope_resource_id!,
						topic_name: row.topic_name ?? undefined,
						subscription_name: row.subscription_name!
					}
				})
			)
		}
		for (const cleanup of cleanups) {
			try {
				sendUserToast((await cleanup()) as string)
			} catch (error) {
				isDeleting = false
				sendUserToast(error.body || error.message, true)
				return
			}
		}
		const msg = (await config.remove({ workspace, path: row.path })) as string
		sendUserToast(msg)
		loadTriggers()
		pendingDelete = undefined
	}

	// A host opens rows itself, and the document's hash is not this page's.
	let hashHandled = false
	$effect(() => {
		if (hosted) return
		if (!hashHandled && triggers.length > 0 && editor) {
			let hash = $page.url.hash
			if (hash.length > 1) {
				let path = hash.slice(1)
				let trigger = triggers.find((t) => t.path === path)
				if (trigger) {
					hashHandled = true
					editor?.openEdit(path, trigger.is_flow)
				}
			}
		}
	})

	let filteredItems: (TriggerW & { marked?: any })[] | undefined = $state([])
	let filter = $state('')
	let ownerFilter: string | undefined = $state(undefined)
	let nbDisplayed = $state(15)

	const TRIGGER_PATH_KIND_FILTER_SETTING = 'filter_path_of'
	const FILTER_USER_FOLDER_SETTING_NAME = 'user_and_folders_only'
	let selectedFilterKind = $state(
		(getLocalSetting(TRIGGER_PATH_KIND_FILTER_SETTING) as 'trigger' | 'script_flow') ?? 'trigger'
	)
	let filterUserFolders = $state(getLocalSetting(FILTER_USER_FOLDER_SETTING_NAME) == 'true')

	$effect(() => {
		storeLocalSetting(TRIGGER_PATH_KIND_FILTER_SETTING, selectedFilterKind)
	})
	$effect(() => {
		storeLocalSetting(FILTER_USER_FOLDER_SETTING_NAME, filterUserFolders ? 'true' : undefined)
	})

	// Read before the first write below, which would otherwise replace what the location asked for.
	function loadQueryFilters(search: string) {
		const params = new URLSearchParams(search)
		const queryFilterKind = params.get(TRIGGER_PATH_KIND_FILTER_SETTING)
		const queryFilterUserFolders = params.get(FILTER_USER_FOLDER_SETTING_NAME)
		if (queryFilterKind) selectedFilterKind = queryFilterKind as 'trigger' | 'script_flow'
		if (queryFilterUserFolders) filterUserFolders = queryFilterUserFolders == 'true'
	}
	untrack(() => loadQueryFilters(hosted ? hosted.search : window.location.search))
	$effect(() => {
		if (!hosted) return
		const search = hosted.search
		untrack(() => loadQueryFilters(search))
	})

	$effect(() => {
		const kind = selectedFilterKind
		const userFolders = String(filterUserFolders)
		untrack(() => {
			if (hosted) {
				const params = new URLSearchParams(hosted.search)
				params.set(TRIGGER_PATH_KIND_FILTER_SETTING, kind)
				params.set(FILTER_USER_FOLDER_SETTING_NAME, userFolders)
				const next = `?${params}`
				if (next !== hosted.search) hosted.setSearch(next)
				return
			}
			setQuery(TRIGGER_PATH_KIND_FILTER_SETTING, kind, window.location.hash || undefined).then(
				() => {
					setQuery(FILTER_USER_FOLDER_SETTING_NAME, userFolders, window.location.hash || undefined)
				}
			)
		})
	})

	function filterItemsPathsBaseOnUserFilters(item: TriggerW) {
		if ($operatingWorkspace == 'admins') return true
		if (!filterUserFolders) return true
		const path = selectedFilterKind === 'trigger' ? item.path : item.script_path
		return !path.startsWith('u/') || path.startsWith('u/' + operatingUser.current?.username + '/')
	}

	const preFilteredItems = $derived(
		triggers.filter(
			(x) =>
				(ownerFilter == undefined ||
					(selectedFilterKind === 'trigger' ? x.path : x.script_path).startsWith(
						ownerFilter + '/'
					)) &&
				filterItemsPathsBaseOnUserFilters(x)
		)
	)

	let lastWorkspace = untrack(() => $operatingWorkspace)
	$effect.pre(() => {
		const workspace = $operatingWorkspace
		if (workspace === lastWorkspace) return
		lastWorkspace = workspace
		ownerFilter = undefined
	})

	const owners = $derived(
		Array.from(
			new Set(
				(filteredItems ?? [])
					.map((x) => (selectedFilterKind === 'trigger' ? x.path : x.script_path))
					.filter(Boolean)
					.map((p) => p.split('/').slice(0, 2).join('/'))
			)
		).sort()
	)

	const items = $derived(filter !== '' ? filteredItems : preFilteredItems)

	// A push delivery has no connection to report and no consumer to suspend: it is an endpoint.
	function isLive(row: TriggerRow): boolean {
		if (triggerKind === 'gcp') return row.delivery_type !== 'push'
		if (triggerKind === 'azure') return row.azure_mode === 'namespace_pull'
		return true
	}

	function copyUrl(row: TriggerRow): { label: string; value: string } | undefined {
		switch (triggerKind) {
			case 'http':
				return {
					label: 'Copy URL',
					value: getHttpRoute(
						'r',
						row.route_path,
						(row.workspaced_route ?? false) || globalHttpWorkspacedRoute,
						row.workspace_id!
					)
				}
			case 'gcp':
			case 'azure':
				return isLive(row)
					? undefined
					: {
							label: 'Copy URL',
							value: getHttpRoute(`${triggerKind}/w`, row.path, true, row.workspace_id!)
						}
			case 'email':
				return { label: 'Copy email address', value: emailAddress(row) }
			default:
				return undefined
		}
	}

	function emailAddress(row: TriggerRow) {
		return getEmailAddress(
			row.local_part,
			row.workspaced_local_part ?? false,
			row.workspace_id!,
			emailDomain ?? ''
		)
	}

	function websocketUrl(url: string) {
		return url.startsWith('$script:')
			? 'URL: ' + url.replace('$script:', 'result of script ')
			: url.startsWith('$flow:')
				? 'URL: ' + url.replace('$flow:', 'result of flow ')
				: url
	}

	const operatorBlocked = $derived(
		config.operatorGate &&
			!!operatingUser.current?.operator &&
			!!$operatingWorkspace &&
			!$userWorkspaces.find((_) => _.id === $operatingWorkspace)?.operator_settings?.triggers
	)
	const canCreate = $derived(!config.adminOnly || isAdmin)
</script>

{#snippet rowTitle(row: TriggerW & { marked?: any }, hasDraft: boolean | undefined)}
	{@const marked = ['http', 'websocket', 'kafka', 'nats', 'email'].includes(triggerKind)
		? row.marked
		: undefined}
	<div class="text-emphasis flex-wrap text-left text-xs font-semibold mb-1 truncate">
		{#if marked}
			<span class="text-xs">
				{@html marked}
			</span>
		{:else if triggerKind === 'http'}
			{#if row.summary}
				{row.summary}
			{:else}
				{row.http_method!.toUpperCase()}
				/{isCloudHosted() || row.workspaced_route || globalHttpWorkspacedRoute
					? row.workspace_id + '/' + row.route_path
					: row.route_path}
			{/if}
		{:else if triggerKind === 'websocket'}
			{websocketUrl(row.url!)}
		{:else if triggerKind === 'kafka'}
			{row.kafka_resource_path} - {row.topics!.join(', ')}
		{:else if triggerKind === 'nats'}
			{row.nats_resource_path} - {row.subjects!.join(', ')}
		{:else if triggerKind === 'email'}
			{emailAddress(row)}
		{:else if triggerKind === 'gcp'}
			{row.path} - {row.topic_id}
		{:else if triggerKind === 'azure'}
			{row.path} - {row.topic_name
				? row.topic_name
				: (row.scope_resource_id?.split('/')?.pop() ?? '')} ({row.azure_mode}, {row.subscription_name})
		{:else}
			{row.path}
		{/if}{hasDraft ? '*' : ''}
	</div>
	{#if ['http', 'websocket', 'kafka', 'nats', 'email'].includes(triggerKind)}
		<div class="text-secondary text-xs truncate text-left font-light">{row.path}</div>
	{:else if triggerKind === 'postgres'}
		<div class="text-secondary text-xs truncate text-left font-light">
			{row.postgres_resource_path}
		</div>
	{/if}
	<div class="text-secondary text-xs truncate text-left font-light">
		{#if triggerKind === 'http' && row.static_asset_config}
			file: {row.static_asset_config.s3}
		{:else}
			runnable: {row.script_path}
		{/if}
	</div>
{/snippet}

{#snippet statusDot(row: TriggerW)}
	{@const status = config.status}
	{#if status}
		{@const ping = status.pings
			? row.last_server_ping
				? new Date(row.last_server_ping)
				: undefined
			: new Date()}
		{@const pinging = ping && ping.getTime() > new Date().getTime() - 15 * 1000}
		{@const effectiveMode = row.draft_only ? 'disabled' : row.mode}
		{@const enabled = effectiveMode === 'enabled' || effectiveMode === 'suspended'}
		<div class="w-10">
			{#if (enabled && (!pinging || row.error)) || (!enabled && row.error) || (enabled && !row.server_id)}
				<Popover notClickable>
					<span class="flex h-4 w-4">
						<Circle class="text-red-600 animate-ping absolute inline-flex fill-current" size={12} />
						<Circle class="text-red-600 relative inline-flex fill-current" size={12} />
					</span>
					{#snippet text()}
						<div>
							{#if enabled}
								{#if !row.server_id}
									{status.starting}
								{:else}
									{status.notConnected}{row.error ? ': ' + row.error : ''}
								{/if}
							{:else}
								{status.disabled}: {row.error}
							{/if}
						</div>
					{/snippet}
				</Popover>
			{:else if enabled}
				<Popover notClickable>
					<span class="flex h-4 w-4">
						<Circle class="text-green-600 relative inline-flex fill-current" size={12} />
					</span>
					{#snippet text()}
						<div>
							{status.connected}{status.shuttingDown && !row.server_id ? ' (shutting down...)' : ''}
						</div>
					{/snippet}
				</Popover>
			{/if}
		</div>
	{/if}
{/snippet}

<ConfirmationModal
	open={pendingDelete !== undefined}
	title={triggerKind === 'postgres'
		? 'Delete Postgres trigger'
		: triggerKind === 'gcp'
			? 'Delete GCP Pub/Sub trigger'
			: 'Delete Azure Event Grid trigger'}
	confirmationText="Remove"
	loading={isDeleting}
	on:canceled={() => {
		isDeleting = false
		pendingDelete = undefined
	}}
	on:confirmed={confirmPendingDelete}
>
	<div class="flex flex-col w-full space-y-4">
		<span>Are you sure you want to remove this trigger?</span>
		{#if pendingDelete && triggerKind === 'postgres'}
			<Toggle
				options={{
					left: `Delete the associated replication slot named: ${pendingDelete.replication_slot_name} ?`
				}}
				bind:checked={deleteA}
			/>
			<Toggle
				options={{
					left: `Delete the associated publication named: ${pendingDelete.publication_name} ?`
				}}
				bind:checked={deleteB}
			/>
		{:else if pendingDelete && triggerKind === 'gcp'}
			<Toggle
				options={{
					left: `Delete subscription "${pendingDelete.subscription_id}" subscribed to topic "${pendingDelete.topic_id}"?`
				}}
				bind:checked={deleteA}
			/>
		{:else if pendingDelete?.subscription_name && triggerKind === 'azure'}
			<Toggle
				options={{
					left: `Also delete Azure subscription "${pendingDelete.subscription_name}" on Azure?`
				}}
				bind:checked={deleteA}
			/>
		{/if}
	</div>
</ConfirmationModal>

<DeployWorkspaceDrawer bind:this={deploymentDrawer} />
{#await EDITORS[triggerKind]() then Editor}
	<Editor.default onUpdate={loadTriggers} bind:this={editor} />
{/await}
{#if triggerKind === 'http'}
	{#await import('./http/RoutesGenerator.svelte') then Generator}
		<Generator.default closeFn={loadTriggers} bind:this={routesGenerator} />
	{/await}
	{#await import('./http/OpenAPISpecGenerator.svelte') then Generator}
		<Generator.default bind:this={openAPISpecGenerator} />
	{/await}
{/if}

<SearchItems
	{filter}
	items={preFilteredItems}
	bind:filteredItems
	f={(x) => (x.summary ?? '') + ' ' + x.path + ' (' + x.script_path + ')'}
/>

{#if operatorBlocked}
	<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4 m-4 mt-12" role="alert">
		<p class="font-bold">Unauthorized</p>
		<p>Page not available for operators</p>
	</div>
{:else}
	<CenteredPage>
		<PageHeader
			title={config.title}
			tooltip={config.tooltip}
			documentationLink={config.documentationLink}
		>
			<div class="flex flex-row gap-2">
				{#if triggerKind === 'http'}
					<Button
						unifiedSize="md"
						variant="default"
						startIcon={{ icon: Plus }}
						disabled={!!$triggerLock}
						title={$triggerLock}
						on:click={() => routesGenerator?.openDrawer()}
					>
						From OpenAPI spec
					</Button>
					<Button
						unifiedSize="md"
						variant="default"
						startIcon={{ icon: Plus }}
						on:click={() => openAPISpecGenerator?.openDrawer()}
					>
						To OpenAPI spec
					</Button>
				{/if}
				{#if canCreate}
					<Button
						unifiedSize="md"
						variant="accent"
						startIcon={{ icon: Plus }}
						disabled={!!$triggerLock}
						title={$triggerLock}
						on:click={() => editor?.openNew(false)}
					>
						New&nbsp;{config.newLabel}
					</Button>
				{/if}
			</div>
		</PageHeader>

		{#if config.cloudDisabled && isCloudHosted()}
			<Alert title="Not compatible with multi-tenant cloud" type="warning">
				{config.title} are disabled in the multi-tenant cloud.
			</Alert>
			<div class="py-4"></div>
		{/if}
		<div class="w-full h-full flex flex-col">
			<div class="w-full pb-4 pt-6">
				<input
					type="text"
					placeholder={config.searchPlaceholder}
					bind:value={filter}
					class="search-item"
				/>
				<div class="flex flex-row items-center gap-2 mt-2">
					<div class="text-xs font-semibold text-emphasis shrink-0"> Filter by path of </div>
					<ToggleButtonGroup bind:selected={selectedFilterKind}>
						{#snippet children({ item })}
							<ToggleButton value="trigger" label={config.filterLabel} icon={Icon} {item} />
							<ToggleButton value="script_flow" label="Script/Flow" icon={Code} {item} />
						{/snippet}
					</ToggleButtonGroup>
				</div>
				<ListFilters syncQuery bind:selectedFilter={ownerFilter} filters={owners} />

				<div class="flex flex-row items-center justify-end gap-4">
					{#if operatingUser.current?.is_super_admin && operatingUser.current.username.includes('@')}
						<Toggle size="xs" bind:checked={filterUserFolders} options={{ right: 'Only f/*' }} />
					{:else if isAdmin}
						<Toggle
							size="xs"
							bind:checked={filterUserFolders}
							options={{ right: `Only u/${operatingUser.current?.username} and f/*` }}
						/>
					{/if}
				</div>
			</div>
			{#if loading}
				{#each new Array(6) as _}
					<Skeleton layout={[[6], 0.4]} />
				{/each}
			{:else if !triggers?.length}
				<EmptyState
					icon={Icon}
					title={config.empty.title}
					description={config.empty.description}
					action={canCreate
						? {
								label: config.empty.actionLabel,
								icon: Plus,
								onClick: () => editor?.openNew(false),
								disabled: !!$triggerLock,
								title: $triggerLock,
								aiId: config.empty.aiId,
								aiDescription: config.empty.aiDescription
							}
						: undefined}
				/>
			{:else if items?.length}
				<div class="border rounded-md divide-y">
					{#each items.slice(0, nbDisplayed) as row (row.path)}
						{@const {
							path,
							edited_by,
							edited_at,
							script_path,
							is_flow,
							extra_perms,
							canWrite,
							mode,
							retry,
							error_handler_path,
							error_handler_args,
							labels,
							draft_only,
							is_draft
						} = row}
						{@const hasDraft =
							getLocalDraftHint($operatingWorkspace, config.draftKind, path) ?? is_draft}
						{@const href = `${is_flow ? '/flows/get' : '/scripts/get'}/${script_path}`}
						{@const effectiveMode = draft_only ? 'disabled' : mode}
						{@const live = isLive(row)}
						{@const copy = copyUrl(row)}
						{@const canEdit = canWrite && !$triggerLock}

						<div
							class="bg-surface-tertiary hover:bg-surface-hover w-full items-center px-4 py-2 gap-4 first-of-type:!border-t-0
				first-of-type:rounded-t-md last-of-type:rounded-b-md flex flex-col"
						>
							<div class="w-full flex gap-5 items-center">
								<RowIcon kind={is_flow ? 'flow' : 'script'} />

								<a
									href="#{path}"
									onclick={(e) => {
										if (hosted) e.preventDefault()
										openEdit(path, is_flow)
									}}
									class="min-w-0 grow hover:underline decoration-gray-400"
								>
									{@render rowTitle(row, hasDraft)}
								</a>

								<div class="hidden lg:flex flex-row gap-1 items-center">
									<SharedBadge {canWrite} extraPerms={extra_perms} />
									{#if triggerKind !== 'http' && triggerKind !== 'azure' && labels?.length}
										{#each labels as label}
											<Badge color="blue" small class="px-1" title="Label: {label}">{label}</Badge>
										{/each}
									{/if}
								</div>

								{#if live}
									{@render statusDot(row)}
								{/if}

								<div class="flex items-center justify-end gap-2 shrink-0 min-w-[8rem]">
									<DraftBadge {draft_only} is_draft={hasDraft} />
									{#if live}
										<TriggerModeToggle
											disabled={draft_only}
											title={$triggerLock ??
												(draft_only
													? 'Draft only: deploy the trigger to enable it'
													: hasDraft
														? 'Enables/disables the deployed trigger; the draft is not affected'
														: undefined)}
											onToggleMode={(newMode) => onToggleMode(path, newMode)}
											triggerMode={effectiveMode}
											includeModalConfig={{
												triggerPath: path,
												triggerKind,
												runnableConfig: {
													path: script_path,
													kind: is_flow ? 'flow' : 'script',
													retry,
													errorHandlerPath: error_handler_path,
													errorHandlerArgs: error_handler_args
												}
											}}
											canWrite={canEdit}
											hideToggleLabels
											hideDropdown
										/>
									{/if}
								</div>

								<div class="flex gap-2 items-center justify-end">
									{#if copy}
										<Button
											on:click={() => copyToClipboard(copy.value)}
											variant="subtle"
											unifiedSize="md"
											startIcon={{ icon: ClipboardCopy }}
										>
											{copy.label}
										</Button>
									{/if}
									<Button
										on:click={() => openEdit(path, is_flow)}
										unifiedSize="md"
										startIcon={canEdit ? { icon: Pen } : { icon: Eye }}
										variant="subtle"
									>
										{canEdit ? 'Edit' : 'View'}
									</Button>
									<Dropdown
										items={[
											{
												displayName: `View ${is_flow ? 'Flow' : 'Script'}`,
												icon: Eye,
												action: () => openLink(href)
											},
											...(canEdit && !draft_only && mode !== 'suspended'
												? [
														{
															displayName: 'Suspend job execution',
															icon: Pause,
															action: () => {
																onToggleMode(path, 'suspended')
															}
														}
													]
												: []),
											{
												displayName: canEdit ? 'Edit' : 'View',
												icon: canEdit ? Pen : Eye,
												action: () => openEdit(path, is_flow)
											},
											...(isDeployable('trigger', path, deployUiSettings)
												? [
														{
															displayName: 'Deploy to prod/staging',
															icon: FileUp,
															action: () => {
																deploymentDrawer?.openDrawer(path, 'trigger', {
																	triggers: { kind: config.deployKind }
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
												displayName: 'Permissions',
												icon: Shield,
												action: () => {
													shareModal?.openDrawer(path, config.shareKind as any)
												}
											},
											{
												displayName: 'Delete',
												type: 'delete',
												icon: Trash,
												disabled: !canEdit || (config.adminOnly && !isAdmin),
												tooltip: $triggerLock,
												action: () => deleteTrigger(row)
											}
										]}
									/>
								</div>
							</div>
							<div class="w-full flex justify-end items-baseline">
								<div
									class="flex flex-wrap text-2xs font-normal text-secondary gap-1 items-center justify-end truncate pr-2"
								>
									{#if edited_by}<div class="truncate">edited by {edited_by}</div>{/if}
									<div class="truncate">{edited_by ? 'at ' : ''}{displayDate(edited_at)}</div>
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
			<span class="text-xs font-normal text-primary"
				>{nbDisplayed} items out of {items.length}
				<button class="ml-4 font-semibold text-emphasis" onclick={() => (nbDisplayed += 30)}
					>load 30 more</button
				></span
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
