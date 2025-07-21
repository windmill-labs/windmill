<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { FlowModule } from '$lib/gen'

	import Section from '$lib/components/Section.svelte'

	interface Props {
		flowModule: FlowModule
		disabled?: boolean
	}

	let { flowModule = $bindable(), disabled = false }: Props = $props()
</script>

<Section label="Delete after use">
	{#snippet header()}
		<Tooltip>
			The logs, arguments and results of this flow step will be completely deleted from Windmill
			once the flow is complete. They might be temporarily visible in UI while the flow is running.
			<br />
			This also applies to a flow step that has failed: the error will not be accessible.
			<br />
			<br />
			The deletion is irreversible.
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
		checked={Boolean(flowModule.delete_after_use)}
		on:change={() => {
			if (flowModule.delete_after_use) {
				flowModule.delete_after_use = undefined
			} else {
				flowModule.delete_after_use = true
			}
		}}
		options={{
			right: 'Delete logs, arguments and results after the flow is complete'
		}}
	/>
</Section>
