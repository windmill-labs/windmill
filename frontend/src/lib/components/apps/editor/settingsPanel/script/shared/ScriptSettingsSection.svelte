<script lang="ts">
	import Tooltip from '$lib/components/Tooltip.svelte'

	interface Props {
		title: string
		tooltip?: string | undefined
		children?: import('svelte').Snippet
	}

	let { title, tooltip = undefined, children }: Props = $props()
</script>

<div class="py-2">
	<div class="mb-2 text-sm font-semibold justify-between flex flex-row items-center">
		{title}

		{#if tooltip}
			<Tooltip>
				{tooltip}
			</Tooltip>
		{/if}
	</div>

	{@render children?.()}
</div>
