<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import type { FlowModule } from '$lib/gen'
	import { stepSettingDefaults } from '../flowStepSettings'
	import { SecondsInput } from '$lib/components/common'
	import { slideDynamic } from '$lib/transitions'

	interface Props {
		flowModule: FlowModule
		disabled?: boolean
	}

	let { flowModule = $bindable(), disabled = false }: Props = $props()

	let enabled = $derived(flowModule.delete_after_secs != null)

	const tip =
		'The logs, arguments and results of this flow step are permanently deleted after the configured delay once the flow completes (they may be briefly visible in the UI while running). This also applies to a failed step: the error will not be accessible. The deletion is irreversible. Set to 0 for immediate deletion.'
</script>

<div class="flex flex-col gap-2">
	<Toggle
		{disabled}
		eeOnly
		size="xs"
		textClass="text-xs font-normal text-primary"
		checked={enabled}
		on:change={() => {
			if (enabled) {
				flowModule.delete_after_secs = undefined
			} else {
				flowModule.delete_after_secs = stepSettingDefaults('lifetime')
			}
		}}
		options={{
			right: 'Delete after use',
			rightTooltip: tip
		}}
	/>
	{#if enabled}
		<div class="pl-9" transition:slideDynamic>
			<SecondsInput bind:seconds={flowModule.delete_after_secs} {disabled} size="sm" />
		</div>
	{/if}
</div>
