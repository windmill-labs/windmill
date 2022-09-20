<script lang="ts">
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'

	import Toggle from '$lib/components/Toggle.svelte'

	import type { FlowModule } from '$lib/gen'

	export let flowModule: FlowModule

	$: isSuspendEnabled = Boolean(flowModule.suspend)
	$: isStopAfterIfEnabled = Boolean(flowModule.stop_after_if)
</script>

<div class="flex flex-col items-start space-y-2">
	<Toggle
		checked={isSuspendEnabled}
		on:change={() => {
			if (isSuspendEnabled && flowModule.suspend != undefined) {
				flowModule.suspend = undefined
			} else {
				flowModule.suspend = 1
			}
		}}
		options={{
			right: 'Suspend flow execution until events received enabled'
		}}
	/>
	<span class="text-xs font-bold">Number of events to wait for</span>

	{#if flowModule.suspend}
		<input bind:value={flowModule.suspend} type="number" min="1" placeholder="1" />
	{:else}
		<input type="number" disabled />
	{/if}

	<Toggle
		checked={isStopAfterIfEnabled}
		on:change={() => {
			if (isStopAfterIfEnabled && flowModule.stop_after_if) {
				flowModule.stop_after_if = undefined
			} else {
				flowModule.stop_after_if = {
					expr: 'result == undefined',
					skip_if_stopped: false
				}
			}
		}}
		options={{
			right: 'Early stop if condition met enabled'
		}}
	/>

	{#if flowModule.stop_after_if}
		<span class="text-xs font-bold">Stop condition expression</span>

		<SimpleEditor
			lang="javascript"
			bind:code={flowModule.stop_after_if.expr}
			class="few-lines-editor border w-full"
		/>
		<span class="text-xs font-bold">Should skip if stopped</span>

		<input type="checkbox" bind:checked={flowModule.stop_after_if.skip_if_stopped} />
	{:else}
		<span class="text-xs font-bold">Stop condition expression</span>
		<textarea disabled rows="3" />
		<span class="text-xs font-bold">Should skip if stopped</span>
		<input type="checkbox" disabled />
	{/if}
</div>
