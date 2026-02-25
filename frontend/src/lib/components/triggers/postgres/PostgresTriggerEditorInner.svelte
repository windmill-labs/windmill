<script lang="ts">
	import { Alert, Button, TabContent } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import {
		PostgresTriggerService,
		type ErrorHandler,
		type Language,
		type Relations,
		type Retry,
		type TriggerMode
	} from '$lib/gen'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, emptyStringTrimmed, sendUserToast } from '$lib/utils'
	import Section from '$lib/components/Section.svelte'
	import { Loader2 } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import PublicationPicker from './PublicationPicker.svelte'
	import SlotPicker from './SlotPicker.svelte'
	import { random_adj } from '$lib/components/random_positive_adjetive'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import RelationPicker from './RelationPicker.svelte'
	import { invalidRelations, savePostgresTriggerFromCfg } from './utils'
	import CheckPostgresRequirement from './CheckPostgresRequirement.svelte'
	import { base } from '$lib/base'
	import { untrack, type Snippet } from 'svelte'
	import TriggerEditorToolbar from '../TriggerEditorToolbar.svelte'
	import TestingBadge from '../testingBadge.svelte'
	import { getHandlerType, handleConfigChange, type Trigger } from '../utils'
	import TriggerRetriesAndErrorHandler from '../TriggerRetriesAndErrorHandler.svelte'
	import { fade } from 'svelte/transition'
	import MultiSelect from '$lib/components/select/MultiSelect.svelte'
	import { safeSelectItems } from '$lib/components/select/utils.svelte'
	import TriggerSuspendedJobsAlert from '../TriggerSuspendedJobsAlert.svelte'
	import TriggerSuspendedJobsModal from '../TriggerSuspendedJobsModal.svelte'
	import { deepEqual } from 'fast-equals'
	import { capitalize } from '$lib/utils'

	interface Props {
		useDrawer?: boolean
		description?: Snippet | undefined
		hideTarget?: boolean
		isEditor?: boolean
		hideTooltips?: boolean
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
		isEditor = false,
		hideTooltips = false,
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
	let edit: boolean = $state(true)
	let itemKind: 'flow' | 'script' = $state('script')
	let script_path: string = $state('')
	let initialScriptPath: string = $state('')
	let fixedScriptPath: string = $state('')
	let path: string = $state('')
	let pathError = $state('')
	let dirtyPath: boolean = $state(false)
	let can_write: boolean = $state(true)
	let drawerLoading: boolean = $state(true)
	let showLoading: boolean = $state(false)
	let postgres_resource_path: string = $state('')
	let publication_name: string = $state('')
	let replication_slot_name: string = $state('')
	let relations: Relations[] | undefined = $state([])
	let transaction_to_track: string[] = $state([])
	let language: Language = 'Typescript'
	let loading = $state(false)
	let postgresVersion: string = $state('')
	let loadingPostgres: boolean = $state(false)
	type actions = 'create' | 'get'
	let selectedPublicationAction: actions | undefined = $state(undefined)
	let selectedSlotAction: actions | undefined = $state(undefined)
	let publicationItems: string[] = $state([])
	let transactionType: string[] = $state(['Insert', 'Update', 'Delete'])
	let tab: 'advanced' | 'basic' = $state('basic')
	let basic_mode = $derived(tab === 'basic')
	let initialConfig: Record<string, any> | undefined = undefined
	let deploymentLoading = $state(false)
	let creatingSlot: boolean = $state(false)
	let creatingPublication: boolean = $state(false)
	let pg14: boolean = $derived(postgresVersion.startsWith('14'))
	let optionTabSelected: 'error_handler' | 'retries' = $state('error_handler')
	let errorHandlerSelected: ErrorHandler = $state('slack')
	let error_handler_path: string | undefined = $state()
	let error_handler_args: Record<string, any> = $state({})
	let retry: Retry | undefined = $state()
	let mode = $state<TriggerMode>('enabled')

	let suspendedJobsModal = $state<TriggerSuspendedJobsModal | null>(null)
	let originalConfig = $state<Record<string, any> | undefined>(undefined)

	let hasChanged = $derived(!deepEqual(getSaveCfg(), originalConfig ?? {}))

	const errorMessage = $derived.by(() => {
		if (relations && relations.length > 0) {
			return invalidRelations(relations, {
				showError: true,
				trackSchemaTableError: true
			})
		}
		return ''
	})

	function isAdvancedTab(t: 'advanced' | 'basic'): boolean {
		return t === 'advanced'
	}

	function isBasicTab(t: 'advanced' | 'basic'): boolean {
		return t === 'basic'
	}

	const isValid = $derived(
		!emptyString(postgres_resource_path) &&
			transaction_to_track.length > 0 &&
			((isAdvancedTab(tab) &&
				!emptyString(replication_slot_name) &&
				!emptyString(publication_name)) ||
				(isBasicTab(tab) && (!relations || relations.length > 0))) &&
			!errorMessage
	)

	const postgresConfig = $derived.by(getSaveCfg)
	const captureConfig = $derived.by(isEditor ? getCaptureConfig : () => ({}))

	const saveDisabled = $derived(
		pathError !== '' ||
			emptyString(script_path) ||
			drawerLoading ||
			!can_write ||
			!isValid ||
			!hasChanged
	)

	async function createPublication() {
		try {
			creatingPublication = true
			const message = await PostgresTriggerService.createPostgresPublication({
				path: postgres_resource_path,
				publication: publication_name as string,
				workspace: $workspaceStore!,
				requestBody: {
					transaction_to_track: transaction_to_track,
					table_to_track: relations
				}
			})

			sendUserToast(message)
		} catch (error) {
			sendUserToast(error.body, true)
		} finally {
			creatingPublication = false
		}
	}

	async function createSlot() {
		try {
			creatingSlot = true
			const message = await PostgresTriggerService.createPostgresReplicationSlot({
				path: postgres_resource_path,
				workspace: $workspaceStore!,
				requestBody: {
					name: replication_slot_name
				}
			})
			sendUserToast(message)
		} catch (error) {
			sendUserToast(error.body, true)
		} finally {
			creatingSlot = false
		}
	}

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
			selectedPublicationAction = 'get'
			selectedSlotAction = 'get'
			selectedPublicationAction = selectedPublicationAction
			selectedSlotAction = selectedSlotAction
			relations = []
			transaction_to_track = []
			tab = 'basic'
			await loadTrigger(defaultConfig)
			originalConfig = structuredClone($state.snapshot(getSaveCfg()))
		} catch (err) {
			sendUserToast(`Could not load postgres trigger: ${err.body}`, true)
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
		defaultValues?: Record<string, any>,
		newDraft?: boolean
	) {
		let loadingTimeout = setTimeout(() => {
			showLoading = true
		}, 100) // Do not show loading spinner for the first 100ms
		drawerLoading = true
		try {
			selectedPublicationAction = 'create'
			selectedSlotAction = 'create'
			tab = 'basic'

			drawer?.openDrawer()
			is_flow = nis_flow
			itemKind = nis_flow ? 'flow' : 'script'
			initialScriptPath = defaultValues?.script_path ?? ''
			fixedScriptPath = fixedScriptPath_ ?? ''
			script_path = fixedScriptPath
			path = defaultValues?.path ?? ''
			initialPath = ''
			postgres_resource_path = defaultValues?.postgres_resource_path ?? ''
			edit = false
			dirtyPath = false
			publication_name = defaultValues?.publication_name ?? `windmill_publication_${random_adj()}`
			replication_slot_name =
				defaultValues?.replication_slot_name ?? `windmill_replication_${random_adj()}`
			transaction_to_track = defaultValues?.publication?.transaction_to_track || [
				'Insert',
				'Update',
				'Delete'
			]
			relations = defaultValues?.publication?.table_to_track || [
				{
					schema_name: 'public',
					table_to_track: []
				}
			]
			error_handler_path = defaultValues?.error_handler_path ?? undefined
			error_handler_args = defaultValues?.error_handler_args ?? {}
			retry = defaultValues?.retry ?? undefined
			errorHandlerSelected = getHandlerType(error_handler_path ?? '')
			mode = defaultValues?.mode ?? 'enabled'
			originalConfig = undefined
		} finally {
			clearTimeout(loadingTimeout)
			drawerLoading = false
			showLoading = false
		}
	}

	function getSaveCfg(): Record<string, any> {
		const cfg = {
			script_path: script_path,
			is_flow: is_flow,
			path: path,
			postgres_resource_path: postgres_resource_path,
			publication_name: edit || tab === 'advanced' ? publication_name : undefined,
			replication_slot_name: edit || tab === 'advanced' ? replication_slot_name : undefined,
			publication:
				!edit || tab === 'basic'
					? {
							transaction_to_track: transaction_to_track,
							table_to_track: relations
						}
					: undefined,
			error_handler_path,
			error_handler_args,
			retry,
			mode
		}
		return cfg
	}

	async function loadTriggerConfig(cfg?: Record<string, any>): Promise<void> {
		script_path = cfg?.script_path
		initialScriptPath = cfg?.script_path
		is_flow = cfg?.is_flow
		path = cfg?.path
		postgres_resource_path = cfg?.postgres_resource_path
		publication_name = cfg?.publication_name
		replication_slot_name = cfg?.replication_slot_name
		can_write = canWrite(path, cfg?.extra_perms, $userStore)
		transaction_to_track = [...cfg?.publication?.transaction_to_track]
		relations = cfg?.publication?.table_to_track ?? []
		error_handler_path = cfg?.error_handler_path
		error_handler_args = cfg?.error_handler_args ?? {}
		retry = cfg?.retry
		errorHandlerSelected = getHandlerType(error_handler_path ?? '')
		mode = cfg?.mode ?? 'enabled'
	}

	async function loadTrigger(defaultConfig?: Record<string, any>): Promise<void> {
		if (defaultConfig) {
			loadTriggerConfig(defaultConfig)
			if (defaultConfig?.publication) {
				transaction_to_track = [...defaultConfig.publication.transaction_to_track]
				relations = defaultConfig.publication.table_to_track ?? []
			}
			return
		} else {
			const s = await PostgresTriggerService.getPostgresTrigger({
				workspace: $workspaceStore!,
				path: initialPath
			})

			const publication_data = await PostgresTriggerService.getPostgresPublication({
				path: s.postgres_resource_path,
				workspace: $workspaceStore!,
				publication: s.publication_name
			})

			loadTriggerConfig({ ...s, publication: publication_data })
		}
	}

	function getCaptureConfig() {
		return {
			postgres_resource_path,
			basic_mode,
			publication:
				!edit || tab === 'basic'
					? {
							transaction_to_track: transaction_to_track,
							table_to_track: relations
						}
					: undefined,
			publication_name: edit || tab !== 'basic' ? publication_name : undefined,
			replication_slot_name: edit || tab !== 'basic' ? replication_slot_name : undefined,
			path
		}
	}

	async function updateTrigger(): Promise<void> {
		const cfg = postgresConfig
		if (!cfg) {
			return
		}
		deploymentLoading = true
		const isSaved = await savePostgresTriggerFromCfg(
			initialPath,
			cfg,
			edit,
			$workspaceStore!,
			usedTriggerKinds
		)
		if (isSaved) {
			onUpdate?.(path)
			originalConfig = structuredClone($state.snapshot(getSaveCfg()))
			initialPath = cfg.path
			initialScriptPath = cfg.script_path
			if (mode !== 'suspended') {
				drawer?.closeDrawer()
			}
		}
		deploymentLoading = false
	}

	const getTemplateScript = async () => {
		if (!relations || relations.length === 0 || emptyString(postgres_resource_path)) {
			sendUserToast('You must pick a database resource and choose at least one schema', true)
			return
		}
		try {
			loading = true
			let templateId = await PostgresTriggerService.createTemplateScript({
				workspace: $workspaceStore!,
				requestBody: {
					relations,
					language,
					postgres_resource_path
				}
			})
			window.open(`${base}/scripts/add?id=${templateId}`)
			loading = false
		} catch (error) {
			loading = false
			sendUserToast(error.body, true)
		}
	}

	async function handleToggleMode(newMode: TriggerMode) {
		mode = newMode
		if (!trigger?.draftConfig) {
			await PostgresTriggerService.setPostgresTriggerMode({
				path: initialPath,
				workspace: $workspaceStore ?? '',
				requestBody: { mode: newMode }
			})
			sendUserToast(`${capitalize(newMode)} postgres trigger ${initialPath}`)

			onUpdate?.(initialPath)
		}
		if (originalConfig) {
			originalConfig['mode'] = newMode
		}
	}

	$effect(() => {
		const args = [captureConfig, isValid] as const
		untrack(() => onCaptureConfigChange?.(...args))
	})

	$effect(() => {
		if (!drawerLoading) {
			handleConfigChange(postgresConfig, initialConfig, saveDisabled, edit, onConfigChange)
		}
	})

	$effect(() => {
		if (postgres_resource_path) {
			loadingPostgres = true
			PostgresTriggerService.getPostgresVersion({
				workspace: $workspaceStore!,
				path: postgres_resource_path
			})
				.then((version: string) => {
					postgresVersion = version
				})
				.catch((error: any) => {
					sendUserToast(error.body, true)
				})
				.finally(() => {
					loadingPostgres = false
				})
		}
	})
</script>

{#if mode === 'suspended'}
	<TriggerSuspendedJobsModal
		bind:this={suspendedJobsModal}
		triggerPath={path}
		triggerKind="postgres"
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
					? `Edit Postgres trigger ${initialPath}`
					: `Postgres trigger ${initialPath}`
				: 'New Postgres trigger'}
			on:close={drawer.closeDrawer}
		>
			{#snippet actions()}{@render actionsSnippet()}{/snippet}
			{@render content()}
		</DrawerContent>
	</Drawer>
{:else}
	<Section label={!customLabel ? 'Postgres trigger' : ''} headerClass="grow min-w-0 h-[30px]">
		{#snippet header()}
			{#if customLabel}
				{@render customLabel()}
			{/if}
		{/snippet}
		{#snippet action()}
			{@render actionsSnippet()}
		{/snippet}
		{@render content()}
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
			{mode}
			onToggleMode={handleToggleMode}
			{suspendedJobsModal}
			{cloudDisabled}
		/>
	{/if}
{/snippet}

{#snippet content()}
	{#if drawerLoading}
		{#if showLoading}
			<div class="flex flex-col items-center justify-center h-full w-full">
				<Loader2 size="50" class="animate-spin" />
				<p>Loading...</p>
			</div>
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
						New postgres triggers can take up to 30 seconds to start listening.
					{/if}
				</Alert>
			{/if}
		</div>
		<div class="flex flex-col gap-12 mt-6">
			{#if mode === 'suspended'}
				<TriggerSuspendedJobsAlert {suspendedJobsModal} />
			{/if}
			<Label label="Path">
				<Path
					bind:dirty={dirtyPath}
					bind:error={pathError}
					bind:path
					{initialPath}
					checkInitialPathExistence={!edit}
					namePlaceholder="postgres_trigger"
					kind="postgres_trigger"
					disabled={!can_write}
					disableEditing={!can_write}
				/>
			</Label>
			{#if !hideTarget}
				<Section label="Runnable">
					<p class="text-xs text-primary">
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
							clearable
						/>

						{#if emptyString(script_path) && is_flow === false}
							<div class="flex">
								<Button
									disabled={!can_write}
									btnClasses="ml-4"
									variant="accent"
									size="xs"
									on:click={getTemplateScript}
									target="_blank"
									{loading}
									>Create from template
									<Tooltip light>
										The conversion requires a <strong>database resource</strong> and at least one
										<strong>schema</strong>
										to be set.<br />
										Please ensure these conditions are met before proceeding.
									</Tooltip>
								</Button>
							</div>
						{/if}
					</div>
				</Section>
			{/if}

			<Section label="Database">
				{#snippet badge()}
					{#if isEditor}
						<TestingBadge />
					{/if}
				{/snippet}
				<p class="text-xs text-primary mb-2">
					Pick a database to connect to <Required required={true} />
				</p>
				<div class="flex flex-col gap-8">
					<div class="flex flex-col gap-2">
						<ResourcePicker
							disabled={!can_write}
							bind:value={postgres_resource_path}
							resourceType={'postgresql'}
							datatableAsPgResource
						/>
						<CheckPostgresRequirement bind:postgres_resource_path bind:can_write />
					</div>
					{#if loadingPostgres}
						<div class="flex flex-col items-center justify-center h-full w-full">
							<Loader2 size="50" class="animate-spin" />
							<p>Loading...</p>
						</div>
					{:else if postgres_resource_path}
						<Label label="Transactions">
							{#snippet header()}
								<Tooltip>
									<p>
										Choose the types of database transactions that should trigger a script or flow.
										You can select from <strong>Insert</strong>, <strong>Update</strong>,
										<strong>Delete</strong>, or any combination of these operations to define when
										the trigger should activate.
									</p>
								</Tooltip>
							{/snippet}
							<MultiSelect
								bind:value={transaction_to_track}
								items={safeSelectItems(transactionType)}
								onCreateItem={(x) => (transactionType.push(x), transaction_to_track.push(x))}
								placeholder="Select transactions"
								disabled={!can_write}
							/>
						</Label>
						<Label label="Table Tracking" headerClass="grow min-w-0">
							{#snippet header()}
								<Tooltip
									documentationLink="https://www.windmill.dev/docs/core_concepts/postgres_triggers#define-what-to-track"
								>
									<p>
										Select the tables to track. You can choose to track
										<strong>all tables in your database</strong>,
										<strong>all tables within a specific schema</strong>,
										<strong>specific tables in a schema</strong>, or even
										<strong>specific columns of a table</strong>. Additionally, you can apply a
										<strong>filter</strong> to retrieve only rows that do not match the specified criteria.
									</p>
								</Tooltip>
								{#if !emptyStringTrimmed(errorMessage)}
									<p
										class="text-red-500 text-xs truncate w-full whitespace-nowrap"
										title={errorMessage}
										transition:fade={{ duration: 100 }}
									>
										{errorMessage}
									</p>
								{/if}
							{/snippet}
							<Tabs bind:selected={tab}>
								<Tab value="basic" label="Basic">
									{#snippet extra()}
										<Tooltip
											documentationLink="https://www.windmill.dev/docs/core_concepts/postgres_triggers#define-what-to-track"
											><p
												>Choose the <strong>relations</strong> to track without worrying about the
												underlying mechanics of creating a
												<strong>publication</strong>
												or <strong>slot</strong>. This simplified option lets you focus only on the
												data you want to monitor.</p
											></Tooltip
										>
									{/snippet}
								</Tab>
								<Tab value="advanced" label="Advanced">
									{#snippet extra()}
										<Tooltip
											documentationLink="https://www.windmill.dev/docs/core_concepts/postgres_triggers#advanced"
											><p
												>Select a specific <strong>publication</strong> from your database to track,
												and manage it by <strong>creating</strong>,
												<strong>updating</strong>, or <strong>deleting</strong>. For
												<strong>slots</strong>, you can <strong>create</strong> or
												<strong>delete</strong>
												them. Both <strong>non-active slots</strong> and the
												<strong>currently used slot</strong> by the trigger will be retrieved from your
												database for management.</p
											></Tooltip
										>
									{/snippet}
								</Tab>
								{#snippet content()}
									<div class="mt-5 overflow-hidden bg-surface">
										<TabContent value="basic">
											<RelationPicker {can_write} bind:pg14 bind:relations disabled={!can_write} />
										</TabContent>
										<TabContent value="advanced">
											<div class="flex flex-col gap-6"
												><Section
													small
													label="Replication slot"
													tooltip="Choose and manage the slots for your trigger. You can create or delete slots. Both non-active slots and the currently used slot by the trigger (if any) will be retrieved from your database for management."
													documentationLink="https://www.windmill.dev/docs/core_concepts/postgres_triggers#managing-postgres-replication-slots"
												>
													<div class="flex flex-col gap-3">
														<ToggleButtonGroup
															bind:selected={selectedSlotAction}
															on:selected={() => {
																replication_slot_name = ''
															}}
															disabled={!can_write}
														>
															{#snippet children({ item })}
																<ToggleButton value="create" label="Create Slot" {item} />
																<ToggleButton value="get" label="Get Slot" {item} />
															{/snippet}
														</ToggleButtonGroup>
														{#if selectedSlotAction === 'create'}
															<div class="flex gap-3">
																<input
																	type="text"
																	bind:value={replication_slot_name}
																	placeholder={'Choose a slot name'}
																	disabled={!can_write}
																/>
																<Button
																	loading={creatingSlot}
																	size="xs"
																	variant="default"
																	disabled={emptyStringTrimmed(replication_slot_name) || !can_write}
																	on:click={createSlot}>Create</Button
																>
															</div>
														{:else}
															<SlotPicker
																bind:edit
																{postgres_resource_path}
																bind:replication_slot_name
																disabled={!can_write}
															/>
														{/if}
													</div>
												</Section>

												<Section
													small
													label="Publication"
													tooltip="Select and manage the publications for tracking data. You can create, update, or delete publications. Only existing publications in your database will be available for selection, giving you full control over what data is tracked."
													documentationLink="https://www.windmill.dev/docs/core_concepts/postgres_triggers#managing-postgres-publications"
												>
													<div class="flex flex-col gap-3">
														<ToggleButtonGroup
															selected={selectedPublicationAction}
															disabled={!can_write}
															on:selected={({ detail }) => {
																selectedPublicationAction = detail
																if (selectedPublicationAction === 'create') {
																	publication_name = `windmill_publication_${random_adj()}`
																	relations = [{ schema_name: 'public', table_to_track: [] }]
																	return
																}

																publication_name = ''
																relations = []
																transaction_to_track = []
															}}
														>
															{#snippet children({ item })}
																<ToggleButton value="create" label="Create Publication" {item} />
																<ToggleButton value="get" label="Get Publication" {item} />
															{/snippet}
														</ToggleButtonGroup>
														{#if selectedPublicationAction === 'create'}
															<div class="flex gap-3">
																<input
																	disabled={!can_write}
																	type="text"
																	bind:value={publication_name}
																	placeholder={'Publication Name'}
																/>
																<Button
																	loading={creatingPublication}
																	size="xs"
																	variant="default"
																	disabled={emptyStringTrimmed(publication_name) ||
																		(relations && relations.length === 0) ||
																		!can_write}
																	on:click={createPublication}>Create</Button
																>
															</div>
														{:else}
															<PublicationPicker
																{can_write}
																{postgres_resource_path}
																bind:transaction_to_track
																bind:relations
																bind:items={publicationItems}
																bind:publication_name
																disabled={!can_write}
															/>
														{/if}
														<RelationPicker
															bind:pg14
															{can_write}
															bind:relations
															disabled={!can_write}
														/>
													</div>
												</Section></div
											>
										</TabContent>
									</div>
								{/snippet}
							</Tabs>
						</Label>
					{/if}
				</div>
			</Section>

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
