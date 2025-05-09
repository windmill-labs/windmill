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
		insertNewPreprocessorModule
	} from '$lib/components/flows/flowStateUtils'
	import type { FlowModule, RawScript, Script } from '$lib/gen'
	import { emptyFlowModuleState, initFlowStepWarnings } from '../utils'
	import FlowSettingsItem from './FlowSettingsItem.svelte'
	import FlowConstantsItem from './FlowConstantsItem.svelte'

	import { dfs } from '../dfs'
	import FlowErrorHandlerItem from './FlowErrorHandlerItem.svelte'
	import { push } from '$lib/history'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import Portal from '$lib/components/Portal.svelte'

	import { getDependentComponents } from '../flowExplorer'
	import type { FlowCopilotContext } from '$lib/components/copilot/flow'
	import { fade } from 'svelte/transition'
	import { copilotInfo, tutorialsToDo, workspaceStore } from '$lib/stores'

	import FlowTutorials from '$lib/components/FlowTutorials.svelte'
	import { ignoredTutorials } from '$lib/components/tutorials/ignoredTutorials'
	import { tutorialInProgress } from '$lib/tutorialUtils'
	import FlowGraphV2 from '$lib/components/graph/FlowGraphV2.svelte'
	import { replaceId } from '../flowStore'
	import { setScheduledPollSchedule, type TriggerContext } from '$lib/components/triggers'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import { JobService } from '$lib/gen'

	export let modules: FlowModule[] | undefined
	export let sidebarSize: number | undefined = undefined
	export let disableStaticInputs = false
	export let disableTutorials = false
	export let disableAi = false
	export let disableSettings = false
	export let newFlow: boolean = false
	export let smallErrorHandler = false
	export let workspace: string | undefined = $workspaceStore

	let flowTutorials: FlowTutorials | undefined = undefined

	const {
		customUi,
		selectedId,
		moving,
		history,
		flowStateStore,
		flowStore,
		flowInputsStore,
		pathStore
	} = getContext<FlowEditorContext>('FlowEditorContext')
	const { triggersCount, triggers } = getContext<TriggerContext>('TriggerContext')

	const { flowPropPickerConfig } = getContext<PropPickerContext>('PropPickerContext')
	async function insertNewModuleAtIndex(
		modules: FlowModule[],
		index: number,
		kind:
			| 'script'
			| 'forloop'
			| 'whileloop'
			| 'branchone'
			| 'branchall'
			| 'flow'
			| 'trigger'
			| 'approval'
			| 'end',
		wsScript?: { path: string; summary: string; hash: string | undefined },
		wsFlow?: { path: string; summary: string },
		inlineScript?: {
			language: RawScript['language']
			kind: Script['kind']
			subkind: 'pgsql' | 'flow'
			id: string
			summary?: string
		}
	): Promise<FlowModule[]> {
		push(history, $flowStore)
		var module = emptyModule($flowStateStore, $flowStore, kind == 'flow')
		var state = emptyFlowModuleState()
		$flowStateStore[module.id] = state
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
		} else if (inlineScript) {
			const { language, kind, subkind } = inlineScript
			;[module, state] = await createInlineScriptModule(
				language,
				kind,
				subkind,
				module.id,
				module.summary
			)
			$flowStateStore[module.id] = state
			if (kind == 'trigger') {
				module.summary = 'Trigger'
			} else if (kind == 'approval') {
				module.summary = 'Approval'
			}
		}
		$flowStateStore[module.id] = state

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
		modules.splice(index, 0, module)
		return modules
	}

	function removeAtId(modules: FlowModule[], id: string): FlowModule[] {
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
			}
			return mod
		})
	}

	$: sidebarMode == 'graph' ? (sidebarSize = 40) : (sidebarSize = 20)

	let sidebarMode: 'list' | 'graph' = 'graph'

	let minHeight = 0

	function selectNextId(id: any) {
		if (modules) {
			let allIds = dfs(modules, (mod) => mod.id)
			if (allIds.length > 1) {
				const idx = allIds.indexOf(id)
				$selectedId = idx == 0 ? allIds[0] : allIds[idx - 1]
			} else {
				$selectedId = 'settings-metadata'
			}
		}
	}
	async function addBranch(module: FlowModule) {
		push(history, $flowStore)

		if (module.value.type === 'branchone' || module.value.type === 'branchall') {
			module.value.branches.splice(module.value.branches.length, 0, {
				summary: '',
				expr: 'false',
				modules: []
			})
		}
	}

	function removeBranch(module: FlowModule, index: number) {
		push(history, $flowStore)

		if (module.value.type === 'branchone' || module.value.type === 'branchall') {
			const offset = module.value.type === 'branchone' ? 1 : 0

			if (module.value.branches[index - offset]?.modules) {
				const leaves = dfs(module.value.branches[index - offset].modules, (mod) => mod.id)
				leaves.forEach((leafId: string) => deleteFlowStateById(leafId, flowStateStore))
			}

			module.value.branches.splice(index - offset, 1)
		}
	}

	let deleteCallback: (() => void) | undefined = undefined
	let dependents: Record<string, string[]> = {}

	const { currentStepStore: copilotCurrentStepStore } =
		getContext<FlowCopilotContext | undefined>('FlowCopilotContext') || {}

	function shouldRunTutorial(tutorialName: string, name: string, index: number) {
		return (
			$tutorialsToDo.includes(index) &&
			name == tutorialName &&
			!$ignoredTutorials.includes(index) &&
			!tutorialInProgress()
		)
	}

	const dispatch = createEventDispatcher()

	async function updateFlowInputsStore() {
		const keys = Object.keys(dependents ?? {})

		for (const key of keys) {
			const module = $flowStore.value.modules.find((m) => m.id === key)

			if (!module) {
				continue
			}

			if (!$flowInputsStore) {
				$flowInputsStore = {}
			}

			$flowInputsStore[module.id] = {
				flowStepWarnings: await initFlowStepWarnings(
					module.value,
					$flowStateStore?.[module.id]?.schema,
					dfs($flowStore.value.modules, (fm) => fm.id)
				)
			}
		}
	}

	function setExpr(module: FlowModule, expr: string) {
		if (module.value.type == 'forloopflow') {
			module.value.iterator = { type: 'javascript', expr }
			module.value.parallel = true
		}
	}

	async function loadLastJob(path: string, moduleId: string) {
		if (!path) {
			return
		}
		const previousJobId = await JobService.listJobs({
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
				$flowStateStore[moduleId] = {
					...($flowStateStore[moduleId] ?? {}),
					previewResult: getJobResult.result,
					previewJobId: previousJobId[0].id,
					previewWorkspaceId: previousJobId[0].workspace_id,
					previewSuccess: getJobResult.success
				}
			}
			$flowStateStore = $flowStateStore
		}
	}
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
		class={`z-10 sticky inline-flex flex-col gap-2 top-0 bg-surface-secondary flex-initial p-2 items-center transition-colors duration-[400ms] ease-linear border-b ${
			$copilotCurrentStepStore !== undefined ? 'border-gray-500/75' : ''
		}`}
	>
		{#if $copilotCurrentStepStore !== undefined}
			<div transition:fade class="absolute inset-0 bg-gray-500 bg-opacity-75 z-[900] !m-0"></div>
		{/if}
		{#if !disableSettings}
			<FlowSettingsItem />
		{/if}
		{#if !disableStaticInputs}
			<FlowConstantsItem />
		{/if}
	</div>

	<div class="z-10 flex-auto grow bg-surface-secondary" bind:clientHeight={minHeight}>
		<FlowGraphV2
			earlyStop={$flowStore.value?.skip_expr !== undefined}
			cache={$flowStore.value?.cache_ttl !== undefined}
			triggerNode={customUi?.triggers != false}
			path={$pathStore}
			{newFlow}
			{disableAi}
			insertable
			scroll
			{minHeight}
			moving={$moving?.module.id}
			maxHeight={minHeight}
			modules={$flowStore.value?.modules}
			preprocessorModule={$flowStore.value?.preprocessor_module}
			{selectedId}
			{flowInputsStore}
			{workspace}
			editMode
			on:delete={({ detail }) => {
				let e = detail.detail
				dependents = getDependentComponents(e.id, $flowStore)
				const cb = () => {
					push(history, $flowStore)
					if (e.id === 'preprocessor') {
						$selectedId = 'Input'
						$flowStore.value.preprocessor_module = undefined
					} else {
						selectNextId(e.id)
						removeAtId($flowStore.value.modules, e.id)
						if ($flowInputsStore) {
							delete $flowInputsStore[e.id]
						}
					}
					$flowStore = $flowStore

					updateFlowInputsStore()
				}

				if (Object.keys(dependents).length > 0) {
					deleteCallback = cb
				} else {
					cb()
				}
			}}
			on:insert={async ({ detail }) => {
				if (shouldRunTutorial('forloop', detail.detail, 1)) {
					flowTutorials?.runTutorialById('forloop', detail.index)
				} else if (shouldRunTutorial('branchone', detail.detail, 2)) {
					flowTutorials?.runTutorialById('branchone')
				} else if (shouldRunTutorial('branchall', detail.detail, 3)) {
					flowTutorials?.runTutorialById('branchall')
				} else {
					if (detail.modules && Array.isArray(detail.modules)) {
						await tick()
						if ($moving) {
							push(history, $flowStore)
							let indexToRemove = $moving.modules.findIndex((m) => $moving?.module?.id == m.id)
							$moving.modules.splice(indexToRemove, 1)
							detail.modules.splice(detail.index, 0, $moving.module)
							$selectedId = $moving.module.id
							$moving = undefined
						} else {
							if (detail.detail === 'preprocessor') {
								insertNewPreprocessorModule(
									flowStore,
									flowStateStore,
									detail.inlineScript,
									detail.script
								)
								$selectedId = 'preprocessor'
							} else {
								const index = detail.index ?? 0
								await insertNewModuleAtIndex(
									detail.modules,
									index,
									detail.kind,
									detail.script,
									detail.flow,
									detail.inlineScript
								)
								const id = detail.modules[detail.index ?? 0].id
								$selectedId = id

								if (detail.kind == 'trigger') {
									await insertNewModuleAtIndex(
										detail.modules,
										index + 1,
										'forloop',
										undefined,
										undefined,
										undefined
									)
									setExpr(detail.modules[index + 1], `results.${id}`)
									setScheduledPollSchedule(triggers, triggersCount)
								}

								if (`flow` in detail) {
									loadLastJob(detail.flow?.path, id)
								} else if (`script` in detail) {
									loadLastJob(detail.script?.path, id)
								}
							}
						}

						if (['branchone', 'branchall'].includes(detail.kind)) {
							await addBranch(detail.modules[detail.index ?? 0])
						}
						$flowStateStore = $flowStateStore
						$flowStore = $flowStore
						dispatch('change')
					}
				}
			}}
			on:newBranch={async ({ detail }) => {
				if (detail.module) {
					await addBranch(detail.module)
					$flowStore = $flowStore
				}
			}}
			on:select={async ({ detail }) => {
				flowPropPickerConfig.set(undefined)
			}}
			on:changeId={({ detail }) => {
				let { id, newId, deps } = detail
				dfs($flowStore.value.modules, (mod) => {
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
				$flowStore = $flowStore
				$selectedId = newId
			}}
			on:deleteBranch={async ({ detail }) => {
				if (detail.module) {
					await removeBranch(detail.module, detail.index)
					$flowStore = $flowStore
					$selectedId = detail.module.id
				}
			}}
			on:move={async ({ detail }) => {
				if (!$moving || $moving.module.id !== detail.module.id) {
					if (detail.module && detail.modules) {
						$moving = { module: detail.module, modules: detail.modules }
					}
				} else {
					$moving = undefined
				}
			}}
			on:updateMock={() => {
				$flowStore = $flowStore
			}}
		/>
	</div>
	<div
		class="z-10 absolute inline-flex w-full text-sm gap-2 bottom-0 left-0 p-2 {smallErrorHandler
			? 'flex-row-reverse'
			: 'justify-center'} border-b"
	>
		<FlowErrorHandlerItem small={smallErrorHandler} />
	</div>
</div>

{#if !disableTutorials}
	<FlowTutorials bind:this={flowTutorials} on:reload />
{/if}
