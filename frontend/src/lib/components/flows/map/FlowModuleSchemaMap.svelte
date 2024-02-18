<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { createEventDispatcher, getContext, tick } from 'svelte'
	import {
		createBranchAll,
		createBranches,
		createLoop,
		deleteFlowStateById,
		emptyModule,
		pickScript
	} from '$lib/components/flows/flowStateUtils'
	import type { FlowModule } from '$lib/gen'
	import { emptyFlowModuleState } from '../utils'
	import FlowSettingsItem from './FlowSettingsItem.svelte'
	import FlowConstantsItem from './FlowConstantsItem.svelte'

	import { dfs } from '../dfs'
	import { FlowGraph } from '$lib/components/graph'
	import FlowErrorHandlerItem from './FlowErrorHandlerItem.svelte'
	import { push } from '$lib/history'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import Portal from 'svelte-portal'
	import { getDependentComponents } from '../flowExplorer'
	import type { FlowCopilotContext } from '$lib/components/copilot/flow'
	import { fade } from 'svelte/transition'
	import { copilotInfo, tutorialsToDo } from '$lib/stores'

	import FlowTutorials from '$lib/components/FlowTutorials.svelte'
	import { ignoredTutorials } from '$lib/components/tutorials/ignoredTutorials'
	import { tutorialInProgress } from '$lib/tutorialUtils'

	export let modules: FlowModule[] | undefined
	export let sidebarSize: number | undefined = undefined
	export let disableStaticInputs = false
	export let disableTutorials = false
	export let disableAi = false
	export let smallErrorHandler = false

	let flowTutorials: FlowTutorials | undefined = undefined

	const { selectedId, moving, history, flowStateStore, flowStore } =
		getContext<FlowEditorContext>('FlowEditorContext')

	async function insertNewModuleAtIndex(
		modules: FlowModule[],
		index: number,
		kind:
			| 'script'
			| 'forloop'
			| 'branchone'
			| 'branchall'
			| 'flow'
			| 'trigger'
			| 'approval'
			| 'end',
		wsScript?: { path: string; summary: string; hash: string | undefined }
	): Promise<FlowModule[]> {
		push(history, $flowStore)
		var module = emptyModule($flowStateStore, $flowStore, kind == 'flow')
		var state = emptyFlowModuleState()
		if (wsScript) {
			;[module, state] = await pickScript(wsScript.path, wsScript.summary, module.id, wsScript.hash)
		} else if (kind == 'forloop') {
			;[module, state] = await createLoop(
				module.id,
				!disableAi && $copilotInfo.exists_openai_resource_path
			)
		} else if (kind == 'branchone') {
			;[module, state] = await createBranches(module.id)
		} else if (kind == 'branchall') {
			;[module, state] = await createBranchAll(module.id)
		}
		$flowStateStore[module.id] = state
		if (kind == 'trigger') {
			module.summary = 'Trigger'
		} else if (kind == 'approval') {
			module.summary = 'Approval'
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
			if (mod.value.type == 'forloopflow') {
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
</script>

<Portal>
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
			<div transition:fade class="absolute inset-0 bg-gray-500 bg-opacity-75 z-[900] !m-0" />
		{/if}
		<FlowSettingsItem />
		{#if !disableStaticInputs}
			<FlowConstantsItem />
		{/if}
	</div>

	<div class="z-10 flex-auto grow" bind:clientHeight={minHeight}>
		<FlowGraph
			{disableAi}
			insertable
			scroll
			{minHeight}
			moving={$moving?.module.id}
			rebuildOnChange={$flowStore}
			maxHeight={minHeight}
			modules={$flowStore.value?.modules}
			{selectedId}
			on:delete={({ detail }) => {
				let e = detail.detail
				dependents = getDependentComponents(e.id, $flowStore)
				const cb = () => {
					push(history, $flowStore)
					selectNextId(e.id)
					removeAtId($flowStore.value.modules, e.id)
					$flowStore = $flowStore
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
					if (detail.modules) {
						await tick()
						if ($moving) {
							push(history, $flowStore)
							let indexToRemove = $moving.modules.findIndex((m) => $moving?.module?.id == m.id)
							$moving.modules.splice(indexToRemove, 1)
							detail.modules.splice(detail.index, 0, $moving.module)
							$selectedId = $moving.module.id
							$moving = undefined
						} else {
							await insertNewModuleAtIndex(
								detail.modules,
								detail.index ?? 0,
								detail.detail,
								detail.script
							)
							$selectedId = detail.modules[detail.index ?? 0].id
						}

						if (['branchone', 'branchall'].includes(detail.detail)) {
							await addBranch(detail.modules[detail.index ?? 0])
						}

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
