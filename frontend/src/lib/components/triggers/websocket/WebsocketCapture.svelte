<script lang="ts">
	import type { CaptureInfo } from '../CaptureSectionV2.svelte'
	import CaptureSection from '../CaptureSectionV2.svelte'
	import CaptureTable from '../CaptureTable.svelte'

	export let captureInfo: CaptureInfo | undefined = undefined
	export let captureTable: CaptureTable | undefined = undefined
	export let isValid: boolean | undefined = undefined
	export let hasPreprocessor: boolean = false
	export let isFlow: boolean = false
</script>

{#if captureInfo}
	<CaptureSection
		captureType="http"
		disabled={isValid === false}
		{captureInfo}
		on:captureToggle
		on:applyArgs
		on:updateSchema
		on:addPreprocessor
		on:testWithArgs
		bind:captureTable
		{hasPreprocessor}
		{isFlow}
	>
		<svelte:fragment slot="description">
			{#if captureInfo.active}
				Send a POST request to the URL below to simulate a webhook event.
			{/if}
		</svelte:fragment>
	</CaptureSection>
{/if}
