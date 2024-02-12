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
	import Popup from '../common/popup/Popup.svelte'

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
		const idOrders = dfs($flowStore.value.modules, (x) => x.id)
		const upToIndex = idOrders.indexOf($selectedId)
		if (upToIndex === -1) {
			throw new Error('Could not find the selected id in the flow')
		}

		const flowDetails =
			'Take into account the following information for never tested results:\n<flowDetails>\n' +
			yamlStringifyExceptKeys(sliceModules($flowStore.value.modules, upToIndex, idOrders), [
				'lock'
			]) +
			'</flowDetails>'
		try {
			const availableData = {
				results: pickableProperties?.priorIds,
				flow_input: pickableProperties?.flow_input
			}
			const user = `I'm building a workflow which is a DAG of script steps.
The current step is ${selectedId} and is a branching step (if-else). 
The user wants to generate a predicate for the branching condition.
Here's the user's request: ${instructions}
You can find the details of all the steps below:
${flowDetails}

Determine for the user the javascript expression for the branching condition composed of the previous results or the flow inptus.
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
		} catch (err) {
			if (!abortController.signal.aborted) {
				sendUserToast('Could not generate summary: ' + err, true)
			}
		} finally {
			loading = false
		}
	}
</script>

<!-- {#if $copilotInfo.exists_openai_resource_path && $stepInputCompletionEnabled}
	<ManualPopover showTooltip={generatedContent.length > 0} placement="bottom" class="p-2">
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
					generatePredicate()
				}
			}}
			endIcon={{
				icon: loading ? Loader2 : generatedContent.length > 0 ? Check : Wand2,
				classes: loading ? 'animate-spin' : ''
			}}
		/>
		<svelte:fragment slot="content">
			<div class="text-sm text-tertiary">
				{generatedContent}
			</div>
		</svelte:fragment>
	</ManualPopover>
{/if} -->

{#if $copilotInfo.exists_openai_resource_path && $stepInputCompletionEnabled}
	<Popup
		floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
		containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
		let:close
	>
		<svelte:fragment slot="button">
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
				on:click={loading ? () => abortController?.abort() : undefined}
			/>
		</svelte:fragment>
		<div class="flex w-96">
			<input
				bind:this={instructionsField}
				type="text"
				placeholder="Predicate description"
				bind:value={instructions}
				on:keypress={({ key }) => {
					if (key === 'Enter' && instructions.length > 0) {
						close(instructionsField || null)
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
					close(instructionsField || null)
					generatePredicate()
				}}
				disabled={instructions.length == 0}
				startIcon={{ icon: Wand2 }}
			/>
		</div>
	</Popup>
{/if}
