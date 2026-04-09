<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { FlowModule } from '$lib/gen'
	import { SecondsInput } from '$lib/components/common'

	import Section from '$lib/components/Section.svelte'

	interface Props {
		flowModule: FlowModule
		disabled?: boolean
	}

	let { flowModule = $bindable(), disabled = false }: Props = $props()

	let enabled = $derived(flowModule.delete_after_secs != null)
</script>

<Section label="Delete after completion">
	{#snippet header()}
		<Tooltip>
			The logs, arguments and results of this flow step will be completely deleted from Windmill
			after the specified delay once the flow is complete. They might be temporarily visible in UI
			while the flow is running.
			<br />
			This also applies to a flow step that has failed: the error will not be accessible.
			<br />
			<br />
			The deletion is irreversible. Set to 0 for immediate deletion.
			{#if disabled}
				<br />
				<br />
				This option is only available on Windmill Enterprise Edition.
			{/if}
		</Tooltip>
	{/snippet}

	<Toggle
		{disabled}
		size="sm"
		checked={enabled}
		on:change={() => {
			if (enabled) {
				flowModule.delete_after_secs = undefined
			} else {
				flowModule.delete_after_secs = 0
			}
		}}
		options={{
			right: 'Delete logs, arguments and results after the flow is complete'
		}}
	/>
	{#if enabled}
		<div class="mt-2">
			<SecondsInput bind:seconds={flowModule.delete_after_secs} {disabled} size="sm" />
		</div>
	{/if}
</Section>
