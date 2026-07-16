<script lang="ts">
	import type { FlowModuleValue } from '$lib/gen'
	import FlowCardHeader from './FlowCardHeader.svelte'

	interface Props {
		title?: string | undefined
		summary?: string | undefined
		description?: string | undefined
		noEditor: boolean
		noHeader?: boolean
		flowModuleValue?: FlowModuleValue | undefined
		header?: import('svelte').Snippet
		action?: import('svelte').Snippet
		children?: import('svelte').Snippet
		isAgentTool?: boolean
		siblingToolNames?: string[]
	}

	let {
		title = undefined,
		summary = $bindable(undefined),
		description = $bindable(undefined),
		noEditor,
		noHeader = false,
		flowModuleValue = undefined,
		header,
		action,
		children,
		isAgentTool = false,
		siblingToolNames = undefined
	}: Props = $props()
</script>

<div class="flex flex-col h-full">
	{#if !noEditor && !noHeader}
		<div>
			<FlowCardHeader
				on:setHash
				on:reload
				{title}
				bind:summary
				bind:description
				{flowModuleValue}
				{action}
				{isAgentTool}
				{siblingToolNames}
			>
				{@render header?.()}
			</FlowCardHeader>
		</div>
	{/if}

	<div class="min-h-0 flex-grow">
		{@render children?.()}
	</div>
</div>
