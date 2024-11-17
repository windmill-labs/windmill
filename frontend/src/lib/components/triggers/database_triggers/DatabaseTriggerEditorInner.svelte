<script lang="ts">
	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import { DatabaseTriggerService, type DatabaseToTrack, type TableToTrack } from '$lib/gen'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, sendUserToast } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import Section from '$lib/components/Section.svelte'
	import { Loader2, Save } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import Toggle from '$lib/components/Toggle.svelte'

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
	let url = ''
	let urlError = ''
	let dirtyUrl = false
	let enabled = false
	let database: DatabaseToTrack
	let tableToTrack: TableToTrack[] | undefined
	let dirtyPath = false
	let can_write = true
	let drawerLoading = true

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
			dirtyUrl = false
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
			url = ''
			dirtyUrl = false
			initialScriptPath = ''
			fixedScriptPath = fixedScriptPath_ ?? ''
			script_path = fixedScriptPath
			path = ''
			initialPath = ''
			database = { username: '', password: '', host: '', port: 0 }
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
		database = s.database
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
					database,
					table_to_track: tableToTrack
				}
			})
			sendUserToast(`Route ${path} updated`)
		} else {
			await DatabaseTriggerService.createDatabaseTrigger({
				workspace: $workspaceStore!,
				requestBody: {
					path,
					script_path,
					is_flow,
					enabled: true,
					database,
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
					disabled={pathError != '' || urlError != '' || emptyString(script_path) || !can_write}
					on:click={updateTrigger}
				>
					Save
				</Button>
			{/if}
		</svelte:fragment>
		{#if drawerLoading}
			<Loader2 class="animate-spin" />
		{:else}
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
			</div>
		{/if}
	</DrawerContent>
</Drawer>
