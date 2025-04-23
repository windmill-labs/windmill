<script lang="ts">
	import { Check, Loader2, Wand2 } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { getNonStreamingCompletion } from './lib'
	import { sendUserToast } from '$lib/toast'
	import type { Flow, InputTransform } from '$lib/gen'
	import ManualPopover from '../ManualPopover.svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import type { PickableProperties } from '../flows/previousResults'
	import YAML from 'yaml'
	import { sliceModules } from '../flows/flowStateUtils'
	import { dfs } from '../flows/dfs'
	import { yamlStringifyExceptKeys } from './utils'
	import type { FlowCopilotContext } from './flow'
	import { copilotInfo, stepInputCompletionEnabled } from '$lib/stores'
	import type { SchemaProperty } from '$lib/common'
	import FlowCopilotInputsModal from './FlowCopilotInputsModal.svelte'
	import { twMerge } from 'tailwind-merge'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'

	let generatedContent = ''
	let loading = false
	export let focused = false
	export let arg: InputTransform | any
	export let schemaProperty: SchemaProperty
	export let pickableProperties: PickableProperties | undefined = undefined
	export let argName: string
	export let showPopup: boolean

	let empty = false
	$: empty =
		Object.keys(arg ?? {}).length === 0 ||
		(arg.type === 'static' && !arg.value) ||
		(arg.type === 'javascript' && !arg.expr)

	let btnFocused = false

	let abortController = new AbortController()
	let newFlowInput = ''

	const { flowStore, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
	const { stepInputsLoading, generatedExprs } =
		getContext<FlowCopilotContext | undefined>('FlowCopilotContext') || {}

	function createFlowInput() {
		if (!newFlowInput) {
			return
		}
		const properties = {
			...($flowStore.schema?.properties as Record<string, SchemaProperty> | undefined),
			[newFlowInput]: schemaProperty
		}
		const required = [
			...(($flowStore.schema?.required as string[] | undefined) ?? []),
			newFlowInput
		]
		$flowStore.schema = {
			$schema: 'https://json-schema.org/draft/2020-12/schema',
			properties,
			required,
			type: 'object'
		}
	}

	async function generateStepInput() {
		if (generatedContent.length > 0 || loading) {
			return
		}
		abortController = new AbortController()
		loading = true
		const flow: Flow = JSON.parse(JSON.stringify($flowStore))
		const idOrders = dfs(flow.value.modules, (x) => x.id)
		const upToIndex = idOrders.indexOf($selectedId)
		if (upToIndex === -1) {
			throw new Error('Could not find the selected id in the flow')
		}

		const flowDetails =
			'Take into account the following information for never tested results:\n<flowDetails>\n' +
			yamlStringifyExceptKeys(sliceModules(flow.value.modules, upToIndex, idOrders), ['lock']) +
			'</flowDetails>'
		try {
			const availableData = {
				results: pickableProperties?.priorIds,
				flow_input: pickableProperties?.flow_input
			}
			const isInsideLoop = availableData.flow_input && 'iter' in availableData.flow_input
			const user = `I'm building a workflow which is a DAG of script steps.
The current step is ${$selectedId}, you can find the details for the step and previous ones below:
${flowDetails}
Determine for the input "${argName}", what to pass either from the previous results or the flow inputs. 
All possibles inputs either start with results. or flow_input. and are followed by the key of the input.
${
	isInsideLoop
		? 'As the step is in a loop, the iterator value is accessible as flow_input.iter.value.'
		: 'As the step is not in a loop, flow_input.iter.value is not available.'
}
Here's a summary of the available data:
<available>
${YAML.stringify(availableData)}</available>
${
	isInsideLoop
		? 'Favor results and flow_input.iter.value over flow inputs.'
		: 'Favor results over flow inputs'
}
If none of the available results are appropriate, are already used or are more appropriate for other inputs, you can also imagine new flow_input properties which we will create programmatically based on what you provide.
Reply with the most probable answer, do not explain or discuss.
Use javascript object dot notation to access the properties.
Only return the expression without any wrapper.`
			generatedContent = await getNonStreamingCompletion(
				[
					{
						role: 'user',
						content: user
					}
				],
				abortController
			)

			if (
				pickableProperties &&
				generatedContent.startsWith('flow_input.') &&
				generatedContent.split('.')[1] &&
				!(generatedContent.split('.')[1] in pickableProperties.flow_input)
			) {
				newFlowInput = generatedContent.split('.')[1]
			} else {
				newFlowInput = ''
			}
		} catch (err) {
			if (!abortController.signal.aborted) {
				sendUserToast('Could not generate summary: ' + err, true)
			}
		} finally {
			loading = false
		}
	}

	export function onKeyUp(event: KeyboardEvent) {
		if (!$copilotInfo.enabled || !$stepInputCompletionEnabled) {
			return
		}
		if (event.key === 'Tab') {
			if (!loading && generatedContent) {
				event.preventDefault()
				dispatch('setExpr', generatedContent)
				if (newFlowInput) {
					openInputsModal = true
				}
				generatedContent = ''
			}
		} else {
			cancel()
		}
	}

	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)

	function cancel() {
		abortController.abort()
		generatedContent = ''
	}

	function automaticGeneration() {
		if (empty) {
			generateStepInput()
		}
	}

	function cancelOnOutOfFocus() {
		setTimeout(() => {
			if (!focused && !btnFocused) {
				// only cancel if out of focus is not due to click on btn
				cancel()
			}
		}, 150)
	}

	$: if (!focused) {
		cancelOnOutOfFocus()
	}

	$: if ($copilotInfo.enabled && $stepInputCompletionEnabled && focused) {
		automaticGeneration()
	}

	$: dispatchIfMounted('showExpr', generatedContent)

	$: dispatchIfMounted('showExpr', $generatedExprs?.[argName] || '')

	let out = true // hack to prevent regenerating answer when accepting the answer due to mouseenter on new icon
	let openInputsModal = false
</script>

{#if $copilotInfo.enabled && $stepInputCompletionEnabled}
	<FlowCopilotInputsModal
		on:confirmed={async () => {
			createFlowInput()
		}}
		bind:open={openInputsModal}
		inputs={[newFlowInput]}
	/>
	<ManualPopover
		showTooltip={showPopup && (generatedContent.length > 0 || !!$generatedExprs?.[argName])}
		placement="bottom"
		class="p-2"
	>
		<Button
			size="xs"
			color="light"
			btnClasses={twMerge(
				'text-violet-800 dark:text-violet-400 bg-violet-100 dark:bg-gray-700 dark:hover:bg-surface-hover',
				!loading && generatedContent.length > 0
					? 'bg-green-100 text-green-800 hover:bg-green-100 dark:text-green-400 dark:bg-green-700 dark:hover:bg-green-700'
					: ''
			)}
			on:click={() => {
				if (!loading && generatedContent.length > 0) {
					dispatch('setExpr', generatedContent)
					if (newFlowInput) {
						openInputsModal = true
					}
					generatedContent = ''
				}
			}}
			on:mouseenter={(ev) => {
				if (out) {
					out = false
					generateStepInput()
				}
			}}
			on:mouseleave={() => {
				out = true
				cancel()
			}}
			endIcon={{
				icon:
					loading || ($stepInputsLoading && empty)
						? Loader2
						: generatedContent.length > 0
							? Check
							: Wand2,
				classes: loading || ($stepInputsLoading && empty) ? 'animate-spin' : ''
			}}
			on:focus={() => {
				btnFocused = true
			}}
			on:blur={() => {
				btnFocused = false
			}}
		>
			{#if focused}
				{#if loading}
					ESC
				{:else if generatedContent.length > 0}
					TAB
				{/if}
			{/if}
		</Button>
		<svelte:fragment slot="content">
			<div class="text-sm text-tertiary">
				{generatedContent || $generatedExprs?.[argName]}
			</div>
		</svelte:fragment>
	</ManualPopover>
{/if}
