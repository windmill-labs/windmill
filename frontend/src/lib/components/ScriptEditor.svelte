<script lang="ts">
	import { BROWSER } from 'esm-env'

	import type { Schema, SupportedLanguage } from '$lib/common'
	import {
		AssetService,
		type CompletedJob,
		type Job,
		JobService,
		type Preview,
		type ScriptLang
	} from '$lib/gen'
	import { copilotInfo, enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { copyToClipboard, emptySchema, sendUserToast } from '$lib/utils'
	import Editor from './Editor.svelte'
	import { inferArgs, inferAssets } from '$lib/infer'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import SchemaForm from './SchemaForm.svelte'
	import LogPanel from './scriptEditor/LogPanel.svelte'
	import EditorBar, { EDITOR_BAR_WIDTH_THRESHOLD } from './EditorBar.svelte'
	import TestJobLoader from './TestJobLoader.svelte'
	import JobProgressBar from '$lib/components/jobs/JobProgressBar.svelte'
	import { createEventDispatcher, onDestroy, onMount, untrack } from 'svelte'
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
	import { getStringError } from './copilot/chat/utils'
	import type { ScriptOptions } from './copilot/chat/ContextManager.svelte'
	import { aiChatManager, AIMode } from './copilot/chat/AIChatManager.svelte'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import AssetsDropdownButton from './assets/AssetsDropdownButton.svelte'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { assetEq, type AssetWithAccessType } from './assets/lib'

	interface Props {
		// Exported
		schema?: Schema | any
		code: string
		path: string | undefined
		lang: Preview['language']
		kind?: string | undefined
		template?: 'pgsql' | 'mysql' | 'script' | 'docker' | 'powershell' | 'bunnative'
		tag: string | undefined
		initialArgs?: Record<string, any>
		fixedOverflowWidgets?: boolean
		noSyncFromGithub?: boolean
		editor?: Editor | undefined
		diffEditor?: DiffEditor | undefined
		collabMode?: boolean
		edit?: boolean
		noHistory?: boolean
		saveToWorkspace?: boolean
		watchChanges?: boolean
		customUi?: ScriptEditorWhitelabelCustomUi | undefined
		args: Record<string, any>
		selectedTab?: 'main' | 'preprocessor'
		hasPreprocessor?: boolean
		captureTable?: CaptureTable | undefined
		showCaptures?: boolean
		stablePathForCaptures?: string
		lastSavedCode?: string | undefined
		lastDeployedCode?: string | undefined
		disableAi?: boolean
		editor_bar_right?: import('svelte').Snippet
		fallbackAccessTypes?: AssetWithAccessType[]
	}

	let {
		schema = $bindable(),
		code = $bindable(),
		path,
		lang,
		kind = undefined,
		template = 'script',
		tag,
		fixedOverflowWidgets = true,
		noSyncFromGithub = false,
		editor = $bindable(undefined),
		diffEditor = $bindable(undefined),
		collabMode = false,
		edit = true,
		noHistory = false,
		saveToWorkspace = false,
		watchChanges = false,
		customUi = undefined,
		args = $bindable(),
		selectedTab = $bindable('main'),
		hasPreprocessor = $bindable(false),
		captureTable = $bindable(undefined),
		showCaptures = true,
		stablePathForCaptures = '',
		lastSavedCode = undefined,
		lastDeployedCode = undefined,
		disableAi = false,
		editor_bar_right,
		fallbackAccessTypes = $bindable()
	}: Props = $props()

	$effect.pre(() => {
		if (schema == undefined) {
			schema = emptySchema()
		}
	})
	let showHistoryDrawer = $state(false)

	let jobProgressReset: (() => void) | undefined = $state(undefined)
	let diffMode = $state(false)

	let websocketAlive = $state({
		pyright: false,
		deno: false,
		go: false,
		ruff: false,
		shellcheck: false
	})

	const dispatch = createEventDispatcher()

	$effect(() => {
		watchChanges &&
			(code != undefined || schema != undefined) &&
			dispatch('change', { code, schema })
	})

	let parsedAssets = usePromise(() => inferAssets(lang, code), { clearValueOnRefresh: false })
	$effect(() => {
		untrack(() => parsedAssets.refresh()), [lang, code]
	})

	// Load initial fallbackAccessTypes
	if (edit && path) {
		AssetService.listAssetsByUsage({
			workspace: $workspaceStore!,
			requestBody: { usages: [{ path, kind: 'script' }] }
		}).then((arr) => {
			const v = arr[0]
			setTimeout(() => {
				for (const a of parsedAssets.value ?? []) {
					const fallback = v.find((a2) => assetEq(a2, a))?.access_type
					if (!a.access_type && fallback) {
						fallbackAccessTypes = [...(fallbackAccessTypes ?? []), { ...a, access_type: fallback }]
					}
				}
			}, 200)
		})
	}

	let width = $state(1200)

	let testJobLoader: TestJobLoader | undefined = $state(undefined)

	let isValid: boolean = $state(true)
	let scriptProgress = $state(undefined)

	let logPanel: LogPanel | undefined = $state(undefined)
	// Test
	let testIsLoading = $state(false)
	let testJob: Job | undefined = $state()
	let pastPreviews: CompletedJob[] = $state([])
	let validCode = $state(true)

	let wsProvider: WebsocketProvider | undefined = $state(undefined)
	let yContent: Y.Text | undefined = $state(undefined)
	let peers: { name: string }[] = $state([])
	let showCollabPopup = $state(false)

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
				? { _ENTRYPOINT_OVERRIDE: 'preprocessor', ...(args ?? {}) }
				: (args ?? {}),
			tag
		)
		logPanel?.setFocusToLogs()
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
		aiChatManager.changeMode(AIMode.SCRIPT)
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
		aiChatManager.scriptEditorApplyCode = undefined
		aiChatManager.scriptEditorShowDiffMode = undefined
		aiChatManager.scriptEditorOptions = undefined
		aiChatManager.changeMode(AIMode.NAVIGATOR)
	})

	function asKind(str: string | undefined) {
		return str as 'script' | 'approval' | 'trigger' | undefined
	}

	function collabUrl() {
		let url = new URL(window.location.toString().split('#')[0])
		url.search = ''
		return `${url}?collab=1` + (edit ? '' : `&path=${path}`)
	}

	let showTabs = $derived(hasPreprocessor)
	$effect(() => {
		!hasPreprocessor && (selectedTab = 'main')
	})
	$effect(() => {
		selectedTab && code && untrack(() => inferSchema(code))
	})

	let argsRender = $state(0)
	export async function updateArgs(newArgs: Record<string, any>) {
		if (Object.keys(newArgs).length > 0) {
			args = { ...newArgs }
			argsRender++
		}
	}

	setContext('disableTooltips', customUi?.disableTooltips === true)

	let codePanelSize = $state(70)
	let testPanelSize = $state(30)
	let storedTestPanelSize = untrack(() => testPanelSize)

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

	let error = $derived(getError(testJob))

	$effect(() => {
		const options: ScriptOptions = {
			code,
			lang: lang as ScriptLang,
			error,
			args: args ?? {},
			path,
			lastSavedCode,
			lastDeployedCode,
			diffMode
		}
		untrack(() => {
			aiChatManager.scriptEditorOptions = options
			aiChatManager.scriptEditorApplyCode = (code: string) => {
				hideDiffMode()
				editor?.reviewAndApplyCode(code)
			}
			aiChatManager.scriptEditorShowDiffMode = showDiffMode
		})
	})
</script>

<TestJobLoader
	on:done={loadPastTests}
	bind:scriptProgress
	bind:this={testJobLoader}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
/>

<svelte:window onkeydown={onKeyDown} />

<!-- Standalone triggerable registration for the script editor -->
<div
	style="display: none"
	use:triggerableByAI={{
		id: 'script-editor',
		description: 'Component to edit a script'
	}}
></div>

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
		{#if args}
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
				{#snippet right()}
					{@render editor_bar_right?.()}
				{/snippet}
			</EditorBar>
		{/if}
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
					{#if parsedAssets.value?.length}
						<AssetsDropdownButton assets={parsedAssets.value} bind:fallbackAccessTypes />
					{/if}
					{#if testPanelSize === 0}
						<HideButton
							hidden={true}
							direction="right"
							size="md"
							panelName="Test"
							shortcut="U"
							customHiddenIcon={{
								icon: PlayIcon
							}}
							on:click={() => {
								toggleTestPanel()
							}}
							btnClasses="bg-marine-400 hover:bg-marine-200 !text-primary-inverse hover:!text-primary-inverse hover:dark:!text-primary-inverse dark:bg-marine-50 dark:hover:bg-marine-50/70"
							color="marine"
						/>
					{/if}
					{#if !aiChatManager.open && !disableAi}
						{#if customUi?.editorBar?.aiGen != false && SUPPORTED_CHAT_SCRIPT_LANGUAGES.includes(lang ?? '')}
							<HideButton
								hidden={true}
								direction="right"
								panelName="AI"
								shortcut="L"
								size="md"
								usePopoverOverride={!$copilotInfo.enabled}
								customHiddenIcon={{
									icon: WandSparkles
								}}
								btnClasses="!text-violet-800 dark:!text-violet-400 border border-gray-200 dark:border-gray-600 bg-surface"
								on:click={() => {
									if (!aiChatManager.open) {
										aiChatManager.changeMode(AIMode.SCRIPT)
									}
									aiChatManager.toggleOpen()
								}}
							>
								{#snippet popoverOverride()}
									<div class="text-sm">
										Enable Windmill AI in the <a
											href="{base}/workspace_settings?tab=ai"
											target="_blank"
											class="inline-flex flex-row items-center gap-1"
										>
											workspace settings <ExternalLink size={16} />
										</a>
									</div>
								{/snippet}
							</HideButton>
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
							bind:this={logPanel}
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
							{#snippet capturesTab()}
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
							{/snippet}
						</LogPanel>
					</Pane>
				</Splitpanes>
			</div>
		</Pane>
	</Splitpanes>
</SplitPanesWrapper>
