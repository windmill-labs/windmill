<script lang="ts">
	import Tooltip from './Tooltip.svelte'
	interface Props {
		twBgColor?: string;
		twTextColor?: string;
		tooltip?: string | undefined;
		children?: import('svelte').Snippet;
	}

	let {
		twBgColor = 'bg-blue-200',
		twTextColor = 'text-secondary',
		tooltip = undefined,
		children
	}: Props = $props();
</script>

<span class="{twBgColor} {twTextColor} text-2xs rounded px-1 whitespace-nowrap">
	{@render children?.()}
	{#if tooltip && tooltip != ''}
		<Tooltip>{tooltip}</Tooltip>
	{/if}
</span>
