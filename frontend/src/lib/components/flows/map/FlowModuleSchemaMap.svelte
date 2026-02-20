<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { createEventDispatcher, getContext, tick } from 'svelte'
	import {
		createInlineScriptModule,
		createBranchAll,
		createBranches,
		createLoop,
		createWhileLoop,
		deleteFlowStateById,
		emptyModule,
		pickScript,
		pickFlow,
		insertNewPreprocessorModule,
		createAiAgent
	} from '$lib/components/flows/flowStateUtils.svelte'
	import type { FlowModule, Job, ScriptLang } from '$lib/gen'
	import { emptyFlowModuleState } from '../utils.svelte'

	import { dfs } from '../dfs'
	import { push } from '$lib/history.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import Portal from '$lib/components/Portal.svelte'

	import { getDependentComponents } from '../flowExplorer'
	import { workspaceStore } from '$lib/stores'
	import { copilotInfo } from '$lib/aiStore'
	import FlowTutorials from '$lib/components/FlowTutorials.svelte'
	import FlowGraphV2 from '$lib/components/graph/FlowGraphV2.svelte'
	import { replaceId } from '../flowStore.svelte'
	import { setScheduledPollSchedule, type TriggerContext } from '$lib/components/triggers'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import { JobService } from '$lib/gen'
	import { dfsByModule } from '../previousResults'
	import type { InlineScript, InsertKind } from '$lib/components/graph/graphBuilder.svelte'
	import { refreshStateStore } from '$lib/svelte5Utils.svelte'
	import type { GraphModuleState } from '$lib/components/graph'
	import FlowStickyNode from './FlowStickyNode.svelte'
	import { getStepHistoryLoaderContext } from '$lib/components/stepHistoryLoader.svelte'
	import { ModulesTestStates } from '$lib/components/modulesTest.svelte'
	import type { StateStore } from '$lib/utils'
	import {
		type AgentTool,
		type SpecialToolKind,
		flowModuleToAgentTool,
		createMcpTool,
		createWebsearchTool,
		createAiAgentTool,
		SPECIAL_TOOL_KINDS,
		agentToolToFlowModule
	} from '../agentToolUtils'
	import { loadFlowModuleState } from '../flowStateUtils.svelte'
	import { getNoteEditorContext } from '$lib/components/graph/noteEditor.svelte'

	interface Props {
		sidebarSize?: number | undefined
		disableStaticInputs?: boolean
		disableTutorials?: boolean
		disableAi?: boolean
		disableSettings?: boolean
		newFlow?: boolean
		smallErrorHandler?: boolean
		workspace?: string | undefined
		onTestUpTo?: ((id: string) => void) | undefined
		onEditInput?: (moduleId: string, key: string) => void
		localModuleStates?: Record<string, GraphModuleState>
		testModuleStates?: ModulesTestStates
		aiChatOpen?: boolean
		showFlowAiButton?: boolean
		toggleAiChat?: () => void
		isOwner?: boolean
		onTestFlow?: () => void
		isRunning?: boolean
		onCancelTestFlow?: () => void
		onOpenPreview?: () => void
		onHideJobStatus?: () => void
		individualStepTests?: boolean
		flowJob?: Job | undefined
		showJobStatus?: boolean
		suspendStatus?: StateStore<Record<string, { job: Job; nb: number }>>
		onDelete?: (id: string) => void
		flowHasChanged?: boolean
	}

	let {
		sidebarSize = $bindable(undefined),
		disableStaticInputs = false,
		disableTutorials = false,
		disableAi = false,
		disableSettings = false,
		newFlow = false,
		smallErrorHandler = false,
		workspace = $workspaceStore,
		onTestUpTo,
		onEditInput,
		localModuleStates = {},
		testModuleStates = new ModulesTestStates(),
		aiChatOpen,
		showFlowAiButton,
		toggleAiChat,
		isOwner,
		onTestFlow,
		isRunning,
		onCancelTestFlow,
		onOpenPreview,
		onHideJobStatus,
		individualStepTests = false,
		flowJob = undefined,
		showJobStatus = false,
		suspendStatus = $bindable({ val: {} }),
		onDelete,
		flowHasChanged
	}: Props = $props()

	const { customUi, selectionManager, moving, history, flowStateStore, flowStore, pathStore } =
		getContext<FlowEditorContext>('FlowEditorContext')
	const { triggersCount, triggersState } = getContext<TriggerContext>('TriggerContext')

	const { flowPropPickerConfig } = getContext<PropPickerContext>('PropPickerContext')

	// Get NoteEditor context for note position updates
	const noteEditorContext = getNoteEditorContext()

	export async function insertNewModuleAtIndex(
		modules: FlowModule[] | AgentTool[],
		index: number,
		kind: InsertKind,
		wsScript?: { path: string; summary: string; hash: string | undefined },
		wsFlow?: { path: string; summary: string },
		inlineScript?: InlineScript,
		toolKind?: SpecialToolKind | 'flowmoduleTool'
	): Promise<FlowModule[] | AgentTool[]> {
		push(history, flowStore.val)
		let module = emptyModule(flowStateStore.val, flowStore.val, kind == 'flow')
		let state = emptyFlowModuleState()
		flowStateStore.val[module.id] = state
		if (wsFlow) {
			;[module, state] = await pickFlow(wsFlow.path, wsFlow.summary, module.id)
		} else if (wsScript) {
			;[module, state] = await pickScript(
				wsScript.path,
				wsScript.summary,
				module.id,
				wsScript.hash,
				kind
			)
		} else if (kind == 'forloop') {
			;[module, state] = await createLoop(module.id, !disableAi && $copilotInfo.enabled)
		} else if (kind == 'whileloop') {
			;[module, state] = await createWhileLoop(module.id)
		} else if (kind == 'branchone') {
			;[module, state] = await createBranches(module.id)
		} else if (kind == 'branchall') {
			;[module, state] = await createBranchAll(module.id)
		} else if (kind == 'aiagent') {
			;[module, state] = await createAiAgent(module.id)
		} else if (inlineScript) {
			const { language, kind, subkind, summary } = inlineScript
			;[module, state] = await createInlineScriptModule(language, kind, subkind, module.id, summary)
			flowStateStore.val[module.id] = state
			if (kind == 'trigger') {
				module.summary = 'Trigger'
			} else if (kind == 'approval') {
				module.summary = 'Approval'
			}
		}
		flowStateStore.val[module.id] = state

		if (kind == 'approval') {
			module.suspend = { required_events: 1, timeout: 1800 }
		} else if (kind == 'trigger') {
			module.stop_after_if = {
				expr: '!result || (Array.isArray(result) && result.length == 0)',
				skip_if_stopped: true
			}
		} else if (kind == 'end') {
			module.summary = 'Terminate flow'
			module.stop_after_if = { skip_if_stopped: false, expr: 'true' }
		}

		if (!modules) return [module]

		if (toolKind === 'mcpTool') {
			// Create MCP AgentTool
			const mcpTool = createMcpTool(module.id)
			;(modules as AgentTool[]).splice(index, 0, mcpTool)
			return modules as AgentTool[]
		} else if (toolKind === 'websearchTool') {
			// Create Websearch AgentTool
			const websearchTool = createWebsearchTool(module.id)
			;(modules as AgentTool[]).splice(index, 0, websearchTool)
			return modules as AgentTool[]
		} else if (toolKind === 'aiAgentTool') {
			// Create AI Agent tool (nested agent)
			const aiAgentTool = createAiAgentTool(module.id)
			flowStateStore.val[module.id] = await loadFlowModuleState(agentToolToFlowModule(aiAgentTool))
			;(modules as AgentTool[]).splice(index, 0, aiAgentTool)
			return modules as AgentTool[]
		} else if (toolKind === 'flowmoduleTool') {
			// Create AgentTool from FlowModule
			const agentTool = flowModuleToAgentTool(module)
			;(modules as AgentTool[]).splice(index, 0, agentTool)
			return modules as AgentTool[]
		} else {
			// Standard FlowModule insertion (existing behavior)
			modules.splice(index, 0, module)
			return modules
		}
	}

	/**
	 * Helper function to remove an AgentTool by id from the tools array
	 * Tools are always leaf nodes, so we just need to delete their state directly
	 */
	function removeAgentToolById(tools: AgentTool[], id: string): AgentTool[] {
		const index = tools.findIndex((tool) => tool.id == id)
		if (index != -1) {
			const [removed] = tools.splice(index, 1)
			deleteFlowStateById(removed.id, flowStateStore)
		}
		return tools
	}

	export function removeAtId(modules: FlowModule[], id: string): FlowModule[] {
		const index = modules.findIndex((mod) => mod.id == id)
		if (index != -1) {
			const [removed] = modules.splice(index, 1)
			const leaves = dfs([removed], (mod) => mod.id)
			leaves.forEach((leafId: string) => deleteFlowStateById(leafId, flowStateStore))
			return modules
		}
		return modules.map((mod) => {
			if (mod.value.type == 'forloopflow' || mod.value.type == 'whileloopflow') {
				mod.value.modules = removeAtId(mod.value.modules, id)
			} else if (mod.value.type == 'branchall') {
				mod.value.branches = mod.value.branches.map((branch) => {
					branch.modules = removeAtId(branch.modules, id)
					return branch
				})
			} else if (mod.value.type == 'branchone') {
				mod.value.branches = mod.value.branches.map((branch) => {
					branch.modules = removeAtId(branch.modules, id)
					return branch
				})
				mod.value.default = removeAtId(mod.value.default, id)
			} else if (mod.value.type == 'aiagent') {
				mod.value.tools = removeAgentToolById(mod.value.tools, id)
			}
			return mod
		})
	}

	let sidebarMode: 'list' | 'graph' = 'graph'

	let minHeight = $state(0)

	export function selectNextId(id: any) {
		if (flowStore.val.value.modules) {
			let allIds = dfs(flowStore.val.value.modules, (mod) => mod.id)
			if (allIds.length > 1) {
				const idx = allIds.indexOf(id)
				selectionManager.selectId(idx == 0 ? allIds[0] : allIds[idx - 1])
			} else {
				selectionManager.selectId('settings-metadata')
			}
		}
	}

	function findModuleById(id: string) {
		return dfsByModule(id, flowStore.val.value.modules)[0]
	}

	export async function addBranch(id: string) {
		push(history, flowStore.val)
		let module = findModuleById(id)

		if (!module) {
			throw new Error(`Node ${id} not found`)
		}

		if (module.value.type === 'branchone' || module.value.type === 'branchall') {
			module.value.branches.splice(module.value.branches.length, 0, {
				summary: '',
				expr: 'false',
				modules: []
			})
		}
	}

	export function removeBranch(id: string, index: number) {
		push(history, flowStore.val)
		let module = findModuleById(id)

		if (!module) {
			throw new Error(`Node ${id} not found`)
		}

		if (module.value.type === 'branchone' || module.value.type === 'branchall') {
			const offset = module.value.type === 'branchone' ? 1 : 0

			if (module.value.branches[index - offset]?.modules) {
				const leaves = dfs(module.value.branches[index - offset].modules, (mod) => mod.id)
				leaves.forEach((leafId: string) => deleteFlowStateById(leafId, flowStateStore))
			}

			module.value.branches.splice(index - offset, 1)
		}
	}

	let deleteCallback: (() => void) | undefined = $state(undefined)
	let dependents: Record<string, string[]> = $state({})

	let graph: FlowGraphV2 | undefined = $state(undefined)
	let noteMode = $state(false)
	let diffManager = $derived(getDiffManager())
	export function isNodeVisible(nodeId: string): boolean {
		return graph?.isNodeVisible(nodeId) ?? false
	}

	export function getDiffManager() {
		return graph?.getDiffManager()
	}

	export function enableNotes(): void {
		graph?.enableNotes?.()
	}

	function toggleNoteMode() {
		noteMode = !noteMode
	}

	const dispatch = createEventDispatcher<{
		generateStep: { moduleId: string; instructions: string; lang: ScriptLang }
		change: void
	}>()

	export function setExpr(module: FlowModule, expr: string) {
		if (module.value.type == 'forloopflow') {
			module.value.iterator = { type: 'javascript', expr }
			module.value.parallel = true
		}
	}

	let stepHistoryLoader = getStepHistoryLoaderContext()

	async function loadLastJob(path: string, moduleId: string) {
		if (!path) {
			return
		}
		if (stepHistoryLoader) {
			stepHistoryLoader.stepStates[moduleId] = {
				initial: true,
				loadingJobs: true
			}
		}
		const previousJobId = await JobService.listCompletedJobs({
			workspace: $workspaceStore!,
			scriptPathExact: path,
			jobKinds: ['preview', 'script', 'flowpreview', 'flow'].join(','),
			page: 1,
			perPage: 1
		})
		if (previousJobId.length > 0) {
			const getJobResult = await JobService.getCompletedJobResultMaybe({
				workspace: $workspaceStore!,
				id: previousJobId[0].id
			})
			if ('result' in getJobResult) {
				flowStateStore.val[moduleId] = {
					...(flowStateStore.val[moduleId] ?? {}),
					previewResult: getJobResult.result,
					previewJobId: previousJobId[0].id,
					previewSuccess: getJobResult.success
				}
				if (stepHistoryLoader) {
					stepHistoryLoader.stepStates[moduleId].loadingJobs = false
				}
			}
		}
	}
	$effect(() => {
		sidebarMode == 'graph' ? (sidebarSize = 40) : (sidebarSize = 20)
	})
</script>

<Portal name="flow-module">
	<ConfirmationModal
		title="Confirm deleting step with dependents"
		confirmationText="Delete step"
		open={Boolean(deleteCallback)}
		on:confirmed={() => {
			if (deleteCallback) {
				deleteCallback()
				deleteCallback = undefined
			}
		}}
		on:canceled={() => {
			deleteCallback = undefined
		}}
	>
		<div class="text-primary pb-2"
			>Found the following steps that will require changes after this step is deleted:</div
		>
		{#each Object.entries(dependents) as [k, v]}
			<div class="pb-3">
				<h3 class="text-secondary font-semibold">{k}</h3>
				<ul class="text-sm">
					{#each v as dep}
						<li>{dep}</li>
					{/each}
				</ul>
			</div>
		{/each}
	</ConfirmationModal>
</Portal>
<div class="flex flex-col h-full relative -pt-1">
	<div
		class={`z-50 absolute inline-flex flex-col gap-2 top-3 left-1/2 -translate-x-1/2 flex-initial  items-center transition-colors duration-[400ms] ease-linear bg-surface-100`}
	>
		<FlowStickyNode
			{disableAi}
			{showFlowAiButton}
			{disableSettings}
			{disableStaticInputs}
			{smallErrorHandler}
			on:generateStep
			{aiChatOpen}
			{toggleAiChat}
			{noteMode}
			{toggleNoteMode}
			{diffManager}
		/>
	</div>

	<div class="z-10 flex-auto grow bg-surface-secondary" bind:clientHeight={minHeight}>
		<FlowGraphV2
			bind:this={graph}
			earlyStop={flowStore.val.value?.skip_expr !== undefined}
			cache={flowStore.val.value?.cache_ttl !== undefined}
			triggerNode={customUi?.triggers != false}
			path={$pathStore}
			{newFlow}
			{disableAi}
			insertable
			scroll
			{minHeight}
			moving={$moving?.id}
			maxHeight={minHeight}
			modules={flowStore.val.value.modules}
			{noteMode}
			notes={flowStore.val.value.notes}
			preprocessorModule={flowStore.val.value?.preprocessor_module}
			failureModule={flowStore.val.value?.failure_module}
			currentInputSchema={flowStore.val.schema}
			{selectionManager}
			{workspace}
			editMode
			{onTestUpTo}
			{onEditInput}
			flowModuleStates={localModuleStates}
			{testModuleStates}
			{isOwner}
			{individualStepTests}
			{flowJob}
			{showJobStatus}
			suspendStatus={suspendStatus.val}
			{flowHasChanged}
			chatInputEnabled={Boolean(flowStore.val.value?.chat_input_enabled)}
			onDelete={(id) => {
				dependents = getDependentComponents(id, flowStore.val)
				const cb = () => {
					push(history, flowStore.val)
					if (id === 'preprocessor') {
						selectionManager.selectId('Input')
						flowStore.val.value.preprocessor_module = undefined
					} else {
						selectNextId(id)
						removeAtId(flowStore.val.value.modules, id)
					}
					refreshStateStore(flowStore)
					onDelete?.(id)
					delete flowStateStore.val[id]
				}

				if (Object.keys(dependents).length > 0) {
					deleteCallback = cb
				} else {
					cb()
				}
			}}
			onInsert={async (detail) => {
				{
					let originalModules
					let targetModules
					if (
						detail.sourceId == 'Input' ||
						detail.targetId == 'Result' ||
						detail.kind == 'trigger'
					) {
						targetModules = flowStore.val.value.modules
					}

					dfs(flowStore.val.value.modules, (mod, modules, branches) => {
						// console.log('mod', mod.id, $moving?.id, detail, branches)
						if (mod.id == $moving?.id) {
							originalModules = modules
						}
						if (detail.branch) {
							if (mod.id == detail.branch.rootId) {
								targetModules = branches[detail.branch.branch]
							}
						} else if (mod.id == detail.sourceId || mod.id == detail.targetId) {
							targetModules = modules
						} else if (mod.id == detail.agentId && mod.value.type === 'aiagent') {
							targetModules = mod.value.tools
						}
					})
					if (flowStore.val.value.modules && Array.isArray(flowStore.val.value.modules)) {
						await tick()
						if ($moving) {
							// console.log('modules', modules, movingModules, movingModule)
							push(history, flowStore.val)
							let indexToRemove = originalModules.findIndex((m) => $moving?.id == m.id)

							let [removedModule] = originalModules.splice(indexToRemove, 1)
							targetModules.splice(detail.index, 0, removedModule)
							selectionManager.selectId(removedModule.id)
							$moving = undefined
						} else {
							if (detail.isPreprocessor) {
								await insertNewPreprocessorModule(
									flowStore,
									flowStateStore,
									detail.inlineScript,
									detail.script
								)
								selectionManager.selectId('preprocessor')

								if (detail.inlineScript?.instructions) {
									dispatch('generateStep', {
										moduleId: 'preprocessor',
										lang: detail.inlineScript?.language,
										instructions: detail.inlineScript?.instructions
									})
								}
							} else {
								const index = (detail.agentId ? targetModules?.length : detail.index) ?? 0
								const toolKind: SpecialToolKind | 'flowmoduleTool' | undefined = detail.agentId
									? (SPECIAL_TOOL_KINDS as readonly string[]).includes(detail.kind)
										? (detail.kind as SpecialToolKind)
										: 'flowmoduleTool'
									: undefined

								await insertNewModuleAtIndex(
									targetModules,
									index,
									detail.kind,
									detail.script,
									detail.flow,
									detail.inlineScript,
									toolKind
								)
								const id = targetModules[index].id
								selectionManager.selectId(id)

								if (detail.inlineScript?.instructions) {
									dispatch('generateStep', {
										moduleId: id,
										lang: detail.inlineScript?.language,
										instructions: detail.inlineScript?.instructions
									})
								}
								if (detail.kind == 'trigger') {
									await insertNewModuleAtIndex(
										targetModules,
										index + 1,
										'forloop',
										undefined,
										undefined,
										undefined
									)
									setExpr(targetModules[index + 1], `results.${id}`)
									setScheduledPollSchedule(triggersState, triggersCount)
								}

								if (detail.flow?.path) {
									loadLastJob(detail.flow.path, id)
								} else if (detail.script?.path) {
									loadLastJob(detail.script?.path, id)
								}
							}
						}

						if (['branchone', 'branchall'].includes(detail.kind)) {
							await addBranch(targetModules[detail.index ?? 0].id)
						}
						refreshStateStore(flowStore)
						dispatch('change')
					}
				}
			}}
			onNewBranch={async (id) => {
				if (id) {
					await addBranch(id)
					refreshStateStore(flowStore)
				}
			}}
			onSelect={(id) => {
				flowPropPickerConfig.set(undefined)
			}}
			onChangeId={(detail) => {
				let { id, newId, deps } = detail

				dfs(flowStore.val.value.modules, (mod) => {
					if (deps[mod.id]) {
						deps[mod.id].forEach((dep) => {
							if (
								mod.value.type == 'rawscript' ||
								mod.value.type == 'script' ||
								mod.value.type == 'flow'
							) {
								mod.value.input_transforms = Object.fromEntries(
									Object.entries(mod.value.input_transforms).map(([k, v]) => {
										if (v.type == 'javascript') {
											return [k, { ...v, expr: replaceId(v.expr, id, newId) }]
										} else {
											return [k, v]
										}
									})
								)
							} else if (mod?.value?.type === 'forloopflow') {
								if (mod.value.iterator.type === 'javascript') {
									mod.value.iterator.expr = replaceId(mod.value.iterator.expr, id, newId)
								}
							} else if (mod?.value?.type === 'branchone') {
								mod.value.branches.forEach((branch) => {
									branch.expr = replaceId(branch.expr, id, newId)
								})
							}
						})
					}
					if (mod.id == id) {
						mod.id = newId
					}
				})
				flowStateStore.val[newId] = flowStateStore.val[id]
				delete flowStateStore.val[id]
				refreshStateStore(flowStore)
				selectionManager.selectId(newId)
			}}
			onDeleteBranch={async ({ id, index }) => {
				if (id) {
					await removeBranch(id, index)
					refreshStateStore(flowStore)
					selectionManager.selectId(id)
				}
			}}
			onMove={(id) => {
				if (!$moving || $moving.id !== id) {
					$moving = { id }
				} else {
					$moving = undefined
				}
			}}
			onUpdateMock={(detail) => {
				let module = findModuleById(detail.id)
				module.mock = $state.snapshot(detail.mock)
				refreshStateStore(flowStore)
			}}
			{onTestFlow}
			{isRunning}
			{onCancelTestFlow}
			{onOpenPreview}
			{onHideJobStatus}
			exitNoteMode={() => (noteMode = false)}
			onNotePositionUpdate={(noteId, position) => {
				// Update note position via NoteEditor context in edit mode
				if (noteEditorContext?.noteEditor) {
					noteEditorContext.noteEditor.updatePosition(noteId, position)
				}
			}}
			multiSelectEnabled
		/>
	</div>
</div>

{#if !disableTutorials}
	<FlowTutorials on:reload />
{/if}
