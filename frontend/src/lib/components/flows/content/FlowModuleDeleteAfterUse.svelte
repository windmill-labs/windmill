<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { FlowModule } from '$lib/gen'

	import Section from '$lib/components/Section.svelte'

	export let flowModule: FlowModule
</script>

<Section label="Delete after use">
	<svelte:fragment slot="header">
		<Tooltip>
			The logs, arguments and results of this flow step will be completely deleted from Windmill
			once the flow is complete. They might be temporarily visible in UI while the flow is running.
			<br />
			This also applies to a flow step that has failed: the error will not be accessible.
			<br />
			<br />
			The deletion is irreversible.
		</Tooltip>
	</svelte:fragment>

	<Toggle
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
			right: 'Delete logs, arguments and results after use'
		}}
	/>
</Section>
