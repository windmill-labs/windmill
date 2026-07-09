<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import type { FlowModule } from '$lib/gen'
	import { SecondsInput } from '$lib/components/common'

	interface Props {
		flowModule: FlowModule
		disabled?: boolean
	}

	let { flowModule = $bindable(), disabled = false }: Props = $props()

	let enabled = $derived(flowModule.delete_after_secs != null)

	let tip = $derived(
		'The logs, arguments and results of this flow step are permanently deleted after the configured delay once the flow completes (they may be briefly visible in the UI while running). This also applies to a failed step: the error will not be accessible. The deletion is irreversible. Set to 0 for immediate deletion.' +
			(disabled ? ' This option is only available on Windmill Enterprise Edition.' : '')
	)
</script>

<div class="flex flex-col gap-2">
	<Toggle
		{disabled}
		size="xs"
		textClass="text-xs font-normal text-primary"
		checked={enabled}
		on:change={() => {
			if (enabled) {
				flowModule.delete_after_secs = undefined
			} else {
				flowModule.delete_after_secs = 0
			}
		}}
		options={{
			right: 'Delete logs, arguments and results after the flow is complete',
			rightTooltip: tip
		}}
	/>
	<div class="mt-2">
		<SecondsInput
			bind:seconds={flowModule.delete_after_secs}
			disabled={disabled || !enabled}
			size="sm"
		/>
	</div>
</div>
