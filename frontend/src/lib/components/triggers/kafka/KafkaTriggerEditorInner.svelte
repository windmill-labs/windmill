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
	import { createEventDispatcher } from 'svelte'
	import Section from '$lib/components/Section.svelte'
	import { Loader2, Save } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import KafkaTriggersConfigSection from './KafkaTriggersConfigSection.svelte'
	import type { Snippet } from 'svelte'
	import TriggerEditorToolbar from '../TriggerEditorToolbar.svelte'
	import { saveKafkaTriggerFromCfg } from './utils'

	interface Props {
		useDrawer?: boolean
		description?: Snippet | undefined
		hideTarget?: boolean
		editMode?: boolean
		hideTooltips?: boolean
		isEditor?: boolean
		allowDraft?: boolean
		hasDraft?: boolean
		isDraftOnly?: boolean
		customLabel?: Snippet
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
		customLabel
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
	let defaultValues: Record<string, any> | undefined = $state(undefined)
	let args: Record<string, any> = $state({})
	let showLoading = $state(false)
	let resetEditMode = $state<(() => void) | undefined>(undefined)
	let initialConfig = $state<Record<string, any> | undefined>(undefined)
	let neverSaved = $state(false)
	let extra_perms = $state<Record<string, any> | undefined>(undefined)

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
		resetEditMode = () => openEdit(ePath, isFlow, defaultConfig ?? initialConfig)
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
		resetEditMode = () => openNew(nis_flow, fixedScriptPath_, nDefaultValues, newDraft)
		let loadingTimeout = setTimeout(() => {
			showLoading = true
		}, 100) // Do not show loading spinner for the first 100ms
		drawerLoading = true
		try {
			drawer?.openDrawer()
			is_flow = nis_flow
			edit = false
			itemKind = nis_flow ? 'flow' : 'script'
			args.kafka_resource_path = nDefaultValues?.kafka_resource_path ?? ''
			args.group_id = nDefaultValues?.group_id ?? ''
			args.topics = nDefaultValues?.topics ?? ['']
			initialScriptPath = ''
			fixedScriptPath = fixedScriptPath_ ?? ''
			script_path = fixedScriptPath
			path = nDefaultValues?.path ?? ''
			initialPath = ''
			dirtyPath = false
			defaultValues = nDefaultValues
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

	function loadTriggerConfig(cfg?: Record<string, any>): void {
		script_path = cfg?.script_path
		initialScriptPath = cfg?.script_path
		is_flow = cfg?.is_flow
		path = cfg?.path
		args.kafka_resource_path = cfg?.kafka_resource_path
		args.group_id = cfg?.group_id
		args.topics = cfg?.topics
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
			initialConfig = s
			loadTriggerConfig(s)
		}
	}

	function getSaveCfg(): Record<string, any> {
		return {
			path,
			script_path,
			is_flow,
			kafka_resource_path: args.kafka_resource_path,
			group_id: args.group_id,
			topics: args.topics,
			enabled,
			extra_perms: extra_perms
		}
	}

	async function updateTrigger(): Promise<void> {
		const cfg = getSaveCfg()
		await saveKafkaTriggerFromCfg(initialPath, cfg, edit, $workspaceStore!, usedTriggerKinds)
		dispatch('update', cfg.path)
		drawer?.closeDrawer()
		toggleEditMode(false)
	}

	function useDefaultValues() {
		if (args.kafka_resource_path && args.kafka_resource_path != '') {
			return false
		}
		if (!defaultValues) {
			return false
		}
		return (
			defaultValues.brokers &&
			defaultValues.brokers.length > 0 &&
			defaultValues.brokers.some((broker: string) => broker.trim() !== '')
		)
	}

	function toggleEditMode(newEditMode: boolean) {
		dispatch('toggle-edit-mode', newEditMode)
	}

	$effect(() => {
		dispatch('update-config', {
			kafka_resource_path: $state.snapshot(args.kafka_resource_path),
			group_id: $state.snapshot(args.group_id),
			topics: structuredClone($state.snapshot(args.topics)),
			isValid,
			path
		})
	})

	let isValid = $state(false)

	function saveDraft() {
		const cfg = getSaveCfg()
		dispatch('save-draft', {
			cfg: cfg
		})
		toggleEditMode(false)
	}

	async function handleToggleEnabled(e: CustomEvent<boolean>) {
		await KafkaTriggerService.setKafkaTriggerEnabled({
			path: initialPath,
			workspace: $workspaceStore ?? '',
			requestBody: { enabled: e.detail }
		})
		sendUserToast(`${e.detail ? 'enabled' : 'disabled'} Kafka trigger ${initialPath}`)
	}
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
			<svelte:fragment slot="actions">
				{@render actionsButtons('sm')}
			</svelte:fragment>
			{@render config()}
		</DrawerContent>
	</Drawer>
{:else}
	<Section label={!customLabel ? 'Kafka trigger' : ''} headerClass="grow min-w-0">
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
	{#if !allowDraft}
		<Button
			{size}
			startIcon={{ icon: Save }}
			disabled={pathError !== '' ||
				!isValid ||
				drawerLoading ||
				!can_write ||
				emptyString(script_path) ||
				emptyString(args.kafka_resource_path) ||
				args.topics.length === 0 ||
				args.topics.some((t) => emptyString(t))}
			on:click={() => {
				updateTrigger()
			}}
		>
			Save
		</Button>
	{:else}
		<TriggerEditorToolbar
			{isDraftOnly}
			{hasDraft}
			permissions={drawerLoading || !can_write ? 'none' : 'create'}
			{editMode}
			{enabled}
			{allowDraft}
			{edit}
			isLoading={false}
			{neverSaved}
			{isEditor}
			saveDisabled={pathError !== '' ||
				!isValid ||
				drawerLoading ||
				!can_write ||
				emptyString(script_path) ||
				emptyString(args.kafka_resource_path) ||
				args.topics.length === 0 ||
				args.topics.some((t) => emptyString(t))}
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
								href={itemKind === 'flow' ? '/flows/add?hub=65' : '/scripts/add?hub=hub%2F11635'}
								target="_blank"
								disabled={!editMode}>Create from template</Button
							>
						{/if}
					</div>
				</Section>
			{/if}

			<KafkaTriggersConfigSection
				bind:args
				bind:isValid
				{path}
				can_write={can_write && editMode}
				defaultValues={useDefaultValues() ? defaultValues : undefined}
				showTestingBadge={isEditor}
			/>
		</div>
	{/if}
{/snippet}
