<script lang="ts">
	import {
		FlowService,
		type Flow,
		type FlowModule,
		DraftService,
		type PathScript,
		ScriptService,
		type OpenFlow,
		type RawScript,
		type InputTransform,
		type TriggersCount,
		CaptureService
	} from '$lib/gen'
	import { initHistory, push, redo, undo } from '$lib/history'
	import {
		copilotInfo,
		enterpriseLicense,
		tutorialsToDo,
		userStore,
		workspaceStore,
		usedTriggerKinds
	} from '$lib/stores'
	import {
		cleanValueProperties,
		encodeState,
		generateRandomString,
		orderedJsonStringify,
		replaceFalseWithUndefined,
		sleep,
		type Value
	} from '$lib/utils'
	import { sendUserToast } from '$lib/toast'
	import { Drawer } from '$lib/components/common'
	import DeployOverrideConfirmationModal from '$lib/components/common/confirmationModal/DeployOverrideConfirmationModal.svelte'

	import { onMount, setContext, tick, type ComponentType } from 'svelte'
	import { writable, type Writable } from 'svelte/store'
	import CenteredPage from './CenteredPage.svelte'
	import { Badge, Button, UndoRedo } from './common'
	import FlowEditor from './flows/FlowEditor.svelte'
	import ScriptEditorDrawer from './flows/content/ScriptEditorDrawer.svelte'
	import type { FlowState } from './flows/flowState'
	import { dfs as dfsApply } from './flows/dfs'
	import { dfs, getPreviousIds } from './flows/previousResults'
	import FlowImportExportMenu from './flows/header/FlowImportExportMenu.svelte'
	import FlowPreviewButtons from './flows/header/FlowPreviewButtons.svelte'
	import type { FlowEditorContext, FlowInput, FlowInputEditorState } from './flows/types'
	import { cleanInputs, emptyFlowModuleState } from './flows/utils'
	import { Calendar, Pen, Save, DiffIcon, HistoryIcon, FileJson, type Icon } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import Awareness from './Awareness.svelte'
	import { getAllModules } from './flows/flowExplorer'
	import {
		stepCopilot,
		type FlowCopilotModule,
		glueCopilot,
		type FlowCopilotContext
	} from './copilot/flow'
	import type { Schema, SchemaProperty } from '$lib/common'
	import FlowCopilotDrawer from './copilot/FlowCopilotDrawer.svelte'
	import FlowCopilotStatus from './copilot/FlowCopilotStatus.svelte'
	import { fade } from 'svelte/transition'
	import { loadFlowModuleState, pickScript } from './flows/flowStateUtils'
	import FlowCopilotInputsModal from './copilot/FlowCopilotInputsModal.svelte'
	import FlowBuilderTutorials from './FlowBuilderTutorials.svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import FlowTutorials from './FlowTutorials.svelte'
	import { ignoredTutorials } from './tutorials/ignoredTutorials'
	import type DiffDrawer from './DiffDrawer.svelte'
	import FlowHistory from './flows/FlowHistory.svelte'
	import Summary from './Summary.svelte'
	import type { FlowBuilderWhitelabelCustomUi } from './custom_ui'
	import FlowYamlEditor from './flows/header/FlowYamlEditor.svelte'
	import { type TriggerContext, type ScheduleTrigger } from './triggers'
	import type { SavedAndModifiedValue } from './common/confirmationModal/unsavedTypes'
	import DeployButton from './DeployButton.svelte'
	import type { Trigger } from './triggers/utils'
	import { deployTriggers, fetchTriggers, handleSelectTriggerFromKind } from './triggers/utils'
	import DraftTriggersConfirmationModal from './common/confirmationModal/DraftTriggersConfirmationModal.svelte'

	export let initialPath: string = ''
	export let pathStoreInit: string | undefined = undefined
	export let newFlow: boolean
	export let selectedId: string | undefined
	export let initialArgs: Record<string, any> = {}
	export let loading = false
	export let flowStore: Writable<OpenFlow>
	export let flowStateStore: Writable<FlowState>
	export let savedFlow:
		| (Flow & {
				draft?: Flow | undefined
		  })
		| undefined = undefined
	export let diffDrawer: DiffDrawer | undefined = undefined
	export let customUi: FlowBuilderWhitelabelCustomUi = {}
	export let disableAi: boolean = false
	export let disabledFlowInputs = false
	export let savedPrimarySchedule: ScheduleTrigger | undefined = undefined // used to set the primary schedule in the legacy primaryScheduleStore
	export let version: number | undefined = undefined
	export let setSavedraftCb: ((cb: () => void) => void) | undefined = undefined
	export let savedDraftTriggers: Trigger[] = []

	let initialPathStore = writable(initialPath)
	$: initialPathStore.set(initialPath)

	// used for new flows for captures
	let fakeInitialPath =
		'u/' +
		($userStore?.username?.includes('@')
			? $userStore!.username.split('@')[0].replace(/[^a-zA-Z0-9_]/g, '')
			: $userStore!.username!) +
		'/' +
		generateRandomString(12)

	// Used by multiplayer deploy collision warning
	let deployedValue: Value | undefined = undefined // Value to diff against
	let deployedBy: string | undefined = undefined // Author
	let confirmCallback: () => void = () => {} // What happens when user clicks `override` in warning
	let open: boolean = false // Is confirmation modal open

	// Draft triggers confirmation modal
	let draftTriggersModalOpen = false
	let confirmDeploymentCallback: (triggersToDeploy: Trigger[]) => void = () => {}

	async function handleDraftTriggersConfirmed(event: CustomEvent<{ selectedTriggers: Trigger[] }>) {
		const { selectedTriggers } = event.detail
		// Continue with saving the flow
		draftTriggersModalOpen = false
		confirmDeploymentCallback(selectedTriggers)
	}

	$: setContext('customUi', customUi)

	export function getInitialAndModifiedValues(): SavedAndModifiedValue {
		return {
			savedValue: savedFlow,
			modifiedValue: $flowStore
		}
	}
	let onLatest = true
	async function compareVersions() {
		if (version === undefined) {
			return
		}
		try {
			if (initialPath && initialPath != '') {
				const flowVersion = await FlowService.getFlowLatestVersion({
					workspace: $workspaceStore!,
					path: initialPath
				})

				onLatest = version === flowVersion?.id
			} else {
				onLatest = true
			}
		} catch (err) {
			console.error('Error comparing versions', err)
			onLatest = true
		}
	}

	const dispatch = createEventDispatcher()

	const primaryScheduleStore = writable<ScheduleTrigger | undefined | false>(savedPrimarySchedule) // kept for legacy reasons
	const triggersCount = writable<TriggersCount | undefined>(undefined) // trigger count only show deployed triggers
	const simplifiedPoll = writable(false)

	// used to set the primary schedule in the legacy primaryScheduleStore
	export function setPrimarySchedule(schedule: ScheduleTrigger | undefined | false) {
		primaryScheduleStore.set(schedule)
	}

	export function setDraftTriggers(triggers: Trigger[]) {
		$triggersStore = [...triggers, ...$triggersStore.filter((t) => !t.draftConfig)]
		loadTriggers()
	}

	let loadingSave = false
	let loadingDraft = false

	export async function saveDraft(forceSave = false): Promise<void> {
		if (!newFlow && !savedFlow) {
			return
		}
		if (savedFlow) {
			const draftOrDeployed = cleanValueProperties(savedFlow.draft || savedFlow)
			const current = cleanValueProperties({ ...$flowStore, path: $pathStore })
			if (!forceSave && orderedJsonStringify(draftOrDeployed) === orderedJsonStringify(current)) {
				sendUserToast('No changes detected, ignoring', false, [
					{
						label: 'Save anyway',
						callback: () => {
							saveDraft(true)
						}
					}
				])
				return
			}
		}
		loadingDraft = true
		try {
			const flow = cleanInputs($flowStore)
			try {
				localStorage.removeItem('flow')
				localStorage.removeItem(`flow-${$pathStore}`)
			} catch (e) {
				console.error('error interacting with local storage', e)
			}
			if (newFlow || savedFlow?.draft_only) {
				if (savedFlow?.draft_only) {
					await FlowService.deleteFlowByPath({
						workspace: $workspaceStore!,
						path: initialPath,
						keepCaptures: true
					})
				}
				if (!initialPath || $pathStore != initialPath) {
					await CaptureService.moveCapturesAndConfigs({
						workspace: $workspaceStore!,
						path: initialPath || fakeInitialPath,
						requestBody: {
							new_path: $pathStore
						},
						runnableKind: 'flow'
					})
				}
				await FlowService.createFlow({
					workspace: $workspaceStore!,
					requestBody: {
						path: $pathStore,
						summary: flow.summary ?? '',
						description: flow.description ?? '',
						value: flow.value,
						schema: flow.schema,
						tag: flow.tag,
						draft_only: true,
						ws_error_handler_muted: flow.ws_error_handler_muted,
						visible_to_runner_only: flow.visible_to_runner_only,
						on_behalf_of_email: flow.on_behalf_of_email
					}
				})
			}
			await DraftService.createDraft({
				workspace: $workspaceStore!,
				requestBody: {
					path: newFlow || savedFlow?.draft_only ? $pathStore : initialPath,
					typ: 'flow',
					value: {
						...flow,
						path: $pathStore,
						draft_triggers: $triggersStore.filter((t) => t.draftConfig)
					}
				}
			})

			savedFlow = {
				...(newFlow || savedFlow?.draft_only
					? {
							...structuredClone($flowStore),
							path: $pathStore,
							draft_only: true
						}
					: savedFlow),
				draft: {
					...structuredClone($flowStore),
					path: $pathStore
				}
			} as Flow & {
				draft?: Flow
			}

			let savedAtNewPath = false
			if (newFlow) {
				dispatch('saveInitial', $pathStore)
			} else if (savedFlow?.draft_only && $pathStore !== initialPath) {
				savedAtNewPath = true
				initialPath = $pathStore
				// this is so we can use the flow builder outside of sveltekit
				dispatch('saveDraftOnlyAtNewPath', { path: $pathStore, selectedId: getSelectedId() })
			}
			dispatch('saveDraft', { path: $pathStore, savedAtNewPath, newFlow })
			sendUserToast('Saved as draft')
		} catch (error) {
			sendUserToast(`Error while saving the flow as a draft: ${error.body || error.message}`, true)
			dispatch('saveDraftError', error)
		}
		loadingDraft = false
	}

	onMount(() => {
		setSavedraftCb?.(() => saveDraft())
	})

	export function computeUnlockedSteps(flow: Flow) {
		return Object.fromEntries(
			getAllModules(flow.value.modules, flow.value.failure_module)
				.filter((m) => m.value.type == 'script' && m.value.hash == null)
				.map((m) => [m.id, (m.value as PathScript).path])
		)
	}

	async function handleSaveFlow(deploymentMsg?: string) {
		await compareVersions()
		if (onLatest || initialPath == '' || savedFlow?.draft_only) {
			// Handle directly
			await saveFlow(deploymentMsg)
		} else {
			// We need it for diff
			await syncWithDeployed()

			if (
				deployedValue &&
				$flowStore &&
				orderedJsonStringify(deployedValue) ===
					orderedJsonStringify(replaceFalseWithUndefined({ ...$flowStore, path: $pathStore }))
			) {
				await saveFlow(deploymentMsg)
			} else {
				// Handle through confirmation modal
				confirmCallback = async () => {
					await saveFlow(deploymentMsg)
				}
				// Open confirmation modal
				open = true
			}
		}
	}
	async function syncWithDeployed() {
		const flow = await FlowService.getFlowByPath({
			workspace: $workspaceStore!,
			path: initialPath,
			withStarredInfo: true
		})
		deployedValue = replaceFalseWithUndefined({
			...flow,
			edited_at: undefined,
			edited_by: undefined,
			workspace_id: undefined
		})
		deployedBy = flow.edited_by
	}

	async function saveFlow(deploymentMsg?: string, triggersToDeploy?: Trigger[]): Promise<void> {
		if (!triggersToDeploy) {
			// Check if there are draft triggers that need confirmation
			const draftTriggers = $triggersStore.filter((trigger) => trigger.draftConfig)
			if (draftTriggers.length > 0) {
				draftTriggersModalOpen = true
				confirmDeploymentCallback = async (triggersToDeploy: Trigger[]) => {
					await saveFlow(deploymentMsg, triggersToDeploy)
				}
				return
			}
		}

		loadingSave = true
		try {
			const flow = cleanInputs($flowStore)
			// console.log('flow', computeUnlockedSteps(flow)) // del
			// loadingSave = false // del
			// return

			if (newFlow) {
				try {
					localStorage.removeItem('flow')
					localStorage.removeItem(`flow-${$pathStore}`)
				} catch (e) {
					console.error('error interacting with local storage', e)
				}
				await FlowService.createFlow({
					workspace: $workspaceStore!,
					requestBody: {
						path: $pathStore,
						summary: flow.summary ?? '',
						description: flow.description ?? '',
						value: flow.value,
						schema: flow.schema,
						ws_error_handler_muted: flow.ws_error_handler_muted,
						tag: flow.tag,
						dedicated_worker: flow.dedicated_worker,
						visible_to_runner_only: flow.visible_to_runner_only,
						on_behalf_of_email: flow.on_behalf_of_email,
						deployment_message: deploymentMsg || undefined
					}
				})
				await CaptureService.moveCapturesAndConfigs({
					workspace: $workspaceStore!,
					path: fakeInitialPath,
					requestBody: {
						new_path: $pathStore
					},
					runnableKind: 'flow'
				})
				if (triggersToDeploy) {
					await deployTriggers(
						triggersToDeploy,
						$workspaceStore,
						!!$userStore?.is_admin || !!$userStore?.is_super_admin,
						usedTriggerKinds,
						$pathStore
					)
				}
			} else {
				try {
					localStorage.removeItem(`flow-${initialPath}`)
				} catch (e) {
					console.error('error interacting with local storage', e)
				}

				if (triggersToDeploy) {
					await deployTriggers(
						triggersToDeploy,
						$workspaceStore,
						!!$userStore?.is_admin || !!$userStore?.is_super_admin,
						usedTriggerKinds,
						initialPath
					)
				}

				await FlowService.updateFlow({
					workspace: $workspaceStore!,
					path: initialPath,
					requestBody: {
						path: $pathStore,
						summary: flow.summary,
						description: flow.description ?? '',
						value: flow.value,
						schema: flow.schema,
						tag: flow.tag,
						dedicated_worker: flow.dedicated_worker,
						ws_error_handler_muted: flow.ws_error_handler_muted,
						visible_to_runner_only: flow.visible_to_runner_only,
						on_behalf_of_email: flow.on_behalf_of_email,
						deployment_message: deploymentMsg || undefined
					}
				})
			}
			savedFlow = {
				...structuredClone($flowStore),
				path: $pathStore
			} as Flow
			loadingSave = false
			dispatch('deploy', $pathStore)
		} catch (err) {
			dispatch('deployError', err)
			sendUserToast(`The flow could not be saved: ${err.body}`, true)
			loadingSave = false
		}
	}

	let timeout: NodeJS.Timeout | undefined = undefined

	$: {
		if ($flowStore || $selectedIdStore) {
			saveSessionDraft()
		}
	}

	function saveSessionDraft() {
		timeout && clearTimeout(timeout)
		timeout = setTimeout(() => {
			try {
				localStorage.setItem(
					initialPath && initialPath != '' ? `flow-${initialPath}` : 'flow',
					encodeState({
						flow: $flowStore,
						path: $pathStore,
						selectedId: $selectedIdStore,
						draftTriggers: $triggersStore.filter((t) => t.draftConfig)
					})
				)
			} catch (err) {
				console.error(err)
			}
		}, 500)
	}

	const selectedIdStore = writable<string>(selectedId ?? 'settings-metadata')

	export function getSelectedId() {
		return $selectedIdStore
	}

	const previewArgsStore = writable<Record<string, any>>(initialArgs)
	const scriptEditorDrawer = writable<ScriptEditorDrawer | undefined>(undefined)
	const moving = writable<{ module: FlowModule; modules: FlowModule[] } | undefined>(undefined)
	const history = initHistory($flowStore)
	const pathStore = writable<string>(pathStoreInit ?? initialPath)
	const captureOn = writable<boolean>(false)
	const showCaptureHint = writable<boolean | undefined>(undefined)
	const flowInputEditorStateStore = writable<FlowInputEditorState>({
		selectedTab: undefined,
		editPanelSize: 0,
		payloadData: undefined
	})
	$: initialPath && ($pathStore = initialPath)

	const testStepStore = writable<Record<string, any>>({})

	function select(selectedId: string) {
		selectedIdStore.set(selectedId)
	}

	let insertButtonOpen = writable<boolean>(false)

	setContext<FlowEditorContext>('FlowEditorContext', {
		selectedId: selectedIdStore,
		previewArgs: previewArgsStore,
		scriptEditorDrawer,
		moving,
		history,
		flowStateStore,
		flowStore,
		pathStore,
		testStepStore,
		saveDraft,
		initialPathStore,
		fakeInitialPath,
		flowInputsStore: writable<FlowInput>({}),
		customUi,
		insertButtonOpen,
		executionCount: writable(0),
		flowInputEditorState: flowInputEditorStateStore
	})

	// Add triggers context store
	const triggersStore = writable<Trigger[]>([
		{ type: 'webhook', path: '', isDraft: false },
		{ type: 'email', path: '', isDraft: false },
		...savedDraftTriggers
	])

	const selectedTriggerStore = writable<Trigger | undefined>(undefined)
	setContext<TriggerContext>('TriggerContext', {
		selectedTrigger: selectedTriggerStore,
		triggersCount,
		simplifiedPoll,
		showCaptureHint,
		triggers: triggersStore
	})

	export async function loadTriggers() {
		$triggersCount = await FlowService.getTriggersCountOfFlow({
			workspace: $workspaceStore!,
			path: initialPath
		})

		// Initialize triggers using utility function
		fetchTriggers(
			triggersStore,
			triggersCount,
			$workspaceStore,
			initialPath,
			true,
			$primaryScheduleStore,
			$userStore
		)
	}

	$: selectedId && select(selectedId)

	$: initialPath && initialPath != '' && $workspaceStore && loadTriggers()

	function onKeyDown(event: KeyboardEvent) {
		let classes = event.target?.['className']
		if (
			(typeof classes === 'string' && classes.includes('inputarea')) ||
			['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName!)
		) {
			return
		}

		switch (event.key) {
			case 'Z':
				if (event.ctrlKey || event.metaKey) {
					$flowStore = redo(history)
					event.preventDefault()
				}
				break
			case 'z':
				if (event.ctrlKey || event.metaKey) {
					$flowStore = undo(history, $flowStore)
					$selectedIdStore = 'Input'
					event.preventDefault()
				}
				break
			case 's':
				if (event.ctrlKey || event.metaKey) {
					saveDraft()
					event.preventDefault()
				}
				break
			case 'ArrowDown': {
				if (!$insertButtonOpen) {
					let ids = generateIds()
					let idx = ids.indexOf($selectedIdStore)
					if (idx > -1 && idx < ids.length - 1) {
						$selectedIdStore = ids[idx + 1]
						event.preventDefault()
					}
				}
				break
			}
			case 'ArrowUp': {
				if (!$insertButtonOpen) {
					let ids = generateIds()
					let idx = ids.indexOf($selectedIdStore)
					if (idx > 0 && idx < ids.length) {
						$selectedIdStore = ids[idx - 1]
						event.preventDefault()
					}
				}
				break
			}
		}
	}

	function generateIds() {
		return [
			'settings-metadata',
			'constants',
			'preprocessor',
			...dfsApply($flowStore.value.modules, (module) => module.id)
		]
	}

	const dropdownItems: Array<{
		label: string
		onClick: () => void
	}> = []

	if (customUi.topBar?.extraDeployOptions != false) {
		if (savedFlow?.draft_only === false || savedFlow?.draft_only === undefined) {
			dropdownItems.push({
				label: 'Exit & see details',
				onClick: () => dispatch('details', $pathStore)
			})
		}

		if (!newFlow) {
			dropdownItems.push({
				label: 'Fork',
				onClick: () => window.open(`/flows/add?template=${initialPath}`)
			})
		}
	}

	let flowCopilotContext: FlowCopilotContext = {
		drawerStore: writable<Drawer | undefined>(undefined),
		modulesStore: writable<FlowCopilotModule[]>([]),
		currentStepStore: writable<string | undefined>(undefined),
		genFlow: undefined,
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

	const {
		drawerStore: copilotDrawerStore,
		modulesStore: copilotModulesStore,
		currentStepStore: copilotCurrentStepStore,
		shouldUpdatePropertyType
	} = flowCopilotContext

	let doneTs = 0
	async function getHubCompletions(text: string, idx: number, type: 'trigger' | 'script') {
		try {
			// make sure we display the results of the last request last
			const ts = Date.now()
			const scripts = (
				await ScriptService.queryHubScripts({
					text: `${text}`,
					limit: 3,
					kind: type
				})
			).map((s) => ({
				...s,
				path: `hub/${s.version_id}/${s.app}/${s.summary.toLowerCase().replaceAll(/\s+/g, '_')}`
			}))
			if (ts < doneTs) return
			doneTs = ts

			$copilotModulesStore[idx].hubCompletions = scripts as {
				path: string
				summary: string
				kind: string
				app: string
				ask_id: number
				id: number
				version_id: number
			}[]
		} catch (err) {
			if (err.name !== 'CancelError') throw err
		}
	}

	let abortController: AbortController | undefined = undefined
	let copilotLoading = false
	let flowCopilotMode: 'trigger' | 'sequence' = 'sequence'
	let copilotStatus: string = ''
	let copilotFlowInputs: Record<string, SchemaProperty> = {}
	let copilotFlowRequiredInputs: string[] = []
	let openCopilotInputsModal = false

	function setInitCopilotModules(mode: typeof flowCopilotMode) {
		$copilotModulesStore = [
			{
				id: 'a',
				type: mode === 'trigger' ? 'trigger' : 'script',
				description: '',
				code: '',
				hubCompletions: [],
				selectedCompletion: undefined,
				source: undefined,
				lang: undefined
			},
			{
				id: 'b',
				type: 'script',
				description: '',
				code: '',
				hubCompletions: [],
				selectedCompletion: undefined,
				source: undefined,
				lang: undefined
			}
		]
	}

	$: setInitCopilotModules(flowCopilotMode)

	function applyCopilotFlowInputs() {
		const properties = {
			...($flowStore.schema?.properties as Record<string, SchemaProperty> | undefined),
			...copilotFlowInputs
		}
		const required = [
			...(($flowStore.schema?.required as string[] | undefined) ?? []),
			...copilotFlowRequiredInputs
		]
		$flowStore.schema = {
			$schema: 'https://json-schema.org/draft/2020-12/schema',
			properties,
			required,
			type: 'object'
		}
	}

	function clearFlowInputsFromStep(id: string | undefined) {
		const module: FlowModule | undefined = dfs(id, $flowStore)[0]
		if (module?.value.type === 'rawscript' || module?.value.type === 'script') {
			// clear step inputs that start with flow_input. but not flow_input.iter
			for (const key in module.value.input_transforms) {
				const input = module.value.input_transforms[key]
				if (
					input.type === 'javascript' &&
					input.expr.includes('flow_input.') &&
					!input.expr.includes('flow_input.iter')
				) {
					module.value.input_transforms[key] = {
						type: 'static',
						value: undefined
					}
					$shouldUpdatePropertyType[key] = 'static'
				}
			}
		}
		$flowStore = $flowStore
	}

	async function finishStepGen() {
		copilotFlowInputs = {}
		copilotFlowRequiredInputs = []
		setInitCopilotModules(flowCopilotMode)
		copilotStatus = "Done! Just check the step's inputs and you're good to go!"
		await sleep(3000)
		copilotStatus = ''
	}

	function snakeCase(e: string): string {
		if (e.toLowerCase() === e) {
			return e
		}

		return (
			e
				.match(/([A-Z])/g)
				?.reduce((str, c) => str.replace(new RegExp(c), '_' + c.toLowerCase()), e)
				?.substring(e.slice(0, 1).match(/([A-Z])/g) ? 1 : 0) ?? e
		)
	}
	async function genFlow(idx: number, flowModules: FlowModule[], stepOnly = false) {
		try {
			push(history, $flowStore)
			let module = stepOnly ? $copilotModulesStore[0] : $copilotModulesStore[idx]

			copilotLoading = true
			copilotStatus = "Generating code for step '" + module.id + "'..."
			$copilotCurrentStepStore = module.id
			focusCopilot()

			if (!stepOnly && flowModules.length > idx) {
				select('')
				await tick()
				flowModules.splice(idx, flowModules.length - idx)
				$flowStore = $flowStore
				focusCopilot()
			}

			if (idx === 0 && !stepOnly) {
				$flowStore.schema = {
					$schema: 'https://json-schema.org/draft/2020-12/schema',
					properties: {},
					required: [],
					type: 'object'
				}
			}

			const flowModule: FlowModule & {
				value: RawScript | PathScript
			} = {
				id: module.id,
				stop_after_if:
					module.type === 'trigger'
						? {
								expr: 'result == undefined || Array.isArray(result) && result.length == 0',
								skip_if_stopped: true
							}
						: undefined,
				value: {
					input_transforms: {},
					content: '',
					language: module.lang ?? 'bun',
					type: 'rawscript'
				},
				summary: module.description
			}

			let isHubStep = false
			if (module.source === 'hub' && module.selectedCompletion) {
				isHubStep = true
				const [hubScriptModule, hubScriptState] = await pickScript(
					module.selectedCompletion.path,
					`${module.selectedCompletion.summary} (${module.selectedCompletion.app})`,
					module.id,
					undefined
				)
				flowModule.value = hubScriptModule.value
				flowModule.summary = hubScriptModule.summary
				$flowStateStore[module.id] = hubScriptState
			} else {
				$flowStateStore[module.id] = emptyFlowModuleState()
			}

			if (stepOnly) {
				flowModules.splice(idx, 0, flowModule)
			} else if (idx === 1 && $copilotModulesStore[idx - 1].type === 'trigger') {
				const loopModule: FlowModule = {
					id: module.id + '_loop',
					value: {
						type: 'forloopflow',
						iterator: {
							type: 'javascript',
							expr: 'results.a'
						},
						skip_failures: true,
						modules: [flowModule]
					}
				}
				const loopState = await loadFlowModuleState(loopModule)
				$flowStateStore[loopModule.id] = loopState
				flowModules.push(loopModule)
			} else {
				flowModules.push(flowModule)
			}

			$copilotDrawerStore?.closeDrawer()
			await tick()
			select(module.id)
			await tick()
			focusCopilot()

			let isFirstInLoop = false
			const parents = dfs(module.id, $flowStore).slice(1)
			if (
				parents[0]?.value.type === 'forloopflow' &&
				parents[0].value.modules[0].id === module.id
			) {
				isFirstInLoop = true
			}
			const prevNodeId = getPreviousIds(module.id, $flowStore, false)[0] as string | undefined
			const pastModule = dfs(prevNodeId, $flowStore, false)[0] as FlowModule | undefined

			if (!module.source) {
				throw new Error('Invalid copilot module source')
			}

			if (module.source === 'custom') {
				const deltaStore = writable<string>('')
				const unsubscribe = deltaStore.subscribe(async (delta) => {
					module.editor?.append(delta)
				})

				abortController = new AbortController()
				await stepCopilot(
					module,
					deltaStore,
					$workspaceStore!,
					pastModule?.value.type === 'rawscript' || pastModule?.value.type === 'script'
						? (pastModule as FlowModule & {
								value: RawScript | PathScript
							})
						: undefined,
					isFirstInLoop,
					abortController
				)
				unsubscribe()
			}

			copilotStatus = "Generating inputs for step '" + module.id + "'..."
			await sleep(500) // make sure code was parsed

			try {
				if (
					(flowModule.value.type === 'rawscript' || flowModule.value.type === 'script') &&
					(pastModule === undefined ||
						pastModule.value.type === 'rawscript' ||
						pastModule.value.type === 'script')
				) {
					const stepSchema: Schema = JSON.parse(JSON.stringify($flowStateStore[module.id].schema)) // deep copy
					if (isHubStep && pastModule !== undefined && $copilotInfo.enabled) {
						// ask AI to set step inputs
						abortController = new AbortController()
						const { inputs, allExprs } = await glueCopilot(
							flowModule.value.input_transforms,
							$workspaceStore!,
							pastModule as FlowModule & {
								value: RawScript | PathScript
							},
							isFirstInLoop,
							abortController
						)

						// create flow inputs used by AI for autocompletion
						copilotFlowInputs = {}
						copilotFlowRequiredInputs = []
						Object.entries(allExprs).forEach(([key, expr]) => {
							if (expr.includes('flow_input.') && !expr.includes('flow_input.iter.')) {
								const flowInputKey = expr.match(/flow_input\.([A-Za-z0-9_]+)/)?.[1]
								if (
									flowInputKey !== undefined &&
									(!$flowStore.schema ||
										!(flowInputKey in (($flowStore.schema.properties as any) ?? {}))) // prevent overriding flow inputs
								) {
									if (key in stepSchema.properties) {
										copilotFlowInputs[flowInputKey] = stepSchema.properties[key]
										if (stepSchema.required.includes(key)) {
											copilotFlowRequiredInputs.push(flowInputKey)
										}
									} else {
										// when the key is nested (e.g. body.content)
										const [firstKey, ...rest] = key.split('.')
										const restKey = rest.join('.')
										const firstKeyProperties = stepSchema.properties[firstKey]?.properties
										if (firstKeyProperties !== undefined && restKey in firstKeyProperties) {
											copilotFlowInputs[flowInputKey] = firstKeyProperties[restKey]
											if (firstKeyProperties[restKey].required?.includes(flowInputKey)) {
												copilotFlowRequiredInputs.push(flowInputKey)
											}
										}
									}
								}
							}
						})

						if (!stepOnly) {
							applyCopilotFlowInputs()
						}

						// set step inputs
						Object.entries(inputs).forEach(([key, expr]) => {
							flowModule.value.input_transforms[key] = {
								type: 'javascript',
								expr
							}
							$shouldUpdatePropertyType[key] = 'javascript'
						})
					} else {
						if (isHubStep && pastModule !== undefined && !$copilotInfo.enabled) {
							sendUserToast(
								'For better input generation, enable Windmill AI in the workspace settings',
								true
							)
						}

						// create possible flow inputs for autocompletion
						copilotFlowInputs = {}
						copilotFlowRequiredInputs = []
						Object.keys(flowModule.value.input_transforms).forEach((key) => {
							if (key !== 'prev_output') {
								const schema = $flowStateStore[module.id].schema
								const schemaProperty = Object.entries(schema?.properties ?? {}).find(
									(x) => x[0] === key
								)?.[1]
								const snakeKey = snakeCase(key)
								if (
									schemaProperty &&
									(!$flowStore.schema ||
										!(snakeKey in ($flowStore?.schema?.properties ?? ({} as any)))) // prevent overriding flow inputs
								) {
									copilotFlowInputs[snakeKey] = schemaProperty
									if (schema?.required.includes(snakeKey)) {
										copilotFlowRequiredInputs.push(snakeKey)
									}
								}
							}
						})
						if (!stepOnly) {
							applyCopilotFlowInputs()
						}

						// programatically set step inputs
						for (const key of Object.keys(flowModule.value.input_transforms)) {
							const snakeKey = snakeCase(key)
							flowModule.value.input_transforms[key] = {
								type: 'javascript',
								expr:
									key === 'prev_output'
										? isFirstInLoop
											? 'flow_input.iter.value'
											: pastModule
												? 'results.' + pastModule.id
												: 'flow_input.' + snakeKey
										: 'flow_input.' + snakeKey
							}
							$shouldUpdatePropertyType[key] = 'javascript'
						}
					}

					$flowStore = $flowStore // force rerendering
				} else {
					if (
						pastModule !== undefined &&
						pastModule.value.type !== 'rawscript' &&
						pastModule.value.type !== 'script'
					) {
						sendUserToast(
							`Linking to previous step ${pastModule.id} of type ${pastModule.value.type} is not yet supported`,
							true
						)
					} else {
						sendUserToast('Something went wrong, could not generate step inputs', true)
					}
				}
			} catch (err) {
				console.error(err)
			}

			if (stepOnly) {
				$copilotCurrentStepStore = undefined
				copilotLoading = false
				copilotStatus = ''
				if (Object.keys(copilotFlowInputs).length > 0) {
					openCopilotInputsModal = true
				} else {
					finishStepGen()
				}
			} else {
				copilotStatus =
					"Waiting for the user to validate code and inputs of step '" + module.id + "'"
			}
		} catch (err) {
			if (stepOnly) {
				copilotStatus = ''
				$copilotCurrentStepStore = undefined
				setInitCopilotModules(flowCopilotMode)
			}
			if (err?.message) {
				sendUserToast('Failed to generate code: ' + err.message, true)
			} else {
				sendUserToast('Failed to generate code', true)
				console.error(err)
			}
		} finally {
			copilotLoading = false
		}
	}

	flowCopilotContext.genFlow = genFlow

	async function finishCopilotFlowBuilder() {
		copilotLoading = true
		select('Input')
		$copilotCurrentStepStore = 'Input'

		copilotStatus = "Done! Just check the flow's inputs and you're good to go!"
		$copilotCurrentStepStore = undefined
		copilotLoading = false
		await sleep(3000)
		copilotStatus = ''
	}

	function focusCopilot() {
		document.querySelectorAll('.splitpanes__splitter').forEach((el) => {
			el.classList.add('hidden')
		})
		document.querySelectorAll('#flow-graph *').forEach((el) => {
			if (el instanceof HTMLElement) {
				el.style.pointerEvents = 'none'
			}
		})
	}

	function blurCopilot() {
		document.querySelectorAll('.splitpanes__splitter').forEach((el) => {
			el.classList.remove('hidden')
		})
		document.querySelectorAll('#flow-graph *').forEach((el) => {
			if (el instanceof HTMLElement) {
				el.style.pointerEvents = ''
			}
		})
	}

	$: $copilotCurrentStepStore === undefined && blurCopilot()

	let renderCount = 0
	let flowTutorials: FlowTutorials | undefined = undefined

	let jsonViewerDrawer: Drawer | undefined = undefined
	let yamlEditorDrawer: Drawer | undefined = undefined
	let flowHistory: FlowHistory | undefined = undefined

	export function triggerTutorial() {
		const urlParams = new URLSearchParams(window.location.search)
		const tutorial = urlParams.get('tutorial')

		if (tutorial) {
			flowTutorials?.runTutorialById(tutorial)
		} else if ($tutorialsToDo.includes(0) && !$ignoredTutorials.includes(0)) {
			flowTutorials?.runTutorialById('action')
		}
	}

	let moreItems: {
		displayName: string
		icon: ComponentType<Icon>
		action: () => void
		disabled?: boolean
	}[] = []

	$: onCustomUiChange(customUi)

	function onCustomUiChange(customUi: FlowBuilderWhitelabelCustomUi | undefined) {
		moreItems = [
			...(customUi?.topBar?.history != false
				? [
						{
							displayName: 'Deployment History',
							icon: HistoryIcon,
							action: () => {
								flowHistory?.open()
							},
							disabled: newFlow
						}
					]
				: []),
			...(customUi?.topBar?.history != false
				? [
						{
							displayName: 'Export',
							icon: FileJson,
							action: () => jsonViewerDrawer?.openDrawer()
						},
						{
							displayName: 'Edit in YAML',
							icon: FileJson,
							action: () => yamlEditorDrawer?.openDrawer()
						}
					]
				: [])
		]
	}

	let flowPreviewButtons: FlowPreviewButtons
</script>

<svelte:window on:keydown={onKeyDown} />

<slot />

<DeployOverrideConfirmationModal
	bind:deployedBy
	bind:confirmCallback
	bind:open
	{diffDrawer}
	bind:deployedValue
	currentValue={$flowStore}
/>

<DraftTriggersConfirmationModal
	bind:open={draftTriggersModalOpen}
	draftTriggers={$triggersStore.filter((t) => t.draftConfig)}
	on:canceled={() => {
		draftTriggersModalOpen = false
	}}
	on:confirmed={handleDraftTriggersConfirmed}
/>

{#key renderCount}
	{#if !$userStore?.operator}
		<FlowCopilotDrawer {getHubCompletions} {genFlow} bind:flowCopilotMode />
		{#if $pathStore}
			<FlowHistory bind:this={flowHistory} path={$pathStore} on:historyRestore />
		{/if}
		<FlowYamlEditor bind:drawer={yamlEditorDrawer} />
		<FlowImportExportMenu bind:drawer={jsonViewerDrawer} />
		<FlowCopilotInputsModal
			on:confirmed={async () => {
				applyCopilotFlowInputs()
				finishStepGen()
			}}
			on:canceled={async () => {
				clearFlowInputsFromStep($copilotModulesStore[0]?.id)
				finishStepGen()
			}}
			bind:open={openCopilotInputsModal}
			inputs={Object.keys(copilotFlowInputs)}
		/>
		<ScriptEditorDrawer bind:this={$scriptEditorDrawer} />

		<div class="flex flex-col flex-1 h-screen">
			<!-- Nav between steps-->
			<div
				class="justify-between flex flex-row items-center pl-2.5 pr-6 space-x-4 scrollbar-hidden overflow-x-auto max-h-12 h-full relative"
			>
				{#if $copilotCurrentStepStore !== undefined}
					<div transition:fade class="absolute inset-0 bg-gray-500 bg-opacity-75 z-[900] !m-0"
					></div>
				{/if}
				<div class="flex w-full max-w-md gap-4 items-center">
					<Summary
						disabled={customUi?.topBar?.editableSummary == false}
						bind:value={$flowStore.summary}
					/>

					<UndoRedo
						undoProps={{ disabled: $history.index === 0 }}
						redoProps={{ disabled: $history.index === $history.history.length - 1 }}
						on:undo={() => {
							const currentModules = $flowStore?.value?.modules

							$flowStore = undo(history, $flowStore)

							const newModules = $flowStore?.value?.modules
							const restoredModules = newModules?.filter(
								(node) => !currentModules?.some((currentNode) => currentNode?.id === node?.id)
							)

							for (const mod of restoredModules) {
								if (mod) {
									try {
										loadFlowModuleState(mod).then((state) => ($flowStateStore[mod.id] = state))
									} catch (e) {
										console.error('Error loading state for restored node', e)
									}
								}
							}

							$selectedIdStore = 'Input'
						}}
						on:redo={() => {
							$flowStore = redo(history)
						}}
					/>
				</div>

				<div class="gap-4 flex-row hidden md:flex w-full max-w-md">
					{#if $triggersStore?.some((t) => t.type === 'schedule' && !t.isDraft)}
						{@const primarySchedule = $triggersStore.find((t) => t.isPrimary && !t.isDraft)}
						{@const schedule = $triggersStore.find((t) => t.type === 'schedule' && !t.isDraft)}
						<Button
							btnClasses="hidden lg:inline-flex"
							startIcon={{ icon: Calendar }}
							variant="contained"
							color="light"
							size="xs"
							on:click={async () => {
								select('triggers')
								const selected = primarySchedule ?? schedule
								if (selected) {
									$selectedTriggerStore = selected
								}
							}}
						>
							{$primaryScheduleStore != undefined
								? $primaryScheduleStore
									? $primaryScheduleStore?.cron
									: ''
								: $triggersCount?.primary_schedule?.schedule}
						</Button>
					{/if}

					{#if customUi?.topBar?.path != false}
						<div class="flex justify-start w-full">
							<div>
								<button
									on:click={async () => {
										select('settings-metadata')
										document.getElementById('path')?.focus()
									}}
								>
									<Badge
										color="gray"
										class="center-center !bg-gray-300 !text-tertiary dark:!bg-gray-700 dark:!text-gray-300 !h-[28px]  !w-[70px] rounded-r-none"
									>
										<Pen size={12} class="mr-2" /> Path
									</Badge>
								</button>
							</div>
							<input
								type="text"
								readonly
								value={$pathStore && $pathStore != '' ? $pathStore : 'Choose a path'}
								class="font-mono !text-xs !min-w-[96px] !max-w-[300px] !w-full !h-[28px] !my-0 !py-0 !border-l-0 !rounded-l-none"
								on:focus={({ currentTarget }) => {
									currentTarget.select()
								}}
							/>
						</div>
					{/if}
				</div>
				<div class="flex flex-row gap-2 items-center">
					{#if $enterpriseLicense && !newFlow}
						<Awareness />
					{/if}
					<div>
						{#if moreItems?.length > 0}
							<Dropdown items={moreItems} />
						{/if}
					</div>
					{#if customUi?.topBar?.tutorials != false}
						<FlowBuilderTutorials
							on:reload={() => {
								renderCount += 1
							}}
						/>
					{/if}
					{#if customUi?.topBar?.diff != false}
						<Button
							color="light"
							variant="border"
							size="xs"
							on:click={async () => {
								if (!savedFlow) {
									return
								}

								await syncWithDeployed()

								diffDrawer?.openDrawer()
								diffDrawer?.setDiff({
									mode: 'normal',
									deployed: deployedValue ?? savedFlow,
									draft: savedFlow['draft'],
									current: { ...$flowStore, path: $pathStore }
								})
							}}
							disabled={!savedFlow}
						>
							<div class="flex flex-row gap-2 items-center">
								<DiffIcon size={14} />
								Diff
							</div>
						</Button>
					{/if}
					{#if !disableAi && customUi?.topBar?.aiBuilder != false}
						<FlowCopilotStatus
							{copilotLoading}
							bind:copilotStatus
							{genFlow}
							{finishCopilotFlowBuilder}
							{abortController}
						/>
					{/if}
					<FlowPreviewButtons
						on:openTriggers={(e) => {
							select('triggers')
							handleSelectTriggerFromKind(
								triggersStore,
								triggersCount,
								selectedTriggerStore,
								initialPath,
								e.detail.kind
							)
							captureOn.set(true)
							showCaptureHint.set(true)
						}}
						bind:this={flowPreviewButtons}
						{loading}
					/>
					<Button
						loading={loadingDraft}
						size="xs"
						startIcon={{ icon: Save }}
						on:click={() => saveDraft()}
						disabled={(!newFlow && !savedFlow) || loading}
						shortCut={{
							key: 'S'
						}}
					>
						Draft
					</Button>

					<DeployButton
						on:save={async ({ detail }) => await handleSaveFlow(detail)}
						{loading}
						{loadingSave}
						{newFlow}
						{dropdownItems}
					/>
				</div>
			</div>

			<!-- metadata -->
			{#if $flowStateStore}
				<FlowEditor
					{disabledFlowInputs}
					disableAi={disableAi || customUi?.stepInputs?.ai == false}
					disableSettings={customUi?.settingsPanel === false}
					{loading}
					on:reload={() => {
						renderCount += 1
					}}
					{newFlow}
					on:applyArgs={(ev) => {
						if (ev.detail.kind === 'preprocessor') {
							$testStepStore['preprocessor'] = ev.detail.args ?? {}
							$selectedIdStore = 'preprocessor'
						}
					}}
					on:testWithArgs={(e) => {
						$previewArgsStore = JSON.parse(JSON.stringify(e.detail))
						flowPreviewButtons?.openPreview(true)
					}}
					{savedFlow}
				/>
			{:else}
				<CenteredPage>Loading...</CenteredPage>
			{/if}
		</div>
	{:else}
		Flow Builder not available to operators
	{/if}
{/key}

<FlowTutorials
	bind:this={flowTutorials}
	on:reload={() => {
		renderCount += 1
	}}
/>
