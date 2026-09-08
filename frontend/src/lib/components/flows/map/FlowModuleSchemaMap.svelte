<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { refreshStateStore } from '$lib/svelte5Utils.svelte'
	import type { OpenInSessionSource } from '$lib/components/sessions/OpenInSessionButton.svelte'
	import { createEventDispatcher, getContext, tick } from 'svelte'
	import {
		insertNewPreprocessorModule,
		createNewModule as createNewModuleIn,
		insertNewModuleAtIndex as insertNewModuleAt,
		insertAgentTool
	} from '$lib/components/flows/flowStateUtils.svelte'
	import type { FlowModule, Job, ScriptLang } from '$lib/gen'
	import { emptyFlowModuleState } from '../utils.svelte'

	import { dfs } from '../dfs'
	import { nextId, copyId } from '../flowModuleNextId'
	import { push } from '$lib/history.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import Portal from '$lib/components/Portal.svelte'
	import { overlayPortalTarget } from '$lib/components/common/overlayHost.svelte'

	import { locateModules, groupByParent } from '../multiSelectUtils'
	import { workspaceStore } from '$lib/stores'
	import FlowTutorials from '$lib/components/FlowTutorials.svelte'
	import FlowGraphV2 from '$lib/components/graph/FlowGraphV2.svelte'
	import { replaceId } from '../flowStore.svelte'
	import { setScheduledPollSchedule, type TriggerContext } from '$lib/components/triggers'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import { JobService } from '$lib/gen'
	import { findModuleInFlow } from '../flowTree'
	import { addBranch as addBranchOp, removeBranch as removeBranchOp } from '../branchOps'
	import type { InlineScript, InsertKind } from '$lib/components/graph/graphBuilder.svelte'
	import { MoveManager } from '$lib/components/graph/moveManager.svelte'
	import type { GraphModuleState } from '$lib/components/graph'
	import FlowStickyNode from './FlowStickyNode.svelte'
	import { getStepHistoryLoaderContext } from '$lib/components/stepHistoryLoader.svelte'
	import { ModulesTestStates } from '$lib/components/modulesTest.svelte'
	import type { StateStore } from '$lib/utils'
	import { type AgentTool, type SpecialToolKind } from '../agentToolUtils'
	import type { DeletePlan } from '../flowDeleteUtils'
	import { executeDeletePlan, prepareDeleteRequest } from '../flowDeleteController'
	import { getNoteEditorContext } from '$lib/components/graph/noteEditor.svelte'
	import {
		GroupedModulesProxy,
		type ExtendedOpenFlow
	} from '$lib/components/graph/groupedModulesProxy.svelte'
	import { GroupDisplayState, type FlowGroup } from '$lib/components/graph/groupEditor.svelte'
	import {
		type FlowStructureNode,
		matchStructureNode,
		dfsStructure,
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
		onTestUpTo?: ((id: string) => void) | undefined
		onEditInput?: (moduleId: string, key: string) => void
		localModuleStates?: Record<string, GraphModuleState>
		testModuleStates?: ModulesTestStates
		aiChatOpen?: boolean
		showFlowAiButton?: boolean
		toggleAiChat?: () => void
		sessionOpen?: OpenInSessionSource
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
		onTestUpTo,
		onEditInput,
		localModuleStates = {},
		testModuleStates = new ModulesTestStates(),
		aiChatOpen,
		showFlowAiButton,
		toggleAiChat,
		sessionOpen,
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

	const { customUi, selectionManager, history, flowStateStore, flowStore, pathStore, opWorkspace } =
		getContext<FlowEditorContext>('FlowEditorContext')

	let opWs = $derived(opWorkspace?.() ?? $workspaceStore)

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
	function createNewModule(
		kind: InsertKind,
		wsScript?: { path: string; summary: string; hash: string | undefined },
		wsFlow?: { path: string; summary: string },
		inlineScript?: InlineScript,
		agentPath?: string
	): Promise<FlowModule> {
		return createNewModuleIn(
			flowStore,
			flowStateStore,
			kind,
			wsScript,
			wsFlow,
			inlineScript,
			agentPath,
			opWs,
			disableAi
		)
	}

	/**
	 * Add a tool to an agent, from the graph's own `+ Tool` or from a surface that has no graph
	 * node to click — the agent step's Tools section. Kept here because it needs the map's history,
	 * id allocation and `flowStateStore` seeding.
	 */
	export async function addToolToAgent(
		agentId: string,
		detail: { kind: string; script?: any; flow?: any; inlineScript?: any }
	) {
		push(history, flowStore.val)
		const agentMod = findModuleInFlow(flowStore.val.value, agentId)
		const agentValue = agentMod?.value as { tools?: AgentTool[] } | undefined
		if (agentValue) {
			const id = await insertAgentTool(
				flowStore,
				flowStateStore,
				agentValue,
				detail,
				opWs,
				disableAi
			)
			// Reveal the new tool's config right away — in modal (unanchored) panel mode its editor
			// is otherwise hidden behind the graph.
			if (id) selectionManager.selectId(id, { openPanel: true })
		}
		refreshStateStore(flowStore)
		dispatch('change')
	}

	export function insertNewModuleAtIndex(
		modules: FlowModule[] | AgentTool[],
		index: number,
		kind: InsertKind,
		wsScript?: { path: string; summary: string; hash: string | undefined },
		wsFlow?: { path: string; summary: string },
		inlineScript?: InlineScript,
		toolKind?: SpecialToolKind | 'flowmoduleTool'
	): Promise<FlowModule[] | AgentTool[]> {
		push(history, flowStore.val)
		return insertNewModuleAt(
			flowStore,
			flowStateStore,
			modules,
			index,
			kind,
			wsScript,
			wsFlow,
			inlineScript,
			toolKind,
			opWs,
			disableAi
		)
	}

	let sidebarMode: 'list' | 'graph' = 'graph'

	let minHeight = $state(0)
	let flowPaneWidth = $state(0)
	let compactTopbar = $derived(flowPaneWidth < 800)

	function findModuleById(id: string) {
		return findModuleInFlow(flowStore.val.value, id)
	}

	export async function addBranch(id: string) {
		addBranchOp(id, { flowStore, history })
	}

	export function removeBranch(id: string, index: number) {
		removeBranchOp(id, index, { flowStore, flowStateStore, history })
	}

	// A single delete can have several consequences at once (emptied groups *and*
	// dependent steps); they are confirmed together so one user action never raises
	// more than one dialog.
	type PendingModuleAction = {
		label: 'delete' | 'move'
		stepCount: number
		groups: FlowGroup[]
		dependents: Record<string, string[]>
		confirm: () => void
		cancel?: () => void
	}

	// The modal keeps rendering `pendingModuleAction` while it fades out, so visibility is
	// driven by `moduleActionOpen` rather than by clearing the value — clearing it would
	// blank the dialog mid-transition. `moduleActionOpen` also makes confirm/cancel
	// one-shot: the buttons stay clickable until the fade ends.
	let pendingModuleAction: PendingModuleAction | undefined = $state(undefined)
	let moduleActionOpen = $state(false)

	function askModuleAction(action: PendingModuleAction) {
		pendingModuleAction = action
		moduleActionOpen = true
	}

	function stepNoun(action: PendingModuleAction | undefined) {
		return (action?.stepCount ?? 1) > 1 ? 'steps' : 'step'
	}

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

	export function reloadExpandedSubflows(): void {
		graph?.reloadExpandedSubflows?.()
	}

	function toggleNoteMode() {
		noteMode = !noteMode
	}

	function applyDeletePlan(plan: DeletePlan) {
		executeDeletePlan(plan, {
			history,
			flowStore,
			flowStateStore,
			selectionManager,
			onDelete
		})
	}

	function requestDelete(ids: string[]) {
		const request = prepareDeleteRequest({
			ids,
			flow: flowStore.val,
			tree: proxy.items,
			proxy,
			displayState: groupDisplayState
		})
		if (!request) {
			return
		}

		const affectedGroups = request.plan.structureDelete?.affectedGroups ?? []

		if (affectedGroups.length === 0 && !request.needsDependencyConfirmation) {
			applyDeletePlan(request.plan)
			return
		}

		askModuleAction({
			label: 'delete',
			stepCount: request.plan.targets.length,
			groups: affectedGroups,
			dependents: request.plan.dependents,
			confirm: () => applyDeletePlan(request.plan)
		})
	}

	export function deleteMultiple(ids: string[]) {
		requestDelete(ids)
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
			workspace: opWs!,
			scriptPathExact: path,
			jobKinds: ['preview', 'script', 'flowpreview', 'flow'].join(','),
			page: 1,
			perPage: 1
		})
		if (previousJobId.length > 0) {
			const getJobResult = await JobService.getCompletedJobResultMaybe({
				workspace: opWs!,
				id: previousJobId[0].id
			})
			if ('result' in getJobResult) {
				flowStateStore.val[moduleId] = {
					...(flowStateStore.val[moduleId] ?? {}),
					previewResult: getJobResult.result,
					previewJobId: previousJobId[0].id,
					previewSuccess: getJobResult.success,
					previewLogs: getJobResult['logs']
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

	const portalTarget = overlayPortalTarget('body')
</script>

<Portal name="flow-module" target={portalTarget()}>
	<ConfirmationModal
		title={`${pendingModuleAction?.label === 'move' ? 'Move' : 'Delete'} ${stepNoun(
			pendingModuleAction
		)}?`}
		confirmationText={`${pendingModuleAction?.label === 'move' ? 'Move' : 'Delete'} ${stepNoun(
			pendingModuleAction
		)}`}
		open={moduleActionOpen}
		on:confirmed={() => {
			if (!moduleActionOpen) return
			moduleActionOpen = false
			pendingModuleAction?.confirm()
		}}
		on:canceled={() => {
			if (!moduleActionOpen) return
			moduleActionOpen = false
			pendingModuleAction?.cancel?.()
		}}
	>
		{#if pendingModuleAction}
			{@const action = pendingModuleAction}
			{@const dependents = Object.entries(action.dependents)}
			{#if action.groups.length === 1}
				{@const group = action.groups[0]}
				<p
					>The group{group.summary ? ` "${group.summary}"` : ''} will be removed (empty or duplicate).</p
				>
			{:else if action.groups.length > 1}
				<p>The following groups will be removed (empty or duplicate):</p>
				<ul class="list-disc pl-4 mt-1">
					{#each action.groups as group}
						<li>{group.summary || `${group.start_id} → ${group.end_id}`}</li>
					{/each}
				</ul>
			{/if}
			{#if dependents.length > 0}
				<p class={action.groups.length > 0 ? 'mt-3' : ''}
					>The following steps will require changes afterwards:</p
				>
				<div class="mt-1">
					{#each dependents as [k, v]}
						<div class="pb-2">
							<h3 class="text-secondary font-semibold">{k}</h3>
							<ul class="text-sm">
								{#each v as dep}
									<li>{dep}</li>
								{/each}
							</ul>
						</div>
					{/each}
				</div>
			{/if}
			<p class="mt-2">Are you sure you want to {action.label} the {stepNoun(action)}?</p>
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
			{sessionOpen}
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
			workspace={opWs}
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
			onDelete={(id) => requestDelete([id])}
			onDismissRunNode={(id) => onDelete?.(id)}
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
						askModuleAction({
							label: 'move',
							stepCount: movedIds.length,
							groups: affectedGroups,
							dependents: {},
							confirm: doMove,
							cancel: () => moveManager.clearMoving()
						})
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
						detail.script,
						opWs
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

				// Agent tool inserts operate on the FlowModule's tools array directly
				if (detail.agentId) {
					await addToolToAgent(detail.agentId, detail)
					return
				}

				push(history, flowStore.val)

				// Regular module insert: create the module, then insert a leaf node via tree mutation
				const module = await createNewModule(
					detail.kind as InsertKind,
					detail.script,
					detail.flow ? { path: detail.flow.path, summary: detail.flow.summary } : undefined,
					detail.inlineScript,
					detail.agentPath
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

				// Inserting is a deliberate "now edit this": in modal mode the new step's
				// editor is otherwise hidden behind the graph.
				selectionManager.selectId(module.id, { openPanel: true })

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
				selectionManager.selectId(clone.id, { openPanel: true })
			}}
			onUpdateMock={(detail) => {
				let module = findModuleById(detail.id)
				if (!module) {
					throw new Error(`Node ${detail.id} not found`)
				}
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
