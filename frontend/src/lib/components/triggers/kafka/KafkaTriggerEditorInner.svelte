<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import { KafkaTriggerService } from '$lib/gen'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, sendUserToast } from '$lib/utils'
	import Section from '$lib/components/Section.svelte'
	import { Loader2 } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import KafkaTriggersConfigSection from './KafkaTriggersConfigSection.svelte'
	import type { Snippet } from 'svelte'
	import TriggerEditorToolbar from '../TriggerEditorToolbar.svelte'
	import { saveKafkaTriggerFromCfg } from './utils'
	import { handleConfigChange, type Trigger } from '../utils'

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
		customLabel,
		isDeployed = false,
		cloudDisabled = false,
		onConfigChange = undefined,
		onCaptureConfigChange = undefined,
		onUpdate = undefined,
		onDelete = undefined,
		onReset = undefined
	}: Props = $props()

	let drawer: Drawer | undefined = $state()
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
	let initialConfig: Record<string, any> | undefined = undefined
	let extra_perms = $state<Record<string, any> | undefined>(undefined)
	let kafkaCfgValid = $state(false)
	let kafkaResourcePath = $state('')
	let kafkaCfg: Record<string, any> = $state({})
	let deploymentLoading = $state(false)

	const isValid = $derived(
		!!kafkaResourcePath &&
			kafkaCfgValid &&
			kafkaCfg.topics &&
			kafkaCfg.topics.length > 0 &&
			kafkaCfg.topics.every((b) => /^[a-zA-Z0-9-_.]+$/.test(b))
	)
	const saveDisabled = $derived(
		pathError !== '' ||
			!isValid ||
			drawerLoading ||
			!can_write ||
			emptyString(script_path) ||
			emptyString(kafkaResourcePath) ||
			kafkaCfg.topics.length === 0 ||
			kafkaCfg.topics.some((t) => emptyString(t))
	)
	const kafkaConfig = $derived.by(getSaveCfg)
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
			sendUserToast(`Could not load Kafka trigger: ${err}`, true)
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
		nDefaultValues?: Record<string, any>
	) {
		let loadingTimeout = setTimeout(() => {
			showLoading = true
		}, 100) // Do not show loading spinner for the first 100ms
		drawerLoading = true
		try {
			drawer?.openDrawer()
			is_flow = nis_flow
			edit = false
			itemKind = nis_flow ? 'flow' : 'script'
			kafkaResourcePath = nDefaultValues?.kafka_resource_path ?? ''
			kafkaCfg = {
				group_id: nDefaultValues?.group_id ?? '',
				topics: nDefaultValues?.topics ?? ['']
			}
			initialScriptPath = ''
			fixedScriptPath = fixedScriptPath_ ?? ''
			script_path = fixedScriptPath
			path = nDefaultValues?.path ?? ''
			initialPath = ''
			dirtyPath = false
		} finally {
			clearTimeout(loadingTimeout)
			drawerLoading = false
			showLoading = false
		}
	}

	function loadTriggerConfig(cfg?: Record<string, any>): void {
		script_path = cfg?.script_path
		initialScriptPath = cfg?.script_path
		is_flow = cfg?.is_flow
		path = cfg?.path
		kafkaResourcePath = cfg?.kafka_resource_path
		kafkaCfg = {
			group_id: cfg?.group_id,
			topics: cfg?.topics
		}
		enabled = cfg?.enabled
		extra_perms = cfg?.extra_perms
		can_write = canWrite(path, cfg?.extra_perms, $userStore)
	}

	async function loadTrigger(defaultConfig?: Record<string, any>): Promise<void> {
		if (defaultConfig) {
			loadTriggerConfig(defaultConfig)
			return
		} else {
			const s = await KafkaTriggerService.getKafkaTrigger({
				workspace: $workspaceStore!,
				path: initialPath
			})
			loadTriggerConfig(s)
		}
	}

	function getSaveCfg(): Record<string, any> {
		return {
			path,
			script_path,
			is_flow,
			kafka_resource_path: kafkaResourcePath,
			group_id: kafkaCfg.group_id,
			topics: kafkaCfg.topics,
			enabled,
			extra_perms: extra_perms
		}
	}

	async function updateTrigger(): Promise<void> {
		deploymentLoading = true
		const cfg = getSaveCfg()
		const isSaved = await saveKafkaTriggerFromCfg(
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

	function getCaptureConfig(): Record<string, any> {
		return {
			kafka_resource_path: kafkaResourcePath,
			group_id: kafkaCfg.group_id,
			topics: structuredClone($state.snapshot(kafkaCfg.topics)),
			path
		}
	}

	async function handleToggleEnabled(nEnabled: boolean) {
		await KafkaTriggerService.setKafkaTriggerEnabled({
			path: initialPath,
			workspace: $workspaceStore ?? '',
			requestBody: { enabled: nEnabled }
		})
		sendUserToast(`${nEnabled ? 'enabled' : 'disabled'} Kafka trigger ${initialPath}`)
	}

	$effect(() => {
		onCaptureConfigChange?.(captureConfig, isValid)
	})

	$effect(() => {
		if (!drawerLoading) {
			handleConfigChange(kafkaConfig, initialConfig, saveDisabled, edit, onConfigChange)
		}
	})
</script>

{#if useDrawer}
	<Drawer size="800px" bind:this={drawer}>
		<DrawerContent
			title={edit
				? can_write
					? `Edit Kafka trigger ${initialPath}`
					: `Kafka trigger ${initialPath}`
				: 'New Kafka trigger'}
			on:close={drawer.closeDrawer}
		>
			{#snippet actions()}
				{@render actionsButtons('sm')}
			{/snippet}
			{@render config()}
		</DrawerContent>
	</Drawer>
{:else}
	<Section label={!customLabel ? 'Kafka trigger' : ''} headerClass="grow min-w-0 h-[30px]">
		<svelte:fragment slot="header">
			{#if customLabel}
				{@render customLabel()}
			{/if}
		</svelte:fragment>
		<svelte:fragment slot="action">
			{@render actionsButtons('xs')}
		</svelte:fragment>
		{@render config()}
	</Section>
{/if}

{#snippet actionsButtons(size: 'xs' | 'sm' = 'sm')}
	{#if !drawerLoading}
		<TriggerEditorToolbar
			{trigger}
			permissions={drawerLoading || !can_write ? 'none' : 'create'}
			{enabled}
			{allowDraft}
			{edit}
			isLoading={deploymentLoading}
			{isDeployed}
			{saveDisabled}
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
						Kafka consumers can take up to 30 seconds to start.
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
						namePlaceholder="kafka_trigger"
						kind="kafka_trigger"
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
								href={itemKind === 'flow' ? '/flows/add?hub=65' : '/scripts/add?hub=hub%2F19659'}
								target="_blank">Create from template</Button
							>
						{/if}
					</div>
				</Section>
			{/if}

			<KafkaTriggersConfigSection
				bind:kafkaCfgValid
				bind:kafkaResourcePath
				bind:kafkaCfg
				{path}
				{can_write}
				showTestingBadge={isEditor}
			/>
		</div>
	{/if}
{/snippet}
