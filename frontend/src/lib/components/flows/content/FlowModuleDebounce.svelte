<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import DebounceLimit from '../DebounceLimit.svelte'

	let {
		flowModule = $bindable(),
		selectedId
	}: {
		flowModule: FlowModule
		selectedId: string
	} = $props()

	const { flowStateStore } = getContext<FlowEditorContext>('FlowEditorContext')

	let debounce_delay_s = $state(flowModule.debouncing?.debounce_delay_s)
	let debounce_key = $state(flowModule.debouncing?.debounce_key)
	let debounce_args_to_accumulate = $state(flowModule.debouncing?.debounce_args_to_accumulate)
	let max_total_debouncing_time = $state(flowModule.debouncing?.max_total_debouncing_time)
	let max_total_debounces_amount = $state(flowModule.debouncing?.max_total_debounces_amount)

	// Re-sync local state when flowModule identity changes (e.g. switching selected module)
	$effect(() => {
		// Reading flowModule.id to track identity changes
		flowModule.id
		debounce_delay_s = flowModule.debouncing?.debounce_delay_s
		debounce_key = flowModule.debouncing?.debounce_key
		debounce_args_to_accumulate = flowModule.debouncing?.debounce_args_to_accumulate
		max_total_debouncing_time = flowModule.debouncing?.max_total_debouncing_time
		max_total_debounces_amount = flowModule.debouncing?.max_total_debounces_amount
	})

	// Write back to flowModule when local state changes
	$effect(() => {
		if (debounce_delay_s !== undefined && debounce_delay_s > 0) {
			flowModule.debouncing = {
				debounce_delay_s,
				debounce_key,
				debounce_args_to_accumulate,
				max_total_debouncing_time,
				max_total_debounces_amount
			}
		} else {
			flowModule.debouncing = undefined
		}
	})
</script>

<DebounceLimit
	bind:debounce_delay_s
	bind:debounce_key
	bind:debounce_args_to_accumulate
	bind:max_total_debouncing_time
	bind:max_total_debounces_amount
	schema={flowStateStore.val[selectedId]?.schema}
	placeholder={`$workspace/flow/<flow_path>-${flowModule.id}`}
	size="xs"
/>
