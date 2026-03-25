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
	import { nextId, copyId } from '../flowModuleNextId'
	import { push } from '$lib/history.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import Portal from '$lib/components/Portal.svelte'

	import { getAllModules, getDependentComponents } from '../flowExplorer'
	import { locateModules, groupByParent } from '../multiSelectUtils'
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
	import { MoveManager } from '$lib/components/graph/moveManager.svelte'
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
	import {
		GroupedModulesProxy,
		type ExtendedOpenFlow
	} from '$lib/components/graph/groupedModulesProxy.svelte'
	import { GroupDisplayState } from '$lib/components/graph/groupEditor.svelte'
	import {
		type FlowStructureNode,
		matchStructureNode,
		dfsStructure,
		findInStructure,
		moduleToStructureNode
	} from '$lib/components/graph/flowStructure'

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
		controlsPosition?: 'top' | 'bottom'
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
		controlsPosition = 'top',
		flowHasChanged
	}: Props = $props()

	const { customUi, selectionManager, history, flowStateStore, flowStore, pathStore } =
		getContext<FlowEditorContext>('FlowEditorContext')

	const moveManager = new MoveManager()
	const { triggersCount, triggersState } = getContext<TriggerContext>('TriggerContext')

	const { flowPropPickerConfig } = getContext<PropPickerContext>('PropPickerContext')

	// Get NoteEditor context for note position updates
	const noteEditorContext = getNoteEditorContext()
	const proxy = new GroupedModulesProxy(flowStore as unknown as StateStore<ExtendedOpenFlow>)
	const groupDisplayState = new GroupDisplayState(() => flowStore.val.value?.groups ?? [])

	$effect(() => {
		if (!moveManager.movingModuleId) return

		function onKeyDown(e: KeyboardEvent) {
			if (e.key === 'Escape') {
				moveManager.clearMoving()
			}
		}

		document.addEventListener('keydown', onKeyDown, true)
		return () => document.removeEventListener('keydown', onKeyDown, true)
	})

	/** Create a new FlowModule without inserting it into any array */
	async function createNewModule(
		kind: InsertKind,
		wsScript?: { path: string; summary: string; hash: string | undefined },
		wsFlow?: { path: string; summary: string },
		inlineScript?: InlineScript
	): Promise<FlowModule> {
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

		return module
	}

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
		const module = await createNewModule(kind, wsScript, wsFlow, inlineScript)

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
			// Standard FlowModule insertion
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
	let flowPaneWidth = $state(0)
	let compactTopbar = $derived(flowPaneWidth < 700)

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

	/** Confirmation gate for actions that would empty or duplicate groups */
	let affectedGroupsPending: import('$lib/components/graph/groupEditor.svelte').FlowGroup[] =
		$state([])
	let affectedGroupsAction: (() => void) | undefined = $state(undefined)
	let affectedGroupsCancel: (() => void) | undefined = $state(undefined)
	let affectedGroupsActionLabel: 'delete' | 'move' = $state('delete')

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

	export function deleteMultiple(ids: string[]) {
		const deletingSet = new Set(ids)
		const allDeps: Record<string, string[]> = {}
		for (const id of ids) {
			const deps = getDependentComponents(id, flowStore.val)
			for (const [depId, exprs] of Object.entries(deps)) {
				if (!deletingSet.has(depId)) {
					allDeps[depId] = [...(allDeps[depId] ?? []), ...exprs]
				}
			}
		}

		const opts = { displayState: groupDisplayState }
		const { emptiedGroups, duplicateGroups, commit } = proxy.prepareMutation((tree) => {
			for (const id of ids) {
				const found = findInStructure(tree, id)
				if (found) found.parentChildren.splice(found.index, 1)
			}
		}, opts)

		const affectedGroups = [...emptiedGroups, ...duplicateGroups]

		const cb = () => {
			push(history, flowStore.val)
			commit({ removeDuplicates: duplicateGroups.length > 0 })
			for (const id of ids) {
				delete flowStateStore.val[id]
			}
			selectionManager.clearSelection()
			refreshStateStore(flowStore)
		}

		const proceed = () => {
			if (Object.keys(allDeps).length > 0) {
				dependents = allDeps
				deleteCallback = cb
			} else {
				cb()
			}
		}

		if (affectedGroups.length > 0) {
			affectedGroupsPending = affectedGroups
			affectedGroupsActionLabel = 'delete'
			affectedGroupsAction = proceed
		} else {
			proceed()
		}
	}

	// Operates directly on the flat module array (not the structure tree).
	// Cloned modules are inserted after the originals, intentionally outside any group.
	export function duplicateMultiple(ids: string[]) {
		const locations = locateModules(ids, flowStore.val.value.modules)
		const groups = groupByParent(locations)

		push(history, flowStore.val)

		const allCloneIds: string[] = []

		for (const group of groups) {
			const sorted = [...group].sort((a, b) => a.index - b.index)
			const parentArr = sorted[0].parentArray
			const lastIndex = sorted[sorted.length - 1].index

			const clones: FlowModule[] = []
			for (const loc of sorted) {
				const original = parentArr[loc.index]
				const clone: FlowModule = $state.snapshot(original)

				clone.id = copyId(original.id, flowStateStore.val, flowStore.val)
				flowStateStore.val[clone.id] = emptyFlowModuleState()

				dfs([clone], (mod) => {
					if (mod.id !== clone.id) {
						const newModId = nextId(flowStateStore.val, flowStore.val)
						mod.id = newModId
						flowStateStore.val[newModId] = emptyFlowModuleState()
					}
				})

				clones.push(clone)
				allCloneIds.push(clone.id)
			}

			parentArr.splice(lastIndex + 1, 0, ...clones)
		}

		refreshStateStore(flowStore)
		selectionManager.selectByIds(allCloneIds)
	}

	export function moveMultiple(ids: string[]) {
		moveManager.toggleMovingMultiple(ids)
	}

	export function createGroup(ids: string[]) {
		graph?.createGroupFromSelection(ids)
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

	<ConfirmationModal
		title={affectedGroupsPending.length === 1 ? 'Remove group?' : 'Remove groups?'}
		confirmationText={affectedGroupsActionLabel === 'delete' ? 'Delete step' : 'Move step'}
		open={affectedGroupsPending.length > 0}
		on:confirmed={() => {
			affectedGroupsAction?.()
			affectedGroupsPending = []
			affectedGroupsAction = undefined
			affectedGroupsCancel = undefined
		}}
		on:canceled={() => {
			affectedGroupsCancel?.()
			affectedGroupsPending = []
			affectedGroupsAction = undefined
			affectedGroupsCancel = undefined
		}}
	>
		{#if affectedGroupsPending.length === 1}
			{@const group = affectedGroupsPending[0]}
			<p
				>The group{group.summary ? ` "${group.summary}"` : ''} will be removed (empty or duplicate).
				Are you sure you want to {affectedGroupsActionLabel} the step?</p
			>
		{:else}
			<p>The following groups will be removed (empty or duplicate):</p>
			<ul class="list-disc pl-4 mt-1">
				{#each affectedGroupsPending as group}
					<li>{group.summary || `${group.start_id} → ${group.end_id}`}</li>
				{/each}
			</ul>
			<p class="mt-2">Are you sure you want to {affectedGroupsActionLabel} the step?</p>
		{/if}
	</ConfirmationModal>
</Portal>
<div class="flex flex-col h-full relative -pt-1" bind:clientWidth={flowPaneWidth}>
	<div
		class={`z-50 absolute inline-flex flex-col gap-2 top-3 left-1/2 -translate-x-1/2 flex-initial  items-center transition-colors duration-[400ms] ease-linear bg-surface-100`}
	>
		<FlowStickyNode
			compact={compactTopbar}
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

	<div class="z-10 flex-auto grow min-h-0 bg-surface-secondary" bind:clientHeight={minHeight}>
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
			{moveManager}
			maxHeight={minHeight}
			modules={flowStore.val.value.modules}
			groupedModules={proxy.items}
			groupError={proxy.error}
			{groupDisplayState}
			{noteMode}
			notes={flowStore.val.value.notes}
			groups={flowStore.val.value.groups}
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

				if (id === 'preprocessor') {
					const cb = () => {
						push(history, flowStore.val)
						selectionManager.selectId('Input')
						flowStore.val.value.preprocessor_module = undefined
						refreshStateStore(flowStore)
						onDelete?.(id)
						delete flowStateStore.val[id]
					}
					if (Object.keys(dependents).length > 0) {
						deleteCallback = cb
					} else {
						cb()
					}
					return
				}

				const dsOpts = { displayState: groupDisplayState }
				const { emptiedGroups, duplicateGroups, commit } = proxy.prepareMutation((tree) => {
					const found = findInStructure(tree, id)
					if (found) found.parentChildren.splice(found.index, 1)
				}, dsOpts)

				const affectedGroups = [...emptiedGroups, ...duplicateGroups]

				const cb = () => {
					push(history, flowStore.val)
					selectNextId(id)
					commit({ removeDuplicates: duplicateGroups.length > 0 })
					refreshStateStore(flowStore)
					onDelete?.(id)
					delete flowStateStore.val[id]
				}

				const proceed = () => {
					if (Object.keys(dependents).length > 0) {
						deleteCallback = cb
					} else {
						cb()
					}
				}

				if (affectedGroups.length > 0) {
					affectedGroupsPending = affectedGroups
					affectedGroupsActionLabel = 'delete'
					affectedGroupsAction = proceed
				} else {
					proceed()
				}
			}}
			onInsert={async (detail) => {
				if (!flowStore.val.value.modules || !Array.isArray(flowStore.val.value.modules)) return
				await tick()

				// --- MOVE ---
				if (moveManager.movingModuleId) {
					const movedIds = moveManager.movingIds ?? [moveManager.movingModuleId]
					const movingId = moveManager.movingModuleId

					let mutated = false
					const moveOpts = { displayState: groupDisplayState }
					const { emptiedGroups, duplicateGroups, commit } = proxy.prepareMutation((tree) => {
						let originalModules: FlowStructureNode[] | undefined
						let targetModules: FlowStructureNode[] | undefined

						if (detail.sourceId == 'Input' || detail.targetId == 'Result') {
							targetModules = tree
						}
						dfsStructure(tree, (node, parentArray) => {
							if (matchStructureNode(node, movingId)) originalModules = parentArray
							if (detail.branch && matchStructureNode(node, detail.branch.rootId)) {
								targetModules = node.branches[detail.branch.branch]?.children
							} else if (
								matchStructureNode(node, detail.sourceId ?? '') ||
								matchStructureNode(node, detail.targetId ?? '')
							) {
								targetModules = parentArray
							}
						})

						if (!originalModules || !targetModules) return

						if (movedIds.length > 1) {
							const firstIndex = originalModules.findIndex((m) =>
								matchStructureNode(m, movedIds[0])
							)
							if (firstIndex < 0) return
							const removedModules = originalModules.splice(firstIndex, movedIds.length)
							let insertIndex = detail.index
							if (originalModules === targetModules && firstIndex < detail.index) {
								insertIndex -= movedIds.length
							}
							targetModules.splice(insertIndex, 0, ...removedModules)
						} else {
							const indexToRemove = originalModules.findIndex((m) =>
								matchStructureNode(m, movingId)
							)
							if (indexToRemove < 0) return
							const [removed] = originalModules.splice(indexToRemove, 1)
							let insertIndex = detail.index
							if (originalModules === targetModules && indexToRemove < detail.index)
								insertIndex -= 1
							targetModules.splice(insertIndex, 0, removed)
						}
						mutated = true
					}, moveOpts)

					if (!mutated) {
						moveManager.clearMoving()
						return
					}

					const affectedGroups = [...emptiedGroups, ...duplicateGroups]

					const doMove = () => {
						push(history, flowStore.val)
						commit({ removeDuplicates: duplicateGroups.length > 0 })
						if (movedIds.length > 1) {
							selectionManager.selectByIds(movedIds)
						} else {
							selectionManager.selectId(movingId)
						}
						moveManager.clearMoving()
						refreshStateStore(flowStore)
						dispatch('change')
					}

					if (affectedGroups.length > 0) {
						affectedGroupsPending = affectedGroups
						affectedGroupsActionLabel = 'move'
						affectedGroupsAction = doMove
						affectedGroupsCancel = () => moveManager.clearMoving()
					} else {
						doMove()
					}
					return
				}

				// --- INSERT ---
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
					refreshStateStore(flowStore)
					dispatch('change')
					return
				}

				push(history, flowStore.val)

				const isAgentInsert = !!detail.agentId
				const toolKind: SpecialToolKind | 'flowmoduleTool' | undefined = isAgentInsert
					? (SPECIAL_TOOL_KINDS as readonly string[]).includes(detail.kind)
						? (detail.kind as SpecialToolKind)
						: 'flowmoduleTool'
					: undefined

				// Agent tool inserts operate on the FlowModule's tools array directly
				if (isAgentInsert) {
					const agentMod = getAllModules(flowStore.val.value.modules).find(
						(m) => m.id === detail.agentId
					)
					if (agentMod && (agentMod.value as any).tools) {
						const tools = (agentMod.value as any).tools as AgentTool[]
						await insertNewModuleAtIndex(
							tools,
							tools.length,
							detail.kind as InsertKind,
							detail.script,
							detail.flow ? { path: detail.flow.path, summary: detail.flow.summary } : undefined,
							detail.inlineScript,
							toolKind
						)
						const id = tools[tools.length - 1].id
						selectionManager.selectId(id)
					}
					refreshStateStore(flowStore)
					dispatch('change')
					return
				}

				// Regular module insert: create the module, then insert a leaf node via tree mutation
				const module = await createNewModule(
					detail.kind as InsertKind,
					detail.script,
					detail.flow ? { path: detail.flow.path, summary: detail.flow.summary } : undefined,
					detail.inlineScript
				)
				const index = detail.index ?? 0
				const extraModules: FlowModule[] = [module]

				// For trigger inserts, also create the forloop module
				let loopModule: FlowModule | undefined
				if (detail.kind == 'trigger') {
					loopModule = await createNewModule('forloop')
					setExpr(loopModule, `results.${module.id}`)
					extraModules.push(loopModule)
				}

				proxy.applyTreeMutation(
					(tree) => {
						// Find target array in the snapshot
						let targetArray: FlowStructureNode[] | undefined
						if (
							detail.sourceId == 'Input' ||
							detail.targetId == 'Result' ||
							detail.kind == 'trigger'
						) {
							targetArray = tree
						}
						dfsStructure(tree, (node, parentArray) => {
							if (detail.branch && matchStructureNode(node, detail.branch.rootId)) {
								targetArray = node.branches[detail.branch.branch]?.children
							} else if (
								matchStructureNode(node, detail.sourceId ?? '') ||
								matchStructureNode(node, detail.targetId ?? '')
							) {
								targetArray = parentArray
							}
						})
						if (!targetArray) targetArray = tree

						// Insert the structure node (correct kind for containers like branchone/branchall)
						targetArray.splice(index, 0, moduleToStructureNode(module))

						// For trigger: also insert the forloop node after it
						if (loopModule) {
							targetArray.splice(index + 1, 0, moduleToStructureNode(loopModule))
						}
					},
					{ extraModules, displayState: groupDisplayState }
				)

				selectionManager.selectId(module.id)

				if (detail.inlineScript?.instructions) {
					dispatch('generateStep', {
						moduleId: module.id,
						lang: detail.inlineScript?.language,
						instructions: detail.inlineScript?.instructions
					})
				}
				if (detail.kind == 'trigger') {
					setScheduledPollSchedule(triggersState, triggersCount)
				}
				if (detail.flow?.path) {
					loadLastJob(detail.flow.path, module.id)
				} else if (detail.script?.path) {
					loadLastJob(detail.script?.path, module.id)
				}

				if (['branchone', 'branchall'].includes(detail.kind)) {
					await addBranch(module.id)
				}
				refreshStateStore(flowStore)
				dispatch('change')
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
				const groups = flowStore.val.value.groups
				if (groups) {
					for (const group of groups) {
						if (group.start_id === id) {
							group.start_id = newId
						}
						if (group.end_id === id) {
							group.end_id = newId
						}
					}
				}
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
				moveManager.toggleMoving(id)
			}}
			onDuplicate={(id) => {
				let targetModules: FlowModule[] | undefined
				let targetIndex: number = -1

				dfs(flowStore.val.value.modules, (mod, modules) => {
					const idx = modules.findIndex((m) => m.id === id)
					if (idx !== -1) {
						targetModules = modules
						targetIndex = idx
					}
				})

				if (!targetModules || targetIndex === -1) return

				push(history, flowStore.val)

				const original = targetModules[targetIndex]
				const clone: FlowModule = $state.snapshot(original)

				// Assign copy id to the clone, and fresh ids to nested modules
				clone.id = copyId(original.id, flowStateStore.val, flowStore.val)
				flowStateStore.val[clone.id] = emptyFlowModuleState()

				dfs([clone], (mod) => {
					if (mod.id !== clone.id) {
						const newModId = nextId(flowStateStore.val, flowStore.val)
						mod.id = newModId
						flowStateStore.val[newModId] = emptyFlowModuleState()
					}
				})

				targetModules.splice(targetIndex + 1, 0, clone)
				refreshStateStore(flowStore)
				selectionManager.selectId(clone.id)
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
			{controlsPosition}
			exitNoteMode={() => (noteMode = false)}
			onNotePositionUpdate={(noteId, position) => {
				// Update note position via NoteEditor context in edit mode
				if (noteEditorContext?.noteEditor) {
					noteEditorContext.noteEditor.updatePosition(noteId, position)
				}
			}}
			multiSelectEnabled
			movingIds={moveManager.movingIds}
			onDeleteMultiple={deleteMultiple}
			onDuplicateMultiple={duplicateMultiple}
			onMoveMultiple={moveMultiple}
		/>
	</div>
</div>

{#if !disableTutorials}
	<FlowTutorials on:reload />
{/if}
