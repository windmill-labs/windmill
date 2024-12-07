<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import { DatabaseTriggerService, type TableToTrack, type TransactionType } from '$lib/gen'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, sendUserToast } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import Section from '$lib/components/Section.svelte'
	import { Loader2, Plus, Save, X } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import MultiSelect from '$lib/components/multiselect/MultiSelect.svelte'
	import { fade } from 'svelte/transition'

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
	let tableToTrack: TableToTrack[] = []
	let transactionType: TransactionType[] = ['Insert', 'Update', 'Delete']
	let transactionValue: TransactionType[] = []
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
			is_flow = nis_flow
			edit = false
			itemKind = nis_flow ? 'flow' : 'script'
			initialScriptPath = ''
			fixedScriptPath = fixedScriptPath_ ?? ''
			script_path = fixedScriptPath
			path = ''
			initialPath = ''
			tableToTrack = []
			dirtyPath = false
			replication_slot_name = ''
			publication_name = ''
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
		tableToTrack = s.table_to_track as TableToTrack[]
		transactionValue = s.transaction_to_track
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
					table_to_track: tableToTrack
				}
			})
			sendUserToast(`Route ${path} updated`)
		} else {
			await DatabaseTriggerService.createDatabaseTrigger({
				workspace: $workspaceStore!,
				requestBody: {
					transaction_to_track: transactionValue,
					path,
					script_path,
					is_flow,
					enabled: true,
					database_resource_path,
					replication_slot_name,
					publication_name,
					table_to_track: tableToTrack
				}
			})
			sendUserToast(`Route ${path} created`)
		}

		if (!$usedTriggerKinds.includes('database')) {
			$usedTriggerKinds = [...$usedTriggerKinds, 'database']
		}
		dispatch('update')
		drawer.closeDrawer()
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
							allowRefresh
						/>
					</div>
				</Section>
				<Section label="Database">
					<p class="text-xs mb-1 text-tertiary">
						Pick a database to connect to<Required required={true} />
					</p>
					<div class="flex flex-row mb-2">
						<ResourcePicker bind:value={database_resource_path} resourceType={'database'} />
					</div>
				</Section>

				<Section label="Publication">
					<p class="text-xs mb-1 text-tertiary">
						Choose a publication name<Required required={true} />
					</p>
					<input type="text" bind:value={publication_name} placeholder={'Publication Name'} />
				</Section>

				<Section label="Slot Name">
					<p class="text-xs mb-1 text-tertiary">
						Choose a slot name<Required required={true} />
					</p>
					<input
						type="text"
						bind:value={replication_slot_name}
						placeholder={'Replication Slot Name'}
					/>
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
						bind:selected={transactionValue}
						duplicates={false}
					/>
				</Section>

				<Section label="Tables">
					<p class="text-xs mb-3 text-tertiary">
						Tables will limit the execution of the trigger to only the specified tables.<br />
						If no tables are selected, this will trigger for all tables.<br />
					</p>

					<div class="flex flex-col gap-4 mt-1">
						{#each tableToTrack as v, i}
							<div class="flex w-full gap-2 items-center">
								<div class="w-full flex flex-col gap-2 border p-2 rounded-md">
									<label class="flex flex-col w-full">
										<div class="text-secondary text-sm mb-2">Table Name</div>
										<input type="text" bind:value={v.table_name} />
									</label>
									<!-- svelte-ignore a11y-label-has-associated-control -->
									<label class="flex flex-col w-full">
										<div class="text-secondary text-sm mb-2">Columns</div>
										<MultiSelect
											options={v.columns_name}
											allowUserOptions="append"
											bind:selected={v.columns_name}
											noMatchingOptionsMsg=""
											createOptionMsg={null}
											duplicates={false}
										/>
									</label>
								</div>
								<button
									transition:fade|local={{ duration: 100 }}
									class="rounded-full p-1 bg-surface-secondary duration-200 hover:bg-surface-hover"
									aria-label="Clear"
									on:click={() => {
										tableToTrack = tableToTrack.filter((_, index) => index !== i)
									}}
								>
									<X size={14} />
								</button>
							</div>
						{/each}

						<div class="flex items-baseline">
							<Button
								variant="border"
								color="light"
								size="xs"
								btnClasses="mt-1"
								on:click={() => {
									if (tableToTrack == undefined || !Array.isArray(tableToTrack)) {
										tableToTrack = []
									}
									tableToTrack = tableToTrack.concat({
										table_name: '',
										columns_name: []
									})
								}}
								startIcon={{ icon: Plus }}
							>
								Add item
							</Button>
						</div>
					</div></Section
				>
			</div>
		{/if}
	</DrawerContent>
</Drawer>
