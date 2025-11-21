<script lang="ts">
	import { Check, Loader2, Wand2 } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { getNonStreamingCompletion } from './lib'
	import { sendUserToast } from '$lib/toast'
	import type { Flow, InputTransform } from '$lib/gen'
	import ManualPopover from '../ManualPopover.svelte'
	import { createEventDispatcher, getContext, untrack } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import type { PickableProperties } from '../flows/previousResults'
	import YAML from 'yaml'
	import { sliceModules } from '../flows/flowStateUtils.svelte'
	import { dfs } from '../flows/dfs'
	import { yamlStringifyExceptKeys } from './utils'
	import { stepInputCompletionEnabled } from '$lib/stores'
	import { copilotInfo } from '$lib/aiStore'
	import { twMerge } from 'tailwind-merge'

	let generatedContent = $state('')
	let loading = $state(false)
	interface Props {
		focused?: boolean
		arg: InputTransform | any
		pickableProperties?: PickableProperties | undefined
	}

	let { focused = false, arg, pickableProperties = undefined }: Props = $props()

	let btnFocused = $state(false)

	let empty = $derived(
		Object.keys(arg ?? {}).length === 0 ||
			(arg.type === 'static' && !arg.value) ||
			(arg.type === 'javascript' && !arg.expr)
	)

	let abortController = new AbortController()
	const { flowStore, selectionManager } = getContext<FlowEditorContext>('FlowEditorContext')

	async function generateIteratorExpr() {
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
			const user = `I'm building a workflow which is a DAG of script steps.
The current step is ${selectionManager.getSelectedId()} and represents a for-loop. You can find the details of all the steps below:
${flowDetails}
Determine the iterator expression to pass either from the previous results or the flow inputs. Here's a summary of the available data:
<available>
${YAML.stringify(availableData)}</available>
Reply with the most probable answer, do not explain or discuss.
Use javascript object dot notation to access the properties.
Only output the expression, do not explain or discuss.`
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
				sendUserToast('Could not generate iterator expression: ' + err, true)
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
				generatedContent = ''
			}
		} else {
			cancel()
		}
	}

	const dispatch = createEventDispatcher()

	function automaticGeneration() {
		if (empty) {
			generateIteratorExpr()
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

	function cancel() {
		abortController.abort()
		generatedContent = ''
	}

	$effect(() => {
		dispatch('showExpr', generatedContent)
	})

	let out = $state(true) // hack to prevent regenerating answer when accepting the answer due to mouseenter on new icon
</script>

{#if $copilotInfo.enabled && $stepInputCompletionEnabled}
	<ManualPopover showTooltip={!empty && generatedContent.length > 0} placement="bottom" class="p-2">
		<Button
			size="xs"
			color="light"
			btnClasses={twMerge(
				'text-ai bg-violet-100 dark:bg-gray-700 dark:hover:bg-surface-hover',
				!loading && generatedContent.length > 0
					? 'bg-green-100 text-green-800 hover:bg-green-100 dark:text-green-400 dark:bg-green-700 dark:hover:bg-green-700'
					: ''
			)}
			on:click={() => {
				if (!loading && generatedContent.length > 0) {
					dispatch('setExpr', generatedContent)
					generatedContent = ''
				}
			}}
			on:focus={() => {
				btnFocused = true
			}}
			on:blur={() => {
				btnFocused = false
			}}
			on:mouseenter={(ev) => {
				if (out) {
					out = false
					generateIteratorExpr()
				}
			}}
			on:mouseleave={() => {
				out = true
				cancel()
			}}
			endIcon={{
				icon: loading ? Loader2 : generatedContent.length > 0 ? Check : Wand2,
				classes: loading ? 'animate-spin' : ''
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
		{#snippet content()}
			<div class="text-sm text-primary">
				{generatedContent}
			</div>
		{/snippet}
	</ManualPopover>
{/if}
