<script lang="ts">
	import {
		FlowService,
		type Flow,
		DraftService,
		type PathScript,
		type OpenFlow,
		type InputTransform,
		type TriggersCount,
		CaptureService,
		type Job
	} from '$lib/gen'
	import { initHistory, redo, undo } from '$lib/history.svelte'
	import { enterpriseLicense, userStore, workspaceStore, usedTriggerKinds } from '$lib/stores'
	import {
		cleanValueProperties,
		encodeState,
		generateRandomString,
		orderedJsonStringify,
		readFieldsRecursively,
		replaceFalseWithUndefined,
		isMac,
		type Item,
		type StateStore,
		type Value
	} from '$lib/utils'
	import { sendUserToast } from '$lib/toast'
	import { Drawer } from '$lib/components/common'
	import DeployOverrideConfirmationModal from '$lib/components/common/confirmationModal/DeployOverrideConfirmationModal.svelte'
	import AIChangesWarningModal from '$lib/components/copilot/chat/flow/AIChangesWarningModal.svelte'

	import { createRawSnippet, onMount, setContext, untrack } from 'svelte'
	import { writable } from 'svelte/store'
	import CenteredPage from './CenteredPage.svelte'
	import { Button } from './common'
	import FlowEditor from './flows/FlowEditor.svelte'
	import ScriptEditorDrawer from './flows/content/ScriptEditorDrawer.svelte'
	import FlowEditorDrawer from './flows/content/FlowEditorDrawer.svelte'
	import { dfs as dfsApply } from './flows/dfs'
	import FlowImportExportMenu from './flows/header/FlowImportExportMenu.svelte'
	import FlowPreviewButtons from './flows/header/FlowPreviewButtons.svelte'
	import type { FlowEditorContext, FlowInput, FlowInputEditorState } from './flows/types'
	import { SelectionManager } from './graph/selectionUtils.svelte'
	import { NoteEditor } from './graph/noteEditor.svelte'
	import { setNoteEditorContext } from './graph/noteEditor.svelte'
	import { cleanFlow } from './flows/utils.svelte'
	import {
		Calendar,
		Save,
		DiffIcon,
		HistoryIcon,
		FileJson,
		Settings,
		Undo,
		Redo,
		BookOpen,
		Circle,
		CheckCircle,
		RefreshCw,
		CheckCheck,
		Focus
	} from 'lucide-svelte'
	import Awareness from './Awareness.svelte'
	import { getAllModules } from './flows/flowExplorer'
	import { type FlowCopilotContext } from './copilot/flow'
	import { loadFlowModuleState } from './flows/flowStateUtils.svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import FlowTutorials from './FlowTutorials.svelte'
	import FlowHistory from './flows/FlowHistory.svelte'
	import { resetAllTodos, skipAllTodos } from '$lib/tutorialUtils'
	import { tutorialsToDo } from '$lib/stores'
	import { getTutorialIndex } from '$lib/tutorials/config'
	import SummaryPathDisplay from './SummaryPathDisplay.svelte'
	import type { FlowBuilderWhitelabelCustomUi } from './custom_ui'
	import FlowYamlEditor from './flows/header/FlowYamlEditor.svelte'
	import { type TriggerContext, type ScheduleTrigger } from './triggers'
	import type { SavedAndModifiedValue } from './common/confirmationModal/unsavedTypes'
	import DeployButton from './DeployButton.svelte'
	import type { FlowWithDraftAndDraftTriggers, Trigger } from './triggers/utils'
	import {
		deployTriggers,
		filterDraftTriggers,
		handleSelectTriggerFromKind
	} from './triggers/utils'
	import DraftTriggersConfirmationModal from './common/confirmationModal/DraftTriggersConfirmationModal.svelte'
	import { Triggers } from './triggers/triggers.svelte'
	import { StepsInputArgs } from './flows/stepsInputArgs.svelte'
	import { aiChatManager } from './copilot/chat/AIChatManager.svelte'
	import type { GraphModuleState } from './graph'
	import { validateRetryConfig } from '$lib/utils'
	import {
		setStepHistoryLoaderContext,
		StepHistoryLoader,
		type stepState
	} from './stepHistoryLoader.svelte'
	import type { FlowBuilderProps } from './flow_builder'
	import { ModulesTestStates } from './modulesTest.svelte'
	import FlowAssetsHandler, { initFlowGraphAssetsCtx } from './flows/FlowAssetsHandler.svelte'
	import { isRuleActive } from '$lib/workspaceProtectionRules.svelte'
	import { buildForkEditUrl } from '$lib/utils/editInFork'

	let {
		initialPath = $bindable(''),
		pathStoreInit = undefined,
		newFlow,
		selectedId,
		initialArgs = {},
		loading = false,
		flowStore,
		flowStateStore,
		savedFlow = $bindable(undefined),
		diffDrawer = undefined,
		customUi = {},
		disableAi = false,
		disabledFlowInputs = false,
		savedPrimarySchedule = undefined,
		version = undefined,
		setSavedraftCb = undefined,
		draftTriggersFromUrl = undefined,
		selectedTriggerIndexFromUrl = undefined,
		children,
		loadedFromHistoryFromUrl,
		noInitial = false,
		onSaveInitial,
		onSaveDraft,
		onDeploy,
		onDeployError,
		onDetails,
		onSaveDraftError,
		onSaveDraftOnlyAtNewPath,
		onHistoryRestore
	}: FlowBuilderProps = $props()

	let initialPathStore = writable(initialPath)

	// For preserve_on_behalf_of feature
	let preserveOnBehalfOf = writable(false)
	let savedOnBehalfOfEmail = writable<string | undefined>(savedFlow?.on_behalf_of_email)

	// used for new flows for captures
	let fakeInitialPath =
		'u/' +
		($userStore?.username?.includes('@')
			? $userStore!.username.split('@')[0].replace(/[^a-zA-Z0-9_]/g, '')
			: $userStore?.username) +
		'/' +
		generateRandomString(12)

	// Used by multiplayer deploy collision warning
	let deployedValue: Value | undefined = $state(undefined) // Value to diff against
	let deployedBy: string | undefined = $state(undefined) // Author
	let confirmCallback: () => void = $state(() => {}) // What happens when user clicks `override` in warning
	let open: boolean = $state(false) // Is confirmation modal open

	// Draft triggers confirmation modal
	let draftTriggersModalOpen = $state(false)
	let confirmDeploymentCallback: (triggersToDeploy: Trigger[]) => void = () => {}

	// AI changes warning modal
	let aiChangesWarningOpen = $state(false)
	let aiChangesConfirmCallback = $state<() => void>(() => {})

	// Flow preview
	let flowPreviewButtons: FlowPreviewButtons | undefined = $state()
	const flowPreviewContent = $derived(flowPreviewButtons?.getFlowPreviewContent())
	const job: Job | undefined = $derived(flowPreviewContent?.getJob())
	let showJobStatus = $state(false)

	async function handleDraftTriggersConfirmed(event: CustomEvent<{ selectedTriggers: Trigger[] }>) {
		const { selectedTriggers } = event.detail
		// Continue with saving the flow
		draftTriggersModalOpen = false
		confirmDeploymentCallback(selectedTriggers)
	}

	function hasAIChanges(): boolean {
		return aiChatManager.flowAiChatHelpers?.hasPendingChanges() ?? false
	}

	function withAIChangesWarning(callback: () => void) {
		if (hasAIChanges()) {
			aiChangesConfirmCallback = () => {
				aiChatManager.flowAiChatHelpers?.rejectAllModuleActions()
				callback()
			}
			aiChangesWarningOpen = true
		} else {
			callback()
		}
	}

	export function getInitialAndModifiedValues(): SavedAndModifiedValue {
		return {
			savedValue: savedFlow,
			modifiedValue: {
				...flowStore.val,
				draft_triggers: structuredClone(triggersState.getDraftTriggersSnapshot())
			}
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

	const primaryScheduleStore = writable<ScheduleTrigger | undefined | false>(savedPrimarySchedule) // kept for legacy reasons
	const triggersCount = writable<TriggersCount | undefined>(undefined)
	const simplifiedPoll = writable(false)

	// used to set the primary schedule in the legacy primaryScheduleStore
	export function setPrimarySchedule(schedule: ScheduleTrigger | undefined | false) {
		primaryScheduleStore.set(schedule)
	}

	export function setDraftTriggers(triggers: Trigger[] | undefined) {
		triggersState.setTriggers([
			...(triggers ?? []),
			...triggersState.triggers.filter((t) => !t.draftConfig)
		])
		loadTriggers()
	}

	export function setSelectedTriggerIndex(index: number | undefined) {
		triggersState.selectedTriggerIndex = index
	}

	let loadingSave = $state(false)
	let loadingDraft = $state(false)

	export async function saveDraft(forceSave = false): Promise<void> {
		withAIChangesWarning(async () => {
			await saveDraftInternal(forceSave)
		})
	}

	async function saveDraftInternal(forceSave = false): Promise<void> {
		if (!newFlow && !savedFlow) {
			return
		}

		if (savedFlow) {
			const draftOrDeployed = cleanValueProperties(savedFlow.draft || savedFlow)
			const currentDraftTriggers = structuredClone(triggersState.getDraftTriggersSnapshot())
			const current = cleanValueProperties(
				$state.snapshot({
					...flowStore.val,
					path: $pathStore,
					draft_triggers: currentDraftTriggers
				})
			)
			if (!forceSave && orderedJsonStringify(draftOrDeployed) === orderedJsonStringify(current)) {
				sendUserToast('No changes detected, ignoring', false, [
					{
						label: 'Save anyway',
						callback: () => {
							saveDraftInternal(true)
						}
					}
				])
				return
			}
		}
		loadingDraft = true
		try {
			const flow = cleanFlow(flowStore.val)
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
						draft_triggers: triggersState.getDraftTriggersSnapshot()
					}
				}
			})

			savedFlow = {
				...(newFlow || savedFlow?.draft_only
					? {
							...structuredClone($state.snapshot(flowStore.val)),
							path: $pathStore,
							draft_only: true
						}
					: savedFlow),
				draft: {
					...structuredClone($state.snapshot(flowStore.val)),
					path: $pathStore,
					draft_triggers: structuredClone(triggersState.getDraftTriggersSnapshot())
				}
			} as FlowWithDraftAndDraftTriggers

			let savedAtNewPath = false
			if (newFlow) {
				onSaveInitial?.({ path: $pathStore, id: getSelectedId() ?? 'settings' })
			} else if (savedFlow?.draft_only && $pathStore !== initialPath) {
				savedAtNewPath = true
				initialPath = $pathStore
				onSaveDraftOnlyAtNewPath?.({ path: $pathStore, selectedId: getSelectedId() ?? 'settings' })
				// this is so we can use the flow builder outside of sveltekit
			}
			onSaveDraft?.({ path: $pathStore, savedAtNewPath, newFlow })
			sendUserToast('Saved as draft')
		} catch (error) {
			sendUserToast(`Error while saving the flow as a draft: ${error.body || error.message}`, true)
			onSaveDraftError?.({ error })
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
		withAIChangesWarning(async () => {
			await handleSaveFlowInternal(deploymentMsg)
		})
	}

	async function handleSaveFlowInternal(deploymentMsg?: string) {
		await compareVersions()
		if (onLatest || initialPath == '' || savedFlow?.draft_only) {
			// Handle directly
			await saveFlow(deploymentMsg)
		} else {
			// We need it for diff
			await syncWithDeployed()

			if (
				deployedValue &&
				flowStore.val &&
				orderedJsonStringify(deployedValue) ===
					orderedJsonStringify(replaceFalseWithUndefined({ ...flowStore.val, path: $pathStore }))
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
			const draftTriggers = triggersState.triggers.filter((trigger) => trigger.draftConfig)
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
			const flow = cleanFlow(flowStore.val)

			if (flow.value?.modules) {
				const validationErrors: string[] = []
				dfsApply(flow.value.modules, (module) => {
					const error = validateRetryConfig(module.retry)
					if (error) {
						validationErrors.push(`Step '${module.id}': ${error}`)
					}
				})

				if (flow.value.failure_module) {
					// add validation logic here for failure module
				}

				if (flow.value.preprocessor_module) {
					// add validation logic here for preprocessor module
				}

				if (validationErrors.length > 0) {
					throw new Error(validationErrors.join('\n'))
				}
			}
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
						preserve_on_behalf_of: $preserveOnBehalfOf || undefined,
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
						$pathStore,
						true
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
						preserve_on_behalf_of: $preserveOnBehalfOf || undefined,
						deployment_message: deploymentMsg || undefined
					}
				})
			}

			const { draft_triggers: _, ...newSavedFlow } = flowStore.val as OpenFlow & {
				draft_triggers: Trigger[]
			}
			savedFlow = {
				...structuredClone($state.snapshot(newSavedFlow)),
				path: $pathStore
			} as Flow
			setDraftTriggers([])
			loadingSave = false
			onDeploy?.({ path: $pathStore })
		} catch (err) {
			onDeployError?.({ error: err })
			// this is so we can use the flow builder outside of sveltekit
			sendUserToast(`The flow could not be saved: ${err.body ?? err}`, true)
			loadingSave = false
		}
	}

	let timeout: number | undefined = undefined

	function saveSessionDraft() {
		timeout && clearTimeout(timeout)
		timeout = window.setTimeout(() => {
			try {
				localStorage.setItem(
					initialPath && initialPath != '' ? `flow-${initialPath}` : 'flow',
					encodeState({
						flow: flowStore.val,
						path: $pathStore,
						selectedId: selectedIdStore,
						draft_triggers: triggersState.getDraftTriggersSnapshot(),
						selected_trigger: triggersState.getSelectedTriggerSnapshot(),
						loadedFromHistory: {
							flowJobInitial: stepHistoryLoader.flowJobInitial,
							stepsState: stepHistoryLoader.stepStates
						}
					})
				)
			} catch (err) {
				console.error(err)
			}
		}, 500)
	}

	const selectionManager = new SelectionManager()
	const selectedIdStore = $derived(selectionManager.getSelectedId())
	// Initialize with selected id if provided
	if (selectedId) {
		selectionManager.selectId(selectedId)
	} else {
		selectionManager.selectId('settings-metadata')
	}

	export function getSelectedId() {
		return selectedIdStore
	}

	const previewArgsStore = $state({ val: initialArgs })
	const scriptEditorDrawer = writable<ScriptEditorDrawer | undefined>(undefined)
	const flowEditorDrawer = writable<FlowEditorDrawer | undefined>(undefined)
	const moving = writable<{ id: string } | undefined>(undefined)
	const history = initHistory(flowStore.val)
	const pathStore = writable<string>(pathStoreInit ?? initialPath)
	const captureOn = writable<boolean>(false)
	const showCaptureHint = writable<boolean | undefined>(undefined)
	const flowInputEditorStateStore = writable<FlowInputEditorState>({
		selectedTab: undefined,
		editPanelSize: 0,
		payloadData: undefined
	})

	const stepsInputArgs = new StepsInputArgs()

	function select(selectedId: string) {
		selectionManager.selectId(selectedId)
	}

	let insertButtonOpen = writable<boolean>(false)
	let modulesTestStates = new ModulesTestStates()
	let outputPickerOpenFns: Record<string, () => void> = $state({})
	let flowEditor: FlowEditor | undefined = $state(undefined)

	setContext<FlowEditorContext>('FlowEditorContext', {
		selectionManager,
		currentEditor: writable(undefined),
		previewArgs: previewArgsStore,
		scriptEditorDrawer,
		flowEditorDrawer,
		moving,
		history,
		flowStateStore,
		flowStore,
		pathStore,
		stepsInputArgs,
		saveDraft,
		initialPathStore,
		fakeInitialPath,
		flowInputsStore: writable<FlowInput>({}),
		customUi,
		insertButtonOpen,
		executionCount: writable(0),
		flowInputEditorState: flowInputEditorStateStore,
		modulesTestStates,
		outputPickerOpenFns,
		preserveOnBehalfOf,
		savedOnBehalfOfEmail
	})

	// Set up NoteEditor context for note editing capabilities
	const noteEditor = new NoteEditor(flowStore, () => {
		// Enable notes display when a note is created
		flowEditor?.enableNotes?.()
	})
	setNoteEditorContext(noteEditor)

	setContext(
		'FlowGraphAssetContext',
		initFlowGraphAssetsCtx({ getModules: () => flowStore.val.value.modules })
	)

	// Add triggers context store
	const triggersState = $state(
		new Triggers(
			[
				{ type: 'webhook', path: '', isDraft: false },
				{ type: 'default_email', path: '', isDraft: false },
				...(draftTriggersFromUrl ?? savedFlow?.draft?.draft_triggers ?? [])
			],
			selectedTriggerIndexFromUrl,
			saveSessionDraft
		)
	)

	setContext<TriggerContext>('TriggerContext', {
		triggersCount,
		simplifiedPoll,
		showCaptureHint,
		triggersState
	})

	export async function loadTriggers() {
		if (initialPath == '') return
		$triggersCount = await FlowService.getTriggersCountOfFlow({
			workspace: $workspaceStore!,
			path: initialPath
		})

		// Initialize triggers using utility function
		await triggersState.fetchTriggers(
			triggersCount,
			$workspaceStore,
			initialPath,
			true,
			$primaryScheduleStore,
			$userStore
		)

		if (savedFlow && savedFlow.draft) {
			savedFlow = filterDraftTriggers(savedFlow, triggersState) as FlowWithDraftAndDraftTriggers
		}
	}

	function handleUndo() {
		const currentModules = flowStore.val?.value?.modules
		flowStore.val = undo(history, flowStore.val)
		const newModules = flowStore.val?.value?.modules
		const restoredModules = newModules?.filter(
			(node) => !currentModules?.some((currentNode) => currentNode?.id === node?.id)
		)
		for (const mod of restoredModules) {
			if (mod) {
				try {
					loadFlowModuleState(mod).then((state) => (flowStateStore.val[mod.id] = state))
				} catch (e) {
					console.error('Error loading state for restored node', e)
				}
			}
		}
		selectionManager.selectId('Input')
	}

	function handleRedo() {
		flowStore.val = redo(history)
	}

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
					handleRedo()
					event.preventDefault()
				}
				break
			case 'z':
				if (event.ctrlKey || event.metaKey) {
					handleUndo()
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
				if (!$insertButtonOpen && !flowPreviewButtons?.getPreviewOpen()) {
					let ids = generateIds()
					let idx = ids.indexOf(selectedIdStore!)
					if (idx > -1 && idx < ids.length - 1) {
						selectionManager.selectId(ids[idx + 1])
						event.preventDefault()
					}
				}
				break
			}
			case 'ArrowUp': {
				if (!$insertButtonOpen && !flowPreviewButtons?.getPreviewOpen()) {
					let ids = generateIds()
					let idx = ids.indexOf(selectedIdStore!)
					if (idx > 0 && idx < ids.length) {
						selectionManager.selectId(ids[idx - 1])
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
			...dfsApply(flowStore.val.value.modules, (module) => module.id)
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
				onClick: () => onDetails?.({ path: $pathStore })
			})
		}

		if (!newFlow) {
			dropdownItems.push({
				label: 'Fork',
				onClick: () => window.open(`/flows/add?template=${initialPath}`)
			})
		}

		if (!newFlow && !isRuleActive('DisableWorkspaceForking')) {
			dropdownItems.push({
				label: 'Edit in workspace fork',
				onClick: () => window.open(buildForkEditUrl('flow', initialPath))
			})
		}
	}

	let flowCopilotContext: FlowCopilotContext = $state({
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
	})

	setContext('FlowCopilotContext', flowCopilotContext)

	let renderCount = $state(0)
	let flowTutorials: FlowTutorials | undefined = $state(undefined)

	let jsonViewerDrawer: Drawer | undefined = $state(undefined)
	let yamlEditorDrawer: Drawer | undefined = $state(undefined)
	let flowHistory: FlowHistory | undefined = $state(undefined)

	export function triggerTutorial() {
		const urlParams = new URLSearchParams(window.location.search)
		const tutorial = urlParams.get('tutorial')

		if (tutorial) {
			flowTutorials?.runTutorialById(tutorial)
		}
	}

	let baseMenuItems: Item[] = $state([])

	const mod = isMac() ? '⌘' : 'Ctrl+'

	const undoShortcutSnippet = createRawSnippet(() => ({
		render: () => `<span class="ml-auto text-2xs text-tertiary">${mod}Z</span>`
	}))

	const redoShortcutSnippet = createRawSnippet(() => ({
		render: () => `<span class="ml-auto text-2xs text-tertiary">${mod}⇧Z</span>`
	}))

	function getMoreItems(): Item[] {
		return [
			...baseMenuItems,
			{
				displayName: 'Undo',
				icon: Undo,
				action: () => handleUndo(),
				disabled: $history.index === 0,
				extra: undoShortcutSnippet,
				separatorTop: baseMenuItems.length > 0
			},
			{
				displayName: 'Redo',
				icon: Redo,
				action: () => handleRedo(),
				disabled: $history.index === $history.history.length - 1,
				extra: redoShortcutSnippet
			},
			{
				displayName: 'Tutorials',
				icon: BookOpen,
				separatorTop: true,
				extra: (() => {
					const remaining = [
						getTutorialIndex('flow-live-tutorial'),
						getTutorialIndex('troubleshoot-flow')
					].filter((i) => $tutorialsToDo.includes(i)).length
					return remaining > 0
						? createRawSnippet(() => ({
								render: () =>
									`<span class="ml-auto inline-flex items-center justify-center w-4 h-4 text-[10px] font-medium text-white rounded-full bg-surface-accent-primary">${remaining}</span>`
							}))
						: undefined
				})(),
				submenuItems: [
					{
						displayName: 'Build a flow',
						action: () => flowTutorials?.runTutorialById('flow-live-tutorial'),
						icon: $tutorialsToDo.includes(getTutorialIndex('flow-live-tutorial'))
							? Circle
							: CheckCircle,
						iconColor: $tutorialsToDo.includes(getTutorialIndex('flow-live-tutorial'))
							? undefined
							: 'green'
					},
					{
						displayName: 'Fix a broken flow',
						action: () => flowTutorials?.runTutorialById('troubleshoot-flow'),
						icon: $tutorialsToDo.includes(getTutorialIndex('troubleshoot-flow'))
							? Circle
							: CheckCircle,
						iconColor: $tutorialsToDo.includes(getTutorialIndex('troubleshoot-flow'))
							? undefined
							: 'green'
					},
					{
						displayName: 'Reset tutorials',
						action: () => resetAllTodos(),
						icon: RefreshCw,
						separatorTop: true
					},
					{
						displayName: 'Skip tutorials',
						action: () => skipAllTodos(),
						icon: CheckCheck
					}
				]
			},
			{
				displayName: 'Test flow & record',
				icon: Focus,
				action: () => flowPreviewButtons?.openRecordingPreview()
			}
		]
	}

	function onCustomUiChange(
		customUi: FlowBuilderWhitelabelCustomUi | undefined,
		hasAiDiff: boolean
	) {
		baseMenuItems = [
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
			...(customUi?.topBar?.export != false
				? [
						{
							displayName: 'Export',
							icon: FileJson,
							action: () => jsonViewerDrawer?.openDrawer()
						},
						{
							displayName: 'Edit in YAML',
							icon: FileJson,
							action: () => yamlEditorDrawer?.openDrawer(),
							disabled: hasAiDiff
						}
					]
				: []),
			...(customUi?.topBar?.settings != false
				? [
						{
							displayName: 'Flow settings',
							icon: Settings,
							action: () => {
								select('settings-metadata')
							}
						}
					]
				: [])
		]
	}

	function handleDeployTrigger(trigger: Trigger) {
		const { id, path, type } = trigger
		//Update the saved flow to remove the draft trigger that is deployed
		if (savedFlow && savedFlow.draft && savedFlow.draft.draft_triggers) {
			const newSavedDraftTrigers = savedFlow.draft.draft_triggers.filter(
				(t) => t.id !== id || t.path !== path || t.type !== type
			)
			savedFlow.draft.draft_triggers =
				newSavedDraftTrigers.length > 0 ? newSavedDraftTrigers : undefined
		}
	}

	let forceTestTab: Record<string, boolean> = $state({})
	let highlightArg: Record<string, string | undefined> = $state({})

	$effect.pre(() => {
		initialPathStore.set(initialPath)
	})
	$effect.pre(() => {
		setContext('customUi', customUi)
	})
	$effect.pre(() => {
		if (flowStore.val || selectedIdStore) {
			readFieldsRecursively(flowStore.val)
			untrack(() => saveSessionDraft())
		}
	})
	$effect.pre(() => {
		initialPath && ($pathStore = initialPath)
	})
	$effect.pre(() => {
		selectedId && untrack(() => select(selectedId))
	})
	$effect.pre(() => {
		initialPath && initialPath != '' && $workspaceStore && untrack(() => loadTriggers())
	})
	$effect.pre(() => {
		const hasAiDiff = aiChatManager.flowAiChatHelpers?.hasPendingChanges() ?? false
		customUi && untrack(() => onCustomUiChange(customUi, hasAiDiff))
	})

	export async function loadFlowState() {
		await stepHistoryLoader.loadIndividualStepsStates(
			flowStore.val as Flow,
			flowStateStore,
			$workspaceStore!,
			$initialPathStore,
			$pathStore
		)
	}

	let stepHistoryLoader = new StepHistoryLoader(
		loadedFromHistoryFromUrl?.stepsState ?? {},
		loadedFromHistoryFromUrl?.flowJobInitial,
		saveSessionDraft,
		noInitial
	)
	setStepHistoryLoaderContext(stepHistoryLoader)

	export function setLoadedFromHistory(
		loadedFromHistoryUrl:
			| {
					flowJobInitial: boolean | undefined
					stepsState: Record<string, stepState>
			  }
			| undefined
	) {
		if (!loadedFromHistoryUrl) {
			return
		}

		stepHistoryLoader.setFlowJobInitial(loadedFromHistoryUrl.flowJobInitial)
		stepHistoryLoader.stepStates = loadedFromHistoryUrl.stepsState
	}

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
				if (flowEditor?.isNodeVisible('Result') && selectedIdStore !== 'Result') {
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
					flowEditor?.isNodeVisible(lastModuleWithJob.id)
				) {
					outputPickerOpenFns[lastModuleWithJob.id]?.()
				}
			}
		}
	}

	let localModuleStates: Record<string, GraphModuleState> = $state({})
	let suspendStatus: StateStore<Record<string, { job: Job; nb: number }>> = $state({ val: {} })

	const flowHasChanged = $derived(flowPreviewContent?.flowHasChanged())
</script>

<svelte:window onkeydown={onKeyDown} />

{@render children?.()}

<DeployOverrideConfirmationModal
	{deployedBy}
	{confirmCallback}
	bind:open
	{diffDrawer}
	bind:deployedValue
	currentValue={flowStore.val}
/>

<DraftTriggersConfirmationModal
	bind:open={draftTriggersModalOpen}
	draftTriggers={triggersState.triggers.filter((t) => t.draftConfig)}
	isFlow={true}
	on:canceled={() => {
		draftTriggersModalOpen = false
	}}
	on:confirmed={handleDraftTriggersConfirmed}
/>

<AIChangesWarningModal bind:open={aiChangesWarningOpen} onConfirm={aiChangesConfirmCallback} />

{#key renderCount}
	{#if !$userStore?.operator}
		{#if $pathStore}
			<FlowHistory bind:this={flowHistory} path={$pathStore} {onHistoryRestore} />
		{/if}
		<FlowYamlEditor bind:drawer={yamlEditorDrawer} />
		<FlowImportExportMenu bind:drawer={jsonViewerDrawer} />
		<ScriptEditorDrawer bind:this={$scriptEditorDrawer} />
		<FlowEditorDrawer bind:this={$flowEditorDrawer} />

		<div class="flex flex-col flex-1 h-screen">
			<!-- Nav between steps-->
			<div
				class="justify-between flex flex-row items-center pl-2 pr-4 space-x-4 scrollbar-hidden overflow-x-auto max-h-12 h-full relative"
			>
				<div class="flex w-full gap-8 items-center min-w-0">
					<SummaryPathDisplay
						bind:summary={flowStore.val.summary}
						bind:path={$pathStore}
						kind="flow"
						editable
					/>
				</div>

				<div class="gap-4 flex-row hidden md:flex whitespace-nowrap">
					{#if triggersState.triggers?.some((t) => t.type === 'schedule')}
						{@const primaryScheduleIndex = triggersState.triggers.findIndex((t) => t.isPrimary)}
						{@const scheduleIndex = triggersState.triggers.findIndex((t) => t.type === 'schedule')}
						<Button
							btnClasses="hidden lg:inline-flex"
							startIcon={{ icon: Calendar }}
							variant="subtle"
							size="xs"
							on:click={async () => {
								select('Trigger')
								const selected = primaryScheduleIndex ?? scheduleIndex
								if (selected) {
									triggersState.selectedTriggerIndex = selected
								}
							}}
						>
							{triggersState.triggers[primaryScheduleIndex]?.draftConfig?.schedule ??
								triggersState.triggers[primaryScheduleIndex]?.lightConfig?.schedule ??
								''}
						</Button>
					{/if}
				</div>
				<div class="flex flex-row gap-2 items-center">
					{#if $enterpriseLicense && !newFlow}
						<Awareness />
					{/if}
					<div class="relative">
						<Dropdown items={getMoreItems} />
						{#if $tutorialsToDo.includes(getTutorialIndex('flow-live-tutorial')) || $tutorialsToDo.includes(getTutorialIndex('troubleshoot-flow'))}
							<span
								class="absolute top-0.5 right-0.5 block w-2 h-2 rounded-full bg-surface-accent-primary pointer-events-none"
							></span>
						{/if}
					</div>
					{#if customUi?.topBar?.diff != false}
						<Button
							variant="default"
							unifiedSize="md"
							on:click={async () => {
								if (!savedFlow) {
									return
								}

								await syncWithDeployed()

								const currentDraftTriggers = structuredClone(
									triggersState.getDraftTriggersSnapshot()
								)

								diffDrawer?.openDrawer()
								const currentFlow = flowStore.val
								diffDrawer?.setDiff({
									mode: 'normal',
									deployed: deployedValue ?? savedFlow,
									draft: savedFlow?.draft,
									current: {
										...currentFlow,
										path: $pathStore,
										draft_triggers: currentDraftTriggers
									}
								})
							}}
							disabled={!savedFlow}
							startIcon={{ icon: DiffIcon }}
						>
							Diff
						</Button>
					{/if}
					<FlowPreviewButtons
						{suspendStatus}
						on:openTriggers={(e) => {
							select('Trigger')
							handleSelectTriggerFromKind(triggersState, triggersCount, initialPath, e.detail.kind)
							captureOn.set(true)
							showCaptureHint.set(true)
						}}
						{onJobDone}
						bind:localModuleStates
						bind:this={flowPreviewButtons}
						{loading}
						onRunPreview={() => {
							// Reset manually edited args inputs when running a preview
							stepsInputArgs.resetManuallyEditedArgs()
							modulesTestStates.hideJobsInGraph()
							localModuleStates = {}
							showJobStatus = true
						}}
					/>
					{#if customUi?.topBar?.draft !== false}
						<Button
							loading={loadingDraft}
							unifiedSize="md"
							variant="accent"
							startIcon={{ icon: Save }}
							on:click={() => saveDraft()}
							disabled={(!newFlow && !savedFlow) || loading}
							shortCut={{ key: 'S' }}
						>
							Draft
						</Button>
					{/if}

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
			{#if flowStateStore.val}
				<FlowEditor
					bind:this={flowEditor}
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
							stepsInputArgs.setStepArgs('preprocessor', ev.detail.args ?? {})
							selectionManager.selectId('preprocessor')
						}
					}}
					on:testWithArgs={(e) => {
						previewArgsStore.val = JSON.parse(JSON.stringify(e.detail))
						flowPreviewButtons?.openPreview(true)
					}}
					onTestUpTo={(id) => {
						flowPreviewButtons?.testUpTo(id)
					}}
					{savedFlow}
					onDeployTrigger={handleDeployTrigger}
					onEditInput={(moduleId, key) => {
						selectionManager.selectId(moduleId)
						// Use new prop-based system
						forceTestTab[moduleId] = true
						highlightArg[moduleId] = key
						// Reset the force flag after a short delay to allow re-triggering
						setTimeout(() => {
							forceTestTab[moduleId] = false
							highlightArg[moduleId] = undefined
						}, 500)
					}}
					{forceTestTab}
					{highlightArg}
					aiChatOpen={aiChatManager.open}
					showFlowAiButton={!disableAi && customUi?.topBar?.aiBuilder != false}
					toggleAiChat={() => aiChatManager.toggleOpen()}
					onOpenPreview={flowPreviewButtons?.openPreview}
					localModuleStates={showJobStatus ? localModuleStates : {}}
					{showJobStatus}
					testModuleStates={modulesTestStates}
					isOwner={flowPreviewContent?.getIsOwner()}
					onTestFlow={flowPreviewButtons?.runPreview}
					isRunning={flowPreviewContent?.getIsRunning()}
					onCancelTestFlow={flowPreviewContent?.cancelTest}
					onHideJobStatus={() => {
						modulesTestStates.hideJobsInGraph()
						showJobStatus = false
					}}
					{job}
					{suspendStatus}
					onDelete={(id) => {
						delete localModuleStates[id]
						delete modulesTestStates.states[id]
					}}
					{flowHasChanged}
					previewOpen={flowPreviewButtons?.getPreviewOpen()}
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

<FlowAssetsHandler
	modules={flowStore.val.value.modules}
	enableParser
	enableDbExplore
	enablePathScriptAndFlowAssets
/>
