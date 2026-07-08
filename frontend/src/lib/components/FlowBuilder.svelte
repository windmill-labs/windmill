<script lang="ts">
	import {
		FlowService,
		type Flow,
		type PathScript,
		type OpenFlow,
		type InputTransform,
		type TriggersCount,
		CaptureService,
		type Job
	} from '$lib/gen'
	import { initHistory, redo, undo } from '$lib/history.svelte'
	import {
		enterpriseLicense,
		userStore,
		userWorkspaces,
		workspaceStore,
		usedTriggerKinds
	} from '$lib/stores'
	import {
		generateRandomString,
		orderedJsonStringify,
		readFieldsRecursively,
		replaceFalseWithUndefined,
		isMac,
		userPathPrefix,
		type Item,
		type StateStore,
		type Value
	} from '$lib/utils'
	import { sendUserToast } from '$lib/toast'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import { Drawer } from '$lib/components/common'
	import DeployOverrideConfirmationModal from '$lib/components/common/confirmationModal/DeployOverrideConfirmationModal.svelte'
	import AIChangesWarningModal from '$lib/components/copilot/chat/flow/AIChangesWarningModal.svelte'

	import { createRawSnippet, setContext, untrack } from 'svelte'
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
	import { GroupEditor, setGroupEditorContext } from './graph/groupEditor.svelte'
	import { cleanFlow } from './flows/utils.svelte'
	import {
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
		Disc
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
	import EditorHeader from './EditorHeader.svelte'
	import AutosaveIndicator from './AutosaveIndicator.svelte'
	import type { FlowBuilderWhitelabelCustomUi } from './custom_ui'
	import FlowYamlEditor from './flows/header/FlowYamlEditor.svelte'
	import { type TriggerContext, type ScheduleTrigger } from './triggers'
	import type { SavedAndModifiedValue } from './common/confirmationModal/unsavedTypes'
	import DeployButton from './DeployButton.svelte'
	import { invalidateWorkspacePaths } from './PathNameAutocomplete.svelte'
	import type { Trigger } from './triggers/utils'
	import { deployTriggers, handleSelectTriggerFromKind } from './triggers/utils'
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
	import { buildForkEditUrl, editInForkAllowed, editInForkLabel } from '$lib/utils/editInFork'
	import { isCloudHosted } from '$lib/cloud'
	import { UserDraft } from '$lib/userDraft.svelte'

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
		draftBaseVersion = undefined,
		draftTriggersFromUrl = undefined,
		selectedTriggerIndexFromUrl = undefined,
		children,
		loadedFromHistoryFromUrl,
		noInitial = false,
		liveEditorDraftStoragePath = undefined,
		autosaveWorkspace = undefined,
		autosavePath = undefined,
		onDeploy,
		onDeployError,
		onDetails,
		onHistoryRestore,
		onNavigate,
		onResetToDeployed,
		loadedFromDraft = false,
		othersDraftsCount = 0,
		onOpenOthersDrafts,
		onTestJob,
		condensedHeader = false
	}: FlowBuilderProps = $props()

	// Top-bar button size + bar height. Condensed (session preview) uses the
	// smallest well-supported unified size (`sm`) so the bar is thinner.
	const headerBtnSize = $derived(condensedHeader ? 'sm' : 'md')

	// The workspace this editor operates on: deploy, save-draft, trigger loading
	// and the AutosaveIndicator all target it. Falls back to the global store, so
	// the full-page editor is unchanged; the sessions preview overrides it to the
	// session's (forked) workspace, so an embedded editor acts on the session's
	// fork rather than the navigation workspace ($workspaceStore, which stays put).
	// indicatorPath is the matching draft path.
	const opWorkspace = $derived(autosaveWorkspace ?? $workspaceStore)
	const indicatorPath = $derived(autosavePath ?? liveEditorDraftStoragePath)

	let initialPathStore = writable(initialPath)

	// For preserve_on_behalf_of feature
	let preserveOnBehalfOf = writable(false)
	let savedOnBehalfOfEmail = writable<string | undefined>(savedFlow?.on_behalf_of_email)

	// Keep savedOnBehalfOfEmail in sync when savedFlow is loaded asynchronously
	$effect(() => {
		if (savedFlow?.on_behalf_of_email !== undefined) {
			savedOnBehalfOfEmail.set(savedFlow.on_behalf_of_email)
		}
	})

	// used for new flows for captures
	let fakeInitialPath = userPathPrefix($userStore?.username) + generateRandomString(12)

	// Used by multiplayer deploy collision warning
	let deployedValue: Value | undefined = $state(undefined) // Value to diff against
	let deployedBy: string | undefined = $state(undefined) // Author
	let confirmCallback: () => void = $state(() => {}) // What happens when user clicks `override` in warning
	let open: boolean = $state(false) // Is confirmation modal open

	// Draft triggers confirmation modal
	let draftTriggersModalOpen = $state(false)

	// Top-bar responsive collapse. Measured via bind:clientWidth — we can't
	// rely on viewport `md:` because the editor lives inside other panes
	// (session pane, drawer, etc.) where the viewport stays wide.
	let topbarWidth = $state(0)
	const compactTopbar = $derived(topbarWidth > 0 && topbarWidth < 720)
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
				// `$pathStore` is the live-edited path (the pen popover binds it).
				// `flowStore.val.path` doesn't track those edits, so without this the
				// rename wouldn't show up in the diff and the unsaved-changes warning
				// wouldn't fire when leaving with a pending rename.
				path: $pathStore,
				draft_triggers: structuredClone(triggersState.getDraftTriggersSnapshot())
			}
		}
	}
	let onLatest = true
	async function compareVersions() {
		// Compare the draft's pinned fork base against the current head when editing
		// a draft, else the load-time head. This catches both a concurrent deploy
		// (head moved since open) AND a stale draft reopened after a deploy (head ==
		// load-time head, but the draft was forked from an older version).
		const base = draftBaseVersion ?? version
		if (base === undefined) {
			return
		}
		try {
			if (initialPath && initialPath != '') {
				const flowVersion = await FlowService.getFlowLatestVersion({
					workspace: opWorkspace!,
					path: initialPath
				})

				onLatest = base === flowVersion?.id
			} else {
				onLatest = true
			}
		} catch (err) {
			console.error('Error comparing versions', err)
			onLatest = true
		}
	}

	const primaryScheduleStore = writable<ScheduleTrigger | undefined | false>(
		untrack(() => savedPrimarySchedule)
	) // kept for legacy reasons
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

	// Ctrl/Cmd+S forces an immediate save of whatever the page-level
	// autosave has pending. Unlike ScriptBuilder we don't have a direct
	// Monaco ref to flush — any focused module Monaco's pending text is
	// constrained by the editor's own ~1s max-wait cap, so the flush
	// here picks up whatever's already in `pendingSaveOpts`. Worst case
	// the user's very last keystroke (<1s ago) isn't in this POST and
	// follows in the next autosave round.
	//
	// No toast — the AutosaveIndicator narrates the flush (Saving... →
	// Saved / Save failed). A toast here would also lie on network
	// failure: `flush` never rejects (postSave catches and routes errors
	// to the failures map), so the success branch fired regardless.
	export async function saveDraft(): Promise<void> {
		if (!opWorkspace || !liveEditorDraftStoragePath) return
		await UserDraftDbSyncer.flush({
			workspace: opWorkspace,
			itemKind: 'flow',
			path: liveEditorDraftStoragePath
		})
	}

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
		if (onLatest || initialPath == '' || newFlow) {
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
			workspace: opWorkspace!,
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
				await FlowService.createFlow({
					workspace: opWorkspace!,
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
						deployment_message: deploymentMsg || undefined,
						labels: (flow as any).labels
					}
				})
				await CaptureService.moveCapturesAndConfigs({
					workspace: opWorkspace!,
					path: fakeInitialPath,
					requestBody: {
						new_path: $pathStore
					},
					runnableKind: 'flow'
				})
				if (triggersToDeploy) {
					await deployTriggers(
						triggersToDeploy,
						opWorkspace,
						!!$userStore?.is_admin || !!$userStore?.is_super_admin,
						usedTriggerKinds,
						$pathStore,
						true
					)
				}
			} else {
				if (triggersToDeploy) {
					await deployTriggers(
						triggersToDeploy,
						opWorkspace,
						!!$userStore?.is_admin || !!$userStore?.is_super_admin,
						usedTriggerKinds,
						initialPath
					)
				}

				await FlowService.updateFlow({
					workspace: opWorkspace!,
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
						deployment_message: deploymentMsg || undefined,
						labels: (flow as any).labels
					}
				})
			}

			// New/updated path now exists server-side — drop the autocomplete
			// cache so it shows up immediately instead of after the 60s TTL.
			invalidateWorkspacePaths(opWorkspace!)

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

	const selectionManager = new SelectionManager()
	const selectedIdStore = $derived(selectionManager.getSelectedId())
	// Initialize with selected id if provided
	if (untrack(() => selectedId)) {
		selectionManager.selectId(untrack(() => selectedId) ?? '')
	} else {
		selectionManager.selectId('settings-metadata')
	}

	export function getSelectedId() {
		return selectedIdStore
	}

	const previewArgsStore = $state({ val: untrack(() => initialArgs) })
	const scriptEditorDrawer = writable<ScriptEditorDrawer | undefined>(undefined)
	const flowEditorDrawer = writable<FlowEditorDrawer | undefined>(undefined)
	const history = initHistory(untrack(() => flowStore).val)
	const pathStore = writable<string>(untrack(() => pathStoreInit) ?? initialPath)

	$effect(() => {
		if (liveEditorDraftStoragePath === undefined || !opWorkspace) return
		const workspace = opWorkspace
		UserDraft.setLiveEditorDraft({
			workspace,
			itemKind: 'flow',
			storagePath: liveEditorDraftStoragePath,
			effectivePath: $pathStore
		})
		return () =>
			UserDraft.clearLiveEditorDraft('flow', {
				workspace,
				storagePath: liveEditorDraftStoragePath
			})
	})

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
		history,
		flowStateStore: untrack(() => flowStateStore),
		flowStore: untrack(() => flowStore),
		pathStore,
		stepsInputArgs,
		saveDraft,
		initialPathStore,
		fakeInitialPath,
		flowInputsStore: writable<FlowInput>({}),
		customUi: untrack(() => customUi),
		insertButtonOpen,
		executionCount: writable(0),
		flowInputEditorState: flowInputEditorStateStore,
		modulesTestStates,
		outputPickerOpenFns,
		preserveOnBehalfOf,
		savedOnBehalfOfEmail,
		opWorkspace: () => opWorkspace
	})

	// Set up NoteEditor context for note editing capabilities
	const noteEditor = new NoteEditor(
		untrack(() => flowStore),
		() => {
			// Enable notes display when a note is created
			flowEditor?.enableNotes?.()
		}
	)
	setNoteEditorContext(noteEditor)

	// Set up GroupEditor context for group editing capabilities
	const groupEditor = new GroupEditor(flowStore)
	let canCreateGroup = $state({ val: false })
	setGroupEditorContext(groupEditor, canCreateGroup)

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
				...(untrack(() => draftTriggersFromUrl) ?? [])
			],
			untrack(() => selectedTriggerIndexFromUrl)
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
			workspace: opWorkspace!,
			path: initialPath
		})

		// Initialize triggers using utility function
		await triggersState.fetchTriggers(
			triggersCount,
			opWorkspace,
			initialPath,
			true,
			$primaryScheduleStore,
			$userStore
		)
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

	let flowBuilderRoot: HTMLDivElement | undefined = $state()

	function onKeyDown(event: KeyboardEvent) {
		// Defer to anything that has explicitly grabbed focus — menus, modals,
		// drawers etc. live outside the flow root. Flow nodes aren't focusable,
		// so the unfocused default (activeElement === body) means "flow is the
		// canvas" and we should react.
		const active = document.activeElement
		if (active && active !== document.body && !flowBuilderRoot?.contains(active)) {
			return
		}

		let classes = event.target?.['className']
		if (
			(typeof classes === 'string' && classes.includes('inputarea')) ||
			['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName!)
		) {
			return
		}

		// Only lowercase single-char keys — named keys like `ArrowDown` must
		// stay PascalCase to match their switch cases.
		switch (event.key.length === 1 ? event.key.toLowerCase() : event.key) {
			case 'z':
				if (event.ctrlKey || event.metaKey) {
					if (event.shiftKey) handleRedo()
					else handleUndo()
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

	if (untrack(() => customUi).topBar?.extraDeployOptions != false) {
		if (!newFlow) {
			dropdownItems.push({
				label: 'Exit & see details',
				// Use the deployed path, not the live `$pathStore` — the latter
				// reflects local rename edits that haven't been deployed yet,
				// which would land the user on a 404 details page.
				onClick: () => onDetails?.({ path: initialPath })
			})
		}

		if (!untrack(() => newFlow)) {
			dropdownItems.push({
				label: 'Fork',
				onClick: () => window.open(`/flows/add?template=${initialPath}`)
			})
		}

		if (
			!untrack(() => newFlow) &&
			!isCloudHosted() &&
			editInForkAllowed(opWorkspace, $userWorkspaces)
		) {
			dropdownItems.push({
				label: editInForkLabel(opWorkspace, $userWorkspaces),
				onClick: () => window.open(buildForkEditUrl('flow', initialPath))
			})
		}
	}

	async function openDiffDrawer() {
		if (!savedFlow) return
		await syncWithDeployed()
		const currentDraftTriggers = structuredClone(triggersState.getDraftTriggersSnapshot())
		diffDrawer?.openDrawer()
		const currentFlow = flowStore.val
		diffDrawer?.setDiff({
			mode: 'normal',
			deployed: deployedValue ?? savedFlow,
			current: {
				...currentFlow,
				path: $pathStore,
				draft_triggers: currentDraftTriggers
			}
		})
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

	function getMoreItems(): Item[] {
		return [
			...baseMenuItems,
			{
				displayName: 'Undo',
				icon: Undo,
				action: () => handleUndo(),
				disabled: $history.index === 0,
				shortcut: `${mod}Z`,
				separatorTop: baseMenuItems.length > 0
			},
			{
				displayName: 'Redo',
				icon: Redo,
				action: () => handleRedo(),
				disabled: $history.index === $history.history.length - 1,
				shortcut: `${mod}⇧Z`
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
				icon: Disc,
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

	function handleDeployTrigger(_trigger: Trigger) {}

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
		}
	})
	// Sync `$pathStore` from `flowStore.val.path` (which `initFlow` populates
	// from the loaded flow — including the draft's rename, when there is one).
	// This effect only tracks `flowStore.val.path`, so popover edits that go
	// straight to `$pathStore` don't trigger it and aren't overwritten.
	// Replaces the previous `$pathStore = initialPath` push (added in #2536 for
	// the VSCode extension), which silently dropped any draft-renamed path
	// because `initialPath` is the URL, not the loaded path.
	$effect.pre(() => {
		// `flowStore.val` is typed `OpenFlow` here but `initFlow` actually puts a
		// `Flow` (with `path`) in it.
		const p = (flowStore.val as Flow | undefined)?.path
		if (p) untrack(() => ($pathStore = p))
	})

	// Persist the user-typed path into the draft JSON as `draft_path`
	// when it differs from the deployed/seeded `flow.path`. The Path
	// widget binds `$pathStore` one-way to the popover input — without
	// this, the friendly auto-name on `/flows/add` and any in-place
	// rename never reach the autosaved Flow, so the home-list draft row
	// kept showing the autogenerated `u/{user}/draft_{uuid}` slot. Drop
	// the field once it matches the baseline again so it doesn't
	// linger after a revert; deploy clears the whole draft, so the
	// field naturally disappears post-deploy too.
	$effect(() => {
		const typed = $pathStore
		const baseline = (flowStore.val as Flow | undefined)?.path ?? ''
		const flow = flowStore.val as (Flow & { draft_path?: string }) | undefined
		if (!flow) return
		untrack(() => {
			if (typed && typed !== baseline) {
				flow.draft_path = typed
			} else if (flow.draft_path !== undefined) {
				delete (flow as any).draft_path
			}
		})
	})

	$effect.pre(() => {
		selectedId && untrack(() => select(selectedId))
	})
	$effect.pre(() => {
		initialPath && initialPath != '' && opWorkspace && untrack(() => loadTriggers())
	})
	$effect.pre(() => {
		const hasAiDiff = aiChatManager.flowAiChatHelpers?.hasPendingChanges() ?? false
		customUi && untrack(() => onCustomUiChange(customUi, hasAiDiff))
	})

	export async function loadFlowState() {
		await stepHistoryLoader.loadIndividualStepsStates(
			flowStore.val as Flow,
			flowStateStore,
			opWorkspace!,
			$initialPathStore,
			$pathStore
		)
	}

	let stepHistoryLoader = new StepHistoryLoader(
		untrack(() => loadedFromHistoryFromUrl)?.stepsState ?? {},
		untrack(() => loadedFromHistoryFromUrl)?.flowJobInitial,
		undefined,
		untrack(() => noInitial)
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

		<div bind:this={flowBuilderRoot} class="flex flex-col h-screen">
			<!-- Nav between steps-->
			<div
				bind:clientWidth={topbarWidth}
				class="justify-between flex flex-row items-center pl-2 pr-4 space-x-4 scrollbar-hidden overflow-x-auto h-full relative {condensedHeader
					? 'max-h-9'
					: 'max-h-12'}"
			>
				<div class="flex flex-row items-center gap-2 min-w-0">
					<div class="min-w-0 overflow-hidden">
						<EditorHeader
							bind:summary={flowStore.val.summary}
							bind:path={$pathStore}
							savedPath={initialPath}
							onBehalfOfEmail={$savedOnBehalfOfEmail}
							hidePath={condensedHeader}
							workspaceId={autosaveWorkspace}
							onNavigate={(item) => onNavigate?.(item)}
						/>
					</div>
					{#if opWorkspace && indicatorPath !== undefined}
						<AutosaveIndicator
							workspace={opWorkspace}
							itemKind="flow"
							path={indicatorPath}
							draftOnly={newFlow}
							{onResetToDeployed}
							{loadedFromDraft}
							{othersDraftsCount}
							{onOpenOthersDrafts}
						/>
					{/if}
				</div>
				<div class="flex flex-row gap-2 items-center shrink-0">
					{#if $enterpriseLicense && !newFlow}
						<Awareness />
					{/if}
					<div class="relative">
						<Dropdown items={getMoreItems} size={headerBtnSize} fixedHeight={!condensedHeader} />
						{#if $tutorialsToDo.includes(getTutorialIndex('flow-live-tutorial')) || $tutorialsToDo.includes(getTutorialIndex('troubleshoot-flow'))}
							<span
								class="absolute top-0.5 right-0.5 block w-2 h-2 rounded-full bg-surface-accent-primary pointer-events-none"
							></span>
						{/if}
					</div>
					{#if customUi?.topBar?.diff != false}
						{@const isDraftOnly = savedFlow?.no_deployed === true}
						{@const diffDisabled = !savedFlow || newFlow || isDraftOnly}
						{@const diffTitle =
							newFlow || isDraftOnly
								? 'Deploy this flow once to compare against the deployed version'
								: 'Diff'}
						<!-- A disabled <button> fires no pointer events, so a title/tooltip on
						     it never shows on hover. pointer-events-none on the button lets the
						     hover reach this titled wrapper instead. -->
						<div title={diffTitle} class={diffDisabled ? 'flex cursor-not-allowed' : 'flex'}>
							<Button
								variant="default"
								unifiedSize={headerBtnSize}
								on:click={() => openDiffDrawer()}
								disabled={diffDisabled}
								btnClasses={diffDisabled ? 'pointer-events-none' : undefined}
								iconOnly={compactTopbar}
								title={diffTitle}
								startIcon={{ icon: DiffIcon }}
							>
								Diff
							</Button>
						</div>
					{/if}
					{#if !compactTopbar}
						{@render previewButtons()}
					{/if}

					<DeployButton
						on:save={async ({ detail }) => await handleSaveFlow(detail)}
						{loading}
						{loadingSave}
						unifiedSize={headerBtnSize}
						{dropdownItems}
					/>
				</div>
			</div>
			<!-- Rendered either inline in the top bar (wide) or as a graph overlay
			     (compactTopbar). Crossing the 720px threshold remounts
			     FlowPreviewButtons; any open preview state will reset. -->
			{#snippet previewButtons()}
				<FlowPreviewButtons
					{suspendStatus}
					unifiedSize={headerBtnSize}
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
					onRunPreview={(jobId) => {
						stepsInputArgs.resetManuallyEditedArgs()
						modulesTestStates.hideJobsInGraph()
						localModuleStates = {}
						showJobStatus = true
						if (jobId) {
							onTestJob?.({ jobId })
						}
					}}
				/>
			{/snippet}
			<!-- metadata -->
			{#if flowStateStore.val}
				<FlowEditor
					bind:this={flowEditor}
					graphOverlay={compactTopbar ? previewButtons : undefined}
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
					sessionOpen={$pathStore
						? {
								target: { kind: 'flow', path: $pathStore },
								workspaceId: opWorkspace ?? undefined,
								// Persist unsaved edits so the session preview
								// (/flows/edit/<path>) opens the flow exactly as it is in the
								// editor right now.
								beforeOpen: saveDraft
							}
						: undefined}
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
					previewOpen={flowPreviewButtons?.getPreviewOpen() ?? false}
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
