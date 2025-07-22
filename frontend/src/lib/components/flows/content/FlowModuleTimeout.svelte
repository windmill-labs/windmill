<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	import type { FlowModule } from '$lib/gen'
	import { Alert, SecondsInput } from '../../common'

	interface Props {
		flowModule: FlowModule
	}

	let { flowModule = $bindable() }: Props = $props()

	let istimeoutEnabled = $derived(Boolean(flowModule.timeout))
</script>

<Section label="Timeout">
	{#snippet header()}
		<Tooltip>
			If defined, the custom timeout will be used instead of the instance timeout for the step. The
			step's timeout cannot be greater than the instance timeout.
		</Tooltip>
	{/snippet}

	<Toggle
		checked={istimeoutEnabled}
		on:change={() => {
			if (istimeoutEnabled && flowModule.timeout != undefined) {
				flowModule.timeout = undefined
			} else {
				flowModule.timeout = 300
			}
		}}
		options={{
			right: 'Add a custom timeout for this step'
		}}
	/>
	<div class="mb-4">
		<span class="text-xs font-bold">Timeout duration</span>

		{#if flowModule.timeout}
			<SecondsInput bind:seconds={flowModule.timeout} />
		{:else}
			<SecondsInput disabled />
		{/if}

		<div class="mt-4"></div>

		<Alert title="Only used when testing the full flow" type="info">
			<p class="text-sm"> The timeout will be ignored when running "Test this step" </p>
		</Alert>
	</div>
</Section>
