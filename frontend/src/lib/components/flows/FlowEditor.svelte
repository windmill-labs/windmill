<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import FlowEditorPanel from './content/FlowEditorPanel.svelte'
	import FlowModuleSchemaMap from './map/FlowModuleSchemaMap.svelte'
	import WindmillIcon from '../icons/WindmillIcon.svelte'
	import { Skeleton } from '../common'
	import { getContext, setContext } from 'svelte'
	import type { FlowEditorContext } from './types'
	import type { FlowCopilotContext } from '../copilot/flow'
	import { classNames } from '$lib/utils'

	import { writable } from 'svelte/store'
	import type { PropPickerContext, FlowPropPickerConfig } from '$lib/components/prop_picker'
	import type { PickableProperties } from '$lib/components/flows/previousResults'
	import type { Flow, FlowModule } from '$lib/gen'
	import type { Trigger } from '$lib/components/triggers/utils'
	import FlowAIChat from '../copilot/flowchat/FlowAIChat.svelte'
	import { dfs } from './previousResults'
	const { flowStore, flowStateStore, selectedId, flowInputsStore } =
		getContext<FlowEditorContext>('FlowEditorContext')

	export let loading: boolean
	export let disableStaticInputs = false
	export let disableTutorials = false
	export let disableAi = false
	export let disableSettings = false
	export let disabledFlowInputs = false
	export let smallErrorHandler = false
	export let newFlow: boolean = false
	export let savedFlow:
		| (Flow & {
				draft?: Flow | undefined
		  })
		| undefined = undefined
	export let onDeployTrigger: (trigger: Trigger) => void = () => {}

	let size = 50

	let flowModuleSchemaMap: FlowModuleSchemaMap | undefined

	const { currentStepStore: copilotCurrentStepStore } =
		getContext<FlowCopilotContext>('FlowCopilotContext')

	setContext<PropPickerContext>('PropPickerContext', {
		flowPropPickerConfig: writable<FlowPropPickerConfig | undefined>(undefined),
		pickablePropertiesFiltered: writable<PickableProperties | undefined>(undefined)
	})

	function getNestedModules(id: string, branchIndex?: number) {
		const { index, modules } = getIndexInNestedModules(id)

		// we know index is correct because we've already checked it in getIndexInNestedModules
		const module = modules[index]

		if (module.value.type === 'forloopflow' || module.value.type === 'whileloopflow') {
			return module.value.modules
		} else if (
			branchIndex !== undefined &&
			(module.value.type === 'branchall' || module.value.type === 'branchone')
		) {
			const branch = module.value.branches[branchIndex]

			if (!branch) {
				throw new Error(
					`Branch not found: ${id} in ${module.value.branches.map((b) => b.modules.map((m) => m.id).join(', ')).join(';')}`
				)
			}

			return branch.modules
		} else {
			throw new Error('Module is not a loop or branch')
		}
	}

	function getIndexInNestedModules(id: string) {
		const accessingModules = dfs(id, $flowStore, true).reverse()

		console.log('accessingModules', accessingModules, id)

		let parent = $flowStore.value.modules
		let lastIndex = -1
		for (const [ai, am] of accessingModules.entries()) {
			const index = parent.findIndex((m) => m.id === am.id)

			if (index === -1) {
				throw new Error(`Module not found: ${am.id} in ${parent.map((m) => m.id).join(', ')}`)
			}

			lastIndex = index

			if (ai === accessingModules.length - 1) {
				break
			}

			if (
				parent[index].value.type === 'forloopflow' ||
				parent[index].value.type === 'whileloopflow'
			) {
				parent = parent[index].value.modules
			} else if (
				parent[index].value.type === 'branchall' ||
				parent[index].value.type === 'branchone'
			) {
				const branchIdx = parent[index].value.branches.findIndex((b) =>
					b.modules.some((m) => m.id === accessingModules[ai + 1].id)
				)
				if (branchIdx === -1) {
					throw new Error(
						`Branch not found: ${am.id} in ${parent[index].value.branches.map((b) => b.modules.map((m) => m.id).join(', ')).join(';')}`
					)
				}
				parent = parent[index].value.branches[branchIdx].modules
			} else {
				throw new Error('Module is not a for loop or branch')
			}
		}

		if (lastIndex === -1) {
			throw new Error('Module not found, should have been caught earlier')
		}

		return {
			index: lastIndex,
			modules: parent
		}
	}
</script>

<div
	id="flow-editor"
	class={classNames(
		'h-full overflow-hidden transition-colors duration-[400ms] ease-linear border-t',
		$copilotCurrentStepStore !== undefined ? 'border-gray-500/75' : ''
	)}
>
	<Splitpanes>
		<Pane {size} minSize={15} class="h-full relative z-0">
			<div class="grow overflow-hidden bg-gray h-full bg-surface-secondary relative">
				{#if loading}
					<div class="p-2 pt-10">
						{#each new Array(6) as _}
							<Skeleton layout={[[2], 1.5]} />
						{/each}
					</div>
				{:else if $flowStore.value.modules}
					<FlowModuleSchemaMap
						bind:this={flowModuleSchemaMap}
						{disableStaticInputs}
						{disableTutorials}
						{disableAi}
						{disableSettings}
						{smallErrorHandler}
						{newFlow}
						bind:modules={$flowStore.value.modules}
						on:reload
					/>
				{/if}
			</div>
		</Pane>
		<Pane class="relative z-10" size={(100 - size) / 2} minSize={20}>
			{#if loading}
				<div class="w-full h-full">
					<div class="block m-auto pt-40 w-10">
						<WindmillIcon height="40px" width="40px" spin="fast" />
					</div>
				</div>
			{:else}
				<FlowEditorPanel
					{disabledFlowInputs}
					{newFlow}
					{savedFlow}
					enableAi={!disableAi}
					on:applyArgs
					on:testWithArgs
					{onDeployTrigger}
				/>
			{/if}
		</Pane>
		<Pane size={(100 - size) / 2} minSize={20}>
			<FlowAIChat
				flowManipulationHelpers={{
					insertStep: async (location, step) => {
						console.log('insertStep', location, step)
						const { index, modules } =
							location.type === 'start'
								? {
										index: -1,
										modules: $flowStore.value.modules
									}
								: location.type === 'start_inside_forloop'
									? {
											index: -1,
											modules: getNestedModules(location.inside)
										}
									: location.type === 'start_inside_branch'
										? {
												index: -1,
												modules: getNestedModules(location.inside, location.branchIndex)
											}
										: getIndexInNestedModules(location.afterId)

						const indexToInsertAt = index + 1

						let newModules: FlowModule[] | undefined
						switch (step.type) {
							case 'rawscript': {
								newModules = await flowModuleSchemaMap?.insertNewModuleAtIndex(
									modules,
									indexToInsertAt,
									'script',
									undefined,
									undefined,
									{
										language: step.language,
										kind: 'script',
										subkind: 'flow'
									}
								)
								break
							}
							case 'script': {
								newModules = await flowModuleSchemaMap?.insertNewModuleAtIndex(
									modules,
									indexToInsertAt,
									'script',
									{
										path: step.path,
										summary: '',
										hash: undefined
									}
								)
								break
							}
							case 'forloop':
							case 'branchall':
							case 'branchone': {
								newModules = await flowModuleSchemaMap?.insertNewModuleAtIndex(
									modules,
									indexToInsertAt,
									step.type
								)
								break
							}
							default: {
								throw new Error('Unknown step type')
							}
						}

						const newModule = newModules?.[indexToInsertAt]

						if (!newModule) {
							throw new Error('Failed to insert module')
						}

						if (['branchone', 'branchall'].includes(step.type)) {
							await flowModuleSchemaMap?.addBranch(newModule)
						}

						$flowStateStore = $flowStateStore
						$flowStore = $flowStore
						return newModule.id
					},
					removeStep: async (id) => {
						console.log('removeStep', id)
						const { modules } = getIndexInNestedModules(id)
						flowModuleSchemaMap?.selectNextId(id)
						flowModuleSchemaMap?.removeAtId(modules, id)

						if ($flowInputsStore) {
							delete $flowInputsStore[id]
						}

						$flowStore = $flowStore

						flowModuleSchemaMap?.updateFlowInputsStore()
					},
					getStepInputs: async (id) => {
						const module = dfs(id, $flowStore, false)[0]
						if (!module) {
							throw new Error('Module not found')
						}

						const inputs =
							module.value.type === 'script' || module.value.type === 'rawscript'
								? module.value.input_transforms
								: {}
						console.log('getStepInputs inputs', id, inputs)

						return inputs
					},
					setStepInputs: async (id, inputs) => {
						const parsedInputs = inputs.split('\n').map((x) => x.split(': '))
						console.log('setStepInputs', id, parsedInputs)

						const module = dfs(id, $flowStore, false)[0]
						if (!module) {
							throw new Error('Module not found')
						}

						if (module.value.type !== 'script' && module.value.type !== 'rawscript') {
							throw new Error('Module is not a script or rawscript')
						}

						for (const [key, value] of parsedInputs) {
							module.value.input_transforms[key] = {
								type: 'javascript',
								expr: value
							}
						}

						$flowStore = $flowStore
					},
					getFlowInputsSchema: async () => {
						return $flowStore.schema ?? {}
					},
					setFlowInputsSchema: async (newInputs) => {
						$flowStore.schema = newInputs
					},
					selectStep: (id) => {
						$selectedId = id
					},
					getStepCode: (id) => {
						const module = dfs(id, $flowStore, false)[0]
						if (!module) {
							throw new Error('Module not found')
						}
						if (module.value.type === 'rawscript') {
							return module.value.content
						} else {
							throw new Error('Module is not a rawscript')
						}
					},
					getFlowModules: () => $flowStore.value,
					setBranchPredicate: async (id, branchIndex, expression) => {
						const module = dfs(id, $flowStore, false)[0]
						if (!module) {
							throw new Error('Module not found')
						}
						if (module.value.type !== 'branchone') {
							throw new Error('Module is not a branchall or branchone')
						}
						const branch = module.value.branches[branchIndex]
						if (!branch) {
							throw new Error('Branch not found')
						}
						branch.expr = expression
					},
					addBranch: async (id) => {
						const module = dfs(id, $flowStore, false)[0]
						if (!module) {
							throw new Error('Module not found')
						}
						if (module.value.type !== 'branchall' && module.value.type !== 'branchone') {
							throw new Error('Module is not a branchall or branchone')
						}
						flowModuleSchemaMap?.addBranch(module)
						$flowStore = $flowStore
					},
					removeBranch: async (id, branchIndex) => {
						const module = dfs(id, $flowStore, false)[0]
						if (!module) {
							throw new Error('Module not found')
						}
						if (module.value.type !== 'branchall' && module.value.type !== 'branchone') {
							throw new Error('Module is not a branchall or branchone')
						}

						// for branch one, we set index + 1 because the removeBranch function assumes the index is shifted by 1 because of the default branch
						flowModuleSchemaMap?.removeBranch(
							module,
							module.value.type === 'branchone' ? branchIndex + 1 : branchIndex
						)
						$flowStore = $flowStore
					},
					setForLoopIteratorExpression: async (id, expression) => {
						const module = dfs(id, $flowStore, false)[0]
						if (!module) {
							throw new Error('Module not found')
						}
						if (module.value.type !== 'forloopflow') {
							throw new Error('Module is not a forloopflow')
						}
						flowModuleSchemaMap?.setExpr(module, expression)
					}
				}}
			/>
		</Pane>
	</Splitpanes>
</div>
