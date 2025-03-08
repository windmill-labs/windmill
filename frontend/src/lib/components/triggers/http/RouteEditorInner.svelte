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
	import { json } from 'svelte-highlight/languages'
	import { Highlight } from 'svelte-highlight'
	import JsonEditor from '$lib/components/JsonEditor.svelte'
	import FileUpload from '$lib/components/common/fileUpload/FileUpload.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import S3FilePicker from '$lib/components/S3FilePicker.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import RouteEditorConfigSection from './RouteEditorConfigSection.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { isCloudHosted } from '$lib/cloud'
	let is_flow: boolean = false
	let initialPath = ''
	let edit = true

	let itemKind: 'flow' | 'script' = 'script'

	$: is_flow = itemKind === 'flow'

	let script_path = ''
	let initialScriptPath = ''
	let fixedScriptPath = ''
	let path: string = ''
	let pathError = ''
	let isValid = false
	let dirtyRoutePath = false
	let is_async = false
	let requires_auth = false
	let route_path = ''
	let http_method: 'get' | 'post' | 'put' | 'patch' | 'delete' = 'post'
	let static_asset_config: { s3: string; storage?: string; filename?: string } | undefined =
		undefined
	let is_static_website = false

	let s3FilePicker: S3FilePicker
	let s3FileUploadRawMode = false
	let s3Editor: SimpleEditor | undefined = undefined
	let workspaced_route: boolean = false

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
		drawerLoading = true
		try {
			drawer?.openDrawer()
			is_flow = nis_flow
			edit = false
			itemKind = nis_flow ? 'flow' : 'script'
			is_async = false
			requires_auth = false
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
			is_static_website = false
			workspaced_route = false
		} finally {
			drawerLoading = false
		}
	}

	const dispatch = createEventDispatcher()

	let can_write = true
	async function loadTrigger(): Promise<void> {
		const s = await HttpTriggerService.getHttpTrigger({
			workspace: $workspaceStore!,
			path: initialPath
		})
		script_path = s.script_path
		initialScriptPath = s.script_path
		workspaced_route = s.workspaced_route ?? false
		is_flow = s.is_flow
		path = s.path
		route_path = s.route_path
		http_method = s.http_method ?? 'post'
		is_async = s.is_async
		requires_auth = s.requires_auth
		if (!isCloudHosted()) {
			static_asset_config = s.static_asset_config
			s3FileUploadRawMode = !!static_asset_config
			is_static_website = s.is_static_website
		}

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
					static_asset_config,
					is_static_website,
					workspaced_route
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
					static_asset_config,
					is_static_website,
					workspaced_route
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

	$: !static_asset_config && (is_static_website = false)
</script>

{#if static_asset_config}
	<S3FilePicker
		bind:this={s3FilePicker}
		folderOnly={is_static_website}
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
						!isValid ||
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

				<Section label="Target">
					{#if !isCloudHosted()}
						<ToggleButtonGroup
							disabled={fixedScriptPath != '' || !can_write}
							selected={static_asset_config
								? is_static_website
									? 'static_website'
									: 'static_asset'
								: 'runnable'}
							on:selected={(ev) => {
								if (ev.detail === 'static_asset' || ev.detail === 'static_website') {
									static_asset_config = { s3: '' }
									s3Editor?.setCode(JSON.stringify(static_asset_config, null, 2))
									script_path = ''
									initialScriptPath = ''
									is_flow = false
									http_method = 'get'
									is_async = false
									is_static_website = ev.detail === 'static_website'
									if (is_static_website) {
										requires_auth = false
									}
								} else if (ev.detail === 'runnable') {
									static_asset_config = undefined
								}
							}}
							let:item
						>
							<ToggleButton label="Runnable" value="runnable" {item} />
							<ToggleButton label="Static asset" value="static_asset" {item} />
							<ToggleButton label="Static website" value="static_website" {item} />
						</ToggleButtonGroup>
					{/if}

					{#if static_asset_config}
						{#if is_static_website}
							<p class="text-xs my-1 text-tertiary">
								Upload or specify a <b>folder</b> on S3. All its files will be served under the path
								above. Use this full path as the base URL of your website to ensure relative imports
								work correctly.
							</p>
						{/if}
						<div class="flex flex-col gap-4">
							<div class="flex flex-col w-full gap-1">
								{#if can_write}
									<Toggle
										class="flex justify-end"
										bind:checked={s3FileUploadRawMode}
										size="xs"
										options={{ left: 'Existing file' }}
										disabled={!can_write}
									/>
								{/if}
								{#if s3FileUploadRawMode}
									{#if can_write}
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
									{:else}
										<Highlight
											language={json}
											code={JSON.stringify(static_asset_config ?? { s3: '' }, null, 2)}
										/>
									{/if}
									{#if can_write}
										<Button
											variant="border"
											color="light"
											size="xs"
											btnClasses="mt-1"
											on:click={() => {
												s3FilePicker?.open?.(!is_static_website ? static_asset_config : undefined)
											}}
											startIcon={{ icon: Pipette }}
										>
											Choose an object from the catalog
										</Button>
									{/if}
								{:else}
									{#key is_static_website}
										<FileUpload
											folderOnly={is_static_website}
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
									{/key}
								{/if}
							</div>
						</div>
					{:else}
						<p class="text-xs mt-0.5 mb-1 text-tertiary">
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
								allowRefresh={can_write}
								allowEdit={!$userStore?.operator}
							/>

							{#if script_path === undefined}
								<Button
									btnClasses="ml-4 mt-2"
									color="dark"
									size="xs"
									href={itemKind === 'flow' ? '/flows/add?hub=62' : '/scripts/add?hub=hub%2F11627'}
									target="_blank">Create from template</Button
								>
							{/if}
						</div>
					{/if}
				</Section>

				<RouteEditorConfigSection
					initialTriggerPath={initialPath}
					bind:route_path
					bind:isValid
					bind:dirtyRoutePath
					bind:http_method
					bind:workspaced_route
					{can_write}
					bind:static_asset_config
				/>

				{#if !is_static_website}
					<Section label="Advanced">
						<div class="flex flex-col gap-4">
							{#if !static_asset_config}
								<div class="flex flex-row justify-between">
									<Label label="Request type" class="w-full">
										<svelte:fragment slot="action">
											<ToggleButtonGroup
												class="w-auto h-full"
												selected={is_async ? 'async' : 'sync'}
												on:selected={({ detail }) => {
													is_async = detail === 'async'
												}}
												disabled={!can_write || !!static_asset_config}
												let:item
											>
												<ToggleButton
													label="Async"
													value="async"
													tooltip="The returning value is the uuid of the job assigned to execute the job."
													{item}
												/>
												<ToggleButton
													label="Sync"
													value="sync"
													tooltip="Triggers the execution, wait for the job to complete and return it as a response."
													{item}
												/>
											</ToggleButtonGroup>
										</svelte:fragment>
									</Label>
								</div>
							{/if}
							<Label label="Authentication" class="w-full">
								<svelte:fragment slot="action">
									<ToggleButtonGroup
										class="w-auto h-full"
										selected={requires_auth ? 'required' : 'none'}
										on:selected={({ detail }) => {
											requires_auth = detail === 'required'
										}}
										disabled={!can_write}
										let:item
									>
										<ToggleButton label="None" value="none" {item} />
										<ToggleButton
											label="Required"
											value="required"
											tooltip="Requires authentication with read access on the route"
											{item}
										/>
									</ToggleButtonGroup>
								</svelte:fragment>
							</Label>
						</div>
					</Section>
				{/if}
			</div>
		{/if}
	</DrawerContent>
</Drawer>
