<script lang="ts">
	import { getNonStreamingMetadataCompletion } from './lib'
	import { sendUserToast } from '$lib/toast'
	import type { Flow } from '$lib/gen'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import type { PickableProperties } from '../flows/previousResults'
	import YAML from 'yaml'
	import { sliceModules } from '../flows/flowStateUtils.svelte'
	import { dfs } from '../flows/dfs'
	import { yamlStringifyExceptKeys } from './utils'
	import type { FlowCopilotContext } from './flow'
	import { logStepInputFill } from './stepInputFillTelemetry'
	import { stepInputCompletionEnabled } from '$lib/stores'
	import type { SchemaProperty } from '$lib/common'
	import FlowCopilotInputsModal from './FlowCopilotInputsModal.svelte'
	import { copilotInfo } from '$lib/aiStore'
	import Button from '../common/button/Button.svelte'
	import { AIBtnClasses } from './chat/AIButtonStyle'
	import { Check, Wand2 } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	let generatedContent = $state('')
	let loading = $state(false)
	interface Props {
		/** Whether the input this belongs to has focus — a suggestion is only worth keeping
		 *  while the user is still on that input. */
		focused?: boolean
		schemaProperty: SchemaProperty
		pickableProperties?: PickableProperties | undefined
		argName: string
	}

	let { focused = false, schemaProperty, pickableProperties = undefined, argName }: Props = $props()

	/** The button takes focus when clicked, which blurs the input — without tracking that,
	 *  asking for a suggestion would immediately look like leaving the field. */
	let btnFocused = $state(false)

	let abortController = new AbortController()
	let newFlowInput = $state('')

	const { flowStore, selectionManager } = getContext<FlowEditorContext>('FlowEditorContext')
	const { generatedExprs } = getContext<FlowCopilotContext | undefined>('FlowCopilotContext') || {}

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
		logStepInputFill('single')
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
			generatedContent = await getNonStreamingMetadataCompletion(
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

	// Drop a suggestion once the user has moved on, so the accept button can't sit there armed
	// against an input nobody is editing. Deferred because focus moves through nothing on its
	// way from the input to the button, and left alone while loading: the click that asked for
	// the suggestion is itself what blurred the input.
	$effect(() => {
		if (focused || btnFocused) return
		const timer = setTimeout(() => {
			if (!focused && !btnFocused && !loading) {
				cancel()
			}
		}, 150)
		return () => clearTimeout(timer)
	})

	$effect(() => {
		dispatch('showExpr', generatedContent)
	})

	$effect(() => {
		dispatch('showExpr', $generatedExprs?.[argName] || '')
	})

	let openInputsModal = $state(false)

	/** A suggestion is waiting to be accepted, rather than waiting to be asked for. */
	let ready = $derived(!loading && generatedContent.length > 0)

	function accept() {
		dispatch('setExpr', generatedContent)
		if (newFlowInput) {
			openInputsModal = true
		}
		generatedContent = ''
	}

	// A suggestion costs a model call, so nothing generates on its own — this button is the
	// only trigger, and the same control then accepts what it produced. Blur must not cancel:
	// clicking here takes focus out of the input, which is the gesture that started the call.
	function onClick() {
		if (loading) cancel()
		else if (ready) accept()
		else generateStepInput()
	}
</script>

{#if $copilotInfo.enabled && $stepInputCompletionEnabled}
	<FlowCopilotInputsModal
		on:confirmed={async () => {
			createFlowInput()
		}}
		bind:open={openInputsModal}
		inputs={[newFlowInput]}
	/>
	<!-- Sized to match FlowPlugConnect: it shares the control row with the connect plug. -->
	<Button
		variant="default"
		size="xs3"
		iconOnly
		{loading}
		clickableWhileLoading
		title={loading ? 'Cancel' : ready ? 'Accept the suggestion' : 'Suggest an expression with AI'}
		startIcon={{ icon: ready ? Check : Wand2 }}
		btnClasses={twMerge(
			AIBtnClasses(ready ? 'green' : 'default'),
			'bg-surface overflow-clip flex p-0'
		)}
		wrapperClasses={twMerge(
			'h-5 w-8 p-0 group-hover:opacity-100 transition-opacity',
			// Same reveal-on-hover as the connect plug beside it, but a request in flight or a
			// suggestion waiting to be accepted has to stay reachable once the pointer leaves.
			loading || ready ? '' : 'opacity-0'
		)}
		on:click={onClick}
		on:focus={() => (btnFocused = true)}
		on:blur={() => (btnFocused = false)}
	/>
{/if}
