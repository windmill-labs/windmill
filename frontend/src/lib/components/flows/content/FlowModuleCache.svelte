<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	import type { FlowModule } from '$lib/gen'
	import { SecondsInput } from '../../common'

	export let flowModule: FlowModule

	$: isCacheEnabled = Boolean(flowModule.cache_ttl)
</script>

<h2 class="pb-4">
	Cache
	<Tooltip documentationLink="https://www.windmill.dev/docs/flows/cache">
		If defined, the result of the step will be cached for the number of seconds defined such that if
		this step were to be re-triggered with the same input it would retrieve and return its cached
		value instead of recomputing it.
	</Tooltip>
</h2>
<Toggle
	checked={isCacheEnabled}
	on:change={() => {
		if (isCacheEnabled && flowModule.cache_ttl != undefined) {
			flowModule.cache_ttl = undefined
		} else {
			flowModule.cache_ttl = 60 * 60 * 24 * 2
		}
	}}
	options={{
		right: 'Cache the results for each possible inputs'
	}}
/>
<div class="mb-4">
	<span class="text-xs font-bold">How long to keep cache valid</span>

	{#if flowModule.cache_ttl}
		<SecondsInput bind:seconds={flowModule.cache_ttl} />
	{:else}
		<SecondsInput disabled />
	{/if}
</div>
