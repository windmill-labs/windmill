<script lang="ts">
	import EmptyInlineScript from '../apps/editor/inlineScriptsPanel/EmptyInlineScript.svelte'
	import InlineScriptRunnableByPath from '../apps/editor/inlineScriptsPanel/InlineScriptRunnableByPath.svelte'
	import {
		isRunnableByName,
		isRunnableByPath,
		type InlineScript,
		type RunnableWithFields,
		type StaticAppInput,
		type UserAppInput,
		type CtxAppInput
	} from '../apps/inputType'
	import { createEventDispatcher } from 'svelte'
	import RawAppInlineScriptEditor from './RawAppInlineScriptEditor.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import Tabs from '../common/tabs/Tabs.svelte'
	import { Tab } from '../common'
	import { SQL_LANGUAGES } from '$lib/infer'
	import RawAppInputsSpecEditor from './RawAppInputsSpecEditor.svelte'
	import SplitPanesWrapper from '../splitPanes/SplitPanesWrapper.svelte'
	import SchemaForm from '../SchemaForm.svelte'
	import RunnableJobPanelInner from '../apps/editor/RunnableJobPanelInner.svelte'
	import JobLoader from '../JobLoader.svelte'
	import { FlowService, JobService, type Job, type OpenFlow, type ScriptLang } from '$lib/gen'
	import { getScriptByPath } from '$lib/scripts'
	import { slide } from 'svelte/transition'
	import { DebugToolbar, DebugPanel, debugState } from '$lib/components/debug'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import RunButton from '$lib/components/RunButton.svelte'
	import { isHubFlowPath } from '$lib/utils'
	import { sendUserToast } from '$lib/toast'
	import { rawAppPreviewModules } from './utils'
	import {
		useOperatingUser,
		useOperatingWorkspace
	} from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspace = useOperatingWorkspace()
	const operatingUser = useOperatingUser()
	const actingUser = $derived(operatingUser.current)
	let opWs = $derived($operatingWorkspace)

	type RunnableWithInlineScript = RunnableWithFields & {
		inlineScript?: InlineScript & { language: ScriptLang }
		delete_after_secs?: number
		viewer_data_access?: boolean
	}
	export type Runnable = RunnableWithInlineScript | undefined
	interface Props {
		runnable: Runnable
		id: string
		appPath: string
		/** All of the app's runnables, which a test run ships as modules for `./<id>` imports */
		runnables?: Record<string, Runnable>
		/** Identity the test runs as instead of the user (see `appPreviewOnBehalfOf`). */
		previewOnBehalfOf?: string
		lastDeployedCode?: string | undefined
		/** Called when code is selected in the editor */
		onSelectionChange?: (
			selection: {
				content: string
				startLine: number
				endLine: number
				startColumn: number
				endColumn: number
			} | null
		) => void
	}

	let {
		runnable = $bindable(),
		id,
		appPath,
		runnables,
		previewOnBehalfOf,
		onSelectionChange
	}: Props = $props()

	const dispatch = createEventDispatcher()

	async function fork(nrunnable: Runnable) {
		runnable = nrunnable == undefined ? undefined : { ...runnable, ...nrunnable }
	}

	function onPick(o: { runnable: Runnable; fields: Record<string, StaticAppInput> }) {
		runnable =
			o.runnable == undefined
				? undefined
				: {
						...(runnable ?? {}),
						...o.runnable,
						fields: o.fields
					}
	}

	// A SQL argument named after a viewer property is most often a per-viewer filter,
	// which only a ctx field protects. `wmill app lint` warns on the same names.
	const VIEWER_IDENTITY_ARGS = ['username', 'email', 'groups']
	let isSql = $derived(
		isRunnableByName(runnable) && SQL_LANGUAGES.includes(runnable.inlineScript?.language ?? '')
	)

	let selectedTab = $state('test')
	let args = $state({})
	let hubFlowPreview: OpenFlow | undefined = $state(undefined)

	function getSchema(runnable: RunnableWithFields) {
		if (isRunnableByPath(runnable)) {
			return runnable.schema
		} else if (isRunnableByName(runnable) && runnable.inlineScript) {
			return runnable.inlineScript.schema
		}
		return {}
	}

	let jobLoader: JobLoader | undefined = $state()
	let testJob: Job | undefined = $state()
	let testIsLoading = $state(false)
	let scriptProgress = $state(0)

	// Reference to the inline script editor for debug functions
	let inlineScriptEditor: RawAppInlineScriptEditor | undefined = $state()

	// Get debug state from the editor
	const editorDebugState = $derived(
		inlineScriptEditor?.getDebugState?.() ?? {
			debugMode: false,
			isDebuggableScript: false,
			showDebugPanel: false,
			hasDebugResult: false,
			dapClient: null,
			selectedDebugFrameId: null,
			debugSessionJobId: null,
			debugBreakpoints: new Set()
		}
	)

	// Reactive debug state values
	const debugMode = $derived(editorDebugState.debugMode)
	const isDebuggableScript = $derived(editorDebugState.isDebuggableScript)
	const showDebugPanel = $derived(editorDebugState.showDebugPanel)
	const hasDebugResult = $derived(editorDebugState.hasDebugResult)
	const dapClient = $derived(editorDebugState.dapClient)
	let selectedDebugFrameId: number | null = $state(null)

	// Auto-switch to test tab when debug mode is enabled
	$effect(() => {
		if (debugMode) {
			selectedTab = 'test'
		}
	})

	// Helper to get actual ctx value for testing
	function getCtxValue(expr: string): any {
		switch (expr) {
			case 'username':
				return actingUser?.username ?? ''
			case 'email':
				return actingUser?.email ?? ''
			case 'groups':
				return actingUser?.groups ?? []
			case 'workspace':
				return opWs ?? ''
			case 'author':
				return actingUser?.email ?? '' // In editor, author is the current user
			default:
				return ''
		}
	}

	function onFieldsChange(fields: Record<string, StaticAppInput | UserAppInput | CtxAppInput>) {
		if (args == undefined) {
			args = {}
		}
		Object.entries(fields ?? {}).forEach(([k, v]) => {
			if (v.type == 'static') {
				args[k] = v.value
			} else if (v.type == 'ctx' && v.ctx) {
				// For test preview, use actual user values
				args[k] = getCtxValue(v.ctx)
			}
		})
	}

	async function testPreview() {
		selectedTab = 'test'
		if (isRunnableByName(runnable)) {
			await jobLoader?.runPreview(
				appPath + '/' + id,
				runnable.inlineScript?.content ?? '',
				runnable.inlineScript?.language,
				args,
				undefined,
				undefined,
				undefined,
				undefined,
				undefined,
				runnables ? rawAppPreviewModules(runnables, id) : undefined,
				undefined,
				undefined,
				{ onBehalfOf: previewOnBehalfOf, viewerDataAccess: runnable.viewer_data_access }
			)
		} else if (isRunnableByPath(runnable)) {
			if (jobLoader && isRunnableByPath(runnable)) {
				if (runnable.runType == 'flow') {
					if (isHubFlowPath(runnable.path)) {
						if (!hubFlowPreview) {
							sendUserToast('Hub flow preview is still loading', true)
							return
						}
						await jobLoader.runFlowPreview(
							args,
							hubFlowPreview,
							undefined,
							runnable.path,
							previewOnBehalfOf
						)
					} else if (previewOnBehalfOf) {
						// The deployed app runs a path runnable as the app's identity, which a run
						// by path cannot ask for: preview the deployed definition instead.
						const flow = await FlowService.getFlowByPath({ workspace: opWs!, path: runnable.path })
						await jobLoader.runFlowPreview(args, flow, undefined, runnable.path, previewOnBehalfOf)
					} else {
						await jobLoader.runFlowByPath(runnable.path, args)
					}
				} else if (runnable.runType == 'agent') {
					// A turn of the agent as it is deployed, as the editor: the direct run has no way
					// to ask for the app's identity.
					const path = runnable.path
					const { user_message, memory_id, user_attachments } = args as Record<string, any>
					await jobLoader.abstractRun(() =>
						JobService.runAgent({
							workspace: opWs!,
							path,
							memoryId: memory_id || undefined,
							requestBody: { user_message, user_attachments }
						})
					)
				} else if (runnable.runType == 'script' || runnable.runType == 'hubscript') {
					if (previewOnBehalfOf) {
						const script = await getScriptByPath(runnable.path, opWs)
						await jobLoader.runPreview(
							runnable.path,
							script.content,
							script.language,
							args,
							script.tag,
							script.lock,
							script.hash,
							undefined,
							undefined,
							undefined,
							undefined,
							undefined,
							{ onBehalfOf: previewOnBehalfOf }
						)
					} else {
						await jobLoader.runScriptByPath(runnable.path, args)
					}
				}
			}
		}
	}
	$effect(() => {
		onFieldsChange(runnable?.fields ?? {})
	})
</script>

<JobLoader
	noCode={true}
	workspaceOverride={opWs}
	bind:scriptProgress
	bind:this={jobLoader}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
/>

{#if isRunnableByPath(runnable) || (isRunnableByName(runnable) && runnable.inlineScript)}
	<Splitpanes>
		<Pane size={55}>
			{#if isRunnableByName(runnable)}
				<RawAppInlineScriptEditor
					bind:this={inlineScriptEditor}
					on:createScriptFromInlineScript={() => dispatch('createScriptFromInlineScript', runnable)}
					{id}
					bind:inlineScript={runnable.inlineScript}
					bind:name={runnable.name}
					bind:fields={runnable.fields}
					bind:delete_after_secs={runnable.delete_after_secs}
					bind:viewer_data_access={runnable.viewer_data_access}
					onRun={testPreview}
					on:delete
					path={appPath}
					{onSelectionChange}
				/>
			{:else if isRunnableByPath(runnable)}
				<InlineScriptRunnableByPath
					rawApps
					bind:runnable
					bind:fields={runnable.fields}
					bind:hubFlowPreview
					on:fork={(e) => fork(e.detail)}
					on:delete
					{id}
					isLoading={testIsLoading}
					onRun={testPreview}
					onCancel={async () => {
						if (jobLoader) {
							await jobLoader.cancelJob()
						}
					}}
				/>
			{/if}
		</Pane>
		<Pane size={45}>
			<Tabs bind:selected={selectedTab}>
				<Tab value="test" label="Test" />
				<Tab value="inputs" label="Inputs" />
				{#snippet content()}
					{#if selectedTab == 'inputs'}
						{#if runnable?.fields}
							<div class="w-full flex flex-col gap-4 p-2">
								{#each Object.keys(runnable.fields) as k}
									{@const meta = runnable.fields[k]}
									<RawAppInputsSpecEditor
										key={k}
										bind:componentInput={runnable.fields[k]}
										{id}
										shouldCapitalize
										fieldType={meta?.['fieldType']}
										subFieldType={meta?.['subFieldType']}
										format={meta?.['format']}
										selectOptions={meta?.['selectOptions']}
										tooltip={meta?.['tooltip']}
										placeholder={meta?.['placeholder']}
										customTitle={meta?.['customTitle']}
										loading={meta?.['loading']}
										documentationLink={meta?.['documentationLink']}
										allowTypeChange={meta?.['allowTypeChange']}
										displayType
										suggestCtx={isSql && VIEWER_IDENTITY_ARGS.includes(k)}
									/>
								{/each}
							</div>
						{:else}
							<div class="text-primary text-xs">No inputs</div>
						{/if}
					{:else if selectedTab == 'test'}
						{#if debugMode && isDebuggableScript}
							<div transition:slide={{ duration: 200 }}>
								<DebugToolbar
									connected={$debugState.connected}
									running={$debugState.running}
									stopped={$debugState.stopped}
									breakpointCount={editorDebugState.debugBreakpoints?.size ?? 0}
									onStart={() => inlineScriptEditor?.startDebugging() ?? Promise.resolve()}
									onStop={() => inlineScriptEditor?.stopDebugging() ?? Promise.resolve()}
									onContinue={() => inlineScriptEditor?.continueExecution() ?? Promise.resolve()}
									onStepOver={() => inlineScriptEditor?.stepOver() ?? Promise.resolve()}
									onStepIn={() => inlineScriptEditor?.stepIn() ?? Promise.resolve()}
									onStepOut={() => inlineScriptEditor?.stepOut() ?? Promise.resolve()}
									onClearBreakpoints={() => inlineScriptEditor?.clearAllBreakpoints()}
									onExitDebug={() => inlineScriptEditor?.toggleDebugMode()}
								/>
							</div>
						{/if}
						<SplitPanesWrapper>
							<Splitpanes horizontal class="grow">
								<Pane size={50}>
									<div class="px-2 py-3 h-full overflow-auto">
										<div class="mx-auto w-fit">
											<RunButton
												isLoading={testIsLoading}
												onRun={testPreview}
												onCancel={async () => {
													if (jobLoader) {
														await jobLoader.cancelJob()
													}
												}}
												size="md"
											/>
										</div>
										<SchemaForm
											on:keydownCmdEnter={testPreview}
											workspace={opWs}
											disabledArgs={Object.entries(runnable?.fields ?? {})
												.filter(([_, v]) => v.type == 'static')
												.map(([k]) => k)}
											schema={runnable ? getSchema(runnable) : {}}
											bind:args
										/>
									</div>
								</Pane>
								<Pane size={50}>
									{#if showDebugPanel || hasDebugResult}
										<Splitpanes horizontal class="h-full">
											<Pane size={50} minSize={15}>
												<Splitpanes horizontal class="h-full">
													<Pane size={50} minSize={10}>
														<LogViewer
															small
															content={$debugState.logs}
															isLoading={$debugState.running && !$debugState.stopped}
															tag={undefined}
														/>
													</Pane>
													<Pane size={50} minSize={10}>
														{#if hasDebugResult}
															<div class="h-full p-2 overflow-auto">
																<DisplayResult
																	result={$debugState.result}
																	language={runnable?.inlineScript?.language}
																/>
															</div>
														{:else}
															<div
																class="h-full flex items-center justify-center text-sm text-tertiary"
															>
																{#if $debugState.running && !$debugState.stopped}
																	Running...
																{:else if $debugState.stopped}
																	Paused at breakpoint
																{:else}
																	Waiting for debug session
																{/if}
															</div>
														{/if}
													</Pane>
												</Splitpanes>
											</Pane>
											<Pane size={50} minSize={15}>
												<DebugPanel
													stackFrames={$debugState.stackFrames}
													scopes={$debugState.scopes}
													variables={$debugState.variables}
													client={dapClient}
													bind:selectedFrameId={selectedDebugFrameId}
												/>
											</Pane>
										</Splitpanes>
									{:else if debugMode && isDebuggableScript}
										<div class="h-full flex items-center justify-center text-sm text-tertiary">
											Click "Debug" in the toolbar to start debugging
										</div>
									{:else}
										<RunnableJobPanelInner frontendJob={false} {testJob} {testIsLoading} />
									{/if}
								</Pane>
							</Splitpanes>
						</SplitPanesWrapper>
					{/if}
				{/snippet}
			</Tabs>
		</Pane>
	</Splitpanes>
{:else}
	<EmptyInlineScript
		unusedInlineScripts={[]}
		rawApps
		on:pick={(e) =>
			onPick(e.detail as { runnable: Runnable; fields: Record<string, StaticAppInput> })}
		on:delete
		showScriptPicker
		on:new={(e) => {
			runnable = {
				type: 'inline',
				inlineScript: e.detail,
				name: runnable?.name ?? 'Background Runnable'
			}
		}}
	/>
{/if}
