<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { Button, Popup, SecondsInput } from '$lib/components/common'
	import { autoPlacement } from '@floating-ui/core'

	export let cache_ttl: number | undefined
</script>

<Popup
	floatingConfig={{
		middleware: [
			autoPlacement({
				allowedPlacements: ['bottom-start', 'bottom-end', 'top-start', 'top-end', 'top', 'bottom']
			})
		]
	}}
>
	<svelte:fragment slot="button">
		<Button
			nonCaptureEvent={true}
			btnClasses={Boolean(cache_ttl)
				? 'bg-blue-100 text-blue-800 border border-blue-300 hover:bg-blue-200 dark:bg-frost-700 dark:text-frost-100 dark:border-frost-600'
				: 'bg-surface text-primay hover:bg-hover'}
			color="light"
			variant="border"
			size="xs2"
		>
			Cache
		</Button>
	</svelte:fragment>
	<div class="block text-primary">
		<Toggle
			checked={Boolean(cache_ttl)}
			on:change={() => {
				if (cache_ttl != undefined) {
					cache_ttl = undefined
				} else {
					cache_ttl = 600
				}
			}}
			options={{
				right: 'Cache the results for each possible inputs'
			}}
		/>
		<div class="mb-4">
			<span class="text-xs font-bold">How long to keep cache valid</span>

			{#if cache_ttl}
				<SecondsInput bind:seconds={cache_ttl} />
			{:else}
				<SecondsInput disabled />
			{/if}
		</div>
	</div>
</Popup>
