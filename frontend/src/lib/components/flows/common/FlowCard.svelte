<script lang="ts">
	import type { FlowModuleValue } from '$lib/gen'
	import FlowCardHeader from './FlowCardHeader.svelte'
	export let title: string | undefined = undefined
	export let summary: string | undefined = undefined
	export let noEditor: boolean
	export let flowModuleValue: FlowModuleValue | undefined = undefined
</script>

<div class="flex flex-col h-full">
	{#if !noEditor}
		<div>
			<FlowCardHeader on:setHash on:reload {title} bind:summary {flowModuleValue}>
				<slot name="header" />
			</FlowCardHeader>
		</div>
	{/if}

	<div class="min-h-0 flex-grow">
		<slot />
	</div>
</div>
