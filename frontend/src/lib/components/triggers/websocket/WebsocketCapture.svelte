<script lang="ts">
	import type { CaptureInfo } from '../CaptureSection.svelte'
	import CaptureSection from '../CaptureSection.svelte'

	export let captureInfo: CaptureInfo | undefined = undefined
	export let isValid: boolean | undefined = undefined
	export let hasPreprocessor: boolean = false
	export let isFlow: boolean = false
</script>

{#if captureInfo}
	<CaptureSection
		captureType="websocket"
		disabled={isValid === false}
		{captureInfo}
		on:captureToggle
		on:applyArgs
		on:updateSchema
		on:addPreprocessor
		on:testWithArgs
		{hasPreprocessor}
		{isFlow}
	>
		<svelte:fragment slot="description">
			{#if captureInfo.active}
				Listenning to websocket events...
			{:else}
				Start capturing to listen to websocket events.
			{/if}
		</svelte:fragment>
	</CaptureSection>
{/if}
