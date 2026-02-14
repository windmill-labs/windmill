<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import {
		FlowService,
		ScriptService,
		WebsocketTriggerService,
		type Flow,
		type Script,
		type ScriptArgs,
		type WebsocketTriggerInitialMessage,
		type Retry,
		type ErrorHandler,
		type TriggerMode
	} from '$lib/gen'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptySchema, emptyString, sendUserToast } from '$lib/utils'
	import Section from '$lib/components/Section.svelte'
	import { Loader2, X, Plus } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import { fade } from 'svelte/transition'
	import type { Schema } from '$lib/common'
	import JsonEditor from '$lib/components/JsonEditor.svelte'
	import TriggerFilters from '../TriggerFilters.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import WebsocketEditorConfigSection from './WebsocketEditorConfigSection.svelte'
	import { untrack, type Snippet } from 'svelte'

	import TriggerEditorToolbar from '../TriggerEditorToolbar.svelte'
	import { saveWebsocketTriggerFromCfg } from './utils'
	import { getHandlerType, handleConfigChange, type Trigger } from '../utils'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import TriggerRetriesAndErrorHandler from '../TriggerRetriesAndErrorHandler.svelte'
	import { deepEqual } from 'fast-equals'
	import TriggerSuspendedJobsAlert from '../TriggerSuspendedJobsAlert.svelte'
	import TriggerSuspendedJobsModal from '../TriggerSuspendedJobsModal.svelte'
	import { capitalize } from '$lib/utils'

	interface Props {
		useDrawer?: boolean
		description?: Snippet | undefined
		hideTarget?: boolean
		hideTooltips?: boolean
		useEditButton?: boolean
		isEditor?: boolean
		allowDraft?: boolean
		trigger?: Trigger
		isDeployed?: boolean
		cloudDisabled?: boolean
		customLabel?: Snippet
		onConfigChange?: (cfg: Record<string, any>, saveDisabled: boolean, updated: boolean) => void
		onCaptureConfigChange?: (cfg: Record<string, any>, isValid: boolean) => void
		onUpdate?: (path?: string) => void
		onDelete?: () => void
		onReset?: () => void
	}

	let {
		useDrawer = true,
		description = undefined,
		hideTarget = false,
		hideTooltips = false,
		isEditor = false,
		allowDraft = false,
		trigger = undefined,
		isDeployed = false,
		customLabel = undefined,
		onConfigChange = undefined,
		onCaptureConfigChange = undefined,
		onUpdate = undefined,
		onDelete = undefined,
		onReset = undefined,
		cloudDisabled = false
	}: Props = $props()

	let drawer: Drawer | undefined = $state()
	let is_flow: boolean = $state(false)
	let initialPath = $state('')
	let edit = $state(true)
	let itemKind: 'flow' | 'script' = $state('script')
	let script_path = $state('')
	let initialScriptPath = $state('')
	let fixedScriptPath = $state('')
	let path: string = $state('')
	let pathError = $state('')
	let url = $state('')
	let dirtyUrl = $state(false)
	let filters: {
		key: string
		value: any
	}[] = $state([])
	let initial_messages: WebsocketTriggerInitialMessage[] = $state([])
	let url_runnable_args: Record<string, any> | undefined = $state({})
	let can_return_message = $state(false)
	let can_return_error_result = $state(false)
	let dirtyPath = $state(false)
	let can_write = $state(true)
	let drawerLoading = $state(true)
	let showLoading = $state(false)
	let initialConfig: Record<string, any> | undefined = undefined
	let deploymentLoading = $state(false)
	let isValid = $state(false)
	let optionTabSelected: 'error_handler' | 'retries' = $state('error_handler')
	let errorHandlerSelected: ErrorHandler = $state('slack')
	let error_handler_path: string | undefined = $state()
	let error_handler_args: Record<string, any> = $state({})
	let retry: Retry | undefined = $state()
	let mode = $state<TriggerMode>('enabled')

	let suspendedJobsModal = $state<TriggerSuspendedJobsModal | null>(null)
	let originalConfig = $state<Record<string, any> | undefined>(undefined)

	let hasChanged = $derived(!deepEqual(getSaveCfg(), originalConfig ?? {}))
	const websocketCfg = $derived.by(getSaveCfg)
	const captureConfig = $derived.by(isEditor ? getCaptureConfig : () => ({}))
	const saveDisabled = $derived.by(() => {
		const invalidInitialMessages = initial_messages.some((v) => {
			if ('runnable_result' in v) {
				return !v.runnable_result.path
			}
			return false
		})
		return (
			pathError !== '' ||
			!isValid ||
			invalidInitialMessages ||
			drawerLoading ||
			!can_write ||
			emptyString(script_path) ||
			!hasChanged
		)
	})

	$effect(() => {
		is_flow = itemKind === 'flow'
	})

	$effect(() => {
		if (!can_return_message) {
			can_return_error_result = false
		}
	})

	export async function openEdit(
		ePath: string,
		isFlow: boolean,
		defaultConfig?: Record<string, any>
	) {
		let loadingTimeout = setTimeout(() => {
			showLoading = true
		}, 100) // Do not show loading spinner for the first 100ms
		drawerLoading = true
		try {
			drawer?.openDrawer()
			initialPath = ePath
			itemKind = isFlow ? 'flow' : 'script'
			edit = true
			dirtyPath = false
			dirtyUrl = false
			await loadTrigger(defaultConfig)
			originalConfig = structuredClone($state.snapshot(getSaveCfg()))
		} catch (err) {
			sendUserToast(`Could not load websocket trigger: ${err}`, true)
		} finally {
			if (!defaultConfig) {
				initialConfig = structuredClone($state.snapshot(getSaveCfg()))
			}
			clearTimeout(loadingTimeout)
			drawerLoading = false
			showLoading = false
		}
	}

	export async function openNew(
		nis_flow: boolean,
		fixedScriptPath_?: string,
		defaultValues?: Record<string, any>
	) {
		let loadingTimeout = setTimeout(() => {
			showLoading = true
		}, 100) // Do not show loading spinner for the first 100ms
		drawerLoading = true
		try {
			drawer?.openDrawer()
			is_flow = nis_flow
			edit = false
			itemKind = nis_flow ? 'flow' : 'script'
			url = defaultValues?.url ?? ''
			dirtyUrl = false
			initialScriptPath = ''
			fixedScriptPath = fixedScriptPath_ ?? ''
			script_path = fixedScriptPath
			path = defaultValues?.path ?? ''
			initialPath = ''
			filters = []
			initial_messages = []
			url_runnable_args = defaultValues?.url_runnable_args ?? {}
			dirtyPath = false
			can_return_message = false
			can_return_error_result = false
			error_handler_path = defaultValues?.error_handler_path ?? undefined
			error_handler_args = defaultValues?.error_handler_args ?? {}
			retry = defaultValues?.retry ?? undefined
			errorHandlerSelected = getHandlerType(error_handler_path ?? '')
			mode = defaultValues?.mode ?? 'enabled'
			originalConfig = undefined
		} finally {
			clearTimeout(loadingTimeout)
			drawerLoading = false
			showLoading = false
		}
	}

	function loadTriggerConfig(cfg?: Record<string, any>): void {
		script_path = cfg?.script_path
		initialScriptPath = cfg?.script_path
		is_flow = cfg?.is_flow
		path = cfg?.path
		url = cfg?.url
		filters = cfg?.filters
		initial_messages = cfg?.initial_messages ?? []
		url_runnable_args = cfg?.url_runnable_args
		can_return_message = cfg?.can_return_message
		can_return_error_result = cfg?.can_return_error_result
		can_write = canWrite(path, cfg?.extra_perms, $userStore)
		error_handler_path = cfg?.error_handler_path
		error_handler_args = cfg?.error_handler_args ?? {}
		retry = cfg?.retry
		errorHandlerSelected = getHandlerType(error_handler_path ?? '')
		mode = cfg?.mode ?? 'enabled'
	}

	function getSaveCfg() {
		return {
			script_path,
			is_flow,
			path,
			url,
			filters,
			initial_messages,
			url_runnable_args,
			can_return_message,
			can_return_error_result,
			error_handler_path,
			error_handler_args,
			retry,
			mode
		}
	}

	async function loadTrigger(defaultConfig?: Record<string, any>): Promise<void> {
		if (defaultConfig) {
			loadTriggerConfig(defaultConfig)
			return
		} else {
			const s = await WebsocketTriggerService.getWebsocketTrigger({
				workspace: $workspaceStore!,
				path: initialPath
			})
			loadTriggerConfig(s)
		}
	}

	let initialMessageRunnableSchemas: Record<string, Schema> = $state({})
	async function loadInitialMessageRunnableSchemas(
		initialMessageRunnables: {
			path: string
			is_flow: boolean
		}[]
	) {
		for (const { path, is_flow } of initialMessageRunnables) {
			if (!path) {
				continue
			}
			try {
				let schema: Schema | undefined = emptySchema()
				let scriptOrFlow: Script | Flow = is_flow
					? await FlowService.getFlowByPath({ workspace: $workspaceStore!, path })
					: await ScriptService.getScriptByPath({ workspace: $workspaceStore!, path })
				schema = scriptOrFlow.schema as Schema
				if (schema && schema.properties) {
					initialMessageRunnableSchemas[(is_flow ? 'flow/' : '') + path] = schema
				}
			} catch (err) {
				sendUserToast(
					`Could not query runnable schema for ${is_flow ? 'flow' : 'script'} ${path}: ${err}`,
					true
				)
			}
		}
	}
	let initialMessageRunnables = $derived(
		initial_messages
			.map((v) => ('runnable_result' in v ? v.runnable_result : undefined))
			.filter((v): v is { path: string; is_flow: boolean; args: ScriptArgs } => !!v)
	)
	$effect(() => {
		;[initialMessageRunnables]
		untrack(() => loadInitialMessageRunnableSchemas(initialMessageRunnables))
	})

	async function updateTrigger(): Promise<void> {
		deploymentLoading = true
		const saveCfg = getSaveCfg()
		const isSaved = await saveWebsocketTriggerFromCfg(
			initialPath,
			saveCfg,
			edit,
			$workspaceStore!,
			usedTriggerKinds
		)
		if (isSaved) {
			onUpdate?.(saveCfg.path)
			originalConfig = structuredClone($state.snapshot(getSaveCfg()))
			initialPath = saveCfg.path
			initialScriptPath = saveCfg.script_path
			if (mode !== 'suspended') {
				drawer?.closeDrawer()
			}
		}
		deploymentLoading = false
	}

	function getCaptureConfig() {
		return {
			url,
			url_runnable_args,
			path
		}
	}

	async function handleToggleMode(newMode: TriggerMode) {
		mode = newMode
		if (!trigger?.draftConfig) {
			await WebsocketTriggerService.setWebsocketTriggerMode({
				path: initialPath,
				workspace: $workspaceStore ?? '',
				requestBody: { mode: newMode }
			})
			sendUserToast(`${capitalize(newMode)} websocket trigger ${initialPath}`)

			onUpdate?.(initialPath)
		}
		if (originalConfig) {
			originalConfig['mode'] = newMode
		}
	}

	$effect(() => {
		const args = [captureConfig, isValid] as const
		untrack(() => onCaptureConfigChange?.(...args))
	})

	$effect(() => {
		if (!drawerLoading) {
			handleConfigChange(websocketCfg, initialConfig, saveDisabled, edit, onConfigChange)
		}
	})
</script>

{#if mode === 'suspended'}
	<TriggerSuspendedJobsModal
		bind:this={suspendedJobsModal}
		triggerPath={path}
		triggerKind="websocket"
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

{#if useDrawer}
	<Drawer size="800px" bind:this={drawer}>
		<DrawerContent
			title={edit
				? can_write
					? `Edit WebSocket trigger ${initialPath}`
					: `WebSocket trigger ${initialPath}`
				: 'New WebSocket trigger'}
			on:close={drawer.closeDrawer}
		>
			{#snippet actions()}
				{@render actionsButtons()}
			{/snippet}
			{@render config()}
		</DrawerContent>
	</Drawer>
{:else}
	<Section label={!customLabel ? 'WebSocket trigger' : ''} headerClass="grow min-w-0 h-[30px]">
		{#snippet header()}
			{#if customLabel}
				{@render customLabel()}
			{/if}
		{/snippet}
		{#snippet action()}
			{@render actionsButtons()}
		{/snippet}
		{@render config()}
	</Section>
{/if}

{#snippet actionsButtons()}
	{#if !drawerLoading}
		<TriggerEditorToolbar
			{trigger}
			permissions={!drawerLoading && can_write ? 'create' : 'none'}
			{allowDraft}
			{edit}
			isLoading={deploymentLoading}
			{saveDisabled}
			{isDeployed}
			onUpdate={updateTrigger}
			{onDelete}
			{onReset}
			{mode}
			onToggleMode={handleToggleMode}
			{suspendedJobsModal}
			{cloudDisabled}
		/>
	{/if}
{/snippet}

{#snippet config()}
	{#if drawerLoading}
		{#if showLoading}
			<Loader2 class="animate-spin" />
		{/if}
	{:else}
		<div class="flex flex-col gap-4">
			{#if mode === 'suspended'}
				<TriggerSuspendedJobsAlert {suspendedJobsModal} />
			{/if}
			{#if description}
				{@render description()}
			{/if}
			{#if !hideTooltips}
				<Alert title="Info" type="info" size="xs">
					{#if edit}
						Changes can take up to 30 seconds to take effect.
					{:else}
						New WebSocket triggers can take up to 30 seconds to start listening.
					{/if}
				</Alert>
			{/if}
		</div>
		<div class="flex flex-col gap-6 mt-6">
			<div class="flex flex-col gap-4">
				<Label label="Path">
					<Path
						bind:dirty={dirtyPath}
						bind:error={pathError}
						bind:path
						{initialPath}
						checkInitialPathExistence={!edit}
						namePlaceholder="ws_trigger"
						kind="websocket_trigger"
						disabled={!can_write}
						disableEditing={!can_write}
					/>
				</Label>
			</div>

			<Section label={hideTarget ? 'Runnable options' : 'Runnable'} class="flex flex-col gap-4">
				{#if !hideTarget}
					<div>
						<p class="text-xs mb-1 text-primary">
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
								allowRefresh={can_write}
								allowEdit={!$userStore?.operator}
								clearable
							/>
							{#if emptyString(script_path)}
								<Button
									btnClasses="ml-4"
									variant="accent"
									size="xs"
									disabled={!can_write}
									href={itemKind === 'flow' ? '/flows/add?hub=64' : '/scripts/add?hub=hub%2F19660'}
									target="_blank"
								>
									Create from template
								</Button>
							{/if}
						</div>
					</div>
				{/if}

				<Toggle
					checked={can_return_message}
					on:change={() => {
						can_return_message = !can_return_message
					}}
					options={{
						right: 'Send runnable result',
						rightTooltip:
							'Whether the runnable result should be sent as a message to the websocket server when not null.'
					}}
					disabled={!can_write}
					textClass="font-semibold"
				/>

				<Toggle
					checked={can_return_error_result}
					on:change={() => {
						can_return_error_result = !can_return_error_result
					}}
					options={{
						right: 'Send result on error',
						rightTooltip:
							'Allows the runnable result to be sent as a message to the WebSocket server if the result is a non-null error.'
					}}
					disabled={!can_write || !can_return_message}
					textClass="font-semibold"
				/>
			</Section>

			<WebsocketEditorConfigSection
				bind:url
				bind:url_runnable_args
				{dirtyUrl}
				{can_write}
				bind:isValid
				showTestingBadge={isEditor}
			/>

			<Section label="Initial messages">
				<p class="text-xs mb-1 text-primary">
					Initial messages are sent at the beginning of the connection. They are sent in order.<br
					/>
					Raw messages and runnable results are supported.
				</p>
				<div class="flex flex-col gap-4 mt-1">
					{#each initial_messages as v, i}
						<div class="flex w-full gap-2 items-center">
							<div class="w-full flex flex-col gap-2 border p-2 rounded-md">
								<div class="flex flex-row gap-2 w-full">
									<label class="flex flex-col w-full">
										<div class="text-secondary text-sm mb-2">Type</div>
										<select
											class="w-20"
											onchange={(e) => {
												if (e.target?.['value'] === 'raw_message') {
													initial_messages[i] = {
														raw_message: '""'
													}
												} else {
													initial_messages[i] = {
														runnable_result: {
															path: '',
															args: {},
															is_flow: false
														}
													}
												}
											}}
											value={'runnable_result' in v ? 'runnable_result' : 'raw_message'}
											disabled={!can_write}
										>
											<option value="raw_message">Raw message</option>
											<option value="runnable_result">Runnable result</option>
										</select>
									</label>
								</div>
								{#if 'raw_message' in v}
									<div class="flex flex-col w-full">
										<div class="text-secondary text-sm mb-2">
											Raw JSON message (if a string, wrapping quotes will be discarded)
										</div>
										<JsonEditor
											on:change={(ev) => {
												const { code } = ev.detail
												initial_messages[i] = {
													raw_message: code
												}
											}}
											code={v.raw_message}
											disabled={!can_write}
										/>
									</div>
								{:else if 'runnable_result' in v}
									<div class="flex flex-col w-full">
										<div class="text-secondary text-sm">Runnable</div>
										<ScriptPicker
											allowFlow={true}
											itemKind={v.runnable_result?.is_flow ? 'flow' : 'script'}
											initialPath={v.runnable_result?.path ?? ''}
											on:select={(ev) => {
												const { path, itemKind } = ev.detail
												initial_messages[i] = {
													runnable_result: {
														path: path ?? '',
														args: {},
														is_flow: itemKind === 'flow'
													}
												}
											}}
											disabled={!can_write}
										/>

										{#if v.runnable_result?.path}
											{@const schema =
												initialMessageRunnableSchemas[
													v.runnable_result.is_flow
														? 'flow/' + v.runnable_result.path
														: v.runnable_result.path
												]}
											{#if schema}
												<p class="font-semibold text-sm mt-4 mb-2">Arguments</p>
												{#await import('$lib/components/SchemaForm.svelte')}
													<Loader2 class="animate-spin mt-2" />
												{:then Module}
													<Module.default
														{schema}
														bind:args={v.runnable_result.args}
														shouldHideNoInputs
														className="text-xs"
														disabled={!can_write}
													/>
												{/await}
												{#if schema && schema.properties && Object.keys(schema.properties).length === 0}
													<div class="text-xs text-secondary">This runnable takes no arguments</div>
												{/if}
											{:else}
												<Loader2 class="animate-spin mt-2" />
											{/if}
										{/if}
									</div>
								{:else}
									Unknown type
								{/if}
							</div>
							<button
								transition:fade|local={{ duration: 100 }}
								class="rounded-full p-1 bg-surface-secondary duration-200 hover:bg-surface-hover"
								aria-label="Clear"
								onclick={() => {
									initial_messages = initial_messages.filter((_, index) => index !== i)
								}}
								disabled={!can_write}
							>
								<X size={14} />
							</button>
						</div>
					{/each}

					<div class="flex items-baseline">
						<Button
							variant="default"
							size="xs"
							btnClasses="mt-1"
							on:click={() => {
								if (initial_messages == undefined || !Array.isArray(initial_messages)) {
									initial_messages = []
								}
								initial_messages = initial_messages.concat({
									raw_message: '""'
								})
							}}
							disabled={!can_write}
							startIcon={{ icon: Plus }}
						>
							Add item
						</Button>
					</div>
				</div>
			</Section>

			<TriggerFilters bind:filters disabled={!can_write} />

			<Section label="Advanced" collapsable>
				<div class="flex flex-col gap-4">
					<div class="min-h-96">
						<Tabs bind:selected={optionTabSelected}>
							<Tab value="error_handler" label="Error Handler" />
							<Tab value="retries" label="Retries" />
						</Tabs>
						<div class="mt-4">
							<TriggerRetriesAndErrorHandler
								{optionTabSelected}
								{itemKind}
								{can_write}
								bind:errorHandlerSelected
								bind:error_handler_path
								bind:error_handler_args
								bind:retry
							/>
						</div>
					</div>
				</div>
			</Section>
		</div>
	{/if}
{/snippet}
