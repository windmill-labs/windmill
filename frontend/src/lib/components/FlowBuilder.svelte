<script lang="ts">
	import {
		FlowService,
		ScheduleService,
		type Flow,
		type FlowModule,
		DraftService,
		type PathScript,
		ScriptService,
		Script,
		type HubScriptKind,
		type OpenFlow
	} from '$lib/gen'
	import { initHistory, push, redo, undo } from '$lib/history'
	import {
		copilotInfo,
		enterpriseLicense,
		tutorialsToDo,
		userStore,
		workspaceStore
	} from '$lib/stores'
	import {
		cleanValueProperties,
		encodeState,
		formatCron,
		orderedJsonStringify,
		sleep
	} from '$lib/utils'
	import { sendUserToast } from '$lib/toast'
	import type { Drawer } from '$lib/components/common'

	import { setContext, tick } from 'svelte'
	import { writable, type Writable } from 'svelte/store'
	import CenteredPage from './CenteredPage.svelte'
	import { Badge, Button, Kbd, UndoRedo } from './common'
	import FlowEditor from './flows/FlowEditor.svelte'
	import ScriptEditorDrawer from './flows/content/ScriptEditorDrawer.svelte'
	import type { FlowState } from './flows/flowState'
	import { dfs as dfsApply } from './flows/dfs'
	import { dfs, getPreviousIds } from './flows/previousResults'
	import FlowImportExportMenu from './flows/header/FlowImportExportMenu.svelte'
	import FlowPreviewButtons from './flows/header/FlowPreviewButtons.svelte'
	import { loadFlowSchedule, type Schedule } from './flows/scheduleUtils'
	import type { FlowEditorContext } from './flows/types'
	import { cleanInputs, emptyFlowModuleState } from './flows/utils'
	import { Calendar, Pen, Save, DiffIcon } from 'lucide-svelte'
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
	import { loadFlowModuleState } from './flows/flowStateUtils'
	import FlowCopilotInputsModal from './copilot/FlowCopilotInputsModal.svelte'
	import { snakeCase } from 'lodash'
	import FlowBuilderTutorials from './FlowBuilderTutorials.svelte'

	import FlowTutorials from './FlowTutorials.svelte'
	import { ignoredTutorials } from './tutorials/ignoredTutorials'
	import type DiffDrawer from './DiffDrawer.svelte'
	import UnsavedConfirmationModal from './common/confirmationModal/UnsavedConfirmationModal.svelte'
	import { cloneDeep } from 'lodash'

	export let initialPath: string = ''
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

	const dispatch = createEventDispatcher()

	async function createSchedule(path: string) {
		const { cron, timezone, args, enabled } = $scheduleStore

		try {
			await ScheduleService.createSchedule({
				workspace: $workspaceStore!,
				requestBody: {
					path: path,
					schedule: formatCron(cron),
					timezone,
					script_path: path,
					is_flow: true,
					args,
					enabled
				}
			})
		} catch (err) {
			sendUserToast(`The primary schedule could not be created: ${err}`, true)
		}
	}

	let loadingSave = false
	let loadingDraft = false

	async function saveDraft(forceSave = false): Promise<void> {
		if (!newFlow && !savedFlow) {
			return
		}
		if (savedFlow) {
			const draftOrDeployed = cleanValueProperties(savedFlow.draft || savedFlow)
			const current = cleanValueProperties($flowStore)
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
			localStorage.removeItem('flow')
			localStorage.removeItem(`flow-${$pathStore}`)

			if (newFlow) {
				await FlowService.createFlow({
					workspace: $workspaceStore!,
					requestBody: {
						path: $pathStore,
						summary: flow.summary,
						description: flow.description ?? '',
						value: flow.value,
						schema: flow.schema,
						tag: flow.tag,
						draft_only: true,
						ws_error_handler_muted: flow.ws_error_handler_muted
					}
				})
			}
			await DraftService.createDraft({
				workspace: $workspaceStore!,
				requestBody: {
					path: newFlow ? $pathStore : initialPath,
					typ: 'flow',
					value: flow
				}
			})

			savedFlow = {
				...(newFlow
					? {
							...cloneDeep($flowStore),
							path: $pathStore,
							draft_only: true
					  }
					: savedFlow),
				draft: {
					...cloneDeep($flowStore),
					path: newFlow ? $pathStore : initialPath
				}
			} as Flow & {
				draft?: Flow
			}

			if (newFlow) {
				dispatch('saveInitial', $pathStore)
			}
			sendUserToast('Saved as draft')
		} catch (error) {
			sendUserToast(`Error while saving the flow as a draft: ${error.body || error.message}`, true)
		}
		loadingDraft = false
	}

	export function computeUnlockedSteps(flow: Flow) {
		return Object.fromEntries(
			getAllModules(flow.value.modules, flow.value.failure_module)
				.filter((m) => m.value.type == 'script' && m.value.hash == null)
				.map((m) => [m.id, (m.value as PathScript).path])
		)
	}

	async function saveFlow(): Promise<void> {
		loadingSave = true
		try {
			const flow = cleanInputs($flowStore)
			// console.log('flow', computeUnlockedSteps(flow)) // del
			// loadingSave = false // del
			// return
			const { cron, timezone, args, enabled } = $scheduleStore
			if (newFlow) {
				localStorage.removeItem('flow')
				localStorage.removeItem(`flow-${$pathStore}`)
				await FlowService.createFlow({
					workspace: $workspaceStore!,
					requestBody: {
						path: $pathStore,
						summary: flow.summary,
						description: flow.description ?? '',
						value: flow.value,
						schema: flow.schema,
						ws_error_handler_muted: flow.ws_error_handler_muted
					}
				})
				if (enabled) {
					await createSchedule($pathStore)
				}
			} else {
				localStorage.removeItem(`flow-${initialPath}`)
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
						ws_error_handler_muted: flow.ws_error_handler_muted
					}
				})
				const scheduleExists = await ScheduleService.existsSchedule({
					workspace: $workspaceStore ?? '',
					path: $pathStore
				})
				if (scheduleExists) {
					const schedule = await ScheduleService.getSchedule({
						workspace: $workspaceStore ?? '',
						path: $pathStore
					})
					if (JSON.stringify(schedule.args) != JSON.stringify(args) || schedule.schedule != cron) {
						await ScheduleService.updateSchedule({
							workspace: $workspaceStore ?? '',
							path: $pathStore,
							requestBody: {
								schedule: formatCron(cron),
								timezone,
								args
							}
						})
					}
					if (enabled != schedule.enabled) {
						await ScheduleService.setScheduleEnabled({
							workspace: $workspaceStore ?? '',
							path: $pathStore,
							requestBody: { enabled }
						})
					}
				} else if (enabled) {
					await createSchedule($pathStore)
				}
			}
			savedFlow = {
				...cloneDeep($flowStore),
				path: $pathStore
			} as Flow
			loadingSave = false
			dispatch('deploy', $pathStore)
		} catch (err) {
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
						selectedId: $selectedIdStore
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

	const scheduleStore = writable<Schedule>({
		args: {},
		cron: '',
		timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
		enabled: false
	})
	const previewArgsStore = writable<Record<string, any>>(initialArgs)
	const scriptEditorDrawer = writable<ScriptEditorDrawer | undefined>(undefined)
	const moving = writable<{ module: FlowModule; modules: FlowModule[] } | undefined>(undefined)
	const history = initHistory($flowStore)
	const pathStore = writable<string>(initialPath)

	$: $pathStore = initialPath

	const testStepStore = writable<Record<string, any>>({})

	function select(selectedId: string) {
		selectedIdStore.set(selectedId)
	}

	setContext<FlowEditorContext>('FlowEditorContext', {
		selectedId: selectedIdStore,
		schedule: scheduleStore,
		previewArgs: previewArgsStore,
		scriptEditorDrawer,
		moving,
		history,
		flowStateStore,
		flowStore,
		pathStore,
		testStepStore,
		saveDraft,
		initialPath
	})

	async function loadSchedule() {
		loadFlowSchedule(initialPath, $workspaceStore)
			.then((schedule: Schedule) => {
				scheduleStore.set(schedule)
			})
			.catch(() => {
				scheduleStore.set({
					cron: '0 */5 * * *',
					timezone: 'UTC',
					args: {},
					enabled: false
				})
			})
	}

	$: selectedId && select(selectedId)

	$: initialPath && initialPath != '' && $workspaceStore && loadSchedule()

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
				let ids = generateIds()
				let idx = ids.indexOf($selectedIdStore)
				if (idx > -1 && idx < ids.length - 1) {
					$selectedIdStore = ids[idx + 1]
					event.preventDefault()
				}
				break
			}
			case 'ArrowUp': {
				let ids = generateIds()
				let idx = ids.indexOf($selectedIdStore)
				if (idx > 0 && idx < ids.length) {
					$selectedIdStore = ids[idx - 1]
					event.preventDefault()
				}
				break
			}
		}
	}

	function generateIds() {
		return [
			'settings-metadata',
			'constants',
			...dfsApply($flowStore.value.modules, (module) => module.id)
		]
	}

	const dropdownItems: Array<{
		label: string
		onClick: () => void
	}> = []

	if (savedFlow?.draft_only === false) {
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

	let flowCopilotContext: FlowCopilotContext = {
		drawerStore: writable<Drawer | undefined>(undefined),
		modulesStore: writable<FlowCopilotModule[]>([]),
		currentStepStore: writable<string | undefined>(undefined),
		genFlow: undefined
	}

	setContext('FlowCopilotContext', flowCopilotContext)

	const {
		drawerStore: copilotDrawerStore,
		modulesStore: copilotModulesStore,
		currentStepStore: copilotCurrentStepStore
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
				path: `hub/${s.version_id}/${s.app}/${s.summary.toLowerCase().replaceAll(/\s+/g, '_')}`,
				summary: `${s.summary} (${s.app})`
			}))
			if (ts < doneTs) return
			doneTs = ts

			$copilotModulesStore[idx].hubCompletions = scripts as {
				path: string
				summary: string
				kind: HubScriptKind
				app: string
				ask_id: number
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
		if (module?.value.type === 'rawscript') {
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

			if (module.type === 'trigger') {
				if (!$scheduleStore.cron) {
					$scheduleStore.cron = '0 */15 * * *'
				}
				$scheduleStore.enabled = true
			}

			let hubScript:
				| {
						content: string
						lockfile?: string | undefined
						schema?: any
						language: string
						summary?: string | undefined
				  }
				| undefined = undefined

			if (module.source === 'hub' && module.selectedCompletion) {
				hubScript = await ScriptService.getHubScriptByPath({
					path: module.selectedCompletion.path
				})
			}

			const flowModule = {
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
					language: (hubScript ? hubScript.language : module.lang ?? 'bun') as Script.language,
					type: 'rawscript' as const
				},
				summary: module.selectedCompletion?.summary ?? module.description
			}

			$flowStateStore[module.id] = emptyFlowModuleState()
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
			const prevNodeId = getPreviousIds(module.id, $flowStore, false)[0]
			const pastModule: FlowModule | undefined = dfs(prevNodeId, $flowStore, false)[0]

			if (hubScript) {
				module.editor?.setCode(hubScript.content)
			} else if (module.source === 'custom') {
				module.editor?.setCode('')
				const deltaStore = writable<string>('')
				const unsubscribe = deltaStore.subscribe(async (delta) => {
					module.editor?.append(delta)
				})

				abortController = new AbortController()
				await stepCopilot(
					module,
					deltaStore,
					pastModule?.value.type === 'rawscript' ? pastModule.value.content : '',
					pastModule?.value.type === 'rawscript' ? pastModule.value.language : undefined,
					pastModule === undefined,
					isFirstInLoop,
					abortController
				)
				unsubscribe()
			} else {
				throw new Error('Invalid copilot module source')
			}

			copilotStatus = "Generating inputs for step '" + module.id + "'..."
			await sleep(500) // make sure code was parsed

			try {
				if (flowModule.value.type === 'rawscript') {
					const stepSchema: Schema = JSON.parse(JSON.stringify($flowStateStore[module.id].schema)) // deep copy
					if (
						module.source === 'hub' &&
						pastModule !== undefined &&
						$copilotInfo.exists_openai_resource_path
					) {
						// ask AI to set step inputs
						abortController = new AbortController()
						const inputs = await glueCopilot(
							Object.keys(flowModule.value.input_transforms),
							pastModule.value.type === 'rawscript' ? pastModule.value.content : '',
							pastModule.value.type === 'rawscript' ? pastModule.value.language : undefined,
							isFirstInLoop,
							abortController
						)

						// create flow inputs used by AI for autocompletion
						copilotFlowInputs = {}
						copilotFlowRequiredInputs = []
						Object.entries(inputs).forEach(([key, expr]) => {
							const snakeKey = snakeCase(key)
							if (
								key in stepSchema.properties &&
								expr.includes('flow_input.') &&
								!expr.includes('flow_input.iter') &&
								(!$flowStore.schema || !(snakeKey in $flowStore.schema.properties)) // prevent overriding flow inputs
							) {
								copilotFlowInputs[snakeKey] = stepSchema.properties[snakeKey]
								if (stepSchema.required.includes(snakeKey)) {
									copilotFlowRequiredInputs.push(snakeKey)
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
								expr: expr.replaceAll(/flow_input\.([A-Za-z0-9_]+)/g, (_, p1) => 'flow_input.' + p1)
							}
						})
					} else {
						if (
							module.source === 'hub' &&
							pastModule !== undefined &&
							!$copilotInfo.exists_openai_resource_path
						) {
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
								const schemaProperty = Object.entries(schema.properties).find(
									(x) => x[0] === key
								)?.[1]
								const snakeKey = snakeCase(key)
								if (
									schemaProperty &&
									(!$flowStore.schema || !(snakeKey in $flowStore.schema.properties)) // prevent overriding flow inputs
								) {
									copilotFlowInputs[snakeKey] = schemaProperty
									if (schema.required.includes(snakeKey)) {
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
						}
					}

					$flowStore = $flowStore // force rerendering
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
		copilotStatus = 'Setting flow inputs...'

		// filter out unused flow inputs
		const flowInputs: Record<string, SchemaProperty> = {}
		const required = new Set<string>()
		function getFlowInputs(modules: FlowModule[]) {
			for (const module of modules) {
				if (module.value.type === 'rawscript') {
					for (const moduleAttr of Object.keys(module.value.input_transforms)) {
						const input = module.value.input_transforms[moduleAttr]
						if (
							input.type === 'javascript' &&
							input.expr.includes('flow_input.') &&
							!input.expr.includes('flow_input.iter')
						) {
							const flowAttr = input.expr.split('.')[1]
							const schema = $flowStateStore[module.id].schema
							const schemaProperty = Object.entries(schema.properties).find(
								(x) => x[0] === moduleAttr
							)?.[1]
							if (schemaProperty) {
								flowInputs[flowAttr] = schemaProperty
								required.add(flowAttr)
							}
						}
					}
				} else if (module.value.type === 'forloopflow') {
					getFlowInputs(module.value.modules)
				}
			}
		}
		getFlowInputs($flowStore.value.modules)

		$flowStore.schema = {
			$schema: 'https://json-schema.org/draft/2020-12/schema',
			properties: flowInputs,
			required: Array.from(required),
			type: 'object'
		}

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

	export function triggerTutorial() {
		const urlParams = new URLSearchParams(window.location.search)
		const tutorial = urlParams.get('tutorial')

		if (tutorial) {
			flowTutorials?.runTutorialById(tutorial)
		} else if ($tutorialsToDo.includes(0) && !$ignoredTutorials.includes(0)) {
			flowTutorials?.runTutorialById('action')
		}
	}
</script>

<svelte:window on:keydown={onKeyDown} />

<UnsavedConfirmationModal
	{diffDrawer}
	savedValue={savedFlow}
	modifiedValue={{
		...$flowStore,
		path: $pathStore
	}}
/>

{#key renderCount}
	{#if !$userStore?.operator}
		<FlowCopilotDrawer {getHubCompletions} {genFlow} bind:flowCopilotMode />
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
				class="justify-between flex flex-row items-center pl-2.5 pr-6 space-x-4 scrollbar-hidden max-h-12 h-full relative"
			>
				{#if $copilotCurrentStepStore !== undefined}
					<div transition:fade class="absolute inset-0 bg-gray-500 bg-opacity-75 z-[900] !m-0" />
				{/if}
				<div class="flex w-full max-w-md gap-4 items-center">
					<div class="min-w-64 w-full">
						<input
							type="text"
							placeholder="Flow summary"
							class="text-sm w-full font-semibold"
							bind:value={$flowStore.summary}
						/>
					</div>
					<UndoRedo
						undoProps={{ disabled: $history.index === 0 }}
						redoProps={{ disabled: $history.index === $history.history.length - 1 }}
						on:undo={() => {
							$flowStore = undo(history, $flowStore)
							$selectedIdStore = 'Input'
						}}
						on:redo={() => {
							$flowStore = redo(history)
						}}
					/>
				</div>

				<div class="gap-4 flex-row hidden md:flex w-full max-w-md">
					{#if $scheduleStore.enabled}
						<Button
							btnClasses="hidden lg:inline-flex"
							startIcon={{ icon: Calendar }}
							variant="contained"
							color="light"
							size="xs"
							on:click={async () => {
								select('settings-schedule')
							}}
						>
							{$scheduleStore.cron ?? ''}
						</Button>
					{/if}
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
				</div>
				<div class="flex flex-row space-x-2">
					{#if $enterpriseLicense && !newFlow}
						<Awareness />
					{/if}
					<FlowBuilderTutorials
						on:reload={() => {
							renderCount += 1
						}}
					/>
					<Button
						color="light"
						variant="border"
						size="xs"
						on:click={() => {
							if (!savedFlow) {
								return
							}
							diffDrawer?.openDrawer()
							diffDrawer?.setDiff({
								mode: 'normal',
								deployed: savedFlow,
								draft: savedFlow['draft'],
								current: $flowStore
							})
						}}
						disabled={!savedFlow}
					>
						<div class="flex flex-row gap-2 items-center">
							<DiffIcon size={14} />
							Diff
						</div>
					</Button>

					<FlowCopilotStatus
						{copilotLoading}
						bind:copilotStatus
						{genFlow}
						{finishCopilotFlowBuilder}
						{abortController}
					/>

					<FlowImportExportMenu />

					<FlowPreviewButtons />
					<Button
						loading={loadingDraft}
						size="xs"
						startIcon={{ icon: Save }}
						on:click={() => saveDraft()}
						disabled={!newFlow && !savedFlow}
					>
						Save draft&nbsp;<Kbd small>Ctrl</Kbd><Kbd small>S</Kbd>
					</Button>
					<Button
						loading={loadingSave}
						size="xs"
						startIcon={{ icon: Save }}
						on:click={() => saveFlow()}
						dropdownItems={!newFlow ? dropdownItems : undefined}
					>
						Deploy
					</Button>
				</div>
			</div>

			<!-- metadata -->
			{#if $flowStateStore}
				<FlowEditor
					{loading}
					on:reload={() => {
						renderCount += 1
					}}
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
