<script lang="ts">
	import { run } from 'svelte/legacy'

	import { Check, Loader2, Wand2 } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { getNonStreamingCompletion } from './lib'
	import { sendUserToast } from '$lib/toast'
	import type { Flow, InputTransform } from '$lib/gen'
	import { createEventDispatcher, getContext, untrack } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import type { PickableProperties } from '../flows/previousResults'
	import YAML from 'yaml'
	import { sliceModules } from '../flows/flowStateUtils.svelte'
	import { dfs } from '../flows/dfs'
	import { yamlStringifyExceptKeys } from './utils'
	import type { FlowCopilotContext } from './flow'
	import { stepInputCompletionEnabled } from '$lib/stores'
	import type { SchemaProperty } from '$lib/common'
	import FlowCopilotInputsModal from './FlowCopilotInputsModal.svelte'
	import { twMerge } from 'tailwind-merge'
	import { copilotInfo } from '$lib/aiStore'
	import { AIBtnClasses } from './chat/AIButtonStyle'

	let generatedContent = $state('')
	let loading = $state(false)
	interface Props {
		focused?: boolean
		arg: InputTransform | any
		schemaProperty: SchemaProperty
		pickableProperties?: PickableProperties | undefined
		argName: string
		btnClass?: string
	}

	let {
		focused = false,
		arg,
		schemaProperty,
		pickableProperties = undefined,
		argName,
		btnClass = ''
	}: Props = $props()

	let empty = $state(false)
	run(() => {
		empty =
			Object.keys(arg ?? {}).length === 0 ||
			(arg.type === 'static' && !arg.value) ||
			(arg.type === 'javascript' && !arg.expr)
	})

	let btnFocused = $state(false)

	let abortController = new AbortController()
	let newFlowInput = $state('')

	const { flowStore, selectionManager } = getContext<FlowEditorContext>('FlowEditorContext')
	const { stepInputsLoading, generatedExprs } =
		getContext<FlowCopilotContext | undefined>('FlowCopilotContext') || {}

	function createFlowInput() {
		if (!newFlowInput) {
			return
		}
		const properties = {
			...(flowStore.val.schema?.properties as Record<string, SchemaProperty> | undefined),
			[newFlowInput]: schemaProperty
		}
		const required = [
			...((flowStore.val.schema?.required as string[] | undefined) ?? []),
			newFlowInput
		]
		flowStore.val.schema = {
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
		const flow: Flow = JSON.parse(JSON.stringify(flowStore.val))
		const idOrders = dfs(flow.value.modules, (x) => x.id)
		const upToIndex = idOrders.indexOf(selectionManager.getSelectedId())
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
The current step is ${selectionManager.getSelectedId()}, you can find the details for the step and previous ones below:
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
				sendUserToast('Could not generate step input: ' + err, true)
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

	$effect(() => {
		if (!focused) {
			untrack(() => {
				cancelOnOutOfFocus()
			})
		}
	})

	$effect(() => {
		if ($copilotInfo.enabled && $stepInputCompletionEnabled && focused) {
			untrack(() => {
				automaticGeneration()
			})
		}
	})

	$effect(() => {
		dispatch('showExpr', generatedContent)
	})

	$effect(() => {
		dispatch('showExpr', $generatedExprs?.[argName] || '')
	})

	let out = $state(true) // hack to prevent regenerating answer when accepting the answer due to mouseenter on new icon
	let openInputsModal = $state(false)
</script>

{#if $copilotInfo.enabled && $stepInputCompletionEnabled}
	<FlowCopilotInputsModal
		on:confirmed={async () => {
			createFlowInput()
		}}
		bind:open={openInputsModal}
		inputs={[newFlowInput]}
	/>
	<Button
		size="xs"
		variant="default"
		btnClasses={twMerge(
			AIBtnClasses(!loading && generatedContent.length > 0 ? 'green' : 'default'),
			btnClass
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
{/if}
