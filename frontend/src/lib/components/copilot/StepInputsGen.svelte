<script lang="ts">
	import { base } from '$lib/base'
	import YAML from 'yaml'
	import { yamlStringifyExceptKeys } from './utils'
	import { sliceModules } from '../flows/flowStateUtils.svelte'
	import { dfs } from '../flows/dfs'
	import type { FlowEditorContext } from '../flows/types'
	import type { PickableProperties } from '../flows/previousResults'
	import { getContext } from 'svelte'
	import { getNonStreamingMetadataCompletion } from './lib'
	import { sendUserToast } from '$lib/toast'
	import Button from '../common/button/Button.svelte'
	import type { FlowCopilotContext } from './flow'
	import { logStepInputFill } from './stepInputFillTelemetry'
	import { Check, ExternalLink, Loader2, Wand2 } from 'lucide-svelte'
	import { stepInputCompletionEnabled } from '$lib/stores'
	import { copilotInfo } from '$lib/aiStore'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import type { SchemaProperty, Schema } from '$lib/common'
	import FlowCopilotInputsModal from './FlowCopilotInputsModal.svelte'
	import type { Flow } from '$lib/gen'
	import { twMerge } from 'tailwind-merge'
	import { AIBtnClasses } from './chat/AIButtonStyle'

	let loading = $state(false)
	interface Props {
		pickableProperties?: PickableProperties | undefined
		argNames?: string[]
		schema?: Schema | { properties?: Record<string, any> } | undefined
	}

	let { pickableProperties = undefined, argNames = [], schema = undefined }: Props = $props()

	const { flowStore, selectionManager } = getContext<FlowEditorContext>('FlowEditorContext')

	const { exprsToSet, stepInputsLoading, generatedExprs } =
		getContext<FlowCopilotContext | undefined>('FlowCopilotContext') || {}

	let generatedContent = ''
	let parsedInputs: string[][] = []
	let newFlowInputs: string[] = $state([])

	let abortController = $state(new AbortController())
	async function generateStepInputs() {
		if (Object.keys($generatedExprs || {}).length > 0 || loading) {
			return
		}
		logStepInputFill('all')
		abortController = new AbortController()
		loading = true
		stepInputsLoading?.set(true)
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

Determine for all the inputs "${argNames.join(
				'", "'
			)}", what to pass either from the previous results of the flow inputs.
All possibles inputs either start with results. or flow_input. and are followed by the key of the input.
${
	isInsideLoop
		? 'As the step is in a loop, the iterator value is accessible as flow_input.iter.value.'
		: 'As the step is not in a loop, flow_input.iter.value is not available.'
}
Here's a summary of the available data:
<available>
${YAML.stringify(availableData)}</available>
If none of the available results are appropriate, are already used or are more appropriate for other inputs, you can also imagine new flow_input properties which we will create programmatically based on what you provide.

Reply with the most probable answer, do not explain or discuss.
Use javascript object dot notation to access the properties.

Your answer has to be in the following format (one line per input):
input_name1: expression1
input_name2: expression2
...`
			generatedContent = await getNonStreamingMetadataCompletion(
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
			newFlowInputs = []
			for (const [key, value] of parsedInputs) {
				if (argNames.includes(key)) {
					exprs[key] = value
					if (
						pickableProperties &&
						value.startsWith('flow_input.') &&
						value.split('.')[1] &&
						!(value.split('.')[1] in pickableProperties.flow_input)
					) {
						newFlowInputs.push(value.split('.')[1])
					}
				}
			}
			generatedExprs?.set(exprs)
		} catch (err) {
			if (!abortController.signal.aborted) {
				sendUserToast('Could not generate step inputs: ' + err, true)
			}
		} finally {
			loading = false

			stepInputsLoading?.set(false)
		}
	}

	function createFlowInputs() {
		if (!newFlowInputs.length) {
			return
		}
		const properties = {
			...(flowStore.val.schema?.properties as Record<string, SchemaProperty> | undefined),
			...newFlowInputs.reduce((acc, x) => {
				acc[x] = (schema?.properties ?? {})[x]
				return acc
			}, {})
		}
		const required = [
			...((flowStore.val.schema?.required as string[] | undefined) ?? []),
			...newFlowInputs
		]
		flowStore.val.schema = {
			$schema: 'https://json-schema.org/draft/2020-12/schema',
			properties,
			required,
			type: 'object'
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
		if (newFlowInputs.length) {
			openInputsModal = true
		}
	}

	let openInputsModal = $state(false)

	let disabled = $derived(argNames.length === 0)

	/** Suggestions are in hand and waiting to be applied, rather than waiting to be asked for. */
	let ready = $derived(!loading && Object.keys($generatedExprs || {}).length > 0)

	function cancel() {
		abortController.abort()
		generatedExprs?.set({})
	}

	// Filling every input costs a model call, so it takes a deliberate click — the pointer
	// merely crossing the button must not spend one. The same control then applies the result.
	function onClick() {
		if (loading) cancel()
		else if (ready) applyExprs()
		else generateStepInputs()
	}
</script>

<div class="flex flex-row justify-end">
	{#if $copilotInfo.enabled && $stepInputCompletionEnabled}
		<FlowCopilotInputsModal
			on:confirmed={async () => {
				createFlowInputs()
			}}
			bind:open={openInputsModal}
			inputs={newFlowInputs}
		/>
		<Button
			size="xs"
			wrapperClasses="flex-1"
			variant="default"
			btnClasses={twMerge(!disabled && AIBtnClasses(ready ? 'green' : 'default'))}
			on:click={onClick}
			on:blur={() => {
				// Suggestions belong to the moment they were asked for; leaving the button drops
				// them so it can't sit on "Accept" against inputs the user has moved on from.
				// A request still in flight is left alone — it was asked for deliberately.
				if (!loading) cancel()
			}}
			startIcon={{
				icon: loading ? Loader2 : ready ? Check : Wand2,
				classes: loading ? 'animate-spin' : ''
			}}
			{disabled}
		>
			{#if loading}
				Cancel
			{:else if ready}
				Accept
			{:else}
				Fill inputs
			{/if}
		</Button>
	{:else if !$copilotInfo.workspaceDisabled}
		<Popover
			floatingConfig={{
				placement: 'top-end'
			}}
			class="w-full"
		>
			{#snippet trigger()}
				<Button
					size="xs"
					variant="default"
					btnClasses={AIBtnClasses('default')}
					nonCaptureEvent
					startIcon={{
						icon: Wand2
					}}
				>
					Fill inputs
				</Button>
			{/snippet}
			{#snippet content({ close })}
				<div class="p-4">
					<p class="text-sm">
						{#if !$copilotInfo.enabled}
							Enable Windmill AI in the{' '}
							<a
								href="{base}/workspace_settings?tab=ai"
								target="_blank"
								class="inline-flex flex-row items-center gap-1"
							>
								workspace settings <ExternalLink size={16} />
							</a>
						{:else}
							Enable step input completion in the{' '}
							<a
								href="#user-settings"
								class="inline-flex flex-row items-center gap-1"
								onclick={() => {
									close()
								}}
							>
								user settings
							</a>
						{/if}
					</p>
				</div>
			{/snippet}
		</Popover>
	{/if}
</div>
