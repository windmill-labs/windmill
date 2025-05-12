<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import { NatsTriggerService } from '$lib/gen'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, sendUserToast } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import Section from '$lib/components/Section.svelte'
	import { Loader2 } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import NatsTriggersConfigSection from './NatsTriggersConfigSection.svelte'
	import type { Snippet } from 'svelte'
	import TriggerEditorToolbar from '../TriggerEditorToolbar.svelte'
	import { saveNatsTriggerFromCfg } from './utils'

	interface Props {
		useDrawer?: boolean
		description?: Snippet | undefined
		hideTarget?: boolean
		editMode?: boolean
		hideTooltips?: boolean
		useEditButton?: boolean
		isEditor?: boolean
		allowDraft?: boolean
		hasDraft?: boolean
		isDraftOnly?: boolean
		customLabel?: Snippet
		isDeployed?: boolean
	}

	let {
		useDrawer = true,
		description = undefined,
		hideTarget = false,
		editMode = true,
		hideTooltips = false,
		isEditor = false,
		allowDraft = false,
		hasDraft = false,
		isDraftOnly = false,
		customLabel = undefined,
		isDeployed = false
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
	let defaultValues: Record<string, any> | undefined = $state(undefined)
	let natsResourcePath = $state('')
	let subjects = $state([''])
	let useJetstream = $state(false)
	let streamName = $state('')
	let consumerName = $state('')
	let initialConfig = $state<Record<string, any> | undefined>(undefined)
	let neverSaved = $state(false)
	let deploymentLoading = $state(false)

	const dispatch = createEventDispatcher()

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
		}
	}

	export async function openNew(
		nis_flow: boolean,
		fixedScriptPath_?: string,
		nDefaultValues?: Record<string, any>,
		newDraft?: boolean
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
			subjects = nDefaultValues?.subjects ?? ['']
			useJetstream = nDefaultValues?.use_jetstream ?? false
			streamName = useJetstream ? (nDefaultValues?.stream_name ?? '') : undefined
			consumerName = useJetstream ? (nDefaultValues?.consumer_name ?? '') : undefined
			initialScriptPath = ''
			fixedScriptPath = fixedScriptPath_ ?? ''
			script_path = fixedScriptPath
			path = nDefaultValues?.path ?? ''
			initialPath = ''
			dirtyPath = false
			defaultValues = nDefaultValues
			enabled = nDefaultValues?.enabled ?? false
			if (newDraft) {
				neverSaved = true
				toggleEditMode(true)
			}
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
		streamName = cfg?.stream_name
		consumerName = cfg?.consumer_name
		subjects = cfg?.subjects || ['']
		useJetstream = cfg?.use_jetstream || false
		enabled = cfg?.enabled
		can_write = canWrite(cfg?.path, cfg?.extra_perms, $userStore)
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

			initialConfig = s
			loadTriggerConfig(initialConfig)
		}
	}

	function saveDraft() {
		const cfg = getSaveCfg()
		dispatch('save-draft', { cfg })
		toggleEditMode(false)
	}

	function getSaveCfg(): Record<string, any> {
		return {
			path,
			script_path,
			is_flow,
			enabled,
			nats_resource_path: natsResourcePath,
			stream_name: streamName,
			consumer_name: consumerName,
			subjects,
			use_jetstream: useJetstream
		}
	}

	async function updateTrigger(): Promise<void> {
		deploymentLoading = true
		const cfg = getSaveCfg()
		const isSaved = await saveNatsTriggerFromCfg(
			initialPath,
			cfg,
			edit,
			$workspaceStore!,
			usedTriggerKinds
		)
		if (isSaved) {
			dispatch('update', cfg.path)
			drawer?.closeDrawer()
			toggleEditMode(false)
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

	let isValid = $state(false)

	function toggleEditMode(newEditMode: boolean) {
		dispatch('toggle-edit-mode', newEditMode)
	}

	async function handleToggleEnabled(e: CustomEvent<boolean>) {
		enabled = e.detail
		if (!isDraftOnly && !hasDraft) {
			await NatsTriggerService.setNatsTriggerEnabled({
				path: initialPath,
				workspace: $workspaceStore ?? '',
				requestBody: { enabled: e.detail }
			})
			sendUserToast(`${e.detail ? 'enabled' : 'disabled'} NATS trigger ${initialPath}`)
		}
	}

	$effect(() => {
		isEditor &&
			dispatch('update-config', {
				nats_resource_path: natsResourcePath,
				subjects,
				isValid,
				path
			})
	})
</script>

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
			<svelte:fragment slot="actions">
				{@render actions()}
			</svelte:fragment>
			{@render config()}
		</DrawerContent>
	</Drawer>
{:else}
	<Section label={!customLabel ? 'NATS trigger' : ''} headerClass="grow min-w-0">
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
			{editMode}
			saveDisabled={pathError != '' || emptyString(script_path) || !can_write || !isValid}
			{enabled}
			{allowDraft}
			{edit}
			isLoading={deploymentLoading}
			{neverSaved}
			{isEditor}
			{isDeployed}
			on:save-draft={() => {
				saveDraft()
			}}
			on:deploy={() => {
				updateTrigger()
			}}
			on:reset
			on:delete
			on:edit={() => {
				initialConfig = getSaveCfg()
				toggleEditMode(true)
			}}
			on:cancel={() => {
				loadTrigger(initialConfig)
				toggleEditMode(false)
			}}
			on:toggle-enabled={handleToggleEnabled}
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
						disableEditing={!editMode}
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
							disabled={fixedScriptPath != '' || !can_write || !editMode}
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
								href={itemKind === 'flow' ? '/flows/add?hub=66' : '/scripts/add?hub=hub%2F11634'}
								target="_blank"
								disabled={!editMode}
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
				bind:subjects
				bind:useJetstream
				bind:streamName
				bind:consumerName
				on:valid-config={({ detail }) => {
					isValid = detail
				}}
				defaultValues={useDefaultValues() ? defaultValues : undefined}
				can_write={can_write && editMode}
				showTestingBadge={isEditor}
			/>
		</div>
	{/if}
{/snippet}
