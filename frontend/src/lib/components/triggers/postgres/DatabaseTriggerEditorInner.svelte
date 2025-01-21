<script lang="ts">
	import { Alert, Button, TabContent } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import { DatabaseTriggerService, type Language, type Relations } from '$lib/gen'
	import { databaseTrigger, usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, emptyStringTrimmed, sendUserToast } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import Section from '$lib/components/Section.svelte'
	import { Loader2, Save } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import { goto } from '$app/navigation'
	import { base } from '$app/paths'
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

	let drawer: Drawer
	let is_flow: boolean = false
	let initialPath = ''
	let edit = true
	let itemKind: 'flow' | 'script' = 'script'
	let script_path = ''
	let initialScriptPath = ''
	let fixedScriptPath = ''
	let path: string = ''
	let pathError = ''
	let enabled = false
	let dirtyPath = false
	let can_write = true
	let drawerLoading = true
	let database_resource_path = ''
	let publication_name: string = ''
	let replication_slot_name: string = ''
	let relations: Relations[] = []
	let transaction_to_track: string[] = []
	let language: Language = 'Typescript'
	let loading = false
	type actions = 'create' | 'get'
	let selectedPublicationAction: actions
	let selectedSlotAction: actions
	let publicationItems: string[] = []
	let transactionType: string[] = ['Insert', 'Update', 'Delete']
	let selectedTable: 'all' | 'specific' = 'specific'
	let tab: 'advanced' | 'basic'
	let config: { message: string | undefined; isLogical?: boolean } = { message: undefined }
	$: table_to_track = selectedTable === 'all' ? [] : relations

	async function createPublication() {
		try {
			const message = await DatabaseTriggerService.createDatabasePublication({
				path: database_resource_path,
				publication: publication_name as string,
				workspace: $workspaceStore!,
				requestBody: {
					transaction_to_track: transaction_to_track,
					table_to_track
				}
			})

			sendUserToast(message)
		} catch (error) {
			sendUserToast(error.body, true)
		}
	}

	async function createSlot() {
		try {
			const message = await DatabaseTriggerService.createDatabaseSlot({
				path: database_resource_path,
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

	$: is_flow = itemKind === 'flow'

	export async function openEdit(ePath: string, isFlow: boolean) {
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
			tab = 'basic'
			config.message = undefined
			await loadTrigger()
		} catch (err) {
			sendUserToast(`Could not load postgres trigger: ${err}`, true)
		} finally {
			drawerLoading = false
		}
	}

	export async function openNew(nis_flow: boolean, fixedScriptPath_?: string) {
		drawerLoading = true
		try {
			selectedPublicationAction = 'create'
			selectedSlotAction = 'create'
			selectedTable = 'specific'
			tab = 'basic'

			drawer?.openDrawer()
			is_flow = nis_flow
			itemKind = nis_flow ? 'flow' : 'script'
			initialScriptPath = ''
			fixedScriptPath = fixedScriptPath_ ?? ''
			script_path = fixedScriptPath
			path = ''
			initialPath = ''
			replication_slot_name = ''
			publication_name = ''
			database_resource_path = ''
			edit = false
			dirtyPath = false
			publication_name = `windmill_publication_${random_adj()}`
			replication_slot_name = `windmill_replication_${random_adj()}`
			transaction_to_track = ['Insert', 'Update', 'Delete']
			config.message = undefined
			relations = [
				{
					schema_name: 'public',
					table_to_track: []
				}
			]
		} finally {
			drawerLoading = false
		}
	}

	async function loadTrigger(): Promise<void> {
		const s = await DatabaseTriggerService.getDatabaseTrigger({
			workspace: $workspaceStore!,
			path: initialPath
		})
		script_path = s.script_path
		initialScriptPath = s.script_path

		is_flow = s.is_flow
		path = s.path
		enabled = s.enabled
		database_resource_path = s.database_resource_path
		publication_name = s.publication_name
		replication_slot_name = s.replication_slot_name

		const publication_data = await DatabaseTriggerService.getDatabasePublication({
			path: database_resource_path,
			workspace: $workspaceStore!,
			publication: publication_name
		})
		transaction_to_track = [...publication_data.transaction_to_track]
		relations = publication_data.table_to_track ?? []
		selectedTable = relations.length === 0 ? 'all' : 'specific'
		can_write = canWrite(s.path, s.extra_perms, $userStore)
	}

	async function updateTrigger(): Promise<void> {
		if (edit) {
			await DatabaseTriggerService.updateDatabaseTrigger({
				workspace: $workspaceStore!,
				path: initialPath,
				requestBody: {
					path,
					script_path,
					is_flow,
					database_resource_path,
					enabled,
					replication_slot_name,
					publication_name,
					publication:
						tab === 'basic'
							? {
									transaction_to_track,
									table_to_track
							  }
							: undefined
				}
			})
			sendUserToast(`Database ${path} updated`)
		} else {
			await DatabaseTriggerService.createDatabaseTrigger({
				workspace: $workspaceStore!,
				requestBody: {
					path,
					script_path,
					is_flow,
					enabled: true,
					database_resource_path,
					replication_slot_name: tab === 'basic' ? undefined : replication_slot_name,
					publication_name: tab === 'basic' ? undefined : publication_name,
					publication: {
						transaction_to_track,
						table_to_track
					}
				}
			})
			sendUserToast(`Database ${path} created`)
		}

		if (!$usedTriggerKinds.includes('database')) {
			$usedTriggerKinds = [...$usedTriggerKinds, 'database']
		}
		dispatch('update')
		drawer.closeDrawer()
	}

	const getTemplateScript = async () => {
		if (relations.length === 0 || emptyString(database_resource_path)) {
			sendUserToast('You must pick a database resource and choose at least one schema', true)
			return
		}

		try {
			loading = true
			let template = await DatabaseTriggerService.getTemplateScript({
				workspace: $workspaceStore!,
				requestBody: {
					relations,
					language,
					database_resource_path
				}
			})
			databaseTrigger.set({
				codeTemplate: template,
				databaseTrigger: {
					path,
					script_path,
					is_flow,
					enabled: true,
					database_resource_path,
					replication_slot_name,
					publication_name
				}
			})
			await goto(`${base}/scripts/add`)
		} catch (error) {
			loading = false
			sendUserToast(error.body, true)
		}
	}

	const checkDatabaseConfiguration = async () => {
		try {
			if (emptyString(database_resource_path)) {
				sendUserToast('You must first pick a database resource', true)
				return
			}

			const isLogical = await DatabaseTriggerService.isValidDatabaseConfiguration({
				workspace: $workspaceStore!,
				path: database_resource_path
			})
			config.isLogical = isLogical
			if (isLogical) {
				config.message =
					'Your database is correctly configured with logical replication enabled. You can proceed with using the streaming feature'
			} else {
				config.message =
					"Logical replication is not enabled on your database. To use this feature, your Postgres database must have <code>wal_level</code> configured as 'logical' in your database configuration."
			}
		} catch (error) {}
	}
</script>

<Drawer size="800px" bind:this={drawer}>
	<DrawerContent
		title={edit
			? can_write
				? `Edit Postgres trigger ${initialPath}`
				: `Postgres trigger ${initialPath}`
			: 'New Postgres trigger'}
		on:close={drawer.closeDrawer}
	>
		<svelte:fragment slot="actions">
			{#if !drawerLoading && can_write}
				{#if edit}
					<div class="mr-8 center-center -mt-1">
						<Toggle
							disabled={!can_write}
							checked={enabled}
							options={{ right: 'enable', left: 'disable' }}
							on:change={async (e) => {
								await DatabaseTriggerService.setDatabaseTriggerEnabled({
									path: initialPath,
									workspace: $workspaceStore ?? '',
									requestBody: { enabled: e.detail }
								})
								sendUserToast(
									`${e.detail ? 'enabled' : 'disabled'} postgres trigger ${initialPath}`
								)
							}}
						/>
					</div>
				{/if}
				<Button
					startIcon={{ icon: Save }}
					disabled={pathError != '' ||
						emptyString(database_resource_path) ||
						emptyString(script_path) ||
						((emptyString(replication_slot_name) || emptyString(publication_name)) &&
							tab === 'advanced') ||
						(relations.length === 0 && tab === 'basic') ||
						!can_write}
					on:click={updateTrigger}
				>
					Save
				</Button>
			{/if}
		</svelte:fragment>
		{#if drawerLoading}
			<Loader2 class="animate-spin" />
		{:else}
			<div class="flex flex-col gap-5">
				<Alert title="Info" type="info">
					{#if edit}
						Changes can take up to 30 seconds to take effect.
					{:else}
						New postgres triggers can take up to 30 seconds to start listening.
					{/if}
				</Alert>
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
							namePlaceholder="postgres_trigger"
							kind="postgres_trigger"
							disabled={!can_write}
						/>
					</Label>
				</div>

				<Section label="Database">
					<p class="text-xs mb-1 text-tertiary">
						Pick a database to connect to <Required required={true} />
					</p>
					<div class="flex flex-col mb-2 gap-3">
						<ResourcePicker bind:value={database_resource_path} resourceType={'postgresql'} />
						{#if database_resource_path}
							<Button on:click={checkDatabaseConfiguration} color="gray" size="sm"
								>Check Database Configuration
								<Tooltip>
									<p class="text-sm">
										Verifies whether the database is configured with the required <strong
											>settings</strong
										>.<br /> The <strong>logical wal_level</strong> setting is essential for the streaming
										feature to works. If it is not set, the trigger feature will not work, and the database
										configuration must be updated.
									</p>
								</Tooltip>
							</Button>
							{#if config.message}
								<Alert
									title="Postgres configuration"
									type={config.isLogical === true ? 'success' : 'error'}
								>
									{config.message}
								</Alert>
							{/if}
						{/if}
					</div>
				</Section>
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
							allowRefresh
						/>

						{#if script_path === undefined && is_flow === false}
							<div class="flex">
								<Button
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
				{#if database_resource_path}
					<Section label="Configuration">
						<div class="flex flex-col gap-5">
							<p class="text-xs mb-3 text-tertiary">
								Choose which table of your database to track as well as what kind of transaction
								should fire the script.<br />
								You must pick a database resource first to make the configuration of your trigger
								<Required required={true} />
							</p>
							<Section label="Transactions">
								<p class="text-xs mb-3 text-tertiary">
									Choose the types of database transactions that should trigger a script or flow.
									You can select from <strong>Insert</strong>, <strong>Update</strong>,
									<strong>Delete</strong>, or any combination of these operations to define when the
									trigger should activate.
								</p>
								<MultiSelect
									ulOptionsClass={'!bg-surface-secondary'}
									noMatchingOptionsMsg=""
									createOptionMsg={null}
									duplicates={false}
									bind:value={transaction_to_track}
									options={transactionType}
									allowUserOptions="append"
									bind:selected={transaction_to_track}
								/>
							</Section>
							<Section label="Publication">
								<p class="text-xs mb-3 text-tertiary">
									Select the tables to track. You can choose to track
									<strong>all tables in your database</strong>,
									<strong>all tables within a specific schema</strong>,
									<strong>specific tables in a schema</strong>, or even
									<strong>specific columns of a table</strong>. Additionally, you can apply a
									<strong>filter</strong> to retrieve only rows that do not match the specified criteria.
								</p>
								<Tabs bind:selected={tab}>
									<Tab value="basic"
										><div class="flex flex-row gap-1"
											>Basic<Tooltip
												><p
													>Choose the <strong>relations</strong> to track without worrying about the
													underlying mechanics of creating a
													<strong>publication</strong>
													or <strong>slot</strong>. This simplified option lets you focus only on
													the data you want to monitor.</p
												></Tooltip
											></div
										></Tab
									>
									<Tab value="advanced"
										><div class="flex flex-row gap-1"
											>Advanced<Tooltip
												><p
													>Select a specific <strong>publication</strong> from your database to
													track, and manage it by <strong>creating</strong>,
													<strong>updating</strong>, or <strong>deleting</strong>. For
													<strong>slots</strong>, you can <strong>create</strong> or
													<strong>delete</strong>
													them. Both <strong>non-active slots</strong> and the
													<strong>currently used slot</strong> by the trigger will be retrieved from
													your database for management.</p
												></Tooltip
											></div
										></Tab
									>
									<svelte:fragment slot="content">
										<div class="mt-5 overflow-hidden bg-surface">
											<TabContent value="basic">
												<RelationPicker bind:selectedTable bind:relations />
											</TabContent>
											<TabContent value="advanced">
												<div class="flex flex-col gap-4"
													><Section
														label="Slot name"
														tooltip="Choose and manage the slots for your trigger. You can create or delete slots. Both non-active slots and the currently used slot by the trigger (if any) will be retrieved from your database for management."
													>
														<div class="flex flex-col gap-3">
															<ToggleButtonGroup
																bind:selected={selectedSlotAction}
																on:selected={() => {
																	replication_slot_name = ''
																}}
															>
																<ToggleButton value="create" label="Create Slot" />
																<ToggleButton value="get" label="Get Slot" />
															</ToggleButtonGroup>
															{#if selectedSlotAction === 'create'}
																<div class="flex gap-3">
																	<input
																		type="text"
																		bind:value={replication_slot_name}
																		placeholder={'Choose a slot name'}
																	/>
																	<Button
																		color="light"
																		size="xs"
																		variant="border"
																		disabled={emptyStringTrimmed(replication_slot_name)}
																		on:click={createSlot}>Create</Button
																	>
																</div>
															{:else}
																<SlotPicker
																	bind:edit
																	{database_resource_path}
																	bind:replication_slot_name
																/>
															{/if}
														</div>
													</Section>

													<Section
														label="Publication"
														tooltip="Select and manage the publications for tracking data. You can create, update, or delete publications. Only existing publications in your database will be available for selection, giving you full control over what data is tracked."
													>
														<div class="flex flex-col gap-3">
															<ToggleButtonGroup
																bind:selected={selectedPublicationAction}
																on:selected={() => {
																	if (selectedPublicationAction === 'create') {
																		selectedTable = 'specific'
																		publication_name = `windmill_publication_${random_adj()}`
																		relations = [{ schema_name: 'public', table_to_track: [] }]
																		return
																	}

																	publication_name = ''
																	relations = []
																	transaction_to_track = []
																}}
															>
																<ToggleButton value="create" label="Create Publication" />
																<ToggleButton value="get" label="Get Publication" />
															</ToggleButtonGroup>
															{#if selectedPublicationAction === 'create'}
																<div class="flex gap-3">
																	<input
																		type="text"
																		bind:value={publication_name}
																		placeholder={'Publication Name'}
																	/>
																	<Button
																		color="light"
																		size="xs"
																		variant="border"
																		disabled={emptyStringTrimmed(publication_name) ||
																			(selectedTable != 'all' && relations.length === 0)}
																		on:click={createPublication}>Create</Button
																	>
																</div>
															{:else}
																<PublicationPicker
																	{database_resource_path}
																	bind:transaction_to_track
																	bind:table_to_track
																	bind:items={publicationItems}
																	bind:publication_name
																	bind:selectedTable
																/>
															{/if}
															<RelationPicker bind:selectedTable bind:relations />
														</div>
													</Section></div
												>
											</TabContent>
										</div>
									</svelte:fragment>
								</Tabs>
							</Section>
						</div>
					</Section>
				{/if}
			</div>
		{/if}
	</DrawerContent>
</Drawer>
