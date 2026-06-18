<script lang="ts">
	import Label from '$lib/components/Label.svelte'
	import Section from '$lib/components/Section.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	import type { FlowModule } from '$lib/gen'
	import { SecondsInput } from '../../common'

	interface Props {
		flowModule: FlowModule
	}

	let { flowModule = $bindable() }: Props = $props()

	let isCacheEnabled = $derived(Boolean(flowModule.cache_ttl))
</script>

<Section label="Cache" class="flex flex-col gap-4">
	{#snippet header()}
		<Tooltip documentationLink="https://www.windmill.dev/docs/flows/cache">
			If defined, the result of the step will be cached for the number of seconds defined such that
			if this step were to be re-triggered with the same input it would retrieve and return its
			cached value instead of recomputing it.
		</Tooltip>
	{/snippet}

	{#if flowModule.value.type != 'rawscript'}
		<p class="text-xs text-secondary">
			The cache settings need to be set in the referenced script/flow settings directly. Cache for
			hub scripts is not available yet.
		</p>
	{:else}
		<Toggle
			checked={isCacheEnabled}
			on:change={() => {
				if (isCacheEnabled && flowModule.cache_ttl != undefined) {
					flowModule.cache_ttl = undefined
				} else {
					flowModule.cache_ttl = 600
				}
			}}
			options={{
				right: 'Cache the results for each possible inputs'
			}}
		/>
		{#if flowModule.cache_ttl}
			<Label label="How long to keep cache valid">
				<SecondsInput bind:seconds={flowModule.cache_ttl} />
			</Label>
			<Toggle
				size="2xs"
				bind:checked={
					() => flowModule.cache_ignore_s3_path,
					(v) => (flowModule.cache_ignore_s3_path = v || undefined)
				}
				options={{
					right: 'Ignore S3 Object paths for caching purposes',
					rightTooltip:
						'If two S3 objects passed as input have the same content, they will hit the same cache entry, regardless of their path.'
				}}
			/>
		{/if}
	{/if}
</Section>
