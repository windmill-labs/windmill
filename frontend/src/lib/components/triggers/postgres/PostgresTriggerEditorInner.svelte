<script lang="ts">
	import { Alert, Button, TabContent } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import { PostgresTriggerService, type Language, type Relations } from '$lib/gen'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, emptyStringTrimmed, sendUserToast } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import Section from '$lib/components/Section.svelte'
	import { Loader2, X } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import MultiSelect from 'svelte-multiselect'
	import PublicationPicker from './PublicationPicker.svelte'
	import SlotPicker from './SlotPicker.svelte'
	import { random_adj } from '$lib/components/random_positive_adjetive'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import RelationPicker from './RelationPicker.svelte'
	import { invalidRelations, savePostgresTriggerFromCfg } from './utils'
	import CheckPostgresRequirement from './CheckPostgresRequirement.svelte'
	import { base } from '$lib/base'
	import type { Snippet } from 'svelte'
	import TriggerEditorToolbar from '../TriggerEditorToolbar.svelte'
	import TestingBadge from '../testingBadge.svelte'

	interface Props {
		useDrawer?: boolean
		description?: Snippet | undefined
		hideTarget?: boolean
		editMode?: boolean
		isEditor?: boolean
		hideTooltips?: boolean
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
		isEditor = false,
		hideTooltips = false,
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
	let postgres_resource_path = $state('')
	let publication_name: string = $state('')
	let replication_slot_name: string = $state('')
	let relations: Relations[] | undefined = $state([])
	let transaction_to_track: string[] = $state([])
	let language: Language = 'Typescript'
	let loading = $state(false)
	type actions = 'create' | 'get'
	let selectedPublicationAction: actions | undefined = $state(undefined)
	let selectedSlotAction: actions | undefined = $state(undefined)
	let publicationItems: string[] = $state([])
	let transactionType: string[] = ['Insert', 'Update', 'Delete']
	let tab: 'advanced' | 'basic' = $state('basic')
	let isLoading = $state(false)
	let initialConfig = $state<Record<string, any> | undefined>(undefined)
	let neverSaved = $state(false)

	function isAdvancedTab(t: 'advanced' | 'basic'): boolean {
		return t === 'advanced'
	}

	function isBasicTab(t: 'advanced' | 'basic'): boolean {
		return t === 'basic'
	}

	let saveDisabled = $derived(
		pathError !== '' ||
			emptyString(postgres_resource_path) ||
			emptyString(script_path) ||
			(isAdvancedTab(tab) && emptyString(replication_slot_name)) ||
			emptyString(publication_name) ||
			(relations && isBasicTab(tab) && relations.length === 0) ||
			transaction_to_track.length === 0 ||
			drawerLoading ||
			!can_write
	)

	async function createPublication() {
		try {
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
		}
	}

	async function createSlot() {
		try {
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
		}
	}

	const dispatch = createEventDispatcher()

	$effect(() => {
		is_flow = itemKind === 'flow'
	})

	$effect(() => {
		isEditor &&
			dispatch('update-config', {
				postgres_resource_path,
				publication: {
					transaction_to_track,
					table_to_track: relations
				},
				path
			})
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
		} catch (err) {
			sendUserToast(`Could not load postgres trigger: ${err.body}`, true)
		} finally {
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

	function getSaveCfg(): Record<string, any> | undefined {
		if (
			relations &&
			invalidRelations(relations, {
				showError: true,
				trackSchemaTableError: true
			}) === true
		) {
			return undefined
		}
		const cfg = {
			script_path: script_path,
			initialScriptPath: initialScriptPath,
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
					: undefined
		}
		return cfg
	}

	async function loadTriggerConfig(cfg?: Record<string, any>): Promise<void> {
		script_path = cfg?.script_path
		initialScriptPath = cfg?.script_path
		is_flow = cfg?.is_flow
		path = cfg?.path
		enabled = cfg?.enabled
		postgres_resource_path = cfg?.postgres_resource_path
		publication_name = cfg?.publication_name
		replication_slot_name = cfg?.replication_slot_name
		can_write = canWrite(path, cfg?.extra_perms, $userStore)
		transaction_to_track = [...cfg?.publication?.transaction_to_track]
		relations = cfg?.publication?.table_to_track ?? []
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

			initialConfig = { ...s, publication: publication_data }
			loadTriggerConfig(initialConfig)
		}
	}

	function saveDraft() {
		const cfg = getSaveCfg()
		if (!cfg) {
			return
		}
		dispatch('save-draft', cfg)
		toggleEditMode(false)
	}

	async function updateTrigger(): Promise<void> {
		const cfg = getSaveCfg()
		if (!cfg) {
			return
		}
		isLoading = true
		await savePostgresTriggerFromCfg(initialPath, cfg, edit, $workspaceStore!, usedTriggerKinds)
		dispatch('update', path)
		drawer?.closeDrawer()
		toggleEditMode(false)
		isLoading = false
	}

	function toggleEditMode(newEditMode: boolean) {
		dispatch('toggle-edit-mode', newEditMode)
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

	async function handleToggleEnabled(e: CustomEvent<boolean>) {
		enabled = e.detail
		if (!isDraftOnly && !hasDraft) {
			await PostgresTriggerService.setPostgresTriggerEnabled({
				path: initialPath,
				workspace: $workspaceStore ?? '',
				requestBody: { enabled: e.detail }
			})
			sendUserToast(`${e.detail ? 'enabled' : 'disabled'} postgres trigger ${initialPath}`)
		}
	}
</script>

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
			<svelte:fragment slot="actions">{@render actions()}</svelte:fragment>
			{@render content()}
		</DrawerContent>
	</Drawer>
{:else}
	<Section label={!customLabel ? 'Postgres trigger' : ''} headerClass="grow min-w-0">
		<svelte:fragment slot="header">
			{#if customLabel}
				{@render customLabel()}
			{/if}
		</svelte:fragment>
		<svelte:fragment slot="action">
			{@render actions()}
		</svelte:fragment>
		{@render content()}
	</Section>
{/if}

{#snippet actions()}
	{#if !drawerLoading}
		<TriggerEditorToolbar
			{isDraftOnly}
			{hasDraft}
			permissions={drawerLoading || !can_write ? 'none' : 'create'}
			{editMode}
			{saveDisabled}
			{allowDraft}
			{edit}
			{isLoading}
			{neverSaved}
			{enabled}
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

{#snippet content()}
	{#if drawerLoading}
		{#if showLoading}
			<Loader2 size="50" class="animate-spin" />
			<p>Loading...</p>
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
					disableEditing={!editMode}
				/>
			</Label>
			{#if !hideTarget}
				<Section label="Runnable">
					<p class="text-xs text-tertiary">
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

						{#if emptyStringTrimmed(script_path) && is_flow === false}
							<div class="flex">
								<Button
									disabled={!can_write || !editMode}
									btnClasses="ml-4 mt-2"
									color="dark"
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
				<svelte:fragment slot="badge">
					{#if isEditor}
						<TestingBadge />
					{/if}
				</svelte:fragment>
				<p class="text-xs text-tertiary mb-2">
					Pick a database to connect to <Required required={true} />
				</p>
				<div class="flex flex-col gap-8">
					<div class="flex flex-col gap-2">
						<ResourcePicker
							disabled={!can_write || !editMode}
							bind:value={postgres_resource_path}
							resourceType={'postgresql'}
						/>
						<CheckPostgresRequirement bind:postgres_resource_path bind:can_write />
					</div>

					{#if postgres_resource_path}
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
								noMatchingOptionsMsg=""
								createOptionMsg={null}
								duplicates={false}
								options={transactionType}
								allowUserOptions="append"
								bind:selected={transaction_to_track}
								ulOptionsClass={'!bg-surface !text-sm'}
								ulSelectedClass="!text-sm"
								outerDivClass="!bg-surface !min-h-[38px] !border-[#d1d5db]"
								placeholder="Select transactions"
								--sms-options-margin="4px"
								--sms-open-z-index="100"
								disabled={!editMode}
							>
								<!-- @migration-task: migrate this slot by hand, `remove-icon` is an invalid identifier -->
								<svelte:fragment slot="remove-icon">
									<div class="hover:text-primary p-0.5">
										<X size={12} />
									</div>
								</svelte:fragment>
							</MultiSelect>
						</Label>
						<Label label="Table Tracking">
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
							{/snippet}
							<Tabs bind:selected={tab}>
								<Tab value="basic"
									><div class="flex flex-row gap-1"
										>Basic<Tooltip
											documentationLink="https://www.windmill.dev/docs/core_concepts/postgres_triggers#define-what-to-track"
											><p
												>Choose the <strong>relations</strong> to track without worrying about the
												underlying mechanics of creating a
												<strong>publication</strong>
												or <strong>slot</strong>. This simplified option lets you focus only on the
												data you want to monitor.</p
											></Tooltip
										></div
									></Tab
								>
								<Tab value="advanced"
									><div class="flex flex-row gap-1"
										>Advanced<Tooltip
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
										></div
									></Tab
								>
								<svelte:fragment slot="content">
									<div class="mt-5 overflow-hidden bg-surface">
										<TabContent value="basic">
											<RelationPicker {can_write} bind:relations disabled={!editMode} />
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
															disabled={!can_write || !editMode}
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
																	disabled={!editMode}
																/>
																<Button
																	color="light"
																	size="xs"
																	variant="border"
																	disabled={emptyStringTrimmed(replication_slot_name) || !editMode}
																	on:click={createSlot}>Create</Button
																>
															</div>
														{:else}
															<SlotPicker
																bind:edit
																{postgres_resource_path}
																bind:replication_slot_name
																disabled={!editMode}
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
															disabled={!can_write || !editMode}
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
																	disabled={!can_write || !editMode}
																	type="text"
																	bind:value={publication_name}
																	placeholder={'Publication Name'}
																/>
																<Button
																	color="light"
																	size="xs"
																	variant="border"
																	disabled={emptyStringTrimmed(publication_name) ||
																		(relations && relations.length === 0) ||
																		!can_write ||
																		!editMode}
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
																disabled={!editMode}
															/>
														{/if}
														<RelationPicker {can_write} bind:relations disabled={!editMode} />
													</div>
												</Section></div
											>
										</TabContent>
									</div>
								</svelte:fragment>
							</Tabs>
						</Label>
					{/if}
				</div>
			</Section>
		</div>
	{/if}
{/snippet}
