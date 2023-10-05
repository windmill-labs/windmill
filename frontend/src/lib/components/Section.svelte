<script lang="ts">
	import Tooltip from './Tooltip.svelte'

	export let label: string | undefined = undefined
	export let tooltip: string | undefined = undefined
</script>

<div>
	<div class="flex flex-row justify-between">
		<h2 class="text-base font-semibold leading-non mb-2">
			{label}
			<slot name="header" />
			{#if tooltip}
				<Tooltip>{tooltip}</Tooltip>
			{/if}
		</h2>
		<slot name="action" />
	</div>

	<slot />
</div>
