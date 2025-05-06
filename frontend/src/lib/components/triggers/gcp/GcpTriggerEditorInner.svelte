<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, sendUserToast } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { Loader2 } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import {
		GcpTriggerService,
		type DeliveryType,
		type PushConfig,
		type SubscriptionMode
	} from '$lib/gen'
	import Section from '$lib/components/Section.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import Required from '$lib/components/Required.svelte'
	import GcpTriggerEditorConfigSection from './GcpTriggerEditorConfigSection.svelte'
	import { base } from '$app/paths'
	import type { Snippet } from 'svelte'
	import TriggerEditorToolbar from '../TriggerEditorToolbar.svelte'
	import { saveGcpTriggerFromCfg } from './utils'

	let is_loading = $state(false)
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
	let neverSaved = $state(false)

	const dispatch = createEventDispatcher()

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
		customLabel = undefined
	}: {
		useDrawer?: boolean
		description?: Snippet | undefined
		hideTarget?: boolean
		editMode?: boolean
		preventSave?: boolean
		hideTooltips?: boolean
		useEditButton?: boolean
		isEditor?: boolean
		allowDraft?: boolean
		hasDraft?: boolean
		isDraftOnly?: boolean
		customLabel?: Snippet
	} = $props()

	let resetEditMode = $state<(() => void) | undefined>(undefined)

	$effect(() => {
		is_flow = itemKind === 'flow'
		dispatch('update-config', {
			gcp_resource_path,
			subscription_mode,
			subscription_id,
			delivery_type,
			delivery_config,
			topic_id,
			isValid,
			path
		})
	})

	export async function openEdit(ePath: string, isFlow: boolean) {
		drawerLoading = true
		resetEditMode = () => openEdit(ePath, isFlow)
		try {
			drawer?.openDrawer()
			initialPath = ePath
			itemKind = isFlow ? 'flow' : 'script'
			edit = true
			dirtyPath = false
			await loadTrigger()
		} catch (err) {
			sendUserToast(`Could not load GCP Pub/Sub trigger: ${err.body}`, true)
		} finally {
			drawerLoading = false
		}
	}

	export async function openNew(
		nis_flow: boolean,
		fixedScriptPath_?: string,
		defaultValues?: Record<string, any>,
		newDraft?: boolean
	) {
		drawerLoading = true
		resetEditMode = () => openNew(nis_flow, fixedScriptPath_, defaultValues, newDraft)
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
			topic_id = defaultValues?.topic_id
			subscription_mode = defaultValues?.subscription_mode ?? 'create_update'
			path = defaultValues?.path ?? ''
			initialPath = ''
			edit = false
			dirtyPath = false
			enabled = defaultValues?.enabled ?? false
			if (newDraft) {
				neverSaved = true
				toggleEditMode(true)
			}
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
		topic_id = cfg?.topic_id
		can_write = canWrite(cfg?.path, cfg?.extra_perms, $userStore)
	}

	async function updateTrigger(): Promise<void> {
		is_loading = true
		const cfg = getSaveCfg()
		if (!cfg) {
			return
		}
		saveGcpTriggerFromCfg(initialPath, cfg, edit, $workspaceStore!, usedTriggerKinds)
		dispatch('update', cfg.path)
		drawer?.closeDrawer()
		is_loading = false
	}

	function toggleEditMode(newEditMode: boolean) {
		dispatch('toggle-edit-mode', newEditMode)
	}

	function getSaveCfg(): Record<string, any> | undefined {
		const base_endpoint = `${window.location.origin}${base}`
		if (delivery_type === 'push') {
			if (!delivery_config) {
				sendUserToast('Must set route path when delivery type is push', true)
				return
			}
		} else {
			delivery_config = undefined
		}
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
			is_flow
		}
	}

	function saveDraft() {
		const cfg = getSaveCfg()
		if (!cfg) {
			return
		}
		dispatch('save-draft', {
			cfg
		})
		toggleEditMode(false)
	}

	async function handleToggleEnabled(e: CustomEvent<boolean>) {
		enabled = e.detail
		if (!isDraftOnly && !hasDraft) {
			await GcpTriggerService.setGcpTriggerEnabled({
				path: initialPath,
				workspace: $workspaceStore ?? '',
				requestBody: { enabled: e.detail }
			})
			sendUserToast(`${e.detail ? 'enabled' : 'disabled'} GCP Pub/Sub trigger ${initialPath}`)
		}
	}
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
			<svelte:fragment slot="actions">
				{@render actionsButtons()}
			</svelte:fragment>
			{@render config()}
		</DrawerContent>
	</Drawer>
{:else}
	<Section label={!customLabel ? 'GCP Pub/Sub trigger' : ''} headerClass="grow min-w-0">
		<svelte:fragment slot="header">
			{#if customLabel}
				{@render customLabel()}
			{/if}
		</svelte:fragment>
		<svelte:fragment slot="action">
			{@render actionsButtons()}
		</svelte:fragment>
		{@render config()}
	</Section>
{/if}

{#snippet actionsButtons()}
	{#if !drawerLoading && can_write}
		<TriggerEditorToolbar
			{isDraftOnly}
			{hasDraft}
			permissions={drawerLoading || !can_write ? 'none' : 'create'}
			{editMode}
			saveDisabled={pathError != '' || emptyString(script_path) || !isValid || !can_write}
			{enabled}
			isLoading={is_loading}
			{edit}
			{allowDraft}
			{neverSaved}
			{isEditor}
			on:save-draft={() => {
				saveDraft()
			}}
			on:deploy={() => {
				updateTrigger()
			}}
			on:reset
			on:delete
			on:edit={() => {
				toggleEditMode(true)
			}}
			on:cancel={() => {
				resetEditMode?.()
				toggleEditMode(false)
			}}
			on:toggle-enabled={handleToggleEnabled}
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
						disableEditing={!editMode}
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
								href={itemKind === 'flow' ? '/flows/add?hub=68' : '/scripts/add?hub=hub%2F14251'}
								target="_blank"
								disabled={!editMode}>Create from template</Button
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
				bind:path
				cloud_subscription_id={subscription_id}
				create_update_subscription_id={subscription_id}
				can_write={can_write && editMode}
				headless={true}
				showTestingBadge={isEditor}
			/>
		</div>
	{/if}
{/snippet}
