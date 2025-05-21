<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, sendUserToast } from '$lib/utils'
	import { Loader2 } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import { SqsTriggerService, type AwsAuthResourceType } from '$lib/gen'
	import SqsTriggerEditorConfigSection from './SqsTriggerEditorConfigSection.svelte'
	import Section from '$lib/components/Section.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import Required from '$lib/components/Required.svelte'
	import type { Snippet } from 'svelte'
	import TriggerEditorToolbar from '../TriggerEditorToolbar.svelte'
	import { saveSqsTriggerFromCfg } from './utils'
	import { handleConfigChange } from '../utils'

	interface Props {
		useDrawer?: boolean
		description?: Snippet | undefined
		hideTarget?: boolean
		hideTooltips?: boolean
		allowDraft?: boolean
		hasDraft?: boolean
		isDraftOnly?: boolean
		isEditor?: boolean
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
		allowDraft = false,
		hasDraft = false,
		isDraftOnly = false,
		isEditor = false,
		customLabel = undefined,
		isDeployed = false,
		cloudDisabled = false,
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
	let enabled = $state(false)
	let dirtyPath = $state(false)
	let can_write = $state(true)
	let drawerLoading = $state(true)
	let showLoading = $state(false)
	let aws_resource_path: string = $state('')
	let queue_url = $state('')
	let message_attributes: string[] = $state([])
	let aws_auth_resource_type: AwsAuthResourceType = $state('credentials')
	let isValid = $state(false)
	let initialConfig: Record<string, any> | undefined = undefined
	let deploymentLoading = $state(false)

	const sqsConfig = $derived.by(getSaveCfg)
	const captureConfig = $derived.by(getCaptureConfig)
	const saveDisabled = $derived(
		pathError != '' || emptyString(script_path) || !isValid || !can_write
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
			sendUserToast(`Could not load sqs trigger: ${err.body}`, true)
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
			drawer?.openDrawer()
			is_flow = nis_flow
			itemKind = nis_flow ? 'flow' : 'script'
			initialScriptPath = ''
			fixedScriptPath = fixedScriptPath_ ?? ''
			script_path = fixedScriptPath
			aws_resource_path = defaultValues?.aws_resource_path ?? ''
			queue_url = defaultValues?.queue_url ?? ''
			path = defaultValues?.path ?? ''
			message_attributes = defaultValues?.message_attributes ?? []
			aws_auth_resource_type = defaultValues?.aws_auth_resource_type ?? 'credentials'
			initialPath = ''
			edit = false
			dirtyPath = false
			enabled = defaultValues?.enabled ?? false
		} finally {
			initialConfig = structuredClone($state.snapshot(getSaveCfg()))
			clearTimeout(loadingTimeout)
			drawerLoading = false
			showLoading = false
		}
	}

	async function loadTriggerConfig(cfg?: Record<string, any>): Promise<void> {
		try {
			script_path = cfg?.script_path
			initialScriptPath = cfg?.script_path
			aws_resource_path = cfg?.aws_resource_path
			queue_url = cfg?.queue_url
			is_flow = cfg?.is_flow
			message_attributes = cfg?.message_attributes ?? []
			path = cfg?.path
			enabled = cfg?.enabled
			aws_auth_resource_type = cfg?.aws_auth_resource_type
			can_write = canWrite(cfg?.path, cfg?.extra_perms, $userStore)
		} catch (error) {
			sendUserToast(`Could not load SQS trigger config: ${error.body}`, true)
		}
	}

	async function loadTrigger(defaultConfig?: Record<string, any>): Promise<void> {
		try {
			if (defaultConfig) {
				loadTriggerConfig(defaultConfig)
				return
			} else {
				const s = await SqsTriggerService.getSqsTrigger({
					workspace: $workspaceStore!,
					path: initialPath
				})
				loadTriggerConfig(s)
			}
		} catch (error) {
			sendUserToast(`Could not load SQS trigger: ${error.body}`, true)
		}
	}

	function getSaveCfg(): Record<string, any> {
		return {
			script_path,
			is_flow,
			path,
			aws_resource_path,
			queue_url,
			message_attributes,
			aws_auth_resource_type,
			enabled
		}
	}

	async function handleToggleEnabled(nEnabled: boolean) {
		enabled = nEnabled
		if (!isDraftOnly && !hasDraft) {
			await SqsTriggerService.setSqsTriggerEnabled({
				path: initialPath,
				workspace: $workspaceStore ?? '',
				requestBody: { enabled: nEnabled }
			})
			sendUserToast(`${nEnabled ? 'enabled' : 'disabled'} SQS trigger ${initialPath}`)
		}
	}

	async function updateTrigger(): Promise<void> {
		deploymentLoading = true
		const cfg = getSaveCfg()
		const isSaved = await saveSqsTriggerFromCfg(
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
			aws_resource_path,
			queue_url,
			message_attributes,
			aws_auth_resource_type,
			path
		}
	}

	$effect(() => {
		onCaptureConfigChange?.(captureConfig, isValid)
	})

	$effect(() => {
		if (!drawerLoading) {
			handleConfigChange(sqsConfig, initialConfig, saveDisabled, edit, onConfigChange)
		}
	})
</script>

{#if useDrawer}
	<Drawer size="800px" bind:this={drawer}>
		<DrawerContent
			title={edit
				? can_write
					? `Edit SQS trigger ${initialPath}`
					: `SQS trigger ${initialPath}`
				: 'New SQS trigger'}
			on:close={drawer.closeDrawer}
		>
			<svelte:fragment slot="actions">
				{@render actions()}
			</svelte:fragment>
			{@render config()}
		</DrawerContent>
	</Drawer>
{:else}
	<Section label={!customLabel ? 'SQS trigger' : ''} headerClass="grow min-w-0 h-[30px]">
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
						New SQS triggers can take up to 30 seconds to start listening.
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
						namePlaceholder="sqs_trigger"
						kind="sqs_trigger"
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
						/>
						{#if emptyString(script_path)}
							<Button
								btnClasses="ml-4 mt-2"
								color="dark"
								size="xs"
								disabled={!can_write}
								href={itemKind === 'flow' ? '/flows/add?hub=59' : '/scripts/add?hub=hub%2F19657'}
								target="_blank"
							>
								Create from template
							</Button>
						{/if}
					</div>
				</Section>
			{/if}

			<SqsTriggerEditorConfigSection
				bind:isValid
				bind:queue_url
				bind:message_attributes
				bind:aws_resource_path
				bind:aws_auth_resource_type
				{can_write}
				headless={true}
				showTestingBadge={isEditor}
			/>
		</div>
	{/if}
{/snippet}
