<script lang="ts">
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import JsonInputs from '$lib/components/JsonInputs.svelte'
	import JobLoader from '$lib/components/JobLoader.svelte'
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
	import { userStore, workspaceStore } from '$lib/stores'
	import {
		emptySchema,
		pluralize,
		readFieldsRecursively,
		sendUserToast,
		type StateStore
	} from '$lib/utils'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { onDestroy, onMount, setContext, untrack } from 'svelte'
	import DarkModeToggle from '$lib/components/sidebar/DarkModeToggle.svelte'
	import { page } from '$app/state'
	import { getUserExt } from '$lib/user'
	import FlowPreviewButtons from './flows/header/FlowPreviewButtons.svelte'
	import FlowModuleSchemaMap from './flows/map/FlowModuleSchemaMap.svelte'
	import FlowEditorPanel from './flows/content/FlowEditorPanel.svelte'
	import { deepEqual } from 'fast-equals'
	import { findModuleInFlow } from './flows/flowDiff'
	import { writable } from 'svelte/store'
	import type { FlowState } from './flows/flowState'
	import { initHistory } from '$lib/history.svelte'
	import type { FlowEditorContext, FlowInput, FlowInputEditorState } from './flows/types'
	import { SelectionManager } from './graph/selectionUtils.svelte'
	import { NoteEditor, setNoteEditorContext } from './graph/noteEditor.svelte'
	import { GroupEditor, setGroupEditorContext } from './graph/groupEditor.svelte'
	import { dfs } from './flows/dfs'
	import { loadSchemaFromModule } from './flows/flowInfers'
	import {
		CornerDownLeft,
		Play,
		Folder,
		FolderTree,
		User,
		Search,
		ChevronDown,
		ChevronUp,
		Code2,
		LayoutDashboard
	} from 'lucide-svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import FlowIcon from '$lib/components/home/FlowIcon.svelte'
	import {
		groupItems,
		type ItemType,
		type FolderItem,
		type UserItem
	} from '$lib/components/home/treeViewUtils'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Row from '$lib/components/common/table/Row.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { HOME_SEARCH_PLACEHOLDER } from '$lib/consts'
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
	import { StepsInputArgs } from './flows/stepsInputArgs.svelte'
	import { ModulesTestStates } from './modulesTest.svelte'
	import type { GraphModuleState } from './graph'
	import { setCopilotInfo } from '$lib/aiStore'

	let {
		initial = undefined
	}: {
		initial?:
			| {
					type: 'script'
					script: LastEditScript
			  }
			| {
					type: 'flow'
					flow: LastEditFlow
			  }
			| undefined
	} = $props()

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
	let darkMode: boolean = $state(document.documentElement.classList.contains('dark'))
	let flowContainerWidth = $state(0)
	let flowContainerHeight = $state(0)
	let flowHorizontalSplit = $derived(flowContainerWidth < flowContainerHeight)
	let modeInitialized = $state(false)
	let paneWidth = $state(0)
	let compactPreview = $derived(paneWidth < 800)
	function initializeMode() {
		modeInitialized = true
		darkModeToggle?.toggle()
	}

	let jobLoader: JobLoader | undefined = $state()
	let socket: WebSocket | undefined = undefined

	// Test args input
	let args: Record<string, any> = $state({})
	let isValid: boolean = $state(true)
	let jsonView: boolean = $state(false)
	let jsonEditor: JsonInputs | undefined = $state(undefined)
	let schemaHeight = $state(0)

	// Test
	let testIsLoading = $state(false)
	let testJob: Job | undefined = $state()
	let pastPreviews: CompletedJob[] = $state([])
	let validCode = $state(true)

	// Flow preview
	let flowPreviewButtons: FlowPreviewButtons | undefined = $state()
	const flowPreviewContent = $derived(flowPreviewButtons?.getFlowPreviewContent())
	const job: Job | undefined = $derived(flowPreviewContent?.getJob())
	let showJobStatus = $state(false)

	type LastEditScript = {
		content: string
		path: string
		language: Preview['language']
		lock?: string
		isCodebase?: boolean
		tag?: string
		modules?: { [key: string]: import('$lib/gen').ScriptModule } | null
	}

	let currentScript: LastEditScript | undefined = $state(undefined)
	let mode: 'script' | 'flow' = $state('script')
	let lastPath: string | undefined = undefined

	let schema = $state(emptySchema())
	const href = window.location.href
	const indexQ = href.indexOf('?')
	const searchParams = indexQ > -1 ? new URLSearchParams(href.substring(indexQ)) : undefined
	let relativePaths: (string | [number, string])[] = $state([])

	type WmPathItem = {
		path: string
		kind: 'flow' | 'script' | 'raw_app'
		summary?: string
	}
	// watchPath is (re)synced on initial load, on popstate, and on explicit
	// pickPath assignments. We don't listen for generic pushState events —
	// the only pushState callsite is pickPath itself, and it updates watchPath
	// directly. If a third caller starts pushing to history, add a resync there.
	function parseWatchPath(): string | undefined {
		const i = window.location.href.indexOf('?')
		if (i < 0) return undefined
		return new URLSearchParams(window.location.href.substring(i)).get('path') ?? undefined
	}
	const PATH_SUFFIX_RE = /(\.(flow|app|raw_app)|__(flow|app|raw_app))\/?$/
	let watchPath = $state(parseWatchPath()?.replace(PATH_SUFFIX_RE, ''))
	let pickerItems: WmPathItem[] = $state([])
	// Picker only makes sense on the local dev page — that's the only context
	// with a wmill dev WebSocket capable of returning the workspace listing.
	// The VS Code extension iframe omits ?local=true and drives the page via
	// postMessage (replaceScript / replaceFlow), so it must skip the picker.
	const isLocalDevPage = !!searchParams?.has('local')
	const pickerMode = $derived(isLocalDevPage && !watchPath)
	let wsState: 'connecting' | 'open' | 'closed' = $state('connecting')
	let pickerFilter = $state('')
	let pickerKind: 'all' | 'flow' | 'script' | 'raw_app' = $state('all')
	// Shape pickerItems into the homepage's ItemType so we can reuse `groupItems`
	// for the folder/user tree structure. `kind` ('script'|'flow'|'raw_app') maps 1:1
	// onto ItemType['type']; missing fields (canWrite, edited_at, etc.) default to safe values.
	const pickerTreeItems = $derived(
		pickerItems.map(
			(item) =>
				({
					path: item.path,
					summary: item.summary ?? '',
					type: item.kind,
					canWrite: true,
					extra_perms: {},
					starred: false,
					edited_at: ''
				}) as unknown as ItemType
		)
	)
	const pickerKindFilteredItems = $derived(
		pickerKind === 'all' ? pickerTreeItems : pickerTreeItems.filter((i) => i.type === pickerKind)
	)
	let pickerFilteredItems: (ItemType & { marked?: string })[] | undefined = $state(undefined)
	const pickerGroups = $derived(groupItems(pickerFilteredItems ?? pickerKindFilteredItems))

	if (searchParams?.has('local')) {
		connectWs()
	}

	let useLock = $state(false)

	let lockChanges = false
	let timeout: number | undefined = undefined

	let loadingCodebaseButton = $state(false)
	let lastCommandId = ''

	const untrackedInitial = untrack(() => initial)
	if (untrackedInitial) {
		if (untrackedInitial.type == 'script') {
			replaceScript(untrackedInitial.script)
		} else if (untrackedInitial.type == 'flow') {
			replaceFlow(untrackedInitial.flow)
		}
		modeInitialized = true
	}

	const el = (event) => {
		// sendUserToast(`Received message from parent ${event.data.type}`, true)
		if (event.data.type == 'runTest') {
			runTest()
			event.preventDefault()
		} else if (event.data.type == 'replaceScript') {
			replaceScript(event.data)
		} else if (event.data.type == 'testBundle') {
			if (event.data.id == lastCommandId) {
				testBundle(event.data.file, event.data.isTar, event.data.format)
			} else {
				sendUserToast(`Bundle received ${lastCommandId} was obsolete, ignoring`, true)
			}
		} else if (event.data.type == 'testPreviewBundle') {
			if (event.data.id == lastCommandId && currentScript) {
				jobLoader?.runPreview(
					currentScript.path,
					event.data.file,
					currentScript.language,
					args,
					currentScript.tag,
					useLock ? currentScript.lock : undefined,
					undefined,
					{
						done(x) {
							loadPastTests()
						}
					},
					undefined,
					currentScript.modules
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
			timeout = window.setTimeout(() => {
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

	async function testBundle(file: string, isTar: boolean, format: 'cjs' | 'esm' | undefined) {
		jobLoader?.abstractRun(
			async () => {
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
							tag: currentScript?.tag,
							format
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
			},
			{
				done(x) {
					loadPastTests()
				}
			}
		)
		loadingCodebaseButton = false
	}
	const onPopState = () => {
		watchPath = parseWatchPath()?.replace(PATH_SUFFIX_RE, '')
		if (watchPath && socket && socket.readyState === WebSocket.OPEN) {
			socket.send(JSON.stringify({ type: 'loadWmPath', path: watchPath }))
		}
	}

	onMount(() => {
		window.addEventListener('popstate', onPopState)
	})

	onDestroy(() => {
		window.removeEventListener('message', el)
		window.removeEventListener('popstate', onPopState)
		if (socket && socket.readyState === WebSocket.OPEN) {
			socket?.close()
		}
	})

	function pickPath(item: WmPathItem) {
		if (item.kind === 'raw_app') {
			sendUserToast(
				`raw_apps aren't previewable here. Run \`wmill app dev\` from inside the app folder.`,
				false
			)
			return
		}
		const url = new URL(window.location.href)
		url.searchParams.set('path', item.path)
		window.history.pushState({}, '', url.toString())
		watchPath = item.path
		socket?.send(JSON.stringify({ type: 'loadWmPath', path: item.path }))
	}

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
			wsState = 'connecting'
			socket = new WebSocket(`ws://localhost:${port}/ws`)

			// On connect, request the watched path if any, otherwise ask for a list to render the picker
			socket.addEventListener('open', () => {
				if (!socket) return
				wsState = 'open'
				if (watchPath) {
					socket.send(JSON.stringify({ type: 'loadWmPath', path: watchPath }))
				} else {
					socket.send(JSON.stringify({ type: 'listPaths' }))
				}
			})

			socket.addEventListener('error', () => {
				wsState = 'closed'
			})

			socket.addEventListener('close', () => {
				wsState = 'closed'
			})

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
				if (data.type === 'paths') {
					pickerItems = data.items ?? []
					return
				}
				// Picker mode (URL has no path) — ignore live broadcasts so a random
				// file change doesn't yank the page out of the picker. (When watchPath
				// IS set the server gates broadcasts itself, so no further filter here.)
				if (!watchPath) return
				if (data.type == 'script' || data.type == 'flow') {
					// Guard against the $effect on flowStore.val (re)serializing the
					// just-received payload back over the same WS to handleFlowRoundTrip
					// (which would re-run the orphan-file scan with the same content).
					// Mirrors the postMessage handler above.
					lockChanges = true
					if (data.type == 'script') {
						replaceScript(data)
					} else {
						replaceFlow(data)
					}
					timeout && clearTimeout(timeout)
					timeout = window.setTimeout(() => {
						lockChanges = false
					}, 500)
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
					jobLoader.runPreview(
						currentScript.path,
						currentScript.content,
						currentScript.language,
						args,
						currentScript.tag,
						useLock ? currentScript.lock : undefined,
						undefined,
						undefined,
						undefined,
						currentScript.modules
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
		path: string
	}
	let lastUriPath: string | undefined = undefined
	async function replaceFlow(lastEdit: LastEditFlow) {
		mode = 'flow'
		lastUriPath = lastEdit.uriPath
		pathStore.set(lastEdit.path)
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
				try {
					let ids = dfs(flowStore.val.value.modules ?? [], (m) => m.id)
					flowStateStore.val = Object.fromEntries(ids.map((k) => [k, {}]))
				} catch (e) {}
				inferModuleArgs(selectedId)
			}
		} catch (e) {
			console.error('issue setting new flowstore', e)
		}
	}

	const flowStateStore = $state({ val: {} }) as StateStore<FlowState>

	const previewArgsStore = $state({ val: {} })
	const scriptEditorDrawer = writable(undefined)
	const history = initHistory(flowStore.val)
	const stepsInputArgs = new StepsInputArgs()
	const selectionManager = new SelectionManager()
	selectionManager.selectId('settings-metadata')
	const triggersCount = writable<TriggersCount | undefined>(undefined)
	const modulesTestStates = new ModulesTestStates((moduleId) => {
		// console.log('FOO')
		// Update the derived store with test job states
		// showJobStatus = false
	})
	const outputPickerOpenFns: Record<string, () => void> = $state({})

	setContext<TriggerContext>('TriggerContext', {
		triggersCount: triggersCount,
		simplifiedPoll: writable(false),
		showCaptureHint: writable(undefined),
		triggersState: new Triggers()
	})

	let pathStore = writable('')
	let initialPathStore = writable('')
	setContext<FlowEditorContext>('FlowEditorContext', {
		selectionManager,
		previewArgs: previewArgsStore,
		scriptEditorDrawer,
		flowEditorDrawer: writable(undefined),
		history,
		pathStore: pathStore,
		flowStateStore,
		flowStore,
		stepsInputArgs,
		saveDraft: () => {},
		initialPathStore,
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
		modulesTestStates,
		outputPickerOpenFns,
		preserveOnBehalfOf: writable(false),
		savedOnBehalfOfEmail: writable<string | undefined>(undefined)
	})
	setContext<PropPickerContext>('PropPickerContext', {
		flowPropPickerConfig: writable<FlowPropPickerConfig | undefined>(undefined),
		pickablePropertiesFiltered: writable<PickableProperties | undefined>(undefined)
	})

	// Set up NoteEditor context for note editing capabilities
	const noteEditor = new NoteEditor(flowStore, () => {
		// Enable notes display when a note is created
		flowModuleSchemaMap?.enableNotes?.()
	})
	setNoteEditorContext(noteEditor)

	// Set up GroupEditor context for group editing capabilities
	const groupEditor = new GroupEditor(flowStore)
	let canCreateGroup = $state({ val: false })
	setGroupEditorContext(groupEditor, canCreateGroup)

	let lastSent: OpenFlow | undefined = undefined
	const isInIframe = window.parent !== window
	function updateFlow(flow: OpenFlow) {
		if (lockChanges) {
			return
		}
		if (deepEqual(flow, lastSent)) {
			return
		}
		const snapshot = $state.snapshot(flow)
		// Prefer the WebSocket whenever a `wmill dev` session is connected — this covers
		// both standalone browser tabs and Claude Code's iframe preview. The VS Code
		// extension never opens this socket (its iframe URL omits `local=true`), so it
		// falls through to the postMessage path it has always used.
		if (socket && socket.readyState === WebSocket.OPEN) {
			socket.send(JSON.stringify({ type: 'flow', flow: snapshot, uriPath: lastUriPath }))
			lastSent = snapshot
		} else if (isInIframe) {
			window?.parent.postMessage({ type: 'flow', flow: snapshot, uriPath: lastUriPath }, '*')
			lastSent = snapshot
		}
		// Else: no channel available yet (WS still connecting, not in an iframe).
		// Don't mark `lastSent` so the next change will retry instead of being silently swallowed.
	}

	let reload = $state(0)

	async function inferModuleArgs(selectedIdStore: string) {
		if (selectedIdStore == '') {
			return
		}
		//@ts-ignore
		dfs(flowStore.val.value.modules, async (mod) => {
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
					if (!deepEqual(schema, flowStateStore.val[mod.id]?.schema)) {
						if (!flowStateStore.val[mod.id]) {
							flowStateStore.val[mod.id] = { schema }
						} else {
							flowStateStore.val[mod.id].schema = schema
						}
						reload++
					}
				}
			}
		})
	}
	let token = $derived(page.url.searchParams.get('wm_token') ?? undefined)
	let workspace = $derived(page.url.searchParams.get('workspace') ?? undefined)
	let themeDarkRaw = $derived(page.url.searchParams.get('activeColorTheme'))
	let themeDark = $derived(themeDarkRaw == '2' || themeDarkRaw == '4')

	$effect.pre(() => {
		setContext<{ token?: string }>('AuthToken', { token })
	})
	$effect.pre(() => {
		if (token) {
			OpenAPI.WITH_CREDENTIALS = true
			OpenAPI.TOKEN = token
		}
	})
	$effect.pre(() => {
		if (workspace) {
			$workspaceStore = workspace
		}
	})
	$effect.pre(() => {
		if (workspace && token) {
			untrack(() => loadUser())
			untrack(() => setupCopilotInfo())
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
		readFieldsRecursively(flowStore.val)
		flowStore.val && untrack(() => updateFlow(flowStore.val))
	})
	$effect(() => {
		selectedId && untrack(() => inferModuleArgs(selectedId))
	})

	let localModuleStates: Record<string, GraphModuleState> = $state({})

	let suspendStatus: StateStore<Record<string, { job: Job; nb: number }>> = $state({ val: {} })

	// Create a derived store that only shows the module states when showModuleStatus is true
	// this store can also be updated

	let flowModuleSchemaMap: FlowModuleSchemaMap | undefined = $state()
	function onJobDone() {
		if (!job) {
			return
		}
		// job was running and is now stopped
		if (!flowPreviewButtons?.getPreviewOpen()) {
			if (
				job.type === 'CompletedJob' &&
				job.success &&
				flowPreviewButtons?.getPreviewMode() === 'whole'
			) {
				if (flowModuleSchemaMap?.isNodeVisible('Result') && selectedId !== 'Result') {
					outputPickerOpenFns['Result']?.()
				}
			} else {
				// Find last module with a job in flow_status
				const lastModuleWithJob = job.flow_status?.modules
					?.slice()
					.reverse()
					.find((module) => 'job' in module)
				if (
					lastModuleWithJob &&
					lastModuleWithJob.id &&
					flowModuleSchemaMap?.isNodeVisible(lastModuleWithJob.id)
				) {
					outputPickerOpenFns[lastModuleWithJob.id]?.()
				}
			}
		}
	}

	function resetModulesStates() {
		showJobStatus = false
	}

	const flowHasChanged = $derived(flowPreviewContent?.flowHasChanged())

	const selectedId = $derived(selectionManager.getSelectedId())

	const selectedModule = $derived(
		selectedId && flowStore.val?.value
			? (findModuleInFlow(flowStore.val.value, selectedId) ?? undefined)
			: undefined
	)
</script>

<svelte:window onkeydown={onKeyDown} />

<JobLoader noCode={true} bind:this={jobLoader} bind:isLoading={testIsLoading} bind:job={testJob} />

<main class="h-screen w-full">
	{#snippet itemRow(item: ItemType & { marked?: string }, depth: number)}
		{@const wmItem = pickerItems.find((p) => p.path === item.path)}
		<button
			type="button"
			onclick={() => wmItem && pickPath(wmItem)}
			class="block w-full text-left cursor-pointer border-b last:border-b-0"
		>
			<Row
				marked={item.marked}
				path={item.path}
				summary={item.summary}
				kind={item.type}
				{depth}
				workspaceId={$workspaceStore ?? ''}
				canFavorite={false}
			/>
		</button>
	{/snippet}
	{#snippet treeNode(node: ItemType | FolderItem | UserItem, depth: number)}
		{#if 'folderName' in node}
			<details open class="group border-b last:border-b-0">
				<summary
					class="px-4 py-2 w-full flex flex-row items-center justify-between cursor-pointer list-none group-open:border-b"
				>
					<div
						class="flex flex-row items-center gap-4 text-sm font-semibold"
						style={depth > 0 ? `padding-left: ${depth * 16}px;` : ''}
					>
						<div class="flex justify-center items-center">
							{#if depth === 0}
								<Folder size={16} class="text-secondary" />
							{:else}
								<FolderTree size={16} class="text-secondary" />
							{/if}
						</div>
						<div>
							<span class="whitespace-nowrap text-xs text-emphasis font-semibold"
								>{#if depth === 0}f/{/if}{node.folderName}</span
							>
							<div class="text-2xs font-normal text-secondary whitespace-nowrap">
								({pluralize(node.items.length, 'item')})
							</div>
						</div>
					</div>
					<div class="w-full flex flex-row-reverse">
						<ChevronUp size={16} class="hidden group-open:block" />
						<ChevronDown size={16} class="block group-open:hidden" />
					</div>
				</summary>
				{#each node.items as child ('folderName' in child ? `f__${child.folderName}` : 'username' in child ? `u__${child.username}` : `i__${child.type}__${child.path}`)}
					{@render treeNode(child, depth + 1)}
				{/each}
			</details>
		{:else if 'username' in node}
			<details open class="group border-b last:border-b-0">
				<summary
					class="px-4 py-2 w-full flex flex-row items-center justify-between cursor-pointer list-none group-open:border-b"
				>
					<div
						class="flex flex-row items-center gap-4 text-sm font-semibold"
						style={depth > 0 ? `padding-left: ${depth * 16}px;` : ''}
					>
						<div class="flex justify-center items-center">
							<User size={16} class="text-secondary" />
						</div>
						<div>
							<span class="whitespace-nowrap text-xs text-emphasis font-semibold"
								>u/{node.username}</span
							>
							<div class="text-2xs font-normal text-secondary whitespace-nowrap">
								({pluralize(node.items.length, 'item')})
							</div>
						</div>
					</div>
					<div class="w-full flex flex-row-reverse">
						<ChevronUp size={16} class="hidden group-open:block" />
						<ChevronDown size={16} class="block group-open:hidden" />
					</div>
				</summary>
				{#each node.items as child ('folderName' in child ? `f__${child.folderName}` : 'username' in child ? `u__${child.username}` : `i__${child.type}__${child.path}`)}
					{@render treeNode(child, depth + 1)}
				{/each}
			</details>
		{:else}
			{@render itemRow(node as ItemType & { marked?: string }, depth)}
		{/if}
	{/snippet}

	{#if pickerMode}
		<div class="h-full w-full overflow-auto p-6">
			<div class="absolute top-2 left-2">
				<DarkModeToggle bind:darkMode bind:this={darkModeToggle} forcedDarkMode={false} />
			</div>
			<div class="absolute top-2 right-2 text-xs text-secondary">
				{#if $userStore}
					{$userStore?.username} on {$workspaceStore}
				{:else}
					<span class="text-red-600">Unable to login on {$workspaceStore}</span>
				{/if}
			</div>
			<div class="max-w-3xl mx-auto pt-8">
				<h1 class="text-2xl font-semibold text-primary mb-1">
					{$workspaceStore}
					<span class="font-normal text-secondary">(local)</span>
				</h1>
				<p class="text-sm text-secondary mb-4"> Click a flow or a script to preview it. </p>

				<SearchItems
					filter={pickerFilter}
					items={pickerKindFilteredItems}
					f={(item: ItemType) => `${item.path} ${item.summary ?? ''}`}
					bind:filteredItems={pickerFilteredItems}
				/>

				{#if wsState !== 'closed'}
					<div class="flex flex-row gap-2 items-center mb-3 w-full">
						<ToggleButtonGroup bind:selected={pickerKind} class="w-fit">
							{#snippet children({ item })}
								<ToggleButton value="all" label="All" size="md" {item} />
								<ToggleButton value="script" icon={Code2} label="Scripts" size="md" {item} />
								<ToggleButton
									value="flow"
									label="Flows"
									icon={FlowIcon}
									selectedColor="#14b8a6"
									size="md"
									{item}
								/>
								<ToggleButton
									value="raw_app"
									label="Apps"
									icon={LayoutDashboard}
									selectedColor="#fb923c"
									size="md"
									{item}
								/>
							{/snippet}
						</ToggleButtonGroup>

						<div class="relative text-primary flex-1 min-w-[100px]">
							<!-- svelte-ignore a11y_autofocus -->
							<TextInput
								inputProps={{
									autofocus: true,
									placeholder: HOME_SEARCH_PLACEHOLDER
								}}
								size="md"
								bind:value={pickerFilter}
								class="!pr-10"
							/>
							<div class="absolute right-0 top-0 mt-2 mr-4 text-secondary" aria-hidden="true">
								<Search size={16} />
							</div>
						</div>
					</div>
				{/if}

				{#if wsState === 'closed'}
					<Alert type="warning" title="Dev server is not running">
						Start it from your workspace root with
						<code class="text-xs px-1 py-0.5 rounded bg-surface-secondary">wmill dev</code>
						to preview your flows, scripts, and apps.
					</Alert>
				{:else if pickerItems.length === 0}
					<div class="text-sm text-secondary"
						>No flows, scripts, or apps detected in this workspace.</div
					>
				{:else if pickerGroups.length === 0}
					<div class="text-sm text-secondary">No items match the search.</div>
				{:else}
					<div class="border rounded-md bg-surface-tertiary overflow-hidden">
						{#each pickerGroups as group ('folderName' in group ? `f__${group.folderName}` : 'username' in group ? `u__${group.username}` : `i__${group.type}__${group.path}`)}
							{@render treeNode(group, 0)}
						{/each}
					</div>
				{/if}
			</div>
		</div>
	{:else if mode == 'script'}
		<div class="flex flex-col min-h-full min-h-screen overflow-auto">
			<div class="absolute top-0 left-2">
				<DarkModeToggle bind:darkMode bind:this={darkModeToggle} forcedDarkMode={false} />
			</div>
			<div class="text-center w-full text-lg truncate py-1 text-primary">
				{currentScript?.path ?? 'Not editing a script'}
				<span class="text-2xs text-secondary">{currentScript?.language ?? ''}</span><span
					class="text-2xs text-secondary">{currentScript?.isCodebase ? ' (codebase)' : ''}</span
				>
			</div>

			<div class="absolute top-2 right-2 !text-primary text-xs">
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
					<Button on:click={jobLoader?.cancelJob} btnClasses="w-full" color="red" size="xs">
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
						variant="accent"
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
			<Splitpanes horizontal style="height: 1000px;">
				<Pane size={33}>
					<div class="px-2">
						<div class="flex justify-end mb-2">
							<Toggle
								bind:checked={jsonView}
								size="xs"
								options={{
									right: 'JSON',
									rightTooltip: 'Fill args from JSON'
								}}
								lightMode
								on:change={() => {
									jsonEditor?.setCode(JSON.stringify(args ?? {}, null, '\t'))
								}}
							/>
						</div>
						{#if jsonView}
							<div class="py-2" style="height: {Math.max(schemaHeight, 300)}px">
								<JsonInputs
									bind:this={jsonEditor}
									on:select={(e) => {
										if (e.detail) {
											args = e.detail
										}
									}}
									updateOnBlur={false}
									placeholder={`Write args as JSON.<br/><br/>Example:<br/><br/>{<br/>&nbsp;&nbsp;"foo": "12"<br/>}`}
								/>
							</div>
						{:else}
							<div class="break-words relative font-sans" bind:clientHeight={schemaHeight}>
								<SchemaForm compact {schema} bind:args bind:isValid />
							</div>
						{/if}
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
		<div
			class="h-full w-full"
			bind:clientWidth={flowContainerWidth}
			bind:clientHeight={flowContainerHeight}
		>
			<div class="flex flex-col max-h-screen h-full relative" bind:clientWidth={paneWidth}>
				<div class="absolute top-0 left-2">
					<DarkModeToggle bind:darkMode bind:this={darkModeToggle} forcedDarkMode={false} />
					{#if $userStore}
						As {$userStore?.username} in {$workspaceStore}
					{:else}
						<span class="text-red-600">Unable to login on {$workspaceStore}</span>
					{/if}
				</div>

				<Splitpanes horizontal={flowHorizontalSplit} class="min-h-0 max-h-screen grow">
					<Pane size={67}>
						<div class="relative h-full w-full">
							{#if flowStore.val?.value?.modules}
								<div id="flow-editor"></div>
								<div
									class="flex justify-center pt-1 z-50 absolute gap-2 {compactPreview
										? 'left-1/2 -translate-x-1/2 top-14'
										: 'right-2 top-2'}"
								>
									<FlowPreviewButtons
										{suspendStatus}
										bind:this={flowPreviewButtons}
										{onJobDone}
										bind:localModuleStates
										onRunPreview={() => {
											showJobStatus = true
										}}
									/>
								</div>
								<FlowModuleSchemaMap
									bind:this={flowModuleSchemaMap}
									disableAi
									disableTutorials
									smallErrorHandler={true}
									disableStaticInputs
									localModuleStates={showJobStatus ? localModuleStates : {}}
									onTestUpTo={flowPreviewButtons?.testUpTo}
									testModuleStates={modulesTestStates}
									isOwner={flowPreviewContent?.getIsOwner?.()}
									onTestFlow={flowPreviewButtons?.runPreview}
									isRunning={flowPreviewContent?.getIsRunning?.()}
									onCancelTestFlow={flowPreviewContent?.cancelTest}
									onOpenPreview={flowPreviewButtons?.openPreview}
									onHideJobStatus={resetModulesStates}
									flowJob={job}
									{showJobStatus}
									onDelete={(id) => {
										delete localModuleStates[id]
										delete modulesTestStates.states[id]
									}}
									{flowHasChanged}
									controlsPosition="bottom"
								/>
							{:else}
								<div class="text-red-400 mt-20">Missing flow modules</div>
							{/if}
						</div></Pane
					>

					<Pane size={33}>
						<div class="h-full w-full pl-0.5">
							{#if selectedModule}
								<div
									class="flex items-center gap-2 px-3 py-1.5 border-b border-border bg-surface shrink-0"
								>
									<span class="text-xs text-secondary shrink-0">{selectedModule.id} summary</span>
									<TextInput
										inputProps={{ placeholder: 'Summary' }}
										bind:value={selectedModule.summary}
									/>
								</div>
							{/if}
							{#key reload}
								<FlowEditorPanel
									enableAi
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
									onTestFlow={flowPreviewButtons?.runPreview}
									{job}
									isOwner={flowPreviewContent?.getIsOwner()}
									{suspendStatus}
									onOpenDetails={flowPreviewButtons?.openPreview}
									previewOpen={flowPreviewButtons?.getPreviewOpen()}
								/>
							{/key}
						</div>
					</Pane>
				</Splitpanes>
			</div>
		</div>
	{/if}
</main>
