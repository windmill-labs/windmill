<script lang="ts">
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import { Button } from '$lib/components/common'
	import { WindmillIcon } from '$lib/components/icons'
	import LogPanel from '$lib/components/scriptEditor/LogPanel.svelte'
	import {
		type CompletedJob,
		type Job,
		JobService,
		OpenAPI,
		type Preview,
		type OpenFlow,
		WorkspaceService,
		type InputTransform,
		type TriggersCount
	} from '$lib/gen'
	import { inferArgs } from '$lib/infer'
	import { setCopilotInfo, userStore, workspaceStore } from '$lib/stores'
	import { emptySchema, sendUserToast } from '$lib/utils'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { onDestroy, onMount, setContext, untrack } from 'svelte'
	import DarkModeToggle from '$lib/components/sidebar/DarkModeToggle.svelte'
	import { page } from '$app/stores'
	import { getUserExt } from '$lib/user'
	import FlowPreviewButtons from './flows/header/FlowPreviewButtons.svelte'
	import FlowModuleSchemaMap from './flows/map/FlowModuleSchemaMap.svelte'
	import FlowEditorPanel from './flows/content/FlowEditorPanel.svelte'
	import { deepEqual } from 'fast-equals'
	import { writable } from 'svelte/store'
	import type { FlowState } from './flows/flowState'
	import { initHistory } from '$lib/history'
	import type { FlowEditorContext, FlowInput, FlowInputEditorState } from './flows/types'
	import { dfs } from './flows/dfs'
	import { loadSchemaFromModule } from './flows/flowInfers'
	import { CornerDownLeft, Play } from 'lucide-svelte'
	import Toggle from './Toggle.svelte'
	import { setLicense } from '$lib/enterpriseUtils'
	import type { FlowCopilotContext } from './copilot/flow'
	import {
		approximateFindPythonRelativePath,
		isTypescriptRelativePath,
		parseTypescriptDeps
	} from '$lib/relative_imports'
	import Tooltip from './Tooltip.svelte'
	import type { TriggerContext } from './triggers'
	import { workspaceAIClients } from './copilot/lib'
	import type { FlowPropPickerConfig, PropPickerContext } from './prop_picker'
	import type { PickableProperties } from './flows/previousResults'
	import { Triggers } from './triggers/triggers.svelte'

	let flowCopilotContext: FlowCopilotContext = {
		shouldUpdatePropertyType: writable<{
			[key: string]: 'static' | 'javascript' | undefined
		}>({}),
		exprsToSet: writable<{
			[key: string]: InputTransform | any | undefined
		}>({}),
		generatedExprs: writable<{
			[key: string]: string | undefined
		}>({}),
		stepInputsLoading: writable<boolean>(false)
	}

	setContext('FlowCopilotContext', flowCopilotContext)

	async function setupCopilotInfo() {
		if (workspace) {
			workspaceAIClients.init(workspace)
			try {
				const info = await WorkspaceService.getCopilotInfo({ workspace })
				setCopilotInfo(info)
			} catch (err) {
				console.error('Could not get copilot info', err)
				setCopilotInfo({})
			}
		}
	}

	async function loadUser() {
		try {
			const user = await getUserExt(workspace!)
			$userStore = user
		} catch (e) {
			sendUserToast(`Failed to load user ${e}`, true)
		}
	}

	let darkModeToggle: DarkModeToggle | undefined = $state()
	let darkMode: boolean | undefined = $state(undefined)
	let modeInitialized = $state(false)
	function initializeMode() {
		modeInitialized = true
		darkModeToggle?.toggle()
	}

	let testJobLoader: TestJobLoader | undefined = $state()
	let socket: WebSocket | undefined = undefined

	// Test args input
	let args: Record<string, any> = $state({})
	let isValid: boolean = $state(true)

	// Test
	let testIsLoading = $state(false)
	let testJob: Job | undefined = $state()
	let pastPreviews: CompletedJob[] = $state([])
	let validCode = $state(true)

	type LastEditScript = {
		content: string
		path: string
		language: Preview['language']
		lock?: string
		isCodebase?: boolean
		tag?: string
	}

	let currentScript: LastEditScript | undefined = $state(undefined)

	let schema = $state(emptySchema())
	const href = window.location.href
	const indexQ = href.indexOf('?')
	const searchParams = indexQ > -1 ? new URLSearchParams(href.substring(indexQ)) : undefined

	if (searchParams?.has('local')) {
		connectWs()
	}

	let useLock = $state(false)

	let lockChanges = false
	let timeout: NodeJS.Timeout | undefined = undefined

	let loadingCodebaseButton = $state(false)
	let lastCommandId = ''

	const el = (event) => {
		// sendUserToast(`Received message from parent ${event.data.type}`, true)
		if (event.data.type == 'runTest') {
			runTest()
			event.preventDefault()
		} else if (event.data.type == 'replaceScript') {
			replaceScript(event.data)
		} else if (event.data.type == 'testBundle') {
			if (event.data.id == lastCommandId) {
				testBundle(event.data.file, event.data.isTar)
			} else {
				sendUserToast(`Bundle received ${lastCommandId} was obsolete, ignoring`, true)
			}
		} else if (event.data.type == 'testPreviewBundle') {
			if (event.data.id == lastCommandId && currentScript) {
				testJobLoader?.runPreview(
					currentScript.path,
					event.data.file,
					currentScript.language,
					args,
					currentScript.tag,
					useLock ? currentScript.lock : undefined
				)
			} else {
				sendUserToast(`Bundle received ${lastCommandId} was obsolete, ignoring`, true)
			}
		} else if (event.data.type == 'testBundleError') {
			loadingCodebaseButton = false
			sendUserToast(
				'Error bundling script:' +
					event.data.errorMessage +
					' ' +
					(typeof event.data.error == 'object'
						? JSON.stringify(event.data.error)
						: event.data.error),
				true
			)
		} else if (event.data.type == 'replaceFlow') {
			lockChanges = true
			replaceFlow(event.data)
			timeout && clearTimeout(timeout)
			timeout = setTimeout(() => {
				lockChanges = false
			}, 500)
		} else if (event.data.type == 'error') {
			sendUserToast(event.data.error.message, true)
		}
	}

	setLicense()

	onMount(() => {
		window.addEventListener('message', el, false)
		document.addEventListener('keydown', (e) => {
			const obj = {
				altKey: e.altKey,
				code: e.code,
				ctrlKey: e.ctrlKey,
				isComposing: e.isComposing,
				key: e.key,
				location: e.location,
				metaKey: e.metaKey,
				repeat: e.repeat,
				shiftKey: e.shiftKey
			}

			if ((obj.ctrlKey || obj.metaKey) && obj.key == 'a') {
				e.stopPropagation()
				return
			}
			window.parent?.postMessage({ type: 'keydown', key: JSON.stringify(obj) }, '*')
		})
		window.parent?.postMessage({ type: 'refresh' }, '*')
	})

	async function testBundle(file: string, isTar: boolean) {
		testJobLoader?.abstractRun(async () => {
			try {
				const form = new FormData()
				form.append(
					'preview',
					JSON.stringify({
						content: currentScript?.content,
						kind: isTar ? 'tarbundle' : 'bundle',
						path: currentScript?.path,
						args,
						language: currentScript?.language,
						tag: currentScript?.tag
					})
				)
				// sendUserToast(JSON.stringify(file))
				if (isTar) {
					var array: number[] = []
					file = atob(file)
					for (var i = 0; i < file.length; i++) {
						array.push(file.charCodeAt(i))
					}
					let blob = new Blob([new Uint8Array(array)], { type: 'application/octet-stream' })

					form.append('file', blob)
				} else {
					form.append('file', file)
				}

				const url = '/api/w/' + workspace + '/jobs/run/preview_bundle'

				const req = await fetch(url, {
					method: 'POST',
					body: form,
					headers: {
						Authorization: 'Bearer ' + token
					}
				})
				if (req.status != 201) {
					throw Error(
						`Script snapshot creation was not successful: ${req.status} - ${
							req.statusText
						} - ${await req.text()}`
					)
				}
				return await req.text()
			} catch (e) {
				sendUserToast(`Failed to send bundle ${e}`, true)
				throw Error(e)
			}
		})
		loadingCodebaseButton = false
	}
	onDestroy(() => {
		window.removeEventListener('message', el)
		if (socket && socket.readyState === WebSocket.OPEN) {
			socket?.close()
		}
	})

	function connectWs() {
		try {
			if (socket) {
				socket.close()
			}
		} catch (e) {
			console.error('Failed to close websocket', e)
		}
		const port = searchParams?.get('port') || '3001'
		try {
			socket = new WebSocket(`ws://localhost:${port}/ws`)

			// Listen for messages
			socket.addEventListener('message', (event) => {
				replaceData(event.data)
			})

			function replaceData(msg: string) {
				let data: any | undefined = undefined
				try {
					data = JSON.parse(msg)
				} catch {
					console.log('Received invalid JSON: ' + msg)
					return
				}
				if (data.type == 'script') {
					replaceScript(data)
				} else if (data.type == 'flow') {
					replaceFlow(data)
				} else {
					sendUserToast(`Received invalid message type ${data.type}`, true)
				}
			}
		} catch (e) {
			sendUserToast('Failed to connect to local server', true)
			console.error(e)
		}
	}

	let typescriptBundlePreviewMode = $state(false)
	function runTest() {
		if (mode == 'script') {
			if (!currentScript) {
				return
			}
			if (currentScript.isCodebase) {
				loadingCodebaseButton = true
				lastCommandId = Math.random().toString(36).substring(7)
				window.parent?.postMessage({ type: 'testBundle', id: lastCommandId }, '*')
			} else {
				if (relativePaths.length > 0 && typescriptBundlePreviewMode) {
					lastCommandId = Math.random().toString(36).substring(7)
					window.parent?.postMessage(
						{ type: 'testPreviewBundle', external: ['!/*'], id: lastCommandId },
						'*'
					)
				} else {
					//@ts-ignore
					testJobLoader.runPreview(
						currentScript.path,
						currentScript.content,
						currentScript.language,
						args,
						currentScript.tag,
						useLock ? currentScript.lock : undefined
					)
				}
			}
		} else {
			flowPreviewButtons?.openPreview()
		}
	}

	async function loadPastTests(): Promise<void> {
		if (!currentScript) {
			return
		}
		console.log('Loading past tests')
		pastPreviews = await JobService.listCompletedJobs({
			workspace: $workspaceStore!,
			jobKinds: 'preview',
			createdBy: $userStore?.username,
			scriptPathExact: currentScript.path
		})
	}

	async function onKeyDown(event: KeyboardEvent) {
		if ((event.ctrlKey || event.metaKey) && event.code === 'KeyC') {
			document.execCommand('copy')
		} else if ((event.ctrlKey || event.metaKey) && event.code === 'KeyX') {
			document.execCommand('cut')
		} else if ((event.ctrlKey || event.metaKey) && event.code === 'KeyV') {
			event.preventDefault()
			document.execCommand('paste')
		}
	}

	let relativePaths: any[] = $state([])
	let lastPath: string | undefined = undefined
	async function replaceScript(lastEdit: LastEditScript) {
		mode = 'script'
		currentScript = lastEdit
		if (lastPath !== lastEdit.path) {
			schema = emptySchema()
			relativePaths = []
		}
		try {
			await inferArgs(lastEdit.language, lastEdit.content, schema)
			if (lastEdit?.language == 'bun') {
				relativePaths = await parseTypescriptDeps(lastEdit.content).filter(isTypescriptRelativePath)
			} else if (lastEdit?.language == 'python3') {
				relativePaths = approximateFindPythonRelativePath(lastEdit.content)
			}
			schema = schema
			lastPath = lastEdit.path
			validCode = true
		} catch (e) {
			console.error(e)
			validCode = false
		}
	}

	let mode: 'script' | 'flow' = $state('script')

	const flowStore = $state({
		val: {
			summary: '',
			value: { modules: [] },
			extra_perms: {},
			schema: emptySchema()
		} as OpenFlow
	})

	type LastEditFlow = {
		flow: OpenFlow
		uriPath: string
	}
	let lastUriPath: string | undefined = undefined
	async function replaceFlow(lastEdit: LastEditFlow) {
		mode = 'flow'
		lastUriPath = lastEdit.uriPath
		// sendUserToast(JSON.stringify(lastEdit.flow), true)
		// return
		try {
			if (!deepEqual(lastEdit.flow, flowStore)) {
				if (!lastEdit.flow.summary) {
					lastEdit.flow.summary = 'New flow'
				}
				if (!lastEdit.flow.value?.modules) {
					lastEdit.flow.value = { modules: [] }
				}
				flowStore.val = lastEdit.flow
				inferModuleArgs($selectedIdStore)
			}
		} catch (e) {
			console.error('issue setting new flowstore', e)
		}
	}

	const flowStateStore = writable({} as FlowState)

	const previewArgsStore = $state({ val: {} })
	const scriptEditorDrawer = writable(undefined)
	const moving = writable<{ id: string } | undefined>(undefined)
	const history = initHistory(flowStore.val)

	const testStepStore = writable<Record<string, any>>({})
	const selectedIdStore = writable('settings-metadata')

	const triggersCount = writable<TriggersCount | undefined>(undefined)
	setContext<TriggerContext>('TriggerContext', {
		triggersCount: triggersCount,
		simplifiedPoll: writable(false),
		showCaptureHint: writable(undefined),
		triggersState: new Triggers()
	})
	setContext<FlowEditorContext>('FlowEditorContext', {
		selectedId: selectedIdStore,
		previewArgs: previewArgsStore,
		scriptEditorDrawer,
		moving,
		history,
		pathStore: writable(''),
		flowStateStore,
		flowStore,
		testStepStore,
		saveDraft: () => {},
		initialPathStore: writable(''),
		fakeInitialPath: '',
		flowInputsStore: writable<FlowInput>({}),
		customUi: {},
		insertButtonOpen: writable(false),
		executionCount: writable(0),
		flowInputEditorState: writable<FlowInputEditorState>({
			selectedTab: undefined,
			editPanelSize: undefined,
			payloadData: undefined
		}),
		currentEditor: writable(undefined)
	})
	setContext<PropPickerContext>('PropPickerContext', {
		flowPropPickerConfig: writable<FlowPropPickerConfig | undefined>(undefined),
		pickablePropertiesFiltered: writable<PickableProperties | undefined>(undefined)
	})

	let lastSent: OpenFlow | undefined = undefined
	function updateFlow(flow: OpenFlow) {
		if (lockChanges) {
			return
		}
		if (!deepEqual(flow, lastSent)) {
			lastSent = JSON.parse(JSON.stringify(flow))
			window?.parent.postMessage({ type: 'flow', flow, uriPath: lastUriPath }, '*')
		}
	}

	let flowPreviewButtons: FlowPreviewButtons | undefined = $state()
	let reload = $state(0)

	async function inferModuleArgs(selectedIdStore: string) {
		if (selectedIdStore == '') {
			return
		}
		//@ts-ignore
		dfs(flowStore.value.modules, async (mod) => {
			if (mod.id == selectedIdStore) {
				if (
					mod.value.type == 'rawscript' ||
					mod.value.type === 'script' ||
					mod.value.type === 'flow'
				) {
					const { input_transforms, schema } = await loadSchemaFromModule(mod)
					if (mod.value.type == 'rawscript' && mod.value.lock != undefined) {
						mod.value.lock = undefined
					}

					mod.value.input_transforms = input_transforms
					if (!deepEqual(schema, $flowStateStore[mod.id]?.schema)) {
						if (!$flowStateStore[mod.id]) {
							$flowStateStore[mod.id] = { schema }
						} else {
							$flowStateStore[mod.id].schema = schema
						}
						reload++
					}
				}
			}
		})
	}
	let token = $derived($page.url.searchParams.get('wm_token') ?? undefined)
	let workspace = $derived($page.url.searchParams.get('workspace') ?? undefined)
	let themeDarkRaw = $derived($page.url.searchParams.get('activeColorTheme'))
	let themeDark = $derived(themeDarkRaw == '2' || themeDarkRaw == '4')
	$effect(() => {
		if (token) {
			OpenAPI.WITH_CREDENTIALS = true
			OpenAPI.TOKEN = token
			untrack(() => loadUser())
		}
	})
	$effect(() => {
		if (workspace) {
			$workspaceStore = workspace
			untrack(() => setupCopilotInfo())
		}
	})
	$effect(() => {
		if (workspace && token) {
			untrack(() => loadUser())
		}
	})
	$effect(() => {
		darkModeToggle &&
			themeDark != darkMode &&
			darkMode != undefined &&
			!modeInitialized &&
			untrack(() => initializeMode())
	})
	$effect(() => {
		flowStore.val && untrack(() => updateFlow(flowStore.val))
	})
	$effect(() => {
		$selectedIdStore && untrack(() => inferModuleArgs($selectedIdStore))
	})
</script>

<svelte:window onkeydown={onKeyDown} />

<TestJobLoader
	on:done={loadPastTests}
	bind:this={testJobLoader}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
/>

<main class="h-screen w-full">
	{#if mode == 'script'}
		<div class="flex flex-col min-h-full overflow-auto">
			<div class="absolute top-0 left-2">
				<DarkModeToggle bind:darkMode bind:this={darkModeToggle} forcedDarkMode={false} />
			</div>
			<div class="text-center w-full text-lg truncate py-1 text-primary">
				{currentScript?.path ?? 'Not editing a script'}
				<span class="text-2xs text-secondary">{currentScript?.language ?? ''}</span><span
					class="text-2xs text-secondary">{currentScript?.isCodebase ? ' (codebase)' : ''}</span
				>
			</div>

			<div class="absolute top-2 right-2 !text-tertiary text-xs">
				{#if $userStore != undefined}
					As {$userStore?.username} in {$workspaceStore}
				{:else}
					<span class="text-red-600">Unable to login</span>
				{/if}
			</div>

			{#if !validCode}
				<div class="text-center w-full text-lg truncate py-1 text-red-500">Invalid code</div>
			{/if}
			<div class="flex flex-row-reverse py-1">
				<Toggle
					size="xs"
					bind:checked={useLock}
					options={{ left: 'Infer lockfile', right: 'Use current lockfile' }}
				/>
			</div>
			{#if (currentScript?.language == 'bun' || currentScript?.language == 'python3') && currentScript?.content != undefined}
				{#if relativePaths.length > 0}
					<div class="flex flex-row-reverse py-1">
						{#if currentScript?.language == 'bun' && !currentScript?.isCodebase}
							<Toggle
								size="xs"
								bind:checked={typescriptBundlePreviewMode}
								options={{
									left: '',
									right: 'bundle relative paths for preview',
									rightTooltip:
										'(Beta) Instead of only sending the current file for preview and rely on already deployed code for the common logic, bundle all code that is imported in relative paths'
								}}
							/>
						{:else if currentScript?.language == 'python3'}
							<div class="text-xs text-yellow-500"
								>relative imports detected<Tooltip
									>Beware that when using relative imports, the code used in preview for those is
									the one that is already deployed. If you make update to the common logic, you will
									need to `wmill sync push` to see it reflected in the preview runs.</Tooltip
								></div
							>
						{/if}
					</div>
				{/if}
			{/if}
			<div class="flex justify-center pt-1">
				{#if testIsLoading}
					<Button on:click={testJobLoader?.cancelJob} btnClasses="w-full" color="red" size="xs">
						<WindmillIcon
							white={true}
							class="mr-2 text-white"
							height="20px"
							width="20px"
							spin="fast"
						/>
						Cancel
					</Button>
				{:else}
					<Button
						disabled={currentScript === undefined}
						color="dark"
						on:click={(e) => {
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
						{#if loadingCodebaseButton}
							Bundle is building...
						{:else if testIsLoading}
							Running
						{:else}
							Test
						{/if}
					</Button>
				{/if}
			</div>
			<Splitpanes horizontal class="h-full">
				<Pane size={33}>
					<div class="px-2">
						<div class="break-words relative font-sans">
							<SchemaForm compact {schema} bind:args bind:isValid />
						</div>
					</div>
				</Pane>
				<Pane size={67}>
					<LogPanel
						{workspace}
						lang={currentScript?.language}
						previewJob={testJob}
						{pastPreviews}
						previewIsLoading={testIsLoading}
					/>
				</Pane>
			</Splitpanes>
		</div>
	{:else}
		<!-- <div class="h-full w-full grid grid-cols-2"> -->
		<div class="h-full w-full">
			<div class="flex flex-col max-h-screen h-full relative">
				<div class="absolute top-0 left-2">
					<DarkModeToggle bind:darkMode bind:this={darkModeToggle} forcedDarkMode={false} />
					{#if $userStore}
						As {$userStore?.username} in {$workspaceStore}
					{:else}
						<span class="text-red-600">Unable to login on {$workspaceStore}</span>
					{/if}
				</div>

				<div class="flex justify-center pt-1 z-50 absolute right-2 top-2 gap-2">
					<FlowPreviewButtons bind:this={flowPreviewButtons} />
				</div>
				<Splitpanes horizontal class="h-full max-h-screen grow">
					<Pane size={67}>
						{#if flowStore.val?.value?.modules}
							<div id="flow-editor"></div>
							<FlowModuleSchemaMap
								disableAi
								disableTutorials
								smallErrorHandler={true}
								disableStaticInputs
							/>
						{:else}
							<div class="text-red-400 mt-20">Missing flow modules</div>
						{/if}
					</Pane>
					<Pane size={33}>
						{#key reload}
							<FlowEditorPanel
								enableAi
								noEditor
								on:applyArgs={(ev) => {
									if (ev.detail.kind === 'preprocessor') {
										$testStepStore['preprocessor'] = ev.detail.args ?? {}
										$selectedIdStore = 'preprocessor'
									} else {
										previewArgsStore.val = ev.detail.args ?? {}
										flowPreviewButtons?.openPreview()
									}
								}}
							/>
						{/key}
					</Pane>
				</Splitpanes>
			</div>
		</div>
	{/if}
</main>
