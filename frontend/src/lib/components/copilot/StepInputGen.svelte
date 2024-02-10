<script lang="ts">
	import { Check, Loader2, Wand2 } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { getNonStreamingCompletion } from './lib'
	import { sendUserToast } from '$lib/toast'
	import type { InputTransform } from '$lib/gen'
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

	let generatedContent = ''
	let loading = false
	export let focused = false
	export let arg: InputTransform | any
	export let pickableProperties: PickableProperties | undefined = undefined
	export let argName: string

	let empty = false
	$: empty =
		!arg || (arg.type === 'static' && !arg.value) || (arg.type === 'javascript' && !arg.expr)

	let abortController = new AbortController()
	const { flowStore, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
	const { stepInputsLoading, generatedExprs } =
		getContext<FlowCopilotContext | undefined>('FlowCopilotContext') || {}

	async function generateStepInput() {
		abortController = new AbortController()
		loading = true
		const idOrders = dfs($flowStore.value.modules, (x) => x.id)
		const upToIndex = idOrders.indexOf($selectedId)
		if (upToIndex === -1) {
			throw new Error('Could not find the selected id in the flow')
		}

		const flowDetails =
			'Take into account the following information for never tested results:\n<flowDetails>\n' +
			yamlStringifyExceptKeys(sliceModules($flowStore.value.modules, upToIndex, idOrders), [
				'lock',
				'input_transforms'
			]) +
			'</flowDetails>'
		try {
			const availableData = {
				results: pickableProperties?.priorIds,
				flow_input: pickableProperties?.flow_input
			}
			const user = `I'm building a workflow which is a DAG of script steps.
The current step is ${selectedId}, you can find the details for the step and previous ones below:
${flowDetails}
Determine for the input "${argName}", what to pass either from the previous results of the flow inputs. Here's a summary of the available data:
<available>
${YAML.stringify(availableData)}</available>
If none of the available results are appropriate, are already used or are more appropriate for other inputs, you can also imagine new flow_input properties which we will create programmatically based on what you provide.
Reply with the most probable answer, do not explain or discuss.
Use javascript object dot notation to access the properties.
Return the input element directly: e.g. flow_input.property, results.a, results.a.property, flow_input.iter.value`

			generatedContent = await getNonStreamingCompletion(
				[
					{
						role: 'user',
						content: user
					}
				],
				abortController
			)
		} catch (err) {
			if (!abortController.signal.aborted) {
				sendUserToast('Could not generate summary: ' + err, true)
			}
		} finally {
			loading = false
		}
	}

	export function onKeyUp(event: KeyboardEvent) {
		if (!$copilotInfo.exists_openai_resource_path || !$stepInputCompletionEnabled) {
			return
		}
		if (event.key === 'Tab') {
			if (!loading && generatedContent) {
				event.preventDefault()
				dispatch('setExpr', generatedContent)
				generatedContent = ''
			}
		} else {
			cancel()
		}
	}

	const dispatch = createEventDispatcher()

	function automaticGeneration() {
		if (empty) {
			generateStepInput()
		}
	}

	$: if ($copilotInfo.exists_openai_resource_path && $stepInputCompletionEnabled && focused) {
		automaticGeneration()
	}

	function cancel() {
		abortController.abort()
		generatedContent = ''
	}
	$: if (!focused) {
		cancel()
	}

	let out = true // hack to prevent regenerating answer when accepting the answer due to mouseenter on new icon
</script>

{#if $copilotInfo.exists_openai_resource_path && $stepInputCompletionEnabled}
	<ManualPopover
		showTooltip={generatedContent.length > 0 || !!$generatedExprs?.[argName]}
		placement="bottom"
		class="p-2"
	>
		<Button
			size="xs"
			color="light"
			btnClasses="text-violet-800 dark:text-violet-400 bg-violet-100 dark:bg-gray-700 dark:hover:bg-surface-hover"
			on:click={() => {
				if (loading) {
					cancel()
				} else if (generatedContent.length > 0) {
					dispatch('setExpr', generatedContent)
					generatedContent = ''
				} else {
					generateStepInput()
				}
			}}
			on:mouseenter={(ev) => {
				if (!generatedContent && !loading && out) {
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
