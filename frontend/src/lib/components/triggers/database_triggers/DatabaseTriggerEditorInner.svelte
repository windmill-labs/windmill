<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import {
		DatabaseTriggerService,
		type Language,
		type Relations,
		type TransactionType
	} from '$lib/gen'
	import { databaseTrigger, usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, sendUserToast } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import Section from '$lib/components/Section.svelte'
	import { Loader2, Plus, Save, X } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import MultiSelect from 'svelte-multiselect'
	import { fade } from 'svelte/transition'
	import { goto } from '$app/navigation'
	import { base } from '$app/paths'

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
	let transactionType: TransactionType[] = ['Insert', 'Update', 'Delete']
	let transactionToTrack: TransactionType[] = []
	let languages = 'bun,deno'
	let language: Language = 'Typescript'
	let loading = false
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
			await loadTrigger()
		} catch (err) {
			sendUserToast(`Could not load database trigger: ${err}`, true)
		} finally {
			drawerLoading = false
		}
	}

	export async function openNew(nis_flow: boolean, fixedScriptPath_?: string) {
		drawerLoading = true
		try {
			drawer?.openDrawer()
			if ($databaseTrigger?.databaseTrigger) {
				let trigger = $databaseTrigger.databaseTrigger
				database_resource_path = trigger.database_resource_path
				is_flow = trigger.is_flow
				itemKind = is_flow ? 'flow' : 'script'
				script_path = trigger.script_path
				path = trigger.path
				replication_slot_name = trigger.replication_slot_name
				publication_name = trigger.publication_name
				databaseTrigger.set(undefined)
			} else {
				is_flow = nis_flow
				itemKind = nis_flow ? 'flow' : 'script'
				initialScriptPath = ''
				fixedScriptPath = fixedScriptPath_ ?? ''
				script_path = fixedScriptPath
				path = ''
				initialPath = ''
				relations = []
				replication_slot_name = ''
				publication_name = ''
				database_resource_path = ''
			}
			edit = false
			dirtyPath = false
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
		relations = s.table_to_track as Relations[]
		transactionToTrack = s.transaction_to_track
		publication_name = s.publication_name
		replication_slot_name = s.replication_slot_name
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
					replication_slot_name,
					publication_name,
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
		if (relations.length === 0) {
			sendUserToast('You must at least choose one schema', true)
			return
		}
		if (!database_resource_path) {
			sendUserToast('You must pick a database resource first', true)
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
					publication_name,
				}
			})
			await goto(`${base}/scripts/add`)
		} catch (error) {
			loading = false
			console.log({ error })
		}
	}
</script>

<Drawer size="800px" bind:this={drawer}>
	<DrawerContent
		title={edit
			? can_write
				? `Edit Database trigger ${initialPath}`
				: `Database trigger ${initialPath}`
			: 'New Database trigger'}
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
									`${e.detail ? 'enabled' : 'disabled'} database trigger ${initialPath}`
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
						emptyString(replication_slot_name) ||
						emptyString(publication_name) ||
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
			<Alert title="Info" type="info">
				{#if edit}
					Changes can take up to 30 seconds to take effect.
				{:else}
					New database triggers can take up to 30 seconds to start listening.
				{/if}
			</Alert>
			<div class="flex flex-col gap-12 mt-6">
				<div class="flex flex-col gap-4">
					<Label label="Path">
						<Path
							bind:dirty={dirtyPath}
							bind:error={pathError}
							bind:path
							{initialPath}
							checkInitialPathExistence={!edit}
							namePlaceholder="database_trigger"
							kind="database_trigger"
							disabled={!can_write}
						/>
					</Label>
				</div>

				<Section label="Database">
					<p class="text-xs mb-1 text-tertiary">
						Pick a database to connect to<Required required={true} />
					</p>
					<div class="flex flex-row mb-2">
						<ResourcePicker bind:value={database_resource_path} resourceType={'database'} />
					</div>
				</Section>

				<Section label="Transactions">
					<p class="text-xs mb-1 text-tertiary">
						Choose what kind of database transaction you want to track allowed operations are
						<strong>Inser</strong>t, <strong>Update</strong>, <strong>Delete</strong><Required
							required={true}
						/>
					</p>

					<MultiSelect
						options={transactionType}
						bind:selected={transactionToTrack}
						duplicates={false}
						liOptionClass={'box'}
					/>
				</Section>

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
							bind:languages
							allowRefresh
						/>

						{#if script_path === undefined && is_flow === false}
							<Button
								btnClasses="ml-4 mt-2"
								color="dark"
								size="xs"
								on:click={getTemplateScript}
								target="_blank"
								{loading}
								>Create from template
							</Button>
						{/if}
					</div>
				</Section>

				<Section label="Settings">
					<div class="flex flex-col gap-2">
						<div>
							<p class="text-xs mb-1 text-tertiary">
								Choose a publication name<Required required={true} />
							</p>
							<input type="text" bind:value={publication_name} placeholder={'Publication Name'} />
						</div>
						<div>
							<p class="text-xs mb-1 text-tertiary">
								Choose a slot name<Required required={true} />
							</p>
							<input
								type="text"
								bind:value={replication_slot_name}
								placeholder={'Replication Slot Name'}
							/>
						</div>
					</div>
				</Section>

				<Section label="Relations">
					<p class="text-xs mb-3 text-tertiary">
						Relations will limit the execution of the trigger to only the specified tables.<br />
						If no tables are selected, this will trigger for all tables.<br />
					</p>

					<div class="flex flex-col gap-4 mt-1">
						{#if relations && relations.length > 0}
							{#each relations as v, i}
								<div class="flex w-full gap-2 items-center">
									<div class="w-full flex flex-col gap-2 border p-2 rounded-md">
										<label class="flex flex-col w-full">
											<div class="text-secondary text-sm mb-2">Schema Name</div>
											<input type="text" bind:value={v.schema_name} />
										</label>
										{#each v.table_to_track as table_to_track, j}
											<div class="flex w-full gap-2 items-center p-5">
												<div class="rounded shadow-inner p-2 flex w-full flex-col gap-4 mt-1">
													<label class="flex flex-col w-full">
														<div class="text-secondary text-sm mb-2">Table Name</div>
														<input type="text" bind:value={table_to_track.table_name} />
													</label>
													<!-- svelte-ignore a11y-label-has-associated-control -->
													<label class="flex flex-col w-full">
														<div class="text-secondary text-sm mb-2">Columns</div>
														<MultiSelect
															options={table_to_track.columns_name ?? []}
															allowUserOptions="append"
															bind:selected={table_to_track.columns_name}
															noMatchingOptionsMsg=""
															createOptionMsg={null}
															duplicates={false}
														/>
													</label>
													<label class="flex flex-col w-full">
														<div class="text-secondary text-sm mb-2">Where Clause</div>
														<input type="text" bind:value={table_to_track.where_clause} />
													</label>
													<button
														transition:fade|local={{ duration: 100 }}
														class="rounded items-center p-1 bg-surface-secondary duration-200 hover:bg-surface-hover"
														aria-label="Clear"
														on:click={() => {
															v.table_to_track = v.table_to_track.filter((_, index) => index !== j)
														}}
													>
														Remove
													</button>
												</div>
											</div>
										{/each}
										<Button
											variant="border"
											color="light"
											size="xs"
											btnClasses="mt-1"
											on:click={() => {
												if (
													relations[i].table_to_track == undefined ||
													!Array.isArray(relations[i].table_to_track)
												) {
													relations[i].table_to_track = []
												}
												relations[i].table_to_track = relations[i].table_to_track.concat({
													table_name: '',
													columns_name: []
												})
											}}
											startIcon={{ icon: Plus }}
										>
											Add Table
										</Button>
									</div>
									<button
										transition:fade|local={{ duration: 100 }}
										class="rounded-full p-1 bg-surface-secondary duration-200 hover:bg-surface-hover"
										aria-label="Clear"
										on:click={() => {
											relations = relations.filter((_, index) => index !== i)
										}}
									>
										<X size={14} />
									</button>
								</div>
							{/each}
						{/if}
						<div class="flex items-baseline">
							<Button
								variant="border"
								color="light"
								size="xs"
								btnClasses="mt-1"
								on:click={() => {
									if (relations == undefined || !Array.isArray(relations)) {
										relations = []
									}
									relations = relations.concat({
										schema_name: '',
										table_to_track: []
									})
								}}
								startIcon={{ icon: Plus }}
							>
								Add Schema
							</Button>
						</div>
					</div></Section
				>
			</div>
		{/if}
	</DrawerContent>
</Drawer>
