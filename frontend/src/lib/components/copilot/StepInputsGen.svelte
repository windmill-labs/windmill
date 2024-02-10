<script lang="ts">
	import YAML from 'yaml'
	import { yamlStringifyExceptKeys } from './utils'
	import { sliceModules } from '../flows/flowStateUtils'
	import { dfs } from '../flows/dfs'
	import type { FlowEditorContext } from '../flows/types'
	import type { PickableProperties } from '../flows/previousResults'
	import { getContext } from 'svelte'
	import { getNonStreamingCompletion } from './lib'
	import { sendUserToast } from '$lib/toast'
	import Button from '../common/button/Button.svelte'
	import type { FlowCopilotContext } from './flow'
	import { Check, ExternalLink, Loader2, Wand2 } from 'lucide-svelte'
	import { copilotInfo, stepInputCompletionEnabled } from '$lib/stores'
	import { Popup } from '../common'

	let loading = false
	export let pickableProperties: PickableProperties | undefined = undefined
	export let argNames: string[] = []

	const { flowStore, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	const { exprsToSet, stepInputsLoading, generatedExprs } =
		getContext<FlowCopilotContext | undefined>('FlowCopilotContext') || {}

	let generatedContent = ''
	let parsedInputs: string[][] = []

	let abortController = new AbortController()
	async function generateStepInputs() {
		abortController = new AbortController()
		loading = true
		stepInputsLoading?.set(true)
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

Determine for the inputs "${argNames.join(
				'", "'
			)}", what to pass either from the previous results of the flow inputs. Here's a summary of the available data:
<available>
${YAML.stringify(availableData)}</available>
If none of the available results are appropriate, are already used or are more appropriate for other inputs, you can also imagine new flow_input properties which we will create programmatically based on what you provide.

Reply with the most probable answer, do not explain or discuss.
Use javascript object dot notation to access the properties.
Return the input element directly: e.g. flow_input.property, results.a, results.a.property, flow_input.iter.value

Your answer has to be in the following format (one line per input):
input_name: expr`

			console.log(user)

			generatedContent = await getNonStreamingCompletion(
				[
					{
						role: 'user',
						content: user
					}
				],
				abortController
			)

			parsedInputs = generatedContent.split('\n').map((x) => x.split(': '))

			const exprs = {}
			for (const [key, value] of parsedInputs) {
				if (argNames.includes(key)) {
					exprs[key] = value
				}
			}
			generatedExprs?.set(exprs)
		} catch (err) {
			if (!abortController.signal.aborted) {
				sendUserToast('Could not generate summary: ' + err, true)
			}
		} finally {
			loading = false

			stepInputsLoading?.set(false)
		}
	}

	function applyExprs() {
		const argsUpdate = {}
		for (const [key, value] of parsedInputs) {
			if (argNames.includes(key)) {
				argsUpdate[key] = {
					type: 'javascript',
					expr: value
				}
			}
		}
		exprsToSet?.set(argsUpdate)
		generatedExprs?.set({})
	}

	let out = true // hack to prevent regenerating answer when accepting the answer due to mouseenter on new icon
</script>

<div class="flex flex-row justify-end">
	{#if $copilotInfo.exists_openai_resource_path && $stepInputCompletionEnabled}
		<Button
			size="xs"
			color="light"
			btnClasses="text-violet-800 dark:text-violet-400"
			on:mouseenter={(ev) => {
				if (Object.keys($generatedExprs || {}).length === 0 && !loading && out) {
					out = false
					generateStepInputs()
				}
			}}
			on:mouseleave={() => {
				out = true
				abortController.abort()
				generatedExprs?.set({})
			}}
			on:click={() => {
				if (loading) {
					abortController.abort()
				} else if (Object.keys($generatedExprs || {}).length > 0) {
					applyExprs()
				} else if (!loading) {
					generateStepInputs()
				}
			}}
			startIcon={{
				icon: loading ? Loader2 : Object.keys($generatedExprs || {}).length > 0 ? Check : Wand2,
				classes: loading ? 'animate-spin' : ''
			}}
			disabled={argNames.length === 0}
		>
			{#if loading}
				Loading
			{:else if Object.keys($generatedExprs || {}).length > 0}
				Accept
			{:else}
				Fill inputs
			{/if}
		</Button>
	{:else}
		<Popup
			floatingConfig={{
				placement: 'top-end'
			}}
			let:close
		>
			<svelte:fragment slot="button">
				<Button
					size="xs"
					color="light"
					btnClasses="text-violet-800 dark:text-violet-400"
					nonCaptureEvent
					startIcon={{
						icon: Wand2
					}}
				>
					Fill inputs
				</Button>
			</svelte:fragment>
			<p class="text-sm">
				{#if !$copilotInfo.exists_openai_resource_path}
					Enable Windmill AI in the{' '}
					<a
						href="/workspace_settings?tab=openai"
						target="_blank"
						class="inline-flex flex-row items-center gap-1"
					>
						workspace settings <ExternalLink size={16} />
					</a>
				{:else}
					Enable step input completion in the{' '}
					<a href="#user-settings" class="inline-flex flex-row items-center gap-1" on:click={() => {
						close(null)
					}}>
						user settings
					</a>
				{/if}
			</p>
		</Popup>
	{/if}
</div>
