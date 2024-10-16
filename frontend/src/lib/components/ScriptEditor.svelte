<script lang="ts">
	import { BROWSER } from 'esm-env'

	import type { Schema, SupportedLanguage } from '$lib/common'
	import { type CompletedJob, type Job, JobService, type Preview } from '$lib/gen'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
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
	import { Clipboard, CornerDownLeft, Github, Play } from 'lucide-svelte'
	import { setLicense } from '$lib/enterpriseUtils'
	import type { ScriptEditorWhitelabelCustomUi } from './custom_ui'
	import Tabs from './common/tabs/Tabs.svelte'
	import Tab from './common/tabs/Tab.svelte'
	import { slide } from 'svelte/transition'

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
	export let customUi: ScriptEditorWhitelabelCustomUi = {}

	let jobProgressReset: () => void

	let websocketAlive = {
		pyright: false,
		deno: false,
		go: false,
		ruff: false,
		shellcheck: false
	}

	const dispatch = createEventDispatcher()

	$: watchChanges &&
		(code != undefined || schema != undefined) &&
		dispatch('change', { code, schema })

	let width = 1200

	let testJobLoader: TestJobLoader

	// Test args input
	let args: Record<string, any> = initialArgs

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
		}
	}

	function runTest() {
		// Not defined if JobProgressBar not loaded
		if (jobProgressReset) jobProgressReset()
		//@ts-ignore
		testJobLoader.runPreview(
			path,
			code,
			lang,
			selectedTab === 'preprocessor' ? { _ENTRYPOINT_OVERRIDE: 'preprocessor', ...args } : args,
			tag
		)
	}

	async function loadPastTests(): Promise<void> {
		pastPreviews = await JobService.listCompletedJobs({
			workspace: $workspaceStore!,
			jobKinds: 'preview',
			createdBy: $userStore?.username,
			scriptPathExact: path
		})
	}

	let hasPreprocessor = false

	export async function inferSchema(code: string, nlang?: SupportedLanguage) {
		let nschema = schema ?? emptySchema()

		try {
			const result = await inferArgs(
				nlang ?? lang,
				code,
				nschema,
				selectedTab === 'preprocessor' ? 'preprocessor' : undefined
			)
			hasPreprocessor =
				(selectedTab === 'preprocessor' ? !result?.no_main_func : result?.has_preprocessor) ?? false

			validCode = true
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
	})

	function asKind(str: string | undefined) {
		return str as 'script' | 'approval' | 'trigger' | undefined
	}

	function collabUrl() {
		let url = new URL(window.location.toString().split('#')[0])
		url.search = ''
		return `${url}?collab=1` + (edit ? '' : `&path=${path}`)
	}
	let selectedTab: 'main' | 'preprocessor' = 'main'
	$: showTabs = hasPreprocessor
	$: !hasPreprocessor && (selectedTab = 'main')

	$: selectedTab && inferSchema(code)
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
			customUi={customUi?.editorBar}
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
		/>
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
		<Pane size={60} minSize={10} class="!overflow-visible">
			<div class="pl-2 h-full !overflow-visible bg-gray-50 dark:bg-[#272D38]">
				{#key lang}
					<Editor
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
						lang={scriptLangToEditorLang(lang)}
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
					/>
				{/key}
			</div>
		</Pane>
		<Pane size={40} minSize={10}>
			<div class="flex flex-col h-full">
				{#if showTabs}
					<div transition:slide={{ duration: 200 }}>
						<Tabs bind:selected={selectedTab}>
							<Tab value="main">Main</Tab>
							<Tab value="preprocessor">Preprocessor</Tab>
						</Tabs>
					</div>
				{/if}
				<div class="flex justify-center pt-1">
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
					{:else}
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
					{/if}
				</div>
				<Splitpanes horizontal class="!max-h-[calc(100%-43px)]">
					<Pane size={33}>
						<div class="px-2">
							<div class="break-words relative font-sans">
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
									showSchemaExplorer
								/>
							</div>
						</div>
					</Pane>
					<Pane size={67}>
						<LogPanel
							{lang}
							previewJob={testJob}
							{pastPreviews}
							previewIsLoading={testIsLoading}
							{editor}
							{diffEditor}
							{args}
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
						</LogPanel>
					</Pane>
				</Splitpanes>
			</div>
		</Pane>
	</Splitpanes>
</SplitPanesWrapper>
