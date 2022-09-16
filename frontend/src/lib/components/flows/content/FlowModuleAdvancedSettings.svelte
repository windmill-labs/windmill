<script lang="ts">
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'

	import Toggle from '$lib/components/Toggle.svelte'

	import type { FlowModule } from '$lib/gen'

	export let flowModule: FlowModule
</script>

<div class="font-bold text-sm mb-2">Suspend</div>
<div class="space-y-4 ">
	<input bind:value={flowModule.suspend} type="number" min="0" placeholder="0" />

	{#if flowModule.stop_after_if}
		<button
			class="flex items-center  text-white bg-blue-500 hover:bg-blue-700 focus:ring-4 focus:ring-blue-300 font-medium rounded-lg text-sm px-4 py-1 focus:outline-none "
			on:click={() => {
				flowModule.stop_after_if = undefined
			}}
		>
			Disable stop expression
		</button>
		<div class="flex flex-col">
			<div class="font-bold text-sm mb-2">Stop condition expression</div>

			<SimpleEditor
				lang="javascript"
				bind:code={flowModule.stop_after_if.expr}
				class="few-lines-editor border"
			/>
			<Toggle
				bind:checked={flowModule.stop_after_if.skip_if_stopped}
				options={{
					right: 'Should skip if stopped'
				}}
				on:change
			/>
		</div>
	{:else}
		<button
			class="flex items-center  text-white bg-blue-500 hover:bg-blue-700 focus:ring-4 focus:ring-blue-300 font-medium rounded-lg text-sm px-4 py-1 focus:outline-none "
			on:click={() => {
				flowModule.stop_after_if = {
					skip_if_stopped: false,
					expr: 'result == undefined'
				}
			}}
		>
			Enable stop expression
		</button>
	{/if}
</div>
