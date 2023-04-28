<script lang="ts">
	import Tooltip from '$lib/components/Tooltip.svelte'

	export let title: string
	export let tooltip: string | undefined = undefined
</script>

<div class="p-2">
	<div class="mb-2 text-sm font-semibold justify-between flex flex-row items-center">
		{title}

		{#if tooltip}
			<Tooltip>
				{tooltip}
			</Tooltip>
		{/if}
	</div>

	<slot />
</div>
