<script lang="ts">
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import { Button, Kbd } from '$lib/components/common'
	import { WindmillIcon } from '$lib/components/icons'
	import LogPanel from '$lib/components/scriptEditor/LogPanel.svelte'
	import {
		CompletedJob,
		Job,
		JobService,
		OpenAPI,
		Preview,
		type OpenFlow,
		type FlowModule
	} from '$lib/gen'
	import { inferArgs } from '$lib/infer'
	import { userStore, workspaceStore } from '$lib/stores'
	import { emptySchema, getModifierKey, sendUserToast } from '$lib/utils'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { onDestroy, onMount, setContext } from 'svelte'
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
	import type { FlowEditorContext } from './flows/types'
	import { dfs } from './flows/dfs'
	import { loadSchemaFromModule } from './flows/flowInfers'
	import { Play } from 'lucide-svelte'

	$: token = $page.url.searchParams.get('wm_token') ?? undefined
	$: workspace = $page.url.searchParams.get('workspace') ?? undefined
	$: themeDarkRaw = $page.url.searchParams.get('activeColorTheme')
	$: themeDark = themeDarkRaw == '2' || themeDarkRaw == '4'

	$: if (token) {
		OpenAPI.WITH_CREDENTIALS = true
		OpenAPI.TOKEN = $page.url.searchParams.get('wm_token')!
	}

	$: if (workspace) {
		$workspaceStore = workspace
	}

	$: if (workspace && token) {
		loadUser()
	}

	async function loadUser() {
		const user = await getUserExt(workspace!)
		$userStore = user
	}

	let darkModeToggle: DarkModeToggle
	let darkMode: boolean | undefined = undefined
	let modeInitialized = false
	function initializeMode() {
		modeInitialized = true
		darkModeToggle.toggle()
	}
	$: darkModeToggle &&
		themeDark != darkMode &&
		darkMode != undefined &&
		!modeInitialized &&
		initializeMode()

	let testJobLoader: TestJobLoader

	// Test args input
	let args: Record<string, any> = {}
	let isValid: boolean = true

	// Test
	let testIsLoading = false
	let testJob: Job | undefined
	let pastPreviews: CompletedJob[] = []
	let validCode = true

	type LastEditScript = {
		content: string
		path: string
		language: Preview.language
	}

	let currentScript: LastEditScript | undefined = undefined

	let schema = emptySchema()
	const href = window.location.href
	const indexQ = href.indexOf('?')
	const searchParams = indexQ > -1 ? new URLSearchParams(href.substring(indexQ)) : undefined

	if (searchParams?.has('local')) {
		connectWs()
	}

	let lockChanges = false
	let timeout: NodeJS.Timeout | undefined = undefined
	const el = (event) => {
		// sendUserToast(`Received message from parent ${event.data.type}`, true)
		if (event.data.type == 'runTest') {
			runTest()
		} else if (event.data.type == 'replaceScript') {
			mode = 'script'
			replaceScript(event.data)
		} else if (event.data.type == 'replaceFlow') {
			mode = 'flow'
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

			if (obj.ctrlKey && obj.key == 'a') {
				e.stopPropagation()
				return
			}
			window.parent?.postMessage({ type: 'keydown', key: JSON.stringify(obj) }, '*')
		})
		window.parent?.postMessage({ type: 'refresh' }, '*')
	})

	onDestroy(() => {
		window.removeEventListener('message', el)
		if (socket && socket.readyState === WebSocket.OPEN) {
			socket?.close()
		}
	})

	let socket: WebSocket | undefined = undefined
	function connectWs() {
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
				replaceScript(data)
			}
		} catch (e) {
			sendUserToast('Failed to connect to local server', true)
			console.error(e)
		}
	}

	function runTest() {
		if (mode == 'script') {
			if (!currentScript) {
				return
			}
			//@ts-ignore
			testJobLoader.runPreview(
				currentScript.path,
				currentScript.content,
				currentScript.language,
				args,
				undefined
			)
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

	function onKeyDown(event: KeyboardEvent) {
		if ((event.ctrlKey || event.metaKey) && event.key == 'Enter') {
			event.preventDefault()
			runTest()
		}
	}

	let lastPath: string | undefined = undefined
	async function replaceScript(lastEdit: LastEditScript) {
		currentScript = lastEdit
		if (lastPath !== lastEdit.path) {
			schema = emptySchema()
		}
		try {
			await inferArgs(lastEdit.language, lastEdit.content, schema)
			schema = schema
			lastPath = lastEdit.path
			validCode = true
		} catch (e) {
			console.error(e)
			validCode = false
		}
	}

	let mode: 'script' | 'flow' = 'script'

	const flowStore = writable({
		summary: '',
		value: { modules: [] },
		extra_perms: {},
		schema: emptySchema()
	} as OpenFlow)

	type LastEditFlow = {
		flow: OpenFlow
		uriPath: string
	}
	let lastUriPath: string | undefined = undefined
	async function replaceFlow(lastEdit: LastEditFlow) {
		lastUriPath = lastEdit.uriPath
		// sendUserToast(JSON.stringify(lastEdit.flow), true)
		// return
		try {
			if (!deepEqual(lastEdit.flow, $flowStore)) {
				if (!lastEdit.flow.summary) {
					lastEdit.flow.summary = 'New flow'
				}
				if (!lastEdit.flow.value?.modules) {
					lastEdit.flow.value = { modules: [] }
				}
				$flowStore = lastEdit.flow
				inferModuleArgs($selectedIdStore)
			}
		} catch (e) {
			console.error('issue setting new flowstore', e)
		}
	}

	const flowStateStore = writable({} as FlowState)
	const scheduleStore = writable({
		args: {},
		cron: '',
		timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
		enabled: false
	})
	const previewArgsStore = writable<Record<string, any>>({})
	const scriptEditorDrawer = writable(undefined)
	const moving = writable<{ module: FlowModule; modules: FlowModule[] } | undefined>(undefined)
	const history = initHistory($flowStore)

	const testStepStore = writable<Record<string, any>>({})
	const selectedIdStore = writable('settings-metadata')

	setContext<FlowEditorContext>('FlowEditorContext', {
		selectedId: selectedIdStore,
		schedule: scheduleStore,
		previewArgs: previewArgsStore,
		scriptEditorDrawer,
		moving,
		history,
		pathStore: writable(''),
		flowStateStore,
		flowStore,
		testStepStore,
		saveDraft: () => {},
		initialPath: ''
	})

	$: updateCode($flowStore)

	let lastSent: OpenFlow | undefined = undefined
	function updateCode(flow: OpenFlow) {
		if (lockChanges) {
			return
		}
		if (!deepEqual(flow, lastSent)) {
			lastSent = JSON.parse(JSON.stringify(flow))
			window?.parent.postMessage({ type: 'flow', flow, uriPath: lastUriPath }, '*')
		}
	}

	$: $selectedIdStore && inferModuleArgs($selectedIdStore)

	let flowPreviewButtons: FlowPreviewButtons
	let reload = 0

	async function inferModuleArgs(selectedIdStore: string) {
		if (selectedIdStore == '') {
			return
		}
		dfs($flowStore.value.modules, async (mod) => {
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
</script>

<svelte:window on:keydown={onKeyDown} />

<TestJobLoader
	on:done={loadPastTests}
	bind:this={testJobLoader}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
/>

<main class="h-screen w-full">
	{#if mode == 'script'}
		<div class="flex flex-col h-full">
			<div class="absolute top-0 left-2">
				<DarkModeToggle bind:darkMode bind:this={darkModeToggle} forcedDarkMode={false} />
			</div>
			<div class="text-center w-full text-lg truncate py-1 text-primary">
				{currentScript?.path ?? 'Not editing a script'}
				{currentScript?.language ?? ''}
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
						on:click={() => {
							runTest()
						}}
						btnClasses="w-full"
						size="xs"
						startIcon={{
							icon: Play,
							classes: 'animate-none'
						}}
					>
						{#if testIsLoading}
							Running
						{:else}
							Test&nbsp;<Kbd small isModifier>{getModifierKey()}</Kbd>
							<Kbd small><span class="text-lg font-bold">⏎</span></Kbd>
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
						{#if $flowStore?.value?.modules}
							<FlowModuleSchemaMap
								bind:modules={$flowStore.value.modules}
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
							<FlowEditorPanel noEditor />
						{/key}
					</Pane>
				</Splitpanes>
			</div>
		</div>
	{/if}
</main>
