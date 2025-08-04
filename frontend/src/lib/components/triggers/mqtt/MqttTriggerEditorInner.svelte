<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, sendUserToast } from '$lib/utils'
	import Section from '$lib/components/Section.svelte'
	import { Loader2 } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import {
		MqttTriggerService,
		type MqttClientVersion,
		type MqttV3Config,
		type MqttV5Config,
		type MqttSubscribeTopic,
		type Retry,
		type ErrorHandler
	} from '$lib/gen'
	import MqttEditorConfigSection from './MqttEditorConfigSection.svelte'
	import type { Snippet } from 'svelte'
	import TriggerEditorToolbar from '../TriggerEditorToolbar.svelte'
	import { saveMqttTriggerFromCfg } from './utils'
	import { getHandlerType, handleConfigChange, type Trigger } from '../utils'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import TriggerRetriesAndErrorHandler from '../TriggerRetriesAndErrorHandler.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { DEFAULT_V3_CONFIG, DEFAULT_V5_CONFIG } from './constant'

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

	let mqtt_resource_path: string = $state('')
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
	let enabled = $state(false)
	let dirtyPath = $state(false)
	let can_write = $state(true)
	let drawerLoading = $state(true)
	let showLoading = $state(false)
	let subscribe_topics: MqttSubscribeTopic[] = $state([])
	let v3_config: MqttV3Config = $state(DEFAULT_V3_CONFIG)
	let v5_config: MqttV5Config = $state(DEFAULT_V5_CONFIG)
	let client_version: MqttClientVersion | undefined = $state()
	let client_id: string | undefined = $state('')
	let isValid: boolean = $state(false)
	let initialConfig: Record<string, any> | undefined = {}
	let deploymentLoading = $state(false)
	let errorHandlerSelected: ErrorHandler = $state('slack')
	let error_handler_path: string | undefined = $state()
	let error_handler_args: Record<string, any> = $state({})
	let retry: Retry | undefined = $state()
	let email_recipients: string[] | undefined = $state([])

	let optionTabSelected: 'connection_options' | 'error_handler' | 'retries' =
		$state('connection_options')

	const mqttConfig = $derived.by(getSaveCfg)
	const captureConfig = $derived.by(isEditor ? getCaptureConfig : () => ({}))
	const saveDisabled = $derived(
		pathError != '' || emptyString(script_path) || !can_write || !isValid
	)
	const activateV5Options = $state({
		topic_alias_maximum: Boolean(DEFAULT_V5_CONFIG.topic_alias_maximum),
		session_expiry_interval: Boolean(DEFAULT_V5_CONFIG.session_expiry_interval)
	})

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
			sendUserToast(`Could not load mqtt trigger: ${err.body}`, true)
		} finally {
			if (!defaultConfig) {
				initialConfig = structuredClone($state.snapshot(getSaveCfg()))
			}
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
			mqtt_resource_path = defaultValues?.mqtt_resource_path ?? ''
			drawer?.openDrawer()
			is_flow = nis_flow
			itemKind = nis_flow ? 'flow' : 'script'
			initialScriptPath = ''
			fixedScriptPath = fixedScriptPath_ ?? ''
			script_path = fixedScriptPath
			subscribe_topics = defaultValues?.topics ?? []
			path = defaultValues?.path ?? ''
			initialPath = ''
			edit = false
			dirtyPath = false
			client_version = defaultValues?.client_version ?? 'v5'
			client_id = defaultValues?.client_id ?? ''
			enabled = defaultValues?.enabled ?? false
			error_handler_path = defaultValues?.error_handler_path ?? undefined
			error_handler_args = defaultValues?.error_handler_args ?? {}
			retry = defaultValues?.retry ?? undefined
			errorHandlerSelected = getHandlerType(error_handler_path ?? '')
			v3_config = defaultValues?.v3_config ?? DEFAULT_V3_CONFIG
			v5_config = defaultValues?.v5_config ?? DEFAULT_V5_CONFIG
			activateV5Options.topic_alias_maximum = Boolean(defaultValues?.v5_config?.topic_alias_maximum)
			activateV5Options.session_expiry_interval = Boolean(
				defaultValues?.v5_config?.session_expiry_interval
			)
			email_recipients = defaultValues?.email_recipients
		} finally {
			clearTimeout(loadingTimeout)
			drawerLoading = false
			showLoading = false
		}
	}

	async function loadTriggerConfig(cfg?: Record<string, any>): Promise<void> {
		try {
			mqtt_resource_path = cfg?.mqtt_resource_path
			subscribe_topics = cfg?.subscribe_topics
			script_path = cfg?.script_path
			initialScriptPath = cfg?.script_path
			is_flow = cfg?.is_flow
			path = cfg?.path
			enabled = cfg?.enabled
			client_version = cfg?.client_version
			v3_config = cfg?.v3_config ?? DEFAULT_V3_CONFIG
			v5_config = cfg?.v5_config ?? DEFAULT_V5_CONFIG
			client_id = cfg?.client_id ?? ''
			can_write = canWrite(cfg?.path, cfg?.extra_perms, $userStore)
			error_handler_path = cfg?.error_handler_path
			error_handler_args = cfg?.error_handler_args ?? {}
			retry = cfg?.retry
			errorHandlerSelected = getHandlerType(error_handler_path ?? '')
			activateV5Options.topic_alias_maximum = Boolean(v5_config.topic_alias_maximum)
			activateV5Options.session_expiry_interval = Boolean(v5_config.session_expiry_interval)
			email_recipients = cfg?.email_recipients
		} catch (error) {
			sendUserToast(`Could not load mqtt trigger config: ${error.body}`, true)
		}
	}

	async function loadTrigger(defaultConfig?: Record<string, any>): Promise<void> {
		try {
			if (defaultConfig) {
				loadTriggerConfig(defaultConfig)
				return
			} else {
				const s = await MqttTriggerService.getMqttTrigger({
					workspace: $workspaceStore!,
					path: initialPath
				})
				loadTriggerConfig(s)
			}
		} catch (error) {
			sendUserToast(`Could not load mqtt trigger: ${error.body}`, true)
		}
	}

	function getSaveCfg(): Record<string, any> {
		return {
			client_id,
			client_version,
			v3_config,
			v5_config,
			mqtt_resource_path,
			subscribe_topics,
			path,
			script_path,
			enabled,
			is_flow,
			error_handler_path,
			error_handler_args,
			email_recipients,
			retry
		}
	}

	function getCaptureConfig(): Record<string, any> {
		return {
			mqtt_resource_path,
			subscribe_topics,
			client_version,
			v3_config,
			v5_config,
			client_id,
			path
		}
	}

	async function updateTrigger(): Promise<void> {
		deploymentLoading = true
		const cfg = getSaveCfg()
		const isSaved = await saveMqttTriggerFromCfg(
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

	async function handleToggleEnabled(newEnabled: boolean) {
		enabled = newEnabled
		if (!trigger?.draftConfig) {
			await MqttTriggerService.setMqttTriggerEnabled({
				path: initialPath,
				workspace: $workspaceStore ?? '',
				requestBody: { enabled: newEnabled }
			})
			sendUserToast(`${newEnabled ? 'enabled' : 'disabled'} MQTT trigger ${initialPath}`)
		}
	}

	$effect(() => {
		let args = [captureConfig, isValid] as const
		onCaptureConfigChange?.(...args)
	})

	$effect(() => {
		if (!drawerLoading) {
			handleConfigChange(mqttConfig, initialConfig, saveDisabled, edit, onConfigChange)
		}
	})
</script>

{#if useDrawer}
	<Drawer size="800px" bind:this={drawer}>
		<DrawerContent
			title={edit
				? can_write
					? `Edit MQTT trigger ${initialPath}`
					: `MQTT trigger ${initialPath}`
				: 'New MQTT trigger'}
			on:close={drawer.closeDrawer}
		>
			{#snippet actions()}
				{@render actionsSnippet()}
			{/snippet}
			{@render config()}
		</DrawerContent>
	</Drawer>
{:else}
	<Section label={!customLabel ? 'MQTT trigger' : ''} headerClass="grow min-w-0 h-[30px]">
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
			{enabled}
			{allowDraft}
			{edit}
			isLoading={deploymentLoading}
			{isDeployed}
			onUpdate={updateTrigger}
			{onReset}
			{onDelete}
			onToggleEnabled={handleToggleEnabled}
			{cloudDisabled}
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
						New MQTT triggers can take up to 30 seconds to start listening.
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
						namePlaceholder="mqtt_trigger"
						kind="mqtt_trigger"
						disabled={!can_write}
						disableEditing={!can_write}
					/>
				</Label>
			</div>

			{#if !hideTarget}
				<Section label="Runnable">
					<p class="text-xs mb-1 text-tertiary">
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
								btnClasses="ml-4 mt-2"
								color="dark"
								size="xs"
								disabled={!can_write}
								href={itemKind === 'flow' ? '/flows/add?hub=61' : '/scripts/add?hub=hub%2F19655'}
								target="_blank"
							>
								Create from template
							</Button>
						{/if}
					</div>
				</Section>
			{/if}

			<MqttEditorConfigSection
				bind:mqtt_resource_path
				bind:subscribe_topics
				{can_write}
				bind:client_version
				bind:isValid
				bind:client_id
				showTestingBadge={isEditor}
			/>

			<Section label="Advanced" collapsable>
				<div class="flex flex-col gap-4">
					<div class="min-h-96">
						<Tabs bind:selected={optionTabSelected}>
							<Tab value="connection_options">Connection Options</Tab>
							<Tab value="error_handler">Error Handler</Tab>
							<Tab value="retries">Retries</Tab>
						</Tabs>
						<div class="mt-4">
							{#if optionTabSelected === 'connection_options'}
								<div class="flex p-2 flex-col gap-2 mt-3">
									<ToggleButtonGroup bind:selected={client_version}>
										{#snippet children({ item })}
											<ToggleButton value="v5" label="Version 5" {item} />
											<ToggleButton value="v3" label="Version 3" {item} />
										{/snippet}
									</ToggleButtonGroup>

									<input
										type="text"
										bind:value={client_id}
										disabled={!can_write}
										placeholder="Client id"
										autocomplete="off"
									/>

									{#if client_version === 'v5'}
										<Toggle
											textClass="font-normal text-sm"
											color="nord"
											size="xs"
											bind:checked={v5_config.clean_start}
											options={{
												right: 'Clean start',
												rightTooltip:
													'Start a new session without any stored messages or subscriptions if enabled. Otherwise, resume the previous session with stored subscriptions and undelivered messages. The default setting is 0.',
												rightDocumentationLink:
													'https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901039'
											}}
											class="py-1"
										/>

										<div class="flex flex-col gap-2">
											<Toggle
												textClass="font-normal text-sm"
												color="nord"
												size="xs"
												bind:checked={activateV5Options.session_expiry_interval}
												on:change={(ev) => {
													if (!ev.detail) {
														v5_config.session_expiry_interval = undefined
													}
												}}
												options={{
													right: 'Session expiry interval',
													rightTooltip:
														'Defines the time in seconds that the broker will retain the session after disconnection. If set to 0, the session ends immediately. If set to 4,294,967,295, the session will be retained indefinitely. Otherwise, subscriptions and undelivered messages are stored until the interval expires.',
													rightDocumentationLink: ''
												}}
												class="py-1"
											/>

											{#if activateV5Options.session_expiry_interval}
												<input
													type="number"
													bind:value={v5_config.session_expiry_interval}
													disabled={!can_write}
													placeholder="Session expiry interval"
													autocomplete="off"
												/>
											{/if}
										</div>

										<div class="flex flex-col gap-2">
											<Toggle
												textClass="font-normal text-sm"
												color="nord"
												size="xs"
												bind:checked={activateV5Options.topic_alias_maximum}
												on:change={(ev) => {
													if (!ev.detail) {
														v5_config.topic_alias_maximum = undefined
													}
												}}
												options={{
													right: 'Topic alias maximum',
													rightTooltip:
														'Defines the maximum topic alias value the client will accept from the broker. A value of 0 indicates that topic aliases are not supported. The default value is 65536, which is the maximum allowed topic alias.',
													rightDocumentationLink:
														'https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901051'
												}}
												class="py-1"
											/>

											{#if activateV5Options.topic_alias_maximum}
												<input
													type="number"
													bind:value={v5_config.topic_alias_maximum}
													disabled={!can_write}
													placeholder="Topic alias"
													autocomplete="off"
												/>
											{/if}
										</div>
									{:else if client_version === 'v3'}
										<Toggle
											textClass="font-normal text-sm"
											color="nord"
											size="xs"
											checked={v3_config.clean_session}
											on:change={() => {
												v3_config.clean_session = !v3_config.clean_session
											}}
											options={{
												right: 'Clean session',
												rightTooltip:
													'Starts a new session without any stored messages or subscriptions if enabled. Otherwise, it resumes the previous session with stored subscriptions and undelivered messages. The default value is 0',
												rightDocumentationLink: ''
											}}
											class="py-1"
										/>
									{/if}
								</div>
							{:else}
								<TriggerRetriesAndErrorHandler
									{optionTabSelected}
									{itemKind}
									{can_write}
									bind:errorHandlerSelected
									bind:error_handler_path
									bind:error_handler_args
									bind:email_recipients
									bind:retry
								/>
							{/if}
						</div>
					</div>
				</div>
			</Section>
		</div>
	{/if}
{/snippet}
