<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, capitalize, emptyString, sendUserToast } from '$lib/utils'
	import { Loader2 } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import {
		AzureTriggerService,
		type AzureMode,
		type Retry,
		type ErrorHandler,
		type TriggerMode
	} from '$lib/gen'
	import Section from '$lib/components/Section.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import Required from '$lib/components/Required.svelte'
	import AzureTriggerEditorConfigSection from './AzureTriggerEditorConfigSection.svelte'
	import { type Snippet, untrack } from 'svelte'
	import TriggerEditorToolbar from '../TriggerEditorToolbar.svelte'
	import PermissionedAsLine from '../PermissionedAsLine.svelte'
	import { saveAzureTriggerFromCfg } from './utils'
	import { getHandlerType, handleConfigChange, type Trigger } from '../utils'
	import { deepEqual } from 'fast-equals'
	import TriggerSuspendedJobsAlert from '../TriggerSuspendedJobsAlert.svelte'
	import TriggerSuspendedJobsModal from '../TriggerSuspendedJobsModal.svelte'
	import { base } from '$lib/base'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import TriggerRetriesAndErrorHandler from '../TriggerRetriesAndErrorHandler.svelte'
	import TriggerAdvancedBadges from '../TriggerAdvancedBadges.svelte'

	let drawer: Drawer | undefined = $state(undefined)
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

	let azure_resource_path: string = $state('')
	let azure_mode: AzureMode = $state('namespace_pull')
	let scope_resource_id: string = $state('')
	let topic_name: string | undefined = $state(undefined)
	let subscription_name: string = $state('')
	let event_type_filters: string[] | undefined = $state(undefined)

	let isValid = $state(false)
	let initialConfig: Record<string, any> | undefined = undefined
	let deploymentLoading = $state(false)
	let permissionedAs = $state<string | undefined>(undefined)
	let selectedPermissionedAs = $state<string | undefined>(undefined)
	let preservePermissionedAs = $state(false)
	let base_endpoint = $derived(`${window.location.origin}${base}`)

	let optionTabSelected: 'error_handler' | 'retries' = $state('error_handler')
	let errorHandlerSelected: ErrorHandler = $state('slack')
	let error_handler_path: string | undefined = $state()
	let error_handler_args: Record<string, any> = $state({})
	let retry: Retry | undefined = $state()
	let suspendedJobsModal = $state<TriggerSuspendedJobsModal | null>(null)
	let originalConfig = $state<Record<string, any> | undefined>(undefined)

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

	let hasChanged = $derived(!deepEqual(getAzureConfig(), originalConfig ?? {}))
	const azureConfig = $derived.by(getAzureConfig)
	const saveDisabled = $derived(
		pathError != '' || emptyString(script_path) || !isValid || !can_write || !hasChanged
	)
	const captureConfig = $derived.by(untrack(() => isEditor) ? getAzureCaptureConfig : () => ({}))

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
			originalConfig = structuredClone($state.snapshot(getAzureConfig()))
		} catch (err) {
			sendUserToast(`Could not load Azure trigger: ${err.body}`, true)
		} finally {
			drawerLoading = false
			if (!defaultValues) {
				initialConfig = structuredClone($state.snapshot(getAzureConfig()))
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
			itemKind = nis_flow ? 'flow' : 'script'
			initialScriptPath = ''
			fixedScriptPath = fixedScriptPath_ ?? ''
			script_path = fixedScriptPath
			azure_resource_path = defaultValues?.azure_resource_path ?? ''
			azure_mode = defaultValues?.azure_mode ?? 'namespace_pull'
			scope_resource_id = defaultValues?.scope_resource_id ?? ''
			topic_name = defaultValues?.topic_name ?? undefined
			subscription_name = defaultValues?.subscription_name ?? ''
			event_type_filters = defaultValues?.event_type_filters ?? undefined
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
			drawerLoading = false
		}
	}

	async function loadTrigger(defaultConfig?: Record<string, any>): Promise<void> {
		if (defaultConfig) {
			loadTriggerConfig(defaultConfig)
			return
		}
		try {
			const s = await AzureTriggerService.getAzureTrigger({
				workspace: $workspaceStore!,
				path: initialPath
			})
			loadTriggerConfig(s)
		} catch (error) {
			sendUserToast(`Could not load Azure trigger: ${error.body}`, true)
		}
	}

	async function loadTriggerConfig(cfg?: Record<string, any>): Promise<void> {
		script_path = cfg?.script_path
		initialScriptPath = cfg?.script_path
		azure_resource_path = cfg?.azure_resource_path
		azure_mode = cfg?.azure_mode
		scope_resource_id = cfg?.scope_resource_id
		topic_name = cfg?.topic_name ?? undefined
		subscription_name = cfg?.subscription_name
		event_type_filters = cfg?.event_type_filters
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
	}

	async function updateTrigger(): Promise<void> {
		deploymentLoading = true
		const cfg = azureConfig
		if (!cfg) return
		const isSaved = await saveAzureTriggerFromCfg(
			initialPath,
			cfg,
			edit,
			$workspaceStore!,
			usedTriggerKinds
		)
		if (isSaved) {
			onUpdate?.(cfg.path)
			originalConfig = structuredClone($state.snapshot(getAzureConfig()))
			initialPath = cfg.path
			initialScriptPath = cfg.script_path
			if (mode !== 'suspended') drawer?.closeDrawer()
		}
		deploymentLoading = false
	}

	function getAzureConfig() {
		return {
			azure_resource_path,
			azure_mode,
			scope_resource_id,
			topic_name,
			subscription_name,
			event_type_filters,
			base_endpoint,
			path,
			script_path,
			mode,
			is_flow: itemKind === 'flow',
			error_handler_path,
			error_handler_args,
			retry,
			permissioned_as: selectedPermissionedAs,
			preserve_permissioned_as: preservePermissionedAs || undefined
		}
	}

	function getAzureCaptureConfig() {
		return {
			azure_resource_path,
			azure_mode,
			scope_resource_id,
			topic_name,
			subscription_name,
			event_type_filters,
			base_endpoint,
			path
		}
	}

	async function handleToggleMode(newMode: TriggerMode) {
		mode = newMode
		if (!trigger?.draftConfig) {
			await AzureTriggerService.setAzureTriggerMode({
				path: initialPath,
				workspace: $workspaceStore ?? '',
				requestBody: { mode: newMode }
			})
			sendUserToast(`${capitalize(newMode)} Azure trigger ${initialPath}`)
			onUpdate?.(initialPath)
		}
		if (originalConfig) originalConfig['mode'] = newMode
	}

	$effect(() => {
		if (!drawerLoading) {
			handleConfigChange(azureConfig, initialConfig, saveDisabled, edit, onConfigChange)
		}
	})

	$effect(() => {
		const args = [captureConfig, isValid] as const
		untrack(() => onCaptureConfigChange?.(...args))
	})
</script>

{#if mode === 'suspended'}
	<TriggerSuspendedJobsModal
		bind:this={suspendedJobsModal}
		triggerPath={path}
		triggerKind="azure"
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
			title={edit
				? can_write
					? `Edit Azure trigger ${initialPath}`
					: `Azure trigger ${initialPath}`
				: 'New Azure trigger'}
			on:close={drawer?.closeDrawer}
		>
			{#snippet actions()}
				{@render actionsButtons()}
			{/snippet}
			{@render config()}
		</DrawerContent>
	</Drawer>
{:else}
	<Section
		label={!customLabel ? 'Azure Event Grid trigger' : ''}
		headerClass="grow min-w-0 h-[30px]"
	>
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
			{mode}
			isLoading={deploymentLoading}
			{edit}
			{allowDraft}
			{isDeployed}
			onUpdate={updateTrigger}
			{onReset}
			{onDelete}
			onToggleMode={handleToggleMode}
			{cloudDisabled}
			{trigger}
			{suspendedJobsModal}
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
		<PermissionedAsLine
			{permissionedAs}
			{path}
			onPermissionedAsChange={(pa, preserve) => {
				selectedPermissionedAs = pa
				preservePermissionedAs = preserve
			}}
		/>
		<div class="flex flex-col gap-5">
			{#if mode === 'suspended'}
				<TriggerSuspendedJobsAlert {suspendedJobsModal} />
			{/if}
			{#if description}
				{@render description()}
			{/if}
			{#if !hideTooltips}
				<Alert title="Info" type="info">
					{#if edit}
						Changes can take up to 30 seconds to take effect.
					{:else}
						New Azure triggers can take up to 30 seconds to start listening.
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
						namePlaceholder="azure_trigger"
						kind="azure_trigger"
						disabled={!can_write}
						disableEditing={!can_write}
					/>
				</Label>
			</div>

			{#if !hideTarget}
				<Section label="Runnable">
					<p class="text-xs mb-1 text-primary">
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
								btnClasses="ml-4"
								variant="default"
								unifiedSize="md"
								disabled={!can_write}
								href={itemKind === 'flow' ? '/flows/add?hub=81' : '/scripts/add?hub=hub%2F28214'}
								target="_blank">Create from template</Button
							>
						{/if}
					</div>
				</Section>
			{/if}

			<AzureTriggerEditorConfigSection
				bind:isValid
				bind:azure_resource_path
				bind:azure_mode
				bind:scope_resource_id
				bind:topic_name
				bind:subscription_name
				bind:event_type_filters
				{path}
				{can_write}
				headless={true}
			/>

			<Section label="Advanced" collapsable>
				{#snippet header()}
					<TriggerAdvancedBadges {error_handler_path} {retry} />
				{/snippet}
				<div class="min-h-96">
					<Tabs bind:selected={optionTabSelected}>
						<Tab value="error_handler" label="Error Handler" />
						<Tab value="retries" label="Retries" />
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
			</Section>
			<div class="pb-8" />
		</div>
	{/if}
{/snippet}
