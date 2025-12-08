<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import { NatsTriggerService, type ErrorHandler, type Retry, type TriggerMode } from '$lib/gen'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, capitalize, emptyString, sendUserToast } from '$lib/utils'
	import Section from '$lib/components/Section.svelte'
	import { Loader2 } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import NatsTriggersConfigSection from './NatsTriggersConfigSection.svelte'
	import { untrack, type Snippet } from 'svelte'
	import TriggerEditorToolbar from '../TriggerEditorToolbar.svelte'
	import { saveNatsTriggerFromCfg } from './utils'
	import { getHandlerType, handleConfigChange, type Trigger } from '../utils'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import TriggerRetriesAndErrorHandler from '../TriggerRetriesAndErrorHandler.svelte'
	import { deepEqual } from 'fast-equals'
	import TriggerSuspendedJobsAlert from '../TriggerSuspendedJobsAlert.svelte'
	import TriggerSuspendedJobsModal from '../TriggerSuspendedJobsModal.svelte'

	interface Props {
		useDrawer?: boolean
		description?: Snippet | undefined
		hideTarget?: boolean
		hideTooltips?: boolean
		useEditButton?: boolean
		isEditor?: boolean
		allowDraft?: boolean
		trigger?: Trigger
		isDeployed?: boolean
		cloudDisabled?: boolean
		customLabel?: Snippet
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
		isDeployed = false,
		cloudDisabled = false,
		customLabel = undefined,
		onConfigChange = undefined,
		onCaptureConfigChange = undefined,
		onUpdate = undefined,
		onDelete = undefined,
		onReset = undefined
	}: Props = $props()

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
	let dirtyPath = $state(false)
	let can_write = $state(true)
	let drawerLoading = $state(true)
	let showLoading = $state(false)
	let defaultValues: Record<string, any> | undefined = $state(undefined)
	let natsResourcePath = $state('')
	let initialConfig: Record<string, any> | undefined = undefined
	let originalConfig = $state<Record<string, any> | undefined>(undefined)
	let natsCfg: {
		subjects: string[]
		use_jetstream: boolean
		stream_name?: string
		consumer_name?: string
	} = $state({
		subjects: [],
		use_jetstream: false
	})
	let deploymentLoading = $state(false)
	let isValid = $state(false)
	let optionTabSelected: 'error_handler' | 'retries' = $state('error_handler')
	let errorHandlerSelected: ErrorHandler = $state('slack')
	let error_handler_path: string | undefined = $state()
	let error_handler_args: Record<string, any> = $state({})
	let retry: Retry | undefined = $state()
	let mode = $state<TriggerMode>('enabled')

	let suspendedJobsModal = $state<TriggerSuspendedJobsModal | null>(null)

	let hasChanged = $derived(!deepEqual(getSaveCfg(), originalConfig ?? {}))

	const saveDisabled = $derived(
		pathError != '' || emptyString(script_path) || !can_write || !isValid || !hasChanged
	)
	const natsConfig = $derived.by(getSaveCfg)
	const captureConfig = $derived.by(isEditor ? getCaptureConfig : () => ({}))

	$effect(() => {
		is_flow = itemKind === 'flow'
	})

	export async function openEdit(
		ePath: string,
		isFlow: boolean,
		defaultConfig?: Record<string, any>
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
			await loadTrigger(defaultConfig)
		} catch (err) {
			sendUserToast(`Could not load nats trigger: ${err}`, true)
		} finally {
			clearTimeout(loadingTimeout)
			drawerLoading = false
			showLoading = false
			if (!defaultConfig) {
				initialConfig = structuredClone($state.snapshot(getSaveCfg()))
			}
			originalConfig = structuredClone($state.snapshot(getSaveCfg()))
		}
	}

	export async function openNew(
		nis_flow: boolean,
		fixedScriptPath_?: string,
		nDefaultValues?: Record<string, any>
	) {
		let loadingTimeout = setTimeout(() => {
			showLoading = true
		}, 100)
		drawerLoading = true
		try {
			drawer?.openDrawer()
			is_flow = nis_flow
			edit = false
			itemKind = nis_flow ? 'flow' : 'script'
			natsResourcePath = nDefaultValues?.nats_resource_path ?? ''
			const useJetstream = nDefaultValues?.use_jetstream ?? false
			natsCfg = {
				subjects: nDefaultValues?.subjects ?? [''],
				use_jetstream: useJetstream,
				stream_name: useJetstream ? (nDefaultValues?.stream_name ?? '') : undefined,
				consumer_name: useJetstream ? (nDefaultValues?.consumer_name ?? '') : undefined
			}
			initialScriptPath = ''
			fixedScriptPath = fixedScriptPath_ ?? ''
			script_path = fixedScriptPath
			path = nDefaultValues?.path ?? ''
			initialPath = ''
			dirtyPath = false
			defaultValues = nDefaultValues
			mode = nDefaultValues?.mode ?? 'enabled'
			error_handler_path = nDefaultValues?.error_handler_path ?? undefined
			error_handler_args = nDefaultValues?.error_handler_args ?? {}
			retry = nDefaultValues?.retry ?? undefined
			errorHandlerSelected = getHandlerType(error_handler_path ?? '')
			originalConfig = undefined
		} finally {
			clearTimeout(loadingTimeout)
			drawerLoading = false
			showLoading = false
		}
	}

	async function loadTriggerConfig(cfg?: Record<string, any>): Promise<void> {
		script_path = cfg?.script_path
		initialScriptPath = cfg?.script_path
		is_flow = cfg?.is_flow
		path = cfg?.path
		natsResourcePath = cfg?.nats_resource_path
		const useJetstream = cfg?.use_jetstream || false
		natsCfg = {
			subjects: cfg?.subjects || [''],
			use_jetstream: useJetstream,
			stream_name: useJetstream ? cfg?.stream_name || '' : undefined,
			consumer_name: useJetstream ? cfg?.consumer_name || '' : undefined
		}
		mode = cfg?.mode ?? 'enabled'
		can_write = canWrite(cfg?.path, cfg?.extra_perms, $userStore)
		error_handler_path = cfg?.error_handler_path
		error_handler_args = cfg?.error_handler_args ?? {}
		retry = cfg?.retry
		errorHandlerSelected = getHandlerType(error_handler_path ?? '')
	}

	async function loadTrigger(defaultConfig?: Record<string, any>): Promise<void> {
		if (defaultConfig) {
			loadTriggerConfig(defaultConfig)
			return
		} else {
			const s = await NatsTriggerService.getNatsTrigger({
				workspace: $workspaceStore!,
				path: initialPath
			})
			loadTriggerConfig(s)
		}
	}

	function getSaveCfg() {
		return {
			path,
			script_path,
			is_flow,
			mode,
			nats_resource_path: natsResourcePath,
			stream_name: natsCfg.stream_name,
			consumer_name: natsCfg.consumer_name,
			subjects: natsCfg.subjects,
			use_jetstream: natsCfg.use_jetstream,
			error_handler_path,
			error_handler_args,
			retry
		}
	}

	async function updateTrigger(): Promise<void> {
		deploymentLoading = true
		const cfg = natsConfig
		const isSaved = await saveNatsTriggerFromCfg(
			initialPath,
			cfg,
			edit,
			$workspaceStore!,
			usedTriggerKinds
		)
		if (isSaved) {
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

	function useDefaultValues() {
		if (natsResourcePath && natsResourcePath != '') {
			return false
		}
		if (!defaultValues) {
			return false
		}
		return (
			defaultValues.servers &&
			defaultValues.servers.length > 0 &&
			defaultValues.servers.some((broker: string) => broker.trim() !== '')
		)
	}

	async function handleToggleMode(newMode: TriggerMode) {
		mode = newMode
		if (!trigger?.draftConfig) {
			await NatsTriggerService.setNatsTriggerMode({
				path: initialPath,
				workspace: $workspaceStore ?? '',
				requestBody: { mode: newMode }
			})
			sendUserToast(`${capitalize(newMode)} NATS trigger ${initialPath}`)

			onUpdate?.(initialPath)
		}
		if (originalConfig) {
			originalConfig['mode'] = newMode
		}
	}

	function getCaptureConfig() {
		const { nats_resource_path, subjects, stream_name, consumer_name, use_jetstream } = natsConfig
		return { nats_resource_path, subjects, stream_name, consumer_name, use_jetstream }
	}

	$effect(() => {
		const args = [captureConfig, isValid] as const
		untrack(() => onCaptureConfigChange?.(...args))
	})

	$effect(() => {
		!drawerLoading &&
			handleConfigChange(natsConfig, initialConfig, saveDisabled, edit, onConfigChange)
	})
</script>

{#if mode === 'suspended'}
	<TriggerSuspendedJobsModal
		bind:this={suspendedJobsModal}
		triggerPath={path}
		triggerKind="nats"
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
					? `Edit NATS trigger ${initialPath}`
					: `NATS trigger ${initialPath}`
				: 'New NATS trigger'}
			on:close={drawer.closeDrawer}
		>
			{#snippet actions()}
				{@render actionsSnippet()}
			{/snippet}
			{@render config()}
		</DrawerContent>
	</Drawer>
{:else}
	<Section label={!customLabel ? 'NATS trigger' : ''} headerClass="grow min-w-0 h-[30px]">
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
			{allowDraft}
			{edit}
			isLoading={deploymentLoading}
			{isDeployed}
			onUpdate={updateTrigger}
			{onReset}
			{onDelete}
			{cloudDisabled}
			{mode}
			onToggleMode={handleToggleMode}
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
		<div class="flex flex-col gap-4">
			{#if description}
				{@render description()}
			{/if}
			{#if !hideTooltips}
				<Alert title="Info" type="info" size="xs">
					{#if edit}
						Changes can take up to 30 seconds to take effect.
					{:else}
						NATS consumers can take up to 30 seconds to start.
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
						bind:dirty={dirtyPath}
						bind:error={pathError}
						bind:path
						{initialPath}
						checkInitialPathExistence={!edit}
						namePlaceholder="nats_trigger"
						kind="nats_trigger"
						disabled={!can_write}
					/>
				</Label>
			</div>
			{#if !hideTarget}
				<Section label="Runnable">
					<p class="text-xs mb-1 text-primary">
						Pick a script or flow to be triggered<Required required={true} />
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
								variant="accent"
								size="xs"
								href={itemKind === 'flow' ? '/flows/add?hub=66' : '/scripts/add?hub=hub%2F19663'}
								target="_blank"
							>
								Create from template
							</Button>
						{/if}
					</div>
				</Section>
			{/if}

			<NatsTriggersConfigSection
				{path}
				bind:natsResourcePath
				bind:natsCfg
				on:valid-config={({ detail }) => {
					isValid = detail
				}}
				defaultValues={useDefaultValues() ? defaultValues : undefined}
				{can_write}
				showTestingBadge={isEditor}
			/>

			<Section label="Advanced" collapsable>
				<div class="flex flex-col gap-4">
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
				</div>
			</Section>
		</div>
	{/if}
{/snippet}
