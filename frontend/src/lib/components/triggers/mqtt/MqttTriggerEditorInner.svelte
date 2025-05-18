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
		type MqttSubscribeTopic
	} from '$lib/gen'
	import MqttEditorConfigSection from './MqttEditorConfigSection.svelte'
	import type { Snippet } from 'svelte'
	import TriggerEditorToolbar from '../TriggerEditorToolbar.svelte'
	import { saveMqttTriggerFromCfg } from './utils'
	import { handleConfigChange } from '../utils'

	interface Props {
		useDrawer?: boolean
		description?: Snippet | undefined
		hideTarget?: boolean
		hideTooltips?: boolean
		isEditor?: boolean
		allowDraft?: boolean
		hasDraft?: boolean
		isDraftOnly?: boolean
		customLabel?: Snippet
		isDeployed?: boolean
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
		hasDraft = false,
		isDraftOnly = false,
		customLabel = undefined,
		isDeployed = false,
		onConfigChange = undefined,
		onCaptureConfigChange = undefined,
		onUpdate = undefined,
		onDelete = undefined,
		onReset = undefined
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
	let v3_config: MqttV3Config | undefined = $state()
	let v5_config: MqttV5Config | undefined = $state()
	let client_version: MqttClientVersion | undefined = $state()
	let client_id: string | undefined = $state(undefined)
	let isValid: boolean | undefined = $state(undefined)
	let initialConfig: Record<string, any> | undefined = undefined
	let deploymentLoading = $state(false)

	const mqttConfig = $derived.by(getSaveCfg)
	const captureConfig = $derived.by(isEditor ? getCaptureConfig : () => ({}))
	const saveDisabled = $derived(
		pathError != '' || emptyString(script_path) || !can_write || !isValid
	)

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
			v3_config = cfg?.v3_config
			v5_config = cfg?.v5_config
			client_id = cfg?.client_id ?? ''
			can_write = canWrite(cfg?.path, cfg?.extra_perms, $userStore)
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
			is_flow
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
		if (!isDraftOnly && !hasDraft) {
			await MqttTriggerService.setMqttTriggerEnabled({
				path: initialPath,
				workspace: $workspaceStore ?? '',
				requestBody: { enabled: newEnabled }
			})
			sendUserToast(`${newEnabled ? 'enabled' : 'disabled'} MQTT trigger ${initialPath}`)
		}
	}

	$effect(() => {
		onCaptureConfigChange?.(captureConfig, isValid ?? false)
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
			<svelte:fragment slot="actions">
				{@render actions()}
			</svelte:fragment>
			{@render config()}
		</DrawerContent>
	</Drawer>
{:else}
	<Section label={!customLabel ? 'MQTT trigger' : ''} headerClass="grow min-w-0">
		<svelte:fragment slot="header">
			{#if customLabel}
				{@render customLabel()}
			{/if}
		</svelte:fragment>
		<svelte:fragment slot="action">
			{@render actions()}
		</svelte:fragment>
		{@render config()}
	</Section>
{/if}

{#snippet actions()}
	{#if !drawerLoading}
		<TriggerEditorToolbar
			{isDraftOnly}
			{hasDraft}
			permissions={drawerLoading || !can_write ? 'none' : 'create'}
			{saveDisabled}
			{enabled}
			{allowDraft}
			{edit}
			isLoading={deploymentLoading}
			{isEditor}
			{isDeployed}
			onUpdate={updateTrigger}
			{onReset}
			{onDelete}
			onToggleEnabled={handleToggleEnabled}
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
				bind:v3_config
				bind:v5_config
				bind:isValid
				bind:client_id
				headless={true}
				showTestingBadge={isEditor}
			/>
		</div>
	{/if}
{/snippet}
