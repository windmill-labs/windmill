<script lang="ts">
	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import { HttpTriggerService } from '$lib/gen'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, sendUserToast } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import Section from '$lib/components/Section.svelte'
	import { Loader2, Save, Pipette } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import S3FilePicker from '../S3FilePicker.svelte'
	import Toggle from '../Toggle.svelte'
	import JsonEditor from '../apps/editor/settingsPanel/inputEditor/JsonEditor.svelte'
	import FileUpload from '../common/fileUpload/FileUpload.svelte'
	import SimpleEditor from '../SimpleEditor.svelte'
	import RouteEditorConfigSection from './RouteEditorConfigSection.svelte'

	let is_flow: boolean = false
	let initialPath = ''
	let edit = true

	let itemKind: 'flow' | 'script' = 'script'

	$: is_flow = itemKind === 'flow'

	let script_path = ''
	let initialScriptPath = ''
	let fixedScriptPath = ''

	let drawerLoading = true
	export async function openEdit(ePath: string, isFlow: boolean) {
		drawerLoading = true
		try {
			drawer?.openDrawer()
			initialPath = ePath
			itemKind = isFlow ? 'flow' : 'script'
			edit = true
			dirtyPath = false
			dirtyRoutePath = false
			await loadTrigger()
		} catch (err) {
			sendUserToast(`Could not load route: ${err}`, true)
		} finally {
			drawerLoading = false
		}
	}

	export async function openNew(
		nis_flow: boolean,
		fixedScriptPath_?: string,
		defaultValues?: Record<string, any>
	) {
		console.log('dbg openNew', defaultValues?.http_method)
		drawerLoading = true
		try {
			drawer?.openDrawer()
			is_flow = nis_flow
			edit = false
			itemKind = nis_flow ? 'flow' : 'script'
			is_async = false
			requires_auth = false
			initialRoutePath = ''
			route_path = defaultValues?.route_path ?? ''
			dirtyRoutePath = false
			http_method = defaultValues?.http_method ?? 'post'
			initialScriptPath = ''
			fixedScriptPath = fixedScriptPath_ ?? ''
			script_path = fixedScriptPath
			static_asset_config = undefined
			s3FileUploadRawMode = false
			path = ''
			initialPath = ''
			dirtyPath = false
		} finally {
			drawerLoading = false
		}
	}

	let path: string = ''
	let pathError = ''
	let routeError = ''
	let dirtyRoutePath = false
	let is_async = false
	let requires_auth = false
	let initialRoutePath = ''
	let route_path = ''
	let http_method: 'get' | 'post' | 'put' | 'patch' | 'delete' = 'post'
	let static_asset_config: { s3: string; storage?: string; filename?: string } | undefined =
		undefined

	let s3FilePicker: S3FilePicker
	let s3FileUploadRawMode = false
	let s3Editor: SimpleEditor | undefined = undefined

	const dispatch = createEventDispatcher()

	let can_write = true
	async function loadTrigger(): Promise<void> {
		const s = await HttpTriggerService.getHttpTrigger({
			workspace: $workspaceStore!,
			path: initialPath
		})
		script_path = s.script_path
		initialScriptPath = s.script_path

		is_flow = s.is_flow
		path = s.path
		route_path = s.route_path
		initialRoutePath = s.route_path
		http_method = s.http_method ?? 'post'
		is_async = s.is_async
		requires_auth = s.requires_auth
		static_asset_config = s.static_asset_config
		s3FileUploadRawMode = !!static_asset_config

		can_write = canWrite(s.path, s.extra_perms, $userStore)
	}

	async function triggerScript(): Promise<void> {
		if (edit) {
			await HttpTriggerService.updateHttpTrigger({
				workspace: $workspaceStore!,
				path: initialPath,
				requestBody: {
					path,
					script_path,
					is_flow,
					is_async,
					requires_auth,
					route_path: $userStore?.is_admin || $userStore?.is_super_admin ? route_path : undefined,
					http_method,
					static_asset_config
				}
			})
			sendUserToast(`Route ${path} updated`)
		} else {
			await HttpTriggerService.createHttpTrigger({
				workspace: $workspaceStore!,
				requestBody: {
					path,
					script_path,
					is_flow,
					is_async,
					requires_auth,
					route_path,
					http_method,
					static_asset_config
				}
			})
			sendUserToast(`Route ${path} created`)
		}
		if (!$usedTriggerKinds.includes('http')) {
			$usedTriggerKinds = [...$usedTriggerKinds, 'http']
		}
		dispatch('update')
		drawer.closeDrawer()
	}

	let drawer: Drawer

	let dirtyPath = false

	let args: Record<string, any> = { route_path: '' }

	$: args && (route_path = args.route_path)
</script>

{#if static_asset_config}
	<S3FilePicker
		bind:this={s3FilePicker}
		bind:selectedFileKey={static_asset_config}
		on:close={() => {
			s3Editor?.setCode(JSON.stringify(static_asset_config, null, 2))
		}}
		readOnlyMode={false}
	/>
{/if}

<Drawer size="700px" bind:this={drawer}>
	<DrawerContent
		title={edit ? (can_write ? `Edit route ${initialPath}` : `Route ${initialPath}`) : 'New route'}
		on:close={drawer.closeDrawer}
	>
		<svelte:fragment slot="actions">
			{#if !drawerLoading && can_write}
				<Button
					startIcon={{ icon: Save }}
					disabled={pathError != '' ||
						routeError != '' ||
						(!static_asset_config && emptyString(script_path)) ||
						(static_asset_config && emptyString(static_asset_config.s3)) ||
						!can_write}
					on:click={triggerScript}
				>
					Save
				</Button>
			{/if}
		</svelte:fragment>
		{#if drawerLoading}
			<Loader2 class="animate-spin" />
		{:else}
			<div class="flex flex-col gap-12">
				<div class="flex flex-col gap-4">
					<Label label="Path">
						<Path
							bind:dirty={dirtyPath}
							bind:error={pathError}
							bind:path
							{initialPath}
							checkInitialPathExistence={!edit}
							namePlaceholder="route"
							kind="http_trigger"
							hideUser
							disabled={!can_write}
						/>
					</Label>
				</div>

				<RouteEditorConfigSection
					bind:route_path
					bind:args
					bind:routeError
					bind:dirtyRoutePath
					bind:http_method
					{can_write}
					bind:static_asset_config
					{initialRoutePath}
				/>

				<Section label="Target">
					<ToggleButtonGroup
						disabled={fixedScriptPath != '' || !can_write}
						selected={static_asset_config ? 'static_asset' : 'runnable'}
						on:selected={(ev) => {
							if (ev.detail === 'static_asset') {
								static_asset_config = { s3: '' }
								script_path = ''
								initialScriptPath = ''
								is_flow = false
								http_method = 'get'
							} else {
								static_asset_config = undefined
							}
						}}
					>
						<ToggleButton label="Runnable" value="runnable" />
						<ToggleButton label="Static asset" value="static_asset" />
					</ToggleButtonGroup>

					{#if static_asset_config}
						<div class="flex flex-col w-full gap-1">
							<Toggle
								class="flex justify-end"
								bind:checked={s3FileUploadRawMode}
								size="xs"
								options={{ left: 'Existing file' }}
							/>
							{#if s3FileUploadRawMode}
								<JsonEditor
									bind:editor={s3Editor}
									on:focus={(e) => {
										dispatch('focus')
									}}
									on:blur={(e) => {
										dispatch('blur')
									}}
									code={JSON.stringify(static_asset_config ?? { s3: '' }, null, 2)}
									bind:value={static_asset_config}
								/>
								<Button
									variant="border"
									color="light"
									size="xs"
									btnClasses="mt-1"
									on:click={() => {
										s3FilePicker?.open?.(static_asset_config)
									}}
									startIcon={{ icon: Pipette }}
								>
									Choose an object from the catalog
								</Button>
							{:else}
								<FileUpload
									allowMultiple={false}
									randomFileKey={true}
									on:addition={(evt) => {
										static_asset_config = {
											s3: evt.detail?.path ?? '',
											filename: evt.detail?.filename ?? undefined
										}
										s3FileUploadRawMode = true
									}}
									on:deletion={(evt) => {
										static_asset_config = {
											s3: ''
										}
									}}
								/>
							{/if}
						</div>
					{:else}
						<p class="text-xs mb-1 text-tertiary">
							Pick a script or flow to be triggered<Required required={true} /><br />
							To handle headers, query or path parameters, add a preprocessor to your runnable.
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

							{#if script_path === undefined}
								<Button
									btnClasses="ml-4 mt-2"
									color="dark"
									size="xs"
									href={itemKind === 'flow'
										? '/flows/add?hub=55'
										: '/scripts/add?hub=hub%2F9088%2Fwindmill%2FHTTP%20route%20script%20with%20preprocessor%20template'}
									target="_blank">Create from template</Button
								>
							{/if}
						</div>
					{/if}
				</Section>

				<Section label="Settings">
					<div class="flex flex-col gap-4">
						<div class="flex flex-row justify-between">
							<div class="text-sm font-semibold flex flex-row items-center">Request type</div>
							<ToggleButtonGroup class="w-auto" bind:selected={is_async} disabled={!can_write}>
								<ToggleButton
									label="Async"
									value={true}
									tooltip="The returning value is the uuid of the job assigned to execute the job."
								/>
								<ToggleButton
									label="Sync"
									value={false}
									tooltip="Triggers the execution, wait for the job to complete and return it as a response."
								/>
							</ToggleButtonGroup>
						</div>
						<div class="flex flex-row justify-between">
							<div class="text-sm font-semibold flex flex-row items-center">Authentication</div>
							<ToggleButtonGroup class="w-auto" bind:selected={requires_auth} disabled={!can_write}>
								<ToggleButton label="None" value={false} />
								<ToggleButton
									label="Required"
									value={true}
									tooltip="Requires authentication with read access on the route"
								/>
							</ToggleButtonGroup>
						</div>
					</div>
				</Section>
			</div>
		{/if}
	</DrawerContent>
</Drawer>
