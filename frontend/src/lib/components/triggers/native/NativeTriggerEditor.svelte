<script lang="ts">
	import { NativeTriggerService } from '$lib/gen/services.gen'
	import type { NativeServiceName } from '$lib/gen/types.gen'
	import type { ExtendedNativeTrigger } from './utils'
	import {
		validateCommonFields,
		getServiceConfig,
		getTemplatePath,
		saveNativeTriggerFromCfg
	} from './utils'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, sendUserToast } from '$lib/utils'
	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import { Loader2, Save } from 'lucide-svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import Section from '$lib/components/Section.svelte'
	import Required from '$lib/components/Required.svelte'
	import NextcloudTriggerForm from './services/nextcloud/NextcloudTriggerForm.svelte'
	import GoogleTriggerForm from './services/google/GoogleTriggerForm.svelte'
	import TriggerEditorToolbar from '$lib/components/triggers/TriggerEditorToolbar.svelte'
	import { handleConfigChange, type Trigger } from '$lib/components/triggers/utils'
	import { deepEqual } from 'fast-equals'
	import type { Snippet } from 'svelte'

	interface Props {
		service: NativeServiceName
		onUpdate?: (path?: string) => void
		useDrawer?: boolean
		hideTarget?: boolean
		allowDraft?: boolean
		trigger?: Trigger
		customLabel?: Snippet
		isDeployed?: boolean
		cloudDisabled?: boolean
		description?: Snippet
		onConfigChange?: (cfg: Record<string, any>, saveDisabled: boolean, updated: boolean) => void
		onDelete?: () => void
		onReset?: () => void
	}

	let {
		service,
		onUpdate,
		useDrawer = true,
		hideTarget = false,
		allowDraft = false,
		trigger = undefined,
		customLabel,
		isDeployed = false,
		cloudDisabled = false,
		description,
		onConfigChange,
		onDelete,
		onReset
	}: Props = $props()

	const serviceInfo = $derived(getServiceConfig(service))
	const scriptTemplateUrl = $derived.by(() => {
		const templateId = getTemplatePath(service, 'script')
		return templateId
	})
	const flowTemplateUrl = $derived.by(() => {
		const templateId = getTemplatePath(service, 'flow')
		return templateId
	})

	let serviceFormRef = $state<any>(null)

	const ServiceFormComponent = $derived.by(() => {
		switch (service) {
			case 'nextcloud':
				return NextcloudTriggerForm
			case 'google':
				return GoogleTriggerForm
			default:
				return null
		}
	})

	let drawer: Drawer | undefined = $state()
	let isNew = $state(false)
	let isRecreate = $state(false)
	let oldExternalIdToDelete = $state<string | null>(null)
	let loading = $state(false)
	let loadingConfig = $state(false)
	let loadingForm = $state(false)
	let showLoading = $state(false)
	let serviceConfig = $state<Record<string, any>>({})
	let externalData = $state<any>(null)
	let errors = $state<Record<string, string>>({})
	let scriptPath = $state('')
	let initialScriptPath = $state('')
	let fixedScriptPath = $state('')
	let isFlow = $state(false)
	let externalId = $state<string | null>(null)
	let can_write = $state(true)
	let originalConfig = $state<Record<string, any> | undefined>(undefined)
	let initialConfig = $state<Record<string, any> | undefined>(undefined)

	export function openNew(
		nis_flow?: boolean,
		fixedScriptPath_?: string,
		defaultValues?: Record<string, any>
	) {
		if (useDrawer) {
			drawer?.openDrawer()
		}
		isNew = true
		isRecreate = false
		oldExternalIdToDelete = null
		serviceConfig = defaultValues ? { ...defaultValues } : {}
		externalData = defaultValues ? { ...defaultValues } : null
		errors = {}
		fixedScriptPath = fixedScriptPath_ ?? ''
		scriptPath = fixedScriptPath
		initialScriptPath = ''
		itemKind = nis_flow ? 'flow' : 'script'
		externalId = null
		loadingConfig = false
		loadingForm = false
		can_write = true
		originalConfig = undefined
		initialConfig = undefined
	}

	export function openRecreate(nativeTrigger: ExtendedNativeTrigger) {
		if (useDrawer) {
			drawer?.openDrawer()
		}
		isNew = true
		isRecreate = true
		oldExternalIdToDelete = nativeTrigger.external_id
		const cachedConfig = (nativeTrigger.service_config as Record<string, any>) || {}
		serviceConfig = { ...cachedConfig }
		// Pass cached config as externalData so the form applies it after loading events
		externalData = { ...cachedConfig }
		errors = {}
		scriptPath = nativeTrigger.script_path
		initialScriptPath = nativeTrigger.script_path
		itemKind = nativeTrigger.is_flow ? 'flow' : 'script'
		externalId = null
		loadingConfig = false
		loadingForm = false
		can_write = true
		originalConfig = undefined
		initialConfig = undefined
	}

	export async function openEdit(
		externalIdOrPath: string,
		nis_flow?: boolean,
		defaultValues?: Record<string, any>
	) {
		let loadingTimeout = setTimeout(() => {
			showLoading = true
		}, 100)
		if (useDrawer) {
			drawer?.openDrawer()
		}
		isNew = false
		isRecreate = false
		oldExternalIdToDelete = null
		externalData = null
		errors = {}
		externalId = externalIdOrPath
		loadingConfig = true
		loadingForm = true
		originalConfig = undefined
		initialConfig = undefined
		itemKind = nis_flow ? 'flow' : 'script'

		try {
			const fullTrigger = await NativeTriggerService.getNativeTrigger({
				workspace: $workspaceStore!,
				serviceName: service,
				externalId: externalIdOrPath
			})

			serviceConfig = (fullTrigger.service_config as Record<string, any>) || {}
			scriptPath = fullTrigger.script_path
			initialScriptPath = fullTrigger.script_path
			can_write = canWrite(fullTrigger.script_path, {}, $userStore)
			externalData = fullTrigger.external_data

			// Apply default values if provided (for draft triggers)
			if (defaultValues) {
				serviceConfig = { ...serviceConfig, ...defaultValues }
				externalData = { ...externalData, ...defaultValues }
			}
		} catch (err: any) {
			sendUserToast(`Failed to load trigger configuration: ${err}`, true)
			externalData = null
		} finally {
			clearTimeout(loadingTimeout)
			loadingConfig = false
			showLoading = false
		}
	}

	function getSaveCfg(): Record<string, any> {
		return {
			script_path: scriptPath,
			is_flow: isFlow,
			service_config: serviceConfig
		}
	}

	// Capture originalConfig after the form has settled (loadingConfig and loadingForm both false)
	// This ensures we compare against the form's normalized config, not raw backend data
	$effect(() => {
		if (!loadingConfig && !loadingForm && originalConfig === undefined && !isNew) {
			originalConfig = structuredClone($state.snapshot(getSaveCfg()))
			initialConfig = structuredClone($state.snapshot(getSaveCfg()))
		}
	})

	function close() {
		drawer?.closeDrawer()
	}

	let validationErrors = $derived.by(() => {
		if (!serviceFormRef) {
			return {}
		}

		const commonErrors = validateCommonFields({
			script_path: scriptPath
		})

		let serviceErrors: Record<string, string> = {}
		if (serviceFormRef?.validate) {
			serviceErrors = serviceFormRef.validate()
		}

		return { ...commonErrors, ...serviceErrors }
	})

	let isValid = $derived.by(() => Object.keys(validationErrors).length === 0)
	let hasChanged = $derived(!deepEqual(getSaveCfg(), originalConfig ?? {}))

	let saveDisabled = $derived(
		loading ||
			!isValid ||
			emptyString(scriptPath) ||
			loadingConfig ||
			loadingForm ||
			!can_write ||
			!hasChanged
	)
	const saveCfg = $derived.by(getSaveCfg)

	$effect(() => {
		if (!loadingConfig && !loadingForm && (isNew || initialConfig)) {
			handleConfigChange(saveCfg, initialConfig, saveDisabled, !isNew, onConfigChange)
		}
	})

	async function save(): Promise<void> {
		loading = true
		const saveCfg = getSaveCfg()
		const newExternalId = await saveNativeTriggerFromCfg(
			service,
			externalId ?? '',
			saveCfg,
			!isNew,
			$workspaceStore!,
			usedTriggerKinds
		)
		if (newExternalId) {
			if (isNew) {
				externalId = newExternalId
				isNew = false
				if (isRecreate && oldExternalIdToDelete) {
					try {
						await NativeTriggerService.deleteNativeTrigger({
							workspace: $workspaceStore!,
							serviceName: service,
							externalId: oldExternalIdToDelete
						})
						sendUserToast(
							`${serviceInfo?.serviceDisplayName} trigger recreated (old trigger deleted)`
						)
					} catch (deleteErr: any) {
						// Still show success for creation, but warn about deletion failure
						sendUserToast(
							`${serviceInfo?.serviceDisplayName} trigger created, but failed to delete old trigger: ${deleteErr.body || deleteErr.message}`,
							true
						)
					}
				}
			}

			onUpdate?.(saveCfg.path)
			originalConfig = structuredClone($state.snapshot(getSaveCfg()))
			initialScriptPath = saveCfg.script_path
			if (useDrawer) {
				close()
			}
		}
		loading = false
	}
	let templateUrl = $derived(isFlow ? flowTemplateUrl : scriptTemplateUrl)
	let itemKind = $state<'flow' | 'script'>('script')

	$effect(() => {
		isFlow = itemKind === 'flow'
	})
</script>

{#if useDrawer}
	<Drawer size="800px" bind:this={drawer}>
		<DrawerContent
			title={isRecreate
				? `Recreate ${serviceInfo?.serviceDisplayName} trigger`
				: isNew
					? `New ${serviceInfo?.serviceDisplayName} trigger`
					: `Edit ${serviceInfo?.serviceDisplayName} trigger`}
			on:close={drawer?.closeDrawer}
		>
			{#snippet actions()}
				{@render drawerActions()}
			{/snippet}
			{@render content()}
		</DrawerContent>
	</Drawer>
{:else}
	<Section
		label={!customLabel ? `${serviceInfo?.serviceDisplayName} trigger` : ''}
		headerClass="grow min-w-0 h-[30px]"
	>
		{#snippet header()}
			{#if customLabel}
				{@render customLabel()}
			{/if}
		{/snippet}
		{#snippet action()}
			{@render inlineActions()}
		{/snippet}
		{@render content()}
	</Section>
{/if}

{#snippet drawerActions()}
	<Button
		size="sm"
		startIcon={{ icon: Save }}
		variant="accent"
		on:click={save}
		disabled={saveDisabled}
		{loading}
	>
		Save
	</Button>
{/snippet}

{#snippet inlineActions()}
	{#if !loadingConfig}
		<TriggerEditorToolbar
			{trigger}
			permissions={loadingConfig || !can_write ? 'none' : 'create'}
			mode="enabled"
			{allowDraft}
			edit={!isNew}
			isLoading={loading}
			{isDeployed}
			{saveDisabled}
			onUpdate={save}
			{onReset}
			{onDelete}
			{cloudDisabled}
			onToggleMode={() => {}}
			disableSuspendedMode={true}
		/>
	{/if}
{/snippet}

{#snippet content()}
	{#if loadingConfig && showLoading}
		<Loader2 class="animate-spin" />
	{:else}
		<div class="flex flex-col gap-4">
			{#if description}
				{@render description()}
			{/if}
		</div>
		<div class="flex flex-col gap-12 mt-6">
			{#if !hideTarget}
				<Section label="Runnable">
					<p class="text-xs mb-1 text-primary">
						Pick a script or flow to be triggered<Required required={true} />
					</p>
					<div class="flex flex-row mb-2">
						<ScriptPicker
							disabled={fixedScriptPath != '' || !can_write}
							initialPath={fixedScriptPath || initialScriptPath}
							bind:scriptPath
							allowRefresh={can_write}
							bind:itemKind
							kinds={['script']}
							allowFlow={true}
							allowEdit={!$userStore?.operator}
							clearable
						/>
						{#if emptyString(scriptPath)}
							<Button
								btnClasses="ml-4"
								variant="accent"
								size="xs"
								href={templateUrl}
								target="_blank"
								disabled={!can_write}
							>
								Create from template
							</Button>
						{/if}
					</div>
					{#if errors.runnable_path}
						<div class="text-red-500 text-xs mt-1">{errors.runnable_path}</div>
					{/if}
				</Section>
			{/if}

			{#if loadingConfig}
				<Section label="{serviceInfo?.serviceDisplayName} configuration">
					<div class="flex items-center gap-2 text-secondary text-xs">
						<Loader2 class="animate-spin" size={16} />
						Loading configuration from {serviceInfo?.serviceDisplayName}...
					</div>
				</Section>
			{:else if ServiceFormComponent}
				<ServiceFormComponent
					bind:loading={loadingForm}
					bind:this={serviceFormRef}
					bind:serviceConfig
					bind:errors
					{externalData}
					disabled={loading || loadingConfig || !can_write}
					path={scriptPath}
					{isFlow}
					token=""
					triggerTokens={undefined}
					scopes={[]}
				/>
			{:else}
				<Section label="{serviceInfo?.serviceDisplayName} configuration">
					<div class="text-red-500 text-xs space-y-2">
						<div>Failed to load service configuration component for {service}.</div>
						<div class="text-secondary">
							Ensure your workspace has a connected {serviceInfo?.serviceDisplayName} integration.
						</div>
						<Button
							unifiedSize="xs"
							variant="default"
							href="/workspace_settings?tab=integrations"
							target="_blank"
						>
							Manage integrations
						</Button>
					</div>
				</Section>
			{/if}
		</div>
	{/if}
{/snippet}
