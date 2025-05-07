<script lang="ts">
	import { emptySchema, sendUserToast } from '$lib/utils'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { onDestroy, onMount, setContext } from 'svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import FlowPreviewButtons from '$lib/components/flows/header/FlowPreviewButtons.svelte'
	import type {
		FlowEditorContext,
		FlowInput,
		FlowInputEditorState
	} from '$lib/components/flows/types'
	import { writable } from 'svelte/store'
	import { OpenAPI, type FlowModule, type OpenFlow, type TriggersCount } from '$lib/gen'
	import { initHistory } from '$lib/history'
	import type { FlowState } from '$lib/components/flows/flowState'
	import FlowModuleSchemaMap from '$lib/components/flows/map/FlowModuleSchemaMap.svelte'
	import FlowEditorPanel from '$lib/components/flows/content/FlowEditorPanel.svelte'
	import { deepEqual } from 'fast-equals'
	import { page } from '$app/stores'
	import { userStore, workspaceStore } from '$lib/stores'
	import { getUserExt } from '$lib/user'
	import DarkModeToggle from '$lib/components/sidebar/DarkModeToggle.svelte'
	import type { ScheduleTrigger, TriggerContext } from '$lib/components/triggers'
	import type { FlowPropPickerConfig, PropPickerContext } from '$lib/components/prop_picker'
	import type { PickableProperties } from '$lib/components/flows/previousResults'

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

	const flowStore = writable({
		summary: '',
		value: { modules: [] },
		extra_perms: {},
		schema: emptySchema()
	} as OpenFlow)

	let initialCode = JSON.stringify($flowStore, null, 4)
	const flowStateStore = writable({} as FlowState)

	const previewArgsStore = writable<Record<string, any>>({})
	const scriptEditorDrawer = writable(undefined)
	const moving = writable<{ module: FlowModule; modules: FlowModule[] } | undefined>(undefined)
	const history = initHistory($flowStore)

	const testStepStore = writable<Record<string, any>>({})
	const selectedIdStore = writable('settings-metadata')
	const primaryScheduleStore = writable<ScheduleTrigger | undefined | false>(undefined)
	const triggersCount = writable<TriggersCount | undefined>(undefined)
	const selectedTriggerStore = writable<
		'webhooks' | 'emails' | 'schedules' | 'cli' | 'routes' | 'websockets' | 'scheduledPoll'
	>('webhooks')
	setContext<TriggerContext>('TriggerContext', {
		primarySchedule: primaryScheduleStore,
		selectedTrigger: selectedTriggerStore,
		triggersCount: triggersCount,
		simplifiedPoll: writable(false),
		defaultValues: writable(undefined),
		captureOn: writable(undefined),
		showCaptureHint: writable(undefined)
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
		})
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
		testJobLoader.runPreview(currentScript.path, currentScript.content, args, undefined)
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
	let editor: SimpleEditor

	$: updateCode(editor, $flowStore)

	function updateCode(editor: SimpleEditor, flow: OpenFlow) {
		if (editor && !deepEqual(flow, JSON.parse(editor.getCode()))) {
			editor.setCode(JSON.stringify(flow, null, 4))
		}
	}

	function updateFromCode(code: string) {
		try {
			if (!deepEqual(JSON.parse(code), $flowStore)) {
				$flowStore = JSON.parse(code)
			}
		} catch (e) {
			console.error('issue parsing new change:', code, e)
		}
	}

	let flowPreviewButtons: FlowPreviewButtons
</script>

<svelte:window on:keydown={onKeyDown} />

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
				<FlowPreviewButtons bind:this={flowPreviewButtons} />
			</div>
			<Splitpanes horizontal class="h-full max-h-screen grow">
				<Pane size={33}>
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
				<Pane size={67}>
					<FlowEditorPanel
						noEditor
						on:applyArgs={(ev) => {
							if (ev.detail.kind === 'preprocessor') {
								$testStepStore['preprocessor'] = ev.detail.args ?? {}
								$selectedIdStore = 'preprocessor'
							} else {
								$previewArgsStore = ev.detail.args ?? {}
								flowPreviewButtons?.openPreview()
							}
						}}
					/>
				</Pane>
			</Splitpanes>
		</div>
	</div>
</main>
