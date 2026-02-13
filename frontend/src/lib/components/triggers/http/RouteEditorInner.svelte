<script lang="ts">
	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import {
		HttpTriggerService,
		VariableService,
		type AuthenticationMethod,
		type ErrorHandler,
		type HttpTrigger,
		type NewHttpTrigger,
		type Retry,
		type TriggerMode
	} from '$lib/gen'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import {
		canWrite,
		capitalize,
		emptyString,
		generateRandomString,
		sendUserToast
	} from '$lib/utils'
	import Section from '$lib/components/Section.svelte'
	import { Loader2, Pipette, Plus } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import VariableEditor from '../../VariableEditor.svelte'
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
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import ItemPicker from '../../ItemPicker.svelte'
	import { Popover } from '$lib/components/meltComponents'
	import { HUB_SCRIPT_ID, saveHttpRouteFromCfg, SECRET_KEY_PATH } from './utils'
	import { HubFlow } from '$lib/hub'
	import RouteBodyTransformerOption from './RouteBodyTransformerOption.svelte'
	import TestingBadge from '../testingBadge.svelte'
	import TriggerEditorToolbar from '../TriggerEditorToolbar.svelte'
	import { getHandlerType, handleConfigChange } from '../utils'
	import autosize from '$lib/autosize'
	import { untrack } from 'svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import TriggerRetriesAndErrorHandler from '../TriggerRetriesAndErrorHandler.svelte'
	import { deepEqual } from 'fast-equals'
	import TriggerSuspendedJobsAlert from '../TriggerSuspendedJobsAlert.svelte'
	import TriggerSuspendedJobsModal from '../TriggerSuspendedJobsModal.svelte'
	import UserSettings from '$lib/components/UserSettings.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	let {
		useDrawer = true,
		hideTarget = false,
		description = undefined,
		isEditor = false,
		customLabel = undefined,
		allowDraft = false,
		isDeployed = false,
		onConfigChange = undefined,
		onCaptureConfigChange = undefined,
		onUpdate = undefined,
		onDelete = undefined,
		onReset = undefined,
		trigger = undefined,
		customSaveBehavior = undefined
	} = $props()

	// Form data state
	let initialPath = $state('')
	let edit = $state(true)
	let itemKind = $state<'flow' | 'script'>('script')
	let is_flow = $state(false)
	let script_path = $state('')
	let initialScriptPath = $state('')
	let fixedScriptPath = $state('')
	let path = $state('')
	let pathError = $state('')
	let isValid = $state(false)
	let dirtyRoutePath = $state(false)
	let dirtyPath = $state(false)
	let request_type = $state<'sync' | 'async' | 'sync_sse'>('sync')
	let authentication_method = $state<AuthenticationMethod>('none')
	let route_path = $state('')
	let http_method = $state<'get' | 'post' | 'put' | 'patch' | 'delete'>('post')
	let static_asset_config = $state<{ s3: string; storage?: string; filename?: string } | undefined>(
		undefined
	)
	let mode = $state<TriggerMode>('enabled')
	let is_static_website = $state(false)
	let s3FileUploadRawMode = $state(false)
	let workspaced_route = $state(false)
	let raw_string = $state(false)
	let wrap_body = $state(false)
	let drawerLoading = $state(true)
	let showLoader = $state(false)
	let authentication_resource_path = $state('')
	let variable_path = $state('')
	let signature_options_type = $state<'custom_script' | 'custom_signature'>('custom_signature')
	let can_write = $state(true)
	let extraPerms = $state<Record<string, boolean> | undefined>(undefined)
	let summary: string | undefined = $state()
	let routeDescription: string | undefined = $state()
	let error_handler_path: string | undefined = $state()
	let error_handler_args: Record<string, any> = $state({})
	let retry: Retry | undefined = $state()
	// Component references
	let s3FilePicker = $state<S3FilePicker | undefined>(undefined)
	let s3Editor = $state<SimpleEditor | undefined>(undefined)
	let variablePicker = $state<ItemPicker | undefined>(undefined)
	let variableEditor = $state<VariableEditor | undefined>(undefined)
	let drawer = $state<Drawer | undefined>(undefined)
	let initialConfig: NewHttpTrigger | undefined = undefined
	let deploymentLoading = $state(false)
	let optionTabSelected: 'request_options' | 'error_handler' | 'retries' = $state('request_options')
	let errorHandlerSelected: ErrorHandler = $state('slack')

	let suspendedJobsModal = $state<TriggerSuspendedJobsModal | null>(null)
	let originalConfig = $state<NewHttpTrigger | undefined>(undefined)
	let userSettings = $state<UserSettings | undefined>(undefined)

	let hasChanged = $derived(!deepEqual(getRouteConfig(), originalConfig ?? {}))
	let scopes = $derived(['http_triggers:read:' + path])

	const isAdmin = $derived($userStore?.is_admin || $userStore?.is_super_admin)
	const routeConfig = $derived.by(getRouteConfig)
	const captureConfig = $derived.by(isEditor ? getCaptureConfig : () => ({}))
	const saveDisabled = $derived(
		drawerLoading ||
			!can_write ||
			pathError != '' ||
			!isValid ||
			(!static_asset_config && emptyString(script_path)) ||
			!hasChanged
	)

	$effect(() => {
		is_flow = itemKind === 'flow'
	})

	// Update is_static_website based on static_asset_config
	$effect(() => {
		if (!static_asset_config) {
			is_static_website = false
		}
	})

	type AuthenticationOption = {
		label: string
		value: AuthenticationMethod
		tooltip?: string
		resource_type?: string
	}

	async function loadVariables() {
		return await VariableService.listVariable({ workspace: $workspaceStore ?? '' })
	}

	const authentication_options: AuthenticationOption[] = [
		{
			label: 'No Auth',
			value: 'none'
		},
		{
			label: 'Windmill',
			value: 'windmill',
			tooltip: 'Requires the user to be authenticated with read access to this route'
		},
		{
			label: 'API Key',
			value: 'api_key',
			resource_type: 'api_key_auth',
			tooltip:
				'Checks for a valid API key in a specified header. Header name can be configured in the resource.'
		},
		{
			label: 'Basic Auth',
			value: 'basic_http',
			resource_type: 'basic_http_auth',
			tooltip:
				'Uses base64-encoded Basic authentication. Defaults to the Authorization header, but a custom header can be configured in the resource.'
		},
		{
			label: 'Signature',
			value: 'signature',
			tooltip:
				'Validates requests using HMAC or signature-based authentication. Can be custom or follow predefined formats from common providers.'
		}
	]

	export async function openEdit(
		ePath: string,
		isFlow: boolean,
		defaultConfig?: Partial<NewHttpTrigger>
	) {
		drawerLoading = true
		let loader = setTimeout(() => {
			showLoader = true
		}, 100) // if loading takes less than 100ms, we don't show the loader
		try {
			drawer?.openDrawer()
			initialPath = ePath
			path = ePath
			itemKind = isFlow ? 'flow' : 'script'
			edit = true
			dirtyPath = false
			dirtyRoutePath = false
			await loadTrigger(defaultConfig)
			originalConfig = structuredClone($state.snapshot(getRouteConfig()))
		} catch (err) {
			sendUserToast(`Could not load route: ${err}`, true)
		} finally {
			if (!defaultConfig) {
				// If the route is loaded from the backend, we to set the initial config
				initialConfig = structuredClone($state.snapshot(getRouteConfig()))
			}
			clearTimeout(loader)
			drawerLoading = false
			showLoader = false
		}
	}

	export async function openNew(
		nis_flow: boolean,
		fixedScriptPath_?: string,
		defaultValues?: Partial<HttpTrigger> & {
			s3FileUploadRawMode?: boolean
			signature_options_type?: 'custom_script' | 'custom_signature'
		}
	) {
		drawerLoading = true
		let loader = setTimeout(() => {
			showLoader = true
		}, 100) // if loading takes less than 100ms, we don't show the loader
		try {
			drawer?.openDrawer()
			is_flow = defaultValues?.is_flow ?? nis_flow
			edit = false
			itemKind = nis_flow ? 'flow' : 'script'
			request_type = defaultValues?.request_type ?? 'sync'
			authentication_method = defaultValues?.authentication_method ?? 'none'
			route_path = defaultValues?.route_path ?? ''
			dirtyRoutePath = false
			http_method = defaultValues?.http_method ?? 'post'
			initialScriptPath = ''
			fixedScriptPath = fixedScriptPath_ ?? ''
			script_path = fixedScriptPath
			static_asset_config = defaultValues?.static_asset_config ?? undefined
			s3FileUploadRawMode = defaultValues?.s3FileUploadRawMode ?? false
			path = defaultValues?.path ?? ''
			initialPath = ''
			mode = defaultValues?.mode ?? 'enabled'
			dirtyPath = false
			is_static_website = defaultValues?.is_static_website ?? false
			workspaced_route = defaultValues?.workspaced_route ?? false
			authentication_resource_path = defaultValues?.authentication_resource_path ?? ''
			variable_path = ''
			signature_options_type = defaultValues?.signature_options_type ?? 'custom_signature'
			raw_string = defaultValues?.raw_string ?? false
			wrap_body = defaultValues?.wrap_body ?? false
			summary = defaultValues?.summary ?? ''
			routeDescription = defaultValues?.description ?? ''
			error_handler_path = defaultValues?.error_handler_path ?? undefined
			error_handler_args = defaultValues?.error_handler_args ?? {}
			retry = defaultValues?.retry ?? undefined
			errorHandlerSelected = getHandlerType(error_handler_path ?? '')
			originalConfig = undefined
		} finally {
			clearTimeout(loader)
			drawerLoading = false
			showLoader = false
		}
	}

	function loadTriggerConfig(cfg?: Partial<HttpTrigger>): void {
		script_path = cfg?.script_path ?? ''
		initialScriptPath = cfg?.script_path ?? ''
		is_flow = cfg?.is_flow ?? false
		path = cfg?.path ?? ''
		route_path = cfg?.route_path ?? ''
		http_method = cfg?.http_method ?? 'post'
		request_type = cfg?.request_type ?? 'sync'
		workspaced_route = cfg?.workspaced_route ?? false
		wrap_body = cfg?.wrap_body ?? false
		raw_string = cfg?.raw_string ?? false
		summary = cfg?.summary ?? ''
		mode = cfg?.mode ?? 'enabled'
		routeDescription = cfg?.description ?? ''
		authentication_resource_path = cfg?.authentication_resource_path ?? ''
		if (cfg?.authentication_method === 'custom_script') {
			authentication_method = 'signature'
			signature_options_type = 'custom_script'
		} else {
			authentication_method = cfg?.authentication_method ?? 'none'
			signature_options_type = 'custom_signature'
		}
		if (!isCloudHosted()) {
			static_asset_config = cfg?.static_asset_config ?? undefined
			s3FileUploadRawMode = !!cfg?.static_asset_config
			is_static_website = cfg?.is_static_website ?? false
		}
		extraPerms = cfg?.extra_perms ?? undefined
		can_write = canWrite(path, cfg?.extra_perms ?? {}, $userStore)
		error_handler_path = cfg?.error_handler_path
		error_handler_args = cfg?.error_handler_args ?? {}
		retry = cfg?.retry
		errorHandlerSelected = getHandlerType(error_handler_path ?? '')
	}

	async function loadTrigger(defaultConfig?: Partial<HttpTrigger>): Promise<void> {
		if (defaultConfig) {
			loadTriggerConfig(defaultConfig)
			return
		} else {
			const s = await HttpTriggerService.getHttpTrigger({
				workspace: $workspaceStore!,
				path: initialPath
			})

			loadTriggerConfig(s)
		}
	}

	async function triggerScript(): Promise<void> {
		if (customSaveBehavior) {
			customSaveBehavior(routeConfig)
			drawer?.closeDrawer()
		} else {
			deploymentLoading = true
			const saveCfg = routeConfig
			const isSaved = await saveHttpRouteFromCfg(
				initialPath,
				saveCfg,
				edit,
				$workspaceStore!,
				!!$userStore?.is_admin || !!$userStore?.is_super_admin,
				usedTriggerKinds
			)
			if (isSaved) {
				onUpdate(saveCfg.path)
				originalConfig = structuredClone($state.snapshot(getRouteConfig()))
				initialPath = saveCfg.path
				initialScriptPath = saveCfg.script_path
				if (mode !== 'suspended') {
					drawer?.closeDrawer()
				}
			}
			deploymentLoading = false
		}
	}

	function getRouteConfig(): NewHttpTrigger {
		// If the user selects "signature" with the "custom_script" option,
		// we explicitly set the authentication method to "custom_script"
		// (which is a valid enum on its own in the backend)
		const auth_method: AuthenticationMethod =
			authentication_method === 'signature' && signature_options_type === 'custom_script'
				? 'custom_script'
				: authentication_method

		const nCfg = {
			script_path,
			is_flow,
			path,
			route_path,
			http_method,
			request_type,
			workspaced_route,
			mode,
			wrap_body,
			raw_string,
			authentication_resource_path,
			authentication_method: auth_method,
			static_asset_config,
			is_static_website,
			extra_perms: extraPerms,
			summary,
			description: routeDescription,
			error_handler_path,
			error_handler_args,
			retry
		}

		return nCfg
	}

	async function handleToggleMode(newMode: TriggerMode) {
		mode = newMode
		if (!trigger?.draftConfig) {
			await HttpTriggerService.setHttpTriggerMode({
				path: initialPath,
				workspace: $workspaceStore ?? '',
				requestBody: { mode: newMode }
			})
			sendUserToast(`${capitalize(newMode)} HTTP trigger ${initialPath}`)

			onUpdate(initialPath)
		}
		if (originalConfig) {
			originalConfig['mode'] = newMode
		}
	}

	// Update config for captures
	function getCaptureConfig() {
		const newCaptureConfig = {
			route_path: routeConfig.route_path,
			http_method: routeConfig.http_method,
			raw_string: routeConfig.raw_string,
			wrap_body: routeConfig.wrap_body,
			path: routeConfig.path
		}
		//
		return newCaptureConfig
	}

	$effect(() => {
		const args = [captureConfig, isValid] as const
		untrack(() => onCaptureConfigChange?.(...args))
	})

	$effect(() => {
		if (!drawerLoading) {
			handleConfigChange(routeConfig, initialConfig, saveDisabled, edit, onConfigChange)
		}
	})
</script>

{#if static_asset_config}
	<S3FilePicker
		bind:this={s3FilePicker}
		folderOnly={is_static_website}
		bind:selectedFileKey={static_asset_config}
		onClose={() => {
			s3Editor?.setCode(JSON.stringify(static_asset_config, null, 2))
			s3FileUploadRawMode = true
		}}
		readOnlyMode={false}
	/>
{/if}

{#if authentication_method === 'windmill'}
	<UserSettings
		bind:this={userSettings}
		newTokenWorkspace={$workspaceStore}
		newTokenLabel={`http-${$userStore?.username ?? 'superadmin'}-${generateRandomString(4)}`}
		{scopes}
	/>
{/if}

{#if mode === 'suspended'}
	<TriggerSuspendedJobsModal
		bind:this={suspendedJobsModal}
		triggerPath={path}
		triggerKind="http"
		{hasChanged}
		onToggleMode={handleToggleMode}
		runnableConfig={{
			path: script_path,
			kind: itemKind,
			retry,
			errorHandlerPath: error_handler_path,
			errorHandlerArgs: error_handler_args
		}}
	/>
{/if}

{#snippet config()}
	{#if drawerLoading}
		{#if showLoader}
			<Loader2 class="animate-spin" />
		{/if}
	{:else}
		<div class="flex flex-col gap-8">
			{#if mode === 'suspended'}
				<TriggerSuspendedJobsAlert {suspendedJobsModal} />
			{/if}
			<Section label="Metadata">
				<div class="flex flex-col gap-6">
					<Label label="Summary" for="summary">
						<!-- svelte-ignore a11y_autofocus -->
						<input
							autofocus
							type="text"
							placeholder="Short summary to be displayed when listed"
							class="text-sm w-full"
							bind:value={summary}
							disabled={!can_write}
							id="summary"
						/>
					</Label>

					<Label label="Path" for="path">
						<Path
							bind:dirty={dirtyPath}
							bind:error={pathError}
							bind:path
							{initialPath}
							checkInitialPathExistence={!edit}
							namePlaceholder="route"
							kind="http_trigger"
							hideUser
							disableEditing={!can_write}
						/>
					</Label>

					<Label label="Description" for="description">
						<textarea
							rows="4"
							use:autosize
							bind:value={routeDescription}
							placeholder="Describe the route"
							disabled={!can_write}
							id="description"
						></textarea>
					</Label>
				</div>
			</Section>

			{#if !hideTarget}
				<Section label="Target">
					<div class="flex flex-col gap-6">
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
										request_type = 'sync'
										is_static_website = ev.detail === 'static_website'
										if (is_static_website) {
											authentication_method = 'none'
										}
									} else if (ev.detail === 'runnable') {
										static_asset_config = undefined
									}
								}}
							>
								{#snippet children({ item, disabled })}
									<ToggleButton label="Runnable" value="runnable" {item} {disabled} />
									<ToggleButton
										label="Static asset"
										value="static_asset"
										{item}
										disabled={disabled || mode === 'suspended'}
									/>
									<ToggleButton
										label="Static website"
										value="static_website"
										{item}
										disabled={disabled || mode === 'suspended'}
									/>
								{/snippet}
							</ToggleButtonGroup>
						{/if}

						{#if static_asset_config}
							<div class="flex flex-col gap-1">
								{#if is_static_website}
									<p class="text-xs text-primary">
										Upload or specify a <b class="font-semibold text-emphasis">folder</b> on S3. All
										its files will be served under the path above. Use this full path as the base URL
										of your website to ensure relative imports work correctly.
									</p>
								{/if}
								<div class="flex flex-col gap-4">
									<div class="flex flex-col w-full gap-1">
										{#if can_write}
											<Toggle
												class="flex justify-end"
												bind:checked={s3FileUploadRawMode}
												size="xs"
												options={{ left: 'Raw S3 object input' }}
												disabled={!can_write}
											/>
										{/if}
										{#if s3FileUploadRawMode}
											{#if can_write}
												<JsonEditor
													bind:editor={s3Editor}
													code={JSON.stringify(static_asset_config ?? { s3: '' }, null, 2)}
													bind:value={static_asset_config}
												/>
											{:else}
												<Highlight
													language={json}
													code={JSON.stringify(static_asset_config ?? { s3: '' }, null, 2)}
												/>
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
													}}
													on:deletion={(evt) => {
														static_asset_config = {
															s3: ''
														}
													}}
												/>
											{/key}
										{/if}
										{#if can_write}
											<Button
												variant="default"
												size="xs"
												on:click={() => {
													s3FilePicker?.open?.(!is_static_website ? static_asset_config : undefined)
												}}
												startIcon={{ icon: Pipette }}
											>
												Choose an object from the catalog
											</Button>
										{/if}
									</div>
								</div>
							</div>
						{:else}
							<div class="flex flex-col gap-1">
								<p class="text-xs text-primary font-normal">
									Pick a script or flow to be triggered<Required required={true} />
								</p>

								<div class="flex flex-row items-center gap-2">
									<ScriptPicker
										disabled={fixedScriptPath != '' || !can_write}
										initialPath={fixedScriptPath || initialScriptPath}
										kinds={['script']}
										allowFlow={true}
										bind:itemKind
										bind:scriptPath={script_path}
										allowRefresh={can_write}
										allowEdit={!$userStore?.operator}
										clearable
									/>

									{#if emptyString(script_path)}
										<Button
											variant="default"
											size="lg"
											href={itemKind === 'flow'
												? '/flows/add?hub=62'
												: '/scripts/add?hub=hub%2F19669'}
											target="_blank">Create from template</Button
										>
									{/if}
								</div>
								<p class="text-2xs text-secondary font-normal">
									To handle headers, query or path parameters, add a preprocessor to your runnable.
								</p>
							</div>
						{/if}
					</div>
				</Section>
			{/if}

			<RouteEditorConfigSection
				initialTriggerPath={initialPath}
				bind:route_path
				bind:isValid
				bind:dirtyRoutePath
				bind:http_method
				bind:workspaced_route
				{can_write}
				bind:static_asset_config
				showTestingBadge={isEditor}
				isDraftOnly={trigger ? trigger.isDraft : false}
			/>

			{#if !is_static_website}
				<Section label="Advanced" collapsable>
					<div class="min-h-96">
						<Tabs bind:selected={optionTabSelected}>
							<Tab value="request_options" label="Request Options" />
							<Tab value="error_handler" label="Error Handler" />
							<Tab value="retries" label="Retries" />
						</Tabs>
						<div class="mt-4">
							{#if optionTabSelected === 'request_options'}
								<div class="flex flex-col gap-4">
									{#if !static_asset_config}
										<div class="flex flex-row justify-between">
											<Label label="Request type" class="w-full">
												{#snippet action()}
													<ToggleButtonGroup
														class="w-auto h-full"
														selected={request_type}
														on:selected={({ detail }) => {
															request_type = detail
														}}
														disabled={!can_write || !!static_asset_config}
													>
														{#snippet children({ item, disabled })}
															<ToggleButton
																label="Sync"
																value="sync"
																tooltip="Triggers the execution, wait for the job to complete and return it as a response."
																{item}
																{disabled}
															/>
															<ToggleButton
																label="Async"
																value="async"
																tooltip="The returning value is the uuid of the job assigned to execute the job."
																{item}
																{disabled}
															/>
															<ToggleButton
																label="Sync SSE"
																value="sync_sse"
																tooltip="Triggers the execution and returns an SSE stream."
																{item}
																{disabled}
															/>
														{/snippet}
													</ToggleButtonGroup>
												{/snippet}
											</Label>
										</div>
									{/if}
									<Label label="Authentication" class="w-full">
										{#snippet action()}
											<ToggleButtonGroup
												class="w-auto h-full"
												bind:selected={authentication_method}
												on:selected={(e) => {
													if (
														e.detail === 'signature' &&
														signature_options_type === 'custom_script'
													) {
														raw_string = true
													}
												}}
												disabled={!can_write}
											>
												{#snippet children({ item, disabled })}
													{#each authentication_options as option}
														{#if option.value === 'signature'}
															<Popover placement="top-end" usePointerDownOutside>
																{#snippet trigger()}
																	<ToggleButton
																		label={option.label}
																		value={option.value}
																		tooltip={option.tooltip}
																		{item}
																		{disabled}
																	/>
																{/snippet}
																{#snippet content()}
																	<ToggleButtonGroup
																		class="w-auto h-full"
																		bind:selected={signature_options_type}
																		on:selected={(e) => {
																			if (e.detail === 'custom_script') {
																				if (!raw_string) {
																					raw_string = true
																				}
																			}
																		}}
																		disabled={!can_write}
																	>
																		{#snippet children({ item, disabled })}
																			<ToggleButton
																				label="Signature validation"
																				value="custom_signature"
																				tooltip="Use a predefined or custom signature-based authentication scheme"
																				{item}
																				{disabled}
																			/>
																			<ToggleButton
																				label="Custom script"
																				value="custom_script"
																				tooltip="Use your own script logic"
																				{item}
																				{disabled}
																			/>
																		{/snippet}
																	</ToggleButtonGroup>
																{/snippet}
															</Popover>
														{:else}
															<ToggleButton
																label={option.label}
																value={option.value}
																tooltip={option.tooltip}
																{item}
																{disabled}
															/>
														{/if}
													{/each}
												{/snippet}
											</ToggleButtonGroup>
										{/snippet}
									</Label>

									{#each authentication_options as option}
										{#if option.resource_type && authentication_method === option.value}
											<ResourcePicker
												bind:value={authentication_resource_path}
												resourceType={option.resource_type}
												disabled={!can_write}
											/>
										{/if}
									{/each}

									{#if authentication_method === 'signature'}
										{#if signature_options_type === 'custom_signature'}
											<ResourcePicker
												bind:value={authentication_resource_path}
												resourceType={'signature_auth'}
												disabled={!can_write}
											/>
										{:else if signature_options_type === 'custom_script'}
											<div class="flex flex-col gap-1">
												<div class="flex flex-row gap-2">
													<div class="flex flex-row gap-2 w-full">
														<input
															type="text"
															autocomplete="off"
															bind:value={variable_path}
															readonly
															disabled={true}
														/>
														<Button
															title="Add variable"
															variant="accent"
															on:click={() => variablePicker?.openDrawer()}
															size="xs"
															disabled={!can_write}
														>
															Pick variable
														</Button>
													</div>
													<Button
														disabled={emptyString(variable_path) || !can_write}
														variant="default"
														size="xs"
														href={itemKind === 'flow'
															? `/flows/add?${SECRET_KEY_PATH}=${encodeURIComponent(variable_path)}&hub=${
																	HubFlow.SIGNATURE_TEMPLATE
																}`
															: `/scripts/add?${SECRET_KEY_PATH}=${encodeURIComponent(
																	variable_path
																)}&hub=hub%2F${HUB_SCRIPT_ID}`}
														target="_blank">Create from template</Button
													>
												</div>
												<p class="text-2xs text-secondary">
													Pick a secret variable or create one which will be used as a secret key
													for your custom script/flow<Required required={true} /><br />
												</p>
											</div>
										{/if}
									{:else if authentication_method === 'windmill'}
										<Button size="xs" variant="default" on:click={() => userSettings?.openDrawer()}>
											Create a route-specific token
											<Tooltip light>
												The token will have a scope such that it can only be used to read and
												trigger this route. It is safe to share as it cannot be used to impersonate
												you.
											</Tooltip>
										</Button>
									{/if}

									<RouteBodyTransformerOption
										bind:raw_string
										bind:wrap_body
										disabled={!can_write}
										{testingBadge}
									/>
								</div>
							{:else}
								<TriggerRetriesAndErrorHandler
									{optionTabSelected}
									{itemKind}
									{can_write}
									bind:errorHandlerSelected
									bind:error_handler_path
									bind:error_handler_args
									bind:retry
								/>
							{/if}
						</div>
					</div>
				</Section>
			{/if}
		</div>
	{/if}
{/snippet}

{#snippet testingBadge()}
	{#if isEditor}
		<TestingBadge />
	{/if}
{/snippet}

{#snippet saveButton()}
	{#if !drawerLoading}
		<TriggerEditorToolbar
			{trigger}
			permissions={drawerLoading || !can_write ? 'none' : can_write && isAdmin ? 'create' : 'write'}
			{saveDisabled}
			{allowDraft}
			{edit}
			isLoading={deploymentLoading}
			onUpdate={triggerScript}
			{onReset}
			{onDelete}
			{isDeployed}
			{mode}
			onToggleMode={handleToggleMode}
			{suspendedJobsModal}
		/>
	{/if}
{/snippet}

{#if useDrawer}
	<Drawer size="700px" bind:this={drawer}>
		<DrawerContent
			title={edit
				? can_write
					? `Edit route ${initialPath}`
					: `Route ${initialPath}`
				: 'New route'}
			on:close={() => drawer?.closeDrawer()}
		>
			{#snippet actions()}
				{@render saveButton()}
			{/snippet}
			{@render config()}
		</DrawerContent>
	</Drawer>
{:else}
	<Section label={!customLabel ? 'HTTP Route' : ''} headerClass="grow min-w-0 h-[30px]">
		{#snippet header()}
			{#if customLabel}
				{@render customLabel()}
			{/if}
		{/snippet}
		{#snippet action()}
			{@render saveButton()}
		{/snippet}
		{#if description}
			{@render description()}
		{/if}
		{@render config()}
	</Section>
{/if}

<ItemPicker
	bind:this={variablePicker}
	pickCallback={(path, _) => {
		variable_path = path
	}}
	tooltip="Variables are dynamic values that have a key associated to them and can be retrieved during the execution of a Script or Flow."
	documentationLink="https://www.windmill.dev/docs/core_concepts/variables_and_secrets"
	itemName="Variable"
	extraField="path"
	loadItems={loadVariables}
	buttons={{ 'Edit/View': (x) => variableEditor?.editVariable(x) }}
>
	{#snippet submission()}
		<div class="flex flex-row">
			<Button
				variant="default"
				size="sm"
				startIcon={{ icon: Plus }}
				on:click={() => {
					variableEditor?.initNew()
				}}
			>
				New variable
			</Button>
		</div>
	{/snippet}
</ItemPicker>

<VariableEditor bind:this={variableEditor} on:create={(e) => variablePicker?.openDrawer()} />
