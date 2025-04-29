<script lang="ts">
	import type { CaptureInfo } from '../CaptureSectionV2.svelte'
	import CaptureSection from '../CaptureSectionV2.svelte'

	export let captureInfo: CaptureInfo | undefined = undefined
	export let isValid: boolean | undefined = undefined
	export let hasPreprocessor: boolean = false
	export let isFlow: boolean = false
</script>

{#if captureInfo}
	<CaptureSection
		captureType="kafka"
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
				Listening to Kafka events...
			{:else}
				Start capturing to listen to Kafka events.
			{/if}
		</svelte:fragment>
	</CaptureSection>
{/if}
