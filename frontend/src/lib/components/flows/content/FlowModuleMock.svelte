<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { FlowModule } from '$lib/gen'

	import JsonEditor from '$lib/components/JsonEditor.svelte'
	import Section from '$lib/components/Section.svelte'

	export let flowModule: FlowModule

	let code: string | undefined = flowModule.mock?.return_value
		? JSON.stringify(flowModule.mock?.return_value, null, 2)
		: undefined
	let isMockEnabled: boolean | undefined = Boolean(flowModule.mock?.enabled)

	// Track the last value to prevent circular updates
	let lastMockValue = JSON.stringify(flowModule.mock)
	let renderCount = 0

	function updateMock(
		newMock: { enabled?: boolean | undefined; return_value?: unknown } | undefined
	) {
		if (!newMock) return

		const newMockString = JSON.stringify(newMock)

		// Only update if it's actually a new value
		if (newMockString !== lastMockValue) {
			renderCount++
			lastMockValue = newMockString
			code = newMock.return_value ? JSON.stringify(newMock.return_value, null, 2) : undefined
			isMockEnabled = Boolean(newMock?.enabled)
		}
	}
	$: updateMock(flowModule.mock)

	function updateMockValue({ detail }: any) {
		const newMock = {
			enabled: true,
			return_value: detail
		}

		// Update the last value to prevent circular updates
		lastMockValue = JSON.stringify(newMock)
		flowModule.mock = newMock
	}
</script>

<Section label="Mock">
	<svelte:fragment slot="header">
		<Tooltip>
			If defined and enabled, the step will immediately return the mock value instead of being
			executed.
		</Tooltip>
	</svelte:fragment>

	<Toggle
		checked={isMockEnabled}
		on:change={() => {
			if (isMockEnabled) {
				flowModule.mock = {
					enabled: false,
					return_value: flowModule.mock?.return_value
				}
			} else {
				flowModule.mock = {
					enabled: true,
					return_value: flowModule.mock?.return_value ?? { example: 'value' }
				}
				code = JSON.stringify(flowModule.mock?.return_value, null, 2)
			}
		}}
		options={{
			right: 'Enable step mocking'
		}}
	/>
	<div>
		<span class="text-xs font-bold">Mocked Return value</span>
		{#if isMockEnabled}
			{#key renderCount}
				<JsonEditor {code} on:changeValue={updateMockValue} />
			{/key}
		{:else}
			<pre class="text-xs border rounded p-2 bg-surface-disabled"
				>{flowModule.mock?.return_value
					? JSON.stringify(flowModule.mock?.return_value, null, 2)
					: ''}</pre
			>
		{/if}
	</div>
</Section>
