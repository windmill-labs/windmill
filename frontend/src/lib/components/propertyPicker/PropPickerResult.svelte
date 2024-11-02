<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import ObjectViewer from './ObjectViewer.svelte'

	export let result: any
	export let extraResults: any = undefined
	export let flow_input: any = undefined

	const dispatch = createEventDispatcher()
</script>

<div class="w-full px-2">
	<span class="font-bold text-sm">Result</span>
	<div class="overflow-y-auto mb-2 w-full">
		<ObjectViewer
			allowCopy={false}
			json={{ result, ...(extraResults ? extraResults : {}) }}
			on:select
		/>
	</div>
	{#if flow_input}
		<span class="font-bold text-sm">Flow Input</span>
		<div class="overflow-y-auto w-full">
			<ObjectViewer
				allowCopy={false}
				json={flow_input}
				on:select={(e) => dispatch('select', `flow_input.${e.detail}`)}
			/>
		</div>
	{/if}
</div>
