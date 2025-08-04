<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, sendUserToast } from '$lib/utils'
	import { Loader2 } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import {
		GcpTriggerService,
		type DeliveryType,
		type PushConfig,
		type SubscriptionMode,
		type Retry
	} from '$lib/gen'
	import Section from '$lib/components/Section.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import Required from '$lib/components/Required.svelte'
	import GcpTriggerEditorConfigSection from './GcpTriggerEditorConfigSection.svelte'
	import { untrack, type Snippet } from 'svelte'
	import TriggerEditorToolbar from '../TriggerEditorToolbar.svelte'
	import { saveGcpTriggerFromCfg } from './utils'
	import { getHandlerType, handleConfigChange, type Trigger } from '../utils'
	import { base } from '$lib/base'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import TriggerRetriesAndErrorHandler from '../TriggerRetriesAndErrorHandler.svelte'

	let drawer: Drawer | undefined = $state(undefined)
	let is_flow: boolean = $state(false)
	let initialPath = $state('')
	let edit = $state(true)
	let delivery_type: DeliveryType = $state('pull')
	let itemKind: 'flow' | 'script' = $state('script')
	let script_path = $state('')
	let initialScriptPath = $state('')
	let fixedScriptPath = $state('')
	let path: string = $state('')
	let pathError = $state('')
	let enabled = $state(false)
	let dirtyPath = $state(false)
	let can_write = $state(true)
	let drawerLoading = $state(true)
	let topic_id: string = $state('')
	let gcp_resource_path: string = $state('')
	let subscription_id: string = $state('')
	let isValid = $state(false)
	let delivery_config: PushConfig | undefined = $state(undefined)
	let subscription_mode: SubscriptionMode = $state('create_update')
	let initialConfig: Record<string, any> | undefined = undefined
	let deploymentLoading = $state(false)
	let base_endpoint = $derived(`${window.location.origin}${base}`)
	let optionTabSelected: 'error_handler' | 'retries' = $state('error_handler')
	let errorHandlerSelected: 'slack' | 'teams' | 'custom' = $state('slack')
	let error_handler_path: string | undefined = $state()
	let error_handler_args: Record<string, any> = $state({})
	let retry: Retry | undefined = $state()
	let auto_acknowledge_msg = $state(true)
	let {
		useDrawer = true,
		description = undefined,
		hideTarget = false,
		hideTooltips = false,
		isEditor = false,
		allowDraft = false,
		trigger = undefined,
		isDeployed = false,
		customLabel = undefined,
		onConfigChange = undefined,
		onCaptureConfigChange = undefined,
		onUpdate = undefined,
		onDelete = undefined,
		onReset = undefined,
		cloudDisabled = false
	}: {
		useDrawer?: boolean
		description?: Snippet | undefined
		hideTarget?: boolean
		hideTooltips?: boolean
		isEditor?: boolean
		allowDraft?: boolean
		trigger?: Trigger
		isDeployed?: boolean
		customLabel?: Snippet
		onConfigChange?: (cfg: Record<string, any>, saveDisabled: boolean, updated: boolean) => void
		onCaptureConfigChange?: (cfg: Record<string, any>, isValid: boolean) => void
		onUpdate?: (path?: string) => void
		onDelete?: () => void
		onReset?: () => void
		cloudDisabled?: boolean
	} = $props()

	const gcpConfig = $derived.by(getGcpConfig)
	const saveDisabled = $derived(
		pathError != '' || emptyString(script_path) || !isValid || !can_write
	)
	const captureConfig = $derived.by(isEditor ? getGcpCaptureConfig : () => ({}))

	export async function openEdit(
		ePath: string,
		isFlow: boolean,
		defaultValues?: Record<string, any>
	) {
		drawerLoading = true
		try {
			drawer?.openDrawer()
			initialPath = ePath
			itemKind = isFlow ? 'flow' : 'script'
			edit = true
			dirtyPath = false
			await loadTrigger(defaultValues)
		} catch (err) {
			sendUserToast(`Could not load GCP Pub/Sub trigger: ${err.body}`, true)
		} finally {
			drawerLoading = false
			if (!defaultValues) {
				initialConfig = structuredClone($state.snapshot(getGcpConfig()))
			}
		}
	}

	export async function openNew(
		nis_flow: boolean,
		fixedScriptPath_?: string,
		defaultValues?: Record<string, any>
	) {
		drawerLoading = true
		try {
			drawer?.openDrawer()
			is_flow = nis_flow
			itemKind = nis_flow ? 'flow' : 'script'
			initialScriptPath = ''
			fixedScriptPath = fixedScriptPath_ ?? ''
			script_path = fixedScriptPath
			gcp_resource_path = defaultValues?.gcp_resource_path ?? ''
			delivery_type = defaultValues?.delivery_type ?? 'pull'
			delivery_config = defaultValues?.delivery_config ?? undefined
			subscription_id = ''
			topic_id = defaultValues?.topic_id ?? ''
			subscription_mode = defaultValues?.subscription_mode ?? 'create_update'
			path = defaultValues?.path ?? ''
			initialPath = ''
			edit = false
			dirtyPath = false
			enabled = defaultValues?.enabled ?? false
			error_handler_path = defaultValues?.error_handler_path ?? undefined
			error_handler_args = defaultValues?.error_handler_args ?? {}
			retry = defaultValues?.retry ?? undefined
			auto_acknowledge_msg = defaultValues?.auto_acknowledge_msg ?? true
			errorHandlerSelected = getHandlerType(error_handler_path ?? '')
		} finally {
			drawerLoading = false
		}
	}

	async function loadTrigger(defaultConfig?: Record<string, any>): Promise<void> {
		if (defaultConfig) {
			loadTriggerConfig(defaultConfig)
			return
		} else {
			try {
				const s = await GcpTriggerService.getGcpTrigger({
					workspace: $workspaceStore!,
					path: initialPath
				})
				loadTriggerConfig(s)
			} catch (error) {
				sendUserToast(`Could not load GCP Pub/Sub trigger: ${error.body}`, true)
			}
		}
	}

	async function loadTriggerConfig(cfg?: Record<string, any>): Promise<void> {
		script_path = cfg?.script_path
		initialScriptPath = cfg?.script_path
		gcp_resource_path = cfg?.gcp_resource_path
		delivery_type = cfg?.delivery_type
		subscription_id = cfg?.subscription_id
		delivery_config = cfg?.delivery_config
		subscription_mode = cfg?.subscription_mode
		is_flow = cfg?.is_flow
		path = cfg?.path
		enabled = cfg?.enabled
		topic_id = cfg?.topic_id ?? ''
		can_write = canWrite(cfg?.path, cfg?.extra_perms, $userStore)
		error_handler_path = cfg?.error_handler_path
		error_handler_args = cfg?.error_handler_args ?? {}
		retry = cfg?.retry
		auto_acknowledge_msg = cfg?.auto_acknowledge_msg ?? true
		errorHandlerSelected = getHandlerType(error_handler_path ?? '')
	}

	async function updateTrigger(): Promise<void> {
		deploymentLoading = true
		const cfg = gcpConfig
		if (!cfg) {
			return
		}
		const isSaved = await saveGcpTriggerFromCfg(
			initialPath,
			cfg,
			edit,
			$workspaceStore!,
			usedTriggerKinds
		)
		if (isSaved) {
			onUpdate?.(cfg.path)
			drawer?.closeDrawer()
		}
		deploymentLoading = false
	}

	function getGcpConfig() {
		return {
			gcp_resource_path,
			subscription_mode,
			subscription_id,
			delivery_type,
			delivery_config,
			base_endpoint,
			topic_id,
			path,
			script_path,
			enabled,
			is_flow,
			error_handler_path,
			error_handler_args,
			retry,
			auto_acknowledge_msg
		}
	}

	function getGcpCaptureConfig() {
		return {
			gcp_resource_path,
			subscription_mode,
			subscription_id,
			delivery_type,
			delivery_config,
			base_endpoint,
			topic_id,
			path
		}
	}

	async function handleToggleEnabled(toggleEnabled: boolean) {
		enabled = toggleEnabled
		if (!trigger?.draftConfig) {
			await GcpTriggerService.setGcpTriggerEnabled({
				path: initialPath,
				workspace: $workspaceStore ?? '',
				requestBody: { enabled: toggleEnabled }
			})
			sendUserToast(`${toggleEnabled ? 'enabled' : 'disabled'} GCP Pub/Sub trigger ${initialPath}`)
		}
	}

	$effect(() => {
		const args = [captureConfig, isValid] as const
		untrack(() => onCaptureConfigChange?.(...args))
	})

	$effect(() => {
		if (!drawerLoading) {
			handleConfigChange(gcpConfig, initialConfig, saveDisabled, edit, onConfigChange)
		}
	})
</script>

{#if useDrawer}
	<Drawer size="800px" bind:this={drawer}>
		<DrawerContent
			title={edit
				? can_write
					? `Edit GCP Pub/Sub trigger ${initialPath}`
					: `GCP Pub/Sub trigger ${initialPath}`
				: 'New GCP Pub/Sub trigger'}
			on:close={drawer?.closeDrawer}
		>
			{#snippet actions()}
				{@render actionsButtons()}
			{/snippet}
			{@render config()}
		</DrawerContent>
	</Drawer>
{:else}
	<Section label={!customLabel ? 'GCP Pub/Sub trigger' : ''} headerClass="grow min-w-0 h-[30px]">
		{#snippet header()}
			{#if customLabel}
				{@render customLabel()}
			{/if}
		{/snippet}
		{#snippet action()}
			{@render actionsButtons()}
		{/snippet}
		{@render config()}
	</Section>
{/if}

{#snippet actionsButtons()}
	{#if !drawerLoading && can_write}
		<TriggerEditorToolbar
			permissions={drawerLoading || !can_write ? 'none' : 'create'}
			{saveDisabled}
			{enabled}
			isLoading={deploymentLoading}
			{edit}
			{allowDraft}
			{isDeployed}
			onUpdate={updateTrigger}
			{onReset}
			{onDelete}
			onToggleEnabled={handleToggleEnabled}
			{cloudDisabled}
			{trigger}
		/>
	{/if}
{/snippet}

{#snippet config()}
	{#if drawerLoading}
		<div class="flex flex-col items-center justify-center h-full w-full">
			<Loader2 size="50" class="animate-spin" />
			<p>Loading...</p>
		</div>
	{:else}
		<div class="flex flex-col gap-5">
			{#if description}
				{@render description()}
			{/if}
			{#if !hideTooltips}
				<Alert title="Info" type="info">
					{#if edit}
						Changes can take up to 30 seconds to take effect.
					{:else}
						New GCP Pub/Sub trigger can take up to 30 seconds to start listening.
					{/if}
				</Alert>
			{/if}
		</div>
		<div class="flex flex-col gap-12 mt-6">
			<div class="flex flex-col gap-4">
				<Label label="Path">
					<Path
						bind:dirty={dirtyPath}
						bind:error={pathError}
						bind:path
						{initialPath}
						checkInitialPathExistence={!edit}
						namePlaceholder="gcp_trigger"
						kind="gcp_trigger"
						disabled={!can_write}
						disableEditing={!can_write}
					/>
				</Label>
			</div>

			{#if !hideTarget}
				<Section label="Runnable">
					<p class="text-xs mb-1 text-tertiary">
						Pick a script or flow to be triggered <Required required={true} />
					</p>
					<div class="flex flex-row mb-2">
						<ScriptPicker
							disabled={fixedScriptPath != '' || !can_write}
							initialPath={fixedScriptPath || initialScriptPath}
							kinds={['script']}
							allowFlow={true}
							bind:itemKind
							bind:scriptPath={script_path}
							allowRefresh={can_write}
							allowEdit={!$userStore?.operator}
							clearable
						/>
						{#if emptyString(script_path)}
							<Button
								btnClasses="ml-4 mt-2"
								color="dark"
								size="xs"
								disabled={!can_write}
								href={itemKind === 'flow' ? '/flows/add?hub=68' : '/scripts/add?hub=hub%2F19662'}
								target="_blank">Create from template</Button
							>
						{/if}
					</div>
				</Section>
			{/if}

			<GcpTriggerEditorConfigSection
				bind:isValid
				bind:gcp_resource_path
				bind:subscription_id
				bind:delivery_type
				bind:delivery_config
				bind:topic_id
				bind:subscription_mode
				bind:auto_acknowledge_msg
				{path}
				cloud_subscription_id={subscription_id}
				create_update_subscription_id={subscription_id}
				{can_write}
				headless={true}
				showTestingBadge={isEditor}
			/>

			<Section label="Advanced" collapsable>
				<div class="flex flex-col gap-4">
					<div class="min-h-96">
						<Tabs bind:selected={optionTabSelected}>
							<Tab value="error_handler">Error Handler</Tab>
							<Tab value="retries">Retries</Tab>
						</Tabs>
						<div class="mt-4">
							<TriggerRetriesAndErrorHandler
								{optionTabSelected}
								{itemKind}
								{can_write}
								bind:errorHandlerSelected
								bind:error_handler_path
								bind:error_handler_args
								bind:retry
							/>
						</div>
					</div>
				</div>
			</Section>
		</div>
	{/if}
{/snippet}
