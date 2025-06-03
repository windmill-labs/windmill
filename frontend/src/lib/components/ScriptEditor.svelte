<script lang="ts">
	import { BROWSER } from 'esm-env'

	import type { Schema, SupportedLanguage } from '$lib/common'
	import { type CompletedJob, type Job, JobService, type Preview, type ScriptLang } from '$lib/gen'
	import { copilotInfo, enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { copyToClipboard, emptySchema, sendUserToast } from '$lib/utils'
	import Editor from './Editor.svelte'
	import { inferArgs } from '$lib/infer'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import SchemaForm from './SchemaForm.svelte'
	import LogPanel from './scriptEditor/LogPanel.svelte'
	import EditorBar, { EDITOR_BAR_WIDTH_THRESHOLD } from './EditorBar.svelte'
	import TestJobLoader from './TestJobLoader.svelte'
	import JobProgressBar from '$lib/components/jobs/JobProgressBar.svelte'
	import { createEventDispatcher, onDestroy, onMount } from 'svelte'
	import { Button } from './common'
	import SplitPanesWrapper from './splitPanes/SplitPanesWrapper.svelte'
	import WindmillIcon from './icons/WindmillIcon.svelte'
	import * as Y from 'yjs'
	import { scriptLangToEditorLang } from '$lib/scripts'
	import { WebsocketProvider } from 'y-websocket'
	import Modal from './common/modal/Modal.svelte'
	import DiffEditor from './DiffEditor.svelte'
	import {
		Clipboard,
		CornerDownLeft,
		ExternalLink,
		Github,
		Play,
		PlayIcon,
		WandSparkles
	} from 'lucide-svelte'
	import { setLicense } from '$lib/enterpriseUtils'
	import type { ScriptEditorWhitelabelCustomUi } from './custom_ui'
	import Tabs from './common/tabs/Tabs.svelte'
	import Tab from './common/tabs/Tab.svelte'
	import { slide } from 'svelte/transition'
	import CaptureTable from '$lib/components/triggers/CaptureTable.svelte'
	import CaptureButton from './triggers/CaptureButton.svelte'
	import { setContext } from 'svelte'
	import HideButton from './apps/editor/settingsPanel/HideButton.svelte'
	import { base } from '$lib/base'
	import { SUPPORTED_CHAT_SCRIPT_LANGUAGES } from './copilot/chat/script/core'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'
	import { getStringError } from './copilot/chat/utils'
	import type { ScriptOptions } from './copilot/chat/ContextManager.svelte'
	import { AIChatService } from './copilot/chat/AIChatManager.svelte'

	// Exported
	export let schema: Schema | any = emptySchema()
	export let code: string
	export let path: string | undefined
	export let lang: Preview['language']
	export let kind: string | undefined = undefined
	export let template: 'pgsql' | 'mysql' | 'script' | 'docker' | 'powershell' | 'bunnative' =
		'script'
	export let tag: string | undefined
	export let initialArgs: Record<string, any> = {}
	export let fixedOverflowWidgets = true
	export let noSyncFromGithub = false
	export let editor: Editor | undefined = undefined
	export let diffEditor: DiffEditor | undefined = undefined
	export let collabMode = false
	export let edit = true
	export let noHistory = false
	export let saveToWorkspace = false
	export let watchChanges = false
	export let customUi: ScriptEditorWhitelabelCustomUi | undefined = undefined
	export let args: Record<string, any> = initialArgs
	export let selectedTab: 'main' | 'preprocessor' = 'main'
	export let hasPreprocessor = false
	export let captureTable: CaptureTable | undefined = undefined
	export let showCaptures: boolean = true
	export let stablePathForCaptures: string = ''
	export let lastSavedCode: string | undefined = undefined
	export let lastDeployedCode: string | undefined = undefined

	let showHistoryDrawer = false

	let jobProgressReset: () => void
	let diffMode = false

	let websocketAlive = {
		pyright: false,
		deno: false,
		go: false,
		ruff: false,
		shellcheck: false
	}

	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)

	$: watchChanges &&
		(code != undefined || schema != undefined) &&
		dispatchIfMounted('change', { code, schema })

	let width = 1200

	let testJobLoader: TestJobLoader

	let isValid: boolean = true
	let scriptProgress = undefined

	// Test
	let testIsLoading = false
	let testJob: Job | undefined
	let pastPreviews: CompletedJob[] = []
	let validCode = true

	let wsProvider: WebsocketProvider | undefined = undefined
	let yContent: Y.Text | undefined = undefined
	let peers: { name: string }[] = []
	let showCollabPopup = false

	const url = new URL(window.location.toString())
	let initialCollab = /true|1/i.test(url.searchParams.get('collab') ?? '0')

	if (initialCollab) {
		setCollaborationMode()
		url.searchParams.delete('collab')
		url.searchParams.delete('path')
		history.replaceState(null, '', url)
	}

	function onKeyDown(event: KeyboardEvent) {
		if ((event.ctrlKey || event.metaKey) && event.key == 'Enter') {
			event.preventDefault()
			runTest()
		} else if ((event.ctrlKey || event.metaKey) && event.key == 'u') {
			event.preventDefault()
			toggleTestPanel()
		}
	}

	export function setArgs(nargs: Record<string, any>) {
		args = nargs
	}

	export async function runTest() {
		// Not defined if JobProgressBar not loaded
		if (jobProgressReset) jobProgressReset()
		//@ts-ignore
		let job = await testJobLoader.runPreview(
			path,
			code,
			lang,
			selectedTab === 'preprocessor' || kind === 'preprocessor'
				? { _ENTRYPOINT_OVERRIDE: 'preprocessor', ...args }
				: args,
			tag
		)
		setFocusToLogs()
		return job
	}

	async function loadPastTests(): Promise<void> {
		pastPreviews = await JobService.listCompletedJobs({
			workspace: $workspaceStore!,
			jobKinds: 'preview',
			createdBy: $userStore?.username,
			scriptPathExact: path
		})
	}

	export async function inferSchema(code: string, nlang?: SupportedLanguage, resetArgs = false) {
		let nschema = schema ?? emptySchema()

		try {
			const result = await inferArgs(
				nlang ?? lang,
				code,
				nschema,
				selectedTab === 'preprocessor' || kind === 'preprocessor' ? 'preprocessor' : undefined
			)

			if (kind === 'preprocessor') {
				hasPreprocessor = false
				selectedTab = 'main'
			} else {
				hasPreprocessor =
					(selectedTab === 'preprocessor' ? !result?.no_main_func : result?.has_preprocessor) ??
					false

				if (!hasPreprocessor && selectedTab === 'preprocessor') {
					selectedTab = 'main'
				}
			}

			validCode = true
			if (resetArgs) {
				args = {}
			}
			schema = nschema
		} catch (e) {
			validCode = false
		}
	}

	onMount(() => {
		inferSchema(code)
		loadPastTests()
	})

	setLicense()
	export async function setCollaborationMode() {
		await setLicense()
		if (!$enterpriseLicense) {
			sendUserToast(`Multiplayer is an enterprise feature`, true, [
				{
					label: 'Upgrade',
					callback: () => {
						window.open('https://www.windmill.dev/pricing', '_blank')
					}
				}
			])
			return
		}

		const ydoc = new Y.Doc()
		if (wsProvider) {
			wsProvider.destroy()
		}
		let yContentInit = ydoc.getText('content')

		const wsProtocol = BROWSER && window.location.protocol == 'https:' ? 'wss' : 'ws'

		wsProvider = new WebsocketProvider(
			`${wsProtocol}://${window.location.host}/ws_mp/`,
			$workspaceStore + '/' + (path ?? 'no-room-name'),
			ydoc,
			{ connect: false }
		)

		wsProvider.on('sync', (isSynced: boolean) => {
			if (isSynced && yContentInit?.toJSON() == '') {
				showCollabPopup = true
				yContentInit?.insert(0, code)
			}
			yContent = yContentInit
		})

		wsProvider.on('connection-error', (WSErrorEvent) => {
			console.error(WSErrorEvent)
			sendUserToast('Multiplayer server connection had an error', true)
		})
		wsProvider.connect()
		const awareness = wsProvider.awareness

		awareness.setLocalStateField('user', {
			name: $userStore?.username
		})

		function setPeers() {
			peers = Array.from(awareness.getStates().values()).map((x) => x?.['user'])
		}

		setPeers()
		// You can observe when a user updates their awareness information
		awareness.on('change', (changes) => {
			setPeers()
		})
	}

	export function disableCollaboration() {
		if (!wsProvider?.shouldConnect) return
		peers = []
		console.log('collab mode disabled')
		wsProvider?.disconnect()
		wsProvider.destroy()
		wsProvider = undefined
	}

	onDestroy(() => {
		disableCollaboration()
		AIChatService.scriptEditorApplyCode = undefined
		AIChatService.scriptEditorShowDiffMode = undefined
		AIChatService.scriptEditorOptions = undefined
	})

	function asKind(str: string | undefined) {
		return str as 'script' | 'approval' | 'trigger' | undefined
	}

	function collabUrl() {
		let url = new URL(window.location.toString().split('#')[0])
		url.search = ''
		return `${url}?collab=1` + (edit ? '' : `&path=${path}`)
	}

	$: showTabs = hasPreprocessor
	$: !hasPreprocessor && (selectedTab = 'main')
	$: selectedTab && inferSchema(code)

	let argsRender = 0
	export async function updateArgs(newArgs: Record<string, any>) {
		if (Object.keys(newArgs).length > 0) {
			args = { ...newArgs }
			argsRender++
		}
	}

	let setFocusToLogs = () => {}

	setContext('disableTooltips', customUi?.disableTooltips === true)

	let codePanelSize = 70
	let testPanelSize = 30
	let storedTestPanelSize = testPanelSize

	function addSelectedLinesToAiChat(
		e: CustomEvent<{ lines: string; startLine: number; endLine: number }>
	) {
		AIChatService.addSelectedLinesToContext(e.detail.lines, e.detail.startLine, e.detail.endLine)
		// AIChatService.focusTextArea() TODO: Add this back
	}

	$: !SUPPORTED_CHAT_SCRIPT_LANGUAGES.includes(lang ?? '') &&
		!AIChatService.open &&
		AIChatService.toggleOpen()

	function toggleTestPanel() {
		if (testPanelSize > 0) {
			storedTestPanelSize = testPanelSize
			codePanelSize += testPanelSize
			testPanelSize = 0
		} else {
			codePanelSize -= storedTestPanelSize
			testPanelSize = storedTestPanelSize
		}
	}

	function getError(job: Job | undefined) {
		if (job != undefined && job.type === 'CompletedJob' && !job.success) {
			return getStringError(job.result)
		}
		return undefined
	}

	function showDiffMode() {
		diffMode = true
		diffEditor?.setOriginal(lastDeployedCode ?? '')
		diffEditor?.setModified(editor?.getCode() ?? '')
		diffEditor?.show()
		editor?.hide()
	}

	function hideDiffMode() {
		diffMode = false
		diffEditor?.hide()
		editor?.show()
	}

	$: error = getError(testJob)

	$: {
		const options: ScriptOptions = {
			code,
			lang: lang as ScriptLang,
			error,
			args,
			path,
			lastSavedCode,
			lastDeployedCode,
			diffMode
		}
		AIChatService.scriptEditorOptions = options
		AIChatService.scriptEditorApplyCode = (code: string) => {
			hideDiffMode()
			editor?.reviewAndApplyCode(code)
		}
		AIChatService.scriptEditorShowDiffMode = showDiffMode
	}
</script>

<TestJobLoader
	on:done={loadPastTests}
	bind:scriptProgress
	bind:this={testJobLoader}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
/>

<svelte:window on:keydown={onKeyDown} />

<Modal title="Invite others" bind:open={showCollabPopup}>
	<div>Have others join by sharing the following url:</div>
	<div class="flex gap-2 pr-4">
		<input type="text" disabled value={collabUrl()} />

		<Button
			color="light"
			startIcon={{ icon: Clipboard }}
			iconOnly
			on:click={() => copyToClipboard(collabUrl())}
		/>
	</div>
</Modal>
<div class="border-b shadow-sm px-1 pr-4" bind:clientWidth={width}>
	<div class="flex justify-between space-x-2">
		<EditorBar
			scriptPath={edit ? path : undefined}
			on:toggleCollabMode={() => {
				if (wsProvider?.shouldConnect) {
					disableCollaboration()
				} else {
					setCollaborationMode()
				}
			}}
			on:showDiffMode={showDiffMode}
			on:hideDiffMode={hideDiffMode}
			customUi={{ ...customUi?.editorBar, aiGen: false }}
			collabLive={wsProvider?.shouldConnect}
			{collabMode}
			{validCode}
			iconOnly={width < EDITOR_BAR_WIDTH_THRESHOLD}
			on:collabPopup={() => (showCollabPopup = true)}
			{editor}
			{lang}
			on:createScriptFromInlineScript
			{websocketAlive}
			collabUsers={peers}
			kind={asKind(kind)}
			{template}
			{diffEditor}
			{args}
			{noHistory}
			{saveToWorkspace}
			lastDeployedCode={lastDeployedCode && lastDeployedCode !== code
				? lastDeployedCode
				: undefined}
			{diffMode}
			bind:showHistoryDrawer
		>
			<slot name="editor-bar-right" slot="right" />
		</EditorBar>
		{#if !noSyncFromGithub && customUi?.editorBar?.useVsCode != false}
			<div class="py-1">
				<Button
					target="_blank"
					href="https://www.windmill.dev/docs/cli_local_dev/vscode-extension"
					color="light"
					size="xs"
					btnClasses="hidden lg:flex"
					startIcon={{
						icon: Github
					}}
				>
					Use VScode
				</Button>
			</div>
		{/if}
	</div>
</div>
<SplitPanesWrapper>
	<Splitpanes class="!overflow-visible">
		<Pane bind:size={codePanelSize} minSize={10} class="!overflow-visible">
			<div class="h-full !overflow-visible bg-gray-50 dark:bg-[#272D38] relative">
				<div class="absolute top-2 right-4 z-10 flex flex-row gap-2">
					{#if !AIChatService.open}
						{#if customUi?.editorBar?.aiGen != false && SUPPORTED_CHAT_SCRIPT_LANGUAGES.includes(lang ?? '')}
							<HideButton
								hidden={true}
								direction="right"
								panelName="AI"
								shortcut="L"
								size="md"
								usePopoverOverride={!$copilotInfo.enabled}
								customHiddenIcon={WandSparkles}
								btnClasses="!text-violet-800 dark:!text-violet-400 border border-gray-200 dark:border-gray-600 bg-surface"
								on:click={() => {
									AIChatService.toggleOpen()
								}}
							>
								<svelte:fragment slot="popoverOverride">
									<div class="text-sm">
										Enable Windmill AI in the <a
											href="{base}/workspace_settings?tab=ai"
											target="_blank"
											class="inline-flex flex-row items-center gap-1"
										>
											workspace settings <ExternalLink size={16} />
										</a>
									</div>
								</svelte:fragment>
							</HideButton>
						{/if}
						{#if testPanelSize === 0}
							<HideButton
								hidden={true}
								direction="right"
								size="md"
								panelName="Test"
								shortcut="U"
								customHiddenIcon={PlayIcon}
								on:click={() => {
									toggleTestPanel()
								}}
								btnClasses="bg-marine-400 hover:bg-marine-200 !text-primary-inverse hover:!text-primary-inverse hover:dark:!text-primary-inverse dark:bg-marine-50 dark:hover:bg-marine-50/70"
								color="marine"
							/>
						{/if}
					{/if}
				</div>
				{#key lang}
					<Editor
						lineNumbersMinChars={4}
						folding
						{path}
						bind:code
						bind:websocketAlive
						bind:this={editor}
						{yContent}
						awareness={wsProvider?.awareness}
						on:change={(e) => {
							inferSchema(e.detail)
						}}
						on:saveDraft
						on:toggleAiPanel={() => AIChatService.toggleOpen()}
						on:addSelectedLinesToAiChat={addSelectedLinesToAiChat}
						on:toggleTestPanel={toggleTestPanel}
						cmdEnterAction={async () => {
							await inferSchema(code)
							runTest()
						}}
						formatAction={async () => {
							await inferSchema(code)
							try {
								localStorage.setItem(path ?? 'last_save', code)
							} catch (e) {
								console.error('Could not save last_save to local storage', e)
							}
							dispatch('format')
						}}
						class="flex flex-1 h-full !overflow-visible"
						scriptLang={lang}
						automaticLayout={true}
						{fixedOverflowWidgets}
						{args}
					/>
					<DiffEditor
						class="h-full"
						bind:this={diffEditor}
						automaticLayout
						defaultLang={scriptLangToEditorLang(lang)}
						{fixedOverflowWidgets}
						showButtons={diffMode}
						on:hideDiffMode={hideDiffMode}
						on:seeHistory={() => {
							showHistoryDrawer = true
						}}
					/>
				{/key}
			</div>
		</Pane>
		<Pane bind:size={testPanelSize} minSize={0}>
			<div class="flex flex-col h-full">
				{#if showTabs}
					<div transition:slide={{ duration: 200 }}>
						<Tabs bind:selected={selectedTab}>
							<Tab value="main">Main</Tab>
							{#if hasPreprocessor}
								<div transition:slide={{ duration: 200, axis: 'x' }}>
									<Tab value="preprocessor">Preprocessor</Tab>
								</div>
							{/if}
						</Tabs>
					</div>
				{/if}

				<div class="flex justify-center pt-1 relative">
					<div class="absolute top-2 left-2">
						<HideButton
							hidden={false}
							direction="right"
							panelName="Test"
							shortcut="U"
							size="md"
							on:click={() => {
								toggleTestPanel()
							}}
						/>
					</div>
					{#if testIsLoading}
						<Button on:click={testJobLoader?.cancelJob} btnClasses="w-full" color="red" size="xs">
							<WindmillIcon
								white={true}
								class="mr-2 text-white"
								height="16px"
								width="20px"
								spin="fast"
							/>
							Cancel
						</Button>
					{:else if customUi?.previewPanel?.disableTriggerButton !== true}
						<div class="flex flex-row divide-x divide-gray-800 dark:divide-gray-300 items-stretch">
							<Button
								color="dark"
								on:click={() => {
									runTest()
								}}
								btnClasses="w-full rounded-r-none"
								size="xs"
								startIcon={{
									icon: Play,
									classes: 'animate-none'
								}}
								shortCut={{ Icon: CornerDownLeft, hide: testIsLoading }}
							>
								{#if testIsLoading}
									Running
								{:else}
									Test
								{/if}
							</Button>
							<CaptureButton on:openTriggers />
						</div>
					{:else}
						<div class="flex flex-row divide-x divide-gray-800 dark:divide-gray-300 items-stretch">
							<Button
								color="dark"
								on:click={() => {
									runTest()
								}}
								btnClasses="w-full"
								size="xs"
								startIcon={{
									icon: Play,
									classes: 'animate-none'
								}}
								shortCut={{ Icon: CornerDownLeft, hide: testIsLoading }}
							>
								{#if testIsLoading}
									Running
								{:else}
									Test
								{/if}
							</Button>
						</div>
					{/if}
				</div>
				<Splitpanes horizontal class="!max-h-[calc(100%-43px)]">
					<Pane size={33}>
						<div class="px-4">
							<div class="break-words relative font-sans">
								{#key argsRender}
									<SchemaForm
										helperScript={{
											type: 'inline',
											code,
											//@ts-ignore
											lang
										}}
										compact
										{schema}
										bind:args
										bind:isValid
										noVariablePicker={customUi?.previewPanel?.disableVariablePicker === true}
										showSchemaExplorer
									/>
								{/key}
							</div>
						</div>
					</Pane>
					<Pane size={67} class="relative">
						<LogPanel
							bind:setFocusToLogs
							on:fix={() => {
								AIChatService.fix()
							}}
							fixChatMode
							{lang}
							previewJob={testJob}
							{pastPreviews}
							previewIsLoading={testIsLoading}
							{editor}
							{diffEditor}
							{args}
							{showCaptures}
							customUi={customUi?.previewPanel}
						>
							{#if scriptProgress}
								<!-- Put to the slot in logpanel -->
								<JobProgressBar
									job={testJob}
									bind:scriptProgress
									bind:reset={jobProgressReset}
									compact={true}
								/>
							{/if}
							<svelte:fragment slot="capturesTab">
								<div class="h-full p-2">
									<CaptureTable
										bind:this={captureTable}
										{hasPreprocessor}
										canHavePreprocessor={lang === 'bun' || lang === 'deno' || lang === 'python3'}
										isFlow={false}
										path={stablePathForCaptures}
										canEdit={true}
										on:applyArgs
										on:updateSchema
										on:addPreprocessor
									/>
								</div>
							</svelte:fragment>
						</LogPanel>
					</Pane>
				</Splitpanes>
			</div>
		</Pane>
	</Splitpanes>
</SplitPanesWrapper>
