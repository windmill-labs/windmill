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
	import { Loader2, Save } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import MultiSelect from '$lib/components/multiselect/MultiSelect.svelte'

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
	let tableToTrack: TableToTrack[] | undefined
	let dirtyPath = false
	let can_write = true
	let drawerLoading = true
	let database_resource_path = ''
	let publication_name: string = ''
	let replication_slot_name: string = ''
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
		tableToTrack = s.table_to_track
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
					table_to_track: tables.map((table) => {
						return {
							table_name: table,
							columns_name: []
						}
					})
				}
			})
			sendUserToast(`Route ${path} updated`)
		} else {
			await DatabaseTriggerService.createDatabaseTrigger({
				workspace: $workspaceStore!,
				requestBody: {
					transaction_type: selected,
					path,
					script_path,
					is_flow,
					enabled: true,
					database_resource_path,
					replication_slot_name,
					publication_name,
					table_to_track: tables.map((table) => {
						return {
							table_name: table,
							columns_name: []
						}
					})
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

	let selected: TransactionType = 'Insert'
	let tables: string[] = []
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

				<Section label="Publication">
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
						Choose what kind of database transaction you want to track<Required required={true} />
					</p>
					<ToggleButtonGroup bind:selected>
						<ToggleButton value="Insert" label="Insert" />
						<ToggleButton value="Update" label="Update" />
						<ToggleButton value="Delete" label="Delete" />
					</ToggleButtonGroup>
				</Section>
				<Section label="Tables">
					<p class="text-xs mb-3 text-tertiary">
						Tables will limit the execution of the trigger to only the specified tables.<br />
						If no tables are selected, this will trigger for all tables.<br />
						<strong
							>If no fully qualified table names (e.g., <code>schema_name.table_name</code>) are
							provided, the system will use the default table names from your current database
							schema for the search.</strong
						><br />
						<strong>Example:</strong> If your schema is <code>public</code> and you select a table
						called <code>users</code>, the system will look for <code>public.users</code> by
						default. If you wish to specify a different schema, you can provide a fully qualified
						name like <code>my_schema.users</code>.
					</p>
					<MultiSelect
						options={tables}
						allowUserOptions="append"
						bind:selected={tables}
						noMatchingOptionsMsg=""
						createOptionMsg={null}
						duplicates={false}
					/>
				</Section>
			</div>
		{/if}
	</DrawerContent>
</Drawer>
