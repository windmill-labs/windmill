<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
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
	import { Loader2, Save } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { random_adj } from '$lib/components/random_positive_adjetive'
	import { invalidRelations } from './utils'
	import { base } from '$lib/base'
	import PostgresEditorConfigSection from './PostgresEditorConfigSection.svelte'

	let drawer: Drawer | undefined
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
	let postgres_resource_path = ''
	let publication_name: string = ''
	let replication_slot_name: string = ''
	let relations: Relations[] | undefined = undefined
	let transaction_to_track: string[] = []
	let language: Language = 'Typescript'
	let loading = false
	let tab: 'advanced' | 'basic' = 'basic'
	let isLoading = false
	type actions = 'create' | 'get'
	let selectedPublicationAction: actions = 'create'
	let selectedSlotAction: actions = 'create'
	let isValid: boolean = false
	const dispatch = createEventDispatcher()

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
			await loadTrigger()
		} catch (err) {
			sendUserToast(`Could not load postgres trigger: ${err.body}`, true)
		} finally {
			drawerLoading = false
		}
	}

	export async function openNew(
		nis_flow: boolean,
		fixedScriptPath_?: string,
		defaultValues?: Record<string, any>
	) {
		drawerLoading = true
		try {
			selectedPublicationAction = 'create'
			selectedSlotAction = 'create'
			tab = 'basic'

			drawer?.openDrawer()
			is_flow = nis_flow
			itemKind = nis_flow ? 'flow' : 'script'
			initialScriptPath = ''
			fixedScriptPath = fixedScriptPath_ ?? ''
			script_path = fixedScriptPath
			path = ''
			initialPath = ''
			postgres_resource_path = defaultValues?.postgres_resource_path ?? ''
			edit = false
			dirtyPath = false
			publication_name = `windmill_publication_${random_adj()}`
			replication_slot_name = `windmill_replication_${random_adj()}`
			transaction_to_track = defaultValues?.publication.transaction_to_track || [
				'Insert',
				'Update',
				'Delete'
			]
			relations = defaultValues?.publication.table_to_track || [
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
		const s = await PostgresTriggerService.getPostgresTrigger({
			workspace: $workspaceStore!,
			path: initialPath
		})
		script_path = s.script_path
		initialScriptPath = s.script_path

		is_flow = s.is_flow
		path = s.path
		enabled = s.enabled
		postgres_resource_path = s.postgres_resource_path
		publication_name = s.publication_name
		replication_slot_name = s.replication_slot_name
		can_write = canWrite(s.path, s.extra_perms, $userStore)
		try {
			const publication_data = await PostgresTriggerService.getPostgresPublication({
				path: postgres_resource_path,
				workspace: $workspaceStore!,
				publication: publication_name
			})
			transaction_to_track = [...publication_data.transaction_to_track]
			relations = publication_data.table_to_track
		} catch (error) {
			sendUserToast(error.body, true)
			transaction_to_track = []
			relations = undefined
		}
	}

	async function updateTrigger(): Promise<void> {
		if (
			relations &&
			invalidRelations(relations, {
				showError: true,
				trackSchemaTableError: true
			}) === true
		) {
			return
		}
		isLoading = true
		try {
			if (edit) {
				await PostgresTriggerService.updatePostgresTrigger({
					workspace: $workspaceStore!,
					path: initialPath,
					requestBody: {
						path,
						script_path,
						is_flow,
						postgres_resource_path,
						enabled,
						replication_slot_name,
						publication_name,
						publication:
							tab === 'basic'
								? {
										transaction_to_track,
										table_to_track: relations
									}
								: undefined
					}
				})
				sendUserToast(`PostgresTrigger ${path} updated`)
			} else {
				await PostgresTriggerService.createPostgresTrigger({
					workspace: $workspaceStore!,
					requestBody: {
						path,
						script_path,
						is_flow,
						enabled: true,
						postgres_resource_path,
						replication_slot_name: tab === 'basic' ? undefined : replication_slot_name,
						publication_name: tab === 'basic' ? undefined : publication_name,
						publication: {
							transaction_to_track,
							table_to_track: relations
						}
					}
				})
				sendUserToast(`PostgresTrigger ${path} created`)
			}
			isLoading = false
			if (!$usedTriggerKinds.includes('postgres')) {
				$usedTriggerKinds = [...$usedTriggerKinds, 'postgres']
			}
			dispatch('update')
			drawer?.closeDrawer()
		} catch (error) {
			isLoading = false
			sendUserToast(error.body || error.message, true)
		}
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

	$: is_flow = itemKind === 'flow'
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
								await PostgresTriggerService.setPostgresTriggerEnabled({
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
					disabled={!isValid || pathError != '' || emptyString(script_path) || !can_write}
					on:click={updateTrigger}
					loading={isLoading}
				>
					Save
				</Button>
			{/if}
		</svelte:fragment>
		{#if drawerLoading}
			<div class="flex flex-col items-center justify-center h-full w-full">
				<Loader2 size="50" class="animate-spin" />
				<p>Loading...</p>
			</div>
		{:else}
			<Alert title="Info" type="info">
				{#if edit}
					Changes can take up to 30 seconds to take effect.
				{:else}
					New postgres triggers can take up to 30 seconds to start listening.
				{/if}
			</Alert>
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
					/>
				</Label>
				<Section label="Runnable">
					<p class="text-xs text-tertiary">
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

						{#if emptyStringTrimmed(script_path) && is_flow === false}
							<div class="flex">
								<Button
									disabled={!can_write}
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
				<PostgresEditorConfigSection
					bind:publication_name
					bind:replication_slot_name
					bind:relations
					bind:transaction_to_track
					bind:postgres_resource_path
					bind:isValid
					{edit}
					{can_write}
				/>
			</div>
		{/if}
	</DrawerContent>
</Drawer>
