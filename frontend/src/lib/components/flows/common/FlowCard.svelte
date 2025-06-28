<script lang="ts">
	import type { FlowModuleValue } from '$lib/gen'
	import FlowCardHeader from './FlowCardHeader.svelte'

	interface Props {
		title?: string | undefined
		summary?: string | undefined
		noEditor: boolean
		noHeader?: boolean
		flowModuleValue?: FlowModuleValue | undefined
		header?: import('svelte').Snippet
		children?: import('svelte').Snippet
	}

	let {
		title = undefined,
		summary = $bindable(undefined),
		noEditor,
		noHeader = false,
		flowModuleValue = undefined,
		header,
		children
	}: Props = $props()
</script>

<div class="flex flex-col h-full">
	{#if !noEditor && !noHeader}
		<div>
			<FlowCardHeader on:setHash on:reload {title} bind:summary {flowModuleValue}>
				{@render header?.()}
			</FlowCardHeader>
		</div>
	{/if}

	<div class="min-h-0 flex-grow">
		{@render children?.()}
	</div>
</div>
