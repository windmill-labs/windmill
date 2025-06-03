<script lang="ts">
	import {
		FlowService,
		type Flow,
		type FlowModule,
		DraftService,
		type PathScript,
		type OpenFlow,
		type InputTransform,
		type TriggersCount,
		CaptureService,
		type HubScriptKind
		CaptureService
	} from '$lib/gen'
	import { initHistory, redo, undo } from '$lib/history'
	import {
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
		type Value
	} from '$lib/utils'
	import { sendUserToast } from '$lib/toast'
	import { Drawer } from '$lib/components/common'
	import DeployOverrideConfirmationModal from '$lib/components/common/confirmationModal/DeployOverrideConfirmationModal.svelte'

	import { onMount, setContext, type ComponentType } from 'svelte'
	import { writable, type Writable } from 'svelte/store'
	import CenteredPage from './CenteredPage.svelte'
	import { Badge, Button, UndoRedo } from './common'
	import FlowEditor from './flows/FlowEditor.svelte'
	import ScriptEditorDrawer from './flows/content/ScriptEditorDrawer.svelte'
	import type { FlowState } from './flows/flowState'
	import { dfs as dfsApply } from './flows/dfs'
	import FlowImportExportMenu from './flows/header/FlowImportExportMenu.svelte'
	import FlowPreviewButtons from './flows/header/FlowPreviewButtons.svelte'
	import type { FlowEditorContext, FlowInput, FlowInputEditorState } from './flows/types'
	import { cleanInputs } from './flows/utils'
	import { Calendar, Pen, Save, DiffIcon, HistoryIcon, FileJson, type Icon } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import Awareness from './Awareness.svelte'
	import { getAllModules } from './flows/flowExplorer'
	import { type FlowCopilotContext } from './copilot/flow'
	import FlowAIButton from './copilot/chat/flow/FlowAIButton.svelte'
	import { loadFlowModuleState } from './flows/flowStateUtils'
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
	import type { FlowWithDraftAndDraftTriggers, Trigger } from './triggers/utils'
	import {
		deployTriggers,
		filterDraftTriggers,
		handleSelectTriggerFromKind
	} from './triggers/utils'
	import DraftTriggersConfirmationModal from './common/confirmationModal/DraftTriggersConfirmationModal.svelte'
	import { Triggers } from './triggers/triggers.svelte'

	export let initialPath: string = ''
	export let pathStoreInit: string | undefined = undefined
	export let newFlow: boolean
	export let selectedId: string | undefined
	export let initialArgs: Record<string, any> = {}
	export let loading = false
	export let flowStore: Writable<OpenFlow>
	export let flowStateStore: Writable<FlowState>
	export let savedFlow: FlowWithDraftAndDraftTriggers | undefined = undefined
	export let diffDrawer: DiffDrawer | undefined = undefined
	export let customUi: FlowBuilderWhitelabelCustomUi = {}
	export let disableAi: boolean = false
	export let disabledFlowInputs = false
	export let savedPrimarySchedule: ScheduleTrigger | undefined = undefined // used to set the primary schedule in the legacy primaryScheduleStore
	export let version: number | undefined = undefined
	export let setSavedraftCb: ((cb: () => void) => void) | undefined = undefined
	export let draftTriggersFromUrl: Trigger[] | undefined = undefined
	export let selectedTriggerIndexFromUrl: number | undefined = undefined

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
			modifiedValue: {
				...$flowStore,
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

	const dispatch = createEventDispatcher()

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

	let loadingSave = false
	let loadingDraft = false

	export async function saveDraft(forceSave = false): Promise<void> {
		if (!newFlow && !savedFlow) {
			return
		}
		if (savedFlow) {
			const draftOrDeployed = cleanValueProperties(savedFlow.draft || savedFlow)
			const currentDraftTriggers = structuredClone(triggersState.getDraftTriggersSnapshot())
			const current = cleanValueProperties({
				...$flowStore,
				path: $pathStore,
				draft_triggers: currentDraftTriggers
			})
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
						draft_triggers: triggersState.getDraftTriggersSnapshot()
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
					path: $pathStore,
					draft_triggers: structuredClone(triggersState.getDraftTriggersSnapshot())
				}
			} as FlowWithDraftAndDraftTriggers

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
						deployment_message: deploymentMsg || undefined
					}
				})
			}

			const { draft_triggers: _, ...newSavedFlow } = $flowStore as OpenFlow & {
				draft_triggers: Trigger[]
			}
			savedFlow = {
				...structuredClone(newSavedFlow),
				path: $pathStore
			} as Flow
			setDraftTriggers([])
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
						draft_triggers: triggersState.getDraftTriggersSnapshot(),
						selected_trigger: triggersState.getSelectedTriggerSnapshot()
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
	const moving = writable<{ id: string } | undefined>(undefined)
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
		currentEditor: writable(undefined),
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
	const triggersState = new Triggers(
		[
			{ type: 'webhook', path: '', isDraft: false },
			{ type: 'email', path: '', isDraft: false },
			...(draftTriggersFromUrl ?? savedFlow?.draft?.draft_triggers ?? [])
		],
		selectedTriggerIndexFromUrl,
		saveSessionDraft
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

	let flowPreviewButtons: FlowPreviewButtons

	let flowEditor: FlowEditor | undefined = undefined
	$: if (flowEditor) {
		flowCopilotContext.toggleAiPanel = flowEditor.toggleAiPanel
		flowCopilotContext.addSelectedLinesToAiChat = flowEditor.addSelectedLinesToAiChat
	}
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
	draftTriggers={triggersState.triggers.filter((t) => t.draftConfig)}
	isFlow={true}
	on:canceled={() => {
		draftTriggersModalOpen = false
	}}
	on:confirmed={handleDraftTriggersConfirmed}
/>

{#key renderCount}
	{#if !$userStore?.operator}
		{#if $pathStore}
			<FlowHistory bind:this={flowHistory} path={$pathStore} on:historyRestore />
		{/if}
		<FlowYamlEditor bind:drawer={yamlEditorDrawer} />
		<FlowImportExportMenu bind:drawer={jsonViewerDrawer} />
		<ScriptEditorDrawer bind:this={$scriptEditorDrawer} />

		<div class="flex flex-col flex-1 h-screen">
			<!-- Nav between steps-->
			<div
				class="justify-between flex flex-row items-center pl-2.5 pr-6 space-x-4 scrollbar-hidden overflow-x-auto max-h-12 h-full relative"
			>
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
					{#if triggersState.triggers?.some((t) => t.type === 'schedule')}
						{@const primaryScheduleIndex = triggersState.triggers.findIndex((t) => t.isPrimary)}
						{@const scheduleIndex = triggersState.triggers.findIndex((t) => t.type === 'schedule')}
						<Button
							btnClasses="hidden lg:inline-flex"
							startIcon={{ icon: Calendar }}
							variant="contained"
							color="light"
							size="xs"
							on:click={async () => {
								select('triggers')
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

								const currentDraftTriggers = structuredClone(
									triggersState.getDraftTriggersSnapshot()
								)

								diffDrawer?.openDrawer()
								diffDrawer?.setDiff({
									mode: 'normal',
									deployed: deployedValue ?? savedFlow,
									draft: savedFlow?.draft,
									current: { ...$flowStore, path: $pathStore, draft_triggers: currentDraftTriggers }
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
					{#if !disableAi && customUi?.topBar?.aiBuilder != false && flowEditor?.getIsAiPanelClosed()}
						<FlowAIButton openPanel={() => flowEditor?.toggleAiPanel()} />
					{/if}
					<FlowPreviewButtons
						on:openTriggers={(e) => {
							select('triggers')
							handleSelectTriggerFromKind(triggersState, triggersCount, initialPath, e.detail.kind)
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
							$testStepStore['preprocessor'] = ev.detail.args ?? {}
							$selectedIdStore = 'preprocessor'
						}
					}}
					on:testWithArgs={(e) => {
						$previewArgsStore = JSON.parse(JSON.stringify(e.detail))
						flowPreviewButtons?.openPreview(true)
					}}
					{savedFlow}
					onDeployTrigger={handleDeployTrigger}
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
