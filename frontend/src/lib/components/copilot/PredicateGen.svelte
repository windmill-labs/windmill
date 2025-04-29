<script lang="ts">
	import { Wand2 } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { getNonStreamingCompletion } from './lib'
	import { sendUserToast } from '$lib/toast'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import type { PickableProperties } from '../flows/previousResults'
	import YAML from 'yaml'
	import { sliceModules } from '../flows/flowStateUtils'
	import { dfs } from '../flows/dfs'
	import { yamlStringifyExceptKeys } from './utils'
	import { copilotInfo, stepInputCompletionEnabled } from '$lib/stores'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import type { Flow } from '$lib/gen'

	let loading = false
	export let pickableProperties: PickableProperties | undefined = undefined

	let instructions = ''
	let instructionsField: HTMLInputElement | undefined = undefined
	$: instructionsField && setTimeout(() => instructionsField?.focus(), 100)

	let abortController = new AbortController()
	const { flowStore, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	const dispatch = createEventDispatcher()

	async function generatePredicate() {
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
			const user = `I'm building a workflow which is a DAG of script steps.
The current step is ${$selectedId} and is a branching step (if-else). 
The user wants to generate a predicate for the branching condition.
Here's the user's request: ${instructions}
You can find the details of all the steps below:
${flowDetails}

Determine for the user the JavaScript expression for the branching condition composed of the previous results or the flow inputs.
All inputs start with either results. or flow_input. and are followed by the key of the input.
Here's a summary of the available data:
<available>
${YAML.stringify(availableData)}</available>
If the branching is made inside a for-loop, the iterator value is accessible as flow_input.iter.value
Only return the expression without any wrapper. Do not explain or discuss.`
			const result = await getNonStreamingCompletion(
				[
					{
						role: 'user',
						content: user
					}
				],
				abortController
			)

			dispatch('setExpr', result)
			dispatch('updateSummary', instructions)
		} catch (err) {
			if (!abortController.signal.aborted) {
				sendUserToast('Could not generate predicate: ' + err, true)
			}
		} finally {
			loading = false
		}
	}
</script>

{#if $copilotInfo.enabled && $stepInputCompletionEnabled}
	<Popover
		floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
		contentClasses="p-4 flex w-96"
	>
		<svelte:fragment slot="trigger">
			<Button
				color={loading ? 'red' : 'light'}
				size="xs"
				nonCaptureEvent={!loading}
				startIcon={{ icon: Wand2 }}
				iconOnly
				title="AI Assistant"
				btnClasses="min-h-[30px] text-violet-800 dark:text-violet-400 bg-violet-100 dark:bg-gray-700"
				{loading}
				clickableWhileLoading
				on:click={loading ? () => abortController?.abort() : () => {}}
			/>
		</svelte:fragment>
		<svelte:fragment slot="content" let:close>
			<input
				bind:this={instructionsField}
				type="text"
				placeholder="Predicate description"
				bind:value={instructions}
				on:keypress={({ key }) => {
					if (key === 'Enter' && instructions.length > 0) {
						close()
						generatePredicate()
					}
				}}
			/>
			<Button
				size="xs"
				color="light"
				variant="contained"
				buttonType="button"
				btnClasses="!p-1 !w-[38px] !ml-2 text-violet-800 dark:text-violet-400 bg-violet-100 dark:bg-gray-700"
				title="Generate predicate from prompt"
				aria-label="Generate"
				iconOnly
				on:click={() => {
					close()
					generatePredicate()
				}}
				disabled={instructions.length == 0}
				startIcon={{ icon: Wand2 }}
			/>
		</svelte:fragment>
	</Popover>
{/if}
