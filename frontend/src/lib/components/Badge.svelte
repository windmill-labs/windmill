<script lang="ts">
	import Tooltip from './Tooltip.svelte'
	export let twBgColor = 'bg-blue-200'
	export let twTextColor = 'text-secondary'
	export let tooltip: string | undefined = undefined
</script>

<span class="{twBgColor} {twTextColor} text-2xs rounded px-1 whitespace-nowrap">
	<slot />
	{#if tooltip && tooltip != ''}
		<Tooltip>{tooltip}</Tooltip>
	{/if}
</span>
