<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { FlowModule } from '$lib/gen'

	import JsonEditor from '$lib/components/apps/editor/settingsPanel/inputEditor/JsonEditor.svelte'

	export let flowModule: FlowModule

	let code: string | undefined = flowModule.mock?.return_value
		? JSON.stringify(flowModule.mock?.return_value, null, 2)
		: undefined

	$: isMockEnabled = Boolean(flowModule.mock?.enabled)
</script>

<h2 class="pb-4">
	Mock
	<Tooltip>
		If defined and enabled, the step will immediately return the mock value instead of being
		executed.
	</Tooltip>
</h2>
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
		console.log(isMockEnabled, flowModule.mock)
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
