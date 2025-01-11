<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import { DatabaseTriggerService, type Language, type Relations } from '$lib/gen'
	import { databaseTrigger, usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, sendUserToast } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import Section from '$lib/components/Section.svelte'
	import { Loader2, Save } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import { goto } from '$app/navigation'
	import { base } from '$app/paths'
	import ConfigurationEditor from './ConfigurationEditor.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

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
	let configurationEditor: ConfigurationEditor
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
					publication_name
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
					publication_name
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
					publication_name
				}
			})
			await goto(`${base}/scripts/add`)
		} catch (error) {
			loading = false
			console.log({ error })
		}
	}
</script>

<ConfigurationEditor
	{edit}
	{database_resource_path}
	bind:relations
	bind:publication_name
	bind:transaction_to_track
	bind:replication_slot_name
	bind:this={configurationEditor}
/>

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
						Pick a database to connect to <Required required={true} />
					</p>
					<div class="flex flex-row mb-2">
						<ResourcePicker bind:value={database_resource_path} resourceType={'postgresql'} />
					</div>
				</Section>

				<Section label="Configuration">
					<p class="text-xs mb-3 text-tertiary">
						Choose which table of your database to track as well as what kind of transaction should
						fire the script.<br />
						You must pick a database resource first to make the configuration of your trigger
						<Required required={true} />
					</p>
					<Button
						size="md"
						on:click={() => configurationEditor.openNew()}
						color="dark"
						disabled={emptyString(database_resource_path)}
					>
						Configuration</Button
					>
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
									disabled={emptyString(database_resource_path) ||
										emptyString(replication_slot_name) ||
										emptyString(publication_name) ||
										relations.length === 0}
									>Create from template
								</Button>
								<Tooltip
									>To enable that features,Select a database resource, and inside database config
									create or get a publication from your database</Tooltip
								>
							</div>
						{/if}
					</div>
				</Section>
			</div>
		{/if}
	</DrawerContent>
</Drawer>
