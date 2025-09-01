<script lang="ts">
	import Tooltip from '$lib/components/Tooltip.svelte'

	interface Props {
		label: string
		children: import('svelte').Snippet
		tooltip?: import('svelte').Snippet
		for?: string
		noLabel?: boolean
	}

	let { label, children, tooltip, for: forAttr = '', noLabel = false }: Props = $props()
</script>

<div class="flex flex-col items-start w-fit">
	<div class="text-xs truncate">
		{#if forAttr}
			<label for={forAttr} class:text-transparent={noLabel}>{label}</label>
		{:else}
			<span class:text-transparent={noLabel}>{label}</span>
		{/if}
		{#if tooltip}
			<Tooltip small>{@render tooltip()}</Tooltip>
		{/if}
	</div>

	<div class="flex flex-row gap-1 items-center justify-start w-full relative h-[34px]">
		{@render children()}
	</div>
</div>
