<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { FlowModule } from '$lib/gen'

	import JsonEditor from '$lib/components/apps/editor/settingsPanel/inputEditor/JsonEditor.svelte'
	import Section from '$lib/components/Section.svelte'

	export let flowModule: FlowModule

	let code: string | undefined = flowModule.mock?.return_value
		? JSON.stringify(flowModule.mock?.return_value, null, 2)
		: undefined

	$: isMockEnabled = Boolean(flowModule.mock?.enabled)
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
		{#if flowModule?.mock?.return_value != undefined}
			<JsonEditor {code} bind:value={flowModule.mock.return_value} />
		{:else}
			<input
				type="text"
				disabled
				value={flowModule.mock?.return_value ? code : ''}
				class="w-full p-2 border rounded-md"
			/>
		{/if}
	</div>
</Section>
