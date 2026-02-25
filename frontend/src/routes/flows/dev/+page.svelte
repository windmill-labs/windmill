<script lang="ts">
	import { emptySchema, sendUserToast, type StateStore } from '$lib/utils'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { onDestroy, onMount, setContext, untrack } from 'svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import FlowPreviewButtons from '$lib/components/flows/header/FlowPreviewButtons.svelte'
	import type {
		FlowEditorContext,
		FlowInput,
		FlowInputEditorState
	} from '$lib/components/flows/types'
	import { SelectionManager } from '$lib/components/graph/selectionUtils.svelte'
	import { writable } from 'svelte/store'
	import { OpenAPI, type Job, type OpenFlow, type TriggersCount } from '$lib/gen'
	import { initHistory } from '$lib/history.svelte'
	import type { FlowState } from '$lib/components/flows/flowState'
	import FlowModuleSchemaMap from '$lib/components/flows/map/FlowModuleSchemaMap.svelte'
	import FlowEditorPanel from '$lib/components/flows/content/FlowEditorPanel.svelte'
	import { deepEqual } from 'fast-equals'
	import { page } from '$app/stores'
	import { userStore, workspaceStore } from '$lib/stores'
	import { getUserExt } from '$lib/user'
	import DarkModeToggle from '$lib/components/sidebar/DarkModeToggle.svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import type { FlowPropPickerConfig, PropPickerContext } from '$lib/components/prop_picker'
	import type { PickableProperties } from '$lib/components/flows/previousResults'
	import { Triggers } from '$lib/components/triggers/triggers.svelte'
	import { StepsInputArgs } from '$lib/components/flows/stepsInputArgs.svelte'
	import { ModulesTestStates } from '$lib/components/modulesTest.svelte'

	let token = $page.url.searchParams.get('wm_token') ?? undefined
	let workspace = $page.url.searchParams.get('workspace') ?? undefined
	let themeDarkRaw = $page.url.searchParams.get('activeColorTheme')
	let themeDark = themeDarkRaw == '2' || themeDarkRaw == '4'

	if (token) {
		OpenAPI.WITH_CREDENTIALS = true
		OpenAPI.TOKEN = $page.url.searchParams.get('wm_token')!
	}

	if (workspace) {
		$workspaceStore = workspace
	}

	if (workspace && token) {
		loadUser()
	}

	async function loadUser() {
		const user = await getUserExt(workspace!)
		userStore.set(user)
	}

	let darkModeToggle: DarkModeToggle | undefined = $state()
	let darkMode: boolean | undefined = $state(undefined)
	let modeInitialized = $state(false)
	function initializeMode() {
		modeInitialized = true
		darkModeToggle?.toggle()
	}

	const flowStore = $state({
		val: {
			summary: '',
			value: { modules: [] },
			extra_perms: {},
			schema: emptySchema()
		} as OpenFlow
	})

	let initialCode = JSON.stringify(flowStore, null, 4)
	const flowStateStore = $state({ val: {} }) as StateStore<FlowState>
	const suspendStatus = $state({ val: {} }) as StateStore<Record<string, { job: Job; nb: number }>>

	const previewArgsStore = $state({ val: {} })
	const scriptEditorDrawer = writable(undefined)
	const moving = writable<{ id: string } | undefined>(undefined)
	const history = initHistory(flowStore.val)

	const stepsInputArgs = new StepsInputArgs()
	const selectionManager = new SelectionManager()
	selectionManager.selectId('settings-metadata')
	const triggersCount = writable<TriggersCount | undefined>(undefined)
	setContext<TriggerContext>('TriggerContext', {
		triggersCount: triggersCount,
		simplifiedPoll: writable(false),
		showCaptureHint: writable(undefined),
		triggersState: new Triggers()
	})

	setContext<FlowEditorContext>('FlowEditorContext', {
		selectionManager,
		previewArgs: previewArgsStore,
		scriptEditorDrawer,
		flowEditorDrawer: writable(undefined),
		moving,
		history,
		pathStore: writable(''),
		flowStateStore,
		flowStore,
		stepsInputArgs,
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
		currentEditor: writable(undefined),
		modulesTestStates: new ModulesTestStates(),
		outputPickerOpenFns: {},
		preserveOnBehalfOf: writable(false),
		savedOnBehalfOfEmail: writable<string | undefined>(undefined)
	})
	setContext<PropPickerContext>('PropPickerContext', {
		flowPropPickerConfig: writable<FlowPropPickerConfig | undefined>(undefined),
		pickablePropertiesFiltered: writable<PickableProperties | undefined>(undefined)
	})
	type LastEdit = {
		content: string
		path: string
	}

	let currentScript: LastEdit | undefined = undefined

	const href = window.location.href
	const indexQ = href.indexOf('?')
	const searchParams = indexQ > -1 ? new URLSearchParams(href.substring(indexQ)) : undefined

	if (searchParams?.has('local')) {
		connectWs()
	}

	const el = (event) => {
		if (event.data.type == 'runTest') {
			runTest()
		} else if (event.data.type == 'replaceScript') {
			replaceScript(event.data)
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
			window.parent?.postMessage(JSON.stringify(obj), '*')
		})
	})

	onDestroy(() => {
		window.removeEventListener('message', el)
	})

	function connectWs() {
		const port = searchParams?.get('port') || '3001'
		try {
			const socket = new WebSocket(`ws://localhost:${port}/ws`)

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
		if (!currentScript) {
			return
		}
		//@ts-ignore
		jobLoader.runPreview(currentScript.path, currentScript.content, args, undefined)
	}

	function onKeyDown(event: KeyboardEvent) {
		if ((event.ctrlKey || event.metaKey) && event.key == 'Enter') {
			event.preventDefault()
			runTest()
		}
	}

	let lastPath: string | undefined = undefined
	async function replaceScript(lastEdit: LastEdit) {
		currentScript = lastEdit
		if (lastPath !== lastEdit.path) {
			// schema = emptySchema()
		}
		// try {
		// 	await inferArgs(lastEdit.language, lastEdit.content, schema)
		// 	schema = schema
		// 	lastPath = lastEdit.path
		// 	validCode = true
		// } catch (e) {
		// 	console.error(e)
		// 	validCode = false
		// }
	}
	let editor: SimpleEditor | undefined = $state()

	function updateCode(editor: SimpleEditor | undefined, flow: OpenFlow) {
		if (editor && !deepEqual(flow, JSON.parse(editor.getCode()))) {
			editor.setCode(JSON.stringify(flow, null, 4))
		}
	}

	function updateFromCode(code: string) {
		try {
			if (!deepEqual(JSON.parse(code), flowStore)) {
				flowStore.val = JSON.parse(code)
			}
		} catch (e) {
			console.error('issue parsing new change:', code, e)
		}
	}

	let flowPreviewButtons: FlowPreviewButtons | undefined = $state()
	$effect(() => {
		darkModeToggle &&
			themeDark != darkMode &&
			darkMode != undefined &&
			!modeInitialized &&
			untrack(() => initializeMode())
	})
	$effect(() => {
		const args = [editor, flowStore.val] as const
		untrack(() => updateCode(...args))
	})
</script>

<svelte:window onkeydown={onKeyDown} />

<main class="h-screen w-full">
	<div class="h-full w-full grid grid-cols-2">
		<SimpleEditor
			bind:this={editor}
			code={initialCode}
			lang="json"
			on:change={(e) => {
				updateFromCode(e.detail.code)
			}}
		/>
		<div class="flex flex-col max-h-screen h-full relative">
			<div class="absolute top-0 left-2">
				<DarkModeToggle bind:darkMode bind:this={darkModeToggle} forcedDarkMode={false} />
				{#if $userStore}
					As {$userStore?.username} in {$workspaceStore}
				{:else}
					<span class="text-red-600">Unable to login</span>
				{/if}
			</div>

			<div class="flex justify-center pt-1 z-50 absolute right-2 top-2 gap-2">
				<FlowPreviewButtons bind:this={flowPreviewButtons} {suspendStatus} />
			</div>
			<Splitpanes horizontal class="h-full max-h-screen grow">
				<Pane size={33}>
					{#if flowStore.val?.value?.modules}
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
				<Pane size={67}>
					<FlowEditorPanel
						noEditor
						on:applyArgs={(ev) => {
							if (ev.detail.kind === 'preprocessor') {
								stepsInputArgs.setStepArgs('preprocessor', ev.detail.args ?? {})
								selectionManager.selectId('preprocessor')
							} else {
								previewArgsStore.val = ev.detail.args ?? {}
								flowPreviewButtons?.openPreview()
							}
						}}
					/>
				</Pane>
			</Splitpanes>
		</div>
	</div>
</main>
