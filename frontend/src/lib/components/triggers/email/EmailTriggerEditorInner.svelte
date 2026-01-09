<script lang="ts">
	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import {
		EmailTriggerService,
		type ErrorHandler,
		type EmailTrigger,
		type NewEmailTrigger,
		type Retry,
		type TriggerMode
	} from '$lib/gen'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, capitalize, emptyString, sendUserToast } from '$lib/utils'
	import Section from '$lib/components/Section.svelte'
	import { Loader2 } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import EmailTriggerEditorConfigSection from './EmailTriggerEditorConfigSection.svelte'
	import TriggerEditorToolbar from '../TriggerEditorToolbar.svelte'
	import { getHandlerType, handleConfigChange } from '../utils'
	import { untrack } from 'svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import TriggerRetriesAndErrorHandler from '../TriggerRetriesAndErrorHandler.svelte'
	import { saveEmailTriggerFromCfg } from './utils'
	import { deepEqual } from 'fast-equals'
	import TriggerSuspendedJobsAlert from '../TriggerSuspendedJobsAlert.svelte'
	import TriggerSuspendedJobsModal from '../TriggerSuspendedJobsModal.svelte'

	let {
		useDrawer = true,
		hideTarget = false,
		description = undefined,
		isEditor = false,
		customLabel = undefined,
		allowDraft = false,
		isDeployed = false,
		onConfigChange = undefined,
		onCaptureConfigChange = undefined,
		onUpdate = undefined,
		onDelete = undefined,
		onReset = undefined,
		trigger = undefined,
		customSaveBehavior = undefined
	} = $props()

	// Form data state
	let initialPath = $state('')
	let edit = $state(true)
	let itemKind = $state<'flow' | 'script'>('script')
	let is_flow = $state(false)
	let script_path = $state('')
	let initialScriptPath = $state('')
	let fixedScriptPath = $state('')
	let path = $state('')
	let pathError = $state('')
	let isValid = $state(false)
	let dirtyLocalPart = $state(false)
	let dirtyPath = $state(false)
	let local_part = $state('')
	let workspaced_local_part = $state(false)
	let drawerLoading = $state(true)
	let showLoader = $state(false)
	let can_write = $state(true)
	let extraPerms = $state<Record<string, boolean> | undefined>(undefined)
	let error_handler_path: string | undefined = $state()
	let error_handler_args: Record<string, any> = $state({})
	let retry: Retry | undefined = $state()
	let mode = $state<TriggerMode>('enabled')
	// Component references
	let drawer = $state<Drawer | undefined>(undefined)
	let initialConfig: NewEmailTrigger | undefined = undefined
	let deploymentLoading = $state(false)
	let optionTabSelected: 'error_handler' | 'retries' = $state('error_handler')
	let errorHandlerSelected: ErrorHandler = $state('slack')

	let suspendedJobsModal = $state<TriggerSuspendedJobsModal | null>(null)
	let originalConfig = $state<NewEmailTrigger | undefined>(undefined)

	let hasChanged = $derived(!deepEqual(getEmailTriggerConfig(), originalConfig ?? {}))
	const isAdmin = $derived($userStore?.is_admin || $userStore?.is_super_admin)
	const emailConfig = $derived.by(getEmailTriggerConfig)
	const captureConfig = $derived.by(isEditor ? getCaptureConfig : () => ({}))
	const saveDisabled = $derived(
		drawerLoading ||
			!can_write ||
			pathError != '' ||
			!isValid ||
			emptyString(script_path) ||
			!hasChanged
	)

	$effect(() => {
		is_flow = itemKind === 'flow'
	})

	export async function openEdit(
		ePath: string,
		isFlow: boolean,
		defaultConfig?: Partial<NewEmailTrigger>
	) {
		drawerLoading = true
		let loader = setTimeout(() => {
			showLoader = true
		}, 100) // if loading takes less than 100ms, we don't show the loader
		try {
			drawer?.openDrawer()
			initialPath = ePath
			path = ePath
			itemKind = isFlow ? 'flow' : 'script'
			edit = true
			dirtyPath = false
			dirtyLocalPart = false
			await loadTrigger(defaultConfig)
			originalConfig = structuredClone($state.snapshot(getEmailTriggerConfig()))
		} catch (err) {
			sendUserToast(`Could not load email trigger: ${err}`, true)
		} finally {
			if (!defaultConfig) {
				// If the email trigger is loaded from the backend, we to set the initial config
				initialConfig = structuredClone($state.snapshot(getEmailTriggerConfig()))
			}
			clearTimeout(loader)
			drawerLoading = false
			showLoader = false
		}
	}

	export async function openNew(
		nis_flow: boolean,
		fixedScriptPath_?: string,
		defaultValues?: Partial<EmailTrigger>
	) {
		drawerLoading = true
		let loader = setTimeout(() => {
			showLoader = true
		}, 100) // if loading takes less than 100ms, we don't show the loader
		try {
			drawer?.openDrawer()
			is_flow = defaultValues?.is_flow ?? nis_flow
			edit = false
			itemKind = nis_flow ? 'flow' : 'script'
			local_part = defaultValues?.local_part ?? ''
			dirtyLocalPart = false
			initialScriptPath = ''
			fixedScriptPath = fixedScriptPath_ ?? ''
			script_path = fixedScriptPath
			path = defaultValues?.path ?? ''
			initialPath = ''
			dirtyPath = false

			workspaced_local_part = defaultValues?.workspaced_local_part ?? false
			error_handler_path = defaultValues?.error_handler_path ?? undefined
			error_handler_args = defaultValues?.error_handler_args ?? {}
			retry = defaultValues?.retry ?? undefined
			errorHandlerSelected = getHandlerType(error_handler_path ?? '')
			mode = defaultValues?.mode ?? 'enabled'
			originalConfig = undefined
		} finally {
			clearTimeout(loader)
			drawerLoading = false
			showLoader = false
		}
	}

	function loadTriggerConfig(cfg?: Partial<EmailTrigger>): void {
		script_path = cfg?.script_path ?? ''
		initialScriptPath = cfg?.script_path ?? ''
		is_flow = cfg?.is_flow ?? false
		path = cfg?.path ?? ''
		local_part = cfg?.local_part ?? ''
		workspaced_local_part = cfg?.workspaced_local_part ?? false
		extraPerms = cfg?.extra_perms ?? undefined
		can_write = canWrite(path, cfg?.extra_perms ?? {}, $userStore)
		error_handler_path = cfg?.error_handler_path
		error_handler_args = cfg?.error_handler_args ?? {}
		retry = cfg?.retry
		errorHandlerSelected = getHandlerType(error_handler_path ?? '')
		mode = cfg?.mode ?? 'enabled'
	}

	async function loadTrigger(defaultConfig?: Partial<EmailTrigger>): Promise<void> {
		if (defaultConfig) {
			loadTriggerConfig(defaultConfig)
			return
		} else {
			const s = await EmailTriggerService.getEmailTrigger({
				workspace: $workspaceStore!,
				path: initialPath
			})

			loadTriggerConfig(s)
		}
	}

	async function triggerScript(): Promise<void> {
		if (customSaveBehavior) {
			customSaveBehavior(emailConfig)
			drawer?.closeDrawer()
		} else {
			deploymentLoading = true
			const saveCfg = emailConfig
			const isSaved = await saveEmailTriggerFromCfg(
				initialPath,
				saveCfg,
				edit,
				$workspaceStore!,
				!!$userStore?.is_admin || !!$userStore?.is_super_admin,
				usedTriggerKinds
			)
			if (isSaved) {
				onUpdate(saveCfg.path)
				originalConfig = structuredClone($state.snapshot(getEmailTriggerConfig()))
				initialPath = saveCfg.path
				initialScriptPath = saveCfg.script_path
				if (mode !== 'suspended') {
					drawer?.closeDrawer()
				}
			}
			deploymentLoading = false
		}
	}

	function getEmailTriggerConfig(): NewEmailTrigger {
		const nCfg = {
			script_path,
			is_flow,
			path,
			local_part,
			workspaced_local_part,
			extra_perms: extraPerms,
			error_handler_path,
			error_handler_args,
			retry,
			mode
		}

		return nCfg
	}

	async function handleToggleMode(newMode: TriggerMode) {
		mode = newMode
		if (!trigger?.draftConfig) {
			await EmailTriggerService.setEmailTriggerMode({
				path: initialPath,
				workspace: $workspaceStore ?? '',
				requestBody: { mode: newMode }
			})
			sendUserToast(`${capitalize(newMode)} email trigger ${initialPath}`)

			onUpdate(initialPath)
		}
		if (originalConfig) {
			originalConfig['mode'] = newMode
		}
	}

	// Update config for captures
	function getCaptureConfig() {
		const newCaptureConfig = {
			local_part: emailConfig.local_part,
			path: emailConfig.path
		}
		//
		return newCaptureConfig
	}

	$effect(() => {
		const args = [captureConfig, isValid] as const
		untrack(() => onCaptureConfigChange?.(...args))
	})

	$effect(() => {
		if (!drawerLoading) {
			handleConfigChange(emailConfig, initialConfig, saveDisabled, edit, onConfigChange)
		}
	})
</script>

{#if mode === 'suspended'}
	<TriggerSuspendedJobsModal
		bind:this={suspendedJobsModal}
		triggerPath={path}
		triggerKind="email"
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

{#snippet config()}
	{#if drawerLoading}
		{#if showLoader}
			<Loader2 class="animate-spin" />
		{/if}
	{:else}
		<div class="flex flex-col gap-12">
			{#if mode === 'suspended'}
				<TriggerSuspendedJobsAlert {suspendedJobsModal} />
			{/if}
			<Section label="Metadata">
				<div class="flex flex-col gap-2">
					<Label label="Path">
						<Path
							bind:dirty={dirtyPath}
							bind:error={pathError}
							bind:path
							{initialPath}
							checkInitialPathExistence={!edit}
							namePlaceholder="email_trigger"
							kind="email_trigger"
							hideUser
							disableEditing={!can_write}
						/>
					</Label>
				</div>
			</Section>

			{#if !hideTarget}
				<Section label="Target">
					<p class="text-xs mt-3 mb-1 text-primary">
						Pick a script or flow to be triggered<Required required={true} />
					</p>
					<div class="flex flex-col gap-2">
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
									href={itemKind === 'flow' ? '/flows/add?hub=72' : '/scripts/add?hub=hub%2F19813'}
									target="_blank">Create from template</Button
								>
							{/if}
						</div>
					</div>
				</Section>
			{/if}

			<EmailTriggerEditorConfigSection
				initialTriggerPath={initialPath}
				bind:local_part
				bind:isValid
				bind:workspaced_local_part
				bind:dirtyLocalPart
				{can_write}
				showTestingBadge={isEditor}
				isDraftOnly={trigger ? trigger.isDraft : false}
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

{#snippet saveButton()}
	{#if !drawerLoading}
		<TriggerEditorToolbar
			{trigger}
			permissions={drawerLoading || !can_write ? 'none' : can_write && isAdmin ? 'create' : 'write'}
			{saveDisabled}
			{allowDraft}
			{edit}
			isLoading={deploymentLoading}
			onUpdate={triggerScript}
			{onReset}
			{onDelete}
			{isDeployed}
			{mode}
			onToggleMode={handleToggleMode}
			{suspendedJobsModal}
		/>
	{/if}
{/snippet}

{#if useDrawer}
	<Drawer size="700px" bind:this={drawer}>
		<DrawerContent
			title={edit
				? can_write
					? `Edit email trigger ${initialPath}`
					: `Email trigger ${initialPath}`
				: 'New email trigger'}
			on:close={() => drawer?.closeDrawer()}
		>
			{#snippet actions()}
				{@render saveButton()}
			{/snippet}
			{@render config()}
		</DrawerContent>
	</Drawer>
{:else}
	<Section label={!customLabel ? 'Email trigger' : ''} headerClass="grow min-w-0 h-[30px]">
		{#snippet header()}
			{#if customLabel}
				{@render customLabel()}
			{/if}
		{/snippet}
		{#snippet action()}
			{@render saveButton()}
		{/snippet}
		{#if description}
			{@render description()}
		{/if}
		{@render config()}
	</Section>
{/if}
