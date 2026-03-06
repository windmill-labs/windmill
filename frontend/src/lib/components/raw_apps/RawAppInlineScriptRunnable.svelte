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
	import RawAppInputsSpecEditor from './RawAppInputsSpecEditor.svelte'
	import SplitPanesWrapper from '../splitPanes/SplitPanesWrapper.svelte'
	import SchemaForm from '../SchemaForm.svelte'
	import RunnableJobPanelInner from '../apps/editor/RunnableJobPanelInner.svelte'
	import JobLoader from '../JobLoader.svelte'
	import type { Job, ScriptLang } from '$lib/gen'
	import { slide } from 'svelte/transition'
	import { DebugToolbar, DebugPanel, debugState } from '$lib/components/debug'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import RunButton from '$lib/components/RunButton.svelte'
	import { userStore, workspaceStore } from '$lib/stores'

	type RunnableWithInlineScript = RunnableWithFields & {
		inlineScript?: InlineScript & { language: ScriptLang }
	}
	export type Runnable = RunnableWithInlineScript | undefined
	interface Props {
		runnable: Runnable
		id: string
		appPath: string
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

	let { runnable = $bindable(), id, appPath, onSelectionChange }: Props = $props()

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

	let selectedTab = $state('test')
	let args = $state({})

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
				return $userStore?.username ?? ''
			case 'email':
				return $userStore?.email ?? ''
			case 'groups':
				return $userStore?.groups ?? []
			case 'workspace':
				return $workspaceStore ?? ''
			case 'author':
				return $userStore?.email ?? '' // In editor, author is the current user
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
				undefined
			)
		} else if (isRunnableByPath(runnable)) {
			if (jobLoader && isRunnableByPath(runnable)) {
				if (runnable.runType == 'flow') {
					await jobLoader.runFlowByPath(runnable.path, args)
				} else if (runnable.runType == 'script' || runnable.runType == 'hubscript') {
					await jobLoader.runScriptByPath(runnable.path, args)
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
