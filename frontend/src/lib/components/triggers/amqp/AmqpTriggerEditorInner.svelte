<script lang="ts">
	import { untrack } from 'svelte'
	import { Alert } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import TriggerRunnablePicker from '$lib/components/triggers/TriggerRunnablePicker.svelte'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { getTriggerWorkspace } from '$lib/components/triggers/triggerWorkspace'
	import { canWrite, capitalize, emptyString, sendUserToast } from '$lib/utils'
	import { withForkConflictRetry } from '$lib/utils/forkConflict'
	import Section from '$lib/components/Section.svelte'
	import { Loader2 } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import {
		AmqpTriggerService,
		type AmqpExchange,
		type AmqpOptions,
		type Retry,
		type ErrorHandler,
		type TriggerMode
	} from '$lib/gen'
	import AmqpEditorConfigSection from './AmqpEditorConfigSection.svelte'
	import type { Snippet } from 'svelte'
	import TriggerEditorToolbar from '../TriggerEditorToolbar.svelte'
	import PermissionedAsLine from '../PermissionedAsLine.svelte'
	import { saveAmqpTriggerFromCfg } from './utils'
	import { getHandlerType, handleConfigChange, type Trigger } from '../utils'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import TriggerRetriesAndErrorHandler from '../TriggerRetriesAndErrorHandler.svelte'
	import TriggerAdvancedBadges from '../TriggerAdvancedBadges.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import TriggerSuspendedJobsAlert from '../TriggerSuspendedJobsAlert.svelte'
	import TriggerSuspendedJobsModal from '../TriggerSuspendedJobsModal.svelte'
	import { deepEqual } from 'fast-equals'
	import { useTriggerDraftSync } from '../useTriggerDraftSync.svelte'
	import LocalDraftBanner from '$lib/components/LocalDraftBanner.svelte'

	interface Props {
		useDrawer?: boolean
		description?: Snippet | undefined
		hideTarget?: boolean
		hideTooltips?: boolean
		isEditor?: boolean
		allowDraft?: boolean
		trigger?: Trigger
		customLabel?: Snippet
		isDeployed?: boolean
		cloudDisabled?: boolean
		onConfigChange?: (cfg: Record<string, any>, saveDisabled: boolean, updated: boolean) => void
		onCaptureConfigChange?: (cfg: Record<string, any>, isValid: boolean) => void
		onUpdate?: (path?: string) => void
		onDelete?: () => void
		onReset?: () => void
	}

	let {
		useDrawer = true,
		description = undefined,
		hideTarget = false,
		hideTooltips = false,
		isEditor = false,
		allowDraft = false,
		trigger = undefined,
		customLabel = undefined,
		isDeployed = false,
		onConfigChange = undefined,
		onCaptureConfigChange = undefined,
		onUpdate = undefined,
		onDelete = undefined,
		onReset = undefined,
		cloudDisabled = false
	}: Props = $props()
	const triggerWs = getTriggerWorkspace()
	const wsId = $derived(triggerWs?.() ?? $workspaceStore)

	let amqp_resource_path: string = $state('')
	let drawer: Drawer | undefined = $state(undefined)
	let is_flow: boolean = $state(false)
	let initialPath = $state('')
	let edit = $state(true)
	let itemKind: 'flow' | 'script' = $state('script')
	let script_path = $state('')
	let initialScriptPath = $state('')
	let fixedScriptPath = $state('')
	let path: string = $state('')
	let pathError = $state('')
	let mode = $state<TriggerMode>('enabled')
	let dirtyPath = $state(false)
	let can_write = $state(true)
	let drawerLoading = $state(true)
	let showLoading = $state(false)
	let queue_name: string = $state('')
	let exchange: AmqpExchange | undefined = $state(undefined)
	let options: AmqpOptions | undefined = $state(undefined)
	let isValid: boolean = $state(false)
	let initialConfig: Record<string, any> | undefined = {}
	let deploymentLoading = $state(false)
	let permissionedAs = $state<string | undefined>(undefined)
	let selectedPermissionedAs = $state<string | undefined>(undefined)
	let preservePermissionedAs = $state(false)
	let errorHandlerSelected: ErrorHandler = $state('slack')
	let error_handler_path: string | undefined = $state()
	let error_handler_args: Record<string, any> = $state({})
	let retry: Retry | undefined = $state()
	let suspendedJobsModal = $state<TriggerSuspendedJobsModal | null>(null)
	let originalConfig = $state<Record<string, any> | undefined>(undefined)

	let optionTabSelected: 'connection_options' | 'error_handler' | 'retries' =
		$state('connection_options')

	// prefetch is surfaced as a plain toggle-backed number in the Advanced tab.
	let enablePrefetch = $state(false)

	let hasChanged = $derived(!deepEqual(getSaveCfg(), originalConfig ?? {}))
	const amqpConfig = $derived.by(getSaveCfg)

	const draftSync = useTriggerDraftSync({
		itemKind: 'trigger_amqp',
		path: () => initialPath,
		workspace: () => wsId,
		drawerLoading: () => drawerLoading,
		getCfg: () => amqpConfig,
		applyCfg: loadTriggerConfig,
		deployed: () => originalConfig
	})
	const captureConfig = $derived.by(untrack(() => isEditor) ? getCaptureConfig : () => ({}))
	const saveDisabled = $derived(
		pathError != '' || emptyString(script_path) || !can_write || !isValid || !hasChanged
	)

	$effect(() => {
		is_flow = itemKind === 'flow'
	})

	export async function openEdit(
		ePath: string,
		isFlow: boolean,
		defaultConfig?: Record<string, any>,
		fixedScriptPath_?: string
	) {
		let loadingTimeout = setTimeout(() => {
			showLoading = true
		}, 100) // Do not show loading spinner for the first 100ms
		drawerLoading = true
		try {
			drawer?.openDrawer()
			initialPath = ePath
			itemKind = isFlow ? 'flow' : 'script'
			edit = true
			dirtyPath = false
			fixedScriptPath = fixedScriptPath_ ?? ''
			const { overlay: draftOverlay, noDeployed } = await loadTrigger(defaultConfig)
			edit = !noDeployed
			originalConfig = structuredClone($state.snapshot(getSaveCfg()))
			if (draftOverlay) loadTriggerConfig(draftOverlay)
			if (!defaultConfig) {
				initialConfig = structuredClone($state.snapshot(getSaveCfg()))
			}
			await draftSync.maybeRestore()
		} catch (err) {
			sendUserToast(`Could not load amqp trigger: ${err.body}`, true)
		} finally {
			clearTimeout(loadingTimeout)
			drawerLoading = false
			showLoading = false
		}
	}

	export async function openNew(
		nis_flow: boolean,
		fixedScriptPath_?: string,
		defaultValues?: Record<string, any>
	) {
		let loadingTimeout = setTimeout(() => {
			showLoading = true
		}, 100)
		drawerLoading = true
		try {
			amqp_resource_path = defaultValues?.amqp_resource_path ?? ''
			drawer?.openDrawer()
			is_flow = nis_flow
			itemKind = nis_flow ? 'flow' : 'script'
			initialScriptPath = ''
			fixedScriptPath = fixedScriptPath_ ?? ''
			script_path = fixedScriptPath
			queue_name = defaultValues?.queue_name ?? ''
			exchange = defaultValues?.exchange ?? undefined
			options = defaultValues?.options ?? undefined
			enablePrefetch = options?.prefetch_count != undefined
			path = defaultValues?.path ?? ''
			initialPath = ''
			edit = false
			dirtyPath = false
			mode = defaultValues?.mode ?? 'enabled'
			error_handler_path = defaultValues?.error_handler_path ?? undefined
			error_handler_args = defaultValues?.error_handler_args ?? {}
			retry = defaultValues?.retry ?? undefined
			errorHandlerSelected = getHandlerType(error_handler_path ?? '')
			permissionedAs = undefined
			selectedPermissionedAs = undefined
			preservePermissionedAs = false
			originalConfig = undefined
		} finally {
			clearTimeout(loadingTimeout)
			drawerLoading = false
			showLoading = false
		}
	}

	async function loadTriggerConfig(cfg?: Record<string, any>): Promise<void> {
		try {
			amqp_resource_path = cfg?.amqp_resource_path
			queue_name = cfg?.queue_name ?? ''
			exchange = cfg?.exchange ?? undefined
			options = cfg?.options ?? undefined
			enablePrefetch = options?.prefetch_count != undefined
			script_path = cfg?.script_path
			initialScriptPath = cfg?.script_path
			is_flow = cfg?.is_flow
			path = cfg?.path
			mode = cfg?.mode ?? 'enabled'
			can_write = canWrite(cfg?.path, cfg?.extra_perms, $userStore)
			error_handler_path = cfg?.error_handler_path
			error_handler_args = cfg?.error_handler_args ?? {}
			retry = cfg?.retry
			errorHandlerSelected = getHandlerType(error_handler_path ?? '')
			permissionedAs = cfg?.permissioned_as
			selectedPermissionedAs = cfg?.permissioned_as
			preservePermissionedAs = !!cfg?.permissioned_as
		} catch (error) {
			sendUserToast(`Could not load amqp trigger config: ${error.body}`, true)
		}
	}

	/** See `NatsTriggerEditorInner.loadTrigger` for the rationale. */
	async function loadTrigger(
		defaultConfig?: Record<string, any>
	): Promise<{ overlay: Record<string, any> | undefined; noDeployed: boolean }> {
		try {
			if (defaultConfig) {
				loadTriggerConfig(defaultConfig)
				return { overlay: undefined, noDeployed: false }
			}
			const s = await AmqpTriggerService.getAmqpTrigger({
				workspace: wsId!,
				path: initialPath,
				getDraft: true
			})
			const { draft: draftFromBackend, ...deployedTrigger } = (s ?? {}) as any
			loadTriggerConfig(deployedTrigger)
			return {
				noDeployed: !!(s as any)?.no_deployed,
				overlay: draftFromBackend
					? ({ ...deployedTrigger, ...draftFromBackend } as Record<string, any>)
					: undefined
			}
		} catch (error) {
			sendUserToast(`Could not load amqp trigger: ${error.body}`, true)
			return { overlay: undefined, noDeployed: false }
		}
	}

	function getSaveCfg(): Record<string, any> {
		return {
			amqp_resource_path,
			queue_name,
			exchange,
			options,
			path,
			script_path,
			mode,
			is_flow,
			error_handler_path,
			error_handler_args,
			retry,
			permissioned_as: selectedPermissionedAs,
			preserve_permissioned_as: preservePermissionedAs || undefined
		}
	}

	function getCaptureConfig(): Record<string, any> {
		return {
			amqp_resource_path,
			queue_name,
			exchange,
			options,
			path
		}
	}

	async function updateTrigger(): Promise<void> {
		deploymentLoading = true
		const previousPath = initialPath
		const cfg = getSaveCfg()
		const isSaved = await saveAmqpTriggerFromCfg(initialPath, cfg, edit, wsId!, usedTriggerKinds)
		if (isSaved) {
			draftSync.discard(previousPath, getSaveCfg())
			onUpdate?.(cfg.path)
			originalConfig = structuredClone($state.snapshot(getSaveCfg()))
			initialPath = cfg.path
			initialScriptPath = cfg.script_path
			if (mode !== 'suspended') {
				drawer?.closeDrawer()
			}
		}
		deploymentLoading = false
	}

	async function handleToggleMode(newMode: TriggerMode) {
		const previousMode = mode
		mode = newMode
		if (!trigger?.draftConfig) {
			const ok = await withForkConflictRetry(
				(force) =>
					AmqpTriggerService.setAmqpTriggerMode({
						path: initialPath,
						workspace: wsId ?? '',
						requestBody: { mode: newMode, force }
					}),
				'AMQP trigger'
			)
			if (!ok) {
				mode = previousMode
				return
			}
			sendUserToast(`${capitalize(newMode)} AMQP trigger ${initialPath}`)
			onUpdate?.(initialPath)
		}
		if (originalConfig) {
			originalConfig['mode'] = newMode
		}
	}

	// Keep `options.prefetch_count` in sync with the enable toggle without
	// leaving a stale value behind when it is switched off.
	function setPrefetchEnabled(enabled: boolean) {
		enablePrefetch = enabled
		if (enabled) {
			options = { ...(options ?? {}), prefetch_count: options?.prefetch_count ?? 10 }
		} else if (options) {
			const { prefetch_count: _drop, ...rest } = options
			options = Object.keys(rest).length ? rest : undefined
		}
	}

	function setDeclareQueue(declare: boolean) {
		options = { ...(options ?? {}), declare_queue: declare }
	}

	$effect(() => {
		let args = [captureConfig, isValid] as const
		onCaptureConfigChange?.(...args)
	})

	$effect(() => {
		if (!drawerLoading) {
			handleConfigChange(amqpConfig, initialConfig, saveDisabled, edit, onConfigChange)
		}
	})
</script>

{#if mode === 'suspended'}
	<TriggerSuspendedJobsModal
		bind:this={suspendedJobsModal}
		triggerPath={path}
		triggerKind="amqp"
		{hasChanged}
		onToggleMode={handleToggleMode}
		runnableConfig={{
			path: script_path,
			kind: itemKind,
			retry,
			errorHandlerPath: error_handler_path,
			errorHandlerArgs: error_handler_args
		}}
	/>
{/if}

{#if useDrawer}
	<Drawer size="800px" bind:this={drawer}>
		<DrawerContent
			bannerReserved={draftSync.hasBaseline}
			title={edit
				? can_write
					? `Edit AMQP trigger ${initialPath}`
					: `AMQP trigger ${initialPath}`
				: 'New AMQP trigger'}
			on:close={drawer.closeDrawer}
		>
			{#snippet actions()}
				{@render actionsSnippet()}
			{/snippet}
			{#snippet banner()}
				<LocalDraftBanner
					show={draftSync.hasDraft}
					getDeployed={() => draftSync.deployed}
					reserveSpace={draftSync.hasBaseline}
					getCurrent={() => draftSync.current}
					onDiscard={() => draftSync.resetToDeployed(initialPath)}
					disabled={!can_write}
				/>
			{/snippet}
			{@render config()}
		</DrawerContent>
	</Drawer>
{:else}
	<Section label={!customLabel ? 'AMQP trigger' : ''} headerClass="grow min-w-0 h-[30px]">
		{#snippet header()}
			{#if customLabel}
				{@render customLabel()}
			{/if}
		{/snippet}
		{#snippet action()}
			{@render actionsSnippet()}
		{/snippet}
		{@render config()}
	</Section>
{/if}

{#snippet actionsSnippet()}
	{#if !drawerLoading}
		<TriggerEditorToolbar
			{trigger}
			permissions={drawerLoading || !can_write ? 'none' : 'create'}
			{saveDisabled}
			{mode}
			{allowDraft}
			{edit}
			isLoading={deploymentLoading}
			{isDeployed}
			onUpdate={updateTrigger}
			{onReset}
			{onDelete}
			onToggleMode={handleToggleMode}
			{cloudDisabled}
			{suspendedJobsModal}
		/>
	{/if}
{/snippet}

{#snippet config()}
	{#if drawerLoading}
		{#if showLoading}
			<Loader2 class="animate-spin" />
		{/if}
	{:else}
		<PermissionedAsLine
			{permissionedAs}
			{path}
			onPermissionedAsChange={(pa, preserve) => {
				selectedPermissionedAs = pa
				preservePermissionedAs = preserve
			}}
		/>
		<div class="flex flex-col gap-4">
			{#if description}
				{@render description()}
			{/if}
			{#if !hideTooltips}
				<Alert title="Info" type="info" size="xs">
					{#if edit}
						Changes can take up to 30 seconds to take effect.
					{:else}
						New AMQP triggers can take up to 30 seconds to start listening.
					{/if}
				</Alert>
			{/if}
		</div>
		<div class="flex flex-col gap-12 mt-6">
			{#if mode === 'suspended'}
				<TriggerSuspendedJobsAlert {suspendedJobsModal} />
			{/if}
			<div class="flex flex-col gap-4">
				<Label label="Path">
					<Path
						workspaceOverride={wsId}
						bind:dirty={dirtyPath}
						bind:error={pathError}
						bind:path
						{initialPath}
						checkInitialPathExistence={!edit}
						namePlaceholder="amqp_trigger"
						kind="amqp_trigger"
						disabled={!can_write}
						disableEditing={!can_write}
					/>
				</Label>
			</div>

			{#if !hideTarget}
				<Section label="Runnable">
					<TriggerRunnablePicker
						workspace={wsId}
						{fixedScriptPath}
						bind:itemKind
						bind:scriptPath={script_path}
						{initialScriptPath}
						canWrite={can_write}
						isOperator={!!$userStore?.operator}
					/>
				</Section>
			{/if}

			<AmqpEditorConfigSection
				bind:amqp_resource_path
				bind:queue_name
				bind:exchange
				bind:options
				{can_write}
				bind:isValid
				showTestingBadge={isEditor}
			/>

			<Section label="Advanced" collapsable>
				{#snippet header()}
					<TriggerAdvancedBadges
						{error_handler_path}
						{retry}
						extraBadges={[
							{ name: 'Exchange bound', active: !!exchange },
							{ name: 'Prefetch', active: enablePrefetch }
						]}
					/>
				{/snippet}
				<div class="flex flex-col gap-6">
					<div class="min-h-96">
						<Tabs bind:selected={optionTabSelected}>
							<Tab value="connection_options" label="Consumer Options" />
							<Tab value="error_handler" label="Error Handler" />
							<Tab value="retries" label="Retries" />
						</Tabs>
						<div class="mt-4">
							{#if optionTabSelected === 'connection_options'}
								<div class="flex p-2 flex-col gap-4 mt-3">
									<Toggle
										textClass="font-normal text-sm"
										color="nord"
										size="xs"
										checked={options?.declare_queue ?? true}
										disabled={!can_write}
										on:change={(ev) => setDeclareQueue(ev.detail)}
										options={{
											right: 'Declare queue',
											rightTooltip:
												'Declare the queue as durable before consuming. When disabled, the queue is declared passively and must already exist on the broker.'
										}}
										class="py-1"
									/>

									<div class="flex flex-col gap-2">
										<Toggle
											textClass="font-normal text-sm"
											color="nord"
											size="xs"
											checked={enablePrefetch}
											disabled={!can_write}
											on:change={(ev) => setPrefetchEnabled(ev.detail)}
											options={{
												right: 'Prefetch count',
												rightTooltip:
													'Maximum number of unacknowledged messages the broker delivers at once (QoS).'
											}}
											class="py-1"
										/>
										{#if enablePrefetch && options}
											<input
												type="number"
												bind:value={options.prefetch_count}
												disabled={!can_write}
												placeholder="Prefetch count"
												autocomplete="off"
											/>
										{/if}
									</div>
								</div>
							{:else}
								<TriggerRetriesAndErrorHandler
									workspace={wsId}
									{optionTabSelected}
									{itemKind}
									{can_write}
									bind:errorHandlerSelected
									bind:error_handler_path
									bind:error_handler_args
									bind:retry
								/>
							{/if}
						</div>
					</div>
				</div>
			</Section>
			<div class="pb-8"></div>
		</div>
	{/if}
{/snippet}
