<script lang="ts">
	import { enterpriseLicense } from '$lib/stores'
	import { AlertTriangle } from 'lucide-svelte'
	import Tooltip from './Tooltip.svelte'

	export let label: string | undefined = undefined
	export let tooltip: string | undefined = undefined
	export let eeOnly = false
</script>

<div class="w-full">
	<div class="flex flex-row justify-between">
		<h2 class="text-base font-semibold mb-2 flex flex-row gap-1">
			{label}
			<slot name="header" />
			{#if tooltip}
				<Tooltip>{tooltip}</Tooltip>
			{/if}
			{#if eeOnly}
				{#if !$enterpriseLicense}
					<div class="flex text-xs items-center gap-1 text-yellow-500 whitespace-nowrap ml-8">
						<AlertTriangle size={16} />
						EE only <Tooltip>Enterprise Edition only feature</Tooltip>
					</div>
				{/if}
			{/if}
		</h2>
		<slot name="action" />
	</div>
	<div class={$$props.class}>
		<slot />
	</div>
</div>
