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
		captureType="nats"
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
				Listening to NATS messages...
			{:else}
				Start capturing to listen to NATS messages.
			{/if}
		</svelte:fragment>
	</CaptureSection>
{/if}
